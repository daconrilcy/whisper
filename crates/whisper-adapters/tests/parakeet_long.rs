use std::{
    fs,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant},
};

use sha2::{Digest, Sha256};
use whisper_adapters::parakeet_import::{ParakeetProcess, ParakeetWorkerPaths};
use whisper_adapters::{archive::ArchiveStore, supervisor::WorkerPaths, worker_ipc::AsyncImportIo};
use whisper_core::{
    AppCommand, Application, ComputeChoice, Generation, ImportApplication, ImportRequest,
    JobConfig, JobId, LanguageChoice,
    model_catalog::{ModelAvailability, ModelId},
    model_registry,
    model_selection_app::ModelCommandGuard,
    parakeet_ipc::{Event, WorkerIdentity},
    parakeet_windows::{
        MAX_NATIVE_SAMPLES, PROFILE_IMPORT_VAD, PROFILE_QUALIFICATION_REFERENCE, Window,
        absolute_word_samples, admissible_silences, plan_windows, word_belongs_to_central_range,
    },
};

const MODEL_SHA256: &str = "e3880d0aaaaf2c308ea2c35016b2b895c423eb3fda924c1b463d1c19b7f4d32e";
const NATIVE_DLLS: [&str; 5] = [
    "nemo_speech_asr_c.dll",
    "nemo_speech_asr.dll",
    "ggml.dll",
    "ggml-base.dll",
    "ggml-cpu.dll",
];

#[derive(Clone, Debug, Eq, PartialEq)]
struct TimedWord {
    text: String,
    start_ms: u32,
    end_ms: u32,
}

struct Inference {
    identity: WorkerIdentity,
    duration_samples: u64,
    text: String,
    words: Vec<TimedWord>,
}

fn path_var(name: &str) -> PathBuf {
    PathBuf::from(std::env::var_os(name).unwrap_or_else(|| panic!("{name} is required")))
}

fn sha256(path: &Path) -> String {
    let bytes = fs::read(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    format!("{:x}", Sha256::digest(bytes))
}

fn runtime_paths() -> (PathBuf, ParakeetWorkerPaths) {
    let runtime = path_var("PARAKEET_L2_RUNTIME_ROOT");
    let paths = ParakeetWorkerPaths {
        executable: path_var("PARAKEET_L2_WORKER"),
        native_dir: path_var("PARAKEET_L2_NATIVE_DIR"),
        model: path_var("PARAKEET_L2_MODEL"),
        runtime_root: runtime.clone(),
    };
    paths
        .validate()
        .expect("worker, native DLLs and model paths are qualified");
    paths
        .verify_qualification(
            MODEL_SHA256,
            &runtime_hash(&paths.native_dir),
            &sha256(&paths.executable),
        )
        .expect("the exact L2b runtime, worker and model key is present");
    (runtime, paths)
}

fn runtime_hash(native: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"nemo-speech-asr 0.2.0");
    for name in NATIVE_DLLS {
        hasher.update(name.as_bytes());
        hasher.update(sha256(&native.join(name)).as_bytes());
    }
    format!("{:x}", hasher.finalize())
}

fn start_process(paths: &ParakeetWorkerPaths, profile: &str) -> (ParakeetProcess, WorkerIdentity) {
    let mut process = ParakeetProcess::spawn_for_qualification(paths, profile)
        .unwrap_or_else(|error| panic!("worker startup failed: {error}"));
    let ready = wait_event(&mut process, Duration::from_secs(120));
    let Event::Ready {
        identity,
        backend,
        native_version,
        ..
    } = ready
    else {
        panic!("worker did not attest Ready: {ready:?}");
    };
    assert_eq!(identity.model_sha256, MODEL_SHA256);
    assert_eq!(identity.runtime_sha256, runtime_hash(&paths.native_dir));
    assert_eq!(identity.build_sha256, sha256(&paths.executable));
    assert_eq!(identity.profile, profile);
    assert_eq!(backend, "CPU");
    assert!(native_version.contains("0.2.0"));
    process.accept_ready(identity.clone()).unwrap();
    (process, identity)
}

fn wait_event(process: &mut ParakeetProcess, limit: Duration) -> Event {
    let started = Instant::now();
    loop {
        match process.poll() {
            Ok(Some(event)) => return event,
            Ok(None) => thread::sleep(Duration::from_millis(20)),
            Err(error) => panic!("worker event failed: {error}"),
        }
        assert!(
            started.elapsed() < limit,
            "worker event exceeded the test bound"
        );
    }
}

