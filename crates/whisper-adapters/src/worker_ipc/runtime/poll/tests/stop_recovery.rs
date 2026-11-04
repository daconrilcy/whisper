use super::*;
use whisper_core::{ComputeChoice, ImportEvent, JobConfig, LanguageChoice, ports::ImportRequest};

fn gpu_candidate() -> whisper_core::gpu_inventory::GpuRuntimeIdentity {
    whisper_core::gpu_inventory::GpuRuntimeIdentity {
        generation: 8,
        pnp_instance_id: "PCI\\VEN_10DE&DEV_1234".into(),
        dxgi_rank: 2,
        luid: 0x1234,
        pci_id: "0000:AB:CD.E".into(),
        cuda_ordinal: 1,
        ggml_ordinal: 4,
    }
}

fn compile_stop_worker(root: &Path) -> PathBuf {
    let source = root.join("stop-worker.rs");
    let executable = root.join("stop-worker.exe");
    let log_path =
        serde_json::to_string(&root.join("stop-worker.log").to_string_lossy().to_string())
            .expect("encode Stop worker log path");
    let program = r#"
use std::{fs::OpenOptions, io::{self, BufRead, Write}};
const LOG: &str = __LOG__;
fn field(line: &str, name: &str) -> String {
    let token = format!("\"{name}\":");
    line.split_once(&token).expect("wire identity").1.trim_start()
        .chars().take_while(|ch| ch.is_ascii_digit()).collect()
}
fn record(line: &str) {
    let mut log = OpenOptions::new().create(true).append(true).open(LOG).expect("open Stop log");
    writeln!(log, "{line}").expect("record Stop worker event");
}
fn main() {
    let stdin = io::stdin();
    let mut job = None;
    let mut generation = None;
    let mut instance = None;
    let mut stopped = false;
    for line in stdin.lock().lines().flatten() {
        if line.contains("\"Stop\"") {
            job = Some(field(&line, "job_id"));
            generation = Some(field(&line, "generation"));
            stopped = true;
            record("Stop");
        } else if line.contains("\"StartGpuLiveWorker\"") || line.contains("\"StartGpuImport\"") {
            job = Some(field(&line, "job_id"));
            generation = Some(field(&line, "generation"));
            instance = Some(field(&line, "instance_id"));
            record("Start");
        }
        if stopped && instance.is_some() {
            let job = job.as_deref().expect("Stop job identity");
            let generation = generation.as_deref().expect("Stop generation identity");
            let instance = instance.as_deref().expect("start instance identity");
            record(&format!("Stopped:{job}:{generation}:{instance}"));
            println!("{{\"version\":2,\"message\":{{\"Stopped\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance}}}}}}}");
            io::stdout().flush().expect("flush Stop acknowledgement");
            return;
        }
    }
}
"#
    .replace("__LOG__", &log_path);
    fs::write(&source, program).expect("write Stop worker fixture");
    let output = Command::new("rustc")
        .args(["--edition=2021", "--crate-name", "poll_stop_worker"])
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

fn compile_cpu_marker_worker(root: &Path, marker: &Path) -> PathBuf {
    let source = root.join("cpu-marker-worker.rs");
    let executable = root.join("cpu-marker-worker.exe");
    let marker = serde_json::to_string(&marker.to_string_lossy().to_string())
        .expect("encode CPU marker path");
    let program = r#"
use std::{fs, io::{self, BufRead}};
const MARKER: &str = __MARKER__;
fn main() {
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    if lines.next().is_some() {
        fs::write(MARKER, "CPU worker launched").expect("write CPU marker");
        for _line in lines.flatten() {}
    }
}
"#
    .replace("__MARKER__", &marker);
    fs::write(&source, program).expect("write CPU marker fixture");
    let output = Command::new("rustc")
        .args(["--edition=2021", "--crate-name", "poll_cpu_marker_worker"])
        .arg(&source)
        .arg("-o")
        .arg(&executable)
        .output()
        .expect("compile CPU marker fixture");
    assert!(
        output.status.success(),
        "rustc CPU marker worker: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    executable
}

fn pending_auto_retry(
    root: &Path,
    job_id: JobId,
) -> (ImportRuntime, whisper_core::ports::LiveRequest, PathBuf) {
    let marker = root.join("cpu-launched");
    let gpu = compile_stop_worker(root);
    let cpu = compile_cpu_marker_worker(root, &marker);
    let gate = Arc::new(PublicationGate::default());
    gate.activate();
    let mut runtime = ImportRuntime::new(WorkerPaths { cpu, gpu }, gate);
    let mut request = live_request(root, job_id);
    request.config.compute = ComputeChoice::Auto;
    runtime
        .start_live(request.clone(), Some(gpu_candidate()))
        .expect("start GPU live attempt");
    (runtime, request, marker)
}

fn inject_gpu_failure(runtime: &mut ImportRuntime, job_id: JobId) {
    let instance_id = runtime.active.as_ref().expect("GPU attempt").instance_id;
    runtime
        .transport
        .as_mut()
        .expect("GPU transport")
        .inject_event(WorkerEvent::Failed {
            job_id: Some(job_id),
            generation: Some(Generation::first()),
            instance_id: Some(instance_id),
            code: whisper_core::ipc::WorkerErrorCode::BackendUnavailable,
            message: "simulated GPU initialization failure".into(),
        });
}

fn poll_until_result(
    runtime: &mut ImportRuntime,
) -> Result<ImportEvent, (JobId, Generation, String)> {
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        match runtime.poll() {
            Some(result) => return result,
            None => std::thread::sleep(Duration::from_millis(2)),
        }
    }
    panic!("expected worker event before deadline");
}

fn poll_until_event(runtime: &mut ImportRuntime) -> ImportEvent {
    poll_until_result(runtime).unwrap_or_else(|error| panic!("unexpected poll error: {error:?}"))
}

fn assert_live_request_preserved(
    runtime: &ImportRuntime,
    request: &whisper_core::ports::LiveRequest,
) {
    let pending = runtime.pending_live.as_ref().expect("pending request");
    assert_eq!(pending.request.job_id, request.job_id);
    assert_eq!(pending.request.generation, request.generation);
    assert_eq!(pending.request.model_path, request.model_path);
    assert_eq!(pending.request.model_sha256, request.model_sha256);
    assert_eq!(pending.request.destination, request.destination);
    assert_eq!(pending.request.config.compute, request.config.compute);
}

fn assert_import_request_preserved(runtime: &ImportRuntime, request: &ImportRequest) {
    let active = runtime.active.as_ref().expect("active import request");
    let saved = active.import_request.as_ref().expect("retry request");
    assert_eq!(saved.job_id, request.job_id);
    assert_eq!(saved.generation, request.generation);
    assert_eq!(saved.source_path, request.source_path);
    assert_eq!(saved.source_sha256, request.source_sha256);
    assert_eq!(saved.model_path, request.model_path);
    assert_eq!(saved.destination, request.destination);
    assert_eq!(saved.config.compute, request.config.compute);
}

fn import_request(root: &Path, job_id: JobId) -> ImportRequest {
    ImportRequest {
        strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
        job_id,
        generation: Generation::first(),
        source_path: root.join("source.wav").to_string_lossy().into_owned(),
        source_sha256: Some([1; 32]),
        source_samples: Some(16_000),
        model_path: root.join("model.bin").to_string_lossy().into_owned(),
        model_sha256: [2; 32],
        destination: root.join("archive").to_string_lossy().into_owned(),
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Auto,
        },
    }
}

