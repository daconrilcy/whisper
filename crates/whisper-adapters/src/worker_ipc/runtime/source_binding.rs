use super::*;

impl ImportRuntime {
    pub(in crate::worker_ipc) fn ensure_source_binding(
        &mut self,
        request: &mut ImportRequest,
    ) -> Result<(), String> {
        let Some(registry_root) = self.snapshot_registry_root.clone() else {
            if self.require_snapshot_registry {
                return Err("SourceSnapshotRegistryUnavailable: application settings root is not configured".into());
            }
            return crate::archive::ArchiveStore::identify_source(request);
        };
        let key = (request.job_id, request.generation.get());
        if let std::collections::hash_map::Entry::Vacant(entry) = self.source_bindings.entry(key) {
            let binding = crate::source_binding::SourceBinding::acquire(request, &registry_root)?;
            entry.insert(binding);
        }
        let binding = self
            .source_bindings
            .get(&key)
            .expect("binding inserted or already present");
        if binding.original_path() != request.source_path
            || binding.strategy() != request.strategy
            || request
                .source_sha256
                .is_some_and(|sha| sha != binding.source_sha256())
            || request
                .source_samples
                .is_some_and(|samples| samples != binding.source_samples())
        {
            return Err(
                "SourceSnapshotMismatch: stored source binding differs from the request".into(),
            );
        }
        request.source_sha256 = Some(binding.source_sha256());
        request.source_samples = Some(binding.source_samples());
        Ok(())
    }

    pub(in crate::worker_ipc) fn rebind_source_binding(
        &mut self,
        old: (JobId, Generation),
        new: (JobId, Generation),
    ) -> Result<(), String> {
        if old == new {
            return Ok(());
        }
        if !self.source_bindings.contains_key(&(old.0, old.1.get()))
            && !self.require_snapshot_registry
        {
            return Ok(());
        }
        let binding = self
            .source_bindings
            .remove(&(old.0, old.1.get()))
            .ok_or_else(|| {
                "SourceSnapshotUnavailable: accepted source binding is missing".to_owned()
            })?;
        let binding = binding.rebind(new.0, new.1)?;
        self.source_bindings.insert((new.0, new.1.get()), binding);
        Ok(())
    }

    pub(in crate::worker_ipc) fn source_binding_matches_resume(
        binding: &crate::source_binding::SourceBinding,
        previous: &ImportRequest,
    ) -> Result<(), String> {
        if binding.original_path() != previous.source_path
            || binding.strategy() != previous.strategy
            || previous
                .source_sha256
                .is_some_and(|sha| sha != binding.source_sha256())
            || previous
                .source_samples
                .is_some_and(|samples| samples != binding.source_samples())
        {
            return Err(
                "SourceSnapshotMismatch: durable resume binding differs from the interrupted request"
                    .into(),
            );
        }
        Ok(())
    }

    pub(in crate::worker_ipc) fn transfer_resume_source_binding(
        &mut self,
        previous: &ImportRequest,
        resumed: &ImportRequest,
    ) -> Result<bool, String> {
        let result = self.transfer_resume_source_binding_inner(previous, resumed);
        #[cfg(test)]
        self.observe_source_binding_map("resume-transfer");
        result
    }

