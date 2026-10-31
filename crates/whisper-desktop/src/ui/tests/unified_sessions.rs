#[test]
fn desktop_parakeet_handlers_select_unified_routes_only_for_enabled_candidate() {
    let parakeet = ModelId::ParakeetTdt06BV3;

    let candidate = RecordingApplication::default();
    let mut candidate_app = DesktopApp::new(candidate.clone()).with_unified_candidate_enabled(true);
    candidate_app.selection.model = Some(parakeet);
    candidate_app.paths.audio = "input.wav".into();
    candidate_app.paths.destination = "output".into();
    candidate_app.start_import();
    candidate_app.start_live(Generation::first());
    candidate_app.stop_active();
    assert!(candidate_app.request_quit());
    let candidate_commands = candidate.0.borrow();
    assert!(matches!(
        candidate_commands.first(),
        Some(AppCommand::EnqueueImport { request })
            if request.strategy
                == whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
    ));
    assert!(matches!(
        candidate_commands.get(1),
        Some(AppCommand::StartUnifiedLiveConfigured { request })
            if request.config.language == LanguageChoice::Automatic
                && request.model_sha256
                    == super::parse_sha256(
                        whisper_core::model_registry::model(parakeet).artifact.sha256,
                    )
    ));
    let live_identity = match candidate_commands.get(1) {
        Some(AppCommand::StartUnifiedLiveConfigured { request }) => {
            (request.job_id, request.generation)
        }
        _ => unreachable!("candidate handler must use unified live command"),
    };
    assert!(matches!(
        candidate_commands.get(2),
        Some(AppCommand::Stop { job_id, generation })
            if (*job_id, *generation) == live_identity
    ));
    assert!(matches!(
        candidate_commands.get(3),
        Some(AppCommand::RequestClose)
    ));
    drop(candidate_commands);

    let historical = RecordingApplication::default();
    let mut historical_app = DesktopApp::new(historical.clone());
    historical_app.selection.model = Some(parakeet);
    historical_app.paths.audio = "input.wav".into();
    historical_app.start_import();
    historical_app.start_live(Generation::first());
    let historical_commands = historical.0.borrow();
    assert!(matches!(
        historical_commands.first(),
        Some(AppCommand::EnqueueImport { request })
            if request.strategy
                == whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1
    ));
    assert!(matches!(
        historical_commands.get(1),
        Some(AppCommand::StartDirectLiveConfigured { .. })
    ));
    drop(historical_commands);

    let turbo = RecordingApplication::default();
    let mut turbo_app = DesktopApp::new(turbo.clone()).with_unified_candidate_enabled(true);
    turbo_app.paths.audio = "input.wav".into();
    turbo_app.start_import();
    turbo_app.start_live(Generation::first());
    let turbo_commands = turbo.0.borrow();
    assert!(matches!(
        turbo_commands.first(),
        Some(AppCommand::EnqueueImport { request })
            if request.strategy
                == whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1
    ));
    assert!(matches!(
        turbo_commands.get(1),
        Some(AppCommand::StartDirectLiveConfigured { .. })
    ));
}

