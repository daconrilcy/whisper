use super::super::{
    ArchiveEntry, ArtifactRecord, BufRead, BufReader, CoverageRecord, Deserialize, Digest, File,
    Generation, JobId, LiveCompleteRecord, LiveRequest, Path, PathBuf, SegmentRecord, Sha256,
    campaign_gate, fs, is_sha256, legacy_transcription_strategy, parse_job_id,
    record_campaign_operation_result, safe_artifact_path, verify_artifact, verify_pcm_staging,
    write_replace_synced,
};

pub(in crate::archive) fn scan_live(
    dir: &Path,
    expected_job: JobId,
) -> Result<Vec<ArchiveEntry>, String> {
    campaign_gate("recovery.before_scan_live", expected_job.0, 0, 0)?;
    if format!("{:032x}", expected_job.0)
        != dir
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .unwrap_or("")
    {
        return Err("live job folder is not canonical".into());
    }
    let mut entries = Vec::new();
    let mut verified = std::collections::BTreeSet::new();
    let committed_generation = live_current_generation(dir)?;
    for item in fs::read_dir(dir).map_err(|error| error.to_string())? {
        let item = item.map_err(|error| error.to_string())?;
        let name = item.file_name().to_string_lossy().into_owned();
        if let Some(number) = name
            .strip_prefix("manifest-")
            .and_then(|s| s.strip_suffix(".json"))
        {
            let generation_number = number
                .parse::<u64>()
                .ok()
                .filter(|value| value.to_string() == number && *value > 0)
                .ok_or_else(|| format!("invalid live manifest name: {name}"))?;
            let generation: Generation =
                serde_json::from_value(serde_json::Value::from(generation_number))
                    .map_err(|error| format!("invalid live generation: {error}"))?;
            read_verified_live_manifest(dir, expected_job, generation)?;
            verified.insert(generation_number);
            let complete =
                committed_generation.is_some_and(|committed| generation_number <= committed);
            entries.push(ArchiveEntry {
                job_id: expected_job,
                generation,
                complete,
                source_sha256: String::new(),
                diagnostic: (!complete).then(|| {
                    "Recoverable: passage manifest exists but CURRENT did not commit it".into()
                }),
                export_basename: None,
                publication_recovery: false,
                strategy: legacy_transcription_strategy(),
            });
        }
    }
    if let Some(number) = committed_generation {
        if !verified.contains(&number) {
            return Err("StorageCorrupt: live CURRENT target is not a verified passage".into());
        }
        let manifest_path = dir.join(format!("manifest-{number}.json"));
        let record: LiveCompleteRecord =
            serde_json::from_slice(&fs::read(manifest_path).map_err(|error| error.to_string())?)
                .map_err(|error| format!("StorageCorrupt: committed live manifest: {error}"))?;
        reconcile_alias(dir, &record.text, "transcript.txt")?;
        reconcile_alias(dir, &record.srt, "transcript.srt")?;
    }
    for item in fs::read_dir(dir).map_err(|error| error.to_string())? {
        let item = item.map_err(|error| error.to_string())?;
        let name = item.file_name().to_string_lossy().into_owned();
        if let Some(number) = name
            .strip_prefix("passage-")
            .and_then(|s| s.strip_suffix(".pending.json"))
        {
            let generation_number = number
                .parse::<u64>()
                .ok()
                .filter(|value| value.to_string() == number && *value > 0)
                .ok_or_else(|| format!("invalid pending live passage name: {name}"))?;
            let generation: Generation =
                serde_json::from_value(serde_json::Value::from(generation_number))
                    .map_err(|error| format!("invalid pending generation: {error}"))?;
            let request: LiveRequest =
                serde_json::from_slice(&fs::read(item.path()).map_err(|e| e.to_string())?)
                    .map_err(|error| format!("StorageCorrupt: live pending record: {error}"))?;
            if request.job_id != expected_job || request.generation != generation {
                return Err(format!(
                    "StorageCorrupt: live pending identity mismatch: {name}"
                ));
            }
            if !entries.iter().any(|entry| entry.generation == generation) {
                entries.push(ArchiveEntry { job_id: expected_job, generation, complete: false,
                    source_sha256: String::new(), diagnostic: Some("Recoverable: live passage was not atomically published; explicit resume or discard required".into()), export_basename: None, publication_recovery: false, strategy: legacy_transcription_strategy() });
            }
        }
    }
    entries.sort_by_key(|entry| entry.generation.get());
    Ok(entries)
}

pub(in crate::archive) fn reconcile_alias(
    dir: &Path,
    source: &ArtifactRecord,
    alias: &str,
) -> Result<(), String> {
    let contents = fs::read(dir.join(&source.path)).map_err(|error| error.to_string())?;
    let alias_path = dir.join(alias);
    if fs::read(&alias_path).ok().as_deref() != Some(contents.as_slice()) {
        write_replace_synced(&alias_path, &contents)?;
    }
    Ok(())
}