    pub(in crate::worker_ipc) fn transfer_resume_source_binding_inner(
        &mut self,
        previous: &ImportRequest,
        resumed: &ImportRequest,
    ) -> Result<bool, String> {
        let old_key = (previous.job_id, previous.generation.get());
        let new_key = (resumed.job_id, resumed.generation.get());
        if previous.job_id != resumed.job_id
            || previous.generation.next() != Some(resumed.generation)
            || previous.destination != resumed.destination
            || previous.source_path != resumed.source_path
            || previous.source_sha256 != resumed.source_sha256
            || previous.source_samples != resumed.source_samples
            || previous.strategy != resumed.strategy
        {
            return Err(
                "SourceSnapshotMismatch: resumed request changed the interrupted source identity"
                    .into(),
            );
        }
        if old_key == new_key {
            return Ok(false);
        }

        let old_memory_binding = self.source_bindings.remove(&old_key);
        let new_memory_binding = self.source_bindings.remove(&new_key);
        let Some(registry_root) = self.snapshot_registry_root.clone() else {
            if let Some(binding) = old_memory_binding {
                self.source_bindings.insert(old_key, binding);
            }
            if let Some(binding) = new_memory_binding {
                self.source_bindings.insert(new_key, binding);
            }
            if self.source_bindings.contains_key(&old_key)
                || self.source_bindings.contains_key(&new_key)
            {
                return Err(
                    "SourceSnapshotRegistryUnavailable: cannot transfer a resume binding without its configured registry root".into(),
                );
            }
            return if self.require_snapshot_registry {
                Err(
                    "SourceSnapshotRegistryUnavailable: resume settings root is not configured"
                        .into(),
                )
            } else {
                Ok(false)
            };
        };

        let old_binding = match old_memory_binding {
            Some(binding) => Some(binding),
            None => match crate::source_binding::SourceBinding::reopen_for_identity(
                &registry_root,
                Path::new(&previous.destination),
                previous.job_id,
                previous.generation,
            ) {
                Ok(binding) => binding,
                Err(error) => {
                    if let Some(binding) = new_memory_binding {
                        self.source_bindings.insert(new_key, binding);
                    }
                    return Err(error);
                }
            },
        };
        let new_binding = match new_memory_binding {
            Some(binding) => Some(binding),
            None => match crate::source_binding::SourceBinding::reopen_for_identity(
                &registry_root,
                Path::new(&resumed.destination),
                resumed.job_id,
                resumed.generation,
            ) {
                Ok(binding) => binding,
                Err(error) => {
                    if let Some(binding) = old_binding {
                        self.source_bindings.insert(old_key, binding);
                    }
                    return Err(error);
                }
            },
        };
        match (old_binding, new_binding) {
            (Some(old_binding), Some(new_binding)) => {
                self.source_bindings.insert(old_key, old_binding);
                self.source_bindings.insert(new_key, new_binding);
                Err("SourceSnapshotMismatch: interrupted and resumed identities are both durably bound".into())
            }
            (Some(binding), None) => {
                if let Err(error) = Self::source_binding_matches_resume(&binding, previous) {
                    self.source_bindings.insert(old_key, binding);
                    return Err(error);
                }
                match binding.rebind(resumed.job_id, resumed.generation) {
                    Ok(binding) => {
                        self.source_bindings.insert(new_key, binding);
                        Ok(true)
                    }
                    Err(error) => {
                        if let Some(binding) =
                            crate::source_binding::SourceBinding::reopen_for_identity(
                                &registry_root,
                                Path::new(&previous.destination),
                                previous.job_id,
                                previous.generation,
                            )?
                        {
                            self.source_bindings.insert(old_key, binding);
                            return Err(error);
                        }
                        if let Some(binding) =
                            crate::source_binding::SourceBinding::reopen_for_identity(
                                &registry_root,
                                Path::new(&resumed.destination),
                                resumed.job_id,
                                resumed.generation,
                            )?
                        {
                            Self::source_binding_matches_resume(&binding, previous)?;
                            self.source_bindings.insert(new_key, binding);
                            return Err(format!(
                                "{error}; source binding now belongs to the resumed identity and remains retained"
                            ));
                        }
                        Err(format!(
                            "{error}; SourceSnapshotUnavailable: neither interrupted nor resumed binding could be reopened"
                        ))
                    }
                }
            }
            (None, Some(binding)) => {
                Self::source_binding_matches_resume(&binding, previous)?;
                self.source_bindings.insert(new_key, binding);
                // A prior interrupted attempt may have moved the durable binding
                // before failing to advance the queue. Restore it if this attempt
                // also fails before the queue transition commits.
                Ok(true)
            }
            (None, None) if self.require_snapshot_registry => Err(
                "SourceSnapshotUnavailable: interrupted request has no durable source binding"
                    .into(),
            ),
            (None, None) => Ok(false),
        }
    }

    pub(in crate::worker_ipc) fn restore_resume_source_binding(
        &mut self,
        previous: &ImportRequest,
        resumed: &ImportRequest,
    ) -> Result<(), String> {
        let result = self.restore_resume_source_binding_inner(previous, resumed);
        #[cfg(test)]
        self.observe_source_binding_map("resume-rollback");
        result
    }

