use super::super::support::decode_sha256;
use super::request_is_parakeet;
use whisper_core::{ComputeChoice, ports::ImportRequest};

#[derive(Clone, Debug)]
pub(in crate::worker_ipc) struct NativeUnifiedRoute {
    pub(in crate::worker_ipc) paths: crate::parakeet_import::ParakeetWorkerPaths,
    pub(in crate::worker_ipc) requested_compute: ComputeChoice,
    pub(in crate::worker_ipc) compute: ComputeChoice,
    pub(in crate::worker_ipc) expected_gpu: Option<whisper_core::parakeet_ipc::ExpectedGpu>,
    pub(in crate::worker_ipc) candidate: Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
    pub(in crate::worker_ipc) fallback_reason: Option<String>,
}

impl NativeUnifiedRoute {
    pub(in crate::worker_ipc) fn same_worker(&self, other: &Self) -> bool {
        self.compute == other.compute
            && self.paths.executable == other.paths.executable
            && self.paths.native_dir == other.paths.native_dir
            && self.paths.model == other.paths.model
            && self.paths.runtime_root == other.paths.runtime_root
            && self.expected_gpu == other.expected_gpu
            && self.candidate == other.candidate
    }

    pub(in crate::worker_ipc) fn backend_label(&self) -> String {
        match (&self.compute, &self.fallback_reason) {
            (ComputeChoice::Gpu, _) => "CUDA0".into(),
            (ComputeChoice::Cpu, Some(reason)) => {
                format!("CPU (Auto fallback before audio: {reason})")
            }
            (ComputeChoice::Cpu, None) => "CPU".into(),
            (ComputeChoice::Auto, _) => "Unresolved Auto".into(),
        }
    }
}

pub(in crate::worker_ipc) fn can_retry_native_auto_on_cpu(
    requested_compute: ComputeChoice,
    effective_compute: ComputeChoice,
    backend_ready_reported: bool,
    call_in_flight: bool,
) -> bool {
    requested_compute == ComputeChoice::Auto
        && effective_compute == ComputeChoice::Gpu
        && !backend_ready_reported
        && !call_in_flight
}

pub(in crate::worker_ipc) fn native_unified_route(
    request: &ImportRequest,
    cpu_paths: Option<&crate::parakeet_import::ParakeetWorkerPaths>,
    gpu_paths: Option<&crate::parakeet_import::ParakeetWorkerPaths>,
    candidate: Option<&whisper_core::gpu_inventory::GpuRuntimeIdentity>,
) -> Result<NativeUnifiedRoute, String> {
    if request.strategy != whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
        || !request_is_parakeet(request)
    {
        return Err(
            "NativeCandidateUnavailable: only BoundedWindowsV1 Parakeet requests are admitted"
                .into(),
        );
    }
    if !matches!(
        &request.config.language,
        whisper_core::LanguageChoice::Automatic
    ) {
        return Err("NativeCandidateUnavailable: Parakeet requires automatic language".into());
    }
    let record =
        whisper_core::model_registry::model(whisper_core::model_catalog::ModelId::ParakeetTdt06BV3);
    if request.model_sha256 != decode_sha256(record.artifact.sha256)? {
        return Err(
            "ModelHashMismatch: request model is not the registered Parakeet artifact".into(),
        );
    }

    let auto = request.config.compute == ComputeChoice::Auto;
    let gpu_candidate =
        candidate.filter(|identity| identity.generation == request.generation.get());
    let attempt_gpu = request.config.compute == ComputeChoice::Gpu
        || (auto && gpu_candidate.is_some() && gpu_paths.is_some());
    if attempt_gpu {
        let gpu_result = (|| {
            let identity = gpu_candidate.ok_or_else(|| {
                "BackendMismatch: selected GPU has no current generation identity".to_owned()
            })?;
            let paths = gpu_paths.ok_or_else(|| {
                "BackendUnavailable: personal Parakeet GPU worker is absent".to_owned()
            })?;
            if !paths.is_personal_runtime() {
                return Err(
                    "NativeCandidateUnavailable: GPU worker must use the personal runtime layout"
                        .into(),
                );
            }
            let expected_gpu = crate::parakeet_import::expected_gpu_for_candidate(identity)?;
            let profile = whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS;
            // This key verifies installed asset identity only. It is deliberately not
            // attached to BoundedWindowsV1 archives or treated as long-session proof.
            let asset_key = paths.gpu_qualification_key(profile)?;
            paths.verify_qualification(
                record.artifact.sha256,
                &asset_key.runtime_build,
                &asset_key.worker_build,
            )?;
            Ok(NativeUnifiedRoute {
                paths: paths.clone(),
                requested_compute: request.config.compute,
                compute: ComputeChoice::Gpu,
                expected_gpu: Some(expected_gpu),
                candidate: Some(identity.clone()),
                fallback_reason: None,
            })
        })();
        match gpu_result {
            Ok(route) => return Ok(route),
            Err(error) if auto => {
                let mut route = native_cpu_route(request, cpu_paths)?;
                route.fallback_reason = Some(format!("GPU pre-audio validation failed: {error}"));
                return Ok(route);
            }
            Err(error) => return Err(error),
        }
    }
    let mut route = native_cpu_route(request, cpu_paths)?;
    if auto {
        route.fallback_reason = Some(if candidate.is_some() {
            "GPU candidate was stale or the personal GPU route was unavailable before audio".into()
        } else {
            "No current GPU candidate was available before audio".into()
        });
    }
    Ok(route)
}

