use super::{
    CatalogSnapshot, DesktopApp, MICROPHONE_SATURATION_COLOR, MICROPHONE_SATURATION_LABEL,
    MicrophoneLabelProjection, MicrophoneMeterPresentation, ModelId, ModelSelectionRuntime,
    command_compute_choice, compute_for_model, gpu_selection_identity, import_enqueue_enabled,
    manual_stop_available, microphone_device_label, microphone_meter_presentation,
    microphone_saturation_is_current, microphone_test_start_enabled,
    model_preference_command_channel, parakeet_gpu_catalog_ready,
    provisional_without_confirmed_prefix, queued_request_for_compute, record_first_render,
    window_must_remain_open,
};
use eframe::egui;
use std::{
    cell::RefCell,
    collections::VecDeque,
    rc::Rc,
    thread,
    time::{Duration, Instant},
};
use whisper_core::gpu_inventory::{GpuObservation, GpuPresence, GpuState};
use whisper_core::gpu_inventory_app::{
    GpuCollection, GpuInventoryApp, GpuInventoryPort, GpuInventoryState,
};
use whisper_core::{
    AppCommand, AppView, Application, ApplicationError, ComputeChoice, Generation, ImportEffect,
    ImportRequest, JobConfig, JobId, LanguageChoice, MicrophoneChoice, MicrophoneDevice,
    MicrophonePort,
};

#[derive(Clone, Default)]
struct RecordingApplication(Rc<RefCell<Vec<AppCommand>>>);

impl Application for RecordingApplication {
    fn dispatch(&mut self, command: AppCommand) -> Result<AppView, ApplicationError> {
        self.0.borrow_mut().push(command);
        Ok(AppView::default())
    }

    fn view(&self) -> AppView {
        AppView::default()
    }
}

#[derive(Clone, Default)]
struct MicrophoneCycleApplication {
    commands: Rc<RefCell<Vec<AppCommand>>>,
    events: Rc<RefCell<VecDeque<whisper_core::ports::ImportEvent>>>,
    view: Rc<RefCell<AppView>>,
}

