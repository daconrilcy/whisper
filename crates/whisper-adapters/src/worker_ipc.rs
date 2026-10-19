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
    collections::VecDeque,
    fs,
    io::{self, BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        Arc,
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
    ArchiveHistoryItem, DiagnosticNotice, Generation, ImportEffect, ImportEvent, ImportIoPort,
    JobId,
    ipc::{BackendKind, IPC_PROTOCOL_VERSION, IpcEnvelope, WorkerCommand, WorkerEvent},
    ports::{
        DecodeRequest, DecoderPort, ImportRequest, LiveRequest, PortError, SourceIdentity,
        WorkerSegment,
    },
};

const MAX_FRAME_BYTES: usize = 1_048_576;
const TDT_FRAME_OVERLAP_SAMPLES: u64 = 80 * 16;

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
    let row = serde_json::json!({
        "process": "adapter",
        "pid": std::process::id(),
        "at_ms": at_ms,
        "stage": stage,
        "elapsed_ms": elapsed.as_secs_f64() * 1000.0,
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
            diagnostic: item.diagnostic,
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
        })
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
    active: Option<ImportRequestState>,
    transport: Option<ChildWorkerTransport>,
    publication_gate: Arc<PublicationGate>,
    resume_prefix: VecDeque<crate::journal::SegmentRecord>,
    pending_live: Option<PendingLiveState>,
    live: Option<LiveCaptureState>,
    live_archive: Option<LiveArchive>,
    direct_live: Option<crate::direct_live::DirectLive>,
    pending_attempt_reset: Option<ImportEvent>,
    pending_recovery_counters: Option<ImportEvent>,
    live_pending_persistence: usize,
    deferred_live_stopped: Option<WorkerEvent>,
    deferred_live_window_finished: Option<FinishedLiveWindow>,
    gpu_candidates: std::collections::HashMap<
        (JobId, u64),
        Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
    >,
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

