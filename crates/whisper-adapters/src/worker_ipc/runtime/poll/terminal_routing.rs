#![warn(clippy::too_many_lines, clippy::cognitive_complexity)]

use super::*;
use super::{
    live_windows::{LiveMp3PacketControl, WindowFinishedControl},
    readiness::{
        AdmissionAckControl, AttestationControl, GpuSessionAttestation, PollResult, ReadyControl,
    },
    segments::ResumePrefixControl,
};

type PollOutput = Option<PollResult>;

struct PreparedPollEvent {
    event: WorkerEvent,
    job_id: JobId,
    generation: Generation,
    instance_id: u64,
    backend: BackendKind,
}

enum PollRouteControl {
    Next(WorkerEvent),
    Return(PollOutput),
    Finish(PollResult),
}

enum FinalRouteControl {
    Return(PollOutput),
    Finish(PollResult),
}

impl ImportRuntime {
    pub(super) fn route_collected_event(&mut self, event: WorkerEvent) -> PollOutput {
        let mut output = None;
        let Some(prepared) = self.prepare_poll_event(event, &mut output) else {
            return output;
        };
        let Some(prepared) = self.route_priority_events(prepared, &mut output) else {
            return output;
        };
        let PreparedPollEvent {
            event,
            job_id,
            generation,
            instance_id,
            ..
        } = prepared;
        match self.route_attestation_and_ready(event, job_id, generation, instance_id) {
            PollRouteControl::Return(output) => output,
            PollRouteControl::Finish(result) => {
                self.finish_routed_event(result, job_id, generation)
            }
            PollRouteControl::Next(event) => {
                match self.route_remaining_event(event, job_id, generation, instance_id) {
                    FinalRouteControl::Return(output) => output,
                    FinalRouteControl::Finish(result) => {
                        self.finish_routed_event(result, job_id, generation)
                    }
                }
            }
        }
    }

    fn prepare_poll_event(
        &mut self,
        mut event: WorkerEvent,
        output: &mut PollOutput,
    ) -> Option<PreparedPollEvent> {
        let active = self.active.as_ref()?;
        let (job_id, generation, instance_id, backend, startup_ready) = (
            active.job_id,
            active.generation,
            active.instance_id,
            active.backend,
            active.startup_ready,
        );
        event = bind_unidentified_startup_failure(
            event,
            (job_id, generation, instance_id),
            startup_ready,
        );
        if worker_event_is_stale(&event, (job_id, generation, instance_id)) {
            return None;
        }
        if let WorkerEvent::ImportStarted { .. } = &event {
            if let AdmissionAckControl::Rejected(message) =
                self.poll_import_started_ack(job_id, generation, instance_id)
            {
                *output = Some(Err((job_id, generation, message)));
            }
            return None;
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
            *output = Some(self.handle_worker_failure(
                job_id,
                generation,
                whisper_core::ipc::WorkerErrorCode::ProtocolMismatch,
                "ProtocolMismatch: worker failure lacks current attempt identity".into(),
            ));
            return None;
        }
        Some(PreparedPollEvent {
            event,
            job_id,
            generation,
            instance_id,
            backend,
        })
    }

    fn route_priority_events(
        &mut self,
        prepared: PreparedPollEvent,
        output: &mut PollOutput,
    ) -> Option<PreparedPollEvent> {
        self.observe_worker_event(
            &prepared.event,
            prepared.job_id,
            prepared.generation,
            prepared.backend,
            self.live.is_some(),
        );
        if matches!(
            &prepared.event,
            WorkerEvent::Stopped {
                job_id,
                generation,
                instance_id,
            } if (*job_id, *generation, *instance_id)
                == (prepared.job_id, prepared.generation, prepared.instance_id)
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
            self.deferred_live_stopped = Some(prepared.event);
            return None;
        }
        match self.poll_live_mp3_packet(
            &prepared.event,
            prepared.job_id,
            prepared.generation,
            prepared.instance_id,
        ) {
            LiveMp3PacketControl::Continue => {}
            LiveMp3PacketControl::ReturnNone => return None,
        }
        match self.poll_window_finished(&prepared.event, prepared.job_id, prepared.generation) {
            WindowFinishedControl::Continue => {}
            WindowFinishedControl::ReturnNone => return None,
            WindowFinishedControl::ImmediateError(job_id, generation, message) => {
                *output = Some(Err((job_id, generation, message)));
                return None;
            }
            WindowFinishedControl::Confirm(finished) => {
                *output = Some(self.confirm_live_window(finished));
                return None;
            }
        }
        if let Some(result) = self.poll_stopped_pending_live_recovery(
            &prepared.event,
            prepared.job_id,
            prepared.generation,
        ) {
            *output = Some(result);
            return None;
        }
        if let Some(result) = self.poll_stopped_import_auto_retry(
            &prepared.event,
            prepared.job_id,
            prepared.generation,
        ) {
            *output = Some(result);
            return None;
        }
        if let Some(result) =
            self.poll_stopped_active_live(&prepared.event, prepared.job_id, prepared.generation)
        {
            *output = Some(result);
            return None;
        }
        match self.poll_resume_prefix(&prepared.event, prepared.job_id, prepared.generation) {
            ResumePrefixControl::Continue => Some(prepared),
            ResumePrefixControl::Suppress => None,
            ResumePrefixControl::Reject(failure) => {
                *output = Some(Err(failure));
                None
            }
        }
    }

