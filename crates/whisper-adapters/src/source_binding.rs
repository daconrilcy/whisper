//! Durable snapshots binding imports to the exact bytes sent to a worker.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use whisper_core::{Generation, JobId, ports::ImportRequest};

const SNAPSHOT_DIRECTORY: &str = ".whisper-source-snapshots";
const REGISTRY_FILE: &str = "reprocessing-snapshots.json";
const REGISTRY_LOCK_FILE: &str = "reprocessing-snapshots.lock";
const MAX_RETAINED_SNAPSHOTS: usize = 2;
const COPY_BUFFER_SIZE: usize = 1024 * 1024;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct SnapshotRecord {
    version: u8,
    job_id: JobId,
    generation: Generation,
    destination: String,
    original_path: String,
    snapshot_path: String,
    source_len: u64,
    source_sha256: [u8; 32],
    source_samples: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct RegistryEntry {
    job_id: JobId,
    generation: Generation,
    destination: String,
    status: RegistryStatus,
    snapshot_path: Option<String>,
}

struct Reservation {
    status: RegistryStatus,
    created: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
enum RegistryStatus {
    Reserved,
    Bound,
}

/// Owns a durable snapshot and a sharing handle that prevents mutation while consumed.
pub struct SourceBinding {
    registry_root: PathBuf,
    directory: PathBuf,
    record_path: PathBuf,
    snapshot_path: PathBuf,
    record: SnapshotRecord,
    snapshot_guard: Option<File>,
}

impl SourceBinding {
    /// Reserves a global slot durably before copying. The registry root must be
    /// supplied by the application composition; there is deliberately no fallback.
    pub fn acquire(request: &ImportRequest, registry_root: &Path) -> Result<Self, String> {
        let destination = ensure_destination(&request.destination)?;
        let registry = SnapshotRegistry::new(registry_root);
        let reservation = registry.reserve(request, &destination)?;

        let directory = destination.join(SNAPSHOT_DIRECTORY);
        if let Err(error) = fs::create_dir_all(&directory) {
            return Err(registry
                .release(request.job_id, request.generation, &destination)
                .err()
                .map_or_else(
                    || format!("StorageUnavailable: snapshot directory: {error}"),
                    |release| format!("StorageUnavailable: snapshot directory: {error}; {release}"),
                ));
        }
        let stem = snapshot_stem(request.job_id, request.generation);
        let record_path = directory.join(format!("{stem}.json"));
        let snapshot_path = directory.join(format!("{stem}.snapshot"));
        let partial_path = directory.join(format!("{stem}.snapshot.partial"));
        if partial_path.exists() {
            return Err("SourceSnapshotOrphaned: partial bytes remain for a reserved operation; slot retained".into());
        }
        if record_path.exists() || snapshot_path.exists() {
            if !record_path.exists() || !snapshot_path.exists() {
                return Err(
                    "SourceSnapshotOrphaned: incomplete durable binding; slot retained".into(),
                );
            }
            let binding = Self::open_existing(
                registry_root,
                request,
                destination,
                directory,
                record_path,
                snapshot_path,
            )?;
            registry.bind(&binding.record)?;
            return Ok(binding);
        }
        if !reservation.created {
            let detail = match reservation.status {
                RegistryStatus::Reserved => "interrupted reservation has no completed snapshot",
                RegistryStatus::Bound => "registry binding has no matching snapshot",
            };
            return Err(format!("SourceSnapshotOrphaned: {detail}; slot retained"));
        }

        let source_path = Path::new(&request.source_path);
        let source = match open_read_exclusive(source_path) {
            Ok(source) => source,
            Err(error) => {
                return Err(registry
                    .release(request.job_id, request.generation, &destination)
                    .err()
                    .map_or_else(
                        || format!("SourceUnavailable: cannot lock source for snapshot: {error}"),
                        |release| format!("SourceUnavailable: {error}; {release}"),
                    ));
            }
        };
        let before = match source.metadata() {
            Ok(metadata) if metadata.is_file() => metadata,
            Ok(_) => {
                let _ = registry.release(request.job_id, request.generation, &destination);
                return Err("SourceMissing: selected audio source is not a file".into());
            }
            Err(error) => {
                let _ = registry.release(request.job_id, request.generation, &destination);
                return Err(format!("SourceUnavailable: source metadata: {error}"));
            }
        };

        let mut partial = match OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&partial_path)
        {
            Ok(file) => file,
            Err(error) => {
                let _ = registry.release(request.job_id, request.generation, &destination);
                return Err(format!(
                    "StorageUnavailable: create source snapshot: {error}"
                ));
            }
        };
        if let Err(error) = partial.set_len(before.len()) {
            drop(partial);
            return Err(cleanup_failed_capture(
                &registry,
                request,
                &destination,
                &[&partial_path],
                format!("StorageUnavailable: reserve snapshot space: {error}"),
            ));
        }
        if let Err(error) = partial.seek(SeekFrom::Start(0)) {
            drop(partial);
            return Err(cleanup_failed_capture(
                &registry,
                request,
                &destination,
                &[&partial_path],
                format!("StorageUnavailable: position source snapshot: {error}"),
            ));
        }

        let mut hasher = Sha256::new();
        let copied = match copy_and_hash(&mut &source, &mut partial, &mut hasher) {
            Ok(copied) => copied,
            Err(error) => {
                drop(partial);
                return Err(cleanup_failed_capture(
                    &registry,
                    request,
                    &destination,
                    &[&partial_path],
                    format!("StorageUnavailable: copy source snapshot: {error}"),
                ));
            }
        };
        let after = match source.metadata() {
            Ok(metadata) => metadata,
            Err(error) => {
                drop(partial);
                return Err(cleanup_failed_capture(
                    &registry,
                    request,
                    &destination,
                    &[&partial_path],
                    format!("SourceChanged: source metadata after snapshot: {error}"),
                ));
            }
        };
        if copied != before.len() || copied != after.len() {
            drop(partial);
            return Err(cleanup_failed_capture(
                &registry,
                request,
                &destination,
                &[&partial_path],
                "SourceChanged: source size changed while its snapshot was copied".into(),
            ));
        }
        if let Err(error) = partial.sync_all() {
            drop(partial);
            return Err(cleanup_failed_capture(
                &registry,
                request,
                &destination,
                &[&partial_path],
                format!("StorageUnavailable: sync source snapshot: {error}"),
            ));
        }
        drop(partial);
        if let Err(error) = fs::rename(&partial_path, &snapshot_path) {
            return Err(cleanup_failed_capture(
                &registry,
                request,
                &destination,
                &[&partial_path, &snapshot_path],
                format!("StorageUnavailable: seal source snapshot: {error}"),
            ));
        }

        let source_sha256: [u8; 32] = hasher.finalize().into();
        let source_samples = match crate::archive::source_output_samples(&snapshot_path) {
            Ok(samples) => samples,
            Err(error) => {
                return Err(cleanup_failed_capture(
                    &registry,
                    request,
                    &destination,
                    &[&snapshot_path],
                    error,
                ));
            }
        };
        if request
            .source_sha256
            .is_some_and(|expected| expected != source_sha256)
            || request
                .source_samples
                .is_some_and(|expected| expected != source_samples)
        {
            return Err(cleanup_failed_capture(
                &registry,
                request,
                &destination,
                &[&snapshot_path],
                "SourceChanged: snapshot differs from the accepted source identity".into(),
            ));
        }

        let record = SnapshotRecord {
            version: 1,
            job_id: request.job_id,
            generation: request.generation,
            destination: destination.to_string_lossy().into_owned(),
            original_path: request.source_path.clone(),
            snapshot_path: snapshot_path.to_string_lossy().into_owned(),
            source_len: copied,
            source_sha256,
            source_samples,
        };
        if let Err(error) = write_record_new(&record_path, &record) {
            return Err(cleanup_failed_capture(
                &registry,
                request,
                &destination,
                &[&snapshot_path, &record_path],
                error,
            ));
        }
        registry.bind(&record)?;
        let snapshot_guard = open_verified_snapshot(&snapshot_path, &record)?;
        Ok(Self {
            registry_root: registry_root.to_path_buf(),
            directory,
            record_path,
            snapshot_path,
            record,
            snapshot_guard: Some(snapshot_guard),
        })
    }

    pub fn reopen_for_identity(
        registry_root: &Path,
        destination: &Path,
        job_id: JobId,
        generation: Generation,
    ) -> Result<Option<Self>, String> {
        let canonical_destination = fs::canonicalize(destination)
            .map_err(|error| format!("StorageUnavailable: canonical destination: {error}"))?;
        let directory = canonical_destination.join(SNAPSHOT_DIRECTORY);
        let stem = snapshot_stem(job_id, generation);
        let record_path = directory.join(format!("{stem}.json"));
        let snapshot_path = directory.join(format!("{stem}.snapshot"));
        let registry = SnapshotRegistry::new(registry_root);
        if !record_path.exists() && !snapshot_path.exists() {
            return Ok(None);
        }
        let record = read_record(&record_path)?;
        validate_record(
            &record,
            &canonical_destination,
            &snapshot_path,
            job_id,
            generation,
        )?;
        let snapshot_guard = open_verified_snapshot(&snapshot_path, &record)?;
        registry.bind(&record)?;
        Ok(Some(Self {
            registry_root: registry_root.to_path_buf(),
            directory,
            record_path,
            snapshot_path,
            record,
            snapshot_guard: Some(snapshot_guard),
        }))
    }

    fn open_existing(
        registry_root: &Path,
        request: &ImportRequest,
        destination: PathBuf,
        directory: PathBuf,
        record_path: PathBuf,
        snapshot_path: PathBuf,
    ) -> Result<Self, String> {
        let record = read_record(&record_path)?;
        validate_record(
            &record,
            &destination,
            &snapshot_path,
            request.job_id,
            request.generation,
        )?;
        if record.original_path != request.source_path
            || request
                .source_sha256
                .is_some_and(|expected| expected != record.source_sha256)
            || request
                .source_samples
                .is_some_and(|expected| expected != record.source_samples)
        {
            return Err(
                "SourceSnapshotMismatch: stored source binding differs from the request".into(),
            );
        }
        let snapshot_guard = open_verified_snapshot(&snapshot_path, &record)?;
        Ok(Self {
            registry_root: registry_root.to_path_buf(),
            directory,
            record_path,
            snapshot_path,
            record,
            snapshot_guard: Some(snapshot_guard),
        })
    }

    pub fn snapshot_path(&self) -> &Path {
        &self.snapshot_path
    }

    pub fn original_path(&self) -> &str {
        &self.record.original_path
    }

    pub fn source_sha256(&self) -> [u8; 32] {
        self.record.source_sha256
    }

    pub fn source_samples(&self) -> u64 {
        self.record.source_samples
    }

    pub fn apply_to_request(&self, request: &ImportRequest) -> ImportRequest {
        let mut execution = request.clone();
        execution.source_path = self.snapshot_path.to_string_lossy().into_owned();
        execution.source_sha256 = Some(self.record.source_sha256);
        execution.source_samples = Some(self.record.source_samples);
        execution
    }

    pub fn rebind(mut self, job_id: JobId, generation: Generation) -> Result<Self, String> {
        if self.record.job_id == job_id && self.record.generation == generation {
            return Ok(self);
        }
        let old_snapshot = self.snapshot_path.clone();
        let old_record_path = self.record_path.clone();
        let new_stem = snapshot_stem(job_id, generation);
        let new_snapshot = self.directory.join(format!("{new_stem}.snapshot"));
        let new_record_path = self.directory.join(format!("{new_stem}.json"));
        if new_snapshot.exists() || new_record_path.exists() {
            return Err(
                "SourceSnapshotMismatch: allocated operation identity is already bound".into(),
            );
        }
        let old_record = self.record.clone();
        let registry = SnapshotRegistry::new(&self.registry_root);
        registry.rebind(&old_record, job_id, generation, &new_snapshot)?;
        drop(self.snapshot_guard.take());
        if let Err(error) = fs::rename(&old_snapshot, &new_snapshot) {
            let interim = SnapshotRecord {
                job_id,
                generation,
                ..old_record.clone()
            };
            let _ = registry.rebind(
                &interim,
                old_record.job_id,
                old_record.generation,
                &old_snapshot,
            );
            self.snapshot_guard = Some(
                open_verified_snapshot(&old_snapshot, &old_record).map_err(|lock_error| {
                    format!("StorageUnavailable: rebind source snapshot: {error}; restore lock failed: {lock_error}")
                })?,
            );
            return Err(format!(
                "StorageUnavailable: rebind source snapshot: {error}"
            ));
        }
        self.record.job_id = job_id;
        self.record.generation = generation;
        self.record.snapshot_path = new_snapshot.to_string_lossy().into_owned();
        if let Err(error) = write_record_new(&new_record_path, &self.record) {
            let _ = fs::rename(&new_snapshot, &old_snapshot);
            let _ = registry.rebind(
                &self.record,
                old_record.job_id,
                old_record.generation,
                &old_snapshot,
            );
            self.record = old_record;
            self.snapshot_guard =
                Some(open_verified_snapshot(&old_snapshot, &self.record).map_err(
                    |lock_error| format!("{error}; snapshot lock restore failed: {lock_error}"),
                )?);
            return Err(error);
        }
        if let Err(error) = fs::remove_file(&old_record_path) {
            let _ = fs::remove_file(&new_record_path);
            let _ = fs::rename(&new_snapshot, &old_snapshot);
            let _ = registry.rebind(
                &self.record,
                old_record.job_id,
                old_record.generation,
                &old_snapshot,
            );
            self.record = old_record;
            self.snapshot_guard = Some(
                open_verified_snapshot(&old_snapshot, &self.record).map_err(|lock_error| {
                    format!("StorageUnavailable: retire previous binding record: {error}; restore lock failed: {lock_error}")
                })?,
            );
            return Err(format!(
                "StorageUnavailable: retire previous binding record: {error}"
            ));
        }
        registry.bind(&self.record)?;
        self.snapshot_path = new_snapshot;
        self.record_path = new_record_path;
        self.snapshot_guard = Some(
            open_verified_snapshot(&self.snapshot_path, &self.record).map_err(|error| {
                format!("StorageUnavailable: lock rebound source snapshot: {error}")
            })?,
        );
        Ok(self)
    }

    /// Frees a global slot only after this binding's snapshot and sidecar were removed.
    pub fn retire(self) -> Result<(), String> {
        let snapshot = self.snapshot_path.clone();
        let record_path = self.record_path.clone();
        let directory = self.directory.clone();
        let registry_root = self.registry_root.clone();
        let record = self.record.clone();
        drop(self.snapshot_guard);
        fs::remove_file(&snapshot)
            .map_err(|error| format!("StorageUnavailable: remove completed snapshot: {error}"))?;
        fs::remove_file(&record_path)
            .map_err(|error| format!("StorageUnavailable: remove snapshot record: {error}"))?;
        SnapshotRegistry::new(&registry_root).release_record(&record)?;
        let _ = fs::remove_dir(directory);
        Ok(())
    }
}

#[derive(Clone)]
struct SnapshotRegistry {
    root: PathBuf,
}

impl SnapshotRegistry {
    fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    fn reserve(&self, request: &ImportRequest, destination: &Path) -> Result<Reservation, String> {
        self.update(|entries| {
            let identity = registry_identity(request.job_id, request.generation, destination);
            if let Some(existing) = entries.iter().find(|entry| registry_same_operation(entry, &identity)) {
                return Ok(Reservation { status: existing.status, created: false });
            }
            if entries.iter().any(|entry| {
                entry.job_id == request.job_id && entry.generation == request.generation
            }) {
                return Err("SourceSnapshotRegistry: operation identity is already bound to another destination".into());
            }
            if entries.len() >= MAX_RETAINED_SNAPSHOTS {
                return Err(capacity_error());
            }
            entries.push(identity);
            Ok(Reservation { status: RegistryStatus::Reserved, created: true })
        })
    }

    fn bind(&self, record: &SnapshotRecord) -> Result<(), String> {
        self.update(|entries| {
            let entry = entries
                .iter_mut()
                .find(|entry| {
                    entry.job_id == record.job_id
                        && entry.generation == record.generation
                        && entry.destination == record.destination
                })
                .ok_or_else(|| "SourceSnapshotRegistry: binding has no reserved slot".to_owned())?;
            match entry.status {
                RegistryStatus::Reserved => {
                    entry.status = RegistryStatus::Bound;
                    entry.snapshot_path = Some(record.snapshot_path.clone());
                    Ok(())
                }
                RegistryStatus::Bound
                    if entry.snapshot_path.as_deref() == Some(record.snapshot_path.as_str()) =>
                {
                    Ok(())
                }
                RegistryStatus::Bound => Err(
                    "SourceSnapshotRegistryAmbiguous: bound operation points to a different snapshot; slot retained"
                        .into(),
                ),
            }
        })
    }

    fn rebind(
        &self,
        record: &SnapshotRecord,
        job_id: JobId,
        generation: Generation,
        snapshot_path: &Path,
    ) -> Result<(), String> {
        let snapshot_path = snapshot_path.to_string_lossy().into_owned();
        self.update(|entries| {
            let index = entries
                .iter()
                .position(|entry| {
                    entry.job_id == record.job_id
                        && entry.generation == record.generation
                        && entry.destination == record.destination
                })
                .ok_or_else(|| {
                    "SourceSnapshotRegistry: prior operation slot is missing".to_owned()
                })?;
            if entries
                .iter()
                .any(|candidate| candidate.job_id == job_id && candidate.generation == generation)
            {
                return Err(
                    "SourceSnapshotRegistry: target operation identity already owns a slot".into(),
                );
            }
            entries[index].job_id = job_id;
            entries[index].generation = generation;
            entries[index].snapshot_path = Some(snapshot_path);
            Ok(())
        })
    }

    fn release(
        &self,
        job_id: JobId,
        generation: Generation,
        destination: &Path,
    ) -> Result<(), String> {
        let destination = destination.to_string_lossy().into_owned();
        self.update(|entries| {
            if let Some(index) = entries.iter().position(|entry| {
                entry.job_id == job_id
                    && entry.generation == generation
                    && entry.destination == destination
            }) {
                if entries[index].status != RegistryStatus::Reserved
                    || entries[index].snapshot_path.is_some()
                {
                    return Err("SourceSnapshotRegistry: refusing to release a bound slot".into());
                }
                entries.remove(index);
            }
            Ok(())
        })
    }

    fn release_record(&self, record: &SnapshotRecord) -> Result<(), String> {
        self.update(|entries| {
            let index = entries
                .iter()
                .position(|entry| {
                    entry.job_id == record.job_id
                        && entry.generation == record.generation
                        && entry.destination == record.destination
                        && entry.snapshot_path.as_deref() == Some(record.snapshot_path.as_str())
                })
                .ok_or_else(|| {
                    "SourceSnapshotRegistry: refusing to release a different binding".to_owned()
                })?;
            entries.remove(index);
            Ok(())
        })
    }

    fn update<T>(
        &self,
        change: impl FnOnce(&mut Vec<RegistryEntry>) -> Result<T, String>,
    ) -> Result<T, String> {
        fs::create_dir_all(&self.root)
            .map_err(|error| format!("SourceSnapshotRegistryUnavailable: create root: {error}"))?;
        let lock_path = self.root.join(REGISTRY_LOCK_FILE);
        let lock_file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(&lock_path)
            .map_err(|error| format!("SourceSnapshotRegistryUnavailable: open lock: {error}"))?;
        lock_file.lock().map_err(|error| {
            format!("SourceSnapshotRegistryUnavailable: lock registry: {error}")
        })?;

        let path = self.root.join(REGISTRY_FILE);
        let mut entries = match fs::read(&path) {
            Ok(bytes) if bytes.is_empty() => {
                return Err("SourceSnapshotRegistryCorrupt: empty registry file".into());
            }
            Ok(bytes) => serde_json::from_slice::<Vec<RegistryEntry>>(&bytes)
                .map_err(|error| format!("SourceSnapshotRegistryCorrupt: {error}"))?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(error) => return Err(format!("SourceSnapshotRegistryUnavailable: read: {error}")),
        };
        validate_registry(&entries)?;
        let result = change(&mut entries)?;
        validate_registry(&entries)?;
        let bytes = serde_json::to_vec(&entries)
            .map_err(|error| format!("SourceSnapshotRegistryUnavailable: serialize: {error}"))?;
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&path)
            .map_err(|error| format!("SourceSnapshotRegistryUnavailable: open state: {error}"))?;
        if let Err(error) = file.write_all(&bytes).and_then(|()| file.sync_all()) {
            return Err(format!(
                "SourceSnapshotRegistryUnavailable: persist state: {error}"
            ));
        }
        Ok(result)
    }
}

