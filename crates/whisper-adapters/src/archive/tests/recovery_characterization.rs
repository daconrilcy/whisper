#[test]
fn archive_characterization_pending_absent_truncated_and_divergent() {
    let root = archive_characterization_root("pending-states");
    let job = JobId(0x5401);
    let (_, directory) = archive_characterization_pending(&root, job);
    let pending_path = directory.join("pending.json");
    let pending_bytes = fs::read(&pending_path).unwrap();
    let pending: PendingRecord = serde_json::from_slice(&pending_bytes).unwrap();
    let journal_path = directory.join("segments.jsonl");
    let journal = fs::read(&journal_path).unwrap();
    fs::remove_file(&pending_path).unwrap();
    // A reconstructed pre-existing journal with absent pending takes the create path.
    assert_eq!(initialize_pending(&directory, &pending).unwrap(), 1);
    assert_eq!(fs::read(&pending_path).unwrap(), pending_bytes);
    assert_eq!(fs::read(&journal_path).unwrap(), journal);
    assert_eq!(initialize_pending(&directory, &pending).unwrap(), 2);
    fs::write(&pending_path, b"{\"version\":").unwrap();
    for _ in 0..2 {
        let error = initialize_pending(&directory, &pending).unwrap_err();
        assert!(
            error.starts_with("StorageCorrupt: pending record:"),
            "{error}"
        );
        assert_eq!(fs::read(&pending_path).unwrap(), b"{\"version\":");
        assert_eq!(fs::read(&journal_path).unwrap(), journal);
    }
    let mut divergent: serde_json::Value = serde_json::from_slice(&pending_bytes).unwrap();
    divergent["source_sha256"] = serde_json::json!("ab".repeat(32));
    let divergent = serde_json::to_vec(&divergent).unwrap();
    fs::write(&pending_path, &divergent).unwrap();
    for _ in 0..2 {
        let error = initialize_pending(&directory, &pending).unwrap_err();
        assert_eq!(
            error,
            "SourceChanged: existing pending generation has another identity"
        );
        assert_eq!(fs::read(&pending_path).unwrap(), divergent);
        assert_eq!(fs::read(&journal_path).unwrap(), journal);
    }
}

#[test]
fn archive_characterization_segment_journal_repairs_initial_and_large_partial_tail() {
    let root = archive_characterization_root("segment-tails");
    let path = root.join("segments.jsonl");
    fs::write(&path, b"{\"sequence\":").unwrap();
    assert!(read_segments(&path).unwrap().is_empty());
    assert!(fs::read(&path).unwrap().is_empty());
    assert!(read_segments(&path).unwrap().is_empty());
    let record = SegmentRecord {
        sequence: 1,
        range: SourceRange::new(0, 160, 16_000).unwrap(),
        text: "durable".into(),
    };
    let mut prefix = serde_json::to_vec(&record).unwrap();
    prefix.push(b'\n');
    let mut contents = prefix.clone();
    contents.extend(std::iter::repeat_n(b'x', 16_500));
    fs::write(&path, &contents).unwrap();
    for _ in 0..2 {
        assert_eq!(read_segments(&path).unwrap(), vec![record.clone()]);
        assert_eq!(fs::read(&path).unwrap(), prefix);
    }
}

#[test]
fn archive_characterization_complete_corrupt_middle_segment_is_not_repaired() {
    let root = archive_characterization_root("segment-corrupt-middle");
    let path = root.join("segments.jsonl");
    let record = SegmentRecord {
        sequence: 1,
        range: SourceRange::new(0, 160, 16_000).unwrap(),
        text: "durable".into(),
    };
    let mut contents = serde_json::to_vec(&record).unwrap();
    contents.extend_from_slice(b"\n{bad json}\n");
    contents.extend_from_slice(&serde_json::to_vec(&record).unwrap());
    contents.push(b'\n');
    fs::write(&path, &contents).unwrap();
    let first = read_segments(&path).unwrap_err();
    assert!(first.starts_with("StorageCorrupt: segment:"), "{first}");
    assert_eq!(fs::read(&path).unwrap(), contents);
    assert_eq!(read_segments(&path).unwrap_err(), first);
    assert_eq!(fs::read(&path).unwrap(), contents);
}
