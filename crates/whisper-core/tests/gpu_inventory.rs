use std::{cell::RefCell, collections::VecDeque, rc::Rc};
use whisper_core::gpu_inventory::{
    Coverage, GpuAvailability, GpuCatalogueCoverage, GpuCause, GpuDevice, GpuIdentityMatch,
    GpuInventory, GpuObservation, GpuPresence, GpuQualifiedObservation, GpuRankSource, GpuState,
    GpuSupport, GpuWindowsRank, normalize, qualify,
};
use whisper_core::gpu_inventory_app::{
    GpuCollection, GpuInventoryApp, GpuInventoryPort, GpuInventoryState, GpuQualifiedCollection,
    GpuQualifiedInventoryState,
};

fn observation(identity: &str, name: Option<&str>, presence: GpuPresence) -> GpuObservation {
    GpuObservation {
        identity: identity.into(),
        name: name.map(str::to_owned),
        state: Some(GpuState::Ready),
        presence,
    }
}

fn qualified(identity: &str, name: &str, rank: Option<u32>) -> GpuQualifiedObservation {
    GpuQualifiedObservation {
        observation: observation(identity, Some(name), GpuPresence::Current),
        windows_rank: rank.map(|ordinal| GpuWindowsRank {
            ordinal,
            source: GpuRankSource::DxgiEnumeration,
        }),
        identity_match: GpuIdentityMatch::Unique,
        support: GpuSupport::ConfirmedSupported,
        availability: GpuAvailability::Available,
        causes: vec![],
    }
}

#[test]
fn inventory_keeps_homonymous_devices_and_excludes_confirmed_absence() {
    let inventory = normalize(
        vec![
            observation("pci:1", Some("Intel UHD"), GpuPresence::Current),
            observation("pci:2", Some("Intel UHD"), GpuPresence::Current),
            observation("pci:old", Some("Old GPU"), GpuPresence::Absent),
        ],
        false,
    );
    assert_eq!(inventory.coverage, Coverage::Complete);
    assert_eq!(inventory.devices.len(), 2);
    assert_ne!(inventory.devices[0].identity, inventory.devices[1].identity);
    assert_eq!(inventory.devices[0].name, inventory.devices[1].name);
}

#[test]
fn current_device_with_missing_properties_is_kept_as_unknown() {
    let inventory = normalize(
        vec![GpuObservation {
            identity: "pci:unknown-fields".into(),
            name: None,
            state: None,
            presence: GpuPresence::Current,
        }],
        false,
    );
    assert_eq!(inventory.coverage, Coverage::Partial);
    assert_eq!(inventory.devices[0].name, "GPU (nom inconnu)");
    assert_eq!(inventory.devices[0].state, GpuState::Unknown);
    assert_eq!(inventory.warnings.len(), 2);
}

#[test]
fn unknown_or_conflicting_presence_is_not_reported_as_a_present_gpu() {
    let inventory = normalize(
        vec![
            observation("pci:unknown", Some("Unknown"), GpuPresence::Unknown),
            observation("pci:collision", Some("Collision"), GpuPresence::Current),
            observation("pci:collision", Some("Collision"), GpuPresence::Absent),
        ],
        false,
    );
    assert_eq!(inventory.coverage, Coverage::Partial);
    assert!(inventory.devices.is_empty());
    assert!(
        inventory
            .warnings
            .iter()
            .any(|warning| warning.contains("contradictoire"))
    );
}

#[test]
fn enumeration_error_after_observations_keeps_observations_and_marks_partial() {
    let inventory = normalize(
        vec![observation("pci:seen", Some("Seen"), GpuPresence::Current)],
        true,
    );
    assert_eq!(inventory.coverage, Coverage::Partial);
    assert_eq!(inventory.devices[0].name, "Seen");
}

#[test]
fn completed_empty_inventory_is_distinct_from_partial_empty_inventory() {
    assert_eq!(normalize(vec![], false).coverage, Coverage::Complete);
    assert_eq!(
        normalize(
            vec![observation("pci:?", None, GpuPresence::Unknown)],
            false
        )
        .coverage,
        Coverage::Partial
    );
}

#[derive(Clone, Default)]
struct ControlledPort(Rc<RefCell<ControlledState>>);

#[derive(Default)]
struct ControlledState {
    started: Vec<u64>,
    responses: VecDeque<(u64, Result<GpuCollection, String>)>,
}

impl GpuInventoryPort for ControlledPort {
    fn start(&mut self, generation: u64) -> Result<(), String> {
        self.0.borrow_mut().started.push(generation);
        Ok(())
    }

