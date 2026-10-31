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

/// Physical NVIDIA identity supplied by the external inventory used for this
/// request. The worker validates it against its own `nvidia-smi` snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExpectedGpu {
    pub pci_bus_id: String,
    pub uuid: String,
}

/// Runtime evidence emitted by a CUDA worker after checking its physical
/// device and capturing NeMo's stderr status line.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct GpuEvidence {
    pub backend: String,
    pub pci_bus_id: String,
    pub uuid: String,
    pub pid: u32,
    pub asr_log_sha256: String,
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
        identity: Box<WorkerIdentity>,
        source_path: String,
        source_sha256: String,
        #[serde(default)]
        expected_gpu: Option<ExpectedGpu>,
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
        #[serde(default)]
        gpu_evidence: Option<GpuEvidence>,
    },
    NativeCallStarted {
        request_id: u64,
        job_id: String,
        generation: u64,
        attempt: u64,
        pid: u32,
    },
    Result {
        request_id: u64,
        job_id: String,
        generation: u64,
        attempt: u64,
        identity: WorkerIdentity,
        source_sha256: String,
        duration_samples: u64,
        /// The normalized word texts joined in order with one ASCII space.
        /// Punctuation already attached to a word remains part of that word's
        /// text; this field is not independently normalized.
        text: String,
        words: Vec<Word>,
        #[serde(default)]
        native_inference_ms: u64,
        #[serde(default)]
        gpu_evidence: Option<GpuEvidence>,
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
/// A normalized lexical word in Parakeet IPC v1.
///
/// The worker emits valid UTF-8 and attaches punctuation-only tokens to the
/// preceding lexical word without changing that word's interval. It resolves
/// lexical overlaps of at most one 80 ms TDT frame before serialization; larger
/// overlaps or invalid native text fail the request. The adapter requires
/// positive, ordered, non-overlapping intervals and must not normalize them.
pub struct Word {
    pub text: String,
    pub start_ms: u32,
    pub end_ms: u32,
}
