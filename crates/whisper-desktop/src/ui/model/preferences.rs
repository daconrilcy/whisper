use super::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelPreferenceCommand {
    ChooseModel(ModelId),
    ChooseStartupCompute(whisper_core::path_settings::StartupComputePreference),
}

pub enum ModelPreferenceEvent {
    Initialized {
        model: Option<ModelId>,
        startup_compute: whisper_core::path_settings::StartupComputePreference,
        notice: Option<String>,
    },
    Saved {
        revision: u64,
        message: String,
    },
}

pub struct ModelSelectionRuntime {
    pub catalog: Receiver<CatalogSnapshot>,
    pub preference_events: Receiver<ModelPreferenceEvent>,
    pub preference_commands: ModelPreferenceCommandSender,
}

impl ModelSelectionRuntime {
    pub fn inspection_requests(&self) -> Option<CatalogInspectionSender> {
        self.preference_commands.inspection.clone()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CatalogInspectionNeed {
    pub model: ModelId,
    pub version: u64,
}

#[derive(Default)]
struct InspectionMailbox {
    selection: Option<(ModelId, u64)>,
    latest_selection: Option<(ModelId, u64)>,
    commands: std::collections::VecDeque<ModelId>,
    command_pending: std::collections::BTreeSet<ModelId>,
    command_interest: std::collections::BTreeSet<ModelId>,
    versions: [u64; 2],
    published_versions: [u64; 2],
    published_generation: u64,
    observed_generations: [u64; 2],
    closed: bool,
}

fn model_index(model: ModelId) -> usize {
    match model {
        ModelId::WhisperLargeV3Turbo => 0,
        ModelId::ParakeetTdt06BV3 => 1,
    }
}

#[derive(Clone)]
pub struct CatalogInspectionSender {
    pending: Arc<Mutex<InspectionMailbox>>,
    wake: SyncSender<()>,
}

pub struct CatalogInspectionReceiver {
    pending: Arc<Mutex<InspectionMailbox>>,
    wake: Receiver<()>,
}

pub fn catalog_inspection_channel() -> (CatalogInspectionSender, CatalogInspectionReceiver) {
    let (wake_tx, wake_rx) = std::sync::mpsc::sync_channel(1);
    let pending = Arc::new(Mutex::new(InspectionMailbox::default()));
    (
        CatalogInspectionSender {
            pending: Arc::clone(&pending),
            wake: wake_tx,
        },
        CatalogInspectionReceiver {
            pending,
            wake: wake_rx,
        },
    )
}

impl CatalogInspectionSender {
    fn wake(&self) -> Result<(), PreferenceChannelError> {
        match self.wake.try_send(()) {
            Ok(()) | Err(TrySendError::Full(())) => Ok(()),
            Err(TrySendError::Disconnected(())) => Err(PreferenceChannelError::Disconnected),
        }
    }

    pub fn select(&self, model: ModelId, revision: u64) -> Result<(), PreferenceChannelError> {
        {
            let mut pending = self
                .pending
                .lock()
                .map_err(|_| PreferenceChannelError::Poisoned)?;
            if pending.closed {
                return Err(PreferenceChannelError::Disconnected);
            }
            if pending
                .latest_selection
                .is_some_and(|(_, current)| revision <= current)
            {
                return Ok(());
            }
            pending.versions[model_index(model)] =
                pending.versions[model_index(model)].saturating_add(1);
            pending.selection = Some((model, revision));
            pending.latest_selection = pending.selection;
        }
        self.wake()
    }

    pub fn command(&self, model: ModelId) -> Result<u64, PreferenceChannelError> {
        let version = {
            let mut pending = self
                .pending
                .lock()
                .map_err(|_| PreferenceChannelError::Poisoned)?;
            if pending.closed {
                return Err(PreferenceChannelError::Disconnected);
            }
            pending.command_interest.insert(model);
            if pending.command_pending.insert(model) {
                pending.versions[model_index(model)] =
                    pending.versions[model_index(model)].saturating_add(1);
                pending.commands.push_back(model);
            }
            pending.versions[model_index(model)]
        };
        self.wake()?;
        Ok(version)
    }

    /// Rejects superseded publications and removes observations invalidated by a newer need.
    pub fn refresh_snapshot(&self, snapshot: &mut CatalogSnapshot) -> bool {
        let Ok(pending) = self.pending.lock() else {
            return false;
        };
        if pending.closed || pending.published_generation != snapshot.generation {
            return false;
        }
        for entry in &mut snapshot.entries {
            let index = model_index(entry.id);
            if pending.published_versions[index] != pending.versions[index]
                || snapshot.generation < pending.observed_generations[index]
            {
                entry.availability = whisper_core::model_catalog::ModelAvailability::Uninspected;
                entry.observed_size_bytes = None;
                entry.observed_sha256 = None;
                entry.diagnostic = Some("Vérification du modèle en cours…".into());
            }
        }
        true
    }

    pub fn model_is_current(&self, model: ModelId, generation: u64) -> bool {
        self.pending.lock().is_ok_and(|pending| {
            let index = model_index(model);
            !pending.closed
                && pending.published_versions[index] == pending.versions[index]
                && generation >= pending.observed_generations[index]
        })
    }

    pub fn observation_completes(&self, model: ModelId, version: u64, generation: u64) -> bool {
        self.pending.lock().is_ok_and(|pending| {
            let index = model_index(model);
            !pending.closed
                && pending.published_versions[index] >= version
                && pending.published_versions[index] == pending.versions[index]
                && generation >= pending.observed_generations[index]
        })
    }

    pub fn selected_model(&self) -> Option<ModelId> {
        self.pending
            .lock()
            .ok()
            .and_then(|pending| pending.latest_selection.map(|(model, _)| model))
    }

    pub fn command_finished(&self, model: ModelId) {
        if let Ok(mut pending) = self.pending.lock() {
            pending.command_interest.remove(&model);
            pending.command_pending.remove(&model);
            pending.commands.retain(|pending| *pending != model);
        }
    }

    pub fn close(&self) {
        if let Ok(mut pending) = self.pending.lock() {
            pending.closed = true;
        }
        let _ = self.wake();
    }
}

impl CatalogInspectionReceiver {
    pub fn need_is_requested(
        &self,
        need: CatalogInspectionNeed,
    ) -> Result<bool, PreferenceChannelError> {
        let pending = self
            .pending
            .lock()
            .map_err(|_| PreferenceChannelError::Poisoned)?;
        Ok(!pending.closed
            && pending.versions[model_index(need.model)] == need.version
            && (pending.command_interest.contains(&need.model)
                || pending
                    .latest_selection
                    .is_some_and(|(model, _)| model == need.model)))
    }

    pub fn need_is_current(
        &self,
        need: CatalogInspectionNeed,
    ) -> Result<bool, PreferenceChannelError> {
        let pending = self
            .pending
            .lock()
            .map_err(|_| PreferenceChannelError::Poisoned)?;
        Ok(!pending.closed && pending.versions[model_index(need.model)] == need.version)
    }
    pub fn next(
        &self,
        timeout: std::time::Duration,
    ) -> Result<Option<CatalogInspectionNeed>, PreferenceChannelError> {
        if let Some(need) = self.take_pending()? {
            return Ok(Some(need));
        }
        match self.wake.recv_timeout(timeout) {
            Ok(()) | Err(std::sync::mpsc::RecvTimeoutError::Timeout) => self.take_pending(),
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                Err(PreferenceChannelError::Disconnected)
            }
        }
    }

    fn take_pending(&self) -> Result<Option<CatalogInspectionNeed>, PreferenceChannelError> {
        let mut pending = self
            .pending
            .lock()
            .map_err(|_| PreferenceChannelError::Poisoned)?;
        if pending.closed {
            return Err(PreferenceChannelError::Disconnected);
        }
        let model = if let Some(model) = pending.commands.pop_front() {
            if pending
                .selection
                .is_some_and(|(selected, _)| selected == model)
            {
                pending.selection = None;
            }
            Some(model)
        } else {
            pending.selection.take().map(|(id, _)| id)
        };
        Ok(model.map(|model| CatalogInspectionNeed {
            model,
            version: pending.versions[model_index(model)],
        }))
    }

    pub fn interests(&self) -> Result<Vec<CatalogInspectionNeed>, PreferenceChannelError> {
        let pending = self
            .pending
            .lock()
            .map_err(|_| PreferenceChannelError::Poisoned)?;
        if pending.closed {
            return Err(PreferenceChannelError::Disconnected);
        }
        let mut models = pending.command_interest.clone();
        if let Some((model, _)) = pending.latest_selection {
            models.insert(model);
        }
        Ok(models
            .into_iter()
            .map(|model| CatalogInspectionNeed {
                model,
                version: pending.versions[model_index(model)],
            })
            .collect())
    }

    pub fn publish(
        &self,
        need: Option<CatalogInspectionNeed>,
        generation: u64,
        observed_versions: [u64; 2],
    ) -> Result<bool, PreferenceChannelError> {
        let mut pending = self
            .pending
            .lock()
            .map_err(|_| PreferenceChannelError::Poisoned)?;
        if pending.closed
            || need.is_some_and(|need| pending.versions[model_index(need.model)] != need.version)
        {
            return Ok(false);
        }
        if let Some(need) = need {
            pending.command_pending.remove(&need.model);
        }
        for (index, version) in observed_versions.iter().enumerate() {
            if *version != pending.published_versions[index] {
                pending.observed_generations[index] = generation;
            }
        }
        pending.published_versions = observed_versions;
        pending.published_generation = generation;
        Ok(true)
    }
}

#[derive(Default)]
struct PendingPreferenceChoices {
    model: Option<ModelId>,
    startup_compute: Option<whisper_core::path_settings::StartupComputePreference>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreferenceChannelError {
    Poisoned,
    Disconnected,
}

impl std::fmt::Display for PreferenceChannelError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Poisoned => formatter.write_str("preference mailbox lock poisoned"),
            Self::Disconnected => formatter.write_str("preference channel disconnected"),
        }
    }
}

