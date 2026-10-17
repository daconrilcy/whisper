//! Isolated process client for the qualified Parakeet import worker.

use sha2::{Digest, Sha256};
use std::{
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver, SyncSender, TryRecvError},
    thread,
    time::{Duration, Instant},
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
        for (label, path) in [
            ("worker", self.executable.as_path()),
            ("native directory", self.native_dir.as_path()),
        ] {
            let canonical = path
                .canonicalize()
                .map_err(|error| format!("{label} unavailable: {error}"))?;
            if !canonical.starts_with(&root) {
                return Err(format!(
                    "PathOutsideRuntime: {label} is outside the active L2 runtime"
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
        let shared_l2b_model = repository
            .join("models/parakeet-tdt-0.6b-v3.q8_0.gguf")
            .canonicalize()
            .ok()
            .is_some_and(|expected| model == expected)
            && l2b_root.canonicalize().ok().as_ref() == Some(&root);
        if !model.starts_with(&root) && !shared_l2b_model {
            return Err("PathOutsideRuntime: model must be under runtime or the read-only L2b models directory".into());
        }
        Ok(())
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
    Ok(format!("{:x}", hasher.finalize()))
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let started = Instant::now();
    let mut file = std::fs::File::open(path).map_err(|error| format!("HashOpen: {error}"))?;
    let mut hasher = Sha256::new();
    let bytes =
        std::io::copy(&mut file, &mut hasher).map_err(|error| format!("HashRead: {error}"))?;
    let sha = format!("{:x}", hasher.finalize());
    crate::worker_ipc::record_r7_timing(
        "sha256",
        started.elapsed(),
        serde_json::json!({
            "file": path.file_name().and_then(|name| name.to_str()).unwrap_or("unknown"),
            "role": if path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("dll")) { "runtime_dll" } else if path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("gguf")) { "model" } else if path.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("wav")) { "audio" } else if path.file_name().is_some_and(|name| name.to_string_lossy().contains("worker")) { "worker" } else { "other" },
            "bytes": bytes,
            "sha256": sha,
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
        let spawn_started = Instant::now();
        let mut child = command
            .env("PARAKEET_L2_RUNTIME_ROOT", &paths.runtime_root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
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
        let identity = self
            .identity
            .clone()
            .ok_or_else(|| "WorkerNotReady".to_owned())?;
        self.request_id = self
            .request_id
            .checked_add(1)
            .ok_or_else(|| "RequestIdentityExhausted".to_owned())?;
        let started = Instant::now();
        self.write(&Envelope::new(WorkerCommand::Transcribe {
            request_id: self.request_id,
            job_id,
            generation,
            attempt,
            identity,
            source_path: source_path.to_string_lossy().into_owned(),
            source_sha256,
        }))?;
        self.request_started_at = Some(started);
        Ok(())
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
}
