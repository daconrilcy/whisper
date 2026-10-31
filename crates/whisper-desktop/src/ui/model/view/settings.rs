use super::*;

impl<A: Application> DesktopApp<A> {
    pub(in crate::ui::model) fn settings_ui(&mut self, ui: &mut egui::Ui, can_enqueue: bool) {
        ui.heading("Whisper — transcription locale");
        ui.label("Import WAV/MP3 ou micro en direct, avec le modèle local.");

        ui.add_enabled_ui(can_enqueue, |ui| {
            self.path_settings_ui(ui);
            self.language_and_compute_ui(ui);
            self.selection_notices_ui(ui);
            self.gpu_preference_ui(ui);
        });
    }

    fn path_settings_ui(&mut self, ui: &mut egui::Ui) {
        let path_field_width = (ui.available_width() - 320.0).clamp(100.0, 640.0);
        let mut model_chosen = false;
        egui::Grid::new("path_fields").show(ui, |ui| {
            ui.label("Fichier audio (WAV ou MP3)");
            let response = path_text_edit(ui, &mut self.paths.audio, path_field_width);
            if ui.button("Parcourir…").clicked() {
                self.paths.browse(PathField::Audio);
            } else if response.lost_focus()
                || (response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)))
            {
                self.paths.commit_manual(PathField::Audio);
            }
            ui.end_row();

            ui.label("Modèle local");
            if let Some(projection) = &self.projection {
                model_chosen = model_selection_combo(ui, projection, &mut self.selection);
            } else {
                ui.label("Chargement des modèles…");
            }
            ui.end_row();

            ui.label("Dossier des transcriptions");
            let response = path_text_edit(ui, &mut self.paths.destination, path_field_width);
            if ui.button("Parcourir…").clicked() {
                self.paths.browse(PathField::Destination);
            } else if response.lost_focus()
                || (response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)))
            {
                self.paths.commit_manual(PathField::Destination);
            }
            ui.end_row();
        });
        if model_chosen
            && self.selection.model == Some(whisper_core::model_catalog::ModelId::ParakeetTdt06BV3)
        {
            self.selection.manual_language = None;
        }
        if model_chosen
            && let (Some(model), Some(sender)) =
                (self.selection.model, self.preference_commands.as_ref())
        {
            self.selection_touched = true;
            record_preference_delivery(
                &mut self.preference_notice,
                sender.send(ModelPreferenceCommand::ChooseModel(model)),
            );
            self.selection_notice = None;
        }
    }

    fn language_and_compute_ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label("Langue");
            if let Some(projection) = &self.projection {
                language_selection_combo(
                    ui,
                    projection,
                    compute_for_model(self.selection.model, self.compute_choice),
                    &mut self.selection,
                );
            } else {
                ui.label("Chargement…");
            }
            ui.label("Calcul");
            let parakeet_selected = self.selected_model_id()
                == Some(whisper_core::model_catalog::ModelId::ParakeetTdt06BV3);
            let previous_compute = self.compute_choice;
            let compute_interacted = self.compute_options_ui(ui, parakeet_selected);
            let compute_changed = self.compute_choice != previous_compute;
            let compute_selection_interacted = record_compute_selection_interaction(
                &mut self.compute_choice_touched,
                compute_interacted,
                compute_changed,
            );
            if compute_selection_interacted && let Some(sender) = &self.preference_commands {
                let preference = if self.compute_choice == ComputeChoice::Auto {
                    whisper_core::path_settings::StartupComputePreference::Auto
                } else {
                    whisper_core::path_settings::StartupComputePreference::FollowProbe
                };
                record_preference_delivery(
                    &mut self.preference_notice,
                    sender.send(ModelPreferenceCommand::ChooseStartupCompute(preference)),
                );
            }
        });
    }

    fn compute_options_ui(&mut self, ui: &mut egui::Ui, parakeet_selected: bool) -> bool {
        let mut compute_interacted = false;
        if parakeet_selected {
            let gpu_selectable = self.projection.as_ref().is_some_and(|projection| {
                projection
                    .models
                    .iter()
                    .find(|model| {
                        model.id == whisper_core::model_catalog::ModelId::ParakeetTdt06BV3
                    })
                    .is_some_and(|model| {
                        model.configurations.iter().any(|configuration| {
                            configuration.key.backend == whisper_core::model_catalog::Backend::Gpu
                                && configuration.selectable
                                && configuration.auto_language_selectable
                        })
                    })
            }) && self
                .catalog
                .as_ref()
                .is_some_and(parakeet_gpu_catalog_ready);
            egui::ComboBox::from_id_salt("compute_choice")
                .selected_text(match self.compute_choice {
                    ComputeChoice::Cpu => "CPU",
                    ComputeChoice::Gpu => "GPU",
                    ComputeChoice::Auto => "Auto",
                })
                .show_ui(ui, |ui| {
                    compute_interacted |= ui
                        .selectable_value(&mut self.compute_choice, ComputeChoice::Auto, "Auto")
                        .clicked();
                    compute_interacted |= ui
                        .selectable_value(&mut self.compute_choice, ComputeChoice::Cpu, "CPU")
                        .clicked();
                    let gpu_response = ui.add_enabled_ui(gpu_selectable, |ui| {
                        ui.selectable_value(&mut self.compute_choice, ComputeChoice::Gpu, "GPU")
                    });
                    compute_interacted |= gpu_response.inner.clicked();
                });
        } else {
            egui::ComboBox::from_id_salt("compute_choice")
                .selected_text(match self.compute_choice {
                    ComputeChoice::Cpu => "CPU",
                    ComputeChoice::Gpu => "GPU",
                    ComputeChoice::Auto => "Auto",
                })
                .show_ui(ui, |ui| {
                    compute_interacted |= ui
                        .selectable_value(&mut self.compute_choice, ComputeChoice::Auto, "Auto")
                        .clicked();
                    compute_interacted |= ui
                        .selectable_value(&mut self.compute_choice, ComputeChoice::Cpu, "CPU")
                        .clicked();
                    compute_interacted |= ui
                        .selectable_value(&mut self.compute_choice, ComputeChoice::Gpu, "GPU")
                        .clicked();
                });
        }
        compute_interacted
    }

    fn selection_notices_ui(&mut self, ui: &mut egui::Ui) {
        if let Some(notice) = &self.selection_notice {
            ui.colored_label(egui::Color32::YELLOW, notice);
        }
        if let Some(notice) = &self.preference_load_notice {
            ui.label(notice);
        }
        if let Some(notice) = &self.preference_notice {
            ui.label(notice);
        }
        if let Some(projection) = &self.projection
            && let Some(model) = projection
                .models
                .iter()
                .find(|model| Some(model.id) == self.selection.model)
            && let Some(cause) = model.cause
        {
            ui.label(format!("{} : indisponible ({cause:?})", model.display_name));
        }
    }

    fn gpu_preference_ui(&mut self, ui: &mut egui::Ui) {
        let gpu_preferences = self
            .gpu_inventory
            .as_ref()
            .map(|inventory| qualified_devices(inventory.qualified_state()))
            .unwrap_or_default();
        let mut preferred_gpu = self.paths.gpu_preference().map(str::to_owned);
        gpu_preference_combo(ui, &gpu_preferences, &mut preferred_gpu);
        if preferred_gpu.as_deref() != self.paths.gpu_preference() {
            self.paths.set_gpu_preference(preferred_gpu);
        }
        if let Some(inventory) = &self.gpu_inventory {
            let resolution =
                inventory.resolve_selection(self.compute_choice, self.paths.gpu_preference());
            ui.label(gpu_selection_resolution_label(
                &resolution,
                &gpu_preferences,
            ));
        }
    }
}
