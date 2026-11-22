use super::{InstallError, InstallRequest, MODEL_SHA256, assets, require_compatible_cpu, validate_inputs};
use std::path::{Path, PathBuf};
use whisper_core::cpu_compatibility::CpuCompatibility;

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
    require_compatible_cpu(compatibility)?;
    let request = InstallRequest {
        local_app_data: local_app_data.map(Path::to_path_buf),
        bundle_root: bundle_root.map(Path::to_path_buf),
        model_path: model_path.to_path_buf(),
        confirmed: true,
    };
    validate_inputs(&request, &assets::bundle_assets(), MODEL_SHA256).map(|_| ())
}
