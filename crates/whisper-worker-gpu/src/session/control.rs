use super::*;

pub(super) fn take_stop(
    rx: &Receiver<Incoming>,
    job_id: whisper_core::JobId,
    generation: whisper_core::Generation,
    instance_id: u64,
    output: &mut impl io::Write,
) -> Result<bool, String> {
    loop {
        match rx.try_recv() {
            Ok(Ok(envelope)) if envelope.version != IPC_PROTOCOL_VERSION => {
                return emit_failure(
                    output,
                    job_id,
                    generation,
                    instance_id,
                    WorkerErrorCode::UnsupportedProtocol,
                    "ProtocolMismatch",
                )
                .map(|_| true);
            }
            Ok(Ok(IpcEnvelope {
                message:
                    WorkerCommand::Stop {
                        job_id: stop_job,
                        generation: stop_generation,
                    },
                ..
            })) if stop_job == job_id && stop_generation == generation => return Ok(true),
            Ok(Ok(IpcEnvelope {
                message: WorkerCommand::Shutdown,
                ..
            })) => return Ok(true),
            Ok(Ok(_)) => continue,
            Ok(Err(error)) => return Err(format!("IPC input error: {error}")),
            Err(mpsc::TryRecvError::Empty) => return Ok(false),
            Err(mpsc::TryRecvError::Disconnected) => return Ok(false),
        }
    }
}

#[cfg(test)]
mod tests;
