use super::*;

pub(super) fn validate_native_terminal(
    scenario: &NativeImportScenario<'_>,
    terminal: &whisper_core::AppView,
    observed_backend: Option<&str>,
    mut timing: Option<&mut native_import_timing::NativeImportTiming>,
) {
    if let Some(trace) = timing.as_deref_mut() {
        trace.event("terminal_view_assertions_begin", serde_json::Value::Null);
    }
    let session = terminal
        .unified_parakeet_sessions
        .iter()
        .find(|session| {
            session.job_id == scenario.request.job_id
                && session.generation == scenario.request.generation
        })
        .expect("real ImportApplication must retain the completed session view");
    assert_eq!(
        session.phase,
        whisper_core::ports::UnifiedSessionPhase::Complete,
        "request must complete through the real unified publication path: {terminal:?}"
    );
    let backend = observed_backend.unwrap_or_default();
    match scenario.options.compute {
        ComputeChoice::Cpu => assert!(backend.contains("CPU"), "effective backend: {backend}"),
        ComputeChoice::Gpu => assert!(backend.contains("CUDA0"), "effective backend: {backend}"),
        ComputeChoice::Auto
            if scenario.has_gpu_candidate && scenario.options.require_gpu_for_auto =>
        {
            assert!(
                backend.contains("CUDA0"),
                "this qualification requires an actual CUDA backend for Auto: {backend}"
            )
        }
        ComputeChoice::Auto if scenario.has_gpu_candidate => assert!(
            backend.contains("CUDA0") || backend.contains("CPU (Auto fallback before audio:"),
            "Auto with a current GPU candidate must report its actual backend: {backend}"
        ),
        ComputeChoice::Auto => assert!(
            backend.contains("CPU (Auto fallback before audio:"),
            "Auto without a candidate must report its actual pre-audio CPU fallback: {backend}"
        ),
    }
    if let Some(trace) = timing {
        trace.event("effective_backend_assertions_end", serde_json::Value::Null);
        trace.event("terminal_view_assertions_end", serde_json::Value::Null);
    }
}

pub(super) fn validate_native_publication(
    scenario: &NativeImportScenario<'_>,
    terminal: &whisper_core::AppView,
    mut timing: Option<&mut native_import_timing::NativeImportTiming>,
) -> NativeImportArtifacts {
    let entry_complete = validate_native_archive(scenario, terminal, timing.as_deref_mut());
    let exports = validate_native_exports(scenario, timing.as_deref_mut());
    validate_native_source_and_settings(scenario, timing);
    NativeImportArtifacts {
        wav_path: exports.wav_path,
        transcript_bytes: exports.transcript_bytes,
        entry_complete,
        published_samples: exports.published_samples,
    }
}

fn validate_native_archive(
    scenario: &NativeImportScenario<'_>,
    terminal: &whisper_core::AppView,
    mut timing: Option<&mut native_import_timing::NativeImportTiming>,
) -> bool {
    let archive_scan_started = Instant::now();
    if let Some(trace) = timing.as_deref_mut() {
        trace.event("archive_scan_begin", serde_json::Value::Null);
    }
    let entry = ArchiveStore::scan(&scenario.destination)
        .expect("scan native import archive")
        .into_iter()
        .find(|entry| {
            entry.job_id == scenario.request.job_id
                && entry.generation == scenario.request.generation
        })
        .expect("native request must leave a durable archive state");
    if let Some(trace) = timing {
        trace.event(
            "archive_scan_end",
            serde_json::json!({"duration_ms":archive_scan_started.elapsed().as_secs_f64()*1000.0}),
        );
    }
    assert!(
        entry.complete,
        "{} native request did not publish: {terminal:?}",
        scenario.case
    );
    assert_eq!(
        entry.strategy,
        whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
    );
    entry.complete
}

struct NativeImportExports {
    wav_path: PathBuf,
    transcript_bytes: usize,
    published_samples: u64,
}

fn validate_native_exports(
    scenario: &NativeImportScenario<'_>,
    mut timing: Option<&mut native_import_timing::NativeImportTiming>,
) -> NativeImportExports {
    let exports_started = Instant::now();
    if let Some(trace) = timing.as_deref_mut() {
        trace.event(
            "published_exports_resolution_begin",
            serde_json::Value::Null,
        );
    }
    let files = ArchiveStore::published_unified_files(
        &scenario.destination,
        scenario.request.job_id,
        scenario.request.generation,
    )
    .expect("published audio, TXT and SRT must be referenced by the real archive");
    if let Some(trace) = timing.as_deref_mut() {
        trace.event(
            "published_exports_resolution_end",
            serde_json::json!({"duration_ms":exports_started.elapsed().as_secs_f64()*1000.0}),
        );
    }
    let transcript_started = Instant::now();
    if let Some(trace) = timing.as_deref_mut() {
        trace.event("transcript_read_and_check_begin", serde_json::Value::Null);
    }
    let transcript =
        std::fs::read_to_string(&files.txt_path).expect("read published transcript text");
    if scenario.options.require_nonempty_text {
        assert!(
            !transcript.trim().is_empty(),
            "native long transcription must publish nonempty text; lexical quality is evaluated against the supplied reference"
        );
    }
    if let Some(trace) = timing.as_deref_mut() {
        trace.event("transcript_read_and_check_end", serde_json::json!({"duration_ms":transcript_started.elapsed().as_secs_f64()*1000.0,"bytes":transcript.len()}));
    }
    if let Some(trace) = timing.as_deref_mut() {
        trace.event("published_wav_read_begin", serde_json::Value::Null);
    }
    let wav_started = Instant::now();
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
    if let Some(trace) = timing.as_deref_mut() {
        trace.event("published_wav_read_end", serde_json::json!({"duration_ms":wav_started.elapsed().as_secs_f64()*1000.0,"wav_bytes":wav_bytes}));
    }
    let artifact_check_started = Instant::now();
    if let Some(trace) = timing.as_deref_mut() {
        trace.event("published_artifacts_check_begin", serde_json::Value::Null);
    }
    for path in [files.wav_path, files.txt_path, files.srt_path] {
        let path = PathBuf::from(path);
        assert!(
            path.is_file(),
            "published artifact missing: {}",
            path.display()
        );
        assert!(std::fs::metadata(path).unwrap().len() > 0);
    }
    if let Some(trace) = timing {
        trace.event("published_artifacts_check_end", serde_json::json!({"duration_ms":artifact_check_started.elapsed().as_secs_f64()*1000.0}));
    }
    NativeImportExports {
        wav_path,
        transcript_bytes: transcript.len(),
        published_samples,
    }
}

