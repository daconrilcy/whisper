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
        if !allowed.contains(&root) {
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
    #[cfg(not(feature = "parakeet-cuda"))]
    if option_env!("WHISPER_DISTRIBUTION_EDITION_DUAL").is_some() {
        if let Ok(executable) = env::current_exe() {
            if executable
                .ancestors()
                .any(|path| path.file_name().is_some_and(|name| name == "WindowsApps"))
            {
                let Ok(executable) = executable.canonicalize() else {
                    return false;
                };
                let Some(program_files) = env::var_os("ProgramFiles").map(PathBuf::from) else {
                    return false;
                };
                let Ok(windows_apps) = program_files.join("WindowsApps").canonicalize() else {
                    return false;
                };
                return msix_cpu_package_root_is_allowed(root, &executable, &windows_apps);
            }
        }
    }

    let Some(local_app_data) = env::var_os("LOCALAPPDATA").map(PathBuf::from) else {
        return false;
    };
    let personal_root = local_app_data.join(personal_install_directory());
    let Ok(personal_root) = personal_root.canonicalize() else {
        return false;
    };
    personal_runtime_path_is_allowed(root, &personal_root)
}

#[cfg(all(feature = "parakeet-personal", not(feature = "parakeet-cuda")))]
fn msix_cpu_package_root_is_allowed(
    runtime_root: &Path,
    executable: &Path,
    windows_apps: &Path,
) -> bool {
    let Some(worker_directory) = executable.parent() else {
        return false;
    };
    let Some(package_root) = worker_directory.parent() else {
        return false;
    };
    worker_directory
        .file_name()
        .is_some_and(|name| name == "worker")
        && executable
            .file_name()
            .is_some_and(|name| name == "whisper-worker-parakeet-cpu.exe")
        && package_root != windows_apps
        && package_root.starts_with(windows_apps)
        && runtime_root == package_root
}

#[cfg(feature = "parakeet-personal")]
fn personal_install_directory() -> &'static str {
    if option_env!("WHISPER_DISTRIBUTION_EDITION_DUAL").is_some() {
        "WhisperTurbo"
    } else {
        "Whisper"
    }
}

#[cfg(feature = "parakeet-personal")]
fn personal_runtime_path_is_allowed(root: &Path, personal_root: &Path) -> bool {
    root == personal_root
        || (root.parent() == Some(personal_root.join(".staging").as_path())
            && root.file_name().is_some_and(|name| !name.is_empty()))
}

#[cfg(any(
    not(feature = "parakeet-personal"),
    all(test, feature = "parakeet-qualification-reference")
))]
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

#[cfg(any(
    not(feature = "parakeet-personal"),
    all(
        test,
        any(
            not(feature = "parakeet-cuda"),
            feature = "parakeet-qualification-reference"
        )
    )
))]
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
        let roots = ["l2a-1", "l2a-2"]
            .into_iter()
            .map(|lot| repository.join("target/parakeet-tests/l2").join(lot))
            .collect::<Vec<_>>();
        #[cfg(feature = "parakeet-qualification-reference")]
        let roots = {
            let mut roots = roots;
            roots.push(repository.join("target/parakeet-tests/l2/l2b"));
            roots
        };
        roots
    }
}

#[cfg(any(
    not(feature = "parakeet-personal"),
    all(
        test,
        any(
            not(feature = "parakeet-cuda"),
            feature = "parakeet-qualification-reference"
        )
    )
))]
fn allowed_runtime_roots(repository: &Path) -> Vec<PathBuf> {
    allowed_runtime_root_candidates(repository)
        .into_iter()
        .filter_map(|candidate| candidate.canonicalize().ok())
        .collect()
}

#[cfg(all(test, not(feature = "parakeet-cuda")))]
fn allowed_runtime_roots_for(repository: &Path, lots: &[&str]) -> Vec<PathBuf> {
    lots.iter()
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

#[cfg(all(test, feature = "parakeet-personal", not(feature = "parakeet-cuda")))]
#[path = "runtime_paths/msix_tests.rs"]
mod msix_tests;
#[cfg(all(test, feature = "parakeet-personal"))]
mod personal_tests;
#[cfg(test)]
mod tests;
