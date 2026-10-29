use crate::decoder;
use std::{fs, path::Path};

#[cfg(feature = "parakeet-qualification-reference")]
pub(crate) fn decode_reference_wav(path: &Path) -> Result<decoder::DecodedAudio, String> {
    let bytes = fs::read(path).map_err(|error| format!("SourceOpen: {error}"))?;
    if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("ReferenceFormat: expected RIFF/WAVE".into());
    }
    let mut format = None;
    let mut data = None;
    let mut offset = 12_usize;
    while offset + 8 <= bytes.len() {
        let id = &bytes[offset..offset + 4];
        let size = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        let start = offset + 8;
        let end = start
            .checked_add(size)
            .filter(|end| *end <= bytes.len())
            .ok_or("ReferenceFormat: chunk exceeds WAV bounds")?;
        if id == b"fmt " {
            if size < 16 {
                return Err("ReferenceFormat: truncated format chunk".into());
            }
            format = Some((
                u16::from_le_bytes(bytes[start..start + 2].try_into().unwrap()),
                u16::from_le_bytes(bytes[start + 2..start + 4].try_into().unwrap()),
                u32::from_le_bytes(bytes[start + 4..start + 8].try_into().unwrap()),
                u16::from_le_bytes(bytes[start + 12..start + 14].try_into().unwrap()),
                u16::from_le_bytes(bytes[start + 14..start + 16].try_into().unwrap()),
            ));
        } else if id == b"data" {
            data = Some(&bytes[start..end]);
        }
        offset = end + (size & 1);
    }
    let Some((encoding, channels, rate, block_align, bits)) = format else {
        return Err("ReferenceFormat: missing format chunk".into());
    };
    if (encoding, channels, rate, block_align, bits) != (1, 1, 16_000, 2, 16) {
        return Err("ReferenceFormat: expected mono 16 kHz PCM16".into());
    }
    let data = data.ok_or("ReferenceFormat: missing data chunk")?;
    if data.len() % 2 != 0 {
        return Err("ReferenceFormat: odd PCM16 payload size".into());
    }
    let samples = data
        .chunks_exact(2)
        .map(|sample| i16::from_le_bytes([sample[0], sample[1]]) as f32 / 32_768.0)
        .collect::<Vec<_>>();
    let count = samples.len();
    if count == 0 || count as u64 > whisper_core::parakeet_windows::MAX_REFERENCE_SAMPLES {
        return Err("AudioTooLong: reference input must be between 0 and 3,600 seconds".into());
    }
    Ok(decoder::DecodedAudio {
        samples,
        source_frames: count as u64,
        source_rate: 16_000,
        channels: 1,
        codec: "PCM_S16LE".into(),
    })
}

#[cfg(test)]
mod tests;
