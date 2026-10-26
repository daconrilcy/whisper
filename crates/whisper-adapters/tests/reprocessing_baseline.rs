use std::{
    fs,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

use sha2::{Digest, Sha256};
use whisper_adapters::{
    archive::ArchiveStore, parakeet_import::ParakeetWorkerPaths, supervisor::WorkerPaths,
    worker_ipc::AsyncImportIo,
};
use whisper_core::{
    AppCommand, Application, ComputeChoice, Generation, ImportApplication, ImportRequest,
    JobConfig, JobId, LanguageChoice, model_catalog::ModelId, ports::DuplicateImportChoice,
};

const SOURCES: [(&str, &str); 3] = [
    ("wav-short", "WHISPER_RTC_BASELINE_WAV_SHORT"),
    ("wav-long", "WHISPER_RTC_BASELINE_WAV_LONG"),
    ("mp3", "WHISPER_RTC_BASELINE_MP3"),
];

#[test]
#[ignore = "E0 baseline: real Parakeet CPU imports; run only under the E0 guard"]
fn r11_application_baseline() {
    let root = required_path("WHISPER_RTC_BASELINE_ROOT");
    let root = validate_campaign_root(&root);
    let allowed_destination = root.join("selftest-positive").join("archive");
    let denied_destination = root
        .parent()
        .expect("E0 root has a parent")
        .join("selftest-negative-escape");
    assert!(validate_destination_under_root(&root, &allowed_destination).is_ok());
    assert!(validate_destination_under_root(&root, &denied_destination).is_err());
    eprintln!(
        "E0_PATH_CHECK {}",
        serde_json::json!({"positive":"PASS", "negative":"PASS", "root":root.display().to_string(), "allowed":allowed_destination.display().to_string(), "denied":denied_destination.display().to_string()})
    );
    let model_path = required_path("WHISPER_RTC_BASELINE_MODEL");
    let worker = required_path("WHISPER_RTC_BASELINE_WORKER");
    let native = required_path("WHISPER_RTC_BASELINE_NATIVE_DIR");
    let runtime = required_path("WHISPER_RTC_BASELINE_RUNTIME_ROOT");
    let fake_turbo = runtime.join("not-used-turbo.exe");
    let io = AsyncImportIo::start_with_model_catalog_and_parakeet(
        WorkerPaths {
            cpu: fake_turbo.clone(),
            gpu: fake_turbo,
        },
        model_path.parent().expect("model directory").to_path_buf(),
        runtime
            .join("not-used-turbo.gguf")
            .to_string_lossy()
            .into_owned(),
        Some(ParakeetWorkerPaths {
            executable: worker,
            native_dir: native,
            model: model_path.clone(),
            runtime_root: runtime.clone(),
        }),
    );
    let model_record = whisper_core::model_registry::model(ModelId::ParakeetTdt06BV3);
    let historical_turbo = runtime.join("historical/ggml-large-v3-turbo.bin");
    let snapshot = whisper_core::model_registry::engine_snapshot(
        1,
        |_| whisper_core::model_catalog::ModelAvailability::Ready,
        vec![],
    );
    let guard = whisper_core::model_selection_app::ModelCommandGuard::new_with_model_paths(
        snapshot,
        1,
        [
            (
                ModelId::ParakeetTdt06BV3,
                model_path.to_string_lossy().into_owned(),
            ),
            (
                ModelId::WhisperLargeV3Turbo,
                historical_turbo.to_string_lossy().into_owned(),
            ),
        ],
    );
    let mut app = ImportApplication::new(io).with_model_guard(guard);
    let campaign_deadline = Instant::now() + Duration::from_secs(3 * 60 * 60);
    let mut total_imports = 0u8;
    let run_id = format!("{}-{}", std::process::id(), unix_nanos());
    let mut serial = 1u128;
    for (source_index, (name, variable)) in SOURCES.into_iter().enumerate() {
        let source = required_path(variable);
        assert!(source.is_file(), "{variable} missing: {}", source.display());
        let source_hash = sha256(&source);
        eprintln!(
            "E0_INPUT {}",
            serde_json::json!({"source":name, "bytes":fs::metadata(&source).expect("source metadata").len(), "sha256":source_hash, "pid":std::process::id()})
        );

        for pass in 1..=2 {
            let destination = root
                .join("archive")
                .join(&run_id)
                .join(name)
                .join(format!("pass-{pass}"));
            validate_destination_under_root(&root, &destination)
                .expect("E0 destination must be inside the canonical campaign root");
            assert!(
                !destination.exists(),
                "refusing to reuse an earlier E0 archive: {}",
                destination.display()
            );
            fs::create_dir_all(&destination).expect("create fresh E0 archive");
            assert_inside(&destination, &root);
            let base = request(
                &source,
                &destination,
                &model_path,
                model_record.artifact.sha256,
                next_job_id(&mut serial),
            );
            run_import(
                &mut app,
                base.clone(),
                &campaign_deadline,
                "no-duplicate",
                name,
                source_index,
                pass,
            );
            total_imports += 1;
            assert_outputs_for_source(&destination, &source, &source_hash, 1);

            let create = request(
                &source,
                &destination,
                &model_path,
                model_record.artifact.sha256,
                next_job_id(&mut serial),
            );
            run_import_with_duplicate_choice(
                &mut app,
                create,
                DuplicateImportChoice::CreateNew,
                &campaign_deadline,
                "create-new",
                name,
                source_index,
                pass,
            );
            total_imports += 1;
            assert_outputs_for_source(&destination, &source, &source_hash, 2);

            let replace = request(
                &source,
                &destination,
                &model_path,
                model_record.artifact.sha256,
                next_job_id(&mut serial),
            );
            let previous_generations = generation_dirs(&destination);
            run_import_with_duplicate_choice(
                &mut app,
                replace,
                DuplicateImportChoice::Replace,
                &campaign_deadline,
                "replace",
                name,
                source_index,
                pass,
            );
            total_imports += 1;
            assert_outputs_for_source(&destination, &source, &source_hash, 1);
            let generations_after = generation_dirs(&destination);
            assert!(
                previous_generations.is_subset(&generations_after),
                "replace removed prior archive data"
            );
        }
        assert!(
            Instant::now() < campaign_deadline,
            "E0 total campaign exceeded three hours"
        );
        assert_eq!(
            sha256(&source),
            source_hash,
            "source changed during measurement"
        );
    }
    assert_eq!(
        total_imports, 18,
        "E0 campaign must perform exactly eighteen imports"
    );
}

fn request(
    source: &Path,
    destination: &Path,
    model: &Path,
    model_hash: &str,
    job_id: u128,
) -> ImportRequest {
    ImportRequest {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
        job_id: JobId(job_id),
        generation: Generation::first(),
        source_path: source.to_string_lossy().into_owned(),
        source_sha256: None,
        source_samples: None,
        model_path: model.to_string_lossy().into_owned(),
        model_sha256: decode_sha256(model_hash),
        destination: destination.to_string_lossy().into_owned(),
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        },
    }
}

