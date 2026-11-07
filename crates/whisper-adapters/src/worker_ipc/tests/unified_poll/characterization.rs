#[test]
fn unified_poll_empty_session_set_has_no_effects() {
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: PathBuf::from("unused-cpu"),
            gpu: PathBuf::from("unused-gpu"),
        },
        Arc::new(PublicationGate::default()),
    );
    assert!(runtime.poll_unified_sessions().is_none());
    assert!(runtime.unified_executor.is_none());
    assert!(runtime.native_executor_route.is_none());
    assert_eq!(runtime.parakeet_scheduler.active_sessions(), 0);
    assert!(runtime.parakeet_pending_events.is_empty());
}

#[test]
fn unified_poll_preview_uses_exact_acknowledgement_threshold() {
    let root = test_artifact_root("unified-poll-exact-preview-ack");
    std::fs::create_dir_all(&root).unwrap();
    let request = unified_live_request(&root, 73_138);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let capture = CaptureStream::empty_test_capture(16_000, 1, "test:exact-preview-ack");
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: PathBuf::from("unused-cpu"),
            gpu: PathBuf::from("unused-gpu"),
        },
        Arc::new(PublicationGate::default()),
    )
    .with_controlled_transcriber(controlled_worker().0);
    runtime
        .start_unified_capture(&request, capture, None, None)
        .unwrap();
    assert!(runtime.poll_unified_sessions().is_none());
    runtime.unified_executor.as_mut().unwrap().in_flight = true;
    for (sequence, boundary, expected_preview) in [(1, 63_999, 0), (2, 64_000, 64_000)] {
        runtime
            .unified_sessions
            .get_mut(&session)
            .unwrap()
            .source
            .stage_acknowledgements(vec![crate::parakeet_audio::CaptureWriteAck {
                sequence,
                range: 0..boundary,
                written_samples: boundary,
                flushed_samples: boundary,
                synchronized_samples: 0,
            }]);
        assert!(runtime.poll_unified_sessions().is_none());
        let state = &runtime.unified_sessions[&session];
        assert_eq!(state.available_samples, boundary);
        assert_eq!(state.preview_sequence, expected_preview);
    }
    runtime.unified_executor.as_mut().unwrap().in_flight = false;
    runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .stop_and_reap()
        .unwrap();
}

#[test]
fn unified_poll_publishes_telemetry_then_gain_then_progress_in_one_poll() {
    let root = test_artifact_root("unified-poll-observation-order");
    std::fs::create_dir_all(&root).unwrap();
    let request = unified_live_request(&root, 73_139);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let capture = CaptureStream::empty_test_capture(16_000, 1, "test:observation-order");
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: PathBuf::from("unused-cpu"),
            gpu: PathBuf::from("unused-gpu"),
        },
        Arc::new(PublicationGate::default()),
    )
    .with_controlled_transcriber(controlled_worker().0);
    runtime
        .start_unified_capture(&request, capture, None, None)
        .unwrap();
    assert!(runtime.poll_unified_sessions().is_none());
    let source = &mut runtime.unified_sessions.get_mut(&session).unwrap().source;
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut acknowledgements = Vec::new();
    for chunk in 1..=6 {
        source.enqueue_test_block(vec![0.1; 320]);
        acknowledgements.extend(source.pump().unwrap());
        while source.probe_processed_chunks() < chunk {
            assert!(
                Instant::now() < deadline,
                "probe did not process block {chunk}"
            );
            thread::yield_now();
        }
    }
    loop {
        acknowledgements.extend(source.pump().unwrap());
        if !acknowledgements.is_empty() && source.probe_snapshot_ready() {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "writer and probe did not both become ready"
        );
        thread::yield_now();
    }
    source.stage_acknowledgements(acknowledgements);
    assert!(runtime.parakeet_pending_events.is_empty());
    assert!(runtime.poll_unified_sessions().is_none());
    let events: Vec<_> = runtime.parakeet_pending_events.iter().collect();
    assert_eq!(events.len(), 3);
    assert!(
        matches!(events[0], ImportEvent::MicrophoneMeter(meter) if meter.job_id == session.job_id && meter.generation == session.generation)
    );
    assert!(
        matches!(events[1], ImportEvent::MicrophoneGainApplied { job_id, generation, .. } if *job_id == session.job_id && *generation == session.generation)
    );
    assert!(
        matches!(events[2], ImportEvent::ParakeetCaptureProgress { job_id, generation, .. } if *job_id == session.job_id && *generation == session.generation)
    );
    runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .stop_and_reap()
        .unwrap();
}

#[test]
fn unified_poll_refuses_route_replacement_while_executor_is_live() {
    let root = test_artifact_root("unified-poll-busy-route");
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("source.wav");
    let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source).unwrap();
    writer.append(&vec![1000_i16; 3 * 16_000]).unwrap();
    writer.finish().unwrap();
    let request = unified_import_request(&root.join("archive"), &source, 73_001);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: PathBuf::from("unused-cpu"),
            gpu: PathBuf::from("unused-gpu"),
        },
        Arc::new(PublicationGate::default()),
    )
    .with_controlled_transcriber(controlled_worker().0);
    runtime.start_controlled_unified_import(&request).unwrap();
    let current = controlled_native_route(&root);
    let mut next = current.clone();
    next.compute = ComputeChoice::Gpu;
    runtime.native_executor_route = Some(current);
    runtime
        .unified_sessions
        .get_mut(&session)
        .unwrap()
        .native_route = Some(next);
    runtime.unified_executor =
        Some(UnifiedInferenceExecutor::start(controlled_worker().0).unwrap());
    let result = runtime.poll_unified_sessions();
    assert!(matches!(result, Some(Err((job, generation, ref message)))
        if job == session.job_id && generation == session.generation
            && message == "NativeRouteBusy: a different backend cannot replace a live executor incarnation"));
    assert!(runtime.unified_sessions.contains_key(&session));
    assert!(runtime.native_executor_route.is_some());
    assert_eq!(runtime.parakeet_scheduler.active_sessions(), 1);
    assert!(runtime.parakeet_pending_events.is_empty());
    runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .stop_and_reap()
        .unwrap();
}

fn unified_poll_import_fixture(name: &str, id: u128) -> (ImportRuntime, ImportRequest) {
    let root = test_artifact_root(name);
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("source.wav");
    let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source).unwrap();
    writer.append(&vec![1000_i16; 3 * 16_000]).unwrap();
    writer.finish().unwrap();
    let request = unified_import_request(&root.join("archive"), &source, id);
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: PathBuf::from("unused-cpu"),
            gpu: PathBuf::from("unused-gpu"),
        },
        Arc::new(PublicationGate::default()),
    )
    .with_controlled_transcriber(controlled_worker().0);
    runtime.start_controlled_unified_import(&request).unwrap();
    (runtime, request)
}

#[test]
fn unified_poll_reports_unavailable_executor_without_changing_session() {
    for (native_candidate, expected) in [
        (
            false,
            "ModelNotQualified: unified inference executor is unavailable",
        ),
        (
            true,
            "NativeCandidateUnavailable: no validated route for unified session",
        ),
    ] {
        let (mut runtime, request) = unified_poll_import_fixture("unified-poll-no-worker", 73_002);
        runtime.controlled_transcriber = None;
        runtime.native_unified_candidate_enabled = native_candidate;
        let result = runtime.poll_unified_sessions();
        assert!(matches!(result, Some(Err((job, generation, ref message)))
            if job == request.job_id && generation == request.generation && message == expected));
        assert_eq!(runtime.unified_sessions.len(), 1);
        assert_eq!(runtime.parakeet_scheduler.active_sessions(), 1);
        assert!(runtime.unified_executor.is_none());
        assert!(runtime.parakeet_pending_events.is_empty());
    }
}

#[test]
fn unified_poll_restart_error_preserves_session_and_executor() {
    let (mut runtime, request) = unified_poll_import_fixture("unified-poll-restart-error", 73_003);
    let mut executor = UnifiedInferenceExecutor::start(controlled_worker().0).unwrap();
    executor.stop_and_reap().unwrap();
    assert!(executor.worker.take().is_some());
    runtime.unified_executor = Some(executor);
    let result = runtime.poll_unified_sessions();
    assert!(matches!(result, Some(Err((job, generation, ref message)))
        if job == request.job_id && generation == request.generation
            && message == "InferenceRestart: InferenceStart: worker is missing"));
    assert_eq!(runtime.unified_sessions.len(), 1);
    assert_eq!(runtime.parakeet_scheduler.active_sessions(), 1);
    assert!(runtime.unified_executor.is_some());
    assert!(runtime.parakeet_pending_events.is_empty());
}

#[test]
fn unified_poll_refuses_route_replacement_for_a_reserved_call() {
    let (mut runtime, request) = unified_poll_import_fixture("unified-poll-busy-scheduler", 73_004);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let root = PathBuf::from(&request.destination);
    let current = controlled_native_route(&root);
    let mut next = current.clone();
    next.compute = ComputeChoice::Gpu;
    runtime.native_executor_route = Some(current);
    runtime
        .unified_sessions
        .get_mut(&session)
        .unwrap()
        .native_route = Some(next);
    runtime.parakeet_scheduler.queue_final(session, 0).unwrap();
    let reserved = runtime.parakeet_scheduler.next_call().unwrap().unwrap();
    assert!(runtime.unified_executor.is_none());
    let result = runtime.poll_unified_sessions();
    assert!(matches!(result, Some(Err((job, generation, ref message)))
        if job == session.job_id && generation == session.generation
            && message == "NativeRouteBusy: a different backend cannot replace a live executor incarnation"));
    assert_eq!(runtime.parakeet_scheduler.in_flight(), Some(reserved));
    assert!(runtime.unified_executor.is_none());
    assert!(runtime.native_executor_route.is_some());
    assert!(runtime.unified_sessions.contains_key(&session));
}

#[test]
fn unified_poll_rejects_stale_result_without_losing_session() {
    let mut trace = Vec::<String>::new();
    let mut transitions = Vec::<String>::new();
    for stale_field in [
        "attempt",
        "request",
        "incarnation",
        "pending",
        "scheduler",
        "unknown_job",
        "unknown_generation",
    ] {
        characterize_stale_result(stale_field, &mut trace, &mut transitions);
    }
    let trace_file = test_artifact_root("unified-poll-stale-trace").with_extension("json");
    let trace_bytes = serde_json::to_vec_pretty(&trace).unwrap();
    std::fs::write(trace_file, &trace_bytes).unwrap();
    assert_eq!(
        trace_bytes.as_slice(),
        include_bytes!("reference_stale.json"),
        "stale-result effects diverged from the frozen baseline"
    );
    let path = test_artifact_root("unified-poll-stale-transitions-baseline").with_extension("json");
    let bytes = serde_json::to_vec_pretty(&transitions).unwrap();
    std::fs::write(path, &bytes).unwrap();
    assert_eq!(
        bytes.as_slice(),
        include_bytes!("reference_stale_poll_transitions.json")
    );
}

#[test]
fn unified_poll_capture_progress_is_monotone_and_preview_starts_at_four_seconds() {
    let root = test_artifact_root("unified-poll-capture-threshold");
    std::fs::create_dir_all(&root).unwrap();
    let request = unified_live_request(&root, 73_007);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let capture = CaptureStream::empty_test_capture(16_000, 1, "test:poll-threshold");
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: PathBuf::from("unused-cpu"),
            gpu: PathBuf::from("unused-gpu"),
        },
        Arc::new(PublicationGate::default()),
    )
    .with_controlled_transcriber(controlled_worker().0)
    .with_child_observer(Arc::new(Mutex::new(Vec::new())));
    let ready_gate = runtime.hold_next_child_ready();
    runtime
        .start_unified_capture(&request, capture, None, None)
        .unwrap();
    assert!(runtime.poll_unified_sessions().is_none());
    assert_eq!(runtime.unified_sessions[&session].available_samples, 0);
    // A live FFT converter retains a partial input chunk and its overlap delay;
    // the acknowledged output boundary is therefore not the input length.
    let block_sizes = [63_999_usize, 640];
    let mut reference_capture = CaptureStream::from_test_blocks(
        16_000,
        1,
        block_sizes.iter().map(|size| vec![0.1; *size]).collect(),
    );
    let mut reference_converter = crate::capture::Pcm16Converter::new(16_000, 1).unwrap();
    let mut expected = 0_u64;
    for block_samples in block_sizes {
        let block = reference_capture.try_next_block().unwrap();
        expected += reference_converter.push(&block).unwrap().len() as u64;
        if block_samples == block_sizes[0] {
            assert!(
                expected < 64_000,
                "first acknowledged boundary must precede the threshold"
            );
        }
        runtime
            .unified_sessions
            .get_mut(&session)
            .unwrap()
            .source
            .enqueue_test_block(vec![0.1; block_samples]);
        let deadline = Instant::now() + Duration::from_secs(10);
        while runtime.unified_sessions[&session].available_samples < expected {
            if let Some(Err(error)) = runtime.poll_unified_sessions() {
                panic!("capture poll failed before {expected}: {error:?}");
            }
            assert!(
                Instant::now() < deadline,
                "capture acknowledgement timed out at {expected}"
            );
            thread::yield_now();
        }
        let state = &runtime.unified_sessions[&session];
        assert_eq!(state.available_samples, expected);
        assert!(state.durable_samples <= state.available_samples);
        assert_eq!(
            state.preview_sequence,
            if expected < 64_000 { 0 } else { expected }
        );
        let matching_progress = runtime.parakeet_pending_events.iter().filter(|event| matches!(
            event,
            ImportEvent::ParakeetCaptureProgress { job_id, generation, captured_samples, .. }
                if *job_id == session.job_id && *generation == session.generation && *captured_samples == expected
        )).count();
        assert_eq!(matching_progress, 1);
    }
    assert!(
        expected >= 64_000,
        "second acknowledged boundary must cross the preview threshold"
    );
    let (released, changed) = &*ready_gate;
    *released.lock().unwrap() = true;
    changed.notify_all();
    runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .stop_and_reap()
        .unwrap();
}

