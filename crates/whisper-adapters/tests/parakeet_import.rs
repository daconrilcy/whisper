use std::{
    fs::{self, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use sha2::{Digest, Sha256};
use whisper_adapters::{
    archive::ArchiveStore,
    model_catalog::LocalModelCatalog,
    parakeet_import::{ParakeetProcess, ParakeetWorkerPaths},
    supervisor::WorkerPaths,
    worker_ipc::AsyncImportIo,
};
use whisper_core::{
    AppCommand, Application, ComputeChoice, Generation, ImportApplication, ImportRequest,
    JobConfig, JobId, LanguageChoice, model_catalog::ModelId,
};

#[test]
fn parakeet_worker_configuration_is_confined_to_the_l2a2_runtime() {
    let runtime = PathBuf::from(
        std::env::var_os("PARAKEET_L2_RUNTIME_ROOT")
            .expect("PARAKEET_L2_RUNTIME_ROOT is required by L2a-2 tests"),
    );
    let paths = ParakeetWorkerPaths {
        executable: PathBuf::from(std::env::var_os("PARAKEET_L2_WORKER").expect("worker path")),
        native_dir: PathBuf::from(std::env::var_os("PARAKEET_L2_NATIVE_DIR").expect("native dir")),
        model: PathBuf::from(std::env::var_os("PARAKEET_L2_MODEL").expect("model path")),
        runtime_root: runtime.clone(),
    };
    assert!(paths.validate().is_ok());
    let record =
        whisper_core::model_registry::model(whisper_core::model_catalog::ModelId::ParakeetTdt06BV3);
    let key = paths
        .qualification_key(whisper_core::parakeet_windows::PROFILE_IMPORT_VAD)
        .expect("the exact L2b import-vad Test key is present");
    assert!(
        paths
            .qualification_key(whisper_core::parakeet_windows::PROFILE_QUALIFICATION_REFERENCE)
            .unwrap_err()
            .starts_with("ProfileUnavailable")
    );
    paths
        .verify_qualification(
            record.artifact.sha256,
            &key.runtime_build,
            &key.worker_build,
        )
        .expect("the complete recorded Test key is present");

    let absent = ParakeetWorkerPaths {
        executable: runtime.join("worker/absent-worker.exe"),
        ..paths.clone()
    };
    assert!(
        absent
            .verify_qualification(
                record.artifact.sha256,
                &key.runtime_build,
                &key.worker_build,
            )
            .is_err()
    );

    let mismatch = paths.clone();
    assert!(
        mismatch
            .verify_qualification(record.artifact.sha256, &key.runtime_build, &"0".repeat(64),)
            .unwrap_err()
            .starts_with("QualificationMismatch")
    );
    let test_root = required_path("PARAKEET_R6_TEST_ROOT");
    let missing_dll_dir = test_root.join("native/missing-dll-test");
    fs::create_dir_all(&missing_dll_dir).unwrap();
    for name in [
        "nemo_speech_asr_c.dll",
        "nemo_speech_asr.dll",
        "ggml.dll",
        "ggml-base.dll",
        "ggml-cpu.dll",
    ] {
        fs::copy(paths.native_dir.join(name), missing_dll_dir.join(name))
            .expect("copy runtime DLL in scoped runtime");
    }
    fs::remove_file(missing_dll_dir.join("ggml.dll")).unwrap();
    let missing_dll = ParakeetWorkerPaths {
        native_dir: missing_dll_dir.clone(),
        ..paths.clone()
    };
    assert!(
        missing_dll
            .verify_qualification(
                record.artifact.sha256,
                &key.runtime_build,
                &key.worker_build,
            )
            .is_err()
    );
    fs::remove_dir_all(missing_dll_dir).unwrap();

    let outside = ParakeetWorkerPaths {
        executable: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
        ..paths
    };
    assert!(
        outside
            .validate()
            .unwrap_err()
            .starts_with("PathOutsideRuntime")
    );
}

#[test]
#[ignore = "R6 single-pass recipe uses the real Parakeet worker and records app timing"]
fn r6_application_single_pass_recipe() {
    let case = std::env::var("PARAKEET_R6_CASE").expect("PARAKEET_R6_CASE");
    let (samples, job_id) = match case.as_str() {
        "en5" => (5 * 60 * 16_000, 0x6a5),
        "en10" => (10 * 60 * 16_000, 0x6a10),
        "en30" => (30 * 60 * 16_000, 0x6a30),
        _ => panic!("unsupported R6 application case {case:?}"),
    };
    let runtime = required_path("PARAKEET_L2_RUNTIME_ROOT");
    let model_path = required_path("PARAKEET_L2_MODEL");
    let worker = required_path("PARAKEET_L2_WORKER");
    let native = required_path("PARAKEET_L2_NATIVE_DIR");
    let source = required_path("PARAKEET_R6_RECIPE_WAV");
    let destination = required_path("PARAKEET_L2_ARCHIVE_ROOT")
        .join("recipes")
        .join(&case);
    let _ = fs::remove_dir_all(&destination);
    fs::create_dir_all(destination.parent().expect("recipe destination parent")).unwrap();
    let turbo_worker = runtime.join("worker/not-used-turbo.exe");
    let historical_turbo = runtime.join("historical/ggml-large-v3-turbo.bin");
    let io = AsyncImportIo::start_with_model_catalog_and_parakeet(
        WorkerPaths {
            cpu: turbo_worker.clone(),
            gpu: turbo_worker,
        },
        model_path.parent().unwrap().to_path_buf(),
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
        job_id: JobId(job_id),
        generation: Generation::first(),
        source_path: source.to_string_lossy().into_owned(),
        source_sha256: None,
        source_samples: None,
        model_path: model_path.to_string_lossy().into_owned(),
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
    let started = Instant::now();
    app.dispatch(AppCommand::StartImport { request }).unwrap();
    let timeout = Duration::from_secs(samples / 16_000 / 4 + 120);
    let deadline = started + timeout;
    loop {
        let view = app.dispatch(AppCommand::Refresh).unwrap_or_else(|error| {
            panic!("R6 {case} refresh failed: {error:?}; view={:?}", app.view())
        });
        if view.last_result.is_some() {
            break;
        }
        if let Some((_, state)) = view.active_job {
            assert_ne!(
                state,
                whisper_core::JobState::Recoverable,
                "R6 {case} failed: {view:?}"
            );
            assert_ne!(
                state,
                whisper_core::JobState::Interrupted,
                "R6 {case} interrupted: {view:?}"
            );
        }
        assert!(
            Instant::now() < deadline,
            "R6 {case} exceeded {timeout:?}: {view:?}"
        );
        thread::sleep(Duration::from_millis(100));
    }
    let elapsed = started.elapsed();
    let passage = destination
        .join("transcriptions")
        .join(format!("{job_id:032x}"))
        .join("1");
    assert!(
        passage.join("transcript.txt").is_file(),
        "R6 {case} TXT missing"
    );
    assert!(
        passage.join("transcript.srt").is_file(),
        "R6 {case} SRT missing"
    );
    let timing = serde_json::json!({
        "case": case,
        "sample_count": samples,
        "application_elapsed_ms": elapsed.as_millis(),
        "timeout_seconds": timeout.as_secs(),
    });
    let log = required_path("PARAKEET_L2_LOG_DIR").join(format!("{case}-application.jsonl"));
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log)
        .unwrap();
    serde_json::to_writer(&mut file, &timing).unwrap();
    file.write_all(b"\n").unwrap();
    eprintln!(
        "R6_APP case={case} samples={samples} elapsed_ms={} timeout_s={}",
        elapsed.as_millis(),
        timeout.as_secs()
    );
}

#[test]
#[ignore = "R7 diagnostic recipe; uses the frozen R6 worker and writes only under R7"]
fn r7_application_timing() {
    let case = std::env::var("PARAKEET_R7_CASE").expect("PARAKEET_R7_CASE");
    let (samples, job_id) = match case.as_str() {
        "en5" => (5 * 60 * 16_000, 0x7a5),
        "en30" => (30 * 60 * 16_000, 0x7a30),
        _ => panic!("unsupported R7 application case {case:?}"),
    };
    let runtime = required_path("PARAKEET_L2_RUNTIME_ROOT");
    let model_path = required_path("PARAKEET_L2_MODEL");
    let worker = required_path("PARAKEET_L2_WORKER");
    let native = required_path("PARAKEET_L2_NATIVE_DIR");
    let source = required_path("PARAKEET_R7_RECIPE_WAV");
    let destination = required_path("PARAKEET_L2_ARCHIVE_ROOT")
        .join("recipes")
        .join(&case);
    let _ = fs::remove_dir_all(&destination);
    fs::create_dir_all(destination.parent().expect("recipe destination parent")).unwrap();
    let worker_sha = sha256(&worker);
    assert_eq!(
        worker_sha,
        "1d8f57b9e92d69629618be422077a0508303b9ae10d622b5f5aaad8867e103c0"
    );
    let turbo_worker = runtime.join("worker/not-used-turbo.exe");
    let historical_turbo = runtime.join("historical/ggml-large-v3-turbo.bin");
    let io = AsyncImportIo::start_with_model_catalog_and_parakeet(
        WorkerPaths {
            cpu: turbo_worker.clone(),
            gpu: turbo_worker,
        },
        model_path.parent().unwrap().to_path_buf(),
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
        job_id: JobId(job_id),
        generation: Generation::first(),
        source_path: source.to_string_lossy().into_owned(),
        source_sha256: None,
        source_samples: None,
        model_path: model_path.to_string_lossy().into_owned(),
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
    let started = Instant::now();
    app.dispatch(AppCommand::StartImport { request }).unwrap();
    let timeout = Duration::from_secs(samples / 16_000 / 4 + 120);
    let deadline = started + timeout;
    let mut refresh_count = 0_u64;
    let mut poll_sleep_ms = 0_u64;
    loop {
        refresh_count += 1;
        let view = app.dispatch(AppCommand::Refresh).unwrap_or_else(|error| {
            panic!("R7 {case} refresh failed: {error:?}; view={:?}", app.view())
        });
        if view.last_result.is_some() {
            break;
        }
        if let Some((_, state)) = view.active_job {
            assert_ne!(
                state,
                whisper_core::JobState::Recoverable,
                "R7 {case} failed: {view:?}"
            );
            assert_ne!(
                state,
                whisper_core::JobState::Interrupted,
                "R7 {case} interrupted: {view:?}"
            );
        }
        assert!(
            Instant::now() < deadline,
            "R7 {case} exceeded {timeout:?}: {view:?}"
        );
        thread::sleep(Duration::from_millis(100));
        poll_sleep_ms += 100;
    }
    let app_elapsed = started.elapsed();
    let passage = destination
        .join("transcriptions")
        .join(format!("{job_id:032x}"))
        .join("1");
    let verify_started = Instant::now();
    assert!(
        passage.join("transcript.txt").is_file(),
        "R7 {case} TXT missing"
    );
    assert!(
        passage.join("transcript.srt").is_file(),
        "R7 {case} SRT missing"
    );
    let txt_bytes = fs::metadata(passage.join("transcript.txt")).unwrap().len();
    let srt_bytes = fs::metadata(passage.join("transcript.srt")).unwrap().len();
    let verify_elapsed = verify_started.elapsed();
    let report = serde_json::json!({
        "case": case, "source_sha256": sha256(&source), "model_sha256": sha256(&model_path),
        "frozen_worker_sha256": worker_sha, "sample_count": samples,
        "application_request_to_result_ms": app_elapsed.as_millis(),
        "harness_refresh_count": refresh_count,
        "harness_poll_sleep_ms": poll_sleep_ms,
        "harness_txt_srt_presence_verification_ms": verify_elapsed.as_millis(),
        "txt_bytes": txt_bytes, "srt_bytes": srt_bytes, "timeout_seconds": timeout.as_secs(),
    });
    let log = required_path("PARAKEET_R7_TIMING_LOG");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log)
        .unwrap();
    serde_json::to_writer(&mut file, &report).unwrap();
    file.write_all(b"\n").unwrap();
    eprintln!("R7_APP case={case} report={report}");
}

#[test]
#[ignore = "R9 Test recipe uses the frozen worker and records adapter SHA cache metrics"]
fn r9_application_recipe() {
    let case = std::env::var("PARAKEET_R9_CASE").expect("PARAKEET_R9_CASE");
    let source = required_path("PARAKEET_R9_RECIPE_WAV");
    let samples = pcm16_mono_sample_count(&source);
    let runtime = required_path("PARAKEET_L2_RUNTIME_ROOT");
    let model_path = required_path("PARAKEET_L2_MODEL");
    let worker = required_path("PARAKEET_L2_WORKER");
    let native = required_path("PARAKEET_L2_NATIVE_DIR");
    let archive_root = required_path("PARAKEET_L2_ARCHIVE_ROOT");
    let destination = archive_root.join("recipes").join(&case);
    let _ = fs::remove_dir_all(&destination);
    fs::create_dir_all(destination.parent().expect("R9 destination parent")).unwrap();

    let record = whisper_core::model_registry::model(ModelId::ParakeetTdt06BV3);
    let worker_sha = sha256(&worker);
    assert_eq!(
        worker_sha, "1d8f57b9e92d69629618be422077a0508303b9ae10d622b5f5aaad8867e103c0",
        "R9 must execute the frozen, qualified R6 worker"
    );
    let mut worker_sha_by_start = Vec::new();
    assert_eq!(
        sha256(&model_path),
        record.artifact.sha256,
        "R9 input model must match the qualified Test artifact"
    );

    let timing_log = required_path("PARAKEET_R6_TIMING_LOG");
    fs::create_dir_all(timing_log.parent().expect("native timing log parent")).unwrap();
    let timing_start = fs::metadata(&timing_log).map_or(0, |metadata| metadata.len());

    if case == "fr87" {
        let verification_worker_sha = sha256(&worker);
        assert_eq!(verification_worker_sha, worker_sha);
        worker_sha_by_start.push(verification_worker_sha);
        let mut process = ParakeetProcess::spawn(&configured_paths())
            .expect("spawn frozen worker for FR87 word interval assertion");
        let identity = wait_for_ready(&mut process);
        process.accept_ready(identity).unwrap();
        let probe_job = format!("{:032x}", 0x9aff_u128);
        process
            .transcribe(probe_job.clone(), 1, 1, &source, sha256(&source))
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(120);
        let result = loop {
            match process.poll().expect("FR87 worker result is readable") {
                Some(whisper_core::parakeet_ipc::Event::Result {
                    job_id: returned_job,
                    words,
                    ..
                }) => {
                    assert_eq!(returned_job, probe_job);
                    break words;
                }
                Some(event) => panic!("unexpected FR87 worker event: {event:?}"),
                None => {
                    assert!(Instant::now() < deadline, "FR87 word probe timed out");
                    thread::sleep(Duration::from_millis(10));
                }
            }
        };
        process
            .shutdown()
            .expect("FR87 word probe worker shuts down");
        let boundary = result
            .windows(2)
            .find(|pair| pair[0].text == "faire?" && pair[1].text == "Je")
            .unwrap_or_else(|| panic!("FR87 worker result lacks `faire? / Je`: {result:?}"));
        assert!(
            boundary[0].start_ms < boundary[0].end_ms,
            "FR87 `faire?` has a non-positive interval: {:?}",
            boundary[0]
        );
        assert!(
            boundary[1].start_ms < boundary[1].end_ms,
            "FR87 `Je` has a non-positive interval: {:?}",
            boundary[1]
        );
    }

    let catalog_started = Instant::now();
    let startup_catalog = LocalModelCatalog::new(model_path.parent().unwrap()).inspect(1);
    let catalog_startup_elapsed = catalog_started.elapsed();
    for required_model in [ModelId::ParakeetTdt06BV3, ModelId::WhisperLargeV3Turbo] {
        let entry = startup_catalog
            .entries
            .iter()
            .find(|entry| entry.id == required_model)
            .unwrap_or_else(|| panic!("R9 startup catalog omitted {required_model:?}"));
        assert_eq!(
            entry.availability,
            whisper_core::model_catalog::ModelAvailability::Ready,
            "R9 startup catalog must admit {required_model:?} before the import request: {entry:?}"
        );
    }
    let reads_before = whisper_adapters::model_catalog::model_sha256_full_read_count(&model_path)
        .expect("model identity is available before import");

    let job_id = JobId(
        0x9a00
            + match case.as_str() {
                "en5" => 5,
                "en30" => 30,
                "fr87" => 87,
                "fr27" => 27,
                "en15" => 15,
                _ => panic!("unsupported R9 recipe {case:?}"),
            },
    );
    let turbo_worker = runtime.join("worker/not-used-turbo.exe");
    let historical_turbo = runtime.join("historical/ggml-large-v3-turbo.bin");
    let io = AsyncImportIo::start_with_model_catalog_and_parakeet(
        WorkerPaths {
            cpu: turbo_worker.clone(),
            gpu: turbo_worker,
        },
        model_path.parent().unwrap().to_path_buf(),
        historical_turbo.to_string_lossy().into_owned(),
        Some(whisper_adapters::parakeet_import::ParakeetWorkerPaths {
            executable: worker.clone(),
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
        job_id,
        generation: Generation::first(),
        source_path: source.to_string_lossy().into_owned(),
        source_sha256: None,
        source_samples: Some(samples),
        model_path: model_path.to_string_lossy().into_owned(),
        model_sha256: decode_sha256(record.artifact.sha256),
        destination: destination.to_string_lossy().into_owned(),
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        },
    };
    let application_worker_sha = sha256(&worker);
    assert_eq!(application_worker_sha, worker_sha);
    worker_sha_by_start.push(application_worker_sha);
    let started = Instant::now();
    app.dispatch(AppCommand::StartImport { request }).unwrap();
    let timeout = Duration::from_secs(samples / 16_000 / 4 + 120);
    let deadline = started + timeout;
    let mut refresh_count = 0_u64;
    let mut poll_wait_count = 0_u64;
    loop {
        refresh_count += 1;
        let view = app.dispatch(AppCommand::Refresh).unwrap_or_else(|error| {
            panic!("R9 {case} refresh failed: {error:?}; view={:?}", app.view())
        });
        if view.last_result.is_some() {
            break;
        }
        if let Some((_, state)) = view.active_job {
            assert_ne!(
                state,
                whisper_core::JobState::Recoverable,
                "R9 {case} failed: {view:?}"
            );
            assert_ne!(
                state,
                whisper_core::JobState::Interrupted,
                "R9 {case} interrupted: {view:?}"
            );
        }
        assert!(
            Instant::now() < deadline,
            "R9 {case} exceeded {timeout:?}: {view:?}"
        );
        thread::sleep(Duration::from_millis(33));
        poll_wait_count += 1;
    }
    let elapsed = started.elapsed();
    let passage = destination
        .join("transcriptions")
        .join(format!("{:032x}", job_id.0))
        .join("1");
    let txt_path = passage.join("transcript.txt");
    let srt_path = passage.join("transcript.srt");
    assert!(txt_path.is_file(), "R9 {case} TXT missing");
    assert!(srt_path.is_file(), "R9 {case} SRT missing");
    let txt = fs::read_to_string(&txt_path).unwrap();
    let srt = fs::read_to_string(&srt_path).unwrap();
    assert!(
        !txt.is_empty() && !srt.is_empty(),
        "R9 {case} output is empty"
    );
    assert!(!txt.contains('\u{fffd}'), "R9 {case} TXT contains U+FFFD");
    assert!(!srt.contains('\u{fffd}'), "R9 {case} SRT contains U+FFFD");
    if case == "fr87" {
        assert!(
            txt.split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .contains("faire? Je"),
            "R9 FR87 boundary: {txt}"
        );
    }
    if case == "fr27" {
        assert!(
            txt.split_whitespace()
                .last()
                .is_some_and(|word| word.chars().any(char::is_alphabetic)),
            "R9 FR27 final token is not valid text: {txt}"
        );
    }
    assert!(
        ArchiveStore::scan(&destination)
            .unwrap()
            .iter()
            .any(|entry| entry.job_id == job_id && entry.complete)
    );

    let reads_after = whisper_adapters::model_catalog::model_sha256_full_read_count(&model_path)
        .expect("model identity remains available after import");
    assert!(
        reads_after.saturating_sub(reads_before) <= 1,
        "R9 {case} adapter read the model more than once: before={reads_before}, after={reads_after}"
    );
    let bytes = fs::read(&timing_log).expect("frozen worker appended its inference timing");
    let added = std::str::from_utf8(&bytes[timing_start as usize..]).unwrap();
    let expected_timing_case = format!("r9-{case}");
    let expected_worker_starts = if case == "fr87" { 2 } else { 1 };
    let timing_records = added
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|record| {
            record.get("case").and_then(serde_json::Value::as_str)
                == Some(expected_timing_case.as_str())
        })
        .collect::<Vec<_>>();
    assert_eq!(
        timing_records.len(),
        expected_worker_starts,
        "R9 {case} expected one native inference record per worker start, found {}: {added}",
        timing_records.len()
    );
    let inference = timing_records
        .last()
        .expect("application worker inference record is present");
    let inference_ms = inference
        .get("inference_elapsed_ms")
        .and_then(serde_json::Value::as_u64)
        .expect("native inference elapsed time is present");
    let en5_latency_pass =
        case != "en5" || elapsed.as_millis() <= u128::from(inference_ms) + 30_000;
    let mut worker_starts = expected_worker_starts;
    let mut reads_after_second_import = reads_after;
    let mut second_import_elapsed_ms = None;
    let mut second_import_poll_wait_count = 0_u64;
    let mut second_import_refresh_count = 0_u64;
    if case == "en5" {
        let reads_after_first_import = reads_after;
        let second_source = required_path("PARAKEET_L2_EN_WAV");
        let second_destination = archive_root.join("recipes").join("en5-second-import");
        let _ = fs::remove_dir_all(&second_destination);
        let second_job_id = JobId(job_id.0 + 1);
        let second_request = ImportRequest {
            job_id: second_job_id,
            generation: Generation::first(),
            source_path: second_source.to_string_lossy().into_owned(),
            source_sha256: None,
            source_samples: Some(pcm16_mono_sample_count(&second_source)),
            model_path: model_path.to_string_lossy().into_owned(),
            model_sha256: decode_sha256(record.artifact.sha256),
            destination: second_destination.to_string_lossy().into_owned(),
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Cpu,
            },
        };
        let second_started = Instant::now();
        let second_worker_sha = sha256(&worker);
        assert_eq!(second_worker_sha, worker_sha);
        worker_sha_by_start.push(second_worker_sha);
        app.dispatch(AppCommand::StartImport {
            request: second_request,
        })
        .unwrap();
        worker_starts += 1;
        let second_deadline = second_started + Duration::from_secs(125);
        loop {
            second_import_refresh_count += 1;
            let view = app.dispatch(AppCommand::Refresh).unwrap_or_else(|error| {
                panic!(
                    "R9 EN5 second import refresh failed: {error:?}; view={:?}",
                    app.view()
                )
            });
            if view.active_job == Some((second_job_id, whisper_core::JobState::Complete)) {
                break;
            }
            if let Some((_, state)) = view.active_job {
                assert_ne!(
                    state,
                    whisper_core::JobState::Recoverable,
                    "R9 EN5 second import failed: {view:?}"
                );
                assert_ne!(
                    state,
                    whisper_core::JobState::Interrupted,
                    "R9 EN5 second import interrupted: {view:?}"
                );
            }
            assert!(
                Instant::now() < second_deadline,
                "R9 EN5 second import timed out: {view:?}"
            );
            thread::sleep(Duration::from_millis(33));
            second_import_poll_wait_count += 1;
        }
        second_import_elapsed_ms = Some(second_started.elapsed().as_millis());
        reads_after_second_import =
            whisper_adapters::model_catalog::model_sha256_full_read_count(&model_path).unwrap();
        assert_eq!(
            reads_after_second_import, reads_after_first_import,
            "R9 second import reread unchanged model"
        );
        let second_passage = second_destination
            .join("transcriptions")
            .join(format!("{:032x}", second_job_id.0))
            .join("1");
        assert!(
            second_passage.join("transcript.txt").is_file()
                && second_passage.join("transcript.srt").is_file(),
            "R9 EN5 second import did not write TXT/SRT"
        );
        eprintln!(
            "R9_SECOND_IMPORT adapter_model_full_reads_before={reads_after_first_import} after={reads_after_second_import}"
        );
    }

    let worker_sha_checks = worker_sha_by_start.len();
    assert_eq!(
        worker_sha_checks, worker_starts,
        "R9 {case} must verify the frozen worker SHA once per worker start"
    );
    let report = serde_json::json!({
        "case": case,
        "source_sha256": sha256(&source),
        "model_sha256": record.artifact.sha256,
        "worker_sha256": worker_sha.clone(),
        "worker_sha256_by_start": worker_sha_by_start,
        "worker_sha_checks": worker_sha_checks,
        "native_inference_records_for_execution": timing_records.len(),
        "sample_count": samples,
        "application_elapsed_ms": elapsed.as_millis(),
        "application_refresh_count": refresh_count,
        "harness_poll_wait_count": poll_wait_count,
        "harness_poll_wait_ms_at_33ms": poll_wait_count * 33,
        "inference_elapsed_ms": inference_ms,
        "adapter_model_full_reads_before": reads_before,
        "adapter_model_full_reads_after": reads_after,
        "adapter_model_full_reads_after_second_import": reads_after_second_import,
        "model_catalog_startup_elapsed_ms": catalog_startup_elapsed.as_millis(),
        "worker_starts": worker_starts,
        "second_import_elapsed_ms": second_import_elapsed_ms,
        "second_import_refresh_count": second_import_refresh_count,
        "second_import_poll_wait_count": second_import_poll_wait_count,
        "second_import_poll_wait_ms_at_33ms": second_import_poll_wait_count * 33,
        "en5_latency_pass": en5_latency_pass,
        "timeout_seconds": timeout.as_secs(),
        "txt": txt_path,
        "srt": srt_path,
    });
    let report_path =
        required_path("PARAKEET_L2_LOG_DIR").join(format!("{case}-r9-application.jsonl"));
    let mut report_file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(report_path)
        .unwrap();
    serde_json::to_writer(&mut report_file, &report).unwrap();
    report_file.write_all(b"\n").unwrap();
    eprintln!("R9_APP {report}");
    assert!(
        en5_latency_pass,
        "R9 EN5 total {} ms exceeds inference {} ms + 30 s",
        elapsed.as_millis(),
        inference_ms
    );
}

fn pcm16_mono_sample_count(path: &std::path::Path) -> u64 {
    let mut file = fs::File::open(path).expect("R9 WAV exists");
    let mut header = [0u8; 12];
    file.read_exact(&mut header)
        .expect("R9 WAV header is complete");
    assert_eq!(&header[0..4], b"RIFF", "R9 fixture is RIFF WAV");
    assert_eq!(&header[8..12], b"WAVE", "R9 fixture is WAVE");
    let mut format = None;
    let mut data_bytes = None;
    loop {
        let mut chunk = [0u8; 8];
        if file.read_exact(&mut chunk).is_err() {
            break;
        }
        let size = u32::from_le_bytes(chunk[4..8].try_into().unwrap());
        let payload_start = file.stream_position().unwrap();
        if &chunk[0..4] == b"fmt " {
            assert!(size >= 16, "R9 WAV fmt chunk is too short");
            let mut fmt = [0u8; 16];
            file.read_exact(&mut fmt).unwrap();
            format = Some((
                u16::from_le_bytes(fmt[0..2].try_into().unwrap()),
                u16::from_le_bytes(fmt[2..4].try_into().unwrap()),
                u32::from_le_bytes(fmt[4..8].try_into().unwrap()),
                u16::from_le_bytes(fmt[14..16].try_into().unwrap()),
            ));
        } else if &chunk[0..4] == b"data" {
            data_bytes = Some(size);
        }
        file.seek(SeekFrom::Start(
            payload_start + u64::from(size) + u64::from(size % 2),
        ))
        .unwrap();
        if format.is_some() && data_bytes.is_some() {
            break;
        }
    }
    let (encoding, channels, rate, bits) = format.expect("R9 WAV fmt chunk exists");
    assert_eq!(encoding, 1, "R9 fixture must be integer PCM");
    assert_eq!(channels, 1, "R9 fixture must be mono");
    assert_eq!(rate, 16_000, "R9 fixture must be 16 kHz");
    assert_eq!(bits, 16, "R9 fixture must be PCM16");
    u64::from(data_bytes.expect("R9 WAV data chunk exists") / 2)
}

#[test]
#[ignore = "exercises qualification refusal and native loader/export failure on private DLL copies"]
fn incompatible_native_dll_is_refused_by_product_and_fails_at_loader_boundary() {
    let runtime = required_path("PARAKEET_L2_RUNTIME_ROOT");
    let execution_root = r8_exec_root();
    let model_path = required_path("PARAKEET_L2_MODEL");
    let worker = required_path("PARAKEET_L2_WORKER");
    let native = required_path("PARAKEET_L2_NATIVE_DIR");
    let incompatible_dir = execution_root
        .join("native")
        .join(format!("r8-incompatible-{}", std::process::id()));
    fs::create_dir_all(&incompatible_dir).unwrap();
    for name in [
        "nemo_speech_asr_c.dll",
        "nemo_speech_asr.dll",
        "ggml.dll",
        "ggml-base.dll",
        "ggml-cpu.dll",
    ] {
        fs::copy(native.join(name), incompatible_dir.join(name)).unwrap();
    }
    fs::copy(
        native.join("ggml.dll"),
        incompatible_dir.join("nemo_speech_asr_c.dll"),
    )
    .unwrap();

    let record = whisper_core::model_registry::model(ModelId::ParakeetTdt06BV3);
    let key = whisper_core::model_registry::configuration_key(
        record,
        whisper_core::model_catalog::Backend::Cpu,
    );
    let bad_paths = ParakeetWorkerPaths {
        executable: worker.clone(),
        native_dir: incompatible_dir.clone(),
        model: model_path.clone(),
        runtime_root: runtime.clone(),
    };
    let refusal = bad_paths
        .verify_qualification(
            record.artifact.sha256,
            &key.runtime_build,
            &key.worker_build,
        )
        .unwrap_err();
    assert!(refusal.starts_with("QualificationMismatch: runtime="));

    let source = required_path("PARAKEET_L2_EN_WAV");
    let destination = execution_root.join(format!("archive/r8-loader-{}", std::process::id()));
    fs::create_dir_all(&destination).unwrap();
    let dummy_turbo = runtime.join("worker/not-used-turbo.exe");
    let io = AsyncImportIo::start_with_model_catalog_and_parakeet(
        WorkerPaths {
            cpu: dummy_turbo.clone(),
            gpu: dummy_turbo,
        },
        model_path.parent().unwrap().to_path_buf(),
        runtime
            .join("historical/ggml-large-v3-turbo.bin")
            .to_string_lossy()
            .into_owned(),
        Some(bad_paths.clone()),
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
                runtime
                    .join("historical/ggml-large-v3-turbo.bin")
                    .to_string_lossy()
                    .into_owned(),
            ),
        ],
    );
    let mut app = ImportApplication::new(io).with_model_guard(guard);
    let job_id = JobId(0x2a4);
    let generation = Generation::first();
    let request = ImportRequest {
        job_id,
        generation,
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
    app.dispatch(AppCommand::StartImport { request }).unwrap();
    // Preparing can include several minutes of adapter/model setup on the
    // constrained qualification host; this is a harness ceiling, not a
    // product response-time criterion.
    let started = Instant::now();
    let deadline = started + Duration::from_secs(240);
    let refusal_message = loop {
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        if view.active_job == Some((job_id, whisper_core::JobState::Recoverable)) {
            eprintln!("R8 loader refusal observed after {:?}", started.elapsed());
            break view.message.unwrap_or_default();
        }
        assert!(
            Instant::now() < deadline,
            "qualification refusal missing: {view:?}"
        );
        thread::sleep(Duration::from_millis(10));
    };
    assert!(
        refusal_message.contains("ModelNotQualified: exact Test worker/runtime key is required"),
        "unexpected refusal: {refusal_message}"
    );
    let passage = passage_dir(&destination, job_id, generation);
    assert!(
        !passage.join("pending.json").exists(),
        "qualification refusal happens before archive admission"
    );
    assert!(!passage.join("transcript.txt").exists());
    assert!(!passage.join("transcript.srt").exists());

    let output = Command::new(&worker)
        .arg("--model")
        .arg(&model_path)
        .arg("--model-sha256")
        .arg(record.artifact.sha256)
        .arg("--native-dir")
        .arg(&incompatible_dir)
        .env("PARAKEET_L2_RUNTIME_ROOT", &runtime)
        .stdin(Stdio::null())
        .output()
        .expect("direct worker loader probe starts from the scoped copy");
    assert!(
        !output.status.success(),
        "incompatible ABI must fail before Ready"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("parakeet-worker fatal:"), "{stderr}");
    assert!(
        stderr.to_ascii_lowercase().contains("procedure")
            || stderr.to_ascii_lowercase().contains("symbol")
            || stderr.contains("GetProcAddress"),
        "loader/export failure phase is not attributed: {stderr}"
    );
    assert!(String::from_utf8_lossy(&output.stdout).is_empty());
    assert!(source.is_file());
    assert!(!passage.join("pending.json").exists());
    assert!(!passage.join("transcript.txt").exists());
    assert!(!passage.join("transcript.srt").exists());
}

