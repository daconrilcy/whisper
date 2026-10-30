// Reconstructed interruption states. They characterize durable files, not power loss.
fn archive_characterization_root(label: &str) -> PathBuf {
    let base = std::env::var_os("ARCHIVE_CHARACTERIZATION_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let root = base.join(format!(
        "{label}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    root
}

fn archive_characterization_pending(root: &Path, job_id: JobId) -> (ArchiveStore, PathBuf) {
    let generation = Generation::first();
    let directory = generation_path(root, job_id, generation);
    fs::create_dir_all(&directory).unwrap();
    let pending = PendingRecord {
        strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
        version: 1,
        job_id,
        generation,
        source_path: "source.wav".into(),
        source_sha256: hex(&[0x51; 32]),
        source_samples: 160,
        model_path: "model.bin".into(),
        model_sha256: APPROVED_MODEL_SHA256.into(),
        model_size: APPROVED_MODEL_SIZE,
        qualification: None,
    };
    fs::write(
        directory.join("pending.json"),
        serde_json::to_vec(&pending).unwrap(),
    )
    .unwrap();
    let segment = SegmentRecord {
        sequence: 1,
        range: SourceRange::new(0, 160, 16_000).unwrap(),
        text: "characterized text".into(),
    };
    fs::write(
        directory.join("segments.jsonl"),
        [serde_json::to_vec(&segment).unwrap(), vec![b'\n']].concat(),
    )
    .unwrap();
    let publication = PublicationIntent {
        version: 1,
        operation_id: "characterization".into(),
        choice: DuplicateImportChoice::CreateNew,
        source_sha256: pending.source_sha256,
        source_samples: pending.source_samples,
        expected_active: Vec::new(),
        export_basename: "characterization".into(),
        internal_only: false,
        cleanup_id: None,
        cleanup_targets: Vec::new(),
    };
    let mut store = ArchiveStore::new(root);
    store.current = Some(ActiveArchive {
        job_id,
        generation,
        directory: directory.clone(),
        source_sha256: [0x51; 32],
        source_samples: 160,
        strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
        expected_sequence: 2,
        publication,
        segmented_source_manifest: None,
    });
    (store, directory)
}

#[test]
fn archive_characterization_stream_error_keeps_final_and_retry_replaces_it() {
    let root = archive_characterization_root("stream-error");
    let final_path = root.join("transcript.txt");
    let temporary = root.join("transcript.txt.tmp");
    fs::write(&final_path, b"old final").unwrap();
    let error = write_replace_streamed(&final_path, |writer| {
        writer.write_all(b"partial").unwrap();
        writer.flush().unwrap();
        Err("simulated stream error".into())
    })
    .unwrap_err();
    assert_eq!(error, "simulated stream error");
    assert_eq!(fs::read(&final_path).unwrap(), b"old final");
    assert_eq!(fs::read(&temporary).unwrap(), b"partial");
    write_replace_streamed(&final_path, |writer| {
        writer
            .write_all(b"retried")
            .map_err(|error| error.to_string())
    })
    .unwrap();
    assert_eq!(fs::read(&final_path).unwrap(), b"retried");
    assert!(!temporary.exists());
}

#[test]
fn archive_characterization_replace_obstacle_keeps_temporary_until_retry() {
    let root = archive_characterization_root("replace-obstacle");
    let final_path = root.join("transcript.txt");
    let temporary = root.join("transcript.txt.tmp");
    fs::create_dir(&final_path).unwrap();
    let error = write_replace_streamed(&final_path, |writer| {
        writer
            .write_all(b"staged")
            .map_err(|error| error.to_string())
    })
    .unwrap_err();
    assert!(error.starts_with("StorageUnavailable:"), "{error}");
    assert!(final_path.is_dir());
    assert_eq!(fs::read(&temporary).unwrap(), b"staged");
    fs::remove_dir(&final_path).unwrap();
    write_replace_streamed(&final_path, |writer| {
        writer
            .write_all(b"retried")
            .map_err(|error| error.to_string())
    })
    .unwrap();
    assert_eq!(fs::read(&final_path).unwrap(), b"retried");
    assert!(!temporary.exists());
}

#[test]
fn archive_characterization_txt_only_prepare_is_retried_before_publication() {
    let root = archive_characterization_root("txt-only");
    let job = JobId(0x5101);
    let generation = Generation::first();
    let (mut store, directory) = archive_characterization_pending(&root, job);
    let srt = directory.join("transcript.srt");
    fs::create_dir(&srt).unwrap();
    let error = store.prepare_publish(job, generation, 1).unwrap_err();
    assert!(error.starts_with("StorageUnavailable:"), "{error}");
    assert_eq!(
        fs::read(directory.join("transcript.txt")).unwrap(),
        b"characterized text"
    );
    assert!(directory.join("transcript.srt.tmp").is_file());
    assert!(!directory.join("manifest.json").exists());
    assert!(!directory.join("CURRENT").exists());
    assert!(!publication_index_path(&root).exists());
    fs::remove_dir(&srt).unwrap();
    store.prepare_publish(job, generation, 1).unwrap();
    assert!(directory.join("manifest.json").is_file());
    assert!(directory.join("r11-publication.json").is_file());
    assert!(directory.join("export.txt.pending").is_file());
    assert!(directory.join("export.srt.pending").is_file());
    assert!(!directory.join("CURRENT").exists());
    assert!(!publication_index_path(&root).exists());
}

#[test]
fn archive_characterization_missing_truncated_and_divergent_manifest_are_refused() {
    let root = archive_characterization_root("manifest");
    let job = JobId(0x5102);
    let generation = Generation::first();
    let (mut store, directory) = archive_characterization_pending(&root, job);
    store.prepare_publish(job, generation, 1).unwrap();
    let manifest_path = directory.join("manifest.json");
    let manifest = fs::read(&manifest_path).unwrap();
    fs::write(directory.join("CURRENT"), b"manifest.json\n").unwrap();
    let job_folder = format!("{:032x}", job.0);
    let scan = || scan_generation(&directory, &job_folder, "1");
    fs::remove_file(&manifest_path).unwrap();
    assert!(scan().is_err());
    fs::write(&manifest_path, b"{\"version\":").unwrap();
    assert!(scan().unwrap_err().starts_with("StorageCorrupt: manifest:"));
    fs::write(&manifest_path, &manifest).unwrap();
    let text = directory.join("transcript.txt");
    fs::write(&text, b"different artifact").unwrap();
    assert!(
        scan()
            .unwrap_err()
            .contains("artifact verification failed: transcript.txt")
    );
    fs::write(&text, b"characterized text").unwrap();
    assert!(scan().unwrap().unwrap().complete);
    assert_eq!(fs::read(&manifest_path).unwrap(), manifest);
    assert!(!publication_index_path(&root).exists());
}

#[test]
fn archive_characterization_intent_and_partial_staging_stay_unpublished_until_retry() {
    let root = archive_characterization_root("partial-staging");
    let job = JobId(0x5103);
    let generation = Generation::first();
    let (mut store, directory) = archive_characterization_pending(&root, job);
    let intent = store.current.as_ref().unwrap().publication.clone();
    fs::write(
        directory.join("r11-publication.json"),
        serde_json::to_vec(&intent).unwrap(),
    )
    .unwrap();
    assert_eq!(
        ArchiveStore::resolve_import_publication(&root, job, generation).unwrap_err(),
        "InvalidInput: publication is not complete"
    );
    assert!(!directory.join("export.txt.pending").exists());
    assert!(!directory.join("CURRENT").exists());
    assert!(!publication_index_path(&root).exists());
    fs::write(directory.join("export.txt.pending"), b"partial").unwrap();
    let error = ArchiveStore::resolve_import_publication(&root, job, generation).unwrap_err();
    assert_eq!(error, "InvalidInput: publication is not complete");
    assert_eq!(
        fs::read(directory.join("export.txt.pending")).unwrap(),
        b"partial"
    );
    assert!(!directory.join("export.srt.pending").exists());
    assert!(!directory.join("CURRENT").exists());
    assert!(!publication_index_path(&root).exists());
    store.prepare_publish(job, generation, 1).unwrap();
    assert_eq!(
        fs::read(directory.join("export.txt.pending")).unwrap(),
        b"characterized text"
    );
    assert!(directory.join("export.srt.pending").is_file());
    assert!(!directory.join("CURRENT").exists());
}

#[test]
fn archive_characterization_divergent_staged_srt_refused_and_retry_succeeds() {
    let root = archive_characterization_root("staged-srt");
    let job = JobId(0x5104);
    let generation = Generation::first();
    let (mut store, directory) = archive_characterization_pending(&root, job);
    store.prepare_publish(job, generation, 1).unwrap();
    let intent = store.current.as_ref().unwrap().publication.clone();
    let staged = directory.join("export.srt.pending");
    let original = fs::read(&staged).unwrap();
    fs::write(&staged, b"divergent").unwrap();
    let error = install_publication_exports(&root, &directory, &intent).unwrap_err();
    assert_eq!(
        error,
        "StorageCorrupt: staged srt export failed verification"
    );
    assert!(!root.join("exports/characterization.txt").exists());
    assert!(!root.join("exports/characterization.srt").exists());
    assert!(!directory.join("CURRENT").exists());
    assert!(!publication_index_path(&root).exists());
    fs::write(&staged, &original).unwrap();
    install_publication_exports(&root, &directory, &intent).unwrap();
    assert_eq!(
        fs::read(root.join("exports/characterization.srt")).unwrap(),
        original
    );
    assert!(!directory.join("CURRENT").exists());
    assert!(!publication_index_path(&root).exists());
}

#[test]
fn archive_characterization_index_rename_obstacle_preserves_temporary_and_retry() {
    let root = archive_characterization_root("index-obstacle");
    let index_path = publication_index_path(&root);
    fs::create_dir_all(&index_path).unwrap();
    let index = PublicationIndex {
        version: 1,
        records: Vec::new(),
    };
    assert!(write_publication_index(&root, &index).is_err());
    let temporary = index_path.with_extension("json.tmp");
    assert!(index_path.is_dir());
    assert_eq!(
        fs::read(&temporary).unwrap(),
        serde_json::to_vec(&index).unwrap()
    );
    fs::remove_dir(&index_path).unwrap();
    write_publication_index(&root, &index).unwrap();
    assert_eq!(read_publication_index(&root).unwrap(), index);
    assert!(!temporary.exists());
}

#[test]
fn archive_characterization_current_before_index_internal_resolution_is_repeatable() {
    let root = archive_characterization_root("current-before-index");
    let job = JobId(0x5105);
    let generation = Generation::first();
    let (mut store, directory) = archive_characterization_pending(&root, job);
    store.current.as_mut().unwrap().publication.internal_only = true;
    store.prepare_publish(job, generation, 1).unwrap();
    fs::write(directory.join("CURRENT"), b"manifest.json\n").unwrap();
    assert!(!publication_index_path(&root).exists());
    ArchiveStore::resolve_import_publication(&root, job, generation).unwrap();
    let first = fs::read(publication_index_path(&root)).unwrap();
    let first_index = read_publication_index(&root).unwrap();
    assert_eq!(first_index.records.len(), 1);
    assert_eq!(first_index.records[0].job_id, job);
    assert_eq!(first_index.records[0].success_ordinal, Some(1));
    ArchiveStore::resolve_import_publication(&root, job, generation).unwrap();
    assert_eq!(fs::read(publication_index_path(&root)).unwrap(), first);
    assert_eq!(read_publication_index(&root).unwrap(), first_index);
    assert_eq!(
        fs::read(directory.join("CURRENT")).unwrap(),
        b"manifest.json\n"
    );
    assert!(!root.join("exports").exists());
}

#[test]
fn archive_characterization_index_absent_truncated_and_temporary_retry() {
    let root = archive_characterization_root("index");
    let index_path = publication_index_path(&root);
    fs::create_dir_all(index_path.parent().unwrap()).unwrap();
    let temporary = index_path.with_extension("json.tmp");
    fs::write(&temporary, b"interrupted temporary").unwrap();
    assert!(
        load_or_migrate_publication_index(&root)
            .unwrap()
            .records
            .is_empty()
    );
    assert!(!index_path.exists());
    assert_eq!(fs::read(&temporary).unwrap(), b"interrupted temporary");
    fs::write(&index_path, b"{\"version\":").unwrap();
    let first = load_or_migrate_publication_index(&root).unwrap_err();
    let second = load_or_migrate_publication_index(&root).unwrap_err();
    assert!(
        first.starts_with("StorageCorrupt: publication index:"),
        "{first}"
    );
    assert_eq!(first, second);
    assert_eq!(fs::read(&index_path).unwrap(), b"{\"version\":");
    assert_eq!(fs::read(&temporary).unwrap(), b"interrupted temporary");
    let index = PublicationIndex {
        version: 1,
        records: Vec::new(),
    };
    write_publication_index(&root, &index).unwrap();
    assert_eq!(read_publication_index(&root).unwrap(), index);
    assert!(!temporary.exists());
    assert_eq!(load_or_migrate_publication_index(&root).unwrap(), index);
}
