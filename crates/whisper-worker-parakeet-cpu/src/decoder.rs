use std::{fs::File, path::Path};

use symphonia::core::{
    audio::{AudioBufferRef, Channels, SampleBuffer},
    codecs::{CODEC_TYPE_MP3, DecoderOptions},
    errors::Error,
    formats::FormatOptions,
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint,
};

pub const OUTPUT_RATE: u32 = 16_000;
pub const MAX_AUDIO_SAMPLES: usize = 28_800_000;

#[derive(Debug)]
pub struct DecodedAudio {
    pub samples: Vec<f32>,
    pub source_frames: u64,
    pub source_rate: u32,
    pub channels: usize,
    pub codec: String,
}

pub fn decode_file(path: &Path) -> Result<DecodedAudio, String> {
    let file = File::open(path).map_err(|e| format!("SourceOpen: {e}"))?;
    let stream = MediaSourceStream::new(Box::new(file), Default::default());
    let probed = symphonia::default::get_probe()
        .format(
            &Hint::new(),
            stream,
            &FormatOptions {
                enable_gapless: true,
                ..Default::default()
            },
            &MetadataOptions::default(),
        )
        .map_err(|e| format!("UnsupportedFormat: {e}"))?;
    let mut format = probed.format;
    let track = format
        .default_track()
        .ok_or_else(|| "UnsupportedFormat: no default audio track".to_owned())?;
    let track_id = track.id;
    let params = track.codec_params.clone();
    let codec = params.codec;
    let source_rate = params
        .sample_rate
        .ok_or_else(|| "UnsupportedFormat: missing sample rate".to_owned())?;
    let channels = params
        .channels
        .ok_or_else(|| "UnsupportedFormat: missing channels".to_owned())?
        .count();
    if source_rate == 0 || channels == 0 || channels > 32 {
        return Err("UnsupportedFormat: unsupported audio layout".into());
    }
    if codec == CODEC_TYPE_MP3 {
        let supported_channels = params.channels == Some(Channels::FRONT_LEFT)
            || params.channels == Some(Channels::FRONT_LEFT | Channels::FRONT_RIGHT);
        if !matches!(
            source_rate,
            8_000 | 11_025 | 12_000 | 16_000 | 22_050 | 24_000 | 32_000 | 44_100 | 48_000
        ) || !supported_channels
            || params.n_frames.is_none()
        {
            return Err("UnsupportedFormat: MP3 profile or duration metadata outside Q-07".into());
        }
    }
    let mut decoder = symphonia::default::get_codecs()
        .make(&params, &DecoderOptions::default())
        .map_err(|e| format!("DecoderInit: {e}"))?;
    let mut output = Vec::new();
    let mut previous = None::<f32>;
    let mut next_output = 0_u64;
    let mut input_index = 0_u64;
    let mut source_frames = 0_u64;
    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(Error::IoError(error)) if error.kind() == std::io::ErrorKind::UnexpectedEof => {
                break;
            }
            Err(error) => return Err(format!("DecodePacket: {error}")),
        };
        if packet.track_id() != track_id {
            continue;
        }
        let decoded = decoder
            .decode(&packet)
            .map_err(|e| format!("DecodeAudio: {e}"))?;
        if codec == CODEC_TYPE_MP3
            && (decoded.spec().rate != source_rate
                || Some(decoded.spec().channels) != params.channels)
        {
            return Err("UnsupportedFormat: MP3 profile changes within the source".into());
        }
        let mut samples = SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
        samples.copy_interleaved_ref(decoded);
        let interleaved = samples.samples();
        let count = interleaved.len() / channels;
        for frame in 0..count {
            let timeline_index = input_index;
            input_index += 1;
            source_frames += 1;
            let current = interleaved[frame * channels..(frame + 1) * channels]
                .iter()
                .copied()
                .sum::<f32>()
                / channels as f32;
            if let Some(prev) = previous {
                while next_output.saturating_mul(u64::from(source_rate))
                    <= timeline_index.saturating_mul(u64::from(OUTPUT_RATE))
                {
                    let numerator = next_output.saturating_mul(u64::from(source_rate));
                    let base = numerator / u64::from(OUTPUT_RATE);
                    let frac = (numerator % u64::from(OUTPUT_RATE)) as f32 / OUTPUT_RATE as f32;
                    let value = if base == timeline_index {
                        current
                    } else {
                        prev + (current - prev) * frac
                    };
                    output.push(value.clamp(-1.0, 1.0));
                    if output.len() > MAX_AUDIO_SAMPLES {
                        return Err("AudioTooLong: source exceeds 1,800 seconds".into());
                    }
                    next_output += 1;
                }
            } else {
                output.push(current.clamp(-1.0, 1.0));
                next_output = 1;
            }
            previous = Some(current);
        }
    }
    if output.is_empty() {
        return Err("DecodeAudio: no samples".into());
    }
    Ok(DecodedAudio {
        samples: output,
        source_frames,
        source_rate,
        channels,
        codec: format!("{:?}", codec),
    })
}

#[allow(dead_code)]
fn _audio_buffer_ref_is_supported(buffer: &AudioBufferRef<'_>) -> usize {
    buffer.frames()
}
