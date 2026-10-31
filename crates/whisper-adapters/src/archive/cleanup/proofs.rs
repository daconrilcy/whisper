#[cfg(windows)]
use super::super::{
    CleanupAncestor, CleanupObject, CompleteRecord, Digest, InstalledExportOwnership,
    PendingRecord, SegmentRecord, Sha256, apply_publication_intent, fs, generation_path, hex,
    installed_export_ownership_path, io, load_or_migrate_publication_index,
    load_publication_intent, persisted_file_identity, publication_index_path, sha256_file,
    verify_artifact, write_publication_index,
};
use super::super::{
    CleanupObjectIdentity, CleanupPlan, CleanupTarget, Path, PublishedImport, TreatmentIdentity,
};
#[cfg(windows)]
use super::{
    cleanup_targets_match_intent,
    execution::{runtime_identity, safe_cleanup_path},
};

#[cfg(windows)]
pub(in crate::archive) fn capture_cleanup_target(
    root: &Path,
    candidate: &PublishedImport,
) -> Result<CleanupTarget, String> {
    use crate::windows_file_identity::inspect;
    let treatment = TreatmentIdentity {
        destination: root.to_string_lossy().into_owned(),
        job_id: candidate.job_id,
        generation: candidate.generation,
    };
    let directory = generation_path(root, candidate.job_id, candidate.generation);
    let canonical = directory
        .canonicalize()
        .map_err(|error| format!("CleanupUnverified: traitement absent: {error}"))?;
    if canonical != directory {
        return Err("CleanupUnverified: chemin de traitement redirigé".into());
    }
    let manifest: CompleteRecord = serde_json::from_slice(
        &fs::read(directory.join("manifest.json"))
            .map_err(|error| format!("CleanupUnverified: manifeste illisible: {error}"))?,
    )
    .map_err(|error| format!("CleanupUnverified: manifeste illisible: {error}"))?;
    if manifest.job_id != candidate.job_id
        || manifest.generation != candidate.generation
        || manifest.source_sha256 != candidate.source_sha256
    {
        return Err("CleanupUnverified: manifeste différent du catalogue".into());
    }
    verify_artifact(&directory, &manifest.text)?;
    verify_artifact(&directory, &manifest.srt)?;
    let mut ancestors = Vec::new();
    for ancestor in [
        root.join("transcriptions"),
        directory.parent().unwrap().to_path_buf(),
    ] {
        capture_cleanup_ancestor(root, &ancestor, &mut ancestors)?;
    }

    const KNOWN_GENERATION_FILES: &[&str] = &[
        "CURRENT",
        "manifest.json",
        "pending.json",
        "segments.jsonl",
        "transcript.txt",
        "transcript.srt",
        "r11-publication.json",
        "installed-exports.json",
        "vad-windows.jsonl",
    ];
    let mut objects = Vec::new();
    let generation_identity = identity_snapshot(inspect(&directory, true)?)?;
    for entry in fs::read_dir(&directory)
        .map_err(|error| format!("CleanupUnverified: lister le traitement: {error}"))?
    {
        let entry =
            entry.map_err(|error| format!("CleanupUnverified: entrée traitement: {error}"))?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("CleanupUnverified: type d'entrée: {error}"))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if file_type.is_symlink() {
            return Err(format!(
                "CleanupUnverified: lien symbolique conservé sans suivi: {}",
                entry.path().display()
            ));
        }
        let known = file_type.is_file() && KNOWN_GENERATION_FILES.contains(&name.as_str());
        let identity = identity_snapshot(inspect(&entry.path(), file_type.is_dir())?)?;
        objects.push(CleanupObject {
            relative_path: relative_to_root(root, &entry.path())?,
            identity,
            sha256: file_type
                .is_file()
                .then(|| sha256_file(&entry.path()).map(|sha| hex(&sha)))
                .transpose()?,
            owned: known,
            removed: false,
            error: (!known).then(|| "élément conservé : propriété non vérifiée".into()),
        });
    }
    objects.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    objects.push(CleanupObject {
        relative_path: relative_to_root(root, &directory)?,
        identity: generation_identity,
        sha256: None,
        owned: true,
        removed: false,
        error: None,
    });

    let ownership_path = installed_export_ownership_path(&directory);
    let ownership = match fs::read(&ownership_path) {
        Ok(bytes) => Some(
            serde_json::from_slice::<InstalledExportOwnership>(&bytes)
                .map_err(|error| format!("CleanupUnverified: preuve export invalide: {error}"))?,
        ),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(format!("CleanupUnverified: lire la preuve export: {error}"));
        }
    };
    if let Some(export_basename) = candidate.export_basename.as_deref() {
        let suffix = format!("{:032x}-{}", candidate.job_id.0, candidate.generation.get());
        for extension in ["txt", "srt"] {
            let candidates = [
                root.join("exports")
                    .join(format!("{export_basename}.{extension}")),
                root.join("transcriptions-remplacees")
                    .join(format!("{export_basename}-{suffix}.{extension}")),
            ];
            for path in candidates {
                let metadata = match fs::symlink_metadata(&path) {
                    Ok(metadata) => metadata,
                    Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                    Err(error) => {
                        return Err(format!(
                            "CleanupUnverified: inspecter export {}: {error}",
                            path.display()
                        ));
                    }
                };
                if metadata.file_type().is_symlink() || !metadata.is_file() {
                    return Err(format!(
                        "CleanupUnverified: export non régulier conservé: {}",
                        path.display()
                    ));
                }
                let parent = path
                    .parent()
                    .ok_or_else(|| "CleanupUnverified: export sans dossier parent".to_owned())?;
                capture_cleanup_ancestor(root, parent, &mut ancestors)?;
                let sha = hex(&sha256_file(&path)?);
                let identity = identity_snapshot(inspect(&path, false)?)?;
                let proof = ownership
                    .as_ref()
                    .filter(|record| {
                        matches!(record.version, 2 | 3)
                            && record.job_id == candidate.job_id
                            && record.generation == candidate.generation
                            && record.source_sha256 == candidate.source_sha256
                            && record.export_basename == export_basename
                    })
                    .and_then(|record| match extension {
                        "txt" => Some((
                            record.text_sha256.as_str(),
                            record.text_file_identity.as_deref(),
                        )),
                        "srt" => Some((
                            record.srt_sha256.as_str(),
                            record.srt_file_identity.as_deref(),
                        )),
                        _ => None,
                    });
                let owned = proof.is_some_and(|(expected_sha, expected_identity)| {
                    sha == expected_sha
                        && persisted_file_identity(&path).ok().as_deref() == expected_identity
                });
                objects.push(CleanupObject {
                    relative_path: relative_to_root(root, &path)?,
                    identity,
                    sha256: Some(sha),
                    owned,
                    removed: false,
                    error: (!owned).then(|| "export conservé : propriété non vérifiée".into()),
                });
            }
        }
    }
    Ok(CleanupTarget {
        treatment,
        source_sha256: candidate.source_sha256.clone(),
        ancestors,
        objects,
        container: None,
    })
}

