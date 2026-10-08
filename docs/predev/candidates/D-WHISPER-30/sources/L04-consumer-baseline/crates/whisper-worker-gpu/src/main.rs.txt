#![forbid(unsafe_code)]

use symphonia as _decoder_available_in_worker;
use whisper_core::ipc::{IpcEnvelope, WorkerEvent};

fn main() {
    // L00 establishes the executable and protocol boundary only. The supervised
    // worker loop, decoder, and inference are delivered by later lots.
    let _protocol_type = std::any::type_name::<IpcEnvelope<WorkerEvent>>();
    let _engine_type = std::any::type_name::<whisper_rs::WhisperContext>();
    let _decoder_type =
        std::any::type_name::<_decoder_available_in_worker::core::formats::FormatOptions>();
}
