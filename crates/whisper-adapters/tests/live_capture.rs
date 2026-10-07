use std::time::{SystemTime, UNIX_EPOCH};
use whisper_adapters::{
    capture::{CAPTURE_SLOTS, Pcm16Converter},
    staging::PcmStaging,
};

#[test]
fn capture_conversion_rejects_unsupported_dimensions_and_uses_bounded_slots() {
    assert_eq!(CAPTURE_SLOTS, 100);
    assert!(Pcm16Converter::new(8_000, 1).is_err());
    assert!(Pcm16Converter::new(48_000, 0).is_err());
    assert!(Pcm16Converter::new(48_000, 2).is_ok());
}

#[test]
fn bounded_pcm_writer_drains_queued_frames_to_a_verified_checkpoint() {
    let path = std::env::temp_dir().join(format!(
        "whisper-live-writer-{}-{}.pcm",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut writer = PcmStaging::create(&path).unwrap().into_writer();
    for _ in 0..25 {
        writer.try_append_frame(vec![23; 320]).unwrap();
    }

    assert_eq!(writer.drain().unwrap(), 8_000);
    assert_eq!(writer.durable_samples(), 8_000);
    assert_eq!(
        PcmStaging::inspect_checkpoint(&path)
            .unwrap()
            .durable_samples,
        8_000
    );
    drop(writer);
    std::fs::remove_file(&path).unwrap();
    std::fs::remove_file(path.with_file_name(format!(
        "{}.state.json",
        path.file_name().unwrap().to_string_lossy()
    )))
    .unwrap();
}
