use std::{
    fs,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

use whisper_adapters::parakeet_import::{ParakeetProcess, ParakeetWorkerPaths};
use whisper_adapters::{
    archive::ArchiveStore,
    queue_store::{QueueStatus, QueueStore},
    supervisor::WorkerPaths,
    worker_ipc::AsyncImportIo,
};
use whisper_core::{
    AppCommand, Application, ComputeChoice, Generation, ImportApplication, ImportRequest,
    JobConfig, JobId, LanguageChoice, model_catalog::ModelId,
};

#[test]
fn stop_kills_and_waits_for_the_isolated_worker_process() {
    let runtime = PathBuf::from(
        std::env::var_os("PARAKEET_L2_RUNTIME_ROOT")
            .expect("PARAKEET_L2_RUNTIME_ROOT is required by L2a-2 tests"),
    );
    let paths = ParakeetWorkerPaths {
        executable: PathBuf::from(std::env::var_os("PARAKEET_L2_WORKER").expect("worker path")),
        native_dir: PathBuf::from(std::env::var_os("PARAKEET_L2_NATIVE_DIR").expect("native dir")),
        model: PathBuf::from(std::env::var_os("PARAKEET_L2_MODEL").expect("model path")),
        runtime_root: runtime,
    };
    let mut process = ParakeetProcess::spawn(&paths).expect("worker process starts");
    let status = process
        .terminate()
        .expect("Stop waits for physical process exit");
    assert!(
        !status.success(),
        "forced Stop must not report successful transcription"
    );
    assert!(process.try_wait().unwrap().is_some());
}

#[test]
#[ignore = "runs a real import, stops during inference, and verifies no archive publication"]
fn stop_during_parakeet_inference_waits_for_exit_and_preserves_pending_without_publication() {
    let runtime = required_path("PARAKEET_L2_RUNTIME_ROOT");
    let source = required_path("PARAKEET_L2_EN_WAV");
    let model = required_path("PARAKEET_L2_MODEL");
    let worker = required_path("PARAKEET_L2_WORKER");
    let native = required_path("PARAKEET_L2_NATIVE_DIR");
    let destination = runtime.join(format!(
        "archive/stop-during-inference-{}",
        std::process::id()
    ));
    let historical = runtime.join("historical/ggml-large-v3-turbo.bin");
    let dummy_turbo = runtime.join("worker/not-used-turbo.exe");
    let io = AsyncImportIo::start_with_model_catalog_and_parakeet(
        WorkerPaths {
            cpu: dummy_turbo.clone(),
            gpu: dummy_turbo,
        },
        model.parent().unwrap().to_path_buf(),
        historical.to_string_lossy().into_owned(),
        Some(ParakeetWorkerPaths {
            executable: worker,
            native_dir: native,
            model: model.clone(),
            runtime_root: runtime.clone(),
        }),
    );
    let snapshot = whisper_core::model_registry::engine_snapshot(
        1,
        |_| whisper_core::model_catalog::ModelAvailability::Ready,
        vec![],
    );
    let guard = whisper_core::model_selection_app::ModelCommandGuard::new_with_model_paths(
        snapshot,
        1,
        [
            (
                ModelId::ParakeetTdt06BV3,
                model.to_string_lossy().into_owned(),
            ),
            (
                ModelId::WhisperLargeV3Turbo,
                historical.to_string_lossy().into_owned(),
            ),
        ],
    );
    let mut app = ImportApplication::new(io).with_model_guard(guard);
    let job_id = JobId(0x1a5);
    let generation = Generation::first();
    let request = ImportRequest {
        job_id,
        generation,
        source_path: source.to_string_lossy().into_owned(),
        source_sha256: None,
        source_samples: None,
        model_path: model.to_string_lossy().into_owned(),
        model_sha256: decode_sha256(
            whisper_core::model_registry::model(ModelId::ParakeetTdt06BV3)
                .artifact
                .sha256,
        ),
        destination: destination.to_string_lossy().into_owned(),
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        },
    };
    app.dispatch(AppCommand::StartImport { request }).unwrap();
    let deadline = Instant::now() + Duration::from_secs(180);
    loop {
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        if view
            .active_job
            .is_some_and(|(_, state)| state == whisper_core::JobState::Running)
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "worker did not reach running state: {view:?}"
        );
        thread::sleep(Duration::from_millis(10));
    }
    app.dispatch(AppCommand::Stop { job_id, generation })
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        if view
            .active_job
            .is_some_and(|(_, state)| state == whisper_core::JobState::Cancelled)
        {
            break;
        }
        assert!(Instant::now() < deadline, "Stop did not complete: {view:?}");
        thread::sleep(Duration::from_millis(10));
    }
    let pending = destination
        .join("transcriptions")
        .join(format!("{:032x}", job_id.0))
        .join("1")
        .join("pending.json");
    assert!(
        pending.is_file(),
        "interrupted archive checkpoint is preserved"
    );
    assert!(!pending.with_file_name("transcript.txt").exists());
    assert!(!pending.with_file_name("transcript.srt").exists());
}

