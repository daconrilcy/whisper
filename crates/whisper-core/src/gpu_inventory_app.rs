use crate::gpu_inventory::{GpuInventory, GpuObservation, normalize};

pub trait GpuInventoryPort {
    fn start(&mut self, generation: u64) -> Result<(), String>;
    fn poll(&mut self) -> Option<(u64, Result<GpuCollection, String>)>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GpuCollection {
    pub observations: Vec<GpuObservation>,
    pub error: Option<String>,
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
    closed: bool,
}

impl GpuInventoryApp {
    pub fn new(port: Box<dyn GpuInventoryPort>) -> Self {
        let mut app = Self {
            port,
            generation: 0,
            state: GpuInventoryState::NotRequested,
            closed: false,
        };
        app.start();
        app
    }

    pub fn start(&mut self) {
        self.generation = self.generation.saturating_add(1);
        self.closed = false;
        self.state = GpuInventoryState::Loading;
        if let Err(error) = self.port.start(self.generation) {
            self.state = GpuInventoryState::Failed(error);
        }
    }

    pub fn poll(&mut self) {
        if self.closed {
            return;
        }
        let Some((generation, result)) = self.port.poll() else {
            return;
        };
        if generation != self.generation {
            return;
        }
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
        self.state = GpuInventoryState::NotRequested;
    }

    pub fn state(&self) -> &GpuInventoryState {
        &self.state
    }
}
