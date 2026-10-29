mod framing;

pub(super) use framing::{MAX_FRAME_BYTES, serialize_frame};
#[cfg(test)]
pub(super) use framing::{read_frame, write_frame};
#[cfg(not(test))]
use framing::{read_frame, write_frame};

use super::EVENTS_PER_STAGE;
#[cfg(feature = "l01-memory-qualification")]
use super::support::memory_trace;
use crate::WorkerTransport;
#[cfg(test)]
use std::collections::VecDeque;
use std::{
    io::{self, BufRead, BufReader},
    path::Path,
    process::{Child, Command, Stdio},
    sync::mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError},
    thread,
    time::Duration,
};
use whisper_core::{
    Generation, JobId,
    ipc::{BackendKind, IPC_PROTOCOL_VERSION, IpcEnvelope, WorkerCommand, WorkerEvent},
    ports::PortError,
};

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
pub(super) enum WorkerReaderMessage {
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
    pub(super) fn inject_event(&mut self, event: WorkerEvent) {
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

impl ChildWorkerTransport {
    pub(super) fn try_request(&self, command: WorkerCommand) -> Result<(), EnqueueError> {
        try_enqueue_worker_command(&self.commands, &self.stop_commands, command)
    }

    pub(super) fn try_request_ordered(&self, command: WorkerCommand) -> Result<(), EnqueueError> {
        try_enqueue_bounded(&self.commands, command)
    }
}

pub(super) enum WorkerCommandPoll {
    Command(WorkerCommand),
    Idle,
    Closed,
}

pub(super) fn take_next_worker_command(
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
pub(super) enum EnqueueError {
    Full(Box<WorkerCommand>),
    Disconnected,
}

pub(super) fn try_enqueue_worker_command(
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

pub(super) fn try_enqueue_bounded(
    commands: &SyncSender<WorkerCommand>,
    command: WorkerCommand,
) -> Result<(), EnqueueError> {
    commands.try_send(command).map_err(|error| match error {
        TrySendError::Full(command) => EnqueueError::Full(Box::new(command)),
        TrySendError::Disconnected(_) => EnqueueError::Disconnected,
    })
}

pub(super) fn read_events(stdout: impl io::Read, tx: SyncSender<WorkerReaderMessage>) {
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
