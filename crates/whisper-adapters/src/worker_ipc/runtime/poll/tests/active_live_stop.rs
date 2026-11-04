use super::*;
use std::sync::atomic::AtomicBool;
use whisper_core::{ComputeChoice, SourceRange};

const INSTANCE_ID: u64 = 7;
const PCM_SAMPLES: usize = 320;

fn live_archive_directory(root: &Path, job_id: JobId) -> PathBuf {
    root.join("archive")
        .join("transcriptions")
        .join(format!("{:032x}", job_id.0))
        .join("live")
}

fn passage_pcm_path(root: &Path, job_id: JobId) -> PathBuf {
    live_archive_directory(root, job_id).join("passage-1.pcm")
}

fn attempts_path(root: &Path, job_id: JobId) -> PathBuf {
    live_archive_directory(root, job_id).join("passage-1.attempts.jsonl")
}

fn attempt_states(journal: &str, attempt: u64) -> Vec<String> {
    journal
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("valid attempt record"))
        .filter_map(|record| {
            let payload = record.get("payload")?;
            (payload.get("attempt")?.as_u64()? == attempt)
                .then(|| payload.get("state")?.as_str().map(str::to_owned))
                .flatten()
        })
        .collect()
}

fn active_live_runtime(root: &Path, job_id: JobId) -> ImportRuntime {
    let request = live_request(root, job_id);
    let archive = crate::archive::LiveArchive::prepare(&request.destination, request.clone())
        .expect("prepare active live archive");
    let mut live = live_capture_state(root, job_id);
    live.staging = crate::staging::PcmStaging::create(passage_pcm_path(root, job_id))
        .expect("create archive-backed PCM staging")
        .into_writer();
    let mut runtime = runtime_with_active_attempt(job_id);
    runtime.live = Some(live);
    runtime.live_archive = Some(archive);
    runtime.live_capture_events_since_worker_poll = 2;
    runtime
}

fn start_gpu_attempt(runtime: &mut ImportRuntime, choice: ComputeChoice) -> u64 {
    let archive = runtime.live_archive.as_mut().expect("live archive");
    let attempt = archive.reserve_attempt().expect("reserve GPU attempt");
    archive
        .mark_attempt_started(attempt)
        .expect("mark GPU attempt started");
    let active = runtime.active.as_mut().expect("active GPU attempt");
    active.backend = BackendKind::Cuda;
    active.attempt_number = attempt;
    let live = runtime.live.as_mut().expect("live capture");
    live.request.config.compute = choice;
    live.recovery = LiveRecoveryState::AutoRetryPending;
    live.inference_admission_closed = true;
    live.barrier_m = Some(live.inference_sequence.saturating_sub(1));
    attempt
}

fn confirm_prefix(runtime: &mut ImportRuntime, samples: usize) {
    let live = runtime.live.as_mut().expect("live capture");
    live.staging
        .append_frame(vec![0; samples])
        .expect("append actual PCM frame");
    live.staging.drain().expect("make PCM frame durable");
    let range = SourceRange::new(0, samples as u64, 16_000).expect("valid prefix range");
    runtime
        .live_archive
        .as_mut()
        .expect("live archive")
        .persist_window_finished(1, range, 0, samples as u64)
        .expect("persist coverage for durable prefix");
    runtime
        .live
        .as_mut()
        .expect("live capture")
        .confirmed_fragment_end = samples as u64;
}

