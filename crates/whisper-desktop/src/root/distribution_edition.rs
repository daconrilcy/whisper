use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use whisper_adapters::model_catalog::CatalogSnapshot;
use whisper_core::model_catalog::ModelId;

pub(super) const TURBO_DISTRIBUTION: bool = whisper_desktop::TURBO_EDITION;
pub(super) const DUAL_DISTRIBUTION: bool =
    option_env!("WHISPER_DISTRIBUTION_EDITION_DUAL").is_some();
pub(super) const PACKAGED_DISTRIBUTION: bool = TURBO_DISTRIBUTION || DUAL_DISTRIBUTION;

#[derive(Clone, Debug)]
pub(super) struct RuntimeRoots {
    pub(super) assets: PathBuf,
    pub(super) data: PathBuf,
    pub(super) installation: PathBuf,
    pub(super) msix: bool,
}

pub(super) enum GpuInventoryLogRoute {
    Default,
    Disabled,
    Explicit(PathBuf),
}

static RUNTIME_ROOTS: OnceLock<Result<Option<RuntimeRoots>, String>> = OnceLock::new();

pub(super) fn current_runtime_roots() -> Result<Option<RuntimeRoots>, String> {
    if !PACKAGED_DISTRIBUTION {
        return Ok(None);
    }
    RUNTIME_ROOTS
        .get_or_init(|| {
            runtime_roots_from(
                whisper_adapters::path_settings::current_msix_local_data_root()?,
                std::env::var_os("LOCALAPPDATA").map(PathBuf::from),
                std::env::current_exe().ok().as_deref(),
            )
        })
        .clone()
}

pub(super) fn personal_root(roots: Option<&RuntimeRoots>) -> Option<PathBuf> {
    roots
        .filter(|roots| roots.msix)
        .map(|roots| roots.installation.clone())
}

pub(super) fn data_root(roots: Option<&RuntimeRoots>) -> Option<PathBuf> {
    roots.map(|roots| roots.data.clone())
}

pub(super) fn assets_root(roots: Option<&RuntimeRoots>) -> Option<PathBuf> {
    roots.map(|roots| roots.assets.clone())
}

pub(super) fn msix_cpu_parakeet_worker(
    roots: Option<&RuntimeRoots>,
) -> Option<whisper_adapters::parakeet_import::ParakeetWorkerPaths> {
    roots.filter(|roots| roots.msix).map(|roots| {
        whisper_adapters::parakeet_import::ParakeetWorkerPaths::from_msix_cpu_package(&roots.assets)
    })
}

pub(super) fn personal_model_root(root: &Path) -> PathBuf {
    root.join("models")
}

pub(super) fn personal_worker_paths(
    root: &Path,
    backend: whisper_core::model_catalog::Backend,
) -> whisper_adapters::parakeet_import::ParakeetWorkerPaths {
    let (worker, native) = match backend {
        whisper_core::model_catalog::Backend::Cpu => {
            ("whisper-worker-parakeet-cpu.exe", "native/cpu")
        }
        whisper_core::model_catalog::Backend::Gpu => {
            ("whisper-worker-parakeet-gpu.exe", "native/gpu")
        }
    };
    whisper_adapters::parakeet_import::ParakeetWorkerPaths {
        executable: root.join("worker").join(worker),
        native_dir: root.join(native),
        model: personal_model_root(root).join("parakeet-tdt-0.6b-v3.q8_0.gguf"),
        runtime_root: root.to_path_buf(),
    }
}

pub(super) fn runtime_roots_from(
    msix_local_folder: Option<PathBuf>,
    local_app_data: Option<PathBuf>,
    executable: Option<&Path>,
) -> Result<Option<RuntimeRoots>, String> {
    if let Some(local_folder) = msix_local_folder {
        if !local_folder.is_absolute() {
            return Err("MSIX LocalFolder path is not absolute".into());
        }
        let executable = executable.ok_or_else(|| "MSIX desktop path unavailable".to_owned())?;
        let app_directory = executable
            .parent()
            .filter(|path| path.file_name().is_some_and(|name| name == "app"))
            .ok_or_else(|| "MSIX desktop is not under the package app directory".to_owned())?;
        let package_root = app_directory
            .parent()
            .filter(|path| path.is_absolute())
            .ok_or_else(|| "MSIX package root unavailable".to_owned())?
            .to_path_buf();
        let data = local_folder.join("WhisperTurbo");
        return Ok(Some(RuntimeRoots {
            assets: package_root.clone(),
            data,
            installation: package_root,
            msix: true,
        }));
    }
    Ok(local_app_data.map(|local| {
        let root = local.join("WhisperTurbo");
        RuntimeRoots {
            assets: root.clone(),
            data: root.clone(),
            installation: root,
            msix: false,
        }
    }))
}

pub(super) fn gpu_inventory_log_route(
    data_root: Option<&Path>,
    msix: bool,
    turbo_distribution: bool,
) -> GpuInventoryLogRoute {
    if msix {
        return data_root.map_or(GpuInventoryLogRoute::Disabled, |root| {
            GpuInventoryLogRoute::Explicit(log_paths(root).1)
        });
    }
    if turbo_distribution {
        return data_root.map_or(GpuInventoryLogRoute::Disabled, |root| {
            GpuInventoryLogRoute::Explicit(log_paths(root).1)
        });
    }
    GpuInventoryLogRoute::Default
}

pub(super) fn turbo_install_root(local_app_data: Option<std::ffi::OsString>) -> Option<PathBuf> {
    local_app_data
        .map(PathBuf::from)
        .map(|root| root.join("WhisperTurbo"))
}

pub(super) fn private_install_root(
    local_app_data: Option<std::ffi::OsString>,
    dual_distribution: bool,
) -> Option<PathBuf> {
    local_app_data.map(|root| {
        PathBuf::from(root).join(if dual_distribution {
            "WhisperTurbo"
        } else {
            "Whisper"
        })
    })
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
    packaged_distribution: bool,
) -> PathBuf {
    if packaged_distribution {
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
