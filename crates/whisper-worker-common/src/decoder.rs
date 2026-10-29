use std::{fs::File, path::Path};
use symphonia::core::{
    audio::{AudioBufferRef, Channels, SampleBuffer},
    codecs::{CODEC_TYPE_MP3, CodecType, Decoder, DecoderOptions},
    errors::Error,
    formats::{FormatOptions, FormatReader, Packet},
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
    on_window: impl FnMut(u64, Vec<f32>) -> Result<(), String>,
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
    let mut state = DecodeState::new(
        sample_rate,
        channels,
        codec,
        channel_mask,
        observer,
        on_window,
    );
    decode_packets(&mut *format, &mut *decoder, track_id, &mut state)?;
    state.finish()
}

struct DecodeState<F> {
    sample_rate: u32,
    channels: usize,
    codec: CodecType,
    channel_mask: Option<Channels>,
    #[cfg(feature = "l01-memory-qualification")]
    observer: Option<crate::observation::MemoryObserver>,
    on_window: F,
    window: Vec<f32>,
    decoded_frames: u64,
    output_samples: u64,
    previous: Option<f32>,
    next_output: u64,
    input_index: u64,
    #[cfg(feature = "l01-memory-qualification")]
    packet_index: u64,
    #[cfg(feature = "l01-memory-qualification")]
    window_index: u64,
}

impl<F: FnMut(u64, Vec<f32>) -> Result<(), String>> DecodeState<F> {
    fn new(
        sample_rate: u32,
        channels: usize,
        codec: CodecType,
        channel_mask: Option<Channels>,
        observer: Option<crate::observation::MemoryObserver>,
        on_window: F,
    ) -> Self {
        #[cfg(not(feature = "l01-memory-qualification"))]
        let _ = observer;
        Self {
            sample_rate,
            channels,
            codec,
            channel_mask,
            #[cfg(feature = "l01-memory-qualification")]
            observer,
            on_window,
            window: Vec::with_capacity(MAX_WINDOW_SAMPLES),
            decoded_frames: 0,
            output_samples: 0,
            previous: None,
            next_output: 0,
            input_index: 0,
            #[cfg(feature = "l01-memory-qualification")]
            packet_index: 0,
            #[cfg(feature = "l01-memory-qualification")]
            window_index: 0,
        }
    }

    fn process_packet(&mut self, decoder: &mut dyn Decoder, packet: &Packet) -> Result<(), String> {
        let decoded = decoder
            .decode(packet)
            .map_err(|e| format!("audio decode failed: {e}"))?;
        if self.codec == CODEC_TYPE_MP3
            && (decoded.spec().rate != self.sample_rate
                || Some(decoded.spec().channels) != self.channel_mask)
        {
            return Err("UnsupportedFormat: MP3 profile changes within the source".into());
        }
        #[cfg(feature = "l01-memory-qualification")]
        let decoded_frame_capacity = if self.observer.is_some() {
            decoded.capacity()
        } else {
            0
        };
        #[cfg(feature = "l01-memory-qualification")]
        let decoded_frame_count = if self.observer.is_some() {
            decoded.frames()
        } else {
            0
        };
        let mut samples = SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
        samples.copy_interleaved_ref(decoded);
        #[cfg(feature = "l01-memory-qualification")]
        if let Some(observer) = self.observer {
            observer(
                "codec.sample_buffer.live",
                serde_json::json!({
                    "packet_index": self.packet_index,
                    "packet_payload_bytes": packet.data.len(),
                    "decoded_frame_capacity": decoded_frame_capacity,
                    "decoded_frames": decoded_frame_count,
                    "sample_buffer_initialized_samples": samples.len(),
                    "sample_buffer_capacity_samples": samples.capacity(),
                    "sample_buffer_capacity_bytes": samples.capacity() * std::mem::size_of::<f32>(),
                    "channel_count": self.channels,
                }),
            );
        }
        self.process_samples(packet, &samples)
    }

    fn process_samples(
        &mut self,
        packet: &Packet,
        samples: &SampleBuffer<f32>,
    ) -> Result<(), String> {
        let interleaved = samples.samples();
        let count = interleaved.len() / self.channels;
        for frame in 0..count {
            let timeline_index = self.input_index;
            self.input_index += 1;
            self.decoded_frames += 1;
            let current = interleaved[frame * self.channels..(frame + 1) * self.channels]
                .iter()
                .copied()
                .sum::<f32>()
                / self.channels as f32;
            if let Some(prev) = self.previous {
                while self.next_output.saturating_mul(u64::from(self.sample_rate))
                    <= timeline_index.saturating_mul(u64::from(OUTPUT_RATE))
                {
                    let numerator = self.next_output.saturating_mul(u64::from(self.sample_rate));
                    let base = numerator / u64::from(OUTPUT_RATE);
                    let frac = (numerator % u64::from(OUTPUT_RATE)) as f32 / OUTPUT_RATE as f32;
                    let value = if base == timeline_index {
                        current
                    } else {
                        prev + (current - prev) * frac
                    };
                    self.window.push(value.clamp(-1.0, 1.0));
                    self.output_samples += 1;
                    self.next_output += 1;
                    if self.window.len() == MAX_WINDOW_SAMPLES {
                        self.emit_full_window(packet, samples)?;
                    }
                }
            } else {
                self.window.push(current.clamp(-1.0, 1.0));
                self.output_samples = 1;
                self.next_output = 1;
            }
            self.previous = Some(current);
        }
        Ok(())
    }

