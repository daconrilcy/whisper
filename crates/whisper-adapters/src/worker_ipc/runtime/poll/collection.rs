use super::*;

pub(super) enum PollCollectionControl {
    Next,
    Return,
    Finish(WorkerEvent),
}

type PollOutput = Option<Result<ImportEvent, (JobId, Generation, String)>>;

fn return_poll(output: &mut PollOutput, value: PollOutput) -> PollCollectionControl {
    *output = value;
    PollCollectionControl::Return
}

#[warn(clippy::too_many_lines, clippy::cognitive_complexity)]
impl ImportRuntime {
    pub(super) fn collect_poll_event(&mut self, output: &mut PollOutput) -> PollCollectionControl {
        match self.poll_priority_sources(output) {
            PollCollectionControl::Next => {}
            control => return control,
        }
        match self.poll_parakeet_capture(output) {
            PollCollectionControl::Next => {}
            control => return control,
        }
        match self.poll_live_observations(output) {
            PollCollectionControl::Next => {}
            control => return control,
        }
        match self.poll_live_capture_event(output) {
            PollCollectionControl::Next => {}
            control => return control,
        }
        match self.poll_parakeet_vad(output) {
            PollCollectionControl::Next => {}
            control => return control,
        }
        match self.poll_parakeet_event(output) {
            PollCollectionControl::Next => {}
            control => return control,
        }
        match self.poll_deferred_live_stop(output) {
            PollCollectionControl::Next => {}
            control => return control,
        }
        self.poll_transport_event(output)
    }

    fn poll_priority_sources(&mut self, output: &mut PollOutput) -> PollCollectionControl {
        if let Some(event) = self.poll_unified_sessions() {
            if let Err((job_id, generation, _)) = &event {
                self.release_unified_terminal_state(*job_id, *generation);
            }
            return return_poll(output, Some(event));
        }
        if let Some(event) = self.poll_unified_document() {
            if let Err((job_id, generation, _)) = &event {
                self.release_unified_terminal_state(*job_id, *generation);
            }
            return return_poll(output, Some(event));
        }
        if let Some(event) = self.parakeet_pending_events.pop_front() {
            return return_poll(output, Some(Ok(event)));
        }
        match self.poll_microphone_test(output) {
            PollCollectionControl::Next => {}
            control => return control,
        }
        if self
            .idle_worker
            .as_ref()
            .is_some_and(|idle| idle.expires_at <= Instant::now())
        {
            self.idle_worker = None;
        }
        self.poll_direct_live(output)
    }

    fn poll_microphone_test(&mut self, output: &mut PollOutput) -> PollCollectionControl {
        if let Some(test) = self.microphone_test.as_mut() {
            let identity = (test.job_id, test.generation);
            match test.pump() {
                Ok(Some(event)) => return return_poll(output, Some(Ok(event))),
                Ok(None) => {}
                Err(error) => {
                    if let Some(mut test) = self.microphone_test.take() {
                        test.probe.invalidate();
                        test.capture.stop();
                    }
                    return return_poll(output, Some(Err((identity.0, identity.1, error))));
                }
            }
        }
        PollCollectionControl::Next
    }

