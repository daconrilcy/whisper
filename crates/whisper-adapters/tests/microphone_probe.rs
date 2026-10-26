use std::{
    f32::consts::TAU,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};

use whisper_adapters::capture::ProbeAudioBlock;
use whisper_adapters::microphone_probe::{
    MicrophoneProbe, MicrophoneSpectrumReader, ProbeSnapshot,
};
use whisper_core::{
    GainDb, Generation, JobId,
    microphone_level::{GainRevision, MicrophoneSpectrumUpdate},
};

fn block(first_frame: u64, samples: usize) -> ProbeAudioBlock {
    let data: Vec<_> = (first_frame..first_frame + samples as u64)
        .map(|frame| {
            let phase = TAU * 64.0 * frame as f32 / 2048.0;
            0.5 * phase.sin()
        })
        .collect();
    ProbeAudioBlock {
        sample_rate_hz: 48_000,
        first_frame,
        acquired_at: Instant::now(),
        gain: GainRevision {
            revision: 0,
            gain: GainDb::new(0).unwrap(),
        },
        mono_pre_gain: data.clone(),
        mono_post_gain_limiter: data,
        limiter_mask: vec![false; samples],
        pre_pcm_limiter_count: 0,
    }
}

fn wait_snapshot(probe: &MicrophoneProbe, after_frame: u64) -> Arc<ProbeSnapshot> {
    let until = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(snapshot) = probe.latest_at(Instant::now())
            && snapshot.last_frame >= after_frame
        {
            return snapshot;
        }
        assert!(Instant::now() < until, "probe snapshot was not published");
        thread::sleep(Duration::from_millis(2));
    }
}

fn submit_blocks(probe: &MicrophoneProbe, first_index: u64, count: u64) {
    for index in first_index..first_index + count {
        probe.submit(block(index * 960, 960));
        thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn analyzer_publishes_bounded_psd_and_slow_readers_keep_immutable_snapshots() {
    let probe = MicrophoneProbe::start(
        48_000,
        Some("test:mic-A".into()),
        JobId(1),
        Generation::first(),
    )
    .unwrap();
    submit_blocks(&probe, 0, 5);
    let held_by_slow_reader = wait_snapshot(&probe, 4800);
    let independent_reader = probe.latest_at(Instant::now()).unwrap();
    assert!(Arc::ptr_eq(&held_by_slow_reader, &independent_reader));
    let spectrum = held_by_slow_reader.pre_gain_psd.as_ref().unwrap();
    assert_eq!(spectrum.bins.len(), 1025);
    assert_eq!(spectrum.first_frame, held_by_slow_reader.last_frame - 2048);
    assert_eq!(spectrum.last_frame, held_by_slow_reader.last_frame);
    assert!(
        spectrum
            .bins
            .iter()
            .all(|bin| bin.is_finite() && *bin >= 0.0)
    );

    submit_blocks(&probe, 5, 5);
    let newer = wait_snapshot(&probe, 9600);
    assert!(newer.last_frame > held_by_slow_reader.last_frame);
    assert!(held_by_slow_reader.pre_gain_psd.is_some());
    drop(independent_reader);
}

#[test]
fn invalidation_hides_a_snapshot_even_when_a_reader_still_holds_it() {
    let probe = MicrophoneProbe::start(
        48_000,
        Some("test:mic-A".into()),
        JobId(2),
        Generation::first(),
    )
    .unwrap();
    submit_blocks(&probe, 0, 5);
    let held_by_reader = wait_snapshot(&probe, 4800);
    probe.invalidate();
    assert!(probe.latest_at(Instant::now()).is_none());
    assert!(held_by_reader.pre_gain.is_some());
}

#[test]
fn real_queue_overflow_is_counted_and_measurement_recovers_after_a_full_window() {
    let probe = MicrophoneProbe::start(
        48_000,
        Some("test:mic-A".into()),
        JobId(3),
        Generation::first(),
    )
    .unwrap();
    let overflow_frames = 480_000_u64;
    probe.submit(block(0, overflow_frames as usize));
    assert!(probe.dropped_updates() > 0);
    let dropped = probe.dropped_updates();

    // Resume at the real post-gap frame position; the probe may use already
    // accepted contiguous chunks after the gap, but must rebuild its window.
    submit_blocks(&probe, overflow_frames / 960, 8);
    let recovered = wait_snapshot(&probe, overflow_frames + 4_800);
    assert!(recovered.dropped_updates >= dropped);
    assert!(recovered.pre_gain.is_some());
}

#[test]
fn spectrum_is_cached_across_rms_ticks_and_invalidated_once_on_loss() {
    let probe = MicrophoneProbe::start(
        48_000,
        Some("test:mic-spectrum".into()),
        JobId(4),
        Generation::first(),
    )
    .unwrap();
    submit_blocks(&probe, 0, 5);
    let first_snapshot = wait_snapshot(&probe, 4_800);
    let first = first_snapshot.spectrum.as_ref().unwrap().clone();
    assert_eq!(first.pre_gain.bins.len(), 1025);
    assert_eq!(first.post_gain_limiter.bins.len(), 1025);

    // New RMS measurements may arrive more often than a fresh FFT. The
    // cached spectral sequence and capture time must remain unchanged.
    submit_blocks(&probe, 5, 3);
    let later = wait_snapshot(&probe, 7_680);
    let cached = later.spectrum.as_ref().unwrap();
    assert_eq!(cached.sequence, first.sequence);
    assert_eq!(cached.captured_at, first.captured_at);

    let mut reader = MicrophoneSpectrumReader::default();
    let update = reader.poll(&probe, Instant::now()).unwrap();
    assert!(matches!(update, MicrophoneSpectrumUpdate::Available(_)));
    probe.invalidate();
    let unavailable = reader.poll(&probe, Instant::now()).unwrap();
    assert!(matches!(
        unavailable,
        MicrophoneSpectrumUpdate::Unavailable { epoch: 1, .. }
    ));
    assert!(reader.poll(&probe, Instant::now()).is_none());
}

#[test]
fn fft_peak_tracks_hz_and_gain_window_records_return_to_initial_value() {
    let probe = MicrophoneProbe::start(
        48_000,
        Some("test:mic-frequency".into()),
        JobId(5),
        Generation::first(),
    )
    .unwrap();
    for (index, gain_db) in [(0_u64, 0_i8), (1, 6), (2, 0), (3, 3), (4, 0)] {
        let mut audio = block(index * 960, 960);
        audio.gain = GainRevision {
            revision: index,
            gain: GainDb::new(gain_db).unwrap(),
        };
        probe.submit(audio);
        thread::sleep(Duration::from_millis(20));
    }
    let snapshot = wait_snapshot(&probe, 4_800);
    let spectrum = snapshot.spectrum.as_ref().unwrap();
    let peak_bin = spectrum
        .pre_gain
        .bins
        .iter()
        .enumerate()
        .max_by(|(_, left), (_, right)| left.total_cmp(right))
        .unwrap()
        .0;
    let peak_hz = peak_bin as f64 * f64::from(spectrum.pre_gain.sample_rate_hz) / 2048.0;
    assert!(
        (peak_hz - 1_500.0).abs() < 24.0,
        "expected bin 64 near 1.5 kHz, got {peak_hz} Hz"
    );
    assert_eq!(spectrum.gain_first.gain.value(), 0);
    assert_eq!(spectrum.gain_last.gain.value(), 0);
    assert_ne!(spectrum.gain_first.revision, spectrum.gain_last.revision);
    assert!(
        spectrum.mixed_gain,
        "a gain change and return must remain marked mixed"
    );
}
