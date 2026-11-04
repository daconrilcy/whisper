use super::*;
use whisper_core::{
    ComputeChoice, JobConfig, LanguageChoice, gpu_inventory::GpuRuntimeIdentity,
    parakeet_session::TranscriptionStrategy, ports::ImportRequest,
};

const INSTANCE: u64 = 7;

fn gpu_candidate() -> GpuRuntimeIdentity {
    GpuRuntimeIdentity {
        generation: 8,
        pnp_instance_id: "PCI\\VEN_10DE&DEV_1234".into(),
        dxgi_rank: 2,
        luid: 0x1234,
        pci_id: "0000:AB:CD.E".into(),
        cuda_ordinal: 1,
        ggml_ordinal: 4,
    }
}

fn import_request(job_id: JobId, compute: ComputeChoice) -> ImportRequest {
    ImportRequest {
        strategy: TranscriptionStrategy::LegacyV1,
        job_id,
        generation: Generation::first(),
        source_path: "unused.wav".into(),
        source_sha256: Some([1; 32]),
        source_samples: Some(16_000),
        model_path: "unused-model".into(),
        model_sha256: [2; 32],
        destination: "unused-output".into(),
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute,
        },
    }
}

fn set_gpu_attempt(runtime: &mut ImportRuntime, compute: ComputeChoice) {
    let active = runtime.active.as_mut().expect("active attempt");
    active.backend = BackendKind::Cuda;
    active.startup_ready = false;
    active.gpu_candidate = Some(gpu_candidate());
    active.gpu_attested = false;
    active.import_request = Some(import_request(active.job_id, compute));
}

fn attestation(job_id: JobId, candidate: &GpuRuntimeIdentity) -> WorkerEvent {
    WorkerEvent::GpuSessionAttested {
        job_id,
        generation: Generation::first(),
        instance_id: INSTANCE,
        candidate: candidate.pnp_instance_id.clone(),
        dxgi_rank: candidate.dxgi_rank,
        luid: candidate.luid,
        pci_id: candidate.pci_id.clone(),
        cuda_ordinal: candidate.cuda_ordinal,
        ggml_ordinal: candidate.ggml_ordinal,
    }
}

fn inject(runtime: &mut ImportRuntime, event: WorkerEvent) {
    runtime
        .transport
        .as_mut()
        .expect("test worker transport")
        .inject_event(event);
}

fn compile_command_worker(root: &Path, marker: &Path) -> PathBuf {
    let source = root.join("readiness-worker.rs");
    let executable = root.join("readiness-worker.exe");
    let marker =
        serde_json::to_string(&marker.to_string_lossy().to_string()).expect("encode marker path");
    let program = r#"
use std::{fs::OpenOptions, io::{self, BufRead, Write}};
const MARKER: &str = __MARKER__;
fn field(line: &str, name: &str) -> String {
    let token = format!("\"{name}\":");
    let tail = line.split_once(&token).expect("command identity").1.trim_start();
    tail.chars().take_while(|c| c.is_ascii_digit()).collect()
}
fn record(value: &str) {
    let mut file = OpenOptions::new().create(true).append(true).open(MARKER).unwrap();
    writeln!(file, "{value}").unwrap();
}
fn main() {
    for line in io::stdin().lock().lines().flatten() {
        if line.contains("\"BeginImport\"") {
            record("BeginImport");
            let job = field(&line, "job_id");
            let generation = field(&line, "generation");
            let instance = field(&line, "instance_id");
            println!("{{\"version\":2,\"message\":{{\"ImportStarted\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance}}}}}}}");
            io::stdout().flush().unwrap();
        } else if line.contains("\"Stop\"") {
            record("Stop");
            let job = field(&line, "job_id");
            let generation = field(&line, "generation");
            println!("{{\"version\":2,\"message\":{{\"Stopped\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":7}}}}}}");
            io::stdout().flush().unwrap();
            return;
        }
    }
}
"#
    .replace("__MARKER__", &marker);
    fs::write(&source, program).expect("write worker fixture");
    let output = Command::new("rustc")
        .args(["--edition=2021", "--crate-name", "poll_readiness_worker"])
        .arg(&source)
        .arg("-o")
        .arg(&executable)
        .output()
        .expect("compile worker fixture");
    assert!(
        output.status.success(),
        "rustc readiness worker: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    executable
}

