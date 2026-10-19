mod decoder;
mod native_engine;
mod native_ffi;

#[cfg(feature = "parakeet-qualification-reference")]
use std::fs;
use std::{
    collections::BTreeMap,
    env,
    fs::File,
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use sha2::{Digest, Sha256};
use whisper_core::parakeet_ipc::{
    Command, Envelope, Event, ExpectedGpu, GpuEvidence, PARAKEET_IPC_VERSION,
    PARAKEET_MAX_FRAME_BYTES, PARAKEET_PROFILE_IMPORT_SINGLE_PASS, WorkerIdentity,
};
use whisper_core::parakeet_windows::{PROFILE_IMPORT_VAD, PROFILE_QUALIFICATION_REFERENCE};

fn main() {
    if let Err(error) = run() {
        eprintln!("parakeet-worker fatal: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
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
    let mut engine = native_engine::Engine::load(&native_dir, &model)?;
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

fn decode_only_mode() -> Result<(), String> {
    let mut args = env::args_os().skip(2);
    let source = args
        .next()
        .map(PathBuf::from)
        .ok_or("DecodeOnly: missing source path")?;
    let output = args
        .next()
        .map(PathBuf::from)
        .ok_or("DecodeOnly: missing output path")?;
    if args.next().is_some() {
        return Err("DecodeOnly: unexpected argument".into());
    }
    let runtime = required_runtime_root(PARAKEET_PROFILE_IMPORT_SINGLE_PASS)?;
    let output_parent = output
        .parent()
        .ok_or("DecodeOnly: output has no parent")?
        .canonicalize()
        .map_err(|e| format!("DecodeOnlyOutput: {e}"))?;
    ensure_inside(&output_parent, &runtime)?;
    let decoded = decoder::decode_file(&source)?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)
        .map_err(|e| format!("DecodeOnlyOutput: {e}"))?;
    for sample in &decoded.samples {
        file.write_all(&sample.to_le_bytes())
            .map_err(|e| e.to_string())?;
    }
    file.sync_all().map_err(|e| e.to_string())?;
    println!(
        "{}",
        serde_json::json!({
            "source_sha256": sha256_file(&source)?, "output_samples": decoded.samples.len(),
            "source_frames": decoded.source_frames, "source_rate": decoded.source_rate,
            "channels": decoded.channels, "codec": decoded.codec,
        })
    );
    Ok(())
}

fn transcribe_one(
    engine: &mut native_engine::Engine,
    source: &Path,
    expected_source_sha: &str,
    profile: &str,
    on_native_call_started: impl FnOnce() -> Result<(), String>,
) -> Result<
    (
        u64,
        String,
        Vec<whisper_core::parakeet_ipc::Word>,
        String,
        u64,
    ),
    (&'static str, String),
> {
    let canonical = source
        .canonicalize()
        .map_err(|e| ("SourceUnavailable", e.to_string()))?;
    if !canonical.is_file() {
        return Err(("SourceUnavailable", "source is not a file".into()));
    }
    let actual_sha = sha256_file(&canonical).map_err(|e| ("SourceUnavailable", e))?;
    if !constant_time_eq(expected_source_sha, &actual_sha) {
        return Err((
            "SourceHashMismatch",
            "source SHA-256 differs from request".into(),
        ));
    }
    let decode_started = Instant::now();
    let decoded = if profile == PROFILE_QUALIFICATION_REFERENCE {
        #[cfg(feature = "parakeet-qualification-reference")]
        {
            decode_reference_wav(&canonical).map_err(|error| ("DecodeFailed", error))?
        }
        #[cfg(not(feature = "parakeet-qualification-reference"))]
        {
            return Err((
                "QualificationReferenceUnavailable",
                "worker lacks the private reference feature".into(),
            ));
        }
    } else {
        decoder::decode_file(&canonical).map_err(|error| ("DecodeFailed", error))?
    };
    record_r7_timing(
        "audio_decode",
        decode_started.elapsed(),
        serde_json::json!({
            "samples": decoded.samples.len(), "source_sha256": actual_sha,
        }),
    );
    let duration_samples = decoded.samples.len() as u64;
    let result = if profile == PROFILE_QUALIFICATION_REFERENCE {
        #[cfg(feature = "parakeet-qualification-reference")]
        {
            engine.transcribe_reference_with_start_callback(&decoded, on_native_call_started)
        }
        #[cfg(not(feature = "parakeet-qualification-reference"))]
        {
            Err("QualificationReferenceUnavailable".to_owned())
        }
    } else {
        engine.transcribe_with_start_callback(&decoded, on_native_call_started)
    };
    let (text, words, native_inference_ms) = result.map_err(|error| {
        let code = match error.as_str() {
            value if value.starts_with("InvalidTimestamp") => "InvalidTimestamp",
            value if value.starts_with("NativeCallStarted:") => "IpcWrite",
            _ => "InferenceFailed",
        };
        (code, error)
    })?;
    Ok((
        duration_samples,
        text,
        words,
        actual_sha,
        native_inference_ms,
    ))
}

fn worker_gpu_identity(backend: &str) -> Result<Option<ExpectedGpu>, String> {
    #[cfg(feature = "parakeet-cuda")]
    {
        if backend != "CUDA0" {
            return Err(format!("BackendMismatch: CUDA feature reported {backend}"));
        }
        return query_single_nvidia_gpu().map(Some);
    }
    #[cfg(not(feature = "parakeet-cuda"))]
    {
        if backend != "CPU" {
            return Err(format!("BackendMismatch: CPU worker reported {backend}"));
        }
        Ok(None)
    }
}

fn configured_backend() -> &'static str {
    #[cfg(feature = "parakeet-cuda")]
    {
        "CUDA0"
    }
    #[cfg(not(feature = "parakeet-cuda"))]
    {
        "CPU"
    }
}

fn shared_model_runtime_allowed(runtime: &Path, repository: &Path) -> bool {
    let l2b = repository
        .join("target/parakeet-tests/l2/l2b")
        .canonicalize()
        .ok();
    if l2b.as_deref() == Some(runtime) {
        return true;
    }
    #[cfg(feature = "parakeet-cuda")]
    {
        return ["l3a", "l3b"].into_iter().any(|lot| {
            repository
                .join("target/parakeet-tests/l3")
                .join(lot)
                .canonicalize()
                .ok()
                .as_deref()
                == Some(runtime)
        });
    }
    #[cfg(not(feature = "parakeet-cuda"))]
    false
}

fn build_gpu_evidence(
    runtime: &Path,
    backend: &str,
    gpu: Option<&ExpectedGpu>,
) -> Result<Option<GpuEvidence>, String> {
    #[cfg(feature = "parakeet-cuda")]
    {
        let gpu = gpu.ok_or("GpuEvidenceMissing: CUDA device identity is unavailable")?;
        if backend != "CUDA0" {
            return Err(format!("BackendMismatch: expected CUDA0, got {backend}"));
        }
        let log_path = std::env::var_os("PARAKEET_L3_ASR_LOG")
            .map(PathBuf::from)
            .ok_or("GpuLogMissing: PARAKEET_L3_ASR_LOG is required")?;
        let parent = log_path
            .parent()
            .ok_or("GpuLogPath: log path has no parent")?
            .canonicalize()
            .map_err(|error| format!("GpuLogPath: {error}"))?;
        if !parent.starts_with(runtime) {
            return Err("GpuLogPath: captured log is outside the runtime root".into());
        }
        let canonical_log = parent.join(
            log_path
                .file_name()
                .ok_or("GpuLogPath: log path has no file name")?,
        );
        let bytes =
            std::fs::read(&canonical_log).map_err(|error| format!("GpuLogRead: {error}"))?;
        let has_cuda_status = bytes.split(|byte| *byte == b'\n').any(|line| {
            std::str::from_utf8(line)
                .ok()
                .is_some_and(|line| line.starts_with("[asr]") && line.contains("backend=CUDA0"))
        });
        if !has_cuda_status {
            return Err("GpuLogMissing: captured log lacks [asr] backend=CUDA0".into());
        }
        let asr_log_sha256 = format!("{:x}", Sha256::digest(&bytes));
        return Ok(Some(GpuEvidence {
            backend: backend.to_owned(),
            pci_bus_id: gpu.pci_bus_id.clone(),
            uuid: gpu.uuid.clone(),
            pid: std::process::id(),
            asr_log_sha256,
        }));
    }
    #[cfg(not(feature = "parakeet-cuda"))]
    {
        let _ = (runtime, backend, gpu);
        Ok(None)
    }
}

fn validate_expected_gpu(
    backend: &str,
    expected: Option<&ExpectedGpu>,
    startup_identity: Option<&ExpectedGpu>,
) -> Result<(), (&'static str, String)> {
    #[cfg(feature = "parakeet-cuda")]
    {
        if backend != "CUDA0" {
            return Err(("BackendMismatch", format!("expected CUDA0, got {backend}")));
        }
        let expected = expected.ok_or((
            "GpuIdentityRequired",
            "CUDA request must include expected PCI bus ID and UUID".into(),
        ))?;
        let actual = query_single_nvidia_gpu().map_err(|error| ("GpuInventoryFailed", error))?;
        if startup_identity != Some(&actual)
            || expected.pci_bus_id != actual.pci_bus_id
            || !expected.uuid.eq_ignore_ascii_case(&actual.uuid)
        {
            return Err((
                "WrongGpuDevice",
                format!(
                    "expected {} / {}, current device is {} / {}",
                    expected.pci_bus_id, expected.uuid, actual.pci_bus_id, actual.uuid
                ),
            ));
        }
        Ok(())
    }
    #[cfg(not(feature = "parakeet-cuda"))]
    {
        let _ = startup_identity;
        if backend != "CPU" || expected.is_some() {
            return Err((
                "GpuUnavailable",
                "CPU worker cannot accept a GPU identity".into(),
            ));
        }
        Ok(())
    }
}

#[cfg(feature = "parakeet-cuda")]
fn query_single_nvidia_gpu() -> Result<ExpectedGpu, String> {
    let output = std::process::Command::new("nvidia-smi")
        .args(["--query-gpu=count,name,pci.bus_id,uuid", "--format=csv"])
        .output()
        .map_err(|error| format!("GpuInventory: unable to start nvidia-smi: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "GpuInventory: nvidia-smi exited with {}",
            output.status
        ));
    }
    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| format!("GpuInventory: nvidia-smi output is not UTF-8: {error}"))?;
    parse_single_nvidia_gpu(&stdout)
}

#[cfg(feature = "parakeet-cuda")]
fn parse_single_nvidia_gpu(stdout: &str) -> Result<ExpectedGpu, String> {
    let rows = stdout
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.trim_start().starts_with("count,"))
        .collect::<Vec<_>>();
    if rows.len() != 1 {
        return Err(format!(
            "GpuInventory: expected one NVIDIA GPU row, found {}",
            rows.len()
        ));
    }
    let fields = rows[0].split(',').map(str::trim).collect::<Vec<_>>();
    if fields.len() != 4 || fields[0] != "1" || fields[2].is_empty() || fields[3].is_empty() {
        return Err(format!(
            "GpuInventory: malformed or non-unique NVIDIA GPU row: {}",
            rows[0]
        ));
    }
    Ok(ExpectedGpu {
        pci_bus_id: fields[2].to_owned(),
        uuid: fields[3].to_owned(),
    })
}

