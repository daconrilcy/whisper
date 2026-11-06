mod queue_inspection_characterization {
    use super::*;
    use crate::model_catalog::model_sha256_full_read_count;
    use whisper_core::{model_catalog::ModelId, model_registry::model};

    struct ModelInspectionFixture(PathBuf);

    impl Drop for ModelInspectionFixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn inspection_fixture(
        label: &str,
    ) -> (ModelInspectionFixture, ImportRequest, PathBuf, PathBuf) {
        let root = test_artifact_root(label);
        fs::create_dir_all(&root).unwrap();
        let models = root.join("models");
        fs::create_dir_all(&models).unwrap();
        let turbo = models.join(model(ModelId::WhisperLargeV3Turbo).artifact.file_name);
        let parakeet = models.join(model(ModelId::ParakeetTdt06BV3).artifact.file_name);
        fs::write(&turbo, b"tiny synthetic Turbo model").unwrap();
        fs::write(&parakeet, b"tiny synthetic Parakeet model").unwrap();

        let mut request = test_request(JobId(0xe201), Generation::first());
        request.source_sha256 = Some([0x31; 32]);
        request.source_samples = Some(16_000);
        request.model_path = parakeet.to_string_lossy().into_owned();
        request.model_sha256 = registered_model_sha(ModelId::ParakeetTdt06BV3);
        request.destination = root.join("archive").to_string_lossy().into_owned();

        (ModelInspectionFixture(root), request, turbo, parakeet)
    }

    fn registered_model_sha(id: ModelId) -> [u8; 32] {
        model(id)
            .artifact
            .sha256
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect::<Vec<_>>()
            .try_into()
            .unwrap()
    }

    fn inspection_runtime(models: &Path) -> ImportRuntime {
        ImportRuntime::new_with_catalog(
            WorkerPaths {
                cpu: models.join("missing-worker-cpu"),
                gpu: models.join("missing-worker-gpu"),
            },
            Arc::new(PublicationGate::default()),
            Some(RuntimeModelCatalog {
                catalog: crate::model_catalog::LocalModelCatalog::new(models),
                known_historical_turbo_path: r"C:\legacy\ggml-large-v3-turbo.bin".into(),
                parakeet_paths: None,
                gpu_parakeet_paths: None,
            }),
        )
    }

    #[test]
    fn queue_processing_inspects_only_its_required_model() {
        let (fixture, request, turbo, parakeet) = inspection_fixture("queue-inspection-scope");
        let mut queue = crate::queue_store::QueueStore::open(&request.destination).unwrap();
        queue.enqueue(request.clone()).unwrap();
        let mut next_model = request.clone();
        next_model.job_id = JobId(0xe202);
        next_model.model_path = turbo.to_string_lossy().into_owned();
        next_model.model_sha256 = registered_model_sha(ModelId::WhisperLargeV3Turbo);
        queue.enqueue(next_model).unwrap();

        let mut runtime = inspection_runtime(&fixture.0.join("models"));
        let (failed_job_id, failed_generation, error) = runtime
            .process_queued(request.clone(), &mut VecDeque::new())
            .expect_err("the count-only Parakeet fixture has no qualified worker");

        assert_eq!(failed_job_id, request.job_id);
        assert_eq!(failed_generation, request.generation);
        assert_eq!(error, "ModelNotQualified: worker CPU Test Parakeet absent");
        assert_eq!(model_sha256_full_read_count(&turbo).unwrap(), 0);
        assert_eq!(model_sha256_full_read_count(&parakeet).unwrap(), 1);
    }

    #[test]
    fn queue_processing_inspects_turbo_without_inspecting_parakeet() {
        let (fixture, mut request, turbo, parakeet) =
            inspection_fixture("queue-turbo-inspection-scope");
        request.model_path = turbo.to_string_lossy().into_owned();
        request.model_sha256 = registered_model_sha(ModelId::WhisperLargeV3Turbo);
        let mut queue = crate::queue_store::QueueStore::open(&request.destination).unwrap();
        queue.enqueue(request.clone()).unwrap();
        let mut next_model = request.clone();
        next_model.job_id = JobId(0xe204);
        next_model.model_path = parakeet.to_string_lossy().into_owned();
        next_model.model_sha256 = registered_model_sha(ModelId::ParakeetTdt06BV3);
        queue.enqueue(next_model).unwrap();

        let mut runtime = inspection_runtime(&fixture.0.join("models"));
        let error = runtime
            .process_queued(request, &mut VecDeque::new())
            .unwrap_err()
            .2;

        assert_eq!(error, "Taille ou SHA-256 différent du catalogue.");
        assert_eq!(model_sha256_full_read_count(&turbo).unwrap(), 1);
        assert_eq!(model_sha256_full_read_count(&parakeet).unwrap(), 0);
    }

    #[test]
    fn queued_item_with_unresolved_model_keeps_model_unknown_error() {
        let (fixture, mut request, turbo, parakeet) = inspection_fixture("queue-unknown-model");
        request.model_path = fixture
            .0
            .join("unknown-model.gguf")
            .to_string_lossy()
            .into_owned();
        let mut queue = crate::queue_store::QueueStore::open(&request.destination).unwrap();
        queue.enqueue(request.clone()).unwrap();

        let mut runtime = inspection_runtime(&fixture.0.join("models"));
        let error = runtime
            .process_queued(request, &mut VecDeque::new())
            .expect_err("unknown queued model must be refused")
            .2;

        assert!(error.starts_with("ModelUnknown:"), "{error}");
        assert_eq!(model_sha256_full_read_count(&turbo).unwrap(), 0);
        assert_eq!(model_sha256_full_read_count(&parakeet).unwrap(), 0);
    }

    #[test]
    fn queued_item_with_missing_model_keeps_explicit_unavailable_error() {
        let (fixture, mut request, turbo, parakeet) = inspection_fixture("queue-missing-model");
        request.model_path = turbo.to_string_lossy().into_owned();
        request.model_sha256 = registered_model_sha(ModelId::WhisperLargeV3Turbo);
        fs::remove_file(&turbo).unwrap();
        let mut queue = crate::queue_store::QueueStore::open(&request.destination).unwrap();
        queue.enqueue(request.clone()).unwrap();

        let mut runtime = inspection_runtime(&fixture.0.join("models"));
        let error = runtime
            .process_queued(request, &mut VecDeque::new())
            .expect_err("missing queued model must be refused")
            .2;

        assert_eq!(error, "Fichier modèle absent.");
        assert_eq!(model_sha256_full_read_count(&parakeet).unwrap(), 0);
        assert!(!turbo.exists());
    }

    #[test]
    fn interrupted_recovery_inspects_only_its_required_model() {
        let (fixture, previous, turbo, parakeet) = inspection_fixture("recovery-inspection-scope");
        let mut queue = crate::queue_store::QueueStore::open(&previous.destination).unwrap();
        queue.enqueue(previous.clone()).unwrap();
        queue
            .set_status(
                previous.job_id,
                previous.generation,
                crate::queue_store::QueueStatus::Interrupted,
            )
            .unwrap();
        let mut resumed = previous.clone();
        resumed.generation = previous.generation.next().unwrap();

        let mut runtime = inspection_runtime(&fixture.0.join("models"));
        let mut events = VecDeque::new();
        let resumed_identity = (resumed.job_id, resumed.generation);
        let _ = runtime.resume_interrupted(Box::new(previous), resumed, &mut events);

        assert_eq!(model_sha256_full_read_count(&turbo).unwrap(), 0);
        assert_eq!(model_sha256_full_read_count(&parakeet).unwrap(), 1);
        assert!(events.iter().any(|event| matches!(event, ImportEvent::Failed { job_id, generation, message }
        if *job_id == resumed_identity.0
            && *generation == resumed_identity.1
            && message == "Modèle indisponible : ModelNotQualified: worker CPU Test Parakeet absent")),
        "count-only recovery preserves the exact worker qualification diagnostic and resume identity: {events:?}"
    );
    }

    #[test]
    fn interrupted_turbo_recovery_accepts_historical_alias_and_preserves_durable_identity() {
        let (fixture, mut previous, turbo, parakeet) = inspection_fixture("recovery-turbo-alias");
        previous.model_path = turbo.to_string_lossy().into_owned();
        previous.model_sha256 = registered_model_sha(ModelId::WhisperLargeV3Turbo);
        let mut queue = crate::queue_store::QueueStore::open(&previous.destination).unwrap();
        queue.enqueue(previous.clone()).unwrap();
        queue
            .set_status(
                previous.job_id,
                previous.generation,
                crate::queue_store::QueueStatus::Interrupted,
            )
            .unwrap();
        let durable_path = queue.entries()[0].request.model_path.clone();
        let mut resumed = previous.clone();
        resumed.generation = previous.generation.next().unwrap();
        resumed.model_path = r"C:\legacy\ggml-large-v3-turbo.bin".into();
        resumed.model_sha256 = registered_model_sha(ModelId::WhisperLargeV3Turbo);

        let mut runtime = inspection_runtime(&fixture.0.join("models"));
        let mut events = VecDeque::new();
        let _ = runtime.resume_interrupted(Box::new(previous.clone()), resumed, &mut events);

        assert_eq!(model_sha256_full_read_count(&turbo).unwrap(), 1);
        assert_eq!(model_sha256_full_read_count(&parakeet).unwrap(), 0);
        let durable = crate::queue_store::QueueStore::open(&previous.destination).unwrap();
        assert_eq!(durable.entries()[0].request.model_path, durable_path);
        assert_eq!(durable.entries()[0].request.model_path, previous.model_path);
        assert!(
            events
                .iter()
                .any(|event| matches!(event, ImportEvent::Failed { message, .. }
        if message == "Modèle indisponible : Taille ou SHA-256 différent du catalogue.")),
            "corrupt Turbo fixture keeps its exact catalogue integrity diagnostic: {events:?}"
        );
    }

    #[test]
    fn interrupted_turbo_recovery_rejects_different_durable_model() {
        let (fixture, mut previous, turbo, parakeet) =
            inspection_fixture("recovery-turbo-model-change");
        previous.model_path = turbo.to_string_lossy().into_owned();
        previous.model_sha256 = registered_model_sha(ModelId::WhisperLargeV3Turbo);
        let mut queue = crate::queue_store::QueueStore::open(&previous.destination).unwrap();
        queue.enqueue(previous.clone()).unwrap();
        queue
            .set_status(
                previous.job_id,
                previous.generation,
                crate::queue_store::QueueStatus::Interrupted,
            )
            .unwrap();
        let mut resumed = previous.clone();
        resumed.generation = previous.generation.next().unwrap();
        resumed.model_path = parakeet.to_string_lossy().into_owned();
        resumed.model_sha256 = registered_model_sha(ModelId::ParakeetTdt06BV3);

        let mut runtime = inspection_runtime(&fixture.0.join("models"));
        let mut events = VecDeque::new();
        let _ = runtime.resume_interrupted(Box::new(previous), resumed, &mut events);

        assert_eq!(model_sha256_full_read_count(&turbo).unwrap(), 1);
        assert_eq!(model_sha256_full_read_count(&parakeet).unwrap(), 1);
        assert!(events.iter().any(|event| matches!(event, ImportEvent::Failed { message, .. }
        if message == "ModelIdentityChanged: la reprise ne correspond pas au modèle Turbo durable")),
        "a different resume model remains explicitly rejected: {events:?}"
    );
    }
}
