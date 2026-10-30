use std::time::{SystemTime, UNIX_EPOCH};
use whisper_adapters::{archive::LiveArchive, staging::PcmStaging};
use whisper_core::{
    ComputeChoice, Generation, JobConfig, JobId, LanguageChoice, LiveRequest, SegmentId,
    SourceRange, ports::WorkerSegment,
};

fn test_root() -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "whisper-live-archive-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

#[test]
fn attempt_journal_reserves_monotonically_and_enforces_transitions() {
    let root = test_root();
    let request = LiveRequest {
        job_id: JobId(0x104),
        generation: Generation::first(),
        model_path: "model.bin".into(),
        model_sha256: [8; 32],
        destination: root.to_string_lossy().into_owned(),
        group_offset_samples: 0,
        auto_stop_after_speech_samples: None,
        microphone: Default::default(),
        microphone_gain_db: 0,
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Auto,
        },
    };
    let mut archive = LiveArchive::prepare(&root, request).unwrap();
    let first = archive.reserve_attempt().unwrap();
    assert_eq!(first, 1);
    assert!(archive.reserve_attempt().is_err());
    archive.mark_attempt_started(first).unwrap();
    archive.retire_attempt(first).unwrap();
    let second = archive.reserve_attempt().unwrap();
    assert_eq!(second, 2);
    let journal = root
        .join("transcriptions")
        .join(format!("{:032x}", 0x104u128))
        .join("live")
        .join("passage-1.attempts.jsonl");
    let records: Vec<serde_json::Value> = std::fs::read_to_string(journal)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(records.len(), 4);
    assert_eq!(records[0]["payload"]["state"], "reserved");
    assert_eq!(records[1]["payload"]["state"], "started");
    assert_eq!(records[2]["payload"]["state"], "retired");
    assert_eq!(records[3]["payload"]["attempt"], 2);
    assert!(
        records
            .iter()
            .all(|record| record["sha256"].as_str().is_some())
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_live_group_has_one_archive_writer_at_a_time() {
    let root = test_root();
    let request = LiveRequest {
        job_id: JobId(0x105),
        generation: Generation::first(),
        model_path: "model.bin".into(),
        model_sha256: [9; 32],
        destination: root.to_string_lossy().into_owned(),
        group_offset_samples: 0,
        auto_stop_after_speech_samples: None,
        microphone: Default::default(),
        microphone_gain_db: 0,
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        },
    };
    let archive = LiveArchive::prepare(&root, request.clone()).unwrap();
    let conflict = LiveArchive::prepare(&root, request);
    assert!(matches!(
        conflict,
        Err(error) if error.contains("another live archive writer owns the passage")
    ));
    drop(archive);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn next_passage_refuses_durable_pcm_without_verified_coverage() {
    let root = test_root();
    let request = LiveRequest {
        job_id: JobId(0x106),
        generation: Generation::first(),
        model_path: "model.bin".into(),
        model_sha256: [9; 32],
        destination: root.to_string_lossy().into_owned(),
        group_offset_samples: 0,
        auto_stop_after_speech_samples: None,
        microphone: Default::default(),
        microphone_gain_db: 0,
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        },
    };
    let archive = LiveArchive::prepare(&root, request.clone()).unwrap();
    let live_dir = root
        .join("transcriptions")
        .join(format!("{:032x}", request.job_id.0))
        .join("live");
    let mut staging = PcmStaging::create(live_dir.join("passage-1.pcm")).unwrap();
    for _ in 0..25 {
        staging.append_frame(&[0; 320]).unwrap();
    }
    assert_eq!(staging.durable_samples(), 8_000);
    drop(staging);
    drop(archive);

    let mut next = request;
    next.generation = next.generation.next().unwrap();
    assert!(matches!(
        LiveArchive::prepare(&root, next),
        Err(error) if error.starts_with("Recoverable:")
    ));
    assert!(!live_dir.join("passage-2.pending.json").exists());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn fully_covered_but_unpublished_passage_stays_recoverable_and_blocks_next_generation() {
    let root = test_root();
    let request = LiveRequest {
        job_id: JobId(0x10a),
        generation: Generation::first(),
        model_path: "model.bin".into(),
        model_sha256: [9; 32],
        destination: root.to_string_lossy().into_owned(),
        group_offset_samples: 0,
        auto_stop_after_speech_samples: None,
        microphone: Default::default(),
        microphone_gain_db: 0,
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        },
    };
    let mut archive = LiveArchive::prepare(&root, request.clone()).unwrap();
    let live_dir = root
        .join("transcriptions")
        .join(format!("{:032x}", request.job_id.0))
        .join("live");
    let mut staging = PcmStaging::create(live_dir.join("passage-1.pcm")).unwrap();
    for _ in 0..25 {
        staging.append_frame(&[0; 320]).unwrap();
    }
    archive
        .persist_window_finished(
            1,
            SourceRange::new(0, 8_000, 16_000).unwrap(),
            0,
            staging.durable_samples(),
        )
        .unwrap();
    drop(staging);
    drop(archive);

    let recoveries = whisper_adapters::archive::scan_live_recoveries(&root).unwrap();
    assert_eq!(recoveries.len(), 1);
    assert_eq!(recoveries[0].request, request);
    assert!(!recoveries[0].publication_ready);
    assert_eq!(
        recoveries,
        whisper_adapters::archive::scan_live_recoveries(&root).unwrap(),
        "the unresolved passage remains stably discoverable under the same identity"
    );

    let mut next = request.clone();
    next.generation = next.generation.next().unwrap();
    let error = match LiveArchive::prepare(&root, next) {
        Ok(_) => panic!("a fully covered but unpublished passage must block the next one"),
        Err(error) => error,
    };
    assert!(error.starts_with("Recoverable:"), "{error}");
    assert!(!live_dir.join("passage-2.pending.json").exists());
    assert_eq!(
        serde_json::from_slice::<LiveRequest>(
            &std::fs::read(live_dir.join("passage-1.pending.json")).unwrap()
        )
        .unwrap(),
        request,
        "blocking the next generation preserves passage 1's replay identity"
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn next_passage_refuses_text_after_the_last_coverage_commit() {
    let root = test_root();
    let request = LiveRequest {
        job_id: JobId(0x107),
        generation: Generation::first(),
        model_path: "model.bin".into(),
        model_sha256: [9; 32],
        destination: root.to_string_lossy().into_owned(),
        group_offset_samples: 0,
        auto_stop_after_speech_samples: None,
        microphone: Default::default(),
        microphone_gain_db: 0,
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        },
    };
    let mut archive = LiveArchive::prepare(&root, request.clone()).unwrap();
    let live_dir = root
        .join("transcriptions")
        .join(format!("{:032x}", request.job_id.0))
        .join("live");
    let mut staging = PcmStaging::create(live_dir.join("passage-1.pcm")).unwrap();
    for _ in 0..25 {
        staging.append_frame(&[0; 320]).unwrap();
    }
    archive
        .persist_window_finished(
            1,
            SourceRange::new(0, 8_000, 16_000).unwrap(),
            0,
            staging.durable_samples(),
        )
        .unwrap();
    archive
        .persist_segment(
            &WorkerSegment {
                job_id: request.job_id,
                generation: request.generation,
                instance_id: 1,
                segment_id: SegmentId(1),
                range: SourceRange::new(7_000, 7_500, 16_000).unwrap(),
                text: "unconfirmed".into(),
            },
            staging.durable_samples(),
        )
        .unwrap();
    drop(staging);
    drop(archive);

    let mut next = request;
    next.generation = next.generation.next().unwrap();
    assert!(matches!(
        LiveArchive::prepare(&root, next),
        Err(error) if error.starts_with("Recoverable:")
    ));
    assert!(!live_dir.join("passage-2.pending.json").exists());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn next_passage_refuses_unconfirmed_pcm_after_the_durable_checkpoint() {
    use std::io::Write;

    let root = test_root();
    let request = LiveRequest {
        job_id: JobId(0x108),
        generation: Generation::first(),
        model_path: "model.bin".into(),
        model_sha256: [9; 32],
        destination: root.to_string_lossy().into_owned(),
        group_offset_samples: 0,
        auto_stop_after_speech_samples: None,
        microphone: Default::default(),
        microphone_gain_db: 0,
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        },
    };
    let mut archive = LiveArchive::prepare(&root, request.clone()).unwrap();
    let live_dir = root
        .join("transcriptions")
        .join(format!("{:032x}", request.job_id.0))
        .join("live");
    let pcm_path = live_dir.join("passage-1.pcm");
    let mut staging = PcmStaging::create(&pcm_path).unwrap();
    for _ in 0..25 {
        staging.append_frame(&[0; 320]).unwrap();
    }
    archive
        .persist_window_finished(
            1,
            SourceRange::new(0, 8_000, 16_000).unwrap(),
            0,
            staging.durable_samples(),
        )
        .unwrap();
    drop(staging);
    drop(archive);
    let checkpoint_len = std::fs::metadata(&pcm_path).unwrap().len();
    std::fs::OpenOptions::new()
        .append(true)
        .open(&pcm_path)
        .unwrap()
        .write_all(&[1, 0])
        .unwrap();

    let mut next = request;
    next.generation = next.generation.next().unwrap();
    assert!(matches!(
        LiveArchive::prepare(&root, next),
        Err(error) if error.starts_with("Recoverable:")
    ));
    assert!(!live_dir.join("passage-2.pending.json").exists());
    assert_eq!(
        std::fs::metadata(&pcm_path).unwrap().len(),
        checkpoint_len + 2
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn passage_publishes_durable_pcm_mp3_and_confirmed_text_as_one_generation() {
    let root = test_root();
    let request = LiveRequest {
        job_id: JobId(0x103),
        generation: Generation::first(),
        model_path: "model.bin".into(),
        model_sha256: [7; 32],
        destination: root.to_string_lossy().into_owned(),
        group_offset_samples: 0,
        auto_stop_after_speech_samples: None,
        microphone: Default::default(),
        microphone_gain_db: 0,
        config: JobConfig {
            language: LanguageChoice::Manual("fr".into()),
            compute: ComputeChoice::Cpu,
        },
    };
    let mut archive = LiveArchive::prepare(&root, request.clone()).unwrap();
    archive.capture_started().unwrap();
    let live_dir = root
        .join("transcriptions")
        .join(format!("{:032x}", 0x103u128))
        .join("live");
    let pcm_path = live_dir.join("passage-1.pcm");
    let mut staging = PcmStaging::create(&pcm_path).unwrap();
    for _ in 0..25 {
        staging.append_frame(&[12; 320]).unwrap();
    }
    assert_eq!(staging.durable_samples(), 8_000);

    archive.append_mp3_packet(b"fixture mp3 packet").unwrap();
    archive
        .persist_segment(
            &WorkerSegment {
                job_id: JobId(0x103),
                generation: Generation::first(),
                instance_id: 1,
                segment_id: SegmentId(1),
                range: SourceRange::new(0, 320, 16_000).unwrap(),
                text: "  bonjour  ".into(),
            },
            staging.durable_samples(),
        )
        .unwrap();
    assert_eq!(
        archive
            .persist_window_finished(
                1,
                SourceRange::new(0, 4_000, 16_000).unwrap(),
                1,
                staging.durable_samples(),
            )
            .unwrap(),
        4_000
    );
    assert_eq!(
        archive
            .persist_window_finished(
                2,
                SourceRange::new(4_000, 8_000, 16_000).unwrap(),
                1,
                staging.durable_samples(),
            )
            .unwrap(),
        8_000,
        "a completed silent window advances durable transcription coverage"
    );
    archive
        .publish(staging.durable_samples(), 320, staging.checksum())
        .unwrap();

    assert_eq!(
        std::fs::read_to_string(live_dir.join("CURRENT")).unwrap(),
        "manifest-1.json\n"
    );
    assert_eq!(
        std::fs::read_to_string(live_dir.join("transcript.txt")).unwrap(),
        "bonjour"
    );
    let generation_one_text = live_dir.join("transcript-1.txt");
    assert_eq!(
        std::fs::read_to_string(&generation_one_text).unwrap(),
        "bonjour"
    );
    assert!(live_dir.join("passage-1.mp3").is_file());
    assert!(!live_dir.join("passage-1.pending.json").exists());

    let mut next_request = request.clone();
    next_request.generation = Generation::first().next().unwrap();
    let mut next_archive = LiveArchive::prepare(&root, next_request.clone()).unwrap();
    next_archive.capture_started().unwrap();
    let next_pcm_path = live_dir.join("passage-2.pcm");
    let mut next_staging = PcmStaging::create(&next_pcm_path).unwrap();
    for _ in 0..25 {
        next_staging.append_frame(&[14; 320]).unwrap();
    }
    next_archive
        .append_mp3_packet(b"second fixture packet")
        .unwrap();
    assert_eq!(
        next_archive
            .persist_window_finished(
                1,
                SourceRange::new(0, 8_000, 16_000).unwrap(),
                0,
                next_staging.durable_samples(),
            )
            .unwrap(),
        8_000
    );
    next_archive
        .publish(
            next_staging.durable_samples(),
            next_staging.durable_samples(),
            next_staging.checksum(),
        )
        .unwrap();

    // Simulate a crash after generation 2's immutable artifacts and manifest
    // were verified, but before CURRENT advanced from generation 1.
    std::fs::write(live_dir.join("CURRENT"), b"manifest-1.json\n").unwrap();
    std::fs::write(
        live_dir.join("passage-2.pending.json"),
        serde_json::to_vec(&next_request).unwrap(),
    )
    .unwrap();
    let first_cut_scan = whisper_adapters::archive::ArchiveStore::scan(&root).unwrap();
    let second_cut_scan = whisper_adapters::archive::ArchiveStore::scan(&root).unwrap();
    assert_eq!(first_cut_scan, second_cut_scan);
    let generation_one = first_cut_scan
        .iter()
        .find(|entry| entry.generation == Generation::first())
        .unwrap();
    let generation_two = first_cut_scan
        .iter()
        .find(|entry| entry.generation == Generation::first().next().unwrap())
        .unwrap();
    assert!(generation_one.complete);
    assert!(!generation_two.complete);
    assert!(
        generation_two
            .diagnostic
            .as_deref()
            .is_some_and(|diagnostic| diagnostic.starts_with("Recoverable:"))
    );
    assert_eq!(
        std::fs::read(live_dir.join("CURRENT")).unwrap(),
        b"manifest-1.json\n",
        "scanning must never advance CURRENT"
    );
    assert_eq!(
        std::fs::read(live_dir.join("transcript-1.txt")).unwrap(),
        b"bonjour",
        "the previously committed publication remains intact"
    );

    let recoveries = whisper_adapters::archive::scan_live_recoveries(&root).unwrap();
    assert_eq!(recoveries.len(), 1);
    assert!(recoveries[0].publication_ready);
    assert_eq!(
        recoveries,
        whisper_adapters::archive::scan_live_recoveries(&root).unwrap(),
        "recovery discovery must remain stable across scans"
    );
    assert_eq!(
        std::fs::read(live_dir.join("CURRENT")).unwrap(),
        b"manifest-1.json\n",
        "recovery discovery must not change CURRENT"
    );
    let mut third_request = next_request;
    third_request.generation = Generation::first().next().unwrap().next().unwrap();
    let prepare_error = match LiveArchive::prepare(&root, third_request) {
        Ok(_) => panic!("a passage after an unresolved publication must be blocked"),
        Err(error) => error,
    };
    assert!(prepare_error.starts_with("Recoverable:"), "{prepare_error}");
    assert!(!live_dir.join("passage-3.pending.json").exists());
    assert_eq!(
        std::fs::read(live_dir.join("CURRENT")).unwrap(),
        b"manifest-1.json\n"
    );
    whisper_adapters::archive::ArchiveStore::resolve_live_publication(
        &root,
        &recoveries[0].request,
    )
    .unwrap();
    assert_eq!(
        std::fs::read(live_dir.join("CURRENT")).unwrap(),
        b"manifest-2.json\n",
        "only explicit resolution may advance CURRENT"
    );
    assert!(!live_dir.join("passage-2.pending.json").exists());
    let resolved = whisper_adapters::archive::ArchiveStore::scan(&root).unwrap();
    assert!(
        resolved
            .iter()
            .filter(|entry| entry.complete)
            .any(|entry| { entry.generation == Generation::first() })
    );
    assert!(
        resolved
            .iter()
            .filter(|entry| entry.complete)
            .any(|entry| { entry.generation == Generation::first().next().unwrap() })
    );
    assert!(
        whisper_adapters::archive::scan_live_recoveries(&root)
            .unwrap()
            .is_empty()
    );

    assert_eq!(
        std::fs::read_to_string(&generation_one_text).unwrap(),
        "bonjour"
    );
    // Simulate interruption after CURRENT committed passage 2 but before the
    // compatibility aliases were refreshed from its immutable snapshot.
    std::fs::write(live_dir.join("transcript.txt"), b"stale alias").unwrap();
    assert_eq!(
        std::fs::read_to_string(live_dir.join("transcript.txt")).unwrap(),
        "stale alias"
    );
    let history = whisper_adapters::archive::ArchiveStore::scan(&root).unwrap();
    assert_eq!(
        std::fs::read_to_string(live_dir.join("transcript.txt")).unwrap(),
        "bonjour",
        "scan reconstructs the alias from the committed CURRENT manifest"
    );
    let live_passages = history.iter().filter(|entry| entry.complete).count();
    assert_eq!(
        live_passages, 2,
        "both immutable passage manifests must scan"
    );
    drop(staging);
    drop(next_staging);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn recovery_adopts_a_synced_silent_window_commit_without_a_durable_ack() {
    let root = test_root();
    let request = LiveRequest {
        job_id: JobId(0x104),
        generation: Generation::first(),
        model_path: "model.bin".into(),
        model_sha256: [8; 32],
        destination: root.to_string_lossy().into_owned(),
        group_offset_samples: 0,
        auto_stop_after_speech_samples: None,
        microphone: Default::default(),
        microphone_gain_db: 0,
        config: JobConfig {
            language: LanguageChoice::Manual("fr".into()),
            compute: ComputeChoice::Cpu,
        },
    };
    let mut archive = LiveArchive::prepare(&root, request).unwrap();
    archive.capture_started().unwrap();
    let live_dir = root
        .join("transcriptions")
        .join(format!("{:032x}", 0x104u128))
        .join("live");
    let mut staging = PcmStaging::create(live_dir.join("passage-1.pcm")).unwrap();
    for _ in 0..25 {
        staging.append_frame(&[0; 320]).unwrap();
    }
    archive
        .append_mp3_packet(b"encoded silent passage")
        .unwrap();
    archive
        .persist_window_finished(
            1,
            SourceRange::new(0, 8_000, 16_000).unwrap(),
            0,
            staging.durable_samples(),
        )
        .unwrap();

    let recoveries = whisper_adapters::archive::scan_live_recoveries(&root).unwrap();
    assert_eq!(recoveries.len(), 1);
    assert_eq!(recoveries[0].durable_samples, 8_000);
    assert_eq!(recoveries[0].confirmed_samples, 8_000);

    use std::io::Write;
    let mut coverage = std::fs::OpenOptions::new()
        .append(true)
        .open(live_dir.join("passage-1.coverage.jsonl"))
        .unwrap();
    coverage.write_all(b"{torn-tail").unwrap();
    drop(coverage);
    let recoveries = whisper_adapters::archive::scan_live_recoveries(&root).unwrap();
    assert_eq!(recoveries[0].confirmed_samples, 8_000);
}

fn characterization_live_request(root: &std::path::Path, job: JobId) -> LiveRequest {
    LiveRequest {
        job_id: job,
        generation: Generation::first(),
        model_path: "model.bin".into(),
        model_sha256: [7; 32],
        destination: root.to_string_lossy().into_owned(),
        group_offset_samples: 0,
        auto_stop_after_speech_samples: None,
        microphone: Default::default(),
        microphone_gain_db: 0,
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        },
    }
}

fn characterization_live_dir(root: &std::path::Path, job: JobId) -> std::path::PathBuf {
    root.join("transcriptions")
        .join(format!("{:032x}", job.0))
        .join("live")
}

#[test]
fn archive_characterization_attempt_journal_corruption_is_not_read_by_recovery() {
    let root = test_root();
    let job = JobId(0x5402);
    let request = characterization_live_request(&root, job);
    let mut archive = LiveArchive::prepare(&root, request.clone()).unwrap();
    let dir = characterization_live_dir(&root, job);
    let mut staging = PcmStaging::create(dir.join("passage-1.pcm")).unwrap();
    for _ in 0..25 {
        staging.append_frame(&[0; 320]).unwrap();
    }
    let attempt = archive.reserve_attempt().unwrap();
    archive.mark_attempt_started(attempt).unwrap();
    archive.retire_attempt(attempt).unwrap();
    drop(staging);
    drop(archive);
    let path = dir.join("passage-1.attempts.jsonl");
    let original = std::fs::read_to_string(&path).unwrap();
    let lines: Vec<_> = original.lines().collect();
    assert_eq!(lines.len(), 3);
    for (index, line) in lines.iter().enumerate() {
        use sha2::{Digest, Sha256};
        let value: serde_json::Value = serde_json::from_str(line).unwrap();
        assert_eq!(value["payload"]["journal_sequence"], (index + 1) as u64);
        let payload = line
            .strip_prefix("{\"payload\":")
            .unwrap()
            .split_once(",\"sha256\":")
            .unwrap()
            .0;
        assert_eq!(
            value["sha256"].as_str().unwrap(),
            format!("{:x}", Sha256::digest(payload.as_bytes()))
        );
    }
    let mut first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    first["sha256"] = serde_json::json!("00".repeat(32));
    let corrupt = format!(
        "{}\n{}\n{}\n",
        lines[2],
        serde_json::to_string(&first).unwrap(),
        lines[1]
    );
    std::fs::write(&path, corrupt.as_bytes()).unwrap();
    let first = whisper_adapters::archive::scan_live_recoveries(&root).unwrap();
    let second = whisper_adapters::archive::scan_live_recoveries(&root).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].request, request);
    assert_eq!(first[0].durable_samples, 8_000);
    assert_eq!(first[0].confirmed_samples, 0);
    assert_eq!(std::fs::read(&path).unwrap(), corrupt.as_bytes());
}

#[test]
fn archive_characterization_live_coverage_checksum_and_order_are_refused() {
    let root = test_root();
    let job = JobId(0x5403);
    let request = characterization_live_request(&root, job);
    let mut archive = LiveArchive::prepare(&root, request).unwrap();
    let dir = characterization_live_dir(&root, job);
    let mut staging = PcmStaging::create(dir.join("passage-1.pcm")).unwrap();
    for _ in 0..25 {
        staging.append_frame(&[0; 320]).unwrap();
    }
    archive
        .persist_window_finished(1, SourceRange::new(0, 8_000, 16_000).unwrap(), 0, 8_000)
        .unwrap();
    drop(staging);
    drop(archive);
    let path = dir.join("passage-1.coverage.jsonl");
    let original = std::fs::read(&path).unwrap();
    let mut record: serde_json::Value = serde_json::from_slice(&original).unwrap();
    record["sha256"] = serde_json::json!("00".repeat(32));
    let corrupt = [serde_json::to_vec(&record).unwrap(), vec![b'\n']].concat();
    std::fs::write(&path, &corrupt).unwrap();
    for _ in 0..2 {
        let error = whisper_adapters::archive::scan_live_recoveries(&root).unwrap_err();
        assert!(
            error.contains("live coverage journal is not a verified contiguous prefix"),
            "{error}"
        );
        assert_eq!(std::fs::read(&path).unwrap(), corrupt);
    }
    let out_of_order = [original.as_slice(), original.as_slice()].concat();
    std::fs::write(&path, &out_of_order).unwrap();
    for _ in 0..2 {
        let error = whisper_adapters::archive::scan_live_recoveries(&root).unwrap_err();
        assert!(
            error.contains("live coverage journal is not a verified contiguous prefix"),
            "{error}"
        );
        assert_eq!(std::fs::read(&path).unwrap(), out_of_order);
    }
    std::fs::write(&path, &original).unwrap();
    let recovered = whisper_adapters::archive::scan_live_recoveries(&root).unwrap();
    assert_eq!(recovered[0].confirmed_samples, 8_000);
}

#[test]
fn archive_characterization_live_segment_before_coverage_stays_unconfirmed() {
    let root = test_root();
    let job = JobId(0x5404);
    let request = characterization_live_request(&root, job);
    let mut archive = LiveArchive::prepare(&root, request.clone()).unwrap();
    let dir = characterization_live_dir(&root, job);
    let mut staging = PcmStaging::create(dir.join("passage-1.pcm")).unwrap();
    for _ in 0..25 {
        staging.append_frame(&[0; 320]).unwrap();
    }
    archive
        .persist_segment(
            &WorkerSegment {
                job_id: job,
                generation: Generation::first(),
                instance_id: 1,
                segment_id: SegmentId(1),
                range: SourceRange::new(0, 320, 16_000).unwrap(),
                text: "not confirmed".into(),
            },
            8_000,
        )
        .unwrap();
    let segment = std::fs::read(dir.join("passage-1.segments.jsonl")).unwrap();
    drop(staging);
    drop(archive);
    let first = whisper_adapters::archive::scan_live_recoveries(&root).unwrap();
    let second = whisper_adapters::archive::scan_live_recoveries(&root).unwrap();
    assert_eq!(first, second);
    assert_eq!(first[0].request, request);
    assert_eq!(first[0].durable_samples, 8_000);
    assert_eq!(first[0].confirmed_samples, 0);
    assert!(!first[0].publication_ready);
    assert_eq!(
        std::fs::read(dir.join("passage-1.segments.jsonl")).unwrap(),
        segment
    );
    assert!(!dir.join("passage-1.coverage.jsonl").exists());
}

#[test]
fn archive_characterization_live_current_manifest_and_snapshot_corruption_are_stable() {
    let root = test_root();
    let job = JobId(0x5405);
    let request = characterization_live_request(&root, job);
    let mut archive = LiveArchive::prepare(&root, request).unwrap();
    archive.capture_started().unwrap();
    let dir = characterization_live_dir(&root, job);
    let mut staging = PcmStaging::create(dir.join("passage-1.pcm")).unwrap();
    for _ in 0..25 {
        staging.append_frame(&[0; 320]).unwrap();
    }
    archive.append_mp3_packet(b"fixture mp3").unwrap();
    archive
        .persist_window_finished(1, SourceRange::new(0, 8_000, 16_000).unwrap(), 0, 8_000)
        .unwrap();
    archive.publish(8_000, 8_000, staging.checksum()).unwrap();
    drop(staging);
    drop(archive);
    let cases = [
        ("CURRENT", b"../bad\n".as_slice()),
        ("manifest-1.json", b"{truncated".as_slice()),
        ("transcript-1.txt", b"changed snapshot".as_slice()),
    ];
    for (name, damaged) in cases {
        let path = dir.join(name);
        let original = std::fs::read(&path).unwrap();
        std::fs::write(&path, damaged).unwrap();
        let first = whisper_adapters::archive::ArchiveStore::scan(&root).unwrap();
        let second = whisper_adapters::archive::ArchiveStore::scan(&root).unwrap();
        assert_eq!(first, second);
        assert!(
            first.iter().any(|entry| entry.job_id == job
                && entry
                    .diagnostic
                    .as_deref()
                    .is_some_and(|text| text.contains("StorageCorrupt"))),
            "{name}: {first:?}"
        );
        assert_eq!(std::fs::read(&path).unwrap(), damaged);
        std::fs::write(&path, original).unwrap();
    }
    let srt_path = dir.join("transcript-1.srt");
    let srt = std::fs::read(&srt_path).unwrap();
    std::fs::remove_file(&srt_path).unwrap();
    let first = whisper_adapters::archive::ArchiveStore::scan(&root).unwrap();
    let second = whisper_adapters::archive::ArchiveStore::scan(&root).unwrap();
    assert_eq!(first, second);
    assert!(
        first
            .iter()
            .any(|entry| entry.job_id == job && entry.diagnostic.is_some())
    );
    assert!(!srt_path.exists());
    std::fs::write(&srt_path, srt).unwrap();
    let healthy = whisper_adapters::archive::ArchiveStore::scan(&root).unwrap();
    assert!(
        healthy
            .iter()
            .any(|entry| entry.job_id == job && entry.complete)
    );
}