    fn route_attestation_and_ready(
        &mut self,
        event: WorkerEvent,
        job_id: JobId,
        generation: Generation,
        instance_id: u64,
    ) -> PollRouteControl {
        match event {
            WorkerEvent::GpuSessionAttested {
                job_id: event_job_id,
                generation: event_generation,
                instance_id: event_instance_id,
                candidate,
                dxgi_rank,
                luid,
                pci_id,
                cuda_ordinal,
                ggml_ordinal,
            } if (event_job_id, event_generation, event_instance_id)
                == (job_id, generation, instance_id) =>
            {
                match self.poll_gpu_session_attestation(GpuSessionAttestation {
                    candidate,
                    dxgi_rank,
                    luid,
                    pci_id,
                    cuda_ordinal,
                    ggml_ordinal,
                }) {
                    AttestationControl::Accepted => PollRouteControl::Return(None),
                    AttestationControl::Mismatch => {
                        PollRouteControl::Finish(self.handle_worker_failure(
                            job_id,
                            generation,
                            whisper_core::ipc::WorkerErrorCode::BackendMismatch,
                            "BackendMismatch: worker attested a different GPU candidate".into(),
                        ))
                    }
                }
            }
            WorkerEvent::Ready {
                job_id: event_job_id,
                generation: event_generation,
                instance_id: event_instance_id,
                backend,
            } if (event_job_id, event_generation, event_instance_id)
                == (job_id, generation, instance_id)
                && self
                    .active
                    .as_ref()
                    .is_some_and(|active| active.backend == backend) =>
            {
                match self.poll_ready_backend(
                    event_job_id,
                    event_generation,
                    event_instance_id,
                    backend,
                ) {
                    ReadyControl::ReturnImmediately(result) => {
                        PollRouteControl::Return(Some(result))
                    }
                    ReadyControl::Finish(result) => PollRouteControl::Finish(result),
                }
            }
            WorkerEvent::Ready { .. } => PollRouteControl::Finish(self.handle_worker_failure(
                job_id,
                generation,
                whisper_core::ipc::WorkerErrorCode::BackendMismatch,
                "BackendMismatch: worker attestation differs from the selected backend".into(),
            )),
            event => PollRouteControl::Next(event),
        }
    }

    fn route_remaining_event(
        &mut self,
        event: WorkerEvent,
        job_id: JobId,
        generation: Generation,
        instance_id: u64,
    ) -> FinalRouteControl {
        match event {
            WorkerEvent::Progress {
                job_id: event_job_id,
                generation: event_generation,
                instance_id: event_instance_id,
                completed_samples,
                total_samples,
            } if (event_job_id, event_generation, event_instance_id)
                == (job_id, generation, instance_id) =>
            {
                FinalRouteControl::Finish(Ok(ImportEvent::Progress {
                    job_id: event_job_id,
                    generation: event_generation,
                    completed_samples,
                    total_samples,
                }))
            }
            WorkerEvent::Segment(dto)
                if (dto.job_id, dto.generation, dto.instance_id)
                    == (job_id, generation, instance_id) =>
            {
                match self.poll_import_segment(dto, job_id, generation) {
                    Ok(event) => FinalRouteControl::Finish(Ok(event)),
                    Err(failure) => FinalRouteControl::Return(Some(Err(failure))),
                }
            }
            WorkerEvent::End {
                job_id: event_job_id,
                generation: event_generation,
                instance_id: event_instance_id,
                last_sequence,
                last_offset,
            } if (event_job_id, event_generation, event_instance_id)
                == (job_id, generation, instance_id) =>
            {
                FinalRouteControl::Finish(Ok(self.poll_import_end(
                    job_id,
                    generation,
                    last_sequence,
                    last_offset,
                )))
            }
            WorkerEvent::Stopped {
                job_id: event_job_id,
                generation: event_generation,
                instance_id: event_instance_id,
            } if (event_job_id, event_generation, event_instance_id)
                == (job_id, generation, instance_id) =>
            {
                self.transport = None;
                self.active = None;
                self.pending_live = None;
                self.live_archive = None;
                FinalRouteControl::Finish(Ok(ImportEvent::Stopped {
                    job_id: event_job_id,
                    generation: event_generation,
                }))
            }
            WorkerEvent::Failed {
                job_id: Some(event_job_id),
                generation: Some(event_generation),
                instance_id: Some(event_instance_id),
                message,
                code,
            } if (event_job_id, event_generation, event_instance_id)
                == (job_id, generation, instance_id) =>
            {
                FinalRouteControl::Finish(
                    self.handle_worker_failure(job_id, generation, code, message),
                )
            }
            WorkerEvent::Failed { .. } => FinalRouteControl::Finish(Err((
                job_id,
                generation,
                "ProtocolMismatch: worker failure lacks current attempt identity".into(),
            ))),
            _ => FinalRouteControl::Finish(Err((
                job_id,
                generation,
                "StaleResponse: worker event identity mismatch".into(),
            ))),
        }
    }

    fn finish_routed_event(
        &mut self,
        result: PollResult,
        job_id: JobId,
        generation: Generation,
    ) -> PollOutput {
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
