mod backends;
mod ipc;
mod publication;
mod runtime;
use runtime::ImportRuntime;
#[cfg(test)]
use runtime::MicrophoneTestCapture;
#[cfg(test)]
use runtime::{manual_gpu_failure_event, retry_native_gpu_startup_on_cpu};
#[cfg(test)]
use runtime::{resolve_reprocessing_targets, verified_present_publications};
mod service;
mod support;

pub(crate) use backends::gpu_evidence_matches;
use backends::{
    IdleWorker, NativeUnifiedRoute, RuntimeModelCatalog, WORKER_IDLE_RETENTION,
    archive_qualification_for_request, can_retry_native_auto_on_cpu,
    gpu_qualification_candidate_allowed, native_cpu_route, native_unified_route,
    request_is_parakeet, validate_current_queued_gpu_candidate, validate_queued_gpu_preflight,
};
pub use backends::{probe_cuda_worker, qualify_gpu_worker};

use publication::PublicationGate;
#[cfg(test)]
use publication::{GATE_ACTIVE, GATE_CANCELLED, GATE_COMMITTED};

pub use ipc::ChildWorkerTransport;
use ipc::{EnqueueError, MAX_FRAME_BYTES, serialize_frame};
#[cfg(test)]
use ipc::{
    WorkerCommandPoll, WorkerReaderMessage, read_events, read_frame, take_next_worker_command,
    try_enqueue_bounded, try_enqueue_worker_command, write_frame,
};
pub use service::AsyncImportIo;
#[cfg(test)]
use service::automatic_final::{
    NEXT_AUTO_EXPORT_STAGE, inspect_automatic_final_queue_with, publish_automatic_staged_pair,
};
use service::automatic_final::{
    attach_automatic_final_import, inspect_automatic_final_queue, publish_automatic_final_exports,
    set_automatic_final_manifest_status,
};
use service::{ServiceRuntime, import_effect_identity};
#[cfg(test)]
use service::{effect_event_reservation, service_loop};
#[cfg(feature = "l01-memory-qualification")]
use support::memory_trace;
use support::{NEXT_IMPORT_JOB_ID, NEXT_INSTANCE, decode_sha256, format_sha256};

use crate::{
    WorkerTransport,
    archive::{ArchiveStore, LiveArchive},
    capture::{CaptureStream, Pcm16Converter},
    diagnostics::{TimingContextGuard, record_stage_timing},
    staging::{PcmStaging, PcmWriter},
    supervisor::{
        DiagnosticCode, DiagnosticPhase, DiagnosticRecord, DiagnosticSink, ProgressPhase,
        ProgressSupervisor, WorkerPaths,
    },
    vad::VoiceActivity,
};
use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::Ordering,
        mpsc::{self, Receiver, SyncSender},
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
#[cfg(test)]
use std::{
    io::BufReader,
    process::Command,
    sync::{
        atomic::AtomicU64,
        mpsc::{TryRecvError, TrySendError},
    },
};
#[cfg(test)]
use whisper_core::ImportIoPort;
#[cfg(test)]
use whisper_core::ipc::IpcEnvelope;
use whisper_core::{
    ArchiveHistoryItem, ComputeChoice, DiagnosticNotice, Generation, ImportEffect, ImportEvent,
    JobId,
    ipc::{BackendKind, WorkerCommand, WorkerEvent},
    ports::{
        DecodeRequest, DecoderPort, DuplicateImportChoice, DuplicateImportPrompt, ImportRequest,
        LiveRequest, PortError, PublishedImport, SourceIdentity, TreatmentIdentity, WorkerSegment,
    },
};

const TDT_FRAME_OVERLAP_SAMPLES: u64 = 80 * 16;
const LIVE_CAPTURE_EVENTS_BEFORE_WORKER_POLL: u8 = 2;
const UNIFIED_CHILD_JOB_CAPACITY: usize = 2;

fn live_capture_poll_allowed(events_since_worker_poll: u8, worker_available: bool) -> bool {
    events_since_worker_poll < LIVE_CAPTURE_EVENTS_BEFORE_WORKER_POLL || !worker_available
}

enum PreflightResult {
    Duplicate(DuplicateImportPrompt),
    Queued {
        submitted: (JobId, Generation),
        admitted: (JobId, Generation),
        entries: Vec<whisper_core::ports::ImportQueueEntry>,
    },
}

fn collision_free_job_id(
    destination: &Path,
    preferred: JobId,
    generation: Generation,
    queue: &[whisper_core::ports::ImportQueueEntry],
) -> JobId {
    let in_use = |candidate: JobId| {
        crate::archive::ArchiveStore::job_identity_exists(destination, candidate, generation)
            || queue.iter().any(|entry| {
                entry.request.job_id == candidate && entry.request.generation == generation
            })
    };
    if !in_use(preferred) {
        return preferred;
    }
    loop {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(1, |time| time.as_nanos().min(u64::MAX as u128) as u64);
        let mut observed = NEXT_IMPORT_JOB_ID.load(Ordering::Relaxed);
        let serial = loop {
            let next = now.max(observed.saturating_add(1));
            match NEXT_IMPORT_JOB_ID.compare_exchange_weak(
                observed,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break next,
                Err(actual) => observed = actual,
            }
        };
        let candidate = JobId(((std::process::id() as u128) << 64) | serial as u128);
        if !in_use(candidate) {
            return candidate;
        }
    }
}

fn vad_word_overlap_within_one_tdt_frame(start_sample: u64, previous_end_sample: u64) -> bool {
    start_sample.saturating_add(TDT_FRAME_OVERLAP_SAMPLES) >= previous_end_sample
}

fn append_vad_parakeet_word_group(
    groups: &mut Vec<(u64, u64, Vec<String>)>,
    start_sample: u64,
    end_sample: u64,
    token: &str,
) -> Result<(), String> {
    if start_sample >= end_sample {
        return Err("InvalidTimestamp: Parakeet word range is empty".into());
    }
    let boundary = groups.last().is_some_and(|(_, _, tokens)| {
        tokens
            .last()
            .is_some_and(|last| last.ends_with('.') || last.ends_with('!') || last.ends_with('?'))
    });
    if groups.is_empty() || (boundary && start_sample >= groups.last().unwrap().1) {
        groups.push((start_sample, end_sample, vec![token.to_owned()]));
    } else if let Some((group_start, group_end, tokens)) = groups.last_mut() {
        *group_start = (*group_start).min(start_sample);
        *group_end = (*group_end).max(end_sample);
        tokens.push(token.to_owned());
    }
    Ok(())
}

fn validate_single_pass_parakeet_words(
    words: &[whisper_core::parakeet_ipc::Word],
    duration_samples: u64,
) -> Result<Vec<(u64, u64, String)>, String> {
    let mut validated = Vec::with_capacity(words.len());
    let mut previous_end = 0_u64;
    for (index, word) in words.iter().enumerate() {
        let start = u64::from(word.start_ms) * 16;
        let end = u64::from(word.end_ms) * 16;
        let text = word.text.trim();
        if text.is_empty() || start >= end || end > duration_samples {
            return Err(format!(
                "InvalidTimestamp: Parakeet word {index} is empty or outside the audio duration"
            ));
        }
        if index > 0 && start < previous_end {
            return Err(format!(
                "InvalidTimestamp: Parakeet word {index} overlaps or is unordered"
            ));
        }
        previous_end = end;
        validated.push((start, end, text.to_owned()));
    }
    Ok(validated)
}

fn append_single_pass_parakeet_word_group(
    groups: &mut Vec<(u64, u64, Vec<String>)>,
    start_sample: u64,
    end_sample: u64,
    token: &str,
) -> Result<(), String> {
    let boundary = groups.last().is_some_and(|(_, _, tokens)| {
        tokens
            .last()
            .is_some_and(|last| last.ends_with('.') || last.ends_with('!') || last.ends_with('?'))
    });
    if start_sample >= end_sample {
        return Err("InvalidTimestamp: sentence boundary leaves an empty word range".into());
    }
    if groups.is_empty() || boundary {
        groups.push((start_sample, end_sample, vec![token.to_owned()]));
    } else if let Some((_, group_end, tokens)) = groups.last_mut() {
        *group_end = end_sample;
        tokens.push(token.to_owned());
    }
    Ok(())
}
const EVENTS_PER_STAGE: usize = 21;

/// Deterministic process-level barriers used only by the acceptance-test build.
/// Their root is checked against the isolated L2a-2 runtime before touching disk.
#[cfg(feature = "recette-hooks")]
fn acceptance_barrier(phase: &str, destination: &Path) -> Result<(), String> {
    if option_env!("PARAKEET_L2_ACCEPTANCE_TEST_BUILD").is_none() {
        return Ok(());
    }
    if !matches!(
        phase,
        "prepare" | "resume-prefix-confirmed" | "publish-before-commit" | "publish-after-commit"
    ) {
        return Err("InvalidInput: unknown L2a-2 acceptance barrier".into());
    }
    let runtime = std::env::var_os("PARAKEET_L2_RUNTIME_ROOT").ok_or_else(|| {
        "MissingRuntime: L2a-2 root is required for acceptance barrier".to_owned()
    })?;
    let runtime = PathBuf::from(runtime)
        .canonicalize()
        .map_err(|error| format!("MissingRuntime: {error}"))?;
    let destination = destination
        .canonicalize()
        .map_err(|error| format!("PathOutsideRuntime: destination unavailable: {error}"))?;
    let root = destination.join(".l2a2-acceptance");
    if !root.join(format!("{phase}.enabled")).is_file() {
        return Ok(());
    }
    fs::create_dir_all(&root).map_err(|error| format!("StorageUnavailable: {error}"))?;
    let root = root
        .canonicalize()
        .map_err(|error| format!("StorageUnavailable: {error}"))?;
    if !destination.starts_with(&runtime) || !root.starts_with(&runtime) {
        return Err("PathOutsideRuntime: acceptance barrier is outside L2a-2".into());
    }
    fs::write(root.join(format!("{phase}.entered")), b"entered")
        .map_err(|error| format!("StorageUnavailable: {error}"))?;
    while !root.join(format!("{phase}.release")).is_file() {
        thread::sleep(Duration::from_millis(5));
    }
    Ok(())
}