fn validate_registry(entries: &[RegistryEntry]) -> Result<(), String> {
    if entries.len() > MAX_RETAINED_SNAPSHOTS {
        return Err("SourceSnapshotRegistryCorrupt: more than two slots are present".into());
    }
    for (index, entry) in entries.iter().enumerate() {
        if entries[index + 1..].iter().any(|other| {
            entry.job_id == other.job_id && entry.generation == other.generation
                || entry.snapshot_path.is_some() && entry.snapshot_path == other.snapshot_path
        }) || (entry.status == RegistryStatus::Bound && entry.snapshot_path.is_none())
            || (entry.status == RegistryStatus::Reserved && entry.snapshot_path.is_some())
        {
            return Err("SourceSnapshotRegistryCorrupt: ambiguous slot identity".into());
        }
    }
    Ok(())
}

fn registry_identity(job_id: JobId, generation: Generation, destination: &Path) -> RegistryEntry {
    RegistryEntry {
        job_id,
        generation,
        destination: destination.to_string_lossy().into_owned(),
        status: RegistryStatus::Reserved,
        snapshot_path: None,
    }
}

fn registry_same_operation(left: &RegistryEntry, right: &RegistryEntry) -> bool {
    left.job_id == right.job_id
        && left.generation == right.generation
        && left.destination == right.destination
}

