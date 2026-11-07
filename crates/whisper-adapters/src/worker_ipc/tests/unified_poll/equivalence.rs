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
    let archive = PathBuf::from(&request.destination);
    let document = start_trace_diagnostics(&mut runtime, &archive, session);
    let mut transitions = Vec::new();
    let mut trace = Vec::new();
    trace.push(format!("resumed {}", unified_fallback_snapshot(&runtime)));
    assert!(record_import_poll(&mut runtime, &document, &mut transitions).is_none());
    trace.push(format!(
        "dispatched {}",
        unified_fallback_snapshot(&runtime)
    ));
    let command = calls.recv_timeout(Duration::from_secs(10)).unwrap();
    transitions.push(format!("fixture received command {command}"));
    trace.push(format!("worker command {command}"));
    release.send(()).unwrap();
    transitions.push("fixture released worker".into());
    let executor = runtime.unified_executor.as_mut().unwrap();
    let result = executor
        .results
        .recv_timeout(Duration::from_secs(10))
        .unwrap();
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    assert!(send.send(result).is_ok());
    executor.results = receive;
    transitions.push("fixture returned worker result".into());
    assert!(record_import_poll(&mut runtime, &document, &mut transitions).is_none());
    trace.push(format!("committed {}", unified_fallback_snapshot(&runtime)));
    assert!(record_import_poll(&mut runtime, &document, &mut transitions).is_none());
    trace.push(format!("terminal {}", unified_fallback_snapshot(&runtime)));
    record_resumed_document_events(&mut runtime, session, &mut trace);
    assert_empty_diagnostics_after_join(&mut runtime, &archive);
    check_resume_transitions(&transitions);
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

fn check_resume_transitions(transitions: &[String]) {
    let path =
        test_artifact_root("unified-poll-resume-transitions-baseline").with_extension("json");
    let bytes = serde_json::to_vec_pretty(transitions).unwrap();
    std::fs::write(path, &bytes).unwrap();
    assert_eq!(
        bytes.as_slice(),
        include_bytes!("reference_resume_poll_transitions.json")
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
    let archive = PathBuf::from(&request.destination);
    let document = start_trace_diagnostics(&mut runtime, &archive, session);
    let mut transitions = Vec::new();
    let mut trace = Vec::new();
    trace.push(format!("admitted {}", unified_fallback_snapshot(&runtime)));
    assert!(record_import_poll(&mut runtime, &document, &mut transitions).is_none());
    assert!(runtime.unified_sessions[&session].pending_call.is_some());
    trace.push(format!(
        "dispatched {}",
        unified_fallback_snapshot(&runtime)
    ));
    let call = calls
        .recv_timeout(Duration::from_secs(10))
        .expect("controlled command");
    transitions.push(format!("fixture received command {call}"));
    trace.push(format!("worker command {call}"));
    release.send(()).unwrap();
    transitions.push("fixture released worker".into());
    let executor = runtime.unified_executor.as_mut().unwrap();
    let result = executor
        .results
        .recv_timeout(Duration::from_secs(10))
        .expect("controlled result");
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    assert!(send.send(result).is_ok());
    executor.results = receive;
    transitions.push("fixture returned worker result".into());
    trace.push("worker result returned".to_owned());
    assert!(record_import_poll(&mut runtime, &document, &mut transitions).is_none());
    trace.push(format!("committed {}", unified_fallback_snapshot(&runtime)));
    assert!(record_import_poll(&mut runtime, &document, &mut transitions).is_none());
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
    assert_empty_diagnostics_after_join(&mut runtime, &archive);
    let transition_path =
        test_artifact_root("unified-poll-import-transitions-baseline").with_extension("json");
    let transition_bytes = serde_json::to_vec_pretty(&transitions).unwrap();
    std::fs::write(transition_path, &transition_bytes).unwrap();
    assert_eq!(
        transition_bytes.as_slice(),
        include_bytes!("reference_import_poll_transitions.json")
    );
    let path = test_artifact_root("unified-poll-import-trace").with_extension("json");
    let trace_bytes = serde_json::to_vec_pretty(&trace).unwrap();
    std::fs::write(path, &trace_bytes).unwrap();
    assert_eq!(
        trace_bytes.as_slice(),
        include_bytes!("reference_import.json"),
        "nominal import effects diverged from the frozen baseline"
    );
}

fn record_import_poll(
    runtime: &mut ImportRuntime,
    document: &Path,
    trace: &mut Vec<String>,
) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
    let before = unified_fallback_snapshot(runtime);
    let files_before = document_file_snapshot(document);
    let result = runtime.poll_unified_sessions();
    let after = unified_fallback_snapshot(runtime);
    let files_after = document_file_snapshot(document);
    if before != after || files_before != files_after || result.is_some() {
        trace.push(
            serde_json::to_string(&(before, &result, after, files_before, files_after)).unwrap(),
        );
    }
    result
}

fn document_file_snapshot(directory: &Path) -> Vec<(&'static str, Option<Vec<u8>>)> {
    [
        "document.idx",
        "document.jsonl",
        "coverage.jsonl",
        "document.final.json",
        "document.preview.json",
    ]
    .into_iter()
    .map(|name| (name, std::fs::read(directory.join(name)).ok()))
    .collect()
}

fn trace_document_path(
    archive: &Path,
    session: whisper_core::parakeet_scheduler::SchedulerSession,
) -> PathBuf {
    archive
        .join("transcriptions")
        .join(format!("{:032x}", session.job_id.0))
        .join(format!(
            "{}/parakeet-unified/document",
            session.generation.get()
        ))
}

fn start_trace_diagnostics(
    runtime: &mut ImportRuntime,
    archive: &Path,
    session: whisper_core::parakeet_scheduler::SchedulerSession,
) -> PathBuf {
    runtime.start_diagnostics(archive);
    assert!(runtime.diagnostics.is_some());
    trace_document_path(archive, session)
}

#[cfg(windows)]
struct LivePollTrace {
    document: PathBuf,
    entries: Vec<String>,
}

