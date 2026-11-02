use super::*;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use whisper_core::{
    DiagnosticNotice, Generation, ImportEvent, JobId,
    ipc::{BackendKind, WorkerEvent},
};

static NEXT_TEST_ROOT: AtomicU64 = AtomicU64::new(0);

struct TestRoot(PathBuf);

impl TestRoot {
    fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock after epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "poll-family-one-{label}-{}-{nonce}-{}",
            std::process::id(),
            NEXT_TEST_ROOT.fetch_add(1, Ordering::Relaxed),
        ));
        fs::create_dir_all(&path).expect("create poll test root");
        Self(path)
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn compile_idle_worker(root: &Path) -> PathBuf {
    let source = root.join("idle-worker.rs");
    let executable = root.join("idle-worker.exe");
    fs::write(&source, "fn main() { std::thread::park(); }").expect("write idle worker fixture");
    let output = Command::new("rustc")
        .args(["--edition=2021", "--crate-name", "poll_test_worker"])
        .arg(&source)
        .arg("-o")
        .arg(&executable)
        .output()
        .expect("compile idle worker fixture");
    assert!(
        output.status.success(),
        "rustc idle worker fixture: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    executable
}

fn runtime_with_active_attempt(job_id: JobId) -> ImportRuntime {
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: PathBuf::from("unused-cpu.exe"),
            gpu: PathBuf::from("unused-gpu.exe"),
        },
        Arc::new(PublicationGate::default()),
    );
    runtime.active = Some(ImportRequestState {
        job_id,
        generation: Generation::first(),
        instance_id: 7,
        backend: BackendKind::Cpu,
        attempt_number: 0,
        import_request: None,
        recovery: LiveRecoveryState::None,
        startup_ready: true,
        gpu_candidate: None,
        gpu_attested: false,
    });
    runtime
}

fn live_request(root: &Path, job_id: JobId) -> whisper_core::ports::LiveRequest {
    let generation = Generation::first();
    whisper_core::ports::LiveRequest {
        job_id,
        generation,
        model_path: "unused-model".into(),
        model_sha256: [0; 32],
        destination: root.join("archive").to_string_lossy().into_owned(),
        group_offset_samples: 0,
        auto_stop_after_speech_samples: None,
        microphone: whisper_core::MicrophoneChoice::Device("test:poll".into()),
        microphone_gain_db: 0,
        config: whisper_core::JobConfig {
            language: whisper_core::LanguageChoice::Automatic,
            compute: whisper_core::ComputeChoice::Cpu,
        },
    }
}

fn live_capture_state(root: &Path, job_id: JobId) -> LiveCaptureState {
    let generation = Generation::first();
    let request = live_request(root, job_id);
    let capture = crate::capture::CaptureStream::empty_test_capture(16_000, 1, "test:poll");
    let probe = crate::microphone_probe::MicrophoneProbe::start(
        16_000,
        Some("test:poll".into()),
        job_id,
        generation,
    )
    .expect("start synthetic progress probe");
    let staging = crate::staging::PcmStaging::create(&root.join("pending.pcm"))
        .expect("create synthetic live staging")
        .into_writer();

    LiveCaptureState {
        request,
        capture,
        probe,
        spectrum_reader: crate::microphone_probe::MicrophoneSpectrumReader::default(),
        last_meter_sequence: 0,
        next_spectrum_telemetry: false,
        telemetry_due_after_event: false,
        converter: crate::capture::Pcm16Converter::new(16_000, 1)
            .expect("create synthetic live converter"),
        vad: crate::vad::VoiceActivity::new(),
        staging,
        frame_pending: Vec::new(),
        inference_cursor: 0,
        inference_sequence: 1,
        pending_window: None,
        replay_until: 0,
        replay_windows_queued: 0,
        replay_windows_to_skip: 0,
        segment_sequence_base: 1,
        window_sequence_base: 1,
        recovery: LiveRecoveryState::None,
        barrier_m: None,
        inference_admission_closed: false,
        worker_ready: true,
        stop_requested: false,
        stop_enqueued: false,
        started_at_unix_ms: 0,
        captured: 0,
        admitted: 0,
        speech: 0,
        confirmed_fragment_end: 0,
        last_reported_durable: 0,
        failure: None,
    }
}