fn capacity_error() -> String {
    "SourceSnapshotCapacity: deux copies audio sont déjà retenues; terminez ou annulez un traitement puis réessayez".into()
}

fn ensure_destination(destination: &str) -> Result<PathBuf, String> {
    let root = Path::new(destination);
    if !root.is_dir() {
        fs::create_dir_all(root)
            .map_err(|error| format!("StorageUnavailable: destination: {error}"))?;
    }
    fs::canonicalize(root)
        .map_err(|error| format!("StorageUnavailable: canonical destination: {error}"))
}

fn cleanup_failed_capture(
    registry: &SnapshotRegistry,
    request: &ImportRequest,
    destination: &Path,
    artifacts: &[&Path],
    cause: String,
) -> String {
    for artifact in artifacts {
        if artifact.exists() {
            if let Err(error) = fs::remove_file(artifact) {
                return format!("{cause}; SourceSnapshotOrphaned: {artifact:?}: {error}");
            }
        }
    }
    match registry.release(request.job_id, request.generation, destination) {
        Ok(()) => cause,
        Err(error) => format!("{cause}; reserved slot retained fail-closed: {error}"),
    }
}

fn snapshot_stem(job_id: JobId, generation: Generation) -> String {
    format!("{:032x}-{}", job_id.0, generation.get())
}

