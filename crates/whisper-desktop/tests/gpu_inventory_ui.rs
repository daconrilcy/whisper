use eframe::egui::{self, Shape};
use whisper_core::gpu_inventory::{Coverage, GpuDevice, GpuInventory, GpuState};
use whisper_core::gpu_inventory_app::GpuInventoryState;
use whisper_desktop::ui::{gpu_inventory_status, gpu_inventory_ui};

fn rendered_text(state: &GpuInventoryState) -> String {
    let context = egui::Context::default();
    let output = context.run_ui(Default::default(), |ui| {
        gpu_inventory_ui(ui, state);
    });
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
fn inventory_frame_contains_card_rows_and_places_the_cuda_note_outside_the_frame() {
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
    assert!(text.contains("Intel UHD 770"));
    assert!(text.contains("Virtual Display"));
    assert!(text.contains("fonctionnement normal"));
    assert!(
        text.contains("Note : la présence selon Windows ne confirme pas la compatibilité CUDA.")
    );
    assert!(!text.contains("Inventaire GPU complet"));
    assert!(!text.contains("Choisir une carte"));
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
    assert!(text.contains("désactivé"));
    assert!(text.contains("état inconnu"));
    assert!(!text.contains("Inventaire partiel"));
    assert!(!text.contains("Nom inconnu."));
    assert!(rendered_status(&state).contains("Inventaire GPU partiel"));
}
