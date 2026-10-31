use super::{
    ArchiveStore, CleanupObject, CleanupPlan, CleanupTarget, Path, PathBuf, ReplacementCleanupInfo,
    fs, io, write_replace_synced,
};
mod execution;
mod proofs;
pub(super) use execution::run_cleanup_plan_locked;
use execution::safe_cleanup_path;
use proofs::recover_published_cleanup_successor;
pub(super) use proofs::{attach_empty_container_proofs, capture_cleanup_target};
#[cfg(windows)]
pub(super) use proofs::{capture_cleanup_file, capture_successor_files, identity_snapshot};

impl ArchiveStore {
    pub fn replacement_cleanups(root: &Path) -> Result<Vec<ReplacementCleanupInfo>, String> {
        let directory = cleanup_plans_path(root);
        if !directory.exists() {
            return Ok(Vec::new());
        }
        // Discovery may recover a publication that reached CURRENT but did
        // not finish writing its index/cleanup proof. Serialize that replay
        // with all other publication-index writers to prevent lost updates.
        let _lock = Self::lock_publications(root)?;
        let mut pending = Vec::new();
        for entry in fs::read_dir(&directory)
            .map_err(|error| format!("StorageUnavailable: lister les reprises: {error}"))?
        {
            let entry =
                entry.map_err(|error| format!("StorageUnavailable: entrée reprise: {error}"))?;
            if entry.path().extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let mut plan: CleanupPlan = serde_json::from_slice(
                &fs::read(entry.path())
                    .map_err(|error| format!("StorageCorrupt: lire la reprise: {error}"))?,
            )
            .map_err(|error| format!("StorageCorrupt: reprise JSON: {error}"))?;
            if !plan.published && recover_published_cleanup_successor(root, &mut plan)? {
                plan.published = true;
                write_cleanup_plan(root, &plan)?;
            }
            if !plan.published {
                continue;
            }
            let remaining = cleanup_remaining(&plan);
            let preserved = cleanup_preserved(root, &plan);
            if remaining.is_empty() && preserved.is_empty() {
                continue;
            }
            pending.push(ReplacementCleanupInfo {
                cleanup_id: plan.cleanup_id,
                successor: plan.successor,
                remaining,
                preserved,
                last_error: plan.last_error,
            });
        }
        pending.sort_by(|a, b| a.cleanup_id.cmp(&b.cleanup_id));
        Ok(pending)
    }

    pub fn resume_replacement_cleanup(root: &Path, cleanup_id: &str) -> Result<(), String> {
        let _lock = Self::lock_publications(root)?;
        let mut plan = read_cleanup_plan(root, cleanup_id)?;
        if !plan.published && recover_published_cleanup_successor(root, &mut plan)? {
            plan.published = true;
            write_cleanup_plan(root, &plan)?;
        }
        if !plan.published {
            return Err("CleanupNotPublished: le nouveau traitement n'est pas publié".into());
        }
        match run_cleanup_plan_locked(root, &mut plan) {
            Ok(()) => Ok(()),
            Err(error) => {
                plan.last_error = Some(error.clone());
                write_cleanup_plan(root, &plan)?;
                Err(error)
            }
        }
    }
}

pub(super) fn cleanup_plans_path(root: &Path) -> PathBuf {
    root.join(".whisper-publications")
        .join("replacement-cleanups")
}

pub(super) fn cleanup_plan_path(root: &Path, cleanup_id: &str) -> PathBuf {
    cleanup_plans_path(root).join(format!("{cleanup_id}.json"))
}

pub(super) fn read_cleanup_plan(root: &Path, cleanup_id: &str) -> Result<CleanupPlan, String> {
    if cleanup_id.is_empty()
        || !cleanup_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err("InvalidInput: identifiant de nettoyage invalide".into());
    }
    serde_json::from_slice(
        &fs::read(cleanup_plan_path(root, cleanup_id))
            .map_err(|error| format!("StorageUnavailable: lire la reprise: {error}"))?,
    )
    .map_err(|error| format!("StorageCorrupt: reprise de nettoyage: {error}"))
}

