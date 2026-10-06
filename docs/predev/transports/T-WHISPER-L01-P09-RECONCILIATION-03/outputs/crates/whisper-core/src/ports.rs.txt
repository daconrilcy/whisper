use crate::domain::{Generation, JobConfig, JobId, JobState, SegmentId, SourceRange};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PortError {
    Unavailable,
    InvalidInput,
    StaleResponse,
    Failed(String),
}

pub trait Clock {
    fn unix_millis(&self) -> u64;
}

pub trait JobRepository {
    fn state(&self, job_id: JobId) -> Result<Option<JobState>, PortError>;
    fn record_state(&mut self, job_id: JobId, state: JobState) -> Result<(), PortError>;
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PcmBlock {
    pub sequence: u64,
    pub range: SourceRange,
    pub channels: u16,
    pub samples: Vec<i16>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SourceIdentity {
    pub source_id: String,
    /// Expected content hash, revalidated by the worker before it opens the source.
    pub sha256: [u8; 32],
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DecodeRequest {
    pub source: SourceIdentity,
    pub job_id: JobId,
    pub generation: Generation,
    pub range: SourceRange,
    pub max_samples: usize,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DecodedPcmBlock {
    pub source: SourceIdentity,
    pub job_id: JobId,
    pub generation: Generation,
    pub block: PcmBlock,
}

pub trait DecoderPort {
    fn decode(&mut self, request: DecodeRequest) -> Result<Vec<DecodedPcmBlock>, PortError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkerRequest {
    pub job_id: JobId,
    pub generation: Generation,
    pub sequence: u64,
    pub config: JobConfig,
    pub range: SourceRange,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkerSegment {
    pub job_id: JobId,
    pub generation: Generation,
    pub segment_id: SegmentId,
    pub range: SourceRange,
    pub text: String,
}

pub trait WorkerPort {
    fn submit(&mut self, request: WorkerRequest) -> Result<(), PortError>;
    fn poll(&mut self) -> Result<Option<WorkerSegment>, PortError>;
    fn request_stop(&mut self, job_id: JobId, generation: Generation) -> Result<(), PortError>;
}
