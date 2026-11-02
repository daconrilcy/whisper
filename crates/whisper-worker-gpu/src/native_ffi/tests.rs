use super::*;

#[test]
fn native_gpu_attestation_requires_a_fresh_matching_log() {
    assert!(validate_native_gpu_log("", "CUDA0", false).is_err());
    assert!(
        validate_native_gpu_log(
            "whisper_backend_init_gpu: using CUDA0 backend\n",
            "CUDA0",
            false
        )
        .is_ok()
    );
    assert!(
        validate_native_gpu_log(
            "whisper_backend_init_gpu: using CUDA1 backend\n",
            "CUDA0",
            false
        )
        .is_err()
    );
}

#[test]
fn native_gpu_attestation_rejects_failure_after_using_backend() {
    let lines = "whisper_backend_init_gpu: using CUDA0 backend\nwhisper_backend_init_gpu: failed to initialize CUDA0 backend\n";
    assert!(validate_native_gpu_log(lines, "CUDA0", false).is_err());
}

#[test]
fn whisper_position_counts_gpu_and_integrated_gpu_not_cuda_ordinals() {
    let registry = [
        BackendRegistryEntry {
            registry_index: 0,
            gpu_or_igpu: false,
            name: "CPU".into(),
            pci_id: None,
        },
        BackendRegistryEntry {
            registry_index: 1,
            gpu_or_igpu: true,
            name: "CUDA0".into(),
            pci_id: Some("0000:00:00.0".into()),
        },
        BackendRegistryEntry {
            registry_index: 2,
            gpu_or_igpu: true,
            name: "IGPU0".into(),
            pci_id: Some("0000:00:01.0".into()),
        },
        BackendRegistryEntry {
            registry_index: 5,
            gpu_or_igpu: true,
            name: "CUDA1".into(),
            pci_id: Some("0000:01:00.0".into()),
        },
    ];
    assert_eq!(whisper_gpu_position(&registry, 5).unwrap(), 2);
    assert!(whisper_gpu_position(&registry, 0).is_err());
    assert!(validate_selected_cuda_registry_device(&registry[3], "0000:01:00.0").is_ok());
    assert!(validate_selected_cuda_registry_device(&registry[3], "0000:02:00.0").is_err());
    assert!(validate_selected_cuda_registry_device(&registry[2], "0000:00:01.0").is_err());
}
