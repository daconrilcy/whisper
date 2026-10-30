use crate::domain::{Generation, JobConfig, JobId, JobState};
use crate::ports::{
    ArchiveHistoryItem, DuplicateImportChoice, DuplicateImportPrompt, ImportQueueEntry,
    ImportRequest, LiveRequest, ReplacementCleanupInfo, SourceSnapshotRecoveryCandidate,
};
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
