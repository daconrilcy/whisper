use super::*;
use std::{
    fs,
    process::Command,
    thread,
    time::{Duration, Instant},
};

fn compile_parakeet_fixture(root: &Path, mode: &str) -> PathBuf {
    let worker_root = root.join("runtime/worker");
    fs::create_dir_all(&worker_root).expect("create runtime worker fixture directory");
    let source = worker_root.join(format!("parakeet-{mode}.rs"));
    let executable = worker_root.join(format!("parakeet-{mode}.exe"));
    let program = r#"
use std::io::{self, Write};
const MODE: &str = "__MODE__";
fn main() {
    if MODE == "quiet" {
        std::thread::park();
    } else if MODE == "invalid" {
        println!("{{invalid frame}}");
        let _ = io::stdout().flush();
    }
}
"#
    .replace("__MODE__", mode);
    fs::write(&source, program).expect("write fake Parakeet worker");
    let output = Command::new("rustc")
        .args(["--edition=2021", "--crate-name", "poll_parakeet_worker"])
        .arg(&source)
        .arg("-o")
        .arg(&executable)
        .output()
        .expect("compile fake Parakeet worker");
    assert!(
        output.status.success(),
        "rustc fake Parakeet worker: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    executable
}

fn parakeet_paths(root: &Path, worker: PathBuf) -> crate::parakeet_import::ParakeetWorkerPaths {
    let runtime_root = root.join("runtime");
    let native_dir = runtime_root.join("native");
    let model = runtime_root.join("model.gguf");
    fs::create_dir_all(&native_dir).expect("create native fixture directory");
    fs::write(&model, b"unused synthetic model path").expect("create model fixture");
    crate::parakeet_import::ParakeetWorkerPaths {
        executable: worker,
        native_dir,
        model,
        runtime_root,
    }
}

fn compile_ready_parakeet_fixture(root: &Path) -> PathBuf {
    let worker_root = root.join("runtime/worker");
    fs::create_dir_all(&worker_root).expect("create runtime worker fixture directory");
    let source = worker_root.join("parakeet-ready.rs");
    let executable = worker_root.join("parakeet-ready.exe");
    let program = r##"
use std::{io::{self, BufRead, Write}, thread, time::Duration};
fn main() {
    let marker = std::env::current_exe()
        .expect("fixture executable path")
        .parent().expect("worker directory")
        .parent().expect("runtime directory")
        .join("ready.release");
    while !marker.is_file() {
        thread::sleep(Duration::from_millis(2));
    }
    const READY: &str = r#"{"version":1,"message":{"type":"ready","identity":{"model_sha256":"fixture","runtime_sha256":"fixture","build_sha256":"fixture","profile":"import-single-pass"},"backend":"CPU","native_version":"fixture"}}"#;
    println!("{READY}");
    let _ = io::stdout().flush();
    for line in io::stdin().lock().lines().flatten() {
        if line.contains("\"type\":\"shutdown\"") {
            break;
        }
    }
}
"##;
    fs::write(&source, program).expect("write ready Parakeet worker fixture");
    let output = Command::new("rustc")
        .args([
            "--edition=2021",
            "--crate-name",
            "poll_parakeet_ready_worker",
        ])
        .arg(&source)
        .arg("-o")
        .arg(&executable)
        .output()
        .expect("compile ready Parakeet worker fixture");
    assert!(
        output.status.success(),
        "rustc ready Parakeet worker fixture: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    executable
}

#[test]
fn poll_collects_queued_parakeet_event_before_legacy_transport() {
    let root = TestRoot::new("parakeet-event");
    let job_id = JobId(73_201);
    let generation = Generation::first();
    let mut runtime = runtime_with_active_attempt(job_id);
    let legacy_worker = compile_idle_worker(&root.0);
    runtime.transport = Some(
        crate::worker_ipc::ipc::ChildWorkerTransport::spawn(&legacy_worker)
            .expect("spawn legacy worker transport"),
    );
    runtime
        .transport
        .as_mut()
        .expect("legacy transport")
        .inject_event(WorkerEvent::Progress {
            job_id,
            generation,
            instance_id: 7,
            completed_samples: 240,
            total_samples: 320,
        });
    runtime.parakeet_events.push_back(WorkerEvent::Progress {
        job_id,
        generation,
        instance_id: 7,
        completed_samples: 160,
        total_samples: 320,
    });

    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::Progress {
            job_id: actual,
            completed_samples: 160,
            total_samples: 320,
            ..
        })) if actual == job_id
    ));
    assert!(runtime.parakeet_events.is_empty());
    assert!(runtime.active.is_some());
    assert!(runtime.transport.is_some());
    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::Progress {
            job_id: actual,
            completed_samples: 240,
            total_samples: 320,
            ..
        })) if actual == job_id
    ));
    assert!(runtime.transport.is_some());
}

