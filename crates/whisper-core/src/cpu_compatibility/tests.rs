use super::*;

fn complete_v2() -> CpuCapabilities {
    CpuCapabilities {
        max_basic_leaf: Some(7),
        max_extended_leaf: Some(0x8000_0001),
        leaf_1_ecx: Some((1 << 0) | (1 << 9) | (1 << 13) | (1 << 19) | (1 << 20) | (1 << 23)),
        leaf_1_edx: Some(
            (1 << 0) | (1 << 8) | (1 << 15) | (1 << 23) | (1 << 24) | (1 << 25) | (1 << 26),
        ),
        leaf_7_0_ebx: Some(0),
        extended_leaf_1_ecx: Some(1),
    }
}

#[test]
fn accepts_complete_x86_64_v2_requirements() {
    assert_eq!(
        complete_v2().compatibility(),
        CpuCompatibility {
            level: CpuCompatibilityLevel::X86_64V2,
            missing: Vec::new(),
        }
    );
}

#[test]
fn identifies_each_missing_v2_requirement() {
    let all = complete_v2();
    for (register, requirements) in [(0, LEAF_1_ECX), (1, LEAF_1_EDX), (2, EXTENDED_LEAF_1_ECX)] {
        for &(feature, bit) in requirements {
            let mut candidate = all;
            match register {
                0 => candidate.leaf_1_ecx = Some(candidate.leaf_1_ecx.unwrap() & !(1 << bit)),
                1 => candidate.leaf_1_edx = Some(candidate.leaf_1_edx.unwrap() & !(1 << bit)),
                _ => {
                    candidate.extended_leaf_1_ecx =
                        Some(candidate.extended_leaf_1_ecx.unwrap() & !(1 << bit));
                }
            }
            let result = candidate.compatibility();
            assert_eq!(result.level, CpuCompatibilityLevel::BelowX86_64V2);
            assert_eq!(result.missing, vec![feature]);
        }
    }
}

#[test]
fn missing_required_leaf_is_indeterminate() {
    let mut candidate = complete_v2();
    candidate.extended_leaf_1_ecx = None;
    assert_eq!(
        candidate.compatibility(),
        CpuCompatibility {
            level: CpuCompatibilityLevel::Indeterminate,
            missing: Vec::new(),
        }
    );
}

#[test]
fn v3_and_v4_bits_are_observed_without_changing_the_v2_floor() {
    let mut candidate = complete_v2();
    candidate.leaf_1_ecx = Some(
        candidate.leaf_1_ecx.unwrap() | (1 << 12) | (1 << 22) | (1 << 27) | (1 << 28) | (1 << 29),
    );
    candidate.leaf_7_0_ebx = Some(
        (1 << 3) | (1 << 5) | (1 << 8) | (1 << 16) | (1 << 17) | (1 << 28) | (1 << 30) | (1 << 31),
    );
    candidate.extended_leaf_1_ecx = Some(1 | (1 << 5));
    assert_eq!(
        candidate.compatibility().level,
        CpuCompatibilityLevel::X86_64V2
    );
    assert_ne!(candidate.leaf_1_ecx.unwrap() & (1 << 12), 0); // FMA
    assert_ne!(candidate.leaf_1_ecx.unwrap() & (1 << 22), 0); // MOVBE
    assert_ne!(candidate.leaf_1_ecx.unwrap() & (1 << 27), 0); // OSXSAVE
    assert_ne!(candidate.leaf_1_ecx.unwrap() & (1 << 28), 0); // AVX
    assert_ne!(candidate.leaf_1_ecx.unwrap() & (1 << 29), 0); // F16C
    assert_ne!(candidate.leaf_7_0_ebx.unwrap() & (1 << 3), 0); // BMI1
    assert_ne!(candidate.leaf_7_0_ebx.unwrap() & (1 << 5), 0); // AVX2
    assert_ne!(candidate.leaf_7_0_ebx.unwrap() & (1 << 8), 0); // BMI2
    assert_ne!(candidate.leaf_7_0_ebx.unwrap() & (1 << 16), 0); // AVX512F
    assert_ne!(candidate.leaf_7_0_ebx.unwrap() & (1 << 17), 0); // AVX512DQ
    assert_ne!(candidate.leaf_7_0_ebx.unwrap() & (1 << 28), 0); // AVX512CD
    assert_ne!(candidate.leaf_7_0_ebx.unwrap() & (1 << 30), 0); // AVX512BW
    assert_ne!(candidate.leaf_7_0_ebx.unwrap() & (1 << 31), 0); // AVX512VL
    assert_ne!(candidate.extended_leaf_1_ecx.unwrap() & (1 << 5), 0); // LZCNT
}

#[test]
fn diagnostics_recompute_the_compatibility_result() {
    let diagnostic = CpuCapabilityDiagnostic::new(CpuProcessRole::WorkerCpu, 42, complete_v2());
    assert!(diagnostic.is_consistent());

    let mut invalid = diagnostic;
    invalid.compatibility.level = CpuCompatibilityLevel::BelowX86_64V2;
    assert!(!invalid.is_consistent());
}
