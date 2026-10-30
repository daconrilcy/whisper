use super::*;

impl<P: ImportIoPort> ImportApplication<P> {
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

    pub(super) fn set_capture_automatic_final_status(
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

    pub(super) fn mark_automatic_final_failed(
        &mut self,
        identity: CommandIdentity,
        message: String,
    ) {
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

    pub(super) fn start_next_automatic_final(&mut self) -> Result<(), ApplicationError> {
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

    pub(super) fn retry_automatic_final_segment(&mut self) -> Result<bool, ApplicationError> {
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

    pub(super) fn handle_automatic_final_event(
        &mut self,
        event: ImportEvent,
        identity: CommandIdentity,
    ) -> Result<(), ApplicationError> {
        self.track_automatic_final_event(&event, identity);
        let is_current_view = self.automatic_final_view_identity == Some(identity)
            && self.active.is_none_or(|active| active == identity);
        match event {
            event @ (ImportEvent::Prepared(_)
            | ImportEvent::Ready { .. }
            | ImportEvent::Progress { .. }) => {
                self.handle_automatic_final_preparation(event, is_current_view)
            }
            event @ (ImportEvent::Segment(_)
            | ImportEvent::Persisted { .. }
            | ImportEvent::End { .. }) => self.handle_automatic_final_persistence(event, identity),
            event @ (ImportEvent::Published { .. }
            | ImportEvent::Failed { .. }
            | ImportEvent::Stopped { .. }) => {
                self.handle_automatic_final_terminal(event, identity, is_current_view)
            }
            _ => Err(ApplicationError::InvalidCommand),
        }
    }

    fn handle_automatic_final_preparation(
        &mut self,
        event: ImportEvent,
        is_current_view: bool,
    ) -> Result<(), ApplicationError> {
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
            _ => unreachable!("automatic final event group already selected"),
        }
        Ok(())
    }

    fn handle_automatic_final_persistence(
        &mut self,
        event: ImportEvent,
        identity: CommandIdentity,
    ) -> Result<(), ApplicationError> {
        match event {
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
            _ => unreachable!("automatic final event group already selected"),
        }
        Ok(())
    }

    fn handle_automatic_final_terminal(
        &mut self,
        event: ImportEvent,
        identity: CommandIdentity,
        is_current_view: bool,
    ) -> Result<(), ApplicationError> {
        match event {
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
            _ => unreachable!("automatic final event group already selected"),
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
