#[cfg(test)]
use serde::Serialize;
use serde::de::DeserializeOwned;
#[cfg(test)]
use std::io::Write;
use std::io::{self, BufRead};

#[cfg(any(test, feature = "l01-memory-qualification"))]
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

#[cfg(test)]
pub fn write_frame<T: Serialize>(output: &mut impl Write, value: &T) -> io::Result<()> {
    whisper_worker_common::ipc::write_frame(output, value, observer())
}

pub(crate) type Incoming =
    Result<whisper_core::ipc::IpcEnvelope<whisper_core::ipc::WorkerCommand>, String>;

#[cfg(test)]
mod tests;
