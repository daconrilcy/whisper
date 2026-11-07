struct UnifiedTraceWorker {
    call_sent: std::sync::mpsc::SyncSender<String>,
    release: std::sync::mpsc::Receiver<()>,
    root: PathBuf,
}

impl crate::parakeet_pipeline::WindowTranscriber for UnifiedTraceWorker {
    fn instance_id(&self) -> u64 {
        9001
    }

    fn transcribe(
        &mut self,
        _: whisper_core::parakeet_session::AudioWindow,
        _: &[i16],
    ) -> Result<Vec<whisper_core::parakeet_ipc::Word>, String> {
        Err("scheduled identity was not supplied".into())
    }

    fn transcribe_scheduled(
        &mut self,
        call: whisper_core::parakeet_scheduler::ScheduledCall,
        scratch: &Path,
        window: whisper_core::parakeet_session::AudioWindow,
        pcm: &[i16],
    ) -> Result<Vec<whisper_core::parakeet_ipc::Word>, String> {
        let mut digest = Sha256::new();
        for sample in pcm {
            digest.update(sample.to_le_bytes());
        }
        let scratch = scratch
            .to_string_lossy()
            .replace(&*self.root.to_string_lossy(), "<ROOT>");
        self.call_sent
            .send(format!(
                "call={call:?},window={window:?},scratch={scratch},pcm_len={},pcm_sha={:x}",
                pcm.len(),
                digest.finalize()
            ))
            .map_err(|e| e.to_string())?;
        self.release.recv().map_err(|e| e.to_string())?;
        let start_ms = ((window.central_start_sample - window.native_start_sample) / 16) as u32;
        Ok(vec![whisper_core::parakeet_ipc::Word {
            text: "controlled.".into(),
            start_ms,
            end_ms: start_ms + 100,
        }])
    }
}

#[test]
fn unified_poll_resumed_prefix_emits_only_new_segments() {
    let root = test_artifact_root("unified-poll-resumed");
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("source.wav");
    let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source).unwrap();
    writer.append(&vec![1000_i16; 61 * 16_000]).unwrap();
    writer.finish().unwrap();
    let mut request = unified_import_request(&root.join("archive"), &source, 73_030);
    let checkpoint = create_resume_fixture(&root, &mut request);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let observations = Arc::new(Mutex::new(Vec::new()));
    let (call_sent, calls) = std::sync::mpsc::sync_channel(1);
    let (release, released) = std::sync::mpsc::sync_channel(1);
    let worker = UnifiedTraceWorker {
        call_sent,
        release: released,
        root: root.clone(),
    };
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: PathBuf::from("unused-cpu"),
            gpu: PathBuf::from("unused-gpu"),
        },
        Arc::new(PublicationGate::default()),
    )
    .with_controlled_transcriber(Box::new(worker));
    runtime
        .unified_resume_checkpoints
        .insert(session, checkpoint);
    runtime.start_controlled_unified_import(&request).unwrap();
    assert_eq!(
        runtime.unified_sessions[&session].document_drain_start_sequence,
        2
    );
    assert_eq!(
        runtime.unified_sessions[&session]
            .finalizer
            .processed_samples(),
        60 * 16_000
    );
    let mut executor = UnifiedInferenceExecutor::start_observed(
        runtime.controlled_transcriber.take().unwrap(),
        Arc::clone(&observations),
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !executor.is_ready().unwrap() {
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    runtime.unified_executor = Some(executor);
    let mut trace = Vec::new();
    trace.push(format!("resumed {}", unified_fallback_snapshot(&runtime)));
    assert!(runtime.poll_unified_sessions().is_none());
    trace.push(format!(
        "dispatched {}",
        unified_fallback_snapshot(&runtime)
    ));
    trace.push(format!(
        "worker command {}",
        calls.recv_timeout(Duration::from_secs(10)).unwrap()
    ));
    release.send(()).unwrap();
    let executor = runtime.unified_executor.as_mut().unwrap();
    let result = executor
        .results
        .recv_timeout(Duration::from_secs(10))
        .unwrap();
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    assert!(send.send(result).is_ok());
    executor.results = receive;
    assert!(runtime.poll_unified_sessions().is_none());
    trace.push(format!("committed {}", unified_fallback_snapshot(&runtime)));
    assert!(runtime.poll_unified_sessions().is_none());
    trace.push(format!("terminal {}", unified_fallback_snapshot(&runtime)));
    record_resumed_document_events(&mut runtime, session, &mut trace);
    trace.push(format!(
        "child {:?}",
        observations.lock().unwrap().as_slice()
    ));
    let path = test_artifact_root("unified-poll-resume-trace").with_extension("json");
    let trace_bytes = serde_json::to_vec_pretty(&trace).unwrap();
    std::fs::write(path, &trace_bytes).unwrap();
    assert_eq!(
        trace_bytes.as_slice(),
        include_bytes!("reference_resume.json"),
        "resumed import effects diverged from the frozen baseline"
    );
}

#[test]
fn unified_poll_nominal_import_records_complete_controlled_effects() {
    let (mut runtime, request) = unified_poll_import_fixture("unified-poll-nominal", 73_020);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let observations = Arc::new(Mutex::new(Vec::new()));
    let (call_sent, calls) = std::sync::mpsc::sync_channel(1);
    let (release, released) = std::sync::mpsc::sync_channel(1);
    let worker = UnifiedTraceWorker {
        call_sent,
        release: released,
        root: PathBuf::from(&request.destination),
    };
    let mut executor =
        UnifiedInferenceExecutor::start_observed(Box::new(worker), Arc::clone(&observations))
            .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !executor.is_ready().unwrap() {
        assert!(Instant::now() < deadline, "worker READY timed out");
        thread::yield_now();
    }
    runtime.controlled_transcriber = None;
    runtime.unified_executor = Some(executor);
    let mut trace = Vec::new();
    trace.push(format!("admitted {}", unified_fallback_snapshot(&runtime)));
    assert!(runtime.poll_unified_sessions().is_none());
    assert!(runtime.unified_sessions[&session].pending_call.is_some());
    trace.push(format!(
        "dispatched {}",
        unified_fallback_snapshot(&runtime)
    ));
    let call = calls
        .recv_timeout(Duration::from_secs(10))
        .expect("controlled command");
    trace.push(format!("worker command {call}"));
    release.send(()).unwrap();
    let executor = runtime.unified_executor.as_mut().unwrap();
    let result = executor
        .results
        .recv_timeout(Duration::from_secs(10))
        .expect("controlled result");
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    assert!(send.send(result).is_ok());
    executor.results = receive;
    trace.push("worker result returned".to_owned());
    assert!(runtime.poll_unified_sessions().is_none());
    trace.push(format!("committed {}", unified_fallback_snapshot(&runtime)));
    assert!(runtime.poll_unified_sessions().is_none());
    trace.push(format!("terminal {}", unified_fallback_snapshot(&runtime)));
    let drain = runtime.document_drains.front().unwrap();
    assert_eq!(
        (drain.job_id, drain.generation),
        (session.job_id, session.generation)
    );
    assert_eq!(
        (drain.revision, drain.total_samples, drain.next_sequence),
        (1, 48_000, 1)
    );
    assert!(drain.ready_pending && drain.progress_pending);
    while let Some(event) = runtime.poll_unified_document() {
        trace.push(format!(
            "event {}",
            serde_json::to_string(&event.expect("document drain")).unwrap()
        ));
    }
    trace.push(format!(
        "child {:?}",
        observations.lock().unwrap().as_slice()
    ));
    trace.push(format!("drained {}", unified_fallback_snapshot(&runtime)));
    let path = test_artifact_root("unified-poll-import-trace").with_extension("json");
    let trace_bytes = serde_json::to_vec_pretty(&trace).unwrap();
    std::fs::write(path, &trace_bytes).unwrap();
    assert_eq!(
        trace_bytes.as_slice(),
        include_bytes!("reference_import.json"),
        "nominal import effects diverged from the frozen baseline"
    );
}

#[test]
fn unified_poll_disconnected_result_reaps_failed_job_and_finishes_peer() {
    let (mut runtime, failed) = unified_poll_import_fixture("unified-poll-eof", 73_021);
    let root = PathBuf::from(&failed.destination)
        .parent()
        .unwrap()
        .to_path_buf();
    let peer = unified_import_request(
        &root.join("peer-archive"),
        Path::new(&failed.source_path),
        73_022,
    );
    runtime.start_controlled_unified_import(&peer).unwrap();
    let failed_session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: failed.job_id,
        generation: failed.generation,
    };
    let peer_session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: peer.job_id,
        generation: peer.generation,
    };
    let observations = Arc::new(Mutex::new(Vec::new()));
    let (call_sent, calls) = std::sync::mpsc::sync_channel(1);
    let (release, released) = std::sync::mpsc::sync_channel(1);
    let worker = UnifiedTraceWorker {
        call_sent,
        release: released,
        root,
    };
    let executor = start_disconnecting_worker(worker, &observations, failed_session);
    runtime.controlled_transcriber = None;
    runtime.unified_executor = Some(executor);
    let mut trace = Vec::new();
    trace.push(format!("admitted {}", unified_fallback_snapshot(&runtime)));
    assert!(runtime.poll_unified_sessions().is_none());
    assert_eq!(
        runtime.parakeet_scheduler.in_flight().unwrap().session,
        failed_session
    );
    trace.push(format!(
        "dispatched {}",
        unified_fallback_snapshot(&runtime)
    ));
    trace.push(format!(
        "worker command {}",
        calls.recv_timeout(Duration::from_secs(10)).unwrap()
    ));
    release.send(()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !runtime
        .unified_executor
        .as_ref()
        .unwrap()
        .thread
        .as_ref()
        .unwrap()
        .is_finished()
    {
        assert!(
            Instant::now() < deadline,
            "disconnected worker did not exit"
        );
        thread::yield_now();
    }
    let outcome = runtime.poll().expect("disconnected result");
    trace.push(format!(
        "outcome {}",
        serde_json::to_string(&outcome).unwrap()
    ));
    assert!(matches!(outcome, Err((job, generation, ref message))
        if job == failed_session.job_id && generation == failed_session.generation
            && message.contains("result channel disconnected")));
    assert!(!runtime.unified_sessions.contains_key(&failed_session));
    assert!(!runtime.unified_archives.contains_key(&failed_session));
    assert!(runtime.unified_sessions.contains_key(&peer_session));
    assert!(runtime.unified_archives.contains_key(&peer_session));
    trace.push(format!(
        "peer preserved {}",
        unified_fallback_snapshot(&runtime)
    ));
    finish_peer_after_disconnect(&mut runtime, &calls, &release, &mut trace);
    trace.push(format!(
        "child {:?}",
        observations.lock().unwrap().as_slice()
    ));
    let path = test_artifact_root("unified-poll-eof-trace").with_extension("json");
    let trace_bytes = serde_json::to_vec_pretty(&trace).unwrap();
    std::fs::write(path, &trace_bytes).unwrap();
    assert_eq!(
        trace_bytes.as_slice(),
        include_bytes!("reference_eof.json"),
        "disconnect and peer recovery effects diverged from the frozen baseline"
    );
}

