use super::*;

impl ImportRuntime {
    pub(super) fn finish_unified_document(
        &mut self,
    ) -> std::ops::ControlFlow<UnifiedPollOutcome, ()> {
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
        let Some(session) = finished_session else {
            return std::ops::ControlFlow::Continue(());
        };
        let Some(state) = self.unified_sessions.remove(&session) else {
            return std::ops::ControlFlow::Break(None);
        };
        std::ops::ControlFlow::Break(self.finish_unified_document_state(session, state))
    }

    fn finish_unified_document_state(
        &mut self,
        session: whisper_core::parakeet_scheduler::SchedulerSession,
        mut state: UnifiedSessionState,
    ) -> UnifiedPollOutcome {
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
                state.failure = Some("Pipeline: closed document did not finalize".to_owned());
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
        if let Err(error) = self
            .unified_executor
            .as_mut()
            .expect("executor remains present during document finalization")
            .stop_and_reap()
        {
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
                self.unified_executor
                    .as_mut()
                    .expect("executor remains present during document finalization")
                    .ready_gate = Some(gate);
            }
            if let Err(error) = self
                .unified_executor
                .as_mut()
                .expect("executor remains present during document finalization")
                .restart()
            {
                self.unified_sessions.insert(session, state);
                return Some(Err((
                    session.job_id,
                    session.generation,
                    format!("InferenceRestart: {error}"),
                )));
            }
        }
        self.document_drains.push_back(UnifiedDocumentDrain {
            document: state.document,
            job_id: session.job_id,
            generation: session.generation,
            revision: result.document_revision,
            total_samples: result.total_samples,
            next_sequence: state.document_drain_start_sequence,
            ready_pending: !state.backend_ready_reported,
            progress_pending: true,
        });
        None
    }
}
