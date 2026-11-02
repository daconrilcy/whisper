use super::*;
use std::collections::BTreeMap;

const SCHEMA: &str = "voxpopuli-gpu-corpus-v1";
const SAMPLE_RATE: u64 = 16_000;

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("repository root must exist")
}

fn execution_and_corpus_roots() -> (PathBuf, PathBuf) {
    let execution_root = required_path("WHISPER_L04_CAMPAIGN_RUN");
    let corpus_root = required_path("WHISPER_VOXPOPULI_CORPUS_ROOT");
    let run_root = execution_root
        .parent()
        .and_then(Path::parent)
        .expect("campaign path must be RunRoot/writes/voxpopuli-campaign");
    let expected = run_root.join("writes").join("voxpopuli-corpus");
    assert_eq!(
        corpus_root, expected,
        "VoxPopuli fixtures must be staged beside the campaign output"
    );
    (execution_root, corpus_root)
}

fn hex_sha256(path: &Path) -> String {
    sha256_file(path)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn field<'a>(value: &'a serde_json::Value, name: &str) -> &'a str {
    value[name]
        .as_str()
        .unwrap_or_else(|| panic!("annotation field {name} must be a string"))
}

fn verify_wav(path: &Path, expected_samples: u64) {
    let metadata = std::fs::metadata(path).expect("stat staged WAV");
    assert!(metadata.len() >= 44);
    let mut wav = std::fs::File::open(path).expect("open staged WAV");
    let mut header = [0_u8; 44];
    wav.read_exact(&mut header).expect("read staged WAV header");
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
    assert_eq!(data_bytes % 2, 0);
    assert_eq!(u64::from(data_bytes) / 2, expected_samples);
    assert_eq!(metadata.len(), 44 + u64::from(data_bytes));
}

fn verify_source_provenance(annotation: &serde_json::Value, long_audio: bool) {
    let repo = repository_root();
    let archive = repo.join(field(annotation, "source_archive"));
    assert!(
        archive.is_file(),
        "pinned VoxPopuli source archive must exist"
    );
    assert_eq!(
        field(annotation, "source_archive_sha256"),
        "c018dbd445dee7d16a45238e8938a8d0671e09df0c0014e1fdf6f27ca4f48b5f"
    );

    if long_audio {
        let metadata = repo.join(field(annotation, "metadata_path"));
        assert!(metadata.is_file(), "pinned source annotations must exist");
        assert_eq!(hex_sha256(&metadata), field(annotation, "metadata_sha256"));

        let raw_root = required_path("WHISPER_VOXPOPULI_PROVENANCE_ROOT");
        let mut checked = BTreeMap::<PathBuf, String>::new();
        let segments = annotation["segments"]
            .as_array()
            .expect("long corpus annotation segments must be an array");
        let mut next_sample = 0_u64;
        for segment in segments {
            let start = segment["start_sample"]
                .as_u64()
                .expect("segment start sample");
            let end = segment["end_sample"].as_u64().expect("segment end sample");
            assert_eq!(
                start, next_sample,
                "long annotation ranges must be contiguous"
            );
            assert!(end > start);
            let source_path = Path::new(field(segment, "source_path"));
            assert!(!source_path.is_absolute());
            let relative = source_path
                .strip_prefix("target/n2-native/qualification-gpu/corpus-preparation/raw")
                .expect("source path must identify a file in the declared raw provenance tree");
            let raw = raw_root
                .join(relative)
                .canonicalize()
                .expect("original raw source file must exist");
            assert!(raw.starts_with(&raw_root));
            let expected_sha = field(segment, "source_sha256").to_owned();
            if let Some(previous) = checked.insert(raw.clone(), expected_sha.clone()) {
                assert_eq!(
                    previous, expected_sha,
                    "one source path cannot have two hashes"
                );
            }
            next_sample = end;
        }
        assert_eq!(
            next_sample,
            annotation["samples"].as_u64().expect("total sample count"),
            "provenance ranges must cover the complete generated WAV"
        );
        for (path, expected_sha) in checked {
            assert_eq!(
                hex_sha256(&path),
                expected_sha,
                "raw VoxPopuli source hash changed: {}",
                path.display()
            );
        }
    } else {
        let candidate = repo.join(field(annotation, "source_candidate"));
        assert!(
            candidate.is_file(),
            "user-confirmed source candidate must exist"
        );
        assert_eq!(
            hex_sha256(&candidate),
            field(annotation, "source_candidate_sha256")
        );
    }
}

