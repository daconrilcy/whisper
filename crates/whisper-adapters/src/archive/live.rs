use super::{
    ArchiveStore, CoverageRecord, CoverageRecordPayload, Digest, LiveArchive, LiveAttemptPayload,
    LiveAttemptRecord, LiveAttemptState, LiveCompleteRecord, LiveRequest, OpenOptions, Path,
    PathBuf, Seek, SeekFrom, SegmentRecord, Sha256, SourceRange, WorkerSegment, Write,
    assert_campaign_archive, campaign_gate, fs, hex, is_sha256, live_current_generation,
    read_segments, read_verified_live_manifest, reconcile_alias, record_campaign_operation_result,
    scan_live, timestamp, unix_time_ms, verified_artifact, verify_artifact, verify_pcm_staging,
    write_replace_synced,
};

impl LiveArchive {
    pub fn prepare(root: impl AsRef<Path>, mut request: LiveRequest) -> Result<Self, String> {
        assert_campaign_archive(root.as_ref())?;
        let directory = root
            .as_ref()
            .join("transcriptions")
            .join(format!("{:032x}", request.job_id.0))
            .join("live");
        fs::create_dir_all(&directory).map_err(|e| format!("StorageUnavailable: {e}"))?;
        let lock_path = directory.join(format!("group-{:032x}.writer.lock", request.job_id.0));
        let writer_lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(lock_path)
            .map_err(|error| format!("StorageUnavailable: archive lock open: {error}"))?;
        writer_lock.try_lock().map_err(|error| {
            format!("Recoverable: another live archive writer owns the passage: {error}")
        })?;
        let mut previous_completed_at_ms = None;
        if request.generation.get() > 1 {
            let previous =
                directory.join(format!("manifest-{}.json", request.generation.get() - 1));
            if previous.is_file() {
                let record: LiveCompleteRecord =
                    serde_json::from_slice(&fs::read(previous).map_err(|e| e.to_string())?)
                        .map_err(|e| {
                            format!("StorageCorrupt: previous live passage manifest: {e}")
                        })?;
                if record.version != 1
                    || record.job_id != request.job_id
                    || record.generation.get() + 1 != request.generation.get()
                    || !is_sha256(&record.pcm_sha256)
                {
                    return Err("StorageCorrupt: previous live passage identity mismatch".into());
                }
                let current = fs::read_to_string(directory.join("CURRENT")).map_err(|e| {
                    format!("StorageCorrupt: previous live passage is not committed: {e}")
                })?;
                if current != format!("manifest-{}.json\n", request.generation.get() - 1) {
                    return Err(
                        "Recoverable: previous live passage is not the committed group tail".into(),
                    );
                }
                verify_artifact(&directory, &record.staging)?;
                verify_pcm_staging(
                    &directory,
                    &record.staging,
                    record.audio_samples,
                    &record.pcm_sha256,
                )?;
                verify_artifact(&directory, &record.mp3)?;
                verify_artifact(&directory, &record.text)?;
                verify_artifact(&directory, &record.srt)?;
                request.group_offset_samples = record
                    .group_offset_samples
                    .checked_add(record.audio_samples)
                    .ok_or_else(|| "StorageCorrupt: live group sample axis overflow".to_owned())?;
                previous_completed_at_ms = Some(record.completed_at_unix_ms);
            } else {
                let pending_path = directory.join(format!(
                    "passage-{}.pending.json",
                    request.generation.get() - 1
                ));
                if pending_path.is_file() {
                    let previous_request: LiveRequest = serde_json::from_slice(
                        &fs::read(&pending_path).map_err(|e| e.to_string())?,
                    )
                    .map_err(|e| format!("StorageCorrupt: previous live pending record: {e}"))?;
                    if previous_request.job_id != request.job_id
                        || previous_request.generation.get() + 1 != request.generation.get()
                        || previous_request.destination != request.destination
                    {
                        return Err(
                            "StorageCorrupt: previous recoverable passage identity mismatch".into(),
                        );
                    }
                    // A pending passage is not a committed group tail, even when
                    // its current PCM and coverage happen to agree. Keep its exact
                    // identity recoverable and require explicit reconciliation
                    // before any later generation can be admitted.
                    return Err(format!(
                        "Recoverable: passage {} remains unresolved; reconcile this passage before starting the next one",
                        previous_request.generation.get()
                    ));
                }
            }
        }
        let pending_path =
            directory.join(format!("passage-{}.pending.json", request.generation.get()));
        let mut pending = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&pending_path)
            .map_err(|e| format!("StorageUnavailable: pending passage: {e}"))?;
        serde_json::to_writer(&mut pending, &request).map_err(|e| e.to_string())?;
        pending
            .sync_all()
            .map_err(|e| format!("StorageUnavailable: pending sync: {e}"))?;
        let mp3_pending_path =
            directory.join(format!("passage-{}.mp3.pending", request.generation.get()));
        let mp3 = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&mp3_pending_path)
            .map_err(|e| format!("StorageUnavailable: passage MP3: {e}"))?;
        Ok(Self {
            directory,
            request,
            _writer_lock: writer_lock,
            pending_path,
            mp3_pending_path,
            mp3: Some(mp3),
            expected_sequence: 1,
            expected_window_sequence: 1,
            confirmed_samples: 0,
            confirmed_segment_count: 0,
            next_attempt: 1,
            next_attempt_record_sequence: 1,
            active_attempt: None,
            segments: Vec::new(),
            previous_completed_at_ms,
        })
    }

    /// Finalizes the group's pause offset immediately before opening the live capture path.
    pub fn capture_started(&mut self) -> Result<(), String> {
        if let Some(completed_at) = self.previous_completed_at_ms.take() {
            let pause_samples = unix_time_ms()?
                .saturating_sub(completed_at)
                .saturating_mul(16);
            self.request.group_offset_samples = self
                .request
                .group_offset_samples
                .checked_add(pause_samples)
                .ok_or_else(|| "StorageCorrupt: live group pause offset overflow".to_owned())?;
            let bytes = serde_json::to_vec(&self.request).map_err(|e| e.to_string())?;
            write_replace_synced(&self.pending_path, &bytes)?;
        }
        Ok(())
    }

    pub fn append_mp3_packet(&mut self, packet: &[u8]) -> Result<(), String> {
        self.mp3
            .as_mut()
            .ok_or_else(|| "StorageUnavailable: MP3 passage is already finalized".to_owned())?
            .write_all(packet)
            .map_err(|e| format!("StorageUnavailable: MP3 write: {e}"))
    }

    pub fn reserve_attempt(&mut self) -> Result<u64, String> {
        if self.active_attempt.is_some() {
            return Err("StorageCorrupt: prior attempt has not been retired".into());
        }
        let attempt = self.next_attempt;
        self.append_attempt_record(attempt, LiveAttemptState::Reserved)?;
        self.active_attempt = Some((attempt, LiveAttemptState::Reserved));
        self.next_attempt = attempt
            .checked_add(1)
            .ok_or_else(|| "StorageCorrupt: live attempt sequence exhausted".to_owned())?;
        Ok(attempt)
    }

    pub fn mark_attempt_started(&mut self, attempt: u64) -> Result<(), String> {
        if self.active_attempt != Some((attempt, LiveAttemptState::Reserved)) {
            return Err("StorageCorrupt: Started requires the matching Reserved attempt".into());
        }
        self.append_attempt_record(attempt, LiveAttemptState::Started)?;
        self.active_attempt = Some((attempt, LiveAttemptState::Started));
        Ok(())
    }

    pub fn retire_attempt(&mut self, attempt: u64) -> Result<(), String> {
        if self.active_attempt != Some((attempt, LiveAttemptState::Started)) {
            return Err("StorageCorrupt: Retired requires the matching Started attempt".into());
        }
        self.append_attempt_record(attempt, LiveAttemptState::Retired)?;
        self.active_attempt = None;
        Ok(())
    }

    pub fn restart_mp3_attempt(&mut self) -> Result<(), String> {
        let file = self
            .mp3
            .as_mut()
            .ok_or_else(|| "StorageUnavailable: MP3 passage is already finalized".to_owned())?;
        file.set_len(0)
            .and_then(|()| file.seek(SeekFrom::Start(0)))
            .and_then(|_| file.sync_all())
            .map_err(|error| format!("StorageUnavailable: reset MP3 attempt: {error}"))?;
        Ok(())
    }

    pub fn discard_unconfirmed_segments(&mut self) -> Result<(), String> {
        self.segments.truncate(self.confirmed_segment_count);
        let path = self.directory.join(format!(
            "passage-{}.segments.jsonl",
            self.request.generation.get()
        ));
        let mut bytes = Vec::new();
        for segment in &self.segments {
            serde_json::to_writer(&mut bytes, segment).map_err(|error| error.to_string())?;
            bytes.extend_from_slice(b"\n");
        }
        write_replace_synced(&path, &bytes)?;
        self.expected_sequence = u64::try_from(self.confirmed_segment_count)
            .ok()
            .and_then(|sequence| sequence.checked_add(1))
            .ok_or_else(|| "StorageCorrupt: live segment sequence exhausted".to_owned())?;
        Ok(())
    }

    fn append_attempt_record(
        &mut self,
        attempt: u64,
        state: LiveAttemptState,
    ) -> Result<(), String> {
        let payload = LiveAttemptPayload {
            version: 1,
            journal_sequence: self.next_attempt_record_sequence,
            job_id: self.request.job_id,
            generation: self.request.generation,
            attempt,
            state,
        };
        let canonical = serde_json::to_vec(&payload).map_err(|error| error.to_string())?;
        let record = LiveAttemptRecord {
            payload,
            sha256: format!("{:x}", Sha256::digest(canonical)),
        };
        let bytes = serde_json::to_vec(&record).map_err(|error| error.to_string())?;
        if bytes.len() + 1 > 1_048_576 {
            return Err("StorageCorrupt: attempt journal record exceeds 1 MiB".into());
        }
        let path = self.directory.join(format!(
            "passage-{}.attempts.jsonl",
            self.request.generation.get()
        ));
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|error| format!("StorageUnavailable: attempt journal: {error}"))?;
        campaign_gate(
            "attempt.before_record",
            self.request.job_id.0,
            self.request.generation.get(),
            0,
        )?;
        let operation = file
            .write_all(&bytes)
            .and_then(|()| file.write_all(b"\n"))
            .and_then(|()| file.sync_all())
            .map_err(|error| format!("StorageUnavailable: attempt journal sync: {error}"));
        record_campaign_operation_result(
            "attempt.before_record",
            self.request.job_id.0,
            self.request.generation.get(),
            0,
            &operation,
            Some(&path),
        )?;
        operation?;
        campaign_gate(
            "attempt.after_sync",
            self.request.job_id.0,
            self.request.generation.get(),
            0,
        )?;
        self.next_attempt_record_sequence = self
            .next_attempt_record_sequence
            .checked_add(1)
            .ok_or_else(|| "StorageCorrupt: attempt journal sequence exhausted".to_owned())?;
        Ok(())
    }

    pub fn persist_segment(
        &mut self,
        segment: &WorkerSegment,
        audio_durable_samples: u64,
    ) -> Result<u64, String> {
        if segment.job_id != self.request.job_id
            || segment.generation != self.request.generation
            || segment.segment_id.0 != self.expected_sequence
            || segment.range.sample_rate_hz != 16_000
            || segment.range.start_sample >= segment.range.end_sample
            || segment.range.end_sample > audio_durable_samples
            || segment.text.trim().is_empty()
        {
            return Err(
                "StorageCorrupt: live fragment is not covered by a durable PCM prefix".into(),
            );
        }
        let range = SourceRange::new(
            self.request
                .group_offset_samples
                .saturating_add(segment.range.start_sample),
            self.request
                .group_offset_samples
                .saturating_add(segment.range.end_sample),
            16_000,
        )
        .ok_or_else(|| "StorageCorrupt: live group offset overflow".to_owned())?;
        let record = SegmentRecord {
            sequence: segment.segment_id.0,
            range,
            text: segment.text.trim().to_owned(),
        };
        let path = self.directory.join(format!(
            "passage-{}.segments.jsonl",
            self.request.generation.get()
        ));
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| format!("StorageUnavailable: {e}"))?;
        serde_json::to_writer(&mut file, &record).map_err(|e| e.to_string())?;
        file.write_all(b"\n").map_err(|e| e.to_string())?;
        file.sync_all()
            .map_err(|e| format!("StorageUnavailable: fragment sync: {e}"))?;
        self.expected_sequence = self
            .expected_sequence
            .checked_add(1)
            .ok_or_else(|| "segment sequence exhausted".to_owned())?;
        self.segments.push(record);
        Ok(self.segments.last().map_or(0, |item| item.range.end_sample))
    }

    pub fn persist_window_finished(
        &mut self,
        sequence: u64,
        range: SourceRange,
        last_segment_sequence: u64,
        audio_durable_samples: u64,
    ) -> Result<u64, String> {
        if sequence != self.expected_window_sequence
            || range.sample_rate_hz != 16_000
            || range.start_sample != self.confirmed_samples
            || range.end_sample <= range.start_sample
            || range.end_sample > audio_durable_samples
            || last_segment_sequence.checked_add(1) != Some(self.expected_sequence)
        {
            return Err("StorageCorrupt: live window confirmation is not contiguous".into());
        }
        let group_start = self
            .request
            .group_offset_samples
            .checked_add(range.start_sample)
            .ok_or_else(|| "StorageCorrupt: live window group offset overflow".to_owned())?;
        let group_end = self
            .request
            .group_offset_samples
            .checked_add(range.end_sample)
            .ok_or_else(|| "StorageCorrupt: live window group offset overflow".to_owned())?;
        let new_segments = self
            .segments
            .get(self.confirmed_segment_count..)
            .ok_or_else(|| "StorageCorrupt: confirmed segment cursor is invalid".to_owned())?;
        if new_segments.iter().any(|segment| {
            segment.sequence > last_segment_sequence
                || segment.range.start_sample < group_start
                || segment.range.end_sample > group_end
        }) || new_segments
            .last()
            .map_or(self.confirmed_segment_count as u64, |segment| {
                segment.sequence
            })
            != last_segment_sequence
        {
            return Err("StorageCorrupt: window confirmation does not cover its segments".into());
        }
        let payload = CoverageRecordPayload {
            version: 2,
            job_id: self.request.job_id,
            generation: self.request.generation,
            window_sequence: sequence,
            start_sample: range.start_sample,
            end_sample: range.end_sample,
            last_segment_sequence,
        };
        let canonical = serde_json::to_vec(&payload).map_err(|error| error.to_string())?;
        let checksum = format!("{:x}", Sha256::digest(canonical));
        let bytes = serde_json::to_vec(&CoverageRecord {
            payload,
            sha256: checksum,
        })
        .map_err(|error| error.to_string())?;
        if bytes.len() + 1 > 1_048_576 {
            return Err("StorageCorrupt: coverage confirmation exceeds record limit".into());
        }
        let path = self.coverage_path();
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|error| format!("StorageUnavailable: coverage confirmation: {error}"))?;
        campaign_gate(
            "coverage.before_record",
            self.request.job_id.0,
            self.request.generation.get(),
            0,
        )?;
        let operation = file
            .write_all(&bytes)
            .and_then(|()| file.write_all(b"\n"))
            .and_then(|()| file.sync_all())
            .map_err(|error| format!("StorageUnavailable: coverage sync: {error}"));
        record_campaign_operation_result(
            "coverage.before_record",
            self.request.job_id.0,
            self.request.generation.get(),
            0,
            &operation,
            Some(&path),
        )?;
        operation?;
        campaign_gate(
            "coverage.after_sync",
            self.request.job_id.0,
            self.request.generation.get(),
            0,
        )?;
        self.expected_window_sequence = self
            .expected_window_sequence
            .checked_add(1)
            .ok_or_else(|| "StorageCorrupt: window sequence exhausted".to_owned())?;
        self.confirmed_samples = range.end_sample;
        self.confirmed_segment_count = self.segments.len();
        Ok(self.confirmed_samples)
    }

    pub fn confirmed_samples(&self) -> u64 {
        self.confirmed_samples
    }

    pub fn next_segment_sequence(&self) -> u64 {
        self.expected_sequence
    }

    pub fn next_window_sequence(&self) -> u64 {
        self.expected_window_sequence
    }

    fn coverage_path(&self) -> PathBuf {
        self.directory.join(format!(
            "passage-{}.coverage.jsonl",
            self.request.generation.get()
        ))
    }

    pub fn publish(
        &mut self,
        audio_durable_samples: u64,
        confirmed_fragment_end: u64,
        pcm_sha256: [u8; 32],
    ) -> Result<(), String> {
        if audio_durable_samples == 0 {
            return Err(
                "Recoverable: no audio samples were durably captured; passage was not published"
                    .into(),
            );
        }
        if self.confirmed_samples != audio_durable_samples {
            return Err(
                "Recoverable: durable PCM is not fully covered by completed windows".into(),
            );
        }
        if self
            .segments
            .iter()
            .any(|segment| segment.range.end_sample > confirmed_fragment_end)
        {
            return Err("StorageCorrupt: fragment confirmation does not cover the journal".into());
        }
        campaign_gate(
            "live.before_mp3_sync",
            self.request.job_id.0,
            self.request.generation.get(),
            0,
        )?;
        let operation = self
            .mp3
            .as_mut()
            .ok_or_else(|| "StorageUnavailable: MP3 passage is already finalized".to_owned())?
            .sync_all()
            .map_err(|e| format!("StorageUnavailable: MP3 sync: {e}"));
        record_campaign_operation_result(
            "live.before_mp3_sync",
            self.request.job_id.0,
            self.request.generation.get(),
            0,
            &operation,
            Some(&self.mp3_pending_path),
        )?;
        operation?;
        campaign_gate(
            "live.after_mp3_sync",
            self.request.job_id.0,
            self.request.generation.get(),
            0,
        )?;
        drop(self.mp3.take());
        let mp3_path = self
            .directory
            .join(format!("passage-{}.mp3", self.request.generation.get()));
        fs::rename(&self.mp3_pending_path, &mp3_path)
            .map_err(|e| format!("StorageUnavailable: MP3 publish: {e}"))?;
        let mut segments = Vec::new();
        for generation in 1..=self.request.generation.get() {
            let path = self
                .directory
                .join(format!("passage-{generation}.segments.jsonl"));
            if path.exists() {
                segments.extend(read_segments(&path)?);
            }
        }
        let text = segments
            .iter()
            .map(|segment| segment.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let srt = segments
            .iter()
            .enumerate()
            .map(|(index, segment)| {
                format!(
                    "{}\n{} --> {}\n{}\n",
                    index + 1,
                    timestamp(segment.range.start_sample),
                    timestamp(segment.range.end_sample),
                    segment.text
                )
            })
            .collect::<String>();
        let passage = self.request.generation.get();
        let text_path = self.directory.join(format!("transcript-{passage}.txt"));
        let srt_path = self.directory.join(format!("transcript-{passage}.srt"));
        write_replace_synced(&text_path, text.as_bytes())?;
        write_replace_synced(&srt_path, srt.as_bytes())?;
        // Manifests reference immutable passage snapshots so later passages cannot
        // make an earlier generation fail hash verification.
        let record = LiveCompleteRecord {
            version: 1,
            job_id: self.request.job_id,
            generation: self.request.generation,
            audio_samples: audio_durable_samples,
            confirmed_samples: self.confirmed_samples,
            group_offset_samples: self.request.group_offset_samples,
            completed_at_unix_ms: unix_time_ms()?,
            pcm_sha256: hex(&pcm_sha256),
            staging: verified_artifact(
                &self.directory,
                &self
                    .directory
                    .join(format!("passage-{}.pcm", self.request.generation.get())),
            )?,
            mp3: verified_artifact(&self.directory, &mp3_path)?,
            text: verified_artifact(&self.directory, &text_path)?,
            srt: verified_artifact(&self.directory, &srt_path)?,
        };
        let manifest_name = format!("manifest-{}.json", self.request.generation.get());
        let manifest_path = self.directory.join(&manifest_name);
        campaign_gate(
            "live.before_manifest",
            self.request.job_id.0,
            self.request.generation.get(),
            0,
        )?;
        let manifest_result = write_replace_synced(
            &manifest_path,
            &serde_json::to_vec(&record).map_err(|e| e.to_string())?,
        );
        record_campaign_operation_result(
            "live.before_manifest",
            self.request.job_id.0,
            self.request.generation.get(),
            0,
            &manifest_result,
            Some(&manifest_path),
        )?;
        manifest_result?;
        campaign_gate(
            "live.after_manifest",
            self.request.job_id.0,
            self.request.generation.get(),
            0,
        )?;
        verify_artifact(&self.directory, &record.mp3)?;
        verify_pcm_staging(
            &self.directory,
            &record.staging,
            audio_durable_samples,
            &record.pcm_sha256,
        )?;
        verify_artifact(&self.directory, &record.text)?;
        verify_artifact(&self.directory, &record.srt)?;
        campaign_gate(
            "live.before_current",
            self.request.job_id.0,
            self.request.generation.get(),
            0,
        )?;
        let current_path = self.directory.join("CURRENT");
        let current_result =
            write_replace_synced(&current_path, format!("{manifest_name}\n").as_bytes());
        record_campaign_operation_result(
            "live.before_current",
            self.request.job_id.0,
            self.request.generation.get(),
            0,
            &current_result,
            Some(&current_path),
        )?;
        current_result?;
        campaign_gate(
            "live.after_current",
            self.request.job_id.0,
            self.request.generation.get(),
            0,
        )?;
        // CURRENT is the commit point. Update compatibility aliases only after it;
        // scan_live repairs aliases from the committed immutable snapshot after a crash.
        let _ = write_replace_synced(&self.directory.join("transcript.txt"), text.as_bytes());
        let _ = write_replace_synced(&self.directory.join("transcript.srt"), srt.as_bytes());
        let _ = fs::remove_file(&self.pending_path);
        self._writer_lock
            .unlock()
            .map_err(|error| format!("StorageUnavailable: archive lock release: {error}"))?;
        Ok(())
    }
}