fn read_record(path: &Path) -> Result<SnapshotRecord, String> {
    serde_json::from_slice(
        &fs::read(path)
            .map_err(|error| format!("SourceSnapshotUnavailable: snapshot record: {error}"))?,
    )
    .map_err(|error| format!("SourceSnapshotCorrupt: snapshot record: {error}"))
}

fn validate_record(
    record: &SnapshotRecord,
    destination: &Path,
    snapshot_path: &Path,
    job_id: JobId,
    generation: Generation,
) -> Result<(), String> {
    let expected_snapshot = fs::canonicalize(snapshot_path)
        .map_err(|error| format!("SourceSnapshotUnavailable: snapshot bytes: {error}"))?;
    let metadata = fs::metadata(&expected_snapshot)
        .map_err(|error| format!("SourceSnapshotUnavailable: snapshot bytes: {error}"))?;
    if record.version != 1
        || record.job_id != job_id
        || record.generation != generation
        || record.destination != destination.to_string_lossy()
        || PathBuf::from(&record.snapshot_path) != expected_snapshot
        || !metadata.is_file()
        || metadata.len() != record.source_len
    {
        return Err("SourceSnapshotCorrupt: durable binding does not match its operation".into());
    }
    Ok(())
}

fn open_read_exclusive(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.share_mode(0x0000_0001);
    }
    options.open(path)
}

