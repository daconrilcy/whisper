use std::{
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{ChildStdin, Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread::{self, JoinHandle},
};

use sha2::{Digest, Sha256};
use whisper_core::parakeet_ipc::{
    Command as WorkerCommand, Envelope, Event, ExpectedGpu, GpuEvidence,
    PARAKEET_PROFILE_IMPORT_SINGLE_PASS, WorkerIdentity,
};

#[path = "support/worker.rs"]
mod support;

const MODEL_SHA: &str = "e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e";
const CPU_RUNTIME: &str = "d5a2cc8ef441dd5934972adae00f997241bea17a7bc5977a8c7f0122aaa958ff";
const GPU_RUNTIME: &str = "cbf7543a711a341df4387fb9047fcd55d3d5c572465cbe9bbb7d14b5963e6b10";
const CPU_REFERENCE: &str = "84c371d4df55043ada4a1d96b80180ed0c03b208410c599262c8ea3762304e60";
const GPU_REFERENCE: &str = "bf58eb9d7300c444864347496b5a27e8584da3d30d634a759be99a598eda5ce4";
const CPU_CANDIDATE: &str = "a965c9503f909124f8ddedad09e83e5d187e70edd1869e880e9f1ad5ae2fb99d";
const GPU_CANDIDATE: &str = "7ffb0b4a022ec26909a8939b37787de6df6c9bdb2903edf14d3ffb32579eefc1";
const OLD_VARS: [&str; 5] = [
    "PARAKEET_R6_TIMING_LOG",
    "PARAKEET_R6_CASE",
    "PARAKEET_R7_TIMING_ACTIVE",
    "PARAKEET_R7_EXEC_ROOT",
    "PARAKEET_R7_TIMING_LOG",
];

#[derive(Debug, PartialEq)]
struct Outcome {
    text: String,
    words: Vec<whisper_core::parakeet_ipc::Word>,
    samples: u64,
}

struct Worker {
    child: support::BoundedChild,
    input: ChildStdin,
    events: Receiver<String>,
    reader: JoinHandle<()>,
}

impl Worker {
    fn event(&self) -> Event {
        let line = self
            .events
            .recv_timeout(self.child.remaining())
            .expect("worker event deadline");
        let envelope: Envelope<Event> = serde_json::from_str(&line).expect("valid IPC event");
        assert_eq!(envelope.version, 1);
        envelope.message
    }
    fn send(&mut self, command: WorkerCommand) {
        serde_json::to_writer(&mut self.input, &Envelope::new(command)).unwrap();
        self.input.write_all(b"\n").unwrap();
        self.input.flush().unwrap();
    }
    fn finish(mut self) {
        self.send(WorkerCommand::Shutdown);
        assert!(matches!(self.event(), Event::Stopped));
        drop(self.input);
        assert!(self.child.wait().expect("worker shutdown").success());
        support::finish_reader(self.reader, &self.child);
    }
}

fn sha(path: &Path) -> String {
    let mut file = File::open(path).unwrap();
    let mut digest = Sha256::new();
    std::io::copy(&mut file, &mut digest).unwrap();
    format!("{:x}", digest.finalize())
}

fn fixture(root: &Path, repository: &Path) -> PathBuf {
    let mut input = File::open(repository.join("sound example/spkr0.wav")).unwrap();
    let mut header = [0_u8; 44];
    input.read_exact(&mut header).unwrap();
    assert_eq!(&header[0..4], b"RIFF");
    assert_eq!(&header[8..16], b"WAVEfmt ");
    assert_eq!(
        &header[20..36],
        &[1, 0, 1, 0, 128, 62, 0, 0, 0, 125, 0, 0, 2, 0, 16, 0]
    );
    assert_eq!(&header[36..40], b"data");
    let mut samples = vec![0; 30 * 16_000 * 2];
    input.read_exact(&mut samples).unwrap();
    header[4..8].copy_from_slice(&((samples.len() + 36) as u32).to_le_bytes());
    header[40..44].copy_from_slice(&(samples.len() as u32).to_le_bytes());
    let path = root
        .join("input")
        .join(format!("speech-{}.wav", std::process::id()));
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .unwrap();
    output.write_all(&header).unwrap();
    output.write_all(&samples).unwrap();
    path
}

fn evidence_check(evidence: &GpuEvidence, pid: u32, log: &Path) {
    assert_eq!(evidence.backend, "CUDA0");
    assert_eq!(
        evidence.pci_bus_id,
        whisper_core::model_registry::PARAKEET_PERSONAL_GPU_PCI_BUS_ID
    );
    assert_eq!(
        evidence.uuid,
        whisper_core::model_registry::PARAKEET_PERSONAL_GPU_UUID
    );
    assert_eq!(evidence.pid, pid);
    assert_eq!(evidence.asr_log_sha256, sha(log));
    assert!(
        fs::read_to_string(log)
            .unwrap()
            .lines()
            .any(|line| line.starts_with("[asr]") && line.contains("backend=CUDA0"))
    );
}

fn run(root: &Path, wav: &Path, backend: &str, case: &str, poisoned: bool) -> Outcome {
    let local = root.join("localappdata");
    let runtime = local.join("Whisper");
    let model = runtime.join("models/parakeet-tdt-0.6b-v3.q8_0.gguf");
    assert_eq!(sha(&model), MODEL_SHA);
    let native = runtime.join("native").join(backend);
    let executable = if case == "reference" {
        root.join("reference")
            .join(format!("whisper-worker-parakeet-{backend}.exe"))
    } else {
        runtime
            .join("worker")
            .join(format!("whisper-worker-parakeet-{backend}.exe"))
    };
    let expected_build = match (backend, case) {
        ("cpu", "reference") => CPU_REFERENCE,
        ("gpu", "reference") => GPU_REFERENCE,
        ("cpu", _) => CPU_CANDIDATE,
        ("gpu", _) => GPU_CANDIDATE,
        _ => unreachable!(),
    };
    assert_eq!(sha(&executable), expected_build);
    let log = runtime.join("settings").join(format!(
        "native-{backend}-{case}-{}.log",
        std::process::id()
    ));
    let stderr = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&log)
        .unwrap();
    let invalid = root
        .join("outputs")
        .join(format!("missing-{backend}-{case}-{}", std::process::id()));
    assert!(!invalid.exists());
    let mut command = Command::new(&executable);
    command
        .args([
            "--model",
            model.to_str().unwrap(),
            "--model-sha256",
            MODEL_SHA,
            "--native-dir",
            native.to_str().unwrap(),
        ])
        .env("LOCALAPPDATA", &local)
        .env("PARAKEET_L2_RUNTIME_ROOT", &runtime)
        .env("PARAKEET_L3_ASR_LOG", &log)
        .env("CUDA_CACHE_PATH", root.join("cuda-cache"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::from(stderr));
    for variable in OLD_VARS {
        command.env_remove(variable);
    }
    if poisoned {
        command
            .env("PARAKEET_R6_TIMING_LOG", invalid.join("r6.log"))
            .env("PARAKEET_R6_CASE", "invalid")
            .env("PARAKEET_R7_TIMING_ACTIVE", "1")
            .env("PARAKEET_R7_EXEC_ROOT", &invalid)
            .env("PARAKEET_R7_TIMING_LOG", invalid.join("r7.log"));
    }
    let child = support::BoundedChild::spawn(&mut command, support::NATIVE_TIMEOUT);
    let input = child.stdin();
    let stdout = child.stdout();
    let (sender, events) = mpsc::channel();
    let reader = thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            if sender.send(line.unwrap()).is_err() {
                break;
            }
        }
    });
    let mut worker = Worker {
        child,
        input,
        events,
        reader,
    };
    let Event::Ready {
        identity,
        backend: actual_backend,
        native_version,
        gpu_evidence,
    } = worker.event()
    else {
        panic!("expected ready")
    };
    let expected_runtime = if backend == "gpu" {
        GPU_RUNTIME
    } else {
        CPU_RUNTIME
    };
    assert_eq!(
        identity,
        WorkerIdentity {
            model_sha256: MODEL_SHA.into(),
            runtime_sha256: expected_runtime.into(),
            build_sha256: expected_build.into(),
            profile: PARAKEET_PROFILE_IMPORT_SINGLE_PASS.into()
        }
    );
    assert_eq!(native_version, "nemo-speech-asr 0.2.0");
    assert_eq!(
        actual_backend,
        if backend == "gpu" { "CUDA0" } else { "CPU" }
    );
    let gpu = if backend == "gpu" {
        evidence_check(
            gpu_evidence.as_ref().expect("GPU ready evidence"),
            worker.child.id(),
            &log,
        );
        Some(ExpectedGpu {
            pci_bus_id: whisper_core::model_registry::PARAKEET_PERSONAL_GPU_PCI_BUS_ID.into(),
            uuid: whisper_core::model_registry::PARAKEET_PERSONAL_GPU_UUID.into(),
        })
    } else {
        assert!(gpu_evidence.is_none());
        None
    };
    let source_sha = sha(wav);
    worker.send(WorkerCommand::Transcribe {
        request_id: 1,
        job_id: "native-diagnostics".into(),
        generation: 1,
        attempt: 1,
        identity: Box::new(identity.clone()),
        source_path: wav.to_string_lossy().into_owned(),
        source_sha256: source_sha.clone(),
        expected_gpu: gpu,
    });
    let result = if backend == "gpu" {
        assert!(
            matches!(worker.event(), Event::NativeCallStarted { request_id: 1, generation: 1, attempt: 1, pid, .. } if pid == worker.child.id())
        );
        worker.event()
    } else {
        worker.event()
    };
    let Event::Result {
        request_id,
        job_id,
        generation,
        attempt,
        identity: result_identity,
        source_sha256,
        duration_samples,
        text,
        words,
        gpu_evidence,
        ..
    } = result
    else {
        panic!("worker returned {result:?}")
    };
    assert_eq!(
        (request_id, job_id.as_str(), generation, attempt),
        (1, "native-diagnostics", 1, 1)
    );
    assert_eq!(result_identity, identity);
    assert_eq!(source_sha256, source_sha);
    assert_eq!(duration_samples, 480_000);
    assert!(
        !text.trim().is_empty() && !words.is_empty(),
        "fixture must contain intelligible speech"
    );
    if backend == "gpu" {
        evidence_check(
            gpu_evidence.as_ref().expect("GPU result evidence"),
            worker.child.id(),
            &log,
        );
    } else {
        assert!(gpu_evidence.is_none());
    }
    let outcome = Outcome {
        text,
        words,
        samples: duration_samples,
    };
    worker.finish();
    assert!(
        !invalid.exists(),
        "obsolete diagnostic path must not be created"
    );
    println!(
        "native backend={backend} case={case} poisoned={poisoned} words={} samples={} text={:?}",
        outcome.words.len(),
        outcome.samples,
        outcome.text
    );
    outcome
}

#[test]
#[ignore = "requires frozen personal workers, native CPU/GPU and the local audio fixture"]
fn native_workers_preserve_transcription_without_historical_diagnostics() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let root = repository
        .join("target/parakeet-tests/refacto-parakeet-live/p3-workers")
        .canonicalize()
        .unwrap();
    let wav = fixture(&root, &repository);
    for backend in ["cpu", "gpu"] {
        let reference = run(&root, &wav, backend, "reference", false);
        let candidate = run(&root, &wav, backend, "candidate", false);
        let poisoned = run(&root, &wav, backend, "poisoned", true);
        assert_eq!(
            candidate, reference,
            "candidate must preserve {backend} text and timed words"
        );
        assert_eq!(
            poisoned, candidate,
            "historical diagnostics must not alter {backend} output"
        );
    }
}
