use std::collections::BTreeSet;

use whisper_core::engine_capabilities::{
    Admission, Capability, ConfigurationCapabilities, EngineProjection, EngineSnapshot,
    InferenceRequest, LanguageChoice, Refusal, RefusalCause, project as project_at_generation,
    validate as validate_at_generation,
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

fn key(
    model: ModelId,
    revision: &str,
    backend: Backend,
    hardware: Option<&str>,
) -> ConfigurationKey {
    ConfigurationKey {
        model,
        artifact: format!("artifact-{model:?}"),
        artifact_revision: revision.to_owned(),
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

fn model(id: ModelId, key: ConfigurationKey, availability: ModelAvailability) -> ModelDefinition {
    ModelDefinition {
        id,
        display_name: format!("{id:?}"),
        availability,
        configurations: BTreeSet::from([key]),
    }
}

fn qualified_capability(key: ConfigurationKey) -> ConfigurationCapabilities {
    ConfigurationCapabilities {
        key,
        compute: Capability::QUALIFIED,
        auto_language: Capability::QUALIFIED,
        languages: vec![],
        resource_requirements: vec![],
    }
}

#[test]
fn catalog_is_closed_to_the_two_planned_model_identities() {
    assert_eq!(ModelId::ALL.len(), 2);
    assert_ne!(ModelId::ALL[0], ModelId::ALL[1]);
    assert_eq!(
        ModelId::ALL,
        [ModelId::WhisperLargeV3Turbo, ModelId::ParakeetTdt06BV3]
    );
}

#[test]
fn configuration_identity_includes_every_qualification_dimension() {
    let original = key(
        ModelId::ParakeetTdt06BV3,
        "rev-1",
        Backend::Gpu,
        Some("gpu-a"),
    );
    let variants = [
        key(
            ModelId::ParakeetTdt06BV3,
            "rev-2",
            Backend::Gpu,
            Some("gpu-a"),
        ),
        key(ModelId::ParakeetTdt06BV3, "rev-1", Backend::Cpu, None),
        key(
            ModelId::ParakeetTdt06BV3,
            "rev-1",
            Backend::Gpu,
            Some("gpu-b"),
        ),
    ];
    assert!(variants.iter().all(|variant| variant != &original));
    let mut changed_runtime = original.clone();
    changed_runtime.runtime = "runtime-next".to_owned();
    assert_ne!(original, changed_runtime);
    let mut changed_artifact = original.clone();
    changed_artifact.artifact = "artifact-other".to_owned();
    assert_ne!(original, changed_artifact);
    let mut changed_build = original.clone();
    changed_build.runtime_build = "build-2".to_owned();
    assert_ne!(original, changed_build);
    let mut changed_worker = original.clone();
    changed_worker.worker_build = "worker-build-2".to_owned();
    assert_ne!(original, changed_worker);
    changed_worker.worker_build = original.worker_build.clone();
    changed_worker.worker = "other-worker".to_owned();
    assert_ne!(original, changed_worker);
    changed_worker.worker = original.worker.clone();
    changed_worker.worker_protocol = 2;
    assert_ne!(original, changed_worker);
    let mut changed_profile = original.clone();
    changed_profile.inference_profile = "long-audio".to_owned();
    assert_ne!(original, changed_profile);
}

#[test]
fn availability_states_keep_both_catalog_entries_and_disable_unavailable_models() {
    let statuses = [
        (ModelAvailability::Ready, None),
        (ModelAvailability::Missing, Some(RefusalCause::ModelMissing)),
        (ModelAvailability::Corrupt, Some(RefusalCause::ModelCorrupt)),
        (
            ModelAvailability::Cancelled,
            Some(RefusalCause::ModelCancelled),
        ),
    ];
    for (availability, expected_cause) in statuses {
        let turbo = key(ModelId::WhisperLargeV3Turbo, "turbo-v1", Backend::Cpu, None);
        let parakeet = key(ModelId::ParakeetTdt06BV3, "para-v1", Backend::Cpu, None);
        let snapshot = EngineSnapshot {
            generation: 4,
            models: vec![
                model(ModelId::WhisperLargeV3Turbo, turbo.clone(), availability),
                model(
                    ModelId::ParakeetTdt06BV3,
                    parakeet.clone(),
                    ModelAvailability::Ready,
                ),
            ],
            capabilities: vec![qualified_capability(turbo), qualified_capability(parakeet)],
            preferred_model: Some(ModelId::WhisperLargeV3Turbo),
            preferred_language: None,
            resources: vec![],
        };
        let projection = project(&snapshot);
        assert_eq!(projection.models.len(), 2);
        let option = projection
            .models
            .iter()
            .find(|option| option.id == ModelId::WhisperLargeV3Turbo)
            .unwrap();
        assert_eq!(option.cause, expected_cause);
        assert_eq!(option.selectable, expected_cause.is_none());
        assert!(
            projection
                .models
                .iter()
                .any(|option| option.id == ModelId::ParakeetTdt06BV3)
        );
    }
}

#[test]
fn a_qualification_for_one_configuration_does_not_transfer_to_another_revision() {
    let qualified_key = key(ModelId::ParakeetTdt06BV3, "rev-1", Backend::Cpu, None);
    let new_key = key(ModelId::ParakeetTdt06BV3, "rev-2", Backend::Cpu, None);
    let snapshot = EngineSnapshot {
        generation: 1,
        models: vec![model(
            ModelId::ParakeetTdt06BV3,
            new_key.clone(),
            ModelAvailability::Ready,
        )],
        capabilities: vec![qualified_capability(qualified_key)],
        preferred_model: Some(ModelId::ParakeetTdt06BV3),
        preferred_language: None,
        resources: vec![],
    };
    let projection = project(&snapshot);
    let option = &projection.models[0].configurations[0];
    assert_eq!(option.cause, Some(RefusalCause::ConfigurationUnknown));
    assert!(!option.selectable);
    assert_eq!(
        validate(
            &snapshot,
            &InferenceRequest {
                generation: 1,
                model: ModelId::ParakeetTdt06BV3,
                configuration: new_key,
                language: LanguageChoice::Automatic,
            }
        )
        .unwrap_err()
        .cause,
        RefusalCause::ConfigurationUnknown
    );
}

#[test]
fn qualification_does_not_transfer_to_a_different_worker_build() {
    let qualified_key = key(ModelId::ParakeetTdt06BV3, "rev-1", Backend::Cpu, None);
    let mut new_worker_key = qualified_key.clone();
    new_worker_key.worker_build = "worker-build-2".to_owned();
    assert_eq!(new_worker_key.worker, qualified_key.worker);
    assert_eq!(
        new_worker_key.worker_protocol,
        qualified_key.worker_protocol
    );
    assert_ne!(new_worker_key, qualified_key);

    let snapshot = EngineSnapshot {
        generation: 10,
        models: vec![model(
            ModelId::ParakeetTdt06BV3,
            new_worker_key.clone(),
            ModelAvailability::Ready,
        )],
        capabilities: vec![qualified_capability(qualified_key)],
        preferred_model: Some(ModelId::ParakeetTdt06BV3),
        preferred_language: None,
        resources: vec![],
    };
    assert_eq!(
        project(&snapshot).models[0].configurations[0].cause,
        Some(RefusalCause::ConfigurationUnknown)
    );
    assert_eq!(
        validate(
            &snapshot,
            &InferenceRequest {
                generation: 10,
                model: ModelId::ParakeetTdt06BV3,
                configuration: new_worker_key,
                language: LanguageChoice::Automatic,
            }
        )
        .unwrap_err()
        .cause,
        RefusalCause::ConfigurationUnknown
    );
}
