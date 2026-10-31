use super::*;
use crate::microphone_probe::MicrophoneSpectrumReader;
use std::collections::VecDeque;
use std::{
    sync::{atomic::AtomicBool, mpsc},
    time::Duration,
};
use whisper_core::microphone_level::{MicrophoneSpectrumUpdate, SpectrumBin};

#[test]
fn parakeet_capture_pump_publishes_frequency_spectrum_and_preserves_wav() {
    let directory = std::env::temp_dir().join(format!(
        "parakeet-spectrum-{}-{}",
        std::process::id(),
        unix_time_ms()
    ));
    fs::create_dir_all(&directory).unwrap();
    let job_id = JobId(7201);
    let generation = Generation::first();
    let request = LiveRequest {
        job_id,
        generation,
        model_path: "unused-model".into(),
        model_sha256: [0; 32],
        destination: directory.to_string_lossy().into_owned(),
        group_offset_samples: 0,
        auto_stop_after_speech_samples: None,
        microphone: whisper_core::MicrophoneChoice::Device("test:mic".into()),
        microphone_gain_db: 0,
        config: whisper_core::JobConfig {
            language: whisper_core::LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        },
    };
    let mut capture = crate::capture::CaptureStream::empty_test_capture(16_000, 1, "test:mic");
    let probe =
        MicrophoneProbe::start(16_000, Some("test:mic".into()), job_id, generation).unwrap();
    let converter = Pcm16Converter::new(16_000, 1).unwrap();
    let wav = WavOutput::create(&request).unwrap();
    let (inference_request, _request_rx) = mpsc::sync_channel(1);
    let (_event_tx, inference_events) = mpsc::sync_channel(2);
    let window_root = directory.join("windows");
    fs::create_dir(&window_root).unwrap();
    let mut live = ParakeetLive {
        request,
        gpu_candidate: None,
        session_id: 1,
        inference_request,
        inference_events,
        cancel_inference: Arc::new(AtomicBool::new(false)),
        capture: Some(capture),
        probe,
        last_meter_sequence: 0,
        spectrum_reader: MicrophoneSpectrumReader::default(),
        next_spectrum_telemetry: false,
        telemetry_due_after_event: false,
        last_acknowledged_gain_revision: None,
        converter: Some(converter),
        wav: Some(wav),
        window_root,
        next_window_start: 0,
        next_sequence: 1,
        last_reported_samples: 0,
        capture_origin_unix_ms: None,
        inflight: false,
        stopped: false,
        unavailable: false,
        ready_sent: false,
        finalizing: false,
        pending_events: VecDeque::new(),
        confirmation: Confirmation::default(),
        recovery_attempts: RecoveryAttemptLedger::default(),
        trace: TraceSink::disabled(),
        d2_worker_diagnostics: D2WorkerDiagnostics::default(),
    };
    let input_blocks = (0..5)
        .map(|block| {
            (block * 960..(block + 1) * 960)
                .map(|frame| 0.5 * (std::f32::consts::TAU * 64.0 * frame as f32 / 2048.0).sin())
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    for samples in &input_blocks {
        live.capture
            .as_mut()
            .unwrap()
            .enqueue_test_block(samples.clone());
        live.pump_capture().unwrap();
        thread::sleep(Duration::from_millis(20));
    }
    let deadline = Instant::now() + Duration::from_secs(2);
    let ready = ImportEvent::ParakeetInferenceReady {
        job_id,
        generation,
        session_id: 1,
        backend: "CPU".into(),
    };
    live.pending_events.extend([
        ready.clone(),
        ImportEvent::ParakeetConfirmed {
            job_id,
            generation,
            session_id: 1,
            segment: whisper_core::ports::WorkerSegment {
                job_id,
                generation,
                instance_id: 1,
                segment_id: whisper_core::SegmentId(1),
                range: whisper_core::SourceRange::new(0, 160, 16_000).unwrap(),
                text: "continuité".into(),
            },
        },
        ready.clone(),
        ImportEvent::ParakeetCaptureStopped {
            job_id,
            generation,
            session_id: 1,
            wav_path: "test.wav".into(),
            txt_path: None,
            srt_path: None,
            complete: false,
            final_segments: Vec::new(),
            unresolved: Vec::new(),
            diagnostic: None,
            automatic_final: None,
        },
    ]);
    let mut ready_count = 0;
    let mut saw_confirmed = false;
    let mut saw_stopped = false;
    let mut saw_meter = false;
    let mut event = None;
    while Instant::now() < deadline
        && (ready_count < 2 || !saw_confirmed || !saw_stopped || !saw_meter || event.is_none())
    {
        if let Some(next) = live.poll().unwrap() {
            match next {
                ImportEvent::ParakeetInferenceReady { .. } => ready_count += 1,
                ImportEvent::ParakeetConfirmed { .. } => saw_confirmed = true,
                ImportEvent::ParakeetCaptureStopped { .. } => saw_stopped = true,
                ImportEvent::MicrophoneMeter(_) => saw_meter = true,
                spectrum @ ImportEvent::MicrophoneSpectrum(MicrophoneSpectrumUpdate::Available(
                    _,
                )) => event = Some(spectrum),
                _ => {}
            }
        } else {
            thread::sleep(Duration::from_millis(5));
        }
    }
    assert_eq!(ready_count, 2, "Parakeet control events did not progress");
    assert!(saw_confirmed, "confirmed Parakeet text did not progress");
    assert!(saw_stopped, "Parakeet terminal event did not progress");
    assert!(saw_meter, "Parakeet RMS telemetry did not progress");
    let event = event.expect("Parakeet spectrum did not progress");
    let ImportEvent::MicrophoneSpectrum(MicrophoneSpectrumUpdate::Available(spectrum)) = event
    else {
        panic!("expected a current Parakeet spectrum")
    };
    assert_eq!(spectrum.source_identity, "test:mic");
    let peak = spectrum
        .pre_gain
        .iter()
        .enumerate()
        .filter_map(|(index, bin)| match bin {
            SpectrumBin::MilliDb(value) => Some((index, value)),
            SpectrumBin::Silence => None,
        })
        .max_by_key(|(_, value)| *value)
        .unwrap()
        .0;
    let hz = peak as f64 * f64::from(spectrum.sample_rate_hz) / 2048.0;
    assert!((hz - 500.0).abs() < 10.0, "expected 500 Hz, got {hz} Hz");
    let tail = live.converter.as_mut().unwrap().finish().unwrap();
    live.wav.as_mut().unwrap().append(&tail).unwrap();
    assert_eq!(live.wav.as_ref().unwrap().sample_count(), 4_800);
    let with_probe_path = live.wav.take().unwrap().finish().unwrap();
    let baseline_dir = directory.join("without-probe");
    fs::create_dir_all(&baseline_dir).unwrap();
    let baseline_request = LiveRequest {
        job_id: JobId(7202),
        generation,
        model_path: "unused-model".into(),
        model_sha256: [0; 32],
        destination: baseline_dir.to_string_lossy().into_owned(),
        group_offset_samples: 0,
        auto_stop_after_speech_samples: None,
        microphone: whisper_core::MicrophoneChoice::Device("test:mic".into()),
        microphone_gain_db: 0,
        config: whisper_core::JobConfig {
            language: whisper_core::LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        },
    };
    let mut baseline_capture = CaptureStream::empty_test_capture(16_000, 1, "test:mic");
    let mut baseline_converter = Pcm16Converter::new(16_000, 1).unwrap();
    let mut baseline_wav = crate::direct_live::WavOutput::create(&baseline_request).unwrap();
    for samples in &input_blocks {
        baseline_capture.enqueue_test_block(samples.clone());
        let block = baseline_capture.try_next_block().unwrap();
        baseline_wav
            .append(&baseline_converter.push(&block).unwrap())
            .unwrap();
        baseline_capture.recycle_block(block).unwrap();
    }
    baseline_wav
        .append(&baseline_converter.finish().unwrap())
        .unwrap();
    let without_probe_path = baseline_wav.finish().unwrap();
    assert_eq!(
        fs::read(with_probe_path).unwrap(),
        fs::read(without_probe_path).unwrap(),
        "Parakeet WAV with spectrum reader is byte-identical to the no-reader baseline"
    );
    drop(live);
    let _ = fs::remove_dir_all(directory);
}
