use super::*;

impl ParakeetLive {
    pub(crate) fn poll(&mut self) -> Result<Option<ImportEvent>, String> {
        if self.telemetry_due_after_event {
            self.telemetry_due_after_event = false;
            if let Some(event) = self.take_next_probe_telemetry() {
                return Ok(Some(event));
            }
        }
        if let Some(event) = self.pop_pending_event() {
            self.telemetry_due_after_event = true;
            return Ok(Some(event));
        }
        loop {
            match self.inference_events.try_recv() {
                Ok(InferenceEvent::Ready { backend }) => {
                    #[cfg(test)]
                    self.d2_worker_diagnostics.record(serde_json::json!({
                        "event":"parakeet-live-poll-ready",
                        "backend":backend.clone(),
                        "at_unix_ms":unix_time_ms(),
                    }));
                    if !self.ready_sent {
                        self.ready_sent = true;
                        self.pending_events
                            .push_back(ImportEvent::ParakeetInferenceReady {
                                job_id: self.request.job_id,
                                generation: self.request.generation,
                                session_id: self.session_id,
                                backend,
                            });
                    }
                }
                Ok(InferenceEvent::Result {
                    sequence,
                    start_sample,
                    end_sample,
                    text,
                    words,
                    captured_at_unix_ms,
                    available_at_unix_ms,
                    recovery_for,
                }) => {
                    #[cfg(test)]
                    self.d2_worker_diagnostics.record(serde_json::json!({
                        "event":"parakeet-live-poll-result",
                        "sequence":sequence,
                        "start_sample":start_sample,
                        "end_sample":end_sample,
                        "text_bytes":text.len(),
                        "word_count":words.len(),
                        "recovery_for":recovery_for.as_ref().map(|range| format!("{range:?}")),
                        "at_unix_ms":unix_time_ms(),
                    }));
                    self.inflight = false;
                    let global_start = self
                        .request
                        .group_offset_samples
                        .checked_add(start_sample)
                        .ok_or_else(|| "Début de fenêtre hors chronologie".to_owned())?;
                    let global_end = self
                        .request
                        .group_offset_samples
                        .checked_add(end_sample)
                        .ok_or_else(|| "Fin de fenêtre hors chronologie".to_owned())?;
                    let range = SourceRange::new(global_start, global_end, SAMPLE_RATE as u32)
                        .ok_or_else(|| "Fenêtre Parakeet hors chronologie".to_owned())?;
                    if recovery_for.is_none() && !text.trim().is_empty() {
                        self.pending_events
                            .push_back(ImportEvent::ParakeetHypothesis {
                                job_id: self.request.job_id,
                                generation: self.request.generation,
                                session_id: self.session_id,
                                sequence,
                                range,
                                text,
                                captured_at_unix_ms,
                                available_at_unix_ms,
                            });
                    }
                    let global_words = words
                        .into_iter()
                        .map(|mut word| {
                            word.range.start_sample =
                                word.range.start_sample.saturating_add(global_start);
                            word.range.end_sample =
                                word.range.end_sample.saturating_add(global_start);
                            word
                        })
                        .collect();
                    match self.confirmation.ingest_observed(Hypothesis {
                        sequence,
                        range,
                        words: global_words,
                    }) {
                        Ok(decision) => {
                            for resolved in decision
                                .unresolved_before
                                .iter()
                                .filter(|range| !decision.unresolved_after.contains(range))
                            {
                                self.pending_events
                                    .push_back(ImportEvent::ParakeetResolved {
                                        job_id: self.request.job_id,
                                        generation: self.request.generation,
                                        session_id: self.session_id,
                                        range: *resolved,
                                    });
                            }
                            for block in decision.published {
                                self.pending_events
                                    .push_back(ImportEvent::ParakeetConfirmed {
                                        job_id: self.request.job_id,
                                        generation: self.request.generation,
                                        session_id: self.session_id,
                                        segment: whisper_core::ports::WorkerSegment {
                                            job_id: self.request.job_id,
                                            generation: self.request.generation,
                                            instance_id: self.session_id,
                                            segment_id: SegmentId(block.segment_id),
                                            range: block.range,
                                            text: block.text,
                                        },
                                    });
                            }
                            for unresolved in decision
                                .unresolved_after
                                .iter()
                                .filter(|range| !decision.unresolved_before.contains(range))
                            {
                                self.pending_events
                                    .push_back(ImportEvent::ParakeetUnresolved {
                                        job_id: self.request.job_id,
                                        generation: self.request.generation,
                                        session_id: self.session_id,
                                        range: *unresolved,
                                    });
                            }
                        }
                        Err(message) => {
                            if recovery_for.is_none() {
                                self.queue_audio_unusable(Some(sequence), range, message);
                            }
                        }
                    }
                    self.remove_completed_window(sequence);
                }
                Ok(InferenceEvent::WindowUnusable {
                    sequence,
                    start_sample,
                    end_sample,
                    message,
                    recovery_for,
                }) => {
                    #[cfg(test)]
                    self.d2_worker_diagnostics.record(serde_json::json!({
                        "event":"parakeet-live-poll-window-unusable",
                        "sequence":sequence,
                        "start_sample":start_sample,
                        "end_sample":end_sample,
                        "message":message.clone(),
                        "at_unix_ms":unix_time_ms(),
                    }));
                    self.inflight = false;
                    let start = self
                        .request
                        .group_offset_samples
                        .checked_add(start_sample)
                        .ok_or_else(|| "Début de fenêtre hors chronologie".to_owned())?;
                    let end = self
                        .request
                        .group_offset_samples
                        .checked_add(end_sample)
                        .ok_or_else(|| "Fin de fenêtre hors chronologie".to_owned())?;
                    let range = SourceRange::new(start, end, SAMPLE_RATE as u32)
                        .ok_or_else(|| "Fenêtre Parakeet hors chronologie".to_owned())?;
                    if recovery_for.is_none() {
                        self.queue_audio_unusable(Some(sequence), range, message);
                    }
                    self.remove_completed_window(sequence);
                }
                Ok(InferenceEvent::Unavailable(message)) => {
                    #[cfg(test)]
                    self.d2_worker_diagnostics.record(serde_json::json!({
                        "event":"parakeet-live-poll-unavailable",
                        "message":message.clone(),
                        "at_unix_ms":unix_time_ms(),
                    }));
                    self.inflight = false;
                    self.unavailable = true;
                    self.pending_events
                        .push_back(ImportEvent::ParakeetInferenceUnavailable {
                            job_id: self.request.job_id,
                            generation: self.request.generation,
                            session_id: self.session_id,
                            message,
                        });
                    break;
                }
                Ok(InferenceEvent::Stopped) => {
                    #[cfg(test)]
                    self.d2_worker_diagnostics.record(serde_json::json!({
                        "event":"parakeet-live-poll-stopped",
                        "at_unix_ms":unix_time_ms(),
                    }));
                    break;
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    #[cfg(test)]
                    self.d2_worker_diagnostics.record(serde_json::json!({
                        "event":"parakeet-live-poll-disconnected",
                        "at_unix_ms":unix_time_ms(),
                    }));
                    self.inflight = false;
                    if !self.unavailable && !self.stopped {
                        self.unavailable = true;
                        self.pending_events
                            .push_back(ImportEvent::ParakeetInferenceUnavailable {
                                job_id: self.request.job_id,
                                generation: self.request.generation,
                                session_id: self.session_id,
                                message: "Le canal du worker Parakeet est interrompu".into(),
                            });
                    }
                    break;
                }
            }
        }
        if let Some(event) = self.pop_pending_event() {
            self.telemetry_due_after_event = true;
            return Ok(Some(event));
        }
        self.schedule_window(false)?;
        Ok(self.take_next_probe_telemetry())
    }

