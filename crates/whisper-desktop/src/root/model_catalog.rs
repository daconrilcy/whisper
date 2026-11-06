use super::*;
use std::{collections::BTreeMap, sync::mpsc::Sender};
use whisper_desktop::ui::{
    CatalogInspectionNeed, CatalogInspectionReceiver, CatalogInspectionSender,
    PreferenceChannelError, catalog_inspection_channel,
};

pub(super) fn uninspected_snapshot(root: &std::path::Path, generation: u64) -> CatalogSnapshot {
    CatalogSnapshot {
        generation,
        entries: model_registry::MODELS
            .iter()
            .map(|record| whisper_adapters::model_catalog::CatalogEntry {
                id: record.artifact.id,
                display_name: record.display_name.to_owned(),
                path: root.join(record.artifact.file_name),
                availability: ModelAvailability::Uninspected,
                expected_size_bytes: record.artifact.size_bytes,
                observed_size_bytes: None,
                expected_sha256: record.artifact.sha256.to_owned(),
                observed_sha256: None,
                diagnostic: Some("Modèle non vérifié.".into()),
            })
            .collect(),
    }
}

pub(super) fn start_catalog_service(
    root: PathBuf,
) -> (
    CatalogInspectionSender,
    Receiver<CatalogSnapshot>,
    Receiver<CatalogSnapshot>,
) {
    start_catalog_service_with(root, |catalog, id, generation| {
        let mut snapshot = catalog.inspect_model(id, generation);
        if id == ModelId::ParakeetTdt06BV3 {
            qualify_parakeet_catalog_entry(&mut snapshot);
        }
        snapshot
    })
}

fn start_catalog_service_with(
    root: PathBuf,
    inspect: impl FnMut(&LocalModelCatalog, ModelId, u64) -> CatalogSnapshot + Send + 'static,
) -> (
    CatalogInspectionSender,
    Receiver<CatalogSnapshot>,
    Receiver<CatalogSnapshot>,
) {
    let (requests, receiver) = catalog_inspection_channel();
    let (catalog_tx, catalog_rx) = mpsc::channel();
    let (admission_tx, admission_rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut service = CatalogService {
            catalog: LocalModelCatalog::new(&root),
            snapshot: uninspected_snapshot(&root, 1),
            observed_versions: BTreeMap::new(),
            observed_identities: BTreeMap::new(),
            gpu_route_metadata: None,
            receiver,
            outputs: [catalog_tx, admission_tx],
            inspect,
        };
        service.run();
    });
    (requests, catalog_rx, admission_rx)
}

struct CatalogService<F> {
    catalog: LocalModelCatalog,
    snapshot: CatalogSnapshot,
    observed_versions: BTreeMap<ModelId, u64>,
    observed_identities:
        BTreeMap<ModelId, Option<whisper_adapters::model_catalog::ArtifactFileIdentity>>,
    gpu_route_metadata: Option<Vec<whisper_adapters::model_catalog::ArtifactFileIdentity>>,
    receiver: CatalogInspectionReceiver,
    outputs: [Sender<CatalogSnapshot>; 2],
    inspect: F,
}

impl<F: FnMut(&LocalModelCatalog, ModelId, u64) -> CatalogSnapshot> CatalogService<F> {
    fn publish(&self, need: Option<CatalogInspectionNeed>) -> bool {
        let versions = ModelId::ALL.map(|id| self.observed_versions.get(&id).copied().unwrap_or(0));
        if !self
            .receiver
            .publish(need, self.snapshot.generation, versions)
            .unwrap_or(false)
        {
            return false;
        }
        self.outputs
            .iter()
            .all(|output| output.send(self.snapshot.clone()).is_ok())
    }

    fn run(&mut self) {
        if !self.publish(None) {
            return;
        }
        loop {
            match self.receiver.next(Duration::from_secs(1)) {
                Ok(Some(need)) => {
                    if self.observe(need).is_err() {
                        return;
                    }
                }
                Ok(None) => {
                    if self.refresh_interested().is_err() {
                        return;
                    }
                }
                Err(_) => return,
            }
        }
    }

