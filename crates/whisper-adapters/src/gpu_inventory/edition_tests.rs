use super::{InventoryLogDestination, InventoryLogSnapshot, log_inventory};

#[test]
fn turbo_gpu_log_writes_to_whisperturbo_and_preserves_personal_log() {
    let root = std::env::temp_dir()
        .join("gpu-inventory-edition-tests")
        .join(format!("{}-{}", std::process::id(), uuid_suffix()));
    let turbo_log = root.join("WhisperTurbo/logs/gpu-inventory.log");
    let personal_log = root.join("Whisper/gpu-inventory.log");
    std::fs::create_dir_all(personal_log.parent().unwrap()).unwrap();
    std::fs::write(&personal_log, "personal sentinel\n").unwrap();

    log_inventory(
        InventoryLogSnapshot {
            generation: 7,
            observations: &[],
            dxgi: &[],
            targets: &[],
            results: None,
            error: None,
            qualification_error: None,
        },
        &InventoryLogDestination::Explicit(turbo_log.clone()),
    );

    let turbo = std::fs::read_to_string(turbo_log).unwrap();
    assert!(turbo.contains("\"generation\":7"));
    assert_eq!(
        std::fs::read_to_string(&personal_log).unwrap(),
        "personal sentinel\n"
    );
    std::fs::remove_dir_all(root).unwrap();
}

fn uuid_suffix() -> String {
    format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    )
}