#[cfg(windows)]
impl LivePollTrace {
    fn new(root: &Path, session: whisper_core::parakeet_scheduler::SchedulerSession) -> Self {
        Self {
            document: root
                .join("transcriptions")
                .join(format!("{:032x}", session.job_id.0))
                .join(format!(
                    "{}/parakeet-unified/document",
                    session.generation.get()
                )),
            entries: Vec::new(),
        }
    }
    fn poll(
        &mut self,
        runtime: &mut ImportRuntime,
    ) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
        record_import_poll(runtime, &self.document, &mut self.entries)
    }
    fn write(&self, name: &str) {
        let path = test_artifact_root(name).with_extension("json");
        let bytes = serde_json::to_vec_pretty(&self.entries).unwrap();
        std::fs::write(path, &bytes).unwrap();
        let reference: &[u8] = match name {
            "unified-poll-stop-prepare-transitions" => {
                include_bytes!("reference_stop_prepare_transitions.json")
            }
            "unified-poll-stop-live-transitions" => {
                include_bytes!("reference_stop_live_transitions.json")
            }
            "unified-poll-stop-before-ready-transitions" => {
                include_bytes!("reference_stop_before_ready_transitions.json")
            }
            "unified-poll-stop-pending-transitions" => {
                include_bytes!("reference_stop_pending_transitions.json")
            }
            _ => panic!("unknown live transition reference"),
        };
        assert_eq!(bytes.as_slice(), reference);
    }
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
    let executor = ready_panicking_executor(failed_session, &observations);
    runtime.controlled_transcriber = None;
    runtime.unified_executor = Some(executor);
    let archive = PathBuf::from(&failed.destination);
    let first_document = start_trace_diagnostics(&mut runtime, &archive, failed_session);
    let peer_document = trace_document_path(Path::new(&peer.destination), peer_session);
    let mut transitions = Vec::new();
    let mut trace = Vec::new();
    trace.push(format!("admitted {}", unified_fallback_snapshot(&runtime)));
    assert!(
        record_pair_poll(
            &mut runtime,
            false,
            &first_document,
            &peer_document,
            &mut transitions
        )
        .is_none()
    );
    assert_eq!(
        runtime.parakeet_scheduler.in_flight().unwrap().session,
        failed_session
    );
    trace.push(format!(
        "dispatched {}",
        unified_fallback_snapshot(&runtime)
    ));
    wait_for_panic_worker(&runtime);
    let outcome = record_pair_poll(
        &mut runtime,
        true,
        &first_document,
        &peer_document,
        &mut transitions,
    )
    .expect("worker panic result");
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
    let event = record_pair_poll(
        &mut runtime,
        true,
        &first_document,
        &peer_document,
        &mut transitions,
    )
    .expect("peer failure")
    .expect("peer event");
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
    assert_empty_diagnostics_after_join(&mut runtime, &archive);
    check_panic_trace_files(&transitions, &trace);
}

fn wait_for_panic_worker(runtime: &ImportRuntime) {
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
}

fn check_panic_trace_files(transitions: &[String], trace: &[String]) {
    let path = test_artifact_root("unified-poll-panic-transitions-baseline").with_extension("json");
    let bytes = serde_json::to_vec_pretty(transitions).unwrap();
    std::fs::write(path, &bytes).unwrap();
    assert_eq!(
        bytes.as_slice(),
        include_bytes!("reference_panic_poll_transitions.json")
    );
    let path = test_artifact_root("unified-poll-panic-trace").with_extension("json");
    let bytes = serde_json::to_vec_pretty(trace).unwrap();
    std::fs::write(path, &bytes).unwrap();
    assert_eq!(
        bytes.as_slice(),
        include_bytes!("reference_panic.json"),
        "panic and terminal release effects diverged from the frozen baseline"
    );
}

fn ready_panicking_executor(
    session: whisper_core::parakeet_scheduler::SchedulerSession,
    observations: &Arc<Mutex<Vec<ControlledChildObservation>>>,
) -> UnifiedInferenceExecutor {
    let mut executor =
        UnifiedInferenceExecutor::start_observed(controlled_worker().0, Arc::clone(observations))
            .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !executor.is_ready().unwrap() {
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    executor.panic_during_transcription_for = Some(session);
    executor.stop_and_reap().unwrap();
    executor.restart().unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !executor.is_ready().unwrap() {
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    executor
}

fn record_pair_poll(
    runtime: &mut ImportRuntime,
    routed: bool,
    first: &Path,
    peer: &Path,
    trace: &mut Vec<String>,
) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
    let before = unified_fallback_snapshot(runtime);
    let files_before = (document_file_snapshot(first), document_file_snapshot(peer));
    let result = if routed {
        runtime.poll()
    } else {
        runtime.poll_unified_sessions()
    };
    let after = unified_fallback_snapshot(runtime);
    let files_after = (document_file_snapshot(first), document_file_snapshot(peer));
    if before != after || files_before != files_after || result.is_some() {
        trace.push(
            serde_json::to_string(&(before, &result, after, files_before, files_after)).unwrap(),
        );
    }
    result
}

#[cfg(windows)]
#[test]
fn unified_poll_stop_during_prepare_records_effects() {
    let root = test_artifact_root("unified-poll-stop-prepare");
    std::fs::create_dir_all(&root).unwrap();
    let mut request = unified_live_request(&root, 73_031);
    request.model_path = verified_personal_model();
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let mut observed = LivePollTrace::new(&root, session);
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
    runtime.start_diagnostics(&root);
    assert!(runtime.diagnostics.is_some());
    runtime
        .start_unified_capture(&request, capture, None, None)
        .unwrap();
    let probe_hold = runtime.unified_sessions[&session]
        .source
        .hold_probe_publication();
    let mut trace = vec![format!("started {}", unified_fallback_snapshot(&runtime))];
    assert!(observed.poll(&mut runtime).is_none());
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
        &mut observed,
    );
    assert_empty_diagnostics_after_join(&mut runtime, &root);
    observed.write("unified-poll-stop-prepare-transitions");
    let path = test_artifact_root("unified-poll-stop-prepare-trace").with_extension("json");
    let trace_bytes = serde_json::to_vec_pretty(&trace).unwrap();
    std::fs::write(path, &trace_bytes).unwrap();
    assert_portable_model_trace(
        &trace_bytes,
        include_bytes!("reference_stop_prepare.json"),
        &request.model_path,
    );
}

#[cfg(windows)]
#[test]
fn unified_poll_stop_during_live_records_effects() {
    let root = test_artifact_root("unified-poll-stop-live");
    std::fs::create_dir_all(&root).unwrap();
    let mut request = unified_live_request(&root, 73_032);
    request.model_path = verified_personal_model();
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let mut observed = LivePollTrace::new(&root, session);
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
    runtime.start_diagnostics(&root);
    assert!(runtime.diagnostics.is_some());
    runtime
        .start_unified_capture(&request, capture, None, None)
        .unwrap();
    let probe_hold = runtime.unified_sessions[&session]
        .source
        .hold_probe_publication();
    let mut trace = vec![format!("started {}", unified_fallback_snapshot(&runtime))];
    assert!(observed.poll(&mut runtime).is_none());
    assert!(runtime.unified_executor.is_some());
    trace.push(format!("preparing {}", unified_fallback_snapshot(&runtime)));
    let _expected = pump_live_before_stop(
        &mut runtime,
        session,
        &ready_gate,
        &mut trace,
        &mut observed,
    );
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
        &mut observed,
    );
    assert_empty_diagnostics_after_join(&mut runtime, &root);
    observed.write("unified-poll-stop-live-transitions");
    let path = test_artifact_root("unified-poll-stop-live-trace").with_extension("json");
    let trace_bytes = serde_json::to_vec_pretty(&trace).unwrap();
    std::fs::write(path, &trace_bytes).unwrap();
    assert_portable_model_trace(
        &trace_bytes,
        include_bytes!("reference_stop_live.json"),
        &request.model_path,
    );
}

