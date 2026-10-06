use std::path::PathBuf;
use whisper_adapters::worker_ipc::AsyncImportIo;
use whisper_core::ImportApplication;

pub fn compose() -> ImportApplication<AsyncImportIo> {
    let executable = std::env::current_exe().ok();
    let worker_name = "whisper-worker-cpu.exe";
    let sibling = executable
        .as_ref()
        .and_then(|path| path.parent().map(|parent| parent.join(worker_name)));
    let isolated_cpu = executable.as_ref().and_then(|path| {
        let release = path.parent()?;
        let target = release.parent()?;
        let package_target = target.parent()?;
        Some(
            package_target
                .parent()?
                .join("cpu")
                .join(target.file_name()?)
                .join(release.file_name()?)
                .join(worker_name),
        )
    });
    let worker = [sibling.clone(), isolated_cpu]
        .into_iter()
        .flatten()
        .find(|path| path.is_file())
        .or(sibling)
        .unwrap_or_else(|| PathBuf::from(worker_name));
    ImportApplication::new(AsyncImportIo::start(worker))
}
