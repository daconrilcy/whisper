use super::*;

mod capture;
mod parakeet;
pub(in crate::worker_ipc) use parakeet::ParakeetVadRun;
#[cfg(test)]
pub(in crate::worker_ipc) use parakeet::{
    ParakeetStoppedExit, TDT_FRAME_OVERLAP_SAMPLES, append_single_pass_parakeet_word_group,
    append_vad_parakeet_word_group, parakeet_stopped_exit, parakeet_vad_segments,
    vad_word_overlap_within_one_tdt_frame, validate_single_pass_parakeet_words,
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
        if let Some(progress) = self.progress.as_mut()
            && let Some((phase, idle_ms)) = progress.take_warning(std::time::Instant::now())
            && let Some(active) = self.active.as_ref()
        {
            if let Some(diagnostics) = &self.diagnostics {
                diagnostics.record(DiagnosticRecord::new(
                    active.job_id.0,
                    active.generation.get(),
                    phase.into(),
                    DiagnosticCode::ProgressWarning,
                    Some(match active.backend {
                        BackendKind::Cpu => "cpu",
                        BackendKind::Cuda => "cuda",
                    }),
                    None,
                ));
            }
            return Some(Ok(ImportEvent::Diagnostic {
                job_id: active.job_id,
                generation: active.generation,
                notice: DiagnosticNotice::ProgressStalled {
                    phase: phase.into(),
                    idle_ms,
                },
            }));
        }
        let diagnostics_degraded = self.diagnostic_start_failed
            || self
                .diagnostics
                .as_ref()
                .is_some_and(DiagnosticSink::is_degraded);
        if !self.diagnostic_degraded_reported
            && diagnostics_degraded
            && let Some(active) = self.active.as_ref()
        {
            self.diagnostic_degraded_reported = true;
            if let Some(diagnostics) = self.diagnostics.as_ref() {
                diagnostics.record(DiagnosticRecord::new(
                    active.job_id.0,
                    active.generation.get(),
                    DiagnosticPhase::Finalizing,
                    DiagnosticCode::LogDegraded,
                    Some(match active.backend {
                        BackendKind::Cpu => "cpu",
                        BackendKind::Cuda => "cuda",
                    }),
                    None,
                ));
            }
            return Some(Ok(ImportEvent::Diagnostic {
                job_id: active.job_id,
                generation: active.generation,
                notice: DiagnosticNotice::LoggingUnavailable,
            }));
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
            let Some(active) = self.active.as_mut().filter(|active| {
                active.job_id == job_id
                    && active.generation == generation
                    && active.instance_id == instance_id
                    && active.backend == BackendKind::Cuda
                    && active.import_request.as_ref().is_some_and(|request| {
                        request.config.compute == whisper_core::ComputeChoice::Auto
                    })
                    && !active.startup_ready
            }) else {
                return Some(Err((
                    job_id,
                    generation,
                    "ProtocolMismatch: unexpected Auto import admission acknowledgement".into(),
                )));
            };
            active.startup_ready = true;
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
            return Some(self.handle_worker_failure(
                job_id,
                generation,
                whisper_core::ipc::WorkerErrorCode::ProtocolMismatch,
                "ProtocolMismatch: worker failure lacks current attempt identity".into(),
            ));
        }
        if let Some(diagnostics) = &self.diagnostics {
            let backend = Some(match active.backend {
                BackendKind::Cpu => "cpu",
                BackendKind::Cuda => "cuda",
            });
            let record = match &event {
                WorkerEvent::Ready { .. } => Some(DiagnosticRecord::new(
                    job_id.0,
                    generation.get(),
                    DiagnosticPhase::Preparing,
                    DiagnosticCode::BackendReady,
                    backend,
                    None,
                )),
                WorkerEvent::Progress {
                    completed_samples,
                    total_samples,
                    ..
                } => Some(DiagnosticRecord::new(
                    job_id.0,
                    generation.get(),
                    if self.live.is_some() {
                        DiagnosticPhase::Inference
                    } else {
                        DiagnosticPhase::Import
                    },
                    DiagnosticCode::Progress,
                    backend,
                    Some((*completed_samples, *total_samples)),
                )),
                WorkerEvent::Failed { .. } => Some(DiagnosticRecord::new(
                    job_id.0,
                    generation.get(),
                    DiagnosticPhase::Finalizing,
                    DiagnosticCode::WorkerFailed,
                    backend,
                    None,
                )),
                WorkerEvent::Stopped { .. } => Some(DiagnosticRecord::new(
                    job_id.0,
                    generation.get(),
                    DiagnosticPhase::Cancelling,
                    DiagnosticCode::WorkerStopped,
                    backend,
                    None,
                )),
                _ => None,
            };
            if let Some(record) = record {
                diagnostics.record(record);
            }
        }
        if let Some(progress) = self.progress.as_mut() {
            match &event {
                WorkerEvent::Ready { .. } => {
                    let now = std::time::Instant::now();
                    progress.close(ProgressPhase::Preparing);
                    progress.open(ProgressPhase::Inference, now);
                    if self.live.is_some() || self.pending_live.is_some() {
                        progress.open(ProgressPhase::Capturing, now);
                        progress.open(ProgressPhase::Draining, now);
                    } else {
                        progress.open(ProgressPhase::Import, now);
                    }
                }
                WorkerEvent::Progress {
                    completed_samples, ..
                } => {
                    let now = std::time::Instant::now();
                    progress.observe(ProgressPhase::Inference, *completed_samples, now);
                    progress.observe(ProgressPhase::Import, *completed_samples, now);
                }
                WorkerEvent::Segment(dto) => progress.observe(
                    ProgressPhase::Inference,
                    dto.segment_id.0,
                    std::time::Instant::now(),
                ),
                WorkerEvent::End { .. } => {
                    let now = std::time::Instant::now();
                    progress.close(ProgressPhase::Inference);
                    progress.close(ProgressPhase::Import);
                    progress.open(ProgressPhase::Finalizing, now);
                }
                WorkerEvent::Stopped { .. } | WorkerEvent::Failed { .. } => progress.close_all(),
                _ => {}
            }
        }
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
        if let WorkerEvent::LiveMp3Packet {
            job_id: packet_job,
            generation: packet_generation,
            instance_id: packet_instance,
            bytes,
            ..
        } = &event
        {
            if (*packet_job, *packet_generation, *packet_instance)
                != (job_id, generation, instance_id)
            {
                return None;
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
                    return None;
                }
                return None;
            }
        }
        if let WorkerEvent::WindowFinished {
            sequence,
            range,
            last_segment_sequence,
            ..
        } = &event
        {
            if self.live.is_none() {
                return Some(Err((
                    job_id,
                    generation,
                    "ProtocolMismatch: WindowFinished outside a live pass".into(),
                )));
            }
            let Some(live) = self.live.as_ref() else {
                unreachable!()
            };
            // Confirmations wholly inside T were already durable before this attempt.
            // Replay only rebuilds the MP3 encoder and must not create duplicate coverage.
            if range.end_sample <= live.replay_until {
                return None;
            }
            let Some(window_sequence) = map_attempt_sequence(
                live.window_sequence_base,
                live.replay_windows_to_skip,
                *sequence,
            ) else {
                return Some(Err((
                    job_id,
                    generation,
                    "ProtocolMismatch: live window sequence cannot be mapped to durable prefix"
                        .into(),
                )));
            };
            let last_segment_sequence = if *last_segment_sequence == 0 {
                live.segment_sequence_base.checked_sub(1)
            } else {
                map_attempt_sequence(live.segment_sequence_base, 0, *last_segment_sequence)
            };
            let Some(last_segment_sequence) = last_segment_sequence else {
                return Some(Err((
                    job_id,
                    generation,
                    "ProtocolMismatch: live segment sequence cannot be mapped to durable prefix"
                        .into(),
                )));
            };
            let finished = FinishedLiveWindow {
                sequence: window_sequence,
                range: *range,
                last_segment_sequence,
            };
            if self.live_pending_persistence > 0 {
                self.deferred_live_window_finished = Some(finished);
                return None;
            }
            return Some(self.confirm_live_window(finished));
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
        if let WorkerEvent::Segment(dto) = &event
            && let Some(expected) = self.resume_prefix.pop_front()
        {
            if dto.segment_id.0 != expected.sequence
                || dto.range != expected.range
                || dto.text != expected.text
            {
                self.transport = None;
                self.active = None;
                self.resume_prefix.clear();
                return Some(Err((
                    job_id,
                    generation,
                    "ResumeMismatch: recomputed segment differs from the confirmed prefix".into(),
                )));
            }
            return None;
        }
        if matches!(event, WorkerEvent::End { .. }) && !self.resume_prefix.is_empty() {
            self.transport = None;
            self.active = None;
            self.resume_prefix.clear();
            return Some(Err((
                job_id,
                generation,
                "ResumeMismatch: worker ended before reproducing the confirmed prefix".into(),
            )));
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
                let matches_candidate = self.active.as_ref().is_some_and(|active| {
                    active.gpu_candidate.as_ref().is_some_and(|expected| {
                        expected.pnp_instance_id == candidate
                            && expected.dxgi_rank == dxgi_rank
                            && expected.luid == luid
                            && expected.pci_id.eq_ignore_ascii_case(&pci_id)
                            && expected.cuda_ordinal == cuda_ordinal
                            && expected.ggml_ordinal == ggml_ordinal
                    })
                });
                if matches_candidate {
                    if let Some(active) = self.active.as_mut() {
                        active.gpu_attested = true;
                    }
                    return None;
                } else {
                    self.handle_worker_failure(
                        j,
                        g,
                        whisper_core::ipc::WorkerErrorCode::BackendMismatch,
                        "BackendMismatch: worker attested a different GPU candidate".into(),
                    )
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
                if backend == BackendKind::Cuda
                    && self.active.as_ref().is_some_and(|active| {
                        active.gpu_candidate.is_some() && !active.gpu_attested
                    })
                {
                    return Some(self.handle_worker_failure(
                        j,
                        g,
                        whisper_core::ipc::WorkerErrorCode::BackendMismatch,
                        "BackendMismatch: GPU Ready arrived before candidate attestation".into(),
                    ));
                }
                // The generic Turbo GPU worker pauses at Ready and requires a
                // BeginImport command. Parakeet's separate IPC has no such
                // command: poll_parakeet_worker already sent Transcribe with
                // the correlated attempt before returning this Ready event.
                let auto_gpu_import = backend == BackendKind::Cuda
                    && self.parakeet.is_none()
                    && self
                        .active
                        .as_ref()
                        .and_then(|active| active.import_request.as_ref())
                        .is_some_and(|request| {
                            request.config.compute == whisper_core::ComputeChoice::Auto
                        });
                if !auto_gpu_import && let Some(active) = self.active.as_mut() {
                    active.startup_ready = true;
                }
                if auto_gpu_import && !self.publication_gate.is_cancelled() {
                    let begin = WorkerCommand::BeginImport {
                        job_id: j,
                        generation: g,
                        instance_id: i,
                    };
                    if let Err(error) = self
                        .transport
                        .as_mut()
                        .expect("ready worker")
                        .request(begin)
                    {
                        return Some(self.handle_worker_failure(
                            j,
                            g,
                            whisper_core::ipc::WorkerErrorCode::WorkerExited,
                            format!("WorkerExited before BeginImport admission: {error:?}"),
                        ));
                    }
                }
                if let Some(live) = self
                    .live
                    .as_mut()
                    .filter(|live| live.request.job_id == j && live.request.generation == g)
                {
                    live.worker_ready = true;
                }
                if self.pending_live.is_some()
                    && !self.publication_gate.is_cancelled()
                    && let Err(failure) = self.activate_live_after_ready()
                {
                    if let Some(transport) = self.transport.as_mut() {
                        let _ = transport.try_request(WorkerCommand::Stop {
                            job_id: j,
                            generation: g,
                        });
                    }
                    Err(failure)
                } else {
                    Ok(ImportEvent::Ready {
                        job_id: j,
                        generation: g,
                        backend: match backend {
                            BackendKind::Cpu => "CPU",
                            BackendKind::Cuda => "CUDA",
                        }
                        .to_owned()
                            + &self
                                .auto_fallback_reason
                                .as_ref()
                                .filter(|_| backend == BackendKind::Cpu)
                                .map(|reason| {
                                    format!(
                                        " — GPU non utilisable ({reason}) ; CPU prend le relais"
                                    )
                                })
                                .unwrap_or_default(),
                    })
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
                let sequence_base = self
                    .live
                    .as_ref()
                    .filter(|live| {
                        live.request.job_id == job_id && live.request.generation == generation
                    })
                    .map_or(1, |live| live.segment_sequence_base);
                let Some(segment_sequence) =
                    map_attempt_sequence(sequence_base, 0, dto.segment_id.0)
                else {
                    return Some(Err((
                        job_id,
                        generation,
                        "ProtocolMismatch: live segment sequence exhausted".into(),
                    )));
                };
                let segment = WorkerSegment {
                    job_id: dto.job_id,
                    generation: dto.generation,
                    instance_id: dto.instance_id,
                    segment_id: whisper_core::SegmentId(segment_sequence),
                    range: dto.range,
                    text: dto.text,
                };
                if let Some(live) = self.live.as_ref().filter(|live| {
                    live.request.job_id == job_id && live.request.generation == generation
                }) {
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
            WorkerEvent::End {
                job_id: j,
                generation: g,
                instance_id: i,
                last_sequence,
                last_offset,
            } if (j, g, i) == (job_id, generation, instance_id) => {
                if let (Some(active), Some(transport)) =
                    (self.active.as_ref(), self.transport.take())
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
                Ok(ImportEvent::End {
                    job_id: j,
                    generation: g,
                    last_sequence,
                    last_offset,
                })
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