#[cfg(windows)]
#[test]
fn unified_poll_stop_before_worker_ready_records_effects() {
    let root = test_artifact_root("unified-poll-stop-before-ready");
    std::fs::create_dir_all(&root).unwrap();
    let mut request = unified_live_request(&root, 73_033);
    request.model_path = verified_personal_model();
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let mut observed = LivePollTrace::new(&root, session);
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
    runtime.start_diagnostics(&root);
    assert!(runtime.diagnostics.is_some());
    runtime
        .start_unified_capture(&request, capture, None, None)
        .unwrap();
    let probe_hold = runtime.unified_sessions[&session]
        .source
        .hold_probe_publication();
    let mut trace = vec![format!("started {}", unified_fallback_snapshot(&runtime))];
    assert!(observed.poll(&mut runtime).is_none());
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
        &mut observed,
    );
    assert_empty_diagnostics_after_join(&mut runtime, &root);
    observed.write("unified-poll-stop-before-ready-transitions");
    let path = test_artifact_root("unified-poll-stop-before-ready-trace").with_extension("json");
    let trace_bytes = serde_json::to_vec_pretty(&trace).unwrap();
    std::fs::write(path, &trace_bytes).unwrap();
    assert_portable_model_trace(
        &trace_bytes,
        include_bytes!("reference_stop_before_ready.json"),
        &request.model_path,
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
    let transitions = std::fs::read(local.join("eof-poll-transitions.json")).unwrap();
    assert_eq!(
        transitions.as_slice(),
        include_bytes!("reference_real_eof_poll_transitions.json")
    );
}

#[cfg(windows)]
fn run_unified_poll_real_ipc_eof_child(native_candidate: bool) {
    const SHA: &str = "e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e";
    let model = verified_personal_model();
    let local = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap());
    let (paths, worker) = prepare_fake_personal_worker(&local, &model, SHA, "fake_eof_worker.rs");
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
    let (startup_release, startup_wait) = std::sync::mpsc::channel();
    let (dispatch_release, dispatch_wait) = std::sync::mpsc::channel();
    if native_candidate {
        runtime.native_unified_candidate_enabled = true;
    } else {
        runtime = runtime.with_controlled_transcriber(Box::new(RecoveryDispatchBarrier {
            inner: NativeParakeetWindowTranscriber::new(route.clone()),
            startup: startup_wait,
            release: dispatch_wait,
        }));
    }
    runtime.native_unified_routes.insert(session, route);
    runtime.start_controlled_unified_import(&request).unwrap();
    let archive = PathBuf::from(&request.destination);
    let mut observed = FallbackPollTrace::new(&worker, &local);
    observed.document = Some(start_trace_diagnostics(&mut runtime, &archive, session));
    let mut trace = vec![format!("started {}", unified_fallback_snapshot(&runtime))];
    assert!(observed.poll(&mut runtime).is_none());
    if !native_candidate {
        assert_recovery_wire_boundary(&worker, 0).unwrap();
        startup_release.send(()).unwrap();
    }
    let deadline = Instant::now() + Duration::from_secs(20);
    wait_recovery_ready(&mut runtime, deadline);
    let ready = observed.poll(&mut runtime).unwrap().unwrap();
    trace.push(format!("event {}", serde_json::to_string(&ready).unwrap()));
    assert!(observed.poll(&mut runtime).is_none());
    trace.push(format!(
        "dispatched {}",
        unified_fallback_snapshot(&runtime)
    ));
    if !native_candidate {
        release_recovery_dispatch(&worker, &dispatch_release, 1);
    }
    let executor = runtime.unified_executor.as_mut().unwrap();
    let result = executor
        .results
        .recv_timeout(Duration::from_secs(20))
        .unwrap();
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    assert!(send.send(result).is_ok());
    executor.results = receive;
    let error = observed.poll(&mut runtime).unwrap().unwrap_err();
    assert!(
        error.2.contains("WorkerExited"),
        "unexpected failure: {error:?}"
    );
    trace.push(format!("failure {error:?}"));
    trace.push(format!(
        "after failure {}",
        unified_fallback_snapshot(&runtime)
    ));
    assert_empty_diagnostics_after_join(&mut runtime, &archive);
    observed.drain_wire("after reap");
    std::fs::write(
        local.join("eof-poll-transitions.json"),
        serde_json::to_vec_pretty(&observed.entries).unwrap(),
    )
    .unwrap();
    let local_path = local.to_string_lossy().replace('\\', "\\\\");
    for line in std::fs::read_to_string(worker.with_extension("wire"))
        .unwrap()
        .lines()
    {
        trace.push(format!("wire {}", line.replace(&local_path, "<ROOT>")));
    }
    assert_eq!(
        crate::parakeet_import::sha256_file(Path::new(&model)).unwrap(),
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
    request.model_path = verified_personal_model();
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let (call_sent, calls) = std::sync::mpsc::sync_channel(1);
    let (release, released) = std::sync::mpsc::sync_channel(1);
    let worker = UnifiedTraceWorker {
        call_sent,
        release: released,
        root: root.clone(),
    };
    let mut runtime = live_preview_runtime(&root, &request, worker);
    let probe_hold = runtime.unified_sessions[&session]
        .source
        .hold_probe_publication();
    let mut trace = vec![format!("started {}", unified_fallback_snapshot(&runtime))];
    let mut detailed_trace = vec![format!("started {}", unified_fallback_snapshot(&runtime))];
    let preview_path = preview_document_path(&root, &request);
    let mut effects = LiveEffectTrace {
        preview_path,
        polls: Vec::new(),
        files: Vec::new(),
        joined: Vec::new(),
    };
    assert!(record_unified_poll(&mut runtime, &mut effects).is_none());
    let mut last_poll_state = unified_fallback_snapshot(&runtime);
    detailed_trace.push(format!("poll {}", last_poll_state));
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
    effects.joined.push("fixture observed ready".into());
    runtime
        .unified_sessions
        .get_mut(&session)
        .unwrap()
        .source
        .enqueue_test_block(vec![0.1; 70_000]);
    effects.joined.push("fixture enqueued 70000 samples".into());
    await_preview_dispatch(
        &mut runtime,
        session,
        deadline,
        &mut effects,
        &mut detailed_trace,
        &mut last_poll_state,
    );
    trace.push(format!(
        "preview dispatched {}",
        unified_fallback_snapshot(&runtime)
    ));
    commit_controlled_preview(&mut runtime, &calls, &release, &mut effects, &mut trace);
    append_preview_detail(&runtime, &request, &root, &mut detailed_trace);
    trace.push(format!(
        "preview committed {}",
        unified_fallback_snapshot(&runtime)
    ));
    let events = stop_live_with_joined_trace(&mut runtime, &request, &root, &mut effects);
    drop(probe_hold);
    append_live_stop_events(&mut trace, &root, events);
    trace.push(format!("stopped {}", unified_fallback_snapshot(&runtime)));
    finish_live_final_trace(
        &mut runtime,
        &calls,
        &release,
        &root,
        &mut trace,
        &mut effects,
    );
    assert_empty_diagnostics_after_join(&mut runtime, &root);
    check_live_detail(&runtime, &mut detailed_trace);
    check_live_trace_files(&effects.polls, &trace, &request.model_path);
    check_live_document_deltas(&effects.files);
    check_live_joined_trace(&effects.joined, &request.model_path);
}

