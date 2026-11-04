use super::*;
use std::collections::VecDeque;
use whisper_core::{
    SegmentId, SourceRange,
    ipc::WorkerSegmentDto,
    ports::{ImportEffect, WorkerSegment},
};

const INSTANCE_ID: u64 = 7;

fn source_range(start: u64, end: u64) -> SourceRange {
    SourceRange::new(start, end, 16_000).expect("valid source range")
}

fn attach_live(runtime: &mut ImportRuntime, root: &Path, job_id: JobId) {
    let live = super::live_capture_state(root, job_id);
    let request = live.request.clone();
    runtime.live = Some(live);
    runtime.live_archive = Some(
        crate::archive::LiveArchive::prepare(request.destination.clone(), request)
            .expect("prepare live archive"),
    );
}

fn attach_worker(runtime: &mut ImportRuntime, root: &Path) {
    let worker = super::compile_idle_worker(root);
    runtime.transport = Some(ChildWorkerTransport::spawn(&worker).expect("spawn idle worker"));
}

fn runtime_with_durable_live_prefix(root: &TestRoot, job_id: JobId) -> ImportRuntime {
    let mut runtime = runtime_with_active_attempt(job_id);
    attach_live(&mut runtime, &root.0, job_id);
    attach_worker(&mut runtime, &root.0);
    for expected_captured in [0, 320] {
        runtime
            .live
            .as_mut()
            .expect("live capture")
            .capture
            .enqueue_test_block(vec![0.1; 320]);
        let event = runtime.poll();
        assert!(matches!(
            event,
            None | Some(Ok(
                ImportEvent::MicrophoneSpectrum(_) | ImportEvent::MicrophoneMeter(_)
            ))
        ));
        let live = runtime.live.as_ref().expect("live capture");
        assert_eq!(live.captured, expected_captured);
        assert_eq!(live.admitted, expected_captured);
        assert_eq!(live.staging.durable_samples(), 0);
    }
    let live = runtime.live.as_mut().expect("live capture");
    assert_eq!(live.staging.drain().expect("persist captured prefix"), 320);
    assert!(live.captured >= live.staging.durable_samples());
    runtime
}

fn persist_segment_and_assert_confirmation(
    runtime: &mut ImportRuntime,
    segment: WorkerSegment,
    job_id: JobId,
) {
    let mut effect_events = VecDeque::new();
    runtime.effect(ImportEffect::PersistSegment(segment), &mut effect_events);
    assert_eq!(runtime.live_pending_persistence, 0);
    assert!(runtime.deferred_live_window_finished.is_none());
    assert_eq!(
        runtime.live_archive.as_ref().unwrap().confirmed_samples(),
        320
    );
    assert!(matches!(
        effect_events.pop_front(),
        Some(ImportEvent::Persisted { job_id: actual, sequence: 1, .. }) if actual == job_id
    ));
    assert!(matches!(
        effect_events.pop_front(),
        Some(ImportEvent::WindowFinished {
            job_id: actual,
            window_sequence: 1,
            confirmed_samples: 320,
            ..
        }) if actual == job_id
    ));
    assert!(effect_events.is_empty());
}

#[test]
fn poll_preserves_queued_worker_event_until_deferred_window_is_persisted() {
    let root = TestRoot::new("deferred-window-poll-event");
    let job_id = JobId(0x99701);
    let generation = Generation::first();
    let range = source_range(0, 320);
    let segment = WorkerSegment {
        job_id,
        generation,
        instance_id: INSTANCE_ID,
        segment_id: SegmentId(1),
        range,
        text: "persisted before window confirmation".into(),
    };
    let mut runtime = runtime_with_durable_live_prefix(&root, job_id);

    runtime
        .transport
        .as_mut()
        .expect("worker transport")
        .inject_event(WorkerEvent::Segment(WorkerSegmentDto {
            job_id,
            generation,
            instance_id: INSTANCE_ID,
            segment_id: SegmentId(1),
            range,
            text: segment.text.clone(),
        }));
    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::LiveProvisional { segment: actual, .. }))
            if actual.text == segment.text
    ));
    assert_eq!(runtime.live_pending_persistence, 1);

    runtime
        .transport
        .as_mut()
        .expect("worker transport")
        .inject_event(WorkerEvent::WindowFinished {
            job_id,
            generation,
            instance_id: INSTANCE_ID,
            sequence: 1,
            range,
            last_segment_sequence: 1,
        });
    assert!(runtime.poll().is_none(), "WindowFinished is deferred");
    assert!(runtime.deferred_live_window_finished.is_some());
    assert_eq!(
        runtime.live_archive.as_ref().unwrap().confirmed_samples(),
        0
    );

    runtime
        .transport
        .as_mut()
        .expect("worker transport")
        .inject_event(WorkerEvent::Progress {
            job_id,
            generation,
            instance_id: INSTANCE_ID,
            completed_samples: 160,
            total_samples: 320,
        });
    assert!(
        runtime.poll().is_none(),
        "deferred confirmation blocks polling"
    );
    assert_eq!(runtime.live_pending_persistence, 1);
    assert!(runtime.deferred_live_window_finished.is_some());
    assert_eq!(
        runtime.live_archive.as_ref().unwrap().confirmed_samples(),
        0,
        "worker event cannot confirm the window before the persistence ACK"
    );
    assert_eq!(runtime.live.as_ref().unwrap().captured, 320);
    assert_eq!(
        runtime.live.as_ref().unwrap().staging.durable_samples(),
        320
    );

    persist_segment_and_assert_confirmation(&mut runtime, segment, job_id);

    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::Progress {
            job_id: actual,
            completed_samples: 160,
            total_samples: 320,
            ..
        })) if actual == job_id
    ));
}