#[cfg(not(feature = "recette-hooks"))]
fn acceptance_barrier(_phase: &str, _destination: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(feature = "recette-hooks")]
fn take_acceptance_worker_fault(
    destination: &Path,
    worker_ready: bool,
) -> Result<Option<&'static str>, String> {
    if option_env!("PARAKEET_L2_ACCEPTANCE_TEST_BUILD").is_none() {
        return Ok(None);
    }
    let runtime = std::env::var_os("PARAKEET_L2_RUNTIME_ROOT")
        .ok_or_else(|| "MissingRuntime: L2a-2 root is required for acceptance fault".to_owned())?;
    let runtime = PathBuf::from(runtime)
        .canonicalize()
        .map_err(|error| format!("MissingRuntime: {error}"))?;
    let destination = destination
        .canonicalize()
        .map_err(|error| format!("PathOutsideRuntime: destination unavailable: {error}"))?;
    let control = destination.join(".l2a2-acceptance");
    let extended_execution_root = canonical_extended_acceptance_root(
        std::env::var_os("PARAKEET_L3C_EXEC_ROOT").map(PathBuf::from),
        std::env::var_os("PARAKEET_L3D_EXEC_ROOT").map(PathBuf::from),
    );
    let in_primary_runtime = destination.starts_with(&runtime) && control.starts_with(&runtime);
    let in_extended_runtime = option_env!("PARAKEET_L2_ACCEPTANCE_TEST_BUILD").is_some()
        && extended_execution_root
            .as_ref()
            .is_some_and(|root| destination.starts_with(root) && control.starts_with(root));
    if !in_primary_runtime && !in_extended_runtime {
        return Err("PathOutsideRuntime: acceptance fault is outside L2a-2".into());
    }
    for (signal, message) in [
        (
            "worker-death",
            "WorkerExited: test-injected process termination after Transcribe",
        ),
        (
            "incomplete-result",
            "IpcDecode: incomplete Result frame at adapter transport boundary",
        ),
    ] {
        if !worker_ready {
            continue;
        }
        let enabled = control.join(format!("{signal}.enabled"));
        if enabled.is_file() {
            fs::remove_file(&enabled).map_err(|error| format!("StorageUnavailable: {error}"))?;
            fs::write(control.join(format!("{signal}.entered")), b"entered")
                .map_err(|error| format!("StorageUnavailable: {error}"))?;
            return Ok(Some(message));
        }
    }
    Ok(None)
}

#[cfg(feature = "recette-hooks")]
fn canonical_extended_acceptance_root(
    primary_root: Option<PathBuf>,
    secondary_root: Option<PathBuf>,
) -> Option<PathBuf> {
    let (configured, lot) = match (primary_root, secondary_root) {
        (Some(path), None) => (path, "l3c"),
        (None, Some(path)) => (path, "l3d"),
        _ => return None,
    };
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .ok()?;
    let expected = repository
        .join(format!("target/parakeet-tests/l3/{lot}"))
        .canonicalize()
        .ok()?;
    (configured.canonicalize().ok()? == expected).then_some(expected)
}

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            elapsed.as_millis().min(u128::from(u64::MAX)) as u64
        })
}

#[cfg(test)]
include!("worker_ipc/tests/stopped_ack.rs");

fn scan_archive_history(root: &Path) -> Vec<ArchiveHistoryItem> {
    let entries = match ArchiveStore::scan(root) {
        Ok(entries) => entries,
        Err(message) => {
            return vec![ArchiveHistoryItem {
                job_id: JobId(0),
                generation: Generation::first(),
                complete: false,
                source_sha256: String::new(),
                diagnostic: Some(format!("StorageCorrupt: {message}")),
                live_recovery: None,
            }];
        }
    };
    let recoveries = match crate::archive::scan_live_recoveries(root) {
        Ok(recoveries) => recoveries,
        Err(error) => {
            return vec![ArchiveHistoryItem {
                job_id: JobId(0),
                generation: Generation::first(),
                complete: false,
                source_sha256: String::new(),
                diagnostic: Some(error),
                live_recovery: None,
            }];
        }
    };
    entries
        .into_iter()
        .map(|item| ArchiveHistoryItem {
            job_id: item.job_id,
            generation: item.generation,
            complete: item.complete,
            source_sha256: item.source_sha256,
            diagnostic: item.diagnostic.or_else(|| {
                if item.publication_recovery {
                    Some(
                        "Recoverable: publication R11 complète, en attente d'activation explicite"
                            .into(),
                    )
                } else {
                    item.export_basename
                        .as_ref()
                        .map(|base| format!("Export: {base}"))
                }
            }),
            live_recovery: recoveries
                .iter()
                .find(|recovery| {
                    recovery.request.job_id == item.job_id
                        && recovery.request.generation == item.generation
                })
                .cloned(),
        })
        .collect()
}

fn take_ready_live_stop(
    pending_persistence: usize,
    deferred: &mut Option<WorkerEvent>,
) -> Option<WorkerEvent> {
    (pending_persistence == 0)
        .then(|| deferred.take())
        .flatten()
}

fn bind_unidentified_startup_failure(
    event: WorkerEvent,
    attempt: (JobId, Generation, u64),
    startup_ready: bool,
) -> WorkerEvent {
    if startup_ready {
        return event;
    }
    match event {
        WorkerEvent::Failed {
            job_id: None,
            generation: None,
            instance_id: None,
            code,
            message,
        } => WorkerEvent::Failed {
            job_id: Some(attempt.0),
            generation: Some(attempt.1),
            instance_id: Some(attempt.2),
            code,
            message,
        },
        event => event,
    }
}

fn worker_event_identity(event: &WorkerEvent) -> Option<(JobId, Generation, u64)> {
    match event {
        WorkerEvent::Ready {
            job_id,
            generation,
            instance_id,
            ..
        }
        | WorkerEvent::ImportStarted {
            job_id,
            generation,
            instance_id,
        }
        | WorkerEvent::LiveMp3Packet {
            job_id,
            generation,
            instance_id,
            ..
        }
        | WorkerEvent::WindowFinished {
            job_id,
            generation,
            instance_id,
            ..
        }
        | WorkerEvent::Stopped {
            job_id,
            generation,
            instance_id,
        }
        | WorkerEvent::Progress {
            job_id,
            generation,
            instance_id,
            ..
        }
        | WorkerEvent::End {
            job_id,
            generation,
            instance_id,
            ..
        } => Some((*job_id, *generation, *instance_id)),
        WorkerEvent::GpuSessionAttested {
            job_id,
            generation,
            instance_id,
            ..
        } => Some((*job_id, *generation, *instance_id)),
        WorkerEvent::Segment(segment) => {
            Some((segment.job_id, segment.generation, segment.instance_id))
        }
        WorkerEvent::Failed {
            job_id: Some(job_id),
            generation: Some(generation),
            instance_id: Some(instance_id),
            ..
        } => Some((*job_id, *generation, *instance_id)),
        WorkerEvent::Failed { .. }
        | WorkerEvent::DecodedBlock { .. }
        | WorkerEvent::CudaAvailable
        | WorkerEvent::GpuCandidateAttested { .. }
        | WorkerEvent::GpusQualified { .. } => None,
    }
}

fn worker_event_is_stale(event: &WorkerEvent, attempt: (JobId, Generation, u64)) -> bool {
    worker_event_identity(event).is_some_and(|identity| identity != attempt)
}

fn map_attempt_sequence(base: u64, replay_prefix: u64, attempt_sequence: u64) -> Option<u64> {
    attempt_sequence
        .checked_sub(replay_prefix)?
        .checked_sub(1)?
        .checked_add(base)
}

fn campaign_reserve_attempt(
    archive: &mut LiveArchive,
    job_id: JobId,
    generation: Generation,
) -> Result<u64, String> {
    crate::archive::campaign_gate("live.before_reserve", job_id.0, generation.get(), 0)?;
    let result = archive.reserve_attempt();
    crate::archive::record_campaign_operation_result(
        "live.before_reserve",
        job_id.0,
        generation.get(),
        0,
        &result,
        None,
    )?;
    let attempt = result?;
    crate::archive::campaign_gate("live.after_reserve", job_id.0, generation.get(), 0)?;
    Ok(attempt)
}

struct UnifiedDocumentDrain {
    document: crate::parakeet_document::ParakeetDocumentStore,
    job_id: JobId,
    generation: Generation,
    revision: u64,
    total_samples: u64,
    next_sequence: u64,
    ready_pending: bool,
    progress_pending: bool,
}

fn restore_unified_document_prefix(
    document: &mut crate::parakeet_document::ParakeetDocumentStore,
    checkpoint: &crate::archive::UnifiedArchiveResumeCheckpoint,
    job_id: JobId,
    generation: Generation,
    source_samples: u64,
) -> Result<(), String> {
    use whisper_core::ports::ParakeetDocumentPort;

    let saved = checkpoint.pipeline;
    if saved.confirmed_segments == 0 && saved.window_sequence == 0 && saved.covered_samples == 0 {
        return Ok(());
    }
    let mut reader = crate::parakeet_document::ParakeetDocumentStore::open_resume_coverage(
        &checkpoint.previous_document_directory,
        checkpoint.previous_job_id,
        checkpoint.previous_generation,
        source_samples,
    )?
    .ok_or_else(|| "StorageCorrupt: resume document checkpoint disappeared".to_owned())?;
    let mut next_segment = 1_u64;
    let mut last_coverage = None;
    loop {
        let page = reader.read_page(128)?;
        if page.is_empty() {
            break;
        }
        for coverage in page {
            if coverage.sequence > saved.window_sequence {
                break;
            }
            while next_segment <= coverage.last_segment_sequence {
                let entries = reader.read_document_page(next_segment, 128)?;
                if entries.is_empty() {
                    return Err("StorageCorrupt: resume document prefix ended early".into());
                }
                for entry in entries {
                    if entry.segment.segment_id.0 > coverage.last_segment_sequence
                        || entry.segment.segment_id.0 > saved.confirmed_segments
                    {
                        break;
                    }
                    let segment = WorkerSegment {
                        job_id,
                        generation,
                        instance_id: entry.segment.instance_id,
                        segment_id: entry.segment.segment_id,
                        range: entry.segment.range,
                        text: entry.segment.text,
                    };
                    document
                        .append_final(segment)
                        .map_err(|error| format!("Document: {error:?}"))?;
                    next_segment = next_segment.checked_add(1).ok_or_else(|| {
                        "StorageCorrupt: resume segment sequence exhausted".to_owned()
                    })?;
                }
            }
            document
                .record_coverage(coverage)
                .map_err(|error| format!("Document: {error:?}"))?;
            last_coverage = Some(coverage.sequence);
            if coverage.sequence == saved.window_sequence {
                break;
            }
        }
        if last_coverage == Some(saved.window_sequence) {
            break;
        }
    }
    if last_coverage != Some(saved.window_sequence)
        || next_segment != saved.confirmed_segments.saturating_add(1)
        || document.revision() != saved.confirmed_segments
        || saved.covered_samples == 0
    {
        return Err("StorageCorrupt: resume checkpoint prefix could not be restored".into());
    }
    Ok(())
}

enum UnifiedSessionSource {
    Closed(crate::parakeet_audio::PcmPartStore),
    Capture(crate::parakeet_live::UnifiedCapturePump),
}

impl crate::parakeet_pipeline::ReadablePcmSource for UnifiedSessionSource {
    fn read_window(&mut self, range: std::ops::Range<u64>) -> Result<Vec<i16>, String> {
        match self {
            Self::Closed(source) => source.read_window(range),
            Self::Capture(source) => source.read_window(range),
        }
    }
}

impl UnifiedSessionSource {
    fn is_capture(&self) -> bool {
        matches!(self, Self::Capture(_))
    }

    fn pump_capture(&mut self) -> Result<Vec<crate::parakeet_audio::CaptureWriteAck>, String> {
        match self {
            Self::Closed(_) => Ok(Vec::new()),
            Self::Capture(capture) => capture.pump(),
        }
    }

    fn stop_capture(
        &mut self,
    ) -> Result<Option<(u64, Vec<crate::parakeet_audio::CaptureWriteAck>)>, String> {
        match self {
            Self::Closed(_) => Ok(None),
            Self::Capture(capture) => capture.stop().map(Some),
        }
    }

