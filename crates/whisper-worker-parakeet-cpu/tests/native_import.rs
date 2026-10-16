use std::{
    collections::BTreeMap,
    env, fs,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    time::Instant,
};

use sha2::{Digest, Sha256};
use whisper_core::parakeet_ipc::{
    Command as WorkerCommand, Envelope, Event, PARAKEET_IPC_VERSION,
    PARAKEET_PROFILE_IMPORT_SINGLE_PASS, WorkerIdentity,
};

#[test]
#[ignore = "runs real Parakeet CPU inference with locally installed native DLLs"]
fn cpu_worker_accepts_fr_and_en_twenty_times_in_one_process_and_exits_without_unload() {
    let runtime = required_path("PARAKEET_L2_RUNTIME_ROOT");
    let output_dir = required_path("PARAKEET_L2_OUTPUT_DIR");
    let model = required_path("PARAKEET_L2_MODEL");
    let native_dir = required_path("PARAKEET_L2_NATIVE_DIR");
    let worker = required_path("PARAKEET_L2_WORKER");
    let french = required_path("PARAKEET_L2_FR_WAV");
    let english = required_path("PARAKEET_L2_EN_WAV");
    for path in [&output_dir, &native_dir] {
        ensure_under(path, &runtime);
    }
    fs::create_dir_all(&output_dir).unwrap();

    let model_sha = sha256(&model);
    let mut process = WorkerProcess::start(&worker, &model, &model_sha, &native_dir, &runtime);
    let ready = process.receive();
    let Event::Ready {
        identity,
        backend,
        native_version,
    } = ready.message
    else {
        panic!("worker did not attest Ready: {ready:?}");
    };
    assert_eq!(ready.version, PARAKEET_IPC_VERSION);
    assert_eq!(identity.model_sha256, model_sha);
    assert_eq!(identity.profile, PARAKEET_PROFILE_IMPORT_SINGLE_PASS);
    assert_eq!(identity.build_sha256, sha256(&worker));
    assert_eq!(identity.runtime_sha256.len(), 64);
    assert_eq!(backend, "CPU");
    assert!(
        native_version.contains("0.2.0"),
        "unexpected native version: {native_version}"
    );

    let bad_protocol = process.send_raw(json_frame(99, WorkerCommand::Shutdown));
    assert!(
        matches!(bad_protocol.message, Event::Failed { ref code, .. } if code == "ProtocolVersionMismatch")
    );
    let missing_audio = runtime.join("does-not-exist.wav");
    let mut bad_model = identity.clone();
    bad_model.model_sha256 = "00".repeat(32);
    let bad_identity = process.request_with_hash(
        1,
        "job-bad-model",
        1,
        1,
        &bad_model,
        &missing_audio,
        &"00".repeat(32),
    );
    assert!(matches!(
        bad_identity.message,
        Event::Failed { ref code, identity: Some(ref actual), attempt: Some(1), .. }
            if code == "ModelHashMismatch" && actual == &identity
    ));
    let mut bad_runtime = identity.clone();
    bad_runtime.runtime_sha256 = "00".repeat(32);
    let runtime_mismatch = process.request_with_hash(
        2,
        "job-bad-runtime",
        1,
        1,
        &bad_runtime,
        &missing_audio,
        &"00".repeat(32),
    );
    assert!(
        matches!(runtime_mismatch.message, Event::Failed { ref code, identity: Some(ref actual), .. } if code == "RuntimeIdentityMismatch" && actual == &identity)
    );
    let mut bad_build = identity.clone();
    bad_build.build_sha256 = "00".repeat(32);
    let build_mismatch = process.request_with_hash(
        3,
        "job-bad-build",
        1,
        1,
        &bad_build,
        &missing_audio,
        &"00".repeat(32),
    );
    assert!(
        matches!(build_mismatch.message, Event::Failed { ref code, identity: Some(ref actual), .. } if code == "BuildIdentityMismatch" && actual == &identity)
    );
    let mut bad_profile = identity.clone();
    bad_profile.profile = "import-vad".into();
    let profile_mismatch = process.request_with_hash(
        4,
        "job-bad-profile",
        1,
        1,
        &bad_profile,
        &missing_audio,
        &"00".repeat(32),
    );
    assert!(
        matches!(profile_mismatch.message, Event::Failed { ref code, identity: Some(ref actual), .. } if code == "ProfileMismatch" && actual == &identity)
    );
    let invalid_attempt = process.request_with_hash(
        5,
        "job-invalid-attempt",
        1,
        0,
        &identity,
        &missing_audio,
        &"00".repeat(32),
    );
    assert!(
        matches!(invalid_attempt.message, Event::Failed { ref code, identity: Some(ref actual), .. } if code == "InvalidRequestIdentity" && actual == &identity)
    );
    let bad_source = process.request_with_hash(
        6,
        "job-source-bad",
        1,
        1,
        &identity,
        &french,
        &"00".repeat(32),
    );
    assert!(
        matches!(bad_source.message, Event::Failed { ref code, identity: Some(ref actual), attempt: Some(1), .. } if code == "SourceHashMismatch" && actual == &identity)
    );
    let stale_identity = process.request_with_hash(
        6,
        "job-stale-request-id",
        1,
        1,
        &identity,
        &missing_audio,
        &"00".repeat(32),
    );
    assert!(
        matches!(stale_identity.message, Event::Failed { ref code, identity: Some(ref actual), .. } if code == "StaleRequest" && actual == &identity)
    );

    let mut results = Vec::new();
    let started = Instant::now();
    for index in 0..20_u64 {
        let (language, path, anchor) = if index % 2 == 0 {
            ("fr", &french, "secondes")
        } else {
            ("en", &english, "victorious")
        };
        let event = process.request(
            index + 7,
            &format!("job-{language}-{index}"),
            if index == 0 { 2 } else { 1 },
            1,
            &identity,
            path,
        );
        let Event::Result {
            request_id,
            job_id,
            generation,
            attempt,
            identity: response_identity,
            source_sha256,
            duration_samples,
            text,
            words,
        } = event.message
        else {
            panic!("inference {index} failed: {event:?}");
        };
        assert_eq!(request_id, index + 7);
        assert_eq!(job_id, format!("job-{language}-{index}"));
        assert_eq!(generation, if index == 0 { 2 } else { 1 });
        assert_eq!(attempt, 1);
        assert_eq!(response_identity, identity);
        assert_eq!(source_sha256, sha256(path));
        assert!(!text.trim().is_empty(), "empty {language} transcript");
        assert!(
            text.to_lowercase().contains(anchor),
            "{language} transcript missed expected lexical anchor: {text}"
        );
        let duration_ms = duration_samples * 1000 / 16_000;
        assert!(
            words
                .iter()
                .all(|word| word.start_ms < word.end_ms && u64::from(word.end_ms) <= duration_ms)
        );
        eprintln!(
            "L2A1_NATIVE_RESULT index={index} language={language} source_samples={duration_samples} words={} text={text:?}",
            words.len()
        );
        results.push(serde_json::json!({"index":index,"language":language,"duration_samples":duration_samples,"text":text,"words":words}));
    }
    let stale_generation = process.request_with_hash(
        27,
        "job-fr-0",
        1,
        2,
        &identity,
        &missing_audio,
        &"00".repeat(32),
    );
    assert!(
        matches!(stale_generation.message, Event::Failed { request_id: Some(27), generation: Some(1), attempt: Some(2), ref code, identity: Some(ref actual), .. } if code == "StaleGeneration" && actual == &identity)
    );
    let stale_attempt = process.request_with_hash(
        28,
        "job-fr-0",
        2,
        1,
        &identity,
        &missing_audio,
        &"00".repeat(32),
    );
    assert!(matches!(
        stale_attempt.message,
        Event::Failed {
            request_id: Some(28),
            job_id: Some(ref job_id),
            generation: Some(2),
            attempt: Some(1),
            ref code,
            identity: Some(ref actual),
            ..
        } if job_id == "job-fr-0" && code == "StaleAttempt" && actual == &identity
    ));
    // Each response is decoded only after native_ffi has copied transcript and
    // word strings and dropped the C result; retain the first across later calls.
    assert!(results[0]["text"].as_str().unwrap().contains("secondes"));
    assert!(!results[0]["words"].as_array().unwrap().is_empty());
    let elapsed = started.elapsed();
    process.shutdown();
    let worker_status = process.wait();
    assert!(
        worker_status.success(),
        "worker crashed on normal process exit: {worker_status}"
    );

    let missing_native = runtime.join("missing-native");
    fs::create_dir_all(&missing_native).unwrap();
    let missing_dll = Command::new(&worker)
        .args([
            "--model",
            model.to_str().unwrap(),
            "--model-sha256",
            &model_sha,
            "--native-dir",
            missing_native.to_str().unwrap(),
        ])
        .env("PARAKEET_L2_RUNTIME_ROOT", &runtime)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap();
    assert_eq!(missing_dll.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&missing_dll.stderr).contains("NativeLibrary:"));

    let mut dll_hashes = BTreeMap::new();
    for name in [
        "nemo_speech_asr_c.dll",
        "nemo_speech_asr.dll",
        "ggml.dll",
        "ggml-base.dll",
        "ggml-cpu.dll",
    ] {
        dll_hashes.insert(name, sha256(&native_dir.join(name)));
    }
    let report = serde_json::json!({
        "worker_sha256": identity.build_sha256,
        "runtime_sha256": identity.runtime_sha256,
        "dll_sha256": dll_hashes,
        "model_sha256": model_sha,
        "native_version": native_version,
        "backend": "CPU",
        "inference_count": 20,
        "source_identity_rejection": "SourceHashMismatch",
        "missing_native_dll": String::from_utf8_lossy(&missing_dll.stderr),
        "process_exit": "success",
        "elapsed_seconds": elapsed.as_secs_f64(),
        "results": results,
    });
    fs::write(
        output_dir.join("native-import-report.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    println!(
        "L2A1_NATIVE_REPORT {}",
        serde_json::to_string(&report).unwrap()
    );
}

