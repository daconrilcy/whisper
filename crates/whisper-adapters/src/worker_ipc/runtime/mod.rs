use super::*;

mod duplicates;
mod effects;
mod import;
mod live;
mod queue;
mod recovery;
mod source_binding;
pub(super) use live::{LiveEffect, MicrophoneTestCapture};
pub(super) use recovery::RecoveryEffect;

#[cfg(test)]
pub(super) use duplicates::resolve_reprocessing_targets;
pub(super) use duplicates::{DuplicateEffect, verified_present_publications};

pub(super) struct ImportRuntime {
    pub(super) workers: WorkerPaths,
    pub(super) model_catalog: Option<RuntimeModelCatalog>,
    pub(super) parakeet_paths: Option<crate::parakeet_import::ParakeetWorkerPaths>,
    pub(super) gpu_parakeet_paths: Option<crate::parakeet_import::ParakeetWorkerPaths>,
    pub(super) snapshot_registry_root: Option<PathBuf>,
    pub(super) require_snapshot_registry: bool,
    pub(super) parakeet: Option<crate::parakeet_import::ParakeetProcess>,
    pub(super) parakeet_events: VecDeque<WorkerEvent>,
    pub(super) parakeet_expected_gpu: Option<whisper_core::parakeet_ipc::ExpectedGpu>,
    pub(super) parakeet_native_call_started: bool,
    pub(super) parakeet_end: Option<(JobId, Generation, u64, u64)>,
    pub(super) parakeet_stopped_ack: bool,
    pub(super) parakeet_vad: Option<ParakeetVadRun>,
    pub(super) idle_worker: Option<IdleWorker>,
    pub(super) pending_idle_worker: Option<IdleWorker>,
    pub(super) auto_fallback_reason: Option<String>,
    pub(super) diagnostics: Option<DiagnosticSink>,
    pub(super) diagnostic_start_failed: bool,
    pub(super) diagnostic_degraded_reported: bool,
    pub(super) progress: Option<ProgressSupervisor>,
    pub(super) archive: ArchiveStore,
    pub(super) source_bindings: HashMap<(JobId, u64), crate::source_binding::SourceBinding>,
    pub(super) active: Option<ImportRequestState>,
    pub(super) transport: Option<ChildWorkerTransport>,
    pub(super) publication_gate: Arc<PublicationGate>,
    pub(super) automatic_publication_gates:
        Option<Arc<Mutex<HashMap<(JobId, u64), Arc<PublicationGate>>>>>,
    pub(super) unified_publication_gates:
        HashMap<whisper_core::parakeet_scheduler::SchedulerSession, Arc<PublicationGate>>,
    pub(super) resume_prefix: VecDeque<crate::journal::SegmentRecord>,
    pub(super) pending_live: Option<PendingLiveState>,
    pub(super) live: Option<LiveCaptureState>,
    pub(super) live_archive: Option<LiveArchive>,
    pub(super) direct_live: Option<crate::direct_live::DirectLive>,
    pub(super) parakeet_live: Option<crate::parakeet_live::ParakeetLive>,
    pub(super) microphone_test: Option<MicrophoneTestCapture>,
    pub(super) automatic_final_outputs:
        HashMap<(JobId, u64), whisper_core::ports::AutomaticFinalImport>,
    pub(super) parakeet_pending_events: VecDeque<ImportEvent>,
    pub(super) pending_attempt_reset: Option<ImportEvent>,
    pub(super) pending_recovery_counters: Option<ImportEvent>,
    pub(super) live_pending_persistence: usize,
    pub(super) live_capture_events_since_worker_poll: u8,
    pub(super) deferred_live_stopped: Option<WorkerEvent>,
    pub(super) deferred_live_window_finished: Option<FinishedLiveWindow>,
    pub(super) gpu_candidates: std::collections::HashMap<
        (JobId, u64),
        Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
    >,
    pub(super) parakeet_scheduler: whisper_core::parakeet_scheduler::ParakeetScheduler,
    pub(super) unified_sessions:
        HashMap<whisper_core::parakeet_scheduler::SchedulerSession, UnifiedSessionState>,
    pub(super) unified_executor: Option<UnifiedInferenceExecutor>,
    pub(super) native_unified_candidate_enabled: bool,
    pub(super) native_unified_routes:
        HashMap<whisper_core::parakeet_scheduler::SchedulerSession, NativeUnifiedRoute>,
    pub(super) native_executor_route: Option<NativeUnifiedRoute>,
    pub(super) unified_archives:
        HashMap<whisper_core::parakeet_scheduler::SchedulerSession, ArchiveStore>,
    pub(super) unified_resume_checkpoints: HashMap<
        whisper_core::parakeet_scheduler::SchedulerSession,
        crate::archive::UnifiedArchiveResumeCheckpoint,
    >,
    pub(super) unified_capture_sessions:
        HashSet<whisper_core::parakeet_scheduler::SchedulerSession>,
    #[cfg(test)]
    pub(super) native_harness_observer: Option<Arc<NativeQualificationObserver>>,
    #[cfg(test)]
    pub(super) controlled_transcriber: Option<Box<dyn crate::parakeet_pipeline::WindowTranscriber>>,
    #[cfg(test)]
    pub(super) controlled_captures: VecDeque<CaptureStream>,
    #[cfg(test)]
    pub(super) controlled_part_limits: VecDeque<Option<u64>>,
    #[cfg(test)]
    pub(super) controlled_disconnect_session:
        Option<whisper_core::parakeet_scheduler::SchedulerSession>,
    #[cfg(test)]
    pub(super) controlled_panic_session: Option<whisper_core::parakeet_scheduler::SchedulerSession>,
    #[cfg(test)]
    pub(super) controlled_finish_failure_session:
        Option<whisper_core::parakeet_scheduler::SchedulerSession>,
    #[cfg(test)]
    pub(super) controlled_resume_binding_transfer_gate:
        Option<Arc<(Mutex<bool>, std::sync::Condvar)>>,
    #[cfg(test)]
    pub(super) controlled_hold_reserved_call_once: bool,
    #[cfg(test)]
    pub(super) controlled_ready_gate: Option<Arc<(Mutex<bool>, std::sync::Condvar)>>,
    #[cfg(test)]
    pub(super) controlled_child_observer: Option<Arc<Mutex<Vec<ControlledChildObservation>>>>,
    #[cfg(test)]
    pub(super) controlled_skip_child_recycle: bool,
    pub(super) document_drains: VecDeque<UnifiedDocumentDrain>,
}

