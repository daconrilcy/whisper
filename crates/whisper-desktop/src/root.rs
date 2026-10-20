use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, TryRecvError},
    time::Duration,
};
use whisper_adapters::model_catalog::CatalogSnapshot;
use whisper_adapters::{
    gpu_inventory::WindowsGpuInventoryPort,
    model_catalog::LocalModelCatalog,
    path_settings::WindowsModelPreferencePort,
    path_settings::WindowsPathPort,
    supervisor::WorkerPaths,
    worker_ipc::{AsyncImportIo, probe_cuda_worker},
};
use whisper_core::gpu_inventory_app::GpuInventoryApp;
use whisper_core::model_selection_app::ModelCommandGuard;
use whisper_core::path_settings::{
    ModelPreferenceSaveStatus, ModelPreferencesApp, StartupComputePreference,
};
use whisper_core::path_settings::{PathDefaults, PathSettingsApp};
use whisper_core::{
    AppCommand, AppView, Application, ApplicationError, ComputeChoice, ImportApplication,
    ImportIoPort,
    model_catalog::{ModelAvailability, ModelId},
    model_registry,
};
use whisper_desktop::ui::{
    ModelPreferenceCommand, ModelPreferenceEvent, ModelSelectionRuntime,
    model_preference_command_channel,
};

pub struct DesktopComposition {
    pub application: DesktopAdmissionApplication<AsyncImportIo>,
    pub compute_choice: ComputeChoice,
    pub startup_message: Option<String>,
    pub paths: PathSettingsApp,
    pub gpu_probe: Receiver<Result<(), String>>,
    pub gpu_inventory: GpuInventoryApp,
    pub model_selection: ModelSelectionRuntime,
}

