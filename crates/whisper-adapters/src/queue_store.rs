use serde::{Deserialize, Serialize};
use serde::{Deserializer, Serializer};
use std::{
    collections::BTreeMap,
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};
use whisper_core::{ComputeChoice, Generation, JobId, ports::ImportRequest};

const QUEUE_LEGACY_SCHEMA: u32 = 1;
const QUEUE_UNIFIED_SCHEMA: u32 = 2;

fn queue_schema(request: &ImportRequest) -> u32 {
    match request.strategy {
        whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1 => QUEUE_LEGACY_SCHEMA,
        whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1 => {
            QUEUE_UNIFIED_SCHEMA
        }
    }
}

fn valid_queue_upsert(version: u32, entry: &QueueEntry) -> bool {
    version == queue_schema(&entry.request) && entry.sequence > 0
}

pub use whisper_core::ports::{ImportQueueEntry as QueueEntry, ImportQueueStatus as QueueStatus};

#[derive(Clone, Debug)]
enum QueueRecord {
    Upsert {
        version: u32,
        entry: Box<QueueEntry>,
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
                entry: Some((**entry).clone()),
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
                    entry: Box::new(entry),
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
    /// Reads the durable queue without creating directories, opening it for
    /// repair, or changing an incomplete tail. Recovery checks must fail closed
    /// on an unreadable journal rather than making it writable as a side effect.
    pub fn inspect_read_only(destination: impl AsRef<Path>) -> Result<Vec<QueueEntry>, String> {
        let path = destination
            .as_ref()
            .join("transcriptions")
            .join("queue.jsonl");
        let file = match File::open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(format!("StorageCorrupt: queue inspection: {error}")),
        };
        let mut entries = BTreeMap::<u64, QueueEntry>::new();
        for (line_no, line) in BufReader::new(file).lines().enumerate() {
            let line = line.map_err(|error| {
                format!(
                    "StorageCorrupt: queue inspection line {}: {error}",
                    line_no + 1
                )
            })?;
            let record: QueueRecord = serde_json::from_str(&line).map_err(|error| {
                format!(
                    "StorageCorrupt: queue inspection line {}: {error}",
                    line_no + 1
                )
            })?;
            match record {
                QueueRecord::Upsert { version, entry } if valid_queue_upsert(version, &entry) => {
                    entries.insert(entry.sequence, *entry);
                }
                QueueRecord::Remove {
                    version,
                    job_id,
                    generation,
                } if version == QUEUE_LEGACY_SCHEMA || version == QUEUE_UNIFIED_SCHEMA => entries
                    .retain(|_, entry| {
                        entry.request.job_id != job_id || entry.request.generation != generation
                    }),
                _ => {
                    return Err(format!(
                        "StorageCorrupt: unsupported queue record at line {}",
                        line_no + 1
                    ));
                }
            }
        }
        Ok(entries.into_values().collect())
    }

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

