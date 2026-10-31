use crate::{
    capture::CaptureStream, direct_live::DirectLive, parakeet_import::ParakeetWorkerPaths,
};
use sha2::{Digest, Sha256};
use std::{
    collections::VecDeque,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant},
};
use whisper_core::{
    ComputeChoice, Generation, ImportEvent, JobConfig, JobId, LanguageChoice, LiveRequest,
    microphone_level::GainDb, model_catalog::ModelId,
};

static CASE_ID: AtomicU64 = AtomicU64::new(1);
const INPUTS: [(&str, &str, &str, &str, &str); 4] = [
    (
        "fr_pauses",
        "sound example/voxpopuli/clips/long_examples/fr_long_2020.wav",
        "docs/cadrage/feas-pm-01-corpus/candidate_fr_pauses_copie.txt",
        "541285a38fb6927c1bb4384825d2a7565a4a41efeade91c5f13de60bfefc5c0e",
        "abd2ac3ec6bc0fceb7c044f56d8ee0c29c4b2ca4a2ba8ca364ee169cb47e78cf",
    ),
    (
        "en_pauses",
        "sound example/voxpopuli/clips/long_examples/en_long_2020.wav",
        "docs/cadrage/feas-pm-01-corpus/candidate_en_pauses_copie.txt",
        "d73e3c08972ccb39eb3fc70ffb202c95c2b386fb3e59b5fafb753dab2c2ec18a",
        "c2971f66c57658ca613d578df807559604b5581c8b5ef72102900700fcf33cb1",
    ),
    (
        "fr_continu",
        "sound example/voxpopuli/clips/long_examples/fr_continuous60_2020_CANDIDATE.wav",
        "docs/cadrage/feas-pm-01-corpus/candidate_fr_continu_copie.txt",
        "322757782eda5332bdec4ff8bc7bbd87a5bbb76330d0d4da7f6e4d271f5a0df2",
        "ed150b6360d409b4d575b23253db5849dd709ab4f1b99416f29ba7d18a921ca7",
    ),
    (
        "en_continu",
        "sound example/voxpopuli/clips/long_examples/en_continuous60_2020_CANDIDATE.wav",
        "docs/cadrage/feas-pm-01-corpus/candidate_en_continu_copie.txt",
        "1fdcf0d8c00c426c7ed78c6724087db857b3acde465ea1f8bd77a3be8ca4b7c8",
        "93a7ae7ac4c940f98d64eecf5ef6ce0bed883da4202a4df5762b4f4782bf8bad",
    ),
];
const CONDITIONS: [(&str, i8, Option<i8>); 4] = [
    ("0db", 0, None),
    ("minus6db", -6, None),
    ("plus6db", 6, None),
    ("abrupt-minus6-to-plus6", -6, Some(6)),
];
const D2_POLL_INTERVAL: Duration = Duration::from_millis(5);
const D2_MAX_EVENTS_PER_DRAIN: usize = 32;
const D2_MAX_DRAIN_BATCHES: usize = 8;

fn d2_frame_deadline(playback_origin: Instant, frame_end: usize) -> Instant {
    let frames = frame_end as u64;
    let seconds = frames / 16_000;
    let remainder = frames % 16_000;
    playback_origin
        + Duration::from_secs(seconds)
        + Duration::from_nanos(remainder * 1_000_000_000 / 16_000)
}

fn d2_rebase_if_late(playback_origin: &mut Instant, frame_end: usize, now: Instant) -> bool {
    let deadline = d2_frame_deadline(*playback_origin, frame_end);
    if now < deadline {
        return false;
    }
    let elapsed = deadline.duration_since(*playback_origin);
    *playback_origin = now.checked_sub(elapsed).unwrap_or(now);
    true
}

fn drain_d2_events<E>(
    poll: &mut impl FnMut() -> Result<Option<E>, String>,
    record: &mut impl FnMut(E),
) -> Result<usize, String> {
    let mut drained = 0;
    for _ in 0..D2_MAX_EVENTS_PER_DRAIN {
        let Some(event) = poll()? else { break };
        record(event);
        drained += 1;
    }
    Ok(drained)
}

