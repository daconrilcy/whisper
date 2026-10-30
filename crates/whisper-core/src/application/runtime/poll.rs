use super::*;

impl<P: ImportIoPort> ImportApplication<P> {
    pub(super) fn poll_once(&mut self) -> Result<(), ApplicationError> {
        self.retry_pending_unified_capture_segments()?;
        if !self.retry_pending_segment()? {
            self.update_close_ready();
            return Ok(());
        }
        let _ = self.retry_automatic_final_segment()?;
        for _ in 0..MAX_EVENTS_PER_REFRESH {
            let polled = match self.io.poll() {
                Ok(polled) => polled,
                Err(error) => {
                    self.view.microphone_acquisition = None;
                    self.view.microphone_spectrum = None;
                    return Err(self.fail(error));
                }
            };
            let Some(event) = polled else {
                self.update_close_ready();
                return Ok(());
            };
            let became_ready = matches!(&event, ImportEvent::Ready { .. });
            let event_identity = event.identity();
            let unified_capture_event = self.unified_captures.contains_key(&event_identity)
                || self
                    .view
                    .unified_parakeet_sessions
                    .iter()
                    .any(|session| (session.job_id, session.generation) == event_identity);
            trace_application_event(&event);
            self.handle_event(event)?;
            if !self.pending_segments.is_empty() && !unified_capture_event {
                self.update_close_ready();
                return Ok(());
            }
            if became_ready {
                self.update_close_ready();
                return Ok(());
            }
        }
        self.update_close_ready();
        Ok(())
    }
}
