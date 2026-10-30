use super::*;

pub(in crate::worker_ipc) enum DuplicateEffect {
    Return,
    Continue(Result<(), (JobId, Generation, String)>),
}

pub(in crate::worker_ipc) fn resolve_reprocessing_targets(
    destination: &Path,
    catalog: &[PublishedImport],
    choice: &DuplicateImportChoice,
) -> Result<Vec<TreatmentIdentity>, String> {
    let root = destination
        .canonicalize()
        .map_err(|error| format!("InvalidInput: destination non accessible: {error}"))?;
    let verified: Vec<_> = catalog
        .iter()
        .filter(|item| item.verified_present)
        .collect();
    let selected = match choice {
        DuplicateImportChoice::ReplaceAll => verified,
        DuplicateImportChoice::ReplaceLatest => {
            let latest_ordinal = verified
                .iter()
                .filter_map(|item| item.success_ordinal)
                .max();
            let latest = if let Some(ordinal) = latest_ordinal {
                verified
                    .iter()
                    .copied()
                    .filter(|item| item.success_ordinal == Some(ordinal))
                    .collect::<Vec<_>>()
            } else if verified.len() == 1 {
                verified.clone()
            } else {
                return Err("Impossible de déterminer le dernier traitement : l'ordre historique est inconnu. Choisissez tous les traitements ou Parcourir.".into());
            };
            if latest.len() != 1 {
                return Err("Impossible de déterminer un dernier traitement unique. Choisissez tous les traitements ou Parcourir.".into());
            }
            latest
        }
        DuplicateImportChoice::ReplaceSelectedPath(selected_path) => {
            let chosen = Path::new(selected_path)
                .canonicalize()
                .map_err(|error| format!("Le dossier choisi n'est pas accessible : {error}"))?;
            let transcriptions = root
                .join("transcriptions")
                .canonicalize()
                .map_err(|error| {
                    format!("La destination ne contient pas de catalogue : {error}")
                })?;
            if !chosen.starts_with(&transcriptions) {
                return Err("Le traitement doit appartenir à la destination courante.".into());
            }
            let matches = verified
                .into_iter()
                .filter(|item| {
                    let path = root
                        .join("transcriptions")
                        .join(format!("{:032x}", item.job_id.0))
                        .join(item.generation.get().to_string());
                    path.canonicalize().ok().as_deref() == Some(chosen.as_path())
                })
                .collect::<Vec<_>>();
            if matches.len() != 1 {
                return Err("Le dossier choisi n'est pas un traitement vérifié du même audio dans cette destination.".into());
            }
            matches
        }
        _ => return Err("Choix de remplacement non pris en charge".into()),
    };
    if selected.is_empty() {
        return Err("Aucun traitement vérifié et présent ne peut être remplacé.".into());
    }
    Ok(selected
        .into_iter()
        .map(|item| TreatmentIdentity {
            destination: root.to_string_lossy().into_owned(),
            job_id: item.job_id,
            generation: item.generation,
        })
        .collect())
}

pub(in crate::worker_ipc) fn verified_present_publications(
    publications: Vec<PublishedImport>,
) -> Vec<PublishedImport> {
    publications
        .into_iter()
        .filter(|publication| publication.verified_present)
        .collect()
}

