use std::path::Path;
use whisper_core::ports::DecodeRequest;

pub use whisper_worker_common::decoder::{MAX_WINDOW_SAMPLES, OUTPUT_RATE};

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

pub fn decode_requested_block(request: &DecodeRequest) -> Result<Option<Vec<i16>>, String> {
    whisper_worker_common::decoder::decode_requested_block(request, observer())
}

pub fn decode_wav_windows(
    path: &Path,
    on_window: impl FnMut(u64, Vec<f32>) -> Result<(), String>,
) -> Result<(u64, u64), String> {
    whisper_worker_common::decoder::decode_wav_windows(path, on_window, observer())
}

#[cfg(test)]
mod tests;
