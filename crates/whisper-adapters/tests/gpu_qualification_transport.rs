use whisper_core::ipc::{GpuQualificationResult, WorkerEvent};

#[test]
fn qualification_response_is_typed_and_keeps_per_device_identity() {
    let event = WorkerEvent::GpusQualified {
        generation: 7,
        build_architectures: "61-real".into(),
        devices: vec![GpuQualificationResult {
            identity: "PCI\\VEN_10DE&DEV_1B06".into(),
            windows_rank: Some(0),
            luid: Some(1),
            pci_id: Some("0000:01:00.0".into()),
            cuda_ordinal: Some(0),
            ggml_ordinal: Some(2),
            identity_match: "Missing".into(),
            support: "Undetermined".into(),
            availability: "NotVerified".into(),
            cause: Some("rank unavailable".into()),
        }],
    };
    let json = serde_json::to_string(&event).unwrap();
    let decoded: WorkerEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded, event);
}
