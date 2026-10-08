use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};
use symphonia::core::audio::Channels;
use symphonia::core::codecs::CODEC_TYPE_MP3;
use symphonia::core::{
    formats::FormatOptions, io::MediaSourceStream, meta::MetadataOptions, probe::Hint,
};
use whisper_core::{
    Generation, JobId, SourceRange,
    ports::{ImportRequest, LiveRequest, WorkerSegment},
};

const APPROVED_MODEL_SIZE: u64 = 1_624_555_275;
const APPROVED_MODEL_SHA256: &str =
    "1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69";
const PCM_STAGING_HEADER: &[u8] = b"WHISPCM1\0\x80\x3e\0\0\x01\0";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SegmentRecord {
    pub sequence: u64,
    pub range: SourceRange,
    pub text: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct PendingRecord {
    version: u32,
    job_id: JobId,
    generation: Generation,
    source_path: String,
    source_sha256: String,
    source_samples: u64,
    model_path: String,
    model_sha256: String,
    model_size: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct ArtifactRecord {
    path: String,
    sha256: String,
    bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct CompleteRecord {
    version: u32,
    job_id: JobId,
    generation: Generation,
    source_sha256: String,
    segments: u64,
    text: ArtifactRecord,
    srt: ArtifactRecord,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchiveEntry {
    pub job_id: JobId,
    pub generation: Generation,
    pub complete: bool,
    pub source_sha256: String,
    pub diagnostic: Option<String>,
}

pub struct ArchiveStore {
    root: PathBuf,
    current: Option<ActiveArchive>,
}

#[derive(Serialize, Deserialize)]
struct LiveCompleteRecord {
    version: u32,
    job_id: JobId,
    generation: Generation,
    audio_samples: u64,
    #[serde(default)]
    confirmed_samples: u64,
    group_offset_samples: u64,
    completed_at_unix_ms: u64,
    pcm_sha256: String,
    staging: ArtifactRecord,
    mp3: ArtifactRecord,
    text: ArtifactRecord,
    srt: ArtifactRecord,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct CoverageRecordPayload {
    version: u32,
    job_id: JobId,
    generation: Generation,
    window_sequence: u64,
    start_sample: u64,
    end_sample: u64,
    last_segment_sequence: u64,
}

#[derive(Serialize, Deserialize)]
struct CoverageRecord {
    payload: CoverageRecordPayload,
    sha256: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum LiveAttemptState {
    Reserved,
    Started,
    Retired,
}

#[derive(Serialize, Deserialize)]
struct LiveAttemptRecord {
    payload: LiveAttemptPayload,
    sha256: String,
}

#[derive(Serialize, Deserialize)]
struct LiveAttemptPayload {
    version: u32,
    journal_sequence: u64,
    job_id: JobId,
    generation: Generation,
    attempt: u64,
    state: LiveAttemptState,
}

/// Passage-oriented durable publisher for live jobs. `CURRENT` is written only after
/// the MP3, cumulative transcript, subtitle, and manifest have been synced and verified.
pub struct LiveArchive {
    directory: PathBuf,
    request: LiveRequest,
    _writer_lock: File,
    pending_path: PathBuf,
    mp3_pending_path: PathBuf,
    mp3: Option<File>,
    expected_sequence: u64,
    expected_window_sequence: u64,
    confirmed_samples: u64,
    confirmed_segment_count: usize,
    next_attempt: u64,
    next_attempt_record_sequence: u64,
    active_attempt: Option<(u64, LiveAttemptState)>,
    segments: Vec<SegmentRecord>,
    previous_completed_at_ms: Option<u64>,
}

impl LiveArchive {
    pub fn prepare(root: impl AsRef<Path>, mut request: LiveRequest) -> Result<Self, String> {
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
                        "StorageCorrupt: previous live passage is not the committed group tail"
                            .into(),
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
                    let pcm_path =
                        directory.join(format!("passage-{}.pcm", request.generation.get() - 1));
                    let checkpoint = crate::staging::PcmStaging::inspect_checkpoint(&pcm_path)?;
                    let old_segments = read_segments(&directory.join(format!(
                        "passage-{}.segments.jsonl",
                        request.generation.get() - 1
                    )))?;
                    if old_segments.iter().any(|segment| {
                        segment.range.sample_rate_hz != 16_000
                            || segment.range.end_sample <= previous_request.group_offset_samples
                            || segment.range.end_sample - previous_request.group_offset_samples
                                > checkpoint.durable_samples
                    }) {
                        return Err(
                            "StorageCorrupt: confirmed text is outside the durable PCM checkpoint"
                                .into(),
                        );
                    }
                    request.group_offset_samples = previous_request
                        .group_offset_samples
                        .checked_add(checkpoint.durable_samples)
                        .ok_or_else(|| {
                            "StorageCorrupt: recovered live group offset overflow".to_owned()
                        })?;
                    previous_completed_at_ms = Some(checkpoint.synced_at_unix_ms);
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
            .open(path)
            .map_err(|error| format!("StorageUnavailable: attempt journal: {error}"))?;
        file.write_all(&bytes)
            .and_then(|()| file.write_all(b"\n"))
            .and_then(|()| file.sync_all())
            .map_err(|error| format!("StorageUnavailable: attempt journal sync: {error}"))?;
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
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.coverage_path())
            .map_err(|error| format!("StorageUnavailable: coverage confirmation: {error}"))?;
        file.write_all(&bytes)
            .and_then(|()| file.write_all(b"\n"))
            .and_then(|()| file.sync_all())
            .map_err(|error| format!("StorageUnavailable: coverage sync: {error}"))?;
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
        self.mp3
            .as_mut()
            .ok_or_else(|| "StorageUnavailable: MP3 passage is already finalized".to_owned())?
            .sync_all()
            .map_err(|e| format!("StorageUnavailable: MP3 sync: {e}"))?;
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
        write_replace_synced(
            &manifest_path,
            &serde_json::to_vec(&record).map_err(|e| e.to_string())?,
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
        write_replace_synced(
            &self.directory.join("CURRENT"),
            format!("{manifest_name}\n").as_bytes(),
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
struct ActiveArchive {
    job_id: JobId,
    generation: Generation,
    directory: PathBuf,
    source_sha256: [u8; 32],
    source_samples: u64,
    expected_sequence: u64,
}

impl ArchiveStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            current: None,
        }
    }
    pub fn root_path(&self) -> PathBuf {
        self.root.clone()
    }

    /// Captures the source identity at durable queue admission without creating an archive.
    pub fn identify_source(request: &mut ImportRequest) -> Result<(), String> {
        let source = Path::new(&request.source_path);
        if !source.is_file() {
            return Err("SourceMissing: selected audio source does not exist".into());
        }
        let source_sha = sha256_file(source)?;
        let source_samples = source_output_samples(source)?;
        if sha256_file(source)? != source_sha {
            return Err("SourceChanged: source changed while its identity was inspected".into());
        }
        request.source_sha256 = Some(source_sha);
        request.source_samples = Some(source_samples);
        Ok(())
    }

    pub fn prepare(&mut self, mut request: ImportRequest) -> Result<ImportRequest, String> {
        if self.current.is_some() {
            return Err("Busy: an import is already active".into());
        }
        let source = Path::new(&request.source_path);
        if !source.is_file() {
            return Err("SourceMissing: selected audio source does not exist".into());
        }
        let source_sha = sha256_file(source)?;
        if request
            .source_sha256
            .is_some_and(|expected| expected != source_sha)
        {
            return Err("SourceChanged: source hash differs from the accepted identity".into());
        }
        let source_samples = source_output_samples(source)?;
        if request
            .source_samples
            .is_some_and(|expected| expected != source_samples)
        {
            return Err("SourceChanged: source duration differs from the accepted identity".into());
        }
        if sha256_file(source)? != source_sha {
            return Err(
                "SourceChanged: audio source changed while its duration was inspected".into(),
            );
        }
        let model_meta =
            fs::metadata(&request.model_path).map_err(|e| format!("ModelMissing: {e}"))?;
        let model_hash = sha256_file(Path::new(&request.model_path))?;
        if hex(&model_hash) != APPROVED_MODEL_SHA256
            || model_meta.len() != APPROVED_MODEL_SIZE
            || request.model_sha256 != model_hash
        {
            return Err("ModelHashMismatch: model is not the approved D19 payload".into());
        }
        let destination = PathBuf::from(&request.destination);
        fs::create_dir_all(&destination).map_err(|e| format!("StorageUnavailable: {e}"))?;
        let directory = destination
            .join("transcriptions")
            .join(format!("{:032x}", request.job_id.0))
            .join(request.generation.get().to_string());
        fs::create_dir_all(&directory).map_err(|e| format!("StorageUnavailable: {e}"))?;
        let pending = PendingRecord {
            version: 1,
            job_id: request.job_id,
            generation: request.generation,
            source_path: request.source_path.clone(),
            source_sha256: hex(&source_sha),
            source_samples,
            model_path: request.model_path.clone(),
            model_sha256: hex(&model_hash),
            model_size: model_meta.len(),
        };
        let expected_sequence = initialize_pending(&directory, &pending)?;
        request.source_sha256 = Some(source_sha);
        request.source_samples = Some(source_samples);
        self.current = Some(ActiveArchive {
            job_id: request.job_id,
            generation: request.generation,
            directory,
            source_sha256: source_sha,
            source_samples,
            expected_sequence,
        });
        Ok(request)
    }

    /// Starts the next generation by copying only the previously confirmed prefix.
    pub fn resume_from(
        &mut self,
        previous: &ImportRequest,
        mut request: ImportRequest,
    ) -> Result<(ImportRequest, Vec<SegmentRecord>), String> {
        if request.job_id != previous.job_id
            || previous.generation.next() != Some(request.generation)
        {
            return Err("InvalidInput: resume generation is not the next generation".into());
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
        if pending.source_path != previous.source_path
            || Some(pending.source_sha256.as_str())
                != previous.source_sha256.map(|sha| hex(&sha)).as_deref()
            || Some(pending.source_samples) != previous.source_samples
            || pending.model_path != previous.model_path
            || pending.model_sha256 != hex(&previous.model_sha256)
        {
            return Err(
                "SourceChanged: resume checkpoint identity differs from the queued request".into(),
            );
        }
        let prefix = read_segments(&previous_dir.join("segments.jsonl"))?;
        if prefix.iter().enumerate().any(|(index, segment)| {
            segment.sequence != index as u64 + 1
                || segment.range.sample_rate_hz != 16_000
                || segment.range.start_sample >= segment.range.end_sample
                || segment.range.end_sample > pending.source_samples
                || segment.text.trim().is_empty()
        }) || prefix
            .windows(2)
            .any(|pair| pair[1].range.start_sample < pair[0].range.end_sample)
        {
            return Err(
                "StorageCorrupt: confirmed resume prefix is not a valid ordered checkpoint".into(),
            );
        }
        let prepared = self.prepare(request.clone())?;
        self.copy_resume_prefix(&prepared, &prefix)?;
        request = prepared.clone();
        Ok((request, prefix))
    }

    fn copy_resume_prefix(
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

    pub fn persist_segment(&mut self, segment: &WorkerSegment) -> Result<u64, String> {
        let active = self
            .current
            .as_mut()
            .ok_or_else(|| "StorageUnavailable: no prepared archive".to_owned())?;
        if segment.job_id != active.job_id || segment.generation != active.generation {
            return Err("StaleResponse: segment does not match pending generation".into());
        }
        if segment.segment_id.0 != active.expected_sequence
            || segment.range.sample_rate_hz != 16_000
            || segment.range.start_sample >= segment.range.end_sample
            || segment.range.end_sample > active.source_samples
            || segment.text.trim().is_empty()
        {
            return Err("StorageCorrupt: invalid segment sequence/range/content".into());
        }
        let record = SegmentRecord {
            sequence: segment.segment_id.0,
            range: segment.range,
            text: segment.text.clone(),
        };
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(active.directory.join("segments.jsonl"))
            .map_err(|e| format!("StorageUnavailable: {e}"))?;
        serde_json::to_writer(&mut file, &record).map_err(|e| e.to_string())?;
        file.write_all(b"\n").map_err(|e| e.to_string())?;
        file.sync_all()
            .map_err(|e| format!("StorageUnavailable: segment sync failed: {e}"))?;
        active.expected_sequence += 1;
        Ok(record.sequence)
    }

    pub fn prepare_publish(
        &mut self,
        job_id: JobId,
        generation: Generation,
        expected_count: u64,
    ) -> Result<(), String> {
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
        let records = read_segments(&active.directory.join("segments.jsonl"))?;
        if records.len() as u64 != expected_count
            || records
                .iter()
                .enumerate()
                .any(|(i, item)| item.sequence != i as u64 + 1)
        {
            return Err("StorageCorrupt: segment journal is incomplete".into());
        }
        let text = records
            .iter()
            .map(|r| r.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let srt = records
            .iter()
            .enumerate()
            .map(|(index, item)| {
                format!(
                    "{}\n{} --> {}\n{}\n",
                    index + 1,
                    timestamp(item.range.start_sample),
                    timestamp(item.range.end_sample),
                    item.text
                )
            })
            .collect::<String>();
        let text_path = active.directory.join("transcript.txt");
        let srt_path = active.directory.join("transcript.srt");
        write_replace_synced(&text_path, text.as_bytes())?;
        write_replace_synced(&srt_path, srt.as_bytes())?;
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
        write_replace_synced(&manifest_path, &manifest_bytes)?;
        let verified: CompleteRecord =
            serde_json::from_slice(&fs::read(&manifest_path).map_err(|e| e.to_string())?)
                .map_err(|e| format!("StorageCorrupt: manifest: {e}"))?;
        verify_artifact(&active.directory, &verified.text)?;
        verify_artifact(&active.directory, &verified.srt)?;
        Ok(())
    }

    pub fn commit_publish(&mut self, job_id: JobId, generation: Generation) -> Result<(), String> {
        let active = self
            .current
            .as_ref()
            .ok_or_else(|| "StorageUnavailable: no pending archive".to_owned())?;
        if active.job_id != job_id || active.generation != generation {
            return Err("StaleResponse: publish commit targets another generation".into());
        }
        write_replace_synced(&active.directory.join("CURRENT"), b"manifest.json\n")?;
        self.current = None;
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

    pub fn scan(root: &Path) -> Result<Vec<ArchiveEntry>, String> {
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

fn scan_live(dir: &Path, expected_job: JobId) -> Result<Vec<ArchiveEntry>, String> {
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
    let current_path = dir.join("CURRENT");
    let committed_generation = if current_path.exists() {
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
        let number = target
            .strip_prefix("manifest-")
            .and_then(|s| s.strip_suffix(".json"))
            .and_then(|s| s.parse::<u64>().ok())
            .filter(|number| {
                number.to_string()
                    == target
                        .trim_start_matches("manifest-")
                        .trim_end_matches(".json")
            })
            .ok_or_else(|| "StorageCorrupt: live CURRENT target is invalid".to_owned())?;
        Some(number)
    } else {
        None
    };
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
            let record: LiveCompleteRecord =
                serde_json::from_slice(&fs::read(item.path()).map_err(|e| e.to_string())?)
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
            });
        }
    }
    if let Some(number) = committed_generation {
        if !verified.contains(&number) {
            return Err("StorageCorrupt: live CURRENT target is not a verified passage".into());
        }
        if verified.iter().any(|generation| *generation > number) {
            return Err(
                "StorageCorrupt: live manifest is newer than the committed group tail".into(),
            );
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
                    source_sha256: String::new(), diagnostic: Some("Recoverable: live passage was not atomically published; explicit resume or discard required".into()) });
            }
        }
    }
    entries.sort_by_key(|entry| entry.generation.get());
    Ok(entries)
}

fn reconcile_alias(dir: &Path, source: &ArtifactRecord, alias: &str) -> Result<(), String> {
    let contents = fs::read(dir.join(&source.path)).map_err(|error| error.to_string())?;
    let alias_path = dir.join(alias);
    if fs::read(&alias_path).ok().as_deref() != Some(contents.as_slice()) {
        write_replace_synced(&alias_path, &contents)?;
    }
    Ok(())
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
        if generation != latest_generation || manifest_generations.contains(&generation) {
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
        let confirmed_samples =
            scan_confirmed_live_prefix(&dir, generation, &request, checkpoint.durable_samples)?;
        recoveries.push(whisper_core::ports::LiveRecoveryInfo {
            request,
            durable_samples: checkpoint.durable_samples,
            confirmed_samples,
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

fn read_bounded_jsonl<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<Vec<T>, String> {
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

fn safe_artifact_path(path: &str) -> bool {
    !path.is_empty()
        && Path::new(path)
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_)))
}

fn unix_time_ms() -> Result<u64, String> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .map_err(|error| format!("ClockUnavailable: system clock precedes Unix epoch: {error}"))
}

fn scan_generation(
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
        }));
    }
    if dir.join("pending.json").is_file() {
        let pending: PendingRecord =
            serde_json::from_slice(&fs::read(dir.join("pending.json")).map_err(|e| e.to_string())?)
                .map_err(|e| format!("StorageCorrupt: pending: {e}"))?;
        validate_pending(&pending, expected_job, expected_generation)?;
        return Ok(Some(ArchiveEntry {
            job_id: pending.job_id,
            generation: pending.generation,
            complete: false,
            source_sha256: pending.source_sha256,
            diagnostic: None,
        }));
    }
    if dir.join("manifest.json").exists() || dir.join("segments.jsonl").exists() {
        return Err("StorageCorrupt: artifacts exist without a pending or CURRENT marker".into());
    }
    Ok(None)
}

fn validate_pending(pending: &PendingRecord, job_id: JobId, generation: u64) -> Result<(), String> {
    if pending.version != 1
        || pending.job_id != job_id
        || pending.generation.get() != generation
        || !is_sha256(&pending.source_sha256)
        || pending.source_samples == 0
        || pending.model_sha256 != APPROVED_MODEL_SHA256
        || pending.model_size != APPROVED_MODEL_SIZE
    {
        return Err("StorageCorrupt: pending identity/schema is invalid".into());
    }
    Ok(())
}

fn initialize_pending(directory: &Path, pending: &PendingRecord) -> Result<u64, String> {
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

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn parse_job_id(value: &str) -> Option<JobId> {
    u128::from_str_radix(value, 16).ok().map(JobId)
}

fn corrupt_entry(job_id: JobId, location: &str, error: String) -> ArchiveEntry {
    ArchiveEntry {
        job_id,
        generation: Generation::first(),
        complete: false,
        source_sha256: String::new(),
        diagnostic: Some(format!("StorageCorrupt: {location}: {error}")),
    }
}

fn read_segments(path: &Path) -> Result<Vec<SegmentRecord>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|e| format!("StorageCorrupt: segment journal: {e}"))?;
    repair_uncommitted_segment_tail(&mut file)?;
    file.seek(SeekFrom::Start(0))
        .map_err(|e| format!("StorageCorrupt: segment seek: {e}"))?;
    BufReader::new(file)
        .lines()
        .map(|line| {
            let line = line.map_err(|e| format!("StorageCorrupt: segment: {e}"))?;
            serde_json::from_str(&line).map_err(|e| format!("StorageCorrupt: segment: {e}"))
        })
        .collect()
}

fn repair_uncommitted_segment_tail(file: &mut File) -> Result<(), String> {
    const SCAN_BYTES: usize = 8 * 1024;
    let length = file
        .metadata()
        .map_err(|e| format!("StorageCorrupt: segment metadata: {e}"))?
        .len();
    if length == 0 {
        return Ok(());
    }
    file.seek(SeekFrom::End(-1))
        .map_err(|e| format!("StorageCorrupt: segment seek: {e}"))?;
    let mut last = [0_u8; 1];
    file.read_exact(&mut last)
        .map_err(|e| format!("StorageCorrupt: segment read: {e}"))?;
    if last[0] == b'\n' {
        return Ok(());
    }
    let mut end = length;
    let mut valid_len = 0_u64;
    let mut chunk = vec![0_u8; SCAN_BYTES];
    while end > 0 {
        let start = end.saturating_sub(SCAN_BYTES as u64);
        let count = (end - start) as usize;
        file.seek(SeekFrom::Start(start))
            .map_err(|e| format!("StorageCorrupt: segment tail scan: {e}"))?;
        file.read_exact(&mut chunk[..count])
            .map_err(|e| format!("StorageCorrupt: segment tail scan: {e}"))?;
        if let Some(index) = chunk[..count].iter().rposition(|byte| *byte == b'\n') {
            valid_len = start + index as u64 + 1;
            break;
        }
        end = start;
    }
    file.set_len(valid_len)
        .and_then(|()| file.sync_all())
        .map_err(|e| format!("StorageUnavailable: segment tail repair failed: {e}"))
}
fn write_new_synced(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("StorageUnavailable: {e}"))?;
    file.write_all(bytes).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())
}
fn write_replace_synced(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temporary = path.with_extension(format!(
        "{}.tmp",
        path.extension().and_then(|x| x.to_str()).unwrap_or("data")
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&temporary)
        .map_err(|e| e.to_string())?;
    file.write_all(bytes).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    drop(file);
    fs::rename(temporary, path).map_err(|e| e.to_string())
}
fn verified_artifact(directory: &Path, path: &Path) -> Result<ArtifactRecord, String> {
    let metadata = fs::metadata(path).map_err(|e| e.to_string())?;
    Ok(ArtifactRecord {
        path: path
            .strip_prefix(directory)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .into_owned(),
        sha256: hex(&sha256_file(path)?),
        bytes: metadata.len(),
    })
}
fn verify_artifact(directory: &Path, artifact: &ArtifactRecord) -> Result<(), String> {
    let path = directory.join(&artifact.path);
    let metadata =
        fs::metadata(&path).map_err(|e| format!("StorageCorrupt: {}: {e}", artifact.path))?;
    if metadata.len() != artifact.bytes || hex(&sha256_file(&path)?) != artifact.sha256 {
        return Err(format!(
            "StorageCorrupt: artifact verification failed: {}",
            artifact.path
        ));
    }
    Ok(())
}
fn verify_pcm_staging(
    directory: &Path,
    artifact: &ArtifactRecord,
    samples: u64,
    expected_pcm_sha256: &str,
) -> Result<(), String> {
    let expected_bytes = (PCM_STAGING_HEADER.len() as u64)
        .checked_add(
            samples
                .checked_mul(2)
                .ok_or_else(|| "StorageCorrupt: PCM sample byte count overflow".to_owned())?,
        )
        .ok_or_else(|| "StorageCorrupt: PCM staging byte count overflow".to_owned())?;
    if artifact.bytes != expected_bytes {
        return Err(
            "StorageCorrupt: PCM staging byte count does not match its sample range".into(),
        );
    }
    let path = directory.join(&artifact.path);
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut header = vec![0; PCM_STAGING_HEADER.len()];
    file.read_exact(&mut header)
        .map_err(|e| format!("StorageCorrupt: PCM staging header: {e}"))?;
    if header != PCM_STAGING_HEADER {
        return Err("StorageCorrupt: PCM staging version/header mismatch".into());
    }
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    if hex(&hash.finalize()) != expected_pcm_sha256 {
        return Err("StorageCorrupt: PCM payload SHA-256 mismatch".into());
    }
    Ok(())
}
pub fn sha256_file(path: &Path) -> Result<[u8; 32], String> {
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    Ok(hash.finalize().into())
}

fn source_output_samples(path: &Path) -> Result<u64, String> {
    let file = File::open(path).map_err(|e| format!("SourceMissing: {e}"))?;
    let source = MediaSourceStream::new(Box::new(file), Default::default());
    let hint = Hint::new();
    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            source,
            &FormatOptions {
                enable_gapless: true,
                ..Default::default()
            },
            &MetadataOptions::default(),
        )
        .map_err(|e| format!("UnsupportedFormat: audio probe failed: {e}"))?;
    let track = probed
        .format
        .default_track()
        .ok_or_else(|| "UnsupportedFormat: audio has no track".to_owned())?;
    if track.codec_params.codec == CODEC_TYPE_MP3
        && (track.codec_params.channels.is_none()
            || track.codec_params.sample_rate.is_none()
            || track.codec_params.n_frames.is_none())
    {
        return Err("UnsupportedFormat: MP3 profile or duration metadata is incomplete".into());
    }
    if track.codec_params.codec == CODEC_TYPE_MP3 {
        let sample_rate = track.codec_params.sample_rate.unwrap_or_default();
        let channels = track.codec_params.channels;
        let stereo = Channels::FRONT_LEFT | Channels::FRONT_RIGHT;
        let supported_layout = channels == Some(Channels::FRONT_LEFT) || channels == Some(stereo);
        if !matches!(
            sample_rate,
            8_000 | 11_025 | 12_000 | 16_000 | 22_050 | 24_000 | 32_000 | 44_100 | 48_000
        ) || !supported_layout
        {
            return Err(
                "UnsupportedFormat: MP3 sample rate or channel layout is outside Q-07".into(),
            );
        }
    }
    let frames = track
        .codec_params
        .n_frames
        .ok_or_else(|| "UnsupportedFormat: audio duration is unavailable".to_owned())?;
    let input_rate = u64::from(
        track
            .codec_params
            .sample_rate
            .filter(|rate| *rate > 0)
            .ok_or_else(|| "UnsupportedFormat: audio sample rate is unavailable".to_owned())?,
    );
    if frames == 0 {
        return Err("UnsupportedFormat: audio contains no samples".into());
    }
    frames
        .saturating_sub(1)
        .checked_mul(16_000)
        .map(|scaled| scaled / input_rate + 1)
        .ok_or_else(|| "UnsupportedFormat: audio duration exceeds supported range".into())
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
fn timestamp(sample: u64) -> String {
    let ms = sample.saturating_mul(1000) / 16_000;
    format!(
        "{:02}:{:02}:{:02},{:03}",
        ms / 3_600_000,
        (ms / 60_000) % 60,
        (ms / 1000) % 60,
        ms % 1000
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn segment_recovery_keeps_synced_prefix_and_drops_only_a_partial_tail() {
        let path = std::env::temp_dir().join(format!(
            "whisper-segments-{}-{}.jsonl",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let record = SegmentRecord {
            sequence: 1,
            range: SourceRange::new(0, 160, 16_000).unwrap(),
            text: "confirmed".into(),
        };
        let mut contents = serde_json::to_vec(&record).unwrap();
        contents.extend_from_slice(b"\n{\"sequence\":");
        fs::write(&path, contents).unwrap();

        assert_eq!(read_segments(&path).unwrap(), vec![record]);
        assert!(fs::read(&path).unwrap().ends_with(b"\n"));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn scan_preserves_healthy_entries_and_reports_each_corrupt_generation() {
        let root = std::env::temp_dir().join(format!(
            "whisper-archive-scan-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let healthy = root
            .join("transcriptions")
            .join(format!("{:032x}", 7_u128))
            .join("1");
        fs::create_dir_all(&healthy).unwrap();
        fs::write(healthy.join("transcript.txt"), b"healthy").unwrap();
        fs::write(
            healthy.join("transcript.srt"),
            b"1\n00:00:00,000 --> 00:00:00,010\nhealthy\n",
        )
        .unwrap();
        let text = verified_artifact(&healthy, &healthy.join("transcript.txt")).unwrap();
        let srt = verified_artifact(&healthy, &healthy.join("transcript.srt")).unwrap();
        let pending = PendingRecord {
            version: 1,
            job_id: JobId(7),
            generation: Generation::first(),
            source_path: "fixture.wav".into(),
            source_sha256: "a".repeat(64),
            source_samples: 160,
            model_path: "model.bin".into(),
            model_sha256: APPROVED_MODEL_SHA256.into(),
            model_size: APPROVED_MODEL_SIZE,
        };
        fs::write(
            healthy.join("pending.json"),
            serde_json::to_vec(&pending).unwrap(),
        )
        .unwrap();
        fs::write(
            healthy.join("segments.jsonl"),
            serde_json::to_vec(&SegmentRecord {
                sequence: 1,
                range: SourceRange::new(0, 160, 16_000).unwrap(),
                text: "healthy".into(),
            })
            .unwrap()
            .into_iter()
            .chain(std::iter::once(b'\n'))
            .collect::<Vec<_>>(),
        )
        .unwrap();
        let manifest = CompleteRecord {
            version: 1,
            job_id: JobId(7),
            generation: Generation::first(),
            source_sha256: "a".repeat(64),
            segments: 1,
            text,
            srt,
        };
        fs::write(
            healthy.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        fs::write(healthy.join("CURRENT"), b"manifest.json\n").unwrap();

        let bad_current = root
            .join("transcriptions")
            .join(format!("{:032x}", 11_u128))
            .join("1");
        fs::create_dir_all(&bad_current).unwrap();
        fs::copy(
            healthy.join("pending.json"),
            bad_current.join("pending.json"),
        )
        .unwrap();
        fs::copy(
            healthy.join("manifest.json"),
            bad_current.join("manifest.json"),
        )
        .unwrap();
        fs::copy(
            healthy.join("segments.jsonl"),
            bad_current.join("segments.jsonl"),
        )
        .unwrap();
        fs::copy(
            healthy.join("transcript.txt"),
            bad_current.join("transcript.txt"),
        )
        .unwrap();
        fs::copy(
            healthy.join("transcript.srt"),
            bad_current.join("transcript.srt"),
        )
        .unwrap();
        fs::write(bad_current.join("CURRENT"), b"../../other/manifest.json\n").unwrap();

        let transplanted = root
            .join("transcriptions")
            .join(format!("{:032x}", 12_u128))
            .join("3");
        fs::create_dir_all(&transplanted).unwrap();
        for name in [
            "pending.json",
            "manifest.json",
            "segments.jsonl",
            "transcript.txt",
            "transcript.srt",
            "CURRENT",
        ] {
            fs::copy(healthy.join(name), transplanted.join(name)).unwrap();
        }

        let bad_pending = root
            .join("transcriptions")
            .join(format!("{:032x}", 8_u128))
            .join("1");
        fs::create_dir_all(&bad_pending).unwrap();
        fs::write(bad_pending.join("pending.json"), b"{corrupt").unwrap();
        let bad_manifest = root
            .join("transcriptions")
            .join(format!("{:032x}", 9_u128))
            .join("2");
        fs::create_dir_all(&bad_manifest).unwrap();
        fs::write(bad_manifest.join("CURRENT"), b"manifest.json\n").unwrap();
        fs::write(bad_manifest.join("manifest.json"), b"{corrupt").unwrap();
        fs::write(
            root.join("transcriptions")
                .join(format!("{:032x}", 10_u128)),
            b"not a job directory",
        )
        .unwrap();
        fs::write(
            root.join("transcriptions").join("queue.jsonl"),
            b"durable queue journal",
        )
        .unwrap();

        let first = ArchiveStore::scan(&root).unwrap();
        let second = ArchiveStore::scan(&root).unwrap();
        assert_eq!(
            first, second,
            "corruption diagnostics are stable across scans"
        );
        assert!(
            !first.iter().any(|item| item.job_id == JobId(0)),
            "non-directory queue journal is not a synthetic corrupt job"
        );
        assert!(
            first
                .iter()
                .any(|item| item.job_id == JobId(7) && item.complete)
        );
        assert!(
            first
                .iter()
                .any(|item| item.job_id == JobId(8) && item.diagnostic.is_some())
        );
        assert!(
            first
                .iter()
                .any(|item| item.job_id == JobId(9) && item.diagnostic.is_some())
        );
        assert!(
            first
                .iter()
                .any(|item| item.job_id == JobId(10) && item.diagnostic.is_some())
        );
        for job_id in [JobId(11), JobId(12)] {
            assert!(
                first
                    .iter()
                    .any(|item| item.job_id == job_id && item.diagnostic.is_some())
            );
            assert!(
                !first
                    .iter()
                    .any(|item| item.job_id == job_id && item.complete)
            );
        }
    }

    #[test]
    fn published_running_row_is_removed_durably_and_recovery_scans_are_stable() {
        use crate::queue_store::{QueueStatus, QueueStore};
        use whisper_core::SegmentId;

        let root = std::env::temp_dir().join(format!(
            "whisper-publish-queue-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let request = ImportRequest {
            job_id: JobId(71),
            generation: Generation::first(),
            source_path: "source.wav".into(),
            source_sha256: Some([0; 32]),
            source_samples: Some(160),
            model_path: "model.bin".into(),
            model_sha256: [0; 32],
            destination: root.to_string_lossy().into_owned(),
            config: whisper_core::JobConfig {
                language: whisper_core::LanguageChoice::Manual("fr".into()),
                compute: whisper_core::ComputeChoice::Cpu,
            },
        };
        let mut other = request.clone();
        other.job_id = JobId(72);
        let mut queue = QueueStore::open(&root).unwrap();
        queue.enqueue(request.clone()).unwrap();
        queue.enqueue(other.clone()).unwrap();
        queue
            .set_status(request.job_id, request.generation, QueueStatus::Running)
            .unwrap();

        let directory = root
            .join("transcriptions")
            .join(format!("{:032x}", request.job_id.0))
            .join(request.generation.get().to_string());
        fs::create_dir_all(&directory).unwrap();
        let pending = PendingRecord {
            version: 1,
            job_id: request.job_id,
            generation: request.generation,
            source_path: request.source_path.clone(),
            source_sha256: hex(&[0; 32]),
            source_samples: 160,
            model_path: request.model_path.clone(),
            model_sha256: APPROVED_MODEL_SHA256.into(),
            model_size: APPROVED_MODEL_SIZE,
        };
        fs::write(
            directory.join("pending.json"),
            serde_json::to_vec(&pending).unwrap(),
        )
        .unwrap();
        let mut archive = ArchiveStore::new(&root);
        archive.current = Some(ActiveArchive {
            job_id: request.job_id,
            generation: request.generation,
            directory,
            source_sha256: [0; 32],
            source_samples: 160,
            expected_sequence: 1,
        });
        archive
            .persist_segment(&WorkerSegment {
                job_id: request.job_id,
                generation: request.generation,
                instance_id: 1,
                segment_id: SegmentId(1),
                range: SourceRange::new(0, 160, 16_000).unwrap(),
                text: "confirmed".into(),
            })
            .unwrap();
        archive
            .publish(request.job_id, request.generation, 1)
            .unwrap();

        let mut reopened = QueueStore::open(&root).unwrap();
        reopened.remove(request.job_id, request.generation).unwrap();
        drop(reopened);
        let first = crate::recovery::scan(&root).unwrap();
        let second = crate::recovery::scan(&root).unwrap();
        assert_eq!(
            first, second,
            "recovery is stable after publication cleanup"
        );
        assert_eq!(first.queue.len(), 1);
        assert_eq!(first.queue[0].request.job_id, other.job_id);
        assert!(ArchiveStore::scan(&root).unwrap().iter().any(|entry| {
            entry.job_id == request.job_id
                && entry.generation == request.generation
                && entry.complete
        }));
        assert!(
            !QueueStore::open(&root)
                .unwrap()
                .entries()
                .iter()
                .any(|entry| entry.request.job_id == request.job_id)
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn resumed_prefix_copy_retries_after_pending_creation_without_duplication() {
        use whisper_core::SegmentId;

        let root = std::env::temp_dir().join(format!(
            "whisper-resume-copy-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let request = ImportRequest {
            job_id: JobId(81),
            generation: Generation::first().next().unwrap(),
            source_path: "source.wav".into(),
            source_sha256: Some([0; 32]),
            source_samples: Some(320),
            model_path: "model.bin".into(),
            model_sha256: [0; 32],
            destination: root.to_string_lossy().into_owned(),
            config: whisper_core::JobConfig {
                language: whisper_core::LanguageChoice::Manual("fr".into()),
                compute: whisper_core::ComputeChoice::Cpu,
            },
        };
        let target = root
            .join("transcriptions")
            .join(format!("{:032x}", request.job_id.0))
            .join(request.generation.get().to_string());
        fs::create_dir_all(&target).unwrap();
        let pending = PendingRecord {
            version: 1,
            job_id: request.job_id,
            generation: request.generation,
            source_path: request.source_path.clone(),
            source_sha256: hex(&[0; 32]),
            source_samples: 320,
            model_path: request.model_path.clone(),
            model_sha256: APPROVED_MODEL_SHA256.into(),
            model_size: APPROVED_MODEL_SIZE,
        };
        assert_eq!(initialize_pending(&target, &pending).unwrap(), 1);
        let prefix = vec![
            SegmentRecord {
                sequence: 1,
                range: SourceRange::new(0, 160, 16_000).unwrap(),
                text: "one".into(),
            },
            SegmentRecord {
                sequence: 2,
                range: SourceRange::new(160, 320, 16_000).unwrap(),
                text: "two".into(),
            },
        ];
        let mut interrupted_archive = ArchiveStore::new(&root);
        interrupted_archive.current = Some(ActiveArchive {
            job_id: request.job_id,
            generation: request.generation,
            directory: target.clone(),
            source_sha256: [0; 32],
            source_samples: 320,
            expected_sequence: 1,
        });
        interrupted_archive
            .persist_segment(&WorkerSegment {
                job_id: request.job_id,
                generation: request.generation,
                instance_id: 0,
                segment_id: SegmentId(1),
                range: prefix[0].range,
                text: prefix[0].text.clone(),
            })
            .unwrap();

        let mut retried_archive = ArchiveStore::new(&root);
        let expected_sequence = initialize_pending(&target, &pending).unwrap();
        assert_eq!(
            expected_sequence, 2,
            "reopened pending preserves the partial prefix"
        );
        retried_archive.current = Some(ActiveArchive {
            job_id: request.job_id,
            generation: request.generation,
            directory: target.clone(),
            source_sha256: [0; 32],
            source_samples: 320,
            expected_sequence,
        });
        retried_archive
            .copy_resume_prefix(&request, &prefix)
            .unwrap();
        let mut second_retry = ArchiveStore::new(&root);
        let expected_sequence = initialize_pending(&target, &pending).unwrap();
        assert_eq!(
            expected_sequence, 3,
            "retry sees the complete copied prefix"
        );
        second_retry.current = Some(ActiveArchive {
            job_id: request.job_id,
            generation: request.generation,
            directory: target.clone(),
            source_sha256: [0; 32],
            source_samples: 320,
            expected_sequence,
        });
        second_retry.copy_resume_prefix(&request, &prefix).unwrap();
        assert_eq!(
            read_segments(&target.join("segments.jsonl")).unwrap(),
            prefix
        );
        fs::remove_dir_all(root).unwrap();
    }
}
