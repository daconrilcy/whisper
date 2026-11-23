use super::{
    PARAKEET_PROFILE_IMPORT_SINGLE_PASS, ParakeetProcess, ParakeetWorkerPaths,
    personal_runtime_path_is_allowed, worker_log_prefix,
};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[test]
fn msix_cpu_paths_are_confined_to_the_package_runtime() {
    let root = Path::new(r"C:\Program Files\WindowsApps\WhisperTurbo.Test");
    let paths = ParakeetWorkerPaths::from_msix_cpu_package(root);

    assert_eq!(
        paths.executable,
        root.join("worker/whisper-worker-parakeet-cpu.exe")
    );
    assert_eq!(paths.native_dir, root.join("native/cpu"));
    assert_eq!(
        paths.model,
        root.join("models/parakeet-tdt-0.6b-v3.q8_0.gguf")
    );
    assert_eq!(paths.runtime_root, root);
}

#[test]
fn validates_and_spawns_from_package_paths_without_repository_access() {
    let (root, desktop, paths) = package_fixture();
    assert!(
        paths
            .validate_with_desktop_executable(Some(&desktop))
            .is_ok()
    );

    let process = ParakeetProcess::spawn_profile_with_desktop_executable(
        &paths,
        PARAKEET_PROFILE_IMPORT_SINGLE_PASS,
        None,
        Some(desktop),
    )
    .expect("MSIX spawn uses the package runtime without CARGO_MANIFEST_DIR");
    drop(process);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn msix_validation_rejects_personal_and_foreign_assets() {
    let (root, desktop, paths) = package_fixture();
    let personal = PathBuf::from(r"C:\Users\tester\AppData\Local\WhisperTurbo");
    let wrong_root_paths = ParakeetWorkerPaths {
        runtime_root: personal,
        ..paths.clone()
    };
    assert!(
        wrong_root_paths
            .validate_with_desktop_executable(Some(&desktop))
            .is_err()
    );

    let mut foreign = paths;
    foreign.model = root.join("other-package/models/parakeet-tdt-0.6b-v3.q8_0.gguf");
    assert!(
        foreign
            .validate_with_desktop_executable(Some(&desktop))
            .is_err()
    );
    fs::remove_dir_all(root).unwrap();
}

fn package_fixture() -> (PathBuf, PathBuf, ParakeetWorkerPaths) {
    let fixture = std::env::temp_dir().join(format!(
        "whisper-msix-parakeet-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let root = fixture.join("WindowsApps/WhisperTurbo.Test_1.0.0.0_x64__test");
    let desktop = root.join("app/whisper-desktop.exe");
    fs::create_dir_all(root.join("app")).unwrap();
    fs::create_dir_all(root.join("worker")).unwrap();
    fs::create_dir_all(root.join("native/cpu")).unwrap();
    fs::create_dir_all(root.join("models")).unwrap();
    crate::pe_fixture::copy_verified_pe(
        &std::env::current_exe().expect("test executable path"),
        &desktop,
    );
    fs::write(
        root.join("models/parakeet-tdt-0.6b-v3.q8_0.gguf"),
        b"model fixture",
    )
    .unwrap();
    crate::pe_fixture::copy_verified_pe(
        &std::env::current_exe().expect("test executable path"),
        &root.join("worker/whisper-worker-parakeet-cpu.exe"),
    );
    (
        fixture,
        desktop,
        ParakeetWorkerPaths::from_msix_cpu_package(&root),
    )
}

#[test]
fn paths_reject_a_worker_outside_the_runtime_root() {
    let root = std::env::var_os("PARAKEET_L2_RUNTIME_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("whisper-parakeet-runtime-tests"));
    fs::create_dir_all(&root).unwrap();
    let paths = ParakeetWorkerPaths {
        executable: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
        native_dir: root.join("native"),
        model: root.join("runtime/model/parakeet-tdt-0.6b-v3.q8_0.gguf"),
        runtime_root: root,
    };
    assert!(
        paths
            .validate()
            .unwrap_err()
            .starts_with("PathOutsideRuntime")
    );
}

#[test]
fn personal_runtime_root_is_published_or_direct_staging_child_only() {
    let personal = Path::new(r"C:\Users\tester\AppData\Local\Whisper");
    assert!(personal_runtime_path_is_allowed(personal, personal));
    assert!(personal_runtime_path_is_allowed(
        Path::new(r"C:\Users\tester\AppData\Local\Whisper\.staging\candidate"),
        personal,
    ));
    assert!(!personal_runtime_path_is_allowed(
        Path::new(r"C:\dev\whisper\target\parakeet-tests\l6-perso"),
        personal,
    ));
    assert!(!personal_runtime_path_is_allowed(
        Path::new(r"C:\Users\tester\AppData\Local\Whisper\.staging\candidate\nested"),
        personal,
    ));
}

#[test]
fn worker_log_prefix_preserves_historical_modes() {
    assert_eq!(worker_log_prefix(true, false, false), "parakeet-asr");
    assert_eq!(worker_log_prefix(true, true, false), "l3c-parakeet-asr");
    assert_eq!(worker_log_prefix(true, false, true), "l3d-parakeet-asr");
    assert_eq!(
        worker_log_prefix(false, false, false),
        "parakeet-worker-cpu"
    );
}