#[cfg(windows)]
pub(in crate::archive) fn attach_empty_container_proofs(
    root: &Path,
    catalog: &[PublishedImport],
    selected: &[TreatmentIdentity],
    targets: &mut [CleanupTarget],
) -> Result<(), String> {
    use crate::windows_file_identity::inspect;

    let mut jobs = selected.iter().map(|item| item.job_id).collect::<Vec<_>>();
    jobs.sort_unstable_by_key(|job_id| job_id.0);
    jobs.dedup();
    for job_id in jobs {
        let selected_generations = selected
            .iter()
            .filter(|item| item.job_id == job_id)
            .map(|item| item.generation.get())
            .collect::<std::collections::HashSet<_>>();
        let container_path = root
            .join("transcriptions")
            .join(format!("{:032x}", job_id.0));
        let entries = fs::read_dir(&container_path)
            .map_err(|error| format!("CleanupUnverified: lister le conteneur: {error}"))?;
        let mut found_generations = std::collections::HashSet::new();
        let mut eligible = true;
        for entry in entries {
            let entry =
                entry.map_err(|error| format!("CleanupUnverified: entrée conteneur: {error}"))?;
            let file_type = entry
                .file_type()
                .map_err(|error| format!("CleanupUnverified: type conteneur: {error}"))?;
            let Some(generation) = entry
                .file_name()
                .to_str()
                .and_then(|name| name.parse::<u64>().ok())
                .filter(|generation| *generation > 0)
            else {
                eligible = false;
                break;
            };
            if !file_type.is_dir()
                || file_type.is_symlink()
                || !selected_generations.contains(&generation)
                || !catalog.iter().any(|candidate| {
                    candidate.job_id == job_id
                        && candidate.generation.get() == generation
                        && candidate.verified_present
                })
            {
                eligible = false;
                break;
            }
            found_generations.insert(generation);
        }
        if !eligible || found_generations != selected_generations {
            continue;
        }
        let identity = identity_snapshot(inspect(&container_path, true)?)?;
        let relative_path = relative_to_root(root, &container_path)?;
        let Some(target_index) = targets
            .iter()
            .rposition(|target| target.treatment.job_id == job_id)
        else {
            continue;
        };
        let target = &mut targets[target_index];
        if !target.ancestors.iter().any(|ancestor| {
            ancestor.relative_path == relative_path && ancestor.identity == identity
        }) {
            return Err("CleanupUnverified: identité du conteneur absente des ancêtres".into());
        }
        target.container = Some(CleanupObject {
            relative_path,
            identity,
            sha256: None,
            owned: true,
            removed: false,
            error: None,
        });
    }
    Ok(())
}