pub(in crate::worker_ipc) fn native_cpu_route(
    request: &ImportRequest,
    cpu_paths: Option<&crate::parakeet_import::ParakeetWorkerPaths>,
) -> Result<NativeUnifiedRoute, String> {
    let paths = cpu_paths
        .ok_or_else(|| "ModelNotQualified: personal Parakeet CPU worker is absent".to_owned())?;
    if !paths.is_personal_runtime() {
        return Err(
            "NativeCandidateUnavailable: CPU worker must use the personal runtime layout".into(),
        );
    }
    let record =
        whisper_core::model_registry::model(whisper_core::model_catalog::ModelId::ParakeetTdt06BV3);
    let asset_key =
        paths.qualification_key(whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS)?;
    paths.verify_qualification(
        record.artifact.sha256,
        &asset_key.runtime_build,
        &asset_key.worker_build,
    )?;
    Ok(NativeUnifiedRoute {
        paths: paths.clone(),
        requested_compute: request.config.compute,
        compute: ComputeChoice::Cpu,
        expected_gpu: None,
        candidate: None,
        fallback_reason: None,
    })
}

pub(in crate::worker_ipc) fn gpu_qualification_candidate_allowed(
    key: &crate::archive::ArchiveQualificationKey,
) -> bool {
    #[cfg(feature = "recette-hooks")]
    let test_injection = cfg!(feature = "l3-gpu-qualification")
        && option_env!("PARAKEET_L2_ACCEPTANCE_TEST_BUILD").is_some()
        && key == &crate::archive::ArchiveQualificationKey::parakeet_gpu_test();
    #[cfg(feature = "recette-hooks")]
    if test_injection {
        return true;
    }
    if key.is_personal_candidate() || key.is_registered_personal_import() {
        return key.backend == "CUDA0"
            && key.inference_profile
                == whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS;
    }
    let record =
        whisper_core::model_registry::model(whisper_core::model_catalog::ModelId::ParakeetTdt06BV3);
    let registered = whisper_core::model_registry::configuration_key(
        record,
        whisper_core::model_catalog::Backend::Gpu,
    );
    key.backend == "CUDA0"
        && key.artifact_revision == registered.artifact_revision
        && key.runtime_build == registered.runtime_build
        && key.worker_build == registered.worker_build
        && key.inference_profile == registered.inference_profile
}

pub(crate) fn gpu_evidence_matches(
    evidence: Option<&whisper_core::parakeet_ipc::GpuEvidence>,
    expected: Option<&whisper_core::parakeet_ipc::ExpectedGpu>,
    pid: u32,
) -> bool {
    let (Some(evidence), Some(expected)) = (evidence, expected) else {
        return false;
    };
    evidence.backend == "CUDA0"
        && evidence
            .pci_bus_id
            .eq_ignore_ascii_case(&expected.pci_bus_id)
        && evidence.uuid.eq_ignore_ascii_case(&expected.uuid)
        && evidence.pid == pid
        && evidence.asr_log_sha256.len() == 64
        && evidence
            .asr_log_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
}