impl ArchiveStore {
    /// Explicitly commits a verified live manifest that was durable before a
    /// crash but was not yet selected by CURRENT.
    pub fn resolve_live_publication(root: &Path, request: &LiveRequest) -> Result<(), String> {
        assert_campaign_archive(root)?;
        let generation = request.generation.get();
        let directory = root
            .join("transcriptions")
            .join(format!("{:032x}", request.job_id.0))
            .join("live");
        let lock_path = directory.join(format!("group-{:032x}.writer.lock", request.job_id.0));
        let writer_lock = OpenOptions::new()
            .read(true)
            .write(true)
            .open(lock_path)
            .map_err(|error| format!("Recoverable: live writer lock unavailable: {error}"))?;
        writer_lock.try_lock().map_err(|error| {
            format!("Recoverable: another live archive writer owns the group: {error}")
        })?;

        let current = live_current_generation(&directory)?;
        let expected_current = generation.checked_sub(1);
        if current != expected_current {
            return Err(
                "Recoverable: live publication is no longer the next group generation".into(),
            );
        }
        let pending_path = directory.join(format!("passage-{generation}.pending.json"));
        let pending: LiveRequest =
            serde_json::from_slice(&fs::read(&pending_path).map_err(|error| {
                format!("Recoverable: live publication request unavailable: {error}")
            })?)
            .map_err(|error| format!("StorageCorrupt: live publication request: {error}"))?;
        if pending != *request {
            return Err("Recoverable: live publication request identity changed".into());
        }

        let record = read_verified_live_manifest(&directory, request.job_id, request.generation)?;
        if record.confirmed_samples != record.audio_samples {
            return Err("Recoverable: live publication does not cover all durable audio".into());
        }
        let entries = scan_live(&directory, request.job_id)?;
        if entries
            .iter()
            .any(|entry| entry.generation.get() > generation)
        {
            return Err(
                "Recoverable: a later live generation prevents publication resolution".into(),
            );
        }
        let target = entries
            .iter()
            .find(|entry| entry.generation == request.generation)
            .ok_or_else(|| "Recoverable: verified live publication is unavailable".to_owned())?;
        if target.complete {
            return Err("Recoverable: live publication is already committed".into());
        }

        let current_path = directory.join("CURRENT");
        let manifest_name = format!("manifest-{generation}.json");
        write_replace_synced(&current_path, format!("{manifest_name}\n").as_bytes())?;
        reconcile_alias(&directory, &record.text, "transcript.txt")?;
        reconcile_alias(&directory, &record.srt, "transcript.srt")?;
        let _ = fs::remove_file(pending_path);
        writer_lock
            .unlock()
            .map_err(|error| format!("StorageUnavailable: archive lock release: {error}"))?;
        Ok(())
    }
}
