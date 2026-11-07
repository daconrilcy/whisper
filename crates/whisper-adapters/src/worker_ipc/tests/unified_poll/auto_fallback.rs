fn unified_fallback_snapshot(runtime: &ImportRuntime) -> String {
    let mut sessions: Vec<_> = runtime.unified_sessions.iter().map(|(id, state)| {
        let prepared = state.pending_prepared.as_ref().map(|(call, window, pcm, final_window)| {
            let mut digest = Sha256::new();
            for sample in pcm { digest.update(sample.to_le_bytes()); }
            format!("call={call:?},window={window:?},pcm_len={},pcm_sha={:x},final={final_window}", pcm.len(), digest.finalize())
        });
        format!("id={id:?},compute={:?},available={},durable={},closed={},ready={},pending_call={:?},prepared={prepared:?},preview={},pending_window={:?},pending_available={:?},failure={:?},processed={},revision={}",
            state.native_route.as_ref().map(|route| route.compute), state.available_samples, state.durable_samples,
            state.source_closed, state.backend_ready_reported, state.pending_call, state.preview_sequence,
            state.pending_window, state.pending_available_samples, state.failure,
            state.finalizer.processed_samples(), state.document.revision())
    }).collect();
    sessions.sort();
    let executor = runtime.unified_executor.as_ref().map(|e| {
        (
            e.thread.is_some(),
            e.in_flight,
            e.ready_seen,
            e.worker.is_some(),
        )
    });
    let pending = runtime
        .parakeet_pending_events
        .iter()
        .map(|event| serde_json::to_string(event).unwrap())
        .collect::<Vec<_>>();
    format!(
        "sessions={sessions:?}; active={}; call={:?}; executor={executor:?}; archives={}; gates={}; drains={}; pending={pending:?}",
        runtime.parakeet_scheduler.active_sessions(),
        runtime.parakeet_scheduler.in_flight(),
        runtime.unified_archives.len(),
        runtime.unified_publication_gates.len(),
        runtime.document_drains.len()
    )
}

#[cfg(windows)]
#[test]
fn unified_poll_auto_fallback_with_manual_peer_uses_verified_personal_model() {
    use std::os::windows::process::CommandExt;

    const CHILD: &str = "UNIFIED_POLL_FALLBACK_CHILD";
    if std::env::var_os(CHILD).is_some() {
        run_unified_poll_auto_fallback_child();
        return;
    }
    let root = test_artifact_root("unified-poll-auto-fallback");
    let local = root.join("localappdata");
    std::fs::create_dir_all(&local).unwrap();
    let result = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "worker_ipc::tests::unified_poll_auto_fallback_with_manual_peer_uses_verified_personal_model", "--nocapture"])
        .env(CHILD, "1")
        .env("LOCALAPPDATA", &local)
        .creation_flags(0x0800_0000)
        .output()
        .expect("start isolated fallback scenario");
    assert!(
        result.status.success(),
        "isolated fallback scenario failed: {}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    let trace = std::fs::read(local.join("fallback-trace.json")).expect("scenario trace");
    assert_eq!(
        trace.as_slice(),
        include_bytes!("reference_auto_fallback.json"),
        "fallback effects diverged from the frozen baseline"
    );
    assert!(
        local.join("fallback-id-map.json").is_file(),
        "PID mapping was not retained"
    );
}

#[cfg(windows)]
fn run_unified_poll_auto_fallback_child() {
    const MODEL: &str =
        "C:/Users/cyril/AppData/Local/Whisper/models/parakeet-tdt-0.6b-v3.q8_0.gguf";
    const MODEL_SHA: &str = "e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e";
    let local = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap());
    let (cpu_paths, worker) =
        prepare_fake_personal_worker(&local, MODEL, MODEL_SHA, "fake_worker.rs");
    let source = local.join("source.wav");
    let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source).unwrap();
    writer.append(&vec![1000_i16; 3 * 16_000]).unwrap();
    writer.finish().unwrap();
    let mut auto = unified_import_request(&local.join("auto"), &source, 73_010);
    auto.config.compute = ComputeChoice::Auto;
    let mut manual = unified_import_request(&local.join("manual"), &source, 73_011);
    manual.config.compute = ComputeChoice::Gpu;
    let (mut runtime, session, mut trace) =
        begin_gpu_fallback(&local, &auto, &manual, cpu_paths, false);
    complete_cpu_fallback_call(&mut runtime, &worker, &local, session, &mut trace);
    trace.push("CPU response received".into());
    assert!(runtime.poll_unified_sessions().is_none());
    assert!(runtime.unified_sessions[&session].pending_call.is_none());
    trace.push(format!(
        "after commit {}",
        unified_fallback_snapshot(&runtime)
    ));
    assert!(runtime.poll_unified_sessions().is_none());
    assert!(!runtime.unified_sessions.contains_key(&session));
    trace.push(format!(
        "after terminal {}",
        unified_fallback_snapshot(&runtime)
    ));
    let mut pid = None;
    while let Some(event) = runtime.poll_unified_document() {
        let event = event.expect("document drain");
        let mut value = serde_json::to_value(&event).unwrap();
        if let ImportEvent::Segment(segment) = &event {
            if let Some(previous) = pid {
                assert_eq!(previous, segment.instance_id);
            }
            pid = Some(segment.instance_id);
            value["Segment"]["instance_id"] = serde_json::Value::String("<PID1>".into());
        }
        trace.push(format!("event {}", serde_json::to_string(&value).unwrap()));
    }
    trace.push(format!(
        "terminal sessions={} active={} drains={} pending={}",
        runtime.unified_sessions.len(),
        runtime.parakeet_scheduler.active_sessions(),
        runtime.document_drains.len(),
        runtime.parakeet_pending_events.len()
    ));
    assert_eq!(
        crate::parakeet_import::sha256_file(Path::new(MODEL)).unwrap(),
        MODEL_SHA
    );
    std::fs::write(
        local.join("fallback-id-map.json"),
        serde_json::to_vec(&pid.expect("worker PID was recorded")).unwrap(),
    )
    .unwrap();
    std::fs::write(
        local.join("fallback-trace.json"),
        serde_json::to_vec_pretty(&trace).unwrap(),
    )
    .unwrap();
}

