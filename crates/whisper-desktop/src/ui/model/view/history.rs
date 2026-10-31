use super::*;

impl<A: Application> DesktopApp<A> {
    pub(in crate::ui::model) fn history_ui(
        &mut self,
        ui: &mut egui::Ui,
        busy: bool,
        destination: String,
        destination_scan_pending: bool,
    ) {
        if destination_scan_pending {
            self.scanned_destination = Some(destination.clone());
            self.dispatch(AppCommand::ScanHistory {
                destination: destination.clone(),
            });
        }
        ui.separator();
        ui.heading("Historique local");
        self.replacement_cleanup_ui(ui, busy, &destination);
        self.history_entries_ui(ui, busy, destination);
    }

    fn replacement_cleanup_ui(&mut self, ui: &mut egui::Ui, busy: bool, destination: &str) {
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
                        destination: destination.to_owned(),
                        cleanup_id: cleanup.cleanup_id.clone(),
                    });
                }
            });
        }
    }

    fn history_entries_ui(&mut self, ui: &mut egui::Ui, busy: bool, destination: String) {
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