#[test]
fn unified_poll_replaces_reaped_executor_for_different_route() {
    let (mut runtime, request) = unified_poll_import_fixture("unified-poll-reaped-route", 73_039);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let root = PathBuf::from(&request.destination);
    let current = controlled_native_route(&root);
    let mut next = current.clone();
    next.compute = ComputeChoice::Gpu;
    runtime.native_executor_route = Some(current);
    runtime
        .unified_sessions
        .get_mut(&session)
        .unwrap()
        .native_route = Some(next);
    let mut old = UnifiedInferenceExecutor::start(controlled_worker().0).unwrap();
    old.stop_and_reap().unwrap();
    runtime.unified_executor = Some(old);
    runtime = runtime.with_child_observer(Arc::new(Mutex::new(Vec::new())));
    let ready_gate = runtime.hold_next_child_ready();
    assert!(runtime.poll_unified_sessions().is_none());
    assert!(runtime.native_executor_route.is_none());
    assert!(runtime.unified_executor.as_ref().unwrap().thread.is_some());
    assert!(runtime.unified_sessions.contains_key(&session));
    let (released, changed) = &*ready_gate;
    *released.lock().unwrap() = true;
    changed.notify_all();
    runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .stop_and_reap()
        .unwrap();
}

#[test]
fn unified_poll_final_queue_uses_live_threshold_and_closed_tail() {
    // Invalid synthetic availability isolates scheduler selection without reading fake PCM.
    for (id, closed, available, should_reserve) in [
        (73_040, false, 0, false),
        (73_041, false, 1_926_399, false),
        (73_042, false, 1_926_400, true),
        (73_043, false, 1_926_401, true),
        (73_044, true, 1, true),
    ] {
        let (mut runtime, request) =
            unified_poll_import_fixture("unified-poll-final-threshold", id);
        let session = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: request.job_id,
            generation: request.generation,
        };
        let state = runtime.unified_sessions.get_mut(&session).unwrap();
        state.source_closed = closed;
        state.available_samples = available;
        runtime.controlled_hold_reserved_call_once = true;
        assert!(runtime.poll_unified_sessions().is_none());
        let reserved = runtime.parakeet_scheduler.in_flight();
        assert_eq!(
            reserved.is_some(),
            should_reserve,
            "closed={closed} available={available}"
        );
        if let Some(call) = reserved {
            assert_eq!(call.session, session);
            assert!(matches!(
                call.work,
                whisper_core::parakeet_scheduler::ScheduledWork::Final { .. }
            ));
        }
        runtime
            .unified_executor
            .as_mut()
            .unwrap()
            .stop_and_reap()
            .unwrap();
    }
}

#[test]
fn unified_poll_missing_selected_session_cancels_reservation() {
    let (mut runtime, missing) =
        unified_poll_import_fixture("unified-poll-missing-selection", 73_045);
    let root = PathBuf::from(&missing.destination)
        .parent()
        .unwrap()
        .to_path_buf();
    let peer = unified_import_request(&root.join("peer"), Path::new(&missing.source_path), 73_046);
    runtime.start_controlled_unified_import(&peer).unwrap();
    let selected = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: missing.job_id,
        generation: missing.generation,
    };
    let peer_id = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: peer.job_id,
        generation: peer.generation,
    };
    let state = runtime.unified_sessions.get_mut(&peer_id).unwrap();
    state.source_closed = false;
    state.available_samples = 0;
    runtime.parakeet_scheduler.queue_final(selected, 0).unwrap();
    let call = runtime.parakeet_scheduler.next_call().unwrap().unwrap();
    assert_eq!(call.session, selected);
    // Deliberately orphan the reservation to exercise the defensive path.
    runtime.unified_sessions.remove(&selected).unwrap();
    let outcome = runtime.poll_unified_sessions().unwrap().unwrap_err();
    assert_eq!(
        (outcome.0, outcome.1),
        (selected.job_id, selected.generation)
    );
    assert_eq!(outcome.2, "SchedulerSelection: selected session is missing");
    assert!(runtime.parakeet_scheduler.in_flight().is_none());
    assert!(runtime.unified_sessions.contains_key(&peer_id));
    runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .stop_and_reap()
        .unwrap();
}

#[test]
fn unified_poll_rejects_changed_retained_call_after_taking_tuple() {
    let (mut runtime, request) =
        unified_poll_import_fixture("unified-poll-retained-mismatch", 73_047);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    runtime.parakeet_scheduler.queue_final(session, 0).unwrap();
    let call = runtime.parakeet_scheduler.next_call().unwrap().unwrap();
    let mut stale = call;
    stale.attempt += 1;
    let window = whisper_core::parakeet_session::AudioWindow {
        sequence: 1,
        central_start_sample: 0,
        central_end_sample: 1,
        native_start_sample: 0,
        native_end_sample: 1,
    };
    // Deliberately inconsistent prepared tuple; this is not a product state.
    runtime
        .unified_sessions
        .get_mut(&session)
        .unwrap()
        .pending_prepared = Some((stale, window, vec![0], true));
    let outcome = runtime.poll_unified_sessions().unwrap().unwrap_err();
    assert_eq!((outcome.0, outcome.1), (session.job_id, session.generation));
    assert_eq!(
        outcome.2,
        "SchedulerPreparation: retained call identity changed"
    );
    assert!(
        runtime.unified_sessions[&session]
            .pending_prepared
            .is_none()
    );
    assert_eq!(runtime.parakeet_scheduler.in_flight(), Some(call));
    runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .stop_and_reap()
        .unwrap();
}

#[test]
fn unified_poll_invalid_preview_preparation_cancels_and_records_failure() {
    let (mut runtime, request) =
        unified_poll_import_fixture("unified-poll-preview-preparation", 73_048);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let state = runtime.unified_sessions.get_mut(&session).unwrap();
    state.source_closed = false;
    state.available_samples = 0;
    // Preview boundary 1 is intentionally invalid; no real capture can queue it.
    runtime
        .parakeet_scheduler
        .queue_preview(session, 1)
        .unwrap();
    let outcome = runtime.poll_unified_sessions().unwrap().unwrap_err();
    assert_eq!((outcome.0, outcome.1), (session.job_id, session.generation));
    assert_eq!(
        outcome.2,
        "Preview: four seconds of acknowledged audio are required"
    );
    assert_eq!(
        runtime.unified_sessions[&session].failure.as_deref(),
        Some(outcome.2.as_str())
    );
    assert!(runtime.parakeet_scheduler.in_flight().is_none());
    runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .stop_and_reap()
        .unwrap();
}

#[test]
fn unified_poll_final_waiting_for_audio_cancels_reservation() {
    let (mut runtime, request) = unified_poll_import_fixture("unified-poll-final-wait", 73_049);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let state = runtime.unified_sessions.get_mut(&session).unwrap();
    state.source_closed = false;
    state.available_samples = 0;
    runtime.parakeet_scheduler.queue_final(session, 0).unwrap();
    assert!(runtime.poll_unified_sessions().is_none());
    assert!(runtime.parakeet_scheduler.in_flight().is_none());
    assert!(runtime.unified_sessions.contains_key(&session));
    assert!(runtime.unified_sessions[&session].failure.is_none());
    runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .stop_and_reap()
        .unwrap();
}

#[test]
fn unified_poll_complete_final_window_cancels_reservation() {
    let (mut runtime, request) = unified_poll_import_fixture("unified-poll-final-complete", 73_127);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let state = runtime.unified_sessions.get_mut(&session).unwrap();
    assert!(matches!(
        state
            .finalizer
            .prepare_next(&mut state.source, state.available_samples, true),
        Ok(crate::parakeet_pipeline::PreparedFinalWindow::Ready { .. })
    ));
    state
        .finalizer
        .commit_prepared(&mut state.document, 1, &[])
        .unwrap();
    state
        .finalizer
        .finish_document(&mut state.document, state.available_samples, true)
        .unwrap();
    runtime.parakeet_scheduler.queue_final(session, 0).unwrap();
    let call = runtime.parakeet_scheduler.next_call().unwrap().unwrap();
    assert_eq!(call.session, session);
    assert!(runtime.poll_unified_sessions().is_none());
    assert!(runtime.parakeet_scheduler.in_flight().is_none());
    assert!(runtime.unified_sessions.contains_key(&session));
    assert!(runtime.unified_sessions[&session].failure.is_none());
    runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .stop_and_reap()
        .unwrap();
}

