use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
};
use whisper_core::{Generation, JobId, ports::ImportRequest};

const QUEUE_SCHEMA: u32 = 1;

pub use whisper_core::ports::{ImportQueueEntry as QueueEntry, ImportQueueStatus as QueueStatus};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
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
        if self.entries.values().any(|entry| {
            entry.request.job_id == request.job_id && entry.request.generation == request.generation
        }) {
            return Err("InvalidInput: job generation is already queued".into());
        }
        let sequence = self.next_sequence;
        self.next_sequence = sequence
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

    pub fn remove(&mut self, job_id: JobId, generation: Generation) -> Result<(), String> {
        let Some(sequence) = self.entries.iter().find_map(|(sequence, entry)| {
            (entry.request.job_id == job_id && entry.request.generation == generation)
                .then_some(*sequence)
        }) else {
            return Err("InvalidInput: queued job was not found".into());
        };
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
        for (line_no, line) in BufReader::new(File::open(&self.path).map_err(|e| e.to_string())?)
            .lines()
            .enumerate()
        {
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
