use super::*;

impl<P: ImportIoPort> ImportApplication<P> {
    pub(super) fn invalidate_microphone_acquisition(
        &mut self,
        event: &ImportEvent,
        identity: CommandIdentity,
    ) {
        let CommandIdentity { job_id, generation } = identity;
        if self
            .view
            .microphone_acquisition
            .as_ref()
            .is_some_and(|owner| owner.job_id == job_id && owner.generation == generation)
            && matches!(
                event,
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
    }

    pub(super) fn handle_microphone_event(
        &mut self,
        event: ImportEvent,
        identity: CommandIdentity,
    ) -> Result<(), ApplicationError> {
        match event {
            event @ (ImportEvent::MicrophoneGainApplied { .. }
            | ImportEvent::MicrophoneGainRejected { .. }) => {
                self.handle_microphone_gain_event(event)
            }
            event => self.handle_microphone_test_event(event, identity),
        }
    }

    fn handle_microphone_gain_event(&mut self, event: ImportEvent) -> Result<(), ApplicationError> {
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
            _ => unreachable!("microphone gain event already selected"),
        }
        Ok(())
    }

    fn handle_microphone_test_event(
        &mut self,
        event: ImportEvent,
        identity: CommandIdentity,
    ) -> Result<(), ApplicationError> {
        match event {
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
            _ => unreachable!("microphone test event already selected"),
        }
        Ok(())
    }

    pub(super) fn handle_unified_microphone_event(
        &mut self,
        event: ImportEvent,
        key: (JobId, Generation),
    ) -> Result<(), ApplicationError> {
        match event {
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
            _ => unreachable!("unified microphone event already selected"),
        }
        Ok(())
    }

    pub(super) fn handle_microphone_test_command(
        &mut self,
        command: AppCommand,
    ) -> Result<AppView, ApplicationError> {
        match command {
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
            _ => unreachable!("microphone test command already selected"),
        }
        Ok(self.view.clone())
    }

    pub(super) fn handle_microphone_gain_command(
        &mut self,
        command: AppCommand,
    ) -> Result<AppView, ApplicationError> {
        match command {
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
            _ => unreachable!("microphone gain command already selected"),
        }
        Ok(self.view.clone())
    }
}