struct WorkerProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl WorkerProcess {
    fn start(
        worker: &Path,
        model: &Path,
        model_sha: &str,
        native_dir: &Path,
        runtime: &Path,
    ) -> Self {
        let mut child = Command::new(worker)
            .args([
                "--model",
                model.to_str().unwrap(),
                "--model-sha256",
                model_sha,
                "--native-dir",
                native_dir.to_str().unwrap(),
            ])
            .env("PARAKEET_L2_RUNTIME_ROOT", runtime)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        Self {
            stdin: child.stdin.take().unwrap(),
            stdout: BufReader::new(child.stdout.take().unwrap()),
            child,
        }
    }

    fn send_raw(&mut self, frame: Vec<u8>) -> Envelope<Event> {
        self.stdin.write_all(&frame).unwrap();
        self.stdin.flush().unwrap();
        self.receive()
    }

    fn request(
        &mut self,
        request_id: u64,
        job_id: &str,
        generation: u64,
        attempt: u64,
        identity: &WorkerIdentity,
        source: &Path,
    ) -> Envelope<Event> {
        self.request_with_hash(
            request_id,
            job_id,
            generation,
            attempt,
            identity,
            source,
            &sha256(source),
        )
    }

    fn request_with_hash(
        &mut self,
        request_id: u64,
        job_id: &str,
        generation: u64,
        attempt: u64,
        identity: &WorkerIdentity,
        source: &Path,
        source_sha256: &str,
    ) -> Envelope<Event> {
        self.send_raw(json_frame(
            PARAKEET_IPC_VERSION,
            WorkerCommand::Transcribe {
                request_id,
                job_id: job_id.into(),
                generation,
                attempt,
                identity: identity.clone(),
                source_path: source.to_string_lossy().into_owned(),
                source_sha256: source_sha256.into(),
            },
        ))
    }