#[test]
fn unified_poll_preview_rejects_each_incomplete_prepared_pair() {
    for (id, has_window, has_boundary) in [
        (73_050, false, false),
        (73_051, true, false),
        (73_052, false, true),
    ] {
        let (mut runtime, request) =
            unified_poll_import_fixture("unified-poll-incomplete-preview", id);
        let session = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: request.job_id,
            generation: request.generation,
        };
        runtime
            .parakeet_scheduler
            .queue_preview(session, 64_000)
            .unwrap();
        let call = runtime.parakeet_scheduler.next_call().unwrap().unwrap();
        runtime.parakeet_scheduler.begin_call(call).unwrap();
        let state = runtime.unified_sessions.get_mut(&session).unwrap();
        state.pending_call = Some(call);
        state.pending_window = has_window.then_some(whisper_core::parakeet_session::AudioWindow {
            sequence: 64_000,
            central_start_sample: 0,
            central_end_sample: 64_000,
            native_start_sample: 0,
            native_end_sample: 64_000,
        });
        state.pending_available_samples = has_boundary.then_some(64_000);
        let mut executor = UnifiedInferenceExecutor::start(controlled_worker().0).unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while !executor.is_ready().unwrap() {
            assert!(Instant::now() < deadline);
            thread::yield_now();
        }
        executor
            .dispatch(UnifiedInferenceCommand {
                call,
                window: whisper_core::parakeet_session::AudioWindow {
                    sequence: 64_000,
                    central_start_sample: 0,
                    central_end_sample: 64_000,
                    native_start_sample: 0,
                    native_end_sample: 64_000,
                },
                pcm: vec![0; 64_000],
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
        assert_eq!(
            outcome.2,
            "Pipeline: preview result has no matching prepared window"
        );
        assert!(!runtime.unified_sessions.contains_key(&session));
        assert!(runtime.parakeet_scheduler.in_flight().is_none());
    }
}

#[test]
fn unified_poll_preview_commit_preserves_state_on_scheduler_complete_error() {
    let (mut runtime, request) = unified_poll_import_fixture("unified-poll-complete-error", 73_053);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    runtime
        .parakeet_scheduler
        .queue_preview(session, 64_000)
        .unwrap();
    let call = runtime.parakeet_scheduler.next_call().unwrap().unwrap();
    // Deliberately do not begin the reserved call: complete must reject it.
    let window = whisper_core::parakeet_session::AudioWindow {
        sequence: 64_000,
        central_start_sample: 0,
        central_end_sample: 64_000,
        native_start_sample: 0,
        native_end_sample: 64_000,
    };
    let state = runtime.unified_sessions.get_mut(&session).unwrap();
    state.pending_call = Some(call);
    state.pending_window = Some(window);
    state.pending_available_samples = Some(64_000);
    let mut executor = UnifiedInferenceExecutor::start(controlled_worker().0).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !executor.is_ready().unwrap() {
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    executor
        .dispatch(UnifiedInferenceCommand {
            call,
            window,
            pcm: vec![0; 64_000],
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
    assert_eq!(outcome.2, "SchedulerComplete: CallNotStarted");
    let state = &runtime.unified_sessions[&session];
    assert!(
        state.pending_call.is_none()
            && state.pending_window.is_none()
            && state.pending_available_samples.is_none()
    );
    assert_eq!(runtime.parakeet_scheduler.in_flight(), Some(call));
    runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .stop_and_reap()
        .unwrap();
}

#[test]
fn unified_poll_disconnect_without_scheduler_call_preserves_session() {
    let (mut runtime, request) =
        unified_poll_import_fixture("unified-poll-disconnect-no-call", 73_054);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let mut executor = UnifiedInferenceExecutor::start(controlled_worker().0).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !executor.is_ready().unwrap() {
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    drop(sender);
    // Deliberately inconsistent executor state: no scheduler reservation exists.
    executor.results = receiver;
    executor.in_flight = true;
    runtime.controlled_transcriber = None;
    runtime.unified_executor = Some(executor);
    let outcome = runtime.poll_unified_sessions().unwrap().unwrap_err();
    assert_eq!((outcome.0, outcome.1), (session.job_id, session.generation));
    assert_eq!(
        outcome.2,
        "InferenceWorkerExited: result channel disconnected; SchedulerCallMissing"
    );
    assert!(runtime.unified_sessions.contains_key(&session));
    assert!(runtime.parakeet_scheduler.in_flight().is_none());
    let executor = runtime.unified_executor.as_mut().unwrap();
    executor.in_flight = false;
    executor.stop_and_reap().unwrap();
}

struct UnifiedFailureWorker;
impl crate::parakeet_pipeline::WindowTranscriber for UnifiedFailureWorker {
    fn instance_id(&self) -> u64 {
        9011
    }
    fn transcribe(
        &mut self,
        _: whisper_core::parakeet_session::AudioWindow,
        _: &[i16],
    ) -> Result<Vec<whisper_core::parakeet_ipc::Word>, String> {
        Err("controlled inference failure".into())
    }
}

#[test]
fn unified_poll_inference_failure_reaps_and_restarts_only_for_peer() {
    for (id, with_peer) in [(73_055, false), (73_057, true)] {
        let (mut runtime, request) =
            unified_poll_import_fixture("unified-poll-inference-failure", id);
        let session = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: request.job_id,
            generation: request.generation,
        };
        let peer = if with_peer {
            let root = PathBuf::from(&request.destination)
                .parent()
                .unwrap()
                .to_path_buf();
            let peer =
                unified_import_request(&root.join("peer"), Path::new(&request.source_path), id + 1);
            runtime.start_controlled_unified_import(&peer).unwrap();
            Some(whisper_core::parakeet_scheduler::SchedulerSession {
                job_id: peer.job_id,
                generation: peer.generation,
            })
        } else {
            None
        };
        let mut executor = UnifiedInferenceExecutor::start(Box::new(UnifiedFailureWorker)).unwrap();
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
        let (send, receive) = std::sync::mpsc::sync_channel(1);
        assert!(send.send(result).is_ok());
        executor.results = receive;
        let outcome = runtime.poll_unified_sessions().unwrap().unwrap_err();
        assert_eq!((outcome.0, outcome.1), (session.job_id, session.generation));
        assert_eq!(outcome.2, "controlled inference failure");
        assert!(!runtime.unified_sessions.contains_key(&session));
        assert!(runtime.parakeet_scheduler.in_flight().is_none());
        assert_eq!(
            runtime.parakeet_scheduler.active_sessions(),
            usize::from(with_peer)
        );
        if let Some(peer) = peer {
            assert!(runtime.unified_sessions.contains_key(&peer));
            assert!(runtime.unified_executor.as_ref().unwrap().thread.is_some());
            runtime
                .unified_executor
                .as_mut()
                .unwrap()
                .stop_and_reap()
                .unwrap();
        } else {
            assert!(runtime.unified_executor.as_ref().unwrap().thread.is_none());
        }
    }
}

struct UnifiedInvalidWordWorker;
impl crate::parakeet_pipeline::WindowTranscriber for UnifiedInvalidWordWorker {
    fn instance_id(&self) -> u64 {
        9012
    }
    fn transcribe(
        &mut self,
        _: whisper_core::parakeet_session::AudioWindow,
        _: &[i16],
    ) -> Result<Vec<whisper_core::parakeet_ipc::Word>, String> {
        Ok(vec![whisper_core::parakeet_ipc::Word {
            text: "invalid".into(),
            start_ms: 1,
            end_ms: 1,
        }])
    }
}

struct UnifiedResultWithStopFailure {
    invalid_words: bool,
}

#[test]
fn unified_poll_deferred_capture_failure_reaps_result_and_restarts_remaining_scheduler() {
    let (mut runtime, request) =
        unified_poll_import_fixture("unified-poll-deferred-success", 73_134);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let mut executor = UnifiedInferenceExecutor::start(Box::new(UnifiedFailureWorker)).unwrap();
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
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    assert!(send.send(result).is_ok());
    executor.results = receive;
    runtime.unified_sessions.get_mut(&session).unwrap().failure =
        Some("controlled capture failure".into());
    let outcome = runtime.poll_unified_sessions().unwrap().unwrap_err();
    assert_eq!(
        outcome,
        (
            session.job_id,
            session.generation,
            "controlled capture failure".into()
        )
    );
    assert!(!runtime.unified_sessions.contains_key(&session));
    assert!(runtime.parakeet_scheduler.in_flight().is_none());
    assert_eq!(runtime.parakeet_scheduler.active_sessions(), 1);
    assert!(runtime.unified_executor.as_ref().unwrap().thread.is_some());
    runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .stop_and_reap()
        .unwrap();
}

#[test]
fn unified_poll_deferred_capture_failure_releases_terminal_scheduler_session() {
    let (mut runtime, request) =
        unified_poll_import_fixture("unified-poll-deferred-terminal", 73_135);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let mut executor = UnifiedInferenceExecutor::start(Box::new(UnifiedFailureWorker)).unwrap();
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
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    assert!(send.send(result).is_ok());
    executor.results = receive;
    runtime.unified_sessions.get_mut(&session).unwrap().failure =
        Some("controlled capture failure".into());
    runtime.parakeet_scheduler.abort_session(session).unwrap();
    let outcome = runtime.poll_unified_sessions().unwrap().unwrap_err();
    assert_eq!(
        outcome,
        (
            session.job_id,
            session.generation,
            "controlled capture failure".into()
        )
    );
    assert!(!runtime.unified_sessions.contains_key(&session));
    assert!(runtime.parakeet_scheduler.in_flight().is_none());
    assert_eq!(runtime.parakeet_scheduler.active_sessions(), 0);
    assert!(runtime.unified_executor.as_ref().unwrap().thread.is_none());
}

#[test]
fn unified_poll_deferred_capture_failure_restarts_for_peer() {
    let (mut runtime, request) = unified_poll_import_fixture("unified-poll-deferred-peer", 73_136);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let root = PathBuf::from(&request.destination)
        .parent()
        .unwrap()
        .to_path_buf();
    let peer = unified_import_request(&root.join("peer"), Path::new(&request.source_path), 73_137);
    let peer_id = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: peer.job_id,
        generation: peer.generation,
    };
    runtime.start_controlled_unified_import(&peer).unwrap();
    let mut executor = UnifiedInferenceExecutor::start(Box::new(UnifiedFailureWorker)).unwrap();
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
    assert_eq!(result.call.session, session);
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    assert!(send.send(result).is_ok());
    executor.results = receive;
    runtime.unified_sessions.get_mut(&session).unwrap().failure =
        Some("controlled capture failure".into());
    runtime.parakeet_scheduler.abort_session(session).unwrap();
    let outcome = runtime.poll_unified_sessions().unwrap().unwrap_err();
    assert_eq!(
        outcome,
        (
            session.job_id,
            session.generation,
            "controlled capture failure".into()
        )
    );
    assert!(!runtime.unified_sessions.contains_key(&session));
    assert!(runtime.unified_sessions.contains_key(&peer_id));
    assert_eq!(runtime.parakeet_scheduler.active_sessions(), 1);
    assert!(runtime.unified_executor.as_ref().unwrap().thread.is_some());
    runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .stop_and_reap()
        .unwrap();
}

impl crate::parakeet_pipeline::WindowTranscriber for UnifiedResultWithStopFailure {
    fn instance_id(&self) -> u64 {
        9015
    }
    fn transcribe(
        &mut self,
        _: whisper_core::parakeet_session::AudioWindow,
        _: &[i16],
    ) -> Result<Vec<whisper_core::parakeet_ipc::Word>, String> {
        if self.invalid_words {
            Ok(vec![whisper_core::parakeet_ipc::Word {
                text: "invalid".into(),
                start_ms: 1,
                end_ms: 1,
            }])
        } else {
            Err("controlled inference failure".into())
        }
    }
    fn stop_instance(&mut self) -> Result<(), String> {
        Err("controlled stop failure".into())
    }
}

#[test]
fn unified_poll_failure_paths_retain_state_when_reap_fails() {
    for (id, scenario) in [
        (73_130, "deferred"),
        (73_131, "inference"),
        (73_132, "commit"),
    ] {
        let (mut runtime, request) = unified_poll_import_fixture("unified-poll-failure-stop", id);
        let session = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: request.job_id,
            generation: request.generation,
        };
        runtime.parakeet_scheduler.queue_final(session, 0).unwrap();
        let call = runtime.parakeet_scheduler.next_call().unwrap().unwrap();
        runtime.parakeet_scheduler.begin_call(call).unwrap();
        let state = runtime.unified_sessions.get_mut(&session).unwrap();
        state.pending_call = Some(call);
        if scenario == "deferred" {
            state.failure = Some("controlled capture failure".into());
        }
        let window = match state
            .finalizer
            .prepare_next(&mut state.source, state.available_samples, true)
            .unwrap()
        {
            crate::parakeet_pipeline::PreparedFinalWindow::Ready { window, .. } => window,
            _ => panic!("fixture must prepare a final window"),
        };
        let mut executor =
            UnifiedInferenceExecutor::start(Box::new(UnifiedResultWithStopFailure {
                invalid_words: scenario == "commit",
            }))
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while !executor.is_ready().unwrap() {
            assert!(Instant::now() < deadline);
            thread::yield_now();
        }
        executor
            .dispatch(UnifiedInferenceCommand {
                call,
                window,
                pcm: vec![0; 1600],
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
        let initial = match scenario {
            "deferred" => "controlled capture failure",
            "inference" => "controlled inference failure",
            _ => "InvalidTimestamp: word 0 is invalid or unordered",
        };
        assert_eq!((outcome.0, outcome.1), (session.job_id, session.generation));
        assert_eq!(
            outcome.2,
            format!("{initial}; InferenceReap: controlled stop failure")
        );
        assert!(runtime.unified_sessions.contains_key(&session));
        assert!(runtime.parakeet_scheduler.in_flight().is_none());
        assert!(runtime.unified_executor.as_ref().unwrap().thread.is_none());
    }
}

#[test]
fn unified_poll_inference_and_commit_failures_retain_state_when_recycle_not_pending() {
    for (id, scenario) in [
        (73_128, "inference"),
        (73_129, "commit"),
        (73_133, "deferred"),
    ] {
        let (mut runtime, request) =
            unified_poll_import_fixture("unified-poll-failure-recycle", id);
        let session = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: request.job_id,
            generation: request.generation,
        };
        runtime.parakeet_scheduler.queue_final(session, 0).unwrap();
        let call = runtime.parakeet_scheduler.next_call().unwrap().unwrap();
        let state = runtime.unified_sessions.get_mut(&session).unwrap();
        state.pending_call = Some(call);
        if scenario == "deferred" {
            state.failure = Some("controlled capture failure".into());
        }
        let window = match state
            .finalizer
            .prepare_next(&mut state.source, state.available_samples, true)
            .unwrap()
        {
            crate::parakeet_pipeline::PreparedFinalWindow::Ready { window, .. } => window,
            _ => panic!("fixture must prepare a final window"),
        };
        let worker: Box<dyn crate::parakeet_pipeline::WindowTranscriber> = if scenario == "commit" {
            Box::new(UnifiedInvalidWordWorker)
        } else {
            Box::new(UnifiedFailureWorker)
        };
        let mut executor = UnifiedInferenceExecutor::start(worker).unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while !executor.is_ready().unwrap() {
            assert!(Instant::now() < deadline);
            thread::yield_now();
        }
        executor
            .dispatch(UnifiedInferenceCommand {
                call,
                window,
                pcm: vec![0; 1600],
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
        let initial = match scenario {
            "commit" => "InvalidTimestamp: word 0 is invalid or unordered",
            "deferred" => "controlled capture failure",
            _ => "controlled inference failure",
        };
        assert_eq!((outcome.0, outcome.1), (session.job_id, session.generation));
        assert_eq!(
            outcome.2,
            format!("{initial}; SchedulerReap: NoRecyclePending")
        );
        assert!(runtime.unified_sessions.contains_key(&session));
        assert!(runtime.parakeet_scheduler.in_flight().is_none());
        assert!(runtime.unified_executor.as_ref().unwrap().thread.is_none());
    }
}

#[test]
fn unified_poll_invalid_final_commit_reaps_and_restarts_only_for_peer() {
    for (id, with_peer) in [(73_059, false), (73_061, true)] {
        let (mut runtime, request) = unified_poll_import_fixture("unified-poll-commit-failure", id);
        let session = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: request.job_id,
            generation: request.generation,
        };
        let peer = if with_peer {
            let root = PathBuf::from(&request.destination)
                .parent()
                .unwrap()
                .to_path_buf();
            let peer =
                unified_import_request(&root.join("peer"), Path::new(&request.source_path), id + 1);
            runtime.start_controlled_unified_import(&peer).unwrap();
            Some(whisper_core::parakeet_scheduler::SchedulerSession {
                job_id: peer.job_id,
                generation: peer.generation,
            })
        } else {
            None
        };
        let mut executor =
            UnifiedInferenceExecutor::start(Box::new(UnifiedInvalidWordWorker)).unwrap();
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
        let (send, receive) = std::sync::mpsc::sync_channel(1);
        assert!(send.send(result).is_ok());
        executor.results = receive;
        let outcome = runtime.poll_unified_sessions().unwrap().unwrap_err();
        assert_eq!((outcome.0, outcome.1), (session.job_id, session.generation));
        assert_eq!(
            outcome.2,
            "InvalidTimestamp: word 0 is invalid or unordered"
        );
        assert!(!runtime.unified_sessions.contains_key(&session));
        assert!(runtime.parakeet_scheduler.in_flight().is_none());
        assert_eq!(
            runtime.parakeet_scheduler.active_sessions(),
            usize::from(with_peer)
        );
        if let Some(peer) = peer {
            assert!(runtime.unified_sessions.contains_key(&peer));
            assert!(runtime.unified_executor.as_ref().unwrap().thread.is_some());
            runtime
                .unified_executor
                .as_mut()
                .unwrap()
                .stop_and_reap()
                .unwrap();
        } else {
            assert!(runtime.unified_executor.as_ref().unwrap().thread.is_none());
        }
    }
}

#[test]
fn unified_poll_rejects_already_begun_final_and_preview_calls() {
    for (id, is_final) in [(73_063, true), (73_064, false)] {
        let (mut runtime, request) = unified_poll_import_fixture("unified-poll-begin-error", id);
        let session = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: request.job_id,
            generation: request.generation,
        };
        if is_final {
            runtime.parakeet_scheduler.queue_final(session, 1).unwrap();
        } else {
            runtime
                .parakeet_scheduler
                .queue_preview(session, 64_000)
                .unwrap();
        }
        let call = runtime.parakeet_scheduler.next_call().unwrap().unwrap();
        runtime.parakeet_scheduler.begin_call(call).unwrap();
        let window = whisper_core::parakeet_session::AudioWindow {
            sequence: 1,
            central_start_sample: 0,
            central_end_sample: 64_000,
            native_start_sample: 0,
            native_end_sample: 64_000,
        };
        // Pre-begun reservation with retained payload exercises a defensive error.
        runtime
            .unified_sessions
            .get_mut(&session)
            .unwrap()
            .pending_prepared = Some((call, window, vec![0; 64_000], is_final));
        let mut executor = UnifiedInferenceExecutor::start(controlled_worker().0).unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while !executor.is_ready().unwrap() {
            assert!(Instant::now() < deadline);
            thread::yield_now();
        }
        runtime.controlled_transcriber = None;
        runtime.unified_executor = Some(executor);
        let outcome = runtime.poll_unified_sessions().unwrap().unwrap_err();
        assert_eq!((outcome.0, outcome.1), (session.job_id, session.generation));
        assert_eq!(outcome.2, "SchedulerBegin: CallInFlight");
        let state = &runtime.unified_sessions[&session];
        assert_eq!(
            state.failure.as_deref(),
            Some("SchedulerBegin: CallInFlight")
        );
        assert_eq!(state.pending_window.is_some(), !is_final);
        assert_eq!(state.pending_available_samples.is_some(), !is_final);
        assert_eq!(runtime.parakeet_scheduler.in_flight(), Some(call));
        runtime
            .unified_executor
            .as_mut()
            .unwrap()
            .stop_and_reap()
            .unwrap();
    }
}

#[test]
fn unified_poll_disconnect_unstarted_final_reports_missing_recycle() {
    let (mut runtime, request) =
        unified_poll_import_fixture("unified-poll-disconnect-unstarted", 73_065);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    runtime.parakeet_scheduler.queue_final(session, 1).unwrap();
    let call = runtime.parakeet_scheduler.next_call().unwrap().unwrap();
    // The reserved call is intentionally dispatched without begin_call.
    let mut executor = UnifiedInferenceExecutor::start(controlled_worker().0).unwrap();
    executor.stop_and_reap().unwrap();
    executor.disconnect_result_for = Some(session);
    executor.restart().unwrap();
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
    while !executor.thread.as_ref().unwrap().is_finished() {
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    runtime.controlled_transcriber = None;
    runtime.unified_executor = Some(executor);
    let outcome = runtime.poll_unified_sessions().unwrap().unwrap_err();
    assert_eq!((outcome.0, outcome.1), (session.job_id, session.generation));
    assert_eq!(
        outcome.2,
        "InferenceWorkerExited: result channel disconnected; SchedulerReap: NoRecyclePending"
    );
    assert!(runtime.unified_sessions.contains_key(&session));
    assert!(runtime.parakeet_scheduler.in_flight().is_none());
    assert!(runtime.unified_executor.as_ref().unwrap().thread.is_none());
}

#[test]
fn unified_poll_replaces_only_own_capture_progress_and_keeps_other_order() {
    let root = test_artifact_root("unified-poll-progress-coalescing");
    std::fs::create_dir_all(&root).unwrap();
    let request = unified_live_request(&root, 73_068);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let capture = CaptureStream::empty_test_capture(16_000, 1, "test:progress-coalescing");
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: PathBuf::from("unused-cpu"),
            gpu: PathBuf::from("unused-gpu"),
        },
        Arc::new(PublicationGate::default()),
    )
    .with_controlled_transcriber(controlled_worker().0)
    .with_child_observer(Arc::new(Mutex::new(Vec::new())));
    let ready_gate = runtime.hold_next_child_ready();
    runtime
        .start_unified_capture(&request, capture, None, None)
        .unwrap();
    let progress = |job_id, samples| ImportEvent::ParakeetCaptureProgress {
        job_id,
        generation: request.generation,
        session_id: 0,
        captured_samples: samples,
        audio_durable_samples: 0,
    };
    runtime
        .parakeet_pending_events
        .push_back(progress(session.job_id, 1));
    runtime
        .parakeet_pending_events
        .push_back(progress(JobId(73_069), 7));
    runtime
        .parakeet_pending_events
        .push_back(ImportEvent::ParakeetCaptureProgress {
            job_id: session.job_id,
            generation: session.generation.next().unwrap(),
            session_id: 0,
            captured_samples: 8,
            audio_durable_samples: 0,
        });
    runtime
        .parakeet_pending_events
        .push_back(progress(session.job_id, 2));
    runtime
        .unified_sessions
        .get_mut(&session)
        .unwrap()
        .source
        .enqueue_test_block(vec![0.1; 16_000]);
    let deadline = Instant::now() + Duration::from_secs(10);
    while runtime.unified_sessions[&session].available_samples == 0 {
        assert!(runtime.poll_unified_sessions().is_none());
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    let relevant: Vec<_> = runtime
        .parakeet_pending_events
        .iter()
        .filter_map(|event| match event {
            ImportEvent::ParakeetCaptureProgress {
                job_id,
                captured_samples,
                ..
            } => Some((*job_id, *captured_samples)),
            _ => None,
        })
        .collect();
    assert_eq!(relevant.len(), 3);
    assert_eq!(relevant[0], (JobId(73_069), 7));
    assert_eq!(relevant[1], (session.job_id, 8));
    assert_eq!(
        relevant[2],
        (
            session.job_id,
            runtime.unified_sessions[&session].available_samples
        )
    );
    let gain_index = runtime.parakeet_pending_events.iter().position(|event| matches!(event,
        ImportEvent::MicrophoneGainApplied { job_id, generation, .. } if *job_id == session.job_id && *generation == session.generation)).unwrap();
    let progress_index = runtime.parakeet_pending_events.iter().position(|event| matches!(event,
        ImportEvent::ParakeetCaptureProgress { job_id, generation, .. } if *job_id == session.job_id && *generation == session.generation)).unwrap();
    assert!(gain_index < progress_index);
    let (released, changed) = &*ready_gate;
    *released.lock().unwrap() = true;
    changed.notify_all();
    runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .stop_and_reap()
        .unwrap();
}

#[test]
fn unified_poll_refuses_route_change_while_executor_reports_inflight() {
    let (mut runtime, request) = unified_poll_import_fixture("unified-poll-inflight-route", 73_070);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let root = PathBuf::from(&request.destination);
    let current = controlled_native_route(&root);
    let mut next = current.clone();
    next.compute = ComputeChoice::Gpu;
    runtime.native_executor_route = Some(current);
    runtime
        .unified_sessions
        .get_mut(&session)
        .unwrap()
        .native_route = Some(next);
    let mut executor = UnifiedInferenceExecutor::start(controlled_worker().0).unwrap();
    executor.stop_and_reap().unwrap();
    // Deliberately inconsistent flags isolate the in-flight guard from thread and scheduler guards.
    executor.in_flight = true;
    runtime.unified_executor = Some(executor);
    let outcome = runtime.poll_unified_sessions().unwrap().unwrap_err();
    assert_eq!((outcome.0, outcome.1), (session.job_id, session.generation));
    assert_eq!(
        outcome.2,
        "NativeRouteBusy: a different backend cannot replace a live executor incarnation"
    );
    assert!(runtime.unified_sessions.contains_key(&session));
    assert!(runtime.unified_executor.as_ref().unwrap().in_flight);
    assert!(runtime.unified_executor.as_ref().unwrap().thread.is_none());
    assert!(runtime.parakeet_scheduler.in_flight().is_none());
}

#[test]
fn unified_poll_preview_queue_error_occurs_after_progress_publication() {
    let root = test_artifact_root("unified-poll-preview-queue-error");
    std::fs::create_dir_all(&root).unwrap();
    let request = unified_live_request(&root, 73_071);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let capture = CaptureStream::empty_test_capture(16_000, 1, "test:preview-queue-error");
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: PathBuf::from("unused-cpu"),
            gpu: PathBuf::from("unused-gpu"),
        },
        Arc::new(PublicationGate::default()),
    )
    .with_controlled_transcriber(controlled_worker().0)
    .with_child_observer(Arc::new(Mutex::new(Vec::new())));
    let ready_gate = runtime.hold_next_child_ready();
    runtime
        .start_unified_capture(&request, capture, None, None)
        .unwrap();
    // Deliberately mark the scheduler session terminal while capture remains open.
    runtime.parakeet_scheduler.abort_session(session).unwrap();
    runtime
        .unified_sessions
        .get_mut(&session)
        .unwrap()
        .source
        .enqueue_test_block(vec![0.1; 70_000]);
    let deadline = Instant::now() + Duration::from_secs(10);
    let outcome = loop {
        if let Some(outcome) = runtime.poll_unified_sessions() {
            break outcome.unwrap_err();
        }
        assert!(Instant::now() < deadline);
        thread::yield_now();
    };
    assert_eq!((outcome.0, outcome.1), (session.job_id, session.generation));
    assert_eq!(outcome.2, "SchedulerPreview: Terminal");
    let available = runtime.unified_sessions[&session].available_samples;
    assert!(available >= 64_000);
    assert!(runtime.parakeet_pending_events.iter().any(|event| matches!(event, ImportEvent::ParakeetCaptureProgress { job_id, generation, captured_samples, .. } if *job_id == session.job_id && *generation == session.generation && *captured_samples == available)));
    let (released, changed) = &*ready_gate;
    *released.lock().unwrap() = true;
    changed.notify_all();
    runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .stop_and_reap()
        .unwrap();
}

#[test]
fn unified_poll_retains_final_and_preview_through_readiness_announcement() {
    for (id, final_work) in [(73_082, true), (73_083, false)] {
        let (mut runtime, request) = unified_poll_import_fixture("unified-poll-readiness", id);
        let session = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: request.job_id,
            generation: request.generation,
        };
        if final_work {
            runtime.parakeet_scheduler.queue_final(session, 1).unwrap();
        } else {
            runtime
                .parakeet_scheduler
                .queue_preview(session, 64_000)
                .unwrap();
        }
        let call = runtime.parakeet_scheduler.next_call().unwrap().unwrap();
        let window = whisper_core::parakeet_session::AudioWindow {
            sequence: 1,
            central_start_sample: 0,
            central_end_sample: 1_600,
            native_start_sample: 0,
            native_end_sample: 1_600,
        };
        let state = runtime.unified_sessions.get_mut(&session).unwrap();
        state.native_route = Some(controlled_native_route(Path::new(&request.destination)));
        state.pending_prepared = Some((call, window, vec![0; 1_600], final_work));
        runtime = runtime.with_child_observer(Arc::new(Mutex::new(Vec::new())));
        let ready_gate = runtime.hold_next_child_ready();
        assert!(runtime.poll_unified_sessions().is_none());
        assert!(
            !runtime
                .unified_executor
                .as_mut()
                .unwrap()
                .is_ready()
                .unwrap()
        );
        assert!(
            runtime.unified_sessions[&session]
                .pending_prepared
                .is_some()
        );
        let (released, changed) = &*ready_gate;
        *released.lock().unwrap() = true;
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
        let event = runtime.poll_unified_sessions().unwrap().unwrap();
        assert!(
            matches!(event, ImportEvent::Ready { job_id, generation, .. } if job_id == session.job_id && generation == session.generation)
        );
        assert!(runtime.unified_sessions[&session].backend_ready_reported);
        assert!(
            runtime.unified_sessions[&session]
                .pending_prepared
                .is_some()
        );
        assert!(runtime.poll_unified_sessions().is_none());
        assert_eq!(runtime.unified_sessions[&session].pending_call, Some(call));
        assert!(
            runtime.unified_sessions[&session]
                .pending_prepared
                .is_none()
        );
        let executor = runtime.unified_executor.as_mut().unwrap();
        let _ = executor
            .results
            .recv_timeout(Duration::from_secs(10))
            .unwrap();
        executor.in_flight = false;
        executor.stop_and_reap().unwrap();
    }
}

