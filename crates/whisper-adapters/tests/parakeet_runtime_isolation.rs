mod parakeet_import {
    pub use whisper_adapters::parakeet_import::{ParakeetProcess, ParakeetWorkerPaths};
}

#[path = "support/pe_fixture.rs"]
mod pe_fixture;

mod support {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/support/parakeet_runtime_isolation.rs"
    ));

    #[test]
    fn personal_import_is_independent_of_historical_environment() {
        run_matrix(|paths, _session| ParakeetProcess::spawn(paths));
    }
}

#[test]
fn historical_gpu_rejects_invalid_log_root_and_qualification() {
    use parakeet_import::{ParakeetProcess, ParakeetWorkerPaths};
    use std::{
        path::PathBuf,
        process::Command,
        time::{SystemTime, UNIX_EPOCH},
    };

    const CHILD: &str = "WHISPER_P2_HISTORICAL_ROOT";
    if let Some(root) = std::env::var_os(CHILD) {
        let root = PathBuf::from(root);
        let paths = ParakeetWorkerPaths {
            executable: root.join("worker/whisper-worker-parakeet-gpu.exe"),
            native_dir: root.join("native"),
            model: root.join("model"),
            runtime_root: root.clone(),
        };
        paths.validate().unwrap();
        assert!(paths.gpu_qualification_key("import-single-pass").is_err());
        let error = match ParakeetProcess::spawn(&paths) {
            Ok(_) => panic!("historical GPU accepted an invalid campaign root"),
            Err(error) => error,
        };
        let expected = std::env::var("WHISPER_P2_EXPECTED_ERROR").unwrap();
        assert!(error.starts_with(&expected), "unexpected refusal: {error}");
        return;
    }
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "parakeet-p2-historical-{}-{stamp}",
        std::process::id()
    ));
    std::fs::create_dir_all(root.join("worker")).unwrap();
    std::fs::create_dir_all(root.join("native")).unwrap();
    pe_fixture::copy_current_executable(&root.join("worker/whisper-worker-parakeet-gpu.exe"));
    std::fs::write(root.join("model"), b"fixture").unwrap();
    for (variable, expected) in [
        ("PARAKEET_L3C_EXEC_ROOT", "WorkerLogPath: L3c"),
        ("PARAKEET_L3D_EXEC_ROOT", "WorkerLogPath: L3d"),
    ] {
        let output = Command::new(std::env::current_exe().unwrap())
            .arg("--exact")
            .arg("historical_gpu_rejects_invalid_log_root_and_qualification")
            .arg("--nocapture")
            .env(CHILD, &root)
            .env("WHISPER_P2_EXPECTED_ERROR", expected)
            .env("LOCALAPPDATA", root.join("nonpersonal"))
            .env_remove("PARAKEET_L3C_EXEC_ROOT")
            .env_remove("PARAKEET_L3D_EXEC_ROOT")
            .env(variable, root.join("missing-campaign"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"));
    }
}
