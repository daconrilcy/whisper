use super::*;

pub(super) struct NativeAbcDocumentView<'a> {
    pub(super) refreshed_revision: u64,
    pub(super) page: Option<&'a whisper_core::ports::ParakeetDocumentPage>,
    pub(super) document_error: Option<&'a str>,
    pub(super) page_summary: String,
}

pub(super) fn native_ab_c_document_view_snapshot<'a>(
    context: &native_execution_helpers::NativeAbcContext,
    state: &NativeAbcDocumentState,
    view: &'a whisper_core::AppView,
) -> NativeAbcDocumentView<'a> {
    let a = context.ids.a;
    let generation = context.ids.generation;
    let refreshed_revision = view
        .unified_parakeet_sessions
        .iter()
        .find(|session| session.job_id == a && session.generation == generation)
        .map(|session| session.document_revision)
        .unwrap_or(0);
    let selection = view
        .selected_unified_session
        .as_ref()
        .filter(|selection| selection.job_id == a && selection.generation == generation);
    let page = selection.and_then(|selection| selection.document_page.as_ref());
    let document_error = selection.and_then(|selection| selection.document_error.as_deref());
    let page_summary = page.map_or_else(
        || "none".to_owned(),
        |page| {
            let first_sequence = page.entries.first().map(|entry| entry.segment.segment_id.0);
            let text = page
                .entries
                .iter()
                .find(|entry| !entry.segment.text.trim().is_empty())
                .map(|entry| entry.segment.text.chars().take(96).collect::<String>());
            format!(
                "revision={},entries={},first_sequence={first_sequence:?},text={text:?}",
                page.revision,
                page.entries.len()
            )
        },
    );
    *context
        .trace
        .document_page_trace
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = format!(
        "selected={a:?}/{generation:?},requested_revision={:?},current_revision={refreshed_revision},page={page_summary},document_error={:?},request_id={},result=refreshed",
        state.requested_revision,
        document_error.map(|message| message.chars().take(240).collect::<String>()),
        state.document_request_id,
    );
    NativeAbcDocumentView {
        refreshed_revision,
        page,
        document_error,
        page_summary,
    }
}

pub(super) fn wait_for_native_gpu_resume(
    app: &mut ImportApplication<RuntimeImportPort>,
    setup: &native_execution_helpers::NativeGpuResumeSetup,
    native_observer: &Arc<NativeQualificationObserver>,
    native_events: &Receiver<NativeQualificationEvent>,
) -> (
    whisper_core::ports::UnifiedParakeetSessionView,
    Vec<NativeQualificationEvent>,
) {
    let identity = (setup.job_id, setup.resumed_generation);
    let deadline = Instant::now() + Duration::from_secs(3_600);
    let mut observed_phase = "not visible".to_owned();
    let mut observed_message = None;
    let mut complete_view = None;
    let mut observed_native_events = Vec::with_capacity(128);
    while Instant::now() < deadline {
        let view = app
            .dispatch(AppCommand::Refresh)
            .expect("refresh native resumed import");
        native_event_drain(native_events, &mut observed_native_events);
        observed_message = view.message.clone();
        if let Some(session) = view
            .unified_parakeet_sessions
            .iter()
            .find(|session| (session.job_id, session.generation) == identity)
        {
            observed_phase = format!("{:?}", session.phase);
            if session.phase == whisper_core::ports::UnifiedSessionPhase::Recoverable {
                panic!(
                    "native GPU resume became recoverable: phase={observed_phase}, message={observed_message:?}"
                );
            }
            if session.phase == whisper_core::ports::UnifiedSessionPhase::Complete {
                complete_view = Some(session.clone());
                break;
            }
        }
        thread::sleep(Duration::from_millis(10));
    }
    native_event_drain(native_events, &mut observed_native_events);
    assert!(
        !native_observer.overflowed.load(Ordering::Relaxed),
        "native event observer overflowed its fixed capacity"
    );
    assert_native_gpu_resume_route(
        &observed_native_events,
        whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: setup.job_id,
            generation: setup.resumed_generation,
        },
    );
    let complete = complete_view.unwrap_or_else(|| panic!("native GPU resume did not complete: phase={observed_phase}, message={observed_message:?}"));
    assert_eq!(complete.total_samples, Some(setup.expected_samples));
    assert_eq!(complete.captured_samples, setup.expected_samples);
    assert_eq!(complete.durable_samples, setup.expected_samples);
    (complete, observed_native_events)
}