struct UnifiedHeldPanicWorker {
    sent: std::sync::mpsc::SyncSender<()>,
    release: std::sync::mpsc::Receiver<()>,
}

impl crate::parakeet_pipeline::WindowTranscriber for UnifiedHeldPanicWorker {
    fn instance_id(&self) -> u64 {
        9014
    }
    fn transcribe(
        &mut self,
        _: whisper_core::parakeet_session::AudioWindow,
        _: &[i16],
    ) -> Result<Vec<whisper_core::parakeet_ipc::Word>, String> {
        self.sent.send(()).unwrap();
        self.release.recv().unwrap();
        panic!("controlled unified worker panic");
    }
}

#[test]
fn unified_poll_worker_panic_closes_capture_and_releases_import_peer() {
    let root = test_artifact_root("unified-poll-panic-captures");
    std::fs::create_dir_all(&root).unwrap();
    let first = unified_live_request(&root.join("first"), 73_084);
    let source = root.join("peer.wav");
    let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source).unwrap();
    writer.append(&vec![1000_i16; 48_000]).unwrap();
    writer.finish().unwrap();
    let peer = unified_import_request(&root.join("peer"), &source, 73_085);
    let first_id = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: first.job_id,
        generation: first.generation,
    };
    let peer_id = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: peer.job_id,
        generation: peer.generation,
    };
    let capture_a = CaptureStream::empty_test_capture(16_000, 1, "test:panic-a");
    let (call_sent, calls) = std::sync::mpsc::sync_channel(1);
    let (release, released) = std::sync::mpsc::sync_channel(1);
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: PathBuf::from("unused-cpu"),
            gpu: PathBuf::from("unused-gpu"),
        },
        Arc::new(PublicationGate::default()),
    )
    .with_controlled_transcriber(Box::new(UnifiedHeldPanicWorker {
        sent: call_sent,
        release: released,
    }));
    runtime
        .start_unified_capture(&first, capture_a, None, None)
        .unwrap();
    let source_path = runtime.unified_sessions[&first_id]
        .request
        .source_path
        .clone();
    runtime
        .unified_sessions
        .get_mut(&first_id)
        .unwrap()
        .source
        .enqueue_test_block(vec![0.1; 70_000]);
    let deadline = Instant::now() + Duration::from_secs(10);
    while calls.try_recv().is_err() {
        assert!(runtime.poll_unified_sessions().is_none());
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    runtime.start_controlled_unified_import(&peer).unwrap();
    release.send(()).unwrap();
    let outcome = loop {
        if let Some(result) = runtime.poll_unified_sessions() {
            break result.unwrap_err();
        }
        assert!(Instant::now() < deadline, "panic result timed out");
        thread::yield_now();
    };
    assert_eq!(
        (outcome.0, outcome.1),
        (first_id.job_id, first_id.generation)
    );
    assert!(
        outcome
            .2
            .contains("InferenceReap: disconnected worker thread panicked"),
        "{}",
        outcome.2
    );
    assert!(!runtime.unified_sessions.contains_key(&first_id));
    assert!(!runtime.unified_sessions.contains_key(&peer_id));
    assert!(!runtime.unified_archives.contains_key(&first_id));
    assert!(!runtime.unified_archives.contains_key(&peer_id));
    let mut reference = CaptureStream::from_test_blocks(16_000, 1, vec![vec![0.1; 70_000]]);
    let mut converter = crate::capture::Pcm16Converter::new(16_000, 1).unwrap();
    let mut expected = converter
        .push(&reference.try_next_block().unwrap())
        .unwrap();
    expected.extend(converter.finish().unwrap());
    let persisted = crate::parakeet_audio::PcmPartStore::open(Path::new(&source_path));
    assert_eq!(
        persisted.read_range(0..expected.len() as u64).unwrap(),
        expected
    );
    assert!(runtime.parakeet_pending_events.iter().any(|event| matches!(event, ImportEvent::Failed { job_id, generation, message } if *job_id == peer_id.job_id && *generation == peer_id.generation && message.contains("worker panic"))));
}

