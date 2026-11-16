use crate::ui::{
    ModelPreferenceCommand, ModelPreferenceEvent, PreferenceChannelError,
    catalog_inspection_channel_with_parakeet,
};

#[test]
fn turbo_composition_rejects_parakeet_projection_preference_and_inspection() {
    let (inspection, _) = catalog_inspection_channel_with_parakeet(false);
    assert_eq!(
        inspection.select(ModelId::ParakeetTdt06BV3, 1),
        Err(PreferenceChannelError::ModelUnavailable)
    );
    assert_eq!(
        inspection.command(ModelId::ParakeetTdt06BV3),
        Err(PreferenceChannelError::ModelUnavailable)
    );
    let preferences = model_preference_command_channel()
        .0
        .with_inspection_requests(inspection)
        .with_parakeet_enabled(false);
    assert_eq!(
        preferences.send(ModelPreferenceCommand::ChooseModel(
            ModelId::ParakeetTdt06BV3
        )),
        Err(PreferenceChannelError::ModelUnavailable)
    );

    let (event_sender, event_receiver) = std::sync::mpsc::sync_channel(1);
    let (catalog_sender, catalog_receiver) = std::sync::mpsc::channel();
    let mut app =
        DesktopApp::new(RecordingApplication::default()).with_parakeet_edition_enabled(false);
    app.startup_preferences_loaded = false;
    app.preference_events = Some(event_receiver);
    app.catalog_receiver = Some(catalog_receiver);
    event_sender
        .send(ModelPreferenceEvent::Initialized {
            model: Some(ModelId::ParakeetTdt06BV3),
            startup_compute: whisper_core::path_settings::StartupComputePreference::FollowProbe,
            notice: None,
        })
        .unwrap();
    let record = whisper_core::model_registry::model(ModelId::ParakeetTdt06BV3);
    catalog_sender
        .send(whisper_adapters::model_catalog::CatalogSnapshot {
            generation: 1,
            entries: vec![whisper_adapters::model_catalog::CatalogEntry {
                id: ModelId::ParakeetTdt06BV3,
                display_name: record.display_name.to_owned(),
                path: record.artifact.file_name.into(),
                availability: whisper_core::model_catalog::ModelAvailability::Uninspected,
                expected_size_bytes: record.artifact.size_bytes,
                observed_size_bytes: None,
                expected_sha256: record.artifact.sha256.to_owned(),
                observed_sha256: None,
                diagnostic: Some("Modèle non vérifié.".into()),
            }],
        })
        .unwrap();
    app.poll_model_selection();

    assert_eq!(app.selection.model, Some(ModelId::WhisperLargeV3Turbo));
    assert!(app.projection.as_ref().is_some_and(|projection| {
        projection
            .models
            .iter()
            .all(|model| model.id != ModelId::ParakeetTdt06BV3)
    }));
}

#[test]
fn generic_composition_keeps_parakeet_selectable_and_inspectable() {
    let (inspection, _receiver) = catalog_inspection_channel_with_parakeet(true);
    assert_eq!(inspection.select(ModelId::ParakeetTdt06BV3, 1), Ok(()));
    assert!(inspection.command(ModelId::ParakeetTdt06BV3).is_ok());
}

#[test]
fn turbo_desktop_constructor_starts_only_the_explicit_turbo_log() {
    let root = std::env::temp_dir().join(format!(
        "desktop-edition-log-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let turbo_log = root.join("WhisperTurbo/logs/reactivity.log");
    let personal_log = root.join("Whisper/reactivity.log");
    std::fs::create_dir_all(personal_log.parent().unwrap()).unwrap();
    std::fs::write(&personal_log, "personal sentinel\n").unwrap();
    let paths = whisper_core::path_settings::PathSettingsApp::new(
        whisper_core::path_settings::PathDefaults {
            audio_directory: "audio".into(),
            model_path: "model".into(),
            destination: "output".into(),
        },
        Box::new(whisper_core::path_settings::NullPathPort),
    );
    let mut app = DesktopApp::with_turbo_microphones(
        RecordingApplication::default(),
        whisper_core::ComputeChoice::Cpu,
        None,
        paths,
        Box::new(OneMicrophone),
        Some(turbo_log.clone()),
    );
    app.reactivity_log
        .record_once("startup", "turbo_constructor", 10, None);
    drop(app);

    assert!(
        std::fs::read_to_string(turbo_log)
            .unwrap()
            .contains("event=turbo_constructor")
    );
    assert_eq!(
        std::fs::read_to_string(&personal_log).unwrap(),
        "personal sentinel\n"
    );
    std::fs::remove_dir_all(root).unwrap();
}
