use crate::reactivity_log::ReactivityLog;
use eframe::egui;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{
    Arc, Mutex,
    mpsc::{Receiver, SyncSender, TryRecvError, TrySendError},
};
use whisper_adapters::model_catalog::CatalogSnapshot;
use whisper_core::engine_capabilities::EngineProjection;
use whisper_core::gpu_inventory::GpuQualifiedDevice;
use whisper_core::gpu_inventory_app::{
    GpuInventoryApp, GpuInventoryState, GpuQualifiedInventoryState,
};
use whisper_core::gpu_selection::{SelectionResolution, SessionCandidate};
use whisper_core::model_catalog::{Backend, ModelId};
use whisper_core::model_registry::TURBO_LANGUAGE_INPUTS;
use whisper_core::model_selection_app::ModelSelectionApp;
use whisper_core::path_settings::{PathDefaults, PathField, PathSettingsApp, SaveStatus};

#[derive(Clone, Debug, PartialEq)]
pub enum MicrophoneMeterPresentation {
    Awaiting,
    Stale,
    WrongCapture,
    Absent,
    Silence,
    Signal { rms_dbfs: f64, peak_dbfs: f64 },
}

fn paint_microphone_spectrum(
    ui: &mut egui::Ui,
    title: &str,
    bins: &[whisper_core::microphone_level::SpectrumBin],
    sample_rate_hz: u32,
) {
    use whisper_core::microphone_level::SpectrumBin;
    ui.label(title);
    if bins.iter().all(|bin| matches!(bin, SpectrumBin::Silence)) {
        ui.label("Silence détecté");
    }
    let width = ui.available_width().max(180.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 116.0), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 2.0, egui::Color32::from_gray(18));
    let left = rect.left() + 34.0;
    let right = rect.right() - 8.0;
    let top = rect.top() + 8.0;
    let bottom = rect.bottom() - 18.0;
    for db in [-120.0, -60.0, 0.0] {
        let y = bottom - (db + 120.0) / 120.0 * (bottom - top);
        painter.line_segment(
            [egui::pos2(left, y), egui::pos2(right, y)],
            egui::Stroke::new(1.0, egui::Color32::DARK_GRAY),
        );
        painter.text(
            egui::pos2(rect.left() + 1.0, y),
            egui::Align2::LEFT_CENTER,
            format!("{db:.0}"),
            egui::FontId::monospace(9.0),
            egui::Color32::GRAY,
        );
    }
    let mut prior = None;
    let mut out_of_scale = false;
    for (index, bin) in bins.iter().enumerate() {
        let SpectrumBin::MilliDb(milli_db) = bin else {
            prior = None;
            continue;
        };
        let db = f64::from(*milli_db) / 1000.0;
        if !(-120.0..=0.0).contains(&db) {
            out_of_scale = true;
        }
        let x = left + (index as f32 / bins.len().saturating_sub(1).max(1) as f32) * (right - left);
        let y = bottom - (((db + 120.0) / 120.0).clamp(0.0, 1.0) as f32) * (bottom - top);
        let point = egui::pos2(x, y);
        if let Some(previous) = prior {
            painter.line_segment(
                [previous, point],
                egui::Stroke::new(1.25, egui::Color32::LIGHT_GREEN),
            );
        }
        prior = Some(point);
    }
    let nyquist = sample_rate_hz / 2;
    painter.text(
        egui::pos2(left, rect.bottom() - 3.0),
        egui::Align2::LEFT_BOTTOM,
        "0 Hz",
        egui::FontId::monospace(9.0),
        egui::Color32::GRAY,
    );
    painter.text(
        egui::pos2(right, rect.bottom() - 3.0),
        egui::Align2::RIGHT_BOTTOM,
        format!("{nyquist} Hz"),
        egui::FontId::monospace(9.0),
        egui::Color32::GRAY,
    );
    if out_of_scale {
        ui.small("Valeurs hors de l’échelle verticale affichées au bord du graphe");
    }
}

const MICROPHONE_SATURATION_LABEL: &str = "Écrêtage après gain détecté";
const MICROPHONE_SATURATION_COLOR: egui::Color32 = egui::Color32::RED;

pub fn microphone_source_change_notice(
    actual: Option<&str>,
    planned: Option<&str>,
    devices: &[MicrophoneDevice],
) -> Option<String> {
    let (Some(actual), Some(planned)) = (actual, planned) else {
        return None;
    };
    if actual == planned {
        return None;
    }
    let labels = MicrophoneLabelProjection::new(devices);
    Some(format!(
        "Le microphone prévu a changé ({}), mais le flux actif reste sur {}. Arrêtez-le puis démarrez une nouvelle action pour ouvrir la nouvelle source.",
        labels.device_label(planned),
        labels.device_label(actual)
    ))
}

pub fn microphone_meter_presentation(
    meter: Option<&whisper_core::microphone_level::MicrophoneMeter>,
    active: Option<(JobId, Generation)>,
    now_ms: u64,
) -> MicrophoneMeterPresentation {
    let Some(meter) = meter else {
        return MicrophoneMeterPresentation::Awaiting;
    };
    if active != Some((meter.job_id, meter.generation)) {
        return MicrophoneMeterPresentation::WrongCapture;
    }
    if now_ms.saturating_sub(meter.captured_at_monotonic_ms) > 500 {
        return MicrophoneMeterPresentation::Stale;
    }
    use whisper_core::microphone_level::MeasurementStatus;
    match meter.post_gain_limiter.status {
        MeasurementStatus::Absent => MicrophoneMeterPresentation::Absent,
        MeasurementStatus::Silence => MicrophoneMeterPresentation::Silence,
        MeasurementStatus::Signal => MicrophoneMeterPresentation::Signal {
            rms_dbfs: meter
                .post_gain_limiter
                .rms_dbfs()
                .unwrap_or(f64::NEG_INFINITY),
            peak_dbfs: if meter.post_gain_limiter.peak() > 0.0 {
                20.0 * meter.post_gain_limiter.peak().log10()
            } else {
                f64::NEG_INFINITY
            },
        },
    }
}

fn microphone_saturation_is_current(
    meter: Option<&whisper_core::microphone_level::MicrophoneMeter>,
    active: Option<(JobId, Generation)>,
    source_identity: Option<&str>,
    capture_active: bool,
    now_ms: u64,
) -> bool {
    let Some(meter) = meter else {
        return false;
    };
    capture_active
        && active == Some((meter.job_id, meter.generation))
        && source_identity == Some(meter.source_identity.as_str())
        && now_ms.saturating_sub(meter.captured_at_monotonic_ms) <= 500
        && meter.limiter_samples > 0
}

fn microphone_test_start_enabled(identified_target: bool, view: &AppView) -> bool {
    identified_target
        && !view.microphone_test_active
        && !view.direct_capture_phase.is_some_and(|phase| {
            matches!(
                phase,
                whisper_core::application::DirectCapturePhase::Initializing
                    | whisper_core::application::DirectCapturePhase::Active
            )
        })
        && !view
            .active_job
            .is_some_and(|(_, state)| window_must_remain_open(state))
}
use whisper_core::ports::DuplicateImportChoice;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelPreferenceCommand {
    ChooseModel(ModelId),
    ChooseStartupCompute(whisper_core::path_settings::StartupComputePreference),
}

pub enum ModelPreferenceEvent {
    Initialized {
        model: Option<ModelId>,
        startup_compute: whisper_core::path_settings::StartupComputePreference,
        notice: Option<String>,
    },
    Saved {
        revision: u64,
        message: String,
    },
}

pub struct ModelSelectionRuntime {
    pub catalog: Receiver<CatalogSnapshot>,
    pub preference_events: Receiver<ModelPreferenceEvent>,
    pub preference_commands: ModelPreferenceCommandSender,
}

#[derive(Default)]
struct PendingPreferenceChoices {
    model: Option<ModelId>,
    startup_compute: Option<whisper_core::path_settings::StartupComputePreference>,
}

/// Bounded wake channel with a coalescing latest-choice mailbox for preference persistence.
#[derive(Clone)]
pub struct ModelPreferenceCommandSender {
    pending: Arc<Mutex<PendingPreferenceChoices>>,
    wake: SyncSender<()>,
}

pub struct ModelPreferenceCommandReceiver {
    pending: Arc<Mutex<PendingPreferenceChoices>>,
    wake: Receiver<()>,
}

pub fn model_preference_command_channel()
-> (ModelPreferenceCommandSender, ModelPreferenceCommandReceiver) {
    let (wake_tx, wake_rx) = std::sync::mpsc::sync_channel(1);
    let pending = Arc::new(Mutex::new(PendingPreferenceChoices::default()));
    (
        ModelPreferenceCommandSender {
            pending: pending.clone(),
            wake: wake_tx,
        },
        ModelPreferenceCommandReceiver {
            pending,
            wake: wake_rx,
        },
    )
}

impl ModelPreferenceCommandSender {
    pub fn send(&self, command: ModelPreferenceCommand) -> Result<(), ()> {
        let mut pending = self.pending.lock().map_err(|_| ())?;
        match command {
            ModelPreferenceCommand::ChooseModel(model) => pending.model = Some(model),
            ModelPreferenceCommand::ChooseStartupCompute(choice) => {
                pending.startup_compute = Some(choice)
            }
        }
        match self.wake.try_send(()) {
            Ok(()) | Err(TrySendError::Full(())) => Ok(()),
            Err(TrySendError::Disconnected(())) => Err(()),
        }
    }
}

