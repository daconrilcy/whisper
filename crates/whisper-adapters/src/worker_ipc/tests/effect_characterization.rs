#[test]
fn automatic_final_binding_failure_updates_companion_and_clears_identity() {
    let root = automatic_final_test_root("dispatcher-binding-failure");
    fs::create_dir_all(&root).unwrap();
    let mut automatic = automatic_final_request(JobId(0xd501));
    automatic.request.destination = root.to_string_lossy().into_owned();
    automatic.request.model_path = root
        .join("unknown-model.bin")
        .to_string_lossy()
        .into_owned();
    automatic.outputs.txt_path = root.join("auto.txt").to_string_lossy().into_owned();
    automatic.outputs.srt_path = root.join("auto.srt").to_string_lossy().into_owned();
    automatic.outputs.companion_json_path =
        root.join("companion.json").to_string_lossy().into_owned();
    fs::write(
        &automatic.outputs.companion_json_path,
        r#"{"confirmed_intervals":[],"unresolved_intervals":[]}"#,
    )
    .unwrap();
    let catalog = RuntimeModelCatalog {
        catalog: crate::model_catalog::LocalModelCatalog::new(root.join("empty-models")),
        known_historical_turbo_path: root.join("known.bin").to_string_lossy().into_owned(),
        parakeet_paths: None,
        gpu_parakeet_paths: None,
    };
    let mut runtime = ImportRuntime::new_with_catalog(
        WorkerPaths {
            cpu: root.join("cpu"),
            gpu: root.join("gpu"),
        },
        Arc::new(PublicationGate::default()),
        Some(catalog),
    );
    let mut events = VecDeque::new();
    runtime.effect(
        ImportEffect::StartAutomaticFinalTranscription(automatic.clone()),
        &mut events,
    );
    assert!(
        matches!(events.pop_front(), Some(ImportEvent::Failed { job_id, generation, .. }) if job_id == automatic.request.job_id && generation == automatic.request.generation)
    );
    assert!(events.is_empty());
    let identity = (automatic.request.job_id, automatic.request.generation.get());
    assert!(!runtime.automatic_final_outputs.contains_key(&identity));
    assert!(!runtime.gpu_candidates.contains_key(&identity));
    let document: serde_json::Value =
        serde_json::from_slice(&fs::read(&automatic.outputs.companion_json_path).unwrap()).unwrap();
    assert_eq!(
        document["automatic_final_transcription"]["status"],
        "failed_or_unavailable"
    );
    assert!(
        document["automatic_final_transcription"]["diagnostic"]
            .as_str()
            .is_some()
    );
    assert!(!Path::new(&automatic.outputs.txt_path).exists());
    assert!(!Path::new(&automatic.outputs.srt_path).exists());
}

#[test]
fn pagination_effects_preserve_request_and_generation_identity_on_error() {
    let root = automatic_final_test_root("pagination-identities");
    fs::create_dir_all(&root).unwrap();
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: root.join("cpu"),
            gpu: root.join("gpu"),
        },
        Arc::new(PublicationGate::default()),
    );
    let mut events = VecDeque::new();
    let generation = Generation::first();
    runtime.effect(
        ImportEffect::ReadUnifiedDocumentPage {
            job_id: JobId(0xd511),
            generation,
            request_id: 711,
            destination: root.to_string_lossy().into_owned(),
            expected_revision: 17,
            first_sequence: 33,
            limit: 2,
        },
        &mut events,
    );
    assert!(
        matches!(events.pop_front(), Some(ImportEvent::UnifiedDocumentPage { job_id: JobId(0xd511), generation: g, request_id: 711, result: Err(_) }) if g == generation)
    );
    runtime.effect(
        ImportEffect::ReadUnifiedAudioPartsPage {
            job_id: JobId(0xd512),
            generation,
            request_id: 722,
            destination: root.to_string_lossy().into_owned(),
            expected_revision: 18,
            offset: 44,
            limit: 3,
        },
        &mut events,
    );
    assert!(
        matches!(events.pop_front(), Some(ImportEvent::UnifiedAudioPartsPage { job_id: JobId(0xd512), generation: g, request_id: 722, result: Err(_) }) if g == generation)
    );
    assert!(events.is_empty());
}
