// These states are reconstructed before publication, unless a test says otherwise.
fn archive_characterization_installed_exports(
    root: &Path,
    job: JobId,
) -> (PathBuf, PublicationIntent) {
    let (mut store, directory) = archive_characterization_pending(root, job);
    store.prepare_publish(job, Generation::first(), 1).unwrap();
    let intent = store.current.as_ref().unwrap().publication.clone();
    install_publication_exports(root, &directory, &intent).unwrap();
    (directory, intent)
}

#[test]
fn archive_characterization_ownership_versions_one_two_three_follow_existing_paths() {
    for version in [1, 2, 3] {
        let root = archive_characterization_root(&format!("ownership-v{version}"));
        let (directory, intent) = archive_characterization_installed_exports(&root, JobId(0x5301));
        let ownership_path = installed_export_ownership_path(&directory);
        let mut ownership: InstalledExportOwnership =
            serde_json::from_slice(&fs::read(&ownership_path).unwrap()).unwrap();
        let txt = root.join("exports/characterization.txt");
        let srt = root.join("exports/characterization.srt");
        let txt_id = persisted_file_identity(&txt).unwrap();
        let srt_id = persisted_file_identity(&srt).unwrap();
        assert!(same_file::is_same_file(&txt, directory.join("export.txt.pending")).unwrap());
        assert!(same_file::is_same_file(&srt, directory.join("export.srt.pending")).unwrap());
        ownership.version = version;
        if version == 2 {
            ownership.text_temporary_identity = None;
            ownership.srt_temporary_identity = None;
        }
        fs::write(&ownership_path, serde_json::to_vec(&ownership).unwrap()).unwrap();
        for _ in 0..2 {
            install_publication_exports(&root, &directory, &intent).unwrap();
            assert_eq!(persisted_file_identity(&txt).unwrap(), txt_id);
            assert_eq!(persisted_file_identity(&srt).unwrap(), srt_id);
            assert_eq!(fs::read(&txt).unwrap(), b"characterized text");
        }
        let actual: InstalledExportOwnership =
            serde_json::from_slice(&fs::read(&ownership_path).unwrap()).unwrap();
        assert_eq!(actual.version, if version == 2 { 3 } else { version });
        if version == 2 {
            assert_eq!(
                actual.text_temporary_identity.as_deref(),
                Some(txt_id.as_str())
            );
            assert_eq!(
                actual.srt_temporary_identity.as_deref(),
                Some(srt_id.as_str())
            );
        }
        assert!(!directory.join("CURRENT").exists());
        assert!(!publication_index_path(&root).exists());
    }
}

#[test]
fn archive_characterization_ownership_truncated_or_unknown_refuses_without_mutation() {
    let root = archive_characterization_root("ownership-invalid");
    let (directory, intent) = archive_characterization_installed_exports(&root, JobId(0x5302));
    let path = installed_export_ownership_path(&directory);
    let mut ownership: InstalledExportOwnership =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let txt = root.join("exports/characterization.txt");
    let srt = root.join("exports/characterization.srt");
    let txt_id = persisted_file_identity(&txt).unwrap();
    let srt_id = persisted_file_identity(&srt).unwrap();
    let txt_bytes = fs::read(&txt).unwrap();
    let srt_bytes = fs::read(&srt).unwrap();
    fs::write(&path, b"{\"version\":").unwrap();
    let error = install_publication_exports(&root, &directory, &intent).unwrap_err();
    assert!(
        error.starts_with("StorageCorrupt: export ownership record:"),
        "{error}"
    );
    assert_eq!(fs::read(&path).unwrap(), b"{\"version\":");
    ownership.version = 99;
    let unknown = serde_json::to_vec(&ownership).unwrap();
    fs::write(&path, &unknown).unwrap();
    for _ in 0..2 {
        let error = install_publication_exports(&root, &directory, &intent).unwrap_err();
        assert_eq!(
            error,
            "StorageCorrupt: unsupported export ownership version"
        );
        assert_eq!(fs::read(&path).unwrap(), unknown);
        assert_eq!(fs::read(&txt).unwrap(), txt_bytes);
        assert_eq!(fs::read(&srt).unwrap(), srt_bytes);
        assert_eq!(persisted_file_identity(&txt).unwrap(), txt_id);
        assert_eq!(persisted_file_identity(&srt).unwrap(), srt_id);
    }
    assert!(!directory.join("CURRENT").exists());
    assert!(!publication_index_path(&root).exists());
}

