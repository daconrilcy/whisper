use crate::reactivity_log::ReactivityLog;
use eframe::egui;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, TryRecvError};
use whisper_core::gpu_inventory::GpuState;
use whisper_core::gpu_inventory_app::{GpuInventoryApp, GpuInventoryState};
use whisper_core::path_settings::{PathDefaults, PathField, PathSettingsApp};
use whisper_core::{
    AppCommand, AppFacade, AppView, Application, ApplicationError, ComputeChoice, Generation,
    ImportRequest, JobConfig, JobId, JobState, LanguageChoice, LiveRequest, MicrophoneChoice,
    MicrophoneDevice, MicrophonePort, MicrophoneSettings, NullMicrophonePort,
};

pub const APPROVED_MODEL_SHA256: &str =
    "1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69";
const IMPORT_CPU_MVP: bool = true;
static NEXT_JOB: AtomicU64 = AtomicU64::new(1);

pub struct DesktopApp<A> {
    application: AppFacade<A>,
    view: AppView,
    paths: PathSettingsApp,
    microphones: MicrophoneSettings,
    path_notice: Option<String>,
    microphone_notice: Option<String>,
    manual_language: bool,
    language: String,
    auto_stop_enabled: bool,
    auto_stop_minutes: u64,
    compute_choice: ComputeChoice,
    compute_choice_touched: bool,
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
        Self {
            application,
            view,
            paths,
            microphones,
            path_notice: None,
            microphone_notice,
            manual_language: false,
            language: "fr".into(),
            auto_stop_enabled: false,
            auto_stop_minutes: 120,
            compute_choice,
            compute_choice_touched: false,
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
        }
    }

    fn dispatch(&mut self, command: AppCommand) -> bool {
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
        let serial = NEXT_JOB.fetch_add(1, Ordering::Relaxed);
        let job_id = JobId(((std::process::id() as u128) << 64) | serial as u128);
        let generation = Generation::first();
        let config = JobConfig {
            language: if self.manual_language {
                LanguageChoice::Manual(self.language.trim().to_owned())
            } else {
                LanguageChoice::Automatic
            },
            compute: self.compute_choice,
        };
        if self.dispatch(AppCommand::EnqueueImport {
            request: ImportRequest {
                job_id,
                generation,
                source_path: self.paths.audio.trim().to_owned(),
                source_sha256: None,
                source_samples: None,
                model_path: self.paths.model.trim().to_owned(),
                model_sha256: parse_sha256(APPROVED_MODEL_SHA256),
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
        let clicked_at = unix_time_ms();
        self.live_clicked_at_unix_ms = Some(clicked_at);
        self.live_attempt_counter = self.live_attempt_counter.saturating_add(1);
        self.compute_choice_touched = true;
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
            model_path: self.paths.model.trim().to_owned(),
            model_sha256: parse_sha256(APPROVED_MODEL_SHA256),
            destination: self.paths.destination.trim().to_owned(),
            group_offset_samples: self.live_request.as_ref().map_or(0, |previous| {
                previous
                    .group_offset_samples
                    .saturating_add(self.view.captured_samples)
            }),
            auto_stop_after_speech_samples: None,
            microphone: self.microphones.effective().clone(),
            config: JobConfig {
                language: if self.manual_language {
                    LanguageChoice::Manual(self.language.trim().to_owned())
                } else {
                    LanguageChoice::Automatic
                },
                compute: self.compute_choice,
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
        self.paths.commit_all();
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
        self.poll_gpu_probe();
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
                let response = path_text_edit(ui, &mut self.paths.model, path_field_width);
                if ui.button("Parcourir…").clicked() {
                    self.paths.browse(PathField::Model);
                } else if response.lost_focus()
                    || (response.has_focus()
                        && ui.input(|input| input.key_pressed(egui::Key::Enter)))
                {
                    self.paths.commit_manual(PathField::Model);
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
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.manual_language, "Langue manuelle");
                if self.manual_language {
                    ui.text_edit_singleline(&mut self.language);
                }
                ui.label("Calcul");
                let previous_compute = self.compute_choice;
                egui::ComboBox::from_id_salt("compute_choice")
                    .selected_text(match self.compute_choice {
                        ComputeChoice::Cpu => "CPU",
                        ComputeChoice::Gpu => "GPU",
                        ComputeChoice::Auto => "Auto",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.compute_choice, ComputeChoice::Auto, "Auto");
                        ui.selectable_value(&mut self.compute_choice, ComputeChoice::Cpu, "CPU");
                        ui.selectable_value(&mut self.compute_choice, ComputeChoice::Gpu, "GPU");
                    });
                self.compute_choice_touched |= self.compute_choice != previous_compute;
            });
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
            let can_start = !self.paths.audio.trim().is_empty()
                && !self.paths.model.trim().is_empty()
                && !self.paths.destination.trim().is_empty()
                && (!self.manual_language || !self.language.trim().is_empty());
            if ui
                .add_enabled(can_start, egui::Button::new("Importer et transcrire"))
                .clicked()
            {
                self.start_import();
            }
            {
                let can_start_live = !busy
                    && !self.paths.model.trim().is_empty()
                    && !self.paths.destination.trim().is_empty()
                    && (!self.manual_language || !self.language.trim().is_empty());
                if ui
                    .add_enabled(can_start_live, egui::Button::new("Démarrer le micro"))
                    .clicked()
                {
                    self.start_live(Generation::first());
                }
            }
        });

        if let Some(inventory) = &self.gpu_inventory {
            gpu_inventory_ui(ui, inventory.state());
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
        }
        if let Some(message) = &self.path_notice {
            ui.colored_label(egui::Color32::YELLOW, message);
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
                    self.dispatch(AppCommand::ProcessQueued {
                        request: entry.request.clone(),
                    });
                }
                if entry.status == whisper_core::ports::ImportQueueStatus::Interrupted {
                    if self.resume_confirmation == Some(identity) {
                        if ui.button("Confirmer la reprise").clicked() {
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
                    } else if ui.button("Reprendre…").clicked() {
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
                ui.colored_label(egui::Color32::LIGHT_RED, diagnostic);
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

pub fn gpu_inventory_ui(ui: &mut egui::Ui, state: &GpuInventoryState) {
    ui.group(|ui| {
        ui.heading("Cartes graphiques détectées par Windows");
        match state {
            GpuInventoryState::Complete(inventory) | GpuInventoryState::Partial(inventory) => {
                render_inventory(
                    ui,
                    inventory.devices.iter().map(|device| {
                        format!("{} — {}", device.name, gpu_state_label(&device.state))
                    }),
                );
            }
            GpuInventoryState::NotRequested
            | GpuInventoryState::Loading
            | GpuInventoryState::Failed(_) => {}
        }
    });
    gpu_inventory_note(ui);
}

pub fn gpu_inventory_note(ui: &mut egui::Ui) {
    ui.label("Note : la présence selon Windows ne confirme pas la compatibilité CUDA.");
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

fn render_inventory(ui: &mut egui::Ui, rows: impl Iterator<Item = String>) {
    for row in rows {
        ui.label(row);
    }
}

fn gpu_state_label(state: &GpuState) -> String {
    match state {
        GpuState::Ready => "fonctionnement normal".into(),
        GpuState::Disabled => "désactivé".into(),
        GpuState::Error(code) => format!("état Windows {code}"),
        GpuState::Unknown => "état inconnu".into(),
    }
}

pub fn default_model_path() -> String {
    std::env::var("LOCALAPPDATA")
        .map(|path| format!(r"{path}\Programs\WhisperBuildDeps\models\ggml-large-v3-turbo.bin"))
        .unwrap_or_default()
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
        DesktopApp, import_enqueue_enabled, manual_stop_available, microphone_device_label,
        record_first_render, speech_limit_samples, window_must_remain_open,
    };
    use std::{cell::RefCell, collections::VecDeque, rc::Rc};
    use whisper_core::gpu_inventory::{GpuObservation, GpuPresence, GpuState};
    use whisper_core::gpu_inventory_app::{
        GpuCollection, GpuInventoryApp, GpuInventoryPort, GpuInventoryState,
    };
    use whisper_core::{
        AppCommand, AppView, Application, ApplicationError, ComputeChoice, Generation, JobId,
        MicrophoneChoice, MicrophoneDevice, MicrophonePort,
    };

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
}