impl ModelPreferenceCommandReceiver {
    pub fn recv(&self) -> Result<Vec<ModelPreferenceCommand>, ()> {
        self.wake.recv().map_err(|_| ())?;
        let mut pending = self.pending.lock().map_err(|_| ())?;
        let mut commands = Vec::with_capacity(2);
        if let Some(model) = pending.model.take() {
            commands.push(ModelPreferenceCommand::ChooseModel(model));
        }
        if let Some(choice) = pending.startup_compute.take() {
            commands.push(ModelPreferenceCommand::ChooseStartupCompute(choice));
        }
        Ok(commands)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SelectionWidgetState {
    pub model: Option<ModelId>,
    pub manual_language: Option<String>,
}
use whisper_core::{
    AppCommand, AppFacade, AppView, Application, ApplicationError, ComputeChoice, Generation,
    ImportRequest, JobConfig, JobId, JobState, LanguageChoice, LiveRequest, MicrophoneChoice,
    MicrophoneDevice, MicrophonePort, MicrophoneSettings, NullMicrophonePort,
};

pub const APPROVED_MODEL_SHA256: &str =
    "1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69";
const IMPORT_CPU_MVP: bool = true;
static NEXT_JOB: AtomicU64 = AtomicU64::new(1);
static NEXT_IMPORT_JOB_ID: AtomicU64 = AtomicU64::new(1);

fn next_import_job_id() -> JobId {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(1, |duration| {
            duration.as_nanos().min(u64::MAX as u128) as u64
        });
    let mut observed = NEXT_IMPORT_JOB_ID.load(Ordering::Relaxed);
    let serial = loop {
        let next = now.max(observed.saturating_add(1));
        match NEXT_IMPORT_JOB_ID.compare_exchange_weak(
            observed,
            next,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => break next,
            Err(actual) => observed = actual,
        }
    };
    JobId(((std::process::id() as u128) << 64) | serial as u128)
}

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
    last_unified_audio_request: Option<(
        JobId,
        Generation,
        u64,
        u64,
        u64,
        Option<u64>,
        whisper_core::ports::UnifiedSessionPhase,
    )>,
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

    fn poll_gpu_probe(&mut self) {
        let result = self
            .gpu_probe
            .as_ref()
            .and_then(|probe| match probe.try_recv() {
                Ok(result) => Some(result),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => Some(Err(
                    "GPU indisponible : vérification interrompue ; choisissez CPU".into(),
                )),
            });
        if let Some(result) = result {
            self.gpu_probe = None;
            match result {
                Ok(()) => {
                    if !self.compute_choice_touched {
                        self.compute_choice = ComputeChoice::Gpu;
                    }
                    self.startup_message = None;
                }
                Err(message) => {
                    if !self.compute_choice_touched {
                        self.compute_choice = ComputeChoice::Cpu;
                    }
                    self.startup_message = Some(message);
                }
            }
        }
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

    #[cfg(test)]
    fn install_test_model_catalog(&mut self) {
        let entries = whisper_core::model_registry::MODELS
            .iter()
            .map(|record| whisper_adapters::model_catalog::CatalogEntry {
                id: record.artifact.id,
                display_name: record.display_name.to_owned(),
                path: std::path::PathBuf::from("test-models").join(record.artifact.file_name),
                availability: whisper_core::model_catalog::ModelAvailability::Ready,
                expected_size_bytes: record.artifact.size_bytes,
                observed_size_bytes: Some(record.artifact.size_bytes),
                expected_sha256: record.artifact.sha256.to_owned(),
                observed_sha256: Some(record.artifact.sha256.to_owned()),
                diagnostic: None,
            })
            .collect();
        let catalog = CatalogSnapshot {
            generation: 1,
            entries,
        };
        self.projection = ModelSelectionApp::new(catalog.engine_snapshot(vec![]))
            .projection(catalog.generation)
            .ok();
        self.catalog = Some(catalog);
    }

    pub fn with_model_selection_runtime(mut self, runtime: ModelSelectionRuntime) -> Self {
        self.catalog_receiver = Some(runtime.catalog);
        self.preference_events = Some(runtime.preference_events);
        self.preference_commands = Some(runtime.preference_commands);
        self.startup_preferences_loaded = false;
        self
    }

    fn poll_model_selection(&mut self) {
        while let Some(receiver) = &self.catalog_receiver {
            match receiver.try_recv() {
                Ok(catalog) => {
                    self.projection = ModelSelectionApp::new(catalog.engine_snapshot(vec![]))
                        .projection(catalog.generation)
                        .ok();
                    self.catalog = Some(catalog);
                }
                Err(TryRecvError::Disconnected) => {
                    self.selection_notice = Some("Catalogue de modèles indisponible.".into());
                    self.catalog_receiver = None;
                    break;
                }
                Err(TryRecvError::Empty) => break,
            }
        }
        let mut events = Vec::new();
        let mut preferences_disconnected = false;
        if let Some(receiver) = &self.preference_events {
            loop {
                match receiver.try_recv() {
                    Ok(event) => events.push(event),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        preferences_disconnected = true;
                        break;
                    }
                }
            }
        }
        if preferences_disconnected {
            self.preference_events = None;
        }
        for event in events {
            match event {
                ModelPreferenceEvent::Initialized {
                    model,
                    startup_compute,
                    notice,
                } => {
                    self.startup_preferences_loaded = true;
                    self.preference_load_notice = apply_preference_event(
                        &mut self.selection,
                        self.selection_touched,
                        &mut self.compute_choice,
                        &mut self.compute_choice_touched,
                        &mut self.preference_revision,
                        ModelPreferenceEvent::Initialized {
                            model,
                            startup_compute,
                            notice,
                        },
                    );
                }
                saved @ ModelPreferenceEvent::Saved { .. } => {
                    if let Some(notice) = apply_preference_event(
                        &mut self.selection,
                        self.selection_touched,
                        &mut self.compute_choice,
                        &mut self.compute_choice_touched,
                        &mut self.preference_revision,
                        saved,
                    ) {
                        self.preference_notice = Some(notice);
                    }
                }
            }
        }
    }

    fn selected_model_artifact(&self) -> Option<(String, [u8; 32])> {
        let id = self.selection.model?;
        if !self.selected_model_is_selectable() {
            return None;
        }
        let entry = self
            .catalog
            .as_ref()?
            .entries
            .iter()
            .find(|entry| entry.id == id)?;
        Some((
            entry.path.to_string_lossy().into_owned(),
            parse_sha256(&entry.expected_sha256),
        ))
    }

    fn selected_model_id(&self) -> Option<whisper_core::model_catalog::ModelId> {
        self.selection.model
    }

    fn selected_model_is_selectable(&self) -> bool {
        let Some(id) = self.selection.model else {
            return false;
        };
        self.projection.as_ref().is_some_and(|projection| {
            projection
                .models
                .iter()
                .any(|model| model.id == id && model.selectable)
        })
    }

    fn dispatch(&mut self, command: AppCommand) -> bool {
        let requested_compute = command_compute_choice(&command).unwrap_or(self.compute_choice);
        let gpu_selection_command = matches!(
            &command,
            AppCommand::StartImport { .. }
                | AppCommand::EnqueueImport { .. }
                | AppCommand::ProcessQueued { .. }
                | AppCommand::ResumeInterrupted { .. }
                | AppCommand::StartLive { .. }
                | AppCommand::StartLiveConfigured { .. }
                | AppCommand::StartUnifiedLiveConfigured { .. }
                | AppCommand::StartDirectLiveConfigured { .. }
        );
        if gpu_selection_command
            && let Some((job_id, generation)) = gpu_selection_identity(&command)
        {
            let candidate = if requested_compute == ComputeChoice::Cpu {
                None
            } else {
                self.gpu_inventory.as_ref().and_then(|inventory| {
                    let preferred = self.paths.gpu_preference();
                    match inventory.resolve_selection(requested_compute, preferred) {
                        SelectionResolution::Candidate(SessionCandidate::Gpu(candidate)) => {
                            inventory
                                .runtime_identities()
                                .iter()
                                .find(|proof| {
                                    proof
                                        .pnp_instance_id
                                        .eq_ignore_ascii_case(&candidate.identity)
                                })
                                .cloned()
                        }
                        _ => None,
                    }
                })
            };
            if let Err(error) = self
                .application
                .set_gpu_candidate(job_id, generation, candidate)
            {
                self.view.message = Some(format!("Sélection GPU non transmise : {error:?}"));
                return false;
            }
        }
        match self.application.dispatch(command) {
            Ok(view) => {
                self.view = view;
                true
            }
            Err(ApplicationError::Failed(message)) => {
                self.view.message = Some(message);
                false
            }
            Err(error) => {
                self.view.message = Some(format!("Commande impossible : {error:?}"));
                false
            }
        }
    }

    fn render_unified_parakeet_ui(&mut self, ui: &mut egui::Ui) {
        let sessions = self.view.unified_parakeet_sessions.clone();
        self.unified_wait_tracking.retain(|identity, _| {
            sessions.iter().any(|session| {
                (session.job_id, session.generation) == *identity
                    && matches!(
                        session.phase,
                        whisper_core::ports::UnifiedSessionPhase::Starting
                            | whisper_core::ports::UnifiedSessionPhase::Capturing
                            | whisper_core::ports::UnifiedSessionPhase::Finalizing
                            | whisper_core::ports::UnifiedSessionPhase::Publishing
                    )
            })
        });
        if sessions.is_empty() {
            return;
        }
        ui.separator();
        ui.heading("Transcriptions Parakeet");
        for session in sessions.iter().rev() {
            let identity = (session.job_id, session.generation);
            let selected = self
                .view
                .selected_unified_session
                .as_ref()
                .is_some_and(|view| (view.job_id, view.generation) == identity);
            let label = format!(
                "{} — {}",
                if session.microphone_capture {
                    "Microphone"
                } else {
                    std::path::Path::new(&session.source_path)
                        .file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("Import audio")
                },
                unified_session_phase_label(session.phase)
            );
            if ui.selectable_label(selected, label).clicked() {
                self.last_unified_document_request = None;
                self.last_unified_audio_request = None;
                self.dispatch(AppCommand::SelectUnifiedSession {
                    job_id: identity.0,
                    generation: identity.1,
                });
            }
        }

        let Some(selected) = self.view.selected_unified_session.clone() else {
            return;
        };
        let Some(session) = sessions.iter().find(|session| {
            (session.job_id, session.generation) == (selected.job_id, selected.generation)
        }) else {
            return;
        };
        let identity = (session.job_id, session.generation);
        let active = matches!(
            session.phase,
            whisper_core::ports::UnifiedSessionPhase::Starting
                | whisper_core::ports::UnifiedSessionPhase::Capturing
                | whisper_core::ports::UnifiedSessionPhase::Finalizing
                | whisper_core::ports::UnifiedSessionPhase::Publishing
        );
        if active {
            let progress = (session.captured_samples, session.durable_samples);
            let started = match self.unified_wait_tracking.get(&identity) {
                Some((previous, durable, phase, started))
                    if (*previous, *durable, *phase) == (progress.0, progress.1, session.phase) =>
                {
                    *started
                }
                _ => {
                    let started = std::time::Instant::now();
                    self.unified_wait_tracking
                        .insert(identity, (progress.0, progress.1, session.phase, started));
                    started
                }
            };
            ui.label(format!(
                "{} — temps écoulé depuis le dernier progrès : {:.1} s",
                unified_session_phase_label(session.phase),
                started.elapsed().as_secs_f64()
            ));
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(100));
        } else {
            self.unified_wait_tracking.remove(&identity);
            ui.label(unified_session_phase_label(session.phase));
        }
        if let Some(total) = session.total_samples {
            ui.label(format!(
                "Audio : {} / {} échantillons capturés; {} durables",
                session.captured_samples, total, session.durable_samples
            ));
        } else {
            ui.label(format!(
                "Audio : {} échantillons capturés; {} durables",
                session.captured_samples, session.durable_samples
            ));
        }
        if active
            && ui
                .button(
                    if session.phase == whisper_core::ports::UnifiedSessionPhase::Starting {
                        "Annuler / arrêter"
                    } else {
                        "Arrêter le microphone"
                    },
                )
                .clicked()
        {
            let command = if session.microphone_capture {
                AppCommand::StopUnifiedLive {
                    job_id: session.job_id,
                    generation: session.generation,
                }
            } else {
                AppCommand::Stop {
                    job_id: session.job_id,
                    generation: session.generation,
                }
            };
            self.dispatch(command);
        }
        if let Some(message) = session.message.as_deref() {
            ui.colored_label(egui::Color32::YELLOW, message);
        }

        let document_current = selected
            .document_page
            .as_ref()
            .is_some_and(|page| page.revision == session.document_revision);
        if session.document_revision > 0 && !document_current && selected.document_error.is_none() {
            let first_sequence = selected
                .document_page
                .as_ref()
                .and_then(|page| page.entries.first())
                .map_or(1, |entry| entry.segment.segment_id.0);
            let request = (
                identity.0,
                identity.1,
                session.document_revision,
                first_sequence,
            );
            if self.last_unified_document_request != Some(request) {
                self.last_unified_document_request = Some(request);
                self.dispatch(AppCommand::ReadUnifiedDocumentPage {
                    job_id: identity.0,
                    generation: identity.1,
                    expected_revision: session.document_revision,
                    first_sequence,
                });
            }
        }
        let parts_current = selected.audio_parts_page.as_ref().is_some_and(|page| {
            page.revision == session.audio_parts_revision && session.audio_parts_revision != 0
        });
        if !parts_current && selected.audio_parts_error.is_none() {
            let offset = selected
                .audio_parts_page
                .as_ref()
                .map_or(0, |page| page.offset);
            let request = (
                identity.0,
                identity.1,
                session.audio_parts_revision,
                offset,
                session.durable_samples,
                session.total_samples,
                session.phase,
            );
            if self.last_unified_audio_request != Some(request) {
                self.last_unified_audio_request = Some(request);
                self.dispatch(AppCommand::ReadUnifiedAudioPartsPage {
                    job_id: identity.0,
                    generation: identity.1,
                    expected_revision: session.audio_parts_revision,
                    offset,
                });
            }
        }

        ui.separator();
        if let Some(preview) = selected.preview.as_ref() {
            ui.weak("Aperçu provisoire");
            ui.label(preview.segment.text.as_str());
        }
        ui.label("Texte final");
        if document_current {
            let page = selected.document_page.as_ref().unwrap();
            egui::ScrollArea::vertical()
                .max_height(240.0)
                .show(ui, |ui| {
                    for entry in &page.entries {
                        ui.label(entry.segment.text.as_str());
                    }
                });
            ui.horizontal(|ui| {
                if ui.button("Précédent").clicked() {
                    let first = page.entries.first().map_or(1, |entry| {
                        entry.segment.segment_id.0.saturating_sub(128).max(1)
                    });
                    self.last_unified_document_request =
                        Some((identity.0, identity.1, session.document_revision, first));
                    self.dispatch(AppCommand::ReadUnifiedDocumentPage {
                        job_id: identity.0,
                        generation: identity.1,
                        expected_revision: session.document_revision,
                        first_sequence: first,
                    });
                }
                if let Some(next) = page.next_sequence
                    && ui.button("Suivant").clicked()
                {
                    self.last_unified_document_request =
                        Some((identity.0, identity.1, session.document_revision, next));
                    self.dispatch(AppCommand::ReadUnifiedDocumentPage {
                        job_id: identity.0,
                        generation: identity.1,
                        expected_revision: session.document_revision,
                        first_sequence: next,
                    });
                }
            });
        } else if let Some(error) = selected.document_error.as_deref() {
            ui.colored_label(egui::Color32::YELLOW, error);
        } else if session.document_revision == 0 {
            ui.weak("Aucun texte final n’est encore disponible.");
        } else {
            ui.label("Chargement de la page…");
        }

        ui.separator();
        ui.label("Parties audio WAV");
        if parts_current {
            let page = selected.audio_parts_page.as_ref().unwrap();
            egui::ScrollArea::vertical()
                .max_height(150.0)
                .show(ui, |ui| {
                    for part in &page.entries {
                        ui.label(format!(
                            "Partie {} — {}..{} échantillons",
                            part.number, part.start_sample, part.end_sample
                        ));
                        ui.selectable_label(false, part.path.as_str());
                    }
                });
            ui.horizontal(|ui| {
                if ui.button("Parties précédentes").clicked() {
                    let offset = page.offset.saturating_sub(128);
                    self.last_unified_audio_request = Some((
                        identity.0,
                        identity.1,
                        session.audio_parts_revision,
                        offset,
                        session.durable_samples,
                        session.total_samples,
                        session.phase,
                    ));
                    self.dispatch(AppCommand::ReadUnifiedAudioPartsPage {
                        job_id: identity.0,
                        generation: identity.1,
                        expected_revision: session.audio_parts_revision,
                        offset,
                    });
                }
                if let Some(next) = page.next_offset
                    && ui.button("Parties suivantes").clicked()
                {
                    self.last_unified_audio_request = Some((
                        identity.0,
                        identity.1,
                        session.audio_parts_revision,
                        next,
                        session.durable_samples,
                        session.total_samples,
                        session.phase,
                    ));
                    self.dispatch(AppCommand::ReadUnifiedAudioPartsPage {
                        job_id: identity.0,
                        generation: identity.1,
                        expected_revision: session.audio_parts_revision,
                        offset: next,
                    });
                }
            });
        } else if let Some(error) = selected.audio_parts_error.as_deref() {
            ui.colored_label(egui::Color32::YELLOW, error);
        } else {
            ui.label("Chargement des références de parties…");
        }
        ui.horizontal(|ui| {
            if let Some(path) = session.txt_path.as_deref() {
                ui.label("TXT");
                ui.selectable_label(false, path);
            }
            if let Some(path) = session.srt_path.as_deref() {
                ui.label("SRT");
                ui.selectable_label(false, path);
            }
        });
        if (session.txt_path.is_none() || session.srt_path.is_none())
            && session.phase != whisper_core::ports::UnifiedSessionPhase::Complete
        {
            ui.weak("Les chemins TXT et SRT seront affichés après publication complète.");
        }
    }

