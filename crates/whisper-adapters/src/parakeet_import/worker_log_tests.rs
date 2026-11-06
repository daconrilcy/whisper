use super::*;

const PERSONAL_LOG_CASE: &str = "WHISPER_PERSONAL_LOG_TEST_CASE";

fn personal_paths(root: &Path) -> ParakeetWorkerPaths {
    let personal = root.join("Whisper");
    ParakeetWorkerPaths {
        executable: personal.join("worker/whisper-worker-parakeet-gpu.exe"),
        native_dir: personal.join("native/gpu"),
        model: personal.join("models/parakeet-tdt-0.6b-v3.q8_0.gguf"),
        runtime_root: personal,
    }
}

fn fixture_root(label: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "parakeet-worker-log-{label}-{}-{stamp}",
        std::process::id()
    ))
}

fn run_personal_log_case(test_name: &str, case: &str, settings_file: bool, logs_file: bool) {
    let root = fixture_root(case);
    let paths = personal_paths(&root);
    std::fs::create_dir_all(&paths.native_dir).unwrap();
    std::fs::create_dir_all(paths.model.parent().unwrap()).unwrap();
    crate::pe_fixture::compile_failing_worker(&root, &paths.executable);
    std::fs::write(&paths.model, b"fixture model").unwrap();
    if settings_file {
        std::fs::write(paths.runtime_root.join("settings"), b"settings sentinel").unwrap();
    } else if logs_file {
        std::fs::create_dir_all(paths.runtime_root.join("settings")).unwrap();
        std::fs::write(paths.runtime_root.join("settings/logs"), b"logs sentinel").unwrap();
    }

    let output = Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg(test_name)
        .arg("--nocapture")
        .env("LOCALAPPDATA", &root)
        .env(PERSONAL_LOG_CASE, case)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "child failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn run_child_case(case: &str) -> bool {
    if std::env::var(PERSONAL_LOG_CASE).ok().as_deref() != Some(case) {
        return false;
    }
    let root = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap());
    let paths = personal_paths(&root);
    assert!(paths.is_personal_runtime());
    true
}

#[test]
fn personal_gpu_log_creates_missing_settings_and_preserves_existing_files() {
    if run_child_case("create") {
        let root = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap());
        let paths = personal_paths(&root);
        let settings = paths.runtime_root.join("settings");
        assert!(!settings.exists());

        let (stderr, first_log) = worker_stderr_log(&paths, false, None).unwrap();
        drop(stderr);
        let first_log = first_log.unwrap();
        assert!(settings.join("logs").is_dir());
        assert!(!settings.join("settings.json").exists());
        assert!(first_log.starts_with(settings.join("logs").canonicalize().unwrap()));

        let preferences = b"existing preferences";
        let old_log = b"existing log contents";
        std::fs::write(settings.join("settings.json"), preferences).unwrap();
        std::fs::write(settings.join("logs/existing.log"), old_log).unwrap();
        let (stderr, second_log) = worker_stderr_log(&paths, false, None).unwrap();
        drop(stderr);
        assert_ne!(first_log, second_log.unwrap());
        assert_eq!(
            std::fs::read(settings.join("settings.json")).unwrap(),
            preferences
        );
        assert_eq!(
            std::fs::read(settings.join("logs/existing.log")).unwrap(),
            old_log
        );
        return;
    }

    run_personal_log_case(
        "parakeet_import::worker_log_tests::personal_gpu_log_creates_missing_settings_and_preserves_existing_files",
        "create",
        false,
        false,
    );
}

#[test]
fn personal_gpu_log_rejects_settings_file_without_replacing_it() {
    if run_child_case("settings-file") {
        let root = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap());
        let paths = personal_paths(&root);
        assert!(matches!(
            worker_stderr_log(&paths, false, None),
            Err(error) if error.starts_with("WorkerLogPath:")
        ));
        assert_eq!(
            std::fs::read(paths.runtime_root.join("settings")).unwrap(),
            b"settings sentinel"
        );
        return;
    }

    run_personal_log_case(
        "parakeet_import::worker_log_tests::personal_gpu_log_rejects_settings_file_without_replacing_it",
        "settings-file",
        true,
        false,
    );
}

#[test]
fn personal_gpu_log_rejects_logs_file_without_replacing_it() {
    if run_child_case("logs-file") {
        let root = PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap());
        let paths = personal_paths(&root);
        assert!(matches!(
            worker_stderr_log(&paths, false, None),
            Err(error) if error.starts_with("WorkerLogPath:")
        ));
        assert_eq!(
            std::fs::read(paths.runtime_root.join("settings/logs")).unwrap(),
            b"logs sentinel"
        );
        return;
    }

    run_personal_log_case(
        "parakeet_import::worker_log_tests::personal_gpu_log_rejects_logs_file_without_replacing_it",
        "logs-file",
        false,
        true,
    );
}