fn poll_until_stopped(runtime: &mut ImportRuntime) -> ImportEvent {
    let deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < deadline {
        if let Some(Ok(event @ ImportEvent::Stopped { .. })) = runtime.poll() {
            return event;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    panic!("worker did not acknowledge Stop");
}

#[test]
fn poll_attestation_matches_all_candidate_fields_and_accepts_pci_case_only_difference() {
    let root = TestRoot::new("attestation-match");
    let worker = compile_idle_worker(&root.0);
    let job_id = JobId(0x99301);
    let mut runtime = runtime_with_active_attempt(job_id);
    set_gpu_attempt(&mut runtime, ComputeChoice::Gpu);
    let candidate = gpu_candidate();
    let mut event = attestation(job_id, &candidate);
    let WorkerEvent::GpuSessionAttested { pci_id, .. } = &mut event else {
        unreachable!();
    };
    *pci_id = "0000:ab:cd.e".into();
    runtime.transport = Some(ChildWorkerTransport::spawn(&worker).expect("spawn worker"));
    inject(&mut runtime, event);

    assert!(runtime.poll().is_none());
    assert!(runtime.active.as_ref().expect("attempt").gpu_attested);
}

#[test]
fn poll_rejects_each_gpu_candidate_field_difference() {
    let root = TestRoot::new("attestation-mismatch");
    let worker = compile_idle_worker(&root.0);
    for field in ["candidate", "rank", "luid", "pci", "cuda", "ggml"] {
        let job_id = JobId(0x99302 + u128::from(field.as_bytes()[0]));
        let candidate = gpu_candidate();
        let mut event = attestation(job_id, &candidate);
        let WorkerEvent::GpuSessionAttested {
            candidate: event_candidate,
            dxgi_rank,
            luid,
            pci_id,
            cuda_ordinal,
            ggml_ordinal,
            ..
        } = &mut event
        else {
            unreachable!();
        };
        match field {
            "candidate" => *event_candidate = "PCI\\VEN_10DE&DEV_9999".into(),
            "rank" => *dxgi_rank += 1,
            "luid" => *luid += 1,
            "pci" => *pci_id = "0000:00:00.0".into(),
            "cuda" => *cuda_ordinal += 1,
            "ggml" => *ggml_ordinal += 1,
            _ => unreachable!(),
        }
        let mut runtime = runtime_with_active_attempt(job_id);
        set_gpu_attempt(&mut runtime, ComputeChoice::Gpu);
        runtime.transport = Some(ChildWorkerTransport::spawn(&worker).expect("spawn worker"));
        inject(&mut runtime, event);

        assert!(
            matches!(runtime.poll(), Some(Err((actual, _, message))) if actual == job_id && message.contains("BackendMismatch")),
            "field {field}"
        );
        assert!(
            runtime.active.is_none(),
            "field {field}: failed attempt remains"
        );
        assert!(
            runtime.transport.is_none(),
            "field {field}: transport remains"
        );
    }
}

#[test]
fn poll_rejects_ready_before_attestation_and_ready_for_a_different_backend() {
    let root = TestRoot::new("ready-attestation");
    let worker = compile_idle_worker(&root.0);
    let job_id = JobId(0x99303);
    let mut runtime = runtime_with_active_attempt(job_id);
    set_gpu_attempt(&mut runtime, ComputeChoice::Gpu);
    runtime.transport = Some(ChildWorkerTransport::spawn(&worker).expect("spawn worker"));
    inject(
        &mut runtime,
        WorkerEvent::Ready {
            job_id,
            generation: Generation::first(),
            instance_id: INSTANCE,
            backend: BackendKind::Cuda,
        },
    );
    assert!(
        matches!(runtime.poll(), Some(Err((actual, _, message))) if actual == job_id && message.contains("before candidate attestation"))
    );
    assert!(runtime.active.is_none());
    assert!(runtime.transport.is_none());

    let job_id = JobId(0x99304);
    let mut runtime = runtime_with_active_attempt(job_id);
    set_gpu_attempt(&mut runtime, ComputeChoice::Gpu);
    runtime.active.as_mut().expect("attempt").gpu_attested = true;
    runtime.transport = Some(ChildWorkerTransport::spawn(&worker).expect("spawn worker"));
    inject(
        &mut runtime,
        WorkerEvent::Ready {
            job_id,
            generation: Generation::first(),
            instance_id: INSTANCE,
            backend: BackendKind::Cpu,
        },
    );
    assert!(
        matches!(runtime.poll(), Some(Err((actual, _, message))) if actual == job_id && message.contains("worker attestation differs"))
    );
    assert!(runtime.active.is_none());
    assert!(runtime.transport.is_none());

    let job_id = JobId(0x99312);
    let mut runtime = runtime_with_active_attempt(job_id);
    runtime.transport = Some(ChildWorkerTransport::spawn(&worker).expect("spawn worker"));
    inject(
        &mut runtime,
        WorkerEvent::Failed {
            job_id: None,
            generation: None,
            instance_id: None,
            code: whisper_core::ipc::WorkerErrorCode::WorkerExited,
            message: "unidentified after Ready".into(),
        },
    );
    assert!(
        matches!(runtime.poll(), Some(Err((actual, _, message))) if actual == job_id && message.contains("ProtocolMismatch"))
    );
    assert!(runtime.active.is_none());
    assert!(runtime.transport.is_none());
}

#[test]
fn poll_cpu_ready_reports_fallback_reason_and_advances_startup() {
    let root = TestRoot::new("ready-cpu-fallback");
    let worker = compile_idle_worker(&root.0);
    let job_id = JobId(0x99305);
    let mut runtime = runtime_with_active_attempt(job_id);
    runtime.active.as_mut().expect("attempt").startup_ready = false;
    runtime.auto_fallback_reason = Some("GPU failed during startup".into());
    runtime.transport = Some(ChildWorkerTransport::spawn(&worker).expect("spawn worker"));
    inject(
        &mut runtime,
        WorkerEvent::Ready {
            job_id,
            generation: Generation::first(),
            instance_id: INSTANCE,
            backend: BackendKind::Cpu,
        },
    );
    assert!(
        matches!(runtime.poll(), Some(Ok(ImportEvent::Ready { backend, .. })) if backend.starts_with("CPU") && backend.contains("GPU failed during startup") && backend.contains("CPU prend le relais"))
    );
    assert!(runtime.active.as_ref().expect("attempt").startup_ready);
    assert!(runtime.transport.is_some());
}

#[test]
fn poll_cpu_ready_marks_existing_live_worker_ready() {
    let root = TestRoot::new("ready-existing-live-worker");
    let worker = compile_idle_worker(&root.0);
    let job_id = JobId(0x99314);
    let mut runtime = runtime_with_active_attempt(job_id);
    runtime.live = Some(live_capture_state(&root.0, job_id));
    runtime.live.as_mut().expect("live capture").worker_ready = false;
    runtime.transport = Some(ChildWorkerTransport::spawn(&worker).expect("spawn worker"));
    inject(
        &mut runtime,
        WorkerEvent::Ready {
            job_id,
            generation: Generation::first(),
            instance_id: INSTANCE,
            backend: BackendKind::Cpu,
        },
    );

    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::Ready { job_id: actual_job, backend, .. }))
            if actual_job == job_id && backend == "CPU"
    ));
    assert!(runtime.live.as_ref().expect("live capture").worker_ready);
    assert!(runtime.active.as_ref().expect("attempt").startup_ready);
    assert!(runtime.transport.is_some());
}