#[test]
fn unified_poll_panicked_worker_releases_both_sessions_in_order() {
    let (mut runtime, failed) = unified_poll_import_fixture("unified-poll-panic", 73_023);
    let root = PathBuf::from(&failed.destination)
        .parent()
        .unwrap()
        .to_path_buf();
    let peer = unified_import_request(
        &root.join("peer-archive"),
        Path::new(&failed.source_path),
        73_024,
    );
    runtime.start_controlled_unified_import(&peer).unwrap();
    let failed_session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: failed.job_id,
        generation: failed.generation,
    };
    let peer_session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: peer.job_id,
        generation: peer.generation,
    };
    let observations = Arc::new(Mutex::new(Vec::new()));
    let mut executor =
        UnifiedInferenceExecutor::start_observed(controlled_worker().0, Arc::clone(&observations))
            .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !executor.is_ready().unwrap() {
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    executor.panic_during_transcription_for = Some(failed_session);
    executor.stop_and_reap().unwrap();
    executor.restart().unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !executor.is_ready().unwrap() {
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    runtime.controlled_transcriber = None;
    runtime.unified_executor = Some(executor);
    let mut trace = Vec::new();
    trace.push(format!("admitted {}", unified_fallback_snapshot(&runtime)));
    assert!(runtime.poll_unified_sessions().is_none());
    assert_eq!(
        runtime.parakeet_scheduler.in_flight().unwrap().session,
        failed_session
    );
    trace.push(format!(
        "dispatched {}",
        unified_fallback_snapshot(&runtime)
    ));
    let deadline = Instant::now() + Duration::from_secs(10);
    while !runtime
        .unified_executor
        .as_ref()
        .unwrap()
        .thread
        .as_ref()
        .unwrap()
        .is_finished()
    {
        assert!(Instant::now() < deadline, "panic worker did not exit");
        thread::yield_now();
    }
    let outcome = runtime.poll().expect("worker panic result");
    trace.push(format!(
        "outcome {}",
        serde_json::to_string(&outcome).unwrap()
    ));
    assert!(matches!(outcome, Err((job, generation, ref message))
        if job == failed_session.job_id && generation == failed_session.generation
            && message.contains("InferenceReap: disconnected worker thread panicked")));
    assert!(!runtime.unified_sessions.contains_key(&failed_session));
    assert!(!runtime.unified_sessions.contains_key(&peer_session));
    assert!(!runtime.unified_archives.contains_key(&failed_session));
    assert!(!runtime.unified_archives.contains_key(&peer_session));
    trace.push(format!(
        "after panic {}",
        unified_fallback_snapshot(&runtime)
    ));
    let event = runtime.poll().expect("peer failure").expect("peer event");
    assert!(
        matches!(&event, ImportEvent::Failed { job_id, generation, message }
        if *job_id == peer_session.job_id && *generation == peer_session.generation && message.contains("peer session terminated"))
    );
    trace.push(format!("event {}", serde_json::to_string(&event).unwrap()));
    trace.push(format!(
        "after peer {}",
        unified_fallback_snapshot(&runtime)
    ));
    trace.push(format!(
        "child {:?}",
        observations.lock().unwrap().as_slice()
    ));
    let path = test_artifact_root("unified-poll-panic-trace").with_extension("json");
    let trace_bytes = serde_json::to_vec_pretty(&trace).unwrap();
    std::fs::write(path, &trace_bytes).unwrap();
    assert_eq!(
        trace_bytes.as_slice(),
        include_bytes!("reference_panic.json"),
        "panic and terminal release effects diverged from the frozen baseline"
    );
}

#[cfg(windows)]
#[test]
fn unified_poll_stop_during_prepare_records_effects() {
    let root = test_artifact_root("unified-poll-stop-prepare");
    std::fs::create_dir_all(&root).unwrap();
    let mut request = unified_live_request(&root, 73_031);
    request.model_path = verified_personal_model().into();
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let mut capture = CaptureStream::empty_test_capture(16_000, 1, "test:stop-prepare");
    capture.enqueue_test_block(vec![0.1; 16_000]);
    let (call_sent, calls) = std::sync::mpsc::sync_channel(1);
    let (release, released) = std::sync::mpsc::sync_channel(1);
    let worker = UnifiedTraceWorker {
        call_sent,
        release: released,
        root: root.clone(),
    };
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: PathBuf::from("unused-cpu"),
            gpu: PathBuf::from("unused-gpu"),
        },
        Arc::new(PublicationGate::default()),
    )
    .with_controlled_transcriber(Box::new(worker));
    let ready_gate = runtime.hold_next_child_ready();
    runtime
        .start_unified_capture(&request, capture, None, None)
        .unwrap();
    let probe_hold = runtime.unified_sessions[&session]
        .source
        .hold_probe_publication();
    let mut trace = vec![format!("started {}", unified_fallback_snapshot(&runtime))];
    assert!(runtime.poll_unified_sessions().is_none());
    assert!(runtime.unified_executor.is_some());
    trace.push(format!("preparing {}", unified_fallback_snapshot(&runtime)));
    let mut stop_events = VecDeque::new();
    runtime
        .stop_controlled_unified_capture(request.job_id, request.generation, &mut stop_events)
        .unwrap();
    drop(probe_hold);
    assert!(runtime.unified_sessions[&session].source_closed);
    trace.push(format!("stopped {}", unified_fallback_snapshot(&runtime)));
    append_live_stop_events(&mut trace, &root, stop_events);
    finish_controlled_live_trace(
        &mut runtime,
        &ready_gate,
        &calls,
        &release,
        &root,
        &mut trace,
    );
    let path = test_artifact_root("unified-poll-stop-prepare-trace").with_extension("json");
    let trace_bytes = serde_json::to_vec_pretty(&trace).unwrap();
    std::fs::write(path, &trace_bytes).unwrap();
    assert_eq!(
        trace_bytes.as_slice(),
        include_bytes!("reference_stop_prepare.json")
    );
}