    fn set_gain(
        &self,
        target_identity: &str,
        expected_revision: u64,
        gain: whisper_core::microphone_level::GainDb,
    ) -> Result<whisper_core::microphone_level::GainRevision, String> {
        match self {
            Self::Capture(capture) => capture.set_gain(target_identity, expected_revision, gain),
            Self::Closed(_) => Err("MicrophoneLevel: unified capture is not open".into()),
        }
    }

    fn take_gain_events(&mut self) -> Vec<crate::parakeet_live::AppliedCaptureGain> {
        match self {
            Self::Capture(capture) => capture.take_gain_events(),
            Self::Closed(_) => Vec::new(),
        }
    }

    #[cfg(test)]
    fn enqueue_test_block(&mut self, samples: Vec<f32>) {
        if let Self::Capture(capture) = self {
            capture.enqueue_test_block(samples);
        }
    }

    fn pump(&mut self) -> Result<Vec<crate::parakeet_audio::CaptureWriteAck>, String> {
        self.pump_capture()
    }
}

struct UnifiedSessionState {
    request: ImportRequest,
    document_drain_start_sequence: u64,
    source: UnifiedSessionSource,
    available_samples: u64,
    durable_samples: u64,
    source_closed: bool,
    document: crate::parakeet_document::ParakeetDocumentStore,
    inference_directory: PathBuf,
    native_route: Option<NativeUnifiedRoute>,
    backend_ready_reported: bool,
    finalizer: crate::parakeet_pipeline::ParakeetFinalizer,
    pending_call: Option<whisper_core::parakeet_scheduler::ScheduledCall>,
    pending_prepared: Option<(
        whisper_core::parakeet_scheduler::ScheduledCall,
        whisper_core::parakeet_session::AudioWindow,
        Vec<i16>,
        bool,
    )>,
    preview_sequence: u64,
    pending_window: Option<whisper_core::parakeet_session::AudioWindow>,
    pending_available_samples: Option<u64>,
    failure: Option<String>,
}

fn validate_native_window_pcm(
    window: whisper_core::parakeet_session::AudioWindow,
    pcm_len: usize,
) -> Result<u64, String> {
    let native_samples = window
        .native_end_sample
        .checked_sub(window.native_start_sample)
        .ok_or_else(|| "InvalidWindow: native bounds are reversed".to_owned())?;
    let pcm_samples = u64::try_from(pcm_len)
        .map_err(|_| "InvalidWindow: PCM length exceeds the addressable range".to_owned())?;
    if native_samples == 0
        || native_samples != pcm_samples
        || native_samples > whisper_core::parakeet_ipc::PARAKEET_MAX_AUDIO_SAMPLES as u64
        || window.central_start_sample < window.native_start_sample
        || window.central_end_sample > window.native_end_sample
        || window.central_start_sample >= window.central_end_sample
    {
        return Err("InvalidWindow: bounded PCM does not match its RIFF range".into());
    }
    Ok(native_samples)
}

struct NativeParakeetWindowTranscriber {
    route: NativeUnifiedRoute,
    process: Option<crate::parakeet_import::ParakeetProcess>,
    identity: Option<whisper_core::parakeet_ipc::WorkerIdentity>,
    scheduler_incarnation: Option<u64>,
    #[cfg(test)]
    harness_observer: Option<Arc<NativeQualificationObserver>>,
    #[cfg(test)]
    harness_incarnation: Option<u64>,
}

#[cfg(test)]
include!("worker_ipc/tests/runtime_support.rs");

impl Drop for NativeParakeetWindowTranscriber {
    fn drop(&mut self) {
        if let Some(mut process) = self.process.take() {
            let _ = process.terminate();
        }
    }
}

impl NativeParakeetWindowTranscriber {
    fn new(route: NativeUnifiedRoute) -> Self {
        Self {
            route,
            process: None,
            identity: None,
            scheduler_incarnation: None,
            #[cfg(test)]
            harness_observer: None,
            #[cfg(test)]
            harness_incarnation: None,
        }
    }

    #[cfg(test)]
    fn with_harness_observer(mut self, observer: Arc<NativeQualificationObserver>) -> Self {
        self.harness_observer = Some(observer);
        self
    }

    fn model_sha256() -> &'static str {
        whisper_core::model_registry::model(whisper_core::model_catalog::ModelId::ParakeetTdt06BV3)
            .artifact
            .sha256
    }

    fn verify_ready(
        &self,
        process: &mut crate::parakeet_import::ParakeetProcess,
        identity: whisper_core::parakeet_ipc::WorkerIdentity,
        backend: String,
        gpu_evidence: Option<whisper_core::parakeet_ipc::GpuEvidence>,
    ) -> Result<(), String> {
        let profile = whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS;
        let key = match self.route.compute {
            ComputeChoice::Cpu => self.route.paths.qualification_key(profile)?,
            ComputeChoice::Gpu => self.route.paths.gpu_qualification_key(profile)?,
            ComputeChoice::Auto => {
                return Err("NativeRoute: Auto must be resolved before READY".into());
            }
        };
        self.route.paths.verify_qualification(
            Self::model_sha256(),
            &key.runtime_build,
            &key.worker_build,
        )?;
        if identity.profile != profile
            || !identity
                .model_sha256
                .eq_ignore_ascii_case(Self::model_sha256())
            || !identity
                .runtime_sha256
                .eq_ignore_ascii_case(&key.runtime_build)
            || !identity
                .build_sha256
                .eq_ignore_ascii_case(&key.worker_build)
        {
            return Err("NativeReadyIdentityMismatch: model/runtime/worker/profile differs".into());
        }
        match self.route.compute {
            ComputeChoice::Cpu if backend != "CPU" => {
                Err(format!("BackendMismatch: expected CPU, received {backend}"))
            }
            ComputeChoice::Gpu => {
                let expected = self.route.expected_gpu.as_ref().ok_or_else(|| {
                    "GpuIdentityMissing: candidate identity was not captured before startup"
                        .to_owned()
                })?;
                if backend != "CUDA0"
                    || gpu_evidence.as_ref().is_none_or(|evidence| {
                        !gpu_evidence_matches(Some(evidence), Some(expected), process.pid())
                    })
                {
                    return Err(
                        "GpuAttestationMismatch: READY did not attest the selected GPU and process"
                            .into(),
                    );
                }
                Ok(())
            }
            ComputeChoice::Auto => Err("NativeRoute: unresolved Auto backend".into()),
            ComputeChoice::Cpu => Ok(()),
        }
    }
}

impl crate::parakeet_pipeline::WindowTranscriber for NativeParakeetWindowTranscriber {
    fn instance_id(&self) -> u64 {
        self.process
            .as_ref()
            .map_or(0, |process| u64::from(process.pid()))
    }

    fn initialize_instance(
        &mut self,
        stop_requested: &std::sync::atomic::AtomicBool,
    ) -> Result<(), String> {
        if !self.route.paths.is_personal_runtime() {
            return Err("NativeCandidateUnavailable: personal runtime layout required".into());
        }
        #[cfg(test)]
        let incarnation = self
            .harness_observer
            .as_ref()
            .map(|observer| observer.allocate_incarnation())
            .transpose()?;
        #[cfg(test)]
        if let (Some(observer), Some(incarnation)) = (self.harness_observer.as_ref(), incarnation) {
            observer.record(NativeQualificationEventKind::Starting { incarnation });
        }
        let log_root = self.route.paths.runtime_root.join("logs");
        fs::create_dir_all(&log_root).map_err(|error| format!("NativeLogDirectory: {error}"))?;
        let mut process =
            crate::parakeet_import::ParakeetProcess::spawn_live(&self.route.paths, &log_root)?;
        #[cfg(test)]
        if let (Some(observer), Some(incarnation)) = (self.harness_observer.as_ref(), incarnation) {
            observer.record(NativeQualificationEventKind::Spawned {
                incarnation,
                pid: process.pid(),
            });
        }
        let ready = (|| loop {
            if stop_requested.load(Ordering::Acquire) {
                return Err("NativeWorkerStartCancelled".into());
            }
            match process.poll()? {
                Some(whisper_core::parakeet_ipc::Event::Ready {
                    identity,
                    backend,
                    gpu_evidence,
                    ..
                }) => {
                    #[cfg(test)]
                    let ready_backend = backend.clone();
                    self.verify_ready(&mut process, identity.clone(), backend, gpu_evidence)?;
                    process.accept_ready(identity.clone())?;
                    #[cfg(test)]
                    if let (Some(observer), Some(incarnation)) =
                        (self.harness_observer.as_ref(), incarnation)
                    {
                        observer.record(NativeQualificationEventKind::Ready {
                            incarnation,
                            pid: process.pid(),
                            backend: ready_backend,
                            profile: identity.profile.clone(),
                            model_sha256: identity.model_sha256.clone(),
                            runtime_sha256: identity.runtime_sha256.clone(),
                            build_sha256: identity.build_sha256.clone(),
                            gpu_attested: self.route.compute == ComputeChoice::Gpu,
                        });
                    }
                    return Ok(identity);
                }
                Some(whisper_core::parakeet_ipc::Event::Failed { message, .. }) => {
                    return Err(format!("NativeWorkerReadyFailed: {message}"));
                }
                Some(event) => {
                    return Err(format!("NativeWorkerUnexpectedBeforeReady: {event:?}"));
                }
                None => thread::sleep(Duration::from_millis(5)),
            }
        })();
        match ready {
            Ok(identity) => {
                self.identity = Some(identity);
                self.process = Some(process);
                #[cfg(test)]
                {
                    self.harness_incarnation = incarnation;
                }
                Ok(())
            }
            Err(error) => {
                let pid = process.pid();
                let reap = process.terminate();
                match reap {
                    Ok(_) => {
                        #[cfg(test)]
                        if let (Some(observer), Some(incarnation)) =
                            (self.harness_observer.as_ref(), incarnation)
                        {
                            observer
                                .record(NativeQualificationEventKind::Reaped { incarnation, pid });
                        }
                        Err(error)
                    }
                    Err(reap_error) => Err(format!("{error}; NativeWorkerReap: {reap_error}")),
                }
            }
        }
    }

    fn stop_instance(&mut self) -> Result<(), String> {
        self.scheduler_incarnation = None;
        self.identity = None;
        #[cfg(test)]
        let incarnation = self.harness_incarnation.take();
        if let Some(mut process) = self.process.take() {
            let pid = process.pid();
            process
                .terminate()
                .map(|_| {
                    #[cfg(test)]
                    if let (Some(observer), Some(incarnation)) =
                        (self.harness_observer.as_ref(), incarnation)
                    {
                        observer.record(NativeQualificationEventKind::Reaped { incarnation, pid });
                    }
                })
                .map_err(|error| format!("NativeWorkerReap: {error}"))
        } else {
            Ok(())
        }
    }

    fn transcribe(
        &mut self,
        _window: whisper_core::parakeet_session::AudioWindow,
        _pcm: &[i16],
    ) -> Result<Vec<whisper_core::parakeet_ipc::Word>, String> {
        Err("NativeContextMissing: scheduled session identity is required".into())
    }

