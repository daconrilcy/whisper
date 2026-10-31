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
