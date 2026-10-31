use super::*;

pub(in crate::ui::model) fn unix_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| {
            duration.as_millis().min(u64::MAX as u128) as u64
        })
}

pub(in crate::ui::model) fn process_log_session() -> String {
    format!("process-{}", std::process::id())
}

pub(in crate::ui::model) fn live_log_session(
    job_id: JobId,
    generation: Generation,
    attempt: u64,
) -> String {
    format!(
        "job-{}-generation-{}-attempt-{attempt}",
        job_id.0,
        generation.get()
    )
}

pub(in crate::ui::model) fn record_first_render(
    identity: (JobId, Generation, u64),
    response: &egui::Response,
    rendered_identity: &mut Option<(JobId, Generation, u64)>,
    rendered_at_unix_ms: &mut Option<u64>,
) {
    if response.rect.width() > 0.0
        && response.rect.height() > 0.0
        && *rendered_identity != Some(identity)
    {
        *rendered_identity = Some(identity);
        *rendered_at_unix_ms = Some(unix_time_ms());
    }
}

impl<A: Application> DesktopApp<A> {
    pub(in crate::ui::model) fn record_reactivity_milestones(&mut self) {
        let Some(request) = self.live_request.as_ref() else {
            return;
        };
        if self.view.active_job.map(|(job_id, _)| job_id) != Some(request.job_id)
            || self.view.active_generation != Some(request.generation)
        {
            return;
        }
        let session = live_log_session(
            request.job_id,
            request.generation,
            self.live_attempt_counter,
        );
        let clicked_at = self.live_clicked_at_unix_ms;
        let milestones = [
            ("first_sample", self.view.direct_first_sample_at_unix_ms),
            (
                "first_wav_block_admitted",
                self.view.direct_first_admitted_at_unix_ms,
            ),
            ("engine_ready", self.view.direct_ready_at_unix_ms),
        ];
        for (event, timestamp) in milestones {
            if let Some(timestamp) = timestamp {
                self.reactivity_log
                    .record_once(session.clone(), event, timestamp, clicked_at);
            }
        }
        if let (Some(captured), Some(available)) = (
            self.view.provisional_captured_at_unix_ms,
            self.view.provisional_available_at_unix_ms,
        ) {
            let sequence = self.view.provisional_sequence.unwrap_or(0);
            self.reactivity_log.record_once(
                session.clone(),
                format!("provisional_available_sequence_{sequence}"),
                available,
                Some(captured),
            );
        }
        if let Some(rendered_at) = self.provisional_first_rendered_at_unix_ms {
            let sequence = self.view.provisional_sequence.unwrap_or(0);
            if self.provisional_render_identity
                == Some((request.job_id, request.generation, sequence))
            {
                let available = self.view.provisional_available_at_unix_ms;
                self.reactivity_log.record_once(
                    session.clone(),
                    format!("provisional_first_render_sequence_{sequence}"),
                    rendered_at,
                    available,
                );
            }
        }
    }
}