    fn emit_full_window(
        &mut self,
        packet: &Packet,
        samples: &SampleBuffer<f32>,
    ) -> Result<(), String> {
        #[cfg(not(feature = "l01-memory-qualification"))]
        let _ = (packet, samples);
        let start = self.output_samples - self.window.len() as u64;
        let full_window =
            std::mem::replace(&mut self.window, Vec::with_capacity(MAX_WINDOW_SAMPLES));
        #[cfg(feature = "l01-memory-qualification")]
        if self.observer.is_some() {
            self.window_index += 1;
        }
        #[cfg(feature = "l01-memory-qualification")]
        if let Some(observer) = self.observer {
            observer(
                "pcm.window.callback.begin",
                serde_json::json!({
                    "window_index": self.window_index,
                    "source_start_sample": start,
                    "source_sample_count_total": self.output_samples,
                    "outgoing_window_len": full_window.len(),
                    "outgoing_window_capacity": full_window.capacity(),
                    "replacement_window_len": self.window.len(),
                    "replacement_window_capacity": self.window.capacity(),
                    "simultaneous_pcm_window_capacity_bytes": (full_window.capacity() + self.window.capacity()) * std::mem::size_of::<f32>(),
                    "codec_packet_payload_bytes_still_live": packet.data.len(),
                    "codec_sample_buffer_capacity_bytes_still_live": samples.capacity() * std::mem::size_of::<f32>(),
                }),
            );
        }
        (self.on_window)(start, full_window)?;
        #[cfg(feature = "l01-memory-qualification")]
        if let Some(observer) = self.observer {
            observer(
                "pcm.window.callback.complete",
                serde_json::json!({
                    "window_index": self.window_index,
                    "replacement_window_len": self.window.len(),
                    "replacement_window_capacity": self.window.capacity(),
                }),
            );
        }
        Ok(())
    }

    fn finish(mut self) -> Result<(u64, u64), String> {
        if !self.window.is_empty() {
            let start = self.output_samples - self.window.len() as u64;
            #[cfg(feature = "l01-memory-qualification")]
            if self.observer.is_some() {
                self.window_index += 1;
            }
            #[cfg(feature = "l01-memory-qualification")]
            let tail_samples = if self.observer.is_some() {
                self.window.len()
            } else {
                0
            };
            #[cfg(feature = "l01-memory-qualification")]
            let tail_capacity = if self.observer.is_some() {
                self.window.capacity()
            } else {
                0
            };
            #[cfg(feature = "l01-memory-qualification")]
            if let Some(observer) = self.observer {
                observer(
                    "pcm.window.callback.begin",
                    serde_json::json!({
                        "window_index": self.window_index,
                        "source_start_sample": start,
                        "source_sample_count_total": self.output_samples,
                        "outgoing_window_len": self.window.len(),
                        "outgoing_window_capacity": self.window.capacity(),
                        "replacement_window_capacity": 0,
                        "simultaneous_pcm_window_capacity_bytes": self.window.capacity() * std::mem::size_of::<f32>(),
                    }),
                );
            }
            (self.on_window)(start, self.window)?;
            #[cfg(feature = "l01-memory-qualification")]
            if let Some(observer) = self.observer {
                observer(
                    "pcm.window.callback.complete",
                    serde_json::json!({
                        "window_index": self.window_index,
                        "released_window_samples": tail_samples,
                        "released_window_capacity": tail_capacity,
                        "replacement_window_capacity": 0,
                    }),
                );
            }
        }
        #[cfg(feature = "l01-memory-qualification")]
        if let Some(observer) = self.observer {
            observer(
                "codec.complete",
                serde_json::json!({
                    "packet_count": self.packet_index,
                    "window_count": self.window_index,
                    "decoded_frames": self.decoded_frames,
                    "output_samples": self.output_samples,
                    "retained_full_source_pcm_samples": 0,
                }),
            );
        }
        Ok((self.decoded_frames, self.output_samples))
    }
}

fn decode_packets<F: FnMut(u64, Vec<f32>) -> Result<(), String>>(
    format: &mut dyn FormatReader,
    decoder: &mut dyn Decoder,
    track_id: u32,
    state: &mut DecodeState<F>,
) -> Result<(), String> {
    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(Error::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => return Err(format!("audio packet read failed: {e}")),
        };
        #[cfg(feature = "l01-memory-qualification")]
        if state.observer.is_some() {
            state.packet_index += 1;
        }
        #[cfg(feature = "l01-memory-qualification")]
        if let Some(observer) = state.observer {
            observer(
                "codec.packet.read",
                serde_json::json!({
                    "packet_index": state.packet_index,
                    "packet_payload_bytes": packet.data.len(),
                    "packet_duration_timebase_units": packet.dur,
                }),
            );
        }
        if packet.track_id() != track_id {
            continue;
        }
        state.process_packet(decoder, &packet)?;
    }
    Ok(())
}

#[allow(dead_code)]
fn _audio_buffer_ref_is_supported(buffer: &AudioBufferRef<'_>) -> usize {
    buffer.frames()
}

#[cfg(test)]
mod tests;