fn parse_args() -> Result<Args, String> {
    let mut model = None;
    let mut model_sha256 = None;
    let mut native_dir = None;
    #[allow(unused_mut)]
    let mut profile = PARAKEET_PROFILE_IMPORT_SINGLE_PASS.to_owned();
    #[allow(unused_mut)]
    let mut profile_was_set = false;
    let mut args = env::args_os().skip(1);
    while let Some(arg) = args.next() {
        match arg.to_string_lossy().as_ref() {
            "--qualification-reference" => {
                #[cfg(feature = "parakeet-qualification-reference")]
                {
                    if profile_was_set {
                        return Err("InvalidProfile: profile flags cannot be combined".into());
                    }
                    profile = PROFILE_QUALIFICATION_REFERENCE.to_owned();
                    profile_was_set = true;
                    continue;
                }
                #[cfg(not(feature = "parakeet-qualification-reference"))]
                return Err("QualificationReferenceUnavailable".into());
            }
            _ => {
                let value = args
                    .next()
                    .ok_or_else(|| format!("missing value for {}", arg.to_string_lossy()))?;
                match arg.to_string_lossy().as_ref() {
                    "--model" => model = Some(PathBuf::from(value)),
                    "--model-sha256" => {
                        model_sha256 = Some(value.to_string_lossy().to_ascii_lowercase())
                    }
                    "--native-dir" => native_dir = Some(PathBuf::from(value)),
                    "--profile" => {
                        if profile_was_set {
                            return Err("InvalidProfile: profile may be specified only once".into());
                        }
                        let requested = value.to_string_lossy().to_string();
                        if requested == PROFILE_IMPORT_VAD {
                            #[cfg(feature = "parakeet-qualification-reference")]
                            {
                                profile = requested;
                                profile_was_set = true;
                            }
                            #[cfg(not(feature = "parakeet-qualification-reference"))]
                            return Err("ProfileUnavailable: import-vad".into());
                        } else {
                            return Err(format!("InvalidProfile: {requested}"));
                        }
                    }
                    _ => return Err(format!("unknown argument {}", arg.to_string_lossy())),
                }
            }
        }
    }
    let model_sha256 = model_sha256.ok_or("missing --model-sha256")?;
    if model_sha256.len() != 64 || !model_sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("invalid --model-sha256".into());
    }
    Ok(Args {
        model: model.ok_or("missing --model")?,
        model_sha256,
        native_dir: native_dir.ok_or("missing --native-dir")?,
        profile,
    })
}

