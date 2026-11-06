mod catalog_inspection {
    use super::*;
    use crate::ui::{ModelPreferenceCommand, ModelPreferenceEvent, catalog_inspection_channel};
    use whisper_core::{model_catalog::ModelAvailability, path_settings::StartupComputePreference};

    fn snapshot(generation: u64, a: ModelAvailability, b: ModelAvailability) -> CatalogSnapshot {
        CatalogSnapshot {
            generation,
            entries: whisper_core::model_registry::MODELS
                .iter()
                .map(|record| {
                    let availability = if record.artifact.id == ModelId::WhisperLargeV3Turbo {
                        a
                    } else {
                        b
                    };
                    whisper_adapters::model_catalog::CatalogEntry {
                        id: record.artifact.id,
                        display_name: record.display_name.into(),
                        path: std::path::PathBuf::from("synthetic-catalog")
                            .join(record.artifact.file_name),
                        availability,
                        expected_size_bytes: record.artifact.size_bytes,
                        observed_size_bytes: (availability == ModelAvailability::Ready)
                            .then_some(record.artifact.size_bytes),
                        expected_sha256: record.artifact.sha256.into(),
                        observed_sha256: (availability == ModelAvailability::Ready)
                            .then(|| record.artifact.sha256.into()),
                        diagnostic: (availability != ModelAvailability::Ready)
                            .then(|| format!("synthetic {availability:?}")),
                    }
                })
                .collect(),
        }
    }

    #[test]
    fn controller_waits_for_effective_startup_preference_before_requesting_any_model() {
        let (requester, receiver) = catalog_inspection_channel();
        let (catalog_tx, catalog_rx) = std::sync::mpsc::channel();
        let (event_tx, event_rx) = std::sync::mpsc::channel();
        let (preferences, _preference_receiver) = model_preference_command_channel();
        let mut app = DesktopApp::new(RecordingApplication::default())
            .with_model_selection_runtime(ModelSelectionRuntime {
                catalog: catalog_rx,
                preference_events: event_rx,
                preference_commands: preferences.with_inspection_requests(requester.clone()),
            });
        assert!(receiver.publish(None, 1, [0, 0]).unwrap());
        catalog_tx
            .send(snapshot(
                1,
                ModelAvailability::Uninspected,
                ModelAvailability::Uninspected,
            ))
            .unwrap();
        app.poll_model_selection();
        assert!(receiver.next(Duration::ZERO).unwrap().is_none());
        assert!(app.selected_model_artifact().is_none());
        event_tx
            .send(ModelPreferenceEvent::Initialized {
                model: Some(ModelId::ParakeetTdt06BV3),
                startup_compute: StartupComputePreference::FollowProbe,
                notice: None,
            })
            .unwrap();
        app.poll_model_selection();
        let need = receiver.next(Duration::ZERO).unwrap().unwrap();
        assert_eq!(need.model, ModelId::ParakeetTdt06BV3);
        assert!(receiver.next(Duration::ZERO).unwrap().is_none());
        assert_eq!(app.selection.model, Some(ModelId::ParakeetTdt06BV3));
        assert!(app.selected_model_artifact().is_none());
        assert!(receiver.publish(Some(need), 2, [0, need.version]).unwrap());
        catalog_tx
            .send(snapshot(
                2,
                ModelAvailability::Uninspected,
                ModelAvailability::Ready,
            ))
            .unwrap();
        app.poll_model_selection();
        assert!(app.selected_model_artifact().is_some());
        assert_eq!(
            app.catalog.as_ref().unwrap().entries[0].availability,
            ModelAvailability::Uninspected
        );
    }

    #[test]
    fn newer_user_choice_survives_late_startup_preference_and_is_requested_once() {
        let (requester, receiver) = catalog_inspection_channel();
        let (_catalog_tx, catalog_rx) = std::sync::mpsc::channel();
        let (event_tx, event_rx) = std::sync::mpsc::channel();
        let (preferences, _preference_receiver) = model_preference_command_channel();
        let preferences = preferences.with_inspection_requests(requester.clone());
        let mut app = DesktopApp::new(RecordingApplication::default())
            .with_model_selection_runtime(ModelSelectionRuntime {
                catalog: catalog_rx,
                preference_events: event_rx,
                preference_commands: preferences.clone(),
            });
        app.selection.model = Some(ModelId::WhisperLargeV3Turbo);
        app.selection_touched = true;
        preferences
            .send(ModelPreferenceCommand::ChooseModel(
                ModelId::WhisperLargeV3Turbo,
            ))
            .unwrap();
        event_tx
            .send(ModelPreferenceEvent::Initialized {
                model: Some(ModelId::ParakeetTdt06BV3),
                startup_compute: StartupComputePreference::FollowProbe,
                notice: None,
            })
            .unwrap();
        app.poll_model_selection();
        assert_eq!(app.selection.model, Some(ModelId::WhisperLargeV3Turbo));
        assert_eq!(
            receiver.next(Duration::ZERO).unwrap().unwrap().model,
            ModelId::WhisperLargeV3Turbo
        );
        assert!(receiver.next(Duration::ZERO).unwrap().is_none());
    }

    #[test]
    fn command_model_observation_merges_without_changing_selection_or_dispatching() {
        let recorder = RecordingApplication::default();
        let (requester, receiver) = catalog_inspection_channel();
        let (catalog_tx, catalog_rx) = std::sync::mpsc::channel();
        let (event_tx, event_rx) = std::sync::mpsc::channel();
        let (preferences, _preference_receiver) = model_preference_command_channel();
        let mut app =
            DesktopApp::new(recorder.clone()).with_model_selection_runtime(ModelSelectionRuntime {
                catalog: catalog_rx,
                preference_events: event_rx,
                preference_commands: preferences.with_inspection_requests(requester.clone()),
            });
        event_tx
            .send(ModelPreferenceEvent::Initialized {
                model: Some(ModelId::WhisperLargeV3Turbo),
                startup_compute: StartupComputePreference::FollowProbe,
                notice: None,
            })
            .unwrap();
        app.poll_model_selection();
        let a = receiver.next(Duration::ZERO).unwrap().unwrap();
        assert!(receiver.publish(Some(a), 2, [a.version, 0]).unwrap());
        catalog_tx
            .send(snapshot(
                2,
                ModelAvailability::Ready,
                ModelAvailability::Uninspected,
            ))
            .unwrap();
        app.poll_model_selection();
        requester.command(ModelId::ParakeetTdt06BV3).unwrap();
        let b = receiver.next(Duration::ZERO).unwrap().unwrap();
        assert!(
            receiver
                .publish(Some(b), 3, [a.version, b.version])
                .unwrap()
        );
        catalog_tx
            .send(snapshot(
                3,
                ModelAvailability::Ready,
                ModelAvailability::Ready,
            ))
            .unwrap();
        catalog_tx
            .send(snapshot(
                2,
                ModelAvailability::Missing,
                ModelAvailability::Missing,
            ))
            .unwrap();
        app.poll_model_selection();
        assert_eq!(app.selection.model, Some(ModelId::WhisperLargeV3Turbo));
        assert_eq!(app.catalog.as_ref().unwrap().generation, 3);
        assert_eq!(
            app.catalog.as_ref().unwrap().entries[1].availability,
            ModelAvailability::Ready
        );
        assert!(recorder.0.borrow().is_empty());
    }

    #[test]
    fn controller_does_not_offer_an_executable_artifact_for_missing_or_corrupt_observation() {
        for availability in [ModelAvailability::Missing, ModelAvailability::Corrupt] {
            let (requester, receiver) = catalog_inspection_channel();
            let (catalog_tx, catalog_rx) = std::sync::mpsc::channel();
            let (event_tx, event_rx) = std::sync::mpsc::channel();
            let (preferences, _preference_receiver) = model_preference_command_channel();
            let mut app = DesktopApp::new(RecordingApplication::default())
                .with_model_selection_runtime(ModelSelectionRuntime {
                    catalog: catalog_rx,
                    preference_events: event_rx,
                    preference_commands: preferences.with_inspection_requests(requester.clone()),
                });
            event_tx
                .send(ModelPreferenceEvent::Initialized {
                    model: Some(ModelId::ParakeetTdt06BV3),
                    startup_compute: StartupComputePreference::FollowProbe,
                    notice: None,
                })
                .unwrap();
            app.poll_model_selection();
            let need = receiver.next(Duration::ZERO).unwrap().unwrap();
            assert!(receiver.publish(Some(need), 2, [0, need.version]).unwrap());
            catalog_tx
                .send(snapshot(2, ModelAvailability::Uninspected, availability))
                .unwrap();
            app.poll_model_selection();
            assert_eq!(app.selection.model, Some(ModelId::ParakeetTdt06BV3));
            assert!(app.selected_model_artifact().is_none());
            assert!(
                app.selection_notice
                    .as_deref()
                    .unwrap()
                    .contains("synthetic")
            );
            assert!(receiver.next(Duration::ZERO).unwrap().is_none());
        }
    }
}