#[test]
fn poll_auto_gpu_ready_sends_begin_import_and_accepts_only_the_expected_ack() {
    let root = TestRoot::new("ready-auto-admission");
    let marker = root.0.join("commands.log");
    let worker = compile_command_worker(&root.0, &marker);
    let job_id = JobId(0x99306);
    let mut runtime = runtime_with_active_attempt(job_id);
    set_gpu_attempt(&mut runtime, ComputeChoice::Auto);
    runtime.active.as_mut().expect("attempt").gpu_attested = true;
    runtime.transport = Some(ChildWorkerTransport::spawn(&worker).expect("spawn worker"));
    inject(
        &mut runtime,
        WorkerEvent::Ready {
            job_id,
            generation: Generation::first(),
            instance_id: INSTANCE,
            backend: BackendKind::Cuda,
        },
    );
    assert!(
        matches!(runtime.poll(), Some(Ok(ImportEvent::Ready { backend, .. })) if backend == "CUDA")
    );
    let deadline = Instant::now() + Duration::from_secs(3);
    while !runtime.active.as_ref().expect("attempt").startup_ready && Instant::now() < deadline {
        let _ = runtime.poll();
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(runtime.active.as_ref().expect("attempt").startup_ready);
    assert_eq!(
        fs::read_to_string(marker).expect("worker command log"),
        "BeginImport\n"
    );

    inject(
        &mut runtime,
        WorkerEvent::ImportStarted {
            job_id,
            generation: Generation::first(),
            instance_id: INSTANCE,
        },
    );
    assert!(
        matches!(runtime.poll(), Some(Err((actual, _, message))) if actual == job_id && message.contains("unexpected Auto import admission acknowledgement"))
    );
    assert!(runtime.active.is_some());
    assert!(runtime.transport.is_some());
}

#[test]
fn poll_ready_reports_begin_import_send_failure_as_cpu_retry_pending() {
    let root = TestRoot::new("ready-auto-dispatcher-failure");
    let worker = compile_idle_worker(&root.0);
    let job_id = JobId(0x99313);
    let mut runtime = runtime_with_active_attempt(job_id);
    set_gpu_attempt(&mut runtime, ComputeChoice::Auto);
    runtime.active.as_mut().expect("attempt").gpu_attested = true;

    let mut transport = ChildWorkerTransport::spawn(&worker).expect("spawn worker");
    transport.abort();
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut queued_after_abort = false;
    loop {
        match transport.try_request_ordered(WorkerCommand::Stop {
            job_id,
            generation: Generation::first(),
        }) {
            Ok(()) => queued_after_abort = true,
            Err(crate::worker_ipc::ipc::EnqueueError::Full(_)) => {}
            Err(crate::worker_ipc::ipc::EnqueueError::Disconnected) => {
                assert!(
                    queued_after_abort,
                    "dispatcher disconnected before the pipe probe"
                );
                break;
            }
        }
        assert!(
            Instant::now() < deadline,
            "dispatcher did not observe the closed child pipe"
        );
        std::thread::sleep(Duration::from_millis(2));
    }

    runtime.transport = Some(transport);
    inject(
        &mut runtime,
        WorkerEvent::Ready {
            job_id,
            generation: Generation::first(),
            instance_id: INSTANCE,
            backend: BackendKind::Cuda,
        },
    );
    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::LiveAttemptStarting { job_id: actual_job, generation }))
            if actual_job == job_id && generation == Generation::first()
    ));
    assert_eq!(
        runtime.auto_fallback_reason.as_deref(),
        Some(
            "WorkerExited before BeginImport admission: Failed(\"WorkerExited: stdin dispatcher stopped\")"
        )
    );
    let active = runtime.active.as_ref().expect("retry pending attempt");
    assert_eq!(active.backend, BackendKind::Cuda);
    assert_eq!(active.recovery, LiveRecoveryState::AutoRetryPending);
    assert!(!active.startup_ready);
    assert!(runtime.transport.is_some());
}

