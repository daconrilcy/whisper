    #[test]
    fn parakeet_provisional_never_becomes_confirmed_without_a_confirmed_event() {
        let job_id = JobId(71);
        let generation = Generation::first();
        let session_id = 9;
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.active = Some(CommandIdentity { job_id, generation });
        app.next_sequence = 1;
        app.io.events.push_back(ImportEvent::ParakeetLiveStarted {
            job_id,
            generation,
            session_id,
            backend: "CPU".into(),
        });
        app.io.events.push_back(ImportEvent::ParakeetHypothesis {
            job_id,
            generation,
            session_id,
            sequence: 1,
            range: crate::domain::SourceRange::new(0, 32_000, 16_000).unwrap(),
            text: "texte provisoire".into(),
            captured_at_unix_ms: 1,
            available_at_unix_ms: 2,
        });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.provisional_text.as_deref(), Some("texte provisoire"));
        assert_eq!(view.confirmed_text, None);
        let block = crate::ports::WorkerSegment {
            job_id,
            generation,
            instance_id: session_id,
            segment_id: crate::domain::SegmentId(1),
            range: crate::domain::SourceRange::new(1_000, 8_000, 16_000).unwrap(),
            text: "texte sûr".into(),
        };
        app.io.events.push_back(ImportEvent::ParakeetConfirmed {
            job_id,
            generation,
            session_id: session_id + 1,
            segment: block.clone(),
        });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.confirmed_text, None);
        app.io.events.push_back(ImportEvent::ParakeetConfirmed {
            job_id,
            generation,
            session_id,
            segment: block.clone(),
        });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.confirmed_text.as_deref(), Some("texte sûr"));
        let stopping = app
            .dispatch(AppCommand::Stop { job_id, generation })
            .unwrap();
        assert_eq!(
            stopping.parakeet_captures[0].capture_state,
            crate::ports::ParakeetCaptureViewState::Finalizing
        );
        app.io
            .events
            .push_back(ImportEvent::ParakeetCaptureStopped {
                job_id,
                generation,
                session_id,
                wav_path: "micro.wav".into(),
                txt_path: Some("micro-incomplet.txt".into()),
                srt_path: Some("micro-incomplet.srt".into()),
                complete: false,
                final_segments: vec![block],
                unresolved: vec![crate::domain::SourceRange::new(8_000, 16_000, 16_000).unwrap()],
                diagnostic: Some("une portion audio demeure non confirmée".into()),
                automatic_final: None,
            });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.confirmed_text.as_deref(), Some("texte sûr"));
        assert_eq!(view.active_job, Some((job_id, JobState::Recoverable)));
        assert_eq!(view.parakeet_unresolved.len(), 1);
    }

    #[test]
    fn parakeet_automatic_final_import_stays_unverified_and_separate_from_confirmed_text() {
        let capture_job = JobId(701);
        let import_job = JobId(702);
        let generation = Generation::first();
        let session_id = 33;
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.active = Some(CommandIdentity {
            job_id: capture_job,
            generation,
        });
        let automatic = crate::ports::AutomaticFinalImport {
            request: ImportRequest {
                strategy: crate::parakeet_session::TranscriptionStrategy::LegacyV1,
                job_id: import_job,
                generation,
                source_path: "capture.wav".into(),
                source_sha256: None,
                source_samples: None,
                model_path: "parakeet.model".into(),
                model_sha256: [4; 32],
                destination: "destination".into(),
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Gpu,
                },
            },
            outputs: crate::ports::AutomaticFinalOutputPaths {
                txt_path: "capture-auto-non-verifie.txt".into(),
                srt_path: "capture-auto-non-verifie.srt".into(),
                companion_json_path: "capture.json".into(),
            },
            gpu_candidate: None,
        };
        let confirmed_a = crate::ports::WorkerSegment {
            job_id: capture_job,
            generation,
            instance_id: session_id,
            segment_id: crate::domain::SegmentId(1),
            range: crate::domain::SourceRange::new(0, 400, 16_000).unwrap(),
            text: "bloc live confirmé".into(),
        };
        app.io.events.push_back(ImportEvent::ParakeetLiveStarted {
            job_id: capture_job,
            generation,
            session_id,
            backend: "GPU".into(),
        });
        app.io.events.push_back(ImportEvent::ParakeetConfirmed {
            job_id: capture_job,
            generation,
            session_id,
            segment: confirmed_a.clone(),
        });
        app.io
            .events
            .push_back(ImportEvent::ParakeetCaptureStopped {
                job_id: capture_job,
                generation,
                session_id,
                wav_path: "capture.wav".into(),
                txt_path: Some("capture-incomplet.txt".into()),
                srt_path: Some("capture-incomplet.srt".into()),
                complete: false,
                final_segments: vec![confirmed_a.clone()],
                unresolved: vec![crate::domain::SourceRange::new(0, 800, 16_000).unwrap()],
                diagnostic: None,
                automatic_final: Some(automatic.clone()),
            });
        let pending = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(
            pending.confirmed_text.as_deref(),
            Some("bloc live confirmé")
        );
        assert_eq!(pending.parakeet_unresolved.len(), 1);
        let capture_a = pending
            .parakeet_captures
            .iter()
            .find(|capture| {
                capture.identity.job_id == capture_job && capture.identity.session_id == session_id
            })
            .unwrap();
        assert_eq!(
            capture_a.capture_state,
            crate::ports::ParakeetCaptureViewState::Finalized
        );
        assert_eq!(capture_a.wav_path.as_deref(), Some("capture.wav"));
        assert_eq!(capture_a.confirmed_segments, vec![confirmed_a]);
        assert_eq!(
            capture_a.automatic_final.as_ref().unwrap().status,
            crate::ports::AutomaticFinalTranscriptionStatus::Pending
        );
        assert_eq!(
            pending
                .automatic_final_transcription
                .as_ref()
                .unwrap()
                .status,
            crate::ports::AutomaticFinalTranscriptionStatus::Pending
        );
        assert!(matches!(
            app.io.effects.as_slice(),
            [ImportEffect::StartAutomaticFinalTranscription(_)]
        ));

        let next_capture = JobId(703);
        app.dispatch(AppCommand::StartDirectLiveConfigured {
            request: LiveRequest {
                job_id: next_capture,
                generation,
                model_path: automatic.request.model_path.clone(),
                model_sha256: automatic.request.model_sha256,
                destination: automatic.request.destination.clone(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                microphone_gain_db: 0,
                config: automatic.request.config.clone(),
            },
        })
        .unwrap();
        app.io.events.push_back(ImportEvent::ParakeetLiveStarted {
            job_id: next_capture,
            generation,
            session_id: session_id + 1,
            backend: "CUDA0".into(),
        });
        app.io.events.push_back(ImportEvent::ParakeetHypothesis {
            job_id: next_capture,
            generation,
            session_id: session_id + 1,
            sequence: 1,
            range: crate::domain::SourceRange::new(0, 800, 16_000).unwrap(),
            text: "hypothèse nouvelle session".into(),
            captured_at_unix_ms: 2,
            available_at_unix_ms: 3,
        });
        let next_capture_view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(
            next_capture_view.provisional_text.as_deref(),
            Some("hypothèse nouvelle session")
        );
        assert!(next_capture_view.confirmed_text.is_none());
        assert!(next_capture_view.parakeet_unresolved.is_empty());
        let capture_b = next_capture_view
            .parakeet_captures
            .iter()
            .find(|capture| {
                capture.identity.job_id == next_capture
                    && capture.identity.session_id == session_id + 1
            })
            .unwrap();
        assert_eq!(
            capture_b.capture_state,
            crate::ports::ParakeetCaptureViewState::Capturing
        );
        assert_eq!(capture_b.requested_compute, Some(ComputeChoice::Gpu));
        assert_eq!(capture_b.effective_backend, None);
        assert_eq!(
            capture_b.provisional_text.as_deref(),
            Some("hypothèse nouvelle session")
        );
        assert_eq!(
            app.active,
            Some(CommandIdentity {
                job_id: next_capture,
                generation
            })
        );
        assert_eq!(
            next_capture_view.active_job,
            Some((next_capture, JobState::Capturing))
        );
        assert_eq!(
            app.automatic_final_identity,
            Some(CommandIdentity {
                job_id: import_job,
                generation
            })
        );
        assert_eq!(
            app.automatic_final_state.as_ref().unwrap().request.job_id,
            import_job
        );

        let mut prepared = automatic.request.clone();
        prepared.source_sha256 = Some([7; 32]);
        prepared.source_samples = Some(16_000);
        app.io.events.push_back(ImportEvent::Prepared(prepared));
        app.dispatch(AppCommand::Refresh).unwrap();
        app.io.events.push_back(ImportEvent::Ready {
            job_id: import_job,
            generation,
            backend: "CUDA0".into(),
        });
        let running = app.dispatch(AppCommand::Refresh).unwrap();
        let running_capture_a = running
            .parakeet_captures
            .iter()
            .find(|capture| capture.identity.job_id == capture_job)
            .unwrap();
        let running_final_a = running_capture_a.automatic_final.as_ref().unwrap();
        assert_eq!(
            running_final_a.status,
            crate::ports::AutomaticFinalTranscriptionStatus::Running
        );
        assert_eq!(running_final_a.effective_backend.as_deref(), Some("CUDA0"));
        let running_capture_b = running
            .parakeet_captures
            .iter()
            .find(|capture| capture.identity.job_id == next_capture)
            .unwrap();
        assert_eq!(
            running_capture_b.capture_state,
            crate::ports::ParakeetCaptureViewState::Capturing
        );
        assert_eq!(
            running_capture_b.provisional_text.as_deref(),
            Some("hypothèse nouvelle session")
        );
        assert_eq!(
            running
                .parakeet_captures
                .iter()
                .find(|capture| { capture.identity.job_id == next_capture })
                .unwrap()
                .provisional_text
                .as_deref(),
            Some("hypothèse nouvelle session")
        );
        assert_eq!(
            running.provisional_text.as_deref(),
            Some("hypothèse nouvelle session")
        );
        assert_eq!(
            app.active,
            Some(CommandIdentity {
                job_id: next_capture,
                generation
            })
        );

        app.io.events.push_back(ImportEvent::End {
            job_id: import_job,
            generation,
            last_sequence: 0,
            last_offset: 16_000,
        });
        app.dispatch(AppCommand::Refresh).unwrap();
        assert!(app.io.effects.iter().any(|effect| matches!(
            effect,
            ImportEffect::Publish { job_id, generation: event_generation, last_sequence: 0 }
                if *job_id == import_job && *event_generation == generation
        )));

        app.io.events.push_back(ImportEvent::Published {
            job_id: import_job,
            generation,
        });
        let produced = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(
            produced.provisional_text.as_deref(),
            Some("hypothèse nouvelle session")
        );
        assert!(produced.parakeet_unresolved.is_empty());
        assert_eq!(
            app.active,
            Some(CommandIdentity {
                job_id: next_capture,
                generation
            })
        );
        let capture_a = produced
            .parakeet_captures
            .iter()
            .find(|capture| capture.identity.job_id == capture_job)
            .unwrap();
        assert_eq!(
            capture_a.automatic_final.as_ref().unwrap().status,
            crate::ports::AutomaticFinalTranscriptionStatus::ProducedUnverified
        );
        assert_eq!(
            capture_a
                .automatic_final
                .as_ref()
                .unwrap()
                .effective_backend
                .as_deref(),
            Some("CUDA0")
        );
        let capture_b = produced
            .parakeet_captures
            .iter()
            .find(|capture| capture.identity.job_id == next_capture)
            .unwrap();
        assert_eq!(
            capture_b.capture_state,
            crate::ports::ParakeetCaptureViewState::Capturing
        );
        assert_eq!(
            capture_b.provisional_text.as_deref(),
            Some("hypothèse nouvelle session")
        );
    }

    #[test]
    fn late_automatic_import_failure_updates_only_its_capture_row() {
        let capture_a = JobId(711);
        let import_a = JobId(712);
        let capture_b = JobId(713);
        let generation = Generation::first();
        let session_a = 41;
        let session_b = 42;
        let automatic = crate::ports::AutomaticFinalImport {
            request: ImportRequest {
                strategy: crate::parakeet_session::TranscriptionStrategy::LegacyV1,
                job_id: import_a,
                generation,
                source_path: "a.wav".into(),
                source_sha256: None,
                source_samples: None,
                model_path: "parakeet.model".into(),
                model_sha256: [4; 32],
                destination: "destination".into(),
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Gpu,
                },
            },
            outputs: crate::ports::AutomaticFinalOutputPaths {
                txt_path: "a-auto-non-verifie.txt".into(),
                srt_path: "a-auto-non-verifie.srt".into(),
                companion_json_path: "a.json".into(),
            },
            gpu_candidate: None,
        };
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.active = Some(CommandIdentity {
            job_id: capture_a,
            generation,
        });
        app.io.events.push_back(ImportEvent::ParakeetLiveStarted {
            job_id: capture_a,
            generation,
            session_id: session_a,
            backend: "CUDA0".into(),
        });
        app.io
            .events
            .push_back(ImportEvent::ParakeetCaptureStopped {
                job_id: capture_a,
                generation,
                session_id: session_a,
                wav_path: "a.wav".into(),
                txt_path: Some("a-incomplet.txt".into()),
                srt_path: Some("a-incomplet.srt".into()),
                complete: false,
                final_segments: Vec::new(),
                unresolved: Vec::new(),
                diagnostic: None,
                automatic_final: Some(automatic),
            });
        app.dispatch(AppCommand::Refresh).unwrap();
        app.dispatch(AppCommand::StartDirectLiveConfigured {
            request: LiveRequest {
                job_id: capture_b,
                generation,
                model_path: "parakeet.model".into(),
                model_sha256: [4; 32],
                destination: "destination".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                microphone_gain_db: 0,
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Gpu,
                },
            },
        })
        .unwrap();
        app.io.events.push_back(ImportEvent::ParakeetLiveStarted {
            job_id: capture_b,
            generation,
            session_id: session_b,
            backend: "CUDA0".into(),
        });
        app.io.events.push_back(ImportEvent::ParakeetHypothesis {
            job_id: capture_b,
            generation,
            session_id: session_b,
            sequence: 1,
            range: crate::domain::SourceRange::new(0, 800, 16_000).unwrap(),
            text: "B provisoire".into(),
            captured_at_unix_ms: 2,
            available_at_unix_ms: 3,
        });
        app.dispatch(AppCommand::Refresh).unwrap();
        app.io
            .events
            .push_back(ImportEvent::ParakeetCaptureStopped {
                job_id: capture_b,
                generation,
                session_id: session_b,
                wav_path: "b.wav".into(),
                txt_path: Some("b.txt".into()),
                srt_path: Some("b.srt".into()),
                complete: true,
                final_segments: Vec::new(),
                unresolved: Vec::new(),
                diagnostic: None,
                automatic_final: None,
            });
        let before_late_failure = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(
            before_late_failure.active_job,
            Some((capture_b, JobState::Complete))
        );
        let prior_global_message = before_late_failure.message.clone();
        let row_b_before = before_late_failure
            .parakeet_captures
            .iter()
            .find(|capture| capture.identity.job_id == capture_b)
            .unwrap()
            .clone();
        assert_eq!(row_b_before.provisional_text, None);
        assert_eq!(
            row_b_before.automatic_final.as_ref().unwrap().status,
            crate::ports::AutomaticFinalTranscriptionStatus::FailedOrUnavailable
        );
        let failure = "publication A impossible";
        app.io.events.push_back(ImportEvent::Failed {
            job_id: import_a,
            generation,
            message: failure.into(),
        });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        let row_a = view
            .parakeet_captures
            .iter()
            .find(|capture| capture.identity.job_id == capture_a)
            .unwrap();
        assert_eq!(
            row_a.automatic_final.as_ref().unwrap().status,
            crate::ports::AutomaticFinalTranscriptionStatus::FailedOrUnavailable
        );
        assert_eq!(
            row_a
                .automatic_final
                .as_ref()
                .unwrap()
                .diagnostic
                .as_deref(),
            Some(failure)
        );
        let row_b = view
            .parakeet_captures
            .iter()
            .find(|capture| capture.identity.job_id == capture_b)
            .unwrap();
        assert_eq!(
            row_b.capture_state,
            crate::ports::ParakeetCaptureViewState::Finalized
        );
        assert_eq!(row_b, &row_b_before);
        assert_eq!(row_b.wav_path.as_deref(), Some("b.wav"));
        assert_eq!(row_b.direct_txt_path.as_deref(), Some("b.txt"));
        assert_eq!(app.active, None);
        assert_eq!(view.active_job, Some((capture_b, JobState::Complete)));
        assert_eq!(view.message, prior_global_message);
    }

    #[test]
    fn direct_start_failure_before_live_started_keeps_a_capture_row_without_wav() {
        let capture_a = JobId(721);
        let capture_b = JobId(722);
        let generation = Generation::first();
        let session_a = 51;
        let model_path = "C:/known/parakeet.bin";
        let mut model_sha256 = [0; 32];
        let digest = crate::model_registry::model(crate::model_catalog::ModelId::ParakeetTdt06BV3)
            .artifact
            .sha256;
        for (index, byte) in model_sha256.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&digest[index * 2..index * 2 + 2], 16).unwrap();
        }
        let guard = crate::model_selection_app::ModelCommandGuard::new_with_model_paths(
            crate::model_registry::engine_snapshot(
                1,
                |_| crate::model_catalog::ModelAvailability::Ready,
                Vec::new(),
            ),
            1,
            [(
                crate::model_catalog::ModelId::ParakeetTdt06BV3,
                model_path.into(),
            )],
        );
        let mut app = ImportApplication::new(ScanTestIo::default()).with_model_guard(guard);
        app.active = Some(CommandIdentity {
            job_id: capture_a,
            generation,
        });
        app.io.events.push_back(ImportEvent::ParakeetLiveStarted {
            job_id: capture_a,
            generation,
            session_id: session_a,
            backend: "CUDA0".into(),
        });
        app.io
            .events
            .push_back(ImportEvent::ParakeetCaptureStopped {
                job_id: capture_a,
                generation,
                session_id: session_a,
                wav_path: "a.wav".into(),
                txt_path: Some("a.txt".into()),
                srt_path: Some("a.srt".into()),
                complete: true,
                final_segments: Vec::new(),
                unresolved: Vec::new(),
                diagnostic: None,
                automatic_final: None,
            });
        app.dispatch(AppCommand::Refresh).unwrap();

        app.dispatch(AppCommand::StartDirectLiveConfigured {
            request: LiveRequest {
                job_id: capture_b,
                generation,
                model_path: model_path.into(),
                model_sha256,
                destination: "destination".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                microphone_gain_db: 0,
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Gpu,
                },
            },
        })
        .unwrap();
        let diagnostic = "WorkerExited avant ParakeetLiveStarted";
        app.io.events.push_back(ImportEvent::Failed {
            job_id: capture_b,
            generation,
            message: diagnostic.into(),
        });
        let view = app.dispatch(AppCommand::Refresh).unwrap();

        let row_a = view
            .parakeet_captures
            .iter()
            .find(|capture| capture.identity.job_id == capture_a)
            .unwrap();
        assert_eq!(
            row_a.capture_state,
            crate::ports::ParakeetCaptureViewState::Finalized
        );
        assert_eq!(row_a.wav_path.as_deref(), Some("a.wav"));
        let row_b = view
            .parakeet_captures
            .iter()
            .find(|capture| capture.identity.job_id == capture_b)
            .unwrap();
        assert_eq!(row_b.identity.session_id, 0);
        assert_eq!(
            row_b.capture_state,
            crate::ports::ParakeetCaptureViewState::Failed
        );
        assert_eq!(row_b.capture_diagnostic.as_deref(), Some(diagnostic));
        assert_eq!(row_b.wav_path, None);
        assert_eq!(row_b.direct_txt_path, None);
        assert_eq!(row_b.direct_srt_path, None);
        assert_eq!(view.active_job, Some((capture_b, JobState::Recoverable)));
    }

    #[test]
    fn turbo_start_failure_before_live_started_does_not_create_parakeet_capture() {
        let job_id = JobId(723);
        let generation = Generation::first();
        let model_path = "C:/known/turbo.bin";
        let mut model_sha256 = [0; 32];
        let digest =
            crate::model_registry::model(crate::model_catalog::ModelId::WhisperLargeV3Turbo)
                .artifact
                .sha256;
        for (index, byte) in model_sha256.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&digest[index * 2..index * 2 + 2], 16).unwrap();
        }
        let guard = crate::model_selection_app::ModelCommandGuard::new_with_model_paths(
            crate::model_registry::engine_snapshot(
                1,
                |_| crate::model_catalog::ModelAvailability::Ready,
                Vec::new(),
            ),
            1,
            [(
                crate::model_catalog::ModelId::WhisperLargeV3Turbo,
                model_path.into(),
            )],
        );
        let mut app = ImportApplication::new(ScanTestIo::default()).with_model_guard(guard);
        app.dispatch(AppCommand::StartDirectLiveConfigured {
            request: LiveRequest {
                job_id,
                generation,
                model_path: model_path.into(),
                model_sha256,
                destination: "destination".into(),
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
        app.io.events.push_back(ImportEvent::Failed {
            job_id,
            generation,
            message: "échec du worker Turbo".into(),
        });

        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert!(view.parakeet_captures.is_empty());
        assert_eq!(view.active_job, Some((job_id, JobState::Recoverable)));
    }

    #[test]
    fn parakeet_late_confirmation_keeps_identity_and_reorders_display_chronologically() {
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.view.provisional_session_id = Some(9);
        let job_id = JobId(72);
        let generation = Generation::first();
        app.append_parakeet_confirmed(
            9,
            crate::ports::WorkerSegment {
                job_id,
                generation,
                instance_id: 9,
                segment_id: crate::domain::SegmentId(1),
                range: crate::domain::SourceRange::new(8_000, 12_000, 16_000).unwrap(),
                text: "fonctionne".into(),
            },
        )
        .unwrap();
        app.append_parakeet_confirmed(
            9,
            crate::ports::WorkerSegment {
                job_id,
                generation,
                instance_id: 9,
                segment_id: crate::domain::SegmentId(2),
                range: crate::domain::SourceRange::new(2_000, 6_000, 16_000).unwrap(),
                text: "test".into(),
            },
        )
        .unwrap();
        assert_eq!(app.view.confirmed_text.as_deref(), Some("test\nfonctionne"));
        assert_eq!(app.parakeet_confirmed[0].segment_id.0, 2);
        assert_eq!(app.parakeet_confirmed[1].segment_id.0, 1);
        assert_eq!(app.view.confirmed_samples, 12_000);
    }

    fn unified_capture_state(job_id: JobId) -> UnifiedCaptureAdmission {
        UnifiedCaptureAdmission {
            identity: CommandIdentity {
                job_id,
                generation: Generation::first(),
            },
            destination: "archive".into(),
            stop_submitted: true,
            publish_submitted: false,
            microphone_gain_requested_revision: 0,
            microphone_gain_requested_target_identity: None,
            next_sequence: 1,
            pending_persistence: 0,
            pending_segments: std::collections::VecDeque::new(),
            pending_sequences: std::collections::BTreeSet::new(),
            last_sequence: None,
            total_samples: None,
        }
    }

    #[test]
    fn late_unified_gain_rejection_does_not_rollback_newer_capture_view() {
        let rejected_job = JobId(0xCA);
        let current_job = JobId(0xCB);
        let rejected_identity = CommandIdentity {
            job_id: rejected_job,
            generation: Generation::first(),
        };
        let current_identity = CommandIdentity {
            job_id: current_job,
            generation: Generation::first(),
        };
        let mut app = ImportApplication::new(ScanTestIo::default());
        let mut rejected_capture = unified_capture_state(rejected_job);
        rejected_capture.microphone_gain_requested_revision = 2;
        rejected_capture.microphone_gain_requested_target_identity = Some("mic:A".into());
        let mut current_capture = unified_capture_state(current_job);
        current_capture.microphone_gain_requested_revision = 3;
        current_capture.microphone_gain_requested_target_identity = Some("mic:B".into());
        app.unified_captures
            .insert((rejected_job, Generation::first()), rejected_capture);
        app.unified_captures
            .insert((current_job, Generation::first()), current_capture);
        app.microphone_gain_view_owner = Some(current_identity);
        app.view.microphone_source_identity = Some("mic:B".into());
        app.view.microphone_gain_db = 6;
        app.view.microphone_gain_revision = 2;
        app.view.microphone_gain_requested_db = 12;
        app.view.microphone_gain_requested_revision = 3;
        app.view.message = Some("Gain B demandé".into());

        app.handle_unified_capture_event(ImportEvent::MicrophoneGainRejected {
            job_id: rejected_identity.job_id,
            generation: rejected_identity.generation,
            target_identity: "mic:A".into(),
            message: "source A unavailable".into(),
        })
        .unwrap();

        assert_eq!(
            app.view.microphone_source_identity.as_deref(),
            Some("mic:B")
        );
        assert_eq!(app.view.microphone_gain_db, 6);
        assert_eq!(app.view.microphone_gain_revision, 2);
        assert_eq!(app.view.microphone_gain_requested_db, 12);
        assert_eq!(app.view.microphone_gain_requested_revision, 3);
        assert_eq!(app.view.message.as_deref(), Some("Gain B demandé"));
        assert_eq!(
            app.unified_captures[&(current_job, Generation::first())]
                .microphone_gain_requested_revision,
            3
        );
    }

    fn prepared_unified_capture(job_id: JobId, source_samples: u64) -> ImportRequest {
        ImportRequest {
            strategy: crate::parakeet_session::TranscriptionStrategy::BoundedWindowsV1,
            job_id,
            generation: Generation::first(),
            source_path: "capture.wav".into(),
            source_sha256: Some([7; 32]),
            source_samples: Some(source_samples),
            model_path: "model.nmodel".into(),
            model_sha256: [8; 32],
            destination: "archive".into(),
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Cpu,
            },
        }
    }

    #[test]
    fn unified_capture_ready_records_the_effective_native_backend() {
        let job_id = JobId(0xC0);
        let generation = Generation::first();
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.unified_captures
            .insert((job_id, generation), unified_capture_state(job_id));
        app.add_unified_session_card(
            job_id,
            generation,
            "archive".into(),
            "capture.wav".into(),
            true,
        );

        app.handle_unified_capture_event(ImportEvent::Ready {
            job_id,
            generation,
            backend: "CPU (Auto fallback before audio: no current candidate)".into(),
        })
        .unwrap();

        let session = app
            .view
            .unified_parakeet_sessions
            .iter()
            .find(|session| (session.job_id, session.generation) == (job_id, generation))
            .unwrap();
        assert_eq!(session.phase, crate::ports::UnifiedSessionPhase::Capturing);
        assert_eq!(
            session.message.as_deref(),
            Some("Moteur actif : CPU (Auto fallback before audio: no current candidate)")
        );
    }

    #[test]
    fn unified_capture_prepared_progress_end_publishes_only_after_total_is_known() {
        let job_id = JobId(0xC1);
        let generation = Generation::first();
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.unified_captures
            .insert((job_id, generation), unified_capture_state(job_id));

        app.handle_unified_capture_event(ImportEvent::Prepared(prepared_unified_capture(
            job_id, 32_000,
        )))
        .unwrap();
        app.handle_unified_capture_event(ImportEvent::Progress {
            job_id,
            generation,
            completed_samples: 32_000,
            total_samples: 32_000,
        })
        .unwrap();
        app.handle_unified_capture_event(ImportEvent::End {
            job_id,
            generation,
            last_sequence: 0,
            last_offset: 32_000,
        })
        .unwrap();

        assert!(matches!(
            app.io.effects.last(),
            Some(ImportEffect::Publish { job_id: actual, generation: actual_generation, last_sequence: 0 })
                if *actual == job_id && *actual_generation == generation
        ));
    }

    #[test]
    fn unified_capture_rejects_progress_with_a_total_different_from_prepared_source() {
        let job_id = JobId(0xC2);
        let generation = Generation::first();
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.unified_captures
            .insert((job_id, generation), unified_capture_state(job_id));
        app.handle_unified_capture_event(ImportEvent::Prepared(prepared_unified_capture(
            job_id, 32_000,
        )))
        .unwrap();

        assert_eq!(
            app.handle_unified_capture_event(ImportEvent::Progress {
                job_id,
                generation,
                completed_samples: 31_999,
                total_samples: 31_999,
            }),
            Err(ApplicationError::InvalidCommand)
        );
        assert!(app.io.effects.is_empty());
    }

    #[test]
    fn unified_capture_rejects_end_until_prepared_total_arrives() {
        let job_id = JobId(0xC3);
        let generation = Generation::first();
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.unified_captures
            .insert((job_id, generation), unified_capture_state(job_id));

        assert_eq!(
            app.handle_unified_capture_event(ImportEvent::End {
                job_id,
                generation,
                last_sequence: 0,
                last_offset: 32_000,
            }),
            Err(ApplicationError::InvalidCommand)
        );
        assert!(app.io.effects.is_empty());
    }
