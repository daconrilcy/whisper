use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use sha2::{Digest, Sha256};
use whisper_adapters::{
    archive::ArchiveStore, parakeet_import::ParakeetWorkerPaths, supervisor::WorkerPaths,
    worker_ipc::AsyncImportIo,
};
use whisper_core::{
    AppCommand, Application, ComputeChoice, Generation, ImportApplication, ImportRequest,
    JobConfig, JobId, JobState, LanguageChoice,
    model_catalog::ModelId,
    model_registry::{PARAKEET_PERSONAL_GPU_PNP_INSTANCE_ID, model},
};

fn required_path(name: &str) -> PathBuf {
    let value = std::env::var_os(name).unwrap_or_else(|| {
        panic!("{name} must point to an input or root inside the guarded PU-L4 lot")
    });
    PathBuf::from(value)
        .canonicalize()
        .unwrap_or_else(|error| panic!("{name} is unavailable: {error}"))
}

fn fixture(name: &str, execution_root: &Path) -> PathBuf {
    let path = required_path(name);
    assert!(
        path.starts_with(execution_root),
        "native test fixtures must be copied under the guarded execution root"
    );
    path
}

fn sha256_file(path: &Path) -> [u8; 32] {
    let mut file = std::fs::File::open(path).expect("open input fixture for immutability proof");
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file
            .read(&mut buffer)
            .expect("read input fixture for immutability proof");
        if read == 0 {
            break;
        }
        digest.update(&buffer[..read]);
    }
    digest.finalize().into()
}

fn personal_paths() -> (
    WorkerPaths,
    PathBuf,
    ParakeetWorkerPaths,
    ParakeetWorkerPaths,
) {
    let local_app_data = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .expect("guarded PU-L4 LOCALAPPDATA is required")
        .canonicalize()
        .expect("guarded LOCALAPPDATA must exist");
    let whisper = local_app_data.join("Whisper");
    let app = whisper.join("app");
    let models = whisper.join("models");
    let worker = whisper.join("worker");
    let cpu = ParakeetWorkerPaths {
        executable: worker.join("whisper-worker-parakeet-cpu.exe"),
        native_dir: whisper.join("native/cpu"),
        model: models.join("parakeet-tdt-0.6b-v3.q8_0.gguf"),
        runtime_root: whisper.clone(),
    };
    let gpu = ParakeetWorkerPaths {
        executable: worker.join("whisper-worker-parakeet-gpu.exe"),
        native_dir: whisper.join("native/gpu"),
        model: models.join("parakeet-tdt-0.6b-v3.q8_0.gguf"),
        runtime_root: whisper,
    };
    for path in [
        &app.join("whisper-worker-cpu.exe"),
        &app.join("whisper-worker-gpu.exe"),
        &models.join("ggml-large-v3-turbo.bin"),
        &cpu.executable,
        &cpu.model,
        &gpu.executable,
    ] {
        assert!(
            path.is_file(),
            "required personal asset missing: {}",
            path.display()
        );
    }
    assert!(
        cpu.validate().is_ok(),
        "personal CPU route must match its exact layout"
    );
    assert!(
        gpu.validate().is_ok(),
        "personal GPU route must match its exact layout"
    );
    (
        WorkerPaths {
            cpu: app.join("whisper-worker-cpu.exe"),
            gpu: app.join("whisper-worker-gpu.exe"),
        },
        models.join("ggml-large-v3-turbo.bin"),
        cpu,
        gpu,
    )
}

fn request(
    source: &Path,
    model_path: &Path,
    destination: &Path,
    compute: ComputeChoice,
) -> ImportRequest {
    let record = model(ModelId::ParakeetTdt06BV3);
    let mut model_sha256 = [0; 32];
    for (index, byte) in model_sha256.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&record.artifact.sha256[index * 2..index * 2 + 2], 16)
            .expect("registered model SHA is hexadecimal");
    }
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock after epoch")
        .as_nanos();
    ImportRequest {
        strategy: whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1,
        job_id: JobId((u128::from(std::process::id()) << 64) | nonce),
        generation: Generation::first(),
        source_path: source.to_string_lossy().into_owned(),
        source_sha256: None,
        source_samples: None,
        model_path: model_path.to_string_lossy().into_owned(),
        model_sha256,
        destination: destination.to_string_lossy().into_owned(),
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute,
        },
    }
}

