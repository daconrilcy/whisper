use super::*;

impl ImportRuntime {
    pub(in crate::worker_ipc) fn prepare(
        &mut self,
        mut request: ImportRequest,
        candidate: Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
        events: &mut VecDeque<ImportEvent>,
    ) -> Result<(), (JobId, Generation, String)> {
        let id = (request.job_id, request.generation);
        let mut native_route = None;
        if request.strategy
            == whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
        {
            let session = whisper_core::parakeet_scheduler::SchedulerSession {
                job_id: id.0,
                generation: id.1,
            };
            if self.unified_admission_full_for(session) {
                return Err((
                    id.0,
                    id.1,
                    "SchedulerCapacity: two unified sessions are already admitted".into(),
                ));
            }
            native_route = self
                .native_route_for_unified_import(&request, candidate.as_ref())
                .map_err(|error| (id.0, id.1, error))?;
        }
        self.ensure_source_binding(&mut request)
            .map_err(|error| (id.0, id.1, error))?;
        crate::archive::assert_campaign_archive(Path::new(&request.destination))
            .map_err(|error| (id.0, id.1, error))?;
        if request_is_parakeet(&request)
            && request.config.compute == whisper_core::ComputeChoice::Gpu
        {
            let candidate = candidate.as_ref().ok_or((
                id.0,
                id.1,
                "BackendMismatch: selected GPU has no current identity proof".into(),
            ))?;
            if candidate.generation != request.generation.get() {
                return Err((
                    id.0,
                    id.1,
                    "BackendMismatch: selected GPU identity belongs to a stale generation".into(),
                ));
            }
            if native_route.is_none() && !self.is_controlled_bounded_test(&request) {
                let paths = self.gpu_parakeet_paths.as_ref().ok_or((
                    id.0,
                    id.1,
                    "BackendUnavailable: worker GPU Test Parakeet non configuré".into(),
                ))?;
                let profile = whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS;
                let key = paths
                    .gpu_qualification_key(profile)
                    .map_err(|error| (id.0, id.1, error))?;
                if !gpu_qualification_candidate_allowed(&key) {
                    return Err((
                        id.0,
                        id.1,
                        "ModelNotQualified: GPU Test key is not admitted".into(),
                    ));
                }
            }
            if native_route.is_none() {
                crate::parakeet_import::expected_gpu_for_candidate(candidate).map_err(|error| {
                    (
                        id.0,
                        id.1,
                        format!("BackendMismatch: GPU identity validation failed: {error}"),
                    )
                })?;
            }
        }
        if self.publication_gate.is_cancelled() {
            events.push_back(ImportEvent::Stopped {
                job_id: id.0,
                generation: id.1,
            });
            return Ok(());
        }
        let mut archive = ArchiveStore::new(&request.destination);
        let qualification = archive_qualification_for_request(
            &request,
            self.parakeet_paths.as_ref(),
            self.gpu_parakeet_paths.as_ref(),
            candidate.as_ref(),
        );
        let prepared = if let Some(binding) = self.source_bindings.get(&(id.0, id.1.get())) {
            archive.prepare_with_source_binding(request, None, qualification, binding)
        } else if self.require_snapshot_registry {
            Err("SourceSnapshotUnavailable: binding missing before prepare".into())
        } else {
            archive.prepare_with_qualification(request, None, qualification)
        };
        match prepared {
            Ok(prepared) => {
                self.archive = archive;
                acceptance_barrier("prepare", Path::new(&prepared.destination))
                    .map_err(|error| (id.0, id.1, error))?;
                if self.publication_gate.is_cancelled() {
                    events.push_back(ImportEvent::Stopped {
                        job_id: id.0,
                        generation: id.1,
                    });
                    return Ok(());
                }
                if let Some(native_route) = native_route {
                    self.register_prepared_native_route(
                        whisper_core::parakeet_scheduler::SchedulerSession {
                            job_id: id.0,
                            generation: id.1,
                        },
                        native_route,
                    )
                    .map_err(|error| (id.0, id.1, error))?;
                }
                events.push_back(ImportEvent::Prepared(prepared));
                Ok(())
            }
            Err(message) => Err((id.0, id.1, message)),
        }
    }
    #[cfg(test)]
    pub(in crate::worker_ipc) fn start_worker(
        &mut self,
        request: whisper_core::ports::ImportRequest,
    ) -> Result<(), (JobId, Generation, String)> {
        self.start_worker_with_selection(request, None)
    }