    /// Resolves a durable Turbo queue record to the current verified catalog path in memory.
    pub fn resolve_queued_with_catalog(
        &self,
        job_id: JobId,
        generation: Generation,
        catalog: &crate::model_catalog::CatalogSnapshot,
        known_historical_turbo_path: &str,
    ) -> Result<ImportRequest, String> {
        let first = self
            .entries
            .first_key_value()
            .map(|(_, entry)| (entry.request.job_id, entry.request.generation));
        if first != Some((job_id, generation)) {
            return Err("InvalidInput: process queued jobs in FIFO order".into());
        }
        let entry = self
            .entries
            .values()
            .find(|entry| entry.request.job_id == job_id && entry.request.generation == generation)
            .ok_or_else(|| "InvalidInput: queue entry no longer exists".to_owned())?;
        if !matches!(
            entry.status,
            QueueStatus::Queued | QueueStatus::AwaitingChoice
        ) {
            return Err(
                "InvalidInput: interrupted jobs require an explicit verified resume".into(),
            );
        }
        catalog.bind_turbo_import_request(&entry.request, known_historical_turbo_path)
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
            version: queue_schema(&entry.request),
            entry: Box::new(entry.clone()),
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
            version: queue_schema(&entry.request),
            entry: Box::new(entry.clone()),
        })?;
        self.entries.insert(entry.sequence, entry.clone());
        Ok(entry)
    }

    /// Changes only the calculation choice of a still-pending durable entry.
    /// `expected` must be the complete entry request observed by the caller so
    /// an intervening identity change cannot be overwritten.
    pub(crate) fn set_pending_compute(
        &mut self,
        expected: &ImportRequest,
        compute: ComputeChoice,
    ) -> Result<QueueEntry, String> {
        let mut entry = self
            .entries
            .values()
            .find(|entry| {
                entry.request.job_id == expected.job_id
                    && entry.request.generation == expected.generation
            })
            .cloned()
            .ok_or_else(|| "InvalidInput: queued job was not found".to_owned())?;
        if !matches!(
            entry.status,
            QueueStatus::Queued | QueueStatus::AwaitingChoice
        ) {
            return Err("InvalidInput: calculation can change only before processing".into());
        }
        if entry.request != *expected {
            return Err("SourceChanged: queued request identity changed before processing".into());
        }
        if self
            .entries
            .first_key_value()
            .map(|(_, first)| (first.request.job_id, first.request.generation))
            != Some((expected.job_id, expected.generation))
        {
            return Err("InvalidInput: process queued jobs in FIFO order".into());
        }
        if entry.request.config.compute == compute {
            return Ok(entry);
        }

        entry.request.config.compute = compute;
        self.append(QueueRecord::Upsert {
            version: queue_schema(&entry.request),
            entry: Box::new(entry.clone()),
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
            version: queue_schema(&entry.request),
            entry: Box::new(entry.clone()),
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

    /// Validates a resumed request against the durable path identity without rewriting the
    /// stored historical path. The returned worker request must be rebound separately through
    /// the verified catalog.
    pub fn validate_resume_with_catalog(
        &self,
        previous_job_id: JobId,
        previous_generation: Generation,
        request: &ImportRequest,
        catalog: &crate::model_catalog::CatalogSnapshot,
        known_historical_turbo_path: &str,
    ) -> Result<(), String> {
        let entry = self
            .entries
            .values()
            .find(|entry| {
                entry.request.job_id == previous_job_id
                    && entry.request.generation == previous_generation
            })
            .ok_or_else(|| "InvalidInput: interrupted queue entry was not found".to_owned())?;
        let previous_model = catalog
            .resolve_model_identity(&entry.request.model_path, known_historical_turbo_path)?;
        let requested_model =
            catalog.resolve_model_identity(&request.model_path, known_historical_turbo_path)?;
        if previous_model != whisper_core::model_catalog::ModelId::WhisperLargeV3Turbo
            || requested_model != whisper_core::model_catalog::ModelId::WhisperLargeV3Turbo
            || previous_model != requested_model
            || entry.request.model_sha256 != request.model_sha256
        {
            return Err(
                "ModelIdentityChanged: la reprise ne correspond pas au modèle Turbo durable".into(),
            );
        }
        let mut comparable = request.clone();
        comparable.model_path = entry.request.model_path.clone();
        self.validate_resume(previous_job_id, previous_generation, &comparable)
    }

    pub fn resume_interrupted_with_catalog(
        &mut self,
        previous_job_id: JobId,
        previous_generation: Generation,
        mut request: ImportRequest,
        catalog: &crate::model_catalog::CatalogSnapshot,
        known_historical_turbo_path: &str,
    ) -> Result<QueueEntry, String> {
        self.validate_resume_with_catalog(
            previous_job_id,
            previous_generation,
            &request,
            catalog,
            known_historical_turbo_path,
        )?;
        let durable_path = self
            .entries
            .values()
            .find(|entry| {
                entry.request.job_id == previous_job_id
                    && entry.request.generation == previous_generation
            })
            .map(|entry| entry.request.model_path.clone())
            .ok_or_else(|| "InvalidInput: interrupted queue entry was not found".to_owned())?;
        request.model_path = durable_path;
        self.resume_interrupted(previous_job_id, previous_generation, request)
    }

    pub fn remove(&mut self, job_id: JobId, generation: Generation) -> Result<(), String> {
        let Some((sequence, status, strategy)) =
            self.entries.iter().find_map(|(sequence, entry)| {
                (entry.request.job_id == job_id && entry.request.generation == generation)
                    .then_some((*sequence, entry.status, entry.request.strategy))
            })
        else {
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
            version: match strategy {
                whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1 => {
                    QUEUE_LEGACY_SCHEMA
                }
                whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1 => {
                    QUEUE_UNIFIED_SCHEMA
                }
            },
            job_id,
            generation,
        })?;
        self.entries.remove(&sequence);
        if status != QueueStatus::Running {
            if let Some(destination) = self.path.parent().and_then(Path::parent) {
                crate::archive::ArchiveStore::clear_duplicate_reservation(
                    destination,
                    job_id,
                    generation,
                )?;
            }
        }
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
                QueueRecord::Upsert { version, entry } if valid_queue_upsert(version, &entry) => {
                    self.next_sequence = self.next_sequence.max(entry.sequence.saturating_add(1));
                    self.entries.insert(entry.sequence, *entry);
                }
                QueueRecord::Remove {
                    version,
                    job_id,
                    generation,
                } if version == QUEUE_LEGACY_SCHEMA || version == QUEUE_UNIFIED_SCHEMA => {
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

#[cfg(test)]
mod tests {
    use super::*;
    use whisper_core::{JobConfig, ports::ImportRequest};

    fn root() -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        std::env::temp_dir()
            .join("r10-queue-store-tests")
            .join(format!("{}-{nonce}", std::process::id()))
    }

    fn request(job_id: u128, destination: &Path) -> ImportRequest {
        ImportRequest {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
            job_id: JobId(job_id),
            generation: Generation::first(),
            source_path: format!("source-{job_id}.wav"),
            source_sha256: Some([job_id as u8; 32]),
            source_samples: Some(16_000),
            model_path: "parakeet.gguf".into(),
            model_sha256: [9; 32],
            destination: destination.to_string_lossy().into_owned(),
            config: JobConfig {
                language: whisper_core::LanguageChoice::Manual("fr".into()),
                compute: ComputeChoice::Gpu,
            },
        }
    }

    #[test]
    fn pending_compute_change_persists_only_the_choice_and_keeps_fifo() {
        let root = root();
        let mut queue = QueueStore::open(&root).unwrap();
        let first = request(1, &root);
        let second = request(2, &root);
        let first_entry = queue.enqueue(first.clone()).unwrap();
        queue.enqueue(second.clone()).unwrap();

        let changed = queue
            .set_pending_compute(&first, ComputeChoice::Cpu)
            .unwrap();

        let mut expected = first;
        expected.config.compute = ComputeChoice::Cpu;
        assert_eq!(changed.request, expected);
        assert_eq!(changed.sequence, first_entry.sequence);
        assert_eq!(
            queue
                .entries()
                .iter()
                .map(|entry| entry.request.job_id)
                .collect::<Vec<_>>(),
            [JobId(1), JobId(2)]
        );
        let reopened = QueueStore::open(&root).unwrap();
        assert_eq!(reopened.entries()[0].request, expected);
        assert_eq!(reopened.entries()[1].request, second);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pending_compute_change_rejects_identity_or_status_changes() {
        let root = root();
        let mut queue = QueueStore::open(&root).unwrap();
        let first = request(3, &root);
        queue.enqueue(first.clone()).unwrap();

        let mut altered = first.clone();
        altered.source_path.push_str("-changed");
        assert!(
            queue
                .set_pending_compute(&altered, ComputeChoice::Cpu)
                .is_err()
        );
        assert_eq!(queue.entries()[0].request, first);

        queue
            .set_status(first.job_id, first.generation, QueueStatus::Interrupted)
            .unwrap();
        assert!(
            queue
                .set_pending_compute(&first, ComputeChoice::Cpu)
                .unwrap_err()
                .contains("only before processing")
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn pending_compute_change_cannot_skip_the_fifo_head() {
        let root = root();
        let mut queue = QueueStore::open(&root).unwrap();
        let first = request(4, &root);
        let second = request(5, &root);
        queue.enqueue(first.clone()).unwrap();
        queue.enqueue(second.clone()).unwrap();

        assert!(
            queue
                .set_pending_compute(&second, ComputeChoice::Cpu)
                .unwrap_err()
                .contains("FIFO order")
        );
        assert_eq!(queue.entries()[1].request, second);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn recovery_inspection_is_read_only_for_missing_and_corrupt_queues() {
        let destination = root();
        assert_eq!(QueueStore::inspect_read_only(&destination).unwrap(), []);
        assert!(!destination.join("transcriptions").exists());

        let queue_path = destination.join("transcriptions/queue.jsonl");
        std::fs::create_dir_all(queue_path.parent().unwrap()).unwrap();
        let corrupt = b"{\"op\":\"upsert\",\"version\":1";
        std::fs::write(&queue_path, corrupt).unwrap();
        let before = std::fs::metadata(&queue_path).unwrap().modified().unwrap();
        assert!(
            QueueStore::inspect_read_only(&destination)
                .unwrap_err()
                .contains("StorageCorrupt: queue inspection")
        );
        assert_eq!(std::fs::read(&queue_path).unwrap(), corrupt);
        assert_eq!(
            std::fs::metadata(&queue_path).unwrap().modified().unwrap(),
            before
        );
        std::fs::remove_dir_all(destination).unwrap();
    }

    #[test]
    fn unified_queue_schema_round_trips_strategy_and_rejects_legacy_schema_mismatch() {
        let destination = root();
        let mut queue = QueueStore::open(&destination).unwrap();
        let mut request = request(88, &destination);
        request.strategy = whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1;
        queue.enqueue(request.clone()).unwrap();
        let journal =
            std::fs::read_to_string(destination.join("transcriptions/queue.jsonl")).unwrap();
        let record: serde_json::Value =
            serde_json::from_str(journal.lines().next().unwrap()).unwrap();
        assert_eq!(record["version"], 2);
        let mut reopened = QueueStore::open(&destination).unwrap();
        assert_eq!(reopened.entries()[0].request.strategy, request.strategy);
        reopened
            .set_status(request.job_id, request.generation, QueueStatus::Interrupted)
            .unwrap();
        let mut substituted = request.clone();
        substituted.generation = request.generation.next().unwrap();
        substituted.strategy = whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1;
        assert!(
            reopened
                .validate_resume(request.job_id, request.generation, &substituted)
                .is_err()
        );

        let mut invalid = record;
        invalid["version"] = serde_json::Value::from(1);
        std::fs::write(
            destination.join("transcriptions/queue.jsonl"),
            format!("{}\n", serde_json::to_string(&invalid).unwrap()),
        )
        .unwrap();
        assert!(
            QueueStore::open(&destination)
                .err()
                .unwrap()
                .contains("unsupported queue record")
        );
        std::fs::remove_dir_all(destination).unwrap();
    }

    #[test]
    fn historical_queue_json_without_strategy_reads_as_legacy_without_rewriting() {
        let destination = root();
        let path = destination.join("transcriptions/queue.jsonl");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut legacy = serde_json::to_value(QueueRecord::Upsert {
            version: QUEUE_LEGACY_SCHEMA,
            entry: Box::new(QueueEntry {
                sequence: 1,
                request: request(89, &destination),
                status: QueueStatus::Queued,
            }),
        })
        .unwrap();
        legacy["entry"]["request"]
            .as_object_mut()
            .unwrap()
            .remove("strategy");
        let bytes = format!("{}\n", serde_json::to_string(&legacy).unwrap());
        std::fs::write(&path, &bytes).unwrap();

        let entries = QueueStore::inspect_read_only(&destination).unwrap();
        assert_eq!(
            entries[0].request.strategy,
            whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), bytes);
        std::fs::remove_dir_all(destination).unwrap();
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