impl Application for MicrophoneCycleApplication {
    fn dispatch(&mut self, command: AppCommand) -> Result<AppView, ApplicationError> {
        self.commands.borrow_mut().push(command.clone());
        match command {
            AppCommand::StartMicrophoneTest {
                job_id,
                generation,
                microphone: _,
                gain_db,
            } => {
                {
                    let mut view = self.view.borrow_mut();
                    view.microphone_test_active = true;
                    view.microphone_acquisition = Some(
                        whisper_core::microphone_level::MicrophoneAcquisitionContext::new(
                            job_id,
                            generation,
                            &MicrophoneChoice::Device("wasapi:chosen".into()),
                        ),
                    );
                    view.active_generation = Some(generation);
                    view.microphone_gain_requested_db = gain_db;
                    view.microphone_gain_requested_revision = 0;
                    view.microphone_gain_db = gain_db;
                    view.microphone_gain_revision = 0;
                    view.message = Some("Ouverture du test microphone…".into());
                }
                let meter = whisper_core::microphone_level::MicrophoneMeter {
                    job_id,
                    generation,
                    source_identity: "wasapi:chosen".into(),
                    sequence: 1,
                    first_frame: 0,
                    last_frame: 4_800,
                    sample_rate_hz: 48_000,
                    analyzed_channels: 1,
                    gain_first_db: 0,
                    gain_first_revision: 0,
                    gain_last_db: 0,
                    gain_last_revision: 0,
                    mixed_gain: false,
                    pre_gain: whisper_core::microphone_level::MeterLevel {
                        status: whisper_core::microphone_level::MeasurementStatus::Signal,
                        sample_count: 4_800,
                        rms_micros: 100_000,
                        peak_micros: 200_000,
                    },
                    post_gain_limiter: whisper_core::microphone_level::MeterLevel {
                        status: whisper_core::microphone_level::MeasurementStatus::Signal,
                        sample_count: 4_800,
                        rms_micros: 100_000,
                        peak_micros: 200_000,
                    },
                    limiter_samples: 42,
                    dropped_updates: 0,
                    captured_at_monotonic_ms: whisper_core::microphone_level::monotonic_millis(),
                };
                let spectrum = whisper_core::microphone_level::MicrophoneSpectrum {
                    job_id,
                    generation,
                    source_identity: "wasapi:chosen".into(),
                    sequence: 1,
                    epoch: 0,
                    first_frame: 0,
                    last_frame: 2048,
                    sample_rate_hz: 48_000,
                    captured_at_monotonic_ms: whisper_core::microphone_level::monotonic_millis(),
                    gain_first_db: 0,
                    gain_first_revision: 0,
                    gain_last_db: 0,
                    gain_last_revision: 0,
                    mixed_gain: false,
                    dropped_updates: 0,
                    pre_gain: vec![
                        whisper_core::microphone_level::SpectrumBin::MilliDb(-50_000);
                        1025
                    ],
                    post_gain_limiter: vec![
                        whisper_core::microphone_level::SpectrumBin::MilliDb(
                            -48_000
                        );
                        1025
                    ],
                };
                self.events.borrow_mut().extend([
                    whisper_core::ports::ImportEvent::MicrophoneTestStarted {
                        job_id,
                        generation,
                        source_identity: "wasapi:chosen".into(),
                        gain_db,
                    },
                    whisper_core::ports::ImportEvent::MicrophoneGainApplied {
                        job_id,
                        generation,
                        target_identity: "wasapi:chosen".into(),
                        gain_db: 0,
                        gain_revision: 0,
                        first_frame: 0,
                    },
                    whisper_core::ports::ImportEvent::MicrophoneMeter(meter),
                    whisper_core::ports::ImportEvent::MicrophoneSpectrum(
                        whisper_core::microphone_level::MicrophoneSpectrumUpdate::Available(
                            spectrum,
                        ),
                    ),
                ]);
            }
            AppCommand::StopMicrophoneTest { job_id, generation } => {
                self.events.borrow_mut().push_back(
                    whisper_core::ports::ImportEvent::MicrophoneTestStopped { job_id, generation },
                )
            }
            AppCommand::Stop { job_id, .. } => {
                let mut view = self.view.borrow_mut();
                view.active_job = Some((job_id, whisper_core::JobState::Draining));
                view.direct_capture_phase =
                    Some(whisper_core::application::DirectCapturePhase::Stopped);
                view.message = Some("Arrêt du micro en cours".into());
            }
            AppCommand::StartUnifiedLiveConfigured { request } => {
                let mut view = self.view.borrow_mut();
                if view.unified_parakeet_sessions.len() < 2
                    && !view.unified_parakeet_sessions.iter().any(|session| {
                        session.microphone_capture
                            && matches!(
                                session.phase,
                                whisper_core::ports::UnifiedSessionPhase::Starting
                                    | whisper_core::ports::UnifiedSessionPhase::Capturing
                            )
                    })
                {
                    view.unified_parakeet_sessions.push(
                        whisper_core::ports::UnifiedParakeetSessionView {
                            job_id: request.job_id,
                            generation: request.generation,
                            microphone_capture: true,
                            destination: request.destination,
                            source_path: String::new(),
                            phase: whisper_core::ports::UnifiedSessionPhase::Capturing,
                            captured_samples: 0,
                            durable_samples: 0,
                            total_samples: None,
                            document_revision: 0,
                            audio_parts_revision: 0,
                            wav_path: None,
                            txt_path: None,
                            srt_path: None,
                            message: None,
                        },
                    );
                } else {
                    return Err(ApplicationError::InvalidCommand);
                }
            }
            AppCommand::Refresh => {
                if let Some(event) = self.events.borrow_mut().pop_front() {
                    apply_microphone_cycle_event(&mut self.view.borrow_mut(), event);
                }
            }
            _ => {}
        }
        Ok(self.view.borrow().clone())
    }

    fn view(&self) -> AppView {
        self.view.borrow().clone()
    }
}

fn apply_microphone_cycle_event(view: &mut AppView, event: whisper_core::ports::ImportEvent) {
    use whisper_core::ports::ImportEvent;
    match event {
        ImportEvent::MicrophoneTestStarted {
            source_identity,
            gain_db,
            ..
        } => {
            view.microphone_test_active = true;
            view.direct_capture_phase = Some(whisper_core::application::DirectCapturePhase::Active);
            view.microphone_source_identity = Some(source_identity);
            view.microphone_gain_db = gain_db;
            view.message = Some("Test microphone actif".into());
        }
        ImportEvent::MicrophoneGainApplied {
            target_identity,
            gain_db,
            gain_revision,
            first_frame,
            ..
        } => {
            view.microphone_source_identity = Some(target_identity);
            view.microphone_gain_db = gain_db;
            view.microphone_gain_revision = gain_revision;
            view.microphone_gain_applied_first_frame = Some(first_frame);
            view.message = Some(format!(
                "Gain microphone appliqué : {gain_db} dB à la trame {first_frame}"
            ));
        }
        ImportEvent::MicrophoneMeter(meter) => view.microphone_meter = Some(meter),
        ImportEvent::MicrophoneSpectrum(
            whisper_core::microphone_level::MicrophoneSpectrumUpdate::Available(spectrum),
        ) => view.microphone_spectrum = Some(spectrum),
        ImportEvent::MicrophoneSpectrum(
            whisper_core::microphone_level::MicrophoneSpectrumUpdate::Unavailable { .. },
        ) => view.microphone_spectrum = None,
        ImportEvent::MicrophoneTestStopped { .. } => {
            view.microphone_test_active = false;
            view.microphone_acquisition = None;
            view.microphone_spectrum = None;
            view.direct_capture_phase =
                Some(whisper_core::application::DirectCapturePhase::Stopped);
            view.microphone_meter = None;
            view.microphone_source_identity = None;
            view.microphone_gain_db = 0;
            view.microphone_gain_revision = 0;
            view.microphone_gain_requested_db = 0;
            view.microphone_gain_requested_revision = 0;
            view.microphone_gain_applied_first_frame = None;
            view.active_generation = None;
            view.message = Some("Test microphone arrêté".into());
        }
        _ => {}
    }
}

