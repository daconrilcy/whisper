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
use whisper_core::ports::DecodeRequest;

pub const OUTPUT_RATE: u32 = 16_000;
pub const MAX_WINDOW_SAMPLES: usize = 80_000;

pub fn decode_requested_block(
    request: &DecodeRequest,
    observer: Option<crate::observation::MemoryObserver>,
) -> Result<Option<Vec<i16>>, String> {
    let requested_range = request
        .range
        .end_sample
        .saturating_sub(request.range.start_sample);
    if request.max_samples == 0
        || request.max_samples > MAX_WINDOW_SAMPLES
        || request.range.sample_rate_hz != OUTPUT_RATE
        || request.range.start_sample >= request.range.end_sample
    {
        return Err("InvalidRequest: decode range exceeds the 80,000-sample bound".into());
    }
    let requested = requested_range.min(request.max_samples as u64);
    let requested_end = request.range.start_sample + requested;
    let capacity = usize::try_from(requested).map_err(|_| "InvalidRequest: range too large")?;
    let mut samples = Vec::with_capacity(capacity);
    let (_, total_samples) = decode_wav_windows(
        Path::new(&request.source.source_id),
        |start, window| {
            let window_end = start + window.len() as u64;
            let from = request.range.start_sample.max(start);
            let to = requested_end.min(window_end);
            if from < to {
                samples.extend(
                    window[(from - start) as usize..(to - start) as usize]
                        .iter()
                        .map(|sample| (sample.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16),
                );
            }
            Ok(())
        },
        observer,
    )?;
    if request.range.start_sample >= total_samples {
        return Ok(None);
    }
    let actual_samples = requested_end.min(total_samples) - request.range.start_sample;
    if samples.len() as u64 != actual_samples {
        return Err(
            "DecodeFailed: decoded block length differs from the bounded source range".into(),
        );
    }
    Ok(Some(samples))
}

pub fn decode_wav_windows(
    path: &Path,
    mut on_window: impl FnMut(u64, Vec<f32>) -> Result<(), String>,
    observer: Option<crate::observation::MemoryObserver>,
) -> Result<(u64, u64), String> {
    #[cfg(not(feature = "l01-memory-qualification"))]
    let _ = observer;
    let file = File::open(path).map_err(|e| format!("source open failed: {e}"))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    // Probe the file contents instead of trusting its extension. Q-07 admits WAV
    // and MP3 profiles and explicitly rejects extension-only classification.
    let hint = Hint::new();
    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions {
                enable_gapless: true,
                ..Default::default()
            },
            &MetadataOptions::default(),
        )
        .map_err(|e| format!("UnsupportedFormat: audio probe failed: {e}"))?;
    let mut format = probed.format;
    let track = format
        .default_track()
        .ok_or_else(|| "WAV has no default audio track".to_owned())?;
    let track_id = track.id;
    let codec = track.codec_params.codec;
    let sample_rate = track
        .codec_params
        .sample_rate
        .ok_or_else(|| "WAV sample rate missing".to_owned())?;
    let channels = track
        .codec_params
        .channels
        .ok_or_else(|| "WAV channel count missing".to_owned())?
        .count();
    if sample_rate == 0 || channels == 0 || channels > 32 {
        return Err("UnsupportedFormat: unsupported WAV audio layout".into());
    }
    let channel_mask = track.codec_params.channels;
    let stereo = Channels::FRONT_LEFT | Channels::FRONT_RIGHT;
    let supported_mp3_layout =
        channel_mask == Some(Channels::FRONT_LEFT) || channel_mask == Some(stereo);
    if track.codec_params.codec == CODEC_TYPE_MP3
        && (!matches!(
            sample_rate,
            8_000 | 11_025 | 12_000 | 16_000 | 22_050 | 24_000 | 32_000 | 44_100 | 48_000
        ) || !supported_mp3_layout
            || track.codec_params.n_frames.is_none())
    {
        return Err("UnsupportedFormat: MP3 profile or duration metadata is outside Q-07".into());
    }
    #[cfg(feature = "l01-memory-qualification")]
    if let Some(observer) = observer {
        observer(
            "codec.ready",
            serde_json::json!({
                "sample_rate_hz": sample_rate,
                "channels": channels,
                "wav_pcm_max_frames_per_packet": 1_152,
                "window_max_samples": MAX_WINDOW_SAMPLES,
            }),
        );
    }
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| format!("audio decoder init failed: {e}"))?;
    let mut window = Vec::with_capacity(MAX_WINDOW_SAMPLES);
    let mut decoded_frames = 0_u64;
    let mut output_samples = 0_u64;
    let mut previous = None::<f32>;
    let mut next_output = 0_u64;
    let mut input_index = 0_u64;
    #[cfg(feature = "l01-memory-qualification")]
    let mut packet_index = 0_u64;
    #[cfg(feature = "l01-memory-qualification")]
    let mut window_index = 0_u64;
    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(format!("audio packet read failed: {e}")),
        };
        #[cfg(feature = "l01-memory-qualification")]
        {
            if observer.is_some() {
                packet_index += 1;
            }
        }
        #[cfg(feature = "l01-memory-qualification")]
        if let Some(observer) = observer {
            observer(
                "codec.packet.read",
                serde_json::json!({
                    "packet_index": packet_index,
                    "packet_payload_bytes": packet.data.len(),
                    "packet_duration_timebase_units": packet.dur,
                }),
            );
        }
        if packet.track_id() != track_id {
            continue;
        }
        let decoded = decoder
            .decode(&packet)
            .map_err(|e| format!("audio decode failed: {e}"))?;
        if codec == CODEC_TYPE_MP3
            && (decoded.spec().rate != sample_rate || Some(decoded.spec().channels) != channel_mask)
        {
            return Err("UnsupportedFormat: MP3 profile changes within the source".into());
        }
        #[cfg(feature = "l01-memory-qualification")]
        let decoded_frame_capacity = if observer.is_some() {
            decoded.capacity()
        } else {
            0
        };
        #[cfg(feature = "l01-memory-qualification")]
        let decoded_frame_count = if observer.is_some() {
            decoded.frames()
        } else {
            0
        };
        let mut samples = SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
        samples.copy_interleaved_ref(decoded);
        let interleaved = samples.samples();
        #[cfg(feature = "l01-memory-qualification")]
        if let Some(observer) = observer {
            observer(
                "codec.sample_buffer.live",
                serde_json::json!({
                    "packet_index": packet_index,
                    "packet_payload_bytes": packet.data.len(),
                    "decoded_frame_capacity": decoded_frame_capacity,
                    "decoded_frames": decoded_frame_count,
                    "sample_buffer_initialized_samples": samples.len(),
                    "sample_buffer_capacity_samples": samples.capacity(),
                    "sample_buffer_capacity_bytes": samples.capacity() * std::mem::size_of::<f32>(),
                    "channel_count": channels,
                }),
            );
        }
        let count = interleaved.len() / channels;
        for frame in 0..count {
            let timeline_index = input_index;
            input_index += 1;
            decoded_frames += 1;
            let current = interleaved[frame * channels..(frame + 1) * channels]
                .iter()
                .copied()
                .sum::<f32>()
                / channels as f32;
            if let Some(prev) = previous {
                while next_output.saturating_mul(u64::from(sample_rate))
                    <= timeline_index.saturating_mul(u64::from(OUTPUT_RATE))
                {
                    let numerator = next_output.saturating_mul(u64::from(sample_rate));
                    let base = numerator / u64::from(OUTPUT_RATE);
                    let frac = (numerator % u64::from(OUTPUT_RATE)) as f32 / OUTPUT_RATE as f32;
                    let value = if base == timeline_index {
                        current
                    } else {
                        prev + (current - prev) * frac
                    };
                    window.push(value.clamp(-1.0, 1.0));
                    output_samples += 1;
                    next_output += 1;
                    if window.len() == MAX_WINDOW_SAMPLES {
                        let start = output_samples - window.len() as u64;
                        let full_window =
                            std::mem::replace(&mut window, Vec::with_capacity(MAX_WINDOW_SAMPLES));
                        #[cfg(feature = "l01-memory-qualification")]
                        {
                            if observer.is_some() {
                                window_index += 1;
                            }
                        }
                        #[cfg(feature = "l01-memory-qualification")]
                        if let Some(observer) = observer {
                            observer(
                                "pcm.window.callback.begin",
                                serde_json::json!({
                                    "window_index": window_index,
                                    "source_start_sample": start,
                                    "source_sample_count_total": output_samples,
                                    "outgoing_window_len": full_window.len(),
                                    "outgoing_window_capacity": full_window.capacity(),
                                    "replacement_window_len": window.len(),
                                    "replacement_window_capacity": window.capacity(),
                                    "simultaneous_pcm_window_capacity_bytes": (full_window.capacity() + window.capacity()) * std::mem::size_of::<f32>(),
                                    "codec_packet_payload_bytes_still_live": packet.data.len(),
                                    "codec_sample_buffer_capacity_bytes_still_live": samples.capacity() * std::mem::size_of::<f32>(),
                                }),
                            );
                        }
                        on_window(start, full_window)?;
                        #[cfg(feature = "l01-memory-qualification")]
                        if let Some(observer) = observer {
                            observer(
                                "pcm.window.callback.complete",
                                serde_json::json!({
                                    "window_index": window_index,
                                    "replacement_window_len": window.len(),
                                    "replacement_window_capacity": window.capacity(),
                                }),
                            );
                        }
                    }
                }
            } else {
                window.push(current.clamp(-1.0, 1.0));
                output_samples = 1;
                next_output = 1;
            }
            previous = Some(current);
        }
    }
    if !window.is_empty() {
        let start = output_samples - window.len() as u64;
        #[cfg(feature = "l01-memory-qualification")]
        {
            if observer.is_some() {
                window_index += 1;
            }
        }
        #[cfg(feature = "l01-memory-qualification")]
        let tail_samples = if observer.is_some() { window.len() } else { 0 };
        #[cfg(feature = "l01-memory-qualification")]
        let tail_capacity = if observer.is_some() {
            window.capacity()
        } else {
            0
        };
        #[cfg(feature = "l01-memory-qualification")]
        if let Some(observer) = observer {
            observer(
                "pcm.window.callback.begin",
                serde_json::json!({
                    "window_index": window_index,
                    "source_start_sample": start,
                    "source_sample_count_total": output_samples,
                    "outgoing_window_len": window.len(),
                    "outgoing_window_capacity": window.capacity(),
                    "replacement_window_capacity": 0,
                    "simultaneous_pcm_window_capacity_bytes": window.capacity() * std::mem::size_of::<f32>(),
                }),
            );
        }
        on_window(start, window)?;
        #[cfg(feature = "l01-memory-qualification")]
        if let Some(observer) = observer {
            observer(
                "pcm.window.callback.complete",
                serde_json::json!({
                    "window_index": window_index,
                    "released_window_samples": tail_samples,
                    "released_window_capacity": tail_capacity,
                    "replacement_window_capacity": 0,
                }),
            );
        }
    }
    #[cfg(feature = "l01-memory-qualification")]
    if let Some(observer) = observer {
        observer(
            "codec.complete",
            serde_json::json!({
                "packet_count": packet_index,
                "window_count": window_index,
                "decoded_frames": decoded_frames,
                "output_samples": output_samples,
                "retained_full_source_pcm_samples": 0,
            }),
        );
    }
    Ok((decoded_frames, output_samples))
}

#[allow(dead_code)]
fn _audio_buffer_ref_is_supported(buffer: &AudioBufferRef<'_>) -> usize {
    buffer.frames()
}

#[cfg(test)]
mod tests;