    fn render_parakeet_capture_ui(&mut self, ui: &mut egui::Ui) {
        let captures = self.view.parakeet_captures.clone();
        if captures.is_empty() {
            return;
        }
        if self.selected_parakeet_capture.is_none() {
            self.selected_parakeet_capture = self
                .live_request
                .as_ref()
                .map(|request| (request.job_id, request.generation))
                .filter(|identity| {
                    captures.iter().any(|capture| {
                        (capture.identity.job_id, capture.identity.generation) == *identity
                    })
                })
                .or_else(|| {
                    captures
                        .last()
                        .map(|capture| (capture.identity.job_id, capture.identity.generation))
                });
            self.selected_capture_text = CaptureTextSelection::Confirmed;
            self.automatic_draft_cache = None;
            self.automatic_draft_error = None;
        }

        ui.separator();
        ui.label("Captures Parakeet — session actuelle");
        for capture in captures.iter().rev() {
            let identity = (capture.identity.job_id, capture.identity.generation);
            let name = capture
                .wav_path
                .as_deref()
                .and_then(|path| std::path::Path::new(path).file_name())
                .and_then(|name| name.to_str())
                .unwrap_or("Microphone en cours");
            let label = format!("{name} — {}", capture_state_label(capture.capture_state));
            if ui
                .selectable_label(self.selected_parakeet_capture == Some(identity), label)
                .clicked()
            {
                self.selected_parakeet_capture = Some(identity);
                self.selected_capture_text = CaptureTextSelection::Confirmed;
                self.automatic_draft_cache = None;
                self.automatic_draft_error = None;
            }
        }

        let Some(capture) = self.selected_parakeet_capture.and_then(|selected| {
            captures
                .iter()
                .find(|capture| (capture.identity.job_id, capture.identity.generation) == selected)
        }) else {
            return;
        };
        ui.separator();
        ui.strong(
            capture
                .wav_path
                .as_deref()
                .and_then(|path| {
                    std::path::Path::new(path)
                        .file_name()
                        .and_then(|name| name.to_str())
                })
                .unwrap_or("Capture Parakeet"),
        );
        ui.small(format!(
            "{} — calcul demandé : {}; moteur effectif : {}",
            capture_state_label(capture.capture_state),
            capture
                .requested_compute
                .or_else(|| self
                    .live_request
                    .as_ref()
                    .filter(|request| {
                        request.job_id == capture.identity.job_id
                            && request.generation == capture.identity.generation
                    })
                    .map(|request| request.config.compute))
                .map_or("non renseigné", compute_choice_label),
            capture
                .effective_backend
                .as_deref()
                .unwrap_or("non renseigné")
        ));

        if let Some(wav) = capture
            .wav_path
            .as_deref()
            .filter(|path| path_is_file(path))
        {
            ui.label("Audio enregistré : WAV finalisé");
            copyable_path(ui, "WAV", wav);
        } else if capture.capture_state == whisper_core::ports::ParakeetCaptureViewState::Finalizing
        {
            ui.label("Arrêt en cours — finalisation du WAV et des résultats directs");
        } else if capture.capture_state == whisper_core::ports::ParakeetCaptureViewState::Failed {
            ui.label("La capture s'est terminée sans confirmation d'un WAV finalisé.");
        }
        if let Some(diagnostic) = capture.capture_diagnostic.as_deref() {
            ui.colored_label(egui::Color32::YELLOW, diagnostic);
        }
        if capture.capture_complete == Some(false) {
            ui.label("Transcription directe incomplète.");
        }

        if !capture.unresolved.is_empty() {
            egui::CollapsingHeader::new(format!(
                "{} portions restent à confirmer",
                capture.unresolved.len()
            ))
            .show(ui, |ui| {
                for range in &capture.unresolved {
                    ui.small(format!(
                        "{:.2}–{:.2} s",
                        range.start_sample as f64 / f64::from(range.sample_rate_hz),
                        range.end_sample as f64 / f64::from(range.sample_rate_hz)
                    ));
                }
            });
        }

        if let Some(path) = capture
            .direct_txt_path
            .as_deref()
            .filter(|path| path_is_file(path))
        {
            copyable_path(ui, "TXT confirmé", path);
        }
        if let Some(path) = capture
            .direct_srt_path
            .as_deref()
            .filter(|path| path_is_file(path))
        {
            copyable_path(ui, "SRT confirmé", path);
        }
        if let Some(path) = capture
            .companion_json_path
            .as_deref()
            .filter(|path| path_is_file(path))
        {
            copyable_path(ui, "JSON des confirmations et lacunes", path);
        }

        let final_view = capture.automatic_final.as_ref();
        if let Some(final_view) = final_view {
            ui.label(match final_view.status {
                whisper_core::ports::AutomaticFinalTranscriptionStatus::Pending => {
                    "Brouillon final en attente"
                }
                whisper_core::ports::AutomaticFinalTranscriptionStatus::Running => {
                    "Brouillon final en cours — non vérifié"
                }
                whisper_core::ports::AutomaticFinalTranscriptionStatus::ProducedUnverified => {
                    "Brouillon final disponible — à relire"
                }
                whisper_core::ports::AutomaticFinalTranscriptionStatus::FailedOrUnavailable => {
                    "Brouillon final indisponible"
                }
            });
            if final_view.status == whisper_core::ports::AutomaticFinalTranscriptionStatus::Running
            {
                ui.small("Texte automatique non vérifié.");
            }
            ui.small(format!(
                "Calcul demandé : {} ; calcul effectif : {}",
                capture.requested_compute.or_else(|| {
                    (final_view.status
                        != whisper_core::ports::AutomaticFinalTranscriptionStatus::FailedOrUnavailable)
                        .then_some(final_view.requested_compute)
                }).map_or("non renseigné", compute_choice_label),
                final_view
                    .effective_backend
                    .as_deref()
                    .unwrap_or("non renseigné")
            ));
            let final_files_ready = final_view.status
                == whisper_core::ports::AutomaticFinalTranscriptionStatus::ProducedUnverified
                && path_is_file(&final_view.txt_path)
                && path_is_file(&final_view.srt_path);
            if final_files_ready && let Some(path) = path_if_file(&final_view.txt_path) {
                copyable_path(ui, "TXT automatique — non vérifié", path);
            }
            if final_files_ready && let Some(path) = path_if_file(&final_view.srt_path) {
                copyable_path(ui, "SRT automatique — non vérifié", path);
            }
            if final_files_ready && ui.button("Lire le brouillon final — à relire").clicked() {
                match std::fs::read_to_string(&final_view.txt_path) {
                    Ok(text) => {
                        self.automatic_draft_cache = Some((
                            (capture.identity.job_id, capture.identity.generation),
                            final_view.txt_path.clone(),
                            text,
                        ));
                        self.automatic_draft_error = None;
                        self.selected_capture_text = CaptureTextSelection::AutomaticDraft;
                    }
                    Err(error) => {
                        self.automatic_draft_error =
                            Some(format!("Lecture du brouillon impossible : {error}"))
                    }
                }
            }
            if let Some(diagnostic) = final_view.diagnostic.as_deref() {
                ui.colored_label(egui::Color32::YELLOW, diagnostic);
            }
            if final_view.status
                == whisper_core::ports::AutomaticFinalTranscriptionStatus::ProducedUnverified
                && !final_files_ready
            {
                ui.colored_label(egui::Color32::YELLOW, "Le statut de publication est terminé, mais les fichiers TXT/SRT attendus ne sont pas tous accessibles.");
            }
        }

        ui.horizontal(|ui| {
            if ui
                .selectable_label(
                    self.selected_capture_text == CaptureTextSelection::Confirmed,
                    "Confirmé en direct",
                )
                .clicked()
            {
                self.selected_capture_text = CaptureTextSelection::Confirmed;
            }
            let final_available = final_view.is_some_and(|view| {
                view.status
                    == whisper_core::ports::AutomaticFinalTranscriptionStatus::ProducedUnverified
                    && path_is_file(&view.txt_path)
                    && path_is_file(&view.srt_path)
            });
            if final_available
                && ui
                    .selectable_label(
                        self.selected_capture_text == CaptureTextSelection::AutomaticDraft,
                        "Brouillon final — non vérifié",
                    )
                    .clicked()
            {
                self.selected_capture_text = CaptureTextSelection::AutomaticDraft;
                if let Some(view) = final_view
                    && path_is_file(&view.txt_path)
                    && !self
                        .automatic_draft_cache
                        .as_ref()
                        .is_some_and(|(identity, path, _)| {
                            *identity == (capture.identity.job_id, capture.identity.generation)
                                && path == &view.txt_path
                        })
                {
                    match std::fs::read_to_string(&view.txt_path) {
                        Ok(text) => {
                            self.automatic_draft_cache = Some((
                                (capture.identity.job_id, capture.identity.generation),
                                view.txt_path.clone(),
                                text,
                            ))
                        }
                        Err(error) => {
                            self.automatic_draft_error =
                                Some(format!("Lecture du brouillon impossible : {error}"))
                        }
                    }
                }
            }
        });
        match self.selected_capture_text {
            CaptureTextSelection::Confirmed => {
                ui.label("Confirmé en direct — stable selon l'algorithme, non relu");
                let text = capture
                    .confirmed_segments
                    .iter()
                    .map(|segment| segment.text.as_str())
                    .collect::<Vec<_>>()
                    .join("\n");
                egui::ScrollArea::vertical()
                    .max_height(180.0)
                    .show(ui, |ui| {
                        ui.label(if text.is_empty() {
                            "Aucun segment confirmé pour cette capture."
                        } else {
                            &text
                        });
                    });
                if capture.capture_state == whisper_core::ports::ParakeetCaptureViewState::Capturing
                {
                    if let Some(text) = capture.provisional_text.as_deref() {
                        let provisional = provisional_without_confirmed_prefix(capture, text);
                        if let Some(provisional) = provisional {
                            ui.label("Texte provisoire — peut changer");
                            ui.label(provisional);
                        } else {
                            ui.small("Une hypothèse recouvre un bloc confirmé ; son contenu non séparable est masqué pour éviter les doublons.");
                        }
                    }
                }
            }
            CaptureTextSelection::AutomaticDraft => {
                ui.label("Brouillon final automatique — non vérifié, à relire");
                let expected = final_view
                    .map(|view| view.txt_path.as_str())
                    .unwrap_or_default();
                if let Some((identity, path, text)) = self.automatic_draft_cache.as_ref()
                    && *identity == (capture.identity.job_id, capture.identity.generation)
                    && path == expected
                {
                    egui::ScrollArea::vertical()
                        .max_height(180.0)
                        .show(ui, |ui| {
                            ui.label(text.as_str());
                        });
                }
            }
        }
        if let Some(error) = self.automatic_draft_error.as_deref() {
            ui.colored_label(egui::Color32::YELLOW, error);
        }
    }

    fn published_result(&self) -> Option<&str> {
        self.view
            .active_job
            .filter(|(_, state)| *state == JobState::Complete)
            .and(self.view.last_result.as_deref())
    }

    fn stop_active(&mut self) {
        if let Some((job_id, generation)) = self.active_identity {
            self.dispatch(AppCommand::Stop { job_id, generation });
        }
    }

    fn request_quit(&mut self) -> bool {
        self.dispatch(AppCommand::RequestClose)
    }

    fn refresh_microphones(&mut self) {
        match self.microphones.refresh() {
            Ok(()) => {
                self.microphone_notice = None;
                if (self.view.microphone_test_active || self.view.active_job.is_some())
                    && let Some(notice) = microphone_source_change_notice(
                        self.view.microphone_source_identity.as_deref(),
                        self.microphones.target_identity(),
                        self.microphones.devices(),
                    )
                {
                    self.microphone_notice = Some(notice);
                }
            }
            Err(_) => self.microphone_notice = self.microphones.take_notice(),
        }
    }

    fn record_reactivity_milestones(&mut self) {
        let Some(request) = self.live_request.as_ref() else {
            return;
        };
        if self.view.active_job.map(|(job_id, _)| job_id) != Some(request.job_id)
            || self.view.active_generation != Some(request.generation)
        {
            return;
        }
        let session = live_log_session(
            request.job_id,
            request.generation,
            self.live_attempt_counter,
        );
        let clicked_at = self.live_clicked_at_unix_ms;
        let milestones = [
            ("first_sample", self.view.direct_first_sample_at_unix_ms),
            (
                "first_wav_block_admitted",
                self.view.direct_first_admitted_at_unix_ms,
            ),
            ("engine_ready", self.view.direct_ready_at_unix_ms),
        ];
        for (event, timestamp) in milestones {
            if let Some(timestamp) = timestamp {
                self.reactivity_log
                    .record_once(session.clone(), event, timestamp, clicked_at);
            }
        }
        if let (Some(captured), Some(available)) = (
            self.view.provisional_captured_at_unix_ms,
            self.view.provisional_available_at_unix_ms,
        ) {
            let sequence = self.view.provisional_sequence.unwrap_or(0);
            self.reactivity_log.record_once(
                session.clone(),
                format!("provisional_available_sequence_{sequence}"),
                available,
                Some(captured),
            );
        }
        if let Some(rendered_at) = self.provisional_first_rendered_at_unix_ms {
            let sequence = self.view.provisional_sequence.unwrap_or(0);
            if self.provisional_render_identity
                == Some((request.job_id, request.generation, sequence))
            {
                let available = self.view.provisional_available_at_unix_ms;
                self.reactivity_log.record_once(
                    session.clone(),
                    format!("provisional_first_render_sequence_{sequence}"),
                    rendered_at,
                    available,
                );
            }
        }
    }

    fn start_import(&mut self) {
        self.compute_choice_touched = true;
        let Some((model_path, model_sha256)) = self.selected_model_artifact() else {
            self.view.message = Some("Le modèle sélectionné n'est pas disponible.".into());
            return;
        };
        let job_id = next_import_job_id();
        let generation = Generation::first();
        let config = JobConfig {
            language: selected_language(&self.selection),
            compute: compute_for_model(self.selection.model, self.compute_choice),
        };
        if self.dispatch(AppCommand::EnqueueImport {
            request: ImportRequest {
                strategy: if self.unified_candidate_enabled
                    && self.selection.model
                        == Some(whisper_core::model_catalog::ModelId::ParakeetTdt06BV3)
                {
                    whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
                } else {
                    whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1
                },
                job_id,
                generation,
                source_path: self.paths.audio.trim().to_owned(),
                source_sha256: None,
                source_samples: None,
                model_path,
                model_sha256,
                destination: self.paths.destination.trim().to_owned(),
                config,
            },
        }) {
            self.reactivity_log.record_once(
                process_log_session(),
                "first_command_accepted",
                unix_time_ms(),
                Some(self.process_started_at_unix_ms),
            );
        }
    }

