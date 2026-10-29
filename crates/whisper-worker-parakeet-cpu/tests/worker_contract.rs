use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{ChildStdin, Command, Stdio},
    sync::mpsc,
    thread,
    time::Duration,
};

#[path = "support/worker.rs"]
mod support;

fn campaign_root() -> PathBuf {
    let root =
        PathBuf::from(std::env::var_os("WHISPER_TEST_WORK_ROOT").expect("work root required"));
    let allowed = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/parakeet-tests/main-facades")
        .canonicalize()
        .unwrap();
    let root = root.canonicalize().unwrap();
    assert!(root.starts_with(allowed), "work root outside campaign");
    root
}

fn decode_case(
    executable: &Path,
    root: &Path,
    source: &Path,
    name: &str,
) -> (serde_json::Value, Vec<u8>) {
    let local = root.join(name).join("localappdata");
    let runtime = local.join("Whisper");
    fs::create_dir_all(&runtime).unwrap();
    let output = runtime.join("audio.pcm");
    let run = || {
        let mut command = Command::new(executable);
        command
            .arg("--decode-only")
            .arg(source)
            .arg(&output)
            .env("LOCALAPPDATA", &local)
            .env("PARAKEET_L2_RUNTIME_ROOT", &runtime);
        support::output(&mut command, None, support::CONTROLLED_TIMEOUT)
    };
    let first = run();
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let json = serde_json::from_slice(&first.stdout).unwrap();
    let pcm = fs::read(&output).unwrap();
    let second = run();
    assert!(!second.status.success(), "existing output must be refused");
    assert_eq!(fs::read(&output).unwrap(), pcm, "existing output changed");
    (json, pcm)
}

#[cfg(feature = "parakeet-personal")]
#[test]
fn parakeet_decode_only_contract() {
    let root = campaign_root();
    let source = root.join("reference-short.wav");
    let reference =
        std::env::var_os("WHISPER_TEST_REFERENCE_EXE").expect("reference executable required");
    let candidate =
        std::env::var_os("WHISPER_TEST_WORKER_EXE").expect("candidate executable required");
    let unique = format!(
        "decode-only-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let run_root = root.join(unique);
    fs::create_dir(&run_root).unwrap();
    let before = decode_case(Path::new(&reference), &run_root, &source, "reference");
    let after = decode_case(Path::new(&candidate), &run_root, &source, "candidate");
    assert_eq!(before, after);
}

struct NativeWorker {
    child: support::BoundedChild,
    input: Option<ChildStdin>,
    events: mpsc::Receiver<String>,
    reader: Option<thread::JoinHandle<()>>,
}

fn sha256(path: &Path) -> String {
    let mut file = fs::File::open(path).unwrap();
    let mut hash = Sha256::new();
    let mut bytes = [0_u8; 1024 * 1024];
    loop {
        let count = file.read(&mut bytes).unwrap();
        if count == 0 {
            break;
        }
        hash.update(&bytes[..count]);
    }
    format!("{:x}", hash.finalize())
}

fn start_worker(executable: &Path) -> NativeWorker {
    let runtime = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap()).join("Whisper");
    let model = runtime.join("models/parakeet-tdt-0.6b-v3.q8_0.gguf");
    let cuda = std::env::var_os("WHISPER_TEST_PARAKEET_CUDA").as_deref()
        == Some(std::ffi::OsStr::new("1"));
    let native = runtime.join(if cuda { "native/gpu" } else { "native/cpu" });
    let mut command = Command::new(executable);
    command
        .args([
            "--model",
            model.to_str().unwrap(),
            "--model-sha256",
            &sha256(&model),
            "--native-dir",
            native.to_str().unwrap(),
        ])
        .env("PARAKEET_L2_RUNTIME_ROOT", &runtime)
        .env("PARAKEET_R7_TIMING_ACTIVE", "0")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped());
    if cuda {
        let directory = PathBuf::from(
            std::env::var_os("WHISPER_TEST_GPU_LOG_DIR").expect("CUDA log directory required"),
        );
        assert_eq!(
            directory.canonicalize().unwrap(),
            runtime.join("settings/logs").canonicalize().unwrap()
        );
        let name = format!(
            "main-facades-{}-{}.log",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let log = directory.join(name);
        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&log)
            .unwrap();
        command
            .env("PARAKEET_L3_ASR_LOG", &log)
            .stderr(Stdio::from(file));
    } else {
        let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/parakeet-tests/main-facades/logs")
            .canonicalize()
            .unwrap();
        let name = format!(
            "parakeet-cpu-{}-{}.stderr.txt",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join(name))
            .unwrap();
        command.stderr(Stdio::from(file));
    }
    let child = support::BoundedChild::spawn(&mut command, support::NATIVE_TIMEOUT);
    let input = Some(child.stdin());
    let stdout = child.stdout();
    let (tx, events) = mpsc::channel();
    let reader = thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            let line = line.unwrap();
            eprintln!("captured Parakeet event: {line}");
            if tx.send(line).is_err() {
                break;
            }
        }
    });
    NativeWorker {
        child,
        input,
        events,
        reader: Some(reader),
    }
}

