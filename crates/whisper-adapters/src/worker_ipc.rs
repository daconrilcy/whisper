mod ipc;
mod publication;
mod runtime;
use runtime::ImportRuntime;
#[cfg(test)]
use runtime::MicrophoneTestCapture;
#[cfg(test)]
use runtime::{resolve_reprocessing_targets, verified_present_publications};
mod service;
mod support;

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
use support::{NEXT_IMPORT_JOB_ID, NEXT_INSTANCE};

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

/// Check the shipped CUDA backend without loading or discarding the model.
pub fn probe_cuda_worker(executable: &Path) -> Result<(), String> {
    if !executable.is_file() {
        return Err("GPU indisponible : worker CUDA introuvable ; choisissez CPU".into());
    }
    let mut worker = ChildWorkerTransport::spawn(executable)
        .map_err(|error| format!("GPU indisponible : {error:?} ; choisissez CPU"))?;
    let result = (|| {
        worker
            .request(WorkerCommand::ProbeCuda)
            .map_err(|error| format!("GPU indisponible : {error:?} ; choisissez CPU"))?;
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            match worker.receive() {
                Ok(Some(WorkerEvent::CudaAvailable)) => return Ok(()),
                Ok(Some(WorkerEvent::Failed { message, .. })) => {
                    return Err(format!("GPU indisponible : {message} ; choisissez CPU"));
                }
                Ok(Some(_)) | Ok(None) => {}
                Err(error) => return Err(format!("GPU indisponible : {error:?} ; choisissez CPU")),
            }
            if Instant::now() >= deadline {
                return Err(
                    "GPU indisponible : délai d'initialisation CUDA dépassé ; choisissez CPU"
                        .into(),
                );
            }
            thread::sleep(Duration::from_millis(10));
        }
    })();
    worker.abort();
    result
}

/// Ask the isolated GPU worker to return per-identity qualification records.
pub fn qualify_gpu_worker(
    executable: &Path,
    generation: u64,
    targets: Vec<whisper_core::ipc::GpuQualificationTarget>,
) -> Result<(String, Vec<whisper_core::ipc::GpuQualificationResult>), String> {
    let mut worker = ChildWorkerTransport::spawn(executable)
        .map_err(|error| format!("GPU worker indisponible : {error:?}"))?;
    let result = (|| {
        worker
            .request(WorkerCommand::QualifyGpus {
                generation,
                targets,
            })
            .map_err(|error| format!("Qualification GPU impossible : {error:?}"))?;
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            match worker.receive() {
                Ok(Some(WorkerEvent::GpusQualified {
                    generation: response_generation,
                    build_architectures,
                    devices,
                })) if response_generation == generation => {
                    return Ok((build_architectures, devices));
                }
                Ok(Some(WorkerEvent::GpusQualified { .. })) => {
                    return Err("Réponse de qualification GPU périmée.".into());
                }
                Ok(Some(WorkerEvent::Failed { message, .. })) => return Err(message),
                Ok(Some(_)) | Ok(None) => {}
                Err(error) => return Err(format!("Transport GPU interrompu : {error:?}")),
            }
            if Instant::now() >= deadline {
                return Err("Délai de qualification GPU dépassé.".into());
            }
            thread::sleep(Duration::from_millis(10));
        }
    })();
    worker.abort();
    result
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

#[derive(Clone, Debug)]
struct NativeUnifiedRoute {
    paths: crate::parakeet_import::ParakeetWorkerPaths,
    requested_compute: ComputeChoice,
    compute: ComputeChoice,
    expected_gpu: Option<whisper_core::parakeet_ipc::ExpectedGpu>,
    candidate: Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
    fallback_reason: Option<String>,
}

impl NativeUnifiedRoute {
    fn same_worker(&self, other: &Self) -> bool {
        self.compute == other.compute
            && self.paths.executable == other.paths.executable
            && self.paths.native_dir == other.paths.native_dir
            && self.paths.model == other.paths.model
            && self.paths.runtime_root == other.paths.runtime_root
            && self.expected_gpu == other.expected_gpu
            && self.candidate == other.candidate
    }

    fn backend_label(&self) -> String {
        match (&self.compute, &self.fallback_reason) {
            (ComputeChoice::Gpu, _) => "CUDA0".into(),
            (ComputeChoice::Cpu, Some(reason)) => {
                format!("CPU (Auto fallback before audio: {reason})")
            }
            (ComputeChoice::Cpu, None) => "CPU".into(),
            (ComputeChoice::Auto, _) => "Unresolved Auto".into(),
        }
    }
}

fn can_retry_native_auto_on_cpu(
    requested_compute: ComputeChoice,
    effective_compute: ComputeChoice,
    backend_ready_reported: bool,
    call_in_flight: bool,
) -> bool {
    requested_compute == ComputeChoice::Auto
        && effective_compute == ComputeChoice::Gpu
        && !backend_ready_reported
        && !call_in_flight
}

struct NativeAutoCpuRetry {
    call: whisper_core::parakeet_scheduler::ScheduledCall,
    failed_manual_gpu: Vec<FailedManualGpuPeer>,
}

struct FailedManualGpuPeer {
    session: whisper_core::parakeet_scheduler::SchedulerSession,
    message: String,
}

fn manual_gpu_failure_event(peer: FailedManualGpuPeer) -> ImportEvent {
    ImportEvent::Failed {
        job_id: peer.session.job_id,
        generation: peer.session.generation,
        message: peer.message,
    }
}

