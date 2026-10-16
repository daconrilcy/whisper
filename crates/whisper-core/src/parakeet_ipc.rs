use serde::{Deserialize, Serialize};

pub const PARAKEET_IPC_VERSION: u16 = 1;
pub const PARAKEET_MAX_FRAME_BYTES: usize = 1_048_576;
pub const PARAKEET_MAX_AUDIO_SAMPLES: usize = 28_800_000;
pub const PARAKEET_PROFILE_IMPORT_SINGLE_PASS: &str = "import-single-pass";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkerIdentity {
    /// SHA-256 of the local model artifact used by this worker.
    pub model_sha256: String,
    /// SHA-256 over the native runtime version and the loaded DLL set.
    pub runtime_sha256: String,
    /// SHA-256 of the worker executable.
    pub build_sha256: String,
    /// Qualified execution profile expected by the caller.
    pub profile: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Envelope<T> {
    pub version: u16,
    pub message: T,
}

impl<T> Envelope<T> {
    pub fn new(message: T) -> Self {
        Self {
            version: PARAKEET_IPC_VERSION,
            message,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    Transcribe {
        request_id: u64,
        job_id: String,
        generation: u64,
        attempt: u64,
        identity: WorkerIdentity,
        source_path: String,
        source_sha256: String,
    },
    Shutdown,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Ready {
        identity: WorkerIdentity,
        backend: String,
        native_version: String,
    },
    Result {
        request_id: u64,
        job_id: String,
        generation: u64,
        attempt: u64,
        identity: WorkerIdentity,
        source_sha256: String,
        duration_samples: u64,
        text: String,
        words: Vec<Word>,
    },
    Failed {
        request_id: Option<u64>,
        job_id: Option<String>,
        generation: Option<u64>,
        attempt: Option<u64>,
        identity: Option<WorkerIdentity>,
        code: String,
        message: String,
    },
    Stopped,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Word {
    pub text: String,
    pub start_ms: u32,
    pub end_ms: u32,
}
