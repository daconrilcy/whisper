use crate::domain::{Generation, JobConfig, JobId, SegmentId, SourceRange};
use crate::ports::{DecodeRequest, DecodedPcmBlock, PortError, WorkerSegment};
use serde::{Deserialize, Serialize};

pub const IPC_PROTOCOL_VERSION: u16 = 2;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct IpcEnvelope<T> {
    pub version: u16,
    pub message: T,
}

impl<T> IpcEnvelope<T> {
    pub fn new(message: T) -> Self {
        Self {
            version: IPC_PROTOCOL_VERSION,
            message,
        }
    }

    pub fn is_supported(&self) -> bool {
        self.version == IPC_PROTOCOL_VERSION
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum WorkerCommand {
    /// Inspect the linked CUDA backend without loading the Whisper model.
    ProbeCuda,
    /// Qualify a bounded set of Windows GPU identities against this worker's linked backend.
    QualifyGpus {
        generation: u64,
        targets: Vec<GpuQualificationTarget>,
    },
    /// Re-attest a selected adapter in the worker that will process its session.
    AttestGpuCandidate {
        session_id: u64,
        model_path: String,
        model_sha256: [u8; 32],
        candidate: crate::gpu_inventory::GpuRuntimeIdentity,
    },
    StartImport {
        #[serde(default)]
        retain_after_publish: bool,
        job_id: JobId,
        generation: Generation,
        instance_id: u64,
        source_path: String,
        source_sha256: [u8; 32],
        expected_source_samples: u64,
        model_path: String,
        model_sha256: [u8; 32],
        config: JobConfig,
    },
    /// Start an import on the exact selected Windows/CUDA adapter.
    StartGpuImport {
        #[serde(default)]
        retain_after_publish: bool,
        job_id: JobId,
        generation: Generation,
        instance_id: u64,
        source_path: String,
        source_sha256: [u8; 32],
        expected_source_samples: u64,
        model_path: String,
        model_sha256: [u8; 32],
        config: JobConfig,
        candidate: crate::gpu_inventory::GpuRuntimeIdentity,
    },
    /// Admit audio only after an Auto GPU worker has attested its backend.
    BeginImport {
        job_id: JobId,
        generation: Generation,
        instance_id: u64,
    },
    StartLiveWorker {
        job_id: JobId,
        generation: Generation,
        instance_id: u64,
        model_path: String,
        model_sha256: [u8; 32],
        config: JobConfig,
    },
    StartDirectLiveWorker {
        job_id: JobId,
        generation: Generation,
        instance_id: u64,
        model_path: String,
        model_sha256: [u8; 32],
        config: JobConfig,
    },
    StartGpuLiveWorker {
        job_id: JobId,
        generation: Generation,
        instance_id: u64,
        model_path: String,
        model_sha256: [u8; 32],
        config: JobConfig,
        candidate: crate::gpu_inventory::GpuRuntimeIdentity,
    },
    StartGpuDirectLiveWorker {
        job_id: JobId,
        generation: Generation,
        instance_id: u64,
        model_path: String,
        model_sha256: [u8; 32],
        config: JobConfig,
        candidate: crate::gpu_inventory::GpuRuntimeIdentity,
    },
    LiveWindow {
        job_id: JobId,
        generation: Generation,
        instance_id: u64,
        sequence: u64,
        range: SourceRange,
        samples: Vec<i16>,
    },
    /// Re-encodes already confirmed PCM during a new live attempt without emitting text.
    LiveReplayWindow {
        job_id: JobId,
        generation: Generation,
        instance_id: u64,
        sequence: u64,
        range: SourceRange,
        samples: Vec<i16>,
    },
    CreateState {
        model_path: String,
        config: JobConfig,
    },
    Transcribe {
        job_id: JobId,
        generation: Generation,
        sequence: u64,
        range: SourceRange,
    },
    DecodeBlock {
        request_id: u64,
        request: DecodeRequest,
    },
    Stop {
        job_id: JobId,
        generation: Generation,
    },
    Shutdown,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum WorkerEvent {
    CudaAvailable,
    GpusQualified {
        generation: u64,
        build_architectures: String,
        devices: Vec<GpuQualificationResult>,
    },
    GpuCandidateAttested {
        session_id: u64,
        candidate: String,
        dxgi_rank: u32,
        luid: u64,
        pci_id: String,
        cuda_ordinal: u32,
        ggml_ordinal: u32,
    },
    GpuSessionAttested {
        job_id: JobId,
        generation: Generation,
        instance_id: u64,
        candidate: String,
        dxgi_rank: u32,
        luid: u64,
        pci_id: String,
        cuda_ordinal: u32,
        ggml_ordinal: u32,
    },
    Ready {
        job_id: JobId,
        generation: Generation,
        instance_id: u64,
        backend: BackendKind,
    },
    /// Confirms that an attested Auto GPU import accepted BeginImport.
    ImportStarted {
        job_id: JobId,
        generation: Generation,
        instance_id: u64,
    },
    Segment(WorkerSegmentDto),
    WindowFinished {
        job_id: JobId,
        generation: Generation,
        instance_id: u64,
        sequence: u64,
        range: SourceRange,
        last_segment_sequence: u64,
    },
    LiveMp3Packet {
        job_id: JobId,
        generation: Generation,
        instance_id: u64,
        sequence: u64,
        bytes: Vec<u8>,
    },
    DecodedBlock {
        request_id: u64,
        block: Option<DecodedPcmBlock>,
    },
    Stopped {
        job_id: JobId,
        generation: Generation,
        instance_id: u64,
    },
    Progress {
        job_id: JobId,
        generation: Generation,
        instance_id: u64,
        completed_samples: u64,
        total_samples: u64,
    },
    End {
        job_id: JobId,
        generation: Generation,
        instance_id: u64,
        last_sequence: u64,
        last_offset: u64,
    },
    Failed {
        job_id: Option<JobId>,
        generation: Option<Generation>,
        instance_id: Option<u64>,
        code: WorkerErrorCode,
        message: String,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GpuQualificationTarget {
    pub generation: u64,
    pub identity: String,
    pub name: Option<String>,
    pub state: Option<String>,
    pub presence: String,
    pub windows_rank: Option<u32>,
    pub luid: Option<u64>,
    pub pci_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GpuQualificationResult {
    pub identity: String,
    pub windows_rank: Option<u32>,
    pub luid: Option<u64>,
    pub pci_id: Option<String>,
    pub cuda_ordinal: Option<u32>,
    pub ggml_ordinal: Option<u32>,
    pub identity_match: String,
    pub support: String,
    pub availability: String,
    pub cause: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum BackendKind {
    Cpu,
    Cuda,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum WorkerErrorCode {
    UnsupportedProtocol,
    ProtocolMismatch,
    InvalidRequest,
    ModelUnavailable,
    ModelHashMismatch,
    SourceMissing,
    SourceChanged,
    UnsupportedLanguage,
    UnsupportedFormat,
    BackendUnavailable,
    BackendMismatch,
    StorageUnavailable,
    WorkerExited,
    Saturated,
    DecodeFailed,
    StaleResponse,
    InferenceFailed,
    Cancelled,
    Internal,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkerSegmentDto {
    pub job_id: JobId,
    pub generation: Generation,
    pub instance_id: u64,
    pub segment_id: SegmentId,
    pub range: SourceRange,
    pub text: String,
}

impl From<WorkerSegment> for WorkerSegmentDto {
    fn from(value: WorkerSegment) -> Self {
        Self {
            job_id: value.job_id,
            generation: value.generation,
            instance_id: value.instance_id,
            segment_id: value.segment_id,
            range: value.range,
            text: value.text,
        }
    }
}

impl From<PortError> for WorkerErrorCode {
    fn from(value: PortError) -> Self {
        match value {
            PortError::Unavailable => Self::BackendUnavailable,
            PortError::Backpressure => Self::Internal,
            PortError::InvalidInput => Self::InvalidRequest,
            PortError::StaleResponse => Self::StaleResponse,
            PortError::Committing => Self::Internal,
            PortError::Failed(_) => Self::Internal,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_version_is_explicit_and_checked() {
        let envelope = IpcEnvelope::new(WorkerCommand::Shutdown);
        assert!(envelope.is_supported());
        assert!(
            !IpcEnvelope {
                version: 1,
                message: WorkerCommand::Shutdown
            }
            .is_supported()
        );
    }
}
