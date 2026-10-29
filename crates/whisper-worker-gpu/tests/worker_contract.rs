use sha2::{Digest, Sha256};
use std::{
    io::{BufRead, BufReader, Read, Write},
    path::Path,
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::Duration,
};
use whisper_core::{
    ComputeChoice, Generation, JobConfig, JobId, LanguageChoice, SourceRange,
    ipc::{IPC_PROTOCOL_VERSION, IpcEnvelope, WorkerCommand, WorkerErrorCode, WorkerEvent},
};

#[path = "support/worker.rs"]
mod support;

fn run_command(command: WorkerCommand) -> std::process::Output {
    let mut input = serde_json::to_vec(&IpcEnvelope::new(command)).unwrap();
    input.push(b'\n');
    support::output(
        &mut Command::new(env!("CARGO_BIN_EXE_whisper-worker-gpu")),
        Some(input),
        support::CONTROLLED_TIMEOUT,
    )
}

#[test]
fn turbo_idle_and_refusal_contract() {
    let idle = run_command(WorkerCommand::Shutdown);
    assert!(idle.status.success());
    assert!(
        idle.stdout.is_empty(),
        "idle shutdown must not emit Stopped"
    );

    let job_id = JobId(51);
    let generation = Generation::first();
    let refusal = run_command(WorkerCommand::StartLiveWorker {
        job_id,
        generation,
        instance_id: 1,
        model_path: "missing-model.bin".into(),
        model_sha256: [0; 32],
        config: JobConfig {
            language: LanguageChoice::Manual("fr".into()),
            compute: ComputeChoice::Gpu,
        },
    });
    assert!(refusal.status.success());
    let lines: Vec<_> = refusal
        .stdout
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .collect();
    assert_eq!(lines.len(), 1, "refusal must precede Ready");
    let event: IpcEnvelope<WorkerEvent> = serde_json::from_slice(lines[0]).unwrap();
    assert_eq!(event.version, IPC_PROTOCOL_VERSION);
    assert!(matches!(event.message, WorkerEvent::Failed {
        job_id: Some(actual_job), generation: Some(actual_generation), instance_id: Some(1),
        code: WorkerErrorCode::ModelHashMismatch, ..
    } if actual_job == job_id && actual_generation == generation));
}

#[test]
fn gpu_identity_contract() {
    use whisper_core::gpu_inventory::GpuRuntimeIdentity;
    let job_id = JobId(87);
    let generation = Generation::first();
    let output = run_command(WorkerCommand::StartGpuLiveWorker {
        job_id,
        generation,
        instance_id: 12,
        model_path: "missing-model.bin".into(),
        model_sha256: [0; 32],
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Gpu,
        },
        candidate: GpuRuntimeIdentity {
            generation: 0,
            pnp_instance_id: "invalid-candidate".into(),
            dxgi_rank: u32::MAX,
            luid: 0,
            pci_id: "invalid".into(),
            cuda_ordinal: u32::MAX,
            ggml_ordinal: u32::MAX,
        },
    });
    assert!(output.status.success());
    let lines: Vec<_> = output
        .stdout
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .collect();
    assert_eq!(lines.len(), 1);
    let event: IpcEnvelope<WorkerEvent> = serde_json::from_slice(lines[0]).unwrap();
    assert_eq!(event.version, IPC_PROTOCOL_VERSION);
    assert!(matches!(event.message, WorkerEvent::Failed {
        job_id: Some(actual_job), generation: Some(actual_generation), instance_id: Some(12),
        code: WorkerErrorCode::BackendMismatch, ..
    } if actual_job == job_id && actual_generation == generation));
}

#[test]
#[ignore = "requires the local Turbo model and an explicit campaign work root"]
fn turbo_reusable_import_contract() {
    let reference =
        std::env::var_os("WHISPER_TEST_REFERENCE_EXE").expect("reference executable required");
    let candidate =
        std::env::var_os("WHISPER_TEST_WORKER_EXE").expect("candidate executable required");
    assert_eq!(
        gpu_reuse::run_for_exe(Path::new(&reference)),
        gpu_reuse::run_for_exe(Path::new(&candidate))
    );
}

fn model_hash(path: &Path) -> [u8; 32] {
    let mut file = std::fs::File::open(path).unwrap();
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let n = file.read(&mut buffer).unwrap();
        if n == 0 {
            break;
        }
        digest.update(&buffer[..n]);
    }
    digest.finalize().into()
}

fn send_command(writer: &mut impl Write, command: WorkerCommand) {
    serde_json::to_writer(&mut *writer, &IpcEnvelope::new(command)).unwrap();
    writer.write_all(b"\n").unwrap();
    writer.flush().unwrap();
}

