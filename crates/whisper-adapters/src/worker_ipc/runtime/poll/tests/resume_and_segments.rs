use super::*;
use whisper_core::{
    ComputeChoice, JobConfig, LanguageChoice, SegmentId, SourceRange, ipc::WorkerSegmentDto,
    parakeet_session::TranscriptionStrategy, ports::ImportRequest,
};

const INSTANCE_ID: u64 = 7;

fn source_range(start_sample: u64, end_sample: u64) -> SourceRange {
    SourceRange::new(start_sample, end_sample, 16_000).expect("valid source range")
}

fn segment_event(job_id: JobId, sequence: u64, range: SourceRange, text: &str) -> WorkerEvent {
    WorkerEvent::Segment(WorkerSegmentDto {
        job_id,
        generation: Generation::first(),
        instance_id: INSTANCE_ID,
        segment_id: SegmentId(sequence),
        range,
        text: text.into(),
    })
}

fn attach_transport(runtime: &mut ImportRuntime, executable: &Path) {
    runtime.transport =
        Some(ChildWorkerTransport::spawn(executable).expect("spawn idle worker transport"));
}

fn import_request(job_id: JobId, compute: ComputeChoice) -> ImportRequest {
    ImportRequest {
        strategy: TranscriptionStrategy::LegacyV1,
        job_id,
        generation: Generation::first(),
        source_path: "source.wav".into(),
        source_sha256: None,
        source_samples: Some(32_000),
        model_path: "model.bin".into(),
        model_sha256: [9; 32],
        destination: "destination".into(),
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute,
        },
    }
}

#[test]
fn poll_consumes_matching_resume_segment_without_emitting_or_consuming_next_event() {
    let root = TestRoot::new("resume-match");
    let executable = compile_idle_worker(&root.0);
    let job_id = JobId(0x99201);
    let first = crate::archive::SegmentRecord {
        sequence: 1,
        range: source_range(0, 8_000),
        text: "first".into(),
    };
    let second = crate::archive::SegmentRecord {
        sequence: 2,
        range: source_range(8_000, 16_000),
        text: "second".into(),
    };
    let mut runtime = runtime_with_active_attempt(job_id);
    runtime
        .resume_prefix
        .extend([first.clone(), second.clone()]);
    attach_transport(&mut runtime, &executable);
    let transport = runtime.transport.as_mut().expect("attached transport");
    transport.inject_event(segment_event(job_id, 1, first.range, &first.text));
    transport.inject_event(WorkerEvent::Progress {
        job_id,
        generation: Generation::first(),
        instance_id: INSTANCE_ID,
        completed_samples: 8_000,
        total_samples: 32_000,
    });

    assert!(
        runtime.poll().is_none(),
        "confirmed prefix is not emitted again"
    );
    assert_eq!(runtime.resume_prefix.front(), Some(&second));
    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::Progress {
            completed_samples: 8_000,
            total_samples: 32_000,
            ..
        }))
    ));
    assert_eq!(runtime.resume_prefix.front(), Some(&second));
}

#[test]
fn poll_cleans_attempt_and_entire_prefix_for_sequence_range_or_text_mismatch() {
    let root = TestRoot::new("resume-mismatch");
    let executable = compile_idle_worker(&root.0);
    let expected_range = source_range(0, 8_000);
    let cases = [
        ("sequence", 2, expected_range, "confirmed"),
        ("range", 1, source_range(1, 8_001), "confirmed"),
        ("text", 1, expected_range, "changed"),
    ];

    for (index, (label, sequence, range, text)) in cases.into_iter().enumerate() {
        let job_id = JobId(0x99210 + index as u128);
        let mut runtime = runtime_with_active_attempt(job_id);
        runtime.resume_prefix.extend([
            crate::archive::SegmentRecord {
                sequence: 1,
                range: expected_range,
                text: "confirmed".into(),
            },
            crate::archive::SegmentRecord {
                sequence: 2,
                range: source_range(8_000, 16_000),
                text: "remaining".into(),
            },
        ]);
        attach_transport(&mut runtime, &executable);
        runtime
            .transport
            .as_mut()
            .expect("attached transport")
            .inject_event(segment_event(job_id, sequence, range, text));

        assert!(
            matches!(
                runtime.poll(),
                Some(Err((actual_job, generation, message)))
                    if actual_job == job_id
                        && generation == Generation::first()
                        && message.contains("ResumeMismatch")
            ),
            "{label} mismatch is rejected"
        );
        assert!(
            runtime.transport.is_none(),
            "{label} mismatch drops transport"
        );
        assert!(runtime.active.is_none(), "{label} mismatch clears attempt");
        assert!(
            runtime.resume_prefix.is_empty(),
            "{label} mismatch clears remaining prefix"
        );
    }
}