#[test]
fn unified_finalizing_capture_keeps_live_button_available_for_second_capture_only() {
    use whisper_core::ports::{UnifiedParakeetSessionView, UnifiedSessionPhase as Phase};

    let application = MicrophoneCycleApplication::default();
    let session = |job_id, phase| UnifiedParakeetSessionView {
        job_id: JobId(job_id),
        generation: Generation::first(),
        microphone_capture: true,
        destination: "output".into(),
        source_path: String::new(),
        phase,
        captured_samples: 16_000,
        durable_samples: 16_000,
        total_samples: Some(16_000),
        document_revision: 1,
        audio_parts_revision: 1,
        wav_path: None,
        txt_path: None,
        srt_path: None,
        message: None,
    };
    {
        let mut view = application.view.borrow_mut();
        view.active_job = Some((JobId(801), whisper_core::JobState::Capturing));
        view.active_generation = Some(Generation::first());
        view.unified_parakeet_sessions
            .push(session(801, Phase::Finalizing));
    }
    let paths = whisper_core::path_settings::PathSettingsApp::new(
        whisper_core::path_settings::PathDefaults {
            audio_directory: String::new(),
            model_path: "model".into(),
            destination: "output".into(),
        },
        Box::new(whisper_core::path_settings::NullPathPort),
    );
    let mut app = DesktopApp::with_microphones(
        application.clone(),
        ComputeChoice::Cpu,
        None,
        paths,
        Box::new(OneMicrophone),
    )
    .with_unified_candidate_enabled(true);
    app.selection.model = Some(ModelId::ParakeetTdt06BV3);
    let context = egui::Context::default();

    run_ui_frame(&context, &mut app, Vec::new());
    assert_eq!(app.live_start_button_enabled, Some(true));
    let start_button = app.live_start_button_rect.expect("live button rendered");
    click_ui_button(&context, &mut app, start_button.center());
    assert_eq!(
        application
            .commands
            .borrow()
            .iter()
            .filter(|command| matches!(command, AppCommand::StartUnifiedLiveConfigured { .. }))
            .count(),
        1,
        "the rendered control admits B while A finalizes"
    );
    assert_eq!(app.view.unified_parakeet_sessions.len(), 2);
    assert!(
        app.view
            .unified_parakeet_sessions
            .iter()
            .any(|session| { session.microphone_capture && session.phase == Phase::Capturing })
    );

    run_ui_frame(&context, &mut app, Vec::new());
    assert_eq!(
        app.live_start_button_enabled,
        Some(false),
        "the rendered control refuses a third admission while B is open"
    );
    let third_button = app.live_start_button_rect.expect("live button rendered");
    click_ui_button(&context, &mut app, third_button.center());
    assert_eq!(
        application
            .commands
            .borrow()
            .iter()
            .filter(|command| matches!(command, AppCommand::StartUnifiedLiveConfigured { .. }))
            .count(),
        1,
        "a disabled third-start control does not submit another capture"
    );
}

#[test]
fn unified_capture_gain_slider_targets_the_current_source_and_revision() {
    use whisper_core::ports::{UnifiedParakeetSessionView, UnifiedSessionPhase as Phase};

    let application = MicrophoneCycleApplication::default();
    application
        .view
        .borrow_mut()
        .unified_parakeet_sessions
        .push(UnifiedParakeetSessionView {
            job_id: JobId(910),
            generation: Generation::first(),
            microphone_capture: true,
            destination: "output".into(),
            source_path: String::new(),
            phase: Phase::Capturing,
            captured_samples: 16_000,
            durable_samples: 16_000,
            total_samples: None,
            document_revision: 1,
            audio_parts_revision: 1,
            wav_path: None,
            txt_path: None,
            srt_path: None,
            message: None,
        });
    {
        let mut view = application.view.borrow_mut();
        view.microphone_source_identity = Some("wasapi:chosen".into());
        view.microphone_gain_requested_db = 3;
        view.microphone_gain_requested_revision = 4;
    }
    let paths = whisper_core::path_settings::PathSettingsApp::new(
        whisper_core::path_settings::PathDefaults {
            audio_directory: String::new(),
            model_path: "model".into(),
            destination: "output".into(),
        },
        Box::new(whisper_core::path_settings::NullPathPort),
    );
    let mut app = DesktopApp::with_microphones(
        application.clone(),
        ComputeChoice::Cpu,
        None,
        paths,
        Box::new(OneMicrophone),
    );
    app.active_identity = Some((JobId(910), Generation::first()));
    let context = egui::Context::default();

    run_ui_frame(&context, &mut app, Vec::new());
    assert_eq!(app.microphone_gain_slider_enabled, Some(true));
    app.set_microphone_gain(9);
    assert!(matches!(
        application.commands.borrow().last(),
        Some(AppCommand::SetMicrophoneGain {
            job_id: JobId(910),
            generation,
            target_identity,
            expected_revision: 4,
            gain,
        }) if *generation == Generation::first()
            && target_identity == "wasapi:chosen"
            && gain.value() == 9
    ));
}

#[test]
fn microphone_labels_number_homonyms_by_sorted_identity() {
    let devices = HomonymousMicrophones.list_inputs().unwrap();
    let expected = [
        ("wasapi:alpha", "Micro de bureau (1)"),
        ("wasapi:beta", "Micro de bureau (2)"),
        ("wasapi:gamma", "Micro de bureau (3)"),
    ];
    for order in [devices.clone(), devices.iter().cloned().rev().collect()] {
        let labels = MicrophoneLabelProjection::new(&order);
        for (identity, label) in expected {
            assert_eq!(labels.device_label(identity), label);
        }
    }
    assert_eq!(
        MicrophoneLabelProjection::new(&devices).device_label("wasapi:missing"),
        "Microphone indisponible"
    );
    assert_eq!(
        MicrophoneLabelProjection::new(&devices).choice_label(&MicrophoneChoice::WindowsDefault),
        "Microphone par défaut de Windows"
    );
    let labels = MicrophoneLabelProjection::new(&devices);
    assert_eq!(
        labels.actual_line("wasapi:alpha"),
        "Microphone utilisé : Micro de bureau (1)"
    );
    assert_eq!(
        labels.planned_line("wasapi:beta"),
        "Microphone prévu : Micro de bureau (2)"
    );
}