#[cfg(windows)]
fn commit_controlled_preview(
    runtime: &mut ImportRuntime,
    calls: &std::sync::mpsc::Receiver<String>,
    release: &std::sync::mpsc::SyncSender<()>,
    effects: &mut LiveEffectTrace,
    trace: &mut Vec<String>,
) {
    let command = calls.recv_timeout(Duration::from_secs(10)).unwrap();
    effects
        .joined
        .push(format!("fixture received command {command}"));
    trace.push(format!("worker command {command}"));
    release.send(()).unwrap();
    effects.joined.push("fixture released worker".into());
    let executor = runtime.unified_executor.as_mut().unwrap();
    let result = executor
        .results
        .recv_timeout(Duration::from_secs(10))
        .unwrap();
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    assert!(send.send(result).is_ok());
    let original_results = std::mem::replace(&mut executor.results, receive);
    effects.joined.push("fixture returned worker result".into());
    assert!(record_unified_poll(runtime, effects).is_none());
    runtime.unified_executor.as_mut().unwrap().results = original_results;
}

#[cfg(windows)]
fn stop_live_with_joined_trace(
    runtime: &mut ImportRuntime,
    request: &LiveRequest,
    root: &Path,
    effects: &mut LiveEffectTrace,
) -> VecDeque<ImportEvent> {
    let mut events = VecDeque::new();
    runtime
        .stop_controlled_unified_capture(request.job_id, request.generation, &mut events)
        .unwrap();
    effects.joined.push("fixture stopped capture".into());
    for event in &events {
        effects.joined.push(format!(
            "stop event {}",
            serde_json::to_string(event)
                .unwrap()
                .replace(&*root.to_string_lossy().replace('\\', "\\\\"), "<ROOT>")
        ));
    }
    events
}

#[cfg(windows)]
fn check_live_joined_trace(joined: &[String], model_path: &str) {
    let path = test_artifact_root("unified-poll-live-joined-baseline").with_extension("json");
    let bytes = serde_json::to_vec_pretty(joined).unwrap();
    std::fs::write(path, &bytes).unwrap();
    assert_portable_model_trace(
        &bytes,
        include_bytes!("reference_live_joined_poll_effects.json"),
        model_path,
    );
}

#[cfg(windows)]
fn check_live_document_deltas(files: &[String]) {
    let file_path = test_artifact_root("unified-poll-live-files-baseline").with_extension("json");
    let bytes = serde_json::to_vec_pretty(files).unwrap();
    std::fs::write(file_path, &bytes).unwrap();
    assert_eq!(
        bytes.as_slice(),
        include_bytes!("reference_live_document_deltas.json")
    );
}

#[cfg(windows)]
fn live_preview_runtime(
    root: &Path,
    request: &LiveRequest,
    worker: UnifiedTraceWorker,
) -> ImportRuntime {
    let capture = CaptureStream::empty_test_capture(16_000, 1, "test:preview-final");
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: PathBuf::from("unused-cpu"),
            gpu: PathBuf::from("unused-gpu"),
        },
        Arc::new(PublicationGate::default()),
    )
    .with_controlled_transcriber(Box::new(worker));
    runtime.start_diagnostics(root);
    assert!(runtime.diagnostics.is_some());
    runtime
        .start_unified_capture(request, capture, None, None)
        .unwrap();
    runtime
}

#[cfg(windows)]
fn await_preview_dispatch(
    runtime: &mut ImportRuntime,
    session: whisper_core::parakeet_scheduler::SchedulerSession,
    deadline: Instant,
    effects: &mut LiveEffectTrace,
    detailed_trace: &mut Vec<String>,
    last_poll_state: &mut String,
) {
    while runtime.unified_sessions[&session].pending_call.is_none() {
        assert!(record_unified_poll(runtime, effects).is_none());
        let snapshot = unified_fallback_snapshot(runtime);
        if snapshot != *last_poll_state {
            detailed_trace.push(format!("poll {}", snapshot));
            *last_poll_state = snapshot;
        }
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
}

#[cfg(windows)]
fn check_live_trace_files(effects: &[String], trace: &[String], model_path: &str) {
    let effect_path =
        test_artifact_root("unified-poll-live-transitions-baseline").with_extension("json");
    let effect_bytes = serde_json::to_vec_pretty(effects).unwrap();
    std::fs::write(effect_path, &effect_bytes).unwrap();
    assert_eq!(
        effect_bytes.as_slice(),
        include_bytes!("reference_live_poll_transitions.json")
    );
    let path = test_artifact_root("unified-poll-live-preview-final-trace").with_extension("json");
    let trace_bytes = serde_json::to_vec_pretty(trace).unwrap();
    std::fs::write(path, &trace_bytes).unwrap();
    assert_portable_model_trace(
        &trace_bytes,
        include_bytes!("reference_live_preview_final.json"),
        model_path,
    );
}

fn assert_empty_diagnostics_after_join(runtime: &mut ImportRuntime, root: &Path) {
    let sink = runtime.diagnostics.take().expect("diagnostic sink started");
    assert_eq!(sink.dropped_count(), 0);
    assert!(!sink.is_degraded());
    drop(sink);
    assert_eq!(
        std::fs::read_dir(root.join(".whisper-diagnostics"))
            .unwrap()
            .count(),
        0,
        "no diagnostic record persisted"
    );
}

#[cfg(windows)]
fn append_preview_detail(
    runtime: &ImportRuntime,
    request: &LiveRequest,
    root: &Path,
    trace: &mut Vec<String>,
) {
    trace.push(format!(
        "preview committed {}",
        unified_fallback_snapshot(runtime)
    ));
    let path = preview_document_path(root, request);
    trace.push(format!(
        "preview {}",
        std::fs::read_to_string(path).expect("preview document")
    ));
}

#[cfg(windows)]
fn preview_document_path(root: &Path, request: &LiveRequest) -> PathBuf {
    root.join("transcriptions")
        .join(format!("{:032x}", request.job_id.0))
        .join(format!(
            "{}/parakeet-unified/document/document.preview.json",
            request.generation.get()
        ))
}

#[cfg(windows)]
struct LiveEffectTrace {
    preview_path: PathBuf,
    polls: Vec<String>,
    files: Vec<String>,
    joined: Vec<String>,
}

#[cfg(windows)]
fn record_unified_poll(
    runtime: &mut ImportRuntime,
    trace: &mut LiveEffectTrace,
) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
    let before = unified_fallback_snapshot(runtime);
    let preview_before = std::fs::read_to_string(&trace.preview_path).ok();
    let document = trace.preview_path.parent().unwrap();
    let files_before = document_file_snapshot(document);
    let result = runtime.poll_unified_sessions();
    let after = unified_fallback_snapshot(runtime);
    let preview_after = std::fs::read_to_string(&trace.preview_path).ok();
    let files_after = document_file_snapshot(document);
    if before != after
        || preview_before != preview_after
        || files_before != files_after
        || result.is_some()
    {
        trace.joined.push(encode_joined_poll(
            &before,
            &result,
            &after,
            &files_before,
            &files_after,
        ));
        trace.polls.push(
            serde_json::to_string(&(before, &result, after, preview_before, preview_after))
                .unwrap(),
        );
    }
    if files_before != files_after {
        trace
            .files
            .push(serde_json::to_string(&(files_before, files_after)).unwrap());
    }
    result
}