    fn transcribe_scheduled(
        &mut self,
        call: whisper_core::parakeet_scheduler::ScheduledCall,
        scratch_directory: &Path,
        window: whisper_core::parakeet_session::AudioWindow,
        pcm: &[i16],
    ) -> Result<Vec<whisper_core::parakeet_ipc::Word>, String> {
        if let Some(incarnation) = self.scheduler_incarnation {
            if incarnation != call.incarnation {
                return Err(
                    "SchedulerIncarnationMismatch: call belongs to a recycled executor".into(),
                );
            }
        } else {
            self.scheduler_incarnation = Some(call.incarnation);
        }
        let expected_samples = validate_native_window_pcm(window, pcm.len())?;
        let expected_instance = self
            .process
            .as_ref()
            .map(|process| u64::from(process.pid()))
            .ok_or_else(|| "NativeWorkerUnavailable: process is not READY".to_owned())?;
        let process = self
            .process
            .as_mut()
            .ok_or_else(|| "NativeWorkerUnavailable: process is not READY".to_owned())?;
        let identity = self
            .identity
            .as_ref()
            .ok_or_else(|| "NativeWorkerUnavailable: identity is not READY".to_owned())?;
        fs::create_dir_all(scratch_directory)
            .map_err(|error| format!("NativeWindowDirectory: {error}"))?;
        let suffix = match call.work {
            whisper_core::parakeet_scheduler::ScheduledWork::Final { sequence }
            | whisper_core::parakeet_scheduler::ScheduledWork::Preview { sequence } => sequence,
        };
        let source_path = scratch_directory.join(format!(
            "window-{:032x}-{}-{}-{}-{suffix}.wav",
            call.session.job_id.0,
            call.session.generation.get(),
            call.incarnation,
            call.request_id
        ));
        let operation = (|| {
            write_pcm16_wav(&source_path, pcm)?;
            let source_sha256 = crate::parakeet_import::sha256_file(&source_path)?;
            let job_id = format!("{:032x}", call.session.job_id.0);
            let attempt = call
                .attempt
                .checked_add(1)
                .ok_or_else(|| "SessionAttemptIdentityExhausted".to_owned())?;
            let wire_request_id = process.transcribe_with_gpu_correlated(
                job_id.clone(),
                call.session.generation.get(),
                attempt,
                &source_path,
                source_sha256.clone(),
                self.route.expected_gpu.clone(),
            )?;
            #[cfg(test)]
            if let (Some(observer), Some(incarnation)) =
                (self.harness_observer.as_ref(), self.harness_incarnation)
            {
                let work = match call.work {
                    whisper_core::parakeet_scheduler::ScheduledWork::Final { sequence } => {
                        NativeQualificationWork::Final(sequence)
                    }
                    whisper_core::parakeet_scheduler::ScheduledWork::Preview { sequence } => {
                        NativeQualificationWork::Preview(sequence)
                    }
                };
                observer.record(NativeQualificationEventKind::RequestSent {
                    incarnation,
                    pid: process.pid(),
                    session: call.session,
                    generation: call.session.generation,
                    attempt,
                    request_id: wire_request_id,
                    work,
                });
            }
            let native_marker_required = self.route.compute == ComputeChoice::Gpu;
            let mut native_call_started = false;
            loop {
                match process.poll()? {
                    Some(whisper_core::parakeet_ipc::Event::NativeCallStarted {
                        request_id,
                        job_id: actual_job,
                        generation,
                        attempt: actual_attempt,
                        pid,
                    }) if request_id == wire_request_id
                        && actual_job == job_id
                        && generation == call.session.generation.get()
                        && actual_attempt == attempt
                        && u64::from(pid) == expected_instance =>
                    {
                        native_call_started = true;
                    }
                    Some(whisper_core::parakeet_ipc::Event::Result {
                        request_id,
                        job_id: actual_job,
                        generation,
                        attempt: actual_attempt,
                        identity: actual_identity,
                        source_sha256: actual_sha,
                        duration_samples,
                        words,
                        gpu_evidence,
                        ..
                    }) if request_id == wire_request_id
                        && actual_job == job_id
                        && generation == call.session.generation.get()
                        && actual_attempt == attempt
                        && actual_identity == *identity
                        && actual_sha.eq_ignore_ascii_case(&source_sha256)
                        && duration_samples == expected_samples
                        && (!native_marker_required || native_call_started)
                        && (self.route.compute != ComputeChoice::Gpu
                            || gpu_evidence.as_ref().is_some_and(|evidence| {
                                self.route.expected_gpu.as_ref().is_some_and(|expected| {
                                    gpu_evidence_matches(
                                        Some(evidence),
                                        Some(expected),
                                        process.pid(),
                                    )
                                })
                            })) =>
                    {
                        #[cfg(test)]
                        if let (Some(observer), Some(incarnation)) =
                            (self.harness_observer.as_ref(), self.harness_incarnation)
                        {
                            observer.record(NativeQualificationEventKind::Result {
                                incarnation,
                                pid: process.pid(),
                                session: call.session,
                                generation: call.session.generation,
                                attempt,
                                request_id: wire_request_id,
                                work: match call.work {
                                    whisper_core::parakeet_scheduler::ScheduledWork::Final {
                                        sequence,
                                    } => NativeQualificationWork::Final(sequence),
                                    whisper_core::parakeet_scheduler::ScheduledWork::Preview {
                                        sequence,
                                    } => NativeQualificationWork::Preview(sequence),
                                },
                                word_count: words.len(),
                            });
                        }
                        return Ok(words);
                    }
                    Some(whisper_core::parakeet_ipc::Event::Failed {
                        request_id: Some(request_id),
                        job_id: Some(actual_job),
                        generation: Some(generation),
                        attempt: Some(actual_attempt),
                        identity: Some(actual_identity),
                        message,
                        ..
                    }) if request_id == wire_request_id
                        && actual_job == job_id
                        && generation == call.session.generation.get()
                        && actual_attempt == attempt
                        && actual_identity == *identity =>
                    {
                        return Err(message);
                    }
                    Some(event) => return Err(format!("NativeResponseMismatch: {event:?}")),
                    None => thread::sleep(Duration::from_millis(5)),
                }
            }
        })();
        let remove_result = fs::remove_file(&source_path);
        match (operation, remove_result) {
            (Ok(words), Ok(())) => Ok(words),
            (Ok(words), Err(error)) if error.kind() == io::ErrorKind::NotFound => Ok(words),
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(format!("NativeWindowCleanup: {error}")),
        }
    }
}

struct UnifiedInferenceCommand {
    call: whisper_core::parakeet_scheduler::ScheduledCall,
    window: whisper_core::parakeet_session::AudioWindow,
    pcm: Vec<i16>,
    scratch_directory: PathBuf,
}

struct UnifiedInferenceResult {
    call: whisper_core::parakeet_scheduler::ScheduledCall,
    instance_id: u64,
    words: Result<Vec<whisper_core::parakeet_ipc::Word>, String>,
}

enum UnifiedExecutorCommand {
    Infer(UnifiedInferenceCommand),
    Stop,
}

struct UnifiedInferenceExecutor {
    commands: SyncSender<UnifiedExecutorCommand>,
    results: Receiver<UnifiedInferenceResult>,
    ready: Receiver<Result<(), String>>,
    ready_seen: bool,
    thread: Option<
        thread::JoinHandle<Result<Box<dyn crate::parakeet_pipeline::WindowTranscriber>, String>>,
    >,
    worker: Option<Box<dyn crate::parakeet_pipeline::WindowTranscriber>>,
    stop_requested: Option<Arc<std::sync::atomic::AtomicBool>>,
    in_flight: bool,
    #[cfg(test)]
    disconnect_result_for: Option<whisper_core::parakeet_scheduler::SchedulerSession>,
    #[cfg(test)]
    panic_during_transcription_for: Option<whisper_core::parakeet_scheduler::SchedulerSession>,
    #[cfg(test)]
    next_child_incarnation: u64,
    #[cfg(test)]
    current_child_incarnation: u64,
    #[cfg(test)]
    child_observer: Option<Arc<Mutex<Vec<ControlledChildObservation>>>>,
    #[cfg(test)]
    ready_gate: Option<Arc<(Mutex<bool>, std::sync::Condvar)>>,
}

impl UnifiedInferenceExecutor {
    fn is_available(&self) -> bool {
        self.thread.is_some() || self.worker.is_some()
    }

    fn start(worker: Box<dyn crate::parakeet_pipeline::WindowTranscriber>) -> Result<Self, String> {
        #[cfg(test)]
        {
            Self::start_inner(worker, None, None)
        }
        #[cfg(not(test))]
        {
            Self::start_inner(worker)
        }
    }

    #[cfg(test)]
    fn start_observed(
        worker: Box<dyn crate::parakeet_pipeline::WindowTranscriber>,
        observer: Arc<Mutex<Vec<ControlledChildObservation>>>,
    ) -> Result<Self, String> {
        Self::start_inner(worker, Some(observer), None)
    }

    #[cfg(test)]
    fn start_observed_with_ready_gate(
        worker: Box<dyn crate::parakeet_pipeline::WindowTranscriber>,
        observer: Arc<Mutex<Vec<ControlledChildObservation>>>,
        ready_gate: Arc<(Mutex<bool>, std::sync::Condvar)>,
    ) -> Result<Self, String> {
        Self::start_inner(worker, Some(observer), Some(ready_gate))
    }

    fn start_inner(
        worker: Box<dyn crate::parakeet_pipeline::WindowTranscriber>,
        #[cfg(test)] child_observer: Option<Arc<Mutex<Vec<ControlledChildObservation>>>>,
        #[cfg(test)] ready_gate: Option<Arc<(Mutex<bool>, std::sync::Condvar)>>,
    ) -> Result<Self, String> {
        let (commands, command_rx) = mpsc::sync_channel(1);
        let (result_tx, results) = mpsc::sync_channel(1);
        let (ready_tx, ready) = mpsc::sync_channel(1);
        drop(command_rx);
        drop(result_tx);
        drop(ready_tx);
        let mut executor = Self {
            commands,
            results,
            ready,
            ready_seen: false,
            thread: None,
            worker: Some(worker),
            stop_requested: None,
            in_flight: false,
            #[cfg(test)]
            disconnect_result_for: None,
            #[cfg(test)]
            panic_during_transcription_for: None,
            #[cfg(test)]
            next_child_incarnation: 0,
            #[cfg(test)]
            current_child_incarnation: 0,
            #[cfg(test)]
            child_observer,
            #[cfg(test)]
            ready_gate,
        };
        executor.start_instance()?;
        Ok(executor)
    }

