use super::*;

#[derive(Clone)]
#[cfg_attr(not(test), derive(Default))]
pub(super) struct D2WorkerDiagnostics {
    #[cfg(test)]
    path: Option<PathBuf>,
    #[cfg(test)]
    write_lock: Arc<std::sync::Mutex<()>>,
}

#[cfg(test)]
impl Default for D2WorkerDiagnostics {
    fn default() -> Self {
        Self {
            path: None,
            write_lock: Arc::new(std::sync::Mutex::new(())),
        }
    }
}

impl D2WorkerDiagnostics {
    #[cfg(test)]
    pub(super) fn for_d2_destination(destination: &Path) -> Result<Self, String> {
        let root = env::var_os("WHISPER_MIC_LEVEL_D2_ROOT")
            .map(PathBuf::from)
            .ok_or_else(|| "D2 diagnostics root is absent".to_owned())?
            .canonicalize()
            .map_err(|error| format!("D2 diagnostics root: {error}"))?;
        let destination = destination
            .canonicalize()
            .map_err(|error| format!("D2 case root: {error}"))?;
        if !destination.starts_with(&root) {
            return Err("D2 diagnostics destination is outside the guarded root".into());
        }
        Ok(Self {
            path: Some(destination.join("d2-worker-events.jsonl")),
            write_lock: Arc::new(std::sync::Mutex::new(())),
        })
    }

    pub(super) fn enabled(&self) -> bool {
        #[cfg(test)]
        {
            return self.path.is_some();
        }
        #[cfg(not(test))]
        false
    }

    pub(super) fn record(&self, value: serde_json::Value) {
        #[cfg(test)]
        if let Some(path) = &self.path {
            if let Ok(_guard) = self.write_lock.lock()
                && let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path)
            {
                let _ = serde_json::to_writer(&mut file, &value);
                let _ = file.write_all(b"\n");
            }
        }
        #[cfg(not(test))]
        let _ = value;
    }
}

enum TraceCommand {
    Record(Vec<u8>),
    Finish,
}

pub(super) struct TraceSink {
    sender: Option<SyncSender<TraceCommand>>,
    writer: Option<JoinHandle<()>>,
    dropped_records: Arc<AtomicU64>,
    truncated_records: Arc<AtomicU64>,
    writer_failed: Arc<AtomicBool>,
}

impl TraceSink {
    pub(super) fn from_environment() -> Self {
        let Some(configured_path) = env::var_os(TRACE_ENV) else {
            return Self::disabled();
        };
        if configured_path.is_empty() {
            return Self::disabled();
        }
        match Self::start(PathBuf::from(configured_path)) {
            Ok(trace) => trace,
            Err(error) => {
                // Tracing is diagnostic only: an unavailable trace destination
                // must not change microphone capture or inference behavior.
                eprintln!("Parakeet confirmation trace disabled: {error}");
                Self::disabled()
            }
        }
    }

    pub(super) fn disabled() -> Self {
        Self {
            sender: None,
            writer: None,
            dropped_records: Arc::new(AtomicU64::new(0)),
            truncated_records: Arc::new(AtomicU64::new(0)),
            writer_failed: Arc::new(AtomicBool::new(false)),
        }
    }

