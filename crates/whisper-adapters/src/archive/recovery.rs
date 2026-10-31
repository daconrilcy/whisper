use super::{
    ArchiveEntry, ArchiveQualificationKey, ArchiveStore, CompleteRecord, Digest, Generation,
    ImportRequest, ImportVadWindowPayload, ImportVadWindowRecord, Instant, JobId, Path, PathBuf,
    PendingRecord, PublicationIdentity, PublicationIntent, SegmentRecord, Sha256,
    UnifiedArchiveResumeCheckpoint, WorkerSegment, assert_campaign_archive, fs, generation_path,
    hex, is_registered_model, is_sha256, legacy_transcription_strategy, parse_job_id,
    publication_index_path, publication_reservation_path, read_publication_index, read_segments,
    validate_unified_resume_prefix, verify_artifact, visit_segments_read_only, write_new_synced,
    write_publication_intent,
};

mod live;
use live::read_bounded_jsonl;
pub use live::scan_live_recoveries;
pub(super) use live::{
    live_current_generation, read_verified_live_manifest, reconcile_alias, scan_live,
};

impl ArchiveStore {
    /// Starts the next generation by copying only the previously confirmed prefix.
    pub fn resume_from(
        &mut self,
        previous: &ImportRequest,
        request: ImportRequest,
    ) -> Result<(ImportRequest, Vec<SegmentRecord>), String> {
        self.resume_from_with_catalog_path(previous, request, None)
    }

    /// Validates the checkpoint's durable identity, then prepares with the current catalog path.
    pub fn resume_from_with_catalog_path(
        &mut self,
        previous: &ImportRequest,
        request: ImportRequest,
        executable_model_path: Option<&str>,
    ) -> Result<(ImportRequest, Vec<SegmentRecord>), String> {
        self.resume_from_with_qualification(previous, request, executable_model_path, None)
    }

