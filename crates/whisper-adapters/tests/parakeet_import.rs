use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

use sha2::{Digest, Sha256};
use whisper_adapters::{
    archive::ArchiveStore,
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
    let key = whisper_core::model_registry::configuration_key(
        record,
        whisper_core::model_catalog::Backend::Cpu,
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

    let mismatch_path = runtime.join("worker/mismatched-worker.exe");
    fs::copy(&paths.executable, &mismatch_path).expect("copy worker in runtime scope");
    OpenOptions::new()
        .append(true)
        .open(&mismatch_path)
        .unwrap()
        .write_all(b"mismatch")
        .unwrap();
    let mismatch = ParakeetWorkerPaths {
        executable: mismatch_path.clone(),
        ..paths.clone()
    };
    assert!(
        mismatch
            .verify_qualification(
                record.artifact.sha256,
                &key.runtime_build,
                &key.worker_build,
            )
            .unwrap_err()
            .starts_with("QualificationMismatch")
    );
    fs::remove_file(mismatch_path).unwrap();

    let missing_dll_dir = runtime.join("native/missing-dll-test");
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
#[ignore = "exercises qualification refusal and native loader/export failure on private DLL copies"]
fn incompatible_native_dll_is_refused_by_product_and_fails_at_loader_boundary() {
    let runtime = required_path("PARAKEET_L2_RUNTIME_ROOT");
    let model_path = required_path("PARAKEET_L2_MODEL");
    let worker = required_path("PARAKEET_L2_WORKER");
    let native = required_path("PARAKEET_L2_NATIVE_DIR");
    let incompatible_dir = runtime
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
    let destination = runtime.join(format!("archive/r8-loader-{}", std::process::id()));
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
    let deadline = Instant::now() + Duration::from_secs(30);
    let refusal_message = loop {
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        if view.active_job == Some((job_id, whisper_core::JobState::Recoverable)) {
            break view.message.unwrap_or_default();
        }
        assert!(
            Instant::now() < deadline,
            "qualification refusal missing: {view:?}"
        );
        thread::sleep(Duration::from_millis(10));
    };
    assert!(refusal_message.contains("QualificationMismatch"));
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
    let runtime = required_path("PARAKEET_L2_RUNTIME_ROOT");
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
    assert!(runtime.join("archive").exists());
}

#[test]
#[ignore = "injects an incomplete Result frame at the process transport boundary"]
fn incomplete_result_frame_is_rejected_as_transport_decode_failure() {
    let runtime = required_path("PARAKEET_L2_RUNTIME_ROOT");
    let fake_source = runtime.join("worker/incomplete-result-worker.rs");
    let fake_worker = runtime.join("worker/incomplete-result-worker.exe");
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
    let source = required_path("PARAKEET_L2_EN_WAV");
    let source_sha_before = format!("{:x}", Sha256::digest(fs::read(&source).unwrap()));
    let model_path = required_path("PARAKEET_L2_MODEL");
    let destination = runtime.join(format!("archive/product-{signal}-{}", std::process::id()));
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