    fn receive(&mut self) -> Envelope<Event> {
        let mut line = Vec::new();
        self.stdout.read_until(b'\n', &mut line).unwrap();
        assert!(
            !line.is_empty(),
            "worker exited before an IPC response: {:?}",
            self.child.try_wait().unwrap()
        );
        assert!(line.len() <= 1_048_576);
        serde_json::from_slice(&line).unwrap()
    }

    fn shutdown(&mut self) {
        let event = self.send_raw(json_frame(PARAKEET_IPC_VERSION, WorkerCommand::Shutdown));
        assert!(matches!(event.message, Event::Stopped));
    }

    fn wait(&mut self) -> std::process::ExitStatus {
        self.child.wait().unwrap()
    }
}

fn json_frame(version: u16, message: WorkerCommand) -> Vec<u8> {
    let mut bytes = serde_json::to_vec(&Envelope { version, message }).unwrap();
    bytes.push(b'\n');
    bytes
}

fn required_path(name: &str) -> PathBuf {
    let path = env::var_os(name).unwrap_or_else(|| panic!("missing required {name}"));
    PathBuf::from(path)
        .canonicalize()
        .unwrap_or_else(|error| panic!("invalid {name}: {error}"))
}

fn ensure_under(path: &Path, root: &Path) {
    assert!(
        path.starts_with(root),
        "{} is outside {}",
        path.display(),
        root.display()
    );
}

fn sha256(path: &Path) -> String {
    let mut file = fs::File::open(path).unwrap();
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher).unwrap();
    format!("{:x}", hasher.finalize())
}