    pub fn resume_from_with_qualification(
        &mut self,
        previous: &ImportRequest,
        request: ImportRequest,
        executable_model_path: Option<&str>,
        qualification: Option<ArchiveQualificationKey>,
    ) -> Result<(ImportRequest, Vec<SegmentRecord>), String> {
        if request.job_id != previous.job_id
            || previous.generation.next() != Some(request.generation)
        {
            return Err("InvalidInput: resume generation is not the next generation".into());
        }
        let previous_destination = PathBuf::from(&previous.destination);
        let requested_destination = PathBuf::from(&request.destination);
        let same_destination = if previous_destination == requested_destination {
            true
        } else {
            let previous_destination =
                fs::canonicalize(&previous_destination).map_err(|error| {
                    format!("InvalidInput: cannot resolve resume checkpoint destination: {error}")
                })?;
            let requested_destination =
                fs::canonicalize(&requested_destination).map_err(|error| {
                    format!("InvalidInput: cannot resolve requested resume destination: {error}")
                })?;
            previous_destination == requested_destination
        };
        if !same_destination {
            return Err(
                "InvalidInput: resume destination differs from checkpoint destination".into(),
            );
        }
        let previous_dir = PathBuf::from(&previous.destination)
            .join("transcriptions")
            .join(format!("{:032x}", previous.job_id.0))
            .join(previous.generation.get().to_string());
        let pending: PendingRecord = serde_json::from_slice(
            &fs::read(previous_dir.join("pending.json"))
                .map_err(|e| format!("StorageCorrupt: resume checkpoint is unavailable: {e}"))?,
        )
        .map_err(|e| format!("StorageCorrupt: resume checkpoint: {e}"))?;
        validate_pending(&pending, previous.job_id, previous.generation.get())?;
        if pending.strategy != previous.strategy || pending.strategy != request.strategy {
            return Err("StrategyMismatch: resume checkpoint and request strategies differ".into());
        }
        if pending
            .qualification
            .as_ref()
            .is_some_and(ArchiveQualificationKey::is_retired_l2a_key)
        {
            return Err(
                "ModelNotQualified: l'ancienne clé Parakeet L2a est retirée pour toute reprise; le pending est conservé".into(),
            );
        }
        if pending
            .qualification
            .as_ref()
            .is_some_and(ArchiveQualificationKey::is_retired_r5_2_key)
        {
            return Err(
                "ModelNotQualified: l'ancienne clé R5-2 Parakeet est retirée pour toute reprise; le pending est conservé".into(),
            );
        }
        if pending.strategy == whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1
            && pending.model_sha256
                == whisper_core::model_registry::model(
                    whisper_core::model_catalog::ModelId::ParakeetTdt06BV3,
                )
                .artifact
                .sha256
            && pending.source_samples
                > whisper_core::model_registry::PARAKEET_MAX_SINGLE_PASS_DURATION_MS
                    .saturating_mul(16)
        {
            return Err("ModelNotQualified: Fichier trop long pour Parakeet, utilisez Turbo; le pending est conservé".into());
        }
        if pending.source_path != previous.source_path
            || Some(pending.source_sha256.as_str())
                != previous.source_sha256.map(|sha| hex(&sha)).as_deref()
            || Some(pending.source_samples) != previous.source_samples
            || pending.model_path != previous.model_path
            || pending.model_sha256 != hex(&previous.model_sha256)
            || pending.strategy != previous.strategy
            || (if pending.strategy
                == whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
            {
                match (&pending.qualification, &qualification) {
                    (Some(previous), Some(current)) => !previous.permits_bounded_resume_to(current),
                    (None, _) => false,
                    (Some(_), None) => true,
                }
            } else {
                pending.qualification != qualification
            })
        {
            return Err(
                "SourceChanged: resume checkpoint identity differs from the queued request".into(),
            );
        }
        let bounded_resume = pending.strategy
            == whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1;
        let mut resume_checkpoint = None;
        let mut prefix = if bounded_resume {
            let source_samples = pending.source_samples;
            let mut archived_count = 0_u64;
            visit_segments_read_only(&previous_dir.join("segments.jsonl"), |index, segment| {
                let sequence = u64::try_from(index)
                    .ok()
                    .and_then(|value| value.checked_add(1))
                    .ok_or_else(|| "StorageCorrupt: resume sequence overflow".to_owned())?;
                if segment.sequence != sequence
                    || segment.range.sample_rate_hz != 16_000
                    || segment.range.start_sample >= segment.range.end_sample
                    || segment.range.end_sample > source_samples
                    || segment.text.trim().is_empty()
                {
                    return Err("StorageCorrupt: resume segment journal is invalid".into());
                }
                archived_count = sequence;
                Ok(())
            })?;
            let previous_document_directory =
                previous_dir.join("parakeet-unified").join("document");
            let checkpoint =
                crate::parakeet_document::ParakeetDocumentStore::latest_resume_checkpoint(
                    &previous_document_directory,
                    previous.job_id,
                    previous.generation,
                    source_samples,
                    archived_count,
                )?
                .unwrap_or(crate::parakeet_document::ResumeCoverageCheckpoint {
                    window_sequence: 0,
                    covered_samples: 0,
                    confirmed_segments: 0,
                });
            let join_anchor = validate_unified_resume_prefix(
                &previous_dir.join("segments.jsonl"),
                &previous_document_directory,
                previous.job_id,
                previous.generation,
                source_samples,
                checkpoint,
            )?;
            resume_checkpoint = Some(UnifiedArchiveResumeCheckpoint {
                pipeline: crate::parakeet_pipeline::ParakeetResumeCheckpoint {
                    window_sequence: checkpoint.window_sequence,
                    covered_samples: checkpoint.covered_samples,
                    confirmed_segments: checkpoint.confirmed_segments,
                    join_anchor,
                },
                previous_document_directory,
                previous_job_id: previous.job_id,
                previous_generation: previous.generation,
            });
            Vec::new()
        } else {
            read_segments(&previous_dir.join("segments.jsonl"))?
        };
        if !bounded_resume
            && pending
                .qualification
                .as_ref()
                .is_some_and(ArchiveQualificationKey::is_parakeet_vad_test)
        {
            let windows = read_import_vad_windows(&previous_dir, &pending)?;
            let confirmed_segments = windows
                .last()
                .map_or(0, |window| window.last_segment_sequence);
            prefix.truncate(usize::try_from(confirmed_segments).map_err(|_| {
                "StorageCorrupt: VAD confirmed segment prefix exceeds address space".to_owned()
            })?);
        }
        if !bounded_resume
            && prefix.iter().enumerate().any(|(index, segment)| {
                segment.sequence != index as u64 + 1
                    || segment.range.sample_rate_hz != 16_000
                    || segment.range.start_sample >= segment.range.end_sample
                    || segment.range.end_sample > pending.source_samples
                    || segment.text.trim().is_empty()
            })
            || prefix
                .windows(2)
                .any(|pair| pair[1].range.start_sample < pair[0].range.end_sample)
        {
            return Err(
                "StorageCorrupt: confirmed resume prefix is not a valid ordered checkpoint".into(),
            );
        }
        let destination = PathBuf::from(&previous.destination);
        let _publication_lock = Self::lock_publications(&destination)?;
        let previous_reservation =
            publication_reservation_path(&destination, previous.job_id, previous.generation);
        if previous_reservation.is_file() {
            let mut intent: PublicationIntent = serde_json::from_slice(
                &fs::read(&previous_reservation)
                    .map_err(|error| format!("StorageCorrupt: resume reservation: {error}"))?,
            )
            .map_err(|error| format!("StorageCorrupt: resume reservation: {error}"))?;
            if intent.source_sha256 != pending.source_sha256
                || intent.source_samples != pending.source_samples
            {
                return Err("SourceChanged: resume reservation does not match checkpoint".into());
            }
            let current = Self::published_imports_for_strategy(
                &destination,
                &intent.source_sha256,
                pending.strategy,
            )?;
            let current_ids: Vec<_> = current
                .iter()
                .map(|item| PublicationIdentity {
                    job_id: item.job_id,
                    generation: item.generation,
                })
                .collect();
            if current_ids != intent.expected_active {
                return Err("DuplicateStateChanged: publications changed before resume".into());
            }
            intent.operation_id = if request.strategy
                == whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1
            {
                format!("{:032x}-{}", request.job_id.0, request.generation.get())
            } else {
                format!(
                    "{}-{:032x}-{}",
                    request.strategy.stable_id(),
                    request.job_id.0,
                    request.generation.get()
                )
            };
            let next_reservation =
                publication_reservation_path(&destination, request.job_id, request.generation);
            write_publication_intent(&destination, &request, &intent)?;
            if previous_reservation != next_reservation {
                fs::remove_file(&previous_reservation).map_err(|error| {
                    format!("StorageUnavailable: settle previous publication reservation: {error}")
                })?;
            }
        }
        let prepared = self.prepare_inner_with_publication_lock(
            request,
            executable_model_path,
            qualification,
            Some(&destination),
        )?;
        if let Some(checkpoint) = &resume_checkpoint {
            self.copy_unified_resume_prefix(
                &prepared,
                &previous_dir.join("segments.jsonl"),
                checkpoint.pipeline.confirmed_segments,
            )?;
            self.unified_resume_checkpoint = resume_checkpoint;
        } else {
            self.copy_resume_prefix(&prepared, &prefix)?;
        }
        if pending
            .qualification
            .as_ref()
            .is_some_and(ArchiveQualificationKey::is_parakeet_vad_test)
        {
            let records = read_import_vad_windows(&previous_dir, &pending)?;
            if !records.is_empty() {
                let target = PathBuf::from(&prepared.destination)
                    .join("transcriptions")
                    .join(format!("{:032x}", prepared.job_id.0))
                    .join(prepared.generation.get().to_string())
                    .join("vad-windows.jsonl");
                fs::copy(previous_dir.join("vad-windows.jsonl"), target)
                    .map_err(|error| format!("StorageUnavailable: copy VAD checkpoint: {error}"))?;
            }
        }
        Ok((prepared, prefix))
    }