pub(super) fn assert_native_gpu_resume_route(
    observed_native_events: &[NativeQualificationEvent],
    session: whisper_core::parakeet_scheduler::SchedulerSession,
) {
    let routes = observed_native_events
        .iter()
        .filter_map(|event| match &event.kind {
            NativeQualificationEventKind::PreparedRoute {
                session: observed,
                requested_compute,
                compute,
                has_candidate,
                has_expected_gpu,
                fallback_reason,
            } if *observed == session => Some((
                *requested_compute,
                *compute,
                *has_candidate,
                *has_expected_gpu,
                fallback_reason.as_deref(),
            )),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(routes.len(), 1, "resume must compose one GPU route");
    assert_eq!(routes[0].0, ComputeChoice::Gpu);
    assert_eq!(routes[0].1, ComputeChoice::Gpu);
    assert!(routes[0].2 && routes[0].3);
    assert_eq!(routes[0].4, None);
}

#[cfg(windows)]
struct NativeGpuResumeSeed {
    source_path: PathBuf,
    expected_samples: u64,
    source_sha: String,
    job_id: JobId,
    previous_generation: Generation,
    resumed_generation: Generation,
    archive_root: PathBuf,
    previous: whisper_core::ImportRequest,
}

#[cfg(windows)]
struct NativeGpuResumePrefix {
    checkpoint_samples: u64,
    prefix_text: String,
    prefix_range: whisper_core::SourceRange,
}

#[cfg(windows)]
fn native_gpu_resume_paths() -> native_execution_helpers::NativeGpuResumePaths {
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
    let cpu_paths = crate::parakeet_import::ParakeetWorkerPaths {
        executable: whisper.join("worker/whisper-worker-parakeet-cpu.exe"),
        native_dir: whisper.join("native/cpu"),
        model: models_root.join("parakeet-tdt-0.6b-v3.q8_0.gguf"),
        runtime_root: whisper.clone(),
    };
    let gpu_paths = crate::parakeet_import::ParakeetWorkerPaths {
        executable: whisper.join("worker/whisper-worker-parakeet-gpu.exe"),
        native_dir: whisper.join("native/gpu"),
        model: cpu_paths.model.clone(),
        runtime_root: whisper.clone(),
    };
    let workers = WorkerPaths {
        cpu: app_root.join("whisper-worker-cpu.exe"),
        gpu: app_root.join("whisper-worker-gpu.exe"),
    };
    assert!(
        workers.cpu.is_file(),
        "personal Turbo CPU worker is missing"
    );
    assert!(
        workers.gpu.is_file(),
        "personal Turbo GPU worker is missing"
    );
    assert!(
        cpu_paths.validate().is_ok(),
        "personal Parakeet CPU route is invalid"
    );
    assert!(
        gpu_paths.validate().is_ok(),
        "personal Parakeet GPU route is invalid"
    );

    native_execution_helpers::NativeGpuResumePaths {
        execution_root,
        models_root,
        cpu_paths,
        gpu_paths,
        workers,
    }
}

#[cfg(windows)]
fn native_gpu_resume_seed(
    paths: &native_execution_helpers::NativeGpuResumePaths,
) -> NativeGpuResumeSeed {
    let execution_root = &paths.execution_root;
    let cpu_paths = &paths.cpu_paths;
    let (source_path, expected_samples, source_sha) =
        native_queued_import_source(execution_root, true);
    let run_id = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is after the epoch")
        .as_nanos();
    let job_id = JobId((u128::from(std::process::id()) << 96) | run_id);
    let previous_generation = Generation::first();
    let resumed_generation = previous_generation.next().unwrap();
    let archive_root = execution_root.join(format!("queued-gpu-resume-{:032x}", job_id.0));
    assert!(
        !archive_root.exists(),
        "native resume archive destination is fresh"
    );

    let mut previous = unified_import_request(&archive_root, &source_path, job_id.0);
    previous.job_id = job_id;
    previous.generation = previous_generation;
    previous.model_path = cpu_paths.model.to_string_lossy().into_owned();
    previous.config.compute = ComputeChoice::Gpu;
    assert_eq!(
        previous.strategy,
        whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
    );
    assert_real_unified_model(&previous);
    ArchiveStore::identify_source(&mut previous).unwrap();
    assert_eq!(previous.source_samples, Some(expected_samples));
    assert_eq!(
        previous.source_sha256,
        Some(decode_sha256(&source_sha).expect("native fixture SHA-256 must decode"))
    );

    NativeGpuResumeSeed {
        source_path,
        expected_samples,
        source_sha,
        job_id,
        previous_generation,
        resumed_generation,
        archive_root,
        previous,
    }
}

#[cfg(windows)]
fn native_gpu_resume_prefix(seed: &NativeGpuResumeSeed) -> NativeGpuResumePrefix {
    let source_path = &seed.source_path;
    let expected_samples = seed.expected_samples;
    let checkpoint_native_samples = whisper_core::parakeet_windows::WINDOW_MAX_SAMPLES
        + whisper_core::parakeet_windows::MARGIN_SAMPLES;
    assert!(
        expected_samples > checkpoint_native_samples,
        "the long source must exceed one complete native window"
    );
    let mut source_file = std::fs::File::open(source_path)
        .expect("open the guarded long FR WAV read-only for its first window");
    let mut wav_header = [0_u8; 44];
    source_file
        .read_exact(&mut wav_header)
        .expect("read guarded FR WAV header");
    assert_eq!(&wav_header[0..4], b"RIFF");
    assert_eq!(&wav_header[8..12], b"WAVE");
    assert_eq!(&wav_header[36..40], b"data");
    assert_eq!(
        u16::from_le_bytes(wav_header[20..22].try_into().unwrap()),
        1
    );
    assert_eq!(
        u16::from_le_bytes(wav_header[22..24].try_into().unwrap()),
        1
    );
    assert_eq!(
        u32::from_le_bytes(wav_header[24..28].try_into().unwrap()),
        16_000
    );
    assert_eq!(
        u16::from_le_bytes(wav_header[34..36].try_into().unwrap()),
        16
    );
    let data_bytes = u32::from_le_bytes(wav_header[40..44].try_into().unwrap());
    assert_eq!(u64::from(data_bytes / 2), expected_samples);
    assert_eq!(
        std::fs::metadata(source_path).unwrap().len(),
        44 + u64::from(data_bytes)
    );
    let first_window_bytes = usize::try_from(checkpoint_native_samples * 2)
        .expect("first native window byte count fits address space");
    let mut first_window_pcm_bytes = vec![0_u8; first_window_bytes];
    source_file
        .read_exact(&mut first_window_pcm_bytes)
        .expect("read only the bounded first native window and margin");
    let first_window_pcm = first_window_pcm_bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
        .collect::<Vec<_>>();
    let mut first_window_cursor = whisper_core::parakeet_session::WindowCursor::default();
    let first_window = first_window_cursor
        .next_window(0, &first_window_pcm, expected_samples, false)
        .expect("plan the first window from the guarded source samples")
        .expect("the long guarded source must produce a first nonterminal window");
    let checkpoint_samples = first_window.central_end_sample;
    assert_eq!(first_window.sequence, 1);
    assert_eq!(first_window.central_start_sample, 0);
    assert!(
        (whisper_core::parakeet_windows::WINDOW_MIN_SAMPLES
            ..=whisper_core::parakeet_windows::WINDOW_MAX_SAMPLES)
            .contains(&checkpoint_samples),
        "checkpoint must match the scheduler's reachable first-window boundary"
    );

    let prefix_text = "prefix checkpointé avant interruption native";
    let prefix_range = whisper_core::SourceRange::new(
        checkpoint_samples - 6_400,
        checkpoint_samples - 3_200,
        16_000,
    )
    .unwrap();
    NativeGpuResumePrefix {
        checkpoint_samples,
        prefix_text: prefix_text.to_owned(),
        prefix_range,
    }
}

#[cfg(windows)]
fn native_gpu_resume_persist_checkpoint(
    paths: &native_execution_helpers::NativeGpuResumePaths,
    seed: &NativeGpuResumeSeed,
    prefix: &NativeGpuResumePrefix,
) -> whisper_core::gpu_inventory::GpuRuntimeIdentity {
    let execution_root = &paths.execution_root;
    let workers = &paths.workers;
    let job_id = seed.job_id;
    let previous_generation = seed.previous_generation;
    let resumed_generation = seed.resumed_generation;
    let archive_root = &seed.archive_root;
    let previous = &seed.previous;
    let checkpoint_samples = prefix.checkpoint_samples;
    let prefix_text = &prefix.prefix_text;
    let prefix_range = prefix.prefix_range;
    let mut archive = ArchiveStore::new(archive_root);
    archive
        .prepare(previous.clone())
        .expect("create the interrupted generation's real archive checkpoint");
    archive
        .persist_segment(&whisper_core::ports::WorkerSegment {
            job_id,
            generation: previous_generation,
            instance_id: 1,
            segment_id: whisper_core::SegmentId(1),
            range: prefix_range,
            text: prefix_text.into(),
        })
        .expect("write a real resumable prefix checkpoint");
    drop(archive);
    let previous_document_directory = archive_root
        .join("transcriptions")
        .join(format!("{:032x}", job_id.0))
        .join(previous_generation.get().to_string())
        .join("parakeet-unified/document");
    let mut previous_document = crate::parakeet_document::ParakeetDocumentStore::create(
        &previous_document_directory,
        job_id,
        previous_generation,
    )
    .expect("create the interrupted generation's real document checkpoint");
    previous_document
        .append_final(whisper_core::ports::WorkerSegment {
            job_id,
            generation: previous_generation,
            instance_id: 1,
            segment_id: whisper_core::SegmentId(1),
            range: prefix_range,
            text: prefix_text.into(),
        })
        .expect("write the same confirmed prefix into the old document");
    previous_document
        .record_coverage(whisper_core::ports::ParakeetWindowCoverage {
            sequence: 1,
            range: whisper_core::SourceRange::new(0, checkpoint_samples, 16_000).unwrap(),
            last_segment_sequence: 1,
        })
        .expect("checkpoint the covered audio prefix in the old document");
    drop(previous_document);
    let mut queue = crate::queue_store::QueueStore::open(&previous.destination).unwrap();
    queue.enqueue(previous.clone()).unwrap();
    queue
        .set_status(
            job_id,
            previous_generation,
            crate::queue_store::QueueStatus::Interrupted,
        )
        .unwrap();
    assert!(queue.entries().iter().any(|entry| {
        (entry.request.job_id, entry.request.generation) == (job_id, previous_generation)
            && entry.status == crate::queue_store::QueueStatus::Interrupted
    }));
    drop(queue);

    let settings_root = execution_root.join("settings");
    drop(
        crate::source_binding::SourceBinding::acquire(previous, &settings_root)
            .expect("bind the source to the interrupted generation before restart"),
    );
    let candidate =
        native_personal_gpu_candidate_for_generation_harness(workers, resumed_generation);
    assert_eq!(candidate.generation, resumed_generation.get());
    assert_eq!(
        candidate.pnp_instance_id,
        whisper_core::model_registry::PARAKEET_PERSONAL_GPU_PNP_INSTANCE_ID
    );

    candidate
}

#[cfg(windows)]
pub(super) fn native_gpu_resume_setup() -> native_execution_helpers::NativeGpuResumeSetup {
    let paths = native_gpu_resume_paths();
    let seed = native_gpu_resume_seed(&paths);
    let prefix = native_gpu_resume_prefix(&seed);
    let candidate = native_gpu_resume_persist_checkpoint(&paths, &seed, &prefix);
    native_execution_helpers::NativeGpuResumeSetup {
        paths,
        source_path: seed.source_path,
        expected_samples: seed.expected_samples,
        source_sha: seed.source_sha,
        job_id: seed.job_id,
        previous_generation: seed.previous_generation,
        resumed_generation: seed.resumed_generation,
        archive_root: seed.archive_root,
        previous: seed.previous,
        candidate,
        checkpoint_samples: prefix.checkpoint_samples,
        prefix_text: prefix.prefix_text,
        prefix_range: prefix.prefix_range,
    }
}