#[test]
fn poll_emits_one_progress_stalled_notice_for_the_active_attempt() {
    let job_id = JobId(0x99101);
    let mut runtime = runtime_with_active_attempt(job_id);
    runtime.progress = Some(ProgressSupervisor::starting(
        Instant::now() - Duration::from_secs(61),
    ));

    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::Diagnostic {
            job_id: actual_job,
            generation,
            notice: DiagnosticNotice::ProgressStalled { phase, idle_ms },
        })) if actual_job == job_id
            && generation == Generation::first()
            && phase == whisper_core::DiagnosticPhase::Preparing
            && idle_ms >= 60_000
    ));
    assert!(runtime.poll().is_none(), "the same stall warns only once");
}

#[test]
fn poll_reports_degraded_diagnostics_once_for_the_active_attempt() {
    let job_id = JobId(0x99102);
    let mut runtime = runtime_with_active_attempt(job_id);
    runtime.diagnostic_start_failed = true;

    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::Diagnostic {
            job_id: actual_job,
            generation,
            notice: DiagnosticNotice::LoggingUnavailable,
        })) if actual_job == job_id && generation == Generation::first()
    ));
    assert!(runtime.diagnostic_degraded_reported);
    assert!(
        runtime.poll().is_none(),
        "logging degradation is reported once"
    );
}

#[test]
fn poll_returns_recovery_counters_then_ready_attempt_reset_before_observations() {
    let root = TestRoot::new("deferred-priority");
    let job_id = JobId(0x99104);
    let generation = Generation::first();
    let mut runtime = runtime_with_active_attempt(job_id);
    runtime.live = Some(live_capture_state(&root.0, job_id));
    runtime.pending_recovery_counters = Some(ImportEvent::LiveCounters {
        job_id,
        generation,
        captured_samples: 40,
        admitted_samples: 30,
        audio_durable_samples: 20,
        confirmed_fragment_samples: 10,
        speech_samples: 8,
        diagnostic: None,
    });
    runtime.pending_attempt_reset = Some(ImportEvent::LiveAttemptReset {
        job_id,
        generation,
        confirmed_samples: 10,
        next_segment_sequence: 2,
    });
    runtime.progress = Some(ProgressSupervisor::starting(
        Instant::now() - Duration::from_secs(61),
    ));
    runtime.diagnostic_start_failed = true;

    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::LiveCounters {
            captured_samples: 40,
            ..
        }))
    ));
    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::LiveAttemptReset {
            confirmed_samples: 10,
            next_segment_sequence: 2,
            ..
        }))
    ));
    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::Diagnostic {
            notice: DiagnosticNotice::ProgressStalled { .. },
            ..
        }))
    ));
    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::Diagnostic {
            notice: DiagnosticNotice::LoggingUnavailable,
            ..
        }))
    ));
}

#[test]
fn poll_keeps_attempt_reset_pending_until_live_worker_is_ready() {
    let root = TestRoot::new("reset-waits-ready");
    let job_id = JobId(0x99109);
    let generation = Generation::first();
    let mut runtime = runtime_with_active_attempt(job_id);
    let mut live = live_capture_state(&root.0, job_id);
    live.worker_ready = false;
    runtime.live = Some(live);
    runtime.pending_attempt_reset = Some(ImportEvent::LiveAttemptReset {
        job_id,
        generation,
        confirmed_samples: 4,
        next_segment_sequence: 2,
    });
    runtime.progress = Some(ProgressSupervisor::starting(
        Instant::now() - Duration::from_secs(61),
    ));

    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::Diagnostic {
            notice: DiagnosticNotice::ProgressStalled { .. },
            ..
        }))
    ));
    assert!(runtime.pending_attempt_reset.is_some());
    runtime.live.as_mut().expect("live attempt").worker_ready = true;
    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::LiveAttemptReset {
            confirmed_samples: 4,
            next_segment_sequence: 2,
            ..
        }))
    ));
    assert!(runtime.pending_attempt_reset.is_none());
}

#[test]
fn poll_defers_degraded_notice_until_an_active_attempt_exists() {
    let job_id = JobId(0x99105);
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: PathBuf::from("unused-cpu.exe"),
            gpu: PathBuf::from("unused-gpu.exe"),
        },
        Arc::new(PublicationGate::default()),
    );
    runtime.diagnostic_start_failed = true;

    assert!(runtime.poll().is_none());
    assert!(!runtime.diagnostic_degraded_reported);
    runtime.active = runtime_with_active_attempt(job_id).active;
    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::Diagnostic {
            job_id: actual_job,
            notice: DiagnosticNotice::LoggingUnavailable,
            ..
        })) if actual_job == job_id
    ));
    assert!(runtime.diagnostic_degraded_reported);
}

