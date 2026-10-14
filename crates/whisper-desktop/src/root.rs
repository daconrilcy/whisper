use std::path::PathBuf;
use whisper_adapters::{
    supervisor::WorkerPaths,
    worker_ipc::{AsyncImportIo, probe_cuda_worker},
};
use whisper_core::{ComputeChoice, ImportApplication};

pub fn compose() -> (
    ImportApplication<AsyncImportIo>,
    ComputeChoice,
    Option<String>,
) {
    let executable = std::env::current_exe().ok();
    let worker_path = |worker_name: &str| {
        let sibling = executable
            .as_ref()
            .and_then(|path| path.parent().map(|parent| parent.join(worker_name)));
        let isolated = executable.as_ref().and_then(|path| {
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
        [sibling.clone(), isolated]
            .into_iter()
            .flatten()
            .find(|path| path.is_file())
            .or(sibling)
            .unwrap_or_else(|| PathBuf::from(worker_name))
    };
    let workers = WorkerPaths {
        cpu: worker_path("whisper-worker-cpu.exe"),
        gpu: worker_path("whisper-worker-gpu.exe"),
    };
    let gpu_status = probe_cuda_worker(
        &workers.gpu,
        &whisper_desktop::ui::default_model_path(),
        whisper_desktop::ui::approved_model_sha256(),
    );
    let (choice, message) = match gpu_status {
        Ok(()) => (ComputeChoice::Gpu, None),
        Err(message) => (ComputeChoice::Cpu, Some(message)),
    };
    (
        ImportApplication::new(AsyncImportIo::start_with_workers(workers)),
        choice,
        message,
    )
}