struct Args {
    model: PathBuf,
    model_sha256: String,
    native_dir: PathBuf,
    profile: String,
}

fn required_runtime_root(profile: &str) -> Result<PathBuf, String> {
    let root = env::var_os("PARAKEET_L2_RUNTIME_ROOT")
        .ok_or("MissingRuntime: PARAKEET_L2_RUNTIME_ROOT")?;
    let root = PathBuf::from(root)
        .canonicalize()
        .map_err(|e| format!("RuntimeRoot: {e}"))?;
    #[cfg(feature = "parakeet-personal")]
    {
        if !personal_runtime_root_is_allowed(&root) {
            return Err(
                "RuntimeRootMismatch: worker runtime is outside the personal installation".into(),
            );
        }
        if profile != PARAKEET_PROFILE_IMPORT_SINGLE_PASS {
            return Err(
                "ProfileUnavailable: personal workers support single-pass import only".into(),
            );
        }
    }
    #[cfg(not(feature = "parakeet-personal"))]
    {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
        let allowed = allowed_runtime_roots(&repository);
        if !allowed.iter().any(|candidate| root == *candidate) {
            return Err(
                "RuntimeRootMismatch: worker runtime is outside authorized Test roots".into(),
            );
        }
        if profile != PARAKEET_PROFILE_IMPORT_SINGLE_PASS
            && !qualification_root_is_valid(profile, &root, &repository)
        {
            return Err(
                "RuntimeRootMismatch: qualification profiles require the L2b runtime root".into(),
            );
        }
    }
    Ok(root)
}

