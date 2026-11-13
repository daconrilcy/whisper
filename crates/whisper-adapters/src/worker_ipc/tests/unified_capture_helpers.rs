use super::*;

type ReservedSessions = (
    whisper_core::parakeet_scheduler::SchedulerSession,
    whisper_core::parakeet_scheduler::SchedulerSession,
);
type ReservedControls = (
    Arc<AtomicU64>,
    Arc<AtomicU64>,
    Arc<Mutex<Vec<String>>>,
    std::sync::mpsc::Receiver<String>,
    std::sync::mpsc::SyncSender<()>,
);

pub(super) struct ReservedFinalScenario {
    root: PathBuf,
    runtime: ImportRuntime,
    ready_gate: Arc<(Mutex<bool>, std::sync::Condvar)>,
    recycled_ready_gate: Option<Arc<(Mutex<bool>, std::sync::Condvar)>>,
    sessions: ReservedSessions,
    controls: ReservedControls,
    child_observations: Arc<Mutex<Vec<ControlledChildObservation>>>,
    ready_labels: HashMap<(JobId, Generation), Vec<String>>,
    persisted_sequences: HashMap<(JobId, Generation), u64>,
    published_sessions: HashSet<(JobId, Generation)>,
    completed_sessions: HashSet<(JobId, Generation)>,
}

pub(super) fn prepare_reserved_final_scenario() -> ReservedFinalScenario {
    const A_ID: u128 = 9_424;
    const B_ID: u128 = 9_425;
    let root = std::env::temp_dir().join(format!(
        "parakeet-unified-reserved-final-preview-{}",
        std::process::id()
    ));
    let archive_root = root.join("archive");
    std::fs::create_dir_all(&archive_root).unwrap();
    let source_a = root.join("source-a.wav");
    let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_a).unwrap();
    writer.append(&vec![10_000_i16; 121 * 16_000]).unwrap();
    writer.finish().unwrap();
    let source_b = root.join("source-b.wav");
    let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_b).unwrap();
    writer.append(&vec![0_i16; 5 * 16_000]).unwrap();
    writer.finish().unwrap();
    let request_a = unified_import_request(&archive_root, &source_a, A_ID);
    let request_b = unified_import_request(&archive_root, &source_b, B_ID);
    let session_a = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request_a.job_id,
        generation: request_a.generation,
    };
    let session_b = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request_b.job_id,
        generation: request_b.generation,
    };
    let (worker, calls, maximum_active, call_sources, held_rx, release_tx) = held_worker(1);
    let child_observations = Arc::new(Mutex::new(Vec::new()));
    let native_paths = crate::parakeet_import::ParakeetWorkerPaths {
        executable: root.join("controlled-native-worker.exe"),
        native_dir: root.join("controlled-native"),
        model: root.join("controlled-model.nemo"),
        runtime_root: root.clone(),
    };
    let native_route = NativeUnifiedRoute {
        paths: native_paths,
        requested_compute: ComputeChoice::Cpu,
        compute: ComputeChoice::Cpu,
        expected_gpu: None,
        candidate: None,
        fallback_reason: None,
    };
    let mut runtime = ImportRuntime::new_with_workers_and_parakeet_registry(
        WorkerPaths {
            cpu: PathBuf::from("unused-cpu"),
            gpu: PathBuf::from("unused-gpu"),
        },
        Arc::new(PublicationGate::default()),
        None,
        WorkerRuntimeConfig::default(),
    )
    .with_controlled_transcriber(worker)
    .with_child_observer(Arc::clone(&child_observations));
    prepare_and_start_controlled_native_route(&mut runtime, &request_a, native_route.clone());
    prepare_and_start_controlled_native_route(&mut runtime, &request_b, native_route);
    runtime.native_unified_candidate_enabled = true;
    let ready_gate = runtime.hold_next_child_ready();
    let ready_labels = HashMap::new();

    assert!(runtime.poll_unified_sessions().is_none());
    ReservedFinalScenario {
        root,
        runtime,
        ready_gate,
        recycled_ready_gate: None,
        sessions: (session_a, session_b),
        controls: (calls, maximum_active, call_sources, held_rx, release_tx),
        child_observations,
        ready_labels,
        persisted_sequences: HashMap::new(),
        published_sessions: HashSet::new(),
        completed_sessions: HashSet::new(),
    }
}