fn compile_exited_worker(root: &Path) -> PathBuf {
    let source = root.join("exited-live-worker.rs");
    let executable = root.join("exited-live-worker.exe");
    fs::write(&source, "fn main() {}").expect("write exited worker fixture");
    let output = Command::new("rustc")
        .args(["--edition=2021", "--crate-name", "poll_exited_live_worker"])
        .arg(&source)
        .arg("-o")
        .arg(&executable)
        .output()
        .expect("compile exited worker fixture");
    assert!(
        output.status.success(),
        "rustc exited worker: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    executable
}

fn attach_exited_worker_stopped(runtime: &mut ImportRuntime, root: &Path, job_id: JobId) {
    let mut transport = ChildWorkerTransport::spawn(&compile_exited_worker(root))
        .expect("spawn exited worker fixture");
    let deadline = Instant::now() + Duration::from_secs(3);
    while transport.try_wait().expect("poll worker exit").is_none() {
        assert!(Instant::now() < deadline, "fixture worker did not exit");
        std::thread::sleep(Duration::from_millis(2));
    }
    transport.inject_event(WorkerEvent::Stopped {
        job_id,
        generation: Generation::first(),
        instance_id: INSTANCE_ID,
    });
    runtime.transport = Some(transport);
}

fn archive_current_path(root: &Path, job_id: JobId) -> PathBuf {
    live_archive_directory(root, job_id).join("CURRENT")
}

#[test]
fn poll_stopped_live_failure_keeps_archive_lock_until_writer_shutdown() {
    let root = TestRoot::new("active-live-failure-order");
    let job_id = JobId(0x99601);
    let mut runtime = active_live_runtime(&root.0, job_id);
    let request = runtime.live.as_ref().unwrap().request.clone();
    runtime.live.as_mut().unwrap().failure = Some("StorageUnavailable: saved capture error".into());
    let lock_held_at_writer_shutdown = Arc::new(AtomicBool::new(false));
    let observed = Arc::clone(&lock_held_at_writer_shutdown);
    runtime
        .live
        .as_mut()
        .unwrap()
        .staging
        .observe_shutdown(move || {
            let result =
                crate::archive::LiveArchive::prepare(&request.destination, request.clone());
            observed.store(
                result.as_ref().is_err_and(|error| {
                    error.contains("another live archive writer owns the passage")
                }),
                Ordering::Release,
            );
        });
    attach_exited_worker_stopped(&mut runtime, &root.0, job_id);

    let result = runtime.poll().expect("process Stopped event");
    assert!(matches!(
        result,
        Err((actual, generation, message))
            if actual == job_id
                && generation == Generation::first()
                && message == "StorageUnavailable: saved capture error"
    ));
    assert!(
        lock_held_at_writer_shutdown.load(Ordering::Acquire),
        "live archive lock must remain held until its PCM writer has joined"
    );
    assert!(runtime.live.is_none());
    assert!(runtime.live_archive.is_none());
    assert!(runtime.transport.is_none());
    assert!(runtime.active.is_none());
}

#[test]
fn poll_stopped_live_drain_checkpoint_failure_cleans_runtime_state() {
    let root = TestRoot::new("active-live-drain-error");
    let job_id = JobId(0x99602);
    let mut runtime = active_live_runtime(&root.0, job_id);
    let checkpoint = passage_pcm_path(&root.0, job_id).with_file_name("passage-1.pcm.state.json");
    fs::remove_file(&checkpoint).expect("remove checkpoint file for rename failure fixture");
    fs::create_dir(&checkpoint).expect("replace checkpoint target with directory");
    attach_exited_worker_stopped(&mut runtime, &root.0, job_id);

    let result = runtime.poll().expect("process Stopped event");
    assert!(matches!(
        result,
        Err((actual, generation, message))
            if actual == job_id
                && generation == Generation::first()
                && message.contains("StorageUnavailable: final audio sync:")
    ));
    assert!(runtime.live.is_none());
    assert!(runtime.live_archive.is_none());
    assert!(runtime.transport.is_none());
    assert!(runtime.active.is_none());
    assert!(!archive_current_path(&root.0, job_id).exists());
}

#[test]
fn poll_stopped_live_attempt_journal_io_failure_cleans_runtime_state() {
    let root = TestRoot::new("active-live-retire-error");
    let job_id = JobId(0x99603);
    let mut runtime = active_live_runtime(&root.0, job_id);
    start_gpu_attempt(&mut runtime, ComputeChoice::Auto);
    let journal = attempts_path(&root.0, job_id);
    fs::remove_file(&journal).expect("remove attempt journal for I/O fixture");
    fs::create_dir(&journal).expect("replace attempt journal with directory");
    attach_exited_worker_stopped(&mut runtime, &root.0, job_id);

    let result = runtime.poll().expect("process Stopped event");
    assert!(matches!(
        result,
        Err((actual, generation, message))
            if actual == job_id
                && generation == Generation::first()
                && message.contains("StorageUnavailable: attempt journal:")
    ));
    assert!(runtime.live.is_none());
    assert!(runtime.live_archive.is_none());
    assert!(runtime.transport.is_none());
    assert!(runtime.active.is_none());
    assert!(!archive_current_path(&root.0, job_id).exists());
}

#[test]
fn poll_stopped_live_gate_cancellation_retires_without_cpu_retry() {
    let root = TestRoot::new("active-live-gate-cancel");
    let job_id = JobId(0x99604);
    let mut runtime = active_live_runtime(&root.0, job_id);
    start_gpu_attempt(&mut runtime, ComputeChoice::Auto);
    runtime.publication_gate.activate();
    assert_eq!(runtime.publication_gate.request_stop(), Ok(true));
    attach_exited_worker_stopped(&mut runtime, &root.0, job_id);

    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::Stopped { job_id: actual, generation }))
            if actual == job_id && generation == Generation::first()
    ));
    assert!(runtime.live.is_none());
    assert!(runtime.live_archive.is_none());
    assert!(runtime.transport.is_none());
    assert!(runtime.active.is_none());
    assert!(!archive_current_path(&root.0, job_id).exists());
}