    fn start_instance(&mut self) -> Result<(), String> {
        let worker = self
            .worker
            .take()
            .ok_or_else(|| "InferenceStart: worker is missing".to_owned())?;
        let (commands, command_rx) = mpsc::sync_channel(1);
        let (result_tx, results) = mpsc::sync_channel(1);
        let (ready_tx, ready) = mpsc::sync_channel(1);
        #[cfg(test)]
        let disconnect_result_for = self.disconnect_result_for;
        #[cfg(test)]
        let panic_during_transcription_for = self.panic_during_transcription_for;
        #[cfg(test)]
        {
            if let Some(observer) = &self.child_observer {
                let last_observed_incarnation = observer
                    .lock()
                    .expect("controlled child observation mutex")
                    .iter()
                    .map(|event| event.incarnation)
                    .max()
                    .unwrap_or(0);
                self.next_child_incarnation =
                    self.next_child_incarnation.max(last_observed_incarnation);
            }
            self.next_child_incarnation = self.next_child_incarnation.saturating_add(1);
            self.current_child_incarnation = self.next_child_incarnation;
        }
        #[cfg(test)]
        let child_incarnation = self.current_child_incarnation;
        #[cfg(test)]
        let child_observer = self.child_observer.clone();
        #[cfg(test)]
        let ready_gate = self.ready_gate.clone();
        let stop_requested = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let thread_stop_requested = Arc::clone(&stop_requested);
        self.thread = Some(
            thread::Builder::new()
                .name("parakeet-unified-inference".into())
                .spawn(move || -> Result<Box<dyn crate::parakeet_pipeline::WindowTranscriber>, String> {
                    let mut worker = worker;
                    let mut child_jobs = BTreeMap::<u128, (Generation, u64)>::new();
                    let mut wire_request_id = 0_u64;
                    #[cfg(test)]
                    observe_controlled_child(
                        child_observer.as_ref(), child_incarnation, "Spawn", None, None, None,
                        wire_request_id, child_jobs.len(),
                    );
                    if let Err(error) = worker.initialize_instance(&thread_stop_requested) {
                        let outcome = match worker.stop_instance() {
                            Ok(()) => {
                                let _ = ready_tx.send(Err(error));
                                Ok(worker)
                            }
                            Err(reap_error) => {
                                let error = format!("{error}; InferenceStartupReap: {reap_error}");
                                let _ = ready_tx.send(Err(error.clone()));
                                Err(error)
                            }
                        };
                        return outcome;
                    }
                    #[cfg(test)]
                    if let Some(gate) = ready_gate.as_ref() {
                        let (released, changed) = &**gate;
                        let mut released = released.lock().expect("controlled ready gate");
                        while !*released {
                            released = changed.wait(released).expect("controlled ready gate");
                        }
                    }
                    #[cfg(test)]
                    observe_controlled_child(
                        child_observer.as_ref(), child_incarnation, "Ready", None, None, None,
                        wire_request_id, child_jobs.len(),
                    );
                    let _ = ready_tx.send(Ok(()));
                    while let Ok(command) = command_rx.recv() {
                        match command {
                            UnifiedExecutorCommand::Infer(command) => {
                                let Some(next_wire_request_id) = wire_request_id.checked_add(1)
                                else {
                                    let _ = result_tx.send(UnifiedInferenceResult {
                                        call: command.call,
                                        instance_id: worker.instance_id(),
                                        words: Err("WireRequestIdentityExhausted".into()),
                                    });
                                    break;
                                };
                                wire_request_id = next_wire_request_id;
                                #[cfg(test)]
                                if panic_during_transcription_for == Some(command.call.session) {
                                    panic!("controlled unified worker panic");
                                }
                                let instance_id = worker.instance_id();
                                let session = command.call.session;
                                let logical_attempt = command.call.attempt.checked_add(1);
                                let child_can_add_job =
                                    child_jobs.len() < UNIFIED_CHILD_JOB_CAPACITY;
                                let job_key = session.job_id.0;
                                let admission = match (logical_attempt, child_jobs.get(&job_key)) {
                                    (None, _) => Err("SessionAttemptIdentityExhausted".to_owned()),
                                    (Some(_), Some((generation, _)))
                                        if *generation != session.generation =>
                                    {
                                        Err("ChildSessionMismatch: JobId is bound to another generation".into())
                                    }
                                    (Some(attempt), Some((_, previous_attempt)))
                                        if attempt < *previous_attempt =>
                                    {
                                        Err("ChildAttemptStale: logical attempt regressed".into())
                                    }
                                    (Some(attempt), Some((generation, _))) => {
                                        child_jobs.insert(job_key, (*generation, attempt));
                                        Ok(attempt)
                                    }
                                    (Some(attempt), None)
                                        if child_can_add_job =>
                                    {
                                        child_jobs.insert(job_key, (session.generation, attempt));
                                        Ok(attempt)
                                    }
                                    (Some(_), None) => Err(format!(
                                        "ChildSessionCapacity: at most {UNIFIED_CHILD_JOB_CAPACITY} JobIds per worker instance"
                                    )),
                                };
                                #[cfg(test)]
                                observe_controlled_child(
                                    child_observer.as_ref(),
                                    child_incarnation,
                                    if admission.is_ok() { "Command" } else { "Rejected" },
                                    Some(session.job_id),
                                    Some(session.generation),
                                    logical_attempt,
                                    wire_request_id,
                                    child_jobs.len(),
                                );
                                let words = admission.and_then(|_| {
                                    worker.transcribe_scheduled(
                                        command.call,
                                        &command.scratch_directory,
                                        command.window,
                                        &command.pcm,
                                    )
                                });
                                #[cfg(test)]
                                if disconnect_result_for == Some(command.call.session) {
                                    break;
                                }
                                if result_tx
                                    .send(UnifiedInferenceResult {
                                        call: command.call,
                                        instance_id,
                                        words,
                                    })
                                    .is_err()
                                {
                                    break;
                                }
                            }
                            UnifiedExecutorCommand::Stop => {
                                #[cfg(test)]
                                observe_controlled_child(
                                    child_observer.as_ref(), child_incarnation, "Stop", None, None,
                                    None, wire_request_id, child_jobs.len(),
                                );
                                break;
                            }
                        }
                    }
                    #[cfg(test)]
                    observe_controlled_child(
                        child_observer.as_ref(), child_incarnation, "Exit", None, None, None,
                        wire_request_id, child_jobs.len(),
                    );
                    worker.stop_instance()?;
                    Ok(worker)
                })
                .map_err(|error| format!("InferenceStart: {error}"))?,
        );
        self.commands = commands;
        self.results = results;
        self.ready = ready;
        self.stop_requested = Some(stop_requested);
        self.ready_seen = false;
        Ok(())
    }

    fn is_ready(&mut self) -> Result<bool, String> {
        if !self.ready_seen {
            match self.ready.try_recv() {
                Ok(Ok(())) => self.ready_seen = true,
                Ok(Err(error)) => return Err(error),
                Err(mpsc::TryRecvError::Empty) => {}
                Err(mpsc::TryRecvError::Disconnected) => {
                    return Err("InferenceReady: worker exited before READY".into());
                }
            }
        }
        Ok(self.ready_seen)
    }

    fn dispatch(&mut self, command: UnifiedInferenceCommand) -> Result<(), String> {
        if self.in_flight {
            return Err("Scheduler: inference executor already has an active call".into());
        }
        self.commands
            .try_send(UnifiedExecutorCommand::Infer(command))
            .map_err(|error| format!("InferenceDispatch: {error}"))?;
        self.in_flight = true;
        Ok(())
    }

    fn try_result(&mut self) -> Result<Option<UnifiedInferenceResult>, String> {
        match self.results.try_recv() {
            Ok(result) => {
                self.in_flight = false;
                Ok(Some(result))
            }
            Err(mpsc::TryRecvError::Empty) => Ok(None),
            Err(mpsc::TryRecvError::Disconnected) if !self.in_flight => Ok(None),
            Err(mpsc::TryRecvError::Disconnected) => {
                Err("InferenceWorkerExited: result channel disconnected".into())
            }
        }
    }

    fn stop_and_reap(&mut self) -> Result<(), String> {
        if self.in_flight {
            return Err("InferenceReap: call is still in flight".into());
        }
        if let Some(stop_requested) = self.stop_requested.take() {
            stop_requested.store(true, Ordering::Release);
        }
        let thread = self
            .thread
            .take()
            .ok_or_else(|| "InferenceReap: worker instance is missing".to_owned())?;
        let stop_result = if thread.is_finished() {
            Ok(())
        } else {
            self.commands
                .send(UnifiedExecutorCommand::Stop)
                .map_err(|error| format!("InferenceStop: {error}"))
        };
        let worker = thread
            .join()
            .map_err(|_| "InferenceReap: worker thread panicked".to_owned())??;
        self.worker = Some(worker);
        stop_result?;
        #[cfg(test)]
        observe_controlled_child(
            self.child_observer.as_ref(),
            self.current_child_incarnation,
            "Reaped",
            None,
            None,
            None,
            0,
            0,
        );
        Ok(())
    }

    fn reap_disconnected(&mut self) -> Result<(), String> {
        let thread = self
            .thread
            .take()
            .ok_or_else(|| "InferenceReap: disconnected worker instance is missing".to_owned())?;
        self.in_flight = false;
        match thread.join() {
            Ok(Ok(worker)) => {
                self.worker = Some(worker);
                #[cfg(test)]
                observe_controlled_child(
                    self.child_observer.as_ref(),
                    self.current_child_incarnation,
                    "Reaped",
                    None,
                    None,
                    None,
                    0,
                    0,
                );
                Ok(())
            }
            Ok(Err(error)) => Err(format!("InferenceReap: {error}")),
            Err(_) => {
                #[cfg(test)]
                observe_controlled_child(
                    self.child_observer.as_ref(),
                    self.current_child_incarnation,
                    "Reaped",
                    None,
                    None,
                    None,
                    0,
                    0,
                );
                Err("InferenceReap: disconnected worker thread panicked".into())
            }
        }
    }

    fn restart(&mut self) -> Result<(), String> {
        if self.in_flight || self.thread.is_some() {
            return Err("InferenceRestart: previous instance has not exited".into());
        }
        self.start_instance()
    }

    fn replace_reaped_worker(
        &mut self,
        worker: Box<dyn crate::parakeet_pipeline::WindowTranscriber>,
    ) -> Result<(), String> {
        if self.in_flight || self.thread.is_some() {
            return Err("InferenceReplace: previous instance has not been reaped".into());
        }
        let Some(reaped_worker) = self.worker.replace(worker) else {
            return Err("InferenceReplace: stopped worker is unavailable".into());
        };
        drop(reaped_worker);
        self.start_instance()
    }
}

struct ParakeetVadRun {
    request: ImportRequest,
    decoder: Option<crate::decoder::DecoderProxy<ChildWorkerTransport>>,
    decode_offset: u64,
    total_samples: u64,
    source_sha256: String,
    pcm: Vec<i16>,
    windows: Vec<whisper_core::parakeet_windows::Window>,
    next_window: usize,
    next_segment_sequence: u64,
    request_count: u64,
    current_attempt: u64,
    current_path: Option<PathBuf>,
    current_source_sha256: Option<String>,
    pending_confirmation: Option<(whisper_core::parakeet_windows::Window, u64)>,
    worker_ready: bool,
    ready_event_sent: bool,
}

