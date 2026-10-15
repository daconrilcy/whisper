use std::{
    io::Write,
    process::{Command, Stdio},
};
use whisper_core::ipc::{IpcEnvelope, WorkerCommand, WorkerEvent};

#[test]
fn linked_gpu_worker_answers_a_native_cuda_probe_without_loading_a_model() {
    let executable = env!("CARGO_BIN_EXE_whisper-worker-gpu");
    let mut child = Command::new(executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let request = serde_json::to_vec(&IpcEnvelope::new(WorkerCommand::ProbeCuda)).unwrap();
    let stdin = child.stdin.as_mut().unwrap();
    stdin.write_all(&request).unwrap();
    stdin.write_all(b"\n").unwrap();
    drop(child.stdin.take());
    let output = child.wait_with_output().unwrap();
    let first_line = output.stdout.split(|byte| *byte == b'\n').next().unwrap();
    let response: IpcEnvelope<WorkerEvent> = serde_json::from_slice(first_line).unwrap();
    match response.message {
        WorkerEvent::CudaAvailable => assert!(output.status.success()),
        WorkerEvent::Failed { message, .. } => {
            assert!(message.contains("CUDA") || message.contains("GPU"));
        }
        other => panic!("unexpected native probe response: {other:?}"),
    }
}