#[test]
fn microphone_labels_keep_unique_names_and_hide_missing_names() {
    let devices = vec![
        MicrophoneDevice {
            id: "wasapi:unique".into(),
            name: "Micro de casque".into(),
            unavailable_reason: None,
        },
        MicrophoneDevice {
            id: "wasapi:unnamed".into(),
            name: "  ".into(),
            unavailable_reason: None,
        },
        MicrophoneDevice {
            id: String::new(),
            name: "Nom associé à une identité vide".into(),
            unavailable_reason: None,
        },
        MicrophoneDevice {
            id: "   ".into(),
            name: "Nom associé à une identité blanche".into(),
            unavailable_reason: None,
        },
    ];
    let labels = MicrophoneLabelProjection::new(&devices);
    assert_eq!(labels.device_label("wasapi:unique"), "Micro de casque");
    assert_eq!(labels.device_label("wasapi:unnamed"), "Microphone sans nom");
    assert_eq!(labels.device_label(""), "Microphone indisponible");
    assert_eq!(labels.device_label("   "), "Microphone indisponible");
    assert_eq!(
        microphone_device_label(&devices[2], &devices),
        "Microphone indisponible"
    );
    assert_eq!(
        microphone_device_label(&devices[3], &devices),
        "Microphone indisponible"
    );
    assert_eq!(
        labels.planned_line(""),
        "Microphone prévu : Microphone indisponible"
    );
    assert_eq!(
        labels.actual_line("   "),
        "Microphone utilisé : Microphone indisponible"
    );
    assert!(!labels.device_label("wasapi:missing").contains("wasapi:"));
}

#[test]
fn microphone_labels_selection_keeps_the_full_identity_in_commands() {
    let devices = HomonymousMicrophones.list_inputs().unwrap();
    let paths = whisper_core::path_settings::PathSettingsApp::with_defaults(
        whisper_core::path_settings::PathDefaults {
            audio_directory: String::new(),
            model_path: "model".into(),
            destination: "output".into(),
        },
    );
    let application = RecordingApplication::default();
    let mut app = DesktopApp::with_microphones(
        application.clone(),
        ComputeChoice::Cpu,
        None,
        paths,
        Box::new(HomonymousMicrophones),
    );
    app.microphones
        .select(MicrophoneChoice::Device("wasapi:beta".into()))
        .unwrap();
    assert_eq!(
        MicrophoneLabelProjection::new(&devices).choice_label(app.microphones.effective()),
        "Micro de bureau (2)"
    );

    app.start_microphone_test();
    assert!(matches!(
        application.0.borrow().last(),
        Some(AppCommand::StartMicrophoneTest {
            microphone: MicrophoneChoice::Device(identity),
            ..
        }) if identity == "wasapi:beta"
    ));
}

#[test]
fn rejected_quit_admission_does_not_latch_pending_close() {
    let mut app = DesktopApp::new(RejectQuitApplication);
    app.active_identity = Some((JobId(22), Generation::first()));
    assert!(!app.request_quit());
    assert!(!app.quit_pending);
    assert_eq!(app.view.message.as_deref(), Some("quit admission rejected"));
}