pub fn compose() -> DesktopComposition {
    let personal_root = personal_install_root();
    let model_root = if cfg!(feature = "personal-install") {
        personal_root
            .as_ref()
            .map(|root| root.join("models"))
            .unwrap_or_else(|| PathBuf::from("\0"))
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap_or_else(|| std::path::Path::new("."))
            .join("models")
    };
    let (catalog_sender, catalog_receiver) = std::sync::mpsc::channel();
    let (admission_sender, admission_receiver) = std::sync::mpsc::channel();
    let catalog_root = model_root.clone();
    std::thread::spawn(move || {
        let mut snapshot = LocalModelCatalog::new(catalog_root).inspect(1);
        qualify_parakeet_catalog_entry(&mut snapshot);
        let mut qualified = parakeet_available(&snapshot);
        let mut gpu_qualified = parakeet_gpu_route_is_qualified(&snapshot);
        let mut gpu_route_metadata = configured_gpu_parakeet_worker()
            .as_ref()
            .and_then(gpu_worker_file_metadata);
        let mut model_metadata = configured_parakeet_worker()
            .as_ref()
            .and_then(|paths| file_metadata(&paths.model));
        let mut generation = snapshot.generation;
        if catalog_sender.send(snapshot.clone()).is_err()
            || admission_sender.send(snapshot.clone()).is_err()
        {
            return;
        }
        loop {
            std::thread::sleep(Duration::from_secs(1));
            let paths = configured_parakeet_worker();
            let next_model_metadata = paths.as_ref().and_then(|paths| file_metadata(&paths.model));
            let artifact_changed = next_model_metadata != model_metadata;
            model_metadata = next_model_metadata;
            let key_is_valid = paths.is_some_and(|paths| {
                let record = model_registry::model(ModelId::ParakeetTdt06BV3);
                let key = model_registry::configuration_key(
                    record,
                    whisper_core::model_catalog::Backend::Cpu,
                );
                if artifact_changed {
                    paths
                        .verify_qualification(
                            record.artifact.sha256,
                            &key.runtime_build,
                            &key.worker_build,
                        )
                        .is_ok()
                } else {
                    parakeet_worker_runtime_is_qualified(&paths)
                }
            });
            let next_qualified = key_is_valid && parakeet_artifact_matches(&snapshot);
            let next_gpu_qualified = parakeet_gpu_route_is_qualified(&snapshot);
            let next_gpu_metadata = configured_gpu_parakeet_worker()
                .as_ref()
                .and_then(gpu_worker_file_metadata);
            let gpu_route_changed = next_gpu_metadata != gpu_route_metadata;
            gpu_route_metadata = next_gpu_metadata;
            let next_gpu_qualified = if gpu_route_changed {
                next_gpu_qualified
            } else {
                gpu_qualified
            };
            if next_qualified != qualified || next_gpu_qualified != gpu_qualified {
                qualified = next_qualified;
                gpu_qualified = next_gpu_qualified;
                generation = generation.saturating_add(1);
                snapshot.generation = generation;
                apply_parakeet_key_availability(&mut snapshot, qualified);
                apply_parakeet_gpu_availability(&mut snapshot, gpu_qualified);
                if catalog_sender.send(snapshot.clone()).is_err()
                    || admission_sender.send(snapshot.clone()).is_err()
                {
                    return;
                }
            }
        }
    });

    let (preference_command_sender, preference_command_receiver) =
        model_preference_command_channel();
    let (preference_event_sender, preference_event_receiver) = std::sync::mpsc::sync_channel(4);
    let personal_install = cfg!(feature = "personal-install");
    let preference_file = personal_root
        .as_ref()
        .map(|root| personal_settings_path(root));
    std::thread::spawn(move || {
        if personal_install && preference_file.is_none() {
            let _ = preference_event_sender.send(ModelPreferenceEvent::Initialized {
                model: None,
                startup_compute: StartupComputePreference::FollowProbe,
                notice: Some("Profil Perso indisponible : LOCALAPPDATA n'est pas défini.".into()),
            });
            return;
        }
        let preference_port: Box<dyn whisper_core::path_settings::ModelPreferencePort> =
            if personal_install {
                Box::new(WindowsModelPreferencePort::new_at(
                    preference_file.expect("personal root checked above"),
                    whisper_desktop::ui::default_model_path(),
                ))
            } else {
                Box::new(WindowsModelPreferencePort::new())
            };
        let preferences = ModelPreferencesApp::new(preference_port);
        let Ok(mut preferences) = preferences else {
            let _ = preference_event_sender.send(ModelPreferenceEvent::Initialized {
                model: None,
                startup_compute: StartupComputePreference::FollowProbe,
                notice: Some("Impossible de lire les préférences de modèle.".into()),
            });
            return;
        };
        let notice = preferences.legacy_path_notice();
        if preference_event_sender
            .send(ModelPreferenceEvent::Initialized {
                model: preferences.preferred_model,
                startup_compute: preferences.startup_compute,
                notice,
            })
            .is_err()
        {
            return;
        }
        while let Ok(commands) = preference_command_receiver.recv() {
            for command in commands {
                match command {
                    ModelPreferenceCommand::ChooseModel(model) => preferences.choose_model(model),
                    ModelPreferenceCommand::ChooseStartupCompute(choice) => {
                        preferences.choose_startup_compute(choice)
                    }
                }
                let (revision, message) = match preferences.save_status() {
                    Some(ModelPreferenceSaveStatus::Saved { revision }) => (
                        *revision,
                        format!("Préférence enregistrée (révision {revision})."),
                    ),
                    Some(ModelPreferenceSaveStatus::Failed { revision, error }) => {
                        (*revision, format!("Préférence non enregistrée : {error}"))
                    }
                    None => (0, "Aucun accusé de sauvegarde reçu.".into()),
                };
                if preference_event_sender
                    .send(ModelPreferenceEvent::Saved { revision, message })
                    .is_err()
                {
                    return;
                }
            }
        }
    });
    let path_port = if cfg!(feature = "personal-install") {
        personal_root.as_ref().map(|root| {
            Box::new(WindowsPathPort::new_at(personal_settings_path(root)))
                as Box<dyn whisper_core::path_settings::PathPort>
        })
    } else {
        Some(Box::new(WindowsPathPort::new()) as Box<dyn whisper_core::path_settings::PathPort>)
    };
    let paths = PathSettingsApp::new(
        PathDefaults {
            audio_directory: whisper_desktop::ui::default_audio_directory(),
            model_path: if cfg!(feature = "personal-install") {
                model_root
                    .join(
                        model_registry::model(ModelId::WhisperLargeV3Turbo)
                            .artifact
                            .file_name,
                    )
                    .to_string_lossy()
                    .into_owned()
            } else {
                whisper_desktop::ui::default_model_path()
            },
            destination: whisper_desktop::ui::default_destination(),
        },
        path_port.unwrap_or_else(|| Box::new(WindowsPathPort::new_at(PathBuf::new()))),
    );
    let executable = std::env::current_exe().ok();
    let worker_path = |worker_name: &str| {
        let sibling = executable
            .as_ref()
            .and_then(|path| path.parent().map(|parent| parent.join(worker_name)));
        if cfg!(feature = "personal-install") {
            return sibling
                .clone()
                .filter(|path| path.is_file())
                .or(sibling)
                .unwrap_or_else(|| PathBuf::from(worker_name));
        }
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
    let inventory_worker = workers.gpu.clone();
    let (sender, receiver) = mpsc::channel();
    let gpu_worker = workers.gpu.clone();
    std::thread::spawn(move || {
        let result = probe_cuda_worker(&gpu_worker);
        let _ = sender.send(result);
    });
    let historical_model_path = whisper_desktop::ui::default_model_path();
    let io = AsyncImportIo::start_with_model_catalog_and_parakeet_routes_and_snapshot_registry(
        workers,
        model_root.clone(),
        historical_model_path.clone(),
        configured_parakeet_worker(),
        configured_gpu_parakeet_worker(),
        reprocessing_snapshot_registry_root(),
    );
    let application =
        guarded_application(io, &model_root, historical_model_path, admission_receiver);
    DesktopComposition {
        application,
        compute_choice: ComputeChoice::Cpu,
        startup_message: if cfg!(feature = "personal-install") && personal_root.is_none() {
            Some("Profil Perso indisponible : LOCALAPPDATA n'est pas défini.".into())
        } else {
            Some("Vérification du GPU en cours…".into())
        },
        paths,
        gpu_probe: receiver,
        gpu_inventory: GpuInventoryApp::new(Box::new(WindowsGpuInventoryPort::new(
            inventory_worker,
        ))),
        model_selection: ModelSelectionRuntime {
            catalog: catalog_receiver,
            preference_events: preference_event_receiver,
            preference_commands: preference_command_sender,
        },
    }
}

fn configured_parakeet_worker() -> Option<whisper_adapters::parakeet_import::ParakeetWorkerPaths> {
    if cfg!(feature = "personal-install") {
        let root = personal_install_root()?;
        return Some(personal_worker_paths(
            &root,
            whisper_core::model_catalog::Backend::Cpu,
        ));
    }
    let runtime_root = PathBuf::from(std::env::var_os("PARAKEET_L2_RUNTIME_ROOT")?);
    Some(whisper_adapters::parakeet_import::ParakeetWorkerPaths {
        executable: std::env::var_os("PARAKEET_L2_CPU_WORKER")
            .map(PathBuf::from)
            .unwrap_or_else(|| runtime_root.join("worker/parakeet-worker.exe")),
        native_dir: std::env::var_os("PARAKEET_L2_CPU_NATIVE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| runtime_root.join("native")),
        model: std::env::var_os("PARAKEET_L2_MODEL")
            .map(PathBuf::from)
            .unwrap_or_else(|| runtime_root.join("runtime/model/parakeet-tdt-0.6b-v3.q8_0.gguf")),
        runtime_root: runtime_root.clone(),
    })
}

fn configured_gpu_parakeet_worker() -> Option<whisper_adapters::parakeet_import::ParakeetWorkerPaths>
{
    if cfg!(feature = "personal-install") {
        let root = personal_install_root()?;
        return Some(personal_worker_paths(
            &root,
            whisper_core::model_catalog::Backend::Gpu,
        ));
    }
    let runtime_root = PathBuf::from(std::env::var_os("PARAKEET_L2_RUNTIME_ROOT")?);
    Some(whisper_adapters::parakeet_import::ParakeetWorkerPaths {
        executable: PathBuf::from(std::env::var_os("PARAKEET_L2_WORKER")?),
        native_dir: PathBuf::from(std::env::var_os("PARAKEET_L2_NATIVE_DIR")?),
        model: PathBuf::from(std::env::var_os("PARAKEET_L2_MODEL")?),
        runtime_root,
    })
}

fn personal_install_root() -> Option<PathBuf> {
    if !cfg!(feature = "personal-install") {
        return None;
    }
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .map(|root| root.join("Whisper"))
}

fn personal_model_root(root: &std::path::Path) -> PathBuf {
    root.join("models")
}

fn personal_settings_path(root: &std::path::Path) -> PathBuf {
    root.join("settings/settings.json")
}

fn reprocessing_snapshot_registry_root() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|root| {
        PathBuf::from(root)
            .join("Whisper")
            .join("settings/reprocessing-snapshots")
    })
}

fn personal_worker_paths(
    root: &std::path::Path,
    backend: whisper_core::model_catalog::Backend,
) -> whisper_adapters::parakeet_import::ParakeetWorkerPaths {
    let (worker, native) = match backend {
        whisper_core::model_catalog::Backend::Cpu => {
            ("whisper-worker-parakeet-cpu.exe", "native/cpu")
        }
        whisper_core::model_catalog::Backend::Gpu => {
            ("whisper-worker-parakeet-gpu.exe", "native/gpu")
        }
    };
    whisper_adapters::parakeet_import::ParakeetWorkerPaths {
        executable: root.join("worker").join(worker),
        native_dir: root.join(native),
        model: personal_model_root(root).join("parakeet-tdt-0.6b-v3.q8_0.gguf"),
        runtime_root: root.to_path_buf(),
    }
}

