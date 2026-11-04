use super::*;
use std::{
    fs,
    process::Command,
    thread,
    time::{Duration, Instant},
};
use whisper_core::{
    SegmentId, SourceRange,
    ipc::WorkerSegmentDto,
    ports::{ImportEffect, WorkerSegment},
};

fn compile_exiting_transport(root: &Path) -> PathBuf {
    let source = root.join("exiting-transport.rs");
    let executable = root.join("exiting-transport.exe");
    fs::write(&source, "fn main() {}").expect("write exiting worker");
    let output = Command::new("rustc")
        .args(["--edition=2021", "--crate-name", "poll_exiting_transport"])
        .arg(&source)
        .arg("-o")
        .arg(&executable)
        .output()
        .expect("compile exiting worker");
    assert!(
        output.status.success(),
        "rustc exiting worker: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    executable
}

fn active_live_runtime(root: &Path, job_id: JobId, failure: &str) -> ImportRuntime {
    let request = live_request(root, job_id);
    let archive = crate::archive::LiveArchive::prepare(&request.destination, request.clone())
        .expect("prepare live archive");
    let passage = Path::new(&request.destination)
        .join("transcriptions")
        .join(format!("{:032x}", job_id.0))
        .join("live")
        .join("passage-1.pcm");
    let mut live = live_capture_state(root, job_id);
    live.staging = crate::staging::PcmStaging::create(passage)
        .expect("create archive-backed staging")
        .into_writer();
    live.failure = Some(failure.into());
    let mut runtime = runtime_with_active_attempt(job_id);
    runtime.live = Some(live);
    runtime.live_archive = Some(archive);
    runtime.live_capture_events_since_worker_poll = 2;
    runtime
}

fn attach_events_to_transport(
    runtime: &mut ImportRuntime,
    root: &Path,
    events: impl IntoIterator<Item = WorkerEvent>,
    exit_before_poll: bool,
) {
    let worker = compile_idle_worker(root);
    let mut transport = crate::worker_ipc::ipc::ChildWorkerTransport::spawn(&worker)
        .expect("spawn synthetic event transport");
    if exit_before_poll {
        transport.abort();
        assert!(transport.try_wait().expect("query exited worker").is_some());
    }
    for event in events {
        transport.inject_event(event);
    }
    runtime.transport = Some(transport);
}

#[test]
fn poll_transport_absence_returns_none_without_releasing_active_attempt() {
    let root = TestRoot::new("transport-none");
    let worker = compile_idle_worker(&root.0);
    let mut runtime = runtime_with_active_attempt(JobId(73_301));
    runtime.transport = Some(
        crate::worker_ipc::ipc::ChildWorkerTransport::spawn(&worker)
            .expect("spawn quiet worker transport"),
    );

    assert!(runtime.poll().is_none());
    assert!(runtime.transport.is_some());
    assert!(runtime.active.is_some());
}

#[test]
fn poll_transport_error_without_recovery_releases_runtime_attempt() {
    let root = TestRoot::new("transport-error");
    let worker = compile_exiting_transport(&root.0);
    let job_id = JobId(73_302);
    let mut runtime = runtime_with_active_attempt(job_id);
    runtime.transport = Some(
        crate::worker_ipc::ipc::ChildWorkerTransport::spawn(&worker)
            .expect("spawn exiting worker transport"),
    );

    let exit_deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if runtime
            .transport
            .as_mut()
            .expect("transport remains until poll handles exit")
            .try_wait()
            .expect("query child status")
            .is_some()
        {
            break;
        }
        assert!(Instant::now() < exit_deadline, "fake worker did not exit");
        thread::sleep(Duration::from_millis(5));
    }

    let poll_deadline = Instant::now() + Duration::from_secs(3);
    let mut failure = None;
    while Instant::now() < poll_deadline {
        if let Some(result) = runtime.poll() {
            failure = Some(result);
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }
    assert!(matches!(
        failure,
        Some(Err((actual, _, message)))
            if actual == job_id && message.contains("WorkerExited without End")
    ));
    assert!(runtime.transport.is_none());
    assert!(runtime.active.is_none());
    assert!(runtime.live.is_none());
    assert!(runtime.live_archive.is_none());
}

#[test]
fn poll_stopped_live_waits_for_pending_segment_persistence_before_cleanup() {
    let root = TestRoot::new("stopped-live-persistence-gate");
    let job_id = JobId(73_303);
    let generation = Generation::first();
    let failure = "StorageUnavailable: controlled live failure";
    let range = SourceRange::new(0, 320, 16_000).expect("valid live segment range");
    let mut runtime = active_live_runtime(&root.0, job_id, failure);
    runtime
        .live
        .as_ref()
        .expect("active live capture")
        .staging
        .append_frame(vec![0; 320])
        .expect("append durable segment audio");
    runtime
        .live
        .as_mut()
        .expect("active live capture")
        .staging
        .drain()
        .expect("durably stage segment audio");
    let segment = WorkerSegment {
        job_id,
        generation,
        instance_id: 7,
        segment_id: SegmentId(1),
        range,
        text: "durable fragment".into(),
    };
    attach_events_to_transport(
        &mut runtime,
        &root.0,
        [
            WorkerEvent::Segment(WorkerSegmentDto {
                job_id,
                generation,
                instance_id: 7,
                segment_id: SegmentId(1),
                range,
                text: segment.text.clone(),
            }),
            WorkerEvent::Stopped {
                job_id,
                generation,
                instance_id: 7,
            },
        ],
        true,
    );

    let provisional = runtime.poll().expect("collect provisional segment");
    assert!(matches!(
        provisional,
        Ok(ImportEvent::LiveProvisional { segment: actual, .. }) if actual == segment
    ));
    assert_eq!(runtime.live_pending_persistence, 1);
    assert!(
        runtime.poll().is_none(),
        "Stopped waits for its pending ACK"
    );
    assert!(matches!(
        runtime.deferred_live_stopped.as_ref(),
        Some(WorkerEvent::Stopped { job_id: actual, .. }) if *actual == job_id
    ));
    assert!(runtime.live.is_some());
    assert!(runtime.live_archive.is_some());
    assert!(runtime.active.is_some());
    assert!(
        !root
            .0
            .join("archive")
            .join("transcriptions")
            .join(format!("{:032x}", job_id.0))
            .join("live")
            .join("CURRENT")
            .exists()
    );

    let mut persisted = std::collections::VecDeque::new();
    runtime.effect(ImportEffect::PersistSegment(segment), &mut persisted);
    assert_eq!(runtime.live_pending_persistence, 0);
    assert!(matches!(
        persisted.pop_front(),
        Some(ImportEvent::Persisted { job_id: actual, sequence: 1, .. }) if actual == job_id
    ));
    assert!(matches!(
        runtime.poll(),
        Some(Err((actual, actual_generation, message)))
            if actual == job_id && actual_generation == generation && message == failure
    ));
    assert!(runtime.deferred_live_stopped.is_none());
    assert!(runtime.live.is_none());
    assert!(runtime.live_archive.is_none());
    assert!(runtime.transport.is_none());
    assert!(runtime.active.is_none());
}

#[test]
fn poll_stopped_live_waits_for_child_exit_before_cleanup() {
    let root = TestRoot::new("stopped-live-process-gate");
    let job_id = JobId(73_304);
    let generation = Generation::first();
    let failure = "StorageUnavailable: controlled live failure";
    let mut runtime = active_live_runtime(&root.0, job_id, failure);
    attach_events_to_transport(
        &mut runtime,
        &root.0,
        [WorkerEvent::Stopped {
            job_id,
            generation,
            instance_id: 7,
        }],
        false,
    );

    assert!(runtime.poll().is_none(), "Stopped waits for the live child");
    assert!(matches!(
        runtime.deferred_live_stopped.as_ref(),
        Some(WorkerEvent::Stopped { job_id: actual, .. }) if *actual == job_id
    ));
    assert!(runtime.live.is_some());
    assert!(runtime.live_archive.is_some());
    assert!(runtime.active.is_some());
    assert!(runtime.transport.is_some());
    assert!(
        !root
            .0
            .join("archive")
            .join("transcriptions")
            .join(format!("{:032x}", job_id.0))
            .join("live")
            .join("CURRENT")
            .exists()
    );

    runtime.transport.as_mut().expect("live transport").abort();
    assert!(
        runtime
            .transport
            .as_mut()
            .expect("exited live transport remains owned")
            .try_wait()
            .expect("query child exit")
            .is_some()
    );
    assert!(matches!(
        runtime.poll(),
        Some(Err((actual, actual_generation, message)))
            if actual == job_id && actual_generation == generation && message == failure
    ));
    assert!(runtime.deferred_live_stopped.is_none());
    assert!(runtime.live.is_none());
    assert!(runtime.live_archive.is_none());
    assert!(runtime.transport.is_none());
    assert!(runtime.active.is_none());
}
