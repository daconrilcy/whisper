use super::*;

pub const APPROVED_MODEL_SHA256: &str =
    "1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69";
pub fn path_save_status_label(status: &SaveStatus) -> String {
    match status {
        SaveStatus::NotSaved => String::new(),
        SaveStatus::Pending { revision } => {
            format!("Sauvegarde des réglages en cours (révision {revision}).")
        }
        SaveStatus::Saved { revision } => {
            format!("Réglages enregistrés (révision {revision}).")
        }
        SaveStatus::Failed { revision, error } => {
            format!("Réglages non mémorisés (révision {revision}) : {error}.")
        }
    }
}

pub(in crate::ui) fn path_text_edit(
    ui: &mut egui::Ui,
    path: &mut String,
    width: f32,
) -> egui::Response {
    ui.scope(|ui| {
        let border = egui::Stroke::new(1.0, ui.visuals().weak_text_color());
        let widgets = &mut ui.style_mut().visuals.widgets;
        widgets.inactive.bg_stroke = border;
        widgets.hovered.bg_stroke = border;
        widgets.active.bg_stroke = border;
        ui.add_sized(
            [width, ui.spacing().interact_size.y],
            egui::TextEdit::singleline(path),
        )
    })
    .inner
}

pub(in crate::ui) fn path_is_file(path: &str) -> bool {
    !path.is_empty() && std::path::Path::new(path).is_file()
}

pub(in crate::ui) fn path_if_file(path: &str) -> Option<&str> {
    path_is_file(path).then_some(path)
}

pub(in crate::ui) fn copyable_path(ui: &mut egui::Ui, label: &str, path: &str) {
    ui.horizontal(|ui| {
        ui.small(format!("{label} : {path}"));
        if ui.small_button("Copier le chemin").clicked() {
            ui.ctx().copy_text(path.to_owned());
        }
    });
}

pub fn default_model_path() -> String {
    std::env::var("LOCALAPPDATA")
        .map(|path| format!(r"{path}\Programs\WhisperBuildDeps\models\ggml-large-v3-turbo.bin"))
        .unwrap_or_default()
}

pub fn default_audio_directory() -> String {
    std::env::var("USERPROFILE")
        .map(|path| format!(r"{path}\Documents"))
        .unwrap_or_default()
}

pub fn default_destination() -> String {
    std::env::var("WHISPER_L04_NORMAL_RUN_ARCHIVE")
        .ok()
        .filter(|path| !path.trim().is_empty())
        .or_else(|| {
            std::env::var("USERPROFILE")
                .ok()
                .map(|path| format!(r"{path}\Documents\WhisperTranscriptions"))
        })
        .unwrap_or_default()
}

pub fn approved_model_sha256() -> [u8; 32] {
    parse_sha256(APPROVED_MODEL_SHA256)
}

pub(in crate::ui) fn parse_sha256(value: &str) -> [u8; 32] {
    let mut bytes = [0_u8; 32];
    for (index, pair) in value.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        bytes[index] = (hex_digit(pair[0]) << 4) | hex_digit(pair[1]);
    }
    bytes
}
pub(in crate::ui) fn hex_digit(value: u8) -> u8 {
    match value {
        b'0'..=b'9' => value - b'0',
        b'a'..=b'f' => value - b'a' + 10,
        _ => 0,
    }
}
