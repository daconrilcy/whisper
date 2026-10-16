use std::{
    fs::File,
    io::{self, Read},
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};
use whisper_core::{model_catalog::ModelAvailability, model_registry::MODELS};

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
    let mut file = File::open(path)?;
    let size = file.metadata()?.len();
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    let digest = hasher.finalize();
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
