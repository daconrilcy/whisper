use eframe::egui::{self, Shape};
use whisper_core::{
    ComputeChoice,
    engine_capabilities::ResourceObservation,
    model_catalog::{Backend, ModelAvailability, ModelId},
    model_registry,
    path_settings::StartupComputePreference,
};
use whisper_desktop::ui::{
    ModelPreferenceCommand, ModelPreferenceEvent, SelectionWidgetState, apply_initial_model,
    apply_preference_event, apply_startup_preference, compute_for_model,
    language_choices_for_compute, language_option_labels, language_selection_combo,
    model_preference_command_channel, model_selection_combo, model_supports_live,
    record_compute_selection_interaction, record_preference_delivery,
};

#[test]
fn snapshot_renders_catalog_models_and_all_turbo_languages_without_model_path_input() {
    let snapshot = model_registry::engine_snapshot(
        1,
        |id| match id {
            ModelId::WhisperLargeV3Turbo => ModelAvailability::Ready,
            ModelId::ParakeetTdt06BV3 => ModelAvailability::Missing,
        },
        Vec::<ResourceObservation>::new(),
    );
    let projection = whisper_core::model_selection_app::ModelSelectionApp::new(snapshot)
        .projection(1)
        .expect("matching snapshot generation");
    let turbo = projection
        .models
        .iter()
        .find(|model| model.id == ModelId::WhisperLargeV3Turbo)
        .unwrap();
    let parakeet = projection
        .models
        .iter()
        .find(|model| model.id == ModelId::ParakeetTdt06BV3)
        .unwrap();
    assert!(turbo.selectable);
    assert!(!parakeet.selectable);
    assert!(parakeet.cause.is_some());
    let turbo_configuration = turbo.configurations.first().unwrap();
    assert_eq!(turbo_configuration.languages.len(), 100);
    assert!(turbo_configuration.auto_language_selectable);
    assert!(
        turbo_configuration
            .languages
            .iter()
            .all(|language| language.selectable)
    );
    let labels = language_option_labels(turbo_configuration);
    assert_eq!(labels.len(), 100);
    assert!(labels.iter().any(|(_, label)| label == "english (en)"));
    assert!(labels.iter().any(|(_, label)| label == "cantonese (yue)"));

    let context = egui::Context::default();
    let output = context.run_ui(Default::default(), |ui| {
        let mut selection = SelectionWidgetState {
            model: Some(ModelId::ParakeetTdt06BV3),
            manual_language: None,
        };
        model_selection_combo(ui, &projection, &mut selection);
        language_selection_combo(ui, &projection, ComputeChoice::Cpu, &mut selection);
    });
    let rendered = rendered_text(output.shapes);
    assert!(rendered.contains("Parakeet TDT 0.6B v3 (indisponible)"));
    assert!(rendered.contains("Auto indisponible"));
    assert!(!rendered.contains("ggml-large-v3-turbo.bin"));
}

#[test]
fn selection_ui_rendering_does_not_hash_model_or_runtime_files() {
    let model_reads_before = whisper_adapters::model_catalog::model_sha256_full_read_total();
    let artifact_reads_before = whisper_adapters::model_catalog::artifact_sha256_full_read_total();
    let snapshot = model_registry::engine_snapshot(
        1,
        |_| ModelAvailability::Ready,
        Vec::<ResourceObservation>::new(),
    );
    let projection = whisper_core::model_selection_app::ModelSelectionApp::new(snapshot)
        .projection(1)
        .expect("matching snapshot generation");
    let context = egui::Context::default();
    let _ = context.run_ui(Default::default(), |ui| {
        let mut selection = SelectionWidgetState {
            model: Some(ModelId::ParakeetTdt06BV3),
            manual_language: None,
        };
        model_selection_combo(ui, &projection, &mut selection);
        language_selection_combo(ui, &projection, ComputeChoice::Gpu, &mut selection);
    });

    assert_eq!(
        whisper_adapters::model_catalog::model_sha256_full_read_total(),
        model_reads_before,
        "the UI must consume a prepared catalog snapshot rather than hash model files"
    );
    assert_eq!(
        whisper_adapters::model_catalog::artifact_sha256_full_read_total(),
        artifact_reads_before,
        "the UI must consume a prepared catalog snapshot rather than hash workers or DLLs"
    );
}

