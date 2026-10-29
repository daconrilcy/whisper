use std::{
    fs::{self, File},
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

#[path = "support/worker.rs"]
mod support;

use sha2::{Digest, Sha256};
use whisper_core::{
    ComputeChoice, Generation, JobConfig, JobId, LanguageChoice,
    ipc::{BackendKind, IpcEnvelope, WorkerCommand, WorkerEvent},
};

fn hash_file(path: &Path) -> [u8; 32] {
    let mut file = File::open(path).unwrap();
    let mut hash = Sha256::new();
    let mut buffer = [0; 1024 * 1024];
    loop {
        let read = file.read(&mut buffer).unwrap();
        if read == 0 {
            break;
        }
        hash.update(&buffer[..read]);
    }
    hash.finalize().into()
}

fn write_silent_wav(path: &Path) {
    let sample_count = 16_000_u32;
    let data_bytes = sample_count * 2;
    let mut wav = Vec::with_capacity(44 + data_bytes as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16_u32.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&16_000_u32.to_le_bytes());
    wav.extend_from_slice(&32_000_u32.to_le_bytes());
    wav.extend_from_slice(&2_u16.to_le_bytes());
    wav.extend_from_slice(&16_u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_bytes.to_le_bytes());
    wav.resize(44 + data_bytes as usize, 0);
    fs::write(path, wav).unwrap();
}

fn send<W: Write>(writer: &mut W, message: WorkerCommand) {
    serde_json::to_writer(&mut *writer, &IpcEnvelope::new(message)).unwrap();
    writer.write_all(b"\n").unwrap();
    writer.flush().unwrap();
}

fn await_end(
    events: &mpsc::Receiver<Result<IpcEnvelope<WorkerEvent>, String>>,
    job_id: JobId,
    instance_id: u64,
    child: &support::BoundedChild,
) -> Vec<IpcEnvelope<WorkerEvent>> {
    let mut observed = Vec::new();
    let deadline = Instant::now() + Duration::from_secs(180);
    let mut ready = false;
    loop {
        let remaining = deadline
            .saturating_duration_since(Instant::now())
            .min(child.remaining());
        assert!(!remaining.is_zero(), "import worker session timed out");
        let envelope = events
            .recv_timeout(remaining)
            .expect("import worker session timed out")
            .unwrap();
        eprintln!("captured worker event: {envelope:?}");
        observed.push(envelope.clone());
        match envelope.message {
            WorkerEvent::Ready {
                job_id: actual,
                generation,
                instance_id: actual_instance,
                backend,
            } => {
                assert_eq!(actual, job_id);
                assert_eq!(generation, Generation::first());
                assert_eq!(actual_instance, instance_id);
                assert_eq!(backend, BackendKind::Cpu);
                assert!(!ready, "duplicate Ready for one session");
                ready = true;
            }
            WorkerEvent::End {
                job_id: actual,
                generation,
                instance_id: actual_instance,
                ..
            } => {
                assert!(ready, "End arrived before Ready");
                assert_eq!(actual, job_id);
                assert_eq!(generation, Generation::first());
                assert_eq!(actual_instance, instance_id);
                return observed;
            }
            WorkerEvent::Failed { message, .. } => panic!("worker failed: {message}"),
            _ => {}
        }
    }
}

#[test]
#[ignore = "charge le modèle large-v3-turbo ; lancer explicitement en Release avec WHISPER_TEST_REUSE_MODEL"]
pub(crate) fn cpu_worker_serves_two_imports_in_one_process_and_hits_model_cache() {
    let _ = run_for_exe(Path::new(env!("CARGO_BIN_EXE_whisper-worker-cpu")));
}

pub(crate) fn run_for_exe(executable: &Path) -> Vec<IpcEnvelope<WorkerEvent>> {
    let model_path = std::env::var_os("WHISPER_TEST_REUSE_MODEL")
        .map(PathBuf::from)
        .expect("set WHISPER_TEST_REUSE_MODEL to the local model path");
    assert!(model_path.is_file(), "the configured model must exist");

    let allowed = fs::canonicalize(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/parakeet-tests/main-facades"),
    )
    .expect("the authorized campaign root must exist");
    let work_root = std::env::var_os("WHISPER_TEST_WORK_ROOT")
        .map(PathBuf::from)
        .expect("set WHISPER_TEST_WORK_ROOT inside the authorized campaign root");
    let work_root = fs::canonicalize(work_root).expect("the configured work root must exist");
    assert!(
        work_root.starts_with(&allowed),
        "work root is outside the campaign"
    );
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = work_root.join(format!("reusable-imports-{}-{unique}", std::process::id()));
    fs::create_dir(&root).expect("the test execution directory must be new");
    let source_path = root.join("silent.wav");
    write_silent_wav(&source_path);
    let source_sha256 = hash_file(&source_path);
    let model_sha256 = hash_file(&model_path);
    let mut command = Command::new(executable);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let child = support::BoundedChild::spawn(&mut command, support::NATIVE_TIMEOUT);
    let mut input = child.stdin();
    let stdout = child.stdout();
    let mut stderr = child.stderr();
    let stderr_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        let mut chunk = [0_u8; 4096];
        loop {
            let n = stderr.read(&mut chunk).unwrap();
            if n == 0 {
                break;
            }
            eprint!("{}", String::from_utf8_lossy(&chunk[..n]));
            bytes.extend_from_slice(&chunk[..n]);
        }
        bytes
    });
    let (event_tx, event_rx) = mpsc::channel();
    let stdout_reader = thread::spawn(move || {
        let mut output = BufReader::new(stdout);
        let mut line = String::new();
        loop {
            line.clear();
            match output.read_line(&mut line) {
                Ok(0) => return,
                Ok(_) => {
                    let event = serde_json::from_str(&line).map_err(|error| error.to_string());
                    if event_tx.send(event).is_err() {
                        return;
                    }
                }
                Err(error) => {
                    let _ = event_tx.send(Err(error.to_string()));
                    return;
                }
            }
        }
    });

    let mut observed = Vec::new();
    for (job, instance) in [(JobId(901), 61), (JobId(902), 62)] {
        send(
            &mut input,
            WorkerCommand::StartImport {
                retain_after_publish: true,
                job_id: job,
                generation: Generation::first(),
                instance_id: instance,
                source_path: source_path.to_string_lossy().into_owned(),
                source_sha256,
                expected_source_samples: 16_000,
                model_path: model_path.to_string_lossy().into_owned(),
                model_sha256,
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            },
        );
        observed.extend(await_end(&event_rx, job, instance, &child));
    }
    send(&mut input, WorkerCommand::Shutdown);
    drop(input);
    assert!(child.wait().expect("import worker deadline").success());
    support::finish_reader(stdout_reader, &child);
    let stderr = String::from_utf8(support::finish_reader(stderr_reader, &child)).unwrap();
    assert!(stderr.contains("startup_perf model_cache=load backend=cpu"));
    assert!(stderr.contains("startup_perf model_cache=hit backend=cpu"));
    observed
}
