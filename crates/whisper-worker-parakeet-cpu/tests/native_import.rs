use std::{
    collections::BTreeMap,
    env,
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use sha2::{Digest, Sha256};
use whisper_core::parakeet_ipc::{
    Command as WorkerCommand, Envelope, Event, ExpectedGpu, GpuEvidence, PARAKEET_IPC_VERSION,
    PARAKEET_PROFILE_IMPORT_SINGLE_PASS, WorkerIdentity,
};
use whisper_core::parakeet_windows::PROFILE_QUALIFICATION_REFERENCE;

const L3A_WINDOWS_GPU_NAME: &str = "NVIDIA GeForce GTX 1080 Ti";
const L3A_WINDOWS_GPU_PCI_BUS_ID: &str = "00000000:01:00.0";
const L3A_WINDOWS_GPU_UUID: &str = "GPU-99e636f6-45ac-0068-a378-7d7a8bca645e";
const L3A_WINDOWS_GPU_INSTANCE_ID: &str =
    r"PCI\VEN_10DE&DEV_1B06&SUBSYS_85F11043&REV_A1\4&8BD6E8D&0&0008";
const L3A_WINDOWS_GPU_LOCATION_PATHS: &str = "PCIROOT(0)#PCI(0100)#PCI(0000)";

#[test]
fn additive_gpu_fields_default_when_reading_existing_cpu_ipc_frames() {
    let command = format!(
        r#"{{"version":{PARAKEET_IPC_VERSION},"message":{{"type":"transcribe","request_id":1,"job_id":"legacy","generation":1,"attempt":1,"identity":{{"model_sha256":"{}","runtime_sha256":"{}","build_sha256":"{}","profile":"{}"}},"source_path":"audio.wav","source_sha256":"{}"}}}}"#,
        "ab".repeat(32),
        "bc".repeat(32),
        "cd".repeat(32),
        PARAKEET_PROFILE_IMPORT_SINGLE_PASS,
        "ef".repeat(32),
    );
    let decoded: Envelope<WorkerCommand> = serde_json::from_str(&command).unwrap();
    assert!(matches!(
        decoded.message,
        WorkerCommand::Transcribe {
            expected_gpu: None,
            ..
        }
    ));

    let ready = format!(
        r#"{{"version":{PARAKEET_IPC_VERSION},"message":{{"type":"ready","identity":{{"model_sha256":"{}","runtime_sha256":"{}","build_sha256":"{}","profile":"{}"}},"backend":"CPU","native_version":"nemo 0.2.0"}}}}"#,
        "ab".repeat(32),
        "bc".repeat(32),
        "cd".repeat(32),
        PARAKEET_PROFILE_IMPORT_SINGLE_PASS,
    );
    let decoded: Envelope<Event> = serde_json::from_str(&ready).unwrap();
    assert!(matches!(
        decoded.message,
        Event::Ready {
            gpu_evidence: None,
            ..
        }
    ));
}

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
        ..
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
            ..
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

#[test]
#[ignore = "R7 private diagnostic worker recipe; has a parent-enforced duration deadline"]
fn r7_private_worker_timing() {
    let case = env::var("PARAKEET_R7_CASE").expect("PARAKEET_R7_CASE");
    assert!(matches!(case.as_str(), "en5" | "en30"));
    let root = required_path("PARAKEET_R7_EXEC_ROOT");
    let log_dir = root.join("logs");
    fs::create_dir_all(&log_dir).unwrap();
    let stdout_path = log_dir.join(format!("{case}-private-child.stdout"));
    let stderr_path = log_dir.join(format!("{case}-private-child.stderr"));
    let stdout = fs::File::create(&stdout_path).unwrap();
    let stderr = fs::File::create(&stderr_path).unwrap();
    let test_exe = env::current_exe().unwrap();
    let mut child = Command::new(test_exe)
        .args([
            "--ignored",
            "--exact",
            "r7_private_worker_timing_child",
            "--nocapture",
        ])
        .env("PARAKEET_R7_PRIVATE_CHILD", "1")
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
        .unwrap();
    let samples = match case.as_str() {
        "en5" => 5 * 60 * 16_000,
        _ => 30 * 60 * 16_000,
    };
    let timeout = Duration::from_secs(samples / 16_000 / 4 + 120);
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(
                status.success(),
                "private worker child failed; stdout={} stderr={}",
                stdout_path.display(),
                stderr_path.display()
            );
            break;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let status = child.wait().unwrap();
            panic!("private worker child exceeded {timeout:?}: {status}");
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

#[test]
#[ignore = "nested child for the parent-enforced R7 private timing recipe"]
fn r7_private_worker_timing_child() {
    if env::var_os("PARAKEET_R7_PRIVATE_CHILD").is_none() {
        return;
    }
    let runtime = required_path("PARAKEET_L2_RUNTIME_ROOT");
    let model = required_path("PARAKEET_L2_MODEL");
    let native_dir = required_path("PARAKEET_L2_NATIVE_DIR");
    let worker = required_path("PARAKEET_R7_PRIVATE_WORKER");
    let source = required_path("PARAKEET_R7_RECIPE_WAV");
    let model_sha = sha256(&model);
    let worker_sha = sha256(&worker);
    let started = Instant::now();
    let mut process = WorkerProcess::start(&worker, &model, &model_sha, &native_dir, &runtime);
    let ready = process.receive();
    let Event::Ready {
        identity, backend, ..
    } = ready.message
    else {
        panic!("private worker not Ready: {ready:?}");
    };
    assert_eq!(backend, "CPU");
    assert_eq!(identity.build_sha256, worker_sha);
    assert_eq!(identity.model_sha256, model_sha);
    assert_eq!(identity.profile, PARAKEET_PROFILE_IMPORT_SINGLE_PASS);
    let event = process.request(1, "r7-private", 1, 1, &identity, &source);
    let Event::Result {
        identity: result_identity,
        duration_samples,
        text,
        words,
        ..
    } = event.message
    else {
        panic!("private worker inference failed: {event:?}");
    };
    assert_eq!(result_identity, identity);
    assert!(!text.trim().is_empty());
    assert!(words.iter().all(|word| word.start_ms < word.end_ms));
    assert!(
        words
            .windows(2)
            .all(|pair| pair[0].end_ms <= pair[1].start_ms)
    );
    process.shutdown();
    let status = process.wait();
    assert!(status.success());
    let report = serde_json::json!({
        "case": env::var("PARAKEET_R7_CASE").unwrap(), "duration_samples": duration_samples,
        "worker_sha256": worker_sha, "model_sha256": model_sha, "source_sha256": sha256(&source),
        "word_count": words.len(), "native_process_elapsed_ms": started.elapsed().as_millis(),
        "result": "private diagnostic only; no product qualification",
    });
    let log = required_path("PARAKEET_R7_TIMING_LOG");
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log)
        .unwrap();
    serde_json::to_writer(&mut file, &report).unwrap();
    file.write_all(b"\n").unwrap();
    println!("R7_PRIVATE {report}");
}

#[test]
#[ignore = "runs the R5 short FR/EN normalization fixtures with the locally qualified CPU DLL set"]
fn cpu_worker_normalizes_fr87_fr27_and_en15_short_fixtures() {
    let runtime = required_path("PARAKEET_L2_RUNTIME_ROOT");
    let output_dir = required_path("PARAKEET_L2_OUTPUT_DIR");
    let model = required_path("PARAKEET_L2_MODEL");
    let native_dir = required_path("PARAKEET_L2_NATIVE_DIR");
    let worker = required_path("PARAKEET_L2_WORKER");
    let fr87 = required_path("PARAKEET_L2_FR_WAV");
    let fr27 = required_path("PARAKEET_L2_FR27_WAV");
    let en15 = required_path("PARAKEET_L2_EN_WAV");
    for path in [&output_dir, &native_dir] {
        ensure_under(path, &runtime);
    }
    fs::create_dir_all(&output_dir).unwrap();

    let model_sha = sha256(&model);
    let mut process = WorkerProcess::start(&worker, &model, &model_sha, &native_dir, &runtime);
    let ready = process.receive();
    let Event::Ready { identity, .. } = ready.message else {
        panic!("worker did not attest Ready: {ready:?}");
    };
    assert_eq!(identity.model_sha256, model_sha);
    assert_eq!(identity.profile, PARAKEET_PROFILE_IMPORT_SINGLE_PASS);
    assert_eq!(identity.build_sha256, sha256(&worker));

    let mut results = Vec::new();
    for (request_id, language, source) in
        [(1, "fr87", &fr87), (2, "fr27", &fr27), (3, "en15", &en15)]
    {
        let event = process.request(
            request_id,
            &format!("r5-{language}"),
            1,
            1,
            &identity,
            source,
        );
        let Event::Result {
            identity: result_identity,
            duration_samples,
            text,
            words,
            ..
        } = event.message
        else {
            panic!("{language} inference failed: {event:?}");
        };
        assert_eq!(result_identity, identity);
        assert!(!text.trim().is_empty());
        assert!(!text.contains('\u{fffd}'));
        assert!(words.iter().all(|word| word.start_ms < word.end_ms));
        assert!(
            words
                .windows(2)
                .all(|pair| pair[0].end_ms <= pair[1].start_ms)
        );
        if language == "fr87" {
            let pair = words
                .windows(2)
                .find(|pair| pair[0].text == "faire?" && pair[1].text == "Je")
                .expect("the normalized real sequence faire / ? / Je is present");
            assert_eq!((pair[0].start_ms, pair[0].end_ms), (17_760, 17_920));
            assert_eq!((pair[1].start_ms, pair[1].end_ms), (18_000, 18_080));
        }
        if language == "fr27" {
            assert!(words.last().is_some_and(|word| word.text == "à..."));
        }
        if language == "en15" {
            assert!(text.to_lowercase().contains("victorious"));
        }
        results.push(serde_json::json!({
            "fixture": language,
            "source_sha256": sha256(source),
            "duration_samples": duration_samples,
            "text": text,
            "words": words,
        }));
    }
    process.shutdown();
    let exit = process.wait();
    assert!(exit.success(), "worker exited unsuccessfully: {exit}");

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
        "worker_sha256": sha256(&worker),
        "model_sha256": model_sha,
        "dll_sha256": dll_hashes,
        "backend": "CPU",
        "inference_count": results.len(),
        "results": results,
        "process_exit": "success",
    });
    fs::write(
        output_dir.join("r5-short-fixtures.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    println!(
        "R5_SHORT_FIXTURES {}",
        serde_json::to_string(&report).unwrap()
    );
}

