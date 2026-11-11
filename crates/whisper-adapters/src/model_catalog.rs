use std::{
    fs::File,
    io,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::SystemTime,
};

use same_file::Handle;
use sha2::{Digest, Sha256};
use whisper_core::{model_catalog::ModelAvailability, model_registry::MODELS};

static ARTIFACT_HASH_CACHE: OnceLock<ModelFileHashCache> = OnceLock::new();
static MODEL_HASH_FULL_READS: AtomicU64 = AtomicU64::new(0);
static ARTIFACT_HASH_FULL_READS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
struct CachedModelHash {
    canonical_path: PathBuf,
    identity: Arc<Handle>,
    size_bytes: u64,
    modified: SystemTime,
    sha256: Mutex<Option<[u8; 32]>>,
}

#[derive(Default)]
struct ModelFileHashCache {
    entries: Mutex<Vec<Arc<CachedModelHash>>>,
}

struct OpenModelFile {
    file: File,
    canonical_path: PathBuf,
    identity: Arc<Handle>,
    size_bytes: u64,
    modified: SystemTime,
}

impl OpenModelFile {
    fn open(path: &Path) -> io::Result<Self> {
        let canonical_path = path.canonicalize()?;
        let file = File::open(path)?;
        let metadata = file.metadata()?;
        if !metadata.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "model artifact is not a regular file",
            ));
        }
        let identity = Handle::from_file(file.try_clone()?)?;
        let canonical_file = File::open(&canonical_path)?;
        let canonical_identity = Handle::from_file(canonical_file)?;
        if identity != canonical_identity {
            return Err(io::Error::other(
                "artifact path changed while its canonical identity was being opened",
            ));
        }
        let modified = metadata.modified()?;
        Ok(Self {
            file,
            canonical_path,
            identity: Arc::new(identity),
            size_bytes: metadata.len(),
            modified,
        })
    }

    fn matches(&self, other: &Self) -> bool {
        self.identity == other.identity
            && self.canonical_path == other.canonical_path
            && self.size_bytes == other.size_bytes
            && self.modified == other.modified
    }
}

impl ModelFileHashCache {
    fn sha256(&self, path: &Path) -> io::Result<(u64, [u8; 32], bool)> {
        self.sha256_with(path, io::copy)
    }

    fn sha256_with(
        &self,
        path: &Path,
        read: impl FnOnce(&mut File, &mut Sha256) -> io::Result<u64>,
    ) -> io::Result<(u64, [u8; 32], bool)> {
        #[cfg(feature = "native-tests")]
        let open_started = std::time::Instant::now();
        let opened = OpenModelFile::open(path)?;
        #[cfg(feature = "native-tests")]
        let open_and_identity_ms = open_started.elapsed().as_secs_f64() * 1000.0;
        let slot = self.slot(&opened)?;
        let result = slot.sha256(path, opened, read);
        #[cfg(feature = "native-tests")]
        if let (Some(model), Ok((bytes, _, full_read))) = (catalog_model(path), &result) {
            crate::native_timing::record(
                "model_cache.lookup",
                serde_json::json!({
                    "model_id":format!("{:?}",model.artifact.id),
                    "model_name":model.display_name,
                    "path":path.to_string_lossy(),
                    "bytes":bytes,
                    "cache_hit":!full_read,
                    "open_and_identity_ms":open_and_identity_ms,
                }),
            );
        }
        result
    }

    fn slot(&self, opened: &OpenModelFile) -> io::Result<Arc<CachedModelHash>> {
        // Only immutable in-memory keys are examined under this index lock.
        // No caller may acquire a slot lock while retaining the index guard.
        let mut entries = self
            .entries
            .lock()
            .map_err(|_| io::Error::other("model hash cache index is poisoned"))?;
        if let Some(slot) = entries.iter().find(|entry| entry.matches(opened)) {
            return Ok(Arc::clone(slot));
        }
        let slot = Arc::new(CachedModelHash {
            canonical_path: opened.canonical_path.clone(),
            identity: Arc::clone(&opened.identity),
            size_bytes: opened.size_bytes,
            modified: opened.modified,
            sha256: Mutex::new(None),
        });
        entries.push(Arc::clone(&slot));
        Ok(slot)
    }