    fn start_microphone_test(&mut self) {
        if !microphone_test_start_enabled(true, &self.view) {
            return;
        }
        let was_default = matches!(
            self.microphones.effective(),
            MicrophoneChoice::WindowsDefault
        );
        let _ = self.microphones.refresh();
        let Some(source_identity) = self.microphones.target_identity().map(str::to_owned) else {
            self.microphone_notice = Some(
                self.microphones
                    .target_error()
                    .unwrap_or("Identité du microphone indisponible")
                    .to_owned(),
            );
            return;
        };
        let Ok(identity) =
            whisper_core::microphone_level::MicrophoneIdentity::new(&source_identity)
        else {
            self.microphone_notice = Some("Identité du microphone invalide".into());
            return;
        };
        let gain_db = self.paths.microphone_gain(&identity).value();
        let job_id = next_import_job_id();
        let generation = Generation::first();
        if self.dispatch(AppCommand::StartMicrophoneTest {
            job_id,
            generation,
            microphone: self.microphones.capture_choice(),
            gain_db,
        }) {
            self.microphone_test_identity = Some((job_id, generation));
            self.last_test_default_identity = was_default.then_some(source_identity);
            self.confirmed_live_default_identity = None;
            self.active_identity = Some((job_id, generation));
            self.microphone_notice = None;
        }
    }

    fn set_microphone_gain(&mut self, gain_db: i8) {
        let unified_capture_identity = self.active_unified_capture_identity();
        let source_identity = if unified_capture_identity.is_some() {
            self.view.microphone_source_identity.as_deref()
        } else {
            self.view
                .microphone_source_identity
                .as_deref()
                .or_else(|| self.microphones.target_identity())
        };
        let Some(source_identity) = source_identity else {
            self.microphone_notice = Some(
                self.microphones
                    .target_error()
                    .unwrap_or("Identité du microphone indisponible")
                    .to_owned(),
            );
            return;
        };
        let Ok(identity) = whisper_core::microphone_level::MicrophoneIdentity::new(source_identity)
        else {
            self.microphone_notice = Some("Identité du microphone invalide".into());
            return;
        };
        let Ok(gain) = whisper_core::microphone_level::GainDb::new(gain_db) else {
            return;
        };
        self.paths.set_microphone_gain(&identity, gain);
        let active_capture = self.view.microphone_test_active
            || self.view.direct_capture_phase.is_some()
            || unified_capture_identity.is_some();
        if let Some((job_id, generation)) = self.active_identity
            && active_capture
            && let Some(target_identity) = if unified_capture_identity.is_some() {
                self.view.microphone_source_identity.clone()
            } else {
                self.view
                    .microphone_source_identity
                    .clone()
                    .or_else(|| self.microphones.target_identity().map(str::to_owned))
            }
        {
            self.dispatch(AppCommand::SetMicrophoneGain {
                job_id,
                generation,
                target_identity,
                expected_revision: self.view.microphone_gain_requested_revision,
                gain,
            });
        }
    }

    fn active_unified_capture_identity(&self) -> Option<(JobId, Generation)> {
        let identity = self.active_identity?;
        self.view
            .unified_parakeet_sessions
            .iter()
            .any(|session| {
                (session.job_id, session.generation) == identity
                    && session.microphone_capture
                    && matches!(
                        session.phase,
                        whisper_core::ports::UnifiedSessionPhase::Starting
                            | whisper_core::ports::UnifiedSessionPhase::Capturing
                    )
            })
            .then_some(identity)
    }

    fn stop_microphone_test(&mut self) {
        let Some((job_id, generation)) = self.microphone_test_identity else {
            return;
        };
        self.dispatch(AppCommand::StopMicrophoneTest { job_id, generation });
    }

    fn start_live(&mut self, generation: Generation) {
        if self.view.microphone_test_active {
            self.stop_microphone_test();
            self.view.message = Some(
                "Test microphone arrêté. Cliquez de nouveau pour démarrer la transcription.".into(),
            );
            return;
        }
        if !model_supports_live(self.selection.model) {
            self.view.message =
                Some("Le modèle sélectionné est qualifié uniquement pour l'import.".into());
            return;
        }
        let _ = self.microphones.refresh();
        let microphone = self.microphones.capture_choice();
        let target_identity = match &microphone {
            MicrophoneChoice::Device(identity) => Some(identity.as_str()),
            MicrophoneChoice::WindowsDefault => None,
        };
        if matches!(
            self.microphones.effective(),
            MicrophoneChoice::WindowsDefault
        ) && let (Some(previous), Some(current)) =
            (self.last_test_default_identity.as_deref(), target_identity)
            && previous != current
            && self.confirmed_live_default_identity.as_deref() != Some(current)
        {
            self.confirmed_live_default_identity = Some(current.to_owned());
            let labels = MicrophoneLabelProjection::new(self.microphones.devices());
            self.microphone_notice = Some(format!(
                "Le microphone par défaut est passé de {} à {} depuis le test. Vérifiez ce changement puis cliquez de nouveau pour démarrer.",
                labels.device_label(previous),
                labels.device_label(current)
            ));
            return;
        }
        if self.selection.model == Some(whisper_core::model_catalog::ModelId::ParakeetTdt06BV3)
            && self.selection.manual_language.is_some()
        {
            self.view.message = Some(
                "Le microphone Parakeet requiert la détection automatique de la langue.".into(),
            );
            return;
        }
        let clicked_at = unix_time_ms();
        self.live_clicked_at_unix_ms = Some(clicked_at);
        self.live_attempt_counter = self.live_attempt_counter.saturating_add(1);
        self.compute_choice_touched = true;
        let Some((model_path, model_sha256)) = self.selected_model_artifact() else {
            self.view.message = Some("Le modèle sélectionné n'est pas disponible.".into());
            return;
        };
        if generation == Generation::first() {
            // The new-recording button starts a new group. Recovery and continuation
            // use a later generation and retain the previous durable identity.
            self.live_request = None;
        }
        let serial = NEXT_JOB.fetch_add(1, Ordering::Relaxed);
        let job_id = self.live_request.as_ref().map_or_else(
            || JobId(((std::process::id() as u128) << 64) | serial as u128),
            |request| request.job_id,
        );
        let request = LiveRequest {
            job_id,
            generation,
            model_path,
            model_sha256,
            destination: self.paths.destination.trim().to_owned(),
            group_offset_samples: self.live_request.as_ref().map_or(0, |previous| {
                previous
                    .group_offset_samples
                    .saturating_add(self.view.captured_samples)
            }),
            auto_stop_after_speech_samples: None,
            microphone: microphone.clone(),
            microphone_gain_db: target_identity
                .and_then(|source_identity| {
                    whisper_core::microphone_level::MicrophoneIdentity::new(source_identity).ok()
                })
                .map(|identity| self.paths.microphone_gain(&identity).value())
                .unwrap_or(0),
            config: JobConfig {
                language: selected_language(&self.selection),
                compute: compute_for_model(self.selection.model, self.compute_choice),
            },
        };
        let session = live_log_session(job_id, generation, self.live_attempt_counter);
        self.reactivity_log
            .record_once(session, "micro_clicked", clicked_at, None);
        self.active_identity = Some((job_id, generation));
        self.live_request = Some(request.clone());
        let command = if self.unified_candidate_enabled
            && self.selection.model == Some(ModelId::ParakeetTdt06BV3)
        {
            AppCommand::StartUnifiedLiveConfigured { request }
        } else {
            AppCommand::StartDirectLiveConfigured { request }
        };
        if self.dispatch(command) {
            self.confirmed_live_default_identity = None;
            if matches!(
                self.microphones.effective(),
                MicrophoneChoice::WindowsDefault
            ) {
                self.last_test_default_identity = target_identity.map(str::to_owned);
            }
            if self.selection.model == Some(ModelId::ParakeetTdt06BV3) {
                self.selected_parakeet_capture = Some((job_id, generation));
                self.selected_capture_text = CaptureTextSelection::Confirmed;
                self.automatic_draft_cache = None;
                self.automatic_draft_error = None;
            }
            self.reactivity_log.record_once(
                process_log_session(),
                "first_command_accepted",
                unix_time_ms(),
                Some(self.process_started_at_unix_ms),
            );
        }
    }
}

impl<A: Application> eframe::App for DesktopApp<A> {
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if self.view.microphone_test_active {
            self.stop_microphone_test();
        }
        self.paths.commit_manual(PathField::Audio);
        self.paths.commit_manual(PathField::Destination);
        if let Some(inventory) = &mut self.gpu_inventory {
            inventory.close();
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.show_ui(ui);
    }
}

impl<A: Application> DesktopApp<A> {
    fn microphone_settings_ui(&mut self, ui: &mut egui::Ui, busy: bool) {
        ui.group(|ui| {
            ui.heading("Microphone");
            ui.add_enabled_ui(!busy, |ui| {
                if ui.button("Actualiser les microphones").clicked() {
                    self.refresh_microphones();
                }
                ui.horizontal(|ui| {
                    ui.label("Source microphone");
                    let mut choice = self.microphones.preferred().clone();
                    let mut choice_requested = false;
                    let selected_text = microphone_choice_label(
                        self.microphones.effective(),
                        self.microphones.devices(),
                    );
                    egui::ComboBox::from_id_salt("microphone_choice")
                        .selected_text(selected_text)
                        .show_ui(ui, |ui| {
                            choice_requested |= ui
                                .selectable_value(
                                    &mut choice,
                                    MicrophoneChoice::WindowsDefault,
                                    "Microphone par défaut de Windows",
                                )
                                .clicked();
                            for device in self.microphones.devices() {
                                ui.add_enabled_ui(device.selectable(), |ui| {
                                    choice_requested |= ui
                                        .selectable_value(
                                            &mut choice,
                                            MicrophoneChoice::Device(device.id.clone()),
                                            microphone_device_label(
                                                device,
                                                self.microphones.devices(),
                                            ),
                                        )
                                        .clicked();
                                });
                            }
                        });
                    if choice_requested {
                        match self.microphones.select(choice) {
                            Ok(()) => {
                                self.paths
                                    .set_microphone_preference(self.microphones.preferred());
                                self.microphone_notice = None;
                            }
                            Err(error) => self.microphone_notice = Some(error),
                        }
                    }
                });
                if let MicrophoneChoice::Device(_) = self.microphones.preferred()
                    && self.microphones.effective() == &MicrophoneChoice::WindowsDefault
                {
                    ui.label(
                        "Source préférée indisponible. Le microphone par défaut de Windows est utilisé.",
                    );
                }
                if self.microphones.devices().is_empty() && !self.microphones.list_failed() {
                    ui.label("Aucun microphone d'entrée disponible.");
                }
                for device in self.microphones.devices() {
                    if let Some(reason) = &device.unavailable_reason {
                        ui.label(format!("{} : {reason}", device.name));
                    }
                }
            });

            let identified_target = self.microphones.target_identity().is_some();
            let button = if self.view.microphone_test_active {
                let response = ui.button("Arrêter le test microphone");
                if response.clicked() {
                    self.stop_microphone_test();
                }
                response
            } else {
                let response = ui.add_enabled(
                    microphone_test_start_enabled(identified_target, &self.view),
                    egui::Button::new("Tester le microphone"),
                );
                if response.clicked() {
                    self.start_microphone_test();
                }
                response
            };
            #[cfg(test)]
            {
                self.microphone_test_button_rect = Some(button.rect);
            }
            #[cfg(not(test))]
            let _ = button;

            if let Some(actual) = self.view.microphone_source_identity.as_deref() {
                let line =
                    MicrophoneLabelProjection::new(self.microphones.devices()).actual_line(actual);
                ui.label(line);
            } else if let Some(expected) = self.microphones.target_identity() {
                let line = MicrophoneLabelProjection::new(self.microphones.devices())
                    .planned_line(expected);
                ui.label(line);
            }
            let meter_state = microphone_meter_presentation(
                self.view.microphone_meter.as_ref(),
                self.active_identity,
                whisper_core::microphone_level::monotonic_millis(),
            );
            let saturated = microphone_saturation_is_current(
                self.view.microphone_meter.as_ref(),
                self.active_identity,
                self.view.microphone_source_identity.as_deref(),
                self.view.microphone_test_active
                    || self.view.direct_capture_phase.is_some_and(|phase| {
                        matches!(
                            phase,
                            whisper_core::application::DirectCapturePhase::Initializing
                                | whisper_core::application::DirectCapturePhase::Active
                        )
                    }),
                whisper_core::microphone_level::monotonic_millis(),
            );
            #[cfg(test)]
            {
                self.microphone_saturation_badge_rect = None;
            }
            match meter_state {
                MicrophoneMeterPresentation::Awaiting if self.view.microphone_test_active => {
                    ui.label("En attente de mesures microphone…");
                }
                MicrophoneMeterPresentation::Awaiting => {}
                MicrophoneMeterPresentation::Stale => {
                    ui.colored_label(egui::Color32::YELLOW, "Jauge périmée");
                }
                MicrophoneMeterPresentation::WrongCapture => {
                    ui.colored_label(egui::Color32::YELLOW, "Mesure d'une ancienne capture ignorée");
                }
                MicrophoneMeterPresentation::Absent => {
                    ui.label("Mesure microphone indisponible");
                }
                MicrophoneMeterPresentation::Silence => {
                    ui.label("Silence détecté");
                }
                MicrophoneMeterPresentation::Signal { rms_dbfs, peak_dbfs } => {
                    ui.label(format!("Niveau micro — RMS {rms_dbfs:.1} dBFS, pic {peak_dbfs:.1} dBFS"));
                }
            }
            if saturated {
                let response = ui.colored_label(
                    MICROPHONE_SATURATION_COLOR,
                    MICROPHONE_SATURATION_LABEL,
                );
                #[cfg(test)]
                {
                    self.microphone_saturation_badge_rect = Some(response.rect);
                }
            }
            if self.view.microphone_acquisition.is_some() {
                ui.separator();
                ui.label("Spectre en direct — échelle commune de −120 à 0 dBFS/Hz");
                if let Some(spectrum) = self.view.microphone_spectrum.as_ref() {
                    let age = whisper_core::microphone_level::monotonic_millis()
                        .saturating_sub(spectrum.captured_at_monotonic_ms);
                    if age <= 500 {
                        paint_microphone_spectrum(ui, "Avant gain", &spectrum.pre_gain, spectrum.sample_rate_hz);
                        paint_microphone_spectrum(ui, "Après gain et limiteur", &spectrum.post_gain_limiter, spectrum.sample_rate_hz);
                        if spectrum.mixed_gain {
                            ui.label(format!("Fenêtre FFT avec gain variable : {} → {} dB", spectrum.gain_first_db, spectrum.gain_last_db));
                        }
                    } else {
                        ui.colored_label(egui::Color32::YELLOW, "Spectre périmé ; attente d’une nouvelle fenêtre audio");
                    }
                } else {
                    ui.label("En attente de 2048 trames audio pour calculer le spectre…");
                }
            }
            let unified_capture_active = self.active_unified_capture_identity().is_some();
            let gain_identity = if unified_capture_active {
                self.view.microphone_source_identity.as_deref()
            } else {
                self.view
                    .microphone_source_identity
                    .as_deref()
                    .or_else(|| self.microphones.target_identity())
            };
            if let Some(gain_identity) = gain_identity
                && let Ok(identity) =
                    whisper_core::microphone_level::MicrophoneIdentity::new(gain_identity)
            {
                let enabled = (self.view.microphone_test_active
                    || self.view.direct_capture_phase.is_some()
                    || unified_capture_active
                    || self.view.active_job.is_none())
                    && (self.view.microphone_source_identity.is_some()
                        || (!unified_capture_active
                            && self.microphones.target_identity().is_some()));
                let mut gain = if self.view.microphone_test_active
                    || self.view.direct_capture_phase.is_some()
                    || unified_capture_active
                {
                    self.view.microphone_gain_requested_db
                } else {
                    self.paths.microphone_gain(&identity).value()
                };
                let response = ui.add_enabled(
                    enabled,
                    egui::Slider::new(&mut gain, -24..=24).text("Gain microphone (dB)"),
                );
                #[cfg(test)]
                {
                    self.microphone_gain_slider_enabled = Some(response.enabled());
                }
                if response.changed() {
                    self.set_microphone_gain(gain);
                }
            } else {
                ui.label("Gain microphone indisponible : cible non identifiée.");
            }
        });
    }

