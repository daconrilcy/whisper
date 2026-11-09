#[cfg(windows)]
fn run_unified_poll_preview_fallback_child(preview: bool, cpu_identity_fails: bool) {
    const MODEL_SHA: &str = "e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e";
    let model = verified_personal_model();
    let local = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap());
    let fake = if preview {
        "fake_preview_worker.rs"
    } else {
        "fake_worker.rs"
    };
    let (cpu_paths, worker) = prepare_fake_personal_worker(&local, &model, MODEL_SHA, fake);
    if cpu_identity_fails {
        let path = worker.with_extension("identity");
        let mut identity: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        identity["model_sha256"] = serde_json::Value::String("0".repeat(64));
        std::fs::write(path, serde_json::to_vec(&identity).unwrap()).unwrap();
    }
    let source = local.join("source.wav");
    let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source).unwrap();
    writer
        .append(&vec![1000_i16; if preview { 4 } else { 3 } * 16_000])
        .unwrap();
    writer.finish().unwrap();
    let mut auto = unified_import_request(&local.join("auto"), &source, 73_110);
    auto.config.compute = ComputeChoice::Auto;
    let mut manual = unified_import_request(&local.join("manual"), &source, 73_111);
    manual.config.compute = ComputeChoice::Gpu;
    let mut poll_trace = FallbackPollTrace::new(&worker, &local);
    let (mut runtime, session, mut trace) =
        begin_gpu_fallback(&local, &auto, &manual, cpu_paths, preview, &mut poll_trace);
    if cpu_identity_fails {
        std::fs::write(worker.with_extension("release"), b"release").unwrap();
        let deadline = Instant::now() + Duration::from_secs(20);
        while !runtime
            .unified_executor
            .as_ref()
            .unwrap()
            .thread
            .as_ref()
            .unwrap()
            .is_finished()
        {
            assert!(Instant::now() < deadline);
            thread::yield_now();
        }
        let outcome = runtime.poll_unified_sessions().unwrap().unwrap_err();
        assert_eq!((outcome.0, outcome.1), (session.job_id, session.generation));
        assert_eq!(
            outcome.2,
            "NativeReadyIdentityMismatch: model/runtime/worker/profile differs"
        );
        assert_eq!(
            runtime.unified_sessions[&session]
                .native_route
                .as_ref()
                .unwrap()
                .compute,
            ComputeChoice::Cpu
        );
        assert_eq!(
            runtime.unified_sessions[&session].failure.as_deref(),
            Some(outcome.2.as_str())
        );
        runtime
            .unified_executor
            .as_mut()
            .unwrap()
            .stop_and_reap()
            .unwrap();
        return;
    }
    complete_cpu_fallback_call(
        &mut runtime,
        &worker,
        &local,
        session,
        &mut trace,
        &mut poll_trace,
    );
    assert!(runtime.poll_unified_sessions().is_none());
    assert_preview_fallback_retained(&runtime, session);
    trace.push(format!(
        "preview committed {}",
        unified_fallback_snapshot(&runtime)
    ));
    runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .stop_and_reap()
        .unwrap();
    std::fs::write(
        local.join("preview-fallback-trace.json"),
        serde_json::to_vec_pretty(&trace).unwrap(),
    )
    .unwrap();
}

#[cfg(windows)]
fn assert_preview_fallback_retained(
    runtime: &ImportRuntime,
    session: whisper_core::parakeet_scheduler::SchedulerSession,
) {
    let retained = &runtime.unified_sessions[&session];
    assert!(retained.pending_call.is_none());
    assert!(retained.pending_window.is_none());
    assert!(retained.pending_available_samples.is_none());
    assert_eq!(
        retained.native_route.as_ref().unwrap().compute,
        ComputeChoice::Cpu
    );
}

#[cfg(windows)]
#[test]
fn unified_poll_auto_preview_falls_back_before_ready_with_manual_peer() {
    use std::os::windows::process::CommandExt;
    const CHILD: &str = "UNIFIED_POLL_PREVIEW_FALLBACK_CHILD";
    if std::env::var_os(CHILD).is_some() {
        run_unified_poll_preview_fallback_child(true, false);
        return;
    }
    let mut root = TemporaryTestRoot::create(test_artifact_root("unified-poll-preview-fallback"))
        .expect("create isolated preview fallback root");
    let local = root.path().join("localappdata");
    std::fs::create_dir_all(&local).unwrap();
    root.child_starting();
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "worker_ipc::tests::unified_poll_auto_preview_falls_back_before_ready_with_manual_peer",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .env("LOCALAPPDATA", &local)
        .creation_flags(0x0800_0000)
        .output();
    let output = match output {
        Ok(output) => {
            root.child_finished();
            output
        }
        Err(error) => panic!("start preview fallback child: {error}"),
    };
    assert!(
        output.status.success(),
        "preview fallback child failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(local.join("preview-fallback-trace.json").is_file());
}

#[cfg(windows)]
#[test]
fn unified_poll_auto_preview_cpu_ready_failure_does_not_retry() {
    use std::os::windows::process::CommandExt;
    const CHILD: &str = "UNIFIED_POLL_PREVIEW_CPU_FAILURE_CHILD";
    if std::env::var_os(CHILD).is_some() {
        run_unified_poll_preview_fallback_child(true, true);
        return;
    }
    let mut root = TemporaryTestRoot::create(test_artifact_root("unified-poll-preview-cpu-failure"))
        .expect("create isolated preview CPU failure root");
    let local = root.path().join("localappdata");
    std::fs::create_dir_all(&local).unwrap();
    root.child_starting();
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "worker_ipc::tests::unified_poll_auto_preview_cpu_ready_failure_does_not_retry",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .env("LOCALAPPDATA", &local)
        .creation_flags(0x0800_0000)
        .output();
    let output = match output {
        Ok(output) => {
            root.child_finished();
            output
        }
        Err(error) => panic!("start preview CPU failure child: {error}"),
    };
    assert!(
        output.status.success(),
        "CPU failure child failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(windows)]
#[test]
fn unified_poll_auto_final_cpu_ready_failure_does_not_retry() {
    use std::os::windows::process::CommandExt;
    const CHILD: &str = "UNIFIED_POLL_FINAL_CPU_FAILURE_CHILD";
    if std::env::var_os(CHILD).is_some() {
        run_unified_poll_preview_fallback_child(false, true);
        return;
    }
    let mut root = TemporaryTestRoot::create(test_artifact_root("unified-poll-final-cpu-failure"))
        .expect("create isolated final CPU failure root");
    let local = root.path().join("localappdata");
    std::fs::create_dir_all(&local).unwrap();
    root.child_starting();
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "worker_ipc::tests::unified_poll_auto_final_cpu_ready_failure_does_not_retry",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .env("LOCALAPPDATA", &local)
        .creation_flags(0x0800_0000)
        .output();
    let output = match output {
        Ok(output) => {
            root.child_finished();
            output
        }
        Err(error) => panic!("start final CPU failure child: {error}"),
    };
    assert!(
        output.status.success(),
        "CPU failure child failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
