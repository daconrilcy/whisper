use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GpuPresence {
    Current,
    Absent,
    Unknown,
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