    pub(in crate::worker_ipc) fn restore_resume_source_binding_inner(
        &mut self,
        previous: &ImportRequest,
        resumed: &ImportRequest,
    ) -> Result<(), String> {
        let old_key = (previous.job_id, previous.generation.get());
        let new_key = (resumed.job_id, resumed.generation.get());
        let new_binding = match self.source_bindings.remove(&new_key) {
            Some(binding) => binding,
            None => {
                let Some(registry_root) = self.snapshot_registry_root.clone() else {
                    return if self.require_snapshot_registry {
                        Err("SourceSnapshotRegistryUnavailable: resume settings root is not configured".into())
                    } else {
                        Ok(())
                    };
                };
                crate::source_binding::SourceBinding::reopen_for_identity(
                    &registry_root,
                    Path::new(&resumed.destination),
                    resumed.job_id,
                    resumed.generation,
                )?
                .ok_or_else(|| {
                    "SourceSnapshotUnavailable: resumed source binding is missing during rollback"
                        .to_owned()
                })?
            }
        };
        match new_binding.rebind(previous.job_id, previous.generation) {
            Ok(binding) => {
                Self::source_binding_matches_resume(&binding, previous)?;
                self.source_bindings.insert(old_key, binding);
                Ok(())
            }
            Err(error) => {
                if let Some(registry_root) = self.snapshot_registry_root.clone()
                    && let Some(binding) =
                        crate::source_binding::SourceBinding::reopen_for_identity(
                            &registry_root,
                            Path::new(&resumed.destination),
                            resumed.job_id,
                            resumed.generation,
                        )?
                {
                    self.source_bindings.insert(new_key, binding);
                }
                Err(format!(
                    "{error}; resumed source binding remains retained if its durable identity could be reopened"
                ))
            }
        }
    }

    pub(in crate::worker_ipc) fn release_source_binding(
        &mut self,
        job_id: JobId,
        generation: Generation,
        destination: &Path,
    ) -> Result<(), String> {
        let result = self.release_source_binding_inner(job_id, generation, destination);
        #[cfg(test)]
        self.observe_source_binding_map("release");
        result
    }

    pub(in crate::worker_ipc) fn release_source_binding_inner(
        &mut self,
        job_id: JobId,
        generation: Generation,
        destination: &Path,
    ) -> Result<(), String> {
        let session = whisper_core::parakeet_scheduler::SchedulerSession { job_id, generation };
        if let Some(binding) = self.source_bindings.remove(&(job_id, generation.get())) {
            return binding.retire();
        }
        if self.unified_capture_sessions.contains(&session) {
            let stem = format!("{:032x}-{}", job_id.0, generation.get());
            let snapshot_dir = destination.join(".whisper-source-snapshots");
            let inspect_snapshot_artifacts = || -> Result<bool, String> {
                let mut found = false;
                for suffix in ["json", "snapshot", "snapshot.partial"] {
                    let artifact = snapshot_dir.join(format!("{stem}.{suffix}"));
                    match std::fs::symlink_metadata(&artifact) {
                        Ok(_) => found = true,
                        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                        Err(error) => {
                            return Err(format!(
                                "SourceSnapshotRegistryUnavailable: inspect capture snapshot artifact {}: {error}",
                                artifact.display()
                            ));
                        }
                    }
                }
                Ok(found)
            };
            let has_snapshot_artifact = inspect_snapshot_artifacts()?;
            let Some(registry_root) = self.snapshot_registry_root.as_deref() else {
                return if has_snapshot_artifact {
                    Err("SourceSnapshotRegistryUnavailable: capture identity has durable snapshot artifacts but no registry root is configured".into())
                } else {
                    Ok(())
                };
            };
            match std::fs::symlink_metadata(registry_root) {
                Ok(metadata) if metadata.is_dir() => {}
                Ok(_) => {
                    return Err(format!(
                        "SourceSnapshotRegistryUnavailable: registry root {} is not a directory",
                        registry_root.display()
                    ));
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    return if has_snapshot_artifact {
                        Err(format!(
                            "SourceSnapshotRegistryUnavailable: capture identity has durable snapshot artifacts but registry root {} is absent",
                            registry_root.display()
                        ))
                    } else {
                        Ok(())
                    };
                }
                Err(error) => {
                    return Err(format!(
                        "SourceSnapshotRegistryUnavailable: inspect registry root {}: {error}",
                        registry_root.display()
                    ));
                }
            }

            let lock_path = registry_root.join("reprocessing-snapshots.lock");
            let state_path = registry_root.join("reprocessing-snapshots.json");
            let has_lock_or_state = [lock_path, state_path].iter().try_fold(
                false,
                |present, path| match std::fs::symlink_metadata(path) {
                    Ok(_) => Ok(true),
                    Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(present),
                    Err(error) => Err(format!(
                        "SourceSnapshotRegistryUnavailable: inspect registry file {}: {error}",
                        path.display()
                    )),
                },
            )?;
            if has_lock_or_state {
                crate::source_binding::SourceBinding::retire_for_identity(
                    registry_root,
                    destination,
                    job_id,
                    generation,
                )?;
                if has_snapshot_artifact && inspect_snapshot_artifacts()? {
                    return Err(
                        "SourceSnapshotOrphaned: capture identity has snapshot artifacts without a matching durable registry binding".into(),
                    );
                }
                return Ok(());
            }
            if has_snapshot_artifact {
                return Err(
                    "SourceSnapshotOrphaned: capture identity has snapshot artifacts but the registry is uninitialized".into(),
                );
            }
            let mut contents = std::fs::read_dir(registry_root).map_err(|error| {
                format!(
                    "SourceSnapshotRegistryUnavailable: inspect registry root {}: {error}",
                    registry_root.display()
                )
            })?;
            if contents
                .next()
                .transpose()
                .map_err(|error| {
                    format!(
                        "SourceSnapshotRegistryUnavailable: inspect registry root {}: {error}",
                        registry_root.display()
                    )
                })?
                .is_some()
            {
                return Err(format!(
                    "SourceSnapshotRegistryUnavailable: uninitialized registry root {} is not empty",
                    registry_root.display()
                ));
            }
            return Ok(());
        }
        let Some(registry_root) = self.snapshot_registry_root.as_deref() else {
            return Ok(());
        };
        crate::source_binding::SourceBinding::retire_for_identity(
            registry_root,
            destination,
            job_id,
            generation,
        )
    }