#[test]
fn unified_poll_emits_active_microphone_telemetry_without_hold() {
    let root = test_artifact_root("unified-poll-active-telemetry");
    std::fs::create_dir_all(&root).unwrap();
    let request = unified_live_request(&root, 73_086);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let capture = CaptureStream::empty_test_capture(16_000, 1, "test:active-telemetry");
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: PathBuf::from("unused-cpu"),
            gpu: PathBuf::from("unused-gpu"),
        },
        Arc::new(PublicationGate::default()),
    )
    .with_controlled_transcriber(controlled_worker().0);
    runtime
        .start_unified_capture(&request, capture, None, None)
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut metered = false;
    for _ in 0..120 {
        runtime
            .unified_sessions
            .get_mut(&session)
            .unwrap()
            .source
            .enqueue_test_block(vec![0.1; 320]);
        assert!(runtime.poll_unified_sessions().is_none());
        metered = runtime.parakeet_pending_events.iter().any(|event| matches!(event, ImportEvent::MicrophoneMeter(meter) if meter.job_id == session.job_id && meter.generation == session.generation));
        if metered {
            break;
        }
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(10));
    }
    assert!(metered, "active probe must publish microphone telemetry");
    runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .stop_and_reap()
        .unwrap();
}

#[test]
fn unified_poll_meter_replaces_same_owner_and_preserves_other_owner() {
    let root = test_artifact_root("unified-poll-meter-coalescing");
    std::fs::create_dir_all(&root).unwrap();
    let request = unified_live_request(&root, 73_087);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let capture = CaptureStream::empty_test_capture(16_000, 1, "test:meter-coalescing");
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: PathBuf::from("unused-cpu"),
            gpu: PathBuf::from("unused-gpu"),
        },
        Arc::new(PublicationGate::default()),
    )
    .with_controlled_transcriber(controlled_worker().0);
    runtime
        .start_unified_capture(&request, capture, None, None)
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let first = loop {
        runtime
            .unified_sessions
            .get_mut(&session)
            .unwrap()
            .source
            .enqueue_test_block(vec![0.1; 320]);
        assert!(runtime.poll_unified_sessions().is_none());
        if let Some(meter) = runtime
            .parakeet_pending_events
            .iter()
            .find_map(|event| match event {
                ImportEvent::MicrophoneMeter(meter) if meter.job_id == session.job_id => {
                    Some(meter.clone())
                }
                _ => None,
            })
        {
            break meter;
        }
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(10));
    };
    let mut other = first.clone();
    other.job_id = JobId(73_088);
    runtime.parakeet_pending_events.clear();
    runtime
        .parakeet_pending_events
        .push_back(ImportEvent::MicrophoneMeter(first.clone()));
    runtime
        .parakeet_pending_events
        .push_back(ImportEvent::MicrophoneMeter(other.clone()));
    runtime
        .parakeet_pending_events
        .push_back(ImportEvent::MicrophoneMeter(first.clone()));
    loop {
        runtime
            .unified_sessions
            .get_mut(&session)
            .unwrap()
            .source
            .enqueue_test_block(vec![0.1; 320]);
        assert!(runtime.poll_unified_sessions().is_none());
        let meters: Vec<_> = runtime
            .parakeet_pending_events
            .iter()
            .filter_map(|event| match event {
                ImportEvent::MicrophoneMeter(meter) => Some(meter),
                _ => None,
            })
            .collect();
        if meters
            .iter()
            .any(|meter| meter.job_id == session.job_id && meter.sequence > first.sequence)
        {
            assert_eq!(meters.len(), 2);
            assert_eq!(meters[0].job_id, other.job_id);
            assert_eq!(meters[1].job_id, session.job_id);
            break;
        }
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(10));
    }
    runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .stop_and_reap()
        .unwrap();
}

#[test]
fn unified_poll_spectrum_replaces_same_owner_and_preserves_other_owner() {
    use whisper_core::microphone_level::MicrophoneSpectrumUpdate;
    let root = test_artifact_root("unified-poll-spectrum-coalescing");
    std::fs::create_dir_all(&root).unwrap();
    let request = unified_live_request(&root, 73_089);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let capture = CaptureStream::empty_test_capture(16_000, 1, "test:spectrum-coalescing");
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: PathBuf::from("unused-cpu"),
            gpu: PathBuf::from("unused-gpu"),
        },
        Arc::new(PublicationGate::default()),
    )
    .with_controlled_transcriber(controlled_worker().0);
    runtime
        .start_unified_capture(&request, capture, None, None)
        .unwrap();
    let unavailable = |job_id| MicrophoneSpectrumUpdate::Unavailable {
        job_id,
        generation: session.generation,
        source_identity: "test:spectrum-coalescing".into(),
        sequence: 0,
        epoch: 0,
        dropped_updates: 0,
        captured_at_monotonic_ms: 0,
    };
    runtime
        .parakeet_pending_events
        .push_back(ImportEvent::MicrophoneSpectrum(unavailable(session.job_id)));
    runtime
        .parakeet_pending_events
        .push_back(ImportEvent::MicrophoneSpectrum(unavailable(JobId(73_090))));
    runtime
        .parakeet_pending_events
        .push_back(ImportEvent::MicrophoneSpectrum(unavailable(session.job_id)));
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut available = false;
    for _ in 0..120 {
        runtime
            .unified_sessions
            .get_mut(&session)
            .unwrap()
            .source
            .enqueue_test_block(vec![0.1; 320]);
        assert!(runtime.poll_unified_sessions().is_none());
        let spectra: Vec<_> = runtime
            .parakeet_pending_events
            .iter()
            .filter_map(|event| match event {
                ImportEvent::MicrophoneSpectrum(update) => Some(update),
                _ => None,
            })
            .collect();
        if spectra.iter().any(|update| matches!(update, MicrophoneSpectrumUpdate::Available(spectrum) if spectrum.job_id == session.job_id)) {
            assert_eq!(spectra.len(), 2); assert_eq!(spectra[0].owner(), (JobId(73_090), session.generation)); assert_eq!(spectra[1].owner(), (session.job_id, session.generation)); available = true; break;
        }
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(10));
    }
    assert!(available, "active probe must publish a spectrum");
    runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .stop_and_reap()
        .unwrap();
}

#[test]
fn unified_poll_finished_selection_requires_each_guard() {
    for (id, blocked) in [
        (73_091, "open"),
        (73_092, "pending_call"),
        (73_093, "pending_prepared"),
        (73_094, "reserved"),
        (73_095, "unprocessed"),
    ] {
        let (mut runtime, request) =
            unified_poll_import_fixture("unified-poll-finished-guards", id);
        let session = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: request.job_id,
            generation: request.generation,
        };
        let call = whisper_core::parakeet_scheduler::ScheduledCall {
            session,
            incarnation: 0,
            request_id: 0,
            attempt: 0,
            work: whisper_core::parakeet_scheduler::ScheduledWork::Final { sequence: 1 },
        };
        let state = runtime.unified_sessions.get_mut(&session).unwrap();
        state.available_samples = 0;
        match blocked {
            "open" => state.source_closed = false,
            "pending_call" => state.pending_call = Some(call),
            "pending_prepared" => {
                state.pending_prepared = Some((
                    call,
                    whisper_core::parakeet_session::AudioWindow {
                        sequence: 1,
                        central_start_sample: 0,
                        central_end_sample: 1,
                        native_start_sample: 0,
                        native_end_sample: 1,
                    },
                    vec![0],
                    true,
                ))
            }
            "unprocessed" => state.available_samples = 1,
            _ => {}
        }
        if blocked == "reserved" {
            runtime.parakeet_scheduler.queue_final(session, 1).unwrap();
            assert!(runtime.parakeet_scheduler.next_call().unwrap().is_some());
        }
        // Prevent downstream selection from consuming a deliberately inconsistent state.
        runtime.controlled_hold_reserved_call_once = true;
        assert!(runtime.poll_unified_sessions().is_none(), "{blocked}");
        assert!(runtime.unified_sessions.contains_key(&session), "{blocked}");
        assert!(
            runtime.unified_sessions[&session].failure.is_none(),
            "{blocked}"
        );
        assert!(runtime.document_drains.is_empty(), "{blocked}");
        runtime
            .unified_executor
            .as_mut()
            .unwrap()
            .stop_and_reap()
            .unwrap();
    }
}

