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
};

use sha2::{Digest, Sha256};
use whisper_core::parakeet_ipc::{
    Command, Envelope, Event, PARAKEET_IPC_VERSION, PARAKEET_MAX_FRAME_BYTES,
    PARAKEET_PROFILE_IMPORT_SINGLE_PASS, WorkerIdentity,
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
    let l2b_root = repository.join("target/parakeet-tests/l2/l2b");
    let shared_l2b_model = repository
        .join("models/parakeet-tdt-0.6b-v3.q8_0.gguf")
        .canonicalize()
        .ok()
        .is_some_and(|expected| model == expected)
        && l2b_root.canonicalize().ok().as_ref() == Some(&runtime);
    if !model.starts_with(&runtime) && !shared_l2b_model {
        return Err(
            "PathOutsideRuntime: model must be under runtime or the read-only L2b models directory"
                .into(),
        );
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
        profile: args.profile.clone(),
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
                let result =
                    transcribe_one(&mut engine, &source, &source_sha256, &identity.profile);
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
    let duration_samples = decoded.samples.len() as u64;
    let result = if profile == PROFILE_QUALIFICATION_REFERENCE {
        #[cfg(feature = "parakeet-qualification-reference")]
        {
            engine.transcribe_reference(&decoded)
        }
        #[cfg(not(feature = "parakeet-qualification-reference"))]
        {
            Err("QualificationReferenceUnavailable".to_owned())
        }
    } else {
        engine.transcribe(&decoded)
    };
    let (text, words) = result.map_err(|error| {
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
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
    let allowed = allowed_runtime_roots(&repository);
    if !allowed.iter().any(|candidate| root == *candidate) {
        return Err(
            "RuntimeRootMismatch: worker runtime is outside the authorized L2a Test roots".into(),
        );
    }
    if profile != PARAKEET_PROFILE_IMPORT_SINGLE_PASS
        && !qualification_root_is_valid(profile, &root, &repository)
    {
        return Err(
            "RuntimeRootMismatch: qualification profiles require the L2b runtime root".into(),
        );
    }
    Ok(root)
}

#[cfg(test)]
mod runtime_root_tests {
    use super::*;

    #[test]
    fn accepts_only_the_two_canonical_l2_test_roots() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
        let l2a2_only = allowed_runtime_roots_for(&repository, &["l2a-2"]);
        let l2a2 = repository
            .join("target/parakeet-tests/l2/l2a-2")
            .canonicalize()
            .unwrap();
        assert_eq!(l2a2_only, vec![l2a2]);
        assert!(allowed_runtime_roots(&repository).contains(&l2a2_only[0]));
        assert!(!allowed_runtime_roots(&repository).contains(&repository));
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

fn allowed_runtime_roots(repository: &Path) -> Vec<PathBuf> {
    #[cfg(feature = "parakeet-qualification-reference")]
    return allowed_runtime_roots_for(repository, &["l2a-1", "l2a-2", "l2b"]);
    #[cfg(not(feature = "parakeet-qualification-reference"))]
    allowed_runtime_roots_for(repository, &["l2a-1", "l2a-2"])
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
