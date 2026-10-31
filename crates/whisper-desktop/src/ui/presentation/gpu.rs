use super::*;

pub(in crate::ui) fn parakeet_gpu_catalog_ready(catalog: &CatalogSnapshot) -> bool {
    catalog.entries.iter().any(|entry| {
        entry.id == ModelId::ParakeetTdt06BV3
            && entry.availability == whisper_core::model_catalog::ModelAvailability::Ready
            && !entry
                .diagnostic
                .as_deref()
                .is_some_and(|message| message.starts_with("GPU indisponible :"))
    })
}

pub fn gpu_qualified_status(ui: &mut egui::Ui, state: &GpuQualifiedInventoryState) {
    use whisper_core::gpu_inventory::GpuCatalogueCoverage;
    match state {
        GpuQualifiedInventoryState::NotRequested => {}
        GpuQualifiedInventoryState::Loading => {
            ui.label("Qualification GPU en cours ; CPU reste utilisable.");
        }
        GpuQualifiedInventoryState::Failed(error) => {
            ui.label(format!(
                "Qualification GPU indisponible : {error} ; CPU reste utilisable."
            ));
        }
        GpuQualifiedInventoryState::Complete(catalogue)
        | GpuQualifiedInventoryState::Partial(catalogue) => {
            let labels = gpu_preference_labels(&catalogue.devices);
            for (device, name) in catalogue.devices.iter().zip(labels) {
                let (color, label) = gpu_qualified_presentation(device);
                let text = format!("{name} : {label}.");
                if let Some(color) = color {
                    ui.colored_label(color, text);
                } else {
                    ui.label(text);
                }
            }
            if catalogue.coverage != GpuCatalogueCoverage::Complete || !catalogue.causes.is_empty()
            {
                ui.label("Inventaire GPU partiel ou indéterminé ; CPU reste utilisable.");
            }
        }
    }
}

pub(in crate::ui) fn qualified_devices(
    state: &GpuQualifiedInventoryState,
) -> Vec<GpuQualifiedDevice> {
    match state {
        GpuQualifiedInventoryState::Complete(catalogue)
        | GpuQualifiedInventoryState::Partial(catalogue) => catalogue.devices.clone(),
        GpuQualifiedInventoryState::NotRequested
        | GpuQualifiedInventoryState::Loading
        | GpuQualifiedInventoryState::Failed(_) => Vec::new(),
    }
}

pub fn gpu_preference_labels(devices: &[GpuQualifiedDevice]) -> Vec<String> {
    use std::collections::HashMap;

    let mut totals = HashMap::<&str, usize>::new();
    for device in devices {
        *totals.entry(&device.device.name).or_default() += 1;
    }
    let mut seen = HashMap::<&str, usize>::new();
    devices
        .iter()
        .map(|device| {
            let name = device.device.name.as_str();
            let total = totals[name];
            if total == 1 {
                return name.to_owned();
            }
            let index = seen.entry(name).or_default();
            *index += 1;
            format!("{name} - {}", *index)
        })
        .collect()
}

pub fn gpu_preference_combo(
    ui: &mut egui::Ui,
    devices: &[GpuQualifiedDevice],
    preferred: &mut Option<String>,
) {
    let labels = gpu_preference_labels(devices);
    let selected_label = preferred.as_deref().map_or_else(
        || "Choisir un GPU".to_owned(),
        |identity| {
            devices
                .iter()
                .position(|device| device.device.identity == identity)
                .map(|index| labels[index].clone())
                .unwrap_or_else(|| "GPU indisponible".to_owned())
        },
    );
    ui.horizontal(|ui| {
        ui.label("GPU préféré");
        egui::ComboBox::from_id_salt("gpu_preference")
            .selected_text(selected_label)
            .show_ui(ui, |ui| {
                gpu_preference_options(ui, devices, &labels, preferred);
            });
    });
}

pub fn gpu_preference_options(
    ui: &mut egui::Ui,
    devices: &[GpuQualifiedDevice],
    labels: &[String],
    preferred: &mut Option<String>,
) {
    for (device, label) in devices.iter().zip(labels.iter().cloned()) {
        let identity = &device.device.identity;
        if gpu_preference_selectable(device) {
            ui.selectable_value(preferred, Some(identity.clone()), label);
        } else if device.support == whisper_core::gpu_inventory::GpuSupport::ConfirmedUnsupported {
            ui.add_enabled_ui(false, |ui| {
                ui.colored_label(egui::Color32::GRAY, label);
            });
        } else {
            ui.add_enabled_ui(false, |ui| {
                ui.label(label);
            });
        }
    }
}

