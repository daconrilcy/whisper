use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use crate::installer::{self, InstallOutcome, InstallRequest};
use whisper_adapters::installer_ui;

#[cfg(test)]
mod msix_tests;

pub(crate) fn run() -> ExitCode {
    let compatibility = whisper_adapters::cpu_capabilities::current_cpu_compatibility();
    if let Err(message) = super::preflight_message(&compatibility) {
        installer_ui::show_error("Prérequis CPU non satisfait", message);
        return ExitCode::FAILURE;
    }
    let local_app_data = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    let action = match startup_action(
        whisper_adapters::path_settings::current_msix_local_data_root(),
        local_app_data.clone(),
    ) {
        Ok(action) => action,
        Err(error) => {
            installer_ui::show_error("Identité MSIX indisponible", &error);
            return ExitCode::FAILURE;
        }
    };
    let bundle_root = current_bundle_root();
    let model_path = match select_model_for_action(&action, bundle_root.as_deref()) {
        Ok(Some(path)) => path,
        Ok(None) => return ExitCode::SUCCESS,
        Err(error) => {
            installer_ui::show_error("Modèle Whisper Turbo indisponible", &error);
            return ExitCode::FAILURE;
        }
    };
    let preflight_local_app_data = action.local_app_data().or(local_app_data.as_deref());
    let preflight = match &action {
        StartupAction::LaunchPackaged { .. } => installer::preflight_packaged_bundle,
        StartupAction::Install { .. } => installer::preflight_bundle,
    };
    if let Err(error) = preflight(
        preflight_local_app_data,
        bundle_root.as_deref(),
        &model_path,
        &compatibility,
    ) {
        installer_ui::show_error("Paquet WhisperTurbo incomplet", &error.to_string());
        return ExitCode::FAILURE;
    }
    match action {
        StartupAction::LaunchPackaged { .. } => launch_packaged_desktop(bundle_root.as_deref()),
        StartupAction::Install { local_app_data } => {
            if !installer_ui::confirm_install() {
                return ExitCode::SUCCESS;
            }
            install(local_app_data, bundle_root, model_path, &compatibility)
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum StartupAction {
    LaunchPackaged { local_data: PathBuf },
    Install { local_app_data: Option<PathBuf> },
}

impl StartupAction {
    fn local_app_data(&self) -> Option<&Path> {
        match self {
            Self::LaunchPackaged { local_data } => Some(local_data),
            Self::Install { .. } => None,
        }
    }
}

fn startup_action(
    msix_local_data: Result<Option<PathBuf>, String>,
    local_app_data: Option<PathBuf>,
) -> Result<StartupAction, String> {
    match msix_local_data {
        Ok(Some(local_data)) => Ok(StartupAction::LaunchPackaged { local_data }),
        Ok(None) => Ok(StartupAction::Install { local_app_data }),
        Err(error) => Err(error),
    }
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

fn select_model_for_action(
    action: &StartupAction,
    bundle_root: Option<&Path>,
) -> Result<Option<PathBuf>, String> {
    match action {
        StartupAction::LaunchPackaged { .. } => installer::bundled_turbo_model(bundle_root)
            .map(Some)
            .map_err(|error| error.to_string()),
        StartupAction::Install { .. } => select_model(bundle_root),
    }
}

fn launch_packaged_desktop(bundle_root: Option<&Path>) -> ExitCode {
    let Some(bundle_root) = bundle_root else {
        installer_ui::show_error(
            "Démarrage WhisperTurbo impossible",
            "Le dossier du paquet MSIX est introuvable.",
        );
        return ExitCode::FAILURE;
    };
    let desktop = packaged_desktop_path(bundle_root);
    match Command::new(&desktop).current_dir(bundle_root).spawn() {
        Ok(_) => ExitCode::SUCCESS,
        Err(error) => {
            installer_ui::show_error(
                "Démarrage WhisperTurbo impossible",
                &format!("{}\n\n{error}", desktop.display()),
            );
            ExitCode::FAILURE
        }
    }
}

fn packaged_desktop_path(bundle_root: &Path) -> PathBuf {
    bundle_root.join("app").join("whisper-desktop.exe")
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
