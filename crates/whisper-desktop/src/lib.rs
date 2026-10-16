#![forbid(unsafe_code)]

mod reactivity_log;
pub mod ui;

pub fn run<A: whisper_core::Application + 'static>(
    application: A,
    compute_choice: whisper_core::ComputeChoice,
    startup_message: Option<String>,
    paths: whisper_core::path_settings::PathSettingsApp,
    microphone_port: Box<dyn whisper_core::MicrophonePort>,
    gpu_probe: std::sync::mpsc::Receiver<Result<(), String>>,
    gpu_inventory: whisper_core::gpu_inventory_app::GpuInventoryApp,
    process_started_at_unix_ms: u64,
    model_selection: ui::ModelSelectionRuntime,
) -> eframe::Result {
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
                .with_model_selection_runtime(model_selection),
            ))
        }),
    )
}
