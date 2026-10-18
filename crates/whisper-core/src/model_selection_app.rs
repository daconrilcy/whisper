use crate::{
    engine_capabilities::{
        Admission, EngineProjection, EngineSnapshot, InferenceRequest, LanguageChoice,
        LanguageCode, Refusal, project_with_existing_recognized, validate_with_existing_recognized,
    },
    model_catalog::{Backend, ModelId},
    model_registry::TURBO_LANGUAGE_INPUTS,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComputePreference {
    Cpu,
    Gpu,
    Auto,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InferencePath {
    Import,
    Live,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectionIntent {
    pub generation: u64,
    pub model: ModelId,
    /// Auto is retained as the user's preference; `effective_backend` is resolved by the
    /// existing compute policy before this admission boundary.
    pub compute: ComputePreference,
    pub effective_backend: Backend,
    pub path: InferencePath,
    pub language: LanguageChoice,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedSelection {
    pub admission: Admission,
    pub compute: ComputePreference,
    pub path: InferencePath,
    pub language: LanguageChoice,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectionRefusal {
    Capability(Refusal),
    UnknownHistoricalModelPath,
}

pub struct ModelSelectionApp {
    snapshot: EngineSnapshot,
}

/// Admission boundary shared by command dispatchers. The accepted paths are supplied by the
/// composition root from the inspected catalog and the one explicitly recognized legacy path.
pub struct ModelCommandGuard {
    selection: ModelSelectionApp,
    generation: u64,
    accepted_model_paths: Vec<(ModelId, String)>,
}

impl ModelCommandGuard {
    pub fn new(
        snapshot: EngineSnapshot,
        generation: u64,
        accepted_turbo_paths: impl IntoIterator<Item = String>,
    ) -> Self {
        Self::new_with_model_paths(
            snapshot,
            generation,
            accepted_turbo_paths
                .into_iter()
                .map(|path| (ModelId::WhisperLargeV3Turbo, path)),
        )
    }

    pub fn new_with_model_paths(
        snapshot: EngineSnapshot,
        generation: u64,
        accepted_model_paths: impl IntoIterator<Item = (ModelId, String)>,
    ) -> Self {
        Self {
            selection: ModelSelectionApp::new(snapshot),
            generation,
            accepted_model_paths: accepted_model_paths.into_iter().collect(),
        }
    }

    pub fn admit_import(
        &self,
        request: &crate::ports::ImportRequest,
        path: InferencePath,
    ) -> Result<AdmittedSelection, SelectionRefusal> {
        self.admit_fields(
            &request.model_path,
            request.model_sha256,
            &request.config,
            self.generation,
            path,
            request.source_samples,
        )
    }

    pub fn admit_live(
        &self,
        request: &crate::ports::LiveRequest,
    ) -> Result<AdmittedSelection, SelectionRefusal> {
        self.admit_fields(
            &request.model_path,
            request.model_sha256,
            &request.config,
            self.generation,
            InferencePath::Live,
            None,
        )
    }

    fn admit_fields(
        &self,
        model_path: &str,
        model_sha256: [u8; 32],
        config: &crate::domain::JobConfig,
        generation: u64,
        path: InferencePath,
        source_samples: Option<u64>,
    ) -> Result<AdmittedSelection, SelectionRefusal> {
        use crate::{domain::ComputeChoice, model_registry::model};

        let model_id = self
            .accepted_model_paths
            .iter()
            .find(|(_, known)| is_known_historical_path(model_path, known))
            .map(|(id, _)| *id)
            .ok_or(SelectionRefusal::UnknownHistoricalModelPath)?;
        let expected_hash = decode_sha256(model(model_id).artifact.sha256)
            .ok_or(SelectionRefusal::UnknownHistoricalModelPath)?;
        if model_sha256 != expected_hash {
            return Err(SelectionRefusal::UnknownHistoricalModelPath);
        }
        if model_id == ModelId::ParakeetTdt06BV3
            && (path != InferencePath::Import
                || !matches!(&config.language, crate::domain::LanguageChoice::Automatic))
        {
            return Err(SelectionRefusal::Capability(Refusal {
                cause: crate::engine_capabilities::RefusalCause::CapabilityNotQualified(
                    crate::engine_capabilities::CapabilityTarget::Compute,
                ),
            }));
        }
        // The core guard admits the product import path independent of duration. The adapter
        // decodes the source and selects single-pass or bounded VAD windows before worker launch;
        // PARAKEET_MAX_AUDIO_SAMPLES is a per-native-window limit, not a whole-file limit.
        let _ = source_samples;
        let compute = match config.compute {
            ComputeChoice::Cpu => (ComputePreference::Cpu, crate::model_catalog::Backend::Cpu),
            ComputeChoice::Gpu => (ComputePreference::Gpu, crate::model_catalog::Backend::Gpu),
            ComputeChoice::Auto if model_id == ModelId::ParakeetTdt06BV3 => {
                let gpu_qualified = self
                    .selection
                    .snapshot
                    .capabilities
                    .iter()
                    .any(|capability| {
                        capability.key.model == model_id
                            && capability.key.backend == Backend::Gpu
                            && capability.compute.support
                                == crate::engine_capabilities::Support::Supported
                            && capability.compute.qualification
                                == crate::engine_capabilities::Qualification::Qualified
                            && capability.auto_language.support
                                == crate::engine_capabilities::Support::Supported
                            && capability.auto_language.qualification
                                == crate::engine_capabilities::Qualification::Qualified
                    });
                if !gpu_qualified {
                    return Err(SelectionRefusal::Capability(Refusal {
                        cause: crate::engine_capabilities::RefusalCause::CapabilityNotQualified(
                            crate::engine_capabilities::CapabilityTarget::Compute,
                        ),
                    }));
                }
                (ComputePreference::Auto, Backend::Gpu)
            }
            ComputeChoice::Auto => (ComputePreference::Auto, Backend::Cpu),
        };
        self.selection.admit(
            SelectionIntent {
                generation,
                model: model_id,
                compute: compute.0,
                effective_backend: compute.1,
                path,
                language: match &config.language {
                    crate::domain::LanguageChoice::Automatic => LanguageChoice::Automatic,
                    crate::domain::LanguageChoice::Manual(value) => {
                        LanguageChoice::Manual(LanguageCode(value.clone()))
                    }
                },
            },
            self.generation,
        )
    }
}

fn decode_sha256(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64 {
        return None;
    }
    let mut bytes = [0; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).ok()?;
    }
    Some(bytes)
}

impl ModelSelectionApp {
    pub fn new(snapshot: EngineSnapshot) -> Self {
        Self { snapshot }
    }

    pub fn projection(&self, current_generation: u64) -> Result<EngineProjection, Refusal> {
        project_with_existing_recognized(&self.snapshot, current_generation)
    }

    /// Shared guard for import, live, queue processing, and explicit resume routes.
    pub fn admit(
        &self,
        intent: SelectionIntent,
        current_generation: u64,
    ) -> Result<AdmittedSelection, SelectionRefusal> {
        if intent.model == ModelId::ParakeetTdt06BV3 && intent.path != InferencePath::Import {
            return Err(SelectionRefusal::Capability(Refusal {
                cause: crate::engine_capabilities::RefusalCause::CapabilityNotQualified(
                    crate::engine_capabilities::CapabilityTarget::Compute,
                ),
            }));
        }
        let language = normalize_language(intent.model, intent.language)?;
        let request = InferenceRequest {
            generation: intent.generation,
            model: intent.model,
            configuration: self
                .snapshot
                .models
                .iter()
                .find(|model| model.id == intent.model)
                .and_then(|model| {
                    model.configurations.iter().find(|key| {
                        key.backend == intent.effective_backend && key.model == intent.model
                    })
                })
                .cloned()
                .ok_or_else(|| {
                    SelectionRefusal::Capability(Refusal {
                        cause: crate::engine_capabilities::RefusalCause::ConfigurationUnknown,
                    })
                })?,
            language: language.clone(),
        };
        let admission =
            validate_with_existing_recognized(&self.snapshot, &request, current_generation)
                .map_err(SelectionRefusal::Capability)?;
        Ok(AdmittedSelection {
            admission,
            compute: intent.compute,
            path: intent.path,
            language,
        })
    }
}

/// Windows path identity is case-insensitive and accepts either separator spelling.
pub fn is_known_historical_path(path: &str, known_path: &str) -> bool {
    normalize_windows_path(path) == normalize_windows_path(known_path)
}

pub fn resolve_legacy_model_path(
    path: &str,
    known_turbo_path: &str,
) -> Result<ModelId, SelectionRefusal> {
    is_known_historical_path(path, known_turbo_path)
        .then_some(ModelId::WhisperLargeV3Turbo)
        .ok_or(SelectionRefusal::UnknownHistoricalModelPath)
}

fn normalize_windows_path(path: &str) -> String {
    path.replace('/', "\\").to_lowercase()
}

fn normalize_language(
    model: ModelId,
    choice: LanguageChoice,
) -> Result<LanguageChoice, SelectionRefusal> {
    match (model, choice) {
        (ModelId::WhisperLargeV3Turbo, LanguageChoice::Manual(code)) => {
            let input = code.0.as_str();
            let canonical = TURBO_LANGUAGE_INPUTS
                .iter()
                .find(|(short, english)| input == *short || input == *english)
                .map(|(short, _)| *short)
                .ok_or_else(|| {
                    SelectionRefusal::Capability(Refusal {
                        cause: crate::engine_capabilities::RefusalCause::LanguageUnavailable,
                    })
                })?;
            Ok(LanguageChoice::Manual(LanguageCode(canonical.to_owned())))
        }
        (_, choice) => Ok(choice),
    }
}
