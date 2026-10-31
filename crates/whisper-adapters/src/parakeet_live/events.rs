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
                    if self.trace.enabled() {
                        self.trace.record(serde_json::json!({
                            "event": "worker_ready",
                            "job_id": format!("{:032x}", self.request.job_id.0),
                            "generation": self.request.generation.get(),
                            "session_id": self.session_id,
                            "backend": backend,
                            "at_unix_ms": unix_time_ms(),
                        }));
                    }
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
                    native,
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
                    let received_at_unix_ms = unix_time_ms();
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
                    let unresolved_before = self.confirmation.unresolved().to_vec();
                    match self.confirmation.ingest_observed(
                        Hypothesis {
                            sequence,
                            range,
                            words: global_words,
                        },
                        self.trace.enabled(),
                    ) {
                        Ok(decision) => {
                            if self.trace.enabled() {
                                self.trace.record(serde_json::json!({
                                    "event": "window_confirmation",
                                    "job_id": format!("{:032x}", self.request.job_id.0),
                                    "generation": self.request.generation.get(),
                                    "session_id": self.session_id,
                                    "window_sequence": sequence,
                                    "recovery_for": recovery_for.as_ref().map(range_trace_json),
                                    "window_start_sample": range.start_sample,
                                    "window_end_sample": range.end_sample,
                                    "sample_rate_hz": range.sample_rate_hz,
                                    "received_at_unix_ms": received_at_unix_ms,
                                    "captured_at_unix_ms": captured_at_unix_ms,
                                    "available_at_unix_ms": available_at_unix_ms,
                                    "native": native.as_ref().map(native_trace_json),
                                    "cursor_before_sample": decision.cursor_before,
                                    "cursor_after_sample": decision.cursor_after,
                                    "published_at_unix_ms": unix_time_ms(),
                                    "publication_phase": self.publication_phase(),
                                    "candidate_word_count": native.as_ref().map_or(0, |trace| trace.words.len()),
                                    "published_blocks": decision.published.iter().map(block_trace_json).collect::<Vec<_>>(),
                                    "first_refusal": decision.first_refusal.as_ref().map(refusal_trace_json).or_else(|| {
                                        native.as_ref().filter(|trace| trace.words.is_empty()).map(|_| serde_json::json!({
                                            "code": "no_words_returned",
                                            "range": range_trace_json(&range),
                                            "gaps": [],
                                        }))
                                    }),
                                    "unresolved_before": decision.unresolved_before.iter().map(range_trace_json).collect::<Vec<_>>(),
                                    "unresolved_after": decision.unresolved_after.iter().map(range_trace_json).collect::<Vec<_>>(),
                                    "unresolved_ranges_at_receive": unresolved_before.iter().map(range_trace_json).collect::<Vec<_>>(),
                                }));
                            }
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
                            if self.trace.enabled() {
                                self.trace.record(serde_json::json!({
                                    "event": "window_confirmation_rejected",
                                    "job_id": format!("{:032x}", self.request.job_id.0),
                                    "generation": self.request.generation.get(),
                                    "session_id": self.session_id,
                                    "window_sequence": sequence,
                                    "recovery_for": recovery_for.as_ref().map(range_trace_json),
                                    "window_start_sample": range.start_sample,
                                    "window_end_sample": range.end_sample,
                                    "received_at_unix_ms": received_at_unix_ms,
                                    "native": native.as_ref().map(native_trace_json),
                                    "cursor_before_sample": self.confirmation.confirmed_end(),
                                    "cursor_after_sample": self.confirmation.confirmed_end(),
                                    "first_refusal": {
                                        "code": confirmation_error_code(&message),
                                        "message": message,
                                        "range": range_trace_json(&range),
                                        "gaps": unresolved_before.iter().map(range_trace_json).collect::<Vec<_>>(),
                                    },
                                    "publication_phase": self.publication_phase(),
                                }));
                            }
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
                    native,
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
                    if self.trace.enabled() {
                        self.trace.record(serde_json::json!({
                            "event": "window_unusable",
                            "job_id": format!("{:032x}", self.request.job_id.0),
                            "generation": self.request.generation.get(),
                            "session_id": self.session_id,
                            "window_sequence": sequence,
                            "recovery_for": recovery_for.as_ref().map(range_trace_json),
                            "window_start_sample": start,
                            "window_end_sample": end,
                            "received_at_unix_ms": unix_time_ms(),
                            "native": native.as_ref().map(native_trace_json),
                            "cursor_before_sample": self.confirmation.confirmed_end(),
                            "cursor_after_sample": self.confirmation.confirmed_end(),
                            "first_refusal": {
                                "code": window_error_code(&message),
                                "message": message,
                                "range": range_trace_json(&range),
                            },
                            "unresolved_ranges": self.confirmation.unresolved().iter().map(range_trace_json).collect::<Vec<_>>(),
                            "publication_phase": self.publication_phase(),
                        }));
                    }
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
                    if self.trace.enabled() {
                        self.trace.record(serde_json::json!({
                            "event": "worker_unavailable",
                            "job_id": format!("{:032x}", self.request.job_id.0),
                            "generation": self.request.generation.get(),
                            "session_id": self.session_id,
                            "message": bounded_text(&message, TRACE_MAX_TEXT_BYTES).0,
                            "at_unix_ms": unix_time_ms(),
                            "publication_phase": self.publication_phase(),
                        }));
                    }
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
                    if self.trace.enabled() {
                        self.trace.record(serde_json::json!({
                            "event": "worker_stopped",
                            "job_id": format!("{:032x}", self.request.job_id.0),
                            "generation": self.request.generation.get(),
                            "session_id": self.session_id,
                            "at_unix_ms": unix_time_ms(),
                            "publication_phase": self.publication_phase(),
                        }));
                    }
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
        if self.trace.enabled() {
            self.trace.record(serde_json::json!({
                "event": "audio_range_unusable",
                "job_id": format!("{:032x}", self.request.job_id.0),
                "generation": self.request.generation.get(),
                "session_id": self.session_id,
                "window_sequence": sequence,
                "range": range_trace_json(&range),
                "at_unix_ms": unix_time_ms(),
                "reason_code": window_error_code(&message),
                "message": bounded_text(&message, TRACE_MAX_TEXT_BYTES).0,
                "cursor_sample": self.confirmation.confirmed_end(),
                "unresolved_ranges": self.confirmation.unresolved().iter().map(range_trace_json).collect::<Vec<_>>(),
                "publication_phase": self.publication_phase(),
            }));
        }
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

    pub(super) fn publication_phase(&self) -> &'static str {
        if self.finalizing {
            "stop_finalization"
        } else {
            "capture_active"
        }
    }

    pub(super) fn pop_pending_event(&mut self) -> Option<ImportEvent> {
        let event = self.pending_events.pop_front()?;
        if self.trace.enabled() {
            match &event {
                ImportEvent::ParakeetHypothesis {
                    session_id,
                    sequence,
                    range,
                    text,
                    captured_at_unix_ms,
                    available_at_unix_ms,
                    ..
                } => self.trace.record(serde_json::json!({
                    "event": "hypothesis_event_returned",
                    "session_id": session_id,
                    "window_sequence": sequence,
                    "range": range_trace_json(range),
                    "text": bounded_text(text, TRACE_MAX_TEXT_BYTES).0,
                    "captured_at_unix_ms": captured_at_unix_ms,
                    "available_at_unix_ms": available_at_unix_ms,
                    "returned_at_unix_ms": unix_time_ms(),
                    "publication_phase": self.publication_phase(),
                })),
                ImportEvent::ParakeetConfirmed {
                    session_id,
                    segment,
                    ..
                } => self.trace.record(serde_json::json!({
                    "event": "confirmed_event_returned",
                    "session_id": session_id,
                    "segment_id": segment.segment_id.0,
                    "range": range_trace_json(&segment.range),
                    "text": bounded_text(&segment.text, TRACE_MAX_TEXT_BYTES).0,
                    "returned_at_unix_ms": unix_time_ms(),
                    "publication_phase": self.publication_phase(),
                })),
                _ => {}
            }
        }
        Some(event)
    }
}