#[test]
#[ignore = "runs native imports while exercising isolated preparation/publication barriers"]
fn stop_and_quit_at_preparation_and_publication_barriers_preserve_truthful_state() {
    let runtime = required_path("PARAKEET_L2_RUNTIME_ROOT");
    let source = required_path("PARAKEET_L2_EN_WAV");

    let prepare_root = runtime.join(format!("archive/r6-prepare-{}", std::process::id()));
    run_barrier_case(
        &runtime,
        &source,
        &prepare_root,
        "prepare",
        AppCommand::Quit {
            job_id: JobId(0x2a0),
            generation: Generation::first(),
        },
        whisper_core::JobState::Cancelled,
        false,
    );
    let prepare_passage = passage_dir(&prepare_root, JobId(0x2a0), Generation::first());
    assert!(prepare_passage.join("pending.json").is_file());
    assert!(!prepare_passage.join("transcript.txt").exists());
    assert!(!prepare_passage.join("transcript.srt").exists());

    let before_root = runtime.join(format!("archive/r6-before-commit-{}", std::process::id()));
    run_barrier_case(
        &runtime,
        &source,
        &before_root,
        "publish-before-commit",
        AppCommand::Stop {
            job_id: JobId(0x2a1),
            generation: Generation::first(),
        },
        whisper_core::JobState::Cancelled,
        false,
    );
    let before_passage = passage_dir(&before_root, JobId(0x2a1), Generation::first());
    assert!(before_passage.join("pending.json").is_file());
    assert!(!before_passage.join("CURRENT").exists());
    assert!(
        !ArchiveStore::scan(&before_root)
            .unwrap()
            .iter()
            .any(|entry| entry.job_id == JobId(0x2a1) && entry.complete)
    );

    let after_root = runtime.join(format!("archive/r6-after-commit-{}", std::process::id()));
    run_barrier_case(
        &runtime,
        &source,
        &after_root,
        "publish-after-commit",
        AppCommand::Quit {
            job_id: JobId(0x2a2),
            generation: Generation::first(),
        },
        whisper_core::JobState::Complete,
        true,
    );
    let after_passage = passage_dir(&after_root, JobId(0x2a2), Generation::first());
    assert!(after_passage.join("CURRENT").is_file());
    assert!(after_passage.join("transcript.txt").is_file());
    assert!(after_passage.join("transcript.srt").is_file());
}

