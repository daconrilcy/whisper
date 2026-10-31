use super::*;
mod actions;
mod active_job;
mod capture;
mod dialogs;
mod history;
mod microphone;
mod notices;
mod queue;
mod settings;
mod unified;

impl<A: Application> eframe::App for DesktopApp<A> {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if self.view.microphone_test_active {
            self.stop_microphone_test();
        }
        self.paths.commit_manual(PathField::Audio);
        self.paths.commit_manual(PathField::Destination);
        if let Some(inventory) = &mut self.gpu_inventory {
            inventory.close();
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.show_ui(ui);
    }
}

impl<A: Application> DesktopApp<A> {
    pub(in crate::ui::model) fn show_ui(&mut self, ui: &mut egui::Ui) {
        let (busy, can_enqueue, state, destination, destination_scan_pending) =
            self.prepare_frame(ui);
        egui::ScrollArea::vertical()
            .id_salt("desktop_main_page")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                self.settings_ui(ui, can_enqueue);
                self.microphone_settings_ui(ui, busy);
                self.start_actions_ui(ui, busy, can_enqueue);
                self.dialogs_ui(ui);
                self.stop_action_ui(ui, state);
                self.active_job_ui(ui);
                self.render_unified_parakeet_ui(ui);
                self.render_parakeet_capture_ui(ui);
                self.notices_ui(ui);
                self.queue_ui(ui, busy);
                self.history_ui(ui, busy, destination, destination_scan_pending);
            });
    }
}