fn run_native_import(
    source_variable: &str,
    case: &str,
    compute: ComputeChoice,
    gpu_candidate: Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
    minimum_samples: Option<u64>,
    require_nonempty_text: bool,
) -> u64 {
    let execution_root = required_path("WHISPER_L04_CAMPAIGN_RUN");
    let source = fixture(source_variable, &execution_root);
    let before_sha = sha256_file(&source);
    let destination = execution_root.join("archive");
    std::fs::create_dir_all(&destination).expect("create archive under guarded PU-L4 root");
    let (workers, turbo_model, cpu, gpu) = personal_paths();
    let parakeet_model = cpu.model.clone();
    let io = AsyncImportIo::start_with_model_catalog_and_parakeet_routes_and_snapshot_registry_native_candidate(
        workers,
        turbo_model
            .parent()
            .expect("personal models folder has a parent")
            .to_path_buf(),
        turbo_model.to_string_lossy().into_owned(),
        Some(cpu),
        Some(gpu),
        Some(execution_root.join("settings")),
    );
    let mut app = ImportApplication::new(io);
    let request = request(&source, &parakeet_model, &destination, compute);
    assert_eq!(PathBuf::from(&request.model_path), parakeet_model);
    let has_gpu_candidate = gpu_candidate.is_some();
    if let Some(candidate) = gpu_candidate {
        assert_eq!(candidate.generation, request.generation.get());
        app.set_gpu_candidate(request.job_id, request.generation, Some(candidate))
            .expect("real current GPU candidate must be accepted by the application");
    }
    app.dispatch(AppCommand::StartImport {
        request: request.clone(),
    })
    .expect("start true ImportApplication route");

    let timeout = if minimum_samples.is_some() {
        Duration::from_secs(90 * 60)
    } else {
        Duration::from_secs(15 * 60)
    };
    let deadline = Instant::now() + timeout;
    let mut observed_backend = None;
    let terminal = loop {
        let view = app
            .dispatch(AppCommand::Refresh)
            .expect("refresh native unified import");
        if let Some(message) = view
            .message
            .as_deref()
            .filter(|message| message.starts_with("Moteur actif : "))
        {
            observed_backend = Some(message.to_owned());
        }
        if view.last_result.is_some()
            || view.active_job.is_some_and(|(job_id, state)| {
                job_id == request.job_id && state == JobState::Recoverable
            })
        {
            break view;
        }
        assert!(
            Instant::now() < deadline,
            "native import exceeded its bounded wait"
        );
        thread::sleep(Duration::from_millis(25));
    };
    let session = terminal
        .unified_parakeet_sessions
        .iter()
        .find(|session| {
            session.job_id == request.job_id && session.generation == request.generation
        })
        .expect("real ImportApplication must retain the completed session view");
    assert_eq!(
        session.phase,
        whisper_core::ports::UnifiedSessionPhase::Complete,
        "request must complete through the real unified publication path: {terminal:?}"
    );
    let backend = observed_backend.as_deref().unwrap_or_default();
    match compute {
        ComputeChoice::Cpu => assert!(backend.contains("CPU"), "effective backend: {backend}"),
        ComputeChoice::Gpu => assert!(backend.contains("CUDA0"), "effective backend: {backend}"),
        ComputeChoice::Auto if has_gpu_candidate => assert!(
            backend.contains("CUDA0") || backend.contains("CPU (Auto fallback before audio:"),
            "Auto with a current GPU candidate must report its actual backend: {backend}"
        ),
        ComputeChoice::Auto => assert!(
            backend.contains("CPU (Auto fallback before audio:"),
            "Auto without a candidate must report its actual pre-audio CPU fallback: {backend}"
        ),
    }

    let entry = ArchiveStore::scan(&destination)
        .expect("scan native import archive")
        .into_iter()
        .find(|entry| entry.job_id == request.job_id && entry.generation == request.generation)
        .expect("native request must leave a durable archive state");
    assert!(
        entry.complete,
        "{case} native request did not publish: {terminal:?}"
    );
    assert_eq!(
        entry.strategy,
        whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
    );
    let files =
        ArchiveStore::published_unified_files(&destination, request.job_id, request.generation)
            .expect("published audio, TXT and SRT must be referenced by the real archive");
    let transcript =
        std::fs::read_to_string(&files.txt_path).expect("read published transcript text");
    if require_nonempty_text {
        assert!(
            !transcript.trim().is_empty(),
            "native long transcription must publish nonempty text; lexical quality is evaluated against the supplied reference"
        );
    }
    let wav_path = PathBuf::from(&files.wav_path);
    let mut wav = std::fs::File::open(&wav_path).expect("open published WAV");
    let mut header = [0_u8; 44];
    wav.read_exact(&mut header)
        .expect("read published WAV header");
    assert_eq!(&header[0..4], b"RIFF");
    assert_eq!(&header[36..40], b"data");
    let wav_bytes = u32::from_le_bytes(header[40..44].try_into().unwrap());
    assert_eq!(wav_bytes % 2, 0, "published PCM16 byte count must be even");
    let published_samples = u64::from(wav_bytes / 2);
    for path in [files.wav_path, files.txt_path, files.srt_path] {
        let path = PathBuf::from(path);
        assert!(
            path.is_file(),
            "published artifact missing: {}",
            path.display()
        );
        assert!(std::fs::metadata(path).unwrap().len() > 0);
    }
    assert_eq!(
        sha256_file(&source),
        before_sha,
        "source fixture changed during native inference"
    );
    let part_store = whisper_adapters::parakeet_audio::PcmPartStore::open(&wav_path);
    let mut part_offset = 0_u64;
    let mut expected_part_number = 1_u64;
    let mut covered_samples = 0_u64;
    let mut part_count = 0_u64;
    let mut nonempty_part_count = 0_u64;
    let mut first_part_samples = None;
    loop {
        let page = part_store
            .read_page(part_offset, 128)
            .expect("read published logical WAV index through PcmPartStore");
        if page.is_empty() {
            break;
        }
        for part in &page {
            assert_eq!(part.number, expected_part_number);
            assert_eq!(
                part.start_sample, covered_samples,
                "published physical-part ranges must remain contiguous"
            );
            assert!(part.end_sample >= part.start_sample);
            let part_samples = part.end_sample - part.start_sample;
            if part.number == 1 {
                first_part_samples = Some(part_samples);
            }
            if part_samples > 0 {
                nonempty_part_count += 1;
            }
            let metadata = std::fs::metadata(&part.path).unwrap_or_else(|error| {
                panic!(
                    "published physical WAV part missing {}: {error}",
                    part.path.display()
                )
            });
            let mut part_file =
                std::fs::File::open(&part.path).expect("open indexed physical WAV part");
            let mut part_header = [0_u8; 44];
            part_file
                .read_exact(&mut part_header)
                .expect("read indexed physical WAV part header");
            assert_eq!(&part_header[0..4], b"RIFF");
            assert_eq!(&part_header[8..12], b"WAVE");
            assert_eq!(&part_header[36..40], b"data");
            assert_eq!(
                u16::from_le_bytes(part_header[20..22].try_into().unwrap()),
                1
            );
            assert_eq!(
                u16::from_le_bytes(part_header[22..24].try_into().unwrap()),
                1
            );
            assert_eq!(
                u32::from_le_bytes(part_header[24..28].try_into().unwrap()),
                16_000
            );
            assert_eq!(
                u16::from_le_bytes(part_header[34..36].try_into().unwrap()),
                16
            );
            let part_data_bytes = part_samples
                .checked_mul(2)
                .and_then(|bytes| u32::try_from(bytes).ok())
                .expect("physical WAV part length fits its RIFF data field");
            assert_eq!(
                u32::from_le_bytes(part_header[40..44].try_into().unwrap()),
                part_data_bytes,
                "physical WAV data length must agree with the indexed sample range"
            );
            assert_eq!(metadata.len(), 44 + u64::from(part_data_bytes));
            covered_samples = part.end_sample;
            expected_part_number += 1;
            part_count += 1;
        }
        part_offset = part_offset
            .checked_add(page.len() as u64)
            .expect("part index offset must not overflow");
    }
    assert!(
        part_count > 0,
        "published logical WAV must reference at least one part"
    );
    assert!(
        nonempty_part_count > 0,
        "published logical WAV must contain at least one physical part with samples"
    );
    assert_eq!(
        first_part_samples,
        Some(published_samples),
        "published WAV header must match the first physical index range"
    );
    let indexed_samples = part_store
        .total_samples()
        .expect("validate complete, contiguous physical-part index");
    assert_eq!(indexed_samples, covered_samples);
    assert_eq!(
        Some(indexed_samples),
        session.total_samples,
        "physical parts, published RIFF data and decoded source sample count must agree"
    );
    if let Some(minimum) = minimum_samples {
        assert!(
            indexed_samples >= minimum,
            "published logical WAV contains {indexed_samples} samples; expected at least {minimum}"
        );
    }
    indexed_samples
}