#[test]
fn turbo_completed_capture_allows_unified_parakeet_button_and_dispatch() {
    use whisper_core::{
        ImportApplication, JobState,
        application::{DirectCapturePhase, DirectEnginePhase},
        ports::{ImportEvent, ImportIoPort, PortError},
    };

    #[derive(Clone, Default)]
    struct LifecycleIo(Rc<RefCell<VecDeque<ImportEvent>>>);
    impl ImportIoPort for LifecycleIo {
        fn submit(&mut self, _effect: ImportEffect) -> Result<(), PortError> {
            Ok(())
        }
        fn poll(&mut self) -> Result<Option<ImportEvent>, PortError> {
            Ok(self.0.borrow_mut().pop_front())
        }
    }

    let io = LifecycleIo::default();
    let mut core = ImportApplication::new(io.clone());
    let application = MicrophoneCycleApplication::default();
    let paths = whisper_core::path_settings::PathSettingsApp::new(
        whisper_core::path_settings::PathDefaults {
            audio_directory: String::new(),
            model_path: "model".into(),
            destination: "output".into(),
        },
        Box::new(whisper_core::path_settings::NullPathPort),
    );
    let mut app = DesktopApp::with_microphones(
        application.clone(),
        ComputeChoice::Cpu,
        None,
        paths,
        Box::new(OneMicrophone),
    )
    .with_unified_candidate_enabled(true);
    app.selection.model = Some(ModelId::WhisperLargeV3Turbo);
    let context = egui::Context::default();
    run_ui_frame(&context, &mut app, Vec::new());
    assert_eq!(app.live_start_button_enabled, Some(true));
    let start = app.live_start_button_rect.unwrap().center();
    click_ui_button(&context, &mut app, start);
    let direct = application
        .commands
        .borrow()
        .iter()
        .find_map(|command| match command {
            AppCommand::StartDirectLiveConfigured { request } => Some(request.clone()),
            _ => None,
        })
        .expect("Turbo start routed through UI");
    let (job_id, generation) = (direct.job_id, direct.generation);
    let initializing = core
        .dispatch(AppCommand::StartDirectLiveConfigured { request: direct })
        .unwrap();
    assert_eq!(
        initializing.direct_capture_phase,
        Some(DirectCapturePhase::Initializing)
    );
    io.0.borrow_mut()
        .push_back(ImportEvent::DirectCaptureActive {
            job_id,
            generation,
            first_sample_at_unix_ms: 100,
            first_admitted_at_unix_ms: 110,
        });
    let active = core.dispatch(AppCommand::Refresh).unwrap();
    assert_eq!(
        active.direct_capture_phase,
        Some(DirectCapturePhase::Active)
    );
    *application.view.borrow_mut() = active;
    run_ui_frame(&context, &mut app, Vec::new());
    let stop = app
        .live_stop_button_rect
        .expect("active Turbo stop rendered")
        .center();
    click_ui_button(&context, &mut app, stop);
    assert!(
        application
            .commands
            .borrow()
            .iter()
            .any(|command| matches!(command,
        AppCommand::Stop { job_id: actual, generation: actual_generation }
            if *actual == job_id && *actual_generation == generation))
    );
    let draining = core
        .dispatch(AppCommand::Stop { job_id, generation })
        .unwrap();
    assert_eq!(draining.active_job, Some((job_id, JobState::Draining)));
    app.selection.model = Some(ModelId::ParakeetTdt06BV3);
    *application.view.borrow_mut() = draining;
    run_ui_frame(&context, &mut app, Vec::new());
    assert_eq!(
        app.live_start_button_enabled,
        Some(false),
        "Turbo drain blocks admission"
    );
    io.0.borrow_mut().push_back(ImportEvent::Ready {
        job_id,
        generation,
        backend: "CPU".into(),
    });
    core.dispatch(AppCommand::Refresh).unwrap();
    io.0.borrow_mut().push_back(ImportEvent::DirectPublished {
        job_id,
        generation,
        txt_path: "a.txt".into(),
        srt_path: "a.srt".into(),
        wav_path: "a.wav".into(),
    });
    let completed = core.dispatch(AppCommand::Refresh).unwrap();
    assert_eq!(completed.active_job, Some((job_id, JobState::Complete)));
    assert_eq!(
        completed.direct_capture_phase,
        Some(DirectCapturePhase::Stopped)
    );
    assert_eq!(
        completed.direct_engine_phase,
        Some(DirectEnginePhase::Released)
    );
    *application.view.borrow_mut() = completed;
    app.selection.manual_language = Some("fr".into());
    run_ui_frame(&context, &mut app, Vec::new());
    assert_eq!(
        app.live_start_button_enabled,
        Some(false),
        "Parakeet requires automatic language"
    );
    app.selection.manual_language = None;
    run_ui_frame(&context, &mut app, Vec::new());
    assert_eq!(app.live_start_button_enabled, Some(true));
    let start = app.live_start_button_rect.unwrap().center();
    click_ui_button(&context, &mut app, start);
    assert!(
        application
            .commands
            .borrow()
            .iter()
            .any(|command| matches!(command, AppCommand::StartUnifiedLiveConfigured { .. })),
        "enabled button dispatches unified Parakeet start"
    );
}