    pub(super) fn copy_resume_prefix(
        &mut self,
        request: &ImportRequest,
        prefix: &[SegmentRecord],
    ) -> Result<(), String> {
        let target_dir = PathBuf::from(&request.destination)
            .join("transcriptions")
            .join(format!("{:032x}", request.job_id.0))
            .join(request.generation.get().to_string());
        let existing = read_segments(&target_dir.join("segments.jsonl"))?;
        if existing.len() > prefix.len()
            || existing
                .iter()
                .zip(prefix)
                .any(|(stored, expected)| stored != expected)
        {
            return Err("StorageCorrupt: resumed generation contains a divergent prefix".into());
        }
        for segment in prefix.iter().skip(existing.len()) {
            self.persist_segment(&WorkerSegment {
                job_id: request.job_id,
                generation: request.generation,
                instance_id: 0,
                segment_id: whisper_core::SegmentId(segment.sequence),
                range: segment.range,
                text: segment.text.clone(),
            })?;
        }
        Ok(())
    }

    fn copy_unified_resume_prefix(
        &mut self,
        request: &ImportRequest,
        source_journal: &Path,
        confirmed_segments: u64,
    ) -> Result<(), String> {
        let mut copied = 0_u64;
        visit_segments_read_only(source_journal, |index, record| {
            let sequence = u64::try_from(index)
                .ok()
                .and_then(|value| value.checked_add(1))
                .ok_or_else(|| "StorageCorrupt: resume sequence overflow".to_owned())?;
            if sequence > confirmed_segments {
                return Ok(());
            }
            if record.sequence != sequence {
                return Err("StorageCorrupt: resumed prefix sequence changed".into());
            }
            self.persist_segment(&WorkerSegment {
                job_id: request.job_id,
                generation: request.generation,
                instance_id: 0,
                segment_id: whisper_core::SegmentId(sequence),
                range: record.range,
                text: record.text,
            })?;
            copied = sequence;
            Ok(())
        })?;
        if copied != confirmed_segments {
            return Err("StorageCorrupt: confirmed resume prefix is incomplete".into());
        }
        Ok(())
    }

