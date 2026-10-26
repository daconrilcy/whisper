use std::{
    fs::{self, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime},
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
    if not_run_without_campaign_variables(&[
        "PARAKEET_L2_RUNTIME_ROOT",
        "PARAKEET_L2_MODEL",
        "PARAKEET_L2_WORKER",
        "PARAKEET_L2_NATIVE_DIR",
    ]) {
        return;
    }
    let runtime = PathBuf::from(
        std::env::var_os("PARAKEET_L2_RUNTIME_ROOT")
            .expect("PARAKEET_L2_RUNTIME_ROOT is required by L2a-2 tests"),
    );
    let paths = ParakeetWorkerPaths {
        executable: cpu_worker_path(),
        native_dir: cpu_native_dir(),
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
fn gpu_app_fr_en_single_pass() {
    if !selected_recipe("gpu_app_fr_en_single_pass") {
        return;
    }
    for (label, variable) in [
        ("fr87", "PARAKEET_L3_FR87_WAV"),
        ("fr27", "PARAKEET_L3_FR27_WAV"),
        ("en15", "PARAKEET_L2_EN_WAV"),
    ] {
        run_gpu_application_import(
            label,
            &required_path(variable),
            ComputeChoice::Auto,
            true,
            None,
        );
    }
}

#[test]
#[cfg(feature = "personal-qualification")]
#[ignore = "L6-perso release recipe exercises the staged CPU or GPU Parakeet worker"]
fn l6_personal_application_recipe() {
    let backend = std::env::var("PARAKEET_L6P_BACKEND").expect("guarded L6-perso backend");
    let case = std::env::var("PARAKEET_L6P_CASE").expect("guarded L6-perso case");
    let source = required_path("PARAKEET_L6P_RECIPE_WAV");
    assert!(matches!(
        case.as_str(),
        "fr87" | "fr27" | "en15" | "en5" | "en30"
    ));
    let personal_runtime = (backend == "CUDA0").then(|| {
        required_path("PARAKEET_L2_RUNTIME_ROOT")
            .canonicalize()
            .expect("personal runtime root is canonical")
    });
    let legacy_log_names = personal_runtime.as_ref().map(|runtime| {
        fs::read_dir(runtime.join("logs"))
            .ok()
            .into_iter()
            .flatten()
            .map(|entry| entry.unwrap().file_name())
            .collect::<std::collections::BTreeSet<_>>()
    });
    run_personal_application_import(&case, &source, &backend, false);
    if let Some(runtime) = personal_runtime {
        let log_dir = runtime.join("settings/logs");
        let logs = fs::read_dir(&log_dir)
            .expect("GPU worker logs stay under the declared settings component")
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        assert!(!logs.is_empty(), "GPU import should capture the worker log");
        assert!(
            logs.iter().all(|entry| entry.path().starts_with(&runtime)),
            "worker logs must remain inside the personal runtime"
        );
        let legacy_log_names_after = fs::read_dir(runtime.join("logs"))
            .ok()
            .into_iter()
            .flatten()
            .map(|entry| entry.unwrap().file_name())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(legacy_log_names.as_ref(), Some(&legacy_log_names_after));
    }
}

#[test]
#[cfg(feature = "personal-qualification")]
#[ignore = "L6-perso GPU release recipe verifies manual refusal and Auto selection/fallback"]
fn l6_personal_gpu_manual_auto_routes() {
    assert_eq!(
        std::env::var("PARAKEET_L6P_BACKEND").as_deref(),
        Ok("CUDA0")
    );
    let source = required_path("PARAKEET_L2_EN_WAV");
    run_personal_application_route(
        "auto-qualified-gpu",
        &source,
        ComputeChoice::Auto,
        true,
        Some("CUDA0"),
        false,
    );
    run_personal_application_route(
        "auto-unqualified-cpu-parakeet",
        &source,
        ComputeChoice::Auto,
        false,
        Some("CPU"),
        false,
    );
    run_personal_application_route(
        "manual-unqualified-refused",
        &source,
        ComputeChoice::Gpu,
        false,
        None,
        false,
    );
}

#[test]
#[cfg(feature = "personal-qualification")]
#[ignore = "L6-perso release recipe verifies the first and second EN5 import timings"]
fn l6_personal_en5_twice_release() {
    let backend = std::env::var("PARAKEET_L6P_BACKEND").expect("guarded L6-perso backend");
    assert!(matches!(backend.as_str(), "CPU" | "CUDA0"));
    let source = required_path("PARAKEET_L3_EN5_WAV");
    let model = required_path("PARAKEET_L2_MODEL");
    let cpu_native = required_path("PARAKEET_L2_CPU_NATIVE_DIR");
    let gpu_native = required_path("PARAKEET_L6P_GPU_NATIVE_DIR");
    let artifact_paths = std::iter::once(required_path("PARAKEET_L2_CPU_WORKER"))
        .chain(std::iter::once(required_path("PARAKEET_L6P_GPU_WORKER")))
        .chain(
            [
                "nemo_speech_asr_c.dll",
                "nemo_speech_asr.dll",
                "ggml.dll",
                "ggml-base.dll",
                "ggml-cpu.dll",
                "ggml-cuda.dll",
                "cudart64_12.dll",
                "cublas64_12.dll",
                "cublasLt64_12.dll",
            ]
            .into_iter()
            .map(|name| cpu_native.join(name))
            .chain(
                [
                    "nemo_speech_asr_c.dll",
                    "nemo_speech_asr.dll",
                    "ggml.dll",
                    "ggml-base.dll",
                    "ggml-cpu.dll",
                    "ggml-cuda.dll",
                    "cudart64_12.dll",
                    "cublas64_12.dll",
                    "cublasLt64_12.dll",
                ]
                .into_iter()
                .map(|name| gpu_native.join(name)),
            ),
        )
        .collect::<Vec<_>>();
    let artifact_reads_before = artifact_paths
        .iter()
        .map(|path| {
            whisper_adapters::model_catalog::artifact_sha256_full_read_count(path).unwrap_or(0)
        })
        .collect::<Vec<_>>();
    let before = whisper_adapters::model_catalog::model_sha256_full_read_count(&model)
        .expect("model cache counter is available");
    let first = run_personal_application_import("en5-first", &source, &backend, false);
    let after_first = whisper_adapters::model_catalog::model_sha256_full_read_count(&model)
        .expect("model cache counter is available after first import");
    let artifact_reads_after_first = artifact_paths
        .iter()
        .map(|path| {
            whisper_adapters::model_catalog::artifact_sha256_full_read_count(path).unwrap_or(0)
        })
        .collect::<Vec<_>>();
    let second = run_personal_application_import("en5-second", &source, &backend, false);
    let after_second = whisper_adapters::model_catalog::model_sha256_full_read_count(&model)
        .expect("model cache counter is available after second import");
    let artifact_reads_after = artifact_paths
        .iter()
        .map(|path| {
            whisper_adapters::model_catalog::artifact_sha256_full_read_count(path).unwrap_or(0)
        })
        .collect::<Vec<_>>();
    assert!(first.as_millis() > 0 && second.as_millis() > 0);
    assert!(after_first.saturating_sub(before) <= 1);
    assert_eq!(after_second, after_first, "second import reread the model");
    for (((path, before), after_first), after_second) in artifact_paths
        .iter()
        .zip(artifact_reads_before)
        .zip(artifact_reads_after_first)
        .zip(artifact_reads_after)
    {
        assert!(
            after_first.saturating_sub(before) <= 1,
            "more than one adapter SHA read for {}",
            path.display()
        );
        assert_eq!(
            after_second,
            after_first,
            "second import reread worker or DLL {}",
            path.display()
        );
    }
    if backend == "CUDA0" {
        assert!(
            first <= Duration::from_secs(15),
            "first release GPU import took {first:?}"
        );
    }
    eprintln!(
        "L6P_EN5 backend={backend} first_total_ms={} second_total_ms={} adapter_model_reads_first={} adapter_model_reads_second={} worker_restart_each_import=true",
        first.as_millis(),
        second.as_millis(),
        after_first.saturating_sub(before),
        after_second.saturating_sub(after_first)
    );
}

#[test]
#[cfg(feature = "personal-qualification")]
#[ignore = "L6-perso release recipe checks the over-30-minute refusal before pending"]
fn l6_personal_rejects_over_limit_before_pending() {
    let backend = std::env::var("PARAKEET_L6P_BACKEND").expect("guarded L6-perso backend");
    let source = required_path("PARAKEET_L3_EN30_WAV");
    let output = required_path("PARAKEET_L6P_EXEC_ROOT").join("fixtures/over-limit-one-sample.wav");
    extend_pcm16_wav_by_one_sample(&source, &output);
    let elapsed = run_personal_application_import("over-limit", &output, &backend, true);
    assert!(
        elapsed.is_zero(),
        "over-limit input should be refused before worker startup"
    );
}

#[cfg(feature = "personal-qualification")]
fn run_personal_application_import(
    case: &str,
    source: &std::path::Path,
    backend: &str,
    expect_over_limit: bool,
) -> Duration {
    run_personal_application_route(
        case,
        source,
        if backend == "CPU" {
            ComputeChoice::Cpu
        } else {
            ComputeChoice::Gpu
        },
        true,
        Some(backend),
        expect_over_limit,
    )
}

#[cfg(feature = "personal-qualification")]
fn run_personal_application_route(
    case: &str,
    source: &std::path::Path,
    compute: ComputeChoice,
    candidate_valid: bool,
    expected_backend: Option<&str>,
    expect_over_limit: bool,
) -> Duration {
    let backend = std::env::var("PARAKEET_L6P_BACKEND").expect("guarded L6-perso backend");
    let runtime = required_path("PARAKEET_L2_RUNTIME_ROOT");
    let execution_root = required_path("PARAKEET_L2_ARCHIVE_ROOT");
    let model_path = required_path("PARAKEET_L2_MODEL");
    let cpu_paths = ParakeetWorkerPaths {
        executable: required_path("PARAKEET_L2_CPU_WORKER"),
        native_dir: required_path("PARAKEET_L2_CPU_NATIVE_DIR"),
        model: model_path.clone(),
        runtime_root: runtime.clone(),
    };
    let gpu_paths = ParakeetWorkerPaths {
        executable: required_path("PARAKEET_L6P_GPU_WORKER"),
        native_dir: required_path("PARAKEET_L6P_GPU_NATIVE_DIR"),
        model: model_path.clone(),
        runtime_root: runtime.clone(),
    };
    let key = if backend == "CPU" {
        cpu_paths.qualification_key(whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS)
    } else {
        gpu_paths
            .gpu_qualification_key(whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS)
    }
    .expect("staged artifact identity produces a candidate key");
    let cpu_key = cpu_paths
        .qualification_key(whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS)
        .expect("staged CPU candidate key");
    assert!(
        key.is_parakeet_import_test(),
        "candidate key must be confined to the L6 harness"
    );
    assert!(
        whisper_adapters::parakeet_import::ParakeetWorkerPaths::personal_candidate_keys_from_env()
            .contains(&key),
        "the exact selected candidate must be the only key injected for this recipe"
    );
    let destination = execution_root.join("recipes").join(case);
    let _ = fs::remove_dir_all(&destination);
    fs::create_dir_all(&destination).unwrap();
    let turbo_root = runtime.join("app");
    let io = AsyncImportIo::start_with_model_catalog_and_parakeet_routes(
        WorkerPaths {
            cpu: turbo_root.join("whisper-worker-cpu.exe"),
            gpu: turbo_root.join("whisper-worker-gpu.exe"),
        },
        model_path.parent().unwrap().to_path_buf(),
        runtime
            .join("models/ggml-large-v3-turbo.bin")
            .to_string_lossy()
            .into_owned(),
        Some(cpu_paths),
        Some(gpu_paths),
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
                    .join("models/ggml-large-v3-turbo.bin")
                    .to_string_lossy()
                    .into_owned(),
            ),
        ],
    );
    let mut app = ImportApplication::new(io).with_model_guard(guard);
    let job_id = JobId(
        0x6f6_u128
            + u128::from(NEXT_GPU_RECIPE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)),
    );
    let generation = Generation::first();
    if backend == "CUDA0" {
        let mut candidate = qualified_gpu_candidate();
        if !candidate_valid {
            candidate.pci_id = "0000:ff:00.0".into();
            candidate.pnp_instance_id = r"PCI\VEN_10DE&DEV_FAKE\foreign".into();
        }
        app.set_gpu_candidate(job_id, generation, Some(candidate))
            .unwrap();
    }
    let record = whisper_core::model_registry::model(ModelId::ParakeetTdt06BV3);
    let request = ImportRequest {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
        job_id,
        generation,
        source_path: source.to_string_lossy().into_owned(),
        source_sha256: None,
        source_samples: Some(pcm16_mono_sample_count(source)),
        model_path: model_path.to_string_lossy().into_owned(),
        model_sha256: decode_sha256(record.artifact.sha256),
        destination: destination.to_string_lossy().into_owned(),
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute,
        },
    };
    let started = Instant::now();
    app.dispatch(AppCommand::StartImport { request }).unwrap();
    let timeout = Duration::from_secs(pcm16_mono_sample_count(source) / 16_000 / 4 + 120);
    let deadline = started + timeout;
    loop {
        let view = app.dispatch(AppCommand::Refresh).unwrap_or_else(|error| {
            panic!(
                "L6-perso {backend}/{case} refresh failed: {error:?}; view={:?}",
                app.view()
            )
        });
        if expect_over_limit {
            if view.message.as_deref().is_some_and(|message| {
                message.contains("DurationUnsupported")
                    && message.contains("Fichier trop long pour Parakeet, utilisez Turbo")
            }) {
                let passage = destination
                    .join("transcriptions")
                    .join(format!("{:032x}", job_id.0))
                    .join("1");
                assert!(!passage.join("pending.json").exists());
                return Duration::ZERO;
            }
        } else if expected_backend.is_none() {
            if view
                .message
                .as_deref()
                .is_some_and(|message| message.contains("BackendMismatch"))
            {
                let passage = destination
                    .join("transcriptions")
                    .join(format!("{:032x}", job_id.0))
                    .join("1");
                assert!(
                    !passage.join("pending.json").exists(),
                    "manual GPU refusal must precede pending"
                );
                return Duration::ZERO;
            }
        } else if view.last_result.is_some() {
            let passage = destination
                .join("transcriptions")
                .join(format!("{:032x}", job_id.0))
                .join("1");
            let txt_path = passage.join("transcript.txt");
            let srt_path = passage.join("transcript.srt");
            assert!(txt_path.is_file(), "L6-perso TXT missing: {case}");
            assert!(srt_path.is_file(), "L6-perso SRT missing: {case}");
            let txt = fs::read_to_string(&txt_path).unwrap();
            let srt = fs::read_to_string(&srt_path).unwrap();
            assert!(!txt.is_empty() && !srt.is_empty());
            assert!(!txt.contains('\u{fffd}') && !srt.contains('\u{fffd}'));
            if case == "fr87" {
                assert!(
                    txt.split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" ")
                        .contains("faire? Je"),
                    "FR87 boundary: {txt}"
                );
            }
            if case == "fr27" {
                assert!(
                    txt.split_whitespace()
                        .last()
                        .is_some_and(|word| word.chars().any(char::is_alphabetic)),
                    "FR27 final token invalid: {txt}"
                );
            }
            let pending: serde_json::Value =
                serde_json::from_slice(&fs::read(passage.join("pending.json")).unwrap()).unwrap();
            let actual_backend =
                expected_backend.expect("successful route has an expected backend");
            let expected_key = if actual_backend == "CPU" {
                &cpu_key
            } else {
                &key
            };
            assert_eq!(
                pending["qualification"]["backend"].as_str(),
                Some(actual_backend)
            );
            assert_eq!(
                pending["qualification"]["worker_build"].as_str(),
                Some(expected_key.worker_build.as_str())
            );
            assert!(
                ArchiveStore::scan(&destination)
                    .unwrap()
                    .iter()
                    .any(|entry| entry.job_id == job_id && entry.complete)
            );
            eprintln!(
                "L6P_RECIPE backend={backend} case={case} actual_backend={actual_backend} elapsed_ms={} model={} worker_sha={}",
                started.elapsed().as_millis(),
                key.model,
                expected_key.worker_build
            );
            if case == "en5" && backend == "CUDA0" && !cfg!(debug_assertions) {
                assert!(
                    started.elapsed() <= Duration::from_secs(15),
                    "first EN5 GPU release import exceeded 15 s"
                );
            }
            return started.elapsed();
        }
        if view.active_job.is_some_and(|(_, state)| {
            matches!(
                state,
                whisper_core::JobState::Recoverable | whisper_core::JobState::Interrupted
            )
        }) {
            let archive_diagnostic = ArchiveStore::scan(&destination);
            panic!(
                "L6-perso {backend}/{case} did not complete cleanly: {view:?}; archive={archive_diagnostic:?}"
            );
        }
        assert!(
            Instant::now() < deadline,
            "L6-perso {backend}/{case} exceeded {timeout:?}: {view:?}"
        );
        thread::sleep(Duration::from_millis(33));
    }
}

