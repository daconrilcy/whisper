use crate::{
    domain::ComputeChoice,
    gpu_inventory::{GpuAvailability, GpuIdentityMatch, GpuQualifiedDevice, GpuSupport},
    gpu_inventory_app::GpuQualifiedInventoryState,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SelectionReason {
    InventoryNotReady,
    InventoryIncomplete,
    QualificationUncertain,
    WindowsOrderUnproven,
    NoCompatibleGpu,
    PreferredGpuUnavailable,
    AttestationMismatch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GpuCandidate {
    pub identity: String,
    pub name: String,
    pub preference_substituted: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SessionCandidate {
    Cpu,
    Gpu(GpuCandidate),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SelectionResolution {
    Candidate(SessionCandidate),
    CpuFallback { reason: SelectionReason },
    Waiting(SelectionReason),
    Refused(SelectionReason),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EffectiveEngine {
    CpuAttested { session_id: u64 },
    GpuAttested { session_id: u64, identity: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GpuSelectionEvent {
    CandidateResolved {
        session_id: u64,
        candidate: SessionCandidate,
    },
    EffectiveEngineAttested {
        engine: EffectiveEngine,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AttestedBackend<'a> {
    Cpu,
    Gpu(&'a str),
}

pub fn resolve(
    mode: ComputeChoice,
    preferred_identity: Option<&str>,
    catalogue: &GpuQualifiedInventoryState,
) -> SelectionResolution {
    if mode == ComputeChoice::Cpu {
        return SelectionResolution::Candidate(SessionCandidate::Cpu);
    }

    let (inventory, partial) = match catalogue {
        GpuQualifiedInventoryState::Complete(inventory) => (inventory, false),
        GpuQualifiedInventoryState::Partial(inventory) => (inventory, true),
        GpuQualifiedInventoryState::NotRequested | GpuQualifiedInventoryState::Loading => {
            return SelectionResolution::Waiting(SelectionReason::InventoryNotReady);
        }
        GpuQualifiedInventoryState::Failed(_) => {
            return SelectionResolution::Waiting(SelectionReason::InventoryIncomplete);
        }
    };

    if let Some(preferred) = preferred_identity {
        if let Some(device) = inventory
            .devices
            .iter()
            .find(|device| device.device.identity == preferred && is_compatible(device))
        {
            return SelectionResolution::Candidate(SessionCandidate::Gpu(candidate(device, false)));
        }
    }

    let mut compatible: Vec<&GpuQualifiedDevice> = inventory
        .devices
        .iter()
        .filter(|device| is_compatible(device))
        .collect();

    if partial {
        return if compatible.is_empty() {
            SelectionResolution::Waiting(SelectionReason::InventoryIncomplete)
        } else {
            SelectionResolution::Waiting(SelectionReason::WindowsOrderUnproven)
        };
    }

    if compatible.is_empty() {
        let uncertain = inventory.devices.iter().any(|device| {
            device.identity_match != GpuIdentityMatch::Unique
                || device.support == GpuSupport::Undetermined
                || (device.support == GpuSupport::ConfirmedSupported
                    && device.availability == GpuAvailability::NotVerified)
        });
        if uncertain {
            return SelectionResolution::Waiting(SelectionReason::QualificationUncertain);
        }
        return fallback_or_refuse(
            mode,
            if preferred_identity.is_some() {
                SelectionReason::PreferredGpuUnavailable
            } else {
                SelectionReason::NoCompatibleGpu
            },
        );
    }

    if compatible
        .iter()
        .any(|device| device.windows_rank.is_none())
    {
        return SelectionResolution::Waiting(SelectionReason::WindowsOrderUnproven);
    }
    compatible.sort_by_key(|device| device.windows_rank.expect("rank checked").ordinal);
    if compatible.windows(2).any(|pair| {
        let left = pair[0].windows_rank.expect("rank checked");
        let right = pair[1].windows_rank.expect("rank checked");
        left.ordinal == right.ordinal || left.source != right.source
    }) {
        return SelectionResolution::Waiting(SelectionReason::WindowsOrderUnproven);
    }
    SelectionResolution::Candidate(SessionCandidate::Gpu(candidate(
        compatible[0],
        preferred_identity.is_some(),
    )))
}

pub fn attest(
    session_id: u64,
    candidate: &SessionCandidate,
    backend: AttestedBackend<'_>,
) -> Result<EffectiveEngine, SelectionReason> {
    match (candidate, backend) {
        (SessionCandidate::Cpu, AttestedBackend::Cpu) => {
            Ok(EffectiveEngine::CpuAttested { session_id })
        }
        (SessionCandidate::Gpu(candidate), AttestedBackend::Gpu(identity))
            if candidate.identity == identity =>
        {
            Ok(EffectiveEngine::GpuAttested {
                session_id,
                identity: identity.to_owned(),
            })
        }
        _ => Err(SelectionReason::AttestationMismatch),
    }
}

pub fn candidate_event(session_id: u64, candidate: SessionCandidate) -> GpuSelectionEvent {
    GpuSelectionEvent::CandidateResolved {
        session_id,
        candidate,
    }
}

pub fn attestation_event(
    session_id: u64,
    candidate: &SessionCandidate,
    backend: AttestedBackend<'_>,
) -> Result<GpuSelectionEvent, SelectionReason> {
    attest(session_id, candidate, backend)
        .map(|engine| GpuSelectionEvent::EffectiveEngineAttested { engine })
}

fn is_compatible(device: &GpuQualifiedDevice) -> bool {
    device.identity_match == GpuIdentityMatch::Unique
        && !device.device.identity.trim().is_empty()
        && device.device.state != crate::gpu_inventory::GpuState::Disabled
        && device.support == GpuSupport::ConfirmedSupported
        && device.availability == GpuAvailability::Available
}

fn candidate(device: &GpuQualifiedDevice, preference_substituted: bool) -> GpuCandidate {
    GpuCandidate {
        identity: device.device.identity.clone(),
        name: device.device.name.clone(),
        preference_substituted,
    }
}

fn fallback_or_refuse(mode: ComputeChoice, reason: SelectionReason) -> SelectionResolution {
    match mode {
        ComputeChoice::Auto => SelectionResolution::CpuFallback { reason },
        ComputeChoice::Gpu => SelectionResolution::Refused(reason),
        ComputeChoice::Cpu => SelectionResolution::Candidate(SessionCandidate::Cpu),
    }
}