impl NativeWorker {
    fn send(&mut self, message: serde_json::Value) {
        let input = self.input.as_mut().unwrap();
        serde_json::to_writer(&mut *input, &message).unwrap();
        input.write_all(b"\n").unwrap();
        input.flush().unwrap();
    }
    fn event(&self) -> serde_json::Value {
        let line = self
            .events
            .recv_timeout(self.child.remaining().min(Duration::from_secs(180)))
            .expect("worker event timeout");
        let value: serde_json::Value = serde_json::from_str(&line).unwrap();
        assert_eq!(value["version"], 1);
        value["message"].clone()
    }
    fn finish(mut self) -> std::process::ExitStatus {
        drop(self.input.take());
        let status = self.child.wait().expect("Parakeet worker deadline");
        support::finish_reader(self.reader.take().unwrap(), &self.child);
        assert!(
            self.events.try_iter().next().is_none(),
            "unexpected trailing event"
        );
        status
    }
}

fn worker_paths() -> (PathBuf, PathBuf) {
    (
        PathBuf::from(
            std::env::var_os("WHISPER_TEST_REFERENCE_EXE").expect("reference executable required"),
        ),
        PathBuf::from(
            std::env::var_os("WHISPER_TEST_WORKER_EXE").expect("candidate executable required"),
        ),
    )
}

fn ready(
    worker: &NativeWorker,
    executable: &Path,
) -> (serde_json::Value, Option<serde_json::Value>) {
    let event = worker.event();
    assert_eq!(event["type"], "ready");
    let cuda = std::env::var_os("WHISPER_TEST_PARAKEET_CUDA").as_deref()
        == Some(std::ffi::OsStr::new("1"));
    assert_eq!(event["backend"], if cuda { "CUDA0" } else { "CPU" });
    assert_eq!(event["identity"]["build_sha256"], sha256(executable));
    let gpu = if cuda {
        let evidence = &event["gpu_evidence"];
        assert_eq!(evidence["backend"], "CUDA0");
        assert_eq!(evidence["pid"], worker.child.id());
        assert_eq!(evidence["asr_log_sha256"].as_str().unwrap().len(), 64);
        Some(serde_json::json!({"pci_bus_id":evidence["pci_bus_id"], "uuid":evidence["uuid"]}))
    } else {
        None
    };
    (event["identity"].clone(), gpu)
}

fn normalize(mut event: serde_json::Value) -> serde_json::Value {
    if let Some(identity) = event
        .get_mut("identity")
        .and_then(serde_json::Value::as_object_mut)
    {
        identity.insert("build_sha256".into(), "<executable>".into());
    }
    if let Some(object) = event.as_object_mut() {
        if object.contains_key("native_inference_ms") {
            object.insert("native_inference_ms".into(), 0.into());
        }
        if let Some(evidence) = object
            .get_mut("gpu_evidence")
            .and_then(serde_json::Value::as_object_mut)
        {
            evidence.insert("pid".into(), 0.into());
            evidence.insert("asr_log_sha256".into(), "<captured-log>".into());
        }
    }
    event
}

fn request(
    id: u64,
    job: &str,
    attempt: u64,
    identity: serde_json::Value,
    source: &Path,
    sha: &str,
    gpu: Option<serde_json::Value>,
) -> serde_json::Value {
    serde_json::json!({"version":1,"message":{"type":"transcribe","request_id":id,"job_id":job,
        "generation":1,"attempt":attempt,"identity":identity,"source_path":source,"source_sha256":sha,"expected_gpu":gpu}})
}

