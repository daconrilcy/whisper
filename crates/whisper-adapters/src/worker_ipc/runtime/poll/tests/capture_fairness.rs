use super::*;
use std::time::Duration;
use whisper_core::microphone_level::GainRevision;

const INSTANCE_ID: u64 = 7;

fn wait_for_probe_chunks(probe: &crate::microphone_probe::MicrophoneProbe, expected: u64) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while probe.processed_chunks() < expected && Instant::now() < deadline {
        std::thread::yield_now();
    }
    assert!(
        probe.processed_chunks() >= expected,
        "microphone probe did not process submitted audio"
    );
}

fn publish_probe_snapshot(runtime: &ImportRuntime, first_frame: u64) {
    let probe = &runtime.live.as_ref().expect("live state").probe;
    for index in 0..5 {
        let processed = probe.processed_chunks();
        probe.submit(crate::capture::ProbeAudioBlock {
            sample_rate_hz: 16_000,
            first_frame: first_frame + index * 320,
            acquired_at: Instant::now(),
            gain: GainRevision::neutral(),
            mono_pre_gain: vec![0.1; 320],
            mono_post_gain_limiter: vec![0.1; 320],
            limiter_mask: vec![false; 320],
            pre_pcm_limiter_count: 0,
        });
        wait_for_probe_chunks(probe, processed + 1);
    }
    assert!(probe.latest_at(Instant::now()).is_some());
}

fn attach_worker(runtime: &mut ImportRuntime, root: &Path) {
    let worker = super::compile_idle_worker(root);
    runtime.transport = Some(ChildWorkerTransport::spawn(&worker).expect("spawn idle worker"));
}

fn prime_live_capture(runtime: &mut ImportRuntime) {
    for expected_captured in [0, 320] {
        runtime
            .live
            .as_mut()
            .unwrap()
            .capture
            .enqueue_test_block(vec![0.1; 320]);
        let event = runtime.poll();
        assert!(matches!(
            event,
            None | Some(Ok(
                ImportEvent::MicrophoneSpectrum(_) | ImportEvent::MicrophoneMeter(_)
            ))
        ));
        let live = runtime.live.as_ref().unwrap();
        assert_eq!(live.captured, expected_captured);
        assert_eq!(live.admitted, expected_captured);
        assert_eq!(live.staging.durable_samples(), 0);
        assert_eq!(runtime.live_capture_events_since_worker_poll, 0);
    }
    let live = runtime.live.as_mut().unwrap();
    assert_eq!(live.staging.drain().unwrap(), 320);
    assert!(live.captured >= live.staging.durable_samples());
    assert!(live.staging.durable_samples() >= live.confirmed_fragment_end);
}

fn assert_live_counters(
    event: Option<Result<ImportEvent, (JobId, Generation, String)>>,
    job_id: JobId,
) {
    assert!(matches!(
        event,
        Some(Ok(ImportEvent::LiveCounters {
            job_id: actual,
            captured_samples,
            audio_durable_samples,
            confirmed_fragment_samples,
            ..
        })) if actual == job_id
            && captured_samples >= audio_durable_samples
            && audio_durable_samples >= confirmed_fragment_samples
    ));
}

#[test]
fn poll_serves_telemetry_then_worker_at_capture_quota_and_resumes() {
    let root = TestRoot::new("capture-poll-fairness");
    let job_id = JobId(0x99702);
    let generation = Generation::first();
    let mut runtime = runtime_with_active_attempt(job_id);
    runtime.live = Some(super::live_capture_state(&root.0, job_id));
    attach_worker(&mut runtime, &root.0);
    prime_live_capture(&mut runtime);

    runtime
        .live
        .as_mut()
        .unwrap()
        .capture
        .enqueue_test_block(vec![0.1; 320]);
    let first_capture_event = runtime.poll();
    assert_live_counters(first_capture_event, job_id);
    assert_eq!(runtime.live_capture_events_since_worker_poll, 1);
    let live = runtime.live.as_mut().unwrap();
    assert!(live.captured >= live.staging.durable_samples());
    assert!(live.staging.drain().unwrap() > 320);

    let processed = runtime.live.as_ref().unwrap().probe.processed_chunks();
    publish_probe_snapshot(&runtime, 100_000);
    runtime
        .live
        .as_mut()
        .unwrap()
        .capture
        .enqueue_test_block(vec![0.1; 320]);
    runtime
        .transport
        .as_mut()
        .unwrap()
        .inject_event(WorkerEvent::Progress {
            job_id,
            generation,
            instance_id: INSTANCE_ID,
            completed_samples: 160,
            total_samples: 320,
        });
    let telemetry_after_first_capture = runtime.poll();
    assert!(matches!(
        telemetry_after_first_capture,
        Some(Ok(
            ImportEvent::MicrophoneSpectrum(_) | ImportEvent::MicrophoneMeter(_)
        ))
    ));
    assert_eq!(runtime.live_capture_events_since_worker_poll, 1);
    assert_eq!(runtime.live.as_ref().unwrap().captured, 640);
    wait_for_probe_chunks(&runtime.live.as_ref().unwrap().probe, processed + 5);

    let second_capture_event = runtime.poll();
    assert_live_counters(second_capture_event, job_id);
    assert_eq!(runtime.live_capture_events_since_worker_poll, 2);
    assert_eq!(runtime.live.as_ref().unwrap().captured, 960);
    assert_eq!(runtime.live.as_mut().unwrap().staging.drain().unwrap(), 960);

    publish_probe_snapshot(&runtime, 200_000);
    runtime
        .live
        .as_mut()
        .unwrap()
        .capture
        .enqueue_test_block(vec![0.1; 320]);

    let telemetry_at_quota = runtime.poll();
    assert!(matches!(
        telemetry_at_quota,
        Some(Ok(
            ImportEvent::MicrophoneSpectrum(_) | ImportEvent::MicrophoneMeter(_)
        ))
    ));
    assert_eq!(runtime.live_capture_events_since_worker_poll, 2);
    assert_eq!(runtime.live.as_ref().unwrap().captured, 960);

    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::Progress {
            job_id: actual,
            completed_samples: 160,
            total_samples: 320,
            ..
        })) if actual == job_id
    ));
    assert_eq!(runtime.live_capture_events_since_worker_poll, 0);
    assert_eq!(runtime.live.as_ref().unwrap().captured, 960);

    assert_live_counters(runtime.poll(), job_id);
    assert_eq!(runtime.live_capture_events_since_worker_poll, 1);
    let live = runtime.live.as_ref().unwrap();
    assert!(live.captured >= live.staging.durable_samples());
    assert!(live.staging.durable_samples() >= live.confirmed_fragment_end);
    assert_eq!(live.captured, 1_280);
    assert_eq!(live.staging.durable_samples(), 960);
}