#[test]
#[ignore = "informative private EN60 reference recipe; not product qualification"]
fn r6_reference_en60() {
    let runtime = required_path("PARAKEET_L2_RUNTIME_ROOT");
    let output_dir = required_path("PARAKEET_L2_OUTPUT_DIR");
    let model = required_path("PARAKEET_L2_MODEL");
    let native_dir = required_path("PARAKEET_L2_NATIVE_DIR");
    let worker = required_path("PARAKEET_L2_WORKER");
    let source = required_path("PARAKEET_R6_RECIPE_WAV");
    ensure_under(&output_dir, &runtime.join("r6"));
    assert!(source.metadata().unwrap().len() > 0);
    fs::create_dir_all(&output_dir).unwrap();

    let model_sha = sha256(&model);
    let started = Instant::now();
    let mut process =
        WorkerProcess::start_reference(&worker, &model, &model_sha, &native_dir, &runtime);
    let ready = process.receive();
    let Event::Ready {
        identity, backend, ..
    } = ready.message
    else {
        panic!("EN60 worker did not attest Ready: {ready:?}");
    };
    assert_eq!(identity.model_sha256, model_sha);
    assert_eq!(identity.profile, PROFILE_QUALIFICATION_REFERENCE);
    assert_eq!(identity.build_sha256, sha256(&worker));
    assert_eq!(backend, "CPU");

    let event = process.request(1, "r6-en60", 1, 1, &identity, &source);
    let Event::Result {
        identity: result_identity,
        duration_samples,
        text,
        words,
        ..
    } = event.message
    else {
        panic!("EN60 reference inference failed: {event:?}");
    };
    assert_eq!(result_identity, identity);
    assert_eq!(duration_samples, 60 * 60 * 16_000);
    assert!(!text.trim().is_empty());
    assert!(words.iter().all(|word| word.start_ms < word.end_ms));
    assert!(
        words
            .windows(2)
            .all(|pair| pair[0].end_ms <= pair[1].start_ms)
    );
    process.shutdown();
    let exit = process.wait();
    assert!(
        exit.success(),
        "EN60 reference worker exited unsuccessfully: {exit}"
    );

    let report = serde_json::json!({
        "case": "en60",
        "profile": PROFILE_QUALIFICATION_REFERENCE,
        "worker_sha256": sha256(&worker),
        "model_sha256": model_sha,
        "source_sha256": sha256(&source),
        "duration_samples": duration_samples,
        "native_and_process_elapsed_ms": started.elapsed().as_millis(),
        "word_count": words.len(),
        "result": "informative private reference; no product qualification",
    });
    fs::write(
        output_dir.join("r6-en60-reference.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    println!(
        "R6_EN60 samples={duration_samples} words={} elapsed_ms={} worker_sha256={}",
        words.len(),
        started.elapsed().as_millis(),
        sha256(&worker)
    );
}

struct WorkerProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    asr_log_path: Option<PathBuf>,
    spawned_at: Option<Instant>,
}