#[test]
#[ignore = "runs native import; corrupts only the scoped queue journal during Stop"]
fn stop_queue_journal_failure_settles_runtime_and_allows_next_import() {
    let runtime = required_path("PARAKEET_L2_RUNTIME_ROOT");
    let source = required_path("PARAKEET_L2_EN_WAV");
    let source_before = fs::read(&source).unwrap();
    let model = required_path("PARAKEET_L2_MODEL");
    let destination = runtime.join(format!("archive/r9-stop-journal-{}", std::process::id()));
    fs::create_dir_all(&destination).unwrap();
    let record = whisper_core::model_registry::model(ModelId::ParakeetTdt06BV3);
    let mut request = ImportRequest {
        job_id: JobId(0x2a8),
        generation: Generation::first(),
        source_path: source.to_string_lossy().into_owned(),
        source_sha256: None,
        source_samples: None,
        model_path: model.to_string_lossy().into_owned(),
        model_sha256: decode_sha256(record.artifact.sha256),
        destination: destination.to_string_lossy().into_owned(),
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        },
    };
    ArchiveStore::identify_source(&mut request).unwrap();
    QueueStore::open(&destination)
        .unwrap()
        .enqueue(request.clone())
        .unwrap();

    let dummy_turbo = runtime.join("worker/not-used-turbo.exe");
    let io = AsyncImportIo::start_with_model_catalog_and_parakeet(
        WorkerPaths {
            cpu: dummy_turbo.clone(),
            gpu: dummy_turbo,
        },
        model.parent().unwrap().to_path_buf(),
        runtime
            .join("historical/ggml-large-v3-turbo.bin")
            .to_string_lossy()
            .into_owned(),
        Some(ParakeetWorkerPaths {
            executable: required_path("PARAKEET_L2_WORKER"),
            native_dir: required_path("PARAKEET_L2_NATIVE_DIR"),
            model: model.clone(),
            runtime_root: runtime.clone(),
        }),
    );
    let snapshot = whisper_core::model_registry::engine_snapshot(
        1,
        |_| whisper_core::model_catalog::ModelAvailability::Ready,
        vec![],
    );
    let guard = whisper_core::model_selection_app::ModelCommandGuard::new_with_model_paths(
        snapshot,
        1,
        [
            (
                ModelId::ParakeetTdt06BV3,
                model.to_string_lossy().into_owned(),
            ),
            (
                ModelId::WhisperLargeV3Turbo,
                runtime
                    .join("historical/ggml-large-v3-turbo.bin")
                    .to_string_lossy()
                    .into_owned(),
            ),
        ],
    );
    let mut app = ImportApplication::new(io).with_model_guard(guard);
    app.dispatch(AppCommand::ProcessQueued {
        request: request.clone(),
    })
    .unwrap();
    wait_for_state(
        &mut app,
        request.job_id,
        whisper_core::JobState::Running,
        Duration::from_secs(240),
    );
    let passage = passage_dir(&destination, request.job_id, request.generation);
    assert!(passage.join("pending.json").is_file());
    let queue_journal = destination.join("transcriptions/queue.jsonl");
    assert!(queue_journal.is_file());
    fs::write(&queue_journal, b"{not-valid-json\n").unwrap();

    app.dispatch(AppCommand::Stop {
        job_id: request.job_id,
        generation: request.generation,
    })
    .unwrap();
    let stop_deadline = Instant::now() + Duration::from_secs(60);
    let stopped = loop {
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        if view.active_job == Some((request.job_id, whisper_core::JobState::Cancelled)) {
            break view;
        }
        assert!(
            Instant::now() < stop_deadline,
            "Stop did not surface queue-journal failure: {view:?}"
        );
        thread::sleep(Duration::from_millis(10));
    };
    let message = stopped.message.unwrap_or_default();
    assert!(message.contains("StorageCorrupt"), "{message}");
    assert!(passage.join("pending.json").is_file());
    assert!(!passage.join("CURRENT").exists());
    assert!(!passage.join("transcript.txt").exists());
    assert!(!passage.join("transcript.srt").exists());
    assert_eq!(fs::read(&source).unwrap(), source_before);
    assert!(
        ArchiveStore::scan(&destination)
            .unwrap()
            .iter()
            .all(|entry| entry.job_id != request.job_id || !entry.complete)
    );

    let next_destination = runtime.join(format!("archive/r9-next-import-{}", std::process::id()));
    fs::create_dir_all(&next_destination).unwrap();
    let barrier = next_destination.join(".l2a2-acceptance");
    fs::create_dir_all(&barrier).unwrap();
    fs::write(barrier.join("prepare.enabled"), b"enabled").unwrap();
    let next_request = ImportRequest {
        job_id: JobId(0x2a9),
        generation: Generation::first(),
        source_path: source.to_string_lossy().into_owned(),
        source_sha256: None,
        source_samples: None,
        model_path: model.to_string_lossy().into_owned(),
        model_sha256: decode_sha256(record.artifact.sha256),
        destination: next_destination.to_string_lossy().into_owned(),
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        },
    };
    app.dispatch(AppCommand::StartImport {
        request: next_request,
    })
    .unwrap();
    let entered = barrier.join("prepare.entered");
    let start_deadline = Instant::now() + Duration::from_secs(60);
    while !entered.is_file() {
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert!(
            view.active_job.is_some_and(|(job, state)| {
                job == JobId(0x2a9)
                    && matches!(
                        state,
                        whisper_core::JobState::Preparing | whisper_core::JobState::Running
                    )
            }),
            "next import did not enter preparation: {view:?}"
        );
        assert!(
            Instant::now() < start_deadline,
            "next import did not reach prepare"
        );
        thread::sleep(Duration::from_millis(10));
    }
    app.dispatch(AppCommand::Stop {
        job_id: JobId(0x2a9),
        generation: Generation::first(),
    })
    .unwrap();
    fs::write(barrier.join("prepare.release"), b"release").unwrap();
    wait_for_state(
        &mut app,
        JobId(0x2a9),
        whisper_core::JobState::Cancelled,
        Duration::from_secs(60),
    );
}

