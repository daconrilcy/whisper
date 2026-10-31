use super::*;

impl ImportRuntime {
    pub(in crate::worker_ipc) fn enqueue(
        &mut self,
        mut request: ImportRequest,
        events: &mut VecDeque<ImportEvent>,
    ) -> Result<(), (JobId, Generation, String)> {
        let submitted = (request.job_id, request.generation);
        let _timing_context = TimingContextGuard::enter(
            "enqueue-import",
            format!("{:032x}-{}", submitted.0.0, submitted.1.get()),
            Path::new(&request.destination),
        );
        let started = Instant::now();
        let mut binding_key = submitted;
        let binding_destination = PathBuf::from(&request.destination);
        let result = self
                    .bind_import_request(&request)
                    .map(|bound| {
                        request = bound;
                    })
                    .and_then(|()| self.ensure_source_binding(&mut request))
                    .and_then(|()| {
                        self.bind_import_request(&request).map(|bound| {
                            request = bound;
                        })
                    })
                    .and_then(|()| {
                        crate::archive::assert_campaign_archive(Path::new(&request.destination))
                    })
                    .and_then(|()| {
                        let destination = Path::new(&request.destination);
                        let _lock = crate::archive::ArchiveStore::lock_publications(destination)?;
                        let source_sha = request
                            .source_sha256
                            .as_ref()
                            .map(format_sha256)
                            .ok_or_else(|| {
                                "SourceChanged: source identity is missing".to_owned()
                            })?;
                        let legacy_export_basename = if self.snapshot_registry_root.is_none() {
                            crate::archive::ArchiveStore::suggest_duplicate_export_basename(
                                destination,
                                &request.source_path,
                            )?
                        } else {
                            String::new()
                        };
                        let publications = match
                            crate::archive::ArchiveStore::reprocessing_catalog(destination, &source_sha)
                        {
                            Ok(publications) => publications,
                            Err(error) if error.starts_with("StorageCorrupt:") => {
                                let source_samples = request.source_samples.unwrap_or(0);
                                return Ok(PreflightResult::Duplicate(DuplicateImportPrompt {
                                    request,
                                    source_sha256: source_sha,
                                    source_samples,
                                    publications: Vec::new(),
                                    export_basename: legacy_export_basename.clone(),
                                    revalidation_notice: Some(
                                        "Certains anciens traitements sont illisibles. Vous pouvez créer un nouveau dossier ; les anciens resteront intacts.".into(),
                                    ),
                                    resume_from: None,
                                    reserved_export_basename: None,
                                }));
                            }
                            Err(error) => return Err(error),
                        };
                        let publications = verified_present_publications(publications);
                        if !publications.is_empty() {
                            let source_samples = request.source_samples.unwrap_or(0);
                            return Ok(PreflightResult::Duplicate(DuplicateImportPrompt {
                                request,
                                source_sha256: source_sha,
                                source_samples,
                                publications,
                                export_basename: legacy_export_basename,
                                revalidation_notice: None,
                                resume_from: None,
                                reserved_export_basename: None,
                            }));
                        }
                        let mut queue = crate::queue_store::QueueStore::open(destination)?;
                        let original = (request.job_id, request.generation);
                        request.job_id = collision_free_job_id(
                            destination,
                            request.job_id,
                            request.generation,
                            &queue.entries(),
                        );
                        let source_sha = request
                            .source_sha256
                            .as_ref()
                            .map(format_sha256)
                            .ok_or_else(|| {
                                "SourceChanged: source identity is missing".to_owned()
                            })?;
                        if self.snapshot_registry_root.is_some() {
                            crate::archive::ArchiveStore::reserve_create_only_new_import(
                                destination,
                                &request,
                                &source_sha,
                            )?;
                        } else {
                            crate::archive::ArchiveStore::reserve_new_import(
                                destination,
                                &request,
                                &source_sha,
                            )?;
                        }
                        if let Err(error) = self
                            .rebind_source_binding(original, (request.job_id, request.generation))
                        {
                            let _ = crate::archive::ArchiveStore::clear_duplicate_reservation(
                                destination,
                                request.job_id,
                                request.generation,
                            );
                            return Err(error);
                        }
                        binding_key = (request.job_id, request.generation);
                        let admitted = (request.job_id, request.generation);
                        if let Err(error) = queue.enqueue(request.clone()) {
                            let _ = crate::archive::ArchiveStore::clear_duplicate_reservation(
                                destination,
                                request.job_id,
                                request.generation,
                            );
                            let _ = self.release_source_binding(
                                request.job_id,
                                request.generation,
                                destination,
                            );
                            return Err(error);
                        }
                        Ok(PreflightResult::Queued {
                            submitted: original,
                            admitted,
                            entries: queue.entries(),
                        })
                    });
        record_stage_timing(
            "import-admission",
            started.elapsed(),
            serde_json::json!({
                "operation": format!("{:032x}-{}", submitted.0.0, submitted.1.get()),
                "pid": std::process::id(),
                "phase": "admission",
                "outcome": match result.as_ref() {
                    Ok(PreflightResult::Duplicate(_)) => "duplicate",
                    Ok(PreflightResult::Queued { .. }) => "queued",
                    Err(_) => "error",
                },
            }),
        );
        match result {
            Ok(PreflightResult::Duplicate(prompt)) => {
                events.push_back(ImportEvent::DuplicateImportFound(prompt));
                Ok(())
            }
            Ok(PreflightResult::Queued {
                submitted,
                admitted,
                entries,
            }) => {
                if submitted == admitted {
                    events.push_back(ImportEvent::Queue(entries));
                } else {
                    events.push_back(ImportEvent::QueueAdmission {
                        submitted_job_id: submitted.0,
                        submitted_generation: submitted.1,
                        queue: entries,
                    });
                }
                Ok(())
            }
            Err(error) => {
                let _ =
                    self.release_source_binding(binding_key.0, binding_key.1, &binding_destination);
                Err((submitted.0, submitted.1, error))
            }
        }
    }
    pub(in crate::worker_ipc) fn remove_queued(
        &mut self,
        job_id: JobId,
        generation: Generation,
        destination: String,
        events: &mut VecDeque<ImportEvent>,
    ) -> Result<(), (JobId, Generation, String)> {
        match crate::archive::assert_campaign_archive(Path::new(&destination))
            .and_then(|()| crate::queue_store::QueueStore::open(&destination))
            .and_then(|mut queue| {
                queue.remove(job_id, generation)?;
                Ok(queue.entries())
            }) {
            Ok(queue) => {
                match self.release_source_binding(job_id, generation, Path::new(&destination)) {
                    Ok(()) => {
                        events.push_back(ImportEvent::Queue(queue));
                        Ok(())
                    }
                    Err(error) => Err((job_id, generation, error)),
                }
            }
            Err(error) => Err((job_id, generation, error)),
        }
    }
    pub(in crate::worker_ipc) fn process_queued(
        &mut self,
        request: ImportRequest,
        events: &mut VecDeque<ImportEvent>,
    ) -> Result<(), (JobId, Generation, String)> {
        let id = (request.job_id, request.generation);
        let _timing_context = TimingContextGuard::enter(
            "process-queued-import",
            format!("{:032x}-{}", id.0.0, id.1.get()),
            Path::new(&request.destination),
        );
        let model_catalog = self.model_catalog.as_ref().map(|catalog| {
            (
                catalog.snapshot(),
                catalog.known_historical_turbo_path.clone(),
            )
        });
        let result = if request.strategy
            == whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
            && self.unified_admission_full_for(whisper_core::parakeet_scheduler::SchedulerSession {
                job_id: id.0,
                generation: id.1,
            }) {
            Err("SchedulerCapacity: two unified sessions are already admitted".into())
        } else {
            crate::archive::assert_campaign_archive(Path::new(&request.destination))
                        .and_then(|()| crate::queue_store::QueueStore::open(&request.destination))
                        .and_then(|mut queue| {
                            let entries = queue.entries();
                            let first = entries
                                .first()
                                .map(|entry| (entry.request.job_id, entry.request.generation));
                            let entry = entries
                                .into_iter()
                                .find(|entry| {
                                    entry.request.job_id == id.0 && entry.request.generation == id.1
                                })
                                .ok_or_else(|| {
                                    "InvalidInput: queue entry no longer exists".to_owned()
                                })?;
                            if first != Some(id) {
                                return Err(
                                    "InvalidInput: process queued jobs in FIFO order".into()
                                );
                            }
                            let same_model = if request_is_parakeet(&entry.request)
                                && request_is_parakeet(&request)
                            {
                                entry.request.model_sha256 == request.model_sha256
                            } else if let Some((snapshot, known_path)) = &model_catalog {
                                let durable_id = snapshot.resolve_model_identity(
                                    &entry.request.model_path,
                                    known_path,
                                )?;
                                let requested_id = snapshot
                                    .resolve_model_identity(&request.model_path, known_path)?;
                                durable_id == requested_id
                            } else {
                                false
                            };
                            if !queued_request_identity_matches(
                                &entry.request,
                                &request,
                                same_model,
                            ) {
                                return Err(
                            "SourceChanged: queued request identity differs from the durable entry"
                                .into(),
                        );
                            }
                            if !matches!(
                                entry.status,
                                crate::queue_store::QueueStatus::Queued
                                    | crate::queue_store::QueueStatus::AwaitingChoice
                            ) {
                                return Err(
                            "InvalidInput: interrupted jobs require an explicit verified resume"
                                .into(),
                        );
                            }
                            let gpu_candidate = self
                                .gpu_candidates
                                .get(&(id.0, id.1.get()))
                                .and_then(Option::as_ref);
                            validate_current_queued_gpu_candidate(&request, gpu_candidate)?;
                            let native_route = if request.strategy
                                == whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
                            {
                                self.native_route_for_unified_import(&request, gpu_candidate)?
                            } else {
                                None
                            };
                            let prevalidated_gpu_executable =
                                if request_is_parakeet(&request)
                                    && request.config.compute == whisper_core::ComputeChoice::Gpu
                                {
                                    let executable = self.bind_import_request(&request)?;
                                    let unified = request.strategy
                                        == whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1;
                                    if !unified {
                                        let qualification = archive_qualification_for_request(
                                            &request,
                                            self.parakeet_paths.as_ref(),
                                            self.gpu_parakeet_paths.as_ref(),
                                            gpu_candidate,
                                        );
                                        validate_queued_gpu_preflight(
                                            &request,
                                            gpu_candidate,
                                            qualification.as_ref(),
                                        )?;
                                    }
                                    if !unified || native_route.is_none() {
                                        crate::parakeet_import::expected_gpu_for_candidate(
                                            gpu_candidate.expect(
                                                "current manual GPU proof was checked",
                                            ),
                                        )
                                        .map_err(|error| {
                                            format!(
                                                "BackendMismatch: GPU identity validation failed: {error}"
                                            )
                                        })?;
                                    }
                                    Some(executable)
                                } else {
                                    None
                                };
                            let durable_request = queue
                                .set_pending_compute(&entry.request, request.config.compute)?
                                .request;
                            if let Some(executable) = prevalidated_gpu_executable {
                                Ok((durable_request, executable, native_route))
                            } else if request_is_parakeet(&durable_request) {
                                self.bind_import_request(&durable_request)
                                    .map(|executable| (durable_request, executable, native_route))
                            } else if let Some((snapshot, known_path)) = &model_catalog {
                                queue
                                    .resolve_queued_with_catalog(id.0, id.1, snapshot, known_path)
                                    .map(|executable| (durable_request, executable, native_route))
                            } else {
                                Ok((durable_request.clone(), durable_request, native_route))
                            }
                        })
        };
        match result {
            Ok((mut durable_request, verified_executable, native_route)) => {
                let binding_result = self.ensure_source_binding(&mut durable_request);
                if let Err(error) = binding_result {
                    Err((id.0, id.1, error))
                } else {
                    let executable_model_path = model_catalog
                        .as_ref()
                        .map(|_| verified_executable.model_path.as_str());
                    let mut archive = ArchiveStore::new(&durable_request.destination);
                    let qualification = archive_qualification_for_request(
                        &durable_request,
                        self.parakeet_paths.as_ref(),
                        self.gpu_parakeet_paths.as_ref(),
                        self.gpu_candidates
                            .get(&(id.0, id.1.get()))
                            .and_then(Option::as_ref),
                    );
                    let prepared =
                        if let Some(binding) = self.source_bindings.get(&(id.0, id.1.get())) {
                            archive.prepare_with_source_binding(
                                durable_request.clone(),
                                executable_model_path,
                                qualification,
                                binding,
                            )
                        } else if self.require_snapshot_registry {
                            Err("SourceSnapshotUnavailable: binding missing before prepare".into())
                        } else {
                            archive.prepare_with_qualification(
                                durable_request.clone(),
                                executable_model_path,
                                qualification,
                            )
                        };
                    match prepared {
                        Ok(prepared) => {
                            let queue_result =
                                crate::queue_store::QueueStore::open(&prepared.destination)
                                    .and_then(|mut queue| {
                                        queue.set_status(
                                            id.0,
                                            id.1,
                                            crate::queue_store::QueueStatus::Running,
                                        )?;
                                        Ok(queue.entries())
                                    });
                            match queue_result {
                                Ok(queue) => {
                                    let route_result = native_route.map_or(Ok(()), |route| {
                                        self.register_prepared_native_route(
                                            whisper_core::parakeet_scheduler::SchedulerSession {
                                                job_id: id.0,
                                                generation: id.1,
                                            },
                                            route,
                                        )
                                    });
                                    match route_result {
                                        Ok(()) => {
                                            self.archive = archive;
                                            events.push_back(ImportEvent::Queue(queue));
                                            events.push_back(ImportEvent::Prepared(prepared));
                                            Ok(())
                                        }
                                        Err(error) => Err((id.0, id.1, error)),
                                    }
                                }
                                Err(error) => Err((id.0, id.1, error)),
                            }
                        }
                        Err(error) => Err((id.0, id.1, error)),
                    }
                }
            }
            Err(error) => Err((id.0, id.1, error)),
        }
    }
    pub(in crate::worker_ipc) fn scan_queue(
        &mut self,
        destination: String,
        events: &mut VecDeque<ImportEvent>,
    ) -> Result<(), (JobId, Generation, String)> {
        match crate::recovery::scan(Path::new(&destination)) {
            Ok(snapshot) => {
                events.push_back(ImportEvent::Queue(snapshot.queue));
                Ok(())
            }
            Err(error) => Err((JobId(0), Generation::first(), error)),
        }
    }
}
