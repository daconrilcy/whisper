#![windows_subsystem = "windows"]

mod root;

fn main() -> eframe::Result {
    let process_started_at_unix_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as u64);
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
        process_started_at_unix_ms,
    )
}
