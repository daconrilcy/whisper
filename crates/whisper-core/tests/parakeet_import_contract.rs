use whisper_core::{
    domain::{ComputeChoice, Generation, JobConfig, JobId, LanguageChoice},
    engine_capabilities::{Capability, LanguageCapability, LanguageCode},
    model_catalog::{Backend, ModelAvailability, ModelId},
    model_registry::{engine_snapshot, model, personal_configuration_key},
    model_selection_app::{InferencePath, ModelCommandGuard},
    ports::{ImportRequest, LiveRequest},
};

const PARA_PATH: &str = r"C:\models\parakeet-tdt-0.6b-v3.q8_0.gguf";

fn qualified_guard() -> ModelCommandGuard {
    let mut snapshot = engine_snapshot(1, |_| ModelAvailability::Ready, vec![]);
    let config = snapshot
        .capabilities
        .iter_mut()
        .find(|capability| {
            capability.key.model == ModelId::ParakeetTdt06BV3
                && capability.key.backend == Backend::Cpu
        })
        .unwrap();
    config.key.runtime = "nemo-speech-cpp".into();
    config.key.runtime_build = "runtime-sha".into();
    config.key.worker = "parakeet-worker".into();
    config.key.worker_build = "worker-sha".into();
    config.key.worker_protocol = 1;
    config.key.inference_profile = "import-single-pass".into();
    config.compute = Capability::QUALIFIED;
    config.auto_language = Capability::QUALIFIED;
    config.languages = ["fr", "en"]
        .into_iter()
        .map(|language| LanguageCapability {
            code: LanguageCode(language.into()),
            recognized: Capability::QUALIFIED,
            manual: Capability {
                support: whisper_core::engine_capabilities::Support::Unsupported,
                qualification: whisper_core::engine_capabilities::Qualification::NotQualified,
            },
        })
        .collect();
    let key = config.key.clone();
    snapshot
        .models
        .iter_mut()
        .find(|model| model.id == ModelId::ParakeetTdt06BV3)
        .unwrap()
        .configurations = [key].into_iter().collect();
    ModelCommandGuard::new_with_model_paths(
        snapshot,
        1,
        [(ModelId::ParakeetTdt06BV3, PARA_PATH.to_owned())],
    )
}

fn import(config: JobConfig, model_path: &str) -> ImportRequest {
    ImportRequest {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
        job_id: JobId(1),
        generation: Generation::first(),
        source_path: "source.wav".into(),
        source_sha256: None,
        source_samples: None,
        model_path: model_path.into(),
        model_sha256: decode_sha256(model(ModelId::ParakeetTdt06BV3).artifact.sha256),
        destination: "archive".into(),
        config,
    }
}

#[test]
fn only_qualified_cpu_import_with_automatic_language_is_admitted() {
    let guard = qualified_guard();
    let accepted = import(
        JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        },
        PARA_PATH,
    );
    assert!(guard.admit_import(&accepted, InferencePath::Import).is_ok());
    let mut resumed = accepted.clone();
    resumed.generation = resumed.generation.next().unwrap();
    assert!(guard.admit_import(&resumed, InferencePath::Import).is_ok());

    for config in [
        JobConfig {
            language: LanguageChoice::Manual("fr".into()),
            compute: ComputeChoice::Cpu,
        },
        JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Gpu,
        },
        JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Auto,
        },
    ] {
        let request = import(config, PARA_PATH);
        assert!(guard.admit_import(&request, InferencePath::Import).is_err());
    }

    let request = import(
        JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        },
        PARA_PATH,
    );
    assert!(guard.admit_import(&request, InferencePath::Live).is_err());
    assert!(
        guard
            .admit_import(
                &import(request.config.clone(), "C:/unknown.gguf"),
                InferencePath::Import
            )
            .is_err()
    );
}

#[test]
fn parakeet_import_admits_long_product_input_but_rejects_wrong_hash() {
    let guard = qualified_guard();
    let mut long = import(
        JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        },
        PARA_PATH,
    );
    long.source_samples = Some(whisper_core::parakeet_ipc::PARAKEET_MAX_AUDIO_SAMPLES as u64 + 1);
    assert!(guard.admit_import(&long, InferencePath::Import).is_ok());
    long.source_samples = None;
    long.model_sha256[0] ^= 1;
    assert!(guard.admit_import(&long, InferencePath::Import).is_err());
}

#[test]
fn parakeet_duration_selects_adapter_profile_not_core_admission() {
    let guard = qualified_guard();
    let mut request = import(
        JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        },
        PARA_PATH,
    );
    request.source_samples = Some(whisper_core::parakeet_ipc::PARAKEET_MAX_AUDIO_SAMPLES as u64);
    assert!(guard.admit_import(&request, InferencePath::Import).is_ok());
    request.source_samples =
        Some(whisper_core::parakeet_ipc::PARAKEET_MAX_AUDIO_SAMPLES as u64 + 1);
    assert!(guard.admit_import(&request, InferencePath::Import).is_ok());
}