#[cfg(windows)]
#[test]
fn unified_poll_stop_during_live_records_effects() {
    let root = test_artifact_root("unified-poll-stop-live");
    std::fs::create_dir_all(&root).unwrap();
    let mut request = unified_live_request(&root, 73_032);
    request.model_path = verified_personal_model().into();
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let capture = CaptureStream::empty_test_capture(16_000, 1, "test:stop-live");

    let (call_sent, calls) = std::sync::mpsc::sync_channel(1);
    let (release, released) = std::sync::mpsc::sync_channel(1);
    let worker = UnifiedTraceWorker {
        call_sent,
        release: released,
        root: root.clone(),
    };
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: PathBuf::from("unused-cpu"),
            gpu: PathBuf::from("unused-gpu"),
        },
        Arc::new(PublicationGate::default()),
    )
    .with_controlled_transcriber(Box::new(worker));
    let ready_gate = runtime.hold_next_child_ready();
    runtime
        .start_unified_capture(&request, capture, None, None)
        .unwrap();
    let probe_hold = runtime.unified_sessions[&session]
        .source
        .hold_probe_publication();
    let mut trace = vec![format!("started {}", unified_fallback_snapshot(&runtime))];
    assert!(runtime.poll_unified_sessions().is_none());
    assert!(runtime.unified_executor.is_some());
    trace.push(format!("preparing {}", unified_fallback_snapshot(&runtime)));
    let _expected = pump_live_before_stop(&mut runtime, session, &ready_gate, &mut trace);
    let mut stop_events = VecDeque::new();
    runtime
        .stop_controlled_unified_capture(request.job_id, request.generation, &mut stop_events)
        .unwrap();
    drop(probe_hold);
    assert!(runtime.unified_sessions[&session].source_closed);
    trace.push(format!("stopped {}", unified_fallback_snapshot(&runtime)));
    append_live_stop_events(&mut trace, &root, stop_events);
    finish_controlled_live_trace(
        &mut runtime,
        &ready_gate,
        &calls,
        &release,
        &root,
        &mut trace,
    );
    let path = test_artifact_root("unified-poll-stop-live-trace").with_extension("json");
    let trace_bytes = serde_json::to_vec_pretty(&trace).unwrap();
    std::fs::write(path, &trace_bytes).unwrap();
    assert_eq!(
        trace_bytes.as_slice(),
        include_bytes!("reference_stop_live.json")
    );
}

#[cfg(windows)]
#[test]
fn unified_poll_stop_before_worker_ready_records_effects() {
    let root = test_artifact_root("unified-poll-stop-before-ready");
    std::fs::create_dir_all(&root).unwrap();
    let mut request = unified_live_request(&root, 73_033);
    request.model_path = verified_personal_model().into();
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let mut capture = CaptureStream::empty_test_capture(16_000, 1, "test:stop-before-ready");
    capture.enqueue_test_block(vec![0.1; 16_000]);
    let (call_sent, calls) = std::sync::mpsc::sync_channel(1);
    let (release, released) = std::sync::mpsc::sync_channel(1);
    let worker = UnifiedTraceWorker {
        call_sent,
        release: released,
        root: root.clone(),
    };
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: PathBuf::from("unused-cpu"),
            gpu: PathBuf::from("unused-gpu"),
        },
        Arc::new(PublicationGate::default()),
    )
    .with_controlled_transcriber(Box::new(worker))
    .with_child_observer(Arc::new(Mutex::new(Vec::new())));
    let ready_gate = runtime.hold_next_child_ready();
    runtime
        .start_unified_capture(&request, capture, None, None)
        .unwrap();
    let probe_hold = runtime.unified_sessions[&session]
        .source
        .hold_probe_publication();
    let mut trace = vec![format!("started {}", unified_fallback_snapshot(&runtime))];
    assert!(runtime.poll_unified_sessions().is_none());
    assert!(runtime.unified_executor.is_some());
    assert!(
        !runtime
            .unified_executor
            .as_mut()
            .unwrap()
            .is_ready()
            .unwrap()
    );
    trace.push(format!("preparing {}", unified_fallback_snapshot(&runtime)));
    let mut stop_events = VecDeque::new();
    runtime
        .stop_controlled_unified_capture(request.job_id, request.generation, &mut stop_events)
        .unwrap();
    drop(probe_hold);
    assert!(runtime.unified_sessions[&session].source_closed);
    trace.push(format!("stopped {}", unified_fallback_snapshot(&runtime)));
    append_live_stop_events(&mut trace, &root, stop_events);
    finish_controlled_live_trace(
        &mut runtime,
        &ready_gate,
        &calls,
        &release,
        &root,
        &mut trace,
    );
    let path = test_artifact_root("unified-poll-stop-before-ready-trace").with_extension("json");
    let trace_bytes = serde_json::to_vec_pretty(&trace).unwrap();
    std::fs::write(path, &trace_bytes).unwrap();
    assert_eq!(
        trace_bytes.as_slice(),
        include_bytes!("reference_stop_before_ready.json")
    );
}

#[cfg(windows)]
#[test]
fn unified_poll_real_ipc_eof_records_effects() {
    use std::os::windows::process::CommandExt;
    const CHILD: &str = "UNIFIED_POLL_EOF_CHILD";
    if std::env::var_os(CHILD).is_some() {
        run_unified_poll_real_ipc_eof_child(false);
        return;
    }
    let local = test_artifact_root("unified-poll-real-eof").join("localappdata");
    std::fs::create_dir_all(&local).unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "worker_ipc::tests::unified_poll_real_ipc_eof_records_effects",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .env("LOCALAPPDATA", &local)
        .creation_flags(0x0800_0000)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "EOF child failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let trace = std::fs::read(local.join("eof-trace.json")).unwrap();
    let path = test_artifact_root("unified-poll-real-eof-trace").with_extension("json");
    std::fs::write(path, &trace).unwrap();
    assert_eq!(trace.as_slice(), include_bytes!("reference_real_eof.json"));
}

#[cfg(windows)]
fn run_unified_poll_real_ipc_eof_child(native_candidate: bool) {
    const MODEL: &str =
        "C:/Users/cyril/AppData/Local/Whisper/models/parakeet-tdt-0.6b-v3.q8_0.gguf";
    const SHA: &str = "e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e";
    let local = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap());
    let (paths, worker) = prepare_fake_personal_worker(&local, MODEL, SHA, "fake_eof_worker.rs");
    let source = three_second_source(&local);
    let request = unified_import_request(&local.join("archive"), &source, 73_034);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let mut route = controlled_native_route(&local);
    route.paths = paths;
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: PathBuf::from("unused-cpu"),
            gpu: PathBuf::from("unused-gpu"),
        },
        Arc::new(PublicationGate::default()),
    );
    if native_candidate {
        runtime.native_unified_candidate_enabled = true;
    } else {
        runtime = runtime.with_controlled_transcriber(Box::new(
            NativeParakeetWindowTranscriber::new(route.clone()),
        ));
    }
    runtime.native_unified_routes.insert(session, route);
    runtime.start_controlled_unified_import(&request).unwrap();
    let mut trace = vec![format!("started {}", unified_fallback_snapshot(&runtime))];
    assert!(runtime.poll_unified_sessions().is_none());
    let deadline = Instant::now() + Duration::from_secs(20);
    while !runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .is_ready()
        .unwrap()
    {
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    let ready = runtime.poll_unified_sessions().unwrap().unwrap();
    trace.push(format!("event {}", serde_json::to_string(&ready).unwrap()));
    assert!(runtime.poll_unified_sessions().is_none());
    trace.push(format!(
        "dispatched {}",
        unified_fallback_snapshot(&runtime)
    ));
    let executor = runtime.unified_executor.as_mut().unwrap();
    let result = executor
        .results
        .recv_timeout(Duration::from_secs(20))
        .unwrap();
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    assert!(send.send(result).is_ok());
    executor.results = receive;
    let error = runtime.poll_unified_sessions().unwrap().unwrap_err();
    assert!(
        error.2.contains("WorkerExited"),
        "unexpected failure: {error:?}"
    );
    trace.push(format!("failure {error:?}"));
    trace.push(format!(
        "after failure {}",
        unified_fallback_snapshot(&runtime)
    ));
    let local_path = local.to_string_lossy().replace('\\', "\\\\");
    for line in std::fs::read_to_string(worker.with_extension("wire"))
        .unwrap()
        .lines()
    {
        trace.push(format!("wire {}", line.replace(&local_path, "<ROOT>")));
    }
    assert_eq!(
        crate::parakeet_import::sha256_file(Path::new(MODEL)).unwrap(),
        SHA
    );
    std::fs::write(
        local.join("eof-trace.json"),
        serde_json::to_vec_pretty(&trace).unwrap(),
    )
    .unwrap();
}

