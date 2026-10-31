use super::*;

impl<A: Application> DesktopApp<A> {
    pub(super) fn install_test_model_catalog(&mut self) {
        let entries = whisper_core::model_registry::MODELS
            .iter()
            .map(|record| whisper_adapters::model_catalog::CatalogEntry {
                id: record.artifact.id,
                display_name: record.display_name.to_owned(),
                path: std::path::PathBuf::from("test-models").join(record.artifact.file_name),
                availability: whisper_core::model_catalog::ModelAvailability::Ready,
                expected_size_bytes: record.artifact.size_bytes,
                observed_size_bytes: Some(record.artifact.size_bytes),
                expected_sha256: record.artifact.sha256.to_owned(),
                observed_sha256: Some(record.artifact.sha256.to_owned()),
                diagnostic: None,
            })
            .collect();
        let catalog = CatalogSnapshot {
            generation: 1,
            entries,
        };
        self.projection = ModelSelectionApp::new(catalog.engine_snapshot(vec![]))
            .projection(catalog.generation)
            .ok();
        self.catalog = Some(catalog);
    }
}
