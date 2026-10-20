use eframe::egui::{self, Shape};
use whisper_core::{Generation, JobId, ports::SourceSnapshotRecoveryCandidate};
use whisper_desktop::ui::source_snapshot_recovery_widget;

#[test]
fn recovery_ui_displays_protected_and_recoverable_entries_without_implicit_recovery() {
    let candidates = [
        SourceSnapshotRecoveryCandidate {
            job_id: JobId(1),
            generation: Generation::first(),
            destination: "C:/isolated/transcripts".into(),
            recoverable: true,
            reason: Some("Copie et fiche vérifiées".into()),
        },
        SourceSnapshotRecoveryCandidate {
            job_id: JobId(2),
            generation: Generation::first(),
            destination: "C:/isolated/transcripts".into(),
            recoverable: false,
            reason: Some("Traitement encore présent dans la file durable".into()),
        },
    ];
    let context = egui::Context::default();
    let mut command = None;
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::pos2(0.0, 0.0),
            egui::vec2(900.0, 900.0),
        )),
        ..Default::default()
    };
    let _ = context.run_ui(input.clone(), |ui| {
        command = source_snapshot_recovery_widget(ui, true, Some(&candidates));
    });
    let output = context.run_ui(input, |ui| {
        command = source_snapshot_recovery_widget(ui, true, Some(&candidates));
    });
    let rendered = output
        .shapes
        .into_iter()
        .filter_map(|clipped| match clipped.shape {
            Shape::Text(text) => Some(text.galley.job.text.to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");

    assert!(rendered.contains("Vérifier les copies audio retenues"));
    assert!(
        rendered.contains("Confirmer la récupération des copies sûres"),
        "rendered text: {rendered:?}"
    );
    assert!(rendered.contains("Copie et fiche vérifiées"));
    assert!(rendered.contains("Traitement encore présent dans la file durable"));
    assert_eq!(
        command, None,
        "displaying the inspection must not free a reservation"
    );
}

#[test]
fn capacity_action_is_available_only_when_the_application_offers_recovery() {
    let context = egui::Context::default();
    let mut unavailable_command = None;
    let unavailable = context.run_ui(Default::default(), |ui| {
        unavailable_command = source_snapshot_recovery_widget(ui, false, None);
    });
    assert!(unavailable.shapes.is_empty());
    assert_eq!(unavailable_command, None);

    let mut available_command = None;
    let available = context.run_ui(Default::default(), |ui| {
        available_command = source_snapshot_recovery_widget(ui, true, None);
    });
    assert!(!available.shapes.is_empty());
    assert_eq!(
        available_command, None,
        "the user must activate the inspection button"
    );
}
