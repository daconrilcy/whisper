use super::{ParakeetWorkerPaths, sha256_file};
use std::path::{Path, PathBuf};

#[cfg(feature = "recette-hooks")]
pub(super) fn validate_acceptance_runtime(expected: &Path) -> Result<(), String> {
    if option_env!("PARAKEET_L2_ACCEPTANCE_TEST_BUILD").is_none() {
        return Err("InvalidInput: acceptance transport seam is disabled".into());
    }
    let runtime = std::env::var_os("PARAKEET_L2_RUNTIME_ROOT")
        .ok_or_else(|| "MissingRuntime: acceptance runtime root is required".to_owned())?;
    let runtime = PathBuf::from(runtime)
        .canonicalize()
        .map_err(|error| format!("MissingRuntime: {error}"))?;
    if runtime.as_path() != expected {
        return Err("PathOutsideRuntime: acceptance runtime differs from worker root".into());
    }
    Ok(())
}

pub(super) fn is_shared_model(root: &Path, model: &Path) -> Result<bool, String> {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .map_err(|error| format!("RepositoryRoot: {error}"))?;
    let l2b_root = repository.join("target/parakeet-tests/l2/l2b");
    let l3b_root = repository.join("target/parakeet-tests/l3/l3b");
    Ok(repository
        .join("models/parakeet-tdt-0.6b-v3.q8_0.gguf")
        .canonicalize()
        .ok()
        .is_some_and(|expected| model == expected)
        && (l2b_root.canonicalize().ok().as_deref() == Some(root)
            || l3b_root.canonicalize().ok().as_deref() == Some(root)))
}

pub(super) fn validate_gpu_qualification(paths: &ParakeetWorkerPaths) -> Result<(), String> {
    let runtime = paths
        .runtime_root
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let expected = repository
        .join("target/parakeet-tests/l3/l3b")
        .canonicalize()
        .map_err(|error| error.to_string())?;
    if runtime != expected
        || std::env::var_os("PARAKEET_L2_WORKER")
            .and_then(|path| PathBuf::from(path).canonicalize().ok())
            != Some(
                paths
                    .executable
                    .canonicalize()
                    .map_err(|error| error.to_string())?,
            )
    {
        return Err(
            "ProfileUnavailable: GPU qualification requires the canonical L3b route".into(),
        );
    }
    Ok(())
}

pub(super) fn validate_qualification_runtime(paths: &ParakeetWorkerPaths) -> Result<(), String> {
    let runtime = paths
        .runtime_root
        .canonicalize()
        .map_err(|error| format!("MissingRuntime: {error}"))?;
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .map_err(|error| format!("RepositoryRoot: {error}"))?;
    let l2b_root = repository
        .join("target/parakeet-tests/l2/l2b")
        .canonicalize()
        .map_err(|error| format!("MissingRuntime: {error}"))?;
    let configured = std::env::var_os("PARAKEET_L2_RUNTIME_ROOT")
        .ok_or("MissingRuntime: PARAKEET_L2_RUNTIME_ROOT")?;
    if runtime != l2b_root || PathBuf::from(configured).canonicalize().ok() != Some(l2b_root) {
        return Err("PathOutsideRuntime: qualification profile requires canonical L2b root".into());
    }
    Ok(())
}

impl ParakeetWorkerPaths {
    pub(super) fn is_canonical_l3b_reference(&self, label: &str, canonical: &Path) -> bool {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .canonicalize();
        let Ok(repository) = repository else {
            return false;
        };
        let expected_runtime = repository.join("target/parakeet-tests/l3/l3b");
        let Ok(expected_runtime) = expected_runtime.canonicalize() else {
            return false;
        };
        if self.runtime_root.canonicalize().ok().as_ref() != Some(&expected_runtime)
            || std::env::var_os("PARAKEET_L2_RUNTIME_ROOT")
                .and_then(|path| PathBuf::from(path).canonicalize().ok())
                .as_ref()
                != Some(&expected_runtime)
        {
            return false;
        }
        let env_name = match label {
            "worker"
                if self
                    .executable
                    .file_name()
                    .is_some_and(|name| name == "whisper-worker-parakeet-gpu.exe") =>
            {
                "PARAKEET_L2_WORKER"
            }
            "worker" => "PARAKEET_L2_CPU_WORKER",
            "native directory"
                if self.native_dir.as_path()
                    == Path::new(
                        &std::env::var_os("PARAKEET_L2_NATIVE_DIR").unwrap_or_default(),
                    ) =>
            {
                "PARAKEET_L2_NATIVE_DIR"
            }
            "native directory" => "PARAKEET_L2_CPU_NATIVE_DIR",
            _ => return false,
        };
        let Some(configured) = std::env::var_os(env_name).map(PathBuf::from) else {
            return false;
        };
        configured
            .canonicalize()
            .ok()
            .is_some_and(|path| path == canonical)
    }

