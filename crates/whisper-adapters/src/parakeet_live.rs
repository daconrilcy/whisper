//! Capture-first Parakeet microphone path.
//!
//! Audio is written to the session WAV before it is considered for inference.
//! Only one window is in flight; the WAV itself is the bounded-memory backlog.

#[path = "parakeet_confirmation.rs"]
mod confirmation;

use confirmation::{Confirmation, Hypothesis, TimedWord};

use crate::microphone_probe::{MicrophoneProbe, MicrophoneSpectrumReader};
use crate::{
    capture::{CaptureStream, Pcm16Converter},
    direct_live::WavOutput,
    parakeet_audio::{CapturePcmWriter, CaptureWriteAck},
    parakeet_import::{ParakeetProcess, ParakeetWorkerPaths},
};
use std::{
    env,
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    thread::JoinHandle,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use whisper_core::{
    ComputeChoice, Generation, ImportEvent, JobId, LiveRequest, SegmentId, SourceRange,
    parakeet_ipc::{Event as WorkerEvent, Word},
    parakeet_windows::{AbsoluteWordSamples, Window, absolute_word_samples},
};

const SAMPLE_RATE: u64 = 16_000;
const WINDOW_SAMPLES: u64 = SAMPLE_RATE * 4;
const WINDOW_STEP_SAMPLES: u64 = SAMPLE_RATE * 2;
const MAX_WINDOW_LAG_SAMPLES: u64 = WINDOW_SAMPLES * 2;
const MAX_CAPTURE_BLOCKS_PER_PUMP: usize = 32;
const FIRST_ATTEMPT: u64 = 1;
const FINALIZE_TIMEOUT: Duration = Duration::from_secs(30);
const TRACE_ENV: &str = "WHISPER_PARAKEET_CONFIRMATION_TRACE";
const TRACE_ROOT: &str = r"C:\dev\whisper\target\parakeet-tests\parakeet-micro-lot1";
const TRACE_QUEUE_CAPACITY: usize = 16;
const TRACE_MAX_RECORD_BYTES: usize = 64 * 1024;
const TRACE_MAX_FILE_BYTES: u64 = 2 * 1024 * 1024;
const TRACE_MAX_WORDS: usize = 128;
const TRACE_MAX_TEXT_BYTES: usize = 2 * 1024;
const TRACE_MAX_WORD_TEXT_BYTES: usize = 128;
const TRACE_TERMINAL_RESERVE: u64 = 1024;
const MAX_RECOVERY_ATTEMPTS: usize = 4;
const RECOVERY_MIN_CONTEXT_SAMPLES: u64 = SAMPLE_RATE / 2;

/// Capture-to-source bridge used by the unified path. It drains the existing
/// nonblocking device ring and sends converted PCM to an independent bounded
/// writer; inference never owns or blocks this writer thread.
pub(crate) struct UnifiedCapturePump {
    capture: Option<CaptureStream>,
    converter: Option<Pcm16Converter>,
    writer: Option<CapturePcmWriter>,
    source_identity: Option<String>,
    last_acknowledged_gain_revision: Option<u64>,
    gain_events: std::collections::VecDeque<AppliedCaptureGain>,
    acknowledgements_received: u64,
    closed: Option<Result<(u64, Vec<CaptureWriteAck>), String>>,
}

pub(crate) struct AppliedCaptureGain {
    pub(crate) target_identity: String,
    pub(crate) gain_db: i8,
    pub(crate) gain_revision: u64,
    pub(crate) first_frame: u64,
}

impl UnifiedCapturePump {
    pub(crate) fn from_capture(
        capture: CaptureStream,
        source_path: PathBuf,
    ) -> Result<Self, String> {
        Self::from_capture_inner(capture, source_path, None)
    }

    #[cfg(test)]
    pub(crate) fn from_capture_with_test_part_limit(
        capture: CaptureStream,
        source_path: PathBuf,
        part_limit_bytes: u64,
    ) -> Result<Self, String> {
        Self::from_capture_inner(capture, source_path, Some(part_limit_bytes))
    }

    fn from_capture_inner(
        capture: CaptureStream,
        source_path: PathBuf,
        test_part_limit: Option<u64>,
    ) -> Result<Self, String> {
        let converter = Pcm16Converter::new(capture.sample_rate_hz(), capture.channels())?;
        let source_identity = capture.source_identity().map(str::to_owned);
        let writer = if let Some(part_limit) = test_part_limit {
            #[cfg(test)]
            {
                CapturePcmWriter::start_with_test_part_limit(
                    source_path,
                    crate::capture::CAPTURE_SLOTS,
                    part_limit,
                )
            }
            #[cfg(not(test))]
            {
                let _ = part_limit;
                Err(crate::parakeet_audio::AudioStoreError::InvalidRange)
            }
        } else {
            CapturePcmWriter::start(source_path, crate::capture::CAPTURE_SLOTS)
        }
        .map_err(|error| format!("StorageUnavailable: {error}"))?;
        Ok(Self {
            capture: Some(capture),
            converter: Some(converter),
            writer: Some(writer),
            source_identity,
            last_acknowledged_gain_revision: None,
            gain_events: std::collections::VecDeque::new(),
            acknowledgements_received: 0,
            closed: None,
        })
    }

    pub(crate) fn pump(&mut self) -> Result<Vec<CaptureWriteAck>, String> {
        if self.closed.is_some() {
            return Err("Capture Parakeet arrêtée".into());
        }
        let capture = self.capture.as_mut().ok_or("Capture Parakeet arrêtée")?;
        if capture.is_saturated() {
            return Err("CaptureLost: microphone ring saturated".into());
        }
        if capture.failure() {
            return Err("CaptureLost: microphone stream failed".into());
        }
        let mut gain_updates = Vec::new();
        for _ in 0..MAX_CAPTURE_BLOCKS_PER_PUMP {
            let Some(block) = capture.try_next_block() else {
                break;
            };
            let block_gain = block.gain();
            let block_first_frame = block.first_frame();
            let samples = self
                .converter
                .as_mut()
                .ok_or("Capture converter already drained")?
                .push(&block)?;
            if !samples.is_empty() {
                self.writer
                    .as_mut()
                    .ok_or("Capture writer already closed")?
                    .try_append(&samples)
                    .map_err(|error| format!("CaptureLost: audio writer: {error:?}"))?;
            }
            gain_updates.push((block_gain, block_first_frame));
        }
        for (gain, first_frame) in gain_updates {
            self.record_gain_acknowledgement(gain, first_frame);
        }
        Ok(self.collect_available_acknowledgements()?)
    }

    pub(crate) fn set_gain(
        &self,
        target_identity: &str,
        expected_revision: u64,
        gain: whisper_core::microphone_level::GainDb,
    ) -> Result<whisper_core::microphone_level::GainRevision, String> {
        let capture = self.capture.as_ref().ok_or("Capture Parakeet arrêtée")?;
        if capture.source_identity() != Some(target_identity) {
            return Err("MicrophoneLevel: source active différente de la cible".into());
        }
        capture.set_gain(expected_revision, gain)
    }

    #[cfg(test)]
    pub(crate) fn enqueue_test_block(&mut self, samples: Vec<f32>) {
        self.capture
            .as_mut()
            .expect("test capture remains open")
            .enqueue_test_block(samples);
    }

    pub(crate) fn take_gain_events(&mut self) -> Vec<AppliedCaptureGain> {
        self.gain_events.drain(..).collect()
    }

    fn record_gain_acknowledgement(
        &mut self,
        gain: whisper_core::microphone_level::GainRevision,
        first_frame: u64,
    ) {
        if self
            .last_acknowledged_gain_revision
            .is_none_or(|revision| gain.revision > revision)
        {
            self.last_acknowledged_gain_revision = Some(gain.revision);
            if let Some(target_identity) = self.source_identity.clone() {
                self.gain_events.push_back(AppliedCaptureGain {
                    target_identity,
                    gain_db: gain.gain.value(),
                    gain_revision: gain.revision,
                    first_frame,
                });
            }
        }
    }

    pub(crate) fn stop(&mut self) -> Result<(u64, Vec<CaptureWriteAck>), String> {
        if let Some(result) = &self.closed {
            return result.clone();
        }
        let result = self.stop_inner();
        self.closed = Some(result.clone());
        result
    }

    fn stop_inner(&mut self) -> Result<(u64, Vec<CaptureWriteAck>), String> {
        if let Some(mut capture) = self.capture.take() {
            if capture.is_saturated() || capture.failure() {
                return Err("CaptureLost: microphone ring overflow or stream failure".into());
            }
            capture.stop();
            while let Some(block) = capture.try_next_block() {
                let block_gain = block.gain();
                let block_first_frame = block.first_frame();
                let samples = self
                    .converter
                    .as_mut()
                    .ok_or("Capture converter already drained")?
                    .push(&block)?;
                if !samples.is_empty() {
                    self.append_during_stop(&samples)?;
                }
                self.record_gain_acknowledgement(block_gain, block_first_frame);
            }
        }
        let tail = self
            .converter
            .take()
            .ok_or("Capture converter already drained")?
            .finish()?;
        if !tail.is_empty() {
            self.append_during_stop(&tail)?;
        }
        let expected_acknowledgements = self
            .writer
            .as_ref()
            .ok_or("Capture writer already closed")?
            .submitted_blocks();
        let acknowledgements = self.collect_acknowledgements_until(expected_acknowledgements)?;
        let total = self
            .writer
            .as_mut()
            .ok_or("Capture writer already closed")?
            .close()
            .map_err(|error| format!("StorageUnavailable: final source close: {error:?}"))?;
        Ok((total, acknowledgements))
    }

    fn collect_available_acknowledgements(&mut self) -> Result<Vec<CaptureWriteAck>, String> {
        let mut acknowledgements = Vec::new();
        while let Some(ack) = self
            .writer
            .as_ref()
            .ok_or("Capture writer already closed")?
            .try_acknowledgement()
            .map_err(|error| format!("StorageUnavailable: writer acknowledgement: {error:?}"))?
        {
            self.acknowledgements_received = self.acknowledgements_received.saturating_add(1);
            acknowledgements.push(ack);
        }
        Ok(acknowledgements)
    }

    fn collect_acknowledgements_until(
        &mut self,
        expected: u64,
    ) -> Result<Vec<CaptureWriteAck>, String> {
        let mut acknowledgements = Vec::new();
        while self.acknowledgements_received < expected {
            if let Some(ack) = self
                .writer
                .as_ref()
                .ok_or("Capture writer already closed")?
                .try_acknowledgement()
                .map_err(|error| format!("StorageUnavailable: writer acknowledgement: {error:?}"))?
            {
                self.acknowledgements_received = self.acknowledgements_received.saturating_add(1);
                acknowledgements.push(ack);
            } else {
                thread::yield_now();
            }
        }
        Ok(acknowledgements)
    }

    fn append_during_stop(&mut self, samples: &[i16]) -> Result<(), String> {
        loop {
            match self
                .writer
                .as_mut()
                .ok_or("Capture writer already closed")?
                .try_append(samples)
            {
                Ok(_) => return Ok(()),
                Err(crate::parakeet_audio::CaptureWriterError::Saturated) => {
                    self.collect_available_acknowledgements()?;
                    thread::yield_now();
                }
                Err(error) => return Err(format!("CaptureLost: final audio write: {error:?}")),
            }
        }
    }
}

impl crate::parakeet_pipeline::ReadablePcmSource for UnifiedCapturePump {
    fn read_window(&mut self, range: std::ops::Range<u64>) -> Result<Vec<i16>, String> {
        self.writer
            .as_ref()
            .ok_or("Capture writer already closed")?
            .read_written_window(range)
            .map_err(|error| format!("AudioSource: {error:?}"))
    }
}

#[derive(Debug)]
enum InferenceRequest {
    Transcribe {
        sequence: u64,
        start_sample: u64,
        samples: Vec<i16>,
        path: PathBuf,
        sha256: String,
        captured_at_unix_ms: u64,
        recovery_for: Option<SourceRange>,
    },
    Stop,
}

#[derive(Clone)]
struct D2WorkerDiagnostics {
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

#[cfg(not(test))]
impl Default for D2WorkerDiagnostics {
    fn default() -> Self {
        Self {}
    }
}

impl D2WorkerDiagnostics {
    #[cfg(test)]
    fn for_d2_destination(destination: &Path) -> Result<Self, String> {
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

    fn enabled(&self) -> bool {
        #[cfg(test)]
        {
            return self.path.is_some();
        }
        #[cfg(not(test))]
        false
    }

    fn record(&self, value: serde_json::Value) {
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

#[derive(Debug)]
enum InferenceEvent {
    Ready {
        backend: String,
    },
    Result {
        sequence: u64,
        start_sample: u64,
        end_sample: u64,
        text: String,
        words: Vec<TimedWord>,
        captured_at_unix_ms: u64,
        available_at_unix_ms: u64,
        native: Option<NativeHypothesisTrace>,
        recovery_for: Option<SourceRange>,
    },
    WindowUnusable {
        sequence: u64,
        start_sample: u64,
        end_sample: u64,
        message: String,
        native: Option<NativeHypothesisTrace>,
        recovery_for: Option<SourceRange>,
    },
    Unavailable(String),
    Stopped,
}

enum WindowTranscription {
    Usable {
        text: String,
        words: Vec<TimedWord>,
        native: Option<NativeHypothesisTrace>,
    },
    Unusable {
        message: String,
        native: Option<NativeHypothesisTrace>,
    },
}

#[derive(Debug)]
struct NativeWordTrace {
    text: String,
    start_ms: u32,
    end_ms: u32,
    raw_global_start_sample: u64,
    raw_global_end_sample: u64,
    effective_global_start_sample: Option<u64>,
    effective_global_end_sample: Option<u64>,
    native_span_shape_valid: bool,
    accepted_by_validation: bool,
    ordered_in_hypothesis: bool,
}

#[derive(Debug)]
struct NativeHypothesisTrace {
    text: String,
    words: Vec<NativeWordTrace>,
    timestamp_error: Option<String>,
    truncated: bool,
}

enum TraceCommand {
    Record(Vec<u8>),
    Finish,
}

struct TraceSink {
    sender: Option<SyncSender<TraceCommand>>,
    writer: Option<JoinHandle<()>>,
    dropped_records: Arc<AtomicU64>,
    truncated_records: Arc<AtomicU64>,
    writer_failed: Arc<AtomicBool>,
}

impl TraceSink {
    fn from_environment() -> Self {
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

    fn disabled() -> Self {
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

    fn enabled(&self) -> bool {
        self.sender.is_some()
    }

    fn record(&self, value: serde_json::Value) {
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

    fn finish(&mut self) {
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

#[derive(Default)]
struct RecoveryAttemptLedger {
    ranges: Vec<SourceRange>,
}

impl RecoveryAttemptLedger {
    fn len(&self) -> usize {
        self.ranges.len()
    }

    fn contains_overlap(&self, range: SourceRange) -> bool {
        self.ranges.iter().any(|attempted| {
            attempted.start_sample < range.end_sample && range.start_sample < attempted.end_sample
        })
    }

    fn record(&mut self, range: SourceRange) {
        if self.ranges.len() < MAX_RECOVERY_ATTEMPTS && !self.contains_overlap(range) {
            self.ranges.push(range);
        }
    }
}

pub(crate) struct ParakeetLive {
    request: LiveRequest,
    gpu_candidate: Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
    session_id: u64,
    inference_request: SyncSender<InferenceRequest>,
    inference_events: Receiver<InferenceEvent>,
    cancel_inference: Arc<AtomicBool>,
    capture: Option<CaptureStream>,
    probe: MicrophoneProbe,
    last_meter_sequence: u64,
    spectrum_reader: crate::microphone_probe::MicrophoneSpectrumReader,
    next_spectrum_telemetry: bool,
    telemetry_due_after_event: bool,
    last_acknowledged_gain_revision: Option<u64>,
    converter: Option<Pcm16Converter>,
    wav: Option<WavOutput>,
    window_root: PathBuf,
    next_window_start: u64,
    next_sequence: u64,
    last_reported_samples: u64,
    capture_origin_unix_ms: Option<u64>,
    inflight: bool,
    stopped: bool,
    unavailable: bool,
    ready_sent: bool,
    finalizing: bool,
    pending_events: std::collections::VecDeque<ImportEvent>,
    confirmation: Confirmation,
    recovery_attempts: RecoveryAttemptLedger,
    trace: TraceSink,
    #[cfg(test)]
    d2_worker_diagnostics: D2WorkerDiagnostics,
}

impl ParakeetLive {
    pub(crate) fn start(
        request: LiveRequest,
        paths: ParakeetWorkerPaths,
        expected_gpu: Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
        session_id: u64,
    ) -> Result<Self, String> {
        Self::start_with_capture(request, paths, expected_gpu, session_id, None)
    }

    #[cfg(test)]
    pub(crate) fn start_test_capture(
        request: LiveRequest,
        paths: ParakeetWorkerPaths,
        session_id: u64,
        capture: CaptureStream,
    ) -> Result<Self, String> {
        Self::start_with_capture(request, paths, None, session_id, Some(capture))
    }

    fn start_with_capture(
        request: LiveRequest,
        paths: ParakeetWorkerPaths,
        expected_gpu: Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
        session_id: u64,
        test_capture: Option<CaptureStream>,
    ) -> Result<Self, String> {
        #[cfg(test)]
        let d2_worker_diagnostics = if test_capture.is_some() {
            D2WorkerDiagnostics::for_d2_destination(Path::new(&request.destination))?
        } else {
            D2WorkerDiagnostics::default()
        };
        #[cfg(not(test))]
        let d2_worker_diagnostics = D2WorkerDiagnostics::default();
        if request.destination.trim().is_empty() || request.model_path.trim().is_empty() {
            return Err("Micro Parakeet : modèle ou dossier de sortie invalide".into());
        }
        paths.validate()?;
        if request.config.compute == ComputeChoice::Gpu && expected_gpu.is_none() {
            return Err("BackendMismatch: Parakeet GPU sans identité courante".into());
        }

        // Open the microphone and durable WAV before starting the potentially slow worker.
        let initial_gain = whisper_core::microphone_level::GainDb::new(request.microphone_gain_db)
            .map_err(|error| format!("MicrophoneLevel: {error}"))?;
        let mut capture = match test_capture {
            Some(capture) => capture,
            None => crate::capture::start_direct_with_gain(&request.microphone, initial_gain)?,
        };
        let converter = match Pcm16Converter::new(capture.sample_rate_hz(), capture.channels()) {
            Ok(converter) => converter,
            Err(error) => {
                capture.stop();
                return Err(error);
            }
        };
        let probe = match MicrophoneProbe::start(
            capture.sample_rate_hz(),
            capture.source_identity().map(str::to_owned),
            request.job_id,
            request.generation,
        ) {
            Ok(probe) => probe,
            Err(error) => {
                capture.stop();
                return Err(error);
            }
        };
        let wav = match WavOutput::create(&request) {
            Ok(wav) => wav,
            Err(error) => {
                capture.stop();
                return Err(error);
            }
        };
        let window_root = PathBuf::from(&request.destination).join(format!(
            ".parakeet-live-{:032x}-{}-{}",
            request.job_id.0,
            request.generation.get(),
            session_id
        ));
        fs::create_dir(&window_root)
            .map_err(|error| format!("Fenêtres Parakeet non créées : {error}"))?;
        let trace = TraceSink::from_environment();
        let trace_enabled = trace.enabled();

        let (inference_request, request_rx) = mpsc::sync_channel(1);
        let (event_tx, inference_events) = mpsc::sync_channel(2);
        let cancel_inference = Arc::new(AtomicBool::new(false));
        let worker_cancel = Arc::clone(&cancel_inference);
        let worker_paths = paths.clone();
        let worker_request = request.clone();
        let worker_log_root = window_root.clone();
        let worker_diagnostics = d2_worker_diagnostics.clone();
        let backend = match request.config.compute {
            ComputeChoice::Cpu => "CPU".to_owned(),
            ComputeChoice::Gpu => "GPU".to_owned(),
            ComputeChoice::Auto => "Auto".to_owned(),
        };
        let worker_gpu = expected_gpu.clone();
        let initial_event = ImportEvent::ParakeetLiveStarted {
            job_id: request.job_id,
            generation: request.generation,
            session_id,
            backend,
        };
        thread::Builder::new()
            .name(format!("parakeet-live-{session_id}"))
            .spawn(move || {
                inference_thread(
                    worker_paths,
                    worker_request,
                    worker_gpu,
                    worker_log_root,
                    worker_cancel,
                    request_rx,
                    event_tx,
                    trace_enabled,
                    worker_diagnostics,
                )
            })
            .map_err(|error| format!("Worker Parakeet non démarré : {error}"))?;

        Ok(Self {
            request,
            gpu_candidate: expected_gpu,
            session_id,
            inference_request,
            inference_events,
            cancel_inference,
            capture: Some(capture),
            probe,
            last_meter_sequence: 0,
            spectrum_reader: crate::microphone_probe::MicrophoneSpectrumReader::default(),
            next_spectrum_telemetry: false,
            telemetry_due_after_event: false,
            last_acknowledged_gain_revision: None,
            converter: Some(converter),
            wav: Some(wav),
            window_root,
            next_window_start: 0,
            next_sequence: 1,
            last_reported_samples: 0,
            capture_origin_unix_ms: None,
            inflight: false,
            stopped: false,
            unavailable: false,
            ready_sent: false,
            finalizing: false,
            pending_events: std::collections::VecDeque::from([initial_event]),
            confirmation: Confirmation::default(),
            recovery_attempts: RecoveryAttemptLedger::default(),
            trace,
            #[cfg(test)]
            d2_worker_diagnostics,
        })
    }

    pub(crate) fn identity(&self) -> (JobId, Generation) {
        (self.request.job_id, self.request.generation)
    }

    pub(crate) fn session_id(&self) -> u64 {
        self.session_id
    }

    pub(crate) fn destination(&self) -> &str {
        &self.request.destination
    }

    #[cfg(test)]
    pub(crate) fn enqueue_d2_capture_block(&mut self, samples: Vec<f32>) {
        self.capture
            .as_mut()
            .expect("D2 capture remains active")
            .enqueue_test_block(samples);
    }

    pub(crate) fn automatic_final_import(
        &self,
        wav_path: &str,
        job_id: JobId,
    ) -> Result<whisper_core::ports::AutomaticFinalImport, String> {
        let outputs = crate::direct_live::automatic_final_output_paths(wav_path)?;
        if Path::new(&outputs.txt_path).exists() || Path::new(&outputs.srt_path).exists() {
            return Err("Les fichiers de transcription automatique existent déjà".into());
        }
        Ok(whisper_core::ports::AutomaticFinalImport {
            request: whisper_core::ports::ImportRequest {
                strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
                job_id,
                generation: self.request.generation,
                source_path: wav_path.to_owned(),
                source_sha256: None,
                source_samples: None,
                model_path: self.request.model_path.clone(),
                model_sha256: self.request.model_sha256,
                destination: self.request.destination.clone(),
                config: self.request.config.clone(),
            },
            outputs,
            gpu_candidate: self.gpu_candidate.clone(),
        })
    }

    pub(crate) fn pump_capture(&mut self) -> Result<(), String> {
        if self.stopped {
            return Ok(());
        }
        let capture = self.capture.as_mut().expect("active capture");
        if capture.is_saturated() {
            return Err("Tampon microphone saturé ; capture incomplète".into());
        }
        if capture.failure() {
            return Err(capture
                .failure_detail()
                .unwrap_or_else(|| "Capture microphone interrompue".into()));
        }
        for _ in 0..MAX_CAPTURE_BLOCKS_PER_PUMP {
            let Some(block) = capture.try_next_block() else {
                break;
            };
            let block_gain = block.gain();
            let block_first_frame = block.first_frame();
            if self
                .last_acknowledged_gain_revision
                .is_none_or(|revision| block_gain.revision > revision)
            {
                self.last_acknowledged_gain_revision = Some(block_gain.revision);
                if let Some(target_identity) = capture.source_identity().map(str::to_owned) {
                    self.pending_events
                        .push_back(ImportEvent::MicrophoneGainApplied {
                            job_id: self.request.job_id,
                            generation: self.request.generation,
                            target_identity,
                            gain_db: block_gain.gain.value(),
                            gain_revision: block_gain.revision,
                            first_frame: block_first_frame,
                        });
                }
            }
            let converted = self
                .converter
                .as_mut()
                .expect("active converter")
                .push_with_probe(&block)?;
            capture.recycle_block(block)?;
            self.probe.submit(converted.probe);
            self.wav
                .as_mut()
                .expect("active WAV")
                .append(&converted.samples)?;
        }
        let samples = self.wav.as_ref().expect("active WAV").sample_count();
        if self.capture_origin_unix_ms.is_none() {
            self.capture_origin_unix_ms = capture.first_sample_at_unix_ms();
        }
        if samples > self.last_reported_samples {
            self.last_reported_samples = samples;
            self.queue_progress(ImportEvent::ParakeetCaptureProgress {
                job_id: self.request.job_id,
                generation: self.request.generation,
                session_id: self.session_id,
                captured_samples: samples,
                audio_durable_samples: samples,
            });
        }
        self.schedule_window(false)
    }

    pub(crate) fn set_gain(
        &self,
        target_identity: &str,
        expected_revision: u64,
        gain: whisper_core::microphone_level::GainDb,
    ) -> Result<whisper_core::microphone_level::GainRevision, String> {
        let capture = self.capture.as_ref().ok_or("Capture microphone arrêtée")?;
        if capture.source_identity() != Some(target_identity) {
            return Err("MicrophoneLevel: source active différente de la cible".into());
        }
        capture.set_gain(expected_revision, gain)
    }

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

    fn take_next_probe_telemetry(&mut self) -> Option<ImportEvent> {
        if self.next_spectrum_telemetry {
            self.next_spectrum_telemetry = false;
            self.take_probe_spectrum()
                .or_else(|| self.take_probe_meter())
        } else {
            self.next_spectrum_telemetry = true;
            self.take_probe_meter()
                .or_else(|| self.take_probe_spectrum())
        }
    }

    fn take_probe_meter(&mut self) -> Option<ImportEvent> {
        let snapshot = self.probe.latest_at(Instant::now())?;
        if snapshot.sequence <= self.last_meter_sequence {
            return None;
        }
        let meter = snapshot.meter()?;
        self.last_meter_sequence = snapshot.sequence;
        Some(ImportEvent::MicrophoneMeter(meter))
    }

    fn take_probe_spectrum(&mut self) -> Option<ImportEvent> {
        self.spectrum_reader
            .poll(&self.probe, Instant::now())
            .map(ImportEvent::MicrophoneSpectrum)
    }

    pub(crate) fn stop(&mut self) -> Result<ImportEvent, String> {
        if self.stopped {
            return Err("Session Parakeet déjà arrêtée".into());
        }
        self.finalizing = true;
        self.probe.invalidate();
        let mut failures = Vec::new();
        if let Some(mut capture) = self.capture.take() {
            capture.stop();
            if let Err(error) = self.drain_stopped_capture(&mut capture) {
                failures.push(format!("Drain microphone incomplet : {error}"));
            }
        }
        if let Some(mut converter) = self.converter.take() {
            match converter.finish() {
                Ok(tail) => {
                    if let Some(wav) = self.wav.as_mut() {
                        if let Err(error) = wav.append(&tail) {
                            failures.push(format!("Dernier bloc audio non écrit : {error}"));
                        }
                    }
                }
                Err(error) => failures.push(format!("Conversion audio incomplète : {error}")),
            }
        }
        let durable_end = self.wav.as_ref().map_or(0, WavOutput::sample_count);
        let deadline = Instant::now() + FINALIZE_TIMEOUT;
        while !self.unavailable && Instant::now() < deadline {
            if let Err(error) = self.schedule_window(true) {
                failures.push(format!("Dernière fenêtre non préparée : {error}"));
                break;
            }
            if let Err(error) = self.poll() {
                failures.push(format!("Finalisation Parakeet interrompue : {error}"));
                break;
            }
            let remaining = durable_end.saturating_sub(self.next_window_start);
            if self.ready_sent && !self.inflight && remaining < SAMPLE_RATE {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        if !self.unavailable && Instant::now() >= deadline {
            failures.push("Délai de finalisation Parakeet dépassé".into());
        }
        if failures.is_empty() && !self.unavailable {
            // One bounded reread per unresolved interval, at most four intervals.
            // The source is the already durable WAV, never an inferred text edit.
            for range in self.confirmation.recovery_ranges() {
                if Instant::now() >= deadline {
                    break;
                }
                if self.recovery_attempts.len() >= MAX_RECOVERY_ATTEMPTS {
                    break;
                }
                // Live and finalization rereads share one ledger. Overlapping
                // targets represent the same unresolved audio portion and may
                // not consume a second attempt.
                if self.recovery_attempts.contains_overlap(range) {
                    continue;
                }
                let local_start = range
                    .start_sample
                    .saturating_sub(self.request.group_offset_samples);
                let start = local_start.saturating_sub(SAMPLE_RATE);
                let count = durable_end.saturating_sub(start).min(WINDOW_SAMPLES);
                if count < SAMPLE_RATE {
                    continue;
                }
                match self.submit_window(start, count, false, Some(range)) {
                    Ok(true) => self.recovery_attempts.record(range),
                    Ok(false) => {
                        failures.push("Récupération Parakeet non admise".into());
                        break;
                    }
                    Err(error) => {
                        self.recovery_attempts.record(range);
                        failures.push(format!("Récupération Parakeet impossible : {error}"));
                        break;
                    }
                }
                while self.inflight && !self.unavailable && Instant::now() < deadline {
                    if let Err(error) = self.poll() {
                        failures.push(format!("Récupération Parakeet interrompue : {error}"));
                        break;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                if self.inflight || self.unavailable || !failures.is_empty() {
                    break;
                }
            }
        }
        if self.inflight && !failures.iter().any(|item| item.contains("Délai")) {
            failures.push("Délai de finalisation Parakeet dépassé".into());
        }
        let finalization = self.confirmation.finalize_observed(self.trace.enabled());
        if self.trace.enabled() {
            self.trace.record(serde_json::json!({
                "event": "confirmation_finalized",
                "job_id": format!("{:032x}", self.request.job_id.0),
                "generation": self.request.generation.get(),
                "session_id": self.session_id,
                "at_unix_ms": unix_time_ms(),
                "cursor_before_sample": finalization.cursor_before,
                "cursor_after_sample": finalization.cursor_after,
                "published_blocks": finalization.published.iter().map(block_trace_json).collect::<Vec<_>>(),
                "first_refusal": finalization.first_refusal.as_ref().map(refusal_trace_json),
                "unresolved_before": finalization.unresolved_before.iter().map(range_trace_json).collect::<Vec<_>>(),
                "unresolved_after": finalization.unresolved_after.iter().map(range_trace_json).collect::<Vec<_>>(),
                "publication_phase": "after_capture_stopped",
            }));
        }
        self.cancel_inference.store(true, Ordering::Release);
        let path = match self.wav.take() {
            Some(wav) => match wav.finish() {
                Ok(path) => path,
                Err(error) => {
                    failures.push(format!("Finalisation WAV impossible : {error}"));
                    String::new()
                }
            },
            None => {
                failures.push("WAV de capture indisponible à la clôture".into());
                String::new()
            }
        };
        let _ = self.inference_request.try_send(InferenceRequest::Stop);
        // The worker writes its stderr log beneath this session root. Keep that
        // diagnostic evidence; a non-empty root is not a capture-stop failure.
        self.stopped = true;
        let mut diagnostic = Vec::new();
        if self.unavailable {
            diagnostic.push("une ou plusieurs hypothèses n'ont pas été produites".to_owned());
        }
        diagnostic.extend(failures);
        let global_durable_end = self
            .request
            .group_offset_samples
            .saturating_add(durable_end);
        let mut complete =
            diagnostic.is_empty() && !self.confirmation.has_remainder(global_durable_end);
        if !complete && diagnostic.is_empty() {
            diagnostic.push("une portion audio demeure non confirmée".into());
        }
        let segments = self.confirmed_segments();
        let unresolved = self.confirmation.remaining_ranges(global_durable_end);
        let outputs = crate::direct_live::export_parakeet_outputs(
            &self.request,
            &path,
            durable_end,
            &segments,
            &unresolved,
            complete,
        );
        complete = outputs.complete;
        diagnostic.extend(outputs.diagnostics);
        if self.trace.enabled() {
            self.trace.record(serde_json::json!({
                "event": "capture_stopped_returned",
                "job_id": format!("{:032x}", self.request.job_id.0),
                "generation": self.request.generation.get(),
                "session_id": self.session_id,
                "at_unix_ms": unix_time_ms(),
                "complete": complete && diagnostic.is_empty(),
                "confirmed_segment_count": segments.len(),
                "unresolved_ranges": self.confirmation.remaining_ranges(global_durable_end).iter().map(range_trace_json).collect::<Vec<_>>(),
                "publication_phase": "after_capture_stopped",
            }));
            self.trace.finish();
        }
        Ok(ImportEvent::ParakeetCaptureStopped {
            job_id: self.request.job_id,
            generation: self.request.generation,
            session_id: self.session_id,
            wav_path: path,
            txt_path: outputs.txt_path,
            srt_path: outputs.srt_path,
            complete: complete && diagnostic.is_empty(),
            final_segments: segments,
            unresolved,
            diagnostic: (!diagnostic.is_empty()).then(|| diagnostic.join(" ; ")),
            automatic_final: None,
        })
    }

    fn drain_stopped_capture(&mut self, capture: &mut CaptureStream) -> Result<(), String> {
        for _ in 0..crate::capture::CAPTURE_SLOTS {
            let Some(block) = capture.try_next_block() else {
                break;
            };
            let samples = self
                .converter
                .as_mut()
                .expect("converter retained until tail")
                .push(&block)?;
            capture.recycle_block(block)?;
            self.wav.as_mut().expect("active WAV").append(&samples)?;
        }
        Ok(())
    }

    pub(crate) fn stop_after_capture_error(&mut self, error: String) -> ImportEvent {
        self.finalizing = true;
        self.stopped = true;
        self.probe.invalidate();
        self.cancel_inference.store(true, Ordering::Release);
        if let Some(mut capture) = self.capture.take() {
            capture.stop();
            let _ = self.drain_stopped_capture(&mut capture);
        }
        if let Some(mut converter) = self.converter.take()
            && let Ok(tail) = converter.finish()
            && let Some(wav) = self.wav.as_mut()
        {
            let _ = wav.append(&tail);
        }
        let durable_end = self.wav.as_ref().map_or(0, WavOutput::sample_count);
        let wav_path = self.wav.take().and_then(|wav| wav.finish().ok());
        let _ = self.inference_request.try_send(InferenceRequest::Stop);
        self.unavailable = true;
        let segments = self.confirmed_segments();
        let unresolved = self.confirmation.remaining_ranges(
            self.request
                .group_offset_samples
                .saturating_add(durable_end),
        );
        let wav_path = wav_path.unwrap_or_default();
        let outputs = crate::direct_live::export_parakeet_outputs(
            &self.request,
            &wav_path,
            durable_end,
            &segments,
            &unresolved,
            false,
        );
        if self.trace.enabled() {
            self.trace.record(serde_json::json!({
                "event": "capture_stopped_after_error",
                "job_id": format!("{:032x}", self.request.job_id.0),
                "generation": self.request.generation.get(),
                "session_id": self.session_id,
                "at_unix_ms": unix_time_ms(),
                "error": bounded_text(&error, TRACE_MAX_TEXT_BYTES).0,
                "confirmed_segment_count": segments.len(),
                "unresolved_ranges": self.confirmation.remaining_ranges(
                    self.request.group_offset_samples.saturating_add(durable_end),
                ).iter().map(range_trace_json).collect::<Vec<_>>(),
                "publication_phase": "after_capture_error",
            }));
            self.trace.finish();
        }
        ImportEvent::ParakeetCaptureStopped {
            job_id: self.request.job_id,
            generation: self.request.generation,
            session_id: self.session_id,
            wav_path,
            txt_path: outputs.txt_path,
            srt_path: outputs.srt_path,
            complete: false,
            final_segments: segments,
            unresolved,
            diagnostic: Some(
                std::iter::once(format!(
                    "{error}; l'audio déjà écrit dans le WAV est conservé"
                ))
                .chain(outputs.diagnostics)
                .collect::<Vec<_>>()
                .join(" ; "),
            ),
            automatic_final: None,
        }
    }

    fn confirmed_segments(&self) -> Vec<whisper_core::ports::WorkerSegment> {
        self.confirmation
            .confirmed()
            .iter()
            .map(|block| whisper_core::ports::WorkerSegment {
                job_id: self.request.job_id,
                generation: self.request.generation,
                instance_id: self.session_id,
                segment_id: SegmentId(block.segment_id),
                range: block.range,
                text: block.text.clone(),
            })
            .collect()
    }

    fn schedule_window(&mut self, final_window: bool) -> Result<(), String> {
        if self.inflight || self.unavailable || !self.ready_sent {
            return Ok(());
        }
        let total = self.wav.as_ref().map_or(0, WavOutput::sample_count);
        if !final_window && !self.finalizing {
            let next_global_start = self
                .request
                .group_offset_samples
                .saturating_add(self.next_window_start);
            if self.recovery_attempts.len() < MAX_RECOVERY_ATTEMPTS {
                if let Some(target) = self
                    .confirmation
                    .recovery_candidates(next_global_start)
                    .into_iter()
                    .find(|range| !self.recovery_attempts.contains_overlap(*range))
                {
                    if let Some((start, count)) =
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

    fn submit_window(
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

    fn remove_completed_window(&self, sequence: u64) {
        let _ = fs::remove_file(self.window_root.join(format!("window-{sequence}.wav")));
    }

    fn queue_progress(&mut self, event: ImportEvent) {
        let session_id = self.session_id;
        self.pending_events.retain(|queued| {
            !matches!(queued, ImportEvent::ParakeetCaptureProgress { session_id: queued_session, .. } if *queued_session == session_id)
        });
        self.pending_events.push_back(event);
    }

    fn queue_audio_unusable(&mut self, sequence: Option<u64>, range: SourceRange, message: String) {
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

    fn publication_phase(&self) -> &'static str {
        if self.finalizing {
            "stop_finalization"
        } else {
            "capture_active"
        }
    }

    fn pop_pending_event(&mut self) -> Option<ImportEvent> {
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

fn native_hypothesis_trace(
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

fn bounded_text(text: &str, max_bytes: usize) -> (&str, bool) {
    if text.len() <= max_bytes {
        return (text, false);
    }
    let mut end = max_bytes;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    (&text[..end], true)
}

fn native_trace_json(trace: &NativeHypothesisTrace) -> serde_json::Value {
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

fn range_trace_json(range: &SourceRange) -> serde_json::Value {
    serde_json::json!({
        "start_sample": range.start_sample,
        "end_sample": range.end_sample,
        "sample_rate_hz": range.sample_rate_hz,
    })
}

fn block_trace_json(block: &confirmation::ConfirmedBlock) -> serde_json::Value {
    serde_json::json!({
        "segment_id": block.segment_id,
        "text": bounded_text(&block.text, TRACE_MAX_TEXT_BYTES).0,
        "range": range_trace_json(&block.range),
    })
}

fn refusal_trace_json(refusal: &confirmation::ConfirmationRefusal) -> serde_json::Value {
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

fn confirmation_error_code(message: &str) -> &'static str {
    if message.contains("Mot Parakeet sans horodatage natif valide") {
        "invalid_native_timestamp"
    } else if message.contains("hors session ou chronologie") {
        "continuity_break"
    } else {
        "confirmation_input_rejected"
    }
}

fn window_error_code(message: &str) -> &'static str {
    if message.contains("Horodatage") || message.contains("Horodatages") {
        "invalid_native_timestamp"
    } else if message.contains("sans mots horodatés") {
        "missing_word_timestamps"
    } else {
        "native_window_unusable"
    }
}

fn recovery_window(
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

impl Drop for ParakeetLive {
    fn drop(&mut self) {
        let _ = self.inference_request.try_send(InferenceRequest::Stop);
        // The bounded worker channel is closed when this session is dropped. Keep the
        // WAV and any unfinished window as recovery evidence; never delete user audio.
    }
}

fn expected_live_worker_backend(compute: ComputeChoice) -> &'static str {
    match compute {
        ComputeChoice::Cpu | ComputeChoice::Auto => "CPU",
        ComputeChoice::Gpu => "CUDA0",
    }
}

fn inference_thread(
    paths: ParakeetWorkerPaths,
    request: LiveRequest,
    candidate: Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
    session_log_root: PathBuf,
    cancel: Arc<AtomicBool>,
    requests: Receiver<InferenceRequest>,
    events: SyncSender<InferenceEvent>,
    trace_enabled: bool,
    diagnostics: D2WorkerDiagnostics,
) {
    diagnostics.record(serde_json::json!({
        "event":"worker-configured",
        "executable":paths.executable.to_string_lossy().into_owned(),
        "model":paths.model.to_string_lossy().into_owned(),
        "native_dir":paths.native_dir.to_string_lossy().into_owned(),
        "runtime_root":paths.runtime_root.to_string_lossy().into_owned(),
        "compute":request.config.compute,
        "model_request_path":request.model_path.clone(),
        "session_log_root":session_log_root.to_string_lossy().into_owned(),
        "at_unix_ms":unix_time_ms(),
    }));
    let expected_gpu = match (request.config.compute, candidate.as_ref()) {
        (ComputeChoice::Gpu, Some(candidate)) => {
            match crate::parakeet_import::expected_gpu_for_candidate(candidate) {
                Ok(expected) => Some(expected),
                Err(error) => {
                    let _ = events.send(InferenceEvent::Unavailable(error));
                    return;
                }
            }
        }
        (ComputeChoice::Gpu, None) => {
            let _ = events.send(InferenceEvent::Unavailable(
                "Identité du GPU absente".into(),
            ));
            return;
        }
        _ => None,
    };
    let mut process = match ParakeetProcess::spawn_live(&paths, &session_log_root) {
        Ok(process) => {
            diagnostics.record(serde_json::json!({
                "event":"worker-spawned",
                "executable":paths.executable.to_string_lossy().into_owned(),
                "pid":process.pid(),
                "at_unix_ms":unix_time_ms(),
            }));
            process
        }
        Err(error) => {
            diagnostics.record(serde_json::json!({
                "event":"worker-spawn-failed",
                "executable":paths.executable.to_string_lossy().into_owned(),
                "error":error.clone(),
                "at_unix_ms":unix_time_ms(),
            }));
            let _ = events.send(InferenceEvent::Unavailable(error));
            return;
        }
    };
    let (identity, native_call_marker_required) = loop {
        if cancel.load(Ordering::Acquire) {
            let exit = process.terminate();
            diagnostics.record(serde_json::json!({
                "event":"worker-terminated-before-ready",
                "pid":process.pid(),
                "exit_status":exit.as_ref().map(|status| status.to_string()).map_err(ToString::to_string),
                "at_unix_ms":unix_time_ms(),
            }));
            let _ = events.send(InferenceEvent::Stopped);
            return;
        }
        match process.poll() {
            Ok(Some(WorkerEvent::Ready {
                identity,
                backend,
                gpu_evidence,
                ..
            })) => {
                diagnostics.record(serde_json::json!({
                    "event":"worker-ready",
                    "pid":process.pid(),
                    "backend":backend.clone(),
                    "worker_sha256":identity.build_sha256.clone(),
                    "profile":identity.profile.clone(),
                    "at_unix_ms":unix_time_ms(),
                }));
                if backend != expected_live_worker_backend(request.config.compute) {
                    let _ = events.send(InferenceEvent::Unavailable(format!(
                        "BackendMismatch: expected {}, got {backend}",
                        expected_live_worker_backend(request.config.compute)
                    )));
                    let _ = process.terminate();
                    return;
                }
                if request.config.compute == ComputeChoice::Gpu {
                    let Some(expected) = expected_gpu.as_ref() else {
                        let _ =
                            events.send(InferenceEvent::Unavailable("Identité GPU absente".into()));
                        let _ = process.terminate();
                        return;
                    };
                    if backend != "CUDA0"
                        || gpu_evidence.as_ref().is_none_or(|evidence| {
                            !crate::worker_ipc::gpu_evidence_matches(
                                Some(evidence),
                                Some(expected),
                                process.pid(),
                            )
                        })
                    {
                        let _ = events.send(InferenceEvent::Unavailable(
                            "Le worker Parakeet n'a pas attesté le GPU qualifié".into(),
                        ));
                        let _ = process.terminate();
                        return;
                    }
                }
                if let Err(error) = process.accept_ready(identity.clone()) {
                    let _ = events.send(InferenceEvent::Unavailable(error));
                    let _ = process.terminate();
                    return;
                }
                let native_call_marker_required = backend == "CUDA0";
                let _ = events.send(InferenceEvent::Ready { backend });
                break (identity, native_call_marker_required);
            }
            Ok(Some(WorkerEvent::Failed { message, .. })) => {
                diagnostics.record(serde_json::json!({
                    "event":"worker-failed-before-ready",
                    "pid":process.pid(),
                    "message":message.clone(),
                    "at_unix_ms":unix_time_ms(),
                }));
                let _ = events.send(InferenceEvent::Unavailable(message));
                let exit = process.terminate();
                diagnostics.record(serde_json::json!({
                    "event":"worker-terminated-after-ready-failure",
                    "pid":process.pid(),
                    "exit_status":exit.as_ref().map(|status| status.to_string()).map_err(ToString::to_string),
                    "at_unix_ms":unix_time_ms(),
                }));
                return;
            }
            Ok(Some(_)) | Ok(None) => thread::sleep(Duration::from_millis(5)),
            Err(error) => {
                diagnostics.record(serde_json::json!({
                    "event":"worker-poll-error-before-ready",
                    "pid":process.pid(),
                    "error":error.clone(),
                    "exit_status":process.try_wait().ok().flatten().map(|status| status.to_string()),
                    "at_unix_ms":unix_time_ms(),
                }));
                let _ = events.send(InferenceEvent::Unavailable(error));
                let exit = process.terminate();
                diagnostics.record(serde_json::json!({
                    "event":"worker-terminated-after-poll-error",
                    "pid":process.pid(),
                    "exit_status":exit.as_ref().map(|status| status.to_string()).map_err(ToString::to_string),
                    "at_unix_ms":unix_time_ms(),
                }));
                return;
            }
        }
        if requests
            .try_recv()
            .is_ok_and(|message| matches!(message, InferenceRequest::Stop))
        {
            let exit = process.terminate();
            diagnostics.record(serde_json::json!({
                "event":"worker-terminated-on-stop-before-ready",
                "pid":process.pid(),
                "exit_status":exit.as_ref().map(|status| status.to_string()).map_err(ToString::to_string),
                "at_unix_ms":unix_time_ms(),
            }));
            let _ = events.send(InferenceEvent::Stopped);
            return;
        }
    };
    let mut attempt = FIRST_ATTEMPT;
    loop {
        match requests.recv_timeout(Duration::from_millis(10)) {
            Ok(InferenceRequest::Stop) => {
                let shutdown = process.shutdown();
                diagnostics.record(serde_json::json!({
                    "event":"worker-shutdown-requested",
                    "pid":process.pid(),
                    "result":shutdown.as_ref().map(|_| "sent").map_err(ToString::to_string),
                    "at_unix_ms":unix_time_ms(),
                }));
                if diagnostics.enabled() {
                    let deadline = Instant::now() + Duration::from_secs(2);
                    loop {
                        match process.try_wait() {
                            Ok(Some(status)) => {
                                diagnostics.record(serde_json::json!({
                                    "event":"worker-exited-after-shutdown",
                                    "pid":process.pid(),
                                    "exit_status":status.to_string(),
                                    "at_unix_ms":unix_time_ms(),
                                }));
                                break;
                            }
                            Ok(None) if Instant::now() < deadline => {
                                thread::sleep(Duration::from_millis(10));
                            }
                            Ok(None) => {
                                diagnostics.record(serde_json::json!({
                                    "event":"worker-exit-status-pending-after-shutdown",
                                    "pid":process.pid(),
                                    "at_unix_ms":unix_time_ms(),
                                }));
                                break;
                            }
                            Err(error) => {
                                diagnostics.record(serde_json::json!({
                                    "event":"worker-exit-status-error",
                                    "pid":process.pid(),
                                    "error":error,
                                    "at_unix_ms":unix_time_ms(),
                                }));
                                break;
                            }
                        }
                    }
                }
                let _ = events.send(InferenceEvent::Stopped);
                return;
            }
            Ok(InferenceRequest::Transcribe {
                sequence,
                start_sample,
                samples,
                path,
                sha256,
                captured_at_unix_ms,
                recovery_for,
            }) => {
                let result = transcribe_window(
                    &mut process,
                    &request,
                    &identity,
                    native_call_marker_required,
                    expected_gpu.clone(),
                    attempt,
                    &cancel,
                    &path,
                    sha256,
                    samples.len() as u64,
                    request.group_offset_samples.saturating_add(start_sample),
                    trace_enabled,
                );
                attempt = attempt.saturating_add(1);
                let _ = fs::remove_file(&path);
                match result {
                    Ok(WindowTranscription::Usable {
                        text,
                        words,
                        native,
                    }) => {
                        diagnostics.record(serde_json::json!({
                            "event":"worker-window-result",
                            "pid":process.pid(),
                            "sequence":sequence,
                            "text_bytes":text.len(),
                            "word_count":words.len(),
                            "at_unix_ms":unix_time_ms(),
                        }));
                        let end_sample = start_sample.saturating_add(samples.len() as u64);
                        if events
                            .send(InferenceEvent::Result {
                                sequence,
                                start_sample,
                                end_sample,
                                text,
                                words,
                                captured_at_unix_ms,
                                available_at_unix_ms: unix_time_ms(),
                                native,
                                recovery_for,
                            })
                            .is_err()
                        {
                            let _ = process.terminate();
                            return;
                        }
                    }
                    Ok(WindowTranscription::Unusable { message, native }) => {
                        diagnostics.record(serde_json::json!({
                            "event":"worker-window-unusable",
                            "pid":process.pid(),
                            "sequence":sequence,
                            "message":message.clone(),
                            "at_unix_ms":unix_time_ms(),
                        }));
                        let end_sample = start_sample.saturating_add(samples.len() as u64);
                        if events
                            .send(InferenceEvent::WindowUnusable {
                                sequence,
                                start_sample,
                                end_sample,
                                message,
                                native,
                                recovery_for,
                            })
                            .is_err()
                        {
                            let _ = process.terminate();
                            return;
                        }
                    }
                    Err(error) => {
                        diagnostics.record(serde_json::json!({
                            "event":"worker-transcribe-error",
                            "pid":process.pid(),
                            "sequence":sequence,
                            "error":error.clone(),
                            "exit_status":process.try_wait().ok().flatten().map(|status| status.to_string()),
                            "at_unix_ms":unix_time_ms(),
                        }));
                        let _ = events.send(InferenceEvent::Unavailable(error));
                        let exit = process.terminate();
                        diagnostics.record(serde_json::json!({
                            "event":"worker-terminated-after-transcribe-error",
                            "pid":process.pid(),
                            "exit_status":exit.as_ref().map(|status| status.to_string()).map_err(ToString::to_string),
                            "at_unix_ms":unix_time_ms(),
                        }));
                        return;
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if let Ok(Some(status)) = process.try_wait() {
                    diagnostics.record(serde_json::json!({
                        "event":"worker-exited-idle",
                        "pid":process.pid(),
                        "exit_status":status.to_string(),
                        "at_unix_ms":unix_time_ms(),
                    }));
                    let _ = events.send(InferenceEvent::Unavailable(
                        "Le worker Parakeet s'est arrêté".into(),
                    ));
                    return;
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let _ = process.terminate();
                return;
            }
        }
    }
}

fn transcribe_window(
    process: &mut ParakeetProcess,
    request: &LiveRequest,
    identity: &whisper_core::parakeet_ipc::WorkerIdentity,
    native_call_marker_required: bool,
    expected_gpu: Option<whisper_core::parakeet_ipc::ExpectedGpu>,
    attempt: u64,
    cancel: &AtomicBool,
    path: &Path,
    sha256: String,
    expected_samples: u64,
    global_window_start_sample: u64,
    trace_enabled: bool,
) -> Result<WindowTranscription, String> {
    let expected_job_id = format!("{:032x}", request.job_id.0);
    process.transcribe_with_gpu(
        expected_job_id.clone(),
        request.generation.get(),
        attempt,
        path,
        sha256.clone(),
        expected_gpu.clone(),
    )?;
    let mut native_call_started = false;
    loop {
        if cancel.load(Ordering::Acquire) {
            return Err("Capture arrêtée pendant l'inférence".into());
        }
        match process.poll()? {
            Some(WorkerEvent::NativeCallStarted {
                request_id,
                job_id,
                generation,
                attempt: actual_attempt,
                pid,
            }) if request_id == attempt
                && job_id == expected_job_id
                && generation == request.generation.get()
                && actual_attempt == attempt
                && pid == process.pid() =>
            {
                native_call_started = true;
            }
            Some(WorkerEvent::Result {
                request_id,
                job_id,
                generation,
                attempt: actual_attempt,
                identity: actual_identity,
                source_sha256,
                duration_samples,
                text,
                words,
                gpu_evidence,
                ..
            }) if window_result_matches(
                WindowResultIdentity {
                    request_id: attempt,
                    job_id: &expected_job_id,
                    generation: request.generation.get(),
                    attempt,
                    identity,
                    source_sha256: &sha256,
                    duration_samples: expected_samples,
                },
                WindowResultIdentity {
                    request_id,
                    job_id: &job_id,
                    generation,
                    attempt: actual_attempt,
                    identity: &actual_identity,
                    source_sha256: &source_sha256,
                    duration_samples,
                },
                native_call_marker_required,
                native_call_started,
            ) && (request.config.compute != ComputeChoice::Gpu
                || gpu_evidence.as_ref().is_some_and(|evidence| {
                    expected_gpu.as_ref().is_some_and(|expected| {
                        crate::worker_ipc::gpu_evidence_matches(
                            Some(evidence),
                            Some(expected),
                            process.pid(),
                        )
                    })
                })) =>
            {
                let validation = validate_words(&words, expected_samples);
                let native = trace_enabled.then(|| {
                    native_hypothesis_trace(&text, &words, global_window_start_sample, &validation)
                });
                return Ok(match validation {
                    Ok(ranges) if ranges.is_empty() && !text.trim().is_empty() => {
                        WindowTranscription::Unusable {
                            message: "Texte Parakeet sans mots horodatés".into(),
                            native,
                        }
                    }
                    Ok(ranges) => WindowTranscription::Usable {
                        text,
                        words: words
                            .into_iter()
                            .zip(ranges)
                            .map(|(word, range)| TimedWord {
                                text: word.text,
                                range: SourceRange {
                                    start_sample: range.start_sample,
                                    end_sample: range.end_sample,
                                    sample_rate_hz: SAMPLE_RATE as u32,
                                },
                            })
                            .collect(),
                        native,
                    },
                    Err(error) => WindowTranscription::Unusable {
                        message: error,
                        native,
                    },
                });
            }
            Some(WorkerEvent::Failed {
                request_id: Some(request_id),
                message,
                ..
            }) if request_id == attempt => return Err(message),
            Some(_) => return Err("Réponse Parakeet tardive ou étrangère à la fenêtre".into()),
            None => thread::sleep(Duration::from_millis(5)),
        }
    }
}

#[derive(Clone, Copy)]
struct WindowResultIdentity<'a> {
    request_id: u64,
    job_id: &'a str,
    generation: u64,
    attempt: u64,
    identity: &'a whisper_core::parakeet_ipc::WorkerIdentity,
    source_sha256: &'a str,
    duration_samples: u64,
}

fn window_result_matches(
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

fn validate_words(words: &[Word], window_samples: u64) -> Result<Vec<AbsoluteWordSamples>, String> {
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

fn write_pcm16_wav(path: &Path, samples: &[i16]) -> Result<(), String> {
    let data_len = u32::try_from(samples.len().checked_mul(2).ok_or("Fenêtre trop longue")?)
        .map_err(|_| "Fenêtre WAV trop longue")?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("WAV fenêtre non créé : {error}"))?;
    file.write_all(b"RIFF")
        .and_then(|()| file.write_all(&(36_u32.saturating_add(data_len)).to_le_bytes()))
        .and_then(|()| file.write_all(b"WAVEfmt "))
        .and_then(|()| file.write_all(&16_u32.to_le_bytes()))
        .and_then(|()| file.write_all(&1_u16.to_le_bytes()))
        .and_then(|()| file.write_all(&1_u16.to_le_bytes()))
        .and_then(|()| file.write_all(&(SAMPLE_RATE as u32).to_le_bytes()))
        .and_then(|()| file.write_all(&((SAMPLE_RATE as u32) * 2).to_le_bytes()))
        .and_then(|()| file.write_all(&2_u16.to_le_bytes()))
        .and_then(|()| file.write_all(&16_u16.to_le_bytes()))
        .and_then(|()| file.write_all(b"data"))
        .and_then(|()| file.write_all(&data_len.to_le_bytes()))
        .map_err(|error| format!("En-tête WAV fenêtre non écrit : {error}"))?;
    for chunk in samples.chunks(16_000) {
        let mut bytes = Vec::with_capacity(chunk.len() * 2);
        for sample in chunk {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        file.write_all(&bytes)
            .map_err(|error| format!("Audio fenêtre non écrit : {error}"))?;
    }
    file.flush()
        .map_err(|error| format!("WAV fenêtre non synchronisé : {error}"))
}

#[cfg(test)]
mod recovery_tests {
    use super::*;

    #[test]
    fn unified_capture_pump_converts_synthetic_input_and_closes_the_complete_source() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = env::temp_dir().join(format!("whisper-unified-capture-{stamp}"));
        fs::create_dir_all(&root).unwrap();
        let path = root.join("capture.wav");
        let mut capture =
            crate::capture::CaptureStream::empty_test_capture(48_000, 1, "test:synthetic");
        capture.enqueue_test_block(vec![0.25; 48_000]);
        let mut pump = UnifiedCapturePump::from_capture(capture, path.clone()).unwrap();
        let _ = pump.pump().unwrap();
        let (total_samples, acknowledgements) = pump.stop().unwrap();
        assert_eq!(pump.stop().unwrap().0, total_samples, "stop is idempotent");
        assert_eq!(total_samples, 16_000);
        assert!(!acknowledgements.is_empty());
        assert!(acknowledgements.iter().all(|ack| {
            ack.range.end <= total_samples
                && ack.written_samples == ack.flushed_samples
                && ack.synchronized_samples == 0
        }));
        let stored = crate::parakeet_audio::PcmPartStore::open(path)
            .read_range(0..total_samples)
            .unwrap();
        assert_eq!(stored.len(), total_samples as usize);
        assert!(stored.iter().any(|sample| *sample != 0));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn stop_recovery_skips_a_portion_already_attempted_during_capture() {
        let mut attempts = RecoveryAttemptLedger::default();
        let live_word = SourceRange::new(116_480, 119_040, SAMPLE_RATE as u32).unwrap();
        attempts.record(live_word);

        // Finalization may represent the same pending word as a wider gap.
        let stop_gap = SourceRange::new(112_000, 120_000, SAMPLE_RATE as u32).unwrap();
        assert!(attempts.contains_overlap(stop_gap));
        assert_eq!(attempts.len(), 1);

        // A disjoint unresolved portion remains eligible for its own bounded attempt.
        let later_gap = SourceRange::new(140_000, 148_000, SAMPLE_RATE as u32).unwrap();
        assert!(!attempts.contains_overlap(later_gap));
        attempts.record(later_gap);
        assert_eq!(attempts.len(), 2);
    }

    #[test]
    fn recovery_window_keeps_context_for_the_observed_ah_word() {
        let word = SourceRange::new(116_480, 119_040, SAMPLE_RATE as u32).unwrap();
        let (start, count) = recovery_window(word, 0, 160_000).unwrap();
        assert_eq!((start, count), (85_760, 64_000));
        assert!(start <= word.start_sample);
        assert!(start + count >= word.end_sample);
        assert!(word.start_sample - start >= RECOVERY_MIN_CONTEXT_SAMPLES);
        assert!(start + count - word.end_sample >= RECOVERY_MIN_CONTEXT_SAMPLES);
    }

    #[test]
    fn recovery_window_refuses_a_portion_without_durable_context() {
        let word = SourceRange::new(116_480, 119_040, SAMPLE_RATE as u32).unwrap();
        assert_eq!(recovery_window(word, 0, 32_000), None);
        assert_eq!(
            recovery_window(
                SourceRange::new(1_000, 20_000, SAMPLE_RATE as u32).unwrap(),
                0,
                64_000,
            ),
            None
        );
    }
}

#[cfg(test)]
mod window_result_contract_tests {
    use super::*;

    fn worker_identity() -> whisper_core::parakeet_ipc::WorkerIdentity {
        whisper_core::parakeet_ipc::WorkerIdentity {
            model_sha256: "model-sha".into(),
            runtime_sha256: "runtime-sha".into(),
            build_sha256: "worker-sha".into(),
            profile: "import-single-pass".into(),
        }
    }

    fn result_identity<'a>(
        identity: &'a whisper_core::parakeet_ipc::WorkerIdentity,
    ) -> WindowResultIdentity<'a> {
        WindowResultIdentity {
            request_id: 7,
            job_id: "0000000000000000000000000000002a",
            generation: 3,
            attempt: 7,
            identity,
            source_sha256: "0123456789abcdef",
            duration_samples: 64_000,
        }
    }

    #[test]
    fn correlated_cpu_result_does_not_require_native_call_marker() {
        let identity = worker_identity();
        let result = result_identity(&identity);
        assert!(window_result_matches(result, result, false, false));
    }

    #[test]
    fn expected_gpu_result_requires_native_call_marker() {
        let identity = worker_identity();
        let result = result_identity(&identity);
        assert!(!window_result_matches(result, result, true, false));
        assert!(window_result_matches(result, result, true, true));
    }

    #[test]
    fn foreign_result_is_rejected_even_without_gpu_marker_requirement() {
        let identity = worker_identity();
        let expected = result_identity(&identity);
        let foreign = WindowResultIdentity {
            request_id: expected.request_id + 1,
            ..expected
        };
        assert!(!window_result_matches(expected, foreign, false, false));
    }

    #[test]
    fn configured_compute_selects_expected_ready_backend() {
        assert_eq!(expected_live_worker_backend(ComputeChoice::Cpu), "CPU");
        assert_eq!(expected_live_worker_backend(ComputeChoice::Auto), "CPU");
        assert_eq!(expected_live_worker_backend(ComputeChoice::Gpu), "CUDA0");
    }
}

#[cfg(test)]
mod microphone_spectrum_tests {
    use super::*;
    use std::collections::VecDeque;
    use std::{
        sync::{atomic::AtomicBool, mpsc},
        time::Duration,
    };
    use whisper_core::microphone_level::{MicrophoneSpectrumUpdate, SpectrumBin};

    #[test]
    fn parakeet_capture_pump_publishes_frequency_spectrum_and_preserves_wav() {
        let directory = std::env::temp_dir().join(format!(
            "parakeet-spectrum-{}-{}",
            std::process::id(),
            unix_time_ms()
        ));
        fs::create_dir_all(&directory).unwrap();
        let job_id = JobId(7201);
        let generation = Generation::first();
        let request = LiveRequest {
            job_id,
            generation,
            model_path: "unused-model".into(),
            model_sha256: [0; 32],
            destination: directory.to_string_lossy().into_owned(),
            group_offset_samples: 0,
            auto_stop_after_speech_samples: None,
            microphone: whisper_core::MicrophoneChoice::Device("test:mic".into()),
            microphone_gain_db: 0,
            config: whisper_core::JobConfig {
                language: whisper_core::LanguageChoice::Automatic,
                compute: ComputeChoice::Cpu,
            },
        };
        let mut capture = crate::capture::CaptureStream::empty_test_capture(16_000, 1, "test:mic");
        let probe =
            MicrophoneProbe::start(16_000, Some("test:mic".into()), job_id, generation).unwrap();
        let converter = Pcm16Converter::new(16_000, 1).unwrap();
        let wav = WavOutput::create(&request).unwrap();
        let (inference_request, _request_rx) = mpsc::sync_channel(1);
        let (_event_tx, inference_events) = mpsc::sync_channel(2);
        let window_root = directory.join("windows");
        fs::create_dir(&window_root).unwrap();
        let mut live = ParakeetLive {
            request,
            gpu_candidate: None,
            session_id: 1,
            inference_request,
            inference_events,
            cancel_inference: Arc::new(AtomicBool::new(false)),
            capture: Some(capture),
            probe,
            last_meter_sequence: 0,
            spectrum_reader: MicrophoneSpectrumReader::default(),
            next_spectrum_telemetry: false,
            telemetry_due_after_event: false,
            last_acknowledged_gain_revision: None,
            converter: Some(converter),
            wav: Some(wav),
            window_root,
            next_window_start: 0,
            next_sequence: 1,
            last_reported_samples: 0,
            capture_origin_unix_ms: None,
            inflight: false,
            stopped: false,
            unavailable: false,
            ready_sent: false,
            finalizing: false,
            pending_events: VecDeque::new(),
            confirmation: Confirmation::default(),
            recovery_attempts: RecoveryAttemptLedger::default(),
            trace: TraceSink::disabled(),
            d2_worker_diagnostics: D2WorkerDiagnostics::default(),
        };
        let input_blocks = (0..5)
            .map(|block| {
                (block * 960..(block + 1) * 960)
                    .map(|frame| 0.5 * (std::f32::consts::TAU * 64.0 * frame as f32 / 2048.0).sin())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        for samples in &input_blocks {
            live.capture
                .as_mut()
                .unwrap()
                .enqueue_test_block(samples.clone());
            live.pump_capture().unwrap();
            thread::sleep(Duration::from_millis(20));
        }
        let deadline = Instant::now() + Duration::from_secs(2);
        let ready = ImportEvent::ParakeetInferenceReady {
            job_id,
            generation,
            session_id: 1,
            backend: "CPU".into(),
        };
        live.pending_events.extend([
            ready.clone(),
            ImportEvent::ParakeetConfirmed {
                job_id,
                generation,
                session_id: 1,
                segment: whisper_core::ports::WorkerSegment {
                    job_id,
                    generation,
                    instance_id: 1,
                    segment_id: whisper_core::SegmentId(1),
                    range: whisper_core::SourceRange::new(0, 160, 16_000).unwrap(),
                    text: "continuité".into(),
                },
            },
            ready.clone(),
            ImportEvent::ParakeetCaptureStopped {
                job_id,
                generation,
                session_id: 1,
                wav_path: "test.wav".into(),
                txt_path: None,
                srt_path: None,
                complete: false,
                final_segments: Vec::new(),
                unresolved: Vec::new(),
                diagnostic: None,
                automatic_final: None,
            },
        ]);
        let mut ready_count = 0;
        let mut saw_confirmed = false;
        let mut saw_stopped = false;
        let mut saw_meter = false;
        let mut event = None;
        while Instant::now() < deadline
            && (ready_count < 2 || !saw_confirmed || !saw_stopped || !saw_meter || event.is_none())
        {
            if let Some(next) = live.poll().unwrap() {
                match next {
                    ImportEvent::ParakeetInferenceReady { .. } => ready_count += 1,
                    ImportEvent::ParakeetConfirmed { .. } => saw_confirmed = true,
                    ImportEvent::ParakeetCaptureStopped { .. } => saw_stopped = true,
                    ImportEvent::MicrophoneMeter(_) => saw_meter = true,
                    spectrum @ ImportEvent::MicrophoneSpectrum(
                        MicrophoneSpectrumUpdate::Available(_),
                    ) => event = Some(spectrum),
                    _ => {}
                }
            } else {
                thread::sleep(Duration::from_millis(5));
            }
        }
        assert_eq!(ready_count, 2, "Parakeet control events did not progress");
        assert!(saw_confirmed, "confirmed Parakeet text did not progress");
        assert!(saw_stopped, "Parakeet terminal event did not progress");
        assert!(saw_meter, "Parakeet RMS telemetry did not progress");
        let event = event.expect("Parakeet spectrum did not progress");
        let ImportEvent::MicrophoneSpectrum(MicrophoneSpectrumUpdate::Available(spectrum)) = event
        else {
            panic!("expected a current Parakeet spectrum")
        };
        assert_eq!(spectrum.source_identity, "test:mic");
        let peak = spectrum
            .pre_gain
            .iter()
            .enumerate()
            .filter_map(|(index, bin)| match bin {
                SpectrumBin::MilliDb(value) => Some((index, value)),
                SpectrumBin::Silence => None,
            })
            .max_by_key(|(_, value)| *value)
            .unwrap()
            .0;
        let hz = peak as f64 * f64::from(spectrum.sample_rate_hz) / 2048.0;
        assert!((hz - 500.0).abs() < 10.0, "expected 500 Hz, got {hz} Hz");
        let tail = live.converter.as_mut().unwrap().finish().unwrap();
        live.wav.as_mut().unwrap().append(&tail).unwrap();
        assert_eq!(live.wav.as_ref().unwrap().sample_count(), 4_800);
        let with_probe_path = live.wav.take().unwrap().finish().unwrap();
        let baseline_dir = directory.join("without-probe");
        fs::create_dir_all(&baseline_dir).unwrap();
        let baseline_request = LiveRequest {
            job_id: JobId(7202),
            generation,
            model_path: "unused-model".into(),
            model_sha256: [0; 32],
            destination: baseline_dir.to_string_lossy().into_owned(),
            group_offset_samples: 0,
            auto_stop_after_speech_samples: None,
            microphone: whisper_core::MicrophoneChoice::Device("test:mic".into()),
            microphone_gain_db: 0,
            config: whisper_core::JobConfig {
                language: whisper_core::LanguageChoice::Automatic,
                compute: ComputeChoice::Cpu,
            },
        };
        let mut baseline_capture = CaptureStream::empty_test_capture(16_000, 1, "test:mic");
        let mut baseline_converter = Pcm16Converter::new(16_000, 1).unwrap();
        let mut baseline_wav = crate::direct_live::WavOutput::create(&baseline_request).unwrap();
        for samples in &input_blocks {
            baseline_capture.enqueue_test_block(samples.clone());
            let block = baseline_capture.try_next_block().unwrap();
            baseline_wav
                .append(&baseline_converter.push(&block).unwrap())
                .unwrap();
            baseline_capture.recycle_block(block).unwrap();
        }
        baseline_wav
            .append(&baseline_converter.finish().unwrap())
            .unwrap();
        let without_probe_path = baseline_wav.finish().unwrap();
        assert_eq!(
            fs::read(with_probe_path).unwrap(),
            fs::read(without_probe_path).unwrap(),
            "Parakeet WAV with spectrum reader is byte-identical to the no-reader baseline"
        );
        drop(live);
        let _ = fs::remove_dir_all(directory);
    }
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            elapsed.as_millis().min(u128::from(u64::MAX)) as u64
        })
}