fn run_import(
    app: &mut ImportApplication<AsyncImportIo>,
    request: ImportRequest,
    deadline: &Instant,
    operation: &str,
    source: &str,
    source_index: usize,
    pass: u8,
) {
    run_import_with_choice(
        app,
        request,
        None,
        deadline,
        operation,
        source,
        source_index,
        pass,
    );
}

fn run_import_with_duplicate_choice(
    app: &mut ImportApplication<AsyncImportIo>,
    request: ImportRequest,
    choice: DuplicateImportChoice,
    deadline: &Instant,
    operation: &str,
    source: &str,
    source_index: usize,
    pass: u8,
) {
    run_import_with_choice(
        app,
        request,
        Some(choice),
        deadline,
        operation,
        source,
        source_index,
        pass,
    );
}

fn run_import_with_choice(
    app: &mut ImportApplication<AsyncImportIo>,
    request: ImportRequest,
    duplicate_choice: Option<DuplicateImportChoice>,
    campaign_deadline: &Instant,
    operation: &str,
    source: &str,
    source_index: usize,
    pass: u8,
) {
    let started = Instant::now();
    let import_deadline = operation_deadline(campaign_deadline);
    let source_path = request.source_path.clone();
    let destination = request.destination.clone();
    let operation_id = format!("{:032x}-{}", request.job_id.0, request.generation.get());
    app.dispatch(AppCommand::EnqueueImport {
        request: request.clone(),
    })
    .expect("enqueue import through the application flow");

    let queued = loop {
        let view = app
            .dispatch(AppCommand::Refresh)
            .expect("refresh enqueue result");
        if let Some(prompt) = view.duplicate_import_prompt {
            let choice = duplicate_choice.unwrap_or_else(|| {
                panic!("{operation}: unexpected duplicate prompt for {source_path}")
            });
            app.dispatch(AppCommand::ResolveDuplicateImport { prompt, choice })
                .expect("confirm duplicate import through the application flow");
            break wait_for_queued_request(app, &source_path, &destination, &import_deadline);
        }

        if let Some(entry) = view.queue.into_iter().find(|entry| {
            entry.request.source_path == source_path && entry.request.destination == destination
        }) {
            assert!(
                duplicate_choice.is_none(),
                "{operation}: expected duplicate confirmation before queue admission"
            );
            break entry.request;
        }

        assert!(
            Instant::now() < import_deadline,
            "{operation}: enqueue did not produce a queue entry or duplicate prompt"
        );
        thread::sleep(Duration::from_millis(50));
    };
    assert_eq!(queued.source_path, source_path);
    assert_eq!(queued.destination, destination);
    app.dispatch(AppCommand::ProcessQueued {
        request: queued.clone(),
    })
    .expect("start queued import through the application flow");
    wait_for_complete(app, &import_deadline, operation);
    record_harness_timing(
        "harness-import-total",
        started.elapsed(),
        serde_json::json!({
            "measurement_scope":"application-request-to-published-txt-srt",
            "operation":operation,
            "operation_id":operation_id,
            "source":source,
            "source_index":source_index,
            "pass":pass,
            "destination":destination,
            "pid":std::process::id(),
            "outcome":"ok"
        }),
    );
}