fn pending_attempt_journal(root: &Path, job_id: JobId) -> PathBuf {
    root.join("archive")
        .join("transcriptions")
        .join(format!("{:032x}", job_id.0))
        .join("live")
        .join("passage-1.attempts.jsonl")
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

fn stop_worker_log(root: &Path) -> PathBuf {
    root.join("stop-worker.log")
}

#[test]
fn poll_pending_live_retirement_error_clears_pending_attempt_and_transport() {
    let root = TestRoot::new("stop-pending-retire-error");
    let job_id = JobId(0x99509);
    let (mut runtime, _, _) = pending_auto_retry(&root.0, job_id);
    inject_gpu_failure(&mut runtime, job_id);
    assert!(matches!(
        poll_until_event(&mut runtime),
        ImportEvent::LiveRetryingOnCpu { .. }
    ));

    let journal = pending_attempt_journal(&root.0, job_id);
    fs::remove_file(&journal).expect("remove fixture attempt journal");
    fs::create_dir(&journal).expect("replace attempt journal with directory fixture");
    let error =
        poll_until_result(&mut runtime).expect_err("retirement must report journal I/O failure");
    assert_eq!(error.0, job_id);
    assert_eq!(error.1, Generation::first());
    assert!(error.2.contains("attempt journal"));
    assert!(runtime.pending_live.is_none());
    assert!(runtime.active.is_none());
    assert!(runtime.transport.is_none());
}

#[test]
fn poll_stopped_auto_retry_cancels_pending_live_without_starting_cpu() {
    let root = TestRoot::new("stop-pending-auto-cancel");
    let job_id = JobId(0x99501);
    let (mut runtime, _, marker) = pending_auto_retry(&root.0, job_id);
    inject_gpu_failure(&mut runtime, job_id);
    assert!(matches!(
        poll_until_event(&mut runtime),
        ImportEvent::LiveRetryingOnCpu { job_id: actual, .. } if actual == job_id
    ));
    assert_eq!(
        runtime.pending_live.as_ref().unwrap().recovery,
        LiveRecoveryState::AutoRetryPending
    );
    assert_eq!(runtime.publication_gate.request_stop(), Ok(true));

    assert!(matches!(
        poll_until_event(&mut runtime),
        ImportEvent::Stopped { job_id: actual, generation }
            if actual == job_id && generation == Generation::first()
    ));
    assert!(runtime.pending_live.is_none());
    assert!(runtime.active.is_none());
    assert!(runtime.transport.is_none());
    assert!(
        !marker.exists(),
        "CPU worker started after Stop was latched"
    );
}

#[test]
fn poll_stopped_auto_retry_starts_cpu_and_preserves_pending_live_request() {
    let root = TestRoot::new("stop-pending-auto-retry");
    let job_id = JobId(0x99502);
    let (mut runtime, request, marker) = pending_auto_retry(&root.0, job_id);
    inject_gpu_failure(&mut runtime, job_id);
    assert!(matches!(
        poll_until_event(&mut runtime),
        ImportEvent::LiveRetryingOnCpu { .. }
    ));

    assert!(matches!(
        poll_until_event(&mut runtime),
        ImportEvent::LiveAttemptStarting { job_id: actual, generation }
            if actual == job_id && generation == Generation::first()
    ));
    assert!(
        runtime
            .active
            .as_ref()
            .is_some_and(|active| active.backend == BackendKind::Cpu)
    );
    assert!(runtime.transport.is_some());
    assert_live_request_preserved(&runtime, &request);
    assert!(
        runtime
            .pending_live
            .as_ref()
            .is_some_and(|pending| pending.recovery == LiveRecoveryState::None)
    );
    let deadline = Instant::now() + Duration::from_secs(3);
    while !marker.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(
        marker.exists(),
        "CPU worker did not receive the retry command"
    );
}

#[test]
fn poll_stopped_manual_gpu_failure_waits_for_choice_and_preserves_pending_request() {
    let root = TestRoot::new("stop-pending-manual-choice");
    let marker = root.0.join("cpu-launched");
    let gpu = compile_stop_worker(&root.0);
    let cpu = compile_cpu_marker_worker(&root.0, &marker);
    let gate = Arc::new(PublicationGate::default());
    gate.activate();
    let mut runtime = ImportRuntime::new(WorkerPaths { cpu, gpu }, gate);
    let job_id = JobId(0x99503);
    let mut request = live_request(&root.0, job_id);
    request.config.compute = ComputeChoice::Gpu;
    runtime
        .start_live(request.clone(), Some(gpu_candidate()))
        .expect("start manual GPU live attempt");
    inject_gpu_failure(&mut runtime, job_id);
    assert!(matches!(
        poll_until_event(&mut runtime),
        ImportEvent::LiveFinalizing { .. }
    ));

    assert!(matches!(
        poll_until_event(&mut runtime),
        ImportEvent::AwaitingCpuChoice { job_id: actual, generation }
            if actual == job_id && generation == Generation::first()
    ));
    assert!(runtime.active.is_none());
    assert!(runtime.transport.is_none());
    assert_live_request_preserved(&runtime, &request);
    assert_eq!(
        runtime.pending_live.as_ref().unwrap().recovery,
        LiveRecoveryState::AwaitingChoice
    );
    assert!(
        !marker.exists(),
        "manual GPU failure started CPU before user choice"
    );
}

#[test]
fn poll_stopped_manual_gpu_choice_is_cancelled_by_publication_gate() {
    let root = TestRoot::new("stop-pending-choice-cancel");
    let marker = root.0.join("cpu-launched");
    let gpu = compile_stop_worker(&root.0);
    let cpu = compile_cpu_marker_worker(&root.0, &marker);
    let gate = Arc::new(PublicationGate::default());
    gate.activate();
    let mut runtime = ImportRuntime::new(WorkerPaths { cpu, gpu }, gate);
    let job_id = JobId(0x99510);
    let mut request = live_request(&root.0, job_id);
    request.config.compute = ComputeChoice::Gpu;
    runtime
        .start_live(request, Some(gpu_candidate()))
        .expect("start manual GPU live attempt");
    inject_gpu_failure(&mut runtime, job_id);
    assert!(matches!(
        poll_until_event(&mut runtime),
        ImportEvent::LiveFinalizing { .. }
    ));
    assert_eq!(
        runtime.pending_live.as_ref().unwrap().recovery,
        LiveRecoveryState::AwaitingChoicePending
    );
    assert_eq!(runtime.publication_gate.request_stop(), Ok(true));

    assert!(matches!(
        poll_until_event(&mut runtime),
        ImportEvent::Stopped { job_id: actual, generation }
            if actual == job_id && generation == Generation::first()
    ));
    assert!(runtime.pending_live.is_none());
    assert!(runtime.active.is_none());
    assert!(runtime.transport.is_none());
    assert!(
        !marker.exists(),
        "CPU worker started after Stop was latched"
    );
}

#[test]
fn poll_pending_cpu_retry_worker_absent_leaves_attempt_reserved_without_started() {
    let root = TestRoot::new("stop-pending-cpu-missing");
    let job_id = JobId(0x99504);
    let (mut runtime, request, marker) = pending_auto_retry(&root.0, job_id);
    runtime.workers.cpu = root.0.join("missing-cpu-worker.exe");
    inject_gpu_failure(&mut runtime, job_id);
    assert!(matches!(
        poll_until_event(&mut runtime),
        ImportEvent::LiveRetryingOnCpu { .. }
    ));

    let error = poll_until_result(&mut runtime).expect_err("missing CPU worker must fail retry");
    assert_eq!(error.0, job_id);
    assert_eq!(error.1, Generation::first());
    assert!(error.2.contains("CPU fallback unavailable"));
    assert!(runtime.active.is_none());
    assert!(runtime.transport.is_none());
    assert_live_request_preserved(&runtime, &request);
    assert!(
        runtime
            .pending_live
            .as_ref()
            .is_some_and(|pending| pending.recovery == LiveRecoveryState::AutoRetryPending)
    );
    assert!(!marker.exists());
    let journal = fs::read_to_string(pending_attempt_journal(&root.0, job_id))
        .expect("read attempt journal after CPU spawn failure");
    assert_eq!(
        attempt_states(&journal, 1),
        vec!["reserved", "started", "retired"]
    );
    assert_eq!(attempt_states(&journal, 2), vec!["reserved"]);
}

fn import_retry_runtime(root: &Path, job_id: JobId) -> (ImportRuntime, ImportRequest, PathBuf) {
    let marker = root.join("cpu-launched");
    let gpu = compile_stop_worker(root);
    let cpu = compile_cpu_marker_worker(root, &marker);
    let gate = Arc::new(PublicationGate::default());
    gate.activate();
    let mut runtime = ImportRuntime::new(WorkerPaths { cpu, gpu }, gate);
    let request = import_request(root, job_id);
    runtime
        .start_worker_with_candidate(request.clone(), Some(gpu_candidate()))
        .expect("start GPU import attempt");
    (runtime, request, marker)
}

fn inject_import_gpu_failure(runtime: &mut ImportRuntime, request: &ImportRequest) {
    let instance_id = runtime.active.as_ref().expect("GPU attempt").instance_id;
    runtime
        .transport
        .as_mut()
        .expect("GPU transport")
        .inject_event(WorkerEvent::Failed {
            job_id: Some(request.job_id),
            generation: Some(request.generation),
            instance_id: Some(instance_id),
            code: whisper_core::ipc::WorkerErrorCode::BackendUnavailable,
            message: "simulated GPU import initialization failure".into(),
        });
}

#[test]
fn poll_stopped_import_auto_retry_drops_request_when_gate_is_cancelled() {
    let root = TestRoot::new("stop-import-auto-cancel");
    let job_id = JobId(0x99505);
    let (mut runtime, request, marker) = import_retry_runtime(&root.0, job_id);
    inject_import_gpu_failure(&mut runtime, &request);
    assert!(matches!(
        poll_until_event(&mut runtime),
        ImportEvent::LiveAttemptStarting { job_id: actual, generation }
            if actual == job_id && generation == request.generation
    ));
    assert_eq!(
        runtime.active.as_ref().unwrap().recovery,
        LiveRecoveryState::AutoRetryPending
    );
    assert_eq!(runtime.publication_gate.request_stop(), Ok(true));

    let stopped = poll_until_event(&mut runtime);
    let deadline = if matches!(&stopped, ImportEvent::LiveAttemptStarting { .. }) {
        Instant::now() + Duration::from_secs(3)
    } else {
        Instant::now()
    };
    while !marker.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(2));
    }
    let cpu_started = marker.exists();
    let cpu_worker_alive = runtime
        .transport
        .as_mut()
        .is_some_and(|transport| transport.try_wait().is_ok_and(|status| status.is_none()));
    assert!(
        matches!(
            &stopped,
            ImportEvent::Stopped { job_id: actual, generation }
                if *actual == job_id && *generation == request.generation
        ) && !cpu_started
            && !cpu_worker_alive,
        "cancelled import retry must return Stopped without launching CPU; got {stopped:?}, cpu_started={cpu_started}, cpu_worker_alive={cpu_worker_alive}"
    );
    assert!(runtime.active.is_none());
    assert!(runtime.transport.is_none());
    assert!(
        !marker.exists(),
        "CPU worker started after Stop was latched"
    );
}