#[test]
fn poll_returns_none_while_parakeet_worker_is_alive_without_an_event() {
    let root = TestRoot::new("parakeet-none");
    let worker = compile_parakeet_fixture(&root.0, "quiet");
    let paths = parakeet_paths(&root.0, worker);
    let mut runtime = runtime_with_active_attempt(JobId(73_202));
    runtime.parakeet = Some(
        crate::parakeet_import::ParakeetProcess::spawn_live(&paths, &root.0)
            .expect("spawn quiet Parakeet process"),
    );

    assert!(runtime.poll().is_none());
    assert!(runtime.parakeet.is_some());
    assert!(runtime.active.is_some());
}

#[test]
fn poll_cleans_active_parakeet_state_after_transport_decode_error() {
    let root = TestRoot::new("parakeet-error");
    let worker = compile_parakeet_fixture(&root.0, "invalid");
    let paths = parakeet_paths(&root.0, worker);
    let mut runtime = runtime_with_active_attempt(JobId(73_203));
    runtime.parakeet = Some(
        crate::parakeet_import::ParakeetProcess::spawn_live(&paths, &root.0)
            .expect("spawn invalid-frame Parakeet process"),
    );

    let deadline = Instant::now() + Duration::from_secs(3);
    let mut failure = None;
    while Instant::now() < deadline {
        if let Some(result) = runtime.poll() {
            failure = Some(result);
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }
    assert!(matches!(
        failure,
        Some(Err((JobId(73_203), _, message))) if message.contains("IpcDecode")
    ));
    assert!(runtime.parakeet.is_none());
    assert!(runtime.active.is_none());
    assert!(runtime.parakeet_events.is_empty());
}

#[test]
fn poll_parakeet_window_write_error_returns_stopped_event_with_automatic_final_import() {
    const CHILD_ROOT: &str = "WHISPER_POLL_PARAKEET_WINDOW_ERROR_CHILD_ROOT";

    if let Some(path) = std::env::var_os(CHILD_ROOT) {
        run_parakeet_window_write_error_child(Path::new(&path));
        return;
    }

    let root = TestRoot::new("parakeet-window-write-child");
    let output = Command::new(std::env::current_exe().expect("test executable path"))
        .arg("--exact")
        .arg("worker_ipc::runtime::poll::tests::parakeet_poll::poll_parakeet_window_write_error_returns_stopped_event_with_automatic_final_import")
        .arg("--nocapture")
        .env(CHILD_ROOT, &root.0)
        .env("WHISPER_MIC_LEVEL_D2_ROOT", &root.0)
        .output()
        .expect("run exact D2 child test");
    assert!(
        output.status.success(),
        "D2 child test failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("1 passed; 0 failed"),
        "D2 child did not execute exactly one test:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
}

fn run_parakeet_window_write_error_child(root: &Path) {
    let job_id = JobId(73_204);
    let generation = Generation::first();
    let worker = compile_ready_parakeet_fixture(root);
    let native_dir = root.join("runtime/native");
    let model = root.join("runtime/model.gguf");
    fs::create_dir_all(worker.parent().expect("worker parent")).expect("create worker path");
    fs::create_dir_all(&native_dir).expect("create native path");
    fs::write(&model, b"unused synthetic model path").expect("create model path");
    let request = whisper_core::ports::LiveRequest {
        job_id,
        generation,
        model_path: model.to_string_lossy().into_owned(),
        model_sha256: [0; 32],
        destination: root.to_string_lossy().into_owned(),
        group_offset_samples: 0,
        auto_stop_after_speech_samples: None,
        microphone: whisper_core::MicrophoneChoice::Device("test:parakeet-poll".into()),
        microphone_gain_db: 0,
        config: whisper_core::JobConfig {
            language: whisper_core::LanguageChoice::Automatic,
            compute: whisper_core::ComputeChoice::Cpu,
        },
    };
    let paths = crate::parakeet_import::ParakeetWorkerPaths {
        executable: worker,
        native_dir,
        model,
        runtime_root: root.join("runtime"),
    };
    let mut capture = crate::capture::CaptureStream::empty_test_capture(
        16_000,
        1,
        "test:parakeet-poll-window-write-error",
    );
    const INJECTED_SAMPLES: usize = 48_000;
    capture.enqueue_test_block(vec![0.25; INJECTED_SAMPLES]);
    let live = crate::parakeet_live::ParakeetLive::start_test_capture(request, paths, 17, capture)
        .expect("start test Parakeet live capture with guarded D2 root");
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: root.join("unused-cpu.exe"),
            gpu: root.join("unused-gpu.exe"),
        },
        Arc::new(PublicationGate::default()),
    );
    runtime.parakeet_live = Some(live);

    assert!(matches!(
        runtime.poll(),
        Some(Ok(ImportEvent::ParakeetLiveStarted {
            job_id: actual,
            generation: actual_generation,
            session_id: 17,
            ..
        })) if actual == job_id && actual_generation == generation
    ));
    assert!(crate::worker_ipc::service::ServiceRuntime::pump_realtime(&mut runtime).is_none());
    let mut capture_progress_seen = false;
    let mut capture_progress_samples = None;
    // One pump queues at most gain acknowledgement and capture progress. The
    // poll may interleave one meter/spectrum sample after Started and after the
    // gain acknowledgement, so these four synchronous polls drain that finite
    // order without adding a delay.
    for _ in 0..4 {
        match runtime.poll() {
            Some(Ok(ImportEvent::ParakeetCaptureProgress {
                job_id: actual,
                generation: actual_generation,
                session_id: 17,
                captured_samples,
                audio_durable_samples,
            })) if actual == job_id
                && actual_generation == generation
                && captured_samples == audio_durable_samples
                && captured_samples >= 32_000 =>
            {
                capture_progress_seen = true;
                capture_progress_samples = Some(captured_samples);
                break;
            }
            Some(Ok(ImportEvent::MicrophoneGainApplied {
                job_id: actual,
                generation: actual_generation,
                target_identity,
                ..
            })) if actual == job_id
                && actual_generation == generation
                && target_identity == "test:parakeet-poll-window-write-error" => {}
            Some(Ok(ImportEvent::MicrophoneMeter(meter)))
                if meter.job_id == job_id && meter.generation == generation => {}
            Some(Ok(ImportEvent::MicrophoneSpectrum(update)))
                if update.owner() == (job_id, generation) => {}
            other => panic!("unexpected poll result before capture progress: {other:?}"),
        }
    }
    assert!(
        capture_progress_seen,
        "the pumped PCM must reach the unchanged 32,000-sample window threshold durably before READY release"
    );
    assert!(
        capture_progress_samples.expect("captured progress was recorded") < INJECTED_SAMPLES as u64,
        "the converter tail must still be pending when the write error stops capture"
    );

    fs::write(root.join("runtime/ready.release"), b"ready")
        .expect("release controlled worker READY");
    let ready_deadline = Instant::now() + Duration::from_secs(5);
    let mut ready = false;
    while Instant::now() < ready_deadline && !ready {
        match runtime.poll() {
            Some(Ok(ImportEvent::ParakeetInferenceReady {
                job_id: actual,
                generation: actual_generation,
                session_id: 17,
                ..
            })) => {
                assert_eq!(actual, job_id, "READY belongs to this live job");
                assert_eq!(actual_generation, generation, "READY generation matches");
                ready = true;
            }
            Some(Err(error)) => panic!("Parakeet worker poll failed before READY: {error:?}"),
            Some(Ok(ImportEvent::MicrophoneMeter(meter)))
                if meter.job_id == job_id && meter.generation == generation =>
            {
                thread::sleep(Duration::from_millis(2));
            }
            Some(Ok(ImportEvent::MicrophoneSpectrum(update)))
                if update.owner() == (job_id, generation) =>
            {
                thread::sleep(Duration::from_millis(2));
            }
            None => thread::sleep(Duration::from_millis(2)),
            other => panic!("unexpected poll result while waiting for READY: {other:?}"),
        }
    }
    assert!(ready, "the controlled Parakeet worker must reach READY");

    let window_root = root.join(format!(
        ".parakeet-live-{:032x}-{}-{}",
        job_id.0,
        generation.get(),
        17
    ));
    fs::create_dir(window_root.join("window-1.wav"))
        .expect("create deterministic first-window I/O obstacle");
    let mut stopped = None;
    // READY marks one telemetry slot. It can be returned before the next poll
    // reaches the scheduled window write; two calls cover that telemetry and
    // the resulting I/O failure without sleeping or extending the READY wait.
    for _ in 0..2 {
        match runtime.poll() {
            Some(Ok(ImportEvent::ParakeetCaptureStopped {
                job_id: actual,
                generation: actual_generation,
                session_id: 17,
                wav_path,
                txt_path,
                srt_path,
                automatic_final: Some(_),
                complete: false,
                ..
            })) if actual == job_id && actual_generation == generation => {
                stopped = Some((wav_path, txt_path, srt_path));
                break;
            }
            Some(Ok(ImportEvent::MicrophoneMeter(meter)))
                if meter.job_id == job_id && meter.generation == generation => {}
            Some(Ok(ImportEvent::MicrophoneSpectrum(update)))
                if update.owner() == (job_id, generation) => {}
            other => panic!(
                "expected only probe telemetry before incomplete stopped event, got {other:?}"
            ),
        }
    }
    let (wav_path, txt_path, srt_path) =
        stopped.expect("poll must report the real window-write failure as stopped");
    assert!(Path::new(&wav_path).is_file(), "final WAV must exist");
    let wav = fs::read(&wav_path).expect("read final WAV bytes independently");
    assert!(wav.len() >= 44, "final WAV contains a complete PCM header");
    assert_eq!(&wav[0..4], b"RIFF", "WAV RIFF marker");
    assert_eq!(
        u32::from_le_bytes(wav[4..8].try_into().expect("RIFF size")) as usize,
        wav.len() - 8,
        "RIFF declared length matches the file"
    );
    assert_eq!(&wav[8..12], b"WAVE", "WAV format marker");
    assert_eq!(&wav[12..16], b"fmt ", "PCM format chunk marker");
    assert_eq!(
        u32::from_le_bytes(wav[16..20].try_into().expect("fmt chunk size")),
        16,
        "PCM fmt chunk size"
    );
    assert_eq!(
        u16::from_le_bytes(wav[20..22].try_into().expect("PCM encoding")),
        1,
        "WAV uses integer PCM"
    );
    assert_eq!(
        u16::from_le_bytes(wav[22..24].try_into().expect("channel count")),
        1,
        "WAV is mono"
    );
    assert_eq!(
        u32::from_le_bytes(wav[24..28].try_into().expect("sample rate")),
        16_000,
        "WAV sample rate"
    );
    assert_eq!(
        u32::from_le_bytes(wav[28..32].try_into().expect("byte rate")),
        32_000,
        "WAV byte rate"
    );
    assert_eq!(
        u16::from_le_bytes(wav[32..34].try_into().expect("block alignment")),
        2,
        "WAV block alignment"
    );
    assert_eq!(
        u16::from_le_bytes(wav[34..36].try_into().expect("bits per sample")),
        16,
        "WAV uses 16-bit samples"
    );
    assert_eq!(&wav[36..40], b"data", "WAV data chunk marker");
    assert_eq!(
        u32::from_le_bytes(wav[40..44].try_into().expect("data chunk size")) as usize,
        INJECTED_SAMPLES * 2,
        "WAV data contains exactly the 48,000 injected mono samples"
    );
    assert_eq!(
        wav.len(),
        44 + INJECTED_SAMPLES * 2,
        "WAV file has no missing or trailing PCM bytes"
    );
    assert!(
        Path::new(txt_path.as_deref().expect("final TXT path")).is_file(),
        "final TXT must exist"
    );
    assert!(
        Path::new(srt_path.as_deref().expect("final SRT path")).is_file(),
        "final SRT must exist"
    );
    assert!(runtime.parakeet_live.is_none());
    assert!(runtime.parakeet_pending_events.is_empty());
}
