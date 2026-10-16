use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

use whisper_adapters::{
    archive::{ArchiveQualificationKey, ArchiveStore},
    queue_store::{QueueStatus, QueueStore},
    supervisor::WorkerPaths,
    worker_ipc::AsyncImportIo,
};
use whisper_core::{
    AppCommand, Application, ComputeChoice, Generation, ImportApplication, JobConfig, JobId,
    LanguageChoice, SegmentId, SourceRange,
    model_catalog::ModelId,
    model_registry::model,
    ports::{ImportRequest, WorkerSegment},
};

fn required_path(name: &str) -> PathBuf {
    PathBuf::from(std::env::var_os(name).unwrap_or_else(|| panic!("{name} is required")))
}

fn archive_root() -> PathBuf {
    let root = required_path("PARAKEET_L2_ARCHIVE_ROOT");
    let canonical = root.canonicalize().expect("archive root exists");
    let allowed = required_path("PARAKEET_L2_RUNTIME_ROOT")
        .canonicalize()
        .expect("runtime root exists");
    assert!(
        canonical.starts_with(allowed),
        "archive root must be inside L2a-2"
    );
    root
}

fn qualified_key() -> ArchiveQualificationKey {
    ArchiveQualificationKey::parakeet_cpu_test()
}

#[test]
fn parakeet_model_identity_prepares_persists_and_publishes_txt_srt() {
    let root = archive_root().join("parakeet-archive-pass");
    let _ = fs::remove_dir_all(&root);
    let source = required_path("PARAKEET_L2_EN_WAV");
    let model_path = required_path("PARAKEET_L2_MODEL");
    let record = model(ModelId::ParakeetTdt06BV3);
    let mut request = ImportRequest {
        job_id: JobId(0x1a2),
        generation: Generation::first(),
        source_path: source.to_string_lossy().into_owned(),
        source_sha256: None,
        source_samples: None,
        model_path: model_path.to_string_lossy().into_owned(),
        model_sha256: decode_sha256(record.artifact.sha256),
        destination: root.to_string_lossy().into_owned(),
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        },
    };
    let mut archive = ArchiveStore::new(&root);
    let prepared = archive
        .prepare_with_qualification(request.clone(), None, Some(qualified_key()))
        .expect("registered Parakeet artifact accepted");
    request = prepared.clone();
    assert!(request.source_samples.unwrap() > 0);
    archive
        .persist_segment(&WorkerSegment {
            job_id: request.job_id,
            generation: request.generation,
            instance_id: 1,
            segment_id: SegmentId(1),
            range: SourceRange::new(0, 16_000, 16_000).unwrap(),
            text: "Test archive Parakeet.".into(),
        })
        .expect("segment journaled");
    archive
        .prepare_publish(request.job_id, request.generation, 1)
        .expect("TXT and SRT verified before commit");
    archive
        .commit_publish(request.job_id, request.generation)
        .expect("publication committed");
    let entries = ArchiveStore::scan(&root).expect("archive scan");
    assert!(entries.iter().any(|entry| {
        entry.job_id == request.job_id && entry.generation == request.generation && entry.complete
    }));
    let passage = root
        .join("transcriptions")
        .join(format!("{:032x}", request.job_id.0))
        .join(request.generation.get().to_string());
    assert_eq!(
        fs::read_to_string(passage.join("transcript.txt")).unwrap(),
        "Test archive Parakeet."
    );
    assert!(
        fs::read_to_string(passage.join("transcript.srt"))
            .unwrap()
            .contains("Test archive Parakeet.")
    );
}

#[test]
fn parakeet_archive_refuses_model_sha_divergence_before_pending_creation() {
    let root = archive_root().join("parakeet-archive-reject");
    let _ = fs::remove_dir_all(&root);
    let source = required_path("PARAKEET_L2_EN_WAV");
    let model_path = required_path("PARAKEET_L2_MODEL");
    let mut request = ImportRequest {
        job_id: JobId(0x1a3),
        generation: Generation::first(),
        source_path: source.to_string_lossy().into_owned(),
        source_sha256: None,
        source_samples: None,
        model_path: model_path.to_string_lossy().into_owned(),
        model_sha256: [0; 32],
        destination: root.to_string_lossy().into_owned(),
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        },
    };
    let mut archive = ArchiveStore::new(&root);
    assert!(
        archive
            .prepare(request.clone())
            .unwrap_err()
            .contains("ModelHashMismatch")
    );
    request.model_sha256 = decode_sha256(model(ModelId::ParakeetTdt06BV3).artifact.sha256);
    assert!(
        !root.exists(),
        "rejected identity must not create pending archive state"
    );
}

