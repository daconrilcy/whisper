use super::*;

impl ImportRuntime {
    pub(super) fn poll_live_capture(
        &mut self,
    ) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
        self.pump_live_windows();
        let live = self.live.as_mut()?;
        if live.stop_requested {
            return None;
        }
        if live.capture.is_saturated() || live.capture.failure() {
            let (job_id, generation) = (live.request.job_id, live.request.generation);
            let reason = if live.capture.is_saturated() {
                "CaptureSaturated: bounded callback slots are full"
            } else {
                "CaptureInterrupted: WASAPI stream failed"
            };
            live.stop_capture();
            live.failure = Some(reason.into());
            if let Some(transport) = self.transport.as_mut() {
                let _ = transport.try_request(WorkerCommand::Stop { job_id, generation });
            }
            return None;
        }
        let block = live.capture.try_next_block()?;
        let converted = match live.converter.push_with_probe(&block) {
            Ok(converted) => converted,
            Err(error) => {
                live.stop_capture();
                live.failure = Some(error.clone());
                if let Some(transport) = self.transport.as_mut() {
                    let _ = transport.try_request(WorkerCommand::Stop {
                        job_id: live.request.job_id,
                        generation: live.request.generation,
                    });
                }
                return None;
            }
        };
        if let Err(error) = live.capture.recycle_block(block) {
            live.stop_capture();
            live.failure = Some(error.clone());
            if let Some(transport) = self.transport.as_mut() {
                let _ = transport.try_request(WorkerCommand::Stop {
                    job_id: live.request.job_id,
                    generation: live.request.generation,
                });
            }
            return None;
        }
        live.probe.submit(converted.probe);
        live.frame_pending.extend(converted.samples);
        while live.frame_pending.len() >= 320 {
            let frame: Vec<i16> = live.frame_pending.drain(..320).collect();
            live.captured = live.captured.saturating_add(320);
            match live.vad.is_speech(&frame) {
                Ok(true) => live.speech = live.speech.saturating_add(320),
                Ok(false) => {}
                Err(error) => {
                    live.stop_capture();
                    live.failure = Some(error.clone());
                    if let Some(transport) = self.transport.as_mut() {
                        let _ = transport.try_request(WorkerCommand::Stop {
                            job_id: live.request.job_id,
                            generation: live.request.generation,
                        });
                    }
                    return None;
                }
            }
            if let Err(error) = live.staging.try_append_frame(frame) {
                live.stop_capture();
                let reason = if error.kind() == io::ErrorKind::WouldBlock {
                    "CaptureSaturated: bounded PCM writer queue is full".to_owned()
                } else {
                    format!("StorageUnavailable: {error}")
                };
                live.failure = Some(reason);
                if let Some(transport) = self.transport.as_mut() {
                    let _ = transport.try_request(WorkerCommand::Stop {
                        job_id: live.request.job_id,
                        generation: live.request.generation,
                    });
                }
                return None;
            }
            live.admitted = live.admitted.saturating_add(320);
        }
        let durable_samples = live.staging.durable_samples();
        if durable_samples == live.last_reported_durable {
            return None;
        }
        live.last_reported_durable = durable_samples;
        let event = ImportEvent::LiveCounters {
            job_id: live.request.job_id,
            generation: live.request.generation,
            captured_samples: live.captured,
            admitted_samples: live.admitted,
            audio_durable_samples: durable_samples,
            confirmed_fragment_samples: live.confirmed_fragment_end,
            speech_samples: live.speech,
            diagnostic: None,
        };
        Some(Ok(event))
    }

    fn pump_live_windows(&mut self) {
        let (Some(live), Some(transport)) = (self.live.as_mut(), self.transport.as_ref()) else {
            return;
        };
        if live.inference_admission_closed || !live.worker_ready {
            return;
        }
        let durable_samples = live.staging.durable_samples();
        if live.pending_window.is_none() {
            let available = durable_samples.saturating_sub(live.inference_cursor);
            let replay_remaining = live.replay_until.saturating_sub(live.inference_cursor);
            let sample_count = if available == 0 {
                0
            } else if replay_remaining > 0 {
                available.min(80_000).min(replay_remaining) as usize
            } else if available >= 80_000 {
                80_000
            } else if live.stop_requested {
                available as usize
            } else {
                0
            };
            if sample_count > 0 {
                match live
                    .staging
                    .read_durable_samples(live.inference_cursor, sample_count)
                {
                    Ok(Some(samples)) => {
                        let start = live.inference_cursor;
                        let end = start.saturating_add(sample_count as u64);
                        live.pending_window = Some((live.inference_sequence, start, end, samples));
                    }
                    Ok(None) => return,
                    Err(error) => {
                        live.failure = Some(format!("StorageUnavailable: PCM spool read: {error}"));
                        live.stop_capture();
                        return;
                    }
                }
            }
        }
        if let Some((sequence, start, end, samples)) = live.pending_window.take() {
            let Some(active) = self.active.as_ref() else {
                live.pending_window = Some((sequence, start, end, samples));
                return;
            };
            let Some(range) = whisper_core::SourceRange::new(start, end, 16_000) else {
                live.failure = Some("StorageCorrupt: live inference range is invalid".into());
                live.stop_capture();
                return;
            };
            let replay = start < live.replay_until;
            let command = if replay {
                WorkerCommand::LiveReplayWindow {
                    job_id: active.job_id,
                    generation: active.generation,
                    instance_id: active.instance_id,
                    sequence,
                    range,
                    samples,
                }
            } else {
                WorkerCommand::LiveWindow {
                    job_id: active.job_id,
                    generation: active.generation,
                    instance_id: active.instance_id,
                    sequence,
                    range,
                    samples,
                }
            };
            match transport.try_request(command) {
                Ok(()) => {
                    live.inference_cursor = end;
                    live.inference_sequence = live.inference_sequence.saturating_add(1);
                    if replay {
                        live.replay_windows_queued = live.replay_windows_queued.saturating_add(1);
                    }
                }
                Err(EnqueueError::Full(command)) => {
                    let (sequence, range, samples) = match *command {
                        WorkerCommand::LiveWindow {
                            sequence,
                            range,
                            samples,
                            ..
                        }
                        | WorkerCommand::LiveReplayWindow {
                            sequence,
                            range,
                            samples,
                            ..
                        } => (sequence, range, samples),
                        _ => unreachable!("only live windows are queued here"),
                    };
                    live.pending_window =
                        Some((sequence, range.start_sample, range.end_sample, samples));
                    return;
                }
                Err(EnqueueError::Disconnected) => {
                    live.failure = Some("WorkerExited: stdin dispatcher stopped".into());
                    live.stop_capture();
                    return;
                }
            }
        }
        let durable_samples = live.staging.durable_samples();
        if live.stop_requested
            && live.pending_window.is_none()
            && live.inference_cursor >= durable_samples
            && !live.stop_enqueued
        {
            let Some(active) = self.active.as_ref() else {
                return;
            };
            match transport.try_request_ordered(WorkerCommand::Stop {
                job_id: active.job_id,
                generation: active.generation,
            }) {
                Ok(()) => live.stop_enqueued = true,
                Err(EnqueueError::Full(_)) => {}
                Err(EnqueueError::Disconnected) => {
                    live.failure = Some("WorkerExited: stdin dispatcher stopped".into());
                }
            }
        }
    }

    pub(super) fn take_legacy_live_telemetry(&mut self) -> Option<ImportEvent> {
        let live = self.live.as_mut()?;
        if live.next_spectrum_telemetry {
            live.next_spectrum_telemetry = false;
            if let Some(update) = live.spectrum_reader.poll(&live.probe, Instant::now()) {
                return Some(ImportEvent::MicrophoneSpectrum(update));
            }
        } else {
            live.next_spectrum_telemetry = true;
        }
        let snapshot = live.probe.latest_at(Instant::now())?;
        if snapshot.sequence <= live.last_meter_sequence {
            return None;
        }
        let meter = snapshot.meter()?;
        live.last_meter_sequence = snapshot.sequence;
        Some(ImportEvent::MicrophoneMeter(meter))
    }
}
