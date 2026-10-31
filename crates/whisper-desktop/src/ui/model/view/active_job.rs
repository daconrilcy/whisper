use super::*;

impl<A: Application> DesktopApp<A> {
    pub(in crate::ui::model) fn active_job_ui(&mut self, ui: &mut egui::Ui) {
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
            self.active_live_metrics_ui(ui, job_id);
            self.active_live_text_ui(ui, job_id, state);
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
    }

    fn active_live_metrics_ui(&mut self, ui: &mut egui::Ui, job_id: JobId) {
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
    }

    fn active_live_text_ui(&mut self, ui: &mut egui::Ui, job_id: JobId, state: JobState) {
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
    }
}
