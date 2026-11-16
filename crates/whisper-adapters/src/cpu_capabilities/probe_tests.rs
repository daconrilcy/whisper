use super::{current_cpu_compatibility, read_current};

#[test]
fn public_probe_applies_the_shared_policy_to_current_cpuid() {
    assert_eq!(current_cpu_compatibility(), read_current().compatibility());
}