#[test]
fn unified_poll_restart_rejects_unreaped_inflight_executor() {
    let (mut runtime, request) =
        unified_poll_import_fixture("unified-poll-restart-inflight", 73_096);
    let mut executor = UnifiedInferenceExecutor::start(controlled_worker().0).unwrap();
    executor.stop_and_reap().unwrap();
    // Isolate the inconsistent in-flight flag when no thread is present.
    executor.in_flight = true;
    runtime.unified_executor = Some(executor);
    let outcome = runtime.poll_unified_sessions().unwrap().unwrap_err();
    assert_eq!((outcome.0, outcome.1), (request.job_id, request.generation));
    assert_eq!(
        outcome.2,
        "InferenceRestart: InferenceRestart: previous instance has not exited"
    );
    assert_eq!(runtime.unified_sessions.len(), 1);
    assert!(runtime.parakeet_scheduler.in_flight().is_none());
    assert!(runtime.unified_executor.as_ref().unwrap().in_flight);
}

#[test]
fn unified_poll_dispatch_failure_records_final_and_preview_state() {
    for (id, final_work) in [(73_097, true), (73_098, false)] {
        let (mut runtime, request) = unified_poll_import_fixture("unified-poll-dispatch-full", id);
        let session = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: request.job_id,
            generation: request.generation,
        };
        if final_work {
            runtime.parakeet_scheduler.queue_final(session, 1).unwrap();
        } else {
            runtime
                .parakeet_scheduler
                .queue_preview(session, 64_000)
                .unwrap();
        }
        let call = runtime.parakeet_scheduler.next_call().unwrap().unwrap();
        let window = whisper_core::parakeet_session::AudioWindow {
            sequence: 1,
            central_start_sample: 0,
            central_end_sample: 1_600,
            native_start_sample: 0,
            native_end_sample: 1_600,
        };
        runtime
            .unified_sessions
            .get_mut(&session)
            .unwrap()
            .pending_prepared = Some((call, window, vec![0; 1_600], final_work));
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
        let mut dummy = call;
        dummy.session.job_id = JobId(id + 1_000);
        let command = |call| UnifiedInferenceCommand {
            call,
            window,
            pcm: vec![0; 1_600],
            scratch_directory: PathBuf::from(&request.destination),
        };
        executor.dispatch(command(dummy)).unwrap();
        calls.recv_timeout(Duration::from_secs(10)).unwrap();
        // Queue a second command while the first is held, then isolate the full-channel failure.
        executor.in_flight = false;
        dummy.session.job_id = JobId(id + 2_000);
        executor.dispatch(command(dummy)).unwrap();
        executor.in_flight = false;
        runtime.controlled_transcriber = None;
        runtime.unified_executor = Some(executor);
        let outcome = runtime.poll_unified_sessions().unwrap().unwrap_err();
        assert_eq!((outcome.0, outcome.1), (session.job_id, session.generation));
        assert_eq!(outcome.2, "InferenceDispatch: sending on a full channel");
        let state = &runtime.unified_sessions[&session];
        assert_eq!(state.failure.as_deref(), Some(outcome.2.as_str()));
        assert!(state.pending_call.is_none());
        assert_eq!(state.pending_window.is_some(), !final_work);
        assert_eq!(state.pending_available_samples.is_some(), !final_work);
        release.send(()).unwrap();
        calls.recv_timeout(Duration::from_secs(10)).unwrap();
        let executor = runtime.unified_executor.as_mut().unwrap();
        let _ = executor
            .results
            .recv_timeout(Duration::from_secs(10))
            .unwrap();
        release.send(()).unwrap();
        let _ = executor
            .results
            .recv_timeout(Duration::from_secs(10))
            .unwrap();
        executor.stop_and_reap().unwrap();
    }
}

#[test]
fn unified_poll_ready_gpu_failure_never_falls_back_for_final_or_preview() {
    for (id, final_work) in [(73_100, true), (73_101, false)] {
        let (mut runtime, request) =
            unified_poll_import_fixture("unified-poll-fallback-after-ready", id);
        let session = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: request.job_id,
            generation: request.generation,
        };
        if final_work {
            runtime.parakeet_scheduler.queue_final(session, 1).unwrap();
        } else {
            runtime
                .parakeet_scheduler
                .queue_preview(session, 64_000)
                .unwrap();
        }
        let call = runtime.parakeet_scheduler.next_call().unwrap().unwrap();
        let window = whisper_core::parakeet_session::AudioWindow {
            sequence: 1,
            central_start_sample: 0,
            central_end_sample: 1_600,
            native_start_sample: 0,
            native_end_sample: 1_600,
        };
        let mut route = controlled_native_route(Path::new(&request.destination));
        route.compute = ComputeChoice::Gpu;
        route.requested_compute = ComputeChoice::Auto;
        let state = runtime.unified_sessions.get_mut(&session).unwrap();
        state.native_route = Some(route);
        state.backend_ready_reported = true;
        state.pending_prepared = Some((call, window, vec![0; 1_600], final_work));
        let starts = Arc::new(AtomicU64::new(0));
        let stops = Arc::new(AtomicU64::new(0));
        let worker = StartFailsOnceUnifiedWorker {
            starts: Arc::clone(&starts),
            stops: Arc::clone(&stops),
        };
        let executor = UnifiedInferenceExecutor::start(Box::new(worker)).unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while !executor.thread.as_ref().unwrap().is_finished() {
            assert!(Instant::now() < deadline);
            thread::yield_now();
        }
        runtime.controlled_transcriber = None;
        runtime.unified_executor = Some(executor);
        let outcome = runtime.poll_unified_sessions().unwrap().unwrap_err();
        assert_eq!((outcome.0, outcome.1), (session.job_id, session.generation));
        assert_eq!(outcome.2, "controlled READY attestation failure");
        assert_eq!(starts.load(Ordering::Relaxed), 1);
        let state = &runtime.unified_sessions[&session];
        assert_eq!(state.failure.as_deref(), Some(outcome.2.as_str()));
        assert_eq!(
            state.native_route.as_ref().unwrap().compute,
            ComputeChoice::Gpu
        );
        assert!(runtime.parakeet_scheduler.in_flight().is_none());
        runtime
            .unified_executor
            .as_mut()
            .unwrap()
            .stop_and_reap()
            .unwrap();
    }
}

