use super::*;

impl ImportRuntime {
    pub(super) fn recover_disconnected_unified_executor(
        &mut self,
        error: String,
    ) -> UnifiedPollOutcome {
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
        let (exited_instance, reap) = {
            let executor = self
                .unified_executor
                .as_mut()
                .expect("executor remains present during inference recovery");
            let exited_instance = executor.thread.is_some();
            (exited_instance, executor.reap_disconnected())
        };
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
                if let Err(scheduler_error) = self.parakeet_scheduler.abort_session(*other) {
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
            && let Err(restart_error) = self
                .unified_executor
                .as_mut()
                .expect("executor remains present during inference recovery")
                .restart()
        {
            failure.push_str(&format!("; InferenceRestart: {restart_error}"));
        }
        Some(Err((session.job_id, session.generation, failure)))
    }

    pub(super) fn finish_failed_idle_capture(
        &mut self,
    ) -> std::ops::ControlFlow<UnifiedPollOutcome, ()> {
        let Some(session) = self
            .unified_sessions
            .iter()
            .find_map(|(session, state)| state.failure.as_ref().map(|_| *session))
        else {
            return std::ops::ControlFlow::Continue(());
        };
        let Some(mut state) = self.unified_sessions.remove(&session) else {
            return std::ops::ControlFlow::Break(None);
        };
        let error = state.failure.take().unwrap_or_else(|| "CaptureLost".into());
        if let Err(reap_error) = self
            .unified_executor
            .as_mut()
            .expect("executor remains present during capture failure recovery")
            .stop_and_reap()
        {
            state.failure = Some(error.clone());
            self.unified_sessions.insert(session, state);
            return std::ops::ControlFlow::Break(Some(Err((
                session.job_id,
                session.generation,
                format!("{error}; InferenceReap: {reap_error}"),
            ))));
        }
        if let Err(reap_error) = self.parakeet_scheduler.confirm_child_exited() {
            state.failure = Some(error.clone());
            self.unified_sessions.insert(session, state);
            return std::ops::ControlFlow::Break(Some(Err((
                session.job_id,
                session.generation,
                format!("{error}; SchedulerReap: {reap_error:?}"),
            ))));
        }
        if self.parakeet_scheduler.active_sessions() > 0
            && let Err(restart_error) = self
                .unified_executor
                .as_mut()
                .expect("executor remains present during capture failure recovery")
                .restart()
        {
            state.failure = Some(error.clone());
            self.unified_sessions.insert(session, state);
            return std::ops::ControlFlow::Break(Some(Err((
                session.job_id,
                session.generation,
                format!("{error}; InferenceRestart: {restart_error}"),
            ))));
        }
        std::ops::ControlFlow::Break(Some(Err((session.job_id, session.generation, error))))
    }
}
