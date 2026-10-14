use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use whisper_core::gpu_inventory::{GpuObservation, GpuPresence, GpuState};
use whisper_core::gpu_inventory_app::{GpuCollection, GpuInventoryPort};

const DISPLAY_CLASS_GUID: &str = "{4d36e968-e325-11ce-bfc1-08002be10318}";

fn gpu_query() -> String {
    format!(
        "SELECT DeviceID, Name, ConfigManagerErrorCode, Present FROM Win32_PnPEntity WHERE ClassGuid = '{DISPLAY_CLASS_GUID}'"
    )
}

pub struct WindowsGpuInventoryPort {
    receiver: Option<Receiver<(u64, Result<GpuCollection, String>)>>,
    generation: Option<u64>,
    collecting: Arc<AtomicBool>,
}

pub fn map_device_fields(
    identity: Option<String>,
    name: Option<String>,
    config_manager_error_code: Option<u32>,
    present: Option<bool>,
) -> GpuObservation {
    let state = match config_manager_error_code {
        Some(0) => Some(GpuState::Ready),
        Some(22) => Some(GpuState::Disabled),
        Some(code) => Some(GpuState::Error(code)),
        None => None,
    };
    let presence = if identity.as_deref().is_none_or(str::is_empty) {
        GpuPresence::Unknown
    } else {
        match present {
            Some(true) => GpuPresence::Current,
            Some(false) => GpuPresence::Absent,
            None => GpuPresence::Unknown,
        }
    };
    GpuObservation {
        identity: identity.unwrap_or_default(),
        name,
        state,
        presence,
    }
}

impl Default for WindowsGpuInventoryPort {
    fn default() -> Self {
        Self {
            receiver: None,
            generation: None,
            collecting: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl GpuInventoryPort for WindowsGpuInventoryPort {
    fn start(&mut self, generation: u64) -> Result<(), String> {
        if self.collecting.swap(true, Ordering::AcqRel) {
            return Err("Une collecte GPU est déjà en cours.".into());
        }
        let (sender, receiver) = mpsc::channel();
        self.receiver = Some(receiver);
        self.generation = Some(generation);
        let collecting = Arc::clone(&self.collecting);
        std::thread::Builder::new()
            .name("whisper-gpu-inventory".into())
            .spawn(move || {
                let result = collect_windows();
                let _ = sender.send((generation, result));
                collecting.store(false, Ordering::Release);
            })
            .map_err(|error| {
                self.collecting.store(false, Ordering::Release);
                format!("Impossible de démarrer la collecte GPU : {error}")
            })?;
        Ok(())
    }

    fn poll(&mut self) -> Option<(u64, Result<GpuCollection, String>)> {
        match self.receiver.as_ref()?.try_recv() {
            Ok(result) => {
                self.receiver = None;
                self.generation = None;
                Some(result)
            }
            Err(TryRecvError::Empty) => None,
            Err(TryRecvError::Disconnected) => {
                self.receiver = None;
                Some((
                    self.generation.take()?,
                    Err("La collecte GPU s’est interrompue.".into()),
                ))
            }
        }
    }
}

#[cfg(windows)]
fn collect_windows() -> Result<GpuCollection, String> {
    use wmi::{Variant, WMIConnection};

    let connection = WMIConnection::new().map_err(|error| format!("WMI indisponible : {error}"))?;
    let rows = connection
        .exec_query(gpu_query())
        .map_err(|error| format!("Énumération WMI des cartes graphiques impossible : {error}"))?;
    let mut observations = Vec::new();
    let mut error = None;
    for row in rows {
        let row = match row {
            Ok(row) => row,
            Err(query_error) => {
                error = Some(format!("Énumération WMI interrompue : {query_error}"));
                break;
            }
        };
        observations.push(map_device_fields(
            row.get_property("DeviceID")
                .ok()
                .and_then(|value| variant_string(&value)),
            row.get_property("Name")
                .ok()
                .and_then(|value| variant_string(&value)),
            row.get_property("ConfigManagerErrorCode")
                .ok()
                .and_then(|value| variant_u32(&value)),
            row.get_property("Present")
                .ok()
                .and_then(|value| match value {
                    Variant::Bool(value) => Some(value),
                    _ => None,
                }),
        ));
    }
    Ok(GpuCollection {
        observations,
        error,
    })
}

#[cfg(not(windows))]
fn collect_windows() -> Result<GpuCollection, String> {
    Err("L’inventaire WMI des GPU est disponible uniquement sous Windows.".into())
}

#[cfg(windows)]
fn variant_string(value: &wmi::Variant) -> Option<String> {
    match value {
        wmi::Variant::String(value) if !value.trim().is_empty() => Some(value.clone()),
        _ => None,
    }
}

#[cfg(windows)]
fn variant_u32(value: &wmi::Variant) -> Option<u32> {
    match value {
        wmi::Variant::UI1(value) => Some(u32::from(*value)),
        wmi::Variant::UI2(value) => Some(u32::from(*value)),
        wmi::Variant::UI4(value) => Some(*value),
        wmi::Variant::I1(value) => u32::try_from(*value).ok(),
        wmi::Variant::I2(value) => u32::try_from(*value).ok(),
        wmi::Variant::I4(value) => u32::try_from(*value).ok(),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{DISPLAY_CLASS_GUID, gpu_query};

    #[test]
    fn query_uses_the_display_class_guid_without_filtering_current_presence() {
        let query = gpu_query();
        assert!(query.contains(&format!("ClassGuid = '{DISPLAY_CLASS_GUID}'")));
        assert!(!query.contains("PNPClass"));
        let where_clause = query.split_once("WHERE").unwrap().1;
        assert!(!where_clause.contains("Present"));
    }
}
