use super::BundleAsset;
use std::path::{Path, PathBuf};

pub(super) const DUAL_FILES: &[&str] = &[
    "app/whisper-desktop.exe|app/whisper-desktop.exe|WHISPER_DUAL_BOOTSTRAP_DESKTOP_SHA256",
    "app/whisper-worker-cpu.exe|app/whisper-worker-cpu.exe|WHISPER_DUAL_BOOTSTRAP_TURBO_CPU_SHA256",
    "worker/whisper-worker-parakeet-cpu.exe|worker/whisper-worker-parakeet-cpu.exe|WHISPER_DUAL_BOOTSTRAP_PARAKEET_CPU_SHA256",
    "models/parakeet-tdt-0.6b-v3.q8_0.gguf|models/parakeet-tdt-0.6b-v3.q8_0.gguf|e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e",
    "app/msvcp140.dll|app/msvcp140.dll|0f885b509a685d2bbfa652fed26b5fb31d88fbdab0a978c641d1c7b8aa460aa9",
    "app/vcruntime140.dll|app/vcruntime140.dll|d5e4d9a3e835fa679450145d6a7d94e36573a509317111904d9b3712c30d9066",
    "app/vcruntime140_1.dll|app/vcruntime140_1.dll|1f2d41c4aa5db0bc33ebf7b66d72943a817d7ce6cbe880502a9403823633093f",
    "native/cpu/ggml-base.dll|native/cpu/ggml-base.dll|24e84c51f0fc48b86a69543fd3f1c0510872d3bc0467eeb7629f09b8707de2b4",
    "native/cpu/ggml-cpu.dll|native/cpu/ggml-cpu.dll|235f6f5ec9e1b6d06788d34d0a2240759ad3e07ef43b18bdb73e9b2a7fa53d30",
    "native/cpu/ggml.dll|native/cpu/ggml.dll|b003bc596dabac2e56402896b614f65ae22966bd8fe7b4b35aca13c8fca32c15",
    "native/cpu/nemo_speech_asr_c.dll|native/cpu/nemo_speech_asr_c.dll|4b9c27b4a505865e28025ccd53a98d2859280d69db51c5b576ddb62e2a6b710b",
    "native/cpu/nemo_speech_asr.dll|native/cpu/nemo_speech_asr.dll|907d1df481f9906089d64919bbdb57052c90e55204ca5b4a97eefe55d97ab959",
    "native/cpu/msvcp140.dll|native/cpu/msvcp140.dll|0f885b509a685d2bbfa652fed26b5fb31d88fbdab0a978c641d1c7b8aa460aa9",
    "native/cpu/vcruntime140.dll|native/cpu/vcruntime140.dll|d5e4d9a3e835fa679450145d6a7d94e36573a509317111904d9b3712c30d9066",
    "native/cpu/vcruntime140_1.dll|native/cpu/vcruntime140_1.dll|1f2d41c4aa5db0bc33ebf7b66d72943a817d7ce6cbe880502a9403823633093f",
    "native/cpu/vcomp140.dll|native/cpu/vcomp140.dll|55aba23cdcd6484fbb06f4155b8ca75adfce7a881f10afd0c49457165e677164",
    "notices/THIRD_PARTY_NOTICES.md|notices/THIRD_PARTY_NOTICES.md|775223b5d3a347d26a25ee4b9564fcd35817bb5d89e39bb8149e22cf8ae47dc8",
];

