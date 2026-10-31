use super::*;

impl<A: Application> DesktopApp<A> {
    pub(in crate::ui::model) fn start_actions_ui(
        &mut self,
        ui: &mut egui::Ui,
        busy: bool,
        can_enqueue: bool,
    ) {
        ui.add_enabled_ui(can_enqueue, |ui| {
            if !IMPORT_CPU_MVP {
                ui.add_enabled_ui(!busy, |ui| {
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.auto_stop_enabled, "Arrêt après durée de parole");
                        if self.auto_stop_enabled {
                            ui.add(
                                egui::DragValue::new(&mut self.auto_stop_minutes)
                                    .range(1..=u64::MAX)
                                    .suffix(" min"),
                            );
                        } else {
                            ui.label("sans limite");
                        }
                    });
                });
            }
            let can_start = self.selected_model_artifact().is_some()
                && !self.paths.audio.trim().is_empty()
                && !self.paths.destination.trim().is_empty()
                && self
                    .selection
                    .manual_language
                    .as_ref()
                    .is_none_or(|language| !language.is_empty());
            if ui
                .add_enabled(can_start, egui::Button::new("Importer et transcrire"))
                .clicked()
            {
                self.start_import();
            }
        });

        let unified_candidate_selected = self.unified_candidate_enabled
            && self.selection.model == Some(ModelId::ParakeetTdt06BV3);
        let can_start_live = (if unified_candidate_selected {
            unified_live_admission_available(&self.view)
        } else {
            !busy || self.view.microphone_test_active
        }) && model_supports_live(self.selection.model)
            && (self.selection.model != Some(ModelId::ParakeetTdt06BV3)
                || self.selection.manual_language.is_none())
            && self.selected_model_artifact().is_some()
            && !self.paths.destination.trim().is_empty()
            && self
                .selection
                .manual_language
                .as_ref()
                .is_none_or(|language| !language.is_empty());
        let response = ui.add_enabled(can_start_live, egui::Button::new("Démarrer le micro"));
        #[cfg(test)]
        {
            self.live_start_button_rect = Some(response.rect);
            self.live_start_button_enabled = Some(response.enabled());
        }
        if response.clicked() {
            self.start_live(Generation::first());
        }
        if !model_supports_live(self.selection.model) {
            ui.label("Le microphone n'est pas qualifié pour ce modèle.");
        } else if self.selection.model == Some(ModelId::ParakeetTdt06BV3)
            && self.selection.manual_language.is_some()
        {
            ui.label("Le microphone Parakeet requiert la détection automatique de la langue.");
        }
    }
    pub(in crate::ui::model) fn stop_action_ui(
        &mut self,
        ui: &mut egui::Ui,
        state: Option<JobState>,
    ) {
        let micro_active = self.view.active_job.is_some_and(|(job_id, _)| {
            self.live_request
                .as_ref()
                .is_some_and(|request| request.job_id == job_id)
        });
        if micro_active
            && state.is_some_and(|state| matches!(state, JobState::Draining | JobState::Finalizing))
        {
            ui.add_enabled(false, egui::Button::new("Arrêt en cours"));
        } else if state.is_some_and(manual_stop_available) && self.active_identity.is_some() {
            let label = if micro_active {
                "Arrêter le micro"
            } else {
                "Arrêter"
            };
            let stop_button = ui.button(label);
            #[cfg(test)]
            {
                self.live_stop_button_rect = Some(stop_button.rect);
            }
            if stop_button.clicked() {
                self.stop_active();
            }
        }
    }
}
