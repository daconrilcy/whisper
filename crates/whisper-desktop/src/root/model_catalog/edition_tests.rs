use super::super::PreferenceChannelError;
use super::super::uninspected_snapshot_for_edition;
use super::{Root, WAIT, start_catalog_service_with};
use std::path::Path;
use whisper_core::{model_catalog::ModelId, model_registry};

#[test]
fn turbo_service_publishes_only_turbo_after_real_inspection() {
    let root = Root::new();
    let (requests, catalog, admission) =
        start_catalog_service_with(root.0.clone(), true, |catalog, id, generation| {
            catalog.inspect_model(id, generation)
        });
    let initial_catalog = catalog.recv_timeout(WAIT).unwrap();
    let initial_admission = admission.recv_timeout(WAIT).unwrap();
    for snapshot in [&initial_catalog, &initial_admission] {
        assert_eq!(snapshot.entries.len(), 1);
        assert_eq!(snapshot.entries[0].id, ModelId::WhisperLargeV3Turbo);
    }
    assert_eq!(
        requests.command(ModelId::ParakeetTdt06BV3),
        Err(PreferenceChannelError::ModelUnavailable)
    );
    requests.select(ModelId::WhisperLargeV3Turbo, 1).unwrap();
    let inspected_catalog = catalog.recv_timeout(WAIT).unwrap();
    let inspected_admission = admission.recv_timeout(WAIT).unwrap();
    for snapshot in [&inspected_catalog, &inspected_admission] {
        assert!(snapshot.generation > initial_catalog.generation);
        assert_eq!(snapshot.entries.len(), 1);
        assert_eq!(snapshot.entries[0].id, ModelId::WhisperLargeV3Turbo);
    }
    requests.close();
}

#[test]
fn turbo_edition_catalog_contains_only_large_v3_turbo() {
    let snapshot = uninspected_snapshot_for_edition(Path::new("models"), 1, true);

    assert_eq!(snapshot.entries.len(), 1);
    assert_eq!(snapshot.entries[0].id, ModelId::WhisperLargeV3Turbo);
    assert!(
        !snapshot
            .entries
            .iter()
            .any(|entry| entry.id == ModelId::ParakeetTdt06BV3)
    );
}

#[test]
fn generic_catalog_keeps_both_models() {
    let snapshot = uninspected_snapshot_for_edition(Path::new("models"), 1, false);

    assert_eq!(snapshot.entries.len(), model_registry::MODELS.len());
    assert!(
        snapshot
            .entries
            .iter()
            .any(|entry| entry.id == ModelId::ParakeetTdt06BV3)
    );
}

#[test]
fn turbo_install_root_is_below_local_app_data() {
    let root = super::super::super::distribution_edition::turbo_install_root(Some(
        "C:/Users/test/AppData/Local".into(),
    ))
    .expect("configured LocalAppData");

    assert_eq!(root, Path::new("C:/Users/test/AppData/Local/WhisperTurbo"));
    assert_eq!(
        super::super::super::distribution_edition::settings_path(&root),
        root.join("settings/settings.json")
    );
    assert_eq!(
        super::super::super::distribution_edition::log_paths(&root),
        (
            root.join("logs/reactivity.log"),
            root.join("logs/gpu-inventory.log")
        )
    );
}

#[test]
fn turbo_edition_ignores_but_does_not_rewrite_parakeet_preference() {
    let preferred = Some(ModelId::ParakeetTdt06BV3);

    assert_eq!(
        super::super::super::distribution_edition::preferred_model_for_edition(preferred, true),
        Some(ModelId::WhisperLargeV3Turbo)
    );
    assert!(
        super::super::super::distribution_edition::ignores_parakeet_preference_for_edition(
            preferred, true
        )
    );
    assert_eq!(
        super::super::super::distribution_edition::preferred_model_for_edition(preferred, false),
        preferred
    );
}
