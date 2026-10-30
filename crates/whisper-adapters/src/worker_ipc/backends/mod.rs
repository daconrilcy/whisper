use super::{ChildWorkerTransport, support::decode_sha256};
use crate::{WorkerTransport, supervisor::WorkerPaths};
use std::{
    path::Path,
    thread,
    time::{Duration, Instant},
};
use whisper_core::{
    ComputeChoice,
    ipc::{WorkerCommand, WorkerEvent},
    ports::{ImportRequest, LiveRequest},
};
mod native;
pub(crate) use native::gpu_evidence_matches;
pub(super) use native::{
    NativeUnifiedRoute, can_retry_native_auto_on_cpu, gpu_qualification_candidate_allowed,
    native_cpu_route, native_unified_route,
};

/// Check the shipped CUDA backend without loading or discarding the model.
pub fn probe_cuda_worker(executable: &Path) -> Result<(), String> {
    if !executable.is_file() {
        return Err("GPU indisponible : worker CUDA introuvable ; choisissez CPU".into());
    }
    let mut worker = ChildWorkerTransport::spawn(executable)
        .map_err(|error| format!("GPU indisponible : {error:?} ; choisissez CPU"))?;
    let result = (|| {
        worker
            .request(WorkerCommand::ProbeCuda)
            .map_err(|error| format!("GPU indisponible : {error:?} ; choisissez CPU"))?;
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            match worker.receive() {
                Ok(Some(WorkerEvent::CudaAvailable)) => return Ok(()),
                Ok(Some(WorkerEvent::Failed { message, .. })) => {
                    return Err(format!("GPU indisponible : {message} ; choisissez CPU"));
                }
                Ok(Some(_)) | Ok(None) => {}
                Err(error) => return Err(format!("GPU indisponible : {error:?} ; choisissez CPU")),
            }
            if Instant::now() >= deadline {
                return Err(
                    "GPU indisponible : délai d'initialisation CUDA dépassé ; choisissez CPU"
                        .into(),
                );
            }
            thread::sleep(Duration::from_millis(10));
        }
    })();
    worker.abort();
    result
}

/// Ask the isolated GPU worker to return per-identity qualification records.
pub fn qualify_gpu_worker(
    executable: &Path,
    generation: u64,
    targets: Vec<whisper_core::ipc::GpuQualificationTarget>,
) -> Result<(String, Vec<whisper_core::ipc::GpuQualificationResult>), String> {
    let mut worker = ChildWorkerTransport::spawn(executable)
        .map_err(|error| format!("GPU worker indisponible : {error:?}"))?;
    let result = (|| {
        worker
            .request(WorkerCommand::QualifyGpus {
                generation,
                targets,
            })
            .map_err(|error| format!("Qualification GPU impossible : {error:?}"))?;
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            match worker.receive() {
                Ok(Some(WorkerEvent::GpusQualified {
                    generation: response_generation,
                    build_architectures,
                    devices,
                })) if response_generation == generation => {
                    return Ok((build_architectures, devices));
                }
                Ok(Some(WorkerEvent::GpusQualified { .. })) => {
                    return Err("Réponse de qualification GPU périmée.".into());
                }
                Ok(Some(WorkerEvent::Failed { message, .. })) => return Err(message),
                Ok(Some(_)) | Ok(None) => {}
                Err(error) => return Err(format!("Transport GPU interrompu : {error:?}")),
            }
            if Instant::now() >= deadline {
                return Err("Délai de qualification GPU dépassé.".into());
            }
            thread::sleep(Duration::from_millis(10));
        }
    })();
    worker.abort();
    result
}

pub(super) struct RuntimeModelCatalog {
    pub(super) catalog: crate::model_catalog::LocalModelCatalog,
    pub(super) known_historical_turbo_path: String,
    pub(super) parakeet_paths: Option<crate::parakeet_import::ParakeetWorkerPaths>,
    pub(super) gpu_parakeet_paths: Option<crate::parakeet_import::ParakeetWorkerPaths>,
}

impl RuntimeModelCatalog {
    pub(super) fn clone_for_worker(&self) -> Self {
        Self {
            catalog: crate::model_catalog::LocalModelCatalog::new(
                self.catalog.root().to_path_buf(),
            ),
            known_historical_turbo_path: self.known_historical_turbo_path.clone(),
            parakeet_paths: self.parakeet_paths.clone(),
            gpu_parakeet_paths: self.gpu_parakeet_paths.clone(),
        }
    }

