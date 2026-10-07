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
        config: JobConfig {
            language: LanguageChoice::Manual("fr".into()),
            compute: ComputeChoice::Cpu,
        },
    };
    let mut archive = LiveArchive::prepare(&root, request).unwrap();
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
    assert!(live_dir.join("passage-1.mp3").is_file());
    assert!(!live_dir.join("passage-1.pending.json").exists());
    drop(staging);
    std::fs::remove_dir_all(root).unwrap();
}