#[test]
#[ignore = "runs native Parakeet import, interruption, and product-route resume"]
fn resumed_native_import_keeps_confirmed_prefix_once_and_publishes_once() {
    let runtime = required_path("PARAKEET_L2_RUNTIME_ROOT");
    let source = required_path("PARAKEET_L2_EN_WAV");
    let model = required_path("PARAKEET_L2_MODEL");
    let destination = runtime.join(format!("archive/r7-resume-{}", std::process::id()));
    fs::create_dir_all(&destination).unwrap();
    let barriers = destination.join(".l2a2-acceptance");
    fs::create_dir_all(&barriers).unwrap();
    fs::write(barriers.join("resume-prefix-confirmed.enabled"), b"enabled").unwrap();
    let dummy_turbo = runtime.join("worker/not-used-turbo.exe");
    let io = AsyncImportIo::start_with_model_catalog_and_parakeet(
        WorkerPaths {
            cpu: dummy_turbo.clone(),
            gpu: dummy_turbo,
        },
        model.parent().unwrap().to_path_buf(),
        runtime
            .join("historical/ggml-large-v3-turbo.bin")
            .to_string_lossy()
            .into_owned(),
        Some(ParakeetWorkerPaths {
            executable: required_path("PARAKEET_L2_WORKER"),
            native_dir: required_path("PARAKEET_L2_NATIVE_DIR"),
            model: model.clone(),
            runtime_root: runtime.clone(),
        }),
    );
    let record = whisper_core::model_registry::model(ModelId::ParakeetTdt06BV3);
    let snapshot = whisper_core::model_registry::engine_snapshot(
        1,
        |_| whisper_core::model_catalog::ModelAvailability::Ready,
        vec![],
    );
    let guard = whisper_core::model_selection_app::ModelCommandGuard::new_with_model_paths(
        snapshot,
        1,
        [
            (
                ModelId::ParakeetTdt06BV3,
                model.to_string_lossy().into_owned(),
            ),
            (
                ModelId::WhisperLargeV3Turbo,
                runtime
                    .join("historical/ggml-large-v3-turbo.bin")
                    .to_string_lossy()
                    .into_owned(),
            ),
        ],
    );
    let mut app = ImportApplication::new(io).with_model_guard(guard);
    let job_id = JobId(0x2a3);
    let first_generation = Generation::first();
    let mut request = ImportRequest {
        job_id,
        generation: first_generation,
        source_path: source.to_string_lossy().into_owned(),
        source_sha256: None,
        source_samples: None,
        model_path: model.to_string_lossy().into_owned(),
        model_sha256: decode_sha256(record.artifact.sha256),
        destination: destination.to_string_lossy().into_owned(),
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        },
    };
    ArchiveStore::identify_source(&mut request).unwrap();
    QueueStore::open(&destination)
        .unwrap()
        .enqueue(request.clone())
        .unwrap();
    let queued_request = request;
    app.dispatch(AppCommand::ProcessQueued {
        request: queued_request.clone(),
    })
    .unwrap();

    let entered = barriers.join("resume-prefix-confirmed.entered");
    let segment_path = passage_dir(&destination, job_id, first_generation).join("segments.jsonl");
    let deadline = Instant::now() + Duration::from_secs(240);
    loop {
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        if entered.is_file() && segment_path.is_file() {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "confirmed prefix was not persisted: {view:?}"
        );
        thread::sleep(Duration::from_millis(10));
    }
    app.dispatch(AppCommand::Stop {
        job_id,
        generation: first_generation,
    })
    .unwrap();
    fs::write(barriers.join("resume-prefix-confirmed.release"), b"release").unwrap();
    wait_for_state(
        &mut app,
        job_id,
        whisper_core::JobState::Cancelled,
        Duration::from_secs(30),
    );

    let first_passage = passage_dir(&destination, job_id, first_generation);
    let prefix_rows = read_segment_rows(&first_passage.join("segments.jsonl"));
    assert!(
        !prefix_rows.is_empty(),
        "at least one segment is durably confirmed at Stop"
    );
    assert_eq!(
        QueueStore::open(&destination).unwrap().entries()[0].status,
        QueueStatus::Interrupted
    );
    assert!(first_passage.join("pending.json").is_file());
    assert!(!first_passage.join("CURRENT").exists());

    app.dispatch(AppCommand::ResumeInterrupted {
        request: queued_request,
    })
    .unwrap();
    wait_for_state(
        &mut app,
        job_id,
        whisper_core::JobState::Complete,
        Duration::from_secs(240),
    );

    let resumed_generation = first_generation.next().unwrap();
    let resumed_passage = passage_dir(&destination, job_id, resumed_generation);
    let resumed_rows = read_segment_rows(&resumed_passage.join("segments.jsonl"));
    assert!(resumed_rows.len() > prefix_rows.len());
    assert_eq!(
        resumed_rows[0], prefix_rows[0],
        "confirmed prefix is byte-identical"
    );
    for (index, row) in resumed_rows.iter().enumerate() {
        assert_eq!(row["sequence"].as_u64(), Some(index as u64 + 1));
    }
    let prefix_text = prefix_rows[0]["text"].as_str().unwrap();
    let transcript = fs::read_to_string(resumed_passage.join("transcript.txt")).unwrap();
    assert_eq!(
        transcript.matches(prefix_text).count(),
        1,
        "prefix is not duplicated"
    );
    assert!(resumed_passage.join("transcript.srt").is_file());
    assert!(resumed_passage.join("CURRENT").is_file());
    let complete: Vec<_> = ArchiveStore::scan(&destination)
        .unwrap()
        .into_iter()
        .filter(|entry| entry.job_id == job_id && entry.complete)
        .collect();
    assert_eq!(complete.len(), 1, "one generation is published");
    assert!(QueueStore::open(&destination).unwrap().entries().is_empty());
}

