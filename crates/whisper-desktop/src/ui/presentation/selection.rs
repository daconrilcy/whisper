use super::*;

pub fn record_compute_selection_interaction(
    touched: &mut bool,
    interacted: bool,
    changed: bool,
) -> bool {
    let selected = interacted || changed;
    *touched |= selected;
    selected
}

pub fn record_preference_delivery(
    notice: &mut Option<String>,
    result: Result<(), crate::ui::model::PreferenceChannelError>,
) {
    if result.is_err() {
        *notice = Some("Préférence non enregistrée : service de préférences indisponible.".into());
    }
}

pub fn apply_startup_preference(
    compute_choice: &mut ComputeChoice,
    compute_choice_touched: &mut bool,
    preference: whisper_core::path_settings::StartupComputePreference,
) {
    if !*compute_choice_touched
        && preference == whisper_core::path_settings::StartupComputePreference::Auto
    {
        *compute_choice = ComputeChoice::Auto;
        *compute_choice_touched = true;
    }
}

pub fn apply_initial_model(
    selection: &mut SelectionWidgetState,
    selection_touched: bool,
    preferred_model: Option<ModelId>,
) {
    if !selection_touched {
        selection.model = preferred_model;
    }
}

pub fn apply_preference_event(
    selection: &mut SelectionWidgetState,
    selection_touched: bool,
    compute_choice: &mut ComputeChoice,
    compute_choice_touched: &mut bool,
    current_revision: &mut u64,
    event: ModelPreferenceEvent,
) -> Option<String> {
    match event {
        ModelPreferenceEvent::Initialized {
            model,
            startup_compute,
            notice,
        } => {
            apply_initial_model(selection, selection_touched, model);
            apply_startup_preference(compute_choice, compute_choice_touched, startup_compute);
            notice
        }
        ModelPreferenceEvent::Saved { revision, message } => {
            if revision < *current_revision {
                return None;
            }
            *current_revision = revision;
            Some(message)
        }
    }
}

pub(in crate::ui) fn selected_language(selection: &SelectionWidgetState) -> LanguageChoice {
    selection
        .manual_language
        .as_ref()
        .map_or(LanguageChoice::Automatic, |language| {
            LanguageChoice::Manual(language.clone())
        })
}

pub fn compute_for_model(
    _model: Option<whisper_core::model_catalog::ModelId>,
    preferred: ComputeChoice,
) -> ComputeChoice {
    preferred
}

pub fn model_supports_live(_model: Option<whisper_core::model_catalog::ModelId>) -> bool {
    true
}

pub(in crate::ui) fn unified_live_admission_available(view: &AppView) -> bool {
    use whisper_core::ports::UnifiedSessionPhase as Phase;

    if view.microphone_test_active
        || view.direct_capture_phase.is_some()
        || (view.close_requested && !view.close_ready)
    {
        return false;
    }
    let sessions = &view.unified_parakeet_sessions;
    let admitted = sessions
        .iter()
        .filter(|session| !matches!(session.phase, Phase::Complete | Phase::Recoverable))
        .count();
    if admitted >= 2
        || sessions.iter().any(|session| {
            session.microphone_capture
                && matches!(session.phase, Phase::Starting | Phase::Capturing)
        })
    {
        return false;
    }
    if let Some((job_id, state)) = view.active_job
        && window_must_remain_open(state)
        && !sessions.iter().any(|session| {
            session.job_id == job_id
                && view.active_generation == Some(session.generation)
                && session.microphone_capture
                && matches!(session.phase, Phase::Finalizing | Phase::Publishing)
        })
    {
        return false;
    }
    true
}

pub fn model_selection_combo(
    ui: &mut egui::Ui,
    projection: &EngineProjection,
    selection: &mut SelectionWidgetState,
) -> bool {
    let mut chosen = false;
    let selected_text = projection
        .models
        .iter()
        .find(|model| Some(model.id) == selection.model)
        .map(|model| {
            if model.selectable {
                model.display_name.clone()
            } else {
                format!("{} (indisponible)", model.display_name)
            }
        })
        .unwrap_or_else(|| "Choisir un modèle".into());
    egui::ComboBox::from_id_salt("model_choice")
        .selected_text(selected_text)
        .show_ui(ui, |ui| {
            for model in &projection.models {
                let response = ui.add_enabled(
                    model.selectable,
                    egui::Button::selectable(
                        selection.model == Some(model.id),
                        &model.display_name,
                    ),
                );
                let clicked = response.clicked();
                if let Some(cause) = model.cause {
                    response.on_hover_text(format!("Indisponible : {cause:?}"));
                }
                if clicked && model.selectable {
                    selection.model = Some(model.id);
                    chosen = true;
                }
            }
        });
    chosen
}

pub fn language_selection_combo(
    ui: &mut egui::Ui,
    projection: &EngineProjection,
    compute_choice: ComputeChoice,
    selection: &mut SelectionWidgetState,
) {
    let Some(model) = projection
        .models
        .iter()
        .find(|model| Some(model.id) == selection.model)
    else {
        ui.add_enabled(false, egui::Button::new("Auto"));
        ui.label("Langue indisponible : modèle non sélectionné.");
        return;
    };
    let Some(configuration) = language_choices_for_compute(model, compute_choice) else {
        ui.add_enabled(false, egui::Button::new("Auto"));
        ui.label("Langue indisponible : configuration absente ou non qualifiée.");
        return;
    };
    let selected_text = selection
        .manual_language
        .as_deref()
        .and_then(|code| {
            TURBO_LANGUAGE_INPUTS
                .iter()
                .find(|(known, _)| *known == code)
        })
        .map(|(code, name)| format!("{name} ({code})"))
        .unwrap_or_else(|| "Auto".into());
    egui::ComboBox::from_id_salt("language_choice")
        .selected_text(selected_text)
        .show_ui(ui, |ui| {
            let response = ui.add_enabled(
                configuration.auto_language_selectable,
                egui::Button::selectable(selection.manual_language.is_none(), "Auto"),
            );
            let clicked = response.clicked();
            if let Some(cause) = configuration.auto_language_cause {
                response.on_hover_text(format!("Auto indisponible : {cause:?}"));
            }
            if clicked && configuration.auto_language_selectable {
                selection.manual_language = None;
            }
            for (language, label) in language_choice_labels(&configuration) {
                let code = language.code.0.as_str();
                let response = ui.add_enabled(
                    language.selectable,
                    egui::Button::selectable(
                        selection.manual_language.as_deref() == Some(code),
                        label,
                    ),
                );
                let clicked = response.clicked();
                if let Some(cause) = language.cause {
                    response.on_hover_text(format!("Langue indisponible : {cause:?}"));
                }
                if clicked && language.selectable {
                    selection.manual_language = Some(code.to_owned());
                }
            }
        });
    if selection.manual_language.is_none() && !configuration.auto_language_selectable {
        ui.label(format!(
            "Auto indisponible : {:?}",
            configuration.auto_language_cause
        ));
    } else if let Some(language) = selection.manual_language.as_deref()
        && let Some(option) = configuration
            .languages
            .iter()
            .find(|option| option.code.0 == language)
        && let Some(cause) = option.cause
    {
        ui.label(format!("Langue indisponible : {cause:?}"));
    }
}

pub(in crate::ui) fn window_must_remain_open(state: JobState) -> bool {
    matches!(
        state,
        JobState::Preparing
            | JobState::Queued
            | JobState::Capturing
            | JobState::Draining
            | JobState::Running
            | JobState::Cancelling
            | JobState::Finalizing
            | JobState::AwaitingChoice
    )
}