#[cfg(feature = "personal-qualification")]
fn extend_pcm16_wav_by_one_sample(source: &std::path::Path, destination: &std::path::Path) {
    let mut bytes = fs::read(source).expect("read synthetic EN30 WAV");
    assert!(bytes.len() >= 44 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WAVE");
    let mut chunk_offset = 12usize;
    let data_offset = loop {
        assert!(chunk_offset + 8 <= bytes.len(), "R9 WAV data chunk exists");
        let chunk_size = u32::from_le_bytes(
            bytes[chunk_offset + 4..chunk_offset + 8]
                .try_into()
                .unwrap(),
        ) as usize;
        if &bytes[chunk_offset..chunk_offset + 4] == b"data" {
            break chunk_offset;
        }
        chunk_offset = chunk_offset
            .checked_add(8 + chunk_size + (chunk_size % 2))
            .expect("R9 WAV chunk offset does not overflow");
    };
    let size_offset = data_offset + 4;
    let data_size = u32::from_le_bytes(bytes[size_offset..size_offset + 4].try_into().unwrap());
    assert_eq!(data_size % 2, 0, "R9 WAV is PCM16 aligned");
    let data_end = data_offset + 8 + data_size as usize;
    bytes.splice(data_end..data_end, [0, 0]);
    let riff_size = (bytes.len() - 8) as u32;
    bytes[4..8].copy_from_slice(&riff_size.to_le_bytes());
    bytes[size_offset..size_offset + 4].copy_from_slice(&(data_size + 2).to_le_bytes());
    fs::create_dir_all(destination.parent().unwrap()).unwrap();
    fs::write(destination, bytes).unwrap();
}

#[test]
fn gpu_app_en5() {
    if !selected_recipe("gpu_app_en5") {
        return;
    }
    run_gpu_application_import(
        "en5",
        &required_path("PARAKEET_L3_EN5_WAV"),
        ComputeChoice::Gpu,
        true,
        None,
    );
}

#[test]
fn gpu_app_en5_timing_diagnostic() {
    if !selected_recipe("gpu_app_en5_timing_diagnostic") {
        return;
    }
    let view = run_gpu_application_import(
        "en5-timing-diagnostic",
        &required_path("PARAKEET_L3_EN5_WAV"),
        ComputeChoice::Gpu,
        true,
        None,
    );
    assert!(
        view.last_result.is_some(),
        "diagnostic import did not publish: {view:?}"
    );
}

#[test]
#[cfg(feature = "l3-gpu-qualification")]
fn gpu_app_en30() {
    if !selected_recipe("gpu_app_en30") {
        return;
    }
    run_gpu_application_import(
        "en30",
        &required_path("PARAKEET_L3_EN30_WAV"),
        ComputeChoice::Gpu,
        true,
        None,
    );
}

#[test]
#[cfg(feature = "l3-gpu-qualification")]
fn gpu_manual_and_auto_fallback() {
    if !selected_recipe("gpu_manual_and_auto_fallback") {
        return;
    }
    let source = required_path("PARAKEET_L2_EN_WAV");
    let manual = run_gpu_application_import(
        "manual-bad-candidate",
        &source,
        ComputeChoice::Gpu,
        false,
        None,
    );
    assert!(
        manual
            .message
            .as_deref()
            .is_some_and(|message| message.contains("BackendMismatch"))
    );
    let auto = run_gpu_application_import(
        "auto-cpu-fallback",
        &source,
        ComputeChoice::Auto,
        false,
        None,
    );
    assert!(
        auto.last_result.is_some(),
        "Auto must complete on the qualified CPU fallback: {auto:?}"
    );

    #[cfg(feature = "l3-gpu-qualification")]
    {
        let failed = run_gpu_application_import(
            "post-native-worker-death",
            &source,
            ComputeChoice::Auto,
            true,
            Some("worker-death"),
        );
        let failed = failed.message.unwrap_or_default();
        assert!(
            failed.contains("WorkerExited"),
            "post-native GPU failure was not reported: {failed}"
        );
        assert!(!failed.to_ascii_lowercase().contains("fallback"));
        assert!(!failed.to_ascii_lowercase().contains("cpu"));
    }
}

#[test]
#[cfg(feature = "l3-gpu-qualification")]
fn gpu_rejects_bad_identity_before_pending() {
    if !selected_recipe("gpu_rejects_bad_identity_before_pending") {
        return;
    }
    let source = required_path("PARAKEET_L2_EN_WAV");
    let view = run_gpu_application_import("bad-identity", &source, ComputeChoice::Gpu, false, None);
    assert!(
        view.message
            .as_deref()
            .is_some_and(|message| message.contains("BackendMismatch"))
    );
    let long = required_path("PARAKEET_L2_EN_LONG_WAV");
    let long_view = run_gpu_application_import("over-limit", &long, ComputeChoice::Gpu, true, None);
    assert!(long_view.message.as_deref().is_some_and(|message| {
        message.contains("DurationUnsupported")
            && message.contains("Fichier trop long pour Parakeet, utilisez Turbo")
    }));
}

fn recipe_selector_matches(
    name: &str,
    l3b: Option<&str>,
    l3c: Option<&str>,
    l3d: Option<&str>,
) -> bool {
    let mut selectors = [l3b, l3c, l3d].into_iter().flatten();
    let Some(selected) = selectors.next() else {
        return false;
    };
    selected == name && selectors.next().is_none()
}

fn selected_recipe(name: &str) -> bool {
    let l3b = std::env::var("PARAKEET_L3B_RECIPE_CASE").ok();
    let l3c = std::env::var("PARAKEET_L3C_RECIPE_CASE").ok();
    let l3d = std::env::var("PARAKEET_L3D_RECIPE_CASE").ok();
    if recipe_selector_matches(name, l3b.as_deref(), l3c.as_deref(), l3d.as_deref()) {
        true
    } else {
        let campaign = if l3d.is_some() {
            "L3d"
        } else if l3c.is_some() {
            "L3c"
        } else {
            "L3b"
        };
        eprintln!("{campaign}_RECIPE_NOT_RUN {name}: no matching guarded recipe selector");
        false
    }
}

#[test]
fn gpu_recipe_selector_preserves_l3b_l3c_and_supports_explicit_l3d() {
    assert!(recipe_selector_matches(
        "gpu_app_en5",
        Some("gpu_app_en5"),
        None,
        None
    ));
    assert!(recipe_selector_matches(
        "gpu_app_en5",
        None,
        Some("gpu_app_en5"),
        None
    ));
    assert!(recipe_selector_matches(
        "gpu_app_en5",
        None,
        None,
        Some("gpu_app_en5")
    ));
    assert!(!recipe_selector_matches(
        "gpu_app_en5",
        Some("gpu_app_en5"),
        Some("gpu_app_en30"),
        None
    ));
    assert!(!recipe_selector_matches("gpu_app_en5", None, None, None));
    assert!(!recipe_selector_matches(
        "gpu_app_en5",
        None,
        Some("gpu_app_en5"),
        Some("gpu_app_en5")
    ));
}

#[test]
fn gpu_app_en5_twice_release() {
    if !selected_recipe("gpu_app_en5_twice_release") {
        return;
    }
    let runtime = required_path("PARAKEET_L2_RUNTIME_ROOT");
    let execution_root = required_path("PARAKEET_L2_ARCHIVE_ROOT");
    let source = required_path("PARAKEET_L3_EN5_WAV");
    let model_path = required_path("PARAKEET_L2_MODEL");
    let gpu_worker = required_path("PARAKEET_L2_WORKER");
    let gpu_native = required_path("PARAKEET_L2_NATIVE_DIR");
    let cpu_worker = cpu_worker_path();
    let cpu_native = cpu_native_dir();
    let turbo_worker = runtime.join("worker/not-used-turbo.exe");
    let historical_turbo = runtime.join("historical/ggml-large-v3-turbo.bin");

    let artifact_paths = std::iter::once(gpu_worker.clone())
        .chain(
            [
                "cublas64_12.dll",
                "cublasLt64_12.dll",
                "cudart64_12.dll",
                "ggml-base.dll",
                "ggml-cpu.dll",
                "ggml-cuda.dll",
                "ggml.dll",
                "nemo_speech_asr.dll",
                "nemo_speech_asr_c.dll",
            ]
            .into_iter()
            .map(|name| gpu_native.join(name)),
        )
        .collect::<Vec<_>>();
    let reads_before = artifact_paths
        .iter()
        .map(|path| {
            whisper_adapters::model_catalog::artifact_sha256_full_read_count(path).unwrap_or(0)
        })
        .collect::<Vec<_>>();
    let model_reads_before =
        whisper_adapters::model_catalog::model_sha256_full_read_count(&model_path)
            .expect("model identity available before release recipe");

    let gpu_paths = ParakeetWorkerPaths {
        executable: gpu_worker,
        native_dir: gpu_native,
        model: model_path.clone(),
        runtime_root: runtime.clone(),
    };
    // The desktop performs this same qualification in its background catalog task
    // before the user can start an import. Keep it in this session's cache, but
    // outside the request-to-publication stopwatch.
    let preflight_started = Instant::now();
    let qualified_key = gpu_paths
        .gpu_qualification_key(whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS)
        .expect("the qualified L3b GPU route is ready before the import request");
    assert_eq!(
        qualified_key,
        whisper_adapters::archive::ArchiveQualificationKey::parakeet_gpu_test(),
        "match the exact GPU qualification completed by the desktop catalog before selection"
    );
    eprintln!(
        "L3C_RELEASE_PREFLIGHT elapsed_ms={} adapter_artifact_full_reads={} model_full_reads={}",
        preflight_started.elapsed().as_millis(),
        artifact_paths
            .iter()
            .map(
                |path| whisper_adapters::model_catalog::artifact_sha256_full_read_count(path)
                    .unwrap_or(0),
            )
            .sum::<u64>(),
        whisper_adapters::model_catalog::model_sha256_full_read_count(&model_path).unwrap()
    );
    let cpu_paths = ParakeetWorkerPaths {
        executable: cpu_worker,
        native_dir: cpu_native,
        model: model_path.clone(),
        runtime_root: required_path("PARAKEET_L2_CPU_RUNTIME_ROOT"),
    };
    let io = AsyncImportIo::start_with_model_catalog_and_parakeet_routes(
        WorkerPaths {
            cpu: turbo_worker.clone(),
            gpu: turbo_worker,
        },
        model_path.parent().unwrap().to_path_buf(),
        historical_turbo.to_string_lossy().into_owned(),
        Some(cpu_paths),
        Some(gpu_paths),
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
    let candidate = qualified_gpu_candidate();
    let record = whisper_core::model_registry::model(ModelId::ParakeetTdt06BV3);
    let archive_root = execution_root.join("en5-twice");
    let _ = fs::remove_dir_all(&archive_root);
    fs::create_dir_all(&archive_root).unwrap();
    let samples = pcm16_mono_sample_count(&source);
    let timeout = Duration::from_secs(samples / 16_000 / 4 + 120);
    let mut elapsed = Vec::new();
    let mut refreshes = Vec::new();
    let mut reads_after_import = Vec::new();

    for (index, name) in ["first", "second"].into_iter().enumerate() {
        let job_id = JobId(0x6c00 + index as u128);
        let destination = archive_root.join(name);
        fs::create_dir_all(&destination).unwrap();
        app.set_gpu_candidate(job_id, Generation::first(), Some(candidate.clone()))
            .unwrap();
        let request = ImportRequest {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
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
                compute: ComputeChoice::Gpu,
            },
        };
        let started = Instant::now();
        app.dispatch(AppCommand::StartImport { request }).unwrap();
        let deadline = started + timeout;
        let mut refresh_count = 0_u64;
        loop {
            refresh_count += 1;
            let view = app.dispatch(AppCommand::Refresh).unwrap_or_else(|error| {
                panic!(
                    "L3c {name} import refresh failed: {error:?}; view={:?}",
                    app.view()
                )
            });
            if view.active_job == Some((job_id, whisper_core::JobState::Complete)) {
                break;
            }
            if let Some((_, state)) = view.active_job {
                assert_ne!(
                    state,
                    whisper_core::JobState::Recoverable,
                    "L3c {name}: {view:?}"
                );
                assert_ne!(
                    state,
                    whisper_core::JobState::Interrupted,
                    "L3c {name}: {view:?}"
                );
            }
            assert!(
                Instant::now() < deadline,
                "L3c {name} exceeded {timeout:?}: {view:?}"
            );
            thread::sleep(Duration::from_millis(33));
        }
        let passage = destination
            .join("transcriptions")
            .join(format!("{:032x}", job_id.0))
            .join("1");
        let txt = fs::read(passage.join("transcript.txt")).expect("durable transcript TXT");
        let srt = fs::read(passage.join("transcript.srt")).expect("durable transcript SRT");
        assert!(
            !txt.is_empty() && !srt.is_empty(),
            "L3c {name} transcript files are empty"
        );
        let import_elapsed = started.elapsed();
        let pending: serde_json::Value =
            serde_json::from_slice(&fs::read(passage.join("pending.json")).unwrap()).unwrap();
        assert_eq!(pending["qualification"]["backend"].as_str(), Some("CUDA0"));
        assert_eq!(
            pending["qualification"]["worker_build"].as_str(),
            Some("5054612a05fbfd1039e108def3aa4e598268d2b4274b8c1dc96cd6b8e3f2cd6d")
        );
        elapsed.push(import_elapsed);
        refreshes.push(refresh_count);
        reads_after_import.push(
            artifact_paths
                .iter()
                .map(|path| {
                    whisper_adapters::model_catalog::artifact_sha256_full_read_count(path)
                        .unwrap_or(0)
                })
                .collect::<Vec<_>>(),
        );
        eprintln!(
            "L3C_RELEASE_IMPORT name={name} total_ms={} refresh_count={refresh_count} txt_bytes={} srt_bytes={}",
            import_elapsed.as_millis(),
            txt.len(),
            srt.len()
        );
    }

    for (index, (path, before)) in artifact_paths.iter().zip(reads_before).enumerate() {
        assert_eq!(
            reads_after_import[0][index] - before,
            1,
            "first import should hash artifact once: {}",
            path.display()
        );
        assert_eq!(
            reads_after_import[1][index] - reads_after_import[0][index],
            0,
            "second import must not reread artifact: {}",
            path.display()
        );
        let after = whisper_adapters::model_catalog::artifact_sha256_full_read_count(path).unwrap();
        assert_eq!(
            after - before,
            1,
            "artifact should be read once across both imports: {}",
            path.display()
        );
    }
    let model_reads_after =
        whisper_adapters::model_catalog::model_sha256_full_read_count(&model_path).unwrap();
    assert_eq!(
        model_reads_after - model_reads_before,
        1,
        "model should be read once across both imports"
    );
    assert_eq!(
        reads_after_import[1], reads_after_import[0],
        "second import must have zero new artifact reads"
    );
    let report = serde_json::json!({
        "first_total_ms": elapsed[0].as_millis(), "second_total_ms": elapsed[1].as_millis(),
        "first_refresh_count": refreshes[0], "second_refresh_count": refreshes[1],
        "adapter_artifact_full_reads": artifact_paths.len(), "adapter_model_full_reads": model_reads_after - model_reads_before,
        "worker_native_sha_is_separate": true,
    });
    let log = required_path("PARAKEET_R7_TIMING_LOG");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log)
        .unwrap();
    serde_json::to_writer(&mut file, &report).unwrap();
    file.write_all(b"\n").unwrap();
    eprintln!("L3C_RELEASE_SUMMARY {report}");
    assert!(
        elapsed[0] <= Duration::from_secs(15),
        "first EN5 release import exceeded 15 s: {:?}",
        elapsed[0]
    );
}

