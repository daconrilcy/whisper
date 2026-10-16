use std::path::PathBuf;

use whisper_adapters::model_catalog::LocalModelCatalog;
use whisper_core::{
    domain::{ComputeChoice, Generation, JobConfig, JobId, LanguageChoice},
    model_catalog::{ModelAvailability, ModelId},
    model_registry::MODELS,
    ports::ImportRequest,
};

fn models_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../models")
}

fn runtime_root(name: &str) -> PathBuf {
    PathBuf::from(std::env::var_os("WHISPER_L1B_RUNTIME_ROOT").expect("bounded runtime root"))
        .join(name)
}

fn turbo_request(model_path: String) -> ImportRequest {
    let turbo = MODELS[0].artifact;
    let mut sha = [0; 32];
    for (index, byte) in sha.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&turbo.sha256[index * 2..index * 2 + 2], 16).unwrap();
    }
    ImportRequest {
        job_id: JobId(7),
        generation: Generation::first(),
        source_path: "fixture.wav".into(),
        source_sha256: Some([1; 32]),
        source_samples: Some(1600),
        model_path,
        model_sha256: sha,
        destination: "output".into(),
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        },
    }
}

#[test]
fn closed_catalog_reports_both_pinned_model_files_and_exact_observations() {
    let snapshot = LocalModelCatalog::new(models_root()).inspect(9);
    assert_eq!(snapshot.entries.len(), 2);
    for record in MODELS {
        let entry = snapshot
            .entries
            .iter()
            .find(|entry| entry.id == record.artifact.id)
            .unwrap();
        assert_eq!(entry.availability, ModelAvailability::Ready);
        assert_eq!(entry.observed_size_bytes, Some(record.artifact.size_bytes));
        assert_eq!(
            entry.observed_sha256.as_deref(),
            Some(record.artifact.sha256)
        );
    }
    assert!(
        snapshot
            .entries
            .iter()
            .any(|entry| entry.id == ModelId::ParakeetTdt06BV3)
    );
}

#[test]
fn missing_and_corrupt_models_are_reported_without_touching_the_catalog_root() {
    let missing_root = runtime_root("catalog-missing");
    let missing = LocalModelCatalog::new(&missing_root).inspect(1);
    assert!(
        missing
            .entries
            .iter()
            .all(|entry| entry.availability == ModelAvailability::Missing)
    );

    let corrupt_root = runtime_root("catalog-corrupt");
    std::fs::create_dir_all(&corrupt_root).unwrap();
    std::fs::write(
        corrupt_root.join("ggml-large-v3-turbo.bin"),
        b"not the approved artifact",
    )
    .unwrap();
    let corrupt = LocalModelCatalog::new(&corrupt_root).inspect(2);
    assert_eq!(
        corrupt
            .entries
            .iter()
            .find(|entry| entry.id == ModelId::WhisperLargeV3Turbo)
            .unwrap()
            .availability,
        ModelAvailability::Corrupt
    );
}

#[test]
fn known_historical_turbo_path_rebinds_to_the_verified_test_catalog_path() {
    let known =
        r"C:\Users\Cyril\AppData\Local\Programs\WhisperBuildDeps\models\ggml-large-v3-turbo.bin";
    let snapshot = LocalModelCatalog::new(models_root()).inspect(1);
    let durable = turbo_request(known.into());
    let executable = snapshot
        .bind_turbo_import_request(&durable, known)
        .expect("known historical Turbo path is recognized");
    assert_eq!(
        executable.model_path,
        models_root()
            .join("ggml-large-v3-turbo.bin")
            .to_string_lossy()
    );
    assert_eq!(
        durable.model_path, known,
        "durable request remains unchanged"
    );

    let unknown = turbo_request(r"C:\old\unknown-model.bin".into());
    assert!(
        snapshot
            .bind_turbo_import_request(&unknown, known)
            .unwrap_err()
            .contains("chemin de modèle inconnu")
    );
}