#[test]
fn poll_cancelled_auto_gpu_ready_never_sends_begin_import_before_stop_barrier() {
    let root = TestRoot::new("ready-auto-cancelled");
    let marker = root.0.join("commands.log");
    let worker = compile_command_worker(&root.0, &marker);
    let job_id = JobId(0x99307);
    let mut runtime = runtime_with_active_attempt(job_id);
    set_gpu_attempt(&mut runtime, ComputeChoice::Auto);
    runtime.active.as_mut().expect("attempt").gpu_attested = true;
    let gate = runtime.publication_gate.clone();
    gate.activate();
    assert_eq!(gate.request_stop(), Ok(true));
    runtime.transport = Some(ChildWorkerTransport::spawn(&worker).expect("spawn worker"));
    inject(
        &mut runtime,
        WorkerEvent::Ready {
            job_id,
            generation: Generation::first(),
            instance_id: INSTANCE,
            backend: BackendKind::Cuda,
        },
    );
    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::Ready { .. }))
    ));
    assert!(!runtime.active.as_ref().expect("attempt").startup_ready);
    runtime
        .transport
        .as_mut()
        .expect("transport")
        .request(WorkerCommand::Stop {
            job_id,
            generation: Generation::first(),
        })
        .expect("send barrier Stop");
    assert!(matches!(
        poll_until_stopped(&mut runtime),
        ImportEvent::Stopped { .. }
    ));
    assert_eq!(
        fs::read_to_string(marker).expect("worker command log"),
        "Stop\n"
    );
}

