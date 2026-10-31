use super::*;

impl<A: Application> DesktopApp<A> {
    pub(in crate::ui::model) fn dialogs_ui(&mut self, ui: &mut egui::Ui) {
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
    }
}
