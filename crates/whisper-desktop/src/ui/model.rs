use crate::reactivity_log::ReactivityLog;
use crate::ui::presentation::*;
use eframe::egui;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{
    Arc, Mutex,
    mpsc::{Receiver, SyncSender, TryRecvError, TrySendError},
};
use whisper_adapters::model_catalog::CatalogSnapshot;
use whisper_core::engine_capabilities::EngineProjection;
use whisper_core::gpu_inventory_app::{GpuInventoryApp, GpuInventoryState};
use whisper_core::gpu_selection::{SelectionResolution, SessionCandidate};
use whisper_core::model_catalog::ModelId;
use whisper_core::model_selection_app::ModelSelectionApp;
use whisper_core::path_settings::{PathDefaults, PathField, PathSettingsApp};
use whisper_core::ports::DuplicateImportChoice;
use whisper_core::{
    AppCommand, AppFacade, AppView, Application, ApplicationError, ComputeChoice, Generation,
    ImportRequest, JobConfig, JobId, JobState, LiveRequest, MicrophoneChoice, MicrophonePort,
    MicrophoneSettings, NullMicrophonePort,
};

mod controller;
mod preferences;
#[cfg(test)]
use controller::{command_compute_choice, gpu_selection_identity, import_enqueue_enabled};
use controller::{
    live_log_session, manual_stop_available, queued_request_for_compute, record_first_render,
    unix_time_ms,
};
mod view;
pub use preferences::{
    ModelPreferenceCommand, ModelPreferenceCommandReceiver, ModelPreferenceCommandSender,
    ModelPreferenceEvent, ModelSelectionRuntime, PreferenceChannelError, SelectionWidgetState,
    model_preference_command_channel,
};
#[cfg(test)]
#[path = "tests/catalog_fixture.rs"]
mod catalog_fixture;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;

const IMPORT_CPU_MVP: bool = true;
static NEXT_JOB: AtomicU64 = AtomicU64::new(1);
static NEXT_IMPORT_JOB_ID: AtomicU64 = AtomicU64::new(1);

type UnifiedAudioRequestKey = (
    JobId,
    Generation,
    u64,
    u64,
    u64,
    Option<u64>,
    whisper_core::ports::UnifiedSessionPhase,
);

pub struct DesktopApp<A> {
    application: AppFacade<A>,
    view: AppView,
    paths: PathSettingsApp,
    microphones: MicrophoneSettings,
    path_notice: Option<String>,
    microphone_notice: Option<String>,
    auto_stop_enabled: bool,
    auto_stop_minutes: u64,
    compute_choice: ComputeChoice,
    unified_candidate_enabled: bool,
    compute_choice_touched: bool,
    startup_preferences_loaded: bool,
    startup_message: Option<String>,
    gpu_probe: Option<Receiver<Result<(), String>>>,
    gpu_inventory: Option<GpuInventoryApp>,
    quit_pending: bool,
    active_identity: Option<(JobId, Generation)>,
    scanned_destination: Option<String>,
    observed_job_state: Option<JobState>,
    resume_confirmation: Option<(JobId, Generation)>,
    live_request: Option<LiveRequest>,
    provisional_first_rendered_at_unix_ms: Option<u64>,
    provisional_render_identity: Option<(JobId, Generation, u64)>,
    process_started_at_unix_ms: u64,
    live_clicked_at_unix_ms: Option<u64>,
    live_attempt_counter: u64,
    reactivity_log: ReactivityLog,
    catalog_receiver: Option<Receiver<CatalogSnapshot>>,
    preference_events: Option<Receiver<ModelPreferenceEvent>>,
    preference_commands: Option<ModelPreferenceCommandSender>,
    catalog: Option<CatalogSnapshot>,
    projection: Option<EngineProjection>,
    selection: SelectionWidgetState,
    selection_touched: bool,
    selection_notice: Option<String>,
    preference_load_notice: Option<String>,
    preference_notice: Option<String>,
    preference_revision: u64,
    selected_parakeet_capture: Option<(JobId, Generation)>,
    selected_capture_text: CaptureTextSelection,
    automatic_draft_cache: Option<((JobId, Generation), String, String)>,
    automatic_draft_error: Option<String>,
    microphone_test_identity: Option<(JobId, Generation)>,
    last_test_default_identity: Option<String>,
    confirmed_live_default_identity: Option<String>,
    last_unified_document_request: Option<(JobId, Generation, u64, u64)>,
    last_unified_audio_request: Option<UnifiedAudioRequestKey>,
    unified_wait_tracking: std::collections::HashMap<
        (JobId, Generation),
        (
            u64,
            u64,
            whisper_core::ports::UnifiedSessionPhase,
            std::time::Instant,
        ),
    >,
    #[cfg(test)]
    microphone_test_button_rect: Option<egui::Rect>,
    #[cfg(test)]
    microphone_saturation_badge_rect: Option<egui::Rect>,
    #[cfg(test)]
    live_stop_button_rect: Option<egui::Rect>,
    #[cfg(test)]
    live_start_button_rect: Option<egui::Rect>,
    #[cfg(test)]
    live_start_button_enabled: Option<bool>,
    #[cfg(test)]
    microphone_gain_slider_enabled: Option<bool>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum CaptureTextSelection {
    #[default]
    Confirmed,
    AutomaticDraft,
}

impl<A: Application> DesktopApp<A> {
    pub fn new(application: A) -> Self {
        Self::with_startup_compute(application, ComputeChoice::Cpu, None)
    }

