use super::*;

pub(super) type NativeAbcBlocks = Arc<Mutex<VecDeque<(JobId, Generation, Vec<f32>)>>>;

pub(super) struct NativeAbcAssets {
    models_root: PathBuf,
    cpu_paths: crate::parakeet_import::ParakeetWorkerPaths,
    gpu_paths: crate::parakeet_import::ParakeetWorkerPaths,
    workers: WorkerPaths,
    turbo_model: PathBuf,
    audio: NativeAbcAudio,
    ids: NativeAbcIds,
    archive_root: PathBuf,
    snapshot_registry_root: PathBuf,
}

pub(super) struct NativeAbcAudio {
    pub(super) spoken: Vec<f32>,
    pub(super) a_samples: usize,
    pub(super) expected_a_samples: u64,
    pub(super) expected_a_tail: Vec<i16>,
}

pub(super) struct NativeAbcTrace {
    pub(super) run_started: Instant,
    pub(super) trace_path: PathBuf,
    pub(super) trace_lines: VecDeque<String>,
    pub(super) document_page_trace: Arc<Mutex<String>>,
    pub(super) other_wait_trace_at: Instant,
}

pub(super) struct NativeGpuResumePaths {
    pub(super) execution_root: PathBuf,
    pub(super) models_root: PathBuf,
    pub(super) cpu_paths: crate::parakeet_import::ParakeetWorkerPaths,
    pub(super) gpu_paths: crate::parakeet_import::ParakeetWorkerPaths,
    pub(super) workers: WorkerPaths,
}

pub(super) struct NativeGpuResumeSetup {
    pub(super) paths: NativeGpuResumePaths,
    pub(super) source_path: PathBuf,
    pub(super) expected_samples: u64,
    pub(super) source_sha: String,
    pub(super) job_id: JobId,
    pub(super) previous_generation: Generation,
    pub(super) resumed_generation: Generation,
    pub(super) archive_root: PathBuf,
    pub(super) previous: whisper_core::ImportRequest,
    pub(super) candidate: whisper_core::gpu_inventory::GpuRuntimeIdentity,
    pub(super) checkpoint_samples: u64,
    pub(super) prefix_text: String,
    pub(super) prefix_range: whisper_core::SourceRange,
}

pub(super) struct NativeGpuResumeRuntime {
    pub(super) setup: NativeGpuResumeSetup,
    pub(super) app: ImportApplication<RuntimeImportPort>,
    pub(super) observer: Arc<NativeQualificationObserver>,
    pub(super) native_events: Receiver<NativeQualificationEvent>,
    pub(super) expected_profile: &'static str,
    pub(super) expected_gpu_key: crate::archive::ArchiveQualificationKey,
}

pub(super) struct NativeGpuResumeRun {
    pub(super) setup: NativeGpuResumeSetup,
    pub(super) app: ImportApplication<RuntimeImportPort>,
    pub(super) complete: whisper_core::ports::UnifiedParakeetSessionView,
    pub(super) observed_native_events: Vec<NativeQualificationEvent>,
    pub(super) expected_profile: &'static str,
    pub(super) expected_gpu_key: crate::archive::ArchiveQualificationKey,
}

pub(super) struct NativeAbcContext {
    pub(super) compute: ComputeChoice,
    pub(super) ids: NativeAbcIds,
    pub(super) audio: NativeAbcAudio,
    pub(super) archive_root: PathBuf,
    pub(super) snapshot_registry_root: PathBuf,
    pub(super) blocks: NativeAbcBlocks,
    pub(super) stop_submissions: Arc<AtomicU64>,
    pub(super) requests: [whisper_core::ports::LiveRequest; 3],
    pub(super) app: ImportApplication<RuntimeImportPort>,
    pub(super) run_started: Instant,
    pub(super) observations: Vec<NativeQualificationEvent>,
    pub(super) trace: NativeAbcTrace,
    pub(super) native_events: Receiver<NativeQualificationEvent>,
    pub(super) observer: Arc<NativeQualificationObserver>,
    pub(super) times: NativeAbcTimes,
    pub(super) first_a_request_id: Option<u64>,
    pub(super) first_a_send: Option<Instant>,
    pub(super) a_stop: Option<Instant>,
    pub(super) a_drain_completed: Option<Instant>,
    pub(super) b_start: Option<Instant>,
    pub(super) b_audio_ack: Option<(Instant, u64, u64)>,
    pub(super) first_a_result: Option<Instant>,
    pub(super) first_text_available: Option<Instant>,
    pub(super) b_stop: Option<Instant>,
    pub(super) reaped_before_c: Option<(u64, u32, Instant)>,
}

