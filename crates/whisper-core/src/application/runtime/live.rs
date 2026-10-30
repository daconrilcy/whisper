use super::super::*;

impl<P: ImportIoPort> ImportApplication<P> {
    pub(in crate::application) fn handle_live_event(
        &mut self,
        event: ImportEvent,
        identity: CommandIdentity,
    ) -> Result<(), ApplicationError> {
        match event {
            event @ (ImportEvent::DirectCaptureActive { .. }
            | ImportEvent::DirectConfirmed(_)
            | ImportEvent::DirectPublished { .. }
            | ImportEvent::Diagnostic { .. }
            | ImportEvent::LiveProvisional { .. }) => self.handle_direct_event(event, identity),
            event @ (ImportEvent::ParakeetLiveStarted { .. }
            | ImportEvent::ParakeetInferenceReady { .. }) => {
                self.handle_parakeet_start_event(event, identity)
            }
            event @ (ImportEvent::ParakeetHypothesis { .. }
            | ImportEvent::ParakeetConfirmed { .. }
            | ImportEvent::ParakeetUnresolved { .. }
            | ImportEvent::ParakeetResolved { .. }) => {
                self.handle_parakeet_progress_event(event, identity)
            }
            event @ (ImportEvent::ParakeetCaptureProgress { .. }
            | ImportEvent::ParakeetInferenceUnavailable { .. }
            | ImportEvent::ParakeetAudioUnusable { .. }) => self.handle_parakeet_audio_event(event),
            event @ ImportEvent::ParakeetCaptureStopped { .. } => {
                self.handle_parakeet_stop_event(event)
            }
            event @ (ImportEvent::LiveCounters { .. }
            | ImportEvent::LiveFinalizing { .. }
            | ImportEvent::AwaitingCpuChoice { .. }) => {
                self.handle_live_counter_event(event, identity)
            }
            event @ (ImportEvent::LiveAttemptReset { .. }
            | ImportEvent::LiveAttemptStarting { .. }) => {
                self.handle_live_reset_event(event, identity)
            }
            event
            @ (ImportEvent::LiveRetryingOnCpu { .. } | ImportEvent::WindowFinished { .. }) => {
                self.handle_live_retry_event(event, identity)
            }
            _ => unreachable!("live event already selected"),
        }
    }

    fn handle_direct_event(
        &mut self,
        event: ImportEvent,
        identity: CommandIdentity,
    ) -> Result<(), ApplicationError> {
        let CommandIdentity { job_id, generation } = identity;
        match event {
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
            _ => unreachable!("live event group already selected"),
        }
        Ok(())
    }

    fn handle_parakeet_start_event(
        &mut self,
        event: ImportEvent,
        identity: CommandIdentity,
    ) -> Result<(), ApplicationError> {
        let job_id = identity.job_id;
        match event {
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
            _ => unreachable!("live event group already selected"),
        }
        Ok(())
    }

    fn handle_parakeet_progress_event(
        &mut self,
        event: ImportEvent,
        identity: CommandIdentity,
    ) -> Result<(), ApplicationError> {
        match event {
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
            _ => unreachable!("live event group already selected"),
        }
        Ok(())
    }

    fn handle_parakeet_audio_event(&mut self, event: ImportEvent) -> Result<(), ApplicationError> {
        match event {
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
            _ => unreachable!("Parakeet audio event already selected"),
        }
        Ok(())
    }

    fn handle_parakeet_stop_event(&mut self, event: ImportEvent) -> Result<(), ApplicationError> {
        match event {
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
                self.enqueue_capture_final(automatic_final)?;
            }
            _ => unreachable!("live event group already selected"),
        }
        Ok(())
    }

    fn handle_live_counter_event(
        &mut self,
        event: ImportEvent,
        identity: CommandIdentity,
    ) -> Result<(), ApplicationError> {
        let CommandIdentity { job_id, generation } = identity;
        match event {
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
            _ => unreachable!("live event group already selected"),
        }
        Ok(())
    }

    fn handle_live_reset_event(
        &mut self,
        event: ImportEvent,
        identity: CommandIdentity,
    ) -> Result<(), ApplicationError> {
        let job_id = identity.job_id;
        match event {
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
            _ => unreachable!("live event group already selected"),
        }
        Ok(())
    }

    fn handle_live_retry_event(
        &mut self,
        event: ImportEvent,
        identity: CommandIdentity,
    ) -> Result<(), ApplicationError> {
        let job_id = identity.job_id;
        match event {
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
            _ => unreachable!("live event group already selected"),
        }
        Ok(())
    }

    fn enqueue_capture_final(
        &mut self,
        automatic_final: Option<crate::ports::AutomaticFinalImport>,
    ) -> Result<(), ApplicationError> {
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
                    Some(
                        "File d'import automatique saturée ; WAV et sorties directes conservés"
                            .into(),
                    ),
                    None,
                );
                self.view.automatic_final_transcription =
                    Some(crate::ports::AutomaticFinalTranscriptionView {
                        status:
                            crate::ports::AutomaticFinalTranscriptionStatus::FailedOrUnavailable,
                        diagnostic: Some(
                            "File d'import automatique saturée ; WAV et sorties directes conservés"
                                .into(),
                        ),
                        ..final_view
                    });
            } else {
                self.view.automatic_final_transcription = Some(final_view);
                self.automatic_final_queue.push_back(automatic_final);
                self.start_next_automatic_final()?;
            }
        } else {
            self.view.automatic_final_transcription =
                Some(crate::ports::AutomaticFinalTranscriptionView {
                    status: crate::ports::AutomaticFinalTranscriptionStatus::FailedOrUnavailable,
                    txt_path: String::new(),
                    srt_path: String::new(),
                    model_sha256: [0; 32],
                    requested_compute: crate::domain::ComputeChoice::Auto,
                    effective_backend: None,
                    diagnostic: Some("Import automatique indisponible pour cette capture".into()),
                });
        }
        Ok(())
    }

    pub(in crate::application) fn start_live_command(
        &mut self,
        command: AppCommand,
        admitted_model: Option<crate::model_catalog::ModelId>,
    ) -> Result<AppView, ApplicationError> {
        match command {
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
                self.initialize_live_view(
                    identity,
                    direct,
                    requested_microphone_identity,
                    requested_microphone_gain_db,
                );
            }
            _ => unreachable!("live start already selected"),
        }
        Ok(self.view.clone())
    }

    fn initialize_live_view(
        &mut self,
        identity: CommandIdentity,
        direct: bool,
        requested_microphone_identity: Option<String>,
        requested_microphone_gain_db: i8,
    ) {
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

    pub(in crate::application) fn finish_live_cpu_command(
        &mut self,
        command: AppCommand,
    ) -> Result<AppView, ApplicationError> {
        match command {
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
            _ => unreachable!("CPU finish already selected"),
        }
        Ok(self.view.clone())
    }
}
