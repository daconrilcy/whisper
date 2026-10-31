use super::*;
mod actions;
mod observability;

impl<A: Application> DesktopApp<A> {
    pub(in crate::ui::model) fn poll_gpu_probe(&mut self) {
        let result = self
            .gpu_probe
            .as_ref()
            .and_then(|probe| match probe.try_recv() {
                Ok(result) => Some(result),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => Some(Err(
                    "GPU indisponible : vérification interrompue ; choisissez CPU".into(),
                )),
            });
        if let Some(result) = result {
            self.gpu_probe = None;
            match result {
                Ok(()) => {
                    if !self.compute_choice_touched {
                        self.compute_choice = ComputeChoice::Gpu;
                    }
                    self.startup_message = None;
                }
                Err(message) => {
                    if !self.compute_choice_touched {
                        self.compute_choice = ComputeChoice::Cpu;
                    }
                    self.startup_message = Some(message);
                }
            }
        }
    }

    pub(in crate::ui::model) fn poll_model_selection(&mut self) {
        while let Some(receiver) = &self.catalog_receiver {
            match receiver.try_recv() {
                Ok(catalog) => {
                    self.projection = ModelSelectionApp::new(catalog.engine_snapshot(vec![]))
                        .projection(catalog.generation)
                        .ok();
                    self.catalog = Some(catalog);
                }
                Err(TryRecvError::Disconnected) => {
                    self.selection_notice = Some("Catalogue de modèles indisponible.".into());
                    self.catalog_receiver = None;
                    break;
                }
                Err(TryRecvError::Empty) => break,
            }
        }
        let mut events = Vec::new();
        let mut preferences_disconnected = false;
        if let Some(receiver) = &self.preference_events {
            loop {
                match receiver.try_recv() {
                    Ok(event) => events.push(event),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        preferences_disconnected = true;
                        break;
                    }
                }
            }
        }
        if preferences_disconnected {
            self.preference_events = None;
        }
        for event in events {
            match event {
                ModelPreferenceEvent::Initialized {
                    model,
                    startup_compute,
                    notice,
                } => {
                    self.startup_preferences_loaded = true;
                    self.preference_load_notice = apply_preference_event(
                        &mut self.selection,
                        self.selection_touched,
                        &mut self.compute_choice,
                        &mut self.compute_choice_touched,
                        &mut self.preference_revision,
                        ModelPreferenceEvent::Initialized {
                            model,
                            startup_compute,
                            notice,
                        },
                    );
                }
                saved @ ModelPreferenceEvent::Saved { .. } => {
                    if let Some(notice) = apply_preference_event(
                        &mut self.selection,
                        self.selection_touched,
                        &mut self.compute_choice,
                        &mut self.compute_choice_touched,
                        &mut self.preference_revision,
                        saved,
                    ) {
                        self.preference_notice = Some(notice);
                    }
                }
            }
        }
    }

    pub(in crate::ui::model) fn selected_model_artifact(&self) -> Option<(String, [u8; 32])> {
        let id = self.selection.model?;
        if !self.selected_model_is_selectable() {
            return None;
        }
        let entry = self
            .catalog
            .as_ref()?
            .entries
            .iter()
            .find(|entry| entry.id == id)?;
        Some((
            entry.path.to_string_lossy().into_owned(),
            parse_sha256(&entry.expected_sha256),
        ))
    }

    pub(in crate::ui::model) fn selected_model_id(
        &self,
    ) -> Option<whisper_core::model_catalog::ModelId> {
        self.selection.model
    }

    pub(in crate::ui::model) fn selected_model_is_selectable(&self) -> bool {
        let Some(id) = self.selection.model else {
            return false;
        };
        self.projection.as_ref().is_some_and(|projection| {
            projection
                .models
                .iter()
                .any(|model| model.id == id && model.selectable)
        })
    }

    pub(in crate::ui::model) fn dispatch(&mut self, command: AppCommand) -> bool {
        let requested_compute = command_compute_choice(&command).unwrap_or(self.compute_choice);
        let gpu_selection_command = matches!(
            &command,
            AppCommand::StartImport { .. }
                | AppCommand::EnqueueImport { .. }
                | AppCommand::ProcessQueued { .. }
                | AppCommand::ResumeInterrupted { .. }
                | AppCommand::StartLive { .. }
                | AppCommand::StartLiveConfigured { .. }
                | AppCommand::StartUnifiedLiveConfigured { .. }
                | AppCommand::StartDirectLiveConfigured { .. }
        );
        if gpu_selection_command
            && let Some((job_id, generation)) = gpu_selection_identity(&command)
        {
            let candidate = if requested_compute == ComputeChoice::Cpu {
                None
            } else {
                self.gpu_inventory.as_ref().and_then(|inventory| {
                    let preferred = self.paths.gpu_preference();
                    match inventory.resolve_selection(requested_compute, preferred) {
                        SelectionResolution::Candidate(SessionCandidate::Gpu(candidate)) => {
                            inventory
                                .runtime_identities()
                                .iter()
                                .find(|proof| {
                                    proof
                                        .pnp_instance_id
                                        .eq_ignore_ascii_case(&candidate.identity)
                                })
                                .cloned()
                        }
                        _ => None,
                    }
                })
            };
            if let Err(error) = self
                .application
                .set_gpu_candidate(job_id, generation, candidate)
            {
                self.view.message = Some(format!("Sélection GPU non transmise : {error:?}"));
                return false;
            }
        }
        match self.application.dispatch(command) {
            Ok(view) => {
                self.view = view;
                true
            }
            Err(ApplicationError::Failed(message)) => {
                self.view.message = Some(message);
                false
            }
            Err(error) => {
                self.view.message = Some(format!("Commande impossible : {error:?}"));
                false
            }
        }
    }

    pub(in crate::ui::model) fn stop_active(&mut self) {
        if let Some((job_id, generation)) = self.active_identity {
            self.dispatch(AppCommand::Stop { job_id, generation });
        }
    }

    pub(in crate::ui::model) fn request_quit(&mut self) -> bool {
        self.dispatch(AppCommand::RequestClose)
    }

    pub(in crate::ui::model) fn refresh_microphones(&mut self) {
        match self.microphones.refresh() {
            Ok(()) => {
                self.microphone_notice = None;
                if (self.view.microphone_test_active || self.view.active_job.is_some())
                    && let Some(notice) = microphone_source_change_notice(
                        self.view.microphone_source_identity.as_deref(),
                        self.microphones.target_identity(),
                        self.microphones.devices(),
                    )
                {
                    self.microphone_notice = Some(notice);
                }
            }
            Err(_) => self.microphone_notice = self.microphones.take_notice(),
        }
    }

    pub(in crate::ui::model) fn prepare_frame(
        &mut self,
        ui: &mut egui::Ui,
    ) -> (bool, bool, Option<JobState>, String, bool) {
        self.poll_frame_inputs(ui);
        if let (Some((job_id, _)), Some(generation)) =
            (self.view.active_job, self.view.active_generation)
            && self
                .active_identity
                .is_some_and(|(active_job, _)| active_job == job_id)
        {
            self.active_identity = Some((job_id, generation));
        }
        let state = self.view.active_job.map(|(_, state)| state);
        if state != self.observed_job_state
            && state.is_some_and(|state| {
                matches!(
                    state,
                    JobState::Complete | JobState::Recoverable | JobState::Cancelled
                )
            })
        {
            self.scanned_destination = None;
        }
        self.observed_job_state = state;
        let unified_busy = self.view.unified_parakeet_sessions.iter().any(|session| {
            matches!(
                session.phase,
                whisper_core::ports::UnifiedSessionPhase::Starting
                    | whisper_core::ports::UnifiedSessionPhase::Capturing
                    | whisper_core::ports::UnifiedSessionPhase::Finalizing
                    | whisper_core::ports::UnifiedSessionPhase::Publishing
            )
        });
        let busy = self.view.microphone_test_active
            || unified_busy
            || (self.view.close_requested && !self.view.close_ready)
            || self
                .view
                .active_job
                .is_some_and(|(_, state)| window_must_remain_open(state));
        let can_enqueue = import_enqueue_enabled(busy);
        if ui.ctx().input(|input| input.viewport().close_requested()) && busy {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::CancelClose);
            if !self.quit_pending && self.request_quit() {
                self.quit_pending = true;
                self.view.message = Some(
                    "Fermeture en attente de l’arrêt coopératif et de la stabilisation du worker."
                        .into(),
                );
            }
        }
        let destination = self.paths.destination.trim().to_owned();
        let destination_scan_pending = !destination.is_empty()
            && self.scanned_destination.as_deref() != Some(destination.as_str());
        // Refresh consumes asynchronous scan and lifecycle events. Schedule a
        // wake for the initial destination scan too: it is submitted later in
        // this frame, while the first-frame queue view is still empty.
        if busy
            || !self.view.queue.is_empty()
            || destination_scan_pending
            || self.view.history_scan_pending
            || self.view.queue_scan_pending
        {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(33));
        }
        if !busy {
            self.active_identity = None;
            if self.quit_pending && self.view.close_ready {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        (
            busy,
            can_enqueue,
            state,
            destination,
            destination_scan_pending,
        )
    }

    fn poll_frame_inputs(&mut self, ui: &mut egui::Ui) {
        if self.view.microphone_acquisition.is_some() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(50));
        }
        self.reactivity_log.record_once(
            process_log_session(),
            "first_ui_callback",
            unix_time_ms(),
            Some(self.process_started_at_unix_ms),
        );
        self.poll_model_selection();
        if self.startup_preferences_loaded || self.compute_choice_touched {
            self.poll_gpu_probe();
        }
        if self.catalog_receiver.is_some() || self.preference_events.is_some() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(50));
        }
        if let Some(inventory) = &mut self.gpu_inventory {
            inventory.poll();
            if matches!(inventory.state(), GpuInventoryState::Loading) {
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(33));
            }
        }
        if self.gpu_probe.is_some() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(33));
        }
        if let Some(message) = self.paths.poll_notice() {
            self.path_notice = Some(message);
        }
        if self.paths.has_pending_save() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(100));
        }
        self.dispatch(AppCommand::Refresh);
        self.record_reactivity_milestones();
    }
}
pub(in crate::ui::model) fn import_enqueue_enabled(busy: bool) -> bool {
    !busy
}

pub(in crate::ui::model) fn manual_stop_available(state: JobState) -> bool {
    matches!(
        state,
        JobState::Preparing
            | JobState::Capturing
            | JobState::Running
            | JobState::Finalizing
            | JobState::AwaitingChoice
    )
}

pub(in crate::ui::model) use actions::{
    command_compute_choice, gpu_selection_identity, queued_request_for_compute,
};
pub(in crate::ui::model) use observability::{
    live_log_session, process_log_session, record_first_render, unix_time_ms,
};
