use super::*;
use whisper_core::{
    SegmentId, SourceRange,
    ipc::WorkerSegmentDto,
    ports::{ImportEffect, WorkerSegment},
};

const INSTANCE_ID: u64 = 7;

fn source_range(start: u64, end: u64) -> SourceRange {
    SourceRange::new(start, end, 16_000).expect("valid source range")
}

fn archive_for(root: &Path, job_id: JobId) -> crate::archive::LiveArchive {
    let request = live_request(root, job_id);
    let destination = request.destination.clone();
    crate::archive::LiveArchive::prepare(destination, request)
        .expect("prepare synthetic live archive")
}

fn attach_live(runtime: &mut ImportRuntime, root: &Path, job_id: JobId) {
    runtime.live = Some(live_capture_state(root, job_id));
    runtime.live_capture_events_since_worker_poll = 2;
}

fn attach_worker(runtime: &mut ImportRuntime, root: &Path) {
    let worker = compile_idle_worker(root);
    runtime.transport = Some(ChildWorkerTransport::spawn(&worker).expect("spawn idle worker"));
}

fn inject(runtime: &mut ImportRuntime, event: WorkerEvent) {
    runtime
        .transport
        .as_mut()
        .expect("worker transport")
        .inject_event(event);
}

fn mp3_packet(job_id: JobId, instance_id: u64, bytes: &[u8]) -> WorkerEvent {
    WorkerEvent::LiveMp3Packet {
        job_id,
        generation: Generation::first(),
        instance_id,
        sequence: 1,
        bytes: bytes.to_vec(),
    }
}

fn window_finished(
    job_id: JobId,
    sequence: u64,
    range: SourceRange,
    last_segment_sequence: u64,
) -> WorkerEvent {
    WorkerEvent::WindowFinished {
        job_id,
        generation: Generation::first(),
        instance_id: INSTANCE_ID,
        sequence,
        range,
        last_segment_sequence,
    }
}

