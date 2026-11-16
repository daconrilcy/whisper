use super::*;

pub(super) fn guarded_application_for_edition<P: ImportIoPort>(
    io: P,
    model_root: &std::path::Path,
    historical_model_path: String,
    admission_catalog: Receiver<CatalogSnapshot>,
    turbo_edition: bool,
) -> DesktopAdmissionApplication<P> {
    let app_model_path = model_root
        .join(
            model_registry::model(whisper_core::model_catalog::ModelId::WhisperLargeV3Turbo)
                .artifact
                .file_name,
        )
        .to_string_lossy()
        .into_owned();
    let accepted_paths = distribution_edition::accepted_paths(
        model_root,
        app_model_path,
        &historical_model_path,
        turbo_edition,
    );
    let core_guard = ModelCommandGuard::new_with_model_paths(
        model_registry::engine_snapshot(1, |_| ModelAvailability::Ready, Vec::new()),
        1,
        accepted_paths,
    );
    DesktopAdmissionApplication {
        inner: ImportApplication::new(io).with_model_guard(core_guard),
        admission_catalog,
        admission_guard: None,
        admission_catalog_failed: false,
        historical_model_path,
        admission_notice: None,
        inspection_requests: None,
        admission_generation: 0,
        inspection_interests: std::collections::BTreeMap::new(),
        visible_inspection: None,
        inspection_sequence: 0,
        admission_snapshot: model_catalog::uninspected_snapshot(model_root, 1),
        pending_gpu_candidate: None,
        turbo_edition,
    }
}
