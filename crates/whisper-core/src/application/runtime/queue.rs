use super::*;

impl<P: ImportIoPort> ImportApplication<P> {
    pub(super) fn handle_queue_event(
        &mut self,
        event: &ImportEvent,
    ) -> Result<bool, ApplicationError> {
        match event {
            ImportEvent::History(_)
            | ImportEvent::ReplacementCleanups(_)
            | ImportEvent::SourceSnapshotRecoveryInspected(_)
            | ImportEvent::SourceSnapshotRecoveryCompleted { .. } => {
                self.handle_history_event(event)
            }
            ImportEvent::Queue(_) | ImportEvent::QueueAdmission { .. } => {
                self.handle_queue_admission_event(event)
            }
            ImportEvent::DuplicateImportFound(prompt) => {
                self.handle_duplicate_event(prompt);
                Ok(true)
            }
            ImportEvent::Failed { .. } => self.handle_queue_failure_event(event),
            _ => Ok(false),
        }
    }

    fn handle_history_event(&mut self, event: &ImportEvent) -> Result<bool, ApplicationError> {
        match event {
            ImportEvent::History(history) => {
                self.view.history = history.clone();
                self.view.history_scan_pending = false;
                return Ok(true);
            }
            ImportEvent::ReplacementCleanups(cleanups) => {
                self.view.replacement_cleanups = cleanups.clone();
                return Ok(true);
            }
            ImportEvent::SourceSnapshotRecoveryInspected(candidates) => {
                if candidates.is_empty() {
                    self.view.source_snapshot_recovery = None;
                    self.view.message = Some(
                        "Inspection terminée : aucune copie audio retenue récupérable.".into(),
                    );
                } else {
                    self.view.source_snapshot_recovery = Some(candidates.clone());
                    self.view.message = Some(
                        "Inspection terminée : vérifiez les copies conservées avant toute récupération.".into(),
                    );
                }
                return Ok(true);
            }
            ImportEvent::SourceSnapshotRecoveryCompleted {
                freed,
                candidates,
                error,
            } => {
                self.view.source_snapshot_recovery = None;
                self.view.source_snapshot_recovery_available = false;
                let protected_count = candidates
                    .iter()
                    .filter(|candidate| !candidate.recoverable)
                    .count();
                let protected = candidates
                    .iter()
                    .filter(|candidate| !candidate.recoverable)
                    .take(4)
                    .map(|candidate| {
                        candidate.reason.as_deref().map_or_else(
                            || {
                                format!(
                                    "Job {:032x}/génération {} : protégé",
                                    candidate.job_id.0,
                                    candidate.generation.get()
                                )
                            },
                            |reason| {
                                format!(
                                    "Job {:032x}/génération {} : {reason}",
                                    candidate.job_id.0,
                                    candidate.generation.get()
                                )
                            },
                        )
                    })
                    .collect::<Vec<_>>();
                let mut message = if let Some(error) = error {
                    format!("La récupération a échoué : {error}")
                } else if *freed > 0 {
                    format!(
                        "{freed} copie(s) audio retenue(s) récupérée(s). Vous pouvez relancer l’import."
                    )
                } else if candidates.is_empty() {
                    "Aucune copie audio retenue récupérable.".into()
                } else {
                    "Aucune copie sûre n’a été libérée.".into()
                };
                if !protected.is_empty() {
                    message.push_str(" Copies protégées conservées : ");
                    message.push_str(&protected.join(" ; "));
                    let omitted = protected_count.saturating_sub(protected.len());
                    if omitted > 0 {
                        message.push_str(&format!(" ; et {omitted} autre(s) copie(s) protégée(s)"));
                    }
                }
                self.view.message = Some(message);
                return Ok(true);
            }
            _ => {}
        }
        Ok(false)
    }

