#[cfg(feature = "l01-memory-qualification")]
use crate::memory_qualification;
use crate::{
    decoder,
    diagnostics::emit_failure,
    encoder,
    engine_cache::cached_cpu_engine,
    integrity::verify_sha256,
    ipc::Incoming,
    ipc_output::write_frame,
    native_engine::CpuEngine,
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
}

pub(crate) fn run_session(
    rx: &Receiver<Incoming>,
    output: &mut impl io::Write,
    first: IpcEnvelope<WorkerCommand>,
    cached_engine: &mut Option<(String, [u8; 32], CpuEngine)>,
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
    let direct_live = matches!(&first.message, WorkerCommand::StartDirectLiveWorker { .. });
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
            if config.compute != ComputeChoice::Cpu
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
            let engine = match cached_cpu_engine(cached_engine, &model_path, model_sha256) {
                Ok(engine) => engine,
                Err(error) => {
                    return emit_failure(
                        output,
                        job_id,
                        generation,
                        instance_id,
                        WorkerErrorCode::ModelUnavailable,
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
                    backend: BackendKind::Cpu,
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
        },
        _ => {
            return Err(
                "first IPC command must be StartImport, StartLiveWorker or DecodeBlock".into(),
            );
        }
    };
    import::run_import(rx, output, cached_engine, request)
}