    fn poll_direct_live(&mut self, output: &mut PollOutput) -> PollCollectionControl {
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
                Ok(Some(event)) => return return_poll(output, Some(Ok(event))),
                Ok(None) => {}
                Err(error) => {
                    return return_poll(output, Some(Err((id.0, id.1, error))));
                }
            }
        }
        PollCollectionControl::Next
    }

    fn poll_parakeet_capture(&mut self, output: &mut PollOutput) -> PollCollectionControl {
        if let Some(live) = self.parakeet_live.as_mut() {
            let id = live.identity();
            match live.poll() {
                Ok(Some(event)) => return return_poll(output, Some(Ok(event))),
                Ok(None) => {}
                Err(error) => {
                    let event = live.stop_after_capture_error(error);
                    let event = attach_automatic_final_import(live, event);
                    self.parakeet_pending_events.push_back(event);
                    self.parakeet_live = None;
                    if let Some(event) = self.parakeet_pending_events.pop_front() {
                        return return_poll(output, Some(Ok(event)));
                    }
                    return return_poll(
                        output,
                        Some(Err((id.0, id.1, "Capture Parakeet interrompue".into()))),
                    );
                }
            }
        }
        PollCollectionControl::Next
    }

    fn poll_live_observations(&mut self, output: &mut PollOutput) -> PollCollectionControl {
        if let Some(event) = self.pending_recovery_counters.take() {
            return return_poll(output, Some(Ok(event)));
        }
        if self.live.as_ref().is_some_and(|live| live.worker_ready)
            && let Some(event) = self.pending_attempt_reset.take()
        {
            return return_poll(output, Some(Ok(event)));
        }
        if let Some(event) = self.take_observation_notice() {
            return return_poll(output, Some(Ok(event)));
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
                return return_poll(output, Some(Ok(event)));
            }
        }
        PollCollectionControl::Next
    }

    fn poll_live_capture_event(&mut self, output: &mut PollOutput) -> PollCollectionControl {
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
            return return_poll(output, Some(event));
        }
        if self.deferred_live_window_finished.is_some() && self.live_pending_persistence > 0 {
            return return_poll(output, None);
        }
        PollCollectionControl::Next
    }

    fn poll_parakeet_vad(&mut self, output: &mut PollOutput) -> PollCollectionControl {
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
            return return_poll(output, Some(Err(failure)));
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
                return return_poll(output, Some(Err(failure)));
            }
            let active = match self.active.as_ref() {
                Some(active) => active,
                None => return return_poll(output, None),
            };
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
        PollCollectionControl::Next
    }

    fn poll_parakeet_event(&mut self, output: &mut PollOutput) -> PollCollectionControl {
        if self.parakeet.is_some() || !self.parakeet_events.is_empty() {
            match self.poll_parakeet_worker() {
                Some(Ok(event)) => return PollCollectionControl::Finish(event),
                Some(Err(error)) => {
                    let active = match self.active.as_ref() {
                        Some(active) => active,
                        None => return return_poll(output, None),
                    };
                    let failure = (active.job_id, active.generation, format!("{error:?}"));
                    self.parakeet = None;
                    self.parakeet_stopped_ack = false;
                    self.parakeet_events.clear();
                    self.active = None;
                    return return_poll(output, Some(Err(failure)));
                }
                None => return return_poll(output, None),
            }
        }
        PollCollectionControl::Next
    }

    fn poll_deferred_live_stop(&mut self, output: &mut PollOutput) -> PollCollectionControl {
        if self.deferred_live_stopped.is_some() {
            if self.live_pending_persistence > 0
                || self
                    .transport
                    .as_mut()
                    .and_then(|transport| transport.try_wait().ok().flatten())
                    .is_none()
            {
                return return_poll(output, None);
            }
            return match take_ready_live_stop(
                self.live_pending_persistence,
                &mut self.deferred_live_stopped,
            ) {
                Some(event) => PollCollectionControl::Finish(event),
                None => return_poll(output, None),
            };
        }
        PollCollectionControl::Next
    }

    fn poll_transport_event(&mut self, output: &mut PollOutput) -> PollCollectionControl {
        let transport = match self.transport.as_mut() {
            Some(transport) => transport,
            None => return return_poll(output, None),
        };
        let received = transport.receive();
        self.live_capture_events_since_worker_poll = 0;
        match received {
            Ok(Some(event)) => PollCollectionControl::Finish(event),
            Ok(None) => {
                let _ = transport.try_wait();
                return_poll(output, self.take_legacy_live_telemetry().map(Ok))
            }
            Err(error) => self.poll_transport_error(error, output),
        }
    }

    fn poll_transport_error(
        &mut self,
        error: PortError,
        output: &mut PollOutput,
    ) -> PollCollectionControl {
        let active = match self.active.as_ref() {
            Some(active) => active,
            None => return return_poll(output, None),
        };
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
            return return_poll(
                output,
                Some(self.handle_worker_failure(
                    job_id,
                    generation,
                    whisper_core::ipc::WorkerErrorCode::WorkerExited,
                    format!("WorkerExited without End: {error:?}"),
                )),
            );
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
        return_poll(output, Some(failure))
    }
}