    pub(in crate::worker_ipc) fn active_snapshot_bindings(&self) -> Vec<(JobId, u64)> {
        let mut active = self.source_bindings.keys().copied().collect::<Vec<_>>();
        if let Some(request) = &self.active {
            active.push((request.job_id, request.generation.get()));
        }
        active.sort_unstable_by_key(|identity| (identity.0.0, identity.1));
        active.dedup();
        active
    }

    pub(in crate::worker_ipc) fn inspect_source_snapshot_recovery(
        &self,
    ) -> Result<Vec<whisper_core::ports::SourceSnapshotRecoveryCandidate>, String> {
        let registry_root = self.snapshot_registry_root.as_deref().ok_or_else(|| {
            "SourceSnapshotRegistryUnavailable: application settings root is not configured"
                .to_owned()
        })?;
        crate::source_binding::SourceBinding::inspect_recovery_candidates(
            registry_root,
            &self.active_snapshot_bindings(),
        )
    }

    pub(in crate::worker_ipc) fn inspect_snapshot_recovery_effect(
        &mut self,
        events: &mut VecDeque<ImportEvent>,
    ) -> Result<(), (JobId, Generation, String)> {
        let candidates = self.inspect_source_snapshot_recovery();
        match candidates {
            Ok(candidates) => {
                events.push_back(ImportEvent::SourceSnapshotRecoveryInspected(candidates))
            }
            Err(error) => events.push_back(ImportEvent::SourceSnapshotRecoveryCompleted {
                freed: 0,
                candidates: Vec::new(),
                error: Some(error),
            }),
        }
        Ok(())
    }
    pub(in crate::worker_ipc) fn confirm_snapshot_recovery_effect(
        &mut self,
        events: &mut VecDeque<ImportEvent>,
    ) -> Result<(), (JobId, Generation, String)> {
        let result = self
            .snapshot_registry_root
            .as_deref()
            .ok_or_else(|| {
                "SourceSnapshotRegistryUnavailable: application settings root is not configured"
                    .to_owned()
            })
            .and_then(|root| {
                crate::source_binding::SourceBinding::recover_abandoned(
                    root,
                    &self.active_snapshot_bindings(),
                )
            });
        match result {
            Ok((freed, candidates)) => {
                events.push_back(ImportEvent::SourceSnapshotRecoveryCompleted {
                    freed,
                    candidates,
                    error: None,
                })
            }
            Err(error) => {
                let candidates = self.inspect_source_snapshot_recovery().unwrap_or_default();
                events.push_back(ImportEvent::SourceSnapshotRecoveryCompleted {
                    freed: 0,
                    candidates,
                    error: Some(error),
                });
            }
        }
        Ok(())
    }
}
