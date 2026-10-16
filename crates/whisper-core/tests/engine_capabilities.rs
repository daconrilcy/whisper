use std::collections::BTreeSet;

use whisper_core::engine_capabilities::{
    Admission, Capability, CapabilityTarget, ConfigurationCapabilities, EngineProjection,
    EngineSnapshot, InferenceRequest, LanguageCapability, LanguageChoice, LanguageCode,
    Qualification, Refusal, RefusalCause, ResourceKind, ResourceObservation, ResourceRequirement,
    Support, project as project_at_generation, project_with_existing_recognized,
    validate as validate_at_generation, validate_with_existing_recognized,
};
use whisper_core::model_catalog::{
    Backend, ConfigurationKey, ModelAvailability, ModelDefinition, ModelId,
};

fn project(snapshot: &EngineSnapshot) -> EngineProjection {
    project_at_generation(snapshot, snapshot.generation).unwrap()
}

fn validate(snapshot: &EngineSnapshot, request: &InferenceRequest) -> Result<Admission, Refusal> {
    validate_at_generation(snapshot, request, snapshot.generation)
}

fn key(model: ModelId, backend: Backend, hardware: Option<&str>) -> ConfigurationKey {
    ConfigurationKey {
        model,
        artifact: format!("artifact-{model:?}"),
        artifact_revision: "artifact-v1".to_owned(),
        runtime: "runtime-test".to_owned(),
        runtime_build: "build-1".to_owned(),
        worker: "worker-rust".to_owned(),
        worker_build: "worker-build-1".to_owned(),
        worker_protocol: 1,
        backend,
        hardware_profile: hardware.map(str::to_owned),
        inference_profile: "default".to_owned(),
    }
}

fn state(support: Support, qualification: Qualification) -> Capability {
    Capability {
        support,
        qualification,
    }
}

fn language(code: &str, recognized: Capability, manual: Capability) -> LanguageCapability {
    LanguageCapability {
        code: LanguageCode(code.to_owned()),
        recognized,
        manual,
    }
}

fn definition(
    id: ModelId,
    keys: &[ConfigurationKey],
    availability: ModelAvailability,
) -> ModelDefinition {
    ModelDefinition {
        id,
        display_name: format!("{id:?}"),
        availability,
        configurations: keys.iter().cloned().collect::<BTreeSet<_>>(),
    }
}

fn config(
    key: ConfigurationKey,
    compute: Capability,
    auto: Capability,
    languages: Vec<LanguageCapability>,
    resource_requirements: Vec<ResourceRequirement>,
) -> ConfigurationCapabilities {
    ConfigurationCapabilities {
        key,
        compute,
        auto_language: auto,
        languages,
        resource_requirements,
    }
}

fn make_engine_snapshot(
    generation: u64,
    models: Vec<ModelDefinition>,
    capabilities: Vec<ConfigurationCapabilities>,
    preferred_model: Option<ModelId>,
    preferred_language: Option<LanguageChoice>,
    resources: Vec<ResourceObservation>,
) -> EngineSnapshot {
    EngineSnapshot {
        generation,
        models,
        capabilities,
        preferred_model,
        preferred_language,
        resources,
    }
}

fn request(
    generation: u64,
    model: ModelId,
    configuration: ConfigurationKey,
    language: LanguageChoice,
) -> InferenceRequest {
    InferenceRequest {
        generation,
        model,
        configuration,
        language,
    }
}