impl NativeAbcContext {
    pub(super) fn checkpoint(&mut self, stage: &str, request_id: Option<u64>) {
        let view = self.app.view();
        let session_state = |job_id| {
            view.unified_parakeet_sessions
                .iter()
                .find(|session| {
                    session.job_id == job_id && session.generation == self.ids.generation
                })
                .map(|session| {
                    format!(
                        "phase={:?},captured={},durable={},total={:?},revision={}",
                        session.phase,
                        session.captured_samples,
                        session.durable_samples,
                        session.total_samples,
                        session.document_revision
                    )
                })
                .unwrap_or_else(|| "absent".to_owned())
        };
        let last_event = self
            .observations
            .last()
            .map(|event| {
                format!("{:?}@{:?}", event.kind, event.at)
                    .chars()
                    .take(512)
                    .collect::<String>()
            })
            .unwrap_or_else(|| "none".to_owned());
        let page_state = self
            .trace
            .document_page_trace
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        let line = format!(
            "elapsed_ms={} stage={} A={:?}/{:?}[{}] B={:?}/{:?}[{}] request_id={:?} active={:?} message={:?} document_page={} events={} last={}",
            self.trace.run_started.elapsed().as_millis(),
            stage,
            self.ids.a,
            self.ids.generation,
            session_state(self.ids.a),
            self.ids.b,
            self.ids.generation,
            session_state(self.ids.b),
            request_id,
            view.active_job,
            view.message
                .as_deref()
                .map(|message| message.chars().take(240).collect::<String>()),
            page_state.chars().take(512).collect::<String>(),
            self.observations.len(),
            last_event
        );
        if self.trace.trace_lines.len() == 128 {
            self.trace.trace_lines.pop_front();
        }
        self.trace.trace_lines.push_back(line);
        let mut file = fs::File::create(&self.trace.trace_path).unwrap_or_else(|error| {
            panic!(
                "create bounded A/B/C progress trace {}: {error}",
                self.trace.trace_path.display()
            )
        });
        for line in &self.trace.trace_lines {
            writeln!(file, "{line}")
                .unwrap_or_else(|error| panic!("write bounded A/B/C progress trace: {error}"));
        }
        file.flush()
            .unwrap_or_else(|error| panic!("flush bounded A/B/C progress trace: {error}"));
    }

    pub(super) fn trace_wait_before_refresh(
        &mut self,
        stage: &str,
        request_id: Option<u64>,
    ) -> bool {
        if self.trace.other_wait_trace_at.elapsed() < Duration::from_secs(5) {
            return false;
        }
        self.checkpoint(&format!("{stage}-before-Refresh"), request_id);
        self.trace.other_wait_trace_at = Instant::now();
        true
    }

    pub(super) fn record_a_phase_observation(&mut self) {
        let observed_at = Instant::now();
        let observed_view = self.app.view();
        if let Some(session) = observed_view
            .unified_parakeet_sessions
            .iter()
            .find(|session| {
                session.job_id == self.ids.a && session.generation == self.ids.generation
            })
        {
            if self.times.a_audio_drained_at.is_none()
                && session.total_samples.is_some_and(|total| {
                    session.captured_samples == total && session.durable_samples == total
                })
            {
                self.times.a_audio_drained_at = Some(observed_at);
            }
            match session.phase {
                whisper_core::ports::UnifiedSessionPhase::Finalizing => {
                    self.times.a_finalizing_at.get_or_insert(observed_at);
                }
                whisper_core::ports::UnifiedSessionPhase::Publishing => {
                    self.times.a_publishing_at.get_or_insert(observed_at);
                }
                whisper_core::ports::UnifiedSessionPhase::Complete => {
                    self.times.a_published_at.get_or_insert(observed_at);
                }
                _ => {}
            }
        }
    }