#[test]
fn poll_filters_stale_events_and_limits_unidentified_failure_binding_to_startup() {
    let root = TestRoot::new("identity-filter");
    let worker = compile_idle_worker(&root.0);
    let job_id = JobId(0x99308);
    let mut runtime = runtime_with_active_attempt(job_id);
    set_gpu_attempt(&mut runtime, ComputeChoice::Gpu);
    runtime.transport = Some(ChildWorkerTransport::spawn(&worker).expect("spawn worker"));
    inject(
        &mut runtime,
        attestation(JobId(job_id.0 + 1), &gpu_candidate()),
    );
    assert!(runtime.poll().is_none());
    assert!(!runtime.active.as_ref().expect("attempt").gpu_attested);
    assert!(!runtime.active.as_ref().expect("attempt").startup_ready);
    assert!(runtime.transport.is_some());

    let partial = WorkerEvent::Failed {
        job_id: Some(job_id),
        generation: None,
        instance_id: Some(INSTANCE),
        code: whisper_core::ipc::WorkerErrorCode::WorkerExited,
        message: "partial identity".into(),
    };
    inject(&mut runtime, partial);
    assert!(
        matches!(runtime.poll(), Some(Err((actual, _, message))) if actual == job_id && message.contains("ProtocolMismatch"))
    );
    assert!(runtime.active.is_none());
    assert!(runtime.transport.is_none());

    let job_id = JobId(0x99309);
    let mut runtime = runtime_with_active_attempt(job_id);
    runtime.active.as_mut().expect("attempt").startup_ready = false;
    runtime.transport = Some(ChildWorkerTransport::spawn(&worker).expect("spawn worker"));
    inject(
        &mut runtime,
        WorkerEvent::Failed {
            job_id: None,
            generation: None,
            instance_id: None,
            code: whisper_core::ipc::WorkerErrorCode::WorkerExited,
            message: "unidentified startup failure".into(),
        },
    );
    assert!(
        matches!(runtime.poll(), Some(Err((actual, _, message))) if actual == job_id && message == "unidentified startup failure")
    );
    assert!(runtime.active.is_none());
    assert!(runtime.transport.is_none());
}

#[test]
fn poll_cancelled_ready_does_not_activate_pending_live_state() {
    let root = TestRoot::new("ready-live-cancelled");
    let worker = compile_idle_worker(&root.0);
    let job_id = JobId(0x99310);
    let mut runtime = runtime_with_active_attempt(job_id);
    runtime.active.as_mut().expect("attempt").startup_ready = false;
    let request = live_request(&root.0, job_id);
    let archive = crate::archive::LiveArchive::prepare(&request.destination, request.clone())
        .expect("create live archive");
    let staging = crate::staging::PcmStaging::create(root.0.join("pending-live.pcm"))
        .expect("create live staging")
        .into_writer();
    runtime.pending_live = Some(PendingLiveState {
        request,
        staging,
        live_archive: archive,
        recovery: LiveRecoveryState::None,
    });
    let gate = runtime.publication_gate.clone();
    gate.activate();
    assert_eq!(gate.request_stop(), Ok(true));
    runtime.transport = Some(ChildWorkerTransport::spawn(&worker).expect("spawn worker"));
    inject(
        &mut runtime,
        WorkerEvent::Ready {
            job_id,
            generation: Generation::first(),
            instance_id: INSTANCE,
            backend: BackendKind::Cpu,
        },
    );
    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::Ready { .. }))
    ));
    assert!(runtime.pending_live.is_some());
    assert!(runtime.live.is_none());
    assert!(
        runtime
            .active
            .as_ref()
            .is_some_and(|active| active.startup_ready)
    );
    assert!(runtime.transport.is_some());
}

#[test]
fn poll_live_activation_error_sends_stop_and_waits_for_worker_acknowledgement() {
    let root = TestRoot::new("ready-live-activation-error");
    let marker = root.0.join("commands.log");
    let worker = compile_command_worker(&root.0, &marker);
    let job_id = JobId(0x99311);
    let mut runtime = runtime_with_active_attempt(job_id);
    runtime.active.as_mut().expect("attempt").startup_ready = false;
    let mut request = live_request(&root.0, job_id);
    request.microphone_gain_db = 25;
    let archive = crate::archive::LiveArchive::prepare(&request.destination, request.clone())
        .expect("create live archive");
    let staging = crate::staging::PcmStaging::create(root.0.join("pending-live.pcm"))
        .expect("create live staging")
        .into_writer();
    runtime.pending_live = Some(PendingLiveState {
        request,
        staging,
        live_archive: archive,
        recovery: LiveRecoveryState::None,
    });
    runtime.transport = Some(ChildWorkerTransport::spawn(&worker).expect("spawn worker"));
    inject(
        &mut runtime,
        WorkerEvent::Ready {
            job_id,
            generation: Generation::first(),
            instance_id: INSTANCE,
            backend: BackendKind::Cpu,
        },
    );
    assert!(
        matches!(runtime.poll(), Some(Err((actual, _, message))) if actual == job_id && message.contains("gain"))
    );
    assert!(runtime.pending_live.is_none());
    assert!(runtime.live.is_none());
    assert!(
        runtime
            .active
            .as_ref()
            .is_some_and(|active| active.startup_ready)
    );
    assert!(runtime.transport.is_some());
    assert!(matches!(
        poll_until_stopped(&mut runtime),
        ImportEvent::Stopped { .. }
    ));
    assert_eq!(
        fs::read_to_string(marker).expect("worker command log"),
        "Stop\n"
    );
}
