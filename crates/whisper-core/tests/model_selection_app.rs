use whisper_core::{
    engine_capabilities::{LanguageChoice, LanguageCode},
    model_catalog::{Backend, ModelAvailability, ModelId},
    model_registry::TURBO_LANGUAGE_INPUTS,
    model_selection_app::{
        ComputePreference, InferencePath, ModelCommandGuard, ModelSelectionApp, SelectionIntent,
        SelectionRefusal, is_known_historical_path,
    },
};

fn app() -> ModelSelectionApp {
    ModelSelectionApp::new(whisper_core::model_registry::engine_snapshot(
        1,
        |_| ModelAvailability::Ready,
        vec![],
    ))
}

#[test]
fn turbo_existing_matrix_admits_all_1206_inputs_without_native_inference() {
    let app = app();
    let mut admitted_count = 0;

    for compute in [
        ComputePreference::Cpu,
        ComputePreference::Gpu,
        ComputePreference::Auto,
    ] {
        let effective_backend = match compute {
            ComputePreference::Gpu => Backend::Gpu,
            ComputePreference::Cpu | ComputePreference::Auto => Backend::Cpu,
        };
        for path in [InferencePath::Import, InferencePath::Live] {
            let mut languages = vec![LanguageChoice::Automatic];
            for (code, english_name) in TURBO_LANGUAGE_INPUTS {
                languages.push(LanguageChoice::Manual(LanguageCode(code.into())));
                languages.push(LanguageChoice::Manual(LanguageCode(english_name.into())));
            }
            for language in languages {
                let admitted = app
                    .admit(
                        SelectionIntent {
                            generation: 1,
                            model: ModelId::WhisperLargeV3Turbo,
                            compute,
                            effective_backend,
                            path,
                            language,
                        },
                        1,
                    )
                    .expect("every currently supported Turbo option remains admissible");
                assert_eq!(admitted.path, path);
                assert_eq!(admitted.compute, compute);
                admitted_count += 1;
            }
        }
    }

    assert_eq!(admitted_count, 1_206);
}

#[test]
fn turbo_existing_matrix_admits_and_submits_all_1206_guard_requests() {
    use whisper_core::{
        domain::{ComputeChoice, Generation, JobConfig, JobId, LanguageChoice as RequestLanguage},
        model_registry::model,
        ports::{ImportRequest, LiveRequest},
    };

    let generation = Generation::first();
    let path =
        r"C:\Users\Cyril\AppData\Local\Programs\WhisperBuildDeps\models\ggml-large-v3-turbo.bin";
    let hash = decode_sha256(model(ModelId::WhisperLargeV3Turbo).artifact.sha256);
    let guard = ModelCommandGuard::new(
        whisper_core::model_registry::engine_snapshot(1, |_| ModelAvailability::Ready, vec![]),
        1,
        [path.to_owned()],
    );
    let mut submitted_count = 0;

    for compute in [
        ComputePreference::Cpu,
        ComputePreference::Gpu,
        ComputePreference::Auto,
    ] {
        for path_kind in [InferencePath::Import, InferencePath::Live] {
            let mut languages = vec![("", RequestLanguage::Automatic)];
            for (code, english_name) in TURBO_LANGUAGE_INPUTS {
                languages.push((code, RequestLanguage::Manual(code.into())));
                languages.push((english_name, RequestLanguage::Manual(english_name.into())));
            }
            for (input, language) in languages {
                let request = |job_id| {
                    let config = JobConfig {
                        language: language.clone(),
                        compute: match compute {
                            ComputePreference::Cpu => ComputeChoice::Cpu,
                            ComputePreference::Gpu => ComputeChoice::Gpu,
                            ComputePreference::Auto => ComputeChoice::Auto,
                        },
                    };
                    match path_kind {
                        InferencePath::Import => {
                            let request = ImportRequest {
                                job_id,
                                generation,
                                source_path: "source.wav".into(),
                                source_sha256: None,
                                source_samples: None,
                                model_path: path.into(),
                                model_sha256: hash,
                                destination: "archive".into(),
                                config,
                            };
                            assert!(guard.admit_import(&request, path_kind).is_ok(), "{input}");
                        }
                        InferencePath::Live => {
                            let request = LiveRequest {
                                job_id,
                                generation,
                                model_path: path.into(),
                                model_sha256: hash,
                                destination: "archive".into(),
                                group_offset_samples: 0,
                                auto_stop_after_speech_samples: None,
                                microphone:
                                    whisper_core::microphone::MicrophoneChoice::WindowsDefault,
                                microphone_gain_db: 0,
                                config,
                            };
                            assert!(guard.admit_live(&request).is_ok(), "{input}");
                        }
                    }
                };
                request(JobId(submitted_count as u128));
                submitted_count += 1;
            }
        }
    }
    assert_eq!(submitted_count, 1_206);
}

