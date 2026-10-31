#[test]
fn empty_snapshot_recovery_result_does_not_render_a_blank_modal() {
    fn rendered_shape_count(
        candidates: Option<&[whisper_core::ports::SourceSnapshotRecoveryCandidate]>,
    ) -> usize {
        let context = egui::Context::default();
        let output = context.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                super::source_snapshot_recovery_widget(ui, false, candidates);
            });
        });
        output.shapes.len()
    }

    assert_eq!(rendered_shape_count(None), rendered_shape_count(Some(&[])));
}

#[test]
fn parakeet_preserves_the_selected_compute_preference() {
    for choice in [ComputeChoice::Auto, ComputeChoice::Cpu, ComputeChoice::Gpu] {
        assert_eq!(
            compute_for_model(Some(ModelId::ParakeetTdt06BV3), choice),
            choice
        );
    }
    assert_eq!(
        compute_for_model(Some(ModelId::WhisperLargeV3Turbo), ComputeChoice::Auto),
        ComputeChoice::Auto
    );
}

#[test]
fn queued_import_uses_current_compute_and_preserves_the_durable_request() {
    let original = ImportRequest {
        strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
        job_id: JobId(74),
        generation: Generation::first(),
        source_path: "queued.wav".into(),
        source_sha256: Some([4; 32]),
        source_samples: Some(16_000),
        model_path: "parakeet.gguf".into(),
        model_sha256: [8; 32],
        destination: "archive".into(),
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Gpu,
        },
    };

    for compute in [ComputeChoice::Cpu, ComputeChoice::Gpu, ComputeChoice::Auto] {
        let request = queued_request_for_compute(&original, compute);
        let mut expected = original.clone();
        expected.config.compute = compute;
        assert_eq!(request, expected);
        assert_eq!(
            command_compute_choice(&AppCommand::ProcessQueued {
                request: request.clone()
            }),
            Some(compute)
        );
    }
    assert_eq!(original.config.compute, ComputeChoice::Gpu);
}

#[test]
fn resumed_import_uses_its_durable_compute_instead_of_the_global_selector() {
    let mut request = ImportRequest {
        strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
        job_id: JobId(75),
        generation: Generation::first(),
        source_path: "queued.wav".into(),
        source_sha256: Some([4; 32]),
        source_samples: Some(16_000),
        model_path: "parakeet.gguf".into(),
        model_sha256: [8; 32],
        destination: "archive".into(),
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Gpu,
        },
    };
    request.generation = request.generation.next().unwrap();

    let command = AppCommand::ResumeInterrupted { request };
    assert_eq!(command_compute_choice(&command), Some(ComputeChoice::Gpu));
    assert_eq!(
        gpu_selection_identity(&command),
        Some((
            JobId(75),
            Generation::first().next().unwrap().next().unwrap()
        ))
    );
}
