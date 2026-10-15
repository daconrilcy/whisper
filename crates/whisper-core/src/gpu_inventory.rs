use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GpuPresence {
    Current,
    Absent,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GpuRankSource {
    DxgiEnumeration,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GpuWindowsRank {
    pub ordinal: u32,
    pub source: GpuRankSource,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GpuIdentityMatch {
    Unique,
    Missing,
    Ambiguous,
    Conflicting,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GpuSupport {
    ConfirmedSupported,
    ConfirmedUnsupported,
    Undetermined,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GpuAvailability {
    Available,
    Unavailable,
    NotVerified,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GpuState {
    Ready,
    Disabled,
    Error(u32),
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GpuObservation {
    pub identity: String,
    pub name: Option<String>,
    pub state: Option<GpuState>,
    pub presence: GpuPresence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GpuDevice {
    pub identity: String,
    pub name: String,
    pub state: GpuState,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Coverage {
    Complete,
    Partial,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GpuInventory {
    pub devices: Vec<GpuDevice>,
    pub coverage: Coverage,
    pub warnings: Vec<String>,
}

/// Extended domain record for GPU selection; the legacy inventory structs stay source-compatible.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GpuQualifiedObservation {
    pub observation: GpuObservation,
    pub windows_rank: Option<GpuWindowsRank>,
    pub identity_match: GpuIdentityMatch,
    pub support: GpuSupport,
    pub availability: GpuAvailability,
    pub causes: Vec<GpuCause>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GpuQualifiedDevice {
    pub device: GpuDevice,
    pub windows_rank: Option<GpuWindowsRank>,
    pub identity_match: GpuIdentityMatch,
    pub support: GpuSupport,
    pub availability: GpuAvailability,
    pub causes: Vec<GpuCause>,
}

/// Ephemeral cross-layer proof for one adapter generation. CUDA and GGML
/// ordinals are intentionally distinct from the Windows DXGI rank.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GpuRuntimeIdentity {
    pub generation: u64,
    pub pnp_instance_id: String,
    pub dxgi_rank: u32,
    pub luid: u64,
    pub pci_id: String,
    pub cuda_ordinal: u32,
    pub ggml_ordinal: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GpuCause {
    IdentityMissing,
    IdentityAmbiguous,
    IdentityConflicting,
    RankMissing,
    RankConflicting,
    SupportUndetermined,
    TemporarilyUnavailable,
    AvailabilityNotVerified,
    Collection(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GpuCatalogueCoverage {
    Complete,
    Partial,
    Failed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GpuQualifiedCatalogue {
    pub devices: Vec<GpuQualifiedDevice>,
    pub coverage: GpuCatalogueCoverage,
    pub causes: Vec<GpuCause>,
}

pub fn qualify(
    observations: Vec<GpuQualifiedObservation>,
    failed: Option<String>,
) -> GpuQualifiedCatalogue {
    let had_observations = !observations.is_empty();
    let mut by_identity: BTreeMap<String, Vec<GpuQualifiedObservation>> = BTreeMap::new();
    let mut order = Vec::new();
    for observation in observations {
        let identity = observation.observation.identity.clone();
        if !by_identity.contains_key(&identity) {
            order.push(identity.clone());
        }
        by_identity.entry(identity).or_default().push(observation);
    }
    let mut devices = Vec::new();
    let mut causes = Vec::new();
    for identity in order {
        let records = by_identity.remove(&identity).expect("indexed identity");
        let presence = records
            .iter()
            .map(|record| record.observation.presence)
            .collect::<Vec<_>>();
        if presence.iter().all(|value| *value == GpuPresence::Absent) {
            continue;
        }
        if presence.iter().any(|value| *value != GpuPresence::Current) {
            causes.push(GpuCause::Collection(format!(
                "Présence indéterminée ou contradictoire pour {identity}."
            )));
            continue;
        }
        let names = consistent(
            records
                .iter()
                .map(|record| record.observation.name.as_deref()),
        );
        let states = consistent(
            records
                .iter()
                .map(|record| record.observation.state.as_ref()),
        );
        let ranks = consistent(records.iter().map(|record| record.windows_rank.as_ref()));
        let identity_match = consistent(records.iter().map(|record| Some(&record.identity_match)))
            .flatten()
            .copied()
            .unwrap_or(GpuIdentityMatch::Conflicting);
        let support = consistent(records.iter().map(|record| Some(&record.support)))
            .flatten()
            .copied()
            .unwrap_or(GpuSupport::Undetermined);
        let availability = consistent(records.iter().map(|record| Some(&record.availability)))
            .flatten()
            .copied()
            .unwrap_or(GpuAvailability::NotVerified);
        let mut device_causes = records
            .iter()
            .flat_map(|record| record.causes.clone())
            .collect::<Vec<_>>();
        let rank = ranks.flatten().copied();
        if rank.is_none() {
            let cause = if records.iter().any(|record| record.windows_rank.is_some()) {
                GpuCause::RankConflicting
            } else {
                GpuCause::RankMissing
            };
            device_causes.push(cause);
        }
        match identity_match {
            GpuIdentityMatch::Unique => {}
            GpuIdentityMatch::Missing => device_causes.push(GpuCause::IdentityMissing),
            GpuIdentityMatch::Ambiguous => device_causes.push(GpuCause::IdentityAmbiguous),
            GpuIdentityMatch::Conflicting => device_causes.push(GpuCause::IdentityConflicting),
        }
        if support == GpuSupport::Undetermined {
            device_causes.push(GpuCause::SupportUndetermined);
        }
        match availability {
            GpuAvailability::Available => {}
            GpuAvailability::Unavailable => device_causes.push(GpuCause::TemporarilyUnavailable),
            GpuAvailability::NotVerified => device_causes.push(GpuCause::AvailabilityNotVerified),
        }
        devices.push(GpuQualifiedDevice {
            device: GpuDevice {
                identity,
                name: names
                    .flatten()
                    .filter(|name| !name.trim().is_empty())
                    .unwrap_or("GPU (nom inconnu)")
                    .to_owned(),
                state: states.flatten().cloned().unwrap_or(GpuState::Unknown),
            },
            windows_rank: rank,
            identity_match,
            support,
            availability,
            causes: device_causes,
        });
    }
    let coverage = if failed.is_some() {
        if had_observations {
            GpuCatalogueCoverage::Partial
        } else {
            GpuCatalogueCoverage::Failed
        }
    } else if causes.is_empty() {
        GpuCatalogueCoverage::Complete
    } else {
        GpuCatalogueCoverage::Partial
    };
    if let Some(error) = failed {
        causes.push(GpuCause::Collection(error));
    }
    GpuQualifiedCatalogue {
        devices,
        coverage,
        causes,
    }
}

pub fn normalize(
    observations: Vec<GpuObservation>,
    failed_after_observations: bool,
) -> GpuInventory {
    let mut by_identity: BTreeMap<String, Vec<GpuObservation>> = BTreeMap::new();
    for observation in observations {
        by_identity
            .entry(observation.identity.clone())
            .or_default()
            .push(observation);
    }

    let mut devices = Vec::new();
    let mut warnings = Vec::new();
    for (identity, records) in by_identity {
        let presences: Vec<_> = records.iter().map(|record| record.presence).collect();
        let presence = if presences.iter().all(|value| *value == GpuPresence::Current) {
            GpuPresence::Current
        } else if presences.iter().all(|value| *value == GpuPresence::Absent) {
            GpuPresence::Absent
        } else if presences.iter().all(|value| *value == GpuPresence::Unknown) {
            GpuPresence::Unknown
        } else {
            warnings.push(format!("Présence contradictoire pour {identity}."));
            GpuPresence::Unknown
        };
        if presence == GpuPresence::Unknown {
            warnings.push(format!("Présence indéterminée pour {identity}."));
            continue;
        }
        if presence == GpuPresence::Absent {
            continue;
        }

        let name = consistent(records.iter().map(|record| record.name.as_deref()));
        let state = consistent(records.iter().map(|record| record.state.as_ref()));
        let name = name
            .flatten()
            .map(str::to_owned)
            .filter(|name| !name.trim().is_empty());
        let state = state.flatten().cloned().unwrap_or(GpuState::Unknown);
        if name.is_none() {
            warnings.push(format!("Nom inconnu pour une carte présente ({identity})."));
        }
        if state == GpuState::Unknown {
            warnings.push(format!(
                "État inconnu pour une carte présente ({identity})."
            ));
        }
        devices.push(GpuDevice {
            identity,
            name: name.unwrap_or_else(|| "GPU (nom inconnu)".into()),
            state,
        });
    }
    if failed_after_observations {
        warnings.push(
            "La collecte Windows a échoué avant sa fin ; la liste peut être incomplète.".into(),
        );
    }
    let coverage = if warnings.is_empty() && !failed_after_observations {
        Coverage::Complete
    } else {
        Coverage::Partial
    };
    GpuInventory {
        devices,
        coverage,
        warnings,
    }
}

fn consistent<'a, T: PartialEq + 'a + ?Sized>(
    values: impl Iterator<Item = Option<&'a T>>,
) -> Option<Option<&'a T>> {
    let mut values = values;
    let first = values.next()?;
    if values.all(|value| value == first) {
        Some(first)
    } else {
        None
    }
}
