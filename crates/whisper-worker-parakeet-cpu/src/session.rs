use crate::{
    gpu_identity::{build_gpu_evidence, validate_expected_gpu},
    integrity::constant_time_eq,
    ipc::{failed, read_frame, write_event},
    native_engine,
    transcription::transcribe_one,
};
use std::{collections::BTreeMap, io, path::PathBuf};
use whisper_core::parakeet_ipc::{
    Command, Envelope, Event, ExpectedGpu, PARAKEET_IPC_VERSION, WorkerIdentity,
};

pub(crate) fn run(
    mut engine: native_engine::Engine,
    runtime: PathBuf,
    identity: WorkerIdentity,
    gpu_identity: Option<ExpectedGpu>,
) -> Result<(), String> {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let mut last_request_id = 0_u64;
    let mut job_attempts = BTreeMap::<String, (u64, u64)>::new();
    loop {
        let Some(frame) = read_frame(&mut input)? else {
            break;
        };
        let envelope: Envelope<Command> = match serde_json::from_slice(&frame) {
            Ok(value) => value,
            Err(error) => {
                write_event(&failed(
                    None,
                    None,
                    None,
                    None,
                    None,
                    "InvalidMessage",
                    &error.to_string(),
                ))?;
                continue;
            }
        };
        if envelope.version != PARAKEET_IPC_VERSION {
            write_event(&failed(
                None,
                None,
                None,
                None,
                None,
                "ProtocolVersionMismatch",
                "unsupported protocol version",
            ))?;
            continue;
        }
        match envelope.message {
            Command::Shutdown => {
                write_event(&Event::Stopped)?;
                break;
            }
            Command::Transcribe {
                request_id,
                job_id,
                generation,
                attempt,
                identity: request_identity,
                source_path,
                source_sha256,
                expected_gpu,
            } => {
                if request_id == 0 || request_id <= last_request_id {
                    write_event(&failed(
                        Some(request_id),
                        Some(&job_id),
                        Some(generation),
                        Some(attempt),
                        Some(identity.clone()),
                        "StaleRequest",
                        "request id must increase monotonically",
                    ))?;
                    continue;
                }
                last_request_id = request_id;
                let (error, message) =
                    if !constant_time_eq(&request_identity.model_sha256, &identity.model_sha256) {
                        (
                            Some("ModelHashMismatch"),
                            "request model identity differs from worker",
                        )
                    } else if !constant_time_eq(
                        &request_identity.runtime_sha256,
                        &identity.runtime_sha256,
                    ) {
                        (
                            Some("RuntimeIdentityMismatch"),
                            "request native runtime differs from worker",
                        )
                    } else if !constant_time_eq(
                        &request_identity.build_sha256,
                        &identity.build_sha256,
                    ) {
                        (
                            Some("BuildIdentityMismatch"),
                            "request worker build differs from worker",
                        )
                    } else if request_identity.profile != identity.profile {
                        (
                            Some("ProfileMismatch"),
                            "request profile is not supported by this worker",
                        )
                    } else {
                        (None, "")
                    };
                if let Some(code) = error {
                    write_event(&failed(
                        Some(request_id),
                        Some(&job_id),
                        Some(generation),
                        Some(attempt),
                        Some(identity.clone()),
                        code,
                        message,
                    ))?;
                    continue;
                }
                if generation == 0 || attempt == 0 {
                    write_event(&failed(
                        Some(request_id),
                        Some(&job_id),
                        Some(generation),
                        Some(attempt),
                        Some(identity.clone()),
                        "InvalidRequestIdentity",
                        "generation and attempt must be positive",
                    ))?;
                    continue;
                }
                if let Err((code, message)) = validate_expected_gpu(
                    engine.backend(),
                    expected_gpu.as_ref(),
                    gpu_identity.as_ref(),
                ) {
                    write_event(&failed(
                        Some(request_id),
                        Some(&job_id),
                        Some(generation),
                        Some(attempt),
                        Some(identity.clone()),
                        code,
                        &message,
                    ))?;
                    continue;
                }
                if let Some((previous_generation, previous_attempt)) = job_attempts.get(&job_id) {
                    let stale_generation = generation < *previous_generation;
                    let stale_attempt =
                        generation == *previous_generation && attempt <= *previous_attempt;
                    if stale_generation || stale_attempt {
                        let (code, message) = if stale_generation {
                            (
                                "StaleGeneration",
                                "request generation is older than the latest accepted generation",
                            )
                        } else {
                            (
                                "StaleAttempt",
                                "request attempt is not newer than the latest accepted attempt",
                            )
                        };
                        write_event(&failed(
                            Some(request_id),
                            Some(&job_id),
                            Some(generation),
                            Some(attempt),
                            Some(identity.clone()),
                            code,
                            message,
                        ))?;
                        continue;
                    }
                }
                job_attempts.insert(job_id.clone(), (generation, attempt));
                let source = PathBuf::from(source_path);
                let is_gpu_worker = engine.backend() == "CUDA0";
                let result = transcribe_one(
                    &mut engine,
                    &source,
                    &source_sha256,
                    &identity.profile,
                    || {
                        if is_gpu_worker {
                            write_event(&Event::NativeCallStarted {
                                request_id,
                                job_id: job_id.clone(),
                                generation,
                                attempt,
                                pid: std::process::id(),
                            })?;
                        }
                        Ok(())
                    },
                );
                match result {
                    Ok((duration_samples, text, words, actual_sha, native_inference_ms)) => {
                        let gpu_evidence =
                            build_gpu_evidence(&runtime, engine.backend(), gpu_identity.as_ref())?;
                        write_event(&Event::Result {
                            request_id,
                            job_id,
                            generation,
                            attempt,
                            identity: identity.clone(),
                            source_sha256: actual_sha,
                            duration_samples,
                            text,
                            words,
                            native_inference_ms,
                            gpu_evidence,
                        })?
                    }
                    Err((code, message)) => write_event(&failed(
                        Some(request_id),
                        Some(&job_id),
                        Some(generation),
                        Some(attempt),
                        Some(identity.clone()),
                        code,
                        &message,
                    ))?,
                }
            }
        }
    }
    Ok(())
}
