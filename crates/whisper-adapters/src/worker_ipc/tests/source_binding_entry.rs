#[test]
fn existing_source_binding_is_reused_and_failed_acquisition_is_not_inserted() {
    let root = std::env::temp_dir().join(format!(
        "whisper-source-binding-entry-{}-{}",
        std::process::id(),
        NEXT_INSTANCE.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&root).unwrap();
    let source = root.join("source.wav");
    let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source).unwrap();
    writer.append(&vec![0_i16; 16_000]).unwrap();
    writer.finish().unwrap();
    let registry = root.join("registry");
    let mut request = unified_import_request(&root, &source, 0xeca1);
    let key = (request.job_id, request.generation.get());
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: root.join("unused-cpu"),
            gpu: root.join("unused-gpu"),
        },
        Arc::new(PublicationGate::default()),
    );
    runtime.snapshot_registry_root = Some(registry.clone());
    runtime.ensure_source_binding(&mut request).unwrap();
    let snapshot = runtime.source_bindings[&key].snapshot_path().to_path_buf();

    let blocked_registry = root.join("not-a-directory");
    fs::write(&blocked_registry, b"not a directory").unwrap();
    runtime.snapshot_registry_root = Some(blocked_registry);
    runtime.ensure_source_binding(&mut request).unwrap();
    assert_eq!(runtime.source_bindings[&key].snapshot_path(), snapshot);

    let mut mismatch = request.clone();
    mismatch.source_path = root.join("another.wav").to_string_lossy().into_owned();
    assert!(
        runtime
            .ensure_source_binding(&mut mismatch)
            .unwrap_err()
            .starts_with("SourceSnapshotMismatch")
    );
    assert_eq!(runtime.source_bindings[&key].snapshot_path(), snapshot);

    let mut strategy_mismatch = request.clone();
    strategy_mismatch.strategy = whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1;
    assert!(
        runtime
            .ensure_source_binding(&mut strategy_mismatch)
            .unwrap_err()
            .starts_with("SourceSnapshotMismatch")
    );
    assert_eq!(runtime.source_bindings[&key].snapshot_path(), snapshot);

    runtime.snapshot_registry_root = Some(registry);
    let mut absent = unified_import_request(&root, &root.join("missing.wav"), 0xeca2);
    let absent_key = (absent.job_id, absent.generation.get());
    assert!(runtime.ensure_source_binding(&mut absent).is_err());
    assert!(!runtime.source_bindings.contains_key(&absent_key));
    assert_eq!(runtime.source_bindings.len(), 1);
    drop(runtime);
    fs::remove_dir_all(root).unwrap();
}
