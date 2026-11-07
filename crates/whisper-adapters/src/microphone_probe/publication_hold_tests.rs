use super::*;

#[test]
fn dropping_probe_releases_a_held_publication_before_joining() {
    let (sent, received) = mpsc::sync_channel(1);
    thread::spawn(move || {
        let probe = MicrophoneProbe::start(
            16_000,
            Some("test:probe-drop".into()),
            JobId(73_125),
            Generation::first(),
        )
        .unwrap();
        let hold = probe.hold_publication();
        for index in 0..5 {
            if index == 4 {
                let deadline = Instant::now() + Duration::from_secs(2);
                while probe.processed_chunks() < 4 {
                    assert!(Instant::now() < deadline);
                    thread::yield_now();
                }
            }
            probe.submit(ProbeAudioBlock {
                sample_rate_hz: 16_000,
                first_frame: index * 320,
                acquired_at: Instant::now(),
                gain: GainRevision::neutral(),
                mono_pre_gain: vec![0.25; 320],
                mono_post_gain_limiter: vec![0.25; 320],
                limiter_mask: vec![false; 320],
                pre_pcm_limiter_count: 0,
            });
        }
        drop(probe);
        drop(hold);
        sent.send(()).unwrap();
    });
    received
        .recv_timeout(Duration::from_secs(5))
        .expect("probe drop must not deadlock");
}
