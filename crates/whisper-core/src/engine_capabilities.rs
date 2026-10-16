use crate::model_catalog::{
    Backend, ConfigurationKey, ModelAvailability, ModelDefinition, ModelId,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Support {
    Supported,
    Unsupported,
    Unknown,
    Partial,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Qualification {
    Qualified,
    NotQualified,
    Unknown,
    Partial,
    ExistingRecognized,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Capability {
    pub support: Support,
    pub qualification: Qualification,
}

impl Capability {
    pub const QUALIFIED: Self = Self {
        support: Support::Supported,
        qualification: Qualification::Qualified,
    };

    fn is_qualified(self) -> bool {
        self.support == Support::Supported && self.qualification == Qualification::Qualified
    }

    /// Existing product behavior may be admitted without calling it a measured qualification.
    pub fn is_admissible(self) -> bool {
        self.support == Support::Supported
            && matches!(
                self.qualification,
                Qualification::Qualified | Qualification::ExistingRecognized
            )
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct LanguageCode(pub String);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguageCapability {
    pub code: LanguageCode,
    pub recognized: Capability,
    pub manual: Capability,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceKind {
    HostRam,
    GpuVram,
    ModelDisk,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapabilityTarget {
    Compute,
    AutomaticLanguage,
    LanguageRecognition,
    ManualLanguage,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceRequirement {
    pub kind: ResourceKind,
    pub minimum_bytes: u64,
    /// Device, volume, and other conditions needed for a valid comparison.
    pub scope: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceObservation {
    pub kind: ResourceKind,
    pub available_bytes: u64,
    pub scope: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigurationCapabilities {
    pub key: ConfigurationKey,
    pub compute: Capability,
    pub auto_language: Capability,
    pub languages: Vec<LanguageCapability>,
    pub resource_requirements: Vec<ResourceRequirement>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EngineSnapshot {
    pub generation: u64,
    pub models: Vec<ModelDefinition>,
    pub capabilities: Vec<ConfigurationCapabilities>,
    pub preferred_model: Option<ModelId>,
    pub preferred_language: Option<LanguageChoice>,
    pub resources: Vec<ResourceObservation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LanguageChoice {
    Automatic,
    Manual(LanguageCode),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RefusalCause {
    ModelUnknown,
    ModelMissing,
    ModelCorrupt,
    ModelCancelled,
    ConfigurationUnknown,
    ConfigurationUnsupported,
    CapabilityUnsupported(CapabilityTarget),
    CapabilityNotQualified(CapabilityTarget),
    LanguageUnavailable,
    ResourceInsufficient(ResourceKind),
    StaleGeneration,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LanguageOption {
    pub code: LanguageCode,
    pub selectable: bool,
    pub cause: Option<RefusalCause>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConfigurationOption {
    pub key: ConfigurationKey,
    pub selectable: bool,
    pub cause: Option<RefusalCause>,
    pub auto_language_selectable: bool,
    pub auto_language_cause: Option<RefusalCause>,
    pub languages: Vec<LanguageOption>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelOption {
    pub id: ModelId,
    pub display_name: String,
    pub selectable: bool,
    pub cause: Option<RefusalCause>,
    pub preferred: bool,
    pub configurations: Vec<ConfigurationOption>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EngineProjection {
    pub generation: u64,
    pub preferred_model: Option<ModelId>,
    pub preferred_language: Option<LanguageChoice>,
    pub models: Vec<ModelOption>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InferenceRequest {
    pub generation: u64,
    pub model: ModelId,
    pub configuration: ConfigurationKey,
    pub language: LanguageChoice,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Admission {
    pub generation: u64,
    pub model: ModelId,
    pub backend: Backend,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Refusal {
    pub cause: RefusalCause,
}

/// Builds display choices from one immutable snapshot, retaining preferences separately.
pub fn project(
    snapshot: &EngineSnapshot,
    current_generation: u64,
) -> Result<EngineProjection, Refusal> {
    if snapshot.generation != current_generation {
        return Err(refuse(RefusalCause::StaleGeneration));
    }
    let models = snapshot
        .models
        .iter()
        .map(|model| {
            let configurations: Vec<_> = model
                .configurations
                .iter()
                .map(|key| {
                    let capability = find_capability(snapshot, key);
                    let (config_selectable, config_cause) =
                        configuration_status(snapshot, model, capability);
                    let (auto_language_capable, auto_language_capability_cause) = match capability {
                        Some(capability) if capability.auto_language.is_qualified() => (true, None),
                        Some(capability) => (
                            false,
                            Some(capability_cause(
                                capability.auto_language,
                                CapabilityTarget::AutomaticLanguage,
                            )),
                        ),
                        None => (false, Some(RefusalCause::ConfigurationUnknown)),
                    };
                    let auto_language_selectable = config_selectable && auto_language_capable;
                    let auto_language_cause = if auto_language_selectable {
                        None
                    } else {
                        config_cause.or(auto_language_capability_cause)
                    };
                    let languages: Vec<LanguageOption> = capability
                        .map(|capability| {
                            capability
                                .languages
                                .iter()
                                .map(|language| {
                                    let (language_capable, capability_cause) =
                                        if language.recognized.is_qualified()
                                            && language.manual.is_qualified()
                                        {
                                            (true, None)
                                        } else if !language.recognized.is_qualified() {
                                            (
                                                false,
                                                Some(capability_cause(
                                                    language.recognized,
                                                    CapabilityTarget::LanguageRecognition,
                                                )),
                                            )
                                        } else {
                                            (
                                                false,
                                                Some(capability_cause(
                                                    language.manual,
                                                    CapabilityTarget::ManualLanguage,
                                                )),
                                            )
                                        };
                                    let selectable = config_selectable && language_capable;
                                    let cause = if selectable {
                                        None
                                    } else {
                                        config_cause.or(capability_cause)
                                    };
                                    LanguageOption {
                                        code: language.code.clone(),
                                        selectable,
                                        cause,
                                    }
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    let has_language_route = auto_language_selectable
                        || languages.iter().any(|language| language.selectable);
                    ConfigurationOption {
                        key: key.clone(),
                        selectable: config_selectable && has_language_route,
                        cause: config_cause.or_else(|| {
                            (!has_language_route).then_some(RefusalCause::LanguageUnavailable)
                        }),
                        auto_language_selectable,
                        auto_language_cause,
                        languages,
                    }
                })
                .collect();
            let cause = model_status_cause(model);
            let selectable = cause.is_none()
                && configurations.iter().any(|configuration| {
                    configuration.selectable
                        && (configuration.auto_language_selectable
                            || configuration
                                .languages
                                .iter()
                                .any(|language| language.selectable))
                });
            ModelOption {
                id: model.id,
                display_name: model.display_name.clone(),
                selectable,
                cause: cause.or_else(|| {
                    (!selectable).then(|| {
                        configurations
                            .iter()
                            .find_map(|configuration| configuration.cause)
                            .unwrap_or(RefusalCause::LanguageUnavailable)
                    })
                }),
                preferred: snapshot.preferred_model == Some(model.id),
                configurations,
            }
        })
        .collect();
    Ok(EngineProjection {
        generation: snapshot.generation,
        preferred_model: snapshot.preferred_model,
        preferred_language: snapshot.preferred_language.clone(),
        models,
    })
}

/// Admits only the exact model, configuration, and language shown by `project`.
pub fn validate(
    snapshot: &EngineSnapshot,
    request: &InferenceRequest,
    current_generation: u64,
) -> Result<Admission, Refusal> {
    if snapshot.generation != current_generation || request.generation != current_generation {
        return Err(refuse(RefusalCause::StaleGeneration));
    }
    let model = snapshot
        .models
        .iter()
        .find(|model| model.id == request.model)
        .ok_or_else(|| refuse(RefusalCause::ModelUnknown))?;
    if let Some(cause) = model_status_cause(model) {
        return Err(refuse(cause));
    }
    if request.configuration.model != request.model
        || !model.configurations.contains(&request.configuration)
    {
        return Err(refuse(RefusalCause::ConfigurationUnknown));
    }
    let capability = find_capability(snapshot, &request.configuration)
        .ok_or_else(|| refuse(RefusalCause::ConfigurationUnknown))?;
    if let Some(cause) = configuration_cause(snapshot, model, capability) {
        return Err(refuse(cause));
    }
    match &request.language {
        LanguageChoice::Automatic if capability.auto_language.is_qualified() => {}
        LanguageChoice::Automatic => {
            return Err(refuse(capability_cause(
                capability.auto_language,
                CapabilityTarget::AutomaticLanguage,
            )));
        }
        LanguageChoice::Manual(code) => {
            let language = capability
                .languages
                .iter()
                .find(|language| &language.code == code)
                .ok_or_else(|| refuse(RefusalCause::LanguageUnavailable))?;
            if !language.recognized.is_qualified() {
                return Err(refuse(capability_cause(
                    language.recognized,
                    CapabilityTarget::LanguageRecognition,
                )));
            }
            if !language.manual.is_qualified() {
                return Err(refuse(capability_cause(
                    language.manual,
                    CapabilityTarget::ManualLanguage,
                )));
            }
        }
    }
    Ok(Admission {
        generation: request.generation,
        model: request.model,
        backend: request.configuration.backend,
    })
}

/// Projects the same capability rules while admitting explicitly recognized legacy behavior.
/// The original L1a API remains strict and unchanged for callers that need measured proof only.
pub fn project_with_existing_recognized(
    snapshot: &EngineSnapshot,
    current_generation: u64,
) -> Result<EngineProjection, Refusal> {
    let mut admitted = snapshot.clone();
    promote_existing_recognized(&mut admitted);
    project(&admitted, current_generation)
}

/// Validates a request using measured proof or the narrowly recorded legacy basis.
pub fn validate_with_existing_recognized(
    snapshot: &EngineSnapshot,
    request: &InferenceRequest,
    current_generation: u64,
) -> Result<Admission, Refusal> {
    let mut admitted = snapshot.clone();
    promote_existing_recognized(&mut admitted);
    validate(&admitted, request, current_generation)
}

fn promote_existing_recognized(snapshot: &mut EngineSnapshot) {
    use crate::{
        model_catalog::{Backend, ModelId},
        model_registry::{TURBO_LANGUAGE_INPUTS, configuration_key, model},
    };
    let turbo = model(ModelId::WhisperLargeV3Turbo);
    let historical_configurations =
        [Backend::Cpu, Backend::Gpu].map(|backend| configuration_key(turbo, backend));
    for capability in &mut snapshot.capabilities {
        if capability.key.model != ModelId::WhisperLargeV3Turbo {
            continue;
        }
        if !historical_configurations.contains(&capability.key) {
            if capability.compute.qualification == Qualification::ExistingRecognized {
                capability.compute.support = Support::Unknown;
            }
            if capability.auto_language.qualification == Qualification::ExistingRecognized {
                capability.auto_language.support = Support::Unknown;
            }
            for language in &mut capability.languages {
                if language.recognized.qualification == Qualification::ExistingRecognized {
                    language.recognized.support = Support::Unknown;
                }
                if language.manual.qualification == Qualification::ExistingRecognized {
                    language.manual.support = Support::Unknown;
                }
            }
            continue;
        }
        if capability.compute.qualification == Qualification::ExistingRecognized {
            capability.compute.qualification = Qualification::Qualified;
        }
        if capability.auto_language.qualification == Qualification::ExistingRecognized {
            capability.auto_language.qualification = Qualification::Qualified;
        }
        for language in &mut capability.languages {
            if !TURBO_LANGUAGE_INPUTS
                .iter()
                .any(|(code, _)| language.code.0 == *code)
            {
                if language.recognized.qualification == Qualification::ExistingRecognized {
                    language.recognized.support = Support::Unknown;
                }
                if language.manual.qualification == Qualification::ExistingRecognized {
                    language.manual.support = Support::Unknown;
                }
                continue;
            }
            if language.recognized.qualification == Qualification::ExistingRecognized {
                language.recognized.qualification = Qualification::Qualified;
            }
            if language.manual.qualification == Qualification::ExistingRecognized {
                language.manual.qualification = Qualification::Qualified;
            }
        }
    }
}

fn find_capability<'a>(
    snapshot: &'a EngineSnapshot,
    key: &ConfigurationKey,
) -> Option<&'a ConfigurationCapabilities> {
    snapshot.capabilities.iter().find(|item| &item.key == key)
}

fn model_status_cause(model: &ModelDefinition) -> Option<RefusalCause> {
    match model.availability {
        ModelAvailability::Ready => None,
        ModelAvailability::Missing => Some(RefusalCause::ModelMissing),
        ModelAvailability::Corrupt => Some(RefusalCause::ModelCorrupt),
        ModelAvailability::Cancelled => Some(RefusalCause::ModelCancelled),
    }
}

fn configuration_status(
    snapshot: &EngineSnapshot,
    model: &ModelDefinition,
    capability: Option<&ConfigurationCapabilities>,
) -> (bool, Option<RefusalCause>) {
    if capability.is_some_and(|capability| capability.key.model != model.id) {
        return (false, Some(RefusalCause::ConfigurationUnknown));
    }
    match (model_status_cause(model), capability) {
        (Some(cause), _) => (false, Some(cause)),
        (None, None) => (false, Some(RefusalCause::ConfigurationUnknown)),
        (None, Some(capability)) => match configuration_cause(snapshot, model, capability) {
            Some(cause) => (false, Some(cause)),
            None => (true, None),
        },
    }
}

fn configuration_cause(
    snapshot: &EngineSnapshot,
    model: &ModelDefinition,
    capability: &ConfigurationCapabilities,
) -> Option<RefusalCause> {
    if let Some(cause) = model_status_cause(model) {
        return Some(cause);
    }
    if capability.compute.support == Support::Unsupported {
        return Some(RefusalCause::ConfigurationUnsupported);
    }
    if !capability.compute.is_qualified() {
        return Some(capability_cause(
            capability.compute,
            CapabilityTarget::Compute,
        ));
    }
    snapshot
        .resources
        .iter()
        .find(|observation| {
            capability.resource_requirements.iter().any(|requirement| {
                requirement.kind == observation.kind
                    && requirement.scope == observation.scope
                    && observation.available_bytes < requirement.minimum_bytes
            })
        })
        .map(|observation| RefusalCause::ResourceInsufficient(observation.kind))
}

fn capability_cause(capability: Capability, target: CapabilityTarget) -> RefusalCause {
    if capability.support == Support::Unsupported {
        RefusalCause::CapabilityUnsupported(target)
    } else {
        RefusalCause::CapabilityNotQualified(target)
    }
}

fn refuse(cause: RefusalCause) -> Refusal {
    Refusal { cause }
}
