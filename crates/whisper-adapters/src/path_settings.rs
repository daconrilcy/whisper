use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    sync::{Mutex, OnceLock},
    thread::{self, JoinHandle},
};
use whisper_core::path_settings::{PathField, PathPort, SaveAcknowledgement, StoredPaths};
use whisper_core::{
    model_catalog::ModelId,
    model_selection_app::is_known_historical_path,
    path_settings::{
        LegacyModelPath, ModelPreferenceAcknowledgement, ModelPreferencePort,
        ModelPreferenceSnapshot, StartupComputePreference,
    },
};

pub struct WindowsPathPort {
    file: PathBuf,
    sender: Option<Sender<(u64, StoredPaths)>>,
    results: Receiver<SaveAcknowledgement>,
    worker: Option<JoinHandle<()>>,
    pending: usize,
}

impl WindowsPathPort {
    pub fn new() -> Self {
        let file = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_default()
            .join("Whisper")
            .join("settings.json");
        Self::new_at(file)
    }

    pub fn new_at(file: PathBuf) -> Self {
        let (sender, receiver) = mpsc::channel();
        let (result_sender, results) = mpsc::channel();
        let path = file.clone();
        let worker = thread::spawn(move || {
            for (revision, paths) in receiver {
                let result = write_settings(&path, revision, &paths);
                let _ = result_sender.send(SaveAcknowledgement { revision, result });
            }
        });
        Self {
            file,
            sender: Some(sender),
            results,
            worker: Some(worker),
            pending: 0,
        }
    }

    pub fn settings_path(&self) -> &Path {
        &self.file
    }
}

impl Default for WindowsPathPort {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for WindowsPathPort {
    fn drop(&mut self) {
        self.sender.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl PathPort for WindowsPathPort {
    fn load(&self) -> Result<Option<StoredPaths>, String> {
        if !self.file.is_absolute() {
            return Err("LOCALAPPDATA indisponible".into());
        }
        let bytes = match fs::read(&self.file) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.to_string()),
        };
        serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|error| error.to_string())
    }

    fn save(&mut self, revision: u64, paths: StoredPaths) -> Result<(), String> {
        if !self.file.is_absolute() {
            return Err("LOCALAPPDATA indisponible".into());
        }
        self.sender
            .as_ref()
            .ok_or_else(|| "écriture arrêtée".to_owned())?
            .send((revision, paths))
            .map_err(|error| error.to_string())?;
        self.pending += 1;
        Ok(())
    }

    fn pick(&mut self, field: PathField, directory: &Path) -> Option<PathBuf> {
        #[cfg(target_os = "windows")]
        {
            let mut dialog = rfd::FileDialog::new();
            if !directory.as_os_str().is_empty() {
                dialog = dialog.set_directory(directory);
            }
            match field {
                PathField::Audio => dialog
                    .add_filter("Audio WAV/MP3", &["wav", "mp3"])
                    .pick_file(),
                PathField::Model => dialog.add_filter("Modèle BIN", &["bin"]).pick_file(),
                PathField::Destination => dialog.pick_folder(),
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            let _ = (field, directory);
            None
        }
    }

    fn exists(&self, path: &Path, field: PathField) -> bool {
        match field {
            PathField::Destination => path.is_dir(),
            PathField::Audio | PathField::Model => {
                let extension = path.extension().and_then(|value| value.to_str());
                path.is_file()
                    && match field {
                        PathField::Audio => extension.is_some_and(|value| {
                            value.eq_ignore_ascii_case("wav") || value.eq_ignore_ascii_case("mp3")
                        }),
                        PathField::Model => {
                            extension.is_some_and(|value| value.eq_ignore_ascii_case("bin"))
                        }
                        PathField::Destination => false,
                    }
            }
        }
    }

    fn has_pending_save(&self) -> bool {
        self.pending > 0
    }

    fn poll_save_acknowledgement(&mut self) -> Option<SaveAcknowledgement> {
        match self.results.try_recv() {
            Ok(acknowledgement) => {
                self.pending = self.pending.saturating_sub(1);
                Some(acknowledgement)
            }
            Err(mpsc::TryRecvError::Empty | mpsc::TryRecvError::Disconnected) => None,
        }
    }
}

pub struct WindowsModelPreferencePort {
    file: PathBuf,
    known_historical_turbo_path: String,
}

impl WindowsModelPreferencePort {
    pub fn new() -> Self {
        let local_app_data = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_default();
        let file = local_app_data.join("Whisper").join("settings.json");
        let known_historical_turbo_path = local_app_data
            .join("Programs")
            .join("WhisperBuildDeps")
            .join("models")
            .join("ggml-large-v3-turbo.bin")
            .to_string_lossy()
            .into_owned();
        Self {
            file,
            known_historical_turbo_path,
        }
    }

