use super::*;

impl ImportRuntime {
    #[warn(clippy::too_many_lines, clippy::cognitive_complexity)]
    pub(super) fn poll_stopped_pending_live_recovery(
        &mut self,
        event: &WorkerEvent,
        job_id: JobId,
        generation: Generation,
    ) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
        if !matches!(event, WorkerEvent::Stopped { .. })
            || self.live.is_some()
            || !self
                .pending_live
                .as_ref()
                .is_some_and(|pending| pending.recovery != LiveRecoveryState::None)
        {
            return None;
        }

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
        Some(
            match self.start_cpu_pending_live_attempt(job_id, generation) {
                Ok(event) => Ok(event),
                Err(error) => Err(error),
            },
        )
    }

    #[warn(clippy::too_many_lines, clippy::cognitive_complexity)]
    pub(super) fn poll_stopped_import_auto_retry(
        &mut self,
        event: &WorkerEvent,
        job_id: JobId,
        generation: Generation,
    ) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
        if !matches!(event, WorkerEvent::Stopped { .. })
            || self.live.is_some()
            || !self.active.as_ref().is_some_and(|active| {
                active.recovery == LiveRecoveryState::AutoRetryPending
                    && active.import_request.is_some()
            })
        {
            return None;
        }

        let request = self
            .active
            .as_ref()
            .and_then(|active| active.import_request.clone());
        self.active = None;
        self.transport = None;
        if self.publication_gate.is_cancelled() {
            return Some(Ok(ImportEvent::Stopped { job_id, generation }));
        }
        Some(
            match request.map(|request| self.start_worker_with_cpu_override(request, true)) {
                Some(Ok(())) => Ok(ImportEvent::LiveAttemptStarting { job_id, generation }),
                Some(Err(error)) => Err(error),
                // Kept to preserve the original branch even though its predicate requires Some.
                None => Err((
                    job_id,
                    generation,
                    "ProtocolMismatch: retry request unavailable".into(),
                )),
            },
        )
    }
}
