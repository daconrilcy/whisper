use super::*;

impl<A: Application> DesktopApp<A> {
    pub(in crate::ui::model) fn render_unified_parakeet_ui(&mut self, ui: &mut egui::Ui) {
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
        self.unified_session_status(ui, session);
        let (document_current, parts_current) = self.request_unified_pages(session, &selected);
        self.unified_document_ui(ui, session, &selected, document_current);
        self.unified_audio_ui(ui, session, &selected, parts_current);
    }

    fn unified_session_status(
        &mut self,
        ui: &mut egui::Ui,
        session: &whisper_core::ports::UnifiedParakeetSessionView,
    ) {
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
    }

    fn request_unified_pages(
        &mut self,
        session: &whisper_core::ports::UnifiedParakeetSessionView,
        selected: &whisper_core::ports::UnifiedParakeetSelectionView,
    ) -> (bool, bool) {
        let identity = (session.job_id, session.generation);
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

        (document_current, parts_current)
    }

    fn unified_document_ui(
        &mut self,
        ui: &mut egui::Ui,
        session: &whisper_core::ports::UnifiedParakeetSessionView,
        selected: &whisper_core::ports::UnifiedParakeetSelectionView,
        document_current: bool,
    ) {
        let identity = (session.job_id, session.generation);
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
    }

    fn unified_audio_ui(
        &mut self,
        ui: &mut egui::Ui,
        session: &whisper_core::ports::UnifiedParakeetSessionView,
        selected: &whisper_core::ports::UnifiedParakeetSelectionView,
        parts_current: bool,
    ) {
        let identity = (session.job_id, session.generation);
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
                        let _ = ui.selectable_label(false, part.path.as_str());
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
                let _ = ui.selectable_label(false, path);
            }
            if let Some(path) = session.srt_path.as_deref() {
                ui.label("SRT");
                let _ = ui.selectable_label(false, path);
            }
        });
        if (session.txt_path.is_none() || session.srt_path.is_none())
            && session.phase != whisper_core::ports::UnifiedSessionPhase::Complete
        {
            ui.weak("Les chemins TXT et SRT seront affichés après publication complète.");
        }
    }
}
