use super::super::UNIFIED_CHILD_JOB_CAPACITY;
#[cfg(test)]
use super::super::{ControlledChildObservation, observe_controlled_child};
#[cfg(test)]
use std::sync::Mutex;
use std::{collections::BTreeMap, sync::atomic::Ordering};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        mpsc::{self, Receiver, SyncSender},
    },
    thread,
};
use whisper_core::Generation;

pub(in crate::worker_ipc) struct UnifiedInferenceCommand {
    pub(in crate::worker_ipc) call: whisper_core::parakeet_scheduler::ScheduledCall,
    pub(in crate::worker_ipc) window: whisper_core::parakeet_session::AudioWindow,
    pub(in crate::worker_ipc) pcm: Vec<i16>,
    pub(in crate::worker_ipc) scratch_directory: PathBuf,
}

pub(in crate::worker_ipc) struct UnifiedInferenceResult {
    pub(in crate::worker_ipc) call: whisper_core::parakeet_scheduler::ScheduledCall,
    pub(in crate::worker_ipc) instance_id: u64,
    pub(in crate::worker_ipc) words: Result<Vec<whisper_core::parakeet_ipc::Word>, String>,
}

enum UnifiedExecutorCommand {
    Infer(UnifiedInferenceCommand),
    Stop,
}

pub(in crate::worker_ipc) struct UnifiedInferenceExecutor {
    commands: SyncSender<UnifiedExecutorCommand>,
    pub(in crate::worker_ipc) results: Receiver<UnifiedInferenceResult>,
    pub(in crate::worker_ipc) ready: Receiver<Result<(), String>>,
    pub(in crate::worker_ipc) ready_seen: bool,
    pub(in crate::worker_ipc) thread: Option<
        thread::JoinHandle<Result<Box<dyn crate::parakeet_pipeline::WindowTranscriber>, String>>,
    >,
    pub(in crate::worker_ipc) worker: Option<Box<dyn crate::parakeet_pipeline::WindowTranscriber>>,
    pub(in crate::worker_ipc) stop_requested: Option<Arc<std::sync::atomic::AtomicBool>>,
    pub(in crate::worker_ipc) in_flight: bool,
    #[cfg(test)]
    pub(in crate::worker_ipc) disconnect_result_for:
        Option<whisper_core::parakeet_scheduler::SchedulerSession>,
    #[cfg(test)]
    pub(in crate::worker_ipc) panic_during_transcription_for:
        Option<whisper_core::parakeet_scheduler::SchedulerSession>,
    #[cfg(test)]
    pub(in crate::worker_ipc) next_child_incarnation: u64,
    #[cfg(test)]
    pub(in crate::worker_ipc) current_child_incarnation: u64,
    #[cfg(test)]
    pub(in crate::worker_ipc) child_observer: Option<Arc<Mutex<Vec<ControlledChildObservation>>>>,
    #[cfg(test)]
    pub(in crate::worker_ipc) ready_gate: Option<Arc<(Mutex<bool>, std::sync::Condvar)>>,
}

impl UnifiedInferenceExecutor {
    pub(in crate::worker_ipc) fn is_available(&self) -> bool {
        self.thread.is_some() || self.worker.is_some()
    }

    pub(in crate::worker_ipc) fn start(
        worker: Box<dyn crate::parakeet_pipeline::WindowTranscriber>,
    ) -> Result<Self, String> {
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
    pub(in crate::worker_ipc) fn start_observed(
        worker: Box<dyn crate::parakeet_pipeline::WindowTranscriber>,
        observer: Arc<Mutex<Vec<ControlledChildObservation>>>,
    ) -> Result<Self, String> {
        Self::start_inner(worker, Some(observer), None)
    }

    #[cfg(test)]
    pub(in crate::worker_ipc) fn start_observed_with_ready_gate(
        worker: Box<dyn crate::parakeet_pipeline::WindowTranscriber>,
        observer: Arc<Mutex<Vec<ControlledChildObservation>>>,
        ready_gate: Arc<(Mutex<bool>, std::sync::Condvar)>,
    ) -> Result<Self, String> {
        Self::start_inner(worker, Some(observer), Some(ready_gate))
    }

    pub(in crate::worker_ipc) fn start_inner(
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

    pub(in crate::worker_ipc) fn start_instance(&mut self) -> Result<(), String> {
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

    pub(in crate::worker_ipc) fn is_ready(&mut self) -> Result<bool, String> {
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

    pub(in crate::worker_ipc) fn dispatch(
        &mut self,
        command: UnifiedInferenceCommand,
    ) -> Result<(), String> {
        if self.in_flight {
            return Err("Scheduler: inference executor already has an active call".into());
        }
        self.commands
            .try_send(UnifiedExecutorCommand::Infer(command))
            .map_err(|error| format!("InferenceDispatch: {error}"))?;
        self.in_flight = true;
        Ok(())
    }

    pub(in crate::worker_ipc) fn try_result(
        &mut self,
    ) -> Result<Option<UnifiedInferenceResult>, String> {
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

    pub(in crate::worker_ipc) fn stop_and_reap(&mut self) -> Result<(), String> {
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

    pub(in crate::worker_ipc) fn reap_disconnected(&mut self) -> Result<(), String> {
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

    pub(in crate::worker_ipc) fn restart(&mut self) -> Result<(), String> {
        if self.in_flight || self.thread.is_some() {
            return Err("InferenceRestart: previous instance has not exited".into());
        }
        self.start_instance()
    }

    pub(in crate::worker_ipc) fn replace_reaped_worker(
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