    #[cfg(feature = "recette-hooks")]
    pub(super) fn is_canonical_l3c_test_artifact(&self, canonical: &Path) -> bool {
        if !cfg!(feature = "l3-gpu-qualification")
            || option_env!("PARAKEET_L2_ACCEPTANCE_TEST_BUILD").is_none()
        {
            return false;
        }
        let Some(root) = l3_acceptance_execution_root() else {
            return false;
        };
        canonical.starts_with(root)
    }

    #[cfg(not(feature = "recette-hooks"))]
    pub(super) fn is_canonical_l3c_test_artifact(&self, _canonical: &Path) -> bool {
        false
    }

    pub(super) fn is_l2b_runtime(&self) -> Result<bool, String> {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .canonicalize()
            .map_err(|error| format!("RepositoryRoot: {error}"))?;
        let expected = repository.join("target/parakeet-tests/l2/l2b");
        Ok(
            self.runtime_root.canonicalize().ok() == expected.canonicalize().ok()
                && std::env::var_os("PARAKEET_L2_RUNTIME_ROOT")
                    .and_then(|root| PathBuf::from(root).canonicalize().ok())
                    == expected.canonicalize().ok(),
        )
    }
}

#[cfg(feature = "recette-hooks")]
fn l3_acceptance_execution_root() -> Option<PathBuf> {
    let l3c = std::env::var_os("PARAKEET_L3C_EXEC_ROOT").map(PathBuf::from);
    let l3d = std::env::var_os("PARAKEET_L3D_EXEC_ROOT").map(PathBuf::from);
    let (configured, lot) = match (l3c, l3d) {
        (Some(path), None) => (path, "l3c"),
        (None, Some(path)) => (path, "l3d"),
        _ => return None,
    };
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .ok()?;
    let expected = repository
        .join(format!("target/parakeet-tests/l3/{lot}"))
        .canonicalize()
        .ok()?;
    let configured = configured.canonicalize().ok()?;
    (configured == expected).then_some(expected)
}

/// The immutable R6 CPU worker authorizes only the canonical L2b runtime. During L3b,
/// keep adapter-owned writes under L3b while giving that exact worker its read-only L2b root.
/// The environment inputs are accepted only for the exact frozen worker and canonical roots.
pub(super) fn l3b_r6_cpu_read_runtime_root(
    paths: &ParakeetWorkerPaths,
) -> Result<Option<PathBuf>, String> {
    if paths.executable.file_name() != Some(std::ffi::OsStr::new("whisper-worker-parakeet-cpu.exe"))
    {
        return Ok(None);
    }
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .map_err(|error| format!("RepositoryRoot: {error}"))?;
    l3b_r6_cpu_read_runtime_root_in_repository(
        paths,
        &repository,
        std::env::var_os("PARAKEET_L2_CPU_WORKER").map(PathBuf::from),
        std::env::var_os("PARAKEET_L2_CPU_RUNTIME_ROOT").map(PathBuf::from),
    )
}

