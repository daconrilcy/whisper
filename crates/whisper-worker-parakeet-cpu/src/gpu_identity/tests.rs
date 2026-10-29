use super::*;

#[cfg(feature = "parakeet-cuda")]
#[test]
fn nvidia_inventory_parser_requires_exactly_one_consistent_gpu_row() {
    let gpu = parse_single_nvidia_gpu(
        "count, name, pci.bus_id, uuid\n1, NVIDIA GeForce GTX 1080 Ti, 00000000:01:00.0, GPU-test\n",
    )
    .unwrap();
    assert_eq!(gpu.pci_bus_id, "00000000:01:00.0");
    assert_eq!(gpu.uuid, "GPU-test");

    assert!(
        parse_single_nvidia_gpu(
            "count, name, pci.bus_id, uuid\n2, NVIDIA GeForce GTX, 00000000:01:00.0, GPU-test\n"
        )
        .is_err()
    );
    assert!(parse_single_nvidia_gpu(
        "count, name, pci.bus_id, uuid\n1, NVIDIA GPU, 00000000:01:00.0, GPU-a\n1, NVIDIA GPU, 00000000:02:00.0, GPU-b\n"
    )
    .is_err());
    assert!(parse_single_nvidia_gpu("not an inventory row").is_err());
}