    pub(super) fn snapshot(&self) -> crate::model_catalog::CatalogSnapshot {
        self.catalog.inspect(0)
    }

    pub(super) fn bind_import(
        &self,
        request: &whisper_core::ports::ImportRequest,
    ) -> Result<whisper_core::ports::ImportRequest, String> {
        use whisper_core::model_catalog::ModelId;
        let snapshot = self.snapshot();
        let id = snapshot
            .resolve_model_identity(&request.model_path, &self.known_historical_turbo_path)?;
        if id == ModelId::WhisperLargeV3Turbo {
            return snapshot.bind_turbo_import_request(request, &self.known_historical_turbo_path);
        }
        if id != ModelId::ParakeetTdt06BV3 {
            return Err("ModelUnknown: modèle non reconnu".into());
        }
        if request.strategy
            == whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
        {
            return Ok(request.clone());
        }
        let record = whisper_core::model_registry::model(id);
        let source_samples = request.source_samples.or_else(|| {
            crate::archive::source_output_samples(Path::new(&request.source_path)).ok()
        });
        if source_samples.is_some_and(|samples| {
            samples
                > whisper_core::model_registry::PARAKEET_MAX_SINGLE_PASS_DURATION_MS
                    .saturating_mul(16)
        }) {
            return Err(
                "DurationUnsupported: Fichier trop long pour Parakeet, utilisez Turbo".into(),
            );
        }
        let profile = if source_samples.is_some_and(|samples| {
            samples > whisper_core::parakeet_ipc::PARAKEET_MAX_AUDIO_SAMPLES as u64
        }) {
            whisper_core::parakeet_windows::PROFILE_IMPORT_VAD
        } else {
            whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS
        };
        let (paths, key) = match request.config.compute {
            whisper_core::ComputeChoice::Cpu | whisper_core::ComputeChoice::Auto => {
                let paths = self.parakeet_paths.as_ref().ok_or_else(|| {
                    "ModelNotQualified: worker CPU Test Parakeet absent".to_owned()
                })?;
                (paths, paths.qualification_key(profile)?)
            }
            whisper_core::ComputeChoice::Gpu => {
                let paths = self.gpu_parakeet_paths.as_ref().ok_or_else(|| {
                    "BackendUnavailable: worker GPU Test Parakeet absent".to_owned()
                })?;
                let key = paths.gpu_qualification_key(profile)?;
                if !gpu_qualification_candidate_allowed(&key) {
                    return Err("ModelNotQualified: Parakeet GPU candidate is not admitted".into());
                }
                (paths, key)
            }
        };
        paths.verify_qualification(
            record.artifact.sha256,
            &key.runtime_build,
            &key.worker_build,
        )?;
        if !matches!(
            &request.config.language,
            whisper_core::LanguageChoice::Automatic
        ) {
            return Err(
                "ModelNotQualified: Parakeet accepte seulement l'import et la langue automatique"
                    .into(),
            );
        }
        if profile == whisper_core::parakeet_windows::PROFILE_IMPORT_VAD
            && !key.is_parakeet_vad_test()
        {
            return Err("ModelNotQualified: import-vad Test key is unavailable".into());
        }
        if !matches!(
            record.admission,
            whisper_core::model_registry::AdmissionBasis::Measured(
                whisper_core::engine_capabilities::Qualification::Qualified
            )
        ) {
            return Err("ModelNotQualified: la fiche Test Parakeet n'est pas qualifiée".into());
        }
        let entry = snapshot
            .entries
            .iter()
            .find(|entry| entry.id == id)
            .ok_or_else(|| "ModelMissing: Parakeet absent du catalogue".to_owned())?;
        if entry.availability != whisper_core::model_catalog::ModelAvailability::Ready
            || request.model_sha256 != decode_sha256(record.artifact.sha256)?
            || entry.expected_size_bytes != record.artifact.size_bytes
            || !entry
                .observed_sha256
                .as_deref()
                .is_some_and(|hash| hash.eq_ignore_ascii_case(record.artifact.sha256))
        {
            return Err(
                "ModelHashMismatch: artefact Parakeet différent du catalogue qualifié".into(),
            );
        }
        let mut executable = request.clone();
        executable.model_path = entry.path.to_string_lossy().into_owned();
        Ok(executable)
    }

    pub(super) fn bind_live(
        &self,
        request: &whisper_core::ports::LiveRequest,
    ) -> Result<whisper_core::ports::LiveRequest, String> {
        self.snapshot()
            .bind_turbo_live_request(request, &self.known_historical_turbo_path)
    }