    pub(in crate::worker_ipc) fn start_worker_with_selection(
        &mut self,
        request: whisper_core::ports::ImportRequest,
        selection: Option<Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>>,
    ) -> Result<(), (JobId, Generation, String)> {
        if let Some(automatic) = self
            .automatic_final_outputs
            .get(&(request.job_id, request.generation.get()))
            && let Err(error) =
                set_automatic_final_manifest_status(automatic, "running", None, None)
        {
            return Err((request.job_id, request.generation, error));
        }
        let request = match self
            .source_bindings
            .get(&(request.job_id, request.generation.get()))
        {
            Some(binding) => binding
                .apply_to_request(&request)
                .map_err(|error| (request.job_id, request.generation, error))?,
            None => request,
        };
        match selection {
            Some(None) if request.config.compute == whisper_core::ComputeChoice::Gpu => Err((
                request.job_id,
                request.generation,
                "BackendMismatch: selected GPU has no current runtime identity proof".into(),
            )),
            Some(None) if request.config.compute == whisper_core::ComputeChoice::Auto => {
                self.auto_fallback_reason =
                    Some("GPU candidate proof unavailable before processing".into());
                self.start_worker_with_cpu_override(request, true)
            }
            Some(Some(candidate)) => self.start_worker_with_candidate(request, Some(candidate)),
            Some(None) => self.start_worker_with_candidate(request, None),
            None => self.start_worker_with_candidate(request, None),
        }
    }

