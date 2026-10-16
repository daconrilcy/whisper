use whisper_core::{
    model_catalog::ModelId,
    path_settings::{
        LegacyModelPath, ModelPreferenceAcknowledgement, ModelPreferencePort,
        ModelPreferenceSaveStatus, ModelPreferenceSnapshot, ModelPreferencesApp,
        StartupComputePreference,
    },
};

struct MemoryPreferences {
    snapshot: Option<ModelPreferenceSnapshot>,
    fail: bool,
    wrong_ack: bool,
}

impl ModelPreferencePort for MemoryPreferences {
    fn load(&self) -> Result<Option<ModelPreferenceSnapshot>, String> {
        Ok(self.snapshot.clone())
    }

    fn save(
        &mut self,
        revision: u64,
        preferred_model: Option<ModelId>,
        startup_compute: StartupComputePreference,
    ) -> ModelPreferenceAcknowledgement {
        if !self.fail {
            self.snapshot = Some(ModelPreferenceSnapshot {
                revision,
                preferred_model,
                startup_compute,
                legacy_model_path: LegacyModelPath::None,
            });
        }
        ModelPreferenceAcknowledgement {
            revision: if self.wrong_ack {
                revision.saturating_sub(1)
            } else {
                revision
            },
            result: if self.fail {
                Err("test write failure".into())
            } else {
                Ok(())
            },
        }
    }
}

#[test]
fn turbo_is_default_and_user_model_and_auto_preferences_are_saved_with_revisions() {
    let port = MemoryPreferences {
        snapshot: None,
        fail: false,
        wrong_ack: false,
    };
    let mut app = ModelPreferencesApp::new(Box::new(port)).unwrap();
    assert_eq!(app.preferred_model, Some(ModelId::WhisperLargeV3Turbo));
    assert_eq!(app.startup_compute, StartupComputePreference::FollowProbe);

    app.choose_startup_compute(StartupComputePreference::Auto);
    assert_eq!(
        app.save_status(),
        Some(&ModelPreferenceSaveStatus::Saved { revision: 1 })
    );
    app.choose_model(ModelId::ParakeetTdt06BV3);
    assert_eq!(
        app.save_status(),
        Some(&ModelPreferenceSaveStatus::Saved { revision: 2 })
    );
}

#[test]
fn failed_or_stale_preference_ack_is_reported_without_reverting_the_session_choice() {
    let port = MemoryPreferences {
        snapshot: None,
        fail: true,
        wrong_ack: false,
    };
    let mut app = ModelPreferencesApp::new(Box::new(port)).unwrap();
    app.choose_model(ModelId::ParakeetTdt06BV3);
    assert_eq!(app.preferred_model, Some(ModelId::ParakeetTdt06BV3));
    assert!(matches!(
        app.save_status(),
        Some(ModelPreferenceSaveStatus::Failed { revision: 1, error }) if error == "test write failure"
    ));

    let port = MemoryPreferences {
        snapshot: None,
        fail: false,
        wrong_ack: true,
    };
    let mut app = ModelPreferencesApp::new(Box::new(port)).unwrap();
    app.choose_model(ModelId::ParakeetTdt06BV3);
    assert!(matches!(
        app.save_status(),
        Some(ModelPreferenceSaveStatus::Failed { revision: 1, error }) if error.contains("autre révision")
    ));
}

#[test]
fn unknown_v1_model_path_is_preserved_as_a_diagnostic() {
    let port = MemoryPreferences {
        snapshot: Some(ModelPreferenceSnapshot {
            revision: 0,
            preferred_model: None,
            startup_compute: StartupComputePreference::FollowProbe,
            legacy_model_path: LegacyModelPath::Unknown("C:\\unknown\\model.bin".into()),
        }),
        fail: false,
        wrong_ack: false,
    };
    let app = ModelPreferencesApp::new(Box::new(port)).unwrap();
    assert_eq!(
        app.legacy_path_notice().as_deref(),
        Some(
            "Chemin de modèle historique inconnu : C:\\unknown\\model.bin. Choisissez un modèle du catalogue avant de continuer."
        )
    );
}
