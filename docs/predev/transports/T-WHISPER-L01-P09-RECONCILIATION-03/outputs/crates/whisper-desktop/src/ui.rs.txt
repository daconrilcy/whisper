use eframe::egui;
use whisper_core::{AppCommand, AppFacade, AppView, Application, ApplicationError};

pub struct DesktopApp<A> {
    application: AppFacade<A>,
    view: AppView,
}

impl<A: Application> DesktopApp<A> {
    pub fn new(application: A) -> Self {
        let application = AppFacade::new(application);
        let view = application.view();
        Self { application, view }
    }
}

impl<A: Application> eframe::App for DesktopApp<A> {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        ui.heading("Whisper");
        ui.label("Socle L00 — aucun traitement audio n’est encore connecté.");
        if ui.button("Actualiser").clicked() {
            match self.application.dispatch(AppCommand::Refresh) {
                Ok(view) => self.view = view,
                Err(ApplicationError::Failed(message)) => self.view.message = Some(message),
                Err(error) => self.view.message = Some(format!("{error:?}")),
            }
        }
        if let Some((_, state)) = self.view.active_job {
            ui.label(format!("État : {state:?}"));
        }
        if let Some(message) = &self.view.message {
            ui.label(message);
        }
    }
}