impl ImportRuntime {
    fn confirm_interrupted_publication(
        &mut self,
        mut prompt: DuplicateImportPrompt,
        choice: DuplicateImportChoice,
        events: &mut VecDeque<ImportEvent>,
    ) {
        let Some(previous) = prompt.resume_from.clone() else {
            return;
        };
        let Some(reserved_basename) = prompt.reserved_export_basename.clone() else {
            events.push_back(ImportEvent::Failed {
                job_id: prompt.request.job_id,
                generation: prompt.request.generation,
                message: "StorageCorrupt: interrupted export prompt is missing its reserved name"
                    .into(),
            });
            return;
        };
        let mut request = prompt.request.clone();
        let result = self
            .bind_import_request(&request)
            .map(|bound| request = bound)
            .and_then(|()| crate::archive::ArchiveStore::identify_source(&mut request))
            .and_then(|()| self.bind_import_request(&request).map(|bound| request = bound))
            .and_then(|()| {
                crate::archive::assert_campaign_archive(Path::new(&request.destination))
            })
            .and_then(|()| {
                let destination = PathBuf::from(&request.destination);
                let source_sha256 = request
                    .source_sha256
                    .as_ref()
                    .map(format_sha256)
                    .ok_or_else(|| "SourceChanged: resume source identity is missing".to_owned())?;
                let same_request = request.job_id == previous.job_id
                    && previous.generation.next() == Some(request.generation)
                    && request.source_path == previous.source_path
                    && request.destination == previous.destination
                    && previous.source_sha256.map(|sha| format_sha256(&sha)).as_deref()
                        == Some(source_sha256.as_str())
                    && previous.source_samples == request.source_samples
                    && request.model_sha256 == previous.model_sha256;
                let source_matches = same_request
                    && source_sha256 == prompt.source_sha256
                    && request.source_samples == Some(prompt.source_samples);
                let current = crate::archive::ArchiveStore::published_imports(
                    &destination,
                    &source_sha256,
                )?;
                if !source_matches || current != prompt.publications {
                    prompt.request = request;
                    prompt.source_sha256 = source_sha256;
                    prompt.source_samples = prompt.request.source_samples.unwrap_or(0);
                    prompt.publications = current;
                    prompt.revalidation_notice = Some(
                        "La source ou les transcriptions existantes ont changé. Vérifiez l'état actuel avant de reprendre.".into(),
                    );
                    events.push_back(ImportEvent::DuplicateImportFound(prompt.clone()));
                    return Ok(None);
                }
                let current_basename =
                    crate::archive::ArchiveStore::suggest_duplicate_export_basename(
                        &destination,
                        &request.source_path,
                    )?;
                if current_basename != prompt.export_basename {
                    prompt.request = request;
                    prompt.export_basename = current_basename;
                    prompt.revalidation_notice = Some(
                        "Le nom proposé n'est plus disponible. Vérifiez le nouveau nom avant de confirmer la reprise.".into(),
                    );
                    events.push_back(ImportEvent::DuplicateImportFound(prompt.clone()));
                    return Ok(None);
                }
                match crate::archive::ArchiveStore::reconfirm_interrupted_publication(
                    &destination,
                    &previous,
                    &source_sha256,
                    prompt.source_samples,
                    &current,
                    choice,
                    &reserved_basename,
                    &prompt.export_basename,
                ) {
                    Ok(()) => Ok(Some(request)),
                    Err(error) if error.starts_with("DuplicateExportNameChanged:") => {
                        prompt.request = request;
                        prompt.export_basename =
                            crate::archive::ArchiveStore::suggest_duplicate_export_basename(
                                &destination,
                                &prompt.request.source_path,
                            )?;
                        prompt.revalidation_notice = Some(
                            "Le nom proposé a été pris pendant la confirmation. Vérifiez ce nouveau nom avant de reprendre.".into(),
                        );
                        events.push_back(ImportEvent::DuplicateImportFound(prompt.clone()));
                        Ok(None)
                    }
                    Err(error) => Err(error),
                }
            });
        match result {
            Ok(Some(request)) => self.effect(
                ImportEffect::ResumeInterrupted {
                    previous: Box::new(previous),
                    request,
                },
                events,
            ),
            Ok(None) => {}
            Err(message) => events.push_back(ImportEvent::Failed {
                job_id: prompt.request.job_id,
                generation: prompt.request.generation,
                message,
            }),
        }
    }