fn qualify_parakeet_catalog_entry(snapshot: &mut CatalogSnapshot) {
    use whisper_core::model_catalog::{Backend, ModelId};
    let record = model_registry::model(ModelId::ParakeetTdt06BV3);
    let key = if cfg!(feature = "personal-install") {
        model_registry::personal_configuration_key(record, Backend::Cpu)
    } else {
        model_registry::configuration_key(record, Backend::Cpu)
    };
    let catalog_model_ready = snapshot.entries.iter().any(|entry| {
        entry.id == ModelId::ParakeetTdt06BV3
            && entry.availability == ModelAvailability::Ready
            && entry.observed_sha256.as_deref() == Some(record.artifact.sha256)
            && entry.observed_size_bytes == Some(record.artifact.size_bytes)
    });
    let qualified = catalog_model_ready
        && configured_parakeet_worker().is_some_and(|paths| {
            paths
                .verify_qualification(
                    record.artifact.sha256,
                    &key.runtime_build,
                    &key.worker_build,
                )
                .is_ok()
        });
    apply_parakeet_key_availability(snapshot, qualified);
    apply_parakeet_gpu_availability(snapshot, parakeet_gpu_route_is_qualified(snapshot));
}

fn parakeet_gpu_route_is_qualified(snapshot: &CatalogSnapshot) -> bool {
    if !parakeet_artifact_matches(snapshot) {
        return false;
    }
    configured_gpu_parakeet_worker().is_some_and(|paths| {
        paths
            .gpu_qualification_key(whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS)
            .is_ok_and(|actual| {
                if cfg!(feature = "personal-install") {
                    actual.is_registered_personal_import()
                } else {
                    actual
                        == whisper_adapters::archive::ArchiveQualificationKey::parakeet_gpu_test()
                }
            })
    })
}

fn gpu_worker_file_metadata(
    paths: &whisper_adapters::parakeet_import::ParakeetWorkerPaths,
) -> Option<Vec<whisper_adapters::model_catalog::ArtifactFileIdentity>> {
    const GPU_DLLS: [&str; 9] = [
        "cublas64_12.dll",
        "cublasLt64_12.dll",
        "cudart64_12.dll",
        "nemo_speech_asr_c.dll",
        "nemo_speech_asr.dll",
        "ggml.dll",
        "ggml-base.dll",
        "ggml-cpu.dll",
        "ggml-cuda.dll",
    ];
    let mut metadata = vec![file_metadata(&paths.executable)?];
    metadata.extend(
        GPU_DLLS
            .into_iter()
            .map(|name| file_metadata(&paths.native_dir.join(name)))
            .collect::<Option<Vec<_>>>()?,
    );
    Some(metadata)
}

fn parakeet_worker_runtime_is_qualified(
    paths: &whisper_adapters::parakeet_import::ParakeetWorkerPaths,
) -> bool {
    let record = model_registry::model(ModelId::ParakeetTdt06BV3);
    let key = if cfg!(feature = "personal-install") {
        model_registry::personal_configuration_key(
            record,
            whisper_core::model_catalog::Backend::Cpu,
        )
    } else {
        model_registry::configuration_key(record, whisper_core::model_catalog::Backend::Cpu)
    };
    paths
        .verify_worker_runtime(&key.runtime_build, &key.worker_build)
        .is_ok()
}

fn file_metadata(
    path: &std::path::Path,
) -> Option<whisper_adapters::model_catalog::ArtifactFileIdentity> {
    let identity = whisper_adapters::model_catalog::artifact_file_identity(path).ok()?;
    Some(identity)
}

fn parakeet_artifact_matches(snapshot: &CatalogSnapshot) -> bool {
    let record = model_registry::model(ModelId::ParakeetTdt06BV3);
    snapshot.entries.iter().any(|entry| {
        entry.id == ModelId::ParakeetTdt06BV3
            && entry.observed_sha256.as_deref() == Some(record.artifact.sha256)
            && entry.observed_size_bytes == Some(record.artifact.size_bytes)
    })
}

fn parakeet_available(snapshot: &CatalogSnapshot) -> bool {
    snapshot.entries.iter().any(|entry| {
        entry.id == ModelId::ParakeetTdt06BV3 && entry.availability == ModelAvailability::Ready
    })
}

fn apply_parakeet_key_availability(snapshot: &mut CatalogSnapshot, key_verified: bool) {
    use whisper_core::model_catalog::ModelId;
    let catalog_model_ready = parakeet_artifact_matches(snapshot);
    if !(key_verified && catalog_model_ready)
        && let Some(entry) = snapshot
            .entries
            .iter_mut()
            .find(|entry| entry.id == ModelId::ParakeetTdt06BV3)
    {
        entry.availability = ModelAvailability::Missing;
        entry.diagnostic = Some(
            "Worker, DLL ou modèle Parakeet absent ou différent de la fiche Test qualifiée.".into(),
        );
    } else if key_verified
        && catalog_model_ready
        && let Some(entry) = snapshot
            .entries
            .iter_mut()
            .find(|entry| entry.id == ModelId::ParakeetTdt06BV3)
    {
        entry.availability = ModelAvailability::Ready;
        entry.diagnostic = None;
    }
}

fn apply_parakeet_gpu_availability(snapshot: &mut CatalogSnapshot, gpu_verified: bool) {
    if let Some(entry) = snapshot.entries.iter_mut().find(|entry| {
        entry.id == ModelId::ParakeetTdt06BV3 && entry.availability == ModelAvailability::Ready
    }) {
        if gpu_verified {
            if entry
                .diagnostic
                .as_deref()
                .is_some_and(|text| text.starts_with("GPU indisponible :"))
            {
                entry.diagnostic = None;
            }
        } else {
            entry.diagnostic = Some(
                "GPU indisponible : worker ou DLL absent, divergent de la fiche Test qualifiée."
                    .into(),
            );
        }
    }
}

pub struct DesktopAdmissionApplication<P> {
    inner: ImportApplication<P>,
    admission_catalog: Receiver<CatalogSnapshot>,
    admission_guard: Option<ModelCommandGuard>,
    admission_catalog_failed: bool,
    historical_model_path: String,
    admission_notice: Option<String>,
    pending_gpu_candidate: Option<(
        whisper_core::JobId,
        whisper_core::Generation,
        Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
    )>,
}

