//! Isolated process client for the qualified Parakeet import worker.

mod historical_runtime;

use sha2::{Digest, Sha256};
#[cfg(feature = "recette-hooks")]
use std::sync::mpsc::SyncSender;
use std::{
    fs::OpenOptions,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver, TryRecvError},
    thread,
    time::{SystemTime, UNIX_EPOCH},
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
        let shared_model = historical_runtime::is_shared_model(&root, &model)?;
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
        historical_runtime::validate_gpu_qualification(self)?;
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
    let (_, digest, _) = crate::model_catalog::cached_artifact_sha256_with_read(path)
        .map_err(|error| format!("HashRead: {error}"))?;
    let sha = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(sha)
}

pub struct ParakeetProcess {
    child: Child,
    input: ChildStdin,
    events: Receiver<Result<Event, String>>,
    #[cfg(feature = "recette-hooks")]
    acceptance_events: Receiver<Result<Event, String>>,
    #[cfg(feature = "recette-hooks")]
    acceptance_sender: SyncSender<Result<Event, String>>,
    #[cfg(feature = "recette-hooks")]
    runtime_root: PathBuf,
    identity: Option<WorkerIdentity>,
    expected_profile: String,
    request_id: u64,
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
        historical_runtime::validate_qualification_runtime(paths)?;
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
        if paths.is_personal_runtime() {
            for name in [
                "PARAKEET_R6_TIMING_LOG",
                "PARAKEET_R6_CASE",
                "PARAKEET_R7_TIMING_ACTIVE",
                "PARAKEET_R7_EXEC_ROOT",
                "PARAKEET_R7_TIMING_LOG",
                "PARAKEET_L3C_EXEC_ROOT",
                "PARAKEET_L3D_EXEC_ROOT",
            ] {
                command.env_remove(name);
            }
        }
        let cpu_read_runtime = if paths.is_personal_runtime() {
            None
        } else {
            historical_runtime::l3b_r6_cpu_read_runtime_root(paths)?
        };
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
        let mut child = command
            .env("PARAKEET_L2_RUNTIME_ROOT", worker_runtime_root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(stderr)
            .spawn()
            .map_err(|error| format!("WorkerStart: {error}"))?;
        let input = child
            .stdin
            .take()
            .ok_or_else(|| "WorkerStart: stdin unavailable".to_owned())?;
        let output = child
            .stdout
            .take()
            .ok_or_else(|| "WorkerStart: stdout unavailable".to_owned())?;
        let (sender, events) = mpsc::sync_channel(2);
        #[cfg(feature = "recette-hooks")]
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
        // Preserve the runtime-root validation even when the test seam is absent.
        let _validated_runtime_root = paths.runtime_root.canonicalize().map_err(|error| {
            format!("MissingRuntime: unable to resolve worker runtime: {error}")
        })?;
        Ok(Self {
            child,
            input,
            events,
            #[cfg(feature = "recette-hooks")]
            acceptance_events,
            #[cfg(feature = "recette-hooks")]
            acceptance_sender,
            #[cfg(feature = "recette-hooks")]
            runtime_root: _validated_runtime_root,
            identity: None,
            expected_profile: profile.to_owned(),
            request_id: 0,
        })
    }

    pub fn poll(&mut self) -> Result<Option<Event>, String> {
        #[cfg(feature = "recette-hooks")]
        match self.acceptance_events.try_recv() {
            Ok(event) => return Ok(Some(event?)),
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => {}
        }
        match self.events.try_recv() {
            Ok(event) => {
                let event = event?;
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
        let request_id = self.request_id;
        self.write(&Envelope::new(WorkerCommand::Transcribe {
            request_id,
            job_id,
            generation,
            attempt,
            identity: Box::new(identity),
            source_path: source_path.to_string_lossy().into_owned(),
            source_sha256,
            expected_gpu,
        }))?;
        Ok(request_id)
    }

    pub fn accept_ready(&mut self, identity: WorkerIdentity) -> Result<(), String> {
        if identity.profile != self.expected_profile {
            return Err("ProfileMismatch".into());
        }
        self.identity = Some(identity);
        Ok(())
    }

    pub fn identity(&self) -> Option<&WorkerIdentity> {
        self.identity.as_ref()
    }

    pub fn pid(&self) -> u32 {
        self.child.id()
    }

    #[cfg(feature = "recette-hooks")]
    pub(crate) fn inject_incomplete_result_for_acceptance(&self) -> Result<(), String> {
        historical_runtime::validate_acceptance_runtime(&self.runtime_root)?;
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
    let (log_root, l3c_log, l3d_log) = if let Some(session_root) = session_log_root {
        let canonical = session_root
            .canonicalize()
            .map_err(|error| format!("WorkerLogPath: session root: {error}"))?;
        if !canonical.is_dir() {
            return Err("WorkerLogPath: session root is not a directory".into());
        }
        (canonical, false, false)
    } else if paths.is_personal_runtime() {
        let settings = create_runtime_directory(&runtime.join("settings"), &runtime, "settings")?;
        (settings, false, false)
    } else {
        historical_runtime::historical_log_root(&runtime, is_gpu_worker)?
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
    if !log_dir.is_dir() {
        return Err(format!(
            "WorkerLogPath: runtime logs path is not a directory: {}",
            log_dir.display()
        ));
    }
    if !log_dir.starts_with(&log_root) {
        return Err("PathOutsideRuntime: worker stderr log is outside its execution root".into());
    }
    static LOG_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let timestamp_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let sequence = LOG_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let prefix = worker_log_prefix(is_gpu_worker, l3c_log, l3d_log);
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

fn create_runtime_directory(path: &Path, runtime: &Path, label: &str) -> Result<PathBuf, String> {
    match std::fs::create_dir(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists && path.is_dir() => {}
        Err(error) => {
            return Err(format!(
                "WorkerLogPath: create runtime {label}: {}: {error}",
                path.display()
            ));
        }
    }
    let canonical = path.canonicalize().map_err(|error| {
        format!(
            "WorkerLogPath: canonical runtime {label}: {}: {error}",
            path.display()
        )
    })?;
    if !canonical.is_dir() {
        return Err(format!(
            "WorkerLogPath: runtime {label} is not a directory: {}",
            canonical.display()
        ));
    }
    if !canonical.starts_with(runtime) {
        return Err(format!(
            "PathOutsideRuntime: worker log {label} is outside its execution root"
        ));
    }
    Ok(canonical)
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

#[cfg(test)]
mod worker_log_tests;

#[cfg(test)]
#[path = "parakeet_import/legacy_runtime_tests.rs"]
mod legacy_runtime_tests;

#[cfg(test)]
mod runtime_isolation_tests;

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

    #[test]
    fn paths_reject_a_worker_outside_the_runtime_root() {
        let root = std::env::var_os("PARAKEET_L2_RUNTIME_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("whisper-parakeet-runtime-tests"));
        std::fs::create_dir_all(&root).unwrap();
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
