use std::io;

#[cfg(test)]
const MAX_IPC_FRAME_BYTES: usize = crate::ipc::MAX_FRAME_BYTES;

pub(crate) fn write_frame<T: serde::Serialize>(
    output: &mut impl io::Write,
    value: &T,
) -> io::Result<()> {
    whisper_worker_common::ipc::write_frame(output, value, None)
}

#[cfg(test)]
mod tests;
