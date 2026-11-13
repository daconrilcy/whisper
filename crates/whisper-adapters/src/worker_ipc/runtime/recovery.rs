use super::*;

pub(in crate::worker_ipc) enum RecoveryEffect {
    Return,
    Continue(Result<(), (JobId, Generation, String)>),
}

impl ImportRuntime {
    pub(in crate::worker_ipc) fn resume_interrupted(
        &mut self,
        previous: Box<ImportRequest>,
        request: ImportRequest,
        events: &mut VecDeque<ImportEvent>,
    ) -> RecoveryEffect {
        let id = (request.job_id, request.generation);
        let inspected_catalog = match self.inspect_resume_queue(&previous, &request) {
            Ok(snapshot) => snapshot,
            Err(message) => return resume_refusal(events, id, message),
        };
        let Some((previous, request)) =
            reconfirm_interrupted_publication(previous, request, events)
        else {
            return RecoveryEffect::Return;
        };
        let plan = match self.prepare_resume_execution(&previous, request, inspected_catalog, id) {
            Ok(plan) => plan,
            Err(message) => return resume_refusal(events, id, message),
        };
        let source_binding_transferred = match self.transfer_resume_binding(&previous, &plan) {
            Ok(transferred) => transferred,
            Err(message) => return resume_refusal(events, id, message),
        };
        match self.resume_archive(&previous, &plan, id) {
            Ok(resumed) => {
                if let Some(route) = plan.native_route
                    && let Err(message) =
                        self.register_prepared_native_route(scheduler_session(id), route)
                {
                    return resume_refusal(events, id, message);
                }
                self.publish_resumed_import(resumed, id, events);
                RecoveryEffect::Continue(Ok(()))
            }
            Err(error) => {
                let error = self.restore_resume_binding_after_error(
                    &previous,
                    &plan.durable_request,
                    source_binding_transferred,
                    error,
                );
                RecoveryEffect::Continue(Err((id.0, id.1, error)))
            }
        }
    }

    fn inspect_resume_queue(
        &self,
        previous: &ImportRequest,
        request: &ImportRequest,
    ) -> Result<Option<InspectedResumeCatalog>, String> {
        let model_catalog = self
            .model_catalog
            .as_ref()
            .map(|catalog| catalog.clone_for_worker());
        crate::archive::assert_campaign_archive(Path::new(&request.destination))
            .and_then(|()| crate::queue_store::QueueStore::open(&request.destination))
            .and_then(|queue| {
                if request_is_parakeet(request) && request_is_parakeet(previous) {
                    let mut comparable = request.clone();
                    comparable.model_path = previous.model_path.clone();
                    queue
                        .validate_resume(previous.job_id, previous.generation, &comparable)
                        .map(|()| None)
                } else if let Some(catalog) = model_catalog.as_ref() {
                    let (previous_id, mut snapshot) = catalog.catalog.inspect_requested_model(
                        &previous.model_path,
                        &catalog.known_historical_turbo_path,
                    )?;
                    let (requested_id, requested_snapshot) =
                        catalog.catalog.inspect_requested_model(
                            &request.model_path,
                            &catalog.known_historical_turbo_path,
                        )?;
                    if previous_id != requested_id {
                        snapshot.entries.extend(requested_snapshot.entries);
                    }
                    queue
                        .validate_resume_with_catalog(
                            previous.job_id,
                            previous.generation,
                            request,
                            &snapshot,
                            &catalog.known_historical_turbo_path,
                        )
                        .map(|()| {
                            Some(InspectedResumeCatalog {
                                snapshot,
                                known_historical_turbo_path: catalog
                                    .known_historical_turbo_path
                                    .clone(),
                            })
                        })
                } else {
                    queue
                        .validate_resume(previous.job_id, previous.generation, request)
                        .map(|()| None)
                }
            })
    }

    fn prepare_resume_execution(
        &self,
        previous: &ImportRequest,
        request: ImportRequest,
        inspected_catalog: Option<InspectedResumeCatalog>,
        id: (JobId, Generation),
    ) -> Result<ResumeExecutionPlan, String> {
        let mut durable_request = request;
        if self.model_catalog.is_some() {
            durable_request.model_path = previous.model_path.clone();
        }
        let executable_model_path =
            self.executable_resume_model_path(&durable_request, inspected_catalog.as_ref())?;
        let native_route = self.resume_native_route(&durable_request, id)?;
        Ok(ResumeExecutionPlan {
            durable_request,
            executable_model_path,
            inspected_catalog,
            native_route,
        })
    }

    fn executable_resume_model_path(
        &self,
        durable_request: &ImportRequest,
        inspected_catalog: Option<&InspectedResumeCatalog>,
    ) -> Result<Option<String>, String> {
        if self.model_catalog.is_none() {
            return Ok(None);
        }
        let verified = if request_is_parakeet(durable_request) {
            self.bind_import_request(durable_request)
        } else if let Some(catalog) = inspected_catalog {
            catalog
                .snapshot
                .bind_turbo_import_request(durable_request, &catalog.known_historical_turbo_path)
        } else {
            Err("ModelUnknown: modèle Turbo non inspecté pour la reprise".into())
        };
        verified
            .map(|request| Some(request.model_path))
            .map_err(|error| format!("Modèle indisponible : {error}"))
    }