#[test]
fn poll_rejects_end_before_reproducing_resume_prefix_and_clears_state() {
    let root = TestRoot::new("resume-early-end");
    let executable = compile_idle_worker(&root.0);
    let job_id = JobId(0x99220);
    let mut runtime = runtime_with_active_attempt(job_id);
    runtime.resume_prefix.extend([
        crate::archive::SegmentRecord {
            sequence: 1,
            range: source_range(0, 8_000),
            text: "first".into(),
        },
        crate::archive::SegmentRecord {
            sequence: 2,
            range: source_range(8_000, 16_000),
            text: "second".into(),
        },
    ]);
    attach_transport(&mut runtime, &executable);
    runtime
        .transport
        .as_mut()
        .expect("attached transport")
        .inject_event(WorkerEvent::End {
            job_id,
            generation: Generation::first(),
            instance_id: INSTANCE_ID,
            last_sequence: 0,
            last_offset: 0,
        });

    assert!(matches!(
        runtime.poll(),
        Some(Err((actual_job, _, message)))
            if actual_job == job_id && message.contains("ended before reproducing")
    ));
    assert!(runtime.transport.is_none());
    assert!(runtime.active.is_none());
    assert!(runtime.resume_prefix.is_empty());
}

#[test]
fn poll_maps_import_segment_into_confirmed_import_event() {
    let root = TestRoot::new("import-segment");
    let executable = compile_idle_worker(&root.0);
    let job_id = JobId(0x99230);
    let range = source_range(16_000, 24_000);
    let mut runtime = runtime_with_active_attempt(job_id);
    attach_transport(&mut runtime, &executable);
    runtime
        .transport
        .as_mut()
        .expect("attached transport")
        .inject_event(segment_event(job_id, 3, range, "import text"));

    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::Segment(segment)))
            if segment.job_id == job_id
                && segment.generation == Generation::first()
                && segment.instance_id == INSTANCE_ID
                && segment.segment_id == SegmentId(3)
                && segment.range == range
                && segment.text == "import text"
    ));
    assert_eq!(runtime.live_pending_persistence, 0);
}

#[test]
fn poll_maps_live_segment_and_tracks_provisional_persistence_and_times() {
    let root = TestRoot::new("live-segment");
    let executable = compile_idle_worker(&root.0);
    let job_id = JobId(0x99240);
    let range = source_range(8_000, 16_000);
    let mut runtime = runtime_with_active_attempt(job_id);
    let mut live = live_capture_state(&root.0, job_id);
    live.segment_sequence_base = 7;
    live.started_at_unix_ms = 500;
    runtime.live = Some(live);
    attach_transport(&mut runtime, &executable);
    runtime
        .transport
        .as_mut()
        .expect("attached transport")
        .inject_event(segment_event(job_id, 3, range, "live text"));

    let before_poll_unix_ms = unix_time_ms();
    let result = runtime.poll();
    let after_poll_unix_ms = unix_time_ms();

    assert!(matches!(
        result,
        Some(Ok(ImportEvent::LiveProvisional {
            segment,
            captured_at_unix_ms: 1_000,
            available_at_unix_ms,
        }))
            if segment.job_id == job_id
                && segment.generation == Generation::first()
                && segment.instance_id == INSTANCE_ID
                && segment.segment_id == SegmentId(9)
                && segment.range == range
                && segment.text == "live text"
                && available_at_unix_ms >= before_poll_unix_ms
                && available_at_unix_ms <= after_poll_unix_ms
    ));
    assert_eq!(runtime.live_pending_persistence, 1);
}

