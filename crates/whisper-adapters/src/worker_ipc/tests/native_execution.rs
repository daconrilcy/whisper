    #[test]
    fn native_window_pcm_requires_a_bounded_matching_riff_range() {
        let valid = whisper_core::parakeet_session::AudioWindow {
            sequence: 3,
            central_start_sample: 160,
            central_end_sample: 15_840,
            native_start_sample: 0,
            native_end_sample: 16_000,
        };
        assert_eq!(validate_native_window_pcm(valid, 16_000), Ok(16_000));
        assert!(validate_native_window_pcm(valid, 15_999).is_err());
        assert!(
            validate_native_window_pcm(
                whisper_core::parakeet_session::AudioWindow {
                    native_end_sample: whisper_core::parakeet_ipc::PARAKEET_MAX_AUDIO_SAMPLES as u64
                        + 1,
                    central_end_sample: whisper_core::parakeet_ipc::PARAKEET_MAX_AUDIO_SAMPLES as u64
                        + 1,
                    ..valid
                },
                whisper_core::parakeet_ipc::PARAKEET_MAX_AUDIO_SAMPLES + 1,
            )
            .is_err()
        );
        assert!(
            validate_native_window_pcm(
                whisper_core::parakeet_session::AudioWindow {
                    central_start_sample: 16_001,
                    ..valid
                },
                16_000,
            )
            .is_err()
        );
    }

    #[test]
    fn native_window_wav_is_short_pcm16_mono_16khz_riff() {
        let root = std::env::temp_dir().join(format!(
            "parakeet-native-window-wav-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let wav = root.join("window.wav");
        write_pcm16_wav(&wav, &[0x0102, -2]).unwrap();
        let bytes = std::fs::read(&wav).unwrap();
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 40);
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(u16::from_le_bytes(bytes[20..22].try_into().unwrap()), 1);
        assert_eq!(u16::from_le_bytes(bytes[22..24].try_into().unwrap()), 1);
        assert_eq!(
            u32::from_le_bytes(bytes[24..28].try_into().unwrap()),
            16_000
        );
        assert_eq!(u16::from_le_bytes(bytes[34..36].try_into().unwrap()), 16);
        assert_eq!(&bytes[36..40], b"data");
        assert_eq!(u32::from_le_bytes(bytes[40..44].try_into().unwrap()), 4);
        assert_eq!(&bytes[44..48], &[0x02, 0x01, 0xfe, 0xff]);
        assert_eq!(bytes.len(), 48);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn failed_ready_is_reaped_before_the_executor_recycles() {
        let starts = Arc::new(AtomicU64::new(0));
        let stops = Arc::new(AtomicU64::new(0));
        let mut executor = UnifiedInferenceExecutor::start(Box::new(StartFailsOnceUnifiedWorker {
            starts: Arc::clone(&starts),
            stops: Arc::clone(&stops),
        }))
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut ready_error = None;
        while Instant::now() < deadline {
            match executor.is_ready() {
                Err(error) => {
                    ready_error = Some(error);
                    break;
                }
                Ok(true) => panic!("the first instance must fail before READY"),
                Ok(false) => thread::sleep(Duration::from_millis(2)),
            }
        }
        assert!(
            ready_error
                .as_deref()
                .is_some_and(|error| error.contains("controlled READY attestation failure"))
        );
        executor.stop_and_reap().unwrap();
        assert_eq!(starts.load(Ordering::Relaxed), 1);
        assert_eq!(stops.load(Ordering::Relaxed), 1);

        executor.restart().unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut ready = false;
        while Instant::now() < deadline {
            if executor.is_ready().unwrap() {
                ready = true;
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        assert!(ready, "recycled instance did not reach READY");
        executor.stop_and_reap().unwrap();
        assert_eq!(starts.load(Ordering::Relaxed), 2);
        assert_eq!(stops.load(Ordering::Relaxed), 2);
    }

    #[test]
    #[cfg_attr(
        not(feature = "native-tests"),
        ignore = "requires models/parakeet-tdt-0.6b-v3.q8_0.gguf for model-catalogue archive preparation; mixed synthetic unified capture/import scenario"
    )]
    // TODO(DEBT-10): extract the synthetic scenario from its real model/catalogue prerequisite.
    fn native_auto_gpu_ready_failure_retries_once_on_cpu_before_first_call() {
        let root = std::env::temp_dir().join(format!(
            "parakeet-auto-startup-fallback-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source_path = root.join("source.wav");
        let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_path).unwrap();
        writer.append(&vec![0_i16; 3 * 16_000]).unwrap();
        writer.finish().unwrap();
        let mut request = unified_import_request(&root.join("archive"), &source_path, 9_430);
        request.config.compute = ComputeChoice::Auto;
        let session = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: request.job_id,
            generation: request.generation,
        };
        let source = crate::parakeet_audio::PcmPartStore::open(&source_path);
        let available_samples = source.total_samples().unwrap();
        let document = crate::parakeet_document::ParakeetDocumentStore::create(
            root.join("document"),
            request.job_id,
            request.generation,
        )
        .unwrap();
        let mut state = UnifiedSessionState {
            request: request.clone(),
            document_drain_start_sequence: 1,
            source: UnifiedSessionSource::Closed(source),
            available_samples,
            durable_samples: available_samples,
            source_closed: true,
            document,
            inference_directory: root.join("inference"),
            native_route: Some(NativeUnifiedRoute {
                paths: crate::parakeet_import::ParakeetWorkerPaths {
                    executable: root.join("gpu-worker.exe"),
                    native_dir: root.join("gpu"),
                    model: root.join("model.nemo"),
                    runtime_root: root.clone(),
                },
                requested_compute: ComputeChoice::Auto,
                compute: ComputeChoice::Gpu,
                expected_gpu: None,
                candidate: None,
                fallback_reason: None,
            }),
            backend_ready_reported: false,
            finalizer: crate::parakeet_pipeline::ParakeetFinalizer::new(
                request.job_id,
                request.generation,
            ),
            pending_call: None,
            pending_prepared: None,
            preview_sequence: 0,
            pending_window: None,
            pending_available_samples: None,
            failure: None,
        };
        let mut manual_request =
            unified_import_request(&root.join("manual-archive"), &source_path, 9_431);
        manual_request.config.compute = ComputeChoice::Gpu;
        let manual_capture_generation = PathBuf::from(&manual_request.destination)
            .join("transcriptions")
            .join(format!("{:032x}", manual_request.job_id.0))
            .join(manual_request.generation.get().to_string());
        let manual_capture_source_path = manual_capture_generation
            .join("parakeet-unified")
            .join("source.wav");
        std::fs::create_dir_all(manual_capture_source_path.parent().unwrap()).unwrap();
        manual_request.source_path = manual_capture_source_path.to_string_lossy().into_owned();
        let mut manual_capture =
            crate::capture::CaptureStream::empty_test_capture(16_000, 1, "test:manual-gpu-capture");
        manual_capture.enqueue_test_block(vec![0.25; 16_000]);
        manual_capture.enqueue_test_block(vec![0.5; 16_000]);
        let mut manual_source =
            crate::parakeet_live::UnifiedCapturePump::from_capture_with_test_part_limit(
                manual_capture,
                manual_capture_source_path.clone(),
                32_000,
            )
            .unwrap();
        let initial_ack_deadline = Instant::now() + Duration::from_secs(5);
        let mut initial_written_samples = 0;
        let mut initial_durable_samples = 0;
        while initial_written_samples == 0 {
            for acknowledgement in manual_source.pump().unwrap() {
                initial_written_samples = initial_written_samples.max(acknowledgement.written_samples);
                initial_durable_samples = initial_durable_samples.max(acknowledgement.flushed_samples);
            }
            assert!(
                Instant::now() < initial_ack_deadline,
                "manual capture did not acknowledge its first PCM blocks"
            );
            if initial_written_samples == 0 {
                thread::sleep(Duration::from_millis(2));
            }
        }
        manual_source.enqueue_test_block(vec![0.75; 16_000]);
        let manual_session = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: manual_request.job_id,
            generation: manual_request.generation,
        };
        let manual_document = crate::parakeet_document::ParakeetDocumentStore::create(
            root.join("manual-document"),
            manual_request.job_id,
            manual_request.generation,
        )
        .unwrap();
        let manual_state = UnifiedSessionState {
            request: manual_request.clone(),
            document_drain_start_sequence: 1,
            source: UnifiedSessionSource::Capture(Box::new(manual_source)),
            available_samples: initial_written_samples,
            durable_samples: initial_durable_samples,
            source_closed: false,
            document: manual_document,
            inference_directory: root.join("manual-inference"),
            native_route: Some(NativeUnifiedRoute {
                paths: crate::parakeet_import::ParakeetWorkerPaths {
                    executable: root.join("gpu-worker.exe"),
                    native_dir: root.join("gpu"),
                    model: root.join("model.nemo"),
                    runtime_root: root.clone(),
                },
                requested_compute: ComputeChoice::Gpu,
                compute: ComputeChoice::Gpu,
                expected_gpu: None,
                candidate: None,
                fallback_reason: None,
            }),
            backend_ready_reported: false,
            finalizer: crate::parakeet_pipeline::ParakeetFinalizer::new(
                manual_request.job_id,
                manual_request.generation,
            ),
            pending_call: None,
            pending_prepared: None,
            preview_sequence: 0,
            pending_window: None,
            pending_available_samples: None,
            failure: None,
        };
        let mut scheduler = whisper_core::parakeet_scheduler::ParakeetScheduler::default();
        scheduler.admit(session).unwrap();
        scheduler.admit(manual_session).unwrap();
        scheduler.queue_final(session, available_samples).unwrap();
        let call = scheduler.next_call().unwrap().unwrap();
        let mut sessions = HashMap::new();
        sessions.insert(manual_session, manual_state);
        let mut archives = HashMap::new();
        archives.insert(
            manual_session,
            ArchiveStore::new(&manual_request.destination),
        );
        let mut executor_route = None;
        let gpu_starts = Arc::new(AtomicU64::new(0));
        let gpu_stops = Arc::new(AtomicU64::new(0));
        let child_observations = Arc::new(Mutex::new(Vec::new()));
        let mut executor = UnifiedInferenceExecutor::start_observed(
            Box::new(StartFailsOnceUnifiedWorker {
                starts: Arc::clone(&gpu_starts),
                stops: Arc::clone(&gpu_stops),
            }),
            Arc::clone(&child_observations),
        )
        .unwrap();
        let startup_deadline = Instant::now() + Duration::from_secs(5);
        let startup_error = loop {
            match executor.is_ready() {
                Err(error) => break error,
                Ok(true) => panic!("the controlled GPU startup must fail before READY"),
                Ok(false) if Instant::now() < startup_deadline => {
                    thread::sleep(Duration::from_millis(2));
                }
                Ok(false) => panic!("the controlled GPU startup did not resolve"),
            }
        };
        assert!(startup_error.contains("controlled READY attestation failure"));
        assert_eq!(gpu_starts.load(Ordering::Relaxed), 1);
        assert_eq!(gpu_stops.load(Ordering::Relaxed), 1);

        let calls = Arc::new(AtomicU64::new(0));
        let mut cpu_route_requests = Vec::new();
        let retry = retry_native_gpu_startup_on_cpu(
            runtime::NativeRetryState {
                scheduler: &mut scheduler,
                sessions: &mut sessions,
                archives: &mut archives,
                executor_route: &mut executor_route,
                executor: &mut executor,
            },
            &mut state,
            call,
            &startup_error,
            |request, reason| {
                cpu_route_requests.push(request.job_id);
                Ok(NativeUnifiedRoute {
                    paths: crate::parakeet_import::ParakeetWorkerPaths {
                        executable: root.join("cpu-worker.exe"),
                        native_dir: root.join("cpu"),
                        model: PathBuf::from(request.model_path.as_str()),
                        runtime_root: root.clone(),
                    },
                    requested_compute: request.config.compute,
                    compute: ComputeChoice::Cpu,
                    expected_gpu: None,
                    candidate: None,
                    fallback_reason: Some(reason.to_owned()),
                })
            },
            |_| {
                Box::new(ControlledUnifiedWorker {
                    calls: Some(Arc::clone(&calls)),
                })
            },
        )
        .unwrap()
        .expect("Auto GPU startup failure must retry CPU and terminate a manual GPU peer");
        let retry_call = retry.call;
        assert_eq!(retry.failed_manual_gpu.len(), 1);
        let manual_failure = retry.failed_manual_gpu.into_iter().next().unwrap();
        assert_eq!(manual_failure.session, manual_session);
        assert!(
            manual_failure
                .message
                .contains("manual capture drained and closed")
        );
        assert!(
            manual_failure
                .message
                .contains("manual GPU sessions do not fall back to CPU")
        );
        assert!(manual_failure.message.contains(&startup_error));
        let failure_event = manual_gpu_failure_event(manual_failure);
        assert!(matches!(
            failure_event,
            ImportEvent::Failed { job_id, generation, message }
                if job_id == manual_session.job_id
                    && generation == manual_session.generation
                    && message.contains("manual capture drained and closed")
        ));
        assert!(!sessions.contains_key(&manual_session));
        assert!(archives.contains_key(&manual_session));
        let manual_audio = crate::parakeet_audio::PcmPartStore::open(&manual_capture_source_path);
        let final_manual_samples = manual_audio.total_samples().unwrap();
        assert_eq!(final_manual_samples, 48_000);
        assert_eq!(
            manual_audio
                .read_window(0..final_manual_samples)
                .unwrap()
                .len() as u64,
            final_manual_samples
        );
        let manual_parts = manual_audio.read_page(0, 128).unwrap();
        assert!(
            manual_parts.len() > 1,
            "stopped manual capture must retain its rotated parts"
        );
        let mut manual_part_cursor = 0_u64;
        for part in &manual_parts {
            assert_eq!(part.start_sample, manual_part_cursor);
            assert!(part.end_sample > part.start_sample);
            assert!(part.path.is_file());
            manual_part_cursor = part.end_sample;
        }
        assert_eq!(manual_part_cursor, final_manual_samples);
        assert!(
            manual_capture_generation
                .join("captured-source-parts.json")
                .is_file()
        );
        assert!(root.join("manual-document/document.jsonl").is_file());

        let mut reference_capture = crate::capture::CaptureStream::empty_test_capture(
            16_000,
            1,
            "test:manual-gpu-capture-reference",
        );
        for level in [0.25_f32, 0.5, 0.75] {
            reference_capture.enqueue_test_block(vec![level; 16_000]);
        }
        let mut reference_converter = crate::capture::Pcm16Converter::new(16_000, 1).unwrap();
        let mut expected_pcm = Vec::with_capacity(48_000);
        while let Some(block) = reference_capture.try_next_block() {
            expected_pcm.extend(reference_converter.push(&block).unwrap());
            reference_capture.recycle_block(block).unwrap();
        }
        expected_pcm.extend(reference_converter.finish().unwrap());
        assert_eq!(expected_pcm.len(), 48_000);
        let actual_pcm = manual_audio.read_window(0..48_000).unwrap();
        assert_eq!(actual_pcm.len(), expected_pcm.len());
        assert!(
            actual_pcm
                .iter()
                .zip(&expected_pcm)
                .all(|(actual, expected)| actual.abs_diff(*expected) <= 2)
        );

        let manual_manifest: serde_json::Value = serde_json::from_slice(
            &std::fs::read(manual_capture_generation.join("captured-source-parts.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(manual_manifest["source_samples"].as_u64(), Some(48_000));
        let manifest_sha = manual_manifest["source_sha256"].as_str().unwrap();
        assert_eq!(manifest_sha.len(), 64);
        assert!(manifest_sha.bytes().all(|byte| byte.is_ascii_hexdigit()));
        let manifest_parts = manual_manifest["parts"].as_array().unwrap();
        assert_eq!(manifest_parts.len(), manual_parts.len());
        for (index, (reference, part)) in manifest_parts.iter().zip(&manual_parts).enumerate() {
            assert_eq!(reference["number"].as_u64(), Some(index as u64 + 1));
            assert_eq!(reference["start_sample"].as_u64(), Some(part.start_sample));
            assert_eq!(reference["end_sample"].as_u64(), Some(part.end_sample));
            assert_eq!(
                reference["path"].as_str(),
                Some(part.path.to_string_lossy().as_ref())
            );
            assert_eq!(reference["sha256"].as_str().unwrap().len(), 64);
            let part_bytes = std::fs::read(&part.path).unwrap();
            assert_eq!(reference["bytes"].as_u64(), Some(part_bytes.len() as u64));
            let expected_part_sha = copy_resume_hex(&Sha256::digest(&part_bytes));
            assert_eq!(
                reference["sha256"].as_str(),
                Some(expected_part_sha.as_str())
            );
        }
        let manual_destination = Path::new(&manual_request.destination);
        let manual_archive_entry = ArchiveStore::scan(manual_destination)
            .unwrap()
            .into_iter()
            .find(|entry| {
                entry.job_id == manual_session.job_id && entry.generation == manual_session.generation
            })
            .expect("failed manual GPU peer keeps its recoverable archive entry");
        assert!(!manual_archive_entry.complete);
        assert_eq!(manual_archive_entry.source_sha256, manifest_sha);
        assert_eq!(
            scheduler.active_sessions(),
            1,
            "manual GPU peer must be terminally removed after reap"
        );
        assert_eq!(
            cpu_route_requests,
            vec![request.job_id],
            "manual GPU must never be converted to CPU"
        );
        assert_eq!(retry_call.session, session);
        assert_ne!(retry_call.incarnation, call.incarnation);
        assert_eq!(retry_call.request_id, call.request_id);
        assert_eq!(retry_call.attempt, call.attempt);
        assert!(
            state
                .native_route
                .as_ref()
                .unwrap()
                .backend_label()
                .contains("CPU (Auto fallback before audio: GPU pre-audio startup failed")
        );
        assert_eq!(request.config.compute, ComputeChoice::Auto);
        assert_eq!(
            state.native_route.as_ref().unwrap().requested_compute,
            ComputeChoice::Auto
        );
        assert!(!can_retry_native_auto_on_cpu(
            ComputeChoice::Auto,
            ComputeChoice::Cpu,
            false,
            false,
        ));
        assert!(!can_retry_native_auto_on_cpu(
            ComputeChoice::Gpu,
            ComputeChoice::Gpu,
            false,
            false,
        ));
        assert!(!can_retry_native_auto_on_cpu(
            ComputeChoice::Auto,
            ComputeChoice::Gpu,
            true,
            false,
        ));

        let ready_deadline = Instant::now() + Duration::from_secs(5);
        let mut cpu_ready = false;
        while Instant::now() < ready_deadline {
            if executor.is_ready().unwrap() {
                cpu_ready = true;
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        assert!(cpu_ready, "CPU retry must reach READY");
        scheduler.begin_call(retry_call).unwrap();
        executor
            .dispatch(UnifiedInferenceCommand {
                call: retry_call,
                window: whisper_core::parakeet_session::AudioWindow {
                    sequence: 1,
                    central_start_sample: 0,
                    central_end_sample: 16_000,
                    native_start_sample: 0,
                    native_end_sample: 16_000,
                },
                pcm: vec![0; 16_000],
                scratch_directory: root.join("inference"),
            })
            .unwrap();
        let result_deadline = Instant::now() + Duration::from_secs(5);
        let result = loop {
            if let Some(result) = executor.try_result().unwrap() {
                break result;
            }
            assert!(
                Instant::now() < result_deadline,
                "CPU retry inference timed out"
            );
            thread::sleep(Duration::from_millis(2));
        };
        assert_eq!(result.call, retry_call);
        assert!(result.words.is_ok(), "CPU retry must complete inference");
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        assert_eq!(
            gpu_starts.load(Ordering::Relaxed),
            1,
            "GPU must not be retried"
        );
        let observations = child_observations.lock().unwrap().clone();
        let first_reaped = observations
            .iter()
            .position(|observation| observation.incarnation == 1 && observation.event == "Reaped")
            .expect("failed GPU startup must be actually reaped");
        let cpu_spawned = observations
            .iter()
            .position(|observation| observation.incarnation == 2 && observation.event == "Spawn")
            .expect("CPU retry must create a new executor incarnation");
        assert!(
            first_reaped < cpu_spawned,
            "the failed child must be reaped before CPU spawn"
        );
        scheduler.complete(retry_call).unwrap();
        executor.stop_and_reap().unwrap();
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn prepared_native_routes_without_start_are_bounded_and_stop_releases_identity() {
        let root = std::env::temp_dir().join(format!(
            "parakeet-native-prepared-route-bound-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let (worker, _) = controlled_worker();
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(worker);
        let route = |id: u128| NativeUnifiedRoute {
            paths: crate::parakeet_import::ParakeetWorkerPaths {
                executable: root.join(format!("worker-{id}.exe")),
                native_dir: root.join(format!("native-{id}")),
                model: root.join("model.nemo"),
                runtime_root: root.clone(),
            },
            requested_compute: ComputeChoice::Auto,
            compute: ComputeChoice::Cpu,
            expected_gpu: None,
            candidate: None,
            fallback_reason: Some("pre-audio CPU route".into()),
        };
        let session = |id| whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: JobId(id),
            generation: Generation::first(),
        };

        runtime
            .register_prepared_native_route(session(9_431), route(9_431))
            .unwrap();
        runtime
            .register_prepared_native_route(session(9_432), route(9_432))
            .unwrap();
        runtime.native_executor_route = Some(route(9_431));
        assert_eq!(runtime.native_unified_routes.len(), 2);
        assert!(runtime.unified_admission_full());
        assert!(
            runtime
                .register_prepared_native_route(session(9_433), route(9_433))
                .is_err()
        );
        assert_eq!(runtime.native_unified_routes.len(), 2);
        let third =
            unified_import_request(&root.join("archive"), Path::new("missing-third.wav"), 9_433);
        let mut prepare_events = VecDeque::new();
        let error = runtime
            .prepare(third.clone(), None, &mut prepare_events)
            .unwrap_err();
        assert!(error.2.contains("SchedulerCapacity"));
        assert!(
            !root
                .join("archive/transcriptions")
                .join(format!("{:032x}", third.job_id.0))
                .exists()
        );

        let source_path = root.join("prepared-source.wav");
        let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_path).unwrap();
        writer.append(&vec![0_i16; 3 * 16_000]).unwrap();
        writer.finish().unwrap();
        let first_request = unified_import_request(&root.join("archive"), &source_path, 9_431);
        runtime
            .start_controlled_unified_import(&first_request)
            .unwrap();
        let first_session = session(9_431);
        assert!(runtime.unified_sessions.contains_key(&first_session));
        assert!(!runtime.native_unified_routes.contains_key(&first_session));
        assert_eq!(runtime.native_unified_routes.len(), 1);

        let mut events = VecDeque::new();
        runtime.effect(
            ImportEffect::Stop {
                job_id: JobId(9_432),
                generation: Generation::first(),
            },
            &mut events,
        );
        assert!(
            matches!(events.pop_front(), Some(ImportEvent::Stopped { job_id, .. }) if job_id == JobId(9_432))
        );
        assert!(!runtime.native_unified_routes.contains_key(&session(9_432)));
        assert!(runtime.native_executor_route.is_some());
        assert_eq!(runtime.native_unified_routes.len(), 0);
        runtime
            .register_prepared_native_route(session(9_433), route(9_433))
            .unwrap();
        assert_eq!(runtime.native_unified_routes.len(), 1);
        assert!(
            runtime
                .register_prepared_native_route(session(9_434), route(9_434))
                .is_err()
        );
        drop(runtime);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn native_executor_backend_changes_only_after_the_prior_instance_is_reaped() {
        let root = std::env::temp_dir().join(format!(
            "parakeet-native-route-recycle-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source_path = root.join("source.wav");
        let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_path).unwrap();
        writer.append(&vec![0_i16; 3 * 16_000]).unwrap();
        writer.finish().unwrap();
        let request = unified_import_request(&root.join("archive"), &source_path, 9_435);
        let make_route = |compute| NativeUnifiedRoute {
            paths: crate::parakeet_import::ParakeetWorkerPaths {
                executable: root.join(format!("{compute:?}-worker.exe")),
                native_dir: root.join(format!("{compute:?}-native")),
                model: PathBuf::from(&request.model_path),
                runtime_root: root.clone(),
            },
            requested_compute: compute,
            compute,
            expected_gpu: None,
            candidate: None,
            fallback_reason: None,
        };

        let (worker, _) = controlled_worker();
        let mut recycled = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(worker);
        recycled.native_executor_route = Some(make_route(ComputeChoice::Gpu));
        let mut executor = UnifiedInferenceExecutor::start(controlled_worker().0).unwrap();
        wait_until_ready_then_reap(&mut executor);
        recycled.unified_executor = Some(executor);
        let session = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: request.job_id,
            generation: request.generation,
        };
        recycled
            .register_prepared_native_route(session, make_route(ComputeChoice::Cpu))
            .unwrap();
        recycled.start_controlled_unified_import(&request).unwrap();
        assert!(recycled.unified_sessions.contains_key(&session));
        assert!(!recycled.native_unified_routes.contains_key(&session));
        let ready_deadline = Instant::now() + Duration::from_secs(5);
        let ready = loop {
            match recycled.poll_unified_sessions() {
                Some(event) => break event,
                None => {
                    assert!(
                        Instant::now() < ready_deadline,
                        "replacement CPU route must start after the GPU instance was reaped"
                    );
                    thread::sleep(Duration::from_millis(2));
                }
            }
        };
        assert!(matches!(
            ready,
            Ok(ImportEvent::Ready { job_id, backend, .. })
                if job_id == request.job_id && backend == "CPU"
        ));
        assert!(
            recycled
                .unified_executor
                .as_ref()
                .is_some_and(|executor| executor.thread.is_some())
        );

        let (worker, _) = controlled_worker();
        let mut overlapping = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(worker);
        overlapping.native_executor_route = Some(make_route(ComputeChoice::Gpu));
        overlapping.unified_executor =
            Some(UnifiedInferenceExecutor::start(controlled_worker().0).unwrap());
        overlapping
            .register_prepared_native_route(session, make_route(ComputeChoice::Cpu))
            .unwrap();
        let error = overlapping
            .start_controlled_unified_import(&request)
            .unwrap_err();
        assert!(error.contains("NativeRouteBusy"));
        assert!(overlapping.unified_sessions.is_empty());
        overlapping
            .unified_executor
            .as_mut()
            .unwrap()
            .stop_and_reap()
            .unwrap();
        std::fs::remove_dir_all(&root).unwrap();
    }

    fn wait_until_ready_then_reap(executor: &mut UnifiedInferenceExecutor) {
        let ready_deadline = Instant::now() + Duration::from_secs(5);
        while !executor.is_ready().unwrap() {
            assert!(
                Instant::now() < ready_deadline,
                "initial worker did not become ready"
            );
            thread::sleep(Duration::from_millis(2));
        }
        executor.stop_and_reap().unwrap();
    }

    #[cfg(windows)]
    fn native_personal_gpu_candidate_for_harness(
        workers: &WorkerPaths,
    ) -> whisper_core::gpu_inventory::GpuRuntimeIdentity {
        native_personal_gpu_candidate_for_generation_harness(workers, Generation::first())
    }

    #[cfg(windows)]
    fn native_personal_gpu_candidate_for_generation_harness(
        workers: &WorkerPaths,
        request_generation: Generation,
    ) -> whisper_core::gpu_inventory::GpuRuntimeIdentity {
        use whisper_core::gpu_inventory_app::GpuInventoryPort;

        let mut inventory = crate::gpu_inventory::WindowsGpuInventoryPort::new(workers.gpu.clone());
        inventory
            .start(request_generation.get())
            .expect("start the real GPU identity inventory worker");
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if let Some((result_generation, result)) = inventory.poll_runtime_identities() {
                assert_eq!(result_generation, request_generation.get());
                return result
                    .expect("real GPU identity inventory must complete")
                    .into_iter()
                    .find(|identity| {
                        identity.pnp_instance_id
                            == whisper_core::model_registry::PARAKEET_PERSONAL_GPU_PNP_INSTANCE_ID
                    })
                    .expect("registered personal GPU must be observed by the real inventory worker");
            }
            assert!(
                Instant::now() < deadline,
                "real GPU identity inventory timed out after its bounded wait"
            );
            thread::sleep(Duration::from_millis(10));
        }
    }

    #[cfg(windows)]
    #[derive(serde::Deserialize)]
    struct NativeLongAnnotation {
        schema: String,
        language: String,
        sample_rate_hz: u32,
        channels: u16,
        source_samples: u64,
        source_sha256: String,
        segments: Vec<NativeLongAnnotationSegment>,
    }

    #[cfg(windows)]
    #[derive(serde::Deserialize)]
    struct NativeLongAnnotationSegment {
        source_path: String,
        start_sample: u64,
        end_sample: u64,
        reference_text: String,
    }

    #[cfg(windows)]
    fn native_queued_import_source(execution_root: &Path, long: bool) -> (PathBuf, u64, String) {
        let (variable, expected_path, annotation_path) = if long {
            (
                "WHISPER_PU_L4_LONG_FR",
                execution_root.join("fixtures/fr-long-35m.wav"),
                Some(execution_root.join("fixtures/fr-long-35m.annotations.json")),
            )
        } else {
            ("WHISPER_PU_L4_FR_SOURCE", PathBuf::new(), None)
        };
        let configured = std::env::var_os(variable)
            .map(PathBuf::from)
            .unwrap_or_else(|| panic!("{variable} must name the guarded French WAV fixture"));
        let source = configured
            .canonicalize()
            .unwrap_or_else(|error| panic!("canonicalize {variable} fixture: {error}"));
        assert!(
            source.starts_with(execution_root),
            "{variable} must remain under the guarded execution root: {}",
            source.display()
        );
        if long {
            assert_eq!(
                source,
                expected_path
                    .canonicalize()
                    .expect("planned long French WAV must be staged before native execution")
            );
        }
        let samples = crate::archive::source_output_samples(&source)
            .unwrap_or_else(|error| panic!("inspect {variable} source: {error}"));
        let sha256 = crate::parakeet_import::sha256_file(&source)
            .unwrap_or_else(|error| panic!("hash {variable} source: {error}"));
        if long {
            let annotation_path = annotation_path.expect("long fixture annotation path");
            let annotation: NativeLongAnnotation = serde_json::from_slice(
                &fs::read(&annotation_path).expect("read paired long French annotation"),
            )
            .expect("parse paired long French annotation");
            assert_eq!(annotation.schema, "pu-l4-corpus-v1");
            assert_eq!(annotation.language, "fr");
            assert_eq!(annotation.sample_rate_hz, 16_000);
            assert_eq!(annotation.channels, 1);
            assert_eq!(annotation.source_samples, samples);
            assert!(annotation.source_sha256.eq_ignore_ascii_case(&sha256));
            assert!(samples >= 35 * 60 * 16_000);
            let mut covered_samples = 0_u64;
            for segment in &annotation.segments {
                assert_eq!(segment.start_sample, covered_samples);
                assert!(segment.end_sample > segment.start_sample);
                assert!(segment.end_sample <= samples);
                assert!(!segment.reference_text.trim().is_empty());
                assert!(segment.source_path.starts_with("sound example/"));
                covered_samples = segment.end_sample;
            }
            assert_eq!(covered_samples, samples);
        } else {
            assert!(samples > 0);
            assert!(samples <= whisper_core::parakeet_windows::MAX_NATIVE_SAMPLES);
        }
        (source, samples, sha256)
    }

    #[cfg(windows)]
    fn assert_native_generation_text_artifacts(
        archive_root: &Path,
        entry: &crate::archive::ArchiveEntry,
    ) -> (String, String) {
        #[derive(serde::Deserialize)]
        struct ManifestArtifact {
            path: String,
            sha256: String,
            bytes: u64,
        }

        #[derive(serde::Deserialize)]
        struct CompleteManifest {
            version: u32,
            job_id: u128,
            generation: u64,
            source_sha256: String,
            segments: u64,
            text: ManifestArtifact,
            srt: ManifestArtifact,
        }

        assert!(entry.complete, "native archive must be complete");
        let generation = archive_root
            .join("transcriptions")
            .join(format!("{:032x}", entry.job_id.0))
            .join(entry.generation.get().to_string());
        assert_eq!(
            std::fs::read(generation.join("CURRENT")).expect("read published CURRENT"),
            b"manifest.json\n"
        );
        let manifest: CompleteManifest = serde_json::from_slice(
            &std::fs::read(generation.join("manifest.json")).expect("read complete manifest"),
        )
        .expect("parse complete manifest");
        assert_eq!(manifest.version, 1);
        assert_eq!(manifest.job_id, entry.job_id.0);
        assert_eq!(manifest.generation, entry.generation.get());
        assert_eq!(manifest.source_sha256, entry.source_sha256);
        assert!(manifest.segments > 0);

        let generation = generation
            .canonicalize()
            .expect("resolve complete generation directory");
        let read_artifact = |artifact: &ManifestArtifact, expected_name: &str| {
            assert_eq!(artifact.path, expected_name);
            let path = generation.join(&artifact.path);
            let canonical = path.canonicalize().expect("resolve manifest artifact");
            assert!(canonical.starts_with(&generation));
            let bytes = std::fs::read(&canonical).expect("read published manifest artifact");
            assert!(
                !bytes.is_empty(),
                "published {expected_name} must not be empty"
            );
            assert_eq!(
                artifact.bytes,
                bytes.len() as u64,
                "published {expected_name} byte count must match its manifest"
            );
            assert_eq!(
                crate::parakeet_import::sha256_file(&canonical)
                    .expect("hash published manifest artifact"),
                artifact.sha256,
                "published {expected_name} SHA-256 must match its manifest"
            );
            String::from_utf8(bytes).expect("published text artifact must be UTF-8")
        };
        (
            read_artifact(&manifest.text, "transcript.txt"),
            read_artifact(&manifest.srt, "transcript.srt"),
        )
    }

    #[cfg(windows)]
    fn run_native_queued_unified_import(
        compute: ComputeChoice,
        long: bool,
        include_current_gpu_candidate: bool,
    ) {
        let execution_root = std::env::var_os("WHISPER_L04_CAMPAIGN_RUN")
            .map(PathBuf::from)
            .expect("guarded PU-L4 execution root is required")
            .canonicalize()
            .expect("guarded execution root must exist");
        let local_app_data = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .expect("isolated LOCALAPPDATA is required")
            .canonicalize()
            .expect("isolated LOCALAPPDATA must exist");
        let whisper = local_app_data.join("Whisper");
        let app_root = whisper.join("app");
        let models_root = whisper.join("models");
        let cpu_paths = crate::parakeet_import::ParakeetWorkerPaths {
            executable: whisper.join("worker/whisper-worker-parakeet-cpu.exe"),
            native_dir: whisper.join("native/cpu"),
            model: models_root.join("parakeet-tdt-0.6b-v3.q8_0.gguf"),
            runtime_root: whisper.clone(),
        };
        let gpu_paths = crate::parakeet_import::ParakeetWorkerPaths {
            executable: whisper.join("worker/whisper-worker-parakeet-gpu.exe"),
            native_dir: whisper.join("native/gpu"),
            model: cpu_paths.model.clone(),
            runtime_root: whisper.clone(),
        };
        let workers = WorkerPaths {
            cpu: app_root.join("whisper-worker-cpu.exe"),
            gpu: app_root.join("whisper-worker-gpu.exe"),
        };
        assert!(
            workers.cpu.is_file(),
            "personal Turbo CPU worker is missing"
        );
        assert!(
            workers.gpu.is_file(),
            "personal Turbo GPU worker is missing"
        );
        assert!(
            cpu_paths.validate().is_ok(),
            "personal Parakeet CPU route is invalid"
        );
        assert!(
            gpu_paths.validate().is_ok(),
            "personal Parakeet GPU route is invalid"
        );

        let (source_path, expected_samples, source_sha) =
            native_queued_import_source(&execution_root, long);
        let candidate = if include_current_gpu_candidate {
            Some(native_personal_gpu_candidate_for_harness(&workers))
        } else {
            None
        };
        assert!(
            compute != ComputeChoice::Gpu || candidate.is_some(),
            "manual GPU queued import requires a current inventory identity"
        );
        let run_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock is after the epoch")
            .as_nanos();
        let job_id = JobId((u128::from(std::process::id()) << 96) | run_id);
        let generation = Generation::first();
        let archive_root = execution_root.join(format!(
            "queued-{}-{}-{:032x}",
            match compute {
                ComputeChoice::Cpu => "cpu",
                ComputeChoice::Gpu => "gpu",
                ComputeChoice::Auto => "auto",
            },
            if long { "fr-long" } else { "fr-short" },
            job_id.0
        ));
        assert!(
            !archive_root.exists(),
            "native test archive destination is fresh"
        );

        let model_catalog = Some(RuntimeModelCatalog {
            catalog: crate::model_catalog::LocalModelCatalog::new(models_root.clone()),
            known_historical_turbo_path: models_root
                .join("ggml-large-v3-turbo.bin")
                .to_string_lossy()
                .into_owned(),
            parakeet_paths: Some(cpu_paths.clone()),
            gpu_parakeet_paths: Some(gpu_paths.clone()),
        });
        let snapshot_registry_root = execution_root.join("settings");
        let (observer, native_events) = NativeQualificationObserver::channel();
        let native_observer = Arc::clone(&observer);
        let mut runtime = ImportRuntime::new_with_workers_and_parakeet_registry(
            workers.clone(),
            Arc::new(PublicationGate::default()),
            model_catalog,
            WorkerRuntimeConfig {
                parakeet_paths: Some(cpu_paths.clone()),
                gpu_parakeet_paths: Some(gpu_paths.clone()),
                snapshot_registry_root: Some(snapshot_registry_root),
                require_snapshot_registry: true,
                native_unified_candidate_enabled: true,
            },
        )
        .with_native_harness_observer(observer);
        let mut request = unified_import_request(&archive_root, &source_path, job_id.0);
        request.job_id = job_id;
        request.generation = generation;
        request.model_path = cpu_paths.model.to_string_lossy().into_owned();
        request.config.compute = compute;
        let expected_effective_compute = match compute {
            ComputeChoice::Cpu => ComputeChoice::Cpu,
            ComputeChoice::Gpu => ComputeChoice::Gpu,
            ComputeChoice::Auto if candidate.is_some() => ComputeChoice::Gpu,
            ComputeChoice::Auto => ComputeChoice::Cpu,
        };
        let expected_fallback = (compute == ComputeChoice::Auto
            && expected_effective_compute == ComputeChoice::Cpu)
            .then_some("No current GPU candidate was available before audio");
        let expected_backend = match (expected_effective_compute, expected_fallback) {
            (ComputeChoice::Gpu, _) => "CUDA0".to_owned(),
            (ComputeChoice::Cpu, Some(reason)) => {
                format!("CPU (Auto fallback before audio: {reason})")
            }
            (ComputeChoice::Cpu, None) => "CPU".to_owned(),
            (ComputeChoice::Auto, _) => unreachable!("effective compute is resolved before start"),
        };
        let expected_ready_backend = match expected_effective_compute {
            ComputeChoice::Gpu => "CUDA0",
            ComputeChoice::Cpu => "CPU",
            ComputeChoice::Auto => unreachable!("effective compute is resolved before start"),
        };
        let expected_backend_message = format!("Moteur actif : {expected_backend}");
        let expected_gpu_key = (expected_effective_compute == ComputeChoice::Gpu).then(|| {
            gpu_paths
                .gpu_qualification_key(whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS)
                .expect("effective GPU route must have a qualification key")
        });
        if let Some(candidate) = candidate.as_ref() {
            assert_eq!(candidate.generation, generation.get());
            runtime
                .gpu_candidates
                .insert((job_id, generation.get()), Some(candidate.clone()));
        }

        let mut app = ImportApplication::new(RuntimeImportPort {
            runtime,
            events: VecDeque::new(),
            capture_progress: None,
            document_revision: None,
            unified_stop_submissions: None,
            controlled_capture_blocks: None,
        });
        app.dispatch(AppCommand::EnqueueImport {
            request: request.clone(),
        })
        .expect("native queued import must enter the durable application queue");
        let identity = (job_id, generation);
        let enqueue_deadline = Instant::now() + Duration::from_secs(30);
        let queued = loop {
            let view = app
                .dispatch(AppCommand::Refresh)
                .expect("refresh durable native queue");
            if let Some(entry) = view.queue.iter().find(|entry| {
                (entry.request.job_id, entry.request.generation) == identity
                    && matches!(entry.status, whisper_core::ports::ImportQueueStatus::Queued)
            }) {
                break entry.request.clone();
            }
            assert!(
                Instant::now() < enqueue_deadline,
                "native Enqueue did not persist the expected identity; view={view:?}"
            );
            thread::sleep(Duration::from_millis(5));
        };
        assert_eq!(
            queued.source_sha256,
            Some(decode_sha256(&source_sha).expect("native fixture SHA-256 must decode"))
        );
        assert_eq!(queued.source_samples, Some(expected_samples));
        assert_eq!(queued.strategy, request.strategy);
        app.dispatch(AppCommand::ProcessQueued { request: queued })
            .expect("ProcessQueued must compose the native route before Prepared");
        let deadline = Instant::now() + Duration::from_secs(3_600);
        let mut observed_phase = "not visible".to_owned();
        let mut observed_message = None;
        let mut observed_backend_message = None;
        let mut fallback_message_before_audio = false;
        let mut complete_view = None;
        let mut observed_native_events = Vec::with_capacity(128);
        while Instant::now() < deadline {
            let view = app
                .dispatch(AppCommand::Refresh)
                .expect("refresh queued native import");
            native_event_drain(&native_events, &mut observed_native_events);
            observed_message = view.message.clone();
            if let Some(session) = view
                .unified_parakeet_sessions
                .iter()
                .find(|session| (session.job_id, session.generation) == identity)
            {
                observed_phase = format!("{:?}", session.phase);
                if expected_fallback.is_some()
                    && view.message.as_deref() == Some(expected_backend_message.as_str())
                {
                    observed_backend_message = view.message.clone();
                    fallback_message_before_audio |= !observed_native_events.iter().any(|event| {
                        matches!(
                            &event.kind,
                            NativeQualificationEventKind::RequestSent {
                                session: sent_session,
                                generation: sent_generation,
                                ..
                            } if sent_session.job_id == job_id
                                && *sent_generation == generation
                        )
                    });
                }
                if session.phase == whisper_core::ports::UnifiedSessionPhase::Recoverable {
                    panic!(
                        "native queued import became recoverable: phase={observed_phase}, message={observed_message:?}"
                    );
                }
                if session.phase == whisper_core::ports::UnifiedSessionPhase::Complete {
                    complete_view = Some(session.clone());
                    break;
                }
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert!(
            !native_observer.overflowed.load(Ordering::Relaxed),
            "native event observer overflowed its fixed capacity"
        );
        let complete = complete_view.unwrap_or_else(|| {
                panic!(
                    "native queued import did not complete within its bounded wait: phase={observed_phase}, message={observed_message:?}"
                )
            });
        assert_eq!(complete.total_samples, Some(expected_samples));
        assert_eq!(complete.captured_samples, expected_samples);
        assert_eq!(complete.durable_samples, expected_samples);
        assert_app_view_internal_publication(&complete, &archive_root);
        assert_eq!(
            crate::parakeet_import::sha256_file(&source_path).unwrap(),
            source_sha.as_str()
        );

        let ready = observed_native_events
            .iter()
            .filter_map(|event| match &event.kind {
                NativeQualificationEventKind::Ready {
                    backend,
                    profile,
                    model_sha256,
                    runtime_sha256,
                    build_sha256,
                    gpu_attested,
                    ..
                } => Some((
                    backend.as_str(),
                    profile.as_str(),
                    model_sha256.as_str(),
                    runtime_sha256.as_str(),
                    build_sha256.as_str(),
                    *gpu_attested,
                )),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            ready.len(),
            1,
            "one native READY must qualify the queued session"
        );
        assert_eq!(ready[0].0, expected_ready_backend);
        assert_eq!(
            ready[0].1,
            whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS
        );
        assert_eq!(
            ready[0].2,
            whisper_core::model_registry::model(whisper_core::model_catalog::ModelId::ParakeetTdt06BV3)
                .artifact
                .sha256
        );
        if let Some(key) = expected_gpu_key.as_ref() {
            assert_eq!(ready[0].3, key.runtime_build);
            assert_eq!(ready[0].4, key.worker_build);
        }
        assert_eq!(ready[0].5, expected_effective_compute == ComputeChoice::Gpu);
        if expected_fallback.is_some() {
            assert_eq!(
                observed_backend_message.as_deref(),
                Some(expected_backend_message.as_str()),
                "AppView must display the effective CPU backend and Auto fallback cause"
            );
            assert!(
                fallback_message_before_audio,
                "AppView must display the Auto fallback before the first RequestSent; backend_message={observed_backend_message:?}, request_events={:?}",
                observed_native_events
                    .iter()
                    .filter_map(|event| match &event.kind {
                        NativeQualificationEventKind::RequestSent {
                            session: sent_session,
                            generation: sent_generation,
                            request_id,
                            ..
                        } if sent_session.job_id == job_id && *sent_generation == generation => {
                            Some(*request_id)
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            );
        }
        let route_evidence = observed_native_events
            .iter()
            .filter_map(|event| match &event.kind {
                NativeQualificationEventKind::PreparedRoute {
                    session,
                    requested_compute,
                    compute: effective_compute,
                    has_candidate,
                    has_expected_gpu,
                    fallback_reason,
                } if session.job_id == job_id && session.generation == generation => Some((
                    *requested_compute,
                    *effective_compute,
                    *has_candidate,
                    *has_expected_gpu,
                    fallback_reason.as_deref(),
                )),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            route_evidence.len(),
            1,
            "the app must compose one native route"
        );
        assert_eq!(route_evidence[0].0, compute);
        assert_eq!(route_evidence[0].1, expected_effective_compute);
        assert_eq!(
            route_evidence[0].2,
            expected_effective_compute == ComputeChoice::Gpu
        );
        assert_eq!(
            route_evidence[0].3,
            expected_effective_compute == ComputeChoice::Gpu
        );
        assert_eq!(route_evidence[0].4, expected_fallback);
        assert!(
            observed_native_events.iter().any(|event| matches!(
                &event.kind,
                NativeQualificationEventKind::RequestSent { session, generation: sent_generation, .. }
                    if session.job_id == job_id && *sent_generation == generation
            )),
            "the composed route must issue a correlated native transcription request"
        );

        let entry = ArchiveStore::scan(&archive_root)
            .expect("scan queued native archive")
            .into_iter()
            .find(|entry| (entry.job_id, entry.generation) == identity && entry.complete)
            .expect("queued native app path must publish a complete archive");
        assert_eq!(entry.strategy, request.strategy);
        assert_eq!(entry.source_sha256, source_sha);
        let _ = assert_native_generation_text_artifacts(&archive_root, &entry);
    }

    #[cfg(windows)]
    fn native_spoken_prefix(execution_root: &Path) -> Vec<f32> {
        let fixture = std::env::var_os("WHISPER_PU_L4_LONG_EN")
            .map(PathBuf::from)
            .expect("WHISPER_PU_L4_LONG_EN must name the guarded annotated EN fixture");
        let source = fixture
            .canonicalize()
            .expect("guarded long EN WAV fixture must exist");
        let expected_source = execution_root.join("fixtures/en-long-40m.wav");
        assert_eq!(
            source,
            expected_source
                .canonicalize()
                .expect("planned long EN WAV must be staged in the guarded lot")
        );
        let annotation_path = execution_root
            .join("fixtures/en-long-40m.annotations.json")
            .canonicalize()
            .expect("paired long EN annotation must exist before native startup");
        assert!(annotation_path.starts_with(execution_root));
        let annotation: NativeLongAnnotation = serde_json::from_slice(
            &fs::read(&annotation_path).expect("read the paired long EN annotation"),
        )
        .expect("parse the long corpus annotation");
        assert_eq!(annotation.schema, "pu-l4-corpus-v1");
        assert_eq!(annotation.language, "en");
        assert_eq!(annotation.sample_rate_hz, 16_000);
        assert_eq!(annotation.channels, 1);

        let metadata = fs::metadata(&source).expect("stat the long EN WAV");
        let mut file = fs::File::open(&source).expect("open the long EN WAV read-only");
        let mut header = [0_u8; 44];
        file.read_exact(&mut header).expect("read PCM WAV header");
        assert_eq!(&header[0..4], b"RIFF");
        assert_eq!(&header[8..12], b"WAVE");
        assert_eq!(&header[36..40], b"data");
        assert_eq!(u16::from_le_bytes(header[20..22].try_into().unwrap()), 1);
        assert_eq!(u16::from_le_bytes(header[22..24].try_into().unwrap()), 1);
        assert_eq!(
            u32::from_le_bytes(header[24..28].try_into().unwrap()),
            16_000
        );
        assert_eq!(u16::from_le_bytes(header[34..36].try_into().unwrap()), 16);
        let data_bytes = u32::from_le_bytes(header[40..44].try_into().unwrap());
        assert_eq!(metadata.len(), 44 + u64::from(data_bytes));
        assert_eq!(u64::from(data_bytes / 2), annotation.source_samples);
        assert!(
            crate::parakeet_import::sha256_file(&source)
                .expect("hash long EN WAV")
                .eq_ignore_ascii_case(&annotation.source_sha256),
            "long EN WAV hash must match its annotation"
        );

        let sample_count = 2 * whisper_core::parakeet_windows::WINDOW_MAX_SAMPLES
            + whisper_core::parakeet_windows::MARGIN_SAMPLES
            + 16_000;
        assert!(annotation.source_samples >= sample_count);
        assert_native_long_annotation_coverage(&annotation, sample_count);

        let byte_count =
            usize::try_from(sample_count * 2).expect("bounded spoken prefix fits address space");
        std::io::Seek::seek(&mut file, std::io::SeekFrom::Start(44))
            .expect("seek to the bounded PCM prefix");
        let mut bytes = vec![0_u8; byte_count];
        file.read_exact(&mut bytes)
            .expect("read only the bounded spoken prefix");
        bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| f32::from(i16::from_le_bytes([pair[0], pair[1]])) / 32_768.0)
            .collect()
    }

    #[cfg(windows)]
    fn assert_native_long_annotation_coverage(annotation: &NativeLongAnnotation, sample_count: u64) {
        let repository_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .canonicalize()
            .expect("resolve repository root for annotation provenance");
        let mut covered_samples = 0_u64;
        for segment in &annotation.segments {
            assert_eq!(segment.start_sample, covered_samples);
            assert!(segment.end_sample > segment.start_sample);
            assert!(segment.end_sample <= annotation.source_samples);
            assert!(!segment.reference_text.trim().is_empty());
            assert!(segment.source_path.starts_with("sound example/"));
            if segment.start_sample < sample_count {
                assert!(repository_root.join(&segment.source_path).is_file());
            }
            covered_samples = segment.end_sample;
        }
        assert_eq!(covered_samples, annotation.source_samples);
        assert!(covered_samples >= sample_count);
    }

    fn source_binding_map_events(
        receiver: &Receiver<NativeQualificationEvent>,
    ) -> Vec<(&'static str, Vec<(JobId, u64)>)> {
        let mut bindings = Vec::new();
        while let Ok(event) = receiver.try_recv() {
            if let NativeQualificationEventKind::SourceBindingMap {
                operation,
                bindings: snapshot,
            } = event.kind
            {
                bindings.push((operation, snapshot));
            }
        }
        bindings
    }

    #[cfg(windows)]
    fn native_event_drain(
        receiver: &Receiver<NativeQualificationEvent>,
        output: &mut Vec<NativeQualificationEvent>,
    ) {
        while output.len() < 512 {
            match receiver.try_recv() {
                Ok(event) => output.push(event),
                Err(mpsc::TryRecvError::Empty | mpsc::TryRecvError::Disconnected) => break,
            }
        }
        assert!(
            output.len() < 512,
            "native event ledger exceeded its fixed test bound"
        );
    }

    #[derive(Clone, Copy)]
    #[cfg(windows)]
    struct NativeAbcIds {
        a: JobId,
        b: JobId,
        c: JobId,
        generation: Generation,
    }

    #[derive(Default)]
    #[cfg(windows)]
    struct NativeAbcTimes {
        a_audio_drained_at: Option<Instant>,
        a_finalizing_at: Option<Instant>,
        a_publishing_at: Option<Instant>,
        a_published_at: Option<Instant>,
        b_audio_drained_at: Option<Instant>,
        b_finalizing_at: Option<Instant>,
        b_publishing_at: Option<Instant>,
        b_published_at: Option<Instant>,
    }

    #[cfg(windows)]
    struct NativeAbcDocumentState {
        deadline: Instant,
        requested_revision: Option<u64>,
        stale_wait_revision: Option<u64>,
        last_stale_document_error: Option<String>,
        document_request_id: u64,
    }

    #[cfg(windows)]
    #[path = "native_execution_helpers.rs"]
    mod native_execution_helpers;

    #[cfg(windows)]
    #[path = "native_execution_resume_helpers.rs"]
    mod native_execution_resume_helpers;

    #[cfg(windows)]
    fn run_native_personal_ab_c(
        compute: ComputeChoice,
        gpu_candidate: Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
    ) {
        let mut context = native_execution_helpers::prepare_native_ab_c_context(compute, gpu_candidate);
        native_ab_c_phase_a(&mut context);
        native_ab_c_phase_b(&mut context);
        native_ab_c_publish_ab(&mut context);
        native_ab_c_phase_c(&mut context);
    }

    #[cfg(windows)]
    fn native_ab_c_phase_a(context: &mut native_execution_helpers::NativeAbcContext) {
        let a = context.ids.a;
        let generation = context.ids.generation;
        let trace_path = context.trace.trace_path.clone();
        context.checkpoint("before-Start-A", None);
        context
            .app
            .dispatch(AppCommand::StartUnifiedLiveConfigured {
                request: context.requests[0].clone(),
            })
            .expect("admit spoken capture A through ImportApplication");
        context.checkpoint("after-Start-A", None);
        let first_send_deadline = Instant::now() + Duration::from_secs(60 * 60);
        let mut first_send_trace_at = Instant::now();
        let (first_a_request_id, first_a_send) = loop {
            let trace_refresh = first_send_trace_at.elapsed() >= Duration::from_secs(5);
            if trace_refresh {
                context.checkpoint("wait-A1-before-Refresh", None);
            }
            context
                .app
                .dispatch(AppCommand::Refresh)
                .expect("poll capture A");
            native_event_drain(&context.native_events, &mut context.observations);
            context.observer.assert_healthy();
            if trace_refresh {
                context.checkpoint("wait-A1-after-Refresh", None);
                first_send_trace_at = Instant::now();
            }
            if let Some((request_id, sent_at)) =
                context
                    .observations
                    .iter()
                    .find_map(|event| match &event.kind {
                        NativeQualificationEventKind::RequestSent {
                            session,
                            work: NativeQualificationWork::Final(1),
                            request_id,
                            ..
                        } if session.job_id == a => Some((*request_id, event.at)),
                        _ => None,
                    })
            {
                context.checkpoint("A1-RequestSent", Some(request_id));
                break (request_id, sent_at);
            }
            assert!(
                Instant::now() < first_send_deadline,
                "A1 native final call was not sent; trace={}",
                trace_path.display()
            );
            thread::sleep(Duration::from_millis(5));
        };

        let a_stop = Instant::now();
        context.checkpoint("before-Stop-A", Some(first_a_request_id));
        context
            .app
            .dispatch(AppCommand::StopUnifiedLive {
                job_id: a,
                generation,
            })
            .expect("stop capture A through ImportApplication while A1 remains in flight");
        context.checkpoint("after-Stop-A", Some(first_a_request_id));
        assert_eq!(
            context.stop_submissions.load(Ordering::Relaxed),
            1,
            "the initial A stop must reach the runtime exactly once"
        );
        context.checkpoint(
            "before-duplicate-Stop-A-during-drain",
            Some(first_a_request_id),
        );
        context
            .app
            .dispatch(AppCommand::StopUnifiedLive {
                job_id: a,
                generation,
            })
            .expect("duplicate Stop A remains idempotent while its capture admission is retained");
        context.checkpoint(
            "after-duplicate-Stop-A-during-drain",
            Some(first_a_request_id),
        );
        assert_eq!(
            context.stop_submissions.load(Ordering::Relaxed),
            1,
            "duplicate Stop A during drain must not resubmit the runtime stop"
        );

        context.first_a_request_id = Some(first_a_request_id);
        context.a_drain_completed = Some(native_ab_c_drain_a(context));
        context.first_a_send = Some(first_a_send);
        context.a_stop = Some(a_stop);
    }

    #[cfg(windows)]
    fn native_ab_c_verify_source(
        context: &mut native_execution_helpers::NativeAbcContext,
        a_view: &whisper_core::ports::UnifiedParakeetSessionView,
    ) -> bool {
        let a = context.ids.a;
        let generation = context.ids.generation;
        let first_a_request_id = context
            .first_a_request_id
            .expect("A1 request identity is available after send");
        let expected_a_samples = context.audio.expected_a_samples;
        let expected_a_tail = &context.audio.expected_a_tail;
        let trace_path = context.trace.trace_path.clone();

        let source = crate::parakeet_audio::PcmPartStore::open(&a_view.source_path);
        let indexed_samples = source.total_samples().unwrap_or_else(|error| {
                        panic!(
                            "Stop A exposed final source bounds but its PCM index is unreadable: {error}; source={}, trace={}",
                            a_view.source_path,
                            trace_path.display()
                        )
                    });
        assert_eq!(
            indexed_samples, expected_a_samples,
            "Stop A's durable PCM index must cover the complete source"
        );
        let tail_start = expected_a_samples
            .checked_sub(160)
            .expect("native spoken source contains at least the converter tail");
        let tail = source
            .read_window(tail_start..expected_a_samples)
            .unwrap_or_else(|error| {
                panic!(
                    "Stop A's final PCM tail is not readable: {error}; source={}, trace={}",
                    a_view.source_path,
                    trace_path.display()
                )
            });
        assert_eq!(tail.len(), 160, "Stop A must retain its final PCM tail");
        assert!(
            tail.iter()
                .zip(expected_a_tail)
                .all(|(actual, expected)| actual.abs_diff(*expected) <= 2),
            "Stop A's final written PCM samples differ from the sequential converter reference"
        );
        let manifest_path = context
            .archive_root
            .join("transcriptions")
            .join(format!("{:032x}", a.0))
            .join(generation.get().to_string())
            .join("captured-source-parts.json");
        let manifest_bytes = fs::read(&manifest_path).unwrap_or_else(|error| {
            panic!(
                "Stop A did not persist its captured-parts manifest: {error}; path={}, trace={}",
                manifest_path.display(),
                trace_path.display()
            )
        });
        let manifest: serde_json::Value =
            serde_json::from_slice(&manifest_bytes).unwrap_or_else(|error| {
                panic!(
                    "Stop A captured-parts manifest is invalid: {error}; path={}, trace={}",
                    manifest_path.display(),
                    trace_path.display()
                )
            });
        assert_eq!(
            manifest["source_samples"].as_u64(),
            Some(expected_a_samples),
            "Stop A manifest must bind the complete source length"
        );
        assert!(
            manifest["source_sha256"]
                .as_str()
                .is_some_and(|sha| sha.len() == 64),
            "Stop A manifest must retain the logical source hash"
        );
        let parts = manifest["parts"]
            .as_array()
            .expect("Stop A manifest contains indexed PCM parts");
        let mut covered_samples = 0_u64;
        for part in parts {
            assert_eq!(part["start_sample"].as_u64(), Some(covered_samples));
            covered_samples = part["end_sample"]
                .as_u64()
                .expect("Stop A part has a final sample bound");
            assert!(
                Path::new(
                    part["path"]
                        .as_str()
                        .expect("Stop A part path is persisted")
                )
                .is_file()
            );
        }
        assert_eq!(covered_samples, expected_a_samples);
        context.checkpoint("A-final-Stop-source-verified", Some(first_a_request_id));
        true
    }

    #[cfg(windows)]
    fn native_ab_c_drain_a(context: &mut native_execution_helpers::NativeAbcContext) -> Instant {
        let a = context.ids.a;
        let generation = context.ids.generation;
        let first_a_request_id = context
            .first_a_request_id
            .expect("A1 request identity was captured before draining A");
        let expected_a_samples = context.audio.expected_a_samples;
        let trace_path = context.trace.trace_path.clone();
        let mut a_drain_trace_at = Instant::now();
        let a_drain_deadline = Instant::now() + Duration::from_secs(60 * 60);
        let mut a_source_flush_verified = false;
        loop {
            let trace_refresh = a_drain_trace_at.elapsed() >= Duration::from_secs(5);
            if trace_refresh {
                context.checkpoint("A-drain-before-Refresh", Some(first_a_request_id));
            }
            context
                .app
                .dispatch(AppCommand::Refresh)
                .expect("close and drain capture A while A1 continues");
            native_event_drain(&context.native_events, &mut context.observations);
            context.record_a_phase_observation();
            context.observer.assert_healthy();
            if trace_refresh {
                context.checkpoint("A-drain-after-Refresh", Some(first_a_request_id));
                a_drain_trace_at = Instant::now();
            }
            let view = context.app.view();
            let Some(a_view) = view
                .unified_parakeet_sessions
                .iter()
                .find(|session| session.job_id == a && session.generation == generation)
                .cloned()
            else {
                panic!(
                    "stopped A disappeared before its drain completed; active_job={:?}, message={:?}, trace={}",
                    view.active_job,
                    view.message,
                    trace_path.display()
                );
            };
            if a_view.phase == whisper_core::ports::UnifiedSessionPhase::Recoverable
                || view
                    .active_job
                    .as_ref()
                    .is_some_and(|(job_id, state)| *job_id == a && *state == JobState::Recoverable)
            {
                panic!(
                    "A became Recoverable before final capture drain; session={a_view:?}, active_job={:?}, message={:?}, trace={}",
                    view.active_job,
                    view.message,
                    trace_path.display()
                );
            }
            let acknowledged = context
                .observations
                .iter()
                .rev()
                .find_map(|event| match &event.kind {
                    NativeQualificationEventKind::CaptureAcknowledged {
                        session,
                        written_samples,
                        durable_samples,
                    } if session.job_id == a => Some((*written_samples, *durable_samples)),
                    _ => None,
                });
            if a_view.total_samples == Some(expected_a_samples) && !a_source_flush_verified {
                a_source_flush_verified = native_ab_c_verify_source(context, &a_view);
            }
            let has_prior_capture_ack = acknowledged.is_some();
            let observed_ack_is_bounded = acknowledged.is_none_or(|(written, durable)| {
                written <= expected_a_samples && durable <= expected_a_samples
            });
            if a_view.total_samples == Some(expected_a_samples)
                && a_view.captured_samples == expected_a_samples
                && a_view.durable_samples == expected_a_samples
                && a_view.phase == whisper_core::ports::UnifiedSessionPhase::Finalizing
                && a_source_flush_verified
                && has_prior_capture_ack
                && observed_ack_is_bounded
            {
                context.checkpoint("A-drain-complete", Some(first_a_request_id));
                return Instant::now();
            }
            assert!(
                Instant::now() < a_drain_deadline,
                "Stop A did not close and durably drain all {expected_a_samples} captured samples; trace={} ",
                trace_path.display()
            );
            thread::sleep(Duration::from_millis(5));
        }
    }

    #[cfg(windows)]
    fn native_ab_c_wait_b_ack(
        context: &mut native_execution_helpers::NativeAbcContext,
    ) -> (Instant, (Instant, u64, u64)) {
        let a = context.ids.a;
        let b = context.ids.b;
        let generation = context.ids.generation;
        let trace_path = context.trace.trace_path.clone();
        let first_a_request_id = context
            .first_a_request_id
            .expect("A1 request identity was captured before B admission");
        let first_a_result_deadline = Instant::now() + Duration::from_secs(60 * 60);
        let mut b_ack_trace_at = Instant::now();
        let (first_a_result, b_audio_ack) = loop {
            let trace_refresh = b_ack_trace_at.elapsed() >= Duration::from_secs(5);
            if trace_refresh {
                context.checkpoint("A1-B-ACK-before-Refresh", Some(first_a_request_id));
            }
            context
                .app
                .dispatch(AppCommand::Refresh)
                .expect("poll A/B native runtime");
            native_event_drain(&context.native_events, &mut context.observations);
            context.record_a_phase_observation();
            context.observer.assert_healthy();
            if trace_refresh {
                context.checkpoint("A1-B-ACK-after-Refresh", Some(first_a_request_id));
                b_ack_trace_at = Instant::now();
            }
            let b_view = context.require_b_session("waiting for B audio ACK");
            let result = context
                .observations
                .iter()
                .find_map(|event| match &event.kind {
                    NativeQualificationEventKind::Result {
                        session,
                        request_id,
                        work: NativeQualificationWork::Final(1),
                        ..
                    } if session.job_id == a && *request_id == first_a_request_id => Some(event.at),
                    _ => None,
                });
            let ack = context
                .observations
                .iter()
                .find_map(|event| match &event.kind {
                    NativeQualificationEventKind::CaptureAcknowledged {
                        session,
                        written_samples,
                        durable_samples,
                    } if session.job_id == b && *written_samples > 0 && *durable_samples > 0 => {
                        Some((event.at, *written_samples, *durable_samples))
                    }
                    _ => None,
                });
            if let Some(result_at) = result {
                if let Some(ack) = ack.filter(|ack| ack.0 < result_at) {
                    context.checkpoint("B-ACK-before-A1-Result", Some(first_a_request_id));
                    break (result_at, ack);
                }
                let view = context.app.view();
                let a_view = view
                    .unified_parakeet_sessions
                    .iter()
                    .find(|session| session.job_id == a && session.generation == generation)
                    .cloned();
                let recent_events = context
                    .observations
                    .iter()
                    .rev()
                    .take(16)
                    .collect::<Vec<_>>();
                panic!(
                    "A1 returned before B had a prior written-and-durable PCM ACK; result_at={result_at:?}, A={a_view:?}, B={b_view:?}, active_job={:?}, app_message={:?}, B_ack={ack:?}, recent_native_events={recent_events:?}, trace={}",
                    view.active_job,
                    view.message,
                    trace_path.display()
                );
            }
            assert!(
                Instant::now() < first_a_result_deadline,
                "A1 correlated result or B written PCM ACK was not observed; B={b_view:?}, trace={}",
                trace_path.display()
            );
            thread::sleep(Duration::from_millis(5));
        };
        (first_a_result, b_audio_ack)
    }

    #[cfg(windows)]
    fn native_ab_c_wait_document_revision(
        context: &mut native_execution_helpers::NativeAbcContext,
        first_a_request_id: u64,
        state: &NativeAbcDocumentState,
    ) {
        let a = context.ids.a;
        let generation = context.ids.generation;

        *context
            .trace
            .document_page_trace
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = format!(
            "selected={a:?}/{generation:?},requested_revision=None,current_revision=0,page_revision=None,page_entries=0,document_error=None,request_id=None,result=waiting-for-revision"
        );
        let trace_refresh =
            context.trace_wait_before_refresh("A-document-revision", Some(first_a_request_id));
        context
            .app
            .dispatch(AppCommand::Refresh)
            .expect("wait for A final document revision");
        native_event_drain(&context.native_events, &mut context.observations);
        context.record_a_phase_observation();
        context.observer.assert_healthy();
        if trace_refresh {
            context.checkpoint(
                "A-document-revision-after-Refresh",
                Some(first_a_request_id),
            );
        }
        context.require_b_session("waiting for A document revision");
        assert!(
            Instant::now() < state.deadline,
            "A final document revision was not exposed to the application"
        );
        thread::sleep(Duration::from_millis(5));
    }

    #[cfg(windows)]
    fn native_ab_c_wait_after_stale_page(
        context: &mut native_execution_helpers::NativeAbcContext,
        first_a_request_id: u64,
        current_revision: u64,
        state: &mut NativeAbcDocumentState,
    ) {
        let a = context.ids.a;
        let generation = context.ids.generation;
        let trace_path = context.trace.trace_path.clone();
        let document_request_id = state.document_request_id;
        let stale_wait_revision = state.stale_wait_revision;
        let last_stale_document_error = state.last_stale_document_error.clone();

        *context
            .trace
            .document_page_trace
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = format!(
            "selected={a:?}/{generation:?},requested_revision={stale_wait_revision:?},current_revision={current_revision},page_revision=None,document_error={:?},request_id={document_request_id},result=waiting-for-revision-update",
            state.last_stale_document_error
        );
        let trace_refresh =
            context.trace_wait_before_refresh("A-document-stale-wait", Some(first_a_request_id));
        context
            .app
            .dispatch(AppCommand::Refresh)
            .expect("refresh after stale A document page");
        native_event_drain(&context.native_events, &mut context.observations);
        context.record_a_phase_observation();
        context.observer.assert_healthy();
        if trace_refresh {
            context.checkpoint(
                "A-document-stale-wait-after-Refresh",
                Some(first_a_request_id),
            );
        }
        context.require_b_session("waiting for A document revision after stale page");
        let refreshed_view = context.app.view();
        let refreshed_session = refreshed_view
            .unified_parakeet_sessions
            .iter()
            .find(|session| session.job_id == a && session.generation == generation);
        let refreshed_revision = refreshed_session.map_or(0, |session| session.document_revision);
        let refreshed_phase = refreshed_session.map(|session| session.phase);
        if refreshed_revision == current_revision
            && refreshed_phase.is_some_and(|phase| {
                matches!(
                    phase,
                    whisper_core::ports::UnifiedSessionPhase::Complete
                        | whisper_core::ports::UnifiedSessionPhase::Recoverable
                )
            })
        {
            let phase = refreshed_phase.expect("terminal phase was checked");
            let current_document_error = refreshed_view
                .selected_unified_session
                .as_ref()
                .filter(|selection| selection.job_id == a && selection.generation == generation)
                .and_then(|selection| selection.document_error.as_deref());
            *context
                .trace
                .document_page_trace
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = format!(
                "selected={a:?}/{generation:?},requested_revision={current_revision},current_revision={refreshed_revision},phase={phase:?},document_error={:?},request_id={document_request_id},result=stale-at-terminal",
                state.last_stale_document_error
            );
            context.checkpoint("A-document-stale-at-terminal", Some(first_a_request_id));
            panic!(
                "A document page request {document_request_id} returned stale and the AppView revision {refreshed_revision} did not advance before A became terminal; requested_revision={current_revision}, phase={phase:?}, stale_document_error={last_stale_document_error:?}, current_document_error={current_document_error:?}, trace={}",
                trace_path.display()
            );
        }
        if refreshed_revision != current_revision {
            state.stale_wait_revision = None;
            state.requested_revision = None;
            state.last_stale_document_error = None;
        }
        assert!(
            Instant::now() < state.deadline,
            "A document response was stale and the application revision did not advance"
        );
        thread::sleep(Duration::from_millis(5));
    }

    #[cfg(windows)]
    fn native_ab_c_refresh_document_view(
        context: &mut native_execution_helpers::NativeAbcContext,
        first_a_request_id: u64,
    ) -> whisper_core::AppView {
        let trace_refresh =
            context.trace_wait_before_refresh("A-document-page", Some(first_a_request_id));
        context
            .app
            .dispatch(AppCommand::Refresh)
            .expect("read A document page");
        native_event_drain(&context.native_events, &mut context.observations);
        context.record_a_phase_observation();
        context.observer.assert_healthy();
        if trace_refresh {
            context.checkpoint("A-document-page-after-Refresh", Some(first_a_request_id));
        }
        context.require_b_session("waiting for A document page");
        context.app.view()
    }

    #[cfg(windows)]
    fn native_ab_c_handle_document_error(
        context: &mut native_execution_helpers::NativeAbcContext,
        state: &mut NativeAbcDocumentState,
        first_a_request_id: u64,
        current_revision: u64,
        refreshed_revision: u64,
        page_summary: &str,
        view: &whisper_core::AppView,
    ) -> bool {
        let message = view
            .selected_unified_session
            .as_ref()
            .and_then(|selection| selection.document_error.as_deref())
            .expect("document error was observed before handling");
        let a = context.ids.a;
        let generation = context.ids.generation;
        let trace_path = context.trace.trace_path.clone();

        if message.starts_with("StaleResponse:") {
            let stale_revision = state.requested_revision.unwrap_or(current_revision);
            state.last_stale_document_error = Some(message.to_owned());
            state.stale_wait_revision =
                (refreshed_revision == stale_revision).then_some(stale_revision);
            context
                .app
                .dispatch(AppCommand::SelectUnifiedSession {
                    job_id: a,
                    generation,
                })
                .expect("clear the consumed stale page response before retrying");
            state.requested_revision = None;
            *context
                .trace
                .document_page_trace
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = format!(
                "selected={a:?}/{generation:?},requested_revision={current_revision},current_revision={refreshed_revision},page={page_summary},document_error={:?},request_id={},result=stale",
                message.chars().take(240).collect::<String>(),
                state.document_request_id
            );
            context.checkpoint("A-document-page-stale", Some(first_a_request_id));
            return true;
        }
        panic!(
            "A document page request {} failed non-stale for revision {current_revision}: {message}; selected={:?}, current_revision={refreshed_revision}, trace={}",
            state.document_request_id,
            view.selected_unified_session,
            trace_path.display()
        );
    }

    #[cfg(windows)]
    fn native_ab_c_process_document_view(
        context: &mut native_execution_helpers::NativeAbcContext,
        state: &mut NativeAbcDocumentState,
        first_a_request_id: u64,
        current_revision: u64,
        view: whisper_core::AppView,
    ) -> Option<Instant> {
        let a = context.ids.a;
        let generation = context.ids.generation;
        let trace_path = context.trace.trace_path.clone();
        let snapshot =
            native_execution_resume_helpers::native_ab_c_document_view_snapshot(context, state, &view);
        let refreshed_revision = snapshot.refreshed_revision;
        let page = snapshot.page;
        let document_error = snapshot.document_error;
        let page_summary = snapshot.page_summary;

        if document_error.is_some()
            && native_ab_c_handle_document_error(
                context,
                state,
                first_a_request_id,
                current_revision,
                refreshed_revision,
                &page_summary,
                &view,
            )
        {
            return None;
        }
        if refreshed_revision != state.requested_revision.unwrap_or(0) {
            state.requested_revision = None;
            state.stale_wait_revision = None;
            context.checkpoint("A-document-revision-changed", Some(first_a_request_id));
            assert!(
                Instant::now() < state.deadline,
                "A final document revision changed repeatedly before a current page became readable"
            );
            return None;
        }
        if let Some(page) = page.filter(|page| page.revision == refreshed_revision) {
            let first_sequence = page.entries.first().map(|entry| entry.segment.segment_id.0);
            assert_eq!(
                first_sequence,
                Some(1),
                "first bounded A page must preserve sequence 1"
            );
            if page
                .entries
                .iter()
                .any(|entry| !entry.segment.text.trim().is_empty())
            {
                let first_text_available = Some(Instant::now());
                *context
                    .trace
                    .document_page_trace
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = format!(
                    "selected={a:?}/{generation:?},requested_revision={current_revision},current_revision={refreshed_revision},page_revision={},page_entries={},document_error=None,request_id={},first_sequence=1,limit=128,result=current-nonempty-text",
                    page.revision,
                    page.entries.len(),
                    state.document_request_id
                );
                context.checkpoint("A-document-page-current-nonempty", Some(first_a_request_id));
                return first_text_available;
            }
            panic!(
                "A current document page {} has no nonempty text at revision {refreshed_revision}; trace={}",
                state.document_request_id,
                trace_path.display()
            );
        }
        assert!(
            Instant::now() < state.deadline,
            "A final text was not readable from the selected application page: selected={:?}, requested_revision={:?}, current_revision={refreshed_revision}, page={page_summary}, document_error={document_error:?}, request_id={}, trace={}",
            view.selected_unified_session,
            state.requested_revision,
            state.document_request_id,
            trace_path.display()
        );
        thread::sleep(Duration::from_millis(5));
        None
    }

    #[cfg(windows)]
    fn native_ab_c_read_a_document(
        context: &mut native_execution_helpers::NativeAbcContext,
        first_a_request_id: u64,
    ) -> Option<Instant> {
        let a = context.ids.a;
        let generation = context.ids.generation;
        context
            .app
            .dispatch(AppCommand::SelectUnifiedSession {
                job_id: a,
                generation,
            })
            .expect("select A to read its real application document");
        let mut state = NativeAbcDocumentState {
            deadline: Instant::now() + Duration::from_secs(60 * 60),
            requested_revision: None,
            stale_wait_revision: None,
            last_stale_document_error: None,
            document_request_id: 0,
        };
        loop {
            let before_request = context.app.view();
            let current_revision = before_request
                .unified_parakeet_sessions
                .iter()
                .find(|session| session.job_id == a && session.generation == generation)
                .map(|session| session.document_revision)
                .unwrap_or(0);
            if current_revision == 0 {
                native_ab_c_wait_document_revision(context, first_a_request_id, &state);
                continue;
            }

            if state
                .requested_revision
                .is_some_and(|requested| requested != current_revision)
            {
                context
                    .app
                    .dispatch(AppCommand::SelectUnifiedSession {
                        job_id: a,
                        generation,
                    })
                    .expect("reselect A before reading its newer document revision");
                state.requested_revision = None;
                state.stale_wait_revision = None;
                state.last_stale_document_error = None;
            }

            if state.stale_wait_revision == Some(current_revision) {
                native_ab_c_wait_after_stale_page(
                    context,
                    first_a_request_id,
                    current_revision,
                    &mut state,
                );
                continue;
            }
            state.stale_wait_revision = None;

            if state.requested_revision != Some(current_revision) {
                state.document_request_id = state
                    .document_request_id
                    .checked_add(1)
                    .expect("bounded document request ID remains representable");
                context
                    .app
                    .dispatch(AppCommand::ReadUnifiedDocumentPage {
                        job_id: a,
                        generation,
                        expected_revision: current_revision,
                        first_sequence: 1,
                    })
                    .expect("request A's bounded page through ImportApplication");
                state.requested_revision = Some(current_revision);
                *context
                    .trace
                    .document_page_trace
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner) = format!(
                    "selected={a:?}/{generation:?},requested_revision={current_revision},current_revision={current_revision},page_revision=None,page_entries=0,document_error=None,request_id={},first_sequence=1,limit=128,result=pending",
                    state.document_request_id
                );
                context.checkpoint("A-document-page-request", Some(first_a_request_id));
            }

            let view = native_ab_c_refresh_document_view(context, first_a_request_id);
            if let Some(first_text) = native_ab_c_process_document_view(
                context,
                &mut state,
                first_a_request_id,
                current_revision,
                view,
            ) {
                return Some(first_text);
            }
        }

    }

    #[cfg(windows)]
    fn native_ab_c_wait_second_final(
        context: &mut native_execution_helpers::NativeAbcContext,
        first_a_request_id: u64,
    ) {
        let a = context.ids.a;
        let a_second_send_deadline = Instant::now() + Duration::from_secs(60 * 60);
        loop {
            let trace_refresh =
                context.trace_wait_before_refresh("A-second-window-send", Some(first_a_request_id));
            context
                .app
                .dispatch(AppCommand::Refresh)
                .expect("progress A/B windows");
            native_event_drain(&context.native_events, &mut context.observations);
            context.record_a_phase_observation();
            context.observer.assert_healthy();
            if trace_refresh {
                context.checkpoint(
                    "A-second-window-send-after-Refresh",
                    Some(first_a_request_id),
                );
            }
            context.require_b_session("waiting for A's second final request");
            if context.observations.iter().any(|event| {
                matches!(
                    &event.kind,
                    NativeQualificationEventKind::RequestSent {
                        session,
                        work: NativeQualificationWork::Final(2),
                        ..
                    } if session.job_id == a
                )
            }) {
                break;
            }
            assert!(
                Instant::now() < a_second_send_deadline,
                "A second native final window was not sent"
            );
            thread::sleep(Duration::from_millis(5));
        }
        let a_second_result_deadline = Instant::now() + Duration::from_secs(60 * 60);
        loop {
            let trace_refresh =
                context.trace_wait_before_refresh("A-second-window-result", Some(first_a_request_id));
            context
                .app
                .dispatch(AppCommand::Refresh)
                .expect("complete A second window");
            native_event_drain(&context.native_events, &mut context.observations);
            context.record_a_phase_observation();
            context.observer.assert_healthy();
            if trace_refresh {
                context.checkpoint(
                    "A-second-window-result-after-Refresh",
                    Some(first_a_request_id),
                );
            }
            context.require_b_session("waiting for A's second final result");
            let second_request = context
                .observations
                .iter()
                .find_map(|event| match &event.kind {
                    NativeQualificationEventKind::RequestSent {
                        session,
                        work: NativeQualificationWork::Final(2),
                        request_id,
                        ..
                    } if session.job_id == a => Some(*request_id),
                    _ => None,
                });
            if let Some(request_id) = second_request
                && context.observations.iter().any(|event| {
                    matches!(
                        &event.kind,
                        NativeQualificationEventKind::Result {
                            session,
                            request_id: result_id,
                            ..
                        } if session.job_id == a && *result_id == request_id
                    )
                })
            {
                break;
            }
            assert!(
                Instant::now() < a_second_result_deadline,
                "A second native final result was not correlated"
            );
            thread::sleep(Duration::from_millis(5));
        }
    }

    #[cfg(windows)]
    fn native_ab_c_phase_b(context: &mut native_execution_helpers::NativeAbcContext) {
        let b = context.ids.b;
        let generation = context.ids.generation;
        let trace_path = context.trace.trace_path.clone();
        let first_a_request_id = context
            .first_a_request_id
            .expect("A1 request identity was captured before B admission");
        let first_a_send = context
            .first_a_send
            .expect("A1 send time was captured before B admission");
        let a_stop = context
            .a_stop
            .expect("A stop time was captured before B admission");
        let a_drain_completed = context
            .a_drain_completed
            .expect("A drain completed before B admission");
        context.blocks.lock().unwrap().push_back((
            b,
            generation,
            context.audio.spoken[..5 * 16_000].to_vec(),
        ));
        let b_start = Instant::now();
        context.checkpoint("before-Start-B", Some(first_a_request_id));
        context
            .app
            .dispatch(AppCommand::StartUnifiedLiveConfigured {
                request: context.requests[1].clone(),
            })
            .expect("admit spoken capture B after stopped A drained, while A1 remains in flight");
        context.checkpoint("after-Start-B", Some(first_a_request_id));
        assert!(
            context
                .require_b_session("after Start B")
                .microphone_capture,
            "the controlled B capture stream must be available after A closes; trace={}",
            trace_path.display()
        );
        let (first_a_result, b_audio_ack) = native_ab_c_wait_b_ack(context);
        assert!(
            first_a_send < a_stop
                && a_stop < a_drain_completed
                && a_drain_completed < b_start
                && b_start < b_audio_ack.0
                && b_audio_ack.0 < first_a_result,
            "A1 must be sent, A stopped/drained, B admitted/written, then A1 returned in that order"
        );

        let first_text_available = native_ab_c_read_a_document(context, first_a_request_id);
        native_ab_c_wait_second_final(context, first_a_request_id);

        context.b_start = Some(b_start);
        context.b_audio_ack = Some(b_audio_ack);
        context.first_a_result = Some(first_a_result);
        context.first_text_available = first_text_available;
    }

    #[cfg(windows)]
    fn native_ab_c_wait_ab_publication(
        context: &mut native_execution_helpers::NativeAbcContext,
        a: JobId,
        b: JobId,
        generation: Generation,
        first_a_request_id: u64,
    ) {
        let publication_deadline = Instant::now() + Duration::from_secs(60 * 60);
        loop {
            let trace_refresh =
                context.trace_wait_before_refresh("A-B-publication", Some(first_a_request_id));
            context
                .app
                .dispatch(AppCommand::Refresh)
                .expect("drain A/B publication");
            native_event_drain(&context.native_events, &mut context.observations);
            context.observer.assert_healthy();
            if trace_refresh {
                context.checkpoint("A-B-publication-after-Refresh", Some(first_a_request_id));
            }
            let view = context.app.view();
            context.require_b_session("waiting for A/B publication");
            let sessions = &view.unified_parakeet_sessions;
            let now = Instant::now();
            for (job_id, drained_at, finalizing_at, publishing_at) in [
                (
                    a,
                    &mut context.times.a_audio_drained_at,
                    &mut context.times.a_finalizing_at,
                    &mut context.times.a_publishing_at,
                ),
                (
                    b,
                    &mut context.times.b_audio_drained_at,
                    &mut context.times.b_finalizing_at,
                    &mut context.times.b_publishing_at,
                ),
            ] {
                if let Some(session) = sessions
                    .iter()
                    .find(|session| session.job_id == job_id && session.generation == generation)
                {
                    if drained_at.is_none()
                        && session.total_samples.is_some_and(|total| {
                            session.captured_samples == total && session.durable_samples == total
                        })
                    {
                        *drained_at = Some(now);
                    }
                    if session.phase == whisper_core::ports::UnifiedSessionPhase::Finalizing
                        && finalizing_at.is_none()
                    {
                        *finalizing_at = Some(now);
                    }
                    if session.phase == whisper_core::ports::UnifiedSessionPhase::Publishing
                        && publishing_at.is_none()
                    {
                        *publishing_at = Some(now);
                    }
                }
            }
            let a_complete = sessions.iter().any(|session| {
                session.job_id == a
                    && session.generation == generation
                    && session.phase == whisper_core::ports::UnifiedSessionPhase::Complete
            });
            let b_complete = sessions.iter().any(|session| {
                session.job_id == b
                    && session.generation == generation
                    && session.phase == whisper_core::ports::UnifiedSessionPhase::Complete
            });
            if a_complete && context.times.a_published_at.is_none() {
                context.times.a_published_at = Some(Instant::now());
            }
            if b_complete && context.times.b_published_at.is_none() {
                context.times.b_published_at = Some(Instant::now());
            }
            if a_complete && b_complete {
                break;
            }
            assert!(
                Instant::now() < publication_deadline,
                "A/B native publication did not complete: {:?}",
                context.app.view().unified_parakeet_sessions
            );
            thread::sleep(Duration::from_millis(10));
        }
    }

    #[cfg(windows)]
    fn native_ab_c_verify_ab_artifacts(
        context: &native_execution_helpers::NativeAbcContext,
        a: JobId,
        b: JobId,
        generation: Generation,
    ) {
        let expected_a_samples = context.audio.expected_a_samples;
        let a_view = context
            .app
            .view()
            .unified_parakeet_sessions
            .iter()
            .find(|session| session.job_id == a && session.generation == generation)
            .cloned()
            .expect("A card remains after publication");
        let b_view = context
            .app
            .view()
            .unified_parakeet_sessions
            .iter()
            .find(|session| session.job_id == b && session.generation == generation)
            .cloned()
            .expect("B card remains after publication");
        for (job_id, session) in [(a, &a_view), (b, &b_view)] {
            assert_eq!(
                session.phase,
                whisper_core::ports::UnifiedSessionPhase::Complete
            );
            assert_eq!(session.total_samples, Some(session.captured_samples));
            assert_eq!(session.captured_samples, session.durable_samples);
            if job_id == a {
                assert_eq!(
                    session.total_samples,
                    Some(expected_a_samples),
                    "published A must contain exactly the complete spoken source prefix"
                );
            }
            let files =
                ArchiveStore::published_unified_files(&context.archive_root, job_id, generation)
                    .expect("native app session must publish its complete artifacts");
            for path in [&files.wav_path, &files.txt_path, &files.srt_path] {
                assert!(
                    Path::new(path).is_file(),
                    "published artifact missing: {path}"
                );
            }
            let generation_directory = context
                .archive_root
                .join("transcriptions")
                .join(format!("{:032x}", job_id.0))
                .join(generation.get().to_string());
            assert_eq!(
                fs::read_to_string(generation_directory.join("CURRENT"))
                    .expect("read committed capture CURRENT"),
                "manifest.json\n"
            );
            assert!(
                !fs::read_to_string(&files.txt_path)
                    .expect("read published TXT")
                    .trim()
                    .is_empty()
            );
            assert_eq!(
                crate::parakeet_audio::PcmPartStore::open(&session.source_path)
                    .total_samples()
                    .expect("validate published source part index"),
                session.captured_samples
            );
            let manifest: serde_json::Value = serde_json::from_slice(
                &fs::read(generation_directory.join("captured-source-parts.json"))
                    .expect("read committed capture source manifest"),
            )
            .expect("decode committed capture source manifest");
            assert_eq!(
                manifest["source_samples"].as_u64(),
                Some(session.captured_samples),
                "CURRENT manifest must bind the complete source for each capture"
            );
        }
        assert_ne!(
            a_view.txt_path, b_view.txt_path,
            "A/B export identities must remain distinct"
        );
        assert_ne!(
            a_view.srt_path, b_view.srt_path,
            "A/B SRT identities must remain distinct"
        );
    }

    #[cfg(windows)]
    fn native_ab_c_verify_warm_a_child(context: &native_execution_helpers::NativeAbcContext, a: JobId) {
        let first_a_windows = context
            .observations
            .iter()
            .filter_map(|event| match &event.kind {
                NativeQualificationEventKind::RequestSent {
                    incarnation,
                    pid,
                    session,
                    work: NativeQualificationWork::Final(sequence @ 1..=2),
                    request_id,
                    ..
                } if session.job_id == a => {
                    Some((*sequence, *incarnation, *pid, *request_id, event.at))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(
            first_a_windows.len() >= 2,
            "A must send at least two final native windows"
        );
        assert_eq!(first_a_windows[0].0, 1);
        assert_eq!(first_a_windows[1].0, 2);
        assert_eq!(first_a_windows[0].1, first_a_windows[1].1);
        assert_eq!(first_a_windows[0].2, first_a_windows[1].2);
        assert!(
            !context.observations.iter().any(|event| {
                event.at > first_a_windows[0].4
                    && event.at < first_a_windows[1].4
                    && matches!(
                        &event.kind,
                        NativeQualificationEventKind::Reaped { incarnation, .. }
                            | NativeQualificationEventKind::Ready { incarnation, .. }
                            if *incarnation == first_a_windows[0].1
                    )
            }),
            "A's first two final windows must share one warm child incarnation"
        );
        assert!(context.observations.iter().any(|event| {
            matches!(
                &event.kind,
                NativeQualificationEventKind::Result {
                    session,
                    request_id,
                    ..
                } if session.job_id == a && *request_id == first_a_windows[0].3
            )
        }));
        assert!(context.observations.iter().any(|event| {
            matches!(
                &event.kind,
                NativeQualificationEventKind::Result {
                    session,
                    request_id,
                    ..
                } if session.job_id == a && *request_id == first_a_windows[1].3
            )
        }));

        let mut in_flight = HashSet::new();
        let mut timeline = context.observations.iter().collect::<Vec<_>>();
        timeline.sort_by_key(|event| event.at);
        let mut maximum_in_flight = 0_usize;
        for event in &timeline {
            match &event.kind {
                NativeQualificationEventKind::RequestSent {
                    incarnation,
                    request_id,
                    ..
                } => {
                    assert!(in_flight.insert((*incarnation, *request_id)));
                    maximum_in_flight = maximum_in_flight.max(in_flight.len());
                }
                NativeQualificationEventKind::Result {
                    incarnation,
                    request_id,
                    ..
                } => {
                    assert!(in_flight.remove(&(*incarnation, *request_id)));
                }
                _ => {}
            }
        }
        assert!(
            in_flight.is_empty(),
            "every written native request must have a correlated result"
        );
        assert_eq!(
            maximum_in_flight, 1,
            "one fixed backend may execute at most one native call"
        );
    }

    #[cfg(windows)]
    fn native_ab_c_reaped_child(
        context: &native_execution_helpers::NativeAbcContext,
    ) -> (u64, u32, Instant) {
        context
            .observations
            .iter()
            .filter_map(|event| match &event.kind {
                NativeQualificationEventKind::Reaped { incarnation, pid } => {
                    Some((*incarnation, *pid, event.at))
                }
                _ => None,
            })
            .max_by_key(|entry| entry.2)
            .expect("A/B terminal publications must reap the prior child")
    }

    #[cfg(windows)]
    fn native_ab_c_publish_ab(context: &mut native_execution_helpers::NativeAbcContext) {
        let a = context.ids.a;
        let b = context.ids.b;
        let generation = context.ids.generation;
        let first_a_request_id = context
            .first_a_request_id
            .expect("A1 request identity was captured before publication");
        let b_stop = Instant::now();
        context.checkpoint("before-Stop-B", Some(first_a_request_id));
        context
            .app
            .dispatch(AppCommand::StopUnifiedLive {
                job_id: b,
                generation,
            })
            .expect("stop B through ImportApplication");
        context.checkpoint("after-Stop-B", Some(first_a_request_id));
        assert_eq!(context.stop_submissions.load(Ordering::Relaxed), 2);
        native_ab_c_wait_ab_publication(context, a, b, generation, first_a_request_id);
        native_ab_c_verify_ab_artifacts(context, a, b, generation);
        native_ab_c_verify_warm_a_child(context, a);
        let reaped_before_c = native_ab_c_reaped_child(context);

        context.b_stop = Some(b_stop);
        context.reaped_before_c = Some(reaped_before_c);
    }

    #[cfg(windows)]
    fn native_ab_c_wait_c_ready(
        context: &mut native_execution_helpers::NativeAbcContext,
        reaped_before_c: (u64, u32, Instant),
    ) -> (u64, u32) {
        let compute = context.compute;
        let c_ready_deadline = Instant::now() + Duration::from_secs(60 * 60);
        let c_ready = loop {
            let trace_refresh = context.trace_wait_before_refresh("C-ready", None);
            context
                .app
                .dispatch(AppCommand::Refresh)
                .expect("start replacement child for C");
            native_event_drain(&context.native_events, &mut context.observations);
            context.observer.assert_healthy();
            if trace_refresh {
                context.checkpoint("C-ready-after-Refresh", None);
            }
            if let Some(event) = context.observations.iter().find(|event| {
                matches!(
                    &event.kind,
                    NativeQualificationEventKind::Ready { incarnation, .. }
                        if *incarnation > reaped_before_c.0
                )
            }) {
                break event;
            }
            assert!(
                Instant::now() < c_ready_deadline,
                "C replacement child did not become READY"
            );
            thread::sleep(Duration::from_millis(10));
        };
        let (c_incarnation, c_pid) = match &c_ready.kind {
            NativeQualificationEventKind::Ready {
                incarnation,
                pid,
                backend,
                gpu_attested,
                ..
            } => {
                assert!(
                    c_ready.at > reaped_before_c.2,
                    "old child must be reaped before C READY"
                );
                assert_eq!(
                    backend,
                    if compute == ComputeChoice::Cpu {
                        "CPU"
                    } else {
                        "CUDA0"
                    }
                );
                assert_eq!(*gpu_attested, compute == ComputeChoice::Gpu);
                (*incarnation, *pid)
            }
            _ => unreachable!(),
        };
        assert_ne!(
            c_incarnation, reaped_before_c.0,
            "C must use a new child incarnation"
        );
        for event in &context.observations {
            if let NativeQualificationEventKind::Ready {
                backend,
                gpu_attested,
                ..
            } = &event.kind
            {
                assert_eq!(
                    backend,
                    if compute == ComputeChoice::Cpu {
                        "CPU"
                    } else {
                        "CUDA0"
                    }
                );
                assert_eq!(*gpu_attested, compute == ComputeChoice::Gpu);
            }
        }
        (c_incarnation, c_pid)
    }

    #[cfg(windows)]
    type NativeAbcCTiming = (
        Instant,
        Option<Instant>,
        Option<Instant>,
        Option<Instant>,
        Option<Instant>,
    );

    #[cfg(windows)]
    fn native_ab_c_finish_c(
        context: &mut native_execution_helpers::NativeAbcContext,
        c: JobId,
        generation: Generation,
    ) -> NativeAbcCTiming {
        let c_stop = Instant::now();
        context.checkpoint("before-Stop-C", None);
        context
            .app
            .dispatch(AppCommand::StopUnifiedLive {
                job_id: c,
                generation,
            })
            .expect("stop C through ImportApplication");
        context.checkpoint("after-Stop-C", None);
        let c_publication_deadline = Instant::now() + Duration::from_secs(60 * 60);
        let c_published_at;
        let mut c_audio_drained_at = None;
        let mut c_finalizing_at = None;
        let mut c_publishing_at = None;
        loop {
            let trace_refresh = context.trace_wait_before_refresh("C-publication", None);
            context
                .app
                .dispatch(AppCommand::Refresh)
                .expect("publish C through the same runtime");
            native_event_drain(&context.native_events, &mut context.observations);
            context.observer.assert_healthy();
            if trace_refresh {
                context.checkpoint("C-publication-after-Refresh", None);
            }
            let view = context.app.view();
            if let Some(session) = view
                .unified_parakeet_sessions
                .iter()
                .find(|session| session.job_id == c && session.generation == generation)
            {
                let now = Instant::now();
                if c_audio_drained_at.is_none()
                    && session.total_samples.is_some_and(|total| {
                        session.captured_samples == total && session.durable_samples == total
                    })
                {
                    c_audio_drained_at = Some(now);
                }
                if session.phase == whisper_core::ports::UnifiedSessionPhase::Finalizing
                    && c_finalizing_at.is_none()
                {
                    c_finalizing_at = Some(now);
                }
                if session.phase == whisper_core::ports::UnifiedSessionPhase::Publishing
                    && c_publishing_at.is_none()
                {
                    c_publishing_at = Some(now);
                }
                if session.phase == whisper_core::ports::UnifiedSessionPhase::Complete {
                    c_published_at = Some(now);
                    break;
                }
            }
            assert!(
                Instant::now() < c_publication_deadline,
                "C native publication did not complete"
            );
            thread::sleep(Duration::from_millis(10));
        }
        let c_files = ArchiveStore::published_unified_files(&context.archive_root, c, generation)
            .expect("C must publish a separate generation-bound result");
        assert!(Path::new(&c_files.wav_path).is_file());
        assert!(Path::new(&c_files.txt_path).is_file());
        assert!(Path::new(&c_files.srt_path).is_file());
        assert!(
            !context.snapshot_registry_root.exists(),
            "capture-only A/B/C publication must not create a source snapshot registry"
        );

        (
            c_stop,
            c_audio_drained_at,
            c_finalizing_at,
            c_publishing_at,
            c_published_at,
        )
    }

    #[cfg(windows)]
    fn native_ab_c_report(
        context: &native_execution_helpers::NativeAbcContext,
        c_timing: NativeAbcCTiming,
        c_incarnation: u64,
        c_pid: u32,
    ) {
        let compute = context.compute;
        let a_samples = context.audio.a_samples;
        let first_a_send = context
            .first_a_send
            .expect("A1 send time is available for summary");
        let first_a_result = context
            .first_a_result
            .expect("A1 result is available for summary");
        let b_audio_ack = context
            .b_audio_ack
            .expect("B durable ACK is available for summary");
        let first_text = context
            .first_text_available
            .expect("A text is available for summary");
        let a_stop = context
            .a_stop
            .expect("A stop time is available for summary");
        let b_stop = context
            .b_stop
            .expect("B stop time is available for summary");
        let (c_stop, c_audio_drained_at, c_finalizing_at, c_publishing_at, c_published_at) = c_timing;
        let startup = context
            .observations
            .iter()
            .find_map(|event| match &event.kind {
                NativeQualificationEventKind::Starting { incarnation } if *incarnation == 1 => {
                    Some(event.at)
                }
                _ => None,
            })
            .expect("initial child startup event");
        let ready = context
            .observations
            .iter()
            .find_map(|event| match &event.kind {
                NativeQualificationEventKind::Ready { incarnation: 1, .. } => Some(event.at),
                _ => None,
            })
            .expect("validated initial READY event");
        assert!(ready > startup && first_text > context.run_started);
        fn phase_elapsed_or_unobserved(stop: Instant, observed: Option<Instant>) -> String {
            observed.map_or_else(
                || "non observée".to_owned(),
                |at| format!("{:?}", at.duration_since(stop)),
            )
        }
        println!(
            "PU-L4 native {} A/B/C; synthetic cadence=source-derived 80k-frame burst blocks, OS cache state uncontrolled; start-to-READY={:?}, A1 send-to-result={:?}, B written-ACK within A1={:?}, start-to-first-app-text={:?}, A stop-to-audio-drain/finalizing/publishing/complete={:?}/{}/{}/{:?}, B stop-to-audio-drain/finalizing/publishing/complete={:?}/{}/{}/{:?}, C stop-to-audio-drain/finalizing/publishing/complete={:?}/{}/{}/{:?}, source prefix samples={a_samples}, C PID={c_pid} incarnation={c_incarnation}",
            if compute == ComputeChoice::Cpu {
                "CPU"
            } else {
                "GPU-attested"
            },
            ready.duration_since(startup),
            first_a_result.duration_since(first_a_send),
            b_audio_ack.0.duration_since(first_a_send),
            first_text.duration_since(context.run_started),
            context
                .times
                .a_audio_drained_at
                .expect("A final audio ACK time")
                .duration_since(a_stop),
            phase_elapsed_or_unobserved(a_stop, context.times.a_finalizing_at),
            phase_elapsed_or_unobserved(a_stop, context.times.a_publishing_at),
            context
                .times
                .a_published_at
                .expect("A completion time")
                .duration_since(a_stop),
            context
                .times
                .b_audio_drained_at
                .expect("B final audio ACK time")
                .duration_since(b_stop),
            phase_elapsed_or_unobserved(b_stop, context.times.b_finalizing_at),
            phase_elapsed_or_unobserved(b_stop, context.times.b_publishing_at),
            context
                .times
                .b_published_at
                .expect("B completion time")
                .duration_since(b_stop),
            c_audio_drained_at
                .expect("C final audio ACK time")
                .duration_since(c_stop),
            phase_elapsed_or_unobserved(c_stop, c_finalizing_at),
            phase_elapsed_or_unobserved(c_stop, c_publishing_at),
            c_published_at
                .expect("C completion time")
                .duration_since(c_stop),
        );
    }

    #[cfg(windows)]
    fn native_ab_c_phase_c(context: &mut native_execution_helpers::NativeAbcContext) {
        let c = context.ids.c;
        let generation = context.ids.generation;
        let reaped_before_c = context
            .reaped_before_c
            .expect("A/B terminal publications must reap the prior child");
        context.blocks.lock().unwrap().push_back((
            c,
            generation,
            context.audio.spoken[..4 * 16_000].to_vec(),
        ));
        context.checkpoint("before-Start-C", None);
        context
            .app
            .dispatch(AppCommand::StartUnifiedLiveConfigured {
                request: context.requests[2].clone(),
            })
            .expect("admit C through the same application and runtime after A/B publication");
        context.checkpoint("after-Start-C", None);
        let (c_incarnation, c_pid) = native_ab_c_wait_c_ready(context, reaped_before_c);
        let c_timing = native_ab_c_finish_c(context, c, generation);
        native_ab_c_report(context, c_timing, c_incarnation, c_pid);
    }

    #[test]
    #[cfg(windows)]
    #[cfg_attr(
        not(feature = "native-tests"),
        ignore = "requires a separately authorized PU-L4 native CPU campaign and the annotated guarded EN long fixture; requires WHISPER_L04_CAMPAIGN_RUN, LOCALAPPDATA/Whisper workers/model/native DLLs and the named WHISPER_PU_L4_FR_SOURCE, WHISPER_PU_L4_LONG_FR or WHISPER_PU_L4_LONG_EN guarded corpus fixture; GPU cases additionally require a compatible CUDA device"
    )]
    #[cfg(windows)]
    fn native_personal_cpu_ab_c_recycles_after_reap_with_spoken_burst_capture() {
        run_native_personal_ab_c(ComputeChoice::Cpu, None);
    }

    #[test]
    #[cfg(windows)]
    #[cfg_attr(
        not(feature = "native-tests"),
        ignore = "requires a separately authorized PU-L4 native GPU campaign, real GPU identity, and the annotated guarded EN long fixture; requires WHISPER_L04_CAMPAIGN_RUN, LOCALAPPDATA/Whisper workers/model/native DLLs and the named WHISPER_PU_L4_FR_SOURCE, WHISPER_PU_L4_LONG_FR or WHISPER_PU_L4_LONG_EN guarded corpus fixture; GPU cases additionally require a compatible CUDA device"
    )]
    #[cfg(windows)]
    fn native_personal_gpu_ab_c_recycles_after_reap_with_attested_candidate() {
        let local_app_data = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .expect("isolated LOCALAPPDATA is required")
            .canonicalize()
            .expect("isolated LOCALAPPDATA must exist");
        let workers = WorkerPaths {
            cpu: local_app_data.join("Whisper/app/whisper-worker-cpu.exe"),
            gpu: local_app_data.join("Whisper/app/whisper-worker-gpu.exe"),
        };
        let candidate = native_personal_gpu_candidate_for_harness(&workers);
        run_native_personal_ab_c(ComputeChoice::Gpu, Some(candidate));
    }

    #[test]
    #[cfg(windows)]
    #[cfg_attr(
        not(feature = "native-tests"),
        ignore = "requires a separately authorized PU-L4 native CPU campaign; uses synthetic capture only; requires WHISPER_L04_CAMPAIGN_RUN, LOCALAPPDATA/Whisper workers/model/native DLLs and the named WHISPER_PU_L4_FR_SOURCE, WHISPER_PU_L4_LONG_FR or WHISPER_PU_L4_LONG_EN guarded corpus fixture; GPU cases additionally require a compatible CUDA device"
    )]
    #[cfg(windows)]
    fn native_personal_cpu_finalizes_a_synthetic_capture_without_opening_hardware() {
        let execution_root = std::env::var_os("WHISPER_L04_CAMPAIGN_RUN")
            .map(PathBuf::from)
            .expect("guarded PU-L4 execution root is required")
            .canonicalize()
            .expect("guarded execution root must exist");
        let local_app_data = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .expect("guarded isolated LOCALAPPDATA is required")
            .canonicalize()
            .expect("guarded LOCALAPPDATA must exist");
        let whisper = local_app_data.join("Whisper");
        let app_root = whisper.join("app");
        let models_root = whisper.join("models");
        let cpu_paths = crate::parakeet_import::ParakeetWorkerPaths {
            executable: whisper
                .join("worker")
                .join("whisper-worker-parakeet-cpu.exe"),
            native_dir: whisper.join("native/cpu"),
            model: models_root.join("parakeet-tdt-0.6b-v3.q8_0.gguf"),
            runtime_root: whisper.clone(),
        };
        let turbo_model = models_root.join("ggml-large-v3-turbo.bin");
        let workers = WorkerPaths {
            cpu: app_root.join("whisper-worker-cpu.exe"),
            gpu: app_root.join("whisper-worker-gpu.exe"),
        };
        for path in [
            &workers.cpu,
            &workers.gpu,
            &cpu_paths.executable,
            &cpu_paths.native_dir,
            &cpu_paths.model,
            &turbo_model,
        ] {
            assert!(
                path.exists(),
                "required personal asset missing: {}",
                path.display()
            );
        }
        assert!(cpu_paths.validate().is_ok());

        let archive_root = execution_root.join("archive");
        std::fs::create_dir_all(&archive_root).unwrap();
        let model_catalog = Some(RuntimeModelCatalog {
            catalog: crate::model_catalog::LocalModelCatalog::new(models_root.clone()),
            known_historical_turbo_path: turbo_model.to_string_lossy().into_owned(),
            parakeet_paths: Some(cpu_paths.clone()),
            gpu_parakeet_paths: None,
        });
        let runtime = ImportRuntime::new_with_workers_and_parakeet_registry(
            workers,
            Arc::new(PublicationGate::default()),
            model_catalog,
            WorkerRuntimeConfig {
                parakeet_paths: Some(cpu_paths.clone()),
                gpu_parakeet_paths: None,
                snapshot_registry_root: Some(execution_root.join("settings")),
                require_snapshot_registry: true,
                native_unified_candidate_enabled: true,
            },
        )
        .with_controlled_capture(
            CaptureStream::empty_test_capture(16_000, 1, "test:pu-l4-synthetic-capture"),
            None,
        );
        let controlled_capture_blocks = Arc::new(Mutex::new(VecDeque::new()));
        let mut request = unified_live_request(&archive_root, 9_438);
        request.model_path = cpu_paths.model.to_string_lossy().into_owned();
        request.microphone =
            whisper_core::MicrophoneChoice::Device("test:pu-l4-synthetic-capture".into());
        request.config.compute = ComputeChoice::Cpu;
        let mut app = ImportApplication::new(RuntimeImportPort {
            runtime,
            events: VecDeque::new(),
            capture_progress: None,
            document_revision: None,
            unified_stop_submissions: None,
            controlled_capture_blocks: Some(Arc::clone(&controlled_capture_blocks)),
        });
        app.dispatch(AppCommand::StartUnifiedLiveConfigured {
            request: request.clone(),
        })
        .unwrap();
        controlled_capture_blocks.lock().unwrap().push_back((
            request.job_id,
            request.generation,
            vec![0.1; 4 * 16_000],
        ));

        stop_and_assert_synthetic_capture_published(&mut app, &request, &archive_root);
    }

    #[cfg(windows)]
    fn stop_and_assert_synthetic_capture_published(
        app: &mut ImportApplication<RuntimeImportPort>,
        request: &LiveRequest,
        archive_root: &Path,
    ) {
        let captured_deadline = Instant::now() + Duration::from_secs(60);
        let mut captured = false;
        while Instant::now() < captured_deadline {
            let view = app.dispatch(AppCommand::Refresh).unwrap();
            captured = view
                .unified_parakeet_sessions
                .iter()
                .find(|session| {
                    session.job_id == request.job_id && session.generation == request.generation
                })
                .is_some_and(|session| session.captured_samples > 0);
            if captured {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert!(
            captured,
            "the synthetic capture block did not make live progress"
        );
        app.dispatch(AppCommand::StopUnifiedLive {
            job_id: request.job_id,
            generation: request.generation,
        })
        .unwrap();

        let publication_deadline = Instant::now() + Duration::from_secs(15 * 60);
        let terminal = loop {
            let view = app.dispatch(AppCommand::Refresh).unwrap();
            if let Some(session) = view.unified_parakeet_sessions.iter().find(|session| {
                session.job_id == request.job_id && session.generation == request.generation
            }) && session.phase == whisper_core::ports::UnifiedSessionPhase::Complete
            {
                break view;
            }
            assert!(
                Instant::now() < publication_deadline,
                "native synthetic capture did not finalize: {view:?}"
            );
            thread::sleep(Duration::from_millis(25));
        };
        let session = terminal
            .unified_parakeet_sessions
            .iter()
            .find(|session| {
                session.job_id == request.job_id && session.generation == request.generation
            })
            .unwrap();
        assert_eq!(session.total_samples, Some(4 * 16_000));
        assert_eq!(session.captured_samples, 4 * 16_000);
        assert_eq!(session.durable_samples, 4 * 16_000);
        assert!(session.wav_path.is_some());
        assert!(session.txt_path.is_some());
        assert!(session.srt_path.is_some());
        let files =
            ArchiveStore::published_unified_files(archive_root, request.job_id, request.generation)
                .expect("native synthetic capture must publish referenced audio and text artifacts");
        for path in [files.wav_path, files.txt_path, files.srt_path] {
            assert!(
                Path::new(&path).is_file(),
                "published artifact missing: {path}"
            );
        }
    }

    #[test]
    #[cfg(windows)]
    #[cfg_attr(
        not(feature = "native-tests"),
        ignore = "requires the separately authorized PU-L4 native campaign and guarded short FR fixture; requires WHISPER_L04_CAMPAIGN_RUN, LOCALAPPDATA/Whisper workers/model/native DLLs and the named WHISPER_PU_L4_FR_SOURCE, WHISPER_PU_L4_LONG_FR or WHISPER_PU_L4_LONG_EN guarded corpus fixture; GPU cases additionally require a compatible CUDA device"
    )]
    fn native_queued_cpu_short_import_uses_the_durable_application_path() {
        run_native_queued_unified_import(ComputeChoice::Cpu, false, false);
    }

    #[test]
    #[cfg(windows)]
    #[cfg_attr(
        not(feature = "native-tests"),
        ignore = "requires the separately authorized PU-L4 native campaign, current GPU identity, and guarded short FR fixture; requires WHISPER_L04_CAMPAIGN_RUN, LOCALAPPDATA/Whisper workers/model/native DLLs and the named WHISPER_PU_L4_FR_SOURCE, WHISPER_PU_L4_LONG_FR or WHISPER_PU_L4_LONG_EN guarded corpus fixture; GPU cases additionally require a compatible CUDA device"
    )]
    #[cfg(windows)]
    fn native_queued_gpu_short_import_uses_current_attestation() {
        run_native_queued_unified_import(ComputeChoice::Gpu, false, true);
    }

    #[cfg(windows)]
    fn native_gpu_resume_runtime(
        setup: native_execution_helpers::NativeGpuResumeSetup,
    ) -> native_execution_helpers::NativeGpuResumeRuntime {
        let models_root = &setup.paths.models_root;
        let cpu_paths = &setup.paths.cpu_paths;
        let gpu_paths = &setup.paths.gpu_paths;
        let workers = &setup.paths.workers;
        let settings_root = setup.paths.execution_root.join("settings");
        let model_catalog = Some(RuntimeModelCatalog {
            catalog: crate::model_catalog::LocalModelCatalog::new(models_root.clone()),
            known_historical_turbo_path: models_root
                .join("ggml-large-v3-turbo.bin")
                .to_string_lossy()
                .into_owned(),
            parakeet_paths: Some(cpu_paths.clone()),
            gpu_parakeet_paths: Some(gpu_paths.clone()),
        });
        let (observer, native_events) = NativeQualificationObserver::channel();
        let native_observer = Arc::clone(&observer);
        let expected_profile = whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS;
        let expected_gpu_key = gpu_paths
            .gpu_qualification_key(expected_profile)
            .expect("current personal GPU route must expose its qualification key");
        let mut runtime = ImportRuntime::new_with_workers_and_parakeet_registry(
            workers.clone(),
            Arc::new(PublicationGate::default()),
            model_catalog,
            WorkerRuntimeConfig {
                parakeet_paths: Some(cpu_paths.clone()),
                gpu_parakeet_paths: Some(gpu_paths.clone()),
                snapshot_registry_root: Some(settings_root.clone()),
                require_snapshot_registry: true,
                native_unified_candidate_enabled: true,
            },
        )
        .with_native_harness_observer(observer);
        assert_eq!(setup.candidate.generation, setup.resumed_generation.get());
        runtime.gpu_candidates.insert(
            (setup.job_id, setup.resumed_generation.get()),
            Some(setup.candidate.clone()),
        );
        let mut app = ImportApplication::new(RuntimeImportPort {
            runtime,
            events: VecDeque::new(),
            capture_progress: None,
            document_revision: None,
            unified_stop_submissions: None,
            controlled_capture_blocks: None,
        });
        app.dispatch(AppCommand::ResumeInterrupted {
            request: setup.previous.clone(),
        })
        .expect("the real app path must resume the interrupted GPU request");

        native_execution_helpers::NativeGpuResumeRuntime {
            setup,
            app,
            observer: native_observer,
            native_events,
            expected_profile,
            expected_gpu_key,
        }
    }

    #[cfg(windows)]
    fn native_gpu_resume_wait(
        runtime: native_execution_helpers::NativeGpuResumeRuntime,
    ) -> native_execution_helpers::NativeGpuResumeRun {
        let native_execution_helpers::NativeGpuResumeRuntime {
            setup,
            mut app,
            observer: native_observer,
            native_events,
            expected_profile,
            expected_gpu_key,
        } = runtime;
        let archive_root = &setup.archive_root;
        let source_path = &setup.source_path;
        let source_sha = &setup.source_sha;
        let expected_samples = setup.expected_samples;
        let (complete, observed_native_events) =
            native_execution_resume_helpers::wait_for_native_gpu_resume(
                &mut app,
                &setup,
                &native_observer,
                &native_events,
            );
        assert_app_view_external_publication(
            &complete,
            archive_root,
            "fr-long-35m",
            source_sha,
            expected_samples,
        );
        assert_eq!(
            crate::parakeet_import::sha256_file(source_path).unwrap(),
            source_sha.as_str()
        );

        native_execution_helpers::NativeGpuResumeRun {
            setup,
            _app: app,
            complete,
            observed_native_events,
            expected_profile,
            expected_gpu_key,
        }
    }

    #[cfg(windows)]
    fn native_gpu_resume_verify_ready(
        run: &native_execution_helpers::NativeGpuResumeRun,
    ) -> (u64, u32) {
        let setup = &run.setup;
        let expected_profile = run.expected_profile;
        let expected_gpu_key = &run.expected_gpu_key;
        let candidate = &setup.candidate;
        let resumed_generation = setup.resumed_generation;
        let observed_native_events = &run.observed_native_events;
        let expected_backend = "CUDA0";
        let ready = observed_native_events
            .iter()
            .filter_map(|event| match &event.kind {
                NativeQualificationEventKind::Ready {
                    incarnation,
                    pid,
                    backend,
                    profile,
                    model_sha256,
                    runtime_sha256,
                    build_sha256,
                    gpu_attested,
                } => Some((
                    *incarnation,
                    *pid,
                    backend,
                    profile,
                    model_sha256,
                    runtime_sha256,
                    build_sha256,
                    *gpu_attested,
                )),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(ready.len(), 1, "resume must produce one native READY");
        let (
            ready_incarnation,
            ready_pid,
            ready_backend,
            ready_profile,
            ready_model_sha256,
            ready_runtime_sha256,
            ready_build_sha256,
            ready_gpu_attested,
        ) = ready[0];
        assert_eq!(ready_backend, &expected_backend);
        assert!(
            ready_gpu_attested,
            "GPU READY must follow current GPU attestation"
        );

        assert_eq!(ready_profile.as_str(), expected_profile);
        assert!(
            ready_model_sha256.eq_ignore_ascii_case(
                whisper_core::model_registry::model(
                    whisper_core::model_catalog::ModelId::ParakeetTdt06BV3
                )
                .artifact
                .sha256
            )
        );
        assert_eq!(ready_runtime_sha256, &expected_gpu_key.runtime_build);
        assert_eq!(ready_build_sha256, &expected_gpu_key.worker_build);
        assert_eq!(candidate.generation, resumed_generation.get());
        assert_eq!(
            candidate.pnp_instance_id,
            whisper_core::model_registry::PARAKEET_PERSONAL_GPU_PNP_INSTANCE_ID
        );
        assert_eq!(
            expected_gpu_key.gpu_pnp_instance_id.as_deref(),
            Some(whisper_core::model_registry::PARAKEET_PERSONAL_GPU_PNP_INSTANCE_ID)
        );
        assert_eq!(
            expected_gpu_key.gpu_pci_bus_id.as_deref(),
            Some(whisper_core::model_registry::PARAKEET_PERSONAL_GPU_PCI_BUS_ID)
        );
        assert_eq!(
            expected_gpu_key.gpu_uuid.as_deref(),
            Some(whisper_core::model_registry::PARAKEET_PERSONAL_GPU_UUID)
        );

        (ready_incarnation, ready_pid)
    }

    #[cfg(windows)]
    fn native_gpu_resume_verify_calls(
        run: &native_execution_helpers::NativeGpuResumeRun,
        ready: (u64, u32),
    ) {
        let job_id = run.setup.job_id;
        let resumed_generation = run.setup.resumed_generation;
        let observed_native_events = &run.observed_native_events;
        let ready_incarnation = ready.0;
        let ready_pid = ready.1;
        let scheduler_session = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id,
            generation: resumed_generation,
        };
        let sent_calls = observed_native_events
            .iter()
            .filter_map(|event| match &event.kind {
                NativeQualificationEventKind::RequestSent {
                    incarnation,
                    pid,
                    session,
                    generation,
                    attempt,
                    request_id,
                    work,
                } if *incarnation == ready_incarnation
                    && *pid == ready_pid
                    && *session == scheduler_session
                    && *generation == resumed_generation =>
                {
                    Some((*request_id, *attempt, *work))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        let completed_calls = observed_native_events
            .iter()
            .filter_map(|event| match &event.kind {
                NativeQualificationEventKind::Result {
                    incarnation,
                    pid,
                    session,
                    generation,
                    attempt,
                    request_id,
                    work,
                    ..
                } if *incarnation == ready_incarnation
                    && *pid == ready_pid
                    && *session == scheduler_session
                    && *generation == resumed_generation =>
                {
                    Some((*request_id, *attempt, *work))
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(
            !sent_calls.is_empty(),
            "attested worker must receive a correlated request"
        );
        let session_request_count = observed_native_events
            .iter()
            .filter(|event| {
                matches!(
                    &event.kind,
                    NativeQualificationEventKind::RequestSent { session, generation, .. }
                        if *session == scheduler_session && *generation == resumed_generation
                )
            })
            .count();
        let session_result_count = observed_native_events
            .iter()
            .filter(|event| {
                matches!(
                    &event.kind,
                    NativeQualificationEventKind::Result { session, generation, .. }
                        if *session == scheduler_session && *generation == resumed_generation
                )
            })
            .count();
        assert_eq!(session_request_count, sent_calls.len());
        assert_eq!(session_result_count, completed_calls.len());
        assert_eq!(
            sent_calls.len(),
            completed_calls.len(),
            "each request on the attested GPU incarnation must have one result"
        );
        for (request_id, attempt, work) in &sent_calls {
            assert!(*request_id > 0 && *attempt > 0);
            assert!(matches!(work, NativeQualificationWork::Final(_)));
            assert_eq!(
                completed_calls
                    .iter()
                    .filter(|completed| {
                        let (completed_request_id, completed_attempt, completed_work) = **completed;
                        completed_request_id == *request_id
                            && completed_attempt == *attempt
                            && completed_work == *work
                    })
                    .count(),
                1,
                "result must match the READY incarnation, PID, session, request ID, attempt, and work"
            );
        }
    }

    #[cfg(windows)]
    fn native_gpu_resume_verify_archive_document(
        run: &native_execution_helpers::NativeGpuResumeRun,
    ) -> crate::archive::ArchiveEntry {
        let setup = &run.setup;
        let archive_root = &setup.archive_root;
        let identity = (setup.job_id, setup.resumed_generation);
        let previous = &setup.previous;
        let source_sha = &setup.source_sha;
        let complete = &run.complete;
        let job_id = setup.job_id;
        let resumed_generation = setup.resumed_generation;
        let prefix_text = &setup.prefix_text;
        let prefix_range = setup.prefix_range;
        let entries = ArchiveStore::scan(archive_root).expect("scan resumed native archive");
        let complete_archive = entries
            .iter()
            .find(|entry| (entry.job_id, entry.generation) == identity && entry.complete)
            .cloned()
            .expect("resumed generation N+1 must publish a complete archive");
        assert_eq!(complete_archive.strategy, previous.strategy);
        assert_eq!(complete_archive.source_sha256, source_sha.as_str());
        let resumed_document = ArchiveStore::read_unified_document_page(
            archive_root,
            job_id,
            resumed_generation,
            complete.document_revision,
            1,
            128,
        )
        .expect("read the complete resumed document prefix and suffix");
        assert!(
            resumed_document.entries.len() >= 2,
            "resume must preserve the covered prefix and append a recalculated suffix: {:?}",
            resumed_document.entries
        );
        let prefix = &resumed_document.entries[0].segment;
        assert_eq!(prefix.segment_id.0, 1);
        assert_eq!(prefix.range.start_sample, prefix_range.start_sample);
        assert_eq!(prefix.range.end_sample, prefix_range.end_sample);
        assert_eq!(prefix.text, prefix_text.as_str());
        assert!(
            resumed_document.entries[1..]
                .iter()
                .enumerate()
                .all(|(index, entry)| entry.segment.segment_id.0 == index as u64 + 2),
            "recomputed suffix must continue from sequence M+1"
        );
        assert!(
            resumed_document.entries[1..]
                .iter()
                .all(|entry| entry.segment.text != prefix_text.as_str()),
            "the promoted prefix must not be duplicated in the recalculated suffix"
        );
        complete_archive
    }

    #[cfg(windows)]
    fn native_gpu_resume_verify_coverage(run: &native_execution_helpers::NativeGpuResumeRun) {
        let setup = &run.setup;
        let archive_root = &setup.archive_root;
        let job_id = setup.job_id;
        let resumed_generation = setup.resumed_generation;
        let expected_samples = setup.expected_samples;
        let checkpoint_samples = setup.checkpoint_samples;
        let resumed_document_directory = archive_root
            .join("transcriptions")
            .join(format!("{:032x}", job_id.0))
            .join(resumed_generation.get().to_string())
            .join("parakeet-unified/document");
        let mut coverage_reader =
            crate::parakeet_document::ParakeetDocumentStore::open_resume_coverage(
                &resumed_document_directory,
                job_id,
                resumed_generation,
                expected_samples,
            )
            .expect("read resumed generation audio coverage")
            .expect("complete resumed generation must have audio coverage");
        let mut next_window_sequence = 1_u64;
        let mut covered_samples = 0_u64;
        let mut first_coverage_end = None;
        let mut second_coverage_start = None;
        loop {
            let page = coverage_reader
                .read_page(128)
                .expect("read bounded resumed audio coverage page");
            if page.is_empty() {
                break;
            }
            for coverage in page {
                assert_eq!(coverage.sequence, next_window_sequence);
                assert_eq!(coverage.range.start_sample, covered_samples);
                if coverage.sequence == 1 {
                    assert_eq!(coverage.range.start_sample, 0);
                    assert_eq!(coverage.range.end_sample, checkpoint_samples);
                    first_coverage_end = Some(coverage.range.end_sample);
                } else if coverage.sequence == 2 {
                    second_coverage_start = Some(coverage.range.start_sample);
                }
                covered_samples = coverage.range.end_sample;
                next_window_sequence += 1;
            }
        }
        assert_eq!(first_coverage_end, Some(checkpoint_samples));
        assert_eq!(second_coverage_start, Some(checkpoint_samples));
        assert_eq!(covered_samples, expected_samples);
    }

    #[cfg(windows)]
    fn native_gpu_resume_verify_exports_and_release(
        run: &native_execution_helpers::NativeGpuResumeRun,
        complete_archive: &crate::archive::ArchiveEntry,
    ) {
        let setup = &run.setup;
        let archive_root = &setup.archive_root;
        let job_id = setup.job_id;
        let previous_generation = setup.previous_generation;
        let resumed_generation = setup.resumed_generation;
        let settings_root = setup.paths.execution_root.join("settings");
        let prefix_text = &setup.prefix_text;
        let source_sha = &setup.source_sha;
        let (txt, srt) = assert_native_generation_text_artifacts(archive_root, complete_archive);
        assert_eq!(
            txt.matches(prefix_text.as_str()).count(),
            1,
            "the confirmed prefix must appear exactly once in TXT"
        );
        assert_eq!(
            srt.matches(prefix_text.as_str()).count(),
            1,
            "the confirmed prefix must appear exactly once in SRT"
        );
        assert!(
            crate::source_binding::SourceBinding::reopen_for_identity(
                &settings_root,
                archive_root,
                job_id,
                previous_generation,
            )
            .unwrap()
            .is_none()
        );
        assert!(
            crate::source_binding::SourceBinding::reopen_for_identity(
                &settings_root,
                archive_root,
                job_id,
                resumed_generation,
            )
            .unwrap()
            .is_none()
        );
        assert_eq!(
            run.complete.phase,
            whisper_core::ports::UnifiedSessionPhase::Complete
        );
        println!(
            "PU_L4_NATIVE_RESUME_GPU_COMPLETE job={:032x} previous_generation={} generation={} backend=GPU gpu_attested=true source_sha256={source_sha}",
            job_id.0,
            previous_generation.get(),
            resumed_generation.get(),
        );
    }

    #[test]
    #[cfg(windows)]
    #[cfg_attr(
        not(feature = "native-tests"),
        ignore = "requires the separately authorized PU-L4 native GPU campaign, current GPU identity, and guarded 35-minute FR fixture; requires WHISPER_L04_CAMPAIGN_RUN, LOCALAPPDATA/Whisper workers/model/native DLLs and the named WHISPER_PU_L4_FR_SOURCE, WHISPER_PU_L4_LONG_FR or WHISPER_PU_L4_LONG_EN guarded corpus fixture; GPU cases additionally require a compatible CUDA device"
    )]
    fn native_queued_gpu_resume_uses_current_attestation_and_rebinds_snapshot() {
        let runtime =
            native_gpu_resume_runtime(native_execution_resume_helpers::native_gpu_resume_setup());
        let run = native_gpu_resume_wait(runtime);
        let ready = native_gpu_resume_verify_ready(&run);
        native_gpu_resume_verify_calls(&run, ready);
        let complete_archive = native_gpu_resume_verify_archive_document(&run);
        native_gpu_resume_verify_coverage(&run);
        native_gpu_resume_verify_exports_and_release(&run, &complete_archive);
    }

    #[test]
    #[cfg(windows)]
    #[cfg_attr(
        not(feature = "native-tests"),
        ignore = "requires the separately authorized PU-L4 native campaign, current GPU identity, and guarded short FR fixture; requires WHISPER_L04_CAMPAIGN_RUN, LOCALAPPDATA/Whisper workers/model/native DLLs and the named WHISPER_PU_L4_FR_SOURCE, WHISPER_PU_L4_LONG_FR or WHISPER_PU_L4_LONG_EN guarded corpus fixture; GPU cases additionally require a compatible CUDA device"
    )]
    fn native_queued_auto_short_import_prefers_current_gpu_candidate() {
        run_native_queued_unified_import(ComputeChoice::Auto, false, true);
    }

    #[test]
    #[cfg(windows)]
    #[cfg_attr(
        not(feature = "native-tests"),
        ignore = "requires the separately authorized PU-L4 native campaign and guarded short FR fixture; requires WHISPER_L04_CAMPAIGN_RUN, LOCALAPPDATA/Whisper workers/model/native DLLs and the named WHISPER_PU_L4_FR_SOURCE, WHISPER_PU_L4_LONG_FR or WHISPER_PU_L4_LONG_EN guarded corpus fixture; GPU cases additionally require a compatible CUDA device"
    )]
    #[cfg(windows)]
    fn native_queued_auto_short_import_falls_back_to_cpu_without_candidate() {
        run_native_queued_unified_import(ComputeChoice::Auto, false, false);
    }

    #[test]
    #[cfg(windows)]
    #[cfg_attr(
        not(feature = "native-tests"),
        ignore = "requires the separately authorized PU-L4 native campaign and guarded 35-minute FR fixture; requires WHISPER_L04_CAMPAIGN_RUN, LOCALAPPDATA/Whisper workers/model/native DLLs and the named WHISPER_PU_L4_FR_SOURCE, WHISPER_PU_L4_LONG_FR or WHISPER_PU_L4_LONG_EN guarded corpus fixture; GPU cases additionally require a compatible CUDA device"
    )]
    #[cfg(windows)]
    fn native_queued_cpu_long_import_uses_the_durable_application_path() {
        run_native_queued_unified_import(ComputeChoice::Cpu, true, false);
    }

    #[test]
    #[cfg(windows)]
    #[cfg_attr(
        not(feature = "native-tests"),
        ignore = "requires the separately authorized PU-L4 native campaign, current GPU identity, and guarded 35-minute FR fixture; requires WHISPER_L04_CAMPAIGN_RUN, LOCALAPPDATA/Whisper workers/model/native DLLs and the named WHISPER_PU_L4_FR_SOURCE, WHISPER_PU_L4_LONG_FR or WHISPER_PU_L4_LONG_EN guarded corpus fixture; GPU cases additionally require a compatible CUDA device"
    )]
    #[cfg(windows)]
    fn native_queued_gpu_long_import_uses_current_attestation() {
        run_native_queued_unified_import(ComputeChoice::Gpu, true, true);
    }

    #[test]
    #[cfg(windows)]
    #[cfg_attr(
        not(feature = "native-tests"),
        ignore = "requires the separately authorized PU-L4 native campaign, current GPU identity, and guarded 35-minute FR fixture; requires WHISPER_L04_CAMPAIGN_RUN, LOCALAPPDATA/Whisper workers/model/native DLLs and the named WHISPER_PU_L4_FR_SOURCE, WHISPER_PU_L4_LONG_FR or WHISPER_PU_L4_LONG_EN guarded corpus fixture; GPU cases additionally require a compatible CUDA device"
    )]
    #[cfg(windows)]
    fn native_queued_auto_long_import_prefers_current_gpu_candidate() {
        run_native_queued_unified_import(ComputeChoice::Auto, true, true);
    }