#[test]
fn poll_stopped_live_awaiting_choice_keeps_live_archive_without_active_worker() {
    let root = TestRoot::new("active-live-awaiting-choice");
    let job_id = JobId(0x99605);
    let mut runtime = active_live_runtime(&root.0, job_id);
    start_gpu_attempt(&mut runtime, ComputeChoice::Gpu);
    runtime.live.as_mut().unwrap().recovery = LiveRecoveryState::AwaitingChoicePending;
    attach_exited_worker_stopped(&mut runtime, &root.0, job_id);

    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::AwaitingCpuChoice { job_id: actual, generation }))
            if actual == job_id && generation == Generation::first()
    ));
    let live = runtime
        .live
        .as_ref()
        .expect("live capture remains for CPU choice");
    assert_eq!(live.recovery, LiveRecoveryState::AwaitingChoice);
    assert!(live.inference_admission_closed);
    assert!(runtime.live_archive.is_some());
    assert!(runtime.transport.is_none());
    assert!(runtime.active.is_none());
}

#[test]
fn poll_stopped_live_latched_stop_skips_auto_cpu_retry() {
    let root = TestRoot::new("active-live-stop-latched");
    let job_id = JobId(0x99606);
    let mut runtime = active_live_runtime(&root.0, job_id);
    start_gpu_attempt(&mut runtime, ComputeChoice::Auto);
    runtime.live.as_mut().unwrap().stop_requested = true;
    attach_exited_worker_stopped(&mut runtime, &root.0, job_id);

    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::Stopped { job_id: actual, generation }))
            if actual == job_id && generation == Generation::first()
    ));
    assert!(runtime.live.is_none());
    assert!(runtime.live_archive.is_none());
    assert!(runtime.transport.is_none());
    assert!(runtime.active.is_none());
}

fn compile_live_cpu_marker_worker(root: &Path) -> (PathBuf, PathBuf) {
    let marker = root.join("live-cpu-worker-started");
    let source = root.join("live-cpu-worker.rs");
    let executable = root.join("live-cpu-worker.exe");
    let marker_literal = serde_json::to_string(&marker.to_string_lossy().to_string())
        .expect("encode worker marker path");
    let program = r#"
use std::{fs, io::{self, BufRead}};
const MARKER: &str = __MARKER__;
fn main() {
    if io::stdin().lock().lines().next().is_some() {
        fs::write(MARKER, "CPU live retry started").expect("write CPU start marker");
    }
}
"#
    .replace("__MARKER__", &marker_literal);
    fs::write(&source, program).expect("write CPU marker worker fixture");
    let output = Command::new("rustc")
        .args(["--edition=2021", "--crate-name", "active_live_cpu_worker"])
        .arg(&source)
        .arg("-o")
        .arg(&executable)
        .output()
        .expect("compile CPU marker worker fixture");
    assert!(
        output.status.success(),
        "rustc CPU marker worker: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    (executable, marker)
}

#[test]
fn poll_stopped_live_auto_retry_starts_cpu_and_records_next_attempt() {
    let root = TestRoot::new("active-live-auto-cpu");
    let job_id = JobId(0x99607);
    let mut runtime = active_live_runtime(&root.0, job_id);
    start_gpu_attempt(&mut runtime, ComputeChoice::Auto);
    let (cpu, marker) = compile_live_cpu_marker_worker(&root.0);
    runtime.workers.cpu = cpu;
    attach_exited_worker_stopped(&mut runtime, &root.0, job_id);

    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::LiveAttemptStarting { job_id: actual, generation }))
            if actual == job_id && generation == Generation::first()
    ));
    assert!(runtime.active.as_ref().is_some_and(|active| {
        active.backend == BackendKind::Cpu && active.attempt_number == 2
    }));
    assert!(runtime.transport.is_some());
    let deadline = Instant::now() + Duration::from_secs(3);
    while !marker.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(
        marker.exists(),
        "CPU live worker did not receive retry start"
    );
    let journal = fs::read_to_string(attempts_path(&root.0, job_id))
        .expect("read active live attempt journal");
    assert_eq!(
        attempt_states(&journal, 1),
        vec!["reserved", "started", "retired"]
    );
    assert_eq!(attempt_states(&journal, 2), vec!["reserved", "started"]);
    let live = runtime.live.as_ref().expect("live state after CPU retry");
    assert_eq!(live.recovery, LiveRecoveryState::None);
    assert!(!live.inference_admission_closed);
    assert!(live.barrier_m.is_none());
}

