use super::*;
use std::{fs, path::Path, process::Command, thread, time::Duration};
use whisper_core::{
    Generation, ImportEvent, JobId,
    ipc::{WorkerErrorCode, WorkerEvent},
    microphone_level::GainRevision,
};

const INSTANCE_ID: u64 = 7;

fn attach_event(runtime: &mut ImportRuntime, root: &Path, event: WorkerEvent) {
    let worker = compile_idle_worker(root);
    let mut transport = ChildWorkerTransport::spawn(&worker).expect("spawn event worker");
    transport.inject_event(event);
    runtime.transport = Some(transport);
}

fn active_live_runtime(root: &Path, job_id: JobId) -> ImportRuntime {
    let request = live_request(root, job_id);
    let archive = crate::archive::LiveArchive::prepare(&request.destination, request.clone())
        .expect("prepare active live archive");
    let passage = Path::new(&request.destination)
        .join("transcriptions")
        .join(format!("{:032x}", job_id.0))
        .join("live")
        .join("passage-1.pcm");
    let mut live = live_capture_state(root, job_id);
    live.staging = crate::staging::PcmStaging::create(passage)
        .expect("create archive-backed PCM staging")
        .into_writer();
    let mut runtime = runtime_with_active_attempt(job_id);
    runtime.live = Some(live);
    runtime.live_archive = Some(archive);
    runtime.live_capture_events_since_worker_poll = 2;
    runtime
}

fn compile_stop_recording_worker(root: &Path, marker: &Path) -> PathBuf {
    let source = root.join("stop-recorder.rs");
    let executable = root.join("stop-recorder.exe");
    let marker = serde_json::to_string(&marker.to_string_lossy().to_string())
        .expect("encode Stop marker path");
    let program = r#"
use std::{fs, io::{self, BufRead}};
const MARKER: &str = __MARKER__;
fn main() {
    for line in io::stdin().lock().lines().flatten() {
        if line.contains("\"Stop\"") {
            fs::write(MARKER, line).expect("write Stop marker");
            return;
        }
    }
}
"#
    .replace("__MARKER__", &marker);
    fs::write(&source, program).expect("write Stop worker fixture");
    let output = Command::new("rustc")
        .args(["--edition=2021", "--crate-name", "poll_stop_recorder"])
        .arg(&source)
        .arg("-o")
        .arg(&executable)
        .output()
        .expect("compile Stop worker fixture");
    assert!(
        output.status.success(),
        "rustc Stop worker: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    executable
}

fn publish_probe_snapshot(probe: &crate::microphone_probe::MicrophoneProbe) {
    for index in 0..5 {
        let expected_chunks = probe.processed_chunks() + 1;
        probe.submit(crate::capture::ProbeAudioBlock {
            sample_rate_hz: 16_000,
            first_frame: index * 320,
            acquired_at: Instant::now(),
            gain: GainRevision::neutral(),
            mono_pre_gain: vec![0.1; 320],
            mono_post_gain_limiter: vec![0.1; 320],
            limiter_mask: vec![false; 320],
            pre_pcm_limiter_count: 0,
        });
        let deadline = Instant::now() + Duration::from_secs(2);
        while probe.processed_chunks() < expected_chunks && Instant::now() < deadline {
            thread::yield_now();
        }
        assert!(
            probe.processed_chunks() >= expected_chunks,
            "microphone probe did not process the submitted block"
        );
    }
    assert!(probe.latest_at(Instant::now()).is_some());
}

#[test]
fn poll_nonterminal_progress_returns_before_due_live_meter_telemetry() {
    let root = TestRoot::new("progress-live-telemetry");
    let job_id = JobId(0x99801);
    let generation = Generation::first();
    let mut runtime = active_live_runtime(&root.0, job_id);
    attach_event(
        &mut runtime,
        &root.0,
        WorkerEvent::Progress {
            job_id,
            generation,
            instance_id: INSTANCE_ID,
            completed_samples: 120,
            total_samples: 640,
        },
    );
    publish_probe_snapshot(&runtime.live.as_ref().expect("live state").probe);

    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::Progress {
            job_id: actual_job,
            generation: actual_generation,
            completed_samples: 120,
            total_samples: 640,
        })) if actual_job == job_id && actual_generation == generation
    ));
    assert!(
        runtime
            .live
            .as_ref()
            .expect("live remains active")
            .telemetry_due_after_event
    );
    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::MicrophoneMeter(meter)))
            if meter.job_id == job_id && meter.generation == generation
    ));
    assert!(
        !runtime
            .live
            .as_ref()
            .expect("live remains active")
            .telemetry_due_after_event
    );
}

#[test]
fn poll_normal_stopped_releases_pending_attempt_state() {
    let root = TestRoot::new("normal-stopped-cleanup");
    let job_id = JobId(0x99802);
    let generation = Generation::first();
    let mut runtime = runtime_with_active_attempt(job_id);
    let request = live_request(&root.0, job_id);
    let archive = crate::archive::LiveArchive::prepare(&request.destination, request.clone())
        .expect("prepare pending live archive");
    let staging = crate::staging::PcmStaging::create(root.0.join("pending-live.pcm"))
        .expect("create pending live staging")
        .into_writer();
    runtime.pending_live = Some(PendingLiveState {
        request: request.clone(),
        staging,
        live_archive: archive,
        recovery: LiveRecoveryState::None,
    });
    runtime.publication_gate.activate();
    assert_eq!(runtime.publication_gate.request_stop(), Ok(true));
    attach_event(
        &mut runtime,
        &root.0,
        WorkerEvent::Stopped {
            job_id,
            generation,
            instance_id: INSTANCE_ID,
        },
    );

    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::Stopped {
            job_id: actual_job,
            generation: actual_generation,
        })) if actual_job == job_id && actual_generation == generation
    ));
    assert!(runtime.transport.is_none());
    assert!(runtime.active.is_none());
    assert!(runtime.pending_live.is_none());
    assert!(runtime.live_archive.is_none());
    assert!(runtime.live.is_none());
    let lock_path = Path::new(&request.destination)
        .join("transcriptions")
        .join(format!("{:032x}", job_id.0))
        .join("live")
        .join(format!("group-{:032x}.writer.lock", job_id.0));
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(lock_path)
        .expect("open released live archive writer lock");
    assert!(
        lock.try_lock().is_ok(),
        "normal Stopped releases the pending live archive writer lock"
    );
}