pub(super) fn l3b_r6_cpu_read_runtime_root_in_repository(
    paths: &ParakeetWorkerPaths,
    repository: &Path,
    configured_worker: Option<PathBuf>,
    configured_root: Option<PathBuf>,
) -> Result<Option<PathBuf>, String> {
    const R6_CPU_WORKER_SHA256: &str =
        "1d8f57b9e92d69629618be422077a0508303b9ae10d622b5f5aaad8867e103c0";

    let active_root = paths
        .runtime_root
        .canonicalize()
        .map_err(|error| format!("RuntimeRootInvalid: active: {error}"))?;
    let l3b_root = match repository
        .join("target/parakeet-tests/l3/l3b")
        .canonicalize()
    {
        Ok(root) => root,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("RuntimeRootInvalid: L3b: {error}")),
    };
    if active_root != l3b_root {
        return Ok(None);
    }

    let l2b_root = repository
        .join("target/parakeet-tests/l2/l2b")
        .canonicalize()
        .map_err(|error| format!("RuntimeRootInvalid: L2b: {error}"))?;
    let expected_worker = l2b_root.join("r6/worker/whisper-worker-parakeet-cpu.exe");
    let worker = paths
        .executable
        .canonicalize()
        .map_err(|error| format!("WorkerPath: {error}"))?;
    if worker != expected_worker {
        return Ok(None);
    }

    let configured_worker = configured_worker
        .ok_or_else(|| {
            "WorkerPathMissing: PARAKEET_L2_CPU_WORKER is required for the frozen R6 worker"
                .to_owned()
        })?
        .canonicalize()
        .map_err(|error| format!("WorkerPathInvalid: PARAKEET_L2_CPU_WORKER: {error}"))?;
    if configured_worker != worker {
        return Err(
            "WorkerPathMismatch: frozen R6 worker differs from configured CPU worker".into(),
        );
    }
    let worker_sha256 = sha256_file(&worker)?;
    if !worker_sha256.eq_ignore_ascii_case(R6_CPU_WORKER_SHA256) {
        return Err("WorkerHashMismatch: frozen R6 CPU worker SHA-256 differs".into());
    }

    let configured_root = configured_root
        .ok_or_else(|| {
            "RuntimeRootMissing: PARAKEET_L2_CPU_RUNTIME_ROOT is required for the frozen R6 worker"
                .to_owned()
        })?
        .canonicalize()
        .map_err(|error| format!("RuntimeRootInvalid: PARAKEET_L2_CPU_RUNTIME_ROOT: {error}"))?;
    validate_r6_read_root(&configured_root, &l2b_root)?;
    Ok(Some(l2b_root))
}

pub(super) fn validate_r6_read_root(configured_root: &Path, l2b_root: &Path) -> Result<(), String> {
    if configured_root != l2b_root {
        return Err(
            "RuntimeRootMismatch: frozen R6 CPU worker requires canonical L2b read root".into(),
        );
    }
    Ok(())
}

pub(super) fn historical_log_root(
    runtime: &Path,
    is_gpu_worker: bool,
) -> Result<(PathBuf, bool, bool), String> {
    let configured_l3c = if let Some(configured) = std::env::var_os("PARAKEET_L3C_EXEC_ROOT") {
        let configured = PathBuf::from(configured)
            .canonicalize()
            .map_err(|error| format!("WorkerLogPath: L3c execution root: {error}"))?;
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .canonicalize()
            .map_err(|error| format!("WorkerLogPath: repository root: {error}"))?;
        let expected = repository
            .join("target/parakeet-tests/l3/l3c")
            .canonicalize()
            .map_err(|error| format!("WorkerLogPath: canonical L3c root: {error}"))?;
        if configured != expected {
            return Err("PathOutsideRuntime: L3c execution root is not canonical".into());
        }
        Some(configured)
    } else {
        None
    };
    let configured_l3d = if let Some(configured) = std::env::var_os("PARAKEET_L3D_EXEC_ROOT") {
        let configured = PathBuf::from(configured)
            .canonicalize()
            .map_err(|error| format!("WorkerLogPath: L3d execution root: {error}"))?;
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .canonicalize()
            .map_err(|error| format!("WorkerLogPath: repository root: {error}"))?;
        let expected = repository
            .join("target/parakeet-tests/l3/l3d")
            .canonicalize()
            .map_err(|error| format!("WorkerLogPath: canonical L3d root: {error}"))?;
        if configured != expected {
            return Err("PathOutsideRuntime: L3d execution root is not canonical".into());
        }
        Some(configured)
    } else {
        None
    };
    if configured_l3c.is_some() && configured_l3d.is_some() {
        return Err("PathOutsideRuntime: only one L3 campaign root may be configured".into());
    }
    let l3_gpu_log = is_gpu_worker && (configured_l3c.is_some() || configured_l3d.is_some());
    let configured_campaign = configured_l3d.clone().or(configured_l3c.clone());
    let log_root = if is_gpu_worker && configured_campaign.is_some() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .canonicalize()
            .map_err(|error| format!("WorkerLogPath: repository root: {error}"))?;
        let expected_runtime = repository
            .join("target/parakeet-tests/l3/l3b")
            .canonicalize()
            .map_err(|error| format!("WorkerLogPath: canonical L3b root: {error}"))?;
        if runtime != expected_runtime.as_path() || !l3_gpu_log {
            return Err(
                "PathOutsideRuntime: GPU ASR log requires canonical L3b/L3c/L3d roots".into(),
            );
        }
        runtime.to_path_buf()
    } else {
        configured_campaign.unwrap_or_else(|| runtime.to_path_buf())
    };
    Ok((log_root, configured_l3c.is_some(), configured_l3d.is_some()))
}
