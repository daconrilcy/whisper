use crate::gpu_inventory::{
    GpuCatalogueCoverage, GpuInventory, GpuObservation, GpuQualifiedCatalogue,
    GpuQualifiedObservation, GpuRuntimeIdentity, normalize, qualify,
};

pub trait GpuInventoryPort {
    fn start(&mut self, generation: u64) -> Result<(), String>;
    fn poll(&mut self) -> Option<(u64, Result<GpuCollection, String>)>;

    fn uses_qualified_catalogue(&self) -> bool {
        false
    }

    fn poll_qualified(&mut self) -> Option<(u64, Result<GpuQualifiedCollection, String>)> {
        None
    }

    fn poll_runtime_identities(
        &mut self,
    ) -> Option<(u64, Result<Vec<GpuRuntimeIdentity>, String>)> {
        None
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GpuCollection {
    pub observations: Vec<GpuObservation>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GpuQualifiedCollection {
    pub observations: Vec<GpuQualifiedObservation>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GpuQualifiedInventoryState {
    NotRequested,
    Loading,
    Complete(GpuQualifiedCatalogue),
    Partial(GpuQualifiedCatalogue),
    Failed(String),
}

impl GpuQualifiedInventoryState {
    fn from_collection(collection: GpuQualifiedCollection) -> Self {
        let catalogue = qualify(collection.observations, collection.error.clone());
        match catalogue.coverage {
            GpuCatalogueCoverage::Complete => Self::Complete(catalogue),
            GpuCatalogueCoverage::Partial => Self::Partial(catalogue),
            GpuCatalogueCoverage::Failed => Self::Failed(
                collection
                    .error
                    .unwrap_or_else(|| "Collecte GPU échouée.".into()),
            ),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GpuInventoryState {
    NotRequested,
    Loading,
    Complete(GpuInventory),
    Partial(GpuInventory),
    Failed(String),
}

pub struct GpuInventoryApp {
    port: Box<dyn GpuInventoryPort>,
    generation: u64,
    state: GpuInventoryState,
    qualified_state: GpuQualifiedInventoryState,
    closed: bool,
    in_flight: bool,
    runtime_identities: Vec<GpuRuntimeIdentity>,
}

impl GpuInventoryApp {
    pub fn new(port: Box<dyn GpuInventoryPort>) -> Self {
        let mut app = Self {
            port,
            generation: 0,
            state: GpuInventoryState::NotRequested,
            qualified_state: GpuQualifiedInventoryState::NotRequested,
            closed: false,
            in_flight: false,
            runtime_identities: Vec::new(),
        };
        app.start();
        app
    }

    pub fn start(&mut self) {
        if self.in_flight {
            return;
        }
        let Some(generation) = self.generation.checked_add(1) else {
            self.state = GpuInventoryState::Failed("Génération GPU épuisée.".into());
            return;
        };
        self.generation = generation;
        self.runtime_identities.clear();
        self.closed = false;
        self.state = GpuInventoryState::Loading;
        self.qualified_state = if self.port.uses_qualified_catalogue() {
            GpuQualifiedInventoryState::Loading
        } else {
            GpuQualifiedInventoryState::NotRequested
        };
        match self.port.start(self.generation) {
            Ok(()) => self.in_flight = true,
            Err(error) => {
                self.state = GpuInventoryState::Failed(error.clone());
                if self.port.uses_qualified_catalogue() {
                    self.qualified_state = GpuQualifiedInventoryState::Failed(error);
                }
            }
        }
    }

    pub fn poll(&mut self) {
        if self.closed {
            return;
        }
        if let Some((generation, Ok(identities))) = self.port.poll_runtime_identities()
            && generation == self.generation
        {
            self.runtime_identities = identities;
        }
        if self.port.uses_qualified_catalogue() {
            let Some((generation, result)) = self.port.poll_qualified() else {
                return;
            };
            if generation != self.generation || !self.in_flight {
                return;
            }
            self.in_flight = false;
            match result {
                Ok(collection) => {
                    self.qualified_state = GpuQualifiedInventoryState::from_collection(collection);
                    self.state = legacy_view(&self.qualified_state);
                }
                Err(error) => {
                    self.state = GpuInventoryState::Failed(error.clone());
                    self.qualified_state = GpuQualifiedInventoryState::Failed(error);
                }
            }
            return;
        }
        let Some((generation, result)) = self.port.poll() else {
            return;
        };
        if generation != self.generation {
            return;
        }
        if !self.in_flight {
            return;
        }
        self.in_flight = false;
        match result {
            Ok(collection) => {
                let error = collection.error;
                let inventory = normalize(collection.observations, error.is_some());
                self.state = if let Some(error) = error {
                    if inventory.devices.is_empty() {
                        GpuInventoryState::Failed(error)
                    } else {
                        let mut inventory = inventory;
                        inventory.warnings.push(error);
                        GpuInventoryState::Partial(inventory)
                    }
                } else if inventory.coverage == crate::gpu_inventory::Coverage::Complete {
                    GpuInventoryState::Complete(inventory)
                } else {
                    GpuInventoryState::Partial(inventory)
                };
            }
            Err(error) => self.state = GpuInventoryState::Failed(error),
        }
    }

    pub fn close(&mut self) {
        self.closed = true;
        self.generation = self.generation.saturating_add(1);
        self.in_flight = false;
        self.state = GpuInventoryState::NotRequested;
        self.qualified_state = GpuQualifiedInventoryState::NotRequested;
    }

    pub fn state(&self) -> &GpuInventoryState {
        &self.state
    }

    pub fn qualified_state(&self) -> &GpuQualifiedInventoryState {
        &self.qualified_state
    }

    /// Runtime join evidence for the current inventory generation. This is
    /// intentionally in-memory only and is never part of settings persistence.
    pub fn runtime_identities(&self) -> &[GpuRuntimeIdentity] {
        &self.runtime_identities
    }

    pub fn resolve_selection(
        &self,
        mode: crate::domain::ComputeChoice,
        preferred_identity: Option<&str>,
    ) -> crate::gpu_selection::SelectionResolution {
        crate::gpu_selection::resolve(mode, preferred_identity, &self.qualified_state)
    }
}

fn legacy_view(state: &GpuQualifiedInventoryState) -> GpuInventoryState {
    let (catalogue, complete) = match state {
        GpuQualifiedInventoryState::Complete(catalogue) => (catalogue, true),
        GpuQualifiedInventoryState::Partial(catalogue) => (catalogue, false),
        GpuQualifiedInventoryState::Failed(error) => {
            return GpuInventoryState::Failed(error.clone());
        }
        GpuQualifiedInventoryState::NotRequested | GpuQualifiedInventoryState::Loading => {
            return GpuInventoryState::Loading;
        }
    };
    let inventory = GpuInventory {
        devices: catalogue
            .devices
            .iter()
            .map(|device| device.device.clone())
            .collect(),
        coverage: if complete {
            crate::gpu_inventory::Coverage::Complete
        } else {
            crate::gpu_inventory::Coverage::Partial
        },
        warnings: catalogue
            .causes
            .iter()
            .map(|cause| format!("{cause:?}"))
            .collect(),
    };
    if complete {
        GpuInventoryState::Complete(inventory)
    } else {
        GpuInventoryState::Partial(inventory)
    }
}
