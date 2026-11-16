#![forbid(unsafe_code)]

use std::process::ExitCode;
use whisper_core::cpu_compatibility::{CpuCompatibility, CpuCompatibilityLevel};

fn main() -> ExitCode {
    let compatibility = whisper_adapters::cpu_capabilities::current_cpu_compatibility();
    match preflight_message(&compatibility) {
        Ok(message) => {
            println!("{message}");
            eprintln!("Le parcours d'installation n'est pas disponible dans ce candidat.");
            ExitCode::from(2)
        }
        Err(message) => {
            eprintln!("{message}");
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

#[cfg(test)]
mod tests;