#[test]
fn poll_reports_asynchronous_diagnostic_sink_degradation_once() {
    let root = TestRoot::new("async-log-degradation");
    let job_id = JobId(0x99108);
    let mut runtime = runtime_with_active_attempt(job_id);
    runtime.start_diagnostics(&root.0);
    let diagnostic_dir = root.0.join(".whisper-diagnostics");
    fs::remove_dir_all(&diagnostic_dir).expect("remove diagnostic directory");
    fs::write(&diagnostic_dir, b"occupy diagnostic directory path")
        .expect("replace diagnostic directory with a file");
    runtime
        .diagnostics
        .as_ref()
        .expect("diagnostic sink started")
        .record(DiagnosticRecord::new(
            job_id.0,
            Generation::first().get(),
            DiagnosticPhase::Import,
            DiagnosticCode::Progress,
            Some("cpu"),
            Some((1, 2)),
        ));

    let deadline = Instant::now() + Duration::from_secs(5);
    while !runtime
        .diagnostics
        .as_ref()
        .expect("diagnostic sink remains available")
        .is_degraded()
    {
        assert!(
            Instant::now() < deadline,
            "diagnostic writer did not degrade"
        );
        std::thread::yield_now();
    }

    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::Diagnostic {
            job_id: actual_job,
            notice: DiagnosticNotice::LoggingUnavailable,
            ..
        })) if actual_job == job_id
    ));
    assert!(runtime.diagnostic_degraded_reported);
    assert!(
        runtime.poll().is_none(),
        "asynchronous degradation reports once"
    );
}

#[test]
fn poll_persists_ready_progress_failed_and_stopped_diagnostics_in_their_phases() {
    let cases = [
        (
            "ready",
            WorkerEvent::Ready {
                job_id: JobId(0x99106),
                generation: Generation::first(),
                instance_id: 7,
                backend: BackendKind::Cpu,
            },
            "\"phase\":\"preparing\"",
            "\"code\":\"backend_ready\"",
            false,
        ),
        (
            "progress",
            WorkerEvent::Progress {
                job_id: JobId(0x99106),
                generation: Generation::first(),
                instance_id: 7,
                completed_samples: 120,
                total_samples: 400,
            },
            "\"phase\":\"import\"",
            "\"code\":\"progress\"",
            false,
        ),
        (
            "progress_live",
            WorkerEvent::Progress {
                job_id: JobId(0x99106),
                generation: Generation::first(),
                instance_id: 7,
                completed_samples: 120,
                total_samples: 400,
            },
            "\"phase\":\"inference\"",
            "\"code\":\"progress\"",
            true,
        ),
        (
            "failed",
            WorkerEvent::Failed {
                job_id: Some(JobId(0x99106)),
                generation: Some(Generation::first()),
                instance_id: Some(7),
                code: whisper_core::ipc::WorkerErrorCode::WorkerExited,
                message: "synthetic failure".into(),
            },
            "\"phase\":\"finalizing\"",
            "\"code\":\"worker_failed\"",
            false,
        ),
        (
            "stopped",
            WorkerEvent::Stopped {
                job_id: JobId(0x99106),
                generation: Generation::first(),
                instance_id: 7,
            },
            "\"phase\":\"cancelling\"",
            "\"code\":\"worker_stopped\"",
            false,
        ),
    ];

    for (label, event, phase, code, live) in cases {
        let root = TestRoot::new(label);
        let worker = compile_idle_worker(&root.0);
        let job_id = JobId(0x99106);
        let mut runtime = runtime_with_active_attempt(job_id);
        if live {
            runtime.live = Some(live_capture_state(&root.0, job_id));
            runtime.live_capture_events_since_worker_poll = 2;
        }
        runtime.start_diagnostics(&root.0);
        let mut transport = ChildWorkerTransport::spawn(&worker).expect("spawn idle worker");
        transport.inject_event(event);
        runtime.transport = Some(transport);

        assert!(runtime.poll().is_some(), "{label} event is processed");
        drop(runtime);
        let diagnostic = fs::read_to_string(root.0.join(".whisper-diagnostics/worker-0.jsonl"))
            .expect("poll recorded worker diagnostic");
        assert!(
            diagnostic.contains(phase),
            "wrong diagnostic phase for {label}"
        );
        assert!(
            diagnostic.contains(code),
            "wrong diagnostic code for {label}"
        );
    }
}

