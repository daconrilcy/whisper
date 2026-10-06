use serde::{Deserialize, Serialize};
use serde::{Deserializer, Serializer};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};
use whisper_core::{Generation, JobId, ports::ImportRequest};

const QUEUE_SCHEMA: u32 = 1;

pub use whisper_core::ports::{ImportQueueEntry as QueueEntry, ImportQueueStatus as QueueStatus};

#[derive(Clone, Debug)]
enum QueueRecord {
    Upsert {
        version: u32,
        entry: QueueEntry,
    },
    Remove {
        version: u32,
        job_id: JobId,
        generation: Generation,
    },
}

#[derive(Serialize, Deserialize)]
struct QueueRecordWire {
    op: String,
    version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    entry: Option<QueueEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    job_id: Option<JobId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    generation: Option<Generation>,
}

impl Serialize for QueueRecord {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let wire = match self {
            Self::Upsert { version, entry } => QueueRecordWire {
                op: "upsert".into(),
                version: *version,
                entry: Some(entry.clone()),
                job_id: None,
                generation: None,
            },
            Self::Remove {
                version,
                job_id,
                generation,
            } => QueueRecordWire {
                op: "remove".into(),
                version: *version,
                entry: None,
                job_id: Some(*job_id),
                generation: Some(*generation),
            },
        };
        wire.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for QueueRecord {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = QueueRecordWire::deserialize(deserializer)?;
        match wire.op.as_str() {
            "upsert" => wire
                .entry
                .map(|entry| Self::Upsert {
                    version: wire.version,
                    entry,
                })
                .ok_or_else(|| serde::de::Error::missing_field("entry")),
            "remove" => match (wire.job_id, wire.generation) {
                (Some(job_id), Some(generation)) => Ok(Self::Remove {
                    version: wire.version,
                    job_id,
                    generation,
                }),
                _ => Err(serde::de::Error::missing_field("job_id/generation")),
            },
            _ => Err(serde::de::Error::unknown_variant(
                &wire.op,
                &["upsert", "remove"],
            )),
        }
    }
}

/// Append-only durable FIFO. A successful mutation means the journal was synced.
pub struct QueueStore {
    path: PathBuf,
    entries: BTreeMap<u64, QueueEntry>,
    next_sequence: u64,
}

impl QueueStore {
    pub fn open(destination: impl AsRef<Path>) -> Result<Self, String> {
        let directory = destination.as_ref().join("transcriptions");
        fs::create_dir_all(&directory).map_err(|e| format!("StorageUnavailable: {e}"))?;
        let path = directory.join("queue.jsonl");
        let mut store = Self {
            path,
            entries: BTreeMap::new(),
            next_sequence: 1,
        };
        store.replay()?;
        Ok(store)
    }

    pub fn entries(&self) -> Vec<QueueEntry> {
        self.entries.values().cloned().collect()
    }

    pub fn enqueue(&mut self, request: ImportRequest) -> Result<QueueEntry, String> {
        if request.source_sha256.is_none()
            || request.source_samples.is_none_or(|samples| samples == 0)
        {
            return Err("InvalidInput: queue admission requires a verified source identity".into());
        }
        if self.entries.values().any(|entry| {
            entry.request.job_id == request.job_id && entry.request.generation == request.generation
        }) {
            return Err("InvalidInput: job generation is already queued".into());
        }
        let sequence = self.next_sequence;
        let next_sequence = sequence
            .checked_add(1)
            .ok_or_else(|| "StorageUnavailable: queue sequence exhausted".to_owned())?;
        let entry = QueueEntry {
            sequence,
            request,
            status: QueueStatus::Queued,
        };
        self.append(QueueRecord::Upsert {
            version: QUEUE_SCHEMA,
            entry: entry.clone(),
        })?;
        self.next_sequence = next_sequence;
        self.entries.insert(sequence, entry.clone());
        Ok(entry)
    }

    pub fn set_status(
        &mut self,
        job_id: JobId,
        generation: Generation,
        status: QueueStatus,
    ) -> Result<QueueEntry, String> {
        let mut entry = self
            .entries
            .values()
            .find(|entry| entry.request.job_id == job_id && entry.request.generation == generation)
            .cloned()
            .ok_or_else(|| "InvalidInput: queued job was not found".to_owned())?;
        entry.status = status;
        self.append(QueueRecord::Upsert {
            version: QUEUE_SCHEMA,
            entry: entry.clone(),
        })?;
        self.entries.insert(entry.sequence, entry.clone());
        Ok(entry)
    }

    pub fn resume_interrupted(
        &mut self,
        previous_job_id: JobId,
        previous_generation: Generation,
        request: ImportRequest,
    ) -> Result<QueueEntry, String> {
        self.validate_resume(previous_job_id, previous_generation, &request)?;
        let mut entry = self
            .entries
            .values()
            .find(|entry| {
                entry.request.job_id == previous_job_id
                    && entry.request.generation == previous_generation
            })
            .cloned()
            .ok_or_else(|| "InvalidInput: interrupted queue entry was not found".to_owned())?;
        entry.request = request;
        entry.status = QueueStatus::Running;
        self.append(QueueRecord::Upsert {
            version: QUEUE_SCHEMA,
            entry: entry.clone(),
        })?;
        self.entries.insert(entry.sequence, entry.clone());
        Ok(entry)
    }

