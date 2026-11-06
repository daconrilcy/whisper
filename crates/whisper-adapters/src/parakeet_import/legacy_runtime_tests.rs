use super::historical_runtime::{
    l3b_r6_cpu_read_runtime_root, l3b_r6_cpu_read_runtime_root_in_repository, validate_r6_read_root,
};
use super::*;
#[cfg(windows)]
use std::os::windows::fs::OpenOptionsExt;

fn fixture_root(label: &str) -> PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("parakeet-{label}-{}-{stamp}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    root
}

fn personal_paths(root: &Path, backend: &str) -> ParakeetWorkerPaths {
    ParakeetWorkerPaths {
        executable: root.join(format!("worker/whisper-worker-parakeet-{backend}.exe")),
        native_dir: root.join(format!("native/{backend}")),
        model: root.join("models/parakeet-tdt-0.6b-v3.q8_0.gguf"),
        runtime_root: root.to_path_buf(),
    }
}

#[test]
fn personal_cpu_and_gpu_do_not_require_l3b() {
    const CHILD_ROOT: &str = "WHISPER_PERSONAL_NO_L3B_TEST_ROOT";
    if let Some(root) = std::env::var_os(CHILD_ROOT) {
        let root = PathBuf::from(root);
        let personal = root.join("Whisper");
        assert!(!root.join("target/parakeet-tests/l3/l3b").exists());
        for backend in ["cpu", "gpu"] {
            let paths = personal_paths(&personal, backend);
            assert!(paths.is_personal_runtime());
            paths.validate().unwrap();
            assert_eq!(l3b_r6_cpu_read_runtime_root(&paths).unwrap(), None);
            assert_eq!(
                l3b_r6_cpu_read_runtime_root_in_repository(&paths, &root, None, None).unwrap(),
                None
            );
            let _exclusive_handle = std::fs::OpenOptions::new()
                .read(true)
                .share_mode(0)
                .open(&paths.executable)
                .expect("hold valid PE worker exclusively for launch failure");
            let result = ParakeetProcess::spawn(&paths);
            assert!(
                matches!(result, Err(ref error) if error.starts_with("WorkerStart:")),
                "the exclusive handle must prevent the valid PE worker from starting"
            );
            if backend == "cpu" {
                assert!(
                    !personal.join("settings").exists(),
                    "the CPU worker without a diagnostic log must not initialize settings"
                );
            } else {
                assert!(personal.join("settings/logs").is_dir());
                assert!(!personal.join("settings/settings.json").exists());
            }
        }
        return;
    }

    let root = fixture_root("personal-no-l3b");
    let personal = root.join("Whisper");
    assert!(!personal.join("settings").exists());
    for backend in ["cpu", "gpu"] {
        let paths = personal_paths(&personal, backend);
        std::fs::create_dir_all(paths.executable.parent().unwrap()).unwrap();
        std::fs::create_dir_all(&paths.native_dir).unwrap();
        std::fs::create_dir_all(paths.model.parent().unwrap()).unwrap();
        crate::pe_fixture::compile_failing_worker(&root, &paths.executable);
        std::fs::write(&paths.model, b"fixture model").unwrap();
    }
    let output = Command::new(std::env::current_exe().unwrap())
        .arg("--exact")
        .arg("parakeet_import::legacy_runtime_tests::personal_cpu_and_gpu_do_not_require_l3b")
        .arg("--nocapture")
        .env("LOCALAPPDATA", &root)
        .env(CHILD_ROOT, &root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "child failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn historical_cpu_requires_exact_worker_configuration_and_hash() {
    let repository = fixture_root("historical-r6");
    let l3b = repository.join("target/parakeet-tests/l3/l3b");
    let l2b = repository.join("target/parakeet-tests/l2/l2b");
    let worker = l2b.join("r6/worker/whisper-worker-parakeet-cpu.exe");
    std::fs::create_dir_all(&l3b).unwrap();
    std::fs::create_dir_all(worker.parent().unwrap()).unwrap();
    crate::pe_fixture::compile_failing_worker(&repository, &worker);
    let paths = ParakeetWorkerPaths {
        executable: worker.clone(),
        native_dir: l3b.join("native"),
        model: l3b.join("model"),
        runtime_root: l3b,
    };
    let check = |configured_worker: Option<PathBuf>, configured_root: Option<PathBuf>| {
        l3b_r6_cpu_read_runtime_root_in_repository(
            &paths,
            &repository,
            configured_worker,
            configured_root,
        )
    };
    assert!(
        check(None, None)
            .unwrap_err()
            .starts_with("WorkerPathMissing")
    );
    assert!(
        check(Some(repository.clone()), None)
            .unwrap_err()
            .starts_with("WorkerPathMismatch")
    );
    assert!(
        check(Some(worker), Some(l2b.clone()))
            .unwrap_err()
            .starts_with("WorkerHashMismatch")
    );
    let l2b = l2b.canonicalize().unwrap();
    let other = repository.canonicalize().unwrap();
    assert!(validate_r6_read_root(&l2b, &l2b).is_ok());
    assert!(
        validate_r6_read_root(&other, &l2b)
            .unwrap_err()
            .starts_with("RuntimeRootMismatch")
    );
}