fn open_verified_snapshot(path: &Path, record: &SnapshotRecord) -> Result<File, String> {
    let mut file = open_read_exclusive(path)
        .map_err(|error| format!("StorageUnavailable: lock source snapshot: {error}"))?;
    let metadata = file
        .metadata()
        .map_err(|error| format!("SourceSnapshotUnavailable: snapshot metadata: {error}"))?;
    if !metadata.is_file() || metadata.len() != record.source_len {
        return Err("SourceSnapshotCorrupt: snapshot size changed after capture".into());
    }
    let mut hasher = Sha256::new();
    let mut sink = io::sink();
    let read = copy_and_hash(&mut file, &mut sink, &mut hasher)
        .map_err(|error| format!("SourceSnapshotUnavailable: verify snapshot: {error}"))?;
    let actual: [u8; 32] = hasher.finalize().into();
    if read != record.source_len || actual != record.source_sha256 {
        return Err("SourceSnapshotCorrupt: snapshot digest changed after capture".into());
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|error| format!("SourceSnapshotUnavailable: rewind verified snapshot: {error}"))?;
    Ok(file)
}

fn write_record_new(path: &Path, record: &SnapshotRecord) -> Result<(), String> {
    let bytes = serde_json::to_vec(record)
        .map_err(|error| format!("StorageUnavailable: serialize snapshot record: {error}"))?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("StorageUnavailable: create snapshot record: {error}"))?;
    if let Err(error) = file.write_all(&bytes).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(format!(
            "StorageUnavailable: persist snapshot record: {error}"
        ));
    }
    Ok(())
}

fn copy_and_hash<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
    hasher: &mut Sha256,
) -> io::Result<u64> {
    let mut buffer = vec![0; COPY_BUFFER_SIZE];
    let mut copied = 0u64;
    loop {
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            return Ok(copied);
        }
        writer.write_all(&buffer[..count])?;
        hasher.update(&buffer[..count]);
        copied = copied
            .checked_add(count as u64)
            .ok_or_else(|| io::Error::other("source snapshot size overflow"))?;
    }
}
