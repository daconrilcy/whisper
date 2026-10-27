//! Isolated process client for the qualified Parakeet import worker.

use sha2::{Digest, Sha256};
use std::{
    fs::OpenOptions,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver, SyncSender, TryRecvError},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use whisper_core::parakeet_ipc::{
    Command as WorkerCommand, Envelope, Event, PARAKEET_IPC_VERSION, PARAKEET_MAX_FRAME_BYTES,
    PARAKEET_PROFILE_IMPORT_SINGLE_PASS, WorkerIdentity,
};
use whisper_core::parakeet_windows::{PROFILE_IMPORT_VAD, PROFILE_QUALIFICATION_REFERENCE};

#[derive(Clone, Debug)]
pub struct ParakeetWorkerPaths {
    pub executable: PathBuf,
    pub native_dir: PathBuf,
    pub model: PathBuf,
    pub runtime_root: PathBuf,
}

impl ParakeetWorkerPaths {
    pub fn validate(&self) -> Result<(), String> {
        let root = self
            .runtime_root
            .canonicalize()
            .map_err(|error| format!("MissingRuntime: {error}"))?;
        if self.matches_personal_layout(&root) {
            return Ok(());
        }
        for (label, path) in [
            ("worker", self.executable.as_path()),
            ("native directory", self.native_dir.as_path()),
        ] {
            let canonical = path
                .canonicalize()
                .map_err(|error| format!("{label} unavailable: {error}"))?;
            if !canonical.starts_with(&root)
                && !self.is_canonical_l3b_reference(label, &canonical)
                && !self.is_canonical_l3c_test_artifact(&canonical)
            {
                return Err(format!(
                    "PathOutsideRuntime: {label} is outside the active Parakeet runtime"
                ));
            }
        }
        let model = self
            .model
            .canonicalize()
            .map_err(|error| format!("model unavailable: {error}"))?;
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .canonicalize()
            .map_err(|error| format!("RepositoryRoot: {error}"))?;
        let l2b_root = repository.join("target/parakeet-tests/l2/l2b");
        let l3b_root = repository.join("target/parakeet-tests/l3/l3b");
        let shared_model = repository
            .join("models/parakeet-tdt-0.6b-v3.q8_0.gguf")
            .canonicalize()
            .ok()
            .is_some_and(|expected| model == expected)
            && (l2b_root.canonicalize().ok().as_ref() == Some(&root)
                || l3b_root.canonicalize().ok().as_ref() == Some(&root));
        if !model.starts_with(&root) && !shared_model {
            return Err("PathOutsideRuntime: model must be under runtime or the read-only qualified models directory".into());
        }
        Ok(())
    }

