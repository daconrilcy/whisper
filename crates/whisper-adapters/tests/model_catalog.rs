use std::path::PathBuf;

use whisper_adapters::model_catalog::LocalModelCatalog;
use whisper_core::{
    domain::{ComputeChoice, Generation, JobConfig, JobId, LanguageChoice},
    model_catalog::{ModelAvailability, ModelId},
    model_registry::MODELS,
    ports::ImportRequest,
};

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

fn models_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../models")
}

fn runtime_root(name: &str) -> PathBuf {
    PathBuf::from(std::env::var_os("WHISPER_L1B_RUNTIME_ROOT").expect("bounded runtime root"))
        .join(name)
}

fn cache_test_path(name: &str) -> PathBuf {
    let root = runtime_root("r9-model-hash-cache").join(format!("{}-{name}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    root
}

fn turbo_request(model_path: String) -> ImportRequest {
    let turbo = MODELS[0].artifact;
    let mut sha = [0; 32];
    for (index, byte) in sha.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&turbo.sha256[index * 2..index * 2 + 2], 16).unwrap();
    }
    ImportRequest {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
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
    if not_run_without_campaign_variables(&["WHISPER_L1B_RUNTIME_ROOT"]) {
        return;
    }
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

#[test]
fn model_hash_cache_reuses_physical_identity_and_rehashes_changed_fingerprints() {
    if not_run_without_campaign_variables(&["WHISPER_L1B_RUNTIME_ROOT"]) {
        return;
    }
    use whisper_adapters::model_catalog::{cached_model_sha256, model_sha256_full_read_count};

    let root = cache_test_path("identity");
    let model = root.join("model.gguf");
    std::fs::write(&model, b"first registered model bytes").unwrap();
    let first = cached_model_sha256(&model).unwrap();
    assert_eq!(model_sha256_full_read_count(&model).unwrap(), 1);

    let alias = root.join("model-hardlink.gguf");
    std::fs::hard_link(&model, &alias).unwrap();
    assert_eq!(cached_model_sha256(&alias).unwrap(), first);
    assert_eq!(model_sha256_full_read_count(&alias).unwrap(), 1);
    assert_eq!(model_sha256_full_read_count(&model).unwrap(), 1);

    let before_date_change = std::fs::metadata(&model).unwrap().modified().unwrap();
    let file = std::fs::OpenOptions::new()
        .write(true)
        .open(&model)
        .unwrap();
    file.set_times(
        std::fs::FileTimes::new()
            .set_modified(before_date_change + std::time::Duration::from_secs(2)),
    )
    .unwrap();
    assert_eq!(cached_model_sha256(&model).unwrap(), first);
    assert_eq!(model_sha256_full_read_count(&model).unwrap(), 1);

    std::fs::write(&model, b"modified model bytes with a new size").unwrap();
    let changed = cached_model_sha256(&model).unwrap();
    assert_ne!(changed.1, first.1);
    assert_eq!(model_sha256_full_read_count(&model).unwrap(), 1);
    assert_eq!(model_sha256_full_read_count(&model).unwrap(), 1);

    let copied = root.join("model-copy.gguf");
    std::fs::copy(&model, &copied).unwrap();
    assert_eq!(cached_model_sha256(&copied).unwrap(), changed);
    assert_eq!(model_sha256_full_read_count(&copied).unwrap(), 1);
    assert_eq!(model_sha256_full_read_count(&copied).unwrap(), 1);
}

#[test]
fn model_hash_cache_serializes_concurrent_first_reads_and_fails_closed_on_missing_files() {
    if not_run_without_campaign_variables(&["WHISPER_L1B_RUNTIME_ROOT"]) {
        return;
    }
    use std::sync::Arc;
    use whisper_adapters::model_catalog::{cached_model_sha256, model_sha256_full_read_count};

    let root = cache_test_path("concurrent");
    let model = root.join("model.gguf");
    std::fs::write(&model, vec![42_u8; 128 * 1024]).unwrap();
    let model = Arc::new(model);
    let threads = (0..8)
        .map(|_| {
            let model = Arc::clone(&model);
            std::thread::spawn(move || cached_model_sha256(&model).unwrap())
        })
        .collect::<Vec<_>>();
    let observations = threads
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect::<Vec<_>>();
    assert!(
        observations
            .iter()
            .all(|observation| *observation == observations[0])
    );
    assert_eq!(model_sha256_full_read_count(&model).unwrap(), 1);

    assert!(cached_model_sha256(&root.join("missing.gguf")).is_err());
    assert!(
        cached_model_sha256(&root).is_err(),
        "directories are not model files"
    );
}
