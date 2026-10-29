use serde::{Serialize, de::DeserializeOwned};
use std::io::{self, BufRead, Write};

#[cfg(test)]
pub use whisper_worker_common::ipc::MAX_FRAME_BYTES;

fn observer() -> Option<whisper_worker_common::observation::MemoryObserver> {
    #[cfg(feature = "l01-memory-qualification")]
    {
        Some(crate::memory_qualification::record)
    }
    #[cfg(not(feature = "l01-memory-qualification"))]
    {
        None
    }
}

pub fn read_frame<T: DeserializeOwned>(input: &mut impl BufRead) -> io::Result<Option<T>> {
    whisper_worker_common::ipc::read_frame(input, observer())
}

pub fn write_frame<T: Serialize>(output: &mut impl Write, value: &T) -> io::Result<()> {
    whisper_worker_common::ipc::write_frame(output, value, observer())
}

pub(crate) type Incoming =
    Result<whisper_core::ipc::IpcEnvelope<whisper_core::ipc::WorkerCommand>, String>;

#[cfg(test)]
mod tests;
