use crate::{WorkerTransport, archive::ArchiveStore};
use serde::{Serialize, de::DeserializeOwned};
use std::{
    collections::VecDeque,
    io::{self, BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicU8, AtomicU64, Ordering},
        mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError},
    },
    thread,
    time::Duration,
};
#[cfg(feature = "l01-memory-qualification")]
use std::{
    fs::{File, OpenOptions},
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};
use whisper_core::{
    ArchiveHistoryItem, Generation, ImportEffect, ImportEvent, ImportIoPort, JobId,
    ipc::{BackendKind, IPC_PROTOCOL_VERSION, IpcEnvelope, WorkerCommand, WorkerEvent},
    ports::{ImportRequest, PortError, WorkerSegment},
};

const MAX_FRAME_BYTES: usize = 1_048_576;
const EVENTS_PER_STAGE: usize = 21;
const GATE_IDLE: u8 = 0;
const GATE_ACTIVE: u8 = 1;
const GATE_CANCELLED: u8 = 2;
const GATE_COMMITTING: u8 = 3;
const GATE_COMMITTED: u8 = 4;
static NEXT_INSTANCE: AtomicU64 = AtomicU64::new(1);

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
    stdin: ChildStdin,
    events: Receiver<Result<WorkerEvent, PortError>>,
}

impl ChildWorkerTransport {
    pub fn spawn(executable: &Path) -> Result<Self, PortError> {
        let mut child = Command::new(executable)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| PortError::Failed(format!("WorkerExited: {e}")))?;
        let stdin = child.stdin.take().ok_or(PortError::Unavailable)?;
        let stdout = child.stdout.take().ok_or(PortError::Unavailable)?;
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
                    "stdin_pipe_handle": stdin.as_raw_handle() as usize,
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
            stdin,
            events,
        })
    }

    pub fn try_wait(&mut self) -> Result<Option<std::process::ExitStatus>, PortError> {
        self.child
            .try_wait()
            .map_err(|e| PortError::Failed(format!("WorkerExited: {e}")))
    }
}

impl WorkerTransport for ChildWorkerTransport {
    fn request(&mut self, command: WorkerCommand) -> Result<(), PortError> {
        let envelope = IpcEnvelope::new(command);
        write_frame(&mut self.stdin, &envelope)
            .map_err(|e| PortError::Failed(format!("ProtocolMismatch: {e}")))
    }

    fn receive(&mut self) -> Result<Option<WorkerEvent>, PortError> {
        match self.events.try_recv() {
            Ok(Ok(event)) => Ok(Some(event)),
            Ok(Err(error)) => Err(error),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => {
                Err(PortError::Failed("WorkerExited without End".into()))
            }
        }
    }
}

fn read_events(stdout: impl io::Read, tx: SyncSender<Result<WorkerEvent, PortError>>) {
    let mut input = BufReader::new(stdout);
    #[cfg(feature = "l01-memory-qualification")]
    memory_trace(
        "adapter.worker_stdout_reader",
        serde_json::json!({ "bufreader_capacity_bytes": input.capacity() }),
    );
    loop {
        let result = read_frame::<IpcEnvelope<WorkerEvent>>(&mut input).and_then(|item| {
            item.ok_or_else(|| {
                io::Error::new(io::ErrorKind::UnexpectedEof, "worker exited without End")
            })
        });
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
        let terminal = event.is_err()
            || matches!(
                event,
                Ok(WorkerEvent::End { .. }
                    | WorkerEvent::Stopped { .. }
                    | WorkerEvent::Failed { .. })
            );
        if tx.send(event).is_err() || terminal {
            break;
        }
    }
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
    let bytes = serde_json::to_vec(value).map_err(io::Error::other)?;
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
    if bytes.len() + 1 > MAX_FRAME_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "IPC frame exceeds 1 MiB",
        ));
    }
    output.write_all(&bytes)?;
    output.write_all(b"\n")?;
    output.flush()
}

struct ImportRuntime {
    worker_path: PathBuf,
    archive: ArchiveStore,
    active: Option<ImportRequestState>,
    transport: Option<ChildWorkerTransport>,
    publication_gate: Arc<PublicationGate>,
}
struct ImportRequestState {
    job_id: JobId,
    generation: Generation,
    instance_id: u64,
}

