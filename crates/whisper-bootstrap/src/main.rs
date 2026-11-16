#![forbid(unsafe_code)]

mod installer;

use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use whisper_core::cpu_compatibility::{CpuCompatibility, CpuCompatibilityLevel};

fn main() -> ExitCode {
    let compatibility = whisper_adapters::cpu_capabilities::current_cpu_compatibility();
    if let Err(message) = preflight_message(&compatibility) {
        eprintln!("{message}");
        return ExitCode::FAILURE;
    }

    let model = match prompt_model_path() {
        Ok(Some(path)) => path,
        Ok(None) => {
            eprintln!("Installation annulée; aucune installation n'a été créée.");
            return ExitCode::SUCCESS;
        }
        Err(error) => {
            eprintln!("Lecture du chemin du modèle impossible: {error}");
            return ExitCode::FAILURE;
        }
    };
    let confirmed = match prompt_confirmation() {
        Ok(confirmed) => confirmed,
        Err(error) => {
            eprintln!("Lecture de la confirmation impossible: {error}");
            return ExitCode::FAILURE;
        }
    };
    let request = installer::InstallRequest {
        local_app_data: std::env::var_os("LOCALAPPDATA").map(PathBuf::from),
        bundle_root: std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(PathBuf::from)),
        model_path: model,
        confirmed,
    };

    match installer::install(request, &compatibility) {
        Ok(installer::InstallOutcome::Installed(path)) => {
            println!("Installation terminée dans {}.", path.display());
            ExitCode::SUCCESS
        }
        Ok(installer::InstallOutcome::Cancelled) => {
            eprintln!("Installation annulée; aucune installation n'a été créée.");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn preflight_message(compatibility: &CpuCompatibility) -> Result<&'static str, &'static str> {
    match compatibility.level {
        CpuCompatibilityLevel::X86_64V2 => Ok("Ce PC satisfait le prérequis CPU x86-64-v2."),
        CpuCompatibilityLevel::BelowX86_64V2 => {
            Err("Installation refusée : le CPU ne satisfait pas le niveau x86-64-v2.")
        }
        CpuCompatibilityLevel::Indeterminate => {
            Err("Installation refusée : la compatibilité du CPU n'a pas pu être vérifiée.")
        }
    }
}

fn prompt_model_path() -> io::Result<Option<PathBuf>> {
    print!("Chemin du modèle ggml-large-v3-turbo.bin (Entrée pour annuler) : ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().trim_matches('"');
    Ok((!input.is_empty()).then(|| PathBuf::from(input)))
}

fn prompt_confirmation() -> io::Result<bool> {
    print!("Installer WhisperTurbo dans %LOCALAPPDATA%\\WhisperTurbo ? [o/N] : ");
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    Ok(matches!(
        input.trim().to_ascii_lowercase().as_str(),
        "o" | "oui" | "y" | "yes"
    ))
}

#[cfg(test)]
mod tests;
