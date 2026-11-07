use super::*;

type PreparedWindow = (whisper_core::parakeet_session::AudioWindow, Vec<i16>, bool);

impl ImportRuntime {
    pub(super) fn launch_next_unified_call(&mut self) -> UnifiedPollOutcome {
        let call = match self.reserve_next_unified_call() {
            std::ops::ControlFlow::Continue(call) => call,
            std::ops::ControlFlow::Break(outcome) => return outcome,
        };
        let Some(state) = self.unified_sessions.remove(&call.session) else {
            let _ = self.parakeet_scheduler.cancel_reservation(call);
            return Some(Err((
                call.session.job_id,
                call.session.generation,
                "SchedulerSelection: selected session is missing".into(),
            )));
        };
        self.prepare_and_dispatch_unified_call(call, state)
    }

    fn reserve_next_unified_call(
        &mut self,
    ) -> std::ops::ControlFlow<UnifiedPollOutcome, whisper_core::parakeet_scheduler::ScheduledCall>
    {
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
                Ok(None) => return std::ops::ControlFlow::Break(None),
                Err(error) => {
                    let Some(identity) = self.unified_sessions.keys().next().copied() else {
                        return std::ops::ControlFlow::Break(None);
                    };
                    return std::ops::ControlFlow::Break(Some(Err((
                        identity.job_id,
                        identity.generation,
                        format!("SchedulerSelect: {error:?}"),
                    ))));
                }
            }
        };
        #[cfg(test)]
        if self.controlled_hold_reserved_call_once {
            self.controlled_hold_reserved_call_once = false;
            return std::ops::ControlFlow::Break(None);
        }
        std::ops::ControlFlow::Continue(call)
    }

    fn prepare_and_dispatch_unified_call(
        &mut self,
        call: whisper_core::parakeet_scheduler::ScheduledCall,
        mut state: UnifiedSessionState,
    ) -> UnifiedPollOutcome {
        let prepared =
            if let Some((prepared_call, window, pcm, is_final)) = state.pending_prepared.take() {
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
                self.prepare_unified_window(call, &mut state)
            };
        match prepared {
            Ok(Some((window, pcm, true))) => self.dispatch_prepared_final(call, state, window, pcm),
            Ok(Some((window, pcm, false))) => {
                self.dispatch_prepared_preview(call, state, window, pcm)
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

    fn prepare_unified_window(
        &self,
        call: whisper_core::parakeet_scheduler::ScheduledCall,
        state: &mut UnifiedSessionState,
    ) -> Result<Option<PreparedWindow>, String> {
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
                    crate::parakeet_pipeline::PreparedFinalWindow::WaitingForAudio { .. }
                    | crate::parakeet_pipeline::PreparedFinalWindow::Complete => None,
                }),
            whisper_core::parakeet_scheduler::ScheduledWork::Preview { sequence } => {
                crate::parakeet_pipeline::prepare_preview(&mut state.source, sequence, sequence)
                    .map(|prepared| Some((prepared.window, prepared.pcm, false)))
            }
        }
    }

    fn dispatch_prepared_final(
        &mut self,
        call: whisper_core::parakeet_scheduler::ScheduledCall,
        state: UnifiedSessionState,
        window: whisper_core::parakeet_session::AudioWindow,
        pcm: Vec<i16>,
    ) -> UnifiedPollOutcome {
        let (state, window, pcm) =
            match self.ensure_unified_backend_ready(call, state, window, pcm, true) {
                std::ops::ControlFlow::Continue(prepared) => prepared,
                std::ops::ControlFlow::Break(outcome) => return outcome,
            };
        self.begin_and_dispatch_unified_call(call, state, window, pcm)
    }

    fn dispatch_prepared_preview(
        &mut self,
        call: whisper_core::parakeet_scheduler::ScheduledCall,
        state: UnifiedSessionState,
        window: whisper_core::parakeet_session::AudioWindow,
        pcm: Vec<i16>,
    ) -> UnifiedPollOutcome {
        let (mut state, window, pcm) =
            match self.ensure_unified_backend_ready(call, state, window, pcm, false) {
                std::ops::ControlFlow::Continue(prepared) => prepared,
                std::ops::ControlFlow::Break(outcome) => return outcome,
            };
        state.pending_window = Some(window);
        state.pending_available_samples = Some(window.central_end_sample);
        self.begin_and_dispatch_unified_call(call, state, window, pcm)
    }

    fn ensure_unified_backend_ready(
        &mut self,
        call: whisper_core::parakeet_scheduler::ScheduledCall,
        mut state: UnifiedSessionState,
        window: whisper_core::parakeet_session::AudioWindow,
        pcm: Vec<i16>,
        is_final: bool,
    ) -> std::ops::ControlFlow<
        UnifiedPollOutcome,
        (
            UnifiedSessionState,
            whisper_core::parakeet_session::AudioWindow,
            Vec<i16>,
        ),
    > {
        match self
            .unified_executor
            .as_mut()
            .expect("executor remains present during call preparation")
            .is_ready()
        {
            Ok(true) if state.native_route.is_some() && !state.backend_ready_reported => {
                state.backend_ready_reported = true;
                let backend = state
                    .native_route
                    .as_ref()
                    .expect("native route guard")
                    .backend_label();
                state.pending_prepared = Some((call, window, pcm, is_final));
                self.unified_sessions.insert(call.session, state);
                std::ops::ControlFlow::Break(Some(Ok(ImportEvent::Ready {
                    job_id: call.session.job_id,
                    generation: call.session.generation,
                    backend,
                })))
            }
            Ok(true) => std::ops::ControlFlow::Continue((state, window, pcm)),
            Ok(false) => {
                state.pending_prepared = Some((call, window, pcm, is_final));
                self.unified_sessions.insert(call.session, state);
                std::ops::ControlFlow::Break(None)
            }
            Err(error) => {
                self.handle_unified_readiness_error(call, state, window, pcm, is_final, error)
            }
        }
    }

    fn handle_unified_readiness_error(
        &mut self,
        call: whisper_core::parakeet_scheduler::ScheduledCall,
        mut state: UnifiedSessionState,
        window: whisper_core::parakeet_session::AudioWindow,
        pcm: Vec<i16>,
        is_final: bool,
        error: String,
    ) -> std::ops::ControlFlow<
        UnifiedPollOutcome,
        (
            UnifiedSessionState,
            whisper_core::parakeet_session::AudioWindow,
            Vec<i16>,
        ),
    > {
        let cpu_paths = self.parakeet_paths.clone();
        match retry_native_gpu_startup_on_cpu(
            NativeRetryState {
                scheduler: &mut self.parakeet_scheduler,
                sessions: &mut self.unified_sessions,
                archives: &mut self.unified_archives,
                executor_route: &mut self.native_executor_route,
                executor: self
                    .unified_executor
                    .as_mut()
                    .expect("executor remains present during native retry"),
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
                state.pending_prepared = Some((retry.call, window, pcm, is_final));
                self.unified_sessions.insert(call.session, state);
                std::ops::ControlFlow::Break(None)
            }
            Err(fallback_error) => {
                let _ = self.parakeet_scheduler.cancel_reservation(call);
                let _ = self.parakeet_scheduler.mark_terminal(call.session);
                state.failure = Some(format!("{error}; AutoFallback: {fallback_error}"));
                self.unified_sessions.insert(call.session, state);
                std::ops::ControlFlow::Break(Some(Err((
                    call.session.job_id,
                    call.session.generation,
                    format!("{error}; AutoFallback: {fallback_error}"),
                ))))
            }
            Ok(None) => {
                let _ = self.parakeet_scheduler.cancel_reservation(call);
                let _ = self.parakeet_scheduler.mark_terminal(call.session);
                state.failure = Some(error.clone());
                self.unified_sessions.insert(call.session, state);
                std::ops::ControlFlow::Break(Some(Err((
                    call.session.job_id,
                    call.session.generation,
                    error,
                ))))
            }
        }
    }

    fn begin_and_dispatch_unified_call(
        &mut self,
        call: whisper_core::parakeet_scheduler::ScheduledCall,
        mut state: UnifiedSessionState,
        window: whisper_core::parakeet_session::AudioWindow,
        pcm: Vec<i16>,
    ) -> UnifiedPollOutcome {
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
        match self
            .unified_executor
            .as_mut()
            .expect("executor remains present during call dispatch")
            .dispatch(UnifiedInferenceCommand {
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
}
