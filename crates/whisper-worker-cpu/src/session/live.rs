use super::*;
use crate::native_engine;

pub(super) fn run_live_service(
    rx: &Receiver<Incoming>,
    output: &mut impl io::Write,
    engine: &CpuEngine,
    job_id: whisper_core::JobId,
    generation: whisper_core::Generation,
    instance_id: u64,
    config: whisper_core::JobConfig,
    encode_audio: bool,
) -> Result<(), String> {
    let mut expected_sequence = 1_u64;
    let mut next_segment_id = 1_u64;
    let mut next_packet_id = 1_u64;
    let mut mp3 = encode_audio.then(encoder::LiveMp3Encoder::new);
    loop {
        let envelope = rx
            .recv()
            .map_err(|_| "live IPC closed before Stop".to_owned())??;
        if envelope.version != IPC_PROTOCOL_VERSION {
            return Err("ProtocolMismatch: live command version changed".into());
        }
        let transcribe_window = matches!(&envelope.message, WorkerCommand::LiveWindow { .. });
        l04_campaign_gate(
            "worker.before_command",
            job_id.0,
            generation.get(),
            instance_id,
        )?;
        match envelope.message {
            WorkerCommand::LiveWindow {
                job_id: target,
                generation: target_generation,
                instance_id: target_instance,
                sequence,
                range,
                samples,
            }
            | WorkerCommand::LiveReplayWindow {
                job_id: target,
                generation: target_generation,
                instance_id: target_instance,
                sequence,
                range,
                samples,
            } => {
                if target != job_id
                    || target_generation != generation
                    || target_instance != instance_id
                    || sequence != expected_sequence
                    || range.sample_rate_hz != 16_000
                    || range.start_sample >= range.end_sample
                    || range.end_sample - range.start_sample != samples.len() as u64
                    || samples.is_empty()
                    || samples.len() > native_engine::MAX_LIVE_WINDOW_SAMPLES
                {
                    return emit_failure(
                        output,
                        job_id,
                        generation,
                        instance_id,
                        WorkerErrorCode::InvalidRequest,
                        "InvalidRequest: live PCM identity or window bounds are invalid",
                    );
                }
                if let Some(mp3) = mp3.as_mut() {
                    for chunk in samples.chunks(16_000) {
                        l04_campaign_gate(
                            "worker.before_encode",
                            job_id.0,
                            generation.get(),
                            instance_id,
                        )?;
                        let encoded = mp3.push_pcm(chunk);
                        l04_campaign_worker_result(
                            "worker.before_encode",
                            job_id.0,
                            generation.get(),
                            instance_id,
                            &encoded,
                        )?;
                        for bytes in encoded? {
                            let sequence = next_packet_id;
                            next_packet_id = next_packet_id
                                .checked_add(1)
                                .ok_or_else(|| "live MP3 packet sequence exhausted".to_owned())?;
                            write_frame(
                                output,
                                &IpcEnvelope::new(WorkerEvent::LiveMp3Packet {
                                    job_id,
                                    generation,
                                    instance_id,
                                    sequence,
                                    bytes,
                                }),
                            )
                            .map_err(|e| e.to_string())?;
                        }
                    }
                }
                expected_sequence = expected_sequence
                    .checked_add(1)
                    .ok_or_else(|| "live window sequence exhausted".to_owned())?;
                if transcribe_window {
                    let pcm: Vec<f32> = samples
                        .iter()
                        .map(|sample| *sample as f32 / 32768.0)
                        .collect();
                    let language = match &config.language {
                        whisper_core::LanguageChoice::Automatic => None,
                        whisper_core::LanguageChoice::Manual(code) => Some(code.as_str()),
                    };
                    let segments = match engine.transcribe(&pcm, language) {
                        Ok(segments) => segments,
                        Err(error) => {
                            return emit_failure(
                                output,
                                job_id,
                                generation,
                                instance_id,
                                WorkerErrorCode::InferenceFailed,
                                &error,
                            );
                        }
                    };
                    for segment in segments {
                        let start = range
                            .start_sample
                            .saturating_add(
                                (segment.start_centiseconds.max(0) as u64).saturating_mul(160),
                            )
                            .min(range.end_sample);
                        let end = range
                            .start_sample
                            .saturating_add(
                                (segment.end_centiseconds.max(0) as u64).saturating_mul(160),
                            )
                            .min(range.end_sample);
                        if end <= start || segment.text.trim().is_empty() {
                            continue;
                        }
                        let segment_id = next_segment_id;
                        next_segment_id = next_segment_id
                            .checked_add(1)
                            .ok_or_else(|| "live segment id exhausted".to_owned())?;
                        let message = WorkerEvent::Segment(WorkerSegmentDto {
                            job_id,
                            generation,
                            instance_id,
                            segment_id: SegmentId(segment_id),
                            range: SourceRange::new(start, end, 16_000)
                                .ok_or_else(|| "live segment range is invalid".to_owned())?,
                            text: segment.text.trim().to_owned(),
                        });
                        write_frame(output, &IpcEnvelope::new(message))
                            .map_err(|e| e.to_string())?;
                    }
                }
                write_frame(
                    output,
                    &IpcEnvelope::new(WorkerEvent::WindowFinished {
                        job_id,
                        generation,
                        instance_id,
                        sequence,
                        range,
                        last_segment_sequence: next_segment_id - 1,
                    }),
                )
                .map_err(|e| e.to_string())?;
                l04_campaign_gate(
                    "worker.after_window",
                    job_id.0,
                    generation.get(),
                    instance_id,
                )?;
            }
            WorkerCommand::Stop {
                job_id: target,
                generation: target_generation,
            } if target == job_id && target_generation == generation => {
                l04_campaign_gate(
                    "worker.before_finish_encode",
                    job_id.0,
                    generation.get(),
                    instance_id,
                )?;
                let finished = mp3
                    .as_mut()
                    .map_or(Ok(Vec::new()), encoder::LiveMp3Encoder::finish);
                l04_campaign_worker_result(
                    "worker.before_finish_encode",
                    job_id.0,
                    generation.get(),
                    instance_id,
                    &finished,
                )?;
                for bytes in finished? {
                    let sequence = next_packet_id;
                    next_packet_id = next_packet_id
                        .checked_add(1)
                        .ok_or_else(|| "live MP3 packet sequence exhausted".to_owned())?;
                    write_frame(
                        output,
                        &IpcEnvelope::new(WorkerEvent::LiveMp3Packet {
                            job_id,
                            generation,
                            instance_id,
                            sequence,
                            bytes,
                        }),
                    )
                    .map_err(|e| e.to_string())?;
                }
                l04_campaign_gate(
                    "worker.before_stopped",
                    job_id.0,
                    generation.get(),
                    instance_id,
                )?;
                let stopped_result = write_frame(
                    output,
                    &IpcEnvelope::new(WorkerEvent::Stopped {
                        job_id,
                        generation,
                        instance_id,
                    }),
                )
                .map_err(|e| e.to_string());
                l04_campaign_worker_result(
                    "worker.before_stopped",
                    job_id.0,
                    generation.get(),
                    instance_id,
                    &stopped_result,
                )?;
                return stopped_result;
            }
            WorkerCommand::Shutdown => return Ok(()),
            _ => {
                return emit_failure(
                    output,
                    job_id,
                    generation,
                    instance_id,
                    WorkerErrorCode::InvalidRequest,
                    "InvalidRequest: unexpected command in live worker",
                );
            }
        }
    }
}
