use rfd::{FileDialog, MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};
use std::path::PathBuf;
use std::process::Command;

const OPENGL_COMPATIBILITY_PACK_STORE_URI: &str = "ms-windows-store://pdp/?ProductId=9NQPSL29BFFF";

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

/// Offers the Store repair path only for the known Glow/OpenGL 2.0 startup failure.
/// Returns false for unrelated renderer errors so the caller can preserve its usual error path.
pub fn show_msix_glow_startup_error(diagnostic: &str) -> bool {
    if !is_missing_glow_compatibility_error(diagnostic) {
        return false;
    }

    let message = format!(
        "L’affichage OpenGL de WhisperTurbo n’a pas pu démarrer. Sur Snapdragon avec Windows ARM64, le pack de compatibilité OpenCL, OpenGL et Vulkan de Microsoft peut être nécessaire.\n\nInstallez le pack depuis le Microsoft Store, puis relancez WhisperTurbo.\n\nDétail technique : {diagnostic}\n\nOui : ouvrir la fiche du pack dans le Store.\nNon ou fermer : quitter WhisperTurbo."
    );
    let choice = MessageDialog::new()
        .set_level(MessageLevel::Error)
        .set_title("Prérequis graphique manquant")
        .set_description(message)
        .set_buttons(MessageButtons::YesNo)
        .show();

    if choice == MessageDialogResult::Yes {
        if let Err(error) = Command::new("explorer.exe")
            .arg(OPENGL_COMPATIBILITY_PACK_STORE_URI)
            .spawn()
        {
            show_error(
                "Ouverture du Microsoft Store impossible",
                &format!(
                    "La fiche du pack n’a pas pu être ouverte. Ouvrez Microsoft Store et recherchez le pack de compatibilité OpenCL, OpenGL et Vulkan.\n\nDétail : {error}"
                ),
            );
        }
    }

    true
}

fn is_missing_glow_compatibility_error(diagnostic: &str) -> bool {
    diagnostic
        .to_ascii_lowercase()
        .contains("egui_glow requires opengl 2.0")
}

#[cfg(test)]
#[path = "installer_ui/tests.rs"]
mod tests;