#[test]
#[ignore = "kills a qualified native worker during an active inference request"]
fn killed_native_worker_is_reported_as_exit_without_result() {
    let execution_root = r8_exec_root();
    let source = required_path("PARAKEET_L2_EN_WAV");
    let mut process = ParakeetProcess::spawn(&configured_paths()).unwrap();
    let identity = wait_for_ready(&mut process);
    process.accept_ready(identity).unwrap();
    let bytes = fs::read(&source).unwrap();
    let source_sha = format!("{:x}", Sha256::digest(bytes));
    process
        .transcribe(
            "00000000000000000000000000002a5".into(),
            1,
            1,
            &source,
            source_sha,
        )
        .unwrap();
    let status = process.terminate().expect("kill and wait for child exit");
    assert!(!status.success());
    let error = poll_until_error(&mut process, Duration::from_secs(10));
    assert!(error.starts_with("WorkerExited:"), "{error}");
    assert!(!error.contains("Result"));
    assert!(execution_root.join("archive").exists());
}

#[test]
#[ignore = "injects an incomplete Result frame at the process transport boundary"]
fn incomplete_result_frame_is_rejected_as_transport_decode_failure() {
    let execution_root = r8_exec_root();
    let worker_dir = execution_root.join("worker");
    fs::create_dir_all(&worker_dir).unwrap();
    let fake_source = worker_dir.join("incomplete-result-worker.rs");
    let fake_worker = worker_dir.join("incomplete-result-worker.exe");
    fs::write(
        &fake_source,
        "fn main() { println!(\"{{\\\"version\\\":1,\\\"message\\\":{{\\\"type\\\":\\\"result\\\"}}}}\"); }\n",
    )
    .unwrap();
    let compile = Command::new("rustc")
        .arg("--edition=2024")
        .arg(&fake_source)
        .arg("-o")
        .arg(&fake_worker)
        .output()
        .expect("Rust toolchain is available for scoped transport fixture");
    assert!(
        compile.status.success(),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let mut paths = configured_paths();
    paths.executable = fake_worker;
    let mut process = ParakeetProcess::spawn(&paths).unwrap();
    let error = poll_until_error(&mut process, Duration::from_secs(10));
    assert!(error.starts_with("IpcDecode:"), "{error}");
    assert!(error.contains("request_id"), "{error}");
}

#[test]
#[ignore = "routes a worker death from the qualified Test process through ImportApplication"]
fn product_route_worker_death_preserves_pending_source_and_rejects_completion() {
    run_product_transport_failure("worker-death", "WorkerExited", JobId(0x2a6));
}

#[test]
#[ignore = "routes an incomplete Result transport failure through ImportApplication"]
fn product_route_incomplete_result_preserves_pending_source_and_rejects_completion() {
    run_product_transport_failure("incomplete-result", "IpcDecode", JobId(0x2a7));
}

fn run_product_transport_failure(signal: &str, expected_error: &str, job_id: JobId) {
    let runtime = required_path("PARAKEET_L2_RUNTIME_ROOT");
    let execution_root = r8_exec_root();
    let source = required_path("PARAKEET_L2_EN_WAV");
    let source_sha_before = format!("{:x}", Sha256::digest(fs::read(&source).unwrap()));
    let model_path = required_path("PARAKEET_L2_MODEL");
    let destination =
        execution_root.join(format!("archive/product-{signal}-{}", std::process::id()));
    let control = destination.join(".l2a2-acceptance");
    fs::create_dir_all(&control).unwrap();
    // Arm before launch; the adapter consumes the signal only after accepting
    // the exact worker identity and sending Transcribe.
    fs::write(control.join(format!("{signal}.enabled")), b"enabled").unwrap();
    let model_record = whisper_core::model_registry::model(ModelId::ParakeetTdt06BV3);
    let worker = required_path("PARAKEET_L2_WORKER");
    let native = required_path("PARAKEET_L2_NATIVE_DIR");
    let dummy_turbo = runtime.join("worker/not-used-turbo.exe");
    let io = AsyncImportIo::start_with_model_catalog_and_parakeet(
        WorkerPaths {
            cpu: dummy_turbo.clone(),
            gpu: dummy_turbo,
        },
        model_path.parent().unwrap().to_path_buf(),
        runtime
            .join("historical/ggml-large-v3-turbo.bin")
            .to_string_lossy()
            .into_owned(),
        Some(ParakeetWorkerPaths {
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
                runtime
                    .join("historical/ggml-large-v3-turbo.bin")
                    .to_string_lossy()
                    .into_owned(),
            ),
        ],
    );
    let mut app = ImportApplication::new(io).with_model_guard(guard);
    let generation = Generation::first();
    let request = ImportRequest {
        job_id,
        generation,
        source_path: source.to_string_lossy().into_owned(),
        source_sha256: None,
        source_samples: None,
        model_path: model_path.to_string_lossy().into_owned(),
        model_sha256: decode_sha256(model_record.artifact.sha256),
        destination: destination.to_string_lossy().into_owned(),
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        },
    };
    app.dispatch(AppCommand::StartImport { request }).unwrap();
    let deadline = Instant::now() + Duration::from_secs(240);
    loop {
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        match view.active_job {
            Some((active, whisper_core::JobState::Running)) if active == job_id => break,
            Some((active, whisper_core::JobState::Recoverable)) if active == job_id => {
                panic!("qualified product import failed before worker engagement: {view:?}")
            }
            _ => {}
        }
        assert!(
            Instant::now() < deadline,
            "worker did not become active: {view:?}"
        );
        thread::sleep(Duration::from_millis(20));
    }
    let passage = passage_dir(&destination, job_id, generation);
    assert!(passage.join("pending.json").is_file());
    let deadline = Instant::now() + Duration::from_secs(30);
    let failure_view = loop {
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        if view.active_job == Some((job_id, whisper_core::JobState::Recoverable)) {
            break view;
        }
        assert!(
            Instant::now() < deadline,
            "failure was not routed: {view:?}"
        );
        thread::sleep(Duration::from_millis(10));
    };
    let message = failure_view.message.unwrap_or_default();
    assert!(message.contains(expected_error), "{message}");
    assert!(control.join(format!("{signal}.entered")).is_file());
    assert!(failure_view.last_result.is_none());
    assert!(passage.join("pending.json").is_file());
    assert!(!passage.join("transcript.txt").exists());
    assert!(!passage.join("transcript.srt").exists());
    assert_eq!(
        format!("{:x}", Sha256::digest(fs::read(&source).unwrap())),
        source_sha_before,
        "source bytes survive transport failure"
    );
    assert!(!message.to_ascii_lowercase().contains("fallback"));
    assert!(!message.to_ascii_lowercase().contains("turbo"));
    assert!(
        ArchiveStore::scan(&destination)
            .unwrap()
            .iter()
            .all(|entry| entry.job_id != job_id || !entry.complete),
        "failed product route did not publish Complete"
    );
}

