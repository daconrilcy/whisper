use eframe::egui::{self, Shape};
use whisper_core::gpu_inventory::{
    Coverage, GpuAvailability, GpuCatalogueCoverage, GpuDevice, GpuIdentityMatch, GpuInventory,
    GpuQualifiedCatalogue, GpuQualifiedDevice, GpuRankSource, GpuState, GpuSupport,
};
use whisper_core::gpu_inventory_app::{GpuInventoryState, GpuQualifiedInventoryState};
use whisper_desktop::ui::{gpu_inventory_status, gpu_qualified_presentation, gpu_qualified_status};

fn rendered_text(state: &GpuInventoryState) -> String {
    let context = egui::Context::default();
    let output = context.run_ui(Default::default(), |ui| gpu_inventory_status(ui, state));
    fn collect(shape: &Shape, text: &mut String) {
        match shape {
            Shape::Text(shape) => {
                text.push_str(&shape.galley.job.text);
                text.push('\n');
            }
            Shape::Vec(shapes) => shapes.iter().for_each(|shape| collect(shape, text)),
            _ => {}
        }
    }
    let mut text = String::new();
    for clipped in output.shapes {
        collect(&clipped.shape, &mut text);
    }
    text
}

fn rendered_status(state: &GpuInventoryState) -> String {
    let context = egui::Context::default();
    let output = context.run_ui(Default::default(), |ui| gpu_inventory_status(ui, state));
    fn collect(shape: &Shape, text: &mut String) {
        match shape {
            Shape::Text(shape) => {
                text.push_str(&shape.galley.job.text);
                text.push('\n');
            }
            Shape::Vec(shapes) => shapes.iter().for_each(|shape| collect(shape, text)),
            _ => {}
        }
    }
    let mut text = String::new();
    for clipped in output.shapes {
        collect(&clipped.shape, &mut text);
    }
    text
}

fn inventory(devices: Vec<GpuDevice>, coverage: Coverage, warnings: Vec<String>) -> GpuInventory {
    GpuInventory {
        devices,
        coverage,
        warnings,
    }
}

#[test]
fn inventory_status_does_not_render_the_removed_fixed_card_list() {
    let state = GpuInventoryState::Complete(inventory(
        vec![
            GpuDevice {
                identity: "pci:intel".into(),
                name: "Intel UHD 770".into(),
                state: GpuState::Ready,
            },
            GpuDevice {
                identity: "root:virtual".into(),
                name: "Virtual Display".into(),
                state: GpuState::Ready,
            },
        ],
        Coverage::Complete,
        vec![],
    ));
    let text = rendered_text(&state);
    assert!(text.contains("Inventaire GPU complet selon Windows"));
    assert!(!text.contains("Cartes graphiques détectées par Windows"));
    assert!(!text.contains("Intel UHD 770"));
    assert!(!text.contains("Virtual Display"));
}

#[test]
fn status_area_distinguishes_loading_empty_partial_and_failure() {
    assert!(rendered_status(&GpuInventoryState::Loading).contains("en cours"));
    let empty = rendered_status(&GpuInventoryState::Complete(inventory(
        vec![],
        Coverage::Complete,
        vec![],
    )));
    assert!(empty.contains("Aucun GPU présent selon Windows"));
    let partial_state = GpuInventoryState::Partial(inventory(
        vec![],
        Coverage::Partial,
        vec!["Présence indéterminée.".into()],
    ));
    let partial = rendered_status(&partial_state);
    assert!(partial.contains("Aucun GPU confirmé présent"));
    assert!(partial.contains("Présence indéterminée"));
    let failed_state = GpuInventoryState::Failed("WMI indisponible".into());
    let failed = rendered_status(&failed_state);
    assert!(failed.contains("Inventaire GPU indisponible"));
    assert!(failed.contains("WMI indisponible"));
    assert!(!failed.contains("Aucun GPU présent selon Windows"));
}
#[test]
fn qualified_loading_and_uncertain_support_are_accessible_and_cpu_remains_available() {
    let context = egui::Context::default();
    let output = context.run_ui(Default::default(), |ui| {
        gpu_qualified_status(ui, &GpuQualifiedInventoryState::Loading)
    });
    fn collect(shape: &Shape, text: &mut String) {
        match shape {
            Shape::Text(shape) => text.push_str(&shape.galley.job.text),
            Shape::Vec(shapes) => shapes.iter().for_each(|shape| collect(shape, text)),
            _ => {}
        }
    }
    let mut text = String::new();
    for clipped in output.shapes {
        collect(&clipped.shape, &mut text);
    }
    assert!(text.contains("CPU reste utilisable"));
}