#[derive(Clone, Debug)]
struct GpuSample {
    timestamp: String,
    utilization_gpu: u32,
    memory_used_mib: u64,
    sampled_at: Instant,
}

#[derive(Clone, Debug)]
struct ProcessMemorySample {
    timestamp_utc_ms: u64,
    working_set_bytes: u64,
    private_bytes: u64,
}

struct ProcessMemorySampler {
    child: Child,
    samples: Arc<Mutex<Vec<ProcessMemorySample>>>,
    raw_lines: Arc<Mutex<Vec<String>>>,
    reader: Option<std::thread::JoinHandle<()>>,
    stderr: Option<ChildStderr>,
}

impl ProcessMemorySampler {
    fn start(pid: u32) -> Self {
        let command = format!(
            "$targetPid = {pid}; while ($true) {{ $p = Get-Process -Id $targetPid -ErrorAction SilentlyContinue; if ($null -eq $p) {{ break }}; [Console]::Out.WriteLine((\"{{0}},{{1}},{{2}}\" -f @([DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds(), $p.WorkingSet64, $p.PrivateMemorySize64))); [Console]::Out.Flush(); Start-Sleep -Milliseconds 250 }}"
        );
        let mut child = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", &command])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("PowerShell process-memory sampler starts");
        let stdout = child
            .stdout
            .take()
            .expect("PowerShell sampler stdout is piped");
        let child_stderr = child.stderr.take();
        let samples = Arc::new(Mutex::new(Vec::new()));
        let raw_lines = Arc::new(Mutex::new(Vec::new()));
        let reader_samples = Arc::clone(&samples);
        let reader_raw_lines = Arc::clone(&raw_lines);
        let reader = std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                reader_raw_lines.lock().unwrap().push(line.clone());
                let mut fields = line.split(',').map(str::trim);
                let Some((Ok(timestamp_utc_ms), Ok(working_set_bytes), Ok(private_bytes))) =
                    fields.next().and_then(|timestamp| {
                        Some((
                            timestamp.parse(),
                            fields.next()?.parse(),
                            fields.next()?.parse(),
                        ))
                    })
                else {
                    continue;
                };
                reader_samples.lock().unwrap().push(ProcessMemorySample {
                    timestamp_utc_ms,
                    working_set_bytes,
                    private_bytes,
                });
            }
        });
        Self {
            child,
            samples,
            raw_lines,
            reader: Some(reader),
            stderr: child_stderr,
        }
    }

    fn stop(mut self) -> (Vec<ProcessMemorySample>, Vec<String>, String, Option<i32>) {
        let _ = self.child.kill();
        let status = self.child.wait().ok().and_then(|status| status.code());
        if let Some(reader) = self.reader.take() {
            reader.join().expect("PowerShell sampler reader joins");
        }
        let samples = self.samples.lock().unwrap().clone();
        let raw_lines = self.raw_lines.lock().unwrap().clone();
        let mut stderr = String::new();
        if let Some(mut pipe) = self.stderr.take() {
            let _ = pipe.read_to_string(&mut stderr);
        }
        (samples, raw_lines, stderr, status)
    }
}

