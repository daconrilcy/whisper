use std::io;

const MAX_IPC_FRAME_BYTES: usize = crate::ipc::MAX_FRAME_BYTES;

struct BoundedJsonFrame(Vec<u8>);

impl io::Write for BoundedJsonFrame {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let limit = MAX_IPC_FRAME_BYTES - 1;
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

pub(crate) fn write_frame<T: serde::Serialize>(
    output: &mut impl io::Write,
    value: &T,
) -> io::Result<()> {
    let mut frame = BoundedJsonFrame(Vec::with_capacity(4096));
    serde_json::to_writer(&mut frame, value).map_err(|error| {
        io::Error::new(
            error.io_error_kind().unwrap_or(io::ErrorKind::InvalidData),
            error,
        )
    })?;
    output.write_all(&frame.0)?;
    output.write_all(b"\n")?;
    output.flush()
}

#[cfg(test)]
mod tests;
