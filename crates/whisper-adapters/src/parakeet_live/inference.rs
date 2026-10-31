use super::*;

pub(super) fn expected_live_worker_backend(compute: ComputeChoice) -> &'static str {
    match compute {
        ComputeChoice::Cpu | ComputeChoice::Auto => "CPU",
        ComputeChoice::Gpu => "CUDA0",
    }
}

pub(super) struct InferenceThreadContext {
    pub(super) paths: ParakeetWorkerPaths,
    pub(super) request: LiveRequest,
    pub(super) candidate: Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
    pub(super) session_log_root: PathBuf,
}

pub(super) fn inference_thread(
    context: InferenceThreadContext,
    cancel: Arc<AtomicBool>,
    requests: Receiver<InferenceRequest>,
    events: SyncSender<InferenceEvent>,
    trace_enabled: bool,
    diagnostics: D2WorkerDiagnostics,
) {
    let InferenceThreadContext {
        paths,
        request,
        candidate,
        session_log_root,
    } = context;
    diagnostics.record(serde_json::json!({
        "event":"worker-configured",
        "executable":paths.executable.to_string_lossy().into_owned(),
        "model":paths.model.to_string_lossy().into_owned(),
        "native_dir":paths.native_dir.to_string_lossy().into_owned(),
        "runtime_root":paths.runtime_root.to_string_lossy().into_owned(),
        "compute":request.config.compute,
        "model_request_path":request.model_path.clone(),
        "session_log_root":session_log_root.to_string_lossy().into_owned(),
        "at_unix_ms":unix_time_ms(),
    }));
    let expected_gpu = match (request.config.compute, candidate.as_ref()) {
        (ComputeChoice::Gpu, Some(candidate)) => {
            match crate::parakeet_import::expected_gpu_for_candidate(candidate) {
                Ok(expected) => Some(expected),
                Err(error) => {
                    let _ = events.send(InferenceEvent::Unavailable(error));
                    return;
                }
            }
        }
        (ComputeChoice::Gpu, None) => {
            let _ = events.send(InferenceEvent::Unavailable(
                "Identité du GPU absente".into(),
            ));
            return;
        }
        _ => None,
    };
    let mut process = match ParakeetProcess::spawn_live(&paths, &session_log_root) {
        Ok(process) => {
            diagnostics.record(serde_json::json!({
                "event":"worker-spawned",
                "executable":paths.executable.to_string_lossy().into_owned(),
                "pid":process.pid(),
                "at_unix_ms":unix_time_ms(),
            }));
            process
        }
        Err(error) => {
            diagnostics.record(serde_json::json!({
                "event":"worker-spawn-failed",
                "executable":paths.executable.to_string_lossy().into_owned(),
                "error":error.clone(),
                "at_unix_ms":unix_time_ms(),
            }));
            let _ = events.send(InferenceEvent::Unavailable(error));
            return;
        }
    };
    let (identity, native_call_marker_required) = loop {
        if cancel.load(Ordering::Acquire) {
            let exit = process.terminate();
            diagnostics.record(serde_json::json!({
                "event":"worker-terminated-before-ready",
                "pid":process.pid(),
                "exit_status":exit.as_ref().map(|status| status.to_string()).map_err(ToString::to_string),
                "at_unix_ms":unix_time_ms(),
            }));
            let _ = events.send(InferenceEvent::Stopped);
            return;
        }
        match process.poll() {
            Ok(Some(WorkerEvent::Ready {
                identity,
                backend,
                gpu_evidence,
                ..
            })) => {
                diagnostics.record(serde_json::json!({
                    "event":"worker-ready",
                    "pid":process.pid(),
                    "backend":backend.clone(),
                    "worker_sha256":identity.build_sha256.clone(),
                    "profile":identity.profile.clone(),
                    "at_unix_ms":unix_time_ms(),
                }));
                if backend != expected_live_worker_backend(request.config.compute) {
                    let _ = events.send(InferenceEvent::Unavailable(format!(
                        "BackendMismatch: expected {}, got {backend}",
                        expected_live_worker_backend(request.config.compute)
                    )));
                    let _ = process.terminate();
                    return;
                }
                if request.config.compute == ComputeChoice::Gpu {
                    let Some(expected) = expected_gpu.as_ref() else {
                        let _ =
                            events.send(InferenceEvent::Unavailable("Identité GPU absente".into()));
                        let _ = process.terminate();
                        return;
                    };
                    if backend != "CUDA0"
                        || gpu_evidence.as_ref().is_none_or(|evidence| {
                            !crate::worker_ipc::gpu_evidence_matches(
                                Some(evidence),
                                Some(expected),
                                process.pid(),
                            )
                        })
                    {
                        let _ = events.send(InferenceEvent::Unavailable(
                            "Le worker Parakeet n'a pas attesté le GPU qualifié".into(),
                        ));
                        let _ = process.terminate();
                        return;
                    }
                }
                if let Err(error) = process.accept_ready(identity.clone()) {
                    let _ = events.send(InferenceEvent::Unavailable(error));
                    let _ = process.terminate();
                    return;
                }
                let native_call_marker_required = backend == "CUDA0";
                let _ = events.send(InferenceEvent::Ready { backend });
                break (identity, native_call_marker_required);
            }
            Ok(Some(WorkerEvent::Failed { message, .. })) => {
                diagnostics.record(serde_json::json!({
                    "event":"worker-failed-before-ready",
                    "pid":process.pid(),
                    "message":message.clone(),
                    "at_unix_ms":unix_time_ms(),
                }));
                let _ = events.send(InferenceEvent::Unavailable(message));
                let exit = process.terminate();
                diagnostics.record(serde_json::json!({
                    "event":"worker-terminated-after-ready-failure",
                    "pid":process.pid(),
                    "exit_status":exit.as_ref().map(|status| status.to_string()).map_err(ToString::to_string),
                    "at_unix_ms":unix_time_ms(),
                }));
                return;
            }
            Ok(Some(_)) | Ok(None) => thread::sleep(Duration::from_millis(5)),
            Err(error) => {
                diagnostics.record(serde_json::json!({
                    "event":"worker-poll-error-before-ready",
                    "pid":process.pid(),
                    "error":error.clone(),
                    "exit_status":process.try_wait().ok().flatten().map(|status| status.to_string()),
                    "at_unix_ms":unix_time_ms(),
                }));
                let _ = events.send(InferenceEvent::Unavailable(error));
                let exit = process.terminate();
                diagnostics.record(serde_json::json!({
                    "event":"worker-terminated-after-poll-error",
                    "pid":process.pid(),
                    "exit_status":exit.as_ref().map(|status| status.to_string()).map_err(ToString::to_string),
                    "at_unix_ms":unix_time_ms(),
                }));
                return;
            }
        }
        if requests
            .try_recv()
            .is_ok_and(|message| matches!(message, InferenceRequest::Stop))
        {
            let exit = process.terminate();
            diagnostics.record(serde_json::json!({
                "event":"worker-terminated-on-stop-before-ready",
                "pid":process.pid(),
                "exit_status":exit.as_ref().map(|status| status.to_string()).map_err(ToString::to_string),
                "at_unix_ms":unix_time_ms(),
            }));
            let _ = events.send(InferenceEvent::Stopped);
            return;
        }
    };
    let mut attempt = FIRST_ATTEMPT;
    loop {
        match requests.recv_timeout(Duration::from_millis(10)) {
            Ok(InferenceRequest::Stop) => {
                let shutdown = process.shutdown();
                diagnostics.record(serde_json::json!({
                    "event":"worker-shutdown-requested",
                    "pid":process.pid(),
                    "result":shutdown.as_ref().map(|_| "sent").map_err(ToString::to_string),
                    "at_unix_ms":unix_time_ms(),
                }));
                if diagnostics.enabled() {
                    let deadline = Instant::now() + Duration::from_secs(2);
                    loop {
                        match process.try_wait() {
                            Ok(Some(status)) => {
                                diagnostics.record(serde_json::json!({
                                    "event":"worker-exited-after-shutdown",
                                    "pid":process.pid(),
                                    "exit_status":status.to_string(),
                                    "at_unix_ms":unix_time_ms(),
                                }));
                                break;
                            }
                            Ok(None) if Instant::now() < deadline => {
                                thread::sleep(Duration::from_millis(10));
                            }
                            Ok(None) => {
                                diagnostics.record(serde_json::json!({
                                    "event":"worker-exit-status-pending-after-shutdown",
                                    "pid":process.pid(),
                                    "at_unix_ms":unix_time_ms(),
                                }));
                                break;
                            }
                            Err(error) => {
                                diagnostics.record(serde_json::json!({
                                    "event":"worker-exit-status-error",
                                    "pid":process.pid(),
                                    "error":error,
                                    "at_unix_ms":unix_time_ms(),
                                }));
                                break;
                            }
                        }
                    }
                }
                let _ = events.send(InferenceEvent::Stopped);
                return;
            }
            Ok(InferenceRequest::Transcribe {
                sequence,
                start_sample,
                samples,
                path,
                sha256,
                captured_at_unix_ms,
                recovery_for,
            }) => {
                let result = transcribe_window(
                    &mut process,
                    WindowRequestContext {
                        request: &request,
                        identity: &identity,
                        native_call_marker_required,
                        expected_gpu: expected_gpu.clone(),
                        attempt,
                        trace_enabled,
                    },
                    &cancel,
                    &path,
                    sha256,
                    samples.len() as u64,
                    request.group_offset_samples.saturating_add(start_sample),
                );
                attempt = attempt.saturating_add(1);
                let _ = fs::remove_file(&path);
                match result {
                    Ok(WindowTranscription::Usable {
                        text,
                        words,
                        native,
                    }) => {
                        diagnostics.record(serde_json::json!({
                            "event":"worker-window-result",
                            "pid":process.pid(),
                            "sequence":sequence,
                            "text_bytes":text.len(),
                            "word_count":words.len(),
                            "at_unix_ms":unix_time_ms(),
                        }));
                        let end_sample = start_sample.saturating_add(samples.len() as u64);
                        if events
                            .send(InferenceEvent::Result {
                                sequence,
                                start_sample,
                                end_sample,
                                text,
                                words,
                                captured_at_unix_ms,
                                available_at_unix_ms: unix_time_ms(),
                                native,
                                recovery_for,
                            })
                            .is_err()
                        {
                            let _ = process.terminate();
                            return;
                        }
                    }
                    Ok(WindowTranscription::Unusable { message, native }) => {
                        diagnostics.record(serde_json::json!({
                            "event":"worker-window-unusable",
                            "pid":process.pid(),
                            "sequence":sequence,
                            "message":message.clone(),
                            "at_unix_ms":unix_time_ms(),
                        }));
                        let end_sample = start_sample.saturating_add(samples.len() as u64);
                        if events
                            .send(InferenceEvent::WindowUnusable {
                                sequence,
                                start_sample,
                                end_sample,
                                message,
                                native,
                                recovery_for,
                            })
                            .is_err()
                        {
                            let _ = process.terminate();
                            return;
                        }
                    }
                    Err(error) => {
                        diagnostics.record(serde_json::json!({
                            "event":"worker-transcribe-error",
                            "pid":process.pid(),
                            "sequence":sequence,
                            "error":error.clone(),
                            "exit_status":process.try_wait().ok().flatten().map(|status| status.to_string()),
                            "at_unix_ms":unix_time_ms(),
                        }));
                        let _ = events.send(InferenceEvent::Unavailable(error));
                        let exit = process.terminate();
                        diagnostics.record(serde_json::json!({
                            "event":"worker-terminated-after-transcribe-error",
                            "pid":process.pid(),
                            "exit_status":exit.as_ref().map(|status| status.to_string()).map_err(ToString::to_string),
                            "at_unix_ms":unix_time_ms(),
                        }));
                        return;
                    }
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if let Ok(Some(status)) = process.try_wait() {
                    diagnostics.record(serde_json::json!({
                        "event":"worker-exited-idle",
                        "pid":process.pid(),
                        "exit_status":status.to_string(),
                        "at_unix_ms":unix_time_ms(),
                    }));
                    let _ = events.send(InferenceEvent::Unavailable(
                        "Le worker Parakeet s'est arrêté".into(),
                    ));
                    return;
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                let _ = process.terminate();
                return;
            }
        }
    }
}

struct WindowRequestContext<'a> {
    request: &'a LiveRequest,
    identity: &'a whisper_core::parakeet_ipc::WorkerIdentity,
    native_call_marker_required: bool,
    expected_gpu: Option<whisper_core::parakeet_ipc::ExpectedGpu>,
    attempt: u64,
    trace_enabled: bool,
}

fn transcribe_window(
    process: &mut ParakeetProcess,
    context: WindowRequestContext<'_>,
    cancel: &AtomicBool,
    path: &Path,
    sha256: String,
    expected_samples: u64,
    global_window_start_sample: u64,
) -> Result<WindowTranscription, String> {
    let WindowRequestContext {
        request,
        identity,
        native_call_marker_required,
        expected_gpu,
        attempt,
        trace_enabled,
    } = context;
    let expected_job_id = format!("{:032x}", request.job_id.0);
    process.transcribe_with_gpu(
        expected_job_id.clone(),
        request.generation.get(),
        attempt,
        path,
        sha256.clone(),
        expected_gpu.clone(),
    )?;
    let mut native_call_started = false;
    loop {
        if cancel.load(Ordering::Acquire) {
            return Err("Capture arrêtée pendant l'inférence".into());
        }
        match process.poll()? {
            Some(WorkerEvent::NativeCallStarted {
                request_id,
                job_id,
                generation,
                attempt: actual_attempt,
                pid,
            }) if request_id == attempt
                && job_id == expected_job_id
                && generation == request.generation.get()
                && actual_attempt == attempt
                && pid == process.pid() =>
            {
                native_call_started = true;
            }
            Some(WorkerEvent::Result {
                request_id,
                job_id,
                generation,
                attempt: actual_attempt,
                identity: actual_identity,
                source_sha256,
                duration_samples,
                text,
                words,
                gpu_evidence,
                ..
            }) if window_result_matches(
                WindowResultIdentity {
                    request_id: attempt,
                    job_id: &expected_job_id,
                    generation: request.generation.get(),
                    attempt,
                    identity,
                    source_sha256: &sha256,
                    duration_samples: expected_samples,
                },
                WindowResultIdentity {
                    request_id,
                    job_id: &job_id,
                    generation,
                    attempt: actual_attempt,
                    identity: &actual_identity,
                    source_sha256: &source_sha256,
                    duration_samples,
                },
                native_call_marker_required,
                native_call_started,
            ) && (request.config.compute != ComputeChoice::Gpu
                || gpu_evidence.as_ref().is_some_and(|evidence| {
                    expected_gpu.as_ref().is_some_and(|expected| {
                        crate::worker_ipc::gpu_evidence_matches(
                            Some(evidence),
                            Some(expected),
                            process.pid(),
                        )
                    })
                })) =>
            {
                let validation = validate_words(&words, expected_samples);
                let native = trace_enabled.then(|| {
                    native_hypothesis_trace(&text, &words, global_window_start_sample, &validation)
                });
                return Ok(match validation {
                    Ok(ranges) if ranges.is_empty() && !text.trim().is_empty() => {
                        WindowTranscription::Unusable {
                            message: "Texte Parakeet sans mots horodatés".into(),
                            native,
                        }
                    }
                    Ok(ranges) => WindowTranscription::Usable {
                        text,
                        words: words
                            .into_iter()
                            .zip(ranges)
                            .map(|(word, range)| TimedWord {
                                text: word.text,
                                range: SourceRange {
                                    start_sample: range.start_sample,
                                    end_sample: range.end_sample,
                                    sample_rate_hz: SAMPLE_RATE as u32,
                                },
                            })
                            .collect(),
                        native,
                    },
                    Err(error) => WindowTranscription::Unusable {
                        message: error,
                        native,
                    },
                });
            }
            Some(WorkerEvent::Failed {
                request_id: Some(request_id),
                message,
                ..
            }) if request_id == attempt => return Err(message),
            Some(_) => return Err("Réponse Parakeet tardive ou étrangère à la fenêtre".into()),
            None => thread::sleep(Duration::from_millis(5)),
        }
    }
}

pub(super) fn write_pcm16_wav(path: &Path, samples: &[i16]) -> Result<(), String> {
    let data_len = u32::try_from(samples.len().checked_mul(2).ok_or("Fenêtre trop longue")?)
        .map_err(|_| "Fenêtre WAV trop longue")?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("WAV fenêtre non créé : {error}"))?;
    file.write_all(b"RIFF")
        .and_then(|()| file.write_all(&(36_u32.saturating_add(data_len)).to_le_bytes()))
        .and_then(|()| file.write_all(b"WAVEfmt "))
        .and_then(|()| file.write_all(&16_u32.to_le_bytes()))
        .and_then(|()| file.write_all(&1_u16.to_le_bytes()))
        .and_then(|()| file.write_all(&1_u16.to_le_bytes()))
        .and_then(|()| file.write_all(&(SAMPLE_RATE as u32).to_le_bytes()))
        .and_then(|()| file.write_all(&((SAMPLE_RATE as u32) * 2).to_le_bytes()))
        .and_then(|()| file.write_all(&2_u16.to_le_bytes()))
        .and_then(|()| file.write_all(&16_u16.to_le_bytes()))
        .and_then(|()| file.write_all(b"data"))
        .and_then(|()| file.write_all(&data_len.to_le_bytes()))
        .map_err(|error| format!("En-tête WAV fenêtre non écrit : {error}"))?;
    for chunk in samples.chunks(16_000) {
        let mut bytes = Vec::with_capacity(chunk.len() * 2);
        for sample in chunk {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        file.write_all(&bytes)
            .map_err(|error| format!("Audio fenêtre non écrit : {error}"))?;
    }
    file.flush()
        .map_err(|error| format!("WAV fenêtre non synchronisé : {error}"))
}
