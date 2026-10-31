#![forbid(unsafe_code)]

mod reactivity_log;
pub mod ui;

pub struct DesktopStartup {
    pub compute_choice: whisper_core::ComputeChoice,
    pub startup_message: Option<String>,
    pub paths: whisper_core::path_settings::PathSettingsApp,
    pub microphone_port: Box<dyn whisper_core::MicrophonePort>,
    pub gpu_probe: std::sync::mpsc::Receiver<Result<(), String>>,
    pub gpu_inventory: whisper_core::gpu_inventory_app::GpuInventoryApp,
    pub process_started_at_unix_ms: u64,
    pub model_selection: ui::ModelSelectionRuntime,
    pub unified_candidate_enabled: bool,
}

pub fn run<A: whisper_core::Application + 'static>(
    application: A,
    startup: DesktopStartup,
) -> eframe::Result {
    let DesktopStartup {
        compute_choice,
        startup_message,
        paths,
        microphone_port,
        gpu_probe,
        gpu_inventory,
        process_started_at_unix_ms,
        model_selection,
        unified_candidate_enabled,
    } = startup;
    eframe::run_native(
        "Whisper",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        },
        Box::new(move |_creation_context| {
            Ok(Box::new(
                ui::DesktopApp::with_microphones(
                    application,
                    compute_choice,
                    startup_message,
                    paths,
                    microphone_port,
                )
                .with_gpu_probe(gpu_probe)
                .with_gpu_inventory(gpu_inventory)
                .with_process_started(process_started_at_unix_ms)
                .with_model_selection_runtime(model_selection)
                .with_unified_candidate_enabled(unified_candidate_enabled),
            ))
        }),
    )
}
