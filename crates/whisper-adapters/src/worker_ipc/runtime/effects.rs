use super::*;

impl ImportRuntime {
    pub(in crate::worker_ipc) fn effect(
        &mut self,
        effect: ImportEffect,
        events: &mut VecDeque<ImportEvent>,
    ) {
        if matches!(
            &effect,
            ImportEffect::StartLive(_) | ImportEffect::StartUnifiedLive(_)
        ) {
            self.idle_worker = None;
        }
        let persisted_live_segment = match &effect {
            ImportEffect::PersistSegment(segment)
                if self.live.as_ref().is_some_and(|live| {
                    (live.request.job_id, live.request.generation)
                        == (segment.job_id, segment.generation)
                }) =>
            {
                Some((segment.job_id, segment.generation))
            }
            _ => None,
        };
        let unified_terminal_effect = match &effect {
            ImportEffect::PersistSegment(segment) => self.unified_archives.contains_key(
                &whisper_core::parakeet_scheduler::SchedulerSession {
                    job_id: segment.job_id,
                    generation: segment.generation,
                },
            ),
            ImportEffect::Publish {
                job_id, generation, ..
            }
            | ImportEffect::StopUnifiedLive {
                job_id, generation, ..
            } => self.unified_archives.contains_key(
                &whisper_core::parakeet_scheduler::SchedulerSession {
                    job_id: *job_id,
                    generation: *generation,
                },
            ),
            ImportEffect::Stop { job_id, generation } => self.native_unified_routes.contains_key(
                &whisper_core::parakeet_scheduler::SchedulerSession {
                    job_id: *job_id,
                    generation: *generation,
                },
            ),
            _ => false,
        };
        let mut result = match effect {
            ImportEffect::SetGpuCandidate {
                job_id,
                generation,
                candidate,
            } => self.set_gpu_candidate(job_id, generation, candidate),
            effect @ ImportEffect::ReadUnifiedDocumentPage { .. } => {
                self.read_unified_document_page_effect(effect, events)
            }
            effect @ ImportEffect::ReadUnifiedAudioPartsPage { .. } => {
                self.read_unified_audio_parts_page_effect(effect, events)
            }
            ImportEffect::StartMicrophoneTest {
                job_id,
                generation,
                microphone,
                gain_db,
            } => self.start_microphone_test(job_id, generation, microphone, gain_db, events),
            ImportEffect::StartDirectLive(request) => match self.start_direct_live(request, events)
            {
                LiveEffect::Return => return,
                LiveEffect::Continue(result) => result,
            },
            ImportEffect::StartUnifiedLive(request) => self.start_unified_live(request),
            ImportEffect::StartLive(request) => self.start_legacy_live(request),
            ImportEffect::SetMicrophoneGain {
                job_id,
                generation,
                target_identity,
                expected_revision,
                gain_db,
            } => self.set_microphone_gain(
                job_id,
                generation,
                target_identity,
                expected_revision,
                gain_db,
                events,
            ),
            ImportEffect::Enqueue(request) => self.enqueue(request, events),
            ImportEffect::ConfirmDuplicateImport { prompt, choice } => {
                match self.confirm_duplicate_import(prompt, choice, events) {
                    DuplicateEffect::Return => return,
                    DuplicateEffect::Continue(result) => result,
                }
            }
            ImportEffect::ReleaseSourceBinding {
                job_id,
                generation,
                destination,
            } => self
                .release_source_binding(job_id, generation, Path::new(&destination))
                .map_err(|error| (job_id, generation, error)),
            ImportEffect::InspectSourceSnapshotRecovery => {
                self.inspect_snapshot_recovery_effect(events)
            }
            ImportEffect::ConfirmSourceSnapshotRecovery => {
                self.confirm_snapshot_recovery_effect(events)
            }
            ImportEffect::RemoveQueued {
                job_id,
                generation,
                destination,
            } => self.remove_queued(job_id, generation, destination, events),
            ImportEffect::ProcessQueued(request) => self.process_queued(request, events),
            ImportEffect::ResumeInterrupted { previous, request } => {
                match self.resume_interrupted(previous, request, events) {
                    RecoveryEffect::Return => return,
                    RecoveryEffect::Continue(result) => result,
                }
            }
            ImportEffect::ScanQueue { destination } => self.scan_queue(destination, events),
            ImportEffect::Prepare(request) => self.prepare_effect(request, events),
            ImportEffect::StartWorker(request) => self.start_worker_effect(request),
            ImportEffect::StartAutomaticFinalTranscription(automatic) => {
                self.start_automatic_final_effect(automatic, events)
            }
            ImportEffect::ScanHistory { destination } => {
                self.scan_history_effect(destination, events)
            }
            ImportEffect::ResumeReplacementCleanup {
                destination,
                cleanup_id,
            } => self.resume_replacement_cleanup(destination, cleanup_id, events),
            ImportEffect::ResolveImportPublication {
                destination,
                job_id,
                generation,
            } => self.resolve_import_publication_effect(destination, job_id, generation, events),
            ImportEffect::ResolveLivePublication(request) => {
                self.resolve_live_publication_effect(request, events)
            }
            ImportEffect::PersistSegment(segment) => self.persist_segment_effect(segment, events),
            ImportEffect::Publish {
                job_id,
                generation,
                last_sequence,
            } => self.publish(job_id, generation, last_sequence, events),
            ImportEffect::FinishLiveOnCpu { job_id, generation } => {
                self.finish_live_on_cpu_effect(job_id, generation, events)
            }
            ImportEffect::StopUnifiedLive { job_id, generation } => {
                self.stop_unified_live_effect(job_id, generation, events)
            }
            ImportEffect::Stop { job_id, generation } => {
                self.stop_effect(job_id, generation, events)
            }
            ImportEffect::StopMicrophoneTest { job_id, generation } => {
                self.stop_microphone_test(job_id, generation, events)
            }
        };
        if persisted_live_segment.is_some() {
            self.live_pending_persistence = self.live_pending_persistence.saturating_sub(1);
            if self.live_pending_persistence == 0
                && let Some(window) = self.deferred_live_window_finished.take()
            {
                match self.confirm_live_window(window) {
                    Ok(event) => events.push_back(event),
                    Err(failure) => result = Err(failure),
                }
            }
        }
        if let Err((job, generation, mut message)) = result {
            if let Some(automatic) = self.automatic_final_outputs.get(&(job, generation.get()))
                && let Err(status_error) = set_automatic_final_manifest_status(
                    automatic,
                    "failed_or_unavailable",
                    None,
                    Some(&message),
                )
            {
                message.push_str(&format!(
                    "; persistance du verdict d'échec dans le JSON compagnon impossible: {status_error}"
                ));
            }
            self.automatic_final_outputs
                .remove(&(job, generation.get()));
            self.gpu_candidates.remove(&(job, generation.get()));
            let message = if let Some(direct) = self.direct_live.as_mut()
                && direct.identity() == (job, generation)
            {
                let message = direct.fail(message);
                self.direct_live = None;
                message
            } else {
                message
            };
            let mut deferred_live_failure = false;
            let mut deferred_unified_failure = false;
            if unified_terminal_effect {
                let session = whisper_core::parakeet_scheduler::SchedulerSession {
                    job_id: job,
                    generation,
                };
                if let Some(state) = self.unified_sessions.get_mut(&session) {
                    state.failure = Some(message.clone());
                    let _ = self.parakeet_scheduler.abort_session(session);
                    deferred_unified_failure = true;
                } else {
                    self.release_unified_terminal_state(job, generation);
                }
            }
            if let Some(live) = self.live.as_mut()
                && (live.request.job_id, live.request.generation) == (job, generation)
            {
                deferred_live_failure = true;
                live.stop_capture();
                let _ = live.staging.drain();
                live.failure = Some(message.clone());
                if let Some(transport) = self.transport.as_mut() {
                    let _ = transport.try_request(WorkerCommand::Stop {
                        job_id: job,
                        generation,
                    });
                }
            }
            if !deferred_live_failure && !deferred_unified_failure {
                events.push_back(ImportEvent::Failed {
                    job_id: job,
                    generation,
                    message,
                });
            }
        }
    }
}
