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
        },
        ModelPreferenceCommandReceiver {
            pending,
            wake: wake_rx,
        },
    )
}

impl ModelPreferenceCommandSender {
    pub fn send(&self, command: ModelPreferenceCommand) -> Result<(), PreferenceChannelError> {
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
