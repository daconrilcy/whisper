use super::*;

#[derive(Default)]
pub(super) struct RecoveryAttemptLedger {
    ranges: Vec<SourceRange>,
}

impl RecoveryAttemptLedger {
    pub(super) fn len(&self) -> usize {
        self.ranges.len()
    }

    pub(super) fn contains_overlap(&self, range: SourceRange) -> bool {
        self.ranges.iter().any(|attempted| {
            attempted.start_sample < range.end_sample && range.start_sample < attempted.end_sample
        })
    }

    pub(super) fn record(&mut self, range: SourceRange) {
        if self.ranges.len() < MAX_RECOVERY_ATTEMPTS && !self.contains_overlap(range) {
            self.ranges.push(range);
        }
    }
}

impl ParakeetLive {
    pub(super) fn schedule_window(&mut self, final_window: bool) -> Result<(), String> {
        if self.inflight || self.unavailable || !self.ready_sent {
            return Ok(());
        }
        let total = self.wav.as_ref().map_or(0, WavOutput::sample_count);
        if !final_window && !self.finalizing {
            let next_global_start = self
                .request
                .group_offset_samples
                .saturating_add(self.next_window_start);
            if self.recovery_attempts.len() < MAX_RECOVERY_ATTEMPTS
                && let Some(target) = self
                    .confirmation
                    .recovery_candidates(next_global_start)
                    .into_iter()
                    .find(|range| !self.recovery_attempts.contains_overlap(*range))
                && let Some((start, count)) =
                    recovery_window(target, self.request.group_offset_samples, total)
            {
                match self.submit_window(start, count, false, Some(target)) {
                    Ok(true) => {
                        self.recovery_attempts.record(target);
                        if self.trace.enabled() {
                            self.trace.record(serde_json::json!({
                                "event": "recovery_scheduled",
                                "job_id": format!("{:032x}", self.request.job_id.0),
                                "generation": self.request.generation.get(),
                                "session_id": self.session_id,
                                "target_range": range_trace_json(&target),
                                "window_start_sample": self.request.group_offset_samples.saturating_add(start),
                                "window_end_sample": self.request.group_offset_samples.saturating_add(start).saturating_add(count),
                                "attempt_for_range": 1,
                                "attempt_limit_per_range": 1,
                                "session_attempt": self.recovery_attempts.len(),
                                "session_attempt_limit": MAX_RECOVERY_ATTEMPTS,
                                "at_unix_ms": unix_time_ms(),
                                "publication_phase": "capture_active",
                            }));
                        }
                        return Ok(());
                    }
                    Ok(false) if self.unavailable => return Ok(()),
                    Ok(false) => {}
                    Err(error) => {
                        self.recovery_attempts.record(target);
                        if self.trace.enabled() {
                            self.trace.record(serde_json::json!({
                                "event": "recovery_not_submitted",
                                "job_id": format!("{:032x}", self.request.job_id.0),
                                "generation": self.request.generation.get(),
                                "session_id": self.session_id,
                                "target_range": range_trace_json(&target),
                                "reason_code": "recovery_window_preparation_failed",
                                "error": bounded_text(&error, TRACE_MAX_TEXT_BYTES).0,
                                "at_unix_ms": unix_time_ms(),
                                "publication_phase": "capture_active",
                            }));
                        }
                    }
                }
            }
        }
        let lag = total.saturating_sub(self.next_window_start);
        if self.next_sequence > 2 && lag > MAX_WINDOW_LAG_SAMPLES {
            let target_start = total
                .saturating_sub(WINDOW_SAMPLES)
                .checked_div(WINDOW_STEP_SAMPLES)
                .unwrap_or(0)
                .saturating_mul(WINDOW_STEP_SAMPLES);
            let skipped_start = if self.next_sequence == 1 && self.next_window_start == 0 {
                // No inference window has ever been submitted, so none of the
                // captured audio is covered by a native hypothesis yet.
                0
            } else {
                // A submitted window starts two seconds before the next cursor;
                // this is the end of the latest actually submitted window.
                self.next_window_start
                    .saturating_add(WINDOW_SAMPLES - WINDOW_STEP_SAMPLES)
            };
            if target_start > skipped_start {
                let start = self
                    .request
                    .group_offset_samples
                    .checked_add(skipped_start)
                    .ok_or_else(|| "Début de retard hors chronologie".to_owned())?;
                let end = self
                    .request
                    .group_offset_samples
                    .checked_add(target_start)
                    .ok_or_else(|| "Fin de retard hors chronologie".to_owned())?;
                if let Some(range) = SourceRange::new(start, end, SAMPLE_RATE as u32) {
                    self.queue_audio_unusable(
                        None,
                        range,
                        "Fenêtres non inférées : retard du moteur supérieur à 8 secondes".into(),
                    );
                }
                self.next_window_start = target_start;
            }
        }
        let remaining = total.saturating_sub(self.next_window_start);
        let count = if self.next_sequence == 1 && remaining >= WINDOW_STEP_SAMPLES {
            WINDOW_STEP_SAMPLES
        } else if remaining >= WINDOW_SAMPLES {
            WINDOW_SAMPLES
        } else if final_window && remaining >= SAMPLE_RATE {
            remaining
        } else {
            return Ok(());
        };
        let start = self.next_window_start;
        self.submit_window(start, count, true, None).map(|_| ())
    }