    fn show_ui(&mut self, ui: &mut egui::Ui) {
        if self.view.microphone_acquisition.is_some() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(50));
        }
        self.reactivity_log.record_once(
            process_log_session(),
            "first_ui_callback",
            unix_time_ms(),
            Some(self.process_started_at_unix_ms),
        );
        self.poll_model_selection();
        if self.startup_preferences_loaded || self.compute_choice_touched {
            self.poll_gpu_probe();
        }
        if self.catalog_receiver.is_some() || self.preference_events.is_some() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(50));
        }
        if let Some(inventory) = &mut self.gpu_inventory {
            inventory.poll();
            if matches!(inventory.state(), GpuInventoryState::Loading) {
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(33));
            }
        }
        if self.gpu_probe.is_some() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(33));
        }
        if let Some(message) = self.paths.poll_notice() {
            self.path_notice = Some(message);
        }
        if self.paths.has_pending_save() {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(100));
        }
        self.dispatch(AppCommand::Refresh);
        self.record_reactivity_milestones();
        if let (Some((job_id, _)), Some(generation)) =
            (self.view.active_job, self.view.active_generation)
            && self
                .active_identity
                .is_some_and(|(active_job, _)| active_job == job_id)
        {
            self.active_identity = Some((job_id, generation));
        }
        let state = self.view.active_job.map(|(_, state)| state);
        if state != self.observed_job_state
            && state.is_some_and(|state| {
                matches!(
                    state,
                    JobState::Complete | JobState::Recoverable | JobState::Cancelled
                )
            })
        {
            self.scanned_destination = None;
        }
        self.observed_job_state = state;
        let unified_busy = self.view.unified_parakeet_sessions.iter().any(|session| {
            matches!(
                session.phase,
                whisper_core::ports::UnifiedSessionPhase::Starting
                    | whisper_core::ports::UnifiedSessionPhase::Capturing
                    | whisper_core::ports::UnifiedSessionPhase::Finalizing
                    | whisper_core::ports::UnifiedSessionPhase::Publishing
            )
        });
        let busy = self.view.microphone_test_active
            || unified_busy
            || (self.view.close_requested && !self.view.close_ready)
            || self
                .view
                .active_job
                .is_some_and(|(_, state)| window_must_remain_open(state));
        let can_enqueue = import_enqueue_enabled(busy);
        if ui.ctx().input(|input| input.viewport().close_requested()) && busy {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::CancelClose);
            if !self.quit_pending && self.request_quit() {
                self.quit_pending = true;
                self.view.message = Some(
                    "Fermeture en attente de l’arrêt coopératif et de la stabilisation du worker."
                        .into(),
                );
            }
        }
        let destination = self.paths.destination.trim().to_owned();
        let destination_scan_pending = !destination.is_empty()
            && self.scanned_destination.as_deref() != Some(destination.as_str());
        // Refresh consumes asynchronous scan and lifecycle events. Schedule a
        // wake for the initial destination scan too: it is submitted later in
        // this frame, while the first-frame queue view is still empty.
        if busy
            || !self.view.queue.is_empty()
            || destination_scan_pending
            || self.view.history_scan_pending
            || self.view.queue_scan_pending
        {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(33));
        }
        if !busy {
            self.active_identity = None;
            if self.quit_pending && self.view.close_ready {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        egui::ScrollArea::vertical()
            .id_salt("desktop_main_page")
            .auto_shrink([false, false])
            .show(ui, |ui| {
        ui.heading("Whisper — transcription locale");
        ui.label("Import WAV/MP3 ou micro en direct, avec le modèle local.");

        ui.add_enabled_ui(can_enqueue, |ui| {
            let path_field_width = (ui.available_width() - 320.0).clamp(100.0, 640.0);
            let mut model_chosen = false;
            egui::Grid::new("path_fields").show(ui, |ui| {
                ui.label("Fichier audio (WAV ou MP3)");
                let response = path_text_edit(ui, &mut self.paths.audio, path_field_width);
                if ui.button("Parcourir…").clicked() {
                    self.paths.browse(PathField::Audio);
                } else if response.lost_focus()
                    || (response.has_focus()
                        && ui.input(|input| input.key_pressed(egui::Key::Enter)))
                {
                    self.paths.commit_manual(PathField::Audio);
                }
                ui.end_row();

                ui.label("Modèle local");
                if let Some(projection) = &self.projection {
                    model_chosen = model_selection_combo(ui, projection, &mut self.selection);
                } else {
                    ui.label("Chargement des modèles…");
                }
                ui.end_row();

                ui.label("Dossier des transcriptions");
                let response = path_text_edit(ui, &mut self.paths.destination, path_field_width);
                if ui.button("Parcourir…").clicked() {
                    self.paths.browse(PathField::Destination);
                } else if response.lost_focus()
                    || (response.has_focus()
                        && ui.input(|input| input.key_pressed(egui::Key::Enter)))
                {
                    self.paths.commit_manual(PathField::Destination);
                }
                ui.end_row();
            });
            if model_chosen
                && self.selection.model
                    == Some(whisper_core::model_catalog::ModelId::ParakeetTdt06BV3)
            {
                self.selection.manual_language = None;
            }
            if model_chosen
                && let (Some(model), Some(sender)) =
                    (self.selection.model, self.preference_commands.as_ref())
            {
                self.selection_touched = true;
                record_preference_delivery(
                    &mut self.preference_notice,
                    sender.send(ModelPreferenceCommand::ChooseModel(model)),
                );
                self.selection_notice = None;
            }
            ui.horizontal(|ui| {
                ui.label("Langue");
                if let Some(projection) = &self.projection {
                    language_selection_combo(
                        ui,
                        projection,
                        compute_for_model(self.selection.model, self.compute_choice),
                        &mut self.selection,
                    );
                } else {
                    ui.label("Chargement…");
                }
                ui.label("Calcul");
                let parakeet_selected = self.selected_model_id()
                    == Some(whisper_core::model_catalog::ModelId::ParakeetTdt06BV3);
                let previous_compute = self.compute_choice;
                let mut compute_interacted = false;
                if parakeet_selected {
                    let gpu_selectable = self.projection.as_ref().is_some_and(|projection| {
                        projection
                            .models
                            .iter()
                            .find(|model| {
                                model.id == whisper_core::model_catalog::ModelId::ParakeetTdt06BV3
                            })
                            .is_some_and(|model| {
                                model.configurations.iter().any(|configuration| {
                                    configuration.key.backend
                                        == whisper_core::model_catalog::Backend::Gpu
                                        && configuration.selectable
                                        && configuration.auto_language_selectable
                                })
                            })
                    }) && self
                        .catalog
                        .as_ref()
                        .is_some_and(parakeet_gpu_catalog_ready);
                    egui::ComboBox::from_id_salt("compute_choice")
                        .selected_text(match self.compute_choice {
                            ComputeChoice::Cpu => "CPU",
                            ComputeChoice::Gpu => "GPU",
                            ComputeChoice::Auto => "Auto",
                        })
                        .show_ui(ui, |ui| {
                            compute_interacted |= ui
                                .selectable_value(
                                    &mut self.compute_choice,
                                    ComputeChoice::Auto,
                                    "Auto",
                                )
                                .clicked();
                            compute_interacted |= ui
                                .selectable_value(
                                    &mut self.compute_choice,
                                    ComputeChoice::Cpu,
                                    "CPU",
                                )
                                .clicked();
                            let gpu_response = ui.add_enabled_ui(gpu_selectable, |ui| {
                                ui.selectable_value(
                                    &mut self.compute_choice,
                                    ComputeChoice::Gpu,
                                    "GPU",
                                )
                            });
                            compute_interacted |= gpu_response.inner.clicked();
                        });
                } else {
                    egui::ComboBox::from_id_salt("compute_choice")
                        .selected_text(match self.compute_choice {
                            ComputeChoice::Cpu => "CPU",
                            ComputeChoice::Gpu => "GPU",
                            ComputeChoice::Auto => "Auto",
                        })
                        .show_ui(ui, |ui| {
                            compute_interacted |= ui
                                .selectable_value(
                                    &mut self.compute_choice,
                                    ComputeChoice::Auto,
                                    "Auto",
                                )
                                .clicked();
                            compute_interacted |= ui
                                .selectable_value(
                                    &mut self.compute_choice,
                                    ComputeChoice::Cpu,
                                    "CPU",
                                )
                                .clicked();
                            compute_interacted |= ui
                                .selectable_value(
                                    &mut self.compute_choice,
                                    ComputeChoice::Gpu,
                                    "GPU",
                                )
                                .clicked();
                        });
                }
                let compute_changed = self.compute_choice != previous_compute;
                let compute_selection_interacted = record_compute_selection_interaction(
                    &mut self.compute_choice_touched,
                    compute_interacted,
                    compute_changed,
                );
                if compute_selection_interacted && let Some(sender) = &self.preference_commands {
                    let preference = if self.compute_choice == ComputeChoice::Auto {
                        whisper_core::path_settings::StartupComputePreference::Auto
                    } else {
                        whisper_core::path_settings::StartupComputePreference::FollowProbe
                    };
                    record_preference_delivery(
                        &mut self.preference_notice,
                        sender.send(ModelPreferenceCommand::ChooseStartupCompute(preference)),
                    );
                }
            });
            if let Some(notice) = &self.selection_notice {
                ui.colored_label(egui::Color32::YELLOW, notice);
            }
            if let Some(notice) = &self.preference_load_notice {
                ui.label(notice);
            }
            if let Some(notice) = &self.preference_notice {
                ui.label(notice);
            }
            if let Some(projection) = &self.projection
                && let Some(model) = projection
                    .models
                    .iter()
                    .find(|model| Some(model.id) == self.selection.model)
                && let Some(cause) = model.cause
            {
                ui.label(format!("{} : indisponible ({cause:?})", model.display_name));
            }
            let gpu_preferences = self
                .gpu_inventory
                .as_ref()
                .map(|inventory| qualified_devices(inventory.qualified_state()))
                .unwrap_or_default();
            let mut preferred_gpu = self.paths.gpu_preference().map(str::to_owned);
            gpu_preference_combo(ui, &gpu_preferences, &mut preferred_gpu);
            if preferred_gpu.as_deref() != self.paths.gpu_preference() {
                self.paths.set_gpu_preference(preferred_gpu);
            }
            if let Some(inventory) = &self.gpu_inventory {
                let resolution =
                    inventory.resolve_selection(self.compute_choice, self.paths.gpu_preference());
                ui.label(gpu_selection_resolution_label(
                    &resolution,
                    &gpu_preferences,
                ));
            }
        });
        self.microphone_settings_ui(ui, busy);
        ui.add_enabled_ui(can_enqueue, |ui| {
            if !IMPORT_CPU_MVP {
                ui.add_enabled_ui(!busy, |ui| {
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.auto_stop_enabled, "Arrêt après durée de parole");
                        if self.auto_stop_enabled {
                            ui.add(
                                egui::DragValue::new(&mut self.auto_stop_minutes)
                                    .range(1..=u64::MAX)
                                    .suffix(" min"),
                            );
                        } else {
                            ui.label("sans limite");
                        }
                    });
                });
            }
            let can_start = self.selected_model_artifact().is_some()
                && !self.paths.audio.trim().is_empty()
                && !self.paths.destination.trim().is_empty()
                && self
                    .selection
                    .manual_language
                    .as_ref()
                    .is_none_or(|language| !language.is_empty());
            if ui
                .add_enabled(can_start, egui::Button::new("Importer et transcrire"))
                .clicked()
            {
                self.start_import();
            }
        });

        let unified_candidate_selected = self.unified_candidate_enabled
            && self.selection.model == Some(ModelId::ParakeetTdt06BV3);
        let can_start_live = (if unified_candidate_selected {
            unified_live_admission_available(&self.view)
        } else {
            !busy || self.view.microphone_test_active
        }) && model_supports_live(self.selection.model)
            && (self.selection.model != Some(ModelId::ParakeetTdt06BV3)
                || self.selection.manual_language.is_none())
            && self.selected_model_artifact().is_some()
            && !self.paths.destination.trim().is_empty()
            && self
                .selection
                .manual_language
                .as_ref()
                .is_none_or(|language| !language.is_empty());
        let response = ui.add_enabled(can_start_live, egui::Button::new("Démarrer le micro"));
        #[cfg(test)]
        {
            self.live_start_button_rect = Some(response.rect);
            self.live_start_button_enabled = Some(response.enabled());
        }
        if response.clicked() {
            self.start_live(Generation::first());
        }
        if !model_supports_live(self.selection.model) {
            ui.label("Le microphone n'est pas qualifié pour ce modèle.");
        } else if self.selection.model == Some(ModelId::ParakeetTdt06BV3)
            && self.selection.manual_language.is_some()
        {
            ui.label("Le microphone Parakeet requiert la détection automatique de la langue.");
        }

        if let Some(prompt) = self.view.duplicate_import_prompt.clone() {
            egui::Modal::new(egui::Id::new("duplicate_import_confirmation")).show(ui.ctx(), |ui| {
                ui.set_min_width(360.0);
                if prompt.resume_from.is_some() {
                    ui.heading("Reprise de publication interrompue");
                    ui.label(format!(
                        "{} transcription(s) antérieure(s) sont conservées. Confirmez explicitement le nouveau nom avant de reprendre.",
                        prompt.publications.len()
                    ));
                } else {
                    ui.heading("Audio déjà traité");
                    ui.label(format!(
                        "Ce contenu audio possède {} transcription(s) dans la destination choisie.",
                        prompt.publications.len()
                    ));
                }
                ui.label("Créer un nouveau enregistre une paire TXT/SRT dans un nouveau dossier de transcription, sans copie dans exports/.");
                if prompt.resume_from.is_some() && !prompt.export_basename.is_empty() {
                    ui.label(format!(
                        "Nom proposé pour le remplacement : {}.txt / {}.srt",
                        prompt.export_basename, prompt.export_basename
                    ));
                }
                if let Some(notice) = &prompt.revalidation_notice {
                    ui.colored_label(egui::Color32::YELLOW, notice);
                }
                if prompt.resume_from.is_some() {
                    ui.horizontal(|ui| {
                        if ui.button("Reprendre et remplacer les traitements existants").clicked() {
                            self.dispatch(AppCommand::ResolveDuplicateImport {
                                prompt: prompt.clone(),
                                choice: DuplicateImportChoice::Replace,
                            });
                        }
                        if ui.button("Reprendre sous un nouveau nom").clicked() {
                            self.dispatch(AppCommand::ResolveDuplicateImport {
                                prompt: prompt.clone(),
                                choice: DuplicateImportChoice::CreateNew,
                            });
                        }
                    });
                } else {
                    let verified_count = prompt.publications.iter().filter(|item| item.verified_present).count();
                    let latest_available = verified_count == 1
                        || (verified_count > 1 && prompt.publications.iter().any(|item| item.verified_present && item.success_ordinal.is_some()));
                    ui.label("Traitements associés à ce fichier audio :");
                    for publication in &prompt.publications {
                        ui.label(format!(
                            "{} — job {:032x}, génération {} — {} — {}",
                            if publication.verified_present { "Présent" } else { "Indisponible / non vérifié" },
                            publication.job_id.0,
                            publication.generation.get(),
                            if publication.active { "actif" } else { "remplacé" },
                            publication.success_ordinal.map_or_else(|| "ordre historique inconnu".to_owned(), |ordinal| format!("succès #{ordinal}")),
                        ));
                    }
                    ui.horizontal(|ui| {
                        if ui.add_enabled(latest_available, egui::Button::new("Remplacer le dernier traitement")).clicked() {
                            self.dispatch(AppCommand::ResolveDuplicateImport {
                                prompt: prompt.clone(),
                                choice: DuplicateImportChoice::ReplaceLatest,
                            });
                        }
                        if ui.add_enabled(verified_count > 0, egui::Button::new("Remplacer tous les traitements")).clicked() {
                            self.dispatch(AppCommand::ResolveDuplicateImport {
                                prompt: prompt.clone(),
                                choice: DuplicateImportChoice::ReplaceAll,
                            });
                        }
                        if ui.add_enabled(verified_count > 0, egui::Button::new("Parcourir…")).clicked()
                            && let Some(selected_path) = whisper_adapters::archive::ArchiveStore::browse_treatment_folder(std::path::Path::new(&prompt.request.destination))
                        {
                            self.dispatch(AppCommand::ResolveDuplicateImport {
                                prompt: prompt.clone(),
                                choice: DuplicateImportChoice::ReplaceSelectedPath(selected_path),
                            });
                        }
                        if ui.button("Créer un nouveau").clicked() {
                            self.dispatch(AppCommand::ResolveDuplicateImport {
                                prompt: prompt.clone(),
                                choice: DuplicateImportChoice::CreateNew,
                            });
                        }
                    });
                }
                if ui.button("Annuler").clicked() {
                    self.dispatch(AppCommand::CancelDuplicateImport);
                }
            });
        }

        if let Some(command) = source_snapshot_recovery_widget(
            ui,
            self.view.source_snapshot_recovery_available,
            self.view.source_snapshot_recovery.as_deref(),
        ) {
            self.dispatch(command);
        }

        let micro_active = self.view.active_job.is_some_and(|(job_id, _)| {
            self.live_request
                .as_ref()
                .is_some_and(|request| request.job_id == job_id)
        });
        if micro_active
            && state.is_some_and(|state| matches!(state, JobState::Draining | JobState::Finalizing))
        {
            ui.add_enabled(false, egui::Button::new("Arrêt en cours"));
        } else if state.is_some_and(manual_stop_available) && self.active_identity.is_some() {
            let label = if micro_active {
                "Arrêter le micro"
            } else {
                "Arrêter"
            };
            let stop_button = ui.button(label);
            #[cfg(test)]
            {
                self.live_stop_button_rect = Some(stop_button.rect);
            }
            if stop_button.clicked() {
                self.stop_active();
            }
        }

        if let Some((job_id, state)) = self.view.active_job {
            ui.separator();
            let source = if self
                .live_request
                .as_ref()
                .is_some_and(|request| request.job_id == job_id)
            {
                "Micro"
            } else {
                "Import"
            };
            ui.label(format!("{source} {job_id:?} — {state:?}"));
            if (!IMPORT_CPU_MVP || self.view.provisional_session_id.is_some())
                && self.live_request.is_some()
            {
                ui.label(format!(
                    "Micro — capturés {} | admis {} | audio durable {} | fragment confirmé {} | parole {} échantillons",
                    self.view.captured_samples,
                    self.view.admitted_samples,
                    self.view.audio_durable_samples,
                    self.view.confirmed_fragment_samples,
                    self.view.speech_samples,
                ));
                if self.view.provisional_session_id.is_none()
                    && let Some(text) = self.view.provisional_text.as_deref()
                {
                    let render_identity = (
                        job_id,
                        self.live_request
                            .as_ref()
                            .map_or(Generation::first(), |request| request.generation),
                        self.view.provisional_sequence.unwrap_or(0),
                    );
                    let text_response = ui.label(format!("Texte provisoire : {text}"));
                    record_first_render(
                        render_identity,
                        &text_response,
                        &mut self.provisional_render_identity,
                        &mut self.provisional_first_rendered_at_unix_ms,
                    );
                    if let Some(range) = self.view.provisional_range {
                        ui.small(format!(
                            "Hypothèse remplaçable — {:.2} à {:.2} s ; non exportée comme transcription confirmée",
                            range.start_sample as f64 / f64::from(range.sample_rate_hz),
                            range.end_sample as f64 / f64::from(range.sample_rate_hz),
                        ));
                    }
                }
            }
            if self
                .live_request
                .as_ref()
                .is_some_and(|request| request.job_id == job_id)
            {
                if matches!(
                    state,
                    JobState::Preparing
                        | JobState::Capturing
                        | JobState::Draining
                        | JobState::Finalizing
                ) {
                    ui.label("Transcription en cours…");
                }
                if self.view.provisional_session_id.is_none()
                    && let Some(text) = self.view.confirmed_text.as_deref()
                {
                    ui.label("Texte confirmé :");
                    let mut text_was_visible = false;
                    egui::ScrollArea::vertical()
                        .max_height(180.0)
                        .show(ui, |ui| {
                            let response = ui.label(text);
                            text_was_visible =
                                response.rect.width() > 0.0 && response.rect.height() > 0.0;
                        });
                    if text_was_visible && let Some(request) = self.live_request.as_ref() {
                        self.reactivity_log.record_once(
                            live_log_session(
                                request.job_id,
                                request.generation,
                                self.live_attempt_counter,
                            ),
                            "first_confirmed_text_displayed",
                            unix_time_ms(),
                            self.live_clicked_at_unix_ms,
                        );
                    }
                }
            }
            if let Some((completed, total)) = self.view.progress {
                if total > 0 {
                    ui.add(
                        egui::ProgressBar::new((completed as f32 / total as f32).clamp(0.0, 1.0))
                            .text(format!("{completed} / {total} échantillons")),
                    );
                } else {
                    ui.label(format!(
                        "Progression : {completed} échantillons PCM traités"
                    ));
                }
            }
            if !IMPORT_CPU_MVP
                && state == JobState::AwaitingChoice
                && ui.button("Terminer ce passage sur CPU").clicked()
            {
                self.dispatch(AppCommand::FinishLiveOnCpu {
                    job_id,
                    generation: self.view.active_generation.unwrap_or(Generation::first()),
                });
            }
        }
        self.render_unified_parakeet_ui(ui);
        self.render_parakeet_capture_ui(ui);
        if let Some(message) = &self.view.message {
            let visible_message = if message.starts_with("ModelMissing:") {
                "Modèle local introuvable. Vérifiez le chemin dans « Modèle local ». Pour cet import déjà en file, choisissez « Retirer », puis ajoutez de nouveau le fichier audio."
            } else {
                message
            };
            let color = if visible_message.contains("CPU prend le relais") {
                egui::Color32::YELLOW
            } else {
                egui::Color32::LIGHT_RED
            };
            ui.colored_label(color, visible_message);
        }
        if let Some(message) = &self.startup_message {
            ui.colored_label(egui::Color32::LIGHT_RED, message);
        }
        if let Some(inventory) = &self.gpu_inventory {
            gpu_inventory_status(ui, inventory.state());
            gpu_qualified_status(ui, inventory.qualified_state());
        }
        if let Some(message) = &self.path_notice {
            ui.colored_label(egui::Color32::YELLOW, message);
        }
        let save_status = path_save_status_label(self.paths.save_status());
        if !save_status.is_empty() {
            ui.label(save_status);
        }
        if let Some(message) = &self.microphone_notice {
            ui.colored_label(egui::Color32::YELLOW, message);
        }
        if self.view.diagnostics_degraded {
            ui.colored_label(
                egui::Color32::LIGHT_RED,
                "Les diagnostics locaux sont indisponibles ; l’opération continue.",
            );
        }
        if let Some(result) = self.published_result() {
            ui.colored_label(egui::Color32::LIGHT_GREEN, result);
        }

        ui.separator();
        ui.heading(format!(
            "File d’import — {} élément(s)",
            self.view.queue.len()
        ));
        let queued = self.view.queue.clone();
        for entry in queued {
            let identity = (entry.request.job_id, entry.request.generation);
            ui.horizontal(|ui| {
                ui.label(format!(
                    "{:?} — {}",
                    entry.status, entry.request.source_path
                ));
                if matches!(
                    entry.status,
                    whisper_core::ports::ImportQueueStatus::Queued
                        | whisper_core::ports::ImportQueueStatus::AwaitingChoice
                ) && ui
                    .add_enabled(!busy, egui::Button::new("Traiter"))
                    .clicked()
                {
                    self.active_identity = Some(identity);
                    let request = queued_request_for_compute(&entry.request, self.compute_choice);
                    self.dispatch(AppCommand::ProcessQueued { request });
                }
                if entry.status == whisper_core::ports::ImportQueueStatus::Interrupted {
                    if self.resume_confirmation == Some(identity) {
                        if ui
                            .add_enabled(!busy, egui::Button::new("Confirmer la reprise"))
                            .clicked()
                        {
                            self.active_identity = Some((
                                entry.request.job_id,
                                entry
                                    .request
                                    .generation
                                    .next()
                                    .unwrap_or(entry.request.generation),
                            ));
                            self.resume_confirmation = None;
                            self.dispatch(AppCommand::ResumeInterrupted {
                                request: entry.request.clone(),
                            });
                        }
                        if ui.button("Annuler").clicked() {
                            self.resume_confirmation = None;
                        }
                    } else if ui
                        .add_enabled(!busy, egui::Button::new("Reprendre…"))
                        .clicked()
                    {
                        self.resume_confirmation = Some(identity);
                    }
                }
                let removable = entry.status != whisper_core::ports::ImportQueueStatus::Running;
                if ui
                    .add_enabled(removable, egui::Button::new("Retirer"))
                    .clicked()
                {
                    self.dispatch(AppCommand::RemoveQueued {
                        job_id: entry.request.job_id,
                        generation: entry.request.generation,
                    });
                }
            });
            if self.resume_confirmation == Some(identity) {
                ui.label(
                    "Le calcul repart du début et vérifie chaque segment déjà confirmé avant de continuer.",
                );
            }
        }

        if destination_scan_pending {
            self.scanned_destination = Some(destination.clone());
            self.dispatch(AppCommand::ScanHistory {
                destination: destination.clone(),
            });
        }
        ui.separator();
        ui.heading("Historique local");
        for cleanup in self.view.replacement_cleanups.clone() {
            ui.group(|ui| {
                ui.label(format!(
                    "Le nouveau traitement job {:032x}, génération {} est publié. Des anciens éléments restent à vérifier ou à retirer.",
                    cleanup.successor.job_id.0,
                    cleanup.successor.generation.get(),
                ));
                for path in &cleanup.remaining {
                    ui.label(path);
                }
                for path in &cleanup.preserved {
                    ui.label(format!("Conservé car sa propriété n'est pas vérifiée : {path}"));
                }
                if let Some(error) = &cleanup.last_error {
                    ui.colored_label(egui::Color32::YELLOW, format!("Nettoyage interrompu : {error}"));
                }
                if cleanup.cleanup_id != "unavailable"
                    && !busy
                    && !cleanup.remaining.is_empty()
                    && ui.button("Reprendre le retrait des anciens traitements").clicked()
                {
                    self.dispatch(AppCommand::ResumeReplacementCleanup {
                        destination: destination.clone(),
                        cleanup_id: cleanup.cleanup_id.clone(),
                    });
                }
            });
        }
        if self.view.history.is_empty() {
            ui.label("Aucune transcription publiée dans ce dossier.");
        }
        let history = self.view.history.clone();
        for item in history {
            ui.label(format!(
                "{} — job {:?}, génération {} — source {}",
                if item.complete {
                    "Terminée"
                } else {
                    "Récupérable"
                },
                item.job_id,
                item.generation.get(),
                item.source_sha256
            ));
            if let Some(diagnostic) = &item.diagnostic {
                if let Some(base) = diagnostic.strip_prefix("Export: ") {
                    ui.label(format!(
                        "TXT/SRT : exports/{base}.txt et exports/{base}.srt"
                    ));
                } else if diagnostic.starts_with("Recoverable: publication R11") {
                    ui.label(diagnostic);
                    if !busy
                        && ui
                            .button("Reprendre l'activation de cette transcription")
                            .clicked()
                    {
                        self.dispatch(AppCommand::ResolveImportPublication {
                            destination: destination.clone(),
                            job_id: item.job_id,
                            generation: item.generation,
                        });
                    }
                } else {
                    ui.colored_label(egui::Color32::LIGHT_RED, diagnostic);
                }
            }
            if let Some(recovery) = item.live_recovery {
                ui.label(format!(
                    "Audio durable : {} échantillons ; audio confirmé : {} échantillons.",
                    recovery.durable_samples, recovery.confirmed_samples
                ));
                if recovery.confirmed_samples < recovery.durable_samples {
                    ui.colored_label(
                        egui::Color32::LIGHT_RED,
                        format!(
                            "{} échantillons durables restent non confirmés ; le nouveau passage attend la reprise du passage en cours.",
                            recovery.durable_samples - recovery.confirmed_samples
                        ),
                    );
                } else if recovery.confirmed_samples > recovery.durable_samples {
                    ui.colored_label(
                        egui::Color32::LIGHT_RED,
                        "Couverture confirmée incohérente avec l'audio durable ; reprise indisponible.",
                    );
                } else if recovery.request.destination.trim() != destination {
                    ui.colored_label(
                        egui::Color32::LIGHT_RED,
                        "La destination enregistrée diffère du dossier consulté ; reprise indisponible.",
                    );
                } else if recovery.publication_ready
                    && recovery.request.destination.trim() != destination
                {
                    ui.colored_label(
                        egui::Color32::LIGHT_RED,
                        "La destination enregistrée diffère du dossier consulté ; publication indisponible.",
                    );
                } else if !IMPORT_CPU_MVP
                    && recovery.publication_ready
                    && !busy
                    && self.view.active_job.is_none()
                    && ui
                        .button("Publier explicitement le passage récupérable")
                        .clicked()
                {
                    self.dispatch(AppCommand::ResolveLivePublication {
                        request: recovery.request,
                    });
                } else if !recovery.publication_ready {
                    ui.colored_label(
                        egui::Color32::YELLOW,
                        format!(
                            "Le passage {} reste récupérable ; réconciliez son point durable avant de démarrer le passage suivant.",
                            recovery.request.generation.get()
                        ),
                    );
                }
            }
        }
            });
    }
}