#[test]
fn existing_recognized_capability_outside_the_historical_matrix_stays_refused() {
    use whisper_core::engine_capabilities::{
        Capability, LanguageCapability, LanguageCode, Qualification, Support,
    };

    let mut snapshot =
        whisper_core::model_registry::engine_snapshot(1, |_| ModelAvailability::Ready, vec![]);
    for capability in &mut snapshot.capabilities {
        if capability.key.model == ModelId::WhisperLargeV3Turbo {
            capability.languages.push(LanguageCapability {
                code: LanguageCode("xx".into()),
                recognized: Capability {
                    support: Support::Supported,
                    qualification: Qualification::ExistingRecognized,
                },
                manual: Capability {
                    support: Support::Supported,
                    qualification: Qualification::ExistingRecognized,
                },
            });
        }
    }
    let mut extra_configuration = snapshot
        .capabilities
        .iter()
        .find(|capability| capability.key.model == ModelId::WhisperLargeV3Turbo)
        .unwrap()
        .clone();
    extra_configuration.key.runtime_build = "unqualified-turbo-build".into();
    snapshot
        .models
        .iter_mut()
        .find(|model| model.id == ModelId::WhisperLargeV3Turbo)
        .unwrap()
        .configurations
        .insert(extra_configuration.key.clone());
    snapshot.capabilities.push(extra_configuration);
    let app = ModelSelectionApp::new(snapshot);
    let projection = app.projection(1).unwrap();
    let turbo = projection
        .models
        .iter()
        .find(|model| model.id == ModelId::WhisperLargeV3Turbo)
        .unwrap();
    assert!(turbo.configurations.iter().all(|configuration| {
        configuration
            .languages
            .iter()
            .find(|language| language.code.0 == "xx")
            .is_some_and(|language| !language.selectable)
    }));
    assert!(
        turbo
            .configurations
            .iter()
            .find(|configuration| configuration.key.runtime_build == "unqualified-turbo-build")
            .is_some_and(|configuration| !configuration.selectable)
    );
    let refused = app.admit(
        SelectionIntent {
            generation: 1,
            model: ModelId::WhisperLargeV3Turbo,
            compute: ComputePreference::Cpu,
            effective_backend: Backend::Cpu,
            path: InferencePath::Import,
            language: LanguageChoice::Manual(LanguageCode("xx".into())),
        },
        1,
    );
    assert!(matches!(refused, Err(SelectionRefusal::Capability(_))));
}

#[test]
fn turbo_snapshot_with_only_a_new_existing_recognized_configuration_is_refused() {
    let mut snapshot =
        whisper_core::model_registry::engine_snapshot(1, |_| ModelAvailability::Ready, vec![]);
    let mut new_configuration = snapshot
        .capabilities
        .iter()
        .find(|capability| {
            capability.key.model == ModelId::WhisperLargeV3Turbo
                && capability.key.backend == Backend::Cpu
        })
        .unwrap()
        .clone();
    new_configuration.key.runtime_build = "new-unqualified-runtime".into();
    snapshot
        .capabilities
        .retain(|capability| capability.key.model != ModelId::WhisperLargeV3Turbo);
    snapshot
        .models
        .iter_mut()
        .find(|model| model.id == ModelId::WhisperLargeV3Turbo)
        .unwrap()
        .configurations = [new_configuration.key.clone()].into_iter().collect();
    snapshot.capabilities.push(new_configuration);

    let app = ModelSelectionApp::new(snapshot);
    let projection = app.projection(1).unwrap();
    let turbo = projection
        .models
        .iter()
        .find(|model| model.id == ModelId::WhisperLargeV3Turbo)
        .unwrap();
    assert_eq!(turbo.configurations.len(), 1);
    assert!(!turbo.configurations[0].selectable);
    assert!(matches!(
        app.admit(
            SelectionIntent {
                generation: 1,
                model: ModelId::WhisperLargeV3Turbo,
                compute: ComputePreference::Cpu,
                effective_backend: Backend::Cpu,
                path: InferencePath::Import,
                language: LanguageChoice::Manual(LanguageCode("en".into())),
            },
            1,
        ),
        Err(SelectionRefusal::Capability(_))
    ));
}

fn decode_sha256(value: &str) -> [u8; 32] {
    let mut bytes = [0; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).unwrap();
    }
    bytes
}

