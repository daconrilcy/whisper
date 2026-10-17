use std::time::Instant;
use std::{
    fs::File,
    io,
    path::{Path, PathBuf},
    sync::{
        Mutex, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::SystemTime,
};

use same_file::Handle;
use sha2::{Digest, Sha256};
use whisper_core::{model_catalog::ModelAvailability, model_registry::MODELS};

static MODEL_HASH_CACHE: OnceLock<Mutex<ModelFileHashCache>> = OnceLock::new();
static MODEL_HASH_FULL_READS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug)]
struct CachedModelHash {
    identity: Handle,
    size_bytes: u64,
    modified: SystemTime,
    sha256: [u8; 32],
    full_reads: u64,
}

#[derive(Default)]
struct ModelFileHashCache {
    entries: Vec<CachedModelHash>,
}

struct OpenModelFile {
    file: File,
    identity: Handle,
    size_bytes: u64,
    modified: SystemTime,
}

impl OpenModelFile {
    fn open(path: &Path) -> io::Result<Self> {
        let file = File::open(path)?;
        let metadata = file.metadata()?;
        if !metadata.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "model artifact is not a regular file",
            ));
        }
        let identity = Handle::from_file(file.try_clone()?)?;
        let modified = metadata.modified()?;
        Ok(Self {
            file,
            identity,
            size_bytes: metadata.len(),
            modified,
        })
    }

    fn matches(&self, other: &Self) -> bool {
        self.identity == other.identity
            && self.size_bytes == other.size_bytes
            && self.modified == other.modified
    }
}

impl ModelFileHashCache {
    fn sha256(&mut self, path: &Path) -> io::Result<(u64, [u8; 32])> {
        let mut opened = OpenModelFile::open(path)?;
        if let Some(entry) = self.entries.iter().find(|entry| {
            entry.identity == opened.identity
                && entry.size_bytes == opened.size_bytes
                && entry.modified == opened.modified
        }) {
            return Ok((entry.size_bytes, entry.sha256));
        }

        let mut hasher = Sha256::new();
        let bytes_read = io::copy(&mut opened.file, &mut hasher)?;
        let after_read = OpenModelFile::open(path)?;
        if bytes_read != opened.size_bytes || !opened.matches(&after_read) {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "model identity changed while its SHA-256 was being computed",
            ));
        }

        let digest: [u8; 32] = hasher.finalize().into();
        let size_bytes = opened.size_bytes;
        self.entries.push(CachedModelHash {
            identity: opened.identity,
            size_bytes,
            modified: opened.modified,
            sha256: digest,
            full_reads: 1,
        });
        MODEL_HASH_FULL_READS.fetch_add(1, Ordering::Relaxed);
        Ok((size_bytes, digest))
    }

    fn full_read_count(&self, path: &Path) -> io::Result<u64> {
        let opened = OpenModelFile::open(path)?;
        Ok(self
            .entries
            .iter()
            .filter(|entry| {
                entry.identity == opened.identity
                    && entry.size_bytes == opened.size_bytes
                    && entry.modified == opened.modified
            })
            .map(|entry| entry.full_reads)
            .sum())
    }
}

/// Returns the cached SHA-256 for a regular model file after checking the current
/// handle identity, size, and modification time. Changed identities are hashed again.
pub fn cached_model_sha256(path: &Path) -> io::Result<(u64, [u8; 32])> {
    let started = Instant::now();
    let mut cache = MODEL_HASH_CACHE
        .get_or_init(|| Mutex::new(ModelFileHashCache::default()))
        .lock()
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "model hash cache is poisoned"))?;
    let reads_before = MODEL_HASH_FULL_READS.load(Ordering::Relaxed);
    let result = cache.sha256(path);
    let reads_after = MODEL_HASH_FULL_READS.load(Ordering::Relaxed);
    drop(cache);
    crate::worker_ipc::record_r7_timing(
        "adapter_model_sha256",
        started.elapsed(),
        serde_json::json!({
            "file": path.file_name().and_then(|name| name.to_str()).unwrap_or("unknown"),
            "cache_hit": reads_after == reads_before,
            "full_reads": reads_after.saturating_sub(reads_before),
            "success": result.is_ok(),
        }),
    );
    result
}

/// Reports whether the current file identity has already incurred its full SHA read.
#[doc(hidden)]
pub fn model_sha256_full_read_count(path: &Path) -> io::Result<u64> {
    MODEL_HASH_CACHE
        .get_or_init(|| Mutex::new(ModelFileHashCache::default()))
        .lock()
        .map_err(|_| io::Error::new(io::ErrorKind::Other, "model hash cache is poisoned"))?
        .full_read_count(path)
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
                    .map_or(ModelAvailability::Missing, |entry| entry.availability)
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