    fn poll(&mut self) -> Option<(u64, Result<GpuCollection, String>)> {
        self.0.borrow_mut().responses.pop_front()
    }
}

#[test]
fn app_rejects_stale_responses_and_close_revokes_publication() {
    let port = ControlledPort::default();
    let state = Rc::clone(&port.0);
    let mut app = GpuInventoryApp::new(Box::new(port));
    assert_eq!(*app.state(), GpuInventoryState::Loading);
    app.start();
    assert_eq!(state.borrow().started, vec![1]);
    state.borrow_mut().responses.push_back((
        0,
        Ok(GpuCollection {
            observations: vec![observation("old", Some("Old"), GpuPresence::Current)],
            error: None,
        }),
    ));
    app.poll();
    assert_eq!(*app.state(), GpuInventoryState::Loading);
    app.close();
    state.borrow_mut().responses.push_back((
        2,
        Ok(GpuCollection {
            observations: vec![observation("late", Some("Late"), GpuPresence::Current)],
            error: None,
        }),
    ));
    app.poll();
    assert_eq!(*app.state(), GpuInventoryState::NotRequested);
}

#[test]
fn app_publishes_partial_collection_and_failure_states() {
    let port = ControlledPort::default();
    let state = Rc::clone(&port.0);
    let mut app = GpuInventoryApp::new(Box::new(port));
    state.borrow_mut().responses.push_back((
        1,
        Ok(GpuCollection {
            observations: vec![observation("seen", Some("Seen"), GpuPresence::Current)],
            error: Some("interrupted".into()),
        }),
    ));
    app.poll();
    assert!(
        matches!(app.state(), GpuInventoryState::Partial(inventory) if inventory.devices.len() == 1 && inventory.warnings.iter().any(|warning| warning == "interrupted"))
    );
    app.start();
    state
        .borrow_mut()
        .responses
        .push_back((2, Err("WMI failed".into())));
    app.poll();
    assert_eq!(*app.state(), GpuInventoryState::Failed("WMI failed".into()));
    app.start();
    state.borrow_mut().responses.push_back((
        3,
        Ok(GpuCollection {
            observations: vec![observation(
                "unknown",
                Some("Unknown"),
                GpuPresence::Unknown,
            )],
            error: Some("enumeration stopped before a current device was found".into()),
        }),
    ));
    app.poll();
    assert_eq!(
        *app.state(),
        GpuInventoryState::Failed("enumeration stopped before a current device was found".into())
    );
}

#[test]
fn app_ignores_duplicate_terminal_response_until_a_new_generation_starts() {
    let port = ControlledPort::default();
    let state = Rc::clone(&port.0);
    let mut app = GpuInventoryApp::new(Box::new(port));
    let terminal = GpuCollection {
        observations: vec![observation("one", Some("One"), GpuPresence::Current)],
        error: None,
    };
    state
        .borrow_mut()
        .responses
        .push_back((1, Ok(terminal.clone())));
    app.poll();
    let published = app.state().clone();
    state
        .borrow_mut()
        .responses
        .push_back((1, Err("duplicate".into())));
    app.poll();
    assert_eq!(*app.state(), published);
    app.start();
    assert_eq!(state.borrow().started, vec![1, 2]);
}

#[derive(Clone, Default)]
struct QualifiedPort(Rc<RefCell<QualifiedPortState>>);

#[derive(Default)]
struct QualifiedPortState {
    started: Vec<u64>,
    responses: VecDeque<(u64, Result<GpuQualifiedCollection, String>)>,
}

impl GpuInventoryPort for QualifiedPort {
    fn start(&mut self, generation: u64) -> Result<(), String> {
        self.0.borrow_mut().started.push(generation);
        Ok(())
    }

    fn poll(&mut self) -> Option<(u64, Result<GpuCollection, String>)> {
        None
    }

    fn uses_qualified_catalogue(&self) -> bool {
        true
    }

    fn poll_qualified(&mut self) -> Option<(u64, Result<GpuQualifiedCollection, String>)> {
        self.0.borrow_mut().responses.pop_front()
    }
}