/// Renders the explicit abandoned-snapshot recovery flow. Inspection never
/// mutates a reservation; only the separate confirmation command can do so.
pub fn source_snapshot_recovery_widget(
    ui: &mut egui::Ui,
    available: bool,
    candidates: Option<&[whisper_core::ports::SourceSnapshotRecoveryCandidate]>,
) -> Option<AppCommand> {
    if available && ui.button("Vérifier les copies audio retenues").clicked() {
        return Some(AppCommand::InspectSourceSnapshotRecovery);
    }
    let candidates = candidates?;
    if candidates.is_empty() {
        return None;
    }
    let mut command = None;
    egui::Modal::new(egui::Id::new("source_snapshot_recovery_confirmation")).show(ui.ctx(), |ui| {
        ui.set_min_width(420.0);
        ui.heading("Copies audio retenues");
        ui.label("L’inspection ne libère aucune place. Les traitements actifs, en file ou ambigus restent protégés.");
        for candidate in candidates {
            ui.separator();
            ui.label(format!("Job {:032x}, génération {} — {}", candidate.job_id.0, candidate.generation.get(), candidate.destination));
            if candidate.recoverable {
                ui.colored_label(egui::Color32::LIGHT_GREEN, candidate.reason.as_deref().unwrap_or("Récupérable après confirmation"));
            } else {
                ui.colored_label(egui::Color32::YELLOW, candidate.reason.as_deref().unwrap_or("Protégée"));
            }
        }
        let can_recover = candidates.iter().any(|candidate| candidate.recoverable);
        ui.horizontal(|ui| {
            if ui.add_enabled(can_recover, egui::Button::new("Confirmer la récupération des copies sûres")).clicked() {
                command = Some(AppCommand::ConfirmSourceSnapshotRecovery);
            }
            if ui.button("Annuler").clicked() {
                command = Some(AppCommand::CancelSourceSnapshotRecovery);
            }
        });
    });
    command
}