#[test]
fn pending_scan_and_resume_require_the_exact_qualified_test_key() {
    let rejected_root = archive_root().join("parakeet-key-reject");
    let _ = fs::remove_dir_all(&rejected_root);
    let source = required_path("PARAKEET_L2_EN_WAV");
    let model_path = required_path("PARAKEET_L2_MODEL");
    let record = model(ModelId::ParakeetTdt06BV3);
    let make_request = |destination: &PathBuf, job| ImportRequest {
        job_id: JobId(job),
        generation: Generation::first(),
        source_path: source.to_string_lossy().into_owned(),
        source_sha256: None,
        source_samples: None,
        model_path: model_path.to_string_lossy().into_owned(),
        model_sha256: decode_sha256(record.artifact.sha256),
        destination: destination.to_string_lossy().into_owned(),
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        },
    };
    let mut wrong_key = qualified_key();
    wrong_key.worker_build = "0".repeat(64);
    assert!(
        ArchiveStore::new(&rejected_root)
            .prepare_with_qualification(make_request(&rejected_root, 0x1a9), None, Some(wrong_key),)
            .unwrap_err()
            .contains("exact Test worker/runtime key")
    );
    assert!(!rejected_root.exists());

    let root = archive_root().join("parakeet-key-scan-resume");
    let _ = fs::remove_dir_all(&root);
    let request = make_request(&root, 0x1aa);
    let prepared = ArchiveStore::new(&root)
        .prepare_with_qualification(request, None, Some(qualified_key()))
        .unwrap();
    let pending_path = root
        .join("transcriptions")
        .join(format!("{:032x}", prepared.job_id.0))
        .join(prepared.generation.get().to_string())
        .join("pending.json");
    let mut pending: serde_json::Value =
        serde_json::from_slice(&fs::read(&pending_path).unwrap()).unwrap();
    pending["qualification"]["worker_build"] = serde_json::json!("0".repeat(64));
    fs::write(&pending_path, serde_json::to_vec(&pending).unwrap()).unwrap();

    let scan = ArchiveStore::scan(&root).unwrap();
    assert!(scan.iter().any(|entry| {
        entry.job_id == prepared.job_id
            && entry
                .diagnostic
                .as_deref()
                .is_some_and(|message| message.contains("pending identity/schema"))
    }));
    let mut resumed = prepared.clone();
    resumed.generation = prepared.generation.next().unwrap();
    assert!(
        ArchiveStore::new(&root)
            .resume_from_with_qualification(
                &prepared,
                resumed,
                Some(&prepared.model_path),
                Some(qualified_key()),
            )
            .unwrap_err()
            .contains("pending identity/schema")
    );
}