#[test]
fn unified_poll_idle_failure_restores_state_when_scheduler_reap_rejects() {
    let (mut runtime, request) =
        unified_poll_import_fixture("unified-poll-idle-reap-error", 73_102);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    runtime.unified_sessions.get_mut(&session).unwrap().failure =
        Some("controlled idle failure".into());
    let mut executor = UnifiedInferenceExecutor::start(controlled_worker().0).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !executor.is_ready().unwrap() {
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    runtime.controlled_transcriber = None;
    runtime.unified_executor = Some(executor);
    let outcome = runtime.poll_unified_sessions().unwrap().unwrap_err();
    assert_eq!((outcome.0, outcome.1), (session.job_id, session.generation));
    assert_eq!(
        outcome.2,
        "controlled idle failure; SchedulerReap: NoRecyclePending"
    );
    assert_eq!(
        runtime.unified_sessions[&session].failure.as_deref(),
        Some("controlled idle failure")
    );
    assert!(runtime.unified_executor.as_ref().unwrap().thread.is_none());
    assert!(runtime.parakeet_scheduler.in_flight().is_none());
}

#[test]
fn unified_poll_idle_failure_restores_state_when_executor_reap_panics() {
    let (mut runtime, request) =
        unified_poll_import_fixture("unified-poll-idle-reap-panic", 73_103);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    runtime.unified_sessions.get_mut(&session).unwrap().failure =
        Some("controlled idle failure".into());
    let executor = UnifiedInferenceExecutor::start(Box::new(UnifiedPanickingInitializer)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !executor.thread.as_ref().unwrap().is_finished() {
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    runtime.controlled_transcriber = None;
    runtime.unified_executor = Some(executor);
    let outcome = runtime.poll_unified_sessions().unwrap().unwrap_err();
    assert_eq!((outcome.0, outcome.1), (session.job_id, session.generation));
    assert_eq!(
        outcome.2,
        "controlled idle failure; InferenceReap: InferenceReap: worker thread panicked"
    );
    assert_eq!(
        runtime.unified_sessions[&session].failure.as_deref(),
        Some("controlled idle failure")
    );
    assert!(runtime.unified_executor.as_ref().unwrap().thread.is_none());
}

#[test]
fn unified_poll_panic_appends_peer_scheduler_abort_error_and_releases_both() {
    let (mut runtime, first) =
        unified_poll_import_fixture("unified-poll-panic-missing-peer", 73_104);
    let root = PathBuf::from(&first.destination)
        .parent()
        .unwrap()
        .to_path_buf();
    let peer = unified_import_request(&root.join("peer"), Path::new(&first.source_path), 73_105);
    runtime.start_controlled_unified_import(&peer).unwrap();
    let first_id = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: first.job_id,
        generation: first.generation,
    };
    let peer_id = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: peer.job_id,
        generation: peer.generation,
    };
    // Deliberately retain the peer runtime state after removing scheduler membership.
    runtime.parakeet_scheduler.abort_session(peer_id).unwrap();
    runtime.parakeet_scheduler.confirm_child_exited().unwrap();
    let mut executor = UnifiedInferenceExecutor::start(controlled_worker().0).unwrap();
    executor.stop_and_reap().unwrap();
    executor.panic_during_transcription_for = Some(first_id);
    executor.restart().unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !executor.is_ready().unwrap() {
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    runtime.controlled_transcriber = None;
    runtime.unified_executor = Some(executor);
    assert!(runtime.poll_unified_sessions().is_none());
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
    assert_eq!(
        (outcome.0, outcome.1),
        (first_id.job_id, first_id.generation)
    );
    assert!(
        outcome.2.contains("; SchedulerAbort: UnknownSession"),
        "{}",
        outcome.2
    );
    assert!(
        !runtime.unified_sessions.contains_key(&first_id)
            && !runtime.unified_sessions.contains_key(&peer_id)
    );
    assert!(
        !runtime.unified_archives.contains_key(&first_id)
            && !runtime.unified_archives.contains_key(&peer_id)
    );
    assert!(runtime.parakeet_pending_events.iter().any(|event| matches!(event, ImportEvent::Failed { job_id, generation, .. } if *job_id == peer_id.job_id && *generation == peer_id.generation)));
}

fn characterize_stale_result(
    stale_field: &str,
    trace: &mut Vec<String>,
    transitions: &mut Vec<String>,
) {
    let (mut runtime, request) = unified_poll_import_fixture("unified-poll-stale", 73_005);
    let archive = PathBuf::from(&request.destination);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let document = start_trace_diagnostics(&mut runtime, &archive, session);
    runtime.parakeet_scheduler.queue_final(session, 0).unwrap();
    let call = runtime.parakeet_scheduler.next_call().unwrap().unwrap();
    runtime.parakeet_scheduler.begin_call(call).unwrap();
    runtime
        .unified_sessions
        .get_mut(&session)
        .unwrap()
        .pending_call = Some(call);
    let mut executor = UnifiedInferenceExecutor::start(controlled_worker().0).unwrap();
    executor
        .dispatch(UnifiedInferenceCommand {
            call,
            window: whisper_core::parakeet_session::AudioWindow {
                sequence: 0,
                central_start_sample: 0,
                central_end_sample: 1600,
                native_start_sample: 0,
                native_end_sample: 1600,
            },
            pcm: vec![1000_i16; 1600],
            scratch_directory: PathBuf::from(&request.destination),
        })
        .unwrap();
    let mut result = executor
        .results
        .recv_timeout(Duration::from_secs(5))
        .unwrap();
    match stale_field {
        "attempt" => result.call.attempt += 1,
        "request" => result.call.request_id += 1,
        "incarnation" => result.call.incarnation += 1,
        "pending" => {
            runtime
                .unified_sessions
                .get_mut(&session)
                .unwrap()
                .pending_call = None
        }
        "scheduler" => {
            runtime.parakeet_scheduler.fail_call(call).unwrap();
        }
        "unknown_job" => result.call.session.job_id = JobId(73_006),
        "unknown_generation" => result.call.session.generation = session.generation.next().unwrap(),
        _ => unreachable!(),
    }
    let returned_session = result.call.session;
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    assert!(sender.send(result).is_ok());
    executor.results = receiver;
    runtime.unified_executor = Some(executor);
    trace.push(format!(
        "{stale_field} before {}",
        unified_fallback_snapshot(&runtime)
    ));
    transitions.push(format!("fixture stale field {stale_field}"));
    let outcome = record_import_poll(&mut runtime, &document, transitions);
    trace.push(format!(
        "{stale_field} outcome {}",
        serde_json::to_string(&outcome).unwrap()
    ));
    let expected_message = if stale_field.starts_with("unknown_") {
        "SchedulerResponse: result belongs to an unknown session"
    } else {
        "SchedulerResponse: stale inference result"
    };
    assert!(
        matches!(outcome, Some(Err((job, generation, ref message)))
        if job == returned_session.job_id && generation == returned_session.generation
            && message == expected_message),
        "{stale_field}"
    );
    assert!(
        runtime.unified_sessions.contains_key(&session),
        "{stale_field}"
    );
    assert!(runtime.parakeet_pending_events.is_empty(), "{stale_field}");
    trace.push(format!(
        "{stale_field} after {}",
        unified_fallback_snapshot(&runtime)
    ));
    runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .stop_and_reap()
        .unwrap();
    assert_empty_diagnostics_after_join(&mut runtime, &archive);
}

#[test]
fn unified_poll_gpu_fallback_reports_missing_cpu_route_for_final_and_preview() {
    for (id, final_work) in [(73_112, true), (73_113, false)] {
        let (mut runtime, request) =
            unified_poll_import_fixture("unified-poll-cpu-route-missing", id);
        let session = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: request.job_id,
            generation: request.generation,
        };
        if final_work {
            runtime.parakeet_scheduler.queue_final(session, 1).unwrap();
        } else {
            runtime
                .parakeet_scheduler
                .queue_preview(session, 64_000)
                .unwrap();
        }
        let call = runtime.parakeet_scheduler.next_call().unwrap().unwrap();
        let window = whisper_core::parakeet_session::AudioWindow {
            sequence: 1,
            central_start_sample: 0,
            central_end_sample: 1_600,
            native_start_sample: 0,
            native_end_sample: 1_600,
        };
        let mut route = controlled_native_route(Path::new(&request.destination));
        route.compute = ComputeChoice::Gpu;
        route.requested_compute = ComputeChoice::Auto;
        let state = runtime.unified_sessions.get_mut(&session).unwrap();
        state.request.config.compute = ComputeChoice::Auto;
        state.native_route = Some(route);
        state.pending_prepared = Some((call, window, vec![0; 1_600], final_work));
        let starts = Arc::new(AtomicU64::new(0));
        let stops = Arc::new(AtomicU64::new(0));
        let executor = UnifiedInferenceExecutor::start(Box::new(StartFailsOnceUnifiedWorker {
            starts: Arc::clone(&starts),
            stops,
        }))
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while !executor.thread.as_ref().unwrap().is_finished() {
            assert!(Instant::now() < deadline);
            thread::yield_now();
        }
        runtime.controlled_transcriber = None;
        runtime.unified_executor = Some(executor);
        let outcome = runtime.poll_unified_sessions().unwrap().unwrap_err();
        assert_eq!((outcome.0, outcome.1), (session.job_id, session.generation));
        assert_eq!(
            outcome.2,
            "controlled READY attestation failure; AutoFallback: GPU pre-audio startup failed: controlled READY attestation failure; CPU fallback route unavailable: ModelNotQualified: personal Parakeet CPU worker is absent"
        );
        assert_eq!(starts.load(Ordering::Relaxed), 1);
        assert_eq!(
            runtime.unified_sessions[&session].failure.as_deref(),
            Some(outcome.2.as_str())
        );
        assert!(runtime.parakeet_scheduler.in_flight().is_none());
        runtime
            .unified_executor
            .as_mut()
            .unwrap()
            .stop_and_reap()
            .unwrap();
    }
}

#[test]
fn unified_poll_propagates_result_instance_without_extra_identity_check() {
    let (mut runtime, _request) =
        unified_poll_import_fixture("unified-poll-instance-characterization", 73_116);
    let mut executor = UnifiedInferenceExecutor::start(controlled_worker().0).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !executor.is_ready().unwrap() {
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    runtime.controlled_transcriber = None;
    runtime.unified_executor = Some(executor);
    assert!(runtime.poll_unified_sessions().is_none());
    let executor = runtime.unified_executor.as_mut().unwrap();
    let mut result = executor
        .results
        .recv_timeout(Duration::from_secs(10))
        .unwrap();
    // Invalid injected identity documents current behavior; it is not a production claim.
    assert_eq!(result.instance_id, 9001);
    result.instance_id = 9002;
    let (send, receive) = std::sync::mpsc::sync_channel(1);
    assert!(send.send(result).is_ok());
    executor.results = receive;
    assert!(runtime.poll_unified_sessions().is_none());
    assert!(runtime.poll_unified_sessions().is_none());
    let mut segment_instance = None;
    while let Some(event) = runtime.poll_unified_document() {
        if let ImportEvent::Segment(segment) = event.unwrap() {
            segment_instance = Some(segment.instance_id);
        }
    }
    assert_eq!(segment_instance, Some(9002));
}

#[test]
fn unified_poll_capture_ack_keeps_existing_higher_available_boundary() {
    let root = test_artifact_root("unified-poll-monotone-ack");
    std::fs::create_dir_all(&root).unwrap();
    let request = unified_live_request(&root, 73_117);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let capture = CaptureStream::empty_test_capture(16_000, 1, "test:monotone-ack");
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: PathBuf::from("unused-cpu"),
            gpu: PathBuf::from("unused-gpu"),
        },
        Arc::new(PublicationGate::default()),
    )
    .with_controlled_transcriber(controlled_worker().0);
    runtime
        .start_unified_capture(&request, capture, None, None)
        .unwrap();
    // Artificially advanced available count isolates the monotone max operation.
    runtime
        .unified_sessions
        .get_mut(&session)
        .unwrap()
        .available_samples = 100_000;
    runtime
        .unified_sessions
        .get_mut(&session)
        .unwrap()
        .source
        .enqueue_test_block(vec![0.1; 16_000]);
    let deadline = Instant::now() + Duration::from_secs(10);
    while runtime.unified_sessions[&session].durable_samples == 0 {
        assert!(runtime.poll_unified_sessions().is_none());
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    let state = &runtime.unified_sessions[&session];
    assert_eq!(state.available_samples, 100_000);
    assert!(state.durable_samples < state.available_samples);
    assert!(runtime.parakeet_pending_events.iter().any(|event| matches!(event, ImportEvent::ParakeetCaptureProgress { job_id, generation, captured_samples, audio_durable_samples, .. } if *job_id == session.job_id && *generation == session.generation && *captured_samples == state.available_samples && *audio_durable_samples == state.durable_samples)));
    runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .stop_and_reap()
        .unwrap();
}

#[test]
fn unified_poll_ignores_final_queue_error_for_terminal_scheduler_state() {
    let (mut runtime, request) =
        unified_poll_import_fixture("unified-poll-final-queue-terminal", 73_118);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    // Deliberately terminate scheduler membership while runtime still has work.
    runtime.parakeet_scheduler.abort_session(session).unwrap();
    assert!(runtime.poll_unified_sessions().is_none());
    assert!(runtime.unified_sessions.contains_key(&session));
    assert!(runtime.unified_sessions[&session].failure.is_none());
    assert!(runtime.parakeet_scheduler.in_flight().is_none());
    assert!(runtime.parakeet_pending_events.is_empty());
    runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .stop_and_reap()
        .unwrap();
}

#[test]
fn unified_poll_empty_closed_source_retains_finalization_failure() {
    let (mut runtime, request) = unified_poll_import_fixture("unified-poll-empty-final", 73_119);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    // A closed zero-length view isolates actual finalizer validation.
    runtime
        .unified_sessions
        .get_mut(&session)
        .unwrap()
        .available_samples = 0;
    assert!(runtime.poll_unified_sessions().is_none());
    assert_eq!(
        runtime.unified_sessions[&session].failure.as_deref(),
        Some("InvalidInput: empty Parakeet source")
    );
    assert!(runtime.document_drains.is_empty());
    runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .stop_and_reap()
        .unwrap();
}

#[test]
fn unified_poll_preview_requires_boundary_to_exceed_previous_sequence() {
    for (id, extra) in [(73_120, 0_u64), (73_121, 1_u64)] {
        let root = test_artifact_root("unified-poll-preview-monotone");
        std::fs::create_dir_all(&root).unwrap();
        let request = unified_live_request(&root, id);
        let session = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: request.job_id,
            generation: request.generation,
        };
        let capture = CaptureStream::empty_test_capture(16_000, 1, "test:preview-monotone");
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(controlled_worker().0)
        .with_child_observer(Arc::new(Mutex::new(Vec::new())));
        let ready_gate = runtime.hold_next_child_ready();
        runtime
            .start_unified_capture(&request, capture, None, None)
            .unwrap();
        let mut reference =
            CaptureStream::from_test_blocks(16_000, 1, vec![vec![0.1; 70_000], vec![0.1; 640]]);
        let mut converter = crate::capture::Pcm16Converter::new(16_000, 1).unwrap();
        let first = converter
            .push(&reference.try_next_block().unwrap())
            .unwrap()
            .len() as u64;
        let second = first
            + converter
                .push(&reference.try_next_block().unwrap())
                .unwrap()
                .len() as u64;
        assert!(first >= 64_000 && second > first + extra);
        // Seed equality or a larger previous sequence to isolate the guard.
        runtime
            .unified_sessions
            .get_mut(&session)
            .unwrap()
            .preview_sequence = first + extra;
        runtime
            .unified_sessions
            .get_mut(&session)
            .unwrap()
            .source
            .enqueue_test_block(vec![0.1; 70_000]);
        let deadline = Instant::now() + Duration::from_secs(10);
        while runtime.unified_sessions[&session].available_samples < first {
            assert!(runtime.poll_unified_sessions().is_none());
            assert!(Instant::now() < deadline);
            thread::yield_now();
        }
        assert_eq!(
            runtime.unified_sessions[&session].preview_sequence,
            first + extra
        );
        assert!(runtime.parakeet_scheduler.in_flight().is_none());
        runtime
            .unified_sessions
            .get_mut(&session)
            .unwrap()
            .source
            .enqueue_test_block(vec![0.1; 640]);
        while runtime.unified_sessions[&session].preview_sequence < second {
            assert!(runtime.poll_unified_sessions().is_none());
            assert!(Instant::now() < deadline);
            thread::yield_now();
        }
        assert_eq!(runtime.unified_sessions[&session].preview_sequence, second);
        assert!(runtime.parakeet_scheduler.in_flight().is_some());
        let (released, changed) = &*ready_gate;
        *released.lock().unwrap() = true;
        changed.notify_all();
        runtime
            .unified_executor
            .as_mut()
            .unwrap()
            .stop_and_reap()
            .unwrap();
    }
}