impl Drop for ParakeetVadRun {
    fn drop(&mut self) {
        if let Some(path) = self.current_path.take() {
            let _ = fs::remove_file(path);
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ParakeetStoppedExit {
    Wait,
    Success,
    Failure,
}

fn parakeet_stopped_exit(status: Option<bool>) -> ParakeetStoppedExit {
    match status {
        None => ParakeetStoppedExit::Wait,
        Some(true) => ParakeetStoppedExit::Success,
        Some(false) => ParakeetStoppedExit::Failure,
    }
}

fn write_pcm16_wav(path: &Path, samples: &[i16]) -> Result<(), String> {
    let data_bytes = samples
        .len()
        .checked_mul(2)
        .and_then(|bytes| u32::try_from(bytes).ok())
        .ok_or_else(|| "InvalidInput: VAD WAV exceeds RIFF size limit".to_owned())?;
    let riff_size = data_bytes
        .checked_add(36)
        .ok_or_else(|| "InvalidInput: VAD WAV exceeds RIFF size limit".to_owned())?;
    let mut file = fs::File::create(path)
        .map_err(|error| format!("StorageUnavailable: create VAD WAV: {error}"))?;
    file.write_all(b"RIFF")
        .and_then(|()| file.write_all(&riff_size.to_le_bytes()))
        .and_then(|()| file.write_all(b"WAVEfmt "))
        .and_then(|()| file.write_all(&16_u32.to_le_bytes()))
        .and_then(|()| file.write_all(&1_u16.to_le_bytes()))
        .and_then(|()| file.write_all(&1_u16.to_le_bytes()))
        .and_then(|()| file.write_all(&16_000_u32.to_le_bytes()))
        .and_then(|()| file.write_all(&32_000_u32.to_le_bytes()))
        .and_then(|()| file.write_all(&2_u16.to_le_bytes()))
        .and_then(|()| file.write_all(&16_u16.to_le_bytes()))
        .and_then(|()| file.write_all(b"data"))
        .and_then(|()| file.write_all(&data_bytes.to_le_bytes()))
        .map_err(|error| format!("StorageUnavailable: write VAD WAV header: {error}"))?;
    let mut bytes = Vec::with_capacity(samples.len().min(32_000) * 2);
    for chunk in samples.chunks(16_000) {
        bytes.clear();
        bytes.extend(chunk.iter().flat_map(|sample| sample.to_le_bytes()));
        file.write_all(&bytes)
            .map_err(|error| format!("StorageUnavailable: write VAD WAV samples: {error}"))?;
    }
    file.sync_all()
        .map_err(|error| format!("StorageUnavailable: sync VAD WAV: {error}"))
}

fn parakeet_vad_segments(
    words: &[whisper_core::parakeet_ipc::Word],
    window: whisper_core::parakeet_windows::Window,
    total_samples: u64,
    job_id: JobId,
    generation: Generation,
    instance_id: u64,
    first_sequence: u64,
) -> Result<Vec<whisper_core::ipc::WorkerSegmentDto>, String> {
    let absolute =
        whisper_core::parakeet_windows::absolute_word_samples(&window, total_samples, words)
            .map_err(|error| format!("InvalidTimestamp: {error}"))?;
    let mut groups: Vec<(u64, u64, Vec<String>)> = Vec::new();
    let mut previous_end = 0_u64;
    for (word, time) in words.iter().zip(absolute) {
        let token = word.text.trim();
        if token.is_empty()
            || !vad_word_overlap_within_one_tdt_frame(time.start_sample, previous_end)
            || time.start_sample >= time.end_sample
        {
            return Err("InvalidTimestamp: Parakeet VAD word is empty or unordered".into());
        }
        previous_end = previous_end.max(time.end_sample);
        if !whisper_core::parakeet_windows::word_belongs_to_central_range(
            time.start_sample,
            time.end_sample,
            window.central_start_sample,
            window.central_end_sample,
        ) {
            continue;
        }
        append_vad_parakeet_word_group(&mut groups, time.start_sample, time.end_sample, token)?;
    }
    groups
        .into_iter()
        .enumerate()
        .map(|(index, (start, end, tokens))| {
            let sequence = first_sequence
                .checked_add(index as u64)
                .ok_or_else(|| "InvalidInput: Parakeet segment sequence exhausted".to_owned())?;
            let range = whisper_core::SourceRange::new(start, end, 16_000)
                .ok_or_else(|| "InvalidTimestamp: VAD segment range is invalid".to_owned())?;
            Ok(whisper_core::ipc::WorkerSegmentDto {
                job_id,
                generation,
                instance_id,
                segment_id: whisper_core::SegmentId(sequence),
                range,
                text: tokens.join(" "),
            })
        })
        .collect()
}

fn queued_request_identity_matches(
    durable: &ImportRequest,
    requested: &ImportRequest,
    same_model: bool,
) -> bool {
    if durable == requested {
        return true;
    }
    if !same_model {
        return false;
    }
    let mut comparable = durable.clone();
    comparable.model_path = requested.model_path.clone();
    comparable.config.compute = requested.config.compute;
    comparable == *requested
}

#[derive(Clone, Copy)]
struct FinishedLiveWindow {
    sequence: u64,
    range: whisper_core::SourceRange,
    last_segment_sequence: u64,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LiveRecoveryState {
    None,
    AutoRetryPending,
    AwaitingChoicePending,
    AwaitingChoice,
}
struct LiveCaptureState {
    request: LiveRequest,
    capture: CaptureStream,
    probe: crate::microphone_probe::MicrophoneProbe,
    spectrum_reader: crate::microphone_probe::MicrophoneSpectrumReader,
    last_meter_sequence: u64,
    next_spectrum_telemetry: bool,
    telemetry_due_after_event: bool,
    converter: Pcm16Converter,
    vad: VoiceActivity,
    staging: PcmWriter,
    frame_pending: Vec<i16>,
    inference_cursor: u64,
    inference_sequence: u64,
    pending_window: Option<(u64, u64, u64, Vec<i16>)>,
    replay_until: u64,
    replay_windows_queued: u64,
    replay_windows_to_skip: u64,
    segment_sequence_base: u64,
    window_sequence_base: u64,
    recovery: LiveRecoveryState,
    barrier_m: Option<u64>,
    inference_admission_closed: bool,
    worker_ready: bool,
    stop_requested: bool,
    stop_enqueued: bool,
    started_at_unix_ms: u64,
    captured: u64,
    admitted: u64,
    speech: u64,
    confirmed_fragment_end: u64,
    last_reported_durable: u64,
    failure: Option<String>,
}
impl LiveCaptureState {
    fn stop_capture(&mut self) {
        self.probe.invalidate();
        self.capture.stop();
    }
}
struct PendingLiveState {
    request: LiveRequest,
    staging: PcmWriter,
    live_archive: LiveArchive,
    recovery: LiveRecoveryState,
}
struct ImportRequestState {
    job_id: JobId,
    generation: Generation,
    instance_id: u64,
    backend: BackendKind,
    attempt_number: u64,
    import_request: Option<whisper_core::ports::ImportRequest>,
    recovery: LiveRecoveryState,
    startup_ready: bool,
    gpu_candidate: Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
    gpu_attested: bool,
}

fn pending_unified_event_identity(event: &ImportEvent) -> Option<(JobId, Generation)> {
    match event {
        ImportEvent::Failed {
            job_id, generation, ..
        }
        | ImportEvent::MicrophoneGainApplied {
            job_id, generation, ..
        }
        | ImportEvent::MicrophoneGainRejected {
            job_id, generation, ..
        }
        | ImportEvent::ParakeetCaptureStopped {
            job_id, generation, ..
        } => Some((*job_id, *generation)),
        ImportEvent::UnifiedDocumentPage {
            job_id, generation, ..
        }
        | ImportEvent::UnifiedAudioPartsPage {
            job_id, generation, ..
        }
        | ImportEvent::UnifiedPublicationArtifacts {
            job_id, generation, ..
        } => Some((*job_id, *generation)),
        _ => None,
    }
}

impl ImportRuntime {
    fn register_prepared_native_route(
        &mut self,
        session: whisper_core::parakeet_scheduler::SchedulerSession,
        route: NativeUnifiedRoute,
    ) -> Result<(), String> {
        if !self.native_unified_routes.contains_key(&session) && self.unified_admission_full() {
            return Err("SchedulerCapacity: two unified sessions are already admitted".into());
        }
        #[cfg(test)]
        let route_observation = (
            route.requested_compute,
            route.compute,
            route.candidate.is_some(),
            route.expected_gpu.is_some(),
            route.fallback_reason.clone(),
        );
        self.native_unified_routes.insert(session, route);
        #[cfg(test)]
        if let Some(observer) = &self.native_harness_observer {
            let (requested_compute, compute, has_candidate, has_expected_gpu, fallback_reason) =
                route_observation;
            observer.record(NativeQualificationEventKind::PreparedRoute {
                session,
                requested_compute,
                compute,
                has_candidate,
                has_expected_gpu,
                fallback_reason,
            });
        }
        Ok(())
    }

    fn native_route_for_unified_import(
        &self,
        request: &ImportRequest,
        candidate: Option<&whisper_core::gpu_inventory::GpuRuntimeIdentity>,
    ) -> Result<Option<NativeUnifiedRoute>, String> {
        if request.strategy
            != whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
        {
            return Ok(None);
        }
        if self.native_unified_candidate_enabled {
            return native_unified_route(
                request,
                self.parakeet_paths.as_ref(),
                self.gpu_parakeet_paths.as_ref(),
                candidate,
            )
            .map(Some);
        }
        #[cfg(test)]
        {
            if self.controlled_transcriber.is_none() && self.unified_executor.is_none() {
                return Err(
                    "ModelNotQualified: unified strategy requires the injected controlled worker"
                        .into(),
                );
            }
            Ok(None)
        }
        #[cfg(not(test))]
        {
            Err("ModelNotQualified: unified Parakeet strategy is not enabled in the native composition".into())
        }
    }

    fn is_controlled_bounded_test(&self, request: &ImportRequest) -> bool {
        if request.strategy
            != whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
        {
            return false;
        }
        #[cfg(test)]
        {
            self.controlled_transcriber.is_some()
        }
        #[cfg(not(test))]
        {
            false
        }
    }

    #[cfg(test)]
    fn observe_source_binding_map(&self, operation: &'static str) {
        let Some(observer) = &self.native_harness_observer else {
            return;
        };
        let mut bindings = self.source_bindings.keys().copied().collect::<Vec<_>>();
        bindings.sort_unstable_by_key(|(job_id, generation)| (job_id.0, *generation));
        observer.record(NativeQualificationEventKind::SourceBindingMap {
            operation,
            bindings,
        });
    }

    fn start_controlled_unified_import(&mut self, request: &ImportRequest) -> Result<(), String> {
        let session = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: request.job_id,
            generation: request.generation,
        };
        if self.unified_admission_full_for(session) {
            return Err("SchedulerCapacity: two unified sessions are already admitted".into());
        }
        let native_route = self.native_unified_routes.remove(&session);
        let executor_available = self
            .unified_executor
            .as_ref()
            .is_some_and(UnifiedInferenceExecutor::is_available);
        #[cfg(test)]
        let controlled_worker_available = self.controlled_transcriber.is_some();
        #[cfg(not(test))]
        let controlled_worker_available = false;
        if !controlled_worker_available && !executor_available && native_route.is_none() {
            return Err("ModelNotQualified: controlled worker injection is required".into());
        }
        if self.native_unified_candidate_enabled
            && native_route.is_none()
            && !controlled_worker_available
        {
            return Err(
                "NativeCandidateUnavailable: the session has no validated fixed backend route"
                    .into(),
            );
        }
        let executor_incarnation_live = self
            .unified_executor
            .as_ref()
            .is_some_and(|executor| executor.thread.is_some() || executor.in_flight);
        if let (Some(current), Some(next)) = (&self.native_executor_route, &native_route)
            && !current.same_worker(next)
            && (executor_incarnation_live || self.parakeet_scheduler.active_sessions() > 0)
        {
            return Err("NativeRouteBusy: all sessions in one executor incarnation must use the same fixed backend candidate".into());
        }
        if let Some(next) = &native_route
            && self
                .unified_sessions
                .values()
                .filter_map(|state| state.native_route.as_ref())
                .any(|current| !current.same_worker(next))
        {
            return Err(
                "NativeRouteBusy: admitted sessions must share one fixed backend candidate".into(),
            );
        }
        let work = PathBuf::from(&request.destination)
            .join("transcriptions")
            .join(format!("{:032x}", request.job_id.0))
            .join(request.generation.get().to_string())
            .join("parakeet-unified");
        fs::create_dir_all(&work)
            .map_err(|error| format!("StorageUnavailable: unified workspace: {error}"))?;
        let source_path = work.join("source.wav");
        let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_path)
            .map_err(|error| format!("AudioSource: {error}"))?;
        crate::parakeet_audio::decode_file_to_parts(&request.source_path, &mut writer)?;
        writer
            .finish()
            .map_err(|error| format!("AudioSource: {error}"))?;
        let source = crate::parakeet_audio::PcmPartStore::open(&source_path);
        let available_samples = source
            .total_samples()
            .map_err(|error| format!("AudioSource: {error}"))?;
        let document = crate::parakeet_document::ParakeetDocumentStore::create(
            work.join("document"),
            request.job_id,
            request.generation,
        )
        .map_err(|error| format!("Document: {error:?}"))?;
        let resume_checkpoint = self.unified_resume_checkpoints.get(&session).cloned();
        let document_drain_start_sequence = resume_checkpoint
            .as_ref()
            .map_or(0, |checkpoint| checkpoint.pipeline.confirmed_segments)
            .checked_add(1)
            .ok_or_else(|| "StorageCorrupt: resume document sequence exhausted".to_owned())?;
        let mut document = document;
        if let Some(checkpoint) = &resume_checkpoint {
            restore_unified_document_prefix(
                &mut document,
                checkpoint,
                request.job_id,
                request.generation,
                available_samples,
            )?;
        }
        let finalizer = match resume_checkpoint.as_ref() {
            Some(checkpoint) => crate::parakeet_pipeline::ParakeetFinalizer::from_checkpoint(
                request.job_id,
                request.generation,
                checkpoint.pipeline,
            )?,
            None => {
                crate::parakeet_pipeline::ParakeetFinalizer::new(request.job_id, request.generation)
            }
        };
        self.parakeet_scheduler
            .admit(session)
            .map_err(|error| format!("SchedulerAdmission: {error:?}"))?;
        let publication_gate = Arc::new(PublicationGate::default());
        publication_gate.activate();
        self.unified_publication_gates
            .insert(session, Arc::clone(&publication_gate));
        self.unified_sessions.insert(
            session,
            UnifiedSessionState {
                request: request.clone(),
                document_drain_start_sequence,
                source: UnifiedSessionSource::Closed(source),
                available_samples,
                durable_samples: available_samples,
                source_closed: true,
                document,
                inference_directory: work.join("native-windows"),
                native_route,
                backend_ready_reported: false,
                finalizer,
                pending_call: None,
                pending_prepared: None,
                preview_sequence: 0,
                pending_window: None,
                pending_available_samples: None,
                failure: None,
            },
        );
        let archive = std::mem::replace(&mut self.archive, ArchiveStore::new(&request.destination));
        self.unified_archives.insert(session, archive);
        self.unified_resume_checkpoints.remove(&session);
        Ok(())
    }

    fn start_unified_capture(
        &mut self,
        request: &whisper_core::ports::LiveRequest,
        capture: CaptureStream,
        test_part_limit: Option<u64>,
        native_route: Option<NativeUnifiedRoute>,
    ) -> Result<(), String> {
        self.validate_unified_capture_admission(request, native_route.as_ref())?;
        let session = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: request.job_id,
            generation: request.generation,
        };
        let work = PathBuf::from(&request.destination)
            .join("transcriptions")
            .join(format!("{:032x}", request.job_id.0))
            .join(request.generation.get().to_string())
            .join("parakeet-unified");
        fs::create_dir_all(&work)
            .map_err(|error| format!("StorageUnavailable: unified workspace: {error}"))?;
        let source_path = work.join("source.wav");
        let capture = if let Some(limit) = test_part_limit {
            #[cfg(test)]
            {
                crate::parakeet_live::UnifiedCapturePump::from_capture_with_test_part_limit(
                    capture,
                    source_path.clone(),
                    limit,
                )
            }
            #[cfg(not(test))]
            {
                let _ = (capture, source_path, limit);
                return Err("TestOnlyRotationUnavailable".into());
            }
        } else {
            crate::parakeet_live::UnifiedCapturePump::from_capture(capture, source_path.clone())
        }
        .map_err(|error| format!("CaptureStart: {error}"))?;
        let document = crate::parakeet_document::ParakeetDocumentStore::create(
            work.join("document"),
            request.job_id,
            request.generation,
        )
        .map_err(|error| format!("Document: {error:?}"))?;
        self.parakeet_scheduler
            .admit(session)
            .map_err(|error| format!("SchedulerAdmission: {error:?}"))?;
        let publication_gate = Arc::new(PublicationGate::default());
        publication_gate.activate();
        self.unified_publication_gates
            .insert(session, Arc::clone(&publication_gate));
        self.unified_sessions.insert(
            session,
            UnifiedSessionState {
                request: ImportRequest {
                    strategy:
                        whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1,
                    job_id: request.job_id,
                    generation: request.generation,
                    source_path: source_path.to_string_lossy().into_owned(),
                    source_sha256: None,
                    source_samples: None,
                    model_path: request.model_path.clone(),
                    model_sha256: request.model_sha256,
                    destination: request.destination.clone(),
                    config: request.config.clone(),
                },
                document_drain_start_sequence: 1,
                source: UnifiedSessionSource::Capture(capture),
                available_samples: 0,
                durable_samples: 0,
                source_closed: false,
                document,
                inference_directory: work.join("native-windows"),
                native_route,
                backend_ready_reported: false,
                finalizer: crate::parakeet_pipeline::ParakeetFinalizer::new(
                    request.job_id,
                    request.generation,
                ),
                pending_call: None,
                pending_prepared: None,
                preview_sequence: 0,
                pending_window: None,
                pending_available_samples: None,
                failure: None,
            },
        );
        self.unified_archives
            .insert(session, ArchiveStore::new(&request.destination));
        self.unified_capture_sessions.insert(session);
        Ok(())
    }

    fn validate_unified_capture_admission(
        &self,
        request: &whisper_core::ports::LiveRequest,
        native_route: Option<&NativeUnifiedRoute>,
    ) -> Result<(), String> {
        if request.destination.trim().is_empty() || request.model_path.trim().is_empty() {
            return Err("Micro Parakeet : modèle ou dossier de sortie invalide".into());
        }
        whisper_core::microphone_level::GainDb::new(request.microphone_gain_db)
            .map_err(|error| format!("MicrophoneLevel: {error}"))?;
        if let whisper_core::MicrophoneChoice::Device(identity) = &request.microphone {
            whisper_core::microphone_level::MicrophoneIdentity::new(identity)
                .map_err(|error| format!("MicrophoneLevel: {error}"))?;
        }
        if self.unified_admission_full() {
            return Err("SchedulerCapacity: two unified sessions are already admitted".into());
        }
        if self
            .unified_sessions
            .values()
            .any(|state| state.source.is_capture() && !state.source_closed)
        {
            return Err("SchedulerCapacity: an open unified capture is already admitted".into());
        }
        let executor_available = self
            .unified_executor
            .as_ref()
            .is_some_and(UnifiedInferenceExecutor::is_available);
        #[cfg(test)]
        let controlled_worker_available = self.controlled_transcriber.is_some();
        #[cfg(not(test))]
        let controlled_worker_available = false;
        if !controlled_worker_available && !executor_available && native_route.is_none() {
            return Err("ModelNotQualified: controlled worker injection is required".into());
        }
        let session = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: request.job_id,
            generation: request.generation,
        };
        if self.native_unified_candidate_enabled && native_route.is_none() {
            return Err(
                "NativeCandidateUnavailable: the capture has no validated fixed backend route"
                    .into(),
            );
        }
        let executor_incarnation_live = self
            .unified_executor
            .as_ref()
            .is_some_and(|executor| executor.thread.is_some() || executor.in_flight);
        if let (Some(current), Some(next)) = (&self.native_executor_route, &native_route)
            && !current.same_worker(next)
            && (executor_incarnation_live || self.parakeet_scheduler.active_sessions() > 0)
        {
            return Err("NativeRouteBusy: all sessions in one executor incarnation must use the same fixed backend candidate".into());
        }
        if let Some(next) = &native_route
            && self
                .unified_sessions
                .values()
                .filter_map(|state| state.native_route.as_ref())
                .any(|current| !current.same_worker(next))
        {
            return Err(
                "NativeRouteBusy: admitted sessions must share one fixed backend candidate".into(),
            );
        }
        Ok(())
    }

