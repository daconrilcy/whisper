use std::{
    io::Write,
    process::{Command, Stdio},
};
use whisper_core::{
    ComputeChoice, Generation, JobConfig, JobId, LanguageChoice,
    ipc::{IpcEnvelope, WorkerCommand, WorkerErrorCode, WorkerEvent},
};

#[test]
fn live_worker_rejects_an_unverified_model_before_reporting_ready() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_whisper-worker-cpu"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let command = IpcEnvelope::new(WorkerCommand::StartLiveWorker {
        job_id: JobId(51),
        generation: Generation::first(),
        instance_id: 1,
        model_path: "missing-model.bin".into(),
        model_sha256: [0; 32],
        config: JobConfig {
            language: LanguageChoice::Manual("fr".into()),
            compute: ComputeChoice::Cpu,
        },
    });
    serde_json::to_writer(child.stdin.as_mut().unwrap(), &command).unwrap();
    child.stdin.as_mut().unwrap().write_all(b"\n").unwrap();
    drop(child.stdin.take());

    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let event: IpcEnvelope<WorkerEvent> =
        serde_json::from_slice(output.stdout.split(|byte| *byte == b'\n').next().unwrap()).unwrap();
    assert!(matches!(
        event.message,
        WorkerEvent::Failed {
            job_id: Some(JobId(51)),
            code: WorkerErrorCode::ModelHashMismatch,
            ..
        }
    ));
}