#[test]
fn poll_progress_observations_reset_idle_warning_and_terminal_events_close_phases() {
    let root = TestRoot::new("progress-phases");
    let worker = compile_idle_worker(&root.0);
    let job_id = JobId(0x99107);
    let generation = Generation::first();
    let mut runtime = runtime_with_active_attempt(job_id);
    runtime.progress = Some(ProgressSupervisor::starting(
        Instant::now() - Duration::from_secs(61),
    ));
    runtime.start_diagnostics(&root.0);
    let mut transport = ChildWorkerTransport::spawn(&worker).expect("spawn idle worker");
    transport.inject_event(WorkerEvent::Ready {
        job_id,
        generation,
        instance_id: 7,
        backend: BackendKind::Cpu,
    });
    transport.inject_event(WorkerEvent::Progress {
        job_id,
        generation,
        instance_id: 7,
        completed_samples: 120,
        total_samples: 400,
    });
    transport.inject_event(WorkerEvent::Stopped {
        job_id,
        generation,
        instance_id: 7,
    });
    runtime.transport = Some(transport);

    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::Diagnostic {
            notice: DiagnosticNotice::ProgressStalled {
                phase: whisper_core::DiagnosticPhase::Preparing,
                ..
            },
            ..
        }))
    ));
    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::Ready { .. }))
    ));
    let stalled_at = Instant::now() + Duration::from_secs(61);
    assert!(matches!(
        runtime
            .progress
            .as_mut()
            .and_then(|progress| progress.take_warning(stalled_at)),
        Some((ProgressPhase::Inference, _))
    ));
    assert!(matches!(
        runtime
            .progress
            .as_mut()
            .and_then(|progress| progress.take_warning(stalled_at)),
        Some((ProgressPhase::Import, _))
    ));
    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::Progress {
            completed_samples: 120,
            ..
        }))
    ));
    let observed_at = Instant::now();
    assert!(
        runtime
            .progress
            .as_mut()
            .and_then(|progress| progress.take_warning(observed_at + Duration::from_secs(59)))
            .is_none()
    );
    assert!(matches!(
        runtime
            .progress
            .as_mut()
            .and_then(|progress| progress.take_warning(observed_at + Duration::from_secs(61))),
        Some((ProgressPhase::Inference, _))
    ));
    assert!(matches!(
        runtime
            .progress
            .as_mut()
            .and_then(|progress| progress.take_warning(observed_at + Duration::from_secs(61))),
        Some((ProgressPhase::Import, _))
    ));
    let rearmed_at = Instant::now();
    for phase in [ProgressPhase::Inference, ProgressPhase::Import] {
        runtime
            .progress
            .as_mut()
            .expect("progress supervisor")
            .observe(phase, 121, rearmed_at);
    }
    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::Stopped { .. }))
    ));
    assert!(
        runtime
            .progress
            .as_mut()
            .and_then(|progress| progress.take_warning(Instant::now() + Duration::from_secs(61)))
            .is_none()
    );

    drop(runtime);
    let diagnostic = fs::read_to_string(root.0.join(".whisper-diagnostics/worker-0.jsonl"))
        .expect("poll recorded phase observations");
    assert!(diagnostic.contains("\"phase\":\"preparing\""));
    assert!(diagnostic.contains("\"phase\":\"import\""));
    assert!(diagnostic.contains("\"completed_samples\":120"));
    assert!(diagnostic.contains("\"phase\":\"cancelling\""));
}

