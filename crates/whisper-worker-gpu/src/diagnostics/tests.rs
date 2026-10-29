use super::*;
use crate::{integrity::verify_sha256, ipc::read_frame};
use sha2::{Digest, Sha256};

#[test]
fn verified_model_gpu_initialization_failure_is_retryable_and_correlated() {
    let root = std::env::temp_dir().join(format!(
        "whisper-gpu-init-classification-{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let model = root.join("model.bin");
    let bytes = b"verified model fixture";
    std::fs::write(&model, bytes).unwrap();
    let digest: [u8; 32] = Sha256::digest(bytes).into();
    verify_sha256(&model, &digest).unwrap();

    let (job_id, generation, instance_id) = (
        whisper_core::JobId(901),
        whisper_core::Generation::first(),
        73,
    );
    let mut frame = Vec::new();
    emit_gpu_initialization_failure(
        &mut frame,
        job_id,
        generation,
        instance_id,
        "injected CUDA initialization failure",
    )
    .unwrap();
    let event = read_frame::<IpcEnvelope<WorkerEvent>>(&mut std::io::Cursor::new(frame))
        .unwrap()
        .unwrap()
        .message;
    assert!(matches!(
        event,
        WorkerEvent::Failed {
            job_id: Some(actual_job),
            generation: Some(actual_generation),
            instance_id: Some(actual_instance),
            code: WorkerErrorCode::BackendUnavailable,
            ..
        } if (actual_job, actual_generation, actual_instance)
            == (job_id, generation, instance_id)
    ));

    let bad_digest = [0; 32];
    assert!(verify_sha256(&model, &bad_digest).is_err());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn gpu_revalidation_failure_is_typed_and_bound_to_the_startup_attempt() {
    let mut output = Vec::new();
    let generation = whisper_core::Generation::first();
    emit_gpu_revalidation_failure(
        &mut output,
        whisper_core::JobId(91),
        generation,
        17,
        "LUID changed",
    )
    .unwrap();
    let event: IpcEnvelope<WorkerEvent> = serde_json::from_slice(&output).unwrap();
    assert!(matches!(
        event.message,
        WorkerEvent::Failed {
            job_id: Some(whisper_core::JobId(91)),
            generation: Some(actual_generation),
            instance_id: Some(17),
            code: WorkerErrorCode::BackendMismatch,
            ..
        } if actual_generation == generation
    ));
}