#[cfg(windows)]
fn encode_joined_poll(
    before: &str,
    result: &Option<Result<ImportEvent, (JobId, Generation, String)>>,
    after: &str,
    files_before: &[(&'static str, Option<Vec<u8>>)],
    files_after: &[(&'static str, Option<Vec<u8>>)],
) -> String {
    serde_json::to_string(&(before, result, after, files_before, files_after)).unwrap()
}

#[cfg(windows)]
#[test]
fn unified_poll_joined_trace_detects_file_effect_moved_between_polls() {
    let old = vec![("document.jsonl", Some(b"old".to_vec()))];
    let new = vec![("document.jsonl", Some(b"new".to_vec()))];
    let result = None;
    let original = [
        encode_joined_poll("a", &result, "b", &old, &new),
        encode_joined_poll("b", &result, "c", &new, &new),
    ];
    let shifted = [
        encode_joined_poll("a", &result, "b", &old, &old),
        encode_joined_poll("b", &result, "c", &old, &new),
    ];
    assert_ne!(
        serde_json::to_vec(&original).unwrap(),
        serde_json::to_vec(&shifted).unwrap()
    );
}

#[cfg(windows)]
fn check_live_detail(runtime: &ImportRuntime, trace: &mut Vec<String>) {
    trace.push(format!(
        "after stop and final {}",
        unified_fallback_snapshot(runtime)
    ));
    let path = test_artifact_root("unified-poll-live-detailed-baseline").with_extension("json");
    let bytes = serde_json::to_vec_pretty(trace).unwrap();
    std::fs::write(path, &bytes).unwrap();
    assert_eq!(
        bytes.as_slice(),
        include_bytes!("reference_live_preview_detailed.json")
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
    request.model_path = verified_personal_model();
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let mut observed = LivePollTrace::new(&root, session);
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
    runtime.start_diagnostics(&root);
    assert!(runtime.diagnostics.is_some());
    runtime
        .start_unified_capture(&request, capture, None, None)
        .unwrap();
    let probe_hold = runtime.unified_sessions[&session]
        .source
        .hold_probe_publication();
    let mut trace = vec![format!("started {}", unified_fallback_snapshot(&runtime))];
    assert!(observed.poll(&mut runtime).is_none());
    assert!(runtime.unified_executor.is_some());
    trace.push(format!("preparing {}", unified_fallback_snapshot(&runtime)));
    let expected = pump_live_before_stop(
        &mut runtime,
        session,
        &ready_gate,
        &mut trace,
        &mut observed,
    );
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
        &mut observed,
    );
    assert_empty_diagnostics_after_join(&mut runtime, &root);
    observed.write("unified-poll-stop-pending-transitions");
    let path = test_artifact_root("unified-poll-stop-pending-audio-trace").with_extension("json");
    let trace_bytes = serde_json::to_vec_pretty(&trace).unwrap();
    std::fs::write(path, &trace_bytes).unwrap();
    assert_portable_model_trace(
        &trace_bytes,
        include_bytes!("reference_stop_pending_audio.json"),
        &request.model_path,
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
    let segment_pid: u64 =
        serde_json::from_slice(&std::fs::read(local.join("recovery-id-map.json")).unwrap())
            .unwrap();
    let transitions = std::fs::read(local.join("recovery-poll-transitions.json")).unwrap();
    assert_portable_index_trace(
        &transitions,
        include_bytes!("reference_real_eof_peer_poll_transitions.json"),
        segment_pid,
        "<PID2>",
    );
    let joined = std::fs::read(local.join("recovery-joined.json")).unwrap();
    assert_portable_index_trace(
        &joined,
        include_bytes!("reference_real_eof_peer_joined_poll_effects.json"),
        segment_pid,
        "<PID2>",
    );
}

#[cfg(windows)]
struct RecoveryDispatchBarrier {
    inner: NativeParakeetWindowTranscriber,
    startup: std::sync::mpsc::Receiver<()>,
    release: std::sync::mpsc::Receiver<()>,
}

#[cfg(windows)]
impl crate::parakeet_pipeline::WindowTranscriber for RecoveryDispatchBarrier {
    fn instance_id(&self) -> u64 {
        self.inner.instance_id()
    }

    fn initialize_instance(&mut self, stop: &std::sync::atomic::AtomicBool) -> Result<(), String> {
        self.startup
            .recv_timeout(Duration::from_secs(20))
            .map_err(|error| error.to_string())?;
        self.inner.initialize_instance(stop)
    }

    fn stop_instance(&mut self) -> Result<(), String> {
        self.inner.stop_instance()
    }

    fn transcribe(
        &mut self,
        window: whisper_core::parakeet_session::AudioWindow,
        pcm: &[i16],
    ) -> Result<Vec<whisper_core::parakeet_ipc::Word>, String> {
        self.inner.transcribe(window, pcm)
    }

    fn transcribe_scheduled(
        &mut self,
        call: whisper_core::parakeet_scheduler::ScheduledCall,
        scratch: &Path,
        window: whisper_core::parakeet_session::AudioWindow,
        pcm: &[i16],
    ) -> Result<Vec<whisper_core::parakeet_ipc::Word>, String> {
        self.release
            .recv_timeout(Duration::from_secs(20))
            .map_err(|error| error.to_string())?;
        self.inner.transcribe_scheduled(call, scratch, window, pcm)
    }
}

#[cfg(windows)]
fn run_unified_poll_real_eof_peer_child() {
    const SHA: &str = "e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e";
    let model = verified_personal_model();
    let local = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap());
    let (paths, worker) =
        prepare_fake_personal_worker(&local, &model, SHA, "fake_recovery_worker.rs");
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
    let (native_observer, native_events) = NativeQualificationObserver::channel();
    let (startup_release, startup_wait) = std::sync::mpsc::channel();
    let (dispatch_release, dispatch_wait) = std::sync::mpsc::channel();
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: PathBuf::from("unused-cpu"),
            gpu: PathBuf::from("unused-gpu"),
        },
        Arc::new(PublicationGate::default()),
    )
    .with_controlled_transcriber(Box::new(RecoveryDispatchBarrier {
        inner: NativeParakeetWindowTranscriber::new(route.clone())
            .with_harness_observer(native_observer),
        startup: startup_wait,
        release: dispatch_wait,
    }));
    runtime
        .native_unified_routes
        .insert(first_id, route.clone());
    runtime.start_controlled_unified_import(&first).unwrap();
    runtime.native_unified_routes.insert(peer_id, route);
    runtime.start_controlled_unified_import(&peer).unwrap();
    let archive = PathBuf::from(&first.destination);
    let peer_archive = PathBuf::from(&peer.destination);
    let mut observed = FallbackPollTrace::new(&worker, &local);
    observed.document = Some(start_trace_diagnostics(&mut runtime, &archive, first_id));
    observed.peer_document = Some(trace_document_path(&peer_archive, peer_id));
    let mut trace = vec![format!("started {}", unified_fallback_snapshot(&runtime))];
    let mut wire_cursor = 0;
    assert!(observed.poll(&mut runtime).is_none());
    assert_recovery_wire_boundary(&worker, 0).unwrap();
    startup_release.send(()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    wait_recovery_ready(&mut runtime, deadline);
    append_recovery_wire(&worker, &local, &mut wire_cursor, &mut trace);
    let ready = observed.poll(&mut runtime).unwrap().unwrap();
    trace.push(format!("event {}", serde_json::to_string(&ready).unwrap()));
    assert!(observed.poll(&mut runtime).is_none());
    trace.push(format!(
        "first dispatched {}",
        unified_fallback_snapshot(&runtime)
    ));
    release_recovery_dispatch(&worker, &dispatch_release, 1);
    hold_recovery_result(&mut runtime);
    append_recovery_wire(&worker, &local, &mut wire_cursor, &mut trace);
    let failed = observed.poll_routed(&mut runtime).unwrap().unwrap_err();
    assert_eq!((failed.0, failed.1), (first_id.job_id, first_id.generation));
    assert!(failed.2.contains("WorkerExited"));
    trace.push(format!("failure {failed:?}"));
    assert!(!runtime.unified_sessions.contains_key(&first_id));
    assert!(!runtime.unified_archives.contains_key(&first_id));
    assert!(runtime.unified_sessions.contains_key(&peer_id));
    assert_recovery_wire_boundary(&worker, 2).unwrap();
    startup_release.send(()).unwrap();
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
        (&mut wire_cursor, &mut observed, &dispatch_release),
        &mut trace,
    );
    assert_empty_diagnostics_after_join(&mut runtime, &archive);
    write_recovery_poll_transitions(&mut observed, &native_events, &local, pid);
    assert_eq!(
        crate::parakeet_import::sha256_file(Path::new(&model)).unwrap(),
        SHA
    );
    write_recovery_trace(&local, pid, &trace);
}

#[cfg(windows)]
fn write_recovery_poll_transitions(
    observed: &mut FallbackPollTrace,
    native_events: &std::sync::mpsc::Receiver<NativeQualificationEvent>,
    local: &Path,
    pid: u64,
) {
    let offset = observed.entries.len();
    observed.drain_wire("after reap");
    for entry in &observed.entries[offset..] {
        observed.joined.push(format!("fixture {entry}"));
    }
    check_recovery_observed_pids(native_events, local, pid);
    std::fs::write(
        local.join("recovery-poll-transitions-raw.json"),
        serde_json::to_vec_pretty(&observed.entries).unwrap(),
    )
    .unwrap();
    normalize_document_pid(&mut observed.entries, pid, "peer document files", "<PID2>");
    std::fs::write(
        local.join("recovery-joined-raw.json"),
        serde_json::to_vec_pretty(&observed.joined).unwrap(),
    )
    .unwrap();
    let mut polls: Vec<_> = observed
        .joined
        .iter()
        .filter(|entry| entry.starts_with('['))
        .cloned()
        .collect();
    normalize_joined_pid(&mut polls, pid, [5, 6], "<PID2>");
    let mut normalized = polls.into_iter();
    let joined: Vec<_> = observed
        .joined
        .iter()
        .map(|entry| {
            if entry.starts_with('[') {
                normalized.next().unwrap()
            } else {
                entry.clone()
            }
        })
        .collect();
    std::fs::write(
        local.join("recovery-joined.json"),
        serde_json::to_vec_pretty(&joined).unwrap(),
    )
    .unwrap();
    std::fs::write(
        local.join("recovery-poll-transitions.json"),
        serde_json::to_vec_pretty(&observed.entries).unwrap(),
    )
    .unwrap();
}

#[cfg(windows)]
fn normalize_document_pid(
    entries: &mut [String],
    observed_pid: u64,
    label: &str,
    placeholder: &str,
) {
    let mut replacements = 0;
    for entry in entries {
        if !entry.starts_with(&format!("[\"{label}\"")) {
            continue;
        }
        let mut value: serde_json::Value = serde_json::from_str(entry).unwrap();
        for side in [1_usize, 2] {
            let files = value[side].as_array_mut().unwrap();
            verify_raw_document_files(files)
                .expect("raw document index invalid before PID normalization");
            for file in files {
                if file[0].as_str() != Some("document.jsonl") || file[1].is_null() {
                    continue;
                }
                let bytes: Vec<u8> = serde_json::from_value(file[1].clone()).unwrap();
                let contents = String::from_utf8(bytes).unwrap();
                let original = format!("\"instance_id\":{observed_pid}");
                if contents.contains(&original) {
                    replacements += contents.matches(&original).count();
                    let normalized =
                        contents.replace(&original, &format!("\"instance_id\":\"{placeholder}\""));
                    file[1] = serde_json::to_value(normalized.into_bytes()).unwrap();
                }
            }
        }
        *entry = serde_json::to_string(&value).unwrap();
    }
    assert!(
        replacements > 0,
        "observed PID must occur in persisted document"
    );
}

#[cfg(windows)]
fn normalize_joined_pid(
    entries: &mut [String],
    observed_pid: u64,
    sides: [usize; 2],
    placeholder: &str,
) {
    let mut replacements = 0;
    for entry in entries {
        let mut value: serde_json::Value = serde_json::from_str(entry).unwrap();
        for side in sides {
            let Some(files) = value[side].as_array_mut() else {
                continue;
            };
            verify_raw_document_files(files)
                .expect("raw document index invalid before PID normalization");
            for file in files {
                if file[0].as_str() != Some("document.jsonl") || file[1].is_null() {
                    continue;
                }
                let bytes: Vec<u8> = serde_json::from_value(file[1].clone()).unwrap();
                let text = String::from_utf8(bytes).unwrap();
                let original = format!("\"instance_id\":{observed_pid}");
                if text.contains(&original) {
                    replacements += text.matches(&original).count();
                    file[1] = serde_json::to_value(
                        text.replace(&original, &format!("\"instance_id\":\"{placeholder}\""))
                            .into_bytes(),
                    )
                    .unwrap();
                }
            }
        }
        *entry = serde_json::to_string(&value).unwrap();
    }
    assert!(
        replacements > 0,
        "independently observed PID must appear in joined document"
    );
}

#[cfg(windows)]
fn write_recovery_trace(local: &Path, pid: u64, trace: &[String]) {
    std::fs::write(
        local.join("recovery-id-map.json"),
        serde_json::to_vec(&pid).unwrap(),
    )
    .unwrap();
    std::fs::write(
        local.join("recovery-trace.json"),
        serde_json::to_vec_pretty(trace).unwrap(),
    )
    .unwrap();
}

#[cfg(windows)]
fn check_recovery_observed_pids(
    events: &std::sync::mpsc::Receiver<NativeQualificationEvent>,
    local: &Path,
    segment_pid: u64,
) {
    let observed: Vec<_> = events
        .try_iter()
        .filter_map(|event| match event.kind {
            NativeQualificationEventKind::Spawned { incarnation, pid } => Some((incarnation, pid)),
            _ => None,
        })
        .collect();
    assert_eq!(
        observed.len(),
        2,
        "both native incarnations must be observed independently"
    );
    assert_ne!(observed[0].0, observed[1].0);
    assert!(pid_matches_observed_incarnation(
        segment_pid,
        observed[1].0,
        &observed
    ));
    std::fs::write(
        local.join("recovery-observed-pids.json"),
        serde_json::to_vec(&observed).unwrap(),
    )
    .unwrap();
}

#[cfg(windows)]
fn pid_matches_observed_incarnation(
    segment_pid: u64,
    incarnation: u64,
    observed: &[(u64, u32)],
) -> bool {
    observed.iter().any(|(seen_incarnation, pid)| {
        *seen_incarnation == incarnation && u64::from(*pid) == segment_pid
    })
}

#[cfg(windows)]
#[test]
fn unified_poll_rejects_pid_from_wrong_observed_incarnation() {
    let observed = [(1, 101_u32), (2, 202_u32)];
    assert!(pid_matches_observed_incarnation(202, 2, &observed));
    assert!(!pid_matches_observed_incarnation(101, 2, &observed));
}

#[cfg(windows)]
fn hold_recovery_result(runtime: &mut ImportRuntime) {
    let executor = runtime.unified_executor.as_mut().unwrap();
    let result = executor
        .results
        .recv_timeout(Duration::from_secs(20))
        .unwrap();
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    assert!(send.send(result).is_ok());
    executor.results = receive;
}

#[cfg(windows)]
fn wait_recovery_ready(runtime: &mut ImportRuntime, deadline: Instant) {
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
}

#[cfg(windows)]
fn check_recovery_wire_boundary(wire: &str, expected_lines: usize) -> Result<(), String> {
    let actual = wire.lines().count();
    if actual != expected_lines {
        return Err(format!(
            "wrong wire boundary: expected {expected_lines}, got {actual}"
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn assert_recovery_wire_boundary(worker: &Path, expected_lines: usize) -> Result<(), String> {
    let wire = match std::fs::read_to_string(worker.with_extension("wire")) {
        Ok(wire) => wire,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error.to_string()),
    };
    check_recovery_wire_boundary(&wire, expected_lines)
}

#[cfg(windows)]
#[test]
fn unified_poll_recovery_boundaries_reject_early_ready_and_dispatch() {
    assert!(check_recovery_wire_boundary("", 0).is_ok());
    assert!(check_recovery_wire_boundary("OUT ready\n", 0).is_err());
    assert!(check_recovery_wire_boundary("OUT ready\n", 1).is_ok());
    assert!(check_recovery_wire_boundary("OUT ready\nIN request\n", 1).is_err());
    assert!(check_recovery_wire_boundary("OUT ready\nIN request\nOUT ready\n", 2).is_err());
}

#[cfg(windows)]
fn release_recovery_dispatch(
    worker: &Path,
    release: &std::sync::mpsc::Sender<()>,
    expected_lines: usize,
) {
    assert_recovery_wire_boundary(worker, expected_lines).unwrap();
    release.send(()).unwrap();
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
    observed: &mut LivePollTrace,
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
    assert!(observed.poll(runtime).is_none());
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
    assert!(observed.poll(runtime).is_none());
    trace.push(format!("committed {}", unified_fallback_snapshot(runtime)));
    assert!(observed.poll(runtime).is_none());
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
fn verified_personal_model() -> String {
    const SHA: &str = "e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e";
    let model = PathBuf::from(
        std::env::var_os("N1_TEST_PERSONAL_PARAKEET_MODEL")
            .expect("installed model: pass -PersonalParakeetModelPath to guard-level1"),
    );
    verify_selected_model(&model, SHA)
        .unwrap_or_else(|error| panic!("installed model verification failed: {error}"));
    model.to_string_lossy().replace('\\', "/")
}

#[cfg(windows)]
fn verify_selected_model(model: &Path, expected_sha: &str) -> Result<(), String> {
    if !model.is_absolute() {
        return Err(format!("model path is not absolute: {}", model.display()));
    }
    if model.file_name().and_then(|name| name.to_str()) != Some("parakeet-tdt-0.6b-v3.q8_0.gguf") {
        return Err(format!("unexpected model name: {}", model.display()));
    }
    let actual = crate::parakeet_import::sha256_file(model)
        .map_err(|error| format!("cannot read {}: {error}", model.display()))?;
    if !actual.eq_ignore_ascii_case(expected_sha) {
        return Err(format!(
            "SHA-256 mismatch for {}: {actual}",
            model.display()
        ));
    }
    Ok(())
}

#[cfg(windows)]
fn portable_model_trace(bytes: &[u8], expected_path: &str) -> Result<Vec<u8>, String> {
    let mut lines: Vec<String> =
        serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    if serde_json::to_vec_pretty(&lines).map_err(|error| error.to_string())? != bytes {
        return Err("noncanonical trace encoding".into());
    }
    let field = format!(
        "\"model_path\":{}",
        serde_json::to_string(expected_path).map_err(|error| error.to_string())?
    );
    let marker = "\"model_path\":\"<SELECTED_MODEL>\"";
    let mut count = 0;
    for line in &mut lines {
        let keys = line.matches("\"model_path\":").count();
        let matches = line.matches(&field).count();
        if keys != matches {
            return Err("trace contains an unexpected model path".into());
        }
        count += matches;
        *line = line.replace(&field, marker);
    }
    if count == 0 {
        return Err("trace contains no selected model path".into());
    }
    serde_json::to_vec_pretty(&lines).map_err(|error| error.to_string())
}

#[cfg(windows)]
fn assert_portable_model_trace(actual: &[u8], frozen: &[u8], selected: &str) {
    const BASELINE: &str =
        "C:/Users/cyril/AppData/Local/Whisper/models/parakeet-tdt-0.6b-v3.q8_0.gguf";
    let actual = portable_model_trace(actual, selected).expect("candidate model path mismatch");
    let baseline = portable_model_trace(frozen, BASELINE).expect("baseline model path mismatch");
    assert_eq!(
        actual, baseline,
        "portable trace differs beyond selected model path"
    );
}

#[cfg(windows)]
#[test]
fn unified_poll_model_selection_and_portable_trace_reject_wrong_inputs() {
    let root = test_artifact_root("unified-poll-portable-model");
    let isolated_local = root.join("isolated-localappdata");
    let profile = root.join("another-account");
    let path = root.join("redirected-data/Whisper/models/parakeet-tdt-0.6b-v3.q8_0.gguf");
    assert!(!path.starts_with(&profile));
    assert!(!path.starts_with(&isolated_local));
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    assert!(verify_selected_model(&path, "00").is_err());
    std::fs::write(&path, b"synthetic model").unwrap();
    let sha = crate::parakeet_import::sha256_file(&path).unwrap();
    assert!(verify_selected_model(&path, &sha).is_ok());
    assert!(verify_selected_model(&path, &"0".repeat(64)).is_err());
    let wrong = path.with_file_name("wrong.gguf");
    std::fs::write(&wrong, b"synthetic model").unwrap();
    assert!(verify_selected_model(&wrong, &sha).is_err());
    let selected = "C:/Users/other/AppData/Local/Whisper/models/parakeet-tdt-0.6b-v3.q8_0.gguf";
    let baseline = "C:/Users/cyril/AppData/Local/Whisper/models/parakeet-tdt-0.6b-v3.q8_0.gguf";
    let candidate = serde_json::to_vec_pretty(&vec![format!(
        "stop event {{\"model_path\":\"{selected}\",\"generation\":1}}"
    )])
    .unwrap();
    let reference = serde_json::to_vec_pretty(&vec![format!(
        "stop event {{\"model_path\":\"{baseline}\",\"generation\":1}}"
    )])
    .unwrap();
    assert_portable_model_trace(&candidate, &reference, selected);
    assert!(portable_model_trace(&candidate, baseline).is_err());
    let different = serde_json::to_vec_pretty(&vec![format!(
        "stop event {{\"model_path\":\"{selected}\",\"generation\":2}}"
    )])
    .unwrap();
    assert_ne!(
        portable_model_trace(&candidate, selected).unwrap(),
        portable_model_trace(&different, selected).unwrap()
    );
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
    observed: &mut LivePollTrace,
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
        assert!(observed.poll(runtime).is_none());
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
    effects: &mut LiveEffectTrace,
) {
    assert!(record_unified_poll(runtime, effects).is_none());
    trace.push(format!(
        "final dispatched {}",
        unified_fallback_snapshot(runtime)
    ));
    let command = calls.recv_timeout(Duration::from_secs(10)).unwrap();
    effects
        .joined
        .push(format!("fixture received command {command}"));
    trace.push(format!("worker command {command}"));
    release.send(()).unwrap();
    effects.joined.push("fixture released worker".into());
    let executor = runtime.unified_executor.as_mut().unwrap();
    let result = executor
        .results
        .recv_timeout(Duration::from_secs(10))
        .unwrap();
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    assert!(send.send(result).is_ok());
    executor.results = receive;
    effects.joined.push("fixture returned worker result".into());
    assert!(record_unified_poll(runtime, effects).is_none());
    trace.push(format!(
        "final committed {}",
        unified_fallback_snapshot(runtime)
    ));
    assert!(record_unified_poll(runtime, effects).is_none());
    trace.push(format!("terminal {}", unified_fallback_snapshot(runtime)));
    while let Some(event) = runtime.poll_unified_document() {
        let event = format!(
            "event {}",
            serde_json::to_string(&event.unwrap())
                .unwrap()
                .replace(&*root.to_string_lossy().replace('\\', "\\\\"), "<ROOT>")
        );
        effects.joined.push(event.clone());
        trace.push(event);
    }
}

#[cfg(windows)]
fn finish_real_eof_peer(
    runtime: &mut ImportRuntime,
    worker: &Path,
    local: &Path,
    peer_id: whisper_core::parakeet_scheduler::SchedulerSession,
    deadline: Instant,
    observers: (
        &mut usize,
        &mut FallbackPollTrace,
        &std::sync::mpsc::Sender<()>,
    ),
    trace: &mut Vec<String>,
) -> u64 {
    let (wire_cursor, observed, dispatch_release) = observers;
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
    let ready = observed.poll(runtime).unwrap().unwrap();
    trace.push(format!("event {}", serde_json::to_string(&ready).unwrap()));
    assert!(observed.poll(runtime).is_none());
    trace.push(format!(
        "peer dispatched {}",
        unified_fallback_snapshot(runtime)
    ));
    release_recovery_dispatch(worker, dispatch_release, 3);
    let executor = runtime.unified_executor.as_mut().unwrap();
    let result = executor
        .results
        .recv_timeout(Duration::from_secs(20))
        .unwrap();
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    assert!(send.send(result).is_ok());
    executor.results = receive;
    append_recovery_wire(worker, local, wire_cursor, trace);
    assert!(observed.poll(runtime).is_none());
    trace.push(format!(
        "peer committed {}",
        unified_fallback_snapshot(runtime)
    ));
    assert!(observed.poll(runtime).is_none());
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
    let observer = Arc::new(Mutex::new(Vec::new()));
    let mut executor =
        UnifiedInferenceExecutor::start_observed(Box::new(worker), Arc::clone(&observer)).unwrap();
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
    let successor_gate = runtime.hold_next_child_ready();
    assert!(runtime.poll_unified_sessions().is_none());
    assert!(runtime.controlled_ready_gate.is_none());
    let transferred_gate = runtime
        .unified_executor
        .as_ref()
        .unwrap()
        .ready_gate
        .as_ref()
        .expect("successor ready gate transferred");
    assert!(Arc::ptr_eq(transferred_gate, &successor_gate));
    assert!(!runtime.unified_sessions.contains_key(&first_id));
    assert!(runtime.unified_sessions.contains_key(&peer_id));
    assert_eq!(runtime.parakeet_scheduler.active_sessions(), 1);
    assert!(runtime.unified_executor.as_ref().unwrap().thread.is_some());
    check_successor_ready_gate(&mut runtime, &observer, &successor_gate, deadline);
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

fn check_successor_ready_gate(
    runtime: &mut ImportRuntime,
    observer: &Arc<Mutex<Vec<ControlledChildObservation>>>,
    gate: &Arc<(Mutex<bool>, std::sync::Condvar)>,
    deadline: Instant,
) {
    let successor = loop {
        let observations = observer.lock().unwrap();
        if let Some(spawned) = observations
            .iter()
            .filter(|entry| entry.event == "Spawn")
            .nth(1)
        {
            assert!(!observations
                .iter()
                .any(|entry| entry.incarnation == spawned.incarnation && entry.event == "Ready"));
            break spawned.incarnation;
        }
        drop(observations);
        assert!(Instant::now() < deadline, "successor did not spawn");
        thread::yield_now();
    };
    assert!(
        !runtime
            .unified_executor
            .as_mut()
            .unwrap()
            .is_ready()
            .unwrap()
    );
    let (released, changed) = &**gate;
    *released.lock().unwrap() = true;
    changed.notify_all();
    while !runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .is_ready()
        .unwrap()
    {
        assert!(
            Instant::now() < deadline,
            "successor did not become ready after release"
        );
        thread::yield_now();
    }
    assert!(
        observer
            .lock()
            .unwrap()
            .iter()
            .any(|entry| entry.incarnation == successor && entry.event == "Ready")
    );
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
