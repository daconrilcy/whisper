#[test]
fn duplicate_confirmation_short_refusals_preserve_runtime_identity() {
    let request = test_request(JobId(0xd201), Generation::first());
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: PathBuf::from("unused-cpu"),
            gpu: PathBuf::from("unused-gpu"),
        },
        Arc::new(PublicationGate::default()),
    );
    runtime.require_snapshot_registry = true;
    let key = (request.job_id, request.generation.get());
    runtime.gpu_candidates.insert(key, None);
    let mut prompt = DuplicateImportPrompt {
        request: request.clone(),
        source_sha256: String::new(),
        source_samples: 0,
        publications: Vec::new(),
        export_basename: String::new(),
        revalidation_notice: None,
        resume_from: None,
        reserved_export_basename: None,
    };
    let mut events = VecDeque::new();
    runtime.effect(
        ImportEffect::ConfirmDuplicateImport {
            prompt: Box::new(prompt.clone()),
            choice: DuplicateImportChoice::CreateNew,
        },
        &mut events,
    );
    assert!(
        matches!(events.pop_front(), Some(ImportEvent::Failed { job_id, generation, message }) if job_id == request.job_id && generation == request.generation && message.contains("SourceSnapshotRegistryUnavailable"))
    );
    assert!(events.is_empty());
    assert!(runtime.gpu_candidates.contains_key(&key));
    prompt.resume_from = Some(request.clone());
    runtime.effect(
        ImportEffect::ConfirmDuplicateImport {
            prompt: Box::new(prompt),
            choice: DuplicateImportChoice::Replace,
        },
        &mut events,
    );
    assert!(
        matches!(events.pop_front(), Some(ImportEvent::Failed { job_id, generation, message }) if job_id == request.job_id && generation == request.generation && message.contains("missing its reserved name"))
    );
    assert!(events.is_empty());
    assert!(runtime.gpu_candidates.contains_key(&key));
}
