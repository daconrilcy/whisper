mod decoder;
mod native_engine;
mod native_ffi;

use std::{
    collections::BTreeMap,
    env,
    fs::File,
    io::{self, BufRead, Write},
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};
use whisper_core::parakeet_ipc::{
    Command, Envelope, Event, PARAKEET_IPC_VERSION, PARAKEET_MAX_FRAME_BYTES,
    PARAKEET_PROFILE_IMPORT_SINGLE_PASS, WorkerIdentity,
};

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
    let runtime = required_runtime_root()?;
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
    let model_sha = sha256_file(&model)?;
    if !constant_time_eq(&args.model_sha256, &model_sha) {
        return Err("ModelHashMismatch: model SHA-256 differs from the configured identity".into());
    }

    let mut engine = native_engine::Engine::load(&native_dir, &model)?;
    let executable = env::current_exe().map_err(|e| format!("WorkerPath: {e}"))?;
    let worker_sha256 = sha256_file(&executable)?;
    let identity = WorkerIdentity {
        model_sha256: model_sha,
        runtime_sha256: runtime_sha256(&native_dir, engine.native_version())?,
        build_sha256: worker_sha256,
        profile: PARAKEET_PROFILE_IMPORT_SINGLE_PASS.into(),
    };
    write_event(&Event::Ready {
        identity: identity.clone(),
        backend: "CPU".into(),
        native_version: engine.native_version().to_owned(),
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
                let result = transcribe_one(&mut engine, &source, &source_sha256);
                match result {
                    Ok((duration_samples, text, words, actual_sha)) => {
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
    let runtime = required_runtime_root()?;
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
) -> Result<(u64, String, Vec<whisper_core::parakeet_ipc::Word>, String), (&'static str, String)> {
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
    let decoded = decoder::decode_file(&canonical).map_err(|error| ("DecodeFailed", error))?;
    let duration_samples = decoded.samples.len() as u64;
    let (text, words) = engine.transcribe(&decoded).map_err(|error| {
        let code = if error.starts_with("InvalidTimestamp") {
            "InvalidTimestamp"
        } else {
            "InferenceFailed"
        };
        (code, error)
    })?;
    Ok((duration_samples, text, words, actual_sha))
}

fn parse_args() -> Result<Args, String> {
    let mut model = None;
    let mut model_sha256 = None;
    let mut native_dir = None;
    let mut args = env::args_os().skip(1);
    while let Some(arg) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {}", arg.to_string_lossy()))?;
        match arg.to_string_lossy().as_ref() {
            "--model" => model = Some(PathBuf::from(value)),
            "--model-sha256" => model_sha256 = Some(value.to_string_lossy().to_ascii_lowercase()),
            "--native-dir" => native_dir = Some(PathBuf::from(value)),
            _ => return Err(format!("unknown argument {}", arg.to_string_lossy())),
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
    })
}

struct Args {
    model: PathBuf,
    model_sha256: String,
    native_dir: PathBuf,
}

fn required_runtime_root() -> Result<PathBuf, String> {
    let root = env::var_os("PARAKEET_L2_RUNTIME_ROOT")
        .ok_or("MissingRuntime: PARAKEET_L2_RUNTIME_ROOT")?;
    let root = PathBuf::from(root)
        .canonicalize()
        .map_err(|e| format!("RuntimeRoot: {e}"))?;
    let allowed = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("target")
        .join("parakeet-tests")
        .join("l2")
        .join("l2a-1")
        .canonicalize()
        .map_err(|e| format!("AllowedRuntime: {e}"))?;
    if root != allowed {
        return Err(
            "RuntimeRootMismatch: worker runtime is outside the authorized L2a-1 root".into(),
        );
    }
    Ok(root)
}

fn ensure_inside(path: &Path, root: &Path) -> Result<(), String> {
    if !path.starts_with(root) {
        return Err("PathOutsideRuntime: native directory outside runtime root".into());
    }
    Ok(())
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
    Ok(format!("{:x}", hasher.finalize()))
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = File::open(path).map_err(|e| format!("HashOpen: {e}"))?;
    let mut hasher = Sha256::new();
    io::copy(&mut file, &mut hasher).map_err(|e| format!("HashRead: {e}"))?;
    Ok(format!("{:x}", hasher.finalize()))
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
}
