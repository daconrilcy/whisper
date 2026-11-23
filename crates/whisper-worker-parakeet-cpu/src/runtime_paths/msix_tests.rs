use super::msix_cpu_package_root_is_allowed;
use std::path::Path;

#[test]
fn accepts_the_cpu_worker_runtime_only_from_its_msix_package() {
    let windows_apps = Path::new(r"C:\Program Files\WindowsApps");
    let package_root = windows_apps.join("WhisperTurbo.Test_1.0.0.0_x64__test");
    let executable = package_root.join("worker/whisper-worker-parakeet-cpu.exe");

    assert!(msix_cpu_package_root_is_allowed(
        &package_root,
        &executable,
        windows_apps
    ));
}

#[test]
fn rejects_non_package_and_personal_runtime_roots_in_msix_layout() {
    let windows_apps = Path::new(r"C:\Program Files\WindowsApps");
    let package_root = windows_apps.join("WhisperTurbo.Test_1.0.0.0_x64__test");
    let executable = package_root.join("worker/whisper-worker-parakeet-cpu.exe");
    let personal_root = Path::new(r"C:\Users\tester\AppData\Local\WhisperTurbo");
    let foreign_package = windows_apps.join("OtherPackage_1.0.0.0_x64__test");

    assert!(!msix_cpu_package_root_is_allowed(
        personal_root,
        &executable,
        windows_apps
    ));
    assert!(!msix_cpu_package_root_is_allowed(
        &foreign_package,
        &executable,
        windows_apps
    ));
    assert!(!msix_cpu_package_root_is_allowed(
        &package_root,
        Path::new(r"C:\Temp\worker\whisper-worker-parakeet-cpu.exe"),
        windows_apps
    ));
    assert!(!msix_cpu_package_root_is_allowed(
        &package_root,
        &package_root.join("app/whisper-worker-parakeet-cpu.exe"),
        windows_apps
    ));
    assert!(!msix_cpu_package_root_is_allowed(
        &package_root,
        &package_root.join("worker/whisper-worker-parakeet-gpu.exe"),
        windows_apps
    ));
}