#[test]
fn auto_and_manual_language_are_independent_and_projection_matches_admission() {
    let key = key(ModelId::ParakeetTdt06BV3, Backend::Cpu, None);
    let fr = LanguageCode("fr".to_owned());
    let snapshot = make_engine_snapshot(
        7,
        vec![definition(
            ModelId::ParakeetTdt06BV3,
            std::slice::from_ref(&key),
            ModelAvailability::Ready,
        )],
        vec![config(
            key.clone(),
            Capability::QUALIFIED,
            state(Support::Unsupported, Qualification::NotQualified),
            vec![language("fr", Capability::QUALIFIED, Capability::QUALIFIED)],
            vec![],
        )],
        Some(ModelId::ParakeetTdt06BV3),
        None,
        vec![],
    );
    let projection = project(&snapshot);
    let option = &projection.models[0].configurations[0];
    assert!(!option.auto_language_selectable);
    assert_eq!(
        option.auto_language_cause,
        Some(RefusalCause::CapabilityUnsupported(
            CapabilityTarget::AutomaticLanguage
        ))
    );
    assert!(option.languages[0].selectable);
    assert!(
        validate(
            &snapshot,
            &request(
                7,
                ModelId::ParakeetTdt06BV3,
                key.clone(),
                LanguageChoice::Manual(fr.clone())
            )
        )
        .is_ok()
    );
    assert_eq!(
        validate(
            &snapshot,
            &request(
                7,
                ModelId::ParakeetTdt06BV3,
                key.clone(),
                LanguageChoice::Automatic
            )
        )
        .unwrap_err()
        .cause,
        RefusalCause::CapabilityUnsupported(CapabilityTarget::AutomaticLanguage)
    );

    let snapshot = make_engine_snapshot(
        8,
        vec![definition(
            ModelId::ParakeetTdt06BV3,
            std::slice::from_ref(&key),
            ModelAvailability::Ready,
        )],
        vec![config(
            key.clone(),
            Capability::QUALIFIED,
            Capability::QUALIFIED,
            vec![language(
                "fr",
                Capability::QUALIFIED,
                state(Support::Unsupported, Qualification::NotQualified),
            )],
            vec![],
        )],
        Some(ModelId::ParakeetTdt06BV3),
        Some(LanguageChoice::Manual(fr.clone())),
        vec![],
    );
    let option = &project(&snapshot).models[0].configurations[0];
    assert!(option.auto_language_selectable);
    assert!(!option.languages[0].selectable);
    assert_eq!(
        validate(
            &snapshot,
            &request(
                8,
                ModelId::ParakeetTdt06BV3,
                key.clone(),
                LanguageChoice::Manual(fr)
            )
        )
        .unwrap_err()
        .cause,
        RefusalCause::CapabilityUnsupported(CapabilityTarget::ManualLanguage)
    );
    assert!(
        validate(
            &snapshot,
            &request(
                8,
                ModelId::ParakeetTdt06BV3,
                key.clone(),
                LanguageChoice::Automatic
            )
        )
        .is_ok()
    );
}

#[test]
fn no_admissible_language_mode_refuses_launch_and_unknown_is_not_unsupported() {
    let key = key(ModelId::ParakeetTdt06BV3, Backend::Cpu, None);
    let unknown = state(Support::Unknown, Qualification::Unknown);
    let snapshot = make_engine_snapshot(
        1,
        vec![definition(
            ModelId::ParakeetTdt06BV3,
            std::slice::from_ref(&key),
            ModelAvailability::Ready,
        )],
        vec![config(
            key.clone(),
            Capability::QUALIFIED,
            unknown,
            vec![language("fr", Capability::QUALIFIED, unknown)],
            vec![],
        )],
        None,
        None,
        vec![],
    );
    let projection = project(&snapshot);
    assert!(!projection.models[0].selectable);
    assert_eq!(
        projection.models[0].configurations[0].auto_language_cause,
        Some(RefusalCause::CapabilityNotQualified(
            CapabilityTarget::AutomaticLanguage
        ))
    );
    assert_eq!(
        projection.models[0].configurations[0].languages[0].cause,
        Some(RefusalCause::CapabilityNotQualified(
            CapabilityTarget::ManualLanguage
        ))
    );
    assert_ne!(unknown.support, Support::Unsupported);
    assert_eq!(
        validate(
            &snapshot,
            &request(
                1,
                ModelId::ParakeetTdt06BV3,
                key.clone(),
                LanguageChoice::Automatic
            )
        )
        .unwrap_err()
        .cause,
        RefusalCause::CapabilityNotQualified(CapabilityTarget::AutomaticLanguage)
    );
    assert_eq!(
        validate(
            &snapshot,
            &request(
                1,
                ModelId::ParakeetTdt06BV3,
                key,
                LanguageChoice::Manual(LanguageCode("fr".to_owned()))
            )
        )
        .unwrap_err()
        .cause,
        RefusalCause::CapabilityNotQualified(CapabilityTarget::ManualLanguage)
    );
}