    pub fn scan(root: &Path) -> Result<Vec<ArchiveEntry>, String> {
        let started = Instant::now();
        let result = Self::scan_publications(root);
        crate::diagnostics::record_stage_timing(
            "archive-publication-scan",
            started.elapsed(),
            serde_json::json!({
                "scan_operation": "ArchiveStore::scan",
                "destination": root.to_string_lossy(),
                "phase": "publication-scan",
                "outcome": if result.is_ok() { "ok" } else { "error" },
            }),
        );
        result
    }

    fn scan_publications(root: &Path) -> Result<Vec<ArchiveEntry>, String> {
        let mut entries = Self::scan_raw(root)?;
        let index_path = publication_index_path(root);
        let index = if index_path.exists() {
            Some(read_publication_index(root)?)
        } else {
            None
        };
        entries.retain_mut(|entry| {
            if !entry.complete {
                return true;
            }
            let directory = generation_path(root, entry.job_id, entry.generation);
            let record = index.as_ref().and_then(|value| {
                value.records.iter().find(|record| {
                    record.job_id == entry.job_id && record.generation == entry.generation
                })
            });
            if directory.join("r11-publication.json").is_file() {
                if let Some(record) = record {
                    if !record.active {
                        return false;
                    }
                    entry.export_basename = record.export_basename.clone();
                    return true;
                }
                entry.complete = false;
                entry.publication_recovery = true;
                entry.diagnostic = Some(
                    "Recoverable: publication R11 complète, en attente d'activation explicite"
                        .into(),
                );
                return true;
            }
            if let Some(record) = record {
                if !record.active {
                    return false;
                }
                entry.export_basename = record.export_basename.clone();
            }
            true
        });
        Ok(entries)
    }