fn parakeet_gpu_catalog_ready(catalog: &CatalogSnapshot) -> bool {
    catalog.entries.iter().any(|entry| {
        entry.id == ModelId::ParakeetTdt06BV3
            && entry.availability == whisper_core::model_catalog::ModelAvailability::Ready
            && !entry
                .diagnostic
                .as_deref()
                .is_some_and(|message| message.starts_with("GPU indisponible :"))
    })
}

pub fn record_compute_selection_interaction(
    touched: &mut bool,
    interacted: bool,
    changed: bool,
) -> bool {
    let selected = interacted || changed;
    *touched |= selected;
    selected
}

pub fn record_preference_delivery(notice: &mut Option<String>, result: Result<(), ()>) {
    if result.is_err() {
        *notice = Some("Préférence non enregistrée : service de préférences indisponible.".into());
    }
}

pub fn gpu_qualified_status(ui: &mut egui::Ui, state: &GpuQualifiedInventoryState) {
    use whisper_core::gpu_inventory::GpuCatalogueCoverage;
    match state {
        GpuQualifiedInventoryState::NotRequested => {}
        GpuQualifiedInventoryState::Loading => {
            ui.label("Qualification GPU en cours ; CPU reste utilisable.");
        }
        GpuQualifiedInventoryState::Failed(error) => {
            ui.label(format!(
                "Qualification GPU indisponible : {error} ; CPU reste utilisable."
            ));
        }
        GpuQualifiedInventoryState::Complete(catalogue)
        | GpuQualifiedInventoryState::Partial(catalogue) => {
            let labels = gpu_preference_labels(&catalogue.devices);
            for (device, name) in catalogue.devices.iter().zip(labels) {
                let (color, label) = gpu_qualified_presentation(device);
                let text = format!("{name} : {label}.");
                if let Some(color) = color {
                    ui.colored_label(color, text);
                } else {
                    ui.label(text);
                }
            }
            if catalogue.coverage != GpuCatalogueCoverage::Complete || !catalogue.causes.is_empty()
            {
                ui.label("Inventaire GPU partiel ou indéterminé ; CPU reste utilisable.");
            }
        }
    }
}

fn qualified_devices(state: &GpuQualifiedInventoryState) -> Vec<GpuQualifiedDevice> {
    match state {
        GpuQualifiedInventoryState::Complete(catalogue)
        | GpuQualifiedInventoryState::Partial(catalogue) => catalogue.devices.clone(),
        GpuQualifiedInventoryState::NotRequested
        | GpuQualifiedInventoryState::Loading
        | GpuQualifiedInventoryState::Failed(_) => Vec::new(),
    }
}

pub fn gpu_preference_labels(devices: &[GpuQualifiedDevice]) -> Vec<String> {
    use std::collections::HashMap;

    let mut totals = HashMap::<&str, usize>::new();
    for device in devices {
        *totals.entry(&device.device.name).or_default() += 1;
    }
    let mut seen = HashMap::<&str, usize>::new();
    devices
        .iter()
        .map(|device| {
            let name = device.device.name.as_str();
            let total = totals[name];
            if total == 1 {
                return name.to_owned();
            }
            let index = seen.entry(name).or_default();
            *index += 1;
            format!("{name} - {}", *index)
        })
        .collect()
}

pub fn gpu_preference_combo(
    ui: &mut egui::Ui,
    devices: &[GpuQualifiedDevice],
    preferred: &mut Option<String>,
) {
    let labels = gpu_preference_labels(devices);
    let selected_label = preferred.as_deref().map_or_else(
        || "Choisir un GPU".to_owned(),
        |identity| {
            devices
                .iter()
                .position(|device| device.device.identity == identity)
                .map(|index| labels[index].clone())
                .unwrap_or_else(|| "GPU indisponible".to_owned())
        },
    );
    ui.horizontal(|ui| {
        ui.label("GPU préféré");
        egui::ComboBox::from_id_salt("gpu_preference")
            .selected_text(selected_label)
            .show_ui(ui, |ui| {
                gpu_preference_options(ui, devices, &labels, preferred);
            });
    });
}

pub fn gpu_preference_options(
    ui: &mut egui::Ui,
    devices: &[GpuQualifiedDevice],
    labels: &[String],
    preferred: &mut Option<String>,
) {
    for (device, label) in devices.iter().zip(labels.iter().cloned()) {
        let identity = &device.device.identity;
        if gpu_preference_selectable(device) {
            ui.selectable_value(preferred, Some(identity.clone()), label);
        } else if device.support == whisper_core::gpu_inventory::GpuSupport::ConfirmedUnsupported {
            ui.add_enabled_ui(false, |ui| {
                ui.colored_label(egui::Color32::GRAY, label);
            });
        } else {
            ui.add_enabled_ui(false, |ui| {
                ui.label(label);
            });
        }
    }
}

pub fn gpu_preference_selectable(device: &whisper_core::gpu_inventory::GpuQualifiedDevice) -> bool {
    use whisper_core::gpu_inventory::{GpuAvailability, GpuIdentityMatch, GpuState, GpuSupport};
    device.identity_match == GpuIdentityMatch::Unique
        && !device.device.identity.trim().is_empty()
        && device.device.state == GpuState::Ready
        && device.windows_rank.is_some()
        && device.support == GpuSupport::ConfirmedSupported
        && device.availability == GpuAvailability::Available
}

pub fn gpu_selection_resolution_label(
    resolution: &SelectionResolution,
    devices: &[GpuQualifiedDevice],
) -> String {
    let labels = gpu_preference_labels(devices);
    let candidate_label = |candidate: &whisper_core::gpu_selection::GpuCandidate| {
        devices
            .iter()
            .position(|device| device.device.identity == candidate.identity)
            .map(|index| labels[index].clone())
            .unwrap_or_else(|| candidate.name.clone())
    };
    match resolution {
        SelectionResolution::Candidate(SessionCandidate::Cpu) => {
            "Pour la prochaine transcription, le CPU est prévu.".into()
        }
        SelectionResolution::Candidate(SessionCandidate::Gpu(candidate))
            if candidate.preference_substituted =>
        {
            format!(
                "Pour la prochaine transcription, {} est prévu temporairement ; préférence conservée.",
                candidate_label(candidate)
            )
        }
        SelectionResolution::Candidate(SessionCandidate::Gpu(candidate)) => format!(
            "Pour la prochaine transcription, {} est prévu.",
            candidate_label(candidate)
        ),
        SelectionResolution::CpuFallback { .. } => {
            "Pour la prochaine transcription, le CPU est prévu.".into()
        }
        SelectionResolution::Waiting(_) => "Vérification du choix GPU en cours.".into(),
        SelectionResolution::Refused(_) => {
            "Le choix GPU n'est pas disponible pour la prochaine transcription.".into()
        }
    }
}

pub fn path_save_status_label(status: &SaveStatus) -> String {
    match status {
        SaveStatus::NotSaved => String::new(),
        SaveStatus::Pending { revision } => {
            format!("Sauvegarde des réglages en cours (révision {revision}).")
        }
        SaveStatus::Saved { revision } => {
            format!("Réglages enregistrés (révision {revision}).")
        }
        SaveStatus::Failed { revision, error } => {
            format!("Réglages non mémorisés (révision {revision}) : {error}.")
        }
    }
}

pub fn gpu_qualified_presentation(
    device: &whisper_core::gpu_inventory::GpuQualifiedDevice,
) -> (Option<egui::Color32>, &'static str) {
    use whisper_core::gpu_inventory::{
        GpuAvailability as Availability, GpuIdentityMatch, GpuState, GpuSupport as Support,
    };
    let rank_present = device.windows_rank.is_some();
    let windows_ready = device.device.state == GpuState::Ready;
    if !rank_present || !windows_ready {
        return (
            None,
            "qualification incomplète ; sélection GPU indisponible",
        );
    }
    match (device.identity_match, device.support, device.availability) {
        (GpuIdentityMatch::Unique, Support::ConfirmedSupported, Availability::Available) => {
            (Some(egui::Color32::GREEN), "compatible et disponible")
        }
        (
            GpuIdentityMatch::Unique | GpuIdentityMatch::Missing,
            Support::ConfirmedUnsupported,
            _,
        ) => (
            Some(egui::Color32::GRAY),
            "exclu : non pris en charge par ce build",
        ),
        (_, Support::ConfirmedSupported, Availability::Unavailable) => {
            (None, "support confirmé, mais indisponible")
        }
        (_, Support::ConfirmedSupported, Availability::NotVerified) => {
            (None, "support confirmé, disponibilité non vérifiée")
        }
        (_, Support::Undetermined, Availability::Unavailable) => {
            (None, "indéterminé, temporairement indisponible")
        }
        _ => (None, "indéterminé"),
    }
}

struct MicrophoneLabelProjection<'a> {
    devices: &'a [MicrophoneDevice],
}

impl<'a> MicrophoneLabelProjection<'a> {
    fn new(devices: &'a [MicrophoneDevice]) -> Self {
        Self { devices }
    }

    fn device_label(&self, identity: &str) -> String {
        if identity.trim().is_empty() {
            return "Microphone indisponible".into();
        }
        let Some(device) = self.devices.iter().find(|device| device.id == identity) else {
            return "Microphone indisponible".into();
        };
        let name = if device.name.trim().is_empty() {
            "Microphone sans nom"
        } else {
            &device.name
        };
        let mut same_name_ids = self
            .devices
            .iter()
            .filter(|other| other.name == device.name)
            .map(|other| other.id.as_str())
            .collect::<Vec<_>>();
        if same_name_ids.len() <= 1 {
            return name.to_owned();
        }
        same_name_ids.sort_unstable();
        let Some(index) = same_name_ids
            .iter()
            .position(|candidate| *candidate == identity)
            .map(|index| index + 1)
        else {
            return "Microphone indisponible".into();
        };
        format!("{name} ({index})")
    }

