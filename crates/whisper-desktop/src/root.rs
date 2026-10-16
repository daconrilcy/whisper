use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver, TryRecvError},
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
    let model_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("models");
    let (catalog_sender, catalog_receiver) = std::sync::mpsc::sync_channel(1);
    let (admission_sender, admission_receiver) = std::sync::mpsc::sync_channel(1);
    let catalog_root = model_root.clone();
    std::thread::spawn(move || {
        let snapshot = LocalModelCatalog::new(catalog_root).inspect(1);
        let _ = catalog_sender.send(snapshot.clone());
        let _ = admission_sender.send(snapshot);
    });

    let (preference_command_sender, preference_command_receiver) =
        model_preference_command_channel();
    let (preference_event_sender, preference_event_receiver) = std::sync::mpsc::sync_channel(4);
    std::thread::spawn(move || {
        let preferences = ModelPreferencesApp::new(Box::new(WindowsModelPreferencePort::new()));
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
    let inventory_worker = workers.gpu.clone();
    let (sender, receiver) = mpsc::channel();
    let gpu_worker = workers.gpu.clone();
    std::thread::spawn(move || {
        let result = probe_cuda_worker(&gpu_worker);
        let _ = sender.send(result);
    });
    let historical_model_path = whisper_desktop::ui::default_model_path();
    let io = AsyncImportIo::start_with_model_catalog(
        workers,
        model_root.clone(),
        historical_model_path.clone(),
    );
    let application =
        guarded_application(io, &model_root, historical_model_path, admission_receiver);
    DesktopComposition {
        application,
        compute_choice: ComputeChoice::Cpu,
        startup_message: Some("Vérification du GPU en cours…".into()),
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
    let core_guard = ModelCommandGuard::new(
        model_registry::engine_snapshot(1, |_| ModelAvailability::Ready, Vec::new()),
        1,
        [app_model_path, historical_model_path.clone()],
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
        if self.admission_guard.is_some() || self.admission_catalog_failed {
            return;
        }
        match self.admission_catalog.try_recv() {
            Ok(snapshot) => {
                let turbo_path = snapshot
                    .entries
                    .iter()
                    .find(|entry| entry.id == ModelId::WhisperLargeV3Turbo)
                    .map(|entry| entry.path.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let guard = ModelCommandGuard::new(
                    snapshot.engine_snapshot(Vec::new()),
                    snapshot.generation,
                    [turbo_path, self.historical_model_path.clone()],
                );
                self.admission_guard = Some(guard);
            }
            Err(TryRecvError::Disconnected) => self.admission_catalog_failed = true,
            Err(TryRecvError::Empty) => {}
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
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        repo.join("target/parakeet-tests/l1b-2")
            .join(format!("{name}-{}-{suffix}", std::process::id()))
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
}