#[test]
fn poll_ready_opens_live_capture_and_draining_phases_for_live_and_pending_live() {
    for pending_live in [false, true] {
        let label = if pending_live { "pending" } else { "active" };
        let root = TestRoot::new(&format!("ready-{label}-phases"));
        let worker = compile_idle_worker(&root.0);
        let job_id = JobId(if pending_live { 0x99111 } else { 0x99110 });
        let generation = Generation::first();
        let mut runtime = runtime_with_active_attempt(job_id);
        runtime.progress = Some(ProgressSupervisor::starting(Instant::now()));
        if pending_live {
            let request = live_request(&root.0, job_id);
            let archive =
                crate::archive::LiveArchive::prepare(&request.destination, request.clone())
                    .expect("prepare synthetic pending live archive");
            let staging = crate::staging::PcmStaging::create(&root.0.join("pending-live.pcm"))
                .expect("prepare synthetic pending live staging")
                .into_writer();
            runtime.pending_live = Some(PendingLiveState {
                request,
                staging,
                live_archive: archive,
                recovery: LiveRecoveryState::None,
            });
            runtime.publication_gate.activate();
            runtime
                .publication_gate
                .request_stop()
                .expect("latch stop to avoid starting real capture");
        } else {
            runtime.live = Some(live_capture_state(&root.0, job_id));
            runtime.live_capture_events_since_worker_poll = 2;
        }
        let mut transport = ChildWorkerTransport::spawn(&worker).expect("spawn idle worker");
        transport.inject_event(WorkerEvent::Ready {
            job_id,
            generation,
            instance_id: 7,
            backend: BackendKind::Cpu,
        });
        runtime.transport = Some(transport);

        assert!(matches!(
            runtime.poll(),
            Some(Ok(ImportEvent::Ready { .. }))
        ));
        let stalled_at = Instant::now() + Duration::from_secs(61);
        for phase in [
            ProgressPhase::Inference,
            ProgressPhase::Capturing,
            ProgressPhase::Draining,
        ] {
            assert!(
                matches!(
                    runtime
                        .progress
                        .as_mut()
                        .and_then(|progress| progress.take_warning(stalled_at)),
                    Some((actual, _)) if actual == phase
                ),
                "{label} Ready should open {phase:?}"
            );
        }
        drop(runtime);
    }
}

#[test]
fn poll_live_segment_refreshes_the_inference_progress_obligation() {
    let root = TestRoot::new("segment-progress");
    let worker = compile_idle_worker(&root.0);
    let job_id = JobId(0x99112);
    let generation = Generation::first();
    let mut runtime = runtime_with_active_attempt(job_id);
    runtime.live = Some(live_capture_state(&root.0, job_id));
    runtime.live_capture_events_since_worker_poll = 2;
    runtime.progress = Some(ProgressSupervisor::default());
    runtime
        .progress
        .as_mut()
        .expect("progress supervisor")
        .open(
            ProgressPhase::Inference,
            Instant::now() - Duration::from_secs(61),
        );
    assert!(
        runtime
            .progress
            .as_mut()
            .and_then(|progress| progress.take_warning(Instant::now()))
            .is_some()
    );

    let mut transport = ChildWorkerTransport::spawn(&worker).expect("spawn idle worker");
    transport.inject_event(WorkerEvent::Segment(whisper_core::ipc::WorkerSegmentDto {
        job_id,
        generation,
        instance_id: 7,
        segment_id: whisper_core::SegmentId(1),
        range: whisper_core::SourceRange {
            start_sample: 0,
            end_sample: 16_000,
            sample_rate_hz: 16_000,
        },
        text: "synthetic segment".into(),
    }));
    runtime.transport = Some(transport);

    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::LiveProvisional { .. }))
    ));
    let observed_at = Instant::now();
    assert!(
        runtime
            .progress
            .as_mut()
            .and_then(|progress| progress.take_warning(observed_at + Duration::from_secs(59)))
            .is_none()
    );
    assert!(matches!(
        runtime
            .progress
            .as_mut()
            .and_then(|progress| progress.take_warning(observed_at + Duration::from_secs(61))),
        Some((ProgressPhase::Inference, _))
    ));
}

#[test]
fn poll_end_closes_inference_and_import_and_opens_finalizing() {
    let root = TestRoot::new("end-progress");
    let worker = compile_idle_worker(&root.0);
    let job_id = JobId(0x99113);
    let generation = Generation::first();
    let mut runtime = runtime_with_active_attempt(job_id);
    runtime.progress = Some(ProgressSupervisor::default());
    let old = Instant::now() - Duration::from_secs(61);
    runtime
        .progress
        .as_mut()
        .expect("progress supervisor")
        .open(ProgressPhase::Inference, old);
    runtime
        .progress
        .as_mut()
        .expect("progress supervisor")
        .open(ProgressPhase::Import, old);
    for phase in [ProgressPhase::Inference, ProgressPhase::Import] {
        assert!(matches!(
            runtime
                .progress
                .as_mut()
                .and_then(|progress| progress.take_warning(Instant::now())),
            Some((actual, _)) if actual == phase
        ));
    }
    let rearmed_at = Instant::now();
    for phase in [ProgressPhase::Inference, ProgressPhase::Import] {
        runtime
            .progress
            .as_mut()
            .expect("progress supervisor")
            .observe(phase, 1, rearmed_at);
    }
    let mut transport = ChildWorkerTransport::spawn(&worker).expect("spawn idle worker");
    transport.inject_event(WorkerEvent::End {
        job_id,
        generation,
        instance_id: 7,
        last_sequence: 0,
        last_offset: 0,
    });
    runtime.transport = Some(transport);

    assert!(matches!(runtime.poll(), Some(Ok(ImportEvent::End { .. }))));
    let stalled_at = Instant::now() + Duration::from_secs(61);
    assert!(matches!(
        runtime
            .progress
            .as_mut()
            .and_then(|progress| progress.take_warning(stalled_at)),
        Some((ProgressPhase::Finalizing, _))
    ));
    assert!(
        runtime
            .progress
            .as_mut()
            .and_then(|progress| progress.take_warning(stalled_at))
            .is_none()
    );
}

