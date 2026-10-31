#[test]
fn auto_stop_is_disabled_by_default_and_uses_speech_sample_clock_when_enabled() {
    assert_eq!(speech_limit_samples(false, 120), None);
    assert_eq!(speech_limit_samples(true, 2), Some(1_920_000));
    assert_eq!(speech_limit_samples(true, 0), Some(960_000));
}

#[test]
fn import_enqueue_waits_during_live_capture() {
    assert!(!import_enqueue_enabled(true));
    assert!(import_enqueue_enabled(false));
}

#[test]
fn window_stays_open_through_capture_and_drain() {
    assert!(window_must_remain_open(whisper_core::JobState::Capturing));
    assert!(window_must_remain_open(whisper_core::JobState::Draining));
    assert!(window_must_remain_open(
        whisper_core::JobState::AwaitingChoice
    ));
    assert!(!window_must_remain_open(whisper_core::JobState::Complete));
}

#[test]
fn manual_stop_is_available_while_micro_is_capturing() {
    assert!(manual_stop_available(whisper_core::JobState::Capturing));
    assert!(!manual_stop_available(whisper_core::JobState::Draining));
    assert!(!manual_stop_available(whisper_core::JobState::Complete));
}

#[test]
fn microphone_test_button_stops_after_neutral_gain_ack_and_reopens() {
    let application = MicrophoneCycleApplication::default();
    {
        let mut view = application.view.borrow_mut();
        view.active_job = Some((JobId(501), whisper_core::JobState::Capturing));
        view.active_generation = Some(Generation::first());
        view.direct_capture_phase = Some(whisper_core::application::DirectCapturePhase::Active);
        view.microphone_source_identity = Some("wasapi:chosen".into());
        view.last_result = Some("Résultat précédent conservé".into());
    }
    let paths = whisper_core::path_settings::PathSettingsApp::new(
        whisper_core::path_settings::PathDefaults {
            audio_directory: "audio".into(),
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

    let baseline_shapes = run_ui_frame(&context, &mut app, Vec::new()).shape_count;
    let blocked_start = app
        .microphone_test_button_rect
        .expect("real microphone test button rendered");
    click_ui_button(&context, &mut app, blocked_start.center());
    assert_eq!(
        application
            .commands
            .borrow()
            .iter()
            .filter(|command| matches!(command, AppCommand::StartMicrophoneTest { .. }))
            .count(),
        0,
        "an active capture keeps the real Test button disabled"
    );
    {
        let mut view = application.view.borrow_mut();
        view.active_job = Some((JobId(501), whisper_core::JobState::Complete));
        view.direct_capture_phase = Some(whisper_core::application::DirectCapturePhase::Stopped);
        view.microphone_source_identity = None;
    }
    run_ui_frame(&context, &mut app, Vec::new());
    assert_eq!(
        app.view.last_result.as_deref(),
        Some("Résultat précédent conservé")
    );
    assert!(microphone_test_start_enabled(true, &app.view));
    let start = app
        .microphone_test_button_rect
        .expect("Test button enabled after completed capture");
    click_ui_button(&context, &mut app, start.center());
    assert!(matches!(
        application
            .commands
            .borrow()
            .iter()
            .find(|command| matches!(command, AppCommand::StartMicrophoneTest { .. })),
        Some(AppCommand::StartMicrophoneTest { gain_db: 0, .. })
    ));

    run_ui_frame(&context, &mut app, Vec::new());
    assert!(
        app.view.microphone_test_active,
        "Started keeps Stop visible"
    );
    assert_eq!(
        app.view.microphone_source_identity.as_deref(),
        Some("wasapi:chosen")
    );
    run_ui_frame(&context, &mut app, Vec::new());
    assert!(app.view.microphone_test_active);
    run_ui_frame(&context, &mut app, Vec::new());
    run_ui_frame(&context, &mut app, Vec::new());
    assert!(app.view.microphone_meter.is_some());
    assert!(app.view.microphone_spectrum.is_some());
    let spectrum_frame = run_ui_frame(&context, &mut app, Vec::new());
    assert!(
        spectrum_frame.shape_count > baseline_shapes + 10,
        "live pre/post-gain spectrum adds plotted geometry during capture"
    );
    assert!(
        run_ui_frame_at_size(&context, &mut app, Vec::new(), egui::vec2(640.0, 480.0)).shape_count
            > 10,
        "spectrum remains part of the rendered UI at a small window size"
    );

    assert_eq!(
        app.view.message.as_deref(),
        Some("Gain microphone appliqué : 0 dB à la trame 0")
    );
    assert_eq!(app.view.microphone_gain_applied_first_frame, Some(0));
    assert_eq!(app.view.microphone_gain_revision, 0);

    run_ui_frame(&context, &mut app, Vec::new());
    let meter = app
        .view
        .microphone_meter
        .as_ref()
        .expect("meter event polled");
    assert_eq!(meter.sequence, 1);
    assert_eq!(
        app.view.microphone_gain_db, 0,
        "meter changes without gain change"
    );
    assert!(app.microphone_saturation_badge_rect.is_some());
    assert!(microphone_saturation_is_current(
        app.view.microphone_meter.as_ref(),
        app.active_identity,
        app.view.microphone_source_identity.as_deref(),
        app.view.microphone_test_active
            || app.view.direct_capture_phase.is_some_and(|phase| {
                matches!(
                    phase,
                    whisper_core::application::DirectCapturePhase::Initializing
                        | whisper_core::application::DirectCapturePhase::Active
                )
            }),
        whisper_core::microphone_level::monotonic_millis(),
    ));
    assert!(matches!(
        microphone_meter_presentation(
            app.view.microphone_meter.as_ref(),
            app.active_identity,
            whisper_core::microphone_level::monotonic_millis(),
        ),
        MicrophoneMeterPresentation::Signal { .. }
    ));

    let stop = app
        .microphone_test_button_rect
        .expect("real Stop button rendered after gain acknowledgement");
    click_ui_button(&context, &mut app, stop.center());
    assert!(
        application
            .commands
            .borrow()
            .iter()
            .any(|command| matches!(command, AppCommand::StopMicrophoneTest { .. })),
        "the egui Stop click must route to the capture owner"
    );

    run_ui_frame(&context, &mut app, Vec::new());
    assert!(!app.view.microphone_test_active);
    assert!(
        app.view.microphone_meter.is_none(),
        "stop invalidates the meter"
    );
    assert!(app.view.microphone_source_identity.is_none());
    assert_eq!(
        app.view.last_result.as_deref(),
        Some("Résultat précédent conservé"),
        "testing and stopping preserves the completed transcription"
    );
    assert_eq!(app.view.message.as_deref(), Some("Test microphone arrêté"));

    let reopen = app
        .microphone_test_button_rect
        .expect("test button rendered after release");
    click_ui_button(&context, &mut app, reopen.center());
    assert_eq!(
        application
            .commands
            .borrow()
            .iter()
            .filter(|command| matches!(command, AppCommand::StartMicrophoneTest { .. }))
            .count(),
        2,
        "a released capture can be opened again"
    );
    run_ui_frame(&context, &mut app, Vec::new());
    assert!(app.view.microphone_test_active);

    run_ui_frame(&context, &mut app, Vec::new());
    run_ui_frame(&context, &mut app, Vec::new());
    assert!(app.view.microphone_meter.is_some());
    let (live_job, live_generation) = app.active_identity.expect("active test identity");
    {
        let mut view = application.view.borrow_mut();
        view.microphone_test_active = false;
        view.active_job = Some((live_job, whisper_core::JobState::Capturing));
        view.active_generation = Some(live_generation);
        view.direct_capture_phase = Some(whisper_core::application::DirectCapturePhase::Active);
        view.microphone_source_identity = Some("wasapi:chosen".into());
    }
    run_ui_frame(&context, &mut app, Vec::new());
    assert!(app.microphone_saturation_badge_rect.is_some());
    let direct_stop = app
        .live_stop_button_rect
        .expect("real direct capture Stop button rendered");
    click_ui_button(&context, &mut app, direct_stop.center());
    assert!(application.commands.borrow().iter().any(|command| matches!(
        command,
        AppCommand::Stop { job_id, generation }
            if *job_id == live_job && *generation == live_generation
    )));
    run_ui_frame(&context, &mut app, Vec::new());
    assert_eq!(
        app.view.active_job,
        Some((live_job, whisper_core::JobState::Draining))
    );
    assert_eq!(
        app.view.direct_capture_phase,
        Some(whisper_core::application::DirectCapturePhase::Stopped)
    );
    assert!(
        app.view.microphone_meter.is_some(),
        "stop may retain the last meter"
    );
    assert!(
        app.microphone_saturation_badge_rect.is_none(),
        "the real panel removes the stale saturation warning immediately after direct Stop"
    );
}

#[test]
fn actual_probe_spectra_stay_visible_across_discarded_egui_passes() {
    let spectrum = actual_probe_spectrum();
    assert_eq!(spectrum.pre_gain.len(), 1025);
    assert_eq!(spectrum.post_gain_limiter.len(), 1025);

    let application = MicrophoneCycleApplication::default();
    {
        let mut view = application.view.borrow_mut();
        view.microphone_test_active = true;
        view.microphone_acquisition = Some(
            whisper_core::microphone_level::MicrophoneAcquisitionContext::new(
                JobId(902),
                Generation::first(),
                &MicrophoneChoice::Device("wasapi:chosen".into()),
            ),
        );
        view.active_generation = Some(Generation::first());
        view.microphone_source_identity = Some("wasapi:chosen".into());
        view.microphone_spectrum = Some(spectrum);
    }
    let paths = whisper_core::path_settings::PathSettingsApp::new(
        whisper_core::path_settings::PathDefaults {
            audio_directory: "audio".into(),
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
    let size = egui::vec2(1200.0, 3000.0);

    let initial = run_ui_frame_at_size(&context, &mut app, Vec::new(), size);
    assert_eq!(initial.visible_spectra.len(), 2);
    assert_eq!(
        app.view.microphone_source_identity.as_deref(),
        Some("wasapi:chosen")
    );

    let mut previous = initial.visible_spectra;
    for _ in 0..4 {
        let frame = run_ui_frame_with_discard(&context, &mut app, Vec::new(), size, true);
        assert!(
            frame.completed_passes >= 2,
            "the UI frame must complete a discarded and final pass"
        );
        assert_eq!(
            frame.visible_spectra.len(),
            2,
            "both clipped graph areas remain visible"
        );
        for (before, after) in previous.iter().zip(&frame.visible_spectra) {
            assert!(before.min.distance(after.min) < 1.0);
            assert!(before.max.distance(after.max) < 1.0);
        }
        assert!(app.view.microphone_spectrum.is_some());
        assert_eq!(
            app.view.microphone_source_identity.as_deref(),
            Some("wasapi:chosen")
        );
        previous = frame.visible_spectra;
    }
}

#[test]
fn microphone_saturation_badge_requires_current_identity_and_fresh_limiter_data() {
    assert_eq!(MICROPHONE_SATURATION_LABEL, "Écrêtage après gain détecté");
    assert_eq!(MICROPHONE_SATURATION_COLOR, egui::Color32::RED);
    let meter = whisper_core::microphone_level::MicrophoneMeter {
        job_id: JobId(7),
        generation: Generation::first(),
        source_identity: "wasapi:A".into(),
        sequence: 1,
        first_frame: 0,
        last_frame: 480,
        sample_rate_hz: 48_000,
        analyzed_channels: 1,
        gain_first_db: 0,
        gain_first_revision: 0,
        gain_last_db: 0,
        gain_last_revision: 0,
        mixed_gain: false,
        pre_gain: whisper_core::microphone_level::MeterLevel {
            status: whisper_core::microphone_level::MeasurementStatus::Signal,
            sample_count: 480,
            rms_micros: 100_000,
            peak_micros: 200_000,
        },
        post_gain_limiter: whisper_core::microphone_level::MeterLevel {
            status: whisper_core::microphone_level::MeasurementStatus::Signal,
            sample_count: 480,
            rms_micros: 100_000,
            peak_micros: 1_000_000,
        },
        limiter_samples: 2,
        dropped_updates: 0,
        captured_at_monotonic_ms: 1_000,
    };
    let capture = Some((JobId(7), Generation::first()));
    assert!(microphone_saturation_is_current(
        Some(&meter),
        capture,
        Some("wasapi:A"),
        true,
        1_500
    ));
    assert!(!microphone_saturation_is_current(
        Some(&meter),
        capture,
        Some("wasapi:B"),
        true,
        1_500
    ));
    assert!(!microphone_saturation_is_current(
        Some(&meter),
        capture,
        Some("wasapi:A"),
        true,
        1_501
    ));
    assert!(!microphone_saturation_is_current(
        Some(&meter),
        None,
        Some("wasapi:A"),
        true,
        1_500
    ));
    assert!(!microphone_saturation_is_current(
        Some(&meter),
        capture,
        Some("wasapi:A"),
        false,
        1_500
    ));
    let mut unclipped = meter;
    unclipped.limiter_samples = 0;
    assert!(!microphone_saturation_is_current(
        Some(&unclipped),
        capture,
        Some("wasapi:A"),
        true,
        1_500
    ));
}

#[test]
fn starting_a_new_microphone_job_hides_the_previous_publication() {
    let mut app = DesktopApp::new(RecordingApplication::default());
    app.view.active_job = Some((JobId(1), whisper_core::JobState::Complete));
    app.view.last_result = Some("Transcription publiée et vérifiée".into());
    assert!(app.published_result().is_some());

    app.start_live(Generation::first());
    let next_job = app.live_request.as_ref().unwrap().job_id;
    app.view.active_job = Some((next_job, whisper_core::JobState::Preparing));
    app.view.last_result = Some("Transcription publiée et vérifiée".into());
    assert_eq!(app.published_result(), None);
}

#[test]
fn refused_ui_model_command_has_no_gpu_candidate_io_effect() {
    let application = RefusingStagedCandidateApplication::default();
    let effects = application.effects.clone();
    let pending_candidate = application.pending_candidate.clone();
    let paths = whisper_core::path_settings::PathSettingsApp::with_defaults(
        whisper_core::path_settings::PathDefaults {
            audio_directory: String::new(),
            model_path: "model.bin".into(),
            destination: "archive".into(),
        },
    );
    let mut app = DesktopApp::with_paths(application, ComputeChoice::Cpu, None, paths);
    let request = ImportRequest {
        strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
        job_id: JobId(73),
        generation: Generation::first(),
        source_path: "input.wav".into(),
        source_sha256: None,
        source_samples: None,
        model_path: "model.bin".into(),
        model_sha256: [0; 32],
        destination: "archive".into(),
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        },
    };

    assert!(app.dispatch(AppCommand::EnqueueImport { request }));
    assert!(
        app.view
            .message
            .as_deref()
            .is_some_and(|message| message.contains("Modèle indisponible"))
    );
    assert!(effects.borrow().is_empty());
    assert!(pending_candidate.borrow().is_none());
}

#[test]
fn direct_live_request_uses_explicit_microphone_without_changing_compute() {
    let recorder = RecordingApplication::default();
    let paths = whisper_core::path_settings::PathSettingsApp::with_defaults(
        whisper_core::path_settings::PathDefaults {
            audio_directory: String::new(),
            model_path: "model".into(),
            destination: "output".into(),
        },
    );
    let mut app = DesktopApp::with_microphones(
        recorder.clone(),
        ComputeChoice::Gpu,
        None,
        paths,
        Box::new(OneMicrophone),
    );
    app.microphones
        .select(MicrophoneChoice::Device("wasapi:chosen".into()))
        .unwrap();
    app.start_live(Generation::first());
    assert!(matches!(
        recorder.0.borrow().last(),
        Some(AppCommand::StartDirectLiveConfigured { request })
            if request.microphone == MicrophoneChoice::Device("wasapi:chosen".into())
                && request.config.compute == ComputeChoice::Gpu
    ));
}
