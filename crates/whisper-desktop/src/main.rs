#![windows_subsystem = "windows"]

mod root;

fn main() -> eframe::Result {
    let root::DesktopComposition {
        application,
        compute_choice,
        startup_message,
        paths,
        gpu_probe,
    } = root::compose();
    whisper_desktop::run(
        application,
        compute_choice,
        startup_message,
        paths,
        Box::new(whisper_adapters::capture::WasapiMicrophonePort),
        gpu_probe,
    )
}
