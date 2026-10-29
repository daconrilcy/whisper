#[cfg(feature = "l01-memory-qualification")]
use crate::memory_qualification;
use crate::{
    decoder,
    diagnostics::{emit_failure, emit_gpu_initialization_failure, emit_gpu_revalidation_failure},
    encoder,
    engine_cache::{CachedGpuEngine, cached_gpu_engine},
    gpu_attestation, gpu_commands,
    integrity::verify_sha256,
    ipc::{Incoming, write_frame},
    native_engine::{self, GpuEngine},
    native_ffi,
    qualification::{l04_campaign_gate, l04_campaign_worker_result},
};
use std::{
    io,
    path::Path,
    sync::mpsc::{self, Receiver},
};
use whisper_core::{
    Generation, JobConfig, JobId, SegmentId,
    domain::{ComputeChoice, SourceRange},
    gpu_inventory::GpuRuntimeIdentity,
    ipc::{
        BackendKind, IPC_PROTOCOL_VERSION, IpcEnvelope, WorkerCommand, WorkerErrorCode,
        WorkerEvent, WorkerSegmentDto,
    },
    ports::{DecodeRequest, DecodedPcmBlock, PcmBlock, WorkerSegment},
};
mod control;
mod decode;
mod import;
mod live;
use control::take_stop;
use decode::run_decode_service;
use live::run_live_service;
struct ImportRequest {
    job_id: JobId,
    generation: Generation,
    instance_id: u64,
    source_path: String,
    source_sha256: [u8; 32],
    expected_source_samples: u64,
    model_path: String,
    model_sha256: [u8; 32],
    config: JobConfig,
    gpu_candidate: Option<GpuRuntimeIdentity>,
}

