use eframe::egui;
use std::sync::atomic::{AtomicU64, Ordering};
use whisper_core::{
    AppCommand, AppFacade, AppView, Application, ApplicationError, ComputeChoice, Generation,
    ImportRequest, JobConfig, JobId, JobState, LanguageChoice,
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
    active_identity: Option<(JobId, Generation)>,
    scanned_destination: Option<String>,
    observed_job_state: Option<JobState>,
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
            active_identity: None,
            scanned_destination: None,
            observed_job_state: None,
        }
    }

    fn dispatch(&mut self, command: AppCommand) {
        match self.application.dispatch(command) {
            Ok(view) => self.view = view,
            Err(ApplicationError::Failed(message)) => self.view.message = Some(message),
            Err(error) => self.view.message = Some(format!("Commande impossible : {error:?}")),
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
        self.active_identity = Some((job_id, generation));
        self.dispatch(AppCommand::StartImport {
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
                    | JobState::Running
                    | JobState::Cancelling
                    | JobState::Finalizing
            )
        });
        if !busy {
            self.active_identity = None;
        }
        ui.heading("Whisper — transcription locale");
        ui.label("Import WAV sur le worker CPU avec le modèle D19 approuvé.");

        ui.add_enabled_ui(!busy, |ui| {
            ui.horizontal(|ui| {
                ui.label("Fichier WAV");
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
        });

        if let Some((job_id, state)) = self.view.active_job {
            ui.separator();
            ui.label(format!("Import {job_id:?} — {state:?}"));
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
            ) && ui.button("Arrêter l’import").clicked()
                && let Some((job, generation)) = self.active_identity
            {
                self.dispatch(AppCommand::Stop {
                    job_id: job,
                    generation,
                });
            }
        }
        if let Some(message) = &self.view.message {
            ui.colored_label(egui::Color32::LIGHT_RED, message);
        }
        if let Some(result) = &self.view.last_result {
            ui.colored_label(egui::Color32::LIGHT_GREEN, result);
        }

        if self.scanned_destination.as_deref() != Some(self.destination.trim())
            && !self.destination.trim().is_empty()
        {
            self.scanned_destination = Some(self.destination.trim().to_owned());
            self.dispatch(AppCommand::ScanHistory {
                destination: self.destination.trim().to_owned(),
            });
        }
        ui.separator();
        ui.heading("Historique local");
        if self.view.history.is_empty() {
            ui.label("Aucune transcription publiée dans ce dossier.");
        }
        for item in &self.view.history {
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
