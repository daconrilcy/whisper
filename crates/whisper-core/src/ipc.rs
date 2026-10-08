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
    StartImport {
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
    StartLiveWorker {
        job_id: JobId,
        generation: Generation,
        instance_id: u64,
        model_path: String,
        model_sha256: [u8; 32],
        config: JobConfig,
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
    Ready {
        job_id: JobId,
        generation: Generation,
        instance_id: u64,
        backend: BackendKind,
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
