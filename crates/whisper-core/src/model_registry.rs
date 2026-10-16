use std::collections::BTreeSet;

use crate::{
    engine_capabilities::{
        Capability, ConfigurationCapabilities, EngineSnapshot, LanguageCapability, LanguageCode,
        Qualification, Support,
    },
    model_catalog::{Backend, ConfigurationKey, ModelAvailability, ModelDefinition, ModelId},
};

/// Admission evidence attached to a model capability.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdmissionBasis {
    Measured(Qualification),
    ExistingRecognized,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModelArtifact {
    pub id: ModelId,
    pub file_name: &'static str,
    pub size_bytes: u64,
    pub sha256: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModelRecord {
    pub artifact: ModelArtifact,
    pub display_name: &'static str,
    pub admission: AdmissionBasis,
    pub provenance: &'static str,
}

pub const TURBO_EXISTING_PROVENANCE: &str =
    "comportement de production actuel, décision utilisateur du 16/10/2026";

pub const MODELS: [ModelRecord; 2] = [
    ModelRecord {
        artifact: ModelArtifact {
            id: ModelId::WhisperLargeV3Turbo,
            file_name: "ggml-large-v3-turbo.bin",
            size_bytes: 1_624_555_275,
            sha256: "1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69",
        },
        display_name: "Whisper large-v3-turbo",
        admission: AdmissionBasis::ExistingRecognized,
        provenance: TURBO_EXISTING_PROVENANCE,
    },
    ModelRecord {
        artifact: ModelArtifact {
            id: ModelId::ParakeetTdt06BV3,
            file_name: "parakeet-tdt-0.6b-v3.q8_0.gguf",
            size_bytes: 713_975_456,
            sha256: "e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e",
        },
        display_name: "Parakeet TDT 0.6B v3",
        admission: AdmissionBasis::Measured(Qualification::NotQualified),
        provenance: "Parakeet produit non qualifié dans L1b-1",
    },
];

/// Turbo language inputs accepted by the current runtime: canonical code and exact English name.
pub const TURBO_LANGUAGE_INPUTS: [(&str, &str); 100] = [
    ("en", "english"),
    ("zh", "chinese"),
    ("de", "german"),
    ("es", "spanish"),
    ("ru", "russian"),
    ("ko", "korean"),
    ("fr", "french"),
    ("ja", "japanese"),
    ("pt", "portuguese"),
    ("tr", "turkish"),
    ("pl", "polish"),
    ("ca", "catalan"),
    ("nl", "dutch"),
    ("ar", "arabic"),
    ("sv", "swedish"),
    ("it", "italian"),
    ("id", "indonesian"),
    ("hi", "hindi"),
    ("fi", "finnish"),
    ("vi", "vietnamese"),
    ("he", "hebrew"),
    ("uk", "ukrainian"),
    ("el", "greek"),
    ("ms", "malay"),
    ("cs", "czech"),
    ("ro", "romanian"),
    ("da", "danish"),
    ("hu", "hungarian"),
    ("ta", "tamil"),
    ("no", "norwegian"),
    ("th", "thai"),
    ("ur", "urdu"),
    ("hr", "croatian"),
    ("bg", "bulgarian"),
    ("lt", "lithuanian"),
    ("la", "latin"),
    ("mi", "maori"),
    ("ml", "malayalam"),
    ("cy", "welsh"),
    ("sk", "slovak"),
    ("te", "telugu"),
    ("fa", "persian"),
    ("lv", "latvian"),
    ("bn", "bengali"),
    ("sr", "serbian"),
    ("az", "azerbaijani"),
    ("sl", "slovenian"),
    ("kn", "kannada"),
    ("et", "estonian"),
    ("mk", "macedonian"),
    ("br", "breton"),
    ("eu", "basque"),
    ("is", "icelandic"),
    ("hy", "armenian"),
    ("ne", "nepali"),
    ("mn", "mongolian"),
    ("bs", "bosnian"),
    ("kk", "kazakh"),
    ("sq", "albanian"),
    ("sw", "swahili"),
    ("gl", "galician"),
    ("mr", "marathi"),
    ("pa", "punjabi"),
    ("si", "sinhala"),
    ("km", "khmer"),
    ("sn", "shona"),
    ("yo", "yoruba"),
    ("so", "somali"),
    ("af", "afrikaans"),
    ("oc", "occitan"),
    ("ka", "georgian"),
    ("be", "belarusian"),
    ("tg", "tajik"),
    ("sd", "sindhi"),
    ("gu", "gujarati"),
    ("am", "amharic"),
    ("yi", "yiddish"),
    ("lo", "lao"),
    ("uz", "uzbek"),
    ("fo", "faroese"),
    ("ht", "haitian creole"),
    ("ps", "pashto"),
    ("tk", "turkmen"),
    ("nn", "nynorsk"),
    ("mt", "maltese"),
    ("sa", "sanskrit"),
    ("lb", "luxembourgish"),
    ("my", "myanmar"),
    ("bo", "tibetan"),
    ("tl", "tagalog"),
    ("mg", "malagasy"),
    ("as", "assamese"),
    ("tt", "tatar"),
    ("haw", "hawaiian"),
    ("ln", "lingala"),
    ("ha", "hausa"),
    ("ba", "bashkir"),
    ("jw", "javanese"),
    ("su", "sundanese"),
    ("yue", "cantonese"),
];

pub fn model(id: ModelId) -> &'static ModelRecord {
    MODELS
        .iter()
        .find(|record| record.artifact.id == id)
        .expect("all closed model ids have a registry record")
}