#[test]
fn turbo_qualification_does_not_admit_unqualified_parakeet_or_substitute_the_model() {
    let turbo = key(ModelId::WhisperLargeV3Turbo, Backend::Cpu, None);
    let parakeet = key(ModelId::ParakeetTdt06BV3, Backend::Cpu, None);
    let snapshot = make_engine_snapshot(
        3,
        vec![
            definition(
                ModelId::WhisperLargeV3Turbo,
                std::slice::from_ref(&turbo),
                ModelAvailability::Ready,
            ),
            definition(
                ModelId::ParakeetTdt06BV3,
                std::slice::from_ref(&parakeet),
                ModelAvailability::Ready,
            ),
        ],
        vec![
            config(
                turbo.clone(),
                Capability::QUALIFIED,
                Capability::QUALIFIED,
                vec![],
                vec![],
            ),
            config(
                parakeet.clone(),
                state(Support::Supported, Qualification::NotQualified),
                Capability::QUALIFIED,
                vec![],
                vec![],
            ),
        ],
        Some(ModelId::ParakeetTdt06BV3),
        Some(LanguageChoice::Automatic),
        vec![],
    );
    assert!(
        validate(
            &snapshot,
            &request(
                3,
                ModelId::WhisperLargeV3Turbo,
                turbo,
                LanguageChoice::Automatic
            )
        )
        .is_ok()
    );
    assert_eq!(snapshot.preferred_model, Some(ModelId::ParakeetTdt06BV3));
    let projection = project(&snapshot);
    let parakeet_option = projection
        .models
        .iter()
        .find(|option| option.id == ModelId::ParakeetTdt06BV3)
        .unwrap();
    assert!(!parakeet_option.configurations[0].selectable);
    assert!(!parakeet_option.configurations[0].auto_language_selectable);
    assert_eq!(
        validate(
            &snapshot,
            &request(
                3,
                ModelId::ParakeetTdt06BV3,
                parakeet,
                LanguageChoice::Automatic
            )
        )
        .unwrap_err()
        .cause,
        RefusalCause::CapabilityNotQualified(CapabilityTarget::Compute)
    );
}

#[test]
fn invalid_or_grayed_language_is_refused_even_when_injected_directly() {
    let key = key(ModelId::ParakeetTdt06BV3, Backend::Cpu, None);
    let snapshot = make_engine_snapshot(
        2,
        vec![definition(
            ModelId::ParakeetTdt06BV3,
            std::slice::from_ref(&key),
            ModelAvailability::Ready,
        )],
        vec![config(
            key.clone(),
            Capability::QUALIFIED,
            Capability::QUALIFIED,
            vec![language(
                "fr",
                Capability::QUALIFIED,
                state(Support::Unsupported, Qualification::NotQualified),
            )],
            vec![],
        )],
        None,
        None,
        vec![],
    );
    let option = &project(&snapshot).models[0].configurations[0].languages[0];
    assert!(!option.selectable);
    for code in ["fr", "en"] {
        assert_eq!(
            validate(
                &snapshot,
                &request(
                    2,
                    ModelId::ParakeetTdt06BV3,
                    key.clone(),
                    LanguageChoice::Manual(LanguageCode(code.to_owned()))
                )
            )
            .unwrap_err()
            .cause,
            if code == "fr" {
                RefusalCause::CapabilityUnsupported(CapabilityTarget::ManualLanguage)
            } else {
                RefusalCause::LanguageUnavailable
            }
        );
    }
}

