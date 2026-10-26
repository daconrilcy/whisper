use crate::domain::{
    ComputeChoice, Generation, JobConfig, JobId, JobState, SegmentId, SourceRange,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PortError {
    Unavailable,
    /// A bounded effect queue is temporarily full; the caller may retry the same effect.
    Backpressure,
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

/// One durable final result in the shared Parakeet transcript document.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ParakeetDocumentEntry {
    pub segment: WorkerSegment,
    pub revision: u64,
}

/// A bounded read of the session document. `next_sequence` is the continuation
/// token; callers must pass the same expected revision when reading another page.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParakeetDocumentPage {
    pub revision: u64,
    pub entries: Vec<ParakeetDocumentEntry>,
    pub next_sequence: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ParakeetWindowCoverage {
    pub sequence: u64,
    pub range: SourceRange,
    pub last_segment_sequence: u64,
}

pub trait ParakeetDocumentPort {
    fn append_final(&mut self, segment: WorkerSegment) -> Result<u64, PortError>;
    fn record_coverage(&mut self, coverage: ParakeetWindowCoverage) -> Result<(), PortError>;
    fn read_page(
        &mut self,
        job_id: JobId,
        generation: Generation,
        expected_revision: u64,
        first_sequence: u64,
        limit: usize,
    ) -> Result<ParakeetDocumentPage, PortError>;
    fn finalize(&mut self, job_id: JobId, generation: Generation, total_samples: u64) -> Result<u64, PortError>;
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ImportRequest {
    #[serde(default = "legacy_transcription_strategy")]
    pub strategy: crate::parakeet_session::TranscriptionStrategy,
    pub job_id: JobId,
    pub generation: Generation,
    pub source_path: String,
    pub source_sha256: Option<[u8; 32]>,
    pub source_samples: Option<u64>,
    pub model_path: String,
    pub model_sha256: [u8; 32],
    pub destination: String,
    pub config: JobConfig,
}

fn legacy_transcription_strategy() -> crate::parakeet_session::TranscriptionStrategy {
    crate::parakeet_session::TranscriptionStrategy::LegacyV1
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum DuplicateImportChoice {
    /// Legacy replacement request kept for historical test and archive callers.
    Replace,
    ReplaceLatest,
    ReplaceAll,
    /// A folder selected by the user. The adapter must resolve and verify it
    /// against the current destination before admitting any work.
    ReplaceSelectedPath(String),
    CreateNew,
}

/// Encoded-file identity used to group prior transcription generations.
/// The format tag keeps differently encoded source formats distinct even
/// when their decoded audio happens to match.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AudioIdentity {
    pub sha256: [u8; 32],
    pub format: AudioFormat,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum AudioFormat {
    Wav,
    Mp3,
    Other,
}

/// Identifies one stored transcription without granting authority to mutate it.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct TreatmentIdentity {
    pub destination: String,
    pub job_id: JobId,
    pub generation: Generation,
}

/// Identifies one import attempt independently from the audio it processes.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct OperationIdentity {
    pub job_id: JobId,
    pub generation: Generation,
}

/// A user-confirmed target selection; adapters resolve it to immutable
/// treatment identities before any destructive operation is attempted.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ReprocessingTarget {
    CreateNew,
    LatestSuccessful,
    AllVerifiedPresent,
    Selected(TreatmentIdentity),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ReprocessingStage {
    Snapshotting,
    Queued,
    Transcribing,
    Publishing,
    Complete,
    Cancelled,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ReprocessingProgress {
    pub operation: OperationIdentity,
    pub stage: ReprocessingStage,
    pub completed: u64,
    pub total: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PublishedImport {
    pub job_id: JobId,
    pub generation: Generation,
    pub source_sha256: String,
    pub export_basename: Option<String>,
    #[serde(default)]
    pub active: bool,
    #[serde(default)]
    pub success_ordinal: Option<u64>,
    #[serde(default)]
    pub verified_present: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DuplicateImportPrompt {
    pub request: ImportRequest,
    pub source_sha256: String,
    pub source_samples: u64,
    pub publications: Vec<PublishedImport>,
    /// Export basename currently available for the user's selected action.
    /// It must be revalidated before enqueue; a collision requires a new prompt.
    #[serde(default)]
    pub export_basename: String,
    #[serde(default)]
    pub revalidation_notice: Option<String>,
    /// Present when the prompt is reconfirming an interrupted publication.
    /// The original interrupted request owns the durable reservation; `request`
    /// is the next generation to resume after the user confirms the new name.
    #[serde(default)]
    pub resume_from: Option<ImportRequest>,
    /// Basename held by the interrupted publication before the user is asked
    /// to confirm a replacement name.
    #[serde(default)]
    pub reserved_export_basename: Option<String>,
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
    /// Optional speech-only auto-stop limit. `None` means unlimited capture.
    #[serde(default)]
    pub auto_stop_after_speech_samples: Option<u64>,
    #[serde(default)]
    pub microphone: crate::microphone::MicrophoneChoice,
    /// Gain loaded for the identity selected before the audio stream is played.
    #[serde(default)]
    pub microphone_gain_db: i8,
    pub config: JobConfig,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AutomaticFinalOutputPaths {
    pub txt_path: String,
    pub srt_path: String,
    pub companion_json_path: String,
}

/// One automatic, explicitly unverified import launched after a finalized
/// Parakeet capture WAV. Its GPU identity is the one selected for that capture.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct AutomaticFinalImport {
    pub request: ImportRequest,
    pub outputs: AutomaticFinalOutputPaths,
    #[serde(default)]
    pub gpu_candidate: Option<crate::gpu_inventory::GpuRuntimeIdentity>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AutomaticFinalTranscriptionStatus {
    Pending,
    Running,
    ProducedUnverified,
    FailedOrUnavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AutomaticFinalTranscriptionView {
    pub status: AutomaticFinalTranscriptionStatus,
    pub txt_path: String,
    pub srt_path: String,
    pub model_sha256: [u8; 32],
    pub requested_compute: ComputeChoice,
    pub effective_backend: Option<String>,
    pub diagnostic: Option<String>,
}

/// Session-scoped presentation data for a Parakeet microphone capture.
/// These rows live only in the current application process; they are not persisted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParakeetCaptureIdentity {
    pub job_id: JobId,
    pub generation: Generation,
    pub session_id: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParakeetCaptureViewState {
    Starting,
    Capturing,
    Finalizing,
    Finalized,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParakeetCaptureView {
    pub identity: ParakeetCaptureIdentity,
    pub wav_path: Option<String>,
    pub direct_txt_path: Option<String>,
    pub direct_srt_path: Option<String>,
    pub companion_json_path: Option<String>,
    pub confirmed_segments: Vec<WorkerSegment>,
    pub provisional_text: Option<String>,
    pub provisional_range: Option<SourceRange>,
    pub provisional_sequence: Option<u64>,
    pub unresolved: Vec<SourceRange>,
    pub capture_state: ParakeetCaptureViewState,
    pub capture_complete: Option<bool>,
    pub capture_diagnostic: Option<String>,
    pub requested_compute: Option<ComputeChoice>,
    /// Populated only by a runtime event that identifies the effective backend.
    pub effective_backend: Option<String>,
    pub automatic_final: Option<AutomaticFinalTranscriptionView>,
    pub automatic_final_identity: Option<(JobId, Generation)>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ImportQueueStatus {
    Queued,
    AwaitingChoice,
    Running,
    Interrupted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum DiagnosticPhase {
    Preparing,
    Capturing,
    Import,
    Inference,
    Draining,
    Finalizing,
    Cancelling,
    Quitting,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum DiagnosticNotice {
    ProgressStalled {
        phase: DiagnosticPhase,
        idle_ms: u64,
    },
    LoggingUnavailable,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ImportQueueEntry {
    pub sequence: u64,
    pub request: ImportRequest,
    pub status: ImportQueueStatus,
}

/// One durable source snapshot reservation shown by the explicit recovery flow.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SourceSnapshotRecoveryCandidate {
    pub job_id: JobId,
    pub generation: Generation,
    pub destination: String,
    pub recoverable: bool,
    pub reason: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ImportEffect {
    SetGpuCandidate {
        job_id: JobId,
        generation: Generation,
        candidate: Option<crate::gpu_inventory::GpuRuntimeIdentity>,
    },
    StartLive(LiveRequest),
    StartDirectLive(LiveRequest),
    SetMicrophoneGain {
        job_id: JobId,
        generation: Generation,
        target_identity: String,
        expected_revision: u64,
        gain_db: i8,
    },
    StartMicrophoneTest {
        job_id: JobId,
        generation: Generation,
        microphone: crate::microphone::MicrophoneChoice,
        gain_db: i8,
    },
    StopMicrophoneTest {
        job_id: JobId,
        generation: Generation,
    },
    StartAutomaticFinalTranscription(AutomaticFinalImport),
    Enqueue(ImportRequest),
    ConfirmDuplicateImport {
        prompt: DuplicateImportPrompt,
        choice: DuplicateImportChoice,
    },
    ReleaseSourceBinding {
        job_id: JobId,
        generation: Generation,
        destination: String,
    },
    InspectSourceSnapshotRecovery,
    ConfirmSourceSnapshotRecovery,
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
    ResumeReplacementCleanup {
        destination: String,
        cleanup_id: String,
    },
    ResolveImportPublication {
        destination: String,
        job_id: JobId,
        generation: Generation,
    },
    ResolveLivePublication(LiveRequest),
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
    FinishLiveOnCpu {
        job_id: JobId,
        generation: Generation,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ImportEvent {
    DirectCaptureActive {
        job_id: JobId,
        generation: Generation,
        first_sample_at_unix_ms: u64,
        first_admitted_at_unix_ms: u64,
    },
    MicrophoneGainApplied {
        job_id: JobId,
        generation: Generation,
        target_identity: String,
        gain_db: i8,
        gain_revision: u64,
        first_frame: u64,
    },
    MicrophoneGainRejected {
        job_id: JobId,
        generation: Generation,
        target_identity: String,
        message: String,
    },
    MicrophoneTestStarted {
        job_id: JobId,
        generation: Generation,
        source_identity: String,
        gain_db: i8,
    },
    MicrophoneMeter(crate::microphone_level::MicrophoneMeter),
    MicrophoneSpectrum(crate::microphone_level::MicrophoneSpectrumUpdate),
    MicrophoneTestStopped {
        job_id: JobId,
        generation: Generation,
    },
    DirectConfirmed(WorkerSegment),
    DirectPublished {
        job_id: JobId,
        generation: Generation,
        txt_path: String,
        srt_path: String,
        wav_path: String,
    },
    Diagnostic {
        job_id: JobId,
        generation: Generation,
        notice: DiagnosticNotice,
    },
    LiveProvisional {
        segment: WorkerSegment,
        captured_at_unix_ms: u64,
        available_at_unix_ms: u64,
    },
    /// Replaceable Parakeet hypothesis. This is deliberately not a `WorkerSegment`:
    /// the application must never route it to persistence or confirmed text.
    ParakeetLiveStarted {
        job_id: JobId,
        generation: Generation,
        session_id: u64,
        backend: String,
    },
    ParakeetInferenceReady {
        job_id: JobId,
        generation: Generation,
        session_id: u64,
        backend: String,
    },
    ParakeetHypothesis {
        job_id: JobId,
        generation: Generation,
        session_id: u64,
        sequence: u64,
        range: SourceRange,
        text: String,
        captured_at_unix_ms: u64,
        available_at_unix_ms: u64,
    },
    /// A native-word-backed block. Once emitted, it cannot be replaced.
    ParakeetConfirmed {
        job_id: JobId,
        generation: Generation,
        session_id: u64,
        segment: WorkerSegment,
    },
    /// Retained audio whose text has not passed the confirmation rule.
    ParakeetUnresolved {
        job_id: JobId,
        generation: Generation,
        session_id: u64,
        range: SourceRange,
    },
    ParakeetResolved {
        job_id: JobId,
        generation: Generation,
        session_id: u64,
        range: SourceRange,
    },
    ParakeetCaptureProgress {
        job_id: JobId,
        generation: Generation,
        session_id: u64,
        captured_samples: u64,
        audio_durable_samples: u64,
    },
    /// Captured audio range was retained, but could not produce a usable hypothesis.
    ParakeetAudioUnusable {
        job_id: JobId,
        generation: Generation,
        session_id: u64,
        sequence: Option<u64>,
        range: SourceRange,
        message: String,
    },
    ParakeetInferenceUnavailable {
        job_id: JobId,
        generation: Generation,
        session_id: u64,
        message: String,
    },
    ParakeetCaptureStopped {
        job_id: JobId,
        generation: Generation,
        session_id: u64,
        wav_path: String,
        txt_path: Option<String>,
        srt_path: Option<String>,
        complete: bool,
        final_segments: Vec<WorkerSegment>,
        unresolved: Vec<SourceRange>,
        diagnostic: Option<String>,
        #[serde(default)]
        automatic_final: Option<AutomaticFinalImport>,
    },
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
    WindowFinished {
        job_id: JobId,
        generation: Generation,
        window_sequence: u64,
        confirmed_samples: u64,
    },
    AwaitingCpuChoice {
        job_id: JobId,
        generation: Generation,
    },
    LiveAttemptReset {
        job_id: JobId,
        generation: Generation,
        confirmed_samples: u64,
        next_segment_sequence: u64,
    },
    LiveAttemptStarting {
        job_id: JobId,
        generation: Generation,
    },
    LiveRetryingOnCpu {
        job_id: JobId,
        generation: Generation,
        spectrum_min_epoch: u64,
        spectrum_not_before_monotonic_ms: u64,
    },
    LiveFinalizing {
        job_id: JobId,
        generation: Generation,
    },
    Queue(Vec<ImportQueueEntry>),
    QueueAdmission {
        submitted_job_id: JobId,
        submitted_generation: Generation,
        queue: Vec<ImportQueueEntry>,
    },
    SourceSnapshotRecoveryInspected(Vec<SourceSnapshotRecoveryCandidate>),
    SourceSnapshotRecoveryCompleted {
        freed: usize,
        candidates: Vec<SourceSnapshotRecoveryCandidate>,
        error: Option<String>,
    },
    DuplicateImportFound(DuplicateImportPrompt),
    ResumeCheckpoint {
        job_id: JobId,
        generation: Generation,
        confirmed_segments: u64,
        confirmed_offset: u64,
    },
    Prepared(ImportRequest),
    History(Vec<ArchiveHistoryItem>),
    ReplacementCleanups(Vec<ReplacementCleanupInfo>),
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
pub struct ReplacementCleanupInfo {
    pub cleanup_id: String,
    pub successor: TreatmentIdentity,
    pub remaining: Vec<String>,
    #[serde(default)]
    pub preserved: Vec<String>,
    pub last_error: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LiveRecoveryInfo {
    pub request: LiveRequest,
    pub durable_samples: u64,
    #[serde(default)]
    pub confirmed_samples: u64,
    #[serde(default)]
    pub publication_ready: bool,
}

pub trait ImportIoPort {
    fn submit(&mut self, effect: ImportEffect) -> Result<(), PortError>;
    fn poll(&mut self) -> Result<Option<ImportEvent>, PortError>;
}
