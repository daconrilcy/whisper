use super::*;
use whisper_core::{ComputeChoice, ImportEvent, JobId, gpu_inventory::GpuRuntimeIdentity};

fn manual_gpu_candidate() -> GpuRuntimeIdentity {
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

fn compile_manual_gpu_failure_worker(root: &Path, command_log: &Path) -> PathBuf {
    let source = root.join("manual-gpu-failure-worker.rs");
    let executable = root.join("manual-gpu-failure-worker.exe");
    let command_log = serde_json::to_string(&command_log.to_string_lossy().to_string())
        .expect("encode worker command log path");
    let program = r#"
use std::{fs::OpenOptions, io::{self, BufRead, Write}};
const LOG: &str = __LOG__;
fn field(line: &str, name: &str) -> String {
    let token = format!("\"{name}\":");
    let tail = line.split_once(&token).expect("wire identity field").1.trim_start();
    tail.chars().take_while(|c| c.is_ascii_digit()).collect()
}
fn record(value: &str) {
    let mut file = OpenOptions::new().create(true).append(true).open(LOG).unwrap();
    writeln!(file, "{value}").unwrap();
}
fn main() {
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    let start = lines.next().expect("GPU live start command").expect("start frame");
    assert!(start.contains("\"StartGpuLiveWorker\""), "expected manual GPU live start");
    let job = field(&start, "job_id");
    let generation = field(&start, "generation");
    let instance = field(&start, "instance_id");
    let identity = format!("{job}:{generation}:{instance}");
    record(&format!("gpu-start:{identity}"));
    record(&format!("gpu-failed:{identity}"));
    println!("{{\"version\":2,\"message\":{{\"Failed\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"code\":\"BackendUnavailable\",\"message\":\"simulated manual GPU initialization failure\"}}}}}}");
    io::stdout().flush().expect("flush Failed event");
    for command in lines.flatten() {
        if command.contains("\"Stop\"") {
            record(&format!("gpu-stop:{identity}"));
            println!("{{\"version\":2,\"message\":{{\"Stopped\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance}}}}}}}");
            io::stdout().flush().expect("flush Stopped acknowledgement");
            record("gpu-exit");
            return;
        }
    }
}
"#
    .replace("__LOG__", &command_log);
    fs::write(&source, program).expect("write manual GPU failure worker");
    let output = Command::new("rustc")
        .args([
            "--edition=2021",
            "--crate-name",
            "poll_manual_gpu_failure_worker",
        ])
        .arg(&source)
        .arg("-o")
        .arg(&executable)
        .output()
        .expect("compile manual GPU failure worker");
    assert!(
        output.status.success(),
        "rustc manual GPU failure worker: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    executable
}

fn compile_cpu_launch_probe(root: &Path, marker: &Path) -> PathBuf {
    let source = root.join("cpu-launch-probe.rs");
    let executable = root.join("cpu-launch-probe.exe");
    let marker = serde_json::to_string(&marker.to_string_lossy().to_string())
        .expect("encode CPU launch marker path");
    let program = r#"
use std::{fs, io::{self, BufRead}};
const MARKER: &str = __MARKER__;
fn main() {
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    if lines.next().is_some() {
        fs::write(MARKER, "CPU worker launched").expect("record CPU worker launch");
    }
}
"#
    .replace("__MARKER__", &marker);
    fs::write(&source, program).expect("write CPU launch probe");
    let output = Command::new("rustc")
        .args(["--edition=2021", "--crate-name", "poll_cpu_launch_probe"])
        .arg(&source)
        .arg("-o")
        .arg(&executable)
        .output()
        .expect("compile CPU launch probe");
    assert!(
        output.status.success(),
        "rustc CPU launch probe: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    executable
}

fn poll_until_cpu_choice(runtime: &mut ImportRuntime, request: &whisper_core::ports::LiveRequest) {
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut saw_finalizing = false;
    let mut saw_cpu_choice = false;
    while Instant::now() < deadline {
        match runtime.poll() {
            Some(Ok(ImportEvent::LiveFinalizing {
                job_id: actual_job,
                generation,
            })) => {
                assert_eq!(
                    (actual_job, generation),
                    (request.job_id, request.generation)
                );
                saw_finalizing = true;
                assert_eq!(
                    runtime.pending_live.as_ref().unwrap().recovery,
                    LiveRecoveryState::AwaitingChoicePending
                );
            }
            Some(Ok(ImportEvent::AwaitingCpuChoice {
                job_id: actual_job,
                generation,
            })) => {
                assert_eq!(
                    (actual_job, generation),
                    (request.job_id, request.generation)
                );
                saw_cpu_choice = true;
                break;
            }
            Some(Ok(event)) => panic!("unexpected event before CPU choice: {event:?}"),
            Some(Err(error)) => panic!("manual GPU failure handling failed: {error:?}"),
            None => std::thread::sleep(Duration::from_millis(2)),
        }
    }
    assert!(
        saw_finalizing,
        "GPU failure did not enter Stop/finalization"
    );
    assert!(
        saw_cpu_choice,
        "Stop acknowledgement did not expose the CPU choice"
    );
}

fn assert_cpu_choice_state(
    runtime: &ImportRuntime,
    request: &whisper_core::ports::LiveRequest,
    cpu_launch_marker: &Path,
) {
    assert!(
        runtime.active.is_none(),
        "GPU attempt remains active after Stop"
    );
    assert!(
        runtime.transport.is_none(),
        "GPU transport remains after Stop"
    );
    assert!(
        runtime.live.is_none(),
        "manual GPU startup unexpectedly activated capture"
    );
    assert!(!runtime.publication_gate.is_cancelled());
    let pending = runtime
        .pending_live
        .as_ref()
        .expect("pending live request preserved");
    assert_eq!(pending.recovery, LiveRecoveryState::AwaitingChoice);
    assert_eq!(pending.request.job_id, request.job_id);
    assert_eq!(pending.request.generation, request.generation);
    assert_eq!(pending.request.model_path, request.model_path);
    assert_eq!(pending.request.model_sha256, request.model_sha256);
    assert_eq!(pending.request.destination, request.destination);
    assert_eq!(pending.request.config.compute, ComputeChoice::Gpu);
    assert!(
        !cpu_launch_marker.exists(),
        "CPU worker launched before user choice"
    );
}

#[test]
fn poll_manual_gpu_failure_waits_for_explicit_cpu_choice_after_stop_ack() {
    let root = TestRoot::new("manual-gpu-choice");
    let command_log = root.0.join("gpu-worker-commands.log");
    let cpu_launch_marker = root.0.join("cpu-worker-launched");
    let gpu_worker = compile_manual_gpu_failure_worker(&root.0, &command_log);
    let cpu_worker = compile_cpu_launch_probe(&root.0, &cpu_launch_marker);
    let job_id = JobId(0x99320);
    let mut request = live_request(&root.0, job_id);
    request.config.compute = ComputeChoice::Gpu;
    let candidate = manual_gpu_candidate();
    let mut runtime = runtime_with_active_attempt(job_id);
    runtime.active = None;
    runtime.workers = crate::supervisor::WorkerPaths {
        cpu: cpu_worker,
        gpu: gpu_worker,
    };

    runtime
        .start_live(request.clone(), Some(candidate.clone()))
        .expect("start live pass on the manually selected GPU");
    let initial_attempt = runtime.active.as_ref().expect("GPU attempt");
    let instance_id = initial_attempt.instance_id;
    assert_eq!(initial_attempt.backend, BackendKind::Cuda);
    assert_eq!(initial_attempt.gpu_candidate, Some(candidate));
    assert!(!initial_attempt.startup_ready);
    assert_eq!(
        runtime
            .pending_live
            .as_ref()
            .unwrap()
            .request
            .config
            .compute,
        ComputeChoice::Gpu
    );

    poll_until_cpu_choice(&mut runtime, &request);
    assert_cpu_choice_state(&runtime, &request, &cpu_launch_marker);

    let identity = format!("{}:{}:{instance_id}", job_id.0, request.generation.get());
    assert_eq!(
        fs::read_to_string(command_log).expect("read GPU worker command log"),
        format!("gpu-start:{identity}\ngpu-failed:{identity}\ngpu-stop:{identity}\ngpu-exit\n")
    );
}
