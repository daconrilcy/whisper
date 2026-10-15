use eframe::egui::{self, Shape};
use whisper_core::{
    gpu_inventory::{
        GpuAvailability, GpuDevice, GpuIdentityMatch, GpuQualifiedDevice, GpuState, GpuSupport,
    },
    gpu_selection::{GpuCandidate, SelectionReason, SelectionResolution, SessionCandidate},
    path_settings::SaveStatus,
};
use whisper_desktop::ui::{
    gpu_preference_combo, gpu_preference_labels, gpu_preference_options, gpu_preference_selectable,
    gpu_selection_resolution_label, path_save_status_label,
};

#[test]
fn homonymous_devices_get_numbered_labels_and_keep_distinct_internal_identity() {
    let first_device = qualified_device("PCI\\VEN_10DE&DEV_A", GpuSupport::ConfirmedSupported);
    let second_device = qualified_device("PCI\\VEN_10DE&DEV_B", GpuSupport::ConfirmedSupported);
    let third_device = qualified_device("PCI\\VEN_10DE&DEV_C", GpuSupport::ConfirmedSupported);
    let devices = vec![
        first_device.clone(),
        second_device.clone(),
        third_device.clone(),
    ];
    assert_eq!(
        gpu_preference_labels(&devices),
        ["GeForce GPU - 1", "GeForce GPU - 2", "GeForce GPU - 3"]
    );
    assert_ne!(first_device.device.identity, second_device.device.identity);
    assert!(gpu_preference_selectable(&first_device));
    assert!(gpu_preference_selectable(&second_device));
    assert!(gpu_preference_selectable(&third_device));
}

#[test]
fn unique_names_have_no_suffix_and_real_combo_has_no_clear_preference_option() {
    let devices = vec![qualified_device("gpu:one", GpuSupport::ConfirmedSupported)];
    assert_eq!(gpu_preference_labels(&devices), ["GeForce GPU"]);
    let context = egui::Context::default();
    let output = context.run_ui(Default::default(), |ui| {
        let mut preferred = None;
        gpu_preference_combo(ui, &devices, &mut preferred);
        gpu_preference_options(
            ui,
            &devices,
            &gpu_preference_labels(&devices),
            &mut preferred,
        );
    });
    let mut rendered = String::new();
    for clipped in output.shapes {
        if let Shape::Text(shape) = clipped.shape {
            rendered.push_str(&shape.galley.job.text);
            rendered.push('\n');
        }
    }
    assert!(rendered.contains("Choisir un GPU"));
    assert!(rendered.contains("GeForce GPU"));
    assert!(!rendered.contains("Aucune préférence"));
    assert!(!rendered.contains("ordre Windows"));
}

#[test]
fn selected_and_resolution_text_use_the_same_numbered_name() {
    let devices = vec![
        qualified_device("gpu:first", GpuSupport::ConfirmedSupported),
        qualified_device("gpu:second", GpuSupport::ConfirmedSupported),
    ];
    let context = egui::Context::default();
    let output = context.run_ui(Default::default(), |ui| {
        let mut preferred = Some("gpu:second".to_owned());
        gpu_preference_combo(ui, &devices, &mut preferred);
    });
    let mut rendered = String::new();
    for clipped in output.shapes {
        if let Shape::Text(shape) = clipped.shape {
            rendered.push_str(&shape.galley.job.text);
        }
    }
    assert!(rendered.contains("GeForce GPU - 2"));

    let resolution = SelectionResolution::Candidate(SessionCandidate::Gpu(GpuCandidate {
        identity: "gpu:second".into(),
        name: "GeForce GPU".into(),
        preference_substituted: false,
    }));
    assert_eq!(
        gpu_selection_resolution_label(&resolution, &devices),
        "Pour la prochaine transcription, GeForce GPU - 2 est prévu."
    );
}

