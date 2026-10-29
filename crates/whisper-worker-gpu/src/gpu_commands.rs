use crate::{
    gpu_attestation, gpu_qualification, integrity::verify_sha256, ipc::write_frame, native_ffi,
};
use std::path::Path;
use whisper_core::ipc::{IpcEnvelope, WorkerCommand, WorkerErrorCode, WorkerEvent};

pub(crate) fn handle(
    first: IpcEnvelope<WorkerCommand>,
    output: &mut impl std::io::Write,
) -> Result<Option<IpcEnvelope<WorkerCommand>>, String> {
    if matches!(first.message, WorkerCommand::ProbeCuda) {
        return match native_ffi::cuda_backend_available() {
            true => write_frame(output, &IpcEnvelope::new(WorkerEvent::CudaAvailable))
                .map_err(|error| error.to_string())
                .map(|_| None),
            false => write_frame(
                output,
                &IpcEnvelope::new(WorkerEvent::Failed {
                    job_id: None,
                    generation: None,
                    instance_id: None,
                    code: WorkerErrorCode::BackendUnavailable,
                    message: "Aucun backend CUDA initialisable dans le worker GPU".into(),
                }),
            )
            .map_err(|error| error.to_string())
            .map(|_| None),
        };
    }
    if let WorkerCommand::AttestGpuCandidate {
        session_id,
        model_path,
        model_sha256,
        candidate,
    } = first.message
    {
        if let Err(error) = verify_sha256(Path::new(&model_path), &model_sha256) {
            return write_frame(
                output,
                &IpcEnvelope::new(WorkerEvent::Failed {
                    job_id: None,
                    generation: None,
                    instance_id: None,
                    code: WorkerErrorCode::ModelHashMismatch,
                    message: format!("ModelHashMismatch: {error}"),
                }),
            )
            .map_err(|error| error.to_string())
            .map(|_| None);
        }
        if let Err(error) = native_ffi::verify_windows_identity(&candidate) {
            return write_frame(
                output,
                &IpcEnvelope::new(WorkerEvent::Failed {
                    job_id: None,
                    generation: None,
                    instance_id: None,
                    code: WorkerErrorCode::BackendMismatch,
                    message: format!("BackendMismatch: {error}"),
                }),
            )
            .map_err(|error| error.to_string())
            .map(|_| None);
        }
        let cuda = native_ffi::cuda_devices()?;
        gpu_attestation::attest_native_identity(&candidate, &cuda, &native_ffi::ggml_devices())?;
        return write_frame(
            output,
            &IpcEnvelope::new(WorkerEvent::GpuCandidateAttested {
                session_id,
                candidate: candidate.pnp_instance_id.clone(),
                dxgi_rank: candidate.dxgi_rank,
                luid: candidate.luid,
                pci_id: candidate.pci_id.clone(),
                cuda_ordinal: candidate.cuda_ordinal,
                ggml_ordinal: candidate.ggml_ordinal,
            }),
        )
        .map_err(|error| error.to_string())
        .map(|_| None);
    }
    if let WorkerCommand::QualifyGpus {
        generation,
        targets,
    } = first.message
    {
        let devices = gpu_qualification::qualify(&targets);
        return write_frame(
            output,
            &IpcEnvelope::new(WorkerEvent::GpusQualified {
                generation,
                build_architectures: gpu_qualification::BUILD_ARCHITECTURES.into(),
                devices,
            }),
        )
        .map_err(|error| error.to_string())
        .map(|_| None);
    }
    Ok(Some(first))
}
