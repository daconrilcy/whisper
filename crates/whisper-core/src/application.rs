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
            trace_application_event(&event);
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
        if let ImportEvent::DuplicateImportFound(prompt) = &event
            && let Some(identity) = self.automatic_final_identity
            && identity.job_id == prompt.request.job_id
            && identity.generation == prompt.request.generation
        {
            self.mark_automatic_final_failed(
                identity,
                "Import final automatique déjà présent ; aucune sortie n'a été écrasée".into(),
            );
            return Ok(());
        }
        if self.handle_queue_event(&event)? {
            return Ok(());
        }
        let (job_id, generation) = event.identity();
        let identity = CommandIdentity { job_id, generation };
        self.invalidate_microphone_acquisition(&event, identity);
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
        self.project_unified_import_event(&event, identity);
        if self.stopping == Some(identity)
            && self.request.is_some()
            && matches!(event, ImportEvent::Segment(_))
        {
            return Ok(());
        }
        match event {
            event @ (ImportEvent::MicrophoneGainApplied { .. }
            | ImportEvent::MicrophoneGainRejected { .. }
            | ImportEvent::MicrophoneTestStarted { .. }
            | ImportEvent::MicrophoneMeter(_)
            | ImportEvent::MicrophoneSpectrum(_)
            | ImportEvent::MicrophoneTestStopped { .. }) => {
                return self.handle_microphone_event(event, identity);
            }
            event @ (ImportEvent::DirectCaptureActive { .. }
            | ImportEvent::DirectConfirmed(_)
            | ImportEvent::DirectPublished { .. }
            | ImportEvent::Diagnostic { .. }
            | ImportEvent::LiveProvisional { .. }
            | ImportEvent::ParakeetLiveStarted { .. }
            | ImportEvent::ParakeetInferenceReady { .. }
            | ImportEvent::ParakeetHypothesis { .. }
            | ImportEvent::ParakeetConfirmed { .. }
            | ImportEvent::ParakeetUnresolved { .. }
            | ImportEvent::ParakeetResolved { .. }
            | ImportEvent::ParakeetCaptureProgress { .. }
            | ImportEvent::ParakeetInferenceUnavailable { .. }
            | ImportEvent::ParakeetAudioUnusable { .. }
            | ImportEvent::ParakeetCaptureStopped { .. }
            | ImportEvent::LiveCounters { .. }
            | ImportEvent::LiveFinalizing { .. }
            | ImportEvent::AwaitingCpuChoice { .. }
            | ImportEvent::LiveAttemptReset { .. }
            | ImportEvent::LiveAttemptStarting { .. }
            | ImportEvent::LiveRetryingOnCpu { .. }
            | ImportEvent::WindowFinished { .. }) => {
                return self.handle_live_event(event, identity);
            }
            ImportEvent::UnifiedDocumentPage { .. }
            | ImportEvent::UnifiedAudioPartsPage { .. }
            | ImportEvent::UnifiedPublicationArtifacts { .. } => {}
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
            event @ (ImportEvent::ResumeCheckpoint { .. }
            | ImportEvent::Prepared(_)
            | ImportEvent::Ready { .. }
            | ImportEvent::Progress { .. }
            | ImportEvent::Segment(_)
            | ImportEvent::Persisted { .. }
            | ImportEvent::End { .. }
            | ImportEvent::Published { .. }
            | ImportEvent::Failed { .. }
            | ImportEvent::Stopped { .. }) => {
                return self.handle_import_event(event, identity);
            }
            // Handled before the job-identity gate above; it is a background
            // archive-history update and does not belong to an active import.
            ImportEvent::ReplacementCleanups(_) => return Ok(()),
        }
        Ok(())
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
                record_application_timing(
                    "refresh",
                    identity.map(|identity| identity.job_id),
                    identity.map(|identity| identity.generation),
                    None,
                );
                self.poll_once()?;
            }
            command @ (AppCommand::SelectUnifiedSession { .. }
            | AppCommand::ReadUnifiedDocumentPage { .. }
            | AppCommand::ReadUnifiedAudioPartsPage { .. }) => {
                return self.handle_unified_selection_command(command);
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
            command @ (AppCommand::StartMicrophoneTest { .. }
            | AppCommand::StopMicrophoneTest { .. }) => {
                return self.handle_microphone_test_command(command);
            }
            command @ (AppCommand::ScanHistory { .. }
            | AppCommand::InspectSourceSnapshotRecovery
            | AppCommand::ConfirmSourceSnapshotRecovery
            | AppCommand::CancelSourceSnapshotRecovery
            | AppCommand::ResolveLivePublication { .. }
            | AppCommand::RemoveQueued { .. }) => return self.handle_queue_command(command),
            command @ AppCommand::StartUnifiedLiveConfigured { .. } => {
                return self.start_unified_live_command(command);
            }
            command @ (AppCommand::StartLiveConfigured { .. }
            | AppCommand::StartDirectLiveConfigured { .. }) => {
                return self.start_live_command(command, admitted_model);
            }
            command @ AppCommand::SetMicrophoneGain { .. } => {
                return self.handle_microphone_gain_command(command);
            }
            command @ AppCommand::FinishLiveOnCpu { .. } => {
                return self.finish_live_cpu_command(command);
            }
            command @ AppCommand::StartImport { .. } => {
                return self.handle_start_import_command(command);
            }
            command @ (AppCommand::EnqueueImport { .. }
            | AppCommand::ResolveDuplicateImport { .. }
            | AppCommand::CancelDuplicateImport
            | AppCommand::ResolveImportPublication { .. }
            | AppCommand::ResumeReplacementCleanup { .. }) => {
                return self.handle_queue_command(command);
            }
            command @ (AppCommand::ProcessQueued { .. } | AppCommand::ResumeInterrupted { .. }) => {
                return self.handle_resume_import_command(command);
            }
            command @ AppCommand::StopUnifiedLive { .. } => {
                return self.stop_unified_live_command(command);
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

#[cfg(test)]
mod tests;