#[cfg(feature = "parakeet-personal")]
fn personal_runtime_root_is_allowed(root: &Path) -> bool {
    let Some(local_app_data) = env::var_os("LOCALAPPDATA").map(PathBuf::from) else {
        return false;
    };
    let personal_root = local_app_data.join("Whisper");
    let Ok(personal_root) = personal_root.canonicalize() else {
        return false;
    };
    personal_runtime_path_is_allowed(root, &personal_root)
}

#[cfg(feature = "parakeet-personal")]
fn personal_runtime_path_is_allowed(root: &Path, personal_root: &Path) -> bool {
    root == personal_root
        || (root.parent() == Some(personal_root.join(".staging").as_path())
            && root.file_name().is_some_and(|name| !name.is_empty()))
}

#[cfg(test)]
mod runtime_root_tests {
    use super::*;

    #[cfg(not(feature = "parakeet-cuda"))]
    #[test]
    fn accepts_only_the_two_canonical_l2_test_roots() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
        let l2a_roots = allowed_runtime_roots_for(&repository, &["l2a-1", "l2a-2"]);
        let l2a1 = repository
            .join("target/parakeet-tests/l2/l2a-1")
            .canonicalize()
            .unwrap();
        let l2a2 = repository
            .join("target/parakeet-tests/l2/l2a-2")
            .canonicalize()
            .unwrap();
        let l2b = repository
            .join("target/parakeet-tests/l2/l2b")
            .canonicalize()
            .unwrap();
        let l3a = repository
            .join("target/parakeet-tests/l3/l3a")
            .canonicalize()
            .unwrap();
        assert_eq!(l2a_roots, vec![l2a1.clone(), l2a2.clone()]);
        assert!(allowed_runtime_roots(&repository).contains(&l2a1));
        assert!(allowed_runtime_roots(&repository).contains(&l2a2));
        assert!(!allowed_runtime_roots(&repository).contains(&l2b));
        assert!(!allowed_runtime_roots(&repository).contains(&l3a));
        assert!(!allowed_runtime_roots(&repository).contains(&repository));
    }

    #[cfg(all(feature = "parakeet-cuda", not(feature = "parakeet-personal")))]
    #[test]
    fn gpu_worker_allows_only_the_canonical_l3_runtime_roots() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
        let l3a = repository
            .join("target/parakeet-tests/l3/l3a")
            .canonicalize()
            .unwrap();
        let l3a_expected = repository.join("target/parakeet-tests/l3/l3a");
        let l3b_expected = repository.join("target/parakeet-tests/l3/l3b");
        let l2a1 = repository
            .join("target/parakeet-tests/l2/l2a-1")
            .canonicalize()
            .unwrap();
        let l2b = repository
            .join("target/parakeet-tests/l2/l2b")
            .canonicalize()
            .unwrap();
        assert_eq!(
            allowed_runtime_root_candidates(&repository),
            vec![l3a_expected, l3b_expected]
        );
        assert!(allowed_runtime_roots(&repository).contains(&l3a));
        assert!(!allowed_runtime_roots(&repository).contains(&l2a1));
        assert!(!allowed_runtime_roots(&repository).contains(&l2b));
    }

    #[cfg(feature = "parakeet-cuda")]
    #[test]
    fn nvidia_inventory_parser_requires_exactly_one_consistent_gpu_row() {
        let gpu = parse_single_nvidia_gpu(
            "count, name, pci.bus_id, uuid\n1, NVIDIA GeForce GTX 1080 Ti, 00000000:01:00.0, GPU-test\n",
        )
        .unwrap();
        assert_eq!(gpu.pci_bus_id, "00000000:01:00.0");
        assert_eq!(gpu.uuid, "GPU-test");

        assert!(
            parse_single_nvidia_gpu(
                "count, name, pci.bus_id, uuid\n2, NVIDIA GeForce GTX, 00000000:01:00.0, GPU-test\n"
            )
            .is_err()
        );
        assert!(parse_single_nvidia_gpu(
            "count, name, pci.bus_id, uuid\n1, NVIDIA GPU, 00000000:01:00.0, GPU-a\n1, NVIDIA GPU, 00000000:02:00.0, GPU-b\n"
        )
        .is_err());
        assert!(parse_single_nvidia_gpu("not an inventory row").is_err());
    }

    #[cfg(feature = "parakeet-qualification-reference")]
    #[test]
    fn qualification_profiles_are_confined_to_l2b() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
        let l2a2 = repository
            .join("target/parakeet-tests/l2/l2a-2")
            .canonicalize()
            .unwrap();
        let l2b = repository
            .join("target/parakeet-tests/l2/l2b")
            .canonicalize()
            .unwrap();
        assert_ne!(l2a2, l2b);
        assert!(qualification_root_is_valid(
            PROFILE_QUALIFICATION_REFERENCE,
            &l2b,
            &repository
        ));
        assert!(!qualification_root_is_valid(
            PROFILE_QUALIFICATION_REFERENCE,
            &l2a2,
            &repository
        ));
        assert!(qualification_root_is_valid(
            PARAKEET_PROFILE_IMPORT_SINGLE_PASS,
            &l2a2,
            &repository
        ));
    }
}

