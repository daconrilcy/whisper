#[test]
fn archive_characterization_truncated_cleanup_plan_is_refused_without_disposition() {
    let root = archive_characterization_root("cleanup-truncated");
    let plans = cleanup_plans_path(&root);
    fs::create_dir_all(&plans).unwrap();
    let path = cleanup_plan_path(&root, "truncated");
    let bytes = b"{\"version\":1,\"targets\":";
    fs::write(&path, bytes).unwrap();
    let sentinel = root.join("old-treatment-sentinel");
    fs::write(&sentinel, b"preserve old treatment").unwrap();
    for _ in 0..2 {
        let discovery = ArchiveStore::replacement_cleanups(&root).unwrap_err();
        assert!(
            discovery.starts_with("StorageCorrupt: reprise JSON:"),
            "{discovery}"
        );
        let resume = ArchiveStore::resume_replacement_cleanup(&root, "truncated").unwrap_err();
        assert!(
            resume.starts_with("StorageCorrupt: reprise de nettoyage:"),
            "{resume}"
        );
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_eq!(fs::read(&sentinel).unwrap(), b"preserve old treatment");
        assert!(!publication_index_path(&root).exists());
    }
}
