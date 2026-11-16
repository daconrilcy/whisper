use super::super::{CatalogEntry, CatalogSnapshot};
use whisper_core::model_catalog::{ModelAvailability, ModelId};
use whisper_core::model_registry::{MODELS, model};

fn uninspected_entry(id: ModelId) -> CatalogEntry {
    let record = model(id);
    CatalogEntry {
        id,
        display_name: record.display_name.to_owned(),
        path: record.artifact.file_name.into(),
        availability: ModelAvailability::Uninspected,
        expected_size_bytes: record.artifact.size_bytes,
        observed_size_bytes: None,
        expected_sha256: record.artifact.sha256.to_owned(),
        observed_sha256: None,
        diagnostic: Some("Modèle non vérifié.".into()),
    }
}

#[test]
fn engine_snapshot_projection_contains_only_catalogued_models_and_capabilities() {
    let turbo = CatalogSnapshot {
        generation: 4,
        entries: vec![uninspected_entry(ModelId::WhisperLargeV3Turbo)],
    };
    let snapshot = turbo.engine_snapshot_for_edition(Vec::new(), false);
    let projection = whisper_core::model_selection_app::ModelSelectionApp::new(snapshot.clone())
        .projection(4)
        .expect("generation matched");

    assert_eq!(snapshot.models.len(), 1);
    assert_eq!(snapshot.models[0].id, ModelId::WhisperLargeV3Turbo);
    assert!(
        snapshot
            .capabilities
            .iter()
            .all(|item| item.key.model == ModelId::WhisperLargeV3Turbo)
    );
    assert_eq!(projection.models.len(), 1);
    assert_eq!(projection.models[0].id, ModelId::WhisperLargeV3Turbo);

    let absent_turbo = CatalogSnapshot {
        generation: 5,
        entries: Vec::new(),
    }
    .engine_snapshot_for_edition(Vec::new(), false);
    assert_eq!(absent_turbo.models.len(), 1);
    assert_eq!(absent_turbo.models[0].id, ModelId::WhisperLargeV3Turbo);
}

#[test]
fn generic_engine_snapshot_retains_every_registered_model() {
    let generic = CatalogSnapshot {
        generation: 4,
        entries: MODELS
            .iter()
            .map(|record| uninspected_entry(record.artifact.id))
            .collect(),
    };
    let projection = whisper_core::model_selection_app::ModelSelectionApp::new(
        generic.engine_snapshot(Vec::new()),
    )
    .projection(4)
    .expect("generation matched");

    assert_eq!(projection.models.len(), MODELS.len());
    assert!(
        projection
            .models
            .iter()
            .any(|item| item.id == ModelId::ParakeetTdt06BV3)
    );
}
