use std::path::PathBuf;

use whisper_adapters::path_settings::{WindowsModelPreferencePort, WindowsPathPort};
use whisper_core::{
    model_catalog::ModelId,
    path_settings::{LegacyModelPath, ModelPreferencePort, PathPort, StartupComputePreference},
};

const KNOWN_TURBO: &str =
    r"C:\Users\Cyril\AppData\Local\Programs\WhisperBuildDeps\models\ggml-large-v3-turbo.bin";

fn test_dir(name: &str) -> PathBuf {
    let root =
        PathBuf::from(std::env::var_os("WHISPER_L1B_RUNTIME_ROOT").expect("bounded runtime root"));
    let path = root.join(format!("{name}-{}", std::process::id()));
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn v1_bytes(model_path: &str) -> Vec<u8> {
    let model_path = serde_json::to_string(model_path).unwrap();
    format!(
        "{{\"version\":1,\"audio_directory\":\"C:\\\\Audio\",\"model_path\":{model_path},\"destination\":\"C:\\\\Out\",\"microphone_id\":\"mic-4\",\"gpu_device_id\":\"gpu-2\"}}\n"
    )
    .into_bytes()
}

fn backup_path(settings: &std::path::Path) -> PathBuf {
    let mut name = settings.file_name().unwrap().to_os_string();
    name.push(".v1.bak");
    settings.with_file_name(name)
}

#[test]
fn v1_backup_is_created_before_v2_and_is_byte_identical_and_never_replaced() {
    let dir = test_dir("preferences-backup");
    let settings = dir.join("settings.json");
    let original = v1_bytes(KNOWN_TURBO);
    std::fs::write(&settings, &original).unwrap();
    let mut port = WindowsModelPreferencePort::new_at(settings.clone(), KNOWN_TURBO);

    assert!(matches!(
        port.load().unwrap().unwrap().legacy_model_path,
        LegacyModelPath::KnownTurbo(_)
    ));
    assert!(
        port.save(
            1,
            Some(ModelId::WhisperLargeV3Turbo),
            StartupComputePreference::Auto
        )
        .result
        .is_ok()
    );
    assert_eq!(std::fs::read(backup_path(&settings)).unwrap(), original);

    let first_v2 = std::fs::read(&settings).unwrap();
    assert!(
        port.save(
            2,
            Some(ModelId::ParakeetTdt06BV3),
            StartupComputePreference::FollowProbe
        )
        .result
        .is_ok()
    );
    assert_eq!(std::fs::read(backup_path(&settings)).unwrap(), original);
    let current: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&settings).unwrap()).unwrap();
    assert_eq!(current["version"], 2);
    assert_eq!(current["audio_directory"], r"C:\Audio");
    assert_eq!(current["destination"], r"C:\Out");
    assert_eq!(current["microphone_id"], "mic-4");
    assert_eq!(current["gpu_device_id"], "gpu-2");
    assert_eq!(current["preferred_model"], "ParakeetTdt06BV3");
    assert_eq!(current["preference_revision"], 2);
    assert_ne!(first_v2, std::fs::read(&settings).unwrap());

    let mut legacy_port = WindowsPathPort::new_at(settings.clone());
    let mut legacy_paths = legacy_port.load().unwrap().unwrap();
    assert_eq!(legacy_paths.version, 2);
    legacy_paths.destination = r"C:\NewOutput".into();
    legacy_port.save(3, legacy_paths).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        if let Some(ack) = legacy_port.poll_save_acknowledgement() {
            assert_eq!(ack.revision, 3);
            assert!(ack.result.is_ok());
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "settings worker acknowledgement timed out"
        );
        std::thread::yield_now();
    }
    drop(legacy_port);
    let merged: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&settings).unwrap()).unwrap();
    assert_eq!(merged["preferred_model"], "ParakeetTdt06BV3");
    assert_eq!(merged["startup_compute"], "FollowProbe");
    assert_eq!(merged["destination"], r"C:\NewOutput");
}

#[test]
fn path_write_before_preference_migration_preserves_paths_and_v1_backup() {
    let dir = test_dir("path-before-preferences");
    let settings = dir.join("settings.json");
    let original = v1_bytes(KNOWN_TURBO);
    std::fs::write(&settings, &original).unwrap();

    let mut paths = WindowsPathPort::new_at(settings.clone());
    let mut snapshot = paths.load().unwrap().unwrap();
    snapshot.destination = r"C:\NewOutput".into();
    snapshot.microphone_id = Some("mic-new".into());
    paths.save(1, snapshot).unwrap();
    wait_for_path_ack(&mut paths, 1);
    drop(paths);
    let v1_before_migration = std::fs::read(&settings).unwrap();

    let mut preferences = WindowsModelPreferencePort::new_at(settings.clone(), KNOWN_TURBO);
    let acknowledgement = preferences.save(
        1,
        Some(ModelId::ParakeetTdt06BV3),
        StartupComputePreference::Auto,
    );
    assert!(acknowledgement.result.is_ok());
    assert_eq!(
        std::fs::read(backup_path(&settings)).unwrap(),
        v1_before_migration
    );
    let current: serde_json::Value =
        serde_json::from_slice(&std::fs::read(settings).unwrap()).unwrap();
    assert_eq!(current["version"], 2);
    assert_eq!(current["destination"], r"C:\NewOutput");
    assert_eq!(current["microphone_id"], "mic-new");
    assert_eq!(current["preferred_model"], "ParakeetTdt06BV3");
    assert_eq!(current["startup_compute"], "Auto");
}