    fn handle_queue_admission_event(
        &mut self,
        event: &ImportEvent,
    ) -> Result<bool, ApplicationError> {
        match event {
            ImportEvent::Queue(queue) => {
                self.view.queue_scan_pending = false;
                let acknowledged = self
                    .pending_enqueues
                    .iter()
                    .filter(|identity| {
                        queue.iter().any(|entry| {
                            entry.request.job_id == identity.job_id
                                && entry.request.generation == identity.generation
                        })
                    })
                    .count();
                self.pending_enqueues.retain(|identity| {
                    !queue.iter().any(|entry| {
                        entry.request.job_id == identity.job_id
                            && entry.request.generation == identity.generation
                    })
                });
                if acknowledged > 0 {
                    self.view.message = Some(format!(
                        "{acknowledged} import(s) ajouté(s) à la file durable. Choisissez Traiter pour les démarrer."
                    ));
                }
                self.view.queued_jobs = queue.len();
                self.view.queue = queue.clone();
                self.close_queue_removals.retain(|identity| {
                    queue.iter().any(|entry| {
                        (entry.request.job_id, entry.request.generation)
                            == (identity.job_id, identity.generation)
                    })
                });
                self.drive_close_queue_removals()?;
                return Ok(true);
            }
            ImportEvent::QueueAdmission {
                submitted_job_id,
                submitted_generation,
                queue,
            } => {
                self.view.queue_scan_pending = false;
                let submitted = CommandIdentity {
                    job_id: *submitted_job_id,
                    generation: *submitted_generation,
                };
                let acknowledged = self.pending_enqueues.contains(&submitted);
                self.pending_enqueues
                    .retain(|identity| *identity != submitted);
                if acknowledged {
                    self.view.message = Some(
                        "Import ajouté à la file durable avec un nouvel identifiant disponible. Choisissez Traiter pour le démarrer.".into(),
                    );
                }
                self.view.queued_jobs = queue.len();
                self.view.queue = queue.clone();
                self.close_queue_removals.retain(|identity| {
                    queue.iter().any(|entry| {
                        (entry.request.job_id, entry.request.generation)
                            == (identity.job_id, identity.generation)
                    })
                });
                self.drive_close_queue_removals()?;
                return Ok(true);
            }
            _ => {}
        }
        Ok(false)
    }

    fn handle_duplicate_event(&mut self, prompt: &crate::ports::DuplicateImportPrompt) {
        self.pending_enqueues.retain(|identity| {
            *identity
                != CommandIdentity {
                    job_id: prompt.request.job_id,
                    generation: prompt.request.generation,
                }
        });
        if prompt.resume_from.is_some() {
            self.active = None;
            self.request = None;
            self.stopping = None;
            self.view.active_job = None;
            self.view.active_generation = None;
            self.view.progress = None;
        }
        self.view.duplicate_import_prompt = Some(prompt.clone());
        self.view.message = None;
    }

    fn handle_queue_failure_event(
        &mut self,
        event: &ImportEvent,
    ) -> Result<bool, ApplicationError> {
        match event {
            ImportEvent::Failed {
                job_id, message, ..
            } if *job_id == JobId(0) && self.view.queue_scan_pending => {
                self.view.queue_scan_pending = false;
                self.view.message = Some(format!("File durable indisponible : {message}"));
                return Ok(true);
            }
            ImportEvent::Failed {
                job_id,
                generation,
                message,
            } => {
                let identity = CommandIdentity {
                    job_id: *job_id,
                    generation: *generation,
                };
                if self.close_queue_removals.contains(&identity) {
                    self.close_queue_removals
                        .retain(|pending| *pending != identity);
                    self.view.message = Some(format!(
                        "Retrait de l’import {} impossible : {message}. La file est conservée.",
                        job_id.0
                    ));
                    self.update_close_ready();
                    return Ok(true);
                }
            }
            _ => {}
        }
        Ok(false)
    }

