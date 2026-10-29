use std::{
    env,
    path::{Path, PathBuf},
};
use whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS;

pub(crate) fn shared_model_runtime_allowed(runtime: &Path, repository: &Path) -> bool {
    let l2b = repository
        .join("target/parakeet-tests/l2/l2b")
        .canonicalize()
        .ok();
    if l2b.as_deref() == Some(runtime) {
        return true;
    }
    #[cfg(feature = "parakeet-cuda")]
    {
        return ["l3a", "l3b"].into_iter().any(|lot| {
            repository
                .join("target/parakeet-tests/l3")
                .join(lot)
                .canonicalize()
                .ok()
                .as_deref()
                == Some(runtime)
        });
    }
    #[cfg(not(feature = "parakeet-cuda"))]
    false
}

pub(crate) fn required_runtime_root(profile: &str) -> Result<PathBuf, String> {
    let root = env::var_os("PARAKEET_L2_RUNTIME_ROOT")
        .ok_or("MissingRuntime: PARAKEET_L2_RUNTIME_ROOT")?;
    let root = PathBuf::from(root)
        .canonicalize()
        .map_err(|e| format!("RuntimeRoot: {e}"))?;
    #[cfg(feature = "parakeet-personal")]
    {
        if !personal_runtime_root_is_allowed(&root) {
            return Err(
                "RuntimeRootMismatch: worker runtime is outside the personal installation".into(),
            );
        }
        if profile != PARAKEET_PROFILE_IMPORT_SINGLE_PASS {
            return Err(
                "ProfileUnavailable: personal workers support single-pass import only".into(),
            );
        }
    }
    #[cfg(not(feature = "parakeet-personal"))]
    {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
        let allowed = allowed_runtime_roots(&repository);
        if !allowed.iter().any(|candidate| root == *candidate) {
            return Err(
                "RuntimeRootMismatch: worker runtime is outside authorized Test roots".into(),
            );
        }
        if profile != PARAKEET_PROFILE_IMPORT_SINGLE_PASS
            && !qualification_root_is_valid(profile, &root, &repository)
        {
            return Err(
                "RuntimeRootMismatch: qualification profiles require the L2b runtime root".into(),
            );
        }
    }
    Ok(root)
}

#[cfg(feature = "parakeet-personal")]
fn personal_runtime_root_is_allowed(root: &Path) -> bool {
    let Some(local_app_data) = env::var_os("LOCALAPPDATA").map(PathBuf::from) else {
        return false;
    };
    let personal_root = local_app_data.join("Whisper");
    let Ok(personal_root) = personal_root.canonicalize() else {
        return false;
    };
    personal_runtime_path_is_allowed(root, &personal_root)
}

#[cfg(feature = "parakeet-personal")]
fn personal_runtime_path_is_allowed(root: &Path, personal_root: &Path) -> bool {
    root == personal_root
        || (root.parent() == Some(personal_root.join(".staging").as_path())
            && root.file_name().is_some_and(|name| !name.is_empty()))
}

fn qualification_root_is_valid(profile: &str, root: &Path, repository: &Path) -> bool {
    if profile == PARAKEET_PROFILE_IMPORT_SINGLE_PASS {
        return allowed_runtime_roots(repository).contains(&root.to_path_buf());
    }
    repository
        .join("target/parakeet-tests/l2/l2b")
        .canonicalize()
        .ok()
        .as_ref()
        == Some(&root.to_path_buf())
}

fn allowed_runtime_root_candidates(repository: &Path) -> Vec<PathBuf> {
    #[cfg(feature = "parakeet-personal")]
    {
        let _ = repository;
        return Vec::new();
    }
    #[cfg(all(feature = "parakeet-cuda", not(feature = "parakeet-personal")))]
    {
        return ["l3a", "l3b"]
            .into_iter()
            .map(|lot| repository.join("target/parakeet-tests/l3").join(lot))
            .collect();
    }
    #[cfg(all(not(feature = "parakeet-cuda"), not(feature = "parakeet-personal")))]
    {
        let mut roots = ["l2a-1", "l2a-2"]
            .into_iter()
            .map(|lot| repository.join("target/parakeet-tests/l2").join(lot))
            .collect::<Vec<_>>();
        #[cfg(feature = "parakeet-qualification-reference")]
        roots.push(repository.join("target/parakeet-tests/l2/l2b"));
        roots
    }
}

fn allowed_runtime_roots(repository: &Path) -> Vec<PathBuf> {
    allowed_runtime_root_candidates(repository)
        .into_iter()
        .filter_map(|candidate| candidate.canonicalize().ok())
        .collect()
}

fn allowed_runtime_roots_for(repository: &Path, lots: &[&str]) -> Vec<PathBuf> {
    lots.into_iter()
        .filter_map(|lot| {
            repository
                .join("target")
                .join("parakeet-tests")
                .join("l2")
                .join(lot)
                .canonicalize()
                .ok()
        })
        .collect()
}

pub(crate) fn ensure_inside(path: &Path, root: &Path) -> Result<(), String> {
    if !path.starts_with(root) {
        return Err("PathOutsideRuntime: native directory outside runtime root".into());
    }
    Ok(())
}

#[cfg(all(test, feature = "parakeet-personal"))]
mod personal_tests;
#[cfg(test)]
mod tests;