fn validate_fixture(name: &str, language: &str, minimum_samples: u64) -> (PathBuf, u64) {
    let (execution_root, corpus_root) = execution_and_corpus_roots();
    let source = corpus_root
        .join(name)
        .canonicalize()
        .expect("staged VoxPopuli input");
    assert!(source.starts_with(&corpus_root));
    let annotation_path = corpus_root
        .join(
            name.rsplit_once('.')
                .expect("fixture extension")
                .0
                .to_owned()
                + ".annotations.json",
        )
        .canonicalize()
        .expect("paired corpus provenance annotation");
    assert!(annotation_path.starts_with(&corpus_root));
    let annotation: serde_json::Value = serde_json::from_slice(
        &std::fs::read(&annotation_path).expect("read staged provenance annotation"),
    )
    .expect("parse VoxPopuli corpus provenance");
    assert_eq!(field(&annotation, "schema"), SCHEMA);
    assert_eq!(field(&annotation, "language"), language);
    assert_eq!(
        field(&annotation, "reference_status"),
        "METADATA_TEXT_NOT_HUMAN_VERIFIED"
    );
    assert_eq!(annotation["sample_rate_hz"].as_u64(), Some(SAMPLE_RATE));
    assert_eq!(annotation["channels"].as_u64(), Some(1));
    assert_eq!(annotation["bits_per_sample"].as_u64(), Some(16));
    let samples = annotation["samples"]
        .as_u64()
        .expect("annotated sample count");
    assert!(samples >= minimum_samples);
    if source
        .extension()
        .is_some_and(|extension| extension == "wav")
    {
        assert_eq!(
            source.file_name(),
            Path::new(field(&annotation, "audio")).file_name(),
            "WAV is resolved by basename only from the staged corpus root"
        );
        verify_wav(&source, samples);
        assert_eq!(hex_sha256(&source), field(&annotation, "audio_sha256"));
    } else {
        let mp3 = &annotation["mp3"];
        assert_eq!(
            source.file_name().and_then(|name| name.to_str()),
            Path::new(field(mp3, "path"))
                .file_name()
                .and_then(|name| name.to_str()),
            "MP3 is resolved by basename only from the staged corpus root"
        );
        assert_eq!(hex_sha256(&source), field(mp3, "sha256"));
        assert_eq!(mp3["decoded"]["samples"].as_u64(), Some(samples));
        assert_eq!(mp3["decoded"]["sample_rate_hz"].as_u64(), Some(SAMPLE_RATE));
        assert_eq!(mp3["decoded"]["channels"].as_u64(), Some(1));
    }
    let reference = corpus_root
        .join(
            Path::new(field(&annotation, "reference"))
                .file_name()
                .unwrap(),
        )
        .canonicalize()
        .expect("staged reference text");
    assert!(reference.starts_with(&corpus_root));
    assert_eq!(
        hex_sha256(&reference),
        field(&annotation, "reference_sha256")
    );
    let reference_text = std::fs::read_to_string(reference).expect("read reference text");
    assert!(!reference_text.trim().is_empty());
    let reference_lines = reference_text.lines().collect::<Vec<_>>();
    let segments = annotation["segments"]
        .as_array()
        .expect("corpus segments must be an array");
    let annotated_lines = if minimum_samples > 60 * SAMPLE_RATE {
        segments
            .iter()
            .map(|segment| field(segment, "reference_text"))
            .collect::<Vec<_>>()
    } else {
        segments
            .iter()
            .flat_map(|segment| {
                segment["parts"]
                    .as_array()
                    .expect("short-fixture annotation parts must be an array")
            })
            .map(|part| field(part, "text"))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        reference_lines, annotated_lines,
        "reference text must be the recorded source annotation lines"
    );
    verify_source_provenance(&annotation, minimum_samples > 60 * SAMPLE_RATE);
    assert!(execution_root.is_dir());
    (source, samples)
}