    fn start(path: PathBuf) -> Result<Self, String> {
        if !path.is_absolute() {
            return Err("trace path must be absolute".into());
        }
        let allowed_root = Path::new(TRACE_ROOT)
            .canonicalize()
            .map_err(|error| format!("trace root is unavailable: {error}"))?;
        let parent = path
            .parent()
            .ok_or_else(|| "trace path has no parent directory".to_owned())?
            .canonicalize()
            .map_err(|error| format!("trace parent is unavailable: {error}"))?;
        if !parent.starts_with(&allowed_root) {
            return Err("trace path is outside the authorized recipe root".into());
        }
        let file_name = path
            .file_name()
            .ok_or_else(|| "trace path has no file name".to_owned())?;
        let canonical_path = parent.join(file_name);
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&canonical_path)
            .map_err(|error| format!("trace file was not created: {error}"))?;
        let (sender, receiver) = mpsc::sync_channel(TRACE_QUEUE_CAPACITY);
        let dropped_records = Arc::new(AtomicU64::new(0));
        let truncated_records = Arc::new(AtomicU64::new(0));
        let writer_failed = Arc::new(AtomicBool::new(false));
        let writer_dropped = Arc::clone(&dropped_records);
        let writer_truncated = Arc::clone(&truncated_records);
        let writer_failed_flag = Arc::clone(&writer_failed);
        let writer = thread::Builder::new()
            .name("parakeet-trace-writer".into())
            .spawn(move || {
                write_trace(
                    file,
                    receiver,
                    writer_dropped,
                    writer_truncated,
                    writer_failed_flag,
                )
            })
            .map_err(|error| format!("trace writer was not started: {error}"))?;
        Ok(Self {
            sender: Some(sender),
            writer: Some(writer),
            dropped_records,
            truncated_records,
            writer_failed,
        })
    }

    pub(super) fn enabled(&self) -> bool {
        self.sender.is_some()
    }

    pub(super) fn record(&self, value: serde_json::Value) {
        let Some(sender) = &self.sender else {
            return;
        };
        let mut bytes = match serde_json::to_vec(&value) {
            Ok(bytes) => bytes,
            Err(_) => {
                self.dropped_records.fetch_add(1, Ordering::Relaxed);
                return;
            }
        };
        if bytes.len() > TRACE_MAX_RECORD_BYTES {
            self.truncated_records.fetch_add(1, Ordering::Relaxed);
            bytes = serde_json::to_vec(&serde_json::json!({
                "event": "trace_record_truncated",
                "original_bytes": bytes.len(),
            }))
            .unwrap_or_default();
        }
        match sender.try_send(TraceCommand::Record(bytes)) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {
                self.dropped_records.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    pub(super) fn finish(&mut self) {
        if let Some(sender) = self.sender.take() {
            let _ = sender.send(TraceCommand::Finish);
        }
        if let Some(writer) = self.writer.take() {
            let _ = writer.join();
        }
        if self.writer_failed.load(Ordering::Relaxed) {
            eprintln!("Parakeet confirmation trace ended after a write error");
        }
    }
}

impl Drop for TraceSink {
    fn drop(&mut self) {
        self.finish();
    }
}

fn write_trace(
    mut file: File,
    receiver: Receiver<TraceCommand>,
    dropped_records: Arc<AtomicU64>,
    truncated_records: Arc<AtomicU64>,
    writer_failed: Arc<AtomicBool>,
) {
    let mut bytes_written = 0_u64;
    let mut size_limited = false;
    while let Ok(command) = receiver.recv() {
        match command {
            TraceCommand::Record(bytes) => {
                if size_limited || writer_failed.load(Ordering::Relaxed) {
                    dropped_records.fetch_add(1, Ordering::Relaxed);
                    continue;
                }
                let record_size = bytes.len() as u64 + 1;
                let normal_limit = TRACE_MAX_FILE_BYTES.saturating_sub(TRACE_TERMINAL_RESERVE);
                if bytes_written.saturating_add(record_size) > normal_limit {
                    size_limited = true;
                    truncated_records.fetch_add(1, Ordering::Relaxed);
                    let marker = serde_json::to_vec(&serde_json::json!({
                        "event": "trace_truncated",
                        "reason": "file_size_limit",
                    }))
                    .unwrap_or_default();
                    write_trace_line(&mut file, &marker, &mut bytes_written, &writer_failed);
                    continue;
                }
                write_trace_line(&mut file, &bytes, &mut bytes_written, &writer_failed);
            }
            TraceCommand::Finish => {
                if !writer_failed.load(Ordering::Relaxed) {
                    let summary = serde_json::to_vec(&serde_json::json!({
                        "event": "trace_finished",
                        "dropped_records": dropped_records.load(Ordering::Relaxed),
                        "truncated_records": truncated_records.load(Ordering::Relaxed),
                        "size_limited": size_limited,
                    }))
                    .unwrap_or_default();
                    if bytes_written.saturating_add(summary.len() as u64 + 1)
                        <= TRACE_MAX_FILE_BYTES
                    {
                        write_trace_line(&mut file, &summary, &mut bytes_written, &writer_failed);
                    }
                }
                let _ = file.flush();
                return;
            }
        }
    }
    let _ = file.flush();
}

fn write_trace_line(file: &mut File, bytes: &[u8], written: &mut u64, failed: &AtomicBool) {
    if file
        .write_all(bytes)
        .and_then(|()| file.write_all(b"\n"))
        .is_err()
    {
        failed.store(true, Ordering::Relaxed);
    } else {
        *written = written.saturating_add(bytes.len() as u64 + 1);
    }
}

pub(super) fn native_hypothesis_trace(
    text: &str,
    words: &[Word],
    global_window_start_sample: u64,
    validation: &Result<Vec<AbsoluteWordSamples>, String>,
) -> NativeHypothesisTrace {
    let (text, text_truncated) = bounded_text(text, TRACE_MAX_TEXT_BYTES);
    let mut truncated = text_truncated || words.len() > TRACE_MAX_WORDS;
    let mut previous_end_sample = 0_u64;
    let mut traced_words = Vec::with_capacity(words.len().min(TRACE_MAX_WORDS));
    for (index, word) in words.iter().take(TRACE_MAX_WORDS).enumerate() {
        let (word_text, word_truncated) = bounded_text(&word.text, TRACE_MAX_WORD_TEXT_BYTES);
        truncated |= word_truncated;
        let raw_start = u64::from(word.start_ms).saturating_mul(SAMPLE_RATE) / 1_000;
        let raw_end = u64::from(word.end_ms).saturating_mul(SAMPLE_RATE) / 1_000;
        let effective = validation
            .as_ref()
            .ok()
            .and_then(|ranges| ranges.get(index))
            .copied();
        let ordered_in_hypothesis =
            effective.is_some_and(|range| range.start_sample >= previous_end_sample);
        if let Some(range) = effective.filter(|_| ordered_in_hypothesis) {
            previous_end_sample = range.end_sample;
        }
        traced_words.push(NativeWordTrace {
            text: word_text.to_owned(),
            start_ms: word.start_ms,
            end_ms: word.end_ms,
            raw_global_start_sample: global_window_start_sample.saturating_add(raw_start),
            raw_global_end_sample: global_window_start_sample.saturating_add(raw_end),
            effective_global_start_sample: effective
                .map(|range| global_window_start_sample.saturating_add(range.start_sample)),
            effective_global_end_sample: effective
                .map(|range| global_window_start_sample.saturating_add(range.end_sample)),
            native_span_shape_valid: raw_start < raw_end,
            accepted_by_validation: effective.is_some(),
            ordered_in_hypothesis,
        });
    }
    NativeHypothesisTrace {
        text: text.to_owned(),
        words: traced_words,
        timestamp_error: validation.as_ref().err().cloned(),
        truncated,
    }
}

pub(super) fn bounded_text(text: &str, max_bytes: usize) -> (&str, bool) {
    if text.len() <= max_bytes {
        return (text, false);
    }
    let mut end = max_bytes;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    (&text[..end], true)
}

pub(super) fn native_trace_json(trace: &NativeHypothesisTrace) -> serde_json::Value {
    serde_json::json!({
        "text": &trace.text,
        "words": trace.words.iter().map(|word| serde_json::json!({
            "text": &word.text,
            "native_start_ms": word.start_ms,
            "native_end_ms": word.end_ms,
            "raw_global_start_sample": word.raw_global_start_sample,
            "raw_global_end_sample": word.raw_global_end_sample,
            "effective_global_start_sample": word.effective_global_start_sample,
            "effective_global_end_sample": word.effective_global_end_sample,
            "native_span_shape_valid": word.native_span_shape_valid,
            "accepted_by_validation": word.accepted_by_validation,
            "ordered_in_hypothesis": word.ordered_in_hypothesis,
        })).collect::<Vec<_>>(),
        "timestamp_error": trace.timestamp_error.as_deref(),
        "truncated": trace.truncated,
    })
}

pub(super) fn range_trace_json(range: &SourceRange) -> serde_json::Value {
    serde_json::json!({
        "start_sample": range.start_sample,
        "end_sample": range.end_sample,
        "sample_rate_hz": range.sample_rate_hz,
    })
}

pub(super) fn block_trace_json(block: &confirmation::ConfirmedBlock) -> serde_json::Value {
    serde_json::json!({
        "segment_id": block.segment_id,
        "text": bounded_text(&block.text, TRACE_MAX_TEXT_BYTES).0,
        "range": range_trace_json(&block.range),
    })
}

pub(super) fn refusal_trace_json(refusal: &confirmation::ConfirmationRefusal) -> serde_json::Value {
    serde_json::json!({
        "code": refusal.code,
        "range": refusal.range.as_ref().map(range_trace_json),
        "gaps": refusal.gaps.iter().map(range_trace_json).collect::<Vec<_>>(),
        "first_alignment_candidate": refusal.alignment_candidate.as_ref().map(|candidate| serde_json::json!({
            "window_sequence": candidate.sequence,
            "range": range_trace_json(&candidate.range),
            "start_delta_samples": candidate.start_delta_samples,
            "end_delta_samples": candidate.end_delta_samples,
            "overlap_samples": candidate.overlap_samples,
            "within_time_tolerance": candidate.within_time_tolerance,
            "same_span": candidate.same_span,
        })),
    })
}

pub(super) fn confirmation_error_code(message: &str) -> &'static str {
    if message.contains("Mot Parakeet sans horodatage natif valide") {
        "invalid_native_timestamp"
    } else if message.contains("hors session ou chronologie") {
        "continuity_break"
    } else {
        "confirmation_input_rejected"
    }
}

pub(super) fn window_error_code(message: &str) -> &'static str {
    if message.contains("Horodatage") || message.contains("Horodatages") {
        "invalid_native_timestamp"
    } else if message.contains("sans mots horodatés") {
        "missing_word_timestamps"
    } else {
        "native_window_unusable"
    }
}