pub fn gpu_preference_selectable(device: &whisper_core::gpu_inventory::GpuQualifiedDevice) -> bool {
    use whisper_core::gpu_inventory::{GpuAvailability, GpuIdentityMatch, GpuState, GpuSupport};
    device.identity_match == GpuIdentityMatch::Unique
        && !device.device.identity.trim().is_empty()
        && device.device.state == GpuState::Ready
        && device.windows_rank.is_some()
        && device.support == GpuSupport::ConfirmedSupported
        && device.availability == GpuAvailability::Available
}

pub fn gpu_selection_resolution_label(
    resolution: &SelectionResolution,
    devices: &[GpuQualifiedDevice],
) -> String {
    let labels = gpu_preference_labels(devices);
    let candidate_label = |candidate: &whisper_core::gpu_selection::GpuCandidate| {
        devices
            .iter()
            .position(|device| device.device.identity == candidate.identity)
            .map(|index| labels[index].clone())
            .unwrap_or_else(|| candidate.name.clone())
    };
    match resolution {
        SelectionResolution::Candidate(SessionCandidate::Cpu) => {
            "Pour la prochaine transcription, le CPU est prévu.".into()
        }
        SelectionResolution::Candidate(SessionCandidate::Gpu(candidate))
            if candidate.preference_substituted =>
        {
            format!(
                "Pour la prochaine transcription, {} est prévu temporairement ; préférence conservée.",
                candidate_label(candidate)
            )
        }
        SelectionResolution::Candidate(SessionCandidate::Gpu(candidate)) => format!(
            "Pour la prochaine transcription, {} est prévu.",
            candidate_label(candidate)
        ),
        SelectionResolution::CpuFallback { .. } => {
            "Pour la prochaine transcription, le CPU est prévu.".into()
        }
        SelectionResolution::Waiting(_) => "Vérification du choix GPU en cours.".into(),
        SelectionResolution::Refused(_) => {
            "Le choix GPU n'est pas disponible pour la prochaine transcription.".into()
        }
    }
}

pub fn gpu_qualified_presentation(
    device: &whisper_core::gpu_inventory::GpuQualifiedDevice,
) -> (Option<egui::Color32>, &'static str) {
    use whisper_core::gpu_inventory::{
        GpuAvailability as Availability, GpuIdentityMatch, GpuState, GpuSupport as Support,
    };
    let rank_present = device.windows_rank.is_some();
    let windows_ready = device.device.state == GpuState::Ready;
    if !rank_present || !windows_ready {
        return (
            None,
            "qualification incomplète ; sélection GPU indisponible",
        );
    }
    match (device.identity_match, device.support, device.availability) {
        (GpuIdentityMatch::Unique, Support::ConfirmedSupported, Availability::Available) => {
            (Some(egui::Color32::GREEN), "compatible et disponible")
        }
        (
            GpuIdentityMatch::Unique | GpuIdentityMatch::Missing,
            Support::ConfirmedUnsupported,
            _,
        ) => (
            Some(egui::Color32::GRAY),
            "exclu : non pris en charge par ce build",
        ),
        (_, Support::ConfirmedSupported, Availability::Unavailable) => {
            (None, "support confirmé, mais indisponible")
        }
        (_, Support::ConfirmedSupported, Availability::NotVerified) => {
            (None, "support confirmé, disponibilité non vérifiée")
        }
        (_, Support::Undetermined, Availability::Unavailable) => {
            (None, "indéterminé, temporairement indisponible")
        }
        _ => (None, "indéterminé"),
    }
}

pub fn gpu_inventory_status(ui: &mut egui::Ui, state: &GpuInventoryState) {
    if let Some(message) = gpu_inventory_status_message(state) {
        ui.label(message);
    }
}

pub fn gpu_inventory_status_message(state: &GpuInventoryState) -> Option<String> {
    match state {
        GpuInventoryState::NotRequested => None,
        GpuInventoryState::Loading => Some("Inventaire GPU en cours…".into()),
        GpuInventoryState::Complete(inventory) if inventory.devices.is_empty() => {
            Some("Aucun GPU présent selon Windows.".into())
        }
        GpuInventoryState::Complete(_) => Some("Inventaire GPU complet selon Windows.".into()),
        GpuInventoryState::Partial(inventory) => {
            let mut messages = Vec::new();
            if inventory.devices.is_empty() {
                messages.push("Aucun GPU confirmé présent".to_owned());
            }
            messages.extend(inventory.warnings.iter().cloned());
            if messages.is_empty() {
                messages.push("Certaines propriétés ou cartes restent incertaines.".into());
            }
            Some(format!("Inventaire GPU partiel : {}", messages.join(" ")))
        }
        GpuInventoryState::Failed(error) => Some(format!("Inventaire GPU indisponible : {error}")),
    }
}
