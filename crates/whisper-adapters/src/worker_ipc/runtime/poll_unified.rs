use super::*;

pub(in crate::worker_ipc) struct NativeAutoCpuRetry {
    pub(in crate::worker_ipc) call: whisper_core::parakeet_scheduler::ScheduledCall,
    pub(in crate::worker_ipc) failed_manual_gpu: Vec<FailedManualGpuPeer>,
}

pub(in crate::worker_ipc) struct FailedManualGpuPeer {
    pub(in crate::worker_ipc) session: whisper_core::parakeet_scheduler::SchedulerSession,
    pub(in crate::worker_ipc) message: String,
}

pub(in crate::worker_ipc) fn manual_gpu_failure_event(peer: FailedManualGpuPeer) -> ImportEvent {
    ImportEvent::Failed {
        job_id: peer.session.job_id,
        generation: peer.session.generation,
        message: peer.message,
    }
}

pub(in crate::worker_ipc) struct NativeRetryState<'a> {
    pub(in crate::worker_ipc) scheduler:
        &'a mut whisper_core::parakeet_scheduler::ParakeetScheduler,
    pub(in crate::worker_ipc) sessions:
        &'a mut HashMap<whisper_core::parakeet_scheduler::SchedulerSession, UnifiedSessionState>,
    pub(in crate::worker_ipc) archives:
        &'a mut HashMap<whisper_core::parakeet_scheduler::SchedulerSession, ArchiveStore>,
    pub(in crate::worker_ipc) executor_route: &'a mut Option<NativeUnifiedRoute>,
    pub(in crate::worker_ipc) executor: &'a mut UnifiedInferenceExecutor,
}

pub(in crate::worker_ipc) fn retry_native_gpu_startup_on_cpu(
    retry_state: NativeRetryState<'_>,
    current_state: &mut UnifiedSessionState,
    call: whisper_core::parakeet_scheduler::ScheduledCall,
    cause: &str,
    mut make_cpu_route: impl FnMut(&ImportRequest, &str) -> Result<NativeUnifiedRoute, String>,
    make_cpu_worker: impl FnOnce(
        NativeUnifiedRoute,
    ) -> Box<dyn crate::parakeet_pipeline::WindowTranscriber>,
) -> Result<Option<NativeAutoCpuRetry>, String> {
    let NativeRetryState {
        scheduler,
        sessions,
        archives,
        executor_route,
        executor,
    } = retry_state;
    let auto_gpu_unstarted = |state: &UnifiedSessionState| {
        state.native_route.as_ref().is_some_and(|route| {
            state.request.config.compute == route.requested_compute
                && can_retry_native_auto_on_cpu(
                    route.requested_compute,
                    route.compute,
                    state.backend_ready_reported,
                    state.pending_call.is_some(),
                )
        })
    };
    if !auto_gpu_unstarted(current_state) {
        return Ok(None);
    }

    let mut auto_peers = Vec::new();
    let mut manual_gpu_peers = Vec::new();
    for (session, state) in sessions.iter() {
        let same_worker = current_state
            .native_route
            .as_ref()
            .zip(state.native_route.as_ref())
            .is_some_and(|(current, peer)| current.same_worker(peer));
        if same_worker && auto_gpu_unstarted(state) {
            auto_peers.push(*session);
            continue;
        }

        let manual_gpu_unstarted = same_worker
            && state.request.config.compute == ComputeChoice::Gpu
            && state.native_route.as_ref().is_some_and(|route| {
                route.requested_compute == ComputeChoice::Gpu
                    && route.compute == ComputeChoice::Gpu
                    && !state.backend_ready_reported
                    && state.pending_call.is_none()
                    && state.pending_prepared.is_none()
                    && state.pending_window.is_none()
            });
        if manual_gpu_unstarted {
            manual_gpu_peers.push(*session);
            continue;
        }
        return Ok(None);
    }

    let fallback_reason = format!("GPU pre-audio startup failed: {cause}");
    let current_route = make_cpu_route(&current_state.request, &fallback_reason)
        .map_err(|error| format!("{fallback_reason}; CPU fallback route unavailable: {error}"))?;
    let mut peer_routes = HashMap::new();
    for session in &auto_peers {
        let state = sessions
            .get(session)
            .expect("auto GPU peer remains in the session table");
        let route = make_cpu_route(&state.request, &fallback_reason).map_err(|error| {
            format!(
                "{fallback_reason}; CPU fallback route unavailable for {:?}/{:?}: {error}",
                session.job_id, session.generation
            )
        })?;
        peer_routes.insert(*session, route);
    }

    if executor.stop_and_reap().is_err() {
        return Ok(None);
    }
    let mut failed_manual_gpu = Vec::new();
    for session in &manual_gpu_peers {
        let state = sessions
            .get_mut(session)
            .expect("manual GPU peer remains in the session table");
        let mut message = format!(
            "GPU pre-audio startup failed: {cause}; manual GPU sessions do not fall back to CPU"
        );
        if state.source.is_capture() && !state.source_closed {
            match state.source.stop_capture() {
                Ok(Some((total_samples, acknowledgements))) => {
                    let durable_samples = state
                        .durable_samples
                        .max(
                            acknowledgements
                                .iter()
                                .map(|ack| ack.flushed_samples)
                                .max()
                                .unwrap_or(0),
                        )
                        .min(total_samples);
                    state.available_samples = total_samples;
                    state.durable_samples = durable_samples;
                    state.source_closed = true;
                    if durable_samples != total_samples {
                        message.push_str(&format!(
                            "; CaptureStop: final durable acknowledgement {durable_samples} does not cover closed source {total_samples}"
                        ));
                    } else {
                        let source =
                            crate::parakeet_audio::PcmPartStore::open(&state.request.source_path);
                        match archives
                            .get_mut(session)
                            .ok_or_else(|| "ArchiveSessionMissing".to_owned())
                            .and_then(|archive| {
                                archive
                                    .prepare_segmented_capture(
                                        state.request.clone(),
                                        &source,
                                        None,
                                        None,
                                    )
                                    .map_err(|error| format!("CaptureArchive: {error}"))
                            }) {
                            Ok(request) => state.request = request,
                            Err(error) => message.push_str(&format!("; {error}")),
                        }
                    }
                    message.push_str(&format!(
                        "; manual capture drained and closed at {total_samples} samples ({durable_samples} durable)"
                    ));
                }
                Ok(None) => message.push_str("; CaptureStop: capture source was unavailable"),
                Err(error) => message.push_str(&format!("; CaptureStop: {error}")),
            }
        }
        scheduler
            .abort_session(*session)
            .map_err(|error| format!("SchedulerManualGpuPeer: {error:?}"))?;
        failed_manual_gpu.push(FailedManualGpuPeer {
            session: *session,
            message,
        });
    }
    scheduler
        .request_recycle()
        .map_err(|error| format!("SchedulerRecycle: {error:?}"))?;
    scheduler
        .cancel_reservation(call)
        .map_err(|error| format!("SchedulerReservation: {error:?}"))?;
    scheduler
        .confirm_child_exited()
        .map_err(|error| format!("SchedulerReap: {error:?}"))?;

    current_state.native_route = Some(current_route.clone());
    for (session, state) in sessions.iter_mut() {
        if let Some(route) = peer_routes.remove(session) {
            state.native_route = Some(route);
            state.backend_ready_reported = false;
        }
    }
    for peer in &failed_manual_gpu {
        sessions.remove(&peer.session);
    }
    *executor_route = Some(current_route.clone());
    executor.replace_reaped_worker(make_cpu_worker(current_route))?;

    let retry = scheduler
        .next_call()
        .map_err(|error| format!("SchedulerRetry: {error:?}"))?
        .ok_or_else(|| {
            "SchedulerRetry: reserved work disappeared during CPU fallback".to_owned()
        })?;
    if retry.session != call.session
        || retry.request_id != call.request_id
        || retry.attempt != call.attempt
        || retry.work != call.work
        || retry.incarnation == call.incarnation
    {
        let _ = scheduler.cancel_reservation(retry);
        let _ = scheduler.request_recycle();
        return Ok(None);
    }
    Ok(Some(NativeAutoCpuRetry {
        call: retry,
        failed_manual_gpu,
    }))
}

