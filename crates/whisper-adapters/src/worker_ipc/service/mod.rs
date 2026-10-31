pub(super) mod automatic_final;

#[cfg(feature = "l01-memory-qualification")]
use super::support::memory_trace;
use super::{
    EVENTS_PER_STAGE, ImportRuntime, MAX_FRAME_BYTES, PublicationGate, PublicationGateRegistry,
    RuntimeModelCatalog, WorkerRuntimeConfig, serialize_frame,
};
use crate::supervisor::WorkerPaths;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError},
    },
    thread,
    time::Duration,
};
use whisper_core::{
    Generation, ImportEffect, ImportEvent, ImportIoPort, JobId,
    ports::{PortError, WorkerSegment},
};

pub struct AsyncImportIo {
    effects: SyncSender<ImportEffect>,
    stops: SyncSender<ImportEffect>,
    events: Receiver<ImportEvent>,
    publication_gate: Arc<PublicationGate>,
    pub(super) active_identity: Option<(JobId, Generation)>,
    unified_identities: HashSet<(JobId, u64)>,
    automatic_effects: SyncSender<ImportEffect>,
    automatic_stops: SyncSender<ImportEffect>,
    automatic_events: Receiver<ImportEvent>,
    pub(super) automatic_publication_gates: PublicationGateRegistry,
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
        Self::start_inner(workers, None, WorkerRuntimeConfig::default())
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
            WorkerRuntimeConfig::default(),
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
            WorkerRuntimeConfig {
                parakeet_paths: cpu_parakeet_paths,
                gpu_parakeet_paths,
                ..WorkerRuntimeConfig::default()
            },
        )
    }

    /// Explicit candidate composition for preparing native T9 validation.
    /// All ordinary constructors keep the unified native path closed.
    pub fn start_with_model_catalog_and_parakeet_routes_and_snapshot_registry_native_candidate(
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
            WorkerRuntimeConfig {
                parakeet_paths: cpu_parakeet_paths,
                gpu_parakeet_paths,
                snapshot_registry_root,
                require_snapshot_registry: true,
                native_unified_candidate_enabled: true,
            },
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
            WorkerRuntimeConfig {
                parakeet_paths: cpu_parakeet_paths,
                gpu_parakeet_paths,
                snapshot_registry_root,
                require_snapshot_registry: true,
                native_unified_candidate_enabled: false,
            },
        )
    }

    fn start_with_model_catalog_and_parakeet_routes_inner(
        workers: WorkerPaths,
        catalog_root: PathBuf,
        known_historical_turbo_path: String,
        config: WorkerRuntimeConfig,
    ) -> Self {
        Self::start_inner(
            workers,
            Some(RuntimeModelCatalog {
                catalog: crate::model_catalog::LocalModelCatalog::new(catalog_root),
                known_historical_turbo_path,
                parakeet_paths: config.parakeet_paths.clone(),
                gpu_parakeet_paths: config.gpu_parakeet_paths.clone(),
            }),
            config,
        )
    }

    fn start_inner(
        workers: WorkerPaths,
        model_catalog: Option<RuntimeModelCatalog>,
        config: WorkerRuntimeConfig,
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
        let mut automatic_config = config.clone();
        automatic_config.native_unified_candidate_enabled = false;
        thread::spawn({
            let publication_gate = publication_gate.clone();
            move || {
                let runtime = ImportRuntime::new_with_workers_and_parakeet_registry(
                    workers,
                    publication_gate,
                    model_catalog,
                    config,
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
                        automatic_config,
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
    pub(super) fn for_test(
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

pub(super) fn import_effect_identity(effect: &ImportEffect) -> Option<(JobId, u64)> {
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
                crate::archive::campaign_gate("stop.before_admit", job_id.0, generation.get(), 0)
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
            crate::archive::campaign_gate("stop.before_admit", job_id.0, generation.get(), 0)
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

pub(super) trait ServiceRuntime {
    fn apply_effect(&mut self, effect: ImportEffect, pending: &mut VecDeque<ImportEvent>);
    fn poll_event(&mut self) -> Option<Result<ImportEvent, (JobId, Generation, String)>>;
    fn pump_realtime(&mut self) -> Option<(JobId, Generation, String)> {
        None
    }
}

pub(super) fn effect_event_reservation(effect: &ImportEffect) -> usize {
    match effect {
        ImportEffect::ResumeInterrupted { .. } => 3,
        ImportEffect::ProcessQueued(_) | ImportEffect::Publish { .. } => 2,
        ImportEffect::Stop { .. } | ImportEffect::StopUnifiedLive { .. } => 2,
        _ => 1,
    }
}

pub(super) fn service_loop<R: ServiceRuntime>(
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