impl std::error::Error for PreferenceChannelError {}

/// Bounded wake channel with a coalescing latest-choice mailbox for preference persistence.
#[derive(Clone)]
pub struct ModelPreferenceCommandSender {
    pending: Arc<Mutex<PendingPreferenceChoices>>,
    wake: SyncSender<()>,
    inspection: Option<CatalogInspectionSender>,
    selection_revision: Arc<AtomicU64>,
}

pub struct ModelPreferenceCommandReceiver {
    pending: Arc<Mutex<PendingPreferenceChoices>>,
    wake: Receiver<()>,
}

pub fn model_preference_command_channel()
-> (ModelPreferenceCommandSender, ModelPreferenceCommandReceiver) {
    let (wake_tx, wake_rx) = std::sync::mpsc::sync_channel(1);
    let pending = Arc::new(Mutex::new(PendingPreferenceChoices::default()));
    (
        ModelPreferenceCommandSender {
            pending: pending.clone(),
            wake: wake_tx,
            inspection: None,
            selection_revision: Arc::new(AtomicU64::new(0)),
        },
        ModelPreferenceCommandReceiver {
            pending,
            wake: wake_rx,
        },
    )
}

impl ModelPreferenceCommandSender {
    pub fn with_inspection_requests(mut self, inspection: CatalogInspectionSender) -> Self {
        self.inspection = Some(inspection);
        self
    }