    pub fn with_startup_compute(
        application: A,
        compute_choice: ComputeChoice,
        startup_message: Option<String>,
    ) -> Self {
        let paths = PathSettingsApp::with_defaults(PathDefaults {
            audio_directory: default_audio_directory(),
            model_path: default_model_path(),
            destination: default_destination(),
        });
        Self::with_paths(application, compute_choice, startup_message, paths)
    }

    pub fn with_paths(
        application: A,
        compute_choice: ComputeChoice,
        startup_message: Option<String>,
        paths: PathSettingsApp,
    ) -> Self {
        Self::with_microphones(
            application,
            compute_choice,
            startup_message,
            paths,
            Box::new(NullMicrophonePort),
        )
    }

    pub fn with_gpu_probe(mut self, gpu_probe: Receiver<Result<(), String>>) -> Self {
        self.gpu_probe = Some(gpu_probe);
        self
    }

    pub fn with_gpu_inventory(mut self, gpu_inventory: GpuInventoryApp) -> Self {
        self.gpu_inventory = Some(gpu_inventory);
        self
    }

    pub fn with_process_started(mut self, started_at_unix_ms: u64) -> Self {
        self.process_started_at_unix_ms = started_at_unix_ms;
        self
    }

    pub(crate) fn with_unified_candidate_enabled(mut self, enabled: bool) -> Self {
        self.unified_candidate_enabled = enabled;
        self
    }

    pub fn with_microphones(
        application: A,
        compute_choice: ComputeChoice,
        startup_message: Option<String>,
        paths: PathSettingsApp,
        microphone_port: Box<dyn MicrophonePort>,
    ) -> Self {
        let mut microphones =
            MicrophoneSettings::new(microphone_port, paths.microphone_preference());
        let microphone_notice = microphones.take_notice();
        let application = AppFacade::new(application);
        let view = application.view();
        let app = Self {
            application,
            view,
            paths,
            microphones,
            path_notice: None,
            microphone_notice,
            auto_stop_enabled: false,
            auto_stop_minutes: 120,
            compute_choice,
            unified_candidate_enabled: false,
            compute_choice_touched: false,
            startup_preferences_loaded: true,
            startup_message,
            gpu_probe: None,
            gpu_inventory: None,
            quit_pending: false,
            active_identity: None,
            scanned_destination: None,
            observed_job_state: None,
            resume_confirmation: None,
            live_request: None,
            provisional_first_rendered_at_unix_ms: None,
            provisional_render_identity: None,
            process_started_at_unix_ms: unix_time_ms(),
            live_clicked_at_unix_ms: None,
            live_attempt_counter: 0,
            reactivity_log: ReactivityLog::start(),
            catalog_receiver: None,
            preference_events: None,
            preference_commands: None,
            catalog: None,
            projection: None,
            selection: SelectionWidgetState {
                model: Some(ModelId::WhisperLargeV3Turbo),
                manual_language: None,
            },
            selection_touched: false,
            selection_notice: None,
            preference_load_notice: None,
            preference_notice: None,
            preference_revision: 0,
            selected_parakeet_capture: None,
            selected_capture_text: CaptureTextSelection::Confirmed,
            automatic_draft_cache: None,
            automatic_draft_error: None,
            microphone_test_identity: None,
            last_test_default_identity: None,
            confirmed_live_default_identity: None,
            last_unified_document_request: None,
            last_unified_audio_request: None,
            unified_wait_tracking: std::collections::HashMap::new(),
            #[cfg(test)]
            microphone_test_button_rect: None,
            #[cfg(test)]
            microphone_saturation_badge_rect: None,
            #[cfg(test)]
            live_stop_button_rect: None,
            #[cfg(test)]
            live_start_button_rect: None,
            #[cfg(test)]
            live_start_button_enabled: None,
            #[cfg(test)]
            microphone_gain_slider_enabled: None,
        };
        #[cfg(test)]
        {
            let mut app = app;
            app.install_test_model_catalog();
            app
        }
        #[cfg(not(test))]
        {
            app
        }
    }

    pub fn with_model_selection_runtime(mut self, runtime: ModelSelectionRuntime) -> Self {
        self.catalog_receiver = Some(runtime.catalog);
        self.preference_events = Some(runtime.preference_events);
        self.preference_commands = Some(runtime.preference_commands);
        self.startup_preferences_loaded = false;
        self
    }
}