#[cfg(windows)]
fn prepare_fake_personal_worker(
    local: &Path,
    model: &str,
    model_sha: &str,
    fake_name: &str,
) -> (crate::parakeet_import::ParakeetWorkerPaths, PathBuf) {
    use std::os::windows::process::CommandExt;
    let root = local.join("Whisper");
    let worker_dir = root.join("worker");
    let native_dir = root.join("native/cpu");
    let models_dir = root.join("models");
    for path in [&worker_dir, &native_dir, &models_dir] {
        std::fs::create_dir_all(path).unwrap();
    }
    assert_eq!(
        crate::parakeet_import::sha256_file(Path::new(model)).unwrap(),
        model_sha
    );
    let model_copy = models_dir.join("parakeet-tdt-0.6b-v3.q8_0.gguf");
    assert_eq!(std::fs::copy(model, &model_copy).unwrap(), 713_975_456);
    assert_eq!(
        crate::parakeet_import::sha256_file(&model_copy).unwrap(),
        model_sha
    );
    let worker = worker_dir.join("whisper-worker-parakeet-cpu.exe");
    let fake_source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("src/worker_ipc/tests/unified_poll/{fake_name}"));
    let compilation = Command::new(std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into()))
        .args(["--edition=2024", "-Cdebuginfo=0", "-C", "link-arg=/Brepro"])
        .arg(fake_source)
        .arg("-o")
        .arg(&worker)
        .creation_flags(0x0800_0000)
        .output()
        .unwrap();
    assert!(
        compilation.status.success(),
        "fake worker build failed: {}",
        String::from_utf8_lossy(&compilation.stderr)
    );
    for name in [
        "nemo_speech_asr_c.dll",
        "nemo_speech_asr.dll",
        "ggml.dll",
        "ggml-base.dll",
        "ggml-cpu.dll",
    ] {
        std::fs::write(native_dir.join(name), format!("fixture:{name}")).unwrap();
    }
    let cpu_paths = crate::parakeet_import::ParakeetWorkerPaths {
        executable: worker.clone(),
        native_dir,
        model: model_copy,
        runtime_root: root.clone(),
    };
    let key = cpu_paths
        .qualification_key(whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS)
        .unwrap();
    let identity = whisper_core::parakeet_ipc::WorkerIdentity {
        model_sha256: model_sha.into(),
        runtime_sha256: key.runtime_build,
        build_sha256: key.worker_build,
        profile: "import-single-pass".into(),
    };
    std::fs::write(
        worker.with_extension("identity"),
        serde_json::to_vec(&identity).unwrap(),
    )
    .unwrap();
    (cpu_paths, worker)
}

