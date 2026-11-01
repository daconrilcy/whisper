use std::{fs, path::Path};

use whisper_adapters::archive::{ArchiveQualificationKey, ArchiveStore};
use whisper_core::{
    ComputeChoice, Generation, JobConfig, JobId, LanguageChoice,
    model_catalog::{Backend, ModelId},
    model_registry::{self, model},
};

fn key(backend: Backend) -> ArchiveQualificationKey {
    let record = model(ModelId::ParakeetTdt06BV3);
    let configuration = model_registry::personal_configuration_key(record, backend);
    let mut key = ArchiveQualificationKey::parakeet_cpu_test();
    key.artifact_revision = configuration.artifact_revision;
    key.runtime_build = configuration.runtime_build;
    key.worker_build = configuration.worker_build;
    key.inference_profile = configuration.inference_profile;
    if backend == Backend::Gpu {
        key.backend = "CUDA0".into();
        key.gpu_pci_bus_id = Some(model_registry::PARAKEET_PERSONAL_GPU_PCI_BUS_ID.into());
        key.gpu_uuid = Some(model_registry::PARAKEET_PERSONAL_GPU_UUID.into());
        key.gpu_pnp_instance_id =
            Some(model_registry::PARAKEET_PERSONAL_GPU_PNP_INSTANCE_ID.into());
    }
    key
}

fn pending(root: &Path, job: u128, key: &ArchiveQualificationKey) {
    let directory = root
        .join("transcriptions")
        .join(format!("{job:032x}"))
        .join("1");
    fs::create_dir_all(&directory).unwrap();
    let artifact = model(ModelId::ParakeetTdt06BV3).artifact;
    let pending = serde_json::json!({
        "version": 2,
        "job_id": job,
        "generation": 1,
        "source_path": "historical-source.wav",
        "source_sha256": "ab".repeat(32),
        "source_samples": 480000,
        "model_path": "historical-model.gguf",
        "model_sha256": artifact.sha256,
        "model_size": artifact.size_bytes,
        "qualification": key,
    });
    fs::write(
        directory.join("pending.json"),
        serde_json::to_vec(&pending).unwrap(),
    )
    .unwrap();
}

#[test]
fn personal_archives_recognize_previous_workers_without_admitting_them_for_execution() {
    let root = std::env::temp_dir().join(format!(
        "parakeet-personal-archive-history-{}",
        std::process::id()
    ));
    fs::create_dir(&root).unwrap();
    let root = root.canonicalize().unwrap();
    let mut expected = Vec::new();
    for (offset, backend, previous, current) in [
        (
            0_u128,
            Backend::Cpu,
            "84c371d4df55043ada4a1d96b80180ed0c03b208410c599262c8ea3762304e60",
            "a965c9503f909124f8ddedad09e83e5d187e70edd1869e880e9f1ad5ae2fb99d",
        ),
        (
            10_u128,
            Backend::Gpu,
            "bf58eb9d7300c444864347496b5a27e8584da3d30d634a759be99a598eda5ce4",
            "7ffb0b4a022ec26909a8939b37787de6df6c9bdb2903edf14d3ffb32579eefc1",
        ),
    ] {
        let active = key(backend);
        assert_eq!(active.worker_build, current);
        assert!(active.is_registered_personal_import());
        let mut old = active.clone();
        old.worker_build = previous.into();
        assert!(!old.is_registered_personal_import());
        assert!(!old.is_parakeet_import_test());
        let old_job = offset + 2;
        pending(&root, old_job, &old);
        let old_pending = root
            .join("transcriptions")
            .join(format!("{old_job:032x}"))
            .join("1/pending.json");
        let old_bytes = fs::read(&old_pending).unwrap();
        let artifact = model(ModelId::ParakeetTdt06BV3).artifact;
        let mut model_sha = [0_u8; 32];
        for (index, byte) in model_sha.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&artifact.sha256[index * 2..index * 2 + 2], 16).unwrap();
        }
        let previous = whisper_core::ports::ImportRequest {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
            job_id: JobId(old_job),
            generation: Generation::first(),
            source_path: "historical-source.wav".into(),
            source_sha256: Some([0xab; 32]),
            source_samples: Some(480_000),
            model_path: "historical-model.gguf".into(),
            model_sha256: model_sha,
            destination: root.to_string_lossy().into_owned(),
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Cpu,
            },
        };
        let mut request = previous.clone();
        request.generation = previous.generation.next().unwrap();
        let error = ArchiveStore::new(&root)
            .resume_from_with_qualification(&previous, request, None, Some(active.clone()))
            .unwrap_err();
        assert!(error.contains("SourceChanged"), "{error}");
        assert_eq!(fs::read(old_pending).unwrap(), old_bytes);
        let mut altered = old.clone();
        altered.runtime_build = "0".repeat(64);
        let mut unknown = active.clone();
        unknown.worker_build = "0".repeat(64);
        for (index, qualification, valid) in [
            (1, &active, true),
            (2, &old, true),
            (3, &altered, false),
            (4, &unknown, false),
        ] {
            let job = offset + index;
            if job != old_job {
                pending(&root, job, qualification);
            }
            expected.push((JobId(job), valid));
        }
    }
    for (job, hash, message) in [
        (
            101_u128,
            "545a4830755da613a1255be97fbc6a6a513ef837d669d8a9870d3bbee9dfd587",
            "L2a",
        ),
        (
            102_u128,
            "e7672a520db87926b3da9b8e5b585a1e7e57ce53ff8b1d0fe95251046aa01f4d",
            "R5-2",
        ),
    ] {
        let mut retired = ArchiveQualificationKey::parakeet_cpu_test();
        retired.worker_build = hash.into();
        pending(&root, job, &retired);
        expected.push((JobId(job), true));
        let file = root
            .join("transcriptions")
            .join(format!("{job:032x}"))
            .join("1/pending.json");
        let bytes = fs::read(&file).unwrap();
        let artifact = model(ModelId::ParakeetTdt06BV3).artifact;
        let mut model_sha = [0_u8; 32];
        for (index, byte) in model_sha.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&artifact.sha256[index * 2..index * 2 + 2], 16).unwrap();
        }
        let previous = whisper_core::ports::ImportRequest {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
            job_id: JobId(job),
            generation: Generation::first(),
            source_path: "historical-source.wav".into(),
            source_sha256: Some([0xab; 32]),
            source_samples: Some(480_000),
            model_path: "historical-model.gguf".into(),
            model_sha256: model_sha,
            destination: root.to_string_lossy().into_owned(),
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Cpu,
            },
        };
        let mut request = previous.clone();
        request.generation = previous.generation.next().unwrap();
        let error = ArchiveStore::new(&root)
            .resume_from_with_qualification(&previous, request, None, Some(retired))
            .unwrap_err();
        assert!(
            error.contains(message) && error.contains("retir"),
            "{error}"
        );
        assert_eq!(fs::read(file).unwrap(), bytes);
    }
    let entries = ArchiveStore::scan(&root).unwrap();
    assert_eq!(entries.len(), expected.len());
    for (job, valid) in expected {
        let entry = entries.iter().find(|entry| entry.job_id == job).unwrap();
        assert_eq!(
            entry.diagnostic.is_none(),
            valid,
            "job {job:?}: {:?}",
            entry.diagnostic
        );
        assert!(!entry.complete);
    }
}