fn run_gpu_application_import(
    case: &str,
    source: &std::path::Path,
    compute: ComputeChoice,
    valid_candidate: bool,
    fault: Option<&str>,
) -> whisper_core::application::AppView {
    run_gpu_application_import_with_native(case, source, compute, valid_candidate, fault, None)
}

fn run_gpu_application_import_with_native(
    case: &str,
    source: &std::path::Path,
    compute: ComputeChoice,
    valid_candidate: bool,
    fault: Option<&str>,
    gpu_native_override: Option<PathBuf>,
) -> whisper_core::application::AppView {
    let runtime = required_path("PARAKEET_L2_RUNTIME_ROOT");
    let execution_root = required_path("PARAKEET_L2_ARCHIVE_ROOT");
    let model_path = required_path("PARAKEET_L2_MODEL");
    let gpu_worker = required_path("PARAKEET_L2_WORKER");
    let gpu_native = gpu_native_override.unwrap_or_else(|| required_path("PARAKEET_L2_NATIVE_DIR"));
    let cpu_worker = cpu_worker_path();
    let cpu_native = cpu_native_dir();
    let destination = execution_root.join("recipes").join(case);
    let _ = fs::remove_dir_all(&destination);
    fs::create_dir_all(&destination).unwrap();
    let turbo_worker = runtime.join("worker/not-used-turbo.exe");
    let historical_turbo = runtime.join("historical/ggml-large-v3-turbo.bin");
    let cpu_paths = ParakeetWorkerPaths {
        executable: cpu_worker,
        native_dir: cpu_native,
        model: model_path.clone(),
        runtime_root: runtime.clone(),
    };
    let gpu_paths = ParakeetWorkerPaths {
        executable: gpu_worker,
        native_dir: gpu_native,
        model: model_path.clone(),
        runtime_root: runtime.clone(),
    };
    let registered_gpu_key = gpu_paths
        .gpu_qualification_key(whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS)
        .ok()
        .is_some_and(|actual| {
            actual == whisper_adapters::archive::ArchiveQualificationKey::parakeet_gpu_test()
        });
    if valid_candidate && registered_gpu_key {
        assert_registered_gpu_key(&gpu_paths);
    }
    let io = AsyncImportIo::start_with_model_catalog_and_parakeet_routes(
        WorkerPaths {
            cpu: turbo_worker.clone(),
            gpu: turbo_worker,
        },
        model_path.parent().unwrap().to_path_buf(),
        historical_turbo.to_string_lossy().into_owned(),
        Some(cpu_paths),
        Some(gpu_paths),
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
    let job_id = JobId(
        0x6b0_u128
            + u128::from(NEXT_GPU_RECIPE_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)),
    );
    let generation = Generation::first();
    let mut candidate = qualified_gpu_candidate();
    if !valid_candidate {
        candidate.pci_id = "0000:ff:00.0".into();
        candidate.pnp_instance_id = r"PCI\VEN_10DE&DEV_FAKE\foreign".into();
    }
    app.set_gpu_candidate(job_id, generation, Some(candidate))
        .unwrap();
    if let Some(fault) = fault {
        let control = destination.join(".l2a2-acceptance");
        fs::create_dir_all(&control).unwrap();
        fs::write(control.join(format!("{fault}.enabled")), b"enabled").unwrap();
    }
    let record = whisper_core::model_registry::model(ModelId::ParakeetTdt06BV3);
    let request = ImportRequest {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
        job_id,
        generation,
        source_path: source.to_string_lossy().into_owned(),
        source_sha256: None,
        source_samples: Some(pcm16_mono_sample_count(source)),
        model_path: model_path.to_string_lossy().into_owned(),
        model_sha256: decode_sha256(record.artifact.sha256),
        destination: destination.to_string_lossy().into_owned(),
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute,
        },
    };
    let timeout = Duration::from_secs(pcm16_mono_sample_count(source) / 16_000 / 4 + 120);
    let deadline = Instant::now() + timeout;
    let started = Instant::now();
    let worker_log_started = SystemTime::now();
    app.dispatch(AppCommand::StartImport { request }).unwrap();
    let mut refresh_count = 0_u64;
    loop {
        refresh_count += 1;
        let view = app.dispatch(AppCommand::Refresh).unwrap_or_else(|error| {
            panic!(
                "L3b {case} refresh failed: {error:?}; view={:?}",
                app.view()
            )
        });
        if view.last_result.is_some()
            || view
                .active_job
                .is_some_and(|(_, state)| state == whisper_core::JobState::Recoverable)
        {
            if view.last_result.is_some() {
                let passage = destination
                    .join("transcriptions")
                    .join(format!("{:032x}", job_id.0))
                    .join(generation.get().to_string());
                assert!(
                    passage.join("transcript.txt").is_file(),
                    "L3b {case} TXT missing"
                );
                assert!(
                    passage.join("transcript.srt").is_file(),
                    "L3b {case} SRT missing"
                );
                let pending: serde_json::Value = serde_json::from_slice(
                    &fs::read(passage.join("pending.json")).expect("qualified request pending"),
                )
                .expect("pending qualification JSON");
                let qualification = &pending["qualification"];
                let expected_gpu = valid_candidate && registered_gpu_key;
                assert_eq!(
                    qualification["backend"].as_str(),
                    Some(if expected_gpu { "CUDA0" } else { "CPU" }),
                    "the archive records the actual backend without a silent fallback"
                );
                assert_eq!(
                    qualification["worker_build"].as_str(),
                    Some(
                        personal_worker_build(if expected_gpu { "CUDA0" } else { "CPU" })
                            .as_deref()
                            .unwrap_or(if expected_gpu {
                                "5054612a05fbfd1039e108def3aa4e598268d2b4274b8c1dc96cd6b8e3f2cd6d"
                            } else {
                                "1d8f57b9e92d69629618be422077a0508303b9ae10d622b5f5aaad8867e103c0"
                            })
                    )
                );
                assert_eq!(
                    qualification["model"].as_str(),
                    Some("parakeet-tdt-0.6b-v3"),
                    "Auto fallback stays on Parakeet rather than switching to Turbo"
                );
                if std::env::var_os("PARAKEET_L3D_TIMING_ACTIVE").is_some() {
                    eprintln!(
                        "L3D_TIMING_DIAGNOSTIC case={case} total_ms={} refresh_count={refresh_count} txt_bytes={} srt_bytes={} worker_log_started_unix_ms={}",
                        started.elapsed().as_millis(),
                        fs::metadata(passage.join("transcript.txt")).unwrap().len(),
                        fs::metadata(passage.join("transcript.srt")).unwrap().len(),
                        worker_log_started
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis()
                    );
                }
                return view;
            }
            let passage = destination
                .join("transcriptions")
                .join(format!("{:032x}", job_id.0))
                .join(generation.get().to_string());
            let over_single_pass_limit = pcm16_mono_sample_count(source)
                > whisper_core::model_registry::PARAKEET_MAX_SINGLE_PASS_DURATION_MS
                    .saturating_mul(16);
            if over_single_pass_limit {
                assert!(
                    !passage.join("pending.json").exists(),
                    "over-limit GPU input must be refused before pending admission: {view:?}"
                );
                return view;
            }
            if valid_candidate || compute == ComputeChoice::Auto {
                assert!(
                    passage.join("pending.json").is_file(),
                    "an admitted GPU or Auto CPU-fallback request must preserve pending on failure: {view:?}"
                );
                if let Some(signal) = fault {
                    assert!(
                        destination
                            .join(".l2a2-acceptance")
                            .join(format!("{signal}.entered"))
                            .is_file(),
                        "acceptance fault was not reached: {view:?}"
                    );
                    return view;
                }
                let diagnostics = adapter_diagnostics(&destination);
                let worker_stderr = worker_stderr_since(worker_log_started);
                panic!(
                    "L3b {case} qualified GPU import became recoverable after admission: state={:?}, message={:?}, adapter_diagnostics={diagnostics}, worker_stderr={worker_stderr}",
                    view.active_job, view.message
                );
            } else {
                assert!(
                    !passage.join("pending.json").exists(),
                    "invalid GPU candidate must be refused before pending admission: {view:?}"
                );
            }
            return view;
        }
        assert!(
            Instant::now() < deadline,
            "L3b {case} exceeded {timeout:?}: {view:?}"
        );
        thread::sleep(Duration::from_millis(
            if std::env::var_os("PARAKEET_L3D_TIMING_ACTIVE").is_some() {
                33
            } else {
                50
            },
        ));
    }
}