#[test]
fn poll_stopped_live_auto_retry_missing_cpu_reports_error_after_retirement() {
    let root = TestRoot::new("active-live-missing-cpu");
    let job_id = JobId(0x99608);
    let mut runtime = active_live_runtime(&root.0, job_id);
    start_gpu_attempt(&mut runtime, ComputeChoice::Auto);
    runtime.workers.cpu = root.0.join("missing-cpu-worker.exe");
    attach_exited_worker_stopped(&mut runtime, &root.0, job_id);

    let result = runtime.poll().expect("process Stopped event");
    assert!(matches!(
        result,
        Err((actual, generation, message))
            if actual == job_id
                && generation == Generation::first()
                && message.contains("CPU fallback unavailable")
    ));
    assert!(runtime.live.as_ref().is_some_and(|live| {
        live.recovery == LiveRecoveryState::AutoRetryPending && live.inference_admission_closed
    }));
    assert!(runtime.live_archive.is_some());
    assert!(runtime.transport.is_none());
    assert!(runtime.active.is_none());
    let journal = fs::read_to_string(attempts_path(&root.0, job_id))
        .expect("read attempt journal after CPU selection failure");
    assert_eq!(
        attempt_states(&journal, 1),
        vec!["reserved", "started", "retired"]
    );
}

#[test]
fn poll_stopped_live_publishes_confirmed_durable_audio() {
    let root = TestRoot::new("active-live-publish-success");
    let job_id = JobId(0x99609);
    let mut runtime = active_live_runtime(&root.0, job_id);
    let attempt = runtime
        .live_archive
        .as_mut()
        .unwrap()
        .reserve_attempt()
        .unwrap();
    runtime
        .live_archive
        .as_mut()
        .unwrap()
        .mark_attempt_started(attempt)
        .unwrap();
    runtime.active.as_mut().unwrap().attempt_number = attempt;
    runtime.live.as_mut().unwrap().stop_requested = true;
    confirm_prefix(&mut runtime, PCM_SAMPLES);
    attach_exited_worker_stopped(&mut runtime, &root.0, job_id);

    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::Published { job_id: actual, generation }))
            if actual == job_id && generation == Generation::first()
    ));
    let archive = live_archive_directory(&root.0, job_id);
    assert_eq!(
        fs::read_to_string(archive.join("CURRENT")).unwrap(),
        "manifest-1.json\n"
    );
    assert!(archive.join("manifest-1.json").is_file());
    assert!(archive.join("passage-1.pcm").is_file());
    assert!(archive.join("passage-1.mp3").is_file());
    assert!(archive.join("transcript-1.txt").is_file());
    assert!(archive.join("transcript-1.srt").is_file());
    assert!(runtime.live.is_none());
    assert!(runtime.live_archive.is_none());
    assert!(runtime.transport.is_none());
    assert!(runtime.active.is_none());
}

#[test]
fn poll_stopped_live_publication_error_cleans_runtime_state() {
    let root = TestRoot::new("active-live-publish-error");
    let job_id = JobId(0x99610);
    let mut runtime = active_live_runtime(&root.0, job_id);
    confirm_prefix(&mut runtime, PCM_SAMPLES);
    let conflicting_text_path = live_archive_directory(&root.0, job_id).join("transcript-1.txt");
    fs::create_dir(&conflicting_text_path).expect("create transcript publication conflict");
    attach_exited_worker_stopped(&mut runtime, &root.0, job_id);

    let result = runtime.poll().expect("process Stopped event");
    assert!(matches!(
        result,
        Err((actual, generation, message))
            if actual == job_id && generation == Generation::first() && !message.is_empty()
    ));
    assert!(conflicting_text_path.is_dir());
    assert!(!archive_current_path(&root.0, job_id).exists());
    assert!(runtime.live.is_none());
    assert!(runtime.live_archive.is_none());
    assert!(runtime.transport.is_none());
    assert!(runtime.active.is_none());
}

