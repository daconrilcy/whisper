#![forbid(unsafe_code)]

pub mod ui;

pub fn run<A: whisper_core::Application + 'static>(application: A) -> eframe::Result {
    eframe::run_native(
        "Whisper",
        eframe::NativeOptions {
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        },
        Box::new(move |_creation_context| Ok(Box::new(ui::DesktopApp::new(application)))),
    )
}
