#[cfg(feature = "l01-memory-qualification")]
use super::super::support::memory_trace;
use serde::{Serialize, de::DeserializeOwned};
use std::io::{self, BufRead, Write};

pub(in crate::worker_ipc) const MAX_FRAME_BYTES: usize = 1_048_576;

pub(in crate::worker_ipc) fn read_frame<T: DeserializeOwned>(
    input: &mut impl BufRead,
) -> io::Result<Option<T>> {
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
        if bytes.len().saturating_add(upto) > MAX_FRAME_BYTES {
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
    memory_trace(
        "adapter.ipc.read_frame.buffer",
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
pub(in crate::worker_ipc) fn write_frame<T: Serialize>(
    output: &mut impl Write,
    value: &T,
) -> io::Result<()> {
    let bytes = serialize_frame(value)?;
    #[cfg(feature = "l01-memory-qualification")]
    memory_trace(
        "adapter.ipc.serialize.buffer",
        serde_json::json!({
            "serialized_payload_len_bytes": bytes.len(),
            "serialized_vec_capacity_bytes": bytes.capacity(),
            "frame_len_with_newline_bytes": bytes.len() + 1,
            "frame_limit_bytes": MAX_FRAME_BYTES,
        }),
    );
    output.write_all(&bytes)?;
    output.write_all(b"\n")?;
    output.flush()
}

struct BoundedFrameWriter {
    bytes: Vec<u8>,
}

impl Write for BoundedFrameWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let limit = MAX_FRAME_BYTES - 1;
        if self.bytes.len().saturating_add(bytes.len()) > limit {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "IPC frame exceeds 1 MiB",
            ));
        }
        self.bytes
            .try_reserve_exact(bytes.len())
            .map_err(io::Error::other)?;
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(in crate::worker_ipc) fn serialize_frame<T: Serialize>(value: &T) -> io::Result<Vec<u8>> {
    let mut writer = BoundedFrameWriter {
        bytes: Vec::with_capacity(4_096),
    };
    serde_json::to_writer(&mut writer, value)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    Ok(writer.bytes)
}