fn qualification_root_is_valid(profile: &str, root: &Path, repository: &Path) -> bool {
    if profile == PARAKEET_PROFILE_IMPORT_SINGLE_PASS {
        return allowed_runtime_roots(repository).contains(&root.to_path_buf());
    }
    repository
        .join("target/parakeet-tests/l2/l2b")
        .canonicalize()
        .ok()
        .as_ref()
        == Some(&root.to_path_buf())
}

fn allowed_runtime_root_candidates(repository: &Path) -> Vec<PathBuf> {
    #[cfg(feature = "parakeet-personal")]
    {
        let _ = repository;
        return Vec::new();
    }
    #[cfg(all(feature = "parakeet-cuda", not(feature = "parakeet-personal")))]
    {
        return ["l3a", "l3b"]
            .into_iter()
            .map(|lot| repository.join("target/parakeet-tests/l3").join(lot))
            .collect();
    }
    #[cfg(all(not(feature = "parakeet-cuda"), not(feature = "parakeet-personal")))]
    {
        let mut roots = ["l2a-1", "l2a-2"]
            .into_iter()
            .map(|lot| repository.join("target/parakeet-tests/l2").join(lot))
            .collect::<Vec<_>>();
        #[cfg(feature = "parakeet-qualification-reference")]
        roots.push(repository.join("target/parakeet-tests/l2/l2b"));
        roots
    }
}

