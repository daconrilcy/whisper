use super::*;

#[test]
fn empty_and_disconnected_channels_do_not_stop_the_session() {
    let (tx, rx) = mpsc::channel();
    let mut out = Vec::new();
    assert!(!take_stop(&rx, JobId(1), Generation::first(), 5, &mut out).unwrap());
    drop(tx);
    assert!(!take_stop(&rx, JobId(1), Generation::first(), 5, &mut out).unwrap());
    assert!(out.is_empty());
}

#[test]
fn wrong_stop_is_ignored_and_matching_stop_is_consumed() {
    let (tx, rx) = mpsc::channel();
    let generation = Generation::first();
    for job_id in [JobId(2), JobId(1)] {
        tx.send(Ok(IpcEnvelope::new(WorkerCommand::Stop {
            job_id,
            generation,
        })))
        .unwrap();
    }
    let mut out = Vec::new();
    assert!(take_stop(&rx, JobId(1), generation, 5, &mut out).unwrap());
    assert!(out.is_empty());
}

#[test]
fn wrong_protocol_emits_correlated_failure_before_stopping() {
    let (tx, rx) = mpsc::channel();
    let mut message = IpcEnvelope::new(WorkerCommand::Shutdown);
    message.version = IPC_PROTOCOL_VERSION + 1;
    tx.send(Ok(message)).unwrap();
    let mut out = Vec::new();
    assert!(take_stop(&rx, JobId(1), Generation::first(), 5, &mut out).unwrap());
    let event: IpcEnvelope<WorkerEvent> = serde_json::from_slice(&out).unwrap();
    assert!(matches!(
        event.message,
        WorkerEvent::Failed {
            job_id: Some(JobId(1)),
            instance_id: Some(5),
            code: WorkerErrorCode::UnsupportedProtocol,
            ..
        }
    ));
}