fn pace_d2_frame_end<E>(
    playback_origin: &mut Instant,
    frame_end: usize,
    poll: &mut impl FnMut() -> Result<Option<E>, String>,
    record: &mut impl FnMut(E),
) -> Result<(), String> {
    let deadline = d2_frame_deadline(*playback_origin, frame_end);
    loop {
        let drained = drain_d2_events(poll, record)?;
        let now = Instant::now();
        let remaining = deadline.saturating_duration_since(now);
        if remaining.is_zero() {
            d2_rebase_if_late(playback_origin, frame_end, now);
            return Ok(());
        }
        if drained == D2_MAX_EVENTS_PER_DRAIN {
            continue;
        }
        std::thread::sleep(remaining.min(D2_POLL_INTERVAL));
    }
}

fn record_parakeet_poll_event(
    event: ImportEvent,
    applied: &mut Vec<serde_json::Value>,
    poll_events: &mut Vec<String>,
) {
    poll_events.push(format!("{event:?}"));
    if let ImportEvent::MicrophoneGainApplied {
        gain_db,
        gain_revision,
        first_frame,
        ..
    } = event
    {
        applied.push(serde_json::json!({"gain_db":gain_db,"revision":gain_revision,"first_frame":first_frame}));
    }
}

fn drain_d2_events_after_playback<E>(
    poll: &mut impl FnMut() -> Result<Option<E>, String>,
    record: &mut impl FnMut(E),
) -> Result<(), String> {
    for _ in 0..D2_MAX_DRAIN_BATCHES {
        if drain_d2_events(poll, record)? < D2_MAX_EVENTS_PER_DRAIN {
            break;
        }
    }
    Ok(())
}

