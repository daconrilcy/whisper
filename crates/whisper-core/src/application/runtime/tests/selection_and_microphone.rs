    #[test]
    fn unified_page_responses_and_errors_follow_latest_selection_request() {
        let mut app = ImportApplication::new(ScanTestIo::default());
        let a = (JobId(993), Generation::first());
        let b = (JobId(994), Generation::first());
        app.add_unified_session_card(a.0, a.1, "archive".into(), "a.wav".into(), false);
        app.add_unified_session_card(b.0, b.1, "archive".into(), "b.wav".into(), false);

        app.dispatch(AppCommand::SelectUnifiedSession {
            job_id: a.0,
            generation: a.1,
        })
        .unwrap();
        app.dispatch(AppCommand::ReadUnifiedDocumentPage {
            job_id: a.0,
            generation: a.1,
            expected_revision: 0,
            first_sequence: 1,
        })
        .unwrap();
        let a_doc_old = match app.io.effects.last().unwrap() {
            ImportEffect::ReadUnifiedDocumentPage {
                request_id, limit, ..
            } => {
                assert_eq!(*limit, 128);
                *request_id
            }
            other => panic!("unexpected effect: {other:?}"),
        };
        app.dispatch(AppCommand::ReadUnifiedAudioPartsPage {
            job_id: a.0,
            generation: a.1,
            expected_revision: 0,
            offset: 0,
        })
        .unwrap();
        let a_audio_old = match app.io.effects.last().unwrap() {
            ImportEffect::ReadUnifiedAudioPartsPage {
                request_id, limit, ..
            } => {
                assert_eq!(*limit, 128);
                *request_id
            }
            other => panic!("unexpected effect: {other:?}"),
        };

        app.dispatch(AppCommand::SelectUnifiedSession {
            job_id: b.0,
            generation: b.1,
        })
        .unwrap();
        app.dispatch(AppCommand::ReadUnifiedDocumentPage {
            job_id: b.0,
            generation: b.1,
            expected_revision: 0,
            first_sequence: 1,
        })
        .unwrap();
        let b_doc = match app.io.effects.last().unwrap() {
            ImportEffect::ReadUnifiedDocumentPage { request_id, .. } => *request_id,
            other => panic!("unexpected effect: {other:?}"),
        };
        app.dispatch(AppCommand::ReadUnifiedAudioPartsPage {
            job_id: b.0,
            generation: b.1,
            expected_revision: 0,
            offset: 0,
        })
        .unwrap();
        let b_audio = match app.io.effects.last().unwrap() {
            ImportEffect::ReadUnifiedAudioPartsPage { request_id, .. } => *request_id,
            other => panic!("unexpected effect: {other:?}"),
        };

        app.dispatch(AppCommand::SelectUnifiedSession {
            job_id: a.0,
            generation: a.1,
        })
        .unwrap();
        app.dispatch(AppCommand::ReadUnifiedDocumentPage {
            job_id: a.0,
            generation: a.1,
            expected_revision: 0,
            first_sequence: 1,
        })
        .unwrap();
        let a_doc_new = match app.io.effects.last().unwrap() {
            ImportEffect::ReadUnifiedDocumentPage { request_id, .. } => *request_id,
            other => panic!("unexpected effect: {other:?}"),
        };
        app.dispatch(AppCommand::ReadUnifiedAudioPartsPage {
            job_id: a.0,
            generation: a.1,
            expected_revision: 0,
            offset: 0,
        })
        .unwrap();
        let a_audio_new = match app.io.effects.last().unwrap() {
            ImportEffect::ReadUnifiedAudioPartsPage { request_id, .. } => *request_id,
            other => panic!("unexpected effect: {other:?}"),
        };

        let page = |text: &str| crate::ports::ParakeetDocumentPage {
            revision: 0,
            entries: vec![crate::ports::ParakeetDocumentEntry {
                segment: crate::ports::WorkerSegment {
                    job_id: a.0,
                    generation: a.1,
                    instance_id: 1,
                    segment_id: crate::domain::SegmentId(1),
                    range: crate::domain::SourceRange::new(0, 16_000, 16_000).unwrap(),
                    text: text.into(),
                },
                revision: 0,
            }],
            next_sequence: None,
        };
        let audio_page = |path: &str| crate::ports::UnifiedAudioPartPage {
            revision: 7,
            offset: 0,
            entries: vec![crate::ports::UnifiedAudioPart {
                number: 1,
                start_sample: 0,
                end_sample: 16_000,
                path: path.into(),
            }],
            next_offset: None,
        };
        app.io.events.extend([
            ImportEvent::UnifiedDocumentPage {
                job_id: a.0,
                generation: a.1,
                request_id: a_doc_old,
                result: Err("stale A document error".into()),
            },
            ImportEvent::UnifiedDocumentPage {
                job_id: a.0,
                generation: a.1,
                request_id: a_doc_old,
                result: Ok(page("stale A document success")),
            },
            ImportEvent::UnifiedAudioPartsPage {
                job_id: a.0,
                generation: a.1,
                request_id: a_audio_old,
                result: Err("stale A audio error".into()),
            },
            ImportEvent::UnifiedAudioPartsPage {
                job_id: a.0,
                generation: a.1,
                request_id: a_audio_old,
                result: Ok(audio_page("stale-a.wav")),
            },
            ImportEvent::UnifiedDocumentPage {
                job_id: b.0,
                generation: b.1,
                request_id: b_doc,
                result: Ok(page("stale B")),
            },
            ImportEvent::UnifiedDocumentPage {
                job_id: b.0,
                generation: b.1,
                request_id: b_doc,
                result: Err("stale B document error".into()),
            },
            ImportEvent::UnifiedAudioPartsPage {
                job_id: b.0,
                generation: b.1,
                request_id: b_audio,
                result: Ok(audio_page("stale-b.wav")),
            },
            ImportEvent::UnifiedAudioPartsPage {
                job_id: b.0,
                generation: b.1,
                request_id: b_audio,
                result: Err("stale B audio error".into()),
            },
            ImportEvent::UnifiedDocumentPage {
                job_id: a.0,
                generation: a.1,
                request_id: a_doc_new,
                result: Ok(page("current A")),
            },
            ImportEvent::UnifiedAudioPartsPage {
                job_id: a.0,
                generation: a.1,
                request_id: a_audio_new,
                result: Ok(audio_page("current-a.wav")),
            },
        ]);
        let max_refreshes = app.io.events.len().div_ceil(MAX_EVENTS_PER_REFRESH);
        for _ in 0..max_refreshes {
            app.dispatch(AppCommand::Refresh).unwrap();
            if app.io.events.is_empty() {
                break;
            }
        }
        assert!(
            app.io.events.is_empty(),
            "queued fixture events were not drained"
        );

        let selected = app.view.selected_unified_session.as_ref().unwrap();
        assert_eq!(
            selected.document_page.as_ref().unwrap().entries[0]
                .segment
                .text,
            "current A"
        );
        assert_eq!(
            selected.audio_parts_page.as_ref().unwrap().entries[0].path,
            "current-a.wav"
        );
        assert_eq!(selected.document_error, None);
        assert_eq!(selected.audio_parts_error, None);
    }

    #[test]
    fn unified_session_card_cache_stays_bounded_at_eight() {
        let mut app = ImportApplication::new(ScanTestIo::default());
        for raw_id in 1_000..=1_009 {
            app.add_unified_session_card(
                JobId(raw_id),
                Generation::first(),
                "archive".into(),
                format!("source-{raw_id}.wav"),
                false,
            );
        }
        assert_eq!(
            app.view.unified_parakeet_sessions.len(),
            MAX_PARAKEET_CAPTURE_VIEWS
        );
        assert_eq!(
            app.view
                .unified_parakeet_sessions
                .last()
                .map(|session| session.job_id),
            Some(JobId(1_007)),
            "the ninth active session cannot grow the bounded metadata list"
        );
        assert!(app.view.selected_unified_session.is_some());
    }

    #[test]
    fn microphone_gain_command_is_scoped_to_the_active_direct_capture() {
        let mut app = ImportApplication::new(ScanTestIo::default());
        let job_id = JobId(901);
        let generation = Generation::first();
        app.dispatch(AppCommand::StartDirectLiveConfigured {
            request: LiveRequest {
                job_id,
                generation,
                model_path: "model.nmodel".into(),
                model_sha256: [1; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: crate::microphone::MicrophoneChoice::Device("mic:A".into()),
                microphone_gain_db: 0,
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            },
        })
        .unwrap();

        app.dispatch(AppCommand::SetMicrophoneGain {
            job_id,
            generation,
            target_identity: "mic:A".into(),
            expected_revision: 0,
            gain: crate::microphone_level::GainDb::new(6).unwrap(),
        })
        .unwrap();
        assert!(matches!(
            app.io.effects.last(),
            Some(ImportEffect::SetMicrophoneGain {
                job_id: actual_job,
                generation: actual_generation,
                target_identity,
                expected_revision: 0,
                gain_db,
            }) if *actual_job == job_id
                && *actual_generation == generation
                && target_identity == "mic:A"
                && *gain_db == 6
        ));
        assert_eq!(app.view.microphone_gain_requested_db, 6);
        assert_eq!(app.view.microphone_gain_requested_revision, 1);
        assert_eq!(
            app.view.microphone_gain_db, 0,
            "requested is not yet applied"
        );
        app.dispatch(AppCommand::SetMicrophoneGain {
            job_id,
            generation,
            target_identity: "mic:A".into(),
            expected_revision: 1,
            gain: crate::microphone_level::GainDb::new(9).unwrap(),
        })
        .unwrap();
        assert_eq!(app.view.microphone_gain_requested_db, 9);
        assert_eq!(app.view.microphone_gain_requested_revision, 2);
        assert_eq!(app.view.microphone_gain_revision, 0);
        assert_eq!(
            app.dispatch(AppCommand::SetMicrophoneGain {
                job_id,
                generation,
                target_identity: "mic:A".into(),
                expected_revision: 0,
                gain: crate::microphone_level::GainDb::new(-3).unwrap(),
            }),
            Err(ApplicationError::InvalidCommand)
        );
        app.io
            .events
            .push_back(ImportEvent::MicrophoneGainRejected {
                job_id,
                generation,
                target_identity: "mic:A".into(),
                message: "révision périmée".into(),
            });
        app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(app.active, Some(CommandIdentity { job_id, generation }));
        assert!(
            app.direct_live,
            "gain refusal must not terminate the capture"
        );
        app.io.events.push_back(ImportEvent::MicrophoneGainApplied {
            job_id,
            generation,
            target_identity: "mic:A".into(),
            gain_db: 9,
            gain_revision: 2,
            first_frame: 16_000,
        });
        app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(app.view.microphone_gain_db, 9);
        assert_eq!(app.view.microphone_gain_revision, 2);
        assert_eq!(app.view.microphone_gain_applied_first_frame, Some(16_000));
        assert_eq!(app.view.microphone_gain_requested_db, 9);
        assert_eq!(
            app.dispatch(AppCommand::SetMicrophoneGain {
                job_id,
                generation: generation.next().unwrap(),
                target_identity: "mic:A".into(),
                expected_revision: 2,
                gain: crate::microphone_level::GainDb::NEUTRAL,
            }),
            Err(ApplicationError::InvalidCommand)
        );
        app.dispatch(AppCommand::Stop { job_id, generation })
            .unwrap();
        assert_eq!(
            app.dispatch(AppCommand::SetMicrophoneGain {
                job_id,
                generation,
                target_identity: "mic:A".into(),
                expected_revision: 2,
                gain: crate::microphone_level::GainDb::NEUTRAL,
            }),
            Err(ApplicationError::InvalidCommand)
        );
    }

    #[test]
    fn microphone_test_gain_requested_during_open_survives_started_event() {
        let mut app = ImportApplication::new(ScanTestIo::default());
        let job_id = JobId(904);
        let generation = Generation::first();

        app.dispatch(AppCommand::StartMicrophoneTest {
            job_id,
            generation,
            microphone: crate::microphone::MicrophoneChoice::Device("mic:A".into()),
            gain_db: 0,
        })
        .unwrap();
        app.dispatch(AppCommand::SetMicrophoneGain {
            job_id,
            generation,
            target_identity: "mic:A".into(),
            expected_revision: 0,
            gain: crate::microphone_level::GainDb::new(6).unwrap(),
        })
        .unwrap();

        assert!(matches!(
            app.io.effects.last(),
            Some(ImportEffect::SetMicrophoneGain {
                expected_revision: 0,
                gain_db: 6,
                ..
            })
        ));
        app.io.events.push_back(ImportEvent::MicrophoneTestStarted {
            job_id,
            generation,
            source_identity: "mic:A".into(),
            gain_db: 0,
        });
        app.dispatch(AppCommand::Refresh).unwrap();

        assert_eq!(
            app.view.microphone_source_identity.as_deref(),
            Some("mic:A")
        );
        assert_eq!(app.view.microphone_gain_requested_db, 6);
        assert_eq!(app.view.microphone_gain_requested_revision, 1);
        assert_eq!(app.view.microphone_gain_db, 0);
        assert_eq!(app.view.microphone_gain_revision, 0);

        app.dispatch(AppCommand::SetMicrophoneGain {
            job_id,
            generation,
            target_identity: "mic:A".into(),
            expected_revision: 1,
            gain: crate::microphone_level::GainDb::new(9).unwrap(),
        })
        .unwrap();
        assert!(matches!(
            app.io.effects.last(),
            Some(ImportEffect::SetMicrophoneGain {
                expected_revision: 1,
                gain_db: 9,
                ..
            })
        ));
        assert_eq!(app.view.microphone_gain_requested_db, 9);
        assert_eq!(app.view.microphone_gain_requested_revision, 2);

        app.io.events.push_back(ImportEvent::MicrophoneGainApplied {
            job_id,
            generation,
            target_identity: "mic:A".into(),
            gain_db: 6,
            gain_revision: 1,
            first_frame: 960,
        });
        app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(app.view.microphone_gain_db, 6);
        assert_eq!(app.view.microphone_gain_revision, 1);
        assert_eq!(app.view.microphone_gain_applied_first_frame, Some(960));
        assert_eq!(app.view.microphone_gain_requested_db, 9);
        assert_eq!(app.view.microphone_gain_requested_revision, 2);
    }

    #[test]
    fn microphone_test_owns_one_capture_then_releases_before_live_restart() {
        let mut app = ImportApplication::new(ScanTestIo::default());
        let job_id = JobId(902);
        let generation = Generation::first();
        app.dispatch(AppCommand::StartMicrophoneTest {
            job_id,
            generation,
            microphone: crate::microphone::MicrophoneChoice::Device("mic:A".into()),
            gain_db: 6,
        })
        .unwrap();
        assert!(matches!(
            app.io.effects.last(),
            Some(ImportEffect::StartMicrophoneTest { job_id: actual, gain_db: 6, .. })
                if *actual == job_id
        ));
        let effect_count = app.io.effects.len();
        assert_eq!(
            app.dispatch(AppCommand::StartDirectLiveConfigured {
                request: LiveRequest {
                    job_id,
                    generation: generation.next().unwrap(),
                    model_path: "model.nmodel".into(),
                    model_sha256: [1; 32],
                    destination: "archive".into(),
                    group_offset_samples: 0,
                    auto_stop_after_speech_samples: None,
                    microphone: crate::microphone::MicrophoneChoice::Device("mic:A".into()),
                    microphone_gain_db: 6,
                    config: JobConfig {
                        language: LanguageChoice::Automatic,
                        compute: ComputeChoice::Cpu,
                    },
                },
            }),
            Err(ApplicationError::InvalidCommand)
        );
        assert_eq!(
            app.io.effects.len(),
            effect_count,
            "test owns the only capture"
        );

        app.io.events.push_back(ImportEvent::MicrophoneTestStarted {
            job_id,
            generation,
            source_identity: "mic:A".into(),
            gain_db: 6,
        });
        app.dispatch(AppCommand::Refresh).unwrap();
        assert!(app.view.microphone_test_active);
        assert_eq!(
            app.view.microphone_source_identity.as_deref(),
            Some("mic:A")
        );
        assert_eq!(app.view.microphone_gain_db, 6);

        app.dispatch(AppCommand::StopMicrophoneTest { job_id, generation })
            .unwrap();
        assert!(app.view.microphone_meter.is_none());
        assert!(matches!(
            app.io.effects.last(),
            Some(ImportEffect::StopMicrophoneTest { job_id: actual, .. }) if *actual == job_id
        ));
        app.io
            .events
            .push_back(ImportEvent::MicrophoneTestStopped { job_id, generation });
        app.dispatch(AppCommand::Refresh).unwrap();
        assert!(!app.view.microphone_test_active);
        assert!(app.view.microphone_source_identity.is_none());

        let next_generation = generation.next().unwrap();
        app.dispatch(AppCommand::StartLiveConfigured {
            request: LiveRequest {
                job_id,
                generation: next_generation,
                model_path: "model.nmodel".into(),
                model_sha256: [1; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: crate::microphone::MicrophoneChoice::Device("mic:A".into()),
                microphone_gain_db: 6,
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            },
        })
        .unwrap();
        assert_eq!(app.view.direct_capture_phase, None);
        assert_eq!(
            app.view
                .microphone_acquisition
                .as_ref()
                .map(|context| context.expected_source_identity.as_deref()),
            Some(Some("mic:A"))
        );
        let mut spectrum = crate::microphone_level::MicrophoneSpectrum {
            job_id,
            generation: next_generation,
            source_identity: "mic:A".into(),
            sequence: 1,
            epoch: 0,
            first_frame: 0,
            last_frame: 2048,
            sample_rate_hz: 48_000,
            captured_at_monotonic_ms: crate::microphone_level::monotonic_millis(),
            gain_first_db: 6,
            gain_first_revision: 0,
            gain_last_db: 6,
            gain_last_revision: 0,
            mixed_gain: false,
            dropped_updates: 0,
            pre_gain: vec![crate::microphone_level::SpectrumBin::Silence; 1025],
            post_gain_limiter: vec![crate::microphone_level::SpectrumBin::Silence; 1025],
        };
        spectrum.source_identity = "mic:wrong".into();
        app.io.events.push_back(ImportEvent::MicrophoneSpectrum(
            crate::microphone_level::MicrophoneSpectrumUpdate::Available(spectrum.clone()),
        ));
        app.dispatch(AppCommand::Refresh).unwrap();
        assert!(
            app.view.microphone_spectrum.is_none(),
            "wrong explicit source cannot bind the acquisition"
        );
        spectrum.source_identity = "mic:A".into();
        spectrum.captured_at_monotonic_ms = crate::microphone_level::monotonic_millis();
        app.io.events.push_back(ImportEvent::MicrophoneSpectrum(
            crate::microphone_level::MicrophoneSpectrumUpdate::Available(spectrum.clone()),
        ));
        app.dispatch(AppCommand::Refresh).unwrap();
        assert!(
            app.view.microphone_spectrum.is_some(),
            "the actual source can bind even while direct capture phase is None"
        );
        let retry_barrier = crate::microphone_level::monotonic_millis();
        app.io.events.push_back(ImportEvent::LiveRetryingOnCpu {
            job_id,
            generation: next_generation,
            spectrum_min_epoch: 1,
            spectrum_not_before_monotonic_ms: retry_barrier,
        });
        app.dispatch(AppCommand::Refresh).unwrap();
        assert!(app.view.microphone_spectrum.is_none());
        let mut old_epoch_spectrum = spectrum.clone();
        old_epoch_spectrum.captured_at_monotonic_ms = retry_barrier.saturating_sub(1);
        app.io.events.push_back(ImportEvent::MicrophoneSpectrum(
            crate::microphone_level::MicrophoneSpectrumUpdate::Available(old_epoch_spectrum),
        ));
        app.dispatch(AppCommand::Refresh).unwrap();
        assert!(
            app.view.microphone_spectrum.is_none(),
            "queued pre-retry spectrum for the same job and generation is rejected"
        );
        app.io.stop_committing = true;
        app.dispatch(AppCommand::Stop {
            job_id,
            generation: next_generation,
        })
        .unwrap();
        assert_eq!(app.view.active_job, Some((job_id, JobState::Finalizing)));
        assert!(app.view.microphone_acquisition.is_none());
        assert!(
            app.view.microphone_spectrum.is_none(),
            "Stop clears before asynchronous events are drained"
        );
        app.io.events.push_back(ImportEvent::MicrophoneSpectrum(
            crate::microphone_level::MicrophoneSpectrumUpdate::Available(spectrum),
        ));
        app.dispatch(AppCommand::Refresh).unwrap();
        assert!(
            app.view.microphone_spectrum.is_none(),
            "late spectrum after Stop is ignored"
        );
        app.io.events.push_back(ImportEvent::MicrophoneMeter(
            crate::microphone_level::MicrophoneMeter {
                job_id,
                generation,
                source_identity: "mic:A".into(),
                sequence: 1,
                first_frame: 0,
                last_frame: 4_800,
                sample_rate_hz: 48_000,
                analyzed_channels: 1,
                gain_first_db: 6,
                gain_first_revision: 0,
                gain_last_db: 6,
                gain_last_revision: 0,
                mixed_gain: false,
                pre_gain: crate::microphone_level::MeterLevel {
                    status: crate::microphone_level::MeasurementStatus::Signal,
                    sample_count: 4_800,
                    rms_micros: 100_000,
                    peak_micros: 200_000,
                },
                post_gain_limiter: crate::microphone_level::MeterLevel {
                    status: crate::microphone_level::MeasurementStatus::Signal,
                    sample_count: 4_800,
                    rms_micros: 200_000,
                    peak_micros: 400_000,
                },
                limiter_samples: 0,
                dropped_updates: 0,
                captured_at_monotonic_ms: 10,
            },
        ));
        app.dispatch(AppCommand::Refresh).unwrap();
        assert!(
            app.view.microphone_meter.is_none(),
            "old generation is ignored"
        );
    }

    #[test]
    fn microphone_test_failure_invalidates_its_meter_and_source() {
        let mut app = ImportApplication::new(ScanTestIo::default());
        let job_id = JobId(903);
        let generation = Generation::first();
        app.dispatch(AppCommand::StartMicrophoneTest {
            job_id,
            generation,
            microphone: crate::microphone::MicrophoneChoice::Device("mic:A".into()),
            gain_db: 0,
        })
        .unwrap();
        app.io.events.push_back(ImportEvent::MicrophoneTestStarted {
            job_id,
            generation,
            source_identity: "mic:A".into(),
            gain_db: 0,
        });
        app.dispatch(AppCommand::Refresh).unwrap();
        app.io.events.push_back(ImportEvent::Failed {
            job_id,
            generation,
            message: "flux microphone interrompu".into(),
        });
        app.dispatch(AppCommand::Refresh).unwrap();
        assert!(!app.view.microphone_test_active);
        assert!(app.view.microphone_meter.is_none());
        assert!(app.view.microphone_source_identity.is_none());
        assert!(app.view.message.as_deref().unwrap().contains("arrêté"));
    }

    #[test]
    fn destination_scan_stays_pending_until_history_and_queue_are_consumed() {
        let mut app = ImportApplication::new(ScanTestIo::default());
        let view = app
            .dispatch(AppCommand::ScanHistory {
                destination: "archive".into(),
            })
            .unwrap();
        assert!(view.history_scan_pending);
        assert!(view.queue_scan_pending);

        app.io.events.push_back(ImportEvent::History(Vec::new()));
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert!(!view.history_scan_pending);
        assert!(view.queue_scan_pending);

        app.io.events.push_back(ImportEvent::Queue(Vec::new()));
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert!(!view.history_scan_pending);
        assert!(!view.queue_scan_pending);
    }

    #[test]
    fn queue_scan_failure_clears_pending_state_and_is_visible() {
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.dispatch(AppCommand::ScanHistory {
            destination: "missing-or-invalid".into(),
        })
        .unwrap();
        app.io.events.push_back(ImportEvent::History(Vec::new()));
        app.dispatch(AppCommand::Refresh).unwrap();
        app.io.events.push_back(ImportEvent::Failed {
            job_id: JobId(0),
            generation: Generation::first(),
            message: "StorageCorrupt: destination is not a directory".into(),
        });

        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert!(!view.queue_scan_pending);
        assert!(
            view.message
                .as_deref()
                .is_some_and(|message| message.contains("destination is not a directory"))
        );
    }

    #[test]
    fn cancelling_duplicate_import_releases_only_the_new_source_snapshot() {
        let mut app = ImportApplication::new(ScanTestIo::default());
        let job_id = JobId(101);
        let generation = Generation::first();
        let destination = "archive".to_owned();
        app.view.duplicate_import_prompt = Some(DuplicateImportPrompt {
            request: ImportRequest {
                strategy: crate::parakeet_session::TranscriptionStrategy::LegacyV1,
                job_id,
                generation,
                source_path: "source.wav".into(),
                source_sha256: Some([1; 32]),
                source_samples: Some(16_000),
                model_path: "model.bin".into(),
                model_sha256: [2; 32],
                destination: destination.clone(),
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            },
            source_sha256: "01".repeat(32),
            source_samples: 16_000,
            publications: Vec::new(),
            export_basename: String::new(),
            revalidation_notice: None,
            resume_from: None,
            reserved_export_basename: None,
        });

        let view = app.dispatch(AppCommand::CancelDuplicateImport).unwrap();

        assert!(view.duplicate_import_prompt.is_none());
        assert_eq!(
            view.message.as_deref(),
            Some("Nouvelle transcription annulée.")
        );
        assert_eq!(
            app.io.effects,
            vec![ImportEffect::ReleaseSourceBinding {
                job_id,
                generation,
                destination,
            }]
        );
    }

    #[test]
    fn stale_duplicate_dialog_response_cannot_submit_a_replacement() {
        let mut app = ImportApplication::new(ScanTestIo::default());
        let current = DuplicateImportPrompt {
            request: ImportRequest {
                strategy: crate::parakeet_session::TranscriptionStrategy::LegacyV1,
                job_id: JobId(201),
                generation: Generation::first(),
                source_path: "source.wav".into(),
                source_sha256: Some([1; 32]),
                source_samples: Some(16_000),
                model_path: "model.bin".into(),
                model_sha256: [2; 32],
                destination: "archive".into(),
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            },
            source_sha256: "01".repeat(32),
            source_samples: 16_000,
            publications: Vec::new(),
            export_basename: String::new(),
            revalidation_notice: None,
            resume_from: None,
            reserved_export_basename: None,
        };
        let mut stale = current.clone();
        stale.request.job_id = JobId(202);
        app.view.duplicate_import_prompt = Some(current);

        let result = app.dispatch(AppCommand::ResolveDuplicateImport {
            prompt: Box::new(stale),
            choice: DuplicateImportChoice::ReplaceAll,
        });

        assert!(matches!(result, Err(ApplicationError::InvalidCommand)));
        assert_eq!(app.io.effects, Vec::new());
        assert_eq!(
            app.view
                .duplicate_import_prompt
                .as_ref()
                .unwrap()
                .request
                .job_id,
            JobId(201)
        );
    }

    #[test]
    fn model_guard_blocks_each_import_and_live_effect_route_before_submission() {
        let generation = Generation::first();
        let config = JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        };
        let import = ImportRequest {
            strategy: crate::parakeet_session::TranscriptionStrategy::LegacyV1,
            job_id: JobId(70),
            generation,
            source_path: "source.wav".into(),
            source_sha256: None,
            source_samples: None,
            model_path: "C:/unknown/model.bin".into(),
            model_sha256: [0; 32],
            destination: "archive".into(),
            config: config.clone(),
        };
        let live = LiveRequest {
            job_id: JobId(70),
            generation,
            model_path: import.model_path.clone(),
            model_sha256: [0; 32],
            destination: "archive".into(),
            group_offset_samples: 0,
            auto_stop_after_speech_samples: None,
            microphone: crate::microphone::MicrophoneChoice::WindowsDefault,
            microphone_gain_db: 0,
            config,
        };
        let snapshot = crate::model_registry::engine_snapshot(
            1,
            |_| crate::model_catalog::ModelAvailability::Ready,
            Vec::new(),
        );
        let commands = [
            AppCommand::StartImport {
                request: import.clone(),
            },
            AppCommand::EnqueueImport {
                request: import.clone(),
            },
            AppCommand::ProcessQueued {
                request: import.clone(),
            },
            AppCommand::ResumeInterrupted { request: import },
            AppCommand::StartLiveConfigured {
                request: live.clone(),
            },
            AppCommand::StartDirectLiveConfigured { request: live },
        ];

        for command in commands {
            let mut app = ImportApplication::new(ScanTestIo::default()).with_model_guard(
                crate::model_selection_app::ModelCommandGuard::new(
                    crate::model_registry::engine_snapshot(
                        1,
                        |_| crate::model_catalog::ModelAvailability::Ready,
                        Vec::new(),
                    ),
                    1,
                    ["C:/known/turbo.bin".to_owned()],
                ),
            );
            let view = app.dispatch(command).unwrap();
            assert!(app.io.effects.is_empty());
            assert!(view.message.as_deref().is_some_and(|message| {
                message.contains("Modèle indisponible") && message.contains("non reconnu")
            }));
        }
        drop(snapshot);
    }

    #[test]
    fn turbo_existing_matrix_dispatches_all_1206_requests_to_effect_submission() {
        use crate::{
            model_catalog::{ModelAvailability, ModelId},
            model_registry::{TURBO_LANGUAGE_INPUTS, engine_snapshot, model},
            model_selection_app::{InferencePath, ModelCommandGuard},
        };

        let model_path = r"C:\Users\Cyril\AppData\Local\Programs\WhisperBuildDeps\models\ggml-large-v3-turbo.bin";
        let mut sha256 = [0; 32];
        let digest = model(ModelId::WhisperLargeV3Turbo).artifact.sha256;
        for (index, byte) in sha256.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&digest[index * 2..index * 2 + 2], 16).unwrap();
        }
        let mut submitted = 0usize;

        for compute in [ComputeChoice::Cpu, ComputeChoice::Gpu, ComputeChoice::Auto] {
            for path in [InferencePath::Import, InferencePath::Live] {
                let mut languages = vec![LanguageChoice::Automatic];
                for (code, english_name) in TURBO_LANGUAGE_INPUTS {
                    languages.push(LanguageChoice::Manual(code.into()));
                    languages.push(LanguageChoice::Manual(english_name.into()));
                }
                for language in languages {
                    let generation = Generation::first();
                    let job_id = JobId(submitted as u128);
                    let config = JobConfig { language, compute };
                    let guard = ModelCommandGuard::new(
                        engine_snapshot(1, |_| ModelAvailability::Ready, Vec::new()),
                        1,
                        [model_path.to_owned()],
                    );
                    let mut app =
                        ImportApplication::new(ScanTestIo::default()).with_model_guard(guard);
                    match path {
                        InferencePath::Import => {
                            app.dispatch(AppCommand::StartImport {
                                request: ImportRequest {
                                    strategy:
                                        crate::parakeet_session::TranscriptionStrategy::LegacyV1,
                                    job_id,
                                    generation,
                                    source_path: "source.wav".into(),
                                    source_sha256: None,
                                    source_samples: None,
                                    model_path: model_path.into(),
                                    model_sha256: sha256,
                                    destination: "archive".into(),
                                    config,
                                },
                            })
                            .unwrap();
                            assert!(matches!(
                                app.io.effects.as_slice(),
                                [ImportEffect::Prepare(_)]
                            ));
                        }
                        InferencePath::Live => {
                            app.dispatch(AppCommand::StartLiveConfigured {
                                request: LiveRequest {
                                    job_id,
                                    generation,
                                    model_path: model_path.into(),
                                    model_sha256: sha256,
                                    destination: "archive".into(),
                                    group_offset_samples: 0,
                                    auto_stop_after_speech_samples: None,
                                    microphone: crate::microphone::MicrophoneChoice::WindowsDefault,
                                    microphone_gain_db: 0,
                                    config,
                                },
                            })
                            .unwrap();
                            assert!(matches!(
                                app.io.effects.as_slice(),
                                [ImportEffect::StartLive(_)]
                            ));
                        }
                    }
                    submitted += 1;
                }
            }
        }
        assert_eq!(submitted, 1_206);
    }