#[test]
fn parakeet_pending_queue_resumes_only_the_same_model_source_and_prefix() {
    let destination = archive_root().join("parakeet-queue-resume");
    let _ = fs::remove_dir_all(&destination);
    let source = required_path("PARAKEET_L2_EN_WAV");
    let model_path = required_path("PARAKEET_L2_MODEL");
    let model_path_text = model_path.to_string_lossy().into_owned();
    let record = model(ModelId::ParakeetTdt06BV3);
    let initial = ImportRequest {
        job_id: JobId(0x1a6),
        generation: Generation::first(),
        source_path: source.to_string_lossy().into_owned(),
        source_sha256: None,
        source_samples: None,
        model_path: model_path_text.clone(),
        model_sha256: decode_sha256(record.artifact.sha256),
        destination: destination.to_string_lossy().into_owned(),
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        },
    };
    let mut archive = ArchiveStore::new(&destination);
    let prepared = archive
        .prepare_with_qualification(initial, None, Some(qualified_key()))
        .expect("prepare verified source and model");
    archive
        .persist_segment(&WorkerSegment {
            job_id: prepared.job_id,
            generation: prepared.generation,
            instance_id: 1,
            segment_id: SegmentId(1),
            range: SourceRange::new(0, 16_000, 16_000).unwrap(),
            text: "Confirmed prefix.".into(),
        })
        .expect("confirmed prefix durable");
    drop(archive);

    let mut queue = QueueStore::open(&destination).unwrap();
    queue.enqueue(prepared.clone()).unwrap();
    queue
        .set_status(
            prepared.job_id,
            prepared.generation,
            QueueStatus::Interrupted,
        )
        .unwrap();
    let mut resumed = prepared.clone();
    resumed.generation = prepared.generation.next().unwrap();
    queue
        .validate_resume(prepared.job_id, prepared.generation, &resumed)
        .expect("exact durable request is resumable");

    let mut divergent = resumed.clone();
    divergent.model_sha256[0] ^= 1;
    assert!(
        queue
            .validate_resume(prepared.job_id, prepared.generation, &divergent)
            .is_err()
    );
    let mut divergent = resumed.clone();
    divergent.source_sha256 = Some([0; 32]);
    assert!(
        queue
            .validate_resume(prepared.job_id, prepared.generation, &divergent)
            .is_err()
    );
    let mut divergent = resumed.clone();
    divergent.source_path.push_str(".changed");
    assert!(
        queue
            .validate_resume(prepared.job_id, prepared.generation, &divergent)
            .is_err()
    );

    let (prepared_again, prefix) = ArchiveStore::new(&destination)
        .resume_from_with_qualification(
            &prepared,
            resumed.clone(),
            Some(&model_path_text),
            Some(qualified_key()),
        )
        .expect("resume copies only confirmed prefix");
    assert_eq!(prefix.len(), 1);
    assert_eq!(prefix[0].text, "Confirmed prefix.");
    assert_eq!(prepared_again.generation, resumed.generation);
    let resumed_segments = destination
        .join("transcriptions")
        .join(format!("{:032x}", prepared.job_id.0))
        .join(resumed.generation.get().to_string())
        .join("segments.jsonl");
    let rows: Vec<serde_json::Value> = fs::read_to_string(resumed_segments)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["text"], "Confirmed prefix.");
    let entry = queue
        .resume_interrupted(prepared.job_id, prepared.generation, resumed)
        .expect("queue status advances to resumed generation");
    assert_eq!(entry.status, QueueStatus::Running);
    assert_eq!(
        QueueStore::open(&destination).unwrap().entries()[0].status,
        QueueStatus::Running
    );
}

#[test]
fn archive_admission_accepts_exactly_thirty_minutes_and_refuses_one_extra_sample() {
    let root = archive_root().join("parakeet-duration-boundary");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let max_samples = whisper_core::parakeet_ipc::PARAKEET_MAX_AUDIO_SAMPLES as u64;
    let exact = root.join("exact-30-min.wav");
    let over = root.join("over-30-min-one-sample.wav");
    write_silence_wav(&exact, max_samples);
    write_silence_wav(&over, max_samples + 1);
    let model_path = required_path("PARAKEET_L2_MODEL");
    let record = model(ModelId::ParakeetTdt06BV3);
    for (job, source, expected) in [(0x1a7, exact, Ok(())), (0x1a8, over, Err(()))] {
        let request = ImportRequest {
            job_id: JobId(job),
            generation: Generation::first(),
            source_path: source.to_string_lossy().into_owned(),
            source_sha256: None,
            source_samples: None,
            model_path: model_path.to_string_lossy().into_owned(),
            model_sha256: decode_sha256(record.artifact.sha256),
            destination: root
                .join(format!("archive-{job}"))
                .to_string_lossy()
                .into_owned(),
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Cpu,
            },
        };
        let result = ArchiveStore::new(&request.destination).prepare_with_qualification(
            request,
            None,
            Some(qualified_key()),
        );
        match expected {
            Ok(()) => assert_eq!(result.unwrap().source_samples, Some(max_samples)),
            Err(()) => assert!(result.unwrap_err().contains("DurationUnsupported")),
        }
    }
}

