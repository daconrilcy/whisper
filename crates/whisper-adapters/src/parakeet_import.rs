//! Isolated process client for the qualified Parakeet import worker.

use sha2::{Digest, Sha256};
use std::{
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver, SyncSender, TryRecvError},
    thread,
};

use whisper_core::parakeet_ipc::{
    Command as WorkerCommand, Envelope, Event, PARAKEET_IPC_VERSION, PARAKEET_MAX_FRAME_BYTES,
    PARAKEET_PROFILE_IMPORT_SINGLE_PASS, WorkerIdentity,
};

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
            ("model", self.model.as_path()),
        ] {
            let canonical = path
                .canonicalize()
                .map_err(|error| format!("{label} unavailable: {error}"))?;
            if !canonical.starts_with(&root) {
                return Err(format!("PathOutsideRuntime: {label} is outside L2a-2"));
            }
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
        let model_sha = sha256_file(&self.model)?;
        self.verify_worker_runtime(expected_runtime_sha256, expected_worker_sha256)?;
        if !model_sha.eq_ignore_ascii_case(expected_model_sha256) {
            return Err(format!("QualificationMismatch: model={model_sha}"));
        }
        Ok(())
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
    let mut file = std::fs::File::open(path).map_err(|error| format!("HashOpen: {error}"))?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher).map_err(|error| format!("HashRead: {error}"))?;
    Ok(format!("{:x}", hasher.finalize()))
}

pub struct ParakeetProcess {
    child: Child,
    input: ChildStdin,
    events: Receiver<Result<Event, String>>,
    acceptance_events: Receiver<Result<Event, String>>,
    acceptance_sender: SyncSender<Result<Event, String>>,
    runtime_root: PathBuf,
    identity: Option<WorkerIdentity>,
    request_id: u64,
}

impl ParakeetProcess {
    pub fn spawn(paths: &ParakeetWorkerPaths) -> Result<Self, String> {
        paths.validate()?;
        let model_sha256 = whisper_core::model_registry::model(
            whisper_core::model_catalog::ModelId::ParakeetTdt06BV3,
        )
        .artifact
        .sha256;
        let mut child = Command::new(&paths.executable)
            .arg("--model")
            .arg(&paths.model)
            .arg("--model-sha256")
            .arg(model_sha256)
            .arg("--native-dir")
            .arg(&paths.native_dir)
            .env("PARAKEET_L2_RUNTIME_ROOT", &paths.runtime_root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
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
            request_id: 0,
        })
    }

    pub fn poll(&mut self) -> Result<Option<Event>, String> {
        match self.acceptance_events.try_recv() {
            Ok(event) => return Ok(Some(event?)),
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => {}
        }
        match self.events.try_recv() {
            Ok(event) => Ok(Some(event?)),
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
        self.write(&Envelope::new(WorkerCommand::Transcribe {
            request_id: self.request_id,
            job_id,
            generation,
            attempt,
            identity,
            source_path: source_path.to_string_lossy().into_owned(),
            source_sha256,
        }))
    }

    pub fn accept_ready(&mut self, identity: WorkerIdentity) -> Result<(), String> {
        if identity.profile != PARAKEET_PROFILE_IMPORT_SINGLE_PASS {
            return Err("ProfileMismatch".into());
        }
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
            .expect("PARAKEET_L2_RUNTIME_ROOT is required by the L2a-2 test environment");
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
