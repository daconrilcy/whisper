use super::*;

impl<A: Application> DesktopApp<A> {
    pub(in crate::ui::model) fn published_result(&self) -> Option<&str> {
        self.view
            .active_job
            .filter(|(_, state)| *state == JobState::Complete)
            .and(self.view.last_result.as_deref())
    }

    pub(in crate::ui::model) fn notices_ui(&mut self, ui: &mut egui::Ui) {
        if let Some(message) = &self.view.message {
            let visible_message = if message.starts_with("ModelMissing:") {
                "Modèle local introuvable. Vérifiez le chemin dans « Modèle local ». Pour cet import déjà en file, choisissez « Retirer », puis ajoutez de nouveau le fichier audio."
            } else {
                message
            };
            let color = if visible_message.contains("CPU prend le relais") {
                egui::Color32::YELLOW
            } else {
                egui::Color32::LIGHT_RED
            };
            ui.colored_label(color, visible_message);
        }
        if let Some(message) = &self.startup_message {
            ui.colored_label(egui::Color32::LIGHT_RED, message);
        }
        if let Some(inventory) = &self.gpu_inventory {
            gpu_inventory_status(ui, inventory.state());
            gpu_qualified_status(ui, inventory.qualified_state());
        }
        if let Some(message) = &self.path_notice {
            ui.colored_label(egui::Color32::YELLOW, message);
        }
        let save_status = path_save_status_label(self.paths.save_status());
        if !save_status.is_empty() {
            ui.label(save_status);
        }
        if let Some(message) = &self.microphone_notice {
            ui.colored_label(egui::Color32::YELLOW, message);
        }
        if self.view.diagnostics_degraded {
            ui.colored_label(
                egui::Color32::LIGHT_RED,
                "Les diagnostics locaux sont indisponibles ; l’opération continue.",
            );
        }
        if let Some(result) = self.published_result() {
            ui.colored_label(egui::Color32::LIGHT_GREEN, result);
        }
    }
}
