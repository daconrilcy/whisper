use super::*;
use std::sync::mpsc;
use whisper_core::ipc::IpcEnvelope;

#[test]
fn idle_worker_shutdown_exits_without_an_error_frame() {
    let (tx, rx) = mpsc::channel();
    tx.send(Ok(IpcEnvelope::new(WorkerCommand::Shutdown)))
        .unwrap();
    drop(tx);
    let mut output = Vec::new();
    assert!(run(rx, &mut output).is_ok());
    assert!(output.is_empty());
}