fn retry_native_gpu_startup_on_cpu(
    scheduler: &mut whisper_core::parakeet_scheduler::ParakeetScheduler,
    sessions: &mut HashMap<whisper_core::parakeet_scheduler::SchedulerSession, UnifiedSessionState>,
    archives: &mut HashMap<whisper_core::parakeet_scheduler::SchedulerSession, ArchiveStore>,
    executor_route: &mut Option<NativeUnifiedRoute>,
    executor: &mut UnifiedInferenceExecutor,
    current_state: &mut UnifiedSessionState,
    call: whisper_core::parakeet_scheduler::ScheduledCall,
    cause: &str,
    mut make_cpu_route: impl FnMut(&ImportRequest, &str) -> Result<NativeUnifiedRoute, String>,
    make_cpu_worker: impl FnOnce(
        NativeUnifiedRoute,
    ) -> Box<dyn crate::parakeet_pipeline::WindowTranscriber>,
) -> Result<Option<NativeAutoCpuRetry>, String> {
    let auto_gpu_unstarted = |state: &UnifiedSessionState| {
        state.native_route.as_ref().is_some_and(|route| {
            state.request.config.compute == route.requested_compute
                && can_retry_native_auto_on_cpu(
                    route.requested_compute,
                    route.compute,
                    state.backend_ready_reported,
                    state.pending_call.is_some(),
                )
        })
    };
    if !auto_gpu_unstarted(current_state) {
        return Ok(None);
    }

    let mut auto_peers = Vec::new();
    let mut manual_gpu_peers = Vec::new();
    for (session, state) in sessions.iter() {
        let same_worker = current_state
            .native_route
            .as_ref()
            .zip(state.native_route.as_ref())
            .is_some_and(|(current, peer)| current.same_worker(peer));
        if same_worker && auto_gpu_unstarted(state) {
            auto_peers.push(*session);
            continue;
        }

        let manual_gpu_unstarted = same_worker
            && state.request.config.compute == ComputeChoice::Gpu
            && state.native_route.as_ref().is_some_and(|route| {
                route.requested_compute == ComputeChoice::Gpu
                    && route.compute == ComputeChoice::Gpu
                    && !state.backend_ready_reported
                    && state.pending_call.is_none()
                    && state.pending_prepared.is_none()
                    && state.pending_window.is_none()
            });
        if manual_gpu_unstarted {
            manual_gpu_peers.push(*session);
            continue;
        }
        return Ok(None);
    }

    let fallback_reason = format!("GPU pre-audio startup failed: {cause}");
    let current_route = make_cpu_route(&current_state.request, &fallback_reason)
        .map_err(|error| format!("{fallback_reason}; CPU fallback route unavailable: {error}"))?;
    let mut peer_routes = HashMap::new();
    for session in &auto_peers {
        let state = sessions
            .get(session)
            .expect("auto GPU peer remains in the session table");
        let route = make_cpu_route(&state.request, &fallback_reason).map_err(|error| {
            format!(
                "{fallback_reason}; CPU fallback route unavailable for {:?}/{:?}: {error}",
                session.job_id, session.generation
            )
        })?;
        peer_routes.insert(*session, route);
    }

    if executor.stop_and_reap().is_err() {
        return Ok(None);
    }
    let mut failed_manual_gpu = Vec::new();
    for session in &manual_gpu_peers {
        let state = sessions
            .get_mut(session)
            .expect("manual GPU peer remains in the session table");
        let mut message = format!(
            "GPU pre-audio startup failed: {cause}; manual GPU sessions do not fall back to CPU"
        );
        if state.source.is_capture() && !state.source_closed {
            match state.source.stop_capture() {
                Ok(Some((total_samples, acknowledgements))) => {
                    let durable_samples = state
                        .durable_samples
                        .max(
                            acknowledgements
                                .iter()
                                .map(|ack| ack.flushed_samples)
                                .max()
                                .unwrap_or(0),
                        )
                        .min(total_samples);
                    state.available_samples = total_samples;
                    state.durable_samples = durable_samples;
                    state.source_closed = true;
                    if durable_samples != total_samples {
                        message.push_str(&format!(
                            "; CaptureStop: final durable acknowledgement {durable_samples} does not cover closed source {total_samples}"
                        ));
                    } else {
                        let source =
                            crate::parakeet_audio::PcmPartStore::open(&state.request.source_path);
                        match archives
                            .get_mut(session)
                            .ok_or_else(|| "ArchiveSessionMissing".to_owned())
                            .and_then(|archive| {
                                archive
                                    .prepare_segmented_capture(
                                        state.request.clone(),
                                        &source,
                                        None,
                                        None,
                                    )
                                    .map_err(|error| format!("CaptureArchive: {error}"))
                            }) {
                            Ok(request) => state.request = request,
                            Err(error) => message.push_str(&format!("; {error}")),
                        }
                    }
                    message.push_str(&format!(
                        "; manual capture drained and closed at {total_samples} samples ({durable_samples} durable)"
                    ));
                }
                Ok(None) => message.push_str("; CaptureStop: capture source was unavailable"),
                Err(error) => message.push_str(&format!("; CaptureStop: {error}")),
            }
        }
        scheduler
            .abort_session(*session)
            .map_err(|error| format!("SchedulerManualGpuPeer: {error:?}"))?;
        failed_manual_gpu.push(FailedManualGpuPeer {
            session: *session,
            message,
        });
    }
    scheduler
        .request_recycle()
        .map_err(|error| format!("SchedulerRecycle: {error:?}"))?;
    scheduler
        .cancel_reservation(call)
        .map_err(|error| format!("SchedulerReservation: {error:?}"))?;
    scheduler
        .confirm_child_exited()
        .map_err(|error| format!("SchedulerReap: {error:?}"))?;

    current_state.native_route = Some(current_route.clone());
    for (session, state) in sessions.iter_mut() {
        if let Some(route) = peer_routes.remove(session) {
            state.native_route = Some(route);
            state.backend_ready_reported = false;
        }
    }
    for peer in &failed_manual_gpu {
        sessions.remove(&peer.session);
    }
    *executor_route = Some(current_route.clone());
    executor.replace_reaped_worker(make_cpu_worker(current_route))?;

    let retry = scheduler
        .next_call()
        .map_err(|error| format!("SchedulerRetry: {error:?}"))?
        .ok_or_else(|| {
            "SchedulerRetry: reserved work disappeared during CPU fallback".to_owned()
        })?;
    if retry.session != call.session
        || retry.request_id != call.request_id
        || retry.attempt != call.attempt
        || retry.work != call.work
        || retry.incarnation == call.incarnation
    {
        let _ = scheduler.cancel_reservation(retry);
        let _ = scheduler.request_recycle();
        return Ok(None);
    }
    Ok(Some(NativeAutoCpuRetry {
        call: retry,
        failed_manual_gpu,
    }))
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

struct RuntimeModelCatalog {
    catalog: crate::model_catalog::LocalModelCatalog,
    known_historical_turbo_path: String,
    parakeet_paths: Option<crate::parakeet_import::ParakeetWorkerPaths>,
    gpu_parakeet_paths: Option<crate::parakeet_import::ParakeetWorkerPaths>,
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

impl RuntimeModelCatalog {
    fn clone_for_worker(&self) -> Self {
        Self {
            catalog: crate::model_catalog::LocalModelCatalog::new(
                self.catalog.root().to_path_buf(),
            ),
            known_historical_turbo_path: self.known_historical_turbo_path.clone(),
            parakeet_paths: self.parakeet_paths.clone(),
            gpu_parakeet_paths: self.gpu_parakeet_paths.clone(),
        }
    }

    fn snapshot(&self) -> crate::model_catalog::CatalogSnapshot {
        self.catalog.inspect(0)
    }

    fn bind_import(
        &self,
        request: &whisper_core::ports::ImportRequest,
    ) -> Result<whisper_core::ports::ImportRequest, String> {
        use whisper_core::model_catalog::ModelId;
        let snapshot = self.snapshot();
        let id = snapshot
            .resolve_model_identity(&request.model_path, &self.known_historical_turbo_path)?;
        if id == ModelId::WhisperLargeV3Turbo {
            return snapshot.bind_turbo_import_request(request, &self.known_historical_turbo_path);
        }
        if id != ModelId::ParakeetTdt06BV3 {
            return Err("ModelUnknown: modèle non reconnu".into());
        }
        if request.strategy
            == whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
        {
            return Ok(request.clone());
        }
        let record = whisper_core::model_registry::model(id);
        let source_samples = request.source_samples.or_else(|| {
            crate::archive::source_output_samples(Path::new(&request.source_path)).ok()
        });
        if source_samples.is_some_and(|samples| {
            samples
                > whisper_core::model_registry::PARAKEET_MAX_SINGLE_PASS_DURATION_MS
                    .saturating_mul(16)
        }) {
            return Err(
                "DurationUnsupported: Fichier trop long pour Parakeet, utilisez Turbo".into(),
            );
        }
        let profile = if source_samples.is_some_and(|samples| {
            samples > whisper_core::parakeet_ipc::PARAKEET_MAX_AUDIO_SAMPLES as u64
        }) {
            whisper_core::parakeet_windows::PROFILE_IMPORT_VAD
        } else {
            whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS
        };
        let (paths, key) = match request.config.compute {
            whisper_core::ComputeChoice::Cpu | whisper_core::ComputeChoice::Auto => {
                let paths = self.parakeet_paths.as_ref().ok_or_else(|| {
                    "ModelNotQualified: worker CPU Test Parakeet absent".to_owned()
                })?;
                (paths, paths.qualification_key(profile)?)
            }
            whisper_core::ComputeChoice::Gpu => {
                let paths = self.gpu_parakeet_paths.as_ref().ok_or_else(|| {
                    "BackendUnavailable: worker GPU Test Parakeet absent".to_owned()
                })?;
                let key = paths.gpu_qualification_key(profile)?;
                if !gpu_qualification_candidate_allowed(&key) {
                    return Err("ModelNotQualified: Parakeet GPU candidate is not admitted".into());
                }
                (paths, key)
            }
        };
        paths.verify_qualification(
            record.artifact.sha256,
            &key.runtime_build,
            &key.worker_build,
        )?;
        if !matches!(
            &request.config.language,
            whisper_core::LanguageChoice::Automatic
        ) {
            return Err(
                "ModelNotQualified: Parakeet accepte seulement l'import et la langue automatique"
                    .into(),
            );
        }
        if profile == whisper_core::parakeet_windows::PROFILE_IMPORT_VAD
            && !key.is_parakeet_vad_test()
        {
            return Err("ModelNotQualified: import-vad Test key is unavailable".into());
        }
        if !matches!(
            record.admission,
            whisper_core::model_registry::AdmissionBasis::Measured(
                whisper_core::engine_capabilities::Qualification::Qualified
            )
        ) {
            return Err("ModelNotQualified: la fiche Test Parakeet n'est pas qualifiée".into());
        }
        let entry = snapshot
            .entries
            .iter()
            .find(|entry| entry.id == id)
            .ok_or_else(|| "ModelMissing: Parakeet absent du catalogue".to_owned())?;
        if entry.availability != whisper_core::model_catalog::ModelAvailability::Ready
            || request.model_sha256 != decode_sha256(record.artifact.sha256)?
            || entry.expected_size_bytes != record.artifact.size_bytes
            || !entry
                .observed_sha256
                .as_deref()
                .is_some_and(|hash| hash.eq_ignore_ascii_case(record.artifact.sha256))
        {
            return Err(
                "ModelHashMismatch: artefact Parakeet différent du catalogue qualifié".into(),
            );
        }
        let mut executable = request.clone();
        executable.model_path = entry.path.to_string_lossy().into_owned();
        Ok(executable)
    }

    fn bind_live(
        &self,
        request: &whisper_core::ports::LiveRequest,
    ) -> Result<whisper_core::ports::LiveRequest, String> {
        self.snapshot()
            .bind_turbo_live_request(request, &self.known_historical_turbo_path)
    }

    fn bind_native_unified_live(&self, request: &LiveRequest) -> Result<LiveRequest, String> {
        use whisper_core::model_catalog::ModelId;
        let id = self
            .snapshot()
            .resolve_model_identity(&request.model_path, &self.known_historical_turbo_path)?;
        if id != ModelId::ParakeetTdt06BV3 {
            return Err("ModelUnknown: native unified capture requires Parakeet TDT v3".into());
        }
        let record = whisper_core::model_registry::model(id);
        if request.model_sha256 != decode_sha256(record.artifact.sha256)? {
            return Err("ModelHashMismatch: native unified capture request differs from the registered artifact".into());
        }
        let model_path = self
            .parakeet_paths
            .as_ref()
            .or(self.gpu_parakeet_paths.as_ref())
            .ok_or_else(|| "NativeCandidateUnavailable: personal model path is absent".to_owned())?
            .model
            .clone();
        let mut bound = request.clone();
        bound.model_path = model_path.to_string_lossy().into_owned();
        Ok(bound)
    }

    fn bind_parakeet_live(
        &self,
        request: &LiveRequest,
        candidate: Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
    ) -> Result<
        (
            LiveRequest,
            crate::parakeet_import::ParakeetWorkerPaths,
            Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
        ),
        String,
    > {
        use whisper_core::model_catalog::ModelId;
        let snapshot = self.snapshot();
        if snapshot
            .resolve_model_identity(&request.model_path, &self.known_historical_turbo_path)?
            != ModelId::ParakeetTdt06BV3
        {
            return Err("ModelUnknown: la voie microphone Parakeet exige Parakeet TDT v3".into());
        }
        let requested = request.config.compute;
        let mut effective = match requested {
            ComputeChoice::Cpu => ComputeChoice::Cpu,
            ComputeChoice::Gpu => ComputeChoice::Gpu,
            ComputeChoice::Auto if candidate.is_some() && self.gpu_parakeet_paths.is_some() => {
                ComputeChoice::Gpu
            }
            ComputeChoice::Auto => ComputeChoice::Cpu,
        };
        let make_import = |compute| ImportRequest {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
            job_id: request.job_id,
            generation: request.generation,
            source_path: request.model_path.clone(),
            source_sha256: None,
            source_samples: Some(1),
            model_path: request.model_path.clone(),
            model_sha256: request.model_sha256,
            destination: request.destination.clone(),
            config: whisper_core::JobConfig {
                compute,
                ..request.config.clone()
            },
        };
        let bound = match self.bind_import(&make_import(effective)) {
            Ok(bound) => bound,
            Err(error) if requested == ComputeChoice::Auto && effective == ComputeChoice::Gpu => {
                effective = ComputeChoice::Cpu;
                self.bind_import(&make_import(effective))
                    .map_err(|cpu_error| {
                        format!("Auto GPU indisponible ({error}) ; CPU indisponible ({cpu_error})")
                    })?
            }
            Err(error) => return Err(error),
        };
        let paths =
            match effective {
                ComputeChoice::Cpu => self.parakeet_paths.clone().ok_or_else(|| {
                    "ModelNotQualified: worker CPU Test Parakeet absent".to_owned()
                })?,
                ComputeChoice::Gpu => self.gpu_parakeet_paths.clone().ok_or_else(|| {
                    "BackendUnavailable: worker GPU Test Parakeet absent".to_owned()
                })?,
                ComputeChoice::Auto => unreachable!(),
            };
        let mut bound_request = request.clone();
        bound_request.model_path = bound.model_path;
        bound_request.config.compute = effective;
        let candidate = if effective == ComputeChoice::Gpu {
            candidate
        } else {
            None
        };
        Ok((bound_request, paths, candidate))
    }

    fn is_parakeet_live_request(&self, request: &LiveRequest) -> Result<bool, String> {
        self.snapshot()
            .resolve_model_identity(&request.model_path, &self.known_historical_turbo_path)
            .map(|id| id == whisper_core::model_catalog::ModelId::ParakeetTdt06BV3)
    }
}

fn decode_sha256(value: &str) -> Result<[u8; 32], String> {
    if value.len() != 64 {
        return Err("ModelHashMismatch: SHA-256 invalide".into());
    }
    let mut output = [0; 32];
    for (index, byte) in output.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|_| "ModelHashMismatch: SHA-256 invalide".to_owned())?;
    }
    Ok(output)
}

