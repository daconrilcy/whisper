use super::{LocalModelCatalog, model_sha256_full_read_count};
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};
use whisper_core::{model_catalog::ModelId, model_registry::model};

#[test]
fn resolving_registered_paths_and_historical_alias_never_reads_models() {
    let root = catalog_root(true);
    let catalog = LocalModelCatalog::new(&root);
    let historical = r"C:\legacy\large-v3-turbo.bin";
    for id in [ModelId::WhisperLargeV3Turbo, ModelId::ParakeetTdt06BV3] {
        let path = root.join(model(id).artifact.file_name);
        assert_eq!(
            catalog
                .resolve_requested_model(&path.to_string_lossy(), historical)
                .unwrap(),
            id
        );
        assert_eq!(model_sha256_full_read_count(&path).unwrap(), 0);
    }
    assert_eq!(
        catalog
            .resolve_requested_model(&historical.replace('\\', "/").to_uppercase(), historical)
            .unwrap(),
        ModelId::WhisperLargeV3Turbo
    );
    assert!(
        catalog
            .resolve_requested_model("unregistered.gguf", historical)
            .unwrap_err()
            .starts_with("ModelUnknown:")
    );
    for record in whisper_core::model_registry::MODELS {
        assert_eq!(
            model_sha256_full_read_count(&root.join(record.artifact.file_name)).unwrap(),
            0
        );
    }
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn targeted_inspection_preserves_generation_and_reads_only_the_requested_id() {
    for id in [ModelId::WhisperLargeV3Turbo, ModelId::ParakeetTdt06BV3] {
        let root = catalog_root(true);
        let catalog = LocalModelCatalog::new(&root);
        let snapshot = catalog.inspect_model(id, 17);
        assert_eq!(snapshot.generation, 17);
        assert_eq!(snapshot.entries.len(), 1);
        assert_eq!(snapshot.entries[0].id, id);
        for record in whisper_core::model_registry::MODELS {
            assert_eq!(
                model_sha256_full_read_count(&root.join(record.artifact.file_name)).unwrap(),
                u64::from(record.artifact.id == id)
            );
        }
        fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn importing_parakeet_inspects_only_parakeet_and_full_catalog_still_inspects_both() {
    let root = catalog_root(true);
    let catalog = LocalModelCatalog::new(&root);
    let parakeet_path = root.join(model(ModelId::ParakeetTdt06BV3).artifact.file_name);
    let turbo_path = root.join(model(ModelId::WhisperLargeV3Turbo).artifact.file_name);

    let (id, requested) = catalog
        .inspect_requested_model(
            &parakeet_path.to_string_lossy(),
            r"C:\legacy\large-v3-turbo.bin",
        )
        .expect("resolve requested Parakeet model");

    assert_eq!(id, ModelId::ParakeetTdt06BV3);
    assert_eq!(requested.entries.len(), 1);
    assert_eq!(requested.entries[0].id, ModelId::ParakeetTdt06BV3);
    assert_eq!(
        requested.entries[0].availability,
        whisper_core::model_catalog::ModelAvailability::Corrupt
    );
    assert_eq!(model_sha256_full_read_count(&turbo_path).unwrap(), 0);
    assert_eq!(model_sha256_full_read_count(&parakeet_path).unwrap(), 1);

    let full = catalog.inspect(1);
    assert_eq!(
        full.entries.len(),
        whisper_core::model_registry::MODELS.len()
    );
    assert_eq!(model_sha256_full_read_count(&turbo_path).unwrap(), 1);
    fs::remove_dir_all(root).expect("remove temporary catalog root");
}

#[test]
fn historical_turbo_alias_resolves_only_turbo_and_unknown_paths_stay_refused() {
    let root = catalog_root(true);
    let catalog = LocalModelCatalog::new(&root);
    let turbo_path = root.join(model(ModelId::WhisperLargeV3Turbo).artifact.file_name);
    let parakeet_path = root.join(model(ModelId::ParakeetTdt06BV3).artifact.file_name);
    let historical = r"C:\legacy\large-v3-turbo.bin";
    let alias = historical.replace('\\', "/").to_uppercase();

    let (id, requested) = catalog
        .inspect_requested_model(&alias, historical)
        .expect("resolve normalized historical Turbo alias");
    assert_eq!(id, ModelId::WhisperLargeV3Turbo);
    assert_eq!(requested.entries.len(), 1);
    assert_eq!(requested.entries[0].id, ModelId::WhisperLargeV3Turbo);
    assert_eq!(model_sha256_full_read_count(&turbo_path).unwrap(), 1);
    assert_eq!(model_sha256_full_read_count(&parakeet_path).unwrap(), 0);

    let error = catalog
        .inspect_requested_model(r"C:\unknown\model.gguf", historical)
        .unwrap_err();
    assert!(error.starts_with("ModelUnknown:"));
    assert_eq!(model_sha256_full_read_count(&parakeet_path).unwrap(), 0);
    fs::remove_dir_all(root).expect("remove temporary catalog root");
}

#[test]
fn missing_requested_model_remains_missing_without_reading_turbo() {
    let root = catalog_root(false);
    let catalog = LocalModelCatalog::new(&root);
    let parakeet_path = root.join(model(ModelId::ParakeetTdt06BV3).artifact.file_name);
    let turbo_path = root.join(model(ModelId::WhisperLargeV3Turbo).artifact.file_name);
    let (id, requested) = catalog
        .inspect_requested_model(&parakeet_path.to_string_lossy(), "legacy-turbo")
        .expect("resolve requested Parakeet model");
    assert_eq!(id, ModelId::ParakeetTdt06BV3);
    assert_eq!(
        requested.entries[0].availability,
        whisper_core::model_catalog::ModelAvailability::Missing
    );
    assert_eq!(model_sha256_full_read_count(&turbo_path).unwrap(), 0);
    fs::remove_dir_all(root).expect("remove temporary catalog root");
}

#[test]
fn requested_model_size_change_forces_rehash_and_remains_corrupt() {
    let root = catalog_root(true);
    let catalog = LocalModelCatalog::new(&root);
    let parakeet_path = root.join(model(ModelId::ParakeetTdt06BV3).artifact.file_name);
    let historical = r"C:\legacy\large-v3-turbo.bin";

    let (_, first) = catalog
        .inspect_requested_model(&parakeet_path.to_string_lossy(), historical)
        .expect("inspect requested Parakeet");
    assert_eq!(
        first.entries[0].availability,
        whisper_core::model_catalog::ModelAvailability::Corrupt
    );
    let first_hash = first.entries[0]
        .observed_sha256
        .as_deref()
        .expect("corrupt fixture still has an observed digest");
    assert_eq!(model_sha256_full_read_count(&parakeet_path).unwrap(), 1);

    fs::write(
        &parakeet_path,
        b"changed corrupt fixture with different size",
    )
    .expect("replace requested model fixture");
    let (_, second) = catalog
        .inspect_requested_model(&parakeet_path.to_string_lossy(), historical)
        .expect("reinspect changed Parakeet");
    assert_eq!(
        second.entries[0].availability,
        whisper_core::model_catalog::ModelAvailability::Corrupt
    );
    assert_eq!(
        second.entries[0].observed_size_bytes,
        Some(fs::metadata(&parakeet_path).unwrap().len())
    );
    assert_ne!(
        second.entries[0].observed_sha256.as_deref(),
        Some(first_hash),
        "changed identity must produce a freshly observed digest"
    );
    assert_eq!(model_sha256_full_read_count(&parakeet_path).unwrap(), 1);
    fs::remove_dir_all(root).expect("remove temporary catalog root");
}

fn catalog_root(include_parakeet: bool) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "requested-model-catalog-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir(&root).expect("create temporary model catalog");
    fs::write(
        root.join(model(ModelId::WhisperLargeV3Turbo).artifact.file_name),
        b"small invalid Turbo fixture",
    )
    .expect("write temporary Turbo fixture");
    if include_parakeet {
        fs::write(
            root.join(model(ModelId::ParakeetTdt06BV3).artifact.file_name),
            b"small invalid Parakeet fixture",
        )
        .expect("write temporary Parakeet fixture");
    }
    root.canonicalize().expect("canonicalize temporary catalog")
}
