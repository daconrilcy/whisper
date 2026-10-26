use std::{
    fs,
    io::Write,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use whisper_adapters::source_binding::SourceBinding;
use whisper_core::{Generation, JobConfig, JobId, ports::ImportRequest};

fn root() -> PathBuf {
    static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);
    let base = PathBuf::from(std::env::var_os("PARAKEET_L2_ARCHIVE_ROOT").expect("campaign root"));
    let path = base.join(format!(
        "source-binding-test-{}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

fn wav(path: &std::path::Path, frames: u32) {
    let data_bytes = frames * 2;
    let mut file = fs::File::create(path).unwrap();
    file.write_all(b"RIFF").unwrap();
    file.write_all(&(36 + data_bytes).to_le_bytes()).unwrap();
    file.write_all(b"WAVEfmt ").unwrap();
    file.write_all(&16_u32.to_le_bytes()).unwrap();
    file.write_all(&1_u16.to_le_bytes()).unwrap();
    file.write_all(&1_u16.to_le_bytes()).unwrap();
    file.write_all(&16_000_u32.to_le_bytes()).unwrap();
    file.write_all(&32_000_u32.to_le_bytes()).unwrap();
    file.write_all(&2_u16.to_le_bytes()).unwrap();
    file.write_all(&16_u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&data_bytes.to_le_bytes()).unwrap();
    file.write_all(&vec![0_u8; data_bytes as usize]).unwrap();
}

fn request(root: &std::path::Path, source: &std::path::Path, id: u128) -> ImportRequest {
    ImportRequest {
        strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
        job_id: JobId(id),
        generation: Generation::first(),
        source_path: source.to_string_lossy().into_owned(),
        source_sha256: None,
        source_samples: None,
        model_path: "model.bin".into(),
        model_sha256: [0; 32],
        destination: root.to_string_lossy().into_owned(),
        config: JobConfig {
            language: whisper_core::LanguageChoice::Automatic,
            compute: whisper_core::ComputeChoice::Cpu,
        },
    }
}

fn registry(root: &std::path::Path) -> PathBuf {
    root.join("settings/reprocessing-snapshots")
}

#[test]
fn captures_once_reuses_snapshot_and_retire_releases_capacity() {
    let root = root();
    let first = root.join("original.wav");
    let renamed = root.join("renamed-copy.wav");
    wav(&first, 16_000);
    fs::copy(&first, &renamed).unwrap();
    let first_request = request(&root, &first, 1);
    let registry_root = registry(&root);
    let first_binding = SourceBinding::acquire(&first_request, &registry_root).unwrap();
    assert_eq!(
        fs::read(first_binding.snapshot_path()).unwrap(),
        fs::read(&first).unwrap()
    );
    assert_eq!(first_binding.source_samples(), 16_000);
    let execution = first_binding.apply_to_request(&first_request).unwrap();
    assert_eq!(
        execution.source_path,
        first_binding.snapshot_path().to_string_lossy().into_owned()
    );
    assert_eq!(execution.source_sha256, Some(first_binding.source_sha256()));
    let snapshot_path = first_binding.snapshot_path().to_path_buf();
    #[cfg(windows)]
    {
        assert!(
            std::fs::OpenOptions::new()
                .write(true)
                .open(&snapshot_path)
                .is_err()
        );
        assert!(fs::rename(&snapshot_path, root.join("renamed-while-bound.wav")).is_err());
    }

    let duplicate_request = request(&root, &first, 1);
    let reopened = SourceBinding::acquire(&duplicate_request, &registry_root).unwrap();
    assert_eq!(reopened.source_sha256(), first_binding.source_sha256());
    assert_eq!(reopened.snapshot_path(), first_binding.snapshot_path());
    drop(reopened);

    let renamed_request = request(&root, &renamed, 2);
    let renamed_binding = SourceBinding::acquire(&renamed_request, &registry_root).unwrap();
    assert_eq!(
        renamed_binding.source_sha256(),
        first_binding.source_sha256()
    );

    let third = root.join("third.wav");
    wav(&third, 16_000);
    let third_result = SourceBinding::acquire(&request(&root, &third, 3), &registry_root);
    assert!(matches!(third_result, Err(error) if error.contains("SourceSnapshotCapacity")));

    first_binding.retire().unwrap();
    let third_binding = SourceBinding::acquire(&request(&root, &third, 3), &registry_root).unwrap();
    assert_eq!(third_binding.source_samples(), 16_000);
    renamed_binding.retire().unwrap();
    third_binding.retire().unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rebinding_operation_moves_only_its_owned_snapshot_and_record() {
    let root = root();
    let source = root.join("audio.wav");
    wav(&source, 8_000);
    let registry_root = registry(&root);
    let binding = SourceBinding::acquire(&request(&root, &source, 11), &registry_root).unwrap();
    let rebound = binding.rebind(JobId(12), Generation::first()).unwrap();
    let rebound_request = request(&root, &source, 12);
    let reopened = SourceBinding::acquire(&rebound_request, &registry_root).unwrap();
    assert_eq!(reopened.source_sha256(), rebound.source_sha256());
    assert_eq!(reopened.snapshot_path(), rebound.snapshot_path());
    drop(rebound);
    reopened.retire().unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn conflicting_bound_snapshot_path_is_refused_and_keeps_both_slots() {
    let root = root();
    let registry_root = registry(&root);
    let destination_a = root.join("destination-a");
    let destination_b = root.join("destination-b");
    let destination_c = root.join("destination-c");
    for destination in [&destination_a, &destination_b, &destination_c] {
        fs::create_dir_all(destination).unwrap();
    }
    let source_a = root.join("a.wav");
    let source_b = root.join("b.wav");
    let source_c = root.join("c.wav");
    wav(&source_a, 8_000);
    wav(&source_b, 8_000);
    wav(&source_c, 8_000);
    let request_a = request(&destination_a, &source_a, 71);
    let request_b = request(&destination_b, &source_b, 72);
    let binding_a = SourceBinding::acquire(&request_a, &registry_root).unwrap();
    let binding_b = SourceBinding::acquire(&request_b, &registry_root).unwrap();
    drop(binding_a);
    drop(binding_b);

    let registry_file = registry_root.join("reprocessing-snapshots.json");
    let original_registry: serde_json::Value =
        serde_json::from_slice(&fs::read(&registry_file).unwrap()).unwrap();
    let mut entries: Vec<serde_json::Value> =
        serde_json::from_value(original_registry.clone()).unwrap();
    let entry_a = entries
        .iter_mut()
        .find(|entry| entry["job_id"] == serde_json::json!(71_u128))
        .unwrap();
    entry_a["snapshot_path"] = serde_json::json!(
        destination_a
            .join(".whisper-source-snapshots/foreign.snapshot")
            .to_string_lossy()
    );
    fs::write(&registry_file, serde_json::to_vec(&entries).unwrap()).unwrap();
    let divergent_registry: serde_json::Value =
        serde_json::from_slice(&fs::read(&registry_file).unwrap()).unwrap();

    assert!(matches!(
        SourceBinding::acquire(&request_a, &registry_root),
        Err(error) if error.contains("SourceSnapshotRegistryAmbiguous")
    ));
    let after_refusal: serde_json::Value =
        serde_json::from_slice(&fs::read(&registry_file).unwrap()).unwrap();
    assert_eq!(after_refusal, divergent_registry);
    assert!(matches!(
        SourceBinding::acquire(&request(&destination_c, &source_c, 73), &registry_root),
        Err(error) if error.contains("SourceSnapshotCapacity")
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn rebind_rejects_operation_identity_owned_in_another_destination() {
    let root = root();
    let registry_root = registry(&root);
    let destination_a = root.join("destination-a");
    let destination_b = root.join("destination-b");
    let destination_c = root.join("destination-c");
    for destination in [&destination_a, &destination_b, &destination_c] {
        fs::create_dir_all(destination).unwrap();
    }
    let source_a = root.join("a.wav");
    let source_b = root.join("b.wav");
    let source_c = root.join("c.wav");
    wav(&source_a, 8_000);
    wav(&source_b, 8_000);
    wav(&source_c, 8_000);
    let request_a = request(&destination_a, &source_a, 81);
    let binding_a = SourceBinding::acquire(&request_a, &registry_root).unwrap();
    let binding_b =
        SourceBinding::acquire(&request(&destination_b, &source_b, 82), &registry_root).unwrap();
    let registry_file = registry_root.join("reprocessing-snapshots.json");
    let before_rebind = fs::read(&registry_file).unwrap();

    assert!(matches!(
        binding_a.rebind(JobId(82), Generation::first()),
        Err(error) if error.contains("target operation identity already owns a slot")
    ));
    assert_eq!(fs::read(&registry_file).unwrap(), before_rebind);
    assert!(matches!(
        SourceBinding::acquire(&request(&destination_c, &source_c, 83), &registry_root),
        Err(error) if error.contains("SourceSnapshotCapacity")
    ));

    let reopened_a = SourceBinding::acquire(&request_a, &registry_root).unwrap();
    reopened_a.retire().unwrap();
    binding_b.retire().unwrap();
    let binding_c =
        SourceBinding::acquire(&request(&destination_c, &source_c, 83), &registry_root).unwrap();
    binding_c.retire().unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn explicit_recovery_releases_only_verified_abandoned_slots_when_destination_is_empty() {
    let root = root();
    let registry_root = registry(&root);
    let destination = root.join("destination");
    fs::create_dir_all(&destination).unwrap();
    let source_a = root.join("a.wav");
    let source_b = root.join("b.wav");
    let source_c = root.join("c.wav");
    for source in [&source_a, &source_b, &source_c] {
        wav(source, 8_000);
    }
    let first =
        SourceBinding::acquire(&request(&destination, &source_a, 901), &registry_root).unwrap();
    let second =
        SourceBinding::acquire(&request(&destination, &source_b, 902), &registry_root).unwrap();
    let first_snapshot = first.snapshot_path().to_path_buf();
    let first_record = first_snapshot.with_extension("json");
    let second_snapshot = second.snapshot_path().to_path_buf();
    let second_record = second_snapshot.with_extension("json");
    drop(first);
    drop(second);

    // Reproduce a previously interrupted release plus one intact abandoned
    // snapshot. All fixture mutations are contained under the campaign root.
    fs::remove_file(&first_snapshot).unwrap();
    fs::remove_file(&first_record).unwrap();
    assert!(!destination.join("transcriptions").exists());

    let inspected = SourceBinding::inspect_recovery_candidates(&registry_root, &[]).unwrap();
    assert_eq!(inspected.len(), 2);
    assert!(inspected.iter().all(|candidate| candidate.recoverable));
    assert!(second_snapshot.is_file() && second_record.is_file());
    assert!(!destination.join("transcriptions").exists());

    let (freed, remaining) = SourceBinding::recover_abandoned(&registry_root, &[]).unwrap();
    assert_eq!(freed, 2);
    assert!(remaining.is_empty());
    assert!(!first_snapshot.exists() && !first_record.exists());
    assert!(!second_snapshot.exists() && !second_record.exists());
    assert!(!destination.join("transcriptions").exists());

    let new_binding =
        SourceBinding::acquire(&request(&destination, &source_c, 903), &registry_root)
            .expect("explicit recovery must free a slot for a fresh destination import");
    new_binding.retire().unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn explicit_recovery_preserves_active_and_incomplete_bindings() {
    let root = root();
    let registry_root = registry(&root);
    let active_destination = root.join("active-destination");
    let incomplete_destination = root.join("incomplete-destination");
    fs::create_dir_all(&active_destination).unwrap();
    fs::create_dir_all(&incomplete_destination).unwrap();
    let active_source = root.join("active.wav");
    let incomplete_source = root.join("incomplete.wav");
    wav(&active_source, 8_000);
    wav(&incomplete_source, 8_000);
    let active = SourceBinding::acquire(
        &request(&active_destination, &active_source, 911),
        &registry_root,
    )
    .unwrap();
    let incomplete = SourceBinding::acquire(
        &request(&incomplete_destination, &incomplete_source, 912),
        &registry_root,
    )
    .unwrap();
    let active_snapshot = active.snapshot_path().to_path_buf();
    let incomplete_snapshot = incomplete.snapshot_path().to_path_buf();
    let incomplete_record = incomplete_snapshot.with_extension("json");
    drop(incomplete);
    fs::remove_file(&incomplete_snapshot).unwrap();

    let inspected = SourceBinding::inspect_recovery_candidates(
        &registry_root,
        &[(JobId(911), Generation::first().get())],
    )
    .unwrap();
    assert_eq!(inspected.len(), 2);
    assert!(inspected.iter().all(|candidate| !candidate.recoverable));
    assert!(
        inspected
            .iter()
            .any(|candidate| candidate.job_id == JobId(911)
                && candidate.reason.as_deref() == Some("Traitement actif dans cette session"))
    );
    assert!(inspected.iter().any(|candidate| {
        candidate.job_id == JobId(912)
            && candidate
                .reason
                .as_deref()
                .is_some_and(|reason| reason.contains("état incomplet"))
    }));
    let (freed, remaining) =
        SourceBinding::recover_abandoned(&registry_root, &[(JobId(911), 1)]).unwrap();
    assert_eq!(freed, 0);
    assert_eq!(remaining.len(), 2);
    assert!(active_snapshot.is_file());
    assert!(incomplete_record.is_file());
    assert!(!incomplete_snapshot.exists());
    let third_destination = root.join("third-destination");
    fs::create_dir_all(&third_destination).unwrap();
    assert!(matches!(
        SourceBinding::acquire(
            &request(&third_destination, &active_source, 913),
            &registry_root
        ),
        Err(error) if error.contains("SourceSnapshotCapacity")
    ));
    drop(active);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn explicit_recovery_keeps_a_snapshot_referenced_by_the_durable_queue() {
    let root = root();
    let registry_root = registry(&root);
    let destination = root.join("destination");
    fs::create_dir_all(&destination).unwrap();
    let source = root.join("queued.wav");
    wav(&source, 8_000);
    let mut import = request(&destination, &source, 921);
    let binding = SourceBinding::acquire(&import, &registry_root).unwrap();
    import.source_sha256 = Some(binding.source_sha256());
    import.source_samples = Some(binding.source_samples());
    let snapshot = binding.snapshot_path().to_path_buf();
    drop(binding);
    whisper_adapters::queue_store::QueueStore::open(&destination)
        .unwrap()
        .enqueue(import)
        .unwrap();

    let inspected = SourceBinding::inspect_recovery_candidates(&registry_root, &[]).unwrap();
    assert_eq!(inspected.len(), 1);
    assert!(!inspected[0].recoverable);
    assert!(
        inspected[0]
            .reason
            .as_deref()
            .is_some_and(|reason| reason.contains("file durable"))
    );
    let (freed, remaining) = SourceBinding::recover_abandoned(&registry_root, &[]).unwrap();
    assert_eq!(freed, 0);
    assert_eq!(remaining.len(), 1);
    assert!(snapshot.is_file());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn explicit_recovery_keeps_a_snapshot_with_a_publication_reservation() {
    let root = root();
    let registry_root = registry(&root);
    let destination = root.join("destination");
    fs::create_dir_all(&destination).unwrap();
    let source = root.join("reserved.wav");
    wav(&source, 8_000);
    let import = request(&destination, &source, 922);
    let binding = SourceBinding::acquire(&import, &registry_root).unwrap();
    let snapshot = binding.snapshot_path().to_path_buf();
    drop(binding);

    whisper_adapters::archive::ArchiveStore::reserve_create_only_new_import(
        &destination,
        &import,
        "ab".repeat(32).as_str(),
    )
    .unwrap();
    assert!(!destination.join("transcriptions").exists());

    let inspected = SourceBinding::inspect_recovery_candidates(&registry_root, &[]).unwrap();
    assert_eq!(inspected.len(), 1);
    assert!(!inspected[0].recoverable);
    assert!(
        inspected[0]
            .reason
            .as_deref()
            .is_some_and(|reason| reason.contains("Réservation de publication"))
    );
    let (freed, remaining) = SourceBinding::recover_abandoned(&registry_root, &[]).unwrap();
    assert_eq!(freed, 0);
    assert_eq!(remaining.len(), 1);
    assert!(snapshot.is_file());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn explicit_recovery_preserves_binding_when_another_generation_has_corrupt_pending() {
    let root = root();
    let registry_root = registry(&root);
    let destination = root.join("destination");
    fs::create_dir_all(&destination).unwrap();
    let source = root.join("pending.wav");
    wav(&source, 8_000);
    let request = request(&destination, &source, 923);
    let binding = SourceBinding::acquire(&request, &registry_root).unwrap();
    let snapshot = binding.snapshot_path().to_path_buf();
    let record = snapshot.with_extension("json");
    drop(binding);

    let corrupt_generation = destination
        .join("transcriptions")
        .join(format!("{:032x}", request.job_id.0))
        .join("2");
    fs::create_dir_all(&corrupt_generation).unwrap();
    let pending = corrupt_generation.join("pending.json");
    fs::write(&pending, b"{ malformed pending generation two").unwrap();

    let inspected = SourceBinding::inspect_recovery_candidates(&registry_root, &[]).unwrap();
    assert_eq!(inspected.len(), 1);
    assert!(!inspected[0].recoverable);
    assert!(
        inspected[0]
            .reason
            .as_deref()
            .is_some_and(|reason| reason.contains("Archive ou pending"))
    );

    let (freed, remaining) = SourceBinding::recover_abandoned(&registry_root, &[]).unwrap();
    assert_eq!(freed, 0);
    assert_eq!(remaining.len(), 1);
    assert!(!remaining[0].recoverable);
    assert!(snapshot.is_file());
    assert!(record.is_file());
    assert_eq!(
        fs::read(&pending).unwrap(),
        b"{ malformed pending generation two"
    );
    assert_eq!(
        serde_json::from_slice::<Vec<serde_json::Value>>(
            &fs::read(registry_root.join("reprocessing-snapshots.json")).unwrap()
        )
        .unwrap()
        .len(),
        1
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn durable_global_capacity_spans_destinations_and_restarts() {
    let root = root();
    let registry_root = registry(&root);
    let source_a = root.join("a.wav");
    let source_b = root.join("b.wav");
    let source_c = root.join("c.wav");
    wav(&source_a, 8_000);
    wav(&source_b, 8_000);
    wav(&source_c, 8_000);
    let destination_a = root.join("destination-a");
    let destination_b = root.join("destination-b");
    let destination_c = root.join("destination-c");
    fs::create_dir_all(&destination_a).unwrap();
    fs::create_dir_all(&destination_b).unwrap();
    fs::create_dir_all(&destination_c).unwrap();
    let mut request_a = request(&destination_a, &source_a, 31);
    let mut request_b = request(&destination_b, &source_b, 32);
    let binding_a = SourceBinding::acquire(&request_a, &registry_root).unwrap();
    let binding_b = SourceBinding::acquire(&request_b, &registry_root).unwrap();
    request_a.source_sha256 = Some(binding_a.source_sha256());
    request_a.source_samples = Some(binding_a.source_samples());
    request_b.source_sha256 = Some(binding_b.source_sha256());
    request_b.source_samples = Some(binding_b.source_samples());

    // Dropping the process-local owners models restart; durable slots remain occupied.
    drop(binding_a);
    drop(binding_b);
    let request_c = request(&destination_c, &source_c, 33);
    assert!(matches!(
        SourceBinding::acquire(&request_c, &registry_root),
        Err(error) if error.contains("SourceSnapshotCapacity")
    ));

    let reopened_a = SourceBinding::acquire(&request_a, &registry_root).unwrap();
    let reopened_b = SourceBinding::acquire(&request_b, &registry_root).unwrap();
    reopened_a.retire().unwrap();
    let binding_c = SourceBinding::acquire(&request_c, &registry_root).unwrap();
    assert_eq!(binding_c.source_samples(), 8_000);
    reopened_b.retire().unwrap();
    binding_c.retire().unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn same_size_snapshot_mutation_is_rejected_and_keeps_its_global_slot() {
    let root = root();
    let registry_root = registry(&root);
    let destination_a = root.join("destination-a");
    let destination_b = root.join("destination-b");
    let destination_c = root.join("destination-c");
    fs::create_dir_all(&destination_a).unwrap();
    fs::create_dir_all(&destination_b).unwrap();
    fs::create_dir_all(&destination_c).unwrap();
    let source_a = root.join("a.wav");
    let source_b = root.join("b.wav");
    let source_c = root.join("c.wav");
    wav(&source_a, 8_000);
    wav(&source_b, 8_000);
    wav(&source_c, 8_000);
    let request_a = request(&destination_a, &source_a, 51);
    let binding_a = SourceBinding::acquire(&request_a, &registry_root).unwrap();
    let snapshot = binding_a.snapshot_path().to_path_buf();
    let recorded_len = fs::metadata(&snapshot).unwrap().len();
    drop(binding_a);

    let mut changed = fs::read(&snapshot).unwrap();
    let last = changed.last_mut().unwrap();
    *last ^= 0x01;
    fs::write(&snapshot, &changed).unwrap();
    assert_eq!(fs::metadata(&snapshot).unwrap().len(), recorded_len);
    assert!(matches!(
        SourceBinding::reopen_for_identity(
            &registry_root,
            &destination_a,
            JobId(51),
            Generation::first()
        ),
        Err(error) if error.contains("SourceSnapshotCorrupt")
    ));

    let binding_b =
        SourceBinding::acquire(&request(&destination_b, &source_b, 52), &registry_root).unwrap();
    drop(binding_b);
    assert!(matches!(
        SourceBinding::acquire(&request(&destination_c, &source_c, 53), &registry_root),
        Err(error) if error.contains("SourceSnapshotCapacity")
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn interrupted_empty_reservation_is_not_reused_after_restart() {
    let root = root();
    let registry_root = registry(&root);
    let destination_a = root.join("destination-a");
    let destination_b = root.join("destination-b");
    let destination_c = root.join("destination-c");
    for destination in [&destination_a, &destination_b, &destination_c] {
        fs::create_dir_all(destination).unwrap();
    }
    let canonical_a = fs::canonicalize(&destination_a).unwrap();
    fs::create_dir_all(&registry_root).unwrap();
    fs::write(
        registry_root.join("reprocessing-snapshots.json"),
        serde_json::to_vec(&vec![serde_json::json!({
            "job_id": 61_u128,
            "generation": 1,
            "destination": canonical_a.to_string_lossy(),
            "status": "Reserved",
            "snapshot_path": null
        })])
        .unwrap(),
    )
    .unwrap();
    let source_a = root.join("a.wav");
    let source_b = root.join("b.wav");
    let source_c = root.join("c.wav");
    wav(&source_a, 8_000);
    wav(&source_b, 8_000);
    wav(&source_c, 8_000);
    assert!(matches!(
        SourceBinding::acquire(&request(&destination_a, &source_a, 61), &registry_root),
        Err(error) if error.contains("SourceSnapshotOrphaned")
    ));
    let binding_b =
        SourceBinding::acquire(&request(&destination_b, &source_b, 62), &registry_root).unwrap();
    drop(binding_b);
    assert!(matches!(
        SourceBinding::acquire(&request(&destination_c, &source_c, 63), &registry_root),
        Err(error) if error.contains("SourceSnapshotCapacity")
    ));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn malformed_global_registry_fails_closed() {
    let root = root();
    let registry_root = registry(&root);
    fs::create_dir_all(&registry_root).unwrap();
    let registry_file = registry_root.join("reprocessing-snapshots.json");
    fs::write(&registry_file, b"").unwrap();
    let source = root.join("audio.wav");
    wav(&source, 8_000);
    assert!(matches!(
        SourceBinding::acquire(&request(&root, &source, 41), &registry_root),
        Err(error) if error.contains("SourceSnapshotRegistryCorrupt")
    ));
    fs::write(&registry_file, b"{").unwrap();
    assert!(matches!(
        SourceBinding::acquire(&request(&root, &source, 42), &registry_root),
        Err(error) if error.contains("SourceSnapshotRegistryCorrupt")
    ));
    fs::remove_dir_all(root).unwrap();
}