    pub(super) fn remove_completed_window(&self, sequence: u64) {
        let _ = fs::remove_file(self.window_root.join(format!("window-{sequence}.wav")));
    }

    pub(super) fn queue_progress(&mut self, event: ImportEvent) {
        let session_id = self.session_id;
        self.pending_events.retain(|queued| {
            !matches!(queued, ImportEvent::ParakeetCaptureProgress { session_id: queued_session, .. } if *queued_session == session_id)
        });
        self.pending_events.push_back(event);
    }

    pub(super) fn queue_audio_unusable(
        &mut self,
        sequence: Option<u64>,
        range: SourceRange,
        message: String,
    ) {
        self.confirmation.mark_unresolved(range);
        self.pending_events
            .push_back(ImportEvent::ParakeetUnresolved {
                job_id: self.request.job_id,
                generation: self.request.generation,
                session_id: self.session_id,
                range,
            });
        self.pending_events
            .push_back(ImportEvent::ParakeetAudioUnusable {
                job_id: self.request.job_id,
                generation: self.request.generation,
                session_id: self.session_id,
                sequence,
                range,
                message,
            });
    }

    pub(super) fn pop_pending_event(&mut self) -> Option<ImportEvent> {
        let event = self.pending_events.pop_front()?;
        Some(event)
    }
}