struct UiFrameReport {
    shape_count: usize,
    completed_passes: usize,
    visible_spectra: Vec<egui::Rect>,
}

fn run_ui_frame(
    context: &egui::Context,
    app: &mut DesktopApp<MicrophoneCycleApplication>,
    events: Vec<egui::Event>,
) -> UiFrameReport {
    run_ui_frame_at_size(context, app, events, egui::vec2(1200.0, 3000.0))
}

fn run_ui_frame_at_size(
    context: &egui::Context,
    app: &mut DesktopApp<MicrophoneCycleApplication>,
    events: Vec<egui::Event>,
    size: egui::Vec2,
) -> UiFrameReport {
    run_ui_frame_with_discard(context, app, events, size, false)
}

fn run_ui_frame_with_discard(
    context: &egui::Context,
    app: &mut DesktopApp<MicrophoneCycleApplication>,
    events: Vec<egui::Event>,
    size: egui::Vec2,
    discard_first_pass: bool,
) -> UiFrameReport {
    let input = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
        events,
        ..Default::default()
    };
    let output = context.run_ui(input, |ui| {
        if discard_first_pass && ui.ctx().current_pass_index() == 0 {
            ui.ctx()
                .request_discard("microphone spectrum multipass test");
        }
        app.show_ui(ui);
    });
    let screen_rect = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
    let visible_spectra = output
        .shapes
        .iter()
        .filter_map(|clipped| {
            let egui::Shape::Rect(rect) = &clipped.shape else {
                return None;
            };
            (rect.fill == egui::Color32::from_gray(18)
                && rect.rect.height() == 116.0
                && rect
                    .rect
                    .intersect(clipped.clip_rect)
                    .intersect(screen_rect)
                    .is_positive())
            .then_some(rect.rect)
        })
        .collect();
    UiFrameReport {
        shape_count: output.shapes.len(),
        completed_passes: output.platform_output.num_completed_passes,
        visible_spectra,
    }
}

fn actual_probe_spectrum() -> whisper_core::microphone_level::MicrophoneSpectrum {
    use whisper_adapters::{
        capture::ProbeAudioBlock,
        microphone_probe::{MicrophoneProbe, MicrophoneSpectrumReader},
    };
    use whisper_core::microphone_level::GainRevision;

    let source_identity = "wasapi:chosen";
    let probe = MicrophoneProbe::start(
        48_000,
        Some(source_identity.into()),
        JobId(902),
        Generation::first(),
    )
    .expect("probe starts");
    for block in 0..6_u64 {
        let first_frame = block * 960;
        let samples = (first_frame..first_frame + 960)
            .map(|frame| 0.35 * (std::f32::consts::TAU * 440.0 * frame as f32 / 48_000.0).sin())
            .collect::<Vec<_>>();
        probe.submit(ProbeAudioBlock {
            sample_rate_hz: 48_000,
            first_frame,
            acquired_at: Instant::now(),
            gain: GainRevision {
                revision: 0,
                gain: whisper_core::microphone_level::GainDb::new(0).unwrap(),
            },
            mono_pre_gain: samples.clone(),
            mono_post_gain_limiter: samples,
            limiter_mask: vec![false; 960],
            pre_pcm_limiter_count: 0,
        });
        thread::sleep(Duration::from_millis(20));
    }
    let mut reader = MicrophoneSpectrumReader::default();
    let deadline = Instant::now() + Duration::from_secs(2);
    loop {
        if let Some(update) = reader.poll(&probe, Instant::now()) {
            return match update {
                whisper_core::microphone_level::MicrophoneSpectrumUpdate::Available(value) => value,
                whisper_core::microphone_level::MicrophoneSpectrumUpdate::Unavailable {
                    ..
                } => {
                    panic!("fresh probe unexpectedly invalidated spectrum")
                }
            };
        }
        assert!(
            Instant::now() < deadline,
            "probe did not publish its FFT pair"
        );
        thread::sleep(Duration::from_millis(2));
    }
}

