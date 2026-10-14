use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver},
};
use whisper_adapters::{
    path_settings::WindowsPathPort,
    supervisor::WorkerPaths,
    worker_ipc::{AsyncImportIo, probe_cuda_worker},
};
use whisper_core::path_settings::{PathDefaults, PathSettingsApp};
use whisper_core::{ComputeChoice, ImportApplication};

pub struct DesktopComposition {
    pub application: ImportApplication<AsyncImportIo>,
    pub compute_choice: ComputeChoice,
    pub startup_message: Option<String>,
    pub paths: PathSettingsApp,
    pub gpu_probe: Receiver<Result<(), String>>,
}

pub fn compose() -> DesktopComposition {
    let paths = PathSettingsApp::new(
        PathDefaults {
            audio_directory: whisper_desktop::ui::default_audio_directory(),
            model_path: whisper_desktop::ui::default_model_path(),
            destination: whisper_desktop::ui::default_destination(),
        },
        Box::new(WindowsPathPort::new()),
    );
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
    let (sender, receiver) = mpsc::channel();
    let gpu_worker = workers.gpu.clone();
    let model = paths.model.clone();
    std::thread::spawn(move || {
        let result = probe_cuda_worker(
            &gpu_worker,
            &model,
            whisper_desktop::ui::approved_model_sha256(),
        );
        let _ = sender.send(result);
    });
    DesktopComposition {
        application: ImportApplication::new(AsyncImportIo::start_with_workers(workers)),
        compute_choice: ComputeChoice::Cpu,
        startup_message: Some("Vérification du GPU en cours…".into()),
        paths,
        gpu_probe: receiver,
    }
}
