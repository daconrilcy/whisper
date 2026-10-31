use super::*;

pub(in crate::worker_ipc) enum PreflightResult {
    Duplicate(Box<DuplicateImportPrompt>),
    Queued {
        submitted: (JobId, Generation),
        admitted: (JobId, Generation),
        entries: Vec<whisper_core::ports::ImportQueueEntry>,
    },
}

pub(in crate::worker_ipc) fn collision_free_job_id(
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

/// Deterministic process-level barriers used only by the acceptance-test build.
/// Their root is checked against the isolated L2a-2 runtime before touching disk.
#[cfg(feature = "recette-hooks")]
pub(in crate::worker_ipc) fn acceptance_barrier(
    phase: &str,
    destination: &Path,
) -> Result<(), String> {
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
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    Ok(())
}

#[cfg(not(feature = "recette-hooks"))]
pub(in crate::worker_ipc) fn acceptance_barrier(
    _phase: &str,
    _destination: &Path,
) -> Result<(), String> {
    Ok(())
}

#[cfg(feature = "recette-hooks")]
pub(in crate::worker_ipc) fn take_acceptance_worker_fault(
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
pub(in crate::worker_ipc) fn canonical_extended_acceptance_root(
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

pub(in crate::worker_ipc) fn unix_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| {
            elapsed.as_millis().min(u128::from(u64::MAX)) as u64
        })
}

pub(in crate::worker_ipc) fn scan_archive_history(root: &Path) -> Vec<ArchiveHistoryItem> {
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

pub(in crate::worker_ipc) fn queued_request_identity_matches(
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

pub(in crate::worker_ipc) fn pending_unified_event_identity(
    event: &ImportEvent,
) -> Option<(JobId, Generation)> {
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

pub(in crate::worker_ipc) fn recovery_stop_counters(
    events: VecDeque<ImportEvent>,
) -> Option<ImportEvent> {
    events
        .into_iter()
        .find(|event| matches!(event, ImportEvent::LiveCounters { .. }))
}

impl ImportRuntime {
    pub(in crate::worker_ipc) fn start_diagnostics(&mut self, destination: &Path) {
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

    #[cfg(test)]
    pub(in crate::worker_ipc) fn with_controlled_transcriber(
        mut self,
        transcriber: Box<dyn crate::parakeet_pipeline::WindowTranscriber>,
    ) -> Self {
        self.controlled_transcriber = Some(transcriber);
        self
    }

    #[cfg(test)]
    pub(in crate::worker_ipc) fn with_disconnected_result_for(
        mut self,
        session: whisper_core::parakeet_scheduler::SchedulerSession,
    ) -> Self {
        self.controlled_disconnect_session = Some(session);
        self
    }

    #[cfg(test)]
    pub(in crate::worker_ipc) fn with_panicking_transcriber_for(
        mut self,
        session: whisper_core::parakeet_scheduler::SchedulerSession,
    ) -> Self {
        self.controlled_panic_session = Some(session);
        self
    }

    #[cfg(test)]
    pub(in crate::worker_ipc) fn with_finish_document_failure_for(
        mut self,
        session: whisper_core::parakeet_scheduler::SchedulerSession,
    ) -> Self {
        self.controlled_finish_failure_session = Some(session);
        self
    }

    #[cfg(test)]
    pub(in crate::worker_ipc) fn with_resume_binding_transfer_gate(
        mut self,
        gate: Arc<(Mutex<bool>, std::sync::Condvar)>,
    ) -> Self {
        self.controlled_resume_binding_transfer_gate = Some(gate);
        self
    }

    #[cfg(test)]
    pub(in crate::worker_ipc) fn hold_next_reserved_call_once(&mut self) {
        self.controlled_hold_reserved_call_once = true;
    }

    #[cfg(test)]
    pub(in crate::worker_ipc) fn hold_next_child_ready(
        &mut self,
    ) -> Arc<(Mutex<bool>, std::sync::Condvar)> {
        let gate = Arc::new((Mutex::new(false), std::sync::Condvar::new()));
        self.controlled_ready_gate = Some(Arc::clone(&gate));
        gate
    }

    #[cfg(test)]
    pub(in crate::worker_ipc) fn with_child_observer(
        mut self,
        observer: Arc<Mutex<Vec<ControlledChildObservation>>>,
    ) -> Self {
        self.controlled_child_observer = Some(observer);
        self
    }

    #[cfg(test)]
    pub(in crate::worker_ipc) fn with_unrecycled_child_mutant(
        mut self,
        observer: Arc<Mutex<Vec<ControlledChildObservation>>>,
    ) -> Self {
        self.controlled_child_observer = Some(observer);
        self.controlled_skip_child_recycle = true;
        self
    }

    #[cfg(test)]
    pub(in crate::worker_ipc) fn with_controlled_capture(
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
    pub(in crate::worker_ipc) fn with_native_harness_observer(
        mut self,
        observer: Arc<NativeQualificationObserver>,
    ) -> Self {
        self.native_harness_observer = Some(observer);
        self
    }
}

impl ServiceRuntime for ImportRuntime {
    fn apply_effect(&mut self, effect: ImportEffect, pending: &mut VecDeque<ImportEvent>) {
        if let (Some(identity), Some(gates)) = (
            import_effect_identity(&effect),
            self.automatic_publication_gates.as_ref(),
        ) && let Some(gate) = gates
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&identity)
            .cloned()
        {
            self.publication_gate = gate;
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
        if let Some(live) = self.parakeet_live.as_mut()
            && let Err(error) = live.pump_capture()
        {
            let event = live.stop_after_capture_error(error);
            let event = attach_automatic_final_import(live, event);
            self.parakeet_pending_events.push_back(event);
            self.parakeet_live = None;
        }
        None
    }
}