fn guarded_application<P: ImportIoPort>(
    io: P,
    model_root: &std::path::Path,
    historical_model_path: String,
    admission_catalog: Receiver<CatalogSnapshot>,
) -> DesktopAdmissionApplication<P> {
    let app_model_path = model_root
        .join(
            model_registry::model(whisper_core::model_catalog::ModelId::WhisperLargeV3Turbo)
                .artifact
                .file_name,
        )
        .to_string_lossy()
        .into_owned();
    let accepted_paths = [
        (ModelId::WhisperLargeV3Turbo, app_model_path),
        (ModelId::WhisperLargeV3Turbo, historical_model_path.clone()),
        (
            ModelId::ParakeetTdt06BV3,
            model_root
                .join(
                    model_registry::model(ModelId::ParakeetTdt06BV3)
                        .artifact
                        .file_name,
                )
                .to_string_lossy()
                .into_owned(),
        ),
    ];
    let core_guard = ModelCommandGuard::new_with_model_paths(
        model_registry::engine_snapshot(1, |_| ModelAvailability::Ready, Vec::new()),
        1,
        accepted_paths,
    );
    DesktopAdmissionApplication {
        inner: ImportApplication::new(io).with_model_guard(core_guard),
        admission_catalog,
        admission_guard: None,
        admission_catalog_failed: false,
        historical_model_path,
        admission_notice: None,
        pending_gpu_candidate: None,
    }
}

impl<P: ImportIoPort> DesktopAdmissionApplication<P> {
    fn poll_admission_catalog(&mut self) {
        loop {
            match self.admission_catalog.try_recv() {
                Ok(snapshot) => {
                    let paths = snapshot
                        .entries
                        .iter()
                        .map(|entry| (entry.id, entry.path.to_string_lossy().into_owned()))
                        .chain(std::iter::once((
                            ModelId::WhisperLargeV3Turbo,
                            self.historical_model_path.clone(),
                        )));
                    let guard = ModelCommandGuard::new_with_model_paths(
                        snapshot.engine_snapshot(Vec::new()),
                        snapshot.generation,
                        paths,
                    );
                    self.admission_guard = Some(guard);
                    self.admission_catalog_failed = false;
                }
                Err(TryRecvError::Disconnected) => {
                    self.admission_catalog_failed = true;
                    break;
                }
                Err(TryRecvError::Empty) => break,
            }
        }
    }

    fn admission_result(&self, command: &AppCommand) -> Option<Result<(), &'static str>> {
        let import = match command {
            AppCommand::StartImport { request }
            | AppCommand::EnqueueImport { request }
            | AppCommand::ProcessQueued { request }
            | AppCommand::ResumeInterrupted { request } => Some(request),
            _ => None,
        };
        let live = match command {
            AppCommand::StartLiveConfigured { request }
            | AppCommand::StartDirectLiveConfigured { request } => Some(request),
            _ => None,
        };
        if import.is_none() && live.is_none() {
            return None;
        }
        let Some(guard) = self.admission_guard.as_ref() else {
            return Some(Err(if self.admission_catalog_failed {
                "catalogue de modèles indisponible"
            } else {
                "catalogue de modèles en cours de vérification"
            }));
        };
        let result = if let Some(request) = import {
            guard.admit_import(
                request,
                whisper_core::model_selection_app::InferencePath::Import,
            )
        } else {
            guard.admit_live(live.expect("live request matched above"))
        };
        Some(result.map(|_| ()).map_err(|error| match error {
            whisper_core::model_selection_app::SelectionRefusal::UnknownHistoricalModelPath => {
                "chemin ou empreinte non reconnu dans le catalogue"
            }
            whisper_core::model_selection_app::SelectionRefusal::Capability(_) => {
                "combinaison modèle, calcul ou langue non admissible"
            }
        }))
    }

    fn core_refuses_before_effect(&self, command: &AppCommand) -> bool {
        let view = self.inner.view();
        let active = view.active_job.is_some_and(|(_, state)| {
            !matches!(
                state,
                whisper_core::JobState::Complete
                    | whisper_core::JobState::Recoverable
                    | whisper_core::JobState::Cancelled
            )
        });
        match command {
            AppCommand::StartImport { .. }
            | AppCommand::ProcessQueued { .. }
            | AppCommand::StartLiveConfigured { .. }
            | AppCommand::StartDirectLiveConfigured { .. } => active,
            AppCommand::ResumeInterrupted { request } => {
                active || request.generation.next().is_none()
            }
            AppCommand::EnqueueImport { .. } => active && view.direct_capture_phase.is_some(),
            _ => false,
        }
    }
}

impl<P: ImportIoPort> Application for DesktopAdmissionApplication<P> {
    fn dispatch(&mut self, command: AppCommand) -> Result<AppView, ApplicationError> {
        self.poll_admission_catalog();
        if let Some(Err(reason)) = self.admission_result(&command) {
            let notice = format!("Modèle indisponible : {reason}");
            self.admission_notice = Some(notice.clone());
            self.pending_gpu_candidate = None;
            let mut view = self.inner.view();
            view.message = Some(notice);
            return Ok(view);
        }
        let is_refresh = matches!(&command, AppCommand::Refresh);
        let is_model_start = matches!(
            &command,
            AppCommand::StartImport { .. }
                | AppCommand::EnqueueImport { .. }
                | AppCommand::ProcessQueued { .. }
                | AppCommand::ResumeInterrupted { .. }
                | AppCommand::StartLiveConfigured { .. }
                | AppCommand::StartDirectLiveConfigured { .. }
        );
        if is_model_start
            && let Some((job_id, generation, candidate)) = self.pending_gpu_candidate.take()
            && command.targets(job_id, generation)
        {
            if !self.core_refuses_before_effect(&command) {
                self.inner
                    .set_gpu_candidate(job_id, generation, candidate)?;
            }
        } else if !is_model_start {
            self.pending_gpu_candidate = None;
        }
        let mut view = self.inner.dispatch(command)?;
        if is_refresh {
            if let Some(notice) = &self.admission_notice {
                view.message = Some(notice.clone());
            }
        } else {
            self.admission_notice = None;
        }
        Ok(view)
    }

    fn view(&self) -> AppView {
        self.inner.view()
    }

    fn set_gpu_candidate(
        &mut self,
        job_id: whisper_core::JobId,
        generation: whisper_core::Generation,
        candidate: Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
    ) -> Result<(), ApplicationError> {
        self.pending_gpu_candidate = Some((job_id, generation, candidate));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};
    use whisper_core::ports::{ImportEvent, PortError};
    use whisper_core::{
        AppCommand, Application, ComputeChoice, Generation, ImportEffect, ImportRequest, JobConfig,
        JobId, LanguageChoice,
    };