    pub(super) fn submit_window(
        &mut self,
        start: u64,
        count: u64,
        advance_cursor: bool,
        recovery_for: Option<SourceRange>,
    ) -> Result<bool, String> {
        if self.inflight || self.unavailable || !self.ready_sent {
            return Ok(false);
        }
        let end = start
            .checked_add(count)
            .ok_or_else(|| "Chronologie Parakeet épuisée".to_owned())?;
        if end > self.wav.as_ref().map_or(0, WavOutput::sample_count) {
            return Err("Fenêtre hors du WAV conservé".into());
        }
        let samples = self
            .wav
            .as_mut()
            .expect("active WAV")
            .read_samples(start, count as usize)?;
        let sequence = self.next_sequence;
        let path = self.window_root.join(format!("window-{sequence}.wav"));
        write_pcm16_wav(&path, &samples)?;
        let sha256 = crate::parakeet_import::sha256_file(&path)?;
        let captured_at_unix_ms = self
            .capture_origin_unix_ms
            .unwrap_or_else(unix_time_ms)
            .saturating_add(end.saturating_mul(1_000) / SAMPLE_RATE);
        let request = InferenceRequest::Transcribe {
            sequence,
            start_sample: start,
            samples,
            path: path.clone(),
            sha256,
            captured_at_unix_ms,
            recovery_for,
        };
        match self.inference_request.try_send(request) {
            Ok(()) => {
                self.inflight = true;
                self.next_sequence = sequence.saturating_add(1);
                if advance_cursor {
                    self.next_window_start = if sequence == 1 {
                        start
                    } else {
                        start.saturating_add(WINDOW_STEP_SAMPLES)
                    };
                }
                if self.trace.enabled() {
                    let global_start = self.request.group_offset_samples.saturating_add(start);
                    let global_end = self.request.group_offset_samples.saturating_add(end);
                    self.trace.record(serde_json::json!({
                        "event": "window_submitted",
                        "job_id": format!("{:032x}", self.request.job_id.0),
                        "generation": self.request.generation.get(),
                        "session_id": self.session_id,
                        "window_sequence": sequence,
                        "window_kind": if recovery_for.is_some() { "recovery" } else { "regular" },
                        "recovery_for": recovery_for.as_ref().map(range_trace_json),
                        "window_start_sample": global_start,
                        "window_end_sample": global_end,
                        "sample_rate_hz": SAMPLE_RATE,
                        "captured_at_unix_ms": captured_at_unix_ms,
                        "submitted_at_unix_ms": unix_time_ms(),
                        "publication_phase": self.publication_phase(),
                    }));
                }
                Ok(true)
            }
            Err(TrySendError::Full(_)) => {
                let _ = fs::remove_file(path);
                Ok(false)
            }
            Err(TrySendError::Disconnected(_)) => {
                let _ = fs::remove_file(path);
                self.unavailable = true;
                self.pending_events
                    .push_back(ImportEvent::ParakeetInferenceUnavailable {
                        job_id: self.request.job_id,
                        generation: self.request.generation,
                        session_id: self.session_id,
                        message: "Le worker Parakeet ne reçoit plus de fenêtres".into(),
                    });
                Ok(false)
            }
        }
    }
}

pub(super) fn recovery_window(
    target: SourceRange,
    group_offset_samples: u64,
    durable_samples: u64,
) -> Option<(u64, u64)> {
    if target.sample_rate_hz != SAMPLE_RATE as u32 {
        return None;
    }
    let local_start = target.start_sample.checked_sub(group_offset_samples)?;
    let local_end = target.end_sample.checked_sub(group_offset_samples)?;
    if local_start >= local_end || local_end > durable_samples {
        return None;
    }
    let target_len = local_end - local_start;
    if target_len.saturating_add(RECOVERY_MIN_CONTEXT_SAMPLES * 2) > WINDOW_SAMPLES {
        return None;
    }
    let centered = local_start.saturating_add(target_len / 2);
    let desired_start = centered.saturating_sub(WINDOW_SAMPLES / 2);
    let latest_start = durable_samples.checked_sub(WINDOW_SAMPLES)?;
    let start = desired_start.min(latest_start);
    let end = start.checked_add(WINDOW_SAMPLES)?;
    if local_start.saturating_sub(start) < RECOVERY_MIN_CONTEXT_SAMPLES
        || end.saturating_sub(local_end) < RECOVERY_MIN_CONTEXT_SAMPLES
    {
        return None;
    }
    Some((start, WINDOW_SAMPLES))
}

pub(super) fn window_result_matches(
    expected: WindowResultIdentity<'_>,
    actual: WindowResultIdentity<'_>,
    native_call_marker_required: bool,
    native_call_started: bool,
) -> bool {
    actual.request_id == expected.request_id
        && actual.job_id == expected.job_id
        && actual.generation == expected.generation
        && actual.attempt == expected.attempt
        && actual.identity == expected.identity
        && actual
            .source_sha256
            .eq_ignore_ascii_case(expected.source_sha256)
        && actual.duration_samples == expected.duration_samples
        && (!native_call_marker_required || native_call_started)
}

pub(super) fn validate_words(
    words: &[Word],
    window_samples: u64,
) -> Result<Vec<AbsoluteWordSamples>, String> {
    let window = Window {
        index: 0,
        central_start_sample: 0,
        central_end_sample: window_samples,
        native_start_sample: 0,
        native_end_sample: window_samples,
        cut: None,
    };
    let absolute = absolute_word_samples(&window, window_samples, words)
        .map_err(|error| format!("Horodatage Parakeet invalide : {error}"))?;
    let mut previous_end = 0;
    for (word, range) in words.iter().zip(&absolute) {
        if word.text.trim().is_empty() || range.start_sample < previous_end {
            return Err("Horodatages Parakeet vides ou inversés".into());
        }
        previous_end = range.end_sample;
    }
    Ok(absolute)
}
