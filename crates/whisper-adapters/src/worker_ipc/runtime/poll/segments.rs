#![warn(clippy::too_many_lines, clippy::cognitive_complexity)]

use super::*;
use whisper_core::ipc::WorkerSegmentDto;

pub(super) enum ResumePrefixControl {
    Continue,
    Suppress,
    Reject((JobId, Generation, String)),
}

impl ImportRuntime {
    pub(super) fn poll_resume_prefix(
        &mut self,
        event: &WorkerEvent,
        job_id: JobId,
        generation: Generation,
    ) -> ResumePrefixControl {
        if let WorkerEvent::Segment(dto) = event
            && let Some(expected) = self.resume_prefix.pop_front()
        {
            if dto.segment_id.0 != expected.sequence
                || dto.range != expected.range
                || dto.text != expected.text
            {
                self.transport = None;
                self.active = None;
                self.resume_prefix.clear();
                return ResumePrefixControl::Reject((
                    job_id,
                    generation,
                    "ResumeMismatch: recomputed segment differs from the confirmed prefix".into(),
                ));
            }
            return ResumePrefixControl::Suppress;
        }
        if matches!(event, WorkerEvent::End { .. }) && !self.resume_prefix.is_empty() {
            self.transport = None;
            self.active = None;
            self.resume_prefix.clear();
            return ResumePrefixControl::Reject((
                job_id,
                generation,
                "ResumeMismatch: worker ended before reproducing the confirmed prefix".into(),
            ));
        }
        ResumePrefixControl::Continue
    }

    pub(super) fn poll_import_segment(
        &mut self,
        dto: WorkerSegmentDto,
        job_id: JobId,
        generation: Generation,
    ) -> Result<ImportEvent, (JobId, Generation, String)> {
        let sequence_base = self
            .live
            .as_ref()
            .filter(|live| live.request.job_id == job_id && live.request.generation == generation)
            .map_or(1, |live| live.segment_sequence_base);
        let Some(segment_sequence) = map_attempt_sequence(sequence_base, 0, dto.segment_id.0)
        else {
            return Err((
                job_id,
                generation,
                "ProtocolMismatch: live segment sequence exhausted".into(),
            ));
        };
        let segment = WorkerSegment {
            job_id: dto.job_id,
            generation: dto.generation,
            instance_id: dto.instance_id,
            segment_id: whisper_core::SegmentId(segment_sequence),
            range: dto.range,
            text: dto.text,
        };
        if let Some(live) = self
            .live
            .as_ref()
            .filter(|live| live.request.job_id == job_id && live.request.generation == generation)
        {
            self.live_pending_persistence = self.live_pending_persistence.saturating_add(1);
            let captured_at_unix_ms = live
                .started_at_unix_ms
                .saturating_add(segment.range.start_sample.saturating_mul(1_000) / 16_000);
            Ok(ImportEvent::LiveProvisional {
                segment,
                captured_at_unix_ms,
                available_at_unix_ms: unix_time_ms(),
            })
        } else {
            Ok(ImportEvent::Segment(segment))
        }
    }

    pub(super) fn poll_import_end(
        &mut self,
        j: JobId,
        g: Generation,
        last_sequence: u64,
        last_offset: u64,
    ) -> ImportEvent {
        if let (Some(active), Some(transport)) = (self.active.as_ref(), self.transport.take())
            && let Some(request) = active.import_request.as_ref()
        {
            self.pending_idle_worker = Some(IdleWorker {
                transport,
                compute: match active.backend {
                    BackendKind::Cpu => whisper_core::ComputeChoice::Cpu,
                    BackendKind::Cuda => whisper_core::ComputeChoice::Gpu,
                },
                model_path: request.model_path.clone(),
                model_sha256: request.model_sha256,
                expires_at: Instant::now(),
            });
        }
        self.active = None;
        ImportEvent::End {
            job_id: j,
            generation: g,
            last_sequence,
            last_offset,
        }
    }
}