    pub(super) fn bind_native_unified_live(
        &self,
        request: &LiveRequest,
    ) -> Result<LiveRequest, String> {
        use whisper_core::model_catalog::ModelId;
        let id = self
            .snapshot()
            .resolve_model_identity(&request.model_path, &self.known_historical_turbo_path)?;
        if id != ModelId::ParakeetTdt06BV3 {
            return Err("ModelUnknown: native unified capture requires Parakeet TDT v3".into());
        }
        let record = whisper_core::model_registry::model(id);
        if request.model_sha256 != decode_sha256(record.artifact.sha256)? {
            return Err("ModelHashMismatch: native unified capture request differs from the registered artifact".into());
        }
        let model_path = self
            .parakeet_paths
            .as_ref()
            .or(self.gpu_parakeet_paths.as_ref())
            .ok_or_else(|| "NativeCandidateUnavailable: personal model path is absent".to_owned())?
            .model
            .clone();
        let mut bound = request.clone();
        bound.model_path = model_path.to_string_lossy().into_owned();
        Ok(bound)
    }

    pub(super) fn bind_parakeet_live(
        &self,
        request: &LiveRequest,
        candidate: Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
    ) -> Result<
        (
            LiveRequest,
            crate::parakeet_import::ParakeetWorkerPaths,
            Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
        ),
        String,
    > {
        use whisper_core::model_catalog::ModelId;
        let snapshot = self.snapshot();
        if snapshot
            .resolve_model_identity(&request.model_path, &self.known_historical_turbo_path)?
            != ModelId::ParakeetTdt06BV3
        {
            return Err("ModelUnknown: la voie microphone Parakeet exige Parakeet TDT v3".into());
        }
        let requested = request.config.compute;
        let mut effective = match requested {
            ComputeChoice::Cpu => ComputeChoice::Cpu,
            ComputeChoice::Gpu => ComputeChoice::Gpu,
            ComputeChoice::Auto if candidate.is_some() && self.gpu_parakeet_paths.is_some() => {
                ComputeChoice::Gpu
            }
            ComputeChoice::Auto => ComputeChoice::Cpu,
        };
        let make_import = |compute| ImportRequest {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
            job_id: request.job_id,
            generation: request.generation,
            source_path: request.model_path.clone(),
            source_sha256: None,
            source_samples: Some(1),
            model_path: request.model_path.clone(),
            model_sha256: request.model_sha256,
            destination: request.destination.clone(),
            config: whisper_core::JobConfig {
                compute,
                ..request.config.clone()
            },
        };
        let bound = match self.bind_import(&make_import(effective)) {
            Ok(bound) => bound,
            Err(error) if requested == ComputeChoice::Auto && effective == ComputeChoice::Gpu => {
                effective = ComputeChoice::Cpu;
                self.bind_import(&make_import(effective))
                    .map_err(|cpu_error| {
                        format!("Auto GPU indisponible ({error}) ; CPU indisponible ({cpu_error})")
                    })?
            }
            Err(error) => return Err(error),
        };
        let paths =
            match effective {
                ComputeChoice::Cpu => self.parakeet_paths.clone().ok_or_else(|| {
                    "ModelNotQualified: worker CPU Test Parakeet absent".to_owned()
                })?,
                ComputeChoice::Gpu => self.gpu_parakeet_paths.clone().ok_or_else(|| {
                    "BackendUnavailable: worker GPU Test Parakeet absent".to_owned()
                })?,
                ComputeChoice::Auto => unreachable!(),
            };
        let mut bound_request = request.clone();
        bound_request.model_path = bound.model_path;
        bound_request.config.compute = effective;
        let candidate = if effective == ComputeChoice::Gpu {
            candidate
        } else {
            None
        };
        Ok((bound_request, paths, candidate))
    }

    pub(super) fn is_parakeet_live_request(&self, request: &LiveRequest) -> Result<bool, String> {
        self.snapshot()
            .resolve_model_identity(&request.model_path, &self.known_historical_turbo_path)
            .map(|id| id == whisper_core::model_catalog::ModelId::ParakeetTdt06BV3)
    }
}

pub(super) fn request_is_parakeet(request: &ImportRequest) -> bool {
    decode_sha256(
        whisper_core::model_registry::model(whisper_core::model_catalog::ModelId::ParakeetTdt06BV3)
            .artifact
            .sha256,
    )
    .is_ok_and(|sha| request.model_sha256 == sha)
}