    fn refresh_interested(&mut self) -> Result<(), PreferenceChannelError> {
        for need in self.receiver.interests()? {
            let inspected = self.snapshot.entries.iter().any(|entry| {
                entry.id == need.model
                    && entry.observed_sha256.as_deref() == Some(entry.expected_sha256.as_str())
                    && entry.observed_size_bytes == Some(entry.expected_size_bytes)
            });
            if inspected && self.needs_refresh(need.model) {
                self.observe(need)?;
            }
        }
        Ok(())
    }

    fn needs_refresh(&self, model: ModelId) -> bool {
        let record = model_registry::model(model);
        let identity = file_metadata(&self.catalog.root().join(record.artifact.file_name));
        if self.observed_identities.get(&model) != Some(&identity) {
            return true;
        }
        if model != ModelId::ParakeetTdt06BV3 {
            return false;
        }
        let cpu_qualified = configured_parakeet_worker()
            .as_ref()
            .is_some_and(parakeet_worker_runtime_is_qualified);
        let gpu_metadata = configured_gpu_parakeet_worker()
            .as_ref()
            .and_then(gpu_worker_file_metadata);
        cpu_qualified != parakeet_available(&self.snapshot)
            || gpu_metadata != self.gpu_route_metadata
    }

    fn observe(&mut self, need: CatalogInspectionNeed) -> Result<(), PreferenceChannelError> {
        if !self.receiver.need_is_requested(need)? {
            return Ok(());
        }
        let record = model_registry::model(need.model);
        let path = self.catalog.root().join(record.artifact.file_name);
        let before = file_metadata(&path);
        let generation = self.snapshot.generation.saturating_add(1);
        let observed = (self.inspect)(&self.catalog, need.model, generation);
        let after = file_metadata(&path);
        let mut entry = observed
            .entries
            .into_iter()
            .find(|entry| entry.id == need.model)
            .unwrap_or_else(|| {
                let mut entry = uninspected_snapshot(self.catalog.root(), generation)
                    .entries
                    .into_iter()
                    .find(|entry| entry.id == need.model)
                    .expect("closed model registry");
                entry.diagnostic =
                    Some("Vérification du modèle sans résultat ; relancez l'action.".into());
                entry
            });
        if before != after || (entry.availability == ModelAvailability::Ready && after.is_none()) {
            entry.availability = ModelAvailability::Uninspected;
            entry.observed_sha256 = None;
            entry.observed_size_bytes = None;
            entry.diagnostic = Some("Identité du modèle indisponible ou modifiée pendant sa vérification ; relancez l'action.".into());
        }
        if !self.receiver.need_is_current(need)? {
            return Ok(());
        }
        if self.observed_versions.get(&need.model) == Some(&need.version)
            && self
                .snapshot
                .entries
                .iter()
                .any(|current| current == &entry)
        {
            return Ok(());
        }
        let mut next = self.snapshot.clone();
        next.generation = generation;
        if let Some(current) = next
            .entries
            .iter_mut()
            .find(|current| current.id == need.model)
        {
            *current = entry;
        }
        let versions = ModelId::ALL.map(|id| {
            if id == need.model {
                need.version
            } else {
                self.observed_versions.get(&id).copied().unwrap_or(0)
            }
        });
        if !self.receiver.publish(Some(need), generation, versions)? {
            return Ok(());
        }
        self.observed_versions.insert(need.model, need.version);
        self.observed_identities.insert(need.model, after);
        if need.model == ModelId::ParakeetTdt06BV3 {
            self.gpu_route_metadata = configured_gpu_parakeet_worker()
                .as_ref()
                .and_then(gpu_worker_file_metadata);
        }
        self.snapshot = next;
        for output in &self.outputs {
            output
                .send(self.snapshot.clone())
                .map_err(|_| PreferenceChannelError::Disconnected)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
