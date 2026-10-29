#[test]
fn queue_effect_scan_and_remove_preserve_remaining_fifo_entry() {
    let destination = test_artifact_root("queue-effect-scan-remove");
    fs::create_dir_all(&destination).unwrap();
    let mut first = test_request(JobId(0xd101), Generation::first());
    first.destination = destination.to_string_lossy().into_owned();
    first.source_sha256 = Some([1; 32]);
    first.source_samples = Some(16000);
    let mut second = first.clone();
    second.job_id = JobId(0xd102);
    {
        let mut queue = crate::queue_store::QueueStore::open(&destination).unwrap();
        queue.enqueue(first.clone()).unwrap();
        queue.enqueue(second.clone()).unwrap();
    }
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: destination.join("cpu"),
            gpu: destination.join("gpu"),
        },
        Arc::new(PublicationGate::default()),
    );
    let mut events = VecDeque::new();
    runtime.effect(
        ImportEffect::ScanQueue {
            destination: first.destination.clone(),
        },
        &mut events,
    );
    let Some(ImportEvent::Queue(entries)) = events.pop_front() else {
        panic!("expected scanned queue")
    };
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.request.job_id)
            .collect::<Vec<_>>(),
        vec![first.job_id, second.job_id]
    );
    runtime.effect(
        ImportEffect::RemoveQueued {
            job_id: first.job_id,
            generation: first.generation,
            destination: first.destination.clone(),
        },
        &mut events,
    );
    let Some(ImportEvent::Queue(entries)) = events.pop_front() else {
        panic!("expected queue after removal")
    };
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.request.job_id)
            .collect::<Vec<_>>(),
        vec![second.job_id]
    );
    runtime.effect(
        ImportEffect::ScanQueue {
            destination: first.destination.clone(),
        },
        &mut events,
    );
    let Some(ImportEvent::Queue(entries)) = events.pop_front() else {
        panic!("expected persisted queue after scan")
    };
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.request.job_id)
            .collect::<Vec<_>>(),
        vec![second.job_id]
    );
}