pub(in crate::archive) fn live_current_generation(dir: &Path) -> Result<Option<u64>, String> {
    let current_path = dir.join("CURRENT");
    if !current_path.exists() {
        return Ok(None);
    }
    let pointer = fs::read_to_string(&current_path).map_err(|e| e.to_string())?;
    let target = pointer.strip_suffix('\n').ok_or_else(|| {
        "StorageCorrupt: live CURRENT pointer is not newline terminated".to_owned()
    })?;
    if pointer != format!("{target}\n")
        || !target.starts_with("manifest-")
        || !target.ends_with(".json")
        || !safe_artifact_path(target)
    {
        return Err("StorageCorrupt: live CURRENT pointer is invalid".into());
    }
    target
        .strip_prefix("manifest-")
        .and_then(|s| s.strip_suffix(".json"))
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|number| {
            *number > 0
                && number.to_string()
                    == target
                        .trim_start_matches("manifest-")
                        .trim_end_matches(".json")
        })
        .map(Some)
        .ok_or_else(|| "StorageCorrupt: live CURRENT target is invalid".into())
}

pub(in crate::archive) fn read_verified_live_manifest(
    dir: &Path,
    expected_job: JobId,
    generation: Generation,
) -> Result<LiveCompleteRecord, String> {
    let name = format!("manifest-{}.json", generation.get());
    let record: LiveCompleteRecord = serde_json::from_slice(
        &fs::read(dir.join(&name))
            .map_err(|error| format!("StorageCorrupt: live manifest: {error}"))?,
    )
    .map_err(|error| format!("StorageCorrupt: live manifest: {error}"))?;
    if record.version != 1
        || record.job_id != expected_job
        || record.generation != generation
        || record.audio_samples == 0
        || !is_sha256(&record.pcm_sha256)
        || !safe_artifact_path(&record.staging.path)
        || !safe_artifact_path(&record.mp3.path)
        || !safe_artifact_path(&record.text.path)
        || !safe_artifact_path(&record.srt.path)
    {
        return Err(format!(
            "StorageCorrupt: live manifest identity or shape mismatch: {name}"
        ));
    }
    verify_artifact(dir, &record.staging)?;
    verify_pcm_staging(
        dir,
        &record.staging,
        record.audio_samples,
        &record.pcm_sha256,
    )?;
    verify_artifact(dir, &record.mp3)?;
    verify_artifact(dir, &record.text)?;
    verify_artifact(dir, &record.srt)?;
    Ok(record)
}

pub fn scan_live_recoveries(
    root: &Path,
) -> Result<Vec<whisper_core::ports::LiveRecoveryInfo>, String> {
    let base = root.join("transcriptions");
    if !base.exists() {
        return Ok(Vec::new());
    }
    let mut recoveries = Vec::new();
    for job_entry in fs::read_dir(base).map_err(|e| e.to_string())? {
        let job_entry = job_entry.map_err(|e| e.to_string())?;
        let job_name = job_entry.file_name().to_string_lossy().into_owned();
        let Some(job_id) =
            parse_job_id(&job_name).filter(|id| format!("{:032x}", id.0) == job_name)
        else {
            continue;
        };
        let dir = job_entry.path().join("live");
        if !dir.is_dir() {
            continue;
        }
        let current_generation = live_current_generation(&dir)?;
        let mut latest_generation = 0u64;
        let mut latest_pending: Option<(u64, PathBuf)> = None;
        let mut manifest_generations = std::collections::BTreeSet::new();
        for item in fs::read_dir(&dir).map_err(|e| e.to_string())? {
            let item = item.map_err(|e| e.to_string())?;
            let name = item.file_name().to_string_lossy().into_owned();
            if let Some(number) = name
                .strip_prefix("manifest-")
                .and_then(|s| s.strip_suffix(".json"))
                && let Ok(generation) = number.parse::<u64>()
                && generation > 0
                && generation.to_string() == number
            {
                latest_generation = latest_generation.max(generation);
                manifest_generations.insert(generation);
            }
            if let Some(number) = name
                .strip_prefix("passage-")
                .and_then(|s| s.strip_suffix(".pending.json"))
                && let Ok(generation) = number.parse::<u64>()
                && generation > 0
                && generation.to_string() == number
            {
                if generation >= latest_pending.as_ref().map_or(0, |(latest, _)| *latest) {
                    latest_pending = Some((generation, item.path()));
                }
                latest_generation = latest_generation.max(generation);
            }
        }
        let Some((generation, pending_path)) = latest_pending else {
            continue;
        };
        let is_next_generation = match current_generation {
            Some(current) => current.checked_add(1) == Some(generation),
            None => generation == 1,
        };
        if generation != latest_generation || !is_next_generation {
            continue;
        }
        let request: LiveRequest =
            serde_json::from_slice(&fs::read(pending_path).map_err(|e| e.to_string())?)
                .map_err(|e| format!("StorageCorrupt: live recovery request: {e}"))?;
        if request.job_id != job_id || request.generation.get() != generation {
            continue;
        }
        let pcm_path = dir.join(format!("passage-{generation}.pcm"));
        let Ok(checkpoint) = crate::staging::PcmStaging::inspect_checkpoint(&pcm_path) else {
            continue;
        };
        let publication_ready = manifest_generations.contains(&generation);
        let manifest = if publication_ready {
            Some(read_verified_live_manifest(
                &dir,
                job_id,
                request.generation,
            )?)
        } else {
            None
        };
        campaign_gate("recovery.before_coverage", job_id.0, generation, 0)?;
        let scan_result =
            scan_confirmed_live_prefix(&dir, generation, &request, checkpoint.durable_samples);
        record_campaign_operation_result(
            "recovery.before_coverage",
            job_id.0,
            generation,
            0,
            &scan_result,
            None,
        )?;
        let confirmed_samples = scan_result?;
        if let Some(manifest) = &manifest
            && (manifest.audio_samples != checkpoint.durable_samples
                || manifest.confirmed_samples != confirmed_samples
                || manifest.confirmed_samples != manifest.audio_samples)
        {
            continue;
        }
        campaign_gate("recovery.after_coverage", job_id.0, generation, 0)?;
        recoveries.push(whisper_core::ports::LiveRecoveryInfo {
            request,
            durable_samples: checkpoint.durable_samples,
            confirmed_samples,
            publication_ready,
        });
    }
    Ok(recoveries)
}

