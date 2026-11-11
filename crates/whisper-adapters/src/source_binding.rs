//! Durable snapshots binding imports to the exact bytes sent to a worker.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use whisper_core::ports::SourceSnapshotRecoveryCandidate;
use whisper_core::{Generation, JobId, ports::ImportRequest};

const SNAPSHOT_DIRECTORY: &str = ".whisper-source-snapshots";
const REGISTRY_FILE: &str = "reprocessing-snapshots.json";
const REGISTRY_LOCK_FILE: &str = "reprocessing-snapshots.lock";
const OPERATION_LOCK_FILE: &str = "source-snapshot-operation.lock";
const MAX_RETAINED_SNAPSHOTS: usize = 2;
const COPY_BUFFER_SIZE: usize = 1024 * 1024;

fn legacy_strategy() -> whisper_core::parakeet_session::TranscriptionStrategy {
    whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1
}

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
    #[serde(default = "legacy_strategy")]
    strategy: whisper_core::parakeet_session::TranscriptionStrategy,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct RegistryEntry {
    job_id: JobId,
    generation: Generation,
    destination: String,
    status: RegistryStatus,
    snapshot_path: Option<String>,
    #[serde(default)]
    retirement: Option<RetirementIntent>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct RetirementIntent {
    snapshot: RecoveryArtifact,
    record: RecoveryArtifact,
    #[serde(default)]
    source_record: Option<SnapshotRecord>,
    #[serde(default)]
    source_record_bytes: Option<Vec<u8>>,
}

#[derive(Deserialize)]
struct RecoveryPendingIdentity {
    job_id: JobId,
    generation: Generation,
    source_path: String,
    source_sha256: String,
    source_samples: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecoveryPublicationIntent {
    version: u32,
    operation_id: String,
    choice: whisper_core::ports::DuplicateImportChoice,
    source_sha256: String,
    source_samples: u64,
    expected_active: Vec<RecoveryPublicationIdentity>,
    export_basename: String,
    #[serde(default)]
    internal_only: bool,
    #[serde(default)]
    cleanup_id: Option<String>,
    #[serde(default)]
    cleanup_targets: Vec<RecoveryCleanupTarget>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecoveryPublicationIdentity {
    job_id: JobId,
    generation: Generation,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecoveryCleanupObjectIdentity {
    volume_serial: u64,
    file_id: [u8; 16],
    is_directory: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecoveryCleanupObject {
    relative_path: String,
    identity: RecoveryCleanupObjectIdentity,
    sha256: Option<String>,
    #[serde(default = "cleanup_owned_default")]
    owned: bool,
    removed: bool,
    error: Option<String>,
}

fn cleanup_owned_default() -> bool {
    true
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecoveryCleanupAncestor {
    relative_path: String,
    identity: RecoveryCleanupObjectIdentity,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecoveryCleanupTarget {
    treatment: RecoveryTreatmentIdentity,
    source_sha256: String,
    ancestors: Vec<RecoveryCleanupAncestor>,
    objects: Vec<RecoveryCleanupObject>,
    #[serde(default)]
    container: Option<RecoveryCleanupObject>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RecoveryTreatmentIdentity {
    destination: String,
    job_id: JobId,
    generation: Generation,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct RecoveryArtifact {
    path: String,
    volume_serial: u64,
    file_id: [u8; 16],
    is_directory: bool,
    sha256: String,
}

struct Reservation {
    status: RegistryStatus,
    created: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
enum RegistryStatus {
    Reserved,
    Bound,
    Retiring,
}

enum OperationGuardProbe {
    Acquired { _guard: File },
    Busy,
    Missing,
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
        Self::acquire_with_hook(request, registry_root, |_| {})
    }

    fn acquire_with_hook<F>(
        request: &ImportRequest,
        registry_root: &Path,
        after_partial_created: F,
    ) -> Result<Self, String>
    where
        F: FnOnce(&Path),
    {
        let destination = ensure_destination(&request.destination)?;
        let _operation_guard = lock_operation(registry_root)?;
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
                RegistryStatus::Retiring => {
                    "source snapshot retirement is awaiting explicit recovery"
                }
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
        after_partial_created(&partial_path);
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
            version: snapshot_schema(request.strategy),
            job_id: request.job_id,
            generation: request.generation,
            destination: destination.to_string_lossy().into_owned(),
            original_path: request.source_path.clone(),
            snapshot_path: snapshot_path.to_string_lossy().into_owned(),
            source_len: copied,
            source_sha256,
            source_samples,
            strategy: request.strategy,
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
        let _operation_guard = lock_operation(registry_root)?;
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
            || record.strategy != request.strategy
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

    pub fn strategy(&self) -> whisper_core::parakeet_session::TranscriptionStrategy {
        self.record.strategy
    }

    pub fn apply_to_request(&self, request: &ImportRequest) -> Result<ImportRequest, String> {
        if request.strategy != self.record.strategy {
            return Err(
                "SourceSnapshotMismatch: transcription strategy differs from binding".into(),
            );
        }
        let mut execution = request.clone();
        execution.source_path = self.snapshot_path.to_string_lossy().into_owned();
        execution.source_sha256 = Some(self.record.source_sha256);
        execution.source_samples = Some(self.record.source_samples);
        Ok(execution)
    }

    pub fn rebind(mut self, job_id: JobId, generation: Generation) -> Result<Self, String> {
        if self.record.job_id == job_id && self.record.generation == generation {
            return Ok(self);
        }
        let _operation_guard = lock_operation(&self.registry_root)?;
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
    pub fn retire(mut self) -> Result<(), String> {
        let _operation_guard = lock_operation(&self.registry_root)?;
        let registry = SnapshotRegistry::new(&self.registry_root);
        let snapshot_lock = self.snapshot_guard.take().ok_or_else(|| {
            "SourceSnapshotUnavailable: verified snapshot lock is missing".to_owned()
        })?;
        let mut record_file =
            crate::windows_file_identity::VerifiedFile::open(&self.record_path, false)?;
        let record_bytes = record_file.read_all()?;
        let current_record: SnapshotRecord = serde_json::from_slice(&record_bytes)
            .map_err(|error| format!("SourceSnapshotCorrupt: snapshot record: {error}"))?;
        if current_record != self.record {
            return Err("SourceSnapshotMismatch: record changed before retirement".into());
        }
        // Drop the ordinary read lock before opening a disposition-capable
        // handle. The verified handle rechecks the file identity and digest
        // before marking that same object for deletion.
        drop(snapshot_lock);
        let snapshot = open_disposition_snapshot(&self.snapshot_path, &self.record)?;
        let intent = RetirementIntent {
            snapshot: recovery_artifact(
                &self.snapshot_path,
                &snapshot,
                &self.record.source_sha256,
            )?,
            record: recovery_artifact_from_file(&self.record_path, &mut record_file)?,
            source_record: Some(self.record.clone()),
            source_record_bytes: Some(record_bytes),
        };
        registry.begin_retirement(&self.record, intent.clone())?;
        delete_verified_handle(snapshot, &intent.snapshot)?;
        delete_verified_handle(record_file, &intent.record)?;
        registry.finish_retirement(&self.record)
    }

    /// Completes an interrupted normal release for this exact operation. A
    /// missing Bound entry is never inferred to be abandoned here; only a
    /// previously persisted Retiring intent may be replayed.
    pub fn retire_for_identity(
        registry_root: &Path,
        destination: &Path,
        job_id: JobId,
        generation: Generation,
    ) -> Result<(), String> {
        let registry = SnapshotRegistry::new(registry_root);
        let entry = registry.read_entries()?.into_iter().find(|entry| {
            entry.job_id == job_id
                && entry.generation == generation
                && entry.destination == destination.to_string_lossy()
        });
        let Some(entry) = entry else {
            return Ok(());
        };
        if entry.status == RegistryStatus::Retiring {
            return resume_retirement(&registry, &entry, false);
        }
        let binding = Self::reopen_for_identity(registry_root, destination, job_id, generation)?
            .ok_or_else(|| {
                "SourceSnapshotOrphaned: bound files are missing; explicit recovery required"
                    .to_owned()
            })?;
        binding.retire()
    }

    /// Read-only inspection for the explicit recovery UI. Every candidate is
    /// checked against queue/archive state, process-owned bindings, file
    /// identity and the snapshot's current lock state.
    pub fn inspect_recovery_candidates(
        registry_root: &Path,
        active: &[(JobId, u64)],
    ) -> Result<Vec<SourceSnapshotRecoveryCandidate>, String> {
        let registry = SnapshotRegistry::new(registry_root);
        registry
            .read_entries()?
            .iter()
            .map(|entry| {
                let publication_guard =
                    recovery_publication_guard(Path::new(&entry.destination), &entry.destination)?;
                let operation_guard = recovery_operation_guard(registry_root)?;
                let (operation_guard_acquired, operation_guard_busy) =
                    operation_guard_state(&operation_guard);
                recovery_candidate(
                    entry,
                    active,
                    operation_guard_acquired,
                    operation_guard_busy,
                    publication_guard.is_some(),
                )
            })
            .collect()
    }

    /// Revalidates each candidate immediately before disposition. Slots are
    /// removed only after an fsynced intent and handle-based deletion.
    pub fn recover_abandoned(
        registry_root: &Path,
        active: &[(JobId, u64)],
    ) -> Result<(usize, Vec<SourceSnapshotRecoveryCandidate>), String> {
        let registry = SnapshotRegistry::new(registry_root);
        let entries = registry.read_entries()?;
        let mut freed = 0;
        for entry in &entries {
            let destination = Path::new(&entry.destination);
            let _publication_lock = crate::archive::ArchiveStore::lock_publications(destination)?;
            let _operation_guard = recovery_operation_guard(registry_root)?;
            let (operation_guard_acquired, operation_guard_busy) =
                operation_guard_state(&_operation_guard);
            let candidate = recovery_candidate(
                entry,
                active,
                operation_guard_acquired,
                operation_guard_busy,
                true,
            )?;
            if !candidate.recoverable {
                continue;
            }
            if entry.status == RegistryStatus::Retiring {
                resume_retirement(&registry, entry, true)?;
            } else {
                recover_new_entry(&registry, entry)?;
            }
            freed += 1;
        }
        let remaining = registry
            .read_entries()?
            .iter()
            .map(|entry| {
                let publication_guard =
                    recovery_publication_guard(Path::new(&entry.destination), &entry.destination)?;
                let operation_guard = recovery_operation_guard(registry_root)?;
                let (operation_guard_acquired, operation_guard_busy) =
                    operation_guard_state(&operation_guard);
                recovery_candidate(
                    entry,
                    active,
                    operation_guard_acquired,
                    operation_guard_busy,
                    publication_guard.is_some(),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok((freed, remaining))
    }
}

fn recovery_candidate(
    entry: &RegistryEntry,
    active: &[(JobId, u64)],
    operation_guard_acquired: bool,
    operation_guard_busy: bool,
    publication_lock_acquired: bool,
) -> Result<SourceSnapshotRecoveryCandidate, String> {
    let protected = |reason: String| SourceSnapshotRecoveryCandidate {
        job_id: entry.job_id,
        generation: entry.generation,
        destination: entry.destination.clone(),
        recoverable: false,
        reason: Some(reason),
    };
    if active.contains(&(entry.job_id, entry.generation.get())) {
        return Ok(protected("Traitement actif dans cette session".into()));
    }
    if operation_guard_busy {
        return Ok(protected("Une opération de copie est en cours".into()));
    }
    if entry.status == RegistryStatus::Reserved && !operation_guard_acquired {
        return Ok(protected(
            "Réservation en cours ou activité impossible à exclure".into(),
        ));
    }
    let destination = PathBuf::from(&entry.destination);
    let canonical = match fs::canonicalize(&destination) {
        Ok(path) if path.to_string_lossy() == entry.destination => path,
        Ok(_) => {
            return Ok(protected(
                "Destination redirigée ou identité modifiée".into(),
            ));
        }
        Err(error) => return Ok(protected(format!("Destination inconnue : {error}"))),
    };
    let queue = match crate::queue_store::QueueStore::inspect_read_only(&canonical) {
        Ok(queue) => queue,
        Err(error) => return Ok(protected(error)),
    };
    if queue.iter().any(|item| {
        item.request.job_id == entry.job_id && item.request.generation == entry.generation
    }) {
        return Ok(protected(
            "Traitement encore présent dans la file durable".into(),
        ));
    }
    let archived = match crate::archive::ArchiveStore::scan(&canonical) {
        Ok(archive) => archive,
        Err(error) => return Ok(protected(format!("État d'archive illisible : {error}"))),
    };
    let publication_reservation = canonical
        .join(".whisper-publications")
        .join("reservations")
        .join(format!(
            "{:032x}-{}.json",
            entry.job_id.0,
            entry.generation.get()
        ));
    let publication_reservation_exists = file_exists_strict(&publication_reservation)?;

    let snapshot_path = expected_snapshot_path(&canonical, entry);
    let partial_path = canonical.join(SNAPSHOT_DIRECTORY).join(format!(
        "{}.snapshot.partial",
        snapshot_stem(entry.job_id, entry.generation)
    ));
    if file_exists_strict(&partial_path)? {
        return Ok(protected(
            "Copie partielle présente : réservation conservée".into(),
        ));
    }
    if entry.status == RegistryStatus::Retiring {
        let Some(intent) = entry.retirement.as_ref() else {
            return Ok(protected("Intention de retrait durable absente".into()));
        };
        if Path::new(&intent.snapshot.path) != snapshot_path
            || Path::new(&intent.record.path) != expected_record_path(&canonical, entry)
        {
            return Ok(protected("Chemin de retrait durable divergent".into()));
        }
        if let Err(error) = check_pending_intent(intent) {
            return Ok(protected(error));
        }
        let archive_blocks = archive_blocks_snapshot_recovery(
            &archived,
            entry.job_id,
            entry.generation,
            Some(&intent.snapshot.sha256),
        );
        if archive_blocks || publication_reservation_exists {
            if !operation_guard_acquired || !publication_lock_acquired {
                return Ok(protected(
                    "Verrous requis indisponibles pour reprendre le retrait".into(),
                ));
            }
            let record = match retirement_source_record(&canonical, entry, intent) {
                Ok(Some(record)) => record,
                Ok(None) => {
                    return Ok(protected(
                        "Intention de retrait sans identité source vérifiable".into(),
                    ));
                }
                Err(error) => return Ok(protected(error)),
            };
            let _pending_guard = match redundant_pending_source_guard(
                &canonical,
                entry,
                &record,
                &archived,
                &publication_reservation,
            ) {
                Ok(Some(guard)) => guard,
                Ok(None) | Err(_) => {
                    return Ok(protected(
                        "Archive ou pending encore présent pour cette réservation".into(),
                    ));
                }
            };
            return Ok(SourceSnapshotRecoveryCandidate {
                job_id: entry.job_id,
                generation: entry.generation,
                destination: entry.destination.clone(),
                recoverable: true,
                reason: Some("Retrait interrompu, source redondante vérifiée".into()),
            });
        }
        return Ok(SourceSnapshotRecoveryCandidate {
            job_id: entry.job_id,
            generation: entry.generation,
            destination: entry.destination.clone(),
            recoverable: true,
            reason: Some("Retrait interrompu, reprise sûre disponible".into()),
        });
    }

    let record_path = expected_record_path(&canonical, entry);
    let snapshot_exists = file_exists_strict(&snapshot_path)?;
    let record_exists = file_exists_strict(&record_path)?;
    if !snapshot_exists && !record_exists {
        if publication_reservation_exists
            || archive_blocks_snapshot_recovery(&archived, entry.job_id, entry.generation, None)
        {
            if publication_reservation_exists {
                return Ok(protected(
                    "Réservation de publication encore présente".into(),
                ));
            }
            return Ok(protected(
                "Archive ou pending encore présent pour cette réservation".into(),
            ));
        }
        return Ok(SourceSnapshotRecoveryCandidate {
            job_id: entry.job_id,
            generation: entry.generation,
            destination: entry.destination.clone(),
            recoverable: true,
            reason: Some(
                "Aucune copie ni fiche ne reste; seule la réservation sera libérée".into(),
            ),
        });
    }
    if snapshot_exists != record_exists {
        return Ok(protected(
            "Copie audio ou fiche seule : état incomplet, réservation conservée".into(),
        ));
    }
    if entry.status != RegistryStatus::Bound {
        return Ok(protected(
            "Une réservation inachevée possède des fichiers inattendus".into(),
        ));
    }
    let (record, _) = match inspect_bound_pair(entry, &canonical, &snapshot_path, &record_path) {
        Ok(pair) => pair,
        Err(error) => return Ok(protected(error)),
    };
    let source_sha256 = hex_bytes(&record.source_sha256);
    let archive_blocks = archive_blocks_snapshot_recovery(
        &archived,
        entry.job_id,
        entry.generation,
        Some(&source_sha256),
    );
    if archive_blocks || publication_reservation_exists {
        if publication_reservation_exists && !archive_blocks {
            return Ok(protected(
                "Réservation de publication encore présente".into(),
            ));
        }
        match redundant_pending_source_guard(
            &canonical,
            entry,
            &record,
            &archived,
            &publication_reservation,
        ) {
            Ok(Some(_guard))
                if operation_guard_acquired
                    && !operation_guard_busy
                    && publication_lock_acquired =>
            {
                return Ok(SourceSnapshotRecoveryCandidate {
                    job_id: entry.job_id,
                    generation: entry.generation,
                    destination: entry.destination.clone(),
                    recoverable: true,
                    reason: Some(
                        "Source originale vérifiée identique, copie redondante récupérable".into(),
                    ),
                });
            }
            Err(error) => {
                return Ok(protected(format!(
                    "Archive ou pending encore présent pour cette réservation : {error}"
                )));
            }
            Ok(_) => {}
        }
        return Ok(protected(
            "Archive ou pending encore présent pour cette réservation".into(),
        ));
    }
    Ok(SourceSnapshotRecoveryCandidate {
        job_id: entry.job_id,
        generation: entry.generation,
        destination: entry.destination.clone(),
        recoverable: true,
        reason: Some("Copie et fiche vérifiées, aucune utilisation détectée".into()),
    })
}

struct RedundantPendingSourceGuard {
    _source: crate::windows_file_identity::VerifiedFile,
    _pending: crate::windows_file_identity::VerifiedFile,
    _reservation: crate::windows_file_identity::VerifiedFile,
}

fn redundant_pending_source_guard(
    destination: &Path,
    entry: &RegistryEntry,
    record: &SnapshotRecord,
    archived: &[crate::archive::ArchiveEntry],
    reservation_path: &Path,
) -> Result<Option<RedundantPendingSourceGuard>, String> {
    let source_sha256 = hex_bytes(&record.source_sha256);
    let mut matching_pending = 0;
    for item in archived {
        if item.diagnostic.is_some() && (item.job_id == entry.job_id || item.job_id == JobId(0)) {
            return Ok(None);
        }
        if item.job_id == entry.job_id && item.generation == entry.generation {
            if item.complete || item.publication_recovery || item.source_sha256 != source_sha256 {
                return Ok(None);
            }
            matching_pending += 1;
        }
    }
    if matching_pending != 1 {
        return Ok(None);
    }

    let generation_directory = destination
        .join("transcriptions")
        .join(format!("{:032x}", entry.job_id.0))
        .join(entry.generation.get().to_string());
    let pending_path = generation_directory.join("pending.json");
    let mut pending_file =
        match crate::windows_file_identity::VerifiedFile::open(&pending_path, false) {
            Ok(file) => file,
            Err(_) => return Ok(None),
        };
    let pending: RecoveryPendingIdentity = match serde_json::from_slice(&pending_file.read_all()?) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    if pending.job_id != entry.job_id
        || pending.generation != entry.generation
        || pending.source_path != record.original_path
        || pending.source_sha256 != source_sha256
        || pending.source_samples != record.source_samples
    {
        return Ok(None);
    }

    let mut reservation_file =
        match crate::windows_file_identity::VerifiedFile::open(reservation_path, false) {
            Ok(file) => file,
            Err(_) => return Ok(None),
        };
    let reservation: RecoveryPublicationIntent =
        match serde_json::from_slice(&reservation_file.read_all()?) {
            Ok(value) => value,
            Err(_) => return Ok(None),
        };
    if !validate_recovery_publication_intent(&reservation, entry, record) {
        return Ok(None);
    }

    let source_path = Path::new(&record.original_path);
    let mut source_file = match crate::windows_file_identity::VerifiedFile::open(source_path, false)
    {
        Ok(file) => file,
        Err(_) => return Ok(None),
    };
    let source_identity = source_file.identity.clone();
    let source_metadata = match fs::metadata(source_path) {
        Ok(metadata) if metadata.is_file() => metadata,
        _ => return Ok(None),
    };
    if source_metadata.len() != record.source_len
        || source_file.sha256()? != source_sha256
        || crate::archive::source_output_samples(source_path).ok() != Some(record.source_samples)
        || fs::metadata(source_path)
            .ok()
            .map(|metadata| metadata.len())
            != Some(record.source_len)
    {
        return Ok(None);
    }
    source_file.verify(&source_identity)?;

    Ok(Some(RedundantPendingSourceGuard {
        _source: source_file,
        _pending: pending_file,
        _reservation: reservation_file,
    }))
}

fn validate_recovery_publication_intent(
    intent: &RecoveryPublicationIntent,
    entry: &RegistryEntry,
    record: &SnapshotRecord,
) -> bool {
    let bare_operation_id = format!("{:032x}-{}", entry.job_id.0, entry.generation.get());
    let resumed_operation_id = format!("{}-{bare_operation_id}", record.strategy.stable_id());
    let legacy_operation_id = format!("legacy-{bare_operation_id}");
    if intent.version != 1
        || (intent.operation_id != bare_operation_id
            && intent.operation_id != resumed_operation_id
            && intent.operation_id != legacy_operation_id)
        || intent.source_sha256 != hex_bytes(&record.source_sha256)
        || !is_hex_sha256(&intent.source_sha256)
        || intent.source_samples == 0
        || intent.source_samples != record.source_samples
        || intent.export_basename.is_empty()
        || intent.export_basename == "."
        || intent.export_basename == ".."
        || intent
            .export_basename
            .chars()
            .any(|character| matches!(character, '/' | '\\' | ':'))
        || intent.cleanup_id.as_ref().is_some_and(String::is_empty)
    {
        return false;
    }

    if matches!(
        &intent.choice,
        whisper_core::ports::DuplicateImportChoice::CreateNew
    ) && (intent.cleanup_id.is_some() || !intent.cleanup_targets.is_empty())
    {
        return false;
    }

    for (index, identity) in intent.expected_active.iter().enumerate() {
        if identity.job_id == entry.job_id && identity.generation == entry.generation
            || intent.expected_active[index + 1..].iter().any(|other| {
                identity.job_id == other.job_id && identity.generation == other.generation
            })
        {
            return false;
        }
    }
    for (index, target) in intent.cleanup_targets.iter().enumerate() {
        if !is_hex_sha256(&target.source_sha256)
            || target.source_sha256 != intent.source_sha256
            || target.treatment.destination != entry.destination
            || (target.treatment.job_id == entry.job_id
                && target.treatment.generation == entry.generation)
            || intent.cleanup_targets[index + 1..].iter().any(|other| {
                target.treatment.job_id == other.treatment.job_id
                    && target.treatment.generation == other.treatment.generation
            })
        {
            return false;
        }
        for ancestor in &target.ancestors {
            if !safe_relative_identity_path(&ancestor.relative_path)
                || !valid_cleanup_identity(&ancestor.identity)
            {
                return false;
            }
        }
        for object in target.objects.iter().chain(target.container.iter()) {
            if !safe_relative_identity_path(&object.relative_path)
                || !valid_cleanup_identity(&object.identity)
                || object
                    .sha256
                    .as_ref()
                    .is_some_and(|sha| !is_hex_sha256(sha))
            {
                return false;
            }
            let _ownership_state = object.owned;
            let _cleanup_state = (object.removed, &object.error);
        }
    }
    let _internal_only = intent.internal_only;
    true
}

fn is_hex_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn safe_relative_identity_path(path: &str) -> bool {
    let path = Path::new(path);
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_)))
}

fn valid_cleanup_identity(identity: &RecoveryCleanupObjectIdentity) -> bool {
    let _is_directory = identity.is_directory;
    identity.volume_serial != 0 || identity.file_id.iter().any(|byte| *byte != 0)
}

fn archive_blocks_snapshot_recovery(
    archived: &[crate::archive::ArchiveEntry],
    job_id: JobId,
    generation: Generation,
    source_sha256: Option<&str>,
) -> bool {
    archived.iter().any(|item| {
        if item.diagnostic.is_some() && (item.job_id == job_id || item.job_id == JobId(0)) {
            return true;
        }
        if item.job_id != job_id || item.generation != generation {
            return false;
        }
        !item.complete
            || item.publication_recovery
            || source_sha256 != Some(item.source_sha256.as_str())
    })
}

fn inspect_bound_pair(
    entry: &RegistryEntry,
    destination: &Path,
    snapshot_path: &Path,
    record_path: &Path,
) -> Result<(SnapshotRecord, RetirementIntent), String> {
    let mut record_file = crate::windows_file_identity::VerifiedFile::open(record_path, false)?;
    let record_bytes = record_file.read_all()?;
    let record: SnapshotRecord = serde_json::from_slice(&record_bytes)
        .map_err(|error| format!("SourceSnapshotCorrupt: snapshot record: {error}"))?;
    validate_record(
        &record,
        destination,
        snapshot_path,
        entry.job_id,
        entry.generation,
    )?;
    if record.destination != entry.destination
        || entry.snapshot_path.as_deref() != Some(record.snapshot_path.as_str())
    {
        return Err("SourceSnapshotMismatch: registre et fiche divergents".into());
    }
    let snapshot = open_disposition_snapshot(snapshot_path, &record)?;
    let intent = RetirementIntent {
        snapshot: recovery_artifact(snapshot_path, &snapshot, &record.source_sha256)?,
        record: recovery_artifact_from_file(record_path, &mut record_file)?,
        source_record: Some(record.clone()),
        source_record_bytes: Some(record_bytes),
    };
    Ok((record, intent))
}

fn recover_new_entry(registry: &SnapshotRegistry, entry: &RegistryEntry) -> Result<(), String> {
    let destination = fs::canonicalize(&entry.destination)
        .map_err(|error| format!("StorageUnavailable: destination: {error}"))?;
    if destination.to_string_lossy() != entry.destination {
        return Err("CleanupUnsafePath: destination identity changed".into());
    }
    let snapshot_path = expected_snapshot_path(&destination, entry);
    let record_path = expected_record_path(&destination, entry);
    let snapshot_exists = file_exists_strict(&snapshot_path)?;
    let record_exists = file_exists_strict(&record_path)?;
    if !snapshot_exists && !record_exists {
        return registry.remove_abandoned_empty(entry);
    }
    if !snapshot_exists || !record_exists || entry.status != RegistryStatus::Bound {
        return Err("SourceSnapshotOrphaned: incomplete or unexpected files; slot retained".into());
    }
    let (record, intent) = inspect_bound_pair(entry, &destination, &snapshot_path, &record_path)?;
    let mut record_file = crate::windows_file_identity::VerifiedFile::open(&record_path, false)?;
    let current_record_bytes = record_file.read_all()?;
    let current_record: SnapshotRecord = serde_json::from_slice(&current_record_bytes)
        .map_err(|error| format!("SourceSnapshotCorrupt: snapshot record: {error}"))?;
    if current_record != record {
        return Err("SourceSnapshotMismatch: record changed before recovery".into());
    }
    let archived = crate::archive::ArchiveStore::scan(&destination)?;
    let reservation_path = destination
        .join(".whisper-publications")
        .join("reservations")
        .join(format!(
            "{:032x}-{}.json",
            entry.job_id.0,
            entry.generation.get()
        ));
    let has_publication_reservation = file_exists_strict(&reservation_path)?;
    let source_sha256 = hex_bytes(&record.source_sha256);
    let _redundant_source_guard = if has_publication_reservation
        || archive_blocks_snapshot_recovery(
            &archived,
            entry.job_id,
            entry.generation,
            Some(&source_sha256),
        ) {
        Some(
            redundant_pending_source_guard(
                &destination,
                entry,
                &record,
                &archived,
                &reservation_path,
            )?
            .ok_or_else(|| {
                "SourceSnapshotMismatch: pending/source identity changed before recovery".to_owned()
            })?,
        )
    } else {
        None
    };
    let snapshot = open_disposition_snapshot(&snapshot_path, &record)?;
    let fresh = RetirementIntent {
        snapshot: recovery_artifact(&snapshot_path, &snapshot, &record.source_sha256)?,
        record: recovery_artifact_from_file(&record_path, &mut record_file)?,
        source_record: Some(record.clone()),
        source_record_bytes: Some(current_record_bytes),
    };
    if fresh != intent {
        return Err("SourceSnapshotMismatch: file identity changed before recovery".into());
    }
    registry.begin_retirement(&record, fresh.clone())?;
    delete_verified_handle(snapshot, &fresh.snapshot)?;
    delete_verified_handle(record_file, &fresh.record)?;
    registry.finish_retirement(&record)
}

fn resume_retirement(
    registry: &SnapshotRegistry,
    entry: &RegistryEntry,
    allow_exact_pending_source: bool,
) -> Result<(), String> {
    let intent = entry
        .retirement
        .as_ref()
        .ok_or_else(|| "SourceSnapshotRegistryCorrupt: retirement intent missing".to_owned())?;
    let record_path = PathBuf::from(&intent.record.path);
    let snapshot_path = PathBuf::from(&intent.snapshot.path);
    check_pending_intent(intent)?;
    let destination = fs::canonicalize(&entry.destination)
        .map_err(|error| format!("StorageUnavailable: destination: {error}"))?;
    if destination.to_string_lossy() != entry.destination {
        return Err("CleanupUnsafePath: destination identity changed".into());
    }
    let archived = crate::archive::ArchiveStore::scan(&destination)?;
    let reservation_path = destination
        .join(".whisper-publications")
        .join("reservations")
        .join(format!(
            "{:032x}-{}.json",
            entry.job_id.0,
            entry.generation.get()
        ));
    let reservation_exists = file_exists_strict(&reservation_path)?;
    let archive_blocks = archive_blocks_snapshot_recovery(
        &archived,
        entry.job_id,
        entry.generation,
        Some(&intent.snapshot.sha256),
    );
    let _pending_source_guard = if reservation_exists || archive_blocks {
        if !allow_exact_pending_source {
            return Err("Archive ou pending encore présent pour cette réservation".into());
        }
        let record = retirement_source_record(&destination, entry, intent)?.ok_or_else(|| {
            "SourceSnapshotMismatch: retirement source identity unavailable".to_owned()
        })?;
        Some(
            redundant_pending_source_guard(
                &destination,
                entry,
                &record,
                &archived,
                &reservation_path,
            )?
            .ok_or_else(|| {
                "SourceSnapshotMismatch: pending, reservation or source changed before retirement"
                    .to_owned()
            })?,
        )
    } else {
        None
    };
    delete_path_if_identity_matches(&snapshot_path, &intent.snapshot)?;
    delete_path_if_identity_matches(&record_path, &intent.record)?;
    let record = intent
        .source_record
        .clone()
        .unwrap_or_else(|| legacy_retirement_record(entry, intent));
    registry.finish_retirement(&record)
}

fn retirement_source_record(
    destination: &Path,
    entry: &RegistryEntry,
    intent: &RetirementIntent,
) -> Result<Option<SnapshotRecord>, String> {
    let record_path = expected_record_path(destination, entry);
    if Path::new(&intent.record.path) != record_path {
        return Err("CleanupUnsafePath: retirement record path diverged".into());
    }
    let mut record = intent.source_record.clone();
    if file_exists_strict(&record_path)? {
        let mut record_file =
            crate::windows_file_identity::VerifiedFile::open(&record_path, false)?;
        if recovery_artifact_from_file(&record_path, &mut record_file)? != intent.record {
            return Err("CleanupIdentityChanged: retirement record changed".into());
        }
        let record_bytes = record_file.read_all()?;
        if intent
            .source_record_bytes
            .as_ref()
            .is_some_and(|expected| expected != &record_bytes)
        {
            return Err("SourceSnapshotMismatch: retirement record bytes changed".into());
        }
        let persisted: SnapshotRecord = serde_json::from_slice(&record_bytes)
            .map_err(|error| format!("SourceSnapshotCorrupt: snapshot record: {error}"))?;
        if record
            .as_ref()
            .is_some_and(|expected| expected != &persisted)
        {
            return Err("SourceSnapshotMismatch: embedded retirement record diverged".into());
        }
        record = Some(persisted);
    } else if let Some(record) = record.as_ref() {
        let bytes = intent.source_record_bytes.as_ref().ok_or_else(|| {
            "SourceSnapshotMismatch: exact retirement record bytes unavailable".to_owned()
        })?;
        let digest = hex_bytes(&Sha256::digest(bytes));
        if digest != intent.record.sha256 {
            return Err("SourceSnapshotMismatch: embedded retirement record hash diverged".into());
        }
        let persisted: SnapshotRecord = serde_json::from_slice(bytes)
            .map_err(|error| format!("SourceSnapshotCorrupt: retirement record: {error}"))?;
        if &persisted != record {
            return Err("SourceSnapshotMismatch: embedded retirement record diverged".into());
        }
    }
    let Some(record) = record else {
        return Ok(None);
    };
    let expected_snapshot = expected_snapshot_path(destination, entry);
    if file_exists_strict(&expected_snapshot)? {
        validate_record(
            &record,
            destination,
            &expected_snapshot,
            entry.job_id,
            entry.generation,
        )?;
    } else if intent.source_record.as_ref() != Some(&record)
        || (record.version == 1 && record.strategy != legacy_strategy())
        || (record.version == 2
            && record.strategy
                != whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1)
        || !matches!(record.version, 1 | 2)
        || record.job_id != entry.job_id
        || record.generation != entry.generation
        || record.destination != destination.to_string_lossy()
        || Path::new(&record.snapshot_path) != expected_snapshot
    {
        return Err("SourceSnapshotMismatch: retirement record does not match its intent".into());
    }
    if record.destination != entry.destination
        || record.snapshot_path != intent.snapshot.path
        || hex_bytes(&record.source_sha256) != intent.snapshot.sha256
    {
        return Err("SourceSnapshotMismatch: retirement source identity diverged".into());
    }
    Ok(Some(record))
}

fn legacy_retirement_record(entry: &RegistryEntry, intent: &RetirementIntent) -> SnapshotRecord {
    SnapshotRecord {
        version: 1,
        job_id: entry.job_id,
        generation: entry.generation,
        destination: entry.destination.clone(),
        original_path: String::new(),
        snapshot_path: intent.snapshot.path.clone(),
        source_len: 0,
        source_sha256: [0; 32],
        source_samples: 0,
        strategy: legacy_strategy(),
    }
}

fn check_pending_intent(intent: &RetirementIntent) -> Result<(), String> {
    for artifact in [&intent.snapshot, &intent.record] {
        let path = PathBuf::from(&artifact.path);
        if file_exists_strict(&path)? {
            let mut file = crate::windows_file_identity::VerifiedFile::open(&path, false)?;
            let actual = recovery_artifact_from_file(&path, &mut file)?;
            if actual != *artifact {
                return Err("CleanupIdentityChanged: fichier remplacé depuis l'intention".into());
            }
        }
    }
    Ok(())
}

fn delete_verified_handle(
    file: crate::windows_file_identity::VerifiedFile,
    artifact: &RecoveryArtifact,
) -> Result<(), String> {
    let identity = file_identity(artifact);
    // The caller hashed this same handle before persisting the retirement
    // intent. It has share-read-only semantics, so its contents cannot change
    // between that proof and this same-handle disposition.
    file.mark_delete(&identity, None)
}

fn delete_path_if_identity_matches(path: &Path, expected: &RecoveryArtifact) -> Result<(), String> {
    if !file_exists_strict(path)? {
        return Ok(());
    }
    let mut file = crate::windows_file_identity::VerifiedFile::open(path, false)?;
    let actual = recovery_artifact_from_file(path, &mut file)?;
    if actual != *expected {
        return Err("CleanupIdentityChanged: fichier remplacé depuis l'intention".into());
    }
    delete_verified_handle(file, expected)
}

fn recovery_artifact(
    path: &Path,
    file: &crate::windows_file_identity::VerifiedFile,
    sha256: &[u8; 32],
) -> Result<RecoveryArtifact, String> {
    Ok(RecoveryArtifact {
        path: path.to_string_lossy().into_owned(),
        volume_serial: file.identity.volume_serial,
        file_id: file.identity.file_id,
        is_directory: file.identity.is_directory,
        sha256: hex_bytes(sha256),
    })
}

fn recovery_artifact_from_file(
    path: &Path,
    file: &mut crate::windows_file_identity::VerifiedFile,
) -> Result<RecoveryArtifact, String> {
    Ok(RecoveryArtifact {
        path: path.to_string_lossy().into_owned(),
        volume_serial: file.identity.volume_serial,
        file_id: file.identity.file_id,
        is_directory: file.identity.is_directory,
        sha256: file.sha256()?,
    })
}

fn file_identity(artifact: &RecoveryArtifact) -> crate::windows_file_identity::FileIdentity {
    crate::windows_file_identity::FileIdentity {
        volume_serial: artifact.volume_serial,
        file_id: artifact.file_id,
        is_directory: artifact.is_directory,
    }
}

fn expected_snapshot_path(destination: &Path, entry: &RegistryEntry) -> PathBuf {
    destination.join(SNAPSHOT_DIRECTORY).join(format!(
        "{}.snapshot",
        snapshot_stem(entry.job_id, entry.generation)
    ))
}

fn expected_record_path(destination: &Path, entry: &RegistryEntry) -> PathBuf {
    destination.join(SNAPSHOT_DIRECTORY).join(format!(
        "{}.json",
        snapshot_stem(entry.job_id, entry.generation)
    ))
}

fn file_exists_strict(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(format!(
            "StorageUnavailable: inspect {}: {error}",
            path.display()
        )),
    }
}

fn hex_bytes(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut value = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        value.push(HEX[(byte >> 4) as usize] as char);
        value.push(HEX[(byte & 15) as usize] as char);
    }
    value
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
                RegistryStatus::Retiring => Err("SourceSnapshotRegistry: retirement requires explicit recovery".into()),
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

    fn read_entries(&self) -> Result<Vec<RegistryEntry>, String> {
        let path = self.root.join(REGISTRY_FILE);
        let lock_path = self.root.join(REGISTRY_LOCK_FILE);
        let lock_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&lock_path)
            .map_err(|error| format!("SourceSnapshotRegistryUnavailable: open lock: {error}"))?;
        lock_file.lock().map_err(|error| {
            format!("SourceSnapshotRegistryUnavailable: lock registry: {error}")
        })?;
        let entries = match fs::read(path) {
            Ok(bytes) if bytes.is_empty() => {
                return Err("SourceSnapshotRegistryCorrupt: empty registry file".into());
            }
            Ok(bytes) => serde_json::from_slice::<Vec<RegistryEntry>>(&bytes)
                .map_err(|error| format!("SourceSnapshotRegistryCorrupt: {error}"))?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(error) => return Err(format!("SourceSnapshotRegistryUnavailable: read: {error}")),
        };
        validate_registry(&entries)?;
        Ok(entries)
    }

    fn begin_retirement(
        &self,
        record: &SnapshotRecord,
        intent: RetirementIntent,
    ) -> Result<(), String> {
        self.update(|entries| {
            let entry = entries
                .iter_mut()
                .find(|entry| {
                    entry.job_id == record.job_id
                        && entry.generation == record.generation
                        && entry.destination == record.destination
                        && entry.snapshot_path.as_deref() == Some(record.snapshot_path.as_str())
                })
                .ok_or_else(|| {
                    "SourceSnapshotRegistry: binding changed before retirement".to_owned()
                })?;
            match entry.status {
                RegistryStatus::Bound => {
                    entry.status = RegistryStatus::Retiring;
                    entry.retirement = Some(intent);
                    Ok(())
                }
                RegistryStatus::Retiring if entry.retirement.as_ref() == Some(&intent) => Ok(()),
                _ => Err("SourceSnapshotRegistry: binding is not eligible for retirement".into()),
            }
        })
    }

    fn remove_abandoned_empty(&self, expected: &RegistryEntry) -> Result<(), String> {
        self.update(|entries| {
            let index = entries
                .iter()
                .position(|entry| entry == expected)
                .ok_or_else(|| {
                    "SourceSnapshotRegistry: reservation changed during recovery".to_owned()
                })?;
            entries.remove(index);
            Ok(())
        })
    }

    fn finish_retirement(&self, record: &SnapshotRecord) -> Result<(), String> {
        self.update(|entries| {
            let index = entries
                .iter()
                .position(|entry| {
                    entry.job_id == record.job_id
                        && entry.generation == record.generation
                        && entry.destination == record.destination
                        && entry.snapshot_path.as_deref() == Some(record.snapshot_path.as_str())
                        && entry.status == RegistryStatus::Retiring
                        && entry.retirement.is_some()
                })
                .ok_or_else(|| {
                    "SourceSnapshotRegistry: durable retirement intent is missing".to_owned()
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
            .truncate(false)
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
        }) || (entry.status == RegistryStatus::Bound
            && (entry.snapshot_path.is_none() || entry.retirement.is_some()))
            || (entry.status == RegistryStatus::Reserved
                && (entry.snapshot_path.is_some() || entry.retirement.is_some()))
            || (entry.status == RegistryStatus::Retiring
                && (entry.snapshot_path.is_none() || entry.retirement.is_none()))
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
        retirement: None,
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
        if artifact.exists()
            && let Err(error) = fs::remove_file(artifact)
        {
            return format!("{cause}; SourceSnapshotOrphaned: {artifact:?}: {error}");
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

fn lock_operation(registry_root: &Path) -> Result<File, String> {
    fs::create_dir_all(registry_root)
        .map_err(|error| format!("SourceSnapshotRegistryUnavailable: create registry: {error}"))?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(registry_root.join(OPERATION_LOCK_FILE))
        .map_err(|error| {
            format!("SourceSnapshotRegistryUnavailable: open operation lock: {error}")
        })?;
    file.lock()
        .map_err(|error| format!("SourceSnapshotRegistryUnavailable: lock operation: {error}"))?;
    Ok(file)
}

fn recovery_publication_guard(
    destination: &Path,
    persisted_destination: &str,
) -> Result<Option<File>, String> {
    let canonical = match fs::canonicalize(destination) {
        Ok(path) if path.to_string_lossy() == persisted_destination => path,
        _ => return Ok(None),
    };
    let path = canonical.join(".whisper-publications.lock");
    let file = match OpenOptions::new().read(true).write(true).open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Ok(None),
    };
    match file.try_lock() {
        Ok(()) => Ok(Some(file)),
        Err(std::fs::TryLockError::WouldBlock) => Ok(None),
        Err(std::fs::TryLockError::Error(_)) => Ok(None),
    }
}

fn recovery_operation_guard(registry_root: &Path) -> Result<OperationGuardProbe, String> {
    let file = match OpenOptions::new()
        .read(true)
        .write(true)
        .open(registry_root.join(OPERATION_LOCK_FILE))
    {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(OperationGuardProbe::Missing);
        }
        Err(error) => {
            return Err(format!(
                "SourceSnapshotRegistryUnavailable: inspect operation lock: {error}"
            ));
        }
    };
    match file.try_lock() {
        Ok(()) => Ok(OperationGuardProbe::Acquired { _guard: file }),
        Err(std::fs::TryLockError::WouldBlock) => Ok(OperationGuardProbe::Busy),
        Err(std::fs::TryLockError::Error(error)) => Err(format!(
            "SourceSnapshotRegistryUnavailable: inspect operation lock: {error}"
        )),
    }
}

fn operation_guard_state(probe: &OperationGuardProbe) -> (bool, bool) {
    match probe {
        OperationGuardProbe::Acquired { .. } => (true, false),
        OperationGuardProbe::Busy => (false, true),
        OperationGuardProbe::Missing => (false, false),
    }
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
    if (record.version == 1 && record.strategy != legacy_strategy())
        || (record.version == 2
            && record.strategy
                != whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1)
        || !matches!(record.version, 1 | 2)
        || record.job_id != job_id
        || record.generation != generation
        || record.destination != destination.to_string_lossy()
        || Path::new(&record.snapshot_path) != expected_snapshot
        || !metadata.is_file()
        || metadata.len() != record.source_len
    {
        return Err("SourceSnapshotCorrupt: durable binding does not match its operation".into());
    }
    Ok(())
}

fn snapshot_schema(strategy: whisper_core::parakeet_session::TranscriptionStrategy) -> u8 {
    match strategy {
        whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1 => 1,
        whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1 => 2,
    }
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
        .map_err(|error| format!("StorageUnavailable: ouvrir la copie audio: {error}"))?;
    let metadata = fs::metadata(path)
        .map_err(|error| format!("SourceSnapshotUnavailable: snapshot metadata: {error}"))?;
    if !metadata.is_file() || metadata.len() != record.source_len {
        return Err("SourceSnapshotCorrupt: snapshot size changed after capture".into());
    }
    let mut hasher = Sha256::new();
    let copied = copy_and_hash(&mut file, &mut io::sink(), &mut hasher)
        .map_err(|error| format!("SourceSnapshotUnavailable: vérifier la copie audio: {error}"))?;
    if copied != record.source_len || hasher.finalize().as_slice() != record.source_sha256 {
        return Err("SourceSnapshotCorrupt: snapshot digest changed after capture".into());
    }
    use std::io::{Seek, SeekFrom};
    file.seek(SeekFrom::Start(0)).map_err(|error| {
        format!("SourceSnapshotUnavailable: repositionner la copie audio: {error}")
    })?;
    Ok(file)
}

fn open_disposition_snapshot(
    path: &Path,
    record: &SnapshotRecord,
) -> Result<crate::windows_file_identity::VerifiedFile, String> {
    let mut file = crate::windows_file_identity::VerifiedFile::open(path, false)?;
    let metadata = fs::metadata(path)
        .map_err(|error| format!("SourceSnapshotUnavailable: snapshot metadata: {error}"))?;
    if !metadata.is_file() || metadata.len() != record.source_len {
        return Err("SourceSnapshotCorrupt: snapshot size changed before retirement".into());
    }
    if file.sha256()? != hex_bytes(&record.source_sha256) {
        return Err("SourceSnapshotCorrupt: snapshot digest changed before retirement".into());
    }
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

#[cfg(test)]
mod recovery_tests {
    use super::*;
    use whisper_core::ports::ImportRequest;

    #[path = "retiring_archive_test.rs"]
    mod retiring_archive_test;

    fn root() -> PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        PathBuf::from(
            std::env::var_os("PARAKEET_L2_ARCHIVE_ROOT")
                .unwrap_or_else(|| std::env::temp_dir().into_os_string()),
        )
        .join("rtc-05-source-binding-unit")
        .join(format!(
            "{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ))
    }

    fn write_wav(path: &Path) {
        let data_len = 32_000_u32 * 2;
        let mut file = File::create(path).unwrap();
        file.write_all(b"RIFF").unwrap();
        file.write_all(&(36 + data_len).to_le_bytes()).unwrap();
        file.write_all(b"WAVEfmt ").unwrap();
        file.write_all(&16_u32.to_le_bytes()).unwrap();
        file.write_all(&1_u16.to_le_bytes()).unwrap();
        file.write_all(&1_u16.to_le_bytes()).unwrap();
        file.write_all(&16_000_u32.to_le_bytes()).unwrap();
        file.write_all(&32_000_u32.to_le_bytes()).unwrap();
        file.write_all(&2_u16.to_le_bytes()).unwrap();
        file.write_all(&16_u16.to_le_bytes()).unwrap();
        file.write_all(b"data").unwrap();
        file.write_all(&data_len.to_le_bytes()).unwrap();
        file.write_all(&vec![0_u8; data_len as usize]).unwrap();
    }

    fn request(destination: &Path, source: &Path, id: u128) -> ImportRequest {
        ImportRequest {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
            job_id: JobId(id),
            generation: Generation::first(),
            source_path: source.to_string_lossy().into_owned(),
            source_sha256: None,
            source_samples: None,
            model_path: "unused-model.bin".into(),
            model_sha256: [0; 32],
            destination: destination.to_string_lossy().into_owned(),
            config: whisper_core::JobConfig {
                language: whisper_core::LanguageChoice::Automatic,
                compute: whisper_core::ComputeChoice::Cpu,
            },
        }
    }

    fn add_matching_pending_and_reservation(
        destination: &Path,
        request: &mut ImportRequest,
        source_sha256: &[u8; 32],
        source_samples: u64,
    ) -> (PathBuf, PathBuf, Vec<u8>, Vec<u8>) {
        let source_sha256_hex = hex_bytes(source_sha256);
        request.source_sha256 = Some(*source_sha256);
        request.source_samples = Some(source_samples);
        {
            let _publication_lock =
                crate::archive::ArchiveStore::lock_publications(destination).unwrap();
            crate::archive::ArchiveStore::reserve_create_only_new_import(
                destination,
                request,
                &source_sha256_hex,
            )
            .unwrap();
        }
        let directory = destination
            .join("transcriptions")
            .join(format!("{:032x}", request.job_id.0))
            .join(request.generation.get().to_string());
        std::fs::create_dir_all(&directory).unwrap();
        let model = &whisper_core::model_registry::MODELS[0].artifact;
        let pending = directory.join("pending.json");
        std::fs::write(
            &pending,
            serde_json::to_vec(&serde_json::json!({
                "version": 1,
                "job_id": request.job_id.0,
                "generation": request.generation.get(),
                "source_path": request.source_path,
                "source_sha256": source_sha256_hex,
                "source_samples": source_samples,
                "model_path": model.file_name,
                "model_sha256": model.sha256,
                "model_size": model.size_bytes,
            }))
            .unwrap(),
        )
        .unwrap();
        let reservation = destination
            .join(".whisper-publications")
            .join("reservations")
            .join(format!(
                "{:032x}-{}.json",
                request.job_id.0,
                request.generation.get()
            ));
        let pending_bytes = std::fs::read(&pending).unwrap();
        let reservation_bytes = std::fs::read(&reservation).unwrap();
        (pending, reservation, pending_bytes, reservation_bytes)
    }

    #[test]
    fn snapshot_v2_preserves_strategy_and_rejects_request_mismatch() {
        let root = root();
        let destination = root.join("destination");
        std::fs::create_dir_all(&destination).unwrap();
        let source = root.join("source.wav");
        write_wav(&source);
        let registry_root = root.join("settings/reprocessing-snapshots");
        let mut request = request(&destination, &source, 2001);
        request.strategy = whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1;
        let binding = SourceBinding::acquire(&request, &registry_root).unwrap();
        let stored: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&binding.record_path).unwrap()).unwrap();
        assert_eq!(stored["version"], 2);
        assert_eq!(stored["strategy"], "bounded_windows_v1");
        assert_eq!(
            binding.apply_to_request(&request).unwrap().strategy,
            request.strategy
        );

        let mut mismatch = request.clone();
        mismatch.strategy = whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1;
        assert!(
            binding
                .apply_to_request(&mismatch)
                .unwrap_err()
                .contains("strategy differs")
        );
        drop(binding);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn retirement_intent_replays_after_each_disposition_boundary() {
        for (index, delete_snapshot, delete_record) in
            [(0_u128, false, false), (1, true, false), (2, true, true)]
        {
            let root = root();
            let destination = root.join("destination");
            std::fs::create_dir_all(&destination).unwrap();
            let source = root.join("source.wav");
            write_wav(&source);
            let registry_root = root.join("settings/reprocessing-snapshots");
            let binding = SourceBinding::acquire(
                &request(&destination, &source, 1000 + index),
                &registry_root,
            )
            .unwrap();
            let snapshot_path = binding.snapshot_path.clone();
            let record_path = binding.record_path.clone();
            let record = binding.record.clone();
            drop(binding);

            let registry = SnapshotRegistry::new(&registry_root);
            let snapshot = open_disposition_snapshot(&snapshot_path, &record).unwrap();
            let mut record_file =
                crate::windows_file_identity::VerifiedFile::open(&record_path, false).unwrap();
            let record_bytes = record_file.read_all().unwrap();
            let intent = RetirementIntent {
                snapshot: recovery_artifact(&snapshot_path, &snapshot, &record.source_sha256)
                    .unwrap(),
                record: recovery_artifact_from_file(&record_path, &mut record_file).unwrap(),
                source_record: Some(record.clone()),
                source_record_bytes: Some(record_bytes),
            };
            registry.begin_retirement(&record, intent.clone()).unwrap();
            drop(snapshot);
            drop(record_file);

            if delete_snapshot {
                delete_path_if_identity_matches(&snapshot_path, &intent.snapshot).unwrap();
            }
            if delete_record {
                delete_path_if_identity_matches(&record_path, &intent.record).unwrap();
            }

            let (freed, remaining) = SourceBinding::recover_abandoned(&registry_root, &[]).unwrap();
            assert_eq!(freed, 1);
            assert!(remaining.is_empty());
            assert!(!snapshot_path.exists());
            assert!(!record_path.exists());
            assert!(registry.read_entries().unwrap().is_empty());
            std::fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn explicit_recovery_retiring_pending_replays_after_each_disposition_boundary() {
        for (index, delete_snapshot, delete_record) in
            [(0_u128, false, false), (1, true, false), (2, true, true)]
        {
            let root = root();
            let destination = root.join("destination");
            std::fs::create_dir_all(&destination).unwrap();
            let source = root.join("source.wav");
            write_wav(&source);
            let registry_root = root.join("settings/reprocessing-snapshots");
            let mut import = request(&destination, &source, 1100 + index);
            let binding = SourceBinding::acquire(&import, &registry_root).unwrap();
            let snapshot_path = binding.snapshot_path.clone();
            let record_path = binding.record_path.clone();
            let record = binding.record.clone();
            let (pending_path, reservation_path, pending_before, reservation_before) =
                add_matching_pending_and_reservation(
                    &destination,
                    &mut import,
                    &record.source_sha256,
                    record.source_samples,
                );
            let source_before = std::fs::read(&source).unwrap();
            drop(binding);

            let registry = SnapshotRegistry::new(&registry_root);
            let snapshot = open_disposition_snapshot(&snapshot_path, &record).unwrap();
            let mut record_file =
                crate::windows_file_identity::VerifiedFile::open(&record_path, false).unwrap();
            let record_bytes = record_file.read_all().unwrap();
            let intent = RetirementIntent {
                snapshot: recovery_artifact(&snapshot_path, &snapshot, &record.source_sha256)
                    .unwrap(),
                record: recovery_artifact_from_file(&record_path, &mut record_file).unwrap(),
                source_record: Some(record.clone()),
                source_record_bytes: Some(record_bytes),
            };
            registry.begin_retirement(&record, intent.clone()).unwrap();
            drop(snapshot);
            drop(record_file);

            if delete_snapshot {
                delete_path_if_identity_matches(&snapshot_path, &intent.snapshot).unwrap();
            }
            if delete_record {
                delete_path_if_identity_matches(&record_path, &intent.record).unwrap();
            }

            let inspected =
                SourceBinding::inspect_recovery_candidates(&registry_root, &[]).unwrap();
            assert_eq!(inspected.len(), 1);
            assert!(inspected[0].recoverable, "{:?}", inspected[0].reason);
            let (freed, remaining) = SourceBinding::recover_abandoned(&registry_root, &[]).unwrap();
            assert_eq!(freed, 1);
            assert!(remaining.is_empty());
            assert!(!snapshot_path.exists());
            assert!(!record_path.exists());
            assert_eq!(std::fs::read(&source).unwrap(), source_before);
            assert_eq!(std::fs::read(&pending_path).unwrap(), pending_before);
            assert_eq!(
                std::fs::read(&reservation_path).unwrap(),
                reservation_before
            );
            assert!(registry.read_entries().unwrap().is_empty());
            std::fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn explicit_recovery_retiring_historical_record_replays_after_sidecar_removal() {
        let root = root();
        let destination = root.join("destination");
        std::fs::create_dir_all(&destination).unwrap();
        let source = root.join("source.wav");
        write_wav(&source);
        let registry_root = root.join("settings/reprocessing-snapshots");
        let mut import = request(&destination, &source, 1199);
        let binding = SourceBinding::acquire(&import, &registry_root).unwrap();
        let snapshot_path = binding.snapshot_path.clone();
        let record_path = binding.record_path.clone();
        let record = binding.record.clone();
        let expected_record = record.clone();
        drop(binding);

        let mut historical_record: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&record_path).unwrap()).unwrap();
        historical_record
            .as_object_mut()
            .unwrap()
            .remove("strategy");
        let record_bytes = serde_json::to_vec(&historical_record).unwrap();
        std::fs::write(&record_path, &record_bytes).unwrap();
        let record: SnapshotRecord = serde_json::from_slice(&record_bytes).unwrap();
        assert_eq!(record, expected_record);

        let (pending_path, reservation_path, pending_before, reservation_before) =
            add_matching_pending_and_reservation(
                &destination,
                &mut import,
                &record.source_sha256,
                record.source_samples,
            );
        let source_before = std::fs::read(&source).unwrap();
        let registry = SnapshotRegistry::new(&registry_root);
        let snapshot = open_disposition_snapshot(&snapshot_path, &record).unwrap();
        let mut record_file =
            crate::windows_file_identity::VerifiedFile::open(&record_path, false).unwrap();
        let intent = RetirementIntent {
            snapshot: recovery_artifact(&snapshot_path, &snapshot, &record.source_sha256).unwrap(),
            record: recovery_artifact_from_file(&record_path, &mut record_file).unwrap(),
            source_record: Some(record.clone()),
            source_record_bytes: Some(record_file.read_all().unwrap()),
        };
        registry.begin_retirement(&record, intent.clone()).unwrap();
        drop(snapshot);
        drop(record_file);
        delete_path_if_identity_matches(&snapshot_path, &intent.snapshot).unwrap();
        delete_path_if_identity_matches(&record_path, &intent.record).unwrap();

        let (freed, remaining) = SourceBinding::recover_abandoned(&registry_root, &[]).unwrap();
        assert_eq!(freed, 1);
        assert!(remaining.is_empty());
        assert!(!snapshot_path.exists());
        assert!(!record_path.exists());
        assert_eq!(std::fs::read(&source).unwrap(), source_before);
        assert_eq!(std::fs::read(&pending_path).unwrap(), pending_before);
        assert_eq!(
            std::fs::read(&reservation_path).unwrap(),
            reservation_before
        );
        assert!(registry.read_entries().unwrap().is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn recovery_refuses_reserved_operation_while_acquire_holds_the_cross_process_lock() {
        let root = root();
        let destination = root.join("destination");
        std::fs::create_dir_all(&destination).unwrap();
        let source = root.join("source.wav");
        write_wav(&source);
        let registry_root = root.join("settings/reprocessing-snapshots");
        let request = request(&destination, &source, 2001);
        let acquire_request = request.clone();
        let acquire_registry = registry_root.clone();
        let (partial_tx, partial_rx) = std::sync::mpsc::sync_channel(0);
        let (resume_tx, resume_rx) = std::sync::mpsc::sync_channel(0);
        let acquire = std::thread::spawn(move || {
            SourceBinding::acquire_with_hook(&acquire_request, &acquire_registry, |partial_path| {
                partial_tx.send(partial_path.to_path_buf()).unwrap();
                resume_rx.recv().unwrap();
            })
        });

        let partial_path = partial_rx.recv().unwrap();
        assert!(
            partial_path.is_file(),
            "acquire paused after creating partial bytes"
        );
        let inspected = SourceBinding::inspect_recovery_candidates(&registry_root, &[]).unwrap();
        assert_eq!(inspected.len(), 1);
        assert!(!inspected[0].recoverable);
        assert!(
            inspected[0]
                .reason
                .as_deref()
                .is_some_and(|reason| reason.contains("opération de copie est en cours"))
        );

        let (freed, remaining) = SourceBinding::recover_abandoned(&registry_root, &[]).unwrap();
        assert_eq!(freed, 0);
        assert_eq!(remaining.len(), 1);
        assert!(!remaining[0].recoverable);
        assert!(
            partial_path.is_file(),
            "recovery must preserve active partial bytes"
        );
        assert_eq!(
            SnapshotRegistry::new(&registry_root)
                .read_entries()
                .unwrap()
                .len(),
            1
        );

        resume_tx.send(()).unwrap();
        acquire.join().unwrap().unwrap().retire().unwrap();
        assert!(
            SnapshotRegistry::new(&registry_root)
                .read_entries()
                .unwrap()
                .is_empty()
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