    pub(super) fn handle_queue_command(
        &mut self,
        command: AppCommand,
    ) -> Result<AppView, ApplicationError> {
        match command {
            AppCommand::ScanHistory { .. }
            | AppCommand::InspectSourceSnapshotRecovery
            | AppCommand::ConfirmSourceSnapshotRecovery
            | AppCommand::CancelSourceSnapshotRecovery => self.handle_history_command(command),
            AppCommand::ResolveLivePublication { .. } | AppCommand::RemoveQueued { .. } => {
                self.handle_queue_removal_command(command)
            }
            AppCommand::EnqueueImport { .. } | AppCommand::ResolveDuplicateImport { .. } => {
                self.handle_enqueue_command(command)
            }
            AppCommand::CancelDuplicateImport
            | AppCommand::ResolveImportPublication { .. }
            | AppCommand::ResumeReplacementCleanup { .. } => {
                self.handle_queue_resolution_command(command)
            }
            _ => unreachable!("only queue commands are routed here"),
        }
    }

    fn handle_history_command(&mut self, command: AppCommand) -> Result<AppView, ApplicationError> {
        match command {
            AppCommand::ScanHistory { destination } => {
                self.io
                    .submit(ImportEffect::ScanHistory {
                        destination: destination.clone(),
                    })
                    .map_err(|e| self.fail(e))?;
                self.io
                    .submit(ImportEffect::ScanQueue { destination })
                    .map_err(|e| self.fail(e))?;
                self.view.history_scan_pending = true;
                self.view.queue_scan_pending = true;
            }
            AppCommand::InspectSourceSnapshotRecovery => {
                self.io
                    .submit(ImportEffect::InspectSourceSnapshotRecovery)
                    .map_err(|error| self.fail(error))?;
                self.view.source_snapshot_recovery = None;
            }
            AppCommand::ConfirmSourceSnapshotRecovery => {
                if self
                    .view
                    .source_snapshot_recovery
                    .as_ref()
                    .is_none_or(|candidates| {
                        !candidates.iter().any(|candidate| candidate.recoverable)
                    })
                {
                    return Err(ApplicationError::InvalidCommand);
                }
                self.io
                    .submit(ImportEffect::ConfirmSourceSnapshotRecovery)
                    .map_err(|error| self.fail(error))?;
                self.view.source_snapshot_recovery = None;
                self.view.message =
                    Some("Nouvelle vérification de sécurité avant récupération…".into());
            }
            AppCommand::CancelSourceSnapshotRecovery => {
                self.view.source_snapshot_recovery = None;
            }
            _ => unreachable!("queue command group already selected"),
        }
        Ok(self.view.clone())
    }

    fn handle_queue_removal_command(
        &mut self,
        command: AppCommand,
    ) -> Result<AppView, ApplicationError> {
        match command {
            AppCommand::ResolveLivePublication { request } => {
                if self.active.is_some() || request.destination.trim().is_empty() {
                    return Err(ApplicationError::InvalidCommand);
                }
                self.io
                    .submit(ImportEffect::ResolveLivePublication(request))
                    .map_err(|error| self.fail(error))?;
                self.view.history_scan_pending = true;
                self.view.message = Some(
                    "Vérification du passage récupérable et résolution explicite en cours…".into(),
                );
            }
            AppCommand::RemoveQueued { job_id, generation } => {
                self.io
                    .submit(ImportEffect::RemoveQueued {
                        job_id,
                        generation,
                        destination: self
                            .view
                            .queue
                            .iter()
                            .find(|item| {
                                item.request.job_id == job_id
                                    && item.request.generation == generation
                            })
                            .map(|item| item.request.destination.clone())
                            .ok_or(ApplicationError::InvalidCommand)?,
                    })
                    .map_err(|e| self.fail(e))?;
                if self.view.close_requested {
                    let identity = CommandIdentity { job_id, generation };
                    if !self.close_queue_removals.contains(&identity) {
                        self.close_queue_removals.push(identity);
                    }
                }
            }
            _ => unreachable!("queue command group already selected"),
        }
        Ok(self.view.clone())
    }