fn transcribe(
    process: &mut ParakeetProcess,
    identity: &WorkerIdentity,
    source: &Path,
    job_id: &str,
    generation: u64,
    attempt: u64,
    limit: Duration,
) -> Inference {
    process
        .transcribe(job_id.into(), generation, attempt, source, sha256(source))
        .unwrap();
    let event = wait_event(process, limit);
    let Event::Result {
        job_id: got_job,
        generation: got_generation,
        attempt: got_attempt,
        identity: got_identity,
        duration_samples,
        text,
        words,
        ..
    } = event
    else {
        panic!("transcription did not return Result: {event:?}");
    };
    assert_eq!(got_job, job_id);
    assert_eq!(got_generation, generation);
    assert_eq!(got_attempt, attempt);
    assert_eq!(&got_identity, identity);
    Inference {
        identity: got_identity,
        duration_samples,
        text,
        words: words
            .into_iter()
            .map(|word| TimedWord {
                text: word.text,
                start_ms: word.start_ms,
                end_ms: word.end_ms,
            })
            .collect(),
    }
}

fn stop_process(process: &mut ParakeetProcess) {
    process.shutdown().unwrap();
    assert!(matches!(
        wait_event(process, Duration::from_secs(120)),
        Event::Stopped
    ));
    let started = Instant::now();
    loop {
        match process.try_wait().unwrap() {
            Some(status) => {
                assert!(status.success(), "worker exit status: {status}");
                return;
            }
            None => thread::sleep(Duration::from_millis(20)),
        }
        assert!(started.elapsed() < Duration::from_secs(120));
    }
}

