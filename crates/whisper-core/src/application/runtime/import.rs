use super::super::*;

impl<P: ImportIoPort> ImportApplication<P> {
    pub(in crate::application) fn handle_import_event(
        &mut self,
        event: ImportEvent,
        identity: CommandIdentity,
    ) -> Result<(), ApplicationError> {
        match event {
            event @ (ImportEvent::ResumeCheckpoint { .. } | ImportEvent::Prepared(_)) => {
                self.handle_import_preparation(event, identity)
            }
            event @ (ImportEvent::Ready { .. } | ImportEvent::Progress { .. }) => {
                self.handle_import_progress(event, identity)
            }
            event @ (ImportEvent::Segment(_)
            | ImportEvent::Persisted { .. }
            | ImportEvent::End { .. }) => self.handle_import_persistence(event, identity),
            event @ (ImportEvent::Published { .. }
            | ImportEvent::Failed { .. }
            | ImportEvent::Stopped { .. }) => self.handle_import_terminal(event, identity),
            _ => unreachable!("import event already selected"),
        }
    }

    fn handle_import_preparation(
        &mut self,
        event: ImportEvent,
        identity: CommandIdentity,
    ) -> Result<(), ApplicationError> {
        let job_id = identity.job_id;
        match event {
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
            _ => unreachable!("import event group already selected"),
        }
        Ok(())
    }

    fn handle_import_progress(
        &mut self,
        event: ImportEvent,
        identity: CommandIdentity,
    ) -> Result<(), ApplicationError> {
        let job_id = identity.job_id;
        match event {
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
            _ => unreachable!("import event group already selected"),
        }
        Ok(())
    }

    fn handle_import_persistence(
        &mut self,
        event: ImportEvent,
        identity: CommandIdentity,
    ) -> Result<(), ApplicationError> {
        let CommandIdentity { job_id, generation } = identity;
        match event {
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
            _ => unreachable!("import event group already selected"),
        }
        Ok(())
    }

    fn handle_import_terminal(
        &mut self,
        event: ImportEvent,
        identity: CommandIdentity,
    ) -> Result<(), ApplicationError> {
        let job_id = identity.job_id;
        match event {
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
            _ => unreachable!("import event group already selected"),
        }
        Ok(())
    }

    pub(in crate::application) fn maybe_publish(
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

    pub(in crate::application) fn accept_segment(
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
    pub(in crate::application) fn retry_pending_segment(
        &mut self,
    ) -> Result<bool, ApplicationError> {
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
    pub(in crate::application) fn handle_start_import_command(
        &mut self,
        command: AppCommand,
    ) -> Result<AppView, ApplicationError> {
        match command {
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
                record_application_timing(
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
            _ => unreachable!("start import already selected"),
        }
        Ok(self.view.clone())
    }

    pub(in crate::application) fn handle_resume_import_command(
        &mut self,
        command: AppCommand,
    ) -> Result<AppView, ApplicationError> {
        match command {
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
            _ => unreachable!("queued import command already selected"),
        }
        Ok(self.view.clone())
    }
}