#[cfg(windows)]
#[test]
fn unified_poll_live_preview_and_final_records_effects() {
    let root = test_artifact_root("unified-poll-live-preview-final");
    std::fs::create_dir_all(&root).unwrap();
    let mut request = unified_live_request(&root, 73_035);
    request.model_path = verified_personal_model().into();
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let capture = CaptureStream::empty_test_capture(16_000, 1, "test:preview-final");
    let (call_sent, calls) = std::sync::mpsc::sync_channel(1);
    let (release, released) = std::sync::mpsc::sync_channel(1);
    let worker = UnifiedTraceWorker {
        call_sent,
        release: released,
        root: root.clone(),
    };
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: PathBuf::from("unused-cpu"),
            gpu: PathBuf::from("unused-gpu"),
        },
        Arc::new(PublicationGate::default()),
    )
    .with_controlled_transcriber(Box::new(worker));
    runtime
        .start_unified_capture(&request, capture, None, None)
        .unwrap();
    let probe_hold = runtime.unified_sessions[&session]
        .source
        .hold_probe_publication();
    let mut trace = vec![format!("started {}", unified_fallback_snapshot(&runtime))];
    assert!(runtime.poll_unified_sessions().is_none());
    let deadline = Instant::now() + Duration::from_secs(15);
    while !runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .is_ready()
        .unwrap()
    {
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    runtime
        .unified_sessions
        .get_mut(&session)
        .unwrap()
        .source
        .enqueue_test_block(vec![0.1; 70_000]);
    while runtime.unified_sessions[&session].pending_call.is_none() {
        assert!(runtime.poll_unified_sessions().is_none());
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    assert!(matches!(
        runtime.unified_sessions[&session]
            .pending_call
            .unwrap()
            .work,
        whisper_core::parakeet_scheduler::ScheduledWork::Preview { .. }
    ));
    trace.push(format!(
        "preview dispatched {}",
        unified_fallback_snapshot(&runtime)
    ));
    trace.push(format!(
        "worker command {}",
        calls.recv_timeout(Duration::from_secs(10)).unwrap()
    ));
    release.send(()).unwrap();
    let executor = runtime.unified_executor.as_mut().unwrap();
    let result = executor
        .results
        .recv_timeout(Duration::from_secs(10))
        .unwrap();
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    assert!(send.send(result).is_ok());
    let original_results = std::mem::replace(&mut executor.results, receive);
    assert!(runtime.poll_unified_sessions().is_none());
    runtime.unified_executor.as_mut().unwrap().results = original_results;
    trace.push(format!(
        "preview committed {}",
        unified_fallback_snapshot(&runtime)
    ));
    let mut events = VecDeque::new();
    runtime
        .stop_controlled_unified_capture(request.job_id, request.generation, &mut events)
        .unwrap();
    drop(probe_hold);
    append_live_stop_events(&mut trace, &root, events);
    trace.push(format!("stopped {}", unified_fallback_snapshot(&runtime)));
    finish_live_final_trace(&mut runtime, &calls, &release, &root, &mut trace);
    let path = test_artifact_root("unified-poll-live-preview-final-trace").with_extension("json");
    let trace_bytes = serde_json::to_vec_pretty(&trace).unwrap();
    std::fs::write(path, &trace_bytes).unwrap();
    assert_eq!(
        trace_bytes.as_slice(),
        include_bytes!("reference_live_preview_final.json")
    );
}

#[test]
fn unified_poll_finalization_failure_retains_failure_until_next_poll() {
    let (mut runtime, request) = unified_poll_import_fixture("unified-poll-finish-failure", 73_036);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let (call_sent, calls) = std::sync::mpsc::sync_channel(1);
    let (release, released) = std::sync::mpsc::sync_channel(1);
    let worker = UnifiedTraceWorker {
        call_sent,
        release: released,
        root: PathBuf::from(&request.destination),
    };
    let mut executor = UnifiedInferenceExecutor::start(Box::new(worker)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !executor.is_ready().unwrap() {
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    runtime.controlled_transcriber = None;
    runtime.unified_executor = Some(executor);
    assert!(runtime.poll_unified_sessions().is_none());
    calls.recv_timeout(Duration::from_secs(10)).unwrap();
    release.send(()).unwrap();
    let executor = runtime.unified_executor.as_mut().unwrap();
    let result = executor
        .results
        .recv_timeout(Duration::from_secs(10))
        .unwrap();
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    assert!(send.send(result).is_ok());
    executor.results = receive;
    assert!(runtime.poll_unified_sessions().is_none());
    runtime.controlled_finish_failure_session = Some(session);
    assert!(runtime.poll_unified_sessions().is_none());
    let state = &runtime.unified_sessions[&session];
    assert_eq!(
        state.failure.as_deref(),
        Some("TestOnly: injected document finalization failure")
    );
    assert!(runtime.document_drains.is_empty());
    let outcome = runtime.poll_unified_sessions().unwrap().unwrap_err();
    assert_eq!((outcome.0, outcome.1), (session.job_id, session.generation));
    assert_eq!(
        outcome.2,
        "TestOnly: injected document finalization failure"
    );
    assert!(!runtime.unified_sessions.contains_key(&session));
}

#[test]
fn unified_poll_inflight_capture_failure_preempts_successful_result() {
    let (mut runtime, request) = unified_poll_import_fixture("unified-poll-failure-result", 73_037);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let (call_sent, calls) = std::sync::mpsc::sync_channel(1);
    let (release, released) = std::sync::mpsc::sync_channel(1);
    let worker = UnifiedTraceWorker {
        call_sent,
        release: released,
        root: PathBuf::from(&request.destination),
    };
    let mut executor = UnifiedInferenceExecutor::start(Box::new(worker)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !executor.is_ready().unwrap() {
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    runtime.controlled_transcriber = None;
    runtime.unified_executor = Some(executor);
    assert!(runtime.poll_unified_sessions().is_none());
    calls.recv_timeout(Duration::from_secs(10)).unwrap();
    runtime.unified_sessions.get_mut(&session).unwrap().failure =
        Some("controlled capture failure".into());
    release.send(()).unwrap();
    let executor = runtime.unified_executor.as_mut().unwrap();
    let result = executor
        .results
        .recv_timeout(Duration::from_secs(10))
        .unwrap();
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    assert!(send.send(result).is_ok());
    executor.results = receive;
    let outcome = runtime.poll_unified_sessions().unwrap().unwrap_err();
    assert_eq!((outcome.0, outcome.1), (session.job_id, session.generation));
    assert_eq!(outcome.2, "controlled capture failure");
    assert!(!runtime.unified_sessions.contains_key(&session));
    assert!(runtime.parakeet_scheduler.in_flight().is_none());
    assert!(runtime.document_drains.is_empty());
}

#[cfg(windows)]
#[test]
fn unified_poll_stop_live_with_pending_audio_records_effects() {
    let root = test_artifact_root("unified-poll-stop-pending-audio");
    std::fs::create_dir_all(&root).unwrap();
    let mut request = unified_live_request(&root, 73_038);
    request.model_path = verified_personal_model().into();
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let capture = CaptureStream::empty_test_capture(16_000, 1, "test:pending-audio");

    let (call_sent, calls) = std::sync::mpsc::sync_channel(1);
    let (release, released) = std::sync::mpsc::sync_channel(1);
    let worker = UnifiedTraceWorker {
        call_sent,
        release: released,
        root: root.clone(),
    };
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: PathBuf::from("unused-cpu"),
            gpu: PathBuf::from("unused-gpu"),
        },
        Arc::new(PublicationGate::default()),
    )
    .with_controlled_transcriber(Box::new(worker));
    let ready_gate = runtime.hold_next_child_ready();
    runtime
        .start_unified_capture(&request, capture, None, None)
        .unwrap();
    let probe_hold = runtime.unified_sessions[&session]
        .source
        .hold_probe_publication();
    let mut trace = vec![format!("started {}", unified_fallback_snapshot(&runtime))];
    assert!(runtime.poll_unified_sessions().is_none());
    assert!(runtime.unified_executor.is_some());
    trace.push(format!("preparing {}", unified_fallback_snapshot(&runtime)));
    let expected = pump_live_before_stop(&mut runtime, session, &ready_gate, &mut trace);
    runtime
        .unified_sessions
        .get_mut(&session)
        .unwrap()
        .source
        .enqueue_test_block(vec![0.2; 3_200]);
    let mut stop_events = VecDeque::new();
    runtime
        .stop_controlled_unified_capture(request.job_id, request.generation, &mut stop_events)
        .unwrap();
    drop(probe_hold);
    assert!(runtime.unified_sessions[&session].source_closed);
    assert!(runtime.unified_sessions[&session].available_samples > expected);
    trace.push(format!("stopped {}", unified_fallback_snapshot(&runtime)));
    append_live_stop_events(&mut trace, &root, stop_events);
    finish_controlled_live_trace(
        &mut runtime,
        &ready_gate,
        &calls,
        &release,
        &root,
        &mut trace,
    );
    let path = test_artifact_root("unified-poll-stop-pending-audio-trace").with_extension("json");
    let trace_bytes = serde_json::to_vec_pretty(&trace).unwrap();
    std::fs::write(path, &trace_bytes).unwrap();
    assert_eq!(
        trace_bytes.as_slice(),
        include_bytes!("reference_stop_pending_audio.json")
    );
}

#[test]
fn unified_poll_inflight_capture_failure_restarts_surviving_peer() {
    let (mut runtime, request) = unified_poll_import_fixture("unified-poll-failure-peer", 73_066);
    let root = PathBuf::from(&request.destination)
        .parent()
        .unwrap()
        .to_path_buf();
    let peer = unified_import_request(&root.join("peer"), Path::new(&request.source_path), 73_067);
    runtime.start_controlled_unified_import(&peer).unwrap();
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let peer_id = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: peer.job_id,
        generation: peer.generation,
    };
    let (call_sent, calls) = std::sync::mpsc::sync_channel(1);
    let (release, released) = std::sync::mpsc::sync_channel(1);
    let worker = UnifiedTraceWorker {
        call_sent,
        release: released,
        root,
    };
    let mut executor = UnifiedInferenceExecutor::start(Box::new(worker)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !executor.is_ready().unwrap() {
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    runtime.controlled_transcriber = None;
    runtime.unified_executor = Some(executor);
    assert!(runtime.poll_unified_sessions().is_none());
    calls.recv_timeout(Duration::from_secs(10)).unwrap();
    runtime.unified_sessions.get_mut(&session).unwrap().failure =
        Some("controlled capture failure".into());
    release.send(()).unwrap();
    let executor = runtime.unified_executor.as_mut().unwrap();
    let result = executor
        .results
        .recv_timeout(Duration::from_secs(10))
        .unwrap();
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    assert!(send.send(result).is_ok());
    executor.results = receive;
    let outcome = runtime.poll_unified_sessions().unwrap().unwrap_err();
    assert_eq!((outcome.0, outcome.1), (session.job_id, session.generation));
    assert_eq!(outcome.2, "controlled capture failure");
    assert!(!runtime.unified_sessions.contains_key(&session));
    assert!(runtime.unified_sessions.contains_key(&peer_id));
    // This path does not mark the failed scheduler session terminal.
    assert_eq!(runtime.parakeet_scheduler.active_sessions(), 2);
    assert!(runtime.unified_executor.as_ref().unwrap().thread.is_some());
    runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .stop_and_reap()
        .unwrap();
}

#[test]
fn unified_poll_finished_document_survives_missing_scheduler_session() {
    let (mut runtime, request) =
        unified_poll_import_fixture("unified-poll-finished-missing-scheduler", 73_072);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let (call_sent, calls) = std::sync::mpsc::sync_channel(1);
    let (release, released) = std::sync::mpsc::sync_channel(1);
    let worker = UnifiedTraceWorker {
        call_sent,
        release: released,
        root: PathBuf::from(&request.destination),
    };
    let mut executor = UnifiedInferenceExecutor::start(Box::new(worker)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !executor.is_ready().unwrap() {
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    runtime.controlled_transcriber = None;
    runtime.unified_executor = Some(executor);
    assert!(runtime.poll_unified_sessions().is_none());
    calls.recv_timeout(Duration::from_secs(10)).unwrap();
    release.send(()).unwrap();
    let executor = runtime.unified_executor.as_mut().unwrap();
    let result = executor
        .results
        .recv_timeout(Duration::from_secs(10))
        .unwrap();
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    assert!(send.send(result).is_ok());
    executor.results = receive;
    assert!(runtime.poll_unified_sessions().is_none());
    // Remove only scheduler membership to isolate the defensive terminal error.
    runtime.parakeet_scheduler.abort_session(session).unwrap();
    runtime.parakeet_scheduler.confirm_child_exited().unwrap();
    let outcome = runtime.poll_unified_sessions().unwrap().unwrap_err();
    assert_eq!((outcome.0, outcome.1), (session.job_id, session.generation));
    assert_eq!(outcome.2, "SchedulerTerminal: UnknownSession");
    assert!(runtime.unified_sessions.contains_key(&session));
    assert!(runtime.document_drains.is_empty());
    runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .stop_and_reap()
        .unwrap();
}

struct UnifiedPanickingInitializer;
impl crate::parakeet_pipeline::WindowTranscriber for UnifiedPanickingInitializer {
    fn instance_id(&self) -> u64 {
        9013
    }
    fn initialize_instance(&mut self, _: &std::sync::atomic::AtomicBool) -> Result<(), String> {
        panic!("controlled initialization panic")
    }
    fn transcribe(
        &mut self,
        _: whisper_core::parakeet_session::AudioWindow,
        _: &[i16],
    ) -> Result<Vec<whisper_core::parakeet_ipc::Word>, String> {
        unreachable!()
    }
}

#[test]
fn unified_poll_finished_document_survives_executor_reap_panic() {
    let (mut runtime, request) =
        unified_poll_import_fixture("unified-poll-finished-reap-panic", 73_073);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let (call_sent, calls) = std::sync::mpsc::sync_channel(1);
    let (release, released) = std::sync::mpsc::sync_channel(1);
    let worker = UnifiedTraceWorker {
        call_sent,
        release: released,
        root: PathBuf::from(&request.destination),
    };
    let mut executor = UnifiedInferenceExecutor::start(Box::new(worker)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !executor.is_ready().unwrap() {
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    runtime.controlled_transcriber = None;
    runtime.unified_executor = Some(executor);
    assert!(runtime.poll_unified_sessions().is_none());
    calls.recv_timeout(Duration::from_secs(10)).unwrap();
    release.send(()).unwrap();
    let executor = runtime.unified_executor.as_mut().unwrap();
    let result = executor
        .results
        .recv_timeout(Duration::from_secs(10))
        .unwrap();
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    assert!(send.send(result).is_ok());
    executor.results = receive;
    assert!(runtime.poll_unified_sessions().is_none());
    runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .stop_and_reap()
        .unwrap();
    let failed = UnifiedInferenceExecutor::start(Box::new(UnifiedPanickingInitializer)).unwrap();
    while !failed.thread.as_ref().unwrap().is_finished() {
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    runtime.unified_executor = Some(failed);
    let outcome = runtime.poll_unified_sessions().unwrap().unwrap_err();
    assert_eq!((outcome.0, outcome.1), (session.job_id, session.generation));
    assert_eq!(
        outcome.2,
        "InferenceReap: InferenceReap: worker thread panicked"
    );
    assert!(runtime.unified_sessions.contains_key(&session));
    assert!(runtime.document_drains.is_empty());
    assert!(runtime.unified_executor.as_ref().unwrap().thread.is_none());
}

#[test]
fn unified_poll_result_failures_reinsert_state_when_reap_panics() {
    for (id, kind) in [
        (73_074, "capture"),
        (73_075, "inference"),
        (73_076, "commit"),
    ] {
        let (mut runtime, request) =
            unified_poll_import_fixture("unified-poll-result-reap-panic", id);
        let session = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: request.job_id,
            generation: request.generation,
        };
        let worker: Box<dyn crate::parakeet_pipeline::WindowTranscriber> = match kind {
            "capture" => controlled_worker().0,
            "inference" => Box::new(UnifiedFailureWorker),
            _ => Box::new(UnifiedInvalidWordWorker),
        };
        let mut executor = UnifiedInferenceExecutor::start(worker).unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while !executor.is_ready().unwrap() {
            assert!(Instant::now() < deadline);
            thread::yield_now();
        }
        runtime.controlled_transcriber = None;
        runtime.unified_executor = Some(executor);
        assert!(runtime.poll_unified_sessions().is_none());
        let executor = runtime.unified_executor.as_mut().unwrap();
        let result = executor
            .results
            .recv_timeout(Duration::from_secs(10))
            .unwrap();
        executor.in_flight = false;
        executor.stop_and_reap().unwrap();
        if kind == "capture" {
            runtime.unified_sessions.get_mut(&session).unwrap().failure =
                Some("controlled capture failure".into());
        }
        let mut failed =
            UnifiedInferenceExecutor::start(Box::new(UnifiedPanickingInitializer)).unwrap();
        while !failed.thread.as_ref().unwrap().is_finished() {
            assert!(Instant::now() < deadline);
            thread::yield_now();
        }
        let (send, receive) = std::sync::mpsc::sync_channel(1);
        assert!(send.send(result).is_ok());
        failed.results = receive;
        failed.in_flight = true;
        runtime.unified_executor = Some(failed);
        let outcome = runtime.poll_unified_sessions().unwrap().unwrap_err();
        assert_eq!((outcome.0, outcome.1), (session.job_id, session.generation));
        let original = match kind {
            "capture" => "controlled capture failure",
            "inference" => "controlled inference failure",
            _ => "InvalidTimestamp: word 0 is invalid or unordered",
        };
        assert_eq!(
            outcome.2,
            format!("{original}; InferenceReap: InferenceReap: worker thread panicked")
        );
        let state = &runtime.unified_sessions[&session];
        assert!(state.pending_call.is_none());
        if kind == "capture" {
            assert_eq!(state.failure.as_deref(), Some(original));
        }
        assert!(runtime.document_drains.is_empty());
        assert!(runtime.unified_executor.as_ref().unwrap().thread.is_none());
    }
}

#[cfg(windows)]
#[test]
fn unified_poll_real_eof_restarts_and_completes_peer() {
    use std::os::windows::process::CommandExt;
    const CHILD: &str = "UNIFIED_POLL_RECOVERY_CHILD";
    if std::env::var_os(CHILD).is_some() {
        run_unified_poll_real_eof_peer_child();
        return;
    }
    let local = test_artifact_root("unified-poll-real-eof-peer").join("localappdata");
    std::fs::create_dir_all(&local).unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "worker_ipc::tests::unified_poll_real_eof_restarts_and_completes_peer",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .env("LOCALAPPDATA", &local)
        .creation_flags(0x0800_0000)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "recovery child failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let trace = std::fs::read(local.join("recovery-trace.json")).unwrap();
    let path = test_artifact_root("unified-poll-real-eof-peer-trace").with_extension("json");
    std::fs::write(path, &trace).unwrap();
    assert_eq!(
        trace.as_slice(),
        include_bytes!("reference_real_eof_peer.json")
    );
    assert!(local.join("recovery-id-map.json").is_file());
}

#[cfg(windows)]
fn run_unified_poll_real_eof_peer_child() {
    const MODEL: &str =
        "C:/Users/cyril/AppData/Local/Whisper/models/parakeet-tdt-0.6b-v3.q8_0.gguf";
    const SHA: &str = "e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e";
    let local = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap());
    let (paths, worker) =
        prepare_fake_personal_worker(&local, MODEL, SHA, "fake_recovery_worker.rs");
    let source = three_second_source(&local);
    let first = unified_import_request(&local.join("first"), &source, 73_077);
    let peer = unified_import_request(&local.join("peer"), &source, 73_078);
    let first_id = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: first.job_id,
        generation: first.generation,
    };
    let peer_id = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: peer.job_id,
        generation: peer.generation,
    };
    let mut route = controlled_native_route(&local);
    route.paths = paths;
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: PathBuf::from("unused-cpu"),
            gpu: PathBuf::from("unused-gpu"),
        },
        Arc::new(PublicationGate::default()),
    )
    .with_controlled_transcriber(Box::new(NativeParakeetWindowTranscriber::new(
        route.clone(),
    )));
    runtime
        .native_unified_routes
        .insert(first_id, route.clone());
    runtime.start_controlled_unified_import(&first).unwrap();
    runtime.native_unified_routes.insert(peer_id, route);
    runtime.start_controlled_unified_import(&peer).unwrap();
    let mut trace = vec![format!("started {}", unified_fallback_snapshot(&runtime))];
    let mut wire_cursor = 0;
    assert!(runtime.poll_unified_sessions().is_none());
    let deadline = Instant::now() + Duration::from_secs(30);
    while !runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .is_ready()
        .unwrap()
    {
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    append_recovery_wire(&worker, &local, &mut wire_cursor, &mut trace);
    let ready = runtime.poll_unified_sessions().unwrap().unwrap();
    trace.push(format!("event {}", serde_json::to_string(&ready).unwrap()));
    assert!(runtime.poll_unified_sessions().is_none());
    trace.push(format!(
        "first dispatched {}",
        unified_fallback_snapshot(&runtime)
    ));
    let executor = runtime.unified_executor.as_mut().unwrap();
    let result = executor
        .results
        .recv_timeout(Duration::from_secs(20))
        .unwrap();
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    assert!(send.send(result).is_ok());
    executor.results = receive;
    append_recovery_wire(&worker, &local, &mut wire_cursor, &mut trace);
    let failed = runtime.poll().unwrap().unwrap_err();
    assert_eq!((failed.0, failed.1), (first_id.job_id, first_id.generation));
    assert!(failed.2.contains("WorkerExited"));
    trace.push(format!("failure {failed:?}"));
    assert!(!runtime.unified_sessions.contains_key(&first_id));
    assert!(!runtime.unified_archives.contains_key(&first_id));
    assert!(runtime.unified_sessions.contains_key(&peer_id));
    trace.push(format!(
        "peer retained {}",
        unified_fallback_snapshot(&runtime)
    ));
    let pid = finish_real_eof_peer(
        &mut runtime,
        &worker,
        &local,
        peer_id,
        deadline,
        &mut wire_cursor,
        &mut trace,
    );
    assert_eq!(
        crate::parakeet_import::sha256_file(Path::new(MODEL)).unwrap(),
        SHA
    );
    std::fs::write(
        local.join("recovery-id-map.json"),
        serde_json::to_vec(&pid).unwrap(),
    )
    .unwrap();
    std::fs::write(
        local.join("recovery-trace.json"),
        serde_json::to_vec_pretty(&trace).unwrap(),
    )
    .unwrap();
}

#[cfg(windows)]
fn append_recovery_wire(worker: &Path, local: &Path, cursor: &mut usize, trace: &mut Vec<String>) {
    let wire = std::fs::read_to_string(worker.with_extension("wire")).unwrap();
    let lines: Vec<_> = wire.lines().collect();
    assert!(lines.len() >= *cursor);
    let local_path = local.to_string_lossy().replace('\\', "\\\\");
    for line in &lines[*cursor..] {
        trace.push(format!("wire {}", line.replace(&local_path, "<ROOT>")));
    }
    *cursor = lines.len();
}

#[test]
fn unified_poll_result_failures_reinsert_state_when_scheduler_reap_rejects() {
    for (id, kind) in [
        (73_079, "capture"),
        (73_080, "inference"),
        (73_081, "commit"),
    ] {
        let (mut runtime, request) =
            unified_poll_import_fixture("unified-poll-result-scheduler-reap", id);
        let session = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: request.job_id,
            generation: request.generation,
        };
        runtime.parakeet_scheduler.queue_final(session, 1).unwrap();
        let call = runtime.parakeet_scheduler.next_call().unwrap().unwrap();
        // An unstarted reservation deliberately reaches result handling.
        let state = runtime.unified_sessions.get_mut(&session).unwrap();
        state.pending_call = Some(call);
        if kind == "capture" {
            state.failure = Some("controlled capture failure".into());
        }
        let worker: Box<dyn crate::parakeet_pipeline::WindowTranscriber> = match kind {
            "capture" => controlled_worker().0,
            "inference" => Box::new(UnifiedFailureWorker),
            _ => Box::new(UnifiedInvalidWordWorker),
        };
        let mut executor = UnifiedInferenceExecutor::start(worker).unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while !executor.is_ready().unwrap() {
            assert!(Instant::now() < deadline);
            thread::yield_now();
        }
        let window = whisper_core::parakeet_session::AudioWindow {
            sequence: 1,
            central_start_sample: 0,
            central_end_sample: 1_600,
            native_start_sample: 0,
            native_end_sample: 1_600,
        };
        executor
            .dispatch(UnifiedInferenceCommand {
                call,
                window,
                pcm: vec![0; 1_600],
                scratch_directory: PathBuf::from(&request.destination),
            })
            .unwrap();
        let result = executor
            .results
            .recv_timeout(Duration::from_secs(10))
            .unwrap();
        let (send, receive) = std::sync::mpsc::sync_channel(1);
        assert!(send.send(result).is_ok());
        executor.results = receive;
        runtime.controlled_transcriber = None;
        runtime.unified_executor = Some(executor);
        let outcome = runtime.poll_unified_sessions().unwrap().unwrap_err();
        assert_eq!((outcome.0, outcome.1), (session.job_id, session.generation));
        let original = match kind {
            "capture" => "controlled capture failure",
            "inference" => "controlled inference failure",
            _ => "Pipeline: no prepared final window",
        };
        assert_eq!(
            outcome.2,
            format!("{original}; SchedulerReap: NoRecyclePending")
        );
        let state = &runtime.unified_sessions[&session];
        assert!(state.pending_call.is_none());
        if kind == "capture" {
            assert_eq!(state.failure.as_deref(), Some(original));
        }
        assert!(runtime.parakeet_scheduler.in_flight().is_none());
        assert!(runtime.unified_executor.as_ref().unwrap().thread.is_none());
    }
}

#[cfg(windows)]
#[test]
fn unified_poll_native_candidate_starts_validated_cpu_route() {
    use std::os::windows::process::CommandExt;
    const CHILD: &str = "UNIFIED_POLL_NATIVE_CANDIDATE_CHILD";
    if std::env::var_os(CHILD).is_some() {
        run_unified_poll_real_ipc_eof_child(true);
        return;
    }
    let local = test_artifact_root("unified-poll-native-candidate").join("localappdata");
    std::fs::create_dir_all(&local).unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "worker_ipc::tests::unified_poll_native_candidate_starts_validated_cpu_route",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .env("LOCALAPPDATA", &local)
        .creation_flags(0x0800_0000)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "native-candidate fixture failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(local.join("eof-trace.json").is_file());
}

#[test]
fn unified_poll_finished_document_survives_scheduler_reap_error() {
    let (mut runtime, request) =
        unified_poll_import_fixture("unified-poll-finished-scheduler-reap", 73_099);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let (call_sent, calls) = std::sync::mpsc::sync_channel(1);
    let (release, released) = std::sync::mpsc::sync_channel(1);
    let worker = UnifiedTraceWorker {
        call_sent,
        release: released,
        root: PathBuf::from(&request.destination),
    };
    let mut executor = UnifiedInferenceExecutor::start(Box::new(worker)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !executor.is_ready().unwrap() {
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    runtime.controlled_transcriber = None;
    runtime.unified_executor = Some(executor);
    assert!(runtime.poll_unified_sessions().is_none());
    calls.recv_timeout(Duration::from_secs(10)).unwrap();
    release.send(()).unwrap();
    let executor = runtime.unified_executor.as_mut().unwrap();
    let result = executor
        .results
        .recv_timeout(Duration::from_secs(10))
        .unwrap();
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    assert!(send.send(result).is_ok());
    executor.results = receive;
    assert!(runtime.poll_unified_sessions().is_none());
    // Keep an artificial final queued so terminal marking does not request recycle.
    runtime.parakeet_scheduler.queue_final(session, 2).unwrap();
    let outcome = runtime.poll_unified_sessions().unwrap().unwrap_err();
    assert_eq!((outcome.0, outcome.1), (session.job_id, session.generation));
    assert_eq!(outcome.2, "SchedulerReap: NoRecyclePending");
    assert!(runtime.unified_sessions.contains_key(&session));
    assert!(runtime.document_drains.is_empty());
    assert!(runtime.unified_executor.as_ref().unwrap().thread.is_none());
}

#[cfg(windows)]
fn finish_controlled_live_trace(
    runtime: &mut ImportRuntime,
    ready_gate: &Arc<(Mutex<bool>, std::sync::Condvar)>,
    calls: &std::sync::mpsc::Receiver<String>,
    release: &std::sync::mpsc::SyncSender<()>,
    root: &Path,
    trace: &mut Vec<String>,
) {
    let (unblocked, changed) = &**ready_gate;
    *unblocked.lock().unwrap() = true;
    changed.notify_all();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .is_ready()
        .unwrap()
    {
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    assert!(runtime.poll_unified_sessions().is_none());
    trace.push(format!("dispatched {}", unified_fallback_snapshot(runtime)));
    trace.push(format!(
        "worker command {}",
        calls.recv_timeout(Duration::from_secs(10)).unwrap()
    ));
    release.send(()).unwrap();
    let executor = runtime.unified_executor.as_mut().unwrap();
    let result = executor
        .results
        .recv_timeout(Duration::from_secs(10))
        .unwrap();
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    assert!(send.send(result).is_ok());
    executor.results = receive;
    assert!(runtime.poll_unified_sessions().is_none());
    trace.push(format!("committed {}", unified_fallback_snapshot(runtime)));
    assert!(runtime.poll_unified_sessions().is_none());
    trace.push(format!("terminal {}", unified_fallback_snapshot(runtime)));
    while let Some(event) = runtime.poll_unified_document() {
        trace.push(format!(
            "event {}",
            serde_json::to_string(&event.unwrap())
                .unwrap()
                .replace(&*root.to_string_lossy().replace('\\', "\\\\"), "<ROOT>")
        ));
    }
}

#[cfg(windows)]
fn verified_personal_model() -> &'static str {
    const MODEL: &str =
        "C:/Users/cyril/AppData/Local/Whisper/models/parakeet-tdt-0.6b-v3.q8_0.gguf";
    const SHA: &str = "e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e";
    assert_eq!(
        crate::parakeet_import::sha256_file(Path::new(MODEL)).unwrap(),
        SHA
    );
    MODEL
}

#[cfg(windows)]
fn append_live_stop_events(
    trace: &mut Vec<String>,
    root: &Path,
    stop_events: VecDeque<ImportEvent>,
) {
    for event in stop_events {
        trace.push(format!(
            "stop event {}",
            serde_json::to_string(&event)
                .unwrap()
                .replace(&*root.to_string_lossy().replace('\\', "\\\\"), "<ROOT>")
        ));
    }
}

fn create_resume_fixture(
    root: &Path,
    request: &mut ImportRequest,
) -> crate::archive::UnifiedArchiveResumeCheckpoint {
    use whisper_core::ports::{ParakeetDocumentPort, ParakeetWindowCoverage};
    let previous_generation = request.generation;
    request.generation = previous_generation.next().unwrap();
    let prior_dir = root.join("previous-document");
    let mut prior = crate::parakeet_document::ParakeetDocumentStore::create(
        &prior_dir,
        request.job_id,
        previous_generation,
    )
    .unwrap();
    let anchor = whisper_core::SourceRange::new(0, 1600, 16_000).unwrap();
    prior
        .append_final(WorkerSegment {
            job_id: request.job_id,
            generation: previous_generation,
            instance_id: 9001,
            segment_id: whisper_core::SegmentId(1),
            range: anchor,
            text: "prior.".into(),
        })
        .unwrap();
    prior
        .record_coverage(ParakeetWindowCoverage {
            sequence: 1,
            range: whisper_core::SourceRange::new(0, 60 * 16_000, 16_000).unwrap(),
            last_segment_sequence: 1,
        })
        .unwrap();
    drop(prior);
    crate::archive::UnifiedArchiveResumeCheckpoint {
        pipeline: crate::parakeet_pipeline::ParakeetResumeCheckpoint {
            window_sequence: 1,
            covered_samples: 60 * 16_000,
            confirmed_segments: 1,
            join_anchor: Some(anchor),
        },
        previous_document_directory: prior_dir,
        previous_job_id: request.job_id,
        previous_generation,
    }
}

fn record_resumed_document_events(
    runtime: &mut ImportRuntime,
    session: whisper_core::parakeet_scheduler::SchedulerSession,
    trace: &mut Vec<String>,
) {
    let mut emitted_segments = Vec::new();
    let drain = runtime.document_drains.front().unwrap();
    assert_eq!(
        (drain.job_id, drain.generation),
        (session.job_id, session.generation)
    );
    assert_eq!(drain.next_sequence, 2);
    assert!(drain.ready_pending && drain.progress_pending);
    while let Some(event) = runtime.poll_unified_document() {
        let event = event.expect("resumed drain");
        if let ImportEvent::Segment(segment) = &event {
            emitted_segments.push(segment.segment_id.0);
        }
        trace.push(format!("event {}", serde_json::to_string(&event).unwrap()));
    }
    assert_eq!(emitted_segments, vec![2]);
}

fn finish_peer_after_disconnect(
    runtime: &mut ImportRuntime,
    calls: &std::sync::mpsc::Receiver<String>,
    release: &std::sync::mpsc::SyncSender<()>,
    trace: &mut Vec<String>,
) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .is_ready()
        .unwrap()
    {
        assert!(Instant::now() < deadline, "peer executor READY timed out");
        thread::yield_now();
    }
    assert!(runtime.poll_unified_sessions().is_none());
    trace.push(format!(
        "peer dispatched {}",
        unified_fallback_snapshot(runtime)
    ));
    trace.push(format!(
        "worker command {}",
        calls.recv_timeout(Duration::from_secs(10)).unwrap()
    ));
    release.send(()).unwrap();
    let executor = runtime.unified_executor.as_mut().unwrap();
    let result = executor
        .results
        .recv_timeout(Duration::from_secs(10))
        .unwrap();
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    assert!(send.send(result).is_ok());
    executor.results = receive;
    assert!(runtime.poll_unified_sessions().is_none());
    trace.push(format!(
        "peer committed {}",
        unified_fallback_snapshot(runtime)
    ));
    assert!(runtime.poll_unified_sessions().is_none());
    trace.push(format!(
        "peer terminal {}",
        unified_fallback_snapshot(runtime)
    ));
    while let Some(event) = runtime.poll_unified_document() {
        trace.push(format!(
            "event {}",
            serde_json::to_string(&event.expect("peer drain")).unwrap()
        ));
    }
}

fn start_disconnecting_worker(
    worker: UnifiedTraceWorker,
    observations: &Arc<Mutex<Vec<ControlledChildObservation>>>,
    failed_session: whisper_core::parakeet_scheduler::SchedulerSession,
) -> UnifiedInferenceExecutor {
    let mut executor =
        UnifiedInferenceExecutor::start_observed(Box::new(worker), Arc::clone(observations))
            .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !executor.is_ready().unwrap() {
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    executor.disconnect_result_for = Some(failed_session);
    executor.stop_and_reap().unwrap();
    executor.restart().unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !executor.is_ready().unwrap() {
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    executor
}

#[cfg(windows)]
fn pump_live_before_stop(
    runtime: &mut ImportRuntime,
    session: whisper_core::parakeet_scheduler::SchedulerSession,
    ready_gate: &Arc<(Mutex<bool>, std::sync::Condvar)>,
    trace: &mut Vec<String>,
) -> u64 {
    let (unblocked, changed) = &**ready_gate;
    *unblocked.lock().unwrap() = true;
    changed.notify_all();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .is_ready()
        .unwrap()
    {
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    let mut reference = CaptureStream::from_test_blocks(16_000, 1, vec![vec![0.1; 16_000]]);
    let mut converter = crate::capture::Pcm16Converter::new(16_000, 1).unwrap();
    let expected = converter
        .push(&reference.try_next_block().unwrap())
        .unwrap()
        .len() as u64;
    runtime
        .unified_sessions
        .get_mut(&session)
        .unwrap()
        .source
        .enqueue_test_block(vec![0.1; 16_000]);
    while runtime.unified_sessions[&session].available_samples < expected {
        assert!(runtime.poll_unified_sessions().is_none());
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    assert_eq!(
        runtime.unified_sessions[&session].available_samples,
        expected
    );
    trace.push(format!("live {}", unified_fallback_snapshot(runtime)));
    expected
}

#[cfg(windows)]
fn finish_live_final_trace(
    runtime: &mut ImportRuntime,
    calls: &std::sync::mpsc::Receiver<String>,
    release: &std::sync::mpsc::SyncSender<()>,
    root: &Path,
    trace: &mut Vec<String>,
) {
    assert!(runtime.poll_unified_sessions().is_none());
    trace.push(format!(
        "final dispatched {}",
        unified_fallback_snapshot(runtime)
    ));
    trace.push(format!(
        "worker command {}",
        calls.recv_timeout(Duration::from_secs(10)).unwrap()
    ));
    release.send(()).unwrap();
    let executor = runtime.unified_executor.as_mut().unwrap();
    let result = executor
        .results
        .recv_timeout(Duration::from_secs(10))
        .unwrap();
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    assert!(send.send(result).is_ok());
    executor.results = receive;
    assert!(runtime.poll_unified_sessions().is_none());
    trace.push(format!(
        "final committed {}",
        unified_fallback_snapshot(runtime)
    ));
    assert!(runtime.poll_unified_sessions().is_none());
    trace.push(format!("terminal {}", unified_fallback_snapshot(runtime)));
    while let Some(event) = runtime.poll_unified_document() {
        trace.push(format!(
            "event {}",
            serde_json::to_string(&event.unwrap())
                .unwrap()
                .replace(&*root.to_string_lossy().replace('\\', "\\\\"), "<ROOT>")
        ));
    }
}

#[cfg(windows)]
fn finish_real_eof_peer(
    runtime: &mut ImportRuntime,
    worker: &Path,
    local: &Path,
    peer_id: whisper_core::parakeet_scheduler::SchedulerSession,
    deadline: Instant,
    wire_cursor: &mut usize,
    trace: &mut Vec<String>,
) -> u64 {
    while !runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .is_ready()
        .unwrap()
    {
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    append_recovery_wire(worker, local, wire_cursor, trace);
    let ready = runtime.poll_unified_sessions().unwrap().unwrap();
    trace.push(format!("event {}", serde_json::to_string(&ready).unwrap()));
    assert!(runtime.poll_unified_sessions().is_none());
    trace.push(format!(
        "peer dispatched {}",
        unified_fallback_snapshot(runtime)
    ));
    let executor = runtime.unified_executor.as_mut().unwrap();
    let result = executor
        .results
        .recv_timeout(Duration::from_secs(20))
        .unwrap();
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    assert!(send.send(result).is_ok());
    executor.results = receive;
    append_recovery_wire(worker, local, wire_cursor, trace);
    assert!(runtime.poll_unified_sessions().is_none());
    trace.push(format!(
        "peer committed {}",
        unified_fallback_snapshot(runtime)
    ));
    assert!(runtime.poll_unified_sessions().is_none());
    trace.push(format!(
        "peer terminal {}",
        unified_fallback_snapshot(runtime)
    ));
    let drain = runtime.document_drains.front().unwrap();
    assert_eq!(
        (drain.job_id, drain.generation),
        (peer_id.job_id, peer_id.generation)
    );
    assert_eq!(
        (drain.revision, drain.total_samples, drain.next_sequence),
        (1, 48_000, 1)
    );
    assert!(!drain.ready_pending && drain.progress_pending);
    let mut pid = None;
    while let Some(event) = runtime.poll_unified_document() {
        let event = event.unwrap();
        let mut value = serde_json::to_value(&event).unwrap();
        if let ImportEvent::Segment(segment) = &event {
            pid = Some(segment.instance_id);
            value["Segment"]["instance_id"] = serde_json::Value::String("<PID1>".into());
        }
        trace.push(format!("event {}", serde_json::to_string(&value).unwrap()));
    }
    append_recovery_wire(worker, local, wire_cursor, trace);
    pid.unwrap()
}

#[cfg(windows)]
fn three_second_source(local: &Path) -> PathBuf {
    let source = local.join("source.wav");
    let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source).unwrap();
    writer.append(&vec![1000_i16; 3 * 16_000]).unwrap();
    writer.finish().unwrap();
    source
}

#[test]
fn unified_poll_completed_session_restarts_worker_for_surviving_peer() {
    let (mut runtime, first) = unified_poll_import_fixture("unified-poll-completed-peer", 73_114);
    let root = PathBuf::from(&first.destination)
        .parent()
        .unwrap()
        .to_path_buf();
    let peer = unified_import_request(&root.join("peer"), Path::new(&first.source_path), 73_115);
    runtime.start_controlled_unified_import(&peer).unwrap();
    let first_id = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: first.job_id,
        generation: first.generation,
    };
    let peer_id = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: peer.job_id,
        generation: peer.generation,
    };
    let (call_sent, calls) = std::sync::mpsc::sync_channel(1);
    let (release, released) = std::sync::mpsc::sync_channel(1);
    let worker = UnifiedTraceWorker {
        call_sent,
        release: released,
        root,
    };
    let mut executor = UnifiedInferenceExecutor::start(Box::new(worker)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !executor.is_ready().unwrap() {
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    runtime.controlled_transcriber = None;
    runtime.unified_executor = Some(executor);
    assert!(runtime.poll_unified_sessions().is_none());
    assert_eq!(
        runtime.parakeet_scheduler.in_flight().unwrap().session,
        first_id
    );
    calls.recv_timeout(Duration::from_secs(10)).unwrap();
    release.send(()).unwrap();
    let executor = runtime.unified_executor.as_mut().unwrap();
    let result = executor
        .results
        .recv_timeout(Duration::from_secs(10))
        .unwrap();
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    assert!(send.send(result).is_ok());
    executor.results = receive;
    assert!(runtime.poll_unified_sessions().is_none());
    assert!(runtime.poll_unified_sessions().is_none());
    assert!(!runtime.unified_sessions.contains_key(&first_id));
    assert!(runtime.unified_sessions.contains_key(&peer_id));
    assert_eq!(runtime.parakeet_scheduler.active_sessions(), 1);
    assert!(runtime.unified_executor.as_ref().unwrap().thread.is_some());
    let drain = runtime.document_drains.front().unwrap();
    assert_eq!(
        (drain.job_id, drain.generation),
        (first_id.job_id, first_id.generation)
    );
    runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .stop_and_reap()
        .unwrap();
}

#[test]
fn unified_poll_finished_session_with_other_reserved_call_reports_reap_error() {
    let (mut runtime, first) =
        unified_poll_import_fixture("unified-poll-finished-other-reservation", 73_133);
    let root = PathBuf::from(&first.destination)
        .parent()
        .unwrap()
        .to_path_buf();
    let peer = unified_import_request(&root.join("peer"), Path::new(&first.source_path), 73_134);
    runtime.start_controlled_unified_import(&peer).unwrap();
    let first_id = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: first.job_id,
        generation: first.generation,
    };
    let peer_id = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: peer.job_id,
        generation: peer.generation,
    };
    let (call_sent, calls) = std::sync::mpsc::sync_channel(1);
    let (release, released) = std::sync::mpsc::sync_channel(1);
    let worker = UnifiedTraceWorker {
        call_sent,
        release: released,
        root,
    };
    let mut executor = UnifiedInferenceExecutor::start(Box::new(worker)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !executor.is_ready().unwrap() {
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    runtime.controlled_transcriber = None;
    runtime.unified_executor = Some(executor);
    assert!(runtime.poll_unified_sessions().is_none());
    calls.recv_timeout(Duration::from_secs(10)).unwrap();
    release.send(()).unwrap();
    let executor = runtime.unified_executor.as_mut().unwrap();
    let result = executor
        .results
        .recv_timeout(Duration::from_secs(10))
        .unwrap();
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    assert!(send.send(result).is_ok());
    executor.results = receive;
    assert!(runtime.poll_unified_sessions().is_none());
    // Deliberately reserve the peer without starting executor work.
    let call = runtime.parakeet_scheduler.next_call().unwrap().unwrap();
    assert_eq!(call.session, peer_id);
    let outcome = runtime.poll_unified_sessions().unwrap().unwrap_err();
    assert_eq!(
        (outcome.0, outcome.1),
        (first_id.job_id, first_id.generation)
    );
    assert_eq!(outcome.2, "SchedulerReap: CallInFlight");
    assert!(runtime.unified_sessions.contains_key(&first_id));
    assert!(runtime.unified_sessions.contains_key(&peer_id));
    assert_eq!(runtime.parakeet_scheduler.in_flight(), Some(call));
    assert!(runtime.document_drains.is_empty());
}
