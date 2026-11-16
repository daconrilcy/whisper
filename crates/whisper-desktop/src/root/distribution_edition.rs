use std::path::{Path, PathBuf};
use whisper_adapters::model_catalog::CatalogSnapshot;
use whisper_core::model_catalog::ModelId;

pub(super) const TURBO_DISTRIBUTION: bool = whisper_desktop::TURBO_EDITION;

pub(super) fn turbo_install_root(local_app_data: Option<std::ffi::OsString>) -> Option<PathBuf> {
    local_app_data
        .map(PathBuf::from)
        .map(|root| root.join("WhisperTurbo"))
}

pub(super) fn settings_path(root: &Path) -> PathBuf {
    root.join("settings/settings.json")
}

pub(super) fn log_paths(root: &Path) -> (PathBuf, PathBuf) {
    (
        root.join("logs/reactivity.log"),
        root.join("logs/gpu-inventory.log"),
    )
}

pub(super) fn model_root(
    turbo_root: Option<&Path>,
    personal_root: Option<&Path>,
    personal_install: bool,
    repository_root: PathBuf,
    turbo_distribution: bool,
) -> PathBuf {
    if turbo_distribution {
        turbo_root
            .map(|root| root.join("models"))
            .unwrap_or_else(|| PathBuf::from("\0"))
    } else if personal_install {
        personal_root
            .map(|root| root.join("models"))
            .unwrap_or_else(|| PathBuf::from("\0"))
    } else {
        repository_root.join("models")
    }
}

pub(super) fn worker_path(
    turbo_root: Option<&Path>,
    executable: Option<&Path>,
    worker_name: &str,
    personal_install: bool,
    turbo_distribution: bool,
) -> PathBuf {
    if turbo_distribution {
        return turbo_root
            .map(|root| root.join("app").join(worker_name))
            .unwrap_or_else(|| PathBuf::from("\0"));
    }
    let sibling = executable.and_then(|path| path.parent().map(|parent| parent.join(worker_name)));
    if personal_install {
        return sibling
            .clone()
            .filter(|path| path.is_file())
            .or(sibling)
            .unwrap_or_else(|| PathBuf::from(worker_name));
    }
    let isolated = executable.and_then(|path| {
        let release = path.parent()?;
        let target = release.parent()?;
        let package_target = target.parent()?;
        Some(
            package_target
                .parent()?
                .join("cpu")
                .join(target.file_name()?)
                .join(release.file_name()?)
                .join(worker_name),
        )
    });
    [sibling.clone(), isolated]
        .into_iter()
        .flatten()
        .find(|path| path.is_file())
        .or(sibling)
        .unwrap_or_else(|| PathBuf::from(worker_name))
}

pub(super) fn default_model_path(
    personal_install: bool,
    model_root: &Path,
    generic_default: String,
    turbo_distribution: bool,
) -> String {
    if turbo_distribution || personal_install {
        model_root
            .join(
                whisper_core::model_registry::model(ModelId::WhisperLargeV3Turbo)
                    .artifact
                    .file_name,
            )
            .to_string_lossy()
            .into_owned()
    } else {
        generic_default
    }
}

pub(super) fn admission_paths(
    snapshot: &CatalogSnapshot,
    historical_model_path: &str,
    turbo_distribution: bool,
) -> Vec<(ModelId, String)> {
    let mut paths = snapshot
        .entries
        .iter()
        .filter(|entry| !turbo_distribution || entry.id != ModelId::ParakeetTdt06BV3)
        .map(|entry| (entry.id, entry.path.to_string_lossy().into_owned()))
        .collect::<Vec<_>>();
    if !turbo_distribution {
        paths.push((
            ModelId::WhisperLargeV3Turbo,
            historical_model_path.to_owned(),
        ));
    }
    paths
}

pub(super) fn accepted_paths(
    model_root: &Path,
    app_model_path: String,
    historical_model_path: &str,
    turbo_distribution: bool,
) -> Vec<(ModelId, String)> {
    let mut paths = vec![(ModelId::WhisperLargeV3Turbo, app_model_path)];
    if !turbo_distribution {
        paths.push((
            ModelId::WhisperLargeV3Turbo,
            historical_model_path.to_owned(),
        ));
        paths.push((
            ModelId::ParakeetTdt06BV3,
            model_root
                .join(
                    whisper_core::model_registry::model(ModelId::ParakeetTdt06BV3)
                        .artifact
                        .file_name,
                )
                .to_string_lossy()
                .into_owned(),
        ));
    }
    paths
}

pub(super) fn preferred_model(preferred: Option<ModelId>) -> Option<ModelId> {
    preferred_model_for_edition(preferred, TURBO_DISTRIBUTION)
}

pub(super) fn preferred_model_for_edition(
    preferred: Option<ModelId>,
    turbo_distribution: bool,
) -> Option<ModelId> {
    if turbo_distribution && preferred == Some(ModelId::ParakeetTdt06BV3) {
        Some(ModelId::WhisperLargeV3Turbo)
    } else {
        preferred
    }
}

pub(super) fn ignores_parakeet_preference(preferred: Option<ModelId>) -> bool {
    ignores_parakeet_preference_for_edition(preferred, TURBO_DISTRIBUTION)
}

pub(super) fn ignores_parakeet_preference_for_edition(
    preferred: Option<ModelId>,
    turbo_distribution: bool,
) -> bool {
    turbo_distribution && preferred == Some(ModelId::ParakeetTdt06BV3)
}