fn wait_for_queued_request(
    app: &mut ImportApplication<AsyncImportIo>,
    source_path: &str,
    destination: &str,
    deadline: &Instant,
) -> ImportRequest {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
    loop {
        let view = app
            .dispatch(AppCommand::Refresh)
            .expect("refresh confirmed queue");
        if let Some(entry) = view.queue.into_iter().find(|entry| {
            entry.request.source_path == source_path && entry.request.destination == destination
        }) {
            return entry.request;
        }
        assert!(
            Instant::now() < *deadline,
            "confirmed duplicate did not enter the durable queue"
        );
        thread::sleep(Duration::from_millis(50));
    }
}

fn operation_deadline(campaign_deadline: &Instant) -> Instant {
    (*campaign_deadline).min(Instant::now() + Duration::from_secs(10 * 60))
}

fn wait_for_complete(app: &mut ImportApplication<AsyncImportIo>, deadline: &Instant, phase: &str) {
    loop {
        let view = app.dispatch(AppCommand::Refresh).expect("refresh import");
        if let Some((_, state)) = view.active_job {
            assert_ne!(
                state,
                whisper_core::JobState::Recoverable,
                "{phase}: {view:?}"
            );
            assert_ne!(
                state,
                whisper_core::JobState::Interrupted,
                "{phase}: {view:?}"
            );
            if state == whisper_core::JobState::Complete {
                assert!(
                    view.last_result.is_some(),
                    "{phase}: completion missing result"
                );
                return;
            }
        }
        assert!(Instant::now() < *deadline, "{phase} exceeded E0 deadline");
        thread::sleep(Duration::from_millis(100));
    }
}

fn assert_outputs_for_source(
    destination: &Path,
    source: &Path,
    source_hash: &str,
    expected: usize,
) {
    let pubs = ArchiveStore::published_imports(destination, source_hash).expect("publication scan");
    assert_eq!(
        pubs.len(),
        expected,
        "wrong active publications for {}",
        source.display()
    );
    for publication in pubs {
        assert_outputs_at(destination, publication.job_id.0);
    }
}