    pub(super) fn require_b_session(
        &self,
        stage: &str,
    ) -> whisper_core::ports::UnifiedParakeetSessionView {
        let view = self.app.view();
        let Some(session) = view
            .unified_parakeet_sessions
            .iter()
            .find(|session| {
                session.job_id == self.ids.b && session.generation == self.ids.generation
            })
            .cloned()
        else {
            let recent_events = self.observations.iter().rev().take(12).collect::<Vec<_>>();
            panic!(
                "{stage}: B session {:?}/{:?} disappeared; active_job={:?}, app_message={:?}, recent_native_events={recent_events:?}, trace={}",
                self.ids.b,
                self.ids.generation,
                view.active_job,
                view.message,
                self.trace.trace_path.display()
            );
        };
        let recoverable_active_job = view.active_job.as_ref().is_some_and(|(job_id, state)| {
            *job_id == self.ids.b && *state == JobState::Recoverable
        });
        if session.phase == whisper_core::ports::UnifiedSessionPhase::Recoverable
            || recoverable_active_job
        {
            let recent_events = self.observations.iter().rev().take(12).collect::<Vec<_>>();
            panic!(
                "{stage}: B became Recoverable; session={session:?}, active_job={:?}, app_message={:?}, recent_native_events={recent_events:?}, trace={}",
                view.active_job,
                view.message,
                self.trace.trace_path.display()
            );
        }
        session
    }
}

pub(super) fn prepare_native_ab_c_context(
    compute: ComputeChoice,
    gpu_candidate: Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
) -> NativeAbcContext {
    let assets = prepare_native_ab_c_assets(compute, gpu_candidate.as_ref());
    let model_catalog = Some(RuntimeModelCatalog {
        catalog: crate::model_catalog::LocalModelCatalog::new(assets.models_root.clone()),
        known_historical_turbo_path: assets.turbo_model.to_string_lossy().into_owned(),
        parakeet_paths: Some(assets.cpu_paths.clone()),
        gpu_parakeet_paths: Some(assets.gpu_paths.clone()),
    });
    assert!(
        !assets.snapshot_registry_root.exists()
            && !assets
                .snapshot_registry_root
                .join("reprocessing-snapshots.lock")
                .exists(),
        "capture-only qualification must begin without a source snapshot registry"
    );
    let (observer, native_events) = NativeQualificationObserver::channel();
    let mut runtime = ImportRuntime::new_with_workers_and_parakeet_registry(
        assets.workers.clone(),
        Arc::new(PublicationGate::default()),
        model_catalog,
        WorkerRuntimeConfig {
            parakeet_paths: Some(assets.cpu_paths.clone()),
            gpu_parakeet_paths: Some(assets.gpu_paths.clone()),
            snapshot_registry_root: Some(assets.snapshot_registry_root.clone()),
            require_snapshot_registry: true,
            native_unified_candidate_enabled: true,
        },
    )
    .with_native_harness_observer(Arc::clone(&observer))
    .with_controlled_capture(
        CaptureStream::empty_test_capture(16_000, 1, "test:pu-l4-native-a"),
        None,
    )
    .with_controlled_capture(
        CaptureStream::empty_test_capture(16_000, 1, "test:pu-l4-native-b"),
        None,
    )
    .with_controlled_capture(
        CaptureStream::empty_test_capture(16_000, 1, "test:pu-l4-native-c"),
        None,
    );
    if let Some(candidate) = &gpu_candidate {
        assert_eq!(candidate.generation, assets.ids.generation.get());
        for job_id in [assets.ids.a, assets.ids.b, assets.ids.c] {
            runtime.gpu_candidates.insert(
                (job_id, assets.ids.generation.get()),
                Some(candidate.clone()),
            );
        }
    }
    let (blocks, stop_submissions) = native_ab_c_capture_queues(&assets.audio, assets.ids);
    let requests = native_ab_c_requests(&assets, compute, &assets.cpu_paths.model);
    let port = RuntimeImportPort {
        runtime,
        events: VecDeque::new(),
        capture_progress: None,
        document_revision: None,
        unified_stop_submissions: Some(Arc::clone(&stop_submissions)),
        controlled_capture_blocks: Some(Arc::clone(&blocks)),
    };
    let app = ImportApplication::new(port);
    let run_started = Instant::now();
    let trace_path = assets.archive_root.join("ab-c-progress.log");
    NativeAbcContext {
        compute,
        ids: assets.ids,
        audio: assets.audio,
        archive_root: assets.archive_root,
        snapshot_registry_root: assets.snapshot_registry_root,
        blocks,
        stop_submissions,
        requests,
        app,
        run_started,
        observations: Vec::with_capacity(256),
        trace: NativeAbcTrace {
            run_started,
            trace_path,
            trace_lines: VecDeque::with_capacity(128),
            document_page_trace: Arc::new(Mutex::new("not requested".to_owned())),
            other_wait_trace_at: Instant::now(),
        },
        native_events,
        observer,
        times: NativeAbcTimes::default(),
        first_a_request_id: None,
        first_a_send: None,
        a_stop: None,
        a_drain_completed: None,
        b_start: None,
        b_audio_ack: None,
        first_a_result: None,
        first_text_available: None,
        b_stop: None,
        reaped_before_c: None,
    }
}