    pub fn new_at(file: PathBuf, known_historical_turbo_path: impl Into<String>) -> Self {
        Self {
            file,
            known_historical_turbo_path: known_historical_turbo_path.into(),
        }
    }
}

impl ModelPreferencePort for WindowsModelPreferencePort {
    fn load(&self) -> Result<Option<ModelPreferenceSnapshot>, String> {
        if !self.file.is_absolute() {
            return Err("LOCALAPPDATA indisponible".into());
        }
        let bytes = match fs::read(&self.file) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.to_string()),
        };
        let value: serde_json::Value = serde_json::from_slice(&bytes)
            .map_err(|error| format!("Réglages illisibles : {error}"))?;
        let version = value
            .get("version")
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| "version de réglages absente".to_owned())?;
        match version {
            1 => {
                let legacy = value
                    .get("model_path")
                    .and_then(serde_json::Value::as_str)
                    .map(|path| {
                        if is_known_historical_path(path, &self.known_historical_turbo_path) {
                            LegacyModelPath::KnownTurbo(path.to_owned())
                        } else {
                            LegacyModelPath::Unknown(path.to_owned())
                        }
                    })
                    .unwrap_or(LegacyModelPath::None);
                let preferred_model = matches!(legacy, LegacyModelPath::KnownTurbo(_))
                    .then_some(ModelId::WhisperLargeV3Turbo);
                Ok(Some(ModelPreferenceSnapshot {
                    revision: 0,
                    preferred_model,
                    startup_compute: StartupComputePreference::FollowProbe,
                    legacy_model_path: legacy,
                }))
            }
            2 => {
                let preferred_model = match value
                    .get("preferred_model")
                    .and_then(serde_json::Value::as_str)
                {
                    Some("WhisperLargeV3Turbo") => Some(ModelId::WhisperLargeV3Turbo),
                    Some("ParakeetTdt06BV3") => Some(ModelId::ParakeetTdt06BV3),
                    _ => None,
                };
                let startup_compute = match value
                    .get("startup_compute")
                    .and_then(serde_json::Value::as_str)
                {
                    Some("Auto") => StartupComputePreference::Auto,
                    _ => StartupComputePreference::FollowProbe,
                };
                let legacy_model_path = value
                    .get("legacy_model_path")
                    .and_then(serde_json::Value::as_str)
                    .map(|path| {
                        if is_known_historical_path(path, &self.known_historical_turbo_path) {
                            LegacyModelPath::KnownTurbo(path.to_owned())
                        } else {
                            LegacyModelPath::Unknown(path.to_owned())
                        }
                    })
                    .unwrap_or(LegacyModelPath::None);
                Ok(Some(ModelPreferenceSnapshot {
                    revision: value
                        .get("preference_revision")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0),
                    preferred_model,
                    startup_compute,
                    legacy_model_path,
                }))
            }
            _ => Err(format!("version de réglages non supportée : {version}")),
        }
    }

    fn save(
        &mut self,
        revision: u64,
        preferred_model: Option<ModelId>,
        startup_compute: StartupComputePreference,
    ) -> ModelPreferenceAcknowledgement {
        let result = write_model_preferences(
            &self.file,
            revision,
            preferred_model,
            startup_compute,
            &self.known_historical_turbo_path,
        );
        ModelPreferenceAcknowledgement { revision, result }
    }
}

fn write_settings(path: &Path, revision: u64, paths: &StoredPaths) -> Result<(), String> {
    let _guard = settings_write_lock()
        .lock()
        .map_err(|_| "verrou des réglages empoisonné".to_owned())?;
    let parent = path.parent().ok_or("profil utilisateur introuvable")?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let mut value = read_settings_value(path)?.unwrap_or_else(|| serde_json::json!({}));
    let object = value
        .as_object_mut()
        .ok_or_else(|| "format des réglages incompatible".to_owned())?;
    let current_version = object
        .get("version")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    object.insert(
        "version".into(),
        serde_json::json!(current_version.max(u64::from(paths.version))),
    );
    object.insert(
        "audio_directory".into(),
        serde_json::to_value(&paths.audio_directory).map_err(|error| error.to_string())?,
    );
    object.insert("model_path".into(), serde_json::json!(paths.model_path));
    object.insert("destination".into(), serde_json::json!(paths.destination));
    object.insert(
        "microphone_id".into(),
        serde_json::json!(paths.microphone_id),
    );
    object.insert(
        "gpu_device_id".into(),
        serde_json::json!(paths.gpu_device_id),
    );
    write_settings_value(path, revision, &value)
}

