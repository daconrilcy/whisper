use rfd::{FileDialog, MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};
use std::path::PathBuf;

pub fn choose_turbo_model() -> Option<PathBuf> {
    FileDialog::new()
        .add_filter("Modèle Whisper Turbo", &["bin"])
        .pick_file()
}

pub fn confirm_install() -> bool {
    MessageDialog::new()
        .set_level(MessageLevel::Info)
        .set_title("Prérequis satisfaits")
        .set_description(
            "Le CPU x86-64-v2, le paquet et le modèle ont été vérifiés.\n\nOui : installer WhisperTurbo dans %LOCALAPPDATA%.\nNon ou fermer : annuler sans modifier une installation existante.",
        )
        .set_buttons(MessageButtons::YesNo)
        .show()
        == MessageDialogResult::Yes
}

pub fn show_information(title: &str, message: &str) {
    MessageDialog::new()
        .set_level(MessageLevel::Info)
        .set_title(title)
        .set_description(message)
        .set_buttons(MessageButtons::Ok)
        .show();
}

pub fn show_error(title: &str, message: &str) {
    MessageDialog::new()
        .set_level(MessageLevel::Error)
        .set_title(title)
        .set_description(message)
        .set_buttons(MessageButtons::Ok)
        .show();
}