fn format_sha256(value: &[u8; 32]) -> String {
    value.iter().map(|byte| format!("{byte:02x}")).collect()
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

fn request_is_parakeet(request: &ImportRequest) -> bool {
    decode_sha256(
        whisper_core::model_registry::model(whisper_core::model_catalog::ModelId::ParakeetTdt06BV3)
            .artifact
            .sha256,
    )
    .is_ok_and(|sha| request.model_sha256 == sha)
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

fn validate_current_queued_gpu_candidate(
    request: &ImportRequest,
    candidate: Option<&whisper_core::gpu_inventory::GpuRuntimeIdentity>,
) -> Result<(), String> {
    if !request_is_parakeet(request) || request.config.compute != whisper_core::ComputeChoice::Gpu {
        return Ok(());
    }
    let candidate = candidate.ok_or_else(|| {
        "BackendMismatch: selected GPU has no current runtime identity proof".to_owned()
    })?;
    if candidate.generation != request.generation.get() {
        return Err("BackendMismatch: selected GPU identity belongs to a stale generation".into());
    }
    Ok(())
}

fn validate_queued_gpu_preflight(
    request: &ImportRequest,
    candidate: Option<&whisper_core::gpu_inventory::GpuRuntimeIdentity>,
    qualification: Option<&crate::archive::ArchiveQualificationKey>,
) -> Result<(), String> {
    validate_current_queued_gpu_candidate(request, candidate)?;
    if !request_is_parakeet(request) || request.config.compute != whisper_core::ComputeChoice::Gpu {
        return Ok(());
    }
    let key = qualification
        .ok_or_else(|| "ModelNotQualified: GPU Test key is not admitted".to_owned())?;
    if key.backend != "CUDA0"
        || key.inference_profile != whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS
    {
        return Err("ModelNotQualified: GPU Test key is not admitted".into());
    }
    Ok(())
}

fn archive_qualification_for_request(
    request: &ImportRequest,
    parakeet_paths: Option<&crate::parakeet_import::ParakeetWorkerPaths>,
    gpu_parakeet_paths: Option<&crate::parakeet_import::ParakeetWorkerPaths>,
    gpu_candidate: Option<&whisper_core::gpu_inventory::GpuRuntimeIdentity>,
) -> Option<crate::archive::ArchiveQualificationKey> {
    if !request_is_parakeet(request)
        || request.strategy
            == whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
    {
        return None;
    }
    let samples = request
        .source_samples
        .or_else(|| crate::archive::source_output_samples(Path::new(&request.source_path)).ok())?;
    let profile = if samples > whisper_core::parakeet_ipc::PARAKEET_MAX_AUDIO_SAMPLES as u64 {
        whisper_core::parakeet_windows::PROFILE_IMPORT_VAD
    } else {
        whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS
    };
    let gpu_requested = request.config.compute == whisper_core::ComputeChoice::Gpu
        || (request.config.compute == whisper_core::ComputeChoice::Auto && gpu_candidate.is_some());
    if gpu_requested {
        if let Some(key) = gpu_parakeet_paths
            .and_then(|paths| paths.gpu_qualification_key(profile).ok())
            .filter(gpu_qualification_candidate_allowed)
        {
            return Some(key);
        }
        if request.config.compute == whisper_core::ComputeChoice::Gpu {
            return None;
        }
    }
    parakeet_paths
        .and_then(|paths| paths.qualification_key(profile).ok())
        .or_else(|| {
            (profile == whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS
                && !parakeet_paths.is_some_and(|paths| paths.is_personal_runtime()))
            .then(crate::archive::ArchiveQualificationKey::parakeet_cpu_test)
        })
}

fn native_unified_route(
    request: &ImportRequest,
    cpu_paths: Option<&crate::parakeet_import::ParakeetWorkerPaths>,
    gpu_paths: Option<&crate::parakeet_import::ParakeetWorkerPaths>,
    candidate: Option<&whisper_core::gpu_inventory::GpuRuntimeIdentity>,
) -> Result<NativeUnifiedRoute, String> {
    if request.strategy != whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
        || !request_is_parakeet(request)
    {
        return Err(
            "NativeCandidateUnavailable: only BoundedWindowsV1 Parakeet requests are admitted"
                .into(),
        );
    }
    if !matches!(
        &request.config.language,
        whisper_core::LanguageChoice::Automatic
    ) {
        return Err("NativeCandidateUnavailable: Parakeet requires automatic language".into());
    }
    let record =
        whisper_core::model_registry::model(whisper_core::model_catalog::ModelId::ParakeetTdt06BV3);
    if request.model_sha256 != decode_sha256(record.artifact.sha256)? {
        return Err(
            "ModelHashMismatch: request model is not the registered Parakeet artifact".into(),
        );
    }

    let auto = request.config.compute == ComputeChoice::Auto;
    let gpu_candidate =
        candidate.filter(|identity| identity.generation == request.generation.get());
    let attempt_gpu = request.config.compute == ComputeChoice::Gpu
        || (auto && gpu_candidate.is_some() && gpu_paths.is_some());
    if attempt_gpu {
        let gpu_result = (|| {
            let identity = gpu_candidate.ok_or_else(|| {
                "BackendMismatch: selected GPU has no current generation identity".to_owned()
            })?;
            let paths = gpu_paths.ok_or_else(|| {
                "BackendUnavailable: personal Parakeet GPU worker is absent".to_owned()
            })?;
            if !paths.is_personal_runtime() {
                return Err(
                    "NativeCandidateUnavailable: GPU worker must use the personal runtime layout"
                        .into(),
                );
            }
            let expected_gpu = crate::parakeet_import::expected_gpu_for_candidate(identity)?;
            let profile = whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS;
            // This key verifies installed asset identity only. It is deliberately not
            // attached to BoundedWindowsV1 archives or treated as long-session proof.
            let asset_key = paths.gpu_qualification_key(profile)?;
            paths.verify_qualification(
                record.artifact.sha256,
                &asset_key.runtime_build,
                &asset_key.worker_build,
            )?;
            Ok(NativeUnifiedRoute {
                paths: paths.clone(),
                requested_compute: request.config.compute,
                compute: ComputeChoice::Gpu,
                expected_gpu: Some(expected_gpu),
                candidate: Some(identity.clone()),
                fallback_reason: None,
            })
        })();
        match gpu_result {
            Ok(route) => return Ok(route),
            Err(error) if auto => {
                let mut route = native_cpu_route(request, cpu_paths)?;
                route.fallback_reason = Some(format!("GPU pre-audio validation failed: {error}"));
                return Ok(route);
            }
            Err(error) => return Err(error),
        }
    }
    let mut route = native_cpu_route(request, cpu_paths)?;
    if auto {
        route.fallback_reason = Some(if candidate.is_some() {
            "GPU candidate was stale or the personal GPU route was unavailable before audio".into()
        } else {
            "No current GPU candidate was available before audio".into()
        });
    }
    Ok(route)
}

fn native_cpu_route(
    request: &ImportRequest,
    cpu_paths: Option<&crate::parakeet_import::ParakeetWorkerPaths>,
) -> Result<NativeUnifiedRoute, String> {
    let paths = cpu_paths
        .ok_or_else(|| "ModelNotQualified: personal Parakeet CPU worker is absent".to_owned())?;
    if !paths.is_personal_runtime() {
        return Err(
            "NativeCandidateUnavailable: CPU worker must use the personal runtime layout".into(),
        );
    }
    let record =
        whisper_core::model_registry::model(whisper_core::model_catalog::ModelId::ParakeetTdt06BV3);
    let asset_key =
        paths.qualification_key(whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS)?;
    paths.verify_qualification(
        record.artifact.sha256,
        &asset_key.runtime_build,
        &asset_key.worker_build,
    )?;
    Ok(NativeUnifiedRoute {
        paths: paths.clone(),
        requested_compute: request.config.compute,
        compute: ComputeChoice::Cpu,
        expected_gpu: None,
        candidate: None,
        fallback_reason: None,
    })
}

fn gpu_qualification_candidate_allowed(key: &crate::archive::ArchiveQualificationKey) -> bool {
    #[cfg(feature = "recette-hooks")]
    let test_injection = cfg!(feature = "l3-gpu-qualification")
        && option_env!("PARAKEET_L2_ACCEPTANCE_TEST_BUILD").is_some()
        && key == &crate::archive::ArchiveQualificationKey::parakeet_gpu_test();
    #[cfg(feature = "recette-hooks")]
    if test_injection {
        return true;
    }
    if key.is_personal_candidate() || key.is_registered_personal_import() {
        return key.backend == "CUDA0"
            && key.inference_profile
                == whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS;
    }
    let record =
        whisper_core::model_registry::model(whisper_core::model_catalog::ModelId::ParakeetTdt06BV3);
    let registered = whisper_core::model_registry::configuration_key(
        record,
        whisper_core::model_catalog::Backend::Gpu,
    );
    key.backend == "CUDA0"
        && key.artifact_revision == registered.artifact_revision
        && key.runtime_build == registered.runtime_build
        && key.worker_build == registered.worker_build
        && key.inference_profile == registered.inference_profile
}

pub(crate) fn gpu_evidence_matches(
    evidence: Option<&whisper_core::parakeet_ipc::GpuEvidence>,
    expected: Option<&whisper_core::parakeet_ipc::ExpectedGpu>,
    pid: u32,
) -> bool {
    let (Some(evidence), Some(expected)) = (evidence, expected) else {
        return false;
    };
    evidence.backend == "CUDA0"
        && evidence
            .pci_bus_id
            .eq_ignore_ascii_case(&expected.pci_bus_id)
        && evidence.uuid.eq_ignore_ascii_case(&expected.uuid)
        && evidence.pid == pid
        && evidence.asr_log_sha256.len() == 64
        && evidence
            .asr_log_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
}
const WORKER_IDLE_RETENTION: Duration = Duration::from_secs(5 * 60);

struct IdleWorker {
    transport: ChildWorkerTransport,
    compute: whisper_core::ComputeChoice,
    model_path: String,
    model_sha256: [u8; 32],
    expires_at: Instant,
}

impl IdleWorker {
    fn matches(&mut self, request: &LiveRequest, workers: &WorkerPaths) -> bool {
        let compute = if request.config.compute == whisper_core::ComputeChoice::Auto {
            if workers.gpu.is_file() {
                whisper_core::ComputeChoice::Gpu
            } else {
                whisper_core::ComputeChoice::Cpu
            }
        } else {
            request.config.compute
        };
        self.matches_identity(compute, &request.model_path, request.model_sha256)
    }

    fn matches_identity(
        &mut self,
        compute: whisper_core::ComputeChoice,
        model_path: &str,
        model_sha256: [u8; 32],
    ) -> bool {
        self.expires_at > Instant::now()
            && self.compute == compute
            && self.model_path == model_path
            && self.model_sha256 == model_sha256
            && self
                .transport
                .try_wait()
                .is_ok_and(|status| status.is_none())
    }
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
    fn release_unified_terminal_state(&mut self, job_id: JobId, generation: Generation) {
        let session = whisper_core::parakeet_scheduler::SchedulerSession { job_id, generation };
        if self.unified_sessions.contains_key(&session) {
            return;
        }
        self.document_drains
            .retain(|drain| (drain.job_id, drain.generation) != (job_id, generation));
        self.unified_archives.remove(&session);
        self.unified_resume_checkpoints.remove(&session);
        self.unified_capture_sessions.remove(&session);
        self.unified_publication_gates.remove(&session);
        self.native_unified_routes.remove(&session);
        self.gpu_candidates.remove(&(job_id, generation.get()));
        self.parakeet_pending_events
            .retain(|event| pending_unified_event_identity(event) != Some((job_id, generation)));
        let executor_reaped = self
            .unified_executor
            .as_ref()
            .is_none_or(|executor| executor.thread.is_none() && !executor.in_flight);
        if self.unified_sessions.is_empty()
            && self.parakeet_scheduler.active_sessions() == 0
            && executor_reaped
        {
            self.unified_executor = None;
            self.native_executor_route = None;
        }
    }

    fn unified_admission_full(&self) -> bool {
        let mut parent_sessions = std::collections::HashSet::new();
        parent_sessions.extend(self.unified_sessions.keys().copied());
        parent_sessions.extend(self.unified_archives.keys().copied());
        parent_sessions.extend(self.unified_resume_checkpoints.keys().copied());
        parent_sessions.extend(self.unified_publication_gates.keys().copied());
        parent_sessions.extend(self.native_unified_routes.keys().copied());
        parent_sessions.extend(self.document_drains.iter().map(|drain| {
            whisper_core::parakeet_scheduler::SchedulerSession {
                job_id: drain.job_id,
                generation: drain.generation,
            }
        }));
        parent_sessions.len() >= 2 || self.parakeet_scheduler.active_sessions() >= 2
    }

    fn unified_admission_full_for(
        &self,
        continuing: whisper_core::parakeet_scheduler::SchedulerSession,
    ) -> bool {
        let mut parent_sessions = std::collections::HashSet::new();
        parent_sessions.extend(
            self.unified_sessions
                .keys()
                .filter(|session| **session != continuing)
                .copied(),
        );
        parent_sessions.extend(
            self.unified_archives
                .keys()
                .filter(|session| **session != continuing)
                .copied(),
        );
        parent_sessions.extend(
            self.unified_resume_checkpoints
                .keys()
                .filter(|session| **session != continuing)
                .copied(),
        );
        parent_sessions.extend(
            self.unified_publication_gates
                .keys()
                .filter(|session| **session != continuing)
                .copied(),
        );
        parent_sessions.extend(
            self.native_unified_routes
                .keys()
                .filter(|session| **session != continuing)
                .copied(),
        );
        parent_sessions.extend(self.document_drains.iter().filter_map(|drain| {
            let session = whisper_core::parakeet_scheduler::SchedulerSession {
                job_id: drain.job_id,
                generation: drain.generation,
            };
            (session != continuing).then_some(session)
        }));
        parent_sessions.len() >= 2
            || (self.parakeet_scheduler.active_sessions() >= 2
                && !self.unified_sessions.contains_key(&continuing))
    }

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

    fn poll_unified_document(
        &mut self,
    ) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
        use whisper_core::ports::ParakeetDocumentPort;
        let mut drain = self.document_drains.pop_front()?;
        if drain.ready_pending {
            drain.ready_pending = false;
            let job_id = drain.job_id;
            let generation = drain.generation;
            self.document_drains.push_front(drain);
            return Some(Ok(ImportEvent::Ready {
                job_id,
                generation,
                backend: "controlled-test-worker".into(),
            }));
        }
        if drain.next_sequence <= drain.revision {
            let page_result = drain.document.read_page(
                drain.job_id,
                drain.generation,
                drain.revision,
                drain.next_sequence,
                1,
            );
            return Some(match page_result {
                Ok(mut page) if page.entries.len() == 1 => {
                    drain.next_sequence += 1;
                    self.document_drains.push_front(drain);
                    Ok(ImportEvent::Segment(page.entries.remove(0).segment))
                }
                Ok(_) => Err((
                    drain.job_id,
                    drain.generation,
                    "StorageCorrupt: controlled page did not contain exactly one segment".into(),
                )),
                Err(error) => Err((
                    drain.job_id,
                    drain.generation,
                    format!("Document: {error:?}"),
                )),
            });
        }
        if drain.progress_pending {
            drain.progress_pending = false;
            let job_id = drain.job_id;
            let generation = drain.generation;
            let total_samples = drain.total_samples;
            self.document_drains.push_front(drain);
            return Some(Ok(ImportEvent::Progress {
                job_id,
                generation,
                completed_samples: total_samples,
                total_samples,
            }));
        }
        Some(Ok(ImportEvent::End {
            job_id: drain.job_id,
            generation: drain.generation,
            last_sequence: drain.revision,
            last_offset: drain.total_samples,
        }))
    }

    fn poll_unified_sessions(
        &mut self,
    ) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
        if self.unified_sessions.is_empty() {
            return None;
        }
        let next_native_route = self
            .unified_sessions
            .values()
            .find_map(|state| state.native_route.as_ref());
        let route_changed = self
            .native_executor_route
            .as_ref()
            .zip(next_native_route)
            .is_some_and(|(current, next)| !current.same_worker(next));
        if route_changed {
            let executor_reaped = self
                .unified_executor
                .as_ref()
                .is_none_or(|executor| executor.thread.is_none() && !executor.in_flight);
            if !executor_reaped || self.parakeet_scheduler.in_flight().is_some() {
                let session = self.unified_sessions.keys().next().copied()?;
                return Some(Err((
                    session.job_id,
                    session.generation,
                    "NativeRouteBusy: a different backend cannot replace a live executor incarnation".into(),
                )));
            }
            self.unified_executor = None;
            self.native_executor_route = None;
        }
        if self.unified_executor.is_none() {
            #[cfg(test)]
            let controlled_worker = self.controlled_transcriber.take();
            #[cfg(not(test))]
            let controlled_worker: Option<
                Box<dyn crate::parakeet_pipeline::WindowTranscriber>,
            > = None;
            let native_route = self
                .unified_sessions
                .values()
                .find_map(|state| state.native_route.clone());
            let start_result = if let Some(worker) = controlled_worker {
                #[cfg(test)]
                {
                    match self.controlled_child_observer.clone() {
                        Some(observer) if self.controlled_ready_gate.is_some() => {
                            UnifiedInferenceExecutor::start_observed_with_ready_gate(
                                worker,
                                observer,
                                self.controlled_ready_gate
                                    .take()
                                    .expect("ready gate checked"),
                            )
                        }
                        Some(observer) => {
                            UnifiedInferenceExecutor::start_observed(worker, observer)
                        }
                        None => UnifiedInferenceExecutor::start(worker),
                    }
                }
                #[cfg(not(test))]
                {
                    UnifiedInferenceExecutor::start(worker)
                }
            } else if self.native_unified_candidate_enabled {
                match native_route {
                    Some(route) => {
                        self.native_executor_route = Some(route.clone());
                        let worker = NativeParakeetWindowTranscriber::new(route);
                        #[cfg(test)]
                        let worker = match self.native_harness_observer.clone() {
                            Some(observer) => worker.with_harness_observer(observer),
                            None => worker,
                        };
                        UnifiedInferenceExecutor::start(Box::new(worker))
                    }
                    None => Err(
                        "NativeCandidateUnavailable: no validated route for unified session".into(),
                    ),
                }
            } else {
                Err("ModelNotQualified: unified inference executor is unavailable".into())
            };
            match start_result {
                Ok(mut executor) => {
                    #[cfg(test)]
                    {
                        executor.disconnect_result_for = self.controlled_disconnect_session;
                        executor.panic_during_transcription_for = self.controlled_panic_session;
                        if executor.disconnect_result_for.is_some()
                            || executor.panic_during_transcription_for.is_some()
                        {
                            if let Err(error) =
                                executor.stop_and_reap().and_then(|()| executor.restart())
                            {
                                return Some(Err((JobId(0), Generation::first(), error)));
                            }
                        }
                    }
                    self.unified_executor = Some(executor);
                }
                Err(error) => {
                    let session = self.unified_sessions.keys().next().copied()?;
                    return Some(Err((session.job_id, session.generation, error)));
                }
            }
        }
        let Some(executor) = self.unified_executor.as_mut() else {
            let identity = self
                .unified_sessions
                .keys()
                .next()
                .map(|session| (session.job_id, session.generation))?;
            return Some(Err((
                identity.0,
                identity.1,
                "ModelNotQualified: unified inference executor is unavailable".into(),
            )));
        };
        if executor.thread.is_none()
            && let Err(error) = executor.restart()
        {
            let session = self.unified_sessions.keys().next().copied()?;
            return Some(Err((
                session.job_id,
                session.generation,
                format!("InferenceRestart: {error}"),
            )));
        }
        let mut pump_error = None;
        let mut progress_updates = Vec::new();
        let mut preview_updates = Vec::new();
        let mut gain_updates = Vec::new();
        for (session, state) in &mut self.unified_sessions {
            if !state.source.is_capture() || state.source_closed {
                continue;
            }
            match state.source.pump() {
                Ok(acknowledgements) => {
                    if let Some(boundary) =
                        acknowledgements.iter().map(|ack| ack.written_samples).max()
                    {
                        let previous_samples = state.available_samples;
                        let previous_durable_samples = state.durable_samples;
                        state.available_samples = state.available_samples.max(boundary);
                        state.durable_samples = state
                            .durable_samples
                            .max(
                                acknowledgements
                                    .iter()
                                    .map(|ack| ack.flushed_samples)
                                    .max()
                                    .unwrap_or(0),
                            )
                            .min(state.available_samples);
                        if state.available_samples > previous_samples
                            || state.durable_samples > previous_durable_samples
                        {
                            progress_updates.push((
                                *session,
                                state.available_samples,
                                state.durable_samples,
                            ));
                        }
                        if boundary >= 4 * 16_000 && boundary > state.preview_sequence {
                            state.preview_sequence = boundary;
                            preview_updates.push((*session, boundary));
                        }
                        gain_updates.extend(
                            state
                                .source
                                .take_gain_events()
                                .into_iter()
                                .map(|gain| (*session, gain)),
                        );
                    }
                }
                Err(error) => {
                    state.failure = Some(error.clone());
                    let _ = self.parakeet_scheduler.abort_session(*session);
                    pump_error = Some((*session, error));
                }
            }
        }
        let _ = pump_error;
        self.parakeet_pending_events
            .extend(gain_updates.into_iter().map(|(session, gain)| {
                ImportEvent::MicrophoneGainApplied {
                    job_id: session.job_id,
                    generation: session.generation,
                    target_identity: gain.target_identity,
                    gain_db: gain.gain_db,
                    gain_revision: gain.gain_revision,
                    first_frame: gain.first_frame,
                }
            }));
        for (session, captured_samples, audio_durable_samples) in progress_updates {
            #[cfg(test)]
            if let Some(observer) = &self.native_harness_observer {
                observer.record(NativeQualificationEventKind::CaptureAcknowledged {
                    session,
                    written_samples: captured_samples,
                    durable_samples: audio_durable_samples,
                });
            }
            self.parakeet_pending_events.retain(|event| {
                !matches!(
                    event,
                    ImportEvent::ParakeetCaptureProgress {
                        job_id,
                        generation,
                        ..
                    } if (*job_id, *generation) == (session.job_id, session.generation)
                )
            });
            self.parakeet_pending_events
                .push_back(ImportEvent::ParakeetCaptureProgress {
                    job_id: session.job_id,
                    generation: session.generation,
                    // Unified capture progress is correlated by this event's
                    // JobId and generation; the legacy session id is unused.
                    session_id: 0,
                    captured_samples,
                    audio_durable_samples,
                });
        }
        for (session, boundary) in preview_updates {
            if let Err(error) = self.parakeet_scheduler.queue_preview(session, boundary) {
                return Some(Err((
                    session.job_id,
                    session.generation,
                    format!("SchedulerPreview: {error:?}"),
                )));
            }
        }
        match executor.try_result() {
            Ok(Some(result)) => {
                let session = result.call.session;
                let Some(mut state) = self.unified_sessions.remove(&session) else {
                    return Some(Err((
                        session.job_id,
                        session.generation,
                        "SchedulerResponse: result belongs to an unknown session".into(),
                    )));
                };
                if state.pending_call != Some(result.call)
                    || self.parakeet_scheduler.in_flight() != Some(result.call)
                {
                    self.unified_sessions.insert(session, state);
                    return Some(Err((
                        session.job_id,
                        session.generation,
                        "SchedulerResponse: stale inference result".into(),
                    )));
                }
                state.pending_call = None;
                if let Some(error) = state.failure.take() {
                    let _ = self.parakeet_scheduler.fail_call(result.call);
                    if let Err(reap_error) = executor.stop_and_reap() {
                        state.failure = Some(error.clone());
                        self.unified_sessions.insert(session, state);
                        return Some(Err((
                            session.job_id,
                            session.generation,
                            format!("{error}; InferenceReap: {reap_error}"),
                        )));
                    }
                    if let Err(reap_error) = self.parakeet_scheduler.confirm_child_exited() {
                        state.failure = Some(error.clone());
                        self.unified_sessions.insert(session, state);
                        return Some(Err((
                            session.job_id,
                            session.generation,
                            format!("{error}; SchedulerReap: {reap_error:?}"),
                        )));
                    }
                    if self.parakeet_scheduler.active_sessions() > 0
                        && let Err(restart_error) = executor.restart()
                    {
                        state.failure = Some(error.clone());
                        self.unified_sessions.insert(session, state);
                        return Some(Err((
                            session.job_id,
                            session.generation,
                            format!("{error}; InferenceRestart: {restart_error}"),
                        )));
                    }
                    return Some(Err((session.job_id, session.generation, error)));
                }
                let words = match result.words {
                    Ok(words) => words,
                    Err(error) => {
                        let _ = self.parakeet_scheduler.fail_call(result.call);
                        let _ = self.parakeet_scheduler.mark_terminal(session);
                        #[cfg(test)]
                        let skip_recycle = self.controlled_skip_child_recycle;
                        #[cfg(not(test))]
                        let skip_recycle = false;
                        if !skip_recycle {
                            if let Err(reap_error) = executor.stop_and_reap() {
                                self.unified_sessions.insert(session, state);
                                return Some(Err((
                                    session.job_id,
                                    session.generation,
                                    format!("{error}; InferenceReap: {reap_error}"),
                                )));
                            }
                        }
                        if let Err(reap_error) = self.parakeet_scheduler.confirm_child_exited() {
                            self.unified_sessions.insert(session, state);
                            return Some(Err((
                                session.job_id,
                                session.generation,
                                format!("{error}; SchedulerReap: {reap_error:?}"),
                            )));
                        }
                        if self.parakeet_scheduler.active_sessions() > 0
                            && let Err(restart_error) = executor.restart()
                        {
                            self.unified_sessions.insert(session, state);
                            return Some(Err((
                                session.job_id,
                                session.generation,
                                format!("{error}; InferenceRestart: {restart_error}"),
                            )));
                        }
                        return Some(Err((session.job_id, session.generation, error)));
                    }
                };
                let applied = match result.call.work {
                    whisper_core::parakeet_scheduler::ScheduledWork::Final { .. } => state
                        .finalizer
                        .commit_prepared(&mut state.document, result.instance_id, &words),
                    whisper_core::parakeet_scheduler::ScheduledWork::Preview { .. } => {
                        match (
                            state.pending_window.take(),
                            state.pending_available_samples.take(),
                        ) {
                            (Some(window), Some(boundary)) => {
                                crate::parakeet_pipeline::commit_preview(
                                    &mut state.document,
                                    window,
                                    boundary,
                                    result.instance_id,
                                    session.job_id,
                                    session.generation,
                                    &words,
                                )
                                .map(|_| {
                                    crate::parakeet_pipeline::FinalizationStep::WaitingForAudio {
                                        processed_samples: state.finalizer.processed_samples(),
                                    }
                                })
                            }
                            _ => {
                                Err("Pipeline: preview result has no matching prepared window"
                                    .into())
                            }
                        }
                    }
                };
                let valid_step = matches!(
                    result.call.work,
                    whisper_core::parakeet_scheduler::ScheduledWork::Preview { .. }
                ) || matches!(
                    &applied,
                    Ok(crate::parakeet_pipeline::FinalizationStep::WindowProcessed { .. })
                );
                match applied {
                    Ok(_) if valid_step => {
                        if let Err(error) = self.parakeet_scheduler.complete(result.call) {
                            self.unified_sessions.insert(session, state);
                            return Some(Err((
                                session.job_id,
                                session.generation,
                                format!("SchedulerComplete: {error:?}"),
                            )));
                        }
                        self.unified_sessions.insert(session, state);
                        return None;
                    }
                    Ok(_) => {
                        self.unified_sessions.insert(session, state);
                        return Some(Err((
                            session.job_id,
                            session.generation,
                            "Pipeline: inference response did not commit a final window".into(),
                        )));
                    }
                    Err(error) => {
                        let _ = self.parakeet_scheduler.fail_call(result.call);
                        let _ = self.parakeet_scheduler.mark_terminal(session);
                        if let Err(reap_error) = executor.stop_and_reap() {
                            self.unified_sessions.insert(session, state);
                            return Some(Err((
                                session.job_id,
                                session.generation,
                                format!("{error}; InferenceReap: {reap_error}"),
                            )));
                        }
                        if let Err(reap_error) = self.parakeet_scheduler.confirm_child_exited() {
                            self.unified_sessions.insert(session, state);
                            return Some(Err((
                                session.job_id,
                                session.generation,
                                format!("{error}; SchedulerReap: {reap_error:?}"),
                            )));
                        }
                        if self.parakeet_scheduler.active_sessions() > 0
                            && let Err(restart_error) = executor.restart()
                        {
                            self.unified_sessions.insert(session, state);
                            return Some(Err((
                                session.job_id,
                                session.generation,
                                format!("{error}; InferenceRestart: {restart_error}"),
                            )));
                        }
                        return Some(Err((session.job_id, session.generation, error)));
                    }
                }
            }
            Ok(None) => {}
            Err(error) => {
                let Some(call) = self.parakeet_scheduler.in_flight() else {
                    let identity = self.unified_sessions.keys().next().copied().unwrap_or(
                        whisper_core::parakeet_scheduler::SchedulerSession {
                            job_id: JobId(0),
                            generation: Generation::first(),
                        },
                    );
                    return Some(Err((
                        identity.job_id,
                        identity.generation,
                        format!("{error}; SchedulerCallMissing"),
                    )));
                };
                let session = call.session;
                let exited_instance = executor.thread.is_some();
                let reap = executor.reap_disconnected();
                if !exited_instance {
                    return Some(Err((
                        session.job_id,
                        session.generation,
                        format!("{error}; InferenceReap: disconnected instance exit is unproven"),
                    )));
                }
                if let Err(reap_error) = reap {
                    let mut failure = format!("{error}; {reap_error}");
                    let sessions: Vec<_> = self.unified_sessions.keys().copied().collect();
                    for (other, state) in &mut self.unified_sessions {
                        if state.source.is_capture() && !state.source_closed {
                            match state.source.stop_capture() {
                                Ok(Some((total_samples, acknowledgements))) => {
                                    state.available_samples = total_samples;
                                    state.durable_samples = state
                                        .durable_samples
                                        .max(
                                            acknowledgements
                                                .iter()
                                                .map(|ack| ack.flushed_samples)
                                                .max()
                                                .unwrap_or(0),
                                        )
                                        .min(total_samples);
                                    state.source_closed = true;
                                }
                                Ok(None) => {}
                                Err(stop_error) => failure.push_str(&format!(
                                    "; capture {:?}/{:?} stop: {stop_error}",
                                    other.job_id, other.generation
                                )),
                            }
                        }
                    }
                    if let Err(scheduler_error) = self.parakeet_scheduler.fail_call(call) {
                        failure.push_str(&format!("; SchedulerCall: {scheduler_error:?}"));
                    }
                    for other in &sessions {
                        if let Err(scheduler_error) = self.parakeet_scheduler.abort_session(*other)
                        {
                            failure.push_str(&format!("; SchedulerAbort: {scheduler_error:?}"));
                        }
                    }
                    if let Err(scheduler_error) = self.parakeet_scheduler.confirm_child_exited() {
                        failure.push_str(&format!("; SchedulerReap: {scheduler_error:?}"));
                    }
                    for other in sessions {
                        self.unified_sessions.remove(&other);
                        self.release_unified_terminal_state(other.job_id, other.generation);
                        if other != session {
                            self.parakeet_pending_events.push_back(ImportEvent::Failed {
                                job_id: other.job_id,
                                generation: other.generation,
                                message: format!(
                                    "InferenceWorkerLost: peer session terminated after worker panic while processing {:?}/{:?}",
                                    session.job_id, session.generation
                                ),
                            });
                        }
                    }
                    return Some(Err((session.job_id, session.generation, failure)));
                }
                let mut failure = error;
                if let Err(scheduler_error) = self.parakeet_scheduler.fail_call(call) {
                    failure.push_str(&format!("; SchedulerCall: {scheduler_error:?}"));
                    return Some(Err((session.job_id, session.generation, failure)));
                }
                if let Err(scheduler_error) = self.parakeet_scheduler.mark_terminal(session) {
                    failure.push_str(&format!("; SchedulerTerminal: {scheduler_error:?}"));
                    return Some(Err((session.job_id, session.generation, failure)));
                }
                if let Err(scheduler_error) = self.parakeet_scheduler.confirm_child_exited() {
                    failure.push_str(&format!("; SchedulerReap: {scheduler_error:?}"));
                    return Some(Err((session.job_id, session.generation, failure)));
                }
                self.unified_sessions.remove(&session);
                if self.parakeet_scheduler.active_sessions() > 0 {
                    if let Err(restart_error) = executor.restart() {
                        failure.push_str(&format!("; InferenceRestart: {restart_error}"));
                    }
                }
                return Some(Err((session.job_id, session.generation, failure)));
            }
        }
        if executor.in_flight {
            return None;
        }
        if let Some(session) = self
            .unified_sessions
            .iter()
            .find_map(|(session, state)| state.failure.as_ref().map(|_| *session))
        {
            let mut state = self.unified_sessions.remove(&session)?;
            let error = state.failure.take().unwrap_or_else(|| "CaptureLost".into());
            if let Err(reap_error) = executor.stop_and_reap() {
                state.failure = Some(error.clone());
                self.unified_sessions.insert(session, state);
                return Some(Err((
                    session.job_id,
                    session.generation,
                    format!("{error}; InferenceReap: {reap_error}"),
                )));
            }
            if let Err(reap_error) = self.parakeet_scheduler.confirm_child_exited() {
                state.failure = Some(error.clone());
                self.unified_sessions.insert(session, state);
                return Some(Err((
                    session.job_id,
                    session.generation,
                    format!("{error}; SchedulerReap: {reap_error:?}"),
                )));
            }
            if self.parakeet_scheduler.active_sessions() > 0
                && let Err(restart_error) = executor.restart()
            {
                state.failure = Some(error.clone());
                self.unified_sessions.insert(session, state);
                return Some(Err((
                    session.job_id,
                    session.generation,
                    format!("{error}; InferenceRestart: {restart_error}"),
                )));
            }
            return Some(Err((session.job_id, session.generation, error)));
        }
        let finished_session = self.unified_sessions.iter().find_map(|(session, state)| {
            (state.source_closed
                && state.pending_call.is_none()
                && state.pending_prepared.is_none()
                && self
                    .parakeet_scheduler
                    .in_flight()
                    .is_none_or(|call| call.session != *session)
                && state.finalizer.processed_samples() == state.available_samples)
                .then_some(*session)
        });
        if let Some(session) = finished_session {
            let mut state = self.unified_sessions.remove(&session)?;
            #[cfg(test)]
            let force_finish_failure = self.controlled_finish_failure_session == Some(session);
            #[cfg(not(test))]
            let force_finish_failure = false;
            let result = if force_finish_failure {
                Err("TestOnly: injected document finalization failure".to_owned())
            } else {
                state.finalizer.finish_document(
                    &mut state.document,
                    state.available_samples,
                    state.source_closed,
                )
            };
            let result = match result {
                Ok(crate::parakeet_pipeline::FinalizationStep::Complete(result)) => result,
                Ok(_) => {
                    let error = "Pipeline: closed document did not finalize".to_owned();
                    state.failure = Some(error);
                    let _ = self.parakeet_scheduler.abort_session(session);
                    self.unified_sessions.insert(session, state);
                    return None;
                }
                Err(error) => {
                    state.failure = Some(error);
                    let _ = self.parakeet_scheduler.abort_session(session);
                    self.unified_sessions.insert(session, state);
                    return None;
                }
            };
            if let Err(error) = self.parakeet_scheduler.mark_terminal(session) {
                self.unified_sessions.insert(session, state);
                return Some(Err((
                    session.job_id,
                    session.generation,
                    format!("SchedulerTerminal: {error:?}"),
                )));
            }
            if let Err(error) = executor.stop_and_reap() {
                self.unified_sessions.insert(session, state);
                return Some(Err((
                    session.job_id,
                    session.generation,
                    format!("InferenceReap: {error}"),
                )));
            }
            if let Err(error) = self.parakeet_scheduler.confirm_child_exited() {
                self.unified_sessions.insert(session, state);
                return Some(Err((
                    session.job_id,
                    session.generation,
                    format!("SchedulerReap: {error:?}"),
                )));
            }
            if self.parakeet_scheduler.active_sessions() > 0 {
                #[cfg(test)]
                if let Some(gate) = self.controlled_ready_gate.take() {
                    executor.ready_gate = Some(gate);
                }
                if let Err(error) = executor.restart() {
                    self.unified_sessions.insert(session, state);
                    return Some(Err((
                        session.job_id,
                        session.generation,
                        format!("InferenceRestart: {error}"),
                    )));
                }
            }
            let ready_pending = !state.backend_ready_reported;
            self.document_drains.push_back(UnifiedDocumentDrain {
                document: state.document,
                job_id: session.job_id,
                generation: session.generation,
                revision: result.document_revision,
                total_samples: result.total_samples,
                next_sequence: state.document_drain_start_sequence,
                ready_pending,
                progress_pending: true,
            });
            return None;
        }
        for (session, state) in &self.unified_sessions {
            let remaining = state
                .available_samples
                .saturating_sub(state.finalizer.processed_samples());
            if state.finalizer.processed_samples() < state.available_samples
                && (state.source_closed || remaining >= 120 * 16_000 + 2 * 200 * 16)
            {
                let _ = self
                    .parakeet_scheduler
                    .queue_final(*session, state.finalizer.next_window_sequence());
            }
        }
        let call = if let Some(call) = self.parakeet_scheduler.in_flight() {
            call
        } else {
            match self.parakeet_scheduler.next_call() {
                Ok(Some(call)) => call,
                Ok(None) => return None,
                Err(error) => {
                    let identity = self.unified_sessions.keys().next().copied()?;
                    return Some(Err((
                        identity.job_id,
                        identity.generation,
                        format!("SchedulerSelect: {error:?}"),
                    )));
                }
            }
        };
        #[cfg(test)]
        if self.controlled_hold_reserved_call_once {
            self.controlled_hold_reserved_call_once = false;
            return None;
        }
        let Some(mut state) = self.unified_sessions.remove(&call.session) else {
            let _ = self.parakeet_scheduler.cancel_reservation(call);
            return Some(Err((
                call.session.job_id,
                call.session.generation,
                "SchedulerSelection: selected session is missing".into(),
            )));
        };
        let prepared = if let Some((prepared_call, window, pcm, is_final)) =
            state.pending_prepared.take()
        {
            if prepared_call != call {
                self.unified_sessions.insert(call.session, state);
                return Some(Err((
                    call.session.job_id,
                    call.session.generation,
                    "SchedulerPreparation: retained call identity changed".into(),
                )));
            }
            Ok(Some((window, pcm, is_final)))
        } else {
            match call.work {
                whisper_core::parakeet_scheduler::ScheduledWork::Final { .. } => state
                    .finalizer
                    .prepare_next(
                        &mut state.source,
                        state.available_samples,
                        state.source_closed,
                    )
                    .map(|prepared| match prepared {
                        crate::parakeet_pipeline::PreparedFinalWindow::Ready { window, pcm } => {
                            Some((window, pcm, true))
                        }
                        crate::parakeet_pipeline::PreparedFinalWindow::WaitingForAudio {
                            ..
                        }
                        | crate::parakeet_pipeline::PreparedFinalWindow::Complete => None,
                    }),
                whisper_core::parakeet_scheduler::ScheduledWork::Preview { sequence } => {
                    crate::parakeet_pipeline::prepare_preview(&mut state.source, sequence, sequence)
                        .map(|prepared| Some((prepared.window, prepared.pcm, false)))
                }
            }
        };
        match prepared {
            Ok(Some((window, pcm, true))) => {
                match executor.is_ready() {
                    Ok(true) if state.native_route.is_some() && !state.backend_ready_reported => {
                        state.backend_ready_reported = true;
                        let backend = state
                            .native_route
                            .as_ref()
                            .expect("native route guard")
                            .backend_label();
                        state.pending_prepared = Some((call, window, pcm, true));
                        self.unified_sessions.insert(call.session, state);
                        return Some(Ok(ImportEvent::Ready {
                            job_id: call.session.job_id,
                            generation: call.session.generation,
                            backend,
                        }));
                    }
                    Ok(true) => {}
                    Ok(false) => {
                        state.pending_prepared = Some((call, window, pcm, true));
                        self.unified_sessions.insert(call.session, state);
                        return None;
                    }
                    Err(error) => {
                        let cpu_paths = self.parakeet_paths.clone();
                        match retry_native_gpu_startup_on_cpu(
                            &mut self.parakeet_scheduler,
                            &mut self.unified_sessions,
                            &mut self.unified_archives,
                            &mut self.native_executor_route,
                            executor,
                            &mut state,
                            call,
                            &error,
                            |request, reason| {
                                let mut route = native_cpu_route(request, cpu_paths.as_ref())?;
                                route.fallback_reason = Some(reason.to_owned());
                                Ok(route)
                            },
                            |route| Box::new(NativeParakeetWindowTranscriber::new(route)),
                        ) {
                            Ok(Some(retry)) => {
                                for peer in retry.failed_manual_gpu {
                                    self.release_unified_terminal_state(
                                        peer.session.job_id,
                                        peer.session.generation,
                                    );
                                    self.parakeet_pending_events
                                        .push_back(manual_gpu_failure_event(peer));
                                }
                                state.pending_prepared = Some((retry.call, window, pcm, true));
                                self.unified_sessions.insert(call.session, state);
                                return None;
                            }
                            Err(fallback_error) => {
                                let _ = self.parakeet_scheduler.cancel_reservation(call);
                                let _ = self.parakeet_scheduler.mark_terminal(call.session);
                                state.failure =
                                    Some(format!("{error}; AutoFallback: {fallback_error}"));
                                self.unified_sessions.insert(call.session, state);
                                return Some(Err((
                                    call.session.job_id,
                                    call.session.generation,
                                    format!("{error}; AutoFallback: {fallback_error}"),
                                )));
                            }
                            Ok(None) => {}
                        }
                        let _ = self.parakeet_scheduler.cancel_reservation(call);
                        let _ = self.parakeet_scheduler.mark_terminal(call.session);
                        state.failure = Some(error.clone());
                        self.unified_sessions.insert(call.session, state);
                        return Some(Err((call.session.job_id, call.session.generation, error)));
                    }
                }
                if let Err(error) = self.parakeet_scheduler.begin_call(call) {
                    let _ = self.parakeet_scheduler.cancel_reservation(call);
                    state.failure = Some(format!("SchedulerBegin: {error:?}"));
                    let _ = self.parakeet_scheduler.abort_session(call.session);
                    self.unified_sessions.insert(call.session, state);
                    return Some(Err((
                        call.session.job_id,
                        call.session.generation,
                        format!("SchedulerBegin: {error:?}"),
                    )));
                }
                match executor.dispatch(UnifiedInferenceCommand {
                    call,
                    window,
                    pcm,
                    scratch_directory: state.inference_directory.clone(),
                }) {
                    Ok(()) => {
                        state.pending_call = Some(call);
                        self.unified_sessions.insert(call.session, state);
                        None
                    }
                    Err(error) => {
                        let _ = self.parakeet_scheduler.fail_call(call);
                        let _ = self.parakeet_scheduler.mark_terminal(call.session);
                        state.failure = Some(error.clone());
                        self.unified_sessions.insert(call.session, state);
                        Some(Err((call.session.job_id, call.session.generation, error)))
                    }
                }
            }
            Ok(Some((window, pcm, false))) => {
                match executor.is_ready() {
                    Ok(true) if state.native_route.is_some() && !state.backend_ready_reported => {
                        state.backend_ready_reported = true;
                        let backend = state
                            .native_route
                            .as_ref()
                            .expect("native route guard")
                            .backend_label();
                        state.pending_prepared = Some((call, window, pcm, false));
                        self.unified_sessions.insert(call.session, state);
                        return Some(Ok(ImportEvent::Ready {
                            job_id: call.session.job_id,
                            generation: call.session.generation,
                            backend,
                        }));
                    }
                    Ok(true) => {}
                    Ok(false) => {
                        state.pending_prepared = Some((call, window, pcm, false));
                        self.unified_sessions.insert(call.session, state);
                        return None;
                    }
                    Err(error) => {
                        let cpu_paths = self.parakeet_paths.clone();
                        match retry_native_gpu_startup_on_cpu(
                            &mut self.parakeet_scheduler,
                            &mut self.unified_sessions,
                            &mut self.unified_archives,
                            &mut self.native_executor_route,
                            executor,
                            &mut state,
                            call,
                            &error,
                            |request, reason| {
                                let mut route = native_cpu_route(request, cpu_paths.as_ref())?;
                                route.fallback_reason = Some(reason.to_owned());
                                Ok(route)
                            },
                            |route| Box::new(NativeParakeetWindowTranscriber::new(route)),
                        ) {
                            Ok(Some(retry)) => {
                                for peer in retry.failed_manual_gpu {
                                    self.release_unified_terminal_state(
                                        peer.session.job_id,
                                        peer.session.generation,
                                    );
                                    self.parakeet_pending_events
                                        .push_back(manual_gpu_failure_event(peer));
                                }
                                state.pending_prepared = Some((retry.call, window, pcm, false));
                                self.unified_sessions.insert(call.session, state);
                                return None;
                            }
                            Err(fallback_error) => {
                                let _ = self.parakeet_scheduler.cancel_reservation(call);
                                let _ = self.parakeet_scheduler.mark_terminal(call.session);
                                state.failure =
                                    Some(format!("{error}; AutoFallback: {fallback_error}"));
                                self.unified_sessions.insert(call.session, state);
                                return Some(Err((
                                    call.session.job_id,
                                    call.session.generation,
                                    format!("{error}; AutoFallback: {fallback_error}"),
                                )));
                            }
                            Ok(None) => {}
                        }
                        let _ = self.parakeet_scheduler.cancel_reservation(call);
                        let _ = self.parakeet_scheduler.mark_terminal(call.session);
                        state.failure = Some(error.clone());
                        self.unified_sessions.insert(call.session, state);
                        return Some(Err((call.session.job_id, call.session.generation, error)));
                    }
                }
                state.pending_window = Some(window);
                state.pending_available_samples = Some(window.central_end_sample);
                if let Err(error) = self.parakeet_scheduler.begin_call(call) {
                    let _ = self.parakeet_scheduler.cancel_reservation(call);
                    state.failure = Some(format!("SchedulerBegin: {error:?}"));
                    let _ = self.parakeet_scheduler.abort_session(call.session);
                    self.unified_sessions.insert(call.session, state);
                    return Some(Err((
                        call.session.job_id,
                        call.session.generation,
                        format!("SchedulerBegin: {error:?}"),
                    )));
                }
                match executor.dispatch(UnifiedInferenceCommand {
                    call,
                    window,
                    pcm,
                    scratch_directory: state.inference_directory.clone(),
                }) {
                    Ok(()) => {
                        state.pending_call = Some(call);
                        self.unified_sessions.insert(call.session, state);
                        None
                    }
                    Err(error) => {
                        let _ = self.parakeet_scheduler.fail_call(call);
                        let _ = self.parakeet_scheduler.mark_terminal(call.session);
                        state.failure = Some(error.clone());
                        self.unified_sessions.insert(call.session, state);
                        Some(Err((call.session.job_id, call.session.generation, error)))
                    }
                }
            }
            Ok(None) => {
                let _ = self.parakeet_scheduler.cancel_reservation(call);
                self.unified_sessions.insert(call.session, state);
                None
            }
            Err(error) => {
                let _ = self.parakeet_scheduler.cancel_reservation(call);
                state.failure = Some(error.clone());
                let _ = self.parakeet_scheduler.abort_session(call.session);
                self.unified_sessions.insert(call.session, state);
                Some(Err((call.session.job_id, call.session.generation, error)))
            }
        }
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

    fn start_worker_with_candidate(
        &mut self,
        request: whisper_core::ports::ImportRequest,
        candidate: Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
    ) -> Result<(), (JobId, Generation, String)> {
        self.start_worker_with_candidate_override(request, false, candidate)
    }

    fn start_worker_with_cpu_override(
        &mut self,
        request: whisper_core::ports::ImportRequest,
        force_cpu: bool,
    ) -> Result<(), (JobId, Generation, String)> {
        self.start_worker_with_candidate_override(request, force_cpu, None)
    }

    fn start_worker_with_candidate_override(
        &mut self,
        request: whisper_core::ports::ImportRequest,
        force_cpu: bool,
        candidate: Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
    ) -> Result<(), (JobId, Generation, String)> {
        let parakeet_sha = decode_sha256(
            whisper_core::model_registry::model(
                whisper_core::model_catalog::ModelId::ParakeetTdt06BV3,
            )
            .artifact
            .sha256,
        )
        .map_err(|error| (request.job_id, request.generation, error))?;
        if request.model_sha256 == parakeet_sha {
            return self.start_parakeet_worker(request, force_cpu, candidate);
        }
        let id = (request.job_id, request.generation);
        if !force_cpu {
            self.auto_fallback_reason = None;
        }
        crate::archive::assert_campaign_archive(Path::new(&request.destination))
            .map_err(|error| (id.0, id.1, error))?;
        if self.publication_gate.is_cancelled() {
            return Err((
                id.0,
                id.1,
                "Cancelled: generation invalidated before worker start".into(),
            ));
        }
        let source_sha256 = request.source_sha256.ok_or((
            id.0,
            id.1,
            "SourceMissing: source identity was not prepared".into(),
        ))?;
        let expected_source_samples = request.source_samples.ok_or((
            id.0,
            id.1,
            "InvalidInput: prepared WAV duration is missing".into(),
        ))?;
        let model_sha256 = request.model_sha256;
        self.start_diagnostics(Path::new(&request.destination));
        self.diagnostic_degraded_reported = false;
        self.progress = Some(ProgressSupervisor::starting(std::time::Instant::now()));
        let selection = self
            .workers
            .select(if force_cpu {
                whisper_core::ComputeChoice::Cpu
            } else {
                request.config.compute
            })
            .map_err(|error| {
                (
                    id.0,
                    id.1,
                    format!("BackendUnavailable: compute policy rejected request ({error:?})"),
                )
            })?;
        let expected_backend = match selection.backend {
            whisper_core::BackendAttempt::Cpu => BackendKind::Cpu,
            whisper_core::BackendAttempt::Gpu => BackendKind::Cuda,
        };
        if request.config.compute == whisper_core::ComputeChoice::Auto
            && expected_backend == BackendKind::Cpu
            && !force_cpu
        {
            self.auto_fallback_reason = Some("worker CUDA absent".into());
        }
        let mut worker_config = request.config.clone();
        if expected_backend == BackendKind::Cpu {
            worker_config.compute = whisper_core::ComputeChoice::Cpu;
        }
        let cached = self.idle_worker.take().and_then(|mut idle| {
            idle.matches_identity(worker_config.compute, &request.model_path, model_sha256)
                .then_some(idle.transport)
        });
        let mut transport = match cached
            .map(Ok)
            .unwrap_or_else(|| ChildWorkerTransport::spawn(&selection.path))
        {
            Ok(transport) => transport,
            Err(error)
                if request.config.compute == whisper_core::ComputeChoice::Auto
                    && expected_backend == BackendKind::Cuda
                    && !force_cpu =>
            {
                self.auto_fallback_reason = Some(format!("lancement CUDA impossible : {error:?}"));
                return self.start_worker_with_cpu_override(request, true);
            }
            Err(error) => return Err((id.0, id.1, format!("WorkerExited: {error:?}"))),
        };
        let instance_id = NEXT_INSTANCE.fetch_add(1, Ordering::Relaxed);
        if instance_id == 0 {
            return Err((
                id.0,
                id.1,
                "WorkerExited: instance identity exhausted".into(),
            ));
        }
        let retry_request = request.clone();
        let attested_candidate = candidate.clone();
        let command = match candidate {
            Some(candidate) if expected_backend == BackendKind::Cuda && !force_cpu => {
                WorkerCommand::StartGpuImport {
                    retain_after_publish: true,
                    job_id: id.0,
                    generation: id.1,
                    instance_id,
                    source_path: request.source_path,
                    source_sha256,
                    expected_source_samples,
                    model_path: request.model_path,
                    model_sha256,
                    config: worker_config,
                    candidate,
                }
            }
            _ => WorkerCommand::StartImport {
                retain_after_publish: true,
                job_id: id.0,
                generation: id.1,
                instance_id,
                source_path: request.source_path,
                source_sha256,
                expected_source_samples,
                model_path: request.model_path,
                model_sha256,
                config: worker_config,
            },
        };
        if let Err(error) = transport.request(command) {
            transport.abort();
            if request.config.compute == whisper_core::ComputeChoice::Auto
                && expected_backend == BackendKind::Cuda
                && !force_cpu
            {
                self.auto_fallback_reason = Some(format!("commande CUDA refusée : {error:?}"));
                return self.start_worker_with_cpu_override(retry_request, true);
            }
            return Err((id.0, id.1, format!("ProtocolMismatch: {error:?}")));
        }
        self.active = Some(ImportRequestState {
            job_id: id.0,
            generation: id.1,
            instance_id,
            backend: expected_backend,
            attempt_number: 0,
            import_request: Some(retry_request),
            recovery: LiveRecoveryState::None,
            startup_ready: false,
            gpu_candidate: attested_candidate,
            gpu_attested: false,
        });
        if let Some(diagnostics) = &self.diagnostics {
            diagnostics.record(DiagnosticRecord::new(
                id.0.0,
                id.1.get(),
                DiagnosticPhase::Preparing,
                DiagnosticCode::WorkerStarted,
                Some(match expected_backend {
                    BackendKind::Cpu => "cpu",
                    BackendKind::Cuda => "cuda",
                }),
                None,
            ));
        }
        self.transport = Some(transport);
        Ok(())
    }

    fn start_parakeet_worker(
        &mut self,
        request: whisper_core::ports::ImportRequest,
        force_cpu: bool,
        candidate: Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
    ) -> Result<(), (JobId, Generation, String)> {
        let id = (request.job_id, request.generation);
        if self.active.is_some() || self.parakeet.is_some() || self.transport.is_some() {
            return Err((id.0, id.1, "Busy: another worker is active".into()));
        }
        if !matches!(
            &request.config.language,
            whisper_core::LanguageChoice::Automatic
        ) {
            return Err((
                id.0,
                id.1,
                "ModelNotQualified: Parakeet requiert la langue automatique".into(),
            ));
        }
        if request.source_sha256.is_none() {
            return Err((
                id.0,
                id.1,
                "SourceMissing: source identity was not prepared".into(),
            ));
        }
        let source_samples = request.source_samples.ok_or((
            id.0,
            id.1,
            "InvalidInput: source duration was not prepared".into(),
        ))?;
        if source_samples == 0 {
            return Err((id.0, id.1, "InvalidInput: source duration is empty".into()));
        }
        if self.publication_gate.is_cancelled() {
            return Err((
                id.0,
                id.1,
                "Cancelled: generation invalidated before worker start".into(),
            ));
        }
        let record = whisper_core::model_registry::model(
            whisper_core::model_catalog::ModelId::ParakeetTdt06BV3,
        );
        let profile =
            if source_samples > whisper_core::parakeet_ipc::PARAKEET_MAX_AUDIO_SAMPLES as u64 {
                whisper_core::parakeet_windows::PROFILE_IMPORT_VAD
            } else {
                whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS
            };
        if profile != whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS
            || source_samples
                > whisper_core::model_registry::PARAKEET_MAX_SINGLE_PASS_DURATION_MS * 16
        {
            return Err((
                id.0,
                id.1,
                "DurationUnsupported: Fichier trop long pour Parakeet, utilisez Turbo".into(),
            ));
        }
        let auto = request.config.compute == whisper_core::ComputeChoice::Auto && !force_cpu;
        let candidate_is_current = candidate
            .as_ref()
            .is_some_and(|candidate| candidate.generation == id.1.get());
        let mut use_gpu = !force_cpu
            && match request.config.compute {
                whisper_core::ComputeChoice::Cpu => false,
                whisper_core::ComputeChoice::Gpu => true,
                whisper_core::ComputeChoice::Auto => candidate_is_current,
            };
        if request.config.compute == whisper_core::ComputeChoice::Gpu && !candidate_is_current {
            return Err((
                id.0,
                id.1,
                "BackendMismatch: selected GPU has no current runtime identity proof".into(),
            ));
        }
        let mut expected_gpu = if use_gpu {
            match candidate
                .as_ref()
                .map(crate::parakeet_import::expected_gpu_for_candidate)
                .transpose()
            {
                Ok(expected) => expected,
                Err(error) if auto => {
                    use_gpu = false;
                    self.auto_fallback_reason = Some(format!(
                        "GPU identity preflight failed before native inference: {error}"
                    ));
                    self.replace_parakeet_pending_with_cpu_key(id)?;
                    None
                }
                Err(error) => return Err((id.0, id.1, error)),
            }
        } else {
            None
        };
        if !use_gpu && request.config.compute == whisper_core::ComputeChoice::Auto && !force_cpu {
            self.auto_fallback_reason = Some(if candidate.is_some() {
                "GPU candidate identity is stale before processing".into()
            } else {
                "GPU candidate proof unavailable before processing".into()
            });
            if candidate.is_some() {
                self.replace_parakeet_pending_with_cpu_key(id)?;
            }
        }
        let selected_paths = if use_gpu {
            self.gpu_parakeet_paths.as_ref()
        } else {
            self.parakeet_paths.as_ref()
        };
        let mut qualification = selected_paths
            .ok_or_else(|| {
                (
                    id.0,
                    id.1,
                    if use_gpu {
                        "BackendUnavailable: worker GPU Test Parakeet non configuré".into()
                    } else {
                        "ModelNotQualified: worker CPU Test Parakeet non configuré".into()
                    },
                )
            })
            .and_then(|paths| {
                if use_gpu {
                    paths.gpu_qualification_key(profile)
                } else {
                    paths.qualification_key(profile)
                }
                .map_err(|error| (id.0, id.1, error))
            });
        if use_gpu
            && qualification
                .as_ref()
                .is_ok_and(|key| !gpu_qualification_candidate_allowed(key))
        {
            qualification = Err((
                id.0,
                id.1,
                "ModelNotQualified: GPU Test key is not admitted".into(),
            ));
        }
        if use_gpu && qualification.is_err() && auto {
            use_gpu = false;
            expected_gpu = None;
            self.auto_fallback_reason =
                Some("GPU qualification unavailable before native inference".into());
            self.replace_parakeet_pending_with_cpu_key(id)?;
            qualification = self
                .parakeet_paths
                .as_ref()
                .ok_or((
                    id.0,
                    id.1,
                    "ModelNotQualified: worker CPU Test Parakeet non configuré".into(),
                ))
                .and_then(|paths| {
                    paths
                        .qualification_key(profile)
                        .map_err(|error| (id.0, id.1, error))
                });
        }
        let qualification = qualification?;
        let paths = if use_gpu {
            self.gpu_parakeet_paths.as_ref()
        } else {
            self.parakeet_paths.as_ref()
        }
        .ok_or((
            id.0,
            id.1,
            "ModelNotQualified: Parakeet worker is absent".into(),
        ))?;
        paths
            .verify_qualification(
                record.artifact.sha256,
                &qualification.runtime_build,
                &qualification.worker_build,
            )
            .map_err(|error| (id.0, id.1, error))?;
        crate::archive::assert_campaign_archive(Path::new(&request.destination))
            .map_err(|error| (id.0, id.1, error))?;
        let process_result = crate::parakeet_import::ParakeetProcess::spawn(paths);
        let process = match process_result {
            Ok(process) => process,
            Err(error) if auto && use_gpu => {
                return self
                    .fallback_parakeet_to_cpu(
                        request,
                        format!("GPU worker preparation failed: {error}"),
                    )
                    .map_err(|message| (id.0, id.1, message));
            }
            Err(error) => return Err((id.0, id.1, format!("WorkerExited: {error}"))),
        };
        let vad_run = if profile == whisper_core::parakeet_windows::PROFILE_IMPORT_VAD {
            let source_sha256 = request
                .source_sha256
                .map(|sha| format_sha256(&sha))
                .ok_or((id.0, id.1, "SourceMissing: source hash is absent".into()))?;
            let decoder_transport = ChildWorkerTransport::spawn(&self.workers.cpu)
                .map_err(|error| (id.0, id.1, format!("DecoderUnavailable: {error:?}")))?;
            Some(ParakeetVadRun {
                request: request.clone(),
                decoder: Some(crate::decoder::DecoderProxy::new(decoder_transport)),
                decode_offset: 0,
                total_samples: source_samples,
                source_sha256,
                pcm: Vec::new(),
                windows: Vec::new(),
                next_window: 0,
                next_segment_sequence: self.archive.next_segment_sequence(),
                request_count: 0,
                current_attempt: 0,
                current_path: None,
                current_source_sha256: None,
                pending_confirmation: None,
                worker_ready: false,
                ready_event_sent: false,
            })
        } else {
            None
        };
        let instance_id = NEXT_INSTANCE.fetch_add(1, Ordering::Relaxed);
        if instance_id == 0 {
            return Err((
                id.0,
                id.1,
                "WorkerExited: instance identity exhausted".into(),
            ));
        }
        self.start_diagnostics(Path::new(&request.destination));
        self.progress = Some(ProgressSupervisor::starting(Instant::now()));
        self.active = Some(ImportRequestState {
            job_id: id.0,
            generation: id.1,
            instance_id,
            backend: if use_gpu {
                BackendKind::Cuda
            } else {
                BackendKind::Cpu
            },
            attempt_number: 1,
            import_request: Some(request),
            recovery: LiveRecoveryState::None,
            startup_ready: false,
            gpu_candidate: if use_gpu { candidate } else { None },
            gpu_attested: false,
        });
        self.parakeet_expected_gpu = expected_gpu;
        self.parakeet_native_call_started = false;
        let _ = source_samples;
        self.parakeet = Some(process);
        self.parakeet_vad = vad_run;
        if self.parakeet_vad.is_some() {
            self.resume_prefix.clear();
        }
        Ok(())
    }

    fn replace_parakeet_pending_with_cpu_key(
        &mut self,
        id: (JobId, Generation),
    ) -> Result<(), (JobId, Generation, String)> {
        let key = self
            .parakeet_paths
            .as_ref()
            .ok_or((id.0, id.1, "CPU Parakeet worker is absent".into()))?
            .qualification_key(whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS)
            .map_err(|error| (id.0, id.1, error))?;
        self.archive
            .replace_pending_qualification(key)
            .map_err(|error| (id.0, id.1, error))
    }

    fn fallback_parakeet_to_cpu(
        &mut self,
        request: ImportRequest,
        reason: String,
    ) -> Result<(), String> {
        if self.parakeet_native_call_started {
            return Err(
                "BackendMismatch: CPU fallback is forbidden after NativeCallStarted".into(),
            );
        }
        let id = (request.job_id, request.generation);
        self.replace_parakeet_pending_with_cpu_key(id)
            .map_err(|(_, _, error)| error)?;
        if let Some(process) = self.parakeet.as_mut() {
            process.terminate().map_err(|error| {
                format!("WorkerExited: GPU preparation failure; termination failed: {error}")
            })?;
        }
        self.parakeet = None;
        self.active = None;
        self.parakeet_expected_gpu = None;
        self.parakeet_native_call_started = false;
        self.auto_fallback_reason = Some(reason);
        self.start_parakeet_worker(request, true, None)
            .map_err(|(_, _, error)| error)
    }

    fn handle_worker_failure(
        &mut self,
        job_id: JobId,
        generation: Generation,
        code: whisper_core::ipc::WorkerErrorCode,
        message: String,
    ) -> Result<ImportEvent, (JobId, Generation, String)> {
        if self.live.is_none()
            && let Some(pending) = self.pending_live.as_mut().filter(|pending| {
                (pending.request.job_id, pending.request.generation) == (job_id, generation)
            })
        {
            let backend = self.active.as_ref().map(|active| active.backend);
            let retryable = matches!(
                code,
                whisper_core::ipc::WorkerErrorCode::BackendUnavailable
                    | whisper_core::ipc::WorkerErrorCode::BackendMismatch
                    | whisper_core::ipc::WorkerErrorCode::WorkerExited
                    | whisper_core::ipc::WorkerErrorCode::InferenceFailed
            );
            let decision = if retryable && backend == Some(BackendKind::Cuda) {
                whisper_core::ComputePolicy::after_failure(
                    pending.request.config.compute,
                    whisper_core::BackendAttempt::Gpu,
                    false,
                )
            } else {
                whisper_core::BackendDecision::Fail
            };
            match decision {
                whisper_core::BackendDecision::RetryCpu => {
                    pending.recovery = LiveRecoveryState::AutoRetryPending
                }
                whisper_core::BackendDecision::AwaitUserChoice => {
                    pending.recovery = LiveRecoveryState::AwaitingChoicePending
                }
                _ => return Err((job_id, generation, message)),
            }
            if let Some(transport) = self.transport.as_mut() {
                let _ = transport.try_request(WorkerCommand::Stop { job_id, generation });
            }
            return Ok(match decision {
                whisper_core::BackendDecision::RetryCpu => ImportEvent::LiveRetryingOnCpu {
                    job_id,
                    generation,
                    spectrum_min_epoch: 0,
                    spectrum_not_before_monotonic_ms:
                        whisper_core::microphone_level::monotonic_millis(),
                },
                _ => ImportEvent::LiveFinalizing { job_id, generation },
            });
        }
        if self.live.is_none()
            && let Some(active) = self.active.as_mut().filter(|active| {
                active.job_id == job_id
                    && active.generation == generation
                    && active.import_request.is_some()
            })
        {
            let retryable = matches!(
                code,
                whisper_core::ipc::WorkerErrorCode::BackendUnavailable
                    | whisper_core::ipc::WorkerErrorCode::BackendMismatch
                    | whisper_core::ipc::WorkerErrorCode::WorkerExited
                    | whisper_core::ipc::WorkerErrorCode::InferenceFailed
            );
            let decision = if retryable && active.backend == BackendKind::Cuda {
                whisper_core::ComputePolicy::after_failure(
                    active
                        .import_request
                        .as_ref()
                        .expect("checked above")
                        .config
                        .compute,
                    whisper_core::BackendAttempt::Gpu,
                    active.startup_ready,
                )
            } else {
                whisper_core::BackendDecision::Fail
            };
            if decision == whisper_core::BackendDecision::RetryCpu {
                self.auto_fallback_reason
                    .get_or_insert_with(|| message.clone());
                active.recovery = LiveRecoveryState::AutoRetryPending;
                if let Some(transport) = self.transport.as_mut() {
                    let _ = transport.try_request(WorkerCommand::Stop { job_id, generation });
                }
                return Ok(ImportEvent::LiveAttemptStarting { job_id, generation });
            }
        }
        if let Some(live) = self.live.as_mut() {
            let backend = self.active.as_ref().map(|active| active.backend);
            let choice = live.request.config.compute;
            let retryable = matches!(
                code,
                whisper_core::ipc::WorkerErrorCode::BackendUnavailable
                    | whisper_core::ipc::WorkerErrorCode::BackendMismatch
                    | whisper_core::ipc::WorkerErrorCode::WorkerExited
                    | whisper_core::ipc::WorkerErrorCode::InferenceFailed
            );
            let decision = if retryable && backend == Some(BackendKind::Cuda) {
                whisper_core::ComputePolicy::after_failure(
                    choice,
                    whisper_core::BackendAttempt::Gpu,
                    self.active
                        .as_ref()
                        .is_some_and(|active| active.startup_ready),
                )
            } else {
                whisper_core::BackendDecision::Fail
            };
            match decision {
                whisper_core::BackendDecision::RetryCpu => {
                    live.recovery = LiveRecoveryState::AutoRetryPending;
                    live.inference_admission_closed = true;
                    live.pending_window = None;
                    live.barrier_m = Some(live.inference_sequence.saturating_sub(1));
                }
                whisper_core::BackendDecision::AwaitUserChoice => {
                    live.recovery = LiveRecoveryState::AwaitingChoicePending;
                    live.inference_admission_closed = true;
                    live.pending_window = None;
                    live.barrier_m = Some(live.inference_sequence.saturating_sub(1));
                    live.stop_capture();
                }
                whisper_core::BackendDecision::Fail => {
                    live.stop_capture();
                    let _ = live.staging.drain();
                    live.failure = Some(message.clone());
                }
                whisper_core::BackendDecision::Start(_) => {
                    live.stop_capture();
                    let _ = live.staging.drain();
                    live.failure = Some(message.clone());
                }
            }
            if decision == whisper_core::BackendDecision::AwaitUserChoice {
                let mut stop_events = VecDeque::new();
                self.stop(job_id, generation, &mut stop_events)?;
                self.pending_recovery_counters = recovery_stop_counters(stop_events);
            }
            if let Some(transport) = self.transport.as_mut() {
                let _ = transport.try_request(WorkerCommand::Stop { job_id, generation });
            }
            let retry_spectrum_barrier = self.live.as_ref().map(|live| live.probe.advance_epoch());
            return match decision {
                whisper_core::BackendDecision::RetryCpu => {
                    let (spectrum_min_epoch, spectrum_not_before_monotonic_ms) =
                        retry_spectrum_barrier.unwrap_or_else(|| {
                            (0, whisper_core::microphone_level::monotonic_millis())
                        });
                    Ok(ImportEvent::LiveRetryingOnCpu {
                        job_id,
                        generation,
                        spectrum_min_epoch,
                        spectrum_not_before_monotonic_ms,
                    })
                }
                whisper_core::BackendDecision::AwaitUserChoice => {
                    Ok(ImportEvent::LiveFinalizing { job_id, generation })
                }
                whisper_core::BackendDecision::Fail => Err((job_id, generation, message)),
                whisper_core::BackendDecision::Start(_) => Err((job_id, generation, message)),
            };
        }
        self.pending_live = None;
        self.live_archive = None;
        self.transport = None;
        self.active = None;
        Err((job_id, generation, message))
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
