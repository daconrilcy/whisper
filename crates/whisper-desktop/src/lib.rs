#![forbid(unsafe_code)]

pub mod ui;

pub fn run<A: whisper_core::Application + 'static>(
    application: A,
    compute_choice: whisper_core::ComputeChoice,
    startup_message: Option<String>,
) -> eframe::Result {
    eframe::run_native(
        "Whisper",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        },
        Box::new(move |_creation_context| {
            Ok(Box::new(ui::DesktopApp::with_startup_compute(
                application,
                compute_choice,
                startup_message,
            )))
        }),
    )
}