fn scan_confirmed_live_prefix(
    dir: &Path,
    generation: u64,
    request: &LiveRequest,
    durable_samples: u64,
) -> Result<u64, String> {
    let coverage_path = dir.join(format!("passage-{generation}.coverage.jsonl"));
    if !coverage_path.exists() {
        return Ok(0);
    }
    let segment_path = dir.join(format!("passage-{generation}.segments.jsonl"));
    let segments = read_bounded_jsonl::<SegmentRecord>(&segment_path)?;
    let coverage = read_bounded_jsonl::<CoverageRecord>(&coverage_path)?;
    let mut confirmed_samples = 0_u64;
    let mut expected_window_sequence = 1_u64;
    let mut confirmed_segment_sequence = 0_u64;
    for record in coverage {
        let payload = record.payload;
        let canonical = serde_json::to_vec(&payload).map_err(|error| error.to_string())?;
        let checksum = format!("{:x}", Sha256::digest(canonical));
        if record.sha256 != checksum
            || payload.version != 2
            || payload.job_id != request.job_id
            || payload.generation != request.generation
            || payload.window_sequence != expected_window_sequence
            || payload.start_sample != confirmed_samples
            || payload.end_sample <= payload.start_sample
            || payload.end_sample > durable_samples
            || payload.last_segment_sequence < confirmed_segment_sequence
            || payload.last_segment_sequence as usize > segments.len()
        {
            return Err(
                "StorageCorrupt: live coverage journal is not a verified contiguous prefix".into(),
            );
        }
        let start = confirmed_segment_sequence as usize;
        let end = payload.last_segment_sequence as usize;
        let group_start = request
            .group_offset_samples
            .checked_add(payload.start_sample)
            .ok_or_else(|| "StorageCorrupt: coverage offset overflow".to_owned())?;
        let group_end = request
            .group_offset_samples
            .checked_add(payload.end_sample)
            .ok_or_else(|| "StorageCorrupt: coverage offset overflow".to_owned())?;
        if segments[start..end]
            .iter()
            .enumerate()
            .any(|(offset, segment)| {
                segment.sequence != confirmed_segment_sequence + offset as u64 + 1
                    || segment.range.sample_rate_hz != 16_000
                    || segment.range.start_sample < group_start
                    || segment.range.end_sample > group_end
                    || segment.range.start_sample >= segment.range.end_sample
            })
        {
            return Err(
                "StorageCorrupt: coverage journal does not cover its persisted segments".into(),
            );
        }
        confirmed_samples = payload.end_sample;
        confirmed_segment_sequence = payload.last_segment_sequence;
        expected_window_sequence = expected_window_sequence
            .checked_add(1)
            .ok_or_else(|| "StorageCorrupt: coverage sequence exhausted".to_owned())?;
    }
    Ok(confirmed_samples)
}

pub(super) fn read_bounded_jsonl<T: for<'de> Deserialize<'de>>(
    path: &Path,
) -> Result<Vec<T>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut input = BufReader::new(file);
    let mut records = Vec::new();
    loop {
        let mut bytes = Vec::with_capacity(4096);
        let mut complete = false;
        loop {
            let available = input.fill_buf().map_err(|error| error.to_string())?;
            if available.is_empty() {
                break;
            }
            let consumed = available
                .iter()
                .position(|byte| *byte == b'\n')
                .map_or(available.len(), |index| index + 1);
            if bytes.len().saturating_add(consumed) > 1_048_576 {
                return Err("StorageCorrupt: JSONL record exceeds 1 MiB".into());
            }
            let has_newline = available.get(consumed.saturating_sub(1)) == Some(&b'\n');
            bytes.extend_from_slice(&available[..consumed]);
            input.consume(consumed);
            if has_newline {
                complete = true;
                break;
            }
        }
        if bytes.is_empty() || !complete {
            break;
        }
        bytes.pop();
        let record = serde_json::from_slice(&bytes)
            .map_err(|error| format!("StorageCorrupt: JSONL record: {error}"))?;
        records.push(record);
    }
    Ok(records)
}
