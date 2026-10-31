#[test]
fn worker_command_poll_prioritizes_control_and_distinguishes_idle_and_closed() {
    let (commands_tx, commands_rx) = mpsc::sync_channel(2);
    let (emergency_tx, emergency_rx) = mpsc::sync_channel(2);
    commands_tx.send(WorkerCommand::Shutdown).unwrap();
    emergency_tx
        .send(WorkerCommand::Stop {
            job_id: JobId(77),
            generation: Generation::first(),
        })
        .unwrap();
    let WorkerCommandPoll::Command(command) =
        take_next_worker_command(&commands_rx, &emergency_rx)
    else {
        panic!("expected emergency stop");
    };
    assert!(matches!(
        *command,
        WorkerCommand::Stop {
            job_id: JobId(77),
            ..
        }
    ));
    let WorkerCommandPoll::Command(command) =
        take_next_worker_command(&commands_rx, &emergency_rx)
    else {
        panic!("expected queued shutdown");
    };
    assert!(matches!(*command, WorkerCommand::Shutdown));
    assert!(matches!(
        take_next_worker_command(&commands_rx, &emergency_rx),
        WorkerCommandPoll::Idle
    ));
    drop(commands_tx);
    drop(emergency_tx);
    assert!(matches!(
        take_next_worker_command(&commands_rx, &emergency_rx),
        WorkerCommandPoll::Closed
    ));
}