#[test]
fn archive_characterization_v3_final_link_before_identity_is_recovered() {
    let root = archive_characterization_root("ownership-before-identity");
    let (directory, intent) = archive_characterization_installed_exports(&root, JobId(0x5303));
    let path = installed_export_ownership_path(&directory);
    let mut ownership: InstalledExportOwnership =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    let txt = root.join("exports/characterization.txt");
    let prior_id = persisted_file_identity(&txt).unwrap();
    assert!(
        same_file::is_same_file(&txt, export_install_temporary_path(&directory, "txt")).unwrap()
    );
    ownership.text_file_identity = None;
    fs::write(&path, serde_json::to_vec(&ownership).unwrap()).unwrap();
    install_publication_exports(&root, &directory, &intent).unwrap();
    let recovered: InstalledExportOwnership =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert_eq!(
        recovered.text_file_identity.as_deref(),
        Some(prior_id.as_str())
    );
    assert_eq!(persisted_file_identity(&txt).unwrap(), prior_id);
    assert!(!directory.join("CURRENT").exists());
    assert!(!publication_index_path(&root).exists());
}

#[test]
fn archive_characterization_reconcile_partial_skips_byte_identical_third_party_then_retries() {
    let root = archive_characterization_root("reconcile-partial");
    let job = JobId(0x5304);
    let generation = Generation::first();
    let (directory, intent) = archive_characterization_installed_exports(&root, job);
    let txt = root.join("exports/characterization.txt");
    let srt = root.join("exports/characterization.srt");
    let saved_owned_srt = root.join("saved-owned.srt");
    let original_srt = fs::read(&srt).unwrap();
    let original_srt_id = persisted_file_identity(&srt).unwrap();
    fs::rename(&srt, &saved_owned_srt).unwrap();
    fs::write(&srt, &original_srt).unwrap();
    let third_party_id = persisted_file_identity(&srt).unwrap();
    assert_ne!(third_party_id, original_srt_id);
    let index = PublicationIndex {
        version: 1,
        records: vec![PublicationRecord {
            job_id: job,
            generation,
            source_sha256: intent.source_sha256.clone(),
            export_basename: Some(intent.export_basename.clone()),
            active: false,
            success_ordinal: Some(1),
        }],
    };
    write_publication_index(&root, &index).unwrap();
    fs::write(directory.join("CURRENT"), b"manifest.json\n").unwrap();
    reconcile_replaced_exports(&root, &index).unwrap();
    let suffix = format!("{:032x}-{}", job.0, generation.get());
    let backup_txt = root
        .join("transcriptions-remplacees")
        .join(format!("characterization-{suffix}.txt"));
    let backup_srt = root
        .join("transcriptions-remplacees")
        .join(format!("characterization-{suffix}.srt"));
    assert!(!txt.exists());
    assert_eq!(fs::read(&backup_txt).unwrap(), b"characterized text");
    assert!(!backup_srt.exists());
    assert_eq!(fs::read(&srt).unwrap(), original_srt);
    assert_eq!(persisted_file_identity(&srt).unwrap(), third_party_id);
    reconcile_replaced_exports(&root, &index).unwrap();
    assert_eq!(persisted_file_identity(&srt).unwrap(), third_party_id);
    assert_eq!(read_publication_index(&root).unwrap(), index);
    assert_eq!(
        fs::read(directory.join("CURRENT")).unwrap(),
        b"manifest.json\n"
    );
    // Explicit fixture transition: retain the third party, restore the known owned file.
    let retained_third_party = root.join("retained-third-party.srt");
    fs::rename(&srt, &retained_third_party).unwrap();
    fs::rename(&saved_owned_srt, &srt).unwrap();
    reconcile_replaced_exports(&root, &index).unwrap();
    assert!(!srt.exists());
    assert_eq!(fs::read(&backup_srt).unwrap(), original_srt);
    assert_eq!(
        persisted_file_identity(&backup_srt).unwrap(),
        original_srt_id
    );
    assert_eq!(
        persisted_file_identity(&retained_third_party).unwrap(),
        third_party_id
    );
    reconcile_replaced_exports(&root, &index).unwrap();
    assert_eq!(read_publication_index(&root).unwrap(), index);
}