#[test]
fn preferences_are_retained_while_unavailable_models_remain_unlaunchable() {
    let turbo = key(ModelId::WhisperLargeV3Turbo, Backend::Cpu, None);
    let parakeet = key(ModelId::ParakeetTdt06BV3, Backend::Cpu, None);
    let preferred_language = LanguageChoice::Manual(LanguageCode("fr".to_owned()));
    let snapshot = make_engine_snapshot(
        9,
        vec![
            definition(
                ModelId::WhisperLargeV3Turbo,
                std::slice::from_ref(&turbo),
                ModelAvailability::Ready,
            ),
            definition(
                ModelId::ParakeetTdt06BV3,
                std::slice::from_ref(&parakeet),
                ModelAvailability::Missing,
            ),
        ],
        vec![
            config(
                turbo,
                Capability::QUALIFIED,
                Capability::QUALIFIED,
                vec![],
                vec![],
            ),
            config(
                parakeet.clone(),
                Capability::QUALIFIED,
                Capability::QUALIFIED,
                vec![],
                vec![],
            ),
        ],
        Some(ModelId::ParakeetTdt06BV3),
        Some(preferred_language.clone()),
        vec![],
    );
    let projection = project(&snapshot);
    assert_eq!(projection.preferred_model, Some(ModelId::ParakeetTdt06BV3));
    assert_eq!(projection.preferred_language, Some(preferred_language));
    assert!(projection.models[1].preferred);
    assert!(!projection.models[1].selectable);
    assert_eq!(
        validate(
            &snapshot,
            &request(
                9,
                ModelId::ParakeetTdt06BV3,
                parakeet,
                LanguageChoice::Automatic
            )
        )
        .unwrap_err()
        .cause,
        RefusalCause::ModelMissing
    );
    assert_eq!(snapshot.preferred_model, Some(ModelId::ParakeetTdt06BV3));
}

#[test]
fn unknown_or_incomparable_resources_do_not_prove_insufficiency_but_comparable_shortfall_does() {
    let cpu = key(ModelId::ParakeetTdt06BV3, Backend::Cpu, None);
    let gpu = key(ModelId::ParakeetTdt06BV3, Backend::Gpu, Some("gpu-a"));
    let requirements = vec![ResourceRequirement {
        kind: ResourceKind::GpuVram,
        minimum_bytes: 100,
        scope: "gpu-a".to_owned(),
    }];
    let make_snapshot = |resources| {
        make_engine_snapshot(
            1,
            vec![definition(
                ModelId::ParakeetTdt06BV3,
                &[cpu.clone(), gpu.clone()],
                ModelAvailability::Ready,
            )],
            vec![
                config(
                    cpu.clone(),
                    Capability::QUALIFIED,
                    Capability::QUALIFIED,
                    vec![],
                    vec![],
                ),
                config(
                    gpu.clone(),
                    Capability::QUALIFIED,
                    Capability::QUALIFIED,
                    vec![],
                    requirements.clone(),
                ),
            ],
            Some(ModelId::ParakeetTdt06BV3),
            None,
            resources,
        )
    };
    for resources in [
        vec![],
        vec![ResourceObservation {
            kind: ResourceKind::GpuVram,
            available_bytes: 1,
            scope: "gpu-b".to_owned(),
        }],
        vec![ResourceObservation {
            kind: ResourceKind::HostRam,
            available_bytes: 1,
            scope: "gpu-a".to_owned(),
        }],
        vec![ResourceObservation {
            kind: ResourceKind::GpuVram,
            available_bytes: 100,
            scope: "gpu-a".to_owned(),
        }],
    ] {
        let snapshot = make_snapshot(resources);
        assert!(
            validate(
                &snapshot,
                &request(
                    1,
                    ModelId::ParakeetTdt06BV3,
                    gpu.clone(),
                    LanguageChoice::Automatic
                )
            )
            .is_ok()
        );
    }
    let snapshot = make_snapshot(vec![ResourceObservation {
        kind: ResourceKind::GpuVram,
        available_bytes: 99,
        scope: "gpu-a".to_owned(),
    }]);
    assert_eq!(
        validate(
            &snapshot,
            &request(1, ModelId::ParakeetTdt06BV3, gpu, LanguageChoice::Automatic)
        )
        .unwrap_err()
        .cause,
        RefusalCause::ResourceInsufficient(ResourceKind::GpuVram)
    );
    assert!(
        validate(
            &snapshot,
            &request(1, ModelId::ParakeetTdt06BV3, cpu, LanguageChoice::Automatic)
        )
        .is_ok()
    );
}