#[test]
fn current_r6_cpu_and_l3b_cuda_test_keys_are_qualified() {
    let snapshot = engine_snapshot(1, |_| ModelAvailability::Ready, vec![]);
    let cpu = snapshot
        .capabilities
        .iter()
        .find(|capability| {
            capability.key.model == ModelId::ParakeetTdt06BV3
                && capability.key.backend == Backend::Cpu
        })
        .unwrap();
    assert_eq!(cpu.key.runtime, "nemo-speech-asr 0.2.0");
    assert_eq!(
        cpu.key.runtime_build,
        "d5a2cc8ef441dd5934972adae00f997241bea17a7bc5977a8c7f0122aaa958ff"
    );
    assert_eq!(
        cpu.key.worker_build,
        "1d8f57b9e92d69629618be422077a0508303b9ae10d622b5f5aaad8867e103c0"
    );
    assert_ne!(
        cpu.key.worker_build, "545a4830755da613a1255be97fbc6a6a513ef837d669d8a9870d3bbee9dfd587",
        "retired L2a-2 key is refused for a new import"
    );
    assert_eq!(cpu.key.worker_protocol, 2);
    assert_eq!(cpu.key.inference_profile, "import-single-pass");
    assert!(cpu.compute.is_admissible());
    assert!(cpu.auto_language.is_admissible());
    assert_eq!(cpu.languages.len(), 2);
    assert!(
        cpu.languages
            .iter()
            .all(|language| language.recognized.is_admissible())
    );

    let gpu = snapshot
        .capabilities
        .iter()
        .find(|capability| {
            capability.key.model == ModelId::ParakeetTdt06BV3
                && capability.key.backend == Backend::Gpu
        })
        .unwrap();
    assert_eq!(
        gpu.key.runtime_build,
        "cbf7543a711a341df4387fb9047fcd55d3d5c572465cbe9bbb7d14b5963e6b10"
    );
    assert_eq!(
        gpu.key.worker_build,
        "5054612a05fbfd1039e108def3aa4e598268d2b4274b8c1dc96cd6b8e3f2cd6d"
    );
    assert_eq!(gpu.key.backend, Backend::Gpu);
    assert!(gpu.compute.is_admissible());
    assert!(gpu.auto_language.is_admissible());
    assert_eq!(gpu.languages.len(), 2);
    assert!(gpu.languages.iter().all(|language| {
        language.recognized.is_admissible() && !language.manual.is_admissible()
    }));
}

#[test]
fn personal_release_keys_match_the_qualified_candidate_b_workers_and_dll_sets() {
    let record = model(ModelId::ParakeetTdt06BV3);
    let cpu = personal_configuration_key(record, Backend::Cpu);
    assert_eq!(cpu.runtime, "nemo-speech-asr 0.2.0");
    assert_eq!(
        cpu.runtime_build,
        "d5a2cc8ef441dd5934972adae00f997241bea17a7bc5977a8c7f0122aaa958ff"
    );
    assert_eq!(
        cpu.worker_build,
        "84c371d4df55043ada4a1d96b80180ed0c03b208410c599262c8ea3762304e60"
    );
    assert_eq!(cpu.backend, Backend::Cpu);
    assert_eq!(cpu.inference_profile, "import-single-pass");

    let gpu = personal_configuration_key(record, Backend::Gpu);
    assert_eq!(
        gpu.runtime_build,
        "cbf7543a711a341df4387fb9047fcd55d3d5c572465cbe9bbb7d14b5963e6b10"
    );
    assert_eq!(
        gpu.worker_build,
        "bf58eb9d7300c444864347496b5a27e8584da3d30d634a759be99a598eda5ce4"
    );
    assert_eq!(gpu.backend, Backend::Gpu);
    assert_eq!(gpu.hardware_profile.as_deref(), Some("selected-gpu"));
    assert_eq!(gpu.inference_profile, "import-single-pass");
    assert_ne!(cpu, gpu);
}

#[test]
fn turbo_live_guard_remains_available_through_legacy_constructor() {
    let turbo = model(ModelId::WhisperLargeV3Turbo);
    let guard = ModelCommandGuard::new(
        engine_snapshot(1, |_| ModelAvailability::Ready, vec![]),
        1,
        [r"C:\legacy\turbo.bin".to_owned()],
    );
    let request = LiveRequest {
        job_id: JobId(2),
        generation: Generation::first(),
        model_path: r"C:\legacy\turbo.bin".into(),
        model_sha256: decode_sha256(turbo.artifact.sha256),
        destination: "archive".into(),
        group_offset_samples: 0,
        auto_stop_after_speech_samples: None,
        microphone: whisper_core::microphone::MicrophoneChoice::WindowsDefault,
        microphone_gain_db: 0,
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        },
    };
    assert!(guard.admit_live(&request).is_ok());
}

fn decode_sha256(value: &str) -> [u8; 32] {
    let mut bytes = [0; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).unwrap();
    }
    bytes
}
