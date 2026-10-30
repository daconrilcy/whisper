use super::{
    ArchiveStore, CompleteRecord, Generation, Instant, JobId, Write, campaign_gate,
    copy_replace_streamed, fs, hex, load_or_migrate_publication_index,
    record_campaign_operation_result, sha256_file, timestamp_checked, validate_segment_journal,
    verified_artifact, verify_artifact, visit_segments, write_replace_streamed,
    write_replace_synced,
};
#[cfg(windows)]
use super::{
    CleanupPlan, PendingRecord, TreatmentIdentity, capture_cleanup_file, capture_successor_files,
    cleanup_plan_path, identity_snapshot, read_cleanup_plan, run_cleanup_plan_locked,
    write_cleanup_plan,
};

mod catalog;
mod exports;
#[cfg(test)]
pub(super) use catalog::write_publication_reservation;
pub(super) use catalog::{
    apply_publication_intent, load_publication_intent, publication_index_path,
    publication_reservation_path, read_publication_index, reconcile_replaced_exports,
    reserve_export_basename, write_publication_index, write_publication_intent,
};
#[cfg(test)]
pub(super) use exports::install_publication_exports_with_copy_hook;
pub(super) use exports::{
    export_install_temporary_path, install_publication_exports, install_publication_exports_with,
    persisted_file_identity,
};

impl ArchiveStore {
    pub fn prepare_publish(
        &mut self,
        job_id: JobId,
        generation: Generation,
        expected_count: u64,
    ) -> Result<(), String> {
        let started = Instant::now();
        let active = self
            .current
            .as_ref()
            .ok_or_else(|| "StorageUnavailable: no pending archive".to_owned())?;
        if active.job_id != job_id
            || active.generation != generation
            || active.expected_sequence != expected_count.saturating_add(1)
        {
            return Err(
                "StorageCorrupt: End does not match the durably persisted segment sequence".into(),
            );
        }
        if let Some((path, expected_sha)) = &active.segmented_source_manifest
            && (!path.is_file() || sha256_file(path)? != *expected_sha)
        {
            return Err(
                "StorageCorrupt: captured PCM part manifest changed before publication".into(),
            );
        }
        let journal_path = active.directory.join("segments.jsonl");
        validate_segment_journal(&journal_path, expected_count)?;
        let text_path = active.directory.join("transcript.txt");
        let srt_path = active.directory.join("transcript.srt");
        write_replace_streamed(&text_path, |writer| {
            visit_segments(&journal_path, |index, record| {
                if index > 0 {
                    writer.write_all(b"\n").map_err(|error| error.to_string())?;
                }
                writer
                    .write_all(record.text.as_bytes())
                    .map_err(|error| error.to_string())
            })
        })?;
        write_replace_streamed(&srt_path, |writer| {
            visit_segments(&journal_path, |index, record| {
                let start = timestamp_checked(record.range.start_sample)?;
                let end = timestamp_checked(record.range.end_sample)?;
                writeln!(
                    writer,
                    "{}\n{} --> {}\n{}",
                    index + 1,
                    start,
                    end,
                    record.text
                )
                .map_err(|error| error.to_string())
            })
        })?;
        let text_artifact = verified_artifact(&active.directory, &text_path)?;
        let srt_artifact = verified_artifact(&active.directory, &srt_path)?;
        let manifest = CompleteRecord {
            version: 1,
            job_id,
            generation,
            source_sha256: hex(&active.source_sha256),
            segments: expected_count,
            text: text_artifact,
            srt: srt_artifact,
        };
        let manifest_bytes = serde_json::to_vec(&manifest).map_err(|e| e.to_string())?;
        let manifest_path = active.directory.join("manifest.json");
        campaign_gate("import.before_manifest", job_id.0, generation.get(), 0)?;
        let operation = write_replace_synced(&manifest_path, &manifest_bytes);
        record_campaign_operation_result(
            "import.before_manifest",
            job_id.0,
            generation.get(),
            0,
            &operation,
            Some(&manifest_path),
        )?;
        operation?;
        campaign_gate("import.after_manifest", job_id.0, generation.get(), 0)?;
        let verified: CompleteRecord =
            serde_json::from_slice(&fs::read(&manifest_path).map_err(|e| e.to_string())?)
                .map_err(|e| format!("StorageCorrupt: manifest: {e}"))?;
        verify_artifact(&active.directory, &verified.text)?;
        verify_artifact(&active.directory, &verified.srt)?;
        let marker_path = active.directory.join("r11-publication.json");
        write_replace_synced(
            &marker_path,
            &serde_json::to_vec(&active.publication).map_err(|error| error.to_string())?,
        )?;
        if !active.publication.internal_only {
            for (extension, artifact) in [("txt", &verified.text), ("srt", &verified.srt)] {
                let staged = active.directory.join(format!("export.{extension}.pending"));
                let source = active.directory.join(&artifact.path);
                copy_replace_streamed(&source, &staged)?;
                if hex(&sha256_file(&staged)?) != artifact.sha256 {
                    return Err(format!(
                        "StorageCorrupt: staged export {extension} does not match the manifest"
                    ));
                }
            }
        }
        crate::diagnostics::record_stage_timing(
            "archive_prepare_publish",
            started.elapsed(),
            serde_json::json!({ "segments": expected_count, "text_bytes": fs::metadata(&text_path).map(|meta| meta.len()).unwrap_or(0), "srt_bytes": fs::metadata(&srt_path).map(|meta| meta.len()).unwrap_or(0) }),
        );
        Ok(())
    }