#[test]
fn poll_stopped_import_auto_retry_starts_cpu_and_preserves_request() {
    let root = TestRoot::new("stop-import-auto-retry");
    let job_id = JobId(0x99506);
    let (mut runtime, request, marker) = import_retry_runtime(&root.0, job_id);
    inject_import_gpu_failure(&mut runtime, &request);
    assert!(matches!(
        poll_until_event(&mut runtime),
        ImportEvent::LiveAttemptStarting { .. }
    ));

    assert!(matches!(
        poll_until_event(&mut runtime),
        ImportEvent::LiveAttemptStarting { job_id: actual, generation }
            if actual == job_id && generation == request.generation
    ));
    assert!(
        runtime
            .active
            .as_ref()
            .is_some_and(|active| active.backend == BackendKind::Cpu)
    );
    assert!(runtime.transport.is_some());
    assert_import_request_preserved(&runtime, &request);
    let deadline = Instant::now() + Duration::from_secs(3);
    while !marker.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(
        marker.exists(),
        "CPU import worker did not receive the retry command"
    );
}

#[test]
fn poll_stopped_import_auto_retry_reports_missing_cpu_worker_without_restoring_request() {
    let root = TestRoot::new("stop-import-cpu-missing");
    let job_id = JobId(0x99507);
    let (mut runtime, request, marker) = import_retry_runtime(&root.0, job_id);
    let cpu_path = root.0.join("missing-cpu-worker.exe");
    runtime.workers.cpu = cpu_path.clone();
    let gpu_attempt = runtime.active.as_ref().unwrap().instance_id;
    inject_import_gpu_failure(&mut runtime, &request);
    assert!(matches!(
        poll_until_event(&mut runtime),
        ImportEvent::LiveAttemptStarting { .. }
    ));

    assert_eq!(
        runtime.active.as_ref().unwrap().recovery,
        LiveRecoveryState::AutoRetryPending
    );
    assert!(
        !cpu_path.is_file(),
        "the CPU executable fixture must be absent"
    );
    let error = poll_until_result(&mut runtime).expect_err("missing CPU worker must fail retry");
    assert_eq!(error.0, job_id);
    assert_eq!(error.1, request.generation);
    assert!(error.2.contains("CpuUnavailable"));
    let stop_log =
        fs::read_to_string(stop_worker_log(&root.0)).expect("read previous GPU Stop log");
    assert!(stop_log.lines().any(|line| {
        line == format!(
            "Stopped:{}:{}:{}",
            job_id.0,
            request.generation.get(),
            gpu_attempt
        )
    }));
    assert!(runtime.active.is_none());
    assert!(runtime.transport.is_none());
    assert!(!marker.exists());
}

