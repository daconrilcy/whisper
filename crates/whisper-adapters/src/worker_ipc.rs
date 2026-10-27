use crate::{
    WorkerTransport,
    archive::{ArchiveStore, LiveArchive},
    capture::{CaptureStream, Pcm16Converter},
    staging::{PcmStaging, PcmWriter},
    supervisor::{
        DiagnosticCode, DiagnosticPhase, DiagnosticRecord, DiagnosticSink, ProgressPhase,
        ProgressSupervisor, WorkerPaths,
    },
    vad::VoiceActivity,
};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    cell::RefCell,
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    fs,
    io::{self, BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU8, AtomicU64, Ordering},
        mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError},
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
#[cfg(feature = "l01-memory-qualification")]
use std::{
    fs::{File, OpenOptions},
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};
use whisper_core::{
    ArchiveHistoryItem, ComputeChoice, DiagnosticNotice, Generation, ImportEffect, ImportEvent,
    ImportIoPort, JobId,
    ipc::{BackendKind, IPC_PROTOCOL_VERSION, IpcEnvelope, WorkerCommand, WorkerEvent},
    ports::{
        DecodeRequest, DecoderPort, DuplicateImportChoice, DuplicateImportPrompt, ImportRequest,
        LiveRequest, PortError, PublishedImport, SourceIdentity, TreatmentIdentity, WorkerSegment,
    },
};

const MAX_FRAME_BYTES: usize = 1_048_576;
const TDT_FRAME_OVERLAP_SAMPLES: u64 = 80 * 16;
const LIVE_CAPTURE_EVENTS_BEFORE_WORKER_POLL: u8 = 2;
const UNIFIED_CHILD_JOB_CAPACITY: usize = 2;

fn live_capture_poll_allowed(events_since_worker_poll: u8, worker_available: bool) -> bool {
    events_since_worker_poll < LIVE_CAPTURE_EVENTS_BEFORE_WORKER_POLL || !worker_available
}
static NEXT_IMPORT_JOB_ID: AtomicU64 = AtomicU64::new(1);
static NEXT_AUTO_EXPORT_STAGE: AtomicU64 = AtomicU64::new(1);

fn next_automatic_final_job_id() -> JobId {
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
    JobId(((std::process::id() as u128) << 64) | serial as u128)
}

fn inspect_automatic_final_queue(
    destination: &Path,
) -> Result<Vec<crate::queue_store::QueueEntry>, String> {
    inspect_automatic_final_queue_with(destination, |path| {
        crate::queue_store::QueueStore::inspect_read_only(path)
    })
}

fn inspect_automatic_final_queue_with(
    destination: &Path,
    inspect: impl FnOnce(&Path) -> Result<Vec<crate::queue_store::QueueEntry>, String>,
) -> Result<Vec<crate::queue_store::QueueEntry>, String> {
    inspect(destination).map_err(|error| format!("AutomaticFinalQueueUnavailable: {error}"))
}

fn resolve_reprocessing_targets(
    destination: &Path,
    catalog: &[PublishedImport],
    choice: &DuplicateImportChoice,
) -> Result<Vec<TreatmentIdentity>, String> {
    let root = destination
        .canonicalize()
        .map_err(|error| format!("InvalidInput: destination non accessible: {error}"))?;
    let verified: Vec<_> = catalog
        .iter()
        .filter(|item| item.verified_present)
        .collect();
    let selected = match choice {
        DuplicateImportChoice::ReplaceAll => verified,
        DuplicateImportChoice::ReplaceLatest => {
            let latest_ordinal = verified
                .iter()
                .filter_map(|item| item.success_ordinal)
                .max();
            let latest = if let Some(ordinal) = latest_ordinal {
                verified
                    .iter()
                    .copied()
                    .filter(|item| item.success_ordinal == Some(ordinal))
                    .collect::<Vec<_>>()
            } else if verified.len() == 1 {
                verified.clone()
            } else {
                return Err("Impossible de déterminer le dernier traitement : l'ordre historique est inconnu. Choisissez tous les traitements ou Parcourir.".into());
            };
            if latest.len() != 1 {
                return Err("Impossible de déterminer un dernier traitement unique. Choisissez tous les traitements ou Parcourir.".into());
            }
            latest
        }
        DuplicateImportChoice::ReplaceSelectedPath(selected_path) => {
            let chosen = Path::new(selected_path)
                .canonicalize()
                .map_err(|error| format!("Le dossier choisi n'est pas accessible : {error}"))?;
            let transcriptions = root
                .join("transcriptions")
                .canonicalize()
                .map_err(|error| {
                    format!("La destination ne contient pas de catalogue : {error}")
                })?;
            if !chosen.starts_with(&transcriptions) {
                return Err("Le traitement doit appartenir à la destination courante.".into());
            }
            let matches = verified
                .into_iter()
                .filter(|item| {
                    let path = root
                        .join("transcriptions")
                        .join(format!("{:032x}", item.job_id.0))
                        .join(item.generation.get().to_string());
                    path.canonicalize().ok().as_deref() == Some(chosen.as_path())
                })
                .collect::<Vec<_>>();
            if matches.len() != 1 {
                return Err("Le dossier choisi n'est pas un traitement vérifié du même audio dans cette destination.".into());
            }
            matches
        }
        _ => return Err("Choix de remplacement non pris en charge".into()),
    };
    if selected.is_empty() {
        return Err("Aucun traitement vérifié et présent ne peut être remplacé.".into());
    }
    Ok(selected
        .into_iter()
        .map(|item| TreatmentIdentity {
            destination: root.to_string_lossy().into_owned(),
            job_id: item.job_id,
            generation: item.generation,
        })
        .collect())
}

fn verified_present_publications(publications: Vec<PublishedImport>) -> Vec<PublishedImport> {
    publications
        .into_iter()
        .filter(|publication| publication.verified_present)
        .collect()
}

#[derive(Clone)]
struct R7TimingContext {
    operation: String,
    operation_id: String,
    destination: String,
}

thread_local! {
    static R7_TIMING_CONTEXT: RefCell<Option<R7TimingContext>> = const { RefCell::new(None) };
}

struct R7TimingContextGuard(Option<R7TimingContext>);

impl R7TimingContextGuard {
    fn enter(operation: &str, operation_id: String, destination: &Path) -> Self {
        let context = R7TimingContext {
            operation: operation.to_owned(),
            operation_id,
            destination: destination.to_string_lossy().into_owned(),
        };
        Self(R7_TIMING_CONTEXT.with(|current| current.replace(Some(context))))
    }
}

impl Drop for R7TimingContextGuard {
    fn drop(&mut self) {
        let previous = self.0.take();
        R7_TIMING_CONTEXT.with(|current| {
            current.replace(previous);
        });
    }
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

pub(crate) fn record_r7_timing(stage: &str, elapsed: Duration, details: serde_json::Value) {
    if std::env::var_os("PARAKEET_R7_TIMING_ACTIVE").as_deref() != Some(std::ffi::OsStr::new("1")) {
        return;
    }
    let Some(path) = std::env::var_os("PARAKEET_R7_TIMING_LOG").map(PathBuf::from) else {
        eprintln!("R7 timing skipped: log path is absent");
        return;
    };
    let Some(root) = std::env::var_os("PARAKEET_R7_EXEC_ROOT").map(PathBuf::from) else {
        eprintln!("R7 timing skipped: execution root is absent");
        return;
    };
    let Ok(root) = root.canonicalize() else {
        eprintln!("R7 timing skipped: execution root is unavailable");
        return;
    };
    let Some(parent) = path.parent() else {
        eprintln!("R7 timing skipped: log path has no parent");
        return;
    };
    let Ok(parent) = parent.canonicalize() else {
        eprintln!("R7 timing skipped: log parent is unavailable");
        return;
    };
    if !parent.starts_with(&root) {
        eprintln!("R7 timing skipped: log path escaped R7");
        return;
    }
    static PROCESS_START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    let at_ms = PROCESS_START
        .get_or_init(Instant::now)
        .elapsed()
        .as_millis();
    let mut file = match fs::OpenOptions::new().create(true).append(true).open(path) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("R7 timing log open failed: {error}");
            return;
        }
    };
    let context = R7_TIMING_CONTEXT.with(|current| current.borrow().clone());
    let row = serde_json::json!({
        "process": "adapter",
        "origin": if context.is_some() { "product" } else { "unscoped" },
        "pid": std::process::id(),
        "at_ms": at_ms,
        "stage": stage,
        "elapsed_ms": elapsed.as_secs_f64() * 1000.0,
        "operation": context.as_ref().map(|value| value.operation.as_str()),
        "operation_id": context.as_ref().map(|value| value.operation_id.as_str()),
        "destination": context.as_ref().map(|value| value.destination.as_str()),
        "details": details,
    });
    if serde_json::to_writer(&mut file, &row).is_err() || file.write_all(b"\n").is_err() {
        eprintln!("R7 timing log write failed");
    }
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
const GATE_IDLE: u8 = 0;
const GATE_ACTIVE: u8 = 1;
const GATE_CANCELLED: u8 = 2;
const GATE_COMMITTING: u8 = 3;
const GATE_COMMITTED: u8 = 4;

/// Deterministic process-level barriers used only by the acceptance-test build.
/// Their root is checked against the isolated L2a-2 runtime before touching disk.
fn l2a2_acceptance_barrier(phase: &str, destination: &Path) -> Result<(), String> {
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

fn l2a2_acceptance_take_worker_fault(
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
    let l3c_execution_root = canonical_l3_acceptance_root(
        std::env::var_os("PARAKEET_L3C_EXEC_ROOT").map(PathBuf::from),
        std::env::var_os("PARAKEET_L3D_EXEC_ROOT").map(PathBuf::from),
    );
    let in_l2a2 = destination.starts_with(&runtime) && control.starts_with(&runtime);
    let in_l3_test_root = option_env!("PARAKEET_L2_ACCEPTANCE_TEST_BUILD").is_some()
        && l3c_execution_root
            .as_ref()
            .is_some_and(|root| destination.starts_with(root) && control.starts_with(root));
    if !in_l2a2 && !in_l3_test_root {
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

fn canonical_l3_acceptance_root(l3c: Option<PathBuf>, l3d: Option<PathBuf>) -> Option<PathBuf> {
    let (configured, lot) = match (l3c, l3d) {
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
static NEXT_INSTANCE: AtomicU64 = AtomicU64::new(1);

fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            elapsed.as_millis().min(u128::from(u64::MAX)) as u64
        })
}

#[cfg(test)]
mod parakeet_stopped_ack_tests {
    use super::{ParakeetStoppedExit, parakeet_stopped_exit};

    #[test]
    fn stopped_ack_waits_for_exit_then_allows_completion() {
        assert_eq!(parakeet_stopped_exit(None), ParakeetStoppedExit::Wait);
        assert_eq!(
            parakeet_stopped_exit(Some(true)),
            ParakeetStoppedExit::Success
        );
        assert_eq!(
            parakeet_stopped_exit(Some(false)),
            ParakeetStoppedExit::Failure
        );
    }
}

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

#[cfg(feature = "l01-memory-qualification")]
static MEMORY_REPORT: OnceLock<Option<Mutex<std::io::BufWriter<File>>>> = OnceLock::new();

#[cfg(feature = "l01-memory-qualification")]
fn memory_trace(stage: &str, details: serde_json::Value) {
    let report = MEMORY_REPORT.get_or_init(|| {
        let path = std::env::var_os("WHISPER_L01_ADAPTER_MEMORY_TRACE")?;
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .ok()
            .map(|file| Mutex::new(std::io::BufWriter::new(file)))
    });
    let Some(report) = report else { return };
    let elapsed_unix_us = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_micros());
    if let Ok(mut output) = report.lock() {
        let record = serde_json::json!({
            "elapsed_unix_us": elapsed_unix_us,
            "stage": stage,
            "details": details,
        });
        let _ = serde_json::to_writer(&mut *output, &record);
        let _ = output.write_all(b"\n");
        let _ = output.flush();
    }
}

#[derive(Default)]
struct PublicationGate(AtomicU8);

impl PublicationGate {
    fn activate(&self) {
        self.0.store(GATE_ACTIVE, Ordering::Release);
    }

    fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire) == GATE_CANCELLED
    }

    fn request_stop(&self) -> Result<bool, PortError> {
        match self.0.compare_exchange(
            GATE_ACTIVE,
            GATE_CANCELLED,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => Ok(true),
            Err(GATE_CANCELLED) => Ok(false),
            Err(GATE_COMMITTING | GATE_COMMITTED) => Err(PortError::Committing),
            Err(GATE_IDLE) => Err(PortError::Unavailable),
            Err(_) => Err(PortError::Unavailable),
        }
    }

    fn rollback_stop(&self) {
        let _ = self.0.compare_exchange(
            GATE_CANCELLED,
            GATE_ACTIVE,
            Ordering::AcqRel,
            Ordering::Acquire,
        );
    }

    fn commit<T>(&self, commit: impl FnOnce() -> Result<T, String>) -> Result<T, String> {
        match self.0.compare_exchange(
            GATE_ACTIVE,
            GATE_COMMITTING,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => {
                let result = commit();
                if result.is_ok() {
                    self.0.store(GATE_COMMITTED, Ordering::Release);
                }
                result
            }
            Err(GATE_CANCELLED) => Err("Cancelled: publication invalidated by Stop".into()),
            Err(GATE_COMMITTING | GATE_COMMITTED) => {
                Err("Publication: commit is already in progress".into())
            }
            Err(_) => Err("Publication: generation is not active".into()),
        }
    }
}

pub struct ChildWorkerTransport {
    child: Child,
    commands: SyncSender<WorkerCommand>,
    stop_commands: SyncSender<WorkerCommand>,
    events: Receiver<WorkerReaderMessage>,
    transport_ended: bool,
    expected_gpu_session: Option<ExpectedGpuSession>,
    gpu_session_attested: bool,
    #[cfg(test)]
    injected_events: VecDeque<WorkerEvent>,
}

#[derive(Clone)]
struct ExpectedGpuSession {
    job_id: JobId,
    generation: Generation,
    instance_id: u64,
    candidate: whisper_core::gpu_inventory::GpuRuntimeIdentity,
}

#[derive(Debug, PartialEq)]
enum WorkerReaderMessage {
    Event(Result<WorkerEvent, PortError>),
    TransportEnded,
}

impl ChildWorkerTransport {
    pub fn spawn(executable: &Path) -> Result<Self, PortError> {
        let mut command = Command::new(executable);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            command.creation_flags(CREATE_NO_WINDOW);
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| PortError::Failed(format!("WorkerExited: {e}")))?;
        let stdin = child.stdin.take().ok_or(PortError::Unavailable)?;
        let stdout = child.stdout.take().ok_or(PortError::Unavailable)?;
        let stderr = child.stderr.take().ok_or(PortError::Unavailable)?;
        // Drain diagnostics so native logging cannot block the worker. Session
        // readiness is proven only by the correlated typed IPC attestation.
        thread::spawn(move || {
            let mut stderr = BufReader::new(stderr);
            let mut line = Vec::new();
            while stderr
                .read_until(b'\n', &mut line)
                .is_ok_and(|size| size > 0)
            {
                line.clear();
            }
        });
        #[cfg(all(feature = "l01-memory-qualification", windows))]
        let stdin_pipe_handle = {
            use std::os::windows::io::AsRawHandle;
            stdin.as_raw_handle() as usize
        };
        let (commands, command_rx) = mpsc::sync_channel(1);
        let (stop_commands, stop_rx) = mpsc::sync_channel(1);
        thread::spawn(move || {
            let mut stdin = stdin;
            loop {
                let command = match take_next_worker_command(&command_rx, &stop_rx) {
                    WorkerCommandPoll::Command(command) => command,
                    WorkerCommandPoll::Idle => continue,
                    WorkerCommandPoll::Closed => return,
                };
                if write_frame(&mut stdin, &IpcEnvelope::new(command)).is_err() {
                    break;
                }
            }
        });
        let (tx, events) = mpsc::sync_channel(EVENTS_PER_STAGE);
        #[cfg(all(feature = "l01-memory-qualification", windows))]
        {
            use std::os::windows::io::AsRawHandle;
            memory_trace(
                "adapter.worker.spawned",
                serde_json::json!({
                    "worker_pid": child.id(),
                    "worker_event_channel_capacity": EVENTS_PER_STAGE,
                    "worker_event_channel_slot_bytes": std::mem::size_of::<Result<WorkerEvent, PortError>>(),
                    "stdin_pipe_handle": stdin_pipe_handle,
                    "stdout_pipe_handle": stdout.as_raw_handle() as usize,
                    "kernel_pipe_capacity_bytes": "queried by Windows parent sampler",
                }),
            );
        }
        #[cfg(all(feature = "l01-memory-qualification", not(windows)))]
        memory_trace(
            "adapter.worker.spawned",
            serde_json::json!({
                "worker_pid": child.id(),
                "worker_event_channel_capacity": EVENTS_PER_STAGE,
                "worker_event_channel_slot_bytes": std::mem::size_of::<Result<WorkerEvent, PortError>>(),
                "kernel_pipe_capacity_bytes": "Windows pipe query not available for this target",
            }),
        );
        thread::spawn(move || read_events(stdout, tx));
        Ok(Self {
            child,
            commands,
            stop_commands,
            events,
            transport_ended: false,
            expected_gpu_session: None,
            gpu_session_attested: false,
            #[cfg(test)]
            injected_events: VecDeque::new(),
        })
    }

    #[cfg(test)]
    fn inject_event(&mut self, event: WorkerEvent) {
        self.injected_events.push_back(event);
    }

    pub fn try_wait(&mut self) -> Result<Option<std::process::ExitStatus>, PortError> {
        self.child
            .try_wait()
            .map_err(|e| PortError::Failed(format!("WorkerExited: {e}")))
    }

    pub(crate) fn abort(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for ChildWorkerTransport {
    fn drop(&mut self) {
        self.abort();
    }
}

impl WorkerTransport for ChildWorkerTransport {
    fn request(&mut self, command: WorkerCommand) -> Result<(), PortError> {
        match &command {
            WorkerCommand::StartGpuImport {
                job_id,
                generation,
                instance_id,
                candidate,
                ..
            }
            | WorkerCommand::StartGpuLiveWorker {
                job_id,
                generation,
                instance_id,
                candidate,
                ..
            }
            | WorkerCommand::StartGpuDirectLiveWorker {
                job_id,
                generation,
                instance_id,
                candidate,
                ..
            } => {
                self.expected_gpu_session = Some(ExpectedGpuSession {
                    job_id: *job_id,
                    generation: *generation,
                    instance_id: *instance_id,
                    candidate: candidate.clone(),
                });
                self.gpu_session_attested = false;
            }
            WorkerCommand::StartImport { .. }
            | WorkerCommand::StartLiveWorker { .. }
            | WorkerCommand::StartDirectLiveWorker { .. } => {
                self.expected_gpu_session = None;
                self.gpu_session_attested = false;
            }
            _ => {}
        }
        self.commands
            .send(command)
            .map_err(|_| PortError::Failed("WorkerExited: stdin dispatcher stopped".into()))
    }

    fn receive(&mut self) -> Result<Option<WorkerEvent>, PortError> {
        #[cfg(test)]
        if let Some(event) = self.injected_events.pop_front() {
            return Ok(Some(event));
        }
        match self.events.try_recv() {
            Ok(WorkerReaderMessage::Event(Ok(event))) => match &event {
                WorkerEvent::GpuSessionAttested {
                    job_id,
                    generation,
                    instance_id,
                    candidate,
                    dxgi_rank,
                    luid,
                    pci_id,
                    cuda_ordinal,
                    ggml_ordinal,
                } => {
                    let Some(expected) = &self.expected_gpu_session else {
                        return Err(PortError::Failed(
                            "BackendMismatch: unsolicited GPU session attestation".into(),
                        ));
                    };
                    let identity_matches = (*job_id, *generation, *instance_id)
                        == (expected.job_id, expected.generation, expected.instance_id);
                    let candidate_matches = candidate == &expected.candidate.pnp_instance_id
                        && *dxgi_rank == expected.candidate.dxgi_rank
                        && *luid == expected.candidate.luid
                        && pci_id.eq_ignore_ascii_case(&expected.candidate.pci_id)
                        && *cuda_ordinal == expected.candidate.cuda_ordinal
                        && *ggml_ordinal == expected.candidate.ggml_ordinal;
                    if !identity_matches || !candidate_matches || self.gpu_session_attested {
                        return Err(PortError::Failed(
                            "BackendMismatch: GPU session attestation is stale, duplicate, or mismatched"
                                .into(),
                        ));
                    }
                    self.gpu_session_attested = true;
                    Ok(Some(event))
                }
                WorkerEvent::Ready {
                    job_id,
                    generation,
                    instance_id,
                    backend: BackendKind::Cuda,
                } => {
                    let ready_matches_session =
                        self.expected_gpu_session.as_ref().is_some_and(|expected| {
                            (*job_id, *generation, *instance_id)
                                == (expected.job_id, expected.generation, expected.instance_id)
                        });
                    if !self.gpu_session_attested || !ready_matches_session {
                        return Err(PortError::Failed(
                            "BackendMismatch: CUDA Ready lacks a matching GPU session attestation"
                                .into(),
                        ));
                    }
                    Ok(Some(event))
                }
                _ => Ok(Some(event)),
            },
            Ok(WorkerReaderMessage::Event(Err(error))) => Err(error),
            Ok(WorkerReaderMessage::TransportEnded) => {
                self.transport_ended = true;
                Err(PortError::Failed(
                    "WorkerExited: worker transport ended after session End".into(),
                ))
            }
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => {
                if self.transport_ended {
                    Ok(None)
                } else {
                    Err(PortError::Failed("WorkerExited without End".into()))
                }
            }
        }
    }
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

impl ChildWorkerTransport {
    fn try_request(&self, command: WorkerCommand) -> Result<(), EnqueueError> {
        try_enqueue_worker_command(&self.commands, &self.stop_commands, command)
    }

    fn try_request_ordered(&self, command: WorkerCommand) -> Result<(), EnqueueError> {
        try_enqueue_bounded(&self.commands, command)
    }
}

enum WorkerCommandPoll {
    Command(WorkerCommand),
    Idle,
    Closed,
}

fn take_next_worker_command(
    commands: &Receiver<WorkerCommand>,
    emergency: &Receiver<WorkerCommand>,
) -> WorkerCommandPoll {
    if let Ok(command) = emergency.try_recv() {
        return WorkerCommandPoll::Command(command);
    }
    match commands.recv_timeout(Duration::from_millis(5)) {
        Ok(command) => WorkerCommandPoll::Command(command),
        Err(mpsc::RecvTimeoutError::Timeout) => match emergency.try_recv() {
            Ok(command) => WorkerCommandPoll::Command(command),
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => WorkerCommandPoll::Idle,
        },
        Err(mpsc::RecvTimeoutError::Disconnected) => match emergency.try_recv() {
            Ok(command) => WorkerCommandPoll::Command(command),
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => WorkerCommandPoll::Closed,
        },
    }
}

#[derive(Debug)]
enum EnqueueError {
    Full(Box<WorkerCommand>),
    Disconnected,
}

fn try_enqueue_worker_command(
    data: &SyncSender<WorkerCommand>,
    control: &SyncSender<WorkerCommand>,
    command: WorkerCommand,
) -> Result<(), EnqueueError> {
    let sender = if matches!(command, WorkerCommand::Stop { .. }) {
        control
    } else {
        data
    };
    try_enqueue_bounded(sender, command)
}

fn try_enqueue_bounded(
    commands: &SyncSender<WorkerCommand>,
    command: WorkerCommand,
) -> Result<(), EnqueueError> {
    commands.try_send(command).map_err(|error| match error {
        TrySendError::Full(command) => EnqueueError::Full(Box::new(command)),
        TrySendError::Disconnected(_) => EnqueueError::Disconnected,
    })
}

fn take_ready_live_stop(
    pending_persistence: usize,
    deferred: &mut Option<WorkerEvent>,
) -> Option<WorkerEvent> {
    (pending_persistence == 0)
        .then(|| deferred.take())
        .flatten()
}

fn read_events(stdout: impl io::Read, tx: SyncSender<WorkerReaderMessage>) {
    let mut input = BufReader::new(stdout);
    let mut saw_stopped = false;
    let mut saw_end = false;
    #[cfg(feature = "l01-memory-qualification")]
    memory_trace(
        "adapter.worker_stdout_reader",
        serde_json::json!({ "bufreader_capacity_bytes": input.capacity() }),
    );
    loop {
        let result = match read_frame::<IpcEnvelope<WorkerEvent>>(&mut input) {
            Ok(Some(envelope)) => Ok(envelope),
            Ok(None) if saw_stopped => break,
            Ok(None) if saw_end => break,
            Ok(None) => Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "worker exited without End",
            )),
            Err(error) => Err(error),
        };
        let event = match result {
            Ok(envelope) if envelope.version == IPC_PROTOCOL_VERSION => Ok(envelope.message),
            Ok(envelope) => Err(PortError::Failed(format!(
                "ProtocolMismatch: v{}",
                envelope.version
            ))),
            Err(error) => Err(PortError::Failed(format!(
                "WorkerExited without End: {error}"
            ))),
        };
        // A Stopped event may belong to an older worker instance. The runtime
        // checks its identity; keep reading until its correlated terminal event.
        let terminal = event.is_err();
        saw_stopped |= matches!(event, Ok(WorkerEvent::Stopped { .. }));
        saw_end |= matches!(event, Ok(WorkerEvent::End { .. }));
        if tx.send(WorkerReaderMessage::Event(event)).is_err() || terminal {
            break;
        }
    }
    let _ = tx.send(WorkerReaderMessage::TransportEnded);
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
    crate::archive::l04_campaign_gate("live.before_reserve", job_id.0, generation.get(), 0)?;
    let result = archive.reserve_attempt();
    crate::archive::l04_campaign_operation_result(
        "live.before_reserve",
        job_id.0,
        generation.get(),
        0,
        &result,
        None,
    )?;
    let attempt = result?;
    crate::archive::l04_campaign_gate("live.after_reserve", job_id.0, generation.get(), 0)?;
    Ok(attempt)
}

fn spawn_live_attempt(
    request: &LiveRequest,
    selection: &crate::supervisor::WorkerSelection,
    candidate: Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
) -> Result<(ChildWorkerTransport, u64), (JobId, Generation, String)> {
    let identity = (request.job_id, request.generation);
    let instance_id = NEXT_INSTANCE.fetch_add(1, Ordering::Relaxed);
    if instance_id == 0 {
        return Err((
            identity.0,
            identity.1,
            "WorkerExited: instance identity exhausted".into(),
        ));
    }
    crate::archive::l04_campaign_gate(
        "live.before_spawn",
        identity.0.0,
        identity.1.get(),
        instance_id,
    )
    .map_err(|error| (identity.0, identity.1, error))?;
    let mut worker_config = request.config.clone();
    worker_config.compute = match selection.backend {
        whisper_core::BackendAttempt::Cpu => whisper_core::ComputeChoice::Cpu,
        whisper_core::BackendAttempt::Gpu => whisper_core::ComputeChoice::Gpu,
    };
    let spawn_result = (|| -> Result<ChildWorkerTransport, String> {
        let mut transport = ChildWorkerTransport::spawn(&selection.path)
            .map_err(|error| format!("WorkerExited: {error:?}"))?;
        let command = match candidate {
            Some(candidate) if selection.backend == whisper_core::BackendAttempt::Gpu => {
                WorkerCommand::StartGpuLiveWorker {
                    job_id: identity.0,
                    generation: identity.1,
                    instance_id,
                    model_path: request.model_path.clone(),
                    model_sha256: request.model_sha256,
                    config: worker_config,
                    candidate,
                }
            }
            _ => WorkerCommand::StartLiveWorker {
                job_id: identity.0,
                generation: identity.1,
                instance_id,
                model_path: request.model_path.clone(),
                model_sha256: request.model_sha256,
                config: worker_config,
            },
        };
        transport
            .request(command)
            .map_err(|error| format!("ProtocolMismatch: {error:?}"))?;
        Ok(transport)
    })();
    crate::archive::l04_campaign_operation_result(
        "live.before_spawn",
        identity.0.0,
        identity.1.get(),
        instance_id,
        &spawn_result,
        None,
    )
    .map_err(|error| (identity.0, identity.1, error))?;
    let transport = spawn_result.map_err(|error| (identity.0, identity.1, error))?;
    crate::archive::l04_campaign_gate(
        "live.after_spawn",
        identity.0.0,
        identity.1.get(),
        instance_id,
    )
    .map_err(|error| (identity.0, identity.1, error))?;
    Ok((transport, instance_id))
}

fn read_frame<T: DeserializeOwned>(input: &mut impl BufRead) -> io::Result<Option<T>> {
    let mut bytes = Vec::with_capacity(4096);
    loop {
        let available = input.fill_buf()?;
        if available.is_empty() {
            if bytes.is_empty() {
                return Ok(None);
            }
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "incomplete IPC frame",
            ));
        }
        let upto = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(available.len(), |i| i + 1);
        if bytes.len().saturating_add(upto) > MAX_FRAME_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "IPC frame exceeds 1 MiB",
            ));
        }
        let finished = available[upto - 1] == b'\n';
        bytes.extend_from_slice(&available[..upto]);
        input.consume(upto);
        if finished {
            break;
        }
    }
    #[cfg(feature = "l01-memory-qualification")]
    memory_trace(
        "adapter.ipc.read_frame.buffer",
        serde_json::json!({
            "frame_payload_len_bytes": bytes.len().saturating_sub(1),
            "frame_buffer_capacity_bytes": bytes.capacity(),
            "frame_limit_bytes_including_newline": MAX_FRAME_BYTES,
        }),
    );
    bytes.pop();
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}
fn write_frame<T: Serialize>(output: &mut impl Write, value: &T) -> io::Result<()> {
    let bytes = serialize_frame(value)?;
    #[cfg(feature = "l01-memory-qualification")]
    memory_trace(
        "adapter.ipc.serialize.buffer",
        serde_json::json!({
            "serialized_payload_len_bytes": bytes.len(),
            "serialized_vec_capacity_bytes": bytes.capacity(),
            "frame_len_with_newline_bytes": bytes.len() + 1,
            "frame_limit_bytes": MAX_FRAME_BYTES,
        }),
    );
    output.write_all(&bytes)?;
    output.write_all(b"\n")?;
    output.flush()
}

struct BoundedFrameWriter {
    bytes: Vec<u8>,
}

impl Write for BoundedFrameWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let limit = MAX_FRAME_BYTES - 1;
        if self.bytes.len().saturating_add(bytes.len()) > limit {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "IPC frame exceeds 1 MiB",
            ));
        }
        self.bytes
            .try_reserve_exact(bytes.len())
            .map_err(io::Error::other)?;
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn serialize_frame<T: Serialize>(value: &T) -> io::Result<Vec<u8>> {
    let mut writer = BoundedFrameWriter {
        bytes: Vec::with_capacity(4_096),
    };
    serde_json::to_writer(&mut writer, value)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    Ok(writer.bytes)
}

struct ImportRuntime {
    workers: WorkerPaths,
    model_catalog: Option<RuntimeModelCatalog>,
    parakeet_paths: Option<crate::parakeet_import::ParakeetWorkerPaths>,
    gpu_parakeet_paths: Option<crate::parakeet_import::ParakeetWorkerPaths>,
    snapshot_registry_root: Option<PathBuf>,
    require_snapshot_registry: bool,
    parakeet: Option<crate::parakeet_import::ParakeetProcess>,
    parakeet_events: VecDeque<WorkerEvent>,
    parakeet_expected_gpu: Option<whisper_core::parakeet_ipc::ExpectedGpu>,
    parakeet_native_call_started: bool,
    parakeet_end: Option<(JobId, Generation, u64, u64)>,
    parakeet_stopped_ack: bool,
    parakeet_vad: Option<ParakeetVadRun>,
    idle_worker: Option<IdleWorker>,
    pending_idle_worker: Option<IdleWorker>,
    auto_fallback_reason: Option<String>,
    diagnostics: Option<DiagnosticSink>,
    diagnostic_start_failed: bool,
    diagnostic_degraded_reported: bool,
    progress: Option<ProgressSupervisor>,
    archive: ArchiveStore,
    source_bindings: HashMap<(JobId, u64), crate::source_binding::SourceBinding>,
    active: Option<ImportRequestState>,
    transport: Option<ChildWorkerTransport>,
    publication_gate: Arc<PublicationGate>,
    automatic_publication_gates: Option<Arc<Mutex<HashMap<(JobId, u64), Arc<PublicationGate>>>>>,
    unified_publication_gates:
        HashMap<whisper_core::parakeet_scheduler::SchedulerSession, Arc<PublicationGate>>,
    resume_prefix: VecDeque<crate::journal::SegmentRecord>,
    pending_live: Option<PendingLiveState>,
    live: Option<LiveCaptureState>,
    live_archive: Option<LiveArchive>,
    direct_live: Option<crate::direct_live::DirectLive>,
    parakeet_live: Option<crate::parakeet_live::ParakeetLive>,
    microphone_test: Option<MicrophoneTestCapture>,
    automatic_final_outputs: HashMap<(JobId, u64), whisper_core::ports::AutomaticFinalImport>,
    parakeet_pending_events: VecDeque<ImportEvent>,
    pending_attempt_reset: Option<ImportEvent>,
    pending_recovery_counters: Option<ImportEvent>,
    live_pending_persistence: usize,
    live_capture_events_since_worker_poll: u8,
    deferred_live_stopped: Option<WorkerEvent>,
    deferred_live_window_finished: Option<FinishedLiveWindow>,
    gpu_candidates: std::collections::HashMap<
        (JobId, u64),
        Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
    >,
    parakeet_scheduler: whisper_core::parakeet_scheduler::ParakeetScheduler,
    unified_sessions:
        HashMap<whisper_core::parakeet_scheduler::SchedulerSession, UnifiedSessionState>,
    unified_executor: Option<UnifiedInferenceExecutor>,
    unified_archives: HashMap<whisper_core::parakeet_scheduler::SchedulerSession, ArchiveStore>,
    #[cfg(test)]
    controlled_transcriber: Option<Box<dyn crate::parakeet_pipeline::WindowTranscriber>>,
    #[cfg(test)]
    controlled_captures: VecDeque<CaptureStream>,
    #[cfg(test)]
    controlled_part_limits: VecDeque<Option<u64>>,
    #[cfg(test)]
    controlled_disconnect_session: Option<whisper_core::parakeet_scheduler::SchedulerSession>,
    #[cfg(test)]
    controlled_panic_session: Option<whisper_core::parakeet_scheduler::SchedulerSession>,
    #[cfg(test)]
    controlled_finish_failure_session: Option<whisper_core::parakeet_scheduler::SchedulerSession>,
    #[cfg(test)]
    controlled_hold_reserved_call_once: bool,
    #[cfg(test)]
    controlled_ready_gate: Option<Arc<(Mutex<bool>, std::sync::Condvar)>>,
    #[cfg(test)]
    controlled_child_observer: Option<Arc<Mutex<Vec<ControlledChildObservation>>>>,
    #[cfg(test)]
    controlled_skip_child_recycle: bool,
    document_drains: VecDeque<UnifiedDocumentDrain>,
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
    source: UnifiedSessionSource,
    available_samples: u64,
    durable_samples: u64,
    source_closed: bool,
    document: crate::parakeet_document::ParakeetDocumentStore,
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

struct UnifiedInferenceCommand {
    call: whisper_core::parakeet_scheduler::ScheduledCall,
    window: whisper_core::parakeet_session::AudioWindow,
    pcm: Vec<i16>,
}

struct UnifiedInferenceResult {
    call: whisper_core::parakeet_scheduler::ScheduledCall,
    instance_id: u64,
    words: Result<Vec<whisper_core::parakeet_ipc::Word>, String>,
}

#[cfg(test)]
#[derive(Clone, Debug, Eq, PartialEq)]
struct ControlledChildObservation {
    incarnation: u64,
    event: &'static str,
    job_id: Option<JobId>,
    generation: Option<Generation>,
    attempt: Option<u64>,
    wire_request_id: u64,
    retained_job_ids: usize,
}

#[cfg(test)]
fn observe_controlled_child(
    observer: Option<&Arc<Mutex<Vec<ControlledChildObservation>>>>,
    incarnation: u64,
    event: &'static str,
    job_id: Option<JobId>,
    generation: Option<Generation>,
    attempt: Option<u64>,
    wire_request_id: u64,
    retained_job_ids: usize,
) {
    if let Some(observer) = observer {
        observer
            .lock()
            .expect("controlled child observation mutex")
            .push(ControlledChildObservation {
                incarnation,
                event,
                job_id,
                generation,
                attempt,
                wire_request_id,
                retained_job_ids,
            });
    }
}

enum UnifiedExecutorCommand {
    Infer(UnifiedInferenceCommand),
    Stop,
}

struct UnifiedInferenceExecutor {
    commands: SyncSender<UnifiedExecutorCommand>,
    results: Receiver<UnifiedInferenceResult>,
    ready: Receiver<()>,
    ready_seen: bool,
    thread: Option<thread::JoinHandle<Box<dyn crate::parakeet_pipeline::WindowTranscriber>>>,
    worker: Option<Box<dyn crate::parakeet_pipeline::WindowTranscriber>>,
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
            self.next_child_incarnation = self.next_child_incarnation.saturating_add(1);
            self.current_child_incarnation = self.next_child_incarnation;
        }
        #[cfg(test)]
        let child_incarnation = self.current_child_incarnation;
        #[cfg(test)]
        let child_observer = self.child_observer.clone();
        #[cfg(test)]
        let ready_gate = self.ready_gate.clone();
        self.thread = Some(
            thread::Builder::new()
                .name("parakeet-unified-inference".into())
                .spawn(move || {
                    let mut worker = worker;
                    let mut child_jobs = BTreeMap::<u128, (Generation, u64)>::new();
                    let mut wire_request_id = 0_u64;
                    #[cfg(test)]
                    observe_controlled_child(
                        child_observer.as_ref(), child_incarnation, "Spawn", None, None, None,
                        wire_request_id, child_jobs.len(),
                    );
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
                    let _ = ready_tx.send(());
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
                                    worker.transcribe(command.window, &command.pcm)
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
                    worker
                })
                .map_err(|error| format!("InferenceStart: {error}"))?,
        );
        self.commands = commands;
        self.results = results;
        self.ready = ready;
        self.ready_seen = false;
        Ok(())
    }

    fn is_ready(&mut self) -> bool {
        if !self.ready_seen && self.ready.try_recv().is_ok() {
            self.ready_seen = true;
        }
        self.ready_seen
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
        self.commands
            .send(UnifiedExecutorCommand::Stop)
            .map_err(|error| format!("InferenceStop: {error}"))?;
        let thread = self
            .thread
            .take()
            .ok_or_else(|| "InferenceReap: worker instance is missing".to_owned())?;
        self.worker = Some(
            thread
                .join()
                .map_err(|_| "InferenceReap: worker thread panicked".to_owned())?,
        );
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
            Ok(worker) => {
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
}

#[cfg(test)]
struct ScheduledWindowTranscriber<'a> {
    scheduler: &'a mut whisper_core::parakeet_scheduler::ParakeetScheduler,
    call: whisper_core::parakeet_scheduler::ScheduledCall,
    transcriber: &'a mut dyn crate::parakeet_pipeline::WindowTranscriber,
}

#[cfg(test)]
impl crate::parakeet_pipeline::WindowTranscriber for ScheduledWindowTranscriber<'_> {
    fn instance_id(&self) -> u64 {
        self.transcriber.instance_id()
    }

    fn transcribe(
        &mut self,
        window: whisper_core::parakeet_session::AudioWindow,
        pcm: &[i16],
    ) -> Result<Vec<whisper_core::parakeet_ipc::Word>, String> {
        self.scheduler
            .begin_call(self.call)
            .map_err(|error| format!("SchedulerInvoke: {error:?}"))?;
        self.transcriber.transcribe(window, pcm)
    }
}

struct MicrophoneTestCapture {
    job_id: JobId,
    generation: Generation,
    capture: crate::capture::CaptureStream,
    converter: Pcm16Converter,
    probe: crate::microphone_probe::MicrophoneProbe,
    spectrum_reader: crate::microphone_probe::MicrophoneSpectrumReader,
    last_meter_sequence: u64,
    next_spectrum_telemetry: bool,
    last_acknowledged_gain_revision: Option<u64>,
    pending_events: VecDeque<ImportEvent>,
}

impl MicrophoneTestCapture {
    fn start(
        job_id: JobId,
        generation: Generation,
        microphone: &whisper_core::microphone::MicrophoneChoice,
        gain_db: i8,
    ) -> Result<(Self, String), String> {
        let gain = whisper_core::microphone_level::GainDb::new(gain_db)
            .map_err(|error| format!("MicrophoneLevel: {error}"))?;
        let capture = crate::capture::start_direct_with_gain(microphone, gain)?;
        Self::from_capture(job_id, generation, capture)
    }

    fn from_capture(
        job_id: JobId,
        generation: Generation,
        capture: crate::capture::CaptureStream,
    ) -> Result<(Self, String), String> {
        let source_identity = capture
            .source_identity()
            .map(str::to_owned)
            .ok_or_else(|| "MicrophoneLevel: identité du flux ouvert indisponible".to_owned())?;
        let converter = Pcm16Converter::new(capture.sample_rate_hz(), capture.channels())?;
        let probe = crate::microphone_probe::MicrophoneProbe::start(
            capture.sample_rate_hz(),
            Some(source_identity.clone()),
            job_id,
            generation,
        )?;
        Ok((
            Self {
                job_id,
                generation,
                capture,
                converter,
                probe,
                spectrum_reader: crate::microphone_probe::MicrophoneSpectrumReader::default(),
                last_meter_sequence: 0,
                next_spectrum_telemetry: false,
                last_acknowledged_gain_revision: None,
                pending_events: VecDeque::new(),
            },
            source_identity,
        ))
    }

    fn set_gain(
        &self,
        target_identity: &str,
        expected_revision: u64,
        gain: whisper_core::microphone_level::GainDb,
    ) -> Result<whisper_core::microphone_level::GainRevision, String> {
        if self.capture.source_identity() != Some(target_identity) {
            return Err("MicrophoneLevel: source active différente de la cible".into());
        }
        self.capture.set_gain(expected_revision, gain)
    }

    fn pump(&mut self) -> Result<Option<ImportEvent>, String> {
        if self.capture.is_saturated() {
            return Err("Tampon du test microphone saturé".into());
        }
        if self.capture.failure() {
            return Err(self
                .capture
                .failure_detail()
                .unwrap_or_else(|| "Flux du test microphone interrompu".into()));
        }
        for _ in 0..crate::capture::CAPTURE_SLOTS {
            let Some(block) = self.capture.try_next_block() else {
                break;
            };
            let gain = block.gain();
            let first_frame = block.first_frame();
            if self
                .last_acknowledged_gain_revision
                .is_none_or(|revision| gain.revision > revision)
            {
                self.last_acknowledged_gain_revision = Some(gain.revision);
                if let Some(target_identity) = self.capture.source_identity().map(str::to_owned) {
                    self.pending_events
                        .push_back(ImportEvent::MicrophoneGainApplied {
                            job_id: self.job_id,
                            generation: self.generation,
                            target_identity,
                            gain_db: gain.gain.value(),
                            gain_revision: gain.revision,
                            first_frame,
                        });
                }
            }
            let converted = self.converter.push_with_probe(&block)?;
            self.capture.recycle_block(block)?;
            self.probe.submit(converted.probe);
        }
        if let Some(event) = self.pending_events.pop_front() {
            return Ok(Some(event));
        }
        if self.next_spectrum_telemetry {
            self.next_spectrum_telemetry = false;
            if let Some(update) = self.spectrum_reader.poll(&self.probe, Instant::now()) {
                return Ok(Some(ImportEvent::MicrophoneSpectrum(update)));
            }
        } else {
            self.next_spectrum_telemetry = true;
        }
        let Some(meter) = self
            .probe
            .latest_at(Instant::now())
            .and_then(|snapshot| snapshot.meter())
        else {
            return Ok(None);
        };
        if meter.sequence <= self.last_meter_sequence {
            return Ok(None);
        }
        self.last_meter_sequence = meter.sequence;
        Ok(Some(ImportEvent::MicrophoneMeter(meter)))
    }

    fn stop(mut self) -> ImportEvent {
        self.probe.invalidate();
        self.capture.stop();
        ImportEvent::MicrophoneTestStopped {
            job_id: self.job_id,
            generation: self.generation,
        }
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
                if !l3_gpu_candidate_allowed(&key) {
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
            .filter(l3_gpu_candidate_allowed)
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

fn l3_gpu_candidate_allowed(key: &crate::archive::ArchiveQualificationKey) -> bool {
    let test_injection = cfg!(feature = "l3-gpu-qualification")
        && option_env!("PARAKEET_L2_ACCEPTANCE_TEST_BUILD").is_some()
        && key == &crate::archive::ArchiveQualificationKey::parakeet_gpu_test();
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
        self.unified_publication_gates.remove(&session);
        self.gpu_candidates.remove(&(job_id, generation.get()));
        self.parakeet_pending_events
            .retain(|event| pending_unified_event_identity(event) != Some((job_id, generation)));
    }

    fn unified_admission_full(&self) -> bool {
        let mut parent_sessions = std::collections::HashSet::new();
        parent_sessions.extend(self.unified_sessions.keys().copied());
        parent_sessions.extend(self.unified_archives.keys().copied());
        parent_sessions.extend(self.unified_publication_gates.keys().copied());
        parent_sessions.extend(self.document_drains.iter().map(|drain| {
            whisper_core::parakeet_scheduler::SchedulerSession {
                job_id: drain.job_id,
                generation: drain.generation,
            }
        }));
        parent_sessions.len() >= 2 || self.parakeet_scheduler.active_sessions() >= 2
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
    fn new(workers: WorkerPaths, publication_gate: Arc<PublicationGate>) -> Self {
        Self::new_with_catalog(workers, publication_gate, None)
    }

    #[cfg(test)]
    fn new_with_catalog(
        workers: WorkerPaths,
        publication_gate: Arc<PublicationGate>,
        model_catalog: Option<RuntimeModelCatalog>,
    ) -> Self {
        Self::new_with_workers_and_parakeet(workers, publication_gate, model_catalog, None, None)
    }

    fn new_with_workers_and_parakeet(
        workers: WorkerPaths,
        publication_gate: Arc<PublicationGate>,
        model_catalog: Option<RuntimeModelCatalog>,
        parakeet_paths: Option<crate::parakeet_import::ParakeetWorkerPaths>,
        gpu_parakeet_paths: Option<crate::parakeet_import::ParakeetWorkerPaths>,
    ) -> Self {
        Self::new_with_workers_and_parakeet_registry(
            workers,
            publication_gate,
            model_catalog,
            parakeet_paths,
            gpu_parakeet_paths,
            None,
            false,
        )
    }

    fn new_with_workers_and_parakeet_registry(
        workers: WorkerPaths,
        publication_gate: Arc<PublicationGate>,
        model_catalog: Option<RuntimeModelCatalog>,
        parakeet_paths: Option<crate::parakeet_import::ParakeetWorkerPaths>,
        gpu_parakeet_paths: Option<crate::parakeet_import::ParakeetWorkerPaths>,
        snapshot_registry_root: Option<PathBuf>,
        require_snapshot_registry: bool,
    ) -> Self {
        Self {
            workers,
            model_catalog,
            parakeet_paths,
            gpu_parakeet_paths,
            snapshot_registry_root,
            require_snapshot_registry,
            parakeet: None,
            parakeet_events: VecDeque::new(),
            parakeet_expected_gpu: None,
            parakeet_native_call_started: false,
            parakeet_end: None,
            parakeet_stopped_ack: false,
            parakeet_vad: None,
            idle_worker: None,
            pending_idle_worker: None,
            auto_fallback_reason: None,
            diagnostics: None,
            diagnostic_start_failed: false,
            diagnostic_degraded_reported: false,
            progress: None,
            archive: ArchiveStore::new("."),
            source_bindings: HashMap::new(),
            active: None,
            transport: None,
            publication_gate,
            automatic_publication_gates: None,
            unified_publication_gates: HashMap::new(),
            resume_prefix: VecDeque::new(),
            pending_live: None,
            live: None,
            live_archive: None,
            direct_live: None,
            parakeet_live: None,
            microphone_test: None,
            automatic_final_outputs: HashMap::new(),
            parakeet_pending_events: VecDeque::new(),
            pending_attempt_reset: None,
            pending_recovery_counters: None,
            live_pending_persistence: 0,
            live_capture_events_since_worker_poll: 0,
            deferred_live_stopped: None,
            deferred_live_window_finished: None,
            gpu_candidates: std::collections::HashMap::new(),
            parakeet_scheduler: whisper_core::parakeet_scheduler::ParakeetScheduler::default(),
            unified_sessions: HashMap::new(),
            unified_executor: None,
            unified_archives: HashMap::new(),
            #[cfg(test)]
            controlled_transcriber: None,
            #[cfg(test)]
            controlled_captures: VecDeque::new(),
            #[cfg(test)]
            controlled_part_limits: VecDeque::new(),
            #[cfg(test)]
            controlled_disconnect_session: None,
            #[cfg(test)]
            controlled_panic_session: None,
            #[cfg(test)]
            controlled_finish_failure_session: None,
            #[cfg(test)]
            controlled_hold_reserved_call_once: false,
            #[cfg(test)]
            controlled_ready_gate: None,
            #[cfg(test)]
            controlled_child_observer: None,
            #[cfg(test)]
            controlled_skip_child_recycle: false,
            document_drains: VecDeque::new(),
        }
    }

    #[cfg(test)]
    fn with_controlled_transcriber(
        mut self,
        transcriber: Box<dyn crate::parakeet_pipeline::WindowTranscriber>,
    ) -> Self {
        self.controlled_transcriber = Some(transcriber);
        self
    }

    #[cfg(test)]
    fn with_disconnected_result_for(
        mut self,
        session: whisper_core::parakeet_scheduler::SchedulerSession,
    ) -> Self {
        self.controlled_disconnect_session = Some(session);
        self
    }

    #[cfg(test)]
    fn with_panicking_transcriber_for(
        mut self,
        session: whisper_core::parakeet_scheduler::SchedulerSession,
    ) -> Self {
        self.controlled_panic_session = Some(session);
        self
    }

    #[cfg(test)]
    fn with_finish_document_failure_for(
        mut self,
        session: whisper_core::parakeet_scheduler::SchedulerSession,
    ) -> Self {
        self.controlled_finish_failure_session = Some(session);
        self
    }

    #[cfg(test)]
    fn hold_next_reserved_call_once(&mut self) {
        self.controlled_hold_reserved_call_once = true;
    }

    #[cfg(test)]
    fn hold_next_child_ready(&mut self) -> Arc<(Mutex<bool>, std::sync::Condvar)> {
        let gate = Arc::new((Mutex::new(false), std::sync::Condvar::new()));
        self.controlled_ready_gate = Some(Arc::clone(&gate));
        gate
    }

    #[cfg(test)]
    fn with_child_observer(
        mut self,
        observer: Arc<Mutex<Vec<ControlledChildObservation>>>,
    ) -> Self {
        self.controlled_child_observer = Some(observer);
        self
    }

    #[cfg(test)]
    fn with_unrecycled_child_mutant(
        mut self,
        observer: Arc<Mutex<Vec<ControlledChildObservation>>>,
    ) -> Self {
        self.controlled_child_observer = Some(observer);
        self.controlled_skip_child_recycle = true;
        self
    }

    #[cfg(test)]
    fn with_controlled_capture(
        mut self,
        capture: CaptureStream,
        part_limit_bytes: Option<u64>,
    ) -> Self {
        if self.controlled_captures.len() < 3 {
            self.controlled_captures.push_back(capture);
        }
        if self.controlled_part_limits.len() < 3 {
            self.controlled_part_limits.push_back(part_limit_bytes);
        }
        self
    }

    fn start_controlled_unified_import(&mut self, request: &ImportRequest) -> Result<(), String> {
        if self.unified_admission_full() {
            return Err("SchedulerCapacity: two unified sessions are already admitted".into());
        }
        let executor_available = self
            .unified_executor
            .as_ref()
            .is_some_and(UnifiedInferenceExecutor::is_available);
        #[cfg(test)]
        let controlled_worker_available = self.controlled_transcriber.is_some();
        #[cfg(not(test))]
        let controlled_worker_available = false;
        if !controlled_worker_available && !executor_available {
            return Err("ModelNotQualified: controlled worker injection is required".into());
        }
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
                source: UnifiedSessionSource::Closed(source),
                available_samples,
                durable_samples: available_samples,
                source_closed: true,
                document,
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
        let archive = std::mem::replace(&mut self.archive, ArchiveStore::new(&request.destination));
        self.unified_archives.insert(session, archive);
        Ok(())
    }

    fn start_controlled_unified_capture(
        &mut self,
        request: &whisper_core::ports::LiveRequest,
        capture: CaptureStream,
        test_part_limit: Option<u64>,
    ) -> Result<(), String> {
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
        if !controlled_worker_available && !executor_available {
            return Err("ModelNotQualified: controlled worker injection is required".into());
        }
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
                source: UnifiedSessionSource::Capture(capture),
                available_samples: 0,
                durable_samples: 0,
                source_closed: false,
                document,
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
        Ok(())
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
        #[cfg(test)]
        if self.unified_executor.is_none()
            && let Some(worker) = self.controlled_transcriber.take()
        {
            #[cfg(test)]
            let start_result = match self.controlled_child_observer.clone() {
                Some(observer) if self.controlled_ready_gate.is_some() => {
                    UnifiedInferenceExecutor::start_observed_with_ready_gate(
                        worker,
                        observer,
                        self.controlled_ready_gate
                            .take()
                            .expect("ready gate checked"),
                    )
                }
                Some(observer) => UnifiedInferenceExecutor::start_observed(worker, observer),
                None => UnifiedInferenceExecutor::start(worker),
            };
            #[cfg(not(test))]
            let start_result = UnifiedInferenceExecutor::start(worker);
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
                Err(error) => return Some(Err((JobId(0), Generation::first(), error))),
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
            self.document_drains.push_back(UnifiedDocumentDrain {
                document: state.document,
                job_id: session.job_id,
                generation: session.generation,
                revision: result.document_revision,
                total_samples: result.total_samples,
                next_sequence: 1,
                ready_pending: true,
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
                if !executor.is_ready() {
                    state.pending_prepared = Some((call, window, pcm, true));
                    self.unified_sessions.insert(call.session, state);
                    return None;
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
                match executor.dispatch(UnifiedInferenceCommand { call, window, pcm }) {
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
                if !executor.is_ready() {
                    state.pending_prepared = Some((call, window, pcm, false));
                    self.unified_sessions.insert(call.session, state);
                    return None;
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
                match executor.dispatch(UnifiedInferenceCommand { call, window, pcm }) {
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

    fn ensure_source_binding(&mut self, request: &mut ImportRequest) -> Result<(), String> {
        let Some(registry_root) = self.snapshot_registry_root.as_deref() else {
            if self.require_snapshot_registry {
                return Err("SourceSnapshotRegistryUnavailable: application settings root is not configured".into());
            }
            return crate::archive::ArchiveStore::identify_source(request);
        };
        let key = (request.job_id, request.generation.get());
        if !self.source_bindings.contains_key(&key) {
            let binding = crate::source_binding::SourceBinding::acquire(request, registry_root)?;
            self.source_bindings.insert(key, binding);
        }
        let binding = self
            .source_bindings
            .get(&key)
            .expect("binding inserted or already present");
        if binding.original_path() != request.source_path
            || binding.strategy() != request.strategy
            || request
                .source_sha256
                .is_some_and(|sha| sha != binding.source_sha256())
            || request
                .source_samples
                .is_some_and(|samples| samples != binding.source_samples())
        {
            return Err(
                "SourceSnapshotMismatch: stored source binding differs from the request".into(),
            );
        }
        request.source_sha256 = Some(binding.source_sha256());
        request.source_samples = Some(binding.source_samples());
        Ok(())
    }

    fn rebind_source_binding(
        &mut self,
        old: (JobId, Generation),
        new: (JobId, Generation),
    ) -> Result<(), String> {
        if old == new {
            return Ok(());
        }
        if !self.source_bindings.contains_key(&(old.0, old.1.get()))
            && !self.require_snapshot_registry
        {
            return Ok(());
        }
        let binding = self
            .source_bindings
            .remove(&(old.0, old.1.get()))
            .ok_or_else(|| {
                "SourceSnapshotUnavailable: accepted source binding is missing".to_owned()
            })?;
        let binding = binding.rebind(new.0, new.1)?;
        self.source_bindings.insert((new.0, new.1.get()), binding);
        Ok(())
    }

    fn release_source_binding(
        &mut self,
        job_id: JobId,
        generation: Generation,
        destination: &Path,
    ) -> Result<(), String> {
        let Some(registry_root) = self.snapshot_registry_root.as_deref() else {
            return Ok(());
        };
        if let Some(binding) = self.source_bindings.remove(&(job_id, generation.get())) {
            return binding.retire();
        }
        crate::source_binding::SourceBinding::retire_for_identity(
            registry_root,
            destination,
            job_id,
            generation,
        )
    }

    fn active_snapshot_bindings(&self) -> Vec<(JobId, u64)> {
        let mut active = self.source_bindings.keys().copied().collect::<Vec<_>>();
        if let Some(request) = &self.active {
            active.push((request.job_id, request.generation.get()));
        }
        active.sort_unstable_by_key(|identity| (identity.0.0, identity.1));
        active.dedup();
        active
    }

    fn inspect_source_snapshot_recovery(
        &self,
    ) -> Result<Vec<whisper_core::ports::SourceSnapshotRecoveryCandidate>, String> {
        let registry_root = self.snapshot_registry_root.as_deref().ok_or_else(|| {
            "SourceSnapshotRegistryUnavailable: application settings root is not configured"
                .to_owned()
        })?;
        crate::source_binding::SourceBinding::inspect_recovery_candidates(
            registry_root,
            &self.active_snapshot_bindings(),
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

    fn confirm_interrupted_publication(
        &mut self,
        mut prompt: DuplicateImportPrompt,
        choice: DuplicateImportChoice,
        events: &mut VecDeque<ImportEvent>,
    ) {
        let Some(previous) = prompt.resume_from.clone() else {
            return;
        };
        let Some(reserved_basename) = prompt.reserved_export_basename.clone() else {
            events.push_back(ImportEvent::Failed {
                job_id: prompt.request.job_id,
                generation: prompt.request.generation,
                message: "StorageCorrupt: interrupted export prompt is missing its reserved name"
                    .into(),
            });
            return;
        };
        let mut request = prompt.request.clone();
        let result = self
            .bind_import_request(&request)
            .map(|bound| request = bound)
            .and_then(|()| crate::archive::ArchiveStore::identify_source(&mut request))
            .and_then(|()| self.bind_import_request(&request).map(|bound| request = bound))
            .and_then(|()| {
                crate::archive::l04_campaign_assert_archive(Path::new(&request.destination))
            })
            .and_then(|()| {
                let destination = PathBuf::from(&request.destination);
                let source_sha256 = request
                    .source_sha256
                    .as_ref()
                    .map(format_sha256)
                    .ok_or_else(|| "SourceChanged: resume source identity is missing".to_owned())?;
                let same_request = request.job_id == previous.job_id
                    && previous.generation.next() == Some(request.generation)
                    && request.source_path == previous.source_path
                    && request.destination == previous.destination
                    && previous.source_sha256.map(|sha| format_sha256(&sha)).as_deref()
                        == Some(source_sha256.as_str())
                    && previous.source_samples == request.source_samples
                    && request.model_sha256 == previous.model_sha256;
                let source_matches = same_request
                    && source_sha256 == prompt.source_sha256
                    && request.source_samples == Some(prompt.source_samples);
                let current = crate::archive::ArchiveStore::published_imports(
                    &destination,
                    &source_sha256,
                )?;
                if !source_matches || current != prompt.publications {
                    prompt.request = request;
                    prompt.source_sha256 = source_sha256;
                    prompt.source_samples = prompt.request.source_samples.unwrap_or(0);
                    prompt.publications = current;
                    prompt.revalidation_notice = Some(
                        "La source ou les transcriptions existantes ont changé. Vérifiez l'état actuel avant de reprendre.".into(),
                    );
                    events.push_back(ImportEvent::DuplicateImportFound(prompt.clone()));
                    return Ok(None);
                }
                let current_basename =
                    crate::archive::ArchiveStore::suggest_duplicate_export_basename(
                        &destination,
                        &request.source_path,
                    )?;
                if current_basename != prompt.export_basename {
                    prompt.request = request;
                    prompt.export_basename = current_basename;
                    prompt.revalidation_notice = Some(
                        "Le nom proposé n'est plus disponible. Vérifiez le nouveau nom avant de confirmer la reprise.".into(),
                    );
                    events.push_back(ImportEvent::DuplicateImportFound(prompt.clone()));
                    return Ok(None);
                }
                match crate::archive::ArchiveStore::reconfirm_interrupted_publication(
                    &destination,
                    &previous,
                    &source_sha256,
                    prompt.source_samples,
                    &current,
                    choice,
                    &reserved_basename,
                    &prompt.export_basename,
                ) {
                    Ok(()) => Ok(Some(request)),
                    Err(error) if error.starts_with("DuplicateExportNameChanged:") => {
                        prompt.request = request;
                        prompt.export_basename =
                            crate::archive::ArchiveStore::suggest_duplicate_export_basename(
                                &destination,
                                &prompt.request.source_path,
                            )?;
                        prompt.revalidation_notice = Some(
                            "Le nom proposé a été pris pendant la confirmation. Vérifiez ce nouveau nom avant de reprendre.".into(),
                        );
                        events.push_back(ImportEvent::DuplicateImportFound(prompt.clone()));
                        Ok(None)
                    }
                    Err(error) => Err(error),
                }
            });
        match result {
            Ok(Some(request)) => self.effect(
                ImportEffect::ResumeInterrupted {
                    previous: Box::new(previous),
                    request,
                },
                events,
            ),
            Ok(None) => {}
            Err(message) => events.push_back(ImportEvent::Failed {
                job_id: prompt.request.job_id,
                generation: prompt.request.generation,
                message,
            }),
        }
    }

    fn effect(&mut self, effect: ImportEffect, events: &mut VecDeque<ImportEvent>) {
        if matches!(
            &effect,
            ImportEffect::StartLive(_) | ImportEffect::StartUnifiedLive(_)
        ) {
            self.idle_worker = None;
        }
        let persisted_live_segment = match &effect {
            ImportEffect::PersistSegment(segment)
                if self.live.as_ref().is_some_and(|live| {
                    (live.request.job_id, live.request.generation)
                        == (segment.job_id, segment.generation)
                }) =>
            {
                Some((segment.job_id, segment.generation))
            }
            _ => None,
        };
        let unified_terminal_effect = match &effect {
            ImportEffect::PersistSegment(segment) => self.unified_archives.contains_key(
                &whisper_core::parakeet_scheduler::SchedulerSession {
                    job_id: segment.job_id,
                    generation: segment.generation,
                },
            ),
            ImportEffect::Publish {
                job_id, generation, ..
            }
            | ImportEffect::StopUnifiedLive {
                job_id, generation, ..
            } => self.unified_archives.contains_key(
                &whisper_core::parakeet_scheduler::SchedulerSession {
                    job_id: *job_id,
                    generation: *generation,
                },
            ),
            _ => false,
        };
        let mut result = match effect {
            ImportEffect::SetGpuCandidate {
                job_id,
                generation,
                candidate,
            } => {
                self.gpu_candidates
                    .insert((job_id, generation.get()), candidate);
                Ok(())
            }
            ImportEffect::ReadUnifiedDocumentPage {
                job_id,
                generation,
                request_id,
                destination,
                expected_revision,
                first_sequence,
                limit,
            } => {
                let result = ArchiveStore::read_unified_document_page(
                    Path::new(&destination),
                    job_id,
                    generation,
                    expected_revision,
                    first_sequence,
                    limit,
                );
                events.push_back(ImportEvent::UnifiedDocumentPage {
                    job_id,
                    generation,
                    request_id,
                    result,
                });
                Ok(())
            }
            ImportEffect::ReadUnifiedAudioPartsPage {
                job_id,
                generation,
                request_id,
                destination,
                expected_revision,
                offset,
                limit,
            } => {
                let result = ArchiveStore::read_unified_audio_parts_page(
                    Path::new(&destination),
                    job_id,
                    generation,
                    expected_revision,
                    offset,
                    limit,
                );
                events.push_back(ImportEvent::UnifiedAudioPartsPage {
                    job_id,
                    generation,
                    request_id,
                    result,
                });
                Ok(())
            }
            ImportEffect::StartMicrophoneTest {
                job_id,
                generation,
                microphone,
                gain_db,
            } => {
                if self.microphone_test.is_some()
                    || self.direct_live.is_some()
                    || self.parakeet_live.is_some()
                    || self.live.is_some()
                    || self.active.is_some()
                {
                    Err((
                        job_id,
                        generation,
                        "Une capture microphone est déjà ouverte".into(),
                    ))
                } else {
                    match MicrophoneTestCapture::start(job_id, generation, &microphone, gain_db) {
                        Ok((capture, source_identity)) => {
                            self.microphone_test = Some(capture);
                            events.push_back(ImportEvent::MicrophoneTestStarted {
                                job_id,
                                generation,
                                source_identity,
                                gain_db,
                            });
                            Ok(())
                        }
                        Err(error) => Err((job_id, generation, error)),
                    }
                }
            }
            ImportEffect::StartDirectLive(request) => {
                let id = (request.job_id, request.generation);
                let candidate = self.gpu_candidates.remove(&(id.0, id.1.get())).flatten();
                let parakeet = match self.model_catalog.as_ref() {
                    Some(catalog) => match catalog.is_parakeet_live_request(&request) {
                        Ok(is_parakeet) => is_parakeet,
                        Err(error) => {
                            events.push_back(ImportEvent::Failed {
                                job_id: id.0,
                                generation: id.1,
                                message: format!("Modèle indisponible : {error}"),
                            });
                            return;
                        }
                    },
                    None => false,
                };
                let automatic_import_active = parakeet
                    && self.active.as_ref().is_some_and(|active| {
                        self.automatic_final_outputs
                            .contains_key(&(active.job_id, active.generation.get()))
                    });
                if self.publication_gate.is_cancelled() {
                    Ok(())
                } else if (self.active.is_some() && !automatic_import_active)
                    || self.live.is_some()
                    || self.direct_live.is_some()
                    || self.parakeet_live.is_some()
                {
                    Err((id.0, id.1, "Une transcription est déjà en cours".into()))
                } else {
                    if parakeet {
                        let catalog = self.model_catalog.as_ref().expect("catalog checked above");
                        match catalog.bind_parakeet_live(&request, candidate) {
                            Ok((request, paths, candidate)) => {
                                let session_id = NEXT_INSTANCE.fetch_add(1, Ordering::Relaxed);
                                match crate::parakeet_live::ParakeetLive::start(
                                    request, paths, candidate, session_id,
                                ) {
                                    Ok(session) => {
                                        self.parakeet_live = Some(session);
                                        Ok(())
                                    }
                                    Err(error) => Err((id.0, id.1, error)),
                                }
                            }
                            Err(error) => {
                                Err((id.0, id.1, format!("Modèle indisponible : {error}")))
                            }
                        }
                    } else {
                        let request = match self.bind_live_request(&request) {
                            Ok(request) => request,
                            Err(error) => {
                                events.push_back(ImportEvent::Failed {
                                    job_id: id.0,
                                    generation: id.1,
                                    message: format!("Modèle indisponible : {error}"),
                                });
                                return;
                            }
                        };
                        let instance = NEXT_INSTANCE.fetch_add(1, Ordering::Relaxed);
                        let reusable_worker = self.idle_worker.take().and_then(|mut idle| {
                            idle.matches(&request, &self.workers)
                                .then_some(idle.transport)
                        });
                        match crate::direct_live::DirectLive::start(
                            request,
                            &self.workers,
                            instance,
                            reusable_worker,
                            candidate,
                        ) {
                            Ok(session) => {
                                self.direct_live = Some(session);
                                Ok(())
                            }
                            Err(error) => Err((id.0, id.1, error)),
                        }
                    }
                }
            }
            ImportEffect::StartUnifiedLive(request) => {
                let id = (request.job_id, request.generation);
                #[cfg(test)]
                let controlled_capture = self.controlled_captures.pop_front();
                #[cfg(test)]
                let controlled_part_limit = self.controlled_part_limits.pop_front().flatten();
                #[cfg(not(test))]
                let controlled_capture: Option<CaptureStream> = None;
                #[cfg(not(test))]
                let controlled_part_limit: Option<u64> = None;
                if let Some(capture) = controlled_capture {
                    match self.bind_live_request(&request) {
                        Ok(bound) => self
                            .start_controlled_unified_capture(
                                &bound,
                                capture,
                                controlled_part_limit,
                            )
                            .map_err(|error| (id.0, id.1, error)),
                        Err(error) => Err((id.0, id.1, format!("Modèle indisponible : {error}"))),
                    }
                } else {
                    Err((id.0, id.1,
                        "ModelNotQualified: unified capture is not enabled in the native composition".into()))
                }
            }
            ImportEffect::StartLive(request) => {
                let id = (request.job_id, request.generation);
                let candidate = self
                    .gpu_candidates
                    .remove(&(request.job_id, request.generation.get()))
                    .flatten();
                if self.publication_gate.is_cancelled() {
                    Ok(())
                } else {
                    match self.bind_live_request(&request) {
                        Ok(request) => self.start_live(request, candidate),
                        Err(error) => Err((id.0, id.1, format!("Modèle indisponible : {error}"))),
                    }
                }
            }
            ImportEffect::SetMicrophoneGain {
                job_id,
                generation,
                target_identity,
                expected_revision,
                gain_db,
            } => {
                let result = whisper_core::microphone_level::GainDb::new(gain_db)
                    .map_err(|error| error.to_string())
                    .and_then(|gain| {
                        let unified = whisper_core::parakeet_scheduler::SchedulerSession {
                            job_id,
                            generation,
                        };
                        if let Some(state) = self.unified_sessions.get(&unified)
                            && state.source.is_capture()
                            && !state.source_closed
                        {
                            self.unified_sessions
                                .get(&unified)
                                .expect("unified capture was found")
                                .source
                                .set_gain(&target_identity, expected_revision, gain)
                                .map(|_| ())
                        } else if let Some(direct) = self.direct_live.as_ref()
                            && direct.identity() == (job_id, generation)
                        {
                            direct
                                .set_gain(&target_identity, expected_revision, gain)
                                .map(|_| ())
                        } else if let Some(parakeet) = self.parakeet_live.as_ref()
                            && parakeet.identity() == (job_id, generation)
                        {
                            parakeet
                                .set_gain(&target_identity, expected_revision, gain)
                                .map(|_| ())
                        } else if let Some(test) = self.microphone_test.as_ref()
                            && (test.job_id, test.generation) == (job_id, generation)
                        {
                            test.set_gain(&target_identity, expected_revision, gain)
                                .map(|_| ())
                        } else if let Some(live) = self.live.as_ref()
                            && (live.request.job_id, live.request.generation)
                                == (job_id, generation)
                            && live.capture.source_identity() == Some(target_identity.as_str())
                        {
                            live.capture.set_gain(expected_revision, gain).map(|_| ())
                        } else {
                            Err("MicrophoneLevel: capture cible absente ou périmée".into())
                        }
                    });
                match result {
                    Ok(applied) => {
                        let _ = applied;
                        Ok(())
                    }
                    Err(message) => {
                        events.push_back(ImportEvent::MicrophoneGainRejected {
                            job_id,
                            generation,
                            target_identity,
                            message,
                        });
                        Ok(())
                    }
                }
            }
            ImportEffect::Enqueue(mut request) => {
                let submitted = (request.job_id, request.generation);
                let _timing_context = R7TimingContextGuard::enter(
                    "enqueue-import",
                    format!("{:032x}-{}", submitted.0.0, submitted.1.get()),
                    Path::new(&request.destination),
                );
                let started = Instant::now();
                let mut binding_key = submitted;
                let binding_destination = PathBuf::from(&request.destination);
                let result = self
                    .bind_import_request(&request)
                    .map(|bound| {
                        request = bound;
                        ()
                    })
                    .and_then(|()| self.ensure_source_binding(&mut request))
                    .and_then(|()| {
                        self.bind_import_request(&request).map(|bound| {
                            request = bound;
                            ()
                        })
                    })
                    .and_then(|()| {
                        crate::archive::l04_campaign_assert_archive(Path::new(&request.destination))
                    })
                    .and_then(|()| {
                        let destination = Path::new(&request.destination);
                        let _lock = crate::archive::ArchiveStore::lock_publications(destination)?;
                        let source_sha = request
                            .source_sha256
                            .as_ref()
                            .map(format_sha256)
                            .ok_or_else(|| {
                                "SourceChanged: source identity is missing".to_owned()
                            })?;
                        let legacy_export_basename = if self.snapshot_registry_root.is_none() {
                            crate::archive::ArchiveStore::suggest_duplicate_export_basename(
                                destination,
                                &request.source_path,
                            )?
                        } else {
                            String::new()
                        };
                        let publications = match
                            crate::archive::ArchiveStore::reprocessing_catalog(destination, &source_sha)
                        {
                            Ok(publications) => publications,
                            Err(error) if error.starts_with("StorageCorrupt:") => {
                                let source_samples = request.source_samples.unwrap_or(0);
                                return Ok(PreflightResult::Duplicate(DuplicateImportPrompt {
                                    request,
                                    source_sha256: source_sha,
                                    source_samples,
                                    publications: Vec::new(),
                                    export_basename: legacy_export_basename.clone(),
                                    revalidation_notice: Some(
                                        "Certains anciens traitements sont illisibles. Vous pouvez créer un nouveau dossier ; les anciens resteront intacts.".into(),
                                    ),
                                    resume_from: None,
                                    reserved_export_basename: None,
                                }));
                            }
                            Err(error) => return Err(error),
                        };
                        let publications = verified_present_publications(publications);
                        if !publications.is_empty() {
                            let source_samples = request.source_samples.unwrap_or(0);
                            return Ok(PreflightResult::Duplicate(DuplicateImportPrompt {
                                request,
                                source_sha256: source_sha,
                                source_samples,
                                publications,
                                export_basename: legacy_export_basename,
                                revalidation_notice: None,
                                resume_from: None,
                                reserved_export_basename: None,
                            }));
                        }
                        let mut queue = crate::queue_store::QueueStore::open(destination)?;
                        let original = (request.job_id, request.generation);
                        request.job_id = collision_free_job_id(
                            destination,
                            request.job_id,
                            request.generation,
                            &queue.entries(),
                        );
                        let source_sha = request
                            .source_sha256
                            .as_ref()
                            .map(format_sha256)
                            .ok_or_else(|| {
                                "SourceChanged: source identity is missing".to_owned()
                            })?;
                        if self.snapshot_registry_root.is_some() {
                            crate::archive::ArchiveStore::reserve_create_only_new_import(
                                destination,
                                &request,
                                &source_sha,
                            )?;
                        } else {
                            crate::archive::ArchiveStore::reserve_new_import(
                                destination,
                                &request,
                                &source_sha,
                            )?;
                        }
                        if let Err(error) = self
                            .rebind_source_binding(original, (request.job_id, request.generation))
                        {
                            let _ = crate::archive::ArchiveStore::clear_duplicate_reservation(
                                destination,
                                request.job_id,
                                request.generation,
                            );
                            return Err(error);
                        }
                        binding_key = (request.job_id, request.generation);
                        let admitted = (request.job_id, request.generation);
                        if let Err(error) = queue.enqueue(request.clone()) {
                            let _ = crate::archive::ArchiveStore::clear_duplicate_reservation(
                                destination,
                                request.job_id,
                                request.generation,
                            );
                            let _ = self.release_source_binding(
                                request.job_id,
                                request.generation,
                                destination,
                            );
                            return Err(error);
                        }
                        Ok(PreflightResult::Queued {
                            submitted: original,
                            admitted,
                            entries: queue.entries(),
                        })
                    });
                record_r7_timing(
                    "import-admission",
                    started.elapsed(),
                    serde_json::json!({
                        "operation": format!("{:032x}-{}", submitted.0.0, submitted.1.get()),
                        "pid": std::process::id(),
                        "phase": "admission",
                        "outcome": match result.as_ref() {
                            Ok(PreflightResult::Duplicate(_)) => "duplicate",
                            Ok(PreflightResult::Queued { .. }) => "queued",
                            Err(_) => "error",
                        },
                    }),
                );
                match result {
                    Ok(PreflightResult::Duplicate(prompt)) => {
                        events.push_back(ImportEvent::DuplicateImportFound(prompt));
                        Ok(())
                    }
                    Ok(PreflightResult::Queued {
                        submitted,
                        admitted,
                        entries,
                    }) => {
                        if submitted == admitted {
                            events.push_back(ImportEvent::Queue(entries));
                        } else {
                            events.push_back(ImportEvent::QueueAdmission {
                                submitted_job_id: submitted.0,
                                submitted_generation: submitted.1,
                                queue: entries,
                            });
                        }
                        Ok(())
                    }
                    Err(error) => {
                        let _ = self.release_source_binding(
                            binding_key.0,
                            binding_key.1,
                            &binding_destination,
                        );
                        Err((submitted.0, submitted.1, error))
                    }
                }
            }
            ImportEffect::ConfirmDuplicateImport { mut prompt, choice } => {
                if prompt.resume_from.is_some() {
                    self.confirm_interrupted_publication(prompt, choice, events);
                    return;
                }
                let submitted = (prompt.request.job_id, prompt.request.generation);
                let _timing_context = R7TimingContextGuard::enter(
                    "confirm-duplicate-import",
                    format!("{:032x}-{}", submitted.0.0, submitted.1.get()),
                    Path::new(&prompt.request.destination),
                );
                let started = Instant::now();
                let mut request = prompt.request.clone();
                let mut binding_key = submitted;
                let binding_destination = PathBuf::from(&request.destination);
                if choice == DuplicateImportChoice::CreateNew
                    && self.require_snapshot_registry
                    && self.snapshot_registry_root.is_none()
                {
                    events.push_back(ImportEvent::Failed {
                        job_id: submitted.0,
                        generation: submitted.1,
                        message: "SourceSnapshotRegistryUnavailable: Créer un nouveau requiert le registre de snapshots de l'application".into(),
                    });
                    return;
                }
                let result = self
                    .bind_import_request(&request)
                    .map(|bound| { request = bound; () })
                    .and_then(|()| self.ensure_source_binding(&mut request))
                    .and_then(|()| self.bind_import_request(&request).map(|bound| { request = bound; () }))
                    .and_then(|()| {
                        crate::archive::l04_campaign_assert_archive(Path::new(&request.destination))
                    })
                    .and_then(|()| {
                        let destination = Path::new(&request.destination);
                        let _lock = crate::archive::ArchiveStore::lock_publications(destination)?;
                        let source_sha = request
                            .source_sha256
                            .as_ref()
                            .map(format_sha256)
                            .ok_or_else(|| "SourceChanged: source identity is missing".to_owned())?;
                        let is_reprocessing_choice = matches!(
                            &choice,
                            DuplicateImportChoice::ReplaceLatest
                                | DuplicateImportChoice::ReplaceAll
                                | DuplicateImportChoice::ReplaceSelectedPath(_)
                        );
                        let current = if choice == DuplicateImportChoice::CreateNew
                            && self.snapshot_registry_root.is_some()
                        {
                            Vec::new()
                        } else if is_reprocessing_choice {
                            verified_present_publications(
                                crate::archive::ArchiveStore::reprocessing_catalog(destination, &source_sha)?,
                            )
                        } else {
                            crate::archive::ArchiveStore::published_imports(
                                destination,
                                &source_sha,
                            )?
                        };
                        let source_matches = source_sha == prompt.source_sha256
                            && request.source_samples == Some(prompt.source_samples)
                            && request.destination == prompt.request.destination;
                        if !source_matches
                            || ((choice != DuplicateImportChoice::CreateNew
                                || self.snapshot_registry_root.is_none())
                                && current != prompt.publications)
                        {
                            prompt.request = request;
                            prompt.source_sha256 = source_sha;
                            prompt.source_samples = prompt.request.source_samples.unwrap_or(0);
                            prompt.publications = current;
                            prompt.revalidation_notice = Some(
                                "La source ou les transcriptions existantes ont changé. Vérifiez l'état actuel avant de choisir.".into(),
                            );
                            return Ok(PreflightResult::Duplicate(prompt));
                        }
                        let current_basename = if choice == DuplicateImportChoice::Replace {
                            Some(crate::archive::ArchiveStore::suggest_duplicate_export_basename(
                                destination,
                                &request.source_path,
                            )?)
                        } else {
                            None
                        };
                        if current_basename.as_deref().is_some_and(|name| name != prompt.export_basename) {
                            let current_basename = current_basename.expect("checked Some");
                            prompt.export_basename = current_basename;
                            prompt.revalidation_notice = Some(
                                "Le nom de sortie proposé n'est plus disponible. Vérifiez le nouveau nom avant de confirmer.".into(),
                            );
                            return Ok(PreflightResult::Duplicate(prompt));
                        }
                        let mut queue = crate::queue_store::QueueStore::open(destination)?;
                        let original = (request.job_id, request.generation);
                        request.job_id = collision_free_job_id(
                            destination,
                            request.job_id,
                            request.generation,
                            &queue.entries(),
                        );
                        let reservation = if choice == DuplicateImportChoice::CreateNew
                            && self.snapshot_registry_root.is_some()
                        {
                            crate::archive::ArchiveStore::reserve_create_only_import(
                                destination,
                                &request,
                                &source_sha,
                                request.source_samples.unwrap_or(0),
                            )
                        } else if is_reprocessing_choice {
                            match resolve_reprocessing_targets(destination, &current, &choice) {
                                Ok(targets) => crate::archive::ArchiveStore::reserve_reprocessing_import(
                                    destination,
                                    &request,
                                    &source_sha,
                                    request.source_samples.unwrap_or(0),
                                    &current,
                                    &targets,
                                    choice.clone(),
                                ),
                                Err(error) => {
                                    prompt.request = request;
                                    prompt.publications = current;
                                    prompt.revalidation_notice = Some(error);
                                    return Ok(PreflightResult::Duplicate(prompt));
                                }
                            }
                        } else {
                            crate::archive::ArchiveStore::reserve_duplicate_import(
                                destination,
                                &request,
                                &source_sha,
                                request.source_samples.unwrap_or(0),
                                &current,
                                choice.clone(),
                                &prompt.export_basename,
                            )
                        };
                        if let Err(error) = reservation {
                            if error.starts_with("DuplicateExportNameChanged:") {
                                prompt.export_basename = crate::archive::ArchiveStore::suggest_duplicate_export_basename(
                                    destination,
                                    &request.source_path,
                                )?;
                                prompt.revalidation_notice = Some(
                                    "Le nom de sortie proposé n'est plus disponible. Vérifiez le nouveau nom avant de confirmer.".into(),
                                );
                                return Ok(PreflightResult::Duplicate(prompt));
                            }
                            return Err(error);
                        }
                        if let Err(error) = self.rebind_source_binding(original, (request.job_id, request.generation)) {
                            let _ = crate::archive::ArchiveStore::clear_duplicate_reservation(
                                destination,
                                request.job_id,
                                request.generation,
                            );
                            return Err(error);
                        }
                        binding_key = (request.job_id, request.generation);
                        let admitted = (request.job_id, request.generation);
                        if let Err(error) = queue.enqueue(request.clone()) {
                            let _ = crate::archive::ArchiveStore::clear_duplicate_reservation(
                                destination,
                                request.job_id,
                                request.generation,
                            );
                            let _ = self.release_source_binding(request.job_id, request.generation, destination);
                            return Err(error);
                        }
                        Ok(PreflightResult::Queued {
                            submitted: original,
                            admitted,
                            entries: queue.entries(),
                        })
                    });
                record_r7_timing(
                    "import-confirmation",
                    started.elapsed(),
                    serde_json::json!({
                        "operation": format!("{:032x}-{}", submitted.0.0, submitted.1.get()),
                        "pid": std::process::id(),
                        "phase": "confirmation",
                        "choice": match choice {
                            DuplicateImportChoice::CreateNew => "create-new",
                            DuplicateImportChoice::Replace => "replace",
                            DuplicateImportChoice::ReplaceLatest => "replace-latest",
                            DuplicateImportChoice::ReplaceAll => "replace-all",
                            DuplicateImportChoice::ReplaceSelectedPath(_) => "replace-selected",
                        },
                        "outcome": match result.as_ref() {
                            Ok(PreflightResult::Duplicate(_)) => "reconfirmation-required",
                            Ok(PreflightResult::Queued { .. }) => "queued",
                            Err(_) => "error",
                        },
                    }),
                );
                match result {
                    Ok(PreflightResult::Duplicate(prompt)) => {
                        events.push_back(ImportEvent::DuplicateImportFound(prompt));
                        Ok(())
                    }
                    Ok(PreflightResult::Queued {
                        submitted,
                        admitted,
                        entries,
                    }) => {
                        if submitted == admitted {
                            events.push_back(ImportEvent::Queue(entries));
                        } else {
                            events.push_back(ImportEvent::QueueAdmission {
                                submitted_job_id: submitted.0,
                                submitted_generation: submitted.1,
                                queue: entries,
                            });
                        }
                        Ok(())
                    }
                    Err(error) => {
                        let _ = self.release_source_binding(
                            binding_key.0,
                            binding_key.1,
                            &binding_destination,
                        );
                        Err((submitted.0, submitted.1, error))
                    }
                }
            }
            ImportEffect::ReleaseSourceBinding {
                job_id,
                generation,
                destination,
            } => self
                .release_source_binding(job_id, generation, Path::new(&destination))
                .map_err(|error| (job_id, generation, error)),
            ImportEffect::InspectSourceSnapshotRecovery => {
                let candidates = self.inspect_source_snapshot_recovery();
                match candidates {
                    Ok(candidates) => {
                        events.push_back(ImportEvent::SourceSnapshotRecoveryInspected(candidates))
                    }
                    Err(error) => events.push_back(ImportEvent::SourceSnapshotRecoveryCompleted {
                        freed: 0,
                        candidates: Vec::new(),
                        error: Some(error),
                    }),
                }
                Ok(())
            }
            ImportEffect::ConfirmSourceSnapshotRecovery => {
                let result = self.snapshot_registry_root.as_deref()
                    .ok_or_else(|| "SourceSnapshotRegistryUnavailable: application settings root is not configured".to_owned())
                    .and_then(|root| crate::source_binding::SourceBinding::recover_abandoned(root, &self.active_snapshot_bindings()));
                match result {
                    Ok((freed, candidates)) => {
                        events.push_back(ImportEvent::SourceSnapshotRecoveryCompleted {
                            freed,
                            candidates,
                            error: None,
                        })
                    }
                    Err(error) => {
                        let candidates =
                            self.inspect_source_snapshot_recovery().unwrap_or_default();
                        events.push_back(ImportEvent::SourceSnapshotRecoveryCompleted {
                            freed: 0,
                            candidates,
                            error: Some(error),
                        });
                    }
                }
                Ok(())
            }
            ImportEffect::RemoveQueued {
                job_id,
                generation,
                destination,
            } => {
                match crate::archive::l04_campaign_assert_archive(Path::new(&destination))
                    .and_then(|()| crate::queue_store::QueueStore::open(&destination))
                    .and_then(|mut queue| {
                        queue.remove(job_id, generation)?;
                        Ok(queue.entries())
                    }) {
                    Ok(queue) => {
                        match self.release_source_binding(
                            job_id,
                            generation,
                            Path::new(&destination),
                        ) {
                            Ok(()) => {
                                events.push_back(ImportEvent::Queue(queue));
                                Ok(())
                            }
                            Err(error) => Err((job_id, generation, error)),
                        }
                    }
                    Err(error) => Err((job_id, generation, error)),
                }
            }
            ImportEffect::ProcessQueued(request) => {
                let id = (request.job_id, request.generation);
                let _timing_context = R7TimingContextGuard::enter(
                    "process-queued-import",
                    format!("{:032x}-{}", id.0.0, id.1.get()),
                    Path::new(&request.destination),
                );
                let model_catalog = self.model_catalog.as_ref().map(|catalog| {
                    (
                        catalog.snapshot(),
                        catalog.known_historical_turbo_path.clone(),
                    )
                });
                let result =
                    crate::archive::l04_campaign_assert_archive(Path::new(&request.destination))
                        .and_then(|()| crate::queue_store::QueueStore::open(&request.destination))
                        .and_then(|mut queue| {
                            let entries = queue.entries();
                            let first = entries
                                .first()
                                .map(|entry| (entry.request.job_id, entry.request.generation));
                            let entry = entries
                                .into_iter()
                                .find(|entry| {
                                    entry.request.job_id == id.0 && entry.request.generation == id.1
                                })
                                .ok_or_else(|| {
                                    "InvalidInput: queue entry no longer exists".to_owned()
                                })?;
                            if first != Some(id) {
                                return Err(
                                    "InvalidInput: process queued jobs in FIFO order".into()
                                );
                            }
                            let same_model = if request_is_parakeet(&entry.request)
                                && request_is_parakeet(&request)
                            {
                                entry.request.model_sha256 == request.model_sha256
                            } else if let Some((snapshot, known_path)) = &model_catalog {
                                let durable_id = snapshot.resolve_model_identity(
                                    &entry.request.model_path,
                                    known_path,
                                )?;
                                let requested_id = snapshot
                                    .resolve_model_identity(&request.model_path, known_path)?;
                                durable_id == requested_id
                            } else {
                                false
                            };
                            if !queued_request_identity_matches(
                                &entry.request,
                                &request,
                                same_model,
                            ) {
                                return Err(
                            "SourceChanged: queued request identity differs from the durable entry"
                                .into(),
                        );
                            }
                            if !matches!(
                                entry.status,
                                crate::queue_store::QueueStatus::Queued
                                    | crate::queue_store::QueueStatus::AwaitingChoice
                            ) {
                                return Err(
                            "InvalidInput: interrupted jobs require an explicit verified resume"
                                .into(),
                        );
                            }
                            let gpu_candidate = self
                                .gpu_candidates
                                .get(&(id.0, id.1.get()))
                                .and_then(Option::as_ref);
                            validate_current_queued_gpu_candidate(&request, gpu_candidate)?;
                            let prevalidated_gpu_executable =
                                if request_is_parakeet(&request)
                                    && request.config.compute == whisper_core::ComputeChoice::Gpu
                                {
                                    let executable = self.bind_import_request(&request)?;
                                    if !self.is_controlled_bounded_test(&request) {
                                        let qualification = archive_qualification_for_request(
                                            &request,
                                            self.parakeet_paths.as_ref(),
                                            self.gpu_parakeet_paths.as_ref(),
                                            gpu_candidate,
                                        );
                                        validate_queued_gpu_preflight(
                                            &request,
                                            gpu_candidate,
                                            qualification.as_ref(),
                                        )?;
                                    }
                                    crate::parakeet_import::expected_gpu_for_candidate(
                                gpu_candidate.expect("current manual GPU proof was checked"),
                            )
                            .map_err(|error| {
                                format!("BackendMismatch: GPU identity validation failed: {error}")
                            })?;
                                    Some(executable)
                                } else {
                                    None
                                };
                            let durable_request = queue
                                .set_pending_compute(&entry.request, request.config.compute)?
                                .request;
                            if let Some(executable) = prevalidated_gpu_executable {
                                Ok((durable_request, executable))
                            } else if request_is_parakeet(&durable_request) {
                                self.bind_import_request(&durable_request)
                                    .map(|executable| (durable_request, executable))
                            } else if let Some((snapshot, known_path)) = &model_catalog {
                                queue
                                    .resolve_queued_with_catalog(id.0, id.1, snapshot, known_path)
                                    .map(|executable| (durable_request, executable))
                            } else {
                                Ok((durable_request.clone(), durable_request))
                            }
                        });
                match result {
                    Ok((mut durable_request, verified_executable)) => {
                        let binding_result = self.ensure_source_binding(&mut durable_request);
                        if let Err(error) = binding_result {
                            Err((id.0, id.1, error))
                        } else {
                            let executable_model_path = model_catalog
                                .as_ref()
                                .map(|_| verified_executable.model_path.as_str());
                            let mut archive = ArchiveStore::new(&durable_request.destination);
                            let qualification = archive_qualification_for_request(
                                &durable_request,
                                self.parakeet_paths.as_ref(),
                                self.gpu_parakeet_paths.as_ref(),
                                self.gpu_candidates
                                    .get(&(id.0, id.1.get()))
                                    .and_then(Option::as_ref),
                            );
                            let prepared = if let Some(binding) =
                                self.source_bindings.get(&(id.0, id.1.get()))
                            {
                                archive.prepare_with_source_binding(
                                    durable_request.clone(),
                                    executable_model_path,
                                    qualification,
                                    binding,
                                )
                            } else if self.require_snapshot_registry {
                                Err("SourceSnapshotUnavailable: binding missing before prepare"
                                    .into())
                            } else {
                                archive.prepare_with_qualification(
                                    durable_request.clone(),
                                    executable_model_path,
                                    qualification,
                                )
                            };
                            match prepared {
                                Ok(prepared) => {
                                    let queue_result =
                                        crate::queue_store::QueueStore::open(&prepared.destination)
                                            .and_then(|mut queue| {
                                                queue.set_status(
                                                    id.0,
                                                    id.1,
                                                    crate::queue_store::QueueStatus::Running,
                                                )?;
                                                Ok(queue.entries())
                                            });
                                    match queue_result {
                                        Ok(queue) => {
                                            self.archive = archive;
                                            events.push_back(ImportEvent::Queue(queue));
                                            events.push_back(ImportEvent::Prepared(prepared));
                                            Ok(())
                                        }
                                        Err(error) => Err((id.0, id.1, error)),
                                    }
                                }
                                Err(error) => Err((id.0, id.1, error)),
                            }
                        }
                    }
                    Err(error) => Err((id.0, id.1, error)),
                }
            }
            ImportEffect::ResumeInterrupted { previous, request } => {
                let id = (request.job_id, request.generation);
                let model_catalog = self.model_catalog.as_ref().map(|catalog| {
                    (
                        catalog.snapshot(),
                        catalog.known_historical_turbo_path.clone(),
                    )
                });
                let queue_validation =
                    crate::archive::l04_campaign_assert_archive(Path::new(&request.destination))
                        .and_then(|()| crate::queue_store::QueueStore::open(&request.destination))
                        .and_then(|queue| {
                            if request_is_parakeet(&request) && request_is_parakeet(&previous) {
                                let mut comparable = request.clone();
                                comparable.model_path = previous.model_path.clone();
                                queue.validate_resume(
                                    previous.job_id,
                                    previous.generation,
                                    &comparable,
                                )
                            } else if let Some((snapshot, known_path)) = &model_catalog {
                                queue.validate_resume_with_catalog(
                                    previous.job_id,
                                    previous.generation,
                                    &request,
                                    snapshot,
                                    known_path,
                                )
                            } else {
                                queue.validate_resume(
                                    previous.job_id,
                                    previous.generation,
                                    &request,
                                )
                            }
                        });
                if let Err(error) = queue_validation {
                    events.push_back(ImportEvent::Failed {
                        job_id: id.0,
                        generation: id.1,
                        message: error,
                    });
                    return;
                }
                let destination = Path::new(&request.destination);
                match crate::archive::ArchiveStore::publication_reconfirmation_required(
                    &destination,
                    previous.job_id,
                    previous.generation,
                ) {
                    Ok(Some(reserved_export_basename)) => {
                        let _lock =
                            match crate::archive::ArchiveStore::lock_publications(destination) {
                                Ok(lock) => lock,
                                Err(error) => {
                                    events.push_back(ImportEvent::Failed {
                                        job_id: id.0,
                                        generation: id.1,
                                        message: error,
                                    });
                                    return;
                                }
                            };
                        let source_sha256 = match previous.source_sha256 {
                            Some(sha) => format_sha256(&sha),
                            None => {
                                events.push_back(ImportEvent::Failed {
                                    job_id: id.0,
                                    generation: id.1,
                                    message: "SourceChanged: resume source identity is missing"
                                        .into(),
                                });
                                return;
                            }
                        };
                        let publications = match crate::archive::ArchiveStore::published_imports(
                            destination,
                            &source_sha256,
                        ) {
                            Ok(publications) => publications,
                            Err(message) => {
                                events.push_back(ImportEvent::Failed {
                                    job_id: id.0,
                                    generation: id.1,
                                    message,
                                });
                                return;
                            }
                        };
                        let export_basename =
                            match crate::archive::ArchiveStore::suggest_duplicate_export_basename(
                                destination,
                                &previous.source_path,
                            ) {
                                Ok(name) => name,
                                Err(message) => {
                                    events.push_back(ImportEvent::Failed {
                                        job_id: id.0,
                                        generation: id.1,
                                        message,
                                    });
                                    return;
                                }
                            };
                        let Some(source_samples) = previous.source_samples else {
                            events.push_back(ImportEvent::Failed {
                                job_id: id.0,
                                generation: id.1,
                                message: "SourceChanged: resume source duration is missing".into(),
                            });
                            return;
                        };
                        events.push_back(ImportEvent::DuplicateImportFound(DuplicateImportPrompt {
                            request,
                            source_sha256,
                            source_samples,
                            publications,
                            export_basename,
                            revalidation_notice: Some(format!(
                                "Une sortie interrompue au nom « {reserved_export_basename} » ne peut pas être attribuée sans risque. Confirmez explicitement ce nouveau nom pour reprendre ; l'ancienne sortie et les traitements existants sont conservés."
                            )),
                            resume_from: Some(*previous),
                            reserved_export_basename: Some(reserved_export_basename),
                        }));
                        return;
                    }
                    Ok(None) => {}
                    Err(message) => {
                        events.push_back(ImportEvent::Failed {
                            job_id: id.0,
                            generation: id.1,
                            message,
                        });
                        return;
                    }
                }
                // Keep the historical model identity in the durable checkpoint. The execution
                // request is rebound to the verified catalog only after the checkpoint exists.
                let archive_destination = request.destination.clone();
                let mut durable_request = request;
                if model_catalog.is_some() {
                    durable_request.model_path = previous.model_path.clone();
                }
                let executable_model_path = if let Some((snapshot, known_path)) = &model_catalog {
                    match if request_is_parakeet(&durable_request) {
                        self.bind_import_request(&durable_request)
                    } else {
                        snapshot.bind_turbo_import_request(&durable_request, known_path)
                    } {
                        Ok(request) => request.model_path,
                        Err(error) => {
                            events.push_back(ImportEvent::Failed {
                                job_id: id.0,
                                generation: id.1,
                                message: format!("Modèle indisponible : {error}"),
                            });
                            return;
                        }
                    }
                } else {
                    durable_request.model_path.clone()
                };
                let executable_model_path = model_catalog
                    .as_ref()
                    .map(|_| executable_model_path.as_str());
                let mut archive = ArchiveStore::new(&archive_destination);
                let result = archive
                    .resume_from_with_qualification(
                        &previous,
                        durable_request.clone(),
                        executable_model_path,
                        archive_qualification_for_request(
                            &durable_request,
                            self.parakeet_paths.as_ref(),
                            self.gpu_parakeet_paths.as_ref(),
                            self.gpu_candidates
                                .get(&(id.0, id.1.get()))
                                .and_then(Option::as_ref),
                        ),
                    )
                    .and_then(|(prepared, prefix)| {
                        crate::queue_store::QueueStore::open(&prepared.destination).and_then(
                            |mut queue| {
                                if request_is_parakeet(&prepared) {
                                    queue.resume_interrupted(
                                        previous.job_id,
                                        previous.generation,
                                        prepared.clone(),
                                    )?;
                                } else if let Some((snapshot, known_path)) = &model_catalog {
                                    queue.resume_interrupted_with_catalog(
                                        previous.job_id,
                                        previous.generation,
                                        prepared.clone(),
                                        snapshot,
                                        known_path,
                                    )?;
                                } else {
                                    queue.resume_interrupted(
                                        previous.job_id,
                                        previous.generation,
                                        prepared.clone(),
                                    )?;
                                }
                                Ok((prepared, prefix, queue.entries()))
                            },
                        )
                    });
                match result {
                    Ok((prepared, prefix, queue)) => {
                        let confirmed_segments = prefix.len() as u64;
                        let confirmed_offset = if prepared.source_samples.is_some_and(|samples| {
                            samples > whisper_core::parakeet_ipc::PARAKEET_MAX_AUDIO_SAMPLES as u64
                        }) {
                            archive.confirmed_import_vad_offset().unwrap_or(0)
                        } else {
                            prefix.last().map_or(0, |segment| segment.range.end_sample)
                        };
                        self.archive = archive;
                        self.resume_prefix = prefix.into();
                        events.push_back(ImportEvent::Queue(queue));
                        events.push_back(ImportEvent::ResumeCheckpoint {
                            job_id: id.0,
                            generation: id.1,
                            confirmed_segments,
                            confirmed_offset,
                        });
                        events.push_back(ImportEvent::Prepared(prepared));
                        Ok(())
                    }
                    Err(error) => Err((id.0, id.1, error)),
                }
            }
            ImportEffect::ScanQueue { destination } => {
                match crate::recovery::scan(Path::new(&destination)) {
                    Ok(snapshot) => {
                        events.push_back(ImportEvent::Queue(snapshot.queue));
                        Ok(())
                    }
                    Err(error) => Err((JobId(0), Generation::first(), error)),
                }
            }
            ImportEffect::Prepare(request) => match self.bind_import_request(&request) {
                Ok(request) => {
                    let candidate = self
                        .gpu_candidates
                        .get(&(request.job_id, request.generation.get()))
                        .and_then(Clone::clone);
                    self.prepare(request, candidate, events)
                }
                Err(error) => Err((
                    request.job_id,
                    request.generation,
                    format!("Modèle indisponible : {error}"),
                )),
            },
            ImportEffect::StartWorker(request) => {
                if request.strategy
                    == whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
                {
                    #[cfg(test)]
                    {
                        let identity = (request.job_id, request.generation);
                        if self.controlled_transcriber.is_none() && self.unified_executor.is_none()
                        {
                            Err((identity.0, identity.1,
                                "ModelNotQualified: unified strategy requires the injected controlled worker".into()))
                        } else {
                            match self.bind_import_request(&request) {
                                Ok(bound) => self
                                    .start_controlled_unified_import(&bound)
                                    .map_err(|error| (identity.0, identity.1, error)),
                                Err(error) => Err((identity.0, identity.1, error)),
                            }
                        }
                    }
                    #[cfg(not(test))]
                    {
                        Err((request.job_id, request.generation,
                            "ModelNotQualified: unified Parakeet strategy is not enabled in the native composition".into()))
                    }
                } else {
                    let candidate = self
                        .gpu_candidates
                        .remove(&(request.job_id, request.generation.get()));
                    match self.bind_import_request(&request) {
                        Ok(request) => self.start_worker_with_selection(request, candidate),
                        Err(error) => Err((
                            request.job_id,
                            request.generation,
                            format!("Modèle indisponible : {error}"),
                        )),
                    }
                }
            }
            ImportEffect::StartAutomaticFinalTranscription(automatic) => {
                let request = automatic.request.clone();
                let identity = (request.job_id, request.generation);
                let map_identity = (identity.0, identity.1.get());
                if self.publication_gate.is_cancelled() {
                    events.push_back(ImportEvent::Stopped {
                        job_id: identity.0,
                        generation: identity.1,
                    });
                    Ok(())
                } else if self.automatic_final_outputs.contains_key(&map_identity)
                    || Path::new(&automatic.outputs.txt_path).exists()
                    || Path::new(&automatic.outputs.srt_path).exists()
                {
                    Err((
                        request.job_id,
                        request.generation,
                        "AutomaticFinalExists: une sortie automatique existe déjà".into(),
                    ))
                } else {
                    (|| {
                        let destination = Path::new(&request.destination);
                        let queue = inspect_automatic_final_queue(destination)
                            .map_err(|error| (identity.0, identity.1, error))?;
                        let identity_in_use = crate::archive::ArchiveStore::job_identity_exists(
                            destination,
                            request.job_id,
                            request.generation,
                        ) || queue.iter().any(|entry| {
                            entry.request.job_id == request.job_id
                                && entry.request.generation == request.generation
                        });
                        if identity_in_use {
                            return Err((
                                identity.0,
                                identity.1,
                                "AutomaticFinalIdentityExists: identifiant déjà utilisé".into(),
                            ));
                        }
                        match set_automatic_final_manifest_status(&automatic, "pending", None, None)
                        {
                            Err(error) => Err((identity.0, identity.1, error)),
                            Ok(()) => {
                                self.automatic_final_outputs
                                    .insert(map_identity, automatic.clone());
                                self.gpu_candidates.insert(
                                    (identity.0, identity.1.get()),
                                    automatic.gpu_candidate,
                                );
                                match self.bind_import_request(&request) {
                                    Ok(request) => {
                                        let candidate = self
                                            .gpu_candidates
                                            .get(&(request.job_id, request.generation.get()))
                                            .and_then(Clone::clone);
                                        self.prepare(request, candidate, events)
                                    }
                                    Err(error) => {
                                        self.gpu_candidates.remove(&(identity.0, identity.1.get()));
                                        Err((identity.0, identity.1, error))
                                    }
                                }
                            }
                        }
                    })()
                }
            }
            ImportEffect::ScanHistory { destination } => {
                let root = PathBuf::from(destination);
                let reconciliation = ArchiveStore::reconcile_publications(&root);
                let mut history = scan_archive_history(&root);
                if let Err(error) = reconciliation {
                    history.push(ArchiveHistoryItem {
                        job_id: JobId(0),
                        generation: Generation::first(),
                        complete: false,
                        source_sha256: String::new(),
                        diagnostic: Some(error),
                        live_recovery: None,
                    });
                }
                events.push_back(ImportEvent::History(history));
                match ArchiveStore::replacement_cleanups(&root) {
                    Ok(cleanups) => events.push_back(ImportEvent::ReplacementCleanups(cleanups)),
                    Err(error) => events.push_back(ImportEvent::ReplacementCleanups(vec![
                        whisper_core::ports::ReplacementCleanupInfo {
                            cleanup_id: "unavailable".into(),
                            successor: TreatmentIdentity {
                                destination: root.to_string_lossy().into_owned(),
                                job_id: JobId(0),
                                generation: Generation::first(),
                            },
                            remaining: Vec::new(),
                            preserved: Vec::new(),
                            last_error: Some(error),
                        },
                    ])),
                }
                Ok(())
            }
            ImportEffect::ResumeReplacementCleanup {
                destination,
                cleanup_id,
            } => {
                let root = PathBuf::from(destination);
                let result = ArchiveStore::resume_replacement_cleanup(&root, &cleanup_id);
                let mut cleanups = ArchiveStore::replacement_cleanups(&root).unwrap_or_default();
                if let Err(error) = result
                    && let Some(cleanup) = cleanups
                        .iter_mut()
                        .find(|item| item.cleanup_id == cleanup_id)
                {
                    cleanup.last_error = Some(error);
                }
                events.push_back(ImportEvent::ReplacementCleanups(cleanups));
                Ok(())
            }
            ImportEffect::ResolveImportPublication {
                destination,
                job_id,
                generation,
            } => {
                let root = PathBuf::from(destination);
                let resolution =
                    ArchiveStore::resolve_import_publication(&root, job_id, generation);
                let mut history = scan_archive_history(&root);
                if let Err(error) = resolution
                    && let Some(item) = history
                        .iter_mut()
                        .find(|item| item.job_id == job_id && item.generation == generation)
                {
                    item.diagnostic = Some(error);
                }
                events.push_back(ImportEvent::History(history));
                Ok(())
            }
            ImportEffect::ResolveLivePublication(request) => {
                let root = PathBuf::from(&request.destination);
                let resolution = ArchiveStore::resolve_live_publication(&root, &request);
                let mut history = scan_archive_history(&root);
                if let Err(error) = resolution
                    && let Some(item) = history.iter_mut().find(|item| {
                        item.job_id == request.job_id && item.generation == request.generation
                    })
                {
                    item.diagnostic = Some(error);
                }
                events.push_back(ImportEvent::History(history));
                Ok(())
            }
            ImportEffect::PersistSegment(segment) => {
                let id = (segment.job_id, segment.generation);
                let sequence = segment.segment_id.0;
                self.persist_segment(segment, events).and_then(|()| {
                    let confirmation = self
                        .parakeet_vad
                        .as_ref()
                        .and_then(|run| run.pending_confirmation)
                        .filter(|(_, last_sequence)| *last_sequence == sequence);
                    if let Some((window, last_sequence)) = confirmation {
                        self.confirm_and_advance_parakeet_vad_window(window, last_sequence)
                            .map_err(|error| (id.0, id.1, error))?;
                    }
                    l2a2_acceptance_barrier(
                        "resume-prefix-confirmed",
                        self.archive.root_path().as_path(),
                    )
                    .map_err(|error| (id.0, id.1, error))
                })
            }
            ImportEffect::Publish {
                job_id,
                generation,
                last_sequence,
            } => self.publish(job_id, generation, last_sequence, events),
            ImportEffect::FinishLiveOnCpu { job_id, generation } => {
                match self.finish_live_on_cpu(job_id, generation) {
                    Ok(event) => {
                        events.push_back(event);
                        Ok(())
                    }
                    Err(error) => Err(error),
                }
            }
            ImportEffect::StopUnifiedLive { job_id, generation } => {
                let session =
                    whisper_core::parakeet_scheduler::SchedulerSession { job_id, generation };
                let publication_gate = self.unified_publication_gates.get(&session).cloned();
                let stopped = if let Some(publication_gate) = publication_gate {
                    match publication_gate.request_stop() {
                        Ok(_) => {
                            let stopped =
                                self.stop_controlled_unified_capture(job_id, generation, events);
                            publication_gate.rollback_stop();
                            stopped
                        }
                        Err(error) => Err(format!("Publication: {error:?}")),
                    }
                } else {
                    self.stop_controlled_unified_capture(job_id, generation, events)
                };
                stopped.map_err(|error| (job_id, generation, error))
            }
            ImportEffect::Stop { job_id, generation } => {
                if let Some(mut live) = self.parakeet_live.take() {
                    if live.identity() != (job_id, generation) {
                        self.parakeet_live = Some(live);
                        Err((job_id, generation, "Arrêt Parakeet périmé".into()))
                    } else {
                        match live.stop() {
                            Ok(event) => {
                                events.push_back(attach_automatic_final_import(&live, event));
                                Ok(())
                            }
                            Err(error) => Err((job_id, generation, error)),
                        }
                    }
                } else {
                    let active_request = self.active.as_ref().and_then(|active| {
                        (active.job_id == job_id && active.generation == generation)
                            .then(|| active.import_request.clone())
                            .flatten()
                    });
                    if active_request.is_some()
                        && let Some(mut worker) = self.parakeet.take()
                    {
                        let termination = worker.terminate();
                        // The child has been detached from this runtime. Settle its
                        // in-memory identity before any fallible queue journal I/O.
                        self.active = None;
                        self.parakeet_events.clear();
                        self.parakeet_end = None;
                        self.parakeet_stopped_ack = false;
                        if let Some(mut run) = self.parakeet_vad.take()
                            && let Some(path) = run.current_path.take()
                        {
                            let _ = fs::remove_file(path);
                        }
                        self.progress = None;
                        match termination {
                            Ok(_status) => {
                                let queue_update = if let Some(request) = active_request {
                                    let queue_journal = Path::new(&request.destination)
                                        .join("transcriptions")
                                        .join("queue.jsonl");
                                    if queue_journal.is_file() {
                                        crate::queue_store::QueueStore::open(&request.destination)
                                            .and_then(|mut queue| {
                                                if queue.entries().iter().any(|entry| {
                                                    entry.request.job_id == job_id
                                                        && entry.request.generation == generation
                                                }) {
                                                    queue.set_status(
                                                    job_id,
                                                    generation,
                                                    crate::queue_store::QueueStatus::Interrupted,
                                                )?;
                                                    Ok(Some(queue.entries()))
                                                } else {
                                                    Ok(None)
                                                }
                                            })
                                            .map_err(|error| (job_id, generation, error))
                                    } else {
                                        Ok(None)
                                    }
                                } else {
                                    Ok(None)
                                };
                                match queue_update {
                                    Err(error) => Err(error),
                                    Ok(queue) => {
                                        if let Some(entries) = queue {
                                            events.push_back(ImportEvent::Queue(entries));
                                        }
                                        events
                                            .push_back(ImportEvent::Stopped { job_id, generation });
                                        Ok(())
                                    }
                                }
                            }
                            Err(error) => Err((job_id, generation, error)),
                        }
                    } else if let Some(direct) = self.direct_live.as_mut() {
                        if direct.identity() != (job_id, generation) {
                            Err((job_id, generation, "Arrêt micro périmé".into()))
                        } else {
                            direct
                                .stop()
                                .map_err(|error| (job_id, generation, error))
                                .map(|()| {
                                    events.push_back(ImportEvent::LiveFinalizing {
                                        job_id,
                                        generation,
                                    });
                                })
                        }
                    } else {
                        if let Some(progress) = self.progress.as_mut() {
                            let now = std::time::Instant::now();
                            progress.close(ProgressPhase::Capturing);
                            progress.open(ProgressPhase::Draining, now);
                            progress.open(ProgressPhase::Cancelling, now);
                        }
                        let live = self.live.as_ref().is_some_and(|live| {
                            (live.request.job_id, live.request.generation) == (job_id, generation)
                        });
                        self.stop(job_id, generation, events).map(|()| {
                            if live && self.live.is_some() {
                                events
                                    .push_back(ImportEvent::LiveFinalizing { job_id, generation });
                            }
                        })
                    }
                }
            }
            ImportEffect::StopMicrophoneTest { job_id, generation } => {
                match self.microphone_test.take() {
                    Some(test) if (test.job_id, test.generation) == (job_id, generation) => {
                        events.push_back(test.stop());
                        Ok(())
                    }
                    Some(test) => {
                        self.microphone_test = Some(test);
                        Err((job_id, generation, "Arrêt du test microphone périmé".into()))
                    }
                    None => Err((job_id, generation, "Test microphone absent".into())),
                }
            }
        };
        if persisted_live_segment.is_some() {
            self.live_pending_persistence = self.live_pending_persistence.saturating_sub(1);
            if self.live_pending_persistence == 0
                && let Some(window) = self.deferred_live_window_finished.take()
            {
                match self.confirm_live_window(window) {
                    Ok(event) => events.push_back(event),
                    Err(failure) => result = Err(failure),
                }
            }
        }
        if let Err((job, generation, mut message)) = result {
            if let Some(automatic) = self.automatic_final_outputs.get(&(job, generation.get()))
                && let Err(status_error) = set_automatic_final_manifest_status(
                    automatic,
                    "failed_or_unavailable",
                    None,
                    Some(&message),
                )
            {
                message.push_str(&format!(
                    "; persistance du verdict d'échec dans le JSON compagnon impossible: {status_error}"
                ));
            }
            self.automatic_final_outputs
                .remove(&(job, generation.get()));
            self.gpu_candidates.remove(&(job, generation.get()));
            let message = if let Some(direct) = self.direct_live.as_mut()
                && direct.identity() == (job, generation)
            {
                let message = direct.fail(message);
                self.direct_live = None;
                message
            } else {
                message
            };
            let mut deferred_live_failure = false;
            let mut deferred_unified_failure = false;
            if unified_terminal_effect {
                let session = whisper_core::parakeet_scheduler::SchedulerSession {
                    job_id: job,
                    generation,
                };
                if let Some(state) = self.unified_sessions.get_mut(&session) {
                    state.failure = Some(message.clone());
                    let _ = self.parakeet_scheduler.abort_session(session);
                    deferred_unified_failure = true;
                } else {
                    self.release_unified_terminal_state(job, generation);
                }
            }
            if let Some(live) = self.live.as_mut()
                && (live.request.job_id, live.request.generation) == (job, generation)
            {
                deferred_live_failure = true;
                live.stop_capture();
                let _ = live.staging.drain();
                live.failure = Some(message.clone());
                if let Some(transport) = self.transport.as_mut() {
                    let _ = transport.try_request(WorkerCommand::Stop {
                        job_id: job,
                        generation,
                    });
                }
            }
            if !deferred_live_failure && !deferred_unified_failure {
                events.push_back(ImportEvent::Failed {
                    job_id: job,
                    generation,
                    message,
                });
            }
        }
    }
    fn start_live(
        &mut self,
        request: LiveRequest,
        candidate: Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
    ) -> Result<(), (JobId, Generation, String)> {
        let id = (request.job_id, request.generation);
        crate::archive::l04_campaign_assert_archive(Path::new(&request.destination))
            .map_err(|error| (id.0, id.1, error))?;
        if self.active.is_some() || self.live.is_some() || self.pending_live.is_some() {
            return Err((
                id.0,
                id.1,
                "Busy: another live/import session is active".into(),
            ));
        }
        let selection = self
            .workers
            .select(
                if request.config.compute == whisper_core::ComputeChoice::Auto
                    && candidate.is_none()
                {
                    whisper_core::ComputeChoice::Cpu
                } else {
                    request.config.compute
                },
            )
            .map_err(|error| {
                (
                    id.0,
                    id.1,
                    format!("BackendUnavailable: compute policy rejected request ({error:?})"),
                )
            })?;
        self.auto_fallback_reason = (request.config.compute == whisper_core::ComputeChoice::Auto
            && selection.backend == whisper_core::BackendAttempt::Cpu
            && candidate.is_none())
        .then(|| "GPU identity proof unavailable before processing".to_owned());
        if selection.backend == whisper_core::BackendAttempt::Gpu && candidate.is_none() {
            return Err((
                id.0,
                id.1,
                "BackendMismatch: selected GPU has no current identity proof".into(),
            ));
        }
        self.start_diagnostics(Path::new(&request.destination));
        self.diagnostic_degraded_reported = false;
        self.progress = Some(ProgressSupervisor::starting(std::time::Instant::now()));
        self.live_pending_persistence = 0;
        self.pending_attempt_reset = None;
        self.pending_recovery_counters = None;
        self.deferred_live_stopped = None;
        self.deferred_live_window_finished = None;
        let directory = PathBuf::from(&request.destination)
            .join("transcriptions")
            .join(format!("{:032x}", request.job_id.0))
            .join("live");
        fs::create_dir_all(&directory)
            .map_err(|e| (id.0, id.1, format!("StorageUnavailable: {e}")))?;
        let live_archive = LiveArchive::prepare(&request.destination, request.clone())
            .map_err(|e| (id.0, id.1, e))?;
        let mut live_archive = live_archive;
        let attempt_number = campaign_reserve_attempt(&mut live_archive, id.0, id.1)
            .map_err(|error| (id.0, id.1, error))?;
        let staging =
            PcmStaging::create(directory.join(format!("passage-{}.pcm", request.generation.get())))
                .map_err(|e| (id.0, id.1, format!("StorageUnavailable: PCM staging: {e}")))?
                .with_campaign_identity(id.0.0, id.1.get())
                .into_writer();
        let (transport, instance_id) = spawn_live_attempt(&request, &selection, candidate.clone())?;
        live_archive
            .mark_attempt_started(attempt_number)
            .map_err(|error| (id.0, id.1, error))?;
        self.active = Some(ImportRequestState {
            job_id: id.0,
            generation: id.1,
            instance_id,
            backend: match selection.backend {
                whisper_core::BackendAttempt::Cpu => BackendKind::Cpu,
                whisper_core::BackendAttempt::Gpu => BackendKind::Cuda,
            },
            attempt_number,
            import_request: None,
            recovery: LiveRecoveryState::None,
            startup_ready: false,
            gpu_candidate: candidate.clone(),
            gpu_attested: false,
        });
        if let Some(diagnostics) = &self.diagnostics {
            diagnostics.record(DiagnosticRecord::new(
                id.0.0,
                id.1.get(),
                DiagnosticPhase::Preparing,
                DiagnosticCode::WorkerStarted,
                Some(match selection.backend {
                    whisper_core::BackendAttempt::Cpu => "cpu",
                    whisper_core::BackendAttempt::Gpu => "cuda",
                }),
                None,
            ));
        }
        self.transport = Some(transport);
        self.pending_live = Some(PendingLiveState {
            request,
            staging,
            live_archive,
            recovery: LiveRecoveryState::None,
        });
        Ok(())
    }

    fn activate_live_after_ready(&mut self) -> Result<(), (JobId, Generation, String)> {
        let Some(pending) = self.pending_live.take() else {
            return Ok(());
        };
        let (job_id, generation) = (pending.request.job_id, pending.request.generation);
        let initial_gain =
            whisper_core::microphone_level::GainDb::new(pending.request.microphone_gain_db)
                .map_err(|error| (job_id, generation, error.to_string()))?;
        let mut capture = crate::capture::start_default_with_gain(initial_gain)
            .map_err(|error| (job_id, generation, error))?;
        let converter = match Pcm16Converter::new(capture.sample_rate_hz(), capture.channels()) {
            Ok(converter) => converter,
            Err(error) => {
                capture.stop();
                return Err((job_id, generation, error));
            }
        };
        let probe = crate::microphone_probe::MicrophoneProbe::start(
            capture.sample_rate_hz(),
            capture.source_identity().map(str::to_owned),
            pending.request.job_id,
            pending.request.generation,
        )
        .map_err(|error| (job_id, generation, error))?;
        let mut live_archive = pending.live_archive;
        if let Err(error) = live_archive.capture_started() {
            capture.stop();
            return Err((job_id, generation, error));
        }
        self.live_archive = Some(live_archive);
        self.live = Some(LiveCaptureState {
            request: pending.request,
            capture,
            probe,
            spectrum_reader: crate::microphone_probe::MicrophoneSpectrumReader::default(),
            last_meter_sequence: 0,
            next_spectrum_telemetry: false,
            telemetry_due_after_event: false,
            converter,
            vad: VoiceActivity::new(),
            staging: pending.staging,
            frame_pending: Vec::with_capacity(320),
            inference_cursor: 0,
            inference_sequence: 1,
            pending_window: None,
            replay_until: 0,
            replay_windows_queued: 0,
            replay_windows_to_skip: 0,
            segment_sequence_base: 1,
            window_sequence_base: 1,
            recovery: LiveRecoveryState::None,
            barrier_m: None,
            inference_admission_closed: false,
            worker_ready: true,
            stop_requested: false,
            stop_enqueued: false,
            started_at_unix_ms: unix_time_ms(),
            captured: 0,
            admitted: 0,
            speech: 0,
            confirmed_fragment_end: 0,
            last_reported_durable: 0,
            failure: None,
        });
        Ok(())
    }
    fn prepare(
        &mut self,
        mut request: ImportRequest,
        candidate: Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
        events: &mut VecDeque<ImportEvent>,
    ) -> Result<(), (JobId, Generation, String)> {
        let id = (request.job_id, request.generation);
        if request.strategy
            == whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
        {
            #[cfg(test)]
            if self.controlled_transcriber.is_none() && self.unified_executor.is_none() {
                return Err((
                    id.0,
                    id.1,
                    "ModelNotQualified: unified strategy requires the injected controlled worker"
                        .into(),
                ));
            }
            #[cfg(not(test))]
            return Err((id.0, id.1,
                "ModelNotQualified: unified Parakeet strategy is not enabled in the native composition".into()));
        }
        self.ensure_source_binding(&mut request)
            .map_err(|error| (id.0, id.1, error))?;
        crate::archive::l04_campaign_assert_archive(Path::new(&request.destination))
            .map_err(|error| (id.0, id.1, error))?;
        if request_is_parakeet(&request)
            && request.config.compute == whisper_core::ComputeChoice::Gpu
        {
            let candidate = candidate.as_ref().ok_or((
                id.0,
                id.1,
                "BackendMismatch: selected GPU has no current identity proof".into(),
            ))?;
            if candidate.generation != request.generation.get() {
                return Err((
                    id.0,
                    id.1,
                    "BackendMismatch: selected GPU identity belongs to a stale generation".into(),
                ));
            }
            if !self.is_controlled_bounded_test(&request) {
                let paths = self.gpu_parakeet_paths.as_ref().ok_or((
                    id.0,
                    id.1,
                    "BackendUnavailable: worker GPU Test Parakeet non configuré".into(),
                ))?;
                let profile = whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS;
                let key = paths
                    .gpu_qualification_key(profile)
                    .map_err(|error| (id.0, id.1, error))?;
                if !l3_gpu_candidate_allowed(&key) {
                    return Err((
                        id.0,
                        id.1,
                        "ModelNotQualified: GPU Test key is not admitted".into(),
                    ));
                }
            }
            crate::parakeet_import::expected_gpu_for_candidate(candidate).map_err(|error| {
                (
                    id.0,
                    id.1,
                    format!("BackendMismatch: GPU identity validation failed: {error}"),
                )
            })?;
        }
        if self.publication_gate.is_cancelled() {
            events.push_back(ImportEvent::Stopped {
                job_id: id.0,
                generation: id.1,
            });
            return Ok(());
        }
        let mut archive = ArchiveStore::new(&request.destination);
        let qualification = archive_qualification_for_request(
            &request,
            self.parakeet_paths.as_ref(),
            self.gpu_parakeet_paths.as_ref(),
            candidate.as_ref(),
        );
        let prepared = if let Some(binding) = self.source_bindings.get(&(id.0, id.1.get())) {
            archive.prepare_with_source_binding(request, None, qualification, binding)
        } else if self.require_snapshot_registry {
            Err("SourceSnapshotUnavailable: binding missing before prepare".into())
        } else {
            archive.prepare_with_qualification(request, None, qualification)
        };
        match prepared {
            Ok(prepared) => {
                self.archive = archive;
                l2a2_acceptance_barrier("prepare", Path::new(&prepared.destination))
                    .map_err(|error| (id.0, id.1, error))?;
                if self.publication_gate.is_cancelled() {
                    events.push_back(ImportEvent::Stopped {
                        job_id: id.0,
                        generation: id.1,
                    });
                    return Ok(());
                }
                events.push_back(ImportEvent::Prepared(prepared));
                Ok(())
            }
            Err(message) => Err((id.0, id.1, message)),
        }
    }
    #[cfg(test)]
    fn start_worker(
        &mut self,
        request: whisper_core::ports::ImportRequest,
    ) -> Result<(), (JobId, Generation, String)> {
        self.start_worker_with_selection(request, None)
    }

    fn start_worker_with_selection(
        &mut self,
        request: whisper_core::ports::ImportRequest,
        selection: Option<Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>>,
    ) -> Result<(), (JobId, Generation, String)> {
        if let Some(automatic) = self
            .automatic_final_outputs
            .get(&(request.job_id, request.generation.get()))
            && let Err(error) =
                set_automatic_final_manifest_status(automatic, "running", None, None)
        {
            return Err((request.job_id, request.generation, error));
        }
        let request = match self
            .source_bindings
            .get(&(request.job_id, request.generation.get()))
        {
            Some(binding) => binding
                .apply_to_request(&request)
                .map_err(|error| (request.job_id, request.generation, error))?,
            None => request,
        };
        match selection {
            Some(None) if request.config.compute == whisper_core::ComputeChoice::Gpu => Err((
                request.job_id,
                request.generation,
                "BackendMismatch: selected GPU has no current runtime identity proof".into(),
            )),
            Some(None) if request.config.compute == whisper_core::ComputeChoice::Auto => {
                self.auto_fallback_reason =
                    Some("GPU candidate proof unavailable before processing".into());
                self.start_worker_with_cpu_override(request, true)
            }
            Some(Some(candidate)) => self.start_worker_with_candidate(request, Some(candidate)),
            Some(None) => self.start_worker_with_candidate(request, None),
            None => self.start_worker_with_candidate(request, None),
        }
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
        crate::archive::l04_campaign_assert_archive(Path::new(&request.destination))
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
                .is_ok_and(|key| !l3_gpu_candidate_allowed(key))
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
        crate::archive::l04_campaign_assert_archive(Path::new(&request.destination))
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

    fn start_cpu_live_attempt(
        &mut self,
        job_id: JobId,
        generation: Generation,
    ) -> Result<ImportEvent, (JobId, Generation, String)> {
        if self.publication_gate.is_cancelled() {
            return Err((
                job_id,
                generation,
                "Cancelled: CPU retry suppressed by latched Stop/Quit".into(),
            ));
        }
        let request = self
            .live
            .as_ref()
            .filter(|live| (live.request.job_id, live.request.generation) == (job_id, generation))
            .map(|live| live.request.clone())
            .ok_or((
                job_id,
                generation,
                "StaleResponse: live passage is unavailable".into(),
            ))?;
        let selection = self
            .workers
            .select(whisper_core::ComputeChoice::Cpu)
            .map_err(|error| {
                (
                    job_id,
                    generation,
                    format!("BackendUnavailable: CPU fallback unavailable ({error:?})"),
                )
            })?;
        let (confirmed_samples, next_segment_sequence, next_window_sequence, attempt_number) = {
            let archive = self.live_archive.as_mut().ok_or((
                job_id,
                generation,
                "StorageUnavailable: live archive is missing".into(),
            ))?;
            archive
                .discard_unconfirmed_segments()
                .and_then(|()| archive.restart_mp3_attempt())
                .map_err(|error| (job_id, generation, error))?;
            let confirmed_samples = archive.confirmed_samples();
            let next_segment_sequence = archive.next_segment_sequence();
            let next_window_sequence = archive.next_window_sequence();
            let attempt_number = campaign_reserve_attempt(archive, job_id, generation)
                .map_err(|error| (job_id, generation, error))?;
            (
                confirmed_samples,
                next_segment_sequence,
                next_window_sequence,
                attempt_number,
            )
        };
        let (transport, instance_id) = match spawn_live_attempt(&request, &selection, None) {
            Ok(spawned) => spawned,
            Err(error) => {
                if let Some(live) = self.live.as_mut() {
                    live.stop_capture();
                    let _ = live.staging.drain();
                }
                self.live = None;
                self.live_archive = None;
                self.active = None;
                self.transport = None;
                return Err(error);
            }
        };
        self.live_archive
            .as_mut()
            .ok_or((
                job_id,
                generation,
                "StorageUnavailable: live archive disappeared after CPU spawn".into(),
            ))?
            .mark_attempt_started(attempt_number)
            .map_err(|error| (job_id, generation, error))?;
        self.active = Some(ImportRequestState {
            job_id,
            generation,
            instance_id,
            backend: BackendKind::Cpu,
            attempt_number,
            import_request: None,
            recovery: LiveRecoveryState::None,
            startup_ready: false,
            gpu_candidate: None,
            gpu_attested: false,
        });
        self.transport = Some(transport);
        let live = self.live.as_mut().ok_or((
            job_id,
            generation,
            "StorageUnavailable: live capture state disappeared".into(),
        ))?;
        live.replay_until = confirmed_samples;
        live.replay_windows_queued = 0;
        live.replay_windows_to_skip = next_window_sequence.saturating_sub(1);
        live.segment_sequence_base = next_segment_sequence;
        live.window_sequence_base = next_window_sequence;
        live.pending_window = None;
        live.inference_cursor = 0;
        live.inference_sequence = 1;
        live.inference_admission_closed = false;
        live.worker_ready = false;
        live.stop_enqueued = false;
        live.recovery = LiveRecoveryState::None;
        live.barrier_m = None;
        if let Some(diagnostics) = &self.diagnostics {
            diagnostics.record(DiagnosticRecord::new(
                job_id.0,
                generation.get(),
                DiagnosticPhase::Preparing,
                DiagnosticCode::WorkerStarted,
                Some("cpu"),
                None,
            ));
        }
        self.pending_attempt_reset = Some(ImportEvent::LiveAttemptReset {
            job_id,
            generation,
            confirmed_samples,
            next_segment_sequence,
        });
        Ok(ImportEvent::LiveAttemptStarting { job_id, generation })
    }

    fn start_cpu_pending_live_attempt(
        &mut self,
        job_id: JobId,
        generation: Generation,
    ) -> Result<ImportEvent, (JobId, Generation, String)> {
        if self.publication_gate.is_cancelled() {
            return Err((
                job_id,
                generation,
                "Cancelled: CPU retry suppressed by latched Stop/Quit".into(),
            ));
        }
        let (request, attempt_number) = {
            let pending = self
                .pending_live
                .as_mut()
                .filter(|pending| {
                    (pending.request.job_id, pending.request.generation) == (job_id, generation)
                })
                .ok_or((
                    job_id,
                    generation,
                    "StaleResponse: pending live passage is unavailable".into(),
                ))?;
            pending
                .live_archive
                .restart_mp3_attempt()
                .map_err(|error| (job_id, generation, error))?;
            (
                pending.request.clone(),
                campaign_reserve_attempt(&mut pending.live_archive, job_id, generation)
                    .map_err(|error| (job_id, generation, error))?,
            )
        };
        let selection = self
            .workers
            .select(whisper_core::ComputeChoice::Cpu)
            .map_err(|error| {
                (
                    job_id,
                    generation,
                    format!("BackendUnavailable: CPU fallback unavailable ({error:?})"),
                )
            })?;
        let (transport, instance_id) = spawn_live_attempt(&request, &selection, None)?;
        self.pending_live
            .as_mut()
            .ok_or((
                job_id,
                generation,
                "StaleResponse: pending live passage is unavailable".into(),
            ))?
            .live_archive
            .mark_attempt_started(attempt_number)
            .map_err(|error| (job_id, generation, error))?;
        self.active = Some(ImportRequestState {
            job_id,
            generation,
            instance_id,
            backend: BackendKind::Cpu,
            attempt_number,
            import_request: None,
            recovery: LiveRecoveryState::None,
            startup_ready: false,
            gpu_candidate: None,
            gpu_attested: false,
        });
        self.transport = Some(transport);
        if let Some(pending) = self.pending_live.as_mut() {
            pending.recovery = LiveRecoveryState::None;
        }
        Ok(ImportEvent::LiveAttemptStarting { job_id, generation })
    }

    fn finish_live_on_cpu(
        &mut self,
        job_id: JobId,
        generation: Generation,
    ) -> Result<ImportEvent, (JobId, Generation, String)> {
        if self.pending_live.as_ref().is_some_and(|pending| {
            (pending.request.job_id, pending.request.generation) == (job_id, generation)
                && pending.recovery == LiveRecoveryState::AwaitingChoice
        }) {
            return self.start_cpu_pending_live_attempt(job_id, generation);
        }
        if !self.live.as_ref().is_some_and(|live| {
            (live.request.job_id, live.request.generation) == (job_id, generation)
                && live.recovery == LiveRecoveryState::AwaitingChoice
        }) {
            return Err((
                job_id,
                generation,
                "InvalidInput: CPU finish requires a pending user choice".into(),
            ));
        }
        self.start_cpu_live_attempt(job_id, generation)
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
    fn persist_segment(
        &mut self,
        segment: WorkerSegment,
        events: &mut VecDeque<ImportEvent>,
    ) -> Result<(), (JobId, Generation, String)> {
        let id = (segment.job_id, segment.generation);
        let unified = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: id.0,
            generation: id.1,
        };
        if let Some(mut archive) = self.unified_archives.remove(&unified) {
            let result = archive.persist_segment(&segment);
            self.unified_archives.insert(unified, archive);
            let sequence = result.map_err(|error| (id.0, id.1, error))?;
            events.push_back(ImportEvent::Persisted {
                job_id: id.0,
                generation: id.1,
                sequence,
            });
            return Ok(());
        }
        if self.live.is_some() {
            let durable = self
                .live
                .as_ref()
                .map_or(0, |live| live.staging.durable_samples());
            let confirmed = self
                .live_archive
                .as_mut()
                .ok_or((
                    id.0,
                    id.1,
                    "StorageUnavailable: live archive is missing".into(),
                ))?
                .persist_segment(&segment, durable)
                .map_err(|e| (id.0, id.1, e))?;
            if let Some(live) = self.live.as_mut() {
                live.confirmed_fragment_end = confirmed;
            }
            if let Some(progress) = self.progress.as_mut() {
                progress.observe(
                    ProgressPhase::Draining,
                    confirmed,
                    std::time::Instant::now(),
                );
            }
            events.push_back(ImportEvent::Persisted {
                job_id: id.0,
                generation: id.1,
                sequence: segment.segment_id.0,
            });
            return Ok(());
        }
        let sequence = self
            .archive
            .persist_segment(&segment)
            .map_err(|e| (id.0, id.1, e))?;
        if let Some(progress) = self.progress.as_mut() {
            progress.observe(ProgressPhase::Draining, sequence, std::time::Instant::now());
        }
        events.push_back(ImportEvent::Persisted {
            job_id: id.0,
            generation: id.1,
            sequence,
        });
        Ok(())
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
        crate::archive::l04_campaign_gate(
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

    fn publish(
        &mut self,
        job: JobId,
        generation: Generation,
        count: u64,
        events: &mut VecDeque<ImportEvent>,
    ) -> Result<(), (JobId, Generation, String)> {
        let session = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: job,
            generation,
        };
        let publication_gate = self
            .unified_publication_gates
            .get(&session)
            .cloned()
            .unwrap_or_else(|| Arc::clone(&self.publication_gate));
        if let Some(archive) = self.unified_archives.remove(&session) {
            let previous_archive = std::mem::replace(&mut self.archive, archive);
            let result = self.publish_inner(job, generation, count, events, &publication_gate);
            let session_archive = std::mem::replace(&mut self.archive, previous_archive);
            if result.is_err() {
                self.unified_archives.insert(session, session_archive);
            } else {
                self.unified_publication_gates.remove(&session);
            }
            return result;
        }
        let result = self.publish_inner(job, generation, count, events, &publication_gate);
        if result.is_ok() {
            self.unified_publication_gates.remove(&session);
        }
        result
    }

    fn publish_inner(
        &mut self,
        job: JobId,
        generation: Generation,
        count: u64,
        events: &mut VecDeque<ImportEvent>,
        publication_gate: &PublicationGate,
    ) -> Result<(), (JobId, Generation, String)> {
        let destination = self.archive.root_path();
        let _timing_context = R7TimingContextGuard::enter(
            "publish-import",
            format!("{:032x}-{}", job.0, generation.get()),
            &destination,
        );
        let result = (|| {
            if let Some(progress) = self.progress.as_mut() {
                progress.close(ProgressPhase::Draining);
                progress.close(ProgressPhase::Finalizing);
            }
            self.archive
                .prepare_publish(job, generation, count)
                .map_err(|e| (job, generation, e))?;
            l2a2_acceptance_barrier("publish-before-commit", self.archive.root_path().as_path())
                .map_err(|error| (job, generation, error))?;
            publication_gate
                .commit(|| {
                    self.archive.commit_publish(job, generation)?;
                    l2a2_acceptance_barrier(
                        "publish-after-commit",
                        self.archive.root_path().as_path(),
                    )
                })
                .map_err(|e| (job, generation, e))?;
            let root = self.archive_root();
            if root
                .join("transcriptions")
                .join(format!("{:032x}", job.0))
                .join(generation.get().to_string())
                .join("parakeet-unified/document")
                .is_dir()
            {
                let result = ArchiveStore::published_unified_files(&root, job, generation);
                events.push_back(ImportEvent::UnifiedPublicationArtifacts {
                    job_id: job,
                    generation,
                    result,
                });
            }
            if let Ok(mut queue) = crate::queue_store::QueueStore::open(&root)
                && queue.remove(job, generation).is_ok()
            {
                events.push_back(ImportEvent::Queue(queue.entries()));
            }
            let first = ArchiveStore::scan(&root).map_err(|e| (job, generation, e))?;
            let second = ArchiveStore::scan(&root).map_err(|e| (job, generation, e))?;
            if first != second
                || !first.iter().any(|entry| {
                    entry.job_id == job && entry.generation == generation && entry.complete
                })
            {
                return Err((
                    job,
                    generation,
                    "StorageCorrupt: archive scan did not verify stable published generation"
                        .into(),
                ));
            }
            self.release_source_binding(job, generation, &root)
                .map_err(|error| (job, generation, error))?;
            if let Some(automatic) = self.automatic_final_outputs.get(&(job, generation.get())) {
                let entry = first
                    .iter()
                    .find(|entry| {
                        entry.job_id == job && entry.generation == generation && entry.complete
                    })
                    .ok_or_else(|| {
                        (
                            job,
                            generation,
                            "ArchiveEntryMissing: import final introuvable".into(),
                        )
                    })?;
                let basename = entry.export_basename.as_deref().ok_or_else(|| {
                    (
                        job,
                        generation,
                        "ArchiveExportMissing: export final absent".into(),
                    )
                })?;
                let archive_exports = root.join("exports");
                publish_automatic_final_exports(
                    &archive_exports.join(format!("{basename}.txt")),
                    &archive_exports.join(format!("{basename}.srt")),
                    automatic,
                )
                .map_err(|error| (job, generation, error))?;
                self.automatic_final_outputs
                    .remove(&(job, generation.get()));
            }
            events.push_back(ImportEvent::Published {
                job_id: job,
                generation,
            });
            let cleanups = ArchiveStore::replacement_cleanups(&root).unwrap_or_else(|error| {
                vec![whisper_core::ports::ReplacementCleanupInfo {
                    cleanup_id: "unavailable".into(),
                    successor: TreatmentIdentity {
                        destination: root.to_string_lossy().into_owned(),
                        job_id: job,
                        generation,
                    },
                    remaining: Vec::new(),
                    preserved: Vec::new(),
                    last_error: Some(error),
                }]
            });
            events.push_back(ImportEvent::ReplacementCleanups(cleanups));
            Ok(())
        })();

        match result {
            Ok(()) => {
                if let Some(mut worker) = self.pending_idle_worker.take() {
                    worker.expires_at = Instant::now() + WORKER_IDLE_RETENTION;
                    self.idle_worker = Some(worker);
                }
                Ok(())
            }
            Err(error) => {
                // End alone does not start the retention period: an unpublished
                // result must not leave a model resident or allow a second worker.
                self.pending_idle_worker = None;
                self.idle_worker = None;
                if let Some(automatic) = self.automatic_final_outputs.get(&(job, generation.get()))
                {
                    let _ = set_automatic_final_manifest_status(
                        automatic,
                        "failed_or_unavailable",
                        None,
                        Some(&error.2),
                    );
                }
                self.automatic_final_outputs
                    .remove(&(job, generation.get()));
                Err(error)
            }
        }
    }
    fn archive_root(&self) -> PathBuf {
        self.archive.root_path()
    }
    fn stop(
        &mut self,
        job: JobId,
        generation: Generation,
        events: &mut VecDeque<ImportEvent>,
    ) -> Result<(), (JobId, Generation, String)> {
        if let Some(live) = self.live.as_mut() {
            if (live.request.job_id, live.request.generation) != (job, generation) {
                return Err((
                    job,
                    generation,
                    "StaleResponse: live Stop targets another passage".into(),
                ));
            }
            if live.recovery == LiveRecoveryState::AwaitingChoice && self.active.is_none() {
                self.live = None;
                self.live_archive = None;
                self.transport = None;
                events.push_back(ImportEvent::Stopped {
                    job_id: job,
                    generation,
                });
                return Ok(());
            }
            live.stop_capture();
            if live.failure.is_none() {
                while let Some(block) = live.capture.try_next_block() {
                    let samples = live
                        .converter
                        .push(&block)
                        .map_err(|e| (job, generation, e))?;
                    live.capture
                        .recycle_block(block)
                        .map_err(|e| (job, generation, e))?;
                    live.frame_pending.extend(samples);
                    while live.frame_pending.len() >= 320 {
                        let frame: Vec<i16> = live.frame_pending.drain(..320).collect();
                        live.captured += 320;
                        if live
                            .vad
                            .is_speech(&frame)
                            .map_err(|e| (job, generation, e))?
                        {
                            live.speech += 320;
                        }
                        live.staging
                            .append_frame(frame)
                            .map_err(|e| (job, generation, format!("StorageUnavailable: {e}")))?;
                        live.admitted += 320;
                    }
                }
                let tail = live.converter.finish().map_err(|e| (job, generation, e))?;
                live.frame_pending.extend(tail);
                while live.frame_pending.len() >= 320 {
                    let frame: Vec<i16> = live.frame_pending.drain(..320).collect();
                    live.captured += 320;
                    if live
                        .vad
                        .is_speech(&frame)
                        .map_err(|e| (job, generation, e))?
                    {
                        live.speech += 320;
                    }
                    live.staging
                        .append_frame(frame)
                        .map_err(|e| (job, generation, format!("StorageUnavailable: {e}")))?;
                    live.admitted += 320;
                }
                if !live.frame_pending.is_empty() {
                    let valid = std::mem::take(&mut live.frame_pending);
                    let valid_len = valid.len();
                    let mut vad_frame = [0i16; 320];
                    vad_frame[..valid_len].copy_from_slice(&valid);
                    if live
                        .vad
                        .is_speech(&vad_frame)
                        .map_err(|e| (job, generation, e))?
                    {
                        live.speech += valid_len as u64;
                    }
                    live.captured += valid_len as u64;
                    live.staging.append_tail(valid).map_err(|e| {
                        (
                            job,
                            generation,
                            format!("StorageUnavailable: final PCM tail: {e}"),
                        )
                    })?;
                    live.admitted += valid_len as u64;
                }
            }
            crate::archive::l04_campaign_gate("pcm.before_drain", job.0, generation.get(), 0)
                .map_err(|error| (job, generation, error))?;
            let drain_result = live.staging.drain();
            crate::archive::l04_campaign_operation_result(
                "pcm.before_drain",
                job.0,
                generation.get(),
                0,
                &drain_result,
                None,
            )
            .map_err(|error| (job, generation, error))?;
            drain_result.map_err(|e| {
                (
                    job,
                    generation,
                    format!("StorageUnavailable: drain failed: {e}"),
                )
            })?;
            crate::archive::l04_campaign_gate("pcm.after_drain", job.0, generation.get(), 0)
                .map_err(|error| (job, generation, error))?;
            let durable_samples = live.staging.durable_samples();
            live.last_reported_durable = durable_samples;
            events.push_back(ImportEvent::LiveCounters {
                job_id: job,
                generation,
                captured_samples: live.captured,
                admitted_samples: live.admitted,
                audio_durable_samples: durable_samples,
                confirmed_fragment_samples: live.confirmed_fragment_end,
                speech_samples: live.speech,
                diagnostic: None,
            });
            live.stop_requested = true;
            return Ok(());
        }
        if !self.publication_gate.is_cancelled() {
            events.push_back(ImportEvent::Stopped {
                job_id: job,
                generation,
            });
            return Ok(());
        }
        let Some(active) = self.active.as_ref() else {
            events.push_back(ImportEvent::Stopped {
                job_id: job,
                generation,
            });
            return Ok(());
        };
        if active.job_id != job || active.generation != generation {
            events.push_back(ImportEvent::Stopped {
                job_id: job,
                generation,
            });
            return Ok(());
        }
        let Some(transport) = self.transport.as_mut() else {
            events.push_back(ImportEvent::Stopped {
                job_id: job,
                generation,
            });
            return Ok(());
        };
        transport
            .request(WorkerCommand::Stop {
                job_id: job,
                generation,
            })
            .map_err(|e| (job, generation, format!("WorkerExited: {e:?}")))
    }
    fn poll_live_capture(&mut self) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
        self.pump_live_windows();
        let live = self.live.as_mut()?;
        if live.stop_requested {
            return None;
        }
        if live.capture.is_saturated() || live.capture.failure() {
            let (job_id, generation) = (live.request.job_id, live.request.generation);
            let reason = if live.capture.is_saturated() {
                "CaptureSaturated: bounded callback slots are full"
            } else {
                "CaptureInterrupted: WASAPI stream failed"
            };
            live.stop_capture();
            live.failure = Some(reason.into());
            if let Some(transport) = self.transport.as_mut() {
                let _ = transport.try_request(WorkerCommand::Stop { job_id, generation });
            }
            return None;
        }
        let block = live.capture.try_next_block()?;
        let converted = match live.converter.push_with_probe(&block) {
            Ok(converted) => converted,
            Err(error) => {
                live.stop_capture();
                live.failure = Some(error.clone());
                if let Some(transport) = self.transport.as_mut() {
                    let _ = transport.try_request(WorkerCommand::Stop {
                        job_id: live.request.job_id,
                        generation: live.request.generation,
                    });
                }
                return None;
            }
        };
        if let Err(error) = live.capture.recycle_block(block) {
            live.stop_capture();
            live.failure = Some(error.clone());
            if let Some(transport) = self.transport.as_mut() {
                let _ = transport.try_request(WorkerCommand::Stop {
                    job_id: live.request.job_id,
                    generation: live.request.generation,
                });
            }
            return None;
        }
        live.probe.submit(converted.probe);
        live.frame_pending.extend(converted.samples);
        while live.frame_pending.len() >= 320 {
            let frame: Vec<i16> = live.frame_pending.drain(..320).collect();
            live.captured = live.captured.saturating_add(320);
            match live.vad.is_speech(&frame) {
                Ok(true) => live.speech = live.speech.saturating_add(320),
                Ok(false) => {}
                Err(error) => {
                    live.stop_capture();
                    live.failure = Some(error.clone());
                    if let Some(transport) = self.transport.as_mut() {
                        let _ = transport.try_request(WorkerCommand::Stop {
                            job_id: live.request.job_id,
                            generation: live.request.generation,
                        });
                    }
                    return None;
                }
            }
            if let Err(error) = live.staging.try_append_frame(frame) {
                live.stop_capture();
                let reason = if error.kind() == io::ErrorKind::WouldBlock {
                    "CaptureSaturated: bounded PCM writer queue is full".to_owned()
                } else {
                    format!("StorageUnavailable: {error}")
                };
                live.failure = Some(reason);
                if let Some(transport) = self.transport.as_mut() {
                    let _ = transport.try_request(WorkerCommand::Stop {
                        job_id: live.request.job_id,
                        generation: live.request.generation,
                    });
                }
                return None;
            }
            live.admitted = live.admitted.saturating_add(320);
        }
        let durable_samples = live.staging.durable_samples();
        if durable_samples == live.last_reported_durable {
            return None;
        }
        live.last_reported_durable = durable_samples;
        let event = ImportEvent::LiveCounters {
            job_id: live.request.job_id,
            generation: live.request.generation,
            captured_samples: live.captured,
            admitted_samples: live.admitted,
            audio_durable_samples: durable_samples,
            confirmed_fragment_samples: live.confirmed_fragment_end,
            speech_samples: live.speech,
            diagnostic: None,
        };
        Some(Ok(event))
    }

    fn pump_live_windows(&mut self) {
        let (Some(live), Some(transport)) = (self.live.as_mut(), self.transport.as_ref()) else {
            return;
        };
        if live.inference_admission_closed || !live.worker_ready {
            return;
        }
        let durable_samples = live.staging.durable_samples();
        if live.pending_window.is_none() {
            let available = durable_samples.saturating_sub(live.inference_cursor);
            let replay_remaining = live.replay_until.saturating_sub(live.inference_cursor);
            let sample_count = if available == 0 {
                0
            } else if replay_remaining > 0 {
                available.min(80_000).min(replay_remaining) as usize
            } else if available >= 80_000 {
                80_000
            } else if live.stop_requested {
                available as usize
            } else {
                0
            };
            if sample_count > 0 {
                match live
                    .staging
                    .read_durable_samples(live.inference_cursor, sample_count)
                {
                    Ok(Some(samples)) => {
                        let start = live.inference_cursor;
                        let end = start.saturating_add(sample_count as u64);
                        live.pending_window = Some((live.inference_sequence, start, end, samples));
                    }
                    Ok(None) => return,
                    Err(error) => {
                        live.failure = Some(format!("StorageUnavailable: PCM spool read: {error}"));
                        live.stop_capture();
                        return;
                    }
                }
            }
        }
        if let Some((sequence, start, end, samples)) = live.pending_window.take() {
            let Some(active) = self.active.as_ref() else {
                live.pending_window = Some((sequence, start, end, samples));
                return;
            };
            let Some(range) = whisper_core::SourceRange::new(start, end, 16_000) else {
                live.failure = Some("StorageCorrupt: live inference range is invalid".into());
                live.stop_capture();
                return;
            };
            let replay = start < live.replay_until;
            let command = if replay {
                WorkerCommand::LiveReplayWindow {
                    job_id: active.job_id,
                    generation: active.generation,
                    instance_id: active.instance_id,
                    sequence,
                    range,
                    samples,
                }
            } else {
                WorkerCommand::LiveWindow {
                    job_id: active.job_id,
                    generation: active.generation,
                    instance_id: active.instance_id,
                    sequence,
                    range,
                    samples,
                }
            };
            match transport.try_request(command) {
                Ok(()) => {
                    live.inference_cursor = end;
                    live.inference_sequence = live.inference_sequence.saturating_add(1);
                    if replay {
                        live.replay_windows_queued = live.replay_windows_queued.saturating_add(1);
                    }
                }
                Err(EnqueueError::Full(command)) => {
                    let (sequence, range, samples) = match *command {
                        WorkerCommand::LiveWindow {
                            sequence,
                            range,
                            samples,
                            ..
                        }
                        | WorkerCommand::LiveReplayWindow {
                            sequence,
                            range,
                            samples,
                            ..
                        } => (sequence, range, samples),
                        _ => unreachable!("only live windows are queued here"),
                    };
                    live.pending_window =
                        Some((sequence, range.start_sample, range.end_sample, samples));
                    return;
                }
                Err(EnqueueError::Disconnected) => {
                    live.failure = Some("WorkerExited: stdin dispatcher stopped".into());
                    live.stop_capture();
                    return;
                }
            }
        }
        let durable_samples = live.staging.durable_samples();
        if live.stop_requested
            && live.pending_window.is_none()
            && live.inference_cursor >= durable_samples
            && !live.stop_enqueued
        {
            let Some(active) = self.active.as_ref() else {
                return;
            };
            match transport.try_request_ordered(WorkerCommand::Stop {
                job_id: active.job_id,
                generation: active.generation,
            }) {
                Ok(()) => live.stop_enqueued = true,
                Err(EnqueueError::Full(_)) => {}
                Err(EnqueueError::Disconnected) => {
                    live.failure = Some("WorkerExited: stdin dispatcher stopped".into());
                }
            }
        }
    }

    fn advance_parakeet_vad_decode(&mut self) -> Result<(), (JobId, Generation, String)> {
        let Some(vad) = self.parakeet_vad.as_mut() else {
            return Ok(());
        };
        if vad.decode_offset >= vad.total_samples || vad.decoder.is_none() {
            return Ok(());
        }
        let request = vad.request.clone();
        let start = vad.decode_offset;
        let end = start.saturating_add(80_000).min(vad.total_samples);
        let Some(range) = whisper_core::SourceRange::new(start, end, 16_000) else {
            return Err((
                request.job_id,
                request.generation,
                "InvalidInput: invalid VAD decode range".into(),
            ));
        };
        let sha256 = decode_sha256(&vad.source_sha256)
            .map_err(|error| (request.job_id, request.generation, error))?;
        let decoded = vad
            .decoder
            .as_mut()
            .ok_or((
                request.job_id,
                request.generation,
                "DecoderUnavailable".into(),
            ))?
            .decode(DecodeRequest {
                source: SourceIdentity {
                    source_id: request.source_path.clone(),
                    sha256,
                },
                job_id: request.job_id,
                generation: request.generation,
                range,
                max_samples: 80_000,
            })
            .map_err(|error| {
                (
                    request.job_id,
                    request.generation,
                    format!("DecoderFailed: {error:?}"),
                )
            })?;
        let Some(block) = decoded.into_iter().next() else {
            return Err((
                request.job_id,
                request.generation,
                "DecoderFailed: empty VAD decode block".into(),
            ));
        };
        if block.block.range.start_sample != start
            || block.block.range.end_sample != end
            || block.block.samples.len() as u64 != end - start
        {
            return Err((
                request.job_id,
                request.generation,
                "DecoderFailed: VAD decode block has a gap or wrong length".into(),
            ));
        }
        vad.pcm
            .try_reserve(block.block.samples.len())
            .map_err(|error| {
                (
                    request.job_id,
                    request.generation,
                    format!("MemoryUnavailable: VAD PCM allocation: {error}"),
                )
            })?;
        vad.pcm.extend(block.block.samples);
        vad.decode_offset = end;
        if vad.decode_offset == vad.total_samples {
            vad.decoder = None;
            if vad.pcm.len() as u64 != vad.total_samples {
                return Err((
                    request.job_id,
                    request.generation,
                    "DecoderFailed: decoded sample count differs from inspected duration".into(),
                ));
            }
            vad.windows = whisper_core::parakeet_windows::plan_windows(&vad.pcm)
                .map_err(|error| (request.job_id, request.generation, format!("{error}")))?;
            let confirmed = self
                .archive
                .confirmed_import_vad_offset()
                .map_err(|error| (request.job_id, request.generation, error))?;
            if confirmed > 0 {
                let next = vad
                    .windows
                    .iter()
                    .position(|window| window.central_start_sample == confirmed)
                    .or_else(|| (confirmed == vad.total_samples).then_some(vad.windows.len()))
                    .ok_or((
                        request.job_id,
                        request.generation,
                        "StorageCorrupt: confirmed VAD offset is not a planned boundary".into(),
                    ))?;
                vad.next_window = next;
            }
            vad.next_segment_sequence = self.archive.next_segment_sequence();
        }
        Ok(())
    }

    fn begin_next_parakeet_vad_window(&mut self) -> Result<(), (JobId, Generation, String)> {
        let Some(vad) = self.parakeet_vad.as_mut() else {
            return Ok(());
        };
        let request = vad.request.clone();
        if vad.next_window >= vad.windows.len() {
            let last_sequence = vad.next_segment_sequence.saturating_sub(1);
            self.parakeet_events.push_back(WorkerEvent::Progress {
                job_id: request.job_id,
                generation: request.generation,
                instance_id: self.active.as_ref().map_or(0, |active| active.instance_id),
                completed_samples: vad.total_samples,
                total_samples: vad.total_samples,
            });
            self.parakeet_end = Some((
                request.job_id,
                request.generation,
                last_sequence,
                vad.total_samples,
            ));
            self.parakeet
                .as_mut()
                .ok_or((
                    request.job_id,
                    request.generation,
                    "WorkerExited: Parakeet process is absent".into(),
                ))?
                .shutdown()
                .map_err(|error| (request.job_id, request.generation, error))?;
            return Ok(());
        }
        let window = vad.windows[vad.next_window];
        let start = usize::try_from(window.native_start_sample).map_err(|_| {
            (
                request.job_id,
                request.generation,
                "InvalidInput: VAD offset exceeds address space".into(),
            )
        })?;
        let end = usize::try_from(window.native_end_sample).map_err(|_| {
            (
                request.job_id,
                request.generation,
                "InvalidInput: VAD offset exceeds address space".into(),
            )
        })?;
        let samples = vad.pcm.get(start..end).ok_or((
            request.job_id,
            request.generation,
            "StorageCorrupt: VAD window is outside decoded PCM".into(),
        ))?;
        let directory = PathBuf::from(&request.destination)
            .join("transcriptions")
            .join(format!("{:032x}", request.job_id.0))
            .join(request.generation.get().to_string())
            .join(".parakeet-vad");
        fs::create_dir_all(&directory).map_err(|error| {
            (
                request.job_id,
                request.generation,
                format!("StorageUnavailable: VAD temp directory: {error}"),
            )
        })?;
        let path = directory.join(format!("window-{:06}.wav", window.index));
        write_pcm16_wav(&path, samples)
            .map_err(|error| (request.job_id, request.generation, error))?;
        let source_sha256 = crate::archive::sha256_file(&path)
            .map(|sha| format_sha256(&sha))
            .map_err(|error| {
                (
                    request.job_id,
                    request.generation,
                    format!("StorageUnavailable: VAD window hash: {error}"),
                )
            })?;
        vad.current_source_sha256 = Some(source_sha256.clone());
        let attempt = window.index as u64 + 1;
        let process = self.parakeet.as_mut().ok_or((
            request.job_id,
            request.generation,
            "WorkerExited: Parakeet process is absent".into(),
        ))?;
        process
            .transcribe(
                format!("{:032x}", request.job_id.0),
                request.generation.get(),
                attempt,
                &path,
                source_sha256,
            )
            .map_err(|error| (request.job_id, request.generation, error))?;
        vad.current_attempt = attempt;
        vad.request_count = vad.request_count.saturating_add(1);
        vad.current_path = Some(path);
        Ok(())
    }

    fn handle_parakeet_vad_result(
        &mut self,
        request_id: u64,
        job_id: String,
        generation: u64,
        attempt: u64,
        identity: whisper_core::parakeet_ipc::WorkerIdentity,
        source_sha256: String,
        duration_samples: u64,
        words: Vec<whisper_core::parakeet_ipc::Word>,
    ) -> Option<Result<WorkerEvent, PortError>> {
        let active = self.active.as_ref()?;
        let id = (active.job_id, active.generation, active.instance_id);
        let Some(run) = self.parakeet_vad.as_mut() else {
            return Some(Err(PortError::Failed(
                "StaleResponse: VAD state disappeared".into(),
            )));
        };
        let Some(window) = run.windows.get(run.next_window).copied() else {
            return Some(Err(PortError::Failed(
                "ProtocolMismatch: VAD result has no pending window".into(),
            )));
        };
        let expected_identity = self
            .parakeet
            .as_ref()
            .and_then(|process| process.identity());
        let expected_duration = window.native_end_sample - window.native_start_sample;
        if request_id != run.request_count
            || job_id != format!("{:032x}", active.job_id.0)
            || generation != active.generation.get()
            || attempt != run.current_attempt
            || Some(&identity) != expected_identity
            || run.current_source_sha256.as_deref() != Some(source_sha256.as_str())
            || duration_samples != expected_duration
        {
            return Some(Err(PortError::Failed(
                "ProtocolMismatch: Parakeet VAD result identity, source, attempt, or duration differs".into(),
            )));
        }
        if let Some(path) = run.current_path.take() {
            let _ = fs::remove_file(path);
        }
        run.current_source_sha256 = None;
        let segments = match parakeet_vad_segments(
            &words,
            window,
            run.total_samples,
            id.0,
            id.1,
            id.2,
            run.next_segment_sequence,
        ) {
            Ok(segments) => segments,
            Err(error) => return Some(Err(PortError::Failed(error))),
        };
        run.next_segment_sequence =
            match run.next_segment_sequence.checked_add(segments.len() as u64) {
                Some(next) => next,
                None => {
                    return Some(Err(PortError::Failed(
                        "InvalidInput: Parakeet segment sequence exhausted".into(),
                    )));
                }
            };
        let last_sequence = run.next_segment_sequence.saturating_sub(1);
        run.pending_confirmation = Some((window, last_sequence));
        self.parakeet_events.push_back(WorkerEvent::Progress {
            job_id: id.0,
            generation: id.1,
            instance_id: id.2,
            completed_samples: window.central_end_sample,
            total_samples: 0,
        });
        for segment in segments {
            self.parakeet_events
                .push_back(WorkerEvent::Segment(segment));
        }
        if self.parakeet_events.len() == 1 {
            if let Err(error) = self.confirm_and_advance_parakeet_vad_window(window, last_sequence)
            {
                return Some(Err(PortError::Failed(error)));
            }
        }
        self.parakeet_events.pop_front().map(Ok)
    }

    fn confirm_and_advance_parakeet_vad_window(
        &mut self,
        window: whisper_core::parakeet_windows::Window,
        last_segment_sequence: u64,
    ) -> Result<(), String> {
        self.archive
            .confirm_import_vad_window(window, last_segment_sequence)?;
        let Some(run) = self.parakeet_vad.as_mut() else {
            return Err("StaleResponse: VAD state disappeared at confirmation".into());
        };
        run.pending_confirmation = None;
        run.next_window = run
            .next_window
            .checked_add(1)
            .ok_or_else(|| "InvalidInput: VAD window sequence exhausted".to_owned())?;
        self.begin_next_parakeet_vad_window()
            .map_err(|(_, _, error)| error)
    }

    fn poll_parakeet_worker(&mut self) -> Option<Result<WorkerEvent, PortError>> {
        let destination = self
            .active
            .as_ref()
            .and_then(|active| active.import_request.as_ref())
            .map(|request| PathBuf::from(&request.destination));
        if let Some(destination) = destination {
            match l2a2_acceptance_take_worker_fault(&destination, self.parakeet_native_call_started)
            {
                Ok(Some(message)) => {
                    let process = self.parakeet.as_mut()?;
                    if message.starts_with("IpcDecode:") {
                        if let Err(error) = process.inject_incomplete_result_for_acceptance() {
                            return Some(Err(PortError::Failed(error)));
                        }
                    } else {
                        return Some(match process.terminate() {
                            Ok(status) => {
                                Err(PortError::Failed(format!("{message}; child={status}")))
                            }
                            Err(error) => Err(PortError::Failed(format!(
                                "{message}; termination failed: {error}"
                            ))),
                        });
                    }
                }
                Ok(None) => {}
                Err(error) => return Some(Err(PortError::Failed(error))),
            }
        }
        if let Some(event) = self.parakeet_events.pop_front() {
            return Some(Ok(event));
        }
        if self.parakeet_stopped_ack {
            let status = self.parakeet.as_mut()?.try_wait();
            return match status {
                Ok(None) if parakeet_stopped_exit(None) == ParakeetStoppedExit::Wait => None,
                Ok(None) => None,
                Ok(Some(status))
                    if parakeet_stopped_exit(Some(status.success()))
                        == ParakeetStoppedExit::Success =>
                {
                    self.parakeet = None;
                    self.parakeet_stopped_ack = false;
                    if let Some((job_id, generation, last_sequence, last_offset)) =
                        self.parakeet_end.take()
                    {
                        let instance_id = self.active.as_ref()?.instance_id;
                        Some(Ok(WorkerEvent::End {
                            job_id,
                            generation,
                            instance_id,
                            last_sequence,
                            last_offset,
                        }))
                    } else {
                        Some(Err(PortError::Failed(
                            "WorkerExited: Parakeet stopped without a transcription result".into(),
                        )))
                    }
                }
                Ok(Some(status)) => {
                    self.parakeet = None;
                    self.parakeet_stopped_ack = false;
                    self.parakeet_end = None;
                    Some(Err(PortError::Failed(format!("WorkerExited: {status}"))))
                }
                Err(error) => Some(Err(PortError::Failed(error))),
            };
        }
        let received = match self.parakeet.as_mut()?.poll() {
            Ok(Some(event)) => event,
            Ok(None) => {
                let status = self.parakeet.as_mut()?.try_wait();
                return match status {
                    Ok(None) => None,
                    Ok(Some(status)) if status.success() => {
                        self.parakeet = None;
                        self.parakeet_stopped_ack = false;
                        self.parakeet_end = None;
                        Some(Err(PortError::Failed(
                            "WorkerExited: process ended before result acknowledgement".into(),
                        )))
                    }
                    Ok(Some(status)) => {
                        self.parakeet = None;
                        self.parakeet_stopped_ack = false;
                        Some(Err(PortError::Failed(format!("WorkerExited: {status}"))))
                    }
                    Err(error) => Some(Err(PortError::Failed(error))),
                };
            }
            Err(error) => {
                if let Some(process) = self.parakeet.as_mut() {
                    let _ = process.terminate();
                }
                self.parakeet = None;
                self.parakeet_stopped_ack = false;
                return Some(Err(PortError::Failed(error)));
            }
        };
        match received {
            whisper_core::parakeet_ipc::Event::Ready {
                identity,
                backend,
                gpu_evidence,
                ..
            } => {
                let (
                    request,
                    expected_backend,
                    attempt_number,
                    active_job_id,
                    active_generation,
                    active_instance_id,
                ) = {
                    let Some(active) = self.active.as_ref() else {
                        return Some(Err(PortError::Failed(
                            "StaleResponse: Parakeet Ready without an active import".into(),
                        )));
                    };
                    let Some(request) = active.import_request.clone() else {
                        return Some(Err(PortError::Failed(
                            "StaleResponse: Parakeet request is unavailable".into(),
                        )));
                    };
                    (
                        request,
                        active.backend,
                        active.attempt_number,
                        active.job_id,
                        active.generation,
                        active.instance_id,
                    )
                };
                let profile = if self.parakeet_vad.is_some() {
                    whisper_core::parakeet_windows::PROFILE_IMPORT_VAD
                } else {
                    whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS
                };
                let record = whisper_core::model_registry::model(
                    whisper_core::model_catalog::ModelId::ParakeetTdt06BV3,
                );
                let expected_gpu = self.parakeet_expected_gpu.clone();
                let expected_worker_paths = if expected_backend == BackendKind::Cuda {
                    self.gpu_parakeet_paths.as_ref()
                } else {
                    self.parakeet_paths.as_ref()
                };
                let key = expected_worker_paths
                    .and_then(|paths| {
                        if expected_backend == BackendKind::Cuda {
                            paths.gpu_qualification_key(profile).ok()
                        } else {
                            paths.qualification_key(profile).ok()
                        }
                    })
                    .unwrap_or_else(crate::archive::ArchiveQualificationKey::parakeet_cpu_test);
                let backend_matches = match expected_backend {
                    BackendKind::Cpu => backend == "CPU" && gpu_evidence.is_none(),
                    BackendKind::Cuda => {
                        backend == "CUDA0"
                            && self.parakeet.as_ref().is_some_and(|process| {
                                gpu_evidence_matches(
                                    gpu_evidence.as_ref(),
                                    expected_gpu.as_ref(),
                                    process.pid(),
                                )
                            })
                    }
                };
                if !backend_matches
                    || identity.model_sha256 != record.artifact.sha256
                    || identity.runtime_sha256 != key.runtime_build
                    || identity.build_sha256 != key.worker_build
                    || identity.profile != profile
                {
                    if expected_backend == BackendKind::Cuda
                        && request.config.compute == whisper_core::ComputeChoice::Auto
                    {
                        return match self.fallback_parakeet_to_cpu(
                            request,
                            "GPU worker Ready proof did not match the qualified key".into(),
                        ) {
                            Ok(()) => None,
                            Err(error) => Some(Err(PortError::Failed(error))),
                        };
                    }
                    return Some(Err(PortError::Failed(
                        "BackendMismatch: Parakeet worker identity differs from the qualified key"
                            .into(),
                    )));
                }
                let process = self.parakeet.as_mut()?;
                if let Err(error) = process.accept_ready(identity) {
                    if expected_backend == BackendKind::Cuda
                        && request.config.compute == whisper_core::ComputeChoice::Auto
                    {
                        return match self.fallback_parakeet_to_cpu(
                            request,
                            format!("GPU worker readiness failed: {error}"),
                        ) {
                            Ok(()) => None,
                            Err(error) => Some(Err(PortError::Failed(error))),
                        };
                    }
                    return Some(Err(PortError::Failed(error)));
                }
                if expected_backend == BackendKind::Cuda
                    && let Some(active) = self.active.as_mut()
                {
                    active.gpu_attested = true;
                }
                if let Some(vad) = self.parakeet_vad.as_mut() {
                    vad.worker_ready = true;
                    return None;
                }
                let source_sha = request
                    .source_sha256
                    .map(|sha| format_sha256(&sha))
                    .unwrap_or_default();
                if let Err(error) = process.transcribe_with_gpu(
                    format!("{:032x}", request.job_id.0),
                    request.generation.get(),
                    attempt_number,
                    Path::new(&request.source_path),
                    source_sha,
                    expected_gpu,
                ) {
                    if expected_backend == BackendKind::Cuda
                        && request.config.compute == whisper_core::ComputeChoice::Auto
                    {
                        return match self.fallback_parakeet_to_cpu(
                            request,
                            format!("GPU request preparation failed: {error}"),
                        ) {
                            Ok(()) => None,
                            Err(error) => Some(Err(PortError::Failed(error))),
                        };
                    }
                    return Some(Err(PortError::Failed(error)));
                }
                match l2a2_acceptance_take_worker_fault(Path::new(&request.destination), true) {
                    Ok(Some(message)) if message.starts_with("IpcDecode:") => {
                        if let Err(error) = self
                            .parakeet
                            .as_ref()?
                            .inject_incomplete_result_for_acceptance()
                        {
                            return Some(Err(PortError::Failed(error)));
                        }
                    }
                    Ok(Some(message)) => {
                        let process = self.parakeet.as_mut()?;
                        if let Err(error) = process.terminate() {
                            return Some(Err(PortError::Failed(format!(
                                "{message}; termination failed: {error}"
                            ))));
                        }
                    }
                    Ok(None) => {}
                    Err(error) => return Some(Err(PortError::Failed(error))),
                }
                Some(Ok(WorkerEvent::Ready {
                    job_id: active_job_id,
                    generation: active_generation,
                    instance_id: active_instance_id,
                    backend: expected_backend,
                }))
            }
            whisper_core::parakeet_ipc::Event::Result {
                request_id,
                job_id,
                generation,
                attempt,
                identity,
                source_sha256,
                duration_samples,
                words,
                gpu_evidence,
                ..
            } => {
                if self.parakeet_vad.is_some() {
                    return self.handle_parakeet_vad_result(
                        request_id,
                        job_id,
                        generation,
                        attempt,
                        identity,
                        source_sha256,
                        duration_samples,
                        words,
                    );
                }
                let Some(active) = self.active.as_ref() else {
                    return Some(Err(PortError::Failed(
                        "StaleResponse: Parakeet result without active import".into(),
                    )));
                };
                let Some(request) = active.import_request.as_ref() else {
                    return Some(Err(PortError::Failed(
                        "StaleResponse: Parakeet request is unavailable".into(),
                    )));
                };
                let current_identity = self.parakeet.as_ref().and_then(|p| p.identity());
                let expected_gpu = self.parakeet_expected_gpu.as_ref();
                let gpu_result_matches = if active.backend == BackendKind::Cuda {
                    self.parakeet.as_ref().is_some_and(|process| {
                        gpu_evidence_matches(gpu_evidence.as_ref(), expected_gpu, process.pid())
                    }) && self.parakeet_native_call_started
                } else {
                    gpu_evidence.is_none()
                };
                if !gpu_result_matches
                    || request_id != 1
                    || job_id != format!("{:032x}", active.job_id.0)
                    || generation != active.generation.get()
                    || attempt != active.attempt_number
                    || request
                        .source_sha256
                        .map(|sha| format_sha256(&sha))
                        .as_deref()
                        != Some(source_sha256.as_str())
                    || request.source_samples != Some(duration_samples)
                    || current_identity != Some(&identity)
                    || words.is_empty()
                {
                    return Some(Err(PortError::Failed(
                        "ProtocolMismatch: Parakeet result identity, duration, or words differ"
                            .into(),
                    )));
                }
                let (job_id, generation, instance_id) =
                    (active.job_id, active.generation, active.instance_id);
                self.parakeet_events.push_back(WorkerEvent::Progress {
                    job_id,
                    generation,
                    instance_id,
                    completed_samples: duration_samples,
                    total_samples: duration_samples,
                });
                let validation_started = Instant::now();
                let validated_words =
                    match validate_single_pass_parakeet_words(&words, duration_samples) {
                        Ok(words) => words,
                        Err(error) => return Some(Err(PortError::Failed(error))),
                    };
                crate::worker_ipc::record_r7_timing(
                    "application_word_validation",
                    validation_started.elapsed(),
                    serde_json::json!({
                        "input_words": words.len(),
                        "validated_words": validated_words.len(),
                    }),
                );
                let grouping_started = Instant::now();
                let mut groups: Vec<(u64, u64, Vec<String>)> = Vec::new();
                for (start, end, token) in validated_words {
                    if let Err(error) =
                        append_single_pass_parakeet_word_group(&mut groups, start, end, &token)
                    {
                        return Some(Err(PortError::Failed(error)));
                    }
                }
                crate::worker_ipc::record_r7_timing(
                    "application_word_grouping",
                    grouping_started.elapsed(),
                    serde_json::json!({ "groups": groups.len() }),
                );
                let last_sequence = groups.len() as u64;
                for (sequence, (start, end, tokens)) in groups.into_iter().enumerate() {
                    let Some(range) = whisper_core::SourceRange::new(start, end, 16_000) else {
                        return Some(Err(PortError::Failed(
                            "InvalidTimestamp: Parakeet sentence range is invalid".into(),
                        )));
                    };
                    self.parakeet_events.push_back(WorkerEvent::Segment(
                        whisper_core::ipc::WorkerSegmentDto {
                            job_id,
                            generation,
                            instance_id,
                            segment_id: whisper_core::SegmentId(sequence as u64 + 1),
                            range,
                            text: tokens.join(" "),
                        },
                    ));
                }
                self.parakeet_end = Some((job_id, generation, last_sequence, duration_samples));
                if let Some(process) = self.parakeet.as_mut()
                    && let Err(error) = process.shutdown()
                {
                    return Some(Err(PortError::Failed(error)));
                }
                self.parakeet_events.pop_front().map(Ok)
            }
            whisper_core::parakeet_ipc::Event::Failed {
                request_id,
                job_id: reported_job_id,
                generation: reported_generation,
                attempt,
                identity,
                code,
                message,
                ..
            } => {
                let Some(active) = self.active.as_ref() else {
                    return Some(Err(PortError::Failed(
                        "StaleResponse: Parakeet failure without active import".into(),
                    )));
                };
                if reported_job_id
                    .as_deref()
                    .is_some_and(|reported| reported != format!("{:032x}", active.job_id.0))
                    || reported_generation
                        .is_some_and(|reported| reported != active.generation.get())
                    || identity.as_ref().is_some_and(|reported| {
                        self.parakeet
                            .as_ref()
                            .and_then(|process| process.identity())
                            .is_some_and(|expected| expected != reported)
                    })
                {
                    return Some(Err(PortError::Failed(
                        "StaleResponse: Parakeet failure identity differs from active import"
                            .into(),
                    )));
                }
                let cpu_retry = self.active.as_ref().and_then(|active| {
                    let request = active.import_request.as_ref()?;
                    (active.backend == BackendKind::Cuda
                        && request.config.compute == whisper_core::ComputeChoice::Auto
                        && !self.parakeet_native_call_started
                        && request_id.is_none_or(|request_id| request_id == 1)
                        && attempt.is_none_or(|attempt| attempt == active.attempt_number))
                    .then(|| request.clone())
                });
                if let Some(request) = cpu_retry {
                    return match self.fallback_parakeet_to_cpu(
                        request,
                        format!("GPU worker failed before native inference: {code}: {message}"),
                    ) {
                        Ok(()) => None,
                        Err(error) => Some(Err(PortError::Failed(error))),
                    };
                }
                let (job_id, generation, instance_id) =
                    (active.job_id, active.generation, active.instance_id);
                if let Some(process) = self.parakeet.as_mut() {
                    let _ = process.terminate();
                }
                self.parakeet = None;
                self.parakeet_stopped_ack = false;
                Some(Ok(WorkerEvent::Failed {
                    job_id: Some(job_id),
                    generation: Some(generation),
                    instance_id: Some(instance_id),
                    code: match code.as_str() {
                        "ModelHashMismatch" => {
                            whisper_core::ipc::WorkerErrorCode::ModelHashMismatch
                        }
                        "ProtocolVersionMismatch" | "InvalidMessage" => {
                            whisper_core::ipc::WorkerErrorCode::ProtocolMismatch
                        }
                        _ => whisper_core::ipc::WorkerErrorCode::Internal,
                    },
                    message: format!("{code}: {message}"),
                }))
            }
            whisper_core::parakeet_ipc::Event::NativeCallStarted {
                request_id,
                job_id,
                generation,
                attempt,
                pid,
            } => {
                let valid = self.active.as_ref().is_some_and(|active| {
                    active.import_request.is_some()
                        && request_id == 1
                        && job_id == format!("{:032x}", active.job_id.0)
                        && generation == active.generation.get()
                        && attempt == active.attempt_number
                        && (active.backend != BackendKind::Cuda || active.gpu_attested)
                        && self
                            .parakeet
                            .as_ref()
                            .is_some_and(|process| process.pid() == pid)
                });
                if !valid {
                    return Some(Err(PortError::Failed(
                        "ProtocolMismatch: NativeCallStarted is not bound to the active Parakeet request".into(),
                    )));
                }
                self.parakeet_native_call_started = true;
                None
            }
            whisper_core::parakeet_ipc::Event::Stopped => {
                self.parakeet_stopped_ack = true;
                self.poll_parakeet_worker()
            }
        }
    }

    fn poll(&mut self) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
        if let Some(event) = self.poll_unified_sessions() {
            if let Err((job_id, generation, _)) = &event {
                self.release_unified_terminal_state(*job_id, *generation);
            }
            return Some(event);
        }
        if let Some(event) = self.poll_unified_document() {
            if let Err((job_id, generation, _)) = &event {
                self.release_unified_terminal_state(*job_id, *generation);
            }
            return Some(event);
        }
        if let Some(event) = self.parakeet_pending_events.pop_front() {
            return Some(Ok(event));
        }
        if let Some(test) = self.microphone_test.as_mut() {
            let identity = (test.job_id, test.generation);
            match test.pump() {
                Ok(Some(event)) => return Some(Ok(event)),
                Ok(None) => {}
                Err(error) => {
                    if let Some(mut test) = self.microphone_test.take() {
                        test.probe.invalidate();
                        test.capture.stop();
                    }
                    return Some(Err((identity.0, identity.1, error)));
                }
            }
        }
        if self
            .idle_worker
            .as_ref()
            .is_some_and(|idle| idle.expires_at <= Instant::now())
        {
            self.idle_worker = None;
        }
        if let Some(direct) = self.direct_live.as_mut() {
            let id = direct.identity();
            let result = direct.poll().map_err(|error| direct.fail(error));
            let done = direct.done() || result.is_err();
            if done {
                if direct.done() && result.is_ok() {
                    let (compute, model_path, model_sha256) = direct.cache_identity();
                    let model_path = model_path.to_owned();
                    if let Some(transport) = direct.take_idle_worker() {
                        self.idle_worker = Some(IdleWorker {
                            transport,
                            compute,
                            model_path,
                            model_sha256,
                            expires_at: Instant::now() + WORKER_IDLE_RETENTION,
                        });
                    }
                }
                self.direct_live = None;
            }
            match result {
                Ok(Some(event)) => return Some(Ok(event)),
                Ok(None) => {}
                Err(error) => return Some(Err((id.0, id.1, error))),
            }
        }
        if let Some(live) = self.parakeet_live.as_mut() {
            let id = live.identity();
            match live.poll() {
                Ok(Some(event)) => return Some(Ok(event)),
                Ok(None) => {}
                Err(error) => {
                    let event = live.stop_after_capture_error(error);
                    let event = attach_automatic_final_import(live, event);
                    self.parakeet_pending_events.push_back(event);
                    self.parakeet_live = None;
                    if let Some(event) = self.parakeet_pending_events.pop_front() {
                        return Some(Ok(event));
                    }
                    return Some(Err((id.0, id.1, "Capture Parakeet interrompue".into())));
                }
            }
        }
        if let Some(event) = self.pending_recovery_counters.take() {
            return Some(Ok(event));
        }
        if self.live.as_ref().is_some_and(|live| live.worker_ready)
            && let Some(event) = self.pending_attempt_reset.take()
        {
            return Some(Ok(event));
        }
        if let Some(progress) = self.progress.as_mut()
            && let Some((phase, idle_ms)) = progress.take_warning(std::time::Instant::now())
            && let Some(active) = self.active.as_ref()
        {
            if let Some(diagnostics) = &self.diagnostics {
                diagnostics.record(DiagnosticRecord::new(
                    active.job_id.0,
                    active.generation.get(),
                    phase.into(),
                    DiagnosticCode::ProgressWarning,
                    Some(match active.backend {
                        BackendKind::Cpu => "cpu",
                        BackendKind::Cuda => "cuda",
                    }),
                    None,
                ));
            }
            return Some(Ok(ImportEvent::Diagnostic {
                job_id: active.job_id,
                generation: active.generation,
                notice: DiagnosticNotice::ProgressStalled {
                    phase: phase.into(),
                    idle_ms,
                },
            }));
        }
        let diagnostics_degraded = self.diagnostic_start_failed
            || self
                .diagnostics
                .as_ref()
                .is_some_and(DiagnosticSink::is_degraded);
        if !self.diagnostic_degraded_reported
            && diagnostics_degraded
            && let Some(active) = self.active.as_ref()
        {
            self.diagnostic_degraded_reported = true;
            if let Some(diagnostics) = self.diagnostics.as_ref() {
                diagnostics.record(DiagnosticRecord::new(
                    active.job_id.0,
                    active.generation.get(),
                    DiagnosticPhase::Finalizing,
                    DiagnosticCode::LogDegraded,
                    Some(match active.backend {
                        BackendKind::Cpu => "cpu",
                        BackendKind::Cuda => "cuda",
                    }),
                    None,
                ));
            }
            return Some(Ok(ImportEvent::Diagnostic {
                job_id: active.job_id,
                generation: active.generation,
                notice: DiagnosticNotice::LoggingUnavailable,
            }));
        }
        if self
            .live
            .as_ref()
            .is_some_and(|live| live.telemetry_due_after_event)
        {
            if let Some(live) = self.live.as_mut() {
                live.telemetry_due_after_event = false;
            }
            if let Some(event) = self.take_legacy_live_telemetry() {
                return Some(Ok(event));
            }
        }
        let capture_may_publish = live_capture_poll_allowed(
            self.live_capture_events_since_worker_poll,
            self.transport.is_some(),
        );
        if capture_may_publish && let Some(event) = self.poll_live_capture() {
            if let Ok(ImportEvent::LiveCounters {
                captured_samples,
                audio_durable_samples,
                ..
            }) = event.as_ref()
                && let Some(progress) = self.progress.as_mut()
            {
                let now = std::time::Instant::now();
                progress.observe(ProgressPhase::Capturing, *captured_samples, now);
                progress.observe(ProgressPhase::Draining, *audio_durable_samples, now);
            }
            if let Some(live) = self.live.as_mut() {
                live.telemetry_due_after_event = true;
            }
            self.live_capture_events_since_worker_poll =
                self.live_capture_events_since_worker_poll.saturating_add(1);
            return Some(event);
        }
        if self.deferred_live_window_finished.is_some() && self.live_pending_persistence > 0 {
            return None;
        }
        if self
            .parakeet_vad
            .as_ref()
            .is_some_and(|run| run.decoder.is_some())
            && let Err(failure) = self.advance_parakeet_vad_decode()
        {
            if let Some(worker) = self.parakeet.as_mut() {
                let _ = worker.terminate();
            }
            self.parakeet = None;
            self.parakeet_vad = None;
            self.parakeet_events.clear();
            self.active = None;
            return Some(Err(failure));
        }
        if self
            .parakeet_vad
            .as_ref()
            .is_some_and(|run| run.decoder.is_none() && run.worker_ready && !run.ready_event_sent)
        {
            if let Err(failure) = self.begin_next_parakeet_vad_window() {
                if let Some(worker) = self.parakeet.as_mut() {
                    let _ = worker.terminate();
                }
                self.parakeet = None;
                self.parakeet_vad = None;
                self.parakeet_events.clear();
                self.active = None;
                return Some(Err(failure));
            }
            let Some(active) = self.active.as_ref() else {
                return None;
            };
            self.parakeet_events.push_back(WorkerEvent::Ready {
                job_id: active.job_id,
                generation: active.generation,
                instance_id: active.instance_id,
                backend: BackendKind::Cpu,
            });
            if let Some(run) = self.parakeet_vad.as_mut() {
                run.ready_event_sent = true;
            }
        }
        let mut event = if self.parakeet.is_some() || !self.parakeet_events.is_empty() {
            match self.poll_parakeet_worker() {
                Some(Ok(event)) => event,
                Some(Err(error)) => {
                    let active = self.active.as_ref()?;
                    let failure = (active.job_id, active.generation, format!("{error:?}"));
                    self.parakeet = None;
                    self.parakeet_stopped_ack = false;
                    self.parakeet_events.clear();
                    self.active = None;
                    return Some(Err(failure));
                }
                None => return None,
            }
        } else if self.deferred_live_stopped.is_some() {
            if self.live_pending_persistence > 0
                || self
                    .transport
                    .as_mut()
                    .and_then(|transport| transport.try_wait().ok().flatten())
                    .is_none()
            {
                return None;
            }
            take_ready_live_stop(
                self.live_pending_persistence,
                &mut self.deferred_live_stopped,
            )?
        } else {
            let transport = self.transport.as_mut()?;
            let received = transport.receive();
            self.live_capture_events_since_worker_poll = 0;
            match received {
                Ok(Some(event)) => event,
                Ok(None) => {
                    let _ = transport.try_wait();
                    return self.take_legacy_live_telemetry().map(Ok);
                }
                Err(error) => {
                    let active = self.active.as_ref()?;
                    let (job_id, generation, instance_id) =
                        (active.job_id, active.generation, active.instance_id);
                    let recovery_session = self.live.is_some()
                        || (active.backend == BackendKind::Cuda
                            && (self.pending_live.is_some()
                                || active.import_request.as_ref().is_some_and(|request| {
                                    request.config.compute == whisper_core::ComputeChoice::Auto
                                })))
                        || active.recovery != LiveRecoveryState::None
                        || self
                            .pending_live
                            .as_ref()
                            .is_some_and(|pending| pending.recovery != LiveRecoveryState::None);
                    if recovery_session {
                        self.deferred_live_stopped = Some(WorkerEvent::Stopped {
                            job_id,
                            generation,
                            instance_id,
                        });
                        return Some(self.handle_worker_failure(
                            job_id,
                            generation,
                            whisper_core::ipc::WorkerErrorCode::WorkerExited,
                            format!("WorkerExited without End: {error:?}"),
                        ));
                    }
                    let failure = Err((
                        job_id,
                        generation,
                        format!("WorkerExited without End: {error:?}"),
                    ));
                    if let Some(live) = self.live.as_mut() {
                        live.stop_capture();
                        let _ = live.staging.drain();
                    }
                    self.live = None;
                    self.live_archive = None;
                    self.transport = None;
                    self.active = None;
                    return Some(failure);
                }
            }
        };
        let active = self.active.as_ref()?;
        let (job_id, generation, instance_id) =
            (active.job_id, active.generation, active.instance_id);
        // A failure received on the newly launched worker's own transport before
        // Ready may omit its identity. Bind only a wholly uncorrelated startup
        // failure to this attempt; partially or fully stale identities stay rejected.
        event = bind_unidentified_startup_failure(
            event,
            (job_id, generation, instance_id),
            active.startup_ready,
        );
        if worker_event_is_stale(&event, (job_id, generation, instance_id)) {
            return None;
        }
        if let WorkerEvent::ImportStarted { .. } = &event {
            let Some(active) = self.active.as_mut().filter(|active| {
                active.job_id == job_id
                    && active.generation == generation
                    && active.instance_id == instance_id
                    && active.backend == BackendKind::Cuda
                    && active.import_request.as_ref().is_some_and(|request| {
                        request.config.compute == whisper_core::ComputeChoice::Auto
                    })
                    && !active.startup_ready
            }) else {
                return Some(Err((
                    job_id,
                    generation,
                    "ProtocolMismatch: unexpected Auto import admission acknowledgement".into(),
                )));
            };
            active.startup_ready = true;
            return None;
        }
        if matches!(
            &event,
            WorkerEvent::Failed { job_id: None, .. }
                | WorkerEvent::Failed {
                    generation: None,
                    ..
                }
                | WorkerEvent::Failed {
                    instance_id: None,
                    ..
                }
        ) {
            return Some(self.handle_worker_failure(
                job_id,
                generation,
                whisper_core::ipc::WorkerErrorCode::ProtocolMismatch,
                "ProtocolMismatch: worker failure lacks current attempt identity".into(),
            ));
        }
        if let Some(diagnostics) = &self.diagnostics {
            let backend = Some(match active.backend {
                BackendKind::Cpu => "cpu",
                BackendKind::Cuda => "cuda",
            });
            let record = match &event {
                WorkerEvent::Ready { .. } => Some(DiagnosticRecord::new(
                    job_id.0,
                    generation.get(),
                    DiagnosticPhase::Preparing,
                    DiagnosticCode::BackendReady,
                    backend,
                    None,
                )),
                WorkerEvent::Progress {
                    completed_samples,
                    total_samples,
                    ..
                } => Some(DiagnosticRecord::new(
                    job_id.0,
                    generation.get(),
                    if self.live.is_some() {
                        DiagnosticPhase::Inference
                    } else {
                        DiagnosticPhase::Import
                    },
                    DiagnosticCode::Progress,
                    backend,
                    Some((*completed_samples, *total_samples)),
                )),
                WorkerEvent::Failed { .. } => Some(DiagnosticRecord::new(
                    job_id.0,
                    generation.get(),
                    DiagnosticPhase::Finalizing,
                    DiagnosticCode::WorkerFailed,
                    backend,
                    None,
                )),
                WorkerEvent::Stopped { .. } => Some(DiagnosticRecord::new(
                    job_id.0,
                    generation.get(),
                    DiagnosticPhase::Cancelling,
                    DiagnosticCode::WorkerStopped,
                    backend,
                    None,
                )),
                _ => None,
            };
            if let Some(record) = record {
                diagnostics.record(record);
            }
        }
        if let Some(progress) = self.progress.as_mut() {
            match &event {
                WorkerEvent::Ready { .. } => {
                    let now = std::time::Instant::now();
                    progress.close(ProgressPhase::Preparing);
                    progress.open(ProgressPhase::Inference, now);
                    if self.live.is_some() || self.pending_live.is_some() {
                        progress.open(ProgressPhase::Capturing, now);
                        progress.open(ProgressPhase::Draining, now);
                    } else {
                        progress.open(ProgressPhase::Import, now);
                    }
                }
                WorkerEvent::Progress {
                    completed_samples, ..
                } => {
                    let now = std::time::Instant::now();
                    progress.observe(ProgressPhase::Inference, *completed_samples, now);
                    progress.observe(ProgressPhase::Import, *completed_samples, now);
                }
                WorkerEvent::Segment(dto) => progress.observe(
                    ProgressPhase::Inference,
                    dto.segment_id.0,
                    std::time::Instant::now(),
                ),
                WorkerEvent::End { .. } => {
                    let now = std::time::Instant::now();
                    progress.close(ProgressPhase::Inference);
                    progress.close(ProgressPhase::Import);
                    progress.open(ProgressPhase::Finalizing, now);
                }
                WorkerEvent::Stopped { .. } | WorkerEvent::Failed { .. } => progress.close_all(),
                _ => {}
            }
        }
        if matches!(
            &event,
            WorkerEvent::Stopped {
                job_id: stopped_job,
                generation: stopped_generation,
                instance_id: stopped_instance,
            } if (*stopped_job, *stopped_generation, *stopped_instance)
                == (job_id, generation, instance_id)
        ) && (self.live.is_some()
            || self
                .pending_live
                .as_ref()
                .is_some_and(|pending| pending.recovery != LiveRecoveryState::None)
            || self
                .active
                .as_ref()
                .is_some_and(|active| active.recovery != LiveRecoveryState::None))
            && (self.live_pending_persistence > 0
                || self
                    .transport
                    .as_mut()
                    .and_then(|transport| transport.try_wait().ok().flatten())
                    .is_none())
        {
            self.deferred_live_stopped = Some(event);
            return None;
        }
        if let WorkerEvent::LiveMp3Packet {
            job_id: packet_job,
            generation: packet_generation,
            instance_id: packet_instance,
            bytes,
            ..
        } = &event
        {
            if (*packet_job, *packet_generation, *packet_instance)
                != (job_id, generation, instance_id)
            {
                return None;
            }
            if let Some(archive) = self.live_archive.as_mut() {
                if let Err(error) = archive.append_mp3_packet(bytes) {
                    if let Some(live) = self.live.as_mut() {
                        live.stop_capture();
                        let _ = live.staging.drain();
                        live.failure = Some(error.clone());
                    }
                    if let Some(transport) = self.transport.as_mut() {
                        let _ = transport.try_request(WorkerCommand::Stop { job_id, generation });
                    }
                    return None;
                }
                return None;
            }
        }
        if let WorkerEvent::WindowFinished {
            sequence,
            range,
            last_segment_sequence,
            ..
        } = &event
        {
            if self.live.is_none() {
                return Some(Err((
                    job_id,
                    generation,
                    "ProtocolMismatch: WindowFinished outside a live pass".into(),
                )));
            }
            let Some(live) = self.live.as_ref() else {
                unreachable!()
            };
            // Confirmations wholly inside T were already durable before this attempt.
            // Replay only rebuilds the MP3 encoder and must not create duplicate coverage.
            if range.end_sample <= live.replay_until {
                return None;
            }
            let Some(window_sequence) = map_attempt_sequence(
                live.window_sequence_base,
                live.replay_windows_to_skip,
                *sequence,
            ) else {
                return Some(Err((
                    job_id,
                    generation,
                    "ProtocolMismatch: live window sequence cannot be mapped to durable prefix"
                        .into(),
                )));
            };
            let last_segment_sequence = if *last_segment_sequence == 0 {
                live.segment_sequence_base.checked_sub(1)
            } else {
                map_attempt_sequence(live.segment_sequence_base, 0, *last_segment_sequence)
            };
            let Some(last_segment_sequence) = last_segment_sequence else {
                return Some(Err((
                    job_id,
                    generation,
                    "ProtocolMismatch: live segment sequence cannot be mapped to durable prefix"
                        .into(),
                )));
            };
            let finished = FinishedLiveWindow {
                sequence: window_sequence,
                range: *range,
                last_segment_sequence,
            };
            if self.live_pending_persistence > 0 {
                self.deferred_live_window_finished = Some(finished);
                return None;
            }
            return Some(self.confirm_live_window(finished));
        }
        if matches!(event, WorkerEvent::Stopped { .. })
            && self.live.is_none()
            && self
                .pending_live
                .as_ref()
                .is_some_and(|pending| pending.recovery != LiveRecoveryState::None)
        {
            let recovery = self.pending_live.as_ref().map(|pending| pending.recovery);
            if let Some(pending) = self.pending_live.as_mut()
                && let Err(error) = pending.live_archive.retire_attempt(
                    self.active
                        .as_ref()
                        .map_or(0, |active| active.attempt_number),
                )
            {
                self.pending_live = None;
                self.active = None;
                self.transport = None;
                return Some(Err((job_id, generation, error)));
            }
            self.active = None;
            self.transport = None;
            if self.publication_gate.is_cancelled() {
                self.pending_live = None;
                return Some(Ok(ImportEvent::Stopped { job_id, generation }));
            }
            if recovery == Some(LiveRecoveryState::AwaitingChoicePending) {
                if let Some(pending) = self.pending_live.as_mut() {
                    pending.recovery = LiveRecoveryState::AwaitingChoice;
                }
                return Some(Ok(ImportEvent::AwaitingCpuChoice { job_id, generation }));
            }
            return Some(
                match self.start_cpu_pending_live_attempt(job_id, generation) {
                    Ok(event) => Ok(event),
                    Err(error) => Err(error),
                },
            );
        }
        if matches!(event, WorkerEvent::Stopped { .. })
            && self.live.is_none()
            && self.active.as_ref().is_some_and(|active| {
                active.recovery == LiveRecoveryState::AutoRetryPending
                    && active.import_request.is_some()
            })
        {
            let request = self
                .active
                .as_ref()
                .and_then(|active| active.import_request.clone());
            self.active = None;
            self.transport = None;
            if self.publication_gate.is_cancelled() {
                return Some(Ok(ImportEvent::Stopped { job_id, generation }));
            }
            return Some(
                match request.map(|request| self.start_worker_with_cpu_override(request, true)) {
                    Some(Ok(())) => Ok(ImportEvent::LiveAttemptStarting { job_id, generation }),
                    Some(Err(error)) => Err(error),
                    None => Err((
                        job_id,
                        generation,
                        "ProtocolMismatch: retry request unavailable".into(),
                    )),
                },
            );
        }
        if matches!(event, WorkerEvent::Stopped { .. }) && self.live.is_some() {
            if let Some(reason) = self.live.as_ref().and_then(|live| live.failure.clone()) {
                if let Some(live) = self.live.as_mut() {
                    live.stop_capture();
                    let _ = live.staging.drain();
                }
                self.live = None;
                self.live_archive = None;
                self.transport = None;
                self.active = None;
                return Some(Err((job_id, generation, reason)));
            }
            if let Err(error) = crate::archive::l04_campaign_gate(
                "pcm.before_final_drain",
                job_id.0,
                generation.get(),
                0,
            ) {
                return Some(Err((job_id, generation, error)));
            }
            let final_drain = self.live.as_mut().map(|live| live.staging.drain());
            if let Some(result) = &final_drain
                && let Err(error) = crate::archive::l04_campaign_operation_result(
                    "pcm.before_final_drain",
                    job_id.0,
                    generation.get(),
                    0,
                    result,
                    None,
                )
            {
                return Some(Err((job_id, generation, error)));
            }
            if let Some(Err(error)) = final_drain {
                if let Some(live) = self.live.as_mut() {
                    live.stop_capture();
                }
                self.live = None;
                self.live_archive = None;
                self.transport = None;
                self.active = None;
                return Some(Err((
                    job_id,
                    generation,
                    format!("StorageUnavailable: final audio sync: {error}"),
                )));
            }
            if let Err(error) = crate::archive::l04_campaign_gate(
                "pcm.after_final_drain",
                job_id.0,
                generation.get(),
                0,
            ) {
                return Some(Err((job_id, generation, error)));
            }
            let (durable, confirmed, pcm_sha256) =
                self.live.as_ref().map_or((0, 0, [0; 32]), |live| {
                    (
                        live.staging.durable_samples(),
                        live.confirmed_fragment_end,
                        live.staging.checksum(),
                    )
                });
            let recovery = self.live.as_ref().map(|live| live.recovery);
            if matches!(
                recovery,
                Some(
                    LiveRecoveryState::AutoRetryPending | LiveRecoveryState::AwaitingChoicePending
                )
            ) {
                let barrier_valid = self.live.as_ref().is_some_and(|live| {
                    live.barrier_m.is_some_and(|watermark| {
                        watermark <= live.inference_sequence.saturating_sub(1)
                    })
                });
                let coverage_valid = self
                    .live_archive
                    .as_ref()
                    .is_some_and(|archive| archive.confirmed_samples() <= durable);
                if !barrier_valid || !coverage_valid {
                    self.live = None;
                    self.live_archive = None;
                    self.transport = None;
                    self.active = None;
                    return Some(Err((
                        job_id,
                        generation,
                        "StorageCorrupt: retry barrier did not settle a valid confirmed prefix"
                            .into(),
                    )));
                }
                let attempt_number = self
                    .active
                    .as_ref()
                    .map_or(0, |active| active.attempt_number);
                if let Some(archive) = self.live_archive.as_mut()
                    && let Err(error) = archive.retire_attempt(attempt_number)
                {
                    self.live = None;
                    self.live_archive = None;
                    self.transport = None;
                    self.active = None;
                    return Some(Err((job_id, generation, error)));
                }
                self.transport = None;
                self.active = None;
                if self.publication_gate.is_cancelled() {
                    if let Some(live) = self.live.as_mut() {
                        live.stop_capture();
                    }
                    self.live = None;
                    self.live_archive = None;
                    return Some(Ok(ImportEvent::Stopped { job_id, generation }));
                }
                if recovery == Some(LiveRecoveryState::AwaitingChoicePending) {
                    if let Some(live) = self.live.as_mut() {
                        live.recovery = LiveRecoveryState::AwaitingChoice;
                        live.inference_admission_closed = true;
                    }
                    return Some(Ok(ImportEvent::AwaitingCpuChoice { job_id, generation }));
                }
                let stop_was_latched = self.live.as_ref().is_some_and(|live| live.stop_requested)
                    || self.publication_gate.is_cancelled();
                if stop_was_latched {
                    if let Some(live) = self.live.as_mut() {
                        live.stop_capture();
                    }
                    self.live = None;
                    self.live_archive = None;
                    return Some(Ok(ImportEvent::Stopped { job_id, generation }));
                }
                return Some(self.start_cpu_live_attempt(job_id, generation));
            }
            let publish_result = self
                .live_archive
                .as_mut()
                .map(|archive| archive.publish(durable, confirmed, pcm_sha256));
            if let Some(Err(error)) = publish_result {
                if let Some(live) = self.live.as_mut() {
                    live.stop_capture();
                }
                self.live = None;
                self.live_archive = None;
                self.transport = None;
                self.active = None;
                return Some(Err((job_id, generation, error)));
            }
            self.live = None;
            self.live_archive = None;
            self.transport = None;
            self.active = None;
            return Some(Ok(ImportEvent::Published { job_id, generation }));
        }
        if let WorkerEvent::Segment(dto) = &event
            && let Some(expected) = self.resume_prefix.pop_front()
        {
            if dto.segment_id.0 != expected.sequence
                || dto.range != expected.range
                || dto.text != expected.text
            {
                self.transport = None;
                self.active = None;
                self.resume_prefix.clear();
                return Some(Err((
                    job_id,
                    generation,
                    "ResumeMismatch: recomputed segment differs from the confirmed prefix".into(),
                )));
            }
            return None;
        }
        if matches!(event, WorkerEvent::End { .. }) && !self.resume_prefix.is_empty() {
            self.transport = None;
            self.active = None;
            self.resume_prefix.clear();
            return Some(Err((
                job_id,
                generation,
                "ResumeMismatch: worker ended before reproducing the confirmed prefix".into(),
            )));
        }
        let result = match event {
            WorkerEvent::GpuSessionAttested {
                job_id: j,
                generation: g,
                instance_id: i,
                candidate,
                dxgi_rank,
                luid,
                pci_id,
                cuda_ordinal,
                ggml_ordinal,
            } if (j, g, i) == (job_id, generation, instance_id) => {
                let matches_candidate = self.active.as_ref().is_some_and(|active| {
                    active.gpu_candidate.as_ref().is_some_and(|expected| {
                        expected.pnp_instance_id == candidate
                            && expected.dxgi_rank == dxgi_rank
                            && expected.luid == luid
                            && expected.pci_id.eq_ignore_ascii_case(&pci_id)
                            && expected.cuda_ordinal == cuda_ordinal
                            && expected.ggml_ordinal == ggml_ordinal
                    })
                });
                if matches_candidate {
                    if let Some(active) = self.active.as_mut() {
                        active.gpu_attested = true;
                    }
                    return None;
                } else {
                    self.handle_worker_failure(
                        j,
                        g,
                        whisper_core::ipc::WorkerErrorCode::BackendMismatch,
                        "BackendMismatch: worker attested a different GPU candidate".into(),
                    )
                }
            }
            WorkerEvent::Ready {
                job_id: j,
                generation: g,
                instance_id: i,
                backend,
            } if (j, g, i) == (job_id, generation, instance_id)
                && self
                    .active
                    .as_ref()
                    .is_some_and(|active| active.backend == backend) =>
            {
                if backend == BackendKind::Cuda
                    && self.active.as_ref().is_some_and(|active| {
                        active.gpu_candidate.is_some() && !active.gpu_attested
                    })
                {
                    return Some(self.handle_worker_failure(
                        j,
                        g,
                        whisper_core::ipc::WorkerErrorCode::BackendMismatch,
                        "BackendMismatch: GPU Ready arrived before candidate attestation".into(),
                    ));
                }
                // The generic Turbo GPU worker pauses at Ready and requires a
                // BeginImport command. Parakeet's separate IPC has no such
                // command: poll_parakeet_worker already sent Transcribe with
                // the correlated attempt before returning this Ready event.
                let auto_gpu_import = backend == BackendKind::Cuda
                    && self.parakeet.is_none()
                    && self
                        .active
                        .as_ref()
                        .and_then(|active| active.import_request.as_ref())
                        .is_some_and(|request| {
                            request.config.compute == whisper_core::ComputeChoice::Auto
                        });
                if !auto_gpu_import && let Some(active) = self.active.as_mut() {
                    active.startup_ready = true;
                }
                if auto_gpu_import && !self.publication_gate.is_cancelled() {
                    let begin = WorkerCommand::BeginImport {
                        job_id: j,
                        generation: g,
                        instance_id: i,
                    };
                    if let Err(error) = self
                        .transport
                        .as_mut()
                        .expect("ready worker")
                        .request(begin)
                    {
                        return Some(self.handle_worker_failure(
                            j,
                            g,
                            whisper_core::ipc::WorkerErrorCode::WorkerExited,
                            format!("WorkerExited before BeginImport admission: {error:?}"),
                        ));
                    }
                }
                if let Some(live) = self
                    .live
                    .as_mut()
                    .filter(|live| live.request.job_id == j && live.request.generation == g)
                {
                    live.worker_ready = true;
                }
                if self.pending_live.is_some()
                    && !self.publication_gate.is_cancelled()
                    && let Err(failure) = self.activate_live_after_ready()
                {
                    if let Some(transport) = self.transport.as_mut() {
                        let _ = transport.try_request(WorkerCommand::Stop {
                            job_id: j,
                            generation: g,
                        });
                    }
                    Err(failure)
                } else {
                    Ok(ImportEvent::Ready {
                        job_id: j,
                        generation: g,
                        backend: match backend {
                            BackendKind::Cpu => "CPU",
                            BackendKind::Cuda => "CUDA",
                        }
                        .to_owned()
                            + &self
                                .auto_fallback_reason
                                .as_ref()
                                .filter(|_| backend == BackendKind::Cpu)
                                .map(|reason| {
                                    format!(
                                        " — GPU non utilisable ({reason}) ; CPU prend le relais"
                                    )
                                })
                                .unwrap_or_default(),
                    })
                }
            }
            WorkerEvent::Ready { .. } => self.handle_worker_failure(
                job_id,
                generation,
                whisper_core::ipc::WorkerErrorCode::BackendMismatch,
                "BackendMismatch: worker attestation differs from the selected backend".into(),
            ),
            WorkerEvent::Progress {
                job_id: j,
                generation: g,
                instance_id: i,
                completed_samples,
                total_samples,
            } if (j, g, i) == (job_id, generation, instance_id) => Ok(ImportEvent::Progress {
                job_id: j,
                generation: g,
                completed_samples,
                total_samples,
            }),
            WorkerEvent::Segment(dto)
                if (dto.job_id, dto.generation, dto.instance_id)
                    == (job_id, generation, instance_id) =>
            {
                let sequence_base = self
                    .live
                    .as_ref()
                    .filter(|live| {
                        live.request.job_id == job_id && live.request.generation == generation
                    })
                    .map_or(1, |live| live.segment_sequence_base);
                let Some(segment_sequence) =
                    map_attempt_sequence(sequence_base, 0, dto.segment_id.0)
                else {
                    return Some(Err((
                        job_id,
                        generation,
                        "ProtocolMismatch: live segment sequence exhausted".into(),
                    )));
                };
                let segment = WorkerSegment {
                    job_id: dto.job_id,
                    generation: dto.generation,
                    instance_id: dto.instance_id,
                    segment_id: whisper_core::SegmentId(segment_sequence),
                    range: dto.range,
                    text: dto.text,
                };
                if let Some(live) = self.live.as_ref().filter(|live| {
                    live.request.job_id == job_id && live.request.generation == generation
                }) {
                    self.live_pending_persistence = self.live_pending_persistence.saturating_add(1);
                    let captured_at_unix_ms = live
                        .started_at_unix_ms
                        .saturating_add(segment.range.start_sample.saturating_mul(1_000) / 16_000);
                    Ok(ImportEvent::LiveProvisional {
                        segment,
                        captured_at_unix_ms,
                        available_at_unix_ms: unix_time_ms(),
                    })
                } else {
                    Ok(ImportEvent::Segment(segment))
                }
            }
            WorkerEvent::End {
                job_id: j,
                generation: g,
                instance_id: i,
                last_sequence,
                last_offset,
            } if (j, g, i) == (job_id, generation, instance_id) => {
                if let (Some(active), Some(transport)) =
                    (self.active.as_ref(), self.transport.take())
                    && let Some(request) = active.import_request.as_ref()
                {
                    self.pending_idle_worker = Some(IdleWorker {
                        transport,
                        compute: match active.backend {
                            BackendKind::Cpu => whisper_core::ComputeChoice::Cpu,
                            BackendKind::Cuda => whisper_core::ComputeChoice::Gpu,
                        },
                        model_path: request.model_path.clone(),
                        model_sha256: request.model_sha256,
                        expires_at: Instant::now(),
                    });
                }
                self.active = None;
                Ok(ImportEvent::End {
                    job_id: j,
                    generation: g,
                    last_sequence,
                    last_offset,
                })
            }
            WorkerEvent::Stopped {
                job_id: j,
                generation: g,
                instance_id: i,
            } if (j, g, i) == (job_id, generation, instance_id) => {
                self.transport = None;
                self.active = None;
                self.pending_live = None;
                self.live_archive = None;
                Ok(ImportEvent::Stopped {
                    job_id: j,
                    generation: g,
                })
            }
            WorkerEvent::Failed {
                job_id: Some(j),
                generation: Some(g),
                instance_id: Some(i),
                message,
                code,
            } if (j, g, i) == (job_id, generation, instance_id) => {
                self.handle_worker_failure(j, g, code, message)
            }
            WorkerEvent::Failed { .. } => Err((
                job_id,
                generation,
                "ProtocolMismatch: worker failure lacks current attempt identity".into(),
            )),
            _ => Err((
                job_id,
                generation,
                "StaleResponse: worker event identity mismatch".into(),
            )),
        };
        if let Err((failed_job, failed_generation, message)) = &result
            && let Some(live) = self.live.as_mut()
            && (live.request.job_id, live.request.generation) == (*failed_job, *failed_generation)
        {
            live.stop_capture();
            let _ = live.staging.drain();
            live.failure = Some(message.clone());
            if let Some(transport) = self.transport.as_mut() {
                let _ = transport.try_request(WorkerCommand::Stop { job_id, generation });
            }
            return None;
        }
        if result.as_ref().is_ok_and(|event| {
            !matches!(
                event,
                ImportEvent::End { .. }
                    | ImportEvent::Failed { .. }
                    | ImportEvent::Stopped { .. }
                    | ImportEvent::Published { .. }
            )
        }) && let Some(live) = self
            .live
            .as_mut()
            .filter(|live| (live.request.job_id, live.request.generation) == (job_id, generation))
        {
            live.telemetry_due_after_event = true;
        }
        Some(result)
    }

    fn take_legacy_live_telemetry(&mut self) -> Option<ImportEvent> {
        let live = self.live.as_mut()?;
        if live.next_spectrum_telemetry {
            live.next_spectrum_telemetry = false;
            if let Some(update) = live.spectrum_reader.poll(&live.probe, Instant::now()) {
                return Some(ImportEvent::MicrophoneSpectrum(update));
            }
        } else {
            live.next_spectrum_telemetry = true;
        }
        let snapshot = live.probe.latest_at(Instant::now())?;
        if snapshot.sequence <= live.last_meter_sequence {
            return None;
        }
        let meter = snapshot.meter()?;
        live.last_meter_sequence = snapshot.sequence;
        Some(ImportEvent::MicrophoneMeter(meter))
    }
}

pub struct AsyncImportIo {
    effects: SyncSender<ImportEffect>,
    stops: SyncSender<ImportEffect>,
    events: Receiver<ImportEvent>,
    publication_gate: Arc<PublicationGate>,
    active_identity: Option<(JobId, Generation)>,
    unified_identities: HashSet<(JobId, u64)>,
    automatic_effects: SyncSender<ImportEffect>,
    automatic_stops: SyncSender<ImportEffect>,
    automatic_events: Receiver<ImportEvent>,
    automatic_publication_gates: Arc<Mutex<HashMap<(JobId, u64), Arc<PublicationGate>>>>,
    automatic_identities: HashSet<(JobId, u64)>,
    automatic_active_identity: Option<(JobId, u64)>,
    automatic_queue: VecDeque<whisper_core::ports::AutomaticFinalImport>,
    pending_automatic_failures: VecDeque<ImportEvent>,
    poll_automatic_first: bool,
}

impl AsyncImportIo {
    pub fn start(worker_path: PathBuf) -> Self {
        let gpu = worker_path
            .parent()
            .map(|parent| parent.join("whisper-worker-gpu.exe"))
            .unwrap_or_else(|| PathBuf::from("whisper-worker-gpu.exe"));
        Self::start_with_workers(WorkerPaths {
            cpu: worker_path,
            gpu,
        })
    }

    pub fn start_with_workers(workers: WorkerPaths) -> Self {
        Self::start_inner(workers, None, None, None, None, false)
    }

    /// Starts the import runtime with a catalog guard used by all model-bearing effects.
    pub fn start_with_model_catalog(
        workers: WorkerPaths,
        catalog_root: PathBuf,
        known_historical_turbo_path: String,
    ) -> Self {
        Self::start_inner(
            workers,
            Some(RuntimeModelCatalog {
                catalog: crate::model_catalog::LocalModelCatalog::new(catalog_root),
                known_historical_turbo_path,
                parakeet_paths: None,
                gpu_parakeet_paths: None,
            }),
            None,
            None,
            None,
            false,
        )
    }

    pub fn start_with_model_catalog_and_parakeet(
        workers: WorkerPaths,
        catalog_root: PathBuf,
        known_historical_turbo_path: String,
        parakeet_paths: Option<crate::parakeet_import::ParakeetWorkerPaths>,
    ) -> Self {
        Self::start_with_model_catalog_and_parakeet_routes(
            workers,
            catalog_root,
            known_historical_turbo_path,
            parakeet_paths,
            None,
        )
    }

    pub fn start_with_model_catalog_and_parakeet_routes(
        workers: WorkerPaths,
        catalog_root: PathBuf,
        known_historical_turbo_path: String,
        cpu_parakeet_paths: Option<crate::parakeet_import::ParakeetWorkerPaths>,
        gpu_parakeet_paths: Option<crate::parakeet_import::ParakeetWorkerPaths>,
    ) -> Self {
        Self::start_with_model_catalog_and_parakeet_routes_inner(
            workers,
            catalog_root,
            known_historical_turbo_path,
            cpu_parakeet_paths,
            gpu_parakeet_paths,
            None,
            false,
        )
    }

    /// Starts the desktop import runtime with an explicitly injected durable
    /// snapshot registry. A missing application settings root is fail-closed.
    pub fn start_with_model_catalog_and_parakeet_routes_and_snapshot_registry(
        workers: WorkerPaths,
        catalog_root: PathBuf,
        known_historical_turbo_path: String,
        cpu_parakeet_paths: Option<crate::parakeet_import::ParakeetWorkerPaths>,
        gpu_parakeet_paths: Option<crate::parakeet_import::ParakeetWorkerPaths>,
        snapshot_registry_root: Option<PathBuf>,
    ) -> Self {
        Self::start_with_model_catalog_and_parakeet_routes_inner(
            workers,
            catalog_root,
            known_historical_turbo_path,
            cpu_parakeet_paths,
            gpu_parakeet_paths,
            snapshot_registry_root,
            true,
        )
    }

    fn start_with_model_catalog_and_parakeet_routes_inner(
        workers: WorkerPaths,
        catalog_root: PathBuf,
        known_historical_turbo_path: String,
        cpu_parakeet_paths: Option<crate::parakeet_import::ParakeetWorkerPaths>,
        gpu_parakeet_paths: Option<crate::parakeet_import::ParakeetWorkerPaths>,
        snapshot_registry_root: Option<PathBuf>,
        require_snapshot_registry: bool,
    ) -> Self {
        Self::start_inner(
            workers,
            Some(RuntimeModelCatalog {
                catalog: crate::model_catalog::LocalModelCatalog::new(catalog_root),
                known_historical_turbo_path,
                parakeet_paths: cpu_parakeet_paths.clone(),
                gpu_parakeet_paths: gpu_parakeet_paths.clone(),
            }),
            cpu_parakeet_paths,
            gpu_parakeet_paths,
            snapshot_registry_root,
            require_snapshot_registry,
        )
    }

    fn start_inner(
        workers: WorkerPaths,
        model_catalog: Option<RuntimeModelCatalog>,
        parakeet_paths: Option<crate::parakeet_import::ParakeetWorkerPaths>,
        gpu_parakeet_paths: Option<crate::parakeet_import::ParakeetWorkerPaths>,
        snapshot_registry_root: Option<PathBuf>,
        require_snapshot_registry: bool,
    ) -> Self {
        let (effect_tx, effect_rx) = mpsc::sync_channel(8);
        let (stop_tx, stop_rx) = mpsc::sync_channel(1);
        let (event_tx, event_rx) = mpsc::sync_channel(EVENTS_PER_STAGE);
        let (automatic_effect_tx, automatic_effect_rx) = mpsc::sync_channel(8);
        let (automatic_stop_tx, automatic_stop_rx) = mpsc::sync_channel(1);
        let (automatic_event_tx, automatic_event_rx) = mpsc::sync_channel(EVENTS_PER_STAGE);
        #[cfg(feature = "l01-memory-qualification")]
        memory_trace(
            "adapter.channels.created",
            serde_json::json!({
                "effect_channel_capacity": 8,
                "stop_channel_capacity": 1,
                "app_event_channel_capacity": EVENTS_PER_STAGE,
                "event_slot_bytes": std::mem::size_of::<ImportEvent>(),
                "pending_event_limit": EVENTS_PER_STAGE,
            }),
        );
        let publication_gate = Arc::new(PublicationGate::default());
        let automatic_publication_gates = Arc::new(Mutex::new(HashMap::new()));
        let automatic_model_catalog = model_catalog
            .as_ref()
            .map(RuntimeModelCatalog::clone_for_worker);
        let automatic_workers = workers.clone();
        let automatic_parakeet_paths = parakeet_paths.clone();
        let automatic_gpu_parakeet_paths = gpu_parakeet_paths.clone();
        let automatic_snapshot_registry_root = snapshot_registry_root.clone();
        thread::spawn({
            let publication_gate = publication_gate.clone();
            move || {
                let runtime = ImportRuntime::new_with_workers_and_parakeet_registry(
                    workers,
                    publication_gate,
                    model_catalog,
                    parakeet_paths,
                    gpu_parakeet_paths,
                    snapshot_registry_root,
                    require_snapshot_registry,
                );
                service_loop(effect_rx, stop_rx, event_tx, runtime)
            }
        });
        let _automatic_service = thread::Builder::new()
            .name("parakeet-auto-final-import".into())
            .spawn({
                let automatic_publication_gates = automatic_publication_gates.clone();
                move || {
                    let mut runtime = ImportRuntime::new_with_workers_and_parakeet_registry(
                        automatic_workers,
                        Arc::new(PublicationGate::default()),
                        automatic_model_catalog,
                        automatic_parakeet_paths,
                        automatic_gpu_parakeet_paths,
                        automatic_snapshot_registry_root,
                        require_snapshot_registry,
                    );
                    runtime.automatic_publication_gates = Some(automatic_publication_gates);
                    service_loop(
                        automatic_effect_rx,
                        automatic_stop_rx,
                        automatic_event_tx,
                        runtime,
                    )
                }
            });
        Self {
            effects: effect_tx,
            stops: stop_tx,
            events: event_rx,
            publication_gate,
            active_identity: None,
            unified_identities: HashSet::new(),
            automatic_effects: automatic_effect_tx,
            automatic_stops: automatic_stop_tx,
            automatic_events: automatic_event_rx,
            automatic_publication_gates,
            automatic_identities: HashSet::new(),
            automatic_active_identity: None,
            automatic_queue: VecDeque::new(),
            pending_automatic_failures: VecDeque::new(),
            poll_automatic_first: false,
        }
    }

    #[cfg(test)]
    fn for_test(
        publication_gate: Arc<PublicationGate>,
    ) -> (
        Self,
        Receiver<ImportEffect>,
        Receiver<ImportEffect>,
        SyncSender<ImportEvent>,
        Receiver<ImportEffect>,
        Receiver<ImportEffect>,
        SyncSender<ImportEvent>,
    ) {
        let (effect_tx, effect_rx) = mpsc::sync_channel(8);
        let (stop_tx, stop_rx) = mpsc::sync_channel(1);
        let (event_tx, event_rx) = mpsc::sync_channel(EVENTS_PER_STAGE);
        let (automatic_effect_tx, automatic_effect_rx) = mpsc::sync_channel(8);
        let (automatic_stop_tx, automatic_stop_rx) = mpsc::sync_channel(1);
        let (automatic_event_tx, automatic_event_rx) = mpsc::sync_channel(EVENTS_PER_STAGE);
        let automatic_publication_gates = Arc::new(Mutex::new(HashMap::new()));
        (
            Self {
                effects: effect_tx,
                stops: stop_tx,
                events: event_rx,
                publication_gate,
                active_identity: None,
                unified_identities: HashSet::new(),
                automatic_effects: automatic_effect_tx,
                automatic_stops: automatic_stop_tx,
                automatic_events: automatic_event_rx,
                automatic_publication_gates,
                automatic_identities: HashSet::new(),
                automatic_active_identity: None,
                automatic_queue: VecDeque::new(),
                pending_automatic_failures: VecDeque::new(),
                poll_automatic_first: false,
            },
            effect_rx,
            stop_rx,
            event_tx,
            automatic_effect_rx,
            automatic_stop_rx,
            automatic_event_tx,
        )
    }

    fn start_next_automatic(&mut self) {
        if self.automatic_active_identity.is_some() {
            return;
        }
        let Some(automatic) = self.automatic_queue.pop_front() else {
            return;
        };
        let identity = (automatic.request.job_id, automatic.request.generation.get());
        let generation = automatic.request.generation;
        let effect = ImportEffect::StartAutomaticFinalTranscription(automatic);
        match self.automatic_effects.try_send(effect) {
            Ok(()) => self.automatic_active_identity = Some(identity),
            Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {
                self.automatic_publication_gates
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .remove(&identity);
                self.automatic_identities.remove(&identity);
                self.pending_automatic_failures
                    .push_back(ImportEvent::Failed {
                        job_id: identity.0,
                        generation,
                        message: "AutomaticFinalUnavailable: file worker is unavailable".into(),
                    });
            }
        }
    }
}

fn import_effect_identity(effect: &ImportEffect) -> Option<(JobId, u64)> {
    let identity = match effect {
        ImportEffect::SetGpuCandidate {
            job_id, generation, ..
        }
        | ImportEffect::ReleaseSourceBinding {
            job_id, generation, ..
        }
        | ImportEffect::RemoveQueued {
            job_id, generation, ..
        }
        | ImportEffect::ResolveImportPublication {
            job_id, generation, ..
        }
        | ImportEffect::ReadUnifiedDocumentPage {
            job_id, generation, ..
        }
        | ImportEffect::ReadUnifiedAudioPartsPage {
            job_id, generation, ..
        }
        | ImportEffect::SetMicrophoneGain {
            job_id, generation, ..
        }
        | ImportEffect::StartMicrophoneTest {
            job_id, generation, ..
        }
        | ImportEffect::StopMicrophoneTest { job_id, generation }
        | ImportEffect::PersistSegment(WorkerSegment {
            job_id, generation, ..
        })
        | ImportEffect::Publish {
            job_id, generation, ..
        }
        | ImportEffect::StopUnifiedLive { job_id, generation }
        | ImportEffect::Stop { job_id, generation }
        | ImportEffect::FinishLiveOnCpu { job_id, generation } => (*job_id, *generation),
        ImportEffect::StartLive(request)
        | ImportEffect::StartDirectLive(request)
        | ImportEffect::StartUnifiedLive(request) => (request.job_id, request.generation),
        ImportEffect::Enqueue(request)
        | ImportEffect::ProcessQueued(request)
        | ImportEffect::Prepare(request)
        | ImportEffect::StartWorker(request) => (request.job_id, request.generation),
        ImportEffect::ResolveLivePublication(request) => (request.job_id, request.generation),
        ImportEffect::StartAutomaticFinalTranscription(automatic) => {
            (automatic.request.job_id, automatic.request.generation)
        }
        ImportEffect::ResumeInterrupted { request, .. } => (request.job_id, request.generation),
        ImportEffect::ConfirmDuplicateImport { prompt, .. } => {
            (prompt.request.job_id, prompt.request.generation)
        }
        ImportEffect::InspectSourceSnapshotRecovery
        | ImportEffect::ConfirmSourceSnapshotRecovery
        | ImportEffect::ScanQueue { .. }
        | ImportEffect::ScanHistory { .. }
        | ImportEffect::ResumeReplacementCleanup { .. } => return None,
    };
    Some((identity.0, identity.1.get()))
}

fn terminal_import_event_identity(event: &ImportEvent) -> Option<(JobId, u64)> {
    let (job_id, generation) = match event {
        ImportEvent::Published {
            job_id, generation, ..
        }
        | ImportEvent::Failed {
            job_id, generation, ..
        }
        | ImportEvent::Stopped {
            job_id, generation, ..
        } => (*job_id, *generation),
        ImportEvent::MicrophoneTestStopped { job_id, generation } => (*job_id, *generation),
        _ => return None,
    };
    Some((job_id, generation.get()))
}

impl ImportIoPort for AsyncImportIo {
    fn submit(&mut self, effect: ImportEffect) -> Result<(), PortError> {
        let encoded = serialize_frame(&effect).map_err(|_| PortError::InvalidInput)?;
        if encoded.len().saturating_add(1) > MAX_FRAME_BYTES {
            return Err(PortError::InvalidInput);
        }
        #[cfg(feature = "l01-memory-qualification")]
        memory_trace(
            "adapter.effect.serialized",
            serde_json::json!({
                "effect_json_len_bytes": encoded.len(),
                "effect_json_capacity_bytes": encoded.capacity(),
                "effect_frame_len_bytes": encoded.len() + 1,
                "frame_limit_bytes": MAX_FRAME_BYTES,
            }),
        );
        if let ImportEffect::StopMicrophoneTest { job_id, generation } = &effect {
            if self.active_identity != Some((*job_id, *generation)) {
                return Err(PortError::StaleResponse);
            }
            return self.stops.try_send(effect).map_err(|error| match error {
                TrySendError::Full(_) | TrySendError::Disconnected(_) => PortError::Unavailable,
            });
        }
        if let ImportEffect::StartUnifiedLive(request) = &effect {
            let identity = (request.job_id, request.generation.get());
            if !self.unified_identities.insert(identity) {
                return Err(PortError::StaleResponse);
            }
            return match self.effects.try_send(effect) {
                Ok(()) => Ok(()),
                Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => {
                    self.unified_identities.remove(&identity);
                    Err(PortError::Unavailable)
                }
            };
        }
        if let ImportEffect::StopUnifiedLive { job_id, generation } = &effect {
            let identity = (*job_id, generation.get());
            if !self.unified_identities.contains(&identity) {
                return Err(PortError::StaleResponse);
            }
            return self.stops.try_send(effect).map_err(|error| match error {
                TrySendError::Full(_) | TrySendError::Disconnected(_) => PortError::Unavailable,
            });
        }
        if let ImportEffect::Stop { job_id, generation } = &effect {
            let identity = (*job_id, generation.get());
            if self.automatic_identities.contains(&identity) {
                crate::archive::l04_campaign_gate(
                    "stop.before_admit",
                    job_id.0,
                    generation.get(),
                    0,
                )
                .map_err(PortError::Failed)?;
                let gate = self
                    .automatic_publication_gates
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .get(&identity)
                    .cloned()
                    .ok_or(PortError::StaleResponse)?;
                if !gate.request_stop()? {
                    return Ok(());
                }
                if self.automatic_active_identity != Some(identity) {
                    return Ok(());
                }
                return match self.automatic_stops.try_send(effect) {
                    Ok(()) => Ok(()),
                    Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {
                        gate.rollback_stop();
                        Err(PortError::Unavailable)
                    }
                };
            }
            if self.active_identity != Some((*job_id, *generation)) {
                return Err(PortError::StaleResponse);
            }
            crate::archive::l04_campaign_gate("stop.before_admit", job_id.0, generation.get(), 0)
                .map_err(PortError::Failed)?;
            if !self.publication_gate.request_stop()? {
                return Ok(());
            }
            match self.stops.try_send(effect) {
                Ok(()) => Ok(()),
                Err(TrySendError::Full(_)) | Err(TrySendError::Disconnected(_)) => {
                    self.publication_gate.rollback_stop();
                    Err(PortError::Unavailable)
                }
            }
        } else {
            if matches!(&effect, ImportEffect::StartAutomaticFinalTranscription(_)) {
                let identity = import_effect_identity(&effect).expect("automatic import identity");
                let gate = Arc::new(PublicationGate::default());
                gate.activate();
                {
                    let mut gates = self
                        .automatic_publication_gates
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                    if gates.contains_key(&identity) {
                        return Err(PortError::StaleResponse);
                    }
                    gates.insert(identity, gate);
                }
                self.automatic_identities.insert(identity);
                if self.automatic_active_identity.is_some() {
                    if self.automatic_queue.len() >= 8 {
                        self.automatic_publication_gates
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner)
                            .remove(&identity);
                        self.automatic_identities.remove(&identity);
                        return Err(PortError::Unavailable);
                    }
                    let ImportEffect::StartAutomaticFinalTranscription(automatic) = effect else {
                        unreachable!("automatic lane start effect checked above")
                    };
                    self.automatic_queue.push_back(automatic);
                    return Ok(());
                }
                if let Err(error) = self.automatic_effects.try_send(effect) {
                    self.automatic_publication_gates
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .remove(&identity);
                    self.automatic_identities.remove(&identity);
                    return Err(match error {
                        TrySendError::Full(_) | TrySendError::Disconnected(_) => {
                            PortError::Unavailable
                        }
                    });
                }
                self.automatic_active_identity = Some(identity);
                return Ok(());
            }
            if let Some(identity) = import_effect_identity(&effect)
                && self.automatic_identities.contains(&identity)
            {
                if self.automatic_active_identity == Some(identity) {
                    return self
                        .automatic_effects
                        .try_send(effect)
                        .map_err(|error| match error {
                            TrySendError::Full(_) | TrySendError::Disconnected(_) => {
                                PortError::Unavailable
                            }
                        });
                }
                return Err(PortError::Backpressure);
            }
            if let ImportEffect::FinishLiveOnCpu { job_id, generation } = &effect
                && self.active_identity != Some((*job_id, *generation))
            {
                return Err(PortError::StaleResponse);
            }
            let active_identity = match &effect {
                ImportEffect::StartMicrophoneTest {
                    job_id, generation, ..
                } => Some((*job_id, *generation)),
                ImportEffect::StartLive(request) | ImportEffect::StartDirectLive(request) => {
                    Some((request.job_id, request.generation))
                }
                ImportEffect::StartUnifiedLive(_) => None,
                ImportEffect::StartAutomaticFinalTranscription(automatic) => {
                    Some((automatic.request.job_id, automatic.request.generation))
                }
                ImportEffect::Prepare(request) | ImportEffect::ProcessQueued(request) => {
                    Some((request.job_id, request.generation))
                }
                ImportEffect::ResumeInterrupted { request, .. } => {
                    Some((request.job_id, request.generation))
                }
                _ => None,
            };
            if let Some(identity) = active_identity {
                self.effects.try_send(effect).map_err(|e| match e {
                    TrySendError::Full(_) | TrySendError::Disconnected(_) => PortError::Unavailable,
                })?;
                self.active_identity = Some(identity);
                self.publication_gate.activate();
                return Ok(());
            }
            let retry_on_full = matches!(&effect, ImportEffect::PersistSegment(_));
            self.effects.try_send(effect).map_err(|error| match error {
                TrySendError::Full(_) if retry_on_full => PortError::Backpressure,
                TrySendError::Full(_) | TrySendError::Disconnected(_) => PortError::Unavailable,
            })
        }
    }
    fn poll(&mut self) -> Result<Option<ImportEvent>, PortError> {
        if let Some(failure) = self.pending_automatic_failures.pop_front() {
            return Ok(Some(failure));
        }
        self.start_next_automatic();
        let (first, second) = if self.poll_automatic_first {
            (&self.automatic_events, &self.events)
        } else {
            (&self.events, &self.automatic_events)
        };
        let first_result = first.try_recv();
        let first_disconnected = matches!(&first_result, Err(TryRecvError::Disconnected));
        let (event, from_automatic) = match first_result {
            Ok(event) => (Some(event), self.poll_automatic_first),
            Err(TryRecvError::Empty | TryRecvError::Disconnected) => {
                let second_result = second.try_recv();
                match second_result {
                    Ok(event) => (Some(event), !self.poll_automatic_first),
                    Err(TryRecvError::Empty) => (None, false),
                    Err(TryRecvError::Disconnected) if first_disconnected => {
                        return Err(PortError::Unavailable);
                    }
                    Err(TryRecvError::Disconnected) => (None, false),
                }
            }
        };
        if let Some(event) = event.as_ref() {
            if let Some(identity) = terminal_import_event_identity(event)
                && self.unified_identities.remove(&identity)
            {
                // The unified session has reached a terminal publication/failure event.
            }
            if from_automatic && let Some(identity) = terminal_import_event_identity(event) {
                self.automatic_identities.remove(&identity);
                self.automatic_publication_gates
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .remove(&identity);
                if self.automatic_active_identity == Some(identity) {
                    self.automatic_active_identity = None;
                    self.start_next_automatic();
                }
            }
            self.poll_automatic_first = !from_automatic;
        }
        Ok(event)
    }
}

trait ServiceRuntime {
    fn apply_effect(&mut self, effect: ImportEffect, pending: &mut VecDeque<ImportEvent>);
    fn poll_event(&mut self) -> Option<Result<ImportEvent, (JobId, Generation, String)>>;
    fn pump_realtime(&mut self) -> Option<(JobId, Generation, String)> {
        None
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

fn attach_automatic_final_import(
    live: &crate::parakeet_live::ParakeetLive,
    event: ImportEvent,
) -> ImportEvent {
    let ImportEvent::ParakeetCaptureStopped {
        job_id: capture_job,
        generation,
        wav_path,
        automatic_final,
        ..
    } = &event
    else {
        return event;
    };
    if automatic_final.is_some() || wav_path.is_empty() || !Path::new(wav_path).is_file() {
        return event;
    }
    // Queue/archive inspection can read an arbitrarily large durable journal.
    // Keep it off the capture service: the automatic-import lane validates this
    // candidate before writing status or starting any import work.
    let import_job = next_automatic_final_job_id();
    let Ok(automatic) = live.automatic_final_import(wav_path, import_job) else {
        return event;
    };
    debug_assert_eq!(*capture_job, live.identity().0);
    match event {
        ImportEvent::ParakeetCaptureStopped {
            job_id,
            generation,
            session_id,
            wav_path,
            txt_path,
            srt_path,
            complete,
            final_segments,
            unresolved,
            diagnostic,
            ..
        } => ImportEvent::ParakeetCaptureStopped {
            job_id,
            generation,
            session_id,
            wav_path,
            txt_path,
            srt_path,
            complete,
            final_segments,
            unresolved,
            diagnostic,
            automatic_final: Some(automatic),
        },
        other => other,
    }
}

fn publish_automatic_final_exports(
    source_txt: &Path,
    source_srt: &Path,
    automatic: &whisper_core::ports::AutomaticFinalImport,
) -> Result<(), String> {
    let outputs = &automatic.outputs;
    let txt =
        fs::read(source_txt).map_err(|error| format!("Export TXT automatique absent : {error}"))?;
    let srt =
        fs::read(source_srt).map_err(|error| format!("Export SRT automatique absent : {error}"))?;
    let txt_path = Path::new(&outputs.txt_path);
    let srt_path = Path::new(&outputs.srt_path);
    if txt_path.parent() != srt_path.parent() {
        return Err("Les exports automatiques doivent partager leur dossier de destination".into());
    }
    if txt_path.exists() || srt_path.exists() {
        return Err("Une sortie automatique existe déjà; aucune sortie n'a été remplacée".into());
    }
    let (txt_stage, mut txt_file) = create_automatic_stage(txt_path)?;
    if let Err(error) = txt_file.write_all(&txt).and_then(|()| txt_file.sync_all()) {
        drop(txt_file);
        return Err(with_cleanup_error(
            format!("Écriture du TXT automatique impossible : {error}"),
            remove_automatic_stage(&txt_stage),
        ));
    }
    drop(txt_file);
    let (srt_stage, mut srt_file) = match create_automatic_stage(srt_path) {
        Ok(stage) => stage,
        Err(error) => {
            return Err(with_cleanup_error(
                format!("Création du SRT automatique impossible : {error}"),
                remove_automatic_stage(&txt_stage),
            ));
        }
    };
    if let Err(error) = srt_file.write_all(&srt).and_then(|()| srt_file.sync_all()) {
        drop(srt_file);
        let cleanup =
            remove_automatic_stage(&srt_stage).and_then(|()| remove_automatic_stage(&txt_stage));
        return Err(with_cleanup_error(
            format!("Écriture du SRT automatique impossible : {error}"),
            cleanup,
        ));
    }
    drop(srt_file);
    publish_automatic_staged_pair(&txt_stage, &srt_stage, txt_path, srt_path)?;
    if let Err(error) =
        set_automatic_final_manifest_status(automatic, "produced_unverified", None, None)
    {
        return Err(with_cleanup_error(
            format!("Mise à jour du statut compagnon impossible : {error}"),
            remove_automatic_paths(&[txt_path, srt_path]),
        ));
    }
    Ok(())
}

fn set_automatic_final_manifest_status(
    automatic: &whisper_core::ports::AutomaticFinalImport,
    status: &str,
    effective_backend: Option<&str>,
    diagnostic: Option<&str>,
) -> Result<(), String> {
    let path = Path::new(&automatic.outputs.companion_json_path);
    let bytes = fs::read(path)
        .map_err(|error| format!("Lecture du JSON compagnon impossible : {error}"))?;
    let mut document: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("JSON compagnon invalide : {error}"))?;
    let object = document
        .as_object_mut()
        .ok_or_else(|| "La racine du JSON compagnon doit être un objet".to_owned())?;
    if !object
        .get("confirmed_intervals")
        .is_some_and(serde_json::Value::is_array)
        || !object
            .get("unresolved_intervals")
            .is_some_and(serde_json::Value::is_array)
    {
        return Err("Le JSON compagnon ne contient pas les intervalles directs attendus".into());
    }
    let model_sha256 = automatic
        .request
        .model_sha256
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    object.insert(
        "automatic_final_transcription".into(),
        serde_json::json!({
            "status": status,
            "txt_path": automatic.outputs.txt_path,
            "srt_path": automatic.outputs.srt_path,
            "model_sha256": model_sha256,
            "requested_compute": format!("{:?}", automatic.request.config.compute).to_ascii_lowercase(),
            "effective_backend": effective_backend,
            "diagnostic": diagnostic,
        }),
    );
    let replacement = serde_json::to_vec_pretty(&document)
        .map_err(|error| format!("Sérialisation du JSON compagnon impossible : {error}"))?;
    let (temporary, mut file) = create_automatic_stage(path)
        .map_err(|error| format!("Réservation du JSON compagnon impossible : {error}"))?;
    if let Err(error) = file
        .write_all(&replacement)
        .and_then(|()| file.write_all(b"\n"))
        .and_then(|()| file.sync_all())
    {
        drop(file);
        return Err(with_cleanup_error(
            format!("Écriture du JSON compagnon impossible : {error}"),
            remove_automatic_stage(&temporary),
        ));
    }
    drop(file);
    fs::rename(&temporary, path).map_err(|error| {
        with_cleanup_error(
            format!("Remplacement du JSON compagnon impossible : {error}"),
            remove_automatic_stage(&temporary),
        )
    })
}

fn create_automatic_stage(final_path: &Path) -> Result<(PathBuf, fs::File), String> {
    let parent = final_path
        .parent()
        .ok_or_else(|| "Sortie automatique sans dossier parent".to_owned())?;
    let name = final_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "Nom de sortie automatique invalide".to_owned())?;
    for _ in 0..8 {
        let nonce = NEXT_AUTO_EXPORT_STAGE.fetch_add(1, Ordering::Relaxed);
        let path = parent.join(format!(".{name}.{}.{}.stage", std::process::id(), nonce));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.to_string()),
        }
    }
    Err("Impossible de réserver un nom temporaire unique pour la sortie automatique".into())
}

fn remove_automatic_stage(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!(
            "suppression temporaire de {} impossible: {error}",
            path.display()
        )),
    }
}

fn remove_automatic_paths(paths: &[&Path]) -> Result<(), String> {
    let errors = paths
        .iter()
        .filter_map(|path| {
            fs::remove_file(path)
                .err()
                .filter(|error| error.kind() != io::ErrorKind::NotFound)
                .map(|error| format!("{}: {error}", path.display()))
        })
        .collect::<Vec<_>>();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("; "))
    }
}

fn with_cleanup_error(primary: String, cleanup: Result<(), String>) -> String {
    match cleanup {
        Ok(()) => primary,
        Err(cleanup) => format!("{primary}; nettoyage incomplet: {cleanup}"),
    }
}

fn publish_automatic_staged_pair(
    txt_stage: &Path,
    srt_stage: &Path,
    txt_path: &Path,
    srt_path: &Path,
) -> Result<(), String> {
    if txt_path.exists() || srt_path.exists() {
        return Err(with_cleanup_error(
            "Une sortie automatique existe déjà; aucune sortie n'a été remplacée".into(),
            remove_automatic_stage(txt_stage).and_then(|()| remove_automatic_stage(srt_stage)),
        ));
    }
    fs::hard_link(txt_stage, txt_path).map_err(|error| {
        with_cleanup_error(
            format!("Publication du TXT automatique impossible : {error}"),
            remove_automatic_stage(txt_stage).and_then(|()| remove_automatic_stage(srt_stage)),
        )
    })?;
    if let Err(error) = fs::hard_link(srt_stage, srt_path) {
        let cleanup = remove_automatic_paths(&[txt_path, txt_stage, srt_stage]);
        return Err(with_cleanup_error(
            format!("Publication du SRT automatique impossible : {error}"),
            cleanup,
        ));
    }
    if let Err(error) =
        remove_automatic_stage(txt_stage).and_then(|()| remove_automatic_stage(srt_stage))
    {
        let rollback = remove_automatic_paths(&[txt_path, srt_path, txt_stage, srt_stage]);
        return Err(with_cleanup_error(
            format!("Nettoyage des exports temporaires impossible : {error}"),
            rollback,
        ));
    }
    Ok(())
}

fn recovery_stop_counters(events: VecDeque<ImportEvent>) -> Option<ImportEvent> {
    events
        .into_iter()
        .find(|event| matches!(event, ImportEvent::LiveCounters { .. }))
}

fn effect_event_reservation(effect: &ImportEffect) -> usize {
    match effect {
        ImportEffect::ResumeInterrupted { .. } => 3,
        ImportEffect::ProcessQueued(_) | ImportEffect::Publish { .. } => 2,
        ImportEffect::Stop { .. } | ImportEffect::StopUnifiedLive { .. } => 2,
        _ => 1,
    }
}

fn service_loop<R: ServiceRuntime>(
    effect_rx: Receiver<ImportEffect>,
    stop_rx: Receiver<ImportEffect>,
    event_tx: SyncSender<ImportEvent>,
    mut runtime: R,
) {
    let mut pending = VecDeque::new();
    let mut deferred_effect = None;
    let mut deferred_stop = None;
    let mut deferred_realtime_failure = None;
    let mut effect_channel_disconnected = false;
    let mut stop_channel_disconnected = false;
    #[cfg(feature = "l01-memory-qualification")]
    memory_trace(
        "adapter.pending.created",
        serde_json::json!({
            "pending_len": pending.len(),
            "pending_capacity": pending.capacity(),
            "pending_slot_bytes": std::mem::size_of::<ImportEvent>(),
            "pending_limit": EVENTS_PER_STAGE,
            "event_channel_capacity": EVENTS_PER_STAGE,
        }),
    );
    loop {
        let mut stop = deferred_stop.take().or_else(|| match stop_rx.try_recv() {
            Ok(stop) => Some(stop),
            Err(TryRecvError::Disconnected) => {
                stop_channel_disconnected = true;
                None
            }
            Err(TryRecvError::Empty) => None,
        });
        if stop.is_some() && pending.len() + 2 > EVENTS_PER_STAGE {
            deferred_stop = stop.take();
        }
        let mut effect = deferred_effect
            .take()
            .or_else(|| match effect_rx.try_recv() {
                Ok(effect) => Some(effect),
                Err(TryRecvError::Disconnected) => {
                    effect_channel_disconnected = true;
                    None
                }
                Err(TryRecvError::Empty) => None,
            });
        if effect_channel_disconnected
            && stop_channel_disconnected
            && pending.is_empty()
            && deferred_effect.is_none()
            && deferred_stop.is_none()
            && deferred_realtime_failure.is_none()
        {
            return;
        }
        if deferred_stop.is_some()
            || effect.as_ref().is_some_and(|effect| {
                pending.len()
                    + effect_event_reservation(effect)
                    + if stop.is_some() { 2 } else { 0 }
                    > EVENTS_PER_STAGE
            })
        {
            deferred_effect = effect.take();
        }
        match (effect, stop) {
            (
                Some(
                    effect @ (ImportEffect::StartLive(_)
                    | ImportEffect::StartUnifiedLive(_)
                    | ImportEffect::StartDirectLive(_)
                    | ImportEffect::StartMicrophoneTest { .. }),
                ),
                Some(stop),
            ) => {
                runtime.apply_effect(effect, &mut pending);
                runtime.apply_effect(stop, &mut pending);
            }
            (Some(effect), Some(stop)) => {
                runtime.apply_effect(stop, &mut pending);
                deferred_effect = Some(effect);
            }
            (Some(effect), None) => runtime.apply_effect(effect, &mut pending),
            (None, Some(stop)) => runtime.apply_effect(stop, &mut pending),
            (None, None) => {}
        }
        if let Some((job_id, generation, message)) = runtime.pump_realtime() {
            deferred_realtime_failure = Some(ImportEvent::Failed {
                job_id,
                generation,
                message,
            });
        }
        if pending.len() < EVENTS_PER_STAGE
            && let Some(failure) = deferred_realtime_failure.take()
        {
            pending.push_back(failure);
        }
        if pending.len() < EVENTS_PER_STAGE {
            #[cfg(feature = "l01-memory-qualification")]
            memory_trace(
                "adapter.pending.after_effect",
                serde_json::json!({
                    "pending_len": pending.len(),
                    "pending_capacity": pending.capacity(),
                    "pending_slot_bytes": std::mem::size_of::<ImportEvent>(),
                }),
            );
        }
        if pending.len() < EVENTS_PER_STAGE
            && let Some(result) = runtime.poll_event()
        {
            pending.push_back(match result {
                Ok(event) => event,
                Err((job_id, generation, message)) => ImportEvent::Failed {
                    job_id,
                    generation,
                    message,
                },
            });
            #[cfg(feature = "l01-memory-qualification")]
            memory_trace(
                "adapter.pending.after_worker_event",
                serde_json::json!({
                    "pending_len": pending.len(),
                    "pending_capacity": pending.capacity(),
                    "pending_slot_bytes": std::mem::size_of::<ImportEvent>(),
                }),
            );
        }
        if let Some(event) = pending.front() {
            match event_tx.try_send(event.clone()) {
                Ok(()) => {
                    pending.pop_front();
                    #[cfg(feature = "l01-memory-qualification")]
                    memory_trace(
                        "adapter.pending.after_app_send",
                        serde_json::json!({
                            "pending_len": pending.len(),
                            "pending_capacity": pending.capacity(),
                            "pending_slot_bytes": std::mem::size_of::<ImportEvent>(),
                        }),
                    );
                }
                Err(TrySendError::Full(_)) => {}
                Err(TrySendError::Disconnected(_)) => return,
            }
        }
        thread::sleep(Duration::from_millis(5));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::io::Cursor;
    use std::{cell::RefCell, rc::Rc};
    use whisper_core::{
        AppCommand, Application, ComputeChoice, Generation, ImportApplication, ImportRequest,
        JobConfig, JobId, JobState, LanguageChoice,
    };

    fn controlled_child_job_bound(
        observations: &[ControlledChildObservation],
    ) -> Result<(), String> {
        if let Some(observation) = observations
            .iter()
            .find(|observation| observation.retained_job_ids > UNIFIED_CHILD_JOB_CAPACITY)
        {
            Err(format!(
                "child incarnation {} retained {} JobIds",
                observation.incarnation, observation.retained_job_ids
            ))
        } else {
            Ok(())
        }
    }

    struct RuntimeImportPort {
        runtime: ImportRuntime,
        events: VecDeque<ImportEvent>,
        capture_progress: Option<(JobId, Arc<AtomicU64>)>,
        document_revision: Option<(JobId, Arc<AtomicU64>)>,
        unified_stop_submissions: Option<Arc<AtomicU64>>,
        controlled_capture_blocks: Option<Arc<Mutex<VecDeque<(JobId, Generation, Vec<f32>)>>>>,
    }

    impl ImportIoPort for RuntimeImportPort {
        fn submit(&mut self, effect: ImportEffect) -> Result<(), PortError> {
            if matches!(&effect, ImportEffect::StopUnifiedLive { .. })
                && let Some(submissions) = &self.unified_stop_submissions
            {
                submissions.fetch_add(1, Ordering::Relaxed);
            }
            let starts_generation = matches!(
                &effect,
                ImportEffect::StartMicrophoneTest { .. }
                    | ImportEffect::StartLive(_)
                    | ImportEffect::StartUnifiedLive(_)
                    | ImportEffect::StartDirectLive(_)
                    | ImportEffect::Prepare(_)
                    | ImportEffect::ProcessQueued(_)
                    | ImportEffect::ResumeInterrupted { .. }
            );
            if starts_generation {
                self.runtime.publication_gate.activate();
            }
            self.runtime.effect(effect, &mut self.events);
            Ok(())
        }

        fn poll(&mut self) -> Result<Option<ImportEvent>, PortError> {
            if let Some(blocks) = &self.controlled_capture_blocks {
                let mut blocks = blocks.lock().expect("controlled capture block queue");
                while let Some((job_id, generation, samples)) = blocks.pop_front() {
                    let session =
                        whisper_core::parakeet_scheduler::SchedulerSession { job_id, generation };
                    if let Some(state) = self.runtime.unified_sessions.get_mut(&session) {
                        state.source.enqueue_test_block(samples);
                    } else {
                        blocks.push_front((job_id, generation, samples));
                        break;
                    }
                }
            }
            if let Some(event) = self.events.pop_front() {
                return Ok(Some(event));
            }
            if let Some((job_id, progress)) = &self.capture_progress {
                if let Some(samples) =
                    self.runtime
                        .unified_sessions
                        .iter()
                        .find_map(|(session, state)| {
                            (session.job_id == *job_id && state.source.is_capture())
                                .then_some(state.available_samples)
                        })
                {
                    progress.fetch_max(samples, Ordering::Relaxed);
                }
            }
            if let Some((job_id, revision)) = &self.document_revision {
                if let Some(current) =
                    self.runtime
                        .unified_sessions
                        .iter()
                        .find_map(|(session, state)| {
                            (session.job_id == *job_id).then_some(state.document.revision())
                        })
                {
                    revision.fetch_max(current, Ordering::Relaxed);
                }
            }
            match self.runtime.poll() {
                Some(Ok(event)) => Ok(Some(event)),
                Some(Err((job_id, generation, message))) => Ok(Some(ImportEvent::Failed {
                    job_id,
                    generation,
                    message,
                })),
                None => Ok(None),
            }
        }
    }

    struct ControlledUnifiedWorker {
        calls: Option<Arc<AtomicU64>>,
    }

    impl crate::parakeet_pipeline::WindowTranscriber for ControlledUnifiedWorker {
        fn instance_id(&self) -> u64 {
            9001
        }

        fn transcribe(
            &mut self,
            window: whisper_core::parakeet_session::AudioWindow,
            _: &[i16],
        ) -> Result<Vec<whisper_core::parakeet_ipc::Word>, String> {
            if let Some(calls) = &self.calls {
                calls.fetch_add(1, Ordering::Relaxed);
            }
            let start_ms = ((window.central_start_sample - window.native_start_sample) / 16) as u32;
            Ok(vec![whisper_core::parakeet_ipc::Word {
                text: "controlled.".into(),
                start_ms,
                end_ms: start_ms + 100,
            }])
        }
    }

    fn controlled_worker() -> (
        Box<dyn crate::parakeet_pipeline::WindowTranscriber>,
        Arc<AtomicU64>,
    ) {
        let calls = Arc::new(AtomicU64::new(0));
        (
            Box::new(ControlledUnifiedWorker {
                calls: Some(Arc::clone(&calls)),
            }),
            calls,
        )
    }

    struct HeldUnifiedWorker {
        calls: Arc<AtomicU64>,
        active: Arc<AtomicU64>,
        maximum_active: Arc<AtomicU64>,
        call_sources: Arc<std::sync::Mutex<Vec<String>>>,
        hold_call: u64,
        hold_first_final_a: bool,
        held: std::sync::mpsc::SyncSender<String>,
        release: std::sync::mpsc::Receiver<()>,
    }

    impl crate::parakeet_pipeline::WindowTranscriber for HeldUnifiedWorker {
        fn instance_id(&self) -> u64 {
            9010
        }

        fn transcribe(
            &mut self,
            window: whisper_core::parakeet_session::AudioWindow,
            pcm: &[i16],
        ) -> Result<Vec<whisper_core::parakeet_ipc::Word>, String> {
            let call = self.calls.fetch_add(1, Ordering::Relaxed) + 1;
            let active = self.active.fetch_add(1, Ordering::Relaxed) + 1;
            self.maximum_active.fetch_max(active, Ordering::Relaxed);
            let source_name = if pcm.first().is_some_and(|sample| sample.abs() < 6_500) {
                "B"
            } else {
                "A"
            };
            let is_final_window = window.central_end_sample - window.central_start_sample
                > 4 * 16_000
                || (source_name == "A" && window.central_start_sample >= 120 * 16_000);
            self.call_sources
                .lock()
                .expect("call-source log mutex")
                .push(format!(
                    "{source_name}:{}",
                    if is_final_window { "final" } else { "preview" }
                ));
            if (self.hold_first_final_a && source_name == "A" && is_final_window)
                || (!self.hold_first_final_a && call == self.hold_call)
            {
                self.hold_first_final_a = false;
                let _ = self.held.send(source_name.to_owned());
                self.release
                    .recv()
                    .map_err(|error| format!("controlled release channel: {error}"))?;
            }
            self.active.fetch_sub(1, Ordering::Relaxed);
            let start_ms = ((window.central_start_sample - window.native_start_sample) / 16) as u32;
            Ok(vec![whisper_core::parakeet_ipc::Word {
                text: format!("{source_name}-final-{call}."),
                start_ms,
                end_ms: start_ms + 100,
            }])
        }
    }

    fn held_worker(
        hold_call: u64,
    ) -> (
        Box<dyn crate::parakeet_pipeline::WindowTranscriber>,
        Arc<AtomicU64>,
        Arc<AtomicU64>,
        Arc<std::sync::Mutex<Vec<String>>>,
        std::sync::mpsc::Receiver<String>,
        std::sync::mpsc::SyncSender<()>,
    ) {
        held_worker_with_mode(hold_call, false)
    }

    fn held_final_a_worker() -> (
        Box<dyn crate::parakeet_pipeline::WindowTranscriber>,
        Arc<AtomicU64>,
        Arc<AtomicU64>,
        Arc<std::sync::Mutex<Vec<String>>>,
        std::sync::mpsc::Receiver<String>,
        std::sync::mpsc::SyncSender<()>,
    ) {
        held_worker_with_mode(0, true)
    }

    fn held_worker_with_mode(
        hold_call: u64,
        hold_first_final_a: bool,
    ) -> (
        Box<dyn crate::parakeet_pipeline::WindowTranscriber>,
        Arc<AtomicU64>,
        Arc<AtomicU64>,
        Arc<std::sync::Mutex<Vec<String>>>,
        std::sync::mpsc::Receiver<String>,
        std::sync::mpsc::SyncSender<()>,
    ) {
        let calls = Arc::new(AtomicU64::new(0));
        let active = Arc::new(AtomicU64::new(0));
        let maximum_active = Arc::new(AtomicU64::new(0));
        let call_sources = Arc::new(std::sync::Mutex::new(Vec::new()));
        let (held, held_rx) = std::sync::mpsc::sync_channel(1);
        let (release_tx, release) = std::sync::mpsc::sync_channel(1);
        (
            Box::new(HeldUnifiedWorker {
                calls: Arc::clone(&calls),
                active: Arc::clone(&active),
                maximum_active: Arc::clone(&maximum_active),
                call_sources: Arc::clone(&call_sources),
                hold_call,
                hold_first_final_a,
                held,
                release,
            }),
            calls,
            maximum_active,
            call_sources,
            held_rx,
            release_tx,
        )
    }

    struct TailAwareUnifiedWorker;

    impl crate::parakeet_pipeline::WindowTranscriber for TailAwareUnifiedWorker {
        fn instance_id(&self) -> u64 {
            9011
        }

        fn transcribe(
            &mut self,
            window: whisper_core::parakeet_session::AudioWindow,
            _: &[i16],
        ) -> Result<Vec<whisper_core::parakeet_ipc::Word>, String> {
            let terminal = window.central_end_sample > 120 * 16_000;
            let start_ms = ((window.central_start_sample - window.native_start_sample) / 16) as u32;
            Ok(vec![whisper_core::parakeet_ipc::Word {
                text: if terminal {
                    "terminal-phrase."
                } else {
                    "intermediate-phrase."
                }
                .into(),
                start_ms,
                end_ms: start_ms + 100,
            }])
        }
    }

    struct ObservedTailAwareUnifiedWorker {
        windows: Arc<Mutex<Vec<(u64, u64, usize)>>>,
    }

    impl crate::parakeet_pipeline::WindowTranscriber for ObservedTailAwareUnifiedWorker {
        fn instance_id(&self) -> u64 {
            TailAwareUnifiedWorker.instance_id()
        }

        fn transcribe(
            &mut self,
            window: whisper_core::parakeet_session::AudioWindow,
            pcm: &[i16],
        ) -> Result<Vec<whisper_core::parakeet_ipc::Word>, String> {
            self.windows
                .lock()
                .expect("observed unified worker windows")
                .push((
                    window.native_start_sample,
                    window.native_end_sample,
                    pcm.len(),
                ));
            let mut inner = TailAwareUnifiedWorker;
            inner.transcribe(window, pcm)
        }
    }

    struct InvalidTimeUnifiedWorker;

    impl crate::parakeet_pipeline::WindowTranscriber for InvalidTimeUnifiedWorker {
        fn instance_id(&self) -> u64 {
            9002
        }
        fn transcribe(
            &mut self,
            _: whisper_core::parakeet_session::AudioWindow,
            _: &[i16],
        ) -> Result<Vec<whisper_core::parakeet_ipc::Word>, String> {
            Ok(vec![whisper_core::parakeet_ipc::Word {
                text: "invalid".into(),
                start_ms: 50,
                end_ms: 20,
            }])
        }
    }

    struct FailedUnifiedWorker;

    impl crate::parakeet_pipeline::WindowTranscriber for FailedUnifiedWorker {
        fn instance_id(&self) -> u64 {
            9003
        }
        fn transcribe(
            &mut self,
            _: whisper_core::parakeet_session::AudioWindow,
            _: &[i16],
        ) -> Result<Vec<whisper_core::parakeet_ipc::Word>, String> {
            Err("controlled worker failure".into())
        }
    }

    struct ExportCollisionWorker {
        path: PathBuf,
        transcribe_completed: Arc<AtomicU64>,
    }

    impl crate::parakeet_pipeline::WindowTranscriber for ExportCollisionWorker {
        fn instance_id(&self) -> u64 {
            9004
        }
        fn transcribe(
            &mut self,
            window: whisper_core::parakeet_session::AudioWindow,
            _: &[i16],
        ) -> Result<Vec<whisper_core::parakeet_ipc::Word>, String> {
            if let Some(parent) = self.path.parent() {
                std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            std::fs::write(&self.path, b"preexisting export").map_err(|error| error.to_string())?;
            self.transcribe_completed.store(1, Ordering::Release);
            let start_ms = ((window.central_start_sample - window.native_start_sample) / 16) as u32;
            Ok(vec![whisper_core::parakeet_ipc::Word {
                text: "export collision".into(),
                start_ms,
                end_ms: start_ms + 100,
            }])
        }
    }

    fn unified_import_request(root: &Path, source: &Path, id: u128) -> ImportRequest {
        let model = whisper_core::model_registry::model(
            whisper_core::model_catalog::ModelId::ParakeetTdt06BV3,
        );
        let model_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("models")
            .join(model.artifact.file_name);
        let model_sha256 = (0..32)
            .map(|index| {
                u8::from_str_radix(&model.artifact.sha256[index * 2..index * 2 + 2], 16).unwrap()
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        ImportRequest {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1,
            job_id: JobId(id),
            generation: Generation::first(),
            source_path: source.to_string_lossy().into_owned(),
            source_sha256: None,
            source_samples: None,
            model_path: model_path.to_string_lossy().into_owned(),
            model_sha256,
            destination: root.to_string_lossy().into_owned(),
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Cpu,
            },
        }
    }

    fn unified_live_request(root: &Path, id: u128) -> whisper_core::ports::LiveRequest {
        let model = whisper_core::model_registry::model(
            whisper_core::model_catalog::ModelId::ParakeetTdt06BV3,
        );
        let model_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("models")
            .join(model.artifact.file_name);
        let model_sha256 = (0..32)
            .map(|index| {
                u8::from_str_radix(&model.artifact.sha256[index * 2..index * 2 + 2], 16).unwrap()
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        whisper_core::ports::LiveRequest {
            job_id: JobId(id),
            generation: Generation::first(),
            model_path: model_path.to_string_lossy().into_owned(),
            model_sha256,
            destination: root.to_string_lossy().into_owned(),
            group_offset_samples: 0,
            auto_stop_after_speech_samples: None,
            microphone: whisper_core::MicrophoneChoice::Device(format!("test:unified-{id}")),
            microphone_gain_db: 0,
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Cpu,
            },
        }
    }

    fn assert_real_unified_model(request: &ImportRequest) {
        assert!(
            Path::new(&request.model_path).is_file(),
            "required real Parakeet test model is missing: {}",
            request.model_path
        );
    }

    #[test]
    fn import_application_uses_real_archive_path_for_controlled_short_and_long_unified_imports() {
        for (index, (seconds, compute)) in [
            (3_u64, ComputeChoice::Cpu),
            (3_u64, ComputeChoice::Gpu),
            (121_u64, ComputeChoice::Cpu),
            (121_u64, ComputeChoice::Gpu),
        ]
        .into_iter()
        .enumerate()
        {
            let root = std::env::temp_dir().join(format!(
                "parakeet-unified-application-{}-{}",
                std::process::id(),
                index
            ));
            std::fs::create_dir_all(&root).unwrap();
            let source_path = root.join("source.wav");
            let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_path).unwrap();
            writer
                .append(&vec![0_i16; (seconds * 16_000) as usize])
                .unwrap();
            writer.finish().unwrap();
            let mut request =
                unified_import_request(&root.join("archive"), &source_path, 700 + index as u128);
            request.config.compute = compute;
            assert_real_unified_model(&request);
            let (worker, calls) = controlled_worker();
            let mut runtime = ImportRuntime::new(
                WorkerPaths {
                    cpu: PathBuf::from("unused-cpu"),
                    gpu: PathBuf::from("unused-gpu"),
                },
                Arc::new(PublicationGate::default()),
            )
            .with_controlled_transcriber(worker);
            if compute == ComputeChoice::Gpu {
                let mut candidate = fake_gpu_candidate();
                candidate.generation = request.generation.get();
                runtime
                    .gpu_candidates
                    .insert((request.job_id, request.generation.get()), Some(candidate));
            }
            let port = RuntimeImportPort {
                runtime,
                events: VecDeque::new(),
                capture_progress: None,
                document_revision: None,
                unified_stop_submissions: None,
                controlled_capture_blocks: None,
            };
            let mut app = ImportApplication::new(port);
            app.dispatch(AppCommand::StartImport { request }).unwrap();
            for _ in 0..32 {
                let view = app.dispatch(AppCommand::Refresh).unwrap();
                if view.active_job.is_none() {
                    break;
                }
            }
            let entries = ArchiveStore::scan(&root.join("archive")).unwrap();
            let entry = entries
                .iter()
                .find(|entry| {
                    entry.complete && entry.strategy
                == whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
                })
                .unwrap();
            assert_eq!(
                ArchiveStore::published_imports_for_strategy(
                    &root.join("archive"),
                    &entry.source_sha256,
                    whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1,
                )
                .unwrap()
                .len(),
                1
            );
            assert!(
                ArchiveStore::published_imports_for_strategy(
                    &root.join("archive"),
                    &entry.source_sha256,
                    whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
                )
                .unwrap()
                .is_empty()
            );
            let archive = root.join(format!(
                "archive/transcriptions/{:032x}/1",
                700 + index as u128
            ));
            assert!(archive.join("transcript.txt").is_file());
            assert!(archive.join("transcript.srt").is_file());
            assert!(
                std::fs::read_to_string(archive.join("transcript.txt"))
                    .unwrap()
                    .contains("controlled.")
            );
            let coverage =
                std::fs::read_to_string(archive.join("parakeet-unified/document/coverage.jsonl"))
                    .unwrap();
            assert_eq!(coverage.lines().count(), if seconds > 120 { 2 } else { 1 });
            assert_eq!(
                calls.load(Ordering::Relaxed),
                coverage.lines().count() as u64,
                "each final window must be invoked through the scheduler"
            );
            let basename = entry.export_basename.as_ref().unwrap();
            assert!(
                root.join("archive/exports")
                    .join(format!("{basename}.txt"))
                    .is_file()
            );
            assert!(
                root.join("archive/exports")
                    .join(format!("{basename}.srt"))
                    .is_file()
            );
            std::fs::remove_dir_all(&root).unwrap();
        }
    }

    #[test]
    fn import_application_finalizes_stopped_capture_a_while_capture_b_pumps_and_publishes_segmented_audio()
     {
        const A_ID: u128 = 9_301;
        const B_ID: u128 = 9_302;
        const THIRD_ID: u128 = 9_303;
        const A_SAMPLES: usize = 121 * 16_000;
        const B_SAMPLES: usize = 35 * 16_000;
        const PART_LIMIT_BYTES: u64 = 32_000;
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-capture-ab-{}",
            std::process::id()
        ));
        let archive_root = root.join("archive");
        std::fs::create_dir_all(&archive_root).unwrap();

        let request_a = unified_live_request(&archive_root, A_ID);
        let request_b = unified_live_request(&archive_root, B_ID);
        let request_third = unified_live_request(&archive_root, THIRD_ID);
        assert!(Path::new(&request_a.model_path).is_file());

        let mut capture_a =
            crate::capture::CaptureStream::empty_test_capture(16_000, 1, "test:capture-a");
        for _ in 0..25 {
            capture_a.enqueue_test_block(vec![0.25; 77_440]);
        }
        let mut capture_b =
            crate::capture::CaptureStream::empty_test_capture(16_000, 1, "test:capture-b");
        for _ in 0..7 {
            capture_b.enqueue_test_block(vec![0.15; 5 * 16_000]);
        }
        let mut capture_third = crate::capture::CaptureStream::empty_test_capture(
            16_000,
            1,
            "test:capture-third-after-reap",
        );
        capture_third.enqueue_test_block(vec![0.1; 3 * 16_000]);

        let (worker, calls, maximum_active, call_sources, held_rx, release_tx) =
            held_final_a_worker();
        let b_progress = Arc::new(AtomicU64::new(0));
        let stop_submissions = Arc::new(AtomicU64::new(0));
        let runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(worker)
        .with_controlled_capture(capture_a, Some(PART_LIMIT_BYTES))
        .with_controlled_capture(capture_b, Some(PART_LIMIT_BYTES))
        .with_controlled_capture(capture_third, Some(PART_LIMIT_BYTES));
        let mut app = ImportApplication::new(RuntimeImportPort {
            runtime,
            events: VecDeque::new(),
            capture_progress: Some((JobId(B_ID), Arc::clone(&b_progress))),
            document_revision: None,
            unified_stop_submissions: Some(Arc::clone(&stop_submissions)),
            controlled_capture_blocks: None,
        });

        app.dispatch(AppCommand::StartUnifiedLiveConfigured {
            request: request_a.clone(),
        })
        .unwrap();
        for _ in 0..100 {
            app.dispatch(AppCommand::Refresh).unwrap();
            if calls.load(Ordering::Relaxed) >= 1 {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        assert!(
            calls.load(Ordering::Relaxed) >= 1,
            "capture A preview was not scheduled"
        );
        assert!(matches!(
            app.dispatch(AppCommand::StartUnifiedLiveConfigured {
                request: request_b.clone(),
            }),
            Err(whisper_core::application::ApplicationError::InvalidCommand)
        ));
        assert!(
            !archive_root
                .join("transcriptions")
                .join(format!("{:032x}", B_ID))
                .exists()
        );
        app.dispatch(AppCommand::StopUnifiedLive {
            job_id: request_a.job_id,
            generation: request_a.generation,
        })
        .unwrap();

        let held_deadline = Instant::now() + Duration::from_secs(5);
        let mut held_source = None;
        while Instant::now() < held_deadline {
            app.dispatch(AppCommand::Refresh).unwrap();
            match held_rx.try_recv() {
                Ok(source) => {
                    held_source = Some(source);
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    thread::sleep(Duration::from_millis(2));
                }
                Err(error) => panic!("capture A final worker exited before holding: {error}"),
            }
        }
        assert_eq!(
            held_source.as_deref(),
            Some("A"),
            "capture A final inference was not held"
        );
        let held_call_count = calls.load(Ordering::Relaxed);
        assert!(held_call_count >= 2);
        assert_eq!(maximum_active.load(Ordering::Relaxed), 1);

        app.dispatch(AppCommand::StartUnifiedLiveConfigured {
            request: request_b.clone(),
        })
        .unwrap();
        assert!(
            matches!(
                app.dispatch(AppCommand::StartUnifiedLiveConfigured {
                    request: request_third.clone(),
                }),
                Err(whisper_core::application::ApplicationError::InvalidCommand)
            ),
            "third session must be refused before the runtime opens its capture"
        );
        assert!(
            !root
                .join("archive/transcriptions")
                .join(format!("{:032x}", THIRD_ID))
                .exists()
        );

        let b_pump_deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < b_pump_deadline && b_progress.load(Ordering::Relaxed) == 0 {
            app.dispatch(AppCommand::Refresh).unwrap();
            thread::sleep(Duration::from_millis(2));
        }
        let b_written_before_stop = b_progress.load(Ordering::Relaxed);
        assert!(
            b_written_before_stop > 0,
            "B did not progress during A's held call"
        );
        assert!(
            b_written_before_stop < B_SAMPLES as u64,
            "the capture's delayed converter tail is acknowledged only at Stop"
        );
        assert_eq!(
            calls.load(Ordering::Relaxed),
            held_call_count,
            "B must pump without a second backend call"
        );
        assert_eq!(maximum_active.load(Ordering::Relaxed), 1);

        app.dispatch(AppCommand::StopUnifiedLive {
            job_id: request_b.job_id,
            generation: request_b.generation,
        })
        .unwrap();
        assert_eq!(stop_submissions.load(Ordering::Relaxed), 2);
        release_tx.send(()).unwrap();

        let publication_deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < publication_deadline {
            app.dispatch(AppCommand::Refresh).unwrap();
            let entries = ArchiveStore::scan(&archive_root).unwrap();
            let published = [JobId(A_ID), JobId(B_ID)].into_iter().all(|job_id| {
                entries
                    .iter()
                    .any(|entry| entry.job_id == job_id && entry.complete)
            });
            if published {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }

        let entries = ArchiveStore::scan(&archive_root).unwrap();
        let final_call_count = calls.load(Ordering::Relaxed);
        let entry_for = |job_id| entries.iter().find(|entry| entry.job_id == job_id);
        for (job_id, label) in [(JobId(A_ID), "A"), (JobId(B_ID), "B")] {
            let entry = entry_for(job_id).unwrap_or_else(|| {
                panic!(
                    "capture {label} has no archive entry; A={:?}, B={:?}, app_view={:?}",
                    entry_for(JobId(A_ID)),
                    entry_for(JobId(B_ID)),
                    app.view()
                )
            });
            assert!(
                entry.complete,
                "capture {label} was not published; A={:?}, B={:?}, app_view={:?}",
                entry_for(JobId(A_ID)),
                entry_for(JobId(B_ID)),
                app.view()
            );
        }
        let a_entry = entry_for(JobId(A_ID)).unwrap();
        let b_entry = entry_for(JobId(B_ID)).unwrap();
        let a_basename = a_entry.export_basename.as_deref().unwrap();
        let b_basename = b_entry.export_basename.as_deref().unwrap();
        assert_ne!(
            a_basename, b_basename,
            "simultaneous unpublished generations must hold distinct export reservations"
        );
        for (job_id, expected_samples, label) in [
            (JobId(A_ID), A_SAMPLES as u64, "A"),
            (JobId(B_ID), B_SAMPLES as u64, "B"),
        ] {
            let entry = entries
                .iter()
                .find(|entry| entry.job_id == job_id && entry.complete)
                .unwrap_or_else(|| {
                    panic!(
                        "capture {label} was not fully published; A={:?}, B={:?}, app_view={:?}",
                        entry_for(JobId(A_ID)),
                        entry_for(JobId(B_ID)),
                        app.view()
                    )
                });
            let generation = archive_root
                .join("transcriptions")
                .join(format!("{:032x}", job_id.0))
                .join("1");
            assert!(generation.join("CURRENT").is_file());
            assert_eq!(
                std::fs::read_to_string(generation.join("CURRENT")).unwrap(),
                "manifest.json\n"
            );
            assert!(generation.join("transcript.txt").is_file());
            assert!(generation.join("transcript.srt").is_file());
            let transcript = std::fs::read_to_string(generation.join("transcript.txt")).unwrap();
            if label == "A" {
                assert!(transcript.contains(&format!("A-final-{final_call_count}.")));
            } else {
                assert!(transcript.contains(&format!("B-final-{}.", final_call_count - 1)));
            }
            let basename = entry.export_basename.as_ref().unwrap();
            assert_eq!(
                std::fs::read(archive_root.join("exports").join(format!("{basename}.txt")))
                    .unwrap(),
                std::fs::read(generation.join("transcript.txt")).unwrap(),
                "capture {label} TXT export must belong to its published generation"
            );
            assert_eq!(
                std::fs::read(archive_root.join("exports").join(format!("{basename}.srt")))
                    .unwrap(),
                std::fs::read(generation.join("transcript.srt")).unwrap(),
                "capture {label} SRT export must belong to its published generation"
            );

            let manifest_path = generation.join("captured-source-parts.json");
            let manifest: serde_json::Value =
                serde_json::from_slice(&std::fs::read(manifest_path).unwrap()).unwrap();
            assert_eq!(manifest["source_samples"].as_u64(), Some(expected_samples));
            let parts = manifest["parts"].as_array().unwrap();
            assert!(
                parts.len() > 1,
                "capture {label} did not rotate into multiple parts"
            );
            let mut cursor = 0_u64;
            for part in parts {
                assert_eq!(part["start_sample"].as_u64(), Some(cursor));
                cursor = part["end_sample"].as_u64().unwrap();
                assert!(Path::new(part["path"].as_str().unwrap()).is_file());
            }
            assert_eq!(cursor, expected_samples);

            let source_path = generation.join("parakeet-unified/source.wav");
            let source = crate::parakeet_audio::PcmPartStore::open(source_path);
            let (block_count, block_samples, input_level) = if label == "B" {
                (7, 5 * 16_000, 0.15_f32)
            } else {
                (25, 77_440, 0.25_f32)
            };
            let mut reference_capture =
                crate::capture::CaptureStream::empty_test_capture(16_000, 1, "test:pcm-reference");
            for _ in 0..block_count {
                reference_capture.enqueue_test_block(vec![input_level; block_samples]);
            }
            let mut reference_converter = crate::capture::Pcm16Converter::new(16_000, 1).unwrap();
            let mut expected_pcm = Vec::with_capacity(expected_samples as usize);
            while let Some(block) = reference_capture.try_next_block() {
                expected_pcm.extend(reference_converter.push(&block).unwrap());
                reference_capture.recycle_block(block).unwrap();
            }
            expected_pcm.extend(reference_converter.finish().unwrap());
            assert_eq!(expected_pcm.len() as u64, expected_samples);
            let steady_level = (input_level * 32_767.0).round() as i16;
            assert!(
                expected_pcm[128..expected_pcm.len() - 128]
                    .iter()
                    .all(|sample| sample.abs_diff(steady_level) <= 2)
            );
            let mut start = 0_u64;
            while start < expected_samples {
                let end = start.saturating_add(80_000).min(expected_samples);
                let samples = source.read_window(start..end).unwrap();
                assert_eq!(samples.len() as u64, end - start);
                assert!(
                    samples
                        .iter()
                        .zip(&expected_pcm[start as usize..end as usize])
                        .all(|(actual, expected)| actual.abs_diff(*expected) <= 2),
                    "capture {label} PCM differs from the sequential converter reference in {start}..{end}"
                );
                let interior_start = start.max(128);
                let interior_end = end.min(expected_samples - 128);
                if interior_start < interior_end {
                    let local_start = (interior_start - start) as usize;
                    let local_end = (interior_end - start) as usize;
                    assert!(
                        samples[local_start..local_end]
                            .iter()
                            .all(|sample| sample.abs_diff(steady_level) <= 2)
                    );
                }
                start = end;
            }

            let coverage = std::fs::read_to_string(
                generation.join("parakeet-unified/document/coverage.jsonl"),
            )
            .unwrap();
            let windows = coverage
                .lines()
                .map(|line| {
                    serde_json::from_str::<whisper_core::ports::ParakeetWindowCoverage>(line)
                        .unwrap()
                })
                .collect::<Vec<_>>();
            assert!(!windows.is_empty());
            assert_eq!(windows.first().unwrap().range.start_sample, 0);
            assert_eq!(windows.last().unwrap().range.end_sample, expected_samples);
            assert!(
                windows
                    .windows(2)
                    .all(|pair| { pair[0].range.end_sample == pair[1].range.start_sample })
            );
        }
        assert_eq!(maximum_active.load(Ordering::Relaxed), 1);
        let call_sources = call_sources.lock().unwrap();
        let final_order = call_sources
            .iter()
            .rev()
            .take(3)
            .map(String::as_str)
            .collect::<Vec<_>>();
        assert_eq!(
            final_order.into_iter().rev().collect::<Vec<_>>(),
            ["A:final", "B:final", "A:final"],
            "final windows must alternate after A's held window and preserve its tail"
        );
        drop(call_sources);

        app.dispatch(AppCommand::StartUnifiedLiveConfigured {
            request: request_third.clone(),
        })
        .unwrap();
        app.dispatch(AppCommand::StopUnifiedLive {
            job_id: request_third.job_id,
            generation: request_third.generation,
        })
        .unwrap();
        let third_publication_deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < third_publication_deadline {
            app.dispatch(AppCommand::Refresh).unwrap();
            if ArchiveStore::scan(&archive_root)
                .unwrap()
                .iter()
                .any(|entry| entry.job_id == request_third.job_id && entry.complete)
            {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        assert!(
            ArchiveStore::scan(&archive_root)
                .unwrap()
                .iter()
                .any(|entry| entry.job_id == request_third.job_id && entry.complete)
        );
        assert_eq!(stop_submissions.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn global_close_waits_for_both_capture_publications_and_refuses_new_admission() {
        const A_ID: u128 = 9_321;
        const B_ID: u128 = 9_322;
        const THIRD_ID: u128 = 9_323;
        const PART_LIMIT_BYTES: u64 = 32_000;
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-global-close-{}",
            std::process::id()
        ));
        let archive_root = root.join("archive");
        std::fs::create_dir_all(&archive_root).unwrap();
        let request_a = unified_live_request(&archive_root, A_ID);
        let request_b = unified_live_request(&archive_root, B_ID);
        let request_third = unified_live_request(&archive_root, THIRD_ID);
        assert!(
            Path::new(&request_a.model_path).is_file(),
            "required real Parakeet test model is missing: {}",
            request_a.model_path
        );

        let mut capture_a =
            crate::capture::CaptureStream::empty_test_capture(16_000, 1, "test:close-a");
        capture_a.enqueue_test_block(vec![0.25; 5 * 16_000]);
        capture_a.enqueue_test_block(vec![0.25; 5 * 16_000]);
        let mut capture_b =
            crate::capture::CaptureStream::empty_test_capture(16_000, 1, "test:close-b");
        capture_b.enqueue_test_block(vec![0.15; 5 * 16_000]);
        capture_b.enqueue_test_block(vec![0.15; 5 * 16_000]);

        let (worker, calls, _, _, held_rx, release_tx) = held_final_a_worker();
        let stop_submissions = Arc::new(AtomicU64::new(0));
        let runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(worker)
        .with_controlled_capture(capture_a, Some(PART_LIMIT_BYTES))
        .with_controlled_capture(capture_b, Some(PART_LIMIT_BYTES));
        let mut app = ImportApplication::new(RuntimeImportPort {
            runtime,
            events: VecDeque::new(),
            capture_progress: None,
            document_revision: None,
            unified_stop_submissions: Some(Arc::clone(&stop_submissions)),
            controlled_capture_blocks: None,
        });

        app.dispatch(AppCommand::StartUnifiedLiveConfigured {
            request: request_a.clone(),
        })
        .unwrap();
        let a_preview_deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < a_preview_deadline && calls.load(Ordering::Relaxed) == 0 {
            app.dispatch(AppCommand::Refresh).unwrap();
            thread::sleep(Duration::from_millis(2));
        }
        assert!(
            calls.load(Ordering::Relaxed) > 0,
            "A did not begin processing"
        );
        app.dispatch(AppCommand::StopUnifiedLive {
            job_id: request_a.job_id,
            generation: request_a.generation,
        })
        .unwrap();
        let held_deadline = Instant::now() + Duration::from_secs(5);
        let mut held_source = None;
        while Instant::now() < held_deadline {
            app.dispatch(AppCommand::Refresh).unwrap();
            match held_rx.try_recv() {
                Ok(source) => {
                    held_source = Some(source);
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    thread::sleep(Duration::from_millis(2));
                }
                Err(error) => panic!("A final call exited before the close test held it: {error}"),
            }
        }
        assert_eq!(held_source.as_deref(), Some("A"));

        app.dispatch(AppCommand::StartUnifiedLiveConfigured {
            request: request_b.clone(),
        })
        .unwrap();
        let b_source_path = archive_root
            .join("transcriptions")
            .join(format!("{:032x}", request_b.job_id.0))
            .join(request_b.generation.get().to_string())
            .join("parakeet-unified/source.wav");
        let b_progress_deadline = Instant::now() + Duration::from_secs(5);
        let mut b_card_samples = 0;
        let mut b_written_samples = 0;
        while Instant::now() < b_progress_deadline {
            app.dispatch(AppCommand::Refresh).unwrap();
            b_card_samples = app
                .view()
                .unified_parakeet_sessions
                .iter()
                .find(|session| session.job_id == request_b.job_id)
                .map_or(0, |session| session.durable_samples);
            if b_source_path.is_file() {
                b_written_samples = crate::parakeet_audio::PcmPartStore::open(&b_source_path)
                    .total_samples()
                    .unwrap_or(0);
            }
            if b_card_samples > 0 && b_written_samples > 0 {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        assert!(
            b_written_samples > 0,
            "B's acknowledged audio write did not progress while A's final inference was held"
        );
        assert!(
            b_card_samples > 0,
            "B's acknowledged capture progress did not reach its session card while A was held"
        );
        assert!(
            b_card_samples <= b_written_samples,
            "B's session card progress exceeded the acknowledged audio write"
        );

        app.dispatch(AppCommand::RequestClose).unwrap();
        assert!(
            !app.view().close_ready,
            "close must await A and B publication"
        );
        assert!(matches!(
            app.dispatch(AppCommand::StartUnifiedLiveConfigured {
                request: request_third.clone(),
            }),
            Err(whisper_core::application::ApplicationError::InvalidCommand)
        ));
        assert!(
            !archive_root
                .join("transcriptions")
                .join(format!("{:032x}", THIRD_ID))
                .exists(),
            "the third admission must be refused before opening capture or archive state"
        );
        app.dispatch(AppCommand::RequestClose).unwrap();
        assert_eq!(stop_submissions.load(Ordering::Relaxed), 2);
        let stopped_progress_deadline = Instant::now() + Duration::from_secs(5);
        let mut both_finalizing = false;
        while Instant::now() < stopped_progress_deadline {
            app.dispatch(AppCommand::Refresh).unwrap();
            both_finalizing = [request_a.job_id, request_b.job_id]
                .into_iter()
                .all(|job_id| {
                    app.view()
                        .unified_parakeet_sessions
                        .iter()
                        .find(|session| session.job_id == job_id)
                        .is_some_and(|session| {
                            session.total_samples.is_some_and(|total| {
                                session.captured_samples == total
                                    && session.durable_samples == total
                                    && session.phase
                                        == whisper_core::ports::UnifiedSessionPhase::Finalizing
                            })
                        })
                });
            if both_finalizing {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        assert!(
            both_finalizing,
            "closed captures must expose final flushed progress without leaving Finalizing"
        );
        release_tx.send(()).unwrap();

        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            app.dispatch(AppCommand::Refresh).unwrap();
            let entries = ArchiveStore::scan(&archive_root).unwrap();
            let both_published = [request_a.job_id, request_b.job_id]
                .into_iter()
                .all(|job_id| {
                    entries
                        .iter()
                        .any(|entry| entry.job_id == job_id && entry.complete)
                });
            if both_published && app.view().close_ready {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        let entries = ArchiveStore::scan(&archive_root).unwrap();
        for job_id in [request_a.job_id, request_b.job_id] {
            assert!(
                entries
                    .iter()
                    .any(|entry| entry.job_id == job_id && entry.complete)
            );
            let session = app
                .view()
                .unified_parakeet_sessions
                .iter()
                .find(|session| session.job_id == job_id)
                .expect("published capture remains visible in its session card");
            let total_samples = session
                .total_samples
                .expect("closed capture reports its final sample count");
            assert_eq!(session.captured_samples, total_samples);
            assert_eq!(session.durable_samples, total_samples);
            assert_eq!(
                session.phase,
                whisper_core::ports::UnifiedSessionPhase::Complete
            );
            let generation = archive_root
                .join("transcriptions")
                .join(format!("{:032x}", job_id.0))
                .join("1");
            assert!(generation.join("CURRENT").is_file());
            assert!(generation.join("transcript.txt").is_file());
            assert!(generation.join("transcript.srt").is_file());
        }
        assert!(
            app.view().close_ready,
            "close is ready only after both commits"
        );
        assert_eq!(stop_submissions.load(Ordering::Relaxed), 2);
        assert_eq!(ArchiveStore::scan(&archive_root).unwrap().len(), 2);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn import_application_import_a_and_capture_b_share_one_held_backend_until_b_is_published() {
        const A_ID: u128 = 9_311;
        const B_ID: u128 = 9_312;
        const B_SAMPLES: usize = 35 * 16_000;
        const PART_LIMIT_BYTES: u64 = 32_000;
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-import-capture-ab-{}",
            std::process::id()
        ));
        let archive_root = root.join("archive");
        std::fs::create_dir_all(&archive_root).unwrap();
        let source_path = root.join("import-a.wav");
        let mut source_writer = crate::parakeet_audio::PcmPartWriter::create(&source_path).unwrap();
        source_writer.append(&vec![0_i16; 4 * 16_000]).unwrap();
        source_writer.finish().unwrap();
        let import_request = unified_import_request(&archive_root, &source_path, A_ID);
        let capture_request = unified_live_request(&archive_root, B_ID);
        assert_real_unified_model(&import_request);

        let mut capture_b =
            crate::capture::CaptureStream::empty_test_capture(16_000, 1, "test:import-capture-b");
        for _ in 0..7 {
            capture_b.enqueue_test_block(vec![0.15; 5 * 16_000]);
        }
        let (worker, calls, maximum_active, _call_sources, held_rx, release_tx) = held_worker(1);
        let b_progress = Arc::new(AtomicU64::new(0));
        let stop_submissions = Arc::new(AtomicU64::new(0));
        let runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(worker)
        .with_controlled_capture(capture_b, Some(PART_LIMIT_BYTES));
        let mut app = ImportApplication::new(RuntimeImportPort {
            runtime,
            events: VecDeque::new(),
            capture_progress: Some((JobId(B_ID), Arc::clone(&b_progress))),
            document_revision: None,
            unified_stop_submissions: Some(Arc::clone(&stop_submissions)),
            controlled_capture_blocks: None,
        });

        app.dispatch(AppCommand::StartImport {
            request: import_request.clone(),
        })
        .unwrap();
        let held_deadline = Instant::now() + Duration::from_secs(5);
        let mut held_source = None;
        while Instant::now() < held_deadline {
            app.dispatch(AppCommand::Refresh).unwrap();
            match held_rx.try_recv() {
                Ok(source) => {
                    held_source = Some(source);
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    thread::sleep(Duration::from_millis(2));
                }
                Err(error) => panic!("import A worker exited before holding: {error}"),
            }
        }
        assert!(
            held_source.is_some(),
            "import A final inference was not held"
        );
        assert_eq!(calls.load(Ordering::Relaxed), 1);

        app.dispatch(AppCommand::StartUnifiedLiveConfigured {
            request: capture_request.clone(),
        })
        .unwrap();
        let b_pump_deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < b_pump_deadline && b_progress.load(Ordering::Relaxed) == 0 {
            app.dispatch(AppCommand::Refresh).unwrap();
            thread::sleep(Duration::from_millis(2));
        }
        let b_written_before_stop = b_progress.load(Ordering::Relaxed);
        assert!(b_written_before_stop > 0);
        assert!(b_written_before_stop < B_SAMPLES as u64);
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        assert_eq!(maximum_active.load(Ordering::Relaxed), 1);

        app.dispatch(AppCommand::StopUnifiedLive {
            job_id: capture_request.job_id,
            generation: capture_request.generation,
        })
        .unwrap();
        app.dispatch(AppCommand::StopUnifiedLive {
            job_id: capture_request.job_id,
            generation: capture_request.generation,
        })
        .unwrap();
        assert_eq!(stop_submissions.load(Ordering::Relaxed), 1);
        release_tx.send(()).unwrap();

        let publication_deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < publication_deadline {
            app.dispatch(AppCommand::Refresh).unwrap();
            let entries = ArchiveStore::scan(&archive_root).unwrap();
            if [JobId(A_ID), JobId(B_ID)].into_iter().all(|job_id| {
                entries
                    .iter()
                    .any(|entry| entry.job_id == job_id && entry.complete)
            }) {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        let entries = ArchiveStore::scan(&archive_root).unwrap();
        for job_id in [JobId(A_ID), JobId(B_ID)] {
            let entry = entries
                .iter()
                .find(|entry| entry.job_id == job_id && entry.complete)
                .expect("both the file import and capture must reach publication");
            let generation = archive_root
                .join("transcriptions")
                .join(format!("{:032x}", job_id.0))
                .join("1");
            assert!(generation.join("transcript.txt").is_file());
            assert!(generation.join("transcript.srt").is_file());
            let basename = entry.export_basename.as_ref().unwrap();
            assert!(
                archive_root
                    .join("exports")
                    .join(format!("{basename}.txt"))
                    .is_file()
            );
            assert!(
                archive_root
                    .join("exports")
                    .join(format!("{basename}.srt"))
                    .is_file()
            );
        }
        let capture_generation = archive_root
            .join("transcriptions")
            .join(format!("{:032x}", B_ID))
            .join("1");
        let manifest: serde_json::Value = serde_json::from_slice(
            &std::fs::read(capture_generation.join("captured-source-parts.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(manifest["source_samples"].as_u64(), Some(B_SAMPLES as u64));
        let parts = manifest["parts"].as_array().unwrap();
        assert!(parts.len() > 1);
        let mut part_cursor = 0_u64;
        for part in parts {
            assert_eq!(part["start_sample"].as_u64(), Some(part_cursor));
            part_cursor = part["end_sample"].as_u64().unwrap();
            assert!(Path::new(part["path"].as_str().unwrap()).is_file());
        }
        assert_eq!(part_cursor, B_SAMPLES as u64);

        let mut reference_capture = crate::capture::CaptureStream::empty_test_capture(
            16_000,
            1,
            "test:import-capture-reference",
        );
        for _ in 0..7 {
            reference_capture.enqueue_test_block(vec![0.15; 5 * 16_000]);
        }
        let mut reference_converter = crate::capture::Pcm16Converter::new(16_000, 1).unwrap();
        let mut expected_pcm = Vec::with_capacity(B_SAMPLES);
        while let Some(block) = reference_capture.try_next_block() {
            expected_pcm.extend(reference_converter.push(&block).unwrap());
            reference_capture.recycle_block(block).unwrap();
        }
        expected_pcm.extend(reference_converter.finish().unwrap());
        assert_eq!(expected_pcm.len(), B_SAMPLES);
        let capture_source = crate::parakeet_audio::PcmPartStore::open(
            capture_generation.join("parakeet-unified/source.wav"),
        );
        let mut sample_cursor = 0_u64;
        while sample_cursor < B_SAMPLES as u64 {
            let end = sample_cursor.saturating_add(80_000).min(B_SAMPLES as u64);
            let samples = capture_source.read_window(sample_cursor..end).unwrap();
            assert_eq!(samples.len() as u64, end - sample_cursor);
            assert!(
                samples
                    .iter()
                    .zip(&expected_pcm[sample_cursor as usize..end as usize])
                    .all(|(actual, expected)| actual.abs_diff(*expected) <= 2)
            );
            sample_cursor = end;
        }
        let coverage = std::fs::read_to_string(
            capture_generation.join("parakeet-unified/document/coverage.jsonl"),
        )
        .unwrap();
        let windows = coverage
            .lines()
            .map(|line| {
                serde_json::from_str::<whisper_core::ports::ParakeetWindowCoverage>(line).unwrap()
            })
            .collect::<Vec<_>>();
        assert!(!windows.is_empty());
        assert_eq!(windows.first().unwrap().range.start_sample, 0);
        assert_eq!(windows.last().unwrap().range.end_sample, B_SAMPLES as u64);
        assert!(
            windows
                .windows(2)
                .all(|pair| pair[0].range.end_sample == pair[1].range.start_sample)
        );
        assert_eq!(maximum_active.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn unified_capture_updates_preview_and_final_document_before_stop_then_publishes_the_tail() {
        const JOB_ID: u128 = 9_321;
        const TOTAL_SAMPLES: u64 = 132 * 16_000;
        const PART_LIMIT_BYTES: u64 = 64_000;
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-live-document-{}",
            std::process::id()
        ));
        let archive_root = root.join("archive");
        std::fs::create_dir_all(&archive_root).unwrap();
        let request = unified_live_request(&archive_root, JOB_ID);
        assert!(Path::new(&request.model_path).is_file());

        let mut capture = crate::capture::CaptureStream::empty_test_capture(
            16_000,
            1,
            "test:unified-document-progress",
        );
        for _ in 0..33 {
            capture.enqueue_test_block(vec![0.2; 4 * 16_000]);
        }
        let document_revision = Arc::new(AtomicU64::new(0));
        let stop_submissions = Arc::new(AtomicU64::new(0));
        let observed_windows = Arc::new(Mutex::new(Vec::new()));
        let runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(Box::new(ObservedTailAwareUnifiedWorker {
            windows: Arc::clone(&observed_windows),
        }))
        .with_controlled_capture(capture, Some(PART_LIMIT_BYTES));
        let mut app = ImportApplication::new(RuntimeImportPort {
            runtime,
            events: VecDeque::new(),
            capture_progress: None,
            document_revision: Some((JobId(JOB_ID), Arc::clone(&document_revision))),
            unified_stop_submissions: Some(Arc::clone(&stop_submissions)),
            controlled_capture_blocks: None,
        });
        app.dispatch(AppCommand::StartUnifiedLiveConfigured {
            request: request.clone(),
        })
        .unwrap();

        let document_directory = archive_root
            .join("transcriptions")
            .join(format!("{:032x}", JOB_ID))
            .join("1/parakeet-unified/document");
        let live_progress_deadline = Instant::now() + Duration::from_secs(30);
        let mut preview_revision = 0_u64;
        let mut preview_range = None;
        while Instant::now() < live_progress_deadline
            && (document_revision.load(Ordering::Relaxed) == 0 || preview_revision < 2)
        {
            app.dispatch(AppCommand::Refresh).unwrap();
            if let Ok(bytes) = std::fs::read(document_directory.join("document.preview.json")) {
                if let Ok(preview) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                    preview_revision = preview["revision"].as_u64().unwrap_or(0);
                    preview_range = Some((
                        preview["segment"]["range"]["start_sample"]
                            .as_u64()
                            .unwrap_or(0),
                        preview["segment"]["range"]["end_sample"]
                            .as_u64()
                            .unwrap_or(0),
                    ));
                }
            }
            thread::sleep(Duration::from_millis(2));
        }
        assert!(
            document_revision.load(Ordering::Relaxed) > 0,
            "a final page must be committed while capture is still open"
        );
        assert!(preview_revision >= 2, "preview must be replaced in place");
        let (preview_start, preview_end) = preview_range.unwrap();
        let observed_windows = observed_windows.lock().unwrap().clone();
        assert!(
            observed_windows.iter().any(|(start, end, pcm_len)| {
                *end - *start == 4 * 16_000 && *pcm_len == 4 * 16_000
            }),
            "controlled worker must receive the 64,000-sample four-second preview PCM window"
        );
        assert!(
            observed_windows.iter().any(|(start, end, pcm_len)| {
                *end - *start == 4 * 16_000
                    && *pcm_len == 4 * 16_000
                    && preview_start >= *start
                    && preview_end <= *end
                    && preview_start < preview_end
            }),
            "preview segment must remain bounded by its actual worker context"
        );
        assert_eq!(stop_submissions.load(Ordering::Relaxed), 0);

        app.dispatch(AppCommand::StopUnifiedLive {
            job_id: request.job_id,
            generation: request.generation,
        })
        .unwrap();
        app.dispatch(AppCommand::StopUnifiedLive {
            job_id: request.job_id,
            generation: request.generation,
        })
        .unwrap();
        assert_eq!(stop_submissions.load(Ordering::Relaxed), 1);

        let publication_deadline = Instant::now() + Duration::from_secs(30);
        while Instant::now() < publication_deadline {
            app.dispatch(AppCommand::Refresh).unwrap();
            if ArchiveStore::scan(&archive_root)
                .unwrap()
                .iter()
                .any(|entry| entry.job_id == request.job_id && entry.complete)
            {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        let entry = ArchiveStore::scan(&archive_root)
            .unwrap()
            .into_iter()
            .find(|entry| entry.job_id == request.job_id && entry.complete)
            .expect("stopped capture must publish the final tail");
        let generation_directory = archive_root
            .join("transcriptions")
            .join(format!("{:032x}", JOB_ID))
            .join("1");
        let transcript =
            std::fs::read_to_string(generation_directory.join("transcript.txt")).unwrap();
        assert!(transcript.contains("terminal-phrase."));
        assert!(generation_directory.join("transcript.srt").is_file());
        let basename = entry.export_basename.unwrap();
        assert!(
            archive_root
                .join("exports")
                .join(format!("{basename}.txt"))
                .is_file()
        );
        assert!(
            archive_root
                .join("exports")
                .join(format!("{basename}.srt"))
                .is_file()
        );

        let manifest: serde_json::Value = serde_json::from_slice(
            &std::fs::read(generation_directory.join("captured-source-parts.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(manifest["source_samples"].as_u64(), Some(TOTAL_SAMPLES));
        let parts = manifest["parts"].as_array().unwrap();
        assert!(parts.len() > 1);
        let mut cursor = 0_u64;
        for part in parts {
            assert_eq!(part["start_sample"].as_u64(), Some(cursor));
            cursor = part["end_sample"].as_u64().unwrap();
            assert!(Path::new(part["path"].as_str().unwrap()).is_file());
        }
        assert_eq!(cursor, TOTAL_SAMPLES);

        let coverage = std::fs::read_to_string(
            generation_directory.join("parakeet-unified/document/coverage.jsonl"),
        )
        .unwrap();
        let windows = coverage
            .lines()
            .map(|line| {
                serde_json::from_str::<whisper_core::ports::ParakeetWindowCoverage>(line).unwrap()
            })
            .collect::<Vec<_>>();
        assert!(
            windows.len() >= 2,
            "the source tail after 120 seconds must be finalized"
        );
        assert_eq!(windows.first().unwrap().range.start_sample, 0);
        assert_eq!(windows.last().unwrap().range.end_sample, TOTAL_SAMPLES);
        assert!(
            windows
                .windows(2)
                .all(|pair| { pair[0].range.end_sample == pair[1].range.start_sample })
        );
    }

    #[test]
    fn controlled_runtime_recycles_scheduler_slots_across_many_unified_imports() {
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-scheduler-lifecycle-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        );
        for sequence in 1..=32_u128 {
            let source_path = root.join(format!("source-{sequence}.wav"));
            let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_path).unwrap();
            writer.append(&vec![0_i16; 48_000]).unwrap();
            writer.finish().unwrap();
            let request = unified_import_request(&root, &source_path, 800 + sequence);
            assert_real_unified_model(&request);
            runtime = runtime.with_controlled_transcriber(controlled_worker().0);
            runtime.run_controlled_unified_import(&request).unwrap();
            assert_eq!(runtime.parakeet_scheduler.active_sessions(), 0);
            assert!(runtime.parakeet_scheduler.in_flight().is_none());
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn import_application_decodes_real_mp3_and_publishes_full_unified_coverage() {
        let fixture = std::env::var_os("WHISPER_PU_L1_MP3_FIXTURE")
            .map(PathBuf::from)
            .expect("WHISPER_PU_L1_MP3_FIXTURE must name the guarded MP3 fixture");
        let fixture = fixture
            .canonicalize()
            .expect("guarded MP3 fixture must exist");
        let source_before = std::fs::read(&fixture).expect("read guarded MP3 fixture");
        assert_eq!(source_before.len(), 22_822);
        assert_eq!(
            format!("{:x}", Sha256::digest(&source_before)),
            "eebce5a2af18d4f929af5c243c50a6bc31b0910a583577cb940e0e4895e7cd80"
        );

        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-mp3-application-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let job_id = JobId(794);
        let request = unified_import_request(&root.join("archive"), &fixture, job_id.0);
        assert_real_unified_model(&request);
        let runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(controlled_worker().0);
        let port = RuntimeImportPort {
            runtime,
            events: VecDeque::new(),
            capture_progress: None,
            document_revision: None,
            unified_stop_submissions: None,
            controlled_capture_blocks: None,
        };
        let mut app = ImportApplication::new(port);
        app.dispatch(AppCommand::StartImport {
            request: request.clone(),
        })
        .unwrap();

        let mut completed = false;
        let mut last_state = None;
        let mut last_message = None;
        for _ in 0..64 {
            let view = app.dispatch(AppCommand::Refresh).unwrap();
            last_state = view.active_job;
            last_message = view.message.clone();
            if last_state == Some((job_id, JobState::Complete)) {
                completed = true;
                break;
            }
            if last_state.is_none() {
                break;
            }
        }
        assert!(
            completed,
            "controlled MP3 import did not publish: state={last_state:?}, message={last_message:?}"
        );

        let destination = root.join("archive");
        let entries = ArchiveStore::scan(&destination).unwrap();
        let entry = entries
            .iter()
            .find(|entry| entry.job_id == job_id && entry.generation == request.generation)
            .unwrap_or_else(|| panic!("MP3 import has no archive entry: {entries:?}"));
        assert!(entry.complete, "MP3 archive entry is incomplete: {entry:?}");
        assert_eq!(
            entry.strategy,
            whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
        );
        assert_eq!(
            ArchiveStore::published_imports_for_strategy(
                &destination,
                &entry.source_sha256,
                whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1,
            )
            .unwrap()
            .len(),
            1
        );
        assert!(
            ArchiveStore::published_imports_for_strategy(
                &destination,
                &entry.source_sha256,
                whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
            )
            .unwrap()
            .is_empty()
        );

        let generation = destination
            .join("transcriptions")
            .join(format!("{:032x}", job_id.0))
            .join(request.generation.get().to_string());
        assert!(generation.join("CURRENT").is_file());
        let decoded_samples = crate::parakeet_audio::PcmPartStore::open(
            generation.join("parakeet-unified/source.wav"),
        )
        .total_samples()
        .unwrap();
        let pending: serde_json::Value =
            serde_json::from_slice(&std::fs::read(generation.join("pending.json")).unwrap())
                .unwrap();
        assert_eq!(pending["source_samples"].as_u64(), Some(decoded_samples));

        let coverage: Vec<whisper_core::ports::ParakeetWindowCoverage> =
            std::fs::read_to_string(generation.join("parakeet-unified/document/coverage.jsonl"))
                .unwrap()
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect();
        assert!(
            !coverage.is_empty(),
            "MP3 coverage must include decoded audio"
        );
        assert_eq!(coverage[0].range.start_sample, 0);
        assert!(
            coverage
                .windows(2)
                .all(|pair| { pair[0].range.end_sample == pair[1].range.start_sample })
        );
        assert_eq!(coverage.last().unwrap().range.end_sample, decoded_samples);

        let internal_txt = std::fs::read(generation.join("transcript.txt")).unwrap();
        let internal_srt = std::fs::read(generation.join("transcript.srt")).unwrap();
        assert!(!internal_txt.is_empty());
        assert!(!internal_srt.is_empty());
        let export_basename = entry.export_basename.as_ref().unwrap();
        assert_eq!(
            std::fs::read(
                destination
                    .join("exports")
                    .join(format!("{export_basename}.txt"))
            )
            .unwrap(),
            internal_txt
        );
        assert_eq!(
            std::fs::read(
                destination
                    .join("exports")
                    .join(format!("{export_basename}.srt"))
            )
            .unwrap(),
            internal_srt
        );

        let source_after = std::fs::read(&fixture).expect("re-read guarded MP3 fixture");
        assert_eq!(
            Sha256::digest(&source_after),
            Sha256::digest(&source_before),
            "the original MP3 source must remain unchanged"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn controlled_bounded_gpu_prepare_uses_identity_without_legacy_session_qualification() {
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-gpu-prepare-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source_path = root.join("source.wav");
        let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_path).unwrap();
        writer.append(&vec![0_i16; 3 * 16_000]).unwrap();
        writer.finish().unwrap();
        let mut request = unified_import_request(&root.join("archive"), &source_path, 799);
        request.config.compute = ComputeChoice::Gpu;
        assert_real_unified_model(&request);
        let mut candidate = fake_gpu_candidate();
        candidate.generation = request.generation.get();
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(controlled_worker().0);
        let mut events = VecDeque::new();

        runtime
            .prepare(request, Some(candidate), &mut events)
            .unwrap();

        assert!(matches!(events.pop_front(), Some(ImportEvent::Prepared(_))));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn controlled_document_drain_reads_one_segment_per_poll_and_preserves_all_pages() {
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-page-drain-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source_path = root.join("source.wav");
        let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_path).unwrap();
        writer.append(&vec![0_i16; 121 * 16_000]).unwrap();
        writer.finish().unwrap();
        let request = unified_import_request(&root.join("archive"), &source_path, 791);
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(controlled_worker().0);
        runtime.run_controlled_unified_import(&request).unwrap();
        let drain = runtime.document_drains.front().unwrap();
        assert_eq!(drain.revision, 2);
        assert_eq!(drain.next_sequence, 1);
        assert!(matches!(
            runtime.poll_unified_document(),
            Some(Ok(ImportEvent::Ready { .. }))
        ));
        for expected in 1..=2 {
            assert!(
                matches!(runtime.poll_unified_document(), Some(Ok(ImportEvent::Segment(segment))) if segment.segment_id.0 == expected)
            );
            assert_eq!(
                runtime.document_drains.front().unwrap().next_sequence,
                expected + 1
            );
        }
        assert!(matches!(
            runtime.poll_unified_document(),
            Some(Ok(ImportEvent::Progress {
                completed_samples,
                total_samples,
                ..
            })) if completed_samples == 121 * 16_000 && total_samples == 121 * 16_000
        ));
        assert!(
            matches!(runtime.poll_unified_document(), Some(Ok(ImportEvent::End { last_sequence: 2, last_offset, .. })) if last_offset == 121 * 16_000)
        );
        assert!(runtime.document_drains.is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn controlled_document_drain_terminates_after_page_read_error() {
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-page-drain-error-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source_path = root.join("source.wav");
        let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_path).unwrap();
        writer.append(&vec![0_i16; 3 * 16_000]).unwrap();
        writer.finish().unwrap();
        let request = unified_import_request(&root.join("archive"), &source_path, 792);
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(controlled_worker().0);
        runtime.run_controlled_unified_import(&request).unwrap();
        let document_path = root.join(format!(
            "archive/transcriptions/{:032x}/1/parakeet-unified/document",
            request.job_id.0
        ));
        let mut port = RuntimeImportPort {
            runtime,
            events: VecDeque::new(),
            capture_progress: None,
            document_revision: None,
            unified_stop_submissions: None,
            controlled_capture_blocks: None,
        };

        assert!(matches!(port.poll(), Ok(Some(ImportEvent::Ready { .. }))));
        std::fs::remove_dir_all(document_path).unwrap();
        assert!(matches!(
            port.poll(),
            Ok(Some(ImportEvent::Failed { job_id, generation, message }))
                if (job_id, generation) == (request.job_id, request.generation)
                    && message.starts_with("Document:")
        ));
        assert!(port.runtime.document_drains.is_empty());
        assert!(
            matches!(port.poll(), Ok(None)),
            "a terminal page error must be emitted only once"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unified_terminal_failures_release_all_parent_state_and_preserve_source_audio() {
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-failure-recycling-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let child_observations = Arc::new(Mutex::new(Vec::new()));
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(Box::new(FailedUnifiedWorker))
        .with_child_observer(Arc::clone(&child_observations));

        for attempt in 0..5_u128 {
            let source_path = root.join(format!("source-{attempt}.wav"));
            let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_path).unwrap();
            writer.append(&vec![1_000_i16; 3 * 16_000]).unwrap();
            writer.finish().unwrap();
            let request =
                unified_import_request(&root.join("archive"), &source_path, 9_400 + attempt);
            runtime.start_controlled_unified_import(&request).unwrap();

            let failure_deadline = Instant::now() + Duration::from_secs(5);
            let mut failed = false;
            while Instant::now() < failure_deadline {
                if let Some(Err((job_id, generation, message))) = runtime.poll() {
                    assert_eq!(job_id, request.job_id);
                    assert_eq!(generation, request.generation);
                    assert!(message.contains("controlled worker failure"));
                    failed = true;
                    break;
                }
                thread::sleep(Duration::from_millis(2));
            }
            assert!(
                failed,
                "attempt {attempt} did not reach its terminal failure"
            );
            assert!(runtime.unified_sessions.is_empty());
            assert!(runtime.unified_archives.is_empty());
            assert!(runtime.document_drains.is_empty());
            assert_eq!(runtime.parakeet_scheduler.active_sessions(), 0);
            assert!(runtime.unified_sessions.len() <= 2);
            assert!(runtime.unified_archives.len() <= 2);
            assert!(runtime.document_drains.len() <= 2);
            let work_source = PathBuf::from(&request.destination)
                .join("transcriptions")
                .join(format!("{:032x}", request.job_id.0))
                .join(request.generation.get().to_string())
                .join("parakeet-unified/source.wav");
            assert!(
                work_source.is_file(),
                "recoverable decoded audio was removed"
            );
        }
        let observations = child_observations.lock().unwrap();
        controlled_child_job_bound(&observations).unwrap();
        let commands = observations
            .iter()
            .filter(|item| item.event == "Command")
            .collect::<Vec<_>>();
        assert_eq!(commands.len(), 5);
        assert!(
            commands
                .iter()
                .all(|item| item.retained_job_ids <= UNIFIED_CHILD_JOB_CAPACITY)
        );
        let incarnations = commands
            .iter()
            .map(|item| item.incarnation)
            .collect::<HashSet<_>>();
        assert_eq!(incarnations.len(), 5);
        for command in commands {
            assert!(command.job_id.is_some());
            assert!(
                command
                    .generation
                    .is_some_and(|generation| generation.get() > 0)
            );
            assert_eq!(command.attempt, Some(1));
            assert_eq!(command.wire_request_id, 1);
            let events = observations
                .iter()
                .filter(|item| item.incarnation == command.incarnation)
                .map(|item| item.event)
                .collect::<Vec<_>>();
            assert_eq!(
                events,
                ["Spawn", "Ready", "Command", "Stop", "Exit", "Reaped"]
            );
        }
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn unrecycled_controlled_child_rejects_third_job_with_bounded_table() {
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-child-table-mutant-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let child_observations = Arc::new(Mutex::new(Vec::new()));
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(Box::new(FailedUnifiedWorker))
        .with_unrecycled_child_mutant(Arc::clone(&child_observations));

        for attempt in 0..3_u128 {
            let source_path = root.join(format!("source-{attempt}.wav"));
            let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_path).unwrap();
            writer.append(&vec![1_000_i16; 3 * 16_000]).unwrap();
            writer.finish().unwrap();
            let request =
                unified_import_request(&root.join("archive"), &source_path, 9_440 + attempt);
            runtime.start_controlled_unified_import(&request).unwrap();

            let failure_deadline = Instant::now() + Duration::from_secs(5);
            let mut failed = false;
            while Instant::now() < failure_deadline {
                if let Some(Err((job_id, generation, message))) = runtime.poll() {
                    assert_eq!((job_id, generation), (request.job_id, request.generation));
                    let expected_failure = if attempt < 2 {
                        "controlled worker failure"
                    } else {
                        "ChildSessionCapacity"
                    };
                    assert!(message.contains(expected_failure));
                    failed = true;
                    break;
                }
                thread::sleep(Duration::from_millis(2));
            }
            assert!(
                failed,
                "mutant attempt {attempt} did not terminate in the parent"
            );
            assert!(runtime.unified_sessions.is_empty());
            assert!(runtime.unified_archives.is_empty());
            assert!(runtime.document_drains.is_empty());
            assert_eq!(runtime.parakeet_scheduler.active_sessions(), 0);
            let work_source = PathBuf::from(&request.destination)
                .join("transcriptions")
                .join(format!("{:032x}", request.job_id.0))
                .join(request.generation.get().to_string())
                .join("parakeet-unified/source.wav");
            assert!(
                work_source.is_file(),
                "mutant removed recoverable source audio"
            );
        }

        let observations = child_observations.lock().unwrap().clone();
        let accepted = observations
            .iter()
            .filter(|item| item.event == "Command")
            .collect::<Vec<_>>();
        let rejected = observations
            .iter()
            .filter(|item| item.event == "Rejected")
            .collect::<Vec<_>>();
        assert_eq!(accepted.len(), 2);
        assert_eq!(accepted[0].retained_job_ids, 1);
        assert_eq!(accepted[1].retained_job_ids, 2);
        assert_eq!(rejected.len(), 1);
        assert_eq!(rejected[0].retained_job_ids, 2);
        assert!(
            accepted
                .iter()
                .chain(rejected.iter())
                .all(|item| item.incarnation == accepted[0].incarnation)
        );
        assert_eq!(
            accepted
                .iter()
                .chain(rejected.iter())
                .map(|item| item.wire_request_id)
                .collect::<Vec<_>>(),
            [1, 2, 3],
            "wire request IDs remain positive and monotone inside the unrecycled child"
        );
        assert_eq!(rejected[0].job_id, Some(JobId(9_442)));
        assert!(
            observations
                .iter()
                .all(|item| item.retained_job_ids <= UNIFIED_CHILD_JOB_CAPACITY),
            "the mutant retains the production child admission bound"
        );
        assert!(controlled_child_job_bound(&observations).is_ok());
        assert!(!observations.iter().any(|item| item.event == "Reaped"));

        runtime
            .unified_executor
            .as_mut()
            .unwrap()
            .stop_and_reap()
            .unwrap();
        let observations = child_observations.lock().unwrap();
        assert!(observations.iter().any(|item| item.event == "Stop"));
        assert!(observations.iter().any(|item| item.event == "Exit"));
        assert!(observations.iter().any(|item| item.event == "Reaped"));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn unified_document_finalization_failure_releases_session_and_allows_replacement() {
        const JOB_ID: u128 = 9_423;
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-finalization-failure-{}",
            std::process::id()
        ));
        let archive_root = root.join("archive");
        std::fs::create_dir_all(&archive_root).unwrap();
        let request = unified_live_request(&archive_root, JOB_ID);
        let session = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: request.job_id,
            generation: request.generation,
        };
        let capture = CaptureStream::empty_test_capture(16_000, 1, "test:finalization-failure");
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(Box::new(TailAwareUnifiedWorker))
        .with_finish_document_failure_for(session);
        runtime
            .start_controlled_unified_capture(&request, capture, None)
            .unwrap();
        runtime
            .unified_sessions
            .get_mut(&session)
            .unwrap()
            .source
            .enqueue_test_block(vec![0.1; 4 * 16_000]);
        runtime
            .stop_controlled_unified_capture(
                request.job_id,
                request.generation,
                &mut VecDeque::new(),
            )
            .unwrap();

        let deadline = Instant::now() + Duration::from_secs(20);
        let mut terminal_error = None;
        while Instant::now() < deadline {
            if let Some(Err((job_id, generation, message))) = runtime.poll() {
                assert_eq!((job_id, generation), (request.job_id, request.generation));
                terminal_error = Some(message);
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        assert!(
            terminal_error
                .as_deref()
                .is_some_and(|message| message.contains("injected document finalization failure"))
        );
        assert!(
            runtime.poll().is_none(),
            "finalization failure is emitted once"
        );
        assert!(!runtime.unified_sessions.contains_key(&session));
        assert!(!runtime.unified_archives.contains_key(&session));
        assert!(runtime.document_drains.is_empty());
        assert_eq!(runtime.parakeet_scheduler.active_sessions(), 0);
        assert!(
            archive_root
                .join("transcriptions")
                .join(format!("{:032x}", request.job_id.0))
                .join(request.generation.get().to_string())
                .join("parakeet-unified/source.wav")
                .is_file()
        );

        let replacement = unified_live_request(&archive_root, JOB_ID + 1);
        let replacement_session = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: replacement.job_id,
            generation: replacement.generation,
        };
        let mut replacement_capture =
            CaptureStream::empty_test_capture(16_000, 1, "test:finalization-replacement");
        replacement_capture.enqueue_test_block(vec![0.1; 4 * 16_000]);
        runtime
            .start_controlled_unified_capture(&replacement, replacement_capture, None)
            .unwrap();
        assert!(runtime.unified_sessions.contains_key(&replacement_session));
        assert_eq!(runtime.unified_sessions.len(), 1);
        assert_eq!(runtime.unified_archives.len(), 1);
        runtime
            .stop_controlled_unified_capture(
                replacement.job_id,
                replacement.generation,
                &mut VecDeque::new(),
            )
            .unwrap();
        let replacement_deadline = Instant::now() + Duration::from_secs(10);
        let mut replacement_ended = false;
        while Instant::now() < replacement_deadline {
            match runtime.poll() {
                Some(Ok(ImportEvent::End {
                    job_id, generation, ..
                })) if (job_id, generation) == (replacement.job_id, replacement.generation) => {
                    replacement_ended = true;
                    break;
                }
                Some(Err((job_id, generation, message)))
                    if (job_id, generation) == (replacement.job_id, replacement.generation) =>
                {
                    panic!("replacement capture failed before terminal End: {message}");
                }
                Some(_) => {}
                None => thread::sleep(Duration::from_millis(2)),
            }
        }
        assert!(replacement_ended, "replacement capture did not reach End");
        assert!(!runtime.unified_sessions.contains_key(&replacement_session));
        assert!(runtime.document_drains.is_empty());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn unified_runtime_keeps_reserved_final_turn_through_preparation_with_peer_preview_ready() {
        const A_ID: u128 = 9_424;
        const B_ID: u128 = 9_425;
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-reserved-final-preview-{}",
            std::process::id()
        ));
        let archive_root = root.join("archive");
        std::fs::create_dir_all(&archive_root).unwrap();
        let source_a = root.join("source-a.wav");
        let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_a).unwrap();
        writer.append(&vec![10_000_i16; 121 * 16_000]).unwrap();
        writer.finish().unwrap();
        let source_b = root.join("source-b.wav");
        let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_b).unwrap();
        writer.append(&vec![0_i16; 5 * 16_000]).unwrap();
        writer.finish().unwrap();
        let request_a = unified_import_request(&archive_root, &source_a, A_ID);
        let request_b = unified_import_request(&archive_root, &source_b, B_ID);
        let session_a = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: request_a.job_id,
            generation: request_a.generation,
        };
        let session_b = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: request_b.job_id,
            generation: request_b.generation,
        };
        let (worker, calls, maximum_active, call_sources, held_rx, release_tx) = held_worker(1);
        let child_observations = Arc::new(Mutex::new(Vec::new()));
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(worker)
        .with_child_observer(Arc::clone(&child_observations));
        let ready_gate = runtime.hold_next_child_ready();
        runtime.start_controlled_unified_import(&request_a).unwrap();
        runtime.start_controlled_unified_import(&request_b).unwrap();

        assert!(runtime.poll_unified_sessions().is_none());
        let reserved = runtime.parakeet_scheduler.in_flight().unwrap();
        assert_eq!(reserved.session, session_a);
        assert!(matches!(
            reserved.work,
            whisper_core::parakeet_scheduler::ScheduledWork::Final { .. }
        ));
        assert_eq!(calls.load(Ordering::Relaxed), 0);
        assert!(
            runtime.unified_sessions[&session_a]
                .pending_prepared
                .is_some()
        );
        assert!(
            !child_observations
                .lock()
                .unwrap()
                .iter()
                .any(|item| item.event == "Ready")
        );

        runtime
            .parakeet_scheduler
            .queue_preview(session_b, 5 * 16_000)
            .unwrap();
        assert!(runtime.poll_unified_sessions().is_none());
        assert_eq!(runtime.parakeet_scheduler.in_flight(), Some(reserved));
        assert!(
            runtime.unified_sessions[&session_a]
                .pending_prepared
                .is_some()
        );
        assert_eq!(calls.load(Ordering::Relaxed), 0);
        let (ready_released, ready_changed) = &*ready_gate;
        *ready_released.lock().unwrap() = true;
        ready_changed.notify_all();
        let ready_deadline = Instant::now() + Duration::from_secs(5);
        let mut ready_seen = false;
        let mut held_source = None;
        while Instant::now() < ready_deadline && held_source.is_none() {
            if let Some(Err((job_id, generation, message))) = runtime.poll_unified_sessions() {
                panic!(
                    "reserved turn failed after READY release for {job_id:?}/{generation:?}: {message}"
                );
            }
            ready_seen |= child_observations
                .lock()
                .unwrap()
                .iter()
                .any(|item| item.event == "Ready");
            match held_rx.try_recv() {
                Ok(source) => held_source = Some(source),
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    thread::sleep(Duration::from_millis(2));
                }
                Err(error) => panic!("held worker exited after READY release: {error}"),
            }
        }
        assert!(
            ready_seen,
            "controlled child READY was not observed after release"
        );
        assert_eq!(held_source.as_deref(), Some("A"));
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        assert!(runtime.poll_unified_sessions().is_none());
        assert_eq!(runtime.parakeet_scheduler.in_flight(), Some(reserved));
        assert_eq!(calls.load(Ordering::Relaxed), 1);

        release_tx.send(()).unwrap();
        let peer_preview_deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < peer_preview_deadline && calls.load(Ordering::Relaxed) < 2 {
            let _ = runtime.poll_unified_sessions();
            thread::sleep(Duration::from_millis(2));
        }
        let call_log = call_sources.lock().unwrap();
        assert!(call_log.len() >= 2);
        assert_eq!(call_log[0], "A:final");
        assert_eq!(call_log[1], "B:preview");
        assert_eq!(maximum_active.load(Ordering::Relaxed), 1);
        drop(call_log);

        let recycled_ready_gate = Arc::new((Mutex::new(false), std::sync::Condvar::new()));
        runtime.controlled_ready_gate = Some(Arc::clone(&recycled_ready_gate));
        let ready_deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < ready_deadline {
            if let Some(Err((job_id, generation, message))) = runtime.poll_unified_sessions() {
                panic!("reserved turn session {job_id:?}/{generation:?} failed: {message}");
            }
            let observations = child_observations.lock().unwrap();
            let recycled_spawned = observations
                .iter()
                .filter(|item| item.event == "Spawn")
                .count()
                >= 2;
            drop(observations);
            if recycled_spawned
                && runtime
                    .unified_sessions
                    .get(&session_a)
                    .is_some_and(|state| state.pending_prepared.is_some())
            {
                break;
            }
            let _ = runtime.poll_unified_document();
            thread::sleep(Duration::from_millis(2));
        }
        let reserved_after_recycle = runtime.parakeet_scheduler.in_flight().unwrap();
        assert_eq!(reserved_after_recycle.session, session_a);
        assert!(matches!(
            reserved_after_recycle.work,
            whisper_core::parakeet_scheduler::ScheduledWork::Final { .. }
        ));
        assert!(
            runtime.unified_sessions[&session_a]
                .pending_prepared
                .is_some()
        );
        assert_eq!(calls.load(Ordering::Relaxed), 3);
        {
            let observations = child_observations.lock().unwrap();
            let second_incarnation = observations
                .iter()
                .filter(|item| item.event == "Spawn")
                .nth(1)
                .expect("recycled child spawned")
                .incarnation;
            assert!(
                !observations.iter().any(|item| {
                    item.event == "Ready" && item.incarnation == second_incarnation
                })
            );
        }
        {
            let (released, changed) = &*recycled_ready_gate;
            *released.lock().unwrap() = true;
            changed.notify_all();
        }

        let drain_deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < drain_deadline
            && (!runtime.unified_sessions.is_empty() || !runtime.document_drains.is_empty())
        {
            if let Some(Err((job_id, generation, message))) = runtime.poll_unified_sessions() {
                panic!("reserved turn session {job_id:?}/{generation:?} failed: {message}");
            }
            let _ = runtime.poll_unified_document();
            thread::sleep(Duration::from_millis(2));
        }
        assert!(runtime.unified_sessions.is_empty());
        assert!(runtime.document_drains.is_empty());
        assert_eq!(runtime.parakeet_scheduler.active_sessions(), 0);
        let observations = child_observations.lock().unwrap();
        controlled_child_job_bound(&observations).unwrap();
        let spawns = observations
            .iter()
            .filter(|item| item.event == "Spawn")
            .map(|item| item.incarnation)
            .collect::<Vec<_>>();
        assert_eq!(
            spawns.len(),
            2,
            "terminal B must be reaped before A resumes"
        );
        for incarnation in &spawns {
            let events = observations
                .iter()
                .filter(|item| item.incarnation == *incarnation)
                .map(|item| item.event)
                .collect::<Vec<_>>();
            assert_eq!(events.first(), Some(&"Spawn"));
            assert_eq!(events.get(1), Some(&"Ready"));
            assert!(events.iter().position(|event| *event == "Reaped").is_some());
        }
        let reaped_first = observations
            .iter()
            .position(|item| item.event == "Reaped" && item.incarnation == spawns[0])
            .unwrap();
        let spawned_second = observations
            .iter()
            .position(|item| item.event == "Spawn" && item.incarnation == spawns[1])
            .unwrap();
        assert!(reaped_first < spawned_second);
        drop(observations);
        let final_order = call_sources
            .lock()
            .unwrap()
            .iter()
            .filter(|entry| entry.ends_with(":final"))
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(
            final_order,
            ["A:final", "B:final", "A:final"],
            "final turns must retain their order across child recycle"
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn unified_child_retains_only_two_job_ids_until_real_worker_recycle() {
        use whisper_core::parakeet_scheduler::{ScheduledCall, ScheduledWork, SchedulerSession};

        let (worker, calls) = controlled_worker();
        let mut executor = UnifiedInferenceExecutor::start(worker).unwrap();
        let command_for = |job_id| UnifiedInferenceCommand {
            call: ScheduledCall {
                session: SchedulerSession {
                    job_id: JobId(job_id),
                    generation: Generation::first(),
                },
                incarnation: 1,
                request_id: 1,
                attempt: 0,
                work: ScheduledWork::Final { sequence: 1 },
            },
            window: whisper_core::parakeet_session::AudioWindow {
                sequence: 1,
                central_start_sample: 0,
                central_end_sample: 1,
                native_start_sample: 0,
                native_end_sample: 1,
            },
            pcm: vec![0],
        };
        let await_result = |executor: &mut UnifiedInferenceExecutor| {
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                if let Some(result) = executor.try_result().unwrap() {
                    break result;
                }
                assert!(
                    Instant::now() < deadline,
                    "controlled child result timed out"
                );
                thread::sleep(Duration::from_millis(2));
            }
        };

        for job_id in [9_428_u128, 9_429] {
            executor.dispatch(command_for(job_id)).unwrap();
            let result = await_result(&mut executor);
            assert_eq!(result.call.session.job_id, JobId(job_id));
            assert!(result.words.is_ok());
        }
        assert_eq!(calls.load(Ordering::Relaxed), 2);

        executor.dispatch(command_for(9_430)).unwrap();
        let rejected = await_result(&mut executor);
        assert_eq!(rejected.call.session.job_id, JobId(9_430));
        assert!(rejected.words.unwrap_err().contains("ChildSessionCapacity"));
        assert_eq!(calls.load(Ordering::Relaxed), 2);

        executor.stop_and_reap().unwrap();
        executor.restart().unwrap();
        executor.dispatch(command_for(9_430)).unwrap();
        let recycled = await_result(&mut executor);
        assert!(recycled.words.is_ok());
        assert_eq!(calls.load(Ordering::Relaxed), 3);
        executor.stop_and_reap().unwrap();
    }

    #[test]
    fn panicking_unified_worker_terminates_the_inflight_session_and_its_peer() {
        const A_ID: u128 = 9_426;
        const B_ID: u128 = 9_427;
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-worker-panic-{}",
            std::process::id()
        ));
        let archive_root = root.join("archive");
        std::fs::create_dir_all(&archive_root).unwrap();
        let source_a = root.join("source-a.wav");
        let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_a).unwrap();
        writer.append(&vec![10_000_i16; 121 * 16_000]).unwrap();
        writer.finish().unwrap();
        let source_b = root.join("source-b.wav");
        let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_b).unwrap();
        writer.append(&vec![0_i16; 5 * 16_000]).unwrap();
        writer.finish().unwrap();
        let request_a = unified_import_request(&archive_root, &source_a, A_ID);
        let request_b = unified_import_request(&archive_root, &source_b, B_ID);
        let session_a = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: request_a.job_id,
            generation: request_a.generation,
        };
        let session_b = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: request_b.job_id,
            generation: request_b.generation,
        };
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(Box::new(TailAwareUnifiedWorker))
        .with_panicking_transcriber_for(session_b);
        runtime.start_controlled_unified_import(&request_a).unwrap();
        runtime.start_controlled_unified_import(&request_b).unwrap();

        let deadline = Instant::now() + Duration::from_secs(20);
        let mut failed_b = false;
        while Instant::now() < deadline {
            if let Some(Err((job_id, generation, message))) = runtime.poll_unified_sessions() {
                assert_eq!(
                    (job_id, generation),
                    (request_b.job_id, request_b.generation)
                );
                assert!(message.contains("worker thread panicked"));
                failed_b = true;
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        assert!(failed_b, "the injected worker panic did not terminate B");
        assert!(matches!(
            runtime.poll(),
            Some(Ok(ImportEvent::Failed { job_id, generation, message }))
                if job_id == request_a.job_id
                    && generation == request_a.generation
                    && message.contains("peer session terminated")
        ));
        assert!(!runtime.unified_sessions.contains_key(&session_a));
        assert!(!runtime.unified_sessions.contains_key(&session_b));
        assert!(!runtime.unified_archives.contains_key(&session_a));
        assert!(!runtime.unified_archives.contains_key(&session_b));
        assert_eq!(runtime.parakeet_scheduler.active_sessions(), 0);
        assert!(runtime.parakeet_scheduler.in_flight().is_none());
        let executor = runtime.unified_executor.as_ref().unwrap();
        assert!(executor.thread.is_none());
        assert!(executor.worker.is_none());
        assert!(
            runtime
                .start_controlled_unified_import(&unified_import_request(
                    &archive_root,
                    &source_a,
                    A_ID + 1,
                ))
                .is_err()
        );
        for request in [&request_a, &request_b] {
            assert!(
                archive_root
                    .join("transcriptions")
                    .join(format!("{:032x}", request.job_id.0))
                    .join(request.generation.get().to_string())
                    .join("parakeet-unified/source.wav")
                    .is_file()
            );
        }
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn unified_document_drains_are_included_in_the_two_session_admission_bound() {
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-drain-capacity-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let (worker, _) = controlled_worker();
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(worker);
        let mut requests = Vec::new();
        for id in [9_410_u128, 9_411] {
            let source_path = root.join(format!("source-{id}.wav"));
            let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_path).unwrap();
            writer.append(&vec![0_i16; 3 * 16_000]).unwrap();
            writer.finish().unwrap();
            let request = unified_import_request(&root.join("archive"), &source_path, id);
            runtime.start_controlled_unified_import(&request).unwrap();
            requests.push(request);
        }

        let drain_deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < drain_deadline
            && (runtime.unified_sessions.len() != 0 || runtime.document_drains.len() != 2)
        {
            if let Some(Err((job_id, generation, message))) = runtime.poll_unified_sessions() {
                panic!("session {job_id:?}/{generation:?} failed before document drain: {message}");
            }
            assert!(runtime.unified_sessions.len() <= 2);
            assert!(runtime.unified_archives.len() <= 2);
            assert!(runtime.document_drains.len() <= 2);
            thread::sleep(Duration::from_millis(2));
        }
        assert!(runtime.unified_sessions.is_empty());
        assert_eq!(runtime.unified_archives.len(), 2);
        assert_eq!(runtime.document_drains.len(), 2);
        // The drain queue itself participates in admission even if an archive
        // handle has already been released by a terminal path.
        runtime.unified_archives.clear();

        let third_source = root.join("source-third.wav");
        let mut writer = crate::parakeet_audio::PcmPartWriter::create(&third_source).unwrap();
        writer.append(&vec![0_i16; 3 * 16_000]).unwrap();
        writer.finish().unwrap();
        let third = unified_import_request(&root.join("archive"), &third_source, 9_412);
        assert!(runtime.start_controlled_unified_import(&third).is_err());
        assert!(runtime.unified_archives.is_empty());
        assert_eq!(runtime.document_drains.len(), 2);
        assert!(
            !root
                .join("archive/transcriptions")
                .join(format!("{:032x}", third.job_id.0))
                .exists()
        );
        assert_eq!(requests.len(), 2);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn disconnected_unified_inference_result_fails_the_inflight_session_and_preserves_its_peer() {
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-disconnect-correlation-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let session_b = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: JobId(9_421),
            generation: Generation::first(),
        };
        let (worker, calls) = controlled_worker();
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(worker)
        .with_disconnected_result_for(session_b);

        let source_a = root.join("source-a.wav");
        let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_a).unwrap();
        writer.append(&vec![0_i16; 121 * 16_000]).unwrap();
        writer.finish().unwrap();
        let request_a = unified_import_request(&root.join("archive"), &source_a, 9_420);
        let source_b = root.join("source-b.wav");
        let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_b).unwrap();
        writer.append(&vec![0_i16; 3 * 16_000]).unwrap();
        writer.finish().unwrap();
        let request_b =
            unified_import_request(&root.join("archive"), &source_b, session_b.job_id.0);
        runtime.start_controlled_unified_import(&request_a).unwrap();
        runtime.start_controlled_unified_import(&request_b).unwrap();

        let failure_deadline = Instant::now() + Duration::from_secs(20);
        let mut failed_b = false;
        while Instant::now() < failure_deadline {
            if let Some(Err((job_id, generation, message))) = runtime.poll() {
                assert_eq!(job_id, session_b.job_id);
                assert_eq!(generation, session_b.generation);
                assert!(message.contains("result channel disconnected"));
                failed_b = true;
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        assert!(
            failed_b,
            "the injected B result-channel disconnect was not observed"
        );
        let session_a = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: request_a.job_id,
            generation: request_a.generation,
        };
        assert!(runtime.unified_sessions.contains_key(&session_a));
        assert!(!runtime.unified_sessions.contains_key(&session_b));
        assert!(runtime.unified_archives.contains_key(&session_a));
        assert!(!runtime.unified_archives.contains_key(&session_b));
        assert_eq!(runtime.parakeet_scheduler.in_flight(), None);
        assert_eq!(runtime.parakeet_scheduler.active_sessions(), 1);
        assert!(runtime.unified_executor.as_ref().unwrap().thread.is_some());

        let peer_deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < peer_deadline && runtime.unified_sessions.contains_key(&session_a) {
            if let Some(Err((job_id, generation, message))) = runtime.poll() {
                panic!("peer A failed after B disconnect: {job_id:?}/{generation:?}: {message}");
            }
            thread::sleep(Duration::from_millis(2));
        }
        assert!(!runtime.unified_sessions.contains_key(&session_a));
        assert!(
            runtime
                .document_drains
                .iter()
                .any(|drain| (drain.job_id, drain.generation)
                    == (request_a.job_id, request_a.generation))
        );
        assert!(calls.load(Ordering::Relaxed) >= 3);
        assert!(runtime.unified_sessions.len() <= 2);
        assert!(runtime.unified_archives.len() <= 2);
        assert!(runtime.document_drains.len() <= 2);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn unified_capture_gain_changes_apply_to_the_next_written_pcm_block_and_reject_stale_targets() {
        const JOB_ID: u128 = 9_322;
        let root =
            std::env::temp_dir().join(format!("parakeet-unified-live-gain-{}", std::process::id()));
        let archive_root = root.join("archive");
        std::fs::create_dir_all(&archive_root).unwrap();
        let mut request = unified_live_request(&archive_root, JOB_ID);
        request.microphone = whisper_core::MicrophoneChoice::Device("test:unified-gain".into());
        let mut capture = CaptureStream::empty_test_capture_with_gain(
            16_000,
            1,
            "test:unified-gain",
            whisper_core::microphone_level::GainDb::NEUTRAL,
        );
        capture.enqueue_test_block(vec![0.1; 16_000]);
        let runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(Box::new(TailAwareUnifiedWorker))
        .with_controlled_capture(capture, None);
        let controlled_capture_blocks = Arc::new(Mutex::new(VecDeque::new()));
        let mut app = ImportApplication::new(RuntimeImportPort {
            runtime,
            events: VecDeque::new(),
            capture_progress: None,
            document_revision: None,
            unified_stop_submissions: None,
            controlled_capture_blocks: Some(Arc::clone(&controlled_capture_blocks)),
        });

        app.dispatch(AppCommand::StartUnifiedLiveConfigured {
            request: request.clone(),
        })
        .unwrap();
        app.dispatch(AppCommand::Refresh).unwrap();
        app.dispatch(AppCommand::SetMicrophoneGain {
            job_id: request.job_id,
            generation: request.generation,
            target_identity: "test:unified-gain".into(),
            expected_revision: 0,
            gain: whisper_core::microphone_level::GainDb::new(6).unwrap(),
        })
        .unwrap();
        controlled_capture_blocks.lock().unwrap().push_back((
            request.job_id,
            request.generation,
            vec![0.1; 16_000],
        ));
        app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(app.view().microphone_gain_db, 6);
        assert_eq!(app.view().microphone_gain_revision, 1);
        assert_eq!(app.view().microphone_gain_applied_first_frame, Some(16_000));

        app.dispatch(AppCommand::SetMicrophoneGain {
            job_id: request.job_id,
            generation: request.generation,
            target_identity: "test:wrong-source".into(),
            expected_revision: 1,
            gain: whisper_core::microphone_level::GainDb::new(0).unwrap(),
        })
        .unwrap();
        assert!(matches!(
            app.dispatch(AppCommand::SetMicrophoneGain {
                job_id: request.job_id,
                generation: request.generation,
                target_identity: "test:unified-gain".into(),
                expected_revision: 0,
                gain: whisper_core::microphone_level::GainDb::new(0).unwrap(),
            }),
            Err(whisper_core::application::ApplicationError::InvalidCommand)
        ));
        app.dispatch(AppCommand::Refresh).unwrap();
        assert!(
            app.view()
                .message
                .as_deref()
                .is_some_and(|message| message.contains("source active différente"))
        );
        assert_eq!(app.view().microphone_gain_revision, 1);
        assert_eq!(app.view().microphone_gain_requested_revision, 1);
        assert_eq!(app.view().microphone_gain_requested_db, 6);
        app.dispatch(AppCommand::SetMicrophoneGain {
            job_id: request.job_id,
            generation: request.generation,
            target_identity: "test:unified-gain".into(),
            expected_revision: 1,
            gain: whisper_core::microphone_level::GainDb::new(0).unwrap(),
        })
        .unwrap();
        controlled_capture_blocks.lock().unwrap().push_back((
            request.job_id,
            request.generation,
            vec![0.1; 16_000],
        ));
        app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(app.view().microphone_gain_db, 0);
        assert_eq!(app.view().microphone_gain_revision, 2);
        assert_eq!(app.view().microphone_gain_applied_first_frame, Some(32_000));

        app.dispatch(AppCommand::StopUnifiedLive {
            job_id: request.job_id,
            generation: request.generation,
        })
        .unwrap();
        let publication_deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < publication_deadline {
            app.dispatch(AppCommand::Refresh).unwrap();
            if ArchiveStore::scan(&archive_root)
                .unwrap()
                .iter()
                .any(|entry| entry.job_id == request.job_id && entry.complete)
            {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        let generation = archive_root
            .join("transcriptions")
            .join(format!("{:032x}", JOB_ID))
            .join("1");
        let source = crate::parakeet_audio::PcmPartStore::open(
            generation.join("parakeet-unified/source.wav"),
        );
        let before_gain = source.read_window(2_000..14_000).unwrap();
        let after_gain = source.read_window(18_000..30_000).unwrap();
        let after_recovery = source.read_window(34_000..46_000).unwrap();
        assert!(
            before_gain
                .iter()
                .all(|sample| (3_000..=3_600).contains(sample))
        );
        assert!(
            after_gain
                .iter()
                .all(|sample| (6_000..=7_000).contains(sample))
        );
        assert!(
            after_recovery
                .iter()
                .all(|sample| (3_000..=3_600).contains(sample))
        );
        assert!(
            ArchiveStore::scan(&archive_root)
                .unwrap()
                .iter()
                .any(|entry| entry.job_id == request.job_id && entry.complete)
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn unified_import_and_capture_without_controlled_injection_are_refused_before_hardware_start() {
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-native-refusal-{}",
            std::process::id()
        ));
        let request = unified_import_request(&root, Path::new("missing.wav"), 777);
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("missing-cpu-worker"),
                gpu: PathBuf::from("missing-gpu-worker"),
            },
            Arc::new(PublicationGate::default()),
        );
        let mut events = VecDeque::new();
        runtime.effect(ImportEffect::Prepare(request.clone()), &mut events);
        assert!(
            matches!(events.pop_front(), Some(ImportEvent::Failed { message, .. })
            if message.contains("ModelNotQualified") && message.contains("controlled worker"))
        );
        assert!(
            !root.exists(),
            "native refusal must precede archive preparation"
        );
        runtime.effect(ImportEffect::StartWorker(request), &mut events);
        assert!(
            matches!(events.pop_front(), Some(ImportEvent::Failed { message, .. })
            if message.contains("ModelNotQualified") && message.contains("controlled worker"))
        );

        let mut gpu_request = unified_import_request(&root, Path::new("missing-gpu.wav"), 776);
        gpu_request.config.compute = ComputeChoice::Gpu;
        runtime.effect(ImportEffect::StartWorker(gpu_request), &mut events);
        assert!(matches!(
            events.pop_front(),
            Some(ImportEvent::Failed { message, .. })
                if message.contains("ModelNotQualified")
                    && message.contains("controlled worker")
        ));
        assert!(runtime.unified_sessions.is_empty());
        assert!(runtime.unified_executor.is_none());

        let live_request = unified_live_request(&root, 778);
        runtime.effect(ImportEffect::StartUnifiedLive(live_request), &mut events);
        assert!(
            matches!(events.pop_front(), Some(ImportEvent::Failed { message, .. })
            if message.contains("ModelNotQualified") && message.contains("not enabled")),
            "native capture must be refused before opening a microphone"
        );
        assert!(
            !root.exists(),
            "native capture refusal must precede storage/open side effects"
        );

        let mut gpu_live_request = unified_live_request(&root, 780);
        gpu_live_request.config.compute = ComputeChoice::Gpu;
        runtime.effect(
            ImportEffect::StartUnifiedLive(gpu_live_request),
            &mut events,
        );
        assert!(matches!(
            events.pop_front(),
            Some(ImportEvent::Failed { message, .. })
                if message.contains("ModelNotQualified")
                    && message.contains("not enabled")
        ));
        assert!(runtime.unified_sessions.is_empty());
        assert!(runtime.unified_executor.is_none());

        let live_request = unified_live_request(&root, 779);
        let mut app = ImportApplication::new(RuntimeImportPort {
            runtime,
            events: VecDeque::new(),
            capture_progress: None,
            document_revision: None,
            unified_stop_submissions: None,
            controlled_capture_blocks: None,
        });
        app.dispatch(AppCommand::StartUnifiedLiveConfigured {
            request: live_request,
        })
        .unwrap();
        app.dispatch(AppCommand::Refresh).unwrap();
        assert!(
            !root.exists(),
            "the real application route must refuse before opening capture"
        );
    }

    #[test]
    fn controlled_worker_and_invalid_times_leave_no_complete_archive() {
        for (index, (worker, expected_failure)) in [
            (
                Box::new(FailedUnifiedWorker)
                    as Box<dyn crate::parakeet_pipeline::WindowTranscriber>,
                "controlled worker failure",
            ),
            (
                Box::new(InvalidTimeUnifiedWorker)
                    as Box<dyn crate::parakeet_pipeline::WindowTranscriber>,
                "InvalidTimestamp",
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let root = std::env::temp_dir().join(format!(
                "parakeet-unified-failure-{}-{}",
                std::process::id(),
                index
            ));
            std::fs::create_dir_all(&root).unwrap();
            let source_path = root.join("source.wav");
            let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_path).unwrap();
            writer.append(&vec![0_i16; 3 * 16_000]).unwrap();
            writer.finish().unwrap();
            let request =
                unified_import_request(&root.join("archive"), &source_path, 780 + index as u128);
            assert_real_unified_model(&request);
            let runtime = ImportRuntime::new(
                WorkerPaths {
                    cpu: PathBuf::from("unused-cpu"),
                    gpu: PathBuf::from("unused-gpu"),
                },
                Arc::new(PublicationGate::default()),
            )
            .with_controlled_transcriber(worker);
            let mut app = ImportApplication::new(RuntimeImportPort {
                runtime,
                events: VecDeque::new(),
                capture_progress: None,
                document_revision: None,
                unified_stop_submissions: None,
                controlled_capture_blocks: None,
            });
            app.dispatch(AppCommand::StartImport {
                request: request.clone(),
            })
            .unwrap();
            let deadline = Instant::now() + Duration::from_secs(5);
            let mut terminal_view = None;
            while Instant::now() < deadline {
                let view = app.dispatch(AppCommand::Refresh).unwrap();
                if view.active_job == Some((request.job_id, JobState::Recoverable)) {
                    terminal_view = Some(view);
                    break;
                }
                thread::sleep(Duration::from_millis(2));
            }
            let terminal_view = terminal_view.expect("controlled import must report Recoverable");
            assert_eq!(
                terminal_view.active_job,
                Some((request.job_id, JobState::Recoverable))
            );
            assert!(
                terminal_view
                    .message
                    .as_deref()
                    .is_some_and(|message| message.contains(expected_failure))
            );
            let entries = ArchiveStore::scan(&root.join("archive")).unwrap();
            assert!(!entries.iter().any(|entry| entry.complete));
            std::fs::remove_dir_all(&root).unwrap();
        }
    }

    #[test]
    fn export_collision_keeps_existing_file_and_does_not_publish_unified_archive() {
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-export-collision-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source_path = root.join("source.wav");
        let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_path).unwrap();
        writer.append(&vec![0_i16; 3 * 16_000]).unwrap();
        writer.finish().unwrap();
        let destination = root.join("archive");
        let request = unified_import_request(&destination, &source_path, 790);
        assert_real_unified_model(&request);
        let collision = destination.join("exports/source.txt");
        let transcribe_completed = Arc::new(AtomicU64::new(0));
        let runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(Box::new(ExportCollisionWorker {
            path: collision.clone(),
            transcribe_completed: Arc::clone(&transcribe_completed),
        }));
        let mut app = ImportApplication::new(RuntimeImportPort {
            runtime,
            events: VecDeque::new(),
            capture_progress: None,
            document_revision: None,
            unified_stop_submissions: None,
            controlled_capture_blocks: None,
        });
        app.dispatch(AppCommand::StartImport {
            request: request.clone(),
        })
        .unwrap();
        let write_deadline = Instant::now() + Duration::from_secs(5);
        while transcribe_completed.load(Ordering::Acquire) == 0 && Instant::now() < write_deadline {
            let _ = app.dispatch(AppCommand::Refresh).unwrap();
            thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(
            transcribe_completed.load(Ordering::Acquire),
            1,
            "worker must create the collision after transcription starts"
        );
        let terminal_deadline = Instant::now() + Duration::from_secs(5);
        let mut terminal_view = None;
        while Instant::now() < terminal_deadline {
            let view = app.dispatch(AppCommand::Refresh).unwrap();
            if view.active_job == Some((request.job_id, JobState::Recoverable)) {
                terminal_view = Some(view);
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        let terminal_view = terminal_view.expect("export collision must be terminal");
        assert!(
            terminal_view
                .message
                .as_deref()
                .is_some_and(|message| message.contains("export name collision"))
        );
        assert_eq!(std::fs::read(&collision).unwrap(), b"preexisting export");
        assert!(
            !ArchiveStore::scan(&destination)
                .unwrap()
                .iter()
                .any(|entry| entry.complete)
        );
        let generation = destination
            .join("transcriptions")
            .join(format!("{:032x}", request.job_id.0))
            .join(request.generation.get().to_string());
        assert!(
            !generation.join("CURRENT").exists(),
            "failed export collision must not commit CURRENT"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn microphone_legacy_capture_budget_forces_worker_poll_within_four_ui_events() {
        let mut capture_events_since_worker = 0;
        let mut worker_events = VecDeque::from(["confirmed text", "terminal"]);
        let mut delivered = vec!["durable counters"];
        capture_events_since_worker += 1;
        delivered.push("spectrum telemetry");
        assert!(live_capture_poll_allowed(capture_events_since_worker, true));
        delivered.push("durable counters");
        capture_events_since_worker += 1;
        assert!(!live_capture_poll_allowed(
            capture_events_since_worker,
            true
        ));
        delivered.push(worker_events.pop_front().unwrap());
        capture_events_since_worker = 0;
        assert_eq!(
            delivered,
            [
                "durable counters",
                "spectrum telemetry",
                "durable counters",
                "confirmed text"
            ]
        );
        assert!(!live_capture_poll_allowed(2, true));
        assert!(live_capture_poll_allowed(2, false));
        assert_eq!(worker_events.pop_front(), Some("terminal"));
    }

    #[test]
    fn microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end() {
        let root = std::env::temp_dir().join(format!(
            "legacy-spectrum-fairness-{}-{}",
            std::process::id(),
            whisper_core::microphone_level::monotonic_millis()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let marker = root.join("worker-ready");
        let worker = compile_fake_worker(&root, "cpu-live", &marker);
        let job_id = JobId(881);
        let generation = Generation::first();
        let request = LiveRequest {
            job_id,
            generation,
            model_path: "unused-model".into(),
            model_sha256: [0; 32],
            destination: root.join("archive").to_string_lossy().into_owned(),
            group_offset_samples: 0,
            auto_stop_after_speech_samples: None,
            microphone: whisper_core::MicrophoneChoice::Device("test:legacy".into()),
            microphone_gain_db: 0,
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Cpu,
            },
        };
        let mut capture = CaptureStream::empty_test_capture(16_000, 1, "test:legacy");
        let probe = crate::microphone_probe::MicrophoneProbe::start(
            16_000,
            Some("test:legacy".into()),
            job_id,
            generation,
        )
        .unwrap();
        let staging_path = root.join("pending.pcm");
        let staging = PcmStaging::create(&staging_path).unwrap().into_writer();
        let mut archive = LiveArchive::prepare(&request.destination, request.clone()).unwrap();
        archive.capture_started().unwrap();
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: worker.clone(),
                gpu: worker.clone(),
            },
            Arc::new(PublicationGate::default()),
        );
        let mut transport = ChildWorkerTransport::spawn(&worker).unwrap();
        transport
            .request(WorkerCommand::StartLiveWorker {
                job_id,
                generation,
                instance_id: 1,
                model_path: request.model_path.clone(),
                model_sha256: request.model_sha256,
                config: request.config.clone(),
            })
            .unwrap();
        runtime.transport = Some(transport);
        runtime.active = Some(ImportRequestState {
            job_id,
            generation,
            instance_id: 1,
            backend: BackendKind::Cpu,
            attempt_number: 0,
            import_request: None,
            recovery: LiveRecoveryState::None,
            startup_ready: true,
            gpu_candidate: None,
            gpu_attested: false,
        });
        runtime.live_archive = Some(archive);
        runtime.live = Some(LiveCaptureState {
            request: request.clone(),
            capture,
            probe,
            spectrum_reader: crate::microphone_probe::MicrophoneSpectrumReader::default(),
            last_meter_sequence: 0,
            next_spectrum_telemetry: false,
            telemetry_due_after_event: false,
            converter: Pcm16Converter::new(16_000, 1).unwrap(),
            vad: VoiceActivity::new(),
            staging,
            frame_pending: Vec::with_capacity(320),
            inference_cursor: 0,
            inference_sequence: 1,
            pending_window: None,
            replay_until: 0,
            replay_windows_queued: 0,
            replay_windows_to_skip: 0,
            segment_sequence_base: 1,
            window_sequence_base: 1,
            recovery: LiveRecoveryState::None,
            barrier_m: None,
            inference_admission_closed: false,
            worker_ready: true,
            stop_requested: false,
            stop_enqueued: false,
            started_at_unix_ms: unix_time_ms(),
            captured: 0,
            admitted: 0,
            speech: 0,
            confirmed_fragment_end: 0,
            last_reported_durable: 0,
            failure: None,
        });

        let mut saw_counters = false;
        let mut saw_meter = false;
        let mut saw_spectrum = false;
        for block in 0..12 {
            let samples = (block * 960..(block + 1) * 960)
                .map(|frame| 0.001 * (std::f32::consts::TAU * 64.0 * frame as f32 / 2048.0).sin())
                .collect::<Vec<_>>();
            runtime
                .live
                .as_mut()
                .unwrap()
                .capture
                .enqueue_test_block(samples);
            let mut refresh_events = 0;
            for _ in 0..4 {
                match runtime.poll() {
                    Some(Ok(event)) => {
                        refresh_events += 1;
                        saw_counters |= matches!(event, ImportEvent::LiveCounters { .. });
                        saw_meter |= matches!(event, ImportEvent::MicrophoneMeter(_));
                        saw_spectrum |= matches!(event, ImportEvent::MicrophoneSpectrum(_));
                    }
                    Some(Err(error)) => panic!("legacy runtime poll failed: {error:?}"),
                    None => {}
                }
            }
            assert!(refresh_events <= 4);
            thread::sleep(Duration::from_millis(10));
        }
        let telemetry_deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < telemetry_deadline && !(saw_meter && saw_spectrum) {
            runtime
                .live
                .as_mut()
                .unwrap()
                .capture
                .enqueue_test_block(vec![0.0; 960]);
            if let Some(Ok(event)) = runtime.poll() {
                saw_counters |= matches!(event, ImportEvent::LiveCounters { .. });
                saw_meter |= matches!(event, ImportEvent::MicrophoneMeter(_));
                saw_spectrum |= matches!(event, ImportEvent::MicrophoneSpectrum(_));
            }
            thread::sleep(Duration::from_millis(5));
        }
        assert!(
            saw_counters,
            "legacy capture did not publish durable counters"
        );
        assert!(saw_meter, "legacy capture did not publish RMS telemetry");
        assert!(saw_spectrum, "legacy capture did not publish a spectrum");

        let segment = WorkerSegment {
            job_id,
            generation,
            instance_id: 1,
            segment_id: whisper_core::SegmentId(1),
            range: whisper_core::SourceRange::new(0, 320, 16_000).unwrap(),
            text: "legacy confirmed".into(),
        };
        while runtime.live.as_ref().unwrap().staging.durable_samples() < 320 {
            thread::sleep(Duration::from_millis(5));
        }
        runtime
            .live_archive
            .as_mut()
            .unwrap()
            .persist_segment(
                &segment,
                runtime.live.as_ref().unwrap().staging.durable_samples(),
            )
            .unwrap();
        let transport = runtime.transport.as_mut().unwrap();
        transport.inject_event(WorkerEvent::Segment(
            whisper_core::ipc::WorkerSegmentDto::from(segment.clone()),
        ));
        transport.inject_event(WorkerEvent::WindowFinished {
            job_id,
            generation,
            instance_id: 1,
            sequence: 1,
            range: segment.range,
            last_segment_sequence: 1,
        });
        transport.inject_event(WorkerEvent::End {
            job_id,
            generation,
            instance_id: 1,
            last_sequence: 1,
            last_offset: 320,
        });
        let mut saw_text = false;
        let mut saw_confirmed = false;
        let mut saw_end = false;
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline && !(saw_text && saw_confirmed && saw_end) {
            runtime
                .live
                .as_mut()
                .unwrap()
                .capture
                .enqueue_test_block(vec![0.0; 960]);
            let mut refresh_events = 0;
            for _ in 0..4 {
                match runtime.poll() {
                    Some(Ok(event)) => {
                        refresh_events += 1;
                        match event {
                            ImportEvent::LiveProvisional { .. } => {
                                saw_text = true;
                                runtime.live_pending_persistence = 0;
                            }
                            ImportEvent::WindowFinished { .. } => saw_confirmed = true,
                            ImportEvent::End { .. } => saw_end = true,
                            _ => {}
                        }
                    }
                    Some(Err(error)) => panic!("legacy worker event failed: {error:?}"),
                    None => {}
                }
            }
            assert!(refresh_events <= 4);
            thread::sleep(Duration::from_millis(5));
        }
        assert!(saw_text, "mock worker confirmed text did not progress");
        assert!(saw_confirmed, "window confirmation did not progress");
        assert!(saw_end, "mock worker End did not progress");
        if let Some(mut live) = runtime.live.take() {
            live.stop_capture();
            drop(live);
        }
        runtime.transport.take();
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn microphone_gain_command_reaches_acquisition_and_finalized_wav_while_worker_is_delayed() {
        #[derive(Clone)]
        struct SimulatedPort(Rc<RefCell<Vec<ImportEffect>>>);

        impl ImportIoPort for SimulatedPort {
            fn submit(&mut self, effect: ImportEffect) -> Result<(), PortError> {
                self.0.borrow_mut().push(effect);
                Ok(())
            }

            fn poll(&mut self) -> Result<Option<ImportEvent>, PortError> {
                Ok(None)
            }
        }

        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("target/parakeet-tests/micro-level/t3-gain-e2e")
            .join(format!(
                "{}-{}",
                std::process::id(),
                NEXT_INSTANCE.fetch_add(1, Ordering::Relaxed)
            ));
        fs::create_dir_all(&root).unwrap();
        let request = LiveRequest {
            job_id: JobId(((std::process::id() as u128) << 64) | 0xA551),
            generation: Generation::first(),
            model_path: "delayed-engine-model".into(),
            model_sha256: [1; 32],
            destination: root.to_string_lossy().into_owned(),
            group_offset_samples: 0,
            auto_stop_after_speech_samples: None,
            microphone: whisper_core::MicrophoneChoice::Device("test:mic".into()),
            microphone_gain_db: 0,
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Cpu,
            },
        };
        let effects = Rc::new(RefCell::new(Vec::new()));
        let mut app = ImportApplication::new(SimulatedPort(effects.clone()));
        app.dispatch(AppCommand::StartDirectLiveConfigured {
            request: request.clone(),
        })
        .unwrap();
        let ImportEffect::StartDirectLive(request) = effects.borrow()[0].clone() else {
            panic!("direct start command must produce the direct capture effect")
        };

        let mut capture = crate::capture::CaptureStream::empty_test_capture(16_000, 1, "test:mic");
        capture.enqueue_test_block(vec![0.25; 16_000]);
        let direct =
            crate::direct_live::DirectLive::from_test_capture(request.clone(), capture).unwrap();
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: root.join("unused-cpu.exe"),
                gpu: root.join("unused-gpu.exe"),
            },
            Arc::new(PublicationGate::default()),
        );
        runtime.direct_live = Some(direct);
        let direct = runtime.direct_live.as_mut().unwrap();
        assert!(
            !direct.test_worker_ready(),
            "engine remains delayed in this test"
        );
        app.dispatch(AppCommand::SetMicrophoneGain {
            job_id: request.job_id,
            generation: request.generation,
            target_identity: "test:mic".into(),
            expected_revision: 0,
            gain: whisper_core::GainDb::new(6).unwrap(),
        })
        .unwrap();
        let gain_effect = effects.borrow().last().unwrap().clone();
        let mut events = VecDeque::new();
        runtime.effect(gain_effect, &mut events);
        assert!(
            events.is_empty(),
            "command acceptance is not an acquisition acknowledgement"
        );
        runtime
            .direct_live
            .as_mut()
            .unwrap()
            .pump_capture()
            .unwrap();
        assert!(
            matches!(
                runtime.direct_live.as_mut().unwrap().poll().unwrap(),
                Some(ImportEvent::MicrophoneGainApplied {
                    gain_db: 0,
                    gain_revision: 0,
                    first_frame: 0,
                    ..
                })
            ),
            "pre-command backlog acknowledges only its old revision"
        );
        assert!(matches!(
            runtime.direct_live.as_mut().unwrap().poll().unwrap(),
            Some(ImportEvent::DirectCaptureActive { .. })
        ));
        runtime
            .direct_live
            .as_mut()
            .unwrap()
            .enqueue_test_capture_block(vec![0.25; 16_000]);
        runtime
            .direct_live
            .as_mut()
            .unwrap()
            .pump_capture()
            .unwrap();
        assert!(matches!(
            runtime.direct_live.as_mut().unwrap().poll().unwrap(),
            Some(ImportEvent::MicrophoneGainApplied {
                gain_db: 6,
                gain_revision: 1,
                first_frame: 16_000,
                ..
            })
        ));
        runtime.effect(
            ImportEffect::SetMicrophoneGain {
                job_id: request.job_id,
                generation: request.generation,
                target_identity: "test:other".into(),
                expected_revision: 1,
                gain_db: 0,
            },
            &mut events,
        );
        assert!(matches!(
            events.pop_front(),
            Some(ImportEvent::MicrophoneGainRejected { target_identity, .. })
                if target_identity == "test:other"
        ));
        assert!(
            runtime.direct_live.is_some(),
            "gain rejection must preserve the capture owner"
        );
        app.dispatch(AppCommand::Stop {
            job_id: request.job_id,
            generation: request.generation,
        })
        .unwrap();
        let stop_effect = effects.borrow().last().unwrap().clone();
        runtime.effect(stop_effect, &mut events);
        assert!(matches!(
            events.pop_front(),
            Some(ImportEvent::LiveFinalizing { .. })
        ));

        let wav_path = runtime
            .direct_live
            .as_mut()
            .unwrap()
            .finish_test_wav()
            .unwrap();
        let bytes = fs::read(&wav_path).unwrap();
        assert_eq!(
            u32::from_le_bytes(bytes[40..44].try_into().unwrap()),
            64_000
        );
        let pcm: Vec<i16> = bytes[44..]
            .chunks_exact(2)
            .map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        assert_eq!(pcm.len(), 32_000);
        assert!(pcm[320..15_680].iter().all(|sample| *sample == 8_192));
        assert!(
            pcm[16_320..31_680]
                .iter()
                .all(|sample| (16_300..=16_400).contains(sample))
        );
        fs::remove_file(wav_path).unwrap();
        fs::remove_dir(root).unwrap();
    }

    #[test]
    fn stopping_before_the_first_gain_revision_block_emits_no_applied_acknowledgement() {
        let capture = crate::capture::CaptureStream::empty_test_capture(16_000, 1, "test:mic");
        let (mut test, identity) =
            MicrophoneTestCapture::from_capture(JobId(904), Generation::first(), capture).unwrap();
        let accepted = test
            .set_gain(&identity, 0, whisper_core::GainDb::new(6).unwrap())
            .unwrap();
        assert_eq!(accepted.revision, 1);
        assert!(
            test.pump().unwrap().is_none(),
            "no captured block means no applied event"
        );
        assert!(matches!(
            test.stop(),
            ImportEvent::MicrophoneTestStopped {
                job_id: JobId(904),
                ..
            }
        ));
    }

    #[test]
    fn microphone_test_capture_owns_only_capture_conversion_and_probe_until_stop() {
        let mut capture =
            crate::capture::CaptureStream::empty_test_capture(48_000, 1, "wasapi:test-device");
        let (mut test, identity) =
            MicrophoneTestCapture::from_capture(JobId(904), Generation::first(), capture).unwrap();
        assert_eq!(identity, "wasapi:test-device");

        let mut observed_meter = None;
        let mut observed_spectrum = None;
        let deadline = Instant::now() + Duration::from_secs(2);
        for _ in 0..5 {
            let processed_before = test.probe.processed_chunks();
            test.capture.enqueue_test_block(vec![0.25; 960]);
            if let Some(event) = test.pump().unwrap() {
                match event {
                    ImportEvent::MicrophoneMeter(meter) => observed_meter = Some(meter),
                    ImportEvent::MicrophoneSpectrum(update) => observed_spectrum = Some(update),
                    _ => {}
                }
            }
            while test.probe.processed_chunks() == processed_before && Instant::now() < deadline {
                std::thread::yield_now();
            }
            if let Some(event) = test.pump().unwrap() {
                match event {
                    ImportEvent::MicrophoneMeter(meter) => observed_meter = Some(meter),
                    ImportEvent::MicrophoneSpectrum(update) => observed_spectrum = Some(update),
                    _ => {}
                }
            }
            std::thread::yield_now();
        }
        while (observed_meter.is_none() || observed_spectrum.is_none()) && Instant::now() < deadline
        {
            if let Some(event) = test.pump().unwrap() {
                match event {
                    ImportEvent::MicrophoneMeter(meter) => observed_meter = Some(meter),
                    ImportEvent::MicrophoneSpectrum(update) => observed_spectrum = Some(update),
                    _ => {}
                }
            }
            std::thread::yield_now();
        }
        let meter = observed_meter.expect("probe reports captured audio");
        assert_eq!(meter.source_identity, identity);
        assert_eq!(meter.job_id, JobId(904));
        assert_eq!(
            meter.pre_gain.status,
            whisper_core::microphone_level::MeasurementStatus::Signal
        );
        let whisper_core::microphone_level::MicrophoneSpectrumUpdate::Available(spectrum) =
            observed_spectrum.expect("microphone test publishes a spectrum during acquisition")
        else {
            panic!("expected a current microphone spectrum")
        };
        assert_eq!(spectrum.source_identity, identity);
        assert_eq!(spectrum.pre_gain.len(), 1025);

        let applied = test
            .set_gain(
                &identity,
                0,
                whisper_core::microphone_level::GainDb::new(6).unwrap(),
            )
            .unwrap();
        assert_eq!(applied.gain.value(), 6);
        assert_eq!(applied.revision, 1);

        assert!(matches!(
            test.stop(),
            ImportEvent::MicrophoneTestStopped {
                job_id: JobId(904),
                ..
            }
        ));
    }

    fn automatic_final_test_root(label: &str) -> PathBuf {
        PathBuf::from(std::env::var_os("CARGO_TARGET_DIR").expect("guarded target dir")).join(
            format!(
                "whisper-auto-final-{label}-{}-{}",
                std::process::id(),
                NEXT_AUTO_EXPORT_STAGE.fetch_add(1, Ordering::Relaxed)
            ),
        )
    }

    fn automatic_final_request(job_id: JobId) -> whisper_core::ports::AutomaticFinalImport {
        let generation = Generation::first();
        let mut request = test_request(job_id, generation);
        request.source_path = format!("capture-{job_id:?}.wav");
        whisper_core::ports::AutomaticFinalImport {
            request,
            outputs: whisper_core::ports::AutomaticFinalOutputPaths {
                txt_path: format!("capture-{job_id:?}-auto.txt"),
                srt_path: format!("capture-{job_id:?}-auto.srt"),
                companion_json_path: format!("capture-{job_id:?}.json"),
            },
            gpu_candidate: None,
        }
    }

    fn wait_for_event(
        io: &mut AsyncImportIo,
        phase: &str,
        timeout: Duration,
        predicate: impl Fn(&ImportEvent) -> bool,
    ) -> ImportEvent {
        let started = Instant::now();
        let deadline = started + timeout;
        loop {
            assert!(
                Instant::now() < deadline,
                "expected runtime event timed out in phase `{phase}`; elapsed={:?}",
                started.elapsed()
            );
            match io.poll().unwrap() {
                Some(event) if predicate(&event) => return event,
                Some(ImportEvent::Queue(_) | ImportEvent::ReplacementCleanups(_)) => {}
                Some(event) => panic!("unexpected runtime event: {event:?}"),
                None => thread::sleep(Duration::from_millis(5)),
            }
        }
    }

    fn direct_live_request(job_id: JobId) -> LiveRequest {
        LiveRequest {
            job_id,
            generation: Generation::first(),
            model_path: "model.nemo".into(),
            model_sha256: [0; 32],
            destination: "live-archive".into(),
            group_offset_samples: 0,
            auto_stop_after_speech_samples: None,
            microphone: Default::default(),
            microphone_gain_db: 0,
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Gpu,
            },
        }
    }

    fn write_test_wav_header(file: &mut fs::File, samples: u32) -> io::Result<()> {
        use std::io::{Seek, SeekFrom};

        let data_bytes = samples.saturating_mul(2);
        file.seek(SeekFrom::Start(0))?;
        file.write_all(b"RIFF")?;
        file.write_all(&(36_u32.saturating_add(data_bytes)).to_le_bytes())?;
        file.write_all(b"WAVEfmt ")?;
        file.write_all(&16_u32.to_le_bytes())?;
        file.write_all(&1_u16.to_le_bytes())?;
        file.write_all(&1_u16.to_le_bytes())?;
        file.write_all(&16_000_u32.to_le_bytes())?;
        file.write_all(&32_000_u32.to_le_bytes())?;
        file.write_all(&2_u16.to_le_bytes())?;
        file.write_all(&16_u16.to_le_bytes())?;
        file.write_all(b"data")?;
        file.write_all(&data_bytes.to_le_bytes())?;
        file.seek(SeekFrom::Start(u64::from(data_bytes) + 44))?;
        file.flush()
    }

    struct IncrementalWavCapture {
        path: PathBuf,
        file: Option<fs::File>,
        identity: Option<(JobId, Generation)>,
        samples: u32,
    }

    impl ServiceRuntime for IncrementalWavCapture {
        fn apply_effect(&mut self, effect: ImportEffect, pending: &mut VecDeque<ImportEvent>) {
            match effect {
                ImportEffect::StartDirectLive(request) => {
                    if let Some(parent) = self.path.parent() {
                        fs::create_dir_all(parent).unwrap();
                    }
                    let mut file = fs::OpenOptions::new()
                        .create_new(true)
                        .write(true)
                        .read(true)
                        .open(&self.path)
                        .unwrap();
                    write_test_wav_header(&mut file, 0).unwrap();
                    self.file = Some(file);
                    self.identity = Some((request.job_id, request.generation));
                }
                ImportEffect::Stop { job_id, generation }
                    if self.identity == Some((job_id, generation)) =>
                {
                    if let Some(mut file) = self.file.take() {
                        write_test_wav_header(&mut file, self.samples).unwrap();
                        file.sync_all().unwrap();
                    }
                    self.identity = None;
                    pending.push_back(ImportEvent::Stopped { job_id, generation });
                }
                _ => {}
            }
        }

        fn poll_event(&mut self) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
            None
        }

        fn pump_realtime(&mut self) -> Option<(JobId, Generation, String)> {
            let file = self.file.as_mut()?;
            let mut block = [0_i16; 160];
            for sample in &mut block {
                *sample = (self.samples & 0x7fff) as i16;
                self.samples = self.samples.saturating_add(1);
            }
            for sample in block {
                if let Err(error) = file.write_all(&sample.to_le_bytes()) {
                    let identity = self.identity.take();
                    self.file = None;
                    return identity.map(|(job_id, generation)| {
                        (
                            job_id,
                            generation,
                            format!("test WAV write failed: {error}"),
                        )
                    });
                }
            }
            let _ = file.flush();
            None
        }
    }

    struct SampleDrivenWavCapture {
        path: PathBuf,
        file: Option<fs::File>,
        identity: Option<(JobId, Generation)>,
        samples_rx: Receiver<(Vec<i16>, SyncSender<Result<u64, String>>)>,
        started_tx: SyncSender<(JobId, Generation)>,
        samples: Vec<i16>,
    }

    impl ServiceRuntime for SampleDrivenWavCapture {
        fn apply_effect(&mut self, effect: ImportEffect, pending: &mut VecDeque<ImportEvent>) {
            match effect {
                ImportEffect::StartDirectLive(request) => {
                    if let Some(parent) = self.path.parent() {
                        fs::create_dir_all(parent).unwrap();
                    }
                    let mut file = fs::OpenOptions::new()
                        .create_new(true)
                        .write(true)
                        .read(true)
                        .open(&self.path)
                        .unwrap();
                    write_test_wav_header(&mut file, 0).unwrap();
                    self.file = Some(file);
                    self.identity = Some((request.job_id, request.generation));
                    self.started_tx
                        .send((request.job_id, request.generation))
                        .unwrap();
                }
                ImportEffect::Stop { job_id, generation }
                    if self.identity == Some((job_id, generation)) =>
                {
                    if let Some(mut file) = self.file.take() {
                        write_test_wav_header(&mut file, self.samples.len() as u32).unwrap();
                        file.sync_all().unwrap();
                    }
                    self.identity = None;
                    pending.push_back(ImportEvent::Stopped { job_id, generation });
                }
                _ => {}
            }
        }

        fn poll_event(&mut self) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
            None
        }

        fn pump_realtime(&mut self) -> Option<(JobId, Generation, String)> {
            let (block, acknowledgement) = match self.samples_rx.try_recv() {
                Ok(block) => block,
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => return None,
            };
            let Some(file) = self.file.as_mut() else {
                let _ = acknowledgement.send(Err("capture is not active".into()));
                return None;
            };
            let write_result = block
                .iter()
                .try_for_each(|sample| file.write_all(&sample.to_le_bytes()))
                .and_then(|()| file.flush());
            if let Err(error) = write_result {
                let identity = self.identity.take();
                self.file = None;
                let message = format!("test WAV write failed: {error}");
                let _ = acknowledgement.send(Err(message.clone()));
                return identity.map(|(job_id, generation)| (job_id, generation, message));
            }
            self.samples.extend_from_slice(&block);
            let _ = acknowledgement.send(Ok(self.samples.len() as u64));
            None
        }
    }

    struct RuntimeAutomaticPublisher {
        runtime: ImportRuntime,
        gates: Arc<Mutex<HashMap<(JobId, u64), Arc<PublicationGate>>>>,
        started_tx: SyncSender<(JobId, Generation)>,
        publication_ready_tx: SyncSender<Result<(JobId, Generation), String>>,
        release_rx: Receiver<()>,
        model_path: PathBuf,
        source_samples: Vec<i16>,
    }

    impl ServiceRuntime for RuntimeAutomaticPublisher {
        fn apply_effect(&mut self, effect: ImportEffect, pending: &mut VecDeque<ImportEvent>) {
            let ImportEffect::StartAutomaticFinalTranscription(mut automatic) = effect else {
                return;
            };
            let identity = (automatic.request.job_id, automatic.request.generation.get());
            if let Some(gate) = self
                .gates
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .get(&identity)
                .cloned()
            {
                self.runtime.publication_gate = gate;
            }
            let mut publication_ready = false;
            let publish_result = (|| {
                self.started_tx
                    .send((automatic.request.job_id, automatic.request.generation))
                    .map_err(|_| "test runtime start acknowledgement disconnected".to_owned())?;
                write_fixture_wav(
                    Path::new(&automatic.request.source_path),
                    &self.source_samples,
                )
                .map_err(|error| error.to_string())?;
                automatic.request.model_path = self.model_path.to_string_lossy().into_owned();
                automatic.request.destination = self
                    .runtime
                    .archive
                    .root_path()
                    .to_string_lossy()
                    .into_owned();
                automatic.request.model_sha256 =
                    crate::model_catalog::cached_model_sha256(&self.model_path)
                        .map_err(|error| error.to_string())?
                        .1;
                let prepared = self
                    .runtime
                    .archive
                    .prepare(automatic.request.clone())
                    .map_err(|error| error.to_string())?;
                automatic.request = prepared;
                let segment = whisper_core::ports::WorkerSegment {
                    job_id: automatic.request.job_id,
                    generation: automatic.request.generation,
                    instance_id: 1,
                    segment_id: whisper_core::SegmentId(1),
                    range: whisper_core::SourceRange::new(
                        0,
                        self.source_samples.len() as u64,
                        16_000,
                    )
                    .ok_or_else(|| "test source range is empty".to_owned())?,
                    text: format!("runtime transcript {}", automatic.request.job_id.0),
                };
                self.runtime
                    .archive
                    .persist_segment(&segment)
                    .map_err(|error| error.to_string())?;
                fs::write(
                    &automatic.outputs.companion_json_path,
                    r#"{"confirmed_intervals":[],"unresolved_intervals":[]}"#,
                )
                .map_err(|error| error.to_string())?;
                self.runtime
                    .automatic_final_outputs
                    .insert(identity, automatic.clone());
                self.publication_ready_tx
                    .send(Ok((automatic.request.job_id, automatic.request.generation)))
                    .map_err(|_| {
                        "test publication-ready acknowledgement disconnected".to_owned()
                    })?;
                publication_ready = true;
                self.release_rx
                    .recv_timeout(Duration::from_secs(120))
                    .map_err(|_| {
                        "test publication release timed out after preparation".to_owned()
                    })?;
                self.runtime
                    .publish(
                        automatic.request.job_id,
                        automatic.request.generation,
                        1,
                        pending,
                    )
                    .map_err(|(_, _, error)| error)
            })();

            if let Err(message) = publish_result {
                if !publication_ready {
                    let _ = self.publication_ready_tx.send(Err(message.clone()));
                }
                pending.push_back(ImportEvent::Failed {
                    job_id: identity.0,
                    generation: automatic.request.generation,
                    message,
                });
            }
        }

        fn poll_event(&mut self) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
            None
        }
    }

    fn write_fixture_wav(path: &Path, samples: &[i16]) -> io::Result<()> {
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .read(true)
            .open(path)?;
        write_test_wav_header(&mut file, u32::try_from(samples.len()).unwrap())?;
        for sample in samples {
            file.write_all(&sample.to_le_bytes())?;
        }
        file.sync_all()
    }

    struct SlowAutomaticFinal {
        delay: Duration,
    }

    impl ServiceRuntime for SlowAutomaticFinal {
        fn apply_effect(&mut self, effect: ImportEffect, pending: &mut VecDeque<ImportEvent>) {
            if let ImportEffect::StartAutomaticFinalTranscription(automatic) = effect {
                // Models a slow source-copy/hash preparation on the final-import lane.
                thread::sleep(self.delay);
                pending.push_back(ImportEvent::Failed {
                    job_id: automatic.request.job_id,
                    generation: automatic.request.generation,
                    message: "simulated final import failure after slow preparation".into(),
                });
            }
        }

        fn poll_event(&mut self) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
            None
        }
    }

    struct BlockedAutomaticQueueInspection {
        inspection_started: mpsc::SyncSender<()>,
        inspection_finished: mpsc::SyncSender<()>,
        release_inspection: Receiver<()>,
    }

    impl ServiceRuntime for BlockedAutomaticQueueInspection {
        fn apply_effect(&mut self, effect: ImportEffect, pending: &mut VecDeque<ImportEvent>) {
            let ImportEffect::StartAutomaticFinalTranscription(automatic) = effect else {
                return;
            };
            let (job_id, generation) = (automatic.request.job_id, automatic.request.generation);
            let inspection = inspect_automatic_final_queue_with(
                Path::new(&automatic.request.destination),
                |destination| {
                    self.inspection_started.send(()).unwrap();
                    self.release_inspection.recv().unwrap();
                    crate::queue_store::QueueStore::inspect_read_only(destination)
                },
            );
            self.inspection_finished.send(()).unwrap();
            let message = match inspection {
                Ok(_) => "simulated final import failure after queue inspection".to_owned(),
                Err(error) => error,
            };
            pending.push_back(ImportEvent::Failed {
                job_id,
                generation,
                message,
            });
        }

        fn poll_event(&mut self) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
            None
        }
    }

    #[test]
    fn live_slow_automatic_final_failure_does_not_block_another_capture_wav() {
        let root = automatic_final_test_root("r5-slow-final");
        fs::create_dir_all(&root).unwrap();
        let wav_path = root.join("capture-b.wav");
        let main_gate = Arc::new(PublicationGate::default());
        let (
            mut io,
            effects,
            stops,
            app_events,
            automatic_effects,
            automatic_stops,
            automatic_events,
        ) = AsyncImportIo::for_test(main_gate.clone());
        let main_thread = thread::spawn({
            let wav_path = wav_path.clone();
            move || {
                service_loop(
                    effects,
                    stops,
                    app_events,
                    IncrementalWavCapture {
                        path: wav_path,
                        file: None,
                        identity: None,
                        samples: 0,
                    },
                )
            }
        });
        let automatic_thread = thread::spawn(move || {
            service_loop(
                automatic_effects,
                automatic_stops,
                automatic_events,
                SlowAutomaticFinal {
                    delay: Duration::from_millis(2_150),
                },
            )
        });

        let import_a = automatic_final_request(JobId(0xA4));
        io.submit(ImportEffect::StartAutomaticFinalTranscription(
            import_a.clone(),
        ))
        .unwrap();
        let started_at = Instant::now();
        thread::sleep(Duration::from_millis(100));
        let capture_b = JobId(0xB3);
        io.submit(ImportEffect::StartDirectLive(direct_live_request(
            capture_b,
        )))
        .unwrap();

        let deadline = Instant::now() + Duration::from_millis(700);
        while Instant::now() < deadline
            && fs::metadata(&wav_path).map_or(true, |metadata| metadata.len() <= 44)
        {
            thread::sleep(Duration::from_millis(10));
        }
        let wav_size_during_import = fs::metadata(&wav_path).unwrap().len();
        assert!(wav_size_during_import > 44, "capture B WAV did not advance");

        let deadline = Instant::now() + Duration::from_secs(4);
        let mut final_failure_at = None;
        loop {
            assert!(
                Instant::now() < deadline,
                "slow final import did not finish"
            );
            match io.poll().unwrap() {
                Some(ImportEvent::Failed { job_id, .. }) if job_id == import_a.request.job_id => {
                    final_failure_at = Some(Instant::now());
                    break;
                }
                Some(other) => panic!("unexpected event during R5 simulation: {other:?}"),
                None => thread::sleep(Duration::from_millis(5)),
            }
        }
        let final_failure_at = final_failure_at.unwrap();
        assert!(final_failure_at.duration_since(started_at) >= Duration::from_millis(2_000));
        let wav_size_after_import = fs::metadata(&wav_path).unwrap().len();
        assert!(
            wav_size_after_import > wav_size_during_import,
            "capture B WAV stopped advancing while final import A was active"
        );
        assert_eq!(io.active_identity, Some((capture_b, Generation::first())));
        assert_eq!(main_gate.0.load(Ordering::Acquire), GATE_ACTIVE);

        io.submit(ImportEffect::Stop {
            job_id: capture_b,
            generation: Generation::first(),
        })
        .unwrap();
        let stop_deadline = Instant::now() + Duration::from_secs(1);
        loop {
            assert!(Instant::now() < stop_deadline, "capture B did not stop");
            match io.poll().unwrap() {
                Some(ImportEvent::Stopped { job_id, .. }) if job_id == capture_b => break,
                Some(other) => panic!("unexpected event while stopping B: {other:?}"),
                None => thread::sleep(Duration::from_millis(5)),
            }
        }

        let bytes = fs::read(&wav_path).unwrap();
        assert_eq!(&bytes[..4], b"RIFF");
        assert_eq!(
            u32::from_le_bytes(bytes[40..44].try_into().unwrap()) as usize,
            bytes.len() - 44
        );
        assert!(
            bytes.len() > 44,
            "finalized WAV contains no captured samples"
        );

        drop(io);
        main_thread.join().unwrap();
        automatic_thread.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn live_blocked_automatic_queue_inspection_does_not_block_capture_b_stop_or_wav() {
        let root = automatic_final_test_root("r5-blocked-queue");
        fs::create_dir_all(&root).unwrap();
        let wav_path = root.join("capture-b.wav");
        let gate = Arc::new(PublicationGate::default());
        let (
            mut io,
            effects,
            stops,
            app_events,
            automatic_effects,
            automatic_stops,
            automatic_events,
        ) = AsyncImportIo::for_test(gate.clone());
        let (samples_tx, samples_rx) = mpsc::sync_channel(3);
        let (capture_started_tx, capture_started_rx) = mpsc::sync_channel(1);
        let main_thread = thread::spawn({
            let wav_path = wav_path.clone();
            move || {
                service_loop(
                    effects,
                    stops,
                    app_events,
                    SampleDrivenWavCapture {
                        path: wav_path,
                        file: None,
                        identity: None,
                        samples_rx,
                        started_tx: capture_started_tx,
                        samples: Vec::new(),
                    },
                )
            }
        });
        let capture_b = JobId(0xB5);
        io.submit(ImportEffect::StartDirectLive(direct_live_request(
            capture_b,
        )))
        .unwrap();
        assert_eq!(
            capture_started_rx
                .recv_timeout(Duration::from_secs(1))
                .unwrap(),
            (capture_b, Generation::first())
        );
        let mut acknowledged_samples = 0_u64;
        let mut inject = |samples: Vec<i16>| {
            let block_samples = u64::try_from(samples.len()).unwrap();
            let expected_cumulative = acknowledged_samples + block_samples;
            let (ack_tx, ack_rx) = mpsc::sync_channel(1);
            samples_tx.send((samples.clone(), ack_tx)).unwrap();
            let actual_cumulative = ack_rx
                .recv_timeout(Duration::from_secs(1))
                .unwrap()
                .unwrap();
            assert_eq!(actual_cumulative, expected_cumulative);
            assert!(actual_cumulative > acknowledged_samples);
            acknowledged_samples = actual_cumulative;
            actual_cumulative
        };
        let before_inspection = vec![101, 102, 103, 104];
        let acknowledged_before = inject(before_inspection.clone());
        assert_eq!(acknowledged_before, before_inspection.len() as u64);
        assert_eq!(acknowledged_before, 4);

        let (inspection_started_tx, inspection_started_rx) = mpsc::sync_channel(1);
        let (inspection_finished_tx, inspection_finished_rx) = mpsc::sync_channel(1);
        let (release_inspection_tx, release_inspection_rx) = mpsc::sync_channel(1);
        let automatic_thread = thread::spawn(move || {
            service_loop(
                automatic_effects,
                automatic_stops,
                automatic_events,
                BlockedAutomaticQueueInspection {
                    inspection_started: inspection_started_tx,
                    inspection_finished: inspection_finished_tx,
                    release_inspection: release_inspection_rx,
                },
            )
        });

        let mut import_a = automatic_final_request(JobId(0xA5));
        import_a.request.destination = root.to_string_lossy().into_owned();
        io.submit(ImportEffect::StartAutomaticFinalTranscription(
            import_a.clone(),
        ))
        .unwrap();
        inspection_started_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("automatic lane did not enter queue inspection");
        let during_inspection = vec![-201, -202, -203];
        let acknowledged_during = inject(during_inspection.clone());
        assert_eq!(
            acknowledged_during,
            (before_inspection.len() + during_inspection.len()) as u64
        );
        assert_eq!(acknowledged_during, 7);
        assert_eq!(
            inspection_finished_rx.recv_timeout(Duration::from_millis(2_100)),
            Err(mpsc::RecvTimeoutError::Timeout),
            "queue inspection unexpectedly finished before the safety interval"
        );
        let mut samples_while_inspection_blocked = before_inspection.clone();
        samples_while_inspection_blocked.extend(during_inspection.clone());
        let expected_blocked_payload = samples_while_inspection_blocked
            .iter()
            .flat_map(|sample| sample.to_le_bytes())
            .collect::<Vec<_>>();
        let wav_during_block = fs::read(&wav_path).unwrap();
        assert!(
            wav_during_block.len() >= 44,
            "B WAV header missing during A inspection"
        );
        assert_eq!(&wav_during_block[44..], expected_blocked_payload);
        assert_eq!(
            io.active_identity,
            Some((capture_b, Generation::first())),
            "B must remain active while A's queue inspection is blocked"
        );

        release_inspection_tx.send(()).unwrap();
        inspection_finished_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("queue inspection did not finish after its explicit release");
        let after_resume = vec![301, 302, 303, 304, 305];
        let acknowledged_after = inject(after_resume.clone());
        let expected_samples_total =
            before_inspection.len() + during_inspection.len() + after_resume.len();
        assert_eq!(acknowledged_after, expected_samples_total as u64);
        assert_eq!(acknowledged_after, 12);
        let mut expected_samples = before_inspection;
        expected_samples.extend(during_inspection);
        expected_samples.extend(after_resume);

        let failure_deadline = Instant::now() + Duration::from_secs(1);
        loop {
            assert!(
                Instant::now() < failure_deadline,
                "A's terminal result was not routed"
            );
            match io.poll().unwrap() {
                Some(ImportEvent::Failed {
                    job_id,
                    generation,
                    message,
                    ..
                }) if job_id == import_a.request.job_id
                    && generation == import_a.request.generation
                    && message.contains("after queue inspection") =>
                {
                    break;
                }
                Some(other) => panic!("unexpected post-inspection event: {other:?}"),
                None => thread::sleep(Duration::from_millis(5)),
            }
        }
        assert_eq!(
            io.active_identity,
            Some((capture_b, Generation::first())),
            "A's delayed queue error must remain associated with A while B is active"
        );

        let stop_started = Instant::now();
        io.submit(ImportEffect::Stop {
            job_id: capture_b,
            generation: Generation::first(),
        })
        .unwrap();
        let stopped_b = wait_for_event(
            &mut io,
            "R5 capture B stop after A inspection resumes",
            Duration::from_millis(1_500),
            |event| {
                matches!(event, ImportEvent::Stopped { job_id, generation }
                    if *job_id == capture_b && *generation == Generation::first())
            },
        );
        assert!(matches!(
            stopped_b,
            ImportEvent::Stopped { job_id, generation }
                if job_id == capture_b && generation == Generation::first()
        ));
        assert!(stop_started.elapsed() < Duration::from_secs(2));
        assert_eq!(gate.0.load(Ordering::Acquire), GATE_CANCELLED);

        let bytes = fs::read(&wav_path).unwrap();
        assert!(bytes.len() >= 44, "finalized B WAV header is missing");
        assert_eq!(&bytes[..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(&bytes[36..40], b"data");
        assert_eq!(
            u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize,
            bytes.len() - 8,
            "finalized B RIFF size must match the complete file"
        );
        let data_size = u32::from_le_bytes(bytes[40..44].try_into().unwrap()) as usize;
        assert_eq!(
            data_size,
            bytes.len() - 44,
            "finalized B data size must match the complete PCM payload"
        );
        assert_eq!(
            data_size % 2,
            0,
            "finalized B PCM16 payload must contain whole samples"
        );
        let actual_samples = bytes[44..]
            .chunks_exact(2)
            .map(|sample| i16::from_le_bytes([sample[0], sample[1]]))
            .collect::<Vec<_>>();
        assert_eq!(actual_samples.len(), expected_samples_total);
        assert_eq!(
            actual_samples, expected_samples,
            "finalized B WAV must preserve exact PCM values and order before, during, and after A's inspection"
        );

        drop(io);
        main_thread.join().unwrap();
        automatic_thread.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn live_automatic_final_queue_corruption_is_reported_explicitly() {
        let root = automatic_final_test_root("r5-corrupt-queue");
        let queue_dir = root.join("transcriptions");
        fs::create_dir_all(&queue_dir).unwrap();
        fs::write(queue_dir.join("queue.jsonl"), b"{ truncated\n").unwrap();

        let error = inspect_automatic_final_queue(&root).unwrap_err();
        assert!(error.starts_with("AutomaticFinalQueueUnavailable: StorageCorrupt:"));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn live_automatic_final_gate_does_not_block_next_session_stop_or_late_publish() {
        let root = automatic_final_test_root("r4-real-publish-isolation");
        fs::create_dir_all(&root).unwrap();
        let archive_root = root.join("archive");
        let capture_b_wav = root.join("capture-b.wav");
        let capture_b_companion = root.join("capture-b.json");
        fs::write(&capture_b_companion, "B companion unchanged").unwrap();
        let capture_b_companion_before = fs::read(&capture_b_companion).unwrap();

        let live_gate = Arc::new(PublicationGate::default());
        let (mut io, effects, stops, app_events, auto_effects, auto_stops, auto_events) =
            AsyncImportIo::for_test(live_gate.clone());
        let (samples_tx, samples_rx) = mpsc::sync_channel(4);
        let (capture_started_tx, capture_started_rx) = mpsc::sync_channel(1);
        let capture_thread = thread::spawn({
            let capture_b_wav = capture_b_wav.clone();
            move || {
                service_loop(
                    effects,
                    stops,
                    app_events,
                    SampleDrivenWavCapture {
                        path: capture_b_wav,
                        file: None,
                        identity: None,
                        samples_rx,
                        started_tx: capture_started_tx,
                        samples: Vec::new(),
                    },
                )
            }
        });

        let capture_b = JobId(0xB1);
        io.submit(ImportEffect::StartDirectLive(direct_live_request(
            capture_b,
        )))
        .unwrap();
        assert_eq!(
            capture_started_rx
                .recv_timeout(Duration::from_secs(1))
                .unwrap(),
            (capture_b, Generation::first())
        );

        let mut acknowledged_samples = 0_u64;
        let mut inject = |samples: Vec<i16>| {
            let block_samples = u64::try_from(samples.len()).unwrap();
            let expected_cumulative = acknowledged_samples + block_samples;
            let (ack_tx, ack_rx) = mpsc::sync_channel(1);
            samples_tx.send((samples.clone(), ack_tx)).unwrap();
            let actual_cumulative = ack_rx
                .recv_timeout(Duration::from_secs(1))
                .unwrap()
                .unwrap();
            assert_eq!(actual_cumulative, expected_cumulative);
            assert!(actual_cumulative > acknowledged_samples);
            acknowledged_samples = actual_cumulative;
        };
        let before_publish = vec![11, 12, 13];
        inject(before_publish.clone());

        let repository_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let model_path = repository_root.join("models/ggml-large-v3-turbo.bin");
        assert!(model_path.is_file(), "registered Turbo fixture is required");
        let gates = io.automatic_publication_gates.clone();
        let workers = WorkerPaths {
            cpu: root.join("unused-cpu.exe"),
            gpu: root.join("unused-gpu.exe"),
        };
        let runtime_gates = gates.clone();
        let runtime_archive_root = archive_root.clone();
        let (publish_started_tx, publish_started_rx) = mpsc::sync_channel(2);
        let (publication_ready_tx, publication_ready_rx) = mpsc::sync_channel(2);
        let (release_publish_tx, release_publish_rx) = mpsc::sync_channel(2);
        let auto_thread = thread::spawn(move || {
            let mut runtime = ImportRuntime::new(workers, Arc::new(PublicationGate::default()));
            runtime.archive = ArchiveStore::new(&runtime_archive_root);
            runtime.automatic_publication_gates = Some(runtime_gates);
            service_loop(
                auto_effects,
                auto_stops,
                auto_events,
                RuntimeAutomaticPublisher {
                    runtime,
                    gates,
                    started_tx: publish_started_tx,
                    publication_ready_tx,
                    release_rx: release_publish_rx,
                    model_path,
                    source_samples: vec![7, 8, 9, 10],
                },
            )
        });

        let mut import_a = automatic_final_request(JobId(0xA1));
        import_a.request.source_path = root.join("source-a.wav").to_string_lossy().into_owned();
        import_a.request.destination = archive_root.to_string_lossy().into_owned();
        import_a.outputs = whisper_core::ports::AutomaticFinalOutputPaths {
            txt_path: root
                .join("capture-a-auto.txt")
                .to_string_lossy()
                .into_owned(),
            srt_path: root
                .join("capture-a-auto.srt")
                .to_string_lossy()
                .into_owned(),
            companion_json_path: root.join("capture-a.json").to_string_lossy().into_owned(),
        };
        let preparation_started = Instant::now();
        io.submit(ImportEffect::StartAutomaticFinalTranscription(
            import_a.clone(),
        ))
        .unwrap();
        assert_eq!(
            publish_started_rx
                .recv_timeout(Duration::from_secs(30))
                .unwrap(),
            (import_a.request.job_id, import_a.request.generation)
        );
        assert_eq!(
            io.active_identity,
            Some((capture_b, Generation::first())),
            "capture B must remain active while the real runtime publication is pending"
        );
        let while_publishing = vec![-21, -22, -23, -24];
        inject(while_publishing.clone());
        let prepared_identity = publication_ready_rx
            .recv_timeout(Duration::from_secs(120))
            .unwrap_or_else(|error| {
                panic!(
                    "R4 phase `A archive preparation`; elapsed={:?}; result={error}",
                    preparation_started.elapsed()
                )
            })
            .unwrap_or_else(|error| {
                panic!(
                    "R4 phase `A archive preparation`; elapsed={:?}; error={error}",
                    preparation_started.elapsed()
                )
            });
        assert_eq!(
            prepared_identity,
            (import_a.request.job_id, import_a.request.generation)
        );
        let publication_started = Instant::now();
        release_publish_tx.send(()).unwrap();

        let published_a = wait_for_event(
            &mut io,
            "A final publication",
            Duration::from_secs(30),
            |event| {
                matches!(event, ImportEvent::Published { job_id, generation }
                    if *job_id == import_a.request.job_id && *generation == import_a.request.generation)
            },
        );
        assert!(
            publication_started.elapsed() < Duration::from_secs(30),
            "R4 phase `A final publication`; elapsed={:?}",
            publication_started.elapsed()
        );
        assert!(
            matches!(published_a, ImportEvent::Published { job_id, .. } if job_id == import_a.request.job_id)
        );
        let cleanup_wait_started = Instant::now();
        let cleanup_deadline = cleanup_wait_started + Duration::from_secs(5);
        let cleanup_event = loop {
            assert!(
                Instant::now() < cleanup_deadline,
                "R4 phase `A replacement-cleanup event after Published`; elapsed={:?}; event absent",
                cleanup_wait_started.elapsed()
            );
            match io.poll().unwrap() {
                Some(ImportEvent::ReplacementCleanups(cleanups)) => break cleanups,
                Some(ImportEvent::Queue(_)) => {}
                Some(event) => panic!(
                    "R4 phase `A replacement-cleanup event after Published`; elapsed={:?}; unexpected event: {event:?}",
                    cleanup_wait_started.elapsed()
                ),
                None => thread::sleep(Duration::from_millis(5)),
            }
        };
        let expected_cleanup_successor = whisper_core::ports::TreatmentIdentity {
            destination: archive_root
                .canonicalize()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            job_id: import_a.request.job_id,
            generation: import_a.request.generation,
        };
        assert!(
            cleanup_event
                .iter()
                .all(|cleanup| cleanup.successor == expected_cleanup_successor),
            "R4 phase `A replacement-cleanup identity`; expected successor {expected_cleanup_successor:?}, got {cleanup_event:?}"
        );
        // `publish` queues Published immediately before ReplacementCleanups in the same
        // runtime. The cleanups payload may be empty; this test has no other publisher.
        let expected_a_text = format!("runtime transcript {}", import_a.request.job_id.0);
        assert_eq!(
            fs::read_to_string(&import_a.outputs.txt_path).unwrap(),
            expected_a_text
        );
        assert!(
            fs::read_to_string(&import_a.outputs.srt_path)
                .unwrap()
                .contains(&expected_a_text)
        );
        assert_eq!(
            io.active_identity,
            Some((capture_b, Generation::first())),
            "publication A must leave capture B identity unchanged"
        );
        assert_eq!(
            fs::read(&capture_b_companion).unwrap(),
            capture_b_companion_before,
            "publication A must leave B's companion untouched"
        );
        assert!(!root.join("capture-b-auto.txt").exists());
        assert!(!root.join("capture-b-auto.srt").exists());

        let mut failing_a = automatic_final_request(JobId(0xA2));
        failing_a.request.source_path = root
            .join("source-a-failing.wav")
            .to_string_lossy()
            .into_owned();
        failing_a.request.destination = archive_root.to_string_lossy().into_owned();
        failing_a.outputs = whisper_core::ports::AutomaticFinalOutputPaths {
            txt_path: root
                .join("capture-a-failing-auto.txt")
                .to_string_lossy()
                .into_owned(),
            srt_path: root
                .join("capture-a-failing-auto.srt")
                .to_string_lossy()
                .into_owned(),
            companion_json_path: root
                .join("capture-a-failing.json")
                .to_string_lossy()
                .into_owned(),
        };
        let sentinel = b"preserve A collision";
        fs::write(&failing_a.outputs.txt_path, sentinel).unwrap();
        let failure_preparation_started = Instant::now();
        io.submit(ImportEffect::StartAutomaticFinalTranscription(
            failing_a.clone(),
        ))
        .unwrap();
        assert_eq!(
            publish_started_rx
                .recv_timeout(Duration::from_secs(30))
                .unwrap(),
            (failing_a.request.job_id, failing_a.request.generation)
        );
        assert_eq!(
            io.active_identity,
            Some((capture_b, Generation::first())),
            "capture B remains active during A's failing publication"
        );
        let prepared_failure_identity = publication_ready_rx
            .recv_timeout(Duration::from_secs(120))
            .unwrap_or_else(|error| {
                panic!(
                    "R4 phase `A failing archive preparation`; elapsed={:?}; result={error}",
                    failure_preparation_started.elapsed()
                )
            })
            .unwrap_or_else(|error| {
                panic!(
                    "R4 phase `A failing archive preparation`; elapsed={:?}; error={error}",
                    failure_preparation_started.elapsed()
                )
            });
        assert_eq!(
            prepared_failure_identity,
            (failing_a.request.job_id, failing_a.request.generation)
        );
        let failure_publication_started = Instant::now();
        release_publish_tx.send(()).unwrap();
        let failed_a = wait_for_event(
            &mut io,
            "A failed final publication",
            Duration::from_secs(30),
            |event| {
                matches!(event, ImportEvent::Failed { job_id, generation, .. }
                    if *job_id == failing_a.request.job_id && *generation == failing_a.request.generation)
            },
        );
        assert!(
            failure_publication_started.elapsed() < Duration::from_secs(30),
            "R4 phase `A failed final publication`; elapsed={:?}",
            failure_publication_started.elapsed()
        );
        assert!(
            matches!(failed_a, ImportEvent::Failed { job_id, message, .. }
            if job_id == failing_a.request.job_id && message.contains("sortie automatique existe déjà"))
        );
        assert_eq!(fs::read(&failing_a.outputs.txt_path).unwrap(), sentinel);
        assert!(!Path::new(&failing_a.outputs.srt_path).exists());
        assert_eq!(
            io.active_identity,
            Some((capture_b, Generation::first())),
            "failure A must not change capture B identity"
        );
        assert_eq!(
            fs::read(&capture_b_companion).unwrap(),
            capture_b_companion_before,
            "failure A must not change B's files"
        );

        let after_publish = vec![31, 32, 33];
        inject(after_publish.clone());
        io.submit(ImportEffect::Stop {
            job_id: capture_b,
            generation: Generation::first(),
        })
        .unwrap();
        let stop_started = Instant::now();
        let stopped_b = wait_for_event(
            &mut io,
            "R4 capture B stop and WAV finalization",
            Duration::from_secs(5),
            |event| {
                matches!(event, ImportEvent::Stopped { job_id, generation }
                    if *job_id == capture_b && *generation == Generation::first())
            },
        );
        assert!(
            matches!(stopped_b, ImportEvent::Stopped { job_id, generation }
                if job_id == capture_b && generation == Generation::first()),
            "R4 expected Stopped for B generation 1; elapsed={:?}; event={stopped_b:?}",
            stop_started.elapsed()
        );
        let capture_b_samples = [before_publish, while_publishing, after_publish].concat();
        let wav = fs::read(&capture_b_wav).unwrap();
        assert!(wav.len() >= 44, "capture B WAV is shorter than its header");
        assert_eq!(&wav[..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[36..40], b"data");
        assert_eq!(
            u32::from_le_bytes(wav[4..8].try_into().unwrap()) as usize,
            wav.len() - 8,
            "capture B RIFF size must match the complete file"
        );
        let data_size = u32::from_le_bytes(wav[40..44].try_into().unwrap()) as usize;
        assert_eq!(
            data_size,
            wav.len() - 44,
            "capture B data size must match the complete PCM payload"
        );
        assert_eq!(data_size % 2, 0, "PCM16 payload must contain whole samples");
        let actual_samples = wav[44..]
            .chunks_exact(2)
            .map(|bytes| i16::from_le_bytes([bytes[0], bytes[1]]))
            .collect::<Vec<_>>();
        assert_eq!(
            capture_b_samples.len(),
            10,
            "test fixture must contain 10 samples"
        );
        assert_eq!(
            actual_samples, capture_b_samples,
            "capture B WAV must preserve every injected sample exactly and in order"
        );
        assert_eq!(
            live_gate.0.load(Ordering::Acquire),
            GATE_CANCELLED,
            "stopping B must cancel only B's capture publication gate"
        );

        drop(io);
        capture_thread.join().unwrap();
        auto_thread.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn live_automatic_final_failure_is_routed_without_changing_active_capture() {
        let root = automatic_final_test_root("r4-failed-publish-isolation");
        fs::create_dir_all(&root).unwrap();
        let source_txt = root.join("a-source.txt");
        let missing_source_srt = root.join("missing-source.srt");
        fs::write(&source_txt, "partial A input").unwrap();
        let live_gate = Arc::new(PublicationGate::default());
        let (mut io, live_effects, live_stops, _, auto_effects, _, auto_events) =
            AsyncImportIo::for_test(live_gate.clone());
        let mut import_a = automatic_final_request(JobId(0xA3));
        import_a.outputs = whisper_core::ports::AutomaticFinalOutputPaths {
            txt_path: root
                .join("capture-a-auto.txt")
                .to_string_lossy()
                .into_owned(),
            srt_path: root
                .join("capture-a-auto.srt")
                .to_string_lossy()
                .into_owned(),
            companion_json_path: root.join("capture-a.json").to_string_lossy().into_owned(),
        };
        fs::write(
            &import_a.outputs.companion_json_path,
            r#"{"confirmed_intervals":[],"unresolved_intervals":[]}"#,
        )
        .unwrap();
        io.submit(ImportEffect::StartAutomaticFinalTranscription(
            import_a.clone(),
        ))
        .unwrap();
        let _ = auto_effects.try_recv().unwrap();
        let capture_b = JobId(0xB2);
        io.submit(ImportEffect::StartDirectLive(direct_live_request(
            capture_b,
        )))
        .unwrap();
        let _ = live_effects.try_recv().unwrap();

        let publication_error =
            publish_automatic_final_exports(&source_txt, &missing_source_srt, &import_a)
                .unwrap_err();
        assert!(publication_error.contains("Export SRT automatique absent"));
        assert!(!Path::new(&import_a.outputs.txt_path).exists());
        assert!(!Path::new(&import_a.outputs.srt_path).exists());

        auto_events
            .send(ImportEvent::Failed {
                job_id: import_a.request.job_id,
                generation: import_a.request.generation,
                message: "simulated automatic-final publication failure".into(),
            })
            .unwrap();
        assert!(matches!(
            io.poll().unwrap(),
            Some(ImportEvent::Failed { job_id, message, .. })
                if job_id == import_a.request.job_id && message.contains("publication failure")
        ));
        assert_eq!(io.active_identity, Some((capture_b, Generation::first())));
        assert_eq!(live_gate.0.load(Ordering::Acquire), GATE_ACTIVE);

        io.submit(ImportEffect::Stop {
            job_id: capture_b,
            generation: Generation::first(),
        })
        .unwrap();
        assert!(
            matches!(live_stops.try_recv(), Ok(ImportEffect::Stop { job_id, .. }) if job_id == capture_b)
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn parakeet_live_automatic_final_exports_are_distinct_create_new_outputs() {
        let root = automatic_final_test_root("exports");
        fs::create_dir_all(&root).unwrap();
        let source_txt = root.join("archive.txt");
        let source_srt = root.join("archive.srt");
        fs::write(&source_txt, "auto non vérifié").unwrap();
        fs::write(&source_srt, "1\n00:00:00,000 --> 00:00:01,000\ntexte\n").unwrap();
        let outputs = whisper_core::ports::AutomaticFinalOutputPaths {
            txt_path: root
                .join("capture-auto-non-verifie.txt")
                .to_string_lossy()
                .into_owned(),
            srt_path: root
                .join("capture-auto-non-verifie.srt")
                .to_string_lossy()
                .into_owned(),
            companion_json_path: root.join("capture.json").to_string_lossy().into_owned(),
        };
        fs::write(
            &outputs.companion_json_path,
            r#"{"confirmed_intervals":[{"text":"direct"}],"unresolved_intervals":[{"status":"unresolved"}]}"#,
        )
        .unwrap();
        let automatic = whisper_core::ports::AutomaticFinalImport {
            request: ImportRequest {
                strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
                job_id: JobId(42),
                generation: Generation::first(),
                source_path: root.join("capture.wav").to_string_lossy().into_owned(),
                source_sha256: Some([1; 32]),
                source_samples: Some(16_000),
                model_path: "model.nemo".into(),
                model_sha256: [2; 32],
                destination: root.to_string_lossy().into_owned(),
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Gpu,
                },
            },
            outputs: outputs.clone(),
            gpu_candidate: None,
        };
        publish_automatic_final_exports(&source_txt, &source_srt, &automatic).unwrap();
        assert_eq!(
            fs::read_to_string(&outputs.txt_path).unwrap(),
            "auto non vérifié"
        );
        assert!(
            fs::read_to_string(&outputs.srt_path)
                .unwrap()
                .starts_with("1\n")
        );
        let manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(&outputs.companion_json_path).unwrap()).unwrap();
        assert_eq!(
            manifest["automatic_final_transcription"]["status"],
            "produced_unverified"
        );
        assert_eq!(
            manifest["automatic_final_transcription"]["model_sha256"],
            "02".repeat(32)
        );
        assert_eq!(manifest["confirmed_intervals"][0]["text"], "direct");
        assert_eq!(manifest["unresolved_intervals"][0]["status"], "unresolved");
        assert!(publish_automatic_final_exports(&source_txt, &source_srt, &automatic).is_err());
        assert_eq!(
            fs::read_to_string(&outputs.txt_path).unwrap(),
            "auto non vérifié"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn automatic_final_publication_failure_leaves_no_partial_public_name() {
        let root = automatic_final_test_root("rollback");
        fs::create_dir_all(&root).unwrap();
        let txt_stage = root.join("txt.stage");
        let missing_srt_stage = root.join("missing-srt.stage");
        let txt_path = root.join("capture-auto-non-verifie.txt");
        let srt_path = root.join("capture-auto-non-verifie.srt");
        fs::write(&txt_stage, "complete staged text").unwrap();
        let error =
            publish_automatic_staged_pair(&txt_stage, &missing_srt_stage, &txt_path, &srt_path)
                .unwrap_err();
        assert!(error.contains("Publication du SRT automatique impossible"));
        assert!(!txt_path.exists(), "TXT partiel publié malgré échec SRT");
        assert!(!srt_path.exists(), "SRT publié malgré échec de la paire");
        assert!(
            !txt_stage.exists(),
            "fichier de staging restant après rollback"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn automatic_final_source_read_failure_publishes_neither_output() {
        let root = automatic_final_test_root("read-failure");
        fs::create_dir_all(&root).unwrap();
        let source_txt = root.join("source.txt");
        let source_srt = root.join("missing.srt");
        let txt_path = root.join("capture-auto-non-verifie.txt");
        let srt_path = root.join("capture-auto-non-verifie.srt");
        fs::write(&source_txt, "source text").unwrap();
        let automatic = whisper_core::ports::AutomaticFinalImport {
            request: whisper_core::ports::ImportRequest {
                strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
                job_id: JobId(501),
                generation: Generation::first(),
                source_path: "capture.wav".into(),
                source_sha256: None,
                source_samples: None,
                model_path: "model.nemo".into(),
                model_sha256: [5; 32],
                destination: root.to_string_lossy().into_owned(),
                config: whisper_core::domain::JobConfig {
                    language: whisper_core::domain::LanguageChoice::Automatic,
                    compute: whisper_core::domain::ComputeChoice::Gpu,
                },
            },
            outputs: whisper_core::ports::AutomaticFinalOutputPaths {
                txt_path: txt_path.to_string_lossy().into_owned(),
                srt_path: srt_path.to_string_lossy().into_owned(),
                companion_json_path: root.join("capture.json").to_string_lossy().into_owned(),
            },
            gpu_candidate: None,
        };

        let error =
            publish_automatic_final_exports(&source_txt, &source_srt, &automatic).unwrap_err();

        assert!(error.contains("Export SRT automatique absent"));
        assert!(!txt_path.exists());
        assert!(!srt_path.exists());
        fs::remove_dir_all(root).unwrap();
    }

    fn reprocessing_candidate(
        job_id: u128,
        source_sha256: &str,
        success_ordinal: Option<u64>,
        active: bool,
        verified_present: bool,
    ) -> PublishedImport {
        PublishedImport {
            job_id: JobId(job_id),
            generation: Generation::first(),
            source_sha256: source_sha256.to_owned(),
            export_basename: None,
            active,
            success_ordinal,
            verified_present,
        }
    }

    fn reprocessing_test_root(name: &str) -> PathBuf {
        let base =
            PathBuf::from(std::env::var_os("PARAKEET_RTC02_EXEC_ROOT").expect("RTC-02 root"));
        let root = base
            .join("archive")
            .join(format!("{name}-{}", std::process::id()));
        let treatment = root.join("transcriptions/00000000000000000000000000000001/1");
        fs::create_dir_all(&treatment).unwrap();
        root
    }

    #[test]
    fn emptied_transcriptions_keeps_history_but_offers_no_duplicate_targets() {
        let root = reprocessing_test_root("empty-destination-history");
        let source_sha = "ab".repeat(32);
        let index_dir = root.join(".whisper-publications");
        fs::create_dir_all(&index_dir).unwrap();
        fs::write(
            index_dir.join("index.json"),
            serde_json::to_vec(&serde_json::json!({
                "version": 1,
                "records": [{
                    "job_id": 1,
                    "generation": 1,
                    "source_sha256": source_sha,
                    "export_basename": null,
                    "active": true,
                    "success_ordinal": 1
                }]
            }))
            .unwrap(),
        )
        .unwrap();
        fs::remove_dir_all(root.join("transcriptions")).unwrap();

        let historical = ArchiveStore::reprocessing_catalog(&root, &source_sha).unwrap();
        assert_eq!(historical.len(), 1, "durable history must remain intact");
        assert!(!historical[0].verified_present);
        assert!(verified_present_publications(historical).is_empty());
        assert!(
            !root.join("transcriptions").exists(),
            "catalog inspection must not recreate the user's emptied destination"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reprocessing_latest_requires_known_unique_order_and_all_includes_inactive_present() {
        let root = reprocessing_test_root("catalog-choice");
        let sha = "a".repeat(64);
        let catalog = vec![
            reprocessing_candidate(1, &sha, None, true, true),
            reprocessing_candidate(2, &sha, None, false, true),
            reprocessing_candidate(3, &sha, Some(3), false, false),
        ];
        let unknown = resolve_reprocessing_targets(
            &root,
            &catalog[..2],
            &DuplicateImportChoice::ReplaceLatest,
        );
        assert!(
            unknown
                .unwrap_err()
                .contains("ordre historique est inconnu")
        );

        let all = resolve_reprocessing_targets(&root, &catalog, &DuplicateImportChoice::ReplaceAll)
            .unwrap();
        assert_eq!(all.len(), 2);
        assert!(all.iter().any(|item| item.job_id == JobId(2)));
        assert!(!all.iter().any(|item| item.job_id == JobId(3)));

        let known_order = vec![
            reprocessing_candidate(1, &sha, Some(1), true, true),
            reprocessing_candidate(2, &sha, Some(2), false, true),
            reprocessing_candidate(3, &sha, Some(3), false, false),
        ];
        let latest = resolve_reprocessing_targets(
            &root,
            &known_order,
            &DuplicateImportChoice::ReplaceLatest,
        )
        .unwrap();
        assert_eq!(latest.len(), 1);
        assert_eq!(latest[0].job_id, JobId(2));
    }

    #[test]
    fn browse_accepts_only_exact_verified_treatment_in_current_destination() {
        let root = reprocessing_test_root("browse-choice");
        let sha = "b".repeat(64);
        let candidate = reprocessing_candidate(1, &sha, Some(1), true, true);
        let selected = root.join("transcriptions/00000000000000000000000000000001/1");
        let accepted = resolve_reprocessing_targets(
            &root,
            std::slice::from_ref(&candidate),
            &DuplicateImportChoice::ReplaceSelectedPath(selected.to_string_lossy().into_owned()),
        )
        .unwrap();
        assert_eq!(accepted.len(), 1);
        assert_eq!(accepted[0].job_id, JobId(1));

        let unrelated_audio = root.join("transcriptions/00000000000000000000000000000002/1");
        fs::create_dir_all(&unrelated_audio).unwrap();
        assert!(
            resolve_reprocessing_targets(
                &root,
                std::slice::from_ref(&candidate),
                &DuplicateImportChoice::ReplaceSelectedPath(
                    unrelated_audio.to_string_lossy().into_owned()
                ),
            )
            .is_err()
        );

        let tier = root.join("transcriptions/not-a-treatment");
        fs::create_dir_all(&tier).unwrap();
        assert!(
            resolve_reprocessing_targets(
                &root,
                std::slice::from_ref(&candidate),
                &DuplicateImportChoice::ReplaceSelectedPath(tier.to_string_lossy().into_owned()),
            )
            .is_err()
        );

        let other_destination = root.parent().unwrap().join("external-treatment");
        fs::create_dir_all(&other_destination).unwrap();
        assert!(
            resolve_reprocessing_targets(
                &root,
                &[candidate],
                &DuplicateImportChoice::ReplaceSelectedPath(
                    other_destination.to_string_lossy().into_owned()
                ),
            )
            .is_err()
        );
    }

    fn test_artifact_root(name: &str) -> PathBuf {
        let base = std::env::var_os("PARAKEET_R6_TEST_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("whisper-adapter-tests"));
        base.join(format!("{name}-{}", std::process::id()))
    }

    fn not_run_without_campaign_variables(variables: &[&str]) -> bool {
        let missing = variables
            .iter()
            .filter(|name| std::env::var_os(name).is_none())
            .copied()
            .collect::<Vec<_>>();
        if missing.is_empty() {
            return false;
        }
        eprintln!(
            "NOT RUN: missing campaign variable(s): {}",
            missing.join(", ")
        );
        true
    }

    #[test]
    fn timestamp_guard_accepts_exactly_one_tdt_frame_overlap() {
        assert_eq!(TDT_FRAME_OVERLAP_SAMPLES, 80 * 16);
        assert!(vad_word_overlap_within_one_tdt_frame(10_000, 11_280));
        assert!(!vad_word_overlap_within_one_tdt_frame(10_000, 11_281));
    }

    #[test]
    fn single_pass_adapter_accepts_normalized_words_and_rejects_any_overlap() {
        let normalized = [
            vad_test_word("faire?", 17_760, 17_920),
            vad_test_word("Je", 18_000, 18_080),
        ];
        assert_eq!(
            validate_single_pass_parakeet_words(&normalized, 27_000 * 16).unwrap(),
            vec![
                (17_760 * 16, 17_920 * 16, "faire?".into()),
                (18_000 * 16, 18_080 * 16, "Je".into()),
            ]
        );
        let overlapping = [
            vad_test_word("faire?", 17_760, 18_080),
            vad_test_word("Je", 18_000, 18_080),
        ];
        assert!(
            validate_single_pass_parakeet_words(&overlapping, 27_000 * 16)
                .unwrap_err()
                .contains("overlaps or is unordered")
        );
    }

    #[test]
    fn single_pass_punctuation_boundary_keeps_original_words_with_positive_ranges() {
        let words = [
            vad_test_word("faire?", 17_760, 17_920),
            vad_test_word("Je", 18_000, 18_080),
        ];
        let validated = validate_single_pass_parakeet_words(&words, 27_000 * 16).unwrap();
        let mut groups = Vec::new();
        for (start, end, token) in validated {
            append_single_pass_parakeet_word_group(&mut groups, start, end, &token).unwrap();
        }
        assert_eq!(groups.len(), 2);
        assert_eq!(
            groups[0],
            (17_760 * 16, 17_920 * 16, vec!["faire?".to_owned()])
        );
        assert_eq!(groups[1], (18_000 * 16, 18_080 * 16, vec!["Je".to_owned()]));
    }

    fn vad_test_window() -> whisper_core::parakeet_windows::Window {
        whisper_core::parakeet_windows::Window {
            index: 0,
            central_start_sample: 0,
            central_end_sample: 400_000,
            native_start_sample: 0,
            native_end_sample: 400_000,
            cut: None,
        }
    }

    fn vad_test_word(text: &str, start_ms: u32, end_ms: u32) -> whisper_core::parakeet_ipc::Word {
        whisper_core::parakeet_ipc::Word {
            text: text.into(),
            start_ms,
            end_ms,
        }
    }

    #[test]
    fn vad_sentence_overlap_within_one_frame_stays_in_one_valid_segment() {
        let words = [
            vad_test_word("?", 17_920, 18_080),
            vad_test_word("Je", 18_000, 18_080),
        ];
        let segments = parakeet_vad_segments(
            &words,
            vad_test_window(),
            400_000,
            JobId(1),
            Generation::first(),
            1,
            1,
        )
        .unwrap();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].text, "? Je");
        assert_eq!(segments[0].range.start_sample, 17_920 * 16);
        assert_eq!(segments[0].range.end_sample, 18_080 * 16);
        assert_eq!(segments[0].text.split_whitespace().count(), 2);
    }

    #[test]
    fn vad_overlapped_sentence_word_extends_group_to_later_end() {
        let words = [
            vad_test_word("?", 17_920, 18_080),
            vad_test_word("Je", 18_000, 18_100),
        ];
        let segments = parakeet_vad_segments(
            &words,
            vad_test_window(),
            400_000,
            JobId(1),
            Generation::first(),
            1,
            1,
        )
        .unwrap();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].range.end_sample, 18_100 * 16);
        assert_eq!(segments[0].text, "? Je");
    }

    #[test]
    fn vad_sentence_overlap_over_one_tdt_frame_is_rejected() {
        let words = [
            vad_test_word("?", 17_920, 18_080),
            vad_test_word("Je", 17_999, 18_080),
        ];
        let error = parakeet_vad_segments(
            &words,
            vad_test_window(),
            400_000,
            JobId(1),
            Generation::first(),
            1,
            1,
        )
        .unwrap_err();
        assert!(error.contains("unordered"));
    }

    #[test]
    fn gpu_attestation_ack_is_bound_to_the_current_session() {
        let event = WorkerEvent::GpuSessionAttested {
            job_id: JobId(42),
            generation: Generation::first(),
            instance_id: 9,
            candidate: "PCI\\VEN_10DE&DEV_1234".into(),
            dxgi_rank: 2,
            luid: 0x1234,
            pci_id: "0000:01:00.0".into(),
            cuda_ordinal: 1,
            ggml_ordinal: 4,
        };
        let current = (JobId(42), Generation::first(), 9);
        assert_eq!(worker_event_identity(&event), Some(current));
        assert!(!worker_event_is_stale(&event, current));
        assert!(worker_event_is_stale(
            &event,
            (JobId(42), Generation::first(), 10)
        ));
        assert!(worker_event_is_stale(
            &event,
            (JobId(43), Generation::first(), 9)
        ));
    }

    #[test]
    fn missing_cuda_worker_is_reported_without_changing_cpu_path() {
        let missing = std::env::temp_dir().join("whisper-absent-gpu-worker-for-test.exe");
        assert!(!missing.is_file());
        let error = probe_cuda_worker(&missing).unwrap_err();
        assert!(error.contains("GPU indisponible"));
        assert!(error.contains("choisissez CPU"));
    }

    fn compile_fake_worker(root: &Path, mode: &str, marker: &Path) -> PathBuf {
        let source = root.join(format!("{mode}-worker.rs"));
        let executable = root.join(format!("{mode}-worker.exe"));
        let marker = marker
            .to_string_lossy()
            .replace('\\', "\\\\")
            .replace('"', "\\\"");
        let program = r#"
use std::io::{self, BufRead, Write};
const MODE: &str = "__MODE__";
const MARKER: &str = "__MARKER__";
fn field(line: &str, name: &str) -> String {
    let token = format!("\"{name}\":");
    let tail = line.split_once(&token).expect("wire identity field").1.trim_start();
    tail.chars().take_while(|c| c.is_ascii_digit()).collect()
}
fn main() {
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    let start = lines.next().expect("start command").expect("start frame");
    let job = field(&start, "job_id");
    let generation = field(&start, "generation");
    let instance = field(&start, "instance_id");
    let attested = format!("{{\"version\":2,\"message\":{{\"GpuSessionAttested\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"candidate\":\"PCI\\\\VEN_10DE&DEV_1B06\",\"dxgi_rank\":2,\"luid\":4660,\"pci_id\":\"0000:01:00.0\",\"cuda_ordinal\":1,\"ggml_ordinal\":4}}}}}}");
    if MODE == "gpu-ready-exit" {
        println!("{attested}");
        println!("{{\"version\":2,\"message\":{{\"Ready\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"backend\":\"Cuda\"}}}}}}");
        let _ = io::stdout().flush();
        return;
    }
    if MODE == "gpu-ready" {
        println!("{attested}");
        println!("{{\"version\":2,\"message\":{{\"Ready\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"backend\":\"Cuda\"}}}}}}");
        let _ = io::stdout().flush();
        for command in lines.flatten() {
            if command.contains("\"BeginImport\"") {
                std::fs::write(MARKER, format!("{job}:{generation}:{instance}")).expect("begin marker");
                println!("{{\"version\":2,\"message\":{{\"ImportStarted\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance}}}}}}}");
                let _ = io::stdout().flush();
            }
            if command.contains("\"Stop\"") {
                println!("{{\"version\":2,\"message\":{{\"Stopped\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance}}}}}}}");
                let _ = io::stdout().flush();
                return;
            }
        }
        return;
    }
    if MODE == "gpu" {
        println!("{{\"version\":2,\"message\":{{\"Failed\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"code\":\"BackendUnavailable\",\"message\":\"injected GPU pre-Ready failure\"}}}}}}");
        let _ = io::stdout().flush();
        return;
    }
    if MODE == "cpu-fail" {
        std::fs::write(MARKER, format!("{job}:{generation}:{instance}")).expect("failed CPU marker");
        println!("{{\"version\":2,\"message\":{{\"Failed\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"code\":\"BackendUnavailable\",\"message\":\"injected CPU startup failure\"}}}}}}");
        let _ = io::stdout().flush();
        return;
    }
    if !start.contains("\"compute\":\"Cpu\"") { return; }
    if MODE == "cpu-count" {
        use std::io::Write as _;
        let mut marker = std::fs::OpenOptions::new().create(true).append(true).open(MARKER).expect("ready marker");
        writeln!(marker, "{job}:{generation}:{instance}").expect("append ready marker");
    } else {
        std::fs::write(MARKER, format!("{job}:{generation}:{instance}")).expect("ready marker");
    }
    println!("{{\"version\":2,\"message\":{{\"Ready\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"backend\":\"Cpu\"}}}}}}");
    let _ = io::stdout().flush();
    if MODE == "cpu" || MODE == "cpu-count" { return; }
    for command in lines.flatten() {
        if command.contains("\"Stop\"") {
            println!("{{\"version\":2,\"message\":{{\"Stopped\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance}}}}}}}");
            let _ = io::stdout().flush();
            return;
        }
    }
}
"#
            .replace("__MODE__", mode)
            .replace("__MARKER__", &marker);
        std::fs::write(&source, program).unwrap();
        let output = Command::new("rustc")
            .args(["--edition=2021", "--crate-name", "whisper_test_worker"])
            .arg(&source)
            .arg("-o")
            .arg(&executable)
            .output()
            .expect("rustc test worker");
        assert!(
            output.status.success(),
            "rustc test worker: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        executable
    }

    fn compile_multi_session_worker(root: &Path, marker: &Path) -> PathBuf {
        let source = root.join("multi-session-worker.rs");
        let executable = root.join("multi-session-worker.exe");
        let marker = serde_json::to_string(&marker.to_string_lossy().to_string()).unwrap();
        let program = r#"
use std::io::{self, BufRead, Write};
const MARKER: &str = __MARKER__;
fn field(line: &str, name: &str) -> String {
    let token = format!("\"{name}\":");
    let tail = line.split_once(&token).expect("wire identity field").1.trim_start();
    tail.chars().take_while(|c| c.is_ascii_digit()).collect()
}
fn event(name: &str, job: &str, generation: &str, instance: &str) {
    let body = match name {
        "Ready" => format!("\"Ready\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"backend\":\"Cpu\"}}"),
        _ => format!("\"End\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"last_sequence\":0,\"last_offset\":0}}"),
    };
    println!("{{\"version\":2,\"message\":{{{body}}}}}");
    let _ = io::stdout().flush();
}
fn main() {
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    let mut cached_identity: Option<String> = None;
    for line in lines.flatten() {
        if line.contains("\"Shutdown\"") { return; }
        if line.contains("\"StartImport\"") {
            let job = field(&line, "job_id");
            let generation = field(&line, "generation");
            let instance = field(&line, "instance_id");
            let identity = format!("{}:{}", line.contains("model.bin"), line.contains("[9,9,9,9]"));
            let result = if cached_identity.as_ref() == Some(&identity) { "hit" } else { "load" };
            cached_identity = Some(identity);
            use std::io::Write as _;
            let mut file = std::fs::OpenOptions::new().create(true).append(true).open(MARKER).unwrap();
            writeln!(file, "{result}").unwrap();
            event("Ready", &job, &generation, &instance);
            event("End", &job, &generation, &instance);
        }
    }
}
"#
        .replace("__MARKER__", &marker);
        std::fs::write(&source, program).unwrap();
        let output = Command::new("rustc")
            .args([
                "--edition=2021",
                "--crate-name",
                "whisper_test_multi_session_worker",
            ])
            .arg(&source)
            .arg("-o")
            .arg(&executable)
            .output()
            .expect("rustc multi-session test worker");
        assert!(
            output.status.success(),
            "rustc test worker: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        executable
    }

    fn receive_worker_event(
        worker: &mut ChildWorkerTransport,
        timeout: Duration,
    ) -> Result<WorkerEvent, PortError> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(event) = worker.receive()? {
                return Ok(event);
            }
            assert!(Instant::now() < deadline, "worker IPC event timed out");
            thread::sleep(Duration::from_millis(2));
        }
    }

    fn retained_import_command(job_id: u128, instance_id: u64) -> WorkerCommand {
        WorkerCommand::StartImport {
            retain_after_publish: true,
            job_id: JobId(job_id),
            generation: Generation::first(),
            instance_id,
            source_path: "source.wav".into(),
            source_sha256: [7; 32],
            expected_source_samples: 16_000,
            model_path: "model.bin".into(),
            model_sha256: [9; 32],
            config: JobConfig {
                language: LanguageChoice::Manual("fr".into()),
                compute: ComputeChoice::Cpu,
            },
        }
    }

    #[test]
    fn retained_worker_reads_two_correlated_import_sessions_and_hits_model_cache() {
        let root = test_artifact_root("whisper-retained-imports");
        std::fs::create_dir_all(&root).unwrap();
        let marker = root.join("cache-events.txt");
        let executable = compile_multi_session_worker(&root, &marker);
        let mut worker = ChildWorkerTransport::spawn(&executable).unwrap();

        for (job_id, instance_id) in [(801_u128, 41_u64), (802, 42)] {
            worker
                .request(retained_import_command(job_id, instance_id))
                .unwrap();
            assert!(matches!(
                receive_worker_event(&mut worker, Duration::from_secs(3)).unwrap(),
                WorkerEvent::Ready { job_id: actual, generation, instance_id: actual_instance, backend: BackendKind::Cpu }
                    if actual == JobId(job_id) && generation == Generation::first() && actual_instance == instance_id
            ));
            assert!(matches!(
                receive_worker_event(&mut worker, Duration::from_secs(3)).unwrap(),
                WorkerEvent::End { job_id: actual, generation, instance_id: actual_instance, .. }
                    if actual == JobId(job_id) && generation == Generation::first() && actual_instance == instance_id
            ));
        }

        assert_eq!(std::fs::read_to_string(&marker).unwrap(), "load\nhit\n");
        worker.request(WorkerCommand::Shutdown).unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if worker.try_wait().unwrap().is_some() {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "retained worker did not shut down"
            );
            thread::sleep(Duration::from_millis(2));
        }
        drop(worker);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_publish_drops_pending_worker_without_starting_retention() {
        let root = test_artifact_root("whisper-publish-failure-cache");
        std::fs::create_dir_all(&root).unwrap();
        let marker = root.join("cache-events.txt");
        let executable = compile_multi_session_worker(&root, &marker);
        let mut transport = ChildWorkerTransport::spawn(&executable).unwrap();
        transport.request(retained_import_command(803, 43)).unwrap();
        assert!(matches!(
            receive_worker_event(&mut transport, Duration::from_secs(3)).unwrap(),
            WorkerEvent::Ready { .. }
        ));

        let gate = Arc::new(PublicationGate::default());
        gate.activate();
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: executable.clone(),
                gpu: root.join("missing-gpu.exe"),
            },
            gate,
        );
        runtime.archive = ArchiveStore::new(root.join("archive"));
        runtime.pending_idle_worker = Some(IdleWorker {
            transport,
            compute: ComputeChoice::Cpu,
            model_path: "model.bin".into(),
            model_sha256: [9; 32],
            expires_at: Instant::now(),
        });

        let error = runtime
            .publish(JobId(803), Generation::first(), 0, &mut VecDeque::new())
            .expect_err("an unprepared archive cannot publish");

        assert_eq!((error.0, error.1), (JobId(803), Generation::first()));
        assert!(runtime.pending_idle_worker.is_none());
        assert!(runtime.idle_worker.is_none());
        assert!(runtime.transport.is_none());
        drop(runtime);
        std::fs::remove_dir_all(root).unwrap();
    }

    fn compile_verified_gpu_init_failure_worker(
        root: &Path,
        model_path: &Path,
        model_bytes: &[u8],
        model_sha256: [u8; 32],
        error_code: &str,
    ) -> PathBuf {
        let source = root.join("gpu-init-failure-worker.rs");
        let executable = root.join("gpu-init-failure-worker.exe");
        let model_path = model_path.to_string_lossy();
        let model_path_json = serde_json::to_string(&*model_path).unwrap();
        let model_path_json_literal = serde_json::to_string(&model_path_json).unwrap();
        let model_sha256_json = serde_json::to_string(&model_sha256).unwrap();
        let model_sha256_json_literal = serde_json::to_string(&model_sha256_json).unwrap();
        let error_code_literal = format!("\\\"{error_code}\\\"");
        let model_bytes_literal =
            serde_json::to_string(std::str::from_utf8(model_bytes).unwrap()).unwrap();
        let program = r#"
use std::io::{self, BufRead, Write};
const MODEL_PATH_JSON: &str = __MODEL_PATH_JSON__;
const MODEL_PATH: &str = __MODEL_PATH__;
const MODEL_SHA256_JSON: &str = __MODEL_SHA256_JSON__;
const MODEL_BYTES: &str = __MODEL_BYTES__;
fn field(line: &str, name: &str) -> String {
    let token = format!("\"{name}\":");
    let tail = line.split_once(&token).expect("wire identity field").1.trim_start();
    tail.chars().take_while(|c| c.is_ascii_digit()).collect()
}
fn main() {
    let stdin = io::stdin();
    let start = stdin.lock().lines().next().expect("start command").expect("start frame");
    if !start.contains(&format!("\"model_path\":{MODEL_PATH_JSON}"))
        || !start.contains(&format!("\"model_sha256\":{MODEL_SHA256_JSON}"))
        || std::fs::read(MODEL_PATH).ok().as_deref()
            != Some(MODEL_BYTES.as_bytes())
    {
        return;
    }
    let job = field(&start, "job_id");
    let generation = field(&start, "generation");
    let instance = field(&start, "instance_id");
    println!("{{\"version\":2,\"message\":{{\"Failed\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"code\":__ERROR_CODE__,\"message\":\"GPU candidate revalidation failed after model hash verification\"}}}}}}");
    let _ = io::stdout().flush();
}
"#
        .replace("__ERROR_CODE__", &error_code_literal)
        .replace("__MODEL_PATH_JSON__", &model_path_json_literal)
        .replace("__MODEL_PATH__", &model_path_json)
        .replace("__MODEL_SHA256_JSON__", &model_sha256_json_literal)
        .replace("__MODEL_BYTES__", &model_bytes_literal);
        std::fs::write(&source, program).unwrap();
        let output = Command::new("rustc")
            .args(["--edition=2021", "--crate-name", "whisper_gpu_init_failure"])
            .arg(&source)
            .arg("-o")
            .arg(&executable)
            .output()
            .expect("rustc test worker");
        assert!(
            output.status.success(),
            "rustc test worker: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        executable
    }

    fn poll_until(
        runtime: &mut ImportRuntime,
        timeout: Duration,
        mut predicate: impl FnMut(&Result<ImportEvent, (JobId, Generation, String)>) -> bool,
    ) -> Result<ImportEvent, (JobId, Generation, String)> {
        let deadline = std::time::Instant::now() + timeout;
        loop {
            if let Some(event) = runtime.poll()
                && predicate(&event)
            {
                return event;
            }
            if std::time::Instant::now() >= deadline {
                let tail = runtime
                    .transport
                    .as_mut()
                    .map(|transport| (transport.try_wait(), transport.receive()));
                panic!(
                    "worker transport timed out; tail={tail:?}; active={:?}",
                    runtime.active.as_ref().map(|active| (
                        active.job_id,
                        active.generation,
                        active.backend
                    ))
                );
            }
            thread::sleep(Duration::from_millis(2));
        }
    }

    fn prepared_import(job_id: JobId) -> ImportRequest {
        let mut request = test_request(job_id, Generation::first());
        request.source_sha256 = Some([7; 32]);
        request.source_samples = Some(16_000);
        request.config.compute = ComputeChoice::Auto;
        request
    }

    fn fake_gpu_candidate() -> whisper_core::gpu_inventory::GpuRuntimeIdentity {
        whisper_core::gpu_inventory::GpuRuntimeIdentity {
            generation: 8,
            pnp_instance_id: whisper_core::model_registry::PARAKEET_TEST_GPU_PNP_INSTANCE_ID.into(),
            dxgi_rank: 2,
            luid: 0x1234,
            pci_id: "0000:01:00.0".into(),
            cuda_ordinal: 1,
            ggml_ordinal: 4,
        }
    }

    fn stop_worker(runtime: &mut ImportRuntime, job_id: JobId, generation: Generation) {
        let mut events = VecDeque::new();
        runtime.stop(job_id, generation, &mut events).unwrap();
        let stopped = poll_until(runtime, Duration::from_secs(3), |event| {
            matches!(event, Ok(ImportEvent::Stopped { .. }) | Err(_))
        });
        assert!(matches!(stopped, Ok(ImportEvent::Stopped { .. }) | Err(_)));
    }

    #[test]
    fn auto_without_gpu_starts_cpu_with_normalized_compute_and_same_identity() {
        let root = std::env::temp_dir().join(format!("whisper-auto-no-gpu-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let marker = root.join("cpu-ready");
        let cpu = compile_fake_worker(&root, "cpu", &marker);
        let gate = Arc::new(PublicationGate::default());
        gate.activate();
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu,
                gpu: root.join("missing-gpu.exe"),
            },
            gate,
        );
        let request = prepared_import(JobId(701));
        runtime.start_worker(request.clone()).unwrap();
        let ready = poll_until(&mut runtime, Duration::from_secs(3), |event| {
            matches!(event, Ok(ImportEvent::Ready { .. }) | Err(_))
        });
        assert!(
            matches!(ready, Ok(ImportEvent::Ready { .. })),
            "transport event: {ready:?}"
        );
        assert!(matches!(
            ready,
            Ok(ImportEvent::Ready { job_id, generation, backend })
                if job_id == request.job_id && generation == request.generation
                    && backend.starts_with("CPU") && backend.contains("worker CUDA absent")
                    && backend.contains("CPU prend le relais")
        ));
        let identity = std::fs::read_to_string(&marker).unwrap();
        assert!(identity.starts_with("701:1:"));
        stop_worker(&mut runtime, request.job_id, request.generation);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn auto_cpu_startup_failure_ends_without_another_attempt() {
        let root = test_artifact_root("whisper-auto-cpu-fail");
        std::fs::create_dir_all(&root).unwrap();
        let marker = root.join("cpu-attempt");
        let cpu = compile_fake_worker(&root, "cpu-fail", &marker);
        let gate = Arc::new(PublicationGate::default());
        gate.activate();
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu,
                gpu: root.join("missing-gpu.exe"),
            },
            gate,
        );
        let request = prepared_import(JobId(709));
        runtime.start_worker(request.clone()).unwrap();
        let failure = poll_until(&mut runtime, Duration::from_secs(3), |event| event.is_err());
        assert!(matches!(failure, Err((job_id, generation, _))
            if (job_id, generation) == (request.job_id, request.generation)));
        assert_eq!(runtime.active.as_ref().map(|active| active.backend), None);
        assert!(
            std::fs::read_to_string(&marker)
                .unwrap()
                .starts_with("709:1:")
        );
        if let Some(transport) = runtime.transport.as_mut() {
            transport.abort();
        }
        drop(runtime);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn auto_gpu_ready_is_attested_and_explicitly_admitted() {
        let root = test_artifact_root("whisper-auto-gpu-ready");
        std::fs::create_dir_all(&root).unwrap();
        let marker = root.join("begin-import");
        let gpu = compile_fake_worker(&root, "gpu-ready", &marker);
        let gate = Arc::new(PublicationGate::default());
        gate.activate();
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: root.join("unused-cpu.exe"),
                gpu,
            },
            gate,
        );
        let request = prepared_import(JobId(706));
        runtime
            .start_worker_with_selection(request.clone(), Some(Some(fake_gpu_candidate())))
            .unwrap();
        let ready = poll_until(&mut runtime, Duration::from_secs(3), |event| {
            matches!(event, Ok(ImportEvent::Ready { .. }) | Err(_))
        });
        assert!(matches!(ready, Ok(ImportEvent::Ready { backend, .. }) if backend == "CUDA"));
        let started = std::time::Instant::now();
        while !marker.is_file() && started.elapsed() < Duration::from_secs(3) {
            let _ = runtime.poll();
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(
            marker.is_file(),
            "BeginImport must reach the attested GPU worker"
        );
        let started = std::time::Instant::now();
        while !runtime.active.as_ref().unwrap().startup_ready
            && started.elapsed() < Duration::from_secs(3)
        {
            let _ = runtime.poll();
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(runtime.active.as_ref().unwrap().startup_ready);
        runtime.transport.as_mut().unwrap().abort();
        runtime.transport = None;
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn auto_import_gpu_exit_after_ready_before_begin_retries_once_on_cpu() {
        let root = test_artifact_root("whisper-auto-gpu-exit-before-begin");
        std::fs::create_dir_all(&root).unwrap();
        let gpu_begin = root.join("gpu-begin");
        let cpu_started = root.join("cpu-started");
        let cpu = compile_fake_worker(&root, "cpu-count", &cpu_started);
        let gpu = compile_fake_worker(&root, "gpu-ready-exit", &gpu_begin);
        let gate = Arc::new(PublicationGate::default());
        gate.activate();
        let mut runtime = ImportRuntime::new(WorkerPaths { cpu, gpu }, gate);
        let request = prepared_import(JobId(709));
        runtime
            .start_worker_with_selection(request.clone(), Some(Some(fake_gpu_candidate())))
            .unwrap();
        let ready = poll_until(&mut runtime, Duration::from_secs(5), |event| {
            matches!(event, Ok(ImportEvent::Ready { backend, .. }) if backend.starts_with("CPU"))
                || event.is_err()
        });
        assert!(
            matches!(&ready, Ok(ImportEvent::Ready { job_id, generation, backend })
            if *job_id == request.job_id && *generation == request.generation
                && backend.starts_with("CPU") && backend.contains("GPU non utilisable")
                && backend.contains("CPU prend le relais")),
            "unexpected recovery result: {ready:?}"
        );
        let active = runtime.active.as_ref().unwrap();
        assert_eq!(active.backend, BackendKind::Cpu);
        assert!(cpu_started.is_file(), "CPU worker reached Ready");
        assert_eq!(
            std::fs::read_to_string(&cpu_started)
                .unwrap()
                .lines()
                .count(),
            1,
            "the single CPU fallback starts once"
        );
        assert!(!gpu_begin.exists(), "GPU never admitted audio processing");
        stop_worker(&mut runtime, request.job_id, request.generation);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn auto_import_unlaunchable_gpu_uses_one_cpu_attempt() {
        let root = test_artifact_root("whisper-auto-gpu-spawn");
        std::fs::create_dir_all(&root).unwrap();
        let marker = root.join("cpu-ready");
        let cpu = compile_fake_worker(&root, "cpu", &marker);
        let gpu = root.join("broken-gpu.exe");
        std::fs::write(&gpu, b"not an executable").unwrap();
        let gate = Arc::new(PublicationGate::default());
        gate.activate();
        let mut runtime = ImportRuntime::new(WorkerPaths { cpu, gpu }, gate);
        let request = prepared_import(JobId(708));
        runtime.start_worker(request.clone()).unwrap();
        let ready = poll_until(&mut runtime, Duration::from_secs(3), |event| {
            matches!(event, Ok(ImportEvent::Ready { .. }) | Err(_))
        });
        assert!(matches!(ready, Ok(ImportEvent::Ready { backend, .. })
            if backend.starts_with("CPU") && backend.contains("lancement CUDA impossible")
                && backend.contains("CPU prend le relais")));
        assert!(marker.is_file());
        stop_worker(&mut runtime, request.job_id, request.generation);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn gpu_import_failed_then_eof_retries_cpu_and_accepts_same_job_ready() {
        let root =
            std::env::temp_dir().join(format!("whisper-import-retry-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let marker = root.join("cpu-ready");
        let model_path = root.join("verified-model.bin");
        let model_bytes = b"verified model fixture";
        std::fs::write(&model_path, model_bytes).unwrap();
        let model_sha256: [u8; 32] = Sha256::digest(model_bytes).into();
        let cpu = compile_fake_worker(&root, "cpu", &marker);
        let gpu = compile_verified_gpu_init_failure_worker(
            &root,
            &model_path,
            model_bytes,
            model_sha256,
            "BackendMismatch",
        );
        let gate = Arc::new(PublicationGate::default());
        gate.activate();
        let mut runtime = ImportRuntime::new(WorkerPaths { cpu, gpu }, gate);
        let request = prepared_import(JobId(702));
        let mut request = request;
        request.model_path = model_path.to_string_lossy().into_owned();
        request.model_sha256 = model_sha256;
        runtime.start_worker(request.clone()).unwrap();
        assert_eq!(runtime.active.as_ref().unwrap().backend, BackendKind::Cuda);
        let ready = poll_until(&mut runtime, Duration::from_secs(5), |event| {
            matches!(event, Ok(ImportEvent::Ready { .. }))
        });
        assert!(matches!(
            ready,
            Ok(ImportEvent::Ready { job_id, generation, backend })
                if job_id == request.job_id && generation == request.generation
                    && backend.starts_with("CPU") && backend.contains("GPU non utilisable")
                    && backend.contains("CPU prend le relais")
        ));
        assert_eq!(runtime.active.as_ref().unwrap().backend, BackendKind::Cpu);
        assert!(
            std::fs::read_to_string(&marker)
                .unwrap()
                .starts_with("702:1:")
        );
        assert_eq!(std::fs::read_to_string(&marker).unwrap().lines().count(), 1);
        stop_worker(&mut runtime, request.job_id, request.generation);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn gpu_live_failed_then_eof_retries_cpu_and_accepts_ready_before_capture_activation() {
        let root = std::env::temp_dir().join(format!("whisper-live-retry-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let marker = root.join("cpu-ready");
        let model_path = root.join("verified-model.bin");
        let model_bytes = b"verified model fixture";
        std::fs::write(&model_path, model_bytes).unwrap();
        let model_sha256: [u8; 32] = Sha256::digest(model_bytes).into();
        let cpu = compile_fake_worker(&root, "cpu-marker", &marker);
        let gpu = compile_verified_gpu_init_failure_worker(
            &root,
            &model_path,
            model_bytes,
            model_sha256,
            "BackendUnavailable",
        );
        let gate = Arc::new(PublicationGate::default());
        gate.activate();
        let mut runtime = ImportRuntime::new(WorkerPaths { cpu, gpu }, gate);
        let request = whisper_core::LiveRequest {
            job_id: JobId(704),
            generation: Generation::first(),
            model_path: model_path.to_string_lossy().into_owned(),
            model_sha256,
            destination: root.to_string_lossy().into_owned(),
            group_offset_samples: 0,
            auto_stop_after_speech_samples: None,
            microphone: Default::default(),
            microphone_gain_db: 0,
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Auto,
            },
        };
        runtime.start_live(request.clone(), None).unwrap();
        let observed = poll_until(&mut runtime, Duration::from_secs(5), |event| {
            matches!(event, Ok(ImportEvent::Ready { backend, .. }) if backend.starts_with("CPU"))
                || event.is_err()
        });
        assert_eq!(runtime.active.as_ref().unwrap().backend, BackendKind::Cpu);
        assert!(
            std::fs::read_to_string(&marker)
                .unwrap()
                .starts_with("704:1:")
        );
        if let Ok(ImportEvent::Ready {
            job_id,
            generation,
            backend,
        }) = observed
        {
            assert_eq!((job_id, generation), (request.job_id, request.generation));
            assert!(backend.starts_with("CPU"));
        } else if let Err((job_id, generation, message)) = observed {
            assert_eq!((job_id, generation), (request.job_id, request.generation));
            assert!(
                !message.contains("WorkerExited"),
                "CPU Ready was consumed before capture activation failed: {message}"
            );
        } else {
            panic!("unexpected event while awaiting CPU Ready after GPU EOF");
        }
        stop_worker(&mut runtime, request.job_id, request.generation);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn forced_gpu_initialization_failure_waits_for_explicit_cpu_choice() {
        let root =
            std::env::temp_dir().join(format!("whisper-forced-gpu-choice-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let marker = root.join("cpu-ready");
        let model_path = root.join("verified-model.bin");
        let model_bytes = b"verified model fixture";
        std::fs::write(&model_path, model_bytes).unwrap();
        let model_sha256: [u8; 32] = Sha256::digest(model_bytes).into();
        let cpu = compile_fake_worker(&root, "cpu", &marker);
        let gpu = compile_verified_gpu_init_failure_worker(
            &root,
            &model_path,
            model_bytes,
            model_sha256,
            "BackendUnavailable",
        );
        let gate = Arc::new(PublicationGate::default());
        gate.activate();
        let mut runtime = ImportRuntime::new(WorkerPaths { cpu, gpu }, gate);
        let request = whisper_core::LiveRequest {
            job_id: JobId(705),
            generation: Generation::first(),
            model_path: model_path.to_string_lossy().into_owned(),
            model_sha256,
            destination: root.to_string_lossy().into_owned(),
            group_offset_samples: 0,
            auto_stop_after_speech_samples: None,
            microphone: Default::default(),
            microphone_gain_db: 0,
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Gpu,
            },
        };
        assert!(
            runtime
                .start_live(request.clone(), None)
                .unwrap_err()
                .2
                .contains("selected GPU has no current identity proof")
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn stop_latched_after_gpu_failure_prevents_cpu_worker_launch() {
        let root =
            std::env::temp_dir().join(format!("whisper-stop-no-retry-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let marker = root.join("cpu-ready");
        let cpu = compile_fake_worker(&root, "cpu", &marker);
        let gpu = compile_fake_worker(&root, "gpu", &root.join("gpu-unused"));
        let gate = Arc::new(PublicationGate::default());
        gate.activate();
        let mut runtime = ImportRuntime::new(WorkerPaths { cpu, gpu }, gate.clone());
        let request = prepared_import(JobId(703));
        runtime.start_worker(request.clone()).unwrap();
        let _ = poll_until(&mut runtime, Duration::from_secs(3), |event| {
            matches!(event, Ok(ImportEvent::LiveAttemptStarting { .. }))
        });
        gate.request_stop().unwrap();
        let stopped = poll_until(&mut runtime, Duration::from_secs(5), |event| {
            matches!(event, Ok(ImportEvent::Stopped { .. }) | Err(_))
        });
        assert!(matches!(stopped, Ok(ImportEvent::Stopped { .. })));
        assert!(runtime.active.is_none());
        assert!(!marker.exists(), "CPU retry must not launch after Stop");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn auto_import_gpu_failure_retains_request_until_correlated_stop_for_cpu_retry() {
        let mut request = test_request(JobId(88), Generation::first());
        request.config.compute = ComputeChoice::Auto;
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("cpu.exe"),
                gpu: PathBuf::from("gpu.exe"),
            },
            Arc::new(PublicationGate::default()),
        );
        runtime.active = Some(ImportRequestState {
            job_id: request.job_id,
            generation: request.generation,
            instance_id: 9,
            backend: BackendKind::Cuda,
            attempt_number: 0,
            import_request: Some(request.clone()),
            recovery: LiveRecoveryState::None,
            startup_ready: false,
            gpu_candidate: None,
            gpu_attested: false,
        });
        let unbound = WorkerEvent::Failed {
            job_id: None,
            generation: None,
            instance_id: None,
            code: whisper_core::ipc::WorkerErrorCode::BackendUnavailable,
            message: "GPU unavailable before Ready".into(),
        };
        let bound = bind_unidentified_startup_failure(
            unbound,
            (request.job_id, request.generation, 9),
            false,
        );
        let WorkerEvent::Failed {
            job_id: Some(job_id),
            generation: Some(generation),
            instance_id: Some(instance_id),
            code,
            message,
        } = bound
        else {
            panic!("unidentified startup failure must bind to the launched attempt");
        };
        assert_eq!(
            (job_id, generation, instance_id),
            (request.job_id, request.generation, 9)
        );
        assert_eq!(code, whisper_core::ipc::WorkerErrorCode::BackendUnavailable);

        let event = runtime
            .handle_worker_failure(job_id, generation, code, message)
            .unwrap();
        assert!(matches!(event, ImportEvent::LiveAttemptStarting { .. }));
        let active = runtime.active.as_ref().unwrap();
        assert_eq!(active.recovery, LiveRecoveryState::AutoRetryPending);
        assert_eq!(active.import_request.as_ref(), Some(&request));
        assert!(
            runtime.transport.is_none(),
            "retry must wait for correlated Stopped and child exit"
        );

        let stale = WorkerEvent::Failed {
            job_id: Some(request.job_id),
            generation: Some(request.generation),
            instance_id: Some(8),
            code: whisper_core::ipc::WorkerErrorCode::BackendUnavailable,
            message: "stale GPU attempt".into(),
        };
        let stale_after_binding = bind_unidentified_startup_failure(
            stale.clone(),
            (request.job_id, request.generation, 9),
            false,
        );
        assert_eq!(stale_after_binding, stale);
        assert_ne!(
            worker_event_identity(&stale_after_binding),
            Some((request.job_id, request.generation, 9)),
            "correlated stale failures remain rejectable"
        );
        let after_ready = WorkerEvent::Failed {
            job_id: None,
            generation: None,
            instance_id: None,
            code: whisper_core::ipc::WorkerErrorCode::BackendUnavailable,
            message: "failure after Ready".into(),
        };
        assert_eq!(
            bind_unidentified_startup_failure(
                after_ready.clone(),
                (request.job_id, request.generation, 9),
                true,
            ),
            after_ready,
            "identity binding is limited to the startup phase"
        );
    }

    #[test]
    fn auto_import_does_not_retry_cpu_after_gpu_ready() {
        let request = prepared_import(JobId(707));
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("cpu.exe"),
                gpu: PathBuf::from("gpu.exe"),
            },
            Arc::new(PublicationGate::default()),
        );
        runtime.active = Some(ImportRequestState {
            job_id: request.job_id,
            generation: request.generation,
            instance_id: 10,
            backend: BackendKind::Cuda,
            attempt_number: 0,
            import_request: Some(request.clone()),
            recovery: LiveRecoveryState::None,
            startup_ready: true,
            gpu_candidate: None,
            gpu_attested: false,
        });
        let result = runtime.handle_worker_failure(
            request.job_id,
            request.generation,
            whisper_core::ipc::WorkerErrorCode::InferenceFailed,
            "injected failure after audio started".into(),
        );
        assert!(result.is_err());
        assert!(runtime.active.is_none());
        assert!(runtime.auto_fallback_reason.is_none());
    }

    #[test]
    fn queued_import_record_accepts_legacy_live_offset_field() {
        let request = ImportRequest {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
            job_id: JobId(1),
            generation: Generation::first(),
            source_path: "queued.wav".into(),
            source_sha256: None,
            source_samples: None,
            model_path: "model.bin".into(),
            model_sha256: [1; 32],
            destination: "archive".into(),
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Cpu,
            },
        };
        let mut legacy = serde_json::to_value(&request).unwrap();
        legacy
            .as_object_mut()
            .unwrap()
            .insert("group_offset_samples".into(), serde_json::json!(1234));
        assert_eq!(
            serde_json::from_value::<ImportRequest>(legacy).unwrap(),
            request
        );
    }

    #[test]
    fn pending_reservation_rejects_three_event_burst_at_twenty_slots() {
        let burst = ImportEffect::ResumeInterrupted {
            previous: Box::new(test_request(JobId(1), Generation::first())),
            request: test_request(JobId(2), Generation::first()),
        };
        assert_eq!(effect_event_reservation(&burst), 3);
        assert!(20 + effect_event_reservation(&burst) > EVENTS_PER_STAGE);
        assert!(18 + effect_event_reservation(&burst) <= EVENTS_PER_STAGE);
    }

    #[test]
    fn stale_mp3_and_failed_events_have_old_attempt_identity() {
        let old = (JobId(7), Generation::first(), 41);
        let packet = WorkerEvent::LiveMp3Packet {
            job_id: old.0,
            generation: old.1,
            instance_id: old.2,
            sequence: 1,
            bytes: vec![1, 2, 3],
        };
        let failed = WorkerEvent::Failed {
            job_id: Some(old.0),
            generation: Some(old.1),
            instance_id: Some(old.2),
            code: whisper_core::ipc::WorkerErrorCode::InvalidRequest,
            message: "old attempt failed".into(),
        };
        let current = (JobId(7), Generation::first(), 42);
        assert_ne!(worker_event_identity(&packet), Some(current));
        assert_ne!(worker_event_identity(&failed), Some(current));
        assert!(worker_event_is_stale(&packet, current));
        assert!(worker_event_is_stale(&failed, current));
    }

    #[test]
    fn failed_worker_message_does_not_hide_later_correlated_stop() {
        let id = (JobId(8), Generation::first(), 51);
        let failed = WorkerEvent::Failed {
            job_id: Some(id.0),
            generation: Some(id.1),
            instance_id: Some(id.2),
            code: whisper_core::ipc::WorkerErrorCode::InvalidRequest,
            message: "reported failure".into(),
        };
        let stopped = WorkerEvent::Stopped {
            job_id: id.0,
            generation: id.1,
            instance_id: id.2,
        };
        let mut bytes = Vec::new();
        write_frame(&mut bytes, &IpcEnvelope::new(failed)).unwrap();
        write_frame(&mut bytes, &IpcEnvelope::new(stopped.clone())).unwrap();
        let (tx, rx) = mpsc::sync_channel(4);
        read_events(Cursor::new(bytes), tx);
        assert!(matches!(
            rx.recv().unwrap(),
            WorkerReaderMessage::Event(Ok(WorkerEvent::Failed { .. }))
        ));
        assert_eq!(rx.recv().unwrap(), WorkerReaderMessage::Event(Ok(stopped)));
    }

    #[test]
    fn stale_stopped_does_not_hide_later_correlated_stop() {
        let job_id = JobId(8);
        let generation = Generation::first();
        let stale = WorkerEvent::Stopped {
            job_id,
            generation,
            instance_id: 52,
        };
        let current = WorkerEvent::Stopped {
            job_id,
            generation,
            instance_id: 51,
        };
        let mut bytes = Vec::new();
        write_frame(&mut bytes, &IpcEnvelope::new(stale.clone())).unwrap();
        write_frame(&mut bytes, &IpcEnvelope::new(current.clone())).unwrap();
        let (tx, rx) = mpsc::sync_channel(3);
        read_events(Cursor::new(bytes), tx);
        assert_eq!(rx.recv().unwrap(), WorkerReaderMessage::Event(Ok(stale)));
        assert_eq!(rx.recv().unwrap(), WorkerReaderMessage::Event(Ok(current)));
        assert!(matches!(
            rx.recv().unwrap(),
            WorkerReaderMessage::TransportEnded
        ));
    }

    #[test]
    fn worker_reader_signals_transport_end_after_session_end() {
        let mut bytes = Vec::new();
        write_frame(
            &mut bytes,
            &IpcEnvelope::new(WorkerEvent::End {
                job_id: JobId(18),
                generation: Generation::first(),
                instance_id: 1,
                last_sequence: 0,
                last_offset: 0,
            }),
        )
        .unwrap();
        let (tx, rx) = mpsc::sync_channel(2);
        read_events(Cursor::new(bytes), tx);
        assert!(matches!(
            rx.recv().unwrap(),
            WorkerReaderMessage::Event(Ok(WorkerEvent::End {
                job_id: JobId(18),
                ..
            }))
        ));
        assert_eq!(rx.recv().unwrap(), WorkerReaderMessage::TransportEnded);
    }

    #[test]
    fn slow_worker_backpressure_keeps_command_queue_bounded_and_retries_window() {
        let (sender, receiver) = mpsc::sync_channel(1);
        let command = || WorkerCommand::LiveWindow {
            job_id: JobId(1),
            generation: Generation::first(),
            instance_id: 1,
            sequence: 1,
            range: whisper_core::SourceRange::new(0, 80_000, 16_000).unwrap(),
            samples: vec![1; 80_000],
        };
        try_enqueue_bounded(&sender, command()).unwrap();
        let retry = try_enqueue_bounded(&sender, command()).unwrap_err();
        assert!(matches!(
            retry,
            EnqueueError::Full(command) if matches!(*command, WorkerCommand::LiveWindow { .. })
        ));
        drop(receiver.recv().unwrap());
        assert!(matches!(receiver.try_recv(), Err(TryRecvError::Empty)));
        try_enqueue_bounded(&sender, command()).unwrap();
        assert!(matches!(
            receiver.try_recv(),
            Ok(WorkerCommand::LiveWindow { .. })
        ));
    }

    #[test]
    fn capture_failure_stop_is_retained_when_data_channel_is_full() {
        let (data_tx, data_rx) = mpsc::sync_channel(1);
        let (control_tx, control_rx) = mpsc::sync_channel(1);
        let window = WorkerCommand::LiveWindow {
            job_id: JobId(2),
            generation: Generation::first(),
            instance_id: 1,
            sequence: 1,
            range: whisper_core::SourceRange::new(0, 80_000, 16_000).unwrap(),
            samples: vec![2; 80_000],
        };
        try_enqueue_worker_command(&data_tx, &control_tx, window).unwrap();
        let stop = WorkerCommand::Stop {
            job_id: JobId(2),
            generation: Generation::first(),
        };
        try_enqueue_worker_command(&data_tx, &control_tx, stop.clone()).unwrap();
        assert!(matches!(
            try_enqueue_worker_command(&data_tx, &control_tx, stop),
            Err(EnqueueError::Full(command)) if matches!(*command, WorkerCommand::Stop { .. })
        ));
        assert!(matches!(
            data_rx.try_recv(),
            Ok(WorkerCommand::LiveWindow { .. })
        ));
        assert!(matches!(
            control_rx.try_recv(),
            Ok(WorkerCommand::Stop { .. })
        ));
    }

    #[test]
    fn stdin_frames_preserve_live_start_and_admitted_window_before_normal_stop() {
        let (commands_tx, commands_rx) = mpsc::sync_channel(4);
        let (_emergency_tx, emergency_rx) = mpsc::sync_channel(1);
        let job_id = JobId(22);
        let generation = Generation::first();
        let config = JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        };
        try_enqueue_bounded(
            &commands_tx,
            WorkerCommand::StartLiveWorker {
                job_id,
                generation,
                instance_id: 1,
                model_path: "model.bin".into(),
                model_sha256: [22; 32],
                config,
            },
        )
        .unwrap();
        try_enqueue_bounded(
            &commands_tx,
            WorkerCommand::LiveWindow {
                job_id,
                generation,
                instance_id: 1,
                sequence: 1,
                range: whisper_core::SourceRange::new(0, 320, 16_000).unwrap(),
                samples: vec![3; 320],
            },
        )
        .unwrap();
        try_enqueue_bounded(&commands_tx, WorkerCommand::Stop { job_id, generation }).unwrap();

        let mut wire = Vec::new();
        for _ in 0..3 {
            let WorkerCommandPoll::Command(command) =
                take_next_worker_command(&commands_rx, &emergency_rx)
            else {
                panic!("expected a queued normal command");
            };
            write_frame(&mut wire, &IpcEnvelope::new(command)).unwrap();
        }
        assert!(matches!(emergency_rx.try_recv(), Err(TryRecvError::Empty)));

        let mut frames = BufReader::new(Cursor::new(wire));
        assert!(matches!(
            read_frame::<IpcEnvelope<WorkerCommand>>(&mut frames)
                .unwrap()
                .unwrap()
                .message,
            WorkerCommand::StartLiveWorker { .. }
        ));
        assert!(matches!(
            read_frame::<IpcEnvelope<WorkerCommand>>(&mut frames)
                .unwrap()
                .unwrap()
                .message,
            WorkerCommand::LiveWindow { sequence: 1, .. }
        ));
        assert!(matches!(
            read_frame::<IpcEnvelope<WorkerCommand>>(&mut frames)
                .unwrap()
                .unwrap()
                .message,
            WorkerCommand::Stop { .. }
        ));
    }

    #[test]
    fn stopped_worker_waits_until_live_segment_persistence_is_acknowledged() {
        let stopped = WorkerEvent::Stopped {
            job_id: JobId(3),
            generation: Generation::first(),
            instance_id: 1,
        };
        let mut deferred = Some(stopped.clone());
        assert!(take_ready_live_stop(1, &mut deferred).is_none());
        assert_eq!(deferred, Some(stopped.clone()));
        assert_eq!(take_ready_live_stop(0, &mut deferred), Some(stopped));
        assert!(deferred.is_none());
    }

    #[test]
    fn replay_windows_map_after_confirmed_prefix_without_duplicate_coverage() {
        assert_eq!(map_attempt_sequence(4, 3, 4), Some(4));
        assert_eq!(map_attempt_sequence(4, 3, 5), Some(5));
        assert_eq!(map_attempt_sequence(4, 3, 3), None);
        assert_eq!(map_attempt_sequence(u64::MAX, 0, 2), None);
    }

    #[test]
    fn queued_live_start_is_applied_before_an_immediate_stop() {
        struct Runtime(Arc<std::sync::Mutex<Vec<&'static str>>>);
        impl ServiceRuntime for Runtime {
            fn apply_effect(&mut self, effect: ImportEffect, pending: &mut VecDeque<ImportEvent>) {
                match effect {
                    ImportEffect::StartLive(request) => {
                        self.0.lock().unwrap().push("start");
                        pending.push_back(ImportEvent::Ready {
                            job_id: request.job_id,
                            generation: request.generation,
                            backend: "CPU".into(),
                        });
                    }
                    ImportEffect::Stop { job_id, generation } => {
                        self.0.lock().unwrap().push("stop");
                        pending.push_back(ImportEvent::Stopped { job_id, generation });
                    }
                    _ => {}
                }
            }

            fn poll_event(&mut self) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
                None
            }
        }

        let job_id = JobId(4);
        let generation = Generation::first();
        let (effect_tx, effect_rx) = mpsc::sync_channel(2);
        let (stop_tx, stop_rx) = mpsc::sync_channel(1);
        let (event_tx, event_rx) = mpsc::sync_channel(4);
        drop(event_rx);
        effect_tx
            .send(ImportEffect::StartLive(whisper_core::LiveRequest {
                job_id,
                generation,
                model_path: "model.bin".into(),
                model_sha256: [0; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                microphone_gain_db: 0,
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            }))
            .unwrap();
        stop_tx
            .send(ImportEffect::Stop { job_id, generation })
            .unwrap();
        let order = Arc::new(std::sync::Mutex::new(Vec::new()));
        let runtime = Runtime(order.clone());
        std::thread::spawn(move || service_loop(effect_rx, stop_rx, event_tx, runtime))
            .join()
            .unwrap();
        assert_eq!(*order.lock().unwrap(), vec!["start", "stop"]);
    }

    #[test]
    fn gpu_choice_keeps_drained_counters_ahead_of_following_events() {
        let job_id = JobId(20);
        let generation = Generation::first();
        let mut stop_events = VecDeque::new();
        stop_events.push_back(ImportEvent::LiveFinalizing { job_id, generation });
        stop_events.push_back(ImportEvent::LiveCounters {
            job_id,
            generation,
            captured_samples: 640,
            admitted_samples: 640,
            audio_durable_samples: 640,
            confirmed_fragment_samples: 0,
            speech_samples: 0,
            diagnostic: None,
        });
        let gate = Arc::new(PublicationGate::default());
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu.exe"),
                gpu: PathBuf::from("unused-gpu.exe"),
            },
            gate,
        );
        runtime.pending_recovery_counters = recovery_stop_counters(stop_events);
        assert!(matches!(
            runtime.poll(),
            Some(Ok(ImportEvent::LiveCounters {
                audio_durable_samples: 640,
                ..
            }))
        ));
        assert!(runtime.pending_recovery_counters.is_none());
    }

    #[test]
    fn stop_counters_reach_application_before_final_window() {
        struct Runtime {
            stopped: bool,
            window_sent: bool,
        }
        impl ServiceRuntime for Runtime {
            fn apply_effect(&mut self, effect: ImportEffect, pending: &mut VecDeque<ImportEvent>) {
                if let ImportEffect::Stop { job_id, generation } = effect {
                    self.stopped = true;
                    pending.push_back(ImportEvent::LiveCounters {
                        job_id,
                        generation,
                        captured_samples: 254_240,
                        admitted_samples: 254_240,
                        audio_durable_samples: 254_240,
                        confirmed_fragment_samples: 160_000,
                        speech_samples: 0,
                        diagnostic: None,
                    });
                    pending.push_back(ImportEvent::LiveFinalizing { job_id, generation });
                    assert!(pending.len() <= EVENTS_PER_STAGE);
                }
            }

            fn poll_event(&mut self) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
                if !self.stopped {
                    return None;
                }
                if self.window_sent {
                    return Some(Ok(ImportEvent::History(Vec::new())));
                }
                self.window_sent = true;
                Some(Ok(ImportEvent::WindowFinished {
                    job_id: JobId(20),
                    generation: Generation::first(),
                    window_sequence: 4,
                    confirmed_samples: 254_240,
                }))
            }
        }

        let (effect_tx, effect_rx) = mpsc::sync_channel(1);
        let (stop_tx, stop_rx) = mpsc::sync_channel(1);
        let (event_tx, event_rx) = mpsc::sync_channel(EVENTS_PER_STAGE);
        drop(effect_tx);
        stop_tx
            .send(ImportEffect::Stop {
                job_id: JobId(20),
                generation: Generation::first(),
            })
            .unwrap();
        let service = thread::spawn(move || {
            service_loop(
                effect_rx,
                stop_rx,
                event_tx,
                Runtime {
                    stopped: false,
                    window_sent: false,
                },
            )
        });
        assert!(matches!(
            event_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            ImportEvent::LiveCounters {
                audio_durable_samples: 254_240,
                ..
            }
        ));
        assert!(matches!(
            event_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            ImportEvent::LiveFinalizing { .. }
        ));
        assert!(matches!(
            event_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            ImportEvent::WindowFinished {
                confirmed_samples: 254_240,
                ..
            }
        ));
        drop(event_rx);
        service.join().unwrap();
    }

    #[test]
    fn direct_micro_start_and_single_stop_are_applied_in_order() {
        struct Runtime(mpsc::Sender<&'static str>);
        impl ServiceRuntime for Runtime {
            fn apply_effect(&mut self, effect: ImportEffect, pending: &mut VecDeque<ImportEvent>) {
                match effect {
                    ImportEffect::StartDirectLive(_) => self.0.send("start").unwrap(),
                    ImportEffect::Stop { job_id, generation } => {
                        self.0.send("stop").unwrap();
                        pending.push_back(ImportEvent::Stopped { job_id, generation });
                    }
                    _ => {}
                }
            }

            fn poll_event(&mut self) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
                Some(Ok(ImportEvent::History(Vec::new())))
            }
        }

        let (effect_tx, effect_rx) = mpsc::sync_channel(1);
        let (stop_tx, stop_rx) = mpsc::sync_channel(1);
        let (event_tx, event_rx) = mpsc::sync_channel(2);
        let (action_tx, action_rx) = mpsc::channel();
        let job_id = JobId(25);
        let generation = Generation::first();
        effect_tx
            .send(ImportEffect::StartDirectLive(whisper_core::LiveRequest {
                job_id,
                generation,
                model_path: "model.bin".into(),
                model_sha256: [25; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                microphone_gain_db: 0,
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            }))
            .unwrap();
        stop_tx
            .send(ImportEffect::Stop { job_id, generation })
            .unwrap();
        let service =
            thread::spawn(move || service_loop(effect_rx, stop_rx, event_tx, Runtime(action_tx)));

        assert_eq!(
            action_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            "start"
        );
        assert_eq!(
            action_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            "stop"
        );
        drop(event_rx);
        service.join().unwrap();
    }

    #[test]
    fn saturated_pending_queue_cannot_start_a_live_job_after_stop_and_allows_next_start() {
        struct Runtime {
            gate: Arc<PublicationGate>,
            remaining_events: usize,
            saturated: Option<mpsc::SyncSender<()>>,
            actions: mpsc::Sender<&'static str>,
        }
        impl ServiceRuntime for Runtime {
            fn apply_effect(&mut self, effect: ImportEffect, pending: &mut VecDeque<ImportEvent>) {
                match effect {
                    ImportEffect::StartLive(_) => {
                        if self.gate.is_cancelled() {
                            self.actions.send("late-start-skipped").unwrap();
                        } else {
                            self.actions.send("new-start-accepted").unwrap();
                        }
                    }
                    ImportEffect::Stop { job_id, generation } => {
                        self.actions.send("stop-without-active-worker").unwrap();
                        pending.push_back(ImportEvent::Stopped { job_id, generation });
                    }
                    _ => {}
                }
            }

            fn poll_event(&mut self) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
                if self.remaining_events == 0 {
                    return None;
                }
                self.remaining_events -= 1;
                if self.remaining_events == 0
                    && let Some(saturated) = self.saturated.take()
                {
                    saturated.send(()).unwrap();
                }
                Some(Ok(ImportEvent::History(Vec::new())))
            }
        }

        let gate = Arc::new(PublicationGate::default());
        let (effect_tx, effect_rx) = mpsc::sync_channel(2);
        let (stop_tx, stop_rx) = mpsc::sync_channel(1);
        let (event_tx, event_rx) = mpsc::sync_channel(EVENTS_PER_STAGE);
        for _ in 0..EVENTS_PER_STAGE {
            event_tx.send(ImportEvent::History(Vec::new())).unwrap();
        }
        let (saturated_tx, saturated_rx) = mpsc::sync_channel(1);
        let (actions_tx, actions_rx) = mpsc::channel();
        let runtime = Runtime {
            gate: gate.clone(),
            remaining_events: EVENTS_PER_STAGE,
            saturated: Some(saturated_tx),
            actions: actions_tx,
        };
        let service = thread::spawn(move || service_loop(effect_rx, stop_rx, event_tx, runtime));
        saturated_rx.recv_timeout(Duration::from_secs(2)).unwrap();

        let first_id = JobId(24);
        let generation = Generation::first();
        gate.activate();
        effect_tx
            .send(ImportEffect::StartLive(whisper_core::LiveRequest {
                job_id: first_id,
                generation,
                model_path: "model.bin".into(),
                model_sha256: [24; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                microphone_gain_db: 0,
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            }))
            .unwrap();
        gate.request_stop().unwrap();
        stop_tx
            .send(ImportEffect::Stop {
                job_id: first_id,
                generation,
            })
            .unwrap();

        for _ in 0..EVENTS_PER_STAGE {
            event_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        }
        let mut saw_stopped = false;
        while !saw_stopped {
            saw_stopped = matches!(
                event_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
                ImportEvent::Stopped { job_id, .. } if job_id == first_id
            );
        }
        assert_eq!(
            actions_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            "stop-without-active-worker"
        );
        assert_eq!(
            actions_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            "late-start-skipped"
        );

        gate.activate();
        effect_tx
            .send(ImportEffect::StartLive(whisper_core::LiveRequest {
                job_id: JobId(25),
                generation,
                model_path: "model.bin".into(),
                model_sha256: [25; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                microphone_gain_db: 0,
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            }))
            .unwrap();
        assert_eq!(
            actions_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            "new-start-accepted"
        );
        gate.request_stop().unwrap();
        stop_tx
            .send(ImportEffect::Stop {
                job_id: JobId(25),
                generation,
            })
            .unwrap();
        drop(event_rx);
        service.join().unwrap();
    }

    #[test]
    fn stop_before_commit_atomically_prevents_publication() {
        let gate = PublicationGate::default();
        gate.activate();
        assert!(gate.request_stop().unwrap());
        let mut disk_commit = false;
        assert_eq!(
            gate.commit(|| {
                disk_commit = true;
                Ok(())
            }),
            Err("Cancelled: publication invalidated by Stop".into())
        );
        assert!(!disk_commit, "CURRENT commit must not run after Stop wins");
    }

    #[test]
    fn cancelled_gate_skips_live_start_before_any_worker_or_capture_side_effect() {
        let gate = Arc::new(PublicationGate::default());
        gate.activate();
        gate.request_stop().unwrap();
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("missing-cpu-worker"),
                gpu: PathBuf::from("missing-gpu-worker"),
            },
            gate,
        );
        let mut events = VecDeque::new();
        runtime.effect(
            ImportEffect::StartLive(whisper_core::LiveRequest {
                job_id: JobId(26),
                generation: Generation::first(),
                model_path: "model.bin".into(),
                model_sha256: [26; 32],
                destination: "unused-archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                microphone_gain_db: 0,
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            }),
            &mut events,
        );
        assert!(runtime.active.is_none());
        assert!(runtime.transport.is_none());
        assert!(runtime.pending_live.is_none());
        assert!(runtime.live.is_none());
        assert!(events.is_empty());
    }

    #[test]
    fn effect_queue_accepts_eight_and_rejects_the_ninth_without_blocking() {
        let gate = Arc::new(PublicationGate::default());
        let (mut io, effects, _, _, _, _, _) = AsyncImportIo::for_test(gate);
        for index in 0..8 {
            io.submit(ImportEffect::ScanHistory {
                destination: format!("archive-{index}"),
            })
            .unwrap();
        }
        assert_eq!(
            io.submit(ImportEffect::ScanHistory {
                destination: "archive-overflow".into(),
            }),
            Err(PortError::Unavailable),
            "the ninth queued effect is refused rather than blocking or growing the queue"
        );
        assert_eq!(effects.try_iter().count(), 8);
    }

    #[test]
    fn persist_segment_queue_full_is_distinct_from_disconnection() {
        let gate = Arc::new(PublicationGate::default());
        let (mut io, effects, _, _, _, _, _) = AsyncImportIo::for_test(gate);
        let segment = |sequence| WorkerSegment {
            job_id: JobId(904),
            generation: Generation::first(),
            instance_id: 1,
            segment_id: whisper_core::SegmentId(sequence),
            range: whisper_core::SourceRange::new((sequence - 1) * 80, sequence * 80, 16_000)
                .unwrap(),
            text: format!("segment-{sequence}"),
        };

        for sequence in 1..=8 {
            io.submit(ImportEffect::PersistSegment(segment(sequence)))
                .unwrap();
        }
        assert_eq!(
            io.submit(ImportEffect::PersistSegment(segment(9))),
            Err(PortError::Backpressure),
            "a full persistence queue is retryable"
        );
        assert_eq!(
            effects
                .try_iter()
                .filter_map(|effect| match effect {
                    ImportEffect::PersistSegment(segment) => Some(segment.segment_id.0),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            (1..=8).collect::<Vec<_>>()
        );

        let gate = Arc::new(PublicationGate::default());
        let (mut disconnected, effects, _, _, _, _, _) = AsyncImportIo::for_test(gate);
        drop(effects);
        assert_eq!(
            disconnected.submit(ImportEffect::PersistSegment(segment(1))),
            Err(PortError::Unavailable),
            "a disconnected consumer remains fatal and is never classified as backpressure"
        );
    }

    #[test]
    fn event_65_backpressures_all_production_stages_then_delivers_in_order() {
        struct WireRuntime {
            worker_events: Receiver<WorkerReaderMessage>,
            consumed: Arc<std::sync::atomic::AtomicUsize>,
            shutdown: Receiver<()>,
        }
        impl ServiceRuntime for WireRuntime {
            fn apply_effect(&mut self, _: ImportEffect, _: &mut VecDeque<ImportEvent>) {}

            fn poll_event(&mut self) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
                match self.worker_events.try_recv() {
                    Ok(WorkerReaderMessage::Event(Ok(WorkerEvent::Progress {
                        job_id,
                        generation,
                        completed_samples,
                        total_samples,
                        ..
                    }))) => {
                        self.consumed.fetch_add(1, Ordering::AcqRel);
                        Some(Ok(ImportEvent::Progress {
                            job_id,
                            generation,
                            completed_samples,
                            total_samples,
                        }))
                    }
                    Ok(WorkerReaderMessage::Event(Ok(WorkerEvent::Failed {
                        job_id: Some(job_id),
                        generation: Some(generation),
                        message,
                        ..
                    }))) => {
                        self.consumed.fetch_add(1, Ordering::AcqRel);
                        Some(Err((job_id, generation, message)))
                    }
                    Ok(WorkerReaderMessage::Event(Err(error))) => Some(Err((
                        JobId(903),
                        Generation::first(),
                        format!("worker reader: {error:?}"),
                    ))),
                    Ok(WorkerReaderMessage::Event(Ok(_)) | WorkerReaderMessage::TransportEnded) => {
                        None
                    }
                    Err(TryRecvError::Empty) => match self.shutdown.try_recv() {
                        Ok(()) => Some(Ok(ImportEvent::History(Vec::new()))),
                        Err(_) => None,
                    },
                    Err(TryRecvError::Disconnected) => match self.shutdown.try_recv() {
                        Ok(()) => Some(Ok(ImportEvent::History(Vec::new()))),
                        Err(_) => None,
                    },
                }
            }
        }

        const BACKLOG: usize = EVENTS_PER_STAGE * 3 + 1;
        const EVENT_COUNT: usize = BACKLOG + 1;
        let job_id = JobId(903);
        let generation = Generation::first();
        let mut wire = Vec::new();
        for completed_samples in 1..=BACKLOG as u64 {
            let envelope = IpcEnvelope::new(WorkerEvent::Progress {
                job_id,
                generation,
                instance_id: 1,
                completed_samples,
                total_samples: EVENT_COUNT as u64,
            });
            serde_json::to_writer(&mut wire, &envelope).unwrap();
            wire.push(b'\n');
        }
        let failure = IpcEnvelope::new(WorkerEvent::Failed {
            job_id: Some(job_id),
            generation: Some(generation),
            instance_id: Some(1),
            code: whisper_core::ipc::WorkerErrorCode::Saturated,
            message: "65th event after bounded backlog".into(),
        });
        serde_json::to_writer(&mut wire, &failure).unwrap();
        wire.push(b'\n');

        let (mut io, effect_rx, stop_rx, app_tx, _, _, _) =
            AsyncImportIo::for_test(Arc::new(PublicationGate::default()));
        let app_capacity_probe = app_tx.clone();
        let (worker_tx, worker_rx) = mpsc::sync_channel(EVENTS_PER_STAGE);
        let consumed = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let consumed_probe = consumed.clone();
        let (shutdown_tx, shutdown_rx) = mpsc::channel();
        let service = thread::spawn(move || {
            service_loop(
                effect_rx,
                stop_rx,
                app_tx,
                WireRuntime {
                    worker_events: worker_rx,
                    consumed,
                    shutdown: shutdown_rx,
                },
            )
        });
        let (reader_done_tx, reader_done_rx) = mpsc::channel();
        let reader = thread::spawn(move || {
            read_events(Cursor::new(wire), worker_tx);
            reader_done_tx.send(()).unwrap();
        });

        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while consumed_probe.load(Ordering::Acquire) < EVENTS_PER_STAGE * 2 {
            assert!(
                std::time::Instant::now() < deadline,
                "pipeline did not fill"
            );
            thread::sleep(Duration::from_millis(5));
        }
        assert!(matches!(
            app_capacity_probe.try_send(ImportEvent::History(Vec::new())),
            Err(TrySendError::Full(_))
        ));
        assert!(
            matches!(reader_done_rx.try_recv(), Err(mpsc::TryRecvError::Empty)),
            "the wire reader must be backpressured while all 64 backlog slots are occupied"
        );
        drop(app_capacity_probe);

        let mut progress = Vec::with_capacity(BACKLOG);
        let mut visible_failure = None;
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        while progress.len() + usize::from(visible_failure.is_some()) < EVENT_COUNT {
            assert!(
                std::time::Instant::now() < deadline,
                "drained events were lost"
            );
            match io.poll().unwrap() {
                Some(ImportEvent::Progress {
                    completed_samples, ..
                }) => progress.push(completed_samples),
                Some(ImportEvent::Failed { message, .. }) => visible_failure = Some(message),
                Some(other) => panic!("unexpected event after saturation: {other:?}"),
                None => thread::sleep(Duration::from_millis(1)),
            }
        }
        assert_eq!(progress, (1..=BACKLOG as u64).collect::<Vec<_>>());
        assert_eq!(
            visible_failure.as_deref(),
            Some("65th event after bounded backlog")
        );
        reader_done_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        reader.join().unwrap();
        drop(io);
        shutdown_tx.send(()).unwrap();
        service.join().unwrap();
    }

    #[test]
    fn stop_control_is_admitted_when_effect_queue_is_full() {
        let gate = Arc::new(PublicationGate::default());
        let (mut io, effects, stops, _, _, _, _) = AsyncImportIo::for_test(gate);
        let job_id = JobId(901);
        let generation = Generation::first();
        io.submit(ImportEffect::Prepare(test_request(job_id, generation)))
            .unwrap();
        for index in 0..7 {
            io.submit(ImportEffect::ScanHistory {
                destination: format!("archive-{index}"),
            })
            .unwrap();
        }
        assert_eq!(effects.try_iter().count(), 8);
        io.submit(ImportEffect::Stop { job_id, generation })
            .unwrap();
        assert!(matches!(stops.try_recv(), Ok(ImportEffect::Stop { .. })));
    }

    #[test]
    fn frame_limit_counts_newline_and_rejects_oversize_before_write() {
        let gate = Arc::new(PublicationGate::default());
        let (mut io, _effects, _, _, _, _, _) = AsyncImportIo::for_test(gate);
        let mut low = 0;
        let mut high = MAX_FRAME_BYTES;
        while low < high {
            let middle = (low + high).div_ceil(2);
            let effect = ImportEffect::ScanHistory {
                destination: "x".repeat(middle),
            };
            let encoded_len = serde_json::to_vec(&effect).unwrap().len() + 1;
            if encoded_len <= MAX_FRAME_BYTES {
                low = middle;
            } else {
                high = middle - 1;
            }
        }
        let accepted = ImportEffect::ScanHistory {
            destination: "x".repeat(low),
        };
        assert_eq!(
            serialize_frame(&accepted).unwrap().len() + 1,
            MAX_FRAME_BYTES
        );
        io.submit(accepted).unwrap();
        let rejected = ImportEffect::ScanHistory {
            destination: "x".repeat(low + 1),
        };
        assert_eq!(io.submit(rejected), Err(PortError::InvalidInput));
    }

    #[test]
    fn framed_ipc_rejects_incomplete_oversized_and_v1_messages() {
        let incomplete = b"{\"version\":2".as_slice();
        assert!(read_frame::<IpcEnvelope<WorkerEvent>>(&mut Cursor::new(incomplete)).is_err());

        let oversized = vec![b'x'; MAX_FRAME_BYTES];
        assert!(read_frame::<serde_json::Value>(&mut Cursor::new(oversized)).is_err());

        let v1 = b"{\"version\":1,\"message\":{}}\n".as_slice();
        let envelope = read_frame::<IpcEnvelope<serde_json::Value>>(&mut Cursor::new(v1))
            .unwrap()
            .unwrap();
        assert!(!envelope.is_supported());
    }

    #[test]
    fn worker_eof_without_end_is_reported_as_failure() {
        let (tx, rx) = mpsc::sync_channel(2);
        read_events(Cursor::new(Vec::<u8>::new()), tx);
        assert!(
            matches!(rx.try_recv(), Ok(WorkerReaderMessage::Event(Err(PortError::Failed(message)))) if message.contains("without End"))
        );
    }

    fn test_destination(job_id: JobId, generation: Generation) -> PathBuf {
        std::env::temp_dir()
            .join("worker-ipc-tests")
            .join(format!("{}-{job_id:?}-{generation:?}", std::process::id()))
    }

    fn test_request(job_id: JobId, generation: Generation) -> ImportRequest {
        ImportRequest {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
            job_id,
            generation,
            source_path: "source.wav".into(),
            source_sha256: None,
            source_samples: None,
            model_path: "model.bin".into(),
            model_sha256: [0; 32],
            destination: test_destination(job_id, generation)
                .to_string_lossy()
                .into_owned(),
            config: JobConfig {
                language: LanguageChoice::Manual("fr".into()),
                compute: ComputeChoice::Cpu,
            },
        }
    }

    fn compile_r7_resume_worker(root: &Path, ready: &Path, release: &Path) -> PathBuf {
        let source = root.join("r7-resume-worker.rs");
        let executable = root.join("r7-resume-worker.exe");
        let ready = serde_json::to_string(&ready.to_string_lossy().to_string()).unwrap();
        let release = serde_json::to_string(&release.to_string_lossy().to_string()).unwrap();
        let program = r#"
use std::{io::{self, BufRead, Write}, thread, time::Duration};
const READY: &str = __READY__;
const RELEASE: &str = __RELEASE__;
fn field(line: &str, name: &str) -> String {
    let token = format!("\"{name}\":");
    line.split_once(&token).expect("wire field").1.trim_start()
        .chars().take_while(|c| c.is_ascii_digit()).collect()
}
fn emit(body: String) { println!("{{\"version\":2,\"message\":{{{body}}}}}"); io::stdout().flush().unwrap(); }
fn main() {
    let start = io::stdin().lock().lines().next().unwrap().unwrap();
    let job = field(&start, "job_id");
    let generation = field(&start, "generation");
    let instance = field(&start, "instance_id");
    let samples = field(&start, "expected_source_samples");
    emit(format!("\"Ready\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"backend\":\"Cpu\"}}"));
    std::fs::write(READY, b"ready").unwrap();
    while !std::path::Path::new(RELEASE).is_file() { thread::sleep(Duration::from_millis(5)); }
    emit(format!("\"Segment\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"segment_id\":1,\"range\":{{\"start_sample\":0,\"end_sample\":16000,\"sample_rate_hz\":16000}},\"text\":\"r7 confirmed transcript\"}}"));
    emit(format!("\"Segment\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"segment_id\":2,\"range\":{{\"start_sample\":16000,\"end_sample\":{samples},\"sample_rate_hz\":16000}},\"text\":\"r7 resumed transcript\"}}"));
    emit(format!("\"Progress\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"completed_samples\":{samples},\"total_samples\":{samples}}}"));
    emit(format!("\"End\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"last_sequence\":2,\"last_offset\":{samples}}}"));
}
"#
            .replace("__READY__", &ready)
            .replace("__RELEASE__", &release);
        std::fs::write(&source, program).unwrap();
        let output = Command::new("rustc")
            .args(["--edition=2021", "--crate-name", "whisper_r7_resume_worker"])
            .arg(&source)
            .arg("-o")
            .arg(&executable)
            .output()
            .expect("compile deterministic R7 resume worker");
        assert!(
            output.status.success(),
            "rustc fake worker failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        executable
    }

    fn write_r7_resume_wav(path: &Path, samples: u32) {
        use std::io::Write;
        let data_bytes = samples * 2;
        let mut file = std::fs::File::create(path).unwrap();
        file.write_all(b"RIFF").unwrap();
        file.write_all(&(36 + data_bytes).to_le_bytes()).unwrap();
        file.write_all(b"WAVEfmt ").unwrap();
        file.write_all(&16_u32.to_le_bytes()).unwrap();
        file.write_all(&1_u16.to_le_bytes()).unwrap();
        file.write_all(&1_u16.to_le_bytes()).unwrap();
        file.write_all(&16_000_u32.to_le_bytes()).unwrap();
        file.write_all(&32_000_u32.to_le_bytes()).unwrap();
        file.write_all(&2_u16.to_le_bytes()).unwrap();
        file.write_all(&16_u16.to_le_bytes()).unwrap();
        file.write_all(b"data").unwrap();
        file.write_all(&data_bytes.to_le_bytes()).unwrap();
        file.write_all(&vec![0_u8; data_bytes as usize]).unwrap();
    }

    fn r7_resume_request(
        job_id: JobId,
        source: &Path,
        destination: &Path,
        model: &Path,
        model_sha256: [u8; 32],
    ) -> ImportRequest {
        ImportRequest {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
            job_id,
            generation: Generation::first(),
            source_path: source.to_string_lossy().into_owned(),
            source_sha256: None,
            source_samples: None,
            model_path: model.to_string_lossy().into_owned(),
            model_sha256,
            destination: destination.to_string_lossy().into_owned(),
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Cpu,
            },
        }
    }

    fn r7_hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    fn r7_generation_path(root: &Path, job_id: JobId, generation: Generation) -> PathBuf {
        root.join("transcriptions")
            .join(format!("{:032x}", job_id.0))
            .join(generation.get().to_string())
    }

    fn wait_r7_prompt(
        app: &mut ImportApplication<AsyncImportIo>,
    ) -> whisper_core::ports::DuplicateImportPrompt {
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            let _ = app.dispatch(AppCommand::Refresh);
            let view = app.view();
            if let Some(prompt) = view.duplicate_import_prompt {
                return prompt;
            }
            assert!(
                Instant::now() < deadline,
                "resume confirmation prompt timed out"
            );
            thread::sleep(Duration::from_millis(5));
        }
    }

    fn run_r7_integrated_copy_resume_case(root: &Path, replace_existing: bool, model: &Path) {
        use whisper_core::ports::DuplicateImportChoice;

        let destination = root.join(if replace_existing { "replace" } else { "first" });
        let source = root.join(if replace_existing {
            "replace-source.wav"
        } else {
            "first-source.wav"
        });
        std::fs::create_dir_all(&destination).unwrap();
        write_r7_resume_wav(&source, 32_000);
        let model_sha256 = crate::model_catalog::cached_model_sha256(model).unwrap().1;
        let source_hash = |request: &ImportRequest| r7_hex(&request.source_sha256.unwrap());

        let old_publications = if replace_existing {
            let mut previous = r7_resume_request(
                JobId(((std::process::id() as u128) << 64) | 0x771),
                &source,
                &destination,
                model,
                model_sha256,
            );
            ArchiveStore::identify_source(&mut previous).unwrap();
            let mut old_archive = ArchiveStore::new(&destination);
            let prepared = old_archive.prepare(previous).unwrap();
            old_archive
                .persist_segment(&whisper_core::ports::WorkerSegment {
                    job_id: prepared.job_id,
                    generation: prepared.generation,
                    instance_id: 1,
                    segment_id: whisper_core::SegmentId(1),
                    range: whisper_core::SourceRange::new(0, 32_000, 16_000).unwrap(),
                    text: "older published transcript".into(),
                })
                .unwrap();
            old_archive
                .publish(prepared.job_id, prepared.generation, 1)
                .unwrap();
            ArchiveStore::published_imports(&destination, &source_hash(&prepared)).unwrap()
        } else {
            Vec::new()
        };

        let mut interrupted = r7_resume_request(
            JobId(
                ((std::process::id() as u128) << 64) | if replace_existing { 0x772 } else { 0x773 },
            ),
            &source,
            &destination,
            model,
            model_sha256,
        );
        ArchiveStore::identify_source(&mut interrupted).unwrap();
        let choice = if replace_existing {
            DuplicateImportChoice::Replace
        } else {
            DuplicateImportChoice::CreateNew
        };
        if replace_existing {
            let basename = ArchiveStore::suggest_duplicate_export_basename(
                &destination,
                &interrupted.source_path,
            )
            .unwrap();
            ArchiveStore::reserve_duplicate_import(
                &destination,
                &interrupted,
                &source_hash(&interrupted),
                interrupted.source_samples.unwrap(),
                &old_publications,
                choice.clone(),
                &basename,
            )
            .unwrap();
        } else {
            crate::archive::ArchiveStore::reserve_new_import(
                &destination,
                &interrupted,
                &source_hash(&interrupted),
            )
            .unwrap();
        }
        let mut interrupted_archive = ArchiveStore::new(&destination);
        interrupted = interrupted_archive.prepare(interrupted).unwrap();
        interrupted_archive
            .persist_segment(&whisper_core::ports::WorkerSegment {
                job_id: interrupted.job_id,
                generation: interrupted.generation,
                instance_id: 1,
                segment_id: whisper_core::SegmentId(1),
                range: whisper_core::SourceRange::new(0, 16_000, 16_000).unwrap(),
                text: "r7 confirmed transcript".into(),
            })
            .unwrap();
        let generation =
            r7_generation_path(&destination, interrupted.job_id, interrupted.generation);
        crate::archive::inject_copy_fallback_publication_cut(&destination, "srt");
        let interrupted_result =
            interrupted_archive.publish(interrupted.job_id, interrupted.generation, 1);
        assert!(
            interrupted_result
                .as_ref()
                .is_err_and(|error| error.contains("test interruption after final srt create_new")),
            "the fault must occur after final SRT create_new and before identity persistence: {interrupted_result:?}"
        );
        assert!(!generation.join("CURRENT").exists());
        assert_eq!(
            ArchiveStore::published_imports(&destination, &source_hash(&interrupted)).unwrap(),
            old_publications,
            "old active publications stay visible after the export-copy interruption"
        );

        let intent_path = destination
            .join(".whisper-publications/reservations")
            .join(format!("{:032x}-1.json", interrupted.job_id.0));
        let intent: serde_json::Value =
            serde_json::from_slice(&std::fs::read(intent_path).unwrap()).unwrap();
        let interrupted_basename = intent["export_basename"].as_str().unwrap().to_owned();
        let ambiguous_txt = destination
            .join("exports")
            .join(format!("{interrupted_basename}.txt"));
        let ambiguous_srt = destination
            .join("exports")
            .join(format!("{interrupted_basename}.srt"));
        let ambiguous_txt_bytes = std::fs::read(&ambiguous_txt).unwrap();
        let ambiguous_srt_bytes = std::fs::read(&ambiguous_srt).unwrap();
        let ambiguous_txt_identity = same_file::Handle::from_path(&ambiguous_txt).unwrap();
        let ambiguous_srt_identity = same_file::Handle::from_path(&ambiguous_srt).unwrap();
        let transcript_srt = std::fs::read(generation.join("transcript.srt")).unwrap();

        // This is an independently created, byte-identical third-party collision at
        // the next candidate name. The restart flow must suggest another name.
        let collision_basename =
            ArchiveStore::suggest_duplicate_export_basename(&destination, &interrupted.source_path)
                .unwrap();
        let third_party = destination
            .join("exports")
            .join(format!("{collision_basename}.srt"));
        std::fs::write(&third_party, &transcript_srt).unwrap();
        let third_party_identity = same_file::Handle::from_path(&third_party).unwrap();
        let third_party_bytes = std::fs::read(&third_party).unwrap();
        let final_basename =
            ArchiveStore::suggest_duplicate_export_basename(&destination, &interrupted.source_path)
                .unwrap();

        let mut queue = crate::queue_store::QueueStore::open(&destination).unwrap();
        queue.enqueue(interrupted.clone()).unwrap();
        queue
            .set_status(
                interrupted.job_id,
                interrupted.generation,
                crate::queue_store::QueueStatus::Interrupted,
            )
            .unwrap();

        let ready = root.join(format!("resume-ready-{}", interrupted.job_id.0));
        let release = root.join(format!("resume-release-{}", interrupted.job_id.0));
        let worker = compile_r7_resume_worker(root, &ready, &release);
        let mut app = ImportApplication::new(AsyncImportIo::start(worker));
        app.dispatch(AppCommand::ResumeInterrupted {
            request: interrupted.clone(),
        })
        .unwrap();
        let prompt = wait_r7_prompt(&mut app);
        assert_eq!(prompt.resume_from, Some(interrupted.clone()));
        assert_eq!(prompt.export_basename, final_basename);
        assert!(
            prompt
                .revalidation_notice
                .as_deref()
                .is_some_and(|notice| { notice.contains("Confirmez explicitement") }),
            "ambiguous export must require visible reconfirmation"
        );
        assert_eq!(prompt.publications, old_publications);
        app.dispatch(AppCommand::ResolveDuplicateImport { prompt, choice })
            .unwrap();

        let deadline = Instant::now() + Duration::from_secs(60);
        while !ready.is_file() {
            assert!(
                Instant::now() < deadline,
                "resumed worker did not reach Ready; view={:?}",
                app.view()
            );
            let _ = app.dispatch(AppCommand::Refresh).unwrap();
            thread::sleep(Duration::from_millis(5));
        }
        let resumed = interrupted.clone_with_next_generation();
        let current_queue = crate::queue_store::QueueStore::open(&destination).unwrap();
        let row = current_queue
            .entries()
            .into_iter()
            .find(|entry| entry.request.job_id == resumed.job_id)
            .expect("resumed request is durable in queue");
        assert_eq!(row.request.generation, resumed.generation);
        assert_eq!(row.status, crate::queue_store::QueueStatus::Running);
        assert_eq!(
            ArchiveStore::published_imports(&destination, &source_hash(&interrupted)).unwrap(),
            old_publications,
            "Replace leaves all old publications active until the resumed generation commits"
        );
        assert_eq!(std::fs::read(&ambiguous_txt).unwrap(), ambiguous_txt_bytes);
        assert_eq!(std::fs::read(&ambiguous_srt).unwrap(), ambiguous_srt_bytes);
        assert_eq!(
            same_file::Handle::from_path(&ambiguous_txt).unwrap(),
            ambiguous_txt_identity
        );
        assert_eq!(
            same_file::Handle::from_path(&ambiguous_srt).unwrap(),
            ambiguous_srt_identity
        );

        std::fs::write(&release, b"continue").unwrap();
        let deadline = Instant::now() + Duration::from_secs(60);
        let completed = loop {
            let view = app.dispatch(AppCommand::Refresh).unwrap_or_else(|error| {
                panic!("resume refresh failed: {error:?}; view={:?}", app.view())
            });
            if view.active_job == Some((resumed.job_id, JobState::Complete)) {
                break view;
            }
            assert!(
                Instant::now() < deadline,
                "resumed worker did not publish; view={:?}",
                app.view()
            );
            thread::sleep(Duration::from_millis(5));
        };
        assert!(completed.last_result.is_some());
        let published =
            ArchiveStore::published_imports(&destination, &source_hash(&interrupted)).unwrap();
        assert_eq!(published.len(), 1);
        assert_eq!(
            (published[0].job_id, published[0].generation),
            (resumed.job_id, resumed.generation)
        );
        let resumed_generation =
            r7_generation_path(&destination, resumed.job_id, resumed.generation);
        assert!(resumed_generation.join("CURRENT").is_file());
        assert_eq!(
            std::fs::read(resumed_generation.join("transcript.txt")).unwrap(),
            b"r7 confirmed transcript\nr7 resumed transcript"
        );
        assert!(
            destination
                .join("exports")
                .join(format!("{final_basename}.txt"))
                .is_file()
        );
        assert!(
            destination
                .join("exports")
                .join(format!("{final_basename}.srt"))
                .is_file()
        );
        assert_eq!(std::fs::read(&third_party).unwrap(), third_party_bytes);
        assert_eq!(
            same_file::Handle::from_path(&third_party).unwrap(),
            third_party_identity,
            "byte-identical third-party collision is never moved or adopted"
        );
        assert_eq!(std::fs::read(&ambiguous_txt).unwrap(), ambiguous_txt_bytes);
        assert_eq!(std::fs::read(&ambiguous_srt).unwrap(), ambiguous_srt_bytes);
        drop(app);
        crate::archive::clear_copy_fallback_publication_cut();
        std::fs::remove_dir_all(destination).unwrap();
    }

    trait NextGeneration {
        fn clone_with_next_generation(&self) -> Self;
    }

    impl NextGeneration for ImportRequest {
        fn clone_with_next_generation(&self) -> Self {
            let mut resumed = self.clone();
            resumed.generation = self.generation.next().unwrap();
            resumed
        }
    }

    #[test]
    fn r7_restart_resume_integrates_copy_crash_dialog_queue_and_publication() {
        let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let model = repository.join("models/ggml-large-v3-turbo.bin");
        if !model.is_file() {
            eprintln!("NOT RUN: local Turbo model missing for integrated R7 resume test");
            return;
        }
        let root = test_artifact_root("r7-integrated-copy-resume").join(format!(
            "{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        run_r7_integrated_copy_resume_case(&root, false, &model);
        run_r7_integrated_copy_resume_case(&root, true, &model);
        crate::archive::clear_copy_fallback_publication_cut();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn queued_request_identity_allows_only_compute_choice_and_verified_model_path_binding() {
        let mut durable = test_request(JobId(910), Generation::first());
        durable.source_sha256 = Some([1; 32]);
        durable.source_samples = Some(16_000);
        let mut requested = durable.clone();
        requested.config.compute = ComputeChoice::Cpu;
        assert!(queued_request_identity_matches(&durable, &requested, true));

        requested.config.language = LanguageChoice::Automatic;
        assert!(!queued_request_identity_matches(&durable, &requested, true));
        requested = durable.clone();
        requested.config.compute = ComputeChoice::Auto;
        requested.model_path = "verified-catalog-path.gguf".into();
        assert!(queued_request_identity_matches(&durable, &requested, true));
        assert!(!queued_request_identity_matches(
            &durable, &requested, false
        ));
    }

    #[test]
    fn queued_manual_gpu_preflight_requires_a_current_candidate_and_gpu_key() {
        let mut request = test_request(JobId(911), Generation::first());
        request.model_sha256 = decode_sha256(
            whisper_core::model_registry::model(
                whisper_core::model_catalog::ModelId::ParakeetTdt06BV3,
            )
            .artifact
            .sha256,
        )
        .unwrap();
        request.config.compute = ComputeChoice::Gpu;
        let key = crate::archive::ArchiveQualificationKey::parakeet_gpu_test();
        let mut candidate = fake_gpu_candidate();
        candidate.generation = request.generation.get();

        assert!(validate_queued_gpu_preflight(&request, Some(&candidate), Some(&key)).is_ok());
        assert_eq!(
            validate_queued_gpu_preflight(&request, None, Some(&key)).unwrap_err(),
            "BackendMismatch: selected GPU has no current runtime identity proof"
        );
        let mut stale = candidate.clone();
        stale.generation += 1;
        assert_eq!(
            validate_queued_gpu_preflight(&request, Some(&stale), Some(&key)).unwrap_err(),
            "BackendMismatch: selected GPU identity belongs to a stale generation"
        );
        assert_eq!(
            validate_queued_gpu_preflight(&request, Some(&candidate), None).unwrap_err(),
            "ModelNotQualified: GPU Test key is not admitted"
        );
    }

    #[test]
    fn queued_manual_gpu_without_current_proof_refuses_before_pending_creation() {
        for stale_proof in [false, true] {
            let job_id = JobId(if stale_proof { 913 } else { 912 });
            let generation = Generation::first();
            let root = test_artifact_root(if stale_proof {
                "r10-queued-gpu-stale-proof"
            } else {
                "r10-queued-gpu-missing-proof"
            });
            let destination = root.join("archive");
            let mut request = test_request(job_id, generation);
            request.model_sha256 = decode_sha256(
                whisper_core::model_registry::model(
                    whisper_core::model_catalog::ModelId::ParakeetTdt06BV3,
                )
                .artifact
                .sha256,
            )
            .unwrap();
            request.source_sha256 = Some([9; 32]);
            request.source_samples = Some(16_000);
            request.destination = destination.to_string_lossy().into_owned();
            request.config.compute = ComputeChoice::Gpu;
            crate::queue_store::QueueStore::open(&destination)
                .unwrap()
                .enqueue(request.clone())
                .unwrap();

            let mut runtime = ImportRuntime::new(
                WorkerPaths {
                    cpu: root.join("unused-cpu.exe"),
                    gpu: root.join("unused-gpu.exe"),
                },
                Arc::new(PublicationGate::default()),
            );
            if stale_proof {
                let mut stale = fake_gpu_candidate();
                stale.generation = generation.get() + 1;
                runtime
                    .gpu_candidates
                    .insert((job_id, generation.get()), Some(stale));
            }

            let mut events = VecDeque::new();
            runtime.effect(ImportEffect::ProcessQueued(request.clone()), &mut events);
            let expected_message = if stale_proof {
                "BackendMismatch: selected GPU identity belongs to a stale generation"
            } else {
                "BackendMismatch: selected GPU has no current runtime identity proof"
            };
            assert!(events.iter().any(|event| matches!(event,
                ImportEvent::Failed { job_id: failed_job, generation: failed_generation, message }
                    if *failed_job == job_id
                        && *failed_generation == generation
                        && message == expected_message
            )));

            let pending = destination
                .join("transcriptions")
                .join(format!("{:032x}", job_id.0))
                .join(generation.get().to_string())
                .join("pending.json");
            assert!(!pending.exists(), "refusal must precede pending creation");
            let queue = crate::queue_store::QueueStore::open(&destination).unwrap();
            assert_eq!(
                queue.entries()[0].request.config.compute,
                ComputeChoice::Gpu
            );
            assert_eq!(
                queue.entries()[0].status,
                crate::queue_store::QueueStatus::Queued
            );
            std::fs::remove_dir_all(root).unwrap();
        }
    }

    #[cfg(feature = "l3-gpu-qualification")]
    #[test]
    fn l3c_acceptance_fault_is_confined_to_its_canonical_execution_root() {
        assert!(option_env!("PARAKEET_L2_ACCEPTANCE_TEST_BUILD").is_some());
        let root = canonical_l3_acceptance_root(
            std::env::var_os("PARAKEET_L3C_EXEC_ROOT").map(PathBuf::from),
            std::env::var_os("PARAKEET_L3D_EXEC_ROOT").map(PathBuf::from),
        )
        .expect("exactly one canonical L3c or L3d execution root");
        let destination = root.join("r8/acceptance-test");
        std::fs::create_dir_all(destination.join(".l2a2-acceptance")).unwrap();
        std::fs::write(
            destination.join(".l2a2-acceptance/worker-death.enabled"),
            b"enabled",
        )
        .unwrap();
        assert!(
            l2a2_acceptance_take_worker_fault(&destination, true)
                .unwrap()
                .is_some()
        );

        let outside = root
            .parent()
            .expect("L3c root parent exists")
            .join("l3a")
            .canonicalize()
            .expect("existing L3a root is a canonical path outside L3c and L3b");
        assert!(
            l2a2_acceptance_take_worker_fault(&outside, true)
                .unwrap_err()
                .starts_with("PathOutsideRuntime")
        );

        let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .canonicalize()
            .unwrap();
        for lot in ["l3c", "l3d"] {
            let canonical = repository
                .join(format!("target/parakeet-tests/l3/{lot}"))
                .canonicalize()
                .unwrap();
            assert_eq!(
                canonical_l3_acceptance_root(
                    (lot == "l3c").then(|| canonical.clone()),
                    (lot == "l3d").then(|| canonical.clone()),
                ),
                Some(canonical),
                "the test-only acceptance boundary keeps canonical {lot}"
            );
        }
    }

    #[test]
    fn over_limit_parakeet_import_is_refused_before_pending_creation() {
        if not_run_without_campaign_variables(&[
            "PARAKEET_L2_MODEL",
            "PARAKEET_L2_RUNTIME_ROOT",
            "PARAKEET_L2_EN_WAV",
            "PARAKEET_L2_WORKER",
            "PARAKEET_L2_NATIVE_DIR",
        ]) {
            return;
        }
        let model = PathBuf::from(std::env::var_os("PARAKEET_L2_MODEL").expect("R6 model path"));
        let runtime =
            PathBuf::from(std::env::var_os("PARAKEET_L2_RUNTIME_ROOT").expect("R6 runtime root"));
        let source =
            PathBuf::from(std::env::var_os("PARAKEET_L2_EN_WAV").expect("R6 short source"));
        let worker = PathBuf::from(std::env::var_os("PARAKEET_L2_WORKER").expect("R6 worker path"));
        let native_dir =
            PathBuf::from(std::env::var_os("PARAKEET_L2_NATIVE_DIR").expect("R6 native directory"));
        let destination = test_artifact_root("r6-over-limit-import");
        let mut request = test_request(JobId(0x6a31), Generation::first());
        request.source_path = source.to_string_lossy().into_owned();
        request.source_samples =
            Some(whisper_core::model_registry::PARAKEET_MAX_SINGLE_PASS_DURATION_MS * 16 + 1);
        request.model_path = model.to_string_lossy().into_owned();
        request.model_sha256 = decode_sha256(
            whisper_core::model_registry::model(
                whisper_core::model_catalog::ModelId::ParakeetTdt06BV3,
            )
            .artifact
            .sha256,
        )
        .unwrap();
        request.destination = destination.to_string_lossy().into_owned();
        request.config = JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        };
        let catalog = RuntimeModelCatalog {
            catalog: crate::model_catalog::LocalModelCatalog::new(
                model.parent().expect("model directory").to_path_buf(),
            ),
            known_historical_turbo_path: runtime
                .join("historical/ggml-large-v3-turbo.bin")
                .to_string_lossy()
                .into_owned(),
            parakeet_paths: Some(crate::parakeet_import::ParakeetWorkerPaths {
                executable: worker,
                native_dir,
                model,
                runtime_root: runtime,
            }),
            gpu_parakeet_paths: None,
        };

        let error = catalog.bind_import(&request).unwrap_err();
        assert!(
            error.contains("Fichier trop long pour Parakeet, utilisez Turbo"),
            "{error}"
        );
        assert!(
            !destination.exists(),
            "refusal must happen before archive/pending creation"
        );
    }

    #[test]
    fn injected_catalog_guards_all_worker_effect_routes_including_checkpoint_resume() {
        if not_run_without_campaign_variables(&["WHISPER_L1B_RUNTIME_ROOT"]) {
            return;
        }
        use std::{
            fs,
            path::PathBuf,
            time::{SystemTime, UNIX_EPOCH},
        };
        let root =
            PathBuf::from(std::env::var_os("WHISPER_L1B_RUNTIME_ROOT").expect("bounded runtime"))
                .join(format!(
                    "runtime-guards-{}-{}",
                    std::process::id(),
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                ));
        fs::create_dir_all(&root).unwrap();
        let gate = Arc::new(PublicationGate::default());
        let catalog = RuntimeModelCatalog {
            catalog: crate::model_catalog::LocalModelCatalog::new(root.join("empty-model-catalog")),
            known_historical_turbo_path: r"C:\known\ggml-large-v3-turbo.bin".into(),
            parakeet_paths: None,
            gpu_parakeet_paths: None,
        };
        let mut runtime = ImportRuntime::new_with_catalog(
            WorkerPaths {
                cpu: root.join("cpu.exe"),
                gpu: root.join("gpu.exe"),
            },
            gate,
            Some(catalog),
        );
        let mut events = VecDeque::new();
        let mut request = test_request(JobId(901), Generation::first());
        request.source_sha256 = Some([7; 32]);
        request.source_samples = Some(16_000);
        request.model_path = r"C:\unknown\model.bin".into();
        let destination = root.join("archive");
        request.destination = destination.to_string_lossy().into_owned();

        runtime.effect(ImportEffect::Prepare(request.clone()), &mut events);
        runtime.effect(ImportEffect::StartWorker(request.clone()), &mut events);
        runtime.effect(ImportEffect::Enqueue(request.clone()), &mut events);

        let mut queue = crate::queue_store::QueueStore::open(&destination).unwrap();
        queue.enqueue(request.clone()).unwrap();
        runtime.effect(ImportEffect::ProcessQueued(request.clone()), &mut events);

        let next_generation = request.generation.next().unwrap();
        queue
            .set_status(
                request.job_id,
                request.generation,
                crate::queue_store::QueueStatus::Interrupted,
            )
            .unwrap();
        let mut resumed = request.clone();
        resumed.generation = next_generation;
        runtime.effect(
            ImportEffect::ResumeInterrupted {
                previous: Box::new(request.clone()),
                request: resumed,
            },
            &mut events,
        );

        let live = whisper_core::ports::LiveRequest {
            job_id: JobId(902),
            generation: Generation::first(),
            model_path: request.model_path.clone(),
            model_sha256: request.model_sha256,
            destination: destination.to_string_lossy().into_owned(),
            group_offset_samples: 0,
            auto_stop_after_speech_samples: None,
            microphone: whisper_core::microphone::MicrophoneChoice::WindowsDefault,
            microphone_gain_db: 0,
            config: request.config.clone(),
        };
        runtime.effect(ImportEffect::StartLive(live.clone()), &mut events);
        runtime.effect(ImportEffect::StartDirectLive(live), &mut events);

        assert!(runtime.active.is_none());
        assert!(runtime.live.is_none());
        assert!(runtime.direct_live.is_none());
        assert_eq!(events.len(), 7);
        assert!(events.iter().all(|event| matches!(event, ImportEvent::Failed { message, .. } if message.contains("ModelUnknown"))));
    }

    #[test]
    fn resume_keeps_legacy_checkpoint_identity_and_returns_catalog_path_for_execution() {
        if not_run_without_campaign_variables(&["WHISPER_L1B_RUNTIME_ROOT"]) {
            return;
        }
        use std::{
            fs,
            path::PathBuf,
            time::{SystemTime, UNIX_EPOCH},
        };
        let root =
            PathBuf::from(std::env::var_os("WHISPER_L1B_RUNTIME_ROOT").expect("bounded runtime"))
                .join(format!(
                    "runtime-resume-{}-{}",
                    std::process::id(),
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                ));
        fs::create_dir_all(root.join("legacy")).unwrap();
        let repository_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let model = repository_root.join("models/ggml-large-v3-turbo.bin");
        let legacy_model = root.join("legacy/ggml-large-v3-turbo.bin");
        fs::hard_link(&model, &legacy_model).unwrap();
        let source = repository_root.join("sound example/spkr0.wav");
        assert!(
            source.is_file(),
            "the accepted English fixture must be present"
        );
        let destination = root.join("archive");
        let mut request = test_request(JobId(903), Generation::first());
        request.source_path = source.to_string_lossy().into_owned();
        request.model_path = legacy_model.to_string_lossy().into_owned();
        request.model_sha256 = hex_sha256(&model);
        request.destination = destination.to_string_lossy().into_owned();
        crate::archive::ArchiveStore::identify_source(&mut request).unwrap();
        let mut archive = ArchiveStore::new(&request.destination);
        archive.prepare(request.clone()).unwrap();
        archive
            .persist_segment(&whisper_core::ports::WorkerSegment {
                job_id: request.job_id,
                generation: request.generation,
                instance_id: 1,
                segment_id: whisper_core::SegmentId(1),
                range: whisper_core::SourceRange::new(0, 16_000, 16_000).unwrap(),
                text: "confirmed prefix".into(),
            })
            .unwrap();
        let mut queue = crate::queue_store::QueueStore::open(&destination).unwrap();
        queue.enqueue(request.clone()).unwrap();
        queue
            .set_status(
                request.job_id,
                request.generation,
                crate::queue_store::QueueStatus::Interrupted,
            )
            .unwrap();
        let mut resumed = request.clone();
        resumed.generation = request.generation.next().unwrap();

        let mut runtime = ImportRuntime::new_with_catalog(
            WorkerPaths {
                cpu: root.join("cpu.exe"),
                gpu: root.join("gpu.exe"),
            },
            Arc::new(PublicationGate::default()),
            Some(RuntimeModelCatalog {
                catalog: crate::model_catalog::LocalModelCatalog::new(
                    repository_root.join("models"),
                ),
                known_historical_turbo_path: legacy_model.to_string_lossy().into_owned(),
                parakeet_paths: None,
                gpu_parakeet_paths: None,
            }),
        );
        let mut events = VecDeque::new();
        runtime.effect(
            ImportEffect::ResumeInterrupted {
                previous: Box::new(request.clone()),
                request: resumed.clone(),
            },
            &mut events,
        );
        let checkpoint = events.iter().find_map(|event| match event {
            ImportEvent::ResumeCheckpoint {
                confirmed_segments,
                confirmed_offset,
                ..
            } => Some((*confirmed_segments, *confirmed_offset)),
            _ => None,
        });
        assert_eq!(checkpoint, Some((1, 16_000)));
        let prepared = events
            .iter()
            .find_map(|event| match event {
                ImportEvent::Prepared(request) => Some(request),
                _ => None,
            })
            .expect("verified resume emits a worker request");
        assert_eq!(
            PathBuf::from(&prepared.model_path).canonicalize().unwrap(),
            model.canonicalize().unwrap()
        );
        assert_eq!(prepared.generation, resumed.generation);
        let queue = crate::queue_store::QueueStore::open(&destination).unwrap();
        assert_eq!(queue.entries()[0].request.model_path, request.model_path);
        let pending_path = destination
            .join("transcriptions")
            .join(format!("{:032x}", request.job_id.0))
            .join(resumed.generation.get().to_string())
            .join("pending.json");
        let pending: serde_json::Value =
            serde_json::from_slice(&fs::read(pending_path).unwrap()).unwrap();
        assert_eq!(pending["model_path"], request.model_path);
    }

    fn hex_sha256(path: &std::path::Path) -> [u8; 32] {
        let mut file = std::fs::File::open(path).unwrap();
        let mut hasher = sha2::Sha256::new();
        let mut buffer = [0; 1024 * 1024];
        loop {
            let count = std::io::Read::read(&mut file, &mut buffer).unwrap();
            if count == 0 {
                break;
            }
            hasher.update(&buffer[..count]);
        }
        let digest = hasher.finalize();
        digest.into()
    }

    #[test]
    fn stop_during_blocked_commit_returns_promptly_and_reports_finalizing() {
        use std::{
            fs::{self, OpenOptions},
            io::Write,
            sync::{Barrier, mpsc},
            time::{SystemTime, UNIX_EPOCH},
        };

        let gate = Arc::new(PublicationGate::default());
        let (io, effects, stops, event_tx, _, _, _) = AsyncImportIo::for_test(gate.clone());
        let mut app = ImportApplication::new(io);
        let job_id = JobId(88);
        let generation = Generation::first();
        app.dispatch(AppCommand::StartImport {
            request: ImportRequest {
                strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
                job_id,
                generation,
                source_path: "fixture.wav".into(),
                source_sha256: None,
                source_samples: None,
                model_path: "model.bin".into(),
                model_sha256: [0; 32],
                destination: "archive".into(),
                config: JobConfig {
                    language: LanguageChoice::Manual("fr".into()),
                    compute: ComputeChoice::Cpu,
                },
            },
        })
        .unwrap();
        assert!(matches!(effects.try_recv(), Ok(ImportEffect::Prepare(_))));
        let commit_directory = std::env::temp_dir().join(format!(
            "whisper-stop-commit-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&commit_directory).unwrap();
        let staged_pointer = commit_directory.join("CURRENT.tmp");
        let current_pointer = commit_directory.join("CURRENT");

        let commit_entered = Arc::new(Barrier::new(2));
        let allow_disk_commit = Arc::new(Barrier::new(2));
        let publisher = {
            let gate = gate.clone();
            let commit_entered = commit_entered.clone();
            let allow_disk_commit = allow_disk_commit.clone();
            let staged_pointer = staged_pointer.clone();
            let current_pointer = current_pointer.clone();
            thread::spawn(move || {
                gate.commit(|| {
                    commit_entered.wait();
                    allow_disk_commit.wait();
                    let mut file = OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(&staged_pointer)
                        .map_err(|error| error.to_string())?;
                    file.write_all(b"manifest.json\n")
                        .map_err(|error| error.to_string())?;
                    file.sync_all().map_err(|error| error.to_string())?;
                    drop(file);
                    fs::rename(staged_pointer, current_pointer)
                        .map_err(|error| error.to_string())?;
                    Ok(())
                })
            })
        };
        commit_entered.wait();

        let (result_tx, result_rx) = mpsc::channel();
        let (finish_ui_tx, finish_ui_rx) = mpsc::channel();
        let (complete_tx, complete_rx) = mpsc::channel();
        let ui = thread::spawn(move || {
            result_tx
                .send(app.dispatch(AppCommand::Stop { job_id, generation }))
                .unwrap();
            finish_ui_rx.recv().unwrap();
            event_tx
                .send(ImportEvent::Published { job_id, generation })
                .unwrap();
            complete_tx.send(app.dispatch(AppCommand::Refresh)).unwrap();
        });
        let immediate_result = result_rx.recv_timeout(Duration::from_millis(200));
        allow_disk_commit.wait();
        publisher.join().unwrap().unwrap();
        assert!(
            immediate_result.is_ok(),
            "Stop submission must not wait for disk commit"
        );
        let view = immediate_result.unwrap().unwrap();
        assert_eq!(view.active_job, Some((job_id, JobState::Finalizing)));
        assert!(
            view.message
                .as_deref()
                .is_some_and(|message| message.contains("trop tardif"))
        );
        assert!(matches!(stops.try_recv(), Err(TryRecvError::Empty)));
        assert_eq!(fs::read(&current_pointer).unwrap(), b"manifest.json\n");
        finish_ui_tx.send(()).unwrap();
        let complete = complete_rx
            .recv_timeout(Duration::from_secs(1))
            .unwrap()
            .unwrap();
        assert_eq!(complete.active_job, Some((job_id, JobState::Complete)));
        assert_eq!(gate.0.load(Ordering::Acquire), GATE_COMMITTED);
        ui.join().unwrap();
        fs::remove_file(current_pointer).unwrap();
        fs::remove_dir(commit_directory).unwrap();
    }
}