    #[derive(Default)]
    struct RecordingIo {
        effects: std::sync::Arc<std::sync::Mutex<Vec<ImportEffect>>>,
        events: std::sync::Arc<std::sync::Mutex<std::collections::VecDeque<ImportEvent>>>,
    }

    impl ImportIoPort for RecordingIo {
        fn submit(&mut self, effect: ImportEffect) -> Result<(), PortError> {
            self.effects.lock().unwrap().push(effect);
            Ok(())
        }

        fn poll(&mut self) -> Result<Option<ImportEvent>, PortError> {
            Ok(self.events.lock().unwrap().pop_front())
        }
    }

    fn fixture_root(name: &str) -> PathBuf {
        let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap()
            .to_path_buf();
        let base = if let Some(configured) = std::env::var_os("WHISPER_DESKTOP_TEST_ROOT") {
            let configured = PathBuf::from(configured)
                .canonicalize()
                .expect("WHISPER_DESKTOP_TEST_ROOT must exist and resolve canonically");
            let repository = repo.canonicalize().expect("canonical repository root");
            assert!(
                is_allowed_campaign_root(&configured, &repository),
                "WHISPER_DESKTOP_TEST_ROOT must be inside canonical L3b, L3c, or L3d"
            );
            configured
        } else {
            std::env::temp_dir().join("whisper-desktop-tests")
        };
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        base.join(format!("{name}-{}-{suffix}", std::process::id()))
    }

    fn is_allowed_campaign_root(configured: &Path, repository: &Path) -> bool {
        let l6_perso = repository
            .join("target/parakeet-tests/l6-perso")
            .canonicalize()
            .is_ok_and(|expected| configured == &expected || configured.starts_with(expected));
        l6_perso
            || ["l3b", "l3c", "l3d"].into_iter().any(|lot| {
                repository
                    .join(format!("target/parakeet-tests/l3/{lot}"))
                    .canonicalize()
                    .is_ok_and(|expected| {
                        configured == &expected || configured.starts_with(expected)
                    })
            })
    }

    #[test]
    fn fixture_root_uses_canonical_l3b_campaign_root_when_configured() {
        let Some(configured) = std::env::var_os("WHISPER_DESKTOP_TEST_ROOT") else {
            eprintln!("NOT RUN: WHISPER_DESKTOP_TEST_ROOT is not set");
            return;
        };
        let configured = PathBuf::from(configured).canonicalize().unwrap();
        let fixture = fixture_root("redirect-check");
        assert!(
            fixture.starts_with(&configured),
            "desktop fixture escaped L3b: {fixture:?}"
        );
    }

    #[test]
    fn fixture_root_accepts_only_the_canonical_l3c_campaign_root() {
        let Some(configured) = std::env::var_os("WHISPER_DESKTOP_TEST_ROOT") else {
            eprintln!("NOT RUN: WHISPER_DESKTOP_TEST_ROOT is not set");
            return;
        };
        let configured = PathBuf::from(configured).canonicalize().unwrap();
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let repo = manifest_dir.ancestors().nth(2).unwrap();
        let l3c = repo
            .join("target/parakeet-tests/l3/l3c")
            .canonicalize()
            .unwrap();
        let l3d = repo
            .join("target/parakeet-tests/l3/l3d")
            .canonicalize()
            .unwrap();
        assert!(is_allowed_campaign_root(&l3c, repo));
        assert!(is_allowed_campaign_root(&l3d, repo));
        assert!(!is_allowed_campaign_root(
            &repo.join("target/parakeet-tests/l3/l3d-neighbor"),
            repo
        ));
        let fixture = fixture_root("l3c-redirect-check");
        assert!(fixture.starts_with(&configured));
    }

    #[test]
    fn artifact_metadata_detects_replacement_with_same_size_and_modified_time() {
        use std::fs::{self, File, FileTimes};
        use std::io::Write;

        let root = fixture_root("artifact-identity");
        fs::create_dir_all(&root).unwrap();
        let original = root.join("worker.dll");
        fs::write(&original, b"alpha").unwrap();
        let before = file_metadata(&original).unwrap();
        let metadata = fs::metadata(&original).unwrap();
        let replacement = root.join("replacement.dll");
        let mut file = File::create(&replacement).unwrap();
        file.write_all(b"bravo").unwrap();
        file.set_times(FileTimes::new().set_modified(metadata.modified().unwrap()))
            .unwrap();
        drop(file);
        fs::rename(&original, root.join("parked.dll")).unwrap();
        fs::rename(&replacement, &original).unwrap();

        let after = file_metadata(&original).unwrap();
        let after_metadata = fs::metadata(&original).unwrap();
        assert_eq!(after_metadata.len(), metadata.len());
        assert_eq!(
            after_metadata.modified().unwrap(),
            metadata.modified().unwrap()
        );
        assert_ne!(
            before, after,
            "the file identifier must distinguish replacement"
        );
        fs::remove_dir_all(root).unwrap();
    }