fn read_segment_rows(path: &std::path::Path) -> Vec<serde_json::Value> {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn run_barrier_case(
    runtime: &std::path::Path,
    source: &std::path::Path,
    destination: &std::path::Path,
    phase: &str,
    stop_command: AppCommand,
    expected: whisper_core::JobState,
    too_late: bool,
) {
    fs::create_dir_all(destination).unwrap();
    let barrier = destination.join(".l2a2-acceptance");
    fs::create_dir_all(&barrier).unwrap();
    fs::write(barrier.join(format!("{phase}.enabled")), b"enabled").unwrap();
    let model = required_path("PARAKEET_L2_MODEL");
    let dummy_turbo = runtime.join("worker/not-used-turbo.exe");
    let io = AsyncImportIo::start_with_model_catalog_and_parakeet(
        WorkerPaths {
            cpu: dummy_turbo.clone(),
            gpu: dummy_turbo,
        },
        model.parent().unwrap().to_path_buf(),
        runtime
            .join("historical/ggml-large-v3-turbo.bin")
            .to_string_lossy()
            .into_owned(),
        Some(ParakeetWorkerPaths {
            executable: required_path("PARAKEET_L2_WORKER"),
            native_dir: required_path("PARAKEET_L2_NATIVE_DIR"),
            model: model.clone(),
            runtime_root: runtime.to_path_buf(),
        }),
    );
    let record = whisper_core::model_registry::model(ModelId::ParakeetTdt06BV3);
    let snapshot = whisper_core::model_registry::engine_snapshot(
        1,
        |_| whisper_core::model_catalog::ModelAvailability::Ready,
        vec![],
    );
    let guard = whisper_core::model_selection_app::ModelCommandGuard::new_with_model_paths(
        snapshot,
        1,
        [
            (
                ModelId::ParakeetTdt06BV3,
                model.to_string_lossy().into_owned(),
            ),
            (
                ModelId::WhisperLargeV3Turbo,
                runtime
                    .join("historical/ggml-large-v3-turbo.bin")
                    .to_string_lossy()
                    .into_owned(),
            ),
        ],
    );
    let mut app = ImportApplication::new(io).with_model_guard(guard);
    let job_id = match &stop_command {
        AppCommand::Stop { job_id, .. } | AppCommand::Quit { job_id, .. } => *job_id,
        _ => panic!("barrier case requires Stop or Quit"),
    };
    let generation = Generation::first();
    let request = ImportRequest {
        job_id,
        generation,
        source_path: source.to_string_lossy().into_owned(),
        source_sha256: None,
        source_samples: None,
        model_path: model.to_string_lossy().into_owned(),
        model_sha256: decode_sha256(record.artifact.sha256),
        destination: destination.to_string_lossy().into_owned(),
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        },
    };
    app.dispatch(AppCommand::StartImport { request }).unwrap();
    let entered = barrier.join(format!("{phase}.entered"));
    let deadline = Instant::now() + Duration::from_secs(240);
    while !entered.is_file() {
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        let valid_state = match phase {
            "prepare" => view.active_job == Some((job_id, whisper_core::JobState::Preparing)),
            _ => view.active_job.is_some_and(|(active, state)| {
                active == job_id
                    && matches!(
                        state,
                        whisper_core::JobState::Preparing
                            | whisper_core::JobState::Running
                            | whisper_core::JobState::Draining
                            | whisper_core::JobState::Finalizing
                    )
            }),
        };
        assert!(
            valid_state,
            "application left the expected phase before its enabled barrier: {view:?}"
        );
        assert!(
            Instant::now() < deadline,
            "barrier was not reached: {entered:?}; latest application view: {view:?}"
        );
        thread::sleep(Duration::from_millis(10));
    }
    let view = app.dispatch(stop_command).unwrap();
    if too_late {
        assert_eq!(
            view.active_job,
            Some((job_id, whisper_core::JobState::Finalizing))
        );
    }
    fs::write(barrier.join(format!("{phase}.release")), b"release").unwrap();
    wait_for_state(&mut app, job_id, expected, Duration::from_secs(240));
}

fn wait_for_state(
    app: &mut ImportApplication<AsyncImportIo>,
    job_id: JobId,
    expected: whisper_core::JobState,
    timeout: Duration,
) {
    let deadline = Instant::now() + timeout;
    loop {
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        if view.active_job == Some((job_id, expected)) {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "state {expected:?} not reached: {view:?}"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn passage_dir(root: &std::path::Path, job_id: JobId, generation: Generation) -> PathBuf {
    root.join("transcriptions")
        .join(format!("{:032x}", job_id.0))
        .join(generation.get().to_string())
}

fn required_path(name: &str) -> PathBuf {
    PathBuf::from(std::env::var_os(name).unwrap_or_else(|| panic!("{name} is required")))
}

fn decode_sha256(value: &str) -> [u8; 32] {
    let mut bytes = [0; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).unwrap();
    }
    bytes
}