#[cfg(windows)]
fn begin_gpu_fallback(
    local: &Path,
    auto: &ImportRequest,
    manual: &ImportRequest,
    cpu_paths: crate::parakeet_import::ParakeetWorkerPaths,
    preview: bool,
) -> (
    ImportRuntime,
    whisper_core::parakeet_scheduler::SchedulerSession,
    Vec<String>,
) {
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: auto.job_id,
        generation: auto.generation,
    };
    let peer = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: manual.job_id,
        generation: manual.generation,
    };
    let starts = Arc::new(AtomicU64::new(0));
    let stops = Arc::new(AtomicU64::new(0));
    let observer = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = ImportRuntime::new_with_workers_and_parakeet(
        WorkerPaths {
            cpu: PathBuf::from("unused-cpu"),
            gpu: PathBuf::from("unused-gpu"),
        },
        Arc::new(PublicationGate::default()),
        None,
        Some(cpu_paths),
        None,
    )
    .with_controlled_transcriber(Box::new(StartFailsOnceUnifiedWorker {
        starts: Arc::clone(&starts),
        stops: Arc::clone(&stops),
    }));
    let mut gpu_route = controlled_native_route(local);
    gpu_route.compute = ComputeChoice::Gpu;
    gpu_route.requested_compute = ComputeChoice::Auto;
    let mut peer_route = gpu_route.clone();
    peer_route.requested_compute = ComputeChoice::Gpu;
    runtime
        .native_unified_routes
        .insert(session, gpu_route.clone());
    runtime.start_controlled_unified_import(auto).unwrap();
    if preview {
        runtime
            .unified_sessions
            .get_mut(&session)
            .unwrap()
            .source_closed = false;
        runtime
            .parakeet_scheduler
            .queue_preview(session, 64_000)
            .unwrap();
    }
    runtime.native_unified_routes.insert(peer, peer_route);
    runtime.start_controlled_unified_import(manual).unwrap();
    runtime.native_executor_route = Some(gpu_route);
    let transcriber = runtime.controlled_transcriber.take().unwrap();
    let executor =
        UnifiedInferenceExecutor::start_observed(transcriber, Arc::clone(&observer)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !executor.thread.as_ref().unwrap().is_finished() {
        assert!(
            Instant::now() < deadline,
            "GPU fixture did not fail startup"
        );
        thread::yield_now();
    }
    runtime.unified_executor = Some(executor);
    let mut trace = Vec::<String>::new();
    trace.push(format!("admitted {}", unified_fallback_snapshot(&runtime)));
    assert!(runtime.poll_unified_sessions().is_none());
    let retained = runtime.unified_sessions.get(&session).unwrap();
    assert!(retained.pending_prepared.is_some());
    assert_eq!(
        retained.native_route.as_ref().unwrap().compute,
        ComputeChoice::Cpu
    );
    assert!(!runtime.unified_sessions.contains_key(&peer));
    assert!(!runtime.unified_archives.contains_key(&peer));
    trace.push(format!(
        "after fallback {}",
        unified_fallback_snapshot(&runtime)
    ));
    let failed = runtime
        .parakeet_pending_events
        .pop_front()
        .expect("manual peer failure");
    assert!(
        matches!(&failed, ImportEvent::Failed { job_id, generation, message } if *job_id == peer.job_id && *generation == peer.generation && message.contains("manual GPU sessions do not fall back to CPU"))
    );
    trace.push(format!(
        "fallback starts={} stops={} active={} sessions={} archives={} peer_event={}",
        starts.load(Ordering::Relaxed),
        stops.load(Ordering::Relaxed),
        runtime.parakeet_scheduler.active_sessions(),
        runtime.unified_sessions.len(),
        runtime.unified_archives.len(),
        serde_json::to_string(&failed).unwrap()
    ));
    assert!(runtime.parakeet_pending_events.is_empty());
    (runtime, session, trace)
}

#[cfg(windows)]
fn complete_cpu_fallback_call(
    runtime: &mut ImportRuntime,
    worker: &Path,
    local: &Path,
    session: whisper_core::parakeet_scheduler::SchedulerSession,
    trace: &mut Vec<String>,
) {
    std::fs::write(worker.with_extension("release"), b"release").unwrap();
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        match runtime.unified_executor.as_mut().unwrap().is_ready() {
            Ok(true) => break,
            Ok(false) => {
                assert!(Instant::now() < deadline, "CPU fixture READY timed out");
                thread::yield_now();
            }
            Err(error) => panic!("CPU fixture failed to become READY: {error}"),
        }
    }
    let initial_wire = std::fs::read_to_string(worker.with_extension("wire")).unwrap();
    let initial_lines: Vec<_> = initial_wire.lines().collect();
    assert_eq!(
        initial_lines.len(),
        1,
        "only CPU READY precedes the parent READY"
    );
    trace.push(format!("wire {}", initial_lines[0]));
    let ready = runtime
        .poll_unified_sessions()
        .expect("CPU READY")
        .expect("successful CPU READY");
    assert!(
        matches!(&ready, ImportEvent::Ready { job_id, generation, backend } if *job_id == session.job_id && *generation == session.generation && backend.contains("CPU (Auto fallback before audio:"))
    );
    trace.push(format!("event {}", serde_json::to_string(&ready).unwrap()));
    trace.push(format!(
        "after ready {}",
        unified_fallback_snapshot(runtime)
    ));
    assert!(runtime.poll_unified_sessions().is_none());
    assert!(runtime.unified_sessions[&session].pending_call.is_some());
    trace.push("dispatch accepted".into());
    trace.push(format!(
        "after dispatch {}",
        unified_fallback_snapshot(runtime)
    ));
    std::fs::write(worker.with_extension("request-release"), b"release").unwrap();
    let executor = runtime.unified_executor.as_mut().unwrap();
    let response = executor
        .results
        .recv_timeout(Duration::from_secs(20))
        .expect("CPU result");
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    assert!(send.send(response).is_ok());
    executor.results = receive;
    let wire = std::fs::read_to_string(worker.with_extension("wire")).unwrap();
    let local_path = local.to_string_lossy().to_string().replace('\\', "\\\\");
    let lines: Vec<_> = wire.lines().collect();
    assert_eq!(
        lines.len(),
        3,
        "READY, Transcribe and Result must be recorded"
    );
    for line in &lines[1..] {
        trace.push(format!("wire {}", line.replace(&local_path, "<ROOT>")));
    }
}