#[test]
fn gpu_identity_rejects_same_pci_with_a_different_windows_device() {
    let mut candidate = qualified_gpu_candidate();
    candidate.pnp_instance_id = r"PCI\VEN_10DE&DEV_1B06\different-instance".into();
    let error =
        whisper_adapters::parakeet_import::expected_gpu_for_candidate(&candidate).unwrap_err();
    assert!(error.starts_with("GpuIdentityMismatch"), "{error}");
}

fn adapter_diagnostics(destination: &std::path::Path) -> String {
    let directory = destination.join(".whisper-diagnostics");
    (0..4)
        .filter_map(|index| {
            let path = directory.join(format!("worker-{index}.jsonl"));
            fs::read_to_string(path)
                .ok()
                .filter(|contents| !contents.is_empty())
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn worker_stderr_since(started: SystemTime) -> String {
    let Ok(entries) = fs::read_dir(required_path("PARAKEET_L2_LOG_DIR")) else {
        return "<log directory unavailable>".into();
    };
    let mut paths = entries
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_name().to_string_lossy().starts_with("parakeet-")
                && entry
                    .metadata()
                    .and_then(|metadata| metadata.modified())
                    .is_ok_and(|modified| modified >= started)
        })
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    paths.sort();
    let contents = paths
        .iter()
        .filter_map(|path| fs::read_to_string(path).ok())
        .collect::<Vec<_>>();
    if contents.is_empty() {
        "<no captured worker stderr>".into()
    } else {
        contents.join("\n")
    }
}

static NEXT_GPU_RECIPE_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn personal_worker_build(backend: &str) -> Option<String> {
    if std::env::var_os("PARAKEET_L6P_STAGE").is_none() {
        return None;
    }
    #[cfg(feature = "personal-qualification")]
    {
        return whisper_adapters::parakeet_import::ParakeetWorkerPaths::personal_candidate_keys_from_env()
            .into_iter()
            .find(|key| key.backend == backend)
            .map(|key| key.worker_build);
    }
    #[cfg(not(feature = "personal-qualification"))]
    {
        let _ = backend;
        None
    }
}

fn qualified_gpu_candidate() -> whisper_core::gpu_inventory::GpuRuntimeIdentity {
    whisper_core::gpu_inventory::GpuRuntimeIdentity {
        generation: 1,
        pnp_instance_id: r"PCI\VEN_10DE&DEV_1B06&SUBSYS_85F11043&REV_A1\4&8BD6E8D&0&0008".into(),
        dxgi_rank: 0,
        luid: 0,
        pci_id: "0000:01:00.0".into(),
        cuda_ordinal: 0,
        ggml_ordinal: 0,
    }
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
    let worker = cpu_worker_path();
    let native = cpu_native_dir();
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
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
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
    let worker = cpu_worker_path();
    let native = cpu_native_dir();
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
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
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
    let worker = cpu_worker_path();
    let native = cpu_native_dir();
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
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
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
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
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
    ensure_staged_incompatible_native_fixtures(&runtime);
    assert_cpu_incompatible_native_refusal(&execution_root, &model_path);
    let worker = gpu_worker_path();
    let native = gpu_native_dir();
    assert_registered_gpu_key(&ParakeetWorkerPaths {
        executable: worker.clone(),
        native_dir: native.clone(),
        model: model_path.clone(),
        runtime_root: runtime.clone(),
    });
    let incompatible_dir = existing_incompatible_gpu_fixture(&runtime);
    let required_dlls = [
        "cublas64_12.dll",
        "cublasLt64_12.dll",
        "cudart64_12.dll",
        "nemo_speech_asr_c.dll",
        "nemo_speech_asr.dll",
        "ggml.dll",
        "ggml-base.dll",
        "ggml-cpu.dll",
        "ggml-cuda.dll",
    ];
    assert!(
        required_dlls
            .iter()
            .all(|name| incompatible_dir.join(name).is_file())
    );
    assert_eq!(
        sha256(&incompatible_dir.join("nemo_speech_asr_c.dll")),
        "9148ba2ed0eaeee385d09eba37e1a297d5e2f0649a02ffd4e325fea562c29ea6",
        "the preserved L3b fixture must retain its intentionally incompatible ABI DLL"
    );
    assert_eq!(
        sha256(&incompatible_dir.join("nemo_speech_asr_c.dll")),
        sha256(&incompatible_dir.join("ggml.dll")),
        "the preserved fixture records the deliberate ggml-to-NeMo ABI mismatch"
    );

    let record = whisper_core::model_registry::model(ModelId::ParakeetTdt06BV3);
    let personal_candidate = std::env::var_os("PARAKEET_L6P_STAGE").is_some();
    let bad_paths = ParakeetWorkerPaths {
        executable: worker.clone(),
        native_dir: incompatible_dir.clone(),
        model: model_path.clone(),
        runtime_root: runtime.clone(),
    };
    let bad_key = bad_paths
        .gpu_qualification_key(whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS);
    if personal_candidate {
        assert!(
            bad_key
                .as_ref()
                .is_err_and(|error| error.starts_with("ProfileUnavailable")),
            "Personal GPU route must reject a noncanonical native directory: {bad_key:?}"
        );
    } else {
        assert!(
            bad_key.is_ok(),
            "the altered native copy should yield a distinct candidate key for admission to reject"
        );
    }

    let source = required_path("PARAKEET_L2_EN_WAV");
    let destination = execution_root.join(format!("archive/r8-loader-{}", std::process::id()));
    fs::create_dir_all(&destination).unwrap();
    let dummy_turbo = runtime.join("worker/not-used-turbo.exe");
    let io = AsyncImportIo::start_with_model_catalog_and_parakeet_routes(
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
            executable: cpu_worker_path(),
            native_dir: cpu_native_dir(),
            model: model_path.clone(),
            runtime_root: runtime.clone(),
        }),
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
    app.set_gpu_candidate(job_id, generation, Some(qualified_gpu_candidate()))
        .unwrap();
    let request = ImportRequest {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
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
            compute: ComputeChoice::Gpu,
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
    if personal_candidate {
        assert!(
            refusal_message.contains("ProfileUnavailable"),
            "Personal route must explain why the incompatible DLL directory is refused: {refusal_message}"
        );
    } else {
        assert!(
            refusal_message.contains("ModelNotQualified: Parakeet GPU candidate is not admitted"),
            "unexpected refusal: {refusal_message}"
        );
    }
    let passage = passage_dir(&destination, job_id, generation);
    assert!(
        !passage.join("pending.json").exists(),
        "qualification refusal happens before archive admission"
    );
    assert!(!passage.join("transcript.txt").exists());
    assert!(!passage.join("transcript.srt").exists());

    let asr_log_dir = runtime.join("logs");
    fs::create_dir_all(&asr_log_dir).unwrap();
    let output = Command::new(&worker)
        .arg("--model")
        .arg(&model_path)
        .arg("--model-sha256")
        .arg(record.artifact.sha256)
        .arg("--native-dir")
        .arg(&incompatible_dir)
        .env("PARAKEET_L2_RUNTIME_ROOT", &runtime)
        .env(
            "PARAKEET_L3_ASR_LOG",
            asr_log_dir.join(format!(
                "{}-parakeet-asr-r8-incompatible-{}-{}.log",
                if std::env::var_os("PARAKEET_L3D_EXEC_ROOT").is_some() {
                    "l3d"
                } else {
                    "l3c"
                },
                std::process::id(),
                SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos()
            )),
        )
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

    let auto = run_gpu_application_import_with_native(
        "auto-bad-dll-cpu-fallback",
        &source,
        ComputeChoice::Auto,
        true,
        None,
        Some(incompatible_dir),
    );
    assert!(
        auto.last_result.is_some(),
        "Auto must complete through Parakeet CPU when the registered GPU DLL identity differs: {auto:?}"
    );
}

fn ensure_staged_incompatible_native_fixtures(runtime: &std::path::Path) {
    let fixture_root = runtime.join("r8/native");
    fs::create_dir_all(&fixture_root).expect("create isolated R8 fixture root");
    let source_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let cpu_root = source_root.join("target/parakeet-tests/l2/l2a-2/native");
    let gpu_root = source_root.join("target/parakeet-tests/l3/l3b/r8/native");
    let has_cpu_fixture = fs::read_dir(&fixture_root)
        .expect("staged R8 fixture root is readable")
        .filter_map(Result::ok)
        .any(|entry| {
            entry.file_type().is_ok_and(|kind| kind.is_dir())
                && entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("r8-incompatible-")
                && fs::read_dir(entry.path())
                    .is_ok_and(|files| files.filter_map(Result::ok).count() == 5)
        });
    if !has_cpu_fixture {
        for entry in fs::read_dir(&cpu_root)
            .expect("read preserved CPU R8 fixtures")
            .filter_map(Result::ok)
            .filter(|entry| {
                entry.file_type().is_ok_and(|kind| kind.is_dir())
                    && entry
                        .file_name()
                        .to_string_lossy()
                        .starts_with("r8-incompatible-")
            })
        {
            let destination = fixture_root.join(entry.file_name());
            if !destination.exists() {
                copy_fixture_directory(&entry.path(), &destination);
            }
        }
    }
    let has_gpu_fixture = fs::read_dir(&fixture_root)
        .expect("staged R8 fixture root is readable")
        .filter_map(Result::ok)
        .any(|entry| {
            entry.file_type().is_ok_and(|kind| kind.is_dir())
                && entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("r8-incompatible-")
                && [
                    "cublas64_12.dll",
                    "cublasLt64_12.dll",
                    "cudart64_12.dll",
                    "nemo_speech_asr_c.dll",
                    "nemo_speech_asr.dll",
                    "ggml.dll",
                    "ggml-base.dll",
                    "ggml-cpu.dll",
                    "ggml-cuda.dll",
                ]
                .iter()
                .all(|name| entry.path().join(name).is_file())
        });
    if !has_gpu_fixture {
        let candidate = fs::read_dir(&gpu_root)
            .expect("read preserved GPU R8 fixtures")
            .filter_map(Result::ok)
            .find(|entry| {
                entry.file_type().is_ok_and(|kind| kind.is_dir())
                    && entry
                        .file_name()
                        .to_string_lossy()
                        .starts_with("r8-incompatible-")
                    && sha256(&entry.path().join("nemo_speech_asr_c.dll"))
                        == "9148ba2ed0eaeee385d09eba37e1a297d5e2f0649a02ffd4e325fea562c29ea6"
            })
            .expect("preserved GPU fixture with the intentional ABI mismatch");
        let destination = fixture_root.join(candidate.file_name());
        if !destination.exists() {
            copy_fixture_directory(&candidate.path(), &destination);
        }
    }
}

fn copy_fixture_directory(source: &std::path::Path, destination: &std::path::Path) {
    fs::create_dir_all(destination).expect("create staged R8 fixture directory");
    for entry in fs::read_dir(source)
        .expect("read preserved R8 fixture directory")
        .filter_map(Result::ok)
    {
        if entry.file_type().is_ok_and(|kind| kind.is_file()) {
            fs::copy(entry.path(), destination.join(entry.file_name()))
                .expect("copy read-only R8 fixture into the staging runtime");
        }
    }
}

fn existing_incompatible_gpu_fixture(runtime: &std::path::Path) -> PathBuf {
    let fixture_root = runtime.join("r8/native");
    let mut candidates = fs::read_dir(&fixture_root)
        .expect("preserved L3b R8 fixture directory exists")
        .filter_map(Result::ok)
        .filter(|entry| {
            entry.file_type().is_ok_and(|kind| kind.is_dir())
                && entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("r8-incompatible-")
        })
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    candidates.sort();
    candidates
        .into_iter()
        .find(|candidate| {
            sha256(&candidate.join("nemo_speech_asr_c.dll"))
                == "9148ba2ed0eaeee385d09eba37e1a297d5e2f0649a02ffd4e325fea562c29ea6"
        })
        .expect("a preserved read-only L3b fixture has the intentional NeMo ABI mismatch")
}

fn assert_cpu_incompatible_native_refusal(
    execution_root: &std::path::Path,
    model_path: &std::path::Path,
) {
    let cpu_runtime = required_path("PARAKEET_L2_CPU_RUNTIME_ROOT");
    let cpu_worker = cpu_worker_path();
    let incompatible_native = existing_cpu_incompatible_fixture(&cpu_runtime);
    let record = whisper_core::model_registry::model(ModelId::ParakeetTdt06BV3);
    let expected_key = whisper_core::model_registry::configuration_key(
        record,
        whisper_core::model_catalog::Backend::Cpu,
    );
    let bad_paths = ParakeetWorkerPaths {
        executable: cpu_worker.clone(),
        native_dir: incompatible_native.clone(),
        model: model_path.to_path_buf(),
        runtime_root: cpu_runtime.clone(),
    };
    let mismatch = bad_paths
        .verify_qualification(
            record.artifact.sha256,
            &expected_key.runtime_build,
            &expected_key.worker_build,
        )
        .expect_err("the read-only incompatible CPU fixture must not qualify");
    assert!(
        mismatch.starts_with("QualificationMismatch: runtime="),
        "{mismatch}"
    );

    let source = required_path("PARAKEET_L2_EN_WAV");
    let destination = execution_root.join(format!("archive/r8-cpu-loader-{}", std::process::id()));
    let dummy_turbo = required_path("PARAKEET_L2_RUNTIME_ROOT").join("worker/not-used-turbo.exe");
    let io = AsyncImportIo::start_with_model_catalog_and_parakeet(
        WorkerPaths {
            cpu: dummy_turbo.clone(),
            gpu: dummy_turbo,
        },
        model_path.parent().unwrap().to_path_buf(),
        required_path("PARAKEET_L2_RUNTIME_ROOT")
            .join("historical/ggml-large-v3-turbo.bin")
            .to_string_lossy()
            .into_owned(),
        Some(bad_paths),
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
                required_path("PARAKEET_L2_RUNTIME_ROOT")
                    .join("historical/ggml-large-v3-turbo.bin")
                    .to_string_lossy()
                    .into_owned(),
            ),
        ],
    );
    let mut app = ImportApplication::new(io).with_model_guard(guard);
    let job_id = JobId(0x2a8);
    let generation = Generation::first();
    app.dispatch(AppCommand::StartImport {
        request: ImportRequest {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
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
        },
    })
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(240);
    let refusal = loop {
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        if view.active_job == Some((job_id, whisper_core::JobState::Recoverable)) {
            break view.message.unwrap_or_default();
        }
        assert!(
            Instant::now() < deadline,
            "CPU DLL refusal missing: {view:?}"
        );
        thread::sleep(Duration::from_millis(10));
    };
    assert!(
        refusal.contains("QualificationMismatch: runtime="),
        "{refusal}"
    );
    let passage = passage_dir(&destination, job_id, generation);
    assert!(!passage.join("pending.json").exists());
    assert!(!passage.join("transcript.txt").exists());
    assert!(!passage.join("transcript.srt").exists());

    let output = Command::new(cpu_worker)
        .arg("--model")
        .arg(model_path)
        .arg("--model-sha256")
        .arg(record.artifact.sha256)
        .arg("--native-dir")
        .arg(&incompatible_native)
        .env("PARAKEET_L2_RUNTIME_ROOT", &cpu_runtime)
        .stdin(Stdio::null())
        .output()
        .expect("frozen CPU worker loader probe starts with the preserved L2b fixture");
    assert!(
        !output.status.success(),
        "incompatible CPU ABI must fail before Ready"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("parakeet-worker fatal:"), "{stderr}");
    assert!(
        stderr.to_ascii_lowercase().contains("procedure")
            || stderr.to_ascii_lowercase().contains("symbol")
            || stderr.contains("GetProcAddress"),
        "CPU loader failure phase is not attributed: {stderr}"
    );
    assert!(String::from_utf8_lossy(&output.stdout).is_empty());
}