#[cfg(all(test, feature = "parakeet-personal"))]
mod personal_runtime_root_tests {
    use super::*;

    #[test]
    fn personal_worker_accepts_only_published_root_or_a_direct_staging_child() {
        let personal = Path::new(r"C:\Users\test\AppData\Local\Whisper");
        assert!(personal_runtime_path_is_allowed(personal, personal));
        assert!(personal_runtime_path_is_allowed(
            Path::new(r"C:\Users\test\AppData\Local\Whisper\.staging\candidate-1"),
            personal
        ));
        assert!(!personal_runtime_path_is_allowed(
            Path::new(r"C:\dev\whisper\target\parakeet-tests\l6-perso"),
            personal
        ));
        assert!(!personal_runtime_path_is_allowed(
            Path::new(r"C:\Users\test\AppData\Local\Whisper\.staging\candidate-1\nested"),
            personal
        ));
    }
}

fn allowed_runtime_roots(repository: &Path) -> Vec<PathBuf> {
    allowed_runtime_root_candidates(repository)
        .into_iter()
        .filter_map(|candidate| candidate.canonicalize().ok())
        .collect()
}

fn allowed_runtime_roots_for(repository: &Path, lots: &[&str]) -> Vec<PathBuf> {
    lots.into_iter()
        .filter_map(|lot| {
            repository
                .join("target")
                .join("parakeet-tests")
                .join("l2")
                .join(lot)
                .canonicalize()
                .ok()
        })
        .collect()
}

fn ensure_inside(path: &Path, root: &Path) -> Result<(), String> {
    if !path.starts_with(root) {
        return Err("PathOutsideRuntime: native directory outside runtime root".into());
    }
    Ok(())
}

#[cfg(feature = "parakeet-qualification-reference")]
fn decode_reference_wav(path: &Path) -> Result<decoder::DecodedAudio, String> {
    let bytes = fs::read(path).map_err(|error| format!("SourceOpen: {error}"))?;
    if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("ReferenceFormat: expected RIFF/WAVE".into());
    }
    let mut format = None;
    let mut data = None;
    let mut offset = 12_usize;
    while offset + 8 <= bytes.len() {
        let id = &bytes[offset..offset + 4];
        let size = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        let start = offset + 8;
        let end = start
            .checked_add(size)
            .filter(|end| *end <= bytes.len())
            .ok_or("ReferenceFormat: chunk exceeds WAV bounds")?;
        if id == b"fmt " {
            if size < 16 {
                return Err("ReferenceFormat: truncated format chunk".into());
            }
            format = Some((
                u16::from_le_bytes(bytes[start..start + 2].try_into().unwrap()),
                u16::from_le_bytes(bytes[start + 2..start + 4].try_into().unwrap()),
                u32::from_le_bytes(bytes[start + 4..start + 8].try_into().unwrap()),
                u16::from_le_bytes(bytes[start + 12..start + 14].try_into().unwrap()),
                u16::from_le_bytes(bytes[start + 14..start + 16].try_into().unwrap()),
            ));
        } else if id == b"data" {
            data = Some(&bytes[start..end]);
        }
        offset = end + (size & 1);
    }
    let Some((encoding, channels, rate, block_align, bits)) = format else {
        return Err("ReferenceFormat: missing format chunk".into());
    };
    if (encoding, channels, rate, block_align, bits) != (1, 1, 16_000, 2, 16) {
        return Err("ReferenceFormat: expected mono 16 kHz PCM16".into());
    }
    let data = data.ok_or("ReferenceFormat: missing data chunk")?;
    if data.len() % 2 != 0 {
        return Err("ReferenceFormat: odd PCM16 payload size".into());
    }
    let samples = data
        .chunks_exact(2)
        .map(|sample| i16::from_le_bytes([sample[0], sample[1]]) as f32 / 32_768.0)
        .collect::<Vec<_>>();
    let count = samples.len();
    if count == 0 || count as u64 > whisper_core::parakeet_windows::MAX_REFERENCE_SAMPLES {
        return Err("AudioTooLong: reference input must be between 0 and 3,600 seconds".into());
    }
    Ok(decoder::DecodedAudio {
        samples,
        source_frames: count as u64,
        source_rate: 16_000,
        channels: 1,
        codec: "PCM_S16LE".into(),
    })
}

