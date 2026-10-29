use std::io::{self, BufRead, Write};
use whisper_core::parakeet_ipc::{Envelope, Event, PARAKEET_MAX_FRAME_BYTES, WorkerIdentity};

pub(crate) fn read_frame(reader: &mut impl BufRead) -> Result<Option<Vec<u8>>, String> {
    let mut frame = Vec::new();
    loop {
        let available = reader.fill_buf().map_err(|e| format!("IpcRead: {e}"))?;
        if available.is_empty() {
            return if frame.is_empty() {
                Ok(None)
            } else {
                Err("IpcTruncated: EOF inside frame".into())
            };
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let count = newline.map_or(available.len(), |index| index + 1);
        if frame.len() + count > PARAKEET_MAX_FRAME_BYTES {
            return Err("IpcFrameTooLarge: frame exceeds 1 MiB".into());
        }
        frame.extend_from_slice(&available[..count]);
        reader.consume(count);
        if newline.is_some() {
            frame.pop();
            return Ok(Some(frame));
        }
    }
}

pub(crate) fn write_event(event: &Event) -> Result<(), String> {
    let mut bytes =
        serde_json::to_vec(&Envelope::new(event)).map_err(|e| format!("IpcSerialize: {e}"))?;
    if bytes.len() + 1 > PARAKEET_MAX_FRAME_BYTES {
        return Err("IpcFrameTooLarge: event exceeds 1 MiB".into());
    }
    bytes.push(b'\n');
    let mut stdout = io::stdout().lock();
    stdout
        .write_all(&bytes)
        .map_err(|e| format!("IpcWrite: {e}"))?;
    stdout.flush().map_err(|e| format!("IpcWrite: {e}"))
}

pub(crate) fn failed(
    request_id: Option<u64>,
    job_id: Option<&str>,
    generation: Option<u64>,
    attempt: Option<u64>,
    identity: Option<WorkerIdentity>,
    code: &str,
    message: &str,
) -> Event {
    Event::Failed {
        request_id,
        job_id: job_id.map(str::to_owned),
        generation,
        attempt,
        identity,
        code: code.into(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests;
