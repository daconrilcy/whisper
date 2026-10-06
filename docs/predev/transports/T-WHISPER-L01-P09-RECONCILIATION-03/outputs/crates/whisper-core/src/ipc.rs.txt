use crate::domain::{Generation, JobConfig, JobId, SegmentId, SourceRange};
use crate::ports::{DecodeRequest, DecodedPcmBlock, PortError, WorkerSegment};
use serde::{Deserialize, Serialize};

pub const IPC_PROTOCOL_VERSION: u16 = 1;

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
        backend: BackendKind,
    },
    Segment(WorkerSegmentDto),
    DecodedBlock {
        request_id: u64,
        block: Option<DecodedPcmBlock>,
    },
    Stopped {
        job_id: JobId,
        generation: Generation,
    },
    Failed {
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
    InvalidRequest,
    ModelUnavailable,
    BackendUnavailable,
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
    pub segment_id: SegmentId,
    pub range: SourceRange,
    pub text: String,
}

impl From<WorkerSegment> for WorkerSegmentDto {
    fn from(value: WorkerSegment) -> Self {
        Self {
            job_id: value.job_id,
            generation: value.generation,
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
                version: IPC_PROTOCOL_VERSION + 1,
                message: WorkerCommand::Shutdown
            }
            .is_supported()
        );
    }
}
