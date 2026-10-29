use super::*;

fn block(first_frame: u64) -> ProbeAudioBlock {
    ProbeAudioBlock {
        sample_rate_hz: 48_000,
        first_frame,
        acquired_at: Instant::now(),
        gain: GainRevision::neutral(),
        mono_pre_gain: vec![0.25; 960],
        mono_post_gain_limiter: vec![0.25; 960],
        limiter_mask: vec![false; 960],
        pre_pcm_limiter_count: 0,
    }
}

#[test]
fn full_queue_invalidates_old_epoch_before_fresh_contiguous_audio() {
    let (sender, receiver) = mpsc::sync_channel(PROBE_QUEUE_BLOCKS);
    let probe = MicrophoneProbe {
        sample_rate_hz: 48_000,
        chunk_samples: 960,
        sender: Some(sender),
        epoch: Arc::new(AtomicU64::new(0)),
        epoch_started_at: Arc::new(RwLock::new(Instant::now())),
        dropped_updates: Arc::new(AtomicU64::new(0)),
        valid: Arc::new(AtomicBool::new(true)),
        latest: Arc::new(RwLock::new(None)),
        job_id: JobId(812),
        generation: Generation::first(),
        source_identity: None,
        worker: None,
        processed_chunks: Arc::new(AtomicU64::new(0)),
    };

    let (retry_epoch, _) = probe.advance_epoch();
    let barrier = *probe.epoch_started_at.read().unwrap();
    let mut same_tick = block(0);
    same_tick.acquired_at = barrier;
    same_tick.mono_pre_gain.resize(4_800, 0.25);
    same_tick.mono_post_gain_limiter.resize(4_800, 0.25);
    same_tick.limiter_mask.resize(4_800, false);
    // Equality is accepted by the acquisition barrier; five chunks cannot
    // fit in a four-slot queue when the analyzer has not drained it yet.
    probe.submit(same_tick);
    assert_eq!(probe.dropped_updates(), 1);
    assert_eq!(probe.epoch(), retry_epoch + 1);
    assert!(!probe.valid.load(Ordering::Acquire));
    for chunk in receiver.try_iter() {
        assert_ne!(chunk.epoch, probe.epoch());
    }

    let mut analyzer = ProbeAnalyzer::new(48_000, None, JobId(812), Generation::first());
    for index in 0..5 {
        probe.submit(block(index * 960));
        let chunk = receiver.try_recv().unwrap();
        assert_eq!(chunk.epoch, probe.epoch());
        let snapshot = analyzer.push(chunk, probe.dropped_updates());
        if index < 4 {
            assert!(
                snapshot.is_none(),
                "a fresh FFT publication needs 4,800 contiguous frames"
            );
        } else {
            let spectrum = snapshot.unwrap().spectrum.unwrap();
            assert_eq!(spectrum.epoch, probe.epoch());
            assert_eq!(spectrum.pre_gain.last_frame, 4_800);
        }
    }
}
