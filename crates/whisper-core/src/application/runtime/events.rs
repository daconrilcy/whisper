use super::*;

impl<P: ImportIoPort> ImportApplication<P> {
    pub(super) fn handle_event(&mut self, event: ImportEvent) -> Result<(), ApplicationError> {
        let event_identity = event.identity();
        if matches!(
            &event,
            ImportEvent::UnifiedDocumentPage { .. }
                | ImportEvent::UnifiedAudioPartsPage { .. }
                | ImportEvent::UnifiedPublicationArtifacts { .. }
        ) {
            return self.handle_unified_capture_event(event);
        }
        if self.unified_captures.contains_key(&event_identity) {
            return self.handle_unified_capture_event(event);
        }
        self.track_parakeet_capture_event(&event);
        if let ImportEvent::DuplicateImportFound(prompt) = &event
            && let Some(identity) = self.automatic_final_identity
            && identity.job_id == prompt.request.job_id
            && identity.generation == prompt.request.generation
        {
            self.mark_automatic_final_failed(
                identity,
                "Import final automatique déjà présent ; aucune sortie n'a été écrasée".into(),
            );
            return Ok(());
        }
        if self.handle_queue_event(&event)? {
            return Ok(());
        }
        let (job_id, generation) = event.identity();
        let identity = CommandIdentity { job_id, generation };
        self.invalidate_microphone_acquisition(&event, identity);
        if self.automatic_final_identity == Some(identity) && self.active != Some(identity) {
            return self.handle_automatic_final_event(event, identity);
        }
        let enqueue_failed = matches!(
            &event,
            ImportEvent::Failed { .. } if self.pending_enqueues.contains(&identity)
        );
        if self.active != Some(identity)
            && !(matches!(
                event,
                ImportEvent::Stopped { .. } | ImportEvent::Failed { .. }
            ) && self.stopping == Some(identity))
            && !enqueue_failed
        {
            return Ok(());
        }
        self.project_unified_import_event(&event, identity);
        if self.stopping == Some(identity)
            && self.request.is_some()
            && matches!(event, ImportEvent::Segment(_))
        {
            return Ok(());
        }
        self.route_active_event(event, identity)
    }

    fn route_active_event(
        &mut self,
        event: ImportEvent,
        identity: CommandIdentity,
    ) -> Result<(), ApplicationError> {
        match event {
            event @ (ImportEvent::MicrophoneGainApplied { .. }
            | ImportEvent::MicrophoneGainRejected { .. }
            | ImportEvent::MicrophoneTestStarted { .. }
            | ImportEvent::MicrophoneMeter(_)
            | ImportEvent::MicrophoneSpectrum(_)
            | ImportEvent::MicrophoneTestStopped { .. }) => {
                return self.handle_microphone_event(event, identity);
            }
            event @ (ImportEvent::DirectCaptureActive { .. }
            | ImportEvent::DirectConfirmed(_)
            | ImportEvent::DirectPublished { .. }
            | ImportEvent::Diagnostic { .. }
            | ImportEvent::LiveProvisional { .. }
            | ImportEvent::ParakeetLiveStarted { .. }
            | ImportEvent::ParakeetInferenceReady { .. }
            | ImportEvent::ParakeetHypothesis { .. }
            | ImportEvent::ParakeetConfirmed { .. }
            | ImportEvent::ParakeetUnresolved { .. }
            | ImportEvent::ParakeetResolved { .. }
            | ImportEvent::ParakeetCaptureProgress { .. }
            | ImportEvent::ParakeetInferenceUnavailable { .. }
            | ImportEvent::ParakeetAudioUnusable { .. }
            | ImportEvent::ParakeetCaptureStopped { .. }
            | ImportEvent::LiveCounters { .. }
            | ImportEvent::LiveFinalizing { .. }
            | ImportEvent::AwaitingCpuChoice { .. }
            | ImportEvent::LiveAttemptReset { .. }
            | ImportEvent::LiveAttemptStarting { .. }
            | ImportEvent::LiveRetryingOnCpu { .. }
            | ImportEvent::WindowFinished { .. }) => {
                return self.handle_live_event(event, identity);
            }
            ImportEvent::UnifiedDocumentPage { .. }
            | ImportEvent::UnifiedAudioPartsPage { .. }
            | ImportEvent::UnifiedPublicationArtifacts { .. } => {}
            ImportEvent::History(_) => {
                unreachable!("non-lifecycle events are handled before job admission")
            }
            ImportEvent::Queue(_)
            | ImportEvent::QueueAdmission { .. }
            | ImportEvent::DuplicateImportFound(_)
            | ImportEvent::SourceSnapshotRecoveryInspected(_)
            | ImportEvent::SourceSnapshotRecoveryCompleted { .. } => {
                unreachable!("queue/admission events are handled before job admission")
            }
            event @ (ImportEvent::ResumeCheckpoint { .. }
            | ImportEvent::Prepared(_)
            | ImportEvent::Ready { .. }
            | ImportEvent::Progress { .. }
            | ImportEvent::Segment(_)
            | ImportEvent::Persisted { .. }
            | ImportEvent::End { .. }
            | ImportEvent::Published { .. }
            | ImportEvent::Failed { .. }
            | ImportEvent::Stopped { .. }) => {
                return self.handle_import_event(event, identity);
            }
            // Handled before the job-identity gate above; it is a background
            // archive-history update and does not belong to an active import.
            ImportEvent::ReplacementCleanups(_) => return Ok(()),
        }
        Ok(())
    }
}