fn write_silence_wav(path: &std::path::Path, samples: u64) {
    let data_bytes = samples * 2;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .unwrap();
    file.write_all(b"RIFF").unwrap();
    file.write_all(&((36 + data_bytes) as u32).to_le_bytes())
        .unwrap();
    file.write_all(b"WAVEfmt ").unwrap();
    file.write_all(&16_u32.to_le_bytes()).unwrap();
    file.write_all(&1_u16.to_le_bytes()).unwrap();
    file.write_all(&1_u16.to_le_bytes()).unwrap();
    file.write_all(&16_000_u32.to_le_bytes()).unwrap();
    file.write_all(&32_000_u32.to_le_bytes()).unwrap();
    file.write_all(&2_u16.to_le_bytes()).unwrap();
    file.write_all(&16_u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&(data_bytes as u32).to_le_bytes()).unwrap();
    file.set_len(44 + data_bytes).unwrap();
    file.sync_all().unwrap();
}

#[test]
#[ignore = "runs real Parakeet CPU import and verifies the published archive"]
fn real_parakeet_import_publishes_complete_txt_and_srt() {
    let runtime = required_path("PARAKEET_L2_RUNTIME_ROOT");
    let archive_root = archive_root().join("parakeet-real-import");
    let _ = fs::remove_dir_all(&archive_root);
    let model_path = required_path("PARAKEET_L2_MODEL");
    let worker = required_path("PARAKEET_L2_WORKER");
    let native = required_path("PARAKEET_L2_NATIVE_DIR");
    let turbo_worker = runtime.join("worker/not-used-turbo.exe");
    let catalog_root = model_path.parent().unwrap().to_path_buf();
    let historical_turbo = runtime.join("historical/ggml-large-v3-turbo.bin");
    let io = AsyncImportIo::start_with_model_catalog_and_parakeet(
        WorkerPaths {
            cpu: turbo_worker.clone(),
            gpu: turbo_worker,
        },
        catalog_root,
        historical_turbo.to_string_lossy().into_owned(),
        Some(whisper_adapters::parakeet_import::ParakeetWorkerPaths {
            executable: worker,
            native_dir: native,
            model: model_path.clone(),
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
                model_path.to_string_lossy().into_owned(),
            ),
            (
                ModelId::WhisperLargeV3Turbo,
                historical_turbo.to_string_lossy().into_owned(),
            ),
        ],
    );
    let mut app = ImportApplication::new(io).with_model_guard(guard);
    let request = ImportRequest {
        job_id: JobId(0x1a4),
        generation: Generation::first(),
        source_path: required_path("PARAKEET_L2_EN_WAV")
            .to_string_lossy()
            .into_owned(),
        source_sha256: None,
        source_samples: None,
        model_path: model_path.to_string_lossy().into_owned(),
        model_sha256: decode_sha256(model(ModelId::ParakeetTdt06BV3).artifact.sha256),
        destination: archive_root.to_string_lossy().into_owned(),
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        },
    };
    app.dispatch(AppCommand::StartImport { request }).unwrap();
    let deadline = Instant::now() + Duration::from_secs(240);
    loop {
        let view = app
            .dispatch(AppCommand::Refresh)
            .unwrap_or_else(|error| panic!("refresh failed: {error:?}; view={:?}", app.view()));
        if view.last_result.is_some() {
            break;
        }
        if let Some((_, state)) = view.active_job {
            assert_ne!(
                state,
                whisper_core::JobState::Recoverable,
                "real import failed: {view:?}"
            );
            assert_ne!(
                state,
                whisper_core::JobState::Interrupted,
                "real import was interrupted: {view:?}"
            );
        }
        assert!(
            Instant::now() < deadline,
            "real Parakeet import timed out: {view:?}"
        );
        thread::sleep(Duration::from_millis(20));
    }
    let passage = archive_root.join("transcriptions/000000000000000000000000000001a4/1");
    assert!(passage.join("transcript.txt").is_file());
    assert!(passage.join("transcript.srt").is_file());
    assert_eq!(
        fs::read_dir(&passage)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| matches!(
                entry.file_name().to_str(),
                Some("transcript.txt" | "transcript.srt")
            ))
            .count(),
        2,
        "successful completion publishes one TXT and one SRT"
    );
    let entries = ArchiveStore::scan(&archive_root).expect("scan real archive");
    assert!(
        entries
            .iter()
            .any(|entry| entry.job_id == JobId(0x1a4) && entry.complete)
    );
}

fn decode_sha256(value: &str) -> [u8; 32] {
    let mut bytes = [0; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).unwrap();
    }
    bytes
}
