use super::*;

pub(in crate::ui::model) fn next_import_job_id() -> JobId {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(1, |duration| {
            duration.as_nanos().min(u64::MAX as u128) as u64
        });
    let mut observed = NEXT_IMPORT_JOB_ID.load(Ordering::Relaxed);
    let serial = loop {
        let next = now.max(observed.saturating_add(1));
        match NEXT_IMPORT_JOB_ID.compare_exchange_weak(
            observed,
            next,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => break next,
            Err(actual) => observed = actual,
        }
    };
    JobId(((std::process::id() as u128) << 64) | serial as u128)
}

pub(in crate::ui::model) fn queued_request_for_compute(
    durable: &ImportRequest,
    compute: ComputeChoice,
) -> ImportRequest {
    let mut request = durable.clone();
    request.config.compute = compute;
    request
}

pub(in crate::ui::model) fn command_compute_choice(command: &AppCommand) -> Option<ComputeChoice> {
    match command {
        AppCommand::StartImport { request }
        | AppCommand::EnqueueImport { request }
        | AppCommand::ProcessQueued { request }
        | AppCommand::ResumeInterrupted { request } => Some(request.config.compute),
        AppCommand::StartLiveConfigured { request }
        | AppCommand::StartUnifiedLiveConfigured { request }
        | AppCommand::StartDirectLiveConfigured { request } => Some(request.config.compute),
        _ => None,
    }
}

pub(in crate::ui::model) fn gpu_selection_identity(
    command: &AppCommand,
) -> Option<(JobId, Generation)> {
    match command {
        AppCommand::ResumeInterrupted { request } => {
            Some((request.job_id, request.generation.next()?))
        }
        _ => command
            .identity()
            .map(|identity| (identity.job_id, identity.generation)),
    }
}

impl<A: Application> DesktopApp<A> {
    pub(in crate::ui::model) fn start_import(&mut self) {
        self.compute_choice_touched = true;
        let Some((model_path, model_sha256)) = self.selected_model_artifact() else {
            self.view.message = Some("Le modèle sélectionné n'est pas disponible.".into());
            return;
        };
        let job_id = next_import_job_id();
        let generation = Generation::first();
        let config = JobConfig {
            language: selected_language(&self.selection),
            compute: compute_for_model(self.selection.model, self.compute_choice),
        };
        if self.dispatch(AppCommand::EnqueueImport {
            request: ImportRequest {
                strategy: if self.unified_candidate_enabled
                    && self.selection.model
                        == Some(whisper_core::model_catalog::ModelId::ParakeetTdt06BV3)
                {
                    whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
                } else {
                    whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1
                },
                job_id,
                generation,
                source_path: self.paths.audio.trim().to_owned(),
                source_sha256: None,
                source_samples: None,
                model_path,
                model_sha256,
                destination: self.paths.destination.trim().to_owned(),
                config,
            },
        }) {
            self.reactivity_log.record_once(
                process_log_session(),
                "first_command_accepted",
                unix_time_ms(),
                Some(self.process_started_at_unix_ms),
            );
        }
    }

    pub(in crate::ui::model) fn start_microphone_test(&mut self) {
        if !microphone_test_start_enabled(true, &self.view) {
            return;
        }
        let was_default = matches!(
            self.microphones.effective(),
            MicrophoneChoice::WindowsDefault
        );
        let _ = self.microphones.refresh();
        let Some(source_identity) = self.microphones.target_identity().map(str::to_owned) else {
            self.microphone_notice = Some(
                self.microphones
                    .target_error()
                    .unwrap_or("Identité du microphone indisponible")
                    .to_owned(),
            );
            return;
        };
        let Ok(identity) =
            whisper_core::microphone_level::MicrophoneIdentity::new(&source_identity)
        else {
            self.microphone_notice = Some("Identité du microphone invalide".into());
            return;
        };
        let gain_db = self.paths.microphone_gain(&identity).value();
        let job_id = next_import_job_id();
        let generation = Generation::first();
        if self.dispatch(AppCommand::StartMicrophoneTest {
            job_id,
            generation,
            microphone: self.microphones.capture_choice(),
            gain_db,
        }) {
            self.microphone_test_identity = Some((job_id, generation));
            self.last_test_default_identity = was_default.then_some(source_identity);
            self.confirmed_live_default_identity = None;
            self.active_identity = Some((job_id, generation));
            self.microphone_notice = None;
        }
    }

