use crate::{engine_cache::CachedGpuEngine, ipc::Incoming, session::run_session};
use std::{io, sync::mpsc::Receiver};
use whisper_core::ipc::{IPC_PROTOCOL_VERSION, WorkerCommand};

pub(crate) fn run(rx: Receiver<Incoming>, output: &mut impl io::Write) -> Result<(), String> {
    let mut cached_engine: Option<CachedGpuEngine> = None;
    loop {
        let first = match rx.recv() {
            Ok(Ok(first)) => first,
            Ok(Err(error)) => return Err(error),
            Err(_) => return Ok(()),
        };
        if first.version == IPC_PROTOCOL_VERSION
            && matches!(&first.message, WorkerCommand::Shutdown)
        {
            return Ok(());
        }
        let reusable = matches!(
            &first.message,
            WorkerCommand::StartDirectLiveWorker { .. }
                | WorkerCommand::StartGpuDirectLiveWorker { .. }
                | WorkerCommand::StartGpuLiveWorker { .. }
                | WorkerCommand::StartImport {
                    retain_after_publish: true,
                    ..
                }
                | WorkerCommand::StartGpuImport {
                    retain_after_publish: true,
                    ..
                }
        );
        run_session(&rx, output, first, &mut cached_engine)?;
        if !reusable {
            return Ok(());
        }
    }
}

#[cfg(test)]
mod tests;