fn assert_outputs_at(destination: &Path, job_id: u128) {
    let passage = destination
        .join("transcriptions")
        .join(format!("{job_id:032x}"))
        .join("1");
    let txt = passage.join("transcript.txt");
    let srt = passage.join("transcript.srt");
    assert!(txt.is_file(), "TXT missing: {}", txt.display());
    assert!(srt.is_file(), "SRT missing: {}", srt.display());
    assert!(!fs::read_to_string(txt).expect("read TXT").trim().is_empty());
    assert!(fs::read_to_string(srt).expect("read SRT").contains("-->"));
}

fn generation_dirs(destination: &Path) -> std::collections::BTreeSet<PathBuf> {
    let root = destination.join("transcriptions");
    let mut dirs = std::collections::BTreeSet::new();
    let Ok(jobs) = fs::read_dir(root) else {
        return dirs;
    };
    for job in jobs.flatten() {
        let Ok(generations) = fs::read_dir(job.path()) else {
            continue;
        };
        for generation in generations.flatten() {
            if generation.path().is_dir() {
                dirs.insert(generation.path());
            }
        }
    }
    dirs
}

fn required_path(name: &str) -> PathBuf {
    std::env::var_os(name)
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("{name} is required; E0 never skips missing inputs"))
}

fn validate_campaign_root(root: &Path) -> PathBuf {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("adapter manifest is under the workspace")
        .canonicalize()
        .expect("canonical workspace root");
    let expected = workspace.join("target/parakeet-tests/retraitement-cible/mesure-avant");
    let actual = root.canonicalize().expect("canonical E0 root");
    assert_eq!(
        actual, expected,
        "WHISPER_RTC_BASELINE_ROOT must be the exact plan-approved E0 root"
    );
    actual
}

fn validate_destination_under_root(root: &Path, destination: &Path) -> Result<(), String> {
    let canonical_root = root
        .canonicalize()
        .map_err(|error| format!("cannot canonicalize E0 root: {error}"))?;
    if !destination.is_absolute() || !destination.starts_with(&canonical_root) {
        return Err(format!(
            "destination is outside E0 root: {}",
            destination.display()
        ));
    }
    if destination
        .components()
        .any(|component| component == std::path::Component::ParentDir)
    {
        return Err(format!(
            "destination has a non-canonical component: {}",
            destination.display()
        ));
    }

    let mut existing = destination;
    while !existing.exists() {
        existing = existing
            .parent()
            .ok_or_else(|| "E0 destination has no existing ancestor".to_owned())?;
    }
    let canonical_existing = existing
        .canonicalize()
        .map_err(|error| format!("cannot canonicalize destination ancestor: {error}"))?;
    if !canonical_existing.starts_with(&canonical_root) {
        return Err(format!(
            "destination ancestor escapes E0 root: {}",
            canonical_existing.display()
        ));
    }
    Ok(())
}

fn assert_inside(path: &Path, root: &Path) {
    let path = path.canonicalize().expect("canonical E0 path");
    let root = root.canonicalize().expect("canonical E0 root");
    assert!(
        path.starts_with(root),
        "path escaped E0 root: {}",
        path.display()
    );
}

fn next_job_id(serial: &mut u128) -> u128 {
    let job_id = 0xe000_0000_0000_0000_0000_0000_0000_0000_u128 | *serial;
    *serial += 1;
    job_id
}

fn unix_nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time is after UNIX epoch")
        .as_nanos()
}

fn sha256(path: &Path) -> String {
    let bytes = fs::read(path).expect("read input for SHA-256");
    format!("{:x}", Sha256::digest(bytes))
}

fn decode_sha256(value: &str) -> [u8; 32] {
    let mut bytes = [0; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .expect("model SHA-256 is hexadecimal");
    }
    bytes
}

fn record_harness_timing(stage: &str, elapsed: Duration, details: serde_json::Value) {
    eprintln!(
        "E0_TIMING {}",
        serde_json::json!({
            "process": "harness",
            "origin": "harness",
            "stage": stage,
            "duration_ms": elapsed.as_millis(),
            "details": details,
        })
    );
}