#[test]
fn poll_explicit_cpu_choice_after_manual_gpu_failure_starts_preserved_request() {
    let root = TestRoot::new("stop-explicit-cpu-choice");
    let marker = root.0.join("cpu-launched");
    let gpu = compile_stop_worker(&root.0);
    let cpu = compile_cpu_marker_worker(&root.0, &marker);
    let gate = Arc::new(PublicationGate::default());
    gate.activate();
    let mut runtime = ImportRuntime::new(WorkerPaths { cpu, gpu }, gate);
    let job_id = JobId(0x99508);
    let mut request = live_request(&root.0, job_id);
    request.config.compute = ComputeChoice::Gpu;
    runtime
        .start_live(request.clone(), Some(gpu_candidate()))
        .expect("start manual GPU attempt");
    inject_gpu_failure(&mut runtime, job_id);
    assert!(matches!(
        poll_until_event(&mut runtime),
        ImportEvent::LiveFinalizing { .. }
    ));
    assert!(matches!(
        poll_until_event(&mut runtime),
        ImportEvent::AwaitingCpuChoice { .. }
    ));

    let event = runtime
        .finish_live_on_cpu(job_id, request.generation)
        .expect("explicit CPU choice");
    assert!(matches!(event, ImportEvent::LiveAttemptStarting { .. }));
    assert!(
        runtime
            .active
            .as_ref()
            .is_some_and(|active| active.backend == BackendKind::Cpu)
    );
    assert!(runtime.transport.is_some());
    assert_live_request_preserved(&runtime, &request);
    let deadline = Instant::now() + Duration::from_secs(3);
    while !marker.exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(
        marker.exists(),
        "CPU worker did not start after explicit user choice"
    );
}
