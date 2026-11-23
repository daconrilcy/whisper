#![windows_subsystem = "windows"]

mod root;

fn main() -> eframe::Result {
    #[cfg(windows)]
    let has_msix_identity = matches!(
        whisper_adapters::path_settings::current_msix_local_data_root(),
        Ok(Some(_))
    );
    whisper_adapters::cpu_capabilities::emit_desktop_startup_diagnostic();
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
        reactivity_log_path,
    } = root::compose();
    let result = whisper_desktop::run(
        application,
        whisper_desktop::DesktopStartup {
            compute_choice,
            startup_message,
            paths,
            microphone_port: Box::new(whisper_adapters::capture::WasapiMicrophonePort),
            gpu_probe,
            gpu_inventory,
            process_started_at_unix_ms,
            model_selection,
            unified_candidate_enabled,
            parakeet_edition_enabled: !whisper_desktop::TURBO_EDITION,
            reactivity_log_path,
        },
    );

    #[cfg(windows)]
    match result {
        Err(error)
            if has_msix_identity
                && whisper_adapters::installer_ui::show_msix_glow_startup_error(
                    &error.to_string(),
                ) =>
        {
            Ok(())
        }
        other => other,
    }

    #[cfg(not(windows))]
    result
}