fn prepare_native_ab_c_assets(
    compute: ComputeChoice,
    gpu_candidate: Option<&whisper_core::gpu_inventory::GpuRuntimeIdentity>,
) -> NativeAbcAssets {
    let execution_root = std::env::var_os("WHISPER_L04_CAMPAIGN_RUN")
        .map(PathBuf::from)
        .expect("guarded PU-L4 execution root is required")
        .canonicalize()
        .expect("guarded execution root must exist");
    let local_app_data = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .expect("isolated LOCALAPPDATA is required")
        .canonicalize()
        .expect("isolated LOCALAPPDATA must exist");
    let whisper = local_app_data.join("Whisper");
    let app_root = whisper.join("app");
    let models_root = whisper.join("models");
    let (cpu_paths, gpu_paths) = native_ab_c_parakeet_paths(&whisper, &models_root);
    let workers = WorkerPaths {
        cpu: app_root.join("whisper-worker-cpu.exe"),
        gpu: app_root.join("whisper-worker-gpu.exe"),
    };
    let turbo_model = models_root.join("ggml-large-v3-turbo.bin");
    for path in [
        &workers.cpu,
        &workers.gpu,
        &cpu_paths.executable,
        &cpu_paths.model,
        &turbo_model,
    ] {
        assert!(
            path.is_file(),
            "required personal asset missing: {}",
            path.display()
        );
    }
    assert!(cpu_paths.validate().is_ok());
    validate_native_ab_c_gpu_paths(compute, gpu_candidate, &gpu_paths);
    let audio = native_ab_c_spoken_reference(&execution_root);
    let a = JobId((u128::from(std::process::id()) << 96) | 0x4e415449564541);
    let ids = NativeAbcIds {
        a,
        b: JobId(a.0 + 1),
        c: JobId(a.0 + 2),
        generation: Generation::first(),
    };
    let backend_case = if compute == ComputeChoice::Cpu {
        "cpu"
    } else {
        "gpu"
    };
    let archive_root = execution_root.join(format!("native-ab-c-{backend_case}"));
    fs::create_dir_all(&archive_root).expect("create archive root under guarded lot");
    let snapshot_registry_root = execution_root.join("settings");
    NativeAbcAssets {
        models_root,
        cpu_paths,
        gpu_paths,
        workers,
        turbo_model,
        audio,
        ids,
        archive_root,
        snapshot_registry_root,
    }
}

fn native_ab_c_parakeet_paths(
    whisper: &Path,
    models_root: &Path,
) -> (
    crate::parakeet_import::ParakeetWorkerPaths,
    crate::parakeet_import::ParakeetWorkerPaths,
) {
    let cpu_paths = crate::parakeet_import::ParakeetWorkerPaths {
        executable: whisper.join("worker/whisper-worker-parakeet-cpu.exe"),
        native_dir: whisper.join("native/cpu"),
        model: models_root.join("parakeet-tdt-0.6b-v3.q8_0.gguf"),
        runtime_root: whisper.to_path_buf(),
    };
    let gpu_paths = crate::parakeet_import::ParakeetWorkerPaths {
        executable: whisper.join("worker/whisper-worker-parakeet-gpu.exe"),
        native_dir: whisper.join("native/gpu"),
        model: cpu_paths.model.clone(),
        runtime_root: whisper.to_path_buf(),
    };
    (cpu_paths, gpu_paths)
}

