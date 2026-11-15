fn stale_audio_page_retry_test_app() -> (
    MicrophoneCycleApplication,
    DesktopApp<MicrophoneCycleApplication>,
) {
    use whisper_core::ports::{
        UnifiedParakeetSelectionView, UnifiedParakeetSessionView, UnifiedSessionPhase,
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
    let app = DesktopApp::with_microphones(
        application.clone(),
        ComputeChoice::Cpu,
        None,
        paths,
        Box::new(OneMicrophone),
    );
    (application, app)
}

fn visible_audio_page_test_app() -> (
    MicrophoneCycleApplication,
    DesktopApp<MicrophoneCycleApplication>,
) {
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
    let app = DesktopApp::with_microphones(
        application.clone(),
        ComputeChoice::Cpu,
        None,
        paths,
        Box::new(OneMicrophone),
    );
    (application, app)
}

fn run_unified_audio_frame(
    context: &egui::Context,
    app: &mut DesktopApp<MicrophoneCycleApplication>,
    time: Option<f64>,
) -> egui::FullOutput {
    context.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 3000.0),
            )),
            time,
            ..Default::default()
        },
        |ui| app.show_ui(ui),
    )
}

fn unified_audio_page_read_count(application: &MicrophoneCycleApplication) -> usize {
    application
        .commands
        .borrow()
        .iter()
        .filter(|command| matches!(command, AppCommand::ReadUnifiedAudioPartsPage { .. }))
        .count()
}

fn unified_audio_frame_shows_corruption(output: &egui::FullOutput) -> bool {
    output.shapes.iter().any(|shape| {
        matches!(
            &shape.shape,
            egui::Shape::Text(text)
                if text.galley.job.text.contains("StorageCorrupt: bad index")
        )
    })
}

fn set_unified_audio_progress(application: &MicrophoneCycleApplication, samples: u64) {
    let mut view = application.view.borrow_mut();
    view.unified_parakeet_sessions[0].captured_samples = samples;
    view.unified_parakeet_sessions[0].durable_samples = samples;
}

fn unified_audio_page_is_updating(output: &egui::FullOutput) -> bool {
    output.shapes.iter().any(|shape| {
        matches!(
            &shape.shape,
            egui::Shape::Text(text)
                if text.galley.job.text.contains("Actualisation des références audio")
        )
    })
}

fn unified_audio_page_part_y(output: &egui::FullOutput) -> Option<f32> {
    output
        .shapes
        .iter()
        .rev()
        .find_map(|shape| match &shape.shape {
            egui::Shape::Text(text) if text.galley.job.text.contains("Partie 1") => {
                Some(text.pos.y)
            }
            _ => None,
        })
}