pub(super) fn hold_reserved_final_until_ready(scenario: &mut ReservedFinalScenario) {
    let runtime = &mut scenario.runtime;
    let (session_a, session_b) = scenario.sessions;
    let child_observations = &scenario.child_observations;
    let ready_labels = &mut scenario.ready_labels;
    let calls = &scenario.controls.0;
    let held_rx = &scenario.controls.3;
    let ready_gate = Arc::clone(&scenario.ready_gate);
    let reserved = runtime.parakeet_scheduler.in_flight().unwrap();
    assert_eq!(reserved.session, session_a);
    assert!(matches!(
        reserved.work,
        whisper_core::parakeet_scheduler::ScheduledWork::Final { .. }
    ));
    assert_eq!(calls.load(Ordering::Relaxed), 0);
    let retained_final = runtime.unified_sessions[&session_a]
        .pending_prepared
        .clone()
        .expect("the reserved final clip is prepared before READY");
    assert_eq!(retained_final.0, reserved);
    assert!(retained_final.3);
    assert!(
        !child_observations
            .lock()
            .unwrap()
            .iter()
            .any(|item| item.event == "Ready")
    );

    runtime
        .parakeet_scheduler
        .queue_preview(session_b, 5 * 16_000)
        .unwrap();
    assert!(runtime.poll_unified_sessions().is_none());
    assert_eq!(runtime.parakeet_scheduler.in_flight(), Some(reserved));
    assert!(
        runtime.unified_sessions[&session_a]
            .pending_prepared
            .is_some()
    );
    assert_eq!(calls.load(Ordering::Relaxed), 0);
    let (ready_released, ready_changed) = &*ready_gate;
    *ready_released.lock().unwrap() = true;
    ready_changed.notify_all();
    let ready_deadline = Instant::now() + Duration::from_secs(5);
    let mut ready_seen = false;
    let mut a_ready_events = 0;
    let mut held_source = None;
    while Instant::now() < ready_deadline && held_source.is_none() {
        match runtime.poll_unified_sessions() {
            Some(Err((job_id, generation, message))) => panic!(
                "reserved turn failed after READY release for {job_id:?}/{generation:?}: {message}"
            ),
            Some(Ok(ImportEvent::Ready {
                job_id,
                generation,
                backend,
            })) if (job_id, generation) == (session_a.job_id, session_a.generation) => {
                a_ready_events += 1;
                ready_labels
                    .entry((job_id, generation))
                    .or_default()
                    .push(backend);
                assert_eq!(
                    runtime.unified_sessions[&session_a].pending_prepared,
                    Some(retained_final.clone()),
                    "READY must retain the exact prepared final clip"
                );
            }
            _ => {}
        }
        ready_seen |= child_observations
            .lock()
            .unwrap()
            .iter()
            .any(|item| item.event == "Ready");
        match held_rx.try_recv() {
            Ok(source) => held_source = Some(source),
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                thread::sleep(Duration::from_millis(2));
            }
            Err(error) => panic!("held worker exited after READY release: {error}"),
        }
    }
    assert!(
        ready_seen,
        "controlled child READY was not observed after release"
    );
    assert_eq!(
        a_ready_events, 1,
        "backend READY is reported exactly once per session"
    );
    assert_eq!(held_source.as_deref(), Some("A"));
    assert_eq!(calls.load(Ordering::Relaxed), 1);
    assert!(
        runtime.unified_sessions[&session_a]
            .pending_prepared
            .is_none()
    );
    assert!(runtime.poll_unified_sessions().is_none());
    assert_eq!(runtime.parakeet_scheduler.in_flight(), Some(reserved));
    assert_eq!(calls.load(Ordering::Relaxed), 1);
}