fn existing_cpu_incompatible_fixture(cpu_runtime: &std::path::Path) -> PathBuf {
    let native_root = cpu_runtime.join("r8/native");
    let baseline_native = cpu_native_dir();
    let expected = [
        "nemo_speech_asr_c.dll",
        "nemo_speech_asr.dll",
        "ggml.dll",
        "ggml-base.dll",
        "ggml-cpu.dll",
    ];
    for directory in fs::read_dir(&native_root)
        .expect("preserved CPU R8 fixture directory is readable")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_dir()
                && path
                    .file_name()
                    .is_some_and(|name| name.to_string_lossy().starts_with("r8-incompatible-"))
        })
    {
        let files = fs::read_dir(&directory)
            .expect("CPU R8 fixture directory is readable")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        if files.len() != expected.len()
            || expected
                .iter()
                .any(|name| !files.iter().any(|file| file == name))
        {
            continue;
        }
        let copied = sha256(&directory.join("nemo_speech_asr_c.dll"));
        if copied == sha256(&baseline_native.join("nemo_speech_asr_c.dll"))
            || copied != sha256(&directory.join("ggml.dll"))
            || expected
                .iter()
                .copied()
                .filter(|name| *name != "nemo_speech_asr_c.dll")
                .any(|name| sha256(&directory.join(name)) != sha256(&baseline_native.join(name)))
        {
            continue;
        }
        return directory;
    }
    panic!("no preserved CPU R8 fixture has the expected single-DLL substitution");
}

