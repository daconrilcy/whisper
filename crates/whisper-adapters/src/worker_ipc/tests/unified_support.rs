    use super::*;
    use sha2::{Digest, Sha256};
    use std::io::{Cursor, Read};
    use std::{cell::RefCell, rc::Rc};
    use whisper_core::ports::ParakeetDocumentPort;
    use whisper_core::{
        AppCommand, Application, ComputeChoice, Generation, ImportApplication, ImportRequest,
        JobConfig, JobId, JobState, LanguageChoice,
    };

    #[test]
    #[ignore = "requires the CI legacy build variables"]
    fn recette_policy_l2a2_requires_the_feature() {
        assert_eq!(option_env!("PARAKEET_L2_ACCEPTANCE_TEST_BUILD"), Some("1"));
        let result = acceptance_barrier("not-an-acceptance-phase", Path::new("."));
        #[cfg(feature = "recette-hooks")]
        assert_eq!(
            result.unwrap_err(),
            "InvalidInput: unknown L2a-2 acceptance barrier"
        );
        #[cfg(not(feature = "recette-hooks"))]
        assert!(result.is_ok());
    }

    thread_local! {
        static IMPORT_PORT_EVENT_TRACE: RefCell<Option<Arc<Mutex<Vec<String>>>>> = RefCell::new(None);
    }

    fn record_import_port_event(event: &ImportEvent) {
        IMPORT_PORT_EVENT_TRACE.with(|trace| {
            if let Some(trace) = trace.borrow().as_ref() {
                let mut trace = trace.lock().expect("import event trace lock");
                if trace.len() < 128 {
                    trace.push(format!("{event:?}"));
                }
            }
        });
    }

    fn controlled_child_job_bound(
        observations: &[ControlledChildObservation],
    ) -> Result<(), String> {
        if let Some(observation) = observations
            .iter()
            .find(|observation| observation.retained_job_ids > UNIFIED_CHILD_JOB_CAPACITY)
        {
            Err(format!(
                "child incarnation {} retained {} JobIds",
                observation.incarnation, observation.retained_job_ids
            ))
        } else {
            Ok(())
        }
    }

    fn persist_and_publish_controlled_document_event(
        runtime: &mut ImportRuntime,
        event: ImportEvent,
        persisted_sequences: &mut HashMap<(JobId, Generation), u64>,
        published_sessions: &mut HashSet<(JobId, Generation)>,
        ready_labels: &mut HashMap<(JobId, Generation), Vec<String>>,
    ) {
        match event {
            ImportEvent::Ready {
                job_id,
                generation,
                backend,
            } => {
                ready_labels
                    .entry((job_id, generation))
                    .or_default()
                    .push(backend);
            }
            ImportEvent::Segment(segment) => {
                let identity = (segment.job_id, segment.generation);
                let sequence = segment.segment_id.0;
                assert!(sequence > persisted_sequences.get(&identity).copied().unwrap_or(0));
                let mut acknowledgements = VecDeque::new();
                runtime.effect(ImportEffect::PersistSegment(segment), &mut acknowledgements);
                assert!(
                    acknowledgements.iter().any(|event| matches!(
                        event,
                        ImportEvent::Persisted {
                            job_id,
                            generation,
                            sequence: acknowledged_sequence,
                        } if (*job_id, *generation, *acknowledged_sequence)
                            == (identity.0, identity.1, sequence)
                    )),
                    "real PersistSegment effect must return a matching ACK; received {acknowledgements:?}"
                );
                persisted_sequences.insert(identity, sequence);
            }
            ImportEvent::End {
                job_id,
                generation,
                last_sequence,
                ..
            } => {
                let identity = (job_id, generation);
                assert!(
                    last_sequence > 0,
                    "the completed document must contain segments"
                );
                assert_eq!(
                    persisted_sequences.get(&identity).copied(),
                    Some(last_sequence),
                    "Publish must wait for the final Persisted ACK"
                );
                let mut publication_events = VecDeque::new();
                runtime.effect(
                    ImportEffect::Publish {
                        job_id,
                        generation,
                        last_sequence,
                    },
                    &mut publication_events,
                );
                assert!(
                    publication_events.iter().any(|event| matches!(
                        event,
                        ImportEvent::Published {
                            job_id: published_job,
                            generation: published_generation,
                        } if (*published_job, *published_generation) == identity
                    )),
                    "real Publish effect must return a matching Published event"
                );
                assert!(published_sessions.insert(identity));
            }
            _ => {}
        }
    }

    fn prepare_and_start_controlled_native_route(
        runtime: &mut ImportRuntime,
        request: &ImportRequest,
        route: NativeUnifiedRoute,
    ) {
        let mut events = VecDeque::new();
        runtime.effect(ImportEffect::Prepare(request.clone()), &mut events);
        let prepared = match events.pop_front() {
            Some(ImportEvent::Prepared(prepared)) => prepared,
            other => panic!("Prepare did not return Prepared: {other:?}; remaining={events:?}"),
        };
        assert_eq!(
            (prepared.job_id, prepared.generation),
            (request.job_id, request.generation)
        );
        assert_eq!(prepared.destination, request.destination);
        assert_eq!(prepared.source_path, request.source_path);
        assert!(prepared.source_sha256.is_some());
        assert!(prepared.source_samples.is_some());
        assert!(
            events.is_empty(),
            "Prepare returned unexpected events: {events:?}"
        );

        let identity = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: prepared.job_id,
            generation: prepared.generation,
        };
        runtime.native_unified_routes.insert(identity, route);
        runtime.effect(ImportEffect::StartWorker(prepared), &mut events);
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, ImportEvent::Failed { .. })),
            "StartWorker failed after archive preparation: {events:?}"
        );
        assert!(runtime.unified_sessions.contains_key(&identity));
        assert!(runtime.unified_archives.contains_key(&identity));
        assert!(runtime.unified_sessions[&identity].native_route.is_some());
    }

    struct RuntimeImportPort {
        runtime: ImportRuntime,
        events: VecDeque<ImportEvent>,
        capture_progress: Option<(JobId, Arc<AtomicU64>)>,
        document_revision: Option<(JobId, Arc<AtomicU64>)>,
        unified_stop_submissions: Option<Arc<AtomicU64>>,
        controlled_capture_blocks: Option<Arc<Mutex<VecDeque<(JobId, Generation, Vec<f32>)>>>>,
    }

    impl ImportIoPort for RuntimeImportPort {
        fn submit(&mut self, effect: ImportEffect) -> Result<(), PortError> {
            if matches!(&effect, ImportEffect::StopUnifiedLive { .. })
                && let Some(submissions) = &self.unified_stop_submissions
            {
                submissions.fetch_add(1, Ordering::Relaxed);
            }
            let starts_generation = matches!(
                &effect,
                ImportEffect::StartMicrophoneTest { .. }
                    | ImportEffect::StartLive(_)
                    | ImportEffect::StartUnifiedLive(_)
                    | ImportEffect::StartDirectLive(_)
                    | ImportEffect::Prepare(_)
                    | ImportEffect::ProcessQueued(_)
                    | ImportEffect::ResumeInterrupted { .. }
            );
            if starts_generation {
                self.runtime.publication_gate.activate();
            }
            self.runtime.effect(effect, &mut self.events);
            Ok(())
        }

        fn poll(&mut self) -> Result<Option<ImportEvent>, PortError> {
            if let Some(blocks) = &self.controlled_capture_blocks {
                let mut blocks = blocks.lock().expect("controlled capture block queue");
                while let Some((job_id, generation, samples)) = blocks.pop_front() {
                    let session =
                        whisper_core::parakeet_scheduler::SchedulerSession { job_id, generation };
                    if let Some(state) = self.runtime.unified_sessions.get_mut(&session) {
                        state.source.enqueue_test_block(samples);
                    } else {
                        blocks.push_front((job_id, generation, samples));
                        break;
                    }
                }
            }
            if let Some(event) = self.events.pop_front() {
                record_import_port_event(&event);
                return Ok(Some(event));
            }
            if let Some((job_id, progress)) = &self.capture_progress {
                if let Some(samples) =
                    self.runtime
                        .unified_sessions
                        .iter()
                        .find_map(|(session, state)| {
                            (session.job_id == *job_id && state.source.is_capture())
                                .then_some(state.available_samples)
                        })
                {
                    progress.fetch_max(samples, Ordering::Relaxed);
                }
            }
            if let Some((job_id, revision)) = &self.document_revision {
                if let Some(current) =
                    self.runtime
                        .unified_sessions
                        .iter()
                        .find_map(|(session, state)| {
                            (session.job_id == *job_id).then_some(state.document.revision())
                        })
                {
                    revision.fetch_max(current, Ordering::Relaxed);
                }
            }
            match self.runtime.poll() {
                Some(Ok(event)) => {
                    record_import_port_event(&event);
                    Ok(Some(event))
                }
                Some(Err((job_id, generation, message))) => {
                    let event = ImportEvent::Failed {
                        job_id,
                        generation,
                        message,
                    };
                    record_import_port_event(&event);
                    Ok(Some(event))
                }
                None => Ok(None),
            }
        }
    }

    struct ControlledUnifiedWorker {
        calls: Option<Arc<AtomicU64>>,
    }

    impl crate::parakeet_pipeline::WindowTranscriber for ControlledUnifiedWorker {
        fn instance_id(&self) -> u64 {
            9001
        }

        fn transcribe(
            &mut self,
            window: whisper_core::parakeet_session::AudioWindow,
            _: &[i16],
        ) -> Result<Vec<whisper_core::parakeet_ipc::Word>, String> {
            if let Some(calls) = &self.calls {
                calls.fetch_add(1, Ordering::Relaxed);
            }
            let start_ms = ((window.central_start_sample - window.native_start_sample) / 16) as u32;
            Ok(vec![whisper_core::parakeet_ipc::Word {
                text: "controlled.".into(),
                start_ms,
                end_ms: start_ms + 100,
            }])
        }
    }

    struct StartFailsOnceUnifiedWorker {
        starts: Arc<AtomicU64>,
        stops: Arc<AtomicU64>,
    }

    impl crate::parakeet_pipeline::WindowTranscriber for StartFailsOnceUnifiedWorker {
        fn instance_id(&self) -> u64 {
            self.starts.load(Ordering::Relaxed).max(1)
        }

        fn initialize_instance(&mut self, _: &std::sync::atomic::AtomicBool) -> Result<(), String> {
            if self.starts.fetch_add(1, Ordering::Relaxed) == 0 {
                Err("controlled READY attestation failure".into())
            } else {
                Ok(())
            }
        }

        fn stop_instance(&mut self) -> Result<(), String> {
            self.stops.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }

        fn transcribe(
            &mut self,
            _: whisper_core::parakeet_session::AudioWindow,
            _: &[i16],
        ) -> Result<Vec<whisper_core::parakeet_ipc::Word>, String> {
            Ok(Vec::new())
        }
    }

    fn controlled_worker() -> (
        Box<dyn crate::parakeet_pipeline::WindowTranscriber>,
        Arc<AtomicU64>,
    ) {
        let calls = Arc::new(AtomicU64::new(0));
        (
            Box::new(ControlledUnifiedWorker {
                calls: Some(Arc::clone(&calls)),
            }),
            calls,
        )
    }

    struct HeldUnifiedWorker {
        calls: Arc<AtomicU64>,
        active: Arc<AtomicU64>,
        maximum_active: Arc<AtomicU64>,
        call_sources: Arc<std::sync::Mutex<Vec<String>>>,
        hold_call: u64,
        hold_first_final_a: bool,
        held: std::sync::mpsc::SyncSender<String>,
        release: std::sync::mpsc::Receiver<()>,
    }

    impl crate::parakeet_pipeline::WindowTranscriber for HeldUnifiedWorker {
        fn instance_id(&self) -> u64 {
            9010
        }

        fn transcribe(
            &mut self,
            window: whisper_core::parakeet_session::AudioWindow,
            pcm: &[i16],
        ) -> Result<Vec<whisper_core::parakeet_ipc::Word>, String> {
            let call = self.calls.fetch_add(1, Ordering::Relaxed) + 1;
            let active = self.active.fetch_add(1, Ordering::Relaxed) + 1;
            self.maximum_active.fetch_max(active, Ordering::Relaxed);
            let source_name = if pcm.first().is_some_and(|sample| sample.abs() < 6_500) {
                "B"
            } else {
                "A"
            };
            let is_final_window = window.central_end_sample - window.central_start_sample
                > 4 * 16_000
                || (source_name == "A" && window.central_start_sample >= 120 * 16_000);
            self.call_sources
                .lock()
                .expect("call-source log mutex")
                .push(format!(
                    "{source_name}:{}",
                    if is_final_window { "final" } else { "preview" }
                ));
            if (self.hold_first_final_a && source_name == "A" && is_final_window)
                || (!self.hold_first_final_a && call == self.hold_call)
            {
                self.hold_first_final_a = false;
                let _ = self.held.send(source_name.to_owned());
                self.release
                    .recv()
                    .map_err(|error| format!("controlled release channel: {error}"))?;
            }
            self.active.fetch_sub(1, Ordering::Relaxed);
            let start_ms = ((window.central_start_sample - window.native_start_sample) / 16) as u32;
            Ok(vec![whisper_core::parakeet_ipc::Word {
                text: format!("{source_name}-final-{call}."),
                start_ms,
                end_ms: start_ms + 100,
            }])
        }
    }

    fn held_worker(
        hold_call: u64,
    ) -> (
        Box<dyn crate::parakeet_pipeline::WindowTranscriber>,
        Arc<AtomicU64>,
        Arc<AtomicU64>,
        Arc<std::sync::Mutex<Vec<String>>>,
        std::sync::mpsc::Receiver<String>,
        std::sync::mpsc::SyncSender<()>,
    ) {
        held_worker_with_mode(hold_call, false)
    }

    fn held_final_a_worker() -> (
        Box<dyn crate::parakeet_pipeline::WindowTranscriber>,
        Arc<AtomicU64>,
        Arc<AtomicU64>,
        Arc<std::sync::Mutex<Vec<String>>>,
        std::sync::mpsc::Receiver<String>,
        std::sync::mpsc::SyncSender<()>,
    ) {
        held_worker_with_mode(0, true)
    }

    fn held_worker_with_mode(
        hold_call: u64,
        hold_first_final_a: bool,
    ) -> (
        Box<dyn crate::parakeet_pipeline::WindowTranscriber>,
        Arc<AtomicU64>,
        Arc<AtomicU64>,
        Arc<std::sync::Mutex<Vec<String>>>,
        std::sync::mpsc::Receiver<String>,
        std::sync::mpsc::SyncSender<()>,
    ) {
        let calls = Arc::new(AtomicU64::new(0));
        let active = Arc::new(AtomicU64::new(0));
        let maximum_active = Arc::new(AtomicU64::new(0));
        let call_sources = Arc::new(std::sync::Mutex::new(Vec::new()));
        let (held, held_rx) = std::sync::mpsc::sync_channel(1);
        let (release_tx, release) = std::sync::mpsc::sync_channel(1);
        (
            Box::new(HeldUnifiedWorker {
                calls: Arc::clone(&calls),
                active: Arc::clone(&active),
                maximum_active: Arc::clone(&maximum_active),
                call_sources: Arc::clone(&call_sources),
                hold_call,
                hold_first_final_a,
                held,
                release,
            }),
            calls,
            maximum_active,
            call_sources,
            held_rx,
            release_tx,
        )
    }

    struct TailAwareUnifiedWorker;

    impl crate::parakeet_pipeline::WindowTranscriber for TailAwareUnifiedWorker {
        fn instance_id(&self) -> u64 {
            9011
        }

        fn transcribe(
            &mut self,
            window: whisper_core::parakeet_session::AudioWindow,
            _: &[i16],
        ) -> Result<Vec<whisper_core::parakeet_ipc::Word>, String> {
            let terminal = window.central_end_sample > 120 * 16_000;
            let start_ms = ((window.central_start_sample - window.native_start_sample) / 16) as u32;
            Ok(vec![whisper_core::parakeet_ipc::Word {
                text: if terminal {
                    "terminal-phrase."
                } else {
                    "intermediate-phrase."
                }
                .into(),
                start_ms,
                end_ms: start_ms + 100,
            }])
        }
    }

    struct ObservedTailAwareUnifiedWorker {
        windows: Arc<Mutex<Vec<(u64, u64, usize)>>>,
    }

    impl crate::parakeet_pipeline::WindowTranscriber for ObservedTailAwareUnifiedWorker {
        fn instance_id(&self) -> u64 {
            TailAwareUnifiedWorker.instance_id()
        }

        fn transcribe(
            &mut self,
            window: whisper_core::parakeet_session::AudioWindow,
            pcm: &[i16],
        ) -> Result<Vec<whisper_core::parakeet_ipc::Word>, String> {
            self.windows
                .lock()
                .expect("observed unified worker windows")
                .push((
                    window.native_start_sample,
                    window.native_end_sample,
                    pcm.len(),
                ));
            let mut inner = TailAwareUnifiedWorker;
            inner.transcribe(window, pcm)
        }
    }

    struct InvalidTimeUnifiedWorker;

    impl crate::parakeet_pipeline::WindowTranscriber for InvalidTimeUnifiedWorker {
        fn instance_id(&self) -> u64 {
            9002
        }
        fn transcribe(
            &mut self,
            _: whisper_core::parakeet_session::AudioWindow,
            _: &[i16],
        ) -> Result<Vec<whisper_core::parakeet_ipc::Word>, String> {
            Ok(vec![whisper_core::parakeet_ipc::Word {
                text: "invalid".into(),
                start_ms: 50,
                end_ms: 20,
            }])
        }
    }

    struct FailedUnifiedWorker;

    impl crate::parakeet_pipeline::WindowTranscriber for FailedUnifiedWorker {
        fn instance_id(&self) -> u64 {
            9003
        }
        fn transcribe(
            &mut self,
            _: whisper_core::parakeet_session::AudioWindow,
            _: &[i16],
        ) -> Result<Vec<whisper_core::parakeet_ipc::Word>, String> {
            Err("controlled worker failure".into())
        }
    }

    struct ExportCollisionWorker {
        path: PathBuf,
        transcribe_completed: Arc<AtomicU64>,
    }

    impl crate::parakeet_pipeline::WindowTranscriber for ExportCollisionWorker {
        fn instance_id(&self) -> u64 {
            9004
        }
        fn transcribe(
            &mut self,
            window: whisper_core::parakeet_session::AudioWindow,
            _: &[i16],
        ) -> Result<Vec<whisper_core::parakeet_ipc::Word>, String> {
            if let Some(parent) = self.path.parent() {
                std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            std::fs::write(&self.path, b"preexisting export").map_err(|error| error.to_string())?;
            self.transcribe_completed.store(1, Ordering::Release);
            let start_ms = ((window.central_start_sample - window.native_start_sample) / 16) as u32;
            Ok(vec![whisper_core::parakeet_ipc::Word {
                text: "export collision".into(),
                start_ms,
                end_ms: start_ms + 100,
            }])
        }
    }

    fn unified_import_request(root: &Path, source: &Path, id: u128) -> ImportRequest {
        let model = whisper_core::model_registry::model(
            whisper_core::model_catalog::ModelId::ParakeetTdt06BV3,
        );
        let model_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("models")
            .join(model.artifact.file_name);
        let model_sha256 = (0..32)
            .map(|index| {
                u8::from_str_radix(&model.artifact.sha256[index * 2..index * 2 + 2], 16).unwrap()
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        ImportRequest {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1,
            job_id: JobId(id),
            generation: Generation::first(),
            source_path: source.to_string_lossy().into_owned(),
            source_sha256: None,
            source_samples: None,
            model_path: model_path.to_string_lossy().into_owned(),
            model_sha256,
            destination: root.to_string_lossy().into_owned(),
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Cpu,
            },
        }
    }

    fn unified_live_request(root: &Path, id: u128) -> whisper_core::ports::LiveRequest {
        let model = whisper_core::model_registry::model(
            whisper_core::model_catalog::ModelId::ParakeetTdt06BV3,
        );
        let model_path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("models")
            .join(model.artifact.file_name);
        let model_sha256 = (0..32)
            .map(|index| {
                u8::from_str_radix(&model.artifact.sha256[index * 2..index * 2 + 2], 16).unwrap()
            })
            .collect::<Vec<_>>()
            .try_into()
            .unwrap();
        whisper_core::ports::LiveRequest {
            job_id: JobId(id),
            generation: Generation::first(),
            model_path: model_path.to_string_lossy().into_owned(),
            model_sha256,
            destination: root.to_string_lossy().into_owned(),
            group_offset_samples: 0,
            auto_stop_after_speech_samples: None,
            microphone: whisper_core::MicrophoneChoice::Device(format!("test:unified-{id}")),
            microphone_gain_db: 0,
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Cpu,
            },
        }
    }

    fn controlled_native_route(root: &Path) -> NativeUnifiedRoute {
        NativeUnifiedRoute {
            paths: crate::parakeet_import::ParakeetWorkerPaths {
                executable: root.join("controlled-worker.exe"),
                native_dir: root.join("native"),
                model: root.join("controlled-model.nemo"),
                runtime_root: root.to_path_buf(),
            },
            requested_compute: ComputeChoice::Cpu,
            compute: ComputeChoice::Cpu,
            expected_gpu: None,
            candidate: None,
            fallback_reason: None,
        }
    }