#[test]
fn unified_poll_invalid_preview_words_trigger_commit_cleanup() {
    let (mut runtime, request) =
        unified_poll_import_fixture("unified-poll-preview-invalid-words", 73_122);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    runtime
        .parakeet_scheduler
        .queue_preview(session, 64_000)
        .unwrap();
    let call = runtime.parakeet_scheduler.next_call().unwrap().unwrap();
    runtime.parakeet_scheduler.begin_call(call).unwrap();
    let window = whisper_core::parakeet_session::AudioWindow {
        sequence: 64_000,
        central_start_sample: 0,
        central_end_sample: 64_000,
        native_start_sample: 0,
        native_end_sample: 64_000,
    };
    let state = runtime.unified_sessions.get_mut(&session).unwrap();
    state.pending_call = Some(call);
    state.pending_window = Some(window);
    state.pending_available_samples = Some(64_000);
    let mut executor = UnifiedInferenceExecutor::start(Box::new(UnifiedInvalidWordWorker)).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !executor.is_ready().unwrap() {
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    executor
        .dispatch(UnifiedInferenceCommand {
            call,
            window,
            pcm: vec![0; 64_000],
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
    assert_eq!(
        outcome.2,
        "InvalidTimestamp: word 0 is invalid or unordered"
    );
    assert!(!runtime.unified_sessions.contains_key(&session));
    assert!(runtime.parakeet_scheduler.in_flight().is_none());
    assert!(runtime.unified_executor.as_ref().unwrap().thread.is_none());
}

#[test]
fn unified_poll_final_source_error_cancels_reservation_and_retains_failure() {
    use crate::parakeet_pipeline::ReadablePcmSource;
    let (mut runtime, request) =
        unified_poll_import_fixture("unified-poll-final-source-error", 73_123);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let missing = PathBuf::from(&request.destination).join("missing-audio.wav");
    let mut oracle = crate::parakeet_audio::PcmPartStore::open(&missing);
    let expected = ReadablePcmSource::read_window(&mut oracle, 0..48_000).unwrap_err();
    // A missing source is deliberate; it does not alter the installed audio.
    runtime.unified_sessions.get_mut(&session).unwrap().source =
        UnifiedSessionSource::Closed(crate::parakeet_audio::PcmPartStore::open(&missing));
    let outcome = runtime.poll_unified_sessions().unwrap().unwrap_err();
    assert_eq!((outcome.0, outcome.1), (session.job_id, session.generation));
    assert_eq!(outcome.2, expected);
    assert_eq!(
        runtime.unified_sessions[&session].failure.as_deref(),
        Some(expected.as_str())
    );
    assert!(runtime.parakeet_scheduler.in_flight().is_none());
    runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .stop_and_reap()
        .unwrap();
}

#[test]
fn unified_poll_preview_source_error_cancels_reservation_and_retains_failure() {
    use crate::parakeet_pipeline::ReadablePcmSource;
    let (mut runtime, request) =
        unified_poll_import_fixture("unified-poll-preview-source-error", 73_124);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let missing = PathBuf::from(&request.destination).join("missing-audio.wav");
    let mut oracle = crate::parakeet_audio::PcmPartStore::open(&missing);
    let expected = ReadablePcmSource::read_window(&mut oracle, 0..64_000).unwrap_err();
    // A missing source is deliberate; it does not alter the installed audio.
    runtime.unified_sessions.get_mut(&session).unwrap().source =
        UnifiedSessionSource::Closed(crate::parakeet_audio::PcmPartStore::open(&missing));
    let state = runtime.unified_sessions.get_mut(&session).unwrap();
    state.source_closed = false;
    state.available_samples = 64_000;
    runtime
        .parakeet_scheduler
        .queue_preview(session, 64_000)
        .unwrap();
    let outcome = runtime.poll_unified_sessions().unwrap().unwrap_err();
    assert_eq!((outcome.0, outcome.1), (session.job_id, session.generation));
    assert_eq!(outcome.2, expected);
    assert_eq!(
        runtime.unified_sessions[&session].failure.as_deref(),
        Some(expected.as_str())
    );
    assert!(runtime.parakeet_scheduler.in_flight().is_none());
    runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .stop_and_reap()
        .unwrap();
}

#[test]
fn unified_poll_defers_capture_pump_error_while_executor_inflight() {
    let root = test_artifact_root("unified-poll-pump-error");
    std::fs::create_dir_all(&root).unwrap();
    let request = unified_live_request(&root, 73_126);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let capture = CaptureStream::empty_test_capture(16_000, 2, "test:pump-error");
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: PathBuf::from("unused-cpu"),
            gpu: PathBuf::from("unused-gpu"),
        },
        Arc::new(PublicationGate::default()),
    )
    .with_controlled_transcriber(controlled_worker().0);
    runtime
        .start_unified_capture(&request, capture, None, None)
        .unwrap();
    let mut executor = UnifiedInferenceExecutor::start(controlled_worker().0).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !executor.is_ready().unwrap() {
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    // Simulate an in-flight call while providing a malformed partial stereo frame.
    executor.in_flight = true;
    runtime.controlled_transcriber = None;
    runtime.unified_executor = Some(executor);
    runtime
        .unified_sessions
        .get_mut(&session)
        .unwrap()
        .source
        .enqueue_test_block(vec![0.1; 321]);
    assert!(runtime.poll_unified_sessions().is_none());
    assert_eq!(
        runtime.unified_sessions[&session].failure.as_deref(),
        Some("CaptureInterrupted: stream format changed or delivered a partial frame")
    );
    assert!(runtime.parakeet_pending_events.is_empty());
    let executor = runtime.unified_executor.as_mut().unwrap();
    executor.in_flight = false;
    executor.stop_and_reap().unwrap();
}

#[test]
fn unified_poll_panic_appends_capture_stop_error_from_partial_frame() {
    let root = test_artifact_root("unified-poll-panic-capture-stop-error");
    std::fs::create_dir_all(&root).unwrap();
    let first = unified_live_request(&root.join("first"), 73_127);
    let source = root.join("peer.wav");
    let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source).unwrap();
    writer.append(&vec![1000_i16; 48_000]).unwrap();
    writer.finish().unwrap();
    let peer = unified_import_request(&root.join("peer"), &source, 73_128);
    let first_id = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: first.job_id,
        generation: first.generation,
    };
    let peer_id = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: peer.job_id,
        generation: peer.generation,
    };
    let capture = CaptureStream::empty_test_capture(16_000, 2, "test:panic-stop-error");
    let (sent, calls) = std::sync::mpsc::sync_channel(1);
    let (release, released) = std::sync::mpsc::sync_channel(1);
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: PathBuf::from("unused-cpu"),
            gpu: PathBuf::from("unused-gpu"),
        },
        Arc::new(PublicationGate::default()),
    )
    .with_controlled_transcriber(Box::new(UnifiedHeldPanicWorker {
        sent,
        release: released,
    }));
    runtime
        .start_unified_capture(&first, capture, None, None)
        .unwrap();
    runtime
        .unified_sessions
        .get_mut(&first_id)
        .unwrap()
        .source
        .enqueue_test_block(vec![0.1; 140_000]);
    let deadline = Instant::now() + Duration::from_secs(10);
    while calls.try_recv().is_err() {
        assert!(runtime.poll_unified_sessions().is_none());
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    runtime.start_controlled_unified_import(&peer).unwrap();
    release.send(()).unwrap();
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
    // The poll consumes 32 blocks; the partial stereo frame remains for stop_capture.
    for _ in 0..32 {
        runtime
            .unified_sessions
            .get_mut(&first_id)
            .unwrap()
            .source
            .enqueue_test_block(vec![0.1; 640]);
    }
    runtime
        .unified_sessions
        .get_mut(&first_id)
        .unwrap()
        .source
        .enqueue_test_block(vec![0.1; 321]);
    let outcome = runtime.poll_unified_sessions().unwrap().unwrap_err();
    assert_eq!(
        (outcome.0, outcome.1),
        (first_id.job_id, first_id.generation)
    );
    assert!(
        outcome.2.contains(
            "stop: CaptureInterrupted: stream format changed or delivered a partial frame"
        ),
        "{}",
        outcome.2
    );
    assert!(
        !runtime.unified_sessions.contains_key(&first_id)
            && !runtime.unified_sessions.contains_key(&peer_id)
    );
    assert!(runtime.parakeet_pending_events.iter().any(|event| matches!(event, ImportEvent::Failed { job_id, generation, .. } if *job_id == peer_id.job_id && *generation == peer_id.generation)));
}

#[test]
fn unified_poll_capture_ack_clamps_artificially_excessive_durable_count() {
    let root = test_artifact_root("unified-poll-durable-clamp");
    std::fs::create_dir_all(&root).unwrap();
    let request = unified_live_request(&root, 73_129);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let capture = CaptureStream::empty_test_capture(16_000, 1, "test:durable-clamp");
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: PathBuf::from("unused-cpu"),
            gpu: PathBuf::from("unused-gpu"),
        },
        Arc::new(PublicationGate::default()),
    )
    .with_controlled_transcriber(controlled_worker().0);
    runtime
        .start_unified_capture(&request, capture, None, None)
        .unwrap();
    // Deliberately violate the durable <= available invariant to characterize the clamp.
    let state = runtime.unified_sessions.get_mut(&session).unwrap();
    state.available_samples = 100_000;
    state.durable_samples = 200_000;
    state.source.enqueue_test_block(vec![0.1; 16_000]);
    let deadline = Instant::now() + Duration::from_secs(10);
    while runtime.unified_sessions[&session].durable_samples == 200_000 {
        assert!(runtime.poll_unified_sessions().is_none());
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    let state = &runtime.unified_sessions[&session];
    assert_eq!(
        (state.available_samples, state.durable_samples),
        (100_000, 100_000)
    );
    assert!(!runtime.parakeet_pending_events.iter().any(|event| matches!(event, ImportEvent::ParakeetCaptureProgress { job_id, generation, .. } if *job_id == session.job_id && *generation == session.generation)));
    runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .stop_and_reap()
        .unwrap();
}

#[test]
fn unified_poll_unchanged_capture_ack_does_not_emit_progress() {
    let root = test_artifact_root("unified-poll-unchanged-ack");
    std::fs::create_dir_all(&root).unwrap();
    let request = unified_live_request(&root, 73_130);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let capture = CaptureStream::empty_test_capture(16_000, 1, "test:unchanged-ack");
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: PathBuf::from("unused-cpu"),
            gpu: PathBuf::from("unused-gpu"),
        },
        Arc::new(PublicationGate::default()),
    )
    .with_controlled_transcriber(controlled_worker().0);
    runtime
        .start_unified_capture(&request, capture, None, None)
        .unwrap();
    let mut reference = CaptureStream::from_test_blocks(16_000, 1, vec![vec![0.1; 16_000]]);
    let mut converter = crate::capture::Pcm16Converter::new(16_000, 1).unwrap();
    let expected = converter
        .push(&reference.try_next_block().unwrap())
        .unwrap()
        .len() as u64;
    // Seed the exact future boundary; the gain event witnesses that ACK processing occurred.
    let state = runtime.unified_sessions.get_mut(&session).unwrap();
    state.available_samples = expected;
    state.durable_samples = expected;
    state.source.enqueue_test_block(vec![0.1; 16_000]);
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        assert!(runtime.poll_unified_sessions().is_none());
        if runtime.parakeet_pending_events.iter().any(|event| matches!(event, ImportEvent::MicrophoneGainApplied { job_id, generation, .. } if *job_id == session.job_id && *generation == session.generation)) { break; }
        assert!(Instant::now() < deadline);
        thread::yield_now();
    }
    assert!(!runtime.parakeet_pending_events.iter().any(|event| matches!(event, ImportEvent::ParakeetCaptureProgress { job_id, generation, .. } if *job_id == session.job_id && *generation == session.generation)));
    runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .stop_and_reap()
        .unwrap();
}

struct UnifiedStopFailureWorker;
impl crate::parakeet_pipeline::WindowTranscriber for UnifiedStopFailureWorker {
    fn instance_id(&self) -> u64 {
        9015
    }
    fn transcribe(
        &mut self,
        _: whisper_core::parakeet_session::AudioWindow,
        _: &[i16],
    ) -> Result<Vec<whisper_core::parakeet_ipc::Word>, String> {
        Ok(Vec::new())
    }
    fn stop_instance(&mut self) -> Result<(), String> {
        Err("controlled stop failure".into())
    }
}

#[test]
fn unified_poll_injected_disconnect_or_panic_reports_recycle_failure() {
    for (id, disconnect) in [(73_131, true), (73_132, false)] {
        let (mut runtime, request) =
            unified_poll_import_fixture("unified-poll-injected-recycle-error", id);
        let session = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: request.job_id,
            generation: request.generation,
        };
        runtime.controlled_transcriber = Some(Box::new(UnifiedStopFailureWorker));
        if disconnect {
            runtime.controlled_disconnect_session = Some(session);
        } else {
            runtime.controlled_panic_session = Some(session);
        }
        let outcome = runtime.poll_unified_sessions().unwrap().unwrap_err();
        assert_eq!((outcome.0, outcome.1), (JobId(0), Generation::first()));
        assert_eq!(outcome.2, "controlled stop failure");
        assert!(runtime.unified_executor.is_none());
        assert!(runtime.unified_sessions.contains_key(&session));
        assert!(runtime.parakeet_scheduler.in_flight().is_none());
    }
}

#[test]
fn unified_poll_meter_preserves_same_job_other_generation() {
    let root = test_artifact_root("unified-poll-meter-other-generation");
    std::fs::create_dir_all(&root).unwrap();
    let request = unified_live_request(&root, 73_135);
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let capture = CaptureStream::empty_test_capture(16_000, 1, "test:meter-other-generation");
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: PathBuf::from("unused-cpu"),
            gpu: PathBuf::from("unused-gpu"),
        },
        Arc::new(PublicationGate::default()),
    )
    .with_controlled_transcriber(controlled_worker().0);
    runtime
        .start_unified_capture(&request, capture, None, None)
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let first = loop {
        runtime
            .unified_sessions
            .get_mut(&session)
            .unwrap()
            .source
            .enqueue_test_block(vec![0.1; 320]);
        assert!(runtime.poll_unified_sessions().is_none());
        if let Some(meter) = runtime
            .parakeet_pending_events
            .iter()
            .find_map(|event| match event {
                ImportEvent::MicrophoneMeter(meter) if meter.job_id == session.job_id => {
                    Some(meter.clone())
                }
                _ => None,
            })
        {
            break meter;
        }
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(10));
    };
    let mut other = first.clone();
    other.generation = first.generation.next().unwrap();
    runtime.parakeet_pending_events.clear();
    runtime
        .parakeet_pending_events
        .push_back(ImportEvent::MicrophoneMeter(first.clone()));
    runtime
        .parakeet_pending_events
        .push_back(ImportEvent::MicrophoneMeter(other.clone()));
    runtime
        .parakeet_pending_events
        .push_back(ImportEvent::MicrophoneMeter(first.clone()));
    loop {
        runtime
            .unified_sessions
            .get_mut(&session)
            .unwrap()
            .source
            .enqueue_test_block(vec![0.1; 320]);
        assert!(runtime.poll_unified_sessions().is_none());
        let meters: Vec<_> = runtime
            .parakeet_pending_events
            .iter()
            .filter_map(|event| match event {
                ImportEvent::MicrophoneMeter(meter) => Some(meter),
                _ => None,
            })
            .collect();
        if meters
            .iter()
            .any(|meter| meter.job_id == session.job_id && meter.sequence > first.sequence)
        {
            assert_eq!(meters.len(), 2);
            assert_eq!(meters[0].job_id, other.job_id);
            assert_eq!(meters[1].job_id, session.job_id);
            assert_eq!(meters[0].generation, other.generation);
            assert_eq!(meters[1].generation, session.generation);
            break;
        }
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(10));
    }
    runtime
        .unified_executor
        .as_mut()
        .unwrap()
        .stop_and_reap()
        .unwrap();
}