    fn handle_enqueue_command(&mut self, command: AppCommand) -> Result<AppView, ApplicationError> {
        match command {
            AppCommand::EnqueueImport { request } => {
                if self.direct_live && self.active.is_some() {
                    return Err(ApplicationError::InvalidCommand);
                }
                let identity = CommandIdentity {
                    job_id: request.job_id,
                    generation: request.generation,
                };
                self.io
                    .submit(ImportEffect::Enqueue(request))
                    .map_err(|e| self.fail(e))?;
                self.pending_enqueues.push(identity);
                self.view.message =
                    Some("Vérification de la source et ajout à la file durable en cours…".into());
            }
            AppCommand::ResolveDuplicateImport { prompt, choice } => {
                if self.active.is_some() {
                    return Err(ApplicationError::InvalidCommand);
                }
                let identity = CommandIdentity {
                    job_id: prompt.request.job_id,
                    generation: prompt.request.generation,
                };
                if self.view.duplicate_import_prompt.as_ref() != Some(&prompt) {
                    return Err(ApplicationError::InvalidCommand);
                }
                let is_resume_confirmation = prompt.resume_from.is_some();
                let confirmed_request = prompt.request.clone();
                self.io
                    .submit(ImportEffect::ConfirmDuplicateImport { prompt, choice })
                    .map_err(|e| self.fail(e))?;
                if is_resume_confirmation {
                    self.request = Some(confirmed_request);
                    self.active = Some(identity);
                    self.stopping = None;
                    self.next_sequence = 1;
                    self.pending_persistence = 0;
                    self.ended_at = None;
                    self.max_segment_offset = 0;
                    self.final_source_samples = None;
                    self.view.active_job = Some((identity.job_id, JobState::Queued));
                    self.view.active_generation = Some(identity.generation);
                    self.view.progress = None;
                } else {
                    self.pending_enqueues.push(identity);
                }
                self.view.duplicate_import_prompt = None;
                self.view.message = Some(if is_resume_confirmation {
                    "Confirmation de reprise en cours…".into()
                } else {
                    "Confirmation en cours…".into()
                });
            }
            _ => unreachable!("queue command group already selected"),
        }
        Ok(self.view.clone())
    }

    fn handle_queue_resolution_command(
        &mut self,
        command: AppCommand,
    ) -> Result<AppView, ApplicationError> {
        match command {
            AppCommand::CancelDuplicateImport => {
                if let Some(prompt) = self.view.duplicate_import_prompt.clone() {
                    self.io
                        .submit(ImportEffect::ReleaseSourceBinding {
                            job_id: prompt.request.job_id,
                            generation: prompt.request.generation,
                            destination: prompt.request.destination.clone(),
                        })
                        .map_err(|error| self.fail(error))?;
                    self.view.duplicate_import_prompt = None;
                }
                self.view.message = Some("Nouvelle transcription annulée.".into());
            }
            AppCommand::ResolveImportPublication {
                destination,
                job_id,
                generation,
            } => {
                if self.active.is_some() || destination.trim().is_empty() {
                    return Err(ApplicationError::InvalidCommand);
                }
                self.io
                    .submit(ImportEffect::ResolveImportPublication {
                        destination,
                        job_id,
                        generation,
                    })
                    .map_err(|error| self.fail(error))?;
                self.view.history_scan_pending = true;
                self.view.message =
                    Some("Vérification et récupération de la publication interrompue…".into());
            }
            AppCommand::ResumeReplacementCleanup {
                destination,
                cleanup_id,
            } => {
                if self.active.is_some() || destination.trim().is_empty() || cleanup_id.is_empty() {
                    return Err(ApplicationError::InvalidCommand);
                }
                self.io
                    .submit(ImportEffect::ResumeReplacementCleanup {
                        destination,
                        cleanup_id,
                    })
                    .map_err(|error| self.fail(error))?;
                self.view.message =
                    Some("Vérification des éléments restants et reprise demandée…".into());
            }
            _ => unreachable!("queue command group already selected"),
        }
        Ok(self.view.clone())
    }
}