fn click_ui_button(
    context: &egui::Context,
    app: &mut DesktopApp<MicrophoneCycleApplication>,
    position: egui::Pos2,
) {
    for pressed in [true, false] {
        run_ui_frame(
            context,
            app,
            vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::default(),
                },
            ],
        );
    }
}

struct RejectQuitApplication;
struct RejectStartApplication;

#[derive(Clone, Default)]
struct RefusingStagedCandidateApplication {
    effects: Rc<RefCell<Vec<ImportEffect>>>,
    pending_candidate: Rc<RefCell<Option<ImportEffect>>>,
}

struct OneMicrophone;
#[derive(Clone)]
struct ChangingDefaultMicrophone(Rc<RefCell<String>>);

#[derive(Clone, Default)]
struct ControlledGpuInventory(Rc<RefCell<VecDeque<(u64, Result<GpuCollection, String>)>>>);

impl GpuInventoryPort for ControlledGpuInventory {
    fn start(&mut self, _generation: u64) -> Result<(), String> {
        Ok(())
    }

    fn poll(&mut self) -> Option<(u64, Result<GpuCollection, String>)> {
        self.0.borrow_mut().pop_front()
    }
}

impl MicrophonePort for OneMicrophone {
    fn list_inputs(&self) -> Result<Vec<MicrophoneDevice>, String> {
        Ok(vec![MicrophoneDevice {
            id: "wasapi:chosen".into(),
            name: "Microphone de test".into(),
            unavailable_reason: None,
        }])
    }

    fn default_input_identity(&self) -> Result<Option<String>, String> {
        Ok(Some("wasapi:chosen".into()))
    }
}

struct HomonymousMicrophones;

impl MicrophonePort for HomonymousMicrophones {
    fn list_inputs(&self) -> Result<Vec<MicrophoneDevice>, String> {
        Ok(vec![
            MicrophoneDevice {
                id: "wasapi:alpha".into(),
                name: "Micro de bureau".into(),
                unavailable_reason: None,
            },
            MicrophoneDevice {
                id: "wasapi:beta".into(),
                name: "Micro de bureau".into(),
                unavailable_reason: None,
            },
            MicrophoneDevice {
                id: "wasapi:gamma".into(),
                name: "Micro de bureau".into(),
                unavailable_reason: None,
            },
        ])
    }

    fn default_input_identity(&self) -> Result<Option<String>, String> {
        Ok(Some("wasapi:alpha".into()))
    }
}

impl MicrophonePort for ChangingDefaultMicrophone {
    fn list_inputs(&self) -> Result<Vec<MicrophoneDevice>, String> {
        let id = self.0.borrow().clone();
        Ok(vec![MicrophoneDevice {
            id: id.clone(),
            name: "Microphone Windows".into(),
            unavailable_reason: None,
        }])
    }

    fn default_input_identity(&self) -> Result<Option<String>, String> {
        Ok(Some(self.0.borrow().clone()))
    }
}

impl Application for RejectQuitApplication {
    fn dispatch(&mut self, command: AppCommand) -> Result<AppView, ApplicationError> {
        if matches!(command, AppCommand::Quit { .. } | AppCommand::RequestClose) {
            Err(ApplicationError::Failed("quit admission rejected".into()))
        } else {
            Ok(AppView::default())
        }
    }

    fn view(&self) -> AppView {
        AppView::default()
    }
}

impl Application for RejectStartApplication {
    fn dispatch(&mut self, _command: AppCommand) -> Result<AppView, ApplicationError> {
        Err(ApplicationError::Failed("start admission rejected".into()))
    }

    fn view(&self) -> AppView {
        AppView::default()
    }
}

impl Application for RefusingStagedCandidateApplication {
    fn dispatch(&mut self, command: AppCommand) -> Result<AppView, ApplicationError> {
        if matches!(
            command,
            AppCommand::StartImport { .. }
                | AppCommand::EnqueueImport { .. }
                | AppCommand::ProcessQueued { .. }
                | AppCommand::ResumeInterrupted { .. }
                | AppCommand::StartLiveConfigured { .. }
                | AppCommand::StartDirectLiveConfigured { .. }
        ) {
            self.pending_candidate.borrow_mut().take();
            return Ok(AppView {
                message: Some("Modèle indisponible : test d'admission".into()),
                ..AppView::default()
            });
        }
        if let Some(effect) = self.pending_candidate.borrow_mut().take() {
            self.effects.borrow_mut().push(effect);
        }
        Ok(AppView::default())
    }

    fn view(&self) -> AppView {
        AppView::default()
    }

    fn set_gpu_candidate(
        &mut self,
        job_id: JobId,
        generation: Generation,
        candidate: Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
    ) -> Result<(), ApplicationError> {
        *self.pending_candidate.borrow_mut() = Some(ImportEffect::SetGpuCandidate {
            job_id,
            generation,
            candidate,
        });
        Ok(())
    }
}