fn record_stop_worker(root: &Path, marker: &Path) -> PathBuf {
    let source = root.join("stop-observer.rs");
    let executable = root.join("stop-observer.exe");
    let marker =
        serde_json::to_string(&marker.to_string_lossy().to_string()).expect("encode marker path");
    let program = r#"
use std::{fs, io::{self, BufRead}};
const MARKER: &str = __MARKER__;
fn main() {
    for line in io::stdin().lock().lines().flatten() {
        if line.contains("\"Stop\"") {
            fs::write(MARKER, line).unwrap();
            return;
        }
    }
}
"#
    .replace("__MARKER__", &marker);
    fs::write(&source, program).expect("write Stop observer worker");
    let output = Command::new("rustc")
        .args(["--edition=2021", "--crate-name", "poll_stop_observer"])
        .arg(&source)
        .arg("-o")
        .arg(&executable)
        .output()
        .expect("compile Stop observer worker");
    assert!(
        output.status.success(),
        "rustc Stop observer: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    executable
}

fn append_durable_samples(runtime: &mut ImportRuntime, samples: usize) {
    let staging = &runtime.live.as_ref().expect("live capture").staging;
    staging
        .append_frame(vec![0; samples])
        .expect("enqueue synthetic PCM frame");
    runtime
        .live
        .as_mut()
        .expect("live capture")
        .staging
        .drain()
        .expect("drain synthetic PCM frame");
}

fn publish_probe_snapshot(runtime: &ImportRuntime) {
    let probe = &runtime.live.as_ref().expect("live state").probe;
    for index in 0..5 {
        let processed = probe.processed_chunks();
        let first_frame = index * 320;
        let acquired_at = Instant::now();
        probe.submit(crate::capture::ProbeAudioBlock {
            sample_rate_hz: 16_000,
            first_frame,
            acquired_at,
            gain: whisper_core::microphone_level::GainRevision::neutral(),
            mono_pre_gain: vec![0.1; 320],
            mono_post_gain_limiter: vec![0.1; 320],
            limiter_mask: vec![false; 320],
            pre_pcm_limiter_count: 0,
        });
        let deadline = Instant::now() + Duration::from_secs(2);
        while probe.processed_chunks() == processed && Instant::now() < deadline {
            std::thread::yield_now();
        }
        assert!(probe.processed_chunks() > processed, "probe consumed audio");
    }
    assert!(probe.latest_at(Instant::now()).is_some());
}

fn mp3_pending_path(root: &Path, job_id: JobId) -> PathBuf {
    root.join("archive")
        .join("transcriptions")
        .join(format!("{:032x}", job_id.0))
        .join("live")
        .join("passage-1.mp3.pending")
}

#[test]
fn poll_filters_stale_mp3_and_window_before_effects_then_appends_current_packet() {
    let root = TestRoot::new("stale-live-packet-window");
    let job_id = JobId(0x99401);
    let mut runtime = runtime_with_active_attempt(job_id);
    attach_live(&mut runtime, &root.0, job_id);
    runtime.live_archive = Some(archive_for(&root.0, job_id));
    attach_worker(&mut runtime, &root.0);
    inject(
        &mut runtime,
        mp3_packet(JobId(job_id.0 + 1), INSTANCE_ID, b"stale"),
    );
    let mut stale_window = window_finished(job_id, 1, source_range(0, 320), 0);
    if let WorkerEvent::WindowFinished { instance_id, .. } = &mut stale_window {
        *instance_id += 1;
    }
    inject(&mut runtime, stale_window);
    inject(&mut runtime, mp3_packet(job_id, INSTANCE_ID, b"current"));

    assert!(runtime.poll().is_none());
    assert_eq!(fs::read(mp3_pending_path(&root.0, job_id)).unwrap(), b"");
    assert!(runtime.poll().is_none());
    assert_eq!(
        runtime.live_archive.as_ref().unwrap().confirmed_samples(),
        0,
        "stale WindowFinished must not reach confirmation"
    );
    assert!(runtime.poll().is_none());
    assert_eq!(
        fs::read(mp3_pending_path(&root.0, job_id)).unwrap(),
        b"current"
    );
    assert!(runtime.live.is_some());
    assert!(runtime.active.is_some());
}

#[test]
fn poll_mp3_append_error_stops_and_drains_capture_records_failure_and_sends_stop() {
    let root = TestRoot::new("mp3-append-error");
    let job_id = JobId(0x99402);
    let mut runtime = runtime_with_active_attempt(job_id);
    attach_live(&mut runtime, &root.0, job_id);
    publish_probe_snapshot(&runtime);
    runtime
        .live
        .as_ref()
        .unwrap()
        .staging
        .append_tail(vec![0; 160])
        .expect("queue non-durable PCM tail");
    assert_eq!(runtime.live.as_ref().unwrap().staging.durable_samples(), 0);
    let mut archive = archive_for(&root.0, job_id);
    crate::archive::test_support::replace_mp3_handle_with_read_only(&mut archive)
        .expect("replace archive handle with read-only handle");
    runtime.live_archive = Some(archive);
    let marker = root.0.join("stop-command.json");
    let worker = record_stop_worker(&root.0, &marker);
    runtime.transport = Some(ChildWorkerTransport::spawn(&worker).expect("spawn Stop observer"));
    inject(
        &mut runtime,
        mp3_packet(job_id, INSTANCE_ID, b"cannot-write"),
    );

    assert!(
        runtime.poll().is_none(),
        "append failure returns immediately"
    );
    {
        let live = runtime.live.as_ref().expect("live state is retained");
        assert!(
            live.probe.latest_at(Instant::now()).is_none(),
            "capture stop invalidates the probe snapshot"
        );
        assert_eq!(
            live.staging.durable_samples(),
            160,
            "failure drains PCM tail"
        );
        assert!(
            live.failure
                .as_deref()
                .is_some_and(|failure| failure.contains("StorageUnavailable: MP3 write:")),
            "actual read-only file write error is retained"
        );
    }
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(status) = runtime
            .transport
            .as_mut()
            .expect("Stop observer transport")
            .try_wait()
            .expect("poll Stop observer exit")
        {
            assert!(status.success(), "Stop observer exited successfully");
            break;
        }
        assert!(Instant::now() < deadline, "Stop observer did not exit");
        std::thread::yield_now();
    }
    let command = fs::read_to_string(marker).expect("worker observed Stop command");
    assert!(command.contains("Stop"));
    assert!(runtime.active.is_some());
    assert!(runtime.transport.is_some());
}

