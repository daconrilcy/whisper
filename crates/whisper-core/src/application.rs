use crate::domain::{Generation, JobConfig, JobId, JobState};
use crate::ports::{
    ArchiveHistoryItem, DiagnosticNotice, DuplicateImportChoice, DuplicateImportPrompt,
    ImportEffect, ImportEvent, ImportIoPort, ImportQueueEntry, ImportQueueStatus, ImportRequest,
    LiveRequest, PortError, ReplacementCleanupInfo, SourceSnapshotRecoveryCandidate,
};

const MAX_EVENTS_PER_REFRESH: usize = 4;

fn l3d_timing_line(
    phase: &str,
    job_id: Option<JobId>,
    generation: Option<Generation>,
    sequence: Option<u64>,
) {
    if std::env::var_os("PARAKEET_L3D_TIMING_ACTIVE").as_deref() != Some(std::ffi::OsStr::new("1"))
    {
        return;
    }
    static ORIGIN: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    static REFRESHES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let elapsed_us = ORIGIN
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_micros();
    let refresh = if phase == "refresh" {
        REFRESHES.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1
    } else {
        REFRESHES.load(std::sync::atomic::Ordering::Relaxed)
    };
    eprintln!(
        "L3D_APP_TRACE pid={} elapsed_us={elapsed_us} refresh={refresh} phase={phase} job_id={job_id:?} generation={generation:?} sequence={sequence:?}",
        std::process::id()
    );
}

fn l3d_event_trace(event: &ImportEvent) {
    let (kind, job_id, generation, sequence) = match event {
        ImportEvent::DirectCaptureActive {
            job_id, generation, ..
        }
        | ImportEvent::DirectPublished {
            job_id, generation, ..
        }
        | ImportEvent::MicrophoneGainApplied {
            job_id, generation, ..
        }
        | ImportEvent::MicrophoneGainRejected {
            job_id, generation, ..
        }
        | ImportEvent::MicrophoneTestStarted {
            job_id, generation, ..
        }
        | ImportEvent::MicrophoneTestStopped { job_id, generation }
        | ImportEvent::Diagnostic {
            job_id, generation, ..
        }
        | ImportEvent::ParakeetLiveStarted {
            job_id, generation, ..
        }
        | ImportEvent::ParakeetInferenceReady {
            job_id, generation, ..
        }
        | ImportEvent::ParakeetHypothesis {
            job_id, generation, ..
        }
        | ImportEvent::ParakeetConfirmed {
            job_id, generation, ..
        }
        | ImportEvent::ParakeetUnresolved {
            job_id, generation, ..
        }
        | ImportEvent::ParakeetResolved {
            job_id, generation, ..
        }
        | ImportEvent::ParakeetCaptureProgress {
            job_id, generation, ..
        }
        | ImportEvent::ParakeetAudioUnusable {
            job_id, generation, ..
        }
        | ImportEvent::ParakeetInferenceUnavailable {
            job_id, generation, ..
        }
        | ImportEvent::ParakeetCaptureStopped {
            job_id, generation, ..
        }
        | ImportEvent::LiveCounters {
            job_id, generation, ..
        }
        | ImportEvent::WindowFinished {
            job_id, generation, ..
        }
        | ImportEvent::AwaitingCpuChoice { job_id, generation }
        | ImportEvent::LiveAttemptReset {
            job_id, generation, ..
        }
        | ImportEvent::LiveAttemptStarting { job_id, generation }
        | ImportEvent::LiveRetryingOnCpu {
            job_id, generation, ..
        }
        | ImportEvent::LiveFinalizing { job_id, generation }
        | ImportEvent::MicrophoneMeter(crate::microphone_level::MicrophoneMeter {
            job_id,
            generation,
            ..
        })
        | ImportEvent::MicrophoneSpectrum(
            crate::microphone_level::MicrophoneSpectrumUpdate::Available(
                crate::microphone_level::MicrophoneSpectrum {
                    job_id, generation, ..
                },
            ),
        )
        | ImportEvent::MicrophoneSpectrum(
            crate::microphone_level::MicrophoneSpectrumUpdate::Unavailable {
                job_id,
                generation,
                ..
            },
        )
        | ImportEvent::ResumeCheckpoint {
            job_id, generation, ..
        }
        | ImportEvent::Ready {
            job_id, generation, ..
        }
        | ImportEvent::Progress {
            job_id, generation, ..
        }
        | ImportEvent::Persisted {
            job_id, generation, ..
        }
        | ImportEvent::End {
            job_id, generation, ..
        }
        | ImportEvent::Published { job_id, generation }
        | ImportEvent::UnifiedDocumentPage {
            job_id, generation, ..
        }
        | ImportEvent::UnifiedAudioPartsPage {
            job_id, generation, ..
        }
        | ImportEvent::UnifiedPublicationArtifacts {
            job_id, generation, ..
        }
        | ImportEvent::Failed {
            job_id, generation, ..
        }
        | ImportEvent::Stopped { job_id, generation } => {
            let sequence = match event {
                ImportEvent::WindowFinished {
                    window_sequence, ..
                } => Some(*window_sequence),
                ImportEvent::Persisted { sequence, .. } => Some(*sequence),
                ImportEvent::End { last_sequence, .. } => Some(*last_sequence),
                _ => None,
            };
            (
                event.kind_name(),
                Some(*job_id),
                Some(*generation),
                sequence,
            )
        }
        ImportEvent::DirectConfirmed(segment)
        | ImportEvent::LiveProvisional { segment, .. }
        | ImportEvent::Segment(segment) => (
            event.kind_name(),
            Some(segment.job_id),
            Some(segment.generation),
            Some(segment.segment_id.0),
        ),
        ImportEvent::Prepared(request) => (
            event.kind_name(),
            Some(request.job_id),
            Some(request.generation),
            None,
        ),
        ImportEvent::History(_)
        | ImportEvent::ReplacementCleanups(_)
        | ImportEvent::Queue(_)
        | ImportEvent::QueueAdmission { .. }
        | ImportEvent::DuplicateImportFound(_)
        | ImportEvent::SourceSnapshotRecoveryInspected(_)
        | ImportEvent::SourceSnapshotRecoveryCompleted { .. } => {
            (event.kind_name(), None, None, None)
        }
    };
    l3d_timing_line(kind, job_id, generation, sequence);
}

