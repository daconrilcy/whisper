#![windows_subsystem = "windows"]

mod root;

fn main() -> eframe::Result {
    let process_started_at_unix_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as u64);
    let root::DesktopComposition {
        application,
        compute_choice,
        unified_candidate_enabled,
        startup_message,
        paths,
        gpu_probe,
        gpu_inventory,
        model_selection,
    } = root::compose();
    whisper_desktop::run(
        application,
        compute_choice,
        startup_message,
        paths,
        Box::new(whisper_adapters::capture::WasapiMicrophonePort),
        gpu_probe,
        gpu_inventory,
        process_started_at_unix_ms,
        model_selection,
        unified_candidate_enabled,
    )
}