#[test]
fn poll_mp3_packet_without_archive_keeps_current_error_cleanup_behavior() {
    let root = TestRoot::new("mp3-no-archive");
    let job_id = JobId(0x99403);
    let mut runtime = runtime_with_active_attempt(job_id);
    attach_live(&mut runtime, &root.0, job_id);
    publish_probe_snapshot(&runtime);
    attach_worker(&mut runtime, &root.0);
    inject(&mut runtime, mp3_packet(job_id, INSTANCE_ID, b"packet"));

    assert!(runtime.poll().is_none());
    let live = runtime
        .live
        .as_ref()
        .expect("current cleanup retains live state");
    assert!(live.probe.latest_at(Instant::now()).is_none());
    assert_eq!(
        live.failure.as_deref(),
        Some("StaleResponse: worker event identity mismatch")
    );
    assert!(runtime.live_archive.is_none());
    assert!(runtime.active.is_some());
    assert!(runtime.transport.is_some());
}

#[test]
fn poll_window_finished_outside_live_returns_protocol_error_before_next_event() {
    let root = TestRoot::new("window-outside-live");
    let job_id = JobId(0x99404);
    let mut runtime = runtime_with_active_attempt(job_id);
    attach_worker(&mut runtime, &root.0);
    inject(
        &mut runtime,
        window_finished(job_id, 1, source_range(0, 320), 0),
    );
    inject(
        &mut runtime,
        WorkerEvent::Progress {
            job_id,
            generation: Generation::first(),
            instance_id: INSTANCE_ID,
            completed_samples: 9,
            total_samples: 20,
        },
    );

    assert!(matches!(
        runtime.poll(),
        Some(Err((actual, generation, message)))
            if actual == job_id
                && generation == Generation::first()
                && message == "ProtocolMismatch: WindowFinished outside a live pass"
    ));
    assert!(runtime.live.is_none());
    assert!(runtime.active.is_some());
    assert!(runtime.transport.is_some());
    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::Progress {
            completed_samples: 9,
            ..
        }))
    ));
}

#[test]
fn poll_ignores_window_fully_inside_replay_and_preserves_following_event() {
    let root = TestRoot::new("window-replay");
    let job_id = JobId(0x99405);
    let mut runtime = runtime_with_active_attempt(job_id);
    attach_live(&mut runtime, &root.0, job_id);
    runtime.live.as_mut().unwrap().replay_until = 320;
    runtime.live_archive = Some(archive_for(&root.0, job_id));
    attach_worker(&mut runtime, &root.0);
    inject(
        &mut runtime,
        window_finished(job_id, 1, source_range(0, 320), 0),
    );
    inject(
        &mut runtime,
        WorkerEvent::Progress {
            job_id,
            generation: Generation::first(),
            instance_id: INSTANCE_ID,
            completed_samples: 11,
            total_samples: 30,
        },
    );

    assert!(runtime.poll().is_none());
    assert_eq!(
        runtime.live_archive.as_ref().unwrap().confirmed_samples(),
        0
    );
    assert!(runtime.live.is_some());
    assert!(runtime.active.is_some());
    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::Progress {
            completed_samples: 11,
            ..
        }))
    ));
}

#[test]
fn poll_rejects_unmappable_window_and_zero_based_last_segment_immediately() {
    for (case, window_base, segment_base, sequence, last_segment_sequence, message) in [
        (
            "window-overflow",
            u64::MAX,
            1,
            2,
            0,
            "live window sequence cannot be mapped",
        ),
        (
            "zero-segment-underflow",
            1,
            0,
            1,
            0,
            "live segment sequence cannot be mapped",
        ),
    ] {
        let root = TestRoot::new(case);
        let job_id = JobId(if window_base == u64::MAX {
            0x99406
        } else {
            0x99407
        });
        let mut runtime = runtime_with_active_attempt(job_id);
        attach_live(&mut runtime, &root.0, job_id);
        let live = runtime.live.as_mut().unwrap();
        live.window_sequence_base = window_base;
        live.segment_sequence_base = segment_base;
        runtime.live_archive = Some(archive_for(&root.0, job_id));
        attach_worker(&mut runtime, &root.0);
        inject(
            &mut runtime,
            window_finished(
                job_id,
                sequence,
                source_range(0, 320),
                last_segment_sequence,
            ),
        );

        assert!(matches!(
            runtime.poll(),
            Some(Err((actual, generation, detail)))
                if actual == job_id
                    && generation == Generation::first()
                    && detail.contains(message)
        ));
        assert!(runtime.live.as_ref().unwrap().failure.is_none());
        assert!(!runtime.live.as_ref().unwrap().stop_requested);
        assert!(runtime.transport.is_some());
        assert!(runtime.active.is_some());
        assert_eq!(
            runtime.live_archive.as_ref().unwrap().confirmed_samples(),
            0
        );
    }
}

