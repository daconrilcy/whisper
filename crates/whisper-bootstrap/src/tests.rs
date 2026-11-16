use super::preflight_message;
use whisper_core::cpu_compatibility::{CpuCompatibility, CpuCompatibilityLevel, CpuRequirement};

#[test]
fn cpu_preflight_admits_only_a_verified_v2_level() {
    assert_eq!(
        preflight_message(&compatibility(CpuCompatibilityLevel::X86_64V2)),
        Ok("Ce PC satisfait le prérequis CPU x86-64-v2.")
    );
    assert!(preflight_message(&compatibility(CpuCompatibilityLevel::BelowX86_64V2)).is_err());
    assert!(preflight_message(&compatibility(CpuCompatibilityLevel::Indeterminate)).is_err());
    assert_ne!(
        preflight_message(&compatibility(CpuCompatibilityLevel::BelowX86_64V2)),
        preflight_message(&compatibility(CpuCompatibilityLevel::Indeterminate))
    );
}

fn compatibility(level: CpuCompatibilityLevel) -> CpuCompatibility {
    CpuCompatibility {
        level,
        missing: if level == CpuCompatibilityLevel::BelowX86_64V2 {
            vec![CpuRequirement::Sse42]
        } else {
            Vec::new()
        },
    }
}
