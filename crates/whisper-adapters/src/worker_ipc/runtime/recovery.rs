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
        RecoveryEffect::Continue({
            let id = (request.job_id, request.generation);
            let model_catalog = self.model_catalog.as_ref().map(|catalog| {
                (
                    catalog.snapshot(),
                    catalog.known_historical_turbo_path.clone(),
                )
            });
            let queue_validation =
                crate::archive::assert_campaign_archive(Path::new(&request.destination))
                    .and_then(|()| crate::queue_store::QueueStore::open(&request.destination))
                    .and_then(|queue| {
                        if request_is_parakeet(&request) && request_is_parakeet(&previous) {
                            let mut comparable = request.clone();
                            comparable.model_path = previous.model_path.clone();
                            queue.validate_resume(previous.job_id, previous.generation, &comparable)
                        } else if let Some((snapshot, known_path)) = &model_catalog {
                            queue.validate_resume_with_catalog(
                                previous.job_id,
                                previous.generation,
                                &request,
                                snapshot,
                                known_path,
                            )
                        } else {
                            queue.validate_resume(previous.job_id, previous.generation, &request)
                        }
                    });
            if let Err(error) = queue_validation {
                events.push_back(ImportEvent::Failed {
                    job_id: id.0,
                    generation: id.1,
                    message: error,
                });
                return RecoveryEffect::Return;
            }
            let destination = Path::new(&request.destination);
            match crate::archive::ArchiveStore::publication_reconfirmation_required(
                destination,
                previous.job_id,
                previous.generation,
            ) {
                Ok(Some(reserved_export_basename)) => {
                    let _lock = match crate::archive::ArchiveStore::lock_publications(destination) {
                        Ok(lock) => lock,
                        Err(error) => {
                            events.push_back(ImportEvent::Failed {
                                job_id: id.0,
                                generation: id.1,
                                message: error,
                            });
                            return RecoveryEffect::Return;
                        }
                    };
                    let source_sha256 = match previous.source_sha256 {
                        Some(sha) => format_sha256(&sha),
                        None => {
                            events.push_back(ImportEvent::Failed {
                                job_id: id.0,
                                generation: id.1,
                                message: "SourceChanged: resume source identity is missing".into(),
                            });
                            return RecoveryEffect::Return;
                        }
                    };
                    let publications = match crate::archive::ArchiveStore::published_imports(
                        destination,
                        &source_sha256,
                    ) {
                        Ok(publications) => publications,
                        Err(message) => {
                            events.push_back(ImportEvent::Failed {
                                job_id: id.0,
                                generation: id.1,
                                message,
                            });
                            return RecoveryEffect::Return;
                        }
                    };
                    let export_basename =
                        match crate::archive::ArchiveStore::suggest_duplicate_export_basename(
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
                                return RecoveryEffect::Return;
                            }
                        };
                    let Some(source_samples) = previous.source_samples else {
                        events.push_back(ImportEvent::Failed {
                            job_id: id.0,
                            generation: id.1,
                            message: "SourceChanged: resume source duration is missing".into(),
                        });
                        return RecoveryEffect::Return;
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
                    return RecoveryEffect::Return;
                }
                Ok(None) => {}
                Err(message) => {
                    events.push_back(ImportEvent::Failed {
                        job_id: id.0,
                        generation: id.1,
                        message,
                    });
                    return RecoveryEffect::Return;
                }
            }
            // Keep the historical model identity in the durable checkpoint. The execution
            // request is rebound to the verified catalog only after the checkpoint exists.
            let archive_destination = request.destination.clone();
            let mut durable_request = request;
            if model_catalog.is_some() {
                durable_request.model_path = previous.model_path.clone();
            }
            let executable_model_path = if let Some((snapshot, known_path)) = &model_catalog {
                match if request_is_parakeet(&durable_request) {
                    self.bind_import_request(&durable_request)
                } else {
                    snapshot.bind_turbo_import_request(&durable_request, known_path)
                } {
                    Ok(request) => request.model_path,
                    Err(error) => {
                        events.push_back(ImportEvent::Failed {
                            job_id: id.0,
                            generation: id.1,
                            message: format!("Modèle indisponible : {error}"),
                        });
                        return RecoveryEffect::Return;
                    }
                }
            } else {
                durable_request.model_path.clone()
            };
            let executable_model_path = model_catalog
                .as_ref()
                .map(|_| executable_model_path.as_str());
            let native_route = if durable_request.strategy
                == whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
            {
                let session = whisper_core::parakeet_scheduler::SchedulerSession {
                    job_id: id.0,
                    generation: id.1,
                };
                if self.unified_admission_full_for(session) {
                    events.push_back(ImportEvent::Failed {
                        job_id: id.0,
                        generation: id.1,
                        message: "SchedulerCapacity: two unified sessions are already admitted"
                            .into(),
                    });
                    return RecoveryEffect::Return;
                }
                let candidate = self
                    .gpu_candidates
                    .get(&(id.0, id.1.get()))
                    .and_then(Option::as_ref);
                match validate_current_queued_gpu_candidate(&durable_request, candidate).and_then(
                    |()| self.native_route_for_unified_import(&durable_request, candidate),
                ) {
                    Ok(route) => route,
                    Err(message) => {
                        events.push_back(ImportEvent::Failed {
                            job_id: id.0,
                            generation: id.1,
                            message,
                        });
                        return RecoveryEffect::Return;
                    }
                }
            } else {
                None
            };
            let source_binding_transferred =
                match self.transfer_resume_source_binding(&previous, &durable_request) {
                    Ok(transferred) => transferred,
                    Err(message) => {
                        events.push_back(ImportEvent::Failed {
                            job_id: id.0,
                            generation: id.1,
                            message,
                        });
                        return RecoveryEffect::Return;
                    }
                };
            #[cfg(test)]
            if source_binding_transferred
                && let Some(gate) = self.controlled_resume_binding_transfer_gate.take()
            {
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
            let mut archive = ArchiveStore::new(&archive_destination);
            let result = archive
                .resume_from_with_qualification(
                    &previous,
                    durable_request.clone(),
                    executable_model_path,
                    archive_qualification_for_request(
                        &durable_request,
                        self.parakeet_paths.as_ref(),
                        self.gpu_parakeet_paths.as_ref(),
                        self.gpu_candidates
                            .get(&(id.0, id.1.get()))
                            .and_then(Option::as_ref),
                    ),
                )
                .and_then(|(prepared, prefix)| {
                    crate::queue_store::QueueStore::open(&prepared.destination).and_then(
                        |mut queue| {
                            if request_is_parakeet(&prepared) {
                                queue.resume_interrupted(
                                    previous.job_id,
                                    previous.generation,
                                    prepared.clone(),
                                )?;
                            } else if let Some((snapshot, known_path)) = &model_catalog {
                                queue.resume_interrupted_with_catalog(
                                    previous.job_id,
                                    previous.generation,
                                    prepared.clone(),
                                    snapshot,
                                    known_path,
                                )?;
                            } else {
                                queue.resume_interrupted(
                                    previous.job_id,
                                    previous.generation,
                                    prepared.clone(),
                                )?;
                            }
                            Ok((prepared, prefix, queue.entries()))
                        },
                    )
                });
            match result {
                Ok((prepared, prefix, queue)) => {
                    if let Some(route) = native_route
                        && let Err(error) = self.register_prepared_native_route(
                            whisper_core::parakeet_scheduler::SchedulerSession {
                                job_id: id.0,
                                generation: id.1,
                            },
                            route,
                        )
                    {
                        events.push_back(ImportEvent::Failed {
                            job_id: id.0,
                            generation: id.1,
                            message: error,
                        });
                        return RecoveryEffect::Return;
                    }
                    let unified_resume_checkpoint = archive.take_unified_resume_checkpoint();
                    let confirmed_segments = unified_resume_checkpoint
                        .as_ref()
                        .map_or(prefix.len() as u64, |checkpoint| {
                            checkpoint.pipeline.confirmed_segments
                        });
                    let confirmed_offset =
                        if let Some(checkpoint) = unified_resume_checkpoint.as_ref() {
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
                    if let Some(checkpoint) = unified_resume_checkpoint {
                        self.unified_resume_checkpoints.insert(
                            whisper_core::parakeet_scheduler::SchedulerSession {
                                job_id: id.0,
                                generation: id.1,
                            },
                            checkpoint,
                        );
                    }
                    events.push_back(ImportEvent::Queue(queue));
                    events.push_back(ImportEvent::ResumeCheckpoint {
                        job_id: id.0,
                        generation: id.1,
                        confirmed_segments,
                        confirmed_offset,
                    });
                    events.push_back(ImportEvent::Prepared(prepared));
                    Ok(())
                }
                Err(error) => {
                    let error = if source_binding_transferred {
                        match self.restore_resume_source_binding(&previous, &durable_request) {
                            Ok(()) => error,
                            Err(rollback_error) => {
                                format!("{error}; rollback source binding failed: {rollback_error}")
                            }
                        }
                    } else {
                        error
                    };
                    Err((id.0, id.1, error))
                }
            }
        })
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
