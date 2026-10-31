use super::*;

/// Renders the explicit abandoned-snapshot recovery flow. Inspection never
/// mutates a reservation; only the separate confirmation command can do so.
pub fn source_snapshot_recovery_widget(
    ui: &mut egui::Ui,
    available: bool,
    candidates: Option<&[whisper_core::ports::SourceSnapshotRecoveryCandidate]>,
) -> Option<AppCommand> {
    if available && ui.button("Vérifier les copies audio retenues").clicked() {
        return Some(AppCommand::InspectSourceSnapshotRecovery);
    }
    let candidates = candidates?;
    if candidates.is_empty() {
        return None;
    }
    let mut command = None;
    egui::Modal::new(egui::Id::new("source_snapshot_recovery_confirmation")).show(ui.ctx(), |ui| {
        ui.set_min_width(420.0);
        ui.heading("Copies audio retenues");
        ui.label("L’inspection ne libère aucune place. Les traitements actifs, en file ou ambigus restent protégés.");
        for candidate in candidates {
            ui.separator();
            ui.label(format!("Job {:032x}, génération {} — {}", candidate.job_id.0, candidate.generation.get(), candidate.destination));
            if candidate.recoverable {
                ui.colored_label(egui::Color32::LIGHT_GREEN, candidate.reason.as_deref().unwrap_or("Récupérable après confirmation"));
            } else {
                ui.colored_label(egui::Color32::YELLOW, candidate.reason.as_deref().unwrap_or("Protégée"));
            }
        }
        let can_recover = candidates.iter().any(|candidate| candidate.recoverable);
        ui.horizontal(|ui| {
            if ui.add_enabled(can_recover, egui::Button::new("Confirmer la récupération des copies sûres")).clicked() {
                command = Some(AppCommand::ConfirmSourceSnapshotRecovery);
            }
            if ui.button("Annuler").clicked() {
                command = Some(AppCommand::CancelSourceSnapshotRecovery);
            }
        });
    });
    command
}