#[cfg(not(windows))]
pub(in crate::archive) fn attach_empty_container_proofs(
    _root: &Path,
    _catalog: &[PublishedImport],
    _selected: &[TreatmentIdentity],
    _targets: &mut [CleanupTarget],
) -> Result<(), String> {
    Ok(())
}

#[cfg(windows)]
fn capture_cleanup_ancestor(
    root: &Path,
    path: &Path,
    ancestors: &mut Vec<CleanupAncestor>,
) -> Result<(), String> {
    use crate::windows_file_identity::inspect;
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("CleanupUnverified: dossier parent: {error}"))?;
    let expected = root
        .canonicalize()
        .map_err(|error| format!("CleanupUnverified: destination: {error}"))?
        .join(relative_to_root(root, path)?);
    if canonical != expected {
        return Err(format!(
            "CleanupUnverified: parent redirigé: {}",
            path.display()
        ));
    }
    let relative_path = relative_to_root(root, path)?;
    if ancestors
        .iter()
        .any(|ancestor| ancestor.relative_path == relative_path)
    {
        return Ok(());
    }
    ancestors.push(CleanupAncestor {
        relative_path,
        identity: identity_snapshot(inspect(path, true)?)?,
    });
    Ok(())
}

#[cfg(windows)]
pub(in crate::archive) fn capture_cleanup_file(
    root: &Path,
    path: &Path,
) -> Result<CleanupObject, String> {
    use crate::windows_file_identity::inspect;
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("CleanupUnverified: artefact successeur absent: {error}"))?;
    let expected = root
        .canonicalize()
        .map_err(|error| format!("CleanupUnverified: destination: {error}"))?
        .join(relative_to_root(root, path)?);
    if canonical != expected {
        return Err(format!(
            "CleanupUnverified: artefact successeur redirigé: {}",
            path.display()
        ));
    }
    Ok(CleanupObject {
        relative_path: relative_to_root(root, path)?,
        identity: identity_snapshot(inspect(path, false)?)?,
        sha256: Some(hex(&sha256_file(path)?)),
        owned: true,
        removed: false,
        error: None,
    })
}