fn fresh_personal_settings() {
    let local_app_data = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .expect("isolated LOCALAPPDATA is required")
        .canonicalize()
        .expect("isolated LOCALAPPDATA must exist");
    let settings = local_app_data.join("Whisper/settings");
    assert!(
        !settings.exists(),
        "personal runtime staging must not supply Whisper/settings"
    );
}

fn import_gpu(name: &str, language: &str, minimum_samples: u64, case: &str, auto: bool) {
    let (source, samples) = validate_fixture(name, language, minimum_samples);
    let imported = run_native_import_from_path(
        &source,
        case,
        if auto {
            ComputeChoice::Auto
        } else {
            ComputeChoice::Gpu
        },
        Some(personal_gpu_candidate()),
        (minimum_samples > 60 * SAMPLE_RATE).then_some(minimum_samples),
        true,
        auto,
        true,
    );
    assert_eq!(
        imported, samples,
        "published audio must preserve source duration"
    );
}

#[test]
#[cfg_attr(
    not(feature = "native-tests"),
    ignore = "requires the guarded VoxPopuli corpus, personal GPU runtime, and a current CUDA candidate"
)]
fn english_short_mp3_gpu() {
    fresh_personal_settings();
    import_gpu(
        "en-short-60s.mp3",
        "en",
        60 * SAMPLE_RATE,
        "voxpopuli-en-short-mp3-gpu",
        false,
    );
}

#[test]
#[cfg_attr(
    not(feature = "native-tests"),
    ignore = "requires the guarded VoxPopuli corpus, personal GPU runtime, and a current CUDA candidate"
)]
fn french_short_wav_and_mp3_gpu() {
    fresh_personal_settings();
    import_gpu(
        "fr-short-60s.wav",
        "fr",
        60 * SAMPLE_RATE,
        "voxpopuli-fr-short-wav-gpu",
        false,
    );
    import_gpu(
        "fr-short-60s.mp3",
        "fr",
        60 * SAMPLE_RATE,
        "voxpopuli-fr-short-mp3-gpu",
        false,
    );
}

#[test]
#[cfg_attr(
    not(feature = "native-tests"),
    ignore = "requires the guarded VoxPopuli corpus, raw provenance inputs, personal GPU runtime, and a current CUDA candidate"
)]
fn french_long_gpu() {
    fresh_personal_settings();
    import_gpu(
        "fr-long-35m.wav",
        "fr",
        35 * 60 * SAMPLE_RATE,
        "voxpopuli-fr-long-gpu",
        false,
    );
}

#[test]
#[cfg_attr(
    not(feature = "native-tests"),
    ignore = "requires the guarded VoxPopuli corpus, raw provenance inputs, personal GPU runtime, and a current CUDA candidate"
)]
fn english_long_gpu() {
    fresh_personal_settings();
    import_gpu(
        "en-long-40m.wav",
        "en",
        40 * 60 * SAMPLE_RATE,
        "voxpopuli-en-long-gpu",
        false,
    );
}

#[test]
#[cfg_attr(
    not(feature = "native-tests"),
    ignore = "requires the guarded VoxPopuli corpus, raw provenance inputs, personal GPU runtime, and a current CUDA candidate"
)]
fn french_long_auto_gpu() {
    fresh_personal_settings();
    import_gpu(
        "fr-long-35m.wav",
        "fr",
        35 * 60 * SAMPLE_RATE,
        "voxpopuli-fr-long-auto-gpu",
        true,
    );
}

#[test]
#[cfg_attr(
    not(feature = "native-tests"),
    ignore = "requires the guarded VoxPopuli corpus, raw provenance inputs, personal GPU runtime, and a current CUDA candidate"
)]
fn english_long_auto_gpu() {
    fresh_personal_settings();
    import_gpu(
        "en-long-40m.wav",
        "en",
        40 * 60 * SAMPLE_RATE,
        "voxpopuli-en-long-auto-gpu",
        true,
    );
}
