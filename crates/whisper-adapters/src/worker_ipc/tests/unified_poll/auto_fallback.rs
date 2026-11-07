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
struct FallbackPollTrace {
    wire: PathBuf,
    document: Option<PathBuf>,
    peer_document: Option<PathBuf>,
    separate_document_files: bool,
    file_entries: Vec<String>,
    escaped_root: String,
    cursor: usize,
    entries: Vec<String>,
    joined: Vec<String>,
}

#[cfg(windows)]
impl FallbackPollTrace {
    fn new(worker: &Path, local: &Path) -> Self {
        Self {
            wire: worker.with_extension("wire"),
            document: None,
            peer_document: None,
            separate_document_files: false,
            file_entries: Vec::new(),
            escaped_root: local.to_string_lossy().replace('\\', "\\\\"),
            cursor: 0,
            entries: Vec::new(),
            joined: Vec::new(),
        }
    }

    fn drain_wire(&mut self, phase: &str) {
        let wire = match std::fs::read_to_string(&self.wire) {
            Ok(wire) => wire,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return,
            Err(error) => panic!("wire read failed: {error}"),
        };
        let lines: Vec<_> = wire.lines().collect();
        assert!(lines.len() >= self.cursor);
        for line in &lines[self.cursor..] {
            self.entries.push(format!(
                "{phase} {}",
                line.replace(&self.escaped_root, "<ROOT>")
            ));
        }
        self.cursor = lines.len();
    }

    fn poll(
        &mut self,
        runtime: &mut ImportRuntime,
    ) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
        self.poll_via(runtime, false)
    }

    fn poll_routed(
        &mut self,
        runtime: &mut ImportRuntime,
    ) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
        self.poll_via(runtime, true)
    }

    fn poll_via(
        &mut self,
        runtime: &mut ImportRuntime,
        routed: bool,
    ) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
        let offset = self.entries.len();
        self.drain_wire("between polls");
        let wire_before = self.entries[offset..].to_vec();
        let before = unified_fallback_snapshot(runtime);
        let files_before = self
            .document
            .as_ref()
            .map(|path| document_file_snapshot(path));
        let peer_before = self
            .peer_document
            .as_ref()
            .map(|path| document_file_snapshot(path));
        let result = if routed {
            runtime.poll()
        } else {
            runtime.poll_unified_sessions()
        };
        let after = unified_fallback_snapshot(runtime);
        let files_after = self
            .document
            .as_ref()
            .map(|path| document_file_snapshot(path));
        let peer_after = self
            .peer_document
            .as_ref()
            .map(|path| document_file_snapshot(path));
        let offset = self.entries.len();
        self.drain_wire("during poll");
        let wire_after = self.entries.split_off(offset);
        if before != after
            || result.is_some()
            || files_before != files_after
            || peer_before != peer_after
            || !wire_before.is_empty()
            || !wire_after.is_empty()
        {
            self.joined.push(
                serde_json::to_string(&(
                    &before,
                    &result,
                    &after,
                    &files_before,
                    &files_after,
                    &peer_before,
                    &peer_after,
                    &wire_before,
                    &wire_after,
                ))
                .unwrap(),
            );
        }
        if before != after || result.is_some() {
            self.entries
                .push(serde_json::to_string(&(before, &result, after)).unwrap());
        }
        if files_before != files_after {
            let line =
                serde_json::to_string(&("document files", files_before, files_after)).unwrap();
            if self.separate_document_files {
                self.file_entries.push(line);
            } else {
                self.entries.push(line);
            }
        }
        if peer_before != peer_after {
            self.entries.push(
                serde_json::to_string(&("peer document files", peer_before, peer_after)).unwrap(),
            );
        }
        self.entries.extend(wire_after);
        result
    }
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
    let complete_wire =
        std::fs::read(local.join("fallback-complete-wire.json")).expect("complete CPU IPC log");
    assert_eq!(
        complete_wire.as_slice(),
        include_bytes!("reference_auto_fallback_complete_wire.json")
    );
    let transitions = std::fs::read(local.join("fallback-poll-transitions.json"))
        .expect("fallback poll transitions");
    assert_eq!(
        transitions.as_slice(),
        include_bytes!("reference_auto_poll_transitions.json")
    );
    let document_deltas = std::fs::read(local.join("fallback-document-deltas.json"))
        .expect("fallback document deltas");
    let (observed_pid, segment_pid): (u32, u64) =
        serde_json::from_slice(&std::fs::read(local.join("fallback-id-map.json")).unwrap())
            .unwrap();
    assert_eq!(u64::from(observed_pid), segment_pid);
    assert_portable_index_trace(
        &document_deltas,
        include_bytes!("reference_auto_document_deltas.json"),
        segment_pid,
        "<PID1>",
    );
    let joined = std::fs::read(local.join("fallback-joined.json")).expect("joined fallback trace");
    assert_portable_index_trace(
        &joined,
        include_bytes!("reference_auto_joined_poll_effects.json"),
        segment_pid,
        "<PID1>",
    );
}

