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
    #[serde(default, deserialize_with = "deserialize_microphone_gains")]
    pub microphone_gains_db: std::collections::BTreeMap<String, i64>,
}

fn deserialize_microphone_gains<'de, D>(
    deserializer: D,
) -> Result<std::collections::BTreeMap<String, i64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let values =
        <std::collections::BTreeMap<String, GainDbEntry> as serde::Deserialize>::deserialize(
            deserializer,
        )?;
    Ok(values
        .into_iter()
        .map(|(identity, value)| (identity, value.0))
        .collect())
}

struct GainDbEntry(i64);

impl<'de> serde::Deserialize<'de> for GainDbEntry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(GainDbEntryVisitor)
    }
}

struct GainDbEntryVisitor;

impl<'de> serde::de::Visitor<'de> for GainDbEntryVisitor {
    type Value = GainDbEntry;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("an integer microphone gain")
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(GainDbEntry(value))
    }
    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(GainDbEntry(i64::try_from(value).unwrap_or(i64::MIN)))
    }
    fn visit_bool<E>(self, _value: bool) -> Result<Self::Value, E> {
        Ok(GainDbEntry(i64::MIN))
    }
    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E> {
        Ok(GainDbEntry(i64::MIN))
    }
    fn visit_str<E>(self, _value: &str) -> Result<Self::Value, E> {
        Ok(GainDbEntry(i64::MIN))
    }
    fn visit_string<E>(self, _value: String) -> Result<Self::Value, E> {
        Ok(GainDbEntry(i64::MIN))
    }
    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(GainDbEntry(i64::MIN))
    }
    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(GainDbEntry(i64::MIN))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}
        Ok(GainDbEntry(i64::MIN))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        while map
            .next_entry::<serde::de::IgnoredAny, serde::de::IgnoredAny>()?
            .is_some()
        {}
        Ok(GainDbEntry(i64::MIN))
    }
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
                version: 2,
                audio_directory: Some(defaults.audio_directory.clone()),
                model_path: defaults.model_path.clone(),
                destination: defaults.destination.clone(),
                microphone_id: None,
                gpu_device_id: None,
                microphone_gains_db: std::collections::BTreeMap::new(),
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
        let mut invalid_gains = 0_usize;
        let mut microphone_gains_db = std::collections::BTreeMap::new();
        for (identity, value) in &saved.microphone_gains_db {
            let valid = i8::try_from(*value)
                .ok()
                .and_then(|value| crate::microphone_level::GainDb::new(value).ok());
            if let Some(gain) = valid {
                microphone_gains_db.insert(identity.clone(), i64::from(gain.value()));
            } else {
                invalid_gains += 1;
            }
        }
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
        let mut notices = Vec::new();
        if !missing.is_empty() {
            notices.push(format!(
                "Chemin mémorisé introuvable ({}) : valeur par défaut rétablie. Vérifiez le chemin avant de démarrer.",
                missing.join(", ")
            ));
        }
        if invalid_gains > 0 {
            notices.push(format!(
                "Gain microphone invalide neutralisé pour {invalid_gains} identité(s)."
            ));
        }
        self.notice = (!notices.is_empty()).then(|| notices.join(" "));
        self.saved = StoredPaths {
            version: saved.version,
            audio_directory: Some(self.audio_directory.clone()),
            model_path: self.model.clone(),
            destination: self.destination.clone(),
            microphone_id: saved.microphone_id,
            gpu_device_id: saved.gpu_device_id,
            microphone_gains_db,
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

    pub fn microphone_gain(
        &self,
        identity: &crate::microphone_level::MicrophoneIdentity,
    ) -> crate::microphone_level::GainDb {
        self.saved
            .microphone_gains_db
            .get(identity.as_str())
            .and_then(|value| i8::try_from(*value).ok())
            .and_then(|value| crate::microphone_level::GainDb::new(value).ok())
            .unwrap_or(crate::microphone_level::GainDb::NEUTRAL)
    }

    pub fn set_microphone_gain(
        &mut self,
        identity: &crate::microphone_level::MicrophoneIdentity,
        gain: crate::microphone_level::GainDb,
    ) {
        if gain == crate::microphone_level::GainDb::NEUTRAL {
            self.saved.microphone_gains_db.remove(identity.as_str());
        } else {
            self.saved
                .microphone_gains_db
                .insert(identity.as_str().to_owned(), i64::from(gain.value()));
        }
        self.saved.version = 2;
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
    use super::{PathDefaults, PathField, PathPort, PathSettingsApp, SaveStatus, StoredPaths};
    use crate::MicrophoneChoice;
    use std::{
        cell::RefCell,
        path::{Path, PathBuf},
        rc::Rc,
    };

    struct MemoryPort(Rc<RefCell<Option<StoredPaths>>>);

    struct AckPort {
        saved: Rc<RefCell<Option<StoredPaths>>>,
        acknowledgements: Rc<RefCell<std::collections::VecDeque<super::SaveAcknowledgement>>>,
        submit_error: Option<String>,
    }

    impl PathPort for AckPort {
        fn load(&self) -> Result<Option<StoredPaths>, String> {
            Ok(self.saved.borrow().clone())
        }

        fn save(&mut self, _revision: u64, paths: StoredPaths) -> Result<(), String> {
            if let Some(error) = &self.submit_error {
                return Err(error.clone());
            }
            *self.saved.borrow_mut() = Some(paths);
            Ok(())
        }

        fn pick(&mut self, _field: PathField, _directory: &Path) -> Option<PathBuf> {
            None
        }

        fn exists(&self, _path: &Path, _field: PathField) -> bool {
            true
        }

        fn has_pending_save(&self) -> bool {
            !self.acknowledgements.borrow().is_empty()
        }

        fn poll_save_acknowledgement(&mut self) -> Option<super::SaveAcknowledgement> {
            self.acknowledgements.borrow_mut().pop_front()
        }
    }

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

    #[test]
    fn microphone_gains_are_persisted_per_identity_and_neutral_is_absent() {
        let state = Rc::new(RefCell::new(None));
        let defaults = || PathDefaults {
            audio_directory: "audio".into(),
            model_path: "model".into(),
            destination: "output".into(),
        };
        let a = crate::microphone_level::MicrophoneIdentity::new("wasapi:id-A").unwrap();
        let b = crate::microphone_level::MicrophoneIdentity::new("wasapi:id-B").unwrap();
        let mut app = PathSettingsApp::new(defaults(), Box::new(MemoryPort(state.clone())));
        assert_eq!(
            app.microphone_gain(&a),
            crate::microphone_level::GainDb::NEUTRAL
        );
        app.set_microphone_preference(&MicrophoneChoice::Device("wasapi:id-A".into()));
        app.set_microphone_gain(&a, crate::microphone_level::GainDb::new(9).unwrap());
        app.set_microphone_gain(&b, crate::microphone_level::GainDb::new(-3).unwrap());
        app.set_microphone_gain(&a, crate::microphone_level::GainDb::NEUTRAL);

        let reopened = PathSettingsApp::new(defaults(), Box::new(MemoryPort(state.clone())));
        assert_eq!(
            reopened.microphone_gain(&a),
            crate::microphone_level::GainDb::NEUTRAL
        );
        assert_eq!(reopened.microphone_gain(&b).value(), -3);
        assert_eq!(
            reopened.microphone_preference(),
            MicrophoneChoice::Device("wasapi:id-A".into())
        );
        let stored = state.borrow().clone().unwrap();
        assert_eq!(stored.version, 2);
        assert!(!stored.microphone_gains_db.contains_key("wasapi:id-A"));
        assert_eq!(stored.microphone_gains_db.get("wasapi:id-B"), Some(&-3));
    }

    #[test]
    fn invalid_gain_is_neutralized_alone_and_reported() {
        let saved = StoredPaths {
            version: 2,
            audio_directory: Some("audio".into()),
            model_path: "model".into(),
            destination: "output".into(),
            microphone_id: Some("wasapi:preferred".into()),
            gpu_device_id: Some("gpu:preferred".into()),
            microphone_gains_db: std::collections::BTreeMap::from([
                ("wasapi:bad".into(), 100),
                ("wasapi:good".into(), 7),
            ]),
        };
        let bad = crate::microphone_level::MicrophoneIdentity::new("wasapi:bad").unwrap();
        let good = crate::microphone_level::MicrophoneIdentity::new("wasapi:good").unwrap();
        let state = Rc::new(RefCell::new(Some(saved.clone())));
        let mut app = PathSettingsApp::new(
            PathDefaults {
                audio_directory: "audio-default".into(),
                model_path: "model-default".into(),
                destination: "output-default".into(),
            },
            Box::new(MemoryPort(state.clone())),
        );
        assert_eq!(
            app.microphone_gain(&bad),
            crate::microphone_level::GainDb::NEUTRAL
        );
        assert_eq!(app.microphone_gain(&good).value(), 7);
        assert!(app.take_startup_notice().unwrap().contains("neutralisé"));
        app.set_gpu_preference(Some("gpu:new".into()));
        let saved = state.borrow().clone().unwrap();
        assert_eq!(saved.microphone_id.as_deref(), Some("wasapi:preferred"));
        assert_eq!(saved.gpu_device_id.as_deref(), Some("gpu:new"));
        assert_eq!(
            saved.microphone_gains_db,
            std::collections::BTreeMap::from([("wasapi:good".into(), 7)])
        );
    }

    #[test]
    fn stale_save_ack_is_ignored_and_failed_latest_save_is_visible_without_losing_live_gain() {
        let saved = Rc::new(RefCell::new(None));
        let acknowledgements = Rc::new(RefCell::new(std::collections::VecDeque::new()));
        let defaults = PathDefaults {
            audio_directory: "audio".into(),
            model_path: "model".into(),
            destination: "output".into(),
        };
        let identity = crate::microphone_level::MicrophoneIdentity::new("wasapi:active").unwrap();
        let mut app = PathSettingsApp::new(
            defaults,
            Box::new(AckPort {
                saved: saved.clone(),
                acknowledgements: acknowledgements.clone(),
                submit_error: None,
            }),
        );
        app.set_microphone_gain(&identity, crate::microphone_level::GainDb::new(4).unwrap());
        app.set_microphone_gain(&identity, crate::microphone_level::GainDb::new(8).unwrap());
        assert_eq!(app.save_status(), &SaveStatus::Pending { revision: 2 });
        acknowledgements
            .borrow_mut()
            .push_front(super::SaveAcknowledgement {
                revision: 1,
                result: Ok(()),
            });
        assert_eq!(app.poll_notice(), None);
        assert_eq!(app.save_status(), &SaveStatus::Pending { revision: 2 });
        acknowledgements
            .borrow_mut()
            .push_front(super::SaveAcknowledgement {
                revision: 2,
                result: Err("disque indisponible".into()),
            });
        assert!(app.poll_notice().unwrap().contains("n'a pas été mémorisée"));
        assert_eq!(
            app.save_status(),
            &SaveStatus::Failed {
                revision: 2,
                error: "disque indisponible".into()
            }
        );
        assert_eq!(app.microphone_gain(&identity).value(), 8);

        let failing = PathSettingsApp::new(
            PathDefaults {
                audio_directory: "audio".into(),
                model_path: "model".into(),
                destination: "output".into(),
            },
            Box::new(AckPort {
                saved: Rc::new(RefCell::new(None)),
                acknowledgements: Rc::new(RefCell::new(std::collections::VecDeque::new())),
                submit_error: Some("écriture refusée".into()),
            }),
        );
        let mut failing = failing;
        failing.set_microphone_gain(&identity, crate::microphone_level::GainDb::new(5).unwrap());
        assert_eq!(failing.microphone_gain(&identity).value(), 5);
        assert!(matches!(failing.save_status(), SaveStatus::Failed { .. }));
        assert!(failing.poll_notice().unwrap().contains("écriture refusée"));
    }
}
