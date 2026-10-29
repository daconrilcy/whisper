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

#[path = "reusable_imports.rs"]
mod reusable_imports;
#[path = "support/worker.rs"]
mod support;

fn run_command(command: WorkerCommand) -> std::process::Output {
    let mut input = serde_json::to_vec(&IpcEnvelope::new(command)).unwrap();
    input.push(b'\n');
    support::output(
        &mut Command::new(env!("CARGO_BIN_EXE_whisper-worker-cpu")),
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
            compute: ComputeChoice::Cpu,
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
#[ignore = "requires the local Turbo model and an explicit campaign work root"]
fn turbo_reusable_import_contract() {
    let reference =
        std::env::var_os("WHISPER_TEST_REFERENCE_EXE").expect("reference executable required");
    let candidate =
        std::env::var_os("WHISPER_TEST_WORKER_EXE").expect("candidate executable required");
    assert_eq!(
        reusable_imports::run_for_exe(Path::new(&reference)),
        reusable_imports::run_for_exe(Path::new(&candidate))
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
                compute: ComputeChoice::Cpu,
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