#[test]
fn poll_stopped_live_defensive_missing_barrier_rejects_without_publication() {
    let root = TestRoot::new("active-live-defensive-missing-barrier");
    let job_id = JobId(0x99611);
    let mut runtime = active_live_runtime(&root.0, job_id);
    start_gpu_attempt(&mut runtime, ComputeChoice::Auto);
    runtime.live.as_mut().unwrap().barrier_m = None;
    attach_exited_worker_stopped(&mut runtime, &root.0, job_id);

    let result = runtime
        .poll()
        .expect("process defensive invalid-barrier state");
    assert!(matches!(
        result,
        Err((actual, generation, message))
            if actual == job_id && generation == Generation::first()
                && message.contains("StorageCorrupt: retry barrier")
    ));
    assert!(!archive_current_path(&root.0, job_id).exists());
    assert!(runtime.live.is_none());
    assert!(runtime.live_archive.is_none());
    assert!(runtime.transport.is_none());
    assert!(runtime.active.is_none());
}

#[test]
fn poll_stopped_live_defensive_barrier_past_sequence_rejects_publishable_prefix() {
    let root = TestRoot::new("active-live-defensive-barrier-high");
    let job_id = JobId(0x99612);
    let mut runtime = active_live_runtime(&root.0, job_id);
    start_gpu_attempt(&mut runtime, ComputeChoice::Auto);
    confirm_prefix(&mut runtime, PCM_SAMPLES);
    runtime.live.as_mut().unwrap().barrier_m = Some(1);
    attach_exited_worker_stopped(&mut runtime, &root.0, job_id);

    let result = runtime
        .poll()
        .expect("process defensive invalid-barrier state");
    let archive = live_archive_directory(&root.0, job_id);
    let artifacts_published = archive_current_path(&root.0, job_id).exists()
        || archive.join("manifest-1.json").exists()
        || archive.join("passage-1.mp3").exists();
    assert!(
        matches!(
            &result,
            Err((actual, generation, message))
                if *actual == job_id
                    && *generation == Generation::first()
                    && message.contains("StorageCorrupt: retry barrier")
        ) && !artifacts_published,
        "defensive invalid barrier must reject the real publishable prefix; got {result:?}, artifacts_published={artifacts_published}"
    );
    assert!(runtime.live.is_none());
    assert!(runtime.live_archive.is_none());
    assert!(runtime.transport.is_none());
    assert!(runtime.active.is_none());
}

#[test]
fn poll_stopped_live_defensive_confirmed_coverage_past_durable_rejects_without_publication() {
    let root = TestRoot::new("active-live-defensive-coverage-high");
    let job_id = JobId(0x99613);
    let mut runtime = active_live_runtime(&root.0, job_id);
    start_gpu_attempt(&mut runtime, ComputeChoice::Auto);
    runtime
        .live
        .as_mut()
        .unwrap()
        .staging
        .append_frame(vec![0; PCM_SAMPLES])
        .expect("append real PCM prefix");
    runtime.live.as_mut().unwrap().staging.drain().unwrap();
    let impossible_range = SourceRange::new(0, 640, 16_000).unwrap();
    runtime
        .live_archive
        .as_mut()
        .unwrap()
        .persist_window_finished(1, impossible_range, 0, 640)
        .expect("fixture explicitly corrupt coverage beyond real durable PCM");
    attach_exited_worker_stopped(&mut runtime, &root.0, job_id);

    let result = runtime.poll().expect("process defensive coverage state");
    assert!(matches!(
        result,
        Err((actual, generation, message))
            if actual == job_id && generation == Generation::first()
                && message.contains("StorageCorrupt: retry barrier")
    ));
    assert!(!archive_current_path(&root.0, job_id).exists());
    assert!(runtime.live.is_none());
    assert!(runtime.live_archive.is_none());
    assert!(runtime.transport.is_none());
    assert!(runtime.active.is_none());
}