    fn turbo_sha() -> [u8; 32] {
        let value =
            model_registry::model(whisper_core::model_catalog::ModelId::WhisperLargeV3Turbo)
                .artifact
                .sha256;
        let mut result = [0_u8; 32];
        for (index, byte) in result.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).unwrap();
        }
        result
    }

    fn model_sha(id: ModelId) -> [u8; 32] {
        let value = model_registry::model(id).artifact.sha256;
        let mut result = [0_u8; 32];
        for (index, byte) in result.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).unwrap();
        }
        result
    }

    fn request(model_path: String, model_sha256: [u8; 32], destination: String) -> ImportRequest {
        ImportRequest {
            job_id: JobId(901),
            generation: Generation::first(),
            source_path: "unused.wav".into(),
            source_sha256: None,
            source_samples: None,
            model_path,
            model_sha256,
            destination,
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Cpu,
            },
        }
    }

    fn admission_snapshot(
        model_root: &std::path::Path,
        turbo_availability: ModelAvailability,
    ) -> CatalogSnapshot {
        CatalogSnapshot {
            generation: 1,
            entries: model_registry::MODELS
                .iter()
                .map(|record| {
                    let availability = if record.artifact.id == ModelId::WhisperLargeV3Turbo {
                        turbo_availability
                    } else {
                        ModelAvailability::Missing
                    };
                    whisper_adapters::model_catalog::CatalogEntry {
                        id: record.artifact.id,
                        display_name: record.display_name.to_owned(),
                        path: model_root.join(record.artifact.file_name),
                        availability,
                        expected_size_bytes: record.artifact.size_bytes,
                        observed_size_bytes: (availability == ModelAvailability::Ready)
                            .then_some(record.artifact.size_bytes),
                        expected_sha256: record.artifact.sha256.to_owned(),
                        observed_sha256: (availability == ModelAvailability::Ready)
                            .then(|| record.artifact.sha256.to_owned()),
                        diagnostic: (availability != ModelAvailability::Ready)
                            .then(|| "Fichier absent ou invalide.".into()),
                    }
                })
                .collect(),
        }
    }

    fn admission_receiver(snapshot: CatalogSnapshot) -> Receiver<CatalogSnapshot> {
        let (sender, receiver) = mpsc::sync_channel(1);
        sender.send(snapshot).unwrap();
        receiver
    }

    #[test]
    fn parakeet_selection_and_admission_require_the_exact_test_key() {
        let model_root = fixture_root("parakeet-key-gate");
        let mut catalog = admission_snapshot(&model_root, ModelAvailability::Ready);
        let record = model_registry::model(ModelId::ParakeetTdt06BV3);
        let parakeet = catalog
            .entries
            .iter_mut()
            .find(|entry| entry.id == ModelId::ParakeetTdt06BV3)
            .unwrap();
        parakeet.availability = ModelAvailability::Ready;
        parakeet.observed_sha256 = Some(record.artifact.sha256.to_owned());
        parakeet.observed_size_bytes = Some(record.artifact.size_bytes);
        parakeet.diagnostic = None;

        let parakeet_request = || {
            request(
                model_root
                    .join(record.artifact.file_name)
                    .to_string_lossy()
                    .into_owned(),
                model_sha(ModelId::ParakeetTdt06BV3),
                "archive".into(),
            )
        };
        let paths = |snapshot: &CatalogSnapshot| {
            snapshot
                .entries
                .iter()
                .map(|entry| (entry.id, entry.path.to_string_lossy().into_owned()))
                .collect::<Vec<_>>()
        };

        let mut unavailable = catalog.clone();
        apply_parakeet_key_availability(&mut unavailable, false);
        let unavailable_projection = whisper_core::model_selection_app::ModelSelectionApp::new(
            unavailable.engine_snapshot(Vec::new()),
        )
        .projection(1)
        .unwrap();
        assert!(
            !unavailable_projection
                .models
                .iter()
                .find(|model| model.id == ModelId::ParakeetTdt06BV3)
                .unwrap()
                .selectable
        );
        let unavailable_guard = ModelCommandGuard::new_with_model_paths(
            unavailable.engine_snapshot(Vec::new()),
            1,
            paths(&unavailable),
        );
        assert!(
            unavailable_guard
                .admit_import(
                    &parakeet_request(),
                    whisper_core::model_selection_app::InferencePath::Import,
                )
                .is_err()
        );

        let mut qualified = catalog;
        apply_parakeet_key_availability(&mut qualified, true);
        let qualified_projection = whisper_core::model_selection_app::ModelSelectionApp::new(
            qualified.engine_snapshot(Vec::new()),
        )
        .projection(1)
        .unwrap();
        assert!(
            qualified_projection
                .models
                .iter()
                .find(|model| model.id == ModelId::ParakeetTdt06BV3)
                .unwrap()
                .selectable
        );
        let qualified_guard = ModelCommandGuard::new_with_model_paths(
            qualified.engine_snapshot(Vec::new()),
            1,
            paths(&qualified),
        );
        assert!(
            qualified_guard
                .admit_import(
                    &parakeet_request(),
                    whisper_core::model_selection_app::InferencePath::Import,
                )
                .is_ok()
        );
        assert!(
            qualified
                .entries
                .iter()
                .find(|entry| entry.id == ModelId::WhisperLargeV3Turbo)
                .unwrap()
                .availability
                == ModelAvailability::Ready
        );
    }

    #[test]
    fn valid_cpu_route_remains_ready_when_gpu_route_is_missing_or_mismatched() {
        let model_root = fixture_root("parakeet-gpu-route-availability");
        let mut snapshot = admission_snapshot(&model_root, ModelAvailability::Ready);
        let entry = snapshot
            .entries
            .iter_mut()
            .find(|entry| entry.id == ModelId::ParakeetTdt06BV3)
            .unwrap();
        let record = model_registry::model(ModelId::ParakeetTdt06BV3);
        entry.availability = ModelAvailability::Ready;
        entry.observed_sha256 = Some(record.artifact.sha256.to_owned());
        entry.observed_size_bytes = Some(record.artifact.size_bytes);

        apply_parakeet_key_availability(&mut snapshot, true);
        apply_parakeet_gpu_availability(&mut snapshot, false);
        let entry = snapshot
            .entries
            .iter()
            .find(|entry| entry.id == ModelId::ParakeetTdt06BV3)
            .unwrap();
        assert_eq!(entry.availability, ModelAvailability::Ready);
        assert!(
            entry
                .diagnostic
                .as_deref()
                .is_some_and(|message| message.starts_with("GPU indisponible :"))
        );

        apply_parakeet_gpu_availability(&mut snapshot, true);
        let entry = snapshot
            .entries
            .iter()
            .find(|entry| entry.id == ModelId::ParakeetTdt06BV3)
            .unwrap();
        assert_eq!(entry.availability, ModelAvailability::Ready);
        assert_eq!(entry.diagnostic, None);
    }

    #[test]
    #[ignore = "mutates a worker copy inside the scoped L2a-2 runtime"]
    fn parakeet_worker_key_changes_regray_and_readmit_without_restarting() {
        use std::io::Write;
        use whisper_adapters::parakeet_import::ParakeetWorkerPaths;

        let runtime_root = PathBuf::from(
            std::env::var_os("PARAKEET_L2_RUNTIME_ROOT").expect("L2a-2 runtime root is required"),
        );
        let original_paths = configured_parakeet_worker().expect("qualified worker paths");
        let mutated_worker = runtime_root.join(format!(
            "worker/a8-runtime-change-{}.exe",
            std::process::id()
        ));
        std::fs::copy(&original_paths.executable, &mutated_worker).unwrap();
        let paths = ParakeetWorkerPaths {
            executable: mutated_worker.clone(),
            ..original_paths.clone()
        };
        let record = model_registry::model(ModelId::ParakeetTdt06BV3);
        let key =
            model_registry::configuration_key(record, whisper_core::model_catalog::Backend::Cpu);
        paths
            .verify_qualification(
                record.artifact.sha256,
                &key.runtime_build,
                &key.worker_build,
            )
            .expect("copied worker initially has the exact Test key");

        let model_root = paths.model.parent().unwrap().to_path_buf();
        let historical = "C:/known/turbo.bin".to_owned();
        let mut snapshot = admission_snapshot(&model_root, ModelAvailability::Ready);
        let parakeet = snapshot
            .entries
            .iter_mut()
            .find(|entry| entry.id == ModelId::ParakeetTdt06BV3)
            .unwrap();
        parakeet.availability = ModelAvailability::Ready;
        parakeet.observed_sha256 = Some(record.artifact.sha256.to_owned());
        parakeet.observed_size_bytes = Some(record.artifact.size_bytes);
        parakeet.diagnostic = None;

        let (sender, receiver) = mpsc::channel();
        let effects = RecordingIo::default().effects.clone();
        let mut app = guarded_application(
            RecordingIo {
                effects: effects.clone(),
                ..RecordingIo::default()
            },
            &model_root,
            historical,
            receiver,
        );
        let parakeet_request = || {
            request(
                model_root
                    .join(record.artifact.file_name)
                    .to_string_lossy()
                    .into_owned(),
                model_sha(ModelId::ParakeetTdt06BV3),
                "archive".into(),
            )
        };
        let is_selectable = |snapshot: &CatalogSnapshot| {
            whisper_core::model_selection_app::ModelSelectionApp::new(
                snapshot.engine_snapshot(Vec::new()),
            )
            .projection(snapshot.generation)
            .unwrap()
            .models
            .iter()
            .find(|model| model.id == ModelId::ParakeetTdt06BV3)
            .unwrap()
            .selectable
        };
        let start = || AppCommand::StartImport {
            request: parakeet_request(),
        };

        assert!(parakeet_worker_runtime_is_qualified(&paths));
        apply_parakeet_key_availability(&mut snapshot, true);
        sender.send(snapshot.clone()).unwrap();
        app.poll_admission_catalog();
        assert!(is_selectable(&snapshot));
        assert!(app.admission_result(&start()).unwrap().is_ok());

        std::fs::OpenOptions::new()
            .append(true)
            .open(&mutated_worker)
            .unwrap()
            .write_all(b"changed worker build")
            .unwrap();
        assert!(!parakeet_worker_runtime_is_qualified(&paths));
        snapshot.generation += 1;
        apply_parakeet_key_availability(&mut snapshot, false);
        sender.send(snapshot.clone()).unwrap();
        app.poll_admission_catalog();
        assert!(!is_selectable(&snapshot));
        assert!(app.admission_result(&start()).unwrap().is_err());
        let refused = app.dispatch(start()).unwrap();
        assert!(
            refused
                .message
                .as_deref()
                .is_some_and(|message| message.contains("Modèle indisponible"))
        );
        assert!(effects.lock().unwrap().is_empty());

        std::fs::copy(&original_paths.executable, &mutated_worker).unwrap();
        assert!(parakeet_worker_runtime_is_qualified(&paths));
        snapshot.generation += 1;
        apply_parakeet_key_availability(&mut snapshot, true);
        sender.send(snapshot.clone()).unwrap();
        app.poll_admission_catalog();
        assert!(is_selectable(&snapshot));
        assert!(app.admission_result(&start()).unwrap().is_ok());
        assert_eq!(
            snapshot
                .entries
                .iter()
                .find(|entry| entry.id == ModelId::WhisperLargeV3Turbo)
                .unwrap()
                .availability,
            ModelAvailability::Ready
        );
    }

    #[test]
    fn composed_guard_rejects_direct_unknown_model_before_any_effect() {
        let model_root = fixture_root("admission");
        std::fs::create_dir_all(&model_root).unwrap();
        let io = RecordingIo::default();
        let effects = io.effects.clone();
        let snapshot = admission_snapshot(&model_root, ModelAvailability::Ready);
        let mut app = guarded_application(
            io,
            &model_root,
            "C:/known/turbo.bin".into(),
            admission_receiver(snapshot),
        );
        let view = app
            .dispatch(AppCommand::StartImport {
                request: request("C:/unknown/model.bin".into(), turbo_sha(), "archive".into()),
            })
            .unwrap();
        assert!(
            effects.lock().unwrap().is_empty(),
            "an inadmissible direct command must have no effects"
        );
        assert!(
            view.message
                .as_deref()
                .is_some_and(|message| message.contains("Modèle indisponible"))
        );
    }

    #[test]
    fn observed_missing_model_is_refused_before_any_effect() {
        let model_root = fixture_root("missing-admission");
        std::fs::create_dir_all(&model_root).unwrap();
        let io = RecordingIo::default();
        let effects = io.effects.clone();
        let model_path = model_root
            .join(
                model_registry::model(ModelId::WhisperLargeV3Turbo)
                    .artifact
                    .file_name,
            )
            .to_string_lossy()
            .into_owned();
        let snapshot = admission_snapshot(&model_root, ModelAvailability::Missing);
        let (admission_sender, admission_receiver) = mpsc::sync_channel(1);
        let mut app = guarded_application(
            io,
            &model_root,
            "C:/known/turbo.bin".into(),
            admission_receiver,
        );
        let command = AppCommand::StartImport {
            request: request(model_path, turbo_sha(), "archive".into()),
        };
        let loading = app.dispatch(command.clone()).unwrap();
        assert!(
            loading
                .message
                .as_deref()
                .is_some_and(|message| { message.contains("en cours de vérification") })
        );
        admission_sender.send(snapshot).unwrap();
        let view = app.dispatch(command).unwrap();
        assert!(effects.lock().unwrap().is_empty());
        assert!(
            view.message
                .as_deref()
                .is_some_and(|message| message.contains("Modèle indisponible"))
        );
    }

    #[test]
    fn refused_ui_request_drops_staged_gpu_candidate_and_refresh_keeps_notice() {
        let model_root = fixture_root("refused-ui-request");
        std::fs::create_dir_all(&model_root).unwrap();
        let io = RecordingIo::default();
        let effects = io.effects.clone();
        let snapshot = admission_snapshot(&model_root, ModelAvailability::Missing);
        let model_path = model_root
            .join(
                model_registry::model(ModelId::WhisperLargeV3Turbo)
                    .artifact
                    .file_name,
            )
            .to_string_lossy()
            .into_owned();
        let mut app = guarded_application(
            io,
            &model_root,
            "C:/known/turbo.bin".into(),
            admission_receiver(snapshot),
        );
        let request = request(model_path, turbo_sha(), "archive".into());
        app.set_gpu_candidate(request.job_id, request.generation, None)
            .unwrap();
        let refused = app.dispatch(AppCommand::EnqueueImport { request }).unwrap();
        assert!(
            refused
                .message
                .as_deref()
                .is_some_and(|message| message.contains("Modèle indisponible"))
        );
        assert!(
            effects.lock().unwrap().is_empty(),
            "refused UI command must not submit SetGpuCandidate or any other IO effect"
        );

        let refreshed = app.dispatch(AppCommand::Refresh).unwrap();
        assert!(
            refreshed
                .message
                .as_deref()
                .is_some_and(|message| message.contains("Modèle indisponible"))
        );
        assert!(
            effects.lock().unwrap().is_empty(),
            "refresh must not submit the staged GPU candidate"
        );
    }

    #[test]
    fn active_job_refuses_interrupted_resume_without_gpu_candidate_effect() {
        let model_root = fixture_root("active-resume-admission");
        std::fs::create_dir_all(&model_root).unwrap();
        let io = RecordingIo::default();
        let effects = io.effects.clone();
        let snapshot = admission_snapshot(&model_root, ModelAvailability::Ready);
        let model_path = model_root
            .join(
                model_registry::model(ModelId::WhisperLargeV3Turbo)
                    .artifact
                    .file_name,
            )
            .to_string_lossy()
            .into_owned();
        let mut app = guarded_application(
            io,
            &model_root,
            "C:/known/turbo.bin".into(),
            admission_receiver(snapshot),
        );
        let request = request(model_path, turbo_sha(), "archive".into());
        app.dispatch(AppCommand::StartImport {
            request: request.clone(),
        })
        .unwrap();
        app.set_gpu_candidate(request.job_id, request.generation, None)
            .unwrap();

        assert_eq!(
            app.dispatch(AppCommand::ResumeInterrupted { request }),
            Err(ApplicationError::InvalidCommand)
        );
        assert!(
            effects
                .lock()
                .unwrap()
                .iter()
                .all(|effect| !matches!(effect, ImportEffect::SetGpuCandidate { .. }))
        );
    }

    #[test]
    fn terminal_job_state_allows_new_gpu_start_with_candidate_before_start_effect() {
        let model_root = fixture_root("terminal-gpu-restart");
        std::fs::create_dir_all(&model_root).unwrap();
        let io = RecordingIo::default();
        let effects = io.effects.clone();
        let events = io.events.clone();
        let snapshot = admission_snapshot(&model_root, ModelAvailability::Ready);
        let model_path = model_root
            .join(
                model_registry::model(ModelId::WhisperLargeV3Turbo)
                    .artifact
                    .file_name,
            )
            .to_string_lossy()
            .into_owned();
        let mut app = guarded_application(
            io,
            &model_root,
            "C:/known/turbo.bin".into(),
            admission_receiver(snapshot),
        );
        let previous = request(model_path.clone(), turbo_sha(), "archive".into());
        app.dispatch(AppCommand::StartImport {
            request: previous.clone(),
        })
        .unwrap();
        events.lock().unwrap().push_back(ImportEvent::Failed {
            job_id: previous.job_id,
            generation: previous.generation,
            message: "échec simulé".into(),
        });
        let terminal = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(
            terminal.active_job,
            Some((previous.job_id, whisper_core::JobState::Recoverable))
        );

        let mut next = request(model_path, turbo_sha(), "archive".into());
        next.job_id = JobId(902);
        next.config.compute = ComputeChoice::Gpu;
        let candidate = whisper_core::gpu_inventory::GpuRuntimeIdentity {
            generation: 1,
            pnp_instance_id: "PCI\\VEN_10DE&DEV_1B06".into(),
            dxgi_rank: 0,
            luid: 7,
            pci_id: "10DE:1B06".into(),
            cuda_ordinal: 0,
            ggml_ordinal: 0,
        };
        app.set_gpu_candidate(next.job_id, next.generation, Some(candidate.clone()))
            .unwrap();
        app.dispatch(AppCommand::StartImport {
            request: next.clone(),
        })
        .unwrap();

        let effects = effects.lock().unwrap();
        assert!(matches!(
            effects.as_slice(),
            [
                ImportEffect::Prepare(_),
                ImportEffect::SetGpuCandidate {
                    job_id,
                    generation,
                    candidate: Some(actual),
                },
                ImportEffect::Prepare(request),
            ] if *job_id == next.job_id
                && *generation == next.generation
                && actual == &candidate
                && request == &next
        ));
    }

    #[test]
    fn composed_catalog_rejects_turbo_changed_after_admission_before_worker_start() {
        let model_root = fixture_root("runtime-catalog");
        std::fs::create_dir_all(&model_root).unwrap();
        let record =
            model_registry::model(whisper_core::model_catalog::ModelId::WhisperLargeV3Turbo);
        let model_path = model_root.join(record.artifact.file_name);
        std::fs::File::create(&model_path)
            .unwrap()
            .set_len(record.artifact.size_bytes)
            .unwrap();
        let workers = WorkerPaths {
            cpu: model_root.join("missing-cpu-worker.exe"),
            gpu: model_root.join("missing-gpu-worker.exe"),
        };
        let historical = "C:/known/turbo.bin".to_owned();
        let io = AsyncImportIo::start_with_model_catalog(
            workers,
            model_root.clone(),
            historical.clone(),
        );
        let snapshot = admission_snapshot(&model_root, ModelAvailability::Ready);
        let mut app =
            guarded_application(io, &model_root, historical, admission_receiver(snapshot));

        // The admission snapshot marked the artifact ready; mutate it before runtime verification
        // to prove the adapter rechecks its digest before launching a worker.
        std::fs::OpenOptions::new()
            .write(true)
            .open(&model_path)
            .unwrap()
            .set_len(1)
            .unwrap();
        let archive = model_root.join("archive");
        app.dispatch(AppCommand::StartImport {
            request: request(
                model_path.to_string_lossy().into_owned(),
                turbo_sha(),
                archive.to_string_lossy().into_owned(),
            ),
        })
        .unwrap();

        let mut refusal = None;
        for _ in 0..300 {
            let view = app.dispatch(AppCommand::Refresh).unwrap();
            if view
                .message
                .as_deref()
                .is_some_and(|message| message.contains("Taille ou SHA-256 différent du catalogue"))
            {
                refusal = view.message;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(
            refusal.is_some(),
            "changed catalog artifact must be rejected before worker delegation; view={:?}",
            app.view().message
        );
    }

    #[cfg(feature = "personal-install")]
    #[test]
    fn personal_profile_uses_only_the_stable_model_worker_and_settings_layout() {
        let root = PathBuf::from(r"C:\Users\tester\AppData\Local\Whisper");
        assert_eq!(personal_model_root(&root), root.join("models"));
        let cpu = personal_worker_paths(&root, whisper_core::model_catalog::Backend::Cpu);
        assert_eq!(
            cpu.executable,
            root.join("worker/whisper-worker-parakeet-cpu.exe")
        );
        assert_eq!(cpu.native_dir, root.join("native/cpu"));
        assert_eq!(
            cpu.model,
            root.join("models/parakeet-tdt-0.6b-v3.q8_0.gguf")
        );
        let gpu = personal_worker_paths(&root, whisper_core::model_catalog::Backend::Gpu);
        assert_eq!(
            gpu.executable,
            root.join("worker/whisper-worker-parakeet-gpu.exe")
        );
        assert_eq!(gpu.native_dir, root.join("native/gpu"));
        assert_eq!(
            personal_settings_path(&root),
            root.join("settings/settings.json")
        );
    }
}