#[cfg(windows)]
#[derive(serde::Deserialize, serde::Serialize)]
struct LongCorpusAnnotation {
    schema: String,
    language: String,
    sample_rate_hz: u32,
    channels: u16,
    source_samples: u64,
    source_sha256: String,
    segments: Vec<LongCorpusSegment>,
}

#[cfg(windows)]
#[derive(serde::Deserialize, serde::Serialize)]
struct LongCorpusSegment {
    audio_id: u64,
    source_path: String,
    start_sample: u64,
    end_sample: u64,
    reference_text: String,
}

#[cfg(windows)]
fn run_native_long_case(language: &str, compute: ComputeChoice, minimum_samples: u64) {
    let execution_root = required_path("WHISPER_L04_CAMPAIGN_RUN");
    let (source_variable, source_name, annotation_name, corpus_name, tsv_name, corpus_language) =
        match language {
            "fr" => (
                "WHISPER_PU_L4_LONG_FR",
                "fr-long-35m.wav",
                "fr-long-35m.annotations.json",
                "sps-corpus-5.0-2026-09-11-fr",
                "ss-corpus-fr.tsv",
                "French",
            ),
            "en" => (
                "WHISPER_PU_L4_LONG_EN",
                "en-long-40m.wav",
                "en-long-40m.annotations.json",
                "sps-corpus-5.0-2026-09-11-en",
                "ss-corpus-en.tsv",
                "English",
            ),
            _ => panic!("only the planned French and English long fixtures are supported"),
        };
    let source = fixture(source_variable, &execution_root);
    let expected_source = execution_root.join("fixtures").join(source_name);
    assert_eq!(
        source,
        expected_source
            .canonicalize()
            .expect("planned long WAV fixture must exist")
    );
    let annotation_path = execution_root
        .join("fixtures")
        .join(annotation_name)
        .canonicalize()
        .expect("the paired long-session annotation must exist before native startup");
    assert!(
        annotation_path.starts_with(&execution_root),
        "long-session annotation must remain under the guarded lot"
    );
    let annotation: LongCorpusAnnotation = serde_json::from_slice(
        &std::fs::read(&annotation_path).expect("read paired long-session annotation"),
    )
    .expect("parse the versioned long-session annotation");
    assert_eq!(annotation.schema, "pu-l4-corpus-v1");
    assert_eq!(annotation.language, language);
    assert_eq!(annotation.sample_rate_hz, 16_000);
    assert_eq!(annotation.channels, 1);
    let repository_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
    let corpus_root = repository_root.join("sound example").join(corpus_name);
    let corpus_tsv = std::fs::read_to_string(corpus_root.join(tsv_name))
        .expect("read source corpus to verify annotation provenance");
    let mut approved_rows = std::collections::HashMap::new();
    for (row_number, line) in corpus_tsv.lines().skip(1).enumerate() {
        let columns = line.splitn(19, '\t').collect::<Vec<_>>();
        assert_eq!(
            columns.len(),
            19,
            "source corpus row {} is malformed",
            row_number + 2
        );
        let audio_id = columns[1].parse::<u64>().unwrap_or_else(|error| {
            panic!("invalid audio_id at corpus row {}: {error}", row_number + 2)
        });
        let votes = columns[7].parse::<u32>().unwrap_or_else(|error| {
            panic!(
                "invalid vote count at corpus row {}: {error}",
                row_number + 2
            )
        });
        let reference = columns[6].trim();
        if votes >= 1 && !reference.is_empty() {
            assert_eq!(columns[12], corpus_language);
            let source_path = PathBuf::from("sound example")
                .join(corpus_name)
                .join("audios")
                .join(columns[2])
                .to_string_lossy()
                .replace('\\', "/");
            assert!(
                approved_rows
                    .insert(audio_id, (source_path, reference.to_owned()))
                    .is_none(),
                "source corpus audio_id values must be unique"
            );
        }
    }

    let source_bytes = std::fs::metadata(&source)
        .expect("stat long-session WAV")
        .len();
    assert!(source_bytes >= 44);
    let mut wav = std::fs::File::open(&source).expect("open long-session WAV");
    let mut header = [0_u8; 44];
    std::io::Read::read_exact(&mut wav, &mut header).expect("read long-session WAV header");
    assert_eq!(&header[0..4], b"RIFF");
    assert_eq!(&header[8..12], b"WAVE");
    assert_eq!(&header[36..40], b"data");
    assert_eq!(u16::from_le_bytes(header[20..22].try_into().unwrap()), 1);
    assert_eq!(u16::from_le_bytes(header[22..24].try_into().unwrap()), 1);
    assert_eq!(
        u32::from_le_bytes(header[24..28].try_into().unwrap()),
        16_000
    );
    assert_eq!(u16::from_le_bytes(header[34..36].try_into().unwrap()), 16);
    let data_bytes = u32::from_le_bytes(header[40..44].try_into().unwrap());
    assert_eq!(source_bytes, 44 + u64::from(data_bytes));
    assert_eq!(data_bytes % 2, 0);
    let actual_samples = u64::from(data_bytes / 2);
    assert!(actual_samples >= minimum_samples);
    assert_eq!(annotation.source_samples, actual_samples);
    assert!(
        sha256_file(&source)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
            .eq_ignore_ascii_case(&annotation.source_sha256),
        "the source hash must match its annotation"
    );
    let mut next_sample = 0_u64;
    for segment in &annotation.segments {
        assert_eq!(
            segment.start_sample, next_sample,
            "annotation ranges must be contiguous"
        );
        assert!(segment.end_sample > segment.start_sample);
        assert!(segment.end_sample <= actual_samples);
        assert!(!segment.source_path.trim().is_empty());
        assert!(segment.source_path.starts_with("sound example/"));
        assert!(!segment.reference_text.trim().is_empty());
        let (expected_path, expected_reference) = approved_rows
            .get(&segment.audio_id)
            .expect("annotation audio_id must identify a voted corpus row");
        assert_eq!(&segment.source_path, expected_path);
        assert_eq!(&segment.reference_text, expected_reference);
        assert!(repository_root.join(expected_path.as_str()).is_file());
        next_sample = segment.end_sample;
    }
    assert_eq!(
        next_sample, actual_samples,
        "annotation must cover the entire WAV"
    );

    let gpu_candidate = match compute {
        ComputeChoice::Cpu => None,
        ComputeChoice::Gpu | ComputeChoice::Auto => Some(personal_gpu_candidate()),
    };
    let imported_samples = run_native_import(
        source_variable,
        &format!("native-{compute:?}-long-{language}"),
        compute,
        gpu_candidate,
        Some(minimum_samples),
        true,
    );
    assert_eq!(
        imported_samples, actual_samples,
        "published logical parts must cover the exact decoded long-session source"
    );
}