    fn resume_native_route(
        &self,
        durable_request: &ImportRequest,
        id: (JobId, Generation),
    ) -> Result<Option<NativeUnifiedRoute>, String> {
        if durable_request.strategy
            != whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
        {
            return Ok(None);
        }
        let session = scheduler_session(id);
        if self.unified_admission_full_for(session) {
            return Err("SchedulerCapacity: two unified sessions are already admitted".into());
        }
        let candidate = self
            .gpu_candidates
            .get(&(id.0, id.1.get()))
            .and_then(Option::as_ref);
        validate_current_queued_gpu_candidate(durable_request, candidate)
            .and_then(|()| self.native_route_for_unified_import(durable_request, candidate))
    }

    fn transfer_resume_binding(
        &mut self,
        previous: &ImportRequest,
        plan: &ResumeExecutionPlan,
    ) -> Result<bool, String> {
        let transferred = self.transfer_resume_source_binding(previous, &plan.durable_request)?;
        #[cfg(test)]
        if transferred && let Some(gate) = self.controlled_resume_binding_transfer_gate.take() {
            let (released, changed) = &*gate;
            let mut released = released
                .lock()
                .expect("resume binding transfer test gate lock");
            *released = true;
            changed.notify_all();
            let (released, timeout) = changed
                .wait_timeout_while(released, Duration::from_secs(30), |released| *released)
                .expect("resume binding transfer test gate wait");
            assert!(
                !timeout.timed_out() && !*released,
                "test must observe the transferred binding before archive resume continues"
            );
        }
        Ok(transferred)
    }

    fn resume_archive(
        &self,
        previous: &ImportRequest,
        plan: &ResumeExecutionPlan,
        id: (JobId, Generation),
    ) -> Result<ResumedArchive, String> {
        let mut archive = ArchiveStore::new(&plan.durable_request.destination);
        let (prepared, prefix) = archive.resume_from_with_qualification(
            previous,
            plan.durable_request.clone(),
            plan.executable_model_path.as_deref(),
            archive_qualification_for_request(
                &plan.durable_request,
                self.parakeet_paths.as_ref(),
                self.gpu_parakeet_paths.as_ref(),
                self.gpu_candidates
                    .get(&(id.0, id.1.get()))
                    .and_then(Option::as_ref),
            ),
        )?;
        let mut queue = crate::queue_store::QueueStore::open(&prepared.destination)?;
        if request_is_parakeet(&prepared) {
            queue.resume_interrupted(previous.job_id, previous.generation, prepared.clone())?;
        } else if let Some(catalog) = &plan.inspected_catalog {
            queue.resume_interrupted_with_catalog(
                previous.job_id,
                previous.generation,
                prepared.clone(),
                &catalog.snapshot,
                &catalog.known_historical_turbo_path,
            )?;
        } else {
            queue.resume_interrupted(previous.job_id, previous.generation, prepared.clone())?;
        }
        Ok(ResumedArchive {
            archive,
            prepared,
            prefix,
            queue: queue.entries(),
        })
    }

    fn restore_resume_binding_after_error(
        &mut self,
        previous: &ImportRequest,
        durable_request: &ImportRequest,
        source_binding_transferred: bool,
        error: String,
    ) -> String {
        if !source_binding_transferred {
            return error;
        }
        match self.restore_resume_source_binding(previous, durable_request) {
            Ok(()) => error,
            Err(rollback_error) => {
                format!("{error}; rollback source binding failed: {rollback_error}")
            }
        }
    }

    fn publish_resumed_import(
        &mut self,
        resumed: ResumedArchive,
        id: (JobId, Generation),
        events: &mut VecDeque<ImportEvent>,
    ) {
        let ResumedArchive {
            mut archive,
            prepared,
            prefix,
            queue,
        } = resumed;
        let checkpoint = archive.take_unified_resume_checkpoint();
        let confirmed_segments = checkpoint
            .as_ref()
            .map_or(prefix.len() as u64, |checkpoint| {
                checkpoint.pipeline.confirmed_segments
            });
        let confirmed_offset = if let Some(checkpoint) = checkpoint.as_ref() {
            checkpoint.pipeline.covered_samples
        } else if prepared.source_samples.is_some_and(|samples| {
            samples > whisper_core::parakeet_ipc::PARAKEET_MAX_AUDIO_SAMPLES as u64
        }) {
            archive.confirmed_import_vad_offset().unwrap_or(0)
        } else {
            prefix.last().map_or(0, |segment| segment.range.end_sample)
        };
        self.archive = archive;
        self.resume_prefix = prefix.into();
        if let Some(checkpoint) = checkpoint {
            self.unified_resume_checkpoints
                .insert(scheduler_session(id), checkpoint);
        }
        events.push_back(ImportEvent::Queue(queue));
        events.push_back(ImportEvent::ResumeCheckpoint {
            job_id: id.0,
            generation: id.1,
            confirmed_segments,
            confirmed_offset,
        });
        events.push_back(ImportEvent::Prepared(prepared));
    }
    pub(in crate::worker_ipc) fn resume_replacement_cleanup(
        &mut self,
        destination: String,
        cleanup_id: String,
        events: &mut VecDeque<ImportEvent>,
    ) -> Result<(), (JobId, Generation, String)> {
        let root = PathBuf::from(destination);
        let result = ArchiveStore::resume_replacement_cleanup(&root, &cleanup_id);
        let mut cleanups = ArchiveStore::replacement_cleanups(&root).unwrap_or_default();
        if let Err(error) = result
            && let Some(cleanup) = cleanups
                .iter_mut()
                .find(|item| item.cleanup_id == cleanup_id)
        {
            cleanup.last_error = Some(error);
        }
        events.push_back(ImportEvent::ReplacementCleanups(cleanups));
        Ok(())
    }
}

