use super::*;
use live_windows::{LiveMp3PacketControl, WindowFinishedControl};
use readiness::{AdmissionAckControl, AttestationControl, GpuSessionAttestation, ReadyControl};
use segments::ResumePrefixControl;

mod capture;
mod live_windows;
mod observations;
mod parakeet;
mod readiness;
mod segments;
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
        if let Some(event) = self.poll_unified_sessions() {
            if let Err((job_id, generation, _)) = &event {
                self.release_unified_terminal_state(*job_id, *generation);
            }
            return Some(event);
        }
        if let Some(event) = self.poll_unified_document() {
            if let Err((job_id, generation, _)) = &event {
                self.release_unified_terminal_state(*job_id, *generation);
            }
            return Some(event);
        }
        if let Some(event) = self.parakeet_pending_events.pop_front() {
            return Some(Ok(event));
        }
        if let Some(test) = self.microphone_test.as_mut() {
            let identity = (test.job_id, test.generation);
            match test.pump() {
                Ok(Some(event)) => return Some(Ok(event)),
                Ok(None) => {}
                Err(error) => {
                    if let Some(mut test) = self.microphone_test.take() {
                        test.probe.invalidate();
                        test.capture.stop();
                    }
                    return Some(Err((identity.0, identity.1, error)));
                }
            }
        }
        if self
            .idle_worker
            .as_ref()
            .is_some_and(|idle| idle.expires_at <= Instant::now())
        {
            self.idle_worker = None;
        }
        if let Some(direct) = self.direct_live.as_mut() {
            let id = direct.identity();
            let result = direct.poll().map_err(|error| direct.fail(error));
            let done = direct.done() || result.is_err();
            if done {
                if direct.done() && result.is_ok() {
                    let (compute, model_path, model_sha256) = direct.cache_identity();
                    let model_path = model_path.to_owned();
                    if let Some(transport) = direct.take_idle_worker() {
                        self.idle_worker = Some(IdleWorker {
                            transport,
                            compute,
                            model_path,
                            model_sha256,
                            expires_at: Instant::now() + WORKER_IDLE_RETENTION,
                        });
                    }
                }
                self.direct_live = None;
            }
            match result {
                Ok(Some(event)) => return Some(Ok(event)),
                Ok(None) => {}
                Err(error) => return Some(Err((id.0, id.1, error))),
            }
        }
        if let Some(live) = self.parakeet_live.as_mut() {
            let id = live.identity();
            match live.poll() {
                Ok(Some(event)) => return Some(Ok(event)),
                Ok(None) => {}
                Err(error) => {
                    let event = live.stop_after_capture_error(error);
                    let event = attach_automatic_final_import(live, event);
                    self.parakeet_pending_events.push_back(event);
                    self.parakeet_live = None;
                    if let Some(event) = self.parakeet_pending_events.pop_front() {
                        return Some(Ok(event));
                    }
                    return Some(Err((id.0, id.1, "Capture Parakeet interrompue".into())));
                }
            }
        }
        if let Some(event) = self.pending_recovery_counters.take() {
            return Some(Ok(event));
        }
        if self.live.as_ref().is_some_and(|live| live.worker_ready)
            && let Some(event) = self.pending_attempt_reset.take()
        {
            return Some(Ok(event));
        }
        if let Some(event) = self.take_observation_notice() {
            return Some(Ok(event));
        }
        if self
            .live
            .as_ref()
            .is_some_and(|live| live.telemetry_due_after_event)
        {
            if let Some(live) = self.live.as_mut() {
                live.telemetry_due_after_event = false;
            }
            if let Some(event) = self.take_legacy_live_telemetry() {
                return Some(Ok(event));
            }
        }
        let capture_may_publish = live_capture_poll_allowed(
            self.live_capture_events_since_worker_poll,
            self.transport.is_some(),
        );
        if capture_may_publish && let Some(event) = self.poll_live_capture() {
            if let Ok(ImportEvent::LiveCounters {
                captured_samples,
                audio_durable_samples,
                ..
            }) = event.as_ref()
                && let Some(progress) = self.progress.as_mut()
            {
                let now = std::time::Instant::now();
                progress.observe(ProgressPhase::Capturing, *captured_samples, now);
                progress.observe(ProgressPhase::Draining, *audio_durable_samples, now);
            }
            if let Some(live) = self.live.as_mut() {
                live.telemetry_due_after_event = true;
            }
            self.live_capture_events_since_worker_poll =
                self.live_capture_events_since_worker_poll.saturating_add(1);
            return Some(event);
        }
        if self.deferred_live_window_finished.is_some() && self.live_pending_persistence > 0 {
            return None;
        }
        if self
            .parakeet_vad
            .as_ref()
            .is_some_and(|run| run.decoder.is_some())
            && let Err(failure) = self.advance_parakeet_vad_decode()
        {
            if let Some(worker) = self.parakeet.as_mut() {
                let _ = worker.terminate();
            }
            self.parakeet = None;
            self.parakeet_vad = None;
            self.parakeet_events.clear();
            self.active = None;
            return Some(Err(failure));
        }
        if self
            .parakeet_vad
            .as_ref()
            .is_some_and(|run| run.decoder.is_none() && run.worker_ready && !run.ready_event_sent)
        {
            if let Err(failure) = self.begin_next_parakeet_vad_window() {
                if let Some(worker) = self.parakeet.as_mut() {
                    let _ = worker.terminate();
                }
                self.parakeet = None;
                self.parakeet_vad = None;
                self.parakeet_events.clear();
                self.active = None;
                return Some(Err(failure));
            }
            let active = self.active.as_ref()?;
            self.parakeet_events.push_back(WorkerEvent::Ready {
                job_id: active.job_id,
                generation: active.generation,
                instance_id: active.instance_id,
                backend: BackendKind::Cpu,
            });
            if let Some(run) = self.parakeet_vad.as_mut() {
                run.ready_event_sent = true;
            }
        }
        let mut event = if self.parakeet.is_some() || !self.parakeet_events.is_empty() {
            match self.poll_parakeet_worker() {
                Some(Ok(event)) => event,
                Some(Err(error)) => {
                    let active = self.active.as_ref()?;
                    let failure = (active.job_id, active.generation, format!("{error:?}"));
                    self.parakeet = None;
                    self.parakeet_stopped_ack = false;
                    self.parakeet_events.clear();
                    self.active = None;
                    return Some(Err(failure));
                }
                None => return None,
            }
        } else if self.deferred_live_stopped.is_some() {
            if self.live_pending_persistence > 0
                || self
                    .transport
                    .as_mut()
                    .and_then(|transport| transport.try_wait().ok().flatten())
                    .is_none()
            {
                return None;
            }
            take_ready_live_stop(
                self.live_pending_persistence,
                &mut self.deferred_live_stopped,
            )?
        } else {
            let transport = self.transport.as_mut()?;
            let received = transport.receive();
            self.live_capture_events_since_worker_poll = 0;
            match received {
                Ok(Some(event)) => event,
                Ok(None) => {
                    let _ = transport.try_wait();
                    return self.take_legacy_live_telemetry().map(Ok);
                }
                Err(error) => {
                    let active = self.active.as_ref()?;
                    let (job_id, generation, instance_id) =
                        (active.job_id, active.generation, active.instance_id);
                    let recovery_session = self.live.is_some()
                        || (active.backend == BackendKind::Cuda
                            && (self.pending_live.is_some()
                                || active.import_request.as_ref().is_some_and(|request| {
                                    request.config.compute == whisper_core::ComputeChoice::Auto
                                })))
                        || active.recovery != LiveRecoveryState::None
                        || self
                            .pending_live
                            .as_ref()
                            .is_some_and(|pending| pending.recovery != LiveRecoveryState::None);
                    if recovery_session {
                        self.deferred_live_stopped = Some(WorkerEvent::Stopped {
                            job_id,
                            generation,
                            instance_id,
                        });
                        return Some(self.handle_worker_failure(
                            job_id,
                            generation,
                            whisper_core::ipc::WorkerErrorCode::WorkerExited,
                            format!("WorkerExited without End: {error:?}"),
                        ));
                    }
                    let failure = Err((
                        job_id,
                        generation,
                        format!("WorkerExited without End: {error:?}"),
                    ));
                    if let Some(live) = self.live.as_mut() {
                        live.stop_capture();
                        let _ = live.staging.drain();
                    }
                    self.live = None;
                    self.live_archive = None;
                    self.transport = None;
                    self.active = None;
                    return Some(failure);
                }
            }
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
        if matches!(event, WorkerEvent::Stopped { .. })
            && self.live.is_none()
            && self
                .pending_live
                .as_ref()
                .is_some_and(|pending| pending.recovery != LiveRecoveryState::None)
        {
            let recovery = self.pending_live.as_ref().map(|pending| pending.recovery);
            if let Some(pending) = self.pending_live.as_mut()
                && let Err(error) = pending.live_archive.retire_attempt(
                    self.active
                        .as_ref()
                        .map_or(0, |active| active.attempt_number),
                )
            {
                self.pending_live = None;
                self.active = None;
                self.transport = None;
                return Some(Err((job_id, generation, error)));
            }
            self.active = None;
            self.transport = None;
            if self.publication_gate.is_cancelled() {
                self.pending_live = None;
                return Some(Ok(ImportEvent::Stopped { job_id, generation }));
            }
            if recovery == Some(LiveRecoveryState::AwaitingChoicePending) {
                if let Some(pending) = self.pending_live.as_mut() {
                    pending.recovery = LiveRecoveryState::AwaitingChoice;
                }
                return Some(Ok(ImportEvent::AwaitingCpuChoice { job_id, generation }));
            }
            return Some(
                match self.start_cpu_pending_live_attempt(job_id, generation) {
                    Ok(event) => Ok(event),
                    Err(error) => Err(error),
                },
            );
        }
        if matches!(event, WorkerEvent::Stopped { .. })
            && self.live.is_none()
            && self.active.as_ref().is_some_and(|active| {
                active.recovery == LiveRecoveryState::AutoRetryPending
                    && active.import_request.is_some()
            })
        {
            let request = self
                .active
                .as_ref()
                .and_then(|active| active.import_request.clone());
            self.active = None;
            self.transport = None;
            if self.publication_gate.is_cancelled() {
                return Some(Ok(ImportEvent::Stopped { job_id, generation }));
            }
            return Some(
                match request.map(|request| self.start_worker_with_cpu_override(request, true)) {
                    Some(Ok(())) => Ok(ImportEvent::LiveAttemptStarting { job_id, generation }),
                    Some(Err(error)) => Err(error),
                    None => Err((
                        job_id,
                        generation,
                        "ProtocolMismatch: retry request unavailable".into(),
                    )),
                },
            );
        }
        if matches!(event, WorkerEvent::Stopped { .. }) && self.live.is_some() {
            if let Some(reason) = self.live.as_ref().and_then(|live| live.failure.clone()) {
                if let Some(live) = self.live.as_mut() {
                    live.stop_capture();
                    let _ = live.staging.drain();
                }
                self.live = None;
                self.live_archive = None;
                self.transport = None;
                self.active = None;
                return Some(Err((job_id, generation, reason)));
            }
            if let Err(error) = crate::archive::campaign_gate(
                "pcm.before_final_drain",
                job_id.0,
                generation.get(),
                0,
            ) {
                return Some(Err((job_id, generation, error)));
            }
            let final_drain = self.live.as_mut().map(|live| live.staging.drain());
            if let Some(result) = &final_drain
                && let Err(error) = crate::archive::record_campaign_operation_result(
                    "pcm.before_final_drain",
                    job_id.0,
                    generation.get(),
                    0,
                    result,
                    None,
                )
            {
                return Some(Err((job_id, generation, error)));
            }
            if let Some(Err(error)) = final_drain {
                if let Some(live) = self.live.as_mut() {
                    live.stop_capture();
                }
                self.live = None;
                self.live_archive = None;
                self.transport = None;
                self.active = None;
                return Some(Err((
                    job_id,
                    generation,
                    format!("StorageUnavailable: final audio sync: {error}"),
                )));
            }
            if let Err(error) = crate::archive::campaign_gate(
                "pcm.after_final_drain",
                job_id.0,
                generation.get(),
                0,
            ) {
                return Some(Err((job_id, generation, error)));
            }
            let (durable, confirmed, pcm_sha256) =
                self.live.as_ref().map_or((0, 0, [0; 32]), |live| {
                    (
                        live.staging.durable_samples(),
                        live.confirmed_fragment_end,
                        live.staging.checksum(),
                    )
                });
            let recovery = self.live.as_ref().map(|live| live.recovery);
            if matches!(
                recovery,
                Some(
                    LiveRecoveryState::AutoRetryPending | LiveRecoveryState::AwaitingChoicePending
                )
            ) {
                let barrier_valid = self.live.as_ref().is_some_and(|live| {
                    live.barrier_m.is_some_and(|watermark| {
                        watermark <= live.inference_sequence.saturating_sub(1)
                    })
                });
                let coverage_valid = self
                    .live_archive
                    .as_ref()
                    .is_some_and(|archive| archive.confirmed_samples() <= durable);
                if !barrier_valid || !coverage_valid {
                    self.live = None;
                    self.live_archive = None;
                    self.transport = None;
                    self.active = None;
                    return Some(Err((
                        job_id,
                        generation,
                        "StorageCorrupt: retry barrier did not settle a valid confirmed prefix"
                            .into(),
                    )));
                }
                let attempt_number = self
                    .active
                    .as_ref()
                    .map_or(0, |active| active.attempt_number);
                if let Some(archive) = self.live_archive.as_mut()
                    && let Err(error) = archive.retire_attempt(attempt_number)
                {
                    self.live = None;
                    self.live_archive = None;
                    self.transport = None;
                    self.active = None;
                    return Some(Err((job_id, generation, error)));
                }
                self.transport = None;
                self.active = None;
                if self.publication_gate.is_cancelled() {
                    if let Some(live) = self.live.as_mut() {
                        live.stop_capture();
                    }
                    self.live = None;
                    self.live_archive = None;
                    return Some(Ok(ImportEvent::Stopped { job_id, generation }));
                }
                if recovery == Some(LiveRecoveryState::AwaitingChoicePending) {
                    if let Some(live) = self.live.as_mut() {
                        live.recovery = LiveRecoveryState::AwaitingChoice;
                        live.inference_admission_closed = true;
                    }
                    return Some(Ok(ImportEvent::AwaitingCpuChoice { job_id, generation }));
                }
                let stop_was_latched = self.live.as_ref().is_some_and(|live| live.stop_requested)
                    || self.publication_gate.is_cancelled();
                if stop_was_latched {
                    if let Some(live) = self.live.as_mut() {
                        live.stop_capture();
                    }
                    self.live = None;
                    self.live_archive = None;
                    return Some(Ok(ImportEvent::Stopped { job_id, generation }));
                }
                return Some(self.start_cpu_live_attempt(job_id, generation));
            }
            let publish_result = self
                .live_archive
                .as_mut()
                .map(|archive| archive.publish(durable, confirmed, pcm_sha256));
            if let Some(Err(error)) = publish_result {
                if let Some(live) = self.live.as_mut() {
                    live.stop_capture();
                }
                self.live = None;
                self.live_archive = None;
                self.transport = None;
                self.active = None;
                return Some(Err((job_id, generation, error)));
            }
            self.live = None;
            self.live_archive = None;
            self.transport = None;
            self.active = None;
            return Some(Ok(ImportEvent::Published { job_id, generation }));
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