impl Drop for ProcessMemorySampler {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        self.stderr.take();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}

struct GpuCall {
    event: Envelope<Event>,
    request_started_at: Instant,
    native_call_started_at: Instant,
    result_received_at: Instant,
}

#[test]
#[ignore = "L3a GPU single-pass recipe; requires the exact release worker and verified CUDA DLLs"]
fn gpu_worker_fr_en_single_pass() {
    let runtime = required_path("PARAKEET_L2_RUNTIME_ROOT");
    let model = required_path("PARAKEET_L2_MODEL");
    let native_dir = required_path("PARAKEET_L2_NATIVE_DIR");
    let worker = required_path("PARAKEET_L2_WORKER");
    let fixtures = [
        ("en5", required_path("PARAKEET_L3_EN5_WAV")),
        ("fr87", required_path("PARAKEET_L3_FR87_WAV")),
        ("fr27", required_path("PARAKEET_L3_FR27_WAV")),
        ("en15", required_path("PARAKEET_L2_EN_WAV")),
        ("en30", required_path("PARAKEET_L3_EN30_WAV")),
    ];
    ensure_under(&native_dir, &runtime);
    let recipes = runtime.join("recipes/gpu-single-pass");
    let logs = runtime.join("logs");
    fs::create_dir_all(&recipes).unwrap();
    fs::create_dir_all(&logs).unwrap();
    let gpu_preflight_utc_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock is after Unix epoch")
        .as_millis();
    let gpu = nvidia_smi_gpu();
    let gpu_preflight = serde_json::json!({
        "timestamp_utc_ms": gpu_preflight_utc_ms,
        "windows_inventory_source": "Windows PnP inventory checked for L3a on 2026-10-18",
        "windows_inventory": {
            "name": L3A_WINDOWS_GPU_NAME,
            "instance_id": L3A_WINDOWS_GPU_INSTANCE_ID,
            "location_paths": L3A_WINDOWS_GPU_LOCATION_PATHS,
            "pci_bus_id": L3A_WINDOWS_GPU_PCI_BUS_ID,
        },
        "nvidia_smi": gpu,
        "result": "single GPU identity matches the Windows inventory before worker start",
    });
    fs::write(
        recipes.join("gpu-preflight.json"),
        serde_json::to_vec_pretty(&gpu_preflight).unwrap(),
    )
    .unwrap();

    let model_sha = sha256(&model);
    let mut process = WorkerProcess::start_gpu(&worker, &model, &model_sha, &native_dir, &runtime);
    let ram_sampler = ProcessMemorySampler::start(process.child.id());
    let ready = process.receive();
    let Event::Ready {
        identity,
        backend,
        native_version,
        gpu_evidence: Some(ready_evidence),
    } = ready.message
    else {
        panic!("GPU worker did not provide Ready with GPU evidence: {ready:?}");
    };
    assert_eq!(backend, "CUDA0");
    assert!(native_version.contains("0.2.0"), "{native_version}");
    assert_eq!(identity.model_sha256, model_sha);
    assert_eq!(identity.build_sha256, sha256(&worker));
    assert_eq!(identity.profile, PARAKEET_PROFILE_IMPORT_SINGLE_PASS);
    let worker_startup_to_ready_ms = process
        .spawned_at
        .expect("GPU worker records its spawn time")
        .elapsed()
        .as_millis();
    let log_path = process.asr_log_path.as_ref().unwrap().clone();
    assert_gpu_evidence(&ready_evidence, &gpu, process.child.id(), &log_path);

    let mut results = Vec::new();
    for (request_id, (name, source)) in fixtures.iter().enumerate() {
        let baseline = if *name == "en5" {
            Some(nvidia_smi_activity().expect("nvidia-smi activity sample before EN5"))
        } else {
            None
        };
        let sampler = baseline.as_ref().map(|_| start_gpu_sampler());
        let call = process.request_gpu(request_id as u64 + 1, name, &identity, source, &gpu);
        let samples = sampler.map(stop_gpu_sampler).unwrap_or_default();
        let event = call.event;
        let Event::Result {
            identity: result_identity,
            duration_samples,
            text,
            words,
            native_inference_ms,
            gpu_evidence: Some(evidence),
            ..
        } = event.message
        else {
            panic!("{name} GPU inference failed: {event:?}");
        };
        assert_eq!(result_identity, identity);
        assert!(!text.trim().is_empty(), "{name} returned empty text");
        assert!(
            !text.contains('\u{fffd}'),
            "{name} emitted invalid UTF-8 replacement"
        );
        assert!(
            words.iter().all(|word| word.start_ms < word.end_ms),
            "{name}"
        );
        assert!(
            words
                .windows(2)
                .all(|pair| pair[0].end_ms <= pair[1].start_ms),
            "{name} has overlapping words"
        );
        assert_gpu_evidence(&evidence, &gpu, process.child.id(), &log_path);
        if *name == "fr27" {
            assert!(words.last().is_some_and(|word| word.text == "à..."));
        }
        if *name == "en15" {
            assert!(text.to_lowercase().contains("victorious"));
        }
        if *name == "en5" {
            assert!(
                native_inference_ms <= 11_100,
                "A2 GPU inference {native_inference_ms} ms exceeds 11.1 s"
            );
            let baseline = baseline.unwrap();
            let native_window_samples = samples
                .iter()
                .filter(|sample| {
                    sample.sampled_at >= call.native_call_started_at
                        && sample.sampled_at <= call.result_received_at
                })
                .collect::<Vec<_>>();
            assert!(
                native_window_samples.iter().any(|sample| {
                    sample.utilization_gpu > 0 || sample.memory_used_mib > baseline.memory_used_mib
                }),
                "A2: no GPU activity sample inside NativeCallStarted→Result; baseline={baseline:?}, samples_inside={native_window_samples:?}, all_samples={samples:?}"
            );
        }
        results.push(serde_json::json!({
            "fixture": name,
            "source_sha256": sha256(source),
            "duration_samples": duration_samples,
            "native_inference_ms": native_inference_ms,
            "text": text,
            "words": words,
            "gpu_evidence": evidence,
            "request_elapsed_ms": call.result_received_at.duration_since(call.request_started_at).as_millis(),
            "native_call_window_ms": {
                "marker_to_result": call.result_received_at.duration_since(call.native_call_started_at).as_millis(),
                "samples": samples.iter().map(|sample| serde_json::json!({
                    "timestamp": sample.timestamp,
                    "request_offset_ms": sample.sampled_at.duration_since(call.request_started_at).as_millis(),
                    "inside_native_call_window": sample.sampled_at >= call.native_call_started_at && sample.sampled_at <= call.result_received_at,
                    "utilization_gpu": sample.utilization_gpu,
                    "memory_used_mib": sample.memory_used_mib,
                })).collect::<Vec<_>>(),
            },
        }));
    }
    let (ram_samples, ram_sampler_raw_lines, ram_sampler_stderr, ram_sampler_exit_code) =
        ram_sampler.stop();
    assert!(
        !ram_samples.is_empty(),
        "PowerShell process memory sampler returned no samples; raw_stdout_first_lines={:?}; stderr={ram_sampler_stderr:?}; exit_code={ram_sampler_exit_code:?}",
        ram_sampler_raw_lines.iter().take(10).collect::<Vec<_>>()
    );
    let peak_working_set_bytes = ram_samples
        .iter()
        .map(|sample| sample.working_set_bytes)
        .max()
        .unwrap();
    let peak_private_bytes = ram_samples
        .iter()
        .map(|sample| sample.private_bytes)
        .max()
        .unwrap();
    process.shutdown();
    let exit = process.wait();
    assert!(exit.success(), "GPU worker exited unsuccessfully: {exit}");
    let report = serde_json::json!({
        "gpu_preflight": gpu_preflight,
        "worker_sha256": sha256(&worker),
        "worker_startup_to_ready_ms": worker_startup_to_ready_ms,
        "worker_memory": {
            "sample_count": ram_samples.len(),
            "first_sample_utc_ms": ram_samples.first().unwrap().timestamp_utc_ms,
            "last_sample_utc_ms": ram_samples.last().unwrap().timestamp_utc_ms,
            "peak_working_set_bytes": peak_working_set_bytes,
            "peak_private_bytes": peak_private_bytes,
        },
        "cpu_r6_en5_total_baseline_ms": 38_600,
        "en5_startup_plus_request_end_to_end_ms": worker_startup_to_ready_ms + results[0]["request_elapsed_ms"].as_u64().unwrap_or(0) as u128,
        "model_sha256": model_sha,
        "gpu": gpu,
        "dll_sha256": cuda_dll_hashes(&native_dir),
        "inference_count": results.len(),
        "process_exit": exit.code(),
        "results": results,
    });
    fs::write(
        recipes.join("gpu-single-pass.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("L3A_GPU_SINGLE_PASS {report}");
}

#[test]
#[ignore = "L3a GPU 20-call recipe; requires the exact release worker and verified CUDA DLLs"]
fn gpu_worker_twenty_calls() {
    let runtime = required_path("PARAKEET_L2_RUNTIME_ROOT");
    let model = required_path("PARAKEET_L2_MODEL");
    let native_dir = required_path("PARAKEET_L2_NATIVE_DIR");
    let worker = required_path("PARAKEET_L2_WORKER");
    let french = required_path("PARAKEET_L3_FR87_WAV");
    let english = required_path("PARAKEET_L2_EN_WAV");
    let recipes = runtime.join("recipes/gpu-twenty-calls");
    fs::create_dir_all(&recipes).unwrap();
    let gpu = nvidia_smi_gpu();
    let mut process =
        WorkerProcess::start_gpu(&worker, &model, &sha256(&model), &native_dir, &runtime);
    let ready = process.receive();
    let Event::Ready {
        identity,
        backend,
        gpu_evidence: Some(ready_evidence),
        ..
    } = ready.message
    else {
        panic!("GPU worker did not attest Ready: {ready:?}");
    };
    assert_eq!(backend, "CUDA0");
    assert_gpu_evidence(
        &ready_evidence,
        &gpu,
        process.child.id(),
        process.asr_log_path.as_ref().unwrap(),
    );
    let mut count = 0_u64;
    let mut summaries = Vec::new();
    for index in 0..20 {
        let (language, source) = if index % 2 == 0 {
            ("fr", &french)
        } else {
            ("en", &english)
        };
        let call = process.request_gpu(
            index + 1,
            &format!("{language}-{index}"),
            &identity,
            source,
            &gpu,
        );
        let event = call.event;
        let Event::Result {
            text,
            words,
            gpu_evidence: Some(evidence),
            ..
        } = event.message
        else {
            panic!("GPU call {index} failed: {event:?}");
        };
        assert!(!text.trim().is_empty());
        assert!(
            words
                .windows(2)
                .all(|pair| pair[0].end_ms <= pair[1].start_ms),
            "overlap at inference {index}"
        );
        assert_gpu_evidence(
            &evidence,
            &gpu,
            process.child.id(),
            process.asr_log_path.as_ref().unwrap(),
        );
        count += 1;
        summaries.push(serde_json::json!({"language":language,"word_count":words.len()}));
    }
    process.shutdown();
    let exit = process.wait();
    assert!(exit.success(), "GPU worker crashed on exit: {exit}");
    assert_eq!(count, 20);
    let report = serde_json::json!({
        "worker_sha256": sha256(&worker),
        "model_sha256": sha256(&model),
        "gpu": gpu,
        "dll_sha256": cuda_dll_hashes(&native_dir),
        "inference_count": count,
        "process_exit": exit.code(),
        "results": summaries,
    });
    fs::write(
        recipes.join("gpu-twenty-calls.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("L3A_GPU_TWENTY_CALLS {report}");
}

#[test]
#[ignore = "L3a wrong-device recipe; requires the exact release worker and verified CUDA DLLs"]
fn gpu_worker_rejects_wrong_device_before_audio() {
    let runtime = required_path("PARAKEET_L2_RUNTIME_ROOT");
    let model = required_path("PARAKEET_L2_MODEL");
    let native_dir = required_path("PARAKEET_L2_NATIVE_DIR");
    let worker = required_path("PARAKEET_L2_WORKER");
    let gpu = nvidia_smi_gpu();
    let recipes = runtime.join("recipes/gpu-wrong-device");
    fs::create_dir_all(&recipes).unwrap();
    let mut process =
        WorkerProcess::start_gpu(&worker, &model, &sha256(&model), &native_dir, &runtime);
    let ready = process.receive();
    let Event::Ready {
        identity,
        backend,
        gpu_evidence: Some(evidence),
        ..
    } = ready.message
    else {
        panic!("GPU worker did not attest Ready: {ready:?}");
    };
    assert_eq!(backend, "CUDA0");
    assert_gpu_evidence(
        &evidence,
        &gpu,
        process.child.id(),
        process.asr_log_path.as_ref().unwrap(),
    );
    let mut wrong_gpu = gpu.clone();
    wrong_gpu.uuid.push_str("-wrong");
    let missing_audio = runtime.join("audio-must-not-be-opened.wav");
    let frame = json_frame(
        PARAKEET_IPC_VERSION,
        WorkerCommand::Transcribe {
            request_id: 1,
            job_id: "wrong-device".into(),
            generation: 1,
            attempt: 1,
            identity: Box::new(identity),
            source_path: missing_audio.to_string_lossy().into_owned(),
            source_sha256: "00".repeat(32),
            expected_gpu: Some(wrong_gpu),
        },
    );
    let response = process.send_raw(frame);
    assert!(matches!(response.message, Event::Failed { ref code, .. } if code == "WrongGpuDevice"));
    assert!(
        !missing_audio.exists(),
        "the wrong-device case created audio"
    );
    process.shutdown();
    let exit = process.wait();
    assert!(exit.success());
    let report = serde_json::json!({
        "worker_sha256": sha256(&worker),
        "model_sha256": sha256(&model),
        "gpu": gpu,
        "dll_sha256": cuda_dll_hashes(&native_dir),
        "result": "WrongGpuDevice before source open or NativeCallStarted",
        "process_exit": exit.code(),
    });
    fs::write(
        recipes.join("gpu-wrong-device.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("L3A_GPU_WRONG_DEVICE {report}");
}

fn nvidia_smi_gpu() -> ExpectedGpu {
    let output = Command::new("nvidia-smi")
        .args(["--query-gpu=count,name,pci.bus_id,uuid", "--format=csv"])
        .output()
        .expect("nvidia-smi inventory query starts");
    assert!(output.status.success(), "nvidia-smi failed: {output:?}");
    let output = String::from_utf8(output.stdout).expect("nvidia-smi CSV is UTF-8");
    let rows = output
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.trim_start().starts_with("count,"))
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 1, "expected exactly one NVIDIA GPU: {output}");
    let fields = rows[0].split(',').map(str::trim).collect::<Vec<_>>();
    assert_eq!(fields.len(), 4, "invalid nvidia-smi row: {}", rows[0]);
    assert_eq!(fields[0], "1", "nvidia-smi did not report one GPU");
    assert_eq!(
        fields[1], L3A_WINDOWS_GPU_NAME,
        "Windows GPU inventory changed"
    );
    assert_eq!(
        fields[2], L3A_WINDOWS_GPU_PCI_BUS_ID,
        "nvidia-smi PCI identity differs from the Windows inventory"
    );
    assert_eq!(
        fields[3], L3A_WINDOWS_GPU_UUID,
        "nvidia-smi UUID differs from the Windows inventory"
    );
    ExpectedGpu {
        pci_bus_id: fields[2].into(),
        uuid: fields[3].into(),
    }
}

fn assert_gpu_evidence(evidence: &GpuEvidence, expected: &ExpectedGpu, pid: u32, log_path: &Path) {
    assert_eq!(evidence.backend, "CUDA0");
    assert_eq!(evidence.pci_bus_id, expected.pci_bus_id);
    assert_eq!(evidence.uuid, expected.uuid);
    assert_eq!(evidence.pid, pid);
    assert_eq!(evidence.asr_log_sha256, sha256(log_path));
    let log = fs::read_to_string(log_path).unwrap();
    assert!(
        log.lines()
            .any(|line| line.starts_with("[asr]") && line.contains("backend=CUDA0"))
    );
}

fn nvidia_smi_activity() -> Option<GpuSample> {
    let sampled_at = Instant::now();
    let output = Command::new("nvidia-smi")
        .args([
            "--query-gpu=timestamp,utilization.gpu,memory.used",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8(output.stdout).ok()?;
    let fields = stdout.trim().split(',').map(str::trim).collect::<Vec<_>>();
    if fields.len() != 3 {
        return None;
    }
    Some(GpuSample {
        timestamp: fields[0].to_owned(),
        utilization_gpu: fields[1].parse().ok()?,
        memory_used_mib: fields[2].parse().ok()?,
        sampled_at,
    })
}

fn start_gpu_sampler() -> (
    Arc<AtomicBool>,
    Arc<Mutex<Vec<GpuSample>>>,
    std::thread::JoinHandle<()>,
) {
    let stop = Arc::new(AtomicBool::new(false));
    let samples = Arc::new(Mutex::new(Vec::new()));
    let thread_stop = Arc::clone(&stop);
    let thread_samples = Arc::clone(&samples);
    let handle = std::thread::spawn(move || {
        while !thread_stop.load(Ordering::Acquire) {
            if let Some(sample) = nvidia_smi_activity()
                && let Ok(mut samples) = thread_samples.lock()
            {
                samples.push(sample);
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    });
    (stop, samples, handle)
}

fn stop_gpu_sampler(
    sampler: (
        Arc<AtomicBool>,
        Arc<Mutex<Vec<GpuSample>>>,
        std::thread::JoinHandle<()>,
    ),
) -> Vec<GpuSample> {
    sampler.0.store(true, Ordering::Release);
    sampler.2.join().expect("GPU sampler exits");
    Arc::try_unwrap(sampler.1)
        .expect("GPU sample storage has one owner")
        .into_inner()
        .expect("GPU sample lock is not poisoned")
}

fn cuda_dll_hashes(native_dir: &Path) -> BTreeMap<String, String> {
    [
        "nemo_speech_asr_c.dll",
        "nemo_speech_asr.dll",
        "ggml.dll",
        "ggml-base.dll",
        "ggml-cpu.dll",
        "ggml-cuda.dll",
        "cudart64_12.dll",
        "cublas64_12.dll",
        "cublasLt64_12.dll",
    ]
    .into_iter()
    .map(|name| (name.to_owned(), sha256(&native_dir.join(name))))
    .collect()
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
            asr_log_path: None,
            spawned_at: None,
        }
    }

    fn start_gpu(
        worker: &Path,
        model: &Path,
        model_sha: &str,
        native_dir: &Path,
        runtime: &Path,
    ) -> Self {
        ensure_under(native_dir, runtime);
        let logs = runtime.join("logs");
        fs::create_dir_all(&logs).unwrap();
        let log_path = logs.join(format!("gpu-asr-{}.log", std::process::id()));
        ensure_under(&log_path, runtime);
        let stderr_file = fs::File::create(&log_path).unwrap();
        let spawned_at = Instant::now();
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
            .env("PARAKEET_L3_ASR_LOG", &log_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::from(stderr_file))
            .spawn()
            .unwrap();
        Self {
            stdin: child.stdin.take().unwrap(),
            stdout: BufReader::new(child.stdout.take().unwrap()),
            child,
            asr_log_path: Some(log_path),
            spawned_at: Some(spawned_at),
        }
    }

    fn start_reference(
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
                "--qualification-reference",
            ])
            .env("PARAKEET_L2_RUNTIME_ROOT", runtime)
            .env("PARAKEET_R6_CASE", "en60")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        Self {
            stdin: child.stdin.take().unwrap(),
            stdout: BufReader::new(child.stdout.take().unwrap()),
            child,
            asr_log_path: None,
            spawned_at: None,
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
                identity: Box::new(identity.clone()),
                source_path: source.to_string_lossy().into_owned(),
                source_sha256: source_sha256.into(),
                expected_gpu: None,
            },
        ))
    }

    fn request_gpu(
        &mut self,
        request_id: u64,
        job_id: &str,
        identity: &WorkerIdentity,
        source: &Path,
        gpu: &ExpectedGpu,
    ) -> GpuCall {
        let request_started_at = Instant::now();
        let frame = json_frame(
            PARAKEET_IPC_VERSION,
            WorkerCommand::Transcribe {
                request_id,
                job_id: job_id.into(),
                generation: 1,
                attempt: 1,
                identity: Box::new(identity.clone()),
                source_path: source.to_string_lossy().into_owned(),
                source_sha256: sha256(source),
                expected_gpu: Some(gpu.clone()),
            },
        );
        self.stdin.write_all(&frame).unwrap();
        self.stdin.flush().unwrap();
        let started = self.receive();
        let native_call_started_at = Instant::now();
        assert!(
            matches!(started.message, Event::NativeCallStarted { request_id: actual, job_id: ref actual_job_id, generation: 1, attempt: 1, pid } if actual == request_id && actual_job_id == job_id && pid == self.child.id()),
            "worker did not report the native call before returning a result: {started:?}"
        );
        let event = self.receive();
        assert!(
            matches!(&event.message, Event::Result { request_id: actual_request_id, job_id: actual_job_id, generation: 1, attempt: 1, .. } if *actual_request_id == request_id && actual_job_id.as_str() == job_id),
            "native call marker was not followed by exactly its correlated result: {event:?}"
        );
        let result_received_at = Instant::now();
        GpuCall {
            event,
            request_started_at,
            native_call_started_at,
            result_received_at,
        }
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
