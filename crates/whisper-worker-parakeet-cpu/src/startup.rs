use crate::{
    args::parse_args,
    decode_only::decode_only_mode,
    gpu_identity::{build_gpu_evidence, configured_backend, worker_gpu_identity},
    integrity::{constant_time_eq, runtime_sha256, sha256_file},
    ipc::write_event,
    native_engine, record_r7_timing,
    runtime_paths::{ensure_inside, required_runtime_root, shared_model_runtime_allowed},
    session,
};
use std::{env, path::Path, time::Instant};
use whisper_core::parakeet_ipc::{Event, WorkerIdentity};

pub(crate) fn run() -> Result<(), String> {
    if let Some(args) = env::args_os().nth(1)
        && args == "--decode-only"
    {
        return decode_only_mode();
    }
    let args = parse_args()?;
    let runtime = required_runtime_root(&args.profile)?;
    let native_dir = args
        .native_dir
        .canonicalize()
        .map_err(|e| format!("NativeDirectory: {e}"))?;
    ensure_inside(&native_dir, &runtime)?;
    let model = args
        .model
        .canonicalize()
        .map_err(|e| format!("Model: {e}"))?;
    if !model.is_file() {
        return Err("Model: path is not a file".into());
    }
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
    let shared_model = repository
        .join("models/parakeet-tdt-0.6b-v3.q8_0.gguf")
        .canonicalize()
        .ok()
        .is_some_and(|expected| model == expected)
        && shared_model_runtime_allowed(&runtime, &repository);
    if !model.starts_with(&runtime) && !shared_model {
        return Err(
            "PathOutsideRuntime: model must be under runtime or the read-only L2b models directory"
                .into(),
        );
    }
    let model_sha = sha256_file(&model)?;
    if !constant_time_eq(&args.model_sha256, &model_sha) {
        return Err("ModelHashMismatch: model SHA-256 differs from the configured identity".into());
    }

    let gpu_identity = worker_gpu_identity(configured_backend())?;
    let engine_started = Instant::now();
    let engine = native_engine::Engine::load(&native_dir, &model)?;
    record_r7_timing(
        "engine_load_model_and_dll",
        engine_started.elapsed(),
        serde_json::json!({}),
    );
    let gpu_evidence = build_gpu_evidence(&runtime, engine.backend(), gpu_identity.as_ref())?;
    let executable = env::current_exe().map_err(|e| format!("WorkerPath: {e}"))?;
    let worker_sha256 = sha256_file(&executable)?;
    let identity = WorkerIdentity {
        model_sha256: model_sha,
        runtime_sha256: runtime_sha256(&native_dir, engine.native_version())?,
        build_sha256: worker_sha256,
        profile: args.profile.clone(),
    };
    write_event(&Event::Ready {
        identity: identity.clone(),
        backend: engine.backend().into(),
        native_version: engine.native_version().to_owned(),
        gpu_evidence,
    })?;

    session::run(engine, runtime, identity, gpu_identity)
}