#[test]
fn qualified_catalogue_uses_green_gray_and_neutral_text_without_color_only_meaning() {
    let device = |name: &str,
                  state: GpuState,
                  rank: bool,
                  identity_match: GpuIdentityMatch,
                  support: GpuSupport,
                  availability: GpuAvailability| {
        GpuQualifiedDevice {
            device: GpuDevice {
                identity: name.into(),
                name: name.into(),
                state,
            },
            windows_rank: rank.then_some(whisper_core::gpu_inventory::GpuWindowsRank {
                ordinal: 0,
                source: GpuRankSource::DxgiEnumeration,
            }),
            identity_match,
            support,
            availability,
            causes: vec![],
        }
    };
    let mut compatible_device = device(
        "A",
        GpuState::Ready,
        true,
        GpuIdentityMatch::Unique,
        GpuSupport::ConfirmedSupported,
        GpuAvailability::Available,
    );
    compatible_device.device.name = "GPU homonyme".into();
    let mut excluded_device = device(
        "B",
        GpuState::Ready,
        true,
        GpuIdentityMatch::Missing,
        GpuSupport::ConfirmedUnsupported,
        GpuAvailability::NotVerified,
    );
    excluded_device.device.name = "GPU homonyme".into();
    let uncertain_device = device(
        "C",
        GpuState::Ready,
        true,
        GpuIdentityMatch::Missing,
        GpuSupport::Undetermined,
        GpuAvailability::NotVerified,
    );
    let (compatible_color, compatible) = gpu_qualified_presentation(&compatible_device);
    let (excluded_color, excluded) = gpu_qualified_presentation(&excluded_device);
    let (uncertain_color, uncertain) = gpu_qualified_presentation(&uncertain_device);
    assert_eq!(compatible_color, Some(egui::Color32::GREEN));
    assert!(compatible.contains("compatible et disponible"));
    assert_eq!(excluded_color, Some(egui::Color32::GRAY));
    assert!(excluded.contains("exclu : non pris en charge par ce build"));
    let unique_cuda_excluded = device(
        "CUDA hors architecture",
        GpuState::Ready,
        true,
        GpuIdentityMatch::Unique,
        GpuSupport::ConfirmedUnsupported,
        GpuAvailability::NotVerified,
    );
    let (cuda_excluded_color, cuda_excluded) = gpu_qualified_presentation(&unique_cuda_excluded);
    assert_eq!(cuda_excluded_color, Some(egui::Color32::GRAY));
    assert!(cuda_excluded.contains("exclu : non pris en charge par ce build"));
    assert_eq!(uncertain_color, None);
    assert!(uncertain.contains("indéterminé"));

    let state = GpuQualifiedInventoryState::Complete(GpuQualifiedCatalogue {
        devices: vec![compatible_device, excluded_device, uncertain_device],
        coverage: GpuCatalogueCoverage::Complete,
        causes: vec![],
    });
    let context = egui::Context::default();
    let output = context.run_ui(Default::default(), |ui| gpu_qualified_status(ui, &state));
    let mut rendered = String::new();
    for clipped in output.shapes {
        if let Shape::Text(shape) = clipped.shape {
            rendered.push_str(&shape.galley.job.text);
            rendered.push('\n');
        }
    }
    assert!(rendered.contains("GPU homonyme - 1 : compatible et disponible"));
    assert!(rendered.contains("GPU homonyme - 2 : exclu : non pris en charge par ce build"));
    assert!(rendered.contains("C : indéterminé"));
}

#[test]
fn disabled_ambiguous_or_unranked_devices_are_never_green() {
    let make = |state, windows_rank, identity_match| GpuQualifiedDevice {
        device: GpuDevice {
            identity: "candidate".into(),
            name: "candidate".into(),
            state,
        },
        windows_rank,
        identity_match,
        support: GpuSupport::ConfirmedSupported,
        availability: GpuAvailability::Available,
        causes: vec![],
    };
    let rank = Some(whisper_core::gpu_inventory::GpuWindowsRank {
        ordinal: 0,
        source: GpuRankSource::DxgiEnumeration,
    });
    for device in [
        make(GpuState::Disabled, rank, GpuIdentityMatch::Unique),
        make(GpuState::Ready, rank, GpuIdentityMatch::Ambiguous),
        make(GpuState::Ready, None, GpuIdentityMatch::Unique),
    ] {
        let (color, label) = gpu_qualified_presentation(&device);
        assert_eq!(color, None);
        assert!(label.contains("qualification incomplète") || label.contains("indéterminé"));
    }
    let ambiguous_excluded = GpuQualifiedDevice {
        support: GpuSupport::ConfirmedUnsupported,
        ..make(GpuState::Ready, rank, GpuIdentityMatch::Ambiguous)
    };
    let (color, label) = gpu_qualified_presentation(&ambiguous_excluded);
    assert_eq!(color, None);
    assert!(label.contains("indéterminé"));
}

#[test]
fn widget_shows_disabled_and_unknown_device_properties() {
    let state = GpuInventoryState::Partial(inventory(
        vec![
            GpuDevice {
                identity: "pci:disabled".into(),
                name: "Disabled".into(),
                state: GpuState::Disabled,
            },
            GpuDevice {
                identity: "pci:unknown".into(),
                name: "GPU (nom inconnu)".into(),
                state: GpuState::Unknown,
            },
        ],
        Coverage::Partial,
        vec!["Nom inconnu.".into()],
    ));
    let text = rendered_text(&state);
    assert!(!text.contains("Disabled"));
    assert!(!text.contains("GPU (nom inconnu)"));
    let status = rendered_status(&state);
    assert!(status.contains("Inventaire GPU partiel"));
    assert!(status.contains("Nom inconnu."));
}