fn read_pcm16_mono(path: &Path) -> Vec<i16> {
    let bytes = fs::read(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    assert!(bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WAVE");
    let mut offset = 12_usize;
    let mut format = None;
    let mut data = None;
    while offset + 8 <= bytes.len() {
        let id = &bytes[offset..offset + 4];
        let length = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        let start = offset + 8;
        let end = start
            .checked_add(length)
            .filter(|end| *end <= bytes.len())
            .unwrap();
        if id == b"fmt " {
            assert!(length >= 16);
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
        offset = end + (length & 1);
    }
    assert_eq!(format, Some((1, 1, 16_000, 2, 16)));
    let data = data.expect("WAV data chunk");
    assert_eq!(data.len() % 2, 0);
    data.chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
        .collect()
}

fn write_pcm16_mono(path: &Path, samples: &[i16]) {
    let data_len = u32::try_from(samples.len() * 2).unwrap();
    let mut bytes = Vec::with_capacity(data_len as usize + 44);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(data_len + 36).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&16_000_u32.to_le_bytes());
    bytes.extend_from_slice(&32_000_u32.to_le_bytes());
    bytes.extend_from_slice(&2_u16.to_le_bytes());
    bytes.extend_from_slice(&16_u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for sample in samples {
        bytes.extend_from_slice(&sample.to_le_bytes());
    }
    fs::write(path, bytes).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
}

fn infer_reference(paths: &ParakeetWorkerPaths, source: &Path, job: &str) -> Inference {
    let (mut process, identity) = start_process(paths, PROFILE_QUALIFICATION_REFERENCE);
    let result = transcribe(
        &mut process,
        &identity,
        source,
        job,
        1,
        1,
        Duration::from_secs(5_400),
    );
    stop_process(&mut process);
    result
}

fn infer_vad(
    runtime: &Path,
    paths: &ParakeetWorkerPaths,
    source: &Path,
    job: &str,
) -> (Inference, Vec<Window>, Vec<TimedWord>) {
    let pcm = read_pcm16_mono(source);
    let windows = plan_windows(&pcm).unwrap_or_else(|error| panic!("VAD planning failed: {error}"));
    assert!(
        windows.len() > 1,
        "long test source should require VAD windows"
    );
    let vad_dir = runtime.join("long").join(job).join("vad");
    fs::create_dir_all(&vad_dir).unwrap();
    let (mut process, identity) = start_process(paths, PROFILE_IMPORT_VAD);
    let duration_samples = pcm.len() as u64;
    let mut all_words = Vec::new();
    let mut texts = Vec::new();
    for window in &windows {
        assert!(window.native_end_sample - window.native_start_sample <= MAX_NATIVE_SAMPLES);
        let chunk_path = vad_dir.join(format!("window-{:03}.wav", window.index));
        let chunk = &pcm[window.native_start_sample as usize..window.native_end_sample as usize];
        write_pcm16_mono(&chunk_path, chunk);
        let result = transcribe(
            &mut process,
            &identity,
            &chunk_path,
            job,
            1,
            window.index as u64 + 1,
            Duration::from_secs(1_800),
        );
        assert_eq!(result.duration_samples, chunk.len() as u64);
        texts.push(result.text);
        let ipc_words = result
            .words
            .iter()
            .map(|word| whisper_core::parakeet_ipc::Word {
                text: word.text.clone(),
                start_ms: word.start_ms,
                end_ms: word.end_ms,
            })
            .collect::<Vec<_>>();
        let absolute = absolute_word_samples(window, duration_samples, &ipc_words)
            .unwrap_or_else(|error| panic!("invalid absolute timestamps: {error}"));
        for (word, timing) in result.words.into_iter().zip(absolute) {
            let start = timing.start_sample;
            let end = timing.end_sample;
            if word_belongs_to_central_range(
                start,
                end,
                window.central_start_sample,
                window.central_end_sample,
            ) {
                all_words.push(TimedWord {
                    text: word.text,
                    start_ms: u32::try_from(start / 16).unwrap(),
                    end_ms: u32::try_from(end / 16).unwrap(),
                });
            }
        }
    }
    stop_process(&mut process);
    all_words.sort_by_key(|word| (word.start_ms, word.end_ms));
    assert!(
        all_words
            .iter()
            .all(|word| word.start_ms < word.end_ms
                && u64::from(word.end_ms) <= duration_samples / 16)
    );
    let inference = Inference {
        identity,
        duration_samples,
        text: texts.join(" "),
        words: all_words.clone(),
    };
    (inference, windows, all_words)
}

struct ProductImport {
    transcript: String,
    segments: Vec<(u64, u64, u64, String)>,
    duration_samples: u64,
}

fn import_through_product_route(
    runtime: &Path,
    paths: &ParakeetWorkerPaths,
    source: &Path,
    job_nonce: u64,
) -> ProductImport {
    let destination = runtime.join(format!("archive/product-vad-{job_nonce}"));
    fs::create_dir_all(&destination).unwrap();
    let turbo_path = runtime.join("historical/ggml-large-v3-turbo.bin");
    let turbo_decoder = path_var("PARAKEET_L2_TURBO_DECODE_WORKER");
    let io = AsyncImportIo::start_with_model_catalog_and_parakeet(
        WorkerPaths {
            cpu: turbo_decoder.clone(),
            gpu: turbo_decoder,
        },
        paths.model.parent().unwrap().to_path_buf(),
        turbo_path.to_string_lossy().into_owned(),
        Some(paths.clone()),
    );
    let snapshot = model_registry::engine_snapshot(1, |_| ModelAvailability::Ready, vec![]);
    let guard = ModelCommandGuard::new_with_model_paths(
        snapshot,
        1,
        [
            (
                ModelId::ParakeetTdt06BV3,
                paths.model.to_string_lossy().into_owned(),
            ),
            (
                ModelId::WhisperLargeV3Turbo,
                turbo_path.to_string_lossy().into_owned(),
            ),
        ],
    );
    let mut app = ImportApplication::new(io).with_model_guard(guard);
    let record = model_registry::model(ModelId::ParakeetTdt06BV3);
    let job_id = JobId(0x4c32_0000_0000_0000_u128 + u128::from(job_nonce));
    let mut request = ImportRequest {
        job_id,
        generation: Generation::first(),
        source_path: source.to_string_lossy().into_owned(),
        source_sha256: None,
        source_samples: None,
        model_path: paths.model.to_string_lossy().into_owned(),
        model_sha256: decode_sha256(record.artifact.sha256),
        destination: destination.to_string_lossy().into_owned(),
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        },
    };
    ArchiveStore::identify_source(&mut request).unwrap();
    let duration_samples = request.source_samples.unwrap();
    app.dispatch(AppCommand::StartImport { request }).unwrap();
    let deadline = Instant::now() + Duration::from_secs(3_600);
    loop {
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        if view.active_job == Some((job_id, whisper_core::JobState::Complete)) {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "product VAD import did not complete: {view:?}"
        );
        thread::sleep(Duration::from_millis(100));
    }
    let passage = destination
        .join("transcriptions")
        .join(format!("{:032x}", job_id.0))
        .join("1");
    let complete = ArchiveStore::scan(&destination).unwrap();
    assert_eq!(complete.len(), 1);
    assert!(
        complete[0].complete,
        "product route published one generation"
    );
    assert!(passage.join("CURRENT").is_file());
    assert!(!passage.join("pending.json").exists());
    let transcript = fs::read_to_string(passage.join("transcript.txt")).unwrap();
    let segments = fs::read_to_string(passage.join("segments.jsonl"))
        .unwrap()
        .lines()
        .map(|line| {
            let row: serde_json::Value = serde_json::from_str(line).unwrap();
            let range = &row["range"];
            (
                row["sequence"].as_u64().unwrap(),
                range["start_sample"].as_u64().unwrap(),
                range["end_sample"].as_u64().unwrap(),
                row["text"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    ProductImport {
        transcript,
        segments,
        duration_samples,
    }
}

fn verify_product_matches_vad_route(product: ProductImport, vad: &Inference, windows: &[Window]) {
    assert_eq!(product.duration_samples, vad.duration_samples);
    let expected = vad
        .words
        .iter()
        .flat_map(|word| normalized_tokens(&word.text))
        .collect::<Vec<_>>();
    let actual = normalized_tokens(&product.transcript);
    assert_eq!(
        actual, expected,
        "product route transcript matches the same-candidate VAD output"
    );
    assert!(!product.segments.is_empty());
    for (index, (sequence, start, end, text)) in product.segments.iter().enumerate() {
        assert_eq!(
            *sequence,
            index as u64 + 1,
            "one contiguous publication sequence"
        );
        assert!(!text.trim().is_empty());
        assert!(start < end && *end <= product.duration_samples);
        let owners = windows
            .iter()
            .filter(|window| {
                *start >= window.central_start_sample && *end <= window.central_end_sample
            })
            .count();
        assert_eq!(owners, 1, "published segment belongs to one central window");
    }
    let cut_count = windows.iter().filter(|window| window.cut.is_some()).count();
    eprintln!(
        "L2B_PRODUCT_ROUTE profile=import-vad duration_samples={} windows={} cuts={} segments={} token_count={} duplicate_segments=0 publication_count=1",
        product.duration_samples,
        windows.len(),
        cut_count,
        product.segments.len(),
        actual.len()
    );
}

fn decode_sha256(value: &str) -> [u8; 32] {
    let mut bytes = [0_u8; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).unwrap();
    }
    bytes
}

fn normalized_tokens(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    for character in text.chars().flat_map(char::to_lowercase) {
        if character.is_alphanumeric() {
            current.push(character);
        } else if !current.is_empty() {
            words.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    let mut normalized = Vec::with_capacity(words.len());
    let mut index = 0;
    while index < words.len() {
        if matches!(
            words[index].as_str(),
            "um" | "uh" | "euh" | "hum" | "ben" | "bah"
        ) {
            index += 1;
        } else if words[index] == "you" && words.get(index + 1).is_some_and(|word| word == "know") {
            index += 2;
        } else if words[index] == "i" && words.get(index + 1).is_some_and(|word| word == "mean") {
            index += 2;
        } else {
            normalized.push(words[index].clone());
            index += 1;
        }
    }
    normalized
}

fn edit_distance(left: &[String], right: &[String]) -> usize {
    let mut row = (0..=right.len()).collect::<Vec<_>>();
    for (left_index, left_word) in left.iter().enumerate() {
        let mut diagonal = row[0];
        row[0] = left_index + 1;
        for (right_index, right_word) in right.iter().enumerate() {
            let above = row[right_index + 1];
            row[right_index + 1] = (diagonal + usize::from(left_word != right_word))
                .min(row[right_index] + 1)
                .min(above + 1);
            diagonal = above;
        }
    }
    row[right.len()]
}

fn boundary_words(words: &[TimedWord], cut_ms: u32) -> Vec<String> {
    words
        .iter()
        .filter(|word| {
            let center = word.start_ms + (word.end_ms - word.start_ms) / 2;
            center.abs_diff(cut_ms) <= 2_000
        })
        .flat_map(|word| normalized_tokens(&word.text))
        .collect()
}

fn verify_comparison(
    reference: &Inference,
    vad: &Inference,
    windows: &[Window],
    words: &[TimedWord],
) {
    assert_eq!(reference.identity.model_sha256, vad.identity.model_sha256);
    assert_eq!(
        reference.identity.runtime_sha256,
        vad.identity.runtime_sha256
    );
    assert_eq!(reference.identity.build_sha256, vad.identity.build_sha256);
    assert_eq!(reference.duration_samples, vad.duration_samples);
    let reference_tokens = normalized_tokens(&reference.text);
    let vad_tokens = normalized_tokens(&vad.text);
    let global_edits = edit_distance(&reference_tokens, &vad_tokens);
    eprintln!(
        "L2B_LONG_SUMMARY profile=import-vad duration_samples={} windows={} reference_tokens={} vad_tokens={} global_edits={global_edits}",
        vad.duration_samples,
        windows.len(),
        reference_tokens.len(),
        vad_tokens.len()
    );
    for window in windows.iter().filter(|window| window.cut.is_some()) {
        let cut_ms = (window.central_end_sample / 16) as u32;
        let ref_zone = boundary_words(&reference.words, cut_ms);
        let vad_zone = boundary_words(words, cut_ms);
        let edits = edit_distance(&ref_zone, &vad_zone);
        let truncated = words
            .iter()
            .filter(|word| word.start_ms < cut_ms && cut_ms < word.end_ms)
            .count();
        eprintln!(
            "L2B_BOUNDARY cut_sample={} cut_ms={} reference_words={} vad_words={} edits={} truncated={truncated}",
            window.central_end_sample,
            cut_ms,
            ref_zone.len(),
            vad_zone.len(),
            edits,
        );
        assert!(edits <= 2, "more than two lexical edits at cut {cut_ms} ms");
        assert_eq!(truncated, 0, "word crossed the central cut at {cut_ms} ms");
    }
    for pair in words.windows(2) {
        let left = normalized_tokens(&pair[0].text);
        let right = normalized_tokens(&pair[1].text);
        if pair[0].end_ms > pair[1].start_ms && !left.is_empty() && left == right {
            panic!(
                "duplicate overlapping boundary word: {:?} / {:?}",
                pair[0], pair[1]
            );
        }
    }
}

#[test]
#[ignore = "runs same-candidate 60-minute reference and CPU VAD qualification"]
fn en_sixty_minute_reference_and_vad_use_same_candidate_and_boundary_policy() {
    let (runtime, paths) = runtime_paths();
    let source = path_var("PARAKEET_L2_EN_LONG_WAV");
    let reference = infer_reference(&paths, &source, "en60-reference");
    let (vad, windows, words) = infer_vad(&runtime, &paths, &source, "en60");
    verify_comparison(&reference, &vad, &windows, &words);
    let product = import_through_product_route(&runtime, &paths, &source, 60);
    verify_product_matches_vad_route(product, &vad, &windows);
}

#[test]
#[ignore = "runs long synthetic French reference and VAD; boundaries are artificial"]
fn fr_synthetic_long_reference_and_vad_use_same_candidate() {
    let (runtime, paths) = runtime_paths();
    let source = path_var("PARAKEET_L2_FR_LONG_WAV");
    let reference = infer_reference(&paths, &source, "fr-long-reference");
    let (vad, windows, words) = infer_vad(&runtime, &paths, &source, "fr-long");
    eprintln!(
        "L2B_FR_SOURCE synthetic=true duration_samples={}",
        vad.duration_samples
    );
    verify_comparison(&reference, &vad, &windows, &words);
    let product = import_through_product_route(&runtime, &paths, &source, 61);
    verify_product_matches_vad_route(product, &vad, &windows);
}

#[test]
#[ignore = "requires explicit B9 noise fixtures for -40, -30, and -20 dBFS"]
fn b9_reports_noise_floor_silence_counts_and_cut_plans() {
    let runtime = path_var("PARAKEET_L2_RUNTIME_ROOT");
    let cases = [
        ("-40", "PARAKEET_L2_NOISE_MINUS40_WAV"),
        ("-30", "PARAKEET_L2_NOISE_MINUS30_WAV"),
        ("-20", "PARAKEET_L2_NOISE_MINUS20_WAV"),
    ];
    for (level, variable) in cases {
        let source = path_var(variable);
        assert!(
            source.starts_with(&runtime),
            "noise fixture escaped L2b runtime"
        );
        let samples = read_pcm16_mono(&source);
        let silences = admissible_silences(&samples);
        match plan_windows(&samples) {
            Ok(windows) => eprintln!(
                "L2B_B9 level_dbfs={level} duration_samples={} admissible_silences={} cuts={} result=planned",
                samples.len(),
                silences.len(),
                windows.iter().filter(|window| window.cut.is_some()).count()
            ),
            Err(error) => eprintln!(
                "L2B_B9 level_dbfs={level} duration_samples={} admissible_silences={} cuts=0 result=error:{error}",
                samples.len(),
                silences.len()
            ),
        }
    }
}