#[test]
fn poll_confirms_zero_segment_window_immediately_and_propagates_confirmation_error() {
    let root = TestRoot::new("zero-segment-window");
    let job_id = JobId(0x99408);
    let mut runtime = runtime_with_active_attempt(job_id);
    attach_live(&mut runtime, &root.0, job_id);
    append_durable_samples(&mut runtime, 320);
    runtime.live_archive = Some(archive_for(&root.0, job_id));
    attach_worker(&mut runtime, &root.0);
    inject(
        &mut runtime,
        window_finished(job_id, 1, source_range(0, 320), 0),
    );

    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::WindowFinished {
            job_id: actual,
            window_sequence: 1,
            confirmed_samples: 320,
            ..
        })) if actual == job_id
    ));
    assert_eq!(
        runtime.live_archive.as_ref().unwrap().confirmed_samples(),
        320
    );
    assert_eq!(runtime.live_pending_persistence, 0);
    assert!(runtime.deferred_live_window_finished.is_none());
    assert!(runtime.active.is_some());

    let root = TestRoot::new("window-confirmation-error");
    let job_id = JobId(0x99409);
    let mut runtime = runtime_with_active_attempt(job_id);
    attach_live(&mut runtime, &root.0, job_id);
    runtime.live_archive = Some(archive_for(&root.0, job_id));
    attach_worker(&mut runtime, &root.0);
    inject(
        &mut runtime,
        window_finished(job_id, 1, source_range(0, 320), 0),
    );

    assert!(matches!(
        runtime.poll(),
        Some(Err((actual, generation, message)))
            if actual == job_id
                && generation == Generation::first()
                && message.contains("live window confirmation is not contiguous")
    ));
    let live = runtime.live.as_ref().expect("immediate error retains live");
    assert!(
        live.failure.is_none(),
        "final live error handler was bypassed"
    );
    assert!(!live.stop_requested);
    assert!(runtime.transport.is_some());
    assert!(runtime.active.is_some());
    assert_eq!(
        runtime.live_archive.as_ref().unwrap().confirmed_samples(),
        0
    );
}

#[test]
fn poll_defers_window_confirmation_until_segment_persistence_then_confirms() {
    let root = TestRoot::new("deferred-window-confirmation");
    let job_id = JobId(0x99410);
    let range = source_range(0, 320);
    let segment = WorkerSegment {
        job_id,
        generation: Generation::first(),
        instance_id: INSTANCE_ID,
        segment_id: SegmentId(1),
        range,
        text: "durable live fragment".into(),
    };
    let mut runtime = runtime_with_active_attempt(job_id);
    attach_live(&mut runtime, &root.0, job_id);
    append_durable_samples(&mut runtime, 320);
    runtime.live_archive = Some(archive_for(&root.0, job_id));
    attach_worker(&mut runtime, &root.0);
    inject(
        &mut runtime,
        WorkerEvent::Segment(WorkerSegmentDto {
            job_id,
            generation: Generation::first(),
            instance_id: INSTANCE_ID,
            segment_id: SegmentId(1),
            range,
            text: segment.text.clone(),
        }),
    );
    inject(&mut runtime, window_finished(job_id, 1, range, 1));

    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::LiveProvisional { segment: actual, .. }))
            if actual.text == "durable live fragment"
    ));
    assert_eq!(runtime.live_pending_persistence, 1);
    assert!(runtime.poll().is_none(), "WindowFinished is deferred");
    assert!(runtime.deferred_live_window_finished.is_some());
    assert_eq!(
        runtime.live_archive.as_ref().unwrap().confirmed_samples(),
        0
    );

    let mut events = std::collections::VecDeque::new();
    runtime.effect(ImportEffect::PersistSegment(segment), &mut events);
    assert_eq!(runtime.live_pending_persistence, 0);
    assert!(runtime.deferred_live_window_finished.is_none());
    assert_eq!(
        runtime.live_archive.as_ref().unwrap().confirmed_samples(),
        320
    );
    assert!(matches!(
        events.pop_front(),
        Some(ImportEvent::Persisted {
            job_id: actual,
            sequence: 1,
            ..
        }) if actual == job_id
    ));
    assert!(matches!(
        events.pop_front(),
        Some(ImportEvent::WindowFinished {
            job_id: actual,
            window_sequence: 1,
            confirmed_samples: 320,
            ..
        }) if actual == job_id
    ));
    assert!(events.is_empty());
    assert!(runtime.live.is_some());
    assert!(runtime.active.is_some());
}