impl ImportEvent {
    fn kind_name(&self) -> &'static str {
        match self {
            Self::DirectCaptureActive { .. } => "direct_capture_active",
            Self::DirectConfirmed(_) => "direct_confirmed",
            Self::DirectPublished { .. } => "direct_published",
            Self::MicrophoneGainApplied { .. } => "microphone_gain_applied",
            Self::MicrophoneGainRejected { .. } => "microphone_gain_rejected",
            Self::MicrophoneTestStarted { .. } => "microphone_test_started",
            Self::MicrophoneMeter(_) => "microphone_meter",
            Self::MicrophoneSpectrum(_) => "microphone_spectrum",
            Self::MicrophoneTestStopped { .. } => "microphone_test_stopped",
            Self::Diagnostic { .. } => "diagnostic",
            Self::LiveProvisional { .. } => "live_provisional",
            Self::ParakeetLiveStarted { .. } => "parakeet_live_started",
            Self::ParakeetInferenceReady { .. } => "parakeet_inference_ready",
            Self::ParakeetHypothesis { .. } => "parakeet_hypothesis",
            Self::ParakeetConfirmed { .. } => "parakeet_confirmed",
            Self::ParakeetUnresolved { .. } => "parakeet_unresolved",
            Self::ParakeetResolved { .. } => "parakeet_resolved",
            Self::ParakeetCaptureProgress { .. } => "parakeet_capture_progress",
            Self::ParakeetAudioUnusable { .. } => "parakeet_audio_unusable",
            Self::ParakeetInferenceUnavailable { .. } => "parakeet_inference_unavailable",
            Self::ParakeetCaptureStopped { .. } => "parakeet_capture_stopped",
            Self::LiveCounters { .. } => "live_counters",
            Self::WindowFinished { .. } => "window_finished",
            Self::AwaitingCpuChoice { .. } => "awaiting_cpu_choice",
            Self::LiveAttemptReset { .. } => "live_attempt_reset",
            Self::LiveAttemptStarting { .. } => "live_attempt_starting",
            Self::LiveRetryingOnCpu { .. } => "live_retrying_on_cpu",
            Self::LiveFinalizing { .. } => "live_finalizing",
            Self::Queue(_) => "queue",
            Self::QueueAdmission { .. } => "queue_admission",
            Self::SourceSnapshotRecoveryInspected(_) => "source_snapshot_recovery_inspected",
            Self::SourceSnapshotRecoveryCompleted { .. } => "source_snapshot_recovery_completed",
            Self::DuplicateImportFound(_) => "duplicate_import_found",
            Self::ResumeCheckpoint { .. } => "resume_checkpoint",
            Self::Prepared(_) => "prepared",
            Self::History(_) => "history",
            Self::ReplacementCleanups(_) => "replacement_cleanups",
            Self::Ready { .. } => "ready",
            Self::Progress { .. } => "progress",
            Self::Segment(_) => "segment",
            Self::Persisted { .. } => "persisted",
            Self::End { .. } => "end",
            Self::Published { .. } => "published",
            Self::UnifiedDocumentPage { .. } => "unified_document_page",
            Self::UnifiedAudioPartsPage { .. } => "unified_audio_parts_page",
            Self::UnifiedPublicationArtifacts { .. } => "unified_publication_artifacts",
            Self::Failed { .. } => "failed",
            Self::Stopped { .. } => "stopped",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandIdentity {
    pub job_id: JobId,
    pub generation: Generation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AppCommand {
    Refresh,
    StartImport {
        request: ImportRequest,
    },
    EnqueueImport {
        request: ImportRequest,
    },
    ResolveDuplicateImport {
        prompt: DuplicateImportPrompt,
        choice: DuplicateImportChoice,
    },
    CancelDuplicateImport,
    InspectSourceSnapshotRecovery,
    ConfirmSourceSnapshotRecovery,
    CancelSourceSnapshotRecovery,
    ProcessQueued {
        request: ImportRequest,
    },
    ResumeInterrupted {
        request: ImportRequest,
    },
    RemoveQueued {
        job_id: JobId,
        generation: Generation,
    },
    StartLive {
        job_id: JobId,
        generation: Generation,
        config: JobConfig,
    },
    StartLiveConfigured {
        request: LiveRequest,
    },
    StartUnifiedLiveConfigured {
        request: LiveRequest,
    },
    ReadUnifiedDocumentPage {
        job_id: JobId,
        generation: Generation,
        expected_revision: u64,
        first_sequence: u64,
    },
    ReadUnifiedAudioPartsPage {
        job_id: JobId,
        generation: Generation,
        expected_revision: u64,
        offset: u64,
    },
    SelectUnifiedSession {
        job_id: JobId,
        generation: Generation,
    },
    RequestClose,
    StartDirectLiveConfigured {
        request: LiveRequest,
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
    SetMicrophoneGain {
        job_id: JobId,
        generation: Generation,
        target_identity: String,
        expected_revision: u64,
        gain: crate::microphone_level::GainDb,
    },
    FinishLiveOnCpu {
        job_id: JobId,
        generation: Generation,
    },
    Stop {
        job_id: JobId,
        generation: Generation,
    },
    StopUnifiedLive {
        job_id: JobId,
        generation: Generation,
    },
    Quit {
        job_id: JobId,
        generation: Generation,
    },
    ScanHistory {
        destination: String,
    },
    ResolveLivePublication {
        request: LiveRequest,
    },
    ResolveImportPublication {
        destination: String,
        job_id: JobId,
        generation: Generation,
    },
    ResumeReplacementCleanup {
        destination: String,
        cleanup_id: String,
    },
}

impl AppCommand {
    pub fn identity(&self) -> Option<CommandIdentity> {
        match self {
            Self::Refresh => None,
            Self::StartLive {
                job_id, generation, ..
            }
            | Self::Stop { job_id, generation }
            | Self::StopUnifiedLive { job_id, generation }
            | Self::Quit { job_id, generation }
            | Self::FinishLiveOnCpu { job_id, generation }
            | Self::StartMicrophoneTest {
                job_id, generation, ..
            }
            | Self::StopMicrophoneTest { job_id, generation } => Some(CommandIdentity {
                job_id: *job_id,
                generation: *generation,
            }),
            Self::SetMicrophoneGain {
                job_id, generation, ..
            } => Some(CommandIdentity {
                job_id: *job_id,
                generation: *generation,
            }),
            Self::StartImport { request } | Self::EnqueueImport { request } => {
                Some(CommandIdentity {
                    job_id: request.job_id,
                    generation: request.generation,
                })
            }
            Self::ProcessQueued { request } | Self::ResumeInterrupted { request } => {
                Some(CommandIdentity {
                    job_id: request.job_id,
                    generation: request.generation,
                })
            }
            Self::RemoveQueued { job_id, generation } => Some(CommandIdentity {
                job_id: *job_id,
                generation: *generation,
            }),
            Self::StartLiveConfigured { request }
            | Self::StartUnifiedLiveConfigured { request }
            | Self::StartDirectLiveConfigured { request }
            | Self::ResolveLivePublication { request } => Some(CommandIdentity {
                job_id: request.job_id,
                generation: request.generation,
            }),
            Self::ScanHistory { .. } => None,
            Self::RequestClose => None,
            Self::ReadUnifiedDocumentPage {
                job_id, generation, ..
            }
            | Self::ReadUnifiedAudioPartsPage {
                job_id, generation, ..
            }
            | Self::SelectUnifiedSession { job_id, generation } => Some(CommandIdentity {
                job_id: *job_id,
                generation: *generation,
            }),
            Self::ResolveDuplicateImport { prompt, .. } => Some(CommandIdentity {
                job_id: prompt.request.job_id,
                generation: prompt.request.generation,
            }),
            Self::CancelDuplicateImport
            | Self::InspectSourceSnapshotRecovery
            | Self::ConfirmSourceSnapshotRecovery
            | Self::CancelSourceSnapshotRecovery
            | Self::ResolveImportPublication { .. }
            | Self::ResumeReplacementCleanup { .. } => None,
        }
    }

    pub fn targets(&self, job_id: JobId, generation: Generation) -> bool {
        self.identity() == Some(CommandIdentity { job_id, generation })
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AppView {
    pub active_job: Option<(JobId, JobState)>,
    pub active_generation: Option<Generation>,
    pub queued_jobs: usize,
    pub message: Option<String>,
    pub diagnostics_degraded: bool,
    pub progress: Option<(u64, u64)>,
    pub last_result: Option<String>,
    pub history: Vec<ArchiveHistoryItem>,
    pub replacement_cleanups: Vec<ReplacementCleanupInfo>,
    pub duplicate_import_prompt: Option<DuplicateImportPrompt>,
    pub source_snapshot_recovery: Option<Vec<SourceSnapshotRecoveryCandidate>>,
    pub source_snapshot_recovery_available: bool,
    pub queue: Vec<ImportQueueEntry>,
    pub history_scan_pending: bool,
    pub queue_scan_pending: bool,
    pub captured_samples: u64,
    pub admitted_samples: u64,
    pub audio_durable_samples: u64,
    pub confirmed_fragment_samples: u64,
    /// Highest end sample among published Parakeet intervals; this may include gaps.
    pub confirmed_samples: u64,
    pub speech_samples: u64,
    pub provisional_text: Option<String>,
    pub provisional_sequence: Option<u64>,
    pub provisional_session_id: Option<u64>,
    pub provisional_range: Option<crate::domain::SourceRange>,
    pub provisional_captured_at_unix_ms: Option<u64>,
    pub provisional_available_at_unix_ms: Option<u64>,
    pub confirmed_text: Option<String>,
    pub parakeet_unresolved: Vec<crate::domain::SourceRange>,
    pub direct_first_sample_at_unix_ms: Option<u64>,
    pub direct_first_admitted_at_unix_ms: Option<u64>,
    pub direct_ready_at_unix_ms: Option<u64>,
    pub direct_capture_phase: Option<DirectCapturePhase>,
    pub direct_engine_phase: Option<DirectEnginePhase>,
    pub automatic_final_transcription: Option<crate::ports::AutomaticFinalTranscriptionView>,
    /// Parakeet captures from this process only; a new process starts with an empty list.
    pub parakeet_captures: Vec<crate::ports::ParakeetCaptureView>,
    pub unified_parakeet_sessions: Vec<crate::ports::UnifiedParakeetSessionView>,
    pub selected_unified_session: Option<crate::ports::UnifiedParakeetSelectionView>,
    pub close_requested: bool,
    pub close_ready: bool,
    pub microphone_test_active: bool,
    pub microphone_acquisition: Option<crate::microphone_level::MicrophoneAcquisitionContext>,
    pub microphone_source_identity: Option<String>,
    pub microphone_meter: Option<crate::microphone_level::MicrophoneMeter>,
    pub microphone_spectrum: Option<crate::microphone_level::MicrophoneSpectrum>,
    pub microphone_gain_db: i8,
    pub microphone_gain_revision: u64,
    pub microphone_gain_requested_db: i8,
    pub microphone_gain_requested_revision: u64,
    pub microphone_gain_applied_first_frame: Option<u64>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DirectCapturePhase {
    Initializing,
    Active,
    Stopped,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DirectEnginePhase {
    Preparing,
    Ready,
    Failed,
    Released,
}

/// Owns queue admission and import lifecycle. Adapters only execute effects.
pub struct ImportApplication<P> {
    io: P,
    view: AppView,
    active: Option<CommandIdentity>,
    request: Option<ImportRequest>,
    direct_live: bool,
    next_sequence: u64,
    pending_persistence: usize,
    pending_segments: std::collections::VecDeque<crate::ports::WorkerSegment>,
    pending_live_segments: std::collections::BTreeMap<u64, crate::ports::WorkerSegment>,
    parakeet_confirmed: Vec<crate::ports::WorkerSegment>,
    pending_enqueues: Vec<CommandIdentity>,
    auto_stop_after_speech_samples: Option<u64>,
    ended_at: Option<u64>,
    stopping: Option<CommandIdentity>,
    max_segment_offset: u64,
    final_source_samples: Option<u64>,
    model_guard: Option<crate::model_selection_app::ModelCommandGuard>,
    automatic_final_identity: Option<CommandIdentity>,
    automatic_final_view_identity: Option<CommandIdentity>,
    automatic_final_state: Option<AutomaticFinalJobState>,
    automatic_final_queue: std::collections::VecDeque<crate::ports::AutomaticFinalImport>,
    active_live_compute: Option<(CommandIdentity, crate::domain::ComputeChoice)>,
    active_live_parakeet: Option<CommandIdentity>,
    unified_captures: std::collections::HashMap<(JobId, Generation), UnifiedCaptureAdmission>,
    microphone_gain_view_owner: Option<CommandIdentity>,
    next_unified_page_request_id: u64,
    current_unified_document_request: Option<((JobId, Generation), u64)>,
    current_unified_audio_request: Option<((JobId, Generation), u64)>,
    close_queue_removals: Vec<CommandIdentity>,
    close_active_stop_submitted: bool,
}

struct UnifiedCaptureAdmission {
    identity: CommandIdentity,
    destination: String,
    stop_submitted: bool,
    publish_submitted: bool,
    microphone_gain_requested_revision: u64,
    microphone_gain_requested_target_identity: Option<String>,
    next_sequence: u64,
    pending_persistence: usize,
    pending_segments: std::collections::VecDeque<crate::ports::WorkerSegment>,
    pending_sequences: std::collections::BTreeSet<u64>,
    last_sequence: Option<u64>,
    total_samples: Option<u64>,
}

struct AutomaticFinalJobState {
    request: ImportRequest,
    next_sequence: u64,
    pending_persistence: usize,
    pending_segments: std::collections::VecDeque<crate::ports::WorkerSegment>,
    pending_sequences: std::collections::BTreeSet<u64>,
    ended_at: Option<u64>,
    max_segment_offset: u64,
    publish_submitted: bool,
}

const MAX_QUEUED_AUTOMATIC_FINAL_IMPORTS: usize = 1;
const MAX_PENDING_AUTOMATIC_FINAL_SEGMENTS: usize = 16;
const MAX_PARAKEET_CAPTURE_VIEWS: usize = 8;

impl<P: ImportIoPort> ImportApplication<P> {
    pub fn new(io: P) -> Self {
        Self {
            io,
            view: AppView::default(),
            active: None,
            request: None,
            direct_live: false,
            next_sequence: 0,
            pending_persistence: 0,
            pending_segments: std::collections::VecDeque::new(),
            pending_live_segments: std::collections::BTreeMap::new(),
            parakeet_confirmed: Vec::new(),
            pending_enqueues: Vec::new(),
            auto_stop_after_speech_samples: None,
            ended_at: None,
            stopping: None,
            max_segment_offset: 0,
            final_source_samples: None,
            model_guard: None,
            automatic_final_identity: None,
            automatic_final_view_identity: None,
            automatic_final_state: None,
            automatic_final_queue: std::collections::VecDeque::new(),
            active_live_compute: None,
            active_live_parakeet: None,
            unified_captures: std::collections::HashMap::new(),
            microphone_gain_view_owner: None,
            next_unified_page_request_id: 0,
            current_unified_document_request: None,
            current_unified_audio_request: None,
            close_queue_removals: Vec::new(),
            close_active_stop_submitted: false,
        }
    }

    /// Enables model admission for every model-bearing command before any effect is submitted.
    pub fn with_model_guard(
        mut self,
        guard: crate::model_selection_app::ModelCommandGuard,
    ) -> Self {
        self.model_guard = Some(guard);
        self
    }

    fn refuse_model_command(&mut self, reason: &str) -> AppView {
        self.view.message = Some(format!("Modèle indisponible : {reason}"));
        self.view.clone()
    }

    fn fail(&mut self, error: PortError) -> ApplicationError {
        self.view.message = Some(format!("Import indisponible : {error:?}"));
        ApplicationError::PortUnavailable
    }

    fn poll_once(&mut self) -> Result<(), ApplicationError> {
        self.retry_pending_unified_capture_segments()?;
        if !self.retry_pending_segment()? {
            self.update_close_ready();
            return Ok(());
        }
        let _ = self.retry_automatic_final_segment()?;
        for _ in 0..MAX_EVENTS_PER_REFRESH {
            let polled = match self.io.poll() {
                Ok(polled) => polled,
                Err(error) => {
                    self.view.microphone_acquisition = None;
                    self.view.microphone_spectrum = None;
                    return Err(self.fail(error));
                }
            };
            let Some(event) = polled else {
                self.update_close_ready();
                return Ok(());
            };
            let became_ready = matches!(&event, ImportEvent::Ready { .. });
            let event_identity = event.identity();
            let unified_capture_event = self.unified_captures.contains_key(&event_identity)
                || self
                    .view
                    .unified_parakeet_sessions
                    .iter()
                    .any(|session| (session.job_id, session.generation) == event_identity);
            l3d_event_trace(&event);
            self.handle_event(event)?;
            if !self.pending_segments.is_empty() && !unified_capture_event {
                self.update_close_ready();
                return Ok(());
            }
            if became_ready {
                self.update_close_ready();
                return Ok(());
            }
        }
        self.update_close_ready();
        Ok(())
    }

    fn update_close_ready(&mut self) {
        self.view.close_ready = self.view.close_requested
            && self.active.is_none()
            && self.unified_captures.is_empty()
            && !self.view.microphone_test_active
            && self.automatic_final_identity.is_none()
            && self.automatic_final_queue.is_empty()
            && self.view.queue.is_empty()
            && self.pending_enqueues.is_empty()
            && self.pending_segments.is_empty()
            && self.pending_live_segments.is_empty()
            && self.pending_persistence == 0
            && self.close_queue_removals.is_empty()
            && self.view.duplicate_import_prompt.is_none()
            && !self.view.queue_scan_pending;
    }

    fn drive_close_queue_removals(&mut self) -> Result<(), ApplicationError> {
        if !self.view.close_requested {
            return Ok(());
        }
        let removals = self
            .view
            .queue
            .iter()
            .filter(|entry| entry.status != ImportQueueStatus::Running)
            .filter_map(|entry| {
                let identity = CommandIdentity {
                    job_id: entry.request.job_id,
                    generation: entry.request.generation,
                };
                (!self.close_queue_removals.contains(&identity))
                    .then_some((identity, entry.request.destination.clone()))
            })
            .collect::<Vec<_>>();
        for (identity, destination) in removals {
            self.io
                .submit(ImportEffect::RemoveQueued {
                    job_id: identity.job_id,
                    generation: identity.generation,
                    destination,
                })
                .map_err(|error| self.fail(error))?;
            self.close_queue_removals.push(identity);
        }
        Ok(())
    }

    fn add_unified_session_card(
        &mut self,
        job_id: JobId,
        generation: Generation,
        destination: String,
        source_path: String,
        microphone_capture: bool,
    ) {
        if self
            .view
            .unified_parakeet_sessions
            .iter()
            .any(|session| (session.job_id, session.generation) == (job_id, generation))
        {
            return;
        }
        if self.view.unified_parakeet_sessions.len() >= MAX_PARAKEET_CAPTURE_VIEWS {
            let Some(index) = self
                .view
                .unified_parakeet_sessions
                .iter()
                .position(|session| {
                    matches!(
                        session.phase,
                        crate::ports::UnifiedSessionPhase::Complete
                            | crate::ports::UnifiedSessionPhase::Recoverable
                    )
                })
            else {
                return;
            };
            let removed = self.view.unified_parakeet_sessions.remove(index);
            if self
                .view
                .selected_unified_session
                .as_ref()
                .is_some_and(|selected| {
                    (selected.job_id, selected.generation) == (removed.job_id, removed.generation)
                })
            {
                self.view.selected_unified_session = None;
            }
        }
        self.view
            .unified_parakeet_sessions
            .push(crate::ports::UnifiedParakeetSessionView {
                job_id,
                generation,
                microphone_capture,
                destination,
                source_path,
                phase: crate::ports::UnifiedSessionPhase::Starting,
                captured_samples: 0,
                durable_samples: 0,
                total_samples: None,
                document_revision: 0,
                audio_parts_revision: 0,
                wav_path: None,
                txt_path: None,
                srt_path: None,
                message: None,
            });
        self.view.selected_unified_session = Some(crate::ports::UnifiedParakeetSelectionView {
            job_id,
            generation,
            document_page: None,
            preview: None,
            audio_parts_page: None,
            document_error: None,
            audio_parts_error: None,
        });
        self.current_unified_document_request = None;
        self.current_unified_audio_request = None;
    }

    fn handle_event(&mut self, event: ImportEvent) -> Result<(), ApplicationError> {
        let event_identity = event.identity();
        if matches!(
            &event,
            ImportEvent::UnifiedDocumentPage { .. }
                | ImportEvent::UnifiedAudioPartsPage { .. }
                | ImportEvent::UnifiedPublicationArtifacts { .. }
        ) {
            return self.handle_unified_capture_event(event);
        }
        if self.unified_captures.contains_key(&event_identity) {
            return self.handle_unified_capture_event(event);
        }
        self.track_parakeet_capture_event(&event);
        match &event {
            ImportEvent::History(history) => {
                self.view.history = history.clone();
                self.view.history_scan_pending = false;
                return Ok(());
            }
            ImportEvent::ReplacementCleanups(cleanups) => {
                self.view.replacement_cleanups = cleanups.clone();
                return Ok(());
            }
            ImportEvent::SourceSnapshotRecoveryInspected(candidates) => {
                self.view.source_snapshot_recovery = Some(candidates.clone());
                return Ok(());
            }
            ImportEvent::SourceSnapshotRecoveryCompleted {
                freed,
                candidates,
                error,
            } => {
                self.view.source_snapshot_recovery = Some(candidates.clone());
                self.view.source_snapshot_recovery_available = false;
                self.view.message = Some(error.clone().unwrap_or_else(|| {
                    if *freed > 0 {
                        format!("{freed} copie(s) audio retenue(s) récupérée(s). Vous pouvez relancer l’import.")
                    } else {
                        "Aucune réservation abandonnée récupérable. Les éléments protégés sont conservés.".into()
                    }
                }));
                return Ok(());
            }
            ImportEvent::Queue(queue) => {
                self.view.queue_scan_pending = false;
                let acknowledged = self
                    .pending_enqueues
                    .iter()
                    .filter(|identity| {
                        queue.iter().any(|entry| {
                            entry.request.job_id == identity.job_id
                                && entry.request.generation == identity.generation
                        })
                    })
                    .count();
                self.pending_enqueues.retain(|identity| {
                    !queue.iter().any(|entry| {
                        entry.request.job_id == identity.job_id
                            && entry.request.generation == identity.generation
                    })
                });
                if acknowledged > 0 {
                    self.view.message = Some(format!(
                        "{acknowledged} import(s) ajouté(s) à la file durable. Choisissez Traiter pour les démarrer."
                    ));
                }
                self.view.queued_jobs = queue.len();
                self.view.queue = queue.clone();
                self.close_queue_removals.retain(|identity| {
                    queue.iter().any(|entry| {
                        (entry.request.job_id, entry.request.generation)
                            == (identity.job_id, identity.generation)
                    })
                });
                self.drive_close_queue_removals()?;
                return Ok(());
            }
            ImportEvent::QueueAdmission {
                submitted_job_id,
                submitted_generation,
                queue,
            } => {
                self.view.queue_scan_pending = false;
                let submitted = CommandIdentity {
                    job_id: *submitted_job_id,
                    generation: *submitted_generation,
                };
                let acknowledged = self.pending_enqueues.contains(&submitted);
                self.pending_enqueues
                    .retain(|identity| *identity != submitted);
                if acknowledged {
                    self.view.message = Some(
                        "Import ajouté à la file durable avec un nouvel identifiant disponible. Choisissez Traiter pour le démarrer.".into(),
                    );
                }
                self.view.queued_jobs = queue.len();
                self.view.queue = queue.clone();
                self.close_queue_removals.retain(|identity| {
                    queue.iter().any(|entry| {
                        (entry.request.job_id, entry.request.generation)
                            == (identity.job_id, identity.generation)
                    })
                });
                self.drive_close_queue_removals()?;
                return Ok(());
            }
            ImportEvent::DuplicateImportFound(prompt) => {
                if let Some(identity) = self.automatic_final_identity
                    && identity.job_id == prompt.request.job_id
                    && identity.generation == prompt.request.generation
                {
                    self.mark_automatic_final_failed(
                        identity,
                        "Import final automatique déjà présent ; aucune sortie n'a été écrasée"
                            .into(),
                    );
                    return Ok(());
                }
                self.pending_enqueues.retain(|identity| {
                    *identity
                        != CommandIdentity {
                            job_id: prompt.request.job_id,
                            generation: prompt.request.generation,
                        }
                });
                if prompt.resume_from.is_some() {
                    self.active = None;
                    self.request = None;
                    self.stopping = None;
                    self.view.active_job = None;
                    self.view.active_generation = None;
                    self.view.progress = None;
                }
                self.view.duplicate_import_prompt = Some(prompt.clone());
                self.view.message = None;
                return Ok(());
            }
            ImportEvent::Failed {
                job_id, message, ..
            } if *job_id == JobId(0) && self.view.queue_scan_pending => {
                self.view.queue_scan_pending = false;
                self.view.message = Some(format!("File durable indisponible : {message}"));
                return Ok(());
            }
            ImportEvent::Failed {
                job_id,
                generation,
                message,
            } => {
                let identity = CommandIdentity {
                    job_id: *job_id,
                    generation: *generation,
                };
                if self.close_queue_removals.contains(&identity) {
                    self.close_queue_removals
                        .retain(|pending| *pending != identity);
                    self.view.message = Some(format!(
                        "Retrait de l’import {} impossible : {message}. La file est conservée.",
                        job_id.0
                    ));
                    self.update_close_ready();
                    return Ok(());
                }
            }
            _ => {}
        }
        let (job_id, generation) = event.identity();
        let identity = CommandIdentity { job_id, generation };
        if self
            .view
            .microphone_acquisition
            .as_ref()
            .is_some_and(|owner| owner.job_id == job_id && owner.generation == generation)
            && matches!(
                &event,
                ImportEvent::End { .. }
                    | ImportEvent::Failed { .. }
                    | ImportEvent::Stopped { .. }
                    | ImportEvent::Published { .. }
                    | ImportEvent::ParakeetCaptureStopped { .. }
            )
        {
            self.view.microphone_acquisition = None;
            self.view.microphone_spectrum = None;
        }
        if self.automatic_final_identity == Some(identity) && self.active != Some(identity) {
            return self.handle_automatic_final_event(event, identity);
        }
        let enqueue_failed = matches!(
            &event,
            ImportEvent::Failed { .. } if self.pending_enqueues.contains(&identity)
        );
        if self.active != Some(identity)
            && !(matches!(
                event,
                ImportEvent::Stopped { .. } | ImportEvent::Failed { .. }
            ) && self.stopping == Some(identity))
            && !enqueue_failed
        {
            return Ok(());
        }
        if let Some(session) = self
            .view
            .unified_parakeet_sessions
            .iter_mut()
            .find(|session| (session.job_id, session.generation) == (job_id, generation))
        {
            match &event {
                ImportEvent::Prepared(request) => {
                    session.total_samples = request.source_samples;
                    session.source_path = request.source_path.clone();
                    session.audio_parts_revision = 0;
                    session.wav_path = Some(
                        std::path::PathBuf::from(&request.destination)
                            .join("transcriptions")
                            .join(format!("{:032x}", request.job_id.0))
                            .join(request.generation.get().to_string())
                            .join("parakeet-unified/source.wav")
                            .to_string_lossy()
                            .into_owned(),
                    );
                    session.phase = crate::ports::UnifiedSessionPhase::Starting;
                }
                ImportEvent::Ready { .. } => {
                    session.phase = crate::ports::UnifiedSessionPhase::Capturing;
                }
                ImportEvent::Progress {
                    completed_samples,
                    total_samples,
                    ..
                } => {
                    session.durable_samples = *completed_samples;
                    session.captured_samples = *completed_samples;
                    session.total_samples = (*total_samples > 0).then_some(*total_samples);
                    session.phase = crate::ports::UnifiedSessionPhase::Capturing;
                }
                ImportEvent::Segment(segment) => {
                    session.document_revision = session
                        .document_revision
                        .max(segment.segment_id.0.saturating_sub(1));
                }
                ImportEvent::Persisted { sequence, .. } => {
                    session.document_revision = session.document_revision.max(*sequence);
                }
                ImportEvent::End { last_offset, .. } => {
                    session.total_samples = Some(*last_offset);
                    session.phase = crate::ports::UnifiedSessionPhase::Publishing;
                }
                ImportEvent::Published { .. } => {
                    session.phase = crate::ports::UnifiedSessionPhase::Complete;
                }
                ImportEvent::Failed { message, .. } => {
                    session.phase = crate::ports::UnifiedSessionPhase::Recoverable;
                    session.message = Some(message.clone());
                }
                ImportEvent::Stopped { .. } => {
                    session.phase = crate::ports::UnifiedSessionPhase::Recoverable;
                    session.message = Some("Session arrêtée avant publication".into());
                }
                _ => {}
            }
        }
        if matches!(&event, ImportEvent::Prepared(_)) {
            self.current_unified_audio_request = None;
            if let Some(selected) = self.view.selected_unified_session.as_mut()
                && (selected.job_id, selected.generation) == (job_id, generation)
            {
                selected.audio_parts_error = None;
            }
        }
        if matches!(&event, ImportEvent::Persisted { .. }) {
            self.current_unified_document_request = None;
            if let Some(selected) = self.view.selected_unified_session.as_mut()
                && (selected.job_id, selected.generation) == (job_id, generation)
            {
                selected.document_error = None;
            }
        }
        if self.stopping == Some(identity)
            && self.request.is_some()
            && matches!(event, ImportEvent::Segment(_))
        {
            return Ok(());
        }
        match event {
            ImportEvent::MicrophoneGainApplied {
                target_identity,
                gain_db,
                gain_revision,
                first_frame,
                ..
            } => {
                if self
                    .view
                    .microphone_source_identity
                    .as_deref()
                    .is_some_and(|current| current != target_identity)
                    || gain_revision < self.view.microphone_gain_revision
                {
                    return Ok(());
                }
                self.view.microphone_source_identity = Some(target_identity);
                self.view.microphone_gain_db = gain_db;
                self.view.microphone_gain_revision = gain_revision;
                self.view.microphone_gain_applied_first_frame = Some(first_frame);
                self.view.message = Some(format!(
                    "Gain microphone appliqué : {} dB à la trame {}",
                    gain_db, first_frame
                ));
            }
            ImportEvent::MicrophoneGainRejected { message, .. } => {
                self.view.message = Some(format!("Gain microphone non appliqué : {message}"));
            }
            ImportEvent::MicrophoneTestStarted {
                source_identity,
                gain_db,
                ..
            } => {
                self.view.microphone_source_identity = Some(source_identity);
                self.view.microphone_gain_db = gain_db;
                self.view.microphone_gain_revision = 0;
                self.view.microphone_gain_applied_first_frame = None;
                // A gain gesture may already have been submitted while the
                // asynchronous capture opening was still in progress. Keep
                // that requested value/revision; the adapter will apply its
                // revision after the opened stream starts acquiring blocks.
                if self.view.microphone_gain_requested_revision == 0 {
                    self.view.microphone_gain_requested_db = gain_db;
                }
                self.view.message = Some("Test microphone actif".into());
            }
            ImportEvent::MicrophoneMeter(meter) => {
                if self.stopping != Some(identity) {
                    if self
                        .view
                        .microphone_acquisition
                        .as_mut()
                        .is_some_and(|acquisition| !acquisition.bind_source(&meter.source_identity))
                    {
                        return Ok(());
                    }
                    self.view.microphone_source_identity = Some(meter.source_identity.clone());
                    self.view.microphone_gain_db = meter.gain_last_db;
                    self.view.microphone_gain_revision = meter.gain_last_revision;
                    if meter.gain_last_revision >= self.view.microphone_gain_requested_revision {
                        self.view.microphone_gain_requested_db = meter.gain_last_db;
                        self.view.microphone_gain_requested_revision = meter.gain_last_revision;
                    }
                    self.view.microphone_meter = Some(meter);
                }
            }
            ImportEvent::MicrophoneSpectrum(update) => {
                if self.stopping == Some(identity) {
                    return Ok(());
                }
                let now = crate::microphone_level::monotonic_millis();
                let Some(acquisition) = self.view.microphone_acquisition.as_mut() else {
                    return Ok(());
                };
                let Some(available) = acquisition.accept_spectrum_update(&update, now) else {
                    return Ok(());
                };
                if available {
                    let crate::microphone_level::MicrophoneSpectrumUpdate::Available(spectrum) =
                        update
                    else {
                        return Ok(());
                    };
                    self.view.microphone_source_identity = Some(spectrum.source_identity.clone());
                    self.view.microphone_spectrum = Some(spectrum.clone());
                } else {
                    self.view.microphone_spectrum = None;
                }
            }
            ImportEvent::MicrophoneTestStopped { .. } => {
                self.view.microphone_test_active = false;
                self.view.microphone_acquisition = None;
                self.view.microphone_meter = None;
                self.view.microphone_spectrum = None;
                self.view.microphone_source_identity = None;
                self.view.microphone_gain_db = 0;
                self.view.microphone_gain_revision = 0;
                self.view.microphone_gain_applied_first_frame = None;
                self.view.microphone_gain_requested_db = 0;
                self.view.microphone_gain_requested_revision = 0;
                self.view.active_generation = None;
                self.view.message = Some("Test microphone arrêté".into());
                self.active = None;
                self.request = None;
                self.stopping = None;
                self.direct_live = false;
            }
            ImportEvent::DirectCaptureActive {
                first_sample_at_unix_ms,
                first_admitted_at_unix_ms,
                ..
            } => {
                self.view.direct_first_sample_at_unix_ms = Some(first_sample_at_unix_ms);
                self.view.direct_first_admitted_at_unix_ms = Some(first_admitted_at_unix_ms);
                if self.stopping != Some(identity) {
                    self.view.direct_capture_phase = Some(DirectCapturePhase::Active);
                    self.view.active_job = Some((job_id, JobState::Capturing));
                    self.view.message = Some("Capture active — moteur en préparation".into());
                }
            }
            ImportEvent::DirectConfirmed(segment) => {
                if segment.segment_id.0 != self.next_sequence
                    || segment.range.sample_rate_hz != 16_000
                    || segment.range.start_sample >= segment.range.end_sample
                    || segment.text.trim().is_empty()
                {
                    return Err(ApplicationError::InvalidCommand);
                }
                self.next_sequence = self
                    .next_sequence
                    .checked_add(1)
                    .ok_or(ApplicationError::InvalidCommand)?;
                let text = self.view.confirmed_text.get_or_insert_with(String::new);
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(&segment.text);
            }
            ImportEvent::DirectPublished {
                txt_path,
                srt_path,
                wav_path,
                ..
            } => {
                self.view.direct_capture_phase = Some(DirectCapturePhase::Stopped);
                self.view.direct_engine_phase = Some(DirectEnginePhase::Released);
                self.view.active_job = Some((job_id, JobState::Complete));
                self.view.last_result = Some(format!(
                    "Transcription micro enregistrée : {txt_path} | {srt_path} | {wav_path}"
                ));
                self.view.message = None;
                self.active = None;
                self.stopping = None;
                self.direct_live = false;
            }
            ImportEvent::Diagnostic { notice, .. } => match notice {
                DiagnosticNotice::ProgressStalled { phase, idle_ms } => {
                    self.view.message = Some(format!(
                        "Aucun progrès observé pendant {} s ({phase:?}) ; l’opération continue.",
                        idle_ms / 1_000
                    ));
                }
                DiagnosticNotice::LoggingUnavailable => {
                    self.view.diagnostics_degraded = true;
                }
            },
            ImportEvent::LiveProvisional {
                segment,
                captured_at_unix_ms,
                available_at_unix_ms,
            } => {
                let sequence = segment.segment_id.0;
                self.view.provisional_text = Some(segment.text.clone());
                self.view.provisional_sequence = Some(sequence);
                self.view.provisional_range = Some(segment.range);
                self.view.provisional_captured_at_unix_ms = Some(captured_at_unix_ms);
                self.view.provisional_available_at_unix_ms = Some(available_at_unix_ms);
                self.pending_live_segments.insert(sequence, segment.clone());
                self.accept_segment(segment, job_id, generation)?;
            }
            ImportEvent::ParakeetLiveStarted {
                session_id,
                backend,
                ..
            } => {
                self.view.provisional_session_id = Some(session_id);
                self.view.provisional_text = None;
                self.view.provisional_sequence = None;
                self.view.provisional_range = None;
                self.view.provisional_captured_at_unix_ms = None;
                self.view.provisional_available_at_unix_ms = None;
                self.view.parakeet_unresolved.clear();
                self.parakeet_confirmed.clear();
                self.view.direct_capture_phase = Some(DirectCapturePhase::Active);
                self.view.direct_engine_phase = Some(DirectEnginePhase::Preparing);
                self.view.active_job = Some((job_id, JobState::Capturing));
                self.view.message = Some(format!(
                    "Capture micro active ; Parakeet {backend} en préparation"
                ));
            }
            ImportEvent::ParakeetInferenceReady {
                session_id,
                backend,
                ..
            } => {
                if self.view.provisional_session_id == Some(session_id) {
                    self.view.direct_engine_phase = Some(DirectEnginePhase::Ready);
                    self.view.message = Some(format!(
                        "Capture active ; hypothèses Parakeet {backend} provisoires"
                    ));
                }
            }
            ImportEvent::ParakeetHypothesis {
                session_id,
                sequence,
                range,
                text,
                captured_at_unix_ms,
                available_at_unix_ms,
                ..
            } => {
                if self.stopping == Some(identity)
                    || self.view.provisional_session_id != Some(session_id)
                    || sequence == 0
                    || self
                        .view
                        .provisional_sequence
                        .is_some_and(|previous| sequence <= previous)
                    || range.sample_rate_hz != 16_000
                    || range.start_sample >= range.end_sample
                    || text.trim().is_empty()
                {
                    return Ok(());
                }
                self.view.provisional_text = Some(text);
                self.view.provisional_sequence = Some(sequence);
                self.view.provisional_range = Some(range);
                self.view.provisional_captured_at_unix_ms = Some(captured_at_unix_ms);
                self.view.provisional_available_at_unix_ms = Some(available_at_unix_ms);
            }
            ImportEvent::ParakeetConfirmed {
                session_id,
                segment,
                ..
            } => {
                if self.view.provisional_session_id == Some(session_id) {
                    self.append_parakeet_confirmed(session_id, segment)?;
                }
            }
            ImportEvent::ParakeetUnresolved {
                session_id, range, ..
            } => {
                if self.view.provisional_session_id == Some(session_id)
                    && range.sample_rate_hz == 16_000
                    && range.start_sample < range.end_sample
                    && !self.view.parakeet_unresolved.contains(&range)
                {
                    self.view.parakeet_unresolved.push(range);
                    self.view.message =
                        Some("Audio conservé : une portion reste à confirmer".into());
                }
            }
            ImportEvent::ParakeetResolved {
                session_id, range, ..
            } => {
                if self.view.provisional_session_id == Some(session_id) {
                    self.view
                        .parakeet_unresolved
                        .retain(|unresolved| *unresolved != range);
                }
            }
            ImportEvent::ParakeetCaptureProgress {
                session_id,
                captured_samples,
                audio_durable_samples,
                ..
            } => {
                if self.view.provisional_session_id != Some(session_id)
                    || audio_durable_samples > captured_samples
                {
                    return Ok(());
                }
                self.view.captured_samples = captured_samples;
                self.view.admitted_samples = audio_durable_samples;
                self.view.audio_durable_samples = audio_durable_samples;
            }
            ImportEvent::ParakeetInferenceUnavailable {
                session_id,
                message,
                ..
            } => {
                if self.view.provisional_session_id == Some(session_id) {
                    self.view.message = Some(format!(
                        "Capture conservée ; hypothèses Parakeet indisponibles : {message}"
                    ));
                }
            }
            ImportEvent::ParakeetAudioUnusable {
                session_id,
                range,
                message,
                ..
            } => {
                if self.view.provisional_session_id == Some(session_id) {
                    self.view.message = Some(format!(
                        "Audio conservé, transcription provisoire indisponible pour {:.2}–{:.2} s : {message}",
                        range.start_sample as f64 / range.sample_rate_hz as f64,
                        range.end_sample as f64 / range.sample_rate_hz as f64,
                    ));
                }
            }
            ImportEvent::ParakeetCaptureStopped {
                job_id,
                generation,
                session_id,
                wav_path,
                txt_path,
                srt_path,
                complete,
                final_segments,
                unresolved,
                diagnostic,
                automatic_final,
                ..
            } => {
                if self.active == Some(CommandIdentity { job_id, generation }) {
                    self.view.microphone_acquisition = None;
                    self.view.microphone_spectrum = None;
                }
                if self.view.provisional_session_id != Some(session_id) {
                    return Ok(());
                }
                for segment in final_segments {
                    if let Some(prior) = self
                        .parakeet_confirmed
                        .iter()
                        .find(|prior| prior.segment_id == segment.segment_id)
                    {
                        if prior != &segment {
                            return Err(ApplicationError::InvalidCommand);
                        }
                    } else {
                        self.append_parakeet_confirmed(session_id, segment)?;
                    }
                }
                self.view.parakeet_unresolved = unresolved;
                let capture_failed = diagnostic.is_some() || !complete;
                self.view.direct_capture_phase = Some(DirectCapturePhase::Stopped);
                self.view.direct_engine_phase = Some(DirectEnginePhase::Released);
                self.view.active_job = Some((
                    job_id,
                    if capture_failed {
                        JobState::Recoverable
                    } else {
                        JobState::Complete
                    },
                ));
                self.view.last_result =
                    (!wav_path.is_empty()).then(|| match (txt_path, srt_path) {
                        (Some(txt), Some(srt)) => format!(
                            "Audio et texte confirmé enregistrés : {wav_path} | {txt} | {srt}"
                        ),
                        _ => format!("Audio microphone conservé : {wav_path}"),
                    });
                self.view.message = Some(diagnostic.map_or_else(
                    || if complete { "Transcription confirmée enregistrée.".into() } else { "Transcription incomplète ; audio conservé, texte provisoire non exporté.".into() },
                    |message| format!("Capture conservée ; session incomplète : {message}"),
                ));
                self.active = None;
                self.stopping = None;
                self.direct_live = false;
                self.request = None;
                if let Some(automatic_final) = automatic_final {
                    let identity = CommandIdentity {
                        job_id: automatic_final.request.job_id,
                        generation: automatic_final.request.generation,
                    };
                    let final_view = crate::ports::AutomaticFinalTranscriptionView {
                        status: crate::ports::AutomaticFinalTranscriptionStatus::Pending,
                        txt_path: automatic_final.outputs.txt_path.clone(),
                        srt_path: automatic_final.outputs.srt_path.clone(),
                        model_sha256: automatic_final.request.model_sha256,
                        requested_compute: automatic_final.request.config.compute,
                        effective_backend: None,
                        diagnostic: None,
                    };
                    self.automatic_final_view_identity = Some(identity);
                    if self.automatic_final_identity.is_some()
                        && self.automatic_final_queue.len() >= MAX_QUEUED_AUTOMATIC_FINAL_IMPORTS
                    {
                        self.set_capture_automatic_final_status(
                            identity,
                            crate::ports::AutomaticFinalTranscriptionStatus::FailedOrUnavailable,
                            Some("File d'import automatique saturée ; WAV et sorties directes conservés".into()),
                            None,
                        );
                        self.view.automatic_final_transcription = Some(
                            crate::ports::AutomaticFinalTranscriptionView {
                                status: crate::ports::AutomaticFinalTranscriptionStatus::FailedOrUnavailable,
                                diagnostic: Some("File d'import automatique saturée ; WAV et sorties directes conservés".into()),
                                ..final_view
                            },
                        );
                    } else {
                        self.view.automatic_final_transcription = Some(final_view);
                        self.automatic_final_queue.push_back(automatic_final);
                        self.start_next_automatic_final()?;
                    }
                } else {
                    self.view.automatic_final_transcription = Some(
                        crate::ports::AutomaticFinalTranscriptionView {
                            status:
                                crate::ports::AutomaticFinalTranscriptionStatus::FailedOrUnavailable,
                            txt_path: String::new(),
                            srt_path: String::new(),
                            model_sha256: [0; 32],
                            requested_compute: crate::domain::ComputeChoice::Auto,
                            effective_backend: None,
                            diagnostic: Some(
                                "Import automatique indisponible pour cette capture".into(),
                            ),
                        },
                    );
                }
            }
            ImportEvent::LiveCounters {
                captured_samples,
                admitted_samples,
                audio_durable_samples,
                confirmed_fragment_samples,
                speech_samples,
                diagnostic,
                ..
            } => {
                self.view.captured_samples = captured_samples;
                self.view.admitted_samples = admitted_samples;
                self.view.audio_durable_samples = audio_durable_samples;
                self.view.confirmed_fragment_samples = confirmed_fragment_samples;
                self.view.speech_samples = speech_samples;
                if let Some(message) = diagnostic {
                    self.view.message = Some(message);
                }
                if self.stopping.is_none()
                    && self
                        .auto_stop_after_speech_samples
                        .is_some_and(|limit| speech_samples >= limit)
                {
                    self.io
                        .submit(ImportEffect::Stop { job_id, generation })
                        .map_err(|e| self.fail(e))?;
                    self.stopping = Some(identity);
                    self.view.active_job = Some((
                        job_id,
                        if self.request.is_none() {
                            JobState::Draining
                        } else {
                            JobState::Cancelling
                        },
                    ));
                    self.view.message =
                        Some("Durée de parole atteinte ; finalisation du passage…".into());
                }
            }
            ImportEvent::LiveFinalizing { .. } => {
                self.view.active_job = Some((job_id, JobState::Finalizing));
            }
            ImportEvent::AwaitingCpuChoice { .. } => {
                self.view.active_job = Some((job_id, JobState::AwaitingChoice));
                self.view.message = Some(
                    "Le GPU s’est arrêté. Choisissez explicitement de terminer ce passage sur CPU."
                        .into(),
                );
            }
            ImportEvent::LiveAttemptReset {
                confirmed_samples,
                next_segment_sequence,
                ..
            } => {
                self.view.microphone_spectrum = None;
                if let Some(acquisition) = &mut self.view.microphone_acquisition {
                    acquisition.spectrum_epoch = None;
                    acquisition.spectrum_sequence = 0;
                    acquisition.spectrum_visible = false;
                }
                if confirmed_samples < self.view.confirmed_samples
                    || confirmed_samples > self.view.audio_durable_samples
                    || next_segment_sequence == 0
                {
                    return Err(ApplicationError::InvalidCommand);
                }
                self.view.confirmed_samples = confirmed_samples;
                self.next_sequence = next_segment_sequence;
                self.pending_live_segments.clear();
                self.view.provisional_text = None;
                self.view.provisional_sequence = None;
                self.view.provisional_session_id = None;
                self.view.provisional_range = None;
                self.view.provisional_captured_at_unix_ms = None;
                self.view.provisional_available_at_unix_ms = None;
            }
            ImportEvent::LiveAttemptStarting { .. } => {
                self.view.active_job = Some((job_id, JobState::Preparing));
                self.view.message = Some("Attente de l’attestation du worker CPU…".into());
            }
            ImportEvent::UnifiedDocumentPage { .. }
            | ImportEvent::UnifiedAudioPartsPage { .. }
            | ImportEvent::UnifiedPublicationArtifacts { .. } => {}
            ImportEvent::LiveRetryingOnCpu {
                spectrum_min_epoch,
                spectrum_not_before_monotonic_ms,
                ..
            } => {
                self.view.microphone_spectrum = None;
                if let Some(acquisition) = &mut self.view.microphone_acquisition {
                    acquisition.spectrum_epoch = None;
                    acquisition.spectrum_sequence = 0;
                    acquisition.spectrum_visible = false;
                    acquisition.spectrum_min_epoch = Some(spectrum_min_epoch);
                    acquisition.spectrum_not_before_monotonic_ms =
                        Some(spectrum_not_before_monotonic_ms);
                }
                self.view.active_job = Some((job_id, JobState::Capturing));
                self.view.message = Some(
                    "Le GPU a échoué ; la capture continue et le passage sera repris sur CPU."
                        .into(),
                );
            }
            ImportEvent::WindowFinished {
                confirmed_samples, ..
            } => {
                if confirmed_samples < self.view.confirmed_samples
                    || confirmed_samples > self.view.audio_durable_samples
                {
                    return Err(ApplicationError::InvalidCommand);
                }
                self.view.confirmed_samples = confirmed_samples;
            }
            ImportEvent::History(_) => {
                unreachable!("non-lifecycle events are handled before job admission")
            }
            ImportEvent::Queue(_)
            | ImportEvent::QueueAdmission { .. }
            | ImportEvent::DuplicateImportFound(_)
            | ImportEvent::SourceSnapshotRecoveryInspected(_)
            | ImportEvent::SourceSnapshotRecoveryCompleted { .. } => {
                unreachable!("queue/admission events are handled before job admission")
            }
            ImportEvent::ResumeCheckpoint {
                confirmed_segments,
                confirmed_offset,
                ..
            } => {
                self.next_sequence = confirmed_segments
                    .checked_add(1)
                    .ok_or(ApplicationError::InvalidCommand)?;
                self.max_segment_offset = confirmed_offset;
            }
            ImportEvent::Prepared(request) => {
                if request.source_sha256.is_none()
                    || request.source_samples.is_none_or(|samples| samples == 0)
                {
                    return Err(ApplicationError::InvalidCommand);
                }
                let Some(accepted) = self.request.as_ref() else {
                    return Err(ApplicationError::InvalidCommand);
                };
                let mut expected = accepted.clone();
                expected.source_sha256 = request.source_sha256;
                expected.source_samples = request.source_samples;
                if expected != request {
                    return Err(ApplicationError::InvalidCommand);
                }
                self.request = Some(request.clone());
                if self.stopping != Some(identity) {
                    self.io
                        .submit(ImportEffect::StartWorker(request))
                        .map_err(|e| self.fail(e))?;
                    self.view.active_job = Some((job_id, JobState::Preparing));
                    self.view.message = Some("Source vérifiée ; préparation du moteur…".into());
                }
            }
            ImportEvent::Ready { backend, .. } => {
                if self.automatic_final_identity == Some(identity) {
                    if let Some(final_view) = self.view.automatic_final_transcription.as_mut() {
                        final_view.status =
                            crate::ports::AutomaticFinalTranscriptionStatus::Running;
                        final_view.effective_backend = Some(backend.clone());
                    }
                    self.view.active_job = Some((job_id, JobState::Running));
                    self.view.message = Some(format!(
                        "Transcription finale automatique en cours — non vérifiée ({backend})"
                    ));
                    return Ok(());
                }
                if self.direct_live {
                    self.view.direct_engine_phase = Some(DirectEnginePhase::Ready);
                    self.view.direct_ready_at_unix_ms = Some(
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map_or(0, |elapsed| elapsed.as_millis() as u64),
                    );
                }
                if self.stopping != Some(identity) {
                    self.view.active_job = Some((
                        job_id,
                        if self.request.is_none() {
                            JobState::Capturing
                        } else {
                            JobState::Running
                        },
                    ));
                    self.view.message = Some(if self.direct_live {
                        format!("Capture et transcription opérationnelles — moteur : {backend}")
                    } else {
                        format!("Moteur actif : {backend}")
                    });
                }
            }
            ImportEvent::Progress {
                completed_samples,
                total_samples,
                ..
            } => {
                let source_samples = self
                    .request
                    .as_ref()
                    .and_then(|request| request.source_samples);
                if source_samples.is_none_or(|limit| completed_samples > limit)
                    || (total_samples > 0
                        && (completed_samples != total_samples
                            || Some(total_samples) != source_samples))
                {
                    return Err(ApplicationError::InvalidCommand);
                }
                if total_samples > 0 {
                    self.final_source_samples = Some(total_samples);
                }
                self.view.progress = Some((completed_samples, total_samples));
            }
            ImportEvent::Segment(segment) => {
                self.accept_segment(segment, job_id, generation)?;
            }
            ImportEvent::Persisted { sequence, .. } => {
                self.pending_persistence = self.pending_persistence.saturating_sub(1);
                if sequence >= self.next_sequence {
                    return Err(ApplicationError::InvalidCommand);
                }
                if self.request.is_none() {
                    self.view.confirmed_fragment_samples = self
                        .view
                        .confirmed_fragment_samples
                        .max(self.max_segment_offset);
                    if let Some(segment) = self.pending_live_segments.remove(&sequence) {
                        self.view.confirmed_text = Some(segment.text);
                        if self.view.provisional_sequence == Some(sequence) {
                            self.view.provisional_text = None;
                            self.view.provisional_sequence = None;
                            self.view.provisional_range = None;
                            self.view.provisional_captured_at_unix_ms = None;
                            self.view.provisional_available_at_unix_ms = None;
                        }
                    }
                }
                self.maybe_publish(job_id, generation)?;
            }
            ImportEvent::End {
                last_sequence,
                last_offset,
                ..
            } => {
                if last_sequence != self.next_sequence.saturating_sub(1)
                    || last_offset == 0
                    || self.final_source_samples != Some(last_offset)
                    || self
                        .request
                        .as_ref()
                        .and_then(|request| request.source_samples)
                        != Some(last_offset)
                    || self.max_segment_offset > last_offset
                {
                    return Err(ApplicationError::InvalidCommand);
                }
                self.ended_at = Some(last_sequence);
                self.maybe_publish(job_id, generation)?;
            }
            ImportEvent::Published { .. } => {
                if self.automatic_final_identity == Some(identity) {
                    if let Some(final_view) = self.view.automatic_final_transcription.as_mut() {
                        final_view.status =
                            crate::ports::AutomaticFinalTranscriptionStatus::ProducedUnverified;
                        final_view.diagnostic = None;
                    }
                    self.view.active_job = Some((job_id, JobState::Complete));
                    self.view.last_result =
                        Some("Transcription automatique finale produite — non vérifiée".into());
                    self.view.message = Some(
                        "Brouillon final automatique enregistré séparément ; texte non vérifié"
                            .into(),
                    );
                    self.active = None;
                    self.request = None;
                    self.stopping = None;
                    self.automatic_final_identity = None;
                    return Ok(());
                }
                if self.stopping == Some(identity) && self.request.is_some() {
                    self.view.active_job = Some((job_id, JobState::Cancelling));
                } else {
                    self.view.active_job = Some((job_id, JobState::Complete));
                    self.view.last_result = Some("Transcription publiée et vérifiée".into());
                    self.view.message = None;
                    self.active = None;
                    self.stopping = None;
                    self.view.provisional_text = None;
                    self.view.provisional_sequence = None;
                    self.view.provisional_range = None;
                    self.view.provisional_captured_at_unix_ms = None;
                    self.view.provisional_available_at_unix_ms = None;
                }
            }
            ImportEvent::Failed { message, .. } => {
                if self.automatic_final_identity == Some(identity) {
                    self.mark_automatic_final_failed(identity, message);
                    return Ok(());
                }
                if self.view.microphone_test_active {
                    self.view.microphone_test_active = false;
                    self.view.microphone_meter = None;
                    self.view.microphone_source_identity = None;
                    self.view.microphone_gain_db = 0;
                    self.view.microphone_gain_revision = 0;
                    self.view.active_generation = None;
                    self.view.message = Some(format!("Test microphone arrêté : {message}"));
                    self.active = None;
                    self.request = None;
                    self.stopping = None;
                    self.direct_live = false;
                    return Ok(());
                }
                if self.direct_live {
                    self.view.direct_capture_phase = Some(DirectCapturePhase::Failed);
                    self.view.direct_engine_phase = Some(DirectEnginePhase::Failed);
                }
                if self.pending_enqueues.contains(&identity) {
                    self.pending_enqueues.retain(|pending| *pending != identity);
                    if message.starts_with("SourceSnapshotCapacity:") {
                        self.view.source_snapshot_recovery_available = true;
                    }
                    self.view.message = Some(message);
                } else if self.stopping == Some(identity) && self.request.is_some() {
                    self.view.active_job = Some((job_id, JobState::Cancelled));
                    self.view.message =
                        Some(format!("Arrêt confirmé; archive conservée ({message})."));
                    self.active = None;
                    self.stopping = None;
                } else {
                    self.view.active_job = Some((job_id, JobState::Recoverable));
                    self.view.message = Some(message);
                    self.active = None;
                    self.stopping = None;
                    self.direct_live = false;
                }
            }
            ImportEvent::Stopped { .. } => {
                if self.automatic_final_identity == Some(identity) {
                    self.mark_automatic_final_failed(
                        identity,
                        "Import final automatique arrêté".into(),
                    );
                    return Ok(());
                }
                self.view.active_job = Some((job_id, JobState::Cancelled));
                self.view.message =
                    Some("Arrêt confirmé ; la source et l’archive pending sont conservées.".into());
                self.active = None;
                self.stopping = None;
                self.ended_at = None;
                self.pending_persistence = 0;
            }
            // Handled before the job-identity gate above; it is a background
            // archive-history update and does not belong to an active import.
            ImportEvent::ReplacementCleanups(_) => return Ok(()),
        }
        Ok(())
    }

    fn handle_unified_capture_event(&mut self, event: ImportEvent) -> Result<(), ApplicationError> {
        let key = event.identity();
        match event {
            ImportEvent::ParakeetLiveStarted { .. }
            | ImportEvent::ParakeetInferenceReady { .. } => {
                if let Some(session) = self
                    .view
                    .unified_parakeet_sessions
                    .iter_mut()
                    .find(|s| (s.job_id, s.generation) == key)
                {
                    session.phase = crate::ports::UnifiedSessionPhase::Capturing;
                }
            }
            ImportEvent::Ready { backend, .. } => {
                if let Some(session) = self
                    .view
                    .unified_parakeet_sessions
                    .iter_mut()
                    .find(|s| (s.job_id, s.generation) == key)
                {
                    session.phase = crate::ports::UnifiedSessionPhase::Capturing;
                    session.message = Some(format!("Moteur actif : {backend}"));
                }
            }
            ImportEvent::ParakeetCaptureProgress {
                captured_samples,
                audio_durable_samples,
                ..
            } => {
                self.current_unified_audio_request = None;
                if let Some(session) = self
                    .view
                    .unified_parakeet_sessions
                    .iter_mut()
                    .find(|s| (s.job_id, s.generation) == key)
                {
                    session.captured_samples = session.captured_samples.max(captured_samples);
                    session.durable_samples = session
                        .durable_samples
                        .max(audio_durable_samples)
                        .min(session.captured_samples);
                    session.audio_parts_revision = 0;
                    if matches!(
                        session.phase,
                        crate::ports::UnifiedSessionPhase::Starting
                            | crate::ports::UnifiedSessionPhase::Capturing
                    ) {
                        session.phase = crate::ports::UnifiedSessionPhase::Capturing;
                    }
                }
                if let Some(selected) = self.view.selected_unified_session.as_mut()
                    && (selected.job_id, selected.generation) == key
                {
                    selected.audio_parts_error = None;
                }
            }
            ImportEvent::ParakeetHypothesis {
                sequence,
                range,
                text,
                ..
            } => {
                if let Some(selected) = self.view.selected_unified_session.as_mut()
                    && (selected.job_id, selected.generation) == key
                {
                    let revision = self
                        .view
                        .unified_parakeet_sessions
                        .iter()
                        .find(|s| (s.job_id, s.generation) == key)
                        .map_or(0, |s| s.document_revision);
                    selected.preview = Some(crate::ports::ParakeetPreview {
                        segment: crate::ports::WorkerSegment {
                            job_id: key.0,
                            generation: key.1,
                            instance_id: 0,
                            segment_id: crate::domain::SegmentId(sequence),
                            range,
                            text,
                        },
                        revision,
                    });
                }
            }
            ImportEvent::ParakeetConfirmed { segment, .. } => {
                self.current_unified_document_request = None;
                if let Some(session) = self
                    .view
                    .unified_parakeet_sessions
                    .iter_mut()
                    .find(|s| (s.job_id, s.generation) == key)
                {
                    session.document_revision = session.document_revision.max(segment.segment_id.0);
                }
                if let Some(selected) = self.view.selected_unified_session.as_mut()
                    && (selected.job_id, selected.generation) == key
                {
                    selected.document_error = None;
                    if selected
                        .preview
                        .as_ref()
                        .is_some_and(|preview| preview.segment.segment_id.0 <= segment.segment_id.0)
                    {
                        selected.preview = None;
                    }
                }
            }
            ImportEvent::ParakeetCaptureStopped { wav_path, .. } => {
                if let Some(session) = self
                    .view
                    .unified_parakeet_sessions
                    .iter_mut()
                    .find(|s| (s.job_id, s.generation) == key)
                {
                    session.wav_path = Some(wav_path);
                    session.phase = crate::ports::UnifiedSessionPhase::Finalizing;
                }
            }
            ImportEvent::MicrophoneGainApplied {
                target_identity,
                gain_db,
                gain_revision,
                first_frame,
                ..
            } => {
                let Some(capture) = self.unified_captures.get_mut(&key) else {
                    return Ok(());
                };
                if gain_revision < self.view.microphone_gain_revision
                    || gain_revision > capture.microphone_gain_requested_revision
                    || self.microphone_gain_view_owner != Some(capture.identity)
                    || self
                        .view
                        .microphone_source_identity
                        .as_deref()
                        .is_some_and(|current| current != target_identity)
                {
                    return Ok(());
                }
                self.view.microphone_source_identity = Some(target_identity.clone());
                self.view.microphone_gain_db = gain_db;
                self.view.microphone_gain_revision = gain_revision;
                self.view.microphone_gain_applied_first_frame = Some(first_frame);
                if capture.microphone_gain_requested_revision == gain_revision
                    && capture.microphone_gain_requested_target_identity.as_deref()
                        == Some(target_identity.as_str())
                {
                    capture.microphone_gain_requested_target_identity = None;
                }
                self.view.message = Some(format!(
                    "Gain microphone appliqué : {} dB à la trame {}",
                    gain_db, first_frame
                ));
            }
            ImportEvent::MicrophoneGainRejected {
                target_identity,
                message,
                ..
            } => {
                let owns_view = self.unified_captures.get(&key).is_some_and(|capture| {
                    self.microphone_gain_view_owner == Some(capture.identity)
                });
                if let Some(capture) = self.unified_captures.get_mut(&key)
                    && capture.microphone_gain_requested_target_identity.as_deref()
                        == Some(target_identity.as_str())
                {
                    capture.microphone_gain_requested_target_identity = None;
                    if self.microphone_gain_view_owner == Some(capture.identity) {
                        capture.microphone_gain_requested_revision =
                            self.view.microphone_gain_revision;
                        capture.microphone_gain_requested_target_identity =
                            self.view.microphone_source_identity.clone();
                        self.view.microphone_gain_requested_revision =
                            self.view.microphone_gain_revision;
                        self.view.microphone_gain_requested_db = self.view.microphone_gain_db;
                    }
                }
                if owns_view {
                    self.view.message = Some(format!("Gain microphone non appliqué : {message}"));
                }
            }
            ImportEvent::Prepared(request) => {
                let capture = self
                    .unified_captures
                    .get_mut(&key)
                    .ok_or(ApplicationError::InvalidCommand)?;
                if !capture.stop_submitted
                    || (request.job_id, request.generation)
                        != (capture.identity.job_id, capture.identity.generation)
                    || request.strategy
                        != crate::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
                    || request.source_sha256.is_none()
                    || request.source_samples.is_none_or(|samples| samples == 0)
                {
                    return Err(ApplicationError::InvalidCommand);
                }
                capture.total_samples = request.source_samples;
                if let Some(session) = self
                    .view
                    .unified_parakeet_sessions
                    .iter_mut()
                    .find(|s| (s.job_id, s.generation) == key)
                {
                    session.total_samples = request.source_samples;
                    session.wav_path = Some(request.source_path);
                    session.phase = crate::ports::UnifiedSessionPhase::Finalizing;
                    session.audio_parts_revision = 0;
                }
                self.current_unified_audio_request = None;
                if let Some(selected) = self.view.selected_unified_session.as_mut()
                    && (selected.job_id, selected.generation) == key
                {
                    selected.audio_parts_error = None;
                }
            }
            ImportEvent::Progress {
                job_id,
                generation,
                completed_samples,
                total_samples,
            } => {
                let capture = self
                    .unified_captures
                    .get(&key)
                    .ok_or(ApplicationError::InvalidCommand)?;
                if (job_id, generation) != (capture.identity.job_id, capture.identity.generation)
                    || capture.total_samples != Some(total_samples)
                    || completed_samples != total_samples
                {
                    return Err(ApplicationError::InvalidCommand);
                }
            }
            ImportEvent::Segment(segment) => {
                let capture = self
                    .unified_captures
                    .get_mut(&key)
                    .ok_or(ApplicationError::InvalidCommand)?;
                if segment.segment_id.0 != capture.next_sequence
                    || (segment.job_id, segment.generation)
                        != (capture.identity.job_id, capture.identity.generation)
                {
                    return Err(ApplicationError::InvalidCommand);
                }
                capture.next_sequence = capture
                    .next_sequence
                    .checked_add(1)
                    .ok_or(ApplicationError::InvalidCommand)?;
                capture.pending_segments.push_back(segment);
                self.retry_unified_capture_segment(key)?;
            }
            ImportEvent::Persisted { sequence, .. } => {
                let capture = self
                    .unified_captures
                    .get_mut(&key)
                    .ok_or(ApplicationError::InvalidCommand)?;
                if !capture.pending_sequences.remove(&sequence) {
                    return Err(ApplicationError::InvalidCommand);
                }
                capture.pending_persistence = capture.pending_sequences.len();
                if let Some(session) = self
                    .view
                    .unified_parakeet_sessions
                    .iter_mut()
                    .find(|s| (s.job_id, s.generation) == key)
                {
                    session.document_revision = session.document_revision.max(sequence);
                    session.phase = crate::ports::UnifiedSessionPhase::Capturing;
                }
                self.current_unified_document_request = None;
                if let Some(selected) = self.view.selected_unified_session.as_mut()
                    && (selected.job_id, selected.generation) == key
                {
                    selected.document_error = None;
                }
                self.maybe_publish_unified_capture(key)?;
            }
            ImportEvent::End {
                job_id,
                generation,
                last_sequence,
                last_offset,
            } => {
                let capture = self
                    .unified_captures
                    .get_mut(&key)
                    .ok_or(ApplicationError::InvalidCommand)?;
                if !capture.stop_submitted
                    || (capture.identity.job_id, capture.identity.generation)
                        != (job_id, generation)
                    || last_sequence != capture.next_sequence.saturating_sub(1)
                    || last_offset == 0
                    || capture.total_samples != Some(last_offset)
                {
                    return Err(ApplicationError::InvalidCommand);
                }
                capture.last_sequence = Some(last_sequence);
                capture.total_samples = Some(last_offset);
                if let Some(session) = self
                    .view
                    .unified_parakeet_sessions
                    .iter_mut()
                    .find(|s| (s.job_id, s.generation) == key)
                {
                    session.total_samples = Some(last_offset);
                    session.phase = crate::ports::UnifiedSessionPhase::Publishing;
                }
                self.maybe_publish_unified_capture(key)?;
            }
            ImportEvent::UnifiedDocumentPage {
                request_id, result, ..
            } => {
                if self.current_unified_document_request != Some((key, request_id))
                    || !self
                        .view
                        .selected_unified_session
                        .as_ref()
                        .is_some_and(|selected| (selected.job_id, selected.generation) == key)
                {
                    return Ok(());
                }
                self.current_unified_document_request = None;
                match result {
                    Ok(page) => {
                        let current_revision = self
                            .view
                            .unified_parakeet_sessions
                            .iter()
                            .find(|s| (s.job_id, s.generation) == key)
                            .map(|s| s.document_revision);
                        if current_revision == Some(page.revision)
                            && self
                                .view
                                .selected_unified_session
                                .as_ref()
                                .is_some_and(|selected| {
                                    (selected.job_id, selected.generation) == key
                                })
                        {
                            self.view
                                .selected_unified_session
                                .as_mut()
                                .unwrap()
                                .document_page = Some(page);
                            self.view
                                .selected_unified_session
                                .as_mut()
                                .unwrap()
                                .document_error = None;
                        }
                    }
                    Err(message) => {
                        if let Some(selected) = self.view.selected_unified_session.as_mut()
                            && (selected.job_id, selected.generation) == key
                        {
                            selected.document_error = Some(message);
                        }
                    }
                }
            }
            ImportEvent::UnifiedAudioPartsPage {
                request_id, result, ..
            } => {
                if self.current_unified_audio_request != Some((key, request_id))
                    || !self
                        .view
                        .selected_unified_session
                        .as_ref()
                        .is_some_and(|selected| (selected.job_id, selected.generation) == key)
                {
                    return Ok(());
                }
                self.current_unified_audio_request = None;
                match result {
                    Ok(page) => {
                        let current_revision = self
                            .view
                            .unified_parakeet_sessions
                            .iter_mut()
                            .find(|s| (s.job_id, s.generation) == key);
                        if current_revision.is_some_and(|session| {
                            session.audio_parts_revision == 0
                                || session.audio_parts_revision == page.revision
                        }) && self
                            .view
                            .selected_unified_session
                            .as_ref()
                            .is_some_and(|selected| (selected.job_id, selected.generation) == key)
                        {
                            let session = self
                                .view
                                .unified_parakeet_sessions
                                .iter_mut()
                                .find(|s| (s.job_id, s.generation) == key)
                                .unwrap();
                            session.audio_parts_revision = page.revision;
                            self.view
                                .selected_unified_session
                                .as_mut()
                                .unwrap()
                                .audio_parts_page = Some(page);
                            self.view
                                .selected_unified_session
                                .as_mut()
                                .unwrap()
                                .audio_parts_error = None;
                        }
                    }
                    Err(message) => {
                        if let Some(selected) = self.view.selected_unified_session.as_mut()
                            && (selected.job_id, selected.generation) == key
                        {
                            selected.audio_parts_error = Some(message);
                        }
                    }
                }
            }
            ImportEvent::UnifiedPublicationArtifacts { result, .. } => {
                if let Some(session) = self
                    .view
                    .unified_parakeet_sessions
                    .iter_mut()
                    .find(|s| (s.job_id, s.generation) == key)
                {
                    match result {
                        Ok(files) => {
                            session.wav_path = Some(files.wav_path);
                            session.txt_path = Some(files.txt_path);
                            session.srt_path = Some(files.srt_path);
                            session.message = None;
                        }
                        Err(message) => session.message = Some(message),
                    }
                }
            }
            ImportEvent::Published { job_id, generation } => {
                if let Some(capture) = self.unified_captures.get(&key) {
                    self.io
                        .submit(ImportEffect::ScanHistory {
                            destination: capture.destination.clone(),
                        })
                        .map_err(|error| self.fail(error))?;
                }
                self.view.message = Some(format!(
                    "Transcription micro publiée ({}:{})",
                    job_id.0,
                    generation.get()
                ));
                if let Some(session) = self
                    .view
                    .unified_parakeet_sessions
                    .iter_mut()
                    .find(|s| (s.job_id, s.generation) == key)
                {
                    session.phase = crate::ports::UnifiedSessionPhase::Complete;
                }
                self.unified_captures.remove(&key);
            }
            ImportEvent::Failed { message, .. } => {
                self.view.message = Some(format!("Échec de la transcription micro : {message}"));
                if let Some(session) = self
                    .view
                    .unified_parakeet_sessions
                    .iter_mut()
                    .find(|s| (s.job_id, s.generation) == key)
                {
                    session.phase = crate::ports::UnifiedSessionPhase::Recoverable;
                    session.message = Some(message);
                }
                self.unified_captures.remove(&key);
            }
            ImportEvent::Stopped { .. } => {
                if let Some(session) = self
                    .view
                    .unified_parakeet_sessions
                    .iter_mut()
                    .find(|s| (s.job_id, s.generation) == key)
                {
                    session.phase = crate::ports::UnifiedSessionPhase::Recoverable;
                    session.message = Some("Session arrêtée avant publication".into());
                }
                self.unified_captures.remove(&key);
            }
            _ => {}
        }
        Ok(())
    }

    fn retry_pending_unified_capture_segments(&mut self) -> Result<(), ApplicationError> {
        let keys = self.unified_captures.keys().copied().collect::<Vec<_>>();
        for key in keys {
            self.retry_unified_capture_segment(key)?;
        }
        Ok(())
    }

    fn retry_unified_capture_segment(
        &mut self,
        key: (JobId, Generation),
    ) -> Result<(), ApplicationError> {
        loop {
            let Some(segment) = self
                .unified_captures
                .get(&key)
                .and_then(|capture| capture.pending_segments.front().cloned())
            else {
                return Ok(());
            };
            match self
                .io
                .submit(ImportEffect::PersistSegment(segment.clone()))
            {
                Ok(()) => {
                    let capture = self
                        .unified_captures
                        .get_mut(&key)
                        .ok_or(ApplicationError::InvalidCommand)?;
                    capture.pending_segments.pop_front();
                    capture.pending_sequences.insert(segment.segment_id.0);
                    capture.pending_persistence = capture.pending_sequences.len();
                }
                Err(PortError::Backpressure) => return Ok(()),
                Err(error) => return Err(self.fail(error)),
            }
        }
    }

    fn maybe_publish_unified_capture(
        &mut self,
        key: (JobId, Generation),
    ) -> Result<(), ApplicationError> {
        self.retry_unified_capture_segment(key)?;
        let Some(capture) = self.unified_captures.get(&key) else {
            return Ok(());
        };
        let ready = capture.stop_submitted
            && !capture.publish_submitted
            && capture.pending_segments.is_empty()
            && capture.pending_sequences.is_empty()
            && capture
                .last_sequence
                .is_some_and(|last| last.saturating_add(1) == capture.next_sequence);
        if ready {
            let capture = self.unified_captures.get(&key).unwrap();
            self.io
                .submit(ImportEffect::Publish {
                    job_id: capture.identity.job_id,
                    generation: capture.identity.generation,
                    last_sequence: capture.last_sequence.unwrap(),
                })
                .map_err(|error| self.fail(error))?;
            if let Some(capture) = self.unified_captures.get_mut(&key) {
                capture.publish_submitted = true;
            }
        }
        Ok(())
    }

    fn track_parakeet_capture_event(&mut self, event: &ImportEvent) {
        use crate::ports::{
            ParakeetCaptureIdentity, ParakeetCaptureView, ParakeetCaptureViewState,
        };

        let ensure_capture = |captures: &mut Vec<ParakeetCaptureView>, identity| {
            if let Some(index) = captures
                .iter()
                .position(|capture| capture.identity == identity)
            {
                return index;
            }
            captures.push(ParakeetCaptureView {
                identity,
                wav_path: None,
                direct_txt_path: None,
                direct_srt_path: None,
                companion_json_path: None,
                confirmed_segments: Vec::new(),
                provisional_text: None,
                provisional_range: None,
                provisional_sequence: None,
                unresolved: Vec::new(),
                capture_state: ParakeetCaptureViewState::Starting,
                capture_complete: None,
                capture_diagnostic: None,
                requested_compute: None,
                effective_backend: None,
                automatic_final: None,
                automatic_final_identity: None,
            });
            captures.len() - 1
        };

        match event {
            ImportEvent::ParakeetLiveStarted {
                job_id,
                generation,
                session_id,
                ..
            } => {
                let index = ensure_capture(
                    &mut self.view.parakeet_captures,
                    ParakeetCaptureIdentity {
                        job_id: *job_id,
                        generation: *generation,
                        session_id: *session_id,
                    },
                );
                let capture = &mut self.view.parakeet_captures[index];
                if matches!(
                    capture.capture_state,
                    ParakeetCaptureViewState::Finalized | ParakeetCaptureViewState::Failed
                ) {
                    return;
                }
                capture.capture_state = ParakeetCaptureViewState::Capturing;
                if let Some((identity, requested)) = self.active_live_compute
                    && identity
                        == (CommandIdentity {
                            job_id: *job_id,
                            generation: *generation,
                        })
                {
                    capture.requested_compute = Some(requested);
                }
                self.trim_parakeet_capture_views();
            }
            ImportEvent::ParakeetInferenceReady {
                job_id,
                generation,
                session_id,
                backend,
            } => {
                if let Some(capture) =
                    self.find_parakeet_capture_mut(*job_id, *generation, *session_id)
                    && matches!(
                        capture.capture_state,
                        ParakeetCaptureViewState::Capturing | ParakeetCaptureViewState::Finalizing
                    )
                {
                    capture.effective_backend = Some(backend.clone());
                }
            }
            ImportEvent::ParakeetHypothesis {
                job_id,
                generation,
                session_id,
                sequence,
                range,
                text,
                ..
            } => {
                if *sequence > 0
                    && range.sample_rate_hz == 16_000
                    && range.start_sample < range.end_sample
                    && !text.trim().is_empty()
                    && let Some(capture) =
                        self.find_parakeet_capture_mut(*job_id, *generation, *session_id)
                    && capture.capture_state == ParakeetCaptureViewState::Capturing
                    && capture
                        .provisional_sequence
                        .is_none_or(|prior| *sequence > prior)
                {
                    capture.provisional_text = Some(text.clone());
                    capture.provisional_range = Some(*range);
                    capture.provisional_sequence = Some(*sequence);
                }
            }
            ImportEvent::ParakeetConfirmed {
                job_id,
                generation,
                session_id,
                segment,
            } => {
                if segment.job_id == *job_id
                    && segment.generation == *generation
                    && segment.instance_id == *session_id
                    && segment.range.sample_rate_hz == 16_000
                    && segment.range.start_sample < segment.range.end_sample
                    && !segment.text.trim().is_empty()
                    && let Some(capture) =
                        self.find_parakeet_capture_mut(*job_id, *generation, *session_id)
                    && matches!(
                        capture.capture_state,
                        ParakeetCaptureViewState::Capturing | ParakeetCaptureViewState::Finalizing
                    )
                {
                    if let Some(prior) = capture
                        .confirmed_segments
                        .iter()
                        .find(|prior| prior.segment_id == segment.segment_id)
                    {
                        if prior != segment {
                            capture.capture_diagnostic = Some(
                                "Événement confirmé incohérent ignoré pour cette capture".into(),
                            );
                        }
                    } else {
                        capture.confirmed_segments.push(segment.clone());
                        capture.confirmed_segments.sort_by_key(|item| {
                            (
                                item.range.start_sample,
                                item.range.end_sample,
                                item.segment_id.0,
                            )
                        });
                        if capture
                            .provisional_range
                            .is_some_and(|range| range.end_sample <= segment.range.end_sample)
                        {
                            capture.provisional_text = None;
                            capture.provisional_range = None;
                            capture.provisional_sequence = None;
                        }
                    }
                }
            }
            ImportEvent::ParakeetUnresolved {
                job_id,
                generation,
                session_id,
                range,
            } => {
                if range.sample_rate_hz == 16_000
                    && range.start_sample < range.end_sample
                    && let Some(capture) =
                        self.find_parakeet_capture_mut(*job_id, *generation, *session_id)
                    && matches!(
                        capture.capture_state,
                        ParakeetCaptureViewState::Capturing | ParakeetCaptureViewState::Finalizing
                    )
                    && !capture.unresolved.contains(range)
                {
                    capture.unresolved.push(*range);
                    capture
                        .unresolved
                        .sort_by_key(|item| (item.start_sample, item.end_sample));
                }
            }
            ImportEvent::ParakeetResolved {
                job_id,
                generation,
                session_id,
                range,
            } => {
                if range.sample_rate_hz == 16_000
                    && range.start_sample < range.end_sample
                    && let Some(capture) =
                        self.find_parakeet_capture_mut(*job_id, *generation, *session_id)
                    && matches!(
                        capture.capture_state,
                        ParakeetCaptureViewState::Capturing | ParakeetCaptureViewState::Finalizing
                    )
                {
                    capture.unresolved.retain(|item| item != range);
                }
            }
            ImportEvent::ParakeetCaptureStopped {
                job_id,
                generation,
                session_id,
                wav_path,
                txt_path,
                srt_path,
                complete,
                final_segments,
                unresolved,
                diagnostic,
                automatic_final,
            } => {
                let identity = ParakeetCaptureIdentity {
                    job_id: *job_id,
                    generation: *generation,
                    session_id: *session_id,
                };
                let index = ensure_capture(&mut self.view.parakeet_captures, identity);
                let capture = &mut self.view.parakeet_captures[index];
                if matches!(
                    capture.capture_state,
                    ParakeetCaptureViewState::Finalized | ParakeetCaptureViewState::Failed
                ) {
                    return;
                }
                capture.capture_state = if wav_path.is_empty() {
                    ParakeetCaptureViewState::Failed
                } else {
                    ParakeetCaptureViewState::Finalized
                };
                if let Some((active_identity, requested)) = self.active_live_compute
                    && active_identity
                        == (CommandIdentity {
                            job_id: *job_id,
                            generation: *generation,
                        })
                {
                    capture.requested_compute = Some(requested);
                    self.active_live_compute = None;
                }
                if self.active_live_parakeet
                    == Some(CommandIdentity {
                        job_id: *job_id,
                        generation: *generation,
                    })
                {
                    self.active_live_parakeet = None;
                }
                capture.wav_path = (!wav_path.is_empty()).then(|| wav_path.clone());
                capture.direct_txt_path = txt_path.clone();
                capture.direct_srt_path = srt_path.clone();
                capture.companion_json_path = (!wav_path.is_empty()).then(|| {
                    std::path::Path::new(wav_path)
                        .with_extension("json")
                        .to_string_lossy()
                        .into_owned()
                });
                capture.capture_complete = Some(*complete);
                capture.capture_diagnostic = diagnostic.clone();
                capture.unresolved = unresolved.clone();
                for segment in final_segments {
                    if segment.job_id != *job_id
                        || segment.generation != *generation
                        || segment.instance_id != *session_id
                        || segment.range.sample_rate_hz != 16_000
                        || segment.range.start_sample >= segment.range.end_sample
                        || segment.text.trim().is_empty()
                    {
                        capture.capture_diagnostic =
                            Some("Segment final invalide ignoré dans la présentation".into());
                        continue;
                    }
                    if let Some(prior) = capture
                        .confirmed_segments
                        .iter()
                        .find(|prior| prior.segment_id == segment.segment_id)
                    {
                        if prior != segment {
                            capture.capture_diagnostic =
                                Some("Segment final incohérent ignoré dans la présentation".into());
                        }
                    } else {
                        capture.confirmed_segments.push(segment.clone());
                    }
                }
                capture.confirmed_segments.sort_by_key(|item| {
                    (
                        item.range.start_sample,
                        item.range.end_sample,
                        item.segment_id.0,
                    )
                });
                capture.provisional_text = None;
                capture.provisional_range = None;
                capture.provisional_sequence = None;
                if let Some(automatic) = automatic_final {
                    let final_identity = (automatic.request.job_id, automatic.request.generation);
                    capture.automatic_final_identity = Some(final_identity);
                    capture.requested_compute = Some(automatic.request.config.compute);
                    capture.companion_json_path =
                        Some(automatic.outputs.companion_json_path.clone());
                    capture.automatic_final = Some(crate::ports::AutomaticFinalTranscriptionView {
                        status: crate::ports::AutomaticFinalTranscriptionStatus::Pending,
                        txt_path: automatic.outputs.txt_path.clone(),
                        srt_path: automatic.outputs.srt_path.clone(),
                        model_sha256: automatic.request.model_sha256,
                        requested_compute: automatic.request.config.compute,
                        effective_backend: None,
                        diagnostic: None,
                    });
                } else {
                    capture.automatic_final = Some(crate::ports::AutomaticFinalTranscriptionView {
                        status:
                            crate::ports::AutomaticFinalTranscriptionStatus::FailedOrUnavailable,
                        txt_path: String::new(),
                        srt_path: String::new(),
                        model_sha256: [0; 32],
                        requested_compute: crate::domain::ComputeChoice::Auto,
                        effective_backend: None,
                        diagnostic: Some(
                            "Import automatique indisponible pour cette capture".into(),
                        ),
                    });
                }
                self.trim_parakeet_capture_views();
            }
            ImportEvent::Failed {
                job_id,
                generation,
                message,
            } => {
                let identity = CommandIdentity {
                    job_id: *job_id,
                    generation: *generation,
                };
                let is_parakeet_attempt = self.active_live_parakeet == Some(identity);
                let requested_compute = self.active_live_compute.and_then(|(identity, compute)| {
                    (identity
                        == CommandIdentity {
                            job_id: *job_id,
                            generation: *generation,
                        })
                    .then_some(compute)
                });
                if requested_compute.is_some() {
                    self.active_live_compute = None;
                }
                if is_parakeet_attempt {
                    self.active_live_parakeet = None;
                }
                let existing = self
                    .view
                    .parakeet_captures
                    .iter_mut()
                    .rev()
                    .find(|capture| {
                        capture.identity.job_id == *job_id
                            && capture.identity.generation == *generation
                    });
                if let Some(capture) = existing {
                    if capture.capture_state == ParakeetCaptureViewState::Starting {
                        capture.capture_state = ParakeetCaptureViewState::Failed;
                        capture.capture_diagnostic = Some(message.clone());
                    }
                } else if is_parakeet_attempt && let Some(requested_compute) = requested_compute {
                    self.view.parakeet_captures.push(ParakeetCaptureView {
                        identity: ParakeetCaptureIdentity {
                            job_id: *job_id,
                            generation: *generation,
                            session_id: 0,
                        },
                        wav_path: None,
                        direct_txt_path: None,
                        direct_srt_path: None,
                        companion_json_path: None,
                        confirmed_segments: Vec::new(),
                        provisional_text: None,
                        provisional_range: None,
                        provisional_sequence: None,
                        unresolved: Vec::new(),
                        capture_state: ParakeetCaptureViewState::Failed,
                        capture_complete: None,
                        capture_diagnostic: Some(message.clone()),
                        requested_compute: Some(requested_compute),
                        effective_backend: None,
                        automatic_final: None,
                        automatic_final_identity: None,
                    });
                    self.trim_parakeet_capture_views();
                }
            }
            ImportEvent::Stopped { job_id, generation } => {
                if self.active_live_compute.is_some_and(|(identity, _)| {
                    identity
                        == (CommandIdentity {
                            job_id: *job_id,
                            generation: *generation,
                        })
                }) {
                    self.active_live_compute = None;
                }
                if self.active_live_parakeet
                    == Some(CommandIdentity {
                        job_id: *job_id,
                        generation: *generation,
                    })
                {
                    self.active_live_parakeet = None;
                }
                if let Some(capture) =
                    self.view
                        .parakeet_captures
                        .iter_mut()
                        .rev()
                        .find(|capture| {
                            capture.identity.job_id == *job_id
                                && capture.identity.generation == *generation
                                && capture.capture_state != ParakeetCaptureViewState::Finalized
                        })
                {
                    capture.capture_state = ParakeetCaptureViewState::Failed;
                    capture.capture_diagnostic =
                        Some("Capture arrêtée avant confirmation de la finalisation WAV".into());
                }
            }
            ImportEvent::MicrophoneGainApplied { .. }
            | ImportEvent::MicrophoneGainRejected { .. } => {}
            _ => {}
        }
    }

    fn find_parakeet_capture_mut(
        &mut self,
        job_id: JobId,
        generation: Generation,
        session_id: u64,
    ) -> Option<&mut crate::ports::ParakeetCaptureView> {
        self.view.parakeet_captures.iter_mut().find(|capture| {
            capture.identity.job_id == job_id
                && capture.identity.generation == generation
                && capture.identity.session_id == session_id
        })
    }

    fn trim_parakeet_capture_views(&mut self) {
        while self.view.parakeet_captures.len() > MAX_PARAKEET_CAPTURE_VIEWS {
            let removable = self.view.parakeet_captures.iter().position(|capture| {
                capture.capture_state == crate::ports::ParakeetCaptureViewState::Finalized
                    && capture.automatic_final.as_ref().is_none_or(|final_view| {
                        matches!(
                        final_view.status,
                        crate::ports::AutomaticFinalTranscriptionStatus::ProducedUnverified
                            | crate::ports::AutomaticFinalTranscriptionStatus::FailedOrUnavailable
                    )
                    })
            });
            let Some(index) = removable else { break };
            self.view.parakeet_captures.remove(index);
        }
    }

    fn track_automatic_final_event(&mut self, event: &ImportEvent, identity: CommandIdentity) {
        let target = self.view.parakeet_captures.iter_mut().find(|capture| {
            capture.automatic_final_identity == Some((identity.job_id, identity.generation))
        });
        let Some(capture) = target else { return };
        let Some(final_view) = capture.automatic_final.as_mut() else {
            return;
        };
        match event {
            ImportEvent::Ready { backend, .. } => {
                final_view.status = crate::ports::AutomaticFinalTranscriptionStatus::Running;
                final_view.effective_backend = Some(backend.clone());
            }
            ImportEvent::Published { .. } => {
                final_view.status =
                    crate::ports::AutomaticFinalTranscriptionStatus::ProducedUnverified;
                final_view.diagnostic = None;
            }
            ImportEvent::Failed { message, .. } => {
                final_view.status =
                    crate::ports::AutomaticFinalTranscriptionStatus::FailedOrUnavailable;
                final_view.diagnostic = Some(message.clone());
            }
            ImportEvent::Stopped { .. } => {
                final_view.status =
                    crate::ports::AutomaticFinalTranscriptionStatus::FailedOrUnavailable;
                final_view.diagnostic = Some("Import final automatique arrêté".into());
            }
            _ => {}
        }
    }

    fn set_capture_automatic_final_status(
        &mut self,
        identity: CommandIdentity,
        status: crate::ports::AutomaticFinalTranscriptionStatus,
        diagnostic: Option<String>,
        backend: Option<String>,
    ) {
        if let Some(capture) = self.view.parakeet_captures.iter_mut().find(|capture| {
            capture.automatic_final_identity == Some((identity.job_id, identity.generation))
        }) && let Some(final_view) = capture.automatic_final.as_mut()
        {
            final_view.status = status;
            final_view.diagnostic = diagnostic;
            if backend.is_some() {
                final_view.effective_backend = backend;
            }
        }
    }

    fn mark_automatic_final_failed(&mut self, identity: CommandIdentity, message: String) {
        self.set_capture_automatic_final_status(
            identity,
            crate::ports::AutomaticFinalTranscriptionStatus::FailedOrUnavailable,
            Some(message.clone()),
            None,
        );
        if self.automatic_final_identity == Some(identity) {
            self.automatic_final_identity = None;
            self.automatic_final_state = None;
            let _ = self.start_next_automatic_final();
        }
    }

    fn start_next_automatic_final(&mut self) -> Result<(), ApplicationError> {
        if self.automatic_final_identity.is_some() {
            return Ok(());
        }
        let Some(automatic) = self.automatic_final_queue.pop_front() else {
            return Ok(());
        };
        let identity = CommandIdentity {
            job_id: automatic.request.job_id,
            generation: automatic.request.generation,
        };
        self.automatic_final_identity = Some(identity);
        self.automatic_final_state = Some(AutomaticFinalJobState {
            request: automatic.request.clone(),
            next_sequence: 1,
            pending_persistence: 0,
            pending_segments: std::collections::VecDeque::new(),
            pending_sequences: std::collections::BTreeSet::new(),
            ended_at: None,
            max_segment_offset: 0,
            publish_submitted: false,
        });
        if let Err(error) = self
            .io
            .submit(ImportEffect::StartAutomaticFinalTranscription(automatic))
        {
            self.mark_automatic_final_failed(identity, format!("{error:?}"));
        }
        Ok(())
    }

    fn retry_automatic_final_segment(&mut self) -> Result<bool, ApplicationError> {
        let Some(segment) = self
            .automatic_final_state
            .as_ref()
            .and_then(|state| state.pending_segments.front().cloned())
        else {
            return Ok(true);
        };
        match self
            .io
            .submit(ImportEffect::PersistSegment(segment.clone()))
        {
            Ok(()) => {}
            Err(PortError::Backpressure) => return Ok(false),
            Err(error) => {
                let identity = self
                    .automatic_final_identity
                    .ok_or(ApplicationError::InvalidCommand)?;
                self.mark_automatic_final_failed(identity, format!("{error:?}"));
                return Ok(true);
            }
        }
        let state = self
            .automatic_final_state
            .as_mut()
            .ok_or(ApplicationError::InvalidCommand)?;
        let queued = state
            .pending_segments
            .pop_front()
            .ok_or(ApplicationError::InvalidCommand)?;
        let sequence = queued.segment_id.0;
        if queued != segment || !state.pending_sequences.insert(sequence) {
            return Err(ApplicationError::InvalidCommand);
        }
        state.next_sequence = state
            .next_sequence
            .checked_add(1)
            .ok_or(ApplicationError::InvalidCommand)?;
        state.max_segment_offset = state.max_segment_offset.max(segment.range.end_sample);
        state.pending_persistence += 1;
        Ok(true)
    }

    fn handle_automatic_final_event(
        &mut self,
        event: ImportEvent,
        identity: CommandIdentity,
    ) -> Result<(), ApplicationError> {
        self.track_automatic_final_event(&event, identity);
        let is_current_view = self.automatic_final_view_identity == Some(identity)
            && self.active.is_none_or(|active| active == identity);
        match event {
            ImportEvent::Prepared(request) => {
                let state = self
                    .automatic_final_state
                    .as_mut()
                    .ok_or(ApplicationError::InvalidCommand)?;
                let mut expected = state.request.clone();
                expected.source_sha256 = request.source_sha256;
                expected.source_samples = request.source_samples;
                if request.source_sha256.is_none()
                    || request.source_samples.is_none_or(|samples| samples == 0)
                    || expected != request
                {
                    return Err(ApplicationError::InvalidCommand);
                }
                state.request = request.clone();
                self.io
                    .submit(ImportEffect::StartWorker(request))
                    .map_err(|error| self.fail(error))?;
            }
            ImportEvent::Ready { backend, .. } => {
                if is_current_view
                    && let Some(view) = self.view.automatic_final_transcription.as_mut()
                {
                    view.status = crate::ports::AutomaticFinalTranscriptionStatus::Running;
                    view.effective_backend = Some(backend);
                }
            }
            ImportEvent::Progress {
                completed_samples,
                total_samples,
                ..
            } => {
                let state = self
                    .automatic_final_state
                    .as_ref()
                    .ok_or(ApplicationError::InvalidCommand)?;
                let expected = state.request.source_samples;
                if expected.is_none_or(|limit| completed_samples > limit)
                    || (total_samples > 0
                        && (completed_samples != total_samples || Some(total_samples) != expected))
                {
                    return Err(ApplicationError::InvalidCommand);
                }
            }
            ImportEvent::Segment(segment) => {
                let state = self
                    .automatic_final_state
                    .as_mut()
                    .ok_or(ApplicationError::InvalidCommand)?;
                let expected_sequence = state
                    .next_sequence
                    .checked_add(state.pending_segments.len() as u64)
                    .ok_or(ApplicationError::InvalidCommand)?;
                if segment.job_id != identity.job_id
                    || segment.generation != identity.generation
                    || segment.segment_id.0 != expected_sequence
                    || segment.range.sample_rate_hz != 16_000
                    || segment.range.start_sample >= segment.range.end_sample
                    || state
                        .request
                        .source_samples
                        .is_some_and(|limit| segment.range.end_sample > limit)
                    || segment.text.trim().is_empty()
                {
                    return Err(ApplicationError::InvalidCommand);
                }
                if state.pending_segments.len() >= MAX_PENDING_AUTOMATIC_FINAL_SEGMENTS {
                    let _ = self.io.submit(ImportEffect::Stop {
                        job_id: identity.job_id,
                        generation: identity.generation,
                    });
                    self.mark_automatic_final_failed(
                        identity,
                        "Trop de segments de transcription finale en attente ; capture et WAV conservés".into(),
                    );
                    return Ok(());
                }
                state.pending_segments.push_back(segment);
                self.retry_automatic_final_segment()?;
            }
            ImportEvent::Persisted { sequence, .. } => {
                let state = self
                    .automatic_final_state
                    .as_mut()
                    .ok_or(ApplicationError::InvalidCommand)?;
                if !state.pending_sequences.remove(&sequence) {
                    return Err(ApplicationError::InvalidCommand);
                }
                state.pending_persistence = state
                    .pending_persistence
                    .checked_sub(1)
                    .ok_or(ApplicationError::InvalidCommand)?;
                self.maybe_publish_automatic_final()?;
            }
            ImportEvent::End {
                last_sequence,
                last_offset,
                ..
            } => {
                let state = self
                    .automatic_final_state
                    .as_mut()
                    .ok_or(ApplicationError::InvalidCommand)?;
                let expected_last_sequence = state
                    .next_sequence
                    .checked_add(state.pending_segments.len() as u64)
                    .and_then(|next| next.checked_sub(1))
                    .ok_or(ApplicationError::InvalidCommand)?;
                if last_sequence != expected_last_sequence
                    || last_offset == 0
                    || state.request.source_samples != Some(last_offset)
                    || state.max_segment_offset > last_offset
                {
                    return Err(ApplicationError::InvalidCommand);
                }
                state.ended_at = Some(last_sequence);
                self.maybe_publish_automatic_final()?;
            }
            ImportEvent::Published { .. } => {
                if !self
                    .automatic_final_state
                    .as_ref()
                    .is_some_and(|state| state.publish_submitted)
                {
                    return Err(ApplicationError::InvalidCommand);
                }
                if is_current_view
                    && let Some(view) = self.view.automatic_final_transcription.as_mut()
                {
                    view.status =
                        crate::ports::AutomaticFinalTranscriptionStatus::ProducedUnverified;
                    view.diagnostic = None;
                }
                self.automatic_final_identity = None;
                self.automatic_final_state = None;
                self.start_next_automatic_final()?;
            }
            ImportEvent::Failed { message, .. } => {
                self.mark_automatic_final_failed(identity, message);
            }
            ImportEvent::Stopped { .. } => {
                self.mark_automatic_final_failed(identity, "Import final automatique arrêté".into())
            }
            _ => return Err(ApplicationError::InvalidCommand),
        }
        Ok(())
    }

    fn maybe_publish_automatic_final(&mut self) -> Result<(), ApplicationError> {
        let publish = self.automatic_final_state.as_mut().and_then(|state| {
            (state.pending_persistence == 0 && state.pending_segments.is_empty())
                .then(|| state.ended_at.take())
                .flatten()
                .map(|last_sequence| {
                    (
                        CommandIdentity {
                            job_id: state.request.job_id,
                            generation: state.request.generation,
                        },
                        last_sequence,
                    )
                })
        });
        if let Some((identity, last_sequence)) = publish {
            match self.io.submit(ImportEffect::Publish {
                job_id: identity.job_id,
                generation: identity.generation,
                last_sequence,
            }) {
                Ok(()) => {
                    if let Some(state) = self.automatic_final_state.as_mut() {
                        state.publish_submitted = true;
                    }
                }
                Err(error) => self.mark_automatic_final_failed(identity, format!("{error:?}")),
            }
        }
        Ok(())
    }

    fn maybe_publish(
        &mut self,
        job_id: JobId,
        generation: Generation,
    ) -> Result<(), ApplicationError> {
        if self.pending_persistence == 0
            && let Some(last_sequence) = self.ended_at
        {
            self.io
                .submit(ImportEffect::Publish {
                    job_id,
                    generation,
                    last_sequence,
                })
                .map_err(|e| self.fail(e))?;
            self.ended_at = None;
            self.view.active_job = Some((job_id, JobState::Finalizing));
        }
        Ok(())
    }

    fn append_parakeet_confirmed(
        &mut self,
        session_id: u64,
        segment: crate::ports::WorkerSegment,
    ) -> Result<(), ApplicationError> {
        if segment.instance_id != session_id
            || segment.segment_id.0 == 0
            || segment.range.sample_rate_hz != 16_000
            || segment.range.start_sample >= segment.range.end_sample
            || segment.text.trim().is_empty()
            || self.parakeet_confirmed.iter().any(|existing| {
                existing.range.start_sample < segment.range.end_sample
                    && segment.range.start_sample < existing.range.end_sample
                    && existing.segment_id != segment.segment_id
            })
        {
            return Err(ApplicationError::InvalidCommand);
        }
        if self
            .parakeet_confirmed
            .iter()
            .any(|existing| existing.segment_id == segment.segment_id)
        {
            return Err(ApplicationError::InvalidCommand);
        }
        self.next_sequence = self.next_sequence.max(
            segment
                .segment_id
                .0
                .checked_add(1)
                .ok_or(ApplicationError::InvalidCommand)?,
        );
        self.parakeet_confirmed.push(segment);
        self.parakeet_confirmed.sort_by_key(|item| {
            (
                item.range.start_sample,
                item.range.end_sample,
                item.segment_id.0,
            )
        });
        self.view.confirmed_text = Some(
            self.parakeet_confirmed
                .iter()
                .map(|item| item.text.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
        );
        self.view.confirmed_samples = self
            .parakeet_confirmed
            .iter()
            .map(|item| item.range.end_sample)
            .max()
            .unwrap_or(0);
        Ok(())
    }

    fn accept_segment(
        &mut self,
        segment: crate::ports::WorkerSegment,
        _job_id: crate::domain::JobId,
        _generation: crate::domain::Generation,
    ) -> Result<(), ApplicationError> {
        if segment.segment_id.0 != self.next_sequence
            || segment.range.sample_rate_hz != 16_000
            || segment.range.start_sample >= segment.range.end_sample
            || self
                .request
                .as_ref()
                .and_then(|request| request.source_samples)
                .is_some_and(|limit| segment.range.end_sample > limit)
        {
            return Err(ApplicationError::InvalidCommand);
        }
        self.pending_segments.push_back(segment);
        let _ = self.retry_pending_segment()?;
        Ok(())
    }

    /// Try one bounded persistence submission. The event remains owned by the application
    /// until the adapter confirms that the effect entered its bounded queue.
    fn retry_pending_segment(&mut self) -> Result<bool, ApplicationError> {
        let Some(segment) = self.pending_segments.front() else {
            return Ok(true);
        };
        let next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or(ApplicationError::InvalidCommand)?;
        match self
            .io
            .submit(ImportEffect::PersistSegment(segment.clone()))
        {
            Ok(()) => {}
            Err(PortError::Backpressure) => return Ok(false),
            Err(error) => {
                self.pending_segments.clear();
                return Err(self.fail(error));
            }
        }
        let segment = self.pending_segments.pop_front().unwrap();
        self.max_segment_offset = self.max_segment_offset.max(segment.range.end_sample);
        self.next_sequence = next_sequence;
        self.pending_persistence += 1;
        Ok(true)
    }
}

impl<P: ImportIoPort> Application for ImportApplication<P> {
    fn set_gpu_candidate(
        &mut self,
        job_id: JobId,
        generation: Generation,
        candidate: Option<crate::gpu_inventory::GpuRuntimeIdentity>,
    ) -> Result<(), ApplicationError> {
        self.io
            .submit(ImportEffect::SetGpuCandidate {
                job_id,
                generation,
                candidate,
            })
            .map_err(|error| {
                self.view.message =
                    Some(format!("Sélection GPU non transmise au worker : {error:?}"));
                ApplicationError::PortUnavailable
            })
    }

    fn dispatch(&mut self, command: AppCommand) -> Result<AppView, ApplicationError> {
        let allowed_while_closing = matches!(
            &command,
            AppCommand::Refresh
                | AppCommand::RequestClose
                | AppCommand::SelectUnifiedSession { .. }
                | AppCommand::ReadUnifiedDocumentPage { .. }
                | AppCommand::ReadUnifiedAudioPartsPage { .. }
                | AppCommand::Stop { .. }
                | AppCommand::StopUnifiedLive { .. }
                | AppCommand::StopMicrophoneTest { .. }
                | AppCommand::Quit { .. }
                | AppCommand::RemoveQueued { .. }
                | AppCommand::CancelDuplicateImport
                | AppCommand::CancelSourceSnapshotRecovery
        );
        if self.view.close_requested && !allowed_while_closing {
            return Err(ApplicationError::InvalidCommand);
        }
        let admission = self.model_guard.as_ref().and_then(|guard| match &command {
            AppCommand::StartImport { request }
            | AppCommand::EnqueueImport { request }
            | AppCommand::ProcessQueued { request }
            | AppCommand::ResumeInterrupted { request } => {
                Some(guard.admit_import(request, crate::model_selection_app::InferencePath::Import))
            }
            AppCommand::StartLiveConfigured { request }
            | AppCommand::StartUnifiedLiveConfigured { request }
            | AppCommand::StartDirectLiveConfigured { request } => Some(guard.admit_live(request)),
            _ => None,
        });
        let admitted_model = admission
            .as_ref()
            .and_then(|result| result.as_ref().ok())
            .map(|selection| selection.admission.model);
        if let Some(Err(error)) = admission {
            let reason = match error {
                crate::model_selection_app::SelectionRefusal::UnknownHistoricalModelPath => {
                    "chemin ou empreinte non reconnu dans le catalogue"
                }
                crate::model_selection_app::SelectionRefusal::Capability(_) => {
                    "combinaison modèle, calcul ou langue non admissible"
                }
            };
            return Ok(self.refuse_model_command(reason));
        }

        match command {
            AppCommand::Refresh => {
                let identity = self.active;
                l3d_timing_line(
                    "refresh",
                    identity.map(|identity| identity.job_id),
                    identity.map(|identity| identity.generation),
                    None,
                );
                self.poll_once()?;
            }
            AppCommand::SelectUnifiedSession { job_id, generation } => {
                if !self
                    .view
                    .unified_parakeet_sessions
                    .iter()
                    .any(|session| (session.job_id, session.generation) == (job_id, generation))
                {
                    return Err(ApplicationError::InvalidCommand);
                }
                self.current_unified_document_request = None;
                self.current_unified_audio_request = None;
                self.view.selected_unified_session =
                    Some(crate::ports::UnifiedParakeetSelectionView {
                        job_id,
                        generation,
                        document_page: None,
                        preview: None,
                        audio_parts_page: None,
                        document_error: None,
                        audio_parts_error: None,
                    });
            }
            AppCommand::ReadUnifiedDocumentPage {
                job_id,
                generation,
                expected_revision,
                first_sequence,
            } => {
                let session_destination =
                    self.view.unified_parakeet_sessions.iter().find(|session| {
                        (session.job_id, session.generation) == (job_id, generation)
                    });
                if first_sequence == 0
                    || !self
                        .view
                        .selected_unified_session
                        .as_ref()
                        .is_some_and(|selected| {
                            (selected.job_id, selected.generation) == (job_id, generation)
                        })
                    || session_destination
                        .is_none_or(|session| session.document_revision != expected_revision)
                {
                    return Err(ApplicationError::InvalidCommand);
                }
                let request_id = self
                    .next_unified_page_request_id
                    .checked_add(1)
                    .ok_or(ApplicationError::InvalidCommand)?;
                self.io
                    .submit(ImportEffect::ReadUnifiedDocumentPage {
                        job_id,
                        generation,
                        request_id,
                        destination: session_destination.unwrap().destination.clone(),
                        expected_revision,
                        first_sequence,
                        limit: 128,
                    })
                    .map_err(|error| self.fail(error))?;
                self.next_unified_page_request_id = request_id;
                self.current_unified_document_request = Some(((job_id, generation), request_id));
            }
            AppCommand::ReadUnifiedAudioPartsPage {
                job_id,
                generation,
                expected_revision,
                offset,
            } => {
                let session_destination =
                    self.view.unified_parakeet_sessions.iter().find(|session| {
                        (session.job_id, session.generation) == (job_id, generation)
                    });
                if !self
                    .view
                    .selected_unified_session
                    .as_ref()
                    .is_some_and(|selected| {
                        (selected.job_id, selected.generation) == (job_id, generation)
                    })
                    || session_destination.is_none_or(|session| {
                        expected_revision != 0 && session.audio_parts_revision != expected_revision
                    })
                {
                    return Err(ApplicationError::InvalidCommand);
                }
                let request_id = self
                    .next_unified_page_request_id
                    .checked_add(1)
                    .ok_or(ApplicationError::InvalidCommand)?;
                self.io
                    .submit(ImportEffect::ReadUnifiedAudioPartsPage {
                        job_id,
                        generation,
                        request_id,
                        destination: session_destination.unwrap().destination.clone(),
                        expected_revision,
                        offset,
                        limit: 128,
                    })
                    .map_err(|error| self.fail(error))?;
                self.next_unified_page_request_id = request_id;
                self.current_unified_audio_request = Some(((job_id, generation), request_id));
            }
            AppCommand::RequestClose => {
                self.view.close_requested = true;
                if self.view.duplicate_import_prompt.is_some() {
                    self.dispatch(AppCommand::CancelDuplicateImport)?;
                }
                let sessions = self
                    .unified_captures
                    .iter()
                    .filter_map(|(key, capture)| (!capture.stop_submitted).then_some(*key))
                    .collect::<Vec<_>>();
                for (job_id, generation) in sessions {
                    self.io
                        .submit(ImportEffect::StopUnifiedLive { job_id, generation })
                        .map_err(|error| self.fail(error))?;
                    if let Some(capture) = self.unified_captures.get_mut(&(job_id, generation)) {
                        capture.stop_submitted = true;
                    }
                    if let Some(session) = self
                        .view
                        .unified_parakeet_sessions
                        .iter_mut()
                        .find(|s| (s.job_id, s.generation) == (job_id, generation))
                    {
                        session.phase = crate::ports::UnifiedSessionPhase::Finalizing;
                    }
                }
                if self.view.microphone_test_active && !self.close_active_stop_submitted {
                    if let Some(identity) = self.active {
                        self.io
                            .submit(ImportEffect::StopMicrophoneTest {
                                job_id: identity.job_id,
                                generation: identity.generation,
                            })
                            .map_err(|error| self.fail(error))?;
                        self.close_active_stop_submitted = true;
                    }
                } else if let Some(identity) = self.active
                    && !self.close_active_stop_submitted
                    && !self
                        .unified_captures
                        .contains_key(&(identity.job_id, identity.generation))
                {
                    self.dispatch(AppCommand::Stop {
                        job_id: identity.job_id,
                        generation: identity.generation,
                    })?;
                    self.close_active_stop_submitted = true;
                }
                self.drive_close_queue_removals()?;
                self.update_close_ready();
                return Ok(self.view.clone());
            }
            AppCommand::StartMicrophoneTest {
                job_id,
                generation,
                microphone,
                gain_db,
            } => {
                if self.active.is_some() || self.view.microphone_test_active {
                    return Err(ApplicationError::InvalidCommand);
                }
                let identity = CommandIdentity { job_id, generation };
                let requested_microphone = microphone.clone();
                self.io
                    .submit(ImportEffect::StartMicrophoneTest {
                        job_id,
                        generation,
                        microphone,
                        gain_db,
                    })
                    .map_err(|error| self.fail(error))?;
                self.view.microphone_acquisition =
                    Some(crate::microphone_level::MicrophoneAcquisitionContext::new(
                        job_id,
                        generation,
                        &requested_microphone,
                    ));
                self.view.microphone_spectrum = None;
                self.active = Some(identity);
                self.request = None;
                self.direct_live = false;
                self.stopping = None;
                self.view.active_generation = Some(generation);
                self.view.microphone_test_active = true;
                self.view.microphone_source_identity = None;
                self.view.microphone_meter = None;
                self.view.microphone_gain_requested_db = gain_db;
                self.view.microphone_gain_requested_revision = 0;
                self.view.microphone_gain_db = gain_db;
                self.view.microphone_gain_revision = 0;
                self.view.message = Some("Ouverture du test microphone…".into());
            }
            AppCommand::StopMicrophoneTest { job_id, generation } => {
                let identity = CommandIdentity { job_id, generation };
                if self.active != Some(identity) || !self.view.microphone_test_active {
                    return Err(ApplicationError::InvalidCommand);
                }
                self.io
                    .submit(ImportEffect::StopMicrophoneTest { job_id, generation })
                    .map_err(|error| self.fail(error))?;
                self.stopping = Some(identity);
                self.view.microphone_acquisition = None;
                self.view.microphone_meter = None;
                self.view.microphone_spectrum = None;
                self.view.message = Some("Arrêt du test microphone…".into());
            }
            AppCommand::ScanHistory { destination } => {
                self.io
                    .submit(ImportEffect::ScanHistory {
                        destination: destination.clone(),
                    })
                    .map_err(|e| self.fail(e))?;
                self.io
                    .submit(ImportEffect::ScanQueue { destination })
                    .map_err(|e| self.fail(e))?;
                self.view.history_scan_pending = true;
                self.view.queue_scan_pending = true;
            }
            AppCommand::InspectSourceSnapshotRecovery => {
                self.io
                    .submit(ImportEffect::InspectSourceSnapshotRecovery)
                    .map_err(|error| self.fail(error))?;
                self.view.source_snapshot_recovery = None;
            }
            AppCommand::ConfirmSourceSnapshotRecovery => {
                if self
                    .view
                    .source_snapshot_recovery
                    .as_ref()
                    .is_none_or(|candidates| {
                        !candidates.iter().any(|candidate| candidate.recoverable)
                    })
                {
                    return Err(ApplicationError::InvalidCommand);
                }
                self.io
                    .submit(ImportEffect::ConfirmSourceSnapshotRecovery)
                    .map_err(|error| self.fail(error))?;
                self.view.source_snapshot_recovery = None;
                self.view.message =
                    Some("Nouvelle vérification de sécurité avant récupération…".into());
            }
            AppCommand::CancelSourceSnapshotRecovery => {
                self.view.source_snapshot_recovery = None;
            }
            AppCommand::ResolveLivePublication { request } => {
                if self.active.is_some() || request.destination.trim().is_empty() {
                    return Err(ApplicationError::InvalidCommand);
                }
                self.io
                    .submit(ImportEffect::ResolveLivePublication(request))
                    .map_err(|error| self.fail(error))?;
                self.view.history_scan_pending = true;
                self.view.message = Some(
                    "Vérification du passage récupérable et résolution explicite en cours…".into(),
                );
            }
            AppCommand::RemoveQueued { job_id, generation } => {
                self.io
                    .submit(ImportEffect::RemoveQueued {
                        job_id,
                        generation,
                        destination: self
                            .view
                            .queue
                            .iter()
                            .find(|item| {
                                item.request.job_id == job_id
                                    && item.request.generation == generation
                            })
                            .map(|item| item.request.destination.clone())
                            .ok_or(ApplicationError::InvalidCommand)?,
                    })
                    .map_err(|e| self.fail(e))?;
                if self.view.close_requested {
                    let identity = CommandIdentity { job_id, generation };
                    if !self.close_queue_removals.contains(&identity) {
                        self.close_queue_removals.push(identity);
                    }
                }
            }
            AppCommand::StartUnifiedLiveConfigured { request } => {
                let import_can_share = self.active.is_some()
                    && self.request.as_ref().is_some_and(|active| {
                        active.strategy
                            == crate::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
                            && active.model_path == request.model_path
                            && active.model_sha256 == request.model_sha256
                            && active.config.compute == request.config.compute
                    });
                let open_capture_exists = self
                    .unified_captures
                    .values()
                    .any(|capture| !capture.stop_submitted);
                let admitted_sessions = self.unified_captures.len()
                    + if import_can_share && self.active.is_some() {
                        1
                    } else {
                        0
                    };
                let no_terminal_card_to_evict =
                    !self.view.unified_parakeet_sessions.iter().any(|session| {
                        matches!(
                            session.phase,
                            crate::ports::UnifiedSessionPhase::Complete
                                | crate::ports::UnifiedSessionPhase::Recoverable
                        )
                    });
                if open_capture_exists
                    || admitted_sessions >= 2
                    || (self.view.unified_parakeet_sessions.len() >= MAX_PARAKEET_CAPTURE_VIEWS
                        && no_terminal_card_to_evict)
                    || (self.active.is_some() && !import_can_share)
                    || self
                        .unified_captures
                        .contains_key(&(request.job_id, request.generation))
                    || self.active
                        == Some(CommandIdentity {
                            job_id: request.job_id,
                            generation: request.generation,
                        })
                {
                    return Err(ApplicationError::InvalidCommand);
                }
                let identity = CommandIdentity {
                    job_id: request.job_id,
                    generation: request.generation,
                };
                let destination = request.destination.clone();
                let source_path = std::path::PathBuf::from(&request.destination)
                    .join("transcriptions")
                    .join(format!("{:032x}", request.job_id.0))
                    .join(request.generation.get().to_string())
                    .join("parakeet-unified/source.wav")
                    .to_string_lossy()
                    .into_owned();
                let microphone = request.microphone.clone();
                let microphone_gain_db = request.microphone_gain_db;
                self.io
                    .submit(ImportEffect::StartUnifiedLive(request))
                    .map_err(|error| self.fail(error))?;
                self.view.microphone_source_identity = match microphone {
                    crate::microphone::MicrophoneChoice::Device(identity) => Some(identity),
                    crate::microphone::MicrophoneChoice::WindowsDefault => None,
                };
                self.view.microphone_gain_db = microphone_gain_db;
                self.view.microphone_gain_requested_db = microphone_gain_db;
                self.view.microphone_gain_requested_revision = 0;
                self.view.microphone_gain_revision = 0;
                self.view.microphone_gain_applied_first_frame = None;
                self.microphone_gain_view_owner = Some(identity);
                self.unified_captures.insert(
                    (identity.job_id, identity.generation),
                    UnifiedCaptureAdmission {
                        identity,
                        destination: destination.clone(),
                        stop_submitted: false,
                        publish_submitted: false,
                        microphone_gain_requested_revision: 0,
                        microphone_gain_requested_target_identity: None,
                        next_sequence: 1,
                        pending_persistence: 0,
                        pending_segments: std::collections::VecDeque::new(),
                        pending_sequences: std::collections::BTreeSet::new(),
                        last_sequence: None,
                        total_samples: None,
                    },
                );
                self.add_unified_session_card(
                    identity.job_id,
                    identity.generation,
                    destination,
                    source_path,
                    true,
                );
                return Ok(self.view.clone());
            }
            command @ (AppCommand::StartLiveConfigured { .. }
            | AppCommand::StartDirectLiveConfigured { .. }) => {
                let direct = matches!(&command, AppCommand::StartDirectLiveConfigured { .. });
                let request = match command {
                    AppCommand::StartLiveConfigured { request }
                    | AppCommand::StartDirectLiveConfigured { request } => request,
                    _ => unreachable!(),
                };
                let automatic_final_pending = self.automatic_final_identity.is_some()
                    || !self.automatic_final_queue.is_empty();
                if self.active.is_some()
                    || (automatic_final_pending && !direct)
                    || (automatic_final_pending
                        && self.automatic_final_state.as_ref().is_some_and(|state| {
                            state.request.model_sha256 != request.model_sha256
                        }))
                {
                    return Err(ApplicationError::InvalidCommand);
                }
                let identity = CommandIdentity {
                    job_id: request.job_id,
                    generation: request.generation,
                };
                let requested_compute = request.config.compute;
                let requested_microphone_gain_db = request.microphone_gain_db;
                let requested_microphone = request.microphone.clone();
                let requested_microphone_identity = match &request.microphone {
                    crate::microphone::MicrophoneChoice::Device(identity) => Some(identity.clone()),
                    crate::microphone::MicrophoneChoice::WindowsDefault => None,
                };
                self.auto_stop_after_speech_samples = request
                    .auto_stop_after_speech_samples
                    .filter(|limit| *limit > 0);
                let effect = if direct {
                    ImportEffect::StartDirectLive(request)
                } else {
                    ImportEffect::StartLive(request)
                };
                self.io.submit(effect).map_err(|e| self.fail(e))?;
                self.view.microphone_acquisition =
                    Some(crate::microphone_level::MicrophoneAcquisitionContext::new(
                        identity.job_id,
                        identity.generation,
                        &requested_microphone,
                    ));
                self.view.microphone_spectrum = None;
                self.request = None;
                self.direct_live = direct;
                self.active_live_compute = direct.then_some((identity, requested_compute));
                self.active_live_parakeet = (direct
                    && admitted_model == Some(crate::model_catalog::ModelId::ParakeetTdt06BV3))
                .then_some(identity);
                self.active = Some(identity);
                self.stopping = None;
                self.next_sequence = 1;
                self.pending_persistence = 0;
                self.pending_live_segments.clear();
                self.max_segment_offset = 0;
                self.ended_at = None;
                self.final_source_samples = None;
                self.view.progress = None;
                self.view.diagnostics_degraded = false;
                self.view.active_job = Some((identity.job_id, JobState::Preparing));
                self.view.active_generation = Some(identity.generation);
                self.view.captured_samples = 0;
                self.view.admitted_samples = 0;
                self.view.audio_durable_samples = 0;
                self.view.confirmed_fragment_samples = 0;
                self.view.confirmed_samples = 0;
                self.view.speech_samples = 0;
                self.view.provisional_text = None;
                self.view.provisional_sequence = None;
                self.view.provisional_session_id = None;
                self.view.provisional_range = None;
                self.view.provisional_captured_at_unix_ms = None;
                self.view.provisional_available_at_unix_ms = None;
                self.view.confirmed_text = None;
                self.parakeet_confirmed.clear();
                self.view.parakeet_unresolved.clear();
                self.view.direct_first_sample_at_unix_ms = None;
                self.view.direct_first_admitted_at_unix_ms = None;
                self.view.direct_ready_at_unix_ms = None;
                self.view.direct_capture_phase = direct.then_some(DirectCapturePhase::Initializing);
                self.view.direct_engine_phase = direct.then_some(DirectEnginePhase::Preparing);
                if direct {
                    self.view.microphone_source_identity = requested_microphone_identity;
                    self.view.microphone_meter = None;
                    self.view.microphone_gain_db = 0;
                    self.view.microphone_gain_revision = 0;
                    self.view.microphone_gain_applied_first_frame = None;
                }
                self.view.microphone_gain_requested_db = requested_microphone_gain_db;
                self.view.microphone_gain_requested_revision = 0;
                self.view.message = Some(if direct {
                    "Microphone en cours d'initialisation".into()
                } else {
                    "Préparation du staging durable et du worker…".into()
                });
            }
            AppCommand::SetMicrophoneGain {
                job_id,
                generation,
                target_identity,
                expected_revision,
                gain,
            } => {
                let identity = CommandIdentity { job_id, generation };
                let unified_capture_active = self
                    .unified_captures
                    .get(&(job_id, generation))
                    .is_some_and(|capture| !capture.stop_submitted);
                let legacy_capture_active = self.active == Some(identity)
                    && (self.direct_live || self.view.microphone_test_active)
                    && self.stopping != Some(identity);
                if !unified_capture_active && !legacy_capture_active {
                    return Err(ApplicationError::InvalidCommand);
                }
                let current_revision = if unified_capture_active {
                    self.unified_captures
                        .get(&(job_id, generation))
                        .map(|capture| capture.microphone_gain_requested_revision)
                        .ok_or(ApplicationError::InvalidCommand)?
                } else {
                    self.view.microphone_gain_requested_revision
                };
                if expected_revision != current_revision {
                    return Err(ApplicationError::InvalidCommand);
                }
                let Some(next_revision) = expected_revision.checked_add(1) else {
                    return Err(ApplicationError::InvalidCommand);
                };
                let requested_target_identity = target_identity.clone();
                self.io
                    .submit(ImportEffect::SetMicrophoneGain {
                        job_id,
                        generation,
                        target_identity,
                        expected_revision,
                        gain_db: gain.value(),
                    })
                    .map_err(|error| self.fail(error))?;
                if unified_capture_active {
                    let capture = self
                        .unified_captures
                        .get_mut(&(job_id, generation))
                        .ok_or(ApplicationError::InvalidCommand)?;
                    capture.microphone_gain_requested_revision = next_revision;
                    capture.microphone_gain_requested_target_identity =
                        Some(requested_target_identity);
                    self.microphone_gain_view_owner = Some(identity);
                }
                self.view.microphone_gain_requested_db = gain.value();
                self.view.microphone_gain_requested_revision = next_revision;
            }
            AppCommand::FinishLiveOnCpu { job_id, generation } => {
                let identity = CommandIdentity { job_id, generation };
                if self.active != Some(identity)
                    || self.view.active_job != Some((job_id, JobState::AwaitingChoice))
                {
                    return Err(ApplicationError::InvalidCommand);
                }
                self.io
                    .submit(ImportEffect::FinishLiveOnCpu { job_id, generation })
                    .map_err(|error| self.fail(error))?;
                self.view.active_job = Some((job_id, JobState::Preparing));
                self.view.message = Some(
                    "Choix CPU confirmé ; finalisation du passage conservé sans relancer le micro…"
                        .into(),
                );
            }
            AppCommand::StartImport { request } => {
                if self.active.is_some() {
                    return Err(ApplicationError::InvalidCommand);
                }
                let identity = CommandIdentity {
                    job_id: request.job_id,
                    generation: request.generation,
                };
                self.io
                    .submit(ImportEffect::Prepare(request.clone()))
                    .map_err(|e| self.fail(e))?;
                if request.strategy
                    == crate::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
                {
                    self.add_unified_session_card(
                        request.job_id,
                        request.generation,
                        request.destination.clone(),
                        request.source_path.clone(),
                        false,
                    );
                }
                l3d_timing_line(
                    "start_import",
                    Some(request.job_id),
                    Some(request.generation),
                    None,
                );
                self.request = Some(request);
                self.auto_stop_after_speech_samples = None;
                self.active = Some(identity);
                self.stopping = None;
                self.next_sequence = 1;
                self.pending_persistence = 0;
                self.ended_at = None;
                self.max_segment_offset = 0;
                self.final_source_samples = None;
                self.view.progress = None;
                self.view.diagnostics_degraded = false;
                self.view.active_job = Some((identity.job_id, JobState::Preparing));
                self.view.active_generation = Some(identity.generation);
                self.view.message = Some("Vérification de la source et du modèle…".into());
            }
            AppCommand::EnqueueImport { request } => {
                if self.direct_live && self.active.is_some() {
                    return Err(ApplicationError::InvalidCommand);
                }
                let identity = CommandIdentity {
                    job_id: request.job_id,
                    generation: request.generation,
                };
                self.io
                    .submit(ImportEffect::Enqueue(request))
                    .map_err(|e| self.fail(e))?;
                self.pending_enqueues.push(identity);
                self.view.message =
                    Some("Vérification de la source et ajout à la file durable en cours…".into());
            }
            AppCommand::ResolveDuplicateImport { prompt, choice } => {
                if self.active.is_some() {
                    return Err(ApplicationError::InvalidCommand);
                }
                let identity = CommandIdentity {
                    job_id: prompt.request.job_id,
                    generation: prompt.request.generation,
                };
                if self.view.duplicate_import_prompt.as_ref() != Some(&prompt) {
                    return Err(ApplicationError::InvalidCommand);
                }
                let is_resume_confirmation = prompt.resume_from.is_some();
                let confirmed_request = prompt.request.clone();
                self.io
                    .submit(ImportEffect::ConfirmDuplicateImport { prompt, choice })
                    .map_err(|e| self.fail(e))?;
                if is_resume_confirmation {
                    self.request = Some(confirmed_request);
                    self.active = Some(identity);
                    self.stopping = None;
                    self.next_sequence = 1;
                    self.pending_persistence = 0;
                    self.ended_at = None;
                    self.max_segment_offset = 0;
                    self.final_source_samples = None;
                    self.view.active_job = Some((identity.job_id, JobState::Queued));
                    self.view.active_generation = Some(identity.generation);
                    self.view.progress = None;
                } else {
                    self.pending_enqueues.push(identity);
                }
                self.view.duplicate_import_prompt = None;
                self.view.message = Some(if is_resume_confirmation {
                    "Confirmation de reprise en cours…".into()
                } else {
                    "Confirmation en cours…".into()
                });
            }
            AppCommand::CancelDuplicateImport => {
                if let Some(prompt) = self.view.duplicate_import_prompt.clone() {
                    self.io
                        .submit(ImportEffect::ReleaseSourceBinding {
                            job_id: prompt.request.job_id,
                            generation: prompt.request.generation,
                            destination: prompt.request.destination.clone(),
                        })
                        .map_err(|error| self.fail(error))?;
                    self.view.duplicate_import_prompt = None;
                }
                self.view.message = Some("Nouvelle transcription annulée.".into());
            }
            AppCommand::ResolveImportPublication {
                destination,
                job_id,
                generation,
            } => {
                if self.active.is_some() || destination.trim().is_empty() {
                    return Err(ApplicationError::InvalidCommand);
                }
                self.io
                    .submit(ImportEffect::ResolveImportPublication {
                        destination,
                        job_id,
                        generation,
                    })
                    .map_err(|error| self.fail(error))?;
                self.view.history_scan_pending = true;
                self.view.message =
                    Some("Vérification et récupération de la publication interrompue…".into());
            }
            AppCommand::ResumeReplacementCleanup {
                destination,
                cleanup_id,
            } => {
                if self.active.is_some() || destination.trim().is_empty() || cleanup_id.is_empty() {
                    return Err(ApplicationError::InvalidCommand);
                }
                self.io
                    .submit(ImportEffect::ResumeReplacementCleanup {
                        destination,
                        cleanup_id,
                    })
                    .map_err(|error| self.fail(error))?;
                self.view.message =
                    Some("Vérification des éléments restants et reprise demandée…".into());
            }
            AppCommand::ProcessQueued { request } => {
                if self.active.is_some() {
                    return Err(ApplicationError::InvalidCommand);
                }
                let identity = CommandIdentity {
                    job_id: request.job_id,
                    generation: request.generation,
                };
                self.io
                    .submit(ImportEffect::ProcessQueued(request.clone()))
                    .map_err(|e| self.fail(e))?;
                if request.strategy
                    == crate::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
                {
                    self.add_unified_session_card(
                        request.job_id,
                        request.generation,
                        request.destination.clone(),
                        request.source_path.clone(),
                        false,
                    );
                }
                self.request = Some(request);
                self.active = Some(identity);
                self.stopping = None;
                self.next_sequence = 1;
                self.pending_persistence = 0;
                self.ended_at = None;
                self.max_segment_offset = 0;
                self.final_source_samples = None;
                self.view.progress = None;
                self.view.diagnostics_degraded = false;
                self.view.active_job = Some((identity.job_id, JobState::Queued));
                self.view.active_generation = Some(identity.generation);
                self.view.message = Some("Vérification du choix explicite et préparation…".into());
            }
            AppCommand::ResumeInterrupted { request } => {
                if self.active.is_some() {
                    return Err(ApplicationError::InvalidCommand);
                }
                let previous = request;
                let mut resumed = previous.clone();
                resumed.generation = previous
                    .generation
                    .next()
                    .ok_or(ApplicationError::InvalidCommand)?;
                let identity = CommandIdentity {
                    job_id: resumed.job_id,
                    generation: resumed.generation,
                };
                self.io
                    .submit(ImportEffect::ResumeInterrupted {
                        previous: Box::new(previous),
                        request: resumed.clone(),
                    })
                    .map_err(|e| self.fail(e))?;
                if resumed.strategy
                    == crate::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
                {
                    self.add_unified_session_card(
                        resumed.job_id,
                        resumed.generation,
                        resumed.destination.clone(),
                        resumed.source_path.clone(),
                        false,
                    );
                }
                self.request = Some(resumed);
                self.active = Some(identity);
                self.stopping = None;
                self.next_sequence = 1;
                self.pending_persistence = 0;
                self.ended_at = None;
                self.max_segment_offset = 0;
                self.final_source_samples = None;
                self.view.progress = None;
                self.view.diagnostics_degraded = false;
                self.view.active_job = Some((identity.job_id, JobState::Queued));
                self.view.active_generation = Some(identity.generation);
                self.view.message = Some(
                    "Reprise confirmée : vérification du préfixe durable avant recalcul…".into(),
                );
            }
            AppCommand::StopUnifiedLive { job_id, generation } => {
                let key = (job_id, generation);
                let stop_submitted = self
                    .unified_captures
                    .get(&key)
                    .ok_or(ApplicationError::InvalidCommand)?
                    .stop_submitted;
                if !stop_submitted {
                    self.io
                        .submit(ImportEffect::StopUnifiedLive { job_id, generation })
                        .map_err(|error| self.fail(error))?;
                    if let Some(capture) = self.unified_captures.get_mut(&key) {
                        capture.stop_submitted = true;
                    }
                }
                return Ok(self.view.clone());
            }
            AppCommand::Stop { job_id, generation } | AppCommand::Quit { job_id, generation } => {
                let key = (job_id, generation);
                if self.unified_captures.contains_key(&key) {
                    let stop_submitted = self
                        .unified_captures
                        .get(&key)
                        .is_some_and(|capture| capture.stop_submitted);
                    if !stop_submitted {
                        self.io
                            .submit(ImportEffect::StopUnifiedLive { job_id, generation })
                            .map_err(|error| self.fail(error))?;
                        if let Some(capture) = self.unified_captures.get_mut(&key) {
                            capture.stop_submitted = true;
                        }
                    }
                    return Ok(self.view.clone());
                }
                if self.active != Some(CommandIdentity { job_id, generation }) {
                    return Err(ApplicationError::InvalidCommand);
                }
                match self.io.submit(ImportEffect::Stop { job_id, generation }) {
                    Ok(()) => {
                        self.stopping = Some(CommandIdentity { job_id, generation });
                        if self
                            .view
                            .microphone_acquisition
                            .as_ref()
                            .is_some_and(|context| {
                                context.job_id == job_id && context.generation == generation
                            })
                        {
                            self.view.microphone_acquisition = None;
                            self.view.microphone_spectrum = None;
                        }
                    }
                    Err(PortError::Committing) => {
                        if self
                            .view
                            .microphone_acquisition
                            .as_ref()
                            .is_some_and(|context| {
                                context.job_id == job_id && context.generation == generation
                            })
                        {
                            self.view.microphone_acquisition = None;
                            self.view.microphone_spectrum = None;
                        }
                        self.view.active_job = Some((job_id, JobState::Finalizing));
                        self.view.message = Some(
                            "Publication en cours ; l’arrêt est trop tardif. Attendez sa fin."
                                .into(),
                        );
                        return Ok(self.view.clone());
                    }
                    Err(error) => return Err(self.fail(error)),
                }
                for capture in &mut self.view.parakeet_captures {
                    if capture.identity.job_id == job_id
                        && capture.identity.generation == generation
                        && capture.capture_state
                            == crate::ports::ParakeetCaptureViewState::Capturing
                    {
                        capture.capture_state = crate::ports::ParakeetCaptureViewState::Finalizing;
                    }
                }
                self.view.active_job = Some((
                    job_id,
                    if self.request.is_none() {
                        JobState::Draining
                    } else {
                        JobState::Cancelling
                    },
                ));
                if self.direct_live {
                    self.view.direct_capture_phase = Some(DirectCapturePhase::Stopped);
                }
            }
            _ => return Err(ApplicationError::InvalidCommand),
        }
        Ok(self.view.clone())
    }

    fn view(&self) -> AppView {
        self.view.clone()
    }
}

impl ImportEvent {
    fn identity(&self) -> (JobId, Generation) {
        match self {
            Self::DirectCaptureActive {
                job_id, generation, ..
            } => (*job_id, *generation),
            Self::DirectConfirmed(segment) => (segment.job_id, segment.generation),
            Self::DirectPublished {
                job_id, generation, ..
            } => (*job_id, *generation),
            Self::MicrophoneGainApplied {
                job_id, generation, ..
            }
            | Self::MicrophoneGainRejected {
                job_id, generation, ..
            } => (*job_id, *generation),
            Self::MicrophoneTestStarted {
                job_id, generation, ..
            }
            | Self::MicrophoneTestStopped { job_id, generation } => (*job_id, *generation),
            Self::MicrophoneMeter(meter) => (meter.job_id, meter.generation),
            Self::MicrophoneSpectrum(update) => update.owner(),
            Self::LiveProvisional { segment, .. } => (segment.job_id, segment.generation),
            Self::ParakeetLiveStarted {
                job_id, generation, ..
            }
            | Self::ParakeetInferenceReady {
                job_id, generation, ..
            }
            | Self::ParakeetHypothesis {
                job_id, generation, ..
            }
            | Self::ParakeetConfirmed {
                job_id, generation, ..
            }
            | Self::ParakeetUnresolved {
                job_id, generation, ..
            }
            | Self::ParakeetResolved {
                job_id, generation, ..
            }
            | Self::ParakeetCaptureProgress {
                job_id, generation, ..
            }
            | Self::ParakeetAudioUnusable {
                job_id, generation, ..
            }
            | Self::ParakeetInferenceUnavailable {
                job_id, generation, ..
            }
            | Self::ParakeetCaptureStopped {
                job_id, generation, ..
            } => (*job_id, *generation),
            Self::LiveCounters {
                job_id, generation, ..
            } => (*job_id, *generation),
            Self::WindowFinished {
                job_id, generation, ..
            } => (*job_id, *generation),
            Self::AwaitingCpuChoice { job_id, generation } => (*job_id, *generation),
            Self::LiveAttemptReset {
                job_id, generation, ..
            } => (*job_id, *generation),
            Self::LiveRetryingOnCpu {
                job_id, generation, ..
            } => (*job_id, *generation),
            Self::LiveAttemptStarting { job_id, generation } => (*job_id, *generation),
            Self::LiveFinalizing { job_id, generation }
            | Self::Diagnostic {
                job_id, generation, ..
            } => (*job_id, *generation),
            Self::Ready {
                job_id, generation, ..
            }
            | Self::Progress {
                job_id, generation, ..
            }
            | Self::Persisted {
                job_id, generation, ..
            }
            | Self::End {
                job_id, generation, ..
            }
            | Self::Published { job_id, generation }
            | Self::UnifiedDocumentPage {
                job_id, generation, ..
            }
            | Self::UnifiedAudioPartsPage {
                job_id, generation, ..
            }
            | Self::UnifiedPublicationArtifacts {
                job_id, generation, ..
            }
            | Self::Failed {
                job_id, generation, ..
            }
            | Self::Stopped { job_id, generation } => (*job_id, *generation),
            Self::ResumeCheckpoint {
                job_id, generation, ..
            } => (*job_id, *generation),
            Self::Prepared(request) => (request.job_id, request.generation),
            Self::History(_)
            | Self::ReplacementCleanups(_)
            | Self::Queue(_)
            | Self::QueueAdmission { .. }
            | Self::DuplicateImportFound(_)
            | Self::SourceSnapshotRecoveryInspected(_)
            | Self::SourceSnapshotRecoveryCompleted { .. } => (JobId(0), Generation::first()),
            Self::Segment(segment) => (segment.job_id, segment.generation),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApplicationError {
    InvalidCommand,
    PortUnavailable,
    Failed(String),
}

/// Application owns orchestration; concrete infrastructure is supplied by a composition root.
pub trait Application {
    fn dispatch(&mut self, command: AppCommand) -> Result<AppView, ApplicationError>;
    fn view(&self) -> AppView;

    fn set_gpu_candidate(
        &mut self,
        job_id: JobId,
        generation: Generation,
        candidate: Option<crate::gpu_inventory::GpuRuntimeIdentity>,
    ) -> Result<(), ApplicationError> {
        let _ = (job_id, generation, candidate);
        Ok(())
    }
}

pub struct AppFacade<A> {
    application: A,
}

impl<A: Application> AppFacade<A> {
    pub fn new(application: A) -> Self {
        Self { application }
    }

    pub fn dispatch(&mut self, command: AppCommand) -> Result<AppView, ApplicationError> {
        self.application.dispatch(command)
    }

    pub fn view(&self) -> AppView {
        self.application.view()
    }

    pub fn set_gpu_candidate(
        &mut self,
        job_id: JobId,
        generation: Generation,
        candidate: Option<crate::gpu_inventory::GpuRuntimeIdentity>,
    ) -> Result<(), ApplicationError> {
        self.application
            .set_gpu_candidate(job_id, generation, candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ComputeChoice, LanguageChoice};
    use std::collections::VecDeque;

    #[derive(Default)]
    struct ScanTestIo {
        effects: Vec<ImportEffect>,
        events: VecDeque<ImportEvent>,
        stop_committing: bool,
    }

    impl ImportIoPort for ScanTestIo {
        fn submit(&mut self, effect: ImportEffect) -> Result<(), PortError> {
            if self.stop_committing && matches!(effect, ImportEffect::Stop { .. }) {
                return Err(PortError::Committing);
            }
            self.effects.push(effect);
            Ok(())
        }

        fn poll(&mut self) -> Result<Option<ImportEvent>, PortError> {
            Ok(self.events.pop_front())
        }
    }

    #[test]
    fn close_blocks_direct_and_indirect_admissions_without_submitting_effects() {
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.dispatch(AppCommand::RequestClose).unwrap();
        assert!(app.view.close_requested);
        assert!(app.view.close_ready);

        let job_id = JobId(991);
        let generation = Generation::first();
        let request = ImportRequest {
            strategy: crate::parakeet_session::TranscriptionStrategy::BoundedWindowsV1,
            job_id,
            generation,
            source_path: "source.wav".into(),
            source_sha256: None,
            source_samples: None,
            model_path: "model.nmodel".into(),
            model_sha256: [1; 32],
            destination: "archive".into(),
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Cpu,
            },
        };
        let prompt = DuplicateImportPrompt {
            request: request.clone(),
            source_sha256: "00".repeat(32),
            source_samples: 16_000,
            publications: Vec::new(),
            export_basename: "source".into(),
            revalidation_notice: None,
            resume_from: None,
            reserved_export_basename: None,
        };
        let live = LiveRequest {
            job_id,
            generation,
            model_path: "model.nmodel".into(),
            model_sha256: [1; 32],
            destination: "archive".into(),
            group_offset_samples: 0,
            auto_stop_after_speech_samples: None,
            microphone: crate::microphone::MicrophoneChoice::WindowsDefault,
            microphone_gain_db: 0,
            config: request.config.clone(),
        };
        let commands = vec![
            AppCommand::StartImport {
                request: request.clone(),
            },
            AppCommand::EnqueueImport {
                request: request.clone(),
            },
            AppCommand::ProcessQueued {
                request: request.clone(),
            },
            AppCommand::ResumeInterrupted {
                request: ImportRequest {
                    generation: generation.next().unwrap(),
                    ..request.clone()
                },
            },
            AppCommand::StartLive {
                job_id,
                generation,
                config: request.config.clone(),
            },
            AppCommand::StartLiveConfigured {
                request: live.clone(),
            },
            AppCommand::StartUnifiedLiveConfigured {
                request: live.clone(),
            },
            AppCommand::StartDirectLiveConfigured { request: live },
            AppCommand::StartMicrophoneTest {
                job_id,
                generation,
                microphone: crate::microphone::MicrophoneChoice::WindowsDefault,
                gain_db: 0,
            },
            AppCommand::ResolveDuplicateImport {
                prompt,
                choice: DuplicateImportChoice::CreateNew,
            },
        ];
        for command in commands {
            assert_eq!(app.dispatch(command), Err(ApplicationError::InvalidCommand));
        }
        assert!(app.io.effects.is_empty());
        assert!(app.view.close_ready);
    }

    #[test]
    fn close_removes_durable_queued_work_before_reporting_ready() {
        let mut app = ImportApplication::new(ScanTestIo::default());
        let request = ImportRequest {
            strategy: crate::parakeet_session::TranscriptionStrategy::BoundedWindowsV1,
            job_id: JobId(992),
            generation: Generation::first(),
            source_path: "source.wav".into(),
            source_sha256: None,
            source_samples: None,
            model_path: "model.nmodel".into(),
            model_sha256: [2; 32],
            destination: "archive".into(),
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Cpu,
            },
        };
        app.view.queue.push(ImportQueueEntry {
            sequence: 1,
            request: request.clone(),
            status: ImportQueueStatus::Queued,
        });
        app.view.queued_jobs = 1;

        app.dispatch(AppCommand::RequestClose).unwrap();

        assert!(!app.view.close_ready);
        assert!(matches!(
            app.io.effects.as_slice(),
            [ImportEffect::RemoveQueued { job_id, generation, destination }]
                if *job_id == request.job_id
                    && *generation == request.generation
                    && destination == "archive"
        ));
        app.io.events.push_back(ImportEvent::Queue(Vec::new()));
        app.dispatch(AppCommand::Refresh).unwrap();
        assert!(app.view.close_ready);
    }

    #[test]
    fn close_keeps_failed_queue_removal_recoverable_and_retries_it() {
        let mut app = ImportApplication::new(ScanTestIo::default());
        let request = ImportRequest {
            strategy: crate::parakeet_session::TranscriptionStrategy::BoundedWindowsV1,
            job_id: JobId(996),
            generation: Generation::first(),
            source_path: "source.wav".into(),
            source_sha256: None,
            source_samples: None,
            model_path: "model.nmodel".into(),
            model_sha256: [8; 32],
            destination: "archive".into(),
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Cpu,
            },
        };
        app.view.queue.push(ImportQueueEntry {
            sequence: 1,
            request: request.clone(),
            status: ImportQueueStatus::Queued,
        });
        app.view.queued_jobs = 1;

        app.dispatch(AppCommand::RequestClose).unwrap();
        app.io.events.push_back(ImportEvent::Failed {
            job_id: request.job_id,
            generation: request.generation,
            message: "source binding still in use".into(),
        });
        let view = app.dispatch(AppCommand::Refresh).unwrap();

        assert!(!view.close_ready);
        assert_eq!(
            view.queue.len(),
            1,
            "failed removal keeps queued work visible"
        );
        assert!(app.close_queue_removals.is_empty());
        assert!(view.message.as_deref().is_some_and(|message| {
            message.contains("source binding still in use")
                && message.contains("La file est conservée")
        }));

        app.dispatch(AppCommand::RequestClose).unwrap();
        assert!(matches!(
            app.io.effects.last(),
            Some(ImportEffect::RemoveQueued { job_id, generation, .. })
                if *job_id == request.job_id && *generation == request.generation
        ));
        app.io.events.push_back(ImportEvent::Queue(Vec::new()));
        assert!(app.dispatch(AppCommand::Refresh).unwrap().close_ready);
    }

    #[test]
    fn request_close_waits_for_an_already_committing_import_result() {
        let mut io = ScanTestIo::default();
        io.stop_committing = true;
        let mut app = ImportApplication::new(io);
        let job_id = JobId(995);
        let generation = Generation::first();
        let request = ImportRequest {
            strategy: crate::parakeet_session::TranscriptionStrategy::LegacyV1,
            job_id,
            generation,
            source_path: "source.wav".into(),
            source_sha256: Some([3; 32]),
            source_samples: Some(16_000),
            model_path: "model.bin".into(),
            model_sha256: [4; 32],
            destination: "archive".into(),
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Cpu,
            },
        };
        app.dispatch(AppCommand::StartImport { request }).unwrap();

        app.dispatch(AppCommand::RequestClose).unwrap();
        assert!(!app.view.close_ready);
        assert_eq!(app.active, Some(CommandIdentity { job_id, generation }));
        assert_eq!(app.view.active_job, Some((job_id, JobState::Finalizing)));

        app.io
            .events
            .push_back(ImportEvent::Published { job_id, generation });
        app.dispatch(AppCommand::Refresh).unwrap();
        assert!(app.view.close_ready);
        assert_eq!(app.view.active_job, Some((job_id, JobState::Complete)));
    }

    #[test]
    fn unified_page_responses_and_errors_follow_latest_selection_request() {
        let mut app = ImportApplication::new(ScanTestIo::default());
        let a = (JobId(993), Generation::first());
        let b = (JobId(994), Generation::first());
        app.add_unified_session_card(a.0, a.1, "archive".into(), "a.wav".into(), false);
        app.add_unified_session_card(b.0, b.1, "archive".into(), "b.wav".into(), false);

        app.dispatch(AppCommand::SelectUnifiedSession {
            job_id: a.0,
            generation: a.1,
        })
        .unwrap();
        app.dispatch(AppCommand::ReadUnifiedDocumentPage {
            job_id: a.0,
            generation: a.1,
            expected_revision: 0,
            first_sequence: 1,
        })
        .unwrap();
        let a_doc_old = match app.io.effects.last().unwrap() {
            ImportEffect::ReadUnifiedDocumentPage {
                request_id, limit, ..
            } => {
                assert_eq!(*limit, 128);
                *request_id
            }
            other => panic!("unexpected effect: {other:?}"),
        };
        app.dispatch(AppCommand::ReadUnifiedAudioPartsPage {
            job_id: a.0,
            generation: a.1,
            expected_revision: 0,
            offset: 0,
        })
        .unwrap();
        let a_audio_old = match app.io.effects.last().unwrap() {
            ImportEffect::ReadUnifiedAudioPartsPage {
                request_id, limit, ..
            } => {
                assert_eq!(*limit, 128);
                *request_id
            }
            other => panic!("unexpected effect: {other:?}"),
        };

        app.dispatch(AppCommand::SelectUnifiedSession {
            job_id: b.0,
            generation: b.1,
        })
        .unwrap();
        app.dispatch(AppCommand::ReadUnifiedDocumentPage {
            job_id: b.0,
            generation: b.1,
            expected_revision: 0,
            first_sequence: 1,
        })
        .unwrap();
        let b_doc = match app.io.effects.last().unwrap() {
            ImportEffect::ReadUnifiedDocumentPage { request_id, .. } => *request_id,
            other => panic!("unexpected effect: {other:?}"),
        };
        app.dispatch(AppCommand::ReadUnifiedAudioPartsPage {
            job_id: b.0,
            generation: b.1,
            expected_revision: 0,
            offset: 0,
        })
        .unwrap();
        let b_audio = match app.io.effects.last().unwrap() {
            ImportEffect::ReadUnifiedAudioPartsPage { request_id, .. } => *request_id,
            other => panic!("unexpected effect: {other:?}"),
        };

        app.dispatch(AppCommand::SelectUnifiedSession {
            job_id: a.0,
            generation: a.1,
        })
        .unwrap();
        app.dispatch(AppCommand::ReadUnifiedDocumentPage {
            job_id: a.0,
            generation: a.1,
            expected_revision: 0,
            first_sequence: 1,
        })
        .unwrap();
        let a_doc_new = match app.io.effects.last().unwrap() {
            ImportEffect::ReadUnifiedDocumentPage { request_id, .. } => *request_id,
            other => panic!("unexpected effect: {other:?}"),
        };
        app.dispatch(AppCommand::ReadUnifiedAudioPartsPage {
            job_id: a.0,
            generation: a.1,
            expected_revision: 0,
            offset: 0,
        })
        .unwrap();
        let a_audio_new = match app.io.effects.last().unwrap() {
            ImportEffect::ReadUnifiedAudioPartsPage { request_id, .. } => *request_id,
            other => panic!("unexpected effect: {other:?}"),
        };

        let page = |text: &str| crate::ports::ParakeetDocumentPage {
            revision: 0,
            entries: vec![crate::ports::ParakeetDocumentEntry {
                segment: crate::ports::WorkerSegment {
                    job_id: a.0,
                    generation: a.1,
                    instance_id: 1,
                    segment_id: crate::domain::SegmentId(1),
                    range: crate::domain::SourceRange::new(0, 16_000, 16_000).unwrap(),
                    text: text.into(),
                },
                revision: 0,
            }],
            next_sequence: None,
        };
        let audio_page = |path: &str| crate::ports::UnifiedAudioPartPage {
            revision: 7,
            offset: 0,
            entries: vec![crate::ports::UnifiedAudioPart {
                number: 1,
                start_sample: 0,
                end_sample: 16_000,
                path: path.into(),
            }],
            next_offset: None,
        };
        app.io.events.extend([
            ImportEvent::UnifiedDocumentPage {
                job_id: a.0,
                generation: a.1,
                request_id: a_doc_old,
                result: Err("stale A document error".into()),
            },
            ImportEvent::UnifiedDocumentPage {
                job_id: a.0,
                generation: a.1,
                request_id: a_doc_old,
                result: Ok(page("stale A document success")),
            },
            ImportEvent::UnifiedAudioPartsPage {
                job_id: a.0,
                generation: a.1,
                request_id: a_audio_old,
                result: Err("stale A audio error".into()),
            },
            ImportEvent::UnifiedAudioPartsPage {
                job_id: a.0,
                generation: a.1,
                request_id: a_audio_old,
                result: Ok(audio_page("stale-a.wav")),
            },
            ImportEvent::UnifiedDocumentPage {
                job_id: b.0,
                generation: b.1,
                request_id: b_doc,
                result: Ok(page("stale B")),
            },
            ImportEvent::UnifiedDocumentPage {
                job_id: b.0,
                generation: b.1,
                request_id: b_doc,
                result: Err("stale B document error".into()),
            },
            ImportEvent::UnifiedAudioPartsPage {
                job_id: b.0,
                generation: b.1,
                request_id: b_audio,
                result: Ok(audio_page("stale-b.wav")),
            },
            ImportEvent::UnifiedAudioPartsPage {
                job_id: b.0,
                generation: b.1,
                request_id: b_audio,
                result: Err("stale B audio error".into()),
            },
            ImportEvent::UnifiedDocumentPage {
                job_id: a.0,
                generation: a.1,
                request_id: a_doc_new,
                result: Ok(page("current A")),
            },
            ImportEvent::UnifiedAudioPartsPage {
                job_id: a.0,
                generation: a.1,
                request_id: a_audio_new,
                result: Ok(audio_page("current-a.wav")),
            },
        ]);
        let max_refreshes = app.io.events.len().div_ceil(MAX_EVENTS_PER_REFRESH);
        for _ in 0..max_refreshes {
            app.dispatch(AppCommand::Refresh).unwrap();
            if app.io.events.is_empty() {
                break;
            }
        }
        assert!(
            app.io.events.is_empty(),
            "queued fixture events were not drained"
        );

        let selected = app.view.selected_unified_session.as_ref().unwrap();
        assert_eq!(
            selected.document_page.as_ref().unwrap().entries[0]
                .segment
                .text,
            "current A"
        );
        assert_eq!(
            selected.audio_parts_page.as_ref().unwrap().entries[0].path,
            "current-a.wav"
        );
        assert_eq!(selected.document_error, None);
        assert_eq!(selected.audio_parts_error, None);
    }

    #[test]
    fn unified_session_card_cache_stays_bounded_at_eight() {
        let mut app = ImportApplication::new(ScanTestIo::default());
        for raw_id in 1_000..=1_009 {
            app.add_unified_session_card(
                JobId(raw_id),
                Generation::first(),
                "archive".into(),
                format!("source-{raw_id}.wav"),
                false,
            );
        }
        assert_eq!(
            app.view.unified_parakeet_sessions.len(),
            MAX_PARAKEET_CAPTURE_VIEWS
        );
        assert_eq!(
            app.view
                .unified_parakeet_sessions
                .last()
                .map(|session| session.job_id),
            Some(JobId(1_007)),
            "the ninth active session cannot grow the bounded metadata list"
        );
        assert!(app.view.selected_unified_session.is_some());
    }

    #[test]
    fn microphone_gain_command_is_scoped_to_the_active_direct_capture() {
        let mut app = ImportApplication::new(ScanTestIo::default());
        let job_id = JobId(901);
        let generation = Generation::first();
        app.dispatch(AppCommand::StartDirectLiveConfigured {
            request: LiveRequest {
                job_id,
                generation,
                model_path: "model.nmodel".into(),
                model_sha256: [1; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: crate::microphone::MicrophoneChoice::Device("mic:A".into()),
                microphone_gain_db: 0,
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            },
        })
        .unwrap();

        app.dispatch(AppCommand::SetMicrophoneGain {
            job_id,
            generation,
            target_identity: "mic:A".into(),
            expected_revision: 0,
            gain: crate::microphone_level::GainDb::new(6).unwrap(),
        })
        .unwrap();
        assert!(matches!(
            app.io.effects.last(),
            Some(ImportEffect::SetMicrophoneGain {
                job_id: actual_job,
                generation: actual_generation,
                target_identity,
                expected_revision: 0,
                gain_db,
            }) if *actual_job == job_id
                && *actual_generation == generation
                && target_identity == "mic:A"
                && *gain_db == 6
        ));
        assert_eq!(app.view.microphone_gain_requested_db, 6);
        assert_eq!(app.view.microphone_gain_requested_revision, 1);
        assert_eq!(
            app.view.microphone_gain_db, 0,
            "requested is not yet applied"
        );
        app.dispatch(AppCommand::SetMicrophoneGain {
            job_id,
            generation,
            target_identity: "mic:A".into(),
            expected_revision: 1,
            gain: crate::microphone_level::GainDb::new(9).unwrap(),
        })
        .unwrap();
        assert_eq!(app.view.microphone_gain_requested_db, 9);
        assert_eq!(app.view.microphone_gain_requested_revision, 2);
        assert_eq!(app.view.microphone_gain_revision, 0);
        assert_eq!(
            app.dispatch(AppCommand::SetMicrophoneGain {
                job_id,
                generation,
                target_identity: "mic:A".into(),
                expected_revision: 0,
                gain: crate::microphone_level::GainDb::new(-3).unwrap(),
            }),
            Err(ApplicationError::InvalidCommand)
        );
        app.io
            .events
            .push_back(ImportEvent::MicrophoneGainRejected {
                job_id,
                generation,
                target_identity: "mic:A".into(),
                message: "révision périmée".into(),
            });
        app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(app.active, Some(CommandIdentity { job_id, generation }));
        assert!(
            app.direct_live,
            "gain refusal must not terminate the capture"
        );
        app.io.events.push_back(ImportEvent::MicrophoneGainApplied {
            job_id,
            generation,
            target_identity: "mic:A".into(),
            gain_db: 9,
            gain_revision: 2,
            first_frame: 16_000,
        });
        app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(app.view.microphone_gain_db, 9);
        assert_eq!(app.view.microphone_gain_revision, 2);
        assert_eq!(app.view.microphone_gain_applied_first_frame, Some(16_000));
        assert_eq!(app.view.microphone_gain_requested_db, 9);
        assert_eq!(
            app.dispatch(AppCommand::SetMicrophoneGain {
                job_id,
                generation: generation.next().unwrap(),
                target_identity: "mic:A".into(),
                expected_revision: 2,
                gain: crate::microphone_level::GainDb::NEUTRAL,
            }),
            Err(ApplicationError::InvalidCommand)
        );
        app.dispatch(AppCommand::Stop { job_id, generation })
            .unwrap();
        assert_eq!(
            app.dispatch(AppCommand::SetMicrophoneGain {
                job_id,
                generation,
                target_identity: "mic:A".into(),
                expected_revision: 2,
                gain: crate::microphone_level::GainDb::NEUTRAL,
            }),
            Err(ApplicationError::InvalidCommand)
        );
    }

    #[test]
    fn microphone_test_gain_requested_during_open_survives_started_event() {
        let mut app = ImportApplication::new(ScanTestIo::default());
        let job_id = JobId(904);
        let generation = Generation::first();

        app.dispatch(AppCommand::StartMicrophoneTest {
            job_id,
            generation,
            microphone: crate::microphone::MicrophoneChoice::Device("mic:A".into()),
            gain_db: 0,
        })
        .unwrap();
        app.dispatch(AppCommand::SetMicrophoneGain {
            job_id,
            generation,
            target_identity: "mic:A".into(),
            expected_revision: 0,
            gain: crate::microphone_level::GainDb::new(6).unwrap(),
        })
        .unwrap();

        assert!(matches!(
            app.io.effects.last(),
            Some(ImportEffect::SetMicrophoneGain {
                expected_revision: 0,
                gain_db: 6,
                ..
            })
        ));
        app.io.events.push_back(ImportEvent::MicrophoneTestStarted {
            job_id,
            generation,
            source_identity: "mic:A".into(),
            gain_db: 0,
        });
        app.dispatch(AppCommand::Refresh).unwrap();

        assert_eq!(
            app.view.microphone_source_identity.as_deref(),
            Some("mic:A")
        );
        assert_eq!(app.view.microphone_gain_requested_db, 6);
        assert_eq!(app.view.microphone_gain_requested_revision, 1);
        assert_eq!(app.view.microphone_gain_db, 0);
        assert_eq!(app.view.microphone_gain_revision, 0);

        app.dispatch(AppCommand::SetMicrophoneGain {
            job_id,
            generation,
            target_identity: "mic:A".into(),
            expected_revision: 1,
            gain: crate::microphone_level::GainDb::new(9).unwrap(),
        })
        .unwrap();
        assert!(matches!(
            app.io.effects.last(),
            Some(ImportEffect::SetMicrophoneGain {
                expected_revision: 1,
                gain_db: 9,
                ..
            })
        ));
        assert_eq!(app.view.microphone_gain_requested_db, 9);
        assert_eq!(app.view.microphone_gain_requested_revision, 2);

        app.io.events.push_back(ImportEvent::MicrophoneGainApplied {
            job_id,
            generation,
            target_identity: "mic:A".into(),
            gain_db: 6,
            gain_revision: 1,
            first_frame: 960,
        });
        app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(app.view.microphone_gain_db, 6);
        assert_eq!(app.view.microphone_gain_revision, 1);
        assert_eq!(app.view.microphone_gain_applied_first_frame, Some(960));
        assert_eq!(app.view.microphone_gain_requested_db, 9);
        assert_eq!(app.view.microphone_gain_requested_revision, 2);
    }

    #[test]
    fn microphone_test_owns_one_capture_then_releases_before_live_restart() {
        let mut app = ImportApplication::new(ScanTestIo::default());
        let job_id = JobId(902);
        let generation = Generation::first();
        app.dispatch(AppCommand::StartMicrophoneTest {
            job_id,
            generation,
            microphone: crate::microphone::MicrophoneChoice::Device("mic:A".into()),
            gain_db: 6,
        })
        .unwrap();
        assert!(matches!(
            app.io.effects.last(),
            Some(ImportEffect::StartMicrophoneTest { job_id: actual, gain_db: 6, .. })
                if *actual == job_id
        ));
        let effect_count = app.io.effects.len();
        assert_eq!(
            app.dispatch(AppCommand::StartDirectLiveConfigured {
                request: LiveRequest {
                    job_id,
                    generation: generation.next().unwrap(),
                    model_path: "model.nmodel".into(),
                    model_sha256: [1; 32],
                    destination: "archive".into(),
                    group_offset_samples: 0,
                    auto_stop_after_speech_samples: None,
                    microphone: crate::microphone::MicrophoneChoice::Device("mic:A".into()),
                    microphone_gain_db: 6,
                    config: JobConfig {
                        language: LanguageChoice::Automatic,
                        compute: ComputeChoice::Cpu,
                    },
                },
            }),
            Err(ApplicationError::InvalidCommand)
        );
        assert_eq!(
            app.io.effects.len(),
            effect_count,
            "test owns the only capture"
        );

        app.io.events.push_back(ImportEvent::MicrophoneTestStarted {
            job_id,
            generation,
            source_identity: "mic:A".into(),
            gain_db: 6,
        });
        app.dispatch(AppCommand::Refresh).unwrap();
        assert!(app.view.microphone_test_active);
        assert_eq!(
            app.view.microphone_source_identity.as_deref(),
            Some("mic:A")
        );
        assert_eq!(app.view.microphone_gain_db, 6);

        app.dispatch(AppCommand::StopMicrophoneTest { job_id, generation })
            .unwrap();
        assert!(app.view.microphone_meter.is_none());
        assert!(matches!(
            app.io.effects.last(),
            Some(ImportEffect::StopMicrophoneTest { job_id: actual, .. }) if *actual == job_id
        ));
        app.io
            .events
            .push_back(ImportEvent::MicrophoneTestStopped { job_id, generation });
        app.dispatch(AppCommand::Refresh).unwrap();
        assert!(!app.view.microphone_test_active);
        assert!(app.view.microphone_source_identity.is_none());

        let next_generation = generation.next().unwrap();
        app.dispatch(AppCommand::StartLiveConfigured {
            request: LiveRequest {
                job_id,
                generation: next_generation,
                model_path: "model.nmodel".into(),
                model_sha256: [1; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: crate::microphone::MicrophoneChoice::Device("mic:A".into()),
                microphone_gain_db: 6,
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            },
        })
        .unwrap();
        assert_eq!(app.view.direct_capture_phase, None);
        assert_eq!(
            app.view
                .microphone_acquisition
                .as_ref()
                .map(|context| context.expected_source_identity.as_deref()),
            Some(Some("mic:A"))
        );
        let mut spectrum = crate::microphone_level::MicrophoneSpectrum {
            job_id,
            generation: next_generation,
            source_identity: "mic:A".into(),
            sequence: 1,
            epoch: 0,
            first_frame: 0,
            last_frame: 2048,
            sample_rate_hz: 48_000,
            captured_at_monotonic_ms: crate::microphone_level::monotonic_millis(),
            gain_first_db: 6,
            gain_first_revision: 0,
            gain_last_db: 6,
            gain_last_revision: 0,
            mixed_gain: false,
            dropped_updates: 0,
            pre_gain: vec![crate::microphone_level::SpectrumBin::Silence; 1025],
            post_gain_limiter: vec![crate::microphone_level::SpectrumBin::Silence; 1025],
        };
        spectrum.source_identity = "mic:wrong".into();
        app.io.events.push_back(ImportEvent::MicrophoneSpectrum(
            crate::microphone_level::MicrophoneSpectrumUpdate::Available(spectrum.clone()),
        ));
        app.dispatch(AppCommand::Refresh).unwrap();
        assert!(
            app.view.microphone_spectrum.is_none(),
            "wrong explicit source cannot bind the acquisition"
        );
        spectrum.source_identity = "mic:A".into();
        spectrum.captured_at_monotonic_ms = crate::microphone_level::monotonic_millis();
        app.io.events.push_back(ImportEvent::MicrophoneSpectrum(
            crate::microphone_level::MicrophoneSpectrumUpdate::Available(spectrum.clone()),
        ));
        app.dispatch(AppCommand::Refresh).unwrap();
        assert!(
            app.view.microphone_spectrum.is_some(),
            "the actual source can bind even while direct capture phase is None"
        );
        let retry_barrier = crate::microphone_level::monotonic_millis();
        app.io.events.push_back(ImportEvent::LiveRetryingOnCpu {
            job_id,
            generation: next_generation,
            spectrum_min_epoch: 1,
            spectrum_not_before_monotonic_ms: retry_barrier,
        });
        app.dispatch(AppCommand::Refresh).unwrap();
        assert!(app.view.microphone_spectrum.is_none());
        let mut old_epoch_spectrum = spectrum.clone();
        old_epoch_spectrum.captured_at_monotonic_ms = retry_barrier.saturating_sub(1);
        app.io.events.push_back(ImportEvent::MicrophoneSpectrum(
            crate::microphone_level::MicrophoneSpectrumUpdate::Available(old_epoch_spectrum),
        ));
        app.dispatch(AppCommand::Refresh).unwrap();
        assert!(
            app.view.microphone_spectrum.is_none(),
            "queued pre-retry spectrum for the same job and generation is rejected"
        );
        app.io.stop_committing = true;
        app.dispatch(AppCommand::Stop {
            job_id,
            generation: next_generation,
        })
        .unwrap();
        assert_eq!(app.view.active_job, Some((job_id, JobState::Finalizing)));
        assert!(app.view.microphone_acquisition.is_none());
        assert!(
            app.view.microphone_spectrum.is_none(),
            "Stop clears before asynchronous events are drained"
        );
        app.io.events.push_back(ImportEvent::MicrophoneSpectrum(
            crate::microphone_level::MicrophoneSpectrumUpdate::Available(spectrum),
        ));
        app.dispatch(AppCommand::Refresh).unwrap();
        assert!(
            app.view.microphone_spectrum.is_none(),
            "late spectrum after Stop is ignored"
        );
        app.io.events.push_back(ImportEvent::MicrophoneMeter(
            crate::microphone_level::MicrophoneMeter {
                job_id,
                generation,
                source_identity: "mic:A".into(),
                sequence: 1,
                first_frame: 0,
                last_frame: 4_800,
                sample_rate_hz: 48_000,
                analyzed_channels: 1,
                gain_first_db: 6,
                gain_first_revision: 0,
                gain_last_db: 6,
                gain_last_revision: 0,
                mixed_gain: false,
                pre_gain: crate::microphone_level::MeterLevel {
                    status: crate::microphone_level::MeasurementStatus::Signal,
                    sample_count: 4_800,
                    rms_micros: 100_000,
                    peak_micros: 200_000,
                },
                post_gain_limiter: crate::microphone_level::MeterLevel {
                    status: crate::microphone_level::MeasurementStatus::Signal,
                    sample_count: 4_800,
                    rms_micros: 200_000,
                    peak_micros: 400_000,
                },
                limiter_samples: 0,
                dropped_updates: 0,
                captured_at_monotonic_ms: 10,
            },
        ));
        app.dispatch(AppCommand::Refresh).unwrap();
        assert!(
            app.view.microphone_meter.is_none(),
            "old generation is ignored"
        );
    }

    #[test]
    fn microphone_test_failure_invalidates_its_meter_and_source() {
        let mut app = ImportApplication::new(ScanTestIo::default());
        let job_id = JobId(903);
        let generation = Generation::first();
        app.dispatch(AppCommand::StartMicrophoneTest {
            job_id,
            generation,
            microphone: crate::microphone::MicrophoneChoice::Device("mic:A".into()),
            gain_db: 0,
        })
        .unwrap();
        app.io.events.push_back(ImportEvent::MicrophoneTestStarted {
            job_id,
            generation,
            source_identity: "mic:A".into(),
            gain_db: 0,
        });
        app.dispatch(AppCommand::Refresh).unwrap();
        app.io.events.push_back(ImportEvent::Failed {
            job_id,
            generation,
            message: "flux microphone interrompu".into(),
        });
        app.dispatch(AppCommand::Refresh).unwrap();
        assert!(!app.view.microphone_test_active);
        assert!(app.view.microphone_meter.is_none());
        assert!(app.view.microphone_source_identity.is_none());
        assert!(app.view.message.as_deref().unwrap().contains("arrêté"));
    }

    #[test]
    fn destination_scan_stays_pending_until_history_and_queue_are_consumed() {
        let mut app = ImportApplication::new(ScanTestIo::default());
        let view = app
            .dispatch(AppCommand::ScanHistory {
                destination: "archive".into(),
            })
            .unwrap();
        assert!(view.history_scan_pending);
        assert!(view.queue_scan_pending);

        app.io.events.push_back(ImportEvent::History(Vec::new()));
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert!(!view.history_scan_pending);
        assert!(view.queue_scan_pending);

        app.io.events.push_back(ImportEvent::Queue(Vec::new()));
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert!(!view.history_scan_pending);
        assert!(!view.queue_scan_pending);
    }

    #[test]
    fn queue_scan_failure_clears_pending_state_and_is_visible() {
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.dispatch(AppCommand::ScanHistory {
            destination: "missing-or-invalid".into(),
        })
        .unwrap();
        app.io.events.push_back(ImportEvent::History(Vec::new()));
        app.dispatch(AppCommand::Refresh).unwrap();
        app.io.events.push_back(ImportEvent::Failed {
            job_id: JobId(0),
            generation: Generation::first(),
            message: "StorageCorrupt: destination is not a directory".into(),
        });

        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert!(!view.queue_scan_pending);
        assert!(
            view.message
                .as_deref()
                .is_some_and(|message| message.contains("destination is not a directory"))
        );
    }

    #[test]
    fn cancelling_duplicate_import_releases_only_the_new_source_snapshot() {
        let mut app = ImportApplication::new(ScanTestIo::default());
        let job_id = JobId(101);
        let generation = Generation::first();
        let destination = "archive".to_owned();
        app.view.duplicate_import_prompt = Some(DuplicateImportPrompt {
            request: ImportRequest {
                strategy: crate::parakeet_session::TranscriptionStrategy::LegacyV1,
                job_id,
                generation,
                source_path: "source.wav".into(),
                source_sha256: Some([1; 32]),
                source_samples: Some(16_000),
                model_path: "model.bin".into(),
                model_sha256: [2; 32],
                destination: destination.clone(),
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            },
            source_sha256: "01".repeat(32),
            source_samples: 16_000,
            publications: Vec::new(),
            export_basename: String::new(),
            revalidation_notice: None,
            resume_from: None,
            reserved_export_basename: None,
        });

        let view = app.dispatch(AppCommand::CancelDuplicateImport).unwrap();

        assert!(view.duplicate_import_prompt.is_none());
        assert_eq!(
            view.message.as_deref(),
            Some("Nouvelle transcription annulée.")
        );
        assert_eq!(
            app.io.effects,
            vec![ImportEffect::ReleaseSourceBinding {
                job_id,
                generation,
                destination,
            }]
        );
    }

    #[test]
    fn stale_duplicate_dialog_response_cannot_submit_a_replacement() {
        let mut app = ImportApplication::new(ScanTestIo::default());
        let current = DuplicateImportPrompt {
            request: ImportRequest {
                strategy: crate::parakeet_session::TranscriptionStrategy::LegacyV1,
                job_id: JobId(201),
                generation: Generation::first(),
                source_path: "source.wav".into(),
                source_sha256: Some([1; 32]),
                source_samples: Some(16_000),
                model_path: "model.bin".into(),
                model_sha256: [2; 32],
                destination: "archive".into(),
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            },
            source_sha256: "01".repeat(32),
            source_samples: 16_000,
            publications: Vec::new(),
            export_basename: String::new(),
            revalidation_notice: None,
            resume_from: None,
            reserved_export_basename: None,
        };
        let mut stale = current.clone();
        stale.request.job_id = JobId(202);
        app.view.duplicate_import_prompt = Some(current);

        let result = app.dispatch(AppCommand::ResolveDuplicateImport {
            prompt: stale,
            choice: DuplicateImportChoice::ReplaceAll,
        });

        assert!(matches!(result, Err(ApplicationError::InvalidCommand)));
        assert_eq!(app.io.effects, Vec::new());
        assert_eq!(
            app.view
                .duplicate_import_prompt
                .as_ref()
                .unwrap()
                .request
                .job_id,
            JobId(201)
        );
    }

    #[test]
    fn model_guard_blocks_each_import_and_live_effect_route_before_submission() {
        let generation = Generation::first();
        let config = JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        };
        let import = ImportRequest {
            strategy: crate::parakeet_session::TranscriptionStrategy::LegacyV1,
            job_id: JobId(70),
            generation,
            source_path: "source.wav".into(),
            source_sha256: None,
            source_samples: None,
            model_path: "C:/unknown/model.bin".into(),
            model_sha256: [0; 32],
            destination: "archive".into(),
            config: config.clone(),
        };
        let live = LiveRequest {
            job_id: JobId(70),
            generation,
            model_path: import.model_path.clone(),
            model_sha256: [0; 32],
            destination: "archive".into(),
            group_offset_samples: 0,
            auto_stop_after_speech_samples: None,
            microphone: crate::microphone::MicrophoneChoice::WindowsDefault,
            microphone_gain_db: 0,
            config,
        };
        let snapshot = crate::model_registry::engine_snapshot(
            1,
            |_| crate::model_catalog::ModelAvailability::Ready,
            Vec::new(),
        );
        let commands = [
            AppCommand::StartImport {
                request: import.clone(),
            },
            AppCommand::EnqueueImport {
                request: import.clone(),
            },
            AppCommand::ProcessQueued {
                request: import.clone(),
            },
            AppCommand::ResumeInterrupted { request: import },
            AppCommand::StartLiveConfigured {
                request: live.clone(),
            },
            AppCommand::StartDirectLiveConfigured { request: live },
        ];

        for command in commands {
            let mut app = ImportApplication::new(ScanTestIo::default()).with_model_guard(
                crate::model_selection_app::ModelCommandGuard::new(
                    crate::model_registry::engine_snapshot(
                        1,
                        |_| crate::model_catalog::ModelAvailability::Ready,
                        Vec::new(),
                    ),
                    1,
                    ["C:/known/turbo.bin".to_owned()],
                ),
            );
            let view = app.dispatch(command).unwrap();
            assert!(app.io.effects.is_empty());
            assert!(view.message.as_deref().is_some_and(|message| {
                message.contains("Modèle indisponible") && message.contains("non reconnu")
            }));
        }
        drop(snapshot);
    }

    #[test]
    fn turbo_existing_matrix_dispatches_all_1206_requests_to_effect_submission() {
        use crate::{
            model_catalog::{ModelAvailability, ModelId},
            model_registry::{TURBO_LANGUAGE_INPUTS, engine_snapshot, model},
            model_selection_app::{InferencePath, ModelCommandGuard},
        };

        let model_path = r"C:\Users\Cyril\AppData\Local\Programs\WhisperBuildDeps\models\ggml-large-v3-turbo.bin";
        let mut sha256 = [0; 32];
        let digest = model(ModelId::WhisperLargeV3Turbo).artifact.sha256;
        for (index, byte) in sha256.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&digest[index * 2..index * 2 + 2], 16).unwrap();
        }
        let mut submitted = 0usize;

        for compute in [ComputeChoice::Cpu, ComputeChoice::Gpu, ComputeChoice::Auto] {
            for path in [InferencePath::Import, InferencePath::Live] {
                let mut languages = vec![LanguageChoice::Automatic];
                for (code, english_name) in TURBO_LANGUAGE_INPUTS {
                    languages.push(LanguageChoice::Manual(code.into()));
                    languages.push(LanguageChoice::Manual(english_name.into()));
                }
                for language in languages {
                    let generation = Generation::first();
                    let job_id = JobId(submitted as u128);
                    let config = JobConfig { language, compute };
                    let guard = ModelCommandGuard::new(
                        engine_snapshot(1, |_| ModelAvailability::Ready, Vec::new()),
                        1,
                        [model_path.to_owned()],
                    );
                    let mut app =
                        ImportApplication::new(ScanTestIo::default()).with_model_guard(guard);
                    match path {
                        InferencePath::Import => {
                            app.dispatch(AppCommand::StartImport {
                                request: ImportRequest {
                                    strategy:
                                        crate::parakeet_session::TranscriptionStrategy::LegacyV1,
                                    job_id,
                                    generation,
                                    source_path: "source.wav".into(),
                                    source_sha256: None,
                                    source_samples: None,
                                    model_path: model_path.into(),
                                    model_sha256: sha256,
                                    destination: "archive".into(),
                                    config,
                                },
                            })
                            .unwrap();
                            assert!(matches!(
                                app.io.effects.as_slice(),
                                [ImportEffect::Prepare(_)]
                            ));
                        }
                        InferencePath::Live => {
                            app.dispatch(AppCommand::StartLiveConfigured {
                                request: LiveRequest {
                                    job_id,
                                    generation,
                                    model_path: model_path.into(),
                                    model_sha256: sha256,
                                    destination: "archive".into(),
                                    group_offset_samples: 0,
                                    auto_stop_after_speech_samples: None,
                                    microphone: crate::microphone::MicrophoneChoice::WindowsDefault,
                                    microphone_gain_db: 0,
                                    config,
                                },
                            })
                            .unwrap();
                            assert!(matches!(
                                app.io.effects.as_slice(),
                                [ImportEffect::StartLive(_)]
                            ));
                        }
                    }
                    submitted += 1;
                }
            }
        }
        assert_eq!(submitted, 1_206);
    }

    #[test]
    fn repeated_start_is_identifiable_and_stale_stop_does_not_target_new_generation() {
        let job_id = JobId(7);
        let current = Generation::first().next().unwrap();
        let stale = Generation::first();
        let config = JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        };
        let start = AppCommand::StartLive {
            job_id,
            generation: current,
            config,
        };
        let repeated_start = start.clone();
        let stale_stop = AppCommand::Stop {
            job_id,
            generation: stale,
        };

        assert_eq!(start, repeated_start);
        assert!(start.targets(job_id, current));
        assert!(!stale_stop.targets(job_id, current));
    }

    #[test]
    fn every_job_lifecycle_command_carries_identity() {
        let job_id = JobId(9);
        let generation = Generation::first();
        let config = JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        };
        let commands = [
            AppCommand::StartImport {
                request: ImportRequest {
                    strategy: crate::parakeet_session::TranscriptionStrategy::LegacyV1,
                    job_id,
                    generation,
                    source_path: "source.wav".into(),
                    source_sha256: None,
                    source_samples: None,
                    model_path: "model.bin".into(),
                    model_sha256: [0; 32],
                    destination: "archive".into(),
                    config: config.clone(),
                },
            },
            AppCommand::ProcessQueued {
                request: ImportRequest {
                    strategy: crate::parakeet_session::TranscriptionStrategy::LegacyV1,
                    job_id,
                    generation,
                    source_path: "source.wav".into(),
                    source_sha256: None,
                    source_samples: None,
                    model_path: "model.bin".into(),
                    model_sha256: [0; 32],
                    destination: "archive".into(),
                    config: config.clone(),
                },
            },
            AppCommand::EnqueueImport {
                request: ImportRequest {
                    strategy: crate::parakeet_session::TranscriptionStrategy::LegacyV1,
                    job_id,
                    generation,
                    source_path: "source.wav".into(),
                    source_sha256: None,
                    source_samples: None,
                    model_path: "model.bin".into(),
                    model_sha256: [0; 32],
                    destination: "archive".into(),
                    config: config.clone(),
                },
            },
            AppCommand::StartLive {
                job_id,
                generation,
                config,
            },
            AppCommand::Stop { job_id, generation },
            AppCommand::Quit { job_id, generation },
        ];
        assert!(commands.iter().all(|command| command.identity().is_some()));
        assert_eq!(AppCommand::Refresh.identity(), None);
    }

    #[test]
    fn live_stop_keeps_generation_until_segments_are_persisted_and_published() {
        let job_id = JobId(10);
        let generation = Generation::first();
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.dispatch(AppCommand::StartLiveConfigured {
            request: LiveRequest {
                job_id,
                generation,
                model_path: "model.bin".into(),
                model_sha256: [1; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                microphone_gain_db: 0,
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            },
        })
        .unwrap();
        app.io.events.push_back(ImportEvent::LiveCounters {
            job_id,
            generation,
            captured_samples: 320,
            admitted_samples: 320,
            audio_durable_samples: 320,
            confirmed_fragment_samples: 0,
            speech_samples: 320,
            diagnostic: None,
        });
        app.dispatch(AppCommand::Refresh).unwrap();
        app.dispatch(AppCommand::Stop { job_id, generation })
            .unwrap();
        assert_eq!(app.active, Some(CommandIdentity { job_id, generation }));

        app.io.events.push_back(ImportEvent::LiveProvisional {
            segment: crate::ports::WorkerSegment {
                job_id,
                generation,
                instance_id: 1,
                segment_id: crate::domain::SegmentId(1),
                range: crate::domain::SourceRange::new(0, 320, 16_000).unwrap(),
                text: "bonjour".into(),
            },
            captured_at_unix_ms: 1,
            available_at_unix_ms: 2,
        });
        app.dispatch(AppCommand::Refresh).unwrap();
        assert!(matches!(
            app.io.effects.last(),
            Some(ImportEffect::PersistSegment(_))
        ));
        app.io.events.push_back(ImportEvent::Persisted {
            job_id,
            generation,
            sequence: 1,
        });
        app.dispatch(AppCommand::Refresh).unwrap();
        app.io
            .events
            .push_back(ImportEvent::Published { job_id, generation });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.active_job, Some((job_id, JobState::Complete)));
        assert!(
            view.last_result
                .as_deref()
                .is_some_and(|text| text.contains("vérifiée"))
        );
    }

    #[test]
    fn final_stop_counters_allow_last_window_and_match_published_view() {
        let job_id = JobId(20);
        let generation = Generation::first();
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.dispatch(AppCommand::StartLiveConfigured {
            request: LiveRequest {
                job_id,
                generation,
                model_path: "model.bin".into(),
                model_sha256: [1; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                microphone_gain_db: 0,
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            },
        })
        .unwrap();
        app.io.events.push_back(ImportEvent::LiveCounters {
            job_id,
            generation,
            captured_samples: 248_640,
            admitted_samples: 248_640,
            audio_durable_samples: 248_640,
            confirmed_fragment_samples: 160_000,
            speech_samples: 0,
            diagnostic: None,
        });
        app.dispatch(AppCommand::Refresh).unwrap();
        app.dispatch(AppCommand::Stop { job_id, generation })
            .unwrap();
        app.io.events.push_back(ImportEvent::LiveCounters {
            job_id,
            generation,
            captured_samples: 254_240,
            admitted_samples: 254_240,
            audio_durable_samples: 254_240,
            confirmed_fragment_samples: 160_000,
            speech_samples: 0,
            diagnostic: None,
        });
        app.io.events.push_back(ImportEvent::WindowFinished {
            job_id,
            generation,
            window_sequence: 4,
            confirmed_samples: 254_240,
        });
        app.io
            .events
            .push_back(ImportEvent::Published { job_id, generation });
        app.dispatch(AppCommand::Refresh).unwrap();
        app.dispatch(AppCommand::Refresh).unwrap();
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.active_job, Some((job_id, JobState::Complete)));
        assert_eq!(view.captured_samples, 254_240);
        assert_eq!(view.admitted_samples, 254_240);
        assert_eq!(view.audio_durable_samples, 254_240);
        assert_eq!(view.confirmed_samples, 254_240);
    }

    #[test]
    fn live_state_advances_through_capture_drain_and_finalization() {
        let job_id = JobId(19);
        let generation = Generation::first();
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.dispatch(AppCommand::StartLiveConfigured {
            request: LiveRequest {
                job_id,
                generation,
                model_path: "model.bin".into(),
                model_sha256: [9; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                microphone_gain_db: 0,
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            },
        })
        .unwrap();
        assert_eq!(app.view.active_job, Some((job_id, JobState::Preparing)));
        app.io.events.push_back(ImportEvent::Ready {
            job_id,
            generation,
            backend: "CPU".into(),
        });
        assert_eq!(
            app.dispatch(AppCommand::Refresh).unwrap().active_job,
            Some((job_id, JobState::Capturing))
        );
        assert_eq!(
            app.dispatch(AppCommand::Stop { job_id, generation })
                .unwrap()
                .active_job,
            Some((job_id, JobState::Draining))
        );
        app.io
            .events
            .push_back(ImportEvent::LiveFinalizing { job_id, generation });
        assert_eq!(
            app.dispatch(AppCommand::Refresh).unwrap().active_job,
            Some((job_id, JobState::Finalizing))
        );
        app.io
            .events
            .push_back(ImportEvent::Published { job_id, generation });
        assert_eq!(
            app.dispatch(AppCommand::Refresh).unwrap().active_job,
            Some((job_id, JobState::Complete))
        );
    }

    #[test]
    fn immediate_live_stop_does_not_reenter_capturing_on_late_ready() {
        let job_id = JobId(23);
        let generation = Generation::first();
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.dispatch(AppCommand::StartLiveConfigured {
            request: LiveRequest {
                job_id,
                generation,
                model_path: "model.bin".into(),
                model_sha256: [11; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                microphone_gain_db: 0,
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            },
        })
        .unwrap();
        app.dispatch(AppCommand::Stop { job_id, generation })
            .unwrap();
        app.io.events.push_back(ImportEvent::Ready {
            job_id,
            generation,
            backend: "CPU".into(),
        });
        assert_eq!(
            app.dispatch(AppCommand::Refresh).unwrap().active_job,
            Some((job_id, JobState::Draining))
        );
        app.io
            .events
            .push_back(ImportEvent::Stopped { job_id, generation });
        assert_eq!(
            app.dispatch(AppCommand::Refresh).unwrap().active_job,
            Some((job_id, JobState::Cancelled))
        );
    }

    #[test]
    fn live_auto_stop_acts_only_when_a_positive_speech_limit_is_configured() {
        for (limit, should_stop) in [(Some(640), true), (None, false), (Some(0), false)] {
            let job_id = JobId(11);
            let generation = Generation::first();
            let mut app = ImportApplication::new(ScanTestIo::default());
            app.dispatch(AppCommand::StartLiveConfigured {
                request: LiveRequest {
                    job_id,
                    generation,
                    model_path: "model.bin".into(),
                    model_sha256: [2; 32],
                    destination: "archive".into(),
                    group_offset_samples: 0,
                    auto_stop_after_speech_samples: limit,
                    microphone: Default::default(),
                    microphone_gain_db: 0,
                    config: JobConfig {
                        language: LanguageChoice::Automatic,
                        compute: ComputeChoice::Cpu,
                    },
                },
            })
            .unwrap();
            app.io.events.push_back(ImportEvent::LiveCounters {
                job_id,
                generation,
                captured_samples: 960,
                admitted_samples: 960,
                audio_durable_samples: 640,
                confirmed_fragment_samples: 0,
                speech_samples: 640,
                diagnostic: None,
            });
            let view = app.dispatch(AppCommand::Refresh).unwrap();
            assert_eq!(
                app.io
                    .effects
                    .iter()
                    .any(|effect| matches!(effect, ImportEffect::Stop { .. })),
                should_stop,
                "limit {limit:?}"
            );
            assert_eq!(
                view.active_job,
                Some((
                    job_id,
                    if should_stop {
                        JobState::Draining
                    } else {
                        JobState::Preparing
                    }
                ))
            );
        }
    }

    #[test]
    fn live_provisional_text_is_exposed_and_persisted_by_the_application() {
        let job_id = JobId(12);
        let generation = Generation::first();
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.dispatch(AppCommand::StartLiveConfigured {
            request: LiveRequest {
                job_id,
                generation,
                model_path: "model.bin".into(),
                model_sha256: [3; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                microphone_gain_db: 0,
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            },
        })
        .unwrap();
        let segment = crate::ports::WorkerSegment {
            job_id,
            generation,
            instance_id: 1,
            segment_id: crate::domain::SegmentId(1),
            range: crate::domain::SourceRange::new(0, 320, 16_000).unwrap(),
            text: "bonjour provisoire".into(),
        };
        app.io.events.push_back(ImportEvent::LiveProvisional {
            segment,
            captured_at_unix_ms: 1_000,
            available_at_unix_ms: 1_020,
        });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.provisional_text.as_deref(), Some("bonjour provisoire"));
        assert_eq!(view.provisional_sequence, Some(1));
        assert_eq!(view.provisional_captured_at_unix_ms, Some(1_000));
        assert_eq!(view.provisional_available_at_unix_ms, Some(1_020));
        assert!(matches!(
            app.io.effects.last(),
            Some(ImportEffect::PersistSegment(_))
        ));
        app.io.events.push_back(ImportEvent::Persisted {
            job_id,
            generation,
            sequence: 1,
        });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.provisional_text, None);
        assert_eq!(view.confirmed_text.as_deref(), Some("bonjour provisoire"));
    }

    #[test]
    fn import_can_be_enqueued_during_live_without_replacing_the_live_job() {
        let job_id = JobId(13);
        let generation = Generation::first();
        let config = JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        };
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.dispatch(AppCommand::StartLiveConfigured {
            request: LiveRequest {
                job_id,
                generation,
                model_path: "model.bin".into(),
                model_sha256: [4; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                microphone_gain_db: 0,
                config: config.clone(),
            },
        })
        .unwrap();
        let import = ImportRequest {
            strategy: crate::parakeet_session::TranscriptionStrategy::LegacyV1,
            job_id: JobId(14),
            generation,
            source_path: "later.wav".into(),
            source_sha256: None,
            source_samples: None,
            model_path: "model.bin".into(),
            model_sha256: [4; 32],
            destination: "archive".into(),
            config,
        };
        app.dispatch(AppCommand::EnqueueImport { request: import })
            .unwrap();
        assert_eq!(app.active, Some(CommandIdentity { job_id, generation }));
        assert!(matches!(
            app.io.effects.last(),
            Some(ImportEffect::Enqueue(_))
        ));
        assert_eq!(app.view.active_job, Some((job_id, JobState::Preparing)));
    }

    #[test]
    fn queued_import_starts_only_after_live_publication_and_explicit_processing() {
        let live_id = JobId(16);
        let generation = Generation::first();
        let config = JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        };
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.dispatch(AppCommand::StartLiveConfigured {
            request: LiveRequest {
                job_id: live_id,
                generation,
                model_path: "model.bin".into(),
                model_sha256: [6; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                microphone_gain_db: 0,
                config: config.clone(),
            },
        })
        .unwrap();
        let queued = ImportRequest {
            strategy: crate::parakeet_session::TranscriptionStrategy::LegacyV1,
            job_id: JobId(17),
            generation,
            source_path: "later.wav".into(),
            source_sha256: None,
            source_samples: None,
            model_path: "model.bin".into(),
            model_sha256: [6; 32],
            destination: "archive".into(),
            config,
        };
        app.dispatch(AppCommand::EnqueueImport {
            request: queued.clone(),
        })
        .unwrap();
        app.dispatch(AppCommand::Stop {
            job_id: live_id,
            generation,
        })
        .unwrap();
        assert!(
            matches!(app.io.effects.last(), Some(ImportEffect::Stop { job_id, .. }) if *job_id == live_id)
        );
        assert_eq!(
            app.active,
            Some(CommandIdentity {
                job_id: live_id,
                generation
            })
        );

        app.io.events.push_back(ImportEvent::Published {
            job_id: live_id,
            generation,
        });
        app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(app.active, None);
        app.dispatch(AppCommand::ProcessQueued { request: queued })
            .unwrap();
        assert_eq!(
            app.active,
            Some(CommandIdentity {
                job_id: JobId(17),
                generation
            })
        );
        assert!(
            matches!(app.io.effects.last(), Some(ImportEffect::ProcessQueued(request)) if request.job_id == JobId(17))
        );
    }

    #[test]
    fn stopped_import_ignores_late_segments_and_remains_cancelled() {
        let job_id = JobId(15);
        let generation = Generation::first();
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.dispatch(AppCommand::StartImport {
            request: ImportRequest {
                strategy: crate::parakeet_session::TranscriptionStrategy::LegacyV1,
                job_id,
                generation,
                source_path: "source.wav".into(),
                source_sha256: None,
                source_samples: None,
                model_path: "model.bin".into(),
                model_sha256: [5; 32],
                destination: "archive".into(),
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            },
        })
        .unwrap();
        app.dispatch(AppCommand::Stop { job_id, generation })
            .unwrap();
        let effects_before_late_segment = app.io.effects.len();
        app.io
            .events
            .push_back(ImportEvent::Segment(crate::ports::WorkerSegment {
                job_id,
                generation,
                instance_id: 1,
                segment_id: crate::domain::SegmentId(1),
                range: crate::domain::SourceRange::new(0, 320, 16_000).unwrap(),
                text: "late".into(),
            }));
        app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(app.io.effects.len(), effects_before_late_segment);
        app.io
            .events
            .push_back(ImportEvent::Stopped { job_id, generation });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.active_job, Some((job_id, JobState::Cancelled)));
    }

    #[test]
    fn direct_capture_and_engine_progress_independently_through_pre_ready_stop() {
        let job_id = JobId(19);
        let generation = Generation::first();
        let mut app = ImportApplication::new(ScanTestIo::default());
        let request = LiveRequest {
            job_id,
            generation,
            model_path: "model.bin".into(),
            model_sha256: [9; 32],
            destination: "archive".into(),
            group_offset_samples: 0,
            auto_stop_after_speech_samples: None,
            microphone: Default::default(),
            microphone_gain_db: 0,
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Cpu,
            },
        };
        let view = app
            .dispatch(AppCommand::StartDirectLiveConfigured { request })
            .unwrap();
        assert_eq!(
            view.direct_capture_phase,
            Some(DirectCapturePhase::Initializing)
        );
        assert_eq!(view.direct_engine_phase, Some(DirectEnginePhase::Preparing));
        app.io.events.push_back(ImportEvent::DirectCaptureActive {
            job_id,
            generation,
            first_sample_at_unix_ms: 100,
            first_admitted_at_unix_ms: 110,
        });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.direct_capture_phase, Some(DirectCapturePhase::Active));
        assert_eq!(view.direct_engine_phase, Some(DirectEnginePhase::Preparing));
        let view = app
            .dispatch(AppCommand::Stop { job_id, generation })
            .unwrap();
        assert_eq!(view.direct_capture_phase, Some(DirectCapturePhase::Stopped));
        assert_eq!(view.direct_engine_phase, Some(DirectEnginePhase::Preparing));
        app.io.events.push_back(ImportEvent::DirectCaptureActive {
            job_id,
            generation,
            first_sample_at_unix_ms: 100,
            first_admitted_at_unix_ms: 110,
        });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.direct_capture_phase, Some(DirectCapturePhase::Stopped));
        assert_eq!(view.direct_first_sample_at_unix_ms, Some(100));
        assert_eq!(view.direct_first_admitted_at_unix_ms, Some(110));
        app.io.events.push_back(ImportEvent::Ready {
            job_id,
            generation,
            backend: "CPU".into(),
        });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.active_job, Some((job_id, JobState::Draining)));
        assert_eq!(view.direct_engine_phase, Some(DirectEnginePhase::Ready));
        app.io.events.push_back(ImportEvent::DirectPublished {
            job_id,
            generation,
            txt_path: "a.txt".into(),
            srt_path: "a.srt".into(),
            wav_path: "a.wav".into(),
        });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.active_job, Some((job_id, JobState::Complete)));
        assert_eq!(view.direct_engine_phase, Some(DirectEnginePhase::Released));
    }

    #[test]
    fn live_capture_failure_remains_recoverable_with_its_diagnostic() {
        let job_id = JobId(16);
        let generation = Generation::first();
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.dispatch(AppCommand::StartLiveConfigured {
            request: LiveRequest {
                job_id,
                generation,
                model_path: "model.bin".into(),
                model_sha256: [6; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                microphone_gain_db: 0,
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            },
        })
        .unwrap();
        let diagnostic = "CaptureSaturated: Stop retained while worker data channel is full";
        app.io.events.push_back(ImportEvent::Failed {
            job_id,
            generation,
            message: diagnostic.into(),
        });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.active_job, Some((job_id, JobState::Recoverable)));
        assert_eq!(view.message.as_deref(), Some(diagnostic));
    }

    #[test]
    fn failed_live_stop_retains_recoverable_diagnostic_and_confirmed_fragment() {
        let job_id = JobId(20);
        let generation = Generation::first();
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.dispatch(AppCommand::StartLiveConfigured {
            request: LiveRequest {
                job_id,
                generation,
                model_path: "model.bin".into(),
                model_sha256: [10; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                microphone_gain_db: 0,
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            },
        })
        .unwrap();
        app.io.events.push_back(ImportEvent::LiveProvisional {
            segment: crate::ports::WorkerSegment {
                job_id,
                generation,
                instance_id: 1,
                segment_id: crate::domain::SegmentId(1),
                range: crate::domain::SourceRange::new(0, 320, 16_000).unwrap(),
                text: "confirmé".into(),
            },
            captured_at_unix_ms: 1,
            available_at_unix_ms: 2,
        });
        app.dispatch(AppCommand::Refresh).unwrap();
        app.io.events.push_back(ImportEvent::Persisted {
            job_id,
            generation,
            sequence: 1,
        });
        app.dispatch(AppCommand::Refresh).unwrap();
        app.dispatch(AppCommand::Stop { job_id, generation })
            .unwrap();
        let diagnostic = "StorageUnavailable: final audio sync failed";
        app.io.events.push_back(ImportEvent::Failed {
            job_id,
            generation,
            message: diagnostic.into(),
        });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.active_job, Some((job_id, JobState::Recoverable)));
        assert_eq!(view.message.as_deref(), Some(diagnostic));
        assert_eq!(view.confirmed_text.as_deref(), Some("confirmé"));
        assert_eq!(view.confirmed_fragment_samples, 320);
    }

    #[test]
    fn parakeet_provisional_never_becomes_confirmed_without_a_confirmed_event() {
        let job_id = JobId(71);
        let generation = Generation::first();
        let session_id = 9;
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.active = Some(CommandIdentity { job_id, generation });
        app.next_sequence = 1;
        app.io.events.push_back(ImportEvent::ParakeetLiveStarted {
            job_id,
            generation,
            session_id,
            backend: "CPU".into(),
        });
        app.io.events.push_back(ImportEvent::ParakeetHypothesis {
            job_id,
            generation,
            session_id,
            sequence: 1,
            range: crate::domain::SourceRange::new(0, 32_000, 16_000).unwrap(),
            text: "texte provisoire".into(),
            captured_at_unix_ms: 1,
            available_at_unix_ms: 2,
        });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.provisional_text.as_deref(), Some("texte provisoire"));
        assert_eq!(view.confirmed_text, None);
        let block = crate::ports::WorkerSegment {
            job_id,
            generation,
            instance_id: session_id,
            segment_id: crate::domain::SegmentId(1),
            range: crate::domain::SourceRange::new(1_000, 8_000, 16_000).unwrap(),
            text: "texte sûr".into(),
        };
        app.io.events.push_back(ImportEvent::ParakeetConfirmed {
            job_id,
            generation,
            session_id: session_id + 1,
            segment: block.clone(),
        });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.confirmed_text, None);
        app.io.events.push_back(ImportEvent::ParakeetConfirmed {
            job_id,
            generation,
            session_id,
            segment: block.clone(),
        });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.confirmed_text.as_deref(), Some("texte sûr"));
        let stopping = app
            .dispatch(AppCommand::Stop { job_id, generation })
            .unwrap();
        assert_eq!(
            stopping.parakeet_captures[0].capture_state,
            crate::ports::ParakeetCaptureViewState::Finalizing
        );
        app.io
            .events
            .push_back(ImportEvent::ParakeetCaptureStopped {
                job_id,
                generation,
                session_id,
                wav_path: "micro.wav".into(),
                txt_path: Some("micro-incomplet.txt".into()),
                srt_path: Some("micro-incomplet.srt".into()),
                complete: false,
                final_segments: vec![block],
                unresolved: vec![crate::domain::SourceRange::new(8_000, 16_000, 16_000).unwrap()],
                diagnostic: Some("une portion audio demeure non confirmée".into()),
                automatic_final: None,
            });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.confirmed_text.as_deref(), Some("texte sûr"));
        assert_eq!(view.active_job, Some((job_id, JobState::Recoverable)));
        assert_eq!(view.parakeet_unresolved.len(), 1);
    }

    #[test]
    fn parakeet_automatic_final_import_stays_unverified_and_separate_from_confirmed_text() {
        let capture_job = JobId(701);
        let import_job = JobId(702);
        let generation = Generation::first();
        let session_id = 33;
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.active = Some(CommandIdentity {
            job_id: capture_job,
            generation,
        });
        let automatic = crate::ports::AutomaticFinalImport {
            request: ImportRequest {
                strategy: crate::parakeet_session::TranscriptionStrategy::LegacyV1,
                job_id: import_job,
                generation,
                source_path: "capture.wav".into(),
                source_sha256: None,
                source_samples: None,
                model_path: "parakeet.model".into(),
                model_sha256: [4; 32],
                destination: "destination".into(),
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Gpu,
                },
            },
            outputs: crate::ports::AutomaticFinalOutputPaths {
                txt_path: "capture-auto-non-verifie.txt".into(),
                srt_path: "capture-auto-non-verifie.srt".into(),
                companion_json_path: "capture.json".into(),
            },
            gpu_candidate: None,
        };
        let confirmed_a = crate::ports::WorkerSegment {
            job_id: capture_job,
            generation,
            instance_id: session_id,
            segment_id: crate::domain::SegmentId(1),
            range: crate::domain::SourceRange::new(0, 400, 16_000).unwrap(),
            text: "bloc live confirmé".into(),
        };
        app.io.events.push_back(ImportEvent::ParakeetLiveStarted {
            job_id: capture_job,
            generation,
            session_id,
            backend: "GPU".into(),
        });
        app.io.events.push_back(ImportEvent::ParakeetConfirmed {
            job_id: capture_job,
            generation,
            session_id,
            segment: confirmed_a.clone(),
        });
        app.io
            .events
            .push_back(ImportEvent::ParakeetCaptureStopped {
                job_id: capture_job,
                generation,
                session_id,
                wav_path: "capture.wav".into(),
                txt_path: Some("capture-incomplet.txt".into()),
                srt_path: Some("capture-incomplet.srt".into()),
                complete: false,
                final_segments: vec![confirmed_a.clone()],
                unresolved: vec![crate::domain::SourceRange::new(0, 800, 16_000).unwrap()],
                diagnostic: None,
                automatic_final: Some(automatic.clone()),
            });
        let pending = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(
            pending.confirmed_text.as_deref(),
            Some("bloc live confirmé")
        );
        assert_eq!(pending.parakeet_unresolved.len(), 1);
        let capture_a = pending
            .parakeet_captures
            .iter()
            .find(|capture| {
                capture.identity.job_id == capture_job && capture.identity.session_id == session_id
            })
            .unwrap();
        assert_eq!(
            capture_a.capture_state,
            crate::ports::ParakeetCaptureViewState::Finalized
        );
        assert_eq!(capture_a.wav_path.as_deref(), Some("capture.wav"));
        assert_eq!(capture_a.confirmed_segments, vec![confirmed_a]);
        assert_eq!(
            capture_a.automatic_final.as_ref().unwrap().status,
            crate::ports::AutomaticFinalTranscriptionStatus::Pending
        );
        assert_eq!(
            pending
                .automatic_final_transcription
                .as_ref()
                .unwrap()
                .status,
            crate::ports::AutomaticFinalTranscriptionStatus::Pending
        );
        assert!(matches!(
            app.io.effects.as_slice(),
            [ImportEffect::StartAutomaticFinalTranscription(_)]
        ));

        let next_capture = JobId(703);
        app.dispatch(AppCommand::StartDirectLiveConfigured {
            request: LiveRequest {
                job_id: next_capture,
                generation,
                model_path: automatic.request.model_path.clone(),
                model_sha256: automatic.request.model_sha256,
                destination: automatic.request.destination.clone(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                microphone_gain_db: 0,
                config: automatic.request.config.clone(),
            },
        })
        .unwrap();
        app.io.events.push_back(ImportEvent::ParakeetLiveStarted {
            job_id: next_capture,
            generation,
            session_id: session_id + 1,
            backend: "CUDA0".into(),
        });
        app.io.events.push_back(ImportEvent::ParakeetHypothesis {
            job_id: next_capture,
            generation,
            session_id: session_id + 1,
            sequence: 1,
            range: crate::domain::SourceRange::new(0, 800, 16_000).unwrap(),
            text: "hypothèse nouvelle session".into(),
            captured_at_unix_ms: 2,
            available_at_unix_ms: 3,
        });
        let next_capture_view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(
            next_capture_view.provisional_text.as_deref(),
            Some("hypothèse nouvelle session")
        );
        assert!(next_capture_view.confirmed_text.is_none());
        assert!(next_capture_view.parakeet_unresolved.is_empty());
        let capture_b = next_capture_view
            .parakeet_captures
            .iter()
            .find(|capture| {
                capture.identity.job_id == next_capture
                    && capture.identity.session_id == session_id + 1
            })
            .unwrap();
        assert_eq!(
            capture_b.capture_state,
            crate::ports::ParakeetCaptureViewState::Capturing
        );
        assert_eq!(capture_b.requested_compute, Some(ComputeChoice::Gpu));
        assert_eq!(capture_b.effective_backend, None);
        assert_eq!(
            capture_b.provisional_text.as_deref(),
            Some("hypothèse nouvelle session")
        );
        assert_eq!(
            app.active,
            Some(CommandIdentity {
                job_id: next_capture,
                generation
            })
        );
        assert_eq!(
            next_capture_view.active_job,
            Some((next_capture, JobState::Capturing))
        );
        assert_eq!(
            app.automatic_final_identity,
            Some(CommandIdentity {
                job_id: import_job,
                generation
            })
        );
        assert_eq!(
            app.automatic_final_state.as_ref().unwrap().request.job_id,
            import_job
        );

        let mut prepared = automatic.request.clone();
        prepared.source_sha256 = Some([7; 32]);
        prepared.source_samples = Some(16_000);
        app.io.events.push_back(ImportEvent::Prepared(prepared));
        app.dispatch(AppCommand::Refresh).unwrap();
        app.io.events.push_back(ImportEvent::Ready {
            job_id: import_job,
            generation,
            backend: "CUDA0".into(),
        });
        let running = app.dispatch(AppCommand::Refresh).unwrap();
        let running_capture_a = running
            .parakeet_captures
            .iter()
            .find(|capture| capture.identity.job_id == capture_job)
            .unwrap();
        let running_final_a = running_capture_a.automatic_final.as_ref().unwrap();
        assert_eq!(
            running_final_a.status,
            crate::ports::AutomaticFinalTranscriptionStatus::Running
        );
        assert_eq!(running_final_a.effective_backend.as_deref(), Some("CUDA0"));
        let running_capture_b = running
            .parakeet_captures
            .iter()
            .find(|capture| capture.identity.job_id == next_capture)
            .unwrap();
        assert_eq!(
            running_capture_b.capture_state,
            crate::ports::ParakeetCaptureViewState::Capturing
        );
        assert_eq!(
            running_capture_b.provisional_text.as_deref(),
            Some("hypothèse nouvelle session")
        );
        assert_eq!(
            running
                .parakeet_captures
                .iter()
                .find(|capture| { capture.identity.job_id == next_capture })
                .unwrap()
                .provisional_text
                .as_deref(),
            Some("hypothèse nouvelle session")
        );
        assert_eq!(
            running.provisional_text.as_deref(),
            Some("hypothèse nouvelle session")
        );
        assert_eq!(
            app.active,
            Some(CommandIdentity {
                job_id: next_capture,
                generation
            })
        );

        app.io.events.push_back(ImportEvent::End {
            job_id: import_job,
            generation,
            last_sequence: 0,
            last_offset: 16_000,
        });
        app.dispatch(AppCommand::Refresh).unwrap();
        assert!(app.io.effects.iter().any(|effect| matches!(
            effect,
            ImportEffect::Publish { job_id, generation: event_generation, last_sequence: 0 }
                if *job_id == import_job && *event_generation == generation
        )));

        app.io.events.push_back(ImportEvent::Published {
            job_id: import_job,
            generation,
        });
        let produced = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(
            produced.provisional_text.as_deref(),
            Some("hypothèse nouvelle session")
        );
        assert!(produced.parakeet_unresolved.is_empty());
        assert_eq!(
            app.active,
            Some(CommandIdentity {
                job_id: next_capture,
                generation
            })
        );
        let capture_a = produced
            .parakeet_captures
            .iter()
            .find(|capture| capture.identity.job_id == capture_job)
            .unwrap();
        assert_eq!(
            capture_a.automatic_final.as_ref().unwrap().status,
            crate::ports::AutomaticFinalTranscriptionStatus::ProducedUnverified
        );
        assert_eq!(
            capture_a
                .automatic_final
                .as_ref()
                .unwrap()
                .effective_backend
                .as_deref(),
            Some("CUDA0")
        );
        let capture_b = produced
            .parakeet_captures
            .iter()
            .find(|capture| capture.identity.job_id == next_capture)
            .unwrap();
        assert_eq!(
            capture_b.capture_state,
            crate::ports::ParakeetCaptureViewState::Capturing
        );
        assert_eq!(
            capture_b.provisional_text.as_deref(),
            Some("hypothèse nouvelle session")
        );
    }

    #[test]
    fn late_automatic_import_failure_updates_only_its_capture_row() {
        let capture_a = JobId(711);
        let import_a = JobId(712);
        let capture_b = JobId(713);
        let generation = Generation::first();
        let session_a = 41;
        let session_b = 42;
        let automatic = crate::ports::AutomaticFinalImport {
            request: ImportRequest {
                strategy: crate::parakeet_session::TranscriptionStrategy::LegacyV1,
                job_id: import_a,
                generation,
                source_path: "a.wav".into(),
                source_sha256: None,
                source_samples: None,
                model_path: "parakeet.model".into(),
                model_sha256: [4; 32],
                destination: "destination".into(),
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Gpu,
                },
            },
            outputs: crate::ports::AutomaticFinalOutputPaths {
                txt_path: "a-auto-non-verifie.txt".into(),
                srt_path: "a-auto-non-verifie.srt".into(),
                companion_json_path: "a.json".into(),
            },
            gpu_candidate: None,
        };
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.active = Some(CommandIdentity {
            job_id: capture_a,
            generation,
        });
        app.io.events.push_back(ImportEvent::ParakeetLiveStarted {
            job_id: capture_a,
            generation,
            session_id: session_a,
            backend: "CUDA0".into(),
        });
        app.io
            .events
            .push_back(ImportEvent::ParakeetCaptureStopped {
                job_id: capture_a,
                generation,
                session_id: session_a,
                wav_path: "a.wav".into(),
                txt_path: Some("a-incomplet.txt".into()),
                srt_path: Some("a-incomplet.srt".into()),
                complete: false,
                final_segments: Vec::new(),
                unresolved: Vec::new(),
                diagnostic: None,
                automatic_final: Some(automatic),
            });
        app.dispatch(AppCommand::Refresh).unwrap();
        app.dispatch(AppCommand::StartDirectLiveConfigured {
            request: LiveRequest {
                job_id: capture_b,
                generation,
                model_path: "parakeet.model".into(),
                model_sha256: [4; 32],
                destination: "destination".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                microphone_gain_db: 0,
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Gpu,
                },
            },
        })
        .unwrap();
        app.io.events.push_back(ImportEvent::ParakeetLiveStarted {
            job_id: capture_b,
            generation,
            session_id: session_b,
            backend: "CUDA0".into(),
        });
        app.io.events.push_back(ImportEvent::ParakeetHypothesis {
            job_id: capture_b,
            generation,
            session_id: session_b,
            sequence: 1,
            range: crate::domain::SourceRange::new(0, 800, 16_000).unwrap(),
            text: "B provisoire".into(),
            captured_at_unix_ms: 2,
            available_at_unix_ms: 3,
        });
        app.dispatch(AppCommand::Refresh).unwrap();
        app.io
            .events
            .push_back(ImportEvent::ParakeetCaptureStopped {
                job_id: capture_b,
                generation,
                session_id: session_b,
                wav_path: "b.wav".into(),
                txt_path: Some("b.txt".into()),
                srt_path: Some("b.srt".into()),
                complete: true,
                final_segments: Vec::new(),
                unresolved: Vec::new(),
                diagnostic: None,
                automatic_final: None,
            });
        let before_late_failure = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(
            before_late_failure.active_job,
            Some((capture_b, JobState::Complete))
        );
        let prior_global_message = before_late_failure.message.clone();
        let row_b_before = before_late_failure
            .parakeet_captures
            .iter()
            .find(|capture| capture.identity.job_id == capture_b)
            .unwrap()
            .clone();
        assert_eq!(row_b_before.provisional_text, None);
        assert_eq!(
            row_b_before.automatic_final.as_ref().unwrap().status,
            crate::ports::AutomaticFinalTranscriptionStatus::FailedOrUnavailable
        );
        let failure = "publication A impossible";
        app.io.events.push_back(ImportEvent::Failed {
            job_id: import_a,
            generation,
            message: failure.into(),
        });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        let row_a = view
            .parakeet_captures
            .iter()
            .find(|capture| capture.identity.job_id == capture_a)
            .unwrap();
        assert_eq!(
            row_a.automatic_final.as_ref().unwrap().status,
            crate::ports::AutomaticFinalTranscriptionStatus::FailedOrUnavailable
        );
        assert_eq!(
            row_a
                .automatic_final
                .as_ref()
                .unwrap()
                .diagnostic
                .as_deref(),
            Some(failure)
        );
        let row_b = view
            .parakeet_captures
            .iter()
            .find(|capture| capture.identity.job_id == capture_b)
            .unwrap();
        assert_eq!(
            row_b.capture_state,
            crate::ports::ParakeetCaptureViewState::Finalized
        );
        assert_eq!(row_b, &row_b_before);
        assert_eq!(row_b.wav_path.as_deref(), Some("b.wav"));
        assert_eq!(row_b.direct_txt_path.as_deref(), Some("b.txt"));
        assert_eq!(app.active, None);
        assert_eq!(view.active_job, Some((capture_b, JobState::Complete)));
        assert_eq!(view.message, prior_global_message);
    }

    #[test]
    fn direct_start_failure_before_live_started_keeps_a_capture_row_without_wav() {
        let capture_a = JobId(721);
        let capture_b = JobId(722);
        let generation = Generation::first();
        let session_a = 51;
        let model_path = "C:/known/parakeet.bin";
        let mut model_sha256 = [0; 32];
        let digest = crate::model_registry::model(crate::model_catalog::ModelId::ParakeetTdt06BV3)
            .artifact
            .sha256;
        for (index, byte) in model_sha256.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&digest[index * 2..index * 2 + 2], 16).unwrap();
        }
        let guard = crate::model_selection_app::ModelCommandGuard::new_with_model_paths(
            crate::model_registry::engine_snapshot(
                1,
                |_| crate::model_catalog::ModelAvailability::Ready,
                Vec::new(),
            ),
            1,
            [(
                crate::model_catalog::ModelId::ParakeetTdt06BV3,
                model_path.into(),
            )],
        );
        let mut app = ImportApplication::new(ScanTestIo::default()).with_model_guard(guard);
        app.active = Some(CommandIdentity {
            job_id: capture_a,
            generation,
        });
        app.io.events.push_back(ImportEvent::ParakeetLiveStarted {
            job_id: capture_a,
            generation,
            session_id: session_a,
            backend: "CUDA0".into(),
        });
        app.io
            .events
            .push_back(ImportEvent::ParakeetCaptureStopped {
                job_id: capture_a,
                generation,
                session_id: session_a,
                wav_path: "a.wav".into(),
                txt_path: Some("a.txt".into()),
                srt_path: Some("a.srt".into()),
                complete: true,
                final_segments: Vec::new(),
                unresolved: Vec::new(),
                diagnostic: None,
                automatic_final: None,
            });
        app.dispatch(AppCommand::Refresh).unwrap();

        app.dispatch(AppCommand::StartDirectLiveConfigured {
            request: LiveRequest {
                job_id: capture_b,
                generation,
                model_path: model_path.into(),
                model_sha256,
                destination: "destination".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                microphone_gain_db: 0,
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Gpu,
                },
            },
        })
        .unwrap();
        let diagnostic = "WorkerExited avant ParakeetLiveStarted";
        app.io.events.push_back(ImportEvent::Failed {
            job_id: capture_b,
            generation,
            message: diagnostic.into(),
        });
        let view = app.dispatch(AppCommand::Refresh).unwrap();

        let row_a = view
            .parakeet_captures
            .iter()
            .find(|capture| capture.identity.job_id == capture_a)
            .unwrap();
        assert_eq!(
            row_a.capture_state,
            crate::ports::ParakeetCaptureViewState::Finalized
        );
        assert_eq!(row_a.wav_path.as_deref(), Some("a.wav"));
        let row_b = view
            .parakeet_captures
            .iter()
            .find(|capture| capture.identity.job_id == capture_b)
            .unwrap();
        assert_eq!(row_b.identity.session_id, 0);
        assert_eq!(
            row_b.capture_state,
            crate::ports::ParakeetCaptureViewState::Failed
        );
        assert_eq!(row_b.capture_diagnostic.as_deref(), Some(diagnostic));
        assert_eq!(row_b.wav_path, None);
        assert_eq!(row_b.direct_txt_path, None);
        assert_eq!(row_b.direct_srt_path, None);
        assert_eq!(view.active_job, Some((capture_b, JobState::Recoverable)));
    }

    #[test]
    fn turbo_start_failure_before_live_started_does_not_create_parakeet_capture() {
        let job_id = JobId(723);
        let generation = Generation::first();
        let model_path = "C:/known/turbo.bin";
        let mut model_sha256 = [0; 32];
        let digest =
            crate::model_registry::model(crate::model_catalog::ModelId::WhisperLargeV3Turbo)
                .artifact
                .sha256;
        for (index, byte) in model_sha256.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&digest[index * 2..index * 2 + 2], 16).unwrap();
        }
        let guard = crate::model_selection_app::ModelCommandGuard::new_with_model_paths(
            crate::model_registry::engine_snapshot(
                1,
                |_| crate::model_catalog::ModelAvailability::Ready,
                Vec::new(),
            ),
            1,
            [(
                crate::model_catalog::ModelId::WhisperLargeV3Turbo,
                model_path.into(),
            )],
        );
        let mut app = ImportApplication::new(ScanTestIo::default()).with_model_guard(guard);
        app.dispatch(AppCommand::StartDirectLiveConfigured {
            request: LiveRequest {
                job_id,
                generation,
                model_path: model_path.into(),
                model_sha256,
                destination: "destination".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                microphone_gain_db: 0,
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            },
        })
        .unwrap();
        app.io.events.push_back(ImportEvent::Failed {
            job_id,
            generation,
            message: "échec du worker Turbo".into(),
        });

        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert!(view.parakeet_captures.is_empty());
        assert_eq!(view.active_job, Some((job_id, JobState::Recoverable)));
    }

    #[test]
    fn parakeet_late_confirmation_keeps_identity_and_reorders_display_chronologically() {
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.view.provisional_session_id = Some(9);
        let job_id = JobId(72);
        let generation = Generation::first();
        app.append_parakeet_confirmed(
            9,
            crate::ports::WorkerSegment {
                job_id,
                generation,
                instance_id: 9,
                segment_id: crate::domain::SegmentId(1),
                range: crate::domain::SourceRange::new(8_000, 12_000, 16_000).unwrap(),
                text: "fonctionne".into(),
            },
        )
        .unwrap();
        app.append_parakeet_confirmed(
            9,
            crate::ports::WorkerSegment {
                job_id,
                generation,
                instance_id: 9,
                segment_id: crate::domain::SegmentId(2),
                range: crate::domain::SourceRange::new(2_000, 6_000, 16_000).unwrap(),
                text: "test".into(),
            },
        )
        .unwrap();
        assert_eq!(app.view.confirmed_text.as_deref(), Some("test\nfonctionne"));
        assert_eq!(app.parakeet_confirmed[0].segment_id.0, 2);
        assert_eq!(app.parakeet_confirmed[1].segment_id.0, 1);
        assert_eq!(app.view.confirmed_samples, 12_000);
    }

    fn unified_capture_state(job_id: JobId) -> UnifiedCaptureAdmission {
        UnifiedCaptureAdmission {
            identity: CommandIdentity {
                job_id,
                generation: Generation::first(),
            },
            destination: "archive".into(),
            stop_submitted: true,
            publish_submitted: false,
            microphone_gain_requested_revision: 0,
            microphone_gain_requested_target_identity: None,
            next_sequence: 1,
            pending_persistence: 0,
            pending_segments: std::collections::VecDeque::new(),
            pending_sequences: std::collections::BTreeSet::new(),
            last_sequence: None,
            total_samples: None,
        }
    }

    #[test]
    fn late_unified_gain_rejection_does_not_rollback_newer_capture_view() {
        let rejected_job = JobId(0xCA);
        let current_job = JobId(0xCB);
        let rejected_identity = CommandIdentity {
            job_id: rejected_job,
            generation: Generation::first(),
        };
        let current_identity = CommandIdentity {
            job_id: current_job,
            generation: Generation::first(),
        };
        let mut app = ImportApplication::new(ScanTestIo::default());
        let mut rejected_capture = unified_capture_state(rejected_job);
        rejected_capture.microphone_gain_requested_revision = 2;
        rejected_capture.microphone_gain_requested_target_identity = Some("mic:A".into());
        let mut current_capture = unified_capture_state(current_job);
        current_capture.microphone_gain_requested_revision = 3;
        current_capture.microphone_gain_requested_target_identity = Some("mic:B".into());
        app.unified_captures
            .insert((rejected_job, Generation::first()), rejected_capture);
        app.unified_captures
            .insert((current_job, Generation::first()), current_capture);
        app.microphone_gain_view_owner = Some(current_identity);
        app.view.microphone_source_identity = Some("mic:B".into());
        app.view.microphone_gain_db = 6;
        app.view.microphone_gain_revision = 2;
        app.view.microphone_gain_requested_db = 12;
        app.view.microphone_gain_requested_revision = 3;
        app.view.message = Some("Gain B demandé".into());

        app.handle_unified_capture_event(ImportEvent::MicrophoneGainRejected {
            job_id: rejected_identity.job_id,
            generation: rejected_identity.generation,
            target_identity: "mic:A".into(),
            message: "source A unavailable".into(),
        })
        .unwrap();

        assert_eq!(
            app.view.microphone_source_identity.as_deref(),
            Some("mic:B")
        );
        assert_eq!(app.view.microphone_gain_db, 6);
        assert_eq!(app.view.microphone_gain_revision, 2);
        assert_eq!(app.view.microphone_gain_requested_db, 12);
        assert_eq!(app.view.microphone_gain_requested_revision, 3);
        assert_eq!(app.view.message.as_deref(), Some("Gain B demandé"));
        assert_eq!(
            app.unified_captures[&(current_job, Generation::first())]
                .microphone_gain_requested_revision,
            3
        );
    }

    fn prepared_unified_capture(job_id: JobId, source_samples: u64) -> ImportRequest {
        ImportRequest {
            strategy: crate::parakeet_session::TranscriptionStrategy::BoundedWindowsV1,
            job_id,
            generation: Generation::first(),
            source_path: "capture.wav".into(),
            source_sha256: Some([7; 32]),
            source_samples: Some(source_samples),
            model_path: "model.nmodel".into(),
            model_sha256: [8; 32],
            destination: "archive".into(),
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Cpu,
            },
        }
    }

    #[test]
    fn unified_capture_ready_records_the_effective_native_backend() {
        let job_id = JobId(0xC0);
        let generation = Generation::first();
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.unified_captures
            .insert((job_id, generation), unified_capture_state(job_id));
        app.add_unified_session_card(
            job_id,
            generation,
            "archive".into(),
            "capture.wav".into(),
            true,
        );

        app.handle_unified_capture_event(ImportEvent::Ready {
            job_id,
            generation,
            backend: "CPU (Auto fallback before audio: no current candidate)".into(),
        })
        .unwrap();

        let session = app
            .view
            .unified_parakeet_sessions
            .iter()
            .find(|session| (session.job_id, session.generation) == (job_id, generation))
            .unwrap();
        assert_eq!(session.phase, crate::ports::UnifiedSessionPhase::Capturing);
        assert_eq!(
            session.message.as_deref(),
            Some("Moteur actif : CPU (Auto fallback before audio: no current candidate)")
        );
    }

    #[test]
    fn unified_capture_prepared_progress_end_publishes_only_after_total_is_known() {
        let job_id = JobId(0xC1);
        let generation = Generation::first();
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.unified_captures
            .insert((job_id, generation), unified_capture_state(job_id));

        app.handle_unified_capture_event(ImportEvent::Prepared(prepared_unified_capture(
            job_id, 32_000,
        )))
        .unwrap();
        app.handle_unified_capture_event(ImportEvent::Progress {
            job_id,
            generation,
            completed_samples: 32_000,
            total_samples: 32_000,
        })
        .unwrap();
        app.handle_unified_capture_event(ImportEvent::End {
            job_id,
            generation,
            last_sequence: 0,
            last_offset: 32_000,
        })
        .unwrap();

        assert!(matches!(
            app.io.effects.last(),
            Some(ImportEffect::Publish { job_id: actual, generation: actual_generation, last_sequence: 0 })
                if *actual == job_id && *actual_generation == generation
        ));
    }

    #[test]
    fn unified_capture_rejects_progress_with_a_total_different_from_prepared_source() {
        let job_id = JobId(0xC2);
        let generation = Generation::first();
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.unified_captures
            .insert((job_id, generation), unified_capture_state(job_id));
        app.handle_unified_capture_event(ImportEvent::Prepared(prepared_unified_capture(
            job_id, 32_000,
        )))
        .unwrap();

        assert_eq!(
            app.handle_unified_capture_event(ImportEvent::Progress {
                job_id,
                generation,
                completed_samples: 31_999,
                total_samples: 31_999,
            }),
            Err(ApplicationError::InvalidCommand)
        );
        assert!(app.io.effects.is_empty());
    }

    #[test]
    fn unified_capture_rejects_end_until_prepared_total_arrives() {
        let job_id = JobId(0xC3);
        let generation = Generation::first();
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.unified_captures
            .insert((job_id, generation), unified_capture_state(job_id));

        assert_eq!(
            app.handle_unified_capture_event(ImportEvent::End {
                job_id,
                generation,
                last_sequence: 0,
                last_offset: 32_000,
            }),
            Err(ApplicationError::InvalidCommand)
        );
        assert!(app.io.effects.is_empty());
    }
}
