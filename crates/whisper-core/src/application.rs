use crate::domain::{Generation, JobId, JobState};
use crate::ports::{
    DiagnosticNotice, ImportEffect, ImportEvent, ImportIoPort, ImportQueueStatus, ImportRequest,
    PortError,
};
#[cfg(test)]
use crate::{
    domain::JobConfig,
    ports::{
        DuplicateImportChoice, DuplicateImportPrompt, ImportQueueEntry, LiveRequest,
        SourceSnapshotRecoveryCandidate,
    },
};

const MAX_EVENTS_PER_REFRESH: usize = 4;

mod contracts;
mod diagnostics;
mod event_metadata;
mod runtime;

pub use contracts::{
    AppCommand, AppFacade, AppView, Application, ApplicationError, CommandIdentity,
    DirectCapturePhase, DirectEnginePhase,
};
use diagnostics::{record_application_timing, trace_application_event};

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

    fn fail(&mut self, error: PortError) -> ApplicationError {
        self.view.message = Some(format!("Import indisponible : {error:?}"));
        ApplicationError::PortUnavailable
    }
}
#[cfg(test)]
mod tests;
