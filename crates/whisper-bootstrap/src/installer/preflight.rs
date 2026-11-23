use super::{InstallError, InstallRequest, MODEL_SHA256, assets, require_compatible_cpu};
use std::path::{Path, PathBuf};
use whisper_core::cpu_compatibility::CpuCompatibility;

#[cfg(test)]
mod tests;

pub(crate) fn bundled_turbo_model(bundle_root: Option<&Path>) -> Result<PathBuf, InstallError> {
    let root = bundle_root.ok_or(InstallError::BundleUnavailable)?;
    let model = root.join("models").join(super::MODEL_FILE);
    super::validate_model(&model, MODEL_SHA256)
}

pub(crate) fn preflight_bundle(
    local_app_data: Option<&Path>,
    bundle_root: Option<&Path>,
    model_path: &Path,
    compatibility: &CpuCompatibility,
) -> Result<(), InstallError> {
    preflight_bundle_with_assets(
        local_app_data,
        bundle_root,
        model_path,
        compatibility,
        &assets::bundle_assets(),
        MODEL_SHA256,
        DestinationRequirement::MustBeAbsent,
    )
}

pub(crate) fn preflight_packaged_bundle(
    local_app_data: Option<&Path>,
    bundle_root: Option<&Path>,
    model_path: &Path,
    compatibility: &CpuCompatibility,
) -> Result<(), InstallError> {
    preflight_bundle_with_assets(
        local_app_data,
        bundle_root,
        model_path,
        compatibility,
        &assets::bundle_assets(),
        MODEL_SHA256,
        DestinationRequirement::MayExist,
    )
}

#[derive(Clone, Copy)]
enum DestinationRequirement {
    MustBeAbsent,
    MayExist,
}

fn preflight_bundle_with_assets(
    local_app_data: Option<&Path>,
    bundle_root: Option<&Path>,
    model_path: &Path,
    compatibility: &CpuCompatibility,
    bundle_assets: &[super::BundleAsset],
    model_sha256: &str,
    destination_requirement: DestinationRequirement,
) -> Result<(), InstallError> {
    require_compatible_cpu(compatibility)?;
    let request = InstallRequest {
        local_app_data: local_app_data.map(Path::to_path_buf),
        bundle_root: bundle_root.map(Path::to_path_buf),
        model_path: model_path.to_path_buf(),
        confirmed: true,
    };
    let local_app_data = super::validate_local_app_data(&request)?;
    if matches!(
        destination_requirement,
        DestinationRequirement::MustBeAbsent
    ) {
        super::require_install_destination_available(&local_app_data)?;
    }
    super::validate_bundle_resources(&request, bundle_assets, model_sha256).map(|_| ())
}