#[test]
fn language_availability_uses_selected_backend_and_auto_intersection() {
    use whisper_core::engine_capabilities::RefusalCause;

    let snapshot = model_registry::engine_snapshot(
        1,
        |_| ModelAvailability::Ready,
        Vec::<ResourceObservation>::new(),
    );
    let mut projection = whisper_core::model_selection_app::ModelSelectionApp::new(snapshot)
        .projection(1)
        .unwrap();
    let turbo = projection
        .models
        .iter_mut()
        .find(|model| model.id == ModelId::WhisperLargeV3Turbo)
        .unwrap();
    let cpu = turbo
        .configurations
        .iter_mut()
        .find(|config| config.key.backend == Backend::Cpu)
        .unwrap();
    let cpu_fr = cpu
        .languages
        .iter_mut()
        .find(|language| language.code.0 == "fr")
        .unwrap();
    cpu_fr.selectable = false;
    cpu_fr.cause = Some(RefusalCause::LanguageUnavailable);

    let cpu_choices = language_choices_for_compute(turbo, ComputeChoice::Cpu).unwrap();
    let gpu_choices = language_choices_for_compute(turbo, ComputeChoice::Gpu).unwrap();
    let auto_choices = language_choices_for_compute(turbo, ComputeChoice::Auto).unwrap();
    let cpu_fr = cpu_choices
        .languages
        .iter()
        .find(|language| language.code.0 == "fr")
        .unwrap();
    let gpu_fr = gpu_choices
        .languages
        .iter()
        .find(|language| language.code.0 == "fr")
        .unwrap();
    let auto_fr = auto_choices
        .languages
        .iter()
        .find(|language| language.code.0 == "fr")
        .unwrap();
    assert!(!cpu_fr.selectable);
    assert_eq!(cpu_fr.cause, Some(RefusalCause::LanguageUnavailable));
    assert!(gpu_fr.selectable);
    assert!(gpu_fr.cause.is_none());
    assert!(!auto_fr.selectable);
    assert_eq!(auto_fr.cause, Some(RefusalCause::LanguageUnavailable));

    let context = egui::Context::default();
    let output = context.run_ui(Default::default(), |ui| {
        for compute in [ComputeChoice::Cpu, ComputeChoice::Gpu, ComputeChoice::Auto] {
            let mut selection = SelectionWidgetState {
                model: Some(ModelId::WhisperLargeV3Turbo),
                manual_language: Some("fr".into()),
            };
            language_selection_combo(ui, &projection, compute, &mut selection);
        }
    });
    let rendered = rendered_text(output.shapes);
    assert!(rendered.contains("LanguageUnavailable"));
}

#[test]
fn parakeet_language_projection_follows_qualified_gpu_or_auto_preference() {
    let snapshot = model_registry::engine_snapshot(
        1,
        |_| ModelAvailability::Ready,
        Vec::<ResourceObservation>::new(),
    );
    let projection = whisper_core::model_selection_app::ModelSelectionApp::new(snapshot)
        .projection(1)
        .unwrap();
    let parakeet = projection
        .models
        .iter()
        .find(|model| model.id == ModelId::ParakeetTdt06BV3)
        .unwrap();

    for preference in [ComputeChoice::Gpu, ComputeChoice::Auto] {
        let effective = compute_for_model(Some(ModelId::ParakeetTdt06BV3), preference);
        let choices = language_choices_for_compute(parakeet, effective).unwrap();
        assert_eq!(effective, preference);
        assert!(choices.auto_language_selectable);
        assert!(
            choices
                .languages
                .iter()
                .all(|language| !language.selectable)
        );
        assert_eq!(
            compute_for_model(Some(ModelId::WhisperLargeV3Turbo), preference),
            preference,
            "Turbo keeps the user's compute preference"
        );
    }
}

#[test]
fn late_saved_preferences_do_not_overwrite_a_newer_user_choice() {
    let mut compute = ComputeChoice::Cpu;
    let mut compute_touched = false;
    apply_startup_preference(
        &mut compute,
        &mut compute_touched,
        StartupComputePreference::Auto,
    );
    assert_eq!(compute, ComputeChoice::Auto);
    assert!(compute_touched);

    let mut compute = ComputeChoice::Gpu;
    let mut compute_touched = true;
    apply_startup_preference(
        &mut compute,
        &mut compute_touched,
        StartupComputePreference::Auto,
    );
    assert_eq!(compute, ComputeChoice::Gpu);

    let mut selection = SelectionWidgetState {
        model: Some(ModelId::WhisperLargeV3Turbo),
        manual_language: None,
    };
    apply_initial_model(&mut selection, true, Some(ModelId::ParakeetTdt06BV3));
    assert_eq!(selection.model, Some(ModelId::WhisperLargeV3Turbo));

    let mut revision = 4;
    let mut compute = ComputeChoice::Auto;
    let mut compute_touched = true;
    let notice = apply_preference_event(
        &mut selection,
        true,
        &mut compute,
        &mut compute_touched,
        &mut revision,
        ModelPreferenceEvent::Saved {
            revision: 5,
            message: "Préférence non enregistrée : accès refusé".into(),
        },
    );
    assert_eq!(
        notice.as_deref(),
        Some("Préférence non enregistrée : accès refusé")
    );
    let stale_notice = apply_preference_event(
        &mut selection,
        true,
        &mut compute,
        &mut compute_touched,
        &mut revision,
        ModelPreferenceEvent::Saved {
            revision: 4,
            message: "Préférence enregistrée".into(),
        },
    );
    assert!(stale_notice.is_none());
    assert_eq!(selection.model, Some(ModelId::WhisperLargeV3Turbo));
    assert_eq!(compute, ComputeChoice::Auto);
}

