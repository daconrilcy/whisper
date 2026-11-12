use super::*;
use std::{
    collections::VecDeque,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use whisper_core::{
    ComputeChoice, Generation, ImportEvent, JobConfig, JobId, LanguageChoice, LiveRequest,
    ports::ImportEffect,
};

const APPROVED_TURBO_MODEL_SHA256: &str =
    "1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69";
const SAMPLE_COUNT: usize = 32_000;

fn required_file(name: &str) -> PathBuf {
    let value =
        std::env::var_os(name).unwrap_or_else(|| panic!("required native input {name} is unset"));
    let path = PathBuf::from(value);
    assert!(
        path.is_file(),
        "required native input {name} is not a file: {}",
        path.display()
    );
    path
}

fn model_hash(path: &Path) -> [u8; 32] {
    let (size, hash) = crate::model_catalog::cached_model_sha256(path)
        .unwrap_or_else(|error| panic!("read Turbo model {}: {error}", path.display()));
    assert_eq!(size, 1_624_555_275, "approved Turbo model size changed");
    let actual = hash
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(
        actual, APPROVED_TURBO_MODEL_SHA256,
        "approved Turbo model hash changed"
    );
    hash
}

fn collect_poll(runtime: &mut ImportRuntime, events: &Arc<Mutex<Vec<ImportEvent>>>) {
    if let Some(result) = runtime.poll() {
        match result {
            Ok(event) => events.lock().expect("event collection lock").push(event),
            Err((job_id, generation, message)) => {
                panic!("ImportRuntime::poll failed for {job_id:?}/{generation:?}: {message}")
            }
        }
    }
}

fn wait_for_cpu_ready(
    runtime: &mut ImportRuntime,
    events: &Arc<Mutex<Vec<ImportEvent>>>,
    job_id: JobId,
) {
    let deadline = Instant::now() + Duration::from_secs(60);
    while !events.lock().expect("event collection lock").iter().any(|event| {
        matches!(event, ImportEvent::Ready { job_id: actual, generation, backend }
            if *actual == job_id && *generation == Generation::first() && backend.starts_with("CPU"))
    }) {
        assert!(Instant::now() < deadline, "native CPU worker did not become Ready");
        collect_poll(runtime, events);
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn feed_paced_audio(runtime: &mut ImportRuntime, events: &Arc<Mutex<Vec<ImportEvent>>>) {
    let origin = Instant::now();
    for block in 0..(SAMPLE_COUNT / 320) {
        let due = origin + Duration::from_millis((block * 20) as u64);
        if let Some(delay) = due.checked_duration_since(Instant::now()) {
            std::thread::sleep(delay);
        }
        let first = block * 320;
        let audio = (first..first + 320)
            .map(|frame| 0.12 * (std::f32::consts::TAU * 440.0 * frame as f32 / 16_000.0).sin())
            .collect();
        runtime
            .direct_live
            .as_mut()
            .expect("direct runtime remains active during capture")
            .enqueue_test_capture_block(audio);
        collect_poll(runtime, events);
    }
}

fn stop_and_collect(runtime: &mut ImportRuntime, job_id: JobId) -> Vec<ImportEvent> {
    let mut events = VecDeque::new();
    runtime.effect(
        ImportEffect::Stop {
            job_id,
            generation: Generation::first(),
        },
        &mut events,
    );
    let mut captured = events.into_iter().collect::<Vec<_>>();
    let deadline = Instant::now() + Duration::from_secs(300);
    while !captured.iter().any(|event| {
        matches!(event, ImportEvent::DirectPublished { job_id: actual, .. } if *actual == job_id)
    }) {
        assert!(Instant::now() < deadline, "native Turbo direct poll did not publish");
        if let Some(result) = runtime.poll() {
            match result {
                Ok(event) => captured.push(event),
                Err((actual, generation, message)) => panic!(
                    "ImportRuntime::poll failed for {actual:?}/{generation:?}: {message}"
                ),
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    captured
}

fn assert_published_outputs(events: &[ImportEvent], job_id: JobId) -> u64 {
    let published = events
        .iter()
        .find_map(|event| match event {
            ImportEvent::DirectPublished {
                job_id: actual,
                generation,
                txt_path,
                srt_path,
                wav_path,
            } if *actual == job_id && *generation == Generation::first() => {
                Some((txt_path, srt_path, wav_path))
            }
            _ => None,
        })
        .expect("matching DirectPublished event");
    let paths = [
        published.0.as_str(),
        published.1.as_str(),
        published.2.as_str(),
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect::<Vec<_>>();
    assert!(
        paths.iter().all(|path| path.is_file()),
        "all direct exports exist"
    );
    let stems = paths
        .iter()
        .map(|path| path.file_stem().expect("output stem").to_owned())
        .collect::<Vec<_>>();
    assert!(
        stems.iter().all(|stem| stem == &stems[0]),
        "direct export stems differ"
    );
    assert_eq!(
        paths[0].extension().and_then(|ext| ext.to_str()),
        Some("txt")
    );
    assert_eq!(
        paths[1].extension().and_then(|ext| ext.to_str()),
        Some("srt")
    );
    assert_eq!(
        paths[2].extension().and_then(|ext| ext.to_str()),
        Some("wav")
    );
    let wav = fs::read(&paths[2]).expect("read published WAV");
    assert_wav_header_and_sample_count(&wav)
}

fn assert_wav_header_and_sample_count(wav: &[u8]) -> u64 {
    assert_eq!(
        wav.len(),
        44 + SAMPLE_COUNT * 2,
        "published WAV byte length"
    );
    assert_eq!(&wav[0..4], b"RIFF");
    assert_eq!(&wav[8..12], b"WAVE");
    assert_eq!(
        u32::from_le_bytes(wav[4..8].try_into().expect("RIFF size")) as usize,
        wav.len() - 8,
        "RIFF declared size"
    );
    assert_eq!(&wav[12..16], b"fmt ");
    assert_eq!(
        u32::from_le_bytes(wav[16..20].try_into().expect("format size")),
        16
    );
    assert_eq!(
        u16::from_le_bytes(wav[20..22].try_into().expect("audio format")),
        1
    );
    assert_eq!(
        u16::from_le_bytes(wav[22..24].try_into().expect("channel count")),
        1
    );
    assert_eq!(
        u32::from_le_bytes(wav[24..28].try_into().expect("sample rate")),
        16_000
    );
    assert_eq!(
        u32::from_le_bytes(wav[28..32].try_into().expect("byte rate")),
        32_000
    );
    assert_eq!(
        u16::from_le_bytes(wav[32..34].try_into().expect("block alignment")),
        2
    );
    assert_eq!(
        u16::from_le_bytes(wav[34..36].try_into().expect("bits per sample")),
        16
    );
    assert_eq!(&wav[36..40], b"data");
    let data_bytes = u32::from_le_bytes(wav[40..44].try_into().expect("WAV data size"));
    assert_eq!(
        data_bytes as usize,
        SAMPLE_COUNT * 2,
        "PCM payload byte length"
    );
    let sample_count = u64::from(data_bytes) / 2;
    assert_eq!(
        sample_count, SAMPLE_COUNT as u64,
        "published WAV sample count"
    );
    sample_count
}

fn run_worker_case(
    root: &Path,
    worker: &Path,
    model: &Path,
    model_sha256: [u8; 32],
    job_id: JobId,
) -> u64 {
    let destination = root.join(format!("worker-{:032x}", job_id.0));
    fs::create_dir_all(&destination).expect("create direct output directory");
    let request = LiveRequest {
        job_id,
        generation: Generation::first(),
        model_path: model.to_string_lossy().into_owned(),
        model_sha256,
        destination: destination.to_string_lossy().into_owned(),
        group_offset_samples: 0,
        auto_stop_after_speech_samples: None,
        microphone: whisper_core::MicrophoneChoice::Device("test:native-direct-cpu".into()),
        microphone_gain_db: 0,
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        },
    };
    let capture =
        crate::capture::CaptureStream::empty_test_capture(16_000, 1, "test:native-direct-cpu");
    let direct = crate::direct_live::DirectLive::from_test_capture_with_cpu_worker(
        request,
        capture,
        worker.to_path_buf(),
        job_id.0 as u64,
    )
    .unwrap_or_else(|error| {
        panic!(
            "start native Turbo CPU worker {}: {error}",
            worker.display()
        )
    });
    assert_eq!(direct.cache_identity().0, ComputeChoice::Cpu);

    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: worker.to_path_buf(),
            gpu: worker.to_path_buf(),
        },
        Arc::new(PublicationGate::default()),
    );
    runtime.direct_live = Some(direct);
    let events = Arc::new(Mutex::new(Vec::new()));
    wait_for_cpu_ready(&mut runtime, &events, job_id);
    feed_paced_audio(&mut runtime, &events);
    let events = stop_and_collect(&mut runtime, job_id);
    assert_published_outputs(&events, job_id)
}

#[test]
#[cfg_attr(
    not(feature = "native-tests"),
    ignore = "requires guarded WHISPER_TEST_WORKER_EXE and verified Turbo model"
)]
fn native_direct_turbo_cpu_finishes_through_poll() {
    let root = TestRoot::new("native-direct-turbo-cpu");
    let worker = required_file("WHISPER_TEST_WORKER_EXE");
    let model = required_file("WHISPER_TEST_REUSE_MODEL");
    let model_sha256 = model_hash(&model);

    let sample_count = run_worker_case(&root.0, &worker, &model, model_sha256, JobId(0xD2101));
    assert_eq!(sample_count, SAMPLE_COUNT as u64);
}
