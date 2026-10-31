//! Capture-first Parakeet microphone path.
//!
//! Audio is written to the session WAV before it is considered for inference.
//! Only one window is in flight; the WAV itself is the bounded-memory backlog.

#[path = "parakeet_confirmation.rs"]
mod confirmation;

use confirmation::{Confirmation, Hypothesis, TimedWord};

use crate::microphone_probe::MicrophoneProbe;
use crate::{
    capture::{CaptureStream, Pcm16Converter},
    direct_live::WavOutput,
    parakeet_audio::{CapturePcmWriter, CaptureWriteAck},
    parakeet_import::{ParakeetProcess, ParakeetWorkerPaths},
};
#[cfg(test)]
use std::env;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
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
const MAX_RECOVERY_ATTEMPTS: usize = 4;
const RECOVERY_MIN_CONTEXT_SAMPLES: u64 = SAMPLE_RATE / 2;

mod capture;
mod events;
mod inference;
mod lifecycle;
mod trace;
mod windows;

pub(crate) use capture::{AppliedCaptureGain, UnifiedCapturePump};
use inference::*;
use trace::*;
use windows::*;

#[cfg(test)]
mod diagnostics_absence_tests;
#[cfg(test)]
mod microphone_spectrum_tests;
#[cfg(test)]
mod recovery_tests;
#[cfg(test)]
mod window_result_contract_tests;
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
        recovery_for: Option<SourceRange>,
    },
    WindowUnusable {
        sequence: u64,
        start_sample: u64,
        end_sample: u64,
        message: String,
        recovery_for: Option<SourceRange>,
    },
    Unavailable(String),
    Stopped,
}

enum WindowTranscription {
    Usable { text: String, words: Vec<TimedWord> },
    Unusable { message: String },
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
    #[cfg(test)]
    d2_worker_diagnostics: D2WorkerDiagnostics,
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

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            elapsed.as_millis().min(u128::from(u64::MAX)) as u64
        })
}