fn validate_native_source_and_settings(
    scenario: &NativeImportScenario<'_>,
    mut timing: Option<&mut native_import_timing::NativeImportTiming>,
) {
    let source_recheck_started = Instant::now();
    if let Some(trace) = timing.as_deref_mut() {
        trace.event("source_immutability_hash_begin", serde_json::Value::Null);
    }
    assert_eq!(
        sha256_file(scenario.source),
        scenario.before_sha,
        "source fixture changed during native inference"
    );
    if let Some(trace) = timing.as_mut() {
        trace.event("source_immutability_hash_end", serde_json::json!({"duration_ms":source_recheck_started.elapsed().as_secs_f64()*1000.0}));
    }
    if let Some(trace) = timing.as_deref_mut() {
        trace.event(
            "personal_settings_validation_begin",
            serde_json::Value::Null,
        );
    }
    if scenario.options.use_personal_settings {
        assert!(
            scenario.settings_root.is_dir(),
            "the product must create Whisper/settings in the personal runtime"
        );
        assert_gpu_log_created_for_import(
            &scenario.personal_gpu_logs,
            &scenario.previous_personal_gpu_logs,
        );
    }
    if let Some(trace) = timing {
        trace.event("personal_settings_validation_end", serde_json::Value::Null);
    }
}

pub(super) fn validate_native_parts(
    scenario: &NativeImportScenario<'_>,
    terminal: &whisper_core::AppView,
    artifacts: &NativeImportArtifacts,
    mut timing: Option<&mut native_import_timing::NativeImportTiming>,
) -> (u64, u64) {
    let part_store_started = Instant::now();
    if let Some(trace) = timing.as_deref_mut() {
        trace.event("published_part_store_open_begin", serde_json::Value::Null);
    }
    let part_store = whisper_adapters::parakeet_audio::PcmPartStore::open(&artifacts.wav_path);
    if let Some(trace) = timing.as_deref_mut() {
        trace.event(
            "published_part_store_open_end",
            serde_json::json!({"duration_ms":part_store_started.elapsed().as_secs_f64()*1000.0}),
        );
    }
    let (part_count, nonempty_part_count, first_part_samples, covered_samples) =
        walk_native_parts(&part_store, timing.as_deref_mut());
    let index_validation_started = Instant::now();
    if let Some(trace) = timing.as_deref_mut() {
        trace.event(
            "published_part_index_validation_begin",
            serde_json::Value::Null,
        );
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
        Some(artifacts.published_samples),
        "published WAV header must match the first physical index range"
    );
    let indexed_samples = part_store
        .total_samples()
        .expect("validate complete, contiguous physical-part index");
    assert_eq!(indexed_samples, covered_samples);
    let session = terminal
        .unified_parakeet_sessions
        .iter()
        .find(|session| {
            session.job_id == scenario.request.job_id
                && session.generation == scenario.request.generation
        })
        .expect("real ImportApplication must retain the completed session view");
    assert_eq!(
        Some(indexed_samples),
        session.total_samples,
        "physical parts, published RIFF data and decoded source sample count must agree"
    );
    if let Some(minimum) = scenario.options.minimum_samples {
        assert!(
            indexed_samples >= minimum,
            "published logical WAV contains {indexed_samples} samples; expected at least {minimum}"
        );
    }
    if let Some(trace) = timing {
        trace.event("published_part_index_validation_end", serde_json::json!({"duration_ms":index_validation_started.elapsed().as_secs_f64()*1000.0,"indexed_samples":indexed_samples}));
    }
    (indexed_samples, part_count)
}

fn walk_native_parts(
    part_store: &whisper_adapters::parakeet_audio::PcmPartStore,
    mut timing: Option<&mut native_import_timing::NativeImportTiming>,
) -> (u64, u64, Option<u64>, u64) {
    let mut part_offset = 0_u64;
    let mut expected_part_number = 1_u64;
    let mut covered_samples = 0_u64;
    let mut part_count = 0_u64;
    let mut nonempty_part_count = 0_u64;
    let mut first_part_samples = None;
    let index_walk_started = Instant::now();
    if let Some(trace) = timing.as_deref_mut() {
        trace.event("published_part_index_walk_begin", serde_json::Value::Null);
    }
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
    if let Some(trace) = timing {
        trace.event("published_part_index_walk_end", serde_json::json!({"duration_ms":index_walk_started.elapsed().as_secs_f64()*1000.0,"parts":part_count}));
    }
    (
        part_count,
        nonempty_part_count,
        first_part_samples,
        covered_samples,
    )
}