fn write_model_preferences(
    path: &Path,
    revision: u64,
    preferred_model: Option<ModelId>,
    startup_compute: StartupComputePreference,
    known_historical_turbo_path: &str,
) -> Result<(), String> {
    let _guard = settings_write_lock()
        .lock()
        .map_err(|_| "verrou des réglages empoisonné".to_owned())?;
    if !path.is_absolute() {
        return Err("profil utilisateur introuvable".into());
    }
    let parent = path.parent().ok_or("profil utilisateur introuvable")?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let original = match fs::read(path) {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(error) => return Err(error.to_string()),
    };
    let mut value = match &original {
        Some(bytes) => serde_json::from_slice::<serde_json::Value>(bytes)
            .map_err(|error| format!("Réglages illisibles : {error}"))?,
        None => serde_json::json!({
            "version": 2,
            "audio_directory": null,
            "model_path": known_historical_turbo_path,
            "destination": "",
            "microphone_id": null,
            "gpu_device_id": null
        }),
    };
    let object = value
        .as_object_mut()
        .ok_or_else(|| "format des réglages incompatible".to_owned())?;
    let version = object
        .get("version")
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| "version de réglages absente".to_owned())?;
    match version {
        1 => {
            let bytes = original
                .as_deref()
                .ok_or_else(|| "octets v1 absents".to_owned())?;
            ensure_v1_backup(path, bytes)?;
            if let Some(path) = object
                .get("model_path")
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
            {
                object.insert("legacy_model_path".into(), serde_json::json!(path));
                let legacy_path = object
                    .get("legacy_model_path")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default();
                if !is_known_historical_path(legacy_path, known_historical_turbo_path) {
                    object.insert("preferred_model".into(), serde_json::Value::Null);
                }
            }
        }
        2 => {}
        other => return Err(format!("version de réglages non supportée : {other}")),
    }
    object.insert("version".into(), serde_json::json!(2));
    object.insert(
        "preferred_model".into(),
        preferred_model.map_or(serde_json::Value::Null, |model| {
            serde_json::json!(match model {
                ModelId::WhisperLargeV3Turbo => "WhisperLargeV3Turbo",
                ModelId::ParakeetTdt06BV3 => "ParakeetTdt06BV3",
            })
        }),
    );
    object.insert(
        "startup_compute".into(),
        serde_json::json!(match startup_compute {
            StartupComputePreference::Auto => "Auto",
            StartupComputePreference::FollowProbe => "FollowProbe",
        }),
    );
    object.insert("preference_revision".into(), serde_json::json!(revision));
    write_settings_value(path, revision, &value)
}

fn settings_write_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn ensure_v1_backup(path: &Path, original: &[u8]) -> Result<(), String> {
    let mut backup_name = path
        .file_name()
        .ok_or_else(|| "nom de fichier de réglages absent".to_owned())?
        .to_os_string();
    backup_name.push(".v1.bak");
    let backup_path = path.with_file_name(backup_name);
    match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&backup_path)
    {
        Ok(mut backup) => {
            backup
                .write_all(original)
                .map_err(|error| error.to_string())?;
            backup.sync_all().map_err(|error| error.to_string())
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let existing = fs::read(&backup_path).map_err(|error| error.to_string())?;
            if existing == original {
                Ok(())
            } else {
                Err("Sauvegarde v1 existante conflictuelle ; réglages v1 conservés.".into())
            }
        }
        Err(error) => Err(format!(
            "Sauvegarde v1 impossible ; réglages v1 conservés : {error}"
        )),
    }
}

fn read_settings_value(path: &Path) -> Result<Option<serde_json::Value>, String> {
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|error| format!("Réglages illisibles : {error}")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

fn write_settings_value(
    path: &Path,
    revision: u64,
    value: &serde_json::Value,
) -> Result<(), String> {
    let parent = path.parent().ok_or("profil utilisateur introuvable")?;
    let temporary = parent.join(format!("settings-{}-{revision}.tmp", std::process::id()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| error.to_string())?;
        serde_json::to_writer_pretty(&mut file, value).map_err(|error| error.to_string())?;
        file.write_all(b"\n").map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
        drop(file);
        fs::rename(&temporary, path).map_err(|error| error.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}
