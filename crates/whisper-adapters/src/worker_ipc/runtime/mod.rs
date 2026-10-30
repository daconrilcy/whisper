use super::*;

mod duplicates;
mod effects;
mod import;
mod live;
mod poll;
mod poll_unified;
mod queue;
mod recovery;
mod source_binding;
mod support;
pub(super) use live::{
    FinishedLiveWindow, ImportRequestState, LiveCaptureState, LiveRecoveryState, PendingLiveState,
    live_capture_poll_allowed,
};
#[cfg(test)]
pub(super) use live::{LIVE_CAPTURE_EVENTS_BEFORE_WORKER_POLL, campaign_reserve_attempt};
pub(super) use poll::ParakeetVadRun;
#[cfg(test)]
pub(super) use poll::{
    ParakeetStoppedExit, TDT_FRAME_OVERLAP_SAMPLES, append_single_pass_parakeet_word_group,
    append_vad_parakeet_word_group, bind_unidentified_startup_failure, map_attempt_sequence,
    parakeet_stopped_exit, parakeet_vad_segments, take_ready_live_stop,
    vad_word_overlap_within_one_tdt_frame, validate_single_pass_parakeet_words,
    worker_event_identity, worker_event_is_stale,
};
pub(super) use support::{
    PreflightResult, acceptance_barrier, collision_free_job_id, pending_unified_event_identity,
    queued_request_identity_matches, recovery_stop_counters, scan_archive_history, unix_time_ms,
};
#[cfg(feature = "recette-hooks")]
pub(super) use support::{canonical_extended_acceptance_root, take_acceptance_worker_fault};

pub(super) use live::{LiveEffect, MicrophoneTestCapture};
#[cfg(test)]
pub(super) use poll_unified::{manual_gpu_failure_event, retry_native_gpu_startup_on_cpu};
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
}
