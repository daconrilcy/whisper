use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    process::Command,
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

#[test]
fn import_strategy_defaults_to_legacy_and_round_trips_explicit_versions() {
    let request = |job_id| ImportRequest {
        strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
        job_id: JobId(job_id),
        generation: Generation::first(),
        source_path: "sample.wav".into(),
        source_sha256: None,
        source_samples: None,
        model_path: "approved-model.bin".into(),
        model_sha256: [7; 32],
        destination: "archive".into(),
        config: JobConfig {
            language: LanguageChoice::Manual("fr".into()),
            compute: ComputeChoice::Cpu,
        },
    };

    let legacy = serde_json::to_value(request(99)).unwrap();
    let mut historical = legacy.clone();
    historical.as_object_mut().unwrap().remove("strategy");
    let decoded: ImportRequest = serde_json::from_value(historical).unwrap();
    assert_eq!(
        decoded.strategy,
        whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1
    );

    let mut unified = request(100);
    unified.strategy = whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1;
    let encoded = serde_json::to_vec(&unified).unwrap();
    let decoded: ImportRequest = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(decoded, unified);

    let mut unknown = serde_json::to_value(unified).unwrap();
    unknown["strategy"] = serde_json::Value::String("future_strategy".into());
    assert!(serde_json::from_value::<ImportRequest>(unknown).is_err());
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

fn not_run_without_l2_archive_inputs() -> bool {
    not_run_without_campaign_variables(&[
        "PARAKEET_L2_ARCHIVE_ROOT",
        "PARAKEET_L2_RUNTIME_ROOT",
        "PARAKEET_L2_MODEL",
        "PARAKEET_L2_EN_WAV",
    ])
}

fn archive_root() -> PathBuf {
    let root = required_path("PARAKEET_L2_ARCHIVE_ROOT");
    let canonical = root.canonicalize().expect("archive root exists");
    let runtime = required_path("PARAKEET_L2_RUNTIME_ROOT")
        .canonicalize()
        .expect("runtime root exists");
    let l3a = std::env::var_os("PARAKEET_L3A_EXEC_ROOT")
        .map(PathBuf::from)
        .and_then(|path| path.canonicalize().ok())
        .filter(|path| {
            let expected = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/parakeet-tests/l3/l3a")
                .canonicalize()
                .expect("canonical L3a runtime root exists");
            path == &expected && canonical.starts_with(path)
        });
    let l3b = std::env::var_os("PARAKEET_L3B_EXEC_ROOT")
        .map(PathBuf::from)
        .and_then(|path| path.canonicalize().ok())
        .filter(|path| {
            let expected = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/parakeet-tests/l3/l3b")
                .canonicalize()
                .expect("canonical L3b runtime root exists");
            path == &expected && canonical.starts_with(path)
        });
    let l3c = std::env::var_os("PARAKEET_L3C_EXEC_ROOT")
        .map(PathBuf::from)
        .and_then(|path| path.canonicalize().ok())
        .filter(|path| {
            let expected = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/parakeet-tests/l3/l3c")
                .canonicalize()
                .expect("canonical L3c runtime root exists");
            path == &expected && canonical.starts_with(path)
        });
    let l3d = std::env::var_os("PARAKEET_L3D_EXEC_ROOT")
        .map(PathBuf::from)
        .and_then(|path| path.canonicalize().ok())
        .filter(|path| {
            let expected = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/parakeet-tests/l3/l3d")
                .canonicalize()
                .expect("canonical L3d runtime root exists");
            path == &expected && canonical.starts_with(path)
        });
    let l6p = std::env::var_os("PARAKEET_L6P_EXEC_ROOT")
        .map(PathBuf::from)
        .and_then(|path| path.canonicalize().ok())
        .filter(|path| {
            let expected = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/parakeet-tests/l6-perso")
                .canonicalize()
                .expect("canonical L6-perso execution root exists");
            path == &expected && canonical.starts_with(path)
        });
    let r10 = std::env::var_os("PARAKEET_R10_EXEC_ROOT")
        .map(PathBuf::from)
        .and_then(|path| path.canonicalize().ok())
        .filter(|path| {
            let expected = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/parakeet-tests/r10")
                .canonicalize()
                .expect("canonical R10 execution root exists");
            path == &expected && canonical.starts_with(path)
        });
    let r11 = std::env::var_os("PARAKEET_R11_EXEC_ROOT")
        .map(PathBuf::from)
        .and_then(|path| path.canonicalize().ok())
        .filter(|path| {
            let expected = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/parakeet-tests/r11")
                .canonicalize()
                .expect("canonical R11 execution root exists");
            path == &expected && canonical.starts_with(path)
        });
    let r8 = std::env::var_os("PARAKEET_R8_EXEC_ROOT")
        .map(PathBuf::from)
        .and_then(|path| path.canonicalize().ok())
        .filter(|path| {
            let expected = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/parakeet-tests/l2/l2b/r8")
                .canonicalize()
                .expect("canonical R8 runtime root exists");
            path == &expected && canonical.starts_with(path)
        });
    let r9 = std::env::var_os("PARAKEET_R9_EXEC_ROOT")
        .map(PathBuf::from)
        .and_then(|path| path.canonicalize().ok())
        .filter(|path| {
            let expected = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../target/parakeet-tests/l2/l2b/r9")
                .canonicalize()
                .expect("canonical R9 runtime root exists");
            path == &expected && canonical.starts_with(path)
        });
    let r7 = std::env::var_os("PARAKEET_R7_EXEC_ROOT")
        .map(PathBuf::from)
        .and_then(|path| path.canonicalize().ok())
        .filter(|path| {
            path.ends_with("target/parakeet-tests/l2/l2b/r7") && canonical.starts_with(path)
        });
    let allowed = l6p
        .or(r11)
        .or(r10)
        .or(l3d)
        .or(l3c)
        .or(l3b)
        .or(l3a)
        .or(r9)
        .or(r8)
        .or(r7)
        .unwrap_or_else(|| {
            runtime
                .join("r6")
                .canonicalize()
                .expect("R6 runtime root exists")
        });
    assert!(
        canonical.starts_with(&allowed),
        "archive root must be inside R6, canonical R7, R8, R9, R10, R11, L3a, L3b, L3c, L3d, or L6-perso"
    );
    root
}

fn qualified_key() -> ArchiveQualificationKey {
    ArchiveQualificationKey::parakeet_cpu_test()
}

fn qualified_vad_key() -> ArchiveQualificationKey {
    let mut key = qualified_key();
    key.inference_profile = whisper_core::parakeet_windows::PROFILE_IMPORT_VAD.into();
    key.worker_build = "e7672a520db87926b3da9b8e5b585a1e7e57ce53ff8b1d0fe95251046aa01f4d".into();
    key
}

fn qualified_l2b_single_pass_key() -> ArchiveQualificationKey {
    qualified_key()
}

fn qualified_r4_single_pass_key() -> ArchiveQualificationKey {
    let mut key = qualified_l2b_single_pass_key();
    key.worker_build = "aa8dfdf317649eabbfe670c5109df46c70d3f1908064bee61d26fe7aa059bf31".into();
    key
}

fn retired_l2a_single_pass_key() -> ArchiveQualificationKey {
    let mut key = qualified_l2b_single_pass_key();
    key.worker_build = "545a4830755da613a1255be97fbc6a6a513ef837d669d8a9870d3bbee9dfd587".into();
    key
}

fn personal_key(backend: whisper_core::model_catalog::Backend) -> ArchiveQualificationKey {
    let record = model(ModelId::ParakeetTdt06BV3);
    let configuration = whisper_core::model_registry::personal_configuration_key(record, backend);
    let mut key = ArchiveQualificationKey::parakeet_cpu_test();
    key.artifact_revision = configuration.artifact_revision;
    key.runtime_build = configuration.runtime_build;
    key.worker_build = configuration.worker_build;
    key.backend = match backend {
        whisper_core::model_catalog::Backend::Cpu => "CPU",
        whisper_core::model_catalog::Backend::Gpu => "CUDA0",
    }
    .into();
    key.inference_profile = configuration.inference_profile;
    if backend == whisper_core::model_catalog::Backend::Gpu {
        key.gpu_pci_bus_id =
            Some(whisper_core::model_registry::PARAKEET_PERSONAL_GPU_PCI_BUS_ID.into());
        key.gpu_uuid = Some(whisper_core::model_registry::PARAKEET_PERSONAL_GPU_UUID.into());
        key.gpu_pnp_instance_id =
            Some(whisper_core::model_registry::PARAKEET_PERSONAL_GPU_PNP_INSTANCE_ID.into());
    }
    key
}

#[test]
fn parakeet_model_identity_prepares_persists_and_publishes_txt_srt() {
    if not_run_without_l2_archive_inputs() {
        return;
    }
    let root = archive_root().join("parakeet-archive-pass");
    let _ = fs::remove_dir_all(&root);
    let source = required_path("PARAKEET_L2_EN_WAV");
    let model_path = required_path("PARAKEET_L2_MODEL");
    let record = model(ModelId::ParakeetTdt06BV3);
    let mut request = ImportRequest {
        strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
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
#[ignore = "copies and mutates the real qualified GGUF inside the isolated R9 runtime"]
fn modified_parakeet_model_is_refused_before_pending_after_cache_revalidation() {
    use std::io::{Read, Seek, SeekFrom};
    use whisper_adapters::model_catalog::cached_model_sha256;

    let fixture_root = archive_root().join("r9-mutated-model");
    fs::create_dir_all(&fixture_root).unwrap();
    let original_model = required_path("PARAKEET_L2_MODEL");
    let modified_model = fixture_root.join("model.gguf");
    fs::copy(&original_model, &modified_model).expect("copy model into the R9 execution root");
    let record = model(ModelId::ParakeetTdt06BV3);
    let (original_size, original_hash) = cached_model_sha256(&modified_model).unwrap();
    assert_eq!(original_size, record.artifact.size_bytes);
    let original_hex = original_hash
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(original_hex, record.artifact.sha256);

    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&modified_model)
        .unwrap();
    let mut first_byte = [0_u8; 1];
    file.read_exact(&mut first_byte).unwrap();
    file.seek(SeekFrom::Start(0)).unwrap();
    file.write_all(&[first_byte[0] ^ 1]).unwrap();
    file.set_times(
        std::fs::FileTimes::new()
            .set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(2)),
    )
    .unwrap();
    file.sync_all().unwrap();
    drop(file);

    let (_, changed_hash) = cached_model_sha256(&modified_model).unwrap();
    assert_ne!(
        changed_hash, original_hash,
        "changed bytes must trigger a new SHA read"
    );

    let source = required_path("PARAKEET_L2_EN_WAV");
    let output = fixture_root.join("refused-import");
    let request = ImportRequest {
        strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
        job_id: JobId(0x9a901),
        generation: Generation::first(),
        source_path: source.to_string_lossy().into_owned(),
        source_sha256: None,
        source_samples: None,
        model_path: modified_model.to_string_lossy().into_owned(),
        model_sha256: decode_sha256(record.artifact.sha256),
        destination: output.to_string_lossy().into_owned(),
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        },
    };
    let error = ArchiveStore::new(&output)
        .prepare_with_qualification(request, None, Some(qualified_key()))
        .unwrap_err();
    assert!(
        error.contains("ModelHashMismatch"),
        "unexpected refusal: {error}"
    );
    assert_eq!(original_hex, record.artifact.sha256);
    assert!(
        !output.exists(),
        "changed model must be refused before destination/pending creation"
    );
}