#[test]
fn archive_characterization_residual_reservation_cleanup_refusal_and_retry() {
    let root = archive_characterization_root("reservation-cleanup");
    let job = JobId(0x5305);
    let generation = Generation::first();
    let index = PublicationIndex {
        version: 1,
        records: vec![PublicationRecord {
            job_id: job,
            generation,
            source_sha256: "51".repeat(32),
            export_basename: Some("characterization".into()),
            active: true,
            success_ordinal: Some(1),
        }],
    };
    write_publication_index(&root, &index).unwrap();
    let directory = generation_path(&root, job, generation);
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join("CURRENT"), b"manifest.json\n").unwrap();
    let path = publication_reservation_path(&root, job, generation);
    fs::create_dir_all(&path).unwrap();
    let error = ArchiveStore::clear_duplicate_reservation(&root, job, generation).unwrap_err();
    assert!(
        error.starts_with("StorageUnavailable: publication reservation cleanup:"),
        "{error}"
    );
    assert!(path.is_dir());
    assert_eq!(read_publication_index(&root).unwrap(), index);
    assert_eq!(
        fs::read(directory.join("CURRENT")).unwrap(),
        b"manifest.json\n"
    );
    fs::remove_dir(&path).unwrap();
    fs::write(&path, b"residual reservation").unwrap();
    ArchiveStore::clear_duplicate_reservation(&root, job, generation).unwrap();
    assert!(!path.exists());
    ArchiveStore::clear_duplicate_reservation(&root, job, generation).unwrap();
    assert!(!path.exists());
    assert_eq!(read_publication_index(&root).unwrap(), index);
    assert_eq!(
        fs::read(directory.join("CURRENT")).unwrap(),
        b"manifest.json\n"
    );
}

#[test]
fn archive_characterization_second_external_resolution_preserves_committed_publication() {
    let root = archive_characterization_root("resolution-repeat");
    let job = JobId(0x5306);
    let generation = Generation::first();
    let (mut store, directory) = archive_characterization_pending(&root, job);
    store.prepare_publish(job, generation, 1).unwrap();
    fs::write(directory.join("CURRENT"), b"manifest.json\n").unwrap();
    ArchiveStore::resolve_import_publication(&root, job, generation).unwrap();
    let index = fs::read(publication_index_path(&root)).unwrap();
    let txt = root.join("exports/characterization.txt");
    let srt = root.join("exports/characterization.srt");
    let txt_id = persisted_file_identity(&txt).unwrap();
    let srt_id = persisted_file_identity(&srt).unwrap();
    let error = ArchiveStore::resolve_import_publication(&root, job, generation).unwrap_err();
    assert_eq!(
        error,
        "DuplicateStateChanged: publication set changed; re-import to confirm"
    );
    assert_eq!(fs::read(publication_index_path(&root)).unwrap(), index);
    assert_eq!(persisted_file_identity(&txt).unwrap(), txt_id);
    assert_eq!(persisted_file_identity(&srt).unwrap(), srt_id);
    assert_eq!(fs::read(&txt).unwrap(), b"characterized text");
    assert_eq!(
        fs::read(directory.join("CURRENT")).unwrap(),
        b"manifest.json\n"
    );
}