#[test]
fn poll_failed_closes_all_open_progress_phases() {
    let root = TestRoot::new("failed-phases");
    let worker = compile_idle_worker(&root.0);
    let job_id = JobId(0x99114);
    let generation = Generation::first();
    let mut runtime = runtime_with_active_attempt(job_id);
    runtime.progress = Some(ProgressSupervisor::default());
    for phase in [
        ProgressPhase::Preparing,
        ProgressPhase::Capturing,
        ProgressPhase::Draining,
        ProgressPhase::Inference,
        ProgressPhase::Import,
        ProgressPhase::Finalizing,
    ] {
        runtime
            .progress
            .as_mut()
            .expect("progress supervisor")
            .open(phase, Instant::now() - Duration::from_secs(61));
    }
    let stalled_at = Instant::now();
    for _ in 0..6 {
        assert!(
            runtime
                .progress
                .as_mut()
                .and_then(|progress| progress.take_warning(stalled_at))
                .is_some()
        );
    }
    let observed_at = Instant::now();
    for phase in [
        ProgressPhase::Preparing,
        ProgressPhase::Capturing,
        ProgressPhase::Draining,
        ProgressPhase::Inference,
        ProgressPhase::Import,
        ProgressPhase::Finalizing,
    ] {
        runtime
            .progress
            .as_mut()
            .expect("progress supervisor")
            .observe(phase, 1, observed_at);
    }
    let mut transport = ChildWorkerTransport::spawn(&worker).expect("spawn idle worker");
    transport.inject_event(WorkerEvent::Failed {
        job_id: Some(job_id),
        generation: Some(generation),
        instance_id: Some(7),
        code: whisper_core::ipc::WorkerErrorCode::WorkerExited,
        message: "synthetic failure".into(),
    });
    runtime.transport = Some(transport);

    assert!(matches!(runtime.poll(), Some(Err(_))));
    assert!(
        runtime
            .progress
            .as_mut()
            .and_then(|progress| progress.take_warning(Instant::now() + Duration::from_secs(61)))
            .is_none()
    );
}

#[test]
fn poll_records_ready_and_advances_progress_before_rejecting_backend_mismatch() {
    let root = TestRoot::new("ready-mismatch");
    let worker = compile_idle_worker(&root.0);
    let job_id = JobId(0x99103);
    let generation = Generation::first();
    let mut runtime = runtime_with_active_attempt(job_id);
    runtime.progress = Some(ProgressSupervisor::starting(Instant::now()));
    runtime.start_diagnostics(&root.0);
    runtime.transport = Some(ChildWorkerTransport::spawn(&worker).expect("spawn idle worker"));
    runtime
        .transport
        .as_mut()
        .expect("worker transport")
        .inject_event(WorkerEvent::Ready {
            job_id,
            generation,
            instance_id: 7,
            backend: BackendKind::Cuda,
        });

    let result = runtime.poll();
    assert!(matches!(
        result,
        Some(Err((actual_job, actual_generation, message)))
            if actual_job == job_id
                && actual_generation == generation
                && message.contains("BackendMismatch")
    ));
    assert!(
        matches!(
            runtime.progress.as_mut().and_then(|progress| {
                progress.take_warning(Instant::now() + Duration::from_secs(61))
            }),
            Some((ProgressPhase::Inference, _)),
        ),
        "Ready must advance the preparing/inference obligations before its backend is refused"
    );

    drop(runtime);
    let diagnostic = fs::read_to_string(root.0.join(".whisper-diagnostics/worker-0.jsonl"))
        .expect("poll recorded Ready before rejecting its backend");
    assert!(diagnostic.contains("\"phase\":\"preparing\""));
    assert!(diagnostic.contains("\"code\":\"backend_ready\""));
}