#[cfg(windows)]
fn build_long_fixture(language: &str, minimum_samples: u64) {
    let enabled = std::env::var("WHISPER_PU_L4_BUILD_LONG_FIXTURES").ok();
    assert_eq!(
        enabled.as_deref(),
        Some("1"),
        "the corpus fixture builder requires explicit opt-in"
    );
    let execution_root = required_path("WHISPER_L04_CAMPAIGN_RUN");
    let (corpus_name, tsv_name, expected_language, wav_name, annotation_name) = match language {
        "fr" => (
            "sps-corpus-5.0-2026-09-11-fr",
            "ss-corpus-fr.tsv",
            "French",
            "fr-long-35m.wav",
            "fr-long-35m.annotations.json",
        ),
        "en" => (
            "sps-corpus-5.0-2026-09-11-en",
            "ss-corpus-en.tsv",
            "English",
            "en-long-40m.wav",
            "en-long-40m.annotations.json",
        ),
        _ => panic!("only the planned French and English corpus fixtures are supported"),
    };
    let repository_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..");
    let corpus_root = repository_root.join("sound example").join(corpus_name);
    let tsv_path = corpus_root.join(tsv_name);
    let tsv = std::fs::read_to_string(&tsv_path)
        .unwrap_or_else(|error| panic!("read source corpus {}: {error}", tsv_path.display()));
    let fixtures = execution_root.join("fixtures");
    std::fs::create_dir_all(&fixtures).expect("create fixture directory under guarded lot");
    let wav_path = fixtures.join(wav_name);
    let annotation_path = fixtures.join(annotation_name);
    assert!(
        !wav_path.exists(),
        "refusing to replace an existing long-session WAV"
    );
    assert!(
        !annotation_path.exists(),
        "refusing to replace an existing long-session annotation"
    );
    let nonce = std::process::id();
    let staging_wav = fixtures.join(format!("{language}-long-build-{nonce}.wav"));
    let staging_index = staging_wav.with_extension("parts.idx");
    let staging_annotation =
        fixtures.join(format!("{language}-long-build-{nonce}.annotations.json"));
    let mut writer = whisper_adapters::parakeet_audio::PcmPartWriter::create(&staging_wav)
        .expect("create staged long-session WAV under guarded lot");
    let mut segments = Vec::new();
    let mut covered_samples = 0_u64;
    for (row_number, line) in tsv.lines().skip(1).enumerate() {
        let columns = line.splitn(19, '\t').collect::<Vec<_>>();
        assert_eq!(
            columns.len(),
            19,
            "source corpus row {} is malformed",
            row_number + 2
        );
        let audio_id = columns[1].parse::<u64>().unwrap_or_else(|error| {
            panic!("invalid audio_id at corpus row {}: {error}", row_number + 2)
        });
        let audio_file = columns[2];
        assert_eq!(
            Path::new(audio_file)
                .file_name()
                .and_then(|name| name.to_str()),
            Some(audio_file),
            "corpus audio filename must not escape its audios directory"
        );
        let transcription = columns[6].trim();
        let votes = columns[7].parse::<u32>().unwrap_or_else(|error| {
            panic!(
                "invalid vote count at corpus row {}: {error}",
                row_number + 2
            )
        });
        if votes < 1 || transcription.is_empty() {
            continue;
        }
        assert_eq!(
            columns[12], expected_language,
            "language mismatch in source corpus"
        );
        let source_relative = PathBuf::from("sound example")
            .join(corpus_name)
            .join("audios")
            .join(audio_file);
        let clip_path = repository_root.join(&source_relative);
        assert!(
            clip_path.is_file(),
            "approved corpus clip missing: {}",
            clip_path.display()
        );
        let start_sample = writer.written_samples();
        whisper_adapters::parakeet_audio::decode_file_to_parts(&clip_path, &mut writer)
            .unwrap_or_else(|error| {
                panic!(
                    "decode approved corpus clip {}: {error}",
                    clip_path.display()
                )
            });
        let end_sample = writer.written_samples();
        assert!(
            end_sample > start_sample,
            "approved corpus clip decoded to no samples"
        );
        segments.push(LongCorpusSegment {
            audio_id,
            source_path: source_relative.to_string_lossy().replace('\\', "/"),
            start_sample,
            end_sample,
            reference_text: transcription.to_owned(),
        });
        covered_samples = end_sample;
        if covered_samples >= minimum_samples {
            break;
        }
    }
    assert!(
        covered_samples >= minimum_samples,
        "voted corpus clips cover {covered_samples} samples; expected at least {minimum_samples}"
    );
    assert_eq!(
        writer.finish().expect("finish staged long-session WAV"),
        covered_samples
    );
    let annotation = LongCorpusAnnotation {
        schema: "pu-l4-corpus-v1".into(),
        language: language.into(),
        sample_rate_hz: 16_000,
        channels: 1,
        source_samples: covered_samples,
        source_sha256: sha256_file(&staging_wav)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
        segments,
    };
    std::fs::write(
        &staging_annotation,
        serde_json::to_vec_pretty(&annotation).expect("serialize staged corpus annotation"),
    )
    .expect("write staged annotation under guarded lot");
    std::fs::rename(&staging_wav, &wav_path).expect("publish built long-session WAV in lot");
    std::fs::rename(&staging_index, wav_path.with_extension("parts.idx"))
        .expect("publish built WAV index in lot");
    std::fs::rename(&staging_annotation, &annotation_path)
        .expect("publish built long-session annotation in lot");
}

