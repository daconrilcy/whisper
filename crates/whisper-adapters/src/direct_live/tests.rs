use super::{
    DirectLive, MAX_BUFFERED_SAMPLES, MAX_WINDOW_SAMPLES, MIN_WINDOW_SAMPLES, PathBuf,
    Pcm16Converter, WavOutput, clear_gpu_proof_for_cpu, export_parakeet_manifest,
    export_parakeet_outputs, export_segments, fill_pending_from_wav, finalize_incomplete_outputs,
    gpu_ready_requires_attestation, should_retry_auto_gpu_before_ready, window_size,
    write_wav_header,
};
use crate::microphone_probe::{MicrophoneProbe, MicrophoneSpectrumReader};
use std::collections::VecDeque;
use std::path::Path;
use whisper_core::ports::WorkerSegment;
use whisper_core::{ComputeChoice, JobConfig, LanguageChoice, SegmentId, SourceRange};

fn test_directory(label: &str) -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "direct-live-{label}-{}-{nonce}",
        std::process::id()
    ))
}

#[test]
fn direct_capture_publishes_frequency_spectrum_without_changing_wav_audio() {
    let directory = test_directory("spectrum-direct");
    std::fs::create_dir_all(&directory).unwrap();
    let mut request = startup_request(&directory, ComputeChoice::Cpu);
    request.microphone = whisper_core::MicrophoneChoice::Device("test:mic".into());
    let mut capture = crate::capture::CaptureStream::empty_test_capture(16_000, 1, "test:mic");
    let mut live = super::DirectLive::from_test_capture(request, capture).unwrap();
    let input_blocks = (0..5)
        .map(|block| {
            (block * 960..(block + 1) * 960)
                .map(|frame| 0.5 * (std::f32::consts::TAU * 64.0 * frame as f32 / 2048.0).sin())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    for audio in &input_blocks {
        live.capture
            .as_mut()
            .unwrap()
            .enqueue_test_block(audio.clone());
        live.pump_capture().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    let event = loop {
        if let Some(event) = live.take_probe_spectrum() {
            break event;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "direct spectrum not published"
        );
        std::thread::sleep(std::time::Duration::from_millis(5));
    };
    let whisper_core::ImportEvent::MicrophoneSpectrum(
        whisper_core::microphone_level::MicrophoneSpectrumUpdate::Available(spectrum),
    ) = event
    else {
        panic!("expected a current direct spectrum")
    };
    assert_eq!(spectrum.source_identity, "test:mic");
    let peak = spectrum
        .pre_gain
        .iter()
        .enumerate()
        .filter_map(|(index, bin)| match bin {
            whisper_core::microphone_level::SpectrumBin::MilliDb(value) => Some((index, value)),
            whisper_core::microphone_level::SpectrumBin::Silence => None,
        })
        .max_by_key(|(_, value)| *value)
        .unwrap()
        .0;
    let hz = peak as f64 * f64::from(spectrum.sample_rate_hz) / 2048.0;
    assert!((hz - 500.0).abs() < 10.0, "expected 500 Hz, got {hz} Hz");
    let baseline_dir = directory.join("without-probe");
    std::fs::create_dir_all(&baseline_dir).unwrap();
    let mut baseline_request = startup_request(&baseline_dir, ComputeChoice::Cpu);
    baseline_request.job_id = whisper_core::JobId(7_202);
    let mut baseline_capture =
        crate::capture::CaptureStream::empty_test_capture(16_000, 1, "test:mic");
    let mut baseline_converter = crate::capture::Pcm16Converter::new(16_000, 1).unwrap();
    let mut baseline_wav = WavOutput::create(&baseline_request).unwrap();
    for audio in &input_blocks {
        baseline_capture.enqueue_test_block(audio.clone());
        let block = baseline_capture.try_next_block().unwrap();
        let pcm = baseline_converter.push(&block).unwrap();
        baseline_wav.append(&pcm).unwrap();
        baseline_capture.recycle_block(block).unwrap();
    }
    baseline_wav
        .append(&baseline_converter.finish().unwrap())
        .unwrap();
    let baseline_path = baseline_wav.finish().unwrap();
    live.stop().unwrap();
    let live_samples = live.wav.as_ref().unwrap().sample_count();
    assert_eq!(
        live_samples, 4_800,
        "probe preserves saved PCM sample count"
    );
    let live_path = live.finish_test_wav().unwrap();
    assert_eq!(
        std::fs::read(live_path).unwrap(),
        std::fs::read(baseline_path).unwrap(),
        "Turbo WAV with spectrum reader is byte-identical to the no-reader baseline"
    );
    drop(live);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn direct_capture_poll_keeps_spectrum_text_and_terminal_progress_within_four_events() {
    let directory = test_directory("spectrum-direct-poll");
    std::fs::create_dir_all(&directory).unwrap();
    let mut request = startup_request(&directory, ComputeChoice::Cpu);
    request.microphone = whisper_core::MicrophoneChoice::Device("test:mic".into());
    let capture = crate::capture::CaptureStream::empty_test_capture(16_000, 1, "test:mic");
    let worker = startup_worker(&directory, "cpu-poll-spectrum");
    let mut live =
        DirectLive::from_test_capture_with_cpu_worker(request, capture, worker, 91).unwrap();

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    let mut saw_ready = false;
    while !saw_ready && std::time::Instant::now() < deadline {
        if matches!(
            live.poll().unwrap(),
            Some(whisper_core::ImportEvent::Ready { .. })
        ) {
            saw_ready = true;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(saw_ready, "simulated direct worker did not become ready");

    let mut saw_spectrum = false;
    let mut spectrum_updates = 0;
    let mut saw_confirmed_text = false;
    let mut saw_terminal = false;
    for block_index in 0..100 {
        let first_frame = block_index * 960;
        let audio = (first_frame..first_frame + 960)
            .map(|frame| 0.5 * (std::f32::consts::TAU * 64.0 * frame as f32 / 2048.0).sin())
            .collect::<Vec<_>>();
        live.capture.as_mut().unwrap().enqueue_test_block(audio);
        let mut events_in_refresh = 0;
        for _ in 0..4 {
            events_in_refresh += 1;
            if let Some(event) = live.poll().unwrap() {
                match event {
                    whisper_core::ImportEvent::MicrophoneSpectrum(
                        whisper_core::microphone_level::MicrophoneSpectrumUpdate::Available(_),
                    ) => {
                        saw_spectrum = true;
                        spectrum_updates += 1;
                    }
                    whisper_core::ImportEvent::DirectConfirmed(segment) => {
                        saw_confirmed_text = !segment.text.trim().is_empty();
                    }
                    whisper_core::ImportEvent::DirectPublished { .. } => saw_terminal = true,
                    _ => {}
                }
            }
            if saw_terminal {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(3));
        }
        assert!(events_in_refresh <= 4);
        std::thread::sleep(std::time::Duration::from_millis(8));
    }

    live.stop().unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while !saw_terminal && std::time::Instant::now() < deadline {
        if let Some(event) = live.poll().unwrap() {
            match event {
                whisper_core::ImportEvent::MicrophoneSpectrum(
                    whisper_core::microphone_level::MicrophoneSpectrumUpdate::Available(_),
                ) => {
                    saw_spectrum = true;
                    spectrum_updates += 1;
                }
                whisper_core::ImportEvent::DirectConfirmed(segment) => {
                    saw_confirmed_text = !segment.text.trim().is_empty();
                }
                whisper_core::ImportEvent::DirectPublished { .. } => saw_terminal = true,
                _ => {}
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }

    assert!(
        saw_spectrum,
        "poll() did not publish spectrum during capture"
    );
    assert!(
        spectrum_updates > 1,
        "spectrum did not refresh during the stream"
    );
    assert!(saw_confirmed_text, "poll() starved confirmed worker text");
    assert!(saw_terminal, "poll() starved the worker terminal event");
    assert!(live.done());
    drop(live);
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn parakeet_live_automatic_final_paths_are_distinct_and_explicitly_unverified() {
    let wav = Path::new(r"C:\captures\micro-123.wav");
    let paths = super::automatic_final_output_paths(&wav.to_string_lossy()).unwrap();
    assert_eq!(
        paths.txt_path,
        r"C:\captures\micro-123-auto-non-verifie.txt"
    );
    assert_eq!(
        paths.srt_path,
        r"C:\captures\micro-123-auto-non-verifie.srt"
    );
    assert_eq!(paths.companion_json_path, r"C:\captures\micro-123.json");
}

#[test]
fn cpu_ready_after_auto_fallback_needs_no_gpu_candidate_attestation() {
    let mut candidate = Some(whisper_core::gpu_inventory::GpuRuntimeIdentity {
        generation: 1,
        pnp_instance_id: "PCI\\VEN_10DE&DEV_A".into(),
        dxgi_rank: 0,
        luid: 1,
        pci_id: "0000:01:00.0".into(),
        cuda_ordinal: 0,
        ggml_ordinal: 1,
    });
    let mut attested = false;
    assert!(gpu_ready_requires_attestation(
        candidate.is_some(),
        attested
    ));
    clear_gpu_proof_for_cpu(&mut candidate, &mut attested);
    assert!(candidate.is_none());
    assert!(!attested);
    assert!(!gpu_ready_requires_attestation(
        candidate.is_some(),
        attested
    ));
    assert!(should_retry_auto_gpu_before_ready(
        true,
        false,
        ComputeChoice::Gpu,
        super::WorkerErrorCode::BackendMismatch,
    ));
    assert!(!should_retry_auto_gpu_before_ready(
        false,
        false,
        ComputeChoice::Gpu,
        super::WorkerErrorCode::BackendMismatch,
    ));
    assert!(!should_retry_auto_gpu_before_ready(
        true,
        true,
        ComputeChoice::Gpu,
        super::WorkerErrorCode::BackendMismatch,
    ));
}

fn startup_request(
    destination: &std::path::Path,
    compute: ComputeChoice,
) -> whisper_core::LiveRequest {
    whisper_core::LiveRequest {
        job_id: whisper_core::JobId(800),
        generation: whisper_core::Generation::first(),
        model_path: "unused-model".into(),
        model_sha256: [0; 32],
        destination: destination.to_string_lossy().into_owned(),
        group_offset_samples: 0,
        auto_stop_after_speech_samples: None,
        microphone: Default::default(),
        microphone_gain_db: 0,
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute,
        },
    }
}

fn startup_gpu_candidate() -> whisper_core::gpu_inventory::GpuRuntimeIdentity {
    whisper_core::gpu_inventory::GpuRuntimeIdentity {
        generation: 1,
        pnp_instance_id: "PCI\\VEN_10DE&DEV_TEST".into(),
        dxgi_rank: 0,
        luid: 1,
        pci_id: "0000:01:00.0".into(),
        cuda_ordinal: 0,
        ggml_ordinal: 0,
    }
}

fn startup_worker(directory: &std::path::Path, name: &str) -> std::path::PathBuf {
    let source = directory.join(format!("auto-{name}.rs"));
    let executable = directory.join(format!("auto-{name}.exe"));
    let marker = directory
        .join(format!("auto-{name}.marker"))
        .to_string_lossy()
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    let program = r#"
use std::io::{self, BufRead, Write};
const MARKER: &str = "__MARKER__";
fn field(line: &str, name: &str) -> String {
    let tail = line.split_once(&format!("\"{name}\":" )).unwrap().1.trim_start();
    tail.chars().take_while(|c| c.is_ascii_digit()).collect()
}


fn main() {
    let start = io::stdin().lock().lines().next().unwrap().unwrap();
    let (job, generation, instance) = (field(&start, "job_id"), field(&start, "generation"), field(&start, "instance_id"));
    if "__NAME__" == "cpu-poll-spectrum" {
        println!("{{\"version\":2,\"message\":{{\"Ready\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"backend\":\"Cpu\"}}}}}}");
        io::stdout().flush().unwrap();
        let mut segment_id = 1;
        for command in io::stdin().lock().lines().flatten() {
            if command.contains("\"LiveWindow\"") {
                let sequence = field(&command, "sequence");
                let start_sample = field(&command, "start_sample");
                let end_sample = field(&command, "end_sample");
                let sample_rate_hz = field(&command, "sample_rate_hz");
                println!("{{\"version\":2,\"message\":{{\"Segment\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"segment_id\":{segment_id},\"range\":{{\"start_sample\":{start_sample},\"end_sample\":{end_sample},\"sample_rate_hz\":{sample_rate_hz}}},\"text\":\"simulated confirmed text\"}}}}}}");
                io::stdout().flush().unwrap();
                println!("{{\"version\":2,\"message\":{{\"WindowFinished\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"sequence\":{sequence},\"range\":{{\"start_sample\":{start_sample},\"end_sample\":{end_sample},\"sample_rate_hz\":{sample_rate_hz}}},\"last_segment_sequence\":{segment_id}}}}}}}");
                io::stdout().flush().unwrap();
                segment_id += 1;
            } else if command.contains("\"Stop\"") {
                println!("{{\"version\":2,\"message\":{{\"Stopped\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance}}}}}}}");
                io::stdout().flush().unwrap();
                return;
            }
        }
        return;
    }
    if "__NAME__" == "gpu-stop-fail" {
        std::thread::sleep(std::time::Duration::from_millis(100));
        println!("{{\"version\":2,\"message\":{{\"Failed\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"code\":\"BackendUnavailable\",\"message\":\"injected delayed CUDA initialization failure\"}}}}}}");
        io::stdout().flush().unwrap();
        std::fs::write(MARKER, "failure-sent").unwrap();
        std::thread::sleep(std::time::Duration::from_secs(1));
        return;
    }
    if "__NAME__" == "cpu-marker" {
        std::fs::write(MARKER, "cpu-started").unwrap();
    }
    if "__NAME__" == "cpu-ready" {
        println!("{{\"version\":2,\"message\":{{\"Ready\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"backend\":\"Cpu\"}}}}}}");
        io::stdout().flush().unwrap();
        for command in io::stdin().lock().lines().flatten() {
            if command.contains("\"Stop\"") {
                println!("{{\"version\":2,\"message\":{{\"Stopped\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance}}}}}}}");
                io::stdout().flush().unwrap();
                return;
            }
        }
        return;
    }
    if "__NAME__" == "gpu" {
        println!("{{\"version\":2,\"message\":{{\"Failed\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"code\":\"BackendUnavailable\",\"message\":\"injected CUDA initialization failure\"}}}}}}");
        io::stdout().flush().unwrap();
    }
}
"#.replace("__NAME__", name).replace("__MARKER__", &marker);
    std::fs::write(&source, program).unwrap();
    let output = std::process::Command::new("rustc")
        .args([
            "--edition=2021",
            "--crate-name",
            "whisper_direct_startup_test",
        ])
        .arg(&source)
        .arg("-o")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    executable
}

#[test]
fn auto_microphone_falls_back_while_capture_is_open_and_keeps_auto_preference() {
    let directory = test_directory("auto-startup");
    std::fs::create_dir_all(&directory).unwrap();
    let cpu = startup_worker(&directory, "cpu");
    let gpu = startup_worker(&directory, "gpu");
    let workers = crate::supervisor::WorkerPaths {
        cpu: cpu.clone(),
        gpu,
    };
    let mut live = super::DirectLive::start(
        startup_request(&directory, ComputeChoice::Auto),
        &workers,
        1,
        None,
        Some(startup_gpu_candidate()),
    )
    .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while live.request.config.compute == ComputeChoice::Gpu && std::time::Instant::now() < deadline
    {
        let _ = live.poll().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(live.request.config.compute, ComputeChoice::Cpu);
    assert!(live.auto_preference);
    assert!(
        live.fallback_reason
            .as_ref()
            .is_some_and(|reason| reason.contains("CUDA"))
    );
    assert!(!live.worker_ready && live.capture.is_some());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    let mut cpu_failure = None;
    while std::time::Instant::now() < deadline {
        match live.poll() {
            Err(error) => {
                cpu_failure = Some(error);
                break;
            }
            Ok(_) => std::thread::sleep(std::time::Duration::from_millis(10)),
        }
    }
    assert!(
        cpu_failure
            .as_deref()
            .is_some_and(|error| error.contains("CPU"))
    );
    assert_eq!(live.request.config.compute, ComputeChoice::Cpu);
    live.worker.as_mut().unwrap().abort();
    drop(live);
    std::fs::remove_file(directory.join(format!("micro-{:032x}.wav", 800))).unwrap();
    let absent = crate::supervisor::WorkerPaths {
        cpu,
        gpu: directory.join("missing-gpu.exe"),
    };
    let mut without_gpu = super::DirectLive::start(
        startup_request(&directory, ComputeChoice::Auto),
        &absent,
        3,
        None,
        None,
    )
    .unwrap();
    assert_eq!(without_gpu.request.config.compute, ComputeChoice::Cpu);
    assert_eq!(
        without_gpu.fallback_reason.as_deref(),
        Some("worker CUDA absent")
    );
    assert!(!without_gpu.worker_ready && without_gpu.capture.is_some());
    without_gpu.worker.as_mut().unwrap().abort();
    drop(without_gpu);
    std::fs::remove_file(directory.join(format!("micro-{:032x}.wav", 800))).unwrap();
    let mut forced = super::DirectLive::start(
        startup_request(&directory, ComputeChoice::Gpu),
        &workers,
        4,
        None,
        Some(startup_gpu_candidate()),
    )
    .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    let mut failed = false;
    while std::time::Instant::now() < deadline {
        if forced.poll().is_err() {
            failed = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(failed && !forced.auto_preference);
    assert_eq!(forced.request.config.compute, ComputeChoice::Gpu);
    assert!(forced.fallback_reason.is_none());
    forced.worker.as_mut().unwrap().abort();
    drop(forced);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn auto_without_gpu_worker_clears_candidate_and_accepts_cpu_ready() {
    let directory = test_directory("no-gpu-worker");
    std::fs::create_dir_all(&directory).unwrap();
    let cpu = startup_worker(&directory, "cpu-ready");
    let workers = crate::supervisor::WorkerPaths {
        cpu,
        gpu: directory.join("missing-gpu-worker.exe"),
    };
    let mut live = super::DirectLive::start(
        startup_request(&directory, ComputeChoice::Auto),
        &workers,
        21,
        None,
        Some(startup_gpu_candidate()),
    )
    .unwrap();
    assert_eq!(live.request.config.compute, ComputeChoice::Cpu);
    assert!(live.auto_preference);
    assert_eq!(live.fallback_reason.as_deref(), Some("worker CUDA absent"));
    assert!(live.gpu_candidate.is_none());
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while !live.worker_ready && std::time::Instant::now() < deadline {
        let _ = live.poll().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(
        live.worker_ready,
        "CPU Ready must be accepted after Auto fallback"
    );
    assert_eq!(live.request.config.compute, ComputeChoice::Cpu);
    assert_eq!(live.fallback_reason.as_deref(), Some("worker CUDA absent"));
    live.stop().unwrap();
    drop(live);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn auto_microphone_stop_before_gpu_ready_keeps_cpu_retry_for_recorded_audio() {
    let directory = test_directory("stop-before-ready");
    std::fs::create_dir_all(&directory).unwrap();
    let cpu = startup_worker(&directory, "cpu-marker");
    let gpu = startup_worker(&directory, "gpu-stop-fail");
    let failure_marker = directory.join("auto-gpu-stop-fail.marker");
    let cpu_marker = directory.join("auto-cpu-marker.marker");
    let workers = crate::supervisor::WorkerPaths { cpu, gpu };
    let mut live = super::DirectLive::start(
        startup_request(&directory, ComputeChoice::Auto),
        &workers,
        20,
        None,
        Some(startup_gpu_candidate()),
    )
    .unwrap();
    live.stop().unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while !failure_marker.is_file() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(
        failure_marker.is_file(),
        "GPU failure was emitted after Stop"
    );
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while !cpu_marker.exists() && std::time::Instant::now() < deadline {
        let _ = live.poll();
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(
        cpu_marker.exists(),
        "Stop keeps a CPU attempt before inference"
    );
    assert_eq!(live.request.config.compute, ComputeChoice::Cpu);
    assert!(live.auto_preference);
    drop(live);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn wav_header_describes_mono_16khz_pcm_and_exact_audio_length() {
    let mut bytes = Vec::new();
    write_wav_header(&mut bytes, 32_000).unwrap();
    assert_eq!(bytes.len(), 44);
    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 64_036);
    assert_eq!(&bytes[8..16], b"WAVEfmt ");
    assert_eq!(u16::from_le_bytes(bytes[20..22].try_into().unwrap()), 1);
    assert_eq!(u16::from_le_bytes(bytes[22..24].try_into().unwrap()), 1);
    assert_eq!(
        u32::from_le_bytes(bytes[24..28].try_into().unwrap()),
        16_000
    );
    assert_eq!(u16::from_le_bytes(bytes[34..36].try_into().unwrap()), 16);
    assert_eq!(&bytes[36..40], b"data");
    assert_eq!(
        u32::from_le_bytes(bytes[40..44].try_into().unwrap()),
        64_000
    );
}

#[test]
fn wav_output_keeps_the_exact_converted_samples() {
    let directory = test_directory("wav-output");
    let job_id = whisper_core::JobId(
        ((std::process::id() as u128) << 64)
            | std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
    );
    let request = whisper_core::LiveRequest {
        job_id,
        generation: whisper_core::Generation::first(),
        model_path: "unused".into(),
        model_sha256: [0; 32],
        destination: directory.to_string_lossy().into_owned(),
        group_offset_samples: 0,
        auto_stop_after_speech_samples: None,
        microphone: Default::default(),
        microphone_gain_db: 0,
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        },
    };
    let mut wav = WavOutput::create(&request).unwrap();
    wav.append(&[-32768, 0]).unwrap();
    assert_eq!(wav.read_samples(0, 2).unwrap(), [-32768, 0]);
    wav.append(&[32767]).unwrap();
    assert_eq!(wav.read_samples(1, 2).unwrap(), [0, 32767]);
    let path = wav.finish().unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(bytes.len(), 50);
    assert_eq!(u32::from_le_bytes(bytes[40..44].try_into().unwrap()), 6);
    assert_eq!(&bytes[44..], &[0, 128, 0, 0, 255, 127]);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn stop_before_engine_ready_flushes_captured_audio_to_the_wav() {
    let directory = test_directory("stop-before-engine-ready");
    let mut request = startup_request(&directory, ComputeChoice::Cpu);
    request.job_id = whisper_core::JobId(
        ((std::process::id() as u128) << 64)
            | std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
    );
    let capture =
        crate::capture::CaptureStream::from_test_blocks(16_000, 1, vec![vec![0.25; 16_000]]);
    let converter = Pcm16Converter::new(16_000, 1).unwrap();
    let wav = WavOutput::create(&request).unwrap();
    let job_id = request.job_id;
    let probe = MicrophoneProbe::start(
        capture.sample_rate_hz(),
        capture.source_identity().map(str::to_owned),
        request.job_id,
        request.generation,
    )
    .unwrap();
    let mut live = DirectLive {
        request,
        auto_preference: false,
        fallback_reason: None,
        cpu_worker_path: PathBuf::new(),
        worker_path: PathBuf::new(),
        worker: None,
        instance: 1,
        worker_ready: false,
        gpu_attested: false,
        gpu_candidate: None,
        legacy_fallback: false,
        capture: Some(capture),
        probe: Some(probe),
        last_meter_sequence: 0,
        spectrum_reader: MicrophoneSpectrumReader::default(),
        next_spectrum_telemetry: false,
        telemetry_due_after_worker_event: false,
        last_acknowledged_gain_revision: None,
        converter: Some(converter),
        wav: Some(wav),
        diagnostics: None,
        pending: VecDeque::new(),
        next_offset: 0,
        next_window: 1,
        inflight: None,
        next_segment: 1,
        segments: Vec::new(),
        window_segments: Vec::new(),
        ready_events: VecDeque::new(),
        capture_announced: false,
        first_admitted_at_unix_ms: None,
        stop_requested: false,
        stop_sent: false,
        done: false,
    };

    live.stop().unwrap();

    assert!(!live.worker_ready);
    assert!(live.stop_requested);
    assert_eq!(live.wav.as_ref().unwrap().samples, 16_000);
    let samples = live.wav.as_mut().unwrap().read_samples(0, 16_000).unwrap();
    assert_eq!(samples.len(), 16_000);
    assert!(samples[320..15_680].iter().all(|sample| *sample == 8_192));
    drop(live);
    let path = directory.join(format!("micro-{:032x}.wav", job_id.0));
    std::fs::remove_file(path).unwrap();
}

#[test]
fn gain_command_changes_only_subsequently_acquired_audio_and_stop_finalizes_wav() {
    let directory = test_directory("gain-next-acquisition");
    let mut request = startup_request(&directory, ComputeChoice::Cpu);
    request.microphone = whisper_core::MicrophoneChoice::Device("test:mic".into());
    request.job_id = whisper_core::JobId(
        ((std::process::id() as u128) << 64)
            | std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
    );
    let mut capture = crate::capture::CaptureStream::empty_test_capture(16_000, 1, "test:mic");
    capture.enqueue_test_block(vec![0.25; 16_000]);
    let converter = Pcm16Converter::new(16_000, 1).unwrap();
    let wav = WavOutput::create(&request).unwrap();
    let job_id = request.job_id;
    let mut live = DirectLive {
        request,
        auto_preference: false,
        fallback_reason: None,
        cpu_worker_path: PathBuf::new(),
        worker_path: PathBuf::new(),
        worker: None,
        instance: 1,
        worker_ready: false,
        gpu_attested: false,
        gpu_candidate: None,
        legacy_fallback: false,
        capture: Some(capture),
        probe: None,
        last_meter_sequence: 0,
        spectrum_reader: MicrophoneSpectrumReader::default(),
        next_spectrum_telemetry: false,
        telemetry_due_after_worker_event: false,
        last_acknowledged_gain_revision: None,
        converter: Some(converter),
        wav: Some(wav),
        diagnostics: None,
        pending: VecDeque::new(),
        next_offset: 0,
        next_window: 1,
        inflight: None,
        next_segment: 1,
        segments: Vec::new(),
        window_segments: Vec::new(),
        ready_events: VecDeque::new(),
        capture_announced: false,
        first_admitted_at_unix_ms: None,
        stop_requested: false,
        stop_sent: false,
        done: false,
    };

    live.pump_capture().unwrap();
    let applied = live
        .set_gain("test:mic", 0, whisper_core::GainDb::new(6).unwrap())
        .unwrap();
    assert_eq!(applied.revision, 1);
    live.capture
        .as_mut()
        .unwrap()
        .enqueue_test_block(vec![0.25; 16_000]);
    live.stop().unwrap();

    let wav = live.wav.as_mut().unwrap();
    assert_eq!(wav.samples, 32_000);
    let samples = wav.read_samples(0, 32_000).unwrap();
    assert_eq!(samples.len(), 32_000);
    assert!(samples[320..15_680].iter().all(|sample| *sample == 8_192));
    assert!(
        samples[16_320..31_680]
            .iter()
            .all(|sample| (16_300..=16_400).contains(sample))
    );
    drop(live);
    std::fs::remove_file(directory.join(format!("micro-{:032x}.wav", job_id.0))).unwrap();
}

#[test]
fn test_capture_gain_preserves_probe_a_and_applies_once_to_wav_pcm() {
    let directory = test_directory("gain-single-application");
    let mut request = startup_request(&directory, ComputeChoice::Cpu);
    request.microphone = whisper_core::MicrophoneChoice::Device("test:gain-once".into());
    request.microphone_gain_db = 6;
    let gain = whisper_core::GainDb::new(6).unwrap();
    let source = (0..16_000)
        .map(|index| if index < 8_000 { 0.25_f32 } else { 0.75_f32 })
        .collect::<Vec<_>>();

    let mut capture = crate::capture::CaptureStream::empty_test_capture_with_gain(
        16_000,
        1,
        "test:gain-once",
        gain,
    );
    capture.enqueue_test_block(source.clone());
    let block = capture.try_next_block().unwrap();
    assert_eq!(block.gain().revision, 0);
    assert_eq!(block.gain().gain, gain);
    assert!(
        block
            .samples()
            .iter()
            .zip(&source)
            .all(|(actual, expected)| (actual - expected).abs() <= f32::EPSILON)
    );

    let mut probe_converter = Pcm16Converter::new(16_000, 1).unwrap();
    let probe = probe_converter.push_with_probe(&block).unwrap().probe;
    assert!(
        probe
            .mono_pre_gain
            .iter()
            .zip(&source)
            .all(|(actual, expected)| (actual - expected).abs() <= f32::EPSILON)
    );
    let factor = 10.0_f32.powf(6.0 / 20.0);
    for (index, actual) in probe.mono_post_gain_limiter.iter().enumerate() {
        let expected = (source[index] * factor).clamp(-1.0, 1.0);
        assert!((actual - expected).abs() <= 1.0e-6);
    }
    capture.recycle_block(block).unwrap();

    let mut live = DirectLive::from_test_capture(request, capture).unwrap();
    live.enqueue_test_capture_block(source);
    live.stop().unwrap();
    let wav = live.wav.as_mut().unwrap();
    assert_eq!(wav.samples, 16_000);
    let pcm = wav.read_samples(0, 16_000).unwrap();
    let expected_unclamped = ((0.25_f32 * factor).clamp(-1.0, 1.0) * 32_767.0).round() as i16;
    assert!(
        pcm[500..7_500]
            .iter()
            .all(|sample| *sample == expected_unclamped)
    );
    assert!(pcm[8_500..15_500].iter().all(|sample| *sample == i16::MAX));
}

#[test]
fn wav_backlog_fills_bounded_windows_without_drops_or_reordering() {
    let directory = test_directory("wav-backlog");
    let job_id = whisper_core::JobId(
        ((std::process::id() as u128) << 64)
            | std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
    );
    let request = startup_request(&directory, ComputeChoice::Cpu);
    let mut request = request;
    request.job_id = job_id;
    let mut wav = WavOutput::create(&request).unwrap();
    let samples: Vec<i16> = (0..(MAX_BUFFERED_SAMPLES * 2 + 731))
        .map(|sample| (sample % 30_000) as i16)
        .collect();
    wav.append(&samples).unwrap();

    let mut pending = VecDeque::new();
    let mut next_offset = 0_u64;
    let mut observed = Vec::with_capacity(samples.len());
    while observed.len() < samples.len() {
        fill_pending_from_wav(&mut wav, next_offset, &mut pending).unwrap();
        assert!(!pending.is_empty());
        let count = pending.len().min(MAX_WINDOW_SAMPLES);
        observed.extend(pending.drain(..count));
        next_offset += count as u64;
    }
    assert_eq!(observed, samples);
    let path = wav.finish().unwrap();
    std::fs::remove_file(path).unwrap();
}

#[test]
fn simulated_gpu_failure_keeps_wav_and_confirmed_text_with_incomplete_suffix() {
    let directory = test_directory("simulated-gpu-failure");
    let job_id = whisper_core::JobId(
        ((std::process::id() as u128) << 64)
            | std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
    );
    let generation = whisper_core::Generation::first();
    let request = whisper_core::LiveRequest {
        job_id,
        generation,
        model_path: "unused".into(),
        model_sha256: [0; 32],
        destination: directory.to_string_lossy().into_owned(),
        group_offset_samples: 0,
        auto_stop_after_speech_samples: None,
        microphone: Default::default(),
        microphone_gain_db: 0,
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Gpu,
        },
    };
    let mut wav = WavOutput::create(&request).unwrap();
    wav.append(&[100, 200, 300, 400]).unwrap();
    let confirmed = WorkerSegment {
        job_id,
        generation,
        instance_id: 1,
        segment_id: SegmentId(1),
        range: SourceRange::new(0, 2, 16_000).unwrap(),
        text: "parole confirmée".into(),
    };
    let message = finalize_incomplete_outputs(
        &request,
        wav,
        &[confirmed],
        "GPU interrompu après un bloc".into(),
    );
    let stem = format!("micro-{:032x}", job_id.0);
    let wav_path = directory.join(format!("{stem}.wav"));
    let txt_path = directory.join(format!("{stem}-incomplet.txt"));
    let srt_path = directory.join(format!("{stem}-incomplet.srt"));
    assert!(message.contains("Réimportez le WAV en CPU"));
    assert!(message.contains(&wav_path.to_string_lossy().to_string()));
    assert_eq!(std::fs::read(&wav_path).unwrap().len(), 52);
    assert_eq!(
        std::fs::read_to_string(&txt_path).unwrap(),
        "parole confirmée"
    );
    assert!(
        std::fs::read_to_string(&srt_path)
            .unwrap()
            .contains("parole confirmée")
    );
    assert!(!directory.join(format!("{stem}.txt")).exists());
    for path in [wav_path, txt_path, srt_path] {
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn microphone_interruption_finalizes_cpu_outputs_without_gpu_message() {
    let directory = test_directory("microphone-interruption");
    let mut request = startup_request(&directory, ComputeChoice::Cpu);
    request.job_id = whisper_core::JobId(
        ((std::process::id() as u128) << 64)
            | std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
    );
    let mut wav = WavOutput::create(&request).unwrap();
    wav.append(&[100, 200]).unwrap();
    let message = finalize_incomplete_outputs(
        &request,
        wav,
        &[],
        "Micro interrompu par Windows : périphérique déconnecté".into(),
    );
    assert!(message.contains("Session micro arrêtée"));
    assert!(!message.contains("Session GPU"));
    let stem = format!("micro-{:032x}", request.job_id.0);
    for path in [
        directory.join(format!("{stem}.wav")),
        directory.join(format!("{stem}-incomplet.txt")),
        directory.join(format!("{stem}-incomplet.srt")),
    ] {
        assert!(path.is_file());
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn live_window_waits_for_five_seconds_then_prefers_a_pause_before_eight() {
    let mut pending = VecDeque::from(vec![1_000_i16; 96_000]);
    assert_eq!(window_size(&pending, false), None);
    for sample in pending.range_mut(80_000..90_000) {
        *sample = 0;
    }
    let cut = window_size(&pending, false).unwrap();
    assert!(cut > MIN_WINDOW_SAMPLES && cut < 90_000);
    assert_eq!(
        window_size(&VecDeque::from(vec![1_000; MAX_WINDOW_SAMPLES]), false),
        Some(MAX_WINDOW_SAMPLES)
    );
    assert_eq!(
        window_size(&VecDeque::from(vec![1_000; 32_000]), true),
        Some(32_000)
    );
}

#[test]
fn parakeet_exports_sort_time_and_write_versioned_companion_intervals() {
    let directory = test_directory("parakeet-companion");
    std::fs::create_dir_all(&directory).unwrap();
    let request = startup_request(&directory, ComputeChoice::Cpu);
    let wav_path = directory.join(format!("micro-{:032x}.wav", request.job_id.0));
    std::fs::write(&wav_path, b"wav").unwrap();
    let later = WorkerSegment {
        job_id: request.job_id,
        generation: request.generation,
        instance_id: 7,
        segment_id: SegmentId(1),
        range: SourceRange::new(8_000, 12_000, 16_000).unwrap(),
        text: "fonctionne".into(),
    };
    let earlier = WorkerSegment {
        segment_id: SegmentId(2),
        range: SourceRange::new(2_000, 6_000, 16_000).unwrap(),
        text: "test".into(),
        ..later.clone()
    };
    let segments = [later, earlier];
    let unresolved = [SourceRange::new(0, 1_000, 16_000).unwrap()];
    let (_, _) = export_segments(&request, &segments, true).unwrap();
    let manifest = export_parakeet_manifest(
        &request,
        &wav_path.to_string_lossy(),
        16_000,
        &segments,
        &unresolved,
        false,
    )
    .unwrap();
    let text_path = directory.join(format!("micro-{:032x}-incomplet.txt", request.job_id.0));
    assert_eq!(
        std::fs::read_to_string(text_path).unwrap(),
        "test\nfonctionne"
    );
    let document: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
    assert_eq!(document["version"], 1);
    assert_eq!(document["complete"], false);
    assert_eq!(document["audio_samples"], 16_000);
    assert_eq!(document["confirmed_intervals"][0]["segment_id"], 2);
    assert_eq!(document["confirmed_intervals"][0]["text"], "test");
    assert_eq!(document["unresolved_intervals"][0]["start_sample"], 0);
    for path in [
        wav_path,
        directory.join(format!("micro-{:032x}-incomplet.txt", request.job_id.0)),
        directory.join(format!("micro-{:032x}-incomplet.srt", request.job_id.0)),
        directory.join(format!("micro-{:032x}.json", request.job_id.0)),
    ] {
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn parakeet_srt_failure_removes_created_txt_and_retries_coherent_incomplete_outputs() {
    let directory = test_directory("parakeet-export-failure");
    std::fs::create_dir_all(&directory).unwrap();
    let request = startup_request(&directory, ComputeChoice::Cpu);
    let stem = format!("micro-{:032x}", request.job_id.0);
    let wav_path = directory.join(format!("{stem}.wav"));
    std::fs::write(&wav_path, b"wav").unwrap();
    let blocked_complete_srt = directory.join(format!("{stem}.srt"));
    std::fs::create_dir(&blocked_complete_srt).unwrap();
    let segment = WorkerSegment {
        job_id: request.job_id,
        generation: request.generation,
        instance_id: 7,
        segment_id: SegmentId(1),
        range: SourceRange::new(1_000, 2_000, 16_000).unwrap(),
        text: "confirmé".into(),
    };

    let outcome = export_parakeet_outputs(
        &request,
        &wav_path.to_string_lossy(),
        16_000,
        &[segment],
        &[],
        true,
    );

    assert!(!outcome.complete);
    assert_eq!(outcome.diagnostics.len(), 1);
    assert!(outcome.diagnostics[0].contains("SRT non enregistré"));
    let txt_path = outcome.txt_path.unwrap();
    let srt_path = outcome.srt_path.unwrap();
    assert!(txt_path.ends_with("-incomplet.txt"));
    assert!(srt_path.ends_with("-incomplet.srt"));
    assert!(std::path::Path::new(&txt_path).is_file());
    assert!(std::path::Path::new(&srt_path).is_file());
    assert!(!directory.join(format!("{stem}.txt")).exists());
    assert!(blocked_complete_srt.is_dir());
    let manifest_path = directory.join(format!("{stem}.json"));
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
    assert_eq!(manifest["complete"], false);

    std::fs::remove_dir(blocked_complete_srt).unwrap();
    for path in [
        wav_path,
        PathBuf::from(txt_path),
        PathBuf::from(srt_path),
        manifest_path,
    ] {
        std::fs::remove_file(path).unwrap();
    }
}

#[test]
fn worker_rejects_stale_ready_and_confirms_segments_only_after_matching_window_end() {
    use whisper_core::ipc::{BackendKind, WorkerEvent, WorkerSegmentDto};

    let directory = test_directory("worker-confirmation");
    let request = startup_request(&directory, ComputeChoice::Cpu);
    let job_id = request.job_id;
    let generation = request.generation;
    let capture = crate::capture::CaptureStream::empty_test_capture(16_000, 1, "test:mic");
    let mut live = DirectLive::from_test_capture(request, capture).unwrap();

    let error = live
        .handle_worker(WorkerEvent::Ready {
            job_id,
            generation,
            instance_id: 2,
            backend: BackendKind::Cpu,
        })
        .unwrap_err();
    assert!(error.contains("réponse inattendue ou périmée"));
    assert!(!live.worker_ready);

    let range = SourceRange::new(0, 1_000, 16_000).unwrap();
    live.inflight = Some((1, range));
    live.next_offset = 1_000;
    let result = live
        .handle_worker(WorkerEvent::Segment(WorkerSegmentDto {
            job_id,
            generation,
            instance_id: 1,
            segment_id: SegmentId(1),
            range,
            text: "confirmé".into(),
        }))
        .unwrap();
    assert!(result.is_none());
    assert!(live.segments.is_empty());
    assert_eq!(live.window_segments.len(), 1);

    let error = live
        .handle_worker(WorkerEvent::WindowFinished {
            job_id,
            generation,
            instance_id: 1,
            sequence: 1,
            range,
            last_segment_sequence: 0,
        })
        .unwrap_err();
    assert!(error.contains("fin de fenêtre incohérente"));
    assert!(live.segments.is_empty());

    let result = live
        .handle_worker(WorkerEvent::WindowFinished {
            job_id,
            generation,
            instance_id: 1,
            sequence: 1,
            range,
            last_segment_sequence: 1,
        })
        .unwrap();
    assert!(
        matches!(result, Some(whisper_core::ImportEvent::DirectConfirmed(segment)) if segment.text == "confirmé")
    );
    assert_eq!(live.segments.len(), 1);
    drop(live);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn gpu_ready_requires_matching_attestation_and_stale_failed_keeps_existing_error() {
    use whisper_core::ipc::{BackendKind, WorkerErrorCode, WorkerEvent};

    let directory = test_directory("worker-attestation");
    let request = startup_request(&directory, ComputeChoice::Gpu);
    let job_id = request.job_id;
    let generation = request.generation;
    let capture = crate::capture::CaptureStream::empty_test_capture(16_000, 1, "test:mic");
    let mut live = DirectLive::from_test_capture(request, capture).unwrap();
    let candidate = startup_gpu_candidate();
    live.gpu_candidate = Some(candidate.clone());

    let ready = || WorkerEvent::Ready {
        job_id,
        generation,
        instance_id: 1,
        backend: BackendKind::Cuda,
    };
    assert!(
        live.handle_worker(ready())
            .unwrap_err()
            .contains("attestation")
    );
    let attestation = |pnp: String| WorkerEvent::GpuSessionAttested {
        job_id,
        generation,
        instance_id: 1,
        candidate: pnp,
        dxgi_rank: candidate.dxgi_rank,
        luid: candidate.luid,
        pci_id: candidate.pci_id.clone(),
        cuda_ordinal: candidate.cuda_ordinal,
        ggml_ordinal: candidate.ggml_ordinal,
    };
    assert!(
        live.handle_worker(attestation("different-candidate".into()))
            .unwrap_err()
            .contains("different GPU candidate")
    );
    assert!(
        live.handle_worker(attestation(candidate.pnp_instance_id.clone()))
            .unwrap()
            .is_none()
    );
    assert!(matches!(
        live.handle_worker(ready()).unwrap(),
        Some(whisper_core::ImportEvent::Ready { .. })
    ));
    let error = live
        .handle_worker(WorkerEvent::Failed {
            job_id: Some(whisper_core::JobId(job_id.0 + 1)),
            generation: Some(generation),
            instance_id: Some(2),
            code: WorkerErrorCode::ModelUnavailable,
            message: "injected".into(),
        })
        .unwrap_err();
    assert!(error.contains("Modèle local indisponible"));
    drop(live);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn parakeet_json_write_failure_preserves_text_outputs_and_reports_incomplete() {
    let directory = test_directory("parakeet-json-failure");
    std::fs::create_dir_all(&directory).unwrap();
    let request = startup_request(&directory, ComputeChoice::Cpu);
    let wav_path = directory.join(format!("micro-{:032x}.wav", request.job_id.0));
    std::fs::write(&wav_path, b"wav").unwrap();
    std::fs::create_dir(wav_path.with_extension("json")).unwrap();

    let outcome = export_parakeet_outputs(
        &request,
        &wav_path.to_string_lossy(),
        16_000,
        &[],
        &[],
        true,
    );
    assert!(!outcome.complete);
    assert!(
        outcome
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("JSON de capture non enregistré"))
    );
    assert!(Path::new(outcome.txt_path.as_ref().unwrap()).is_file());
    assert!(Path::new(outcome.srt_path.as_ref().unwrap()).is_file());
    std::fs::remove_dir_all(directory).unwrap();
}