#[test]
#[ignore = "kills a qualified native worker during an active inference request"]
fn killed_native_worker_is_reported_as_exit_without_result() {
    let execution_root = r8_exec_root();
    let source = required_path("PARAKEET_L2_EN_WAV");
    assert_worker_death(
        &configured_r6_cpu_paths(),
        &source,
        "00000000000000000000000000002a5",
    );
    let candidate = qualified_gpu_candidate();
    let expected_gpu = whisper_adapters::parakeet_import::expected_gpu_for_candidate(&candidate)
        .expect("qualified R8 GPU identity matches the host runtime");
    assert_worker_death_with_gpu(
        &configured_gpu_paths(),
        &source,
        "00000000000000000000000000002a6",
        expected_gpu,
    );
    assert!(execution_root.join("archive").exists());
}

fn assert_worker_death(paths: &ParakeetWorkerPaths, source: &std::path::Path, job_id: &str) {
    let mut process = ParakeetProcess::spawn(paths).unwrap();
    let identity = wait_for_ready(&mut process);
    process.accept_ready(identity).unwrap();
    let bytes = fs::read(&source).unwrap();
    let source_sha = format!("{:x}", Sha256::digest(bytes));
    process
        .transcribe(job_id.into(), 1, 1, &source, source_sha)
        .unwrap();
    // The frozen R6 CPU worker does not emit NativeCallStarted; retain the
    // historical R8 behavior and terminate immediately after Transcribe.
    let status = process.terminate().expect("kill and wait for child exit");
    assert!(!status.success());
    let error = poll_until_error(&mut process, Duration::from_secs(10));
    assert!(error.starts_with("WorkerExited:"), "{error}");
    assert!(!error.contains("Result"));
}