fn read_frame(reader: &mut impl BufRead) -> Result<Option<Vec<u8>>, String> {
    let mut frame = Vec::new();
    loop {
        let available = reader.fill_buf().map_err(|e| format!("IpcRead: {e}"))?;
        if available.is_empty() {
            return if frame.is_empty() {
                Ok(None)
            } else {
                Err("IpcTruncated: EOF inside frame".into())
            };
        }
        let newline = available.iter().position(|byte| *byte == b'\n');
        let count = newline.map_or(available.len(), |index| index + 1);
        if frame.len() + count > PARAKEET_MAX_FRAME_BYTES {
            return Err("IpcFrameTooLarge: frame exceeds 1 MiB".into());
        }
        frame.extend_from_slice(&available[..count]);
        reader.consume(count);
        if newline.is_some() {
            frame.pop();
            return Ok(Some(frame));
        }
    }
}

fn write_event(event: &Event) -> Result<(), String> {
    let mut bytes =
        serde_json::to_vec(&Envelope::new(event)).map_err(|e| format!("IpcSerialize: {e}"))?;
    if bytes.len() + 1 > PARAKEET_MAX_FRAME_BYTES {
        return Err("IpcFrameTooLarge: event exceeds 1 MiB".into());
    }
    bytes.push(b'\n');
    let mut stdout = io::stdout().lock();
    stdout
        .write_all(&bytes)
        .map_err(|e| format!("IpcWrite: {e}"))?;
    stdout.flush().map_err(|e| format!("IpcWrite: {e}"))
}

fn failed(
    request_id: Option<u64>,
    job_id: Option<&str>,
    generation: Option<u64>,
    attempt: Option<u64>,
    identity: Option<WorkerIdentity>,
    code: &str,
    message: &str,
) -> Event {
    Event::Failed {
        request_id,
        job_id: job_id.map(str::to_owned),
        generation,
        attempt,
        identity,
        code: code.into(),
        message: message.into(),
    }
}

fn runtime_sha256(native_dir: &Path, native_version: &str) -> Result<String, String> {
    let mut hasher = Sha256::new();
    hasher.update(native_version.as_bytes());
    for name in [
        "nemo_speech_asr_c.dll",
        "nemo_speech_asr.dll",
        "ggml.dll",
        "ggml-base.dll",
        "ggml-cpu.dll",
    ] {
        hasher.update(name.as_bytes());
        hasher.update(sha256_file(&native_dir.join(name))?.as_bytes());
    }
    #[cfg(feature = "parakeet-cuda")]
    for name in [
        "ggml-cuda.dll",
        "cudart64_12.dll",
        "cublas64_12.dll",
        "cublasLt64_12.dll",
    ] {
        hasher.update(name.as_bytes());
        hasher.update(sha256_file(&native_dir.join(name))?.as_bytes());
    }
    Ok(format!("{:x}", hasher.finalize()))
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let started = Instant::now();
    let mut file = File::open(path).map_err(|e| format!("HashOpen: {e}"))?;
    let mut hasher = Sha256::new();
    let bytes = io::copy(&mut file, &mut hasher).map_err(|e| format!("HashRead: {e}"))?;
    let sha = format!("{:x}", hasher.finalize());
    record_r7_timing(
        "sha256",
        started.elapsed(),
        serde_json::json!({
            "file": path.file_name().and_then(|name| name.to_str()).unwrap_or("unknown"),
            "bytes": bytes,
            "sha256": sha,
        }),
    );
    Ok(sha)
}

