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

pub fn decode_requested_block(request: &DecodeRequest) -> Result<Option<Vec<i16>>, String> {
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
    let (_, total_samples) =
        decode_wav_windows(Path::new(&request.source.source_id), |start, window| {
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
        })?;
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
) -> Result<(u64, u64), String> {
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
    crate::memory_qualification::record(
        "codec.ready",
        serde_json::json!({
            "sample_rate_hz": sample_rate,
            "channels": channels,
            "wav_pcm_max_frames_per_packet": 1_152,
            "window_max_samples": MAX_WINDOW_SAMPLES,
        }),
    );
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
            packet_index += 1;
        }
        #[cfg(feature = "l01-memory-qualification")]
        crate::memory_qualification::record(
            "codec.packet.read",
            serde_json::json!({
                "packet_index": packet_index,
                "packet_payload_bytes": packet.data.len(),
                "packet_duration_timebase_units": packet.dur,
            }),
        );
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
        let decoded_frame_capacity = decoded.capacity();
        #[cfg(feature = "l01-memory-qualification")]
        let decoded_frame_count = decoded.frames();
        let mut samples = SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
        samples.copy_interleaved_ref(decoded);
        let interleaved = samples.samples();
        #[cfg(feature = "l01-memory-qualification")]
        crate::memory_qualification::record(
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
                            window_index += 1;
                        }
                        #[cfg(feature = "l01-memory-qualification")]
                        crate::memory_qualification::record(
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
                        on_window(start, full_window)?;
                        #[cfg(feature = "l01-memory-qualification")]
                        crate::memory_qualification::record(
                            "pcm.window.callback.complete",
                            serde_json::json!({
                                "window_index": window_index,
                                "replacement_window_len": window.len(),
                                "replacement_window_capacity": window.capacity(),
                            }),
                        );
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
            window_index += 1;
        }
        #[cfg(feature = "l01-memory-qualification")]
        let tail_samples = window.len();
        #[cfg(feature = "l01-memory-qualification")]
        let tail_capacity = window.capacity();
        #[cfg(feature = "l01-memory-qualification")]
        crate::memory_qualification::record(
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
        on_window(start, window)?;
        #[cfg(feature = "l01-memory-qualification")]
        crate::memory_qualification::record(
            "pcm.window.callback.complete",
            serde_json::json!({
                "window_index": window_index,
                "released_window_samples": tail_samples,
                "released_window_capacity": tail_capacity,
                "replacement_window_capacity": 0,
            }),
        );
    }
    #[cfg(feature = "l01-memory-qualification")]
    crate::memory_qualification::record(
        "codec.complete",
        serde_json::json!({
            "packet_count": packet_index,
            "window_count": window_index,
            "decoded_frames": decoded_frames,
            "output_samples": output_samples,
            "retained_full_source_pcm_samples": 0,
        }),
    );
    Ok((decoded_frames, output_samples))
}