impl ImportRuntime {
    pub(in crate::worker_ipc) fn release_unified_terminal_state(
        &mut self,
        job_id: JobId,
        generation: Generation,
    ) {
        let session = whisper_core::parakeet_scheduler::SchedulerSession { job_id, generation };
        if self.unified_sessions.contains_key(&session) {
            return;
        }
        self.document_drains
            .retain(|drain| (drain.job_id, drain.generation) != (job_id, generation));
        self.unified_archives.remove(&session);
        self.unified_resume_checkpoints.remove(&session);
        self.unified_capture_sessions.remove(&session);
        self.unified_publication_gates.remove(&session);
        self.native_unified_routes.remove(&session);
        self.gpu_candidates.remove(&(job_id, generation.get()));
        self.parakeet_pending_events
            .retain(|event| pending_unified_event_identity(event) != Some((job_id, generation)));
        let executor_reaped = self
            .unified_executor
            .as_ref()
            .is_none_or(|executor| executor.thread.is_none() && !executor.in_flight);
        if self.unified_sessions.is_empty()
            && self.parakeet_scheduler.active_sessions() == 0
            && executor_reaped
        {
            self.unified_executor = None;
            self.native_executor_route = None;
        }
    }

    pub(in crate::worker_ipc) fn unified_admission_full(&self) -> bool {
        let mut parent_sessions = std::collections::HashSet::new();
        parent_sessions.extend(self.unified_sessions.keys().copied());
        parent_sessions.extend(self.unified_archives.keys().copied());
        parent_sessions.extend(self.unified_resume_checkpoints.keys().copied());
        parent_sessions.extend(self.unified_publication_gates.keys().copied());
        parent_sessions.extend(self.native_unified_routes.keys().copied());
        parent_sessions.extend(self.document_drains.iter().map(|drain| {
            whisper_core::parakeet_scheduler::SchedulerSession {
                job_id: drain.job_id,
                generation: drain.generation,
            }
        }));
        parent_sessions.len() >= 2 || self.parakeet_scheduler.active_sessions() >= 2
    }

    pub(in crate::worker_ipc) fn unified_admission_full_for(
        &self,
        continuing: whisper_core::parakeet_scheduler::SchedulerSession,
    ) -> bool {
        let mut parent_sessions = std::collections::HashSet::new();
        parent_sessions.extend(
            self.unified_sessions
                .keys()
                .filter(|session| **session != continuing)
                .copied(),
        );
        parent_sessions.extend(
            self.unified_archives
                .keys()
                .filter(|session| **session != continuing)
                .copied(),
        );
        parent_sessions.extend(
            self.unified_resume_checkpoints
                .keys()
                .filter(|session| **session != continuing)
                .copied(),
        );
        parent_sessions.extend(
            self.unified_publication_gates
                .keys()
                .filter(|session| **session != continuing)
                .copied(),
        );
        parent_sessions.extend(
            self.native_unified_routes
                .keys()
                .filter(|session| **session != continuing)
                .copied(),
        );
        parent_sessions.extend(self.document_drains.iter().filter_map(|drain| {
            let session = whisper_core::parakeet_scheduler::SchedulerSession {
                job_id: drain.job_id,
                generation: drain.generation,
            };
            (session != continuing).then_some(session)
        }));
        parent_sessions.len() >= 2
            || (self.parakeet_scheduler.active_sessions() >= 2
                && !self.unified_sessions.contains_key(&continuing))
    }

