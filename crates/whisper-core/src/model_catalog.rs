use std::collections::BTreeSet;

/// Closed set of models that this application version knows how to handle.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ModelId {
    WhisperLargeV3Turbo,
    ParakeetTdt06BV3,
}

impl ModelId {
    pub const ALL: [Self; 2] = [Self::WhisperLargeV3Turbo, Self::ParakeetTdt06BV3];
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Backend {
    Cpu,
    Gpu,
}

/// All identity dimensions that scope one set of qualification facts.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ConfigurationKey {
    pub model: ModelId,
    pub artifact: String,
    pub artifact_revision: String,
    pub runtime: String,
    pub runtime_build: String,
    pub worker: String,
    pub worker_build: String,
    pub worker_protocol: u32,
    pub backend: Backend,
    pub hardware_profile: Option<String>,
    pub inference_profile: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelAvailability {
    Ready,
    Missing,
    Corrupt,
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelDefinition {
    pub id: ModelId,
    pub display_name: String,
    pub availability: ModelAvailability,
    pub configurations: BTreeSet<ConfigurationKey>,
}