#[test]
fn poll_returns_mapping_failure_before_final_live_failure_handling() {
    let root = TestRoot::new("live-sequence-exhausted");
    let executable = compile_idle_worker(&root.0);
    let job_id = JobId(0x99250);
    let mut runtime = runtime_with_active_attempt(job_id);
    let mut live = live_capture_state(&root.0, job_id);
    live.segment_sequence_base = u64::MAX;
    runtime.live = Some(live);
    attach_transport(&mut runtime, &executable);
    runtime
        .transport
        .as_mut()
        .expect("attached transport")
        .inject_event(segment_event(job_id, 2, source_range(0, 8_000), "overflow"));

    assert!(matches!(
        runtime.poll(),
        Some(Err((actual_job, generation, message)))
            if actual_job == job_id
                && generation == Generation::first()
                && message.contains("live segment sequence exhausted")
    ));
    let live = runtime
        .live
        .as_ref()
        .expect("immediate mapping error retains live state");
    assert!(
        live.failure.is_none(),
        "final live error handler did not run"
    );
    assert!(
        !live.stop_requested,
        "mapping error did not request final live stop"
    );
    assert!(
        runtime.transport.is_some(),
        "immediate error did not clear transport"
    );
    assert!(
        runtime.active.is_some(),
        "immediate error did not clear attempt"
    );
    assert_eq!(runtime.live_pending_persistence, 0);
}

#[test]
fn poll_end_retires_transport_to_pending_idle_only_for_import_request() {
    let root = TestRoot::new("end-idle-transfer");
    let executable = compile_idle_worker(&root.0);
    for (index, backend, choice, expected) in [
        (0, BackendKind::Cpu, ComputeChoice::Auto, ComputeChoice::Cpu),
        (
            1,
            BackendKind::Cuda,
            ComputeChoice::Auto,
            ComputeChoice::Gpu,
        ),
    ] {
        let job_id = JobId(0x99260 + index);
        let mut runtime = runtime_with_active_attempt(job_id);
        let active = runtime.active.as_mut().expect("active attempt");
        active.backend = backend;
        active.import_request = Some(import_request(job_id, choice));
        attach_transport(&mut runtime, &executable);
        runtime
            .transport
            .as_mut()
            .expect("attached transport")
            .inject_event(WorkerEvent::End {
                job_id,
                generation: Generation::first(),
                instance_id: INSTANCE_ID,
                last_sequence: 3,
                last_offset: 24_000,
            });

        assert!(matches!(
            runtime.poll(),
            Some(Ok(ImportEvent::End {
                job_id: actual_job,
                last_sequence: 3,
                last_offset: 24_000,
                ..
            })) if actual_job == job_id
        ));
        assert!(runtime.transport.is_none());
        assert!(runtime.active.is_none());
        let pending = runtime
            .pending_idle_worker
            .as_ref()
            .expect("request retains idle worker");
        assert_eq!(pending.compute, expected);
        assert_eq!(pending.model_path, "model.bin");
        assert_eq!(pending.model_sha256, [9; 32]);
        assert!(pending.expires_at <= Instant::now());
    }

    let job_id = JobId(0x99262);
    let mut runtime = runtime_with_active_attempt(job_id);
    attach_transport(&mut runtime, &executable);
    runtime
        .transport
        .as_mut()
        .expect("attached transport")
        .inject_event(WorkerEvent::End {
            job_id,
            generation: Generation::first(),
            instance_id: INSTANCE_ID,
            last_sequence: 0,
            last_offset: 0,
        });

    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::End { job_id: actual_job, .. })) if actual_job == job_id
    ));
    assert!(runtime.transport.is_none());
    assert!(runtime.active.is_none());
    assert!(runtime.pending_idle_worker.is_none());
}
