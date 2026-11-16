use super::{DxgiLogEntry, InventoryLogSnapshot, append_inventory_log};
use std::path::PathBuf;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) enum InventoryLogDestination {
    #[default]
    Default,
    Explicit(PathBuf),
    Disabled,
}

pub(super) fn log_inventory(
    snapshot: InventoryLogSnapshot<'_>,
    destination: &InventoryLogDestination,
) {
    let path = match destination {
        InventoryLogDestination::Explicit(path) => path.clone(),
        InventoryLogDestination::Disabled => return,
        InventoryLogDestination::Default => {
            let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") else {
                let message = "GPU inventory log unavailable: LOCALAPPDATA is unset".to_owned();
                eprintln!("{message}");
                return;
            };
            PathBuf::from(local_app_data)
                .join("Whisper")
                .join("gpu-inventory.log")
        }
    };
    if let Err(error) = append_inventory_log(&path, snapshot) {
        let message = format!(
            "GPU inventory log append failed ({}): {error}",
            path.display()
        );
        eprintln!("{message}");
    }
}