pub(super) fn write_cleanup_plan(root: &Path, plan: &CleanupPlan) -> Result<(), String> {
    if root
        .canonicalize()
        .map_err(|error| format!("CleanupUnsafePath: destination: {error}"))?
        .to_string_lossy()
        != plan.destination
    {
        return Err("CleanupUnsafePath: destination de reprise divergente".into());
    }
    let directory = cleanup_plans_path(root);
    fs::create_dir_all(&directory)
        .map_err(|error| format!("StorageUnavailable: dossier de reprise: {error}"))?;
    write_replace_synced(
        &cleanup_plan_path(root, &plan.cleanup_id),
        &serde_json::to_vec(plan).map_err(|error| error.to_string())?,
    )
}

fn cleanup_remaining(plan: &CleanupPlan) -> Vec<String> {
    plan.targets
        .iter()
        .flat_map(|target| target.objects.iter().chain(target.container.iter()))
        .filter(|object| object.owned && !object.removed)
        .map(|object| {
            object.error.as_ref().map_or_else(
                || object.relative_path.clone(),
                |error| format!("{} ({error})", object.relative_path),
            )
        })
        .collect()
}

pub(super) fn cleanup_targets_match_intent(
    intent: &[CleanupTarget],
    plan: &[CleanupTarget],
) -> bool {
    intent.len() == plan.len()
        && intent.iter().zip(plan).all(|(expected, actual)| {
            if expected.treatment != actual.treatment
                || expected.source_sha256 != actual.source_sha256
                || expected.ancestors != actual.ancestors
                || !cleanup_container_matches(
                    expected.container.as_ref(),
                    actual.container.as_ref(),
                )
            {
                return false;
            }

            let mut expected_objects = expected.objects.iter();
            let mut matched_expected = 0;
            let mut actual_paths = std::collections::HashSet::new();
            for actual_object in &actual.objects {
                if !actual_paths.insert(&actual_object.relative_path) {
                    return false;
                }
                if let Some(expected_object) = expected_objects.clone().next()
                    && expected_object.relative_path == actual_object.relative_path
                    && expected_object.identity == actual_object.identity
                    && expected_object.sha256 == actual_object.sha256
                    && expected_object.owned == actual_object.owned
                {
                    expected_objects.next();
                    matched_expected += 1;
                } else if actual_object.owned {
                    // A plan may gain late, unowned entries for reporting, but
                    // it must never gain or replace a deletion target.
                    return false;
                }
            }
            matched_expected == expected.objects.len()
        })
}

fn cleanup_container_matches(
    expected: Option<&CleanupObject>,
    actual: Option<&CleanupObject>,
) -> bool {
    match (expected, actual) {
        (None, None) => true,
        (Some(expected), Some(actual)) => {
            expected.relative_path == actual.relative_path
                && expected.identity == actual.identity
                && expected.sha256 == actual.sha256
                && (expected.owned == actual.owned
                    || (expected.owned && !actual.owned && !actual.removed))
        }
        _ => false,
    }
}

fn cleanup_preserved(root: &Path, plan: &CleanupPlan) -> Vec<String> {
    plan.targets
        .iter()
        .flat_map(|target| target.objects.iter().chain(target.container.iter()))
        .filter(|object| !object.owned && !object.removed)
        .filter(|object| {
            let Ok(path) = safe_cleanup_path(root, &object.relative_path) else {
                return true;
            };
            !matches!(
                fs::symlink_metadata(path),
                Err(ref error) if error.kind() == io::ErrorKind::NotFound
            )
        })
        .map(|object| {
            format!(
                "{} ({})",
                object.relative_path,
                object.error.as_deref().unwrap_or("propriété non vérifiée")
            )
        })
        .collect()
}
