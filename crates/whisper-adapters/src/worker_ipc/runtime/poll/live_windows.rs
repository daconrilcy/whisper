#![warn(clippy::too_many_lines, clippy::cognitive_complexity)]

use super::*;

pub(super) enum LiveMp3PacketControl {
    Continue,
    ReturnNone,
}

pub(super) enum WindowFinishedControl {
    Continue,
    ReturnNone,
    ImmediateError(JobId, Generation, String),
    Confirm(FinishedLiveWindow),
}

impl ImportRuntime {
    pub(super) fn poll_live_mp3_packet(
        &mut self,
        event: &WorkerEvent,
        job_id: JobId,
        generation: Generation,
        instance_id: u64,
    ) -> LiveMp3PacketControl {
        if let WorkerEvent::LiveMp3Packet {
            job_id: packet_job,
            generation: packet_generation,
            instance_id: packet_instance,
            bytes,
            ..
        } = event
        {
            if (*packet_job, *packet_generation, *packet_instance)
                != (job_id, generation, instance_id)
            {
                return LiveMp3PacketControl::ReturnNone;
            }
            if let Some(archive) = self.live_archive.as_mut() {
                if let Err(error) = archive.append_mp3_packet(bytes) {
                    if let Some(live) = self.live.as_mut() {
                        live.stop_capture();
                        let _ = live.staging.drain();
                        live.failure = Some(error.clone());
                    }
                    if let Some(transport) = self.transport.as_mut() {
                        let _ = transport.try_request(WorkerCommand::Stop { job_id, generation });
                    }
                    return LiveMp3PacketControl::ReturnNone;
                }
                return LiveMp3PacketControl::ReturnNone;
            }
        }
        LiveMp3PacketControl::Continue
    }

    pub(super) fn poll_window_finished(
        &mut self,
        event: &WorkerEvent,
        job_id: JobId,
        generation: Generation,
    ) -> WindowFinishedControl {
        if let WorkerEvent::WindowFinished {
            sequence,
            range,
            last_segment_sequence,
            ..
        } = event
        {
            if self.live.is_none() {
                return WindowFinishedControl::ImmediateError(
                    job_id,
                    generation,
                    "ProtocolMismatch: WindowFinished outside a live pass".into(),
                );
            }
            let Some(live) = self.live.as_ref() else {
                unreachable!()
            };
            // Confirmations wholly inside T were already durable before this attempt.
            // Replay only rebuilds the MP3 encoder and must not create duplicate coverage.
            if range.end_sample <= live.replay_until {
                return WindowFinishedControl::ReturnNone;
            }
            let Some(window_sequence) = map_attempt_sequence(
                live.window_sequence_base,
                live.replay_windows_to_skip,
                *sequence,
            ) else {
                return WindowFinishedControl::ImmediateError(
                    job_id,
                    generation,
                    "ProtocolMismatch: live window sequence cannot be mapped to durable prefix"
                        .into(),
                );
            };
            let last_segment_sequence = if *last_segment_sequence == 0 {
                live.segment_sequence_base.checked_sub(1)
            } else {
                map_attempt_sequence(live.segment_sequence_base, 0, *last_segment_sequence)
            };
            let Some(last_segment_sequence) = last_segment_sequence else {
                return WindowFinishedControl::ImmediateError(
                    job_id,
                    generation,
                    "ProtocolMismatch: live segment sequence cannot be mapped to durable prefix"
                        .into(),
                );
            };
            let finished = FinishedLiveWindow {
                sequence: window_sequence,
                range: *range,
                last_segment_sequence,
            };
            if self.live_pending_persistence > 0 {
                self.deferred_live_window_finished = Some(finished);
                return WindowFinishedControl::ReturnNone;
            }
            return WindowFinishedControl::Confirm(finished);
        }
        WindowFinishedControl::Continue
    }
}
