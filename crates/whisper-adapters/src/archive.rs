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
    ports::{ImportRequest, WorkerSegment},
};

const APPROVED_MODEL_SIZE: u64 = 1_624_555_275;
const APPROVED_MODEL_SHA256: &str =
    "1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69";

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
