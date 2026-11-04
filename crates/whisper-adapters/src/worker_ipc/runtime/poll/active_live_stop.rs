use super::*;

impl ImportRuntime {
    #[warn(clippy::too_many_lines, clippy::cognitive_complexity)]
    pub(super) fn poll_stopped_active_live(
        &mut self,
        event: &WorkerEvent,
        job_id: JobId,
        generation: Generation,
    ) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
        if !matches!(event, WorkerEvent::Stopped { .. }) || self.live.is_none() {
            return None;
        }

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

        Some(self.finish_stopped_active_live(job_id, generation))
    }

    #[warn(clippy::too_many_lines, clippy::cognitive_complexity)]
    fn finish_stopped_active_live(
        &mut self,
        job_id: JobId,
        generation: Generation,
    ) -> Result<ImportEvent, (JobId, Generation, String)> {
        if let Err(error) =
            crate::archive::campaign_gate("pcm.before_final_drain", job_id.0, generation.get(), 0)
        {
            return Err((job_id, generation, error));
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
            return Err((job_id, generation, error));
        }
        if let Some(Err(error)) = final_drain {
            if let Some(live) = self.live.as_mut() {
                live.stop_capture();
            }
            self.live = None;
            self.live_archive = None;
            self.transport = None;
            self.active = None;
            return Err((
                job_id,
                generation,
                format!("StorageUnavailable: final audio sync: {error}"),
            ));
        }
        if let Err(error) =
            crate::archive::campaign_gate("pcm.after_final_drain", job_id.0, generation.get(), 0)
        {
            return Err((job_id, generation, error));
        }

        let (durable, confirmed, pcm_sha256) = self.live.as_ref().map_or((0, 0, [0; 32]), |live| {
            (
                live.staging.durable_samples(),
                live.confirmed_fragment_end,
                live.staging.checksum(),
            )
        });
        let recovery = self.live.as_ref().map(|live| live.recovery);
        if matches!(
            recovery,
            Some(LiveRecoveryState::AutoRetryPending | LiveRecoveryState::AwaitingChoicePending)
        ) {
            return self.finish_stopped_active_live_recovery(job_id, generation, recovery, durable);
        }
        self.publish_stopped_active_live(job_id, generation, durable, confirmed, pcm_sha256)
    }

    #[warn(clippy::too_many_lines, clippy::cognitive_complexity)]
    fn finish_stopped_active_live_recovery(
        &mut self,
        job_id: JobId,
        generation: Generation,
        recovery: Option<LiveRecoveryState>,
        durable: u64,
    ) -> Result<ImportEvent, (JobId, Generation, String)> {
        let barrier_valid = self.live.as_ref().is_some_and(|live| {
            live.barrier_m
                .is_some_and(|watermark| watermark <= live.inference_sequence.saturating_sub(1))
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
            return Err((
                job_id,
                generation,
                "StorageCorrupt: retry barrier did not settle a valid confirmed prefix".into(),
            ));
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
            return Err((job_id, generation, error));
        }
        self.transport = None;
        self.active = None;
        if self.publication_gate.is_cancelled() {
            if let Some(live) = self.live.as_mut() {
                live.stop_capture();
            }
            self.live = None;
            self.live_archive = None;
            return Ok(ImportEvent::Stopped { job_id, generation });
        }
        if recovery == Some(LiveRecoveryState::AwaitingChoicePending) {
            if let Some(live) = self.live.as_mut() {
                live.recovery = LiveRecoveryState::AwaitingChoice;
                live.inference_admission_closed = true;
            }
            return Ok(ImportEvent::AwaitingCpuChoice { job_id, generation });
        }
        let stop_was_latched = self.live.as_ref().is_some_and(|live| live.stop_requested)
            || self.publication_gate.is_cancelled();
        if stop_was_latched {
            if let Some(live) = self.live.as_mut() {
                live.stop_capture();
            }
            self.live = None;
            self.live_archive = None;
            return Ok(ImportEvent::Stopped { job_id, generation });
        }
        self.start_cpu_live_attempt(job_id, generation)
    }

    #[warn(clippy::too_many_lines, clippy::cognitive_complexity)]
    fn publish_stopped_active_live(
        &mut self,
        job_id: JobId,
        generation: Generation,
        durable: u64,
        confirmed: u64,
        pcm_sha256: [u8; 32],
    ) -> Result<ImportEvent, (JobId, Generation, String)> {
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
            return Err((job_id, generation, error));
        }
        self.live = None;
        self.live_archive = None;
        self.transport = None;
        self.active = None;
        Ok(ImportEvent::Published { job_id, generation })
    }
}