struct InspectedResumeCatalog {
    snapshot: crate::model_catalog::CatalogSnapshot,
    known_historical_turbo_path: String,
}

struct ResumeExecutionPlan {
    durable_request: ImportRequest,
    executable_model_path: Option<String>,
    inspected_catalog: Option<InspectedResumeCatalog>,
    native_route: Option<NativeUnifiedRoute>,
}

struct ResumedArchive {
    archive: ArchiveStore,
    prepared: ImportRequest,
    prefix: Vec<crate::journal::SegmentRecord>,
    queue: Vec<crate::queue_store::QueueEntry>,
}

fn scheduler_session(
    id: (JobId, Generation),
) -> whisper_core::parakeet_scheduler::SchedulerSession {
    whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: id.0,
        generation: id.1,
    }
}

fn resume_refusal(
    events: &mut VecDeque<ImportEvent>,
    id: (JobId, Generation),
    message: String,
) -> RecoveryEffect {
    events.push_back(ImportEvent::Failed {
        job_id: id.0,
        generation: id.1,
        message,
    });
    RecoveryEffect::Return
}

fn reconfirm_interrupted_publication(
    previous: Box<ImportRequest>,
    request: ImportRequest,
    events: &mut VecDeque<ImportEvent>,
) -> Option<(Box<ImportRequest>, ImportRequest)> {
    let id = (request.job_id, request.generation);
    let destination = Path::new(&request.destination);
    let reserved_export_basename =
        match crate::archive::ArchiveStore::publication_reconfirmation_required(
            destination,
            previous.job_id,
            previous.generation,
        ) {
            Ok(Some(name)) => name,
            Ok(None) => return Some((previous, request)),
            Err(message) => {
                events.push_back(ImportEvent::Failed {
                    job_id: id.0,
                    generation: id.1,
                    message,
                });
                return None;
            }
        };
    let _lock = match crate::archive::ArchiveStore::lock_publications(destination) {
        Ok(lock) => lock,
        Err(error) => {
            events.push_back(ImportEvent::Failed {
                job_id: id.0,
                generation: id.1,
                message: error,
            });
            return None;
        }
    };
    let source_sha256 = match previous.source_sha256.as_ref() {
        Some(sha) => format_sha256(sha),
        None => {
            events.push_back(ImportEvent::Failed {
                job_id: id.0,
                generation: id.1,
                message: "SourceChanged: resume source identity is missing".into(),
            });
            return None;
        }
    };
    let publications =
        match crate::archive::ArchiveStore::published_imports(destination, &source_sha256) {
            Ok(publications) => publications,
            Err(message) => {
                events.push_back(ImportEvent::Failed {
                    job_id: id.0,
                    generation: id.1,
                    message,
                });
                return None;
            }
        };
    let export_basename = match crate::archive::ArchiveStore::suggest_duplicate_export_basename(
        destination,
        &previous.source_path,
    ) {
        Ok(name) => name,
        Err(message) => {
            events.push_back(ImportEvent::Failed {
                job_id: id.0,
                generation: id.1,
                message,
            });
            return None;
        }
    };
    let Some(source_samples) = previous.source_samples else {
        events.push_back(ImportEvent::Failed {
            job_id: id.0,
            generation: id.1,
            message: "SourceChanged: resume source duration is missing".into(),
        });
        return None;
    };
    events.push_back(ImportEvent::DuplicateImportFound(DuplicateImportPrompt {
        request,
        source_sha256,
        source_samples,
        publications,
        export_basename,
        revalidation_notice: Some(format!(
            "Une sortie interrompue au nom « {reserved_export_basename} » ne peut pas être attribuée sans risque. Confirmez explicitement ce nouveau nom pour reprendre ; l'ancienne sortie et les traitements existants sont conservés."
        )),
        resume_from: Some(*previous),
        reserved_export_basename: Some(reserved_export_basename),
    }));
    None
}