impl ImportRuntime {
    fn new(worker_path: PathBuf, publication_gate: Arc<PublicationGate>) -> Self {
        Self {
            worker_path,
            archive: ArchiveStore::new("."),
            active: None,
            transport: None,
            publication_gate,
        }
    }
    fn effect(&mut self, effect: ImportEffect, events: &mut VecDeque<ImportEvent>) {
        let result = match effect {
            ImportEffect::Enqueue(request) => {
                let id = (request.job_id, request.generation);
                match crate::queue_store::QueueStore::open(&request.destination).and_then(
                    |mut queue| {
                        queue.enqueue(request)?;
                        Ok(queue.entries())
                    },
                ) {
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
                match crate::queue_store::QueueStore::open(&destination).and_then(|mut queue| {
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
                let result = crate::queue_store::QueueStore::open(&request.destination)
                    .and_then(|mut queue| {
                        let entries = queue.entries();
                        let first = entries.first().map(|entry| (entry.request.job_id, entry.request.generation));
                        let entry = entries.into_iter().find(|entry| {
                            entry.request.job_id == id.0 && entry.request.generation == id.1
                        }).ok_or_else(|| "InvalidInput: queue entry no longer exists".to_owned())?;
                        if first != Some(id) {
                            return Err("InvalidInput: process queued jobs in FIFO order".into());
                        }
                        if !matches!(entry.status, crate::queue_store::QueueStatus::Queued | crate::queue_store::QueueStatus::AwaitingChoice) {
                            return Err("InvalidInput: interrupted jobs require an explicit verified resume".into());
                        }
                        queue.set_status(id.0, id.1, crate::queue_store::QueueStatus::Running)?;
                        events.push_back(ImportEvent::Queue(queue.entries()));
                        Ok(())
                    });
                match result {
                    Ok(()) => self.prepare(request, events),
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
            ImportEffect::Prepare(request) => self.prepare(request, events),
            ImportEffect::StartWorker(request) => self.start_worker(request),
            ImportEffect::ScanHistory { destination } => {
                let root = PathBuf::from(destination);
                match ArchiveStore::scan(&root) {
                    Ok(entries) => {
                        events.push_back(ImportEvent::History(
                            entries
                                .into_iter()
                                .map(|item| ArchiveHistoryItem {
                                    job_id: item.job_id,
                                    generation: item.generation,
                                    complete: item.complete,
                                    source_sha256: item.source_sha256,
                                    diagnostic: item.diagnostic,
                                })
                                .collect(),
                        ));
                        Ok(())
                    }
                    Err(message) => {
                        events.push_back(ImportEvent::History(vec![ArchiveHistoryItem {
                            job_id: JobId(0),
                            generation: Generation::first(),
                            complete: false,
                            source_sha256: String::new(),
                            diagnostic: Some(format!("StorageCorrupt: {message}")),
                        }]));
                        Ok(())
                    }
                }
            }
            ImportEffect::PersistSegment(segment) => self.persist_segment(segment, events),
            ImportEffect::Publish {
                job_id,
                generation,
                last_sequence,
            } => self.publish(job_id, generation, last_sequence, events),
            ImportEffect::Stop { job_id, generation } => self.stop(job_id, generation, events),
        };
        if let Err((job, generation, message)) = result {
            events.push_back(ImportEvent::Failed {
                job_id: job,
                generation,
                message,
            });
        }
    }
    fn prepare(
        &mut self,
        request: ImportRequest,
        events: &mut VecDeque<ImportEvent>,
    ) -> Result<(), (JobId, Generation, String)> {
        let id = (request.job_id, request.generation);
        if self.publication_gate.is_cancelled() {
            events.push_back(ImportEvent::Stopped {
                job_id: id.0,
                generation: id.1,
            });
            return Ok(());
        }
        let mut archive = ArchiveStore::new(&request.destination);
        match archive.prepare(request) {
            Ok(prepared) => {
                self.archive = archive;
                events.push_back(ImportEvent::Prepared(prepared));
                Ok(())
            }
            Err(message) => Err((id.0, id.1, message)),
        }
    }
    fn start_worker(
        &mut self,
        request: whisper_core::ports::ImportRequest,
    ) -> Result<(), (JobId, Generation, String)> {
        let id = (request.job_id, request.generation);
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
        let mut transport = ChildWorkerTransport::spawn(&self.worker_path)
            .map_err(|e| (id.0, id.1, format!("WorkerExited: {e:?}")))?;
        let instance_id = NEXT_INSTANCE.fetch_add(1, Ordering::Relaxed);
        if instance_id == 0 {
            return Err((
                id.0,
                id.1,
                "WorkerExited: instance identity exhausted".into(),
            ));
        }
        transport
            .request(WorkerCommand::StartImport {
                job_id: id.0,
                generation: id.1,
                instance_id,
                source_path: request.source_path,
                source_sha256,
                expected_source_samples,
                model_path: request.model_path,
                model_sha256,
                config: request.config,
            })
            .map_err(|e| (id.0, id.1, format!("ProtocolMismatch: {e:?}")))?;
        self.active = Some(ImportRequestState {
            job_id: id.0,
            generation: id.1,
            instance_id,
        });
        self.transport = Some(transport);
        Ok(())
    }
    fn persist_segment(
        &mut self,
        segment: WorkerSegment,
        events: &mut VecDeque<ImportEvent>,
    ) -> Result<(), (JobId, Generation, String)> {
        let id = (segment.job_id, segment.generation);
        let sequence = self
            .archive
            .persist_segment(&segment)
            .map_err(|e| (id.0, id.1, e))?;
        events.push_back(ImportEvent::Persisted {
            job_id: id.0,
            generation: id.1,
            sequence,
        });
        Ok(())
    }
    fn publish(
        &mut self,
        job: JobId,
        generation: Generation,
        count: u64,
        events: &mut VecDeque<ImportEvent>,
    ) -> Result<(), (JobId, Generation, String)> {
        self.archive
            .prepare_publish(job, generation, count)
            .map_err(|e| (job, generation, e))?;
        self.publication_gate
            .commit(|| self.archive.commit_publish(job, generation))
            .map_err(|e| (job, generation, e))?;
        let root = self.archive_root();
        if let Ok(mut queue) = crate::queue_store::QueueStore::open(&root) {
            if queue.remove(job, generation).is_ok() {
                events.push_back(ImportEvent::Queue(queue.entries()));
            }
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
                "StorageCorrupt: archive scan did not verify stable published generation".into(),
            ));
        }
        events.push_back(ImportEvent::Published {
            job_id: job,
            generation,
        });
        Ok(())
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
    fn poll(&mut self) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
        let transport = self.transport.as_mut()?;
        let event = match transport.receive() {
            Ok(Some(event)) => event,
            Ok(None) => {
                let _ = transport.try_wait();
                return None;
            }
            Err(error) => {
                let active = self.active.as_ref()?;
                let failure = Err((
                    active.job_id,
                    active.generation,
                    format!("WorkerExited without End: {error:?}"),
                ));
                self.transport = None;
                self.active = None;
                return Some(failure);
            }
        };
        let active = self.active.as_ref()?;
        let (job_id, generation, instance_id) =
            (active.job_id, active.generation, active.instance_id);
        let result = match event {
            WorkerEvent::Ready {
                job_id: j,
                generation: g,
                instance_id: i,
                backend: BackendKind::Cpu,
            } if (j, g, i) == (job_id, generation, instance_id) => Ok(ImportEvent::Ready {
                job_id: j,
                generation: g,
                backend: "CPU".into(),
            }),
            WorkerEvent::Ready { .. } => Err((
                job_id,
                generation,
                "BackendMismatch: worker did not attest CPU for this instance".into(),
            )),
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
                Ok(ImportEvent::Segment(WorkerSegment {
                    job_id: dto.job_id,
                    generation: dto.generation,
                    instance_id: dto.instance_id,
                    segment_id: dto.segment_id,
                    range: dto.range,
                    text: dto.text,
                }))
            }
            WorkerEvent::End {
                job_id: j,
                generation: g,
                instance_id: i,
                last_sequence,
                last_offset,
            } if (j, g, i) == (job_id, generation, instance_id) => {
                self.transport = None;
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
                ..
            } if (j, g, i) == (job_id, generation, instance_id) => {
                self.transport = None;
                self.active = None;
                Err((j, g, message))
            }
            WorkerEvent::Failed { message, .. } => {
                self.transport = None;
                self.active = None;
                Err((job_id, generation, message))
            }
            _ => Err((
                job_id,
                generation,
                "StaleResponse: worker event identity mismatch".into(),
            )),
        };
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
                let runtime = ImportRuntime::new(worker_path, publication_gate);
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
        let encoded = serde_json::to_vec(&effect).map_err(|e| PortError::Failed(e.to_string()))?;
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
        if encoded.len() + 1 > MAX_FRAME_BYTES {
            return Err(PortError::InvalidInput);
        }
        if let ImportEffect::Stop { job_id, generation } = &effect {
            if self.active_identity != Some((*job_id, *generation)) {
                return Err(PortError::StaleResponse);
            }
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
            let active_identity = match &effect {
                ImportEffect::Prepare(request) | ImportEffect::ProcessQueued(request) => {
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
            self.effects.try_send(effect).map_err(|e| match e {
                TrySendError::Full(_) => PortError::Unavailable,
                TrySendError::Disconnected(_) => PortError::Unavailable,
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
}

impl ServiceRuntime for ImportRuntime {
    fn apply_effect(&mut self, effect: ImportEffect, pending: &mut VecDeque<ImportEvent>) {
        self.effect(effect, pending);
    }

    fn poll_event(&mut self) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
        self.poll()
    }
}

fn service_loop<R: ServiceRuntime>(
    effect_rx: Receiver<ImportEffect>,
    stop_rx: Receiver<ImportEffect>,
    event_tx: SyncSender<ImportEvent>,
    mut runtime: R,
) {
    let mut pending = VecDeque::new();
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
        if let Ok(stop) = stop_rx.try_recv() {
            runtime.apply_effect(stop, &mut pending);
            #[cfg(feature = "l01-memory-qualification")]
            memory_trace(
                "adapter.pending.after_stop_effect",
                serde_json::json!({
                    "pending_len": pending.len(),
                    "pending_capacity": pending.capacity(),
                    "pending_slot_bytes": std::mem::size_of::<ImportEvent>(),
                }),
            );
        }
        if pending.len() < EVENTS_PER_STAGE
            && let Ok(effect) = effect_rx.try_recv()
        {
            runtime.apply_effect(effect, &mut pending);
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
    use std::io::Cursor;
    use whisper_core::{
        AppCommand, Application, ComputeChoice, Generation, ImportApplication, ImportRequest,
        JobConfig, JobId, JobState, LanguageChoice,
    };

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
    fn event_65_backpressures_all_production_stages_then_delivers_in_order() {
        struct WireRuntime {
            worker_events: Receiver<Result<WorkerEvent, PortError>>,
            consumed: Arc<std::sync::atomic::AtomicUsize>,
            shutdown: Receiver<()>,
        }
        impl ServiceRuntime for WireRuntime {
            fn apply_effect(&mut self, _: ImportEffect, _: &mut VecDeque<ImportEvent>) {}

            fn poll_event(&mut self) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
                match self.worker_events.try_recv() {
                    Ok(Ok(WorkerEvent::Progress {
                        job_id,
                        generation,
                        completed_samples,
                        total_samples,
                        ..
                    })) => {
                        self.consumed.fetch_add(1, Ordering::AcqRel);
                        Some(Ok(ImportEvent::Progress {
                            job_id,
                            generation,
                            completed_samples,
                            total_samples,
                        }))
                    }
                    Ok(Ok(WorkerEvent::Failed {
                        job_id: Some(job_id),
                        generation: Some(generation),
                        message,
                        ..
                    })) => {
                        self.consumed.fetch_add(1, Ordering::AcqRel);
                        Some(Err((job_id, generation, message)))
                    }
                    Ok(Err(error)) => Some(Err((
                        JobId(903),
                        Generation::first(),
                        format!("worker reader: {error:?}"),
                    ))),
                    Ok(Ok(_)) => None,
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
            serde_json::to_vec(&accepted).unwrap().len() + 1,
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
        let (tx, rx) = mpsc::sync_channel(1);
        read_events(Cursor::new(Vec::<u8>::new()), tx);
        assert!(
            matches!(rx.try_recv(), Ok(Err(PortError::Failed(message))) if message.contains("without End"))
        );
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
            destination: "archive".into(),
            config: JobConfig {
                language: LanguageChoice::Manual("fr".into()),
                compute: ComputeChoice::Cpu,
            },
        }
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
