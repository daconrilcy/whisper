#[cfg(windows)]
use super::super::{
    CleanupObject, CleanupObjectIdentity, CompleteRecord, PendingRecord, PublicationIndex,
    SegmentRecord, fs, generation_path, hex, io, load_publication_intent, publication_index_path,
    sha256_file,
};
use super::super::{CleanupPlan, Path, PathBuf};
#[cfg(windows)]
use super::{
    cleanup_targets_match_intent,
    proofs::{identity_snapshot, relative_to_root},
    write_cleanup_plan,
};

#[cfg(windows)]
pub(super) fn runtime_identity(
    value: &CleanupObjectIdentity,
) -> crate::windows_file_identity::FileIdentity {
    crate::windows_file_identity::FileIdentity {
        volume_serial: value.volume_serial,
        file_id: value.file_id,
        is_directory: value.is_directory,
    }
}

#[cfg(windows)]
pub(in crate::archive) fn run_cleanup_plan_locked(
    root: &Path,
    plan: &mut CleanupPlan,
) -> Result<(), String> {
    use crate::windows_file_identity::{
        VerifiedFile, delete_if_identity_matches, open_and_hash_source,
    };
    if !plan.published
        || root.canonicalize().ok().as_deref()
            != Some(
                Path::new(&plan.destination)
                    .canonicalize()
                    .map_err(|error| format!("CleanupUnsafePath: destination: {error}"))?
                    .as_path(),
            )
    {
        return Err("CleanupUnsafePath: destination de reprise divergente".into());
    }
    let source = open_and_hash_source(Path::new(&plan.source_path))?;
    if hex(&source.sha256) != plan.source_sha256 {
        return Err("SourceChanged: l'audio original ne correspond plus au snapshot traité".into());
    }
    let _source_handle = source;

    // Build from the canonical destination captured in the durable intent so
    // Windows' `\\?\` canonical path prefix does not cause a false mismatch.
    let successor_root = Path::new(&plan.destination);
    let successor_path = generation_path(
        successor_root,
        plan.successor.job_id,
        plan.successor.generation,
    );
    if successor_path.canonicalize().ok().as_deref() != Some(successor_path.as_path()) {
        return Err("CleanupIdentityChanged: le nouveau traitement a été déplacé".into());
    }
    let successor_handle = VerifiedFile::open(&successor_path, true)?;
    successor_handle.verify(&runtime_identity(&plan.successor_directory))?;
    if plan.successor_files.len() != 6 || plan.successor_index.is_none() {
        return Err("CleanupIdentityChanged: preuve durable du successeur incomplète".into());
    }
    let expected_successor_names = [
        "manifest.json",
        "transcript.txt",
        "transcript.srt",
        "pending.json",
        "segments.jsonl",
        "CURRENT",
    ];
    let mut held_successor_files = Vec::new();
    let mut successor_manifest = None;
    let mut successor_pending = None;
    let mut successor_segments = None;
    for (index, expected_name) in expected_successor_names.iter().enumerate() {
        let proof = &plan.successor_files[index];
        if proof.relative_path
            != relative_to_root(successor_root, &successor_path.join(expected_name))?
        {
            return Err("CleanupIdentityChanged: chemin de preuve successeur inattendu".into());
        }
        let path = safe_cleanup_path(root, &proof.relative_path)?;
        let mut handle = VerifiedFile::open(&path, false)?;
        handle.verify(&runtime_identity(&proof.identity))?;
        if Some(handle.sha256()?).as_deref() != proof.sha256.as_deref() {
            return Err(format!(
                "CleanupIdentityChanged: preuve successeur divergente {expected_name}"
            ));
        }
        if *expected_name == "CURRENT" && handle.read_all()? != b"manifest.json\n" {
            return Err(
                "CleanupIdentityChanged: CURRENT ne référence pas le manifeste attendu".into(),
            );
        }
        if *expected_name == "manifest.json" {
            let manifest: CompleteRecord =
                serde_json::from_slice(&handle.read_all()?).map_err(|error| {
                    format!("CleanupIdentityChanged: manifeste successeur: {error}")
                })?;
            if manifest.job_id != plan.successor.job_id
                || manifest.generation != plan.successor.generation
                || manifest.source_sha256 != plan.source_sha256
                || manifest.text.path != "transcript.txt"
                || manifest.srt.path != "transcript.srt"
            {
                return Err(
                    "CleanupIdentityChanged: manifeste successeur ne correspond pas".into(),
                );
            }
            successor_manifest = Some(manifest);
        }
        if *expected_name == "pending.json" {
            successor_pending = Some(
                serde_json::from_slice::<PendingRecord>(&handle.read_all()?).map_err(|error| {
                    format!("CleanupIdentityChanged: pending successeur: {error}")
                })?,
            );
        }
        if *expected_name == "segments.jsonl" {
            successor_segments = Some(handle.read_all()?);
        }
        held_successor_files.push(handle);
    }
    let manifest = successor_manifest.expect("six successor proofs include manifest");
    let pending = successor_pending.expect("six successor proofs include pending");
    let segment_bytes = successor_segments.expect("six successor proofs include segments");
    if manifest.source_sha256 != plan.source_sha256
        || pending.job_id != plan.successor.job_id
        || pending.generation != plan.successor.generation
        || pending.source_sha256 != plan.source_sha256
    {
        return Err("CleanupIdentityChanged: identité du successeur divergente".into());
    }
    if !segment_bytes.is_empty() && !segment_bytes.ends_with(b"\n") {
        return Err("CleanupIdentityChanged: journal segments tronqué".into());
    }
    let segments = segment_bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| {
            serde_json::from_slice::<SegmentRecord>(line)
                .map_err(|error| format!("CleanupIdentityChanged: journal segments: {error}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if segments.len() as u64 != manifest.segments
        || segments
            .iter()
            .enumerate()
            .any(|(index, segment)| segment.sequence != index as u64 + 1)
    {
        return Err("CleanupIdentityChanged: journal segments incohérent".into());
    }
    let intent = load_publication_intent(&successor_path)?;
    if intent.cleanup_id.as_deref() != Some(plan.cleanup_id.as_str())
        || intent.source_sha256 != plan.source_sha256
        || !cleanup_targets_match_intent(&intent.cleanup_targets, &plan.targets)
    {
        return Err("CleanupIdentityChanged: intention de publication divergente".into());
    }
    let index_proof = plan.successor_index.as_ref().unwrap();
    if index_proof.relative_path != relative_to_root(root, &publication_index_path(root))? {
        return Err("CleanupIdentityChanged: chemin de l'index successeur inattendu".into());
    }
    let index_path = safe_cleanup_path(root, &index_proof.relative_path)?;
    let mut index_handle = VerifiedFile::open(&index_path, false)?;
    index_handle.verify(&runtime_identity(&index_proof.identity))?;
    if Some(index_handle.sha256()?).as_deref() != index_proof.sha256.as_deref() {
        return Err("CleanupIdentityChanged: index de publication modifié".into());
    }
    let index: PublicationIndex = serde_json::from_slice(&index_handle.read_all()?)
        .map_err(|error| format!("CleanupIdentityChanged: index de publication: {error}"))?;
    if index.version != 1
        || !index.records.iter().any(|record| {
            record.job_id == plan.successor.job_id
                && record.generation == plan.successor.generation
                && record.source_sha256 == plan.source_sha256
                && record.active
        })
    {
        return Err("CleanupIdentityChanged: successeur absent de l'index actif".into());
    }
    let successor_identity_set: std::collections::HashSet<_> = std::iter::once((
        plan.successor_directory.volume_serial,
        plan.successor_directory.file_id,
        plan.successor_directory.is_directory,
    ))
    .chain(plan.successor_files.iter().map(|object| {
        (
            object.identity.volume_serial,
            object.identity.file_id,
            object.identity.is_directory,
        )
    }))
    .chain(std::iter::once((
        index_proof.identity.volume_serial,
        index_proof.identity.file_id,
        index_proof.identity.is_directory,
    )))
    .collect();
    for target in &plan.targets {
        for object in target.objects.iter().chain(target.container.iter()) {
            if successor_identity_set.contains(&(
                object.identity.volume_serial,
                object.identity.file_id,
                object.identity.is_directory,
            )) {
                return Err(
                    "CleanupUnsafePath: un objet successeur est aliasé par une cible".into(),
                );
            }
        }
    }
    let _successor_handles = (successor_handle, held_successor_files, index_handle);

    // Preflight every target before the first disposition. The original source,
    // successor and all ancestor handles remain open with no write/delete sharing.
    let mut held_ancestors_by_path: std::collections::HashMap<
        String,
        (CleanupObjectIdentity, VerifiedFile),
    > = std::collections::HashMap::new();
    let mut removed_container_paths = std::collections::HashSet::new();
    for target in &mut plan.targets {
        if let Some(container) = &mut target.container {
            if container.removed {
                removed_container_paths.insert(container.relative_path.clone());
            } else if fs::symlink_metadata(safe_cleanup_path(root, &container.relative_path)?)
                .is_err_and(|error| error.kind() == io::ErrorKind::NotFound)
            {
                container.removed = true;
                container.error = None;
                removed_container_paths.insert(container.relative_path.clone());
            }
        }
    }
    if !removed_container_paths.is_empty() {
        write_cleanup_plan(root, plan)?;
    }
    for target in &plan.targets {
        if target.treatment.destination != plan.destination
            || target.source_sha256 != plan.source_sha256
            || (target.treatment.job_id == plan.successor.job_id
                && target.treatment.generation == plan.successor.generation)
        {
            return Err(
                "CleanupUnsafePath: cible chevauche le successeur ou autre destination".into(),
            );
        }
        if let Some(container) = &target.container
            && !container.removed
        {
            let expected_path = root
                .join("transcriptions")
                .join(format!("{:032x}", target.treatment.job_id.0));
            if !container.identity.is_directory
                || container.sha256.is_some()
                || container.relative_path != relative_to_root(root, &expected_path)?
                || !target.ancestors.iter().any(|ancestor| {
                    ancestor.relative_path == container.relative_path
                        && ancestor.identity == container.identity
                })
            {
                return Err("CleanupIdentityChanged: preuve du conteneur invalide".into());
            }
            let path = safe_cleanup_path(root, &container.relative_path)?;
            let canonical = path.canonicalize().map_err(|error| {
                format!(
                    "CleanupIdentityChanged: conteneur inaccessible {}: {error}",
                    container.relative_path
                )
            })?;
            let canonical_root = Path::new(&plan.destination)
                .canonicalize()
                .map_err(|error| format!("CleanupIdentityChanged: destination: {error}"))?;
            if canonical != canonical_root.join(&container.relative_path) {
                return Err(format!(
                    "CleanupUnsafePath: conteneur redirigé {}",
                    container.relative_path
                ));
            }
        }
        for ancestor in &target.ancestors {
            if removed_container_paths.contains(&ancestor.relative_path) {
                continue;
            }
            if let Some(previous) = held_ancestors_by_path.get(&ancestor.relative_path) {
                if previous.0 != ancestor.identity {
                    return Err(format!(
                        "CleanupIdentityChanged: identités d'ancêtre divergentes {}",
                        ancestor.relative_path
                    ));
                }
                continue;
            }
            let path = safe_cleanup_path(root, &ancestor.relative_path)?;
            let handle = VerifiedFile::open(&path, true)?;
            handle
                .verify(&runtime_identity(&ancestor.identity))
                .map_err(|error| format!("{error}: ancêtre {}", ancestor.relative_path))?;
            held_ancestors_by_path.insert(
                ancestor.relative_path.clone(),
                (ancestor.identity.clone(), handle),
            );
        }
    }
    for target in &mut plan.targets {
        for object in &mut target.objects {
            if object.removed {
                continue;
            }
            let path = safe_cleanup_path(root, &object.relative_path)?;
            if fs::symlink_metadata(&path)
                .is_err_and(|error| error.kind() == io::ErrorKind::NotFound)
            {
                object.removed = true;
                object.error = None;
                continue;
            }
            let path = safe_cleanup_path(root, &object.relative_path)?;
            let actual =
                crate::windows_file_identity::inspect(&path, object.identity.is_directory)?;
            if identity_snapshot(actual)? != object.identity {
                return Err(format!("CleanupIdentityChanged: {}", object.relative_path));
            }
            if let Some(expected_sha) = &object.sha256
                && hex(&sha256_file(&path)?) != *expected_sha
            {
                return Err(format!(
                    "CleanupIdentityChanged: empreinte modifiée {}",
                    object.relative_path
                ));
            }
        }
        if let Some(container) = &target.container
            && !container.removed
        {
            let Some((identity, _)) = held_ancestors_by_path.get(&container.relative_path) else {
                return Err(format!(
                    "CleanupIdentityChanged: ancêtre du conteneur absent {}",
                    container.relative_path
                ));
            };
            if identity != &container.identity {
                return Err(format!(
                    "CleanupIdentityChanged: identité du conteneur divergente {}",
                    container.relative_path
                ));
            }
        }
    }
    let mut held_target_directories = std::collections::HashMap::new();
    for target in &mut plan.targets {
        let directory = generation_path(root, target.treatment.job_id, target.treatment.generation);
        let directory_relative = relative_to_root(root, &directory)?;
        let directory_proof = target
            .objects
            .iter()
            .find(|object| {
                object.relative_path == directory_relative && object.identity.is_directory
            })
            .ok_or_else(|| "CleanupIdentityChanged: preuve du dossier cible absente".to_owned())?;
        if directory_proof.removed {
            continue;
        }
        let directory_handle = VerifiedFile::open(&directory, true)?;
        directory_handle
            .verify(&runtime_identity(&directory_proof.identity))
            .map_err(|error| format!("{error}: génération {directory_relative}"))?;
        for entry in fs::read_dir(&directory).map_err(|error| {
            format!("CleanupIdentityChanged: inventorier le dossier cible: {error}")
        })? {
            let entry =
                entry.map_err(|error| format!("CleanupIdentityChanged: entrée cible: {error}"))?;
            let relative = relative_to_root(root, &entry.path())?;
            if target
                .objects
                .iter()
                .any(|object| object.relative_path == relative)
            {
                continue;
            }
            let file_type = entry.file_type().map_err(|error| error.to_string())?;
            if file_type.is_symlink() {
                return Err(format!(
                    "CleanupUnsafePath: nouvel élément reparse conservé: {relative}"
                ));
            }
            let identity = identity_snapshot(crate::windows_file_identity::inspect(
                &entry.path(),
                file_type.is_dir(),
            )?)?;
            let sha = file_type
                .is_file()
                .then(|| sha256_file(&entry.path()).map(|value| hex(&value)))
                .transpose()?;
            target.objects.push(CleanupObject {
                relative_path: relative.clone(),
                identity,
                sha256: sha,
                owned: false,
                removed: false,
                error: Some(format!(
                    "élément ajouté après confirmation, conservé: {relative}"
                )),
            });
        }
        held_target_directories.insert(directory_relative, directory_handle);
    }
    write_cleanup_plan(root, plan)?;

    for target_index in 0..plan.targets.len() {
        let mut indices: Vec<usize> = (0..plan.targets[target_index].objects.len()).collect();
        indices.sort_by_key(|index| {
            let object = &plan.targets[target_index].objects[*index];
            (
                object.identity.is_directory,
                std::cmp::Reverse(
                    object
                        .relative_path
                        .chars()
                        .filter(|character| matches!(character, '\\' | '/'))
                        .count(),
                ),
            )
        });
        for object_index in indices {
            let object = &mut plan.targets[target_index].objects[object_index];
            if object.removed || !object.owned {
                continue;
            }
            let path = safe_cleanup_path(root, &object.relative_path)?;
            let result = (|| {
                if fs::symlink_metadata(&path)
                    .is_err_and(|error| error.kind() == io::ErrorKind::NotFound)
                {
                    return Ok(());
                }
                if object.identity.is_directory {
                    // Only the generation directory is included; parent folders are
                    // deliberately retained, so no broad recursive removal occurs.
                    let handle = held_target_directories
                        .remove(&object.relative_path)
                        .ok_or_else(|| {
                            "CleanupIdentityChanged: handle du dossier cible absent".to_owned()
                        })?;
                    handle.mark_delete(&runtime_identity(&object.identity), None)
                } else {
                    delete_if_identity_matches(
                        &path,
                        &runtime_identity(&object.identity),
                        object.sha256.as_deref(),
                    )
                }
            })();
            match result {
                Ok(()) => {
                    object.removed = true;
                    object.error = None;
                }
                Err(error) => {
                    object.error = Some(error.clone());
                    plan.last_error = Some(error.clone());
                    write_cleanup_plan(root, plan)?;
                    return Err(error);
                }
            }
            write_cleanup_plan(root, plan)?;
        }
    }
    for target_index in 0..plan.targets.len() {
        let Some(container_snapshot) = plan.targets[target_index].container.clone() else {
            continue;
        };
        if container_snapshot.removed || !container_snapshot.owned {
            continue;
        }
        let path = safe_cleanup_path(root, &container_snapshot.relative_path)?;
        if fs::symlink_metadata(&path).is_err_and(|error| error.kind() == io::ErrorKind::NotFound) {
            let container = plan.targets[target_index].container.as_mut().unwrap();
            container.removed = true;
            container.error = None;
            write_cleanup_plan(root, plan)?;
            continue;
        }
        let canonical = path
            .canonicalize()
            .map_err(|error| format!("CleanupIdentityChanged: conteneur inaccessible: {error}"))?;
        let expected = Path::new(&plan.destination)
            .canonicalize()
            .map_err(|error| format!("CleanupIdentityChanged: destination: {error}"))?
            .join(&container_snapshot.relative_path);
        if canonical != expected {
            return Err(format!(
                "CleanupUnsafePath: conteneur redirigé {}",
                container_snapshot.relative_path
            ));
        }
        let mut entries = fs::read_dir(&path)
            .map_err(|error| format!("CleanupIdentityChanged: lire conteneur: {error}"))?;
        if entries
            .next()
            .transpose()
            .map_err(|error| {
                format!("CleanupIdentityChanged: lire le contenu du conteneur: {error}")
            })?
            .is_some()
        {
            let container = plan.targets[target_index].container.as_mut().unwrap();
            container.owned = false;
            container.error = Some("contenu non sélectionné présent : conteneur conservé".into());
            write_cleanup_plan(root, plan)?;
            continue;
        }
        let identity = container_snapshot.identity.clone();
        let relative_path = container_snapshot.relative_path.clone();
        let handle = held_ancestors_by_path
            .remove(&relative_path)
            .ok_or_else(|| {
                format!("CleanupIdentityChanged: handle du conteneur absent {relative_path}")
            })?;
        if let Err(error) = handle.1.mark_delete(&runtime_identity(&identity), None) {
            let error = format!("{error}: conteneur {}", relative_path);
            let container = plan.targets[target_index].container.as_mut().unwrap();
            container.error = Some(error.clone());
            plan.last_error = Some(error.clone());
            write_cleanup_plan(root, plan)?;
            return Err(error);
        }
        let container = plan.targets[target_index].container.as_mut().unwrap();
        container.removed = true;
        container.error = None;
        write_cleanup_plan(root, plan)?;
    }
    let _keep_ancestors = (held_ancestors_by_path, held_target_directories);
    plan.last_error = None;
    write_cleanup_plan(root, plan)
}

#[cfg(not(windows))]
pub(in crate::archive) fn run_cleanup_plan_locked(
    _root: &Path,
    _plan: &mut CleanupPlan,
) -> Result<(), String> {
    Err("Unsupported: le nettoyage RTC-02 nécessite les handles Windows".into())
}

pub(super) fn safe_cleanup_path(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let path = Path::new(relative);
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err("CleanupUnsafePath: chemin relatif invalide dans le plan".into());
    }
    Ok(root.join(path))
}