    fn matches_personal_layout(&self, root: &Path) -> bool {
        let Some(local_app_data) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) else {
            return false;
        };
        let Ok(personal_root) = local_app_data.join("Whisper").canonicalize() else {
            return false;
        };
        if !personal_runtime_path_is_allowed(root, &personal_root) {
            return false;
        }
        let (worker_name, native_name) = if self
            .executable
            .file_name()
            .is_some_and(|name| name == "whisper-worker-parakeet-cpu.exe")
        {
            ("whisper-worker-parakeet-cpu.exe", "cpu")
        } else if self
            .executable
            .file_name()
            .is_some_and(|name| name == "whisper-worker-parakeet-gpu.exe")
        {
            ("whisper-worker-parakeet-gpu.exe", "gpu")
        } else {
            return false;
        };
        self.executable.canonicalize().ok().as_deref()
            == Some(root.join("worker").join(worker_name).as_path())
            && self.native_dir.canonicalize().ok().as_deref()
                == Some(root.join("native").join(native_name).as_path())
            && self.model.canonicalize().ok().as_deref()
                == Some(root.join("models/parakeet-tdt-0.6b-v3.q8_0.gguf").as_path())
    }

    pub(crate) fn is_personal_runtime(&self) -> bool {
        self.runtime_root
            .canonicalize()
            .ok()
            .is_some_and(|root| self.matches_personal_layout(&root))
    }

    fn is_canonical_l3b_reference(&self, label: &str, canonical: &Path) -> bool {
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
                if self.native_dir
                    == PathBuf::from(
                        std::env::var_os("PARAKEET_L2_NATIVE_DIR").unwrap_or_default(),
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

    fn is_canonical_l3c_test_artifact(&self, canonical: &Path) -> bool {
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

    pub fn verify_qualification(
        &self,
        expected_model_sha256: &str,
        expected_runtime_sha256: &str,
        expected_worker_sha256: &str,
    ) -> Result<(), String> {
        self.validate()?;
        let (_, model_sha) = crate::model_catalog::cached_model_sha256(&self.model)
            .map_err(|error| format!("ModelHashRead: {error}"))?;
        let model_sha = model_sha
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        self.verify_worker_runtime(expected_runtime_sha256, expected_worker_sha256)?;
        if !model_sha.eq_ignore_ascii_case(expected_model_sha256) {
            return Err(format!("QualificationMismatch: model={model_sha}"));
        }
        Ok(())
    }

    /// Builds the exact per-run archive key for the private L2b Test candidate.
    /// The profile is kept in the key so VAD pending work cannot resume as single-pass.
    pub fn qualification_key(
        &self,
        profile: &str,
    ) -> Result<crate::archive::ArchiveQualificationKey, String> {
        if ![PARAKEET_PROFILE_IMPORT_SINGLE_PASS, PROFILE_IMPORT_VAD].contains(&profile) {
            return Err("ProfileUnavailable: archive key profile is not an import profile".into());
        }
        self.validate()?;
        if self.is_personal_runtime() {
            if profile != PARAKEET_PROFILE_IMPORT_SINGLE_PASS {
                return Err(
                    "ProfileUnavailable: personal workers support single-pass import only".into(),
                );
            }
            return self.personal_qualification_key("CPU");
        }
        if !self.is_l2b_runtime()? {
            if profile == PARAKEET_PROFILE_IMPORT_SINGLE_PASS {
                return Ok(crate::archive::ArchiveQualificationKey::parakeet_cpu_test());
            }
            return Err(
                "ProfileUnavailable: import-vad requires the isolated L2b Test runtime".into(),
            );
        }
        let record = whisper_core::model_registry::model(
            whisper_core::model_catalog::ModelId::ParakeetTdt06BV3,
        );
        let (_, model_sha) = crate::model_catalog::cached_model_sha256(&self.model)
            .map_err(|error| format!("ModelHashRead: {error}"))?;
        let model_sha = model_sha
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        if !model_sha.eq_ignore_ascii_case(record.artifact.sha256) {
            return Err(format!("ModelHashMismatch: {model_sha}"));
        }
        let mut key = crate::archive::ArchiveQualificationKey::parakeet_cpu_test();
        key.artifact_revision = model_sha;
        key.runtime_build = runtime_sha256(&self.native_dir)?;
        key.worker_build = sha256_file(&self.executable)?;
        key.inference_profile = profile.to_owned();
        Ok(key)
    }

    pub fn gpu_qualification_key(
        &self,
        profile: &str,
    ) -> Result<crate::archive::ArchiveQualificationKey, String> {
        if profile != PARAKEET_PROFILE_IMPORT_SINGLE_PASS {
            return Err(
                "ProfileUnavailable: GPU key is available for single-pass import only".into(),
            );
        }
        self.validate()?;
        if self.is_personal_runtime() {
            return self.personal_qualification_key("CUDA0");
        }
        let runtime = self
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
                    self.executable
                        .canonicalize()
                        .map_err(|error| error.to_string())?,
                )
        {
            return Err(
                "ProfileUnavailable: GPU qualification requires the canonical L3b route".into(),
            );
        }
        let record = whisper_core::model_registry::model(
            whisper_core::model_catalog::ModelId::ParakeetTdt06BV3,
        );
        let (_, model_sha) = crate::model_catalog::cached_model_sha256(&self.model)
            .map_err(|error| format!("ModelHashRead: {error}"))?;
        let model_sha = model_sha
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        if !model_sha.eq_ignore_ascii_case(record.artifact.sha256) {
            return Err(format!("ModelHashMismatch: {model_sha}"));
        }
        let mut key = crate::archive::ArchiveQualificationKey::parakeet_gpu_test();
        key.artifact_revision = model_sha;
        key.runtime_build = runtime_sha256(&self.native_dir)?;
        key.worker_build = sha256_file(&self.executable)?;
        key.inference_profile = profile.to_owned();
        Ok(key)
    }

    fn personal_qualification_key(
        &self,
        backend: &str,
    ) -> Result<crate::archive::ArchiveQualificationKey, String> {
        let record = whisper_core::model_registry::model(
            whisper_core::model_catalog::ModelId::ParakeetTdt06BV3,
        );
        let (_, model_sha) = crate::model_catalog::cached_model_sha256(&self.model)
            .map_err(|error| format!("ModelHashRead: {error}"))?;
        let model_sha = model_sha
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        if !model_sha.eq_ignore_ascii_case(record.artifact.sha256) {
            return Err(format!("ModelHashMismatch: {model_sha}"));
        }
        let mut key = crate::archive::ArchiveQualificationKey::parakeet_cpu_test();
        key.artifact_revision = model_sha;
        key.runtime_build = runtime_sha256(&self.native_dir)?;
        key.worker_build = sha256_file(&self.executable)?;
        key.inference_profile = PARAKEET_PROFILE_IMPORT_SINGLE_PASS.to_owned();
        key.backend = backend.to_owned();
        if backend == "CUDA0" {
            key.gpu_pci_bus_id =
                Some(whisper_core::model_registry::PARAKEET_PERSONAL_GPU_PCI_BUS_ID.to_owned());
            key.gpu_uuid =
                Some(whisper_core::model_registry::PARAKEET_PERSONAL_GPU_UUID.to_owned());
            key.gpu_pnp_instance_id = Some(
                whisper_core::model_registry::PARAKEET_PERSONAL_GPU_PNP_INSTANCE_ID.to_owned(),
            );
        }
        Ok(key)
    }

    #[cfg(feature = "personal-qualification")]
    #[doc(hidden)]
    pub fn personal_candidate_keys_from_env() -> Vec<crate::archive::ArchiveQualificationKey> {
        let (Some(runtime_root), Some(model)) = (
            std::env::var_os("PARAKEET_L2_RUNTIME_ROOT"),
            std::env::var_os("PARAKEET_L2_MODEL"),
        ) else {
            return Vec::new();
        };
        let runtime_root = PathBuf::from(runtime_root);
        let model = PathBuf::from(model);
        let mut candidates = Vec::new();
        for (backend, worker_variable, native_variable) in [
            (
                "CPU",
                "PARAKEET_L2_CPU_WORKER",
                "PARAKEET_L2_CPU_NATIVE_DIR",
            ),
            (
                "CUDA0",
                "PARAKEET_L6P_GPU_WORKER",
                "PARAKEET_L6P_GPU_NATIVE_DIR",
            ),
        ] {
            let Some(worker) = std::env::var_os(worker_variable).map(PathBuf::from) else {
                continue;
            };
            let Some(native_dir) = std::env::var_os(native_variable).map(PathBuf::from) else {
                continue;
            };
            let paths = Self {
                executable: worker,
                native_dir,
                model: model.clone(),
                runtime_root: runtime_root.clone(),
            };
            if paths.validate().is_ok()
                && let Ok(key) = paths.personal_qualification_key(backend)
            {
                candidates.push(key);
            }
        }
        candidates
    }

    fn is_l2b_runtime(&self) -> Result<bool, String> {
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

    /// Rechecks the mutable native runtime key without rehashing the large model artifact.
    pub fn verify_worker_runtime(
        &self,
        expected_runtime_sha256: &str,
        expected_worker_sha256: &str,
    ) -> Result<(), String> {
        self.validate()?;
        let worker_sha = sha256_file(&self.executable)?;
        let runtime_sha = runtime_sha256(&self.native_dir)?;
        if !worker_sha.eq_ignore_ascii_case(expected_worker_sha256)
            || !runtime_sha.eq_ignore_ascii_case(expected_runtime_sha256)
        {
            return Err(format!(
                "QualificationMismatch: runtime={runtime_sha}, worker={worker_sha}"
            ));
        }
        Ok(())
    }
}

fn personal_runtime_path_is_allowed(root: &Path, personal_root: &Path) -> bool {
    root == personal_root
        || (root.parent() == Some(personal_root.join(".staging").as_path())
            && root.file_name().is_some_and(|name| !name.is_empty()))
}

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

fn runtime_sha256(native_dir: &Path) -> Result<String, String> {
    let mut hasher = Sha256::new();
    hasher.update(b"nemo-speech-asr 0.2.0");
    for name in [
        "nemo_speech_asr_c.dll",
        "nemo_speech_asr.dll",
        "ggml.dll",
        "ggml-base.dll",
        "ggml-cpu.dll",
    ] {
        hasher.update(name.as_bytes());
        hasher.update(sha256_file(&native_dir.join(name))?.as_bytes());
    }
    for name in [
        "ggml-cuda.dll",
        "cudart64_12.dll",
        "cublas64_12.dll",
        "cublasLt64_12.dll",
    ] {
        let path = native_dir.join(name);
        if path.is_file() {
            hasher.update(name.as_bytes());
            hasher.update(sha256_file(&path)?.as_bytes());
        }
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub(crate) fn sha256_file(path: &Path) -> Result<String, String> {
    let started = Instant::now();
    let (bytes, digest, full_read) = crate::model_catalog::cached_artifact_sha256_with_read(path)
        .map_err(|error| format!("HashRead: {error}"))?;
    let sha = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    crate::worker_ipc::record_r7_timing(
        "sha256",
        started.elapsed(),
        serde_json::json!({
            "file": path.file_name().and_then(|name| name.to_str()).unwrap_or("unknown"),
            "role": if path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("dll")) { "runtime_dll" } else if path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("gguf")) { "model" } else if path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("wav")) { "audio" } else if path.file_name().is_some_and(|name| name.to_string_lossy().contains("worker")) { "worker" } else { "other" },
            "bytes": bytes,
            "sha256": sha,
            "full_read": full_read,
        }),
    );
    Ok(sha)
}

pub struct ParakeetProcess {
    child: Child,
    input: ChildStdin,
    events: Receiver<Result<Event, String>>,
    acceptance_events: Receiver<Result<Event, String>>,
    acceptance_sender: SyncSender<Result<Event, String>>,
    runtime_root: PathBuf,
    identity: Option<WorkerIdentity>,
    expected_profile: String,
    request_id: u64,
    spawned_at: Instant,
    spawn_elapsed: Duration,
    request_started_at: Option<Instant>,
}

impl ParakeetProcess {
    pub fn spawn(paths: &ParakeetWorkerPaths) -> Result<Self, String> {
        Self::spawn_profile(paths, PARAKEET_PROFILE_IMPORT_SINGLE_PASS)
    }

    /// Starts the unchanged import worker while keeping its diagnostic files with this session.
    pub(crate) fn spawn_live(
        paths: &ParakeetWorkerPaths,
        session_log_root: &Path,
    ) -> Result<Self, String> {
        // The GPU worker requires its ASR evidence log beneath its runtime root.
        // Reuse the import path for GPU; retain the session log for CPU.
        let log_root = (paths.executable.file_name()
            != Some(std::ffi::OsStr::new("whisper-worker-parakeet-gpu.exe")))
        .then_some(session_log_root);
        Self::spawn_profile_with_log_root(paths, PARAKEET_PROFILE_IMPORT_SINGLE_PASS, log_root)
    }

    /// Private L2b harness entry point. It cannot be used outside the canonical L2b runtime.
    #[doc(hidden)]
    pub fn spawn_for_qualification(
        paths: &ParakeetWorkerPaths,
        profile: &str,
    ) -> Result<Self, String> {
        if ![PROFILE_IMPORT_VAD, PROFILE_QUALIFICATION_REFERENCE].contains(&profile) {
            return Err("ProfileUnavailable: unsupported private qualification profile".into());
        }
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
            return Err(
                "PathOutsideRuntime: qualification profile requires canonical L2b root".into(),
            );
        }
        Self::spawn_profile(paths, profile)
    }

    fn spawn_profile(paths: &ParakeetWorkerPaths, profile: &str) -> Result<Self, String> {
        Self::spawn_profile_with_log_root(paths, profile, None)
    }

    fn spawn_profile_with_log_root(
        paths: &ParakeetWorkerPaths,
        profile: &str,
        session_log_root: Option<&Path>,
    ) -> Result<Self, String> {
        paths.validate()?;
        let model_sha256 = whisper_core::model_registry::model(
            whisper_core::model_catalog::ModelId::ParakeetTdt06BV3,
        )
        .artifact
        .sha256;
        let mut command = Command::new(&paths.executable);
        command
            .arg("--model")
            .arg(&paths.model)
            .arg("--model-sha256")
            .arg(model_sha256)
            .arg("--native-dir")
            .arg(&paths.native_dir);
        if profile == PROFILE_IMPORT_VAD {
            command.args(["--profile", PROFILE_IMPORT_VAD]);
        } else if profile == PROFILE_QUALIFICATION_REFERENCE {
            command.arg("--qualification-reference");
        }
        let cpu_read_runtime = l3b_r6_cpu_read_runtime_root(paths)?;
        let (stderr, worker_log) =
            worker_stderr_log(paths, cpu_read_runtime.is_some(), session_log_root)?;
        let worker_runtime_root = cpu_read_runtime.unwrap_or_else(|| paths.runtime_root.clone());
        if paths
            .executable
            .file_name()
            .is_some_and(|name| name == "whisper-worker-parakeet-gpu.exe")
            && let Some(asr_log) = worker_log.as_ref()
        {
            command.env("PARAKEET_L3_ASR_LOG", asr_log);
        }
        let spawn_started = Instant::now();
        let mut child = command
            .env("PARAKEET_L2_RUNTIME_ROOT", worker_runtime_root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(stderr)
            .spawn()
            .map_err(|error| format!("WorkerStart: {error}"))?;
        let spawn_elapsed = spawn_started.elapsed();
        crate::worker_ipc::record_r7_timing(
            "worker_spawn",
            spawn_elapsed,
            serde_json::json!({
                "worker": paths.executable.file_name().and_then(|name| name.to_str()).unwrap_or("unknown"),
            }),
        );
        let input = child
            .stdin
            .take()
            .ok_or_else(|| "WorkerStart: stdin unavailable".to_owned())?;
        let output = child
            .stdout
            .take()
            .ok_or_else(|| "WorkerStart: stdout unavailable".to_owned())?;
        let (sender, events) = mpsc::sync_channel(2);
        let (acceptance_sender, acceptance_events) = mpsc::sync_channel(1);
        thread::spawn(move || {
            let mut reader = BufReader::new(output);
            loop {
                let mut frame = Vec::new();
                match reader.read_until(b'\n', &mut frame) {
                    Ok(0) => break,
                    Ok(length) if length <= PARAKEET_MAX_FRAME_BYTES => {
                        if frame.last() == Some(&b'\n') {
                            frame.pop();
                        } else {
                            let _ = sender.send(Err("IpcTruncated: missing newline".into()));
                            break;
                        }
                        let decoded = decode_worker_frame(&frame);
                        if sender.send(decoded).is_err() {
                            break;
                        }
                    }
                    Ok(_) => {
                        let _ = sender.send(Err("IpcFrameTooLarge".into()));
                        break;
                    }
                    Err(error) => {
                        let _ = sender.send(Err(format!("IpcRead: {error}")));
                        break;
                    }
                }
            }
        });
        Ok(Self {
            child,
            input,
            events,
            acceptance_events,
            acceptance_sender,
            runtime_root: paths.runtime_root.canonicalize().map_err(|error| {
                format!("MissingRuntime: unable to resolve worker runtime: {error}")
            })?,
            identity: None,
            expected_profile: profile.to_owned(),
            request_id: 0,
            spawned_at: spawn_started,
            spawn_elapsed,
            request_started_at: None,
        })
    }

    pub fn poll(&mut self) -> Result<Option<Event>, String> {
        match self.acceptance_events.try_recv() {
            Ok(event) => return Ok(Some(event?)),
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => {}
        }
        match self.events.try_recv() {
            Ok(event) => {
                let event = event?;
                match &event {
                    Event::Ready {
                        identity, backend, ..
                    } => {
                        crate::worker_ipc::record_r7_timing(
                            "worker_start_to_ready",
                            self.spawned_at.elapsed(),
                            serde_json::json!({
                                "spawn_elapsed_ms": self.spawn_elapsed.as_secs_f64() * 1000.0,
                                "backend": backend,
                                "worker_sha256": identity.build_sha256,
                            }),
                        );
                    }
                    Event::Result { request_id, .. } => {
                        if let Some(started) = self.request_started_at.take() {
                            crate::worker_ipc::record_r7_timing(
                                "worker_request_to_result",
                                started.elapsed(),
                                serde_json::json!({ "request_id": request_id }),
                            );
                        }
                    }
                    _ => {}
                }
                Ok(Some(event))
            }
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => match self.child.try_wait() {
                Ok(Some(status)) => Err(format!("WorkerExited: {status}")),
                Ok(None) => Ok(None),
                Err(error) => Err(format!("WorkerWait: {error}")),
            },
        }
    }

    pub fn transcribe(
        &mut self,
        job_id: String,
        generation: u64,
        attempt: u64,
        source_path: &Path,
        source_sha256: String,
    ) -> Result<(), String> {
        self.transcribe_with_gpu(
            job_id,
            generation,
            attempt,
            source_path,
            source_sha256,
            None,
        )
    }

    pub fn transcribe_with_gpu(
        &mut self,
        job_id: String,
        generation: u64,
        attempt: u64,
        source_path: &Path,
        source_sha256: String,
        expected_gpu: Option<whisper_core::parakeet_ipc::ExpectedGpu>,
    ) -> Result<(), String> {
        self.transcribe_with_gpu_correlated(
            job_id,
            generation,
            attempt,
            source_path,
            source_sha256,
            expected_gpu,
        )
        .map(|_| ())
    }

    /// Sends one window and returns the exact wire request ID used for its reply.
    pub(crate) fn transcribe_with_gpu_correlated(
        &mut self,
        job_id: String,
        generation: u64,
        attempt: u64,
        source_path: &Path,
        source_sha256: String,
        expected_gpu: Option<whisper_core::parakeet_ipc::ExpectedGpu>,
    ) -> Result<u64, String> {
        let identity = self
            .identity
            .clone()
            .ok_or_else(|| "WorkerNotReady".to_owned())?;
        self.request_id = self
            .request_id
            .checked_add(1)
            .ok_or_else(|| "RequestIdentityExhausted".to_owned())?;
        let started = Instant::now();
        let request_id = self.request_id;
        self.write(&Envelope::new(WorkerCommand::Transcribe {
            request_id,
            job_id,
            generation,
            attempt,
            identity,
            source_path: source_path.to_string_lossy().into_owned(),
            source_sha256,
            expected_gpu,
        }))?;
        self.request_started_at = Some(started);
        Ok(request_id)
    }

    pub fn accept_ready(&mut self, identity: WorkerIdentity) -> Result<(), String> {
        let started = Instant::now();
        if identity.profile != self.expected_profile {
            return Err("ProfileMismatch".into());
        }
        crate::worker_ipc::record_r7_timing(
            "worker_ready_identity_validation",
            started.elapsed(),
            serde_json::json!({ "profile": identity.profile, "worker_sha256": identity.build_sha256 }),
        );
        self.identity = Some(identity);
        Ok(())
    }

    pub fn identity(&self) -> Option<&WorkerIdentity> {
        self.identity.as_ref()
    }

    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    pub(crate) fn inject_incomplete_result_for_acceptance(&self) -> Result<(), String> {
        if option_env!("PARAKEET_L2_ACCEPTANCE_TEST_BUILD").is_none() {
            return Err("InvalidInput: acceptance transport seam is disabled".into());
        }
        let runtime = std::env::var_os("PARAKEET_L2_RUNTIME_ROOT")
            .ok_or_else(|| "MissingRuntime: acceptance runtime root is required".to_owned())?;
        let runtime = PathBuf::from(runtime)
            .canonicalize()
            .map_err(|error| format!("MissingRuntime: {error}"))?;
        if runtime != self.runtime_root {
            return Err("PathOutsideRuntime: acceptance runtime differs from worker root".into());
        }
        let malformed = br#"{"version":1,"message":{"type":"result"}}"#;
        let error = decode_worker_frame(malformed)
            .expect_err("acceptance fixture must be an incomplete Result frame");
        if !error.starts_with("IpcDecode:") {
            return Err(format!("ProtocolMismatch: {error}"));
        }
        self.acceptance_sender
            .try_send(Err(error))
            .map_err(|error| format!("IpcUnavailable: {error}"))
    }

    pub fn shutdown(&mut self) -> Result<(), String> {
        self.write(&Envelope::new(WorkerCommand::Shutdown))
    }

    pub fn terminate(&mut self) -> Result<std::process::ExitStatus, String> {
        if self
            .child
            .try_wait()
            .map_err(|error| error.to_string())?
            .is_none()
        {
            self.child
                .kill()
                .map_err(|error| format!("WorkerKill: {error}"))?;
        }
        self.child
            .wait()
            .map_err(|error| format!("WorkerWait: {error}"))
    }

    pub fn try_wait(&mut self) -> Result<Option<std::process::ExitStatus>, String> {
        self.child
            .try_wait()
            .map_err(|error| format!("WorkerWait: {error}"))
    }

    fn write<T: serde::Serialize>(&mut self, value: &T) -> Result<(), String> {
        let mut frame = serde_json::to_vec(value).map_err(|error| error.to_string())?;
        if frame.len() + 1 > PARAKEET_MAX_FRAME_BYTES {
            return Err("IpcFrameTooLarge".into());
        }
        frame.push(b'\n');
        self.input
            .write_all(&frame)
            .and_then(|()| self.input.flush())
            .map_err(|error| format!("IpcWrite: {error}"))
    }
}

fn worker_stderr_log(
    paths: &ParakeetWorkerPaths,
    is_l3b_r6_cpu_worker: bool,
    session_log_root: Option<&Path>,
) -> Result<(Stdio, Option<PathBuf>), String> {
    let is_gpu_worker = paths
        .executable
        .file_name()
        .is_some_and(|name| name == "whisper-worker-parakeet-gpu.exe");
    if !is_gpu_worker && !is_l3b_r6_cpu_worker {
        return Ok((Stdio::null(), None));
    }
    let runtime = paths
        .runtime_root
        .canonicalize()
        .map_err(|error| format!("WorkerLogPath: runtime root: {error}"))?;
    let configured_l3c = if session_log_root.is_none()
        && let Some(configured) = std::env::var_os("PARAKEET_L3C_EXEC_ROOT")
    {
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
    let configured_l3d = if session_log_root.is_none()
        && let Some(configured) = std::env::var_os("PARAKEET_L3D_EXEC_ROOT")
    {
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
    let log_root = if let Some(session_root) = session_log_root {
        let canonical = session_root
            .canonicalize()
            .map_err(|error| format!("WorkerLogPath: session root: {error}"))?;
        if !canonical.is_dir() {
            return Err("WorkerLogPath: session root is not a directory".into());
        }
        canonical
    } else if is_gpu_worker && configured_campaign.is_some() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .canonicalize()
            .map_err(|error| format!("WorkerLogPath: repository root: {error}"))?;
        let expected_runtime = repository
            .join("target/parakeet-tests/l3/l3b")
            .canonicalize()
            .map_err(|error| format!("WorkerLogPath: canonical L3b root: {error}"))?;
        if runtime != expected_runtime || !l3_gpu_log {
            return Err(
                "PathOutsideRuntime: GPU ASR log requires canonical L3b/L3c/L3d roots".into(),
            );
        }
        runtime.clone()
    } else {
        configured_campaign
            .or_else(|| {
                paths
                    .is_personal_runtime()
                    .then(|| runtime.join("settings"))
            })
            .unwrap_or_else(|| runtime.clone())
    };
    let log_dir = log_root.join("logs");
    match std::fs::create_dir(&log_dir) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(format!("WorkerLogPath: create runtime logs: {error}")),
    }
    let log_dir = log_dir
        .canonicalize()
        .map_err(|error| format!("WorkerLogPath: canonical runtime logs: {error}"))?;
    if !log_dir.starts_with(&log_root) {
        return Err("PathOutsideRuntime: worker stderr log is outside its execution root".into());
    }
    static LOG_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let timestamp_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let sequence = LOG_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let prefix = worker_log_prefix(
        is_gpu_worker,
        configured_l3c.is_some(),
        configured_l3d.is_some(),
    );
    let path = log_dir.join(format!(
        "{prefix}-{timestamp_ms}-{}-{sequence}.log",
        std::process::id()
    ));
    let file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| format!("WorkerLogOpen: {}: {error}", path.display()))?;
    Ok((Stdio::from(file), is_gpu_worker.then_some(path)))
}

