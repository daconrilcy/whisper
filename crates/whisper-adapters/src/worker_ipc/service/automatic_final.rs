use super::super::support::NEXT_IMPORT_JOB_ID;
use std::{
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};
use whisper_core::{ImportEvent, JobId};

pub(in crate::worker_ipc) static NEXT_AUTO_EXPORT_STAGE: AtomicU64 = AtomicU64::new(1);

fn next_automatic_final_job_id() -> JobId {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(1, |time| time.as_nanos().min(u64::MAX as u128) as u64);
    let mut observed = NEXT_IMPORT_JOB_ID.load(Ordering::Relaxed);
    let serial = loop {
        let next = now.max(observed.saturating_add(1));
        match NEXT_IMPORT_JOB_ID.compare_exchange_weak(
            observed,
            next,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => break next,
            Err(actual) => observed = actual,
        }
    };
    JobId(((std::process::id() as u128) << 64) | serial as u128)
}

pub(in crate::worker_ipc) fn inspect_automatic_final_queue(
    destination: &Path,
) -> Result<Vec<crate::queue_store::QueueEntry>, String> {
    inspect_automatic_final_queue_with(destination, |path| {
        crate::queue_store::QueueStore::inspect_read_only(path)
    })
}

pub(in crate::worker_ipc) fn inspect_automatic_final_queue_with(
    destination: &Path,
    inspect: impl FnOnce(&Path) -> Result<Vec<crate::queue_store::QueueEntry>, String>,
) -> Result<Vec<crate::queue_store::QueueEntry>, String> {
    inspect(destination).map_err(|error| format!("AutomaticFinalQueueUnavailable: {error}"))
}

pub(in crate::worker_ipc) fn attach_automatic_final_import(
    live: &crate::parakeet_live::ParakeetLive,
    event: ImportEvent,
) -> ImportEvent {
    let ImportEvent::ParakeetCaptureStopped {
        job_id: capture_job,
        generation: _,
        wav_path,
        automatic_final,
        ..
    } = &event
    else {
        return event;
    };
    if automatic_final.is_some() || wav_path.is_empty() || !Path::new(wav_path).is_file() {
        return event;
    }
    // Queue/archive inspection can read an arbitrarily large durable journal.
    // Keep it off the capture service: the automatic-import lane validates this
    // candidate before writing status or starting any import work.
    let import_job = next_automatic_final_job_id();
    let Ok(automatic) = live.automatic_final_import(wav_path, import_job) else {
        return event;
    };
    debug_assert_eq!(*capture_job, live.identity().0);
    match event {
        ImportEvent::ParakeetCaptureStopped {
            job_id,
            generation,
            session_id,
            wav_path,
            txt_path,
            srt_path,
            complete,
            final_segments,
            unresolved,
            diagnostic,
            ..
        } => ImportEvent::ParakeetCaptureStopped {
            job_id,
            generation,
            session_id,
            wav_path,
            txt_path,
            srt_path,
            complete,
            final_segments,
            unresolved,
            diagnostic,
            automatic_final: Some(automatic),
        },
        other => other,
    }
}

pub(in crate::worker_ipc) fn publish_automatic_final_exports(
    source_txt: &Path,
    source_srt: &Path,
    automatic: &whisper_core::ports::AutomaticFinalImport,
) -> Result<(), String> {
    let outputs = &automatic.outputs;
    let txt =
        fs::read(source_txt).map_err(|error| format!("Export TXT automatique absent : {error}"))?;
    let srt =
        fs::read(source_srt).map_err(|error| format!("Export SRT automatique absent : {error}"))?;
    let txt_path = Path::new(&outputs.txt_path);
    let srt_path = Path::new(&outputs.srt_path);
    if txt_path.parent() != srt_path.parent() {
        return Err("Les exports automatiques doivent partager leur dossier de destination".into());
    }
    if txt_path.exists() || srt_path.exists() {
        return Err("Une sortie automatique existe déjà; aucune sortie n'a été remplacée".into());
    }
    let (txt_stage, mut txt_file) = create_automatic_stage(txt_path)?;
    if let Err(error) = txt_file.write_all(&txt).and_then(|()| txt_file.sync_all()) {
        drop(txt_file);
        return Err(with_cleanup_error(
            format!("Écriture du TXT automatique impossible : {error}"),
            remove_automatic_stage(&txt_stage),
        ));
    }
    drop(txt_file);
    let (srt_stage, mut srt_file) = match create_automatic_stage(srt_path) {
        Ok(stage) => stage,
        Err(error) => {
            return Err(with_cleanup_error(
                format!("Création du SRT automatique impossible : {error}"),
                remove_automatic_stage(&txt_stage),
            ));
        }
    };
    if let Err(error) = srt_file.write_all(&srt).and_then(|()| srt_file.sync_all()) {
        drop(srt_file);
        let cleanup =
            remove_automatic_stage(&srt_stage).and_then(|()| remove_automatic_stage(&txt_stage));
        return Err(with_cleanup_error(
            format!("Écriture du SRT automatique impossible : {error}"),
            cleanup,
        ));
    }
    drop(srt_file);
    publish_automatic_staged_pair(&txt_stage, &srt_stage, txt_path, srt_path)?;
    if let Err(error) =
        set_automatic_final_manifest_status(automatic, "produced_unverified", None, None)
    {
        return Err(with_cleanup_error(
            format!("Mise à jour du statut compagnon impossible : {error}"),
            remove_automatic_paths(&[txt_path, srt_path]),
        ));
    }
    Ok(())
}