pub(crate) fn run_session(
    rx: &Receiver<Incoming>,
    output: &mut impl io::Write,
    first: IpcEnvelope<WorkerCommand>,
    cached_engine: &mut Option<CachedGpuEngine>,
) -> Result<(), String> {
    if first.version != IPC_PROTOCOL_VERSION {
        write_frame(
            output,
            &IpcEnvelope::new(WorkerEvent::Failed {
                job_id: None,
                generation: None,
                instance_id: None,
                code: WorkerErrorCode::UnsupportedProtocol,
                message: format!(
                    "ProtocolMismatch: expected v{IPC_PROTOCOL_VERSION}, got v{}",
                    first.version
                ),
            }),
        )
        .map_err(|e| e.to_string())?;
        return Ok(());
    }
    let Some(first) = gpu_commands::handle(first, output)? else {
        return Ok(());
    };
    let gpu_candidate = match &first.message {
        WorkerCommand::StartGpuImport { candidate, .. } => Some(candidate.clone()),
        WorkerCommand::StartGpuLiveWorker { candidate, .. }
        | WorkerCommand::StartGpuDirectLiveWorker { candidate, .. } => Some(candidate.clone()),
        _ => None,
    };
    let startup_identity = match &first.message {
        WorkerCommand::StartGpuImport {
            job_id,
            generation,
            instance_id,
            ..
        }
        | WorkerCommand::StartGpuLiveWorker {
            job_id,
            generation,
            instance_id,
            ..
        }
        | WorkerCommand::StartGpuDirectLiveWorker {
            job_id,
            generation,
            instance_id,
            ..
        } => Some((*job_id, *generation, *instance_id)),
        _ => None,
    };
    if let Some(candidate) = &gpu_candidate {
        let validation = native_ffi::verify_windows_identity(candidate)
            .and_then(|()| native_ffi::cuda_devices())
            .and_then(|cuda| {
                gpu_attestation::attest_native_identity(
                    candidate,
                    &cuda,
                    &native_ffi::ggml_devices(),
                )
            });
        if let Err(error) = validation {
            let (job_id, generation, instance_id) =
                startup_identity.expect("GPU startup command has correlated identity");
            return emit_gpu_revalidation_failure(output, job_id, generation, instance_id, &error);
        }
    }
    let direct_live = matches!(
        &first.message,
        WorkerCommand::StartDirectLiveWorker { .. }
            | WorkerCommand::StartGpuDirectLiveWorker { .. }
    );
    let request = match first.message {
        WorkerCommand::DecodeBlock {
            request_id,
            request,
        } => {
            return run_decode_service(rx, output, request_id, request);
        }
        WorkerCommand::StartLiveWorker {
            job_id,
            generation,
            instance_id,
            model_path,
            model_sha256,
            config,
        }
        | WorkerCommand::StartDirectLiveWorker {
            job_id,
            generation,
            instance_id,
            model_path,
            model_sha256,
            config,
        } => {
            if config.compute == ComputeChoice::Cpu
                || verify_sha256(Path::new(&model_path), &model_sha256).is_err()
            {
                return emit_failure(
                    output,
                    job_id,
                    generation,
                    instance_id,
                    WorkerErrorCode::ModelHashMismatch,
                    "ModelHashMismatch: live model identity could not be verified",
                );
            }
            let engine = match cached_gpu_engine(
                cached_engine,
                &model_path,
                model_sha256,
                gpu_candidate.as_ref(),
            ) {
                Ok(engine) => engine,
                Err(error) => {
                    return emit_gpu_initialization_failure(
                        output,
                        job_id,
                        generation,
                        instance_id,
                        &error,
                    );
                }
            };
            write_frame(
                output,
                &IpcEnvelope::new(WorkerEvent::Ready {
                    job_id,
                    generation,
                    instance_id,
                    backend: BackendKind::Cuda,
                }),
            )
            .map_err(|e| e.to_string())?;
            return run_live_service(
                rx,
                output,
                engine,
                job_id,
                generation,
                instance_id,
                config,
                !direct_live,
            );
        }
        WorkerCommand::StartGpuLiveWorker {
            job_id,
            generation,
            instance_id,
            model_path,
            model_sha256,
            config,
            ..
        }
        | WorkerCommand::StartGpuDirectLiveWorker {
            job_id,
            generation,
            instance_id,
            model_path,
            model_sha256,
            config,
            ..
        } => {
            if config.compute == ComputeChoice::Cpu
                || verify_sha256(Path::new(&model_path), &model_sha256).is_err()
            {
                return emit_failure(
                    output,
                    job_id,
                    generation,
                    instance_id,
                    WorkerErrorCode::ModelHashMismatch,
                    "ModelHashMismatch: live model identity could not be verified",
                );
            }
            let engine = match cached_gpu_engine(
                cached_engine,
                &model_path,
                model_sha256,
                gpu_candidate.as_ref(),
            ) {
                Ok(engine) => engine,
                Err(error) => {
                    return emit_gpu_initialization_failure(
                        output,
                        job_id,
                        generation,
                        instance_id,
                        &error,
                    );
                }
            };
            if let Some(candidate) = &gpu_candidate {
                write_frame(
                    output,
                    &IpcEnvelope::new(WorkerEvent::GpuSessionAttested {
                        job_id,
                        generation,
                        instance_id,
                        candidate: candidate.pnp_instance_id.clone(),
                        dxgi_rank: candidate.dxgi_rank,
                        luid: candidate.luid,
                        pci_id: candidate.pci_id.clone(),
                        cuda_ordinal: candidate.cuda_ordinal,
                        ggml_ordinal: candidate.ggml_ordinal,
                    }),
                )
                .map_err(|e| e.to_string())?;
            }
            write_frame(
                output,
                &IpcEnvelope::new(WorkerEvent::Ready {
                    job_id,
                    generation,
                    instance_id,
                    backend: BackendKind::Cuda,
                }),
            )
            .map_err(|e| e.to_string())?;
            return run_live_service(
                rx,
                output,
                engine,
                job_id,
                generation,
                instance_id,
                config,
                !direct_live,
            );
        }
        WorkerCommand::StartImport {
            job_id,
            generation,
            instance_id,
            source_path,
            source_sha256,
            expected_source_samples,
            model_path,
            model_sha256,
            config,
            ..
        } => ImportRequest {
            job_id,
            generation,
            instance_id,
            source_path,
            source_sha256,
            expected_source_samples,
            model_path,
            model_sha256,
            config,
            gpu_candidate,
        },
        WorkerCommand::StartGpuImport {
            job_id,
            generation,
            instance_id,
            source_path,
            source_sha256,
            expected_source_samples,
            model_path,
            model_sha256,
            config,
            ..
        } => ImportRequest {
            job_id,
            generation,
            instance_id,
            source_path,
            source_sha256,
            expected_source_samples,
            model_path,
            model_sha256,
            config,
            gpu_candidate,
        },
        _ => {
            return Err(
                "first IPC command must be StartImport, StartLiveWorker or DecodeBlock".into(),
            );
        }
    };
    import::run_import(rx, output, cached_engine, request)
}