fn refusal_case(executable: &Path) -> Vec<serde_json::Value> {
    let mut worker = start_worker(executable);
    let (identity, gpu) = ready(&worker, executable);
    let source = campaign_root().join("reference-short.wav");
    let mut wrong = identity.clone();
    wrong["model_sha256"] = "wrong".into();
    worker.send(request(
        1,
        "refusal",
        1,
        wrong,
        &source,
        "wrong",
        gpu.clone(),
    ));
    let first = worker.event();
    worker.send(request(
        1,
        "refusal",
        1,
        identity.clone(),
        &source,
        "wrong",
        gpu.clone(),
    ));
    let second = worker.event();
    worker.send(request(
        2,
        "refusal",
        1,
        identity.clone(),
        &source,
        "wrong",
        gpu.clone(),
    ));
    let third = worker.event();
    worker.send(request(3, "refusal", 1, identity, &source, "wrong", gpu));
    let fourth = worker.event();
    assert_eq!(
        [
            first["code"].as_str(),
            second["code"].as_str(),
            third["code"].as_str(),
            fourth["code"].as_str()
        ],
        [
            Some("ModelHashMismatch"),
            Some("StaleRequest"),
            Some("SourceHashMismatch"),
            Some("StaleAttempt")
        ]
    );
    worker.send(serde_json::json!({"version":1,"message":{"type":"shutdown"}}));
    assert_eq!(worker.event()["type"], "stopped");
    assert!(worker.finish().success());
    [first, second, third, fourth]
        .into_iter()
        .map(normalize)
        .collect()
}

#[cfg(feature = "parakeet-personal")]
#[test]
#[ignore = "requires the installed Parakeet CPU model and runtime"]
fn parakeet_refusal_order_contract() {
    let (reference, candidate) = worker_paths();
    assert_eq!(refusal_case(&reference), refusal_case(&candidate));
}

fn errors_case(executable: &Path) -> Vec<serde_json::Value> {
    let mut worker = start_worker(executable);
    ready(&worker, executable);
    worker
        .input
        .as_mut()
        .unwrap()
        .write_all(b"{bad json}\n")
        .unwrap();
    let json_error = worker.event();
    worker.send(serde_json::json!({"version":99,"message":{"type":"shutdown"}}));
    let version_error = worker.event();
    assert_eq!(json_error["code"], "InvalidMessage");
    assert_eq!(version_error["code"], "ProtocolVersionMismatch");
    worker.send(serde_json::json!({"version":1,"message":{"type":"shutdown"}}));
    assert_eq!(worker.event()["type"], "stopped");
    assert!(worker.finish().success());

    let worker = start_worker(executable);
    ready(&worker, executable);
    assert!(worker.finish().success());

    let mut worker = start_worker(executable);
    ready(&worker, executable);
    let _ = worker
        .input
        .as_mut()
        .unwrap()
        .write_all(&vec![b'x'; 1_048_577]);
    assert_eq!(worker.finish().code(), Some(2));
    vec![normalize(json_error), normalize(version_error)]
}

#[cfg(feature = "parakeet-personal")]
#[test]
#[ignore = "requires the installed Parakeet CPU model and runtime"]
fn parakeet_errors_and_shutdown_contract() {
    let (reference, candidate) = worker_paths();
    assert_eq!(errors_case(&reference), errors_case(&candidate));
}

fn native_case(executable: &Path) -> serde_json::Value {
    let mut worker = start_worker(executable);
    let (identity, gpu) = ready(&worker, executable);
    let source = campaign_root().join("reference-short.wav");
    worker.send(request(
        1,
        "native",
        1,
        identity,
        &source,
        &sha256(&source),
        gpu.clone(),
    ));
    let mut result = worker.event();
    if gpu.is_some() {
        assert_eq!(result["type"], "native_call_started");
        assert_eq!(result["request_id"], 1);
        assert_eq!(result["pid"], worker.child.id());
        result = worker.event();
        assert_eq!(result["gpu_evidence"]["backend"], "CUDA0");
        assert_eq!(result["gpu_evidence"]["pid"], worker.child.id());
        assert_eq!(
            result["gpu_evidence"]["asr_log_sha256"]
                .as_str()
                .unwrap()
                .len(),
            64
        );
    }
    assert_eq!(result["type"], "result", "{result}");
    assert_eq!(result["request_id"], 1);
    worker.send(serde_json::json!({"version":1,"message":{"type":"shutdown"}}));
    assert_eq!(worker.event()["type"], "stopped");
    assert!(worker.finish().success());
    normalize(result)
}

#[cfg(feature = "parakeet-personal")]
#[test]
#[ignore = "requires the installed Parakeet CPU model and runtime"]
fn parakeet_native_transcription_contract() {
    let (reference, candidate) = worker_paths();
    assert_eq!(native_case(&reference), native_case(&candidate));
}
