use serde::{Deserialize, Serialize};

const LEAF_1_ECX: &[(CpuRequirement, u8)] = &[
    (CpuRequirement::Sse3, 0),
    (CpuRequirement::Ssse3, 9),
    (CpuRequirement::Cx16, 13),
    (CpuRequirement::Sse41, 19),
    (CpuRequirement::Sse42, 20),
    (CpuRequirement::Popcnt, 23),
];
const LEAF_1_EDX: &[(CpuRequirement, u8)] = &[
    (CpuRequirement::Fpu, 0),
    (CpuRequirement::Cx8, 8),
    (CpuRequirement::Cmov, 15),
    (CpuRequirement::Fxsr, 24),
    (CpuRequirement::Mmx, 23),
    (CpuRequirement::Sse, 25),
    (CpuRequirement::Sse2, 26),
];
const EXTENDED_LEAF_1_ECX: &[(CpuRequirement, u8)] = &[(CpuRequirement::LahfSahf, 0)];

pub const DIAGNOSTIC_PREFIX: &str = "WHISPER_CPU_CAPABILITIES_V1";
pub const MAX_DIAGNOSTIC_BYTES: usize = 1024;

/// Raw registers needed to evaluate the x86-64-v2 floor and inspect v3/v4 flags.
#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CpuCapabilities {
    pub max_basic_leaf: Option<u32>,
    pub max_extended_leaf: Option<u32>,
    pub leaf_1_ecx: Option<u32>,
    pub leaf_1_edx: Option<u32>,
    pub leaf_7_0_ebx: Option<u32>,
    pub extended_leaf_1_ecx: Option<u32>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CpuCompatibilityLevel {
    #[serde(rename = "x86_64_v2")]
    X86_64V2,
    #[serde(rename = "below_x86_64_v2")]
    BelowX86_64V2,
    #[serde(rename = "indeterminate")]
    Indeterminate,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CpuRequirement {
    Fpu,
    Cx8,
    Cmov,
    Fxsr,
    Mmx,
    Sse,
    Sse2,
    Sse3,
    Ssse3,
    Cx16,
    Sse41,
    Sse42,
    Popcnt,
    LahfSahf,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CpuCompatibility {
    pub level: CpuCompatibilityLevel,
    pub missing: Vec<CpuRequirement>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CpuProcessRole {
    Desktop,
    WorkerCpu,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CpuCapabilityDiagnostic {
    pub role: CpuProcessRole,
    pub pid: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_sha256: Option<String>,
    pub registers: CpuCapabilities,
    pub compatibility: CpuCompatibility,
}

impl CpuCapabilities {
    /// Computes the v2 floor from already-read process-local CPUID values.
    pub fn compatibility(self) -> CpuCompatibility {
        if self.max_basic_leaf.is_none()
            || self.max_extended_leaf.is_none()
            || self.max_basic_leaf.is_some_and(|leaf| leaf < 1)
            || self
                .max_extended_leaf
                .is_some_and(|leaf| leaf < 0x8000_0001)
            || self.leaf_1_ecx.is_none()
            || self.leaf_1_edx.is_none()
            || self.extended_leaf_1_ecx.is_none()
        {
            return CpuCompatibility {
                level: CpuCompatibilityLevel::Indeterminate,
                missing: Vec::new(),
            };
        }

        let mut missing = Vec::new();
        append_missing(
            self.leaf_1_ecx.unwrap_or_default(),
            LEAF_1_ECX,
            &mut missing,
        );
        append_missing(
            self.leaf_1_edx.unwrap_or_default(),
            LEAF_1_EDX,
            &mut missing,
        );
        append_missing(
            self.extended_leaf_1_ecx.unwrap_or_default(),
            EXTENDED_LEAF_1_ECX,
            &mut missing,
        );

        CpuCompatibility {
            level: if missing.is_empty() {
                CpuCompatibilityLevel::X86_64V2
            } else {
                CpuCompatibilityLevel::BelowX86_64V2
            },
            missing,
        }
    }
}

impl CpuCapabilityDiagnostic {
    pub fn new(role: CpuProcessRole, pid: u32, registers: CpuCapabilities) -> Self {
        Self {
            role,
            pid,
            image_sha256: None,
            compatibility: registers.compatibility(),
            registers,
        }
    }

    pub fn is_consistent(&self) -> bool {
        self.compatibility == self.registers.compatibility()
    }
}

fn append_missing(
    register: u32,
    requirements: &[(CpuRequirement, u8)],
    missing: &mut Vec<CpuRequirement>,
) {
    for &(requirement, bit) in requirements {
        if register & (1_u32 << bit) == 0 {
            missing.push(requirement);
        }
    }
}

#[cfg(test)]
mod tests;
