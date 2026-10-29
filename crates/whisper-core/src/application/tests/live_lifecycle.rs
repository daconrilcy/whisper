    #[test]
    fn repeated_start_is_identifiable_and_stale_stop_does_not_target_new_generation() {
        let job_id = JobId(7);
        let current = Generation::first().next().unwrap();
        let stale = Generation::first();
        let config = JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        };
        let start = AppCommand::StartLive {
            job_id,
            generation: current,
            config,
        };
        let repeated_start = start.clone();
        let stale_stop = AppCommand::Stop {
            job_id,
            generation: stale,
        };

        assert_eq!(start, repeated_start);
        assert!(start.targets(job_id, current));
        assert!(!stale_stop.targets(job_id, current));
    }

    #[test]
    fn every_job_lifecycle_command_carries_identity() {
        let job_id = JobId(9);
        let generation = Generation::first();
        let config = JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        };
        let commands = [
            AppCommand::StartImport {
                request: ImportRequest {
                    strategy: crate::parakeet_session::TranscriptionStrategy::LegacyV1,
                    job_id,
                    generation,
                    source_path: "source.wav".into(),
                    source_sha256: None,
                    source_samples: None,
                    model_path: "model.bin".into(),
                    model_sha256: [0; 32],
                    destination: "archive".into(),
                    config: config.clone(),
                },
            },
            AppCommand::ProcessQueued {
                request: ImportRequest {
                    strategy: crate::parakeet_session::TranscriptionStrategy::LegacyV1,
                    job_id,
                    generation,
                    source_path: "source.wav".into(),
                    source_sha256: None,
                    source_samples: None,
                    model_path: "model.bin".into(),
                    model_sha256: [0; 32],
                    destination: "archive".into(),
                    config: config.clone(),
                },
            },
            AppCommand::EnqueueImport {
                request: ImportRequest {
                    strategy: crate::parakeet_session::TranscriptionStrategy::LegacyV1,
                    job_id,
                    generation,
                    source_path: "source.wav".into(),
                    source_sha256: None,
                    source_samples: None,
                    model_path: "model.bin".into(),
                    model_sha256: [0; 32],
                    destination: "archive".into(),
                    config: config.clone(),
                },
            },
            AppCommand::StartLive {
                job_id,
                generation,
                config,
            },
            AppCommand::Stop { job_id, generation },
            AppCommand::Quit { job_id, generation },
        ];
        assert!(commands.iter().all(|command| command.identity().is_some()));
        assert_eq!(AppCommand::Refresh.identity(), None);
    }

    #[test]
    fn live_stop_keeps_generation_until_segments_are_persisted_and_published() {
        let job_id = JobId(10);
        let generation = Generation::first();
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.dispatch(AppCommand::StartLiveConfigured {
            request: LiveRequest {
                job_id,
                generation,
                model_path: "model.bin".into(),
                model_sha256: [1; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                microphone_gain_db: 0,
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            },
        })
        .unwrap();
        app.io.events.push_back(ImportEvent::LiveCounters {
            job_id,
            generation,
            captured_samples: 320,
            admitted_samples: 320,
            audio_durable_samples: 320,
            confirmed_fragment_samples: 0,
            speech_samples: 320,
            diagnostic: None,
        });
        app.dispatch(AppCommand::Refresh).unwrap();
        app.dispatch(AppCommand::Stop { job_id, generation })
            .unwrap();
        assert_eq!(app.active, Some(CommandIdentity { job_id, generation }));

        app.io.events.push_back(ImportEvent::LiveProvisional {
            segment: crate::ports::WorkerSegment {
                job_id,
                generation,
                instance_id: 1,
                segment_id: crate::domain::SegmentId(1),
                range: crate::domain::SourceRange::new(0, 320, 16_000).unwrap(),
                text: "bonjour".into(),
            },
            captured_at_unix_ms: 1,
            available_at_unix_ms: 2,
        });
        app.dispatch(AppCommand::Refresh).unwrap();
        assert!(matches!(
            app.io.effects.last(),
            Some(ImportEffect::PersistSegment(_))
        ));
        app.io.events.push_back(ImportEvent::Persisted {
            job_id,
            generation,
            sequence: 1,
        });
        app.dispatch(AppCommand::Refresh).unwrap();
        app.io
            .events
            .push_back(ImportEvent::Published { job_id, generation });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.active_job, Some((job_id, JobState::Complete)));
        assert!(
            view.last_result
                .as_deref()
                .is_some_and(|text| text.contains("vérifiée"))
        );
    }

    #[test]
    fn final_stop_counters_allow_last_window_and_match_published_view() {
        let job_id = JobId(20);
        let generation = Generation::first();
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.dispatch(AppCommand::StartLiveConfigured {
            request: LiveRequest {
                job_id,
                generation,
                model_path: "model.bin".into(),
                model_sha256: [1; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                microphone_gain_db: 0,
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            },
        })
        .unwrap();
        app.io.events.push_back(ImportEvent::LiveCounters {
            job_id,
            generation,
            captured_samples: 248_640,
            admitted_samples: 248_640,
            audio_durable_samples: 248_640,
            confirmed_fragment_samples: 160_000,
            speech_samples: 0,
            diagnostic: None,
        });
        app.dispatch(AppCommand::Refresh).unwrap();
        app.dispatch(AppCommand::Stop { job_id, generation })
            .unwrap();
        app.io.events.push_back(ImportEvent::LiveCounters {
            job_id,
            generation,
            captured_samples: 254_240,
            admitted_samples: 254_240,
            audio_durable_samples: 254_240,
            confirmed_fragment_samples: 160_000,
            speech_samples: 0,
            diagnostic: None,
        });
        app.io.events.push_back(ImportEvent::WindowFinished {
            job_id,
            generation,
            window_sequence: 4,
            confirmed_samples: 254_240,
        });
        app.io
            .events
            .push_back(ImportEvent::Published { job_id, generation });
        app.dispatch(AppCommand::Refresh).unwrap();
        app.dispatch(AppCommand::Refresh).unwrap();
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.active_job, Some((job_id, JobState::Complete)));
        assert_eq!(view.captured_samples, 254_240);
        assert_eq!(view.admitted_samples, 254_240);
        assert_eq!(view.audio_durable_samples, 254_240);
        assert_eq!(view.confirmed_samples, 254_240);
    }

    #[test]
    fn live_state_advances_through_capture_drain_and_finalization() {
        let job_id = JobId(19);
        let generation = Generation::first();
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.dispatch(AppCommand::StartLiveConfigured {
            request: LiveRequest {
                job_id,
                generation,
                model_path: "model.bin".into(),
                model_sha256: [9; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                microphone_gain_db: 0,
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            },
        })
        .unwrap();
        assert_eq!(app.view.active_job, Some((job_id, JobState::Preparing)));
        app.io.events.push_back(ImportEvent::Ready {
            job_id,
            generation,
            backend: "CPU".into(),
        });
        assert_eq!(
            app.dispatch(AppCommand::Refresh).unwrap().active_job,
            Some((job_id, JobState::Capturing))
        );
        assert_eq!(
            app.dispatch(AppCommand::Stop { job_id, generation })
                .unwrap()
                .active_job,
            Some((job_id, JobState::Draining))
        );
        app.io
            .events
            .push_back(ImportEvent::LiveFinalizing { job_id, generation });
        assert_eq!(
            app.dispatch(AppCommand::Refresh).unwrap().active_job,
            Some((job_id, JobState::Finalizing))
        );
        app.io
            .events
            .push_back(ImportEvent::Published { job_id, generation });
        assert_eq!(
            app.dispatch(AppCommand::Refresh).unwrap().active_job,
            Some((job_id, JobState::Complete))
        );
    }

    #[test]
    fn immediate_live_stop_does_not_reenter_capturing_on_late_ready() {
        let job_id = JobId(23);
        let generation = Generation::first();
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.dispatch(AppCommand::StartLiveConfigured {
            request: LiveRequest {
                job_id,
                generation,
                model_path: "model.bin".into(),
                model_sha256: [11; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                microphone_gain_db: 0,
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            },
        })
        .unwrap();
        app.dispatch(AppCommand::Stop { job_id, generation })
            .unwrap();
        app.io.events.push_back(ImportEvent::Ready {
            job_id,
            generation,
            backend: "CPU".into(),
        });
        assert_eq!(
            app.dispatch(AppCommand::Refresh).unwrap().active_job,
            Some((job_id, JobState::Draining))
        );
        app.io
            .events
            .push_back(ImportEvent::Stopped { job_id, generation });
        assert_eq!(
            app.dispatch(AppCommand::Refresh).unwrap().active_job,
            Some((job_id, JobState::Cancelled))
        );
    }

    #[test]
    fn live_auto_stop_acts_only_when_a_positive_speech_limit_is_configured() {
        for (limit, should_stop) in [(Some(640), true), (None, false), (Some(0), false)] {
            let job_id = JobId(11);
            let generation = Generation::first();
            let mut app = ImportApplication::new(ScanTestIo::default());
            app.dispatch(AppCommand::StartLiveConfigured {
                request: LiveRequest {
                    job_id,
                    generation,
                    model_path: "model.bin".into(),
                    model_sha256: [2; 32],
                    destination: "archive".into(),
                    group_offset_samples: 0,
                    auto_stop_after_speech_samples: limit,
                    microphone: Default::default(),
                    microphone_gain_db: 0,
                    config: JobConfig {
                        language: LanguageChoice::Automatic,
                        compute: ComputeChoice::Cpu,
                    },
                },
            })
            .unwrap();
            app.io.events.push_back(ImportEvent::LiveCounters {
                job_id,
                generation,
                captured_samples: 960,
                admitted_samples: 960,
                audio_durable_samples: 640,
                confirmed_fragment_samples: 0,
                speech_samples: 640,
                diagnostic: None,
            });
            let view = app.dispatch(AppCommand::Refresh).unwrap();
            assert_eq!(
                app.io
                    .effects
                    .iter()
                    .any(|effect| matches!(effect, ImportEffect::Stop { .. })),
                should_stop,
                "limit {limit:?}"
            );
            assert_eq!(
                view.active_job,
                Some((
                    job_id,
                    if should_stop {
                        JobState::Draining
                    } else {
                        JobState::Preparing
                    }
                ))
            );
        }
    }

    #[test]
    fn live_provisional_text_is_exposed_and_persisted_by_the_application() {
        let job_id = JobId(12);
        let generation = Generation::first();
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.dispatch(AppCommand::StartLiveConfigured {
            request: LiveRequest {
                job_id,
                generation,
                model_path: "model.bin".into(),
                model_sha256: [3; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                microphone_gain_db: 0,
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            },
        })
        .unwrap();
        let segment = crate::ports::WorkerSegment {
            job_id,
            generation,
            instance_id: 1,
            segment_id: crate::domain::SegmentId(1),
            range: crate::domain::SourceRange::new(0, 320, 16_000).unwrap(),
            text: "bonjour provisoire".into(),
        };
        app.io.events.push_back(ImportEvent::LiveProvisional {
            segment,
            captured_at_unix_ms: 1_000,
            available_at_unix_ms: 1_020,
        });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.provisional_text.as_deref(), Some("bonjour provisoire"));
        assert_eq!(view.provisional_sequence, Some(1));
        assert_eq!(view.provisional_captured_at_unix_ms, Some(1_000));
        assert_eq!(view.provisional_available_at_unix_ms, Some(1_020));
        assert!(matches!(
            app.io.effects.last(),
            Some(ImportEffect::PersistSegment(_))
        ));
        app.io.events.push_back(ImportEvent::Persisted {
            job_id,
            generation,
            sequence: 1,
        });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.provisional_text, None);
        assert_eq!(view.confirmed_text.as_deref(), Some("bonjour provisoire"));
    }

    #[test]
    fn import_can_be_enqueued_during_live_without_replacing_the_live_job() {
        let job_id = JobId(13);
        let generation = Generation::first();
        let config = JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        };
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.dispatch(AppCommand::StartLiveConfigured {
            request: LiveRequest {
                job_id,
                generation,
                model_path: "model.bin".into(),
                model_sha256: [4; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                microphone_gain_db: 0,
                config: config.clone(),
            },
        })
        .unwrap();
        let import = ImportRequest {
            strategy: crate::parakeet_session::TranscriptionStrategy::LegacyV1,
            job_id: JobId(14),
            generation,
            source_path: "later.wav".into(),
            source_sha256: None,
            source_samples: None,
            model_path: "model.bin".into(),
            model_sha256: [4; 32],
            destination: "archive".into(),
            config,
        };
        app.dispatch(AppCommand::EnqueueImport { request: import })
            .unwrap();
        assert_eq!(app.active, Some(CommandIdentity { job_id, generation }));
        assert!(matches!(
            app.io.effects.last(),
            Some(ImportEffect::Enqueue(_))
        ));
        assert_eq!(app.view.active_job, Some((job_id, JobState::Preparing)));
    }

    #[test]
    fn queued_import_starts_only_after_live_publication_and_explicit_processing() {
        let live_id = JobId(16);
        let generation = Generation::first();
        let config = JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        };
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.dispatch(AppCommand::StartLiveConfigured {
            request: LiveRequest {
                job_id: live_id,
                generation,
                model_path: "model.bin".into(),
                model_sha256: [6; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                microphone_gain_db: 0,
                config: config.clone(),
            },
        })
        .unwrap();
        let queued = ImportRequest {
            strategy: crate::parakeet_session::TranscriptionStrategy::LegacyV1,
            job_id: JobId(17),
            generation,
            source_path: "later.wav".into(),
            source_sha256: None,
            source_samples: None,
            model_path: "model.bin".into(),
            model_sha256: [6; 32],
            destination: "archive".into(),
            config,
        };
        app.dispatch(AppCommand::EnqueueImport {
            request: queued.clone(),
        })
        .unwrap();
        app.dispatch(AppCommand::Stop {
            job_id: live_id,
            generation,
        })
        .unwrap();
        assert!(
            matches!(app.io.effects.last(), Some(ImportEffect::Stop { job_id, .. }) if *job_id == live_id)
        );
        assert_eq!(
            app.active,
            Some(CommandIdentity {
                job_id: live_id,
                generation
            })
        );

        app.io.events.push_back(ImportEvent::Published {
            job_id: live_id,
            generation,
        });
        app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(app.active, None);
        app.dispatch(AppCommand::ProcessQueued { request: queued })
            .unwrap();
        assert_eq!(
            app.active,
            Some(CommandIdentity {
                job_id: JobId(17),
                generation
            })
        );
        assert!(
            matches!(app.io.effects.last(), Some(ImportEffect::ProcessQueued(request)) if request.job_id == JobId(17))
        );
    }

    #[test]
    fn stopped_import_ignores_late_segments_and_remains_cancelled() {
        let job_id = JobId(15);
        let generation = Generation::first();
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.dispatch(AppCommand::StartImport {
            request: ImportRequest {
                strategy: crate::parakeet_session::TranscriptionStrategy::LegacyV1,
                job_id,
                generation,
                source_path: "source.wav".into(),
                source_sha256: None,
                source_samples: None,
                model_path: "model.bin".into(),
                model_sha256: [5; 32],
                destination: "archive".into(),
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            },
        })
        .unwrap();
        app.dispatch(AppCommand::Stop { job_id, generation })
            .unwrap();
        let effects_before_late_segment = app.io.effects.len();
        app.io
            .events
            .push_back(ImportEvent::Segment(crate::ports::WorkerSegment {
                job_id,
                generation,
                instance_id: 1,
                segment_id: crate::domain::SegmentId(1),
                range: crate::domain::SourceRange::new(0, 320, 16_000).unwrap(),
                text: "late".into(),
            }));
        app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(app.io.effects.len(), effects_before_late_segment);
        app.io
            .events
            .push_back(ImportEvent::Stopped { job_id, generation });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.active_job, Some((job_id, JobState::Cancelled)));
    }

    #[test]
    fn direct_capture_and_engine_progress_independently_through_pre_ready_stop() {
        let job_id = JobId(19);
        let generation = Generation::first();
        let mut app = ImportApplication::new(ScanTestIo::default());
        let request = LiveRequest {
            job_id,
            generation,
            model_path: "model.bin".into(),
            model_sha256: [9; 32],
            destination: "archive".into(),
            group_offset_samples: 0,
            auto_stop_after_speech_samples: None,
            microphone: Default::default(),
            microphone_gain_db: 0,
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Cpu,
            },
        };
        let view = app
            .dispatch(AppCommand::StartDirectLiveConfigured { request })
            .unwrap();
        assert_eq!(
            view.direct_capture_phase,
            Some(DirectCapturePhase::Initializing)
        );
        assert_eq!(view.direct_engine_phase, Some(DirectEnginePhase::Preparing));
        app.io.events.push_back(ImportEvent::DirectCaptureActive {
            job_id,
            generation,
            first_sample_at_unix_ms: 100,
            first_admitted_at_unix_ms: 110,
        });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.direct_capture_phase, Some(DirectCapturePhase::Active));
        assert_eq!(view.direct_engine_phase, Some(DirectEnginePhase::Preparing));
        let view = app
            .dispatch(AppCommand::Stop { job_id, generation })
            .unwrap();
        assert_eq!(view.direct_capture_phase, Some(DirectCapturePhase::Stopped));
        assert_eq!(view.direct_engine_phase, Some(DirectEnginePhase::Preparing));
        app.io.events.push_back(ImportEvent::DirectCaptureActive {
            job_id,
            generation,
            first_sample_at_unix_ms: 100,
            first_admitted_at_unix_ms: 110,
        });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.direct_capture_phase, Some(DirectCapturePhase::Stopped));
        assert_eq!(view.direct_first_sample_at_unix_ms, Some(100));
        assert_eq!(view.direct_first_admitted_at_unix_ms, Some(110));
        app.io.events.push_back(ImportEvent::Ready {
            job_id,
            generation,
            backend: "CPU".into(),
        });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.active_job, Some((job_id, JobState::Draining)));
        assert_eq!(view.direct_engine_phase, Some(DirectEnginePhase::Ready));
        app.io.events.push_back(ImportEvent::DirectPublished {
            job_id,
            generation,
            txt_path: "a.txt".into(),
            srt_path: "a.srt".into(),
            wav_path: "a.wav".into(),
        });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.active_job, Some((job_id, JobState::Complete)));
        assert_eq!(view.direct_engine_phase, Some(DirectEnginePhase::Released));
    }

    #[test]
    fn live_capture_failure_remains_recoverable_with_its_diagnostic() {
        let job_id = JobId(16);
        let generation = Generation::first();
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.dispatch(AppCommand::StartLiveConfigured {
            request: LiveRequest {
                job_id,
                generation,
                model_path: "model.bin".into(),
                model_sha256: [6; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                microphone_gain_db: 0,
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            },
        })
        .unwrap();
        let diagnostic = "CaptureSaturated: Stop retained while worker data channel is full";
        app.io.events.push_back(ImportEvent::Failed {
            job_id,
            generation,
            message: diagnostic.into(),
        });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.active_job, Some((job_id, JobState::Recoverable)));
        assert_eq!(view.message.as_deref(), Some(diagnostic));
    }

    #[test]
    fn failed_live_stop_retains_recoverable_diagnostic_and_confirmed_fragment() {
        let job_id = JobId(20);
        let generation = Generation::first();
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.dispatch(AppCommand::StartLiveConfigured {
            request: LiveRequest {
                job_id,
                generation,
                model_path: "model.bin".into(),
                model_sha256: [10; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                microphone_gain_db: 0,
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            },
        })
        .unwrap();
        app.io.events.push_back(ImportEvent::LiveProvisional {
            segment: crate::ports::WorkerSegment {
                job_id,
                generation,
                instance_id: 1,
                segment_id: crate::domain::SegmentId(1),
                range: crate::domain::SourceRange::new(0, 320, 16_000).unwrap(),
                text: "confirmé".into(),
            },
            captured_at_unix_ms: 1,
            available_at_unix_ms: 2,
        });
        app.dispatch(AppCommand::Refresh).unwrap();
        app.io.events.push_back(ImportEvent::Persisted {
            job_id,
            generation,
            sequence: 1,
        });
        app.dispatch(AppCommand::Refresh).unwrap();
        app.dispatch(AppCommand::Stop { job_id, generation })
            .unwrap();
        let diagnostic = "StorageUnavailable: final audio sync failed";
        app.io.events.push_back(ImportEvent::Failed {
            job_id,
            generation,
            message: diagnostic.into(),
        });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.active_job, Some((job_id, JobState::Recoverable)));
        assert_eq!(view.message.as_deref(), Some(diagnostic));
        assert_eq!(view.confirmed_text.as_deref(), Some("confirmé"));
        assert_eq!(view.confirmed_fragment_samples, 320);
    }