fn validate_native_ab_c_gpu_paths(
    compute: ComputeChoice,
    gpu_candidate: Option<&whisper_core::gpu_inventory::GpuRuntimeIdentity>,
    gpu_paths: &crate::parakeet_import::ParakeetWorkerPaths,
) {
    if compute == ComputeChoice::Gpu {
        assert!(gpu_paths.executable.is_file());
        assert!(gpu_paths.validate().is_ok());
        assert!(
            gpu_candidate.is_some(),
            "GPU route requires a real inventory identity"
        );
    } else {
        assert!(
            gpu_candidate.is_none(),
            "CPU case must not imply GPU inventory evidence"
        );
    }
}

fn native_ab_c_spoken_reference(execution_root: &Path) -> NativeAbcAudio {
    let spoken = native_spoken_prefix(execution_root);
    let a_samples = spoken.len();
    let expected_a_samples = u64::try_from(a_samples).expect("spoken prefix sample count fits u64");
    let expected_a_tail = native_ab_c_expected_tail(&spoken);
    NativeAbcAudio {
        spoken,
        a_samples,
        expected_a_samples,
        expected_a_tail,
    }
}

fn native_ab_c_expected_tail(spoken: &[f32]) -> Vec<i16> {
    let mut reference_capture = crate::capture::CaptureStream::empty_test_capture(
        16_000,
        1,
        "test:pu-l4-native-a-reference",
    );
    for block in spoken.chunks(80_000) {
        reference_capture.enqueue_test_block(block.to_vec());
    }
    let mut converter = crate::capture::Pcm16Converter::new(16_000, 1)
        .expect("create capture-compatible PCM reference converter");
    let mut tail = VecDeque::with_capacity(160);
    while let Some(block) = reference_capture.try_next_block() {
        for sample in converter
            .push(&block)
            .expect("convert bounded spoken reference block")
        {
            if tail.len() == 160 {
                tail.pop_front();
            }
            tail.push_back(sample);
        }
        reference_capture
            .recycle_block(block)
            .expect("recycle bounded spoken reference block");
    }
    for sample in converter
        .finish()
        .expect("drain capture-compatible reference converter at EOF")
    {
        if tail.len() == 160 {
            tail.pop_front();
        }
        tail.push_back(sample);
    }
    let tail = tail.into_iter().collect::<Vec<_>>();
    assert_eq!(
        tail.len(),
        160,
        "reference converter must produce the full tail"
    );
    tail
}

fn native_ab_c_capture_queues(
    audio: &NativeAbcAudio,
    ids: NativeAbcIds,
) -> (NativeAbcBlocks, Arc<AtomicU64>) {
    let blocks = Arc::new(Mutex::new(VecDeque::new()));
    let stop_submissions = Arc::new(AtomicU64::new(0));
    for chunk in audio.spoken.chunks(80_000) {
        blocks
            .lock()
            .unwrap()
            .push_back((ids.a, ids.generation, chunk.to_vec()));
    }
    (blocks, stop_submissions)
}

fn native_ab_c_requests(
    assets: &NativeAbcAssets,
    compute: ComputeChoice,
    model: &Path,
) -> [whisper_core::ports::LiveRequest; 3] {
    let ids = [assets.ids.a, assets.ids.b, assets.ids.c];
    ids.map(|job_id| {
        let mut request = unified_live_request(&assets.archive_root, job_id.0);
        request.model_path = model.to_string_lossy().into_owned();
        request.microphone = whisper_core::MicrophoneChoice::Device(format!(
            "test:pu-l4-native-{}",
            if job_id == assets.ids.a {
                "a"
            } else if job_id == assets.ids.b {
                "b"
            } else {
                "c"
            }
        ));
        request.config.compute = compute;
        request
    })
}