    pub fn inspect_selection(&self, model: ModelId) -> Result<(), PreferenceChannelError> {
        if let Some(inspection) = &self.inspection {
            let revision = self
                .selection_revision
                .fetch_add(1, Ordering::Relaxed)
                .saturating_add(1);
            inspection.select(model, revision)?;
        }
        Ok(())
    }

    pub fn send(&self, command: ModelPreferenceCommand) -> Result<(), PreferenceChannelError> {
        if let ModelPreferenceCommand::ChooseModel(model) = &command {
            self.inspect_selection(*model)?;
        }
        let mut pending = self
            .pending
            .lock()
            .map_err(|_| PreferenceChannelError::Poisoned)?;
        match command {
            ModelPreferenceCommand::ChooseModel(model) => pending.model = Some(model),
            ModelPreferenceCommand::ChooseStartupCompute(choice) => {
                pending.startup_compute = Some(choice)
            }
        }
        match self.wake.try_send(()) {
            Ok(()) | Err(TrySendError::Full(())) => Ok(()),
            Err(TrySendError::Disconnected(())) => Err(PreferenceChannelError::Disconnected),
        }
    }
}

impl ModelPreferenceCommandReceiver {
    pub fn recv(&self) -> Result<Vec<ModelPreferenceCommand>, PreferenceChannelError> {
        self.wake
            .recv()
            .map_err(|_| PreferenceChannelError::Disconnected)?;
        let mut pending = self
            .pending
            .lock()
            .map_err(|_| PreferenceChannelError::Poisoned)?;
        let mut commands = Vec::with_capacity(2);
        if let Some(model) = pending.model.take() {
            commands.push(ModelPreferenceCommand::ChooseModel(model));
        }
        if let Some(choice) = pending.startup_compute.take() {
            commands.push(ModelPreferenceCommand::ChooseStartupCompute(choice));
        }
        Ok(commands)
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SelectionWidgetState {
    pub model: Option<ModelId>,
    pub manual_language: Option<String>,
}
