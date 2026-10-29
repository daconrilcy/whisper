#[cfg(feature = "l01-memory-qualification")]
use crate::ipc;
#[cfg(feature = "l01-memory-qualification")]
use crate::memory_qualification;
use crate::{
    ipc::{Incoming, read_frame},
    ipc_output::write_frame,
    worker::run as run_worker,
};
use std::{
    io::{self, BufReader, BufWriter},
    sync::mpsc::{self, SyncSender},
    thread,
};
use whisper_core::ipc::{IpcEnvelope, WorkerCommand, WorkerErrorCode, WorkerEvent};

pub(crate) fn run() {
    eprintln!("{}", whisper_rs::print_system_info());
    let (tx, rx) = mpsc::sync_channel(8);
    thread::spawn(move || input_loop(tx));
    let stdout = io::stdout();
    let mut output = BufWriter::new(stdout.lock());
    #[cfg(feature = "l01-memory-qualification")]
    memory_qualification::record(
        "worker.started",
        serde_json::json!({
            "input_event_channel_capacity": 8,
            "stdout_bufwriter_capacity_bytes": output.capacity(),
            "ipc_frame_limit_bytes": ipc::MAX_FRAME_BYTES,
        }),
    );
    if let Err(message) = run_worker(rx, &mut output) {
        let event = IpcEnvelope::new(WorkerEvent::Failed {
            job_id: None,
            generation: None,
            instance_id: None,
            code: WorkerErrorCode::Internal,
            message,
        });
        let _ = write_frame(&mut output, &event);
        std::process::exit(2);
    }
}

fn input_loop(tx: SyncSender<Incoming>) {
    let stdin = io::stdin();
    let mut input = BufReader::new(stdin.lock());
    #[cfg(feature = "l01-memory-qualification")]
    {
        #[cfg(windows)]
        use std::os::windows::io::AsRawHandle;
        let details = {
            #[cfg(windows)]
            {
                serde_json::json!({
                    "stdin_bufreader_capacity_bytes": input.capacity(),
                    "worker_pid": std::process::id(),
                    "stdin_read_pipe_handle": input.get_ref().as_raw_handle() as usize,
                    "kernel_pipe_capacity": "query through worker read handle in Windows sampler",
                })
            }
            #[cfg(not(windows))]
            {
                serde_json::json!({
                    "stdin_bufreader_capacity_bytes": input.capacity(),
                    "kernel_pipe_capacity": "Windows-only qualification",
                })
            }
        };
        memory_qualification::record("worker.input.reader.created", details);
    }
    loop {
        match read_frame::<IpcEnvelope<WorkerCommand>>(&mut input) {
            Ok(Some(envelope)) => {
                if tx.send(Ok(envelope)).is_err() {
                    break;
                }
            }
            Ok(None) => break,
            Err(error) => {
                let _ = tx.send(Err(error.to_string()));
                break;
            }
        }
    }
}
