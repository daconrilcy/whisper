use crate::ipc::write_frame;
use std::io;
use whisper_core::ipc::{IpcEnvelope, WorkerErrorCode, WorkerEvent};

pub(crate) fn emit_failure(
    output: &mut impl io::Write,
    job_id: whisper_core::JobId,
    generation: whisper_core::Generation,
    instance_id: u64,
    code: WorkerErrorCode,
    message: &str,
) -> Result<(), String> {
    write_frame(
        output,
        &IpcEnvelope::new(WorkerEvent::Failed {
            job_id: Some(job_id),
            generation: Some(generation),
            instance_id: Some(instance_id),
            code,
            message: message.into(),
        }),
    )
    .map_err(|e| e.to_string())
}

pub(crate) fn emit_gpu_initialization_failure(
    output: &mut impl io::Write,
    job_id: whisper_core::JobId,
    generation: whisper_core::Generation,
    instance_id: u64,
    message: &str,
) -> Result<(), String> {
    emit_failure(
        output,
        job_id,
        generation,
        instance_id,
        WorkerErrorCode::BackendUnavailable,
        &format!("BackendUnavailable: CUDA initialization failed: {message}"),
    )
}

pub(crate) fn emit_gpu_revalidation_failure(
    output: &mut impl io::Write,
    job_id: whisper_core::JobId,
    generation: whisper_core::Generation,
    instance_id: u64,
    error: &str,
) -> Result<(), String> {
    emit_failure(
        output,
        job_id,
        generation,
        instance_id,
        WorkerErrorCode::BackendMismatch,
        &format!("BackendMismatch: selected GPU identity changed before admission: {error}"),
    )
}

#[cfg(test)]
mod tests;