    pub(super) fn scan_raw(root: &Path) -> Result<Vec<ArchiveEntry>, String> {
        assert_campaign_archive(root)?;
        let mut entries = Vec::new();
        let base = root.join("transcriptions");
        if !base.exists() {
            return Ok(entries);
        }
        for job in fs::read_dir(base).map_err(|e| e.to_string())? {
            let job = match job {
                Ok(job) => job,
                Err(error) => {
                    entries.push(corrupt_entry(JobId(0), "job directory", error.to_string()));
                    continue;
                }
            };
            // The durable FIFO journal shares this parent with per-job directories.
            // It is not an archive and must not be reported as a corrupt JobId(0).
            if job.file_name() == "queue.jsonl" {
                continue;
            }
            let generations = match fs::read_dir(job.path()) {
                Ok(generations) => generations,
                Err(error) => {
                    let job_id =
                        parse_job_id(&job.file_name().to_string_lossy()).unwrap_or(JobId(0));
                    entries.push(corrupt_entry(job_id, "job directory", error.to_string()));
                    continue;
                }
            };
            for generation in generations {
                let generation = match generation {
                    Ok(generation) => generation,
                    Err(error) => {
                        let job_id =
                            parse_job_id(&job.file_name().to_string_lossy()).unwrap_or(JobId(0));
                        entries.push(corrupt_entry(
                            job_id,
                            "generation directory",
                            error.to_string(),
                        ));
                        continue;
                    }
                };
                let dir = generation.path();
                let job_id = parse_job_id(&job.file_name().to_string_lossy()).unwrap_or(JobId(0));
                if generation.file_name() == "live" {
                    match scan_live(&dir, job_id) {
                        Ok(live_entries) => entries.extend(live_entries),
                        Err(error) => entries.push(corrupt_entry(job_id, "live", error)),
                    }
                    continue;
                }
                match scan_generation(
                    &dir,
                    &job.file_name().to_string_lossy(),
                    &generation.file_name().to_string_lossy(),
                ) {
                    Ok(Some(entry)) => entries.push(entry),
                    Ok(None) => continue,
                    Err(error) => entries.push(corrupt_entry(
                        job_id,
                        &generation.file_name().to_string_lossy(),
                        error,
                    )),
                }
            }
        }
        entries.sort_by_key(|entry| (entry.job_id.0, entry.generation.get()));
        Ok(entries)
    }
}

