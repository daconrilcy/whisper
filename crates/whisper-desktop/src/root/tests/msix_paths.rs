use super::distribution_edition::{
    GpuInventoryLogRoute, gpu_inventory_log_route, msix_cpu_parakeet_worker, runtime_roots_from,
};
use std::path::{Path, PathBuf};

#[test]
fn unpackaged_distribution_keeps_assets_and_user_data_in_the_stable_root() {
    let roots = runtime_roots_from(
        None,
        Some(PathBuf::from(r"C:\Users\tester\AppData\Local")),
        None,
    )
    .unwrap()
    .unwrap();
    let stable = PathBuf::from(r"C:\Users\tester\AppData\Local\WhisperTurbo");

    assert!(!roots.msix);
    assert_eq!(roots.assets, stable);
    assert_eq!(roots.data, stable);
    assert_eq!(roots.installation, stable);
}

#[test]
fn packaged_distribution_separates_read_only_assets_from_local_state() {
    let local_folder =
        PathBuf::from(r"C:\Users\tester\AppData\Local\Packages\WhisperTurbo.Test\LocalState");
    let executable =
        Path::new(r"C:\Program Files\WindowsApps\WhisperTurbo.Test\app\whisper-desktop.exe");
    let roots = runtime_roots_from(Some(local_folder.clone()), None, Some(executable))
        .unwrap()
        .unwrap();
    let package_root = PathBuf::from(r"C:\Program Files\WindowsApps\WhisperTurbo.Test");

    assert!(roots.msix);
    assert_eq!(roots.assets, package_root);
    assert_eq!(roots.data, local_folder.join("WhisperTurbo"));
    assert_eq!(roots.installation, package_root);
}

#[test]
fn packaged_distribution_rejects_an_unexpected_executable_layout() {
    let executable = Path::new(r"C:\temp\whisper-desktop.exe");
    let result = runtime_roots_from(
        Some(PathBuf::from(r"C:\Users\tester\LocalState")),
        None,
        Some(executable),
    );
    assert_eq!(
        result.unwrap_err(),
        "MSIX desktop is not under the package app directory"
    );
}

#[test]
fn catalog_cpu_worker_route_uses_package_assets_without_a_personal_root() {
    let local_folder = PathBuf::from(r"C:\Users\tester\LocalState");
    let executable =
        Path::new(r"C:\Program Files\WindowsApps\WhisperTurbo.Test\app\whisper-desktop.exe");
    let roots = runtime_roots_from(Some(local_folder), None, Some(executable))
        .unwrap()
        .unwrap();
    let paths = msix_cpu_parakeet_worker(Some(&roots)).unwrap();

    assert_eq!(
        paths.executable,
        roots.assets.join("worker/whisper-worker-parakeet-cpu.exe")
    );
    assert_eq!(paths.native_dir, roots.assets.join("native/cpu"));
    assert_eq!(
        paths.model,
        roots.assets.join("models/parakeet-tdt-0.6b-v3.q8_0.gguf")
    );
    assert_eq!(paths.runtime_root, roots.assets);
    assert!(msix_cpu_parakeet_worker(None).is_none());
}

#[test]
fn gpu_inventory_log_routes_msix_zip_and_turbo_destinations() {
    let local_state = Path::new(
        r"C:\Users\tester\AppData\Local\Packages\WhisperTurbo.Test\LocalState\WhisperTurbo",
    );
    assert!(matches!(
        gpu_inventory_log_route(Some(local_state), true, false),
        GpuInventoryLogRoute::Explicit(path)
            if path == local_state.join("logs/gpu-inventory.log")
    ));
    assert!(matches!(
        gpu_inventory_log_route(Some(local_state), false, false),
        GpuInventoryLogRoute::Default
    ));
    assert!(matches!(
        gpu_inventory_log_route(None, false, true),
        GpuInventoryLogRoute::Disabled
    ));
}
