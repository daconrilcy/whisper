    #[test]
    fn provisional_display_removes_only_a_proven_confirmed_prefix() {
        let mut capture = whisper_core::ports::ParakeetCaptureView {
            identity: whisper_core::ports::ParakeetCaptureIdentity {
                job_id: JobId(91),
                generation: Generation::first(),
                session_id: 7,
            },
            wav_path: None,
            direct_txt_path: None,
            direct_srt_path: None,
            companion_json_path: None,
            confirmed_segments: vec![whisper_core::ports::WorkerSegment {
                job_id: JobId(91),
                generation: Generation::first(),
                instance_id: 7,
                segment_id: whisper_core::domain::SegmentId(1),
                range: whisper_core::domain::SourceRange::new(0, 1_000, 16_000).unwrap(),
                text: "Bonjour".into(),
            }],
            provisional_text: None,
            provisional_range: Some(
                whisper_core::domain::SourceRange::new(500, 2_000, 16_000).unwrap(),
            ),
            provisional_sequence: Some(2),
            unresolved: Vec::new(),
            capture_state: whisper_core::ports::ParakeetCaptureViewState::Capturing,
            capture_complete: None,
            capture_diagnostic: None,
            requested_compute: Some(ComputeChoice::Gpu),
            effective_backend: Some("CUDA0".into()),
            automatic_final: None,
            automatic_final_identity: None,
        };

        assert_eq!(
            provisional_without_confirmed_prefix(&capture, "bonjour, monde"),
            Some("monde".into())
        );
        assert_eq!(
            provisional_without_confirmed_prefix(&capture, "bonjour le monde"),
            Some("le monde".into())
        );
        assert_eq!(
            provisional_without_confirmed_prefix(&capture, "salut le monde"),
            None
        );
        capture.provisional_range =
            Some(whisper_core::domain::SourceRange::new(1_000, 2_000, 16_000).unwrap());
        assert_eq!(
            provisional_without_confirmed_prefix(&capture, "nouveau texte"),
            Some("nouveau texte".into())
        );
    }

    #[test]
    fn unified_ui_renders_one_bounded_page_preview_parts_outputs_and_wait_state() {
        let application = MicrophoneCycleApplication::default();
        let identity = (JobId(9_901), Generation::first());
        let final_segment = whisper_core::ports::WorkerSegment {
            job_id: identity.0,
            generation: identity.1,
            instance_id: 1,
            segment_id: whisper_core::domain::SegmentId(1),
            range: whisper_core::domain::SourceRange::new(0, 16_000, 16_000).unwrap(),
            text: "Texte final paginé".into(),
        };
        let preview_segment = whisper_core::ports::WorkerSegment {
            segment_id: whisper_core::domain::SegmentId(2),
            text: "Aperçu remplaçable".into(),
            ..final_segment.clone()
        };
        let mut view = application.view.borrow_mut();
        view.unified_parakeet_sessions
            .push(whisper_core::ports::UnifiedParakeetSessionView {
                job_id: identity.0,
                generation: identity.1,
                microphone_capture: true,
                destination: "archive".into(),
                source_path: "archive/source.wav".into(),
                phase: whisper_core::ports::UnifiedSessionPhase::Capturing,
                captured_samples: 16_000,
                durable_samples: 8_000,
                total_samples: None,
                document_revision: 1,
                audio_parts_revision: 1,
                wav_path: Some("archive/source.wav".into()),
                txt_path: Some("archive/exports/capture.txt".into()),
                srt_path: Some("archive/exports/capture.srt".into()),
                message: None,
            });
        view.selected_unified_session = Some(whisper_core::ports::UnifiedParakeetSelectionView {
            job_id: identity.0,
            generation: identity.1,
            document_page: Some(whisper_core::ports::ParakeetDocumentPage {
                revision: 1,
                entries: vec![whisper_core::ports::ParakeetDocumentEntry {
                    segment: final_segment,
                    revision: 1,
                }],
                next_sequence: None,
            }),
            preview: Some(whisper_core::ports::ParakeetPreview {
                segment: preview_segment,
                revision: 2,
            }),
            audio_parts_page: Some(whisper_core::ports::UnifiedAudioPartPage {
                revision: 1,
                offset: 0,
                entries: vec![whisper_core::ports::UnifiedAudioPart {
                    number: 1,
                    start_sample: 0,
                    end_sample: 16_000,
                    path: "archive/source.wav.part-000000.wav".into(),
                }],
                next_offset: None,
            }),
            document_error: None,
            audio_parts_error: None,
        });
        drop(view);

        let paths = whisper_core::path_settings::PathSettingsApp::new(
            whisper_core::path_settings::PathDefaults {
                audio_directory: "synthetic-audio".into(),
                model_path: "synthetic-model".into(),
                destination: "synthetic-archive".into(),
            },
            Box::new(whisper_core::path_settings::NullPathPort),
        );
        let mut app = DesktopApp::with_paths(application, ComputeChoice::Cpu, None, paths);
        let context = egui::Context::default();
        let output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 2600.0),
                )),
                ..Default::default()
            },
            |ui| app.show_ui(ui),
        );
        let rendered = output
            .shapes
            .into_iter()
            .filter_map(|clipped| match clipped.shape {
                egui::Shape::Text(text) => Some(text.galley.job.text.to_string()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");

        assert!(rendered.contains("Texte final paginé"), "{rendered}");
        assert!(rendered.contains("Aperçu remplaçable"), "{rendered}");
        assert!(
            rendered.contains("source.wav.part-000000.wav"),
            "{rendered}"
        );
        assert!(rendered.contains("capture.txt"), "{rendered}");
        assert!(rendered.contains("capture.srt"), "{rendered}");
        assert!(
            rendered.contains("temps écoulé depuis le dernier progrès"),
            "{rendered}"
        );
    }

    #[test]
    fn unified_ui_requests_only_the_selected_identity_pages() {
        let application = MicrophoneCycleApplication::default();
        let identity = (JobId(9_902), Generation::first());
        {
            let mut view = application.view.borrow_mut();
            view.unified_parakeet_sessions
                .push(whisper_core::ports::UnifiedParakeetSessionView {
                    job_id: identity.0,
                    generation: identity.1,
                    microphone_capture: false,
                    destination: "archive".into(),
                    source_path: "archive/source.wav".into(),
                    phase: whisper_core::ports::UnifiedSessionPhase::Complete,
                    captured_samples: 0,
                    durable_samples: 0,
                    total_samples: Some(16_000),
                    document_revision: 4,
                    audio_parts_revision: 5,
                    wav_path: Some("archive/source.wav".into()),
                    txt_path: None,
                    srt_path: None,
                    message: None,
                });
            view.selected_unified_session =
                Some(whisper_core::ports::UnifiedParakeetSelectionView {
                    job_id: identity.0,
                    generation: identity.1,
                    document_page: None,
                    preview: None,
                    audio_parts_page: None,
                    document_error: None,
                    audio_parts_error: None,
                });
        }
        let paths = whisper_core::path_settings::PathSettingsApp::new(
            whisper_core::path_settings::PathDefaults {
                audio_directory: "synthetic-audio".into(),
                model_path: "synthetic-model".into(),
                destination: "synthetic-archive".into(),
            },
            Box::new(whisper_core::path_settings::NullPathPort),
        );
        let mut app = DesktopApp::with_paths(application.clone(), ComputeChoice::Cpu, None, paths);
        let context = egui::Context::default();
        let _ = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 2600.0),
                )),
                ..Default::default()
            },
            |ui| app.show_ui(ui),
        );

        let commands = application.commands.borrow();
        assert!(commands.iter().any(|command| matches!(
            command,
            AppCommand::ReadUnifiedDocumentPage {
                job_id,
                generation,
                expected_revision: 4,
                first_sequence: 1,
            } if (*job_id, *generation) == identity
        )));
        assert!(commands.iter().any(|command| matches!(
            command,
            AppCommand::ReadUnifiedAudioPartsPage {
                job_id,
                generation,
                expected_revision: 5,
                offset: 0,
            } if (*job_id, *generation) == identity
        )));
    }

    #[test]
    fn unified_wait_tracking_prunes_finished_evicted_sessions_and_keeps_selected_wait() {
        let application = MicrophoneCycleApplication::default();
        let a = (JobId(9_910), Generation::first());
        let b = (JobId(9_911), Generation::first());
        let session = |identity: (JobId, Generation), phase| {
            whisper_core::ports::UnifiedParakeetSessionView {
                job_id: identity.0,
                generation: identity.1,
                microphone_capture: true,
                destination: "synthetic-archive".into(),
                source_path: "synthetic-source.wav".into(),
                phase,
                captured_samples: 1,
                durable_samples: 1,
                total_samples: None,
                document_revision: 0,
                audio_parts_revision: 0,
                wav_path: None,
                txt_path: None,
                srt_path: None,
                message: None,
            }
        };
        {
            let mut view = application.view.borrow_mut();
            view.unified_parakeet_sessions.extend([
                session(a, whisper_core::ports::UnifiedSessionPhase::Capturing),
                session(b, whisper_core::ports::UnifiedSessionPhase::Capturing),
            ]);
            view.selected_unified_session =
                Some(whisper_core::ports::UnifiedParakeetSelectionView {
                    job_id: a.0,
                    generation: a.1,
                    document_page: None,
                    preview: None,
                    audio_parts_page: None,
                    document_error: None,
                    audio_parts_error: None,
                });
        }
        let paths = whisper_core::path_settings::PathSettingsApp::new(
            whisper_core::path_settings::PathDefaults {
                audio_directory: "synthetic-audio".into(),
                model_path: "synthetic-model".into(),
                destination: "synthetic-archive".into(),
            },
            Box::new(whisper_core::path_settings::NullPathPort),
        );
        let mut app = DesktopApp::with_paths(application.clone(), ComputeChoice::Cpu, None, paths);
        let context = egui::Context::default();
        run_ui_frame(&context, &mut app, Vec::new());
        assert!(app.unified_wait_tracking.contains_key(&a));

        application.view.borrow_mut().selected_unified_session =
            Some(whisper_core::ports::UnifiedParakeetSelectionView {
                job_id: b.0,
                generation: b.1,
                document_page: None,
                preview: None,
                audio_parts_page: None,
                document_error: None,
                audio_parts_error: None,
            });
        app.view = application.view.borrow().clone();
        run_ui_frame(&context, &mut app, Vec::new());
        assert!(app.unified_wait_tracking.contains_key(&a));
        let b_started = app.unified_wait_tracking.get(&b).unwrap().3;

        {
            let mut view = application.view.borrow_mut();
            view.unified_parakeet_sessions
                .iter_mut()
                .find(|entry| (entry.job_id, entry.generation) == a)
                .unwrap()
                .phase = whisper_core::ports::UnifiedSessionPhase::Complete;
        }
        app.view = application.view.borrow().clone();
        run_ui_frame(&context, &mut app, Vec::new());
        assert!(!app.unified_wait_tracking.contains_key(&a));

        for raw_id in 9_912..=9_922 {
            let mut view = application.view.borrow_mut();
            if view.unified_parakeet_sessions.len() == 8 {
                let finished = view
                    .unified_parakeet_sessions
                    .iter()
                    .position(|entry| {
                        matches!(
                            entry.phase,
                            whisper_core::ports::UnifiedSessionPhase::Complete
                                | whisper_core::ports::UnifiedSessionPhase::Recoverable
                        )
                    })
                    .expect("a finished card is available for bounded eviction");
                view.unified_parakeet_sessions.remove(finished);
            }
            view.unified_parakeet_sessions.push(session(
                (JobId(raw_id), Generation::first()),
                whisper_core::ports::UnifiedSessionPhase::Complete,
            ));
            app.view = view.clone();
            drop(view);
            run_ui_frame(&context, &mut app, Vec::new());
            assert!(app.unified_wait_tracking.len() <= 8);
            assert!(app.unified_wait_tracking.contains_key(&b));
            assert_eq!(app.unified_wait_tracking.get(&b).unwrap().3, b_started);
        }
        let view = application.view.borrow();
        assert_eq!(view.unified_parakeet_sessions.len(), 8);
        assert!(
            !view
                .unified_parakeet_sessions
                .iter()
                .any(|entry| (entry.job_id, entry.generation) == a)
        );
    }
