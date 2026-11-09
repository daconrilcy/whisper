use super::*;
use std::thread;
use whisper_core::{
    ComputeChoice, JobConfig, LanguageChoice, parakeet_session::TranscriptionStrategy,
    ports::ImportRequest,
};

#[cfg(feature = "personal-qualification")]
const CHILD_ROOT: &str = "WHISPER_POLL_PARAKEET_EQ_CHILD_ROOT";
#[cfg(feature = "personal-qualification")]
const GPU_READY_CHILD_ROOT: &str = "WHISPER_POLL_PARAKEET_GPU_READY_CHILD_ROOT";
const Q03_JOB_ID: JobId = JobId(73_221);

fn compile_worker(root: &Path, name: &str, source: &str) -> PathBuf {
    let source_path = root.join(format!("{name}.rs"));
    let executable = root.join(format!("{name}.exe"));
    fs::write(&source_path, source).expect("write Parakeet worker fixture");
    let output = Command::new("rustc")
        .args([
            "--edition=2021",
            "--crate-name",
            name.replace('-', "_").as_str(),
        ])
        .arg(source_path)
        .arg("-o")
        .arg(&executable)
        .output()
        .expect("compile Parakeet worker fixture");
    assert!(
        output.status.success(),
        "rustc {name}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    executable
}

fn compile_scripted_worker(
    worker_root: &Path,
    name: &str,
    initial_frames: &[String],
    transcribe_frame: Option<&str>,
    shutdown_frame: Option<&str>,
) -> PathBuf {
    let initial = serde_json::to_string(initial_frames).expect("encode initial IPC frames");
    let transcribe = serde_json::to_string(&transcribe_frame.unwrap_or_default())
        .expect("encode transcribe IPC frame");
    let shutdown = serde_json::to_string(&shutdown_frame.unwrap_or_default())
        .expect("encode shutdown IPC frame");
    let source = r###"
use std::io::{self, BufRead, Write};
const INITIAL: &[&str] = &__INITIAL__;
const TRANSCRIBE: &str = __TRANSCRIBE__;
const SHUTDOWN: &str = __SHUTDOWN__;
fn main() {
    for frame in INITIAL { println!("{}", frame); }
    let _ = io::stdout().flush();
    for line in io::stdin().lock().lines().flatten() {
        if line.contains("\"type\":\"transcribe\"") && !TRANSCRIBE.is_empty() {
            println!("{}", TRANSCRIBE);
            let _ = io::stdout().flush();
        }
        if line.contains("\"type\":\"shutdown\"") {
            if !SHUTDOWN.is_empty() { println!("{}", SHUTDOWN); }
            let _ = io::stdout().flush();
            return;
        }
    }
}
"###
    .replace("__INITIAL__", &initial)
    .replace("__TRANSCRIBE__", &transcribe)
    .replace("__SHUTDOWN__", &shutdown);
    compile_worker(worker_root, name, &source)
}

fn compile_native_started_worker(
    worker_root: &Path,
    name: &str,
    request: &ImportRequest,
    pid_marker: &Path,
    pid_adjustment: i32,
) -> PathBuf {
    let raw_frame = serde_json::json!({
        "version": 1,
        "message": {
            "type": "native_call_started",
            "request_id": 1,
            "job_id": format!("{:032x}", request.job_id.0),
            "generation": request.generation.get(),
            "attempt": 0,
            "pid": "__PID__",
        }
    })
    .to_string()
    .replace("\"pid\":\"__PID__\"", "\"pid\":__PID__");
    let frame = serde_json::to_string(&raw_frame).expect("encode native-call frame source");
    let marker = serde_json::to_string(&pid_marker.to_string_lossy())
        .expect("encode native-call PID marker path");
    let source = r###"
use std::io::{self, BufRead, Write};
const FRAME: &str = __FRAME__;
const PID_MARKER: &str = __PID_MARKER__;
const PID_ADJUSTMENT: i32 = __PID_ADJUSTMENT__;
fn main() {
    let reported_pid = (std::process::id() as i64 + i64::from(PID_ADJUSTMENT)) as u32;
    let frame = FRAME.replace("__PID__", &reported_pid.to_string());
    println!("{frame}");
    let _ = io::stdout().flush();
    std::fs::write(PID_MARKER, frame).expect("write emitted NativeCallStarted frame");
    for line in io::stdin().lock().lines().flatten() {
        if line.contains("\"type\":\"shutdown\"") { return; }
    }
}
"###
    .replace("__FRAME__", &frame)
    .replace("__PID_MARKER__", &marker)
    .replace("__PID_ADJUSTMENT__", &pid_adjustment.to_string());
    compile_worker(worker_root, name, &source)
}

#[cfg(feature = "personal-qualification")]
fn compile_worker_relative(
    root: &Path,
    crate_name: &str,
    source_name: &str,
    executable_name: &str,
    source: &str,
) -> PathBuf {
    fs::write(root.join(source_name), source).expect("write relative worker source");
    let output = Command::new("rustc")
        .current_dir(root)
        .args([
            "--edition=2021",
            "-Cdebuginfo=0",
            "-C",
            "link-arg=/Brepro",
            "--crate-name",
            crate_name,
        ])
        .arg(source_name)
        .arg("-o")
        .arg(executable_name)
        .output()
        .expect("compile relative worker fixture");
    assert!(
        output.status.success(),
        "rustc {crate_name}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    root.join(executable_name)
}

fn personal_import_request(root: &Path, model: &Path) -> ImportRequest {
    let source = root.join("source.wav");
    let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source)
        .expect("create one-second Parakeet PCM source");
    writer
        .append(&vec![0_i16; 16_000])
        .expect("write synthetic source samples");
    writer.finish().expect("close source WAV");
    let model_record =
        whisper_core::model_registry::model(whisper_core::model_catalog::ModelId::ParakeetTdt06BV3);
    let model_sha256 = (0..32)
        .map(|index| {
            u8::from_str_radix(&model_record.artifact.sha256[index * 2..index * 2 + 2], 16)
                .expect("registered model SHA is hexadecimal")
        })
        .collect::<Vec<_>>()
        .try_into()
        .expect("registered model SHA has 32 bytes");
    ImportRequest {
        strategy: TranscriptionStrategy::LegacyV1,
        job_id: Q03_JOB_ID,
        generation: Generation::first(),
        source_path: source.to_string_lossy().into_owned(),
        source_sha256: Some(crate::archive::sha256_file(&source).expect("hash synthetic source")),
        source_samples: Some(16_000),
        model_path: model.to_string_lossy().into_owned(),
        model_sha256,
        destination: root.join("archive").to_string_lossy().into_owned(),
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Auto,
        },
    }
}

fn parakeet_test_identity(profile: &str) -> serde_json::Value {
    let key = crate::archive::ArchiveQualificationKey::parakeet_cpu_test();
    serde_json::json!({
        "model_sha256": key.artifact_revision,
        "runtime_sha256": key.runtime_build,
        "build_sha256": key.worker_build,
        "profile": profile,
    })
}

fn parakeet_ready_frame(identity: serde_json::Value, backend: &str) -> String {
    serde_json::json!({
        "version": 1,
        "message": {
            "type": "ready",
            "identity": identity,
            "backend": backend,
            "native_version": "fixture",
        }
    })
    .to_string()
}

fn parakeet_failed_frame(message: serde_json::Value) -> String {
    serde_json::json!({ "version": 1, "message": message }).to_string()
}

fn parakeet_result_frame(
    request: &ImportRequest,
    identity: serde_json::Value,
    words: serde_json::Value,
) -> String {
    serde_json::json!({
        "version": 1,
        "message": {
            "type": "result",
            "request_id": 1,
            "job_id": format!("{:032x}", request.job_id.0),
            "generation": request.generation.get(),
            "attempt": 0,
            "identity": identity,
            "source_sha256": request.source_sha256.as_ref().expect("fixture source hash")
                .iter().map(|byte| format!("{byte:02x}")).collect::<String>(),
            "duration_samples": request.source_samples.expect("fixture duration"),
            "text": "fixture",
            "words": words,
            "native_inference_ms": 0,
        }
    })
    .to_string()
}

fn one_window_vad_run(request: ImportRequest) -> crate::worker_ipc::runtime::ParakeetVadRun {
    crate::worker_ipc::runtime::ParakeetVadRun {
        request: request.clone(),
        decoder: None,
        decode_offset: 16_000,
        total_samples: 16_000,
        source_sha256: request
            .source_sha256
            .as_ref()
            .unwrap()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
        pcm: vec![0; 16_000],
        windows: vec![whisper_core::parakeet_windows::Window {
            index: 0,
            central_start_sample: 0,
            central_end_sample: 16_000,
            native_start_sample: 0,
            native_end_sample: 16_000,
            cut: None,
        }],
        next_window: 0,
        next_segment_sequence: 1,
        request_count: 1,
        current_attempt: 0,
        current_path: None,
        current_source_sha256: request
            .source_sha256
            .as_ref()
            .map(|sha| sha.iter().map(|byte| format!("{byte:02x}")).collect()),
        pending_confirmation: None,
        worker_ready: true,
        ready_event_sent: true,
    }
}

fn request_runtime(root: &Path, job_id: JobId) -> (ImportRequest, ImportRuntime) {
    let model = root.join("model.gguf");
    fs::write(&model, b"synthetic model path").expect("create synthetic model");
    let mut request = personal_import_request(root, &model);
    request.job_id = job_id;
    let mut runtime = runtime_with_active_attempt(job_id);
    runtime.active.as_mut().unwrap().import_request = Some(request.clone());
    runtime.parakeet_paths = Some(synthetic_parakeet_paths(compile_worker(
        &synthetic_worker_root(root),
        "unused-parakeet-worker",
        "fn main() {}",
    )));
    (request, runtime)
}

fn attach_scripted_worker(
    root: &Path,
    runtime: &mut ImportRuntime,
    name: &str,
    initial_frames: &[String],
    transcribe_frame: Option<&str>,
    shutdown_frame: Option<&str>,
) {
    let worker = compile_scripted_worker(
        &synthetic_worker_root(root),
        name,
        initial_frames,
        transcribe_frame,
        shutdown_frame,
    );
    let paths = synthetic_parakeet_paths(worker);
    runtime.parakeet_paths = Some(paths.clone());
    runtime.parakeet = Some(
        crate::parakeet_import::ParakeetProcess::spawn_live(&paths, root)
            .expect("spawn scripted Parakeet worker"),
    );
}

struct FailedPollFixture {
    runtime: ImportRuntime,
    trace: Vec<serde_json::Value>,
    diagnostics: Vec<serde_json::Value>,
    result: Result<WorkerEvent, PortError>,
}

fn run_failed_poll_fixture(
    root: &Path,
    name: &str,
    job_id: JobId,
    backend: BackendKind,
    compute: ComputeChoice,
    native_call_started: bool,
    frames: &[String],
) -> FailedPollFixture {
    let (_, mut runtime) = request_runtime(root, job_id);
    let active = runtime.active.as_mut().unwrap();
    active.backend = backend;
    active.gpu_attested = backend == BackendKind::Cuda && native_call_started;
    active.import_request.as_mut().unwrap().config.compute = compute;
    runtime.parakeet_native_call_started = native_call_started;
    attach_scripted_worker(root, &mut runtime, name, frames, None, None);
    let mut trace = vec![serde_json::json!({
        "phase": "failed_fixture_started",
        "scenario": {
            "backend": format!("{backend:?}"),
            "compute": format!("{compute:?}"),
            "attempt": 0,
            "gpu_attested": backend == BackendKind::Cuda && native_call_started,
            "native_call_started": native_call_started,
        },
        "state": poll_state_snapshot(&runtime),
        "input_frames": frames,
    })];
    let mut diagnostics = Vec::new();
    let first = poll_traced_until_event(&mut runtime, &mut trace, &mut diagnostics, "first_event");
    let result = if matches!(&first, Ok(WorkerEvent::Ready { .. })) {
        poll_traced_until_event(&mut runtime, &mut trace, &mut diagnostics, "failed_event")
    } else {
        first
    };
    FailedPollFixture {
        runtime,
        trace,
        diagnostics,
        result,
    }
}

fn synthetic_worker_root(root: &Path) -> PathBuf {
    let worker_root = root.join("synthetic-runtime/worker");
    fs::create_dir_all(&worker_root).expect("create synthetic worker directory");
    worker_root
}

fn synthetic_parakeet_paths(worker: PathBuf) -> crate::parakeet_import::ParakeetWorkerPaths {
    let runtime_root = worker
        .parent()
        .and_then(Path::parent)
        .expect("synthetic worker is beneath its runtime")
        .to_path_buf();
    let native_dir = runtime_root.join("native");
    let model = runtime_root.join("model.gguf");
    fs::create_dir_all(&native_dir).expect("create synthetic native directory");
    fs::write(&model, b"synthetic model path").expect("create synthetic model path");
    crate::parakeet_import::ParakeetWorkerPaths {
        executable: worker,
        native_dir,
        model,
        runtime_root,
    }
}

struct PipeHoldFiles {
    ready: PathBuf,
    release: PathBuf,
    finished: PathBuf,
}

struct PipeHoldRelease(PipeHoldFiles);

impl Drop for PipeHoldRelease {
    fn drop(&mut self) {
        let _ = fs::write(&self.0.release, b"release");
        let _ = wait_for_marker(&self.0.finished, Duration::from_secs(3));
    }
}

fn wait_for_marker(path: &Path, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while !path.is_file() && Instant::now() < deadline {
        thread::yield_now();
    }
    path.is_file()
}

fn wait_for_parakeet_exit(
    process: &mut crate::parakeet_import::ParakeetProcess,
    timeout: Duration,
) -> Option<std::process::ExitStatus> {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        match process.try_wait().expect("wait for fake worker exit") {
            Some(status) => return Some(status),
            None => thread::yield_now(),
        }
    }
    None
}

fn pipe_hold_files(root: &Path, name: &str) -> PipeHoldFiles {
    PipeHoldFiles {
        ready: root.join(format!("{name}-ready")),
        release: root.join(format!("{name}-release")),
        finished: root.join(format!("{name}-finished")),
    }
}