#[test]
fn only_confirmed_available_and_unambiguous_gpu_is_selectable() {
    let supported = qualified_device("gpu:ok", GpuSupport::ConfirmedSupported);
    let unsupported = qualified_device("gpu:old", GpuSupport::ConfirmedUnsupported);
    let uncertain = qualified_device("gpu:unknown", GpuSupport::Undetermined);
    assert!(gpu_preference_selectable(&supported));
    assert!(!gpu_preference_selectable(&unsupported));
    assert!(!gpu_preference_selectable(&uncertain));
    assert!(!gpu_preference_selectable(&GpuQualifiedDevice {
        identity_match: GpuIdentityMatch::Ambiguous,
        ..supported.clone()
    }));
    let unranked = GpuQualifiedDevice {
        windows_rank: None,
        ..supported.clone()
    };
    let unready = GpuQualifiedDevice {
        device: GpuDevice {
            state: GpuState::Error(22),
            ..supported.device.clone()
        },
        ..supported
    };
    assert_eq!(
        gpu_preference_labels(std::slice::from_ref(&unranked)),
        ["GeForce GPU"]
    );
    assert_eq!(
        gpu_preference_labels(std::slice::from_ref(&unready)),
        ["GeForce GPU"]
    );
    assert!(!gpu_preference_selectable(&unranked));
    assert!(!gpu_preference_selectable(&unready));
}

fn qualified_device(identity: &str, support: GpuSupport) -> GpuQualifiedDevice {
    GpuQualifiedDevice {
        device: GpuDevice {
            identity: identity.into(),
            name: "GeForce GPU".into(),
            state: GpuState::Ready,
        },
        windows_rank: Some(whisper_core::gpu_inventory::GpuWindowsRank {
            ordinal: 0,
            source: whisper_core::gpu_inventory::GpuRankSource::DxgiEnumeration,
        }),
        identity_match: GpuIdentityMatch::Unique,
        support,
        availability: GpuAvailability::Available,
        causes: Vec::new(),
    }
}

#[test]
fn selection_messages_are_plain_and_never_expose_internal_identity() {
    let devices = vec![qualified_device("gpu:Q", GpuSupport::ConfirmedSupported)];
    let resolution = SelectionResolution::Candidate(SessionCandidate::Gpu(GpuCandidate {
        identity: "gpu:Q".into(),
        name: "Même nom".into(),
        preference_substituted: true,
    }));
    let label = gpu_selection_resolution_label(&resolution, &devices);
    assert!(label.contains("GeForce GPU"));
    assert!(label.contains("temporairement"));
    assert!(label.contains("préférence conservée"));
    assert!(!label.contains("gpu:Q"));
    assert!(!label.contains("résolution"));

    let ordinary = SelectionResolution::Candidate(SessionCandidate::Gpu(GpuCandidate {
        identity: "gpu:Q".into(),
        name: "Même nom".into(),
        preference_substituted: false,
    }));
    let ordinary_label = gpu_selection_resolution_label(&ordinary, &devices);
    assert_eq!(
        ordinary_label,
        "Pour la prochaine transcription, GeForce GPU est prévu."
    );
    assert!(!ordinary_label.contains("gpu:Q"));

    let cpu = gpu_selection_resolution_label(
        &SelectionResolution::Candidate(SessionCandidate::Cpu),
        &devices,
    );
    assert_eq!(cpu, "Pour la prochaine transcription, le CPU est prévu.");
    let fallback = gpu_selection_resolution_label(
        &SelectionResolution::CpuFallback {
            reason: SelectionReason::NoCompatibleGpu,
        },
        &devices,
    );
    assert_eq!(fallback, cpu);
    assert_eq!(
        gpu_selection_resolution_label(
            &SelectionResolution::Waiting(SelectionReason::InventoryNotReady),
            &devices,
        ),
        "Vérification du choix GPU en cours."
    );
    assert!(
        gpu_selection_resolution_label(
            &SelectionResolution::Refused(SelectionReason::AttestationMismatch),
            &devices,
        )
        .contains("n'est pas disponible")
    );
}

#[test]
fn uncertain_selection_is_neutral_and_save_acknowledgement_is_revisioned() {
    let label = gpu_selection_resolution_label(
        &SelectionResolution::Waiting(SelectionReason::QualificationUncertain),
        &[],
    );
    assert_eq!(label, "Vérification du choix GPU en cours.");
    assert!(!label.contains("vert"));

    assert_eq!(
        path_save_status_label(&SaveStatus::Pending { revision: 7 }),
        "Sauvegarde des réglages en cours (révision 7)."
    );
    assert_eq!(
        path_save_status_label(&SaveStatus::Saved { revision: 7 }),
        "Réglages enregistrés (révision 7)."
    );
    assert_eq!(
        path_save_status_label(&SaveStatus::Failed {
            revision: 8,
            error: "disque indisponible".into(),
        }),
        "Réglages non mémorisés (révision 8) : disque indisponible."
    );
}