    pub(in crate::worker_ipc) fn poll_unified_document(
        &mut self,
    ) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
        use whisper_core::ports::ParakeetDocumentPort;
        let mut drain = self.document_drains.pop_front()?;
        if drain.ready_pending {
            drain.ready_pending = false;
            let job_id = drain.job_id;
            let generation = drain.generation;
            self.document_drains.push_front(drain);
            return Some(Ok(ImportEvent::Ready {
                job_id,
                generation,
                backend: "controlled-test-worker".into(),
            }));
        }
        if drain.next_sequence <= drain.revision {
            let page_result = drain.document.read_page(
                drain.job_id,
                drain.generation,
                drain.revision,
                drain.next_sequence,
                1,
            );
            return Some(match page_result {
                Ok(mut page) if page.entries.len() == 1 => {
                    drain.next_sequence += 1;
                    self.document_drains.push_front(drain);
                    Ok(ImportEvent::Segment(page.entries.remove(0).segment))
                }
                Ok(_) => Err((
                    drain.job_id,
                    drain.generation,
                    "StorageCorrupt: controlled page did not contain exactly one segment".into(),
                )),
                Err(error) => Err((
                    drain.job_id,
                    drain.generation,
                    format!("Document: {error:?}"),
                )),
            });
        }
        if drain.progress_pending {
            drain.progress_pending = false;
            let job_id = drain.job_id;
            let generation = drain.generation;
            let total_samples = drain.total_samples;
            self.document_drains.push_front(drain);
            return Some(Ok(ImportEvent::Progress {
                job_id,
                generation,
                completed_samples: total_samples,
                total_samples,
            }));
        }
        Some(Ok(ImportEvent::End {
            job_id: drain.job_id,
            generation: drain.generation,
            last_sequence: drain.revision,
            last_offset: drain.total_samples,
        }))
    }

    pub(in crate::worker_ipc) fn poll_unified_sessions(
        &mut self,
    ) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
        if self.unified_sessions.is_empty() {
            return None;
        }
        let next_native_route = self
            .unified_sessions
            .values()
            .find_map(|state| state.native_route.as_ref());
        let route_changed = self
            .native_executor_route
            .as_ref()
            .zip(next_native_route)
            .is_some_and(|(current, next)| !current.same_worker(next));
        if route_changed {
            let executor_reaped = self
                .unified_executor
                .as_ref()
                .is_none_or(|executor| executor.thread.is_none() && !executor.in_flight);
            if !executor_reaped || self.parakeet_scheduler.in_flight().is_some() {
                let session = self.unified_sessions.keys().next().copied()?;
                return Some(Err((
                    session.job_id,
                    session.generation,
                    "NativeRouteBusy: a different backend cannot replace a live executor incarnation".into(),
                )));
            }
            self.unified_executor = None;
            self.native_executor_route = None;
        }
        if self.unified_executor.is_none() {
            #[cfg(test)]
            let controlled_worker = self.controlled_transcriber.take();
            #[cfg(not(test))]
            let controlled_worker: Option<
                Box<dyn crate::parakeet_pipeline::WindowTranscriber>,
            > = None;
            let native_route = self
                .unified_sessions
                .values()
                .find_map(|state| state.native_route.clone());
            let start_result = if let Some(worker) = controlled_worker {
                #[cfg(test)]
                {
                    match self.controlled_child_observer.clone() {
                        Some(observer) if self.controlled_ready_gate.is_some() => {
                            UnifiedInferenceExecutor::start_observed_with_ready_gate(
                                worker,
                                observer,
                                self.controlled_ready_gate
                                    .take()
                                    .expect("ready gate checked"),
                            )
                        }
                        Some(observer) => {
                            UnifiedInferenceExecutor::start_observed(worker, observer)
                        }
                        None => UnifiedInferenceExecutor::start(worker),
                    }
                }
                #[cfg(not(test))]
                {
                    UnifiedInferenceExecutor::start(worker)
                }
            } else if self.native_unified_candidate_enabled {
                match native_route {
                    Some(route) => {
                        self.native_executor_route = Some(route.clone());
                        let worker = NativeParakeetWindowTranscriber::new(route);
                        #[cfg(test)]
                        let worker = match self.native_harness_observer.clone() {
                            Some(observer) => worker.with_harness_observer(observer),
                            None => worker,
                        };
                        UnifiedInferenceExecutor::start(Box::new(worker))
                    }
                    None => Err(
                        "NativeCandidateUnavailable: no validated route for unified session".into(),
                    ),
                }
            } else {
                Err("ModelNotQualified: unified inference executor is unavailable".into())
            };
            match start_result {
                Ok(executor) => {
                    #[cfg(test)]
                    let mut executor = executor;
                    #[cfg(test)]
                    {
                        executor.disconnect_result_for = self.controlled_disconnect_session;
                        executor.panic_during_transcription_for = self.controlled_panic_session;
                        if executor.disconnect_result_for.is_some()
                            || executor.panic_during_transcription_for.is_some()
                        {
                            if let Err(error) =
                                executor.stop_and_reap().and_then(|()| executor.restart())
                            {
                                return Some(Err((JobId(0), Generation::first(), error)));
                            }
                        }
                    }
                    self.unified_executor = Some(executor);
                }
                Err(error) => {
                    let session = self.unified_sessions.keys().next().copied()?;
                    return Some(Err((session.job_id, session.generation, error)));
                }
            }
        }
        let Some(executor) = self.unified_executor.as_mut() else {
            let identity = self
                .unified_sessions
                .keys()
                .next()
                .map(|session| (session.job_id, session.generation))?;
            return Some(Err((
                identity.0,
                identity.1,
                "ModelNotQualified: unified inference executor is unavailable".into(),
            )));
        };
        if executor.thread.is_none()
            && let Err(error) = executor.restart()
        {
            let session = self.unified_sessions.keys().next().copied()?;
            return Some(Err((
                session.job_id,
                session.generation,
                format!("InferenceRestart: {error}"),
            )));
        }
        let mut pump_error = None;
        let mut progress_updates = Vec::new();
        let mut preview_updates = Vec::new();
        let mut gain_updates = Vec::new();
        for (session, state) in &mut self.unified_sessions {
            if !state.source.is_capture() || state.source_closed {
                continue;
            }
            match state.source.pump() {
                Ok(acknowledgements) => {
                    if let Some(boundary) =
                        acknowledgements.iter().map(|ack| ack.written_samples).max()
                    {
                        let previous_samples = state.available_samples;
                        let previous_durable_samples = state.durable_samples;
                        state.available_samples = state.available_samples.max(boundary);
                        state.durable_samples = state
                            .durable_samples
                            .max(
                                acknowledgements
                                    .iter()
                                    .map(|ack| ack.flushed_samples)
                                    .max()
                                    .unwrap_or(0),
                            )
                            .min(state.available_samples);
                        if state.available_samples > previous_samples
                            || state.durable_samples > previous_durable_samples
                        {
                            progress_updates.push((
                                *session,
                                state.available_samples,
                                state.durable_samples,
                            ));
                        }
                        if boundary >= 4 * 16_000 && boundary > state.preview_sequence {
                            state.preview_sequence = boundary;
                            preview_updates.push((*session, boundary));
                        }
                        gain_updates.extend(
                            state
                                .source
                                .take_gain_events()
                                .into_iter()
                                .map(|gain| (*session, gain)),
                        );
                    }
                }
                Err(error) => {
                    state.failure = Some(error.clone());
                    let _ = self.parakeet_scheduler.abort_session(*session);
                    pump_error = Some((*session, error));
                }
            }
        }
        let _ = pump_error;
        self.parakeet_pending_events
            .extend(gain_updates.into_iter().map(|(session, gain)| {
                ImportEvent::MicrophoneGainApplied {
                    job_id: session.job_id,
                    generation: session.generation,
                    target_identity: gain.target_identity,
                    gain_db: gain.gain_db,
                    gain_revision: gain.gain_revision,
                    first_frame: gain.first_frame,
                }
            }));
        for (session, captured_samples, audio_durable_samples) in progress_updates {
            #[cfg(test)]
            if let Some(observer) = &self.native_harness_observer {
                observer.record(NativeQualificationEventKind::CaptureAcknowledged {
                    session,
                    written_samples: captured_samples,
                    durable_samples: audio_durable_samples,
                });
            }
            self.parakeet_pending_events.retain(|event| {
                !matches!(
                    event,
                    ImportEvent::ParakeetCaptureProgress {
                        job_id,
                        generation,
                        ..
                    } if (*job_id, *generation) == (session.job_id, session.generation)
                )
            });
            self.parakeet_pending_events
                .push_back(ImportEvent::ParakeetCaptureProgress {
                    job_id: session.job_id,
                    generation: session.generation,
                    // Unified capture progress is correlated by this event's
                    // JobId and generation; the legacy session id is unused.
                    session_id: 0,
                    captured_samples,
                    audio_durable_samples,
                });
        }
        for (session, boundary) in preview_updates {
            if let Err(error) = self.parakeet_scheduler.queue_preview(session, boundary) {
                return Some(Err((
                    session.job_id,
                    session.generation,
                    format!("SchedulerPreview: {error:?}"),
                )));
            }
        }
        match executor.try_result() {
            Ok(Some(result)) => {
                let session = result.call.session;
                let Some(mut state) = self.unified_sessions.remove(&session) else {
                    return Some(Err((
                        session.job_id,
                        session.generation,
                        "SchedulerResponse: result belongs to an unknown session".into(),
                    )));
                };
                if state.pending_call != Some(result.call)
                    || self.parakeet_scheduler.in_flight() != Some(result.call)
                {
                    self.unified_sessions.insert(session, state);
                    return Some(Err((
                        session.job_id,
                        session.generation,
                        "SchedulerResponse: stale inference result".into(),
                    )));
                }
                state.pending_call = None;
                if let Some(error) = state.failure.take() {
                    let _ = self.parakeet_scheduler.fail_call(result.call);
                    if let Err(reap_error) = executor.stop_and_reap() {
                        state.failure = Some(error.clone());
                        self.unified_sessions.insert(session, state);
                        return Some(Err((
                            session.job_id,
                            session.generation,
                            format!("{error}; InferenceReap: {reap_error}"),
                        )));
                    }
                    if let Err(reap_error) = self.parakeet_scheduler.confirm_child_exited() {
                        state.failure = Some(error.clone());
                        self.unified_sessions.insert(session, state);
                        return Some(Err((
                            session.job_id,
                            session.generation,
                            format!("{error}; SchedulerReap: {reap_error:?}"),
                        )));
                    }
                    if self.parakeet_scheduler.active_sessions() > 0
                        && let Err(restart_error) = executor.restart()
                    {
                        state.failure = Some(error.clone());
                        self.unified_sessions.insert(session, state);
                        return Some(Err((
                            session.job_id,
                            session.generation,
                            format!("{error}; InferenceRestart: {restart_error}"),
                        )));
                    }
                    return Some(Err((session.job_id, session.generation, error)));
                }
                let words = match result.words {
                    Ok(words) => words,
                    Err(error) => {
                        let _ = self.parakeet_scheduler.fail_call(result.call);
                        let _ = self.parakeet_scheduler.mark_terminal(session);
                        #[cfg(test)]
                        let skip_recycle = self.controlled_skip_child_recycle;
                        #[cfg(not(test))]
                        let skip_recycle = false;
                        if !skip_recycle && let Err(reap_error) = executor.stop_and_reap() {
                            self.unified_sessions.insert(session, state);
                            return Some(Err((
                                session.job_id,
                                session.generation,
                                format!("{error}; InferenceReap: {reap_error}"),
                            )));
                        }
                        if let Err(reap_error) = self.parakeet_scheduler.confirm_child_exited() {
                            self.unified_sessions.insert(session, state);
                            return Some(Err((
                                session.job_id,
                                session.generation,
                                format!("{error}; SchedulerReap: {reap_error:?}"),
                            )));
                        }
                        if self.parakeet_scheduler.active_sessions() > 0
                            && let Err(restart_error) = executor.restart()
                        {
                            self.unified_sessions.insert(session, state);
                            return Some(Err((
                                session.job_id,
                                session.generation,
                                format!("{error}; InferenceRestart: {restart_error}"),
                            )));
                        }
                        return Some(Err((session.job_id, session.generation, error)));
                    }
                };
                let applied = match result.call.work {
                    whisper_core::parakeet_scheduler::ScheduledWork::Final { .. } => state
                        .finalizer
                        .commit_prepared(&mut state.document, result.instance_id, &words),
                    whisper_core::parakeet_scheduler::ScheduledWork::Preview { .. } => {
                        match (
                            state.pending_window.take(),
                            state.pending_available_samples.take(),
                        ) {
                            (Some(window), Some(boundary)) => {
                                crate::parakeet_pipeline::commit_preview(
                                    &mut state.document,
                                    window,
                                    boundary,
                                    result.instance_id,
                                    session.job_id,
                                    session.generation,
                                    &words,
                                )
                                .map(|_| {
                                    crate::parakeet_pipeline::FinalizationStep::WaitingForAudio {
                                        processed_samples: state.finalizer.processed_samples(),
                                    }
                                })
                            }
                            _ => {
                                Err("Pipeline: preview result has no matching prepared window"
                                    .into())
                            }
                        }
                    }
                };
                let valid_step = matches!(
                    result.call.work,
                    whisper_core::parakeet_scheduler::ScheduledWork::Preview { .. }
                ) || matches!(
                    &applied,
                    Ok(crate::parakeet_pipeline::FinalizationStep::WindowProcessed { .. })
                );
                match applied {
                    Ok(_) if valid_step => {
                        if let Err(error) = self.parakeet_scheduler.complete(result.call) {
                            self.unified_sessions.insert(session, state);
                            return Some(Err((
                                session.job_id,
                                session.generation,
                                format!("SchedulerComplete: {error:?}"),
                            )));
                        }
                        self.unified_sessions.insert(session, state);
                        return None;
                    }
                    Ok(_) => {
                        self.unified_sessions.insert(session, state);
                        return Some(Err((
                            session.job_id,
                            session.generation,
                            "Pipeline: inference response did not commit a final window".into(),
                        )));
                    }
                    Err(error) => {
                        let _ = self.parakeet_scheduler.fail_call(result.call);
                        let _ = self.parakeet_scheduler.mark_terminal(session);
                        if let Err(reap_error) = executor.stop_and_reap() {
                            self.unified_sessions.insert(session, state);
                            return Some(Err((
                                session.job_id,
                                session.generation,
                                format!("{error}; InferenceReap: {reap_error}"),
                            )));
                        }
                        if let Err(reap_error) = self.parakeet_scheduler.confirm_child_exited() {
                            self.unified_sessions.insert(session, state);
                            return Some(Err((
                                session.job_id,
                                session.generation,
                                format!("{error}; SchedulerReap: {reap_error:?}"),
                            )));
                        }
                        if self.parakeet_scheduler.active_sessions() > 0
                            && let Err(restart_error) = executor.restart()
                        {
                            self.unified_sessions.insert(session, state);
                            return Some(Err((
                                session.job_id,
                                session.generation,
                                format!("{error}; InferenceRestart: {restart_error}"),
                            )));
                        }
                        return Some(Err((session.job_id, session.generation, error)));
                    }
                }
            }
            Ok(None) => {}
            Err(error) => {
                let Some(call) = self.parakeet_scheduler.in_flight() else {
                    let identity = self.unified_sessions.keys().next().copied().unwrap_or(
                        whisper_core::parakeet_scheduler::SchedulerSession {
                            job_id: JobId(0),
                            generation: Generation::first(),
                        },
                    );
                    return Some(Err((
                        identity.job_id,
                        identity.generation,
                        format!("{error}; SchedulerCallMissing"),
                    )));
                };
                let session = call.session;
                let exited_instance = executor.thread.is_some();
                let reap = executor.reap_disconnected();
                if !exited_instance {
                    return Some(Err((
                        session.job_id,
                        session.generation,
                        format!("{error}; InferenceReap: disconnected instance exit is unproven"),
                    )));
                }
                if let Err(reap_error) = reap {
                    let mut failure = format!("{error}; {reap_error}");
                    let sessions: Vec<_> = self.unified_sessions.keys().copied().collect();
                    for (other, state) in &mut self.unified_sessions {
                        if state.source.is_capture() && !state.source_closed {
                            match state.source.stop_capture() {
                                Ok(Some((total_samples, acknowledgements))) => {
                                    state.available_samples = total_samples;
                                    state.durable_samples = state
                                        .durable_samples
                                        .max(
                                            acknowledgements
                                                .iter()
                                                .map(|ack| ack.flushed_samples)
                                                .max()
                                                .unwrap_or(0),
                                        )
                                        .min(total_samples);
                                    state.source_closed = true;
                                }
                                Ok(None) => {}
                                Err(stop_error) => failure.push_str(&format!(
                                    "; capture {:?}/{:?} stop: {stop_error}",
                                    other.job_id, other.generation
                                )),
                            }
                        }
                    }
                    if let Err(scheduler_error) = self.parakeet_scheduler.fail_call(call) {
                        failure.push_str(&format!("; SchedulerCall: {scheduler_error:?}"));
                    }
                    for other in &sessions {
                        if let Err(scheduler_error) = self.parakeet_scheduler.abort_session(*other)
                        {
                            failure.push_str(&format!("; SchedulerAbort: {scheduler_error:?}"));
                        }
                    }
                    if let Err(scheduler_error) = self.parakeet_scheduler.confirm_child_exited() {
                        failure.push_str(&format!("; SchedulerReap: {scheduler_error:?}"));
                    }
                    for other in sessions {
                        self.unified_sessions.remove(&other);
                        self.release_unified_terminal_state(other.job_id, other.generation);
                        if other != session {
                            self.parakeet_pending_events.push_back(ImportEvent::Failed {
                                job_id: other.job_id,
                                generation: other.generation,
                                message: format!(
                                    "InferenceWorkerLost: peer session terminated after worker panic while processing {:?}/{:?}",
                                    session.job_id, session.generation
                                ),
                            });
                        }
                    }
                    return Some(Err((session.job_id, session.generation, failure)));
                }
                let mut failure = error;
                if let Err(scheduler_error) = self.parakeet_scheduler.fail_call(call) {
                    failure.push_str(&format!("; SchedulerCall: {scheduler_error:?}"));
                    return Some(Err((session.job_id, session.generation, failure)));
                }
                if let Err(scheduler_error) = self.parakeet_scheduler.mark_terminal(session) {
                    failure.push_str(&format!("; SchedulerTerminal: {scheduler_error:?}"));
                    return Some(Err((session.job_id, session.generation, failure)));
                }
                if let Err(scheduler_error) = self.parakeet_scheduler.confirm_child_exited() {
                    failure.push_str(&format!("; SchedulerReap: {scheduler_error:?}"));
                    return Some(Err((session.job_id, session.generation, failure)));
                }
                self.unified_sessions.remove(&session);
                if self.parakeet_scheduler.active_sessions() > 0
                    && let Err(restart_error) = executor.restart()
                {
                    failure.push_str(&format!("; InferenceRestart: {restart_error}"));
                }
                return Some(Err((session.job_id, session.generation, failure)));
            }
        }
        if executor.in_flight {
            return None;
        }
        if let Some(session) = self
            .unified_sessions
            .iter()
            .find_map(|(session, state)| state.failure.as_ref().map(|_| *session))
        {
            let mut state = self.unified_sessions.remove(&session)?;
            let error = state.failure.take().unwrap_or_else(|| "CaptureLost".into());
            if let Err(reap_error) = executor.stop_and_reap() {
                state.failure = Some(error.clone());
                self.unified_sessions.insert(session, state);
                return Some(Err((
                    session.job_id,
                    session.generation,
                    format!("{error}; InferenceReap: {reap_error}"),
                )));
            }
            if let Err(reap_error) = self.parakeet_scheduler.confirm_child_exited() {
                state.failure = Some(error.clone());
                self.unified_sessions.insert(session, state);
                return Some(Err((
                    session.job_id,
                    session.generation,
                    format!("{error}; SchedulerReap: {reap_error:?}"),
                )));
            }
            if self.parakeet_scheduler.active_sessions() > 0
                && let Err(restart_error) = executor.restart()
            {
                state.failure = Some(error.clone());
                self.unified_sessions.insert(session, state);
                return Some(Err((
                    session.job_id,
                    session.generation,
                    format!("{error}; InferenceRestart: {restart_error}"),
                )));
            }
            return Some(Err((session.job_id, session.generation, error)));
        }
        let finished_session = self.unified_sessions.iter().find_map(|(session, state)| {
            (state.source_closed
                && state.pending_call.is_none()
                && state.pending_prepared.is_none()
                && self
                    .parakeet_scheduler
                    .in_flight()
                    .is_none_or(|call| call.session != *session)
                && state.finalizer.processed_samples() == state.available_samples)
                .then_some(*session)
        });
        if let Some(session) = finished_session {
            let mut state = self.unified_sessions.remove(&session)?;
            #[cfg(test)]
            let force_finish_failure = self.controlled_finish_failure_session == Some(session);
            #[cfg(not(test))]
            let force_finish_failure = false;
            let result = if force_finish_failure {
                Err("TestOnly: injected document finalization failure".to_owned())
            } else {
                state.finalizer.finish_document(
                    &mut state.document,
                    state.available_samples,
                    state.source_closed,
                )
            };
            let result = match result {
                Ok(crate::parakeet_pipeline::FinalizationStep::Complete(result)) => result,
                Ok(_) => {
                    let error = "Pipeline: closed document did not finalize".to_owned();
                    state.failure = Some(error);
                    let _ = self.parakeet_scheduler.abort_session(session);
                    self.unified_sessions.insert(session, state);
                    return None;
                }
                Err(error) => {
                    state.failure = Some(error);
                    let _ = self.parakeet_scheduler.abort_session(session);
                    self.unified_sessions.insert(session, state);
                    return None;
                }
            };
            if let Err(error) = self.parakeet_scheduler.mark_terminal(session) {
                self.unified_sessions.insert(session, state);
                return Some(Err((
                    session.job_id,
                    session.generation,
                    format!("SchedulerTerminal: {error:?}"),
                )));
            }
            if let Err(error) = executor.stop_and_reap() {
                self.unified_sessions.insert(session, state);
                return Some(Err((
                    session.job_id,
                    session.generation,
                    format!("InferenceReap: {error}"),
                )));
            }
            if let Err(error) = self.parakeet_scheduler.confirm_child_exited() {
                self.unified_sessions.insert(session, state);
                return Some(Err((
                    session.job_id,
                    session.generation,
                    format!("SchedulerReap: {error:?}"),
                )));
            }
            if self.parakeet_scheduler.active_sessions() > 0 {
                #[cfg(test)]
                if let Some(gate) = self.controlled_ready_gate.take() {
                    executor.ready_gate = Some(gate);
                }
                if let Err(error) = executor.restart() {
                    self.unified_sessions.insert(session, state);
                    return Some(Err((
                        session.job_id,
                        session.generation,
                        format!("InferenceRestart: {error}"),
                    )));
                }
            }
            let ready_pending = !state.backend_ready_reported;
            self.document_drains.push_back(UnifiedDocumentDrain {
                document: state.document,
                job_id: session.job_id,
                generation: session.generation,
                revision: result.document_revision,
                total_samples: result.total_samples,
                next_sequence: state.document_drain_start_sequence,
                ready_pending,
                progress_pending: true,
            });
            return None;
        }
        for (session, state) in &self.unified_sessions {
            let remaining = state
                .available_samples
                .saturating_sub(state.finalizer.processed_samples());
            if state.finalizer.processed_samples() < state.available_samples
                && (state.source_closed || remaining >= 120 * 16_000 + 2 * 200 * 16)
            {
                let _ = self
                    .parakeet_scheduler
                    .queue_final(*session, state.finalizer.next_window_sequence());
            }
        }
        let call = if let Some(call) = self.parakeet_scheduler.in_flight() {
            call
        } else {
            match self.parakeet_scheduler.next_call() {
                Ok(Some(call)) => call,
                Ok(None) => return None,
                Err(error) => {
                    let identity = self.unified_sessions.keys().next().copied()?;
                    return Some(Err((
                        identity.job_id,
                        identity.generation,
                        format!("SchedulerSelect: {error:?}"),
                    )));
                }
            }
        };
        #[cfg(test)]
        if self.controlled_hold_reserved_call_once {
            self.controlled_hold_reserved_call_once = false;
            return None;
        }
        let Some(mut state) = self.unified_sessions.remove(&call.session) else {
            let _ = self.parakeet_scheduler.cancel_reservation(call);
            return Some(Err((
                call.session.job_id,
                call.session.generation,
                "SchedulerSelection: selected session is missing".into(),
            )));
        };
        let prepared = if let Some((prepared_call, window, pcm, is_final)) =
            state.pending_prepared.take()
        {
            if prepared_call != call {
                self.unified_sessions.insert(call.session, state);
                return Some(Err((
                    call.session.job_id,
                    call.session.generation,
                    "SchedulerPreparation: retained call identity changed".into(),
                )));
            }
            Ok(Some((window, pcm, is_final)))
        } else {
            match call.work {
                whisper_core::parakeet_scheduler::ScheduledWork::Final { .. } => state
                    .finalizer
                    .prepare_next(
                        &mut state.source,
                        state.available_samples,
                        state.source_closed,
                    )
                    .map(|prepared| match prepared {
                        crate::parakeet_pipeline::PreparedFinalWindow::Ready { window, pcm } => {
                            Some((window, pcm, true))
                        }
                        crate::parakeet_pipeline::PreparedFinalWindow::WaitingForAudio {
                            ..
                        }
                        | crate::parakeet_pipeline::PreparedFinalWindow::Complete => None,
                    }),
                whisper_core::parakeet_scheduler::ScheduledWork::Preview { sequence } => {
                    crate::parakeet_pipeline::prepare_preview(&mut state.source, sequence, sequence)
                        .map(|prepared| Some((prepared.window, prepared.pcm, false)))
                }
            }
        };
        match prepared {
            Ok(Some((window, pcm, true))) => {
                match executor.is_ready() {
                    Ok(true) if state.native_route.is_some() && !state.backend_ready_reported => {
                        state.backend_ready_reported = true;
                        let backend = state
                            .native_route
                            .as_ref()
                            .expect("native route guard")
                            .backend_label();
                        state.pending_prepared = Some((call, window, pcm, true));
                        self.unified_sessions.insert(call.session, state);
                        return Some(Ok(ImportEvent::Ready {
                            job_id: call.session.job_id,
                            generation: call.session.generation,
                            backend,
                        }));
                    }
                    Ok(true) => {}
                    Ok(false) => {
                        state.pending_prepared = Some((call, window, pcm, true));
                        self.unified_sessions.insert(call.session, state);
                        return None;
                    }
                    Err(error) => {
                        let cpu_paths = self.parakeet_paths.clone();
                        match retry_native_gpu_startup_on_cpu(
                            NativeRetryState {
                                scheduler: &mut self.parakeet_scheduler,
                                sessions: &mut self.unified_sessions,
                                archives: &mut self.unified_archives,
                                executor_route: &mut self.native_executor_route,
                                executor,
                            },
                            &mut state,
                            call,
                            &error,
                            |request, reason| {
                                let mut route = native_cpu_route(request, cpu_paths.as_ref())?;
                                route.fallback_reason = Some(reason.to_owned());
                                Ok(route)
                            },
                            |route| Box::new(NativeParakeetWindowTranscriber::new(route)),
                        ) {
                            Ok(Some(retry)) => {
                                for peer in retry.failed_manual_gpu {
                                    self.release_unified_terminal_state(
                                        peer.session.job_id,
                                        peer.session.generation,
                                    );
                                    self.parakeet_pending_events
                                        .push_back(manual_gpu_failure_event(peer));
                                }
                                state.pending_prepared = Some((retry.call, window, pcm, true));
                                self.unified_sessions.insert(call.session, state);
                                return None;
                            }
                            Err(fallback_error) => {
                                let _ = self.parakeet_scheduler.cancel_reservation(call);
                                let _ = self.parakeet_scheduler.mark_terminal(call.session);
                                state.failure =
                                    Some(format!("{error}; AutoFallback: {fallback_error}"));
                                self.unified_sessions.insert(call.session, state);
                                return Some(Err((
                                    call.session.job_id,
                                    call.session.generation,
                                    format!("{error}; AutoFallback: {fallback_error}"),
                                )));
                            }
                            Ok(None) => {}
                        }
                        let _ = self.parakeet_scheduler.cancel_reservation(call);
                        let _ = self.parakeet_scheduler.mark_terminal(call.session);
                        state.failure = Some(error.clone());
                        self.unified_sessions.insert(call.session, state);
                        return Some(Err((call.session.job_id, call.session.generation, error)));
                    }
                }
                if let Err(error) = self.parakeet_scheduler.begin_call(call) {
                    let _ = self.parakeet_scheduler.cancel_reservation(call);
                    state.failure = Some(format!("SchedulerBegin: {error:?}"));
                    let _ = self.parakeet_scheduler.abort_session(call.session);
                    self.unified_sessions.insert(call.session, state);
                    return Some(Err((
                        call.session.job_id,
                        call.session.generation,
                        format!("SchedulerBegin: {error:?}"),
                    )));
                }
                match executor.dispatch(UnifiedInferenceCommand {
                    call,
                    window,
                    pcm,
                    scratch_directory: state.inference_directory.clone(),
                }) {
                    Ok(()) => {
                        state.pending_call = Some(call);
                        self.unified_sessions.insert(call.session, state);
                        None
                    }
                    Err(error) => {
                        let _ = self.parakeet_scheduler.fail_call(call);
                        let _ = self.parakeet_scheduler.mark_terminal(call.session);
                        state.failure = Some(error.clone());
                        self.unified_sessions.insert(call.session, state);
                        Some(Err((call.session.job_id, call.session.generation, error)))
                    }
                }
            }
            Ok(Some((window, pcm, false))) => {
                match executor.is_ready() {
                    Ok(true) if state.native_route.is_some() && !state.backend_ready_reported => {
                        state.backend_ready_reported = true;
                        let backend = state
                            .native_route
                            .as_ref()
                            .expect("native route guard")
                            .backend_label();
                        state.pending_prepared = Some((call, window, pcm, false));
                        self.unified_sessions.insert(call.session, state);
                        return Some(Ok(ImportEvent::Ready {
                            job_id: call.session.job_id,
                            generation: call.session.generation,
                            backend,
                        }));
                    }
                    Ok(true) => {}
                    Ok(false) => {
                        state.pending_prepared = Some((call, window, pcm, false));
                        self.unified_sessions.insert(call.session, state);
                        return None;
                    }
                    Err(error) => {
                        let cpu_paths = self.parakeet_paths.clone();
                        match retry_native_gpu_startup_on_cpu(
                            NativeRetryState {
                                scheduler: &mut self.parakeet_scheduler,
                                sessions: &mut self.unified_sessions,
                                archives: &mut self.unified_archives,
                                executor_route: &mut self.native_executor_route,
                                executor,
                            },
                            &mut state,
                            call,
                            &error,
                            |request, reason| {
                                let mut route = native_cpu_route(request, cpu_paths.as_ref())?;
                                route.fallback_reason = Some(reason.to_owned());
                                Ok(route)
                            },
                            |route| Box::new(NativeParakeetWindowTranscriber::new(route)),
                        ) {
                            Ok(Some(retry)) => {
                                for peer in retry.failed_manual_gpu {
                                    self.release_unified_terminal_state(
                                        peer.session.job_id,
                                        peer.session.generation,
                                    );
                                    self.parakeet_pending_events
                                        .push_back(manual_gpu_failure_event(peer));
                                }
                                state.pending_prepared = Some((retry.call, window, pcm, false));
                                self.unified_sessions.insert(call.session, state);
                                return None;
                            }
                            Err(fallback_error) => {
                                let _ = self.parakeet_scheduler.cancel_reservation(call);
                                let _ = self.parakeet_scheduler.mark_terminal(call.session);
                                state.failure =
                                    Some(format!("{error}; AutoFallback: {fallback_error}"));
                                self.unified_sessions.insert(call.session, state);
                                return Some(Err((
                                    call.session.job_id,
                                    call.session.generation,
                                    format!("{error}; AutoFallback: {fallback_error}"),
                                )));
                            }
                            Ok(None) => {}
                        }
                        let _ = self.parakeet_scheduler.cancel_reservation(call);
                        let _ = self.parakeet_scheduler.mark_terminal(call.session);
                        state.failure = Some(error.clone());
                        self.unified_sessions.insert(call.session, state);
                        return Some(Err((call.session.job_id, call.session.generation, error)));
                    }
                }
                state.pending_window = Some(window);
                state.pending_available_samples = Some(window.central_end_sample);
                if let Err(error) = self.parakeet_scheduler.begin_call(call) {
                    let _ = self.parakeet_scheduler.cancel_reservation(call);
                    state.failure = Some(format!("SchedulerBegin: {error:?}"));
                    let _ = self.parakeet_scheduler.abort_session(call.session);
                    self.unified_sessions.insert(call.session, state);
                    return Some(Err((
                        call.session.job_id,
                        call.session.generation,
                        format!("SchedulerBegin: {error:?}"),
                    )));
                }
                match executor.dispatch(UnifiedInferenceCommand {
                    call,
                    window,
                    pcm,
                    scratch_directory: state.inference_directory.clone(),
                }) {
                    Ok(()) => {
                        state.pending_call = Some(call);
                        self.unified_sessions.insert(call.session, state);
                        None
                    }
                    Err(error) => {
                        let _ = self.parakeet_scheduler.fail_call(call);
                        let _ = self.parakeet_scheduler.mark_terminal(call.session);
                        state.failure = Some(error.clone());
                        self.unified_sessions.insert(call.session, state);
                        Some(Err((call.session.job_id, call.session.generation, error)))
                    }
                }
            }
            Ok(None) => {
                let _ = self.parakeet_scheduler.cancel_reservation(call);
                self.unified_sessions.insert(call.session, state);
                None
            }
            Err(error) => {
                let _ = self.parakeet_scheduler.cancel_reservation(call);
                state.failure = Some(error.clone());
                let _ = self.parakeet_scheduler.abort_session(call.session);
                self.unified_sessions.insert(call.session, state);
                Some(Err((call.session.job_id, call.session.generation, error)))
            }
        }
    }
}