#[test]
fn poll_correlated_failed_returns_error_and_releases_attempt() {
    let root = TestRoot::new("correlated-failed");
    let job_id = JobId(0x99803);
    let generation = Generation::first();
    let mut runtime = runtime_with_active_attempt(job_id);
    attach_event(
        &mut runtime,
        &root.0,
        WorkerEvent::Failed {
            job_id: Some(job_id),
            generation: Some(generation),
            instance_id: Some(INSTANCE_ID),
            code: WorkerErrorCode::WorkerExited,
            message: "correlated worker failure".into(),
        },
    );

    assert!(matches!(
        runtime.poll(),
        Some(Err((actual_job, actual_generation, message)))
            if actual_job == job_id
                && actual_generation == generation
                && message == "correlated worker failure"
    ));
    assert!(runtime.active.is_none());
    assert!(runtime.transport.is_none());
    assert!(runtime.pending_live.is_none());
    assert!(runtime.live_archive.is_none());
}

#[test]
fn poll_unexpected_identity_free_event_returns_stale_response() {
    let root = TestRoot::new("unexpected-worker-event");
    let job_id = JobId(0x99804);
    let generation = Generation::first();
    let mut runtime = runtime_with_active_attempt(job_id);
    attach_event(&mut runtime, &root.0, WorkerEvent::CudaAvailable);

    assert!(matches!(
        runtime.poll(),
        Some(Err((actual_job, actual_generation, message)))
            if actual_job == job_id
                && actual_generation == generation
                && message == "StaleResponse: worker event identity mismatch"
    ));
    assert!(
        runtime
            .active
            .as_ref()
            .is_some_and(|active| active.job_id == job_id)
    );
    assert!(runtime.transport.is_some());
}

#[test]
fn poll_live_failed_drains_and_sends_stop_before_deferring_failure() {
    let root = TestRoot::new("live-failed-deferred");
    let job_id = JobId(0x99805);
    let generation = Generation::first();
    let failure = "synthetic live inference failure";
    let marker = root.0.join("stop-command.json");
    let worker = compile_stop_recording_worker(&root.0, &marker);
    let mut runtime = active_live_runtime(&root.0, job_id);
    publish_probe_snapshot(&runtime.live.as_ref().expect("active live state").probe);
    assert!(
        runtime
            .live
            .as_ref()
            .expect("active live state")
            .probe
            .latest_at(Instant::now())
            .is_some(),
        "probe snapshot is valid before the live failure"
    );
    let live = runtime.live.as_mut().expect("active live state");
    live.staging
        .append_frame(vec![0; 320])
        .expect("queue captured PCM before failure");
    live.captured = 320;
    live.admitted = 320;
    let mut transport = ChildWorkerTransport::spawn(&worker).expect("spawn Stop recorder");
    transport.inject_event(WorkerEvent::Failed {
        job_id: Some(job_id),
        generation: Some(generation),
        instance_id: Some(INSTANCE_ID),
        code: WorkerErrorCode::InferenceFailed,
        message: failure.into(),
    });
    runtime.transport = Some(transport);

    assert!(runtime.poll().is_none());
    let live = runtime.live.as_ref().expect("live failure is deferred");
    assert!(!live.stop_requested);
    assert!(live.probe.latest_at(Instant::now()).is_none());
    assert_eq!(live.failure.as_deref(), Some(failure));
    assert_eq!(live.captured, 320);
    assert_eq!(live.admitted, 320);
    assert_eq!(live.staging.durable_samples(), 320);
    assert!(runtime.transport.is_some());
    assert!(runtime.active.is_some());
    assert!(runtime.live_archive.is_some());

    let deadline = Instant::now() + Duration::from_secs(3);
    while !marker.exists() && Instant::now() < deadline {
        thread::yield_now();
    }
    let command = fs::read_to_string(&marker).expect("worker received the Stop command");
    assert!(
        command.contains("\"Stop\""),
        "unexpected worker command: {command}"
    );
    let status = loop {
        if let Some(status) = runtime
            .transport
            .as_mut()
            .expect("transport remains until terminal event")
            .try_wait()
            .expect("check Stop worker exit")
        {
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "Stop worker did not exit in time"
        );
        thread::yield_now();
    };
    assert!(
        status.success(),
        "Stop worker exited unsuccessfully: {status}"
    );
    runtime
        .transport
        .as_mut()
        .expect("transport remains until terminal event")
        .inject_event(WorkerEvent::Stopped {
            job_id,
            generation,
            instance_id: INSTANCE_ID,
        });
    assert!(matches!(
        runtime.poll(),
        Some(Err((actual_job, actual_generation, message)))
            if actual_job == job_id
                && actual_generation == generation
                && message == failure
    ));
    assert!(runtime.live.is_none());
    assert!(runtime.transport.is_none());
    assert!(runtime.active.is_none());
    assert!(runtime.live_archive.is_none());
}