#[allow(dead_code)]
fn _audio_buffer_ref_is_supported(buffer: &AudioBufferRef<'_>) -> usize {
    buffer.frames()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs::File, io::Write, path::PathBuf, time::SystemTime};
    use whisper_core::{
        Generation, JobId, SourceRange,
        ports::{DecodeRequest, SourceIdentity},
    };

    #[test]
    fn long_source_is_streamed_as_bounded_contiguous_windows() {
        const SOURCE_SAMPLES: u32 = 2_000_123;
        let path = std::env::temp_dir().join(format!(
            "whisper-r7-stream-{}-{}.wav",
            std::process::id(),
            SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        write_silence_wav(&path, SOURCE_SAMPLES);

        let mut observed_windows = Vec::new();
        let mut next_expected_start = 0_u64;
        let (decoded_frames, total_samples) = decode_wav_windows(&path, |start, window| {
            assert!(!window.is_empty());
            assert!(window.len() <= MAX_WINDOW_SAMPLES);
            assert_eq!(
                start, next_expected_start,
                "no gap or overlap between windows"
            );
            next_expected_start += window.len() as u64;
            observed_windows.push((start, window.len()));
            Ok(())
        })
        .unwrap();
        assert_eq!(decoded_frames, u64::from(SOURCE_SAMPLES));
        assert_eq!(total_samples, u64::from(SOURCE_SAMPLES));
        assert_eq!(next_expected_start, u64::from(SOURCE_SAMPLES));
        let mut expected_windows = (0..25)
            .map(|index| (index * 80_000, 80_000))
            .collect::<Vec<_>>();
        expected_windows.push((2_000_000, 123));
        assert_eq!(observed_windows, expected_windows);

        let bounded = decode_requested_block(&DecodeRequest {
            source: SourceIdentity {
                source_id: path.to_string_lossy().into_owned(),
                sha256: [0; 32],
            },
            job_id: JobId(904),
            generation: Generation::first(),
            range: SourceRange::new(40_000, 160_000, OUTPUT_RATE).unwrap(),
            max_samples: MAX_WINDOW_SAMPLES,
        })
        .unwrap()
        .unwrap();
        assert_eq!(bounded.len(), MAX_WINDOW_SAMPLES);
        drop(bounded);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn lame_gapless_delay_and_padding_are_trimmed_once() {
        let Some(path) = std::env::var_os("WHISPER_L02_GAPLESS_MP3").map(PathBuf::from) else {
            eprintln!("V-IMPORT-MP3 decoder proof NOT RUN: set WHISPER_L02_GAPLESS_MP3");
            return;
        };
        let file = File::open(&path).unwrap();
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
            .unwrap();
        let (rate, frame_count) = {
            let track = probed.format.default_track().unwrap();
            let params = &track.codec_params;
            assert_eq!(params.codec, CODEC_TYPE_MP3);
            assert!(params.delay.unwrap_or_default() > 0);
            assert!(params.padding.unwrap_or_default() > 0);
            (
                u64::from(params.sample_rate.unwrap()),
                params.n_frames.unwrap(),
            )
        };
        let expected_output = frame_count
            .saturating_sub(1)
            .saturating_mul(u64::from(OUTPUT_RATE))
            / rate
            + 1;
        drop(probed);

        let (decoded_frames, output_samples) = decode_wav_windows(&path, |_, window| {
            assert!(!window.is_empty());
            Ok(())
        })
        .unwrap();
        assert_eq!(
            decoded_frames, frame_count,
            "MP3 delay/padding trimmed once"
        );
        assert_eq!(
            output_samples, expected_output,
            "source timeline duration retained"
        );
    }

    fn write_silence_wav(path: &Path, samples: u32) {
        let data_bytes = samples * 2;
        let mut file = File::create(path).unwrap();
        file.write_all(b"RIFF").unwrap();
        file.write_all(&(36 + data_bytes).to_le_bytes()).unwrap();
        file.write_all(b"WAVEfmt ").unwrap();
        file.write_all(&16_u32.to_le_bytes()).unwrap();
        file.write_all(&1_u16.to_le_bytes()).unwrap();
        file.write_all(&1_u16.to_le_bytes()).unwrap();
        file.write_all(&OUTPUT_RATE.to_le_bytes()).unwrap();
        file.write_all(&(OUTPUT_RATE * 2).to_le_bytes()).unwrap();
        file.write_all(&2_u16.to_le_bytes()).unwrap();
        file.write_all(&16_u16.to_le_bytes()).unwrap();
        file.write_all(b"data").unwrap();
        file.write_all(&data_bytes.to_le_bytes()).unwrap();
        let silence = [0_u8; 8192];
        let mut remaining = data_bytes as usize;
        while remaining > 0 {
            let count = remaining.min(silence.len());
            file.write_all(&silence[..count]).unwrap();
            remaining -= count;
        }
        file.sync_all().unwrap();
    }
}
