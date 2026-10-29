use super::*;
use whisper_core::gpu_inventory::GpuRuntimeIdentity;

fn identity() -> GpuRuntimeIdentity {
    GpuRuntimeIdentity {
        generation: 1,
        pnp_instance_id: "PCI\\GPU".into(),
        dxgi_rank: 1,
        luid: 42,
        pci_id: "10DE:1234".into(),
        cuda_ordinal: 0,
        ggml_ordinal: 2,
    }
}

#[test]
fn cache_target_matches_complete_device_identity() {
    let first = identity();
    let mut equivalent = first.clone();
    equivalent.pnp_instance_id = equivalent.pnp_instance_id.to_ascii_lowercase();
    equivalent.pci_id = equivalent.pci_id.to_ascii_lowercase();
    assert!(same_gpu_target(Some(&first), Some(&equivalent)));
    equivalent.luid += 1;
    assert!(!same_gpu_target(Some(&first), Some(&equivalent)));
    assert!(!same_gpu_target(Some(&first), None));
    assert!(same_gpu_target(None, None));
}

#[test]
fn cached_engine_is_attested_again_and_evicted_on_failure() {
    let mut cache = Some(42);
    let mut calls = 0;
    attest_cached(&mut cache, |value| {
        calls += 1;
        assert_eq!(*value, 42);
        Ok(())
    })
    .unwrap();
    assert_eq!(calls, 1);
    assert_eq!(cache, Some(42));
    let error = attest_cached(&mut cache, |_| Err("attestation failed".into())).unwrap_err();
    assert_eq!(error, "attestation failed");
    assert_eq!(cache, None);
}
