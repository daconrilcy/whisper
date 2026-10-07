use crate::domain::{Generation, JobConfig, JobId, JobState, SegmentId, SourceRange};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PortError {
    Unavailable,
    InvalidInput,
    StaleResponse,
    Committing,
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

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct WorkerSegment {
    pub job_id: JobId,
    pub generation: Generation,
    pub instance_id: u64,
    pub segment_id: SegmentId,
    pub range: SourceRange,
    pub text: String,
}

pub trait WorkerPort {
    fn submit(&mut self, request: WorkerRequest) -> Result<(), PortError>;
    fn poll(&mut self) -> Result<Option<WorkerSegment>, PortError>;
    fn request_stop(&mut self, job_id: JobId, generation: Generation) -> Result<(), PortError>;
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ImportRequest {
    pub job_id: JobId,
    pub generation: Generation,
    pub source_path: String,
    pub source_sha256: Option<[u8; 32]>,
    pub source_samples: Option<u64>,
    pub model_path: String,
    pub model_sha256: [u8; 32],
    pub destination: String,
    pub group_offset_samples: u64,
    pub config: JobConfig,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LiveRequest {
    pub job_id: JobId,
    pub generation: Generation,
    pub model_path: String,
    pub model_sha256: [u8; 32],
    pub destination: String,
    /// Absolute start sample of this passage on the transcription group's clock.
    pub group_offset_samples: u64,
    pub config: JobConfig,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ImportQueueStatus {
    Queued,
    AwaitingChoice,
    Running,
    Interrupted,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ImportQueueEntry {
    pub sequence: u64,
    pub request: ImportRequest,
    pub status: ImportQueueStatus,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ImportEffect {
    StartLive(LiveRequest),
    Enqueue(ImportRequest),
    RemoveQueued {
        job_id: JobId,
        generation: Generation,
        destination: String,
    },
    ProcessQueued(ImportRequest),
    ResumeInterrupted {
        previous: Box<ImportRequest>,
        request: ImportRequest,
    },
    ScanQueue {
        destination: String,
    },
    Prepare(ImportRequest),
    StartWorker(ImportRequest),
    ScanHistory {
        destination: String,
    },
    PersistSegment(WorkerSegment),
    Publish {
        job_id: JobId,
        generation: Generation,
        last_sequence: u64,
    },
    Stop {
        job_id: JobId,
        generation: Generation,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ImportEvent {
    LiveCounters {
        job_id: JobId,
        generation: Generation,
        captured_samples: u64,
        admitted_samples: u64,
        audio_durable_samples: u64,
        confirmed_fragment_samples: u64,
        speech_samples: u64,
        diagnostic: Option<String>,
    },
    Queue(Vec<ImportQueueEntry>),
    ResumeCheckpoint {
        job_id: JobId,
        generation: Generation,
        confirmed_segments: u64,
        confirmed_offset: u64,
    },
    Prepared(ImportRequest),
    History(Vec<ArchiveHistoryItem>),
    Ready {
        job_id: JobId,
        generation: Generation,
        backend: String,
    },
    Progress {
        job_id: JobId,
        generation: Generation,
        completed_samples: u64,
        total_samples: u64,
    },
    Segment(WorkerSegment),
    Persisted {
        job_id: JobId,
        generation: Generation,
        sequence: u64,
    },
    End {
        job_id: JobId,
        generation: Generation,
        last_sequence: u64,
        last_offset: u64,
    },
    Published {
        job_id: JobId,
        generation: Generation,
    },
    Failed {
        job_id: JobId,
        generation: Generation,
        message: String,
    },
    Stopped {
        job_id: JobId,
        generation: Generation,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ArchiveHistoryItem {
    pub job_id: JobId,
    pub generation: Generation,
    pub complete: bool,
    pub source_sha256: String,
    pub diagnostic: Option<String>,
    pub live_recovery: Option<LiveRecoveryInfo>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LiveRecoveryInfo {
    pub request: LiveRequest,
    pub durable_samples: u64,
}

pub trait ImportIoPort {
    fn submit(&mut self, effect: ImportEffect) -> Result<(), PortError>;
    fn poll(&mut self) -> Result<Option<ImportEvent>, PortError>;
}
