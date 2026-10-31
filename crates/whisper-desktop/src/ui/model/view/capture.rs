use super::*;

impl<A: Application> DesktopApp<A> {
    pub(in crate::ui::model) fn render_parakeet_capture_ui(&mut self, ui: &mut egui::Ui) {
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
        self.capture_details_ui(ui, capture);
        self.capture_automatic_final_ui(ui, capture);
        self.capture_text_selector_ui(ui, capture);
        self.capture_text_ui(ui, capture);
    }

    fn capture_details_ui(
        &mut self,
        ui: &mut egui::Ui,
        capture: &whisper_core::ports::ParakeetCaptureView,
    ) {
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
    }

    fn capture_automatic_final_ui(
        &mut self,
        ui: &mut egui::Ui,
        capture: &whisper_core::ports::ParakeetCaptureView,
    ) {
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
    }

    fn capture_text_selector_ui(
        &mut self,
        ui: &mut egui::Ui,
        capture: &whisper_core::ports::ParakeetCaptureView,
    ) {
        let final_view = capture.automatic_final.as_ref();
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
    }

    fn capture_text_ui(
        &mut self,
        ui: &mut egui::Ui,
        capture: &whisper_core::ports::ParakeetCaptureView,
    ) {
        let final_view = capture.automatic_final.as_ref();
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
}