    fn full_read_count(&self, path: &Path) -> io::Result<u64> {
        let opened = OpenModelFile::open(path)?;
        let slot = {
            let entries = self
                .entries
                .lock()
                .map_err(|_| io::Error::other("model hash cache index is poisoned"))?;
            entries.iter().find(|entry| entry.matches(&opened)).cloned()
        };
        let Some(slot) = slot else { return Ok(0) };
        let cached = slot
            .sha256
            .lock()
            .map_err(|_| io::Error::other("model hash cache entry is poisoned"))?;
        opened.revalidate(path)?;
        Ok(u64::from(cached.is_some()))
    }
}

impl CachedModelHash {
    fn matches(&self, opened: &OpenModelFile) -> bool {
        self.canonical_path == opened.canonical_path
            && self.identity == opened.identity
            && self.size_bytes == opened.size_bytes
            && self.modified == opened.modified
    }

    fn sha256(
        &self,
        path: &Path,
        opened: OpenModelFile,
        read: impl FnOnce(&mut File, &mut Sha256) -> io::Result<u64>,
    ) -> io::Result<(u64, [u8; 32], bool)> {
        #[cfg(feature = "native-tests")]
        let lock_started = std::time::Instant::now();
        #[cfg(feature = "native-tests")]
        record_cache_lock(path, "model_cache.lock_wait_begin", 0.0);
        let mut cached = self
            .sha256
            .lock()
            .map_err(|_| io::Error::other("model hash cache entry is poisoned"))?;
        #[cfg(feature = "native-tests")]
        record_cache_lock(
            path,
            "model_cache.lock_acquired",
            lock_started.elapsed().as_secs_f64() * 1000.0,
        );
        // A caller may have waited for a previous calculation of this key.
        // Revalidate even on a cache hit, before returning its digest.
        if let Err(error) = opened.revalidate(path) {
            *cached = None;
            return Err(error);
        }
        if let Some(digest) = *cached {
            return Ok((opened.size_bytes, digest, false));
        }
        let (size, digest) = hash_opened_file(path, opened, read)?;
        *cached = Some(digest);
        Ok((size, digest, true))
    }
}

impl OpenModelFile {
    fn revalidate(&self, path: &Path) -> io::Result<()> {
        if !self.matches(&Self::open(path)?) {
            return Err(io::Error::other(
                "model identity changed while waiting for its SHA-256",
            ));
        }
        Ok(())
    }
}

fn hash_opened_file(
    path: &Path,
    mut opened: OpenModelFile,
    read: impl FnOnce(&mut File, &mut Sha256) -> io::Result<u64>,
) -> io::Result<(u64, [u8; 32])> {
    let mut hasher = Sha256::new();
    #[cfg(feature = "native-tests")]
    let hash_started = std::time::Instant::now();
    let bytes_read = read(&mut opened.file, &mut hasher)?;
    #[cfg(feature = "native-tests")]
    if let Some(model) = catalog_model(path) {
        crate::native_timing::record(
            "model_cache.full_read",
            serde_json::json!({
                "model_id":format!("{:?}",model.artifact.id),
                "model_name":model.display_name,
                "path":path.to_string_lossy(),
                "bytes_read":bytes_read,
                "duration_ms":hash_started.elapsed().as_secs_f64()*1000.0,
            }),
        );
    }
    #[cfg(feature = "native-tests")]
    let revalidation_started = std::time::Instant::now();
    let after_read_result = OpenModelFile::open(path);
    #[cfg(feature = "native-tests")]
    if let Some(model) = catalog_model(path) {
        crate::native_timing::record(
            "model_cache.revalidation",
            serde_json::json!({
                "model_id":format!("{:?}",model.artifact.id),
                "model_name":model.display_name,
                "path":path.to_string_lossy(),
                "duration_ms":revalidation_started.elapsed().as_secs_f64()*1000.0,
                "opened":after_read_result.is_ok(),
            }),
        );
    }
    let after_read = after_read_result?;
    let identity_stable = opened.matches(&after_read);
    #[cfg(feature = "native-tests")]
    if let Some(model) = catalog_model(path) {
        crate::native_timing::record(
            "model_cache.identity_result",
            serde_json::json!({
                "model_id":format!("{:?}",model.artifact.id),
                "path":path.to_string_lossy(),
                "identity_stable":identity_stable,
                "bytes_read":bytes_read,
                "expected_bytes":opened.size_bytes,
            }),
        );
    }
    if bytes_read != opened.size_bytes || !identity_stable {
        return Err(io::Error::other(
            "model identity changed while its SHA-256 was being computed",
        ));
    }

    let digest: [u8; 32] = hasher.finalize().into();
    Ok((opened.size_bytes, digest))
}

