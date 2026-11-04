use super::*;
use live_windows::{LiveMp3PacketControl, WindowFinishedControl};
use readiness::{AdmissionAckControl, AttestationControl, GpuSessionAttestation, ReadyControl};
use segments::ResumePrefixControl;

mod active_live_stop;
mod capture;
mod collection;
mod live_windows;
mod observations;
mod parakeet;
mod readiness;
mod segments;
mod stop_recovery;
#[cfg(test)]
mod tests;
pub(in crate::worker_ipc) use parakeet::ParakeetVadRun;
#[cfg(test)]
pub(in crate::worker_ipc) use parakeet::{
    ParakeetStoppedExit, TDT_FRAME_OVERLAP_SAMPLES, append_single_pass_parakeet_word_group,
    parakeet_stopped_exit, parakeet_vad_segments, vad_word_overlap_within_one_tdt_frame,
    validate_single_pass_parakeet_words,
};

pub(in crate::worker_ipc) fn take_ready_live_stop(
    pending_persistence: usize,
    deferred: &mut Option<WorkerEvent>,
) -> Option<WorkerEvent> {
    (pending_persistence == 0)
        .then(|| deferred.take())
        .flatten()
}

pub(in crate::worker_ipc) fn bind_unidentified_startup_failure(
    event: WorkerEvent,
    attempt: (JobId, Generation, u64),
    startup_ready: bool,
) -> WorkerEvent {
    if startup_ready {
        return event;
    }
    match event {
        WorkerEvent::Failed {
            job_id: None,
            generation: None,
            instance_id: None,
            code,
            message,
        } => WorkerEvent::Failed {
            job_id: Some(attempt.0),
            generation: Some(attempt.1),
            instance_id: Some(attempt.2),
            code,
            message,
        },
        event => event,
    }
}

pub(in crate::worker_ipc) fn worker_event_identity(
    event: &WorkerEvent,
) -> Option<(JobId, Generation, u64)> {
    match event {
        WorkerEvent::Ready {
            job_id,
            generation,
            instance_id,
            ..
        }
        | WorkerEvent::ImportStarted {
            job_id,
            generation,
            instance_id,
        }
        | WorkerEvent::LiveMp3Packet {
            job_id,
            generation,
            instance_id,
            ..
        }
        | WorkerEvent::WindowFinished {
            job_id,
            generation,
            instance_id,
            ..
        }
        | WorkerEvent::Stopped {
            job_id,
            generation,
            instance_id,
        }
        | WorkerEvent::Progress {
            job_id,
            generation,
            instance_id,
            ..
        }
        | WorkerEvent::End {
            job_id,
            generation,
            instance_id,
            ..
        } => Some((*job_id, *generation, *instance_id)),
        WorkerEvent::GpuSessionAttested {
            job_id,
            generation,
            instance_id,
            ..
        } => Some((*job_id, *generation, *instance_id)),
        WorkerEvent::Segment(segment) => {
            Some((segment.job_id, segment.generation, segment.instance_id))
        }
        WorkerEvent::Failed {
            job_id: Some(job_id),
            generation: Some(generation),
            instance_id: Some(instance_id),
            ..
        } => Some((*job_id, *generation, *instance_id)),
        WorkerEvent::Failed { .. }
        | WorkerEvent::DecodedBlock { .. }
        | WorkerEvent::CudaAvailable
        | WorkerEvent::GpuCandidateAttested { .. }
        | WorkerEvent::GpusQualified { .. } => None,
    }
}

pub(in crate::worker_ipc) fn worker_event_is_stale(
    event: &WorkerEvent,
    attempt: (JobId, Generation, u64),
) -> bool {
    worker_event_identity(event).is_some_and(|identity| identity != attempt)
}

pub(in crate::worker_ipc) fn map_attempt_sequence(
    base: u64,
    replay_prefix: u64,
    attempt_sequence: u64,
) -> Option<u64> {
    attempt_sequence
        .checked_sub(replay_prefix)?
        .checked_sub(1)?
        .checked_add(base)
}