fn worker_log_prefix(is_gpu_worker: bool, l3c_gpu_log: bool, l3d_gpu_log: bool) -> &'static str {
    if l3d_gpu_log {
        "l3d-parakeet-asr"
    } else if l3c_gpu_log {
        "l3c-parakeet-asr"
    } else if is_gpu_worker {
        "parakeet-asr"
    } else {
        "parakeet-worker-cpu"
    }
}

#[cfg(test)]
mod worker_log_mode_tests {
    #[test]
    fn gpu_log_prefix_preserves_l3b_and_scopes_l3c_names() {
        assert_eq!(super::worker_log_prefix(true, false, false), "parakeet-asr");
        assert_eq!(
            super::worker_log_prefix(true, true, false),
            "l3c-parakeet-asr"
        );
        assert_eq!(
            super::worker_log_prefix(true, false, true),
            "l3d-parakeet-asr"
        );
        assert_eq!(
            super::worker_log_prefix(false, false, false),
            "parakeet-worker-cpu"
        );
    }
}

/// The immutable R6 CPU worker authorizes only the canonical L2b runtime. During L3b,
/// keep adapter-owned writes under L3b while giving that exact worker its read-only L2b root.
/// The environment inputs are accepted only for the exact frozen worker and canonical roots.
fn l3b_r6_cpu_read_runtime_root(paths: &ParakeetWorkerPaths) -> Result<Option<PathBuf>, String> {
    const R6_CPU_WORKER_SHA256: &str =
        "1d8f57b9e92d69629618be422077a0508303b9ae10d622b5f5aaad8867e103c0";

    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .map_err(|error| format!("RepositoryRoot: {error}"))?;
    let l3b_root = repository
        .join("target/parakeet-tests/l3/l3b")
        .canonicalize()
        .map_err(|error| format!("RuntimeRootInvalid: L3b: {error}"))?;
    let active_root = paths
        .runtime_root
        .canonicalize()
        .map_err(|error| format!("RuntimeRootInvalid: active: {error}"))?;
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

    let configured_worker = std::env::var_os("PARAKEET_L2_CPU_WORKER")
        .map(PathBuf::from)
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

    let configured_root = std::env::var_os("PARAKEET_L2_CPU_RUNTIME_ROOT")
        .map(PathBuf::from)
        .ok_or_else(|| {
            "RuntimeRootMissing: PARAKEET_L2_CPU_RUNTIME_ROOT is required for the frozen R6 worker"
                .to_owned()
        })?
        .canonicalize()
        .map_err(|error| format!("RuntimeRootInvalid: PARAKEET_L2_CPU_RUNTIME_ROOT: {error}"))?;
    if configured_root != l2b_root {
        return Err(
            "RuntimeRootMismatch: frozen R6 CPU worker requires canonical L2b read root".into(),
        );
    }
    Ok(Some(l2b_root))
}