#[test]
#[cfg(windows)]
#[ignore = "opt-in fixture builder; requires WHISPER_PU_L4_BUILD_LONG_FIXTURES=1 and a separately authorized guarded campaign"]
fn build_french_and_english_long_session_fixtures_from_voted_sps_clips() {
    let fixtures = required_path("WHISPER_L04_CAMPAIGN_RUN").join("fixtures");
    for name in [
        "fr-long-35m.wav",
        "fr-long-35m.annotations.json",
        "en-long-40m.wav",
        "en-long-40m.annotations.json",
    ] {
        assert!(
            !fixtures.join(name).exists(),
            "fixture builder refuses to replace existing output: {}",
            fixtures.join(name).display()
        );
    }
    build_long_fixture("fr", 35 * 60 * 16_000);
    build_long_fixture("en", 40 * 60 * 16_000);
}

#[cfg(windows)]
fn build_physical_rotation_fixture() {
    assert_eq!(
        std::env::var("WHISPER_PU_L4_BUILD_ROTATION_FIXTURE")
            .ok()
            .as_deref(),
        Some("1"),
        "physical-rotation fixture generation requires explicit opt-in"
    );
    let execution_root = required_path("WHISPER_L04_CAMPAIGN_RUN");
    let fixtures = execution_root.join("fixtures");
    std::fs::create_dir_all(&fixtures).expect("create fixture directory under guarded lot");
    let output = fixtures.join("physical-rotation-over-one-gib.wav");
    const BLOCK_BYTES: u64 = 80_000 * 2;
    const DATA_BYTES: u64 = (1_u64 << 30) + BLOCK_BYTES;
    assert!(DATA_BYTES <= u64::from(u32::MAX));
    assert!(
        !output.exists(),
        "refusing to replace the physical-rotation fixture"
    );
    let data_bytes = u32::try_from(DATA_BYTES).unwrap();
    let mut header = Vec::with_capacity(44);
    header.extend_from_slice(b"RIFF");
    header.extend_from_slice(&(36_u32 + data_bytes).to_le_bytes());
    header.extend_from_slice(b"WAVEfmt ");
    header.extend_from_slice(&16_u32.to_le_bytes());
    header.extend_from_slice(&1_u16.to_le_bytes());
    header.extend_from_slice(&1_u16.to_le_bytes());
    header.extend_from_slice(&16_000_u32.to_le_bytes());
    header.extend_from_slice(&32_000_u32.to_le_bytes());
    header.extend_from_slice(&2_u16.to_le_bytes());
    header.extend_from_slice(&16_u16.to_le_bytes());
    header.extend_from_slice(b"data");
    header.extend_from_slice(&data_bytes.to_le_bytes());
    assert_eq!(header.len(), 44);

    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)
        .expect("create physical-rotation fixture under guarded lot");
    file.write_all(&header)
        .expect("write physical-rotation WAV header");
    let block = vec![0_u8; BLOCK_BYTES as usize];
    let mut remaining = DATA_BYTES;
    while remaining > 0 {
        let count = remaining.min(BLOCK_BYTES) as usize;
        file.write_all(&block[..count])
            .expect("write bounded physical-rotation PCM block");
        remaining -= count as u64;
    }
    file.sync_all().expect("sync physical-rotation fixture");
    assert_eq!(
        std::fs::metadata(&output)
            .expect("stat physical-rotation fixture")
            .len(),
        44 + DATA_BYTES
    );
}