#[test]
fn app_publishes_qualified_partial_catalogue_and_rejects_late_or_duplicate_results() {
    let port = QualifiedPort::default();
    let shared = Rc::clone(&port.0);
    let mut app = GpuInventoryApp::new(Box::new(port));
    assert_eq!(*app.qualified_state(), GpuQualifiedInventoryState::Loading);
    let mut seen = qualified("seen", "Seen", Some(0));
    seen.support = GpuSupport::ConfirmedSupported;
    shared.borrow_mut().responses.push_back((
        1,
        Ok(GpuQualifiedCollection {
            observations: vec![seen],
            error: Some("collecte interrompue".into()),
        }),
    ));
    app.poll();
    assert!(
        matches!(app.qualified_state(), GpuQualifiedInventoryState::Partial(catalogue) if catalogue.devices.len() == 1 && catalogue.causes.contains(&GpuCause::Collection("collecte interrompue".into())))
    );
    assert!(
        matches!(app.state(), GpuInventoryState::Partial(inventory) if inventory.devices.len() == 1)
    );
    let published = app.qualified_state().clone();
    shared
        .borrow_mut()
        .responses
        .push_back((1, Err("duplicate".into())));
    app.poll();
    assert_eq!(*app.qualified_state(), published);
    app.close();
    shared
        .borrow_mut()
        .responses
        .push_back((2, Err("late".into())));
    app.poll();
    assert_eq!(
        *app.qualified_state(),
        GpuQualifiedInventoryState::NotRequested
    );
}

#[test]
fn ready_and_disabled_states_are_preserved() {
    let ready = normalize(
        vec![observation("1", Some("A"), GpuPresence::Current)],
        false,
    );
    let mut disabled = observation("2", Some("B"), GpuPresence::Current);
    disabled.state = Some(GpuState::Disabled);
    let disabled = normalize(vec![disabled], false);
    assert_eq!(ready.devices[0].state, GpuState::Ready);
    assert_eq!(disabled.devices[0].state, GpuState::Disabled);
}

#[test]
fn gpu_inventory_types_are_plain_owned_domain_data() {
    let expected = GpuInventory {
        devices: vec![GpuDevice {
            identity: "pci:1".into(),
            name: "GPU".into(),
            state: GpuState::Ready,
        }],
        coverage: Coverage::Complete,
        warnings: vec![],
    };
    assert_eq!(
        normalize(
            vec![observation("pci:1", Some("GPU"), GpuPresence::Current)],
            false
        ),
        expected
    );
}

#[test]
fn qualified_inventory_keeps_homonyms_and_sourced_windows_ranks() {
    let inventory = qualify(
        vec![
            qualified("id-a", "NVIDIA", Some(1)),
            qualified("id-b", "NVIDIA", Some(0)),
        ],
        None,
    );
    assert_eq!(inventory.coverage, GpuCatalogueCoverage::Complete);
    assert_eq!(inventory.devices.len(), 2);
    assert_eq!(
        inventory.devices[0].device.name,
        inventory.devices[1].device.name
    );
    assert_eq!(
        inventory.devices[0].windows_rank.unwrap().source,
        GpuRankSource::DxgiEnumeration
    );
    assert_ne!(
        inventory.devices[0].device.identity,
        inventory.devices[1].device.identity
    );
}

#[test]
fn qualified_inventory_distinguishes_absent_and_conflicting_ranks_and_preserves_failure_cause() {
    let mut rank_conflict = qualified("id-conflict", "GPU", Some(0));
    let mut conflicting_copy = rank_conflict.clone();
    conflicting_copy.windows_rank = Some(GpuWindowsRank {
        ordinal: 2,
        source: GpuRankSource::DxgiEnumeration,
    });
    let inventory = qualify(
        vec![
            qualified("id-missing", "Missing rank", None),
            rank_conflict.clone(),
            conflicting_copy,
        ],
        Some("enumeration stopped".into()),
    );
    assert_eq!(inventory.coverage, GpuCatalogueCoverage::Partial);
    assert!(
        inventory
            .devices
            .iter()
            .find(|device| device.device.identity == "id-missing")
            .unwrap()
            .causes
            .contains(&GpuCause::RankMissing)
    );
    assert!(
        inventory
            .devices
            .iter()
            .find(|device| device.device.identity == "id-conflict")
            .unwrap()
            .causes
            .contains(&GpuCause::RankConflicting)
    );
    assert_eq!(
        inventory.causes,
        vec![GpuCause::Collection("enumeration stopped".into())]
    );
    rank_conflict.identity_match = GpuIdentityMatch::Ambiguous;
    let ambiguous = qualify(vec![rank_conflict], None);
    assert!(
        ambiguous.devices[0]
            .causes
            .contains(&GpuCause::IdentityAmbiguous)
    );
}