fn assert_worker_death_with_gpu(
    paths: &ParakeetWorkerPaths,
    source: &std::path::Path,
    job_id: &str,
    expected_gpu: whisper_core::parakeet_ipc::ExpectedGpu,
) {
    let mut process = ParakeetProcess::spawn(paths).unwrap();
    let identity = wait_for_ready(&mut process);
    process.accept_ready(identity).unwrap();
    let bytes = fs::read(source).unwrap();
    let source_sha = format!("{:x}", Sha256::digest(bytes));
    process
        .transcribe_with_gpu(job_id.into(), 1, 1, source, source_sha, Some(expected_gpu))
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        match process.poll().expect("GPU worker event is readable") {
            Some(whisper_core::parakeet_ipc::Event::NativeCallStarted {
                job_id: actual,
                generation,
                attempt,
                ..
            }) => {
                assert_eq!(actual, job_id);
                assert_eq!(generation, 1);
                assert_eq!(attempt, 1);
                break;
            }
            Some(event) => panic!("unexpected GPU event before native call: {event:?}"),
            None => {
                assert!(Instant::now() < deadline, "GPU native call did not start");
                thread::sleep(Duration::from_millis(10));
            }
        }
    }
    let status = process.terminate().expect("kill and wait for child exit");
    assert!(!status.success());
    let error = poll_until_error(&mut process, Duration::from_secs(10));
    assert!(error.starts_with("WorkerExited:"), "{error}");
    assert!(!error.contains("Result"));
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
    let model = required_path("PARAKEET_L2_MODEL");
    let cpu_native_fixture = execution_root
        .join("native")
        .join(format!("r8-cpu-transport-{}", std::process::id()));
    fs::create_dir_all(&cpu_native_fixture).unwrap();
    let cpu_native_source = cpu_native_dir();
    for name in [
        "nemo_speech_asr_c.dll",
        "nemo_speech_asr.dll",
        "ggml.dll",
        "ggml-base.dll",
        "ggml-cpu.dll",
    ] {
        fs::copy(cpu_native_source.join(name), cpu_native_fixture.join(name)).unwrap();
    }
    let cpu_paths = ParakeetWorkerPaths {
        executable: fake_worker.clone(),
        native_dir: cpu_native_fixture,
        model: model.clone(),
        runtime_root: required_path("PARAKEET_L2_RUNTIME_ROOT"),
    };
    assert_incomplete_result_frame(&cpu_paths);

    let mut gpu_paths = configured_gpu_paths();
    gpu_paths.executable = fake_worker;
    assert_incomplete_result_frame(&gpu_paths);
}

fn assert_incomplete_result_frame(paths: &ParakeetWorkerPaths) {
    let mut process = ParakeetProcess::spawn(paths).unwrap();
    let error = poll_until_error(&mut process, Duration::from_secs(10));
    assert!(error.starts_with("IpcDecode:"), "{error}");
    assert!(error.contains("request_id"), "{error}");
}

#[test]
#[ignore = "routes a worker death from the qualified Test process through ImportApplication"]
fn product_route_worker_death_preserves_pending_source_and_rejects_completion() {
    run_product_transport_failure(
        "worker-death-cpu",
        "worker-death",
        "WorkerExited",
        JobId(0x2a6),
        ComputeChoice::Cpu,
    );
    run_product_transport_failure(
        "worker-death-gpu",
        "worker-death",
        "WorkerExited",
        JobId(0x2a8),
        ComputeChoice::Gpu,
    );
}