#[test]
fn qualified_parakeet_cpu_gpu_auto_and_invalid_language_selections_are_checked() {
    let app = app();
    let accepted = app.admit(
        SelectionIntent {
            generation: 1,
            model: ModelId::ParakeetTdt06BV3,
            compute: ComputePreference::Cpu,
            effective_backend: Backend::Cpu,
            path: InferencePath::Import,
            language: LanguageChoice::Automatic,
        },
        1,
    );
    assert!(accepted.is_ok());

    for compute in [ComputePreference::Gpu, ComputePreference::Auto] {
        let admitted = app.admit(
            SelectionIntent {
                generation: 1,
                model: ModelId::ParakeetTdt06BV3,
                compute,
                effective_backend: Backend::Gpu,
                path: InferencePath::Import,
                language: LanguageChoice::Automatic,
            },
            1,
        );
        let admitted = admitted.expect("qualified GPU configuration admits manual and Auto");
        assert_eq!(
            admitted.compute, compute,
            "keep requested compute preference"
        );
    }

    for (compute, backend, path, language) in [
        (
            ComputePreference::Cpu,
            Backend::Cpu,
            InferencePath::Import,
            LanguageChoice::Manual(LanguageCode("fr".into())),
        ),
        (
            ComputePreference::Gpu,
            Backend::Gpu,
            InferencePath::Import,
            LanguageChoice::Manual(LanguageCode("en".into())),
        ),
    ] {
        let refused = app.admit(
            SelectionIntent {
                generation: 1,
                model: ModelId::ParakeetTdt06BV3,
                compute,
                effective_backend: backend,
                path,
                language: language.clone(),
            },
            1,
        );
        assert!(
            matches!(refused, Err(SelectionRefusal::Capability(_))),
            "unexpected admission for {compute:?}/{path:?}/{language:?}: {refused:?}"
        );
    }

    let live = app.admit(
        SelectionIntent {
            generation: 1,
            model: ModelId::ParakeetTdt06BV3,
            compute: ComputePreference::Cpu,
            effective_backend: Backend::Cpu,
            path: InferencePath::Live,
            language: LanguageChoice::Automatic,
        },
        1,
    );
    assert!(live.is_ok(), "qualified Parakeet CPU live path: {live:?}");

    let invalid = app.admit(
        SelectionIntent {
            generation: 1,
            model: ModelId::WhisperLargeV3Turbo,
            compute: ComputePreference::Cpu,
            effective_backend: Backend::Cpu,
            path: InferencePath::Import,
            language: LanguageChoice::Manual(LanguageCode("English".into())),
        },
        1,
    );
    assert!(matches!(invalid, Err(SelectionRefusal::Capability(_))));
}

#[test]
fn parakeet_live_is_admitted_only_through_the_live_guard() {
    use whisper_core::{
        domain::{ComputeChoice, Generation, JobConfig, JobId, LanguageChoice as RequestLanguage},
        model_registry::model,
        ports::{ImportRequest, LiveRequest},
    };

    let model = model(ModelId::ParakeetTdt06BV3);
    let path = r"C:\models\parakeet-tdt-v3.gguf";
    let guard = ModelCommandGuard::new_with_model_paths(
        whisper_core::model_registry::engine_snapshot(1, |_| ModelAvailability::Ready, vec![]),
        1,
        [(ModelId::ParakeetTdt06BV3, path.to_owned())],
    );
    let config = JobConfig {
        language: RequestLanguage::Automatic,
        compute: ComputeChoice::Cpu,
    };
    let live = LiveRequest {
        job_id: JobId(3),
        generation: Generation::first(),
        model_path: path.into(),
        model_sha256: decode_sha256(model.artifact.sha256),
        destination: "archive".into(),
        group_offset_samples: 0,
        auto_stop_after_speech_samples: None,
        microphone: whisper_core::microphone::MicrophoneChoice::WindowsDefault,
        microphone_gain_db: 0,
        config: config.clone(),
    };
    assert!(guard.admit_live(&live).is_ok());

    let import = ImportRequest {
        job_id: JobId(4),
        generation: Generation::first(),
        source_path: "source.wav".into(),
        source_sha256: None,
        source_samples: None,
        model_path: path.into(),
        model_sha256: decode_sha256(model.artifact.sha256),
        destination: "archive".into(),
        config,
    };
    assert!(guard.admit_import(&import, InferencePath::Live).is_err());
}

#[test]
fn known_historical_paths_are_case_and_separator_insensitive_but_exact() {
    let known =
        r"C:\Users\Cyril\AppData\Local\Programs\WhisperBuildDeps\models\ggml-large-v3-turbo.bin";
    assert!(is_known_historical_path(
        r"c:/users/cyril/appdata/local/programs/whisperbuilddeps/models/ggml-large-v3-turbo.bin",
        known
    ));
    assert!(!is_known_historical_path(
        r"C:\Users\Cyril\AppData\Local\Programs\WhisperBuildDeps\models\other.bin",
        known
    ));
}