const TURBO_FILES: &[(&str, &str, &str)] = &[
    ("app/whisper-desktop.exe", "app/whisper-desktop.exe", "7f352689d12787d529462438c5e1ad24c8a741f5c7cf134e0cb5ddc09090ecfb"),
    ("app/whisper-worker-cpu.exe", "app/whisper-worker-cpu.exe", "a16a5452610ab1dae8af349b2d97d1eea36f44a263125a15fd884288a3ec960b"),
    ("app/whisper-worker-gpu.exe", "app/whisper-worker-gpu.exe", "d874136c6d9561ca06a80e7c2b150513e10a05e46e37edd4cb28ba6eaabd17a9"),
    ("app/msvcp140.dll", "app/msvcp140.dll", "0f885b509a685d2bbfa652fed26b5fb31d88fbdab0a978c641d1c7b8aa460aa9"),
    ("app/vcruntime140.dll", "app/vcruntime140.dll", "d5e4d9a3e835fa679450145d6a7d94e36573a509317111904d9b3712c30d9066"),
    ("app/vcruntime140_1.dll", "app/vcruntime140_1.dll", "1f2d41c4aa5db0bc33ebf7b66d72943a817d7ce6cbe880502a9403823633093f"),
    ("app/cudart64_12.dll", "app/cudart64_12.dll", "760c38928bbe5759f7b31ed6692599eb7ec83cedd5702e84c2b72028a89837e1"),
    ("app/cublas64_12.dll", "app/cublas64_12.dll", "52ce5ba0ec5327d39be6021f0f9362d2ed4d64d7206f9b5afc06398d827b8e82"),
    ("app/cublasLt64_12.dll", "app/cublasLt64_12.dll", "71705a7cf0923f4ab034d0d2620bbfc989ff669ac3d79f868b7a7c597ca559c5"),
    ("notices-candidate/plan-implementation-distribution-cpu-x64.md", "notices/plan-implementation-distribution-cpu-x64.md", "73b64807815e5df4c010690a66dcd292899f924c4a0e1793ac684fa6831dcd78"),
    ("notices-candidate/bilan-l2-distribution-turbo.md", "notices/bilan-l2-distribution-turbo.md", "2a17257a7123248b58fa63cb0051d629e0785ffd4d34bd105b3136011b5ded48"),
    ("notices-candidate/inventaire-contrat-installation-turbo.md", "notices/inventaire-contrat-installation-turbo.md", "48aec306a89c4b6365f8ca41eb68d3219ac14b58473581c4c6a1a5c3a1e14983"),
    ("notices-candidate/index-notices-turbo.md", "notices/index-notices-turbo.md", "63b77f7b7a3106ff800d08014ac9f7971a43de6512ba74741f721e43e7af68f3"),
    ("notices-candidate/recueil-licences-candidates-turbo.md", "notices/recueil-licences-candidates-turbo.md", "cb78a991c2225d59f05a88593c422b65a0cea4d009ee9bb9c50e715fdf4b04ee"),
    ("notices-candidate/notices-complementaires-turbo.md", "notices/notices-complementaires-turbo.md", "190dabab50fc91dfd3d17b6428c7379b003d266878bd20586e4b53a54ee65068"),
    ("notices-candidate/notices-cuda-entetes.md", "notices/notices-cuda-entetes.md", "4872d1ffa0cbbdfc51ba783c877b1c2cfd305feef72e7f61f1fdc82cb10a5731"),
    ("notices-candidate/LICENSE-renamest-MIT", "notices/LICENSE-renamest-MIT", "4db8d464c5b0810ab9674735e1133d2713561bfe92986152abc4d35bc3195d64"),
];

pub(super) fn bundle_assets() -> Vec<BundleAsset> {
    if option_env!("WHISPER_DISTRIBUTION_EDITION_DUAL").is_some() {
        return asset_records(DUAL_FILES);
    }
    TURBO_FILES
        .iter()
        .map(|(source, destination, sha256)| BundleAsset {
            source: PathBuf::from(source),
            destination: PathBuf::from(destination),
            sha256: (*sha256).to_owned(),
            label: Path::new(source)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("resource"),
        })
        .collect()
}

fn asset_records(records: &[&'static str]) -> Vec<BundleAsset> {
    records
        .iter()
        .copied()
        .map(|record| {
            let mut fields = record.split('|');
            let source = fields.next().expect("asset source is declared");
            let destination = fields.next().expect("asset destination is declared");
            let identity = fields.next().expect("asset identity is declared");
            let sha256 = compiled_asset_hash(identity).unwrap_or_default();
            BundleAsset {
                source: PathBuf::from(source),
                destination: PathBuf::from(destination),
                sha256: sha256.to_owned(),
                label: Path::new(source)
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("resource"),
            }
        })
        .collect()
}

fn compiled_asset_hash(identity: &'static str) -> Option<&'static str> {
    match identity {
        "WHISPER_DUAL_BOOTSTRAP_DESKTOP_SHA256" => {
            option_env!("WHISPER_DUAL_BOOTSTRAP_DESKTOP_SHA256")
        }
        "WHISPER_DUAL_BOOTSTRAP_TURBO_CPU_SHA256" => {
            option_env!("WHISPER_DUAL_BOOTSTRAP_TURBO_CPU_SHA256")
        }
        "WHISPER_DUAL_BOOTSTRAP_PARAKEET_CPU_SHA256" => {
            option_env!("WHISPER_DUAL_BOOTSTRAP_PARAKEET_CPU_SHA256")
        }
        value => Some(value),
    }
}