fn record_r7_timing(stage: &str, elapsed: Duration, details: serde_json::Value) {
    if env::var_os("PARAKEET_R7_TIMING_ACTIVE").as_deref() != Some(std::ffi::OsStr::new("1")) {
        return;
    }
    let (Some(root), Some(path)) = (
        env::var_os("PARAKEET_R7_EXEC_ROOT").map(PathBuf::from),
        env::var_os("PARAKEET_R7_TIMING_LOG").map(PathBuf::from),
    ) else {
        eprintln!("R7 worker timing skipped: root or log path missing");
        return;
    };
    let (Ok(root), Some(parent)) = (root.canonicalize(), path.parent()) else {
        eprintln!("R7 worker timing skipped: root or log parent unavailable");
        return;
    };
    let Ok(parent) = parent.canonicalize() else {
        eprintln!("R7 worker timing skipped: log parent unavailable");
        return;
    };
    if !parent.starts_with(&root) {
        eprintln!("R7 worker timing skipped: log path escaped R7");
        return;
    }
    let row = serde_json::json!({
        "process": "private-worker",
        "pid": std::process::id(),
        "stage": stage,
        "elapsed_ms": elapsed.as_secs_f64() * 1000.0,
        "details": details,
    });
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        Ok(mut file) => {
            if serde_json::to_writer(&mut file, &row).is_err() || file.write_all(b"\n").is_err() {
                eprintln!("R7 worker timing write failed");
            }
        }
        Err(error) => eprintln!("R7 worker timing open failed: {error}"),
    }
}

fn constant_time_eq(left: &str, right: &str) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.bytes()
        .zip(right.bytes())
        .fold(0_u8, |diff, (a, b)| diff | (a ^ b))
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn oversized_and_truncated_frames_are_rejected_before_json_handling() {
        let mut oversized = Cursor::new(vec![b'x'; PARAKEET_MAX_FRAME_BYTES + 1]);
        assert!(
            read_frame(&mut oversized)
                .unwrap_err()
                .contains("IpcFrameTooLarge")
        );
        let mut truncated = Cursor::new(b"{\"version\":1".to_vec());
        assert!(
            read_frame(&mut truncated)
                .unwrap_err()
                .contains("IpcTruncated")
        );
    }

    #[cfg(feature = "parakeet-qualification-reference")]
    #[test]
    fn reference_wav_decoder_matches_product_decoder_and_reads_sixty_minutes() {
        let short = PathBuf::from(
            env::var_os("PARAKEET_L2_FR_WAV").expect("PARAKEET_L2_FR_WAV is required"),
        );
        let product = decoder::decode_file(&short).unwrap();
        let reference = decode_reference_wav(&short).unwrap();
        assert_eq!(reference.samples, product.samples);

        let long = PathBuf::from(
            env::var_os("PARAKEET_L2_EN_LONG_WAV").expect("PARAKEET_L2_EN_LONG_WAV is required"),
        );
        let decoded = decode_reference_wav(&long).unwrap();
        assert_eq!(decoded.samples.len(), 3_600 * 16_000);
    }

    #[cfg(feature = "parakeet-qualification-reference")]
    #[test]
    fn reference_wav_decoder_rejects_audio_over_sixty_minutes() {
        let runtime = PathBuf::from(
            env::var_os("PARAKEET_L2_RUNTIME_ROOT").expect("PARAKEET_L2_RUNTIME_ROOT is required"),
        );
        let path = runtime.join(format!("reference-over-limit-{}.wav", std::process::id()));
        let data_bytes = (whisper_core::parakeet_windows::MAX_REFERENCE_SAMPLES + 1) * 2;
        let mut header = Vec::with_capacity(44);
        header.extend_from_slice(b"RIFF");
        header.extend_from_slice(&((36_u64 + data_bytes) as u32).to_le_bytes());
        header.extend_from_slice(b"WAVEfmt ");
        header.extend_from_slice(&16_u32.to_le_bytes());
        header.extend_from_slice(&1_u16.to_le_bytes());
        header.extend_from_slice(&1_u16.to_le_bytes());
        header.extend_from_slice(&16_000_u32.to_le_bytes());
        header.extend_from_slice(&32_000_u32.to_le_bytes());
        header.extend_from_slice(&2_u16.to_le_bytes());
        header.extend_from_slice(&16_u16.to_le_bytes());
        header.extend_from_slice(b"data");
        header.extend_from_slice(&(data_bytes as u32).to_le_bytes());
        assert_eq!(header.len(), 44);
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .expect("create isolated over-limit fixture");
        file.write_all(&header).unwrap();
        file.set_len(44 + data_bytes).unwrap();
        drop(file);

        let error = decode_reference_wav(&path).unwrap_err();
        assert!(error.contains("AudioTooLong"), "{error}");
        fs::remove_file(path).unwrap();
    }
}