pub(super) fn scan_generation(
    dir: &Path,
    job_folder: &str,
    generation_folder: &str,
) -> Result<Option<ArchiveEntry>, String> {
    let expected_job = parse_job_id(job_folder)
        .filter(|id| format!("{:032x}", id.0) == job_folder)
        .ok_or_else(|| "job folder is not a canonical 32-digit identity".to_owned())?;
    let expected_generation = generation_folder
        .parse::<u64>()
        .ok()
        .filter(|number| number.to_string() == generation_folder && *number > 0)
        .ok_or_else(|| "generation folder is not a canonical positive integer".to_owned())?;
    if dir.join("CURRENT").is_file() {
        let pointer = fs::read(dir.join("CURRENT")).map_err(|e| e.to_string())?;
        if pointer.as_slice() != b"manifest.json\n" {
            return Err("CURRENT must reference exactly manifest.json".into());
        }
        let manifest: CompleteRecord = serde_json::from_slice(
            &fs::read(dir.join("manifest.json")).map_err(|e| e.to_string())?,
        )
        .map_err(|e| format!("StorageCorrupt: manifest: {e}"))?;
        let pending: PendingRecord =
            serde_json::from_slice(&fs::read(dir.join("pending.json")).map_err(|e| e.to_string())?)
                .map_err(|e| format!("StorageCorrupt: pending: {e}"))?;
        validate_pending(&pending, expected_job, expected_generation)?;
        let _ = read_import_vad_windows(dir, &pending)?;
        if manifest.version != 1
            || manifest.job_id != expected_job
            || manifest.job_id != pending.job_id
            || manifest.generation.get() != expected_generation
            || manifest.generation != pending.generation
            || manifest.source_sha256 != pending.source_sha256
            || !is_sha256(&manifest.source_sha256)
            || manifest.text.path != "transcript.txt"
            || manifest.srt.path != "transcript.srt"
            || !is_sha256(&manifest.text.sha256)
            || !is_sha256(&manifest.srt.sha256)
        {
            return Err(
                "StorageCorrupt: manifest identity/schema does not match pending generation".into(),
            );
        }
        verify_artifact(dir, &manifest.text)?;
        verify_artifact(dir, &manifest.srt)?;
        let records = read_segments(&dir.join("segments.jsonl"))?;
        if records.len() as u64 != manifest.segments
            || records.iter().enumerate().any(|(index, record)| {
                record.sequence != index as u64 + 1
                    || record.range.sample_rate_hz != 16_000
                    || record.range.start_sample >= record.range.end_sample
                    || record.range.end_sample > pending.source_samples
            })
        {
            return Err(
                "StorageCorrupt: manifest segment count/ranges do not match the journal".into(),
            );
        }
        let transcript = fs::read(dir.join("transcript.txt")).map_err(|e| e.to_string())?;
        let expected_text = records
            .iter()
            .map(|record| record.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        if transcript != expected_text.as_bytes() {
            return Err(
                "StorageCorrupt: transcript does not match the verified segment journal".into(),
            );
        }
        return Ok(Some(ArchiveEntry {
            job_id: expected_job,
            generation: pending.generation,
            complete: true,
            source_sha256: pending.source_sha256,
            diagnostic: None,
            export_basename: None,
            publication_recovery: false,
            strategy: pending.strategy,
        }));
    }
    if dir.join("pending.json").is_file() {
        let pending: PendingRecord =
            serde_json::from_slice(&fs::read(dir.join("pending.json")).map_err(|e| e.to_string())?)
                .map_err(|e| format!("StorageCorrupt: pending: {e}"))?;
        validate_pending(&pending, expected_job, expected_generation)?;
        let _ = read_import_vad_windows(dir, &pending)?;
        return Ok(Some(ArchiveEntry {
            job_id: pending.job_id,
            generation: pending.generation,
            complete: false,
            source_sha256: pending.source_sha256,
            diagnostic: None,
            export_basename: None,
            publication_recovery: false,
            strategy: pending.strategy,
        }));
    }
    if dir.join("manifest.json").exists() || dir.join("segments.jsonl").exists() {
        return Err("StorageCorrupt: artifacts exist without a pending or CURRENT marker".into());
    }
    Ok(None)
}

pub(super) fn validate_pending(
    pending: &PendingRecord,
    job_id: JobId,
    generation: u64,
) -> Result<(), String> {
    let parakeet_sha =
        whisper_core::model_registry::model(whisper_core::model_catalog::ModelId::ParakeetTdt06BV3)
            .artifact
            .sha256;
    let unified = pending.version == 4
        && pending.strategy
            == whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
        && pending.qualification.is_none()
        && pending.model_sha256 == parakeet_sha;
    let qualification_valid = if unified {
        true
    } else if pending.model_sha256 == parakeet_sha {
        (pending.version == 2
            && pending.qualification.as_ref().is_some_and(|key| {
                key == &ArchiveQualificationKey::parakeet_cpu_test()
                    || key == &ArchiveQualificationKey::parakeet_gpu_test()
                    || key.is_registered_personal_import()
                    || key.is_retired_l2a_key()
                    || key.is_retired_r5_2_key()
                    || key.is_personal_candidate()
            }))
            || (pending.version == 3
                && pending
                    .qualification
                    .as_ref()
                    .is_some_and(ArchiveQualificationKey::is_parakeet_vad_test))
    } else {
        pending.version == 1 && pending.qualification.is_none()
    };
    if (!matches!(pending.version, 1..=3) && !unified)
        || (pending.version < 4 && pending.strategy != legacy_transcription_strategy())
        || pending.job_id != job_id
        || pending.generation.get() != generation
        || !is_sha256(&pending.source_sha256)
        || pending.source_samples == 0
        || !is_registered_model(&pending.model_sha256, pending.model_size)
        || !qualification_valid
    {
        return Err("StorageCorrupt: pending identity/schema is invalid".into());
    }
    Ok(())
}

pub(super) fn read_import_vad_windows(
    directory: &Path,
    pending: &PendingRecord,
) -> Result<Vec<ImportVadWindowPayload>, String> {
    let Some(qualification) = pending
        .qualification
        .as_ref()
        .filter(|key| key.is_parakeet_vad_test())
    else {
        if directory.join("vad-windows.jsonl").exists() {
            return Err("StorageCorrupt: VAD checkpoint exists without an import-vad key".into());
        }
        return Ok(Vec::new());
    };
    let path = directory.join("vad-windows.jsonl");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let entries = read_bounded_jsonl::<ImportVadWindowRecord>(&path)?;
    let segments = read_segments(&directory.join("segments.jsonl"))?;
    let mut output = Vec::with_capacity(entries.len());
    let mut previous_end = 0_u64;
    let mut previous_segment_sequence = 0_u64;
    for (index, entry) in entries.into_iter().enumerate() {
        let canonical = serde_json::to_vec(&entry.payload).map_err(|error| error.to_string())?;
        let expected_hash = format!("{:x}", Sha256::digest(canonical));
        let payload = entry.payload;
        if entry.sha256 != expected_hash
            || payload.version != 1
            || payload.job_id != pending.job_id
            || payload.generation != pending.generation
            || payload.sequence != index as u64 + 1
            || payload.central_start_sample != previous_end
            || payload.central_start_sample >= payload.central_end_sample
            || payload.central_end_sample > pending.source_samples
            || payload.native_start_sample > payload.central_start_sample
            || payload.native_end_sample < payload.central_end_sample
            || payload.native_end_sample - payload.native_start_sample
                > whisper_core::parakeet_windows::MAX_NATIVE_SAMPLES
            || payload.last_segment_sequence < previous_segment_sequence
            || payload.last_segment_sequence > segments.len() as u64
            || payload.qualification != *qualification
        {
            return Err("StorageCorrupt: import-vad checkpoint identity/range is invalid".into());
        }
        let owned_segments = segments
            .get(previous_segment_sequence as usize..payload.last_segment_sequence as usize)
            .ok_or_else(|| {
                "StorageCorrupt: VAD window references missing transcript segments".to_owned()
            })?;
        if owned_segments.iter().any(|segment| {
            let center = segment.range.start_sample
                + (segment.range.end_sample - segment.range.start_sample) / 2;
            center < payload.central_start_sample || center >= payload.central_end_sample
        }) {
            return Err("StorageCorrupt: transcript segment belongs to another VAD window".into());
        }
        previous_end = payload.central_end_sample;
        previous_segment_sequence = payload.last_segment_sequence;
        output.push(payload);
    }
    Ok(output)
}

pub(super) fn initialize_pending(directory: &Path, pending: &PendingRecord) -> Result<u64, String> {
    let pending_path = directory.join("pending.json");
    if pending_path.exists() {
        if directory.join("CURRENT").exists() {
            return Err("InvalidInput: generation is already published".into());
        }
        let existing: PendingRecord = serde_json::from_slice(
            &fs::read(&pending_path).map_err(|e| format!("StorageCorrupt: pending record: {e}"))?,
        )
        .map_err(|e| format!("StorageCorrupt: pending record: {e}"))?;
        if existing != *pending {
            return Err("SourceChanged: existing pending generation has another identity".into());
        }
        Ok(read_segments(&directory.join("segments.jsonl"))?.len() as u64 + 1)
    } else {
        write_new_synced(
            &pending_path,
            &serde_json::to_vec(pending).map_err(|e| e.to_string())?,
        )?;
        Ok(1)
    }
}

fn corrupt_entry(job_id: JobId, location: &str, error: String) -> ArchiveEntry {
    ArchiveEntry {
        job_id,
        generation: Generation::first(),
        complete: false,
        source_sha256: String::new(),
        diagnostic: Some(format!("StorageCorrupt: {location}: {error}")),
        export_basename: None,
        publication_recovery: false,
        strategy: legacy_transcription_strategy(),
    }
}
