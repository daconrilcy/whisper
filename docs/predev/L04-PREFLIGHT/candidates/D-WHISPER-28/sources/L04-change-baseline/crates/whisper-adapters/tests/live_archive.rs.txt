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

    let mut next_request = request;
    next_request.generation = Generation::first().next().unwrap();
    let mut next_archive = LiveArchive::prepare(&root, next_request).unwrap();
    next_archive.capture_started().unwrap();
    let next_pcm_path = live_dir.join("passage-2.pcm");
    let mut next_staging = PcmStaging::create(&next_pcm_path).unwrap();
    for _ in 0..25 {
        next_staging.append_frame(&[14; 320]).unwrap();
    }
    next_archive
        .append_mp3_packet(b"second fixture packet")
        .unwrap();
    next_archive
        .publish(
            next_staging.durable_samples(),
            next_staging.durable_samples(),
            next_staging.checksum(),
        )
        .unwrap();

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
