use super::*;
use crate::worker_ipc::{
    NativeUnifiedRoute, parakeet_unified::UnifiedDocumentDrain, runtime::MicrophoneTestCapture,
};
use std::sync::Mutex;

struct ReadyPriorityWorker;

impl crate::parakeet_pipeline::WindowTranscriber for ReadyPriorityWorker {
    fn instance_id(&self) -> u64 {
        73_100
    }

    fn transcribe(
        &mut self,
        _: whisper_core::parakeet_session::AudioWindow,
        _: &[i16],
    ) -> Result<Vec<whisper_core::parakeet_ipc::Word>, String> {
        Ok(Vec::new())
    }
}

#[test]
fn poll_ready_unified_import_precedes_ready_document_drain() {
    let root = TestRoot::new("unified-session-priority");
    let session_job = JobId(73_010);
    let document_job = JobId(73_011);
    let source_path = root.0.join("closed-source.wav");
    let mut source_writer = crate::parakeet_audio::PcmPartWriter::create(&source_path)
        .expect("create closed unified PCM source");
    source_writer
        .append(&vec![0_i16; 5 * 16_000])
        .expect("write durable unified PCM source");
    source_writer.finish().expect("finish unified PCM source");
    let model =
        whisper_core::model_registry::model(whisper_core::model_catalog::ModelId::ParakeetTdt06BV3);
    let model_path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("models")
        .join(model.artifact.file_name);
    let model_sha256 = (0..32)
        .map(|index| {
            u8::from_str_radix(&model.artifact.sha256[index * 2..index * 2 + 2], 16)
                .expect("registry model SHA-256 is hexadecimal")
        })
        .collect::<Vec<_>>()
        .try_into()
        .expect("registry model SHA-256 has 32 bytes");
    let request = whisper_core::ports::ImportRequest {
        strategy: whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1,
        job_id: session_job,
        generation: Generation::first(),
        source_path: source_path.to_string_lossy().into_owned(),
        source_sha256: None,
        source_samples: None,
        model_path: model_path.to_string_lossy().into_owned(),
        model_sha256,
        destination: root.0.to_string_lossy().into_owned(),
        config: whisper_core::JobConfig {
            language: whisper_core::LanguageChoice::Automatic,
            compute: whisper_core::ComputeChoice::Cpu,
        },
    };
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: session_job,
        generation: Generation::first(),
    };
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: root.0.join("unused-cpu.exe"),
            gpu: root.0.join("unused-gpu.exe"),
        },
        Arc::new(PublicationGate::default()),
    )
    .with_controlled_transcriber(Box::new(ReadyPriorityWorker));
    runtime = runtime.with_child_observer(Arc::new(Mutex::new(Vec::new())));
    let ready_gate = runtime.hold_next_child_ready();
    let route = NativeUnifiedRoute {
        paths: crate::parakeet_import::ParakeetWorkerPaths {
            executable: root.0.join("controlled-worker.exe"),
            native_dir: root.0.join("native"),
            model: root.0.join("controlled-model.nemo"),
            runtime_root: root.0.clone(),
        },
        requested_compute: whisper_core::ComputeChoice::Cpu,
        compute: whisper_core::ComputeChoice::Cpu,
        expected_gpu: None,
        candidate: None,
        fallback_reason: None,
    };
    runtime.native_unified_routes.insert(session, route);
    runtime.native_unified_candidate_enabled = true;
    runtime
        .start_controlled_unified_import(&request)
        .expect("start controlled unified import from closed PCM");

    let prepared_deadline = Instant::now() + Duration::from_secs(5);
    let mut prepared_while_waiting_for_ready = false;
    while Instant::now() < prepared_deadline && !prepared_while_waiting_for_ready {
        match runtime.poll() {
            Some(Ok(ImportEvent::Ready { .. })) => {
                panic!("session READY escaped its held gate")
            }
            Some(Err(error)) => panic!("import poll failed before READY: {error:?}"),
            Some(Ok(_)) | None => {}
        }
        prepared_while_waiting_for_ready =
            runtime.unified_sessions.get(&session).is_some_and(|state| {
                state.pending_prepared.is_some() && state.durable_samples == 5 * 16_000
            });
        if !prepared_while_waiting_for_ready {
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    assert!(
        prepared_while_waiting_for_ready,
        "closed durable PCM must be prepared while worker READY remains gated"
    );

    let document = crate::parakeet_document::ParakeetDocumentStore::create(
        root.0.join("ready-document"),
        document_job,
        Generation::first(),
    )
    .expect("create empty ready document");
    runtime.document_drains.push_back(UnifiedDocumentDrain {
        document,
        job_id: document_job,
        generation: Generation::first(),
        revision: 0,
        total_samples: 0,
        next_sequence: 1,
        ready_pending: true,
        progress_pending: false,
    });

    let (released, changed) = &*ready_gate;
    *released.lock().expect("READY gate") = true;
    changed.notify_all();
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut ready_seen = false;
    while Instant::now() < deadline && !ready_seen {
        ready_seen = runtime
            .unified_executor
            .as_mut()
            .expect("unified inference executor")
            .is_ready()
            .expect("controlled worker must become ready");
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(
        ready_seen,
        "controlled worker must pass the READY gate before polling"
    );

    let ready_deadline = Instant::now() + Duration::from_secs(5);
    let mut session_ready = false;
    let mut document_ready = false;
    while Instant::now() < ready_deadline && !document_ready {
        match runtime.poll() {
            Some(Ok(ImportEvent::Ready { job_id, .. })) if job_id == session_job => {
                session_ready = true;
            }
            Some(Ok(ImportEvent::Ready { job_id, .. })) if job_id == document_job => {
                assert!(session_ready, "session READY must precede document READY");
                document_ready = true;
            }
            Some(Err(error)) => panic!("unified import poll failed: {error:?}"),
            Some(Ok(_)) | None => {}
        }
        if !document_ready {
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    assert!(session_ready, "the unified session READY must be collected");
    assert!(document_ready, "the document READY must be collected");
    assert_eq!(runtime.document_drains.len(), 1);
    assert!(!runtime.document_drains[0].ready_pending);
    let document_end_deadline = Instant::now() + Duration::from_secs(5);
    let mut document_ended = false;
    while Instant::now() < document_end_deadline && !document_ended {
        match runtime.poll() {
            Some(Ok(ImportEvent::End {
                job_id: actual,
                last_sequence: 0,
                ..
            })) if actual == document_job => document_ended = true,
            Some(Err(error)) => panic!("unified import poll failed: {error:?}"),
            Some(Ok(_)) | None => {
                assert_eq!(runtime.document_drains.len(), 1);
                assert!(!runtime.document_drains[0].ready_pending);
                std::thread::sleep(Duration::from_millis(2));
            }
        }
    }
    assert!(
        document_ended,
        "the document drain must reach its final End"
    );
    assert!(runtime.document_drains.is_empty());
}

#[test]
fn poll_drains_document_before_queued_event_and_microphone_block() {
    let root = TestRoot::new("document-priority");
    let job_id = JobId(73_001);
    let generation = Generation::first();
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: root.0.join("unused-cpu.exe"),
            gpu: root.0.join("unused-gpu.exe"),
        },
        Arc::new(PublicationGate::default()),
    );
    let document = crate::parakeet_document::ParakeetDocumentStore::create(
        root.0.join("document"),
        job_id,
        generation,
    )
    .expect("create empty unified document");
    runtime.document_drains.push_back(UnifiedDocumentDrain {
        document,
        job_id,
        generation,
        revision: 0,
        total_samples: 0,
        next_sequence: 1,
        ready_pending: true,
        progress_pending: false,
    });
    runtime
        .parakeet_pending_events
        .push_back(ImportEvent::Progress {
            job_id,
            generation,
            completed_samples: 0,
            total_samples: 0,
        });
    let mut capture =
        crate::capture::CaptureStream::empty_test_capture(16_000, 2, "test:poll-document-priority");
    capture.enqueue_test_block(vec![0.25, -0.25]);
    runtime.microphone_test = Some(
        MicrophoneTestCapture::from_capture(job_id, generation, capture)
            .expect("create test microphone")
            .0,
    );

    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::Ready { job_id: actual, .. })) if actual == job_id
    ));
    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::End { job_id: actual, last_sequence: 0, .. })) if actual == job_id
    ));
    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::Progress { job_id: actual, .. })) if actual == job_id
    ));
    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::MicrophoneGainApplied { job_id: actual, .. })) if actual == job_id
    ));
    assert!(runtime.document_drains.is_empty());
    assert!(runtime.parakeet_pending_events.is_empty());
    assert!(runtime.microphone_test.is_some());
}

#[test]
fn poll_microphone_capture_absence_is_none_and_bad_frame_cleans_up() {
    let root = TestRoot::new("microphone-collection");
    let job_id = JobId(73_002);
    let generation = Generation::first();
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: root.0.join("unused-cpu.exe"),
            gpu: root.0.join("unused-gpu.exe"),
        },
        Arc::new(PublicationGate::default()),
    );
    let capture =
        crate::capture::CaptureStream::empty_test_capture(16_000, 2, "test:poll-microphone-error");
    runtime.microphone_test = Some(
        MicrophoneTestCapture::from_capture(job_id, generation, capture)
            .expect("create test microphone")
            .0,
    );
    assert!(
        runtime.poll().is_none(),
        "an empty capture produces no event"
    );

    runtime
        .microphone_test
        .as_mut()
        .expect("microphone remains active after absence")
        .capture
        .enqueue_test_block(vec![0.25]);
    assert!(matches!(
        runtime.poll(),
        Some(Err((actual_job, actual_generation, message)))
            if actual_job == job_id
                && actual_generation == generation
                && message.contains("CaptureInterrupted")
    ));
    assert!(runtime.microphone_test.is_none());
}
