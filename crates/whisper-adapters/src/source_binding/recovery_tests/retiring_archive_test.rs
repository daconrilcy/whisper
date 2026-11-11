use super::super::*;
use super::{request, root, write_wav};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

fn write_complete_archive(
    destination: &Path,
    record: &SnapshotRecord,
) -> (PathBuf, Vec<(PathBuf, Vec<u8>)>) {
    let directory = destination
        .join("transcriptions")
        .join(format!("{:032x}", record.job_id.0))
        .join(record.generation.get().to_string());
    fs::create_dir_all(&directory).unwrap();

    let model = &whisper_core::model_registry::MODELS[0].artifact;
    let text = b"completed transcript";
    let srt = b"1\n00:00:00,000 --> 00:00:00,500\ncompleted transcript\n";
    let text_sha256 = format!("{:x}", Sha256::digest(text));
    let srt_sha256 = format!("{:x}", Sha256::digest(srt));
    fs::write(
        directory.join("pending.json"),
        serde_json::to_vec(&serde_json::json!({
            "version": 1,
            "job_id": record.job_id.0,
            "generation": record.generation.get(),
            "source_path": record.original_path,
            "source_sha256": hex_bytes(&record.source_sha256),
            "source_samples": record.source_samples,
            "model_path": model.file_name,
            "model_sha256": model.sha256,
            "model_size": model.size_bytes,
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(directory.join("transcript.txt"), text).unwrap();
    fs::write(directory.join("transcript.srt"), srt).unwrap();
    fs::write(
        directory.join("segments.jsonl"),
        format!(
            "{}\n",
            serde_json::json!({
                "sequence": 1,
                "range": {
                    "start_sample": 0,
                    "end_sample": record.source_samples,
                    "sample_rate_hz": 16_000,
                },
                "text": "completed transcript",
            })
        ),
    )
    .unwrap();
    fs::write(
        directory.join("manifest.json"),
        serde_json::to_vec(&serde_json::json!({
            "version": 1,
            "job_id": record.job_id.0,
            "generation": record.generation.get(),
            "source_sha256": hex_bytes(&record.source_sha256),
            "segments": 1,
            "text": { "path": "transcript.txt", "sha256": text_sha256, "bytes": text.len() },
            "srt": { "path": "transcript.srt", "sha256": srt_sha256, "bytes": srt.len() },
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(directory.join("CURRENT"), b"manifest.json\n").unwrap();

    let archive_files = [
        "pending.json",
        "transcript.txt",
        "transcript.srt",
        "segments.jsonl",
        "manifest.json",
        "CURRENT",
    ]
    .into_iter()
    .map(|name| {
        let path = directory.join(name);
        let bytes = fs::read(&path).unwrap();
        (path, bytes)
    })
    .collect();
    (directory, archive_files)
}

fn prepare_retiring_fixture(
    root: &Path,
    job_id: u128,
    corrupt_manifest: bool,
) -> (
    PathBuf,
    PathBuf,
    PathBuf,
    PathBuf,
    SnapshotRegistry,
    Vec<(PathBuf, Vec<u8>)>,
) {
    fs::create_dir_all(root).unwrap();
    let destination = root.join("destination");
    fs::create_dir_all(&destination).unwrap();
    let source = root.join("source.wav");
    write_wav(&source);
    let registry_root = root.join("settings/reprocessing-snapshots");
    let binding =
        SourceBinding::acquire(&request(&destination, &source, job_id), &registry_root).unwrap();
    let snapshot_path = binding.snapshot_path.clone();
    let record_path = binding.record_path.clone();
    let record = binding.record.clone();
    drop(binding);

    let (archive_directory, _) = write_complete_archive(&destination, &record);
    if corrupt_manifest {
        fs::write(archive_directory.join("manifest.json"), b"{").unwrap();
    }
    let archive_files = [
        "pending.json",
        "transcript.txt",
        "transcript.srt",
        "segments.jsonl",
        "manifest.json",
        "CURRENT",
    ]
    .into_iter()
    .map(|name| {
        let path = archive_directory.join(name);
        let bytes = fs::read(&path).unwrap();
        (path, bytes)
    })
    .collect();

    let snapshot = open_disposition_snapshot(&snapshot_path, &record).unwrap();
    let mut record_file =
        crate::windows_file_identity::VerifiedFile::open(&record_path, false).unwrap();
    let record_bytes = record_file.read_all().unwrap();
    let intent = RetirementIntent {
        snapshot: recovery_artifact(&snapshot_path, &snapshot, &record.source_sha256).unwrap(),
        record: recovery_artifact_from_file(&record_path, &mut record_file).unwrap(),
        source_record: Some(record.clone()),
        source_record_bytes: Some(record_bytes),
    };
    let registry = SnapshotRegistry::new(&registry_root);
    registry.begin_retirement(&record, intent).unwrap();
    drop(snapshot);
    drop(record_file);
    (
        destination,
        registry_root,
        snapshot_path,
        record_path,
        registry,
        archive_files,
    )
}

#[test]
fn explicit_recovery_retires_retiring_snapshot_for_complete_archive_and_protects_diagnostic() {
    let root_path = root();
    let (destination, registry_root, snapshot_path, record_path, registry, archive_before) =
        prepare_retiring_fixture(&root_path, 1201, false);
    let record: SnapshotRecord = serde_json::from_slice(&fs::read(&record_path).unwrap()).unwrap();
    let archived = crate::archive::ArchiveStore::scan(&destination).unwrap();
    assert!(archived.iter().any(|entry| {
        entry.job_id == record.job_id
            && entry.generation == record.generation
            && entry.complete
            && entry.diagnostic.is_none()
            && entry.source_sha256 == hex_bytes(&record.source_sha256)
    }));

    let inspected = SourceBinding::inspect_recovery_candidates(&registry_root, &[]).unwrap();
    assert_eq!(inspected.len(), 1);
    assert!(inspected[0].recoverable, "{:?}", inspected[0].reason);
    let (freed, remaining) = SourceBinding::recover_abandoned(&registry_root, &[]).unwrap();

    assert_eq!(freed, 1);
    assert!(remaining.is_empty());
    assert!(!snapshot_path.exists());
    assert!(!record_path.exists());
    assert!(registry.read_entries().unwrap().is_empty());
    for (path, expected) in archive_before {
        assert_eq!(fs::read(path).unwrap(), expected);
    }
    fs::remove_dir_all(root_path).unwrap();

    let diagnostic_root = root();
    let (destination, registry_root, snapshot_path, record_path, registry, archive_before) =
        prepare_retiring_fixture(&diagnostic_root, 1202, true);
    let diagnostic_archive = crate::archive::ArchiveStore::scan(&destination).unwrap();
    assert!(
        diagnostic_archive
            .iter()
            .any(|entry| { entry.job_id == JobId(1202) && entry.diagnostic.is_some() })
    );

    let inspected = SourceBinding::inspect_recovery_candidates(&registry_root, &[]).unwrap();
    assert_eq!(inspected.len(), 1);
    assert!(!inspected[0].recoverable);
    let (freed, remaining) = SourceBinding::recover_abandoned(&registry_root, &[]).unwrap();
    assert_eq!(freed, 0);
    assert_eq!(remaining.len(), 1);
    assert!(snapshot_path.is_file());
    assert!(record_path.is_file());
    assert_eq!(registry.read_entries().unwrap().len(), 1);
    for (path, expected) in archive_before {
        assert_eq!(fs::read(path).unwrap(), expected);
    }
    fs::remove_dir_all(diagnostic_root).unwrap();
}