    pub(in crate::worker_ipc) fn confirm_duplicate_import(
        &mut self,
        mut prompt: DuplicateImportPrompt,
        choice: DuplicateImportChoice,
        events: &mut VecDeque<ImportEvent>,
    ) -> DuplicateEffect {
        if prompt.resume_from.is_some() {
            self.confirm_interrupted_publication(prompt, choice, events);
            return DuplicateEffect::Return;
        }
        let submitted = (prompt.request.job_id, prompt.request.generation);
        let _timing_context = TimingContextGuard::enter(
            "confirm-duplicate-import",
            format!("{:032x}-{}", submitted.0.0, submitted.1.get()),
            Path::new(&prompt.request.destination),
        );
        let started = Instant::now();
        let mut request = prompt.request.clone();
        let mut binding_key = submitted;
        let binding_destination = PathBuf::from(&request.destination);
        if choice == DuplicateImportChoice::CreateNew
            && self.require_snapshot_registry
            && self.snapshot_registry_root.is_none()
        {
            events.push_back(ImportEvent::Failed {
                        job_id: submitted.0,
                        generation: submitted.1,
                        message: "SourceSnapshotRegistryUnavailable: Créer un nouveau requiert le registre de snapshots de l'application".into(),
                    });
            return DuplicateEffect::Return;
        }
        let result = self
                    .bind_import_request(&request)
                    .map(|bound| { request = bound; () })
                    .and_then(|()| self.ensure_source_binding(&mut request))
                    .and_then(|()| self.bind_import_request(&request).map(|bound| { request = bound; () }))
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
                            .ok_or_else(|| "SourceChanged: source identity is missing".to_owned())?;
                        let is_reprocessing_choice = matches!(
                            &choice,
                            DuplicateImportChoice::ReplaceLatest
                                | DuplicateImportChoice::ReplaceAll
                                | DuplicateImportChoice::ReplaceSelectedPath(_)
                        );
                        let current = if choice == DuplicateImportChoice::CreateNew
                            && self.snapshot_registry_root.is_some()
                        {
                            Vec::new()
                        } else if is_reprocessing_choice {
                            verified_present_publications(
                                crate::archive::ArchiveStore::reprocessing_catalog(destination, &source_sha)?,
                            )
                        } else {
                            crate::archive::ArchiveStore::published_imports(
                                destination,
                                &source_sha,
                            )?
                        };
                        let source_matches = source_sha == prompt.source_sha256
                            && request.source_samples == Some(prompt.source_samples)
                            && request.destination == prompt.request.destination;
                        if !source_matches
                            || ((choice != DuplicateImportChoice::CreateNew
                                || self.snapshot_registry_root.is_none())
                                && current != prompt.publications)
                        {
                            prompt.request = request;
                            prompt.source_sha256 = source_sha;
                            prompt.source_samples = prompt.request.source_samples.unwrap_or(0);
                            prompt.publications = current;
                            prompt.revalidation_notice = Some(
                                "La source ou les transcriptions existantes ont changé. Vérifiez l'état actuel avant de choisir.".into(),
                            );
                            return Ok(PreflightResult::Duplicate(prompt));
                        }
                        let current_basename = if choice == DuplicateImportChoice::Replace {
                            Some(crate::archive::ArchiveStore::suggest_duplicate_export_basename(
                                destination,
                                &request.source_path,
                            )?)
                        } else {
                            None
                        };
                        if current_basename.as_deref().is_some_and(|name| name != prompt.export_basename) {
                            let current_basename = current_basename.expect("checked Some");
                            prompt.export_basename = current_basename;
                            prompt.revalidation_notice = Some(
                                "Le nom de sortie proposé n'est plus disponible. Vérifiez le nouveau nom avant de confirmer.".into(),
                            );
                            return Ok(PreflightResult::Duplicate(prompt));
                        }
                        let mut queue = crate::queue_store::QueueStore::open(destination)?;
                        let original = (request.job_id, request.generation);
                        request.job_id = collision_free_job_id(
                            destination,
                            request.job_id,
                            request.generation,
                            &queue.entries(),
                        );
                        let reservation = if choice == DuplicateImportChoice::CreateNew
                            && self.snapshot_registry_root.is_some()
                        {
                            crate::archive::ArchiveStore::reserve_create_only_import(
                                destination,
                                &request,
                                &source_sha,
                                request.source_samples.unwrap_or(0),
                            )
                        } else if is_reprocessing_choice {
                            match resolve_reprocessing_targets(destination, &current, &choice) {
                                Ok(targets) => crate::archive::ArchiveStore::reserve_reprocessing_import(
                                    destination,
                                    &request,
                                    &source_sha,
                                    request.source_samples.unwrap_or(0),
                                    &current,
                                    &targets,
                                    choice.clone(),
                                ),
                                Err(error) => {
                                    prompt.request = request;
                                    prompt.publications = current;
                                    prompt.revalidation_notice = Some(error);
                                    return Ok(PreflightResult::Duplicate(prompt));
                                }
                            }
                        } else {
                            crate::archive::ArchiveStore::reserve_duplicate_import(
                                destination,
                                &request,
                                &source_sha,
                                request.source_samples.unwrap_or(0),
                                &current,
                                choice.clone(),
                                &prompt.export_basename,
                            )
                        };
                        if let Err(error) = reservation {
                            if error.starts_with("DuplicateExportNameChanged:") {
                                prompt.export_basename = crate::archive::ArchiveStore::suggest_duplicate_export_basename(
                                    destination,
                                    &request.source_path,
                                )?;
                                prompt.revalidation_notice = Some(
                                    "Le nom de sortie proposé n'est plus disponible. Vérifiez le nouveau nom avant de confirmer.".into(),
                                );
                                return Ok(PreflightResult::Duplicate(prompt));
                            }
                            return Err(error);
                        }
                        if let Err(error) = self.rebind_source_binding(original, (request.job_id, request.generation)) {
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
                            let _ = self.release_source_binding(request.job_id, request.generation, destination);
                            return Err(error);
                        }
                        Ok(PreflightResult::Queued {
                            submitted: original,
                            admitted,
                            entries: queue.entries(),
                        })
                    });
        record_stage_timing(
            "import-confirmation",
            started.elapsed(),
            serde_json::json!({
                "operation": format!("{:032x}-{}", submitted.0.0, submitted.1.get()),
                "pid": std::process::id(),
                "phase": "confirmation",
                "choice": match choice {
                    DuplicateImportChoice::CreateNew => "create-new",
                    DuplicateImportChoice::Replace => "replace",
                    DuplicateImportChoice::ReplaceLatest => "replace-latest",
                    DuplicateImportChoice::ReplaceAll => "replace-all",
                    DuplicateImportChoice::ReplaceSelectedPath(_) => "replace-selected",
                },
                "outcome": match result.as_ref() {
                    Ok(PreflightResult::Duplicate(_)) => "reconfirmation-required",
                    Ok(PreflightResult::Queued { .. }) => "queued",
                    Err(_) => "error",
                },
            }),
        );
        DuplicateEffect::Continue(match result {
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
        })
    }
}