#[cfg(windows)]
fn run_unified_poll_auto_fallback_child() {
    const MODEL_SHA: &str = "e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e";
    let model = verified_personal_model();
    let local = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap());
    let (cpu_paths, worker) =
        prepare_fake_personal_worker(&local, &model, MODEL_SHA, "fake_worker.rs");
    let source = local.join("source.wav");
    let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source).unwrap();
    writer.append(&vec![1000_i16; 3 * 16_000]).unwrap();
    writer.finish().unwrap();
    let mut auto = unified_import_request(&local.join("auto"), &source, 73_010);
    auto.config.compute = ComputeChoice::Auto;
    let mut manual = unified_import_request(&local.join("manual"), &source, 73_011);
    manual.config.compute = ComputeChoice::Gpu;
    let mut poll_trace = FallbackPollTrace::new(&worker, &local);
    poll_trace.document = Some(trace_document_path(
        Path::new(&auto.destination),
        whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: auto.job_id,
            generation: auto.generation,
        },
    ));
    poll_trace.separate_document_files = true;
    let (mut runtime, session, mut trace) =
        begin_gpu_fallback(&local, &auto, &manual, cpu_paths, false, &mut poll_trace);
    complete_cpu_fallback_call(
        &mut runtime,
        &worker,
        &local,
        session,
        &mut trace,
        &mut poll_trace,
    );
    let observed_pid = observe_child_pid(&worker);
    poll_trace
        .joined
        .push("fixture observed child identity".into());
    trace.push("CPU response received".into());
    assert!(poll_trace.poll(&mut runtime).is_none());
    assert!(runtime.unified_sessions[&session].pending_call.is_none());
    trace.push(format!(
        "after commit {}",
        unified_fallback_snapshot(&runtime)
    ));
    assert!(poll_trace.poll(&mut runtime).is_none());
    assert!(!runtime.unified_sessions.contains_key(&session));
    assert_empty_diagnostics_after_join(&mut runtime, &local);
    poll_trace.drain_wire("after reap");
    std::fs::write(
        local.join("fallback-poll-transitions.json"),
        serde_json::to_vec_pretty(&poll_trace.entries).unwrap(),
    )
    .unwrap();
    trace.push(format!(
        "after terminal {}",
        unified_fallback_snapshot(&runtime)
    ));
    record_complete_fallback_wire(&runtime, &worker, &local);
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
        let event = format!("event {}", serde_json::to_string(&value).unwrap());
        poll_trace.joined.push(event.clone());
        trace.push(event);
    }
    trace.push(format!(
        "terminal sessions={} active={} drains={} pending={}",
        runtime.unified_sessions.len(),
        runtime.parakeet_scheduler.active_sessions(),
        runtime.document_drains.len(),
        runtime.parakeet_pending_events.len()
    ));
    assert_eq!(
        crate::parakeet_import::sha256_file(Path::new(&model)).unwrap(),
        MODEL_SHA
    );
    let segment_pid = pid.expect("worker PID was recorded");
    assert!(pid_matches_observed_incarnation(
        segment_pid,
        1,
        &[(1, observed_pid)]
    ));
    write_fallback_document_deltas(&mut poll_trace, &local, observed_pid);
    write_fallback_identity_and_trace(&local, observed_pid, segment_pid, &trace);
}

#[cfg(windows)]
fn write_fallback_identity_and_trace(
    local: &Path,
    observed_pid: u32,
    segment_pid: u64,
    trace: &[String],
) {
    std::fs::write(
        local.join("fallback-id-map.json"),
        serde_json::to_vec(&(observed_pid, segment_pid)).unwrap(),
    )
    .unwrap();
    std::fs::write(
        local.join("fallback-trace.json"),
        serde_json::to_vec_pretty(trace).unwrap(),
    )
    .unwrap();
}