    fn start_native_unified_capture_with<F>(
        &mut self,
        request: &whisper_core::ports::LiveRequest,
        native_route: NativeUnifiedRoute,
        open_capture: F,
    ) -> Result<(), String>
    where
        F: FnOnce(
            &whisper_core::microphone::MicrophoneChoice,
            whisper_core::microphone_level::GainDb,
        ) -> Result<CaptureStream, String>,
    {
        self.validate_unified_capture_admission(request, Some(&native_route))?;
        let gain = whisper_core::microphone_level::GainDb::new(request.microphone_gain_db)
            .map_err(|error| format!("MicrophoneLevel: {error}"))?;
        let capture = open_capture(&request.microphone, gain)?;
        self.start_unified_capture(request, capture, None, Some(native_route))
    }

    fn stop_controlled_unified_capture(
        &mut self,
        job_id: JobId,
        generation: Generation,
        events: &mut VecDeque<ImportEvent>,
    ) -> Result<(), String> {
        let session = whisper_core::parakeet_scheduler::SchedulerSession { job_id, generation };
        let Some(state) = self.unified_sessions.get_mut(&session) else {
            return Err("UnifiedCaptureMissing".into());
        };
        if !state.source.is_capture() {
            return Err("UnifiedCaptureSourceMismatch".into());
        }
        if state.source_closed {
            return Ok(());
        }
        let (total_samples, final_acknowledgements) = state
            .source
            .stop_capture()
            .map_err(|error| format!("CaptureStop: {error}"))?
            .ok_or_else(|| "UnifiedCaptureMissing".to_owned())?;
        let final_durable_samples = state
            .durable_samples
            .max(
                final_acknowledgements
                    .iter()
                    .map(|ack| ack.flushed_samples)
                    .max()
                    .unwrap_or(0),
            )
            .min(total_samples);
        if final_durable_samples != total_samples {
            return Err(format!(
                "CaptureStop: final durable acknowledgement {} does not cover closed source {total_samples}",
                final_durable_samples
            ));
        }
        let final_gain_updates = state.source.take_gain_events();
        state.available_samples = total_samples;
        state.durable_samples = final_durable_samples;
        state.source_closed = true;
        let source = crate::parakeet_audio::PcmPartStore::open(&state.request.source_path);
        let request = self
            .unified_archives
            .get_mut(&session)
            .ok_or_else(|| "ArchiveSessionMissing".to_owned())?
            .prepare_segmented_capture(state.request.clone(), &source, None, None)?;
        state.request = request.clone();
        self.parakeet_scheduler
            .clear_preview(session)
            .map_err(|error| format!("SchedulerPreview: {error:?}"))?;
        for gain in final_gain_updates {
            events.push_back(ImportEvent::MicrophoneGainApplied {
                job_id,
                generation,
                target_identity: gain.target_identity,
                gain_db: gain.gain_db,
                gain_revision: gain.gain_revision,
                first_frame: gain.first_frame,
            });
        }
        events.push_back(ImportEvent::Prepared(request));
        events.push_back(ImportEvent::ParakeetCaptureProgress {
            job_id,
            generation,
            // Unified capture progress is correlated by JobId and generation.
            session_id: 0,
            captured_samples: state.available_samples,
            audio_durable_samples: state.durable_samples,
        });
        Ok(())
    }