#[test]
#[ignore = "routes an incomplete Result transport failure through ImportApplication"]
fn product_route_incomplete_result_preserves_pending_source_and_rejects_completion() {
    run_product_transport_failure(
        "incomplete-result-cpu",
        "incomplete-result",
        "IpcDecode",
        JobId(0x2a7),
        ComputeChoice::Cpu,
    );
    run_product_transport_failure(
        "incomplete-result-gpu",
        "incomplete-result",
        "IpcDecode",
        JobId(0x2a9),
        ComputeChoice::Gpu,
    );
}

fn run_product_transport_failure(
    case: &str,
    signal: &str,
    expected_error: &str,
    job_id: JobId,
    compute: ComputeChoice,
) {
    let runtime = required_path("PARAKEET_L2_RUNTIME_ROOT");
    let execution_root = r8_exec_root();
    let source = required_path("PARAKEET_L2_EN_WAV");
    let source_sha_before = format!("{:x}", Sha256::digest(fs::read(&source).unwrap()));
    let model_path = required_path("PARAKEET_L2_MODEL");
    let destination = execution_root.join(format!("archive/product-{case}-{}", std::process::id()));
    let control = destination.join(".l2a2-acceptance");
    fs::create_dir_all(&control).unwrap();
    // Arm before launch; the adapter consumes the signal only after accepting
    // the exact worker identity and sending Transcribe.
    fs::write(control.join(format!("{signal}.enabled")), b"enabled").unwrap();
    let model_record = whisper_core::model_registry::model(ModelId::ParakeetTdt06BV3);
    let worker = gpu_worker_path();
    let native = gpu_native_dir();
    let dummy_turbo = runtime.join("worker/not-used-turbo.exe");
    let io = AsyncImportIo::start_with_model_catalog_and_parakeet_routes(
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
            executable: cpu_worker_path(),
            native_dir: cpu_native_dir(),
            model: model_path.clone(),
            runtime_root: runtime.clone(),
        }),
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
    if compute == ComputeChoice::Gpu {
        app.set_gpu_candidate(job_id, generation, Some(qualified_gpu_candidate()))
            .unwrap();
    }
    let request = ImportRequest {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
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
            compute,
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
        executable: cpu_worker_path(),
        native_dir: cpu_native_dir(),
        model: required_path("PARAKEET_L2_MODEL"),
        runtime_root: required_path("PARAKEET_L2_RUNTIME_ROOT"),
    }
}

fn configured_r6_cpu_paths() -> ParakeetWorkerPaths {
    ParakeetWorkerPaths {
        executable: cpu_worker_path(),
        native_dir: cpu_native_dir(),
        model: required_path("PARAKEET_L2_MODEL"),
        runtime_root: required_path("PARAKEET_L2_RUNTIME_ROOT"),
    }
}

fn configured_gpu_paths() -> ParakeetWorkerPaths {
    ParakeetWorkerPaths {
        executable: gpu_worker_path(),
        native_dir: gpu_native_dir(),
        model: required_path("PARAKEET_L2_MODEL"),
        runtime_root: required_path("PARAKEET_L2_RUNTIME_ROOT"),
    }
}

fn assert_registered_gpu_key(paths: &ParakeetWorkerPaths) {
    let actual = paths
        .gpu_qualification_key(whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS)
        .expect("the frozen GPU worker and DLL set produce a qualification key");
    #[cfg(feature = "personal-qualification")]
    let personal_candidate = std::env::var_os("PARAKEET_L6P_STAGE").is_some();
    #[cfg(not(feature = "personal-qualification"))]
    let personal_candidate = false;
    let registered = whisper_core::model_registry::configuration_key(
        whisper_core::model_registry::model(ModelId::ParakeetTdt06BV3),
        whisper_core::model_catalog::Backend::Gpu,
    );
    if personal_candidate {
        #[cfg(feature = "personal-qualification")]
        assert!(
            whisper_adapters::parakeet_import::ParakeetWorkerPaths::personal_candidate_keys_from_env()
                .contains(&actual),
            "staged GPU identity is the exact candidate admitted only to this harness"
        );
    } else {
        assert_eq!(
            actual,
            whisper_adapters::archive::ArchiveQualificationKey::parakeet_gpu_test(),
            "the qualified worker, DLLs, and archive key use the same frozen identity"
        );
    }
    assert_eq!(actual.model, "parakeet-tdt-0.6b-v3");
    assert_eq!(actual.artifact, registered.artifact);
    assert_eq!(actual.artifact_revision, registered.artifact_revision);
    assert_eq!(actual.runtime, registered.runtime);
    if !personal_candidate {
        assert_eq!(actual.runtime_build, registered.runtime_build);
    }
    assert_eq!(actual.worker, registered.worker);
    if !personal_candidate {
        assert_eq!(actual.worker_build, registered.worker_build);
    }
    assert_eq!(actual.worker_protocol, registered.worker_protocol);
    assert_eq!(actual.backend, "CUDA0");
    assert_eq!(actual.inference_profile, registered.inference_profile);
}

fn gpu_worker_path() -> PathBuf {
    #[cfg(feature = "personal-qualification")]
    if std::env::var_os("PARAKEET_L6P_STAGE").is_some() {
        if let Some(path) = std::env::var_os("PARAKEET_L6P_GPU_WORKER") {
            return PathBuf::from(path);
        }
    }
    required_path("PARAKEET_L2_WORKER")
}

fn gpu_native_dir() -> PathBuf {
    #[cfg(feature = "personal-qualification")]
    if std::env::var_os("PARAKEET_L6P_STAGE").is_some() {
        if let Some(path) = std::env::var_os("PARAKEET_L6P_GPU_NATIVE_DIR") {
            return PathBuf::from(path);
        }
    }
    required_path("PARAKEET_L2_NATIVE_DIR")
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

fn not_run_without_campaign_variables(variables: &[&str]) -> bool {
    let missing = variables
        .iter()
        .filter(|name| std::env::var_os(name).is_none())
        .copied()
        .collect::<Vec<_>>();
    if missing.is_empty() {
        return false;
    }
    eprintln!(
        "NOT RUN: missing campaign variable(s): {}",
        missing.join(", ")
    );
    true
}

fn cpu_worker_path() -> PathBuf {
    std::env::var_os("PARAKEET_L2_CPU_WORKER")
        .or_else(|| std::env::var_os("PARAKEET_L2_WORKER"))
        .map(PathBuf::from)
        .expect("PARAKEET_L2_CPU_WORKER or legacy PARAKEET_L2_WORKER is required")
}

fn cpu_native_dir() -> PathBuf {
    std::env::var_os("PARAKEET_L2_CPU_NATIVE_DIR")
        .or_else(|| std::env::var_os("PARAKEET_L2_NATIVE_DIR"))
        .map(PathBuf::from)
        .expect("PARAKEET_L2_CPU_NATIVE_DIR or legacy PARAKEET_L2_NATIVE_DIR is required")
}

fn r8_exec_root() -> PathBuf {
    let root = required_path("PARAKEET_R8_EXEC_ROOT")
        .canonicalize()
        .expect("R8 execution root exists");
    let l3b_runtime = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/parakeet-tests/l3/l3b")
        .canonicalize()
        .expect("canonical L3b runtime root exists");
    let configured_runtime = required_path("PARAKEET_L2_RUNTIME_ROOT")
        .canonicalize()
        .expect("configured runtime root exists");
    if let (Some(local_app_data), Ok(backend)) = (
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from),
        std::env::var("PARAKEET_L6P_BACKEND"),
    ) {
        if matches!(backend.as_str(), "CPU" | "CUDA0") {
            let staging_parent = local_app_data
                .join("Whisper/.staging")
                .canonicalize()
                .expect("canonical Personal staging parent exists");
            if configured_runtime.parent() == Some(staging_parent.as_path()) {
                let expected = configured_runtime
                    .join("qualification")
                    .join(backend)
                    .join("r8")
                    .canonicalize()
                    .expect("canonical Personal R8 execution root exists");
                assert_eq!(
                    root, expected,
                    "Personal R8 scenarios stay in the selected backend staging root"
                );
                return root;
            }
        }
    }
    let l3b_r8 = l3b_runtime.join("r8");
    if configured_runtime == l3b_runtime && root == l3b_r8 {
        return root;
    }
    let r8 = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/parakeet-tests/l2/l2b/r8")
        .canonicalize()
        .expect("canonical R8 execution root exists");
    let r9 = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/parakeet-tests/l2/l2b/r9")
        .canonicalize()
        .expect("canonical R9 execution root exists");
    let l3c_r8 = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/parakeet-tests/l3/l3c/r8")
        .canonicalize()
        .expect("canonical L3c R8 execution root exists");
    let l3d_r8 = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/parakeet-tests/l3/l3d/r8")
        .canonicalize()
        .expect("canonical L3d R8 execution root exists");
    assert!(
        root == r8 || root == r9 || root == l3c_r8 || root == l3d_r8,
        "R8 scenarios are confined to a canonical R8, R9, L3c, or L3d execution root"
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
