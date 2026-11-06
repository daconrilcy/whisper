mod live_inspection {
    use super::{RuntimeModelCatalog, test_artifact_root};
    use crate::{
        model_catalog::{LocalModelCatalog, model_sha256_full_read_count},
        parakeet_import::ParakeetWorkerPaths,
    };
    use std::{fs, path::PathBuf};
    use whisper_core::{
        ComputeChoice, Generation, JobConfig, JobId, LanguageChoice,
        gpu_inventory::GpuRuntimeIdentity, model_catalog::ModelId, model_registry::model,
        ports::LiveRequest,
    };

    struct Fixture {
        root: PathBuf,
        turbo: PathBuf,
        parakeet: PathBuf,
        runtime: RuntimeModelCatalog,
    }

    impl Fixture {
        fn new() -> Self {
            let root = test_artifact_root("live-model-inspection");
            let models = root.join("models");
            fs::create_dir_all(&models).unwrap();
            let turbo = models.join(model(ModelId::WhisperLargeV3Turbo).artifact.file_name);
            let parakeet = models.join(model(ModelId::ParakeetTdt06BV3).artifact.file_name);
            fs::write(&turbo, b"synthetic corrupt Turbo").unwrap();
            fs::write(&parakeet, b"synthetic corrupt Parakeet").unwrap();
            let runtime = RuntimeModelCatalog {
                catalog: LocalModelCatalog::new(&models),
                known_historical_turbo_path: r"C:\legacy\ggml-large-v3-turbo.bin".into(),
                parakeet_paths: None,
                gpu_parakeet_paths: None,
            };
            Self {
                root,
                turbo,
                parakeet,
                runtime,
            }
        }

        fn request(&self, id: ModelId, compute: ComputeChoice) -> LiveRequest {
            let path = match id {
                ModelId::WhisperLargeV3Turbo => &self.turbo,
                ModelId::ParakeetTdt06BV3 => &self.parakeet,
            };
            LiveRequest {
                job_id: JobId(0xe501),
                generation: Generation::first(),
                model_path: path.to_string_lossy().into_owned(),
                model_sha256: super::decode_sha256(model(id).artifact.sha256).unwrap(),
                destination: self.root.join("archive").to_string_lossy().into_owned(),
                group_offset_samples: 320,
                auto_stop_after_speech_samples: Some(16_000),
                microphone: whisper_core::MicrophoneChoice::Device("test:chosen-source".into()),
                microphone_gain_db: 3,
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute,
                },
            }
        }

        fn paths(&self, label: &str) -> ParakeetWorkerPaths {
            let root = self.root.join(label);
            ParakeetWorkerPaths {
                executable: root.join("worker.exe"),
                native_dir: root.join("native"),
                model: root.join("personal-model.gguf"),
                runtime_root: root,
            }
        }

        fn assert_reads(&self, turbo: u64, parakeet: u64) {
            assert_eq!(model_sha256_full_read_count(&self.turbo).unwrap(), turbo);
            assert_eq!(
                model_sha256_full_read_count(&self.parakeet).unwrap(),
                parakeet
            );
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn candidate() -> GpuRuntimeIdentity {
        GpuRuntimeIdentity {
            generation: 1,
            pnp_instance_id: "test:gpu".into(),
            dxgi_rank: 0,
            luid: 1,
            pci_id: "0000:01:00.0".into(),
            cuda_ordinal: 0,
            ggml_ordinal: 0,
        }
    }

    #[test]
    fn turbo_binding_and_historical_alias_inspect_only_turbo_and_keep_integrity_refusal() {
        for alias in [false, true] {
            let fixture = Fixture::new();
            let mut request = fixture.request(ModelId::WhisperLargeV3Turbo, ComputeChoice::Cpu);
            if alias {
                request.model_path = fixture
                    .runtime
                    .known_historical_turbo_path
                    .replace('\\', "/")
                    .to_uppercase();
            }
            let original = request.clone();
            assert_eq!(
                fixture.runtime.bind_live(&request).unwrap_err(),
                "Taille ou SHA-256 différent du catalogue."
            );
            assert_eq!(request, original);
            fixture.assert_reads(1, 0);
        }
    }

    #[test]
    fn turbo_binding_preserves_missing_unknown_and_wrong_model_refusals() {
        let fixture = Fixture::new();
        let mut request = fixture.request(ModelId::WhisperLargeV3Turbo, ComputeChoice::Cpu);
        request.model_path = "unknown-model.bin".into();
        assert!(
            fixture
                .runtime
                .bind_live(&request)
                .unwrap_err()
                .starts_with("ModelUnknown:")
        );
        fixture.assert_reads(0, 0);
        request = fixture.request(ModelId::ParakeetTdt06BV3, ComputeChoice::Cpu);
        assert!(
            fixture
                .runtime
                .bind_live(&request)
                .unwrap_err()
                .starts_with("ModelNotQualified:")
        );
        fixture.assert_reads(0, 1);
        fs::remove_file(&fixture.turbo).unwrap();
        request = fixture.request(ModelId::WhisperLargeV3Turbo, ComputeChoice::Cpu);
        assert_eq!(
            fixture.runtime.bind_live(&request).unwrap_err(),
            "Fichier modèle absent."
        );
        assert_eq!(model_sha256_full_read_count(&fixture.parakeet).unwrap(), 1);
    }

    #[test]
    fn parakeet_binding_inspects_only_parakeet_and_keeps_manual_route_refusals() {
        for compute in [ComputeChoice::Cpu, ComputeChoice::Gpu, ComputeChoice::Auto] {
            let fixture = Fixture::new();
            let request = fixture.request(ModelId::ParakeetTdt06BV3, compute);
            let original = request.clone();
            let error = fixture
                .runtime
                .bind_parakeet_live(&request, None)
                .unwrap_err();
            let expected = if compute == ComputeChoice::Gpu {
                "BackendUnavailable: worker GPU Test Parakeet absent"
            } else {
                "ModelNotQualified: worker CPU Test Parakeet absent"
            };
            assert_eq!(error, expected);
            assert_eq!(request, original);
            fixture.assert_reads(0, 1);
        }
    }

    #[test]
    fn parakeet_auto_retries_cpu_after_gpu_route_failure_but_manual_gpu_does_not() {
        for compute in [ComputeChoice::Auto, ComputeChoice::Gpu] {
            let mut fixture = Fixture::new();
            fixture.runtime.gpu_parakeet_paths = Some(fixture.paths("missing-gpu-runtime"));
            let request = fixture.request(ModelId::ParakeetTdt06BV3, compute);
            let error = fixture
                .runtime
                .bind_parakeet_live(&request, Some(candidate()))
                .unwrap_err();
            if compute == ComputeChoice::Auto {
                assert!(
                    error.starts_with("Auto GPU indisponible (MissingRuntime:"),
                    "{error}"
                );
                assert!(
                    error.contains(
                        "CPU indisponible (ModelNotQualified: worker CPU Test Parakeet absent)"
                    ),
                    "{error}"
                );
            } else {
                assert!(error.starts_with("MissingRuntime:"), "{error}");
                assert!(!error.contains("CPU indisponible"));
            }
            fixture.assert_reads(0, 1);
        }
    }

    #[test]
    fn parakeet_binding_rejects_unknown_or_incompatible_model_without_reading_either() {
        let fixture = Fixture::new();
        let mut request = fixture.request(ModelId::WhisperLargeV3Turbo, ComputeChoice::Cpu);
        assert_eq!(
            fixture
                .runtime
                .bind_parakeet_live(&request, None)
                .unwrap_err(),
            "ModelUnknown: la voie microphone Parakeet exige Parakeet TDT v3"
        );
        request.model_path = "unknown-model.gguf".into();
        assert!(
            fixture
                .runtime
                .bind_parakeet_live(&request, None)
                .unwrap_err()
                .starts_with("ModelUnknown:")
        );
        fixture.assert_reads(0, 0);
    }

    #[test]
    fn native_binding_preserves_personal_path_and_request_without_catalog_reads() {
        for gpu_only in [false, true] {
            let mut fixture = Fixture::new();
            let paths = fixture.paths("personal");
            let request = fixture.request(ModelId::ParakeetTdt06BV3, ComputeChoice::Auto);
            if gpu_only {
                fixture.runtime.gpu_parakeet_paths = Some(paths.clone());
            } else {
                fixture.runtime.parakeet_paths = Some(paths.clone());
                fixture.runtime.gpu_parakeet_paths = Some(fixture.paths("other-gpu-path"));
            }
            let bound = fixture.runtime.bind_native_unified_live(&request).unwrap();
            let mut expected = request.clone();
            expected.model_path = paths.model.to_string_lossy().into_owned();
            assert_eq!(bound, expected);
            fixture.assert_reads(0, 0);
            assert!(
                !paths.model.exists(),
                "binding preserves downstream validation responsibility"
            );
        }
    }

    #[test]
    fn native_binding_still_checks_model_sha_and_route_before_returning_a_path() {
        let mut fixture = Fixture::new();
        let mut request = fixture.request(ModelId::ParakeetTdt06BV3, ComputeChoice::Cpu);
        assert_eq!(
            fixture
                .runtime
                .bind_native_unified_live(&request)
                .unwrap_err(),
            "NativeCandidateUnavailable: personal model path is absent"
        );
        fixture.runtime.parakeet_paths = Some(fixture.paths("personal"));
        request.model_sha256 = [0; 32];
        assert!(
            fixture
                .runtime
                .bind_native_unified_live(&request)
                .unwrap_err()
                .starts_with("ModelHashMismatch:")
        );
        request = fixture.request(ModelId::WhisperLargeV3Turbo, ComputeChoice::Cpu);
        assert_eq!(
            fixture
                .runtime
                .bind_native_unified_live(&request)
                .unwrap_err(),
            "ModelUnknown: native unified capture requires Parakeet TDT v3"
        );
        request.model_path = "unknown-model.gguf".into();
        assert!(
            fixture
                .runtime
                .bind_native_unified_live(&request)
                .unwrap_err()
                .starts_with("ModelUnknown:")
        );
        fixture.assert_reads(0, 0);
    }

    #[test]
    fn classification_reads_neither_model_and_does_not_admit_corrupt_parakeet() {
        let fixture = Fixture::new();
        let request = fixture.request(ModelId::ParakeetTdt06BV3, ComputeChoice::Cpu);
        assert!(fixture.runtime.is_parakeet_live_request(&request).unwrap());
        let mut turbo = fixture.request(ModelId::WhisperLargeV3Turbo, ComputeChoice::Cpu);
        assert!(!fixture.runtime.is_parakeet_live_request(&turbo).unwrap());
        turbo.model_path = fixture.runtime.known_historical_turbo_path.clone();
        assert!(!fixture.runtime.is_parakeet_live_request(&turbo).unwrap());
        turbo.model_path = "unknown-model.gguf".into();
        assert!(
            fixture
                .runtime
                .is_parakeet_live_request(&turbo)
                .unwrap_err()
                .starts_with("ModelUnknown:")
        );
        fixture.assert_reads(0, 0);
        assert!(fixture.runtime.bind_parakeet_live(&request, None).is_err());
        fixture.assert_reads(0, 1);
    }

    #[test]
    fn classification_of_a_missing_registered_model_is_still_only_classification() {
        let fixture = Fixture::new();
        let request = fixture.request(ModelId::ParakeetTdt06BV3, ComputeChoice::Cpu);
        fs::remove_file(&fixture.parakeet).unwrap();
        assert!(fixture.runtime.is_parakeet_live_request(&request).unwrap());
        assert!(!fixture.parakeet.exists());
        assert_eq!(model_sha256_full_read_count(&fixture.turbo).unwrap(), 0);
        assert!(fixture.runtime.bind_parakeet_live(&request, None).is_err());
        assert!(!fixture.parakeet.exists());
        assert_eq!(model_sha256_full_read_count(&fixture.turbo).unwrap(), 0);
    }
}