#[test]
fn concurrent_path_and_preference_writes_merge_after_v1_migration() {
    let dir = test_dir("concurrent-settings-writes");
    let settings = dir.join("settings.json");
    let original = v1_bytes(KNOWN_TURBO);
    std::fs::write(&settings, &original).unwrap();

    let mut paths = WindowsPathPort::new_at(settings.clone());
    let mut stale_paths = paths.load().unwrap().unwrap();
    stale_paths.destination = r"C:\ConcurrentOutput".into();
    stale_paths.audio_directory = Some(r"C:\ConcurrentAudio".into());

    let mut preferences = WindowsModelPreferencePort::new_at(settings.clone(), KNOWN_TURBO);
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    let preference_barrier = barrier.clone();
    let preference_thread = std::thread::spawn(move || {
        preference_barrier.wait();
        preferences.save(
            4,
            Some(ModelId::ParakeetTdt06BV3),
            StartupComputePreference::Auto,
        )
    });
    barrier.wait();
    paths.save(7, stale_paths).unwrap();
    let preference_ack = preference_thread.join().unwrap();
    assert!(preference_ack.result.is_ok());
    wait_for_path_ack(&mut paths, 7);
    drop(paths);

    assert_eq!(std::fs::read(backup_path(&settings)).unwrap(), original);
    let current: serde_json::Value =
        serde_json::from_slice(&std::fs::read(settings).unwrap()).unwrap();
    assert_eq!(current["version"], 2);
    assert_eq!(current["audio_directory"], r"C:\ConcurrentAudio");
    assert_eq!(current["destination"], r"C:\ConcurrentOutput");
    assert_eq!(current["preferred_model"], "ParakeetTdt06BV3");
    assert_eq!(current["startup_compute"], "Auto");
}

fn wait_for_path_ack(paths: &mut WindowsPathPort, revision: u64) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        if let Some(ack) = paths.poll_save_acknowledgement() {
            assert_eq!(ack.revision, revision);
            assert!(ack.result.is_ok(), "{:?}", ack.result);
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "settings worker acknowledgement timed out"
        );
        std::thread::yield_now();
    }
}

#[test]
fn conflicting_or_unwritable_backup_blocks_migration_and_preserves_v1() {
    let dir = test_dir("preferences-conflict");
    let settings = dir.join("settings.json");
    let original = v1_bytes(KNOWN_TURBO);
    std::fs::write(&settings, &original).unwrap();
    std::fs::write(backup_path(&settings), b"different old backup").unwrap();
    let mut port = WindowsModelPreferencePort::new_at(settings.clone(), KNOWN_TURBO);

    let acknowledgement = port.save(
        1,
        Some(ModelId::ParakeetTdt06BV3),
        StartupComputePreference::Auto,
    );
    assert!(
        acknowledgement
            .result
            .unwrap_err()
            .contains("conflictuelle")
    );
    assert_eq!(std::fs::read(&settings).unwrap(), original);
}

#[test]
fn matching_backup_is_reused_once_and_backup_io_failure_leaves_v1_untouched() {
    let dir = test_dir("preferences-existing-backup");
    let settings = dir.join("settings.json");
    let original = v1_bytes(KNOWN_TURBO);
    std::fs::write(&settings, &original).unwrap();
    std::fs::write(backup_path(&settings), &original).unwrap();
    let mut port = WindowsModelPreferencePort::new_at(settings.clone(), KNOWN_TURBO);
    assert!(
        port.save(
            1,
            Some(ModelId::WhisperLargeV3Turbo),
            StartupComputePreference::Auto,
        )
        .result
        .is_ok()
    );
    assert_eq!(std::fs::read(backup_path(&settings)).unwrap(), original);

    let blocked_dir = test_dir("preferences-backup-io-failure");
    let blocked_settings = blocked_dir.join("settings.json");
    let blocked_original = v1_bytes(KNOWN_TURBO);
    std::fs::write(&blocked_settings, &blocked_original).unwrap();
    std::fs::create_dir(backup_path(&blocked_settings)).unwrap();
    let mut blocked_port =
        WindowsModelPreferencePort::new_at(blocked_settings.clone(), KNOWN_TURBO);
    assert!(
        blocked_port
            .save(
                1,
                Some(ModelId::ParakeetTdt06BV3),
                StartupComputePreference::Auto,
            )
            .result
            .is_err()
    );
    assert_eq!(std::fs::read(&blocked_settings).unwrap(), blocked_original);
}

#[test]
fn unknown_v1_path_is_retained_for_diagnostics_and_does_not_map_to_turbo() {
    let dir = test_dir("preferences-unknown");
    let settings = dir.join("settings.json");
    let unknown = r"D:\archive\renamed.bin";
    std::fs::write(&settings, v1_bytes(unknown)).unwrap();
    let port = WindowsModelPreferencePort::new_at(settings, KNOWN_TURBO);

    let loaded = port.load().unwrap().unwrap();
    assert_eq!(loaded.preferred_model, None);
    assert_eq!(
        loaded.legacy_model_path,
        LegacyModelPath::Unknown(unknown.into())
    );
}