#[test]
fn comparable_shortfall_is_checked_for_ram_vram_and_model_disk() {
    for (kind, backend, hardware) in [
        (ResourceKind::HostRam, Backend::Cpu, None),
        (ResourceKind::GpuVram, Backend::Gpu, Some("gpu-a")),
        (ResourceKind::ModelDisk, Backend::Cpu, None),
    ] {
        let key = key(ModelId::ParakeetTdt06BV3, backend, hardware);
        let snapshot = make_engine_snapshot(
            5,
            vec![definition(
                ModelId::ParakeetTdt06BV3,
                std::slice::from_ref(&key),
                ModelAvailability::Ready,
            )],
            vec![config(
                key.clone(),
                Capability::QUALIFIED,
                Capability::QUALIFIED,
                vec![],
                vec![ResourceRequirement {
                    kind,
                    minimum_bytes: 10,
                    scope: "same-device-or-volume".to_owned(),
                }],
            )],
            None,
            None,
            vec![ResourceObservation {
                kind,
                available_bytes: 9,
                scope: "same-device-or-volume".to_owned(),
            }],
        );
        assert_eq!(
            validate(
                &snapshot,
                &request(5, ModelId::ParakeetTdt06BV3, key, LanguageChoice::Automatic,)
            )
            .unwrap_err()
            .cause,
            RefusalCause::ResourceInsufficient(kind)
        );
    }
}

#[test]
fn stale_generation_result_is_rejected_without_changing_current_preferences() {
    let old_key = key(ModelId::ParakeetTdt06BV3, Backend::Cpu, None);
    let old_snapshot = make_engine_snapshot(
        11,
        vec![definition(
            ModelId::ParakeetTdt06BV3,
            std::slice::from_ref(&old_key),
            ModelAvailability::Ready,
        )],
        vec![config(
            old_key.clone(),
            Capability::QUALIFIED,
            Capability::QUALIFIED,
            vec![],
            vec![],
        )],
        Some(ModelId::ParakeetTdt06BV3),
        Some(LanguageChoice::Automatic),
        vec![],
    );
    let current_key = key(ModelId::WhisperLargeV3Turbo, Backend::Cpu, None);
    let current_snapshot = make_engine_snapshot(
        12,
        vec![definition(
            ModelId::WhisperLargeV3Turbo,
            std::slice::from_ref(&current_key),
            ModelAvailability::Ready,
        )],
        vec![config(
            current_key,
            Capability::QUALIFIED,
            Capability::QUALIFIED,
            vec![],
            vec![],
        )],
        Some(ModelId::WhisperLargeV3Turbo),
        Some(LanguageChoice::Automatic),
        vec![],
    );
    assert_eq!(
        project_at_generation(&old_snapshot, current_snapshot.generation)
            .unwrap_err()
            .cause,
        RefusalCause::StaleGeneration
    );
    assert_eq!(
        validate_at_generation(
            &old_snapshot,
            &request(
                11,
                ModelId::ParakeetTdt06BV3,
                old_key,
                LanguageChoice::Automatic,
            ),
            current_snapshot.generation,
        )
        .unwrap_err()
        .cause,
        RefusalCause::StaleGeneration
    );
    let current_projection =
        project_at_generation(&current_snapshot, current_snapshot.generation).unwrap();
    assert_eq!(current_projection.generation, 12);
    assert_eq!(
        current_projection.preferred_model,
        Some(ModelId::WhisperLargeV3Turbo)
    );
    assert_eq!(
        current_snapshot.preferred_model,
        Some(ModelId::WhisperLargeV3Turbo)
    );
}

#[test]
fn existing_recognized_is_admissible_without_changing_the_l1a_qualified_api() {
    let snapshot =
        whisper_core::model_registry::engine_snapshot(11, |_| ModelAvailability::Ready, vec![]);
    let key = snapshot
        .capabilities
        .iter()
        .find(|capability| {
            capability.key.model == ModelId::WhisperLargeV3Turbo
                && capability.key.backend == Backend::Cpu
        })
        .unwrap()
        .key
        .clone();
    let existing = state(Support::Supported, Qualification::ExistingRecognized);
    let request = request(
        11,
        ModelId::WhisperLargeV3Turbo,
        key,
        LanguageChoice::Manual(LanguageCode("fr".into())),
    );

    assert!(validate(&snapshot, &request).is_err());
    assert!(validate_with_existing_recognized(&snapshot, &request, 11).is_ok());
    assert!(project_with_existing_recognized(&snapshot, 11).is_ok());
    assert!(existing.is_admissible());
    assert!(!state(Support::Unsupported, Qualification::ExistingRecognized).is_admissible());
}
