use super::*;
use std::{
    fs,
    process::Command,
    thread,
    time::{Duration, Instant},
};

fn compile_direct_worker(root: &Path, mode: &str) -> PathBuf {
    let source = root.join(format!("direct-{mode}.rs"));
    let executable = root.join(format!("direct-{mode}.exe"));
    let program = r#"
use std::io::{self, BufRead, Write};
const MODE: &str = "__MODE__";
fn field(line: &str, name: &str) -> String {
    line.split_once(&format!("\"{name}\":"))
        .expect("request identity field")
        .1
        .trim_start()
        .chars()
        .take_while(|character| character.is_ascii_digit())
        .collect()
}
fn main() {
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    let start = lines.next().expect("start command").expect("start frame");
    let job = field(&start, "job_id");
    let generation = field(&start, "generation");
    let instance = field(&start, "instance_id");
    if MODE == "quiet" {
        std::thread::park();
        return;
    }
    if MODE == "failed" {
        println!("{{\"version\":2,\"message\":{{\"Failed\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"code\":\"BackendUnavailable\",\"message\":\"synthetic Direct failure\"}}}}}}");
        let _ = io::stdout().flush();
        return;
    }
    println!("{{\"version\":2,\"message\":{{\"Ready\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"backend\":\"Cpu\"}}}}}}");
    let _ = io::stdout().flush();
    for command in lines.flatten() {
        if command.contains("\"Stop\"") {
            println!("{{\"version\":2,\"message\":{{\"Stopped\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance}}}}}}}");
            let _ = io::stdout().flush();
            return;
        }
    }
}
"#
    .replace("__MODE__", mode);
    fs::write(&source, program).expect("write fake direct worker");
    let output = Command::new("rustc")
        .args(["--edition=2021", "--crate-name", "poll_direct_worker"])
        .arg(&source)
        .arg("-o")
        .arg(&executable)
        .output()
        .expect("compile fake direct worker");
    assert!(
        output.status.success(),
        "rustc fake direct worker: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    executable
}

fn direct_runtime(root: &Path, worker: &Path, job_id: JobId) -> ImportRuntime {
    let request = live_request(root, job_id);
    let capture = crate::capture::CaptureStream::empty_test_capture(16_000, 1, "test:direct-poll");
    let direct = crate::direct_live::DirectLive::from_test_capture_with_cpu_worker(
        request,
        capture,
        worker.to_path_buf(),
        81,
    )
    .expect("start synthetic direct worker");
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: root.join("unused-cpu.exe"),
            gpu: root.join("unused-gpu.exe"),
        },
        Arc::new(PublicationGate::default()),
    );
    runtime.direct_live = Some(direct);
    runtime
}

#[test]
fn poll_direct_none_keeps_capture_and_direct_error_retires_without_worker_cache() {
    let quiet_root = TestRoot::new("direct-poll-quiet");
    let quiet_worker = compile_direct_worker(&quiet_root.0, "quiet");
    let mut runtime = direct_runtime(&quiet_root.0, &quiet_worker, JobId(73_101));
    assert!(
        runtime.poll().is_none(),
        "a quiet direct worker has no event"
    );
    assert!(runtime.direct_live.is_some());

    drop(runtime);
    let failed_root = TestRoot::new("direct-poll-failure");
    let failing_worker = compile_direct_worker(&failed_root.0, "failed");
    let mut runtime = direct_runtime(&failed_root.0, &failing_worker, JobId(73_102));
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut failure = None;
    while Instant::now() < deadline {
        if let Some(result) = runtime.poll() {
            failure = Some(result);
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }
    assert!(matches!(
        failure,
        Some(Err((JobId(73_102), _, message))) if message.contains("synthetic Direct failure")
    ));
    assert!(runtime.direct_live.is_none());
    assert!(runtime.idle_worker.is_none());
}

#[test]
fn poll_direct_terminal_reuses_worker_after_expiring_an_old_idle_worker() {
    let root = TestRoot::new("direct-poll-reuse");
    let direct_worker = compile_direct_worker(&root.0, "ready-stop");
    let idle_worker_path = compile_idle_worker(&root.0);
    let mut runtime = direct_runtime(&root.0, &direct_worker, JobId(73_103));
    runtime.idle_worker = Some(crate::worker_ipc::runtime::IdleWorker {
        transport: crate::worker_ipc::ipc::ChildWorkerTransport::spawn(&idle_worker_path)
            .expect("spawn old idle worker"),
        compute: whisper_core::ComputeChoice::Cpu,
        model_path: "older-model".into(),
        model_sha256: [1; 32],
        expires_at: Instant::now() - Duration::from_secs(1),
    });

    let deadline = Instant::now() + Duration::from_secs(3);
    let mut ready = false;
    while Instant::now() < deadline && !ready {
        ready = matches!(runtime.poll(), Some(Ok(ImportEvent::Ready { .. })));
        assert!(
            runtime.idle_worker.is_none(),
            "expired idle worker is discarded first"
        );
        if !ready {
            thread::sleep(Duration::from_millis(5));
        }
    }
    assert!(
        ready,
        "direct worker Ready is collected through ImportRuntime::poll"
    );
    runtime
        .direct_live
        .as_mut()
        .expect("direct capture stays active")
        .stop()
        .expect("request direct stop");

    let deadline = Instant::now() + Duration::from_secs(3);
    let mut published = false;
    while Instant::now() < deadline && !published {
        if let Some(event) = runtime.poll() {
            match event {
                Ok(ImportEvent::DirectPublished { job_id, .. }) => {
                    assert_eq!(job_id, JobId(73_103));
                    published = true;
                }
                Ok(_) => {}
                Err(error) => panic!("direct stop failed: {error:?}"),
            }
        }
        if !published {
            thread::sleep(Duration::from_millis(5));
        }
    }
    assert!(published, "terminal Direct publication is collected");
    assert!(runtime.direct_live.is_none());
    assert!(
        runtime.idle_worker.is_some(),
        "the finished CPU worker is reusable"
    );
}
