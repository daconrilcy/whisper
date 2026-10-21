use std::{env, fs, process};
use webrtc_vad::{SampleRate, Vad, VadMode};

fn chunk<'a>(bytes: &'a [u8], name: &[u8; 4]) -> Option<&'a [u8]> {
    if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return None;
    }
    let mut at = 12usize;
    while at + 8 <= bytes.len() {
        let size = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().ok()?) as usize;
        let end = at.checked_add(8)?.checked_add(size)?;
        if end > bytes.len() {
            return None;
        }
        if &bytes[at..at + 4] == name {
            return Some(&bytes[at + 8..end]);
        }
        at = end.checked_add(size & 1)?;
    }
    None
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let mode_text = args.next().ok_or("usage: vad_probe MODE FRAME_MS WAV")?;
    let frame_ms_text = args.next().ok_or("usage: vad_probe MODE FRAME_MS WAV")?;
    let wav = args.next().ok_or("usage: vad_probe MODE FRAME_MS WAV")?;
    if args.next().is_some() {
        return Err("unexpected argument".into());
    }
    let mode = match mode_text.as_str() {
        "0" => VadMode::Quality,
        "1" => VadMode::LowBitrate,
        "2" => VadMode::Aggressive,
        "3" => VadMode::VeryAggressive,
        _ => return Err("mode must be 0, 1, 2, or 3".into()),
    };
    let frame_ms: usize = frame_ms_text.parse().map_err(|_| "invalid frame_ms")?;
    if ![10, 20, 30].contains(&frame_ms) {
        return Err("frame_ms must be 10, 20, or 30".into());
    }
    let bytes = fs::read(&wav).map_err(|e| e.to_string())?;
    let fmt = chunk(&bytes, b"fmt ").ok_or("missing fmt chunk")?;
    if fmt.len() < 16
        || u16::from_le_bytes(fmt[0..2].try_into().unwrap()) != 1
        || u16::from_le_bytes(fmt[2..4].try_into().unwrap()) != 1
        || u32::from_le_bytes(fmt[4..8].try_into().unwrap()) != 16_000
        || u16::from_le_bytes(fmt[14..16].try_into().unwrap()) != 16
    {
        return Err("expected PCM16 mono 16kHz".into());
    }
    let data = chunk(&bytes, b"data").ok_or("missing data chunk")?;
    if data.len() % 2 != 0 {
        return Err("odd PCM byte count".into());
    }
    let mut vad = Vad::new_with_rate_and_mode(SampleRate::Rate16kHz, mode);
    let frame_samples = 16 * frame_ms;
    let frame_bytes = 2 * frame_samples;
    let mut bits = String::with_capacity(data.len().div_ceil(frame_bytes));
    for frame in data.chunks(frame_bytes) {
        let mut pcm = vec![0_i16; frame_samples];
        for (sample, pair) in pcm.iter_mut().zip(frame.chunks_exact(2)) {
            *sample = i16::from_le_bytes([pair[0], pair[1]]);
        }
        bits.push(if vad.is_voice_segment(&pcm).map_err(|_| "VAD rejected frame")? {
            '1'
        } else {
            '0'
        });
    }
    println!("{}\t{}\t{}\t{}\t{}", mode_text, frame_ms, data.len() / 2, bits.len(), bits);
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}
