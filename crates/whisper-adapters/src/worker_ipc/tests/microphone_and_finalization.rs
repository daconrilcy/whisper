    #[test]
    fn controlled_worker_and_invalid_times_leave_no_complete_archive() {
        for (index, (worker, expected_failure)) in [
            (
                Box::new(FailedUnifiedWorker)
                    as Box<dyn crate::parakeet_pipeline::WindowTranscriber>,
                "controlled worker failure",
            ),
            (
                Box::new(InvalidTimeUnifiedWorker)
                    as Box<dyn crate::parakeet_pipeline::WindowTranscriber>,
                "InvalidTimestamp",
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let root = std::env::temp_dir().join(format!(
                "parakeet-unified-failure-{}-{}",
                std::process::id(),
                index
            ));
            std::fs::create_dir_all(&root).unwrap();
            let source_path = root.join("source.wav");
            let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_path).unwrap();
            writer.append(&vec![0_i16; 3 * 16_000]).unwrap();
            writer.finish().unwrap();
            let request =
                unified_import_request(&root.join("archive"), &source_path, 780 + index as u128);
            assert_real_unified_model(&request);
            let runtime = ImportRuntime::new(
                WorkerPaths {
                    cpu: PathBuf::from("unused-cpu"),
                    gpu: PathBuf::from("unused-gpu"),
                },
                Arc::new(PublicationGate::default()),
            )
            .with_controlled_transcriber(worker);
            let mut app = ImportApplication::new(RuntimeImportPort {
                runtime,
                events: VecDeque::new(),
                capture_progress: None,
                document_revision: None,
                unified_stop_submissions: None,
                controlled_capture_blocks: None,
            });
            app.dispatch(AppCommand::StartImport {
                request: request.clone(),
            })
            .unwrap();
            let deadline = Instant::now() + Duration::from_secs(5);
            let mut terminal_view = None;
            while Instant::now() < deadline {
                let view = app.dispatch(AppCommand::Refresh).unwrap();
                if view.active_job == Some((request.job_id, JobState::Recoverable)) {
                    terminal_view = Some(view);
                    break;
                }
                thread::sleep(Duration::from_millis(2));
            }
            let terminal_view = terminal_view.expect("controlled import must report Recoverable");
            assert_eq!(
                terminal_view.active_job,
                Some((request.job_id, JobState::Recoverable))
            );
            assert!(
                terminal_view
                    .message
                    .as_deref()
                    .is_some_and(|message| message.contains(expected_failure))
            );
            let entries = ArchiveStore::scan(&root.join("archive")).unwrap();
            assert!(!entries.iter().any(|entry| entry.complete));
            std::fs::remove_dir_all(&root).unwrap();
        }
    }

    #[test]
    fn export_collision_keeps_existing_file_and_does_not_publish_unified_archive() {
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-export-collision-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source_path = root.join("source.wav");
        let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_path).unwrap();
        writer.append(&vec![0_i16; 3 * 16_000]).unwrap();
        writer.finish().unwrap();
        let destination = root.join("archive");
        let request = unified_import_request(&destination, &source_path, 790);
        assert_real_unified_model(&request);
        let collision = destination.join("exports/source.txt");
        let transcribe_completed = Arc::new(AtomicU64::new(0));
        let runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(Box::new(ExportCollisionWorker {
            path: collision.clone(),
            transcribe_completed: Arc::clone(&transcribe_completed),
        }));
        let mut app = ImportApplication::new(RuntimeImportPort {
            runtime,
            events: VecDeque::new(),
            capture_progress: None,
            document_revision: None,
            unified_stop_submissions: None,
            controlled_capture_blocks: None,
        });
        app.dispatch(AppCommand::StartImport {
            request: request.clone(),
        })
        .unwrap();
        let write_deadline = Instant::now() + Duration::from_secs(5);
        while transcribe_completed.load(Ordering::Acquire) == 0 && Instant::now() < write_deadline {
            let _ = app.dispatch(AppCommand::Refresh).unwrap();
            thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(
            transcribe_completed.load(Ordering::Acquire),
            1,
            "worker must create the collision after transcription starts"
        );
        let terminal_deadline = Instant::now() + Duration::from_secs(5);
        let mut terminal_view = None;
        while Instant::now() < terminal_deadline {
            let view = app.dispatch(AppCommand::Refresh).unwrap();
            if view.active_job == Some((request.job_id, JobState::Recoverable)) {
                terminal_view = Some(view);
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        let terminal_view = terminal_view.expect("export collision must be terminal");
        assert!(
            terminal_view
                .message
                .as_deref()
                .is_some_and(|message| message.contains("export name collision"))
        );
        assert_eq!(std::fs::read(&collision).unwrap(), b"preexisting export");
        assert!(
            !ArchiveStore::scan(&destination)
                .unwrap()
                .iter()
                .any(|entry| entry.complete)
        );
        let generation = destination
            .join("transcriptions")
            .join(format!("{:032x}", request.job_id.0))
            .join(request.generation.get().to_string());
        assert!(
            !generation.join("CURRENT").exists(),
            "failed export collision must not commit CURRENT"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn microphone_legacy_capture_budget_forces_worker_poll_within_four_ui_events() {
        let mut capture_events_since_worker = 0;
        let mut worker_events = VecDeque::from(["confirmed text", "terminal"]);
        let mut delivered = vec!["durable counters"];
        capture_events_since_worker += 1;
        delivered.push("spectrum telemetry");
        assert!(live_capture_poll_allowed(capture_events_since_worker, true));
        delivered.push("durable counters");
        capture_events_since_worker += 1;
        assert!(!live_capture_poll_allowed(
            capture_events_since_worker,
            true
        ));
        delivered.push(worker_events.pop_front().unwrap());
        capture_events_since_worker = 0;
        assert_eq!(
            delivered,
            [
                "durable counters",
                "spectrum telemetry",
                "durable counters",
                "confirmed text"
            ]
        );
        assert!(!live_capture_poll_allowed(2, true));
        assert!(live_capture_poll_allowed(2, false));
        assert_eq!(worker_events.pop_front(), Some("terminal"));
    }

    #[test]
    fn microphone_legacy_live_pump_advances_capture_spectrum_confirmed_text_and_end() {
        let root = std::env::temp_dir().join(format!(
            "legacy-spectrum-fairness-{}-{}",
            std::process::id(),
            whisper_core::microphone_level::monotonic_millis()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let marker = root.join("worker-ready");
        let worker = compile_fake_worker(&root, "cpu-live", &marker);
        let job_id = JobId(881);
        let generation = Generation::first();
        let request = LiveRequest {
            job_id,
            generation,
            model_path: "unused-model".into(),
            model_sha256: [0; 32],
            destination: root.join("archive").to_string_lossy().into_owned(),
            group_offset_samples: 0,
            auto_stop_after_speech_samples: None,
            microphone: whisper_core::MicrophoneChoice::Device("test:legacy".into()),
            microphone_gain_db: 0,
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Cpu,
            },
        };
        let mut capture = CaptureStream::empty_test_capture(16_000, 1, "test:legacy");
        let probe = crate::microphone_probe::MicrophoneProbe::start(
            16_000,
            Some("test:legacy".into()),
            job_id,
            generation,
        )
        .unwrap();
        let staging_path = root.join("pending.pcm");
        let staging = PcmStaging::create(&staging_path).unwrap().into_writer();
        let mut archive = LiveArchive::prepare(&request.destination, request.clone()).unwrap();
        archive.capture_started().unwrap();
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: worker.clone(),
                gpu: worker.clone(),
            },
            Arc::new(PublicationGate::default()),
        );
        let mut transport = ChildWorkerTransport::spawn(&worker).unwrap();
        transport
            .request(WorkerCommand::StartLiveWorker {
                job_id,
                generation,
                instance_id: 1,
                model_path: request.model_path.clone(),
                model_sha256: request.model_sha256,
                config: request.config.clone(),
            })
            .unwrap();
        runtime.transport = Some(transport);
        runtime.active = Some(ImportRequestState {
            job_id,
            generation,
            instance_id: 1,
            backend: BackendKind::Cpu,
            attempt_number: 0,
            import_request: None,
            recovery: LiveRecoveryState::None,
            startup_ready: true,
            gpu_candidate: None,
            gpu_attested: false,
        });
        runtime.live_archive = Some(archive);
        runtime.live = Some(LiveCaptureState {
            request: request.clone(),
            capture,
            probe,
            spectrum_reader: crate::microphone_probe::MicrophoneSpectrumReader::default(),
            last_meter_sequence: 0,
            next_spectrum_telemetry: false,
            telemetry_due_after_event: false,
            converter: Pcm16Converter::new(16_000, 1).unwrap(),
            vad: VoiceActivity::new(),
            staging,
            frame_pending: Vec::with_capacity(320),
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
            started_at_unix_ms: unix_time_ms(),
            captured: 0,
            admitted: 0,
            speech: 0,
            confirmed_fragment_end: 0,
            last_reported_durable: 0,
            failure: None,
        });

        let mut saw_counters = false;
        let mut saw_meter = false;
        let mut saw_spectrum = false;
        for block in 0..12 {
            let samples = (block * 960..(block + 1) * 960)
                .map(|frame| 0.001 * (std::f32::consts::TAU * 64.0 * frame as f32 / 2048.0).sin())
                .collect::<Vec<_>>();
            runtime
                .live
                .as_mut()
                .unwrap()
                .capture
                .enqueue_test_block(samples);
            let mut refresh_events = 0;
            for _ in 0..4 {
                match runtime.poll() {
                    Some(Ok(event)) => {
                        refresh_events += 1;
                        saw_counters |= matches!(event, ImportEvent::LiveCounters { .. });
                        saw_meter |= matches!(event, ImportEvent::MicrophoneMeter(_));
                        saw_spectrum |= matches!(event, ImportEvent::MicrophoneSpectrum(_));
                    }
                    Some(Err(error)) => panic!("legacy runtime poll failed: {error:?}"),
                    None => {}
                }
            }
            assert!(refresh_events <= 4);
            thread::sleep(Duration::from_millis(10));
        }
        let telemetry_deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < telemetry_deadline && !(saw_meter && saw_spectrum) {
            runtime
                .live
                .as_mut()
                .unwrap()
                .capture
                .enqueue_test_block(vec![0.0; 960]);
            if let Some(Ok(event)) = runtime.poll() {
                saw_counters |= matches!(event, ImportEvent::LiveCounters { .. });
                saw_meter |= matches!(event, ImportEvent::MicrophoneMeter(_));
                saw_spectrum |= matches!(event, ImportEvent::MicrophoneSpectrum(_));
            }
            thread::sleep(Duration::from_millis(5));
        }
        assert!(
            saw_counters,
            "legacy capture did not publish durable counters"
        );
        assert!(saw_meter, "legacy capture did not publish RMS telemetry");
        assert!(saw_spectrum, "legacy capture did not publish a spectrum");

        let segment = WorkerSegment {
            job_id,
            generation,
            instance_id: 1,
            segment_id: whisper_core::SegmentId(1),
            range: whisper_core::SourceRange::new(0, 320, 16_000).unwrap(),
            text: "legacy confirmed".into(),
        };
        while runtime.live.as_ref().unwrap().staging.durable_samples() < 320 {
            thread::sleep(Duration::from_millis(5));
        }
        runtime
            .live_archive
            .as_mut()
            .unwrap()
            .persist_segment(
                &segment,
                runtime.live.as_ref().unwrap().staging.durable_samples(),
            )
            .unwrap();
        let transport = runtime.transport.as_mut().unwrap();
        transport.inject_event(WorkerEvent::Segment(
            whisper_core::ipc::WorkerSegmentDto::from(segment.clone()),
        ));
        transport.inject_event(WorkerEvent::WindowFinished {
            job_id,
            generation,
            instance_id: 1,
            sequence: 1,
            range: segment.range,
            last_segment_sequence: 1,
        });
        transport.inject_event(WorkerEvent::End {
            job_id,
            generation,
            instance_id: 1,
            last_sequence: 1,
            last_offset: 320,
        });
        let mut saw_text = false;
        let mut saw_confirmed = false;
        let mut saw_end = false;
        let deadline = Instant::now() + Duration::from_secs(3);
        while Instant::now() < deadline && !(saw_text && saw_confirmed && saw_end) {
            runtime
                .live
                .as_mut()
                .unwrap()
                .capture
                .enqueue_test_block(vec![0.0; 960]);
            let mut refresh_events = 0;
            for _ in 0..4 {
                match runtime.poll() {
                    Some(Ok(event)) => {
                        refresh_events += 1;
                        match event {
                            ImportEvent::LiveProvisional { .. } => {
                                saw_text = true;
                                runtime.live_pending_persistence = 0;
                            }
                            ImportEvent::WindowFinished { .. } => saw_confirmed = true,
                            ImportEvent::End { .. } => saw_end = true,
                            _ => {}
                        }
                    }
                    Some(Err(error)) => panic!("legacy worker event failed: {error:?}"),
                    None => {}
                }
            }
            assert!(refresh_events <= 4);
            thread::sleep(Duration::from_millis(5));
        }
        assert!(saw_text, "mock worker confirmed text did not progress");
        assert!(saw_confirmed, "window confirmation did not progress");
        assert!(saw_end, "mock worker End did not progress");
        if let Some(mut live) = runtime.live.take() {
            live.stop_capture();
            drop(live);
        }
        runtime.transport.take();
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn microphone_gain_command_reaches_acquisition_and_finalized_wav_while_worker_is_delayed() {
        #[derive(Clone)]
        struct SimulatedPort(Rc<RefCell<Vec<ImportEffect>>>);

        impl ImportIoPort for SimulatedPort {
            fn submit(&mut self, effect: ImportEffect) -> Result<(), PortError> {
                self.0.borrow_mut().push(effect);
                Ok(())
            }

            fn poll(&mut self) -> Result<Option<ImportEvent>, PortError> {
                Ok(None)
            }
        }

        let root = std::env::temp_dir().join(format!(
            "whisper-t3-gain-e2e-{}-{}",
            std::process::id(),
            NEXT_INSTANCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        let request = LiveRequest {
            job_id: JobId(((std::process::id() as u128) << 64) | 0xA551),
            generation: Generation::first(),
            model_path: "delayed-engine-model".into(),
            model_sha256: [1; 32],
            destination: root.to_string_lossy().into_owned(),
            group_offset_samples: 0,
            auto_stop_after_speech_samples: None,
            microphone: whisper_core::MicrophoneChoice::Device("test:mic".into()),
            microphone_gain_db: 0,
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Cpu,
            },
        };
        let effects = Rc::new(RefCell::new(Vec::new()));
        let mut app = ImportApplication::new(SimulatedPort(effects.clone()));
        app.dispatch(AppCommand::StartDirectLiveConfigured {
            request: request.clone(),
        })
        .unwrap();
        let ImportEffect::StartDirectLive(request) = effects.borrow()[0].clone() else {
            panic!("direct start command must produce the direct capture effect")
        };

        let mut capture = crate::capture::CaptureStream::empty_test_capture(16_000, 1, "test:mic");
        capture.enqueue_test_block(vec![0.25; 16_000]);
        let direct =
            crate::direct_live::DirectLive::from_test_capture(request.clone(), capture).unwrap();
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: root.join("unused-cpu.exe"),
                gpu: root.join("unused-gpu.exe"),
            },
            Arc::new(PublicationGate::default()),
        );
        runtime.direct_live = Some(direct);
        let direct = runtime.direct_live.as_mut().unwrap();
        assert!(
            !direct.test_worker_ready(),
            "engine remains delayed in this test"
        );
        app.dispatch(AppCommand::SetMicrophoneGain {
            job_id: request.job_id,
            generation: request.generation,
            target_identity: "test:mic".into(),
            expected_revision: 0,
            gain: whisper_core::GainDb::new(6).unwrap(),
        })
        .unwrap();
        let gain_effect = effects.borrow().last().unwrap().clone();
        let mut events = VecDeque::new();
        runtime.effect(gain_effect, &mut events);
        assert!(
            events.is_empty(),
            "command acceptance is not an acquisition acknowledgement"
        );
        runtime
            .direct_live
            .as_mut()
            .unwrap()
            .pump_capture()
            .unwrap();
        assert!(
            matches!(
                runtime.direct_live.as_mut().unwrap().poll().unwrap(),
                Some(ImportEvent::MicrophoneGainApplied {
                    gain_db: 0,
                    gain_revision: 0,
                    first_frame: 0,
                    ..
                })
            ),
            "pre-command backlog acknowledges only its old revision"
        );
        assert!(matches!(
            runtime.direct_live.as_mut().unwrap().poll().unwrap(),
            Some(ImportEvent::DirectCaptureActive { .. })
        ));
        runtime
            .direct_live
            .as_mut()
            .unwrap()
            .enqueue_test_capture_block(vec![0.25; 16_000]);
        runtime
            .direct_live
            .as_mut()
            .unwrap()
            .pump_capture()
            .unwrap();
        assert!(matches!(
            runtime.direct_live.as_mut().unwrap().poll().unwrap(),
            Some(ImportEvent::MicrophoneGainApplied {
                gain_db: 6,
                gain_revision: 1,
                first_frame: 16_000,
                ..
            })
        ));
        runtime.effect(
            ImportEffect::SetMicrophoneGain {
                job_id: request.job_id,
                generation: request.generation,
                target_identity: "test:other".into(),
                expected_revision: 1,
                gain_db: 0,
            },
            &mut events,
        );
        assert!(matches!(
            events.pop_front(),
            Some(ImportEvent::MicrophoneGainRejected { target_identity, .. })
                if target_identity == "test:other"
        ));
        assert!(
            runtime.direct_live.is_some(),
            "gain rejection must preserve the capture owner"
        );
        app.dispatch(AppCommand::Stop {
            job_id: request.job_id,
            generation: request.generation,
        })
        .unwrap();
        let stop_effect = effects.borrow().last().unwrap().clone();
        runtime.effect(stop_effect, &mut events);
        assert!(matches!(
            events.pop_front(),
            Some(ImportEvent::LiveFinalizing { .. })
        ));

        let wav_path = runtime
            .direct_live
            .as_mut()
            .unwrap()
            .finish_test_wav()
            .unwrap();
        let bytes = fs::read(&wav_path).unwrap();
        assert_eq!(
            u32::from_le_bytes(bytes[40..44].try_into().unwrap()),
            64_000
        );
        let pcm: Vec<i16> = bytes[44..]
            .chunks_exact(2)
            .map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        assert_eq!(pcm.len(), 32_000);
        assert!(pcm[320..15_680].iter().all(|sample| *sample == 8_192));
        assert!(
            pcm[16_320..31_680]
                .iter()
                .all(|sample| (16_300..=16_400).contains(sample))
        );
        fs::remove_file(wav_path).unwrap();
        fs::remove_dir(root).unwrap();
    }

    #[test]
    fn stopping_before_the_first_gain_revision_block_emits_no_applied_acknowledgement() {
        let capture = crate::capture::CaptureStream::empty_test_capture(16_000, 1, "test:mic");
        let (mut test, identity) =
            MicrophoneTestCapture::from_capture(JobId(904), Generation::first(), capture).unwrap();
        let accepted = test
            .set_gain(&identity, 0, whisper_core::GainDb::new(6).unwrap())
            .unwrap();
        assert_eq!(accepted.revision, 1);
        assert!(
            test.pump().unwrap().is_none(),
            "no captured block means no applied event"
        );
        assert!(matches!(
            test.stop(),
            ImportEvent::MicrophoneTestStopped {
                job_id: JobId(904),
                ..
            }
        ));
    }

    #[test]
    fn microphone_test_capture_owns_only_capture_conversion_and_probe_until_stop() {
        let mut capture =
            crate::capture::CaptureStream::empty_test_capture(48_000, 1, "wasapi:test-device");
        let (mut test, identity) =
            MicrophoneTestCapture::from_capture(JobId(904), Generation::first(), capture).unwrap();
        assert_eq!(identity, "wasapi:test-device");

        let mut observed_meter = None;
        let mut observed_spectrum = None;
        let deadline = Instant::now() + Duration::from_secs(2);
        for _ in 0..5 {
            let processed_before = test.probe.processed_chunks();
            test.capture.enqueue_test_block(vec![0.25; 960]);
            if let Some(event) = test.pump().unwrap() {
                match event {
                    ImportEvent::MicrophoneMeter(meter) => observed_meter = Some(meter),
                    ImportEvent::MicrophoneSpectrum(update) => observed_spectrum = Some(update),
                    _ => {}
                }
            }
            while test.probe.processed_chunks() == processed_before && Instant::now() < deadline {
                std::thread::yield_now();
            }
            if let Some(event) = test.pump().unwrap() {
                match event {
                    ImportEvent::MicrophoneMeter(meter) => observed_meter = Some(meter),
                    ImportEvent::MicrophoneSpectrum(update) => observed_spectrum = Some(update),
                    _ => {}
                }
            }
            std::thread::yield_now();
        }
        while (observed_meter.is_none() || observed_spectrum.is_none()) && Instant::now() < deadline
        {
            if let Some(event) = test.pump().unwrap() {
                match event {
                    ImportEvent::MicrophoneMeter(meter) => observed_meter = Some(meter),
                    ImportEvent::MicrophoneSpectrum(update) => observed_spectrum = Some(update),
                    _ => {}
                }
            }
            std::thread::yield_now();
        }
        let meter = observed_meter.expect("probe reports captured audio");
        assert_eq!(meter.source_identity, identity);
        assert_eq!(meter.job_id, JobId(904));
        assert_eq!(
            meter.pre_gain.status,
            whisper_core::microphone_level::MeasurementStatus::Signal
        );
        let whisper_core::microphone_level::MicrophoneSpectrumUpdate::Available(spectrum) =
            observed_spectrum.expect("microphone test publishes a spectrum during acquisition")
        else {
            panic!("expected a current microphone spectrum")
        };
        assert_eq!(spectrum.source_identity, identity);
        assert_eq!(spectrum.pre_gain.len(), 1025);

        let applied = test
            .set_gain(
                &identity,
                0,
                whisper_core::microphone_level::GainDb::new(6).unwrap(),
            )
            .unwrap();
        assert_eq!(applied.gain.value(), 6);
        assert_eq!(applied.revision, 1);

        assert!(matches!(
            test.stop(),
            ImportEvent::MicrophoneTestStopped {
                job_id: JobId(904),
                ..
            }
        ));
    }

    fn automatic_final_test_root(label: &str) -> PathBuf {
        PathBuf::from(std::env::var_os("CARGO_TARGET_DIR").expect("guarded target dir")).join(
            format!(
                "whisper-auto-final-{label}-{}-{}",
                std::process::id(),
                NEXT_AUTO_EXPORT_STAGE.fetch_add(1, Ordering::Relaxed)
            ),
        )
    }

    fn automatic_final_request(job_id: JobId) -> whisper_core::ports::AutomaticFinalImport {
        let generation = Generation::first();
        let mut request = test_request(job_id, generation);
        request.source_path = format!("capture-{job_id:?}.wav");
        whisper_core::ports::AutomaticFinalImport {
            request,
            outputs: whisper_core::ports::AutomaticFinalOutputPaths {
                txt_path: format!("capture-{job_id:?}-auto.txt"),
                srt_path: format!("capture-{job_id:?}-auto.srt"),
                companion_json_path: format!("capture-{job_id:?}.json"),
            },
            gpu_candidate: None,
        }
    }

    fn wait_for_event(
        io: &mut AsyncImportIo,
        phase: &str,
        timeout: Duration,
        predicate: impl Fn(&ImportEvent) -> bool,
    ) -> ImportEvent {
        let started = Instant::now();
        let deadline = started + timeout;
        loop {
            assert!(
                Instant::now() < deadline,
                "expected runtime event timed out in phase `{phase}`; elapsed={:?}",
                started.elapsed()
            );
            match io.poll().unwrap() {
                Some(event) if predicate(&event) => return event,
                Some(ImportEvent::Queue(_) | ImportEvent::ReplacementCleanups(_)) => {}
                Some(event) => panic!("unexpected runtime event: {event:?}"),
                None => thread::sleep(Duration::from_millis(5)),
            }
        }
    }

    fn direct_live_request(job_id: JobId) -> LiveRequest {
        LiveRequest {
            job_id,
            generation: Generation::first(),
            model_path: "model.nemo".into(),
            model_sha256: [0; 32],
            destination: "live-archive".into(),
            group_offset_samples: 0,
            auto_stop_after_speech_samples: None,
            microphone: Default::default(),
            microphone_gain_db: 0,
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Gpu,
            },
        }
    }

    fn write_test_wav_header(file: &mut fs::File, samples: u32) -> io::Result<()> {
        use std::io::{Seek, SeekFrom};

        let data_bytes = samples.saturating_mul(2);
        file.seek(SeekFrom::Start(0))?;
        file.write_all(b"RIFF")?;
        file.write_all(&(36_u32.saturating_add(data_bytes)).to_le_bytes())?;
        file.write_all(b"WAVEfmt ")?;
        file.write_all(&16_u32.to_le_bytes())?;
        file.write_all(&1_u16.to_le_bytes())?;
        file.write_all(&1_u16.to_le_bytes())?;
        file.write_all(&16_000_u32.to_le_bytes())?;
        file.write_all(&32_000_u32.to_le_bytes())?;
        file.write_all(&2_u16.to_le_bytes())?;
        file.write_all(&16_u16.to_le_bytes())?;
        file.write_all(b"data")?;
        file.write_all(&data_bytes.to_le_bytes())?;
        file.seek(SeekFrom::Start(u64::from(data_bytes) + 44))?;
        file.flush()
    }

    struct IncrementalWavCapture {
        path: PathBuf,
        file: Option<fs::File>,
        identity: Option<(JobId, Generation)>,
        samples: u32,
    }

    impl ServiceRuntime for IncrementalWavCapture {
        fn apply_effect(&mut self, effect: ImportEffect, pending: &mut VecDeque<ImportEvent>) {
            match effect {
                ImportEffect::StartDirectLive(request) => {
                    if let Some(parent) = self.path.parent() {
                        fs::create_dir_all(parent).unwrap();
                    }
                    let mut file = fs::OpenOptions::new()
                        .create_new(true)
                        .write(true)
                        .read(true)
                        .open(&self.path)
                        .unwrap();
                    write_test_wav_header(&mut file, 0).unwrap();
                    self.file = Some(file);
                    self.identity = Some((request.job_id, request.generation));
                }
                ImportEffect::Stop { job_id, generation }
                    if self.identity == Some((job_id, generation)) =>
                {
                    if let Some(mut file) = self.file.take() {
                        write_test_wav_header(&mut file, self.samples).unwrap();
                        file.sync_all().unwrap();
                    }
                    self.identity = None;
                    pending.push_back(ImportEvent::Stopped { job_id, generation });
                }
                _ => {}
            }
        }

        fn poll_event(&mut self) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
            None
        }

        fn pump_realtime(&mut self) -> Option<(JobId, Generation, String)> {
            let file = self.file.as_mut()?;
            let mut block = [0_i16; 160];
            for sample in &mut block {
                *sample = (self.samples & 0x7fff) as i16;
                self.samples = self.samples.saturating_add(1);
            }
            for sample in block {
                if let Err(error) = file.write_all(&sample.to_le_bytes()) {
                    let identity = self.identity.take();
                    self.file = None;
                    return identity.map(|(job_id, generation)| {
                        (
                            job_id,
                            generation,
                            format!("test WAV write failed: {error}"),
                        )
                    });
                }
            }
            let _ = file.flush();
            None
        }
    }

    struct SampleDrivenWavCapture {
        path: PathBuf,
        file: Option<fs::File>,
        identity: Option<(JobId, Generation)>,
        samples_rx: Receiver<(Vec<i16>, SyncSender<Result<u64, String>>)>,
        started_tx: SyncSender<(JobId, Generation)>,
        samples: Vec<i16>,
    }

    impl ServiceRuntime for SampleDrivenWavCapture {
        fn apply_effect(&mut self, effect: ImportEffect, pending: &mut VecDeque<ImportEvent>) {
            match effect {
                ImportEffect::StartDirectLive(request) => {
                    if let Some(parent) = self.path.parent() {
                        fs::create_dir_all(parent).unwrap();
                    }
                    let mut file = fs::OpenOptions::new()
                        .create_new(true)
                        .write(true)
                        .read(true)
                        .open(&self.path)
                        .unwrap();
                    write_test_wav_header(&mut file, 0).unwrap();
                    self.file = Some(file);
                    self.identity = Some((request.job_id, request.generation));
                    self.started_tx
                        .send((request.job_id, request.generation))
                        .unwrap();
                }
                ImportEffect::Stop { job_id, generation }
                    if self.identity == Some((job_id, generation)) =>
                {
                    if let Some(mut file) = self.file.take() {
                        write_test_wav_header(&mut file, self.samples.len() as u32).unwrap();
                        file.sync_all().unwrap();
                    }
                    self.identity = None;
                    pending.push_back(ImportEvent::Stopped { job_id, generation });
                }
                _ => {}
            }
        }

        fn poll_event(&mut self) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
            None
        }

        fn pump_realtime(&mut self) -> Option<(JobId, Generation, String)> {
            let (block, acknowledgement) = match self.samples_rx.try_recv() {
                Ok(block) => block,
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => return None,
            };
            let Some(file) = self.file.as_mut() else {
                let _ = acknowledgement.send(Err("capture is not active".into()));
                return None;
            };
            let write_result = block
                .iter()
                .try_for_each(|sample| file.write_all(&sample.to_le_bytes()))
                .and_then(|()| file.flush());
            if let Err(error) = write_result {
                let identity = self.identity.take();
                self.file = None;
                let message = format!("test WAV write failed: {error}");
                let _ = acknowledgement.send(Err(message.clone()));
                return identity.map(|(job_id, generation)| (job_id, generation, message));
            }
            self.samples.extend_from_slice(&block);
            let _ = acknowledgement.send(Ok(self.samples.len() as u64));
            None
        }
    }

    struct RuntimeAutomaticPublisher {
        runtime: ImportRuntime,
        gates: Arc<Mutex<HashMap<(JobId, u64), Arc<PublicationGate>>>>,
        started_tx: SyncSender<(JobId, Generation)>,
        publication_ready_tx: SyncSender<Result<(JobId, Generation), String>>,
        release_rx: Receiver<()>,
        model_path: PathBuf,
        source_samples: Vec<i16>,
    }

    impl ServiceRuntime for RuntimeAutomaticPublisher {
        fn apply_effect(&mut self, effect: ImportEffect, pending: &mut VecDeque<ImportEvent>) {
            let ImportEffect::StartAutomaticFinalTranscription(mut automatic) = effect else {
                return;
            };
            let identity = (automatic.request.job_id, automatic.request.generation.get());
            if let Some(gate) = self
                .gates
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .get(&identity)
                .cloned()
            {
                self.runtime.publication_gate = gate;
            }
            let mut publication_ready = false;
            let publish_result = (|| {
                self.started_tx
                    .send((automatic.request.job_id, automatic.request.generation))
                    .map_err(|_| "test runtime start acknowledgement disconnected".to_owned())?;
                write_fixture_wav(
                    Path::new(&automatic.request.source_path),
                    &self.source_samples,
                )
                .map_err(|error| error.to_string())?;
                automatic.request.model_path = self.model_path.to_string_lossy().into_owned();
                automatic.request.destination = self
                    .runtime
                    .archive
                    .root_path()
                    .to_string_lossy()
                    .into_owned();
                automatic.request.model_sha256 =
                    crate::model_catalog::cached_model_sha256(&self.model_path)
                        .map_err(|error| error.to_string())?
                        .1;
                let prepared = self
                    .runtime
                    .archive
                    .prepare(automatic.request.clone())
                    .map_err(|error| error.to_string())?;
                automatic.request = prepared;
                let segment = whisper_core::ports::WorkerSegment {
                    job_id: automatic.request.job_id,
                    generation: automatic.request.generation,
                    instance_id: 1,
                    segment_id: whisper_core::SegmentId(1),
                    range: whisper_core::SourceRange::new(
                        0,
                        self.source_samples.len() as u64,
                        16_000,
                    )
                    .ok_or_else(|| "test source range is empty".to_owned())?,
                    text: format!("runtime transcript {}", automatic.request.job_id.0),
                };
                self.runtime
                    .archive
                    .persist_segment(&segment)
                    .map_err(|error| error.to_string())?;
                fs::write(
                    &automatic.outputs.companion_json_path,
                    r#"{"confirmed_intervals":[],"unresolved_intervals":[]}"#,
                )
                .map_err(|error| error.to_string())?;
                self.runtime
                    .automatic_final_outputs
                    .insert(identity, automatic.clone());
                self.publication_ready_tx
                    .send(Ok((automatic.request.job_id, automatic.request.generation)))
                    .map_err(|_| {
                        "test publication-ready acknowledgement disconnected".to_owned()
                    })?;
                publication_ready = true;
                self.release_rx
                    .recv_timeout(Duration::from_secs(120))
                    .map_err(|_| {
                        "test publication release timed out after preparation".to_owned()
                    })?;
                self.runtime
                    .publish(
                        automatic.request.job_id,
                        automatic.request.generation,
                        1,
                        pending,
                    )
                    .map_err(|(_, _, error)| error)
            })();

            if let Err(message) = publish_result {
                if !publication_ready {
                    let _ = self.publication_ready_tx.send(Err(message.clone()));
                }
                pending.push_back(ImportEvent::Failed {
                    job_id: identity.0,
                    generation: automatic.request.generation,
                    message,
                });
            }
        }

        fn poll_event(&mut self) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
            None
        }
    }

    fn write_fixture_wav(path: &Path, samples: &[i16]) -> io::Result<()> {
        let mut file = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .read(true)
            .open(path)?;
        write_test_wav_header(&mut file, u32::try_from(samples.len()).unwrap())?;
        for sample in samples {
            file.write_all(&sample.to_le_bytes())?;
        }
        file.sync_all()
    }

    struct SlowAutomaticFinal {
        delay: Duration,
    }

    impl ServiceRuntime for SlowAutomaticFinal {
        fn apply_effect(&mut self, effect: ImportEffect, pending: &mut VecDeque<ImportEvent>) {
            if let ImportEffect::StartAutomaticFinalTranscription(automatic) = effect {
                // Models a slow source-copy/hash preparation on the final-import lane.
                thread::sleep(self.delay);
                pending.push_back(ImportEvent::Failed {
                    job_id: automatic.request.job_id,
                    generation: automatic.request.generation,
                    message: "simulated final import failure after slow preparation".into(),
                });
            }
        }

        fn poll_event(&mut self) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
            None
        }
    }

    struct BlockedAutomaticQueueInspection {
        inspection_started: mpsc::SyncSender<()>,
        inspection_finished: mpsc::SyncSender<()>,
        release_inspection: Receiver<()>,
    }

    impl ServiceRuntime for BlockedAutomaticQueueInspection {
        fn apply_effect(&mut self, effect: ImportEffect, pending: &mut VecDeque<ImportEvent>) {
            let ImportEffect::StartAutomaticFinalTranscription(automatic) = effect else {
                return;
            };
            let (job_id, generation) = (automatic.request.job_id, automatic.request.generation);
            let inspection = inspect_automatic_final_queue_with(
                Path::new(&automatic.request.destination),
                |destination| {
                    self.inspection_started.send(()).unwrap();
                    self.release_inspection.recv().unwrap();
                    crate::queue_store::QueueStore::inspect_read_only(destination)
                },
            );
            self.inspection_finished.send(()).unwrap();
            let message = match inspection {
                Ok(_) => "simulated final import failure after queue inspection".to_owned(),
                Err(error) => error,
            };
            pending.push_back(ImportEvent::Failed {
                job_id,
                generation,
                message,
            });
        }

        fn poll_event(&mut self) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
            None
        }
    }

    #[test]
    fn live_slow_automatic_final_failure_does_not_block_another_capture_wav() {
        let root = automatic_final_test_root("r5-slow-final");
        fs::create_dir_all(&root).unwrap();
        let wav_path = root.join("capture-b.wav");
        let main_gate = Arc::new(PublicationGate::default());
        let (
            mut io,
            effects,
            stops,
            app_events,
            automatic_effects,
            automatic_stops,
            automatic_events,
        ) = AsyncImportIo::for_test(main_gate.clone());
        let main_thread = thread::spawn({
            let wav_path = wav_path.clone();
            move || {
                service_loop(
                    effects,
                    stops,
                    app_events,
                    IncrementalWavCapture {
                        path: wav_path,
                        file: None,
                        identity: None,
                        samples: 0,
                    },
                )
            }
        });
        let automatic_thread = thread::spawn(move || {
            service_loop(
                automatic_effects,
                automatic_stops,
                automatic_events,
                SlowAutomaticFinal {
                    delay: Duration::from_millis(2_150),
                },
            )
        });

        let import_a = automatic_final_request(JobId(0xA4));
        io.submit(ImportEffect::StartAutomaticFinalTranscription(
            import_a.clone(),
        ))
        .unwrap();
        let started_at = Instant::now();
        thread::sleep(Duration::from_millis(100));
        let capture_b = JobId(0xB3);
        io.submit(ImportEffect::StartDirectLive(direct_live_request(
            capture_b,
        )))
        .unwrap();

        let deadline = Instant::now() + Duration::from_millis(700);
        while Instant::now() < deadline
            && fs::metadata(&wav_path).map_or(true, |metadata| metadata.len() <= 44)
        {
            thread::sleep(Duration::from_millis(10));
        }
        let wav_size_during_import = fs::metadata(&wav_path).unwrap().len();
        assert!(wav_size_during_import > 44, "capture B WAV did not advance");

        let deadline = Instant::now() + Duration::from_secs(4);
        let mut final_failure_at = None;
        loop {
            assert!(
                Instant::now() < deadline,
                "slow final import did not finish"
            );
            match io.poll().unwrap() {
                Some(ImportEvent::Failed { job_id, .. }) if job_id == import_a.request.job_id => {
                    final_failure_at = Some(Instant::now());
                    break;
                }
                Some(other) => panic!("unexpected event during R5 simulation: {other:?}"),
                None => thread::sleep(Duration::from_millis(5)),
            }
        }
        let final_failure_at = final_failure_at.unwrap();
        assert!(final_failure_at.duration_since(started_at) >= Duration::from_millis(2_000));
        let wav_size_after_import = fs::metadata(&wav_path).unwrap().len();
        assert!(
            wav_size_after_import > wav_size_during_import,
            "capture B WAV stopped advancing while final import A was active"
        );
        assert_eq!(io.active_identity, Some((capture_b, Generation::first())));
        assert_eq!(main_gate.0.load(Ordering::Acquire), GATE_ACTIVE);

        io.submit(ImportEffect::Stop {
            job_id: capture_b,
            generation: Generation::first(),
        })
        .unwrap();
        let stop_deadline = Instant::now() + Duration::from_secs(1);
        loop {
            assert!(Instant::now() < stop_deadline, "capture B did not stop");
            match io.poll().unwrap() {
                Some(ImportEvent::Stopped { job_id, .. }) if job_id == capture_b => break,
                Some(other) => panic!("unexpected event while stopping B: {other:?}"),
                None => thread::sleep(Duration::from_millis(5)),
            }
        }

        let bytes = fs::read(&wav_path).unwrap();
        assert_eq!(&bytes[..4], b"RIFF");
        assert_eq!(
            u32::from_le_bytes(bytes[40..44].try_into().unwrap()) as usize,
            bytes.len() - 44
        );
        assert!(
            bytes.len() > 44,
            "finalized WAV contains no captured samples"
        );

        drop(io);
        main_thread.join().unwrap();
        automatic_thread.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn live_blocked_automatic_queue_inspection_does_not_block_capture_b_stop_or_wav() {
        let root = automatic_final_test_root("r5-blocked-queue");
        fs::create_dir_all(&root).unwrap();
        let wav_path = root.join("capture-b.wav");
        let gate = Arc::new(PublicationGate::default());
        let (
            mut io,
            effects,
            stops,
            app_events,
            automatic_effects,
            automatic_stops,
            automatic_events,
        ) = AsyncImportIo::for_test(gate.clone());
        let (samples_tx, samples_rx) = mpsc::sync_channel(3);
        let (capture_started_tx, capture_started_rx) = mpsc::sync_channel(1);
        let main_thread = thread::spawn({
            let wav_path = wav_path.clone();
            move || {
                service_loop(
                    effects,
                    stops,
                    app_events,
                    SampleDrivenWavCapture {
                        path: wav_path,
                        file: None,
                        identity: None,
                        samples_rx,
                        started_tx: capture_started_tx,
                        samples: Vec::new(),
                    },
                )
            }
        });
        let capture_b = JobId(0xB5);
        io.submit(ImportEffect::StartDirectLive(direct_live_request(
            capture_b,
        )))
        .unwrap();
        assert_eq!(
            capture_started_rx
                .recv_timeout(Duration::from_secs(1))
                .unwrap(),
            (capture_b, Generation::first())
        );
        let mut acknowledged_samples = 0_u64;
        let mut inject = |samples: Vec<i16>| {
            let block_samples = u64::try_from(samples.len()).unwrap();
            let expected_cumulative = acknowledged_samples + block_samples;
            let (ack_tx, ack_rx) = mpsc::sync_channel(1);
            samples_tx.send((samples.clone(), ack_tx)).unwrap();
            let actual_cumulative = ack_rx
                .recv_timeout(Duration::from_secs(1))
                .unwrap()
                .unwrap();
            assert_eq!(actual_cumulative, expected_cumulative);
            assert!(actual_cumulative > acknowledged_samples);
            acknowledged_samples = actual_cumulative;
            actual_cumulative
        };
        let before_inspection = vec![101, 102, 103, 104];
        let acknowledged_before = inject(before_inspection.clone());
        assert_eq!(acknowledged_before, before_inspection.len() as u64);
        assert_eq!(acknowledged_before, 4);

        let (inspection_started_tx, inspection_started_rx) = mpsc::sync_channel(1);
        let (inspection_finished_tx, inspection_finished_rx) = mpsc::sync_channel(1);
        let (release_inspection_tx, release_inspection_rx) = mpsc::sync_channel(1);
        let automatic_thread = thread::spawn(move || {
            service_loop(
                automatic_effects,
                automatic_stops,
                automatic_events,
                BlockedAutomaticQueueInspection {
                    inspection_started: inspection_started_tx,
                    inspection_finished: inspection_finished_tx,
                    release_inspection: release_inspection_rx,
                },
            )
        });

        let mut import_a = automatic_final_request(JobId(0xA5));
        import_a.request.destination = root.to_string_lossy().into_owned();
        io.submit(ImportEffect::StartAutomaticFinalTranscription(
            import_a.clone(),
        ))
        .unwrap();
        inspection_started_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("automatic lane did not enter queue inspection");
        let during_inspection = vec![-201, -202, -203];
        let acknowledged_during = inject(during_inspection.clone());
        assert_eq!(
            acknowledged_during,
            (before_inspection.len() + during_inspection.len()) as u64
        );
        assert_eq!(acknowledged_during, 7);
        assert_eq!(
            inspection_finished_rx.recv_timeout(Duration::from_millis(2_100)),
            Err(mpsc::RecvTimeoutError::Timeout),
            "queue inspection unexpectedly finished before the safety interval"
        );
        let mut samples_while_inspection_blocked = before_inspection.clone();
        samples_while_inspection_blocked.extend(during_inspection.clone());
        let expected_blocked_payload = samples_while_inspection_blocked
            .iter()
            .flat_map(|sample| sample.to_le_bytes())
            .collect::<Vec<_>>();
        let wav_during_block = fs::read(&wav_path).unwrap();
        assert!(
            wav_during_block.len() >= 44,
            "B WAV header missing during A inspection"
        );
        assert_eq!(&wav_during_block[44..], expected_blocked_payload);
        assert_eq!(
            io.active_identity,
            Some((capture_b, Generation::first())),
            "B must remain active while A's queue inspection is blocked"
        );

        release_inspection_tx.send(()).unwrap();
        inspection_finished_rx
            .recv_timeout(Duration::from_secs(2))
            .expect("queue inspection did not finish after its explicit release");
        let after_resume = vec![301, 302, 303, 304, 305];
        let acknowledged_after = inject(after_resume.clone());
        let expected_samples_total =
            before_inspection.len() + during_inspection.len() + after_resume.len();
        assert_eq!(acknowledged_after, expected_samples_total as u64);
        assert_eq!(acknowledged_after, 12);
        let mut expected_samples = before_inspection;
        expected_samples.extend(during_inspection);
        expected_samples.extend(after_resume);

        let failure_deadline = Instant::now() + Duration::from_secs(1);
        loop {
            assert!(
                Instant::now() < failure_deadline,
                "A's terminal result was not routed"
            );
            match io.poll().unwrap() {
                Some(ImportEvent::Failed {
                    job_id,
                    generation,
                    message,
                    ..
                }) if job_id == import_a.request.job_id
                    && generation == import_a.request.generation
                    && message.contains("after queue inspection") =>
                {
                    break;
                }
                Some(other) => panic!("unexpected post-inspection event: {other:?}"),
                None => thread::sleep(Duration::from_millis(5)),
            }
        }
        assert_eq!(
            io.active_identity,
            Some((capture_b, Generation::first())),
            "A's delayed queue error must remain associated with A while B is active"
        );

        let stop_started = Instant::now();
        io.submit(ImportEffect::Stop {
            job_id: capture_b,
            generation: Generation::first(),
        })
        .unwrap();
        let stopped_b = wait_for_event(
            &mut io,
            "R5 capture B stop after A inspection resumes",
            Duration::from_millis(1_500),
            |event| {
                matches!(event, ImportEvent::Stopped { job_id, generation }
                    if *job_id == capture_b && *generation == Generation::first())
            },
        );
        assert!(matches!(
            stopped_b,
            ImportEvent::Stopped { job_id, generation }
                if job_id == capture_b && generation == Generation::first()
        ));
        assert!(stop_started.elapsed() < Duration::from_secs(2));
        assert_eq!(gate.0.load(Ordering::Acquire), GATE_CANCELLED);

        let bytes = fs::read(&wav_path).unwrap();
        assert!(bytes.len() >= 44, "finalized B WAV header is missing");
        assert_eq!(&bytes[..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        assert_eq!(&bytes[36..40], b"data");
        assert_eq!(
            u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as usize,
            bytes.len() - 8,
            "finalized B RIFF size must match the complete file"
        );
        let data_size = u32::from_le_bytes(bytes[40..44].try_into().unwrap()) as usize;
        assert_eq!(
            data_size,
            bytes.len() - 44,
            "finalized B data size must match the complete PCM payload"
        );
        assert_eq!(
            data_size % 2,
            0,
            "finalized B PCM16 payload must contain whole samples"
        );
        let actual_samples = bytes[44..]
            .chunks_exact(2)
            .map(|sample| i16::from_le_bytes([sample[0], sample[1]]))
            .collect::<Vec<_>>();
        assert_eq!(actual_samples.len(), expected_samples_total);
        assert_eq!(
            actual_samples, expected_samples,
            "finalized B WAV must preserve exact PCM values and order before, during, and after A's inspection"
        );

        drop(io);
        main_thread.join().unwrap();
        automatic_thread.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn live_automatic_final_queue_corruption_is_reported_explicitly() {
        let root = automatic_final_test_root("r5-corrupt-queue");
        let queue_dir = root.join("transcriptions");
        fs::create_dir_all(&queue_dir).unwrap();
        fs::write(queue_dir.join("queue.jsonl"), b"{ truncated\n").unwrap();

        let error = inspect_automatic_final_queue(&root).unwrap_err();
        assert!(error.starts_with("AutomaticFinalQueueUnavailable: StorageCorrupt:"));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn live_automatic_final_gate_does_not_block_next_session_stop_or_late_publish() {
        let root = automatic_final_test_root("r4-real-publish-isolation");
        fs::create_dir_all(&root).unwrap();
        let archive_root = root.join("archive");
        let capture_b_wav = root.join("capture-b.wav");
        let capture_b_companion = root.join("capture-b.json");
        fs::write(&capture_b_companion, "B companion unchanged").unwrap();
        let capture_b_companion_before = fs::read(&capture_b_companion).unwrap();

        let live_gate = Arc::new(PublicationGate::default());
        let (mut io, effects, stops, app_events, auto_effects, auto_stops, auto_events) =
            AsyncImportIo::for_test(live_gate.clone());
        let (samples_tx, samples_rx) = mpsc::sync_channel(4);
        let (capture_started_tx, capture_started_rx) = mpsc::sync_channel(1);
        let capture_thread = thread::spawn({
            let capture_b_wav = capture_b_wav.clone();
            move || {
                service_loop(
                    effects,
                    stops,
                    app_events,
                    SampleDrivenWavCapture {
                        path: capture_b_wav,
                        file: None,
                        identity: None,
                        samples_rx,
                        started_tx: capture_started_tx,
                        samples: Vec::new(),
                    },
                )
            }
        });

        let capture_b = JobId(0xB1);
        io.submit(ImportEffect::StartDirectLive(direct_live_request(
            capture_b,
        )))
        .unwrap();
        assert_eq!(
            capture_started_rx
                .recv_timeout(Duration::from_secs(1))
                .unwrap(),
            (capture_b, Generation::first())
        );

        let mut acknowledged_samples = 0_u64;
        let mut inject = |samples: Vec<i16>| {
            let block_samples = u64::try_from(samples.len()).unwrap();
            let expected_cumulative = acknowledged_samples + block_samples;
            let (ack_tx, ack_rx) = mpsc::sync_channel(1);
            samples_tx.send((samples.clone(), ack_tx)).unwrap();
            let actual_cumulative = ack_rx
                .recv_timeout(Duration::from_secs(1))
                .unwrap()
                .unwrap();
            assert_eq!(actual_cumulative, expected_cumulative);
            assert!(actual_cumulative > acknowledged_samples);
            acknowledged_samples = actual_cumulative;
        };
        let before_publish = vec![11, 12, 13];
        inject(before_publish.clone());

        let repository_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let model_path = repository_root.join("models/ggml-large-v3-turbo.bin");
        assert!(model_path.is_file(), "registered Turbo fixture is required");
        let gates = io.automatic_publication_gates.clone();
        let workers = WorkerPaths {
            cpu: root.join("unused-cpu.exe"),
            gpu: root.join("unused-gpu.exe"),
        };
        let runtime_gates = gates.clone();
        let runtime_archive_root = archive_root.clone();
        let (publish_started_tx, publish_started_rx) = mpsc::sync_channel(2);
        let (publication_ready_tx, publication_ready_rx) = mpsc::sync_channel(2);
        let (release_publish_tx, release_publish_rx) = mpsc::sync_channel(2);
        let auto_thread = thread::spawn(move || {
            let mut runtime = ImportRuntime::new(workers, Arc::new(PublicationGate::default()));
            runtime.archive = ArchiveStore::new(&runtime_archive_root);
            runtime.automatic_publication_gates = Some(runtime_gates);
            service_loop(
                auto_effects,
                auto_stops,
                auto_events,
                RuntimeAutomaticPublisher {
                    runtime,
                    gates,
                    started_tx: publish_started_tx,
                    publication_ready_tx,
                    release_rx: release_publish_rx,
                    model_path,
                    source_samples: vec![7, 8, 9, 10],
                },
            )
        });

        let mut import_a = automatic_final_request(JobId(0xA1));
        import_a.request.source_path = root.join("source-a.wav").to_string_lossy().into_owned();
        import_a.request.destination = archive_root.to_string_lossy().into_owned();
        import_a.outputs = whisper_core::ports::AutomaticFinalOutputPaths {
            txt_path: root
                .join("capture-a-auto.txt")
                .to_string_lossy()
                .into_owned(),
            srt_path: root
                .join("capture-a-auto.srt")
                .to_string_lossy()
                .into_owned(),
            companion_json_path: root.join("capture-a.json").to_string_lossy().into_owned(),
        };
        let preparation_started = Instant::now();
        io.submit(ImportEffect::StartAutomaticFinalTranscription(
            import_a.clone(),
        ))
        .unwrap();
        assert_eq!(
            publish_started_rx
                .recv_timeout(Duration::from_secs(30))
                .unwrap(),
            (import_a.request.job_id, import_a.request.generation)
        );
        assert_eq!(
            io.active_identity,
            Some((capture_b, Generation::first())),
            "capture B must remain active while the real runtime publication is pending"
        );
        let while_publishing = vec![-21, -22, -23, -24];
        inject(while_publishing.clone());
        let prepared_identity = publication_ready_rx
            .recv_timeout(Duration::from_secs(120))
            .unwrap_or_else(|error| {
                panic!(
                    "R4 phase `A archive preparation`; elapsed={:?}; result={error}",
                    preparation_started.elapsed()
                )
            })
            .unwrap_or_else(|error| {
                panic!(
                    "R4 phase `A archive preparation`; elapsed={:?}; error={error}",
                    preparation_started.elapsed()
                )
            });
        assert_eq!(
            prepared_identity,
            (import_a.request.job_id, import_a.request.generation)
        );
        let publication_started = Instant::now();
        release_publish_tx.send(()).unwrap();

        let published_a = wait_for_event(
            &mut io,
            "A final publication",
            Duration::from_secs(30),
            |event| {
                matches!(event, ImportEvent::Published { job_id, generation }
                    if *job_id == import_a.request.job_id && *generation == import_a.request.generation)
            },
        );
        assert!(
            publication_started.elapsed() < Duration::from_secs(30),
            "R4 phase `A final publication`; elapsed={:?}",
            publication_started.elapsed()
        );
        assert!(
            matches!(published_a, ImportEvent::Published { job_id, .. } if job_id == import_a.request.job_id)
        );
        let cleanup_wait_started = Instant::now();
        let cleanup_deadline = cleanup_wait_started + Duration::from_secs(5);
        let cleanup_event = loop {
            assert!(
                Instant::now() < cleanup_deadline,
                "R4 phase `A replacement-cleanup event after Published`; elapsed={:?}; event absent",
                cleanup_wait_started.elapsed()
            );
            match io.poll().unwrap() {
                Some(ImportEvent::ReplacementCleanups(cleanups)) => break cleanups,
                Some(ImportEvent::Queue(_)) => {}
                Some(event) => panic!(
                    "R4 phase `A replacement-cleanup event after Published`; elapsed={:?}; unexpected event: {event:?}",
                    cleanup_wait_started.elapsed()
                ),
                None => thread::sleep(Duration::from_millis(5)),
            }
        };
        let expected_cleanup_successor = whisper_core::ports::TreatmentIdentity {
            destination: archive_root
                .canonicalize()
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            job_id: import_a.request.job_id,
            generation: import_a.request.generation,
        };
        assert!(
            cleanup_event
                .iter()
                .all(|cleanup| cleanup.successor == expected_cleanup_successor),
            "R4 phase `A replacement-cleanup identity`; expected successor {expected_cleanup_successor:?}, got {cleanup_event:?}"
        );
        // `publish` queues Published immediately before ReplacementCleanups in the same
        // runtime. The cleanups payload may be empty; this test has no other publisher.
        let expected_a_text = format!("runtime transcript {}", import_a.request.job_id.0);
        assert_eq!(
            fs::read_to_string(&import_a.outputs.txt_path).unwrap(),
            expected_a_text
        );
        assert!(
            fs::read_to_string(&import_a.outputs.srt_path)
                .unwrap()
                .contains(&expected_a_text)
        );
        assert_eq!(
            io.active_identity,
            Some((capture_b, Generation::first())),
            "publication A must leave capture B identity unchanged"
        );
        assert_eq!(
            fs::read(&capture_b_companion).unwrap(),
            capture_b_companion_before,
            "publication A must leave B's companion untouched"
        );
        assert!(!root.join("capture-b-auto.txt").exists());
        assert!(!root.join("capture-b-auto.srt").exists());

        let mut failing_a = automatic_final_request(JobId(0xA2));
        failing_a.request.source_path = root
            .join("source-a-failing.wav")
            .to_string_lossy()
            .into_owned();
        failing_a.request.destination = archive_root.to_string_lossy().into_owned();
        failing_a.outputs = whisper_core::ports::AutomaticFinalOutputPaths {
            txt_path: root
                .join("capture-a-failing-auto.txt")
                .to_string_lossy()
                .into_owned(),
            srt_path: root
                .join("capture-a-failing-auto.srt")
                .to_string_lossy()
                .into_owned(),
            companion_json_path: root
                .join("capture-a-failing.json")
                .to_string_lossy()
                .into_owned(),
        };
        let sentinel = b"preserve A collision";
        fs::write(&failing_a.outputs.txt_path, sentinel).unwrap();
        let failure_preparation_started = Instant::now();
        io.submit(ImportEffect::StartAutomaticFinalTranscription(
            failing_a.clone(),
        ))
        .unwrap();
        assert_eq!(
            publish_started_rx
                .recv_timeout(Duration::from_secs(30))
                .unwrap(),
            (failing_a.request.job_id, failing_a.request.generation)
        );
        assert_eq!(
            io.active_identity,
            Some((capture_b, Generation::first())),
            "capture B remains active during A's failing publication"
        );
        let prepared_failure_identity = publication_ready_rx
            .recv_timeout(Duration::from_secs(120))
            .unwrap_or_else(|error| {
                panic!(
                    "R4 phase `A failing archive preparation`; elapsed={:?}; result={error}",
                    failure_preparation_started.elapsed()
                )
            })
            .unwrap_or_else(|error| {
                panic!(
                    "R4 phase `A failing archive preparation`; elapsed={:?}; error={error}",
                    failure_preparation_started.elapsed()
                )
            });
        assert_eq!(
            prepared_failure_identity,
            (failing_a.request.job_id, failing_a.request.generation)
        );
        let failure_publication_started = Instant::now();
        release_publish_tx.send(()).unwrap();
        let failed_a = wait_for_event(
            &mut io,
            "A failed final publication",
            Duration::from_secs(30),
            |event| {
                matches!(event, ImportEvent::Failed { job_id, generation, .. }
                    if *job_id == failing_a.request.job_id && *generation == failing_a.request.generation)
            },
        );
        assert!(
            failure_publication_started.elapsed() < Duration::from_secs(30),
            "R4 phase `A failed final publication`; elapsed={:?}",
            failure_publication_started.elapsed()
        );
        assert!(
            matches!(failed_a, ImportEvent::Failed { job_id, message, .. }
            if job_id == failing_a.request.job_id && message.contains("sortie automatique existe déjà"))
        );
        assert_eq!(fs::read(&failing_a.outputs.txt_path).unwrap(), sentinel);
        assert!(!Path::new(&failing_a.outputs.srt_path).exists());
        assert_eq!(
            io.active_identity,
            Some((capture_b, Generation::first())),
            "failure A must not change capture B identity"
        );
        assert_eq!(
            fs::read(&capture_b_companion).unwrap(),
            capture_b_companion_before,
            "failure A must not change B's files"
        );

        let after_publish = vec![31, 32, 33];
        inject(after_publish.clone());
        io.submit(ImportEffect::Stop {
            job_id: capture_b,
            generation: Generation::first(),
        })
        .unwrap();
        let stop_started = Instant::now();
        let stopped_b = wait_for_event(
            &mut io,
            "R4 capture B stop and WAV finalization",
            Duration::from_secs(5),
            |event| {
                matches!(event, ImportEvent::Stopped { job_id, generation }
                    if *job_id == capture_b && *generation == Generation::first())
            },
        );
        assert!(
            matches!(stopped_b, ImportEvent::Stopped { job_id, generation }
                if job_id == capture_b && generation == Generation::first()),
            "R4 expected Stopped for B generation 1; elapsed={:?}; event={stopped_b:?}",
            stop_started.elapsed()
        );
        let capture_b_samples = [before_publish, while_publishing, after_publish].concat();
        let wav = fs::read(&capture_b_wav).unwrap();
        assert!(wav.len() >= 44, "capture B WAV is shorter than its header");
        assert_eq!(&wav[..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        assert_eq!(&wav[36..40], b"data");
        assert_eq!(
            u32::from_le_bytes(wav[4..8].try_into().unwrap()) as usize,
            wav.len() - 8,
            "capture B RIFF size must match the complete file"
        );
        let data_size = u32::from_le_bytes(wav[40..44].try_into().unwrap()) as usize;
        assert_eq!(
            data_size,
            wav.len() - 44,
            "capture B data size must match the complete PCM payload"
        );
        assert_eq!(data_size % 2, 0, "PCM16 payload must contain whole samples");
        let actual_samples = wav[44..]
            .chunks_exact(2)
            .map(|bytes| i16::from_le_bytes([bytes[0], bytes[1]]))
            .collect::<Vec<_>>();
        assert_eq!(
            capture_b_samples.len(),
            10,
            "test fixture must contain 10 samples"
        );
        assert_eq!(
            actual_samples, capture_b_samples,
            "capture B WAV must preserve every injected sample exactly and in order"
        );
        assert_eq!(
            live_gate.0.load(Ordering::Acquire),
            GATE_CANCELLED,
            "stopping B must cancel only B's capture publication gate"
        );

        drop(io);
        capture_thread.join().unwrap();
        auto_thread.join().unwrap();
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn live_automatic_final_failure_is_routed_without_changing_active_capture() {
        let root = automatic_final_test_root("r4-failed-publish-isolation");
        fs::create_dir_all(&root).unwrap();
        let source_txt = root.join("a-source.txt");
        let missing_source_srt = root.join("missing-source.srt");
        fs::write(&source_txt, "partial A input").unwrap();
        let live_gate = Arc::new(PublicationGate::default());
        let (mut io, live_effects, live_stops, _, auto_effects, _, auto_events) =
            AsyncImportIo::for_test(live_gate.clone());
        let mut import_a = automatic_final_request(JobId(0xA3));
        import_a.outputs = whisper_core::ports::AutomaticFinalOutputPaths {
            txt_path: root
                .join("capture-a-auto.txt")
                .to_string_lossy()
                .into_owned(),
            srt_path: root
                .join("capture-a-auto.srt")
                .to_string_lossy()
                .into_owned(),
            companion_json_path: root.join("capture-a.json").to_string_lossy().into_owned(),
        };
        fs::write(
            &import_a.outputs.companion_json_path,
            r#"{"confirmed_intervals":[],"unresolved_intervals":[]}"#,
        )
        .unwrap();
        io.submit(ImportEffect::StartAutomaticFinalTranscription(
            import_a.clone(),
        ))
        .unwrap();
        let _ = auto_effects.try_recv().unwrap();
        let capture_b = JobId(0xB2);
        io.submit(ImportEffect::StartDirectLive(direct_live_request(
            capture_b,
        )))
        .unwrap();
        let _ = live_effects.try_recv().unwrap();

        let publication_error =
            publish_automatic_final_exports(&source_txt, &missing_source_srt, &import_a)
                .unwrap_err();
        assert!(publication_error.contains("Export SRT automatique absent"));
        assert!(!Path::new(&import_a.outputs.txt_path).exists());
        assert!(!Path::new(&import_a.outputs.srt_path).exists());

        auto_events
            .send(ImportEvent::Failed {
                job_id: import_a.request.job_id,
                generation: import_a.request.generation,
                message: "simulated automatic-final publication failure".into(),
            })
            .unwrap();
        assert!(matches!(
            io.poll().unwrap(),
            Some(ImportEvent::Failed { job_id, message, .. })
                if job_id == import_a.request.job_id && message.contains("publication failure")
        ));
        assert_eq!(io.active_identity, Some((capture_b, Generation::first())));
        assert_eq!(live_gate.0.load(Ordering::Acquire), GATE_ACTIVE);

        io.submit(ImportEffect::Stop {
            job_id: capture_b,
            generation: Generation::first(),
        })
        .unwrap();
        assert!(
            matches!(live_stops.try_recv(), Ok(ImportEffect::Stop { job_id, .. }) if job_id == capture_b)
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn parakeet_live_automatic_final_exports_are_distinct_create_new_outputs() {
        let root = automatic_final_test_root("exports");
        fs::create_dir_all(&root).unwrap();
        let source_txt = root.join("archive.txt");
        let source_srt = root.join("archive.srt");
        fs::write(&source_txt, "auto non vérifié").unwrap();
        fs::write(&source_srt, "1\n00:00:00,000 --> 00:00:01,000\ntexte\n").unwrap();
        let outputs = whisper_core::ports::AutomaticFinalOutputPaths {
            txt_path: root
                .join("capture-auto-non-verifie.txt")
                .to_string_lossy()
                .into_owned(),
            srt_path: root
                .join("capture-auto-non-verifie.srt")
                .to_string_lossy()
                .into_owned(),
            companion_json_path: root.join("capture.json").to_string_lossy().into_owned(),
        };
        fs::write(
            &outputs.companion_json_path,
            r#"{"confirmed_intervals":[{"text":"direct"}],"unresolved_intervals":[{"status":"unresolved"}]}"#,
        )
        .unwrap();
        let automatic = whisper_core::ports::AutomaticFinalImport {
            request: ImportRequest {
                strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
                job_id: JobId(42),
                generation: Generation::first(),
                source_path: root.join("capture.wav").to_string_lossy().into_owned(),
                source_sha256: Some([1; 32]),
                source_samples: Some(16_000),
                model_path: "model.nemo".into(),
                model_sha256: [2; 32],
                destination: root.to_string_lossy().into_owned(),
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Gpu,
                },
            },
            outputs: outputs.clone(),
            gpu_candidate: None,
        };
        publish_automatic_final_exports(&source_txt, &source_srt, &automatic).unwrap();
        assert_eq!(
            fs::read_to_string(&outputs.txt_path).unwrap(),
            "auto non vérifié"
        );
        assert!(
            fs::read_to_string(&outputs.srt_path)
                .unwrap()
                .starts_with("1\n")
        );
        let manifest: serde_json::Value =
            serde_json::from_slice(&fs::read(&outputs.companion_json_path).unwrap()).unwrap();
        assert_eq!(
            manifest["automatic_final_transcription"]["status"],
            "produced_unverified"
        );
        assert_eq!(
            manifest["automatic_final_transcription"]["model_sha256"],
            "02".repeat(32)
        );
        assert_eq!(manifest["confirmed_intervals"][0]["text"], "direct");
        assert_eq!(manifest["unresolved_intervals"][0]["status"], "unresolved");
        assert!(publish_automatic_final_exports(&source_txt, &source_srt, &automatic).is_err());
        assert_eq!(
            fs::read_to_string(&outputs.txt_path).unwrap(),
            "auto non vérifié"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn automatic_final_publication_failure_leaves_no_partial_public_name() {
        let root = automatic_final_test_root("rollback");
        fs::create_dir_all(&root).unwrap();
        let txt_stage = root.join("txt.stage");
        let missing_srt_stage = root.join("missing-srt.stage");
        let txt_path = root.join("capture-auto-non-verifie.txt");
        let srt_path = root.join("capture-auto-non-verifie.srt");
        fs::write(&txt_stage, "complete staged text").unwrap();
        let error =
            publish_automatic_staged_pair(&txt_stage, &missing_srt_stage, &txt_path, &srt_path)
                .unwrap_err();
        assert!(error.contains("Publication du SRT automatique impossible"));
        assert!(!txt_path.exists(), "TXT partiel publié malgré échec SRT");
        assert!(!srt_path.exists(), "SRT publié malgré échec de la paire");
        assert!(
            !txt_stage.exists(),
            "fichier de staging restant après rollback"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn automatic_final_source_read_failure_publishes_neither_output() {
        let root = automatic_final_test_root("read-failure");
        fs::create_dir_all(&root).unwrap();
        let source_txt = root.join("source.txt");
        let source_srt = root.join("missing.srt");
        let txt_path = root.join("capture-auto-non-verifie.txt");
        let srt_path = root.join("capture-auto-non-verifie.srt");
        fs::write(&source_txt, "source text").unwrap();
        let automatic = whisper_core::ports::AutomaticFinalImport {
            request: whisper_core::ports::ImportRequest {
                strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
                job_id: JobId(501),
                generation: Generation::first(),
                source_path: "capture.wav".into(),
                source_sha256: None,
                source_samples: None,
                model_path: "model.nemo".into(),
                model_sha256: [5; 32],
                destination: root.to_string_lossy().into_owned(),
                config: whisper_core::domain::JobConfig {
                    language: whisper_core::domain::LanguageChoice::Automatic,
                    compute: whisper_core::domain::ComputeChoice::Gpu,
                },
            },
            outputs: whisper_core::ports::AutomaticFinalOutputPaths {
                txt_path: txt_path.to_string_lossy().into_owned(),
                srt_path: srt_path.to_string_lossy().into_owned(),
                companion_json_path: root.join("capture.json").to_string_lossy().into_owned(),
            },
            gpu_candidate: None,
        };

        let error =
            publish_automatic_final_exports(&source_txt, &source_srt, &automatic).unwrap_err();

        assert!(error.contains("Export SRT automatique absent"));
        assert!(!txt_path.exists());
        assert!(!srt_path.exists());
        fs::remove_dir_all(root).unwrap();
    }
