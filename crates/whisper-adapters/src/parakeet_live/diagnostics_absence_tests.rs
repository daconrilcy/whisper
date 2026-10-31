use std::{
    path::PathBuf,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn legacy_environment_does_not_create_diagnostic_files() {
    const CHILD_ROOT: &str = "WHISPER_P3_DIAGNOSTIC_TEST_ROOT";
    if let Some(root) = std::env::var_os(CHILD_ROOT) {
        let source = PathBuf::from(root).join("artifact.bin");
        std::fs::write(&source, b"diagnostic isolation fixture").unwrap();
        crate::model_catalog::cached_artifact_sha256(&source).unwrap();
        return;
    }

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "parakeet-no-legacy-diagnostics-{}-{stamp}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let r7_log = root.join("r7.jsonl");
    let trace = root.join("confirmation.jsonl");
    for test in [
        "parakeet_live::diagnostics_absence_tests::legacy_environment_does_not_create_diagnostic_files",
        "parakeet_live::microphone_spectrum_tests::parakeet_capture_pump_publishes_frequency_spectrum_and_preserves_wav",
    ] {
        let output = Command::new(std::env::current_exe().unwrap())
            .arg("--exact")
            .arg(test)
            .arg("--nocapture")
            .env(CHILD_ROOT, &root)
            .env("PARAKEET_R7_TIMING_ACTIVE", "1")
            .env("PARAKEET_R7_EXEC_ROOT", &root)
            .env("PARAKEET_R7_TIMING_LOG", &r7_log)
            .env("WHISPER_PARAKEET_CONFIRMATION_TRACE", &trace)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{test}: {}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"),
            "{test} did not execute"
        );
        assert!(!r7_log.exists(), "legacy R7 log was created");
        assert!(!trace.exists(), "legacy confirmation trace was created");
    }
}

#[test]
fn d2_test_diagnostics_remain_available_in_guarded_destination() {
    const CHILD_ROOT: &str = "WHISPER_P3_D2_DIAGNOSTIC_TEST_ROOT";
    if let Some(root) = std::env::var_os(CHILD_ROOT) {
        let root = PathBuf::from(root);
        let diagnostics = super::D2WorkerDiagnostics::for_d2_destination(&root).unwrap();
        assert!(diagnostics.enabled());
        diagnostics.record(serde_json::json!({ "event": "d2-preserved" }));
        let content = std::fs::read_to_string(root.join("d2-worker-events.jsonl")).unwrap();
        assert!(content.contains("d2-preserved"));
        return;
    }
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "parakeet-d2-preserved-{}-{stamp}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("parakeet_live::diagnostics_absence_tests::d2_test_diagnostics_remain_available_in_guarded_destination")
        .arg("--nocapture")
        .env(CHILD_ROOT, &root)
        .env("WHISPER_MIC_LEVEL_D2_ROOT", &root)
        .output().unwrap();
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"));
    assert!(root.join("d2-worker-events.jsonl").is_file());
}
