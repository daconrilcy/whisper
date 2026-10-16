use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PathField {
    Audio,
    Model,
    Destination,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StoredPaths {
    pub version: u8,
    pub audio_directory: Option<String>,
    pub model_path: String,
    pub destination: String,
    #[serde(default)]
    pub microphone_id: Option<String>,
    #[serde(default)]
    pub gpu_device_id: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SaveStatus {
    NotSaved,
    Pending { revision: u64 },
    Saved { revision: u64 },
    Failed { revision: u64, error: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SaveAcknowledgement {
    pub revision: u64,
    pub result: Result<(), String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StartupComputePreference {
    Auto,
    FollowProbe,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LegacyModelPath {
    None,
    KnownTurbo(String),
    Unknown(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelPreferenceSnapshot {
    pub revision: u64,
    pub preferred_model: Option<crate::model_catalog::ModelId>,
    pub startup_compute: StartupComputePreference,
    pub legacy_model_path: LegacyModelPath,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelPreferenceAcknowledgement {
    pub revision: u64,
    pub result: Result<(), String>,
}

pub trait ModelPreferencePort {
    fn load(&self) -> Result<Option<ModelPreferenceSnapshot>, String>;
    fn save(
        &mut self,
        revision: u64,
        preferred_model: Option<crate::model_catalog::ModelId>,
        startup_compute: StartupComputePreference,
    ) -> ModelPreferenceAcknowledgement;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelPreferenceSaveStatus {
    Saved { revision: u64 },
    Failed { revision: u64, error: String },
}

pub struct ModelPreferencesApp {
    port: Box<dyn ModelPreferencePort>,
    pub preferred_model: Option<crate::model_catalog::ModelId>,
    pub startup_compute: StartupComputePreference,
    pub legacy_model_path: LegacyModelPath,
    revision: u64,
    status: Option<ModelPreferenceSaveStatus>,
}

impl ModelPreferencesApp {
    pub fn new(port: Box<dyn ModelPreferencePort>) -> Result<Self, String> {
        let loaded = port.load()?;
        let snapshot = loaded.unwrap_or(ModelPreferenceSnapshot {
            revision: 0,
            preferred_model: Some(crate::model_catalog::ModelId::WhisperLargeV3Turbo),
            startup_compute: StartupComputePreference::FollowProbe,
            legacy_model_path: LegacyModelPath::None,
        });
        let preferred_model = match (snapshot.preferred_model, &snapshot.legacy_model_path) {
            (Some(model), _) => Some(model),
            (None, LegacyModelPath::Unknown(_)) => None,
            (None, LegacyModelPath::None | LegacyModelPath::KnownTurbo(_)) => {
                Some(crate::model_catalog::ModelId::WhisperLargeV3Turbo)
            }
        };
        Ok(Self {
            port,
            preferred_model,
            startup_compute: snapshot.startup_compute,
            legacy_model_path: snapshot.legacy_model_path,
            revision: snapshot.revision,
            status: None,
        })
    }

    pub fn choose_model(&mut self, model: crate::model_catalog::ModelId) {
        self.preferred_model = Some(model);
        self.persist();
    }

    pub fn choose_startup_compute(&mut self, preference: StartupComputePreference) {
        self.startup_compute = preference;
        self.persist();
    }

    pub fn save_status(&self) -> Option<&ModelPreferenceSaveStatus> {
        self.status.as_ref()
    }

    pub fn legacy_path_notice(&self) -> Option<String> {
        if self.preferred_model.is_some() {
            return None;
        }
        match &self.legacy_model_path {
            LegacyModelPath::Unknown(path) => Some(format!(
                "Chemin de modèle historique inconnu : {path}. Choisissez un modèle du catalogue avant de continuer."
            )),
            LegacyModelPath::None | LegacyModelPath::KnownTurbo(_) => None,
        }
    }

    fn persist(&mut self) {
        let Some(revision) = self.revision.checked_add(1) else {
            self.status = Some(ModelPreferenceSaveStatus::Failed {
                revision: self.revision,
                error: "compteur de révision épuisé".into(),
            });
            return;
        };
        let acknowledgement = self
            .port
            .save(revision, self.preferred_model, self.startup_compute);
        if acknowledgement.revision != revision {
            self.status = Some(ModelPreferenceSaveStatus::Failed {
                revision,
                error: "acquittement d'une autre révision".into(),
            });
            return;
        }
        self.revision = revision;
        self.status = Some(match acknowledgement.result {
            Ok(()) => ModelPreferenceSaveStatus::Saved { revision },
            Err(error) => ModelPreferenceSaveStatus::Failed { revision, error },
        });
    }
}

pub trait PathPort {
    fn load(&self) -> Result<Option<StoredPaths>, String>;
    fn save(&mut self, revision: u64, paths: StoredPaths) -> Result<(), String>;
    fn pick(&mut self, field: PathField, directory: &Path) -> Option<PathBuf>;
    fn exists(&self, path: &Path, field: PathField) -> bool;
    fn has_pending_save(&self) -> bool;
    fn poll_save_acknowledgement(&mut self) -> Option<SaveAcknowledgement>;
}

pub struct PathDefaults {
    pub audio_directory: String,
    pub model_path: String,
    pub destination: String,
}

pub struct NullPathPort;

impl PathPort for NullPathPort {
    fn load(&self) -> Result<Option<StoredPaths>, String> {
        Ok(None)
    }
    fn save(&mut self, _revision: u64, _paths: StoredPaths) -> Result<(), String> {
        Ok(())
    }
    fn pick(&mut self, _field: PathField, _directory: &Path) -> Option<PathBuf> {
        None
    }
    fn exists(&self, _path: &Path, _field: PathField) -> bool {
        false
    }
    fn has_pending_save(&self) -> bool {
        false
    }
    fn poll_save_acknowledgement(&mut self) -> Option<SaveAcknowledgement> {
        None
    }
}

pub struct PathSettingsApp {
    port: Box<dyn PathPort>,
    pub audio: String,
    pub model: String,
    pub destination: String,
    audio_directory: String,
    saved: StoredPaths,
    notice: Option<String>,
    save_revision: u64,
    save_status: SaveStatus,
}

impl PathSettingsApp {
    pub fn with_defaults(defaults: PathDefaults) -> Self {
        Self::new(defaults, Box::new(NullPathPort))
    }

    pub fn new(defaults: PathDefaults, port: Box<dyn PathPort>) -> Self {
        let mut app = Self {
            audio: String::new(),
            model: defaults.model_path.clone(),
            destination: defaults.destination.clone(),
            audio_directory: defaults.audio_directory.clone(),
            saved: StoredPaths {
                version: 1,
                audio_directory: Some(defaults.audio_directory.clone()),
                model_path: defaults.model_path.clone(),
                destination: defaults.destination.clone(),
                microphone_id: None,
                gpu_device_id: None,
            },
            port,
            notice: None,
            save_revision: 0,
            save_status: SaveStatus::NotSaved,
        };
        match app.port.load() {
            Ok(Some(saved)) if matches!(saved.version, 1 | 2) => app.restore(saved),
            Ok(Some(_)) => {
                app.notice = Some(
                    "Réglages des chemins incompatibles : valeurs par défaut utilisées.".into(),
                )
            }
            Ok(None) => {}
            Err(error) => {
                app.notice = Some(format!(
                    "Réglages des chemins illisibles : {error}. Valeurs par défaut utilisées."
                ))
            }
        }
        app
    }

    fn restore(&mut self, saved: StoredPaths) {
        let mut missing = Vec::new();
        if let Some(directory) = saved.audio_directory {
            if self
                .port
                .exists(Path::new(&directory), PathField::Destination)
            {
                self.audio_directory = directory;
            } else {
                missing.push("dossier du dernier audio");
            }
        }
        if self
            .port
            .exists(Path::new(&saved.model_path), PathField::Model)
        {
            self.model = saved.model_path;
        } else {
            missing.push("modèle local");
        }
        if self
            .port
            .exists(Path::new(&saved.destination), PathField::Destination)
        {
            self.destination = saved.destination;
        } else {
            missing.push("dossier des transcriptions");
        }
        if !missing.is_empty() {
            self.notice = Some(format!(
                "Chemin mémorisé introuvable ({}) : valeur par défaut rétablie. Vérifiez le chemin avant de démarrer.",
                missing.join(", ")
            ));
        }
        self.saved = StoredPaths {
            version: saved.version,
            audio_directory: Some(self.audio_directory.clone()),
            model_path: self.model.clone(),
            destination: self.destination.clone(),
            microphone_id: saved.microphone_id,
            gpu_device_id: saved.gpu_device_id,
        };
    }

    pub fn browse(&mut self, field: PathField) {
        let current = match field {
            PathField::Audio => self.audio_directory.as_str(),
            PathField::Model => self.model.as_str(),
            PathField::Destination => self.destination.as_str(),
        };
        let initial = match field {
            PathField::Model => Path::new(current).parent().unwrap_or(Path::new("")),
            _ => Path::new(current),
        };
        let initial = initial
            .ancestors()
            .find(|path| self.port.exists(path, PathField::Destination))
            .unwrap_or(Path::new(""));
        let Some(selected) = self.port.pick(field, initial) else {
            return;
        };
        let Some(selected) = selected.to_str() else {
            self.notice = Some("Chemin Windows non représentable dans les champs actuels.".into());
            return;
        };
        if !self.port.exists(Path::new(selected), field) {
            self.notice = Some("Le chemin choisi est introuvable ou n'a pas le bon type.".into());
            return;
        }
        match field {
            PathField::Audio => {
                self.audio = selected.to_owned();
                if let Some(parent) = Path::new(selected).parent() {
                    self.audio_directory = parent.to_string_lossy().into_owned();
                    self.saved.audio_directory = Some(self.audio_directory.clone());
                }
            }
            PathField::Model => {
                self.model = selected.to_owned();
                self.saved.model_path = self.model.clone();
            }
            PathField::Destination => {
                self.destination = selected.to_owned();
                self.saved.destination = self.destination.clone();
            }
        }
        self.save();
    }

    pub fn commit_manual(&mut self, field: PathField) {
        let valid = match field {
            PathField::Audio => {
                let path = Path::new(self.audio.trim());
                if self.audio.trim().is_empty() {
                    true
                } else if self.port.exists(path, PathField::Audio) {
                    if let Some(parent) = path.parent() {
                        self.audio_directory = parent.to_string_lossy().into_owned();
                        self.saved.audio_directory = Some(self.audio_directory.clone());
                    }
                    true
                } else {
                    false
                }
            }
            PathField::Model => {
                if self
                    .port
                    .exists(Path::new(self.model.trim()), PathField::Model)
                {
                    self.saved.model_path = self.model.trim().to_owned();
                    true
                } else {
                    false
                }
            }
            PathField::Destination => {
                if self
                    .port
                    .exists(Path::new(self.destination.trim()), PathField::Destination)
                {
                    self.saved.destination = self.destination.trim().to_owned();
                    true
                } else {
                    false
                }
            }
        };
        if valid {
            self.save();
        }
    }

    pub fn commit_all(&mut self) {
        self.commit_manual(PathField::Audio);
        self.commit_manual(PathField::Model);
        self.commit_manual(PathField::Destination);
    }

    pub fn microphone_preference(&self) -> crate::microphone::MicrophoneChoice {
        match &self.saved.microphone_id {
            Some(id) => crate::microphone::MicrophoneChoice::Device(id.clone()),
            None => crate::microphone::MicrophoneChoice::WindowsDefault,
        }
    }

    pub fn set_microphone_preference(&mut self, choice: &crate::microphone::MicrophoneChoice) {
        self.saved.microphone_id = match choice {
            crate::microphone::MicrophoneChoice::WindowsDefault => None,
            crate::microphone::MicrophoneChoice::Device(id) => Some(id.clone()),
        };
        self.save();
    }

    pub fn gpu_preference(&self) -> Option<&str> {
        self.saved.gpu_device_id.as_deref()
    }

    pub fn set_gpu_preference(&mut self, identity: Option<String>) {
        self.saved.gpu_device_id = identity;
        self.save();
    }

    pub fn save_status(&self) -> &SaveStatus {
        &self.save_status
    }

    pub fn poll_notice(&mut self) -> Option<String> {
        let mut latest_notice = self.notice.take();
        while let Some(acknowledgement) = self.port.poll_save_acknowledgement() {
            if acknowledgement.revision != self.save_revision {
                continue;
            }
            match acknowledgement.result {
                Ok(()) => {
                    self.save_status = SaveStatus::Saved {
                        revision: acknowledgement.revision,
                    };
                }
                Err(error) => {
                    self.save_status = SaveStatus::Failed {
                        revision: acknowledgement.revision,
                        error: error.clone(),
                    };
                    latest_notice = Some(format!(
                        "Réglages utilisés, mais la révision {} n'a pas été mémorisée : {error}",
                        acknowledgement.revision
                    ));
                }
            }
        }
        latest_notice
    }

    pub fn has_pending_save(&self) -> bool {
        self.port.has_pending_save()
    }

    pub fn take_startup_notice(&mut self) -> Option<String> {
        self.notice.take()
    }

    fn save(&mut self) {
        let Some(revision) = self.save_revision.checked_add(1) else {
            self.save_status = SaveStatus::Failed {
                revision: self.save_revision,
                error: "compteur de révision épuisé".into(),
            };
            self.notice =
                Some("Réglages utilisés, mais non mémorisés : compteur de révision épuisé.".into());
            return;
        };
        self.save_revision = revision;
        match self.port.save(revision, self.saved.clone()) {
            Ok(()) => self.save_status = SaveStatus::Pending { revision },
            Err(error) => {
                self.save_status = SaveStatus::Failed {
                    revision,
                    error: error.clone(),
                };
                self.notice = Some(format!(
                    "Réglages utilisés, mais la révision {revision} n'a pas été mémorisée : {error}"
                ));
            }
        }
    }
}

#[cfg(test)]
mod microphone_tests {
    use super::{PathDefaults, PathField, PathPort, PathSettingsApp, StoredPaths};
    use crate::MicrophoneChoice;
    use std::{
        cell::RefCell,
        path::{Path, PathBuf},
        rc::Rc,
    };

    struct MemoryPort(Rc<RefCell<Option<StoredPaths>>>);

    impl PathPort for MemoryPort {
        fn load(&self) -> Result<Option<StoredPaths>, String> {
            Ok(self.0.borrow().clone())
        }

        fn save(&mut self, _revision: u64, paths: StoredPaths) -> Result<(), String> {
            *self.0.borrow_mut() = Some(paths);
            Ok(())
        }

        fn pick(&mut self, _field: PathField, _directory: &Path) -> Option<PathBuf> {
            None
        }

        fn exists(&self, _path: &Path, _field: PathField) -> bool {
            true
        }

        fn has_pending_save(&self) -> bool {
            false
        }

        fn poll_save_acknowledgement(&mut self) -> Option<super::SaveAcknowledgement> {
            None
        }
    }

    #[test]
    fn microphone_preference_survives_path_save_and_reopen() {
        let state = Rc::new(RefCell::new(None));
        let defaults = || PathDefaults {
            audio_directory: "audio".into(),
            model_path: "model".into(),
            destination: "output".into(),
        };
        let mut first = PathSettingsApp::new(defaults(), Box::new(MemoryPort(state.clone())));
        first.set_microphone_preference(&MicrophoneChoice::Device("wasapi:one".into()));
        first.commit_manual(PathField::Model);
        let reopened = PathSettingsApp::new(defaults(), Box::new(MemoryPort(state)));
        assert_eq!(
            reopened.microphone_preference(),
            MicrophoneChoice::Device("wasapi:one".into())
        );
        assert_eq!(reopened.model, "model");
    }
}