#[test]
fn unified_admission_keeps_direct_capture_and_other_busy_guards() {
    use whisper_core::{
        JobState,
        application::{DirectCapturePhase as Capture, DirectEnginePhase as Engine},
    };
    let available = crate::ui::presentation::unified_live_admission_available;
    let mut view = AppView::default();
    assert!(available(&view));
    for phase in [Capture::Initializing, Capture::Active, Capture::Failed] {
        view.direct_capture_phase = Some(phase);
        view.direct_engine_phase = Some(Engine::Released);
        assert!(!available(&view), "{phase:?} stays blocked");
    }
    view.direct_capture_phase = Some(Capture::Stopped);
    for phase in [
        None,
        Some(Engine::Preparing),
        Some(Engine::Ready),
        Some(Engine::Failed),
    ] {
        view.direct_engine_phase = phase;
        assert!(
            !available(&view),
            "stopped without release stays blocked: {phase:?}"
        );
    }
    view.direct_engine_phase = Some(Engine::Released);
    for state in [
        JobState::Capturing,
        JobState::Draining,
        JobState::Finalizing,
    ] {
        view.active_job = Some((JobId(7), state));
        assert!(!available(&view), "busy job stays blocked: {state:?}");
    }
    view.active_job = Some((JobId(7), JobState::Complete));
    assert!(available(&view));
    view.microphone_test_active = true;
    assert!(!available(&view));
    view.microphone_test_active = false;
    view.close_requested = true;
    view.close_ready = false;
    assert!(!available(&view));
}

#[test]
fn stale_audio_page_retries_are_delayed_and_corruption_does_not_retry() {
    use whisper_core::ports::{
        UnifiedAudioPartPage, UnifiedParakeetSelectionView, UnifiedParakeetSessionView,
        UnifiedSessionPhase,
    };
    let application = MicrophoneCycleApplication::default();
    let job_id = JobId(99281);
    let generation = Generation::first();
    {
        let mut view = application.view.borrow_mut();
        view.unified_parakeet_sessions
            .push(UnifiedParakeetSessionView {
                job_id,
                generation,
                microphone_capture: true,
                destination: "output".into(),
                source_path: String::new(),
                phase: UnifiedSessionPhase::Capturing,
                captured_samples: 16_000,
                durable_samples: 16_000,
                total_samples: None,
                document_revision: 0,
                audio_parts_revision: 0,
                wav_path: None,
                txt_path: None,
                srt_path: None,
                message: None,
            });
        view.selected_unified_session = Some(UnifiedParakeetSelectionView {
            job_id,
            generation,
            document_page: None,
            preview: None,
            audio_parts_page: None,
            document_error: None,
            audio_parts_error: Some("StaleResponse: unified audio parts changed".into()),
        });
    }
    let paths = whisper_core::path_settings::PathSettingsApp::new(
        whisper_core::path_settings::PathDefaults {
            audio_directory: String::new(),
            model_path: "model".into(),
            destination: "output".into(),
        },
        Box::new(whisper_core::path_settings::NullPathPort),
    );
    let mut app = DesktopApp::with_microphones(
        application.clone(),
        ComputeChoice::Cpu,
        None,
        paths,
        Box::new(OneMicrophone),
    );
    let context = egui::Context::default();
    let frame = |time: f64, app: &mut DesktopApp<MicrophoneCycleApplication>| {
        let output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 3000.0),
                )),
                time: Some(time),
                ..Default::default()
            },
            |ui| app.show_ui(ui),
        );
        output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text.contains("StorageCorrupt: bad index")))
    };
    let count = || {
        application
            .commands
            .borrow()
            .iter()
            .filter(|command| matches!(command, AppCommand::ReadUnifiedAudioPartsPage { .. }))
            .count()
    };
    frame(0.0, &mut app);
    frame(0.1, &mut app);
    assert_eq!(
        count(),
        0,
        "a stale response must not trigger an immediate retry"
    );
    frame(0.3, &mut app);
    assert_eq!(count(), 1);
    assert!(application.commands.borrow().iter().any(|command| matches!(command,
        AppCommand::ReadUnifiedAudioPartsPage { job_id: actual, generation: actual_generation, expected_revision: 0, .. }
            if *actual == job_id && *actual_generation == generation)));
    frame(0.4, &mut app);
    assert_eq!(count(), 1, "unchanged state is coalesced between retries");
    application
        .view
        .borrow_mut()
        .selected_unified_session
        .as_mut()
        .unwrap()
        .audio_parts_error = Some("StaleResponse: unified audio parts changed during read".into());
    frame(0.6, &mut app);
    assert_eq!(count(), 2, "a second stale read is retried after the delay");
    {
        let mut view = application.view.borrow_mut();
        view.unified_parakeet_sessions[0].phase = UnifiedSessionPhase::Complete;
        view.unified_parakeet_sessions[0].audio_parts_revision = 44;
        let selected = view.selected_unified_session.as_mut().unwrap();
        selected.audio_parts_error = None;
        selected.audio_parts_page = Some(UnifiedAudioPartPage {
            revision: 44,
            offset: 0,
            entries: vec![],
            next_offset: None,
        });
    }
    frame(1.0, &mut app);
    assert_eq!(count(), 2, "stable fresh page ends retries");
    {
        let mut view = application.view.borrow_mut();
        view.selected_unified_session
            .as_mut()
            .unwrap()
            .audio_parts_error = Some("StorageCorrupt: bad index".into());
    }
    frame(2.0, &mut app);
    assert!(
        frame(3.0, &mut app),
        "genuine corruption remains visible even with a previous current page"
    );
    assert_eq!(count(), 2, "genuine corruption is not retried or hidden");
}