#[cfg(windows)]
fn write_fallback_document_deltas(trace: &mut FallbackPollTrace, local: &Path, pid: u32) {
    std::fs::write(
        local.join("fallback-document-deltas-raw.json"),
        serde_json::to_vec_pretty(&trace.file_entries).unwrap(),
    )
    .unwrap();
    normalize_document_pid(
        &mut trace.file_entries,
        pid.into(),
        "document files",
        "<PID1>",
    );
    std::fs::write(
        local.join("fallback-document-deltas.json"),
        serde_json::to_vec_pretty(&trace.file_entries).unwrap(),
    )
    .unwrap();
    let mut joined = Vec::new();
    for entry in &trace.joined {
        if entry.starts_with('[') {
            joined.push(entry.clone());
        }
    }
    std::fs::write(
        local.join("fallback-joined-raw.json"),
        serde_json::to_vec_pretty(&trace.joined).unwrap(),
    )
    .unwrap();
    normalize_joined_pid(&mut joined, pid.into(), [3, 4], "<PID1>");
    let mut normalized = joined.into_iter();
    let complete: Vec<_> = trace
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
        local.join("fallback-joined.json"),
        serde_json::to_vec_pretty(&complete).unwrap(),
    )
    .unwrap();
}

#[cfg(windows)]
fn record_complete_fallback_wire(runtime: &ImportRuntime, worker: &Path, local: &Path) {
    let wire =
        std::fs::read_to_string(worker.with_extension("wire")).expect("complete CPU IPC log");
    let escaped_root = local.to_string_lossy().replace('\\', "\\\\");
    let full_wire: Vec<_> = wire
        .lines()
        .map(|line| line.replace(&escaped_root, "<ROOT>"))
        .collect();
    assert!(
        runtime.unified_executor.as_ref().unwrap().thread.is_none(),
        "worker must be reaped before final IPC read"
    );
    assert_eq!(
        full_wire.len(),
        3,
        "READY, request and result are the complete wire: termination uses kill, not shutdown"
    );
    std::fs::write(
        local.join("fallback-complete-wire.json"),
        serde_json::to_vec_pretty(&full_wire).unwrap(),
    )
    .unwrap();
}

#[cfg(windows)]
fn observe_child_pid(executable: &Path) -> u32 {
    #[derive(serde::Deserialize)]
    struct ProcessRow {
        #[serde(rename = "ProcessId")]
        process_id: u32,
        #[serde(rename = "ExecutablePath")]
        executable_path: Option<String>,
    }
    let query = format!(
        "SELECT ProcessId, ExecutablePath FROM Win32_Process WHERE ParentProcessId = {}",
        std::process::id()
    );
    let connection = wmi::WMIConnection::new().expect("WMI process observation unavailable");
    let rows: Vec<ProcessRow> = connection
        .raw_query(&query)
        .expect("WMI process observation failed");
    let expected = executable.to_string_lossy().replace('/', "\\");
    let matching: Vec<_> = rows
        .into_iter()
        .filter(|row| {
            row.executable_path
                .as_ref()
                .is_some_and(|path| path.eq_ignore_ascii_case(&expected))
        })
        .collect();
    assert_eq!(
        matching.len(),
        1,
        "exactly one direct child must match the isolated worker executable"
    );
    matching[0].process_id
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
    poll_trace: &mut FallbackPollTrace,
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
    runtime.start_diagnostics(local);
    assert!(runtime.diagnostics.is_some());
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
    assert!(poll_trace.poll(&mut runtime).is_none());
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
    poll_trace: &mut FallbackPollTrace,
) {
    std::fs::write(worker.with_extension("release"), b"release").unwrap();
    poll_trace.joined.push("fixture released CPU READY".into());
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
    poll_trace.joined.push("fixture observed CPU READY".into());
    let initial_wire = std::fs::read_to_string(worker.with_extension("wire")).unwrap();
    let initial_lines: Vec<_> = initial_wire.lines().collect();
    assert_eq!(
        initial_lines.len(),
        1,
        "only CPU READY precedes the parent READY"
    );
    trace.push(format!("wire {}", initial_lines[0]));
    let ready = poll_trace
        .poll(runtime)
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
    assert!(poll_trace.poll(runtime).is_none());
    assert!(runtime.unified_sessions[&session].pending_call.is_some());
    trace.push("dispatch accepted".into());
    trace.push(format!(
        "after dispatch {}",
        unified_fallback_snapshot(runtime)
    ));
    std::fs::write(worker.with_extension("request-release"), b"release").unwrap();
    poll_trace
        .joined
        .push("fixture released CPU request".into());
    let executor = runtime.unified_executor.as_mut().unwrap();
    let response = executor
        .results
        .recv_timeout(Duration::from_secs(20))
        .expect("CPU result");
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    assert!(send.send(response).is_ok());
    executor.results = receive;
    poll_trace.joined.push("fixture returned CPU result".into());
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