pub(super) fn validate_current_queued_gpu_candidate(
    request: &ImportRequest,
    candidate: Option<&whisper_core::gpu_inventory::GpuRuntimeIdentity>,
) -> Result<(), String> {
    if !request_is_parakeet(request) || request.config.compute != whisper_core::ComputeChoice::Gpu {
        return Ok(());
    }
    let candidate = candidate.ok_or_else(|| {
        "BackendMismatch: selected GPU has no current runtime identity proof".to_owned()
    })?;
    if candidate.generation != request.generation.get() {
        return Err("BackendMismatch: selected GPU identity belongs to a stale generation".into());
    }
    Ok(())
}

pub(super) fn validate_queued_gpu_preflight(
    request: &ImportRequest,
    candidate: Option<&whisper_core::gpu_inventory::GpuRuntimeIdentity>,
    qualification: Option<&crate::archive::ArchiveQualificationKey>,
) -> Result<(), String> {
    validate_current_queued_gpu_candidate(request, candidate)?;
    if !request_is_parakeet(request) || request.config.compute != whisper_core::ComputeChoice::Gpu {
        return Ok(());
    }
    let key = qualification
        .ok_or_else(|| "ModelNotQualified: GPU Test key is not admitted".to_owned())?;
    if key.backend != "CUDA0"
        || key.inference_profile != whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS
    {
        return Err("ModelNotQualified: GPU Test key is not admitted".into());
    }
    Ok(())
}

pub(super) fn archive_qualification_for_request(
    request: &ImportRequest,
    parakeet_paths: Option<&crate::parakeet_import::ParakeetWorkerPaths>,
    gpu_parakeet_paths: Option<&crate::parakeet_import::ParakeetWorkerPaths>,
    gpu_candidate: Option<&whisper_core::gpu_inventory::GpuRuntimeIdentity>,
) -> Option<crate::archive::ArchiveQualificationKey> {
    if !request_is_parakeet(request)
        || request.strategy
            == whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
    {
        return None;
    }
    let samples = request
        .source_samples
        .or_else(|| crate::archive::source_output_samples(Path::new(&request.source_path)).ok())?;
    let profile = if samples > whisper_core::parakeet_ipc::PARAKEET_MAX_AUDIO_SAMPLES as u64 {
        whisper_core::parakeet_windows::PROFILE_IMPORT_VAD
    } else {
        whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS
    };
    let gpu_requested = request.config.compute == whisper_core::ComputeChoice::Gpu
        || (request.config.compute == whisper_core::ComputeChoice::Auto && gpu_candidate.is_some());
    if gpu_requested {
        if let Some(key) = gpu_parakeet_paths
            .and_then(|paths| paths.gpu_qualification_key(profile).ok())
            .filter(gpu_qualification_candidate_allowed)
        {
            return Some(key);
        }
        if request.config.compute == whisper_core::ComputeChoice::Gpu {
            return None;
        }
    }
    parakeet_paths
        .and_then(|paths| paths.qualification_key(profile).ok())
        .or_else(|| {
            (profile == whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS
                && !parakeet_paths.is_some_and(|paths| paths.is_personal_runtime()))
            .then(crate::archive::ArchiveQualificationKey::parakeet_cpu_test)
        })
}

pub(super) const WORKER_IDLE_RETENTION: Duration = Duration::from_secs(5 * 60);

pub(super) struct IdleWorker {
    pub(super) transport: ChildWorkerTransport,
    pub(super) compute: whisper_core::ComputeChoice,
    pub(super) model_path: String,
    pub(super) model_sha256: [u8; 32],
    pub(super) expires_at: Instant,
}

impl IdleWorker {
    pub(super) fn matches(&mut self, request: &LiveRequest, workers: &WorkerPaths) -> bool {
        let compute = if request.config.compute == whisper_core::ComputeChoice::Auto {
            if workers.gpu.is_file() {
                whisper_core::ComputeChoice::Gpu
            } else {
                whisper_core::ComputeChoice::Cpu
            }
        } else {
            request.config.compute
        };
        self.matches_identity(compute, &request.model_path, request.model_sha256)
    }

    pub(super) fn matches_identity(
        &mut self,
        compute: whisper_core::ComputeChoice,
        model_path: &str,
        model_sha256: [u8; 32],
    ) -> bool {
        self.expires_at > Instant::now()
            && self.compute == compute
            && self.model_path == model_path
            && self.model_sha256 == model_sha256
            && self
                .transport
                .try_wait()
                .is_ok_and(|status| status.is_none())
    }
}