pub fn expected_gpu_for_candidate(
    candidate: &whisper_core::gpu_inventory::GpuRuntimeIdentity,
) -> Result<whisper_core::parakeet_ipc::ExpectedGpu, String> {
    use whisper_core::model_registry::{
        PARAKEET_TEST_GPU_PCI_BUS_ID, PARAKEET_TEST_GPU_PNP_INSTANCE_ID, PARAKEET_TEST_GPU_UUID,
    };

    let candidate_pci = normalize_pci_bus_id(&candidate.pci_id)
        .ok_or_else(|| "GpuInventory: selected GPU PCI identity is malformed".to_owned())?;
    let qualified_pci = normalize_pci_bus_id(PARAKEET_TEST_GPU_PCI_BUS_ID)
        .expect("qualified PCI constant is valid");
    if candidate_pci != qualified_pci
        || !candidate
            .pnp_instance_id
            .eq_ignore_ascii_case(PARAKEET_TEST_GPU_PNP_INSTANCE_ID)
    {
        return Err(format!(
            "GpuIdentityMismatch: candidate is not the qualified physical GPU (pci={}, pnp={})",
            candidate.pci_id, candidate.pnp_instance_id
        ));
    }

    let output = Command::new("nvidia-smi")
        .args([
            "--query-gpu=count,pci.bus_id,uuid",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .map_err(|error| format!("GpuInventory: unable to start nvidia-smi: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "GpuInventory: nvidia-smi exited with {}",
            output.status
        ));
    }
    let text = String::from_utf8(output.stdout)
        .map_err(|error| format!("GpuInventory: output is not UTF-8: {error}"))?;
    let rows = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>();
    if rows.len() != 1 {
        return Err(format!(
            "GpuInventory: expected one NVIDIA device, found {}",
            rows.len()
        ));
    }
    let fields = rows[0].split(',').map(str::trim).collect::<Vec<_>>();
    if fields.len() != 3 || fields[0] != "1" {
        return Err("GpuInventory: malformed single-device nvidia-smi response".into());
    }
    let observed_pci = normalize_pci_bus_id(fields[1])
        .ok_or_else(|| "GpuInventory: observed GPU PCI identity is malformed".to_owned())?;
    if candidate_pci != observed_pci
        || observed_pci != qualified_pci
        || !fields[2].eq_ignore_ascii_case(PARAKEET_TEST_GPU_UUID)
    {
        return Err(format!(
            "GpuIdentityMismatch: selected/runtime GPU {} / {} does not match qualified PCI/UUID {} / {}",
            candidate.pci_id, fields[1], PARAKEET_TEST_GPU_PCI_BUS_ID, fields[2]
        ));
    }
    Ok(whisper_core::parakeet_ipc::ExpectedGpu {
        pci_bus_id: fields[1].to_owned(),
        uuid: fields[2].to_owned(),
    })
}

fn normalize_pci_bus_id(value: &str) -> Option<String> {
    let (domain, rest) = value.split_once(':')?;
    let (bus, rest) = rest.split_once(':')?;
    let (device, function) = rest.split_once('.')?;
    Some(format!(
        "{:08x}:{:02x}:{:02x}.{}",
        u32::from_str_radix(domain, 16).ok()?,
        u8::from_str_radix(bus, 16).ok()?,
        u8::from_str_radix(device, 16).ok()?,
        function
    ))
}

fn decode_worker_frame(frame: &[u8]) -> Result<Event, String> {
    serde_json::from_slice::<Envelope<Event>>(frame)
        .map_err(|error| format!("IpcDecode: {error}"))
        .and_then(|envelope| {
            (envelope.version == PARAKEET_IPC_VERSION)
                .then_some(envelope.message)
                .ok_or_else(|| "ProtocolVersionMismatch".to_owned())
        })
}

impl Drop for ParakeetProcess {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn not_run_without_campaign_variables(variables: &[&str]) -> bool {
        let missing = variables
            .iter()
            .filter(|name| std::env::var_os(name).is_none())
            .copied()
            .collect::<Vec<_>>();
        if missing.is_empty() {
            return false;
        }
        eprintln!(
            "NOT RUN: missing campaign variable(s): {}",
            missing.join(", ")
        );
        true
    }

    #[test]
    fn paths_reject_a_worker_outside_the_runtime_root() {
        if not_run_without_campaign_variables(&["PARAKEET_L2_RUNTIME_ROOT"]) {
            return;
        }
        let root = std::env::var_os("PARAKEET_L2_RUNTIME_ROOT")
            .map(PathBuf::from)
            .expect("PARAKEET_L2_RUNTIME_ROOT is required by the L2b test environment");
        let paths = ParakeetWorkerPaths {
            executable: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
            native_dir: root.join("native"),
            model: root.join("runtime/model/parakeet-tdt-0.6b-v3.q8_0.gguf"),
            runtime_root: root,
        };
        assert!(
            paths
                .validate()
                .unwrap_err()
                .starts_with("PathOutsideRuntime")
        );
    }

    #[test]
    fn personal_runtime_root_is_published_or_direct_staging_child_only() {
        let personal = Path::new(r"C:\Users\tester\AppData\Local\Whisper");
        assert!(personal_runtime_path_is_allowed(personal, personal));
        assert!(personal_runtime_path_is_allowed(
            Path::new(r"C:\Users\tester\AppData\Local\Whisper\.staging\candidate"),
            personal,
        ));
        assert!(!personal_runtime_path_is_allowed(
            Path::new(r"C:\dev\whisper\target\parakeet-tests\l6-perso"),
            personal,
        ));
        assert!(!personal_runtime_path_is_allowed(
            Path::new(r"C:\Users\tester\AppData\Local\Whisper\.staging\candidate\nested"),
            personal,
        ));
    }
}
