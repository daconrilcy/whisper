#[path = "../../../whisper-worker-common/tests/support/native_boundary.rs"]
mod native_boundary;

#[path = "../../src/decoder.rs"]
mod decoder;
#[path = "../../src/diagnostics.rs"]
mod diagnostics;
#[path = "../../src/encoder.rs"]
mod encoder;
#[path = "../../src/ipc.rs"]
mod ipc;
#[path = "../../src/ipc_output.rs"]
mod ipc_output;
#[path = "../../src/worker.rs"]
mod worker;

mod native_engine {
    pub(crate) struct CpuEngine;
}

mod session {
    use crate::{diagnostics::emit_failure, ipc::Incoming};
    use std::{
        io,
        sync::mpsc::{self, Receiver},
    };
    use whisper_core::ipc::{
        IPC_PROTOCOL_VERSION, IpcEnvelope, WorkerCommand, WorkerErrorCode, WorkerEvent,
    };
    use whisper_core::{Generation, JobId};

    #[path = "../../../src/session/control.rs"]
    mod control;

    pub(crate) fn run_session(
        _rx: &std::sync::mpsc::Receiver<crate::ipc::Incoming>,
        _output: &mut impl std::io::Write,
        _first: whisper_core::ipc::IpcEnvelope<whisper_core::ipc::WorkerCommand>,
        _cached_engine: &mut Option<(String, [u8; 32], crate::native_engine::CpuEngine)>,
    ) -> Result<(), String> {
        crate::native_boundary::forbid("session::run_session")
    }
}