pub(in crate::worker_ipc) fn set_automatic_final_manifest_status(
    automatic: &whisper_core::ports::AutomaticFinalImport,
    status: &str,
    effective_backend: Option<&str>,
    diagnostic: Option<&str>,
) -> Result<(), String> {
    let path = Path::new(&automatic.outputs.companion_json_path);
    let bytes = fs::read(path)
        .map_err(|error| format!("Lecture du JSON compagnon impossible : {error}"))?;
    let mut document: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("JSON compagnon invalide : {error}"))?;
    let object = document
        .as_object_mut()
        .ok_or_else(|| "La racine du JSON compagnon doit être un objet".to_owned())?;
    if !object
        .get("confirmed_intervals")
        .is_some_and(serde_json::Value::is_array)
        || !object
            .get("unresolved_intervals")
            .is_some_and(serde_json::Value::is_array)
    {
        return Err("Le JSON compagnon ne contient pas les intervalles directs attendus".into());
    }
    let model_sha256 = automatic
        .request
        .model_sha256
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    object.insert(
        "automatic_final_transcription".into(),
        serde_json::json!({
            "status": status,
            "txt_path": automatic.outputs.txt_path,
            "srt_path": automatic.outputs.srt_path,
            "model_sha256": model_sha256,
            "requested_compute": format!("{:?}", automatic.request.config.compute).to_ascii_lowercase(),
            "effective_backend": effective_backend,
            "diagnostic": diagnostic,
        }),
    );
    let replacement = serde_json::to_vec_pretty(&document)
        .map_err(|error| format!("Sérialisation du JSON compagnon impossible : {error}"))?;
    let (temporary, mut file) = create_automatic_stage(path)
        .map_err(|error| format!("Réservation du JSON compagnon impossible : {error}"))?;
    if let Err(error) = file
        .write_all(&replacement)
        .and_then(|()| file.write_all(b"\n"))
        .and_then(|()| file.sync_all())
    {
        drop(file);
        return Err(with_cleanup_error(
            format!("Écriture du JSON compagnon impossible : {error}"),
            remove_automatic_stage(&temporary),
        ));
    }
    drop(file);
    fs::rename(&temporary, path).map_err(|error| {
        with_cleanup_error(
            format!("Remplacement du JSON compagnon impossible : {error}"),
            remove_automatic_stage(&temporary),
        )
    })
}

fn create_automatic_stage(final_path: &Path) -> Result<(PathBuf, fs::File), String> {
    let parent = final_path
        .parent()
        .ok_or_else(|| "Sortie automatique sans dossier parent".to_owned())?;
    let name = final_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "Nom de sortie automatique invalide".to_owned())?;
    for _ in 0..8 {
        let nonce = NEXT_AUTO_EXPORT_STAGE.fetch_add(1, Ordering::Relaxed);
        let path = parent.join(format!(".{name}.{}.{}.stage", std::process::id(), nonce));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.to_string()),
        }
    }
    Err("Impossible de réserver un nom temporaire unique pour la sortie automatique".into())
}

fn remove_automatic_stage(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!(
            "suppression temporaire de {} impossible: {error}",
            path.display()
        )),
    }
}

fn remove_automatic_paths(paths: &[&Path]) -> Result<(), String> {
    let errors = paths
        .iter()
        .filter_map(|path| {
            fs::remove_file(path)
                .err()
                .filter(|error| error.kind() != io::ErrorKind::NotFound)
                .map(|error| format!("{}: {error}", path.display()))
        })
        .collect::<Vec<_>>();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn with_cleanup_error(primary: String, cleanup: Result<(), String>) -> String {
    match cleanup {
        Ok(()) => primary,
        Err(cleanup) => format!("{primary}; nettoyage incomplet: {cleanup}"),
    }
}

pub(in crate::worker_ipc) fn publish_automatic_staged_pair(
    txt_stage: &Path,
    srt_stage: &Path,
    txt_path: &Path,
    srt_path: &Path,
) -> Result<(), String> {
    if txt_path.exists() || srt_path.exists() {
        return Err(with_cleanup_error(
            "Une sortie automatique existe déjà; aucune sortie n'a été remplacée".into(),
            remove_automatic_stage(txt_stage).and_then(|()| remove_automatic_stage(srt_stage)),
        ));
    }
    fs::hard_link(txt_stage, txt_path).map_err(|error| {
        with_cleanup_error(
            format!("Publication du TXT automatique impossible : {error}"),
            remove_automatic_stage(txt_stage).and_then(|()| remove_automatic_stage(srt_stage)),
        )
    })?;
    if let Err(error) = fs::hard_link(srt_stage, srt_path) {
        let cleanup = remove_automatic_paths(&[txt_path, txt_stage, srt_stage]);
        return Err(with_cleanup_error(
            format!("Publication du SRT automatique impossible : {error}"),
            cleanup,
        ));
    }
    if let Err(error) =
        remove_automatic_stage(txt_stage).and_then(|()| remove_automatic_stage(srt_stage))
    {
        let rollback = remove_automatic_paths(&[txt_path, srt_path, txt_stage, srt_stage]);
        return Err(with_cleanup_error(
            format!("Nettoyage des exports temporaires impossible : {error}"),
            rollback,
        ));
    }
    Ok(())
}