    #[cfg(test)]
    fn run_controlled_unified_import(&mut self, request: &ImportRequest) -> Result<(), String> {
        let mut transcriber = self.controlled_transcriber.take().ok_or_else(|| {
            "ModelNotQualified: controlled worker injection is required".to_owned()
        })?;
        let work = PathBuf::from(&request.destination)
            .join("transcriptions")
            .join(format!("{:032x}", request.job_id.0))
            .join(request.generation.get().to_string())
            .join("parakeet-unified");
        fs::create_dir_all(&work)
            .map_err(|error| format!("StorageUnavailable: unified workspace: {error}"))?;
        let source_path = work.join("source.wav");
        let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_path)
            .map_err(|error| format!("AudioSource: {error}"))?;
        crate::parakeet_audio::decode_file_to_parts(&request.source_path, &mut writer)?;
        writer
            .finish()
            .map_err(|error| format!("AudioSource: {error}"))?;
        let source = crate::parakeet_audio::PcmPartStore::open(&source_path);
        let mut document = crate::parakeet_document::ParakeetDocumentStore::create(
            work.join("document"),
            request.job_id,
            request.generation,
        )
        .map_err(|error| format!("Document: {error:?}"))?;
        let session = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: request.job_id,
            generation: request.generation,
        };
        self.parakeet_scheduler
            .admit(session)
            .map_err(|error| format!("SchedulerAdmission: {error:?}"))?;
        let total_samples = source
            .total_samples()
            .map_err(|error| format!("AudioSource: {error}"))?;
        let mut finalizer =
            crate::parakeet_pipeline::ParakeetFinalizer::new(request.job_id, request.generation);
        let result = loop {
            if finalizer.processed_samples() < total_samples {
                if let Err(error) = self
                    .parakeet_scheduler
                    .queue_final(session, finalizer.next_window_sequence())
                {
                    self.parakeet_scheduler
                        .abort_session(session)
                        .map_err(|cleanup| format!("SchedulerQueue: {error:?}; {cleanup:?}"))?;
                    self.parakeet_scheduler
                        .confirm_child_exited()
                        .map_err(|cleanup| format!("SchedulerQueue: {error:?}; {cleanup:?}"))?;
                    return Err(format!("SchedulerQueue: {error:?}"));
                }
                let call = match self.parakeet_scheduler.next_call() {
                    Ok(Some(call)) => call,
                    Ok(None) => {
                        self.parakeet_scheduler
                            .abort_session(session)
                            .map_err(|error| format!("SchedulerReady: {error:?}"))?;
                        self.parakeet_scheduler
                            .confirm_child_exited()
                            .map_err(|error| format!("SchedulerReap: {error:?}"))?;
                        return Err("SchedulerReady: final range was not selected".into());
                    }
                    Err(error) => {
                        self.parakeet_scheduler
                            .abort_session(session)
                            .map_err(|cleanup| {
                                format!("SchedulerSelect: {error:?}; {cleanup:?}")
                            })?;
                        self.parakeet_scheduler
                            .confirm_child_exited()
                            .map_err(|cleanup| {
                                format!("SchedulerSelect: {error:?}; {cleanup:?}")
                            })?;
                        return Err(format!("SchedulerSelect: {error:?}"));
                    }
                };
                let step = {
                    let mut scheduled = ScheduledWindowTranscriber {
                        scheduler: &mut self.parakeet_scheduler,
                        call,
                        transcriber: transcriber.as_mut(),
                    };
                    finalizer.step(
                        &mut &source,
                        &mut document,
                        &mut scheduled,
                        total_samples,
                        true,
                    )
                };
                match step {
                    Ok(crate::parakeet_pipeline::FinalizationStep::WindowProcessed { .. }) => {
                        self.parakeet_scheduler
                            .complete(call)
                            .map_err(|error| format!("SchedulerComplete: {error:?}"))?;
                    }
                    Ok(crate::parakeet_pipeline::FinalizationStep::Complete(result)) => {
                        self.parakeet_scheduler
                            .fail_call(call)
                            .map_err(|error| format!("SchedulerComplete: {error:?}"))?;
                        self.parakeet_scheduler
                            .abort_session(session)
                            .map_err(|error| format!("SchedulerTerminal: {error:?}"))?;
                        self.parakeet_scheduler
                            .confirm_child_exited()
                            .map_err(|error| format!("SchedulerReap: {error:?}"))?;
                        return Err(format!(
                            "SchedulerSource: completed without a final call ({})",
                            result.windows_completed
                        ));
                    }
                    Ok(crate::parakeet_pipeline::FinalizationStep::WaitingForAudio { .. }) => {
                        self.parakeet_scheduler
                            .fail_call(call)
                            .map_err(|error| format!("SchedulerComplete: {error:?}"))?;
                        self.parakeet_scheduler
                            .abort_session(session)
                            .map_err(|error| format!("SchedulerTerminal: {error:?}"))?;
                        self.parakeet_scheduler
                            .confirm_child_exited()
                            .map_err(|error| format!("SchedulerReap: {error:?}"))?;
                        return Err(
                            "SchedulerSource: closed unified audio unexpectedly waits".into()
                        );
                    }
                    Err(error) => {
                        self.parakeet_scheduler
                            .fail_call(call)
                            .map_err(|scheduler_error| {
                                format!("{error}; SchedulerFailure: {scheduler_error:?}")
                            })?;
                        self.parakeet_scheduler.mark_terminal(session).map_err(
                            |scheduler_error| {
                                format!("{error}; SchedulerTerminal: {scheduler_error:?}")
                            },
                        )?;
                        let _ = self.parakeet_scheduler.confirm_child_exited();
                        return Err(error);
                    }
                }
            } else {
                match finalizer.step(
                    &mut &source,
                    &mut document,
                    transcriber.as_mut(),
                    total_samples,
                    true,
                )? {
                    crate::parakeet_pipeline::FinalizationStep::Complete(result) => break result,
                    _ => return Err("SchedulerSource: final document did not complete".into()),
                }
            }
        };
        self.parakeet_scheduler
            .mark_terminal(session)
            .map_err(|error| format!("SchedulerTerminal: {error:?}"))?;
        self.parakeet_scheduler
            .confirm_child_exited()
            .map_err(|error| format!("SchedulerReap: {error:?}"))?;
        self.document_drains.push_back(UnifiedDocumentDrain {
            document,
            job_id: request.job_id,
            generation: request.generation,
            revision: result.document_revision,
            total_samples: result.total_samples,
            next_sequence: 1,
            ready_pending: true,
            progress_pending: true,
        });
        Ok(())
    }

    fn start_diagnostics(&mut self, destination: &Path) {
        match DiagnosticSink::start(destination) {
            Ok(sink) => {
                self.diagnostics = Some(sink);
                self.diagnostic_start_failed = false;
            }
            Err(_) => {
                self.diagnostics = None;
                self.diagnostic_start_failed = true;
            }
        }
    }

    fn bind_import_request(
        &self,
        request: &whisper_core::ports::ImportRequest,
    ) -> Result<whisper_core::ports::ImportRequest, String> {
        self.model_catalog.as_ref().map_or_else(
            || Ok(request.clone()),
            |catalog| catalog.bind_import(request),
        )
    }

    fn bind_live_request(
        &self,
        request: &whisper_core::ports::LiveRequest,
    ) -> Result<whisper_core::ports::LiveRequest, String> {
        self.model_catalog
            .as_ref()
            .map_or_else(|| Ok(request.clone()), |catalog| catalog.bind_live(request))
    }

    fn confirm_live_window(
        &mut self,
        window: FinishedLiveWindow,
    ) -> Result<ImportEvent, (JobId, Generation, String)> {
        let active = self.active.as_ref().ok_or((
            JobId(0),
            Generation::first(),
            "StaleResponse: finished window has no active worker".into(),
        ))?;
        let id = (active.job_id, active.generation);
        let durable = self
            .live
            .as_ref()
            .map_or(0, |live| live.staging.durable_samples());
        let confirmed_samples = self
            .live_archive
            .as_mut()
            .ok_or((
                id.0,
                id.1,
                "StorageUnavailable: live archive is missing".into(),
            ))?
            .persist_window_finished(
                window.sequence,
                window.range,
                window.last_segment_sequence,
                durable,
            )
            .map_err(|error| (id.0, id.1, error))?;
        crate::archive::campaign_gate(
            "live.before_window_ticket",
            id.0.0,
            id.1.get(),
            active.instance_id,
        )
        .map_err(|error| (id.0, id.1, error))?;
        if let Some(progress) = self.progress.as_mut() {
            progress.observe(
                ProgressPhase::Draining,
                confirmed_samples,
                std::time::Instant::now(),
            );
        }
        Ok(ImportEvent::WindowFinished {
            job_id: id.0,
            generation: id.1,
            window_sequence: window.sequence,
            confirmed_samples,
        })
    }
}

impl ServiceRuntime for ImportRuntime {
    fn apply_effect(&mut self, effect: ImportEffect, pending: &mut VecDeque<ImportEvent>) {
        if let (Some(identity), Some(gates)) = (
            import_effect_identity(&effect),
            self.automatic_publication_gates.as_ref(),
        ) {
            if let Some(gate) = gates
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .get(&identity)
                .cloned()
            {
                self.publication_gate = gate;
            }
        }
        self.effect(effect, pending);
    }

    fn poll_event(&mut self) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
        self.poll()
    }

    fn pump_realtime(&mut self) -> Option<(JobId, Generation, String)> {
        if let Some(direct) = self.direct_live.as_mut() {
            let (job_id, generation) = direct.identity();
            return match direct.pump_capture() {
                Ok(()) => None,
                Err(error) => {
                    let error = direct.fail(error);
                    self.direct_live = None;
                    Some((job_id, generation, error))
                }
            };
        }
        if let Some(live) = self.parakeet_live.as_mut() {
            if let Err(error) = live.pump_capture() {
                let event = live.stop_after_capture_error(error);
                let event = attach_automatic_final_import(live, event);
                self.parakeet_pending_events.push_back(event);
                self.parakeet_live = None;
            }
        }
        None
    }
}

fn recovery_stop_counters(events: VecDeque<ImportEvent>) -> Option<ImportEvent> {
    events
        .into_iter()
        .find(|event| matches!(event, ImportEvent::LiveCounters { .. }))
}

#[cfg(test)]
mod tests;