fn compile_pipe_holding_eof_worker(
    worker_root: &Path,
    name: &str,
    files: &PipeHoldFiles,
    exit_code: i32,
) -> PathBuf {
    let source = r###"
use std::{
    env,
    fs,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
const READY: &str = __READY__;
const RELEASE: &str = __RELEASE__;
const FINISHED: &str = __FINISHED__;
const EXIT_CODE: i32 = __EXIT_CODE__;
fn wait_for(path: &str, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while !Path::new(path).is_file() && Instant::now() < deadline {
        thread::yield_now();
    }
    Path::new(path).is_file()
}
fn main() {
    if env::args().nth(1).as_deref() == Some("--hold-stdout") {
        fs::write(READY, b"ready").expect("signal pipe holder ready");
        let _ = wait_for(RELEASE, Duration::from_secs(12));
        fs::write(FINISHED, b"finished").expect("signal pipe holder finished");
        return;
    }
    let executable = env::current_exe().expect("worker executable path");
    let mut holder = Command::new(executable)
        .arg("--hold-stdout")
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn stdout pipe holder");
    if !wait_for(READY, Duration::from_secs(8)) {
        let _ = holder.kill();
        std::process::exit(97);
    }
    drop(holder);
    std::process::exit(EXIT_CODE);
}
"###
    .replace(
        "__READY__",
        &serde_json::to_string(files.ready.to_string_lossy().as_ref()).unwrap(),
    )
    .replace(
        "__RELEASE__",
        &serde_json::to_string(files.release.to_string_lossy().as_ref()).unwrap(),
    )
    .replace(
        "__FINISHED__",
        &serde_json::to_string(files.finished.to_string_lossy().as_ref()).unwrap(),
    )
    .replace("__EXIT_CODE__", &exit_code.to_string());
    compile_worker(worker_root, name, &source)
}

const NOMINAL_IMPORT_WORKER_SOURCE: &str = r###"
use std::{env, fs::{self, OpenOptions}, io::{self, BufRead, Write}, path::Path, process::{Command, Stdio}, thread, time::{Duration, Instant}};
const READY: &str = __READY__;
const RESULT: &str = __RESULT__;
const COMMANDS: &str = __COMMANDS__;
const ACKNOWLEDGE_STOP: bool = __ACKNOWLEDGE_STOP__;
const HOLD_READY: &str = __HOLD_READY__;
const HOLD_RELEASE: &str = __HOLD_RELEASE__;
const HOLD_FINISHED: &str = __HOLD_FINISHED__;
const SHUTDOWN_MARKER: &str = __SHUTDOWN_MARKER__;
fn wait_for(path: &str, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while !Path::new(path).is_file() && Instant::now() < deadline { thread::yield_now(); }
    Path::new(path).is_file()
}
fn main() {
    if env::args().nth(1).as_deref() == Some("--hold-stdout") {
        fs::write(HOLD_READY, b"ready").expect("signal holder ready");
        let _ = wait_for(HOLD_RELEASE, Duration::from_secs(12));
        fs::write(HOLD_FINISHED, b"finished").expect("signal holder finished");
        return;
    }
    if !HOLD_READY.is_empty() {
        let executable = env::current_exe().expect("worker executable");
        let holder = Command::new(executable).arg("--hold-stdout")
            .stdin(Stdio::null()).stdout(Stdio::inherit()).stderr(Stdio::null())
            .spawn().expect("spawn stdout pipe holder");
        assert!(wait_for(HOLD_READY, Duration::from_secs(8)), "stdout holder ready");
        drop(holder);
    }
    println!("{READY}");
    let _ = io::stdout().flush();
    for line in io::stdin().lock().lines().flatten() {
        let mut file = OpenOptions::new().create(true).append(true).open(COMMANDS).expect("command trace");
        writeln!(file, "{line}").expect("record command");
        if line.contains("\"type\":\"transcribe\"") {
            println!("{RESULT}");
            let _ = io::stdout().flush();
        } else if line.contains("\"type\":\"shutdown\"") {
            if !SHUTDOWN_MARKER.is_empty() {
                fs::write(SHUTDOWN_MARKER, b"shutdown").expect("signal shutdown received");
            }
            if ACKNOWLEDGE_STOP {
                println!("{}", r#"{"version":1,"message":{"type":"stopped"}}"#);
                let _ = io::stdout().flush();
            }
            return;
        }
    }
}
"###;

fn compile_nominal_import_worker(
    worker_root: &Path,
    request: &ImportRequest,
    source_sha256: &str,
    acknowledge_stop: bool,
    pipe_hold: Option<&PipeHoldFiles>,
    shutdown_marker: Option<&Path>,
) -> PathBuf {
    let key = crate::archive::ArchiveQualificationKey::parakeet_cpu_test();
    let identity = serde_json::json!({
        "model_sha256": key.artifact_revision,
        "runtime_sha256": key.runtime_build,
        "build_sha256": key.worker_build,
        "profile": key.inference_profile,
    });
    let ready = serde_json::json!({
        "version": 1,
        "message": {
            "type": "ready",
            "identity": identity.clone(),
            "backend": "CPU",
            "native_version": "fixture",
        }
    });
    let result = serde_json::json!({
        "version": 1,
        "message": {
            "type": "result",
            "request_id": 1,
            "job_id": format!("{:032x}", request.job_id.0),
            "generation": request.generation.get(),
            "attempt": 0,
            "identity": identity,
            "source_sha256": source_sha256,
            "duration_samples": request.source_samples.expect("fixture duration"),
            "text": "hello.",
            "words": [{ "text": "hello.", "start_ms": 0, "end_ms": 500 }],
            "native_inference_ms": 0,
        }
    });
    let ready_literal = serde_json::to_string(&ready.to_string()).expect("encode READY frame");
    let result_literal = serde_json::to_string(&result.to_string()).expect("encode RESULT frame");
    let commands = worker_root.join("commands.jsonl");
    let commands_literal = serde_json::to_string(&commands.to_string_lossy().to_string())
        .expect("encode command trace path");
    let acknowledge_stop = acknowledge_stop.to_string();
    let hold_ready = pipe_hold.map_or_else(String::new, |files| {
        files.ready.to_string_lossy().into_owned()
    });
    let hold_release = pipe_hold.map_or_else(String::new, |files| {
        files.release.to_string_lossy().into_owned()
    });
    let hold_finished = pipe_hold.map_or_else(String::new, |files| {
        files.finished.to_string_lossy().into_owned()
    });
    let hold_ready_literal = serde_json::to_string(&hold_ready).expect("encode hold ready path");
    let hold_release_literal =
        serde_json::to_string(&hold_release).expect("encode hold release path");
    let hold_finished_literal =
        serde_json::to_string(&hold_finished).expect("encode hold finished path");
    let shutdown_marker =
        shutdown_marker.map_or_else(String::new, |path| path.to_string_lossy().into_owned());
    let shutdown_literal =
        serde_json::to_string(&shutdown_marker).expect("encode shutdown marker path");
    let source = NOMINAL_IMPORT_WORKER_SOURCE
        .replace("__READY__", &ready_literal)
        .replace("__RESULT__", &result_literal)
        .replace("__COMMANDS__", &commands_literal)
        .replace("__ACKNOWLEDGE_STOP__", &acknowledge_stop)
        .replace("__HOLD_READY__", &hold_ready_literal)
        .replace("__HOLD_RELEASE__", &hold_release_literal)
        .replace("__HOLD_FINISHED__", &hold_finished_literal)
        .replace("__SHUTDOWN_MARKER__", &shutdown_literal);
    compile_worker(worker_root, "parakeet-nominal-import", &source)
}

fn poll_state_snapshot(runtime: &ImportRuntime) -> serde_json::Value {
    let active = runtime.active.as_ref();
    serde_json::json!({
        "active": active.map(|state| serde_json::json!({
            "job_id": format!("{:032x}", state.job_id.0),
            "generation": state.generation.get(),
            "instance_id": state.instance_id,
            "backend": format!("{:?}", state.backend),
            "attempt": state.attempt_number,
            "gpu_attested": state.gpu_attested,
        })),
        "process": runtime.parakeet.is_some(),
        "stopped_ack": runtime.parakeet_stopped_ack,
        "end": runtime.parakeet_end.map(|(job_id, generation, sequence, offset)| serde_json::json!([
            format!("{:032x}", job_id.0), generation.get(), sequence, offset
        ])),
        "queued_events": runtime.parakeet_events.iter().map(|event| format!("{event:?}")).collect::<Vec<_>>(),
        "vad": runtime.parakeet_vad.is_some(),
        "native_call_started": runtime.parakeet_native_call_started,
        "fallback_reason": runtime.auto_fallback_reason.as_deref(),
    })
}

fn record_poll_result(
    runtime: &mut ImportRuntime,
    trace: &mut Vec<serde_json::Value>,
    phase: &str,
) -> Option<Result<WorkerEvent, PortError>> {
    let before = poll_state_snapshot(runtime);
    let result = runtime.poll_parakeet_worker();
    let after = poll_state_snapshot(runtime);
    trace.push(serde_json::json!({
        "phase": phase,
        "before": before,
        "result": result.as_ref().map(|value| format!("{value:?}")),
        "after": after,
    }));
    result
}

fn poll_traced_until_event(
    runtime: &mut ImportRuntime,
    trace: &mut Vec<serde_json::Value>,
    helper_diagnostics: &mut Vec<serde_json::Value>,
    phase: &str,
) -> Result<WorkerEvent, PortError> {
    trace.push(serde_json::json!({
        "phase": format!("{phase}_wait_started"),
        "observation": "fixed_helper_boundary",
    }));
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut unchanged_wait_polls = 0_u64;
    let mut previous = poll_state_snapshot(runtime);
    while Instant::now() < deadline {
        let before = poll_state_snapshot(runtime);
        let result = runtime.poll_parakeet_worker();
        let after = poll_state_snapshot(runtime);
        if let Some(result) = result {
            helper_diagnostics.push(serde_json::json!({
                "phase": phase,
                "polls_without_state_change": unchanged_wait_polls,
            }));
            trace.push(serde_json::json!({
                "phase": phase,
                "before": before,
                "result": format!("{result:?}"),
                "after": after,
            }));
            return result;
        }
        if before != after || previous != after {
            trace.push(serde_json::json!({
                "phase": format!("{phase}_no_event_transition"),
                "result": null,
                "before": before,
                "after": after,
            }));
            unchanged_wait_polls = 0;
        } else {
            unchanged_wait_polls = unchanged_wait_polls.saturating_add(1);
        }
        previous = after;
        thread::yield_now();
    }
    panic!("timed out polling {phase}");
}

fn assert_q06_ready_and_result_events(
    runtime: &mut ImportRuntime,
    trace: &mut Vec<serde_json::Value>,
    diagnostics: &mut Vec<serde_json::Value>,
) {
    assert!(matches!(
        poll_traced_until_event(runtime, trace, diagnostics, "ready"),
        Ok(WorkerEvent::Ready { .. })
    ));
    assert!(matches!(
        poll_traced_until_event(runtime, trace, diagnostics, "result_progress"),
        Ok(WorkerEvent::Progress {
            completed_samples: 16_000,
            total_samples: 16_000,
            ..
        })
    ));
    assert!(matches!(
        poll_traced_until_event(runtime, trace, diagnostics, "result_segment"),
        Ok(WorkerEvent::Segment(_))
    ));
}

fn trace_root_forms(root: &Path) -> Vec<String> {
    let root_text = root.to_string_lossy().into_owned();
    let forward_slashes = root_text.replace('\\', "/");
    let escaped = serde_json::to_string(&root_text).expect("encode escaped trace root");
    let escaped = escaped
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .expect("quoted JSON root");
    let escaped_forward =
        serde_json::to_string(&forward_slashes).expect("encode escaped forward-slash root");
    let escaped_forward = escaped_forward
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .expect("quoted JSON root");
    let mut forms = vec![
        root_text,
        forward_slashes,
        escaped.to_owned(),
        escaped_forward.to_owned(),
    ];
    for form in forms.clone() {
        let nested = serde_json::to_string(&form).expect("encode nested trace root");
        forms.push(
            nested
                .strip_prefix('"')
                .and_then(|value| value.strip_suffix('"'))
                .expect("quoted nested root")
                .to_owned(),
        );
    }
    forms
}

fn replace_trace_roots(value: &mut serde_json::Value, root: &Path) {
    let forms = trace_root_forms(root);
    match value {
        serde_json::Value::String(text) => {
            for root_text in &forms {
                *text = text.replace(root_text.as_str(), "<ROOT>");
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                replace_trace_roots(value, root);
            }
        }
        serde_json::Value::Object(values) => {
            for value in values.values_mut() {
                replace_trace_roots(value, root);
            }
        }
        _ => {}
    }
}

fn assert_frozen_direct_trace(label: &str, stable: &[u8]) {
    let references: std::collections::BTreeMap<String, String> = serde_json::from_str(
        include_str!("../../../../../tests/fixtures/poll_parakeet_equivalence/direct-traces.json"),
    )
    .expect("decode frozen direct Parakeet poll traces");
    let expected = references
        .get(label)
        .unwrap_or_else(|| panic!("missing frozen direct poll trace {label}"));
    assert_eq!(
        stable,
        expected.as_bytes(),
        "direct Parakeet poll trace changed for {label}"
    );
}

fn assert_frozen_q03_trace(stable: &[u8]) {
    assert_eq!(
        stable,
        include_bytes!(
            "../../../../../tests/fixtures/poll_parakeet_equivalence/q03-qualified-cpu-fallback.json"
        ),
        "qualified CPU fallback trace changed for Q03"
    );
}

fn assert_frozen_added_trace(label: &str, stable: &[u8]) {
    let references: std::collections::BTreeMap<String, String> =
        serde_json::from_str(include_str!(
            "../../../../../tests/fixtures/poll_parakeet_equivalence/added-direct-traces.json"
        ))
        .expect("decode frozen additional direct Parakeet traces");
    let expected = references
        .get(label)
        .unwrap_or_else(|| panic!("missing frozen additional direct trace {label}"));
    assert_eq!(
        stable,
        expected.as_bytes(),
        "additional direct Parakeet poll trace changed for {label}"
    );
}

fn assert_frozen_native_call_trace(label: &str, stable: &[u8]) {
    let references: std::collections::BTreeMap<String, String> = serde_json::from_str(
        include_str!("../../../../../tests/fixtures/poll_parakeet_equivalence/native-call-stopped-traces.json"),
    )
    .expect("decode frozen NativeCallStarted and Stopped traces");
    let expected = references
        .get(label)
        .unwrap_or_else(|| panic!("missing frozen native-state trace {label}"));
    assert_eq!(
        stable,
        expected.as_bytes(),
        "native-state poll trace changed for {label}"
    );
}

fn assert_frozen_failed_path_trace(label: &str, stable: &[u8]) {
    let references: std::collections::BTreeMap<String, String> =
        serde_json::from_str(include_str!(
            "../../../../../tests/fixtures/poll_parakeet_equivalence/failed-path-traces.json"
        ))
        .expect("decode frozen Failed routing traces");
    let expected = references
        .get(label)
        .unwrap_or_else(|| panic!("missing frozen Failed routing trace {label}"));
    assert_eq!(
        stable,
        expected.as_bytes(),
        "Failed routing trace changed for {label}"
    );
}

fn assert_frozen_pending_result_trace(label: &str, stable: &[u8]) {
    let references: std::collections::BTreeMap<String, String> =
        serde_json::from_str(include_str!(
            "../../../../../tests/fixtures/poll_parakeet_equivalence/pending-result-traces.json"
        ))
        .expect("decode frozen pending Result traces");
    let expected = references
        .get(label)
        .unwrap_or_else(|| panic!("missing frozen pending Result trace {label}"));
    assert_eq!(
        stable,
        expected.as_bytes(),
        "pending Result trace changed for {label}"
    );
}

fn assert_frozen_gpu_ready_trace(label: &str, stable: &[u8]) {
    let references: std::collections::BTreeMap<String, String> =
        serde_json::from_str(include_str!(
            "../../../../../tests/fixtures/poll_parakeet_equivalence/gpu-ready-traces.json"
        ))
        .expect("decode frozen GPU Ready traces");
    let expected = references
        .get(label)
        .unwrap_or_else(|| panic!("missing frozen GPU Ready trace {label}"));
    assert_eq!(
        stable,
        expected.as_bytes(),
        "GPU Ready trace changed for {label}"
    );
}

#[cfg(feature = "personal-qualification")]
fn assert_frozen_personal_gpu_ready_trace(label: &str, stable: &[u8]) {
    let references: std::collections::BTreeMap<String, String> =
        serde_json::from_str(include_str!(
            "../../../../../tests/fixtures/poll_parakeet_equivalence/personal-gpu-ready-traces.json"
        ))
        .expect("decode frozen personal GPU Ready traces");
    let expected = references
        .get(label)
        .unwrap_or_else(|| panic!("missing frozen personal GPU Ready trace {label}"));
    assert_eq!(
        stable,
        expected.as_bytes(),
        "personal GPU Ready trace changed for {label}"
    );
}

fn write_poll_trace(
    root: &Path,
    label: &str,
    trace: &[serde_json::Value],
    helper_diagnostics: &[serde_json::Value],
) {
    let parent = root.parent().expect("test root has TEMP parent");
    let root_name = root.file_name().expect("test root name").to_string_lossy();
    let raw_path = parent.join(format!("poll-parakeet-{label}-{root_name}-raw.json"));
    let stable_path = parent.join(format!("poll-parakeet-{label}-{root_name}-stable.json"));
    let raw_value = serde_json::json!({
        "trace": trace,
        "helper_diagnostics": helper_diagnostics,
    });
    let raw = serde_json::to_vec_pretty(&raw_value).expect("serialize raw poll trace");
    let mut normalized = serde_json::Value::Array(trace.to_vec());
    replace_trace_roots(&mut normalized, root);
    let stable = serde_json::to_vec_pretty(&normalized).expect("serialize stable poll trace");
    let stable_text = String::from_utf8_lossy(&stable);
    for root_form in trace_root_forms(root) {
        assert!(
            !stable_text.contains(&root_form),
            "normalized trace retained temporary root spelling"
        );
    }
    if label == "Q03" {
        assert_frozen_q03_trace(&stable);
    } else {
        assert_frozen_direct_trace(label, &stable);
    }
    fs::write(&raw_path, raw).expect("persist raw poll trace in lot TEMP");
    fs::write(&stable_path, stable).expect("persist normalized poll trace in lot TEMP");
}

fn write_candidate_poll_trace(
    root: &Path,
    label: &str,
    trace: &[serde_json::Value],
    helper_diagnostics: &[serde_json::Value],
) {
    let parent = root.parent().expect("test root has TEMP parent");
    let root_name = root.file_name().expect("test root name").to_string_lossy();
    let raw_path = parent.join(format!("poll-parakeet-{label}-{root_name}-raw.json"));
    let stable_path = parent.join(format!("poll-parakeet-{label}-{root_name}-stable.json"));
    let raw_value = serde_json::json!({ "trace": trace, "helper_diagnostics": helper_diagnostics });
    let raw = serde_json::to_vec_pretty(&raw_value).expect("serialize raw poll trace");
    let mut stable_value = serde_json::Value::Array(trace.to_vec());
    replace_trace_roots(&mut stable_value, root);
    let stable = serde_json::to_vec_pretty(&stable_value).expect("serialize stable poll trace");
    assert_frozen_added_trace(label, &stable);
    fs::write(raw_path, raw).expect("persist raw candidate poll trace in lot TEMP");
    fs::write(stable_path, stable).expect("persist candidate poll trace in lot TEMP");
}

fn write_candidate_native_trace(
    root: &Path,
    label: &str,
    trace: &[serde_json::Value],
    helper_diagnostics: &[serde_json::Value],
) {
    let parent = root.parent().expect("test root has TEMP parent");
    let root_name = root.file_name().expect("test root name").to_string_lossy();
    let raw_path = parent.join(format!("poll-parakeet-{label}-{root_name}-raw.json"));
    let stable_path = parent.join(format!("poll-parakeet-{label}-{root_name}-stable.json"));
    let raw = serde_json::to_vec_pretty(&serde_json::json!({
        "trace": trace,
        "helper_diagnostics": helper_diagnostics,
    }))
    .expect("serialize raw pending trace");
    let mut stable_value = serde_json::Value::Array(trace.to_vec());
    replace_trace_roots(&mut stable_value, root);
    remap_native_call_proof_pids(&mut stable_value);
    let stable = serde_json::to_vec_pretty(&stable_value).expect("serialize stable pending trace");
    assert_frozen_native_call_trace(label, &stable);
    fs::write(raw_path, raw).expect("persist raw pending trace in lot TEMP");
    fs::write(stable_path, stable).expect("persist stable pending trace in lot TEMP");
}

fn write_candidate_failed_trace(
    root: &Path,
    label: &str,
    trace: &[serde_json::Value],
    helper_diagnostics: &[serde_json::Value],
) {
    let parent = root.parent().expect("test root has TEMP parent");
    let root_name = root.file_name().expect("test root name").to_string_lossy();
    let raw_path = parent.join(format!("poll-parakeet-{label}-{root_name}-raw.json"));
    let stable_path = parent.join(format!("poll-parakeet-{label}-{root_name}-stable.json"));
    let raw = serde_json::to_vec_pretty(&serde_json::json!({
        "trace": trace,
        "helper_diagnostics": helper_diagnostics,
    }))
    .expect("serialize raw provisional trace");
    let mut stable_value = serde_json::Value::Array(trace.to_vec());
    replace_trace_roots(&mut stable_value, root);
    let stable =
        serde_json::to_vec_pretty(&stable_value).expect("serialize stable provisional trace");
    assert_frozen_failed_path_trace(label, &stable);
    fs::write(raw_path, raw).expect("persist raw provisional trace in lot TEMP");
    fs::write(stable_path, stable).expect("persist stable provisional trace in lot TEMP");
}

fn write_pending_result_trace(
    root: &Path,
    label: &str,
    trace: &[serde_json::Value],
    helper_diagnostics: &[serde_json::Value],
) {
    let parent = root.parent().expect("test root has TEMP parent");
    let root_name = root.file_name().expect("test root name").to_string_lossy();
    let raw_path = parent.join(format!("poll-parakeet-{label}-{root_name}-raw.json"));
    let stable_path = parent.join(format!("poll-parakeet-{label}-{root_name}-stable.json"));
    let raw = serde_json::to_vec_pretty(&serde_json::json!({
        "trace": trace,
        "helper_diagnostics": helper_diagnostics,
    }))
    .expect("serialize raw pending result trace");
    let mut stable_value = serde_json::Value::Array(trace.to_vec());
    replace_trace_roots(&mut stable_value, root);
    let stable =
        serde_json::to_vec_pretty(&stable_value).expect("serialize stable pending result trace");
    assert_frozen_pending_result_trace(label, &stable);
    fs::write(raw_path, raw).expect("persist raw pending result trace in lot TEMP");
    fs::write(stable_path, stable).expect("persist stable pending result trace in lot TEMP");
}

fn remap_native_call_proof_pids(trace: &mut serde_json::Value) {
    fn remap(value: &mut serde_json::Value, ids: &mut std::collections::BTreeMap<u64, String>) {
        if let Some(pid) = value.as_u64() {
            let next = ids.len() + 1;
            let stable = ids.entry(pid).or_insert_with(|| format!("<PID-{next}>"));
            *value = serde_json::Value::String(stable.clone());
        }
    }

    let Some(entries) = trace.as_array_mut() else {
        return;
    };
    let mut ids = std::collections::BTreeMap::new();
    for entry in entries {
        if entry.get("phase").and_then(serde_json::Value::as_str)
            != Some("native_call_started_event")
        {
            continue;
        }
        if let Some(worker_pid) = entry.get_mut("worker_pid") {
            remap(worker_pid, &mut ids);
        }
        if let Some(reported_pid) = entry.get_mut("reported_pid") {
            remap(reported_pid, &mut ids);
        }
        if let Some(message_pid) = entry
            .get_mut("envelope")
            .and_then(|envelope| envelope.get_mut("message"))
            .and_then(|message| message.get_mut("pid"))
        {
            remap(message_pid, &mut ids);
        }
    }
}

fn poll_until_native_call_state(
    runtime: &mut ImportRuntime,
    trace: &mut Vec<serde_json::Value>,
    diagnostics: &mut Vec<serde_json::Value>,
) -> Option<Result<WorkerEvent, PortError>> {
    trace.push(serde_json::json!({
        "phase": "native_call_started_wait_started",
        "observation": "fixed_helper_boundary",
    }));
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut unchanged_polls = 0_u64;
    while Instant::now() < deadline {
        let before = poll_state_snapshot(runtime);
        let result = runtime.poll_parakeet_worker();
        let after = poll_state_snapshot(runtime);
        if runtime.parakeet_native_call_started || result.is_some() {
            diagnostics.push(serde_json::json!({
                "phase": "native_call_started_wait",
                "polls_without_state_change": unchanged_polls,
            }));
            trace.push(serde_json::json!({
                "phase": "native_call_started_observation",
                "before": before,
                "result": result.as_ref().map(|value| format!("{value:?}")),
                "after": after,
            }));
            return result;
        }
        if before != after {
            unchanged_polls = 0;
        } else {
            unchanged_polls = unchanged_polls.saturating_add(1);
        }
        thread::yield_now();
    }
    panic!("timed out polling NativeCallStarted");
}

#[test]
fn poll_parakeet_cpu_import_records_nominal_commands_and_events() {
    let root = TestRoot::new("parakeet-cpu-import-nominal");
    let worker_root = synthetic_worker_root(&root.0);
    let model = root.0.join("model.gguf");
    fs::write(&model, b"synthetic model path").expect("create synthetic model file");
    let mut request = personal_import_request(&root.0, &model);
    request.config.compute = whisper_core::ComputeChoice::Cpu;
    let source_sha = request
        .source_sha256
        .as_ref()
        .map(|sha| {
            sha.iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        })
        .expect("source hash");
    let worker =
        compile_nominal_import_worker(&worker_root, &request, &source_sha, true, None, None);
    let paths = synthetic_parakeet_paths(worker);
    let mut runtime = runtime_with_active_attempt(request.job_id);
    runtime.parakeet_paths = Some(paths.clone());
    runtime.active.as_mut().unwrap().import_request = Some(request.clone());
    runtime.parakeet = Some(
        crate::parakeet_import::ParakeetProcess::spawn_live(&paths, &root.0)
            .expect("spawn nominal CPU fixture worker"),
    );

    let mut trace = Vec::new();
    let mut helper_diagnostics = Vec::new();
    trace.push(serde_json::json!({
        "phase": "before_first_poll",
        "state": poll_state_snapshot(&runtime),
    }));
    let ready = poll_traced_until_event(&mut runtime, &mut trace, &mut helper_diagnostics, "ready");
    assert!(matches!(
        &ready,
        Ok(WorkerEvent::Ready { job_id, generation, instance_id: 7, backend: BackendKind::Cpu })
            if *job_id == request.job_id && *generation == request.generation
    ));

    let mut terminal = None;
    while terminal.is_none() {
        let event = poll_traced_until_event(
            &mut runtime,
            &mut trace,
            &mut helper_diagnostics,
            "next_import_event",
        );
        let is_terminal = matches!(&event, Ok(WorkerEvent::End { .. }) | Err(_));
        if is_terminal {
            terminal = Some(event);
        }
    }
    assert!(
        matches!(terminal, Some(Ok(WorkerEvent::End { job_id, generation, last_sequence: 1, last_offset: 16_000, .. })) if job_id == request.job_id && generation == request.generation)
    );
    let commands_path = worker_root.join("commands.jsonl");
    let command_lines = fs::read_to_string(commands_path).expect("worker command trace");
    assert!(command_lines.contains("\"type\":\"transcribe\""));
    assert!(command_lines.contains("\"type\":\"shutdown\""));
    trace.push(serde_json::json!({
        "phase": "ipc_commands",
        "commands": command_lines.lines().collect::<Vec<_>>(),
    }));
    write_poll_trace(&root.0, "Q01", &trace, &helper_diagnostics);
}

#[test]
fn poll_parakeet_eof_after_result_preserves_unacknowledged_end_behavior() {
    let root = TestRoot::new("parakeet-eof-after-result");
    let worker_root = synthetic_worker_root(&root.0);
    let model = root.0.join("model.gguf");
    fs::write(&model, b"synthetic model path").expect("create synthetic model file");
    let mut request = personal_import_request(&root.0, &model);
    request.config.compute = whisper_core::ComputeChoice::Cpu;
    let source_sha = request
        .source_sha256
        .as_ref()
        .map(|sha| {
            sha.iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        })
        .expect("source hash");
    let hold_files = pipe_hold_files(&root.0, "parakeet-eof-after-result");
    let release = PipeHoldRelease(PipeHoldFiles {
        ready: hold_files.ready.clone(),
        release: hold_files.release.clone(),
        finished: hold_files.finished.clone(),
    });
    let shutdown_marker = root.0.join("parakeet-shutdown-received");
    let worker = compile_nominal_import_worker(
        &worker_root,
        &request,
        &source_sha,
        false,
        Some(&hold_files),
        Some(&shutdown_marker),
    );
    let paths = synthetic_parakeet_paths(worker);
    let mut runtime = runtime_with_active_attempt(request.job_id);
    runtime.parakeet_paths = Some(paths.clone());
    runtime.active.as_mut().unwrap().import_request = Some(request.clone());
    runtime.parakeet = Some(
        crate::parakeet_import::ParakeetProcess::spawn_live(&paths, &root.0)
            .expect("spawn result-then-EOF worker"),
    );
    let mut trace = vec![serde_json::json!({
        "phase": "started",
        "state": poll_state_snapshot(&runtime),
    })];
    let mut helper_diagnostics = Vec::new();

    assert_q06_ready_and_result_events(&mut runtime, &mut trace, &mut helper_diagnostics);
    assert!(wait_for_marker(&shutdown_marker, Duration::from_secs(8)));
    let status = wait_for_parakeet_exit(runtime.parakeet.as_mut().unwrap(), Duration::from_secs(8));
    let exit_success = status.as_ref().is_some_and(|status| status.success());
    assert!(exit_success);
    trace.push(serde_json::json!({
        "phase": "shutdown_received_and_parent_exited",
        "success": exit_success,
    }));
    let eof =
        record_poll_result(&mut runtime, &mut trace, "eof_after_result").expect("EOF after result");
    drop(release);
    trace.push(serde_json::json!({
        "phase": "holder_released",
        "holder_finished": hold_files.finished.is_file(),
    }));
    assert!(
        matches!(
            &eof,
            Err(PortError::Failed(message))
                if message == "WorkerExited: process ended before result acknowledgement"
        ),
        "unexpected Q06 EOF result: {eof:?}"
    );
    assert!(runtime.parakeet.is_none());
    assert!(runtime.parakeet_end.is_none());
    assert_eq!(
        fs::read_to_string(worker_root.join("commands.jsonl"))
            .expect("record commands through EOF")
            .lines()
            .count(),
        2,
        "Transcribe and Shutdown are sent before EOF is observed"
    );
    let commands =
        fs::read_to_string(worker_root.join("commands.jsonl")).expect("commands sent through EOF");
    trace.push(serde_json::json!({
        "phase": "ipc_commands",
        "commands": commands.lines().collect::<Vec<_>>(),
    }));
    write_poll_trace(&root.0, "Q06", &trace, &helper_diagnostics);
}

#[test]
fn poll_parakeet_failed_message_from_older_attempt_is_terminal() {
    let root = TestRoot::new("parakeet-older-attempt-failed");
    let worker_root = synthetic_worker_root(&root.0);
    let model = root.0.join("model.gguf");
    fs::write(&model, b"synthetic model path").expect("create synthetic model file");
    let request = personal_import_request(&root.0, &model);
    let stale_failure = serde_json::json!({
        "version": 1,
        "message": {
            "type": "failed",
            "request_id": 1,
            "job_id": format!("{:032x}", request.job_id.0),
            "generation": request.generation.get(),
            "attempt": 0,
            "identity": null,
            "code": "BackendUnavailable",
            "message": "synthetic failure from previous attempt",
        }
    });
    let stale_literal =
        serde_json::to_string(&stale_failure.to_string()).expect("encode previous-attempt failure");
    let source = r###"
use std::io::{self, BufRead, Write};
const FAILED: &str = __FAILED__;
fn main() {
    println!("{FAILED}");
    let _ = io::stdout().flush();
    for line in io::stdin().lock().lines().flatten() {
        if line.contains("\"type\":\"shutdown\"") { return; }
    }
}
"###
    .replace("__FAILED__", &stale_literal);
    let worker = compile_worker(&worker_root, "parakeet-older-attempt", &source);
    let paths = synthetic_parakeet_paths(worker);
    let mut runtime = runtime_with_active_attempt(request.job_id);
    let active = runtime.active.as_mut().unwrap();
    active.backend = BackendKind::Cuda;
    active.attempt_number = 1;
    active.import_request = Some(request);
    runtime.parakeet = Some(
        crate::parakeet_import::ParakeetProcess::spawn_live(&paths, &root.0)
            .expect("spawn old-attempt worker"),
    );

    let mut trace = vec![serde_json::json!({
        "phase": "active_attempt_one",
        "state": poll_state_snapshot(&runtime),
    })];
    let mut helper_diagnostics = Vec::new();
    let failure = poll_traced_until_event(
        &mut runtime,
        &mut trace,
        &mut helper_diagnostics,
        "old_attempt_failed",
    );
    assert!(matches!(
        failure,
        Ok(WorkerEvent::Failed {
            code: whisper_core::ipc::WorkerErrorCode::Internal,
            message,
            ..
        }) if message == "BackendUnavailable: synthetic failure from previous attempt"
    ));
    assert!(runtime.parakeet.is_none());
    assert!(runtime.active.is_some());
    assert!(runtime.auto_fallback_reason.is_none());
    write_poll_trace(&root.0, "Q07", &trace, &helper_diagnostics);
}

#[test]
fn poll_parakeet_stopped_ack_preserves_wait_and_maps_all_exit_states() {
    let root = TestRoot::new("parakeet-stopped-ack");
    let worker_root = synthetic_worker_root(&root.0);
    let job_id = JobId(73_240);
    let generation = Generation::first();
    let mut trace = Vec::new();
    let mut helper_diagnostics = Vec::new();

    let waiting_worker = compile_worker(
        &worker_root,
        "parakeet-waiting-worker",
        "fn main() { std::thread::park(); }",
    );
    let waiting_paths = synthetic_parakeet_paths(waiting_worker);
    let mut waiting = runtime_with_active_attempt(job_id);
    waiting.parakeet = Some(
        crate::parakeet_import::ParakeetProcess::spawn_live(&waiting_paths, &root.0)
            .expect("spawn waiting fake worker"),
    );
    waiting.parakeet_stopped_ack = true;
    waiting.parakeet_end = Some((job_id, generation, 2, 320));
    assert!(record_poll_result(&mut waiting, &mut trace, "stopped_ack_wait").is_none());
    assert!(waiting.parakeet.is_some());
    assert!(waiting.parakeet_stopped_ack);
    assert_eq!(waiting.parakeet_end, Some((job_id, generation, 2, 320)));
    waiting.parakeet.as_mut().unwrap().terminate().unwrap();

    let success_worker = compile_worker(&worker_root, "parakeet-success-worker", "fn main() {}");
    let success_paths = synthetic_parakeet_paths(success_worker);
    let mut success = runtime_with_active_attempt(job_id);
    success.parakeet = Some(
        crate::parakeet_import::ParakeetProcess::spawn_live(&success_paths, &root.0)
            .expect("spawn successful fake worker"),
    );
    success.parakeet_stopped_ack = true;
    success.parakeet_end = Some((job_id, generation, 2, 320));
    assert!(matches!(
        poll_traced_until_event(
            &mut success,
            &mut trace,
            &mut helper_diagnostics,
            "stopped_ack_success"
        ),
        Ok(WorkerEvent::End {
            job_id: actual_job,
            generation: actual_generation,
            instance_id: 7,
            last_sequence: 2,
            last_offset: 320,
        }) if actual_job == job_id && actual_generation == generation
    ));
    assert!(success.parakeet.is_none());
    assert!(!success.parakeet_stopped_ack);
    assert!(success.parakeet_end.is_none());

    let no_end_worker = compile_worker(&worker_root, "parakeet-no-end-worker", "fn main() {}");
    let no_end_paths = synthetic_parakeet_paths(no_end_worker);
    let mut no_end = runtime_with_active_attempt(job_id);
    no_end.parakeet = Some(
        crate::parakeet_import::ParakeetProcess::spawn_live(&no_end_paths, &root.0)
            .expect("spawn no-result fake worker"),
    );
    no_end.parakeet_stopped_ack = true;
    assert!(matches!(
        poll_traced_until_event(
            &mut no_end,
            &mut trace,
            &mut helper_diagnostics,
            "stopped_ack_without_end"
        ),
        Err(PortError::Failed(message))
            if message == "WorkerExited: Parakeet stopped without a transcription result"
    ));
    assert!(no_end.parakeet.is_none());
    assert!(!no_end.parakeet_stopped_ack);

    let failed_worker = compile_worker(
        &worker_root,
        "parakeet-failed-worker",
        "fn main() { std::process::exit(9); }",
    );
    let failed_paths = synthetic_parakeet_paths(failed_worker);
    let mut failed = runtime_with_active_attempt(job_id);
    failed.parakeet = Some(
        crate::parakeet_import::ParakeetProcess::spawn_live(&failed_paths, &root.0)
            .expect("spawn failing fake worker"),
    );
    failed.parakeet_stopped_ack = true;
    failed.parakeet_end = Some((job_id, generation, 2, 320));
    assert!(matches!(
        poll_traced_until_event(
            &mut failed,
            &mut trace,
            &mut helper_diagnostics,
            "stopped_ack_failure"
        ),
        Err(PortError::Failed(message)) if message.starts_with("WorkerExited:")
    ));
    assert!(failed.parakeet.is_none());
    assert!(!failed.parakeet_stopped_ack);
    assert!(failed.parakeet_end.is_none());
    write_poll_trace(&root.0, "P03", &trace, &helper_diagnostics);
}

#[test]
fn poll_parakeet_eof_status_preserves_the_existing_end_asymmetry() {
    let root = TestRoot::new("parakeet-eof-status");
    let worker_root = synthetic_worker_root(&root.0);
    let job_id = JobId(73_241);
    let generation = Generation::first();
    let mut trace = Vec::new();
    let success_files = pipe_hold_files(&root.0, "parakeet-eof-success");
    let success_release = PipeHoldRelease(PipeHoldFiles {
        ready: success_files.ready.clone(),
        release: success_files.release.clone(),
        finished: success_files.finished.clone(),
    });
    let success_worker =
        compile_pipe_holding_eof_worker(&worker_root, "parakeet-eof-success", &success_files, 0);
    let success_paths = synthetic_parakeet_paths(success_worker);
    let mut success = runtime_with_active_attempt(job_id);
    success.parakeet = Some(
        crate::parakeet_import::ParakeetProcess::spawn_live(&success_paths, &root.0)
            .expect("spawn successful EOF worker"),
    );
    success.parakeet_end = Some((job_id, generation, 1, 160));
    assert!(wait_for_marker(
        &success_files.ready,
        Duration::from_secs(8)
    ));
    let success_status =
        wait_for_parakeet_exit(success.parakeet.as_mut().unwrap(), Duration::from_secs(8));
    let success_exited = success_status
        .as_ref()
        .is_some_and(|status| status.success());
    assert!(success_exited);
    trace.push(serde_json::json!({
        "phase": "success_parent_exited_holder_open",
        "status_success": success_exited,
    }));
    let success_result = record_poll_result(&mut success, &mut trace, "success_empty_channel_exit");
    drop(success_release);
    trace.push(serde_json::json!({
        "phase": "success_holder_released",
        "finished": success_files.finished.is_file(),
    }));
    assert!(
        matches!(
            &success_result,
            Some(Err(PortError::Failed(message)))
                if message == "WorkerExited: process ended before result acknowledgement"
        ),
        "unexpected P04 successful-worker poll result: {success_result:?}"
    );
    assert!(success.parakeet.is_none());
    assert!(success.parakeet_end.is_none());

    let failed_files = pipe_hold_files(&root.0, "parakeet-eof-failed");
    let failed_release = PipeHoldRelease(PipeHoldFiles {
        ready: failed_files.ready.clone(),
        release: failed_files.release.clone(),
        finished: failed_files.finished.clone(),
    });
    let failed_worker =
        compile_pipe_holding_eof_worker(&worker_root, "parakeet-eof-failed", &failed_files, 9);
    let failed_paths = synthetic_parakeet_paths(failed_worker);
    let mut failed = runtime_with_active_attempt(job_id);
    failed.parakeet = Some(
        crate::parakeet_import::ParakeetProcess::spawn_live(&failed_paths, &root.0)
            .expect("spawn failed EOF worker"),
    );
    failed.parakeet_end = Some((job_id, generation, 1, 160));
    assert!(wait_for_marker(&failed_files.ready, Duration::from_secs(8)));
    let failed_status =
        wait_for_parakeet_exit(failed.parakeet.as_mut().unwrap(), Duration::from_secs(8));
    let failed_exit_code = failed_status.as_ref().and_then(|status| status.code());
    assert_eq!(failed_exit_code, Some(9));
    trace.push(serde_json::json!({
        "phase": "failed_parent_exited_holder_open",
        "exit_code": failed_exit_code,
    }));
    let failed_result = record_poll_result(&mut failed, &mut trace, "failed_empty_channel_exit");
    drop(failed_release);
    trace.push(serde_json::json!({
        "phase": "failed_holder_released",
        "finished": failed_files.finished.is_file(),
    }));
    assert!(matches!(
        failed_result,
        Some(Err(PortError::Failed(message))) if message.starts_with("WorkerExited:")
    ));
    assert!(failed.parakeet.is_none());
    assert_eq!(failed.parakeet_end, Some((job_id, generation, 1, 160)));
    write_poll_trace(&root.0, "P04", &trace, &[]);
}

#[test]
fn poll_parakeet_ready_without_active_import_is_stale() {
    let root = TestRoot::new("parakeet-ready-stale");
    let worker_root = synthetic_worker_root(&root.0);
    let ready_worker = compile_worker(
        &worker_root,
        "parakeet-ready-stale-worker",
        r###"fn main() { println!("{}", r#"{"version":1,"message":{"type":"ready","identity":{"model_sha256":"fixture","runtime_sha256":"fixture","build_sha256":"fixture","profile":"import-single-pass"},"backend":"CPU","native_version":"fixture"}}"#); }"###,
    );
    let paths = synthetic_parakeet_paths(ready_worker);
    let mut runtime = runtime_with_active_attempt(JobId(73_242));
    runtime.active = None;
    runtime.parakeet = Some(
        crate::parakeet_import::ParakeetProcess::spawn_live(&paths, &root.0)
            .expect("spawn READY worker"),
    );

    let mut trace = Vec::new();
    let mut helper_diagnostics = Vec::new();
    assert!(matches!(
        poll_traced_until_event(
            &mut runtime,
            &mut trace,
            &mut helper_diagnostics,
            "ready_without_active"
        ),
        Err(PortError::Failed(message))
            if message == "StaleResponse: Parakeet Ready without an active import"
    ));
    write_poll_trace(&root.0, "P06", &trace, &helper_diagnostics);
}

#[test]
fn poll_parakeet_stopped_success_without_active_import_consumes_end_without_event() {
    let root = TestRoot::new("parakeet-stopped-without-active");
    let worker_root = synthetic_worker_root(&root.0);
    let worker = compile_worker(
        &worker_root,
        "parakeet-exited-without-active",
        "fn main() {}",
    );
    let paths = synthetic_parakeet_paths(worker);
    let job_id = JobId(73_243);
    let generation = Generation::first();
    let mut runtime = runtime_with_active_attempt(job_id);
    runtime.active = None;
    runtime.parakeet = Some(
        crate::parakeet_import::ParakeetProcess::spawn_live(&paths, &root.0)
            .expect("spawn exited worker"),
    );
    runtime.parakeet_stopped_ack = true;
    runtime.parakeet_end = Some((job_id, generation, 4, 640));
    let status = wait_for_parakeet_exit(runtime.parakeet.as_mut().unwrap(), Duration::from_secs(8));
    assert!(status.is_some_and(|status| status.success()));

    let mut trace = vec![
        serde_json::json!({ "phase": "exited_with_no_active", "state": poll_state_snapshot(&runtime) }),
    ];
    let result = record_poll_result(&mut runtime, &mut trace, "stopped_success_no_active");
    assert!(result.is_none());
    assert!(runtime.parakeet.is_none());
    assert!(!runtime.parakeet_stopped_ack);
    assert!(runtime.parakeet_end.is_none());
    write_candidate_poll_trace(&root.0, "P03-no-active", &trace, &[]);
}

#[test]
fn poll_parakeet_ready_without_import_request_is_stale() {
    let root = TestRoot::new("parakeet-ready-without-request");
    let (request, mut runtime) = request_runtime(&root.0, JobId(73_244));
    let identity =
        parakeet_test_identity(whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS);
    attach_scripted_worker(
        &root.0,
        &mut runtime,
        "parakeet-ready-without-request",
        &[parakeet_ready_frame(identity, "CPU")],
        None,
        None,
    );
    runtime.active.as_mut().unwrap().import_request = None;
    let mut trace = Vec::new();
    let mut diagnostics = Vec::new();
    let result = poll_traced_until_event(
        &mut runtime,
        &mut trace,
        &mut diagnostics,
        "ready_without_request",
    );
    assert!(
        matches!(result, Err(PortError::Failed(message)) if message == "StaleResponse: Parakeet request is unavailable")
    );
    assert_eq!(request.job_id, JobId(73_244));
    write_candidate_poll_trace(&root.0, "P06-no-request", &trace, &diagnostics);
}

#[test]
fn poll_parakeet_ready_rejects_identity_before_accepting_process() {
    let root = TestRoot::new("parakeet-ready-identity-mismatch");
    let (_, mut runtime) = request_runtime(&root.0, JobId(73_245));
    let mut identity =
        parakeet_test_identity(whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS);
    identity["runtime_sha256"] = serde_json::Value::String("wrong-runtime".into());
    attach_scripted_worker(
        &root.0,
        &mut runtime,
        "parakeet-ready-identity-mismatch",
        &[parakeet_ready_frame(identity, "CPU")],
        None,
        None,
    );
    let mut trace = Vec::new();
    let mut diagnostics = Vec::new();
    let result = poll_traced_until_event(
        &mut runtime,
        &mut trace,
        &mut diagnostics,
        "ready_identity_mismatch",
    );
    assert!(
        matches!(result, Err(PortError::Failed(message)) if message == "BackendMismatch: Parakeet worker identity differs from the qualified key")
    );
    assert!(runtime.parakeet.is_some());
    write_candidate_poll_trace(&root.0, "P08-ready-identity", &trace, &diagnostics);
}

#[test]
fn poll_parakeet_vad_ready_reaches_single_pass_process_profile_mismatch() {
    let root = TestRoot::new("parakeet-vad-ready-profile-mismatch");
    let (request, mut runtime) = request_runtime(&root.0, JobId(73_246));
    let identity = parakeet_test_identity(whisper_core::parakeet_windows::PROFILE_IMPORT_VAD);
    attach_scripted_worker(
        &root.0,
        &mut runtime,
        "parakeet-vad-ready-profile-mismatch",
        &[parakeet_ready_frame(identity, "CPU")],
        None,
        None,
    );
    let mut vad = one_window_vad_run(request);
    vad.worker_ready = false;
    runtime.parakeet_vad = Some(vad);
    let mut trace = Vec::new();
    let mut diagnostics = Vec::new();
    let result = poll_traced_until_event(
        &mut runtime,
        &mut trace,
        &mut diagnostics,
        "vad_ready_single_pass_process",
    );
    assert!(matches!(result, Err(PortError::Failed(message)) if message == "ProfileMismatch"));
    assert!(
        runtime
            .parakeet_vad
            .as_ref()
            .is_some_and(|vad| !vad.worker_ready)
    );
    write_candidate_poll_trace(&root.0, "P08-vad-process-profile", &trace, &diagnostics);
}

#[test]
fn poll_parakeet_vad_result_dispatches_after_private_vad_state_is_installed() {
    let root = TestRoot::new("parakeet-vad-result-dispatch");
    let (request, mut runtime) = request_runtime(&root.0, JobId(73_247));
    let identity =
        parakeet_test_identity(whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS);
    let words = serde_json::json!([{ "text": "vad.", "start_ms": 0, "end_ms": 500 }]);
    let result = parakeet_result_frame(&request, identity.clone(), words);
    attach_scripted_worker(
        &root.0,
        &mut runtime,
        "parakeet-vad-result-dispatch",
        &[parakeet_ready_frame(identity, "CPU")],
        Some(&result),
        None,
    );
    let mut trace = Vec::new();
    let mut diagnostics = Vec::new();
    let ready = poll_traced_until_event(
        &mut runtime,
        &mut trace,
        &mut diagnostics,
        "single_pass_ready",
    );
    assert!(matches!(
        ready,
        Ok(WorkerEvent::Ready {
            backend: BackendKind::Cpu,
            ..
        })
    ));
    runtime.parakeet_vad = Some(one_window_vad_run(request));
    let progress = poll_traced_until_event(
        &mut runtime,
        &mut trace,
        &mut diagnostics,
        "vad_result_progress",
    );
    assert!(matches!(
        progress,
        Ok(WorkerEvent::Progress {
            completed_samples: 16_000,
            total_samples: 0,
            ..
        })
    ));
    let segment = record_poll_result(&mut runtime, &mut trace, "vad_result_segment")
        .expect("queued VAD segment");
    assert!(matches!(segment, Ok(WorkerEvent::Segment(segment)) if segment.text == "vad."));
    assert!(
        runtime
            .parakeet_vad
            .as_ref()
            .unwrap()
            .pending_confirmation
            .is_some()
    );
    write_candidate_poll_trace(&root.0, "P10-vad-result", &trace, &diagnostics);
}

#[test]
fn poll_parakeet_result_rejects_missing_request_and_bad_correlation() {
    let root = TestRoot::new("parakeet-result-missing-and-correlation");
    let mut trace = Vec::new();
    let mut diagnostics = Vec::new();

    let (request, mut without_request) = request_runtime(&root.0, JobId(73_248));
    let identity =
        parakeet_test_identity(whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS);
    let result = parakeet_result_frame(
        &request,
        identity.clone(),
        serde_json::json!([{ "text": "ok", "start_ms": 0, "end_ms": 100 }]),
    );
    attach_scripted_worker(
        &root.0,
        &mut without_request,
        "result-without-request",
        &[result],
        None,
        None,
    );
    without_request.active.as_mut().unwrap().import_request = None;
    let stale = poll_traced_until_event(
        &mut without_request,
        &mut trace,
        &mut diagnostics,
        "result_without_request",
    );
    assert!(
        matches!(stale, Err(PortError::Failed(message)) if message == "StaleResponse: Parakeet request is unavailable")
    );

    let mismatched_root = root.0.join("bad-correlation");
    fs::create_dir_all(&mismatched_root).expect("create independent bad-correlation fixture");
    let (request, mut mismatched) = request_runtime(&mismatched_root, JobId(73_249));
    let identity =
        parakeet_test_identity(whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS);
    let mut bad_result: serde_json::Value = serde_json::from_str(&parakeet_result_frame(
        &request,
        identity.clone(),
        serde_json::json!([{ "text": "ok", "start_ms": 0, "end_ms": 100 }]),
    ))
    .unwrap();
    bad_result["message"]["generation"] = serde_json::json!(99);
    let bad_result = bad_result.to_string();
    attach_scripted_worker(
        &mismatched_root,
        &mut mismatched,
        "result-bad-correlation",
        &[parakeet_ready_frame(identity, "CPU")],
        Some(&bad_result),
        None,
    );
    assert!(matches!(
        poll_traced_until_event(
            &mut mismatched,
            &mut trace,
            &mut diagnostics,
            "result_ready"
        ),
        Ok(WorkerEvent::Ready { .. })
    ));
    let mismatch = poll_traced_until_event(
        &mut mismatched,
        &mut trace,
        &mut diagnostics,
        "result_bad_correlation",
    );
    assert!(
        matches!(mismatch, Err(PortError::Failed(message)) if message.starts_with("ProtocolMismatch: Parakeet result identity"))
    );
    write_candidate_poll_trace(&root.0, "P11-result-guards", &trace, &diagnostics);
}

#[test]
fn poll_parakeet_invalid_result_words_preserve_queued_progress() {
    let root = TestRoot::new("parakeet-invalid-result-words");
    let (request, mut runtime) = request_runtime(&root.0, JobId(73_250));
    let identity =
        parakeet_test_identity(whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS);
    let result = parakeet_result_frame(
        &request,
        identity.clone(),
        serde_json::json!([{ "text": "bad", "start_ms": 0, "end_ms": 0 }]),
    );
    attach_scripted_worker(
        &root.0,
        &mut runtime,
        "invalid-result-words",
        &[parakeet_ready_frame(identity, "CPU")],
        Some(&result),
        None,
    );
    let mut trace = Vec::new();
    let mut diagnostics = Vec::new();
    assert!(matches!(
        poll_traced_until_event(
            &mut runtime,
            &mut trace,
            &mut diagnostics,
            "invalid_result_ready"
        ),
        Ok(WorkerEvent::Ready { .. })
    ));
    let invalid = poll_traced_until_event(
        &mut runtime,
        &mut trace,
        &mut diagnostics,
        "invalid_result_words",
    );
    assert!(
        matches!(invalid, Err(PortError::Failed(message)) if message.starts_with("InvalidTimestamp: Parakeet word 0"))
    );
    let queued = record_poll_result(&mut runtime, &mut trace, "progress_after_invalid_words")
        .expect("progress queued before validation");
    assert!(matches!(
        queued,
        Ok(WorkerEvent::Progress {
            completed_samples: 16_000,
            total_samples: 16_000,
            ..
        })
    ));
    write_candidate_poll_trace(&root.0, "P12-invalid-words", &trace, &diagnostics);
}

#[test]
fn poll_parakeet_native_call_started_binds_request_and_worker_pid() {
    let root = TestRoot::new("parakeet-native-call-binding");
    for (name, job_id, pid_adjustment, expected_error) in [
        ("valid", JobId(73_251), 0, false),
        ("invalid-pid", JobId(73_252), 1, true),
    ] {
        let fixture_root = root.0.join(name);
        fs::create_dir_all(&fixture_root).expect("create independent NativeCallStarted fixture");
        let (request, mut runtime) = request_runtime(&fixture_root, job_id);
        let worker_root = synthetic_worker_root(&fixture_root);
        let marker = worker_root.join("reported-pid.txt");
        let worker =
            compile_native_started_worker(&worker_root, name, &request, &marker, pid_adjustment);
        let paths = synthetic_parakeet_paths(worker);
        let worker_pid = crate::parakeet_import::ParakeetProcess::spawn_live(&paths, &fixture_root)
            .expect("spawn NativeCallStarted worker");
        let actual_pid = worker_pid.pid();
        runtime.parakeet_paths = Some(paths);
        runtime.parakeet = Some(worker_pid);
        let mut trace = vec![serde_json::json!({
            "phase": "native_call_worker_started",
            "state": poll_state_snapshot(&runtime),
        })];
        let mut diagnostics = Vec::new();
        assert!(wait_for_marker(&marker, Duration::from_secs(8)));
        let emitted_frame_raw =
            fs::read_to_string(&marker).expect("read emitted NativeCallStarted frame");
        let emitted_frame: serde_json::Value =
            serde_json::from_str(&emitted_frame_raw).expect("decode emitted frame");
        let message = &emitted_frame["message"];
        assert_eq!(message["type"], "native_call_started");
        assert_eq!(message["request_id"], 1);
        assert_eq!(message["job_id"], format!("{:032x}", request.job_id.0));
        assert_eq!(message["generation"], request.generation.get());
        assert_eq!(message["attempt"], 0);
        let reported_pid = message["pid"]
            .as_u64()
            .expect("emitted NativeCallStarted PID");
        let pid_matches = u64::from(actual_pid) == reported_pid;
        diagnostics.push(serde_json::json!({
            "phase": "pid_binding",
            "worker_pid": actual_pid,
            "reported_pid": reported_pid,
            "emitted_frame_raw": emitted_frame_raw,
        }));
        trace.push(serde_json::json!({
            "phase": "native_call_started_event",
            "worker_pid": actual_pid,
            "reported_pid": reported_pid,
            "envelope": emitted_frame,
        }));
        if expected_error {
            let result = poll_traced_until_event(
                &mut runtime,
                &mut trace,
                &mut diagnostics,
                "native_call_wrong_pid",
            );
            assert!(
                matches!(result, Err(PortError::Failed(message)) if message == "ProtocolMismatch: NativeCallStarted is not bound to the active Parakeet request")
            );
            assert!(!runtime.parakeet_native_call_started);
            assert!(!pid_matches);
        } else {
            let result = poll_until_native_call_state(&mut runtime, &mut trace, &mut diagnostics);
            assert!(result.is_none());
            assert!(runtime.parakeet_native_call_started);
            assert!(pid_matches);
        }
        runtime
            .parakeet
            .as_mut()
            .expect("worker remains attached after IPC event")
            .terminate()
            .expect("terminate NativeCallStarted fixture");
        write_candidate_native_trace(&root.0, &format!("P15-{name}"), &trace, &diagnostics);
    }
}

fn compile_pipe_holding_stopped_worker(
    worker_root: &Path,
    name: &str,
    files: &PipeHoldFiles,
) -> PathBuf {
    let ready = serde_json::to_string(files.ready.to_string_lossy().as_ref())
        .expect("encode stopped holder ready path");
    let release = serde_json::to_string(files.release.to_string_lossy().as_ref())
        .expect("encode stopped holder release path");
    let finished = serde_json::to_string(files.finished.to_string_lossy().as_ref())
        .expect("encode stopped holder finished path");
    let source = r###"
use std::{env, fs, io::{self, Write}, path::Path, process::{Command, Stdio}, thread, time::{Duration, Instant}};
const READY: &str = __READY__;
const RELEASE: &str = __RELEASE__;
const FINISHED: &str = __FINISHED__;
fn wait_for(path: &str, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while !Path::new(path).is_file() && Instant::now() < deadline { thread::yield_now(); }
    Path::new(path).is_file()
}
fn main() {
    if env::args().nth(1).as_deref() == Some("--hold-stdout") {
        fs::write(READY, b"ready").expect("signal stopped holder ready");
        let _ = wait_for(RELEASE, Duration::from_secs(12));
        fs::write(FINISHED, b"finished").expect("signal stopped holder finished");
        return;
    }
    let executable = env::current_exe().expect("worker executable path");
    let holder = Command::new(executable)
        .arg("--hold-stdout")
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn stopped stdout holder");
    assert!(wait_for(READY, Duration::from_secs(8)), "stdout holder ready");
    println!("{}", r#"{"version":1,"message":{"type":"stopped"}}"#);
    let _ = io::stdout().flush();
    drop(holder);
}
"###
        .replace("__READY__", &ready)
        .replace("__RELEASE__", &release)
        .replace("__FINISHED__", &finished);
    compile_worker(worker_root, name, &source)
}

#[test]
fn poll_parakeet_stopped_ipc_recurses_to_queued_end_after_exit() {
    let root = TestRoot::new("parakeet-stopped-ipc-recursion");
    let worker_root = synthetic_worker_root(&root.0);
    let files = pipe_hold_files(&root.0, "parakeet-stopped-ipc");
    let release = PipeHoldRelease(PipeHoldFiles {
        ready: files.ready.clone(),
        release: files.release.clone(),
        finished: files.finished.clone(),
    });
    let worker = compile_pipe_holding_stopped_worker(&worker_root, "stopped-ipc-worker", &files);
    let paths = synthetic_parakeet_paths(worker);
    let job_id = JobId(73_253);
    let generation = Generation::first();
    let mut runtime = runtime_with_active_attempt(job_id);
    runtime.parakeet = Some(
        crate::parakeet_import::ParakeetProcess::spawn_live(&paths, &root.0)
            .expect("spawn STOPPED IPC worker"),
    );
    runtime.parakeet_end = Some((job_id, generation, 1, 160));
    assert!(wait_for_marker(&files.ready, Duration::from_secs(8)));
    let exit = wait_for_parakeet_exit(runtime.parakeet.as_mut().unwrap(), Duration::from_secs(8));
    assert!(exit.is_some_and(|status| status.success()));

    let mut trace = vec![serde_json::json!({
        "phase": "stopped_parent_exited_holder_open",
        "state": poll_state_snapshot(&runtime),
    })];
    let mut diagnostics = Vec::new();
    let result = poll_traced_until_event(
        &mut runtime,
        &mut trace,
        &mut diagnostics,
        "stopped_recursion",
    );
    assert!(
        matches!(result, Ok(WorkerEvent::End { job_id: actual, generation: actual_generation, last_sequence: 1, last_offset: 160, .. }) if actual == job_id && actual_generation == generation)
    );
    assert!(runtime.parakeet.is_none());
    assert!(!runtime.parakeet_stopped_ack);
    assert!(runtime.parakeet_end.is_none());
    drop(release);
    trace.push(serde_json::json!({
        "phase": "stopped_holder_released",
        "finished": files.finished.is_file(),
    }));
    write_candidate_native_trace(&root.0, "P16-stopped-ipc", &trace, &diagnostics);
}

#[test]
fn poll_parakeet_failed_stale_identity_variants_keep_the_active_worker() {
    let root = TestRoot::new("parakeet-failed-stale-job");
    for (name, job_id, stale_job, generation, mismatch_identity) in [
        ("stale-job", JobId(73_254), true, 1, false),
        ("stale-generation", JobId(73_255), false, 2, false),
        ("stale-worker-identity", JobId(73_256), false, 1, true),
    ] {
        let fixture_root = root.0.join(name);
        fs::create_dir_all(&fixture_root).expect("create stale Failed fixture root");
        let mut identity =
            parakeet_test_identity(whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS);
        if mismatch_identity {
            identity["runtime_sha256"] = serde_json::json!("different-runtime");
        }
        let failure = parakeet_failed_frame(serde_json::json!({
            "type": "failed",
            "request_id": 1,
            "job_id": format!("{:032x}", if stale_job { job_id.0 + 1 } else { job_id.0 }),
            "generation": generation,
            "attempt": 0,
            "identity": identity.clone(),
            "code": "BackendUnavailable",
            "message": name,
        }));
        let frames = [
            parakeet_ready_frame(
                parakeet_test_identity(
                    whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS,
                ),
                "CPU",
            ),
            failure,
        ];
        let outcome = run_failed_poll_fixture(
            &fixture_root,
            name,
            job_id,
            BackendKind::Cpu,
            ComputeChoice::Cpu,
            false,
            &frames,
        );
        assert!(matches!(
            &outcome.result,
            Err(PortError::Failed(message))
                if message == "StaleResponse: Parakeet failure identity differs from active import"
        ));
        assert!(outcome.runtime.parakeet.is_some());
        assert!(outcome.runtime.active.is_some());
        write_candidate_failed_trace(
            &root.0,
            &format!("P13-{name}"),
            &outcome.trace,
            &outcome.diagnostics,
        );
    }
}

#[test]
fn poll_parakeet_failed_without_active_import_is_stale() {
    let root = TestRoot::new("parakeet-failed-without-active");
    let (request, mut runtime) = request_runtime(&root.0, JobId(73_259));
    runtime.active = None;
    let failure = parakeet_failed_frame(serde_json::json!({
        "type": "failed",
        "request_id": 1,
        "job_id": format!("{:032x}", request.job_id.0),
        "generation": request.generation.get(),
        "attempt": 0,
        "identity": null,
        "code": "BackendUnavailable",
        "message": "failure without active import",
    }));
    let frames = [failure];
    attach_scripted_worker(
        &root.0,
        &mut runtime,
        "failed-without-active",
        &frames,
        None,
        None,
    );
    let mut trace = vec![serde_json::json!({
        "phase": "failed_without_active_started",
        "state": poll_state_snapshot(&runtime),
        "input_frames": frames,
    })];
    let mut diagnostics = Vec::new();
    let result = poll_traced_until_event(
        &mut runtime,
        &mut trace,
        &mut diagnostics,
        "failed_without_active",
    );
    assert!(matches!(
        result,
        Err(PortError::Failed(message))
            if message == "StaleResponse: Parakeet failure without active import"
    ));
    assert!(runtime.active.is_none());
    assert!(runtime.parakeet.is_some());
    write_candidate_failed_trace(&root.0, "P13-no-active", &trace, &diagnostics);
}

#[test]
fn poll_parakeet_failed_maps_protocol_error_codes() {
    let root = TestRoot::new("parakeet-failed-error-codes");
    for (name, code, expected) in [
        (
            "model-hash-mismatch",
            "ModelHashMismatch",
            whisper_core::ipc::WorkerErrorCode::ModelHashMismatch,
        ),
        (
            "protocol-version-mismatch",
            "ProtocolVersionMismatch",
            whisper_core::ipc::WorkerErrorCode::ProtocolMismatch,
        ),
        (
            "invalid-message",
            "InvalidMessage",
            whisper_core::ipc::WorkerErrorCode::ProtocolMismatch,
        ),
        (
            "default-internal",
            "BackendUnavailable",
            whisper_core::ipc::WorkerErrorCode::Internal,
        ),
    ] {
        let fixture_root = root.0.join(name);
        fs::create_dir_all(&fixture_root).expect("create Failed code fixture root");
        let job_id = JobId(73_260);
        let failure = parakeet_failed_frame(serde_json::json!({
            "type": "failed",
            "request_id": 1,
            "job_id": format!("{:032x}", job_id.0),
            "generation": 1,
            "attempt": 0,
            "identity": null,
            "code": code,
            "message": name,
        }));
        let frames = [failure];
        let outcome = run_failed_poll_fixture(
            &fixture_root,
            name,
            job_id,
            BackendKind::Cpu,
            ComputeChoice::Cpu,
            false,
            &frames,
        );
        assert!(matches!(
            &outcome.result,
            Ok(WorkerEvent::Failed { code: actual, message, .. })
                if *actual == expected && message == &format!("{code}: {name}")
        ));
        assert!(outcome.runtime.parakeet.is_none());
        assert!(outcome.runtime.active.is_some());
        write_candidate_failed_trace(
            &root.0,
            &format!("P13-{name}"),
            &outcome.trace,
            &outcome.diagnostics,
        );
    }
}

fn assert_gpu_failed_terminal(
    root: &Path,
    name: &str,
    job_id: JobId,
    compute: ComputeChoice,
    native_call_started: bool,
    request_id: u64,
) -> FailedPollFixture {
    let failure = parakeet_failed_frame(serde_json::json!({
        "type": "failed",
        "request_id": request_id,
        "job_id": format!("{:032x}", job_id.0),
        "generation": Generation::first().get(),
        "attempt": 0,
        "code": "BackendUnavailable",
        "message": name,
    }));
    let frames = [failure];
    let outcome = run_failed_poll_fixture(
        root,
        name,
        job_id,
        BackendKind::Cuda,
        compute,
        native_call_started,
        &frames,
    );
    assert!(matches!(
        &outcome.result,
        Ok(WorkerEvent::Failed {
            code: whisper_core::ipc::WorkerErrorCode::Internal,
            ..
        })
    ));
    assert!(outcome.runtime.parakeet.is_none());
    let active = outcome.runtime.active.as_ref().expect("active GPU attempt");
    assert_eq!(active.backend, BackendKind::Cuda);
    assert_eq!(active.attempt_number, 0);
    assert_eq!(
        active.import_request.as_ref().unwrap().config.compute,
        compute
    );
    assert_eq!(
        outcome.runtime.parakeet_native_call_started,
        native_call_started
    );
    assert!(outcome.runtime.auto_fallback_reason.is_none());
    outcome
}

#[test]
fn poll_parakeet_failed_preserves_optional_and_backend_routing() {
    let root = TestRoot::new("parakeet-failed-routing");
    let optional_root = root.0.join("cpu-optional-fields");
    fs::create_dir_all(&optional_root).expect("create CPU optional-fields root");
    let optional_failure = parakeet_failed_frame(serde_json::json!({
        "type": "failed",
        "code": "BackendUnavailable",
        "message": "optional fields omitted",
    }));
    let optional_frames = [optional_failure];
    let optional = run_failed_poll_fixture(
        &optional_root,
        "cpu-optional-fields",
        JobId(73_255),
        BackendKind::Cpu,
        ComputeChoice::Cpu,
        false,
        &optional_frames,
    );
    assert!(matches!(
        optional.result,
        Ok(WorkerEvent::Failed {
            code: whisper_core::ipc::WorkerErrorCode::Internal,
            message,
            ..
        }) if message == "BackendUnavailable: optional fields omitted"
    ));
    assert!(optional.runtime.parakeet.is_none());
    assert!(optional.runtime.active.is_some());
    write_candidate_failed_trace(
        &root.0,
        "P14-cpu-optional-fields",
        &optional.trace,
        &optional.diagnostics,
    );

    for (name, job_id, compute, native_call_started, request_id) in [
        ("manual-gpu", JobId(73_256), ComputeChoice::Gpu, false, 1),
        (
            "auto-after-native",
            JobId(73_257),
            ComputeChoice::Auto,
            true,
            1,
        ),
        (
            "auto-wrong-request",
            JobId(73_258),
            ComputeChoice::Auto,
            false,
            2,
        ),
    ] {
        let fixture_root = root.0.join(name);
        fs::create_dir_all(&fixture_root).expect("create GPU Failed fixture root");
        let outcome = assert_gpu_failed_terminal(
            &fixture_root,
            name,
            job_id,
            compute,
            native_call_started,
            request_id,
        );
        write_candidate_failed_trace(
            &root.0,
            &format!("P14-{name}"),
            &outcome.trace,
            &outcome.diagnostics,
        );
    }
}

#[test]
fn poll_parakeet_result_rejects_inactive_identity_duration_and_empty_words() {
    let root = TestRoot::new("parakeet-result-guards");
    for (name, active, mutation) in [
        ("no-active", false, "none"),
        ("identity-mismatch", true, "identity"),
        ("duration-mismatch", true, "duration"),
        ("empty-words", true, "empty_words"),
    ] {
        let fixture_root = root.0.join(name);
        fs::create_dir_all(&fixture_root).expect("create Result guard fixture root");
        let job_id = JobId(73_270);
        let (request, mut runtime) = request_runtime(&fixture_root, job_id);
        if !active {
            runtime.active = None;
        }
        let identity =
            parakeet_test_identity(whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS);
        let mut result: serde_json::Value = serde_json::from_str(&parakeet_result_frame(
            &request,
            identity.clone(),
            serde_json::json!([{ "text": "word", "start_ms": 0, "end_ms": 100 }]),
        ))
        .expect("decode synthetic Result frame");
        match mutation {
            "identity" => {
                result["message"]["identity"]["runtime_sha256"] =
                    serde_json::json!("different-runtime");
            }
            "duration" => result["message"]["duration_samples"] = serde_json::json!(8_000),
            "empty_words" => result["message"]["words"] = serde_json::json!([]),
            "none" => {}
            _ => unreachable!("known Result fixture variant"),
        }
        let result_frame = result.to_string();
        let mut frames = Vec::new();
        if active {
            frames.push(parakeet_ready_frame(identity, "CPU"));
        }
        frames.push(result_frame);
        attach_scripted_worker(&fixture_root, &mut runtime, name, &frames, None, None);
        let mut trace = vec![serde_json::json!({
            "phase": "result_fixture_started",
            "scenario": { "active": active, "mutation": mutation },
            "state": poll_state_snapshot(&runtime),
            "input_frames": frames,
        })];
        let mut diagnostics = Vec::new();
        let first = poll_traced_until_event(
            &mut runtime,
            &mut trace,
            &mut diagnostics,
            "result_first_event",
        );
        let result = if active && matches!(&first, Ok(WorkerEvent::Ready { .. })) {
            poll_traced_until_event(
                &mut runtime,
                &mut trace,
                &mut diagnostics,
                "result_rejected",
            )
        } else {
            first
        };
        let expected = if active {
            "ProtocolMismatch: Parakeet result identity, duration, or words differ"
        } else {
            "StaleResponse: Parakeet result without active import"
        };
        assert!(matches!(
            result,
            Err(PortError::Failed(message)) if message == expected
        ));
        assert!(runtime.parakeet_events.is_empty());
        assert!(runtime.parakeet_end.is_none());
        assert!(runtime.parakeet.is_some());
        assert_eq!(runtime.active.is_some(), active);
        write_pending_result_trace(&root.0, &format!("P11-{name}"), &trace, &diagnostics);
    }
}

#[cfg(feature = "personal-qualification")]
const Q03_CPU_WORKER_SOURCE: &str = r###"
use std::{env, fs::OpenOptions, io::{self, BufRead, Write}};
fn main() {
    let worker_dir = env::current_exe().expect("worker path").parent().expect("worker dir").to_path_buf();
    let ready = std::fs::read_to_string(worker_dir.join("cpu-ready-identity.json")).expect("fixture identity");
    let mut frame = String::from(r#"{"version":1,"message":{"type":"ready","identity":{"#);
    frame.push_str(&ready);
    frame.push_str(r#"},"backend":"CPU","native_version":"fixture"}}"#);
    println!("{frame}");
    let _ = io::stdout().flush();
    for line in io::stdin().lock().lines().flatten() {
        let mut commands = OpenOptions::new().create(true).append(true).open(worker_dir.join("cpu-commands.jsonl")).expect("command trace");
        writeln!(commands, "{line}").expect("record CPU command");
        if line.contains("\"type\":\"transcribe\"") {
            let marker = worker_dir.join("cpu-transcribe.jsonl");
            let mut file = OpenOptions::new().create(true).append(true).open(marker).expect("marker");
            writeln!(file, "{line}").expect("record CPU transcribe");
        }
        if line.contains("\"type\":\"shutdown\"") { break; }
    }
}
"###;

#[cfg(feature = "personal-qualification")]
const Q03_GPU_WORKER_SOURCE: &str = r###"
use std::{env, fs::OpenOptions, io::{self, BufRead, Write}};
fn main() {
    let directory = env::current_exe().expect("worker path").parent().expect("worker directory").to_path_buf();
    let event = r#"{"version":1,"message":{"type":"failed","request_id":1,"job_id":"__JOB_ID__","generation":1,"attempt":1,"identity":null,"code":"BackendUnavailable","message":"synthetic pre-audio GPU failure"}}"#;
    let mut events = OpenOptions::new().create(true).append(true).open(directory.join("gpu-events.jsonl")).expect("event trace");
    writeln!(events, "{event}").expect("record GPU Failed");
    println!("{event}");
    let _ = io::stdout().flush();
    for line in io::stdin().lock().lines().flatten() {
        let mut commands = OpenOptions::new().create(true).append(true).open(directory.join("gpu-commands.jsonl")).expect("command trace");
        writeln!(commands, "{line}").expect("record GPU command");
        if line.contains("\"type\":\"shutdown\"") { break; }
    }
}
"###;

#[cfg(feature = "personal-qualification")]
#[test]
fn poll_parakeet_gpu_failure_falls_back_to_qualified_cpu_fixture() {
    if let Some(root) = std::env::var_os(CHILD_ROOT) {
        run_personal_cpu_fallback_child(Path::new(&root));
        return;
    }

    let root = TestRoot::new("parakeet-equivalence-child");
    let model = std::env::var_os("N1_TEST_PERSONAL_PARAKEET_MODEL")
        .map(PathBuf::from)
        .expect("guard supplies the validated Personal Parakeet model");
    let local_app_data = root.0.join("isolated-localappdata");
    let runtime_root = local_app_data.join("Whisper");
    let output = Command::new(std::env::current_exe().expect("test executable path"))
        .arg("--exact")
        .arg("worker_ipc::runtime::poll::tests::parakeet_equivalence::poll_parakeet_gpu_failure_falls_back_to_qualified_cpu_fixture")
        .arg("--nocapture")
        .env(CHILD_ROOT, &root.0)
        .env("LOCALAPPDATA", &local_app_data)
        .env("PARAKEET_L2_RUNTIME_ROOT", &runtime_root)
        .env(
            "PARAKEET_L2_MODEL",
            runtime_root.join("models/parakeet-tdt-0.6b-v3.q8_0.gguf"),
        )
        .env(
            "PARAKEET_L2_CPU_WORKER",
            runtime_root.join("worker/whisper-worker-parakeet-cpu.exe"),
        )
        .env("PARAKEET_L2_CPU_NATIVE_DIR", runtime_root.join("native/cpu"))
        .env("N1_TEST_PERSONAL_PARAKEET_MODEL", model)
        .output()
        .expect("run isolated Parakeet fallback child");
    assert!(
        output.status.success(),
        "Q03 child failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"),
        "Q03 child did not execute exactly one test:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[cfg(feature = "personal-qualification")]
fn prepare_q03_workers(
    root: &Path,
) -> (
    PathBuf,
    PathBuf,
    String,
    crate::parakeet_import::ParakeetWorkerPaths,
    crate::parakeet_import::ParakeetWorkerPaths,
) {
    let model_source = PathBuf::from(
        std::env::var_os("N1_TEST_PERSONAL_PARAKEET_MODEL")
            .expect("guard supplies the validated Personal Parakeet model"),
    );
    let install_root = model_source
        .parent()
        .and_then(Path::parent)
        .expect("model lives under the personal Whisper installation");
    let runtime_root = root.join("isolated-localappdata").join("Whisper");
    let worker_dir = runtime_root.join("worker");
    let cpu_native = runtime_root.join("native/cpu");
    let gpu_native = runtime_root.join("native/gpu");
    let model_path = runtime_root.join("models/parakeet-tdt-0.6b-v3.q8_0.gguf");
    fs::create_dir_all(&worker_dir).expect("create isolated worker directory");
    fs::create_dir_all(&cpu_native).expect("create isolated CPU runtime directory");
    fs::create_dir_all(&gpu_native).expect("create isolated GPU runtime directory");
    fs::create_dir_all(model_path.parent().expect("model parent"))
        .expect("create isolated model directory");
    fs::copy(&model_source, &model_path).expect("copy verified model into isolated fixture");
    copy_q03_cpu_runtime(&install_root, &cpu_native);
    let transcribe_marker = worker_dir.join("cpu-transcribe.jsonl");
    let cpu_worker = compile_worker_relative(
        &worker_dir,
        "whisper_worker_parakeet_cpu",
        "whisper-worker-parakeet-cpu.rs",
        "whisper-worker-parakeet-cpu.exe",
        Q03_CPU_WORKER_SOURCE,
    );
    let cpu_worker_sha256 = crate::archive::sha256_file(&cpu_worker)
        .expect("hash deterministic CPU worker fixture")
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    fs::write(worker_dir.join("gpu-commands.jsonl"), b"")
        .expect("record that GPU received no IPC command before termination");
    let cpu_paths = crate::parakeet_import::ParakeetWorkerPaths {
        executable: cpu_worker,
        native_dir: cpu_native,
        model: model_path.clone(),
        runtime_root: runtime_root.clone(),
    };
    let key = cpu_paths
        .qualification_key(whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS)
        .expect("dynamic isolated personal CPU qualification key");
    assert_eq!(key.worker_build, cpu_worker_sha256);
    write_q03_cpu_ready_identity(&worker_dir, &key);
    let gpu_worker_source =
        Q03_GPU_WORKER_SOURCE.replace("__JOB_ID__", &format!("{:032x}", Q03_JOB_ID.0));
    let gpu_worker = compile_worker_relative(
        &worker_dir,
        "whisper_worker_parakeet_gpu",
        "whisper-worker-parakeet-gpu.rs",
        "whisper-worker-parakeet-gpu.exe",
        &gpu_worker_source,
    );
    let gpu_paths = crate::parakeet_import::ParakeetWorkerPaths {
        executable: gpu_worker,
        native_dir: gpu_native,
        model: model_path.clone(),
        runtime_root,
    };
    verify_q03_personal_model(&model_path);
    (
        worker_dir,
        transcribe_marker,
        cpu_worker_sha256,
        cpu_paths,
        gpu_paths,
    )
}

#[cfg(feature = "personal-qualification")]
fn copy_q03_cpu_runtime(install_root: &Path, cpu_native: &Path) {
    for name in [
        "nemo_speech_asr_c.dll",
        "nemo_speech_asr.dll",
        "ggml.dll",
        "ggml-base.dll",
        "ggml-cpu.dll",
    ] {
        fs::copy(
            install_root.join("native/cpu").join(name),
            cpu_native.join(name),
        )
        .expect("copy CPU runtime DLL as read-only qualification data");
    }
}

#[cfg(feature = "personal-qualification")]
fn write_q03_cpu_ready_identity(worker_dir: &Path, key: &crate::archive::ArchiveQualificationKey) {
    let identity = format!(
        r#""model_sha256":"{}","runtime_sha256":"{}","build_sha256":"{}","profile":"{}""#,
        key.artifact_revision, key.runtime_build, key.worker_build, key.inference_profile
    );
    fs::write(worker_dir.join("cpu-ready-identity.json"), identity)
        .expect("write dynamic CPU READY identity");
}

#[cfg(feature = "personal-qualification")]
fn verify_q03_personal_model(model_path: &Path) {
    let model_record =
        whisper_core::model_registry::model(whisper_core::model_catalog::ModelId::ParakeetTdt06BV3);
    assert_eq!(
        crate::model_catalog::cached_model_sha256(model_path)
            .expect("verify copied model")
            .1
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
        model_record.artifact.sha256
    );
}

#[cfg(feature = "personal-qualification")]
fn start_q03_gpu_attempt(
    root: &Path,
    model_path: &Path,
    cpu_paths: crate::parakeet_import::ParakeetWorkerPaths,
    gpu_paths: &crate::parakeet_import::ParakeetWorkerPaths,
) -> (ImportRequest, ImportRuntime) {
    let request = personal_import_request(root, model_path);
    let mut runtime = runtime_with_active_attempt(request.job_id);
    runtime.archive = crate::archive::ArchiveStore::new(&request.destination);
    runtime
        .archive
        .prepare_with_qualification(
            request.clone(),
            None,
            Some(crate::archive::ArchiveQualificationKey::parakeet_gpu_test()),
        )
        .expect("prepare replaceable GPU pending import");
    runtime.parakeet_paths = Some(cpu_paths);
    runtime.parakeet = Some(
        crate::parakeet_import::ParakeetProcess::spawn(gpu_paths)
            .expect("spawn synthetic GPU worker"),
    );
    runtime.active = Some(crate::worker_ipc::runtime::ImportRequestState {
        job_id: request.job_id,
        generation: request.generation,
        instance_id: 7,
        backend: BackendKind::Cuda,
        attempt_number: 1,
        import_request: Some(request.clone()),
        recovery: crate::worker_ipc::runtime::LiveRecoveryState::None,
        startup_ready: false,
        gpu_candidate: None,
        gpu_attested: false,
    });
    (request, runtime)
}

#[cfg(feature = "personal-qualification")]
fn wait_q03_gpu_fallback(
    runtime: &mut ImportRuntime,
    trace: &mut Vec<serde_json::Value>,
    helper_diagnostics: &mut Vec<serde_json::Value>,
) -> bool {
    trace.push(serde_json::json!({
        "phase": "gpu_fallback_wait_started",
        "observation": "fixed_helper_boundary",
    }));
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut fallback_completed = false;
    let mut unchanged_wait_polls = 0_u64;
    while Instant::now() < deadline && !fallback_completed {
        let before = poll_state_snapshot(runtime);
        let result = runtime.poll_parakeet_worker();
        let after = poll_state_snapshot(runtime);
        match result {
            Some(Err(error)) => panic!("GPU Failed or CPU fallback failed: {error:?}"),
            Some(Ok(event)) => panic!("unexpected event before CPU READY: {event:?}"),
            None if runtime
                .active
                .as_ref()
                .is_some_and(|active| active.backend == BackendKind::Cpu) =>
            {
                trace.push(serde_json::json!({
                    "phase": "gpu_failed_cpu_fallback_started",
                    "before": before,
                    "state": after,
                    "fallback_reason": runtime.auto_fallback_reason.as_deref(),
                }));
                fallback_completed = true;
            }
            None => {
                if before != after {
                    trace.push(serde_json::json!({
                        "phase": "fallback_poll_state_transition",
                        "result": null,
                        "before": before,
                        "state": after,
                    }));
                    unchanged_wait_polls = 0;
                } else {
                    unchanged_wait_polls = unchanged_wait_polls.saturating_add(1);
                }
                thread::yield_now();
            }
        }
    }
    helper_diagnostics.push(serde_json::json!({
        "phase": "gpu_fallback_wait",
        "polls_without_state_change": unchanged_wait_polls,
    }));
    fallback_completed
}

#[cfg(feature = "personal-qualification")]
fn assert_q03_cpu_attempt_and_ready(
    request: &ImportRequest,
    runtime: &mut ImportRuntime,
    trace: &mut Vec<serde_json::Value>,
    diagnostics: &mut Vec<serde_json::Value>,
) {
    assert_eq!(
        runtime.active.as_ref().expect("CPU attempt").backend,
        BackendKind::Cpu
    );
    assert_eq!(
        runtime.active.as_ref().expect("CPU attempt").attempt_number,
        1
    );
    assert!(
        runtime
            .auto_fallback_reason
            .as_deref()
            .is_some_and(|reason| { reason.contains("GPU worker failed before native inference") })
    );
    let cpu_ready = poll_traced_until_event(runtime, trace, diagnostics, "cpu_ready");
    assert!(matches!(
        cpu_ready,
        Ok(WorkerEvent::Ready {
            job_id,
            generation,
            backend: BackendKind::Cpu,
            ..
        }) if job_id == request.job_id && generation == request.generation
    ));
}

#[cfg(feature = "personal-qualification")]
fn record_q03_cpu_transcription(
    worker_dir: &Path,
    transcribe_marker: &Path,
    worker_build_sha256: &str,
    request: &ImportRequest,
    runtime: &ImportRuntime,
    trace: &mut Vec<serde_json::Value>,
) {
    let marker_deadline = Instant::now() + Duration::from_secs(3);
    while Instant::now() < marker_deadline && !transcribe_marker.is_file() {
        thread::yield_now();
    }
    let transcribe = fs::read_to_string(transcribe_marker).expect("CPU Transcribe wire marker");
    assert_eq!(transcribe.lines().count(), 1, "exactly one CPU Transcribe");
    assert!(transcribe.contains(&format!("\"job_id\":\"{:032x}\"", request.job_id.0)));
    assert!(
        !runtime.parakeet_native_call_started,
        "fixture never enters native inference"
    );
    assert!(runtime.auto_fallback_reason.is_some());
    let pending_path = Path::new(&request.destination)
        .join("transcriptions")
        .join(format!("{:032x}", request.job_id.0))
        .join(request.generation.get().to_string())
        .join("pending.json");
    let pending: serde_json::Value =
        serde_json::from_slice(&fs::read(pending_path).expect("read persisted pending record"))
            .expect("decode persisted pending record");
    let cpu_key = runtime
        .parakeet_paths
        .as_ref()
        .expect("CPU paths")
        .qualification_key(whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS)
        .expect("CPU qualification key");
    assert_eq!(
        pending["qualification"],
        serde_json::to_value(&cpu_key).expect("serialize CPU key")
    );
    trace.push(serde_json::json!({
        "phase": "cpu_transcribe_and_persisted_qualification",
        "transcribe_commands": transcribe.lines().collect::<Vec<_>>(),
        "pending": pending,
        "cpu_worker_build_sha256": worker_build_sha256,
        "cpu_state": poll_state_snapshot(runtime),
    }));
    for (name, path) in [
        ("gpu_events", worker_dir.join("gpu-events.jsonl")),
        ("gpu_commands", worker_dir.join("gpu-commands.jsonl")),
        ("cpu_commands", worker_dir.join("cpu-commands.jsonl")),
    ] {
        let contents = fs::read_to_string(path).expect("read complete worker trace file");
        trace.push(serde_json::json!({
            "phase": name,
            "lines": contents.lines().map(str::to_owned).collect::<Vec<_>>(),
        }));
    }
}

#[cfg(feature = "personal-qualification")]
fn run_personal_cpu_fallback_child(root: &Path) {
    let (worker_dir, transcribe_marker, cpu_worker_sha256, cpu_paths, gpu_paths) =
        prepare_q03_workers(root);
    let model_path = cpu_paths.model.clone();

    let (request, mut runtime) = start_q03_gpu_attempt(root, &model_path, cpu_paths, &gpu_paths);

    let mut trace = vec![serde_json::json!({
        "phase": "gpu_started",
        "state": poll_state_snapshot(&runtime),
    })];
    let mut helper_diagnostics = Vec::new();
    let fallback_completed =
        wait_q03_gpu_fallback(&mut runtime, &mut trace, &mut helper_diagnostics);
    assert!(fallback_completed, "GPU failure must start the CPU attempt");
    assert_q03_cpu_attempt_and_ready(&request, &mut runtime, &mut trace, &mut helper_diagnostics);
    record_q03_cpu_transcription(
        &worker_dir,
        &transcribe_marker,
        &cpu_worker_sha256,
        &request,
        &runtime,
        &mut trace,
    );
    drop(runtime);
    write_poll_trace(root, "Q03", &trace, &helper_diagnostics);
}

fn compile_gpu_ready_worker(
    worker_root: &Path,
    name: &str,
    identity: &serde_json::Value,
) -> PathBuf {
    let identity = serde_json::to_string(identity).expect("encode synthetic GPU identity");
    let identity_literal = serde_json::to_string(&identity).expect("encode GPU identity literal");
    let pci_bus_json =
        serde_json::to_string(whisper_core::model_registry::PARAKEET_TEST_GPU_PCI_BUS_ID)
            .expect("encode GPU PCI identity");
    let pci_bus_literal = serde_json::to_string(&pci_bus_json).expect("encode PCI Rust literal");
    let gpu_uuid_json = serde_json::to_string(whisper_core::model_registry::PARAKEET_TEST_GPU_UUID)
        .expect("encode GPU UUID");
    let gpu_uuid_literal = serde_json::to_string(&gpu_uuid_json).expect("encode UUID Rust literal");
    let source = r###"
use std::{fs, io::{self, BufRead, Write}};
const IDENTITY: &str = __IDENTITY__;
const PCI_BUS_ID: &str = __PCI_BUS_ID__;
const GPU_UUID: &str = __GPU_UUID__;
fn main() {
    let worker_dir = std::env::current_exe().expect("worker path").parent().expect("worker dir").to_path_buf();
    let mut frame = String::from(r#"{"version":1,"message":{"type":"ready","identity":"#);
    frame.push_str(IDENTITY);
    frame.push_str(",\"backend\":\"CUDA0\",\"gpu_evidence\":{\"backend\":\"CUDA0\",\"pci_bus_id\":");
    frame.push_str(PCI_BUS_ID);
    frame.push_str(",\"uuid\":");
    frame.push_str(GPU_UUID);
    frame.push_str(",\"pid\":");
    frame.push_str(&std::process::id().to_string());
    frame.push_str(",\"asr_log_sha256\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\"},\"native_version\":\"fixture\"}}");
    let frame_pending = worker_dir.join("gpu-ready-frame.pending");
    fs::write(&frame_pending, &frame).expect("record emitted READY frame");
    fs::rename(frame_pending, worker_dir.join("gpu-ready-frame.json")).expect("publish complete READY frame");
    println!("{frame}");
    let _ = io::stdout().flush();
    for line in io::stdin().lock().lines().flatten() {
        if line.contains("\"type\":\"transcribe\"") {
            let command_pending = worker_dir.join("gpu-transcribe-command.pending");
            fs::write(&command_pending, &line).expect("record received Transcribe command");
            fs::rename(command_pending, worker_dir.join("gpu-transcribe-command.json")).expect("publish complete Transcribe command");
            return;
        }
    }
}
"###
    .replace("__IDENTITY__", &identity_literal)
    .replace("__PCI_BUS_ID__", &pci_bus_literal)
    .replace("__GPU_UUID__", &gpu_uuid_literal);
    compile_worker(worker_root, name, &source)
}

fn write_pending_gpu_ready_trace(
    root: &Path,
    label: &str,
    trace: &[serde_json::Value],
    helper_diagnostics: &[serde_json::Value],
) {
    let stable = persist_gpu_ready_trace(root, label, trace, helper_diagnostics);
    assert_frozen_gpu_ready_trace(label, &stable);
}

#[cfg(feature = "personal-qualification")]
fn write_pending_personal_gpu_ready_trace(
    root: &Path,
    label: &str,
    trace: &[serde_json::Value],
    helper_diagnostics: &[serde_json::Value],
) {
    let stable = persist_gpu_ready_trace(root, label, trace, helper_diagnostics);
    assert_frozen_personal_gpu_ready_trace(label, &stable);
}

fn persist_gpu_ready_trace(
    root: &Path,
    label: &str,
    trace: &[serde_json::Value],
    helper_diagnostics: &[serde_json::Value],
) -> Vec<u8> {
    let parent = root.parent().expect("test root has TEMP parent");
    let root_name = root.file_name().expect("test root name").to_string_lossy();
    let raw_path = parent.join(format!("poll-parakeet-{label}-{root_name}-raw.json"));
    let stable_path = parent.join(format!("poll-parakeet-{label}-{root_name}-stable.json"));
    let raw = serde_json::to_vec_pretty(&serde_json::json!({
        "trace": trace,
        "helper_diagnostics": helper_diagnostics,
    }))
    .expect("serialize raw GPU Ready trace");
    let mut stable_value = serde_json::Value::Array(trace.to_vec());
    replace_trace_roots(&mut stable_value, root);
    remap_gpu_ready_frame_pid(&mut stable_value);
    let stable =
        serde_json::to_vec_pretty(&stable_value).expect("serialize stable GPU Ready trace");
    let stable_text = String::from_utf8_lossy(&stable);
    for root_form in trace_root_forms(root) {
        assert!(
            !stable_text.contains(&root_form),
            "normalized GPU Ready trace retained temporary root spelling"
        );
    }
    fs::write(raw_path, raw).expect("persist raw GPU Ready trace in lot TEMP");
    fs::write(stable_path, &stable).expect("persist stable GPU Ready trace in lot TEMP");
    stable
}

fn remap_gpu_ready_frame_pid(trace: &mut serde_json::Value) {
    let Some(entries) = trace.as_array_mut() else {
        return;
    };
    for entry in entries {
        let reported_pid = entry
            .get("emitted_frame")
            .and_then(serde_json::Value::as_str)
            .and_then(|frame| serde_json::from_str::<serde_json::Value>(frame).ok())
            .and_then(|frame| frame["message"]["gpu_evidence"]["pid"].as_u64());
        if reported_pid.is_none() {
            continue;
        }
        let pid = reported_pid.expect("checked GPU PID");
        if let Some(frame) = entry
            .get("emitted_frame")
            .and_then(serde_json::Value::as_str)
        {
            let marker = format!("\"pid\":{pid}");
            assert_eq!(
                frame.matches(&marker).count(),
                1,
                "one PID position in GPU frame"
            );
            let stable_frame = frame.replacen(&marker, "\"pid\":\"<GPU-PID-1>\"", 1);
            *entry.get_mut("emitted_frame").expect("frame field exists") =
                serde_json::Value::String(stable_frame);
        }
        if let Some(worker_pid) = entry.get("worker_pid").and_then(serde_json::Value::as_u64) {
            assert_eq!(worker_pid, pid, "frame and process PID evidence agree");
            entry["worker_pid"] = serde_json::json!("<GPU-PID-1>");
        }
    }
}

fn configure_synthetic_gpu_ready_runtime(
    root: &Path,
    job_id: JobId,
    compute: ComputeChoice,
    invalid_build: bool,
) -> (ImportRequest, ImportRuntime, PathBuf, PathBuf) {
    let (mut request, mut runtime) = request_runtime(root, job_id);
    request.config.compute = compute;
    let active = runtime.active.as_mut().expect("active GPU attempt");
    active.backend = BackendKind::Cuda;
    active.attempt_number = 1;
    active.import_request = Some(request.clone());
    let mut identity =
        parakeet_test_identity(whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS);
    if invalid_build {
        identity["build_sha256"] = serde_json::json!("0".repeat(64));
    }
    let worker_root = synthetic_worker_root(root);
    let worker = compile_gpu_ready_worker(&worker_root, "synthetic-gpu-ready", &identity);
    let paths = synthetic_parakeet_paths(worker);
    let gpu_key = paths
        .gpu_qualification_key(whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS);
    assert!(
        gpu_key.is_err(),
        "synthetic GPU fixture must use the existing CPU test-key fallback"
    );
    runtime.gpu_parakeet_paths = Some(paths.clone());
    runtime.parakeet_expected_gpu = Some(whisper_core::parakeet_ipc::ExpectedGpu {
        pci_bus_id: whisper_core::model_registry::PARAKEET_TEST_GPU_PCI_BUS_ID.to_owned(),
        uuid: whisper_core::model_registry::PARAKEET_TEST_GPU_UUID.to_owned(),
    });
    runtime.parakeet = Some(
        crate::parakeet_import::ParakeetProcess::spawn_live(&paths, root)
            .expect("spawn synthetic GPU Ready worker"),
    );
    let worker_dir = paths
        .executable
        .parent()
        .expect("synthetic worker dir")
        .to_path_buf();
    let frame_path = worker_dir.join("gpu-ready-frame.json");
    let command_path = worker_dir.join("gpu-transcribe-command.json");
    (request, runtime, frame_path, command_path)
}

fn poll_until_gpu_result_or_fallback(
    runtime: &mut ImportRuntime,
    trace: &mut Vec<serde_json::Value>,
    helper_diagnostics: &mut Vec<serde_json::Value>,
) -> Option<Result<WorkerEvent, PortError>> {
    trace.push(serde_json::json!({
        "phase": "gpu_ready_wait_started",
        "observation": "fixed_helper_boundary",
    }));
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut unchanged_polls = 0_u64;
    while Instant::now() < deadline {
        let before = poll_state_snapshot(runtime);
        let result = runtime.poll_parakeet_worker();
        let after = poll_state_snapshot(runtime);
        let fallback_started = runtime
            .active
            .as_ref()
            .is_some_and(|active| active.backend == BackendKind::Cpu);
        if let Some(result) = result {
            trace.push(serde_json::json!({
                "phase": "gpu_ready_poll_result",
                "before": before,
                "result": format!("{result:?}"),
                "after": after,
            }));
            helper_diagnostics.push(serde_json::json!({
                "phase": "gpu_ready_wait",
                "polls_without_state_change": unchanged_polls,
            }));
            return Some(result);
        }
        if before != after {
            trace.push(serde_json::json!({
                "phase": "gpu_ready_poll_transition",
                "result": null,
                "before": before,
                "after": after,
            }));
            helper_diagnostics.push(serde_json::json!({
                "phase": "gpu_ready_wait",
                "polls_without_state_change": unchanged_polls,
            }));
            if fallback_started {
                return None;
            }
            unchanged_polls = 0;
            continue;
        }
        unchanged_polls = unchanged_polls.saturating_add(1);
        thread::yield_now();
    }
    panic!("timed out polling synthetic GPU READY")
}

fn assert_gpu_transcribe_command(
    raw: &str,
    request: &ImportRequest,
    identity: &serde_json::Value,
    expected_gpu: &whisper_core::parakeet_ipc::ExpectedGpu,
) {
    let envelope: serde_json::Value =
        serde_json::from_str(raw).expect("decode Transcribe envelope");
    let source_sha256 = request
        .source_sha256
        .as_ref()
        .expect("fixture source SHA")
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(
        envelope,
        serde_json::json!({
            "version": 1,
            "message": {
                "type": "transcribe",
                "request_id": 1,
                "job_id": format!("{:032x}", request.job_id.0),
                "generation": request.generation.get(),
                "attempt": 1,
                "identity": identity,
                "source_path": request.source_path,
                "source_sha256": source_sha256,
                "expected_gpu": expected_gpu,
            }
        })
    );
}

fn record_gpu_ready_frame(
    trace: &mut Vec<serde_json::Value>,
    frame: String,
    runtime: &ImportRuntime,
) {
    let parsed: serde_json::Value = serde_json::from_str(&frame).expect("decode READY frame");
    let reported_pid = parsed["message"]["gpu_evidence"]["pid"]
        .as_u64()
        .expect("worker PID");
    let worker_pid = runtime.parakeet.as_ref().expect("GPU process").pid();
    assert_eq!(u64::from(worker_pid), reported_pid);
    trace.push(serde_json::json!({
        "phase": "gpu_ready_frame_emitted",
        "worker_pid": worker_pid,
        "emitted_frame": frame,
    }));
}

#[test]
fn poll_parakeet_gpu_ready_attests_worker_pid_and_transcribes_once() {
    let root = TestRoot::new("parakeet-gpu-ready-attested");
    let (request, mut runtime, frame_path, command_path) =
        configure_synthetic_gpu_ready_runtime(&root.0, JobId(73_300), ComputeChoice::Auto, false);
    let identity =
        parakeet_test_identity(whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS);
    let expected_gpu = runtime.parakeet_expected_gpu.clone().expect("expected GPU");
    let mut trace = vec![serde_json::json!({
        "phase": "P07_gpu_ready_fixture_started",
        "key_route": "ArchiveQualificationKey::parakeet_cpu_test fallback",
        "state": poll_state_snapshot(&runtime),
    })];
    let mut diagnostics = Vec::new();
    assert!(wait_for_marker(&frame_path, Duration::from_secs(8)));
    let emitted_frame = fs::read_to_string(&frame_path).expect("read exact emitted READY frame");
    record_gpu_ready_frame(&mut trace, emitted_frame, &runtime);
    let outcome = poll_until_gpu_result_or_fallback(&mut runtime, &mut trace, &mut diagnostics);
    assert!(
        matches!(&outcome, Some(Ok(WorkerEvent::Ready { job_id, backend: BackendKind::Cuda, .. })) if *job_id == request.job_id)
    );
    assert!(
        runtime
            .active
            .as_ref()
            .is_some_and(|active| active.gpu_attested)
    );
    assert!(!runtime.parakeet_native_call_started);
    assert!(runtime.auto_fallback_reason.is_none());
    assert!(wait_for_marker(&command_path, Duration::from_secs(8)));
    let command = fs::read_to_string(&command_path).expect("read exact Transcribe command");
    assert_gpu_transcribe_command(&command, &request, &identity, &expected_gpu);
    trace.push(serde_json::json!({ "phase": "gpu_transcribe_command", "transcribe_command": command, "state": poll_state_snapshot(&runtime) }));
    write_pending_gpu_ready_trace(&root.0, "P07-gpu-ready-attested", &trace, &diagnostics);
}

#[test]
fn poll_parakeet_gpu_ready_key_mismatch_does_not_fallback_in_manual_mode() {
    let root = TestRoot::new("parakeet-gpu-ready-manual-mismatch");
    let (_, mut runtime, frame_path, _) =
        configure_synthetic_gpu_ready_runtime(&root.0, JobId(73_301), ComputeChoice::Gpu, true);
    let mut trace = vec![serde_json::json!({
        "phase": "P08_gpu_ready_manual_fixture_started",
        "key_route": "ArchiveQualificationKey::parakeet_cpu_test fallback",
        "state": poll_state_snapshot(&runtime),
    })];
    let mut diagnostics = Vec::new();
    assert!(wait_for_marker(&frame_path, Duration::from_secs(8)));
    let emitted_frame = fs::read_to_string(&frame_path).expect("read exact emitted READY frame");
    record_gpu_ready_frame(&mut trace, emitted_frame, &runtime);
    let outcome = poll_until_gpu_result_or_fallback(&mut runtime, &mut trace, &mut diagnostics);
    assert!(
        matches!(&outcome, Some(Err(PortError::Failed(message))) if message == "BackendMismatch: Parakeet worker identity differs from the qualified key")
    );
    assert!(
        runtime
            .active
            .as_ref()
            .is_some_and(|active| active.backend == BackendKind::Cuda && !active.gpu_attested)
    );
    assert!(runtime.parakeet.is_some());
    assert!(runtime.auto_fallback_reason.is_none());
    assert!(!runtime.parakeet_native_call_started);
    trace.push(serde_json::json!({ "phase": "gpu_ready_manual_rejected", "state": poll_state_snapshot(&runtime) }));
    write_pending_gpu_ready_trace(
        &root.0,
        "P08-gpu-ready-manual-rejected",
        &trace,
        &diagnostics,
    );
}

#[cfg(feature = "personal-qualification")]
#[test]
fn poll_parakeet_personal_gpu_ready_auto_fallback_and_oversized_transcribe() {
    if let Some(root) = std::env::var_os(GPU_READY_CHILD_ROOT) {
        run_personal_gpu_ready_cases(Path::new(&root));
        return;
    }

    let root = TestRoot::new("pk-auto");
    let model = std::env::var_os("N1_TEST_PERSONAL_PARAKEET_MODEL")
        .map(PathBuf::from)
        .expect("guard supplies the validated Personal Parakeet model");
    let local_app_data = root.0.join("isolated-localappdata");
    let runtime_root = local_app_data.join("Whisper");
    let output = Command::new(std::env::current_exe().expect("test executable path"))
        .arg("--exact")
        .arg("worker_ipc::runtime::poll::tests::parakeet_equivalence::poll_parakeet_personal_gpu_ready_auto_fallback_and_oversized_transcribe")
        .arg("--nocapture")
        .env(GPU_READY_CHILD_ROOT, &root.0)
        .env("LOCALAPPDATA", &local_app_data)
        .env("PARAKEET_L2_RUNTIME_ROOT", &runtime_root)
        .env(
            "PARAKEET_L2_MODEL",
            runtime_root.join("models/parakeet-tdt-0.6b-v3.q8_0.gguf"),
        )
        .env(
            "PARAKEET_L2_CPU_WORKER",
            runtime_root.join("worker/whisper-worker-parakeet-cpu.exe"),
        )
        .env("PARAKEET_L2_CPU_NATIVE_DIR", runtime_root.join("native/cpu"))
        .env("N1_TEST_PERSONAL_PARAKEET_MODEL", model)
        .output()
        .expect("run isolated personal GPU Ready child");
    assert!(
        output.status.success(),
        "personal GPU Ready child failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"),
        "personal GPU Ready child did not execute exactly one test:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[cfg(feature = "personal-qualification")]
fn run_personal_gpu_ready_cases(root: &Path) {
    let (worker_dir, transcribe_marker, cpu_worker_sha256, cpu_paths, _) =
        prepare_q03_workers(root);
    fs::write(worker_dir.join("gpu-events.jsonl"), b"")
        .expect("create empty Q03 GPU event log for Ready fixture");
    let auto_root = root.join("auto-ready-mismatch");
    let oversized_root = root.join("oversized-transcribe");
    fs::create_dir_all(&auto_root).expect("create Auto Ready fixture root");
    fs::create_dir_all(&oversized_root).expect("create oversized Transcribe fixture root");
    run_personal_auto_ready_mismatch(
        &auto_root,
        root,
        &worker_dir,
        &transcribe_marker,
        &cpu_worker_sha256,
        &cpu_paths,
    );
    run_personal_oversized_transcribe(&oversized_root, root, &cpu_paths);
}

#[cfg(feature = "personal-qualification")]
fn personal_gpu_ready_runtime(
    root: &Path,
    cpu_paths: &crate::parakeet_import::ParakeetWorkerPaths,
    job_id: JobId,
    compute: ComputeChoice,
    invalid_build: bool,
) -> (ImportRequest, ImportRuntime, PathBuf, PathBuf) {
    let mut request = personal_import_request(root, &cpu_paths.model);
    request.job_id = job_id;
    request.config.compute = compute;
    let mut runtime = runtime_with_active_attempt(job_id);
    runtime.archive = crate::archive::ArchiveStore::new(&request.destination);
    runtime
        .archive
        .prepare_with_qualification(
            request.clone(),
            None,
            Some(crate::archive::ArchiveQualificationKey::parakeet_gpu_test()),
        )
        .expect("prepare replaceable GPU pending import");
    runtime.parakeet_paths = Some(cpu_paths.clone());
    runtime.gpu_parakeet_paths = None;
    let active = runtime.active.as_mut().expect("active GPU attempt");
    active.backend = BackendKind::Cuda;
    active.attempt_number = 1;
    active.import_request = Some(request.clone());

    let mut identity =
        parakeet_test_identity(whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS);
    if invalid_build {
        identity["build_sha256"] = serde_json::json!("0".repeat(64));
    }
    let worker = compile_gpu_ready_worker(
        &synthetic_worker_root(root),
        "personal-gpu-ready",
        &identity,
    );
    let paths = synthetic_parakeet_paths(worker);
    runtime.parakeet_expected_gpu = Some(whisper_core::parakeet_ipc::ExpectedGpu {
        pci_bus_id: whisper_core::model_registry::PARAKEET_TEST_GPU_PCI_BUS_ID.to_owned(),
        uuid: whisper_core::model_registry::PARAKEET_TEST_GPU_UUID.to_owned(),
    });
    runtime.parakeet = Some(
        crate::parakeet_import::ParakeetProcess::spawn_live(&paths, root)
            .expect("spawn synthetic personal GPU Ready worker"),
    );
    let worker_dir = paths.executable.parent().expect("GPU worker directory");
    (
        request,
        runtime,
        worker_dir.join("gpu-ready-frame.json"),
        worker_dir.join("gpu-transcribe-command.json"),
    )
}

#[cfg(feature = "personal-qualification")]
fn run_personal_auto_ready_mismatch(
    root: &Path,
    trace_root: &Path,
    worker_dir: &Path,
    transcribe_marker: &Path,
    cpu_worker_sha256: &str,
    cpu_paths: &crate::parakeet_import::ParakeetWorkerPaths,
) {
    let (request, mut runtime, frame_path, command_path) =
        personal_gpu_ready_runtime(root, cpu_paths, JobId(73_320), ComputeChoice::Auto, true);
    let mut trace = vec![serde_json::json!({
        "phase": "P08_auto_ready_mismatch_fixture_started",
        "gpu_key_route": "ArchiveQualificationKey::parakeet_cpu_test fallback",
        "state": poll_state_snapshot(&runtime),
    })];
    let mut diagnostics = Vec::new();
    assert!(wait_for_marker(&frame_path, Duration::from_secs(8)));
    record_gpu_ready_frame(
        &mut trace,
        fs::read_to_string(&frame_path).expect("read emitted GPU READY frame"),
        &runtime,
    );
    let outcome = poll_until_gpu_result_or_fallback(&mut runtime, &mut trace, &mut diagnostics);
    assert!(outcome.is_none(), "Auto fallback outcome: {outcome:?}");
    assert_eq!(
        runtime.auto_fallback_reason.as_deref(),
        Some("GPU worker Ready proof did not match the qualified key")
    );
    assert!(runtime.active.as_ref().is_some_and(|active| {
        active.backend == BackendKind::Cpu && active.attempt_number == 1 && !active.gpu_attested
    }));
    assert!(runtime.parakeet.is_some(), "CPU retry worker is started");
    assert!(!runtime.parakeet_native_call_started);
    assert!(
        !command_path.exists(),
        "GPU mismatch emits no Transcribe command"
    );
    assert_personal_cpu_ready(&request, &mut runtime, &mut trace, &mut diagnostics);
    record_q03_cpu_transcription(
        worker_dir,
        transcribe_marker,
        cpu_worker_sha256,
        &request,
        &runtime,
        &mut trace,
    );
    trace.push(serde_json::json!({
        "phase": "P08_auto_ready_mismatch_after_fallback",
        "state": poll_state_snapshot(&runtime),
        "gpu_transcribe_command": null,
    }));
    write_pending_personal_gpu_ready_trace(
        trace_root,
        "P08-auto-ready-mismatch",
        &trace,
        &diagnostics,
    );
}

#[cfg(feature = "personal-qualification")]
fn run_personal_oversized_transcribe(
    root: &Path,
    trace_root: &Path,
    cpu_paths: &crate::parakeet_import::ParakeetWorkerPaths,
) {
    let (mut request, mut runtime, frame_path, command_path) =
        personal_gpu_ready_runtime(root, cpu_paths, JobId(73_321), ComputeChoice::Auto, false);
    request.source_path = "x".repeat(whisper_core::parakeet_ipc::PARAKEET_MAX_FRAME_BYTES);
    request.source_samples = None;
    runtime
        .active
        .as_mut()
        .expect("active GPU request")
        .import_request = Some(request.clone());
    let mut trace = vec![serde_json::json!({
        "phase": "P09_over_limit_fixture_started",
        "source_samples": request.source_samples,
        "source_path_bytes": request.source_path.len(),
        "state": poll_state_snapshot(&runtime),
    })];
    let mut diagnostics = Vec::new();
    assert!(wait_for_marker(&frame_path, Duration::from_secs(8)));
    let emitted_frame = fs::read_to_string(&frame_path).expect("read emitted GPU READY frame");
    record_gpu_ready_frame(&mut trace, emitted_frame.clone(), &runtime);
    let attempted = attempted_gpu_transcribe_frame(
        &request,
        &emitted_frame,
        runtime
            .parakeet_expected_gpu
            .as_ref()
            .expect("expected GPU"),
    );
    assert!(
        attempted.len() + 1 > whisper_core::parakeet_ipc::PARAKEET_MAX_FRAME_BYTES,
        "actual Transcribe envelope plus newline exceeds the IPC frame limit"
    );
    trace.push(serde_json::json!({
        "phase": "P09_transcribe_rejected_before_write",
        "attempted_frame": String::from_utf8(attempted).expect("Transcribe JSON is UTF-8"),
        "write_error": "IpcFrameTooLarge",
    }));
    let outcome = poll_until_gpu_result_or_fallback(&mut runtime, &mut trace, &mut diagnostics);
    assert!(
        matches!(
            &outcome,
            Some(Err(PortError::Failed(message)))
                if message == "InvalidInput: source duration was not prepared"
        ),
        "P09 fallback preflight outcome: {outcome:?}"
    );
    assert_eq!(
        runtime.auto_fallback_reason.as_deref(),
        Some("GPU request preparation failed: IpcFrameTooLarge")
    );
    assert!(runtime.active.is_none());
    assert!(runtime.parakeet.is_none());
    assert!(!runtime.parakeet_native_call_started);
    assert!(
        !command_path.exists(),
        "oversized Transcribe was not written"
    );
    record_personal_cpu_pending(&request, &runtime, &mut trace);
    trace.push(serde_json::json!({
        "phase": "P09_fallback_preflight_failed",
        "state": poll_state_snapshot(&runtime),
    }));
    write_pending_personal_gpu_ready_trace(
        trace_root,
        "P09-ready-transcribe-over-limit",
        &trace,
        &diagnostics,
    );
}

#[cfg(feature = "personal-qualification")]
fn assert_personal_cpu_ready(
    request: &ImportRequest,
    runtime: &mut ImportRuntime,
    trace: &mut Vec<serde_json::Value>,
    diagnostics: &mut Vec<serde_json::Value>,
) {
    let active = runtime.active.as_ref().expect("CPU fallback attempt");
    assert_eq!(active.backend, BackendKind::Cpu);
    assert_eq!(active.attempt_number, 1);
    assert!(runtime.auto_fallback_reason.is_some());
    let result =
        poll_traced_until_event(runtime, trace, diagnostics, "cpu_ready_after_gpu_fallback");
    assert!(
        matches!(
            &result,
            Ok(WorkerEvent::Ready { job_id, generation, backend: BackendKind::Cpu, .. })
                if *job_id == request.job_id && *generation == request.generation
        ),
        "CPU retry READY outcome: {result:?}"
    );
}

#[cfg(feature = "personal-qualification")]
fn attempted_gpu_transcribe_frame(
    request: &ImportRequest,
    ready_frame: &str,
    expected_gpu: &whisper_core::parakeet_ipc::ExpectedGpu,
) -> Vec<u8> {
    let ready: serde_json::Value = serde_json::from_str(ready_frame).expect("decode READY frame");
    let identity: whisper_core::parakeet_ipc::WorkerIdentity =
        serde_json::from_value(ready["message"]["identity"].clone())
            .expect("decode READY worker identity");
    let source_sha256 = request
        .source_sha256
        .as_ref()
        .expect("source hash prepared")
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let command = whisper_core::parakeet_ipc::Command::Transcribe {
        request_id: 1,
        job_id: format!("{:032x}", request.job_id.0),
        generation: request.generation.get(),
        attempt: 1,
        identity: Box::new(identity),
        source_path: request.source_path.clone(),
        source_sha256,
        expected_gpu: Some(expected_gpu.clone()),
    };
    serde_json::to_vec(&whisper_core::parakeet_ipc::Envelope::new(command))
        .expect("serialize attempted Transcribe frame")
}

#[cfg(feature = "personal-qualification")]
fn record_personal_cpu_pending(
    request: &ImportRequest,
    runtime: &ImportRuntime,
    trace: &mut Vec<serde_json::Value>,
) {
    let pending_path = Path::new(&request.destination)
        .join("transcriptions")
        .join(format!("{:032x}", request.job_id.0))
        .join(request.generation.get().to_string())
        .join("pending.json");
    let pending: serde_json::Value = serde_json::from_slice(
        &fs::read(pending_path).expect("read persisted fallback qualification"),
    )
    .expect("decode persisted fallback qualification");
    let cpu_key = runtime
        .parakeet_paths
        .as_ref()
        .expect("personal CPU paths")
        .qualification_key(whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS)
        .expect("dynamic personal CPU qualification key");
    assert_eq!(
        pending["qualification"],
        serde_json::to_value(&cpu_key).expect("serialize personal CPU key")
    );
    trace.push(serde_json::json!({
        "phase": "personal_cpu_fallback_pending",
        "qualification": pending["qualification"],
        "state": poll_state_snapshot(runtime),
    }));
}
