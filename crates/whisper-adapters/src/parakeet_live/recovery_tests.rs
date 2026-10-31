use super::*;

#[test]
fn unified_capture_pump_converts_synthetic_input_and_closes_the_complete_source() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = env::temp_dir().join(format!("whisper-unified-capture-{stamp}"));
    fs::create_dir_all(&root).unwrap();
    let path = root.join("capture.wav");
    let mut capture =
        crate::capture::CaptureStream::empty_test_capture(48_000, 1, "test:synthetic");
    capture.enqueue_test_block(vec![0.25; 48_000]);
    let mut pump = UnifiedCapturePump::from_capture(capture, path.clone()).unwrap();
    let _ = pump.pump().unwrap();
    let (total_samples, acknowledgements) = pump.stop().unwrap();
    assert_eq!(pump.stop().unwrap().0, total_samples, "stop is idempotent");
    assert_eq!(total_samples, 16_000);
    assert!(!acknowledgements.is_empty());
    assert!(acknowledgements.iter().all(|ack| {
        ack.range.end <= total_samples
            && ack.written_samples == ack.flushed_samples
            && ack.synchronized_samples == 0
    }));
    let stored = crate::parakeet_audio::PcmPartStore::open(path)
        .read_range(0..total_samples)
        .unwrap();
    assert_eq!(stored.len(), total_samples as usize);
    assert!(stored.iter().any(|sample| *sample != 0));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn stop_recovery_skips_a_portion_already_attempted_during_capture() {
    let mut attempts = RecoveryAttemptLedger::default();
    let live_word = SourceRange::new(116_480, 119_040, SAMPLE_RATE as u32).unwrap();
    attempts.record(live_word);

    // Finalization may represent the same pending word as a wider gap.
    let stop_gap = SourceRange::new(112_000, 120_000, SAMPLE_RATE as u32).unwrap();
    assert!(attempts.contains_overlap(stop_gap));
    assert_eq!(attempts.len(), 1);

    // A disjoint unresolved portion remains eligible for its own bounded attempt.
    let later_gap = SourceRange::new(140_000, 148_000, SAMPLE_RATE as u32).unwrap();
    assert!(!attempts.contains_overlap(later_gap));
    attempts.record(later_gap);
    assert_eq!(attempts.len(), 2);
}

#[test]
fn recovery_window_keeps_context_for_the_observed_ah_word() {
    let word = SourceRange::new(116_480, 119_040, SAMPLE_RATE as u32).unwrap();
    let (start, count) = recovery_window(word, 0, 160_000).unwrap();
    assert_eq!((start, count), (85_760, 64_000));
    assert!(start <= word.start_sample);
    assert!(start + count >= word.end_sample);
    assert!(word.start_sample - start >= RECOVERY_MIN_CONTEXT_SAMPLES);
    assert!(start + count - word.end_sample >= RECOVERY_MIN_CONTEXT_SAMPLES);
}

#[test]
fn recovery_window_refuses_a_portion_without_durable_context() {
    let word = SourceRange::new(116_480, 119_040, SAMPLE_RATE as u32).unwrap();
    assert_eq!(recovery_window(word, 0, 32_000), None);
    assert_eq!(
        recovery_window(
            SourceRange::new(1_000, 20_000, SAMPLE_RATE as u32).unwrap(),
            0,
            64_000,
        ),
        None
    );
}