#[test]
fn same_value_cpu_click_blocks_late_saved_auto_preference() {
    let mut compute = ComputeChoice::Cpu;
    let mut touched = false;
    let should_persist = record_compute_selection_interaction(&mut touched, true, false);
    assert!(should_persist);
    assert!(touched);
    apply_startup_preference(&mut compute, &mut touched, StartupComputePreference::Auto);
    assert_eq!(compute, ComputeChoice::Cpu);
}

#[test]
fn preference_mailbox_is_bounded_and_keeps_the_latest_choice_per_field() {
    let (sender, receiver) = model_preference_command_channel();
    for model in [ModelId::ParakeetTdt06BV3, ModelId::WhisperLargeV3Turbo] {
        sender
            .send(ModelPreferenceCommand::ChooseModel(model))
            .unwrap();
    }
    sender
        .send(ModelPreferenceCommand::ChooseStartupCompute(
            StartupComputePreference::Auto,
        ))
        .unwrap();
    let latest = receiver.recv().unwrap();
    assert_eq!(
        latest,
        vec![
            ModelPreferenceCommand::ChooseModel(ModelId::WhisperLargeV3Turbo),
            ModelPreferenceCommand::ChooseStartupCompute(StartupComputePreference::Auto),
        ]
    );
}

#[test]
fn disconnected_preference_worker_reports_failed_model_and_compute_saves() {
    let (sender, receiver) = model_preference_command_channel();
    drop(receiver);
    let load_notice = Some("Impossible de lire les préférences de modèle.".to_owned());
    let mut save_notice = None;

    let result = sender.send(ModelPreferenceCommand::ChooseModel(
        ModelId::WhisperLargeV3Turbo,
    ));
    record_preference_delivery(&mut save_notice, result);
    assert_eq!(
        save_notice.as_deref(),
        Some("Préférence non enregistrée : service de préférences indisponible.")
    );
    assert_eq!(
        load_notice.as_deref(),
        Some("Impossible de lire les préférences de modèle.")
    );

    let result = sender.send(ModelPreferenceCommand::ChooseStartupCompute(
        StartupComputePreference::Auto,
    ));
    record_preference_delivery(&mut save_notice, result);
    assert_eq!(
        save_notice.as_deref(),
        Some("Préférence non enregistrée : service de préférences indisponible.")
    );
    assert_eq!(
        load_notice.as_deref(),
        Some("Impossible de lire les préférences de modèle.")
    );
}

fn rendered_text(shapes: Vec<egui::epaint::ClippedShape>) -> String {
    shapes
        .into_iter()
        .filter_map(|clipped| match clipped.shape {
            Shape::Text(text) => Some(text.galley.job.text.to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn parakeet_preserves_import_compute_preferences_and_disables_live_without_changing_turbo() {
    assert_eq!(
        compute_for_model(Some(ModelId::ParakeetTdt06BV3), ComputeChoice::Gpu),
        ComputeChoice::Gpu
    );
    assert_eq!(
        compute_for_model(Some(ModelId::ParakeetTdt06BV3), ComputeChoice::Auto),
        ComputeChoice::Auto
    );
    assert_eq!(
        compute_for_model(Some(ModelId::WhisperLargeV3Turbo), ComputeChoice::Gpu),
        ComputeChoice::Gpu
    );
    assert_eq!(
        compute_for_model(Some(ModelId::WhisperLargeV3Turbo), ComputeChoice::Auto),
        ComputeChoice::Auto
    );
    assert!(!model_supports_live(Some(ModelId::ParakeetTdt06BV3)));
    assert!(model_supports_live(Some(ModelId::WhisperLargeV3Turbo)));
}