fn configured_paths() -> ParakeetWorkerPaths {
    ParakeetWorkerPaths {
        executable: required_path("PARAKEET_L2_WORKER"),
        native_dir: required_path("PARAKEET_L2_NATIVE_DIR"),
        model: required_path("PARAKEET_L2_MODEL"),
        runtime_root: required_path("PARAKEET_L2_RUNTIME_ROOT"),
    }
}

fn wait_for_ready(process: &mut ParakeetProcess) -> whisper_core::parakeet_ipc::WorkerIdentity {
    let deadline = Instant::now() + Duration::from_secs(90);
    loop {
        if let Some(whisper_core::parakeet_ipc::Event::Ready { identity, .. }) =
            process.poll().unwrap()
        {
            return identity;
        }
        assert!(Instant::now() < deadline, "worker did not send Ready");
        thread::sleep(Duration::from_millis(10));
    }
}

fn poll_until_error(process: &mut ParakeetProcess, timeout: Duration) -> String {
    let deadline = Instant::now() + timeout;
    loop {
        match process.poll() {
            Ok(Some(event)) => panic!("unexpected worker event instead of failure: {event:?}"),
            Ok(None) => {}
            Err(error) => return error,
        }
        assert!(
            Instant::now() < deadline,
            "worker transport failure not observed"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn required_path(name: &str) -> PathBuf {
    PathBuf::from(std::env::var_os(name).unwrap_or_else(|| panic!("{name} is required")))
}

fn r8_exec_root() -> PathBuf {
    let root = required_path("PARAKEET_R8_EXEC_ROOT")
        .canonicalize()
        .expect("R8 execution root exists");
    let r8 = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/parakeet-tests/l2/l2b/r8")
        .canonicalize()
        .expect("canonical R8 execution root exists");
    let r9 = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/parakeet-tests/l2/l2b/r9")
        .canonicalize()
        .expect("canonical R9 execution root exists");
    assert!(
        root == r8 || root == r9,
        "R8 scenarios are confined to the canonical R8 or R9 execution root"
    );
    root
}

fn sha256(path: &std::path::Path) -> String {
    let mut file = fs::File::open(path).unwrap();
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher).unwrap();
    format!("{:x}", hasher.finalize())
}

fn decode_sha256(value: &str) -> [u8; 32] {
    let mut bytes = [0; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).unwrap();
    }
    bytes
}

fn passage_dir(root: &std::path::Path, job_id: JobId, generation: Generation) -> PathBuf {
    root.join("transcriptions")
        .join(format!("{:032x}", job_id.0))
        .join(generation.get().to_string())
}