pub fn engine_snapshot(
    generation: u64,
    availability: impl Fn(ModelId) -> ModelAvailability,
    resources: Vec<crate::engine_capabilities::ResourceObservation>,
) -> EngineSnapshot {
    let models: Vec<_> = MODELS
        .iter()
        .map(|record| {
            let configurations = [Backend::Cpu, Backend::Gpu]
                .into_iter()
                .map(|backend| configuration_key(record, backend))
                .collect::<BTreeSet<_>>();
            ModelDefinition {
                id: record.artifact.id,
                display_name: record.display_name.to_owned(),
                availability: availability(record.artifact.id),
                configurations,
            }
        })
        .collect();
    let capabilities = MODELS
        .iter()
        .flat_map(|record| [Backend::Cpu, Backend::Gpu].map(|backend| (record, backend)))
        .map(|(record, backend)| {
            let key = configuration_key(record, backend);
            if record.artifact.id == ModelId::WhisperLargeV3Turbo {
                ConfigurationCapabilities {
                    key,
                    compute: existing(),
                    auto_language: existing(),
                    languages: TURBO_LANGUAGE_INPUTS
                        .iter()
                        .map(|(code, _)| LanguageCapability {
                            code: LanguageCode((*code).to_owned()),
                            recognized: existing(),
                            manual: existing(),
                        })
                        .collect(),
                    resource_requirements: vec![],
                }
            } else {
                ConfigurationCapabilities {
                    key,
                    compute: unknown(),
                    auto_language: unknown(),
                    languages: vec![
                        LanguageCapability {
                            code: LanguageCode("fr".into()),
                            recognized: unknown(),
                            manual: unsupported(),
                        },
                        LanguageCapability {
                            code: LanguageCode("en".into()),
                            recognized: unknown(),
                            manual: unsupported(),
                        },
                    ],
                    resource_requirements: vec![],
                }
            }
        })
        .collect();
    EngineSnapshot {
        generation,
        models,
        capabilities,
        preferred_model: Some(ModelId::WhisperLargeV3Turbo),
        preferred_language: None,
        resources,
    }
}

pub fn configuration_key(record: &ModelRecord, backend: Backend) -> ConfigurationKey {
    ConfigurationKey {
        model: record.artifact.id,
        artifact: record.artifact.file_name.to_owned(),
        artifact_revision: record.artifact.sha256.to_owned(),
        runtime: match record.artifact.id {
            ModelId::WhisperLargeV3Turbo => "whisper-rs",
            ModelId::ParakeetTdt06BV3 => "parakeet-product-candidate",
        }
        .to_owned(),
        runtime_build: match record.artifact.id {
            ModelId::WhisperLargeV3Turbo => "current-production-recognized",
            ModelId::ParakeetTdt06BV3 => "not-integrated",
        }
        .to_owned(),
        worker: match record.artifact.id {
            ModelId::WhisperLargeV3Turbo => "whisper-worker",
            ModelId::ParakeetTdt06BV3 => "parakeet-worker",
        }
        .to_owned(),
        worker_build: match record.artifact.id {
            ModelId::WhisperLargeV3Turbo => "current-production-recognized",
            ModelId::ParakeetTdt06BV3 => "not-integrated",
        }
        .to_owned(),
        worker_protocol: 2,
        backend,
        hardware_profile: (backend == Backend::Gpu).then(|| "selected-gpu".to_owned()),
        inference_profile: "default".to_owned(),
    }
}

fn existing() -> Capability {
    Capability {
        support: Support::Supported,
        qualification: Qualification::ExistingRecognized,
    }
}

fn unknown() -> Capability {
    Capability {
        support: Support::Unknown,
        qualification: Qualification::Unknown,
    }
}

fn unsupported() -> Capability {
    Capability {
        support: Support::Unsupported,
        qualification: Qualification::NotQualified,
    }
}
