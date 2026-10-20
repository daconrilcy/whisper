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
        let Some((job_id, generation)) = self.active_identity else {
            return false;
        };
        self.dispatch(AppCommand::Quit { job_id, generation })
    }

    fn refresh_microphones(&mut self) {
        match self.microphones.refresh() {
            Ok(()) => self.microphone_notice = None,
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

    fn start_live(&mut self, generation: Generation) {
        if !model_supports_live(self.selection.model) {
            self.view.message =
                Some("Le modèle sélectionné est qualifié uniquement pour l'import.".into());
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
            microphone: self.microphones.effective().clone(),
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
        if self.dispatch(AppCommand::StartDirectLiveConfigured { request }) {
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
        self.paths.commit_manual(PathField::Audio);
        self.paths.commit_manual(PathField::Destination);
        if let Some(inventory) = &mut self.gpu_inventory {
            inventory.close();
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
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
        let busy = self
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
            if self.quit_pending && state == Some(JobState::Recoverable) {
                self.quit_pending = false;
            }
            if self.quit_pending {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
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
                            .find(|model| model.id == whisper_core::model_catalog::ModelId::ParakeetTdt06BV3)
                            .is_some_and(|model| {
                                model.configurations.iter().any(|configuration| {
                                    configuration.key.backend == whisper_core::model_catalog::Backend::Gpu
                                        && configuration.selectable
                                        && configuration.auto_language_selectable
                                })
                            })
                    }) && self.catalog.as_ref().is_some_and(parakeet_gpu_catalog_ready);
                    egui::ComboBox::from_id_salt("compute_choice")
                        .selected_text(match self.compute_choice {
                            ComputeChoice::Cpu => "CPU",
                            ComputeChoice::Gpu => "GPU",
                            ComputeChoice::Auto => "Auto",
                        })
                        .show_ui(ui, |ui| {
                            compute_interacted |= ui.selectable_value(
                                &mut self.compute_choice,
                                ComputeChoice::Auto,
                                "Auto",
                            ).clicked();
                            compute_interacted |= ui.selectable_value(
                                &mut self.compute_choice,
                                ComputeChoice::Cpu,
                                "CPU",
                            ).clicked();
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
                            .selectable_value(&mut self.compute_choice, ComputeChoice::Auto, "Auto")
                            .clicked();
                        compute_interacted |= ui
                            .selectable_value(&mut self.compute_choice, ComputeChoice::Cpu, "CPU")
                            .clicked();
                        compute_interacted |= ui
                            .selectable_value(&mut self.compute_choice, ComputeChoice::Gpu, "GPU")
                            .clicked();
                    });
                }
                let compute_changed = self.compute_choice != previous_compute;
                let compute_selection_interacted = record_compute_selection_interaction(
                    &mut self.compute_choice_touched,
                    compute_interacted,
                    compute_changed,
                );
                if compute_selection_interacted
                    && let Some(sender) = &self.preference_commands
                {
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
                let resolution = inventory.resolve_selection(
                    self.compute_choice,
                    self.paths.gpu_preference(),
                );
                ui.label(gpu_selection_resolution_label(&resolution, &gpu_preferences));
            }
            ui.group(|ui| {
                ui.heading("Microphone");
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
                && self.selection.manual_language.as_ref().is_none_or(|language| !language.is_empty());
            if ui
                .add_enabled(can_start, egui::Button::new("Importer et transcrire"))
                .clicked()
            {
                self.start_import();
            }
            {
                let can_start_live = !busy
                    && model_supports_live(self.selection.model)
                    && self.selected_model_artifact().is_some()
                    && !self.paths.destination.trim().is_empty()
                    && self.selection.manual_language.as_ref().is_none_or(|language| !language.is_empty());
                if ui
                    .add_enabled(can_start_live, egui::Button::new("Démarrer le micro"))
                    .clicked()
                {
                    self.start_live(Generation::first());
                }
                if !model_supports_live(self.selection.model) {
                    ui.label("Le microphone n'est pas qualifié pour ce modèle.");
                }
            }
        });

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
            if ui.button(label).clicked() {
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
            if !IMPORT_CPU_MVP && self.live_request.is_some() {
                ui.label(format!(
                    "Micro — capturés {} | admis {} | audio durable {} | fragment confirmé {} | parole {} échantillons",
                    self.view.captured_samples,
                    self.view.admitted_samples,
                    self.view.audio_durable_samples,
                    self.view.confirmed_fragment_samples,
                    self.view.speech_samples,
                ));
                if let Some(text) = self.view.provisional_text.as_deref() {
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
                if let Some(text) = self.view.confirmed_text.as_deref() {
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
    }
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

fn microphone_device_label(device: &MicrophoneDevice, devices: &[MicrophoneDevice]) -> String {
    if devices
        .iter()
        .filter(|other| other.name == device.name)
        .count()
        > 1
    {
        let suffix = (8..=device.id.chars().count())
            .find_map(|length| {
                let suffix = device
                    .id
                    .chars()
                    .rev()
                    .take(length)
                    .collect::<String>()
                    .chars()
                    .rev()
                    .collect::<String>();
                devices
                    .iter()
                    .filter(|other| other.name == device.name)
                    .all(|other| other.id == device.id || !other.id.ends_with(&suffix))
                    .then_some(suffix)
            })
            .unwrap_or_else(|| device.id.clone());
        format!("{} ({suffix})", device.name)
    } else {
        device.name.clone()
    }
}

fn microphone_choice_label(choice: &MicrophoneChoice, devices: &[MicrophoneDevice]) -> String {
    match choice {
        MicrophoneChoice::WindowsDefault => "Microphone par défaut de Windows".into(),
        MicrophoneChoice::Device(id) => devices.iter().find(|device| device.id == *id).map_or_else(
            || "Microphone sélectionné indisponible".into(),
            |device| microphone_device_label(device, devices),
        ),
    }
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
    model != Some(whisper_core::model_catalog::ModelId::ParakeetTdt06BV3)
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
fn speech_limit_samples(enabled: bool, minutes: u64) -> Option<u64> {
    enabled.then(|| minutes.max(1).saturating_mul(60 * 16_000))
}

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
mod tests {
    use super::{
        CatalogSnapshot, DesktopApp, ModelId, ModelSelectionRuntime, command_compute_choice,
        compute_for_model, gpu_selection_identity, import_enqueue_enabled, manual_stop_available,
        microphone_device_label, model_preference_command_channel, parakeet_gpu_catalog_ready,
        queued_request_for_compute, record_first_render, speech_limit_samples,
        window_must_remain_open,
    };
    use std::{cell::RefCell, collections::VecDeque, rc::Rc};
    use whisper_core::gpu_inventory::{GpuObservation, GpuPresence, GpuState};
    use whisper_core::gpu_inventory_app::{
        GpuCollection, GpuInventoryApp, GpuInventoryPort, GpuInventoryState,
    };
    use whisper_core::{
        AppCommand, AppView, Application, ApplicationError, ComputeChoice, Generation,
        ImportEffect, ImportRequest, JobConfig, JobId, LanguageChoice, MicrophoneChoice,
        MicrophoneDevice, MicrophonePort,
    };

    #[test]
    fn parakeet_preserves_the_selected_compute_preference() {
        for choice in [ComputeChoice::Auto, ComputeChoice::Cpu, ComputeChoice::Gpu] {
            assert_eq!(
                compute_for_model(Some(ModelId::ParakeetTdt06BV3), choice),
                choice
            );
        }
        assert_eq!(
            compute_for_model(Some(ModelId::WhisperLargeV3Turbo), ComputeChoice::Auto),
            ComputeChoice::Auto
        );
    }

    #[test]
    fn queued_import_uses_current_compute_and_preserves_the_durable_request() {
        let original = ImportRequest {
            job_id: JobId(74),
            generation: Generation::first(),
            source_path: "queued.wav".into(),
            source_sha256: Some([4; 32]),
            source_samples: Some(16_000),
            model_path: "parakeet.gguf".into(),
            model_sha256: [8; 32],
            destination: "archive".into(),
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Gpu,
            },
        };

        for compute in [ComputeChoice::Cpu, ComputeChoice::Gpu, ComputeChoice::Auto] {
            let request = queued_request_for_compute(&original, compute);
            let mut expected = original.clone();
            expected.config.compute = compute;
            assert_eq!(request, expected);
            assert_eq!(
                command_compute_choice(&AppCommand::ProcessQueued {
                    request: request.clone()
                }),
                Some(compute)
            );
        }
        assert_eq!(original.config.compute, ComputeChoice::Gpu);
    }

    #[test]
    fn resumed_import_uses_its_durable_compute_instead_of_the_global_selector() {
        let mut request = ImportRequest {
            job_id: JobId(75),
            generation: Generation::first(),
            source_path: "queued.wav".into(),
            source_sha256: Some([4; 32]),
            source_samples: Some(16_000),
            model_path: "parakeet.gguf".into(),
            model_sha256: [8; 32],
            destination: "archive".into(),
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Gpu,
            },
        };
        request.generation = request.generation.next().unwrap();

        let command = AppCommand::ResumeInterrupted { request };
        assert_eq!(command_compute_choice(&command), Some(ComputeChoice::Gpu));
        assert_eq!(
            gpu_selection_identity(&command),
            Some((
                JobId(75),
                Generation::first().next().unwrap().next().unwrap()
            ))
        );
    }

    #[derive(Clone, Default)]
    struct RecordingApplication(Rc<RefCell<Vec<AppCommand>>>);

    impl Application for RecordingApplication {
        fn dispatch(&mut self, command: AppCommand) -> Result<AppView, ApplicationError> {
            self.0.borrow_mut().push(command);
            Ok(AppView::default())
        }

        fn view(&self) -> AppView {
            AppView::default()
        }
    }

    struct RejectQuitApplication;
    struct RejectStartApplication;

    #[derive(Clone, Default)]
    struct RefusingStagedCandidateApplication {
        effects: Rc<RefCell<Vec<ImportEffect>>>,
        pending_candidate: Rc<RefCell<Option<ImportEffect>>>,
    }

    struct OneMicrophone;

    #[derive(Clone, Default)]
    struct ControlledGpuInventory(Rc<RefCell<VecDeque<(u64, Result<GpuCollection, String>)>>>);

    impl GpuInventoryPort for ControlledGpuInventory {
        fn start(&mut self, _generation: u64) -> Result<(), String> {
            Ok(())
        }

        fn poll(&mut self) -> Option<(u64, Result<GpuCollection, String>)> {
            self.0.borrow_mut().pop_front()
        }
    }

    impl MicrophonePort for OneMicrophone {
        fn list_inputs(&self) -> Result<Vec<MicrophoneDevice>, String> {
            Ok(vec![MicrophoneDevice {
                id: "wasapi:chosen".into(),
                name: "Microphone de test".into(),
                unavailable_reason: None,
            }])
        }
    }

    impl Application for RejectQuitApplication {
        fn dispatch(&mut self, command: AppCommand) -> Result<AppView, ApplicationError> {
            if matches!(command, AppCommand::Quit { .. }) {
                Err(ApplicationError::Failed("quit admission rejected".into()))
            } else {
                Ok(AppView::default())
            }
        }

        fn view(&self) -> AppView {
            AppView::default()
        }
    }

    impl Application for RejectStartApplication {
        fn dispatch(&mut self, _command: AppCommand) -> Result<AppView, ApplicationError> {
            Err(ApplicationError::Failed("start admission rejected".into()))
        }

        fn view(&self) -> AppView {
            AppView::default()
        }
    }

    impl Application for RefusingStagedCandidateApplication {
        fn dispatch(&mut self, command: AppCommand) -> Result<AppView, ApplicationError> {
            if matches!(
                command,
                AppCommand::StartImport { .. }
                    | AppCommand::EnqueueImport { .. }
                    | AppCommand::ProcessQueued { .. }
                    | AppCommand::ResumeInterrupted { .. }
                    | AppCommand::StartLiveConfigured { .. }
                    | AppCommand::StartDirectLiveConfigured { .. }
            ) {
                self.pending_candidate.borrow_mut().take();
                return Ok(AppView {
                    message: Some("Modèle indisponible : test d'admission".into()),
                    ..AppView::default()
                });
            }
            if let Some(effect) = self.pending_candidate.borrow_mut().take() {
                self.effects.borrow_mut().push(effect);
            }
            Ok(AppView::default())
        }

        fn view(&self) -> AppView {
            AppView::default()
        }

        fn set_gpu_candidate(
            &mut self,
            job_id: JobId,
            generation: Generation,
            candidate: Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
        ) -> Result<(), ApplicationError> {
            *self.pending_candidate.borrow_mut() = Some(ImportEffect::SetGpuCandidate {
                job_id,
                generation,
                candidate,
            });
            Ok(())
        }
    }

    #[test]
    fn auto_stop_is_disabled_by_default_and_uses_speech_sample_clock_when_enabled() {
        assert_eq!(speech_limit_samples(false, 120), None);
        assert_eq!(speech_limit_samples(true, 2), Some(1_920_000));
        assert_eq!(speech_limit_samples(true, 0), Some(960_000));
    }

    #[test]
    fn import_enqueue_waits_during_live_capture() {
        assert!(!import_enqueue_enabled(true));
        assert!(import_enqueue_enabled(false));
    }

    #[test]
    fn window_stays_open_through_capture_and_drain() {
        assert!(window_must_remain_open(whisper_core::JobState::Capturing));
        assert!(window_must_remain_open(whisper_core::JobState::Draining));
        assert!(window_must_remain_open(
            whisper_core::JobState::AwaitingChoice
        ));
        assert!(!window_must_remain_open(whisper_core::JobState::Complete));
    }

    #[test]
    fn manual_stop_is_available_while_micro_is_capturing() {
        assert!(manual_stop_available(whisper_core::JobState::Capturing));
        assert!(!manual_stop_available(whisper_core::JobState::Draining));
        assert!(!manual_stop_available(whisper_core::JobState::Complete));
    }

    #[test]
    fn starting_a_new_microphone_job_hides_the_previous_publication() {
        let mut app = DesktopApp::new(RecordingApplication::default());
        app.view.active_job = Some((JobId(1), whisper_core::JobState::Complete));
        app.view.last_result = Some("Transcription publiée et vérifiée".into());
        assert!(app.published_result().is_some());

        app.start_live(Generation::first());
        let next_job = app.live_request.as_ref().unwrap().job_id;
        app.view.active_job = Some((next_job, whisper_core::JobState::Preparing));
        app.view.last_result = Some("Transcription publiée et vérifiée".into());
        assert_eq!(app.published_result(), None);
    }

    #[test]
    fn refused_ui_model_command_has_no_gpu_candidate_io_effect() {
        let application = RefusingStagedCandidateApplication::default();
        let effects = application.effects.clone();
        let pending_candidate = application.pending_candidate.clone();
        let paths = whisper_core::path_settings::PathSettingsApp::with_defaults(
            whisper_core::path_settings::PathDefaults {
                audio_directory: String::new(),
                model_path: "model.bin".into(),
                destination: "archive".into(),
            },
        );
        let mut app = DesktopApp::with_paths(application, ComputeChoice::Cpu, None, paths);
        let request = ImportRequest {
            job_id: JobId(73),
            generation: Generation::first(),
            source_path: "input.wav".into(),
            source_sha256: None,
            source_samples: None,
            model_path: "model.bin".into(),
            model_sha256: [0; 32],
            destination: "archive".into(),
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Cpu,
            },
        };

        assert!(app.dispatch(AppCommand::EnqueueImport { request }));
        assert!(
            app.view
                .message
                .as_deref()
                .is_some_and(|message| message.contains("Modèle indisponible"))
        );
        assert!(effects.borrow().is_empty());
        assert!(pending_candidate.borrow().is_none());
    }

    #[test]
    fn direct_live_request_uses_explicit_microphone_without_changing_compute() {
        let recorder = RecordingApplication::default();
        let paths = whisper_core::path_settings::PathSettingsApp::with_defaults(
            whisper_core::path_settings::PathDefaults {
                audio_directory: String::new(),
                model_path: "model".into(),
                destination: "output".into(),
            },
        );
        let mut app = DesktopApp::with_microphones(
            recorder.clone(),
            ComputeChoice::Gpu,
            None,
            paths,
            Box::new(OneMicrophone),
        );
        app.microphones
            .select(MicrophoneChoice::Device("wasapi:chosen".into()))
            .unwrap();
        app.start_live(Generation::first());
        assert!(matches!(
            recorder.0.borrow().last(),
            Some(AppCommand::StartDirectLiveConfigured { request })
                if request.microphone == MicrophoneChoice::Device("wasapi:chosen".into())
                    && request.config.compute == ComputeChoice::Gpu
        ));
    }

    #[test]
    fn homonymous_microphones_have_distinct_labels() {
        let devices = vec![
            MicrophoneDevice {
                id: "wasapi:one".into(),
                name: "Micro".into(),
                unavailable_reason: None,
            },
            MicrophoneDevice {
                id: "wasapi:two".into(),
                name: "Micro".into(),
                unavailable_reason: None,
            },
        ];
        assert_ne!(
            microphone_device_label(&devices[0], &devices),
            microphone_device_label(&devices[1], &devices)
        );
    }

    #[test]
    fn rejected_quit_admission_does_not_latch_pending_close() {
        let mut app = DesktopApp::new(RejectQuitApplication);
        app.active_identity = Some((JobId(22), Generation::first()));
        assert!(!app.request_quit());
        assert!(!app.quit_pending);
        assert_eq!(app.view.message.as_deref(), Some("quit admission rejected"));
    }

    #[test]
    fn enqueue_then_stop_still_targets_the_live_identity() {
        let recorder = RecordingApplication::default();
        let mut app = DesktopApp::new(recorder.clone());
        let live = (JobId(20), Generation::first());
        app.active_identity = Some(live);
        app.start_import();
        assert_eq!(app.active_identity, Some(live));
        assert!(matches!(
            recorder.0.borrow().first(),
            Some(AppCommand::EnqueueImport { .. })
        ));
        app.stop_active();
        assert_eq!(
            recorder.0.borrow().last(),
            Some(&AppCommand::Stop {
                job_id: live.0,
                generation: live.1,
            })
        );
    }

    #[test]
    fn import_uses_selected_gpu() {
        let recorder = RecordingApplication::default();
        let mut app = DesktopApp::new(recorder.clone());
        assert_eq!(app.compute_choice, ComputeChoice::Cpu);
        app.compute_choice = ComputeChoice::Gpu;
        app.start_import();
        assert!(matches!(
            recorder.0.borrow().first(),
            Some(AppCommand::EnqueueImport { request })
                if request.config.compute == ComputeChoice::Gpu
        ));
    }

    #[test]
    fn pending_and_late_gpu_inventory_does_not_change_compute_or_gate_transcription() {
        for choice in [ComputeChoice::Auto, ComputeChoice::Cpu, ComputeChoice::Gpu] {
            let recorder = RecordingApplication::default();
            let port = ControlledGpuInventory::default();
            let responses = Rc::clone(&port.0);
            let mut app = DesktopApp::new(recorder.clone())
                .with_gpu_inventory(GpuInventoryApp::new(Box::new(port)));
            app.paths.audio = "sample.wav".into();
            app.paths.model = "model.bin".into();
            app.paths.destination = "output".into();
            app.compute_choice = choice;

            app.start_import();
            app.start_live(Generation::first());
            assert_eq!(app.compute_choice, choice);
            assert!(matches!(
                app.gpu_inventory.as_ref().unwrap().state(),
                GpuInventoryState::Loading
            ));
            {
                let commands = recorder.0.borrow();
                assert!(
                    matches!(&commands[0], AppCommand::EnqueueImport { request } if request.config.compute == choice)
                );
                assert!(
                    matches!(&commands[1], AppCommand::StartDirectLiveConfigured { request } if request.config.compute == choice)
                );
            }

            responses.borrow_mut().push_back((
                1,
                Ok(GpuCollection {
                    observations: vec![GpuObservation {
                        identity: "late-gpu".into(),
                        name: Some("Late GPU".into()),
                        state: Some(GpuState::Ready),
                        presence: GpuPresence::Current,
                    }],
                    error: None,
                }),
            ));
            app.gpu_inventory.as_mut().unwrap().poll();

            assert!(matches!(
                app.gpu_inventory.as_ref().unwrap().state(),
                GpuInventoryState::Complete(_)
            ));
            assert_eq!(app.compute_choice, choice);
            let commands = recorder.0.borrow();
            assert!(
                matches!(&commands[0], AppCommand::EnqueueImport { request } if request.config.compute == choice)
            );
            assert!(
                matches!(&commands[1], AppCommand::StartDirectLiveConfigured { request } if request.config.compute == choice)
            );
        }
    }

    #[test]
    fn auto_preference_is_sent_to_both_paths_without_becoming_cpu() {
        let recorder = RecordingApplication::default();
        let mut app = DesktopApp::new(recorder.clone());
        app.compute_choice = ComputeChoice::Auto;
        app.start_import();
        app.start_live(Generation::first());
        let commands = recorder.0.borrow();
        assert!(matches!(&commands[0], AppCommand::EnqueueImport { request }
            if request.config.compute == ComputeChoice::Auto));
        assert!(
            matches!(&commands[1], AppCommand::StartDirectLiveConfigured { request }
            if request.config.compute == ComputeChoice::Auto)
        );
        assert_eq!(app.compute_choice, ComputeChoice::Auto);
    }

    #[test]
    fn cuda_startup_selects_gpu_for_import_and_microphone() {
        let recorder = RecordingApplication::default();
        let mut app = DesktopApp::with_startup_compute(recorder.clone(), ComputeChoice::Gpu, None);
        assert_eq!(app.compute_choice, ComputeChoice::Gpu);
        app.start_import();
        app.start_live(Generation::first());
        let commands = recorder.0.borrow();
        assert!(
            matches!(&commands[0], AppCommand::EnqueueImport { request } if request.config.compute == ComputeChoice::Gpu)
        );
        assert!(
            matches!(&commands[1], AppCommand::StartDirectLiveConfigured { request } if request.config.compute == ComputeChoice::Gpu)
        );
    }

    #[test]
    fn asynchronous_gpu_probe_sets_the_existing_startup_choice() {
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut app = DesktopApp::with_startup_compute(
            RecordingApplication::default(),
            ComputeChoice::Cpu,
            Some("Vérification du GPU en cours…".into()),
        )
        .with_gpu_probe(receiver);
        app.poll_gpu_probe();
        assert!(app.gpu_probe.is_some());
        sender.send(Ok(())).unwrap();
        app.poll_gpu_probe();
        assert!(app.gpu_probe.is_none());
        assert_eq!(app.compute_choice, ComputeChoice::Gpu);
        assert!(app.startup_message.is_none());
    }

    #[test]
    fn unavailable_cuda_leaves_cpu_usable_and_explains_choice() {
        let recorder = RecordingApplication::default();
        let message = "GPU indisponible : choisissez CPU".to_owned();
        let mut app = DesktopApp::with_startup_compute(
            recorder.clone(),
            ComputeChoice::Cpu,
            Some(message.clone()),
        );
        assert_eq!(app.startup_message.as_deref(), Some(message.as_str()));
        app.start_import();
        app.start_live(Generation::first());
        let commands = recorder.0.borrow();
        assert!(
            matches!(&commands[0], AppCommand::EnqueueImport { request } if request.config.compute == ComputeChoice::Cpu)
        );
        assert!(
            matches!(&commands[1], AppCommand::StartDirectLiveConfigured { request } if request.config.compute == ComputeChoice::Cpu)
        );
    }

    #[test]
    fn microphone_uses_selected_gpu() {
        let recorder = RecordingApplication::default();
        let mut app = DesktopApp::new(recorder.clone());
        app.compute_choice = ComputeChoice::Gpu;
        app.start_live(Generation::first());
        assert!(matches!(
            recorder.0.borrow().first(),
            Some(AppCommand::StartDirectLiveConfigured { request })
                if request.config.compute == ComputeChoice::Gpu
        ));
    }

    #[test]
    fn new_microphone_start_gets_a_new_group_while_continuation_keeps_its_identity() {
        let recorder = RecordingApplication::default();
        let mut app = DesktopApp::new(recorder.clone());
        app.start_live(Generation::first());
        let first = app.live_request.clone().unwrap();
        app.start_live(Generation::first());
        let second = app.live_request.clone().unwrap();
        assert_ne!(first.job_id, second.job_id);
        assert_eq!(second.group_offset_samples, 0);
        app.start_live(second.generation.next().unwrap());
        let continued = app.live_request.as_ref().unwrap();
        assert_eq!(continued.job_id, second.job_id);
        assert_eq!(continued.generation.get(), 2);
        assert_eq!(recorder.0.borrow().len(), 3);
    }

    #[test]
    fn first_render_timestamp_is_recorded_after_the_label_in_an_egui_frame() {
        let context = eframe::egui::Context::default();
        let mut identity = None;
        let mut first_render = None;
        let _ = context.run_ui(Default::default(), |ui| {
            eframe::egui::CentralPanel::default().show(ui, |ui| {
                let label = ui.label("provisional transcript");
                record_first_render(
                    (JobId(21), Generation::first(), 1),
                    &label,
                    &mut identity,
                    &mut first_render,
                );
            });
        });
        assert_eq!(identity, Some((JobId(21), Generation::first(), 1)));
        assert!(first_render.is_some());
    }

    #[test]
    fn first_effective_action_is_not_recorded_when_application_rejects_start() {
        let mut app = DesktopApp::new(RejectStartApplication);
        app.start_live(Generation::first());
        assert!(app.live_clicked_at_unix_ms.is_some());
        assert_eq!(
            app.view.message.as_deref(),
            Some("start admission rejected")
        );
    }

    #[test]
    fn changing_catalog_snapshots_regray_and_restore_parakeet_in_the_open_ui() {
        let (catalog_sender, catalog_receiver) = std::sync::mpsc::channel();
        let (_preference_event_sender, preference_events) =
            std::sync::mpsc::channel::<super::ModelPreferenceEvent>();
        let (preference_commands, _preference_command_receiver) =
            model_preference_command_channel();
        let mut app = DesktopApp::new(RecordingApplication::default())
            .with_model_selection_runtime(ModelSelectionRuntime {
                catalog: catalog_receiver,
                preference_events,
                preference_commands,
            });
        app.selection.model = Some(ModelId::ParakeetTdt06BV3);

        let snapshot = |generation, parakeet_availability| CatalogSnapshot {
            generation,
            entries: whisper_core::model_registry::MODELS
                .iter()
                .map(|record| {
                    let availability = if record.artifact.id == ModelId::WhisperLargeV3Turbo {
                        whisper_core::model_catalog::ModelAvailability::Ready
                    } else {
                        parakeet_availability
                    };
                    whisper_adapters::model_catalog::CatalogEntry {
                        id: record.artifact.id,
                        display_name: record.display_name.to_owned(),
                        path: std::path::PathBuf::from("dynamic-test-models")
                            .join(record.artifact.file_name),
                        availability,
                        expected_size_bytes: record.artifact.size_bytes,
                        observed_size_bytes: Some(record.artifact.size_bytes),
                        expected_sha256: record.artifact.sha256.to_owned(),
                        observed_sha256: Some(record.artifact.sha256.to_owned()),
                        diagnostic: None,
                    }
                })
                .collect(),
        };
        let is_selectable = |app: &DesktopApp<RecordingApplication>| {
            app.projection
                .as_ref()
                .unwrap()
                .models
                .iter()
                .find(|model| model.id == ModelId::ParakeetTdt06BV3)
                .unwrap()
                .selectable
        };

        catalog_sender
            .send(snapshot(
                1,
                whisper_core::model_catalog::ModelAvailability::Ready,
            ))
            .unwrap();
        app.poll_model_selection();
        assert!(is_selectable(&app));
        assert_eq!(app.selection.model, Some(ModelId::ParakeetTdt06BV3));

        catalog_sender
            .send(snapshot(
                2,
                whisper_core::model_catalog::ModelAvailability::Missing,
            ))
            .unwrap();
        app.poll_model_selection();
        assert!(!is_selectable(&app));
        assert_eq!(app.selection.model, Some(ModelId::ParakeetTdt06BV3));
        assert!(
            app.projection
                .as_ref()
                .unwrap()
                .models
                .iter()
                .find(|model| model.id == ModelId::WhisperLargeV3Turbo)
                .unwrap()
                .selectable
        );

        catalog_sender
            .send(snapshot(
                3,
                whisper_core::model_catalog::ModelAvailability::Ready,
            ))
            .unwrap();
        app.poll_model_selection();
        assert!(is_selectable(&app));
        assert_eq!(app.selection.model, Some(ModelId::ParakeetTdt06BV3));
    }

    #[test]
    fn gpu_compute_control_requires_a_ready_qualified_runtime_catalog_entry() {
        let record = whisper_core::model_registry::model(ModelId::ParakeetTdt06BV3);
        let entry = || whisper_adapters::model_catalog::CatalogEntry {
            id: ModelId::ParakeetTdt06BV3,
            display_name: record.display_name.to_owned(),
            path: std::path::PathBuf::from(record.artifact.file_name),
            availability: whisper_core::model_catalog::ModelAvailability::Ready,
            expected_size_bytes: record.artifact.size_bytes,
            observed_size_bytes: Some(record.artifact.size_bytes),
            expected_sha256: record.artifact.sha256.to_owned(),
            observed_sha256: Some(record.artifact.sha256.to_owned()),
            diagnostic: None,
        };
        let catalog = |entry| CatalogSnapshot {
            generation: 1,
            entries: vec![entry],
        };

        assert!(parakeet_gpu_catalog_ready(&catalog(entry())));

        let mut divergent = entry();
        divergent.diagnostic = Some("GPU indisponible : DLL différente".into());
        assert!(!parakeet_gpu_catalog_ready(&catalog(divergent)));

        let mut missing = entry();
        missing.availability = whisper_core::model_catalog::ModelAvailability::Missing;
        assert!(!parakeet_gpu_catalog_ready(&catalog(missing)));
    }
}
