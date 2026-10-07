use eframe::egui;
use std::sync::atomic::{AtomicU64, Ordering};
use whisper_core::{
    AppCommand, AppFacade, AppView, Application, ApplicationError, ComputeChoice, Generation,
    ImportRequest, JobConfig, JobId, JobState, LanguageChoice, LiveRequest,
};

const APPROVED_MODEL_SHA256: &str =
    "1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69";
static NEXT_JOB: AtomicU64 = AtomicU64::new(1);

pub struct DesktopApp<A> {
    application: AppFacade<A>,
    view: AppView,
    source_path: String,
    model_path: String,
    destination: String,
    manual_language: bool,
    language: String,
    auto_stop_enabled: bool,
    auto_stop_minutes: u64,
    active_identity: Option<(JobId, Generation)>,
    scanned_destination: Option<String>,
    observed_job_state: Option<JobState>,
    resume_confirmation: Option<(JobId, Generation)>,
    live_request: Option<LiveRequest>,
    provisional_first_rendered_at_unix_ms: Option<u64>,
    provisional_render_identity: Option<(JobId, Generation, u64)>,
}

impl<A: Application> DesktopApp<A> {
    pub fn new(application: A) -> Self {
        let application = AppFacade::new(application);
        let view = application.view();
        let model_path = std::env::var("LOCALAPPDATA")
            .map(|path| format!(r"{path}\Programs\WhisperBuildDeps\models\ggml-large-v3-turbo.bin"))
            .unwrap_or_default();
        let destination = std::env::var("USERPROFILE")
            .map(|path| format!(r"{path}\Documents\WhisperTranscriptions"))
            .unwrap_or_default();
        Self {
            application,
            view,
            source_path: String::new(),
            model_path,
            destination,
            manual_language: false,
            language: "fr".into(),
            auto_stop_enabled: false,
            auto_stop_minutes: 120,
            active_identity: None,
            scanned_destination: None,
            observed_job_state: None,
            resume_confirmation: None,
            live_request: None,
            provisional_first_rendered_at_unix_ms: None,
            provisional_render_identity: None,
        }
    }

    fn dispatch(&mut self, command: AppCommand) {
        match self.application.dispatch(command) {
            Ok(view) => self.view = view,
            Err(ApplicationError::Failed(message)) => self.view.message = Some(message),
            Err(error) => self.view.message = Some(format!("Commande impossible : {error:?}")),
        }
    }

    fn stop_active(&mut self) {
        if let Some((job_id, generation)) = self.active_identity {
            self.dispatch(AppCommand::Stop { job_id, generation });
        }
    }

    fn start_import(&mut self) {
        let serial = NEXT_JOB.fetch_add(1, Ordering::Relaxed);
        let job_id = JobId(((std::process::id() as u128) << 64) | serial as u128);
        let generation = Generation::first();
        let config = JobConfig {
            language: if self.manual_language {
                LanguageChoice::Manual(self.language.trim().to_owned())
            } else {
                LanguageChoice::Automatic
            },
            compute: ComputeChoice::Cpu,
        };
        self.dispatch(AppCommand::EnqueueImport {
            request: ImportRequest {
                job_id,
                generation,
                source_path: self.source_path.trim().to_owned(),
                source_sha256: None,
                source_samples: None,
                model_path: self.model_path.trim().to_owned(),
                model_sha256: parse_sha256(APPROVED_MODEL_SHA256),
                destination: self.destination.trim().to_owned(),
                config,
            },
        });
    }

    fn start_live(&mut self, generation: Generation) {
        let serial = NEXT_JOB.fetch_add(1, Ordering::Relaxed);
        let job_id = self.live_request.as_ref().map_or_else(
            || JobId(((std::process::id() as u128) << 64) | serial as u128),
            |request| request.job_id,
        );
        let request = LiveRequest {
            job_id,
            generation,
            model_path: self.model_path.trim().to_owned(),
            model_sha256: parse_sha256(APPROVED_MODEL_SHA256),
            destination: self.destination.trim().to_owned(),
            group_offset_samples: self.live_request.as_ref().map_or(0, |previous| {
                previous
                    .group_offset_samples
                    .saturating_add(self.view.captured_samples)
            }),
            auto_stop_after_speech_samples: speech_limit_samples(
                self.auto_stop_enabled,
                self.auto_stop_minutes,
            ),
            config: JobConfig {
                language: if self.manual_language {
                    LanguageChoice::Manual(self.language.trim().to_owned())
                } else {
                    LanguageChoice::Automatic
                },
                compute: ComputeChoice::Cpu,
            },
        };
        self.active_identity = Some((job_id, generation));
        self.live_request = Some(request.clone());
        self.dispatch(AppCommand::StartLiveConfigured { request });
    }

    fn continue_live_group(&mut self, previous: LiveRequest) {
        let Some(generation) = previous.generation.next() else {
            self.view.message =
                Some("Le numéro de passage est épuisé ; le groupe reste récupérable.".into());
            return;
        };
        self.model_path = previous.model_path.clone();
        self.destination = previous.destination.clone();
        match &previous.config.language {
            LanguageChoice::Automatic => {
                self.manual_language = false;
                self.language.clear();
            }
            LanguageChoice::Manual(language) => {
                self.manual_language = true;
                self.language = language.clone();
            }
        }
        // Keep the durable group identity and configuration. LiveArchive derives
        // the new passage offset from the prior passage's confirmed checkpoint.
        self.live_request = Some(previous);
        self.start_live(generation);
    }
}

