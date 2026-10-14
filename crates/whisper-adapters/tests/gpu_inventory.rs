use whisper_adapters::gpu_inventory::map_device_fields;
use whisper_core::gpu_inventory::{GpuPresence, GpuState};

#[test]
fn maps_integrated_gpu_as_current_without_cuda_eligibility_inference() {
    let device = map_device_fields(
        Some("PCI\\VEN_8086&DEV_XXXX".into()),
        Some("Intel(R) UHD Graphics".into()),
        Some(0),
        Some(true),
    );
    assert_eq!(device.presence, GpuPresence::Current);
    assert_eq!(device.state, Some(GpuState::Ready));
    assert_eq!(device.name.as_deref(), Some("Intel(R) UHD Graphics"));
}

#[test]
fn maps_disabled_present_card_and_preserves_ambiguous_windows_error_codes() {
    let disabled = map_device_fields(Some("id".into()), None, Some(22), Some(true));
    assert_eq!(disabled.presence, GpuPresence::Current);
    assert_eq!(disabled.state, Some(GpuState::Disabled));
    assert!(disabled.name.is_none());
    let ambiguous = map_device_fields(Some("old".into()), Some("Old".into()), Some(24), None);
    assert_eq!(ambiguous.presence, GpuPresence::Unknown);
    assert_eq!(ambiguous.state, Some(GpuState::Error(24)));
}

#[test]
fn missing_presence_is_unknown_and_false_presence_is_absent() {
    assert_eq!(
        map_device_fields(Some("a".into()), None, None, None).presence,
        GpuPresence::Unknown
    );
    assert_eq!(
        map_device_fields(Some("b".into()), None, Some(0), Some(false)).presence,
        GpuPresence::Absent
    );
}

#[test]
fn missing_identity_cannot_be_claimed_as_a_current_device() {
    let device = map_device_fields(None, Some("Name".into()), Some(0), Some(true));
    assert_eq!(device.presence, GpuPresence::Unknown);
}