#[test]
#[cfg(windows)]
#[ignore = "opt-in builder for the >1 GiB controlled storage case; requires WHISPER_PU_L4_BUILD_ROTATION_FIXTURE=1 and a separately authorized guarded campaign"]
fn build_physical_rotation_fixture_over_one_gibibyte() {
    build_physical_rotation_fixture();
}

#[cfg(windows)]
fn personal_gpu_candidate() -> whisper_core::gpu_inventory::GpuRuntimeIdentity {
    use whisper_core::gpu_inventory_app::GpuInventoryPort;

    let (workers, _, _, _) = personal_paths();
    let mut port = whisper_adapters::gpu_inventory::WindowsGpuInventoryPort::new(workers.gpu);
    port.start(Generation::first().get())
        .expect("start real GPU runtime identity collection");
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if let Some((generation, result)) = port.poll_runtime_identities() {
            assert_eq!(generation, Generation::first().get());
            let identities = result.expect("GPU inventory worker must return runtime identities");
            return identities
                .into_iter()
                .find(|identity| identity.pnp_instance_id == PARAKEET_PERSONAL_GPU_PNP_INSTANCE_ID)
                .expect(
                    "the registered personal GPU must be observed by the real inventory worker",
                );
        }
        assert!(
            Instant::now() < deadline,
            "GPU runtime identity collection timed out"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
#[cfg(windows)]
#[ignore = "requires a separately authorized PU-L4 native CPU campaign and guarded copied FR fixture"]
fn native_personal_cpu_imports_french_mp3_through_the_unified_application() {
    run_native_import(
        "WHISPER_PU_L4_FR_SOURCE",
        "native-cpu-fr",
        ComputeChoice::Cpu,
        None,
        None,
        false,
    );
}

#[cfg(windows)]
#[test]
#[ignore = "requires a separately authorized PU-L4 native GPU campaign and guarded copied EN fixture"]
fn native_personal_gpu_imports_english_mp3_with_attested_candidate() {
    run_native_import(
        "WHISPER_PU_L4_EN_SOURCE",
        "native-gpu-en",
        ComputeChoice::Gpu,
        Some(personal_gpu_candidate()),
        None,
        false,
    );
}

#[test]
#[cfg(windows)]
#[ignore = "requires a separately authorized PU-L4 native Auto campaign and guarded copied EN fixture"]
fn native_personal_auto_without_candidate_reports_preaudio_cpu_fallback() {
    run_native_import(
        "WHISPER_PU_L4_EN_SOURCE",
        "native-auto-en",
        ComputeChoice::Auto,
        None,
        None,
        false,
    );
}

#[test]
#[cfg(windows)]
#[ignore = "requires the separately authorized PU-L4 campaign and paired guarded French long-session fixtures"]
fn native_personal_cpu_imports_annotated_french_long_audio() {
    run_native_long_case("fr", ComputeChoice::Cpu, 35 * 60 * 16_000);
}

#[test]
#[cfg(windows)]
#[ignore = "requires the separately authorized PU-L4 campaign and paired guarded English long-session fixtures"]
fn native_personal_cpu_imports_annotated_english_long_audio() {
    run_native_long_case("en", ComputeChoice::Cpu, 40 * 60 * 16_000);
}

#[test]
#[cfg(windows)]
#[ignore = "requires the separately authorized PU-L4 campaign, paired French fixtures, and selected GPU"]
fn native_personal_gpu_imports_annotated_french_long_audio() {
    run_native_long_case("fr", ComputeChoice::Gpu, 35 * 60 * 16_000);
}

#[test]
#[cfg(windows)]
#[ignore = "requires the separately authorized PU-L4 campaign, paired English fixtures, and selected GPU"]
fn native_personal_gpu_imports_annotated_english_long_audio() {
    run_native_long_case("en", ComputeChoice::Gpu, 40 * 60 * 16_000);
}

#[test]
#[cfg(windows)]
#[ignore = "requires the separately authorized PU-L4 campaign, paired French fixtures, and selected GPU candidate"]
fn native_personal_auto_imports_annotated_french_long_audio() {
    run_native_long_case("fr", ComputeChoice::Auto, 35 * 60 * 16_000);
}

#[test]
#[cfg(windows)]
#[ignore = "requires the separately authorized PU-L4 campaign, paired English fixtures, and selected GPU candidate"]
fn native_personal_auto_imports_annotated_english_long_audio() {
    run_native_long_case("en", ComputeChoice::Auto, 40 * 60 * 16_000);
}
