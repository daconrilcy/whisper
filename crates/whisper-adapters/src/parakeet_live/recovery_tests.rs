use super::*;

#[test]
fn unified_capture_recycles_slots_and_preserves_audio_beyond_ring_capacity() {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = env::temp_dir().join(format!("whisper-unified-recycling-{stamp}"));
    fs::create_dir_all(&root).unwrap();
    let path = root.join("capture.wav");
    let mut pump = UnifiedCapturePump::from_capture(
        CaptureStream::empty_test_capture(16_000, 1, "test:synthetic"),
        path.clone(),
    )
    .unwrap();
    pump.enable_probe(JobId(101), Generation::first()).unwrap();
    let mut reference_capture = CaptureStream::empty_test_capture(16_000, 1, "test:synthetic");
    let mut reference_converter = Pcm16Converter::new(16_000, 1).unwrap();
    let mut expected = Vec::new();
    let mut acknowledgements = Vec::new();
    let mut gain_events = Vec::new();
    let blocks = crate::capture::CAPTURE_SLOTS * 2 + 3;
    for index in 0..blocks {
        if index == crate::capture::CAPTURE_SLOTS + 1 {
            let gain = whisper_core::microphone_level::GainDb::new(6).unwrap();
            pump.set_gain("test:synthetic", 0, gain).unwrap();
            reference_capture.set_gain(0, gain).unwrap();
        }
        let input = vec![if index % 2 == 0 { 0.1 } else { -0.1 }; 16];
        reference_capture.enqueue_test_block(input.clone());
        let reference_block = reference_capture.try_next_block().unwrap();
        expected.extend(reference_converter.push(&reference_block).unwrap());
        reference_capture.recycle_block(reference_block).unwrap();
        pump.enqueue_test_block(input);
        if index < blocks - 3 {
            acknowledgements.extend(pump.pump().unwrap());
            gain_events.extend(pump.take_gain_events());
        }
    }
    expected.extend(reference_converter.finish().unwrap());
    let (total_samples, final_acknowledgements) = pump.stop().unwrap();
    acknowledgements.extend(final_acknowledgements);
    gain_events.extend(pump.take_gain_events());
    assert_eq!(total_samples, (blocks * 16) as u64);
    assert_eq!(total_samples as usize, expected.len());
    let stored = crate::parakeet_audio::PcmPartStore::open(path)
        .read_range(0..total_samples)
        .unwrap();
    assert_eq!(stored, expected);
    assert!(!acknowledgements.is_empty());
    assert_eq!(acknowledgements.last().unwrap().range.end, total_samples);
    assert!(acknowledgements.iter().all(|ack| {
        ack.range.end <= total_samples && ack.written_samples == ack.flushed_samples
    }));
    assert_eq!(gain_events.len(), 2);
    assert_eq!(
        (gain_events[0].gain_revision, gain_events[0].first_frame),
        (0, 0)
    );
    assert_eq!(
        (gain_events[1].gain_revision, gain_events[1].first_frame),
        (1, ((crate::capture::CAPTURE_SLOTS + 1) * 16) as u64)
    );
}

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
