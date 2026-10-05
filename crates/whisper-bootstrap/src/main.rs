#![forbid(unsafe_code)]

use whisper_core::{AppView, ipc::IPC_PROTOCOL_VERSION};

fn main() {
    // Installation, network acquisition, and readiness checks belong to L06.
    let _initial_view = AppView::default();
    let _ipc_version = IPC_PROTOCOL_VERSION;
    let _transport_type = std::any::type_name::<&dyn whisper_adapters::WorkerTransport>();
}