#[cfg(windows)]
pub(in crate::archive) fn capture_successor_files(
    root: &Path,
    directory: &Path,
) -> Result<Vec<CleanupObject>, String> {
    let manifest_path = directory.join("manifest.json");
    let manifest: CompleteRecord = serde_json::from_slice(
        &fs::read(&manifest_path)
            .map_err(|error| format!("CleanupUnverified: lire manifeste successeur: {error}"))?,
    )
    .map_err(|error| format!("CleanupUnverified: décoder manifeste successeur: {error}"))?;
    verify_artifact(directory, &manifest.text)?;
    verify_artifact(directory, &manifest.srt)?;
    [
        manifest_path,
        directory.join(&manifest.text.path),
        directory.join(&manifest.srt.path),
        directory.join("pending.json"),
        directory.join("segments.jsonl"),
    ]
    .iter()
    .map(|path| capture_cleanup_file(root, path))
    .collect()
}

#[cfg(windows)]
pub(in crate::archive) fn recover_published_cleanup_successor(
    root: &Path,
    plan: &mut CleanupPlan,
) -> Result<bool, String> {
    use crate::windows_file_identity::VerifiedFile;

    let canonical_root = root
        .canonicalize()
        .map_err(|error| format!("CleanupUnsafePath: destination: {error}"))?;
    if canonical_root.to_string_lossy() != plan.destination {
        return Err("CleanupUnsafePath: destination de reprise divergente".into());
    }
    let successor_root = Path::new(&plan.destination);
    let successor_path = generation_path(
        successor_root,
        plan.successor.job_id,
        plan.successor.generation,
    );
    if successor_path.canonicalize().ok().as_deref() != Some(successor_path.as_path()) {
        return Err("CleanupIdentityChanged: le nouveau traitement a été déplacé".into());
    }
    let successor_directory = VerifiedFile::open(&successor_path, true)?;
    successor_directory.verify(&runtime_identity(&plan.successor_directory))?;

    const PREPUBLICATION_FILES: [&str; 5] = [
        "manifest.json",
        "transcript.txt",
        "transcript.srt",
        "pending.json",
        "segments.jsonl",
    ];
    if plan.successor_files.len() != PREPUBLICATION_FILES.len() {
        return Err("CleanupIdentityChanged: preuves prépublication incomplètes".into());
    }
    let mut held = Vec::new();
    let mut manifest = None;
    let mut pending = None;
    let mut segments = None;
    for (proof, expected_name) in plan.successor_files.iter().zip(PREPUBLICATION_FILES) {
        let expected_path = generation_path(root, plan.successor.job_id, plan.successor.generation)
            .join(expected_name);
        if proof.relative_path != relative_to_root(root, &expected_path)? {
            return Err("CleanupIdentityChanged: chemin de preuve successeur inattendu".into());
        }
        let path = safe_cleanup_path(root, &proof.relative_path)?;
        let mut handle = VerifiedFile::open(&path, false)?;
        handle.verify(&runtime_identity(&proof.identity))?;
        let bytes = handle.read_all()?;
        if Some(hex(&Sha256::digest(&bytes))) != proof.sha256 {
            return Err(format!(
                "CleanupIdentityChanged: preuve successeur divergente {expected_name}"
            ));
        }
        match expected_name {
            "manifest.json" => {
                manifest = Some(serde_json::from_slice::<CompleteRecord>(&bytes).map_err(
                    |error| format!("CleanupIdentityChanged: manifeste successeur: {error}"),
                )?);
            }
            "pending.json" => {
                pending = Some(serde_json::from_slice::<PendingRecord>(&bytes).map_err(
                    |error| format!("CleanupIdentityChanged: pending successeur: {error}"),
                )?);
            }
            "segments.jsonl" => segments = Some(bytes),
            _ => {}
        }
        held.push(handle);
    }
    let manifest = manifest.expect("prepublication proof list contains manifest");
    let pending = pending.expect("prepublication proof list contains pending");
    if manifest.job_id != plan.successor.job_id
        || manifest.generation != plan.successor.generation
        || manifest.source_sha256 != plan.source_sha256
        || manifest.text.path != "transcript.txt"
        || manifest.srt.path != "transcript.srt"
        || pending.job_id != plan.successor.job_id
        || pending.generation != plan.successor.generation
        || pending.source_sha256 != plan.source_sha256
    {
        return Err("CleanupIdentityChanged: contrat du successeur divergent".into());
    }
    let segment_bytes = segments.expect("prepublication proof list contains segments");
    if !segment_bytes.is_empty() && !segment_bytes.ends_with(b"\n") {
        return Err("CleanupIdentityChanged: journal segments tronqué".into());
    }
    let parsed_segments = segment_bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| {
            serde_json::from_slice::<SegmentRecord>(line)
                .map_err(|error| format!("CleanupIdentityChanged: journal segments: {error}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if parsed_segments.len() as u64 != manifest.segments
        || parsed_segments
            .iter()
            .enumerate()
            .any(|(index, segment)| segment.sequence != index as u64 + 1)
    {
        return Err("CleanupIdentityChanged: journal segments incohérent".into());
    }

    let intent = load_publication_intent(&successor_path)?;
    if intent.cleanup_id.as_deref() != Some(plan.cleanup_id.as_str()) {
        return Err("CleanupIdentityChanged: identifiant d'intention divergent".into());
    }
    if intent.source_sha256 != plan.source_sha256 {
        return Err("CleanupIdentityChanged: empreinte de source d'intention divergente".into());
    }
    if !cleanup_targets_match_intent(&intent.cleanup_targets, &plan.targets) {
        return Err("CleanupIdentityChanged: cibles de l'intention divergentes".into());
    }

    let current_path =
        generation_path(root, plan.successor.job_id, plan.successor.generation).join("CURRENT");
    let mut current = match VerifiedFile::open(&current_path, false) {
        Ok(current) => current,
        Err(_) => return Ok(false),
    };
    if current.read_all()? != b"manifest.json\n" {
        return Err("CleanupIdentityChanged: CURRENT ne référence pas le manifeste attendu".into());
    }

    // CURRENT is the commit marker. If a crash occurred before the index write,
    // replaying this intent idempotently restores the successor's active row.
    let index = apply_publication_intent(
        load_or_migrate_publication_index(root)?,
        plan.successor.job_id,
        plan.successor.generation,
        &intent,
    );
    write_publication_index(root, &index)?;
    plan.successor_files.push(CleanupObject {
        relative_path: relative_to_root(root, &current_path)?,
        identity: identity_snapshot(current.identity.clone())?,
        sha256: Some(current.sha256()?),
        owned: true,
        removed: false,
        error: None,
    });
    plan.successor_index = Some(capture_cleanup_file(root, &publication_index_path(root))?);
    let _held_through_recovery = (successor_directory, held, current);
    Ok(true)
}

#[cfg(not(windows))]
pub(in crate::archive) fn recover_published_cleanup_successor(
    _root: &Path,
    _plan: &mut CleanupPlan,
) -> Result<bool, String> {
    Ok(false)
}

#[cfg(not(windows))]
pub(in crate::archive) fn capture_cleanup_target(
    _root: &Path,
    _candidate: &PublishedImport,
) -> Result<CleanupTarget, String> {
    Err("Unsupported: le nettoyage RTC-02 nécessite les handles Windows qualifiés".into())
}

#[cfg(windows)]
pub(in crate::archive) fn identity_snapshot(
    identity: crate::windows_file_identity::FileIdentity,
) -> Result<CleanupObjectIdentity, String> {
    Ok(CleanupObjectIdentity {
        volume_serial: identity.volume_serial,
        file_id: identity.file_id,
        is_directory: identity.is_directory,
    })
}

#[cfg(not(windows))]
pub(in crate::archive) fn identity_snapshot(
    _identity: (),
) -> Result<CleanupObjectIdentity, String> {
    unreachable!()
}

pub(in crate::archive) fn relative_to_root(root: &Path, path: &Path) -> Result<String, String> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| "CleanupUnsafePath: chemin hors destination".to_owned())?;
    if relative
        .components()
        .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err("CleanupUnsafePath: chemin relatif non canonique".into());
    }
    Ok(relative.to_string_lossy().into_owned())
}