pub(super) fn admit_peer_preview_after_final(scenario: &mut ReservedFinalScenario) {
    let runtime = &mut scenario.runtime;
    let (_, session_b) = scenario.sessions;
    let ready_labels = &mut scenario.ready_labels;
    let calls = &scenario.controls.0;
    let maximum_active = &scenario.controls.1;
    let call_sources = &scenario.controls.2;
    let release_tx = &scenario.controls.4;
    release_tx.send(()).unwrap();
    let peer_preview_deadline = Instant::now() + Duration::from_secs(10);
    let mut b_preview_ready_events = 0;
    let mut retained_preview = None;
    while Instant::now() < peer_preview_deadline && calls.load(Ordering::Relaxed) < 2 {
        match runtime.poll_unified_sessions() {
            Some(Err((job_id, generation, message))) => {
                panic!("peer preview failed after final A for {job_id:?}/{generation:?}: {message}")
            }
            Some(Ok(ImportEvent::Ready {
                job_id,
                generation,
                backend,
            })) if (job_id, generation) == (session_b.job_id, session_b.generation) => {
                b_preview_ready_events += 1;
                ready_labels
                    .entry((job_id, generation))
                    .or_default()
                    .push(backend);
                let prepared = runtime.unified_sessions[&session_b]
                    .pending_prepared
                    .clone()
                    .expect("READY must retain the prepared peer preview");
                assert!(!prepared.3, "the retained peer clip must remain a preview");
                assert!(matches!(
                    prepared.0.work,
                    whisper_core::parakeet_scheduler::ScheduledWork::Preview { .. }
                ));
                assert_eq!(runtime.parakeet_scheduler.in_flight(), Some(prepared.0));
                retained_preview = Some(prepared);
            }
            _ => {}
        }
        thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(
        b_preview_ready_events, 1,
        "peer backend READY is reported exactly once"
    );
    let retained_preview = retained_preview.expect("the prepared preview survives READY");
    assert_eq!(
        runtime.parakeet_scheduler.in_flight(),
        Some(retained_preview.0)
    );
    assert!(
        runtime.unified_sessions[&session_b]
            .pending_prepared
            .is_none()
    );
    let call_log = call_sources.lock().unwrap();
    assert!(call_log.len() >= 2);
    assert_eq!(call_log[0], "A:final");
    assert_eq!(call_log[1], "B:preview");
    assert_eq!(maximum_active.load(Ordering::Relaxed), 1);
    drop(call_log);
}

pub(super) fn recycle_child_with_final_still_reserved(scenario: &mut ReservedFinalScenario) {
    let runtime = &mut scenario.runtime;
    let (session_a, _) = scenario.sessions;
    let child_observations = &scenario.child_observations;
    let ready_labels = &mut scenario.ready_labels;
    let calls = &scenario.controls.0;
    let persisted_sequences = &mut scenario.persisted_sequences;
    let published_sessions = &mut scenario.published_sessions;
    let completed_sessions = &mut scenario.completed_sessions;
    let recycled_ready_gate = Arc::new((Mutex::new(false), std::sync::Condvar::new()));
    runtime.controlled_ready_gate = Some(Arc::clone(&recycled_ready_gate));
    scenario.recycled_ready_gate = Some(recycled_ready_gate);
    let ready_deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < ready_deadline {
        match runtime.poll_unified_sessions() {
            Some(Err((job_id, generation, message))) => {
                panic!("reserved turn session {job_id:?}/{generation:?} failed: {message}");
            }
            Some(Ok(ImportEvent::Ready {
                job_id,
                generation,
                backend,
            })) => {
                ready_labels
                    .entry((job_id, generation))
                    .or_default()
                    .push(backend);
            }
            _ => {}
        }
        let observations = child_observations.lock().unwrap();
        let recycled_spawned = observations
            .iter()
            .filter(|item| item.event == "Spawn")
            .count()
            >= 2;
        drop(observations);
        if recycled_spawned
            && runtime
                .unified_sessions
                .get(&session_a)
                .is_some_and(|state| state.pending_prepared.is_some())
        {
            break;
        }
        if let Some(event) = runtime.poll_unified_document() {
            match event {
                Ok(event) => {
                    if let ImportEvent::End {
                        job_id, generation, ..
                    } = &event
                    {
                        completed_sessions.insert((*job_id, *generation));
                    }
                    persist_and_publish_controlled_document_event(
                        runtime,
                        event,
                        persisted_sequences,
                        published_sessions,
                        ready_labels,
                    );
                }
                Err((job_id, generation, message)) => {
                    panic!("document drain failed for {job_id:?}/{generation:?}: {message}")
                }
            }
        }
        thread::sleep(Duration::from_millis(2));
    }
    let reserved_after_recycle = runtime.parakeet_scheduler.in_flight().unwrap();
    assert_eq!(reserved_after_recycle.session, session_a);
    assert!(matches!(
        reserved_after_recycle.work,
        whisper_core::parakeet_scheduler::ScheduledWork::Final { .. }
    ));
    assert!(
        runtime.unified_sessions[&session_a]
            .pending_prepared
            .is_some()
    );
    assert_eq!(calls.load(Ordering::Relaxed), 3);
    {
        let observations = child_observations.lock().unwrap();
        let second_incarnation = observations
            .iter()
            .filter(|item| item.event == "Spawn")
            .nth(1)
            .expect("recycled child spawned")
            .incarnation;
        assert!(
            !observations
                .iter()
                .any(|item| { item.event == "Ready" && item.incarnation == second_incarnation })
        );
    }
}

pub(super) fn release_recycled_child_and_drain_documents(scenario: &mut ReservedFinalScenario) {
    let runtime = &mut scenario.runtime;
    let (session_a, session_b) = scenario.sessions;
    let ready_labels = &mut scenario.ready_labels;
    let persisted_sequences = &mut scenario.persisted_sequences;
    let published_sessions = &mut scenario.published_sessions;
    let completed_sessions = &mut scenario.completed_sessions;
    let recycled_ready_gate = scenario.recycled_ready_gate.as_ref().unwrap();
    {
        let (released, changed) = &**recycled_ready_gate;
        *released.lock().unwrap() = true;
        changed.notify_all();
    }

    let drain_deadline = Instant::now() + Duration::from_secs(20);
    while Instant::now() < drain_deadline
        && (!runtime.unified_sessions.is_empty() || !runtime.document_drains.is_empty())
    {
        match runtime.poll_unified_sessions() {
            Some(Err((job_id, generation, message))) => {
                panic!("reserved turn session {job_id:?}/{generation:?} failed: {message}");
            }
            Some(Ok(ImportEvent::Ready {
                job_id,
                generation,
                backend,
            })) => {
                ready_labels
                    .entry((job_id, generation))
                    .or_default()
                    .push(backend);
            }
            _ => {}
        }
        if let Some(event) = runtime.poll_unified_document() {
            match event {
                Ok(event) => {
                    if let ImportEvent::End {
                        job_id, generation, ..
                    } = &event
                    {
                        completed_sessions.insert((*job_id, *generation));
                    }
                    persist_and_publish_controlled_document_event(
                        runtime,
                        event,
                        persisted_sequences,
                        published_sessions,
                        ready_labels,
                    );
                }
                Err((job_id, generation, message)) => {
                    panic!("document drain failed for {job_id:?}/{generation:?}: {message}")
                }
            }
        }
        thread::sleep(Duration::from_millis(2));
    }
    assert!(runtime.unified_sessions.is_empty());
    assert!(runtime.document_drains.is_empty());
    assert_eq!(runtime.parakeet_scheduler.active_sessions(), 0);
    assert!(completed_sessions.contains(&(session_a.job_id, session_a.generation)));
    assert!(completed_sessions.contains(&(session_b.job_id, session_b.generation)));
    assert!(published_sessions.contains(&(session_a.job_id, session_a.generation)));
    assert!(published_sessions.contains(&(session_b.job_id, session_b.generation)));
    assert_eq!(ready_labels.len(), 2, "no other session may receive READY");
    for session in [session_a, session_b] {
        assert_eq!(
            ready_labels.get(&(session.job_id, session.generation)),
            Some(&vec!["CPU".to_owned()]),
            "each synthetic native CPU route reports one truthful READY through publication"
        );
    }
}

pub(super) fn verify_reserved_final_publication_and_reaping(scenario: &mut ReservedFinalScenario) {
    let (session_a, session_b) = scenario.sessions;
    let root = &scenario.root;
    let archive_root = root.join("archive");
    let child_observations = &scenario.child_observations;
    let call_sources = &scenario.controls.2;
    let archives = ArchiveStore::scan(&archive_root).unwrap();
    for (session, expected_samples) in [(session_a, 121 * 16_000_u64), (session_b, 5 * 16_000_u64)]
    {
        let archive = archives
            .iter()
            .find(|entry| entry.job_id == session.job_id && entry.generation == session.generation)
            .expect("completed controlled session must remain in archive history");
        assert!(
            archive.complete,
            "document completion must publish a complete archive"
        );
        let files = ArchiveStore::published_unified_files(
            &archive_root,
            session.job_id,
            session.generation,
        )
        .expect("Published must resolve the generation-bound CURRENT artifacts");
        for path in [&files.wav_path, &files.txt_path, &files.srt_path] {
            assert!(
                Path::new(path).is_file(),
                "published artifact missing: {path}"
            );
        }
        let source_path = archive_root
            .join("transcriptions")
            .join(format!("{:032x}", session.job_id.0))
            .join(session.generation.get().to_string())
            .join("parakeet-unified/source.wav");
        assert_eq!(
            crate::parakeet_audio::PcmPartStore::open(&source_path)
                .total_samples()
                .unwrap(),
            expected_samples,
            "completed coverage must include the whole source"
        );
    }
    let observations = child_observations.lock().unwrap();
    controlled_child_job_bound(&observations).unwrap();
    let spawns = observations
        .iter()
        .filter(|item| item.event == "Spawn")
        .map(|item| item.incarnation)
        .collect::<Vec<_>>();
    assert_eq!(
        spawns.len(),
        2,
        "terminal B must be reaped before A resumes"
    );
    for incarnation in &spawns {
        let events = observations
            .iter()
            .filter(|item| item.incarnation == *incarnation)
            .map(|item| item.event)
            .collect::<Vec<_>>();
        assert_eq!(events.first(), Some(&"Spawn"));
        assert_eq!(events.get(1), Some(&"Ready"));
        assert!(events.iter().position(|event| *event == "Reaped").is_some());
    }
    let reaped_first = observations
        .iter()
        .position(|item| item.event == "Reaped" && item.incarnation == spawns[0])
        .unwrap();
    let spawned_second = observations
        .iter()
        .position(|item| item.event == "Spawn" && item.incarnation == spawns[1])
        .unwrap();
    assert!(reaped_first < spawned_second);
    drop(observations);
    let final_order = call_sources
        .lock()
        .unwrap()
        .iter()
        .filter(|entry| entry.ends_with(":final"))
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(
        final_order,
        ["A:final", "B:final", "A:final"],
        "final turns must retain their order across child recycle"
    );
    std::fs::remove_dir_all(root).unwrap();
}