#[test]
fn unified_audio_page_stays_visible_at_same_position_while_progress_refreshes() {
    use whisper_core::ports::{
        UnifiedAudioPart, UnifiedAudioPartPage, UnifiedParakeetSelectionView,
        UnifiedParakeetSessionView, UnifiedSessionPhase,
    };
    let application = MicrophoneCycleApplication::default();
    let job_id = JobId(99391);
    let generation = Generation::first();
    {
        let mut view = application.view.borrow_mut();
        view.unified_parakeet_sessions
            .push(UnifiedParakeetSessionView {
                job_id,
                generation,
                microphone_capture: true,
                destination: "output".into(),
                source_path: String::new(),
                phase: UnifiedSessionPhase::Capturing,
                captured_samples: 16_000,
                durable_samples: 16_000,
                total_samples: None,
                document_revision: 0,
                audio_parts_revision: 0,
                wav_path: None,
                txt_path: None,
                srt_path: None,
                message: None,
            });
        view.selected_unified_session = Some(UnifiedParakeetSelectionView {
            job_id,
            generation,
            document_page: None,
            preview: None,
            audio_parts_page: Some(UnifiedAudioPartPage {
                revision: 10,
                offset: 0,
                entries: vec![UnifiedAudioPart {
                    number: 1,
                    start_sample: 0,
                    end_sample: 16_000,
                    path: "archive/source.wav".into(),
                }],
                next_offset: None,
            }),
            document_error: None,
            audio_parts_error: None,
        });
    }
    let paths = whisper_core::path_settings::PathSettingsApp::new(
        whisper_core::path_settings::PathDefaults {
            audio_directory: String::new(),
            model_path: "model".into(),
            destination: "output".into(),
        },
        Box::new(whisper_core::path_settings::NullPathPort),
    );
    let mut app = DesktopApp::with_microphones(
        application.clone(),
        ComputeChoice::Cpu,
        None,
        paths,
        Box::new(OneMicrophone),
    );
    let context = egui::Context::default();
    let mut positions = Vec::new();
    for samples in [16_000, 17_000, 18_000, 19_000] {
        {
            let mut view = application.view.borrow_mut();
            view.unified_parakeet_sessions[0].captured_samples = samples;
            view.unified_parakeet_sessions[0].durable_samples = samples;
        }
        let output = context.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 3000.0),
                )),
                ..Default::default()
            },
            |ui| app.show_ui(ui),
        );
        let mut part = None;
        let mut updating = false;
        for clipped in &output.shapes {
            if let egui::Shape::Text(text) = &clipped.shape {
                if text.galley.job.text.contains("Partie 1") {
                    part = Some(text.pos.y);
                }
                if text
                    .galley
                    .job
                    .text
                    .contains("Actualisation des références audio")
                {
                    updating = true;
                }
            }
        }
        assert!(updating, "older page must be labelled as updating");
        positions.push(part.expect("the old audio part remains visible"));
    }
    assert!(
        positions
            .windows(2)
            .all(|pair| (pair[0] - pair[1]).abs() < 0.1),
        "page position should not oscillate: {positions:?}"
    );
}