    pub fn commit_publish(&mut self, job_id: JobId, generation: Generation) -> Result<(), String> {
        let started = Instant::now();
        let active = self
            .current
            .as_ref()
            .ok_or_else(|| "StorageUnavailable: no pending archive".to_owned())?;
        if active.job_id != job_id || active.generation != generation {
            return Err("StaleResponse: publish commit targets another generation".into());
        }
        let root = self.root.clone();
        let _lock = Self::lock_publications(&root)?;
        if !active.publication.internal_only {
            let current = Self::published_imports_for_strategy(
                &root,
                &active.publication.source_sha256,
                active.strategy,
            )?;
            let expected: Vec<_> = active
                .publication
                .expected_active
                .iter()
                .map(|item| (item.job_id, item.generation))
                .collect();
            let actual: Vec<_> = current
                .iter()
                .map(|item| (item.job_id, item.generation))
                .collect();
            if expected != actual {
                return Err(
                    "DuplicateStateChanged: publications changed during processing; confirm again"
                        .into(),
                );
            }
        }
        let publication = active.publication.clone();
        if let Some(cleanup_id) = publication.cleanup_id.as_deref() {
            #[cfg(windows)]
            {
                let pending: PendingRecord = serde_json::from_slice(
                    &fs::read(active.directory.join("pending.json"))
                        .map_err(|error| format!("StorageCorrupt: replacement pending: {error}"))?,
                )
                .map_err(|error| format!("StorageCorrupt: replacement pending: {error}"))?;
                let successor = TreatmentIdentity {
                    destination: root
                        .canonicalize()
                        .map_err(|error| {
                            format!("StorageUnavailable: canonicaliser destination: {error}")
                        })?
                        .to_string_lossy()
                        .into_owned(),
                    job_id,
                    generation,
                };
                let successor_directory = identity_snapshot(
                    crate::windows_file_identity::inspect(&active.directory, true)?,
                )?;
                let successor_files = capture_successor_files(&root, &active.directory)?;
                let mut plan = CleanupPlan {
                    version: 1,
                    cleanup_id: cleanup_id.to_owned(),
                    destination: root
                        .canonicalize()
                        .map_err(|error| {
                            format!("StorageUnavailable: canonicaliser destination: {error}")
                        })?
                        .to_string_lossy()
                        .into_owned(),
                    source_path: pending.source_path,
                    source_sha256: publication.source_sha256.clone(),
                    successor,
                    successor_directory,
                    successor_files,
                    successor_index: None,
                    published: false,
                    targets: publication.cleanup_targets.clone(),
                    last_error: None,
                };
                let path = cleanup_plan_path(&root, cleanup_id);
                if path.exists() {
                    let existing = read_cleanup_plan(&root, cleanup_id)?;
                    if existing != plan {
                        return Err(
                            "DuplicateStateChanged: nettoyage durable déjà différent".into()
                        );
                    }
                    plan = existing;
                } else {
                    write_cleanup_plan(&root, &plan)?;
                }
            }
            #[cfg(not(windows))]
            return Err("Unsupported: le remplacement RTC-02 nécessite les handles Windows".into());
        }
        if !publication.internal_only {
            install_publication_exports_with(
                &root,
                &active.directory,
                &publication,
                self.export_hard_linker.as_ref(),
            )?;
        }
        campaign_gate("import.before_current", job_id.0, generation.get(), 0)?;
        let current_path = active.directory.join("CURRENT");
        let operation = write_replace_synced(&current_path, b"manifest.json\n");
        record_campaign_operation_result(
            "import.before_current",
            job_id.0,
            generation.get(),
            0,
            &operation,
            Some(&current_path),
        )?;
        operation?;
        campaign_gate("import.after_current", job_id.0, generation.get(), 0)?;
        let index = apply_publication_intent(
            load_or_migrate_publication_index(&root)?,
            job_id,
            generation,
            &publication,
        );
        write_publication_index(&root, &index)?;
        if let Some(cleanup_id) = publication.cleanup_id.as_deref() {
            #[cfg(windows)]
            {
                let mut plan = read_cleanup_plan(&root, cleanup_id)?;
                plan.successor_files = capture_successor_files(&root, &active.directory)?;
                plan.successor_files.push(capture_cleanup_file(
                    &root,
                    &active.directory.join("CURRENT"),
                )?);
                plan.successor_index =
                    Some(capture_cleanup_file(&root, &publication_index_path(&root))?);
                plan.published = true;
                write_cleanup_plan(&root, &plan)?;
                if let Err(error) = run_cleanup_plan_locked(&root, &mut plan) {
                    plan.last_error = Some(error);
                    write_cleanup_plan(&root, &plan)?;
                }
            }
        }
        if !publication.internal_only {
            reconcile_replaced_exports(&root, &index)?;
        }
        Self::clear_duplicate_reservation(&root, job_id, generation)?;
        self.current = None;
        crate::diagnostics::record_stage_timing(
            "archive_commit_publish",
            started.elapsed(),
            serde_json::json!({ "job_id": format!("{:032x}", job_id.0), "generation": generation.get() }),
        );
        Ok(())
    }

    pub fn publish(
        &mut self,
        job_id: JobId,
        generation: Generation,
        expected_count: u64,
    ) -> Result<(), String> {
        self.prepare_publish(job_id, generation, expected_count)?;
        self.commit_publish(job_id, generation)
    }
}
