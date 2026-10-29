use serde::{Serialize, de::DeserializeOwned};
use std::io::{self, BufRead, Write};

pub const MAX_FRAME_BYTES: usize = 1_048_576;

pub fn read_frame<T: DeserializeOwned>(input: &mut impl BufRead) -> io::Result<Option<T>> {
    let mut bytes = Vec::with_capacity(4096);
    loop {
        let available = input.fill_buf()?;
        if available.is_empty() {
            if bytes.is_empty() {
                return Ok(None);
            }
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "incomplete IPC frame",
            ));
        }
        let upto = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(available.len(), |i| i + 1);
        let new_len = bytes.len().saturating_add(upto);
        if new_len > MAX_FRAME_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "IPC frame exceeds 1 MiB",
            ));
        }
        let finished = available[upto - 1] == b'\n';
        bytes.extend_from_slice(&available[..upto]);
        input.consume(upto);
        if finished {
            break;
        }
    }
    #[cfg(feature = "l01-memory-qualification")]
    crate::memory_qualification::record(
        "ipc.read_frame.buffer",
        serde_json::json!({
            "frame_payload_len_bytes": bytes.len().saturating_sub(1),
            "frame_buffer_capacity_bytes": bytes.capacity(),
            "frame_limit_bytes_including_newline": MAX_FRAME_BYTES,
        }),
    );
    bytes.pop();
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

struct BoundedJsonFrame(Vec<u8>);

impl Write for BoundedJsonFrame {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let limit = MAX_FRAME_BYTES - 1;
        if self.0.len().saturating_add(bytes.len()) > limit {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "IPC frame exceeds 1 MiB",
            ));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub fn write_frame<T: Serialize>(output: &mut impl Write, value: &T) -> io::Result<()> {
    let mut frame = BoundedJsonFrame(Vec::with_capacity(4096));
    serde_json::to_writer(&mut frame, value).map_err(|error| {
        io::Error::new(
            error.io_error_kind().unwrap_or(io::ErrorKind::InvalidData),
            error,
        )
    })?;
    #[cfg(feature = "l01-memory-qualification")]
    crate::memory_qualification::record(
        "ipc.serialize.buffer",
        serde_json::json!({
            "serialized_payload_len_bytes": frame.0.len(),
            "serialized_vec_capacity_bytes": frame.0.capacity(),
            "frame_len_with_newline_bytes": frame.0.len() + 1,
            "frame_limit_bytes": MAX_FRAME_BYTES,
        }),
    );
    output.write_all(&frame.0)?;
    output.write_all(b"\n")?;
    output.flush()
}

pub(crate) type Incoming =
    Result<whisper_core::ipc::IpcEnvelope<whisper_core::ipc::WorkerCommand>, String>;

#[cfg(test)]
mod tests;