/// Returns the cached SHA-256 for a regular model file after checking the current
/// handle identity, size, and modification time. Changed identities are hashed again.
pub fn cached_model_sha256(path: &Path) -> io::Result<(u64, [u8; 32])> {
    let (size, sha256, full_read) = cached_file_sha256(path)?;
    if full_read {
        MODEL_HASH_FULL_READS.fetch_add(1, Ordering::Relaxed);
    }
    Ok((size, sha256))
}

/// Returns an execution artifact SHA-256 after revalidating its canonical path,
/// file identity, size, and modification time. Stable identities are not reread.
#[doc(hidden)]
pub fn cached_artifact_sha256(path: &Path) -> io::Result<(u64, [u8; 32])> {
    let (size, sha256, _) = cached_artifact_sha256_with_read(path)?;
    Ok((size, sha256))
}

#[doc(hidden)]
pub fn cached_artifact_sha256_with_read(path: &Path) -> io::Result<(u64, [u8; 32], bool)> {
    let (size, sha256, full_read) = cached_file_sha256(path)?;
    if full_read {
        ARTIFACT_HASH_FULL_READS.fetch_add(1, Ordering::Relaxed);
    }
    Ok((size, sha256, full_read))
}

fn cached_file_sha256(path: &Path) -> io::Result<(u64, [u8; 32], bool)> {
    ARTIFACT_HASH_CACHE
        .get_or_init(ModelFileHashCache::default)
        .sha256(path)
}

#[cfg(feature = "native-tests")]
fn record_cache_lock(path: &Path, event: &str, wait_ms: f64) {
    #[cfg(feature = "native-tests")]
    if let Some(model) = catalog_model(path) {
        crate::native_timing::record(
            event,
            serde_json::json!({
                "model_id":format!("{:?}",model.artifact.id),
                "model_name":model.display_name,
                "path":path.to_string_lossy(),
                "wait_ms":wait_ms,
            }),
        );
    }
}

#[cfg(feature = "native-tests")]
fn catalog_model(path: &Path) -> Option<&'static whisper_core::model_registry::ModelRecord> {
    let file_name = path.file_name()?;
    MODELS
        .iter()
        .find(|record| std::ffi::OsStr::new(record.artifact.file_name) == file_name)
}

/// Reports whether the current file identity has already incurred its full SHA read.
#[doc(hidden)]
pub fn model_sha256_full_read_count(path: &Path) -> io::Result<u64> {
    ARTIFACT_HASH_CACHE
        .get_or_init(ModelFileHashCache::default)
        .full_read_count(path)
}

/// Reports the full adapter SHA reads for the current identity of any execution file.
#[doc(hidden)]
pub fn artifact_sha256_full_read_count(path: &Path) -> io::Result<u64> {
    ARTIFACT_HASH_CACHE
        .get_or_init(ModelFileHashCache::default)
        .full_read_count(path)
}

/// Total complete SHA reads of worker and runtime files in this adapter process.
#[doc(hidden)]
pub fn artifact_sha256_full_read_total() -> u64 {
    ARTIFACT_HASH_FULL_READS.load(Ordering::Relaxed)
}

/// Captures the identity used to decide whether an execution artifact needs a new SHA.
#[derive(Debug, Eq, PartialEq)]
pub struct ArtifactFileIdentity {
    canonical_path: PathBuf,
    identity: Arc<Handle>,
    size_bytes: u64,
    modified: SystemTime,
}

pub fn artifact_file_identity(path: &Path) -> io::Result<ArtifactFileIdentity> {
    let opened = OpenModelFile::open(path)?;
    Ok(ArtifactFileIdentity {
        canonical_path: opened.canonical_path,
        identity: opened.identity,
        size_bytes: opened.size_bytes,
        modified: opened.modified,
    })
}