#[test]
fn parakeet_archive_refuses_model_sha_divergence_before_pending_creation() {
    if not_run_without_l2_archive_inputs() {
        return;
    }
    let root = archive_root().join("parakeet-archive-reject");
    let _ = fs::remove_dir_all(&root);
    let source = required_path("PARAKEET_L2_EN_WAV");
    let model_path = required_path("PARAKEET_L2_MODEL");
    let mut request = ImportRequest {
        strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
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
    if not_run_without_l2_archive_inputs() {
        return;
    }
    let rejected_root = archive_root().join("parakeet-key-reject");
    let _ = fs::remove_dir_all(&rejected_root);
    let source = required_path("PARAKEET_L2_EN_WAV");
    let model_path = required_path("PARAKEET_L2_MODEL");
    let record = model(ModelId::ParakeetTdt06BV3);
    let make_request = |destination: &PathBuf, job| ImportRequest {
        strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
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
    assert!(!wrong_key.is_parakeet_import_test());
    assert!(
        ArchiveStore::new(&rejected_root)
            .prepare_with_qualification(make_request(&rejected_root, 0x1a9), None, Some(wrong_key),)
            .unwrap_err()
            .contains("exact Test worker/runtime key")
    );
    assert!(!rejected_root.exists());

    let exact_vad_key = qualified_vad_key();
    assert!(exact_vad_key.is_parakeet_vad_test());
    let mut wrong_vad_key = exact_vad_key.clone();
    wrong_vad_key.worker_build = "0".repeat(64);
    assert!(!wrong_vad_key.is_parakeet_vad_test());

    let exact_l2b_single_pass_key = qualified_l2b_single_pass_key();
    assert!(exact_l2b_single_pass_key.is_parakeet_import_test());
    assert!(!exact_l2b_single_pass_key.is_parakeet_vad_test());
    let mut wrong_l2b_runtime = exact_l2b_single_pass_key.clone();
    wrong_l2b_runtime.runtime_build = "0".repeat(64);
    assert!(!wrong_l2b_runtime.is_parakeet_import_test());
    let r4_key = qualified_r4_single_pass_key();
    assert!(!r4_key.is_parakeet_import_test());
    let l2a_key = retired_l2a_single_pass_key();
    assert!(!l2a_key.is_parakeet_import_test());

    let l2b_root = archive_root().join("parakeet-l2b-single-pass-key");
    let _ = fs::remove_dir_all(&l2b_root);
    let source_sha = whisper_adapters::archive::sha256_file(&source).unwrap();
    assert!(
        ArchiveStore::new(&l2b_root)
            .prepare_with_qualification(make_request(&l2b_root, 0x1ab), None, Some(r4_key),)
            .unwrap_err()
            .contains("exact Test worker/runtime key"),
        "the R4 short key is refused with an explicit qualification error"
    );
    assert!(
        !l2b_root.exists(),
        "refused R4 key must not create a destination or pending record"
    );
    assert_eq!(
        whisper_adapters::archive::sha256_file(&source).unwrap(),
        source_sha
    );

    let l2a_root = archive_root().join("parakeet-l2a-new-import-reject");
    let _ = fs::remove_dir_all(&l2a_root);
    assert!(
        ArchiveStore::new(&l2a_root)
            .prepare_with_qualification(make_request(&l2a_root, 0x1ad), None, Some(l2a_key),)
            .unwrap_err()
            .contains("exact Test worker/runtime key"),
        "new import with the retired L2a key is rejected explicitly"
    );
    assert!(
        !l2a_root.exists(),
        "rejected L2a identity must not create a destination or pending record"
    );

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
    pending["qualification"]["runtime_build"] =
        serde_json::json!("d5a2cc8ef441dd5934972adae00f997241bea17a7bc5977a8c7f0122aaa958ff");
    pending["qualification"]["worker_build"] =
        serde_json::json!("aa8dfdf317649eabbfe670c5109df46c70d3f1908064bee61d26fe7aa059bf31");
    let pending_bytes = serde_json::to_vec(&pending).unwrap();
    fs::write(&pending_path, &pending_bytes).unwrap();

    let scan = ArchiveStore::scan(&root).unwrap();
    assert!(scan.iter().any(|entry| {
        entry.job_id == prepared.job_id
            && entry
                .diagnostic
                .as_deref()
                .is_some_and(|message| message.contains("pending identity/schema"))
    }));
    assert_eq!(fs::read(&pending_path).unwrap(), pending_bytes);
    assert_eq!(
        whisper_adapters::archive::sha256_file(&source).unwrap(),
        source_sha
    );
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
    assert!(
        source.exists(),
        "refused resume must preserve its source file"
    );
    assert!(
        pending_path.exists(),
        "refused resume must preserve the old pending record"
    );
    assert_eq!(fs::read(&pending_path).unwrap(), pending_bytes);
    assert_eq!(
        whisper_adapters::archive::sha256_file(&source).unwrap(),
        source_sha
    );

    let gpu_root = archive_root().join("parakeet-gpu-key-scan");
    let _ = fs::remove_dir_all(&gpu_root);
    let mut gpu_request = make_request(&gpu_root, 0x1ae);
    gpu_request.config.compute = ComputeChoice::Gpu;
    let gpu_prepared = ArchiveStore::new(&gpu_root)
        .prepare_with_qualification(
            gpu_request,
            None,
            Some(ArchiveQualificationKey::parakeet_gpu_test()),
        )
        .unwrap();
    let gpu_scan = ArchiveStore::scan(&gpu_root).unwrap();
    assert!(
        gpu_scan.iter().any(|entry| {
            entry.job_id == gpu_prepared.job_id && entry.diagnostic.is_none() && !entry.complete
        }),
        "the exact qualified GPU key must remain valid when scanning its pending archive"
    );
}

#[test]
fn registered_personal_pending_cpu_and_gpu_scan_and_resume_without_qualification_feature() {
    if not_run_without_l2_archive_inputs() {
        return;
    }
    let source = required_path("PARAKEET_L2_EN_WAV");
    let model_path = required_path("PARAKEET_L2_MODEL");
    let record = model(ModelId::ParakeetTdt06BV3);

    for (backend, job) in [
        (whisper_core::model_catalog::Backend::Cpu, 0x1c1),
        (whisper_core::model_catalog::Backend::Gpu, 0x1c2),
    ] {
        let root = archive_root().join(format!("personal-registered-{backend:?}"));
        let _ = fs::remove_dir_all(&root);
        let mut request = ImportRequest {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
            job_id: JobId(job),
            generation: Generation::first(),
            source_path: source.to_string_lossy().into_owned(),
            source_sha256: None,
            source_samples: None,
            model_path: model_path.to_string_lossy().into_owned(),
            model_sha256: decode_sha256(record.artifact.sha256),
            destination: root.to_string_lossy().into_owned(),
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: match backend {
                    whisper_core::model_catalog::Backend::Cpu => ComputeChoice::Cpu,
                    whisper_core::model_catalog::Backend::Gpu => ComputeChoice::Gpu,
                },
            },
        };
        let key = personal_key(backend);
        assert!(key.is_registered_personal_import());
        let prepared = ArchiveStore::new(&root)
            .prepare_with_qualification(request.clone(), None, Some(key.clone()))
            .unwrap();
        let entries = ArchiveStore::scan(&root).unwrap();
        assert!(entries.iter().any(|entry| {
            entry.job_id == prepared.job_id && !entry.complete && entry.diagnostic.is_none()
        }));

        request.generation = prepared.generation.next().unwrap();
        let (resumed, _segments) = ArchiveStore::new(&root)
            .resume_from_with_qualification(
                &prepared,
                request,
                Some(&prepared.model_path),
                Some(key.clone()),
            )
            .unwrap();
        assert_eq!(resumed.generation, prepared.generation.next().unwrap());

        let pending_path = root
            .join("transcriptions")
            .join(format!("{:032x}", resumed.job_id.0))
            .join(resumed.generation.get().to_string())
            .join("pending.json");
        let pending_bytes = fs::read(&pending_path).unwrap();
        let mut pending: serde_json::Value = serde_json::from_slice(&pending_bytes).unwrap();
        let mut divergent = key.clone();
        divergent.worker_build = "0".repeat(64);
        pending["qualification"] = serde_json::to_value(&divergent).unwrap();
        let divergent_bytes = serde_json::to_vec(&pending).unwrap();
        fs::write(&pending_path, &divergent_bytes).unwrap();

        let entries = ArchiveStore::scan(&root).unwrap();
        assert!(entries.iter().any(|entry| {
            entry.job_id == resumed.job_id
                && entry
                    .diagnostic
                    .as_deref()
                    .is_some_and(|message| message.contains("pending identity/schema"))
        }));
        let mut next = resumed.clone();
        next.generation = resumed.generation.next().unwrap();
        let error = ArchiveStore::new(&root)
            .resume_from_with_qualification(
                &resumed,
                next,
                Some(&resumed.model_path),
                Some(divergent),
            )
            .unwrap_err();
        assert!(error.contains("pending identity/schema"), "{error}");
        assert_eq!(fs::read(&pending_path).unwrap(), divergent_bytes);
        assert!(source.is_file());
    }
}

#[test]
fn r5_2_vad_recipe_is_scoped_and_desktop_test_import_is_rejected_before_pending() {
    if not_run_without_campaign_variables(&[
        "PARAKEET_L2_ARCHIVE_ROOT",
        "PARAKEET_L2_RUNTIME_ROOT",
        "PARAKEET_L2_MODEL",
        "PARAKEET_R7_EXEC_ROOT",
    ]) {
        return;
    }
    if std::env::var_os("PARAKEET_R7_VAD_CHILD").is_none() {
        let r7 = required_path("PARAKEET_R7_EXEC_ROOT")
            .canonicalize()
            .expect("R7 root exists");
        let scenario = r7.join("recipe-scope");
        if scenario.exists() {
            assert_eq!(
                scenario.canonicalize().unwrap(),
                scenario,
                "only the exact R7 recipe fixture may be replaced"
            );
            fs::remove_dir_all(&scenario).expect("remove prior R7 recipe fixture");
        }
        let injected_archive = scenario.join("r5-2/archive");
        fs::create_dir_all(&injected_archive).expect("create isolated R7 recipe scope");
        let output = Command::new(std::env::current_exe().expect("test executable path"))
            .args([
                "--exact",
                "r5_2_vad_recipe_is_scoped_and_desktop_test_import_is_rejected_before_pending",
                "--nocapture",
            ])
            .env("PARAKEET_R7_VAD_CHILD", "1")
            .env("PARAKEET_L2_RUNTIME_ROOT", &scenario)
            .env("PARAKEET_L2_ARCHIVE_ROOT", &injected_archive)
            .output()
            .expect("start isolated R4 test child");
        assert!(
            output.status.success(),
            "R4 child failed: stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }

    let r7 = required_path("PARAKEET_R7_EXEC_ROOT")
        .canonicalize()
        .expect("R7 root exists");
    let runtime = required_path("PARAKEET_L2_RUNTIME_ROOT")
        .canonicalize()
        .expect("injected child runtime exists");
    assert!(runtime.starts_with(&r7), "injected runtime escaped R7");
    let archive = required_path("PARAKEET_L2_ARCHIVE_ROOT")
        .canonicalize()
        .expect("injected archive exists");
    assert!(archive.starts_with(runtime.join("r5-2")));

    let source = required_path("PARAKEET_L2_EN_LONG_WAV");
    let model_path = required_path("PARAKEET_L2_MODEL");
    let record = model(ModelId::ParakeetTdt06BV3);
    let make_request = |destination: &std::path::Path, job| ImportRequest {
        strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
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

    let recipe_destination = archive.join("r5-2-vad-recipe-accepted");
    let _ = fs::remove_dir_all(&recipe_destination);
    ArchiveStore::new(&recipe_destination)
        .prepare_with_qualification(
            make_request(&recipe_destination, 0x1ae),
            None,
            Some(qualified_vad_key()),
        )
        .expect("the exact VAD key remains usable for the scoped recipe");

    let desktop_destination = runtime.join("r5-2").join("ordinary-desktop-test-import");
    let error = ArchiveStore::new(&desktop_destination)
        .prepare_with_qualification(
            make_request(&desktop_destination, 0x1af),
            None,
            Some(qualified_vad_key()),
        )
        .unwrap_err();
    assert!(
        error.contains("réservé aux recettes R5-2"),
        "ordinary desktop Test import must be refused: {error}"
    );
    assert!(
        !desktop_destination.exists(),
        "refused ordinary desktop import must not create a destination or pending record"
    );
}

#[test]
fn retired_l2a_pending_remains_visible_but_cannot_be_resumed() {
    if not_run_without_l2_archive_inputs() {
        return;
    }
    let root = archive_root().join("parakeet-retired-l2a-pending");
    let _ = fs::remove_dir_all(&root);
    let source = required_path("PARAKEET_L2_EN_WAV");
    let model_path = required_path("PARAKEET_L2_MODEL");
    let record = model(ModelId::ParakeetTdt06BV3);
    let mut request = ImportRequest {
        strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
        job_id: JobId(0x1ac),
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
    let prepared = ArchiveStore::new(&root)
        .prepare_with_qualification(request.clone(), None, Some(qualified_key()))
        .unwrap();
    request = prepared.clone();
    let pending_path = root
        .join("transcriptions")
        .join(format!("{:032x}", prepared.job_id.0))
        .join(prepared.generation.get().to_string())
        .join("pending.json");
    let mut pending: serde_json::Value =
        serde_json::from_slice(&fs::read(&pending_path).unwrap()).unwrap();
    let mut retired_key = qualified_key();
    retired_key.worker_build =
        "545a4830755da613a1255be97fbc6a6a513ef837d669d8a9870d3bbee9dfd587".into();
    pending["qualification"] = serde_json::to_value(&retired_key).unwrap();
    let bytes = serde_json::to_vec(&pending).unwrap();
    fs::write(&pending_path, &bytes).unwrap();

    let entries = ArchiveStore::scan(&root).unwrap();
    assert!(entries.iter().any(|entry| {
        entry.job_id == prepared.job_id && !entry.complete && entry.diagnostic.is_none()
    }));
    let mut next = request.clone();
    next.generation = request.generation.next().unwrap();
    let error = ArchiveStore::new(&root)
        .resume_from_with_qualification(
            &request,
            next,
            Some(&request.model_path),
            Some(retired_key),
        )
        .unwrap_err();
    assert!(
        error.contains("ancienne clé Parakeet L2a est retirée"),
        "{error}"
    );
    assert!(error.contains("pending est conservé"), "{error}");
    assert_eq!(fs::read(&pending_path).unwrap(), bytes);
    assert!(source.is_file());
}

#[test]
fn retired_r5_2_pending_is_rejected_with_retirement_cause_and_preserved() {
    if not_run_without_l2_archive_inputs() {
        return;
    }
    let root = archive_root().join("parakeet-retired-r5-2-pending");
    let _ = fs::remove_dir_all(&root);
    let source = required_path("PARAKEET_L2_EN_WAV");
    let model_path = required_path("PARAKEET_L2_MODEL");
    let record = model(ModelId::ParakeetTdt06BV3);
    let request = ImportRequest {
        strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
        job_id: JobId(0x1b0),
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
    let mut retired_key = qualified_key();
    retired_key.worker_build =
        "e7672a520db87926b3da9b8e5b585a1e7e57ce53ff8b1d0fe95251046aa01f4d".into();
    pending["qualification"] = serde_json::to_value(&retired_key).unwrap();
    let bytes = serde_json::to_vec(&pending).unwrap();
    fs::write(&pending_path, &bytes).unwrap();

    let mut next = prepared.clone();
    next.generation = prepared.generation.next().unwrap();
    let error = ArchiveStore::new(&root)
        .resume_from_with_qualification(
            &prepared,
            next,
            Some(&prepared.model_path),
            Some(retired_key),
        )
        .unwrap_err();
    assert!(
        error.contains("ancienne clé R5-2 Parakeet est retirée"),
        "{error}"
    );
    assert!(error.contains("pending est conservé"), "{error}");
    assert_eq!(fs::read(&pending_path).unwrap(), bytes);
}

#[test]
fn gpu_pending_is_bound_to_the_qualified_physical_identity() {
    if not_run_without_l2_archive_inputs() {
        return;
    }
    let root = archive_root().join("parakeet-gpu-physical-identity");
    let _ = fs::remove_dir_all(&root);
    let source = required_path("PARAKEET_L2_EN_WAV");
    let model_path = required_path("PARAKEET_L2_MODEL");
    let record = model(ModelId::ParakeetTdt06BV3);
    let mut qualified_gpu = ArchiveQualificationKey::parakeet_gpu_test();
    assert!(qualified_gpu.is_parakeet_import_test());
    let request = ImportRequest {
        strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
        job_id: JobId(0x1b1),
        generation: Generation::first(),
        source_path: source.to_string_lossy().into_owned(),
        source_sha256: None,
        source_samples: None,
        model_path: model_path.to_string_lossy().into_owned(),
        model_sha256: decode_sha256(record.artifact.sha256),
        destination: root.to_string_lossy().into_owned(),
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Gpu,
        },
    };
    let prepared = ArchiveStore::new(&root)
        .prepare_with_qualification(request, None, Some(qualified_gpu.clone()))
        .unwrap();
    let pending_path = root
        .join("transcriptions")
        .join(format!("{:032x}", prepared.job_id.0))
        .join(prepared.generation.get().to_string())
        .join("pending.json");
    let mut pending: serde_json::Value =
        serde_json::from_slice(&fs::read(&pending_path).unwrap()).unwrap();
    pending["qualification"]["gpu_uuid"] = serde_json::json!("GPU-foreign-device");
    qualified_gpu.gpu_uuid = Some("GPU-foreign-device".into());
    assert!(
        !qualified_gpu.is_parakeet_import_test(),
        "another physical GPU cannot inherit the qualified key"
    );
    let bytes = serde_json::to_vec(&pending).unwrap();
    fs::write(&pending_path, &bytes).unwrap();

    let mut resumed = prepared.clone();
    resumed.generation = prepared.generation.next().unwrap();
    let error = ArchiveStore::new(&root)
        .resume_from_with_qualification(
            &prepared,
            resumed,
            Some(&prepared.model_path),
            Some(ArchiveQualificationKey::parakeet_gpu_test()),
        )
        .unwrap_err();
    assert!(error.contains("StorageCorrupt"), "{error}");
    assert_eq!(fs::read(&pending_path).unwrap(), bytes);
    assert!(source.is_file());
}

#[test]
fn parakeet_pending_queue_resumes_only_the_same_model_source_and_prefix() {
    if not_run_without_l2_archive_inputs() {
        return;
    }
    let destination = archive_root().join("parakeet-queue-resume");
    let _ = fs::remove_dir_all(&destination);
    let source = required_path("PARAKEET_L2_EN_WAV");
    let model_path = required_path("PARAKEET_L2_MODEL");
    let model_path_text = model_path.to_string_lossy().into_owned();
    let record = model(ModelId::ParakeetTdt06BV3);
    let initial = ImportRequest {
        strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
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
fn archive_admission_accepts_exactly_qualified_limit_and_refuses_one_extra_sample() {
    if not_run_without_l2_archive_inputs() {
        return;
    }
    let root = archive_root().join("parakeet-duration-boundary");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let max_samples = whisper_core::model_registry::PARAKEET_MAX_SINGLE_PASS_DURATION_MS * 16;
    let exact = root.join("exact-qualified-limit.wav");
    let over = root.join("over-qualified-limit-one-sample.wav");
    write_silence_wav(&exact, max_samples);
    write_silence_wav(&over, max_samples + 1);
    let model_path = required_path("PARAKEET_L2_MODEL");
    let record = model(ModelId::ParakeetTdt06BV3);
    for (job, source, expected) in [(0x1a7, exact, Ok(())), (0x1a8, over, Err(()))] {
        let request = ImportRequest {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
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

#[test]
fn resume_refuses_over_limit_pending_and_preserves_its_bytes() {
    if not_run_without_l2_archive_inputs() {
        return;
    }
    let root = archive_root().join("parakeet-duration-resume-boundary");
    let _ = fs::remove_dir_all(&root);
    let source = required_path("PARAKEET_L2_EN_WAV");
    let model_path = required_path("PARAKEET_L2_MODEL");
    let record = model(ModelId::ParakeetTdt06BV3);
    let request = ImportRequest {
        strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
        job_id: JobId(0x1a9),
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
    pending["source_samples"] = serde_json::json!(
        whisper_core::model_registry::PARAKEET_MAX_SINGLE_PASS_DURATION_MS * 16 + 1
    );
    let bytes = serde_json::to_vec(&pending).unwrap();
    fs::write(&pending_path, &bytes).unwrap();

    let mut next = prepared.clone();
    next.generation = prepared.generation.next().unwrap();
    let error = ArchiveStore::new(&root)
        .resume_from_with_qualification(
            &prepared,
            next,
            Some(&prepared.model_path),
            Some(qualified_key()),
        )
        .unwrap_err();
    assert!(
        error.contains("Fichier trop long pour Parakeet, utilisez Turbo"),
        "{error}"
    );
    assert!(error.contains("pending est conservé"), "{error}");
    assert_eq!(fs::read(&pending_path).unwrap(), bytes);
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

fn pcm16_mono_wav(path: &std::path::Path) -> Vec<i16> {
    let bytes = fs::read(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    assert!(bytes.len() >= 44 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WAVE");
    let mut offset = 12;
    let mut format = None;
    let mut data = None;
    while offset + 8 <= bytes.len() {
        let id = &bytes[offset..offset + 4];
        let length = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        let start = offset + 8;
        let end = start
            .checked_add(length)
            .expect("WAV chunk length overflow");
        assert!(end <= bytes.len(), "truncated WAV chunk");
        if id == b"fmt " {
            assert!(length >= 16);
            format = Some((
                u16::from_le_bytes(bytes[start..start + 2].try_into().unwrap()),
                u16::from_le_bytes(bytes[start + 2..start + 4].try_into().unwrap()),
                u32::from_le_bytes(bytes[start + 4..start + 8].try_into().unwrap()),
                u16::from_le_bytes(bytes[start + 12..start + 14].try_into().unwrap()),
                u16::from_le_bytes(bytes[start + 14..start + 16].try_into().unwrap()),
            ));
        } else if id == b"data" {
            data = Some(&bytes[start..end]);
        }
        offset = end + (length & 1);
    }
    assert_eq!(format, Some((1, 1, 16_000, 2, 16)));
    let data = data.expect("WAV data chunk");
    assert_eq!(data.len() % 2, 0);
    data.chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
        .collect()
}

#[test]
#[ignore = "reports B9 noise-floor silence and cut counts from R5-2 fixtures"]
fn r5_2_b9_reports_noise_floor_silences_and_cut_plans() {
    let allowed = required_path("PARAKEET_L2_RUNTIME_ROOT")
        .join("r5-2")
        .canonicalize()
        .unwrap();
    let cases = [
        ("-40", "PARAKEET_L2_NOISE_MINUS40_WAV"),
        ("-30", "PARAKEET_L2_NOISE_MINUS30_WAV"),
        ("-20", "PARAKEET_L2_NOISE_MINUS20_WAV"),
    ];
    for (level, variable) in cases {
        let path = required_path(variable);
        let canonical = path.canonicalize().unwrap();
        assert!(canonical.starts_with(&allowed), "B9 fixture escaped R5-2");
        let samples = pcm16_mono_wav(&canonical);
        let silences = whisper_core::parakeet_windows::admissible_silences(&samples);
        match whisper_core::parakeet_windows::plan_windows(&samples) {
            Ok(windows) => eprintln!(
                "R5_2_B9 level_dbfs={level} duration_samples={} admissible_silences={} cuts={} result=planned",
                samples.len(),
                silences.len(),
                windows.iter().filter(|window| window.cut.is_some()).count()
            ),
            Err(error) => eprintln!(
                "R5_2_B9 level_dbfs={level} duration_samples={} admissible_silences={} cuts=0 result=error:{error}",
                samples.len(),
                silences.len(),
            ),
        }
    }
}

#[test]
#[ignore = "runs real Parakeet CPU import and verifies the published archive"]
fn r5_2_a4_product_route_publishes_french_and_english_txt_srt() {
    let runtime = required_path("PARAKEET_L2_RUNTIME_ROOT");
    let model_path = required_path("PARAKEET_L2_MODEL");
    let worker = required_path("PARAKEET_L2_WORKER");
    let native = required_path("PARAKEET_L2_NATIVE_DIR");
    let fixtures = [
        ("fr87", "PARAKEET_L2_FR87_WAV", JobId(0x1a4)),
        ("fr27", "PARAKEET_L2_FR27_WAV", JobId(0x1a5)),
        ("en15", "PARAKEET_L2_EN_WAV", JobId(0x1a6)),
    ];
    for (name, variable, job_id) in fixtures {
        let source = required_path(variable);
        let destination = archive_root().join("a4").join(name);
        let _ = fs::remove_dir_all(&destination);
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
                native_dir: native.clone(),
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
            source_samples: None,
            model_path: model_path.to_string_lossy().into_owned(),
            model_sha256: decode_sha256(model(ModelId::ParakeetTdt06BV3).artifact.sha256),
            destination: destination.to_string_lossy().into_owned(),
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Cpu,
            },
        };
        app.dispatch(AppCommand::StartImport { request }).unwrap();
        let deadline = Instant::now() + Duration::from_secs(300);
        loop {
            let view = app.dispatch(AppCommand::Refresh).unwrap_or_else(|error| {
                panic!(
                    "refresh failed for {name}: {error:?}; view={:?}",
                    app.view()
                )
            });
            if view.last_result.is_some() {
                break;
            }
            if let Some((_, state)) = view.active_job {
                assert_ne!(
                    state,
                    whisper_core::JobState::Recoverable,
                    "A4 {name} failed: {view:?}"
                );
                assert_ne!(
                    state,
                    whisper_core::JobState::Interrupted,
                    "A4 {name} interrupted: {view:?}"
                );
            }
            assert!(Instant::now() < deadline, "A4 {name} timed out: {view:?}");
            thread::sleep(Duration::from_millis(20));
        }
        let passage = destination
            .join("transcriptions")
            .join(format!("{:032x}", job_id.0))
            .join("1");
        let txt = fs::read_to_string(passage.join("transcript.txt")).unwrap();
        let srt = fs::read_to_string(passage.join("transcript.srt")).unwrap();
        assert!(!txt.is_empty(), "A4 {name} TXT is empty");
        assert!(!srt.is_empty(), "A4 {name} SRT is empty");
        assert!(!txt.contains('\u{fffd}'), "A4 {name} emitted U+FFFD");
        assert!(!srt.contains('\u{fffd}'), "A4 {name} SRT emitted U+FFFD");
        if name == "fr87" {
            let normalized = txt.split_whitespace().collect::<Vec<_>>().join(" ");
            assert!(normalized.contains("faire? Je"), "A4 FR87 boundary: {txt}");
        }
        if name == "fr27" {
            let last_word = txt
                .split_whitespace()
                .last()
                .expect("French TXT has a last word");
            assert!(
                last_word.chars().any(char::is_alphabetic),
                "A4 FR27 final token is not French text: {last_word:?}"
            );
        }
        let entries = ArchiveStore::scan(&destination).expect("scan A4 archive");
        assert!(
            entries
                .iter()
                .any(|entry| entry.job_id == job_id && entry.complete)
        );
        eprintln!(
            "R5_2_A4 fixture={name} job={job_id:?} TXT={} SRT={} destination={}",
            passage.join("transcript.txt").display(),
            passage.join("transcript.srt").display(),
            destination.display()
        );
    }
}

fn decode_sha256(value: &str) -> [u8; 32] {
    let mut bytes = [0; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).unwrap();
    }
    bytes
}

mod r5_2_long_tests {
    use std::{
        fs,
        path::{Path, PathBuf},
        thread,
        time::{Duration, Instant},
    };

    use sha2::{Digest, Sha256};
    use whisper_adapters::parakeet_import::{ParakeetProcess, ParakeetWorkerPaths};
    use whisper_adapters::{
        archive::ArchiveStore, supervisor::WorkerPaths, worker_ipc::AsyncImportIo,
    };
    use whisper_core::{
        AppCommand, Application, ComputeChoice, Generation, ImportApplication, ImportRequest,
        JobConfig, JobId, LanguageChoice,
        model_catalog::{ModelAvailability, ModelId},
        model_registry,
        model_selection_app::ModelCommandGuard,
        parakeet_ipc::{Event, WorkerIdentity},
        parakeet_windows::{
            MAX_NATIVE_SAMPLES, PROFILE_IMPORT_VAD, PROFILE_QUALIFICATION_REFERENCE, Window,
            absolute_word_samples, plan_windows, word_belongs_to_central_range,
        },
    };

    const MODEL_SHA256: &str = "e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e";
    const NATIVE_DLLS: [&str; 5] = [
        "nemo_speech_asr_c.dll",
        "nemo_speech_asr.dll",
        "ggml.dll",
        "ggml-base.dll",
        "ggml-cpu.dll",
    ];

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct TimedWord {
        text: String,
        start_ms: u32,
        end_ms: u32,
    }

    struct Inference {
        identity: WorkerIdentity,
        duration_samples: u64,
        text: String,
        words: Vec<TimedWord>,
    }

    fn path_var(name: &str) -> PathBuf {
        PathBuf::from(std::env::var_os(name).unwrap_or_else(|| panic!("{name} is required")))
    }

    fn sha256(path: &Path) -> String {
        let bytes = fs::read(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        format!("{:x}", Sha256::digest(bytes))
    }

    fn runtime_paths() -> (PathBuf, ParakeetWorkerPaths) {
        let anchor = path_var("PARAKEET_L2_RUNTIME_ROOT");
        let runtime = path_var("PARAKEET_L2_ARCHIVE_ROOT");
        let paths = ParakeetWorkerPaths {
            executable: path_var("PARAKEET_L2_WORKER"),
            native_dir: path_var("PARAKEET_L2_NATIVE_DIR"),
            model: path_var("PARAKEET_L2_MODEL"),
            runtime_root: anchor,
        };
        paths
            .validate()
            .expect("worker, native DLLs and model paths are qualified");
        paths
            .verify_qualification(
                MODEL_SHA256,
                &runtime_hash(&paths.native_dir),
                &sha256(&paths.executable),
            )
            .expect("the exact L2b runtime, worker and model key is present");
        (runtime, paths)
    }

    fn runtime_hash(native: &Path) -> String {
        let mut hasher = Sha256::new();
        hasher.update(b"nemo-speech-asr 0.2.0");
        for name in NATIVE_DLLS {
            hasher.update(name.as_bytes());
            hasher.update(sha256(&native.join(name)).as_bytes());
        }
        format!("{:x}", hasher.finalize())
    }

    fn start_process(
        paths: &ParakeetWorkerPaths,
        profile: &str,
    ) -> (ParakeetProcess, WorkerIdentity) {
        let mut process = ParakeetProcess::spawn_for_qualification(paths, profile)
            .unwrap_or_else(|error| panic!("worker startup failed: {error}"));
        let ready = wait_event(&mut process, Duration::from_secs(120));
        let Event::Ready {
            identity,
            backend,
            native_version,
            ..
        } = ready
        else {
            panic!("worker did not attest Ready: {ready:?}");
        };
        assert_eq!(identity.model_sha256, MODEL_SHA256);
        assert_eq!(identity.runtime_sha256, runtime_hash(&paths.native_dir));
        assert_eq!(identity.build_sha256, sha256(&paths.executable));
        assert_eq!(identity.profile, profile);
        assert_eq!(backend, "CPU");
        assert!(native_version.contains("0.2.0"));
        process.accept_ready(identity.clone()).unwrap();
        (process, identity)
    }

    fn wait_event(process: &mut ParakeetProcess, limit: Duration) -> Event {
        let started = Instant::now();
        loop {
            match process.poll() {
                Ok(Some(event)) => return event,
                Ok(None) => thread::sleep(Duration::from_millis(20)),
                Err(error) => panic!("worker event failed: {error}"),
            }
            assert!(
                started.elapsed() < limit,
                "worker event exceeded the test bound"
            );
        }
    }

    fn transcribe(
        process: &mut ParakeetProcess,
        identity: &WorkerIdentity,
        source: &Path,
        job_id: &str,
        generation: u64,
        attempt: u64,
        limit: Duration,
    ) -> Inference {
        process
            .transcribe(job_id.into(), generation, attempt, source, sha256(source))
            .unwrap();
        let event = wait_event(process, limit);
        let Event::Result {
            job_id: got_job,
            generation: got_generation,
            attempt: got_attempt,
            identity: got_identity,
            duration_samples,
            text,
            words,
            ..
        } = event
        else {
            panic!("transcription did not return Result: {event:?}");
        };
        assert_eq!(got_job, job_id);
        assert_eq!(got_generation, generation);
        assert_eq!(got_attempt, attempt);
        assert_eq!(&got_identity, identity);
        Inference {
            identity: got_identity,
            duration_samples,
            text,
            words: words
                .into_iter()
                .map(|word| TimedWord {
                    text: word.text,
                    start_ms: word.start_ms,
                    end_ms: word.end_ms,
                })
                .collect(),
        }
    }

    fn stop_process(process: &mut ParakeetProcess) {
        process.shutdown().unwrap();
        assert!(matches!(
            wait_event(process, Duration::from_secs(120)),
            Event::Stopped
        ));
        let started = Instant::now();
        loop {
            match process.try_wait().unwrap() {
                Some(status) => {
                    assert!(status.success(), "worker exit status: {status}");
                    return;
                }
                None => thread::sleep(Duration::from_millis(20)),
            }
            assert!(started.elapsed() < Duration::from_secs(120));
        }
    }

    fn read_pcm16_mono(path: &Path) -> Vec<i16> {
        let bytes = fs::read(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
        assert!(bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WAVE");
        let mut offset = 12_usize;
        let mut format = None;
        let mut data = None;
        while offset + 8 <= bytes.len() {
            let id = &bytes[offset..offset + 4];
            let length =
                u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
            let start = offset + 8;
            let end = start
                .checked_add(length)
                .filter(|end| *end <= bytes.len())
                .unwrap();
            if id == b"fmt " {
                assert!(length >= 16);
                format = Some((
                    u16::from_le_bytes(bytes[start..start + 2].try_into().unwrap()),
                    u16::from_le_bytes(bytes[start + 2..start + 4].try_into().unwrap()),
                    u32::from_le_bytes(bytes[start + 4..start + 8].try_into().unwrap()),
                    u16::from_le_bytes(bytes[start + 12..start + 14].try_into().unwrap()),
                    u16::from_le_bytes(bytes[start + 14..start + 16].try_into().unwrap()),
                ));
            } else if id == b"data" {
                data = Some(&bytes[start..end]);
            }
            offset = end + (length & 1);
        }
        assert_eq!(format, Some((1, 1, 16_000, 2, 16)));
        let data = data.expect("WAV data chunk");
        assert_eq!(data.len() % 2, 0);
        data.chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
            .collect()
    }

    fn write_pcm16_mono(path: &Path, samples: &[i16]) {
        let data_len = u32::try_from(samples.len() * 2).unwrap();
        let mut bytes = Vec::with_capacity(data_len as usize + 44);
        bytes.extend_from_slice(b"RIFF");
        bytes.extend_from_slice(&(data_len + 36).to_le_bytes());
        bytes.extend_from_slice(b"WAVEfmt ");
        bytes.extend_from_slice(&16_u32.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&1_u16.to_le_bytes());
        bytes.extend_from_slice(&16_000_u32.to_le_bytes());
        bytes.extend_from_slice(&32_000_u32.to_le_bytes());
        bytes.extend_from_slice(&2_u16.to_le_bytes());
        bytes.extend_from_slice(&16_u16.to_le_bytes());
        bytes.extend_from_slice(b"data");
        bytes.extend_from_slice(&data_len.to_le_bytes());
        for sample in samples {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        fs::write(path, bytes).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    }

    fn infer_reference(paths: &ParakeetWorkerPaths, source: &Path, job: &str) -> Inference {
        let (mut process, identity) = start_process(paths, PROFILE_QUALIFICATION_REFERENCE);
        let result = transcribe(
            &mut process,
            &identity,
            source,
            job,
            1,
            1,
            Duration::from_secs(5_400),
        );
        stop_process(&mut process);
        result
    }

    fn infer_vad(
        runtime: &Path,
        paths: &ParakeetWorkerPaths,
        source: &Path,
        job: &str,
    ) -> (Inference, Vec<Window>, Vec<TimedWord>) {
        let pcm = read_pcm16_mono(source);
        let windows =
            plan_windows(&pcm).unwrap_or_else(|error| panic!("VAD planning failed: {error}"));
        assert!(
            windows.len() > 1,
            "long test source should require VAD windows"
        );
        let vad_dir = runtime.join("long").join(job).join("vad");
        fs::create_dir_all(&vad_dir).unwrap();
        let (mut process, identity) = start_process(paths, PROFILE_IMPORT_VAD);
        let duration_samples = pcm.len() as u64;
        let mut all_words = Vec::new();
        let mut texts = Vec::new();
        for window in &windows {
            assert!(window.native_end_sample - window.native_start_sample <= MAX_NATIVE_SAMPLES);
            let chunk_path = vad_dir.join(format!("window-{:03}.wav", window.index));
            let chunk =
                &pcm[window.native_start_sample as usize..window.native_end_sample as usize];
            write_pcm16_mono(&chunk_path, chunk);
            let result = transcribe(
                &mut process,
                &identity,
                &chunk_path,
                job,
                1,
                window.index as u64 + 1,
                Duration::from_secs(1_800),
            );
            assert_eq!(result.duration_samples, chunk.len() as u64);
            texts.push(result.text);
            let ipc_words = result
                .words
                .iter()
                .map(|word| whisper_core::parakeet_ipc::Word {
                    text: word.text.clone(),
                    start_ms: word.start_ms,
                    end_ms: word.end_ms,
                })
                .collect::<Vec<_>>();
            let absolute = absolute_word_samples(window, duration_samples, &ipc_words)
                .unwrap_or_else(|error| panic!("invalid absolute timestamps: {error}"));
            for (word, timing) in result.words.into_iter().zip(absolute) {
                let start = timing.start_sample;
                let end = timing.end_sample;
                if word_belongs_to_central_range(
                    start,
                    end,
                    window.central_start_sample,
                    window.central_end_sample,
                ) {
                    all_words.push(TimedWord {
                        text: word.text,
                        start_ms: u32::try_from(start / 16).unwrap(),
                        end_ms: u32::try_from(end / 16).unwrap(),
                    });
                }
            }
        }
        stop_process(&mut process);
        all_words.sort_by_key(|word| (word.start_ms, word.end_ms));
        assert!(
            all_words.iter().all(|word| word.start_ms < word.end_ms
                && u64::from(word.end_ms) <= duration_samples / 16)
        );
        let inference = Inference {
            identity,
            duration_samples,
            text: texts.join(" "),
            words: all_words.clone(),
        };
        (inference, windows, all_words)
    }

    struct ProductImport {
        transcript: String,
        segments: Vec<(u64, u64, u64, String)>,
        duration_samples: u64,
    }

    fn import_through_product_route(
        runtime: &Path,
        paths: &ParakeetWorkerPaths,
        source: &Path,
        job_nonce: u64,
    ) -> ProductImport {
        let destination = runtime.join(format!("archive/product-vad-{job_nonce}"));
        fs::create_dir_all(&destination).unwrap();
        let turbo_path =
            path_var("PARAKEET_L2_RUNTIME_ROOT").join("historical/ggml-large-v3-turbo.bin");
        let turbo_decoder = path_var("PARAKEET_L2_TURBO_DECODE_WORKER");
        let io = AsyncImportIo::start_with_model_catalog_and_parakeet(
            WorkerPaths {
                cpu: turbo_decoder.clone(),
                gpu: turbo_decoder,
            },
            paths.model.parent().unwrap().to_path_buf(),
            turbo_path.to_string_lossy().into_owned(),
            Some(paths.clone()),
        );
        let snapshot = model_registry::engine_snapshot(1, |_| ModelAvailability::Ready, vec![]);
        let guard = ModelCommandGuard::new_with_model_paths(
            snapshot,
            1,
            [
                (
                    ModelId::ParakeetTdt06BV3,
                    paths.model.to_string_lossy().into_owned(),
                ),
                (
                    ModelId::WhisperLargeV3Turbo,
                    turbo_path.to_string_lossy().into_owned(),
                ),
            ],
        );
        let mut app = ImportApplication::new(io).with_model_guard(guard);
        let record = model_registry::model(ModelId::ParakeetTdt06BV3);
        let job_id = JobId(0x4c32_0000_0000_0000_u128 + u128::from(job_nonce));
        let mut request = ImportRequest {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
            job_id,
            generation: Generation::first(),
            source_path: source.to_string_lossy().into_owned(),
            source_sha256: None,
            source_samples: None,
            model_path: paths.model.to_string_lossy().into_owned(),
            model_sha256: decode_sha256(record.artifact.sha256),
            destination: destination.to_string_lossy().into_owned(),
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Cpu,
            },
        };
        ArchiveStore::identify_source(&mut request).unwrap();
        let duration_samples = request.source_samples.unwrap();
        app.dispatch(AppCommand::StartImport { request }).unwrap();
        let deadline = Instant::now() + Duration::from_secs(3_600);
        loop {
            let view = app.dispatch(AppCommand::Refresh).unwrap();
            if view.active_job == Some((job_id, whisper_core::JobState::Complete)) {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "product VAD import did not complete: {view:?}"
            );
            thread::sleep(Duration::from_millis(100));
        }
        let passage = destination
            .join("transcriptions")
            .join(format!("{:032x}", job_id.0))
            .join("1");
        let complete = ArchiveStore::scan(&destination).unwrap();
        assert_eq!(complete.len(), 1);
        assert!(
            complete[0].complete,
            "product route published one generation"
        );
        assert!(passage.join("CURRENT").is_file());
        assert!(!passage.join("pending.json").exists());
        let transcript = fs::read_to_string(passage.join("transcript.txt")).unwrap();
        let segments = fs::read_to_string(passage.join("segments.jsonl"))
            .unwrap()
            .lines()
            .map(|line| {
                let row: serde_json::Value = serde_json::from_str(line).unwrap();
                let range = &row["range"];
                (
                    row["sequence"].as_u64().unwrap(),
                    range["start_sample"].as_u64().unwrap(),
                    range["end_sample"].as_u64().unwrap(),
                    row["text"].as_str().unwrap().to_owned(),
                )
            })
            .collect();
        ProductImport {
            transcript,
            segments,
            duration_samples,
        }
    }

    fn verify_product_matches_vad_route(
        product: ProductImport,
        vad: &Inference,
        windows: &[Window],
    ) {
        assert_eq!(product.duration_samples, vad.duration_samples);
        let expected = vad
            .words
            .iter()
            .flat_map(|word| normalized_tokens(&word.text))
            .collect::<Vec<_>>();
        let actual = normalized_tokens(&product.transcript);
        assert_eq!(
            actual, expected,
            "product route transcript matches the same-candidate VAD output"
        );
        assert!(!product.segments.is_empty());
        for (index, (sequence, start, end, text)) in product.segments.iter().enumerate() {
            assert_eq!(
                *sequence,
                index as u64 + 1,
                "one contiguous publication sequence"
            );
            assert!(!text.trim().is_empty());
            assert!(start < end && *end <= product.duration_samples);
            let owners = windows
                .iter()
                .filter(|window| {
                    *start >= window.central_start_sample && *end <= window.central_end_sample
                })
                .count();
            assert_eq!(owners, 1, "published segment belongs to one central window");
        }
        let cut_count = windows.iter().filter(|window| window.cut.is_some()).count();
        eprintln!(
            "L2B_PRODUCT_ROUTE profile=import-vad duration_samples={} windows={} cuts={} segments={} token_count={} duplicate_segments=0 publication_count=1",
            product.duration_samples,
            windows.len(),
            cut_count,
            product.segments.len(),
            actual.len()
        );
    }

    fn decode_sha256(value: &str) -> [u8; 32] {
        let mut bytes = [0_u8; 32];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).unwrap();
        }
        bytes
    }

    fn normalized_tokens(text: &str) -> Vec<String> {
        let mut words = Vec::new();
        let mut current = String::new();
        for character in text.chars().flat_map(char::to_lowercase) {
            if character.is_alphanumeric() {
                current.push(character);
            } else if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
        }
        if !current.is_empty() {
            words.push(current);
        }
        let mut normalized = Vec::with_capacity(words.len());
        let mut index = 0;
        while index < words.len() {
            if matches!(
                words[index].as_str(),
                "um" | "uh" | "euh" | "hum" | "ben" | "bah"
            ) {
                index += 1;
            } else if words[index] == "you"
                && words.get(index + 1).is_some_and(|word| word == "know")
            {
                index += 2;
            } else if words[index] == "i" && words.get(index + 1).is_some_and(|word| word == "mean")
            {
                index += 2;
            } else {
                normalized.push(words[index].clone());
                index += 1;
            }
        }
        normalized
    }

    fn edit_distance(left: &[String], right: &[String]) -> usize {
        let mut row = (0..=right.len()).collect::<Vec<_>>();
        for (left_index, left_word) in left.iter().enumerate() {
            let mut diagonal = row[0];
            row[0] = left_index + 1;
            for (right_index, right_word) in right.iter().enumerate() {
                let above = row[right_index + 1];
                row[right_index + 1] = (diagonal + usize::from(left_word != right_word))
                    .min(row[right_index] + 1)
                    .min(above + 1);
                diagonal = above;
            }
        }
        row[right.len()]
    }

    fn boundary_words(words: &[TimedWord], cut_ms: u32) -> Vec<String> {
        words
            .iter()
            .filter(|word| {
                let center = word.start_ms + (word.end_ms - word.start_ms) / 2;
                center.abs_diff(cut_ms) <= 2_000
            })
            .flat_map(|word| normalized_tokens(&word.text))
            .collect()
    }

    fn verify_comparison(
        reference: &Inference,
        vad: &Inference,
        windows: &[Window],
        words: &[TimedWord],
    ) {
        assert_eq!(reference.identity.model_sha256, vad.identity.model_sha256);
        assert_eq!(
            reference.identity.runtime_sha256,
            vad.identity.runtime_sha256
        );
        assert_eq!(reference.identity.build_sha256, vad.identity.build_sha256);
        assert_eq!(reference.duration_samples, vad.duration_samples);
        let reference_tokens = normalized_tokens(&reference.text);
        let vad_tokens = normalized_tokens(&vad.text);
        let global_edits = edit_distance(&reference_tokens, &vad_tokens);
        eprintln!(
            "L2B_LONG_SUMMARY profile=import-vad duration_samples={} windows={} reference_tokens={} vad_tokens={} global_edits={global_edits}",
            vad.duration_samples,
            windows.len(),
            reference_tokens.len(),
            vad_tokens.len()
        );
        for window in windows.iter().filter(|window| window.cut.is_some()) {
            let cut_ms = (window.central_end_sample / 16) as u32;
            let ref_zone = boundary_words(&reference.words, cut_ms);
            let vad_zone = boundary_words(words, cut_ms);
            let edits = edit_distance(&ref_zone, &vad_zone);
            let truncated = words
                .iter()
                .filter(|word| word.start_ms < cut_ms && cut_ms < word.end_ms)
                .count();
            eprintln!(
                "L2B_BOUNDARY cut_sample={} cut_ms={} reference_words={} vad_words={} edits={} truncated={truncated}",
                window.central_end_sample,
                cut_ms,
                ref_zone.len(),
                vad_zone.len(),
                edits,
            );
            assert!(edits <= 2, "more than two lexical edits at cut {cut_ms} ms");
            assert_eq!(truncated, 0, "word crossed the central cut at {cut_ms} ms");
        }
        for pair in words.windows(2) {
            let left = normalized_tokens(&pair[0].text);
            let right = normalized_tokens(&pair[1].text);
            if pair[0].end_ms > pair[1].start_ms && !left.is_empty() && left == right {
                panic!(
                    "duplicate overlapping boundary word: {:?} / {:?}",
                    pair[0], pair[1]
                );
            }
        }
    }

    #[test]
    #[ignore = "runs same-candidate 60-minute reference and CPU VAD qualification"]
    fn en_sixty_minute_reference_and_vad_use_same_candidate_and_boundary_policy() {
        let (runtime, paths) = runtime_paths();
        let source = path_var("PARAKEET_L2_EN_LONG_WAV");
        let reference = infer_reference(&paths, &source, "en60-reference");
        let (vad, windows, words) = infer_vad(&runtime, &paths, &source, "en60");
        verify_comparison(&reference, &vad, &windows, &words);
        let product = import_through_product_route(&runtime, &paths, &source, 60);
        verify_product_matches_vad_route(product, &vad, &windows);
    }

    #[test]
    #[ignore = "runs long synthetic French reference and VAD; boundaries are artificial"]
    fn fr_synthetic_long_reference_and_vad_use_same_candidate() {
        let (runtime, paths) = runtime_paths();
        let source = path_var("PARAKEET_L2_FR_LONG_WAV");
        let reference = infer_reference(&paths, &source, "fr-long-reference");
        let (vad, windows, words) = infer_vad(&runtime, &paths, &source, "fr-long");
        eprintln!(
            "L2B_FR_SOURCE synthetic=true duration_samples={}",
            vad.duration_samples
        );
        verify_comparison(&reference, &vad, &windows, &words);
        let product = import_through_product_route(&runtime, &paths, &source, 61);
        verify_product_matches_vad_route(product, &vad, &windows);
    }
}