    pub(in crate::ui::model) fn set_microphone_gain(&mut self, gain_db: i8) {
        let unified_capture_identity = self.active_unified_capture_identity();
        let source_identity = if unified_capture_identity.is_some() {
            self.view.microphone_source_identity.as_deref()
        } else {
            self.view
                .microphone_source_identity
                .as_deref()
                .or_else(|| self.microphones.target_identity())
        };
        let Some(source_identity) = source_identity else {
            self.microphone_notice = Some(
                self.microphones
                    .target_error()
                    .unwrap_or("Identité du microphone indisponible")
                    .to_owned(),
            );
            return;
        };
        let Ok(identity) = whisper_core::microphone_level::MicrophoneIdentity::new(source_identity)
        else {
            self.microphone_notice = Some("Identité du microphone invalide".into());
            return;
        };
        let Ok(gain) = whisper_core::microphone_level::GainDb::new(gain_db) else {
            return;
        };
        self.paths.set_microphone_gain(&identity, gain);
        let active_capture = self.view.microphone_test_active
            || self.view.direct_capture_phase.is_some()
            || unified_capture_identity.is_some();
        if let Some((job_id, generation)) = self.active_identity
            && active_capture
            && let Some(target_identity) = if unified_capture_identity.is_some() {
                self.view.microphone_source_identity.clone()
            } else {
                self.view
                    .microphone_source_identity
                    .clone()
                    .or_else(|| self.microphones.target_identity().map(str::to_owned))
            }
        {
            self.dispatch(AppCommand::SetMicrophoneGain {
                job_id,
                generation,
                target_identity,
                expected_revision: self.view.microphone_gain_requested_revision,
                gain,
            });
        }
    }

    pub(in crate::ui::model) fn active_unified_capture_identity(
        &self,
    ) -> Option<(JobId, Generation)> {
        let identity = self.active_identity?;
        self.view
            .unified_parakeet_sessions
            .iter()
            .any(|session| {
                (session.job_id, session.generation) == identity
                    && session.microphone_capture
                    && matches!(
                        session.phase,
                        whisper_core::ports::UnifiedSessionPhase::Starting
                            | whisper_core::ports::UnifiedSessionPhase::Capturing
                    )
            })
            .then_some(identity)
    }

    pub(in crate::ui::model) fn stop_microphone_test(&mut self) {
        let Some((job_id, generation)) = self.microphone_test_identity else {
            return;
        };
        self.dispatch(AppCommand::StopMicrophoneTest { job_id, generation });
    }

