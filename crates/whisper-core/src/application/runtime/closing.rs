use super::*;

impl<P: ImportIoPort> ImportApplication<P> {
    pub(super) fn update_close_ready(&mut self) {
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

    pub(super) fn drive_close_queue_removals(&mut self) -> Result<(), ApplicationError> {
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

    pub(super) fn request_close(&mut self) -> Result<AppView, ApplicationError> {
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
        Ok(self.view.clone())
    }

    pub(super) fn stop_or_quit(
        &mut self,
        job_id: JobId,
        generation: Generation,
    ) -> Result<AppView, ApplicationError> {
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
                self.view.message =
                    Some("Publication en cours ; l’arrêt est trop tardif. Attendez sa fin.".into());
                return Ok(self.view.clone());
            }
            Err(error) => return Err(self.fail(error)),
        }
        for capture in &mut self.view.parakeet_captures {
            if capture.identity.job_id == job_id
                && capture.identity.generation == generation
                && capture.capture_state == crate::ports::ParakeetCaptureViewState::Capturing
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
        Ok(self.view.clone())
    }
}