#[test]
fn d2_corpus_real_cpu_turbo_and_parakeet() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let root =
        PathBuf::from(std::env::var_os("WHISPER_MIC_LEVEL_D2_ROOT").expect("guarded D2 root"));
    fs::create_dir_all(&root).unwrap();
    let checkpoint = root.join("checkpoint.jsonl");
    let engine_filter =
        std::env::var("WHISPER_MIC_LEVEL_D2_ENGINE").expect("guarded D2 engine filter");
    assert_eq!(engine_filter, "parakeet-cpu", "D2 replay is Parakeet-only");
    record(
        &checkpoint,
        serde_json::json!({"event":"campaign-start","engine_filter":engine_filter,"cases":16,"timeout_seconds":1162}),
    );
    assert!(
        std::env::var_os("CUDA_CACHE_PATH").is_none(),
        "CPU D2 must not redirect CUDA cache"
    );
    let local = PathBuf::from(std::env::var_os("LOCALAPPDATA").expect("LOCALAPPDATA"));
    let turbo_worker = local.join("Whisper/app/whisper-worker-cpu.exe");
    let turbo_model = local.join("Whisper/models/ggml-large-v3-turbo.bin");
    assert!(turbo_worker.is_file() && turbo_model.is_file());
    let parakeet = ParakeetWorkerPaths {
        executable: local.join("Whisper/worker/whisper-worker-parakeet-cpu.exe"),
        native_dir: local.join("Whisper/native/cpu"),
        model: local.join("Whisper/models/parakeet-tdt-0.6b-v3.q8_0.gguf"),
        runtime_root: local.join("Whisper"),
    };
    parakeet.validate().expect("personal Parakeet CPU route");
    let engines = [
        ("turbo-cpu", turbo_model, ModelId::WhisperLargeV3Turbo),
        (
            "parakeet-cpu",
            parakeet.model.clone(),
            ModelId::ParakeetTdt06BV3,
        ),
    ];
    for (engine, model_path, model_id) in engines
        .into_iter()
        .filter(|(engine, _, _)| *engine == engine_filter.as_str())
    {
        for (corpus, wav_rel, ref_rel, wav_hash, ref_hash) in INPUTS {
            let input = repo.join(wav_rel);
            let reference_path = repo.join(ref_rel);
            assert_eq!(
                hash(&input),
                wav_hash,
                "approved WAV hash changed: {corpus}"
            );
            assert_eq!(
                hash(&reference_path),
                ref_hash,
                "approved reference hash changed: {corpus}"
            );
            let reference = fs::read_to_string(reference_path).unwrap();
            let (rate, channels, samples) = read_wav16(&input);
            assert_eq!(
                (rate, channels),
                (16_000, 1),
                "approved corpus WAV format changed"
            );
            for (condition, initial, transition) in CONDITIONS {
                let ordinal = CASE_ID.fetch_add(1, Ordering::Relaxed);
                let case_root = root.join(format!("{ordinal:02}-{engine}-{corpus}-{condition}"));
                fs::create_dir_all(&case_root).unwrap();
                record(
                    &checkpoint,
                    serde_json::json!({"event":"case-start","ordinal":ordinal,"engine":engine,"corpus":corpus,"condition":condition,"input_frames":samples.len()}),
                );
                let req = LiveRequest {
                    job_id: JobId(((std::process::id() as u128) << 64) | u128::from(ordinal)),
                    generation: Generation::first(),
                    model_path: model_path.to_string_lossy().into_owned(),
                    model_sha256: decode_sha256(
                        whisper_core::model_registry::model(model_id)
                            .artifact
                            .sha256,
                    ),
                    destination: case_root.to_string_lossy().into_owned(),
                    group_offset_samples: 0,
                    auto_stop_after_speech_samples: None,
                    microphone: whisper_core::MicrophoneChoice::Device("d2:approved-wav".into()),
                    microphone_gain_db: initial,
                    config: JobConfig {
                        language: LanguageChoice::Automatic,
                        compute: ComputeChoice::Cpu,
                    },
                };
                let capture = CaptureStream::empty_test_capture_with_gain(
                    rate,
                    channels,
                    "d2:approved-wav",
                    GainDb::new(initial).unwrap(),
                );
                let start = Instant::now();
                let (wav, txt, srt, frames, transcript, gain_frames) = if engine == "turbo-cpu" {
                    turbo_case(req, capture, &turbo_worker, &samples, transition)
                } else {
                    parakeet_case(req, capture, &parakeet, &samples, transition)
                };
                assert!(
                    Path::new(&wav).is_file()
                        && Path::new(&txt).is_file()
                        && Path::new(&srt).is_file()
                );
                assert_eq!(
                    frames,
                    samples.len() as u64,
                    "output frame continuity mismatch"
                );
                assert_eq!(
                    gain_frames.len(),
                    if transition.is_some() { 2 } else { 1 },
                    "gain application acknowledgements"
                );
                assert_eq!(gain_frames[0]["gain_db"], initial);
                assert_eq!(gain_frames[0]["revision"], 0);
                assert_eq!(gain_frames[0]["first_frame"], 0);
                if let Some(gain) = transition {
                    assert_eq!(gain_frames[1]["gain_db"], gain);
                    assert_eq!(gain_frames[1]["revision"], 1);
                    assert_eq!(gain_frames[1]["first_frame"], samples.len() / 2);
                }
                record(
                    &checkpoint,
                    serde_json::json!({
                        "event":"case-complete","ordinal":ordinal,"engine":engine,"corpus":corpus,"condition":condition,
                        "input_wav_sha256":wav_hash,"reference_sha256":ref_hash,"output_wav_sha256":hash(Path::new(&wav)),"input_frames":samples.len(),"output_frames":frames,
                        "wav":wav,"txt":txt,"srt":srt,"elapsed_ms":start.elapsed().as_millis(),
                        "approved_reference":reference,"transcript":transcript,"gain_frame_revisions":gain_frames,"comparison":"descriptive only; no WER threshold"
                    }),
                );
                eprintln!("D2_CASE_DONE {ordinal}/16 {engine} {corpus} {condition}");
            }
        }
    }
    record(
        &checkpoint,
        serde_json::json!({"event":"campaign-complete","engine_filter":engine_filter,"cases":16}),
    );
}

