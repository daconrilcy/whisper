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
}

pub trait PathPort {
    fn load(&self) -> Result<Option<StoredPaths>, String>;
    fn save(&mut self, paths: StoredPaths) -> Result<(), String>;
    fn pick(&mut self, field: PathField, directory: &Path) -> Option<PathBuf>;
    fn exists(&self, path: &Path, field: PathField) -> bool;
    fn take_save_error(&mut self) -> Option<String>;
    fn has_pending_save(&self) -> bool;
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
    fn save(&mut self, _paths: StoredPaths) -> Result<(), String> {
        Ok(())
    }
    fn pick(&mut self, _field: PathField, _directory: &Path) -> Option<PathBuf> {
        None
    }
    fn exists(&self, _path: &Path, _field: PathField) -> bool {
        false
    }
    fn take_save_error(&mut self) -> Option<String> {
        None
    }
    fn has_pending_save(&self) -> bool {
        false
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
            },
            port,
            notice: None,
        };
        match app.port.load() {
            Ok(Some(saved)) if saved.version == 1 => app.restore(saved),
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
            version: 1,
            audio_directory: Some(self.audio_directory.clone()),
            model_path: self.model.clone(),
            destination: self.destination.clone(),
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

    pub fn poll_notice(&mut self) -> Option<String> {
        self.notice.take().or_else(|| self.port.take_save_error())
    }

    pub fn has_pending_save(&self) -> bool {
        self.port.has_pending_save()
    }

    pub fn take_startup_notice(&mut self) -> Option<String> {
        self.notice.take()
    }

    fn save(&mut self) {
        if let Err(error) = self.port.save(self.saved.clone()) {
            self.notice = Some(format!("Chemins utilisés, mais non mémorisés : {error}"));
        }
    }
}
