use crate::ui::model::{ModelPreferenceEvent, SelectionWidgetState};
use eframe::egui;
use whisper_adapters::model_catalog::CatalogSnapshot;
use whisper_core::engine_capabilities::EngineProjection;
use whisper_core::gpu_inventory::GpuQualifiedDevice;
use whisper_core::gpu_inventory_app::{GpuInventoryState, GpuQualifiedInventoryState};
use whisper_core::gpu_selection::{SelectionResolution, SessionCandidate};
use whisper_core::model_catalog::{Backend, ModelId};
use whisper_core::model_registry::TURBO_LANGUAGE_INPUTS;
use whisper_core::path_settings::SaveStatus;
use whisper_core::{
    AppCommand, AppView, ComputeChoice, Generation, JobId, JobState, LanguageChoice,
    MicrophoneChoice, MicrophoneDevice,
};

mod gpu;
mod language;
mod microphone;
mod paths;
mod recovery;
mod selection;
mod text;

pub use gpu::{
    gpu_inventory_status, gpu_inventory_status_message, gpu_preference_combo,
    gpu_preference_labels, gpu_preference_options, gpu_preference_selectable,
    gpu_qualified_presentation, gpu_qualified_status, gpu_selection_resolution_label,
};
pub(in crate::ui) use gpu::{parakeet_gpu_catalog_ready, qualified_devices};
pub(in crate::ui) use language::language_choice_labels;
pub use language::{LanguageChoices, language_choices_for_compute, language_option_labels};
pub(in crate::ui) use microphone::{
    MICROPHONE_SATURATION_COLOR, MICROPHONE_SATURATION_LABEL, MicrophoneLabelProjection,
    microphone_choice_label, microphone_device_label, microphone_saturation_is_current,
    microphone_test_start_enabled,
};
pub use microphone::{
    MicrophoneMeterPresentation, microphone_meter_presentation, microphone_source_change_notice,
};
pub use paths::{
    APPROVED_MODEL_SHA256, approved_model_sha256, default_audio_directory, default_destination,
    default_model_path, path_save_status_label,
};
pub(in crate::ui) use paths::{
    copyable_path, parse_sha256, path_if_file, path_is_file, path_text_edit,
};
pub use recovery::source_snapshot_recovery_widget;
pub use selection::{
    apply_initial_model, apply_preference_event, apply_startup_preference, compute_for_model,
    language_selection_combo, model_selection_combo, model_supports_live,
    record_compute_selection_interaction, record_preference_delivery,
};
pub(in crate::ui) use selection::{
    selected_language, unified_live_admission_available, window_must_remain_open,
};
pub(in crate::ui) use text::{
    capture_state_label, compute_choice_label, provisional_without_confirmed_prefix,
    unified_session_phase_label,
};
