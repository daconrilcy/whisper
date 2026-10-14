use whisper_core::path_settings::StoredPaths;

#[test]
fn old_settings_default_to_windows_microphone_without_losing_paths() {
    let old = r#"{"version":1,"audio_directory":"C:\\Audio","model_path":"C:\\Model.bin","destination":"C:\\Output"}"#;
    let restored: StoredPaths = serde_json::from_str(old).unwrap();
    assert_eq!(restored.microphone_id, None);
    assert_eq!(restored.model_path, "C:\\Model.bin");
    assert_eq!(restored.destination, "C:\\Output");
}

#[test]
fn microphone_id_is_kept_in_the_same_settings_snapshot() {
    let settings = StoredPaths {
        version: 1,
        audio_directory: Some("C:\\Audio".into()),
        model_path: "C:\\Model.bin".into(),
        destination: "C:\\Output".into(),
        microphone_id: Some("wasapi:{device-one}".into()),
    };
    let json = serde_json::to_string(&settings).unwrap();
    let restored: StoredPaths = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, settings);
}