impl ImportRuntime {
    #[cfg(test)]
    pub(super) fn new(workers: WorkerPaths, publication_gate: Arc<PublicationGate>) -> Self {
        Self::new_with_catalog(workers, publication_gate, None)
    }

    #[cfg(test)]
    pub(super) fn new_with_catalog(
        workers: WorkerPaths,
        publication_gate: Arc<PublicationGate>,
        model_catalog: Option<RuntimeModelCatalog>,
    ) -> Self {
        Self::new_with_workers_and_parakeet(workers, publication_gate, model_catalog, None, None)
    }

    pub(super) fn new_with_workers_and_parakeet(
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
            false,
        )
    }

    pub(super) fn new_with_workers_and_parakeet_registry(
        workers: WorkerPaths,
        publication_gate: Arc<PublicationGate>,
        model_catalog: Option<RuntimeModelCatalog>,
        parakeet_paths: Option<crate::parakeet_import::ParakeetWorkerPaths>,
        gpu_parakeet_paths: Option<crate::parakeet_import::ParakeetWorkerPaths>,
        snapshot_registry_root: Option<PathBuf>,
        require_snapshot_registry: bool,
        native_unified_candidate_enabled: bool,
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
            native_unified_candidate_enabled,
            native_unified_routes: HashMap::new(),
            native_executor_route: None,
            unified_archives: HashMap::new(),
            unified_resume_checkpoints: HashMap::new(),
            unified_capture_sessions: HashSet::new(),
            #[cfg(test)]
            native_harness_observer: None,
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
            controlled_resume_binding_transfer_gate: None,
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
    pub(super) fn with_controlled_transcriber(
        mut self,
        transcriber: Box<dyn crate::parakeet_pipeline::WindowTranscriber>,
    ) -> Self {
        self.controlled_transcriber = Some(transcriber);
        self
    }

    #[cfg(test)]
    pub(super) fn with_disconnected_result_for(
        mut self,
        session: whisper_core::parakeet_scheduler::SchedulerSession,
    ) -> Self {
        self.controlled_disconnect_session = Some(session);
        self
    }

    #[cfg(test)]
    pub(super) fn with_panicking_transcriber_for(
        mut self,
        session: whisper_core::parakeet_scheduler::SchedulerSession,
    ) -> Self {
        self.controlled_panic_session = Some(session);
        self
    }

    #[cfg(test)]
    pub(super) fn with_finish_document_failure_for(
        mut self,
        session: whisper_core::parakeet_scheduler::SchedulerSession,
    ) -> Self {
        self.controlled_finish_failure_session = Some(session);
        self
    }

    #[cfg(test)]
    pub(super) fn with_resume_binding_transfer_gate(
        mut self,
        gate: Arc<(Mutex<bool>, std::sync::Condvar)>,
    ) -> Self {
        self.controlled_resume_binding_transfer_gate = Some(gate);
        self
    }

    #[cfg(test)]
    pub(super) fn hold_next_reserved_call_once(&mut self) {
        self.controlled_hold_reserved_call_once = true;
    }

    #[cfg(test)]
    pub(super) fn hold_next_child_ready(&mut self) -> Arc<(Mutex<bool>, std::sync::Condvar)> {
        let gate = Arc::new((Mutex::new(false), std::sync::Condvar::new()));
        self.controlled_ready_gate = Some(Arc::clone(&gate));
        gate
    }

    #[cfg(test)]
    pub(super) fn with_child_observer(
        mut self,
        observer: Arc<Mutex<Vec<ControlledChildObservation>>>,
    ) -> Self {
        self.controlled_child_observer = Some(observer);
        self
    }

    #[cfg(test)]
    pub(super) fn with_unrecycled_child_mutant(
        mut self,
        observer: Arc<Mutex<Vec<ControlledChildObservation>>>,
    ) -> Self {
        self.controlled_child_observer = Some(observer);
        self.controlled_skip_child_recycle = true;
        self
    }

    #[cfg(test)]
    pub(super) fn with_controlled_capture(
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

    #[cfg(test)]
    pub(super) fn with_native_harness_observer(
        mut self,
        observer: Arc<NativeQualificationObserver>,
    ) -> Self {
        self.native_harness_observer = Some(observer);
        self
    }
}