fn turbo_case(
    req: LiveRequest,
    capture: CaptureStream,
    worker: &Path,
    samples: &[i16],
    transition: Option<i8>,
) -> (String, String, String, u64, String, Vec<serde_json::Value>) {
    let mut live =
        DirectLive::from_test_capture_with_cpu_worker(req, capture, worker.to_path_buf(), 1)
            .unwrap();
    let mut applied = Vec::new();
    let mut offset = 0;
    let midpoint = samples.len() / 2;
    let mut changed = false;
    while offset < samples.len() {
        if !changed && offset >= midpoint {
            if let Some(gain) = transition {
                live.set_gain("d2:approved-wav", 0, GainDb::new(gain).unwrap())
                    .unwrap();
                changed = true;
            }
        }
        let boundary = if !changed && transition.is_some() {
            midpoint
        } else {
            samples.len()
        };
        let end = (offset + 1600).min(boundary).max(offset + 1);
        live.enqueue_test_capture_block(
            samples[offset..end]
                .iter()
                .map(|sample| f32::from(*sample) / 32768.0)
                .collect(),
        );
        if let Some(ImportEvent::MicrophoneGainApplied {
            gain_db,
            gain_revision,
            first_frame,
            ..
        }) = live.poll().unwrap()
        {
            applied.push(serde_json::json!({"gain_db":gain_db,"revision":gain_revision,"first_frame":first_frame}));
        }
        offset = end;
    }
    live.stop().unwrap();
    let deadline = Instant::now() + Duration::from_secs(3500);
    let mut published = None;
    while !live.done() && Instant::now() < deadline {
        if let Some(event) = live.poll().unwrap() {
            match event {
                    ImportEvent::DirectPublished { wav_path, txt_path, srt_path, .. } => published = Some((wav_path,txt_path,srt_path)),
                    ImportEvent::MicrophoneGainApplied { gain_db, gain_revision, first_frame, .. } => applied.push(serde_json::json!({"gain_db":gain_db,"revision":gain_revision,"first_frame":first_frame})),
                    _ => {}
                }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(live.done(), "Turbo CPU direct capture did not finish");
    let (wav, txt, srt) = published.expect("DirectPublished");
    let frames = pcm_frames(&wav);
    let transcript = fs::read_to_string(&txt).unwrap();
    assert!(!transcript.trim().is_empty(), "Turbo CPU transcript empty");
    (wav, txt, srt, frames, transcript, applied)
}

fn parakeet_case(
    req: LiveRequest,
    capture: CaptureStream,
    paths: &ParakeetWorkerPaths,
    samples: &[i16],
    transition: Option<i8>,
) -> (String, String, String, u64, String, Vec<serde_json::Value>) {
    let session = CASE_ID.load(Ordering::Relaxed);
    let destination = PathBuf::from(&req.destination);
    let mut live = crate::parakeet_live::ParakeetLive::start_test_capture(
        req,
        paths.clone(),
        session,
        capture,
    )
    .unwrap();
    let mut applied = Vec::new();
    let mut poll_events = Vec::new();
    let mut playback_origin = Instant::now();
    let mut offset = 0;
    let midpoint = samples.len() / 2;
    let mut changed = false;
    while offset < samples.len() {
        if !changed && offset >= midpoint {
            if let Some(gain) = transition {
                live.set_gain("d2:approved-wav", 0, GainDb::new(gain).unwrap())
                    .unwrap();
                changed = true;
            }
        }
        let boundary = if !changed && transition.is_some() {
            midpoint
        } else {
            samples.len()
        };
        let end = (offset + 1600).min(boundary).max(offset + 1);
        live.enqueue_d2_capture_block(
            samples[offset..end]
                .iter()
                .map(|sample| f32::from(*sample) / 32768.0)
                .collect(),
        );
        live.pump_capture().unwrap();
        let mut poll = || live.poll();
        let mut record = |event| record_parakeet_poll_event(event, &mut applied, &mut poll_events);
        pace_d2_frame_end(&mut playback_origin, end, &mut poll, &mut record).unwrap();
        offset = end;
    }
    {
        let mut poll = || live.poll();
        let mut record = |event| record_parakeet_poll_event(event, &mut applied, &mut poll_events);
        drain_d2_events_after_playback(&mut poll, &mut record).unwrap();
    }
    let event = live.stop().unwrap();
    let ImportEvent::ParakeetCaptureStopped {
        wav_path,
        txt_path: Some(txt),
        srt_path: Some(srt),
        final_segments,
        complete,
        diagnostic,
        unresolved,
        ..
    } = event
    else {
        panic!("missing Parakeet finalized outputs: {event:?}")
    };
    record_engine_status(
        &destination,
        serde_json::json!({
            "engine":"parakeet-cpu",
            "complete":complete,
            "diagnostic":diagnostic.clone(),
            "confirmed_segments":final_segments.len(),
            "unresolved":unresolved.iter().map(|range| format!("{range:?}")).collect::<Vec<_>>(),
            "gain_frame_revisions":applied.clone(),
            "poll_events":poll_events,
            "worker_events_path":destination.join("d2-worker-events.jsonl"),
            "wav_path":wav_path.clone(),
            "txt_path":txt.clone(),
            "srt_path":srt.clone(),
        }),
    );
    assert!(
        !final_segments.is_empty(),
        "Parakeet CPU returned no segments"
    );
    let frames = pcm_frames(&wav_path);
    let transcript = fs::read_to_string(&txt).unwrap();
    assert!(
        !transcript.trim().is_empty(),
        "Parakeet CPU transcript empty"
    );
    (wav_path, txt, srt, frames, transcript, applied)
}

fn record_engine_status(root: &Path, status: serde_json::Value) {
    record(&root.join("d2-engine-status.json"), status);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn d2_frame_deadlines_follow_the_monotone_one_x_sample_clock() {
        let origin = Instant::now();
        let after_100ms = d2_frame_deadline(origin, 1_600);
        let after_200ms = d2_frame_deadline(origin, 3_200);
        assert_eq!(
            after_100ms.duration_since(origin),
            Duration::from_millis(100)
        );
        assert_eq!(
            after_200ms.duration_since(origin),
            Duration::from_millis(200)
        );
        assert!(after_200ms > after_100ms);
    }

    #[test]
    fn d2_event_drain_stops_after_its_bounded_batch() {
        let mut pending = (0..D2_MAX_EVENTS_PER_DRAIN + 3).collect::<VecDeque<_>>();
        let mut poll = || Ok(pending.pop_front());
        let mut observed = Vec::new();
        let mut record = |event| observed.push(event);
        assert_eq!(
            drain_d2_events(&mut poll, &mut record).unwrap(),
            D2_MAX_EVENTS_PER_DRAIN
        );
        drop(record);
        drop(poll);
        assert_eq!(observed.len(), D2_MAX_EVENTS_PER_DRAIN);
        assert_eq!(pending.len(), 3);
        let mut poll = || Ok(pending.pop_front());
        let mut record = |event| observed.push(event);
        assert_eq!(drain_d2_events(&mut poll, &mut record).unwrap(), 3);
        assert_eq!(observed.len(), D2_MAX_EVENTS_PER_DRAIN + 3);
    }

    #[test]
    fn d2_driver_paces_frames_and_drains_events_before_deadline() {
        let origin = Instant::now();
        let mut pending = VecDeque::from([1, 2]);
        let mut polls = 0;
        let mut poll = || {
            polls += 1;
            Ok(pending.pop_front())
        };
        let mut observed = Vec::new();
        let mut record = |event| observed.push(event);
        let mut playback_origin = origin;
        pace_d2_frame_end(&mut playback_origin, 1_600, &mut poll, &mut record).unwrap();

        assert!(Instant::now().duration_since(origin) >= Duration::from_millis(100));
        assert_eq!(observed, vec![1, 2]);
        assert!(polls > 2, "driver must poll throughout the frame interval");
    }

    #[test]
    fn d2_expired_frame_deadline_rebases_without_catch_up_burst() {
        let now = Instant::now();
        let mut playback_origin = now - Duration::from_secs(8);
        assert!(d2_rebase_if_late(&mut playback_origin, 1_600, now));

        assert_eq!(d2_frame_deadline(playback_origin, 1_600), now);
        assert_eq!(
            d2_frame_deadline(playback_origin, 3_200),
            now + Duration::from_millis(100)
        );
        assert!(!d2_rebase_if_late(&mut playback_origin, 3_200, now));
    }

    #[test]
    fn d2_post_playback_drain_has_a_bounded_batch_count() {
        let mut pending =
            (0..D2_MAX_EVENTS_PER_DRAIN * (D2_MAX_DRAIN_BATCHES + 2)).collect::<VecDeque<_>>();
        let mut observed = Vec::new();
        let mut poll = || Ok(pending.pop_front());
        let mut record = |event| observed.push(event);
        drain_d2_events_after_playback(&mut poll, &mut record).unwrap();

        assert_eq!(
            observed.len(),
            D2_MAX_EVENTS_PER_DRAIN * D2_MAX_DRAIN_BATCHES
        );
        assert_eq!(pending.len(), D2_MAX_EVENTS_PER_DRAIN * 2);
    }

    #[test]
    fn test_status_writer_preserves_diagnostics() {
        let root =
            PathBuf::from(std::env::var_os("WHISPER_MIC_LEVEL_D2_ROOT").expect("guarded D2 root"));
        fs::create_dir_all(&root).unwrap();
        let case_root = root.join(format!("status-selftest-{}", std::process::id()));
        fs::create_dir_all(&case_root).unwrap();
        let status = serde_json::json!({
            "engine":"parakeet-cpu",
            "complete":false,
            "confirmed_segments":0,
            "diagnostic":"synthetic worker failure",
            "worker_events_path":"synthetic-events.jsonl",
        });
        record_engine_status(&case_root, status.clone());
        let saved: serde_json::Value =
            serde_json::from_slice(&fs::read(case_root.join("d2-engine-status.json")).unwrap())
                .unwrap();
        assert_eq!(saved, status);
    }
}

fn read_wav16(path: &Path) -> (u32, u16, Vec<i16>) {
    let bytes = fs::read(path).unwrap();
    assert_eq!(&bytes[..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
    let (mut rate, mut channels, mut format, mut bits, mut pcm) = (None, None, None, None, None);
    let mut at = 12;
    while at + 8 <= bytes.len() {
        let size = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().unwrap()) as usize;
        let begin = at + 8;
        let end = begin.checked_add(size).unwrap();
        assert!(end <= bytes.len());
        match &bytes[at..at + 4] {
            b"fmt " => {
                format = Some(u16::from_le_bytes(
                    bytes[begin..begin + 2].try_into().unwrap(),
                ));
                channels = Some(u16::from_le_bytes(
                    bytes[begin + 2..begin + 4].try_into().unwrap(),
                ));
                rate = Some(u32::from_le_bytes(
                    bytes[begin + 4..begin + 8].try_into().unwrap(),
                ));
                bits = Some(u16::from_le_bytes(
                    bytes[begin + 14..begin + 16].try_into().unwrap(),
                ));
            }
            b"data" => pcm = Some(&bytes[begin..end]),
            _ => {}
        }
        at = end + (size % 2);
    }
    assert_eq!(format, Some(1));
    assert_eq!(bits, Some(16));
    let samples = pcm
        .unwrap()
        .chunks_exact(2)
        .map(|p| i16::from_le_bytes([p[0], p[1]]))
        .collect();
    (rate.unwrap(), channels.unwrap(), samples)
}
fn pcm_frames(path: &str) -> u64 {
    let bytes = fs::read(path).unwrap();
    assert_eq!(&bytes[..4], b"RIFF");
    (bytes.len() - 44) as u64 / 2
}
fn hash(path: &Path) -> String {
    format!("{:x}", Sha256::digest(fs::read(path).unwrap()))
}
fn decode_sha256(value: &str) -> [u8; 32] {
    assert_eq!(value.len(), 64);
    std::array::from_fn(|index| u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).unwrap())
}
fn record(path: &Path, value: serde_json::Value) {
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .unwrap();
    serde_json::to_writer(&mut f, &value).unwrap();
    f.write_all(b"\n").unwrap();
    f.sync_data().unwrap();
}