    pub(in crate::ui::model) fn start_live(&mut self, generation: Generation) {
        let Some(microphone) = self.microphone_for_live() else {
            return;
        };
        let target_identity = match &microphone {
            MicrophoneChoice::Device(identity) => Some(identity.as_str()),
            MicrophoneChoice::WindowsDefault => None,
        };
        let clicked_at = unix_time_ms();
        self.live_clicked_at_unix_ms = Some(clicked_at);
        self.live_attempt_counter = self.live_attempt_counter.saturating_add(1);
        self.compute_choice_touched = true;
        let Some((model_path, model_sha256)) = self.selected_model_artifact() else {
            self.view.message = Some("Le modèle sélectionné n'est pas disponible.".into());
            return;
        };
        if generation == Generation::first() {
            // The new-recording button starts a new group. Recovery and continuation
            // use a later generation and retain the previous durable identity.
            self.live_request = None;
        }
        let serial = NEXT_JOB.fetch_add(1, Ordering::Relaxed);
        let job_id = self.live_request.as_ref().map_or_else(
            || JobId(((std::process::id() as u128) << 64) | serial as u128),
            |request| request.job_id,
        );
        let request = LiveRequest {
            job_id,
            generation,
            model_path,
            model_sha256,
            destination: self.paths.destination.trim().to_owned(),
            group_offset_samples: self.live_request.as_ref().map_or(0, |previous| {
                previous
                    .group_offset_samples
                    .saturating_add(self.view.captured_samples)
            }),
            auto_stop_after_speech_samples: None,
            microphone: microphone.clone(),
            microphone_gain_db: target_identity
                .and_then(|source_identity| {
                    whisper_core::microphone_level::MicrophoneIdentity::new(source_identity).ok()
                })
                .map(|identity| self.paths.microphone_gain(&identity).value())
                .unwrap_or(0),
            config: JobConfig {
                language: selected_language(&self.selection),
                compute: compute_for_model(self.selection.model, self.compute_choice),
            },
        };
        let session = live_log_session(job_id, generation, self.live_attempt_counter);
        self.reactivity_log
            .record_once(session, "micro_clicked", clicked_at, None);
        self.active_identity = Some((job_id, generation));
        self.live_request = Some(request.clone());
        let command = if self.unified_candidate_enabled
            && self.selection.model == Some(ModelId::ParakeetTdt06BV3)
        {
            AppCommand::StartUnifiedLiveConfigured { request }
        } else {
            AppCommand::StartDirectLiveConfigured { request }
        };
        if self.dispatch(command) {
            self.confirmed_live_default_identity = None;
            if matches!(
                self.microphones.effective(),
                MicrophoneChoice::WindowsDefault
            ) {
                self.last_test_default_identity = target_identity.map(str::to_owned);
            }
            if self.selection.model == Some(ModelId::ParakeetTdt06BV3) {
                self.selected_parakeet_capture = Some((job_id, generation));
                self.selected_capture_text = CaptureTextSelection::Confirmed;
                self.automatic_draft_cache = None;
                self.automatic_draft_error = None;
            }
            self.reactivity_log.record_once(
                process_log_session(),
                "first_command_accepted",
                unix_time_ms(),
                Some(self.process_started_at_unix_ms),
            );
        }
    }

    fn microphone_for_live(&mut self) -> Option<MicrophoneChoice> {
        if self.view.microphone_test_active {
            self.stop_microphone_test();
            self.view.message = Some(
                "Test microphone arrêté. Cliquez de nouveau pour démarrer la transcription.".into(),
            );
            return None;
        }
        if !model_supports_live(self.selection.model) {
            self.view.message =
                Some("Le modèle sélectionné est qualifié uniquement pour l'import.".into());
            return None;
        }
        let _ = self.microphones.refresh();
        let microphone = self.microphones.capture_choice();
        let target_identity = match &microphone {
            MicrophoneChoice::Device(identity) => Some(identity.as_str()),
            MicrophoneChoice::WindowsDefault => None,
        };
        if matches!(
            self.microphones.effective(),
            MicrophoneChoice::WindowsDefault
        ) && let (Some(previous), Some(current)) =
            (self.last_test_default_identity.as_deref(), target_identity)
            && previous != current
            && self.confirmed_live_default_identity.as_deref() != Some(current)
        {
            self.confirmed_live_default_identity = Some(current.to_owned());
            let labels = MicrophoneLabelProjection::new(self.microphones.devices());
            self.microphone_notice = Some(format!(
                "Le microphone par défaut est passé de {} à {} depuis le test. Vérifiez ce changement puis cliquez de nouveau pour démarrer.",
                labels.device_label(previous),
                labels.device_label(current)
            ));
            return None;
        }
        if self.selection.model == Some(whisper_core::model_catalog::ModelId::ParakeetTdt06BV3)
            && self.selection.manual_language.is_some()
        {
            self.view.message = Some(
                "Le microphone Parakeet requiert la détection automatique de la langue.".into(),
            );
            return None;
        }
        Some(microphone)
    }
}