    pub(in crate::worker_ipc) fn persist_segment(
        &mut self,
        segment: WorkerSegment,
        events: &mut VecDeque<ImportEvent>,
    ) -> Result<(), (JobId, Generation, String)> {
        let id = (segment.job_id, segment.generation);
        let unified = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: id.0,
            generation: id.1,
        };
        if let Some(mut archive) = self.unified_archives.remove(&unified) {
            let result = archive.persist_segment(&segment);
            self.unified_archives.insert(unified, archive);
            let sequence = result.map_err(|error| (id.0, id.1, error))?;
            events.push_back(ImportEvent::Persisted {
                job_id: id.0,
                generation: id.1,
                sequence,
            });
            return Ok(());
        }
        if self.live.is_some() {
            let durable = self
                .live
                .as_ref()
                .map_or(0, |live| live.staging.durable_samples());
            let confirmed = self
                .live_archive
                .as_mut()
                .ok_or((
                    id.0,
                    id.1,
                    "StorageUnavailable: live archive is missing".into(),
                ))?
                .persist_segment(&segment, durable)
                .map_err(|e| (id.0, id.1, e))?;
            if let Some(live) = self.live.as_mut() {
                live.confirmed_fragment_end = confirmed;
            }
            if let Some(progress) = self.progress.as_mut() {
                progress.observe(
                    ProgressPhase::Draining,
                    confirmed,
                    std::time::Instant::now(),
                );
            }
            events.push_back(ImportEvent::Persisted {
                job_id: id.0,
                generation: id.1,
                sequence: segment.segment_id.0,
            });
            return Ok(());
        }
        let sequence = self
            .archive
            .persist_segment(&segment)
            .map_err(|e| (id.0, id.1, e))?;
        if let Some(progress) = self.progress.as_mut() {
            progress.observe(ProgressPhase::Draining, sequence, std::time::Instant::now());
        }
        events.push_back(ImportEvent::Persisted {
            job_id: id.0,
            generation: id.1,
            sequence,
        });
        Ok(())
    }

    pub(in crate::worker_ipc) fn publish(
        &mut self,
        job: JobId,
        generation: Generation,
        count: u64,
        events: &mut VecDeque<ImportEvent>,
    ) -> Result<(), (JobId, Generation, String)> {
        let session = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: job,
            generation,
        };
        let publication_gate = self
            .unified_publication_gates
            .get(&session)
            .cloned()
            .unwrap_or_else(|| Arc::clone(&self.publication_gate));
        if let Some(archive) = self.unified_archives.remove(&session) {
            let previous_archive = std::mem::replace(&mut self.archive, archive);
            let result = self.publish_inner(job, generation, count, events, &publication_gate);
            let session_archive = std::mem::replace(&mut self.archive, previous_archive);
            if result.is_err() {
                self.unified_archives.insert(session, session_archive);
            } else {
                self.unified_publication_gates.remove(&session);
                self.unified_capture_sessions.remove(&session);
            }
            return result;
        }
        let result = self.publish_inner(job, generation, count, events, &publication_gate);
        if result.is_ok() {
            self.unified_publication_gates.remove(&session);
            self.unified_capture_sessions.remove(&session);
        }
        result
    }

    pub(in crate::worker_ipc) fn publish_inner(
        &mut self,
        job: JobId,
        generation: Generation,
        count: u64,
        events: &mut VecDeque<ImportEvent>,
        publication_gate: &PublicationGate,
    ) -> Result<(), (JobId, Generation, String)> {
        let destination = self.archive.root_path();
        let _timing_context = TimingContextGuard::enter(
            "publish-import",
            format!("{:032x}-{}", job.0, generation.get()),
            &destination,
        );
        let result = (|| {
            if let Some(progress) = self.progress.as_mut() {
                progress.close(ProgressPhase::Draining);
                progress.close(ProgressPhase::Finalizing);
            }
            self.archive
                .prepare_publish(job, generation, count)
                .map_err(|e| (job, generation, e))?;
            acceptance_barrier("publish-before-commit", self.archive.root_path().as_path())
                .map_err(|error| (job, generation, error))?;
            publication_gate
                .commit(|| {
                    self.archive.commit_publish(job, generation)?;
                    acceptance_barrier("publish-after-commit", self.archive.root_path().as_path())
                })
                .map_err(|e| (job, generation, e))?;
            let root = self.archive_root();
            if root
                .join("transcriptions")
                .join(format!("{:032x}", job.0))
                .join(generation.get().to_string())
                .join("parakeet-unified/document")
                .is_dir()
            {
                let result = ArchiveStore::published_unified_files(&root, job, generation);
                events.push_back(ImportEvent::UnifiedPublicationArtifacts {
                    job_id: job,
                    generation,
                    result,
                });
            }
            if let Ok(mut queue) = crate::queue_store::QueueStore::open(&root)
                && queue.remove(job, generation).is_ok()
            {
                events.push_back(ImportEvent::Queue(queue.entries()));
            }
            let first = ArchiveStore::scan(&root).map_err(|e| (job, generation, e))?;
            let second = ArchiveStore::scan(&root).map_err(|e| (job, generation, e))?;
            if first != second
                || !first.iter().any(|entry| {
                    entry.job_id == job && entry.generation == generation && entry.complete
                })
            {
                return Err((
                    job,
                    generation,
                    "StorageCorrupt: archive scan did not verify stable published generation"
                        .into(),
                ));
            }
            self.release_source_binding(job, generation, &root)
                .map_err(|error| (job, generation, error))?;
            if let Some(automatic) = self.automatic_final_outputs.get(&(job, generation.get())) {
                let entry = first
                    .iter()
                    .find(|entry| {
                        entry.job_id == job && entry.generation == generation && entry.complete
                    })
                    .ok_or_else(|| {
                        (
                            job,
                            generation,
                            "ArchiveEntryMissing: import final introuvable".into(),
                        )
                    })?;
                let basename = entry.export_basename.as_deref().ok_or_else(|| {
                    (
                        job,
                        generation,
                        "ArchiveExportMissing: export final absent".into(),
                    )
                })?;
                let archive_exports = root.join("exports");
                publish_automatic_final_exports(
                    &archive_exports.join(format!("{basename}.txt")),
                    &archive_exports.join(format!("{basename}.srt")),
                    automatic,
                )
                .map_err(|error| (job, generation, error))?;
                self.automatic_final_outputs
                    .remove(&(job, generation.get()));
            }
            events.push_back(ImportEvent::Published {
                job_id: job,
                generation,
            });
            let cleanups = ArchiveStore::replacement_cleanups(&root).unwrap_or_else(|error| {
                vec![whisper_core::ports::ReplacementCleanupInfo {
                    cleanup_id: "unavailable".into(),
                    successor: TreatmentIdentity {
                        destination: root.to_string_lossy().into_owned(),
                        job_id: job,
                        generation,
                    },
                    remaining: Vec::new(),
                    preserved: Vec::new(),
                    last_error: Some(error),
                }]
            });
            events.push_back(ImportEvent::ReplacementCleanups(cleanups));
            Ok(())
        })();

        match result {
            Ok(()) => {
                if let Some(mut worker) = self.pending_idle_worker.take() {
                    worker.expires_at = Instant::now() + WORKER_IDLE_RETENTION;
                    self.idle_worker = Some(worker);
                }
                Ok(())
            }
            Err(error) => {
                // End alone does not start the retention period: an unpublished
                // result must not leave a model resident or allow a second worker.
                self.pending_idle_worker = None;
                self.idle_worker = None;
                if let Some(automatic) = self.automatic_final_outputs.get(&(job, generation.get()))
                {
                    let _ = set_automatic_final_manifest_status(
                        automatic,
                        "failed_or_unavailable",
                        None,
                        Some(&error.2),
                    );
                }
                self.automatic_final_outputs
                    .remove(&(job, generation.get()));
                Err(error)
            }
        }
    }
    pub(in crate::worker_ipc) fn archive_root(&self) -> PathBuf {
        self.archive.root_path()
    }
    pub(in crate::worker_ipc) fn stop(
        &mut self,
        job: JobId,
        generation: Generation,
        events: &mut VecDeque<ImportEvent>,
    ) -> Result<(), (JobId, Generation, String)> {
        if let Some(live) = self.live.as_mut() {
            if (live.request.job_id, live.request.generation) != (job, generation) {
                return Err((
                    job,
                    generation,
                    "StaleResponse: live Stop targets another passage".into(),
                ));
            }
            if live.recovery == LiveRecoveryState::AwaitingChoice && self.active.is_none() {
                self.live = None;
                self.live_archive = None;
                self.transport = None;
                events.push_back(ImportEvent::Stopped {
                    job_id: job,
                    generation,
                });
                return Ok(());
            }
            live.stop_capture();
            if live.failure.is_none() {
                while let Some(block) = live.capture.try_next_block() {
                    let samples = live
                        .converter
                        .push(&block)
                        .map_err(|e| (job, generation, e))?;
                    live.capture
                        .recycle_block(block)
                        .map_err(|e| (job, generation, e))?;
                    live.frame_pending.extend(samples);
                    while live.frame_pending.len() >= 320 {
                        let frame: Vec<i16> = live.frame_pending.drain(..320).collect();
                        live.captured += 320;
                        if live
                            .vad
                            .is_speech(&frame)
                            .map_err(|e| (job, generation, e))?
                        {
                            live.speech += 320;
                        }
                        live.staging
                            .append_frame(frame)
                            .map_err(|e| (job, generation, format!("StorageUnavailable: {e}")))?;
                        live.admitted += 320;
                    }
                }
                let tail = live.converter.finish().map_err(|e| (job, generation, e))?;
                live.frame_pending.extend(tail);
                while live.frame_pending.len() >= 320 {
                    let frame: Vec<i16> = live.frame_pending.drain(..320).collect();
                    live.captured += 320;
                    if live
                        .vad
                        .is_speech(&frame)
                        .map_err(|e| (job, generation, e))?
                    {
                        live.speech += 320;
                    }
                    live.staging
                        .append_frame(frame)
                        .map_err(|e| (job, generation, format!("StorageUnavailable: {e}")))?;
                    live.admitted += 320;
                }
                if !live.frame_pending.is_empty() {
                    let valid = std::mem::take(&mut live.frame_pending);
                    let valid_len = valid.len();
                    let mut vad_frame = [0i16; 320];
                    vad_frame[..valid_len].copy_from_slice(&valid);
                    if live
                        .vad
                        .is_speech(&vad_frame)
                        .map_err(|e| (job, generation, e))?
                    {
                        live.speech += valid_len as u64;
                    }
                    live.captured += valid_len as u64;
                    live.staging.append_tail(valid).map_err(|e| {
                        (
                            job,
                            generation,
                            format!("StorageUnavailable: final PCM tail: {e}"),
                        )
                    })?;
                    live.admitted += valid_len as u64;
                }
            }
            crate::archive::campaign_gate("pcm.before_drain", job.0, generation.get(), 0)
                .map_err(|error| (job, generation, error))?;
            let drain_result = live.staging.drain();
            crate::archive::record_campaign_operation_result(
                "pcm.before_drain",
                job.0,
                generation.get(),
                0,
                &drain_result,
                None,
            )
            .map_err(|error| (job, generation, error))?;
            drain_result.map_err(|e| {
                (
                    job,
                    generation,
                    format!("StorageUnavailable: drain failed: {e}"),
                )
            })?;
            crate::archive::campaign_gate("pcm.after_drain", job.0, generation.get(), 0)
                .map_err(|error| (job, generation, error))?;
            let durable_samples = live.staging.durable_samples();
            live.last_reported_durable = durable_samples;
            events.push_back(ImportEvent::LiveCounters {
                job_id: job,
                generation,
                captured_samples: live.captured,
                admitted_samples: live.admitted,
                audio_durable_samples: durable_samples,
                confirmed_fragment_samples: live.confirmed_fragment_end,
                speech_samples: live.speech,
                diagnostic: None,
            });
            live.stop_requested = true;
            return Ok(());
        }
        if !self.publication_gate.is_cancelled() {
            events.push_back(ImportEvent::Stopped {
                job_id: job,
                generation,
            });
            return Ok(());
        }
        let Some(active) = self.active.as_ref() else {
            events.push_back(ImportEvent::Stopped {
                job_id: job,
                generation,
            });
            return Ok(());
        };
        if active.job_id != job || active.generation != generation {
            events.push_back(ImportEvent::Stopped {
                job_id: job,
                generation,
            });
            return Ok(());
        }
        let Some(transport) = self.transport.as_mut() else {
            events.push_back(ImportEvent::Stopped {
                job_id: job,
                generation,
            });
            return Ok(());
        };
        transport
            .request(WorkerCommand::Stop {
                job_id: job,
                generation,
            })
            .map_err(|e| (job, generation, format!("WorkerExited: {e:?}")))
    }
    pub(in crate::worker_ipc) fn set_gpu_candidate(
        &mut self,
        job_id: JobId,
        generation: Generation,
        candidate: Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
    ) -> Result<(), (JobId, Generation, String)> {
        self.gpu_candidates
            .insert((job_id, generation.get()), candidate);
        Ok(())
    }
    pub(in crate::worker_ipc) fn read_unified_document_page_effect(
        &mut self,
        effect: ImportEffect,
        events: &mut VecDeque<ImportEvent>,
    ) -> Result<(), (JobId, Generation, String)> {
        let ImportEffect::ReadUnifiedDocumentPage {
            job_id,
            generation,
            request_id,
            destination,
            expected_revision,
            first_sequence,
            limit,
        } = effect
        else {
            unreachable!("pagination handler received a different effect");
        };
        let result = ArchiveStore::read_unified_document_page(
            Path::new(&destination),
            job_id,
            generation,
            expected_revision,
            first_sequence,
            limit,
        );
        events.push_back(ImportEvent::UnifiedDocumentPage {
            job_id,
            generation,
            request_id,
            result,
        });
        Ok(())
    }
    pub(in crate::worker_ipc) fn read_unified_audio_parts_page_effect(
        &mut self,
        effect: ImportEffect,
        events: &mut VecDeque<ImportEvent>,
    ) -> Result<(), (JobId, Generation, String)> {
        let ImportEffect::ReadUnifiedAudioPartsPage {
            job_id,
            generation,
            request_id,
            destination,
            expected_revision,
            offset,
            limit,
        } = effect
        else {
            unreachable!("pagination handler received a different effect");
        };
        let result = ArchiveStore::read_unified_audio_parts_page(
            Path::new(&destination),
            job_id,
            generation,
            expected_revision,
            offset,
            limit,
        );
        events.push_back(ImportEvent::UnifiedAudioPartsPage {
            job_id,
            generation,
            request_id,
            result,
        });
        Ok(())
    }
    pub(in crate::worker_ipc) fn start_worker_effect(
        &mut self,
        request: ImportRequest,
    ) -> Result<(), (JobId, Generation, String)> {
        if request.strategy
            == whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
        {
            let identity = (request.job_id, request.generation);
            #[cfg(test)]
            let controlled_worker_available = self.controlled_transcriber.is_some();
            #[cfg(not(test))]
            let controlled_worker_available = false;
            let session = whisper_core::parakeet_scheduler::SchedulerSession {
                job_id: identity.0,
                generation: identity.1,
            };
            let native_route_available = self.native_unified_routes.contains_key(&session)
                && self.native_unified_candidate_enabled;
            if self.unified_executor.is_none()
                && !controlled_worker_available
                && !native_route_available
            {
                Err((identity.0, identity.1,
                            "ModelNotQualified: unified strategy requires a controlled worker or explicit native candidate composition".into()))
            } else {
                match self.bind_import_request(&request) {
                    Ok(bound) => self
                        .start_controlled_unified_import(&bound)
                        .map_err(|error| (identity.0, identity.1, error)),
                    Err(error) => Err((identity.0, identity.1, error)),
                }
            }
        } else {
            let candidate = self
                .gpu_candidates
                .remove(&(request.job_id, request.generation.get()));
            match self.bind_import_request(&request) {
                Ok(request) => self.start_worker_with_selection(request, candidate),
                Err(error) => Err((
                    request.job_id,
                    request.generation,
                    format!("Modèle indisponible : {error}"),
                )),
            }
        }
    }
    pub(in crate::worker_ipc) fn start_automatic_final_effect(
        &mut self,
        automatic: whisper_core::ports::AutomaticFinalImport,
        events: &mut VecDeque<ImportEvent>,
    ) -> Result<(), (JobId, Generation, String)> {
        let request = automatic.request.clone();
        let identity = (request.job_id, request.generation);
        let map_identity = (identity.0, identity.1.get());
        if self.publication_gate.is_cancelled() {
            events.push_back(ImportEvent::Stopped {
                job_id: identity.0,
                generation: identity.1,
            });
            Ok(())
        } else if self.automatic_final_outputs.contains_key(&map_identity)
            || Path::new(&automatic.outputs.txt_path).exists()
            || Path::new(&automatic.outputs.srt_path).exists()
        {
            Err((
                request.job_id,
                request.generation,
                "AutomaticFinalExists: une sortie automatique existe déjà".into(),
            ))
        } else {
            (|| {
                let destination = Path::new(&request.destination);
                let queue = inspect_automatic_final_queue(destination)
                    .map_err(|error| (identity.0, identity.1, error))?;
                let identity_in_use = crate::archive::ArchiveStore::job_identity_exists(
                    destination,
                    request.job_id,
                    request.generation,
                ) || queue.iter().any(|entry| {
                    entry.request.job_id == request.job_id
                        && entry.request.generation == request.generation
                });
                if identity_in_use {
                    return Err((
                        identity.0,
                        identity.1,
                        "AutomaticFinalIdentityExists: identifiant déjà utilisé".into(),
                    ));
                }
                match set_automatic_final_manifest_status(&automatic, "pending", None, None) {
                    Err(error) => Err((identity.0, identity.1, error)),
                    Ok(()) => {
                        self.automatic_final_outputs
                            .insert(map_identity, automatic.clone());
                        self.gpu_candidates
                            .insert((identity.0, identity.1.get()), automatic.gpu_candidate);
                        match self.bind_import_request(&request) {
                            Ok(request) => {
                                let candidate = self
                                    .gpu_candidates
                                    .get(&(request.job_id, request.generation.get()))
                                    .and_then(Clone::clone);
                                self.prepare(request, candidate, events)
                            }
                            Err(error) => {
                                self.gpu_candidates.remove(&(identity.0, identity.1.get()));
                                Err((identity.0, identity.1, error))
                            }
                        }
                    }
                }
            })()
        }
    }
    pub(in crate::worker_ipc) fn scan_history_effect(
        &mut self,
        destination: String,
        events: &mut VecDeque<ImportEvent>,
    ) -> Result<(), (JobId, Generation, String)> {
        let root = PathBuf::from(destination);
        let reconciliation = ArchiveStore::reconcile_publications(&root);
        let mut history = scan_archive_history(&root);
        if let Err(error) = reconciliation {
            history.push(ArchiveHistoryItem {
                job_id: JobId(0),
                generation: Generation::first(),
                complete: false,
                source_sha256: String::new(),
                diagnostic: Some(error),
                live_recovery: None,
            });
        }
        events.push_back(ImportEvent::History(history));
        match ArchiveStore::replacement_cleanups(&root) {
            Ok(cleanups) => events.push_back(ImportEvent::ReplacementCleanups(cleanups)),
            Err(error) => events.push_back(ImportEvent::ReplacementCleanups(vec![
                whisper_core::ports::ReplacementCleanupInfo {
                    cleanup_id: "unavailable".into(),
                    successor: TreatmentIdentity {
                        destination: root.to_string_lossy().into_owned(),
                        job_id: JobId(0),
                        generation: Generation::first(),
                    },
                    remaining: Vec::new(),
                    preserved: Vec::new(),
                    last_error: Some(error),
                },
            ])),
        }
        Ok(())
    }
    pub(in crate::worker_ipc) fn resolve_import_publication_effect(
        &mut self,
        destination: String,
        job_id: JobId,
        generation: Generation,
        events: &mut VecDeque<ImportEvent>,
    ) -> Result<(), (JobId, Generation, String)> {
        let root = PathBuf::from(destination);
        let resolution = ArchiveStore::resolve_import_publication(&root, job_id, generation);
        let mut history = scan_archive_history(&root);
        if let Err(error) = resolution
            && let Some(item) = history
                .iter_mut()
                .find(|item| item.job_id == job_id && item.generation == generation)
        {
            item.diagnostic = Some(error);
        }
        events.push_back(ImportEvent::History(history));
        Ok(())
    }
    pub(in crate::worker_ipc) fn resolve_live_publication_effect(
        &mut self,
        request: LiveRequest,
        events: &mut VecDeque<ImportEvent>,
    ) -> Result<(), (JobId, Generation, String)> {
        let root = PathBuf::from(&request.destination);
        let resolution = ArchiveStore::resolve_live_publication(&root, &request);
        let mut history = scan_archive_history(&root);
        if let Err(error) = resolution
            && let Some(item) = history
                .iter_mut()
                .find(|item| item.job_id == request.job_id && item.generation == request.generation)
        {
            item.diagnostic = Some(error);
        }
        events.push_back(ImportEvent::History(history));
        Ok(())
    }
    pub(in crate::worker_ipc) fn persist_segment_effect(
        &mut self,
        segment: WorkerSegment,
        events: &mut VecDeque<ImportEvent>,
    ) -> Result<(), (JobId, Generation, String)> {
        let id = (segment.job_id, segment.generation);
        let sequence = segment.segment_id.0;
        self.persist_segment(segment, events).and_then(|()| {
            let confirmation = self
                .parakeet_vad
                .as_ref()
                .and_then(|run| run.pending_confirmation)
                .filter(|(_, last_sequence)| *last_sequence == sequence);
            if let Some((window, last_sequence)) = confirmation {
                self.confirm_and_advance_parakeet_vad_window(window, last_sequence)
                    .map_err(|error| (id.0, id.1, error))?;
            }
            acceptance_barrier(
                "resume-prefix-confirmed",
                self.archive.root_path().as_path(),
            )
            .map_err(|error| (id.0, id.1, error))
        })
    }
    pub(in crate::worker_ipc) fn stop_effect(
        &mut self,
        job_id: JobId,
        generation: Generation,
        events: &mut VecDeque<ImportEvent>,
    ) -> Result<(), (JobId, Generation, String)> {
        let session = whisper_core::parakeet_scheduler::SchedulerSession { job_id, generation };
        if self.native_unified_routes.remove(&session).is_some()
            && !self.unified_sessions.contains_key(&session)
        {
            events.push_back(ImportEvent::Stopped { job_id, generation });
            self.release_unified_terminal_state(job_id, generation);
            Ok(())
        } else if let Some(mut live) = self.parakeet_live.take() {
            if live.identity() != (job_id, generation) {
                self.parakeet_live = Some(live);
                Err((job_id, generation, "Arrêt Parakeet périmé".into()))
            } else {
                match live.stop() {
                    Ok(event) => {
                        events.push_back(attach_automatic_final_import(&live, event));
                        Ok(())
                    }
                    Err(error) => Err((job_id, generation, error)),
                }
            }
        } else {
            let active_request = self.active.as_ref().and_then(|active| {
                (active.job_id == job_id && active.generation == generation)
                    .then(|| active.import_request.clone())
                    .flatten()
            });
            if active_request.is_some()
                && let Some(mut worker) = self.parakeet.take()
            {
                let termination = worker.terminate();
                // The child has been detached from this runtime. Settle its
                // in-memory identity before any fallible queue journal I/O.
                self.active = None;
                self.parakeet_events.clear();
                self.parakeet_end = None;
                self.parakeet_stopped_ack = false;
                if let Some(mut run) = self.parakeet_vad.take()
                    && let Some(path) = run.current_path.take()
                {
                    let _ = fs::remove_file(path);
                }
                self.progress = None;
                match termination {
                    Ok(_status) => {
                        let queue_update = if let Some(request) = active_request {
                            let queue_journal = Path::new(&request.destination)
                                .join("transcriptions")
                                .join("queue.jsonl");
                            if queue_journal.is_file() {
                                crate::queue_store::QueueStore::open(&request.destination)
                                    .and_then(|mut queue| {
                                        if queue.entries().iter().any(|entry| {
                                            entry.request.job_id == job_id
                                                && entry.request.generation == generation
                                        }) {
                                            queue.set_status(
                                                job_id,
                                                generation,
                                                crate::queue_store::QueueStatus::Interrupted,
                                            )?;
                                            Ok(Some(queue.entries()))
                                        } else {
                                            Ok(None)
                                        }
                                    })
                                    .map_err(|error| (job_id, generation, error))
                            } else {
                                Ok(None)
                            }
                        } else {
                            Ok(None)
                        };
                        match queue_update {
                            Err(error) => Err(error),
                            Ok(queue) => {
                                if let Some(entries) = queue {
                                    events.push_back(ImportEvent::Queue(entries));
                                }
                                events.push_back(ImportEvent::Stopped { job_id, generation });
                                Ok(())
                            }
                        }
                    }
                    Err(error) => Err((job_id, generation, error)),
                }
            } else if let Some(direct) = self.direct_live.as_mut() {
                if direct.identity() != (job_id, generation) {
                    Err((job_id, generation, "Arrêt micro périmé".into()))
                } else {
                    direct
                        .stop()
                        .map_err(|error| (job_id, generation, error))
                        .map(|()| {
                            events.push_back(ImportEvent::LiveFinalizing { job_id, generation });
                        })
                }
            } else {
                if let Some(progress) = self.progress.as_mut() {
                    let now = std::time::Instant::now();
                    progress.close(ProgressPhase::Capturing);
                    progress.open(ProgressPhase::Draining, now);
                    progress.open(ProgressPhase::Cancelling, now);
                }
                let live = self.live.as_ref().is_some_and(|live| {
                    (live.request.job_id, live.request.generation) == (job_id, generation)
                });
                self.stop(job_id, generation, events).map(|()| {
                    if live && self.live.is_some() {
                        events.push_back(ImportEvent::LiveFinalizing { job_id, generation });
                    }
                })
            }
        }
    }
    pub(in crate::worker_ipc) fn prepare_effect(
        &mut self,
        request: ImportRequest,
        events: &mut VecDeque<ImportEvent>,
    ) -> Result<(), (JobId, Generation, String)> {
        match self.bind_import_request(&request) {
            Ok(request) => {
                let candidate = self
                    .gpu_candidates
                    .get(&(request.job_id, request.generation.get()))
                    .and_then(Clone::clone);
                self.prepare(request, candidate, events)
            }
            Err(error) => Err((
                request.job_id,
                request.generation,
                format!("Modèle indisponible : {error}"),
            )),
        }
    }
}