/// Total number of complete model-file reads made by this process' adapter cache.
#[doc(hidden)]
pub fn model_sha256_full_read_total() -> u64 {
    MODEL_HASH_FULL_READS.load(Ordering::Relaxed)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CatalogEntry {
    pub id: whisper_core::model_catalog::ModelId,
    pub display_name: String,
    pub path: PathBuf,
    pub availability: ModelAvailability,
    pub expected_size_bytes: u64,
    pub observed_size_bytes: Option<u64>,
    pub expected_sha256: String,
    pub observed_sha256: Option<String>,
    pub diagnostic: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CatalogSnapshot {
    pub generation: u64,
    pub entries: Vec<CatalogEntry>,
}

/// Read-only inspection of the closed application model catalog.
pub struct LocalModelCatalog {
    root: PathBuf,
}

impl LocalModelCatalog {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn inspect(&self, generation: u64) -> CatalogSnapshot {
        let entries = MODELS
            .iter()
            .map(|record| {
                let path = self.root.join(record.artifact.file_name);
                inspect_entry(record, path)
            })
            .collect();
        CatalogSnapshot {
            generation,
            entries,
        }
    }

    pub(crate) fn inspect_requested_model(
        &self,
        path: &str,
        known_historical_turbo_path: &str,
    ) -> Result<(whisper_core::model_catalog::ModelId, CatalogSnapshot), String> {
        let id = self.resolve_requested_model(path, known_historical_turbo_path)?;
        Ok((id, self.inspect_model(id, 0)))
    }

    /// Resolves a registered path or the historical Turbo alias without reading a model.
    pub(crate) fn resolve_requested_model(
        &self,
        path: &str,
        known_historical_turbo_path: &str,
    ) -> Result<whisper_core::model_catalog::ModelId, String> {
        use whisper_core::{model_catalog::ModelId, model_selection_app::is_known_historical_path};

        if is_known_historical_path(path, known_historical_turbo_path) {
            return Ok(ModelId::WhisperLargeV3Turbo);
        }
        MODELS
            .iter()
            .find(|record| {
                let catalog_path = self.root.join(record.artifact.file_name);
                let catalog_path = catalog_path.to_string_lossy();
                is_known_historical_path(path, catalog_path.as_ref())
            })
            .map(|record| record.artifact.id)
            .ok_or_else(|| {
                format!(
                    "ModelUnknown: chemin de modèle inconnu dans la file ou le checkpoint : {path}"
                )
            })
    }

    /// Observes only one registered model, preserving the requested generation.
    pub fn inspect_model(
        &self,
        id: whisper_core::model_catalog::ModelId,
        generation: u64,
    ) -> CatalogSnapshot {
        let record = MODELS
            .iter()
            .find(|record| record.artifact.id == id)
            .expect("every model id has a catalog record");
        let entry = inspect_entry(record, self.root.join(record.artifact.file_name));
        CatalogSnapshot {
            generation,
            entries: vec![entry],
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
}

impl CatalogSnapshot {
    pub fn engine_snapshot(
        &self,
        resources: Vec<whisper_core::engine_capabilities::ResourceObservation>,
    ) -> whisper_core::engine_capabilities::EngineSnapshot {
        whisper_core::model_registry::engine_snapshot(
            self.generation,
            |id| {
                self.entries
                    .iter()
                    .find(|entry| entry.id == id)
                    .map_or(ModelAvailability::Uninspected, |entry| entry.availability)
            },
            resources,
        )
    }

    /// Rebinds a durable historical path to the verified Test catalog path before dispatch.
    /// The durable queue/checkpoint remains untouched by this helper.
    pub fn bind_turbo_import_request(
        &self,
        request: &whisper_core::ports::ImportRequest,
        known_historical_turbo_path: &str,
    ) -> Result<whisper_core::ports::ImportRequest, String> {
        use whisper_core::model_catalog::ModelId;
        let id = self.resolve_model_identity(&request.model_path, known_historical_turbo_path)?;
        if id != ModelId::WhisperLargeV3Turbo {
            return Err("ModelNotQualified: ce modèle n'est pas exécutable dans L1b-1".into());
        }
        let path = self.ready_turbo_path(
            &request.model_path,
            request.model_sha256,
            known_historical_turbo_path,
        )?;
        let mut executable = request.clone();
        executable.model_path = path.to_string_lossy().into_owned();
        Ok(executable)
    }

    pub fn bind_turbo_live_request(
        &self,
        request: &whisper_core::ports::LiveRequest,
        known_historical_turbo_path: &str,
    ) -> Result<whisper_core::ports::LiveRequest, String> {
        use whisper_core::model_catalog::ModelId;
        let id = self.resolve_model_identity(&request.model_path, known_historical_turbo_path)?;
        if id != ModelId::WhisperLargeV3Turbo {
            return Err("ModelNotQualified: ce modèle n'est pas exécutable dans L1b-1".into());
        }
        let path = self.ready_turbo_path(
            &request.model_path,
            request.model_sha256,
            known_historical_turbo_path,
        )?;
        let mut executable = request.clone();
        executable.model_path = path.to_string_lossy().into_owned();
        Ok(executable)
    }

    fn ready_turbo_path(
        &self,
        requested_path: &str,
        requested_sha256: [u8; 32],
        known_historical_turbo_path: &str,
    ) -> Result<PathBuf, String> {
        use whisper_core::{model_catalog::ModelId, model_registry::model};
        let id = self.resolve_model_identity(requested_path, known_historical_turbo_path)?;
        if id != ModelId::WhisperLargeV3Turbo {
            return Err("ModelNotQualified: ce modèle n'est pas exécutable dans L1b-1".into());
        }
        let entry = self
            .entries
            .iter()
            .find(|entry| entry.id == id)
            .ok_or_else(|| "ModelMissing: modèle Turbo absent du catalogue".to_owned())?;
        if entry.availability != ModelAvailability::Ready {
            return Err(entry
                .diagnostic
                .clone()
                .unwrap_or_else(|| "ModelMissing: modèle Turbo indisponible".to_owned()));
        }
        let record = model(id);
        let (size, hash) = inspect_file(&entry.path).map_err(|error| {
            format!("ModelMissing: revalidation du catalogue impossible : {error}")
        })?;
        if size != record.artifact.size_bytes || hash != record.artifact.sha256 {
            return Err("ModelHashMismatch: modèle Turbo modifié après l'inventaire".into());
        }
        if requested_sha256 != decode_sha256(record.artifact.sha256)? {
            return Err("ModelHashMismatch: empreinte de la requête durable différente".into());
        }
        Ok(entry.path.clone())
    }

    pub fn resolve_model_identity(
        &self,
        path: &str,
        known_historical_turbo_path: &str,
    ) -> Result<whisper_core::model_catalog::ModelId, String> {
        use whisper_core::{model_catalog::ModelId, model_selection_app::is_known_historical_path};
        if is_known_historical_path(path, known_historical_turbo_path) {
            return Ok(ModelId::WhisperLargeV3Turbo);
        }
        self.entries
            .iter()
            .find(|entry| is_known_historical_path(path, &entry.path.to_string_lossy()))
            .map(|entry| entry.id)
            .ok_or_else(|| {
                format!(
                    "ModelUnknown: chemin de modèle inconnu dans la file ou le checkpoint : {path}"
                )
            })
    }
}

fn inspect_entry(
    record: &whisper_core::model_registry::ModelRecord,
    path: PathBuf,
) -> CatalogEntry {
    let expected_sha256 = record.artifact.sha256.to_owned();
    match inspect_file(&path) {
        Ok((size, hash)) if size == record.artifact.size_bytes && hash == expected_sha256 => {
            CatalogEntry {
                id: record.artifact.id,
                display_name: record.display_name.to_owned(),
                path,
                availability: ModelAvailability::Ready,
                expected_size_bytes: record.artifact.size_bytes,
                observed_size_bytes: Some(size),
                expected_sha256,
                observed_sha256: Some(hash),
                diagnostic: None,
            }
        }
        Ok((size, hash)) => CatalogEntry {
            id: record.artifact.id,
            display_name: record.display_name.to_owned(),
            path,
            availability: ModelAvailability::Corrupt,
            expected_size_bytes: record.artifact.size_bytes,
            observed_size_bytes: Some(size),
            expected_sha256,
            observed_sha256: Some(hash),
            diagnostic: Some("Taille ou SHA-256 différent du catalogue.".into()),
        },
        Err(error) if error.kind() == io::ErrorKind::NotFound => CatalogEntry {
            id: record.artifact.id,
            display_name: record.display_name.to_owned(),
            path,
            availability: ModelAvailability::Missing,
            expected_size_bytes: record.artifact.size_bytes,
            observed_size_bytes: None,
            expected_sha256,
            observed_sha256: None,
            diagnostic: Some("Fichier modèle absent.".into()),
        },
        Err(error) => CatalogEntry {
            id: record.artifact.id,
            display_name: record.display_name.to_owned(),
            path,
            availability: ModelAvailability::Corrupt,
            expected_size_bytes: record.artifact.size_bytes,
            observed_size_bytes: None,
            expected_sha256,
            observed_sha256: None,
            diagnostic: Some(format!("Lecture du modèle impossible : {error}")),
        },
    }
}

fn inspect_file(path: &Path) -> io::Result<(u64, String)> {
    let (size, digest) = cached_model_sha256(path)?;
    Ok((
        size,
        digest.iter().map(|byte| format!("{byte:02x}")).collect(),
    ))
}

fn decode_sha256(value: &str) -> Result<[u8; 32], String> {
    if value.len() != 64 {
        return Err("ModelCatalogCorrupt: SHA-256 de catalogue invalide".into());
    }
    let mut bytes = [0_u8; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)
            .map_err(|error| format!("ModelCatalogCorrupt: SHA-256 invalide : {error}"))?;
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod concurrency_tests;

#[cfg(test)]
mod execution_artifact_cache_tests {
    use super::{artifact_file_identity, artifact_sha256_full_read_count, cached_artifact_sha256};
    use std::{
        fs::{self, File, FileTimes},
        io::Write,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
        time::SystemTime,
    };

    static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

    struct TestRoot(PathBuf);

    impl TestRoot {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "parakeet-artifact-cache-{}-{}",
                std::process::id(),
                NEXT_ROOT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn execution_artifact_cache_keys_hardlink_paths_and_rehashes_replaced_identity() {
        let root = TestRoot::new();
        let file = root.0.join("worker.dll");
        fs::write(&file, b"alpha").unwrap();
        let alias = root.0.join("worker-hardlink.dll");
        fs::hard_link(&file, &alias).unwrap();

        let first = cached_artifact_sha256(&file).unwrap();
        assert_eq!(artifact_sha256_full_read_count(&file).unwrap(), 1);
        assert_eq!(cached_artifact_sha256(&alias).unwrap(), first);
        assert_eq!(artifact_sha256_full_read_count(&alias).unwrap(), 1);
        assert_eq!(artifact_sha256_full_read_count(&file).unwrap(), 1);

        let old_identity = artifact_file_identity(&file).unwrap();
        let old_metadata = fs::metadata(&file).unwrap();
        let old_size = old_metadata.len();
        let old_modified = old_metadata.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        let replacement = root.0.join("replacement.dll");
        let mut output = File::create(&replacement).unwrap();
        output.write_all(b"bravo").unwrap();
        output
            .set_times(FileTimes::new().set_modified(old_modified))
            .unwrap();
        drop(output);

        let parked = root.0.join("parked.dll");
        fs::rename(&file, &parked).unwrap();
        fs::rename(&replacement, &file).unwrap();
        let new_identity = artifact_file_identity(&file).unwrap();
        assert_ne!(
            old_identity, new_identity,
            "replacement must have a new file identity"
        );
        let new_metadata = fs::metadata(&file).unwrap();
        assert_eq!(new_metadata.len(), old_size);
        assert_eq!(new_metadata.modified().unwrap(), old_modified);
        let replaced = cached_artifact_sha256(&file).unwrap();
        assert_ne!(replaced.1, first.1);
        assert_eq!(artifact_sha256_full_read_count(&file).unwrap(), 1);
    }

    #[test]
    fn concurrent_execution_artifact_requests_hash_one_identity_once() {
        let root = TestRoot::new();
        let file = root.0.join("runtime.dll");
        fs::write(&file, vec![0x5a; 1024 * 1024]).unwrap();
        let workers = (0..12)
            .map(|_| {
                let file = file.clone();
                std::thread::spawn(move || cached_artifact_sha256(&file).unwrap())
            })
            .collect::<Vec<_>>();
        let hashes = workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect::<Vec<_>>();
        assert!(hashes.windows(2).all(|pair| pair[0] == pair[1]));
        assert_eq!(artifact_sha256_full_read_count(&file).unwrap(), 1);
    }
}
