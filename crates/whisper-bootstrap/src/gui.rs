use std::path::{Path, PathBuf};
use std::process::ExitCode;

use crate::installer::{self, InstallOutcome, InstallRequest};
use whisper_adapters::installer_ui;

pub(crate) fn run() -> ExitCode {
    let compatibility = whisper_adapters::cpu_capabilities::current_cpu_compatibility();
    if let Err(message) = super::preflight_message(&compatibility) {
        installer_ui::show_error("Prérequis CPU non satisfait", message);
        return ExitCode::FAILURE;
    }
    let local_app_data = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    let bundle_root = current_bundle_root();
    let model_path = match select_model(bundle_root.as_deref()) {
        Ok(Some(path)) => path,
        Ok(None) => return ExitCode::SUCCESS,
        Err(error) => {
            installer_ui::show_error("Modèle Whisper Turbo indisponible", &error);
            return ExitCode::FAILURE;
        }
    };
    if let Err(error) = installer::preflight_bundle(
        local_app_data.as_deref(),
        bundle_root.as_deref(),
        &model_path,
        &compatibility,
    ) {
        installer_ui::show_error("Paquet WhisperTurbo incomplet", &error.to_string());
        return ExitCode::FAILURE;
    }
    if !installer_ui::confirm_install() {
        return ExitCode::SUCCESS;
    }
    install(local_app_data, bundle_root, model_path, &compatibility)
}

fn current_bundle_root() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
}

fn select_model(bundle_root: Option<&Path>) -> Result<Option<PathBuf>, String> {
    if installer::DUAL_EDITION {
        installer::bundled_turbo_model(bundle_root)
            .map(Some)
            .map_err(|error| error.to_string())
    } else {
        Ok(whisper_adapters::installer_ui::choose_turbo_model())
    }
}

fn install(
    local_app_data: Option<PathBuf>,
    bundle_root: Option<PathBuf>,
    model_path: PathBuf,
    compatibility: &whisper_core::cpu_compatibility::CpuCompatibility,
) -> ExitCode {
    let request = InstallRequest {
        local_app_data,
        bundle_root,
        model_path,
        confirmed: true,
    };
    match installer::install(request, compatibility) {
        Ok(InstallOutcome::Installed(path)) => {
            installer_ui::show_information(
                "Installation terminée",
                &format!("WhisperTurbo a été installé dans :\n{}", path.display()),
            );
            ExitCode::SUCCESS
        }
        Ok(InstallOutcome::Cancelled) => ExitCode::SUCCESS,
        Err(error) => {
            installer_ui::show_error("Installation WhisperTurbo refusée", &error.to_string());
            ExitCode::FAILURE
        }
    }
}