impl<A: Application> eframe::App for DesktopApp<A> {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.dispatch(AppCommand::Refresh);
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
        let busy = self.view.active_job.is_some_and(|(_, state)| {
            matches!(
                state,
                JobState::Preparing
                    | JobState::Queued
                    | JobState::Running
                    | JobState::Cancelling
                    | JobState::Finalizing
            )
        });
        let busy_live = self.view.active_job.is_some_and(|(job_id, state)| {
            self.live_request
                .as_ref()
                .is_some_and(|request| request.job_id == job_id)
                && matches!(
                    state,
                    JobState::Preparing
                        | JobState::Capturing
                        | JobState::Draining
                        | JobState::Running
                        | JobState::Cancelling
                        | JobState::Finalizing
                )
        });
        let can_enqueue = import_enqueue_enabled(busy, busy_live);
        if ui.ctx().input(|input| input.viewport().close_requested())
            && busy
            && state != Some(JobState::Cancelling)
        {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::CancelClose);
            if self.active_identity.is_some() {
                self.stop_active();
                self.view.message = Some(
                    "Arrêt demandé ; la fenêtre reste ouverte jusqu’à la stabilisation du passage."
                        .into(),
                );
            }
        }
        let destination = self.destination.trim().to_owned();
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
        }
        ui.heading("Whisper — transcription locale");
        ui.label("Import WAV ou MP3 sur le worker CPU avec le modèle D19 approuvé.");

        ui.add_enabled_ui(can_enqueue, |ui| {
            ui.horizontal(|ui| {
                ui.label("Fichier audio (WAV ou MP3)");
                ui.text_edit_singleline(&mut self.source_path);
            });
            ui.horizontal(|ui| {
                ui.label("Modèle local");
                ui.text_edit_singleline(&mut self.model_path);
            });
            ui.horizontal(|ui| {
                ui.label("Dossier des transcriptions");
                ui.text_edit_singleline(&mut self.destination);
            });
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.manual_language, "Langue manuelle");
                if self.manual_language {
                    ui.text_edit_singleline(&mut self.language);
                }
                ui.label("Calcul : CPU");
            });
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
            let can_start = !self.source_path.trim().is_empty()
                && !self.model_path.trim().is_empty()
                && !self.destination.trim().is_empty()
                && (!self.manual_language || !self.language.trim().is_empty());
            if ui
                .add_enabled(can_start, egui::Button::new("Importer et transcrire"))
                .clicked()
            {
                self.start_import();
            }
            let can_start_live = !busy
                && !self.model_path.trim().is_empty()
                && !self.destination.trim().is_empty()
                && (!self.manual_language || !self.language.trim().is_empty());
            if ui
                .add_enabled(can_start_live, egui::Button::new("Démarrer le micro"))
                .clicked()
            {
                self.start_live(Generation::first());
            }
            if state == Some(JobState::Cancelled)
                && let Some(request) = self.live_request.as_ref()
                && let Some(next) = request.generation.next()
                && ui
                    .button("Reprendre — démarrer un nouveau passage")
                    .clicked()
            {
                self.start_live(next);
            }
        });

        if let Some((job_id, state)) = self.view.active_job {
            ui.separator();
            ui.label(format!("Import {job_id:?} — {state:?}"));
            if self.live_request.is_some() {
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
                    if let (Some(captured), Some(available)) = (
                        self.view.provisional_captured_at_unix_ms,
                        self.view.provisional_available_at_unix_ms,
                    ) {
                        let first_rendered =
                            self.provisional_first_rendered_at_unix_ms.unwrap_or(0);
                        ui.label(format!(
                            "Capture Unix {captured} ms | disponibilité Unix {available} ms | premier affichage Unix {first_rendered} ms"
                        ));
                    }
                }
                if let Some(text) = self.view.confirmed_text.as_deref() {
                    ui.label(format!("Texte confirmé : {text}"));
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
            if matches!(
                state,
                JobState::Preparing | JobState::Running | JobState::Finalizing
            ) && ui.button("Arrêter").clicked()
                && self.active_identity.is_some()
            {
                self.stop_active();
            }
        }
        if let Some(message) = &self.view.message {
            ui.colored_label(egui::Color32::LIGHT_RED, message);
        }
        if let Some(result) = &self.view.last_result {
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
                ) && ui.button("Traiter").clicked()
                {
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
            self.dispatch(AppCommand::ScanHistory { destination });
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
            if let Some(recovery) = item.live_recovery
                && !busy
                && self.view.active_job.is_none()
                && ui
                    .button(format!(
                        "Continuer le groupe — nouveau passage ({} échantillons confirmés)",
                        recovery.durable_samples
                    ))
                    .clicked()
            {
                self.continue_live_group(recovery.request);
            }
        }
    }
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

fn speech_limit_samples(enabled: bool, minutes: u64) -> Option<u64> {
    enabled.then(|| minutes.max(1).saturating_mul(60 * 16_000))
}

fn import_enqueue_enabled(busy: bool, busy_live: bool) -> bool {
    !busy || busy_live
}

fn unix_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| {
            duration.as_millis().min(u64::MAX as u128) as u64
        })
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
    use super::{DesktopApp, import_enqueue_enabled, record_first_render, speech_limit_samples};
    use std::{cell::RefCell, rc::Rc};
    use whisper_core::{AppCommand, AppView, Application, ApplicationError, Generation, JobId};

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

    #[test]
    fn auto_stop_is_disabled_by_default_and_uses_speech_sample_clock_when_enabled() {
        assert_eq!(speech_limit_samples(false, 120), None);
        assert_eq!(speech_limit_samples(true, 2), Some(1_920_000));
        assert_eq!(speech_limit_samples(true, 0), Some(960_000));
    }

    #[test]
    fn import_enqueue_stays_available_during_live_capture() {
        assert!(import_enqueue_enabled(true, true));
        assert!(!import_enqueue_enabled(true, false));
        assert!(import_enqueue_enabled(false, false));
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
}