impl ImportRuntime {
    #[warn(clippy::too_many_lines, clippy::cognitive_complexity)]
    pub(in crate::worker_ipc) fn poll(
        &mut self,
    ) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
        let mut event = match self.collect_poll_event() {
            collection::PollCollectionControl::Next => return None,
            collection::PollCollectionControl::Return(output) => return output,
            collection::PollCollectionControl::Finish(event) => event,
        };
        let active = self.active.as_ref()?;
        let (job_id, generation, instance_id) =
            (active.job_id, active.generation, active.instance_id);
        // A failure received on the newly launched worker's own transport before
        // Ready may omit its identity. Bind only a wholly uncorrelated startup
        // failure to this attempt; partially or fully stale identities stay rejected.
        event = bind_unidentified_startup_failure(
            event,
            (job_id, generation, instance_id),
            active.startup_ready,
        );
        if worker_event_is_stale(&event, (job_id, generation, instance_id)) {
            return None;
        }
        if let WorkerEvent::ImportStarted { .. } = &event {
            return match self.poll_import_started_ack(job_id, generation, instance_id) {
                AdmissionAckControl::Accepted => None,
                AdmissionAckControl::Rejected(message) => Some(Err((job_id, generation, message))),
            };
        }
        if matches!(
            &event,
            WorkerEvent::Failed { job_id: None, .. }
                | WorkerEvent::Failed {
                    generation: None,
                    ..
                }
                | WorkerEvent::Failed {
                    instance_id: None,
                    ..
                }
        ) {
            return Some(self.handle_worker_failure(
                job_id,
                generation,
                whisper_core::ipc::WorkerErrorCode::ProtocolMismatch,
                "ProtocolMismatch: worker failure lacks current attempt identity".into(),
            ));
        }
        let backend = active.backend;
        let live = self.live.is_some();
        self.observe_worker_event(&event, job_id, generation, backend, live);
        if matches!(
            &event,
            WorkerEvent::Stopped {
                job_id: stopped_job,
                generation: stopped_generation,
                instance_id: stopped_instance,
            } if (*stopped_job, *stopped_generation, *stopped_instance)
                == (job_id, generation, instance_id)
        ) && (self.live.is_some()
            || self
                .pending_live
                .as_ref()
                .is_some_and(|pending| pending.recovery != LiveRecoveryState::None)
            || self
                .active
                .as_ref()
                .is_some_and(|active| active.recovery != LiveRecoveryState::None))
            && (self.live_pending_persistence > 0
                || self
                    .transport
                    .as_mut()
                    .and_then(|transport| transport.try_wait().ok().flatten())
                    .is_none())
        {
            self.deferred_live_stopped = Some(event);
            return None;
        }
        match self.poll_live_mp3_packet(&event, job_id, generation, instance_id) {
            LiveMp3PacketControl::Continue => {}
            LiveMp3PacketControl::ReturnNone => return None,
        }
        match self.poll_window_finished(&event, job_id, generation) {
            WindowFinishedControl::Continue => {}
            WindowFinishedControl::ReturnNone => return None,
            WindowFinishedControl::ImmediateError(job_id, generation, message) => {
                return Some(Err((job_id, generation, message)));
            }
            WindowFinishedControl::Confirm(finished) => {
                return Some(self.confirm_live_window(finished));
            }
        }
        if let Some(result) = self.poll_stopped_pending_live_recovery(&event, job_id, generation) {
            return Some(result);
        }
        if let Some(result) = self.poll_stopped_import_auto_retry(&event, job_id, generation) {
            return Some(result);
        }
        if let Some(result) = self.poll_stopped_active_live(&event, job_id, generation) {
            return Some(result);
        }
        match self.poll_resume_prefix(&event, job_id, generation) {
            ResumePrefixControl::Continue => {}
            ResumePrefixControl::Suppress => return None,
            ResumePrefixControl::Reject(failure) => return Some(Err(failure)),
        }
        let result = match event {
            WorkerEvent::GpuSessionAttested {
                job_id: j,
                generation: g,
                instance_id: i,
                candidate,
                dxgi_rank,
                luid,
                pci_id,
                cuda_ordinal,
                ggml_ordinal,
            } if (j, g, i) == (job_id, generation, instance_id) => {
                match self.poll_gpu_session_attestation(GpuSessionAttestation {
                    candidate,
                    dxgi_rank,
                    luid,
                    pci_id,
                    cuda_ordinal,
                    ggml_ordinal,
                }) {
                    AttestationControl::Accepted => return None,
                    AttestationControl::Mismatch => self.handle_worker_failure(
                        j,
                        g,
                        whisper_core::ipc::WorkerErrorCode::BackendMismatch,
                        "BackendMismatch: worker attested a different GPU candidate".into(),
                    ),
                }
            }
            WorkerEvent::Ready {
                job_id: j,
                generation: g,
                instance_id: i,
                backend,
            } if (j, g, i) == (job_id, generation, instance_id)
                && self
                    .active
                    .as_ref()
                    .is_some_and(|active| active.backend == backend) =>
            {
                match self.poll_ready_backend(j, g, i, backend) {
                    ReadyControl::ReturnImmediately(result) => return Some(result),
                    ReadyControl::Finish(result) => result,
                }
            }
            WorkerEvent::Ready { .. } => self.handle_worker_failure(
                job_id,
                generation,
                whisper_core::ipc::WorkerErrorCode::BackendMismatch,
                "BackendMismatch: worker attestation differs from the selected backend".into(),
            ),
            WorkerEvent::Progress {
                job_id: j,
                generation: g,
                instance_id: i,
                completed_samples,
                total_samples,
            } if (j, g, i) == (job_id, generation, instance_id) => Ok(ImportEvent::Progress {
                job_id: j,
                generation: g,
                completed_samples,
                total_samples,
            }),
            WorkerEvent::Segment(dto)
                if (dto.job_id, dto.generation, dto.instance_id)
                    == (job_id, generation, instance_id) =>
            {
                match self.poll_import_segment(dto, job_id, generation) {
                    Ok(event) => Ok(event),
                    Err(failure) => return Some(Err(failure)),
                }
            }
            WorkerEvent::End {
                job_id: j,
                generation: g,
                instance_id: i,
                last_sequence,
                last_offset,
            } if (j, g, i) == (job_id, generation, instance_id) => {
                Ok(self.poll_import_end(j, g, last_sequence, last_offset))
            }
            WorkerEvent::Stopped {
                job_id: j,
                generation: g,
                instance_id: i,
            } if (j, g, i) == (job_id, generation, instance_id) => {
                self.transport = None;
                self.active = None;
                self.pending_live = None;
                self.live_archive = None;
                Ok(ImportEvent::Stopped {
                    job_id: j,
                    generation: g,
                })
            }
            WorkerEvent::Failed {
                job_id: Some(j),
                generation: Some(g),
                instance_id: Some(i),
                message,
                code,
            } if (j, g, i) == (job_id, generation, instance_id) => {
                self.handle_worker_failure(j, g, code, message)
            }
            WorkerEvent::Failed { .. } => Err((
                job_id,
                generation,
                "ProtocolMismatch: worker failure lacks current attempt identity".into(),
            )),
            _ => Err((
                job_id,
                generation,
                "StaleResponse: worker event identity mismatch".into(),
            )),
        };
        if let Err((failed_job, failed_generation, message)) = &result
            && let Some(live) = self.live.as_mut()
            && (live.request.job_id, live.request.generation) == (*failed_job, *failed_generation)
        {
            live.stop_capture();
            let _ = live.staging.drain();
            live.failure = Some(message.clone());
            if let Some(transport) = self.transport.as_mut() {
                let _ = transport.try_request(WorkerCommand::Stop { job_id, generation });
            }
            return None;
        }
        if result.as_ref().is_ok_and(|event| {
            !matches!(
                event,
                ImportEvent::End { .. }
                    | ImportEvent::Failed { .. }
                    | ImportEvent::Stopped { .. }
                    | ImportEvent::Published { .. }
            )
        }) && let Some(live) = self
            .live
            .as_mut()
            .filter(|live| (live.request.job_id, live.request.generation) == (job_id, generation))
        {
            live.telemetry_due_after_event = true;
        }
        Some(result)
    }
}
