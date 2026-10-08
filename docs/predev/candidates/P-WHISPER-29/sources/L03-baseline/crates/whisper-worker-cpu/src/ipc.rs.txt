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

pub fn write_frame<T: Serialize>(output: &mut impl Write, value: &T) -> io::Result<()> {
    let bytes = serde_json::to_vec(value).map_err(io::Error::other)?;
    #[cfg(feature = "l01-memory-qualification")]
    crate::memory_qualification::record(
        "ipc.serialize.buffer",
        serde_json::json!({
            "serialized_payload_len_bytes": bytes.len(),
            "serialized_vec_capacity_bytes": bytes.capacity(),
            "frame_len_with_newline_bytes": bytes.len() + 1,
            "frame_limit_bytes": MAX_FRAME_BYTES,
        }),
    );
    if bytes.len() + 1 > MAX_FRAME_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "IPC frame exceeds 1 MiB",
        ));
    }
    output.write_all(&bytes)?;
    output.write_all(b"\n")?;
    output.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_oversized_and_incomplete_frames_before_deserialization() {
        let mut oversized = vec![b'a'; MAX_FRAME_BYTES];
        oversized.push(b'\n');
        assert_eq!(
            read_frame::<serde_json::Value>(&mut oversized.as_slice())
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
        let mut incomplete = br#"{"version":2}"#.as_slice();
        assert_eq!(
            read_frame::<serde_json::Value>(&mut incomplete)
                .unwrap_err()
                .kind(),
            io::ErrorKind::UnexpectedEof
        );
    }

    #[test]
    fn rejects_serialized_frames_over_one_mibibyte() {
        let oversized = "x".repeat(MAX_FRAME_BYTES);
        assert_eq!(
            write_frame(&mut Vec::new(), &oversized).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }
}