fn archive_qualification_for_request(
    request: &ImportRequest,
    parakeet_paths: Option<&crate::parakeet_import::ParakeetWorkerPaths>,
    gpu_parakeet_paths: Option<&crate::parakeet_import::ParakeetWorkerPaths>,
    gpu_candidate: Option<&whisper_core::gpu_inventory::GpuRuntimeIdentity>,
) -> Option<crate::archive::ArchiveQualificationKey> {
    if !request_is_parakeet(request) {
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
            (profile == whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS)
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

fn gpu_evidence_matches(
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

impl ImportRuntime {
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
        Self {
            workers,
            model_catalog,
            parakeet_paths,
            gpu_parakeet_paths,
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
            active: None,
            transport: None,
            publication_gate,
            resume_prefix: VecDeque::new(),
            pending_live: None,
            live: None,
            live_archive: None,
            direct_live: None,
            pending_attempt_reset: None,
            pending_recovery_counters: None,
            live_pending_persistence: 0,
            deferred_live_stopped: None,
            deferred_live_window_finished: None,
            gpu_candidates: std::collections::HashMap::new(),
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
    fn effect(&mut self, effect: ImportEffect, events: &mut VecDeque<ImportEvent>) {
        if matches!(&effect, ImportEffect::StartLive(_)) {
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
            ImportEffect::StartDirectLive(request) => {
                let id = (request.job_id, request.generation);
                let candidate = self.gpu_candidates.remove(&(id.0, id.1.get())).flatten();
                if self.publication_gate.is_cancelled() {
                    Ok(())
                } else if self.active.is_some() || self.live.is_some() || self.direct_live.is_some()
                {
                    Err((id.0, id.1, "Une transcription est déjà en cours".into()))
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
            ImportEffect::Enqueue(mut request) => {
                let id = (request.job_id, request.generation);
                let result = self
                    .bind_import_request(&request)
                    .map(|_| ())
                    .and_then(|()| crate::archive::ArchiveStore::identify_source(&mut request))
                    .and_then(|()| self.bind_import_request(&request).map(|_| ()))
                    .and_then(|()| {
                        crate::archive::l04_campaign_assert_archive(Path::new(&request.destination))
                    })
                    .and_then(|()| crate::queue_store::QueueStore::open(&request.destination))
                    .and_then(|mut queue| {
                        queue.enqueue(request)?;
                        Ok(queue.entries())
                    });
                match result {
                    Ok(queue) => {
                        events.push_back(ImportEvent::Queue(queue));
                        Ok(())
                    }
                    Err(error) => Err((id.0, id.1, error)),
                }
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
                        events.push_back(ImportEvent::Queue(queue));
                        Ok(())
                    }
                    Err(error) => Err((job_id, generation, error)),
                }
            }
            ImportEffect::ProcessQueued(request) => {
                let id = (request.job_id, request.generation);
                let model_catalog = self.model_catalog.as_ref().map(|catalog| {
                    (
                        catalog.snapshot(),
                        catalog.known_historical_turbo_path.clone(),
                    )
                });
                let result =
                    crate::archive::l04_campaign_assert_archive(Path::new(&request.destination))
                        .and_then(|()| crate::queue_store::QueueStore::open(&request.destination))
                        .and_then(|queue| {
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
                            let mut comparable = entry.request.clone();
                            let same_model = if request_is_parakeet(&entry.request)
                                && request_is_parakeet(&request)
                            {
                                comparable.model_path = request.model_path.clone();
                                entry.request.model_sha256 == request.model_sha256
                            } else if let Some((snapshot, known_path)) = &model_catalog {
                                let durable_id = snapshot.resolve_model_identity(
                                    &entry.request.model_path,
                                    known_path,
                                )?;
                                let requested_id = snapshot
                                    .resolve_model_identity(&request.model_path, known_path)?;
                                comparable.model_path = request.model_path.clone();
                                durable_id == requested_id
                            } else {
                                false
                            };
                            if entry.request != request && (!same_model || comparable != request) {
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
                            let durable_request = entry.request;
                            if request_is_parakeet(&durable_request) {
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
                    Ok((durable_request, verified_executable)) => {
                        let executable_model_path = model_catalog
                            .as_ref()
                            .map(|_| verified_executable.model_path.as_str());
                        let mut archive = ArchiveStore::new(&durable_request.destination);
                        match archive.prepare_with_qualification(
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
                        ) {
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
                            Err(message) => Err((id.0, id.1, message)),
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
            ImportEffect::ScanHistory { destination } => {
                let root = PathBuf::from(destination);
                events.push_back(ImportEvent::History(scan_archive_history(&root)));
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
            ImportEffect::Stop { job_id, generation } => {
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
                                    events.push_back(ImportEvent::Stopped { job_id, generation });
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
                                events
                                    .push_back(ImportEvent::LiveFinalizing { job_id, generation });
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
                            events.push_back(ImportEvent::LiveFinalizing { job_id, generation });
                        }
                    })
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
        if let Err((job, generation, message)) = result {
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
            if let Some(live) = self.live.as_mut()
                && (live.request.job_id, live.request.generation) == (job, generation)
            {
                deferred_live_failure = true;
                live.capture.stop();
                let _ = live.staging.drain();
                live.failure = Some(message.clone());
                if let Some(transport) = self.transport.as_mut() {
                    let _ = transport.try_request(WorkerCommand::Stop {
                        job_id: job,
                        generation,
                    });
                }
            }
            if !deferred_live_failure {
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
        let mut capture =
            crate::capture::start_default().map_err(|error| (job_id, generation, error))?;
        let converter = match Pcm16Converter::new(capture.sample_rate_hz(), capture.channels()) {
            Ok(converter) => converter,
            Err(error) => {
                capture.stop();
                return Err((job_id, generation, error));
            }
        };
        let mut live_archive = pending.live_archive;
        if let Err(error) = live_archive.capture_started() {
            capture.stop();
            return Err((job_id, generation, error));
        }
        self.live_archive = Some(live_archive);
        self.live = Some(LiveCaptureState {
            request: pending.request,
            capture,
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
        request: ImportRequest,
        candidate: Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
        events: &mut VecDeque<ImportEvent>,
    ) -> Result<(), (JobId, Generation, String)> {
        let id = (request.job_id, request.generation);
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
        match archive.prepare_with_qualification(request, None, qualification) {
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
                    live.capture.stop();
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
                whisper_core::BackendDecision::RetryCpu => {
                    ImportEvent::LiveRetryingOnCpu { job_id, generation }
                }
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
                    live.capture.stop();
                }
                whisper_core::BackendDecision::Fail => {
                    live.capture.stop();
                    let _ = live.staging.drain();
                    live.failure = Some(message.clone());
                }
                whisper_core::BackendDecision::Start(_) => {
                    live.capture.stop();
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
            return match decision {
                whisper_core::BackendDecision::RetryCpu => {
                    Ok(ImportEvent::LiveRetryingOnCpu { job_id, generation })
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
            self.publication_gate
                .commit(|| {
                    self.archive.commit_publish(job, generation)?;
                    l2a2_acceptance_barrier(
                        "publish-after-commit",
                        self.archive.root_path().as_path(),
                    )
                })
                .map_err(|e| (job, generation, e))?;
            let root = self.archive_root();
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
            events.push_back(ImportEvent::Published {
                job_id: job,
                generation,
            });
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
            live.capture.stop();
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
            live.capture.stop();
            live.failure = Some(reason.into());
            if let Some(transport) = self.transport.as_mut() {
                let _ = transport.try_request(WorkerCommand::Stop { job_id, generation });
            }
            return None;
        }
        let block = live.capture.try_next_block()?;
        let converted = match live.converter.push(&block) {
            Ok(samples) => samples,
            Err(error) => {
                live.capture.stop();
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
            live.capture.stop();
            live.failure = Some(error.clone());
            if let Some(transport) = self.transport.as_mut() {
                let _ = transport.try_request(WorkerCommand::Stop {
                    job_id: live.request.job_id,
                    generation: live.request.generation,
                });
            }
            return None;
        }
        live.frame_pending.extend(converted);
        while live.frame_pending.len() >= 320 {
            let frame: Vec<i16> = live.frame_pending.drain(..320).collect();
            live.captured = live.captured.saturating_add(320);
            match live.vad.is_speech(&frame) {
                Ok(true) => live.speech = live.speech.saturating_add(320),
                Ok(false) => {}
                Err(error) => {
                    live.capture.stop();
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
                live.capture.stop();
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
                        live.capture.stop();
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
                live.capture.stop();
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
                    live.capture.stop();
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
        if let Some(event) = self.poll_live_capture() {
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
            match transport.receive() {
                Ok(Some(event)) => event,
                Ok(None) => {
                    let _ = transport.try_wait();
                    return None;
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
                        live.capture.stop();
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
                        live.capture.stop();
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
                    live.capture.stop();
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
                    live.capture.stop();
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
                        live.capture.stop();
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
                        live.capture.stop();
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
                    live.capture.stop();
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
            live.capture.stop();
            let _ = live.staging.drain();
            live.failure = Some(message.clone());
            if let Some(transport) = self.transport.as_mut() {
                let _ = transport.try_request(WorkerCommand::Stop { job_id, generation });
            }
            return None;
        }
        Some(result)
    }
}

pub struct AsyncImportIo {
    effects: SyncSender<ImportEffect>,
    stops: SyncSender<ImportEffect>,
    events: Receiver<ImportEvent>,
    publication_gate: Arc<PublicationGate>,
    active_identity: Option<(JobId, Generation)>,
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
        Self::start_inner(workers, None, None, None)
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
        )
    }

    fn start_inner(
        workers: WorkerPaths,
        model_catalog: Option<RuntimeModelCatalog>,
        parakeet_paths: Option<crate::parakeet_import::ParakeetWorkerPaths>,
        gpu_parakeet_paths: Option<crate::parakeet_import::ParakeetWorkerPaths>,
    ) -> Self {
        let (effect_tx, effect_rx) = mpsc::sync_channel(8);
        let (stop_tx, stop_rx) = mpsc::sync_channel(1);
        let (event_tx, event_rx) = mpsc::sync_channel(EVENTS_PER_STAGE);
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
        thread::spawn({
            let publication_gate = publication_gate.clone();
            move || {
                let runtime = ImportRuntime::new_with_workers_and_parakeet(
                    workers,
                    publication_gate,
                    model_catalog,
                    parakeet_paths,
                    gpu_parakeet_paths,
                );
                service_loop(effect_rx, stop_rx, event_tx, runtime)
            }
        });
        Self {
            effects: effect_tx,
            stops: stop_tx,
            events: event_rx,
            publication_gate,
            active_identity: None,
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
    ) {
        let (effect_tx, effect_rx) = mpsc::sync_channel(8);
        let (stop_tx, stop_rx) = mpsc::sync_channel(1);
        let (event_tx, event_rx) = mpsc::sync_channel(EVENTS_PER_STAGE);
        (
            Self {
                effects: effect_tx,
                stops: stop_tx,
                events: event_rx,
                publication_gate,
                active_identity: None,
            },
            effect_rx,
            stop_rx,
            event_tx,
        )
    }
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
        if let ImportEffect::Stop { job_id, generation } = &effect {
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
            if let ImportEffect::FinishLiveOnCpu { job_id, generation } = &effect
                && self.active_identity != Some((*job_id, *generation))
            {
                return Err(PortError::StaleResponse);
            }
            let active_identity = match &effect {
                ImportEffect::StartLive(request) | ImportEffect::StartDirectLive(request) => {
                    Some((request.job_id, request.generation))
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
        match self.events.try_recv() {
            Ok(event) => Ok(Some(event)),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => Err(PortError::Unavailable),
        }
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
        self.effect(effect, pending);
    }

    fn poll_event(&mut self) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
        self.poll()
    }

    fn pump_realtime(&mut self) -> Option<(JobId, Generation, String)> {
        let direct = self.direct_live.as_mut()?;
        let (job_id, generation) = direct.identity();
        match direct.pump_capture() {
            Ok(()) => None,
            Err(error) => {
                let error = direct.fail(error);
                self.direct_live = None;
                Some((job_id, generation, error))
            }
        }
    }
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
        ImportEffect::Stop { .. } => 2,
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
        let mut stop = deferred_stop.take().or_else(|| stop_rx.try_recv().ok());
        if stop.is_some() && pending.len() + 2 > EVENTS_PER_STAGE {
            deferred_stop = stop.take();
        }
        let mut effect = deferred_effect.take().or_else(|| effect_rx.try_recv().ok());
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
                Some(effect @ (ImportEffect::StartLive(_) | ImportEffect::StartDirectLive(_))),
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
    use whisper_core::{
        AppCommand, Application, ComputeChoice, Generation, ImportApplication, ImportRequest,
        JobConfig, JobId, JobState, LanguageChoice,
    };

    fn test_artifact_root(name: &str) -> PathBuf {
        let base = std::env::var_os("PARAKEET_R6_TEST_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target"));
        base.join(format!("{name}-{}", std::process::id()))
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
            pnp_instance_id: "PCI\\VEN_10DE&DEV_1B06".into(),
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
        let (mut io, effects, _, _) = AsyncImportIo::for_test(gate);
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
        let (mut io, effects, _, _) = AsyncImportIo::for_test(gate);
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
        let (mut disconnected, effects, _, _) = AsyncImportIo::for_test(gate);
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

        let (mut io, effect_rx, stop_rx, app_tx) =
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
        let (mut io, effects, stops, _) = AsyncImportIo::for_test(gate);
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
        let (mut io, _effects, _, _) = AsyncImportIo::for_test(gate);
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
        let (io, effects, stops, event_tx) = AsyncImportIo::for_test(gate.clone());
        let mut app = ImportApplication::new(io);
        let job_id = JobId(88);
        let generation = Generation::first();
        app.dispatch(AppCommand::StartImport {
            request: ImportRequest {
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