fn live_case(
    model: &Path,
    hash: [u8; 32],
    shutdown: bool,
    executable: &Path,
) -> Vec<IpcEnvelope<WorkerEvent>> {
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
        let mut chunk = [0_u8; 4096];
        loop {
            let n = stderr.read(&mut chunk).unwrap();
            if n == 0 {
                break;
            }
            eprint!("{}", String::from_utf8_lossy(&chunk[..n]));
        }
    });
    let (tx, rx) = mpsc::channel();
    let reader = thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            if let Ok(line) = &line {
                eprintln!("captured worker event: {line}");
            }
            if tx.send(line).is_err() {
                break;
            }
        }
    });
    let job_id = JobId(73);
    let generation = Generation::first();
    send_command(
        &mut input,
        WorkerCommand::StartLiveWorker {
            job_id,
            generation,
            instance_id: 9,
            model_path: model.to_string_lossy().into_owned(),
            model_sha256: hash,
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Gpu,
            },
        },
    );
    let receive = || -> IpcEnvelope<WorkerEvent> {
        let line = rx
            .recv_timeout(child.remaining().min(Duration::from_secs(180)))
            .expect("worker event timeout")
            .unwrap();
        serde_json::from_str(&line).unwrap()
    };
    let ready = receive();
    assert!(
        matches!(ready.message, WorkerEvent::Ready { job_id: actual, instance_id: 9, .. } if actual == job_id)
    );
    let mut events = vec![ready];
    send_command(
        &mut input,
        WorkerCommand::LiveWindow {
            job_id,
            generation,
            instance_id: 9,
            sequence: 1,
            range: SourceRange::new(0, 32_000, 16_000).unwrap(),
            samples: vec![0; 32_000],
        },
    );
    let mut packets = 0;
    loop {
        let event = receive();
        events.push(event.clone());
        match event.message {
            WorkerEvent::LiveMp3Packet { bytes, .. } => {
                assert!(!bytes.is_empty());
                packets += 1;
            }
            WorkerEvent::Segment(_) => {}
            WorkerEvent::WindowFinished { sequence: 1, .. } => break,
            event => panic!("unexpected live event: {event:?}"),
        }
    }
    send_command(
        &mut input,
        if shutdown {
            WorkerCommand::Shutdown
        } else {
            WorkerCommand::Stop { job_id, generation }
        },
    );
    if !shutdown {
        loop {
            let event = receive();
            events.push(event.clone());
            match event.message {
                WorkerEvent::LiveMp3Packet { bytes, .. } => {
                    assert!(!bytes.is_empty());
                    packets += 1;
                }
                WorkerEvent::Stopped {
                    job_id: actual,
                    generation: actual_generation,
                    instance_id: 9,
                } => {
                    assert_eq!((actual, actual_generation), (job_id, generation));
                    break;
                }
                event => panic!("unexpected stop event: {event:?}"),
            }
        }
        assert!(packets > 0, "Stop must flush encoded packets");
    }
    drop(input);
    assert!(child.wait().expect("live worker deadline").success());
    support::finish_reader(reader, &child);
    support::finish_reader(stderr_reader, &child);
    assert!(
        rx.try_iter().all(|line| line.unwrap().is_empty()),
        "unexpected events after end"
    );
    events
}

#[test]
#[ignore = "requires the local Turbo model"]
fn turbo_live_stop_and_shutdown_contract() {
    let model = std::env::var_os("WHISPER_TEST_REUSE_MODEL").expect("set WHISPER_TEST_REUSE_MODEL");
    let path = Path::new(&model);
    let hash = model_hash(path);
    let reference =
        std::env::var_os("WHISPER_TEST_REFERENCE_EXE").expect("set WHISPER_TEST_REFERENCE_EXE");
    let candidate =
        std::env::var_os("WHISPER_TEST_WORKER_EXE").expect("set WHISPER_TEST_WORKER_EXE");
    for shutdown in [false, true] {
        assert_eq!(
            live_case(path, hash, shutdown, Path::new(&reference)),
            live_case(path, hash, shutdown, Path::new(&candidate))
        );
    }
}

mod gpu_reuse {
    use std::{
        fs::{self, File},
        io::{BufRead, BufReader, Read, Write},
        path::{Path, PathBuf},
        process::{Command, Stdio},
        sync::mpsc,
        thread,
        time::{Duration, Instant},
    };

    use super::support;

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
                    assert_eq!(backend, BackendKind::Cuda);
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
                        compute: ComputeChoice::Gpu,
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
        assert!(stderr.contains("startup_perf model_cache=load backend=gpu"));
        assert!(stderr.contains("startup_perf model_cache=hit backend=gpu"));
        observed
    }
}