    fn choice_label(&self, choice: &MicrophoneChoice) -> String {
        match choice {
            MicrophoneChoice::WindowsDefault => "Microphone par défaut de Windows".into(),
            MicrophoneChoice::Device(identity) => self.device_label(identity),
        }
    }

    fn actual_line(&self, identity: &str) -> String {
        format!("Microphone utilisé : {}", self.device_label(identity))
    }

    fn planned_line(&self, identity: &str) -> String {
        format!("Microphone prévu : {}", self.device_label(identity))
    }
}

fn microphone_device_label(device: &MicrophoneDevice, devices: &[MicrophoneDevice]) -> String {
    MicrophoneLabelProjection::new(devices).device_label(&device.id)
}

fn microphone_choice_label(choice: &MicrophoneChoice, devices: &[MicrophoneDevice]) -> String {
    MicrophoneLabelProjection::new(devices).choice_label(choice)
}

fn path_text_edit(ui: &mut egui::Ui, path: &mut String, width: f32) -> egui::Response {
    ui.scope(|ui| {
        let border = egui::Stroke::new(1.0, ui.visuals().weak_text_color());
        let widgets = &mut ui.style_mut().visuals.widgets;
        widgets.inactive.bg_stroke = border;
        widgets.hovered.bg_stroke = border;
        widgets.active.bg_stroke = border;
        ui.add_sized(
            [width, ui.spacing().interact_size.y],
            egui::TextEdit::singleline(path),
        )
    })
    .inner
}

pub fn gpu_inventory_status(ui: &mut egui::Ui, state: &GpuInventoryState) {
    if let Some(message) = gpu_inventory_status_message(state) {
        ui.label(message);
    }
}

pub fn gpu_inventory_status_message(state: &GpuInventoryState) -> Option<String> {
    match state {
        GpuInventoryState::NotRequested => None,
        GpuInventoryState::Loading => Some("Inventaire GPU en cours…".into()),
        GpuInventoryState::Complete(inventory) if inventory.devices.is_empty() => {
            Some("Aucun GPU présent selon Windows.".into())
        }
        GpuInventoryState::Complete(_) => Some("Inventaire GPU complet selon Windows.".into()),
        GpuInventoryState::Partial(inventory) => {
            let mut messages = Vec::new();
            if inventory.devices.is_empty() {
                messages.push("Aucun GPU confirmé présent".to_owned());
            }
            messages.extend(inventory.warnings.iter().cloned());
            if messages.is_empty() {
                messages.push("Certaines propriétés ou cartes restent incertaines.".into());
            }
            Some(format!("Inventaire GPU partiel : {}", messages.join(" ")))
        }
        GpuInventoryState::Failed(error) => Some(format!("Inventaire GPU indisponible : {error}")),
    }
}

fn path_is_file(path: &str) -> bool {
    !path.is_empty() && std::path::Path::new(path).is_file()
}

fn capture_state_label(state: whisper_core::ports::ParakeetCaptureViewState) -> &'static str {
    match state {
        whisper_core::ports::ParakeetCaptureViewState::Starting => "Démarrage du microphone",
        whisper_core::ports::ParakeetCaptureViewState::Capturing => "Capture en cours",
        whisper_core::ports::ParakeetCaptureViewState::Finalizing => "Finalisation du WAV",
        whisper_core::ports::ParakeetCaptureViewState::Finalized => "Capture terminée",
        whisper_core::ports::ParakeetCaptureViewState::Failed => "Capture interrompue",
    }
}

fn unified_session_phase_label(state: whisper_core::ports::UnifiedSessionPhase) -> &'static str {
    match state {
        whisper_core::ports::UnifiedSessionPhase::Starting => "Démarrage",
        whisper_core::ports::UnifiedSessionPhase::Capturing => "Capture ou transcription en cours",
        whisper_core::ports::UnifiedSessionPhase::Finalizing => "Finalisation",
        whisper_core::ports::UnifiedSessionPhase::Publishing => "Publication",
        whisper_core::ports::UnifiedSessionPhase::Complete => "Terminé",
        whisper_core::ports::UnifiedSessionPhase::Recoverable => "Récupérable",
    }
}

fn compute_choice_label(choice: ComputeChoice) -> &'static str {
    match choice {
        ComputeChoice::Cpu => "CPU",
        ComputeChoice::Gpu => "GPU",
        ComputeChoice::Auto => "Auto",
    }
}

fn path_if_file(path: &str) -> Option<&str> {
    path_is_file(path).then_some(path)
}

fn copyable_path(ui: &mut egui::Ui, label: &str, path: &str) {
    ui.horizontal(|ui| {
        ui.small(format!("{label} : {path}"));
        if ui.small_button("Copier le chemin").clicked() {
            ui.ctx().copy_text(path.to_owned());
        }
    });
}

fn provisional_without_confirmed_prefix(
    capture: &whisper_core::ports::ParakeetCaptureView,
    provisional: &str,
) -> Option<String> {
    let range = capture.provisional_range?;
    let mut overlaps = capture
        .confirmed_segments
        .iter()
        .filter(|segment| {
            segment.range.start_sample < range.end_sample
                && range.start_sample < segment.range.end_sample
        })
        .collect::<Vec<_>>();
    if overlaps.is_empty() {
        return Some(provisional.to_owned());
    }
    overlaps.sort_by_key(|segment| (segment.range.start_sample, segment.segment_id.0));
    let confirmed_words = overlaps
        .iter()
        .flat_map(|segment| segment.text.split_whitespace())
        .map(normalized_word)
        .collect::<Vec<_>>();
    let provisional_words = provisional.split_whitespace().collect::<Vec<_>>();
    if confirmed_words.is_empty() || provisional_words.len() < confirmed_words.len() {
        return None;
    }
    let prefix_matches = provisional_words
        .iter()
        .take(confirmed_words.len())
        .map(|word| normalized_word(word))
        .eq(confirmed_words.iter().cloned());
    if !prefix_matches {
        return None;
    }
    let suffix = provisional_words[confirmed_words.len()..].join(" ");
    (!suffix.trim().is_empty()).then_some(suffix)
}

fn normalized_word(word: &str) -> String {
    word.chars()
        .filter(|character| character.is_alphanumeric() || *character == '\'')
        .flat_map(char::to_lowercase)
        .collect()
}

pub fn default_model_path() -> String {
    std::env::var("LOCALAPPDATA")
        .map(|path| format!(r"{path}\Programs\WhisperBuildDeps\models\ggml-large-v3-turbo.bin"))
        .unwrap_or_default()
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

fn selected_language(selection: &SelectionWidgetState) -> LanguageChoice {
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

fn queued_request_for_compute(durable: &ImportRequest, compute: ComputeChoice) -> ImportRequest {
    let mut request = durable.clone();
    request.config.compute = compute;
    request
}

fn command_compute_choice(command: &AppCommand) -> Option<ComputeChoice> {
    match command {
        AppCommand::StartImport { request }
        | AppCommand::EnqueueImport { request }
        | AppCommand::ProcessQueued { request }
        | AppCommand::ResumeInterrupted { request } => Some(request.config.compute),
        AppCommand::StartLiveConfigured { request }
        | AppCommand::StartUnifiedLiveConfigured { request }
        | AppCommand::StartDirectLiveConfigured { request } => Some(request.config.compute),
        _ => None,
    }
}

fn gpu_selection_identity(command: &AppCommand) -> Option<(JobId, Generation)> {
    match command {
        AppCommand::ResumeInterrupted { request } => {
            Some((request.job_id, request.generation.next()?))
        }
        _ => command
            .identity()
            .map(|identity| (identity.job_id, identity.generation)),
    }
}

pub fn model_supports_live(model: Option<whisper_core::model_catalog::ModelId>) -> bool {
    true
}

fn unified_live_admission_available(view: &AppView) -> bool {
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguageChoices {
    pub auto_language_selectable: bool,
    pub auto_language_cause: Option<whisper_core::engine_capabilities::RefusalCause>,
    pub languages: Vec<whisper_core::engine_capabilities::LanguageOption>,
}

/// Projects language capability for the selected backend. Auto only exposes choices supported
/// by both possible backends because its effective backend can be either one.
pub fn language_choices_for_compute(
    model: &whisper_core::engine_capabilities::ModelOption,
    compute: ComputeChoice,
) -> Option<LanguageChoices> {
    use whisper_core::engine_capabilities::{LanguageOption, RefusalCause};
    let find = |backend| {
        model
            .configurations
            .iter()
            .find(|config| config.key.backend == backend)
    };
    let unavailable = |config_cause: Option<RefusalCause>| {
        config_cause.or(Some(RefusalCause::ConfigurationUnsupported))
    };
    match compute {
        ComputeChoice::Cpu | ComputeChoice::Gpu => {
            let backend = if compute == ComputeChoice::Cpu {
                Backend::Cpu
            } else {
                Backend::Gpu
            };
            let config = find(backend)?;
            Some(LanguageChoices {
                auto_language_selectable: config.selectable && config.auto_language_selectable,
                auto_language_cause: if config.selectable {
                    config.auto_language_cause
                } else {
                    unavailable(config.cause)
                },
                languages: config
                    .languages
                    .iter()
                    .map(|language| LanguageOption {
                        code: language.code.clone(),
                        selectable: config.selectable && language.selectable,
                        cause: if config.selectable {
                            language.cause
                        } else {
                            unavailable(config.cause)
                        },
                    })
                    .collect(),
            })
        }
        ComputeChoice::Auto => {
            let cpu = find(Backend::Cpu)?;
            let gpu = find(Backend::Gpu)?;
            Some(LanguageChoices {
                auto_language_selectable: cpu.selectable
                    && gpu.selectable
                    && cpu.auto_language_selectable
                    && gpu.auto_language_selectable,
                auto_language_cause: if !cpu.selectable || !gpu.selectable {
                    unavailable(cpu.cause.or(gpu.cause))
                } else {
                    cpu.auto_language_cause.or(gpu.auto_language_cause)
                },
                languages: cpu
                    .languages
                    .iter()
                    .map(|cpu_language| {
                        let gpu_language = gpu
                            .languages
                            .iter()
                            .find(|language| language.code == cpu_language.code);
                        let selectable = cpu.selectable
                            && gpu.selectable
                            && cpu_language.selectable
                            && gpu_language.is_some_and(|language| language.selectable);
                        let cause = if !cpu.selectable || !gpu.selectable {
                            unavailable(cpu.cause.or(gpu.cause))
                        } else if !cpu_language.selectable {
                            cpu_language
                                .cause
                                .or(Some(RefusalCause::ConfigurationUnsupported))
                        } else if let Some(gpu_language) = gpu_language {
                            gpu_language
                                .cause
                                .or(Some(RefusalCause::ConfigurationUnknown))
                        } else {
                            Some(RefusalCause::ConfigurationUnknown)
                        };
                        LanguageOption {
                            code: cpu_language.code.clone(),
                            selectable,
                            cause: if selectable { None } else { cause },
                        }
                    })
                    .collect(),
            })
        }
    }
}

fn language_choice_labels(
    choices: &LanguageChoices,
) -> Vec<(&whisper_core::engine_capabilities::LanguageOption, String)> {
    choices
        .languages
        .iter()
        .map(|language| {
            let label = TURBO_LANGUAGE_INPUTS
                .iter()
                .find(|(code, _)| *code == language.code.0)
                .map(|(code, name)| format!("{name} ({code})"))
                .unwrap_or_else(|| language.code.0.clone());
            (language, label)
        })
        .collect()
}

pub fn language_option_labels(
    configuration: &whisper_core::engine_capabilities::ConfigurationOption,
) -> Vec<(&whisper_core::engine_capabilities::LanguageOption, String)> {
    configuration
        .languages
        .iter()
        .map(|language| {
            let code = language.code.0.as_str();
            let name = TURBO_LANGUAGE_INPUTS
                .iter()
                .find(|(known, _)| *known == code)
                .map_or(code, |(_, english)| *english);
            (language, format!("{name} ({code})"))
        })
        .collect()
}

pub fn default_audio_directory() -> String {
    std::env::var("USERPROFILE")
        .map(|path| format!(r"{path}\Documents"))
        .unwrap_or_default()
}

pub fn default_destination() -> String {
    std::env::var("WHISPER_L04_NORMAL_RUN_ARCHIVE")
        .ok()
        .filter(|path| !path.trim().is_empty())
        .or_else(|| {
            std::env::var("USERPROFILE")
                .ok()
                .map(|path| format!(r"{path}\Documents\WhisperTranscriptions"))
        })
        .unwrap_or_default()
}

pub fn approved_model_sha256() -> [u8; 32] {
    parse_sha256(APPROVED_MODEL_SHA256)
}

fn parse_sha256(value: &str) -> [u8; 32] {
    let mut bytes = [0_u8; 32];
    for (index, pair) in value.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        bytes[index] = (hex_digit(pair[0]) << 4) | hex_digit(pair[1]);
    }
    bytes
}
fn hex_digit(value: u8) -> u8 {
    match value {
        b'0'..=b'9' => value - b'0',
        b'a'..=b'f' => value - b'a' + 10,
        _ => 0,
    }
}

#[cfg(test)]
include!("ui/tests/root_support.rs");

fn import_enqueue_enabled(busy: bool) -> bool {
    !busy
}

fn manual_stop_available(state: JobState) -> bool {
    matches!(
        state,
        JobState::Preparing
            | JobState::Capturing
            | JobState::Running
            | JobState::Finalizing
            | JobState::AwaitingChoice
    )
}

fn window_must_remain_open(state: JobState) -> bool {
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

fn unix_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| {
            duration.as_millis().min(u64::MAX as u128) as u64
        })
}

fn process_log_session() -> String {
    format!("process-{}", std::process::id())
}

fn live_log_session(job_id: JobId, generation: Generation, attempt: u64) -> String {
    format!(
        "job-{}-generation-{}-attempt-{attempt}",
        job_id.0,
        generation.get()
    )
}

fn record_first_render(
    identity: (JobId, Generation, u64),
    response: &egui::Response,
    rendered_identity: &mut Option<(JobId, Generation, u64)>,
    rendered_at_unix_ms: &mut Option<u64>,
) {
    if response.rect.width() > 0.0
        && response.rect.height() > 0.0
        && *rendered_identity != Some(identity)
    {
        *rendered_identity = Some(identity);
        *rendered_at_unix_ms = Some(unix_time_ms());
    }
}

#[cfg(test)]
mod tests;
