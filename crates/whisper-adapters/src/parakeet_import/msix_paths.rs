use std::path::Path;

use super::ParakeetWorkerPaths;

pub(super) fn worker_paths(package_root: &Path) -> ParakeetWorkerPaths {
    ParakeetWorkerPaths {
        executable: package_root.join("worker/whisper-worker-parakeet-cpu.exe"),
        native_dir: package_root.join("native/cpu"),
        model: package_root.join("models/parakeet-tdt-0.6b-v3.q8_0.gguf"),
        runtime_root: package_root.to_path_buf(),
    }
}

pub(super) fn matches_package(
    paths: &ParakeetWorkerPaths,
    root: &Path,
    desktop_executable: Option<&Path>,
) -> bool {
    let Some(desktop_executable) = desktop_executable.and_then(|path| path.canonicalize().ok())
    else {
        return false;
    };
    let Some(app_directory) = desktop_executable.parent() else {
        return false;
    };
    let Some(package_root) = app_directory.parent() else {
        return false;
    };
    let under_windows_apps = package_root.ancestors().any(|path| {
        path.file_name()
            .is_some_and(|name| name.eq_ignore_ascii_case("WindowsApps"))
    });
    if !under_windows_apps
        || app_directory.file_name().is_none_or(|name| name != "app")
        || desktop_executable
            .file_name()
            .is_none_or(|name| name != "whisper-desktop.exe")
        || package_root != root
    {
        return false;
    }
    paths.executable.canonicalize().ok().as_deref()
        == Some(
            root.join("worker/whisper-worker-parakeet-cpu.exe")
                .as_path(),
        )
        && paths.native_dir.canonicalize().ok().as_deref()
            == Some(root.join("native/cpu").as_path())
        && paths.model.canonicalize().ok().as_deref()
            == Some(root.join("models/parakeet-tdt-0.6b-v3.q8_0.gguf").as_path())
}
