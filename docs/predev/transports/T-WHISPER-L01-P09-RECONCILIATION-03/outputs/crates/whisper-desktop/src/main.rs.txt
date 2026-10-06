use whisper_core::{AppCommand, AppView, Application, ApplicationError};

struct DesktopComposition;

impl Application for DesktopComposition {
    fn dispatch(&mut self, command: AppCommand) -> Result<AppView, ApplicationError> {
        match command {
            AppCommand::Refresh => Ok(self.view()),
            _ => Err(ApplicationError::InvalidCommand),
        }
    }

    fn view(&self) -> AppView {
        AppView::default()
    }
}

fn main() -> eframe::Result {
    whisper_desktop::run(DesktopComposition)
}