    pub fn validate_resume(
        &self,
        previous_job_id: JobId,
        previous_generation: Generation,
        request: &ImportRequest,
    ) -> Result<(), String> {
        let entry = self
            .entries
            .values()
            .find(|entry| {
                entry.request.job_id == previous_job_id
                    && entry.request.generation == previous_generation
            })
            .ok_or_else(|| "InvalidInput: interrupted queue entry was not found".to_owned())?;
        if entry.status != QueueStatus::Interrupted
            || request.job_id != previous_job_id
            || previous_generation.next() != Some(request.generation)
        {
            return Err("InvalidInput: interrupted queue identity cannot be resumed".into());
        }
        if self
            .entries
            .first_key_value()
            .map(|(_, first)| (first.request.job_id, first.request.generation))
            != Some((previous_job_id, previous_generation))
        {
            return Err("InvalidInput: resume queued jobs in FIFO order".into());
        }
        let mut expected = entry.request.clone();
        expected.generation = request.generation;
        if expected != *request {
            return Err(
                "SourceChanged: resume request differs from the durable queue entry".into(),
            );
        }
        if self
            .entries
            .values()
            .any(|other| other.sequence != entry.sequence && other.request.job_id == request.job_id)
        {
            return Err("InvalidInput: job identity is already present in the queue".into());
        }
        Ok(())
    }

    pub fn remove(&mut self, job_id: JobId, generation: Generation) -> Result<(), String> {
        let Some((sequence, status)) = self.entries.iter().find_map(|(sequence, entry)| {
            (entry.request.job_id == job_id && entry.request.generation == generation)
                .then_some((*sequence, entry.status))
        }) else {
            return Err("InvalidInput: queued job was not found".into());
        };
        if status == QueueStatus::Running {
            let destination = self
                .path
                .parent()
                .and_then(Path::parent)
                .ok_or_else(|| "StorageCorrupt: queue path has no destination".to_owned())?;
            let published =
                crate::archive::ArchiveStore::scan(destination)?
                    .iter()
                    .any(|archive| {
                        archive.complete
                            && archive.job_id == job_id
                            && archive.generation == generation
                    });
            if !published {
                return Err("Busy: a running import cannot be removed from the queue".into());
            }
        }
        self.append(QueueRecord::Remove {
            version: QUEUE_SCHEMA,
            job_id,
            generation,
        })?;
        self.entries.remove(&sequence);
        Ok(())
    }

    fn append(&self, record: QueueRecord) -> Result<(), String> {
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .map_err(|e| format!("StorageUnavailable: {e}"))?;
        serde_json::to_writer(&mut file, &record)
            .map_err(|e| format!("StorageUnavailable: {e}"))?;
        file.write_all(b"\n")
            .map_err(|e| format!("StorageUnavailable: {e}"))?;
        file.sync_all()
            .map_err(|e| format!("StorageUnavailable: queue sync failed: {e}"))
    }

    fn replay(&mut self) -> Result<(), String> {
        if !self.path.exists() {
            return Ok(());
        }
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.path)
            .map_err(|e| format!("StorageCorrupt: queue: {e}"))?;
        repair_uncommitted_tail(&mut file)?;
        file.seek(SeekFrom::Start(0))
            .map_err(|e| format!("StorageCorrupt: queue seek: {e}"))?;
        for (line_no, line) in BufReader::new(file).lines().enumerate() {
            let line =
                line.map_err(|e| format!("StorageCorrupt: queue line {}: {e}", line_no + 1))?;
            let record: QueueRecord = serde_json::from_str(&line)
                .map_err(|e| format!("StorageCorrupt: queue line {}: {e}", line_no + 1))?;
            match record {
                QueueRecord::Upsert { version, entry }
                    if version == QUEUE_SCHEMA && entry.sequence > 0 =>
                {
                    self.next_sequence = self.next_sequence.max(entry.sequence.saturating_add(1));
                    self.entries.insert(entry.sequence, entry);
                }
                QueueRecord::Remove {
                    version,
                    job_id,
                    generation,
                } if version == QUEUE_SCHEMA => {
                    self.entries.retain(|_, entry| {
                        entry.request.job_id != job_id || entry.request.generation != generation
                    });
                }
                _ => {
                    return Err(format!(
                        "StorageCorrupt: unsupported queue record at line {}",
                        line_no + 1
                    ));
                }
            }
        }
        Ok(())
    }
}

fn repair_uncommitted_tail(file: &mut File) -> Result<(), String> {
    const SCAN_BYTES: usize = 8 * 1024;
    let length = file
        .metadata()
        .map_err(|e| format!("StorageCorrupt: queue metadata: {e}"))?
        .len();
    if length == 0 {
        return Ok(());
    }
    file.seek(SeekFrom::End(-1))
        .map_err(|e| format!("StorageCorrupt: queue seek: {e}"))?;
    let mut last = [0_u8; 1];
    file.read_exact(&mut last)
        .map_err(|e| format!("StorageCorrupt: queue read: {e}"))?;
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
            .map_err(|e| format!("StorageCorrupt: queue tail scan: {e}"))?;
        file.read_exact(&mut chunk[..count])
            .map_err(|e| format!("StorageCorrupt: queue tail scan: {e}"))?;
        if let Some(index) = chunk[..count].iter().rposition(|byte| *byte == b'\n') {
            valid_len = start + index as u64 + 1;
            break;
        }
        end = start;
    }
    file.set_len(valid_len)
        .and_then(|()| file.sync_all())
        .map_err(|e| format!("StorageUnavailable: queue tail repair failed: {e}"))
}
