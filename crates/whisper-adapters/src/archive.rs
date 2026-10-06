use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{BufRead, Read, Write},
    path::{Path, PathBuf},
};
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
        write_new_synced(
            &directory.join("pending.json"),
            &serde_json::to_vec(&pending).map_err(|e| e.to_string())?,
        )?;
        request.source_sha256 = Some(source_sha);
        request.source_samples = Some(source_samples);
        self.current = Some(ActiveArchive {
            job_id: request.job_id,
            generation: request.generation,
            directory,
            source_sha256: source_sha,
            source_samples,
            expected_sequence: 1,
        });
        Ok(request)
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
    let file = File::open(path).map_err(|e| e.to_string())?;
    std::io::BufReader::new(file)
        .lines()
        .map(|line| {
            let line = line.map_err(|e| e.to_string())?;
            serde_json::from_str(&line).map_err(|e| format!("StorageCorrupt: segment: {e}"))
        })
        .collect()
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
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| format!("UnsupportedFormat: audio probe failed: {e}"))?;
    let track = probed
        .format
        .default_track()
        .ok_or_else(|| "UnsupportedFormat: audio has no track".to_owned())?;
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

        let first = ArchiveStore::scan(&root).unwrap();
        let second = ArchiveStore::scan(&root).unwrap();
        assert_eq!(
            first, second,
            "corruption diagnostics are stable across scans"
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
}
