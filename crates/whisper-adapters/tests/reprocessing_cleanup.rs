use sha2::{Digest, Sha256};
use std::{
    fs,
    hash::{Hash, Hasher},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use whisper_adapters::{
    archive::{ArchiveStore, sha256_file},
    model_catalog::LocalModelCatalog,
    source_binding::SourceBinding,
};
use whisper_core::{
    AppCommand, Application, ComputeChoice, Generation, ImportApplication, JobConfig, JobId,
    JobState, LanguageChoice, ports::DuplicateImportChoice, ports::ImportRequest,
};

fn root() -> PathBuf {
    let base = PathBuf::from(std::env::var_os("PARAKEET_RTC02_EXEC_ROOT").expect("RTC-02 root"));
    let path = base.join("archive").join(format!(
        "cleanup-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

fn wav(path: &Path) {
    let frames = 16_000_u32;
    let data_bytes = frames * 2;
    let mut file = fs::File::create(path).unwrap();
    file.write_all(b"RIFF").unwrap();
    file.write_all(&(36 + data_bytes).to_le_bytes()).unwrap();
    file.write_all(b"WAVEfmt ").unwrap();
    file.write_all(&16_u32.to_le_bytes()).unwrap();
    file.write_all(&1_u16.to_le_bytes()).unwrap();
    file.write_all(&1_u16.to_le_bytes()).unwrap();
    file.write_all(&16_000_u32.to_le_bytes()).unwrap();
    file.write_all(&32_000_u32.to_le_bytes()).unwrap();
    file.write_all(&2_u16.to_le_bytes()).unwrap();
    file.write_all(&16_u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&data_bytes.to_le_bytes()).unwrap();
    file.write_all(&vec![0_u8; data_bytes as usize]).unwrap();
}

fn request(destination: &Path, source: &Path, id: u128) -> ImportRequest {
    ImportRequest {
        job_id: JobId(id),
        generation: Generation::first(),
        source_path: source.to_string_lossy().into_owned(),
        source_sha256: None,
        source_samples: None,
        model_path: "model.bin".into(),
        model_sha256: [0; 32],
        destination: destination.to_string_lossy().into_owned(),
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        },
    }
}

fn sha_hex(sha: [u8; 32]) -> String {
    sha.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn snapshot_registry(root: &Path) -> PathBuf {
    root.join("settings/reprocessing-snapshots")
}

#[derive(Default)]
struct IdentityBytes(Vec<u8>);

impl Hasher for IdentityBytes {
    fn finish(&self) -> u64 {
        0
    }
    fn write(&mut self, bytes: &[u8]) {
        self.0.extend_from_slice(bytes);
    }
}

fn persisted_identity(path: &Path) -> String {
    let handle = same_file::Handle::from_path(path).unwrap();
    let mut bytes = IdentityBytes::default();
    handle.hash(&mut bytes);
    sha_hex(Sha256::digest(bytes.0).into())
}

fn add_owned_and_unowned_export_fixtures(destination: &Path, id: u128) {
    let generation = destination
        .join("transcriptions")
        .join(format!("{id:032x}"))
        .join("1");
    let index_path = destination.join(".whisper-publications/index.json");
    let mut index: serde_json::Value =
        serde_json::from_slice(&fs::read(&index_path).unwrap()).unwrap();
    let record = index["records"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|record| record["job_id"] == serde_json::json!(id))
        .unwrap();
    record["export_basename"] = serde_json::json!("voice-bench");
    fs::write(&index_path, serde_json::to_vec(&index).unwrap()).unwrap();

    let text = fs::read(generation.join("transcript.txt")).unwrap();
    let srt = fs::read(generation.join("transcript.srt")).unwrap();
    let exports = destination.join("exports");
    let moved = destination.join("transcriptions-remplacees");
    fs::create_dir_all(&exports).unwrap();
    fs::create_dir_all(&moved).unwrap();
    let owned_text = exports.join("voice-bench.txt");
    let third_party_srt = exports.join("voice-bench.srt");
    let moved_srt = moved.join(format!("voice-bench-{id:032x}-1.srt"));
    fs::write(&owned_text, &text).unwrap();
    fs::write(&third_party_srt, b"unrelated third-party subtitle").unwrap();
    fs::write(&moved_srt, &srt).unwrap();
    let record = index["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|record| record["job_id"] == serde_json::json!(id))
        .unwrap();
    let ownership = serde_json::json!({
        "version": 3,
        "job_id": id,
        "generation": 1,
        "source_sha256": record["source_sha256"],
        "export_basename": "voice-bench",
        "text_sha256": sha_hex(Sha256::digest(&text).into()),
        "srt_sha256": sha_hex(Sha256::digest(&srt).into()),
        "text_file_identity": persisted_identity(&owned_text),
        "srt_file_identity": persisted_identity(&moved_srt)
    });
    fs::write(
        generation.join("installed-exports.json"),
        serde_json::to_vec(&ownership).unwrap(),
    )
    .unwrap();
}

// Seed a fully verifiable, real archive generation without invoking an ASR
// engine. The fixture is private to target/rtc-02/archive.
fn seed_treatment(destination: &Path, source: &Path, id: u128) {
    let mut request = request(destination, source, id);
    let binding = SourceBinding::acquire(&request, &snapshot_registry(destination)).unwrap();
    request.source_sha256 = Some(binding.source_sha256());
    request.source_samples = Some(binding.source_samples());
    let source_sha = sha_hex(binding.source_sha256());
    ArchiveStore::reserve_create_only_new_import(destination, &request, &source_sha).unwrap();

    let generation = destination
        .join("transcriptions")
        .join(format!("{id:032x}"))
        .join("1");
    fs::create_dir_all(&generation).unwrap();
    let text = format!("fixture transcript {id}");
    let srt = format!("1\n00:00:00,000 --> 00:00:01,000\n{text}\n");
    fs::write(generation.join("transcript.txt"), &text).unwrap();
    fs::write(generation.join("transcript.srt"), &srt).unwrap();
    let model = whisper_core::model_registry::model(
        whisper_core::model_catalog::ModelId::WhisperLargeV3Turbo,
    );
    let pending = serde_json::json!({
        "version": 1,
        "job_id": id,
        "generation": 1,
        "source_path": request.source_path,
        "source_sha256": source_sha,
        "source_samples": binding.source_samples(),
        "model_path": "model.bin",
        "model_sha256": model.artifact.sha256,
        "model_size": model.artifact.size_bytes,
        "qualification": null
    });
    fs::write(
        generation.join("pending.json"),
        serde_json::to_vec(&pending).unwrap(),
    )
    .unwrap();
    let segment = serde_json::json!({
        "sequence": 1,
        "range": {"start_sample": 0, "end_sample": binding.source_samples(), "sample_rate_hz": 16_000},
        "text": text
    });
    fs::write(
        generation.join("segments.jsonl"),
        [serde_json::to_vec(&segment).unwrap(), b"\n".to_vec()].concat(),
    )
    .unwrap();
    let artifact = |relative: &str, contents: &str| {
        serde_json::json!({
            "path": relative,
            "sha256": sha_hex(Sha256::digest(contents.as_bytes()).into()),
            "bytes": contents.len()
        })
    };
    let manifest = serde_json::json!({
        "version": 1,
        "job_id": id,
        "generation": 1,
        "source_sha256": source_sha,
        "segments": 1,
        "text": artifact("transcript.txt", &text),
        "srt": artifact("transcript.srt", &srt)
    });
    fs::write(
        generation.join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    fs::write(generation.join("CURRENT"), b"manifest.json\n").unwrap();
    ArchiveStore::resolve_import_publication(destination, JobId(id), Generation::first()).unwrap();
    binding.retire().unwrap();
}

fn compile_worker(root: &Path) -> PathBuf {
    let source = root.join("rtc02-fake-worker.rs");
    let binary = root.join("rtc02-fake-worker.exe");
    let code = r#"
use std::io::{self, BufRead, Write};
fn number(line: &str, field: &str) -> String {
    let marker = format!("\"{field}\":");
    line.split_once(&marker).expect("IPC field").1.trim_start()
        .chars().take_while(|c| c.is_ascii_digit()).collect()
}
fn emit(body: String) {
    println!("{{\"version\":2,\"message\":{{{body}}}}}");
    io::stdout().flush().unwrap();
}
fn main() {
    let line = io::stdin().lock().lines().next().unwrap().unwrap();
    let job = number(&line, "job_id");
    let generation = number(&line, "generation");
    let instance = number(&line, "instance_id");
    let samples = number(&line, "expected_source_samples");
    emit(format!("\"Ready\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"backend\":\"Cpu\"}}"));
    emit(format!("\"Segment\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"segment_id\":1,\"range\":{{\"start_sample\":0,\"end_sample\":{samples},\"sample_rate_hz\":16000}},\"text\":\"replacement transcript\"}}"));
    emit(format!("\"Progress\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"completed_samples\":{samples},\"total_samples\":{samples}}}"));
    emit(format!("\"End\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"last_sequence\":1,\"last_offset\":{samples}}}"));
}
"#;
    fs::write(&source, code).unwrap();
    let output = Command::new("rustc")
        .args(["--edition=2021", "--crate-name", "rtc02_fake_worker"])
        .arg(&source)
        .arg("-o")
        .arg(&binary)
        .output()
        .expect("compile fake worker");
    assert!(
        output.status.success(),
        "fake worker: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    binary
}

#[test]
fn replace_all_publishes_first_stops_partial_cleanup_and_resumes_only_on_request() {
    let root = root();
    let destination = root.join("destination");
    fs::create_dir_all(&destination).unwrap();
    let source = root.join("audio.wav");
    wav(&source);
    seed_treatment(&destination, &source, 101);
    seed_treatment(&destination, &source, 102);
    seed_treatment(&destination, &source, 103);
    add_owned_and_unowned_export_fixtures(&destination, 102);

    let source_sha = sha_hex(sha256_file(&source).unwrap());
    let initial_catalog = ArchiveStore::reprocessing_catalog(&destination, &source_sha).unwrap();
    let publication_index = destination.join(".whisper-publications/index.json");
    let prepublication_index = fs::read(&publication_index).unwrap();
    assert_eq!(initial_catalog.len(), 3);
    assert_eq!(
        initial_catalog
            .iter()
            .map(|item| item.success_ordinal)
            .collect::<Vec<_>>(),
        vec![Some(1), Some(2), Some(3)]
    );
    assert!(
        initial_catalog
            .iter()
            .all(|item| item.active && item.verified_present)
    );

    let latest_generation = destination.join("transcriptions/00000000000000000000000000000066/1");
    let latest_txt = latest_generation.join("transcript.txt");
    let latest_srt = latest_generation.join("transcript.srt");
    let older_generation = destination.join("transcriptions/00000000000000000000000000000065/1");
    let third_generation = destination.join("transcriptions/00000000000000000000000000000067/1");
    let unknown_generation_file = latest_generation.join("user-note.txt");
    fs::write(&unknown_generation_file, b"keep this unrelated file").unwrap();

    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let model_path = repository.join("models/ggml-large-v3-turbo.bin");
    let catalog = LocalModelCatalog::new(repository.join("models")).inspect(1);
    let model = catalog
        .entries
        .iter()
        .find(|entry| entry.id == whisper_core::model_catalog::ModelId::WhisperLargeV3Turbo)
        .and_then(|entry| entry.observed_sha256.as_ref())
        .expect("local Turbo model");
    let model_sha: [u8; 32] = model
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();
    let worker = compile_worker(&root);
    let io = whisper_adapters::worker_ipc::AsyncImportIo::start_with_model_catalog_and_parakeet_routes_and_snapshot_registry(
        whisper_adapters::supervisor::WorkerPaths { cpu: worker.clone(), gpu: worker },
        repository.join("models"),
        fs::canonicalize(&model_path).unwrap().to_string_lossy().into_owned(),
        None,
        None,
        Some(snapshot_registry(&root)),
    );
    let mut app = ImportApplication::new(io);
    let mut incoming = request(&destination, &source, 104);
    incoming.model_path = fs::canonicalize(&model_path)
        .unwrap()
        .to_string_lossy()
        .into_owned();
    incoming.model_sha256 = model_sha;
    app.dispatch(AppCommand::EnqueueImport { request: incoming })
        .unwrap();

    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    let prompt = loop {
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        if let Some(prompt) = view.duplicate_import_prompt {
            break prompt;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "duplicate prompt missing: {view:?}"
        );
        thread::sleep(Duration::from_millis(10));
    };
    assert_eq!(prompt.publications.len(), 3);
    assert_eq!(
        prompt
            .publications
            .iter()
            .map(|item| item.success_ordinal)
            .collect::<Vec<_>>(),
        vec![Some(1), Some(2), Some(3)]
    );
    app.dispatch(AppCommand::ResolveDuplicateImport {
        prompt,
        choice: DuplicateImportChoice::ReplaceAll,
    })
    .unwrap();
    fs::remove_file(&unknown_generation_file).unwrap();
    let late_unknown_generation_file = latest_generation.join("added-after-confirmation.txt");

    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    let queued = loop {
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        if let Some(entry) = view.queue.iter().find(|entry| {
            entry.request.job_id != JobId(101)
                && entry.request.job_id != JobId(102)
                && entry.request.job_id != JobId(103)
        }) {
            break entry.request.clone();
        }
        assert!(
            std::time::Instant::now() < deadline,
            "replacement not queued: {view:?}"
        );
        thread::sleep(Duration::from_millis(10));
    };

    // A changed original must block every deletion. After it is restored, an
    // unknown child forces a durable partial cleanup and explicit resume.
    let original_source = fs::read(&source).unwrap();
    let mut changed_source = original_source.clone();
    *changed_source.last_mut().unwrap() ^= 1;
    fs::write(&source, &changed_source).unwrap();
    app.dispatch(AppCommand::ProcessQueued {
        request: queued.clone(),
    })
    .unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    let published = loop {
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        if view.active_job == Some((queued.job_id, JobState::Complete)) {
            break view;
        }
        assert_ne!(
            view.active_job,
            Some((queued.job_id, JobState::Recoverable)),
            "replacement failed: {:?}",
            view.message
        );
        assert!(
            std::time::Instant::now() < deadline,
            "replacement timed out: {view:?}"
        );
        thread::sleep(Duration::from_millis(10));
    };
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    let cleanup = loop {
        let cleanups = ArchiveStore::replacement_cleanups(&destination).unwrap();
        if let Some(cleanup) = cleanups
            .into_iter()
            .find(|cleanup| !cleanup.remaining.is_empty())
        {
            break cleanup;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "replacement cleanup state not persisted: {published:?}"
        );
        thread::sleep(Duration::from_millis(10));
    };
    assert!(
        latest_txt.exists() && latest_srt.exists(),
        "source mismatch must stop before deleting any old artifact"
    );
    assert!(
        cleanup
            .last_error
            .as_deref()
            .is_some_and(|error| error.contains("SourceChanged"))
    );
    assert!(older_generation.join("transcript.txt").is_file());
    assert!(third_generation.join("transcript.txt").is_file());
    assert!(destination.join("exports/voice-bench.txt").is_file());
    assert!(destination.join("exports/voice-bench.srt").is_file());
    assert!(
        destination
            .join("transcriptions")
            .join(format!("{:032x}", queued.job_id.0))
            .join("1/transcript.txt")
            .is_file(),
        "successor publishes before cleanup"
    );

    // Recreate the durable state at the crash boundary after CURRENT and the
    // publication index were written, but before the plan's published proofs
    // were refreshed. Discovery must replay the publication intent idempotently
    // and must not start deleting old generations as part of recovery.
    let plan_path = destination
        .join(".whisper-publications/replacement-cleanups")
        .join(format!("{}.json", cleanup.cleanup_id));
    let completed_plan = fs::read(&plan_path).unwrap();
    let mut interrupted_plan: serde_json::Value = serde_json::from_slice(&completed_plan).unwrap();
    let prepublication_files =
        interrupted_plan["successor_files"].as_array().unwrap()[..5].to_vec();
    interrupted_plan["successor_files"] = serde_json::Value::Array(prepublication_files);
    interrupted_plan["successor_index"] = serde_json::Value::Null;
    interrupted_plan["published"] = serde_json::Value::Bool(false);
    fs::write(&plan_path, serde_json::to_vec(&interrupted_plan).unwrap()).unwrap();

    // Reconstruct a durable reservation after the plan was written but before
    // CURRENT. Discovery must leave old targets and the prior index intact.
    let successor_current = destination
        .join("transcriptions")
        .join(format!("{:032x}", queued.job_id.0))
        .join("1/CURRENT");
    let held_current = successor_current.with_extension("rtc02-held");
    fs::rename(&successor_current, &held_current).unwrap();
    fs::write(&publication_index, &prepublication_index).unwrap();
    let before_current = ArchiveStore::replacement_cleanups(&destination).unwrap();
    assert!(
        before_current
            .iter()
            .all(|item| item.cleanup_id != cleanup.cleanup_id)
    );
    assert_eq!(fs::read(&publication_index).unwrap(), prepublication_index);
    assert!(latest_txt.is_file() && older_generation.join("transcript.txt").is_file());
    assert!(third_generation.join("transcript.txt").is_file());

    // Reconstruct after CURRENT but before its publication-index update. The
    // next discovery must replay the index row idempotently and not clean old
    // targets until the user explicitly resumes.
    fs::rename(&held_current, &successor_current).unwrap();

    let recovered = ArchiveStore::replacement_cleanups(&destination).unwrap();
    assert!(
        recovered
            .iter()
            .any(|item| item.cleanup_id == cleanup.cleanup_id)
    );
    assert!(latest_txt.is_file() && latest_srt.is_file());
    assert!(older_generation.join("transcript.txt").is_file());
    assert!(third_generation.join("transcript.txt").is_file());
    let recovered_index = fs::read(destination.join(".whisper-publications/index.json")).unwrap();
    let recovered_catalog = ArchiveStore::reprocessing_catalog(&destination, &source_sha).unwrap();
    assert_eq!(
        recovered_catalog
            .iter()
            .filter(|item| item.job_id == queued.job_id && item.active)
            .count(),
        1,
        "recovery must leave exactly one active successor"
    );
    let recovered_plan: serde_json::Value =
        serde_json::from_slice(&fs::read(&plan_path).unwrap()).unwrap();
    assert_eq!(recovered_plan["published"], serde_json::Value::Bool(true));
    assert_eq!(
        recovered_plan["successor_files"].as_array().unwrap().len(),
        6
    );
    assert!(recovered_plan["successor_index"].is_object());
    let index_after_first_recovery =
        fs::read(destination.join(".whisper-publications/index.json")).unwrap();
    assert_eq!(recovered_index, index_after_first_recovery);
    // Re-running discovery after the recovery is idempotent and does not make
    // a second successor or begin deletion.
    let _ = ArchiveStore::replacement_cleanups(&destination).unwrap();
    let index_after_second_recovery =
        fs::read(destination.join(".whisper-publications/index.json")).unwrap();
    assert_eq!(index_after_first_recovery, index_after_second_recovery);
    assert!(latest_txt.is_file() && older_generation.join("transcript.txt").is_file());
    let recovery_plan_before_cleanup = fs::read(&plan_path).unwrap();

    fs::write(&source, &original_source).unwrap();

    // A replacement export parent must be rejected before any old artifact is
    // disposed. The test uses a same-root directory rename/substitution so no
    // user data or external path is touched.
    let exports = destination.join("exports");
    let moved_exports = destination.join("exports-saved");
    fs::rename(&exports, &moved_exports).unwrap();
    let junction = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &format!(
                "New-Item -ItemType Junction -Path '{}' -Target '{}' | Out-Null",
                exports.display(),
                moved_exports.display()
            ),
        ])
        .output()
        .expect("create same-root export junction");
    assert!(
        junction.status.success(),
        "create export junction: {} {}",
        String::from_utf8_lossy(&junction.stdout),
        String::from_utf8_lossy(&junction.stderr)
    );
    assert!(ArchiveStore::resume_replacement_cleanup(&destination, &cleanup.cleanup_id).is_err());
    assert!(latest_txt.is_file() && latest_srt.is_file());
    assert!(moved_exports.join("voice-bench.txt").is_file());
    assert!(moved_exports.join("voice-bench.srt").is_file());
    fs::remove_dir(&exports).unwrap();
    fs::rename(&moved_exports, &exports).unwrap();

    // Tampering with the durable plan to alias a cleanup target to a successor
    // object must be rejected before deleting any old generation.
    let plan_path = destination
        .join(".whisper-publications/replacement-cleanups")
        .join(format!("{}.json", cleanup.cleanup_id));
    let original_plan = fs::read(&plan_path).unwrap();
    let mut plan: serde_json::Value = serde_json::from_slice(&original_plan).unwrap();
    let successor_identity = plan["successor_files"][0]["identity"].clone();
    plan["targets"][0]["objects"][0]["identity"] = successor_identity;
    fs::write(&plan_path, serde_json::to_vec(&plan).unwrap()).unwrap();
    assert!(ArchiveStore::resume_replacement_cleanup(&destination, &cleanup.cleanup_id).is_err());
    assert!(latest_txt.is_file() && latest_srt.is_file());
    assert!(older_generation.join("transcript.txt").is_file());
    assert!(third_generation.join("transcript.txt").is_file());
    fs::write(&plan_path, original_plan).unwrap();

    // Corrupt each successor proof independently. Every refusal must leave all
    // old treatment targets available; restore bytes before the next case.
    let successor = destination
        .join("transcriptions")
        .join(format!("{:032x}", queued.job_id.0))
        .join("1");
    for path in [
        successor.join("CURRENT"),
        successor.join("manifest.json"),
        successor.join("transcript.txt"),
        successor.join("transcript.srt"),
        successor.join("pending.json"),
        successor.join("segments.jsonl"),
        publication_index,
    ] {
        let original = fs::read(&path).unwrap();
        fs::write(&path, b"corrupted successor proof").unwrap();
        assert!(
            ArchiveStore::resume_replacement_cleanup(&destination, &cleanup.cleanup_id).is_err()
        );
        assert!(latest_txt.is_file() && latest_srt.is_file());
        assert!(older_generation.join("transcript.txt").is_file());
        assert!(third_generation.join("transcript.txt").is_file());
        fs::write(path, original).unwrap();
    }
    // Missing pending/segment proof is independently rejected before any
    // disposition; renaming preserves the original file identity for restore.
    for proof_path in [
        successor.join("pending.json"),
        successor.join("segments.jsonl"),
    ] {
        let held_path = proof_path.with_extension("rtc02-held");
        fs::rename(&proof_path, &held_path).unwrap();
        assert!(
            ArchiveStore::resume_replacement_cleanup(&destination, &cleanup.cleanup_id).is_err(),
            "missing proof must prevent deletion: {}",
            proof_path.display()
        );
        assert!(latest_txt.is_file() && latest_srt.is_file());
        assert!(older_generation.join("transcript.txt").is_file());
        assert!(third_generation.join("transcript.txt").is_file());
        fs::rename(&held_path, &proof_path).unwrap();
    }
    // The cleanup reservation is already durable and the publication intent
    // has fixed its exact targets. A later unknown file must be reported and
    // preserved, without changing the confirmed target sequence.
    fs::write(
        &late_unknown_generation_file,
        b"keep this later unrelated file",
    )
    .unwrap();
    app.dispatch(AppCommand::ResumeReplacementCleanup {
        destination: destination.to_string_lossy().into_owned(),
        cleanup_id: cleanup.cleanup_id.clone(),
    })
    .unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    let partial = loop {
        let state = ArchiveStore::replacement_cleanups(&destination).unwrap();
        if let Some(item) = state
            .iter()
            .find(|item| item.cleanup_id == cleanup.cleanup_id)
        {
            if !latest_txt.exists()
                && !item.remaining.is_empty()
                && item.remaining.iter().any(|path| path.ends_with("\\1"))
            {
                break item.clone();
            }
        }
        assert!(
            std::time::Instant::now() < deadline,
            "partial cleanup was not persisted: {state:?}"
        );
        thread::sleep(Duration::from_millis(10));
    };
    assert!(!latest_txt.exists() && !latest_srt.exists());
    assert!(
        !older_generation.exists(),
        "the first selected target is removed"
    );
    assert!(
        third_generation.join("transcript.txt").is_file(),
        "cleanup stops before the third selected target"
    );
    assert_eq!(
        fs::read(&late_unknown_generation_file).unwrap(),
        b"keep this later unrelated file"
    );
    assert!(
        partial
            .preserved
            .iter()
            .any(|path| path.contains("added-after-confirmation.txt"))
    );

    fs::remove_file(&late_unknown_generation_file).unwrap();
    // Simulate a crash after disposition but before the matching progress
    // write: the old generation is absent while its durable object bit is
    // stale. Replaying must observe absence and continue without a second
    // successor or a newly added target.
    let mut stale_progress: serde_json::Value =
        serde_json::from_slice(&fs::read(&plan_path).unwrap()).unwrap();
    let older_target = stale_progress["targets"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|target| target["treatment"]["job_id"] == serde_json::json!(101))
        .expect("first confirmed treatment remains in durable target list");
    let stale_object = older_target["objects"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|object| {
            object["relative_path"]
                .as_str()
                .is_some_and(|path| path.ends_with("\\transcript.txt"))
        })
        .expect("reserved old text remains in durable target list");
    stale_object["removed"] = serde_json::Value::Bool(false);
    stale_object["error"] = serde_json::Value::Null;
    fs::write(&plan_path, serde_json::to_vec(&stale_progress).unwrap()).unwrap();
    app.dispatch(AppCommand::ResumeReplacementCleanup {
        destination: destination.to_string_lossy().into_owned(),
        cleanup_id: cleanup.cleanup_id.clone(),
    })
    .unwrap();
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    loop {
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        if view
            .replacement_cleanups
            .iter()
            .find(|item| item.cleanup_id == cleanup.cleanup_id)
            .is_some_and(|item| item.remaining.is_empty())
        {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "cleanup resume timed out: {view:?}"
        );
        thread::sleep(Duration::from_millis(10));
    }
    assert!(!latest_generation.exists());
    assert!(!third_generation.exists());
    assert!(!destination.join("exports/voice-bench.txt").exists());
    assert!(
        destination.join("exports/voice-bench.srt").is_file(),
        "third-party homonym must be preserved"
    );
    assert!(
        !destination
            .join("transcriptions-remplacees/voice-bench-00000000000000000000000000000066-1.srt")
            .exists(),
        "owned moved SRT must be removed"
    );
    let durable = ArchiveStore::replacement_cleanups(&destination).unwrap();
    let durable_cleanup = durable
        .iter()
        .find(|item| item.cleanup_id == cleanup.cleanup_id)
        .expect("selected treatment cleanup remains visible");
    assert!(durable_cleanup.remaining.is_empty());
    assert_eq!(durable_cleanup.preserved.len(), 1);
    assert!(durable_cleanup.preserved[0].contains("exports\\voice-bench.srt"));
    let final_catalog = ArchiveStore::reprocessing_catalog(&destination, &source_sha).unwrap();
    assert!(
        final_catalog
            .iter()
            .any(|item| item.job_id == JobId(102) && !item.active && !item.verified_present)
    );
    assert!(
        final_catalog
            .iter()
            .filter(|item| item.job_id.0 <= 103)
            .all(|item| { !item.active && !item.verified_present })
    );
    assert_eq!(
        final_catalog
            .iter()
            .filter(|item| item.active && item.verified_present)
            .count(),
        1
    );

    // Concurrently publish a different treatment while cleanup discovery is
    // attempting to replay the successor intent. Both operations share the
    // publication lock; recovery must observe and preserve the concurrent row.
    {
        let _lock = ArchiveStore::lock_publications(&destination).unwrap();
        let mut interrupted_plan: serde_json::Value =
            serde_json::from_slice(&recovery_plan_before_cleanup).unwrap();
        interrupted_plan["successor_files"] = serde_json::Value::Array(
            interrupted_plan["successor_files"].as_array().unwrap()[..5].to_vec(),
        );
        interrupted_plan["successor_index"] = serde_json::Value::Null;
        interrupted_plan["published"] = serde_json::Value::Bool(false);
        fs::write(&plan_path, serde_json::to_vec(&interrupted_plan).unwrap()).unwrap();

        let index_path = destination.join(".whisper-publications/index.json");
        let mut index: serde_json::Value =
            serde_json::from_slice(&fs::read(&index_path).unwrap()).unwrap();
        index["records"]
            .as_array_mut()
            .unwrap()
            .retain(|record| record["job_id"] != serde_json::json!(queued.job_id.0));
        fs::write(&index_path, serde_json::to_vec(&index).unwrap()).unwrap();
    }

    let publication_lock = ArchiveStore::lock_publications(&destination).unwrap();
    let (recovery_started_tx, recovery_started_rx) = std::sync::mpsc::channel();
    let (recovery_done_tx, recovery_done_rx) = std::sync::mpsc::channel();
    let recovery_root = destination.clone();
    let recovery = thread::spawn(move || {
        recovery_started_tx.send(()).unwrap();
        recovery_done_tx
            .send(ArchiveStore::replacement_cleanups(&recovery_root))
            .unwrap();
    });
    recovery_started_rx
        .recv_timeout(Duration::from_secs(2))
        .unwrap();

    let (publisher_started_tx, publisher_started_rx) = std::sync::mpsc::channel();
    let publisher_root = destination.clone();
    let publisher_source = source.clone();
    let publisher = thread::spawn(move || {
        publisher_started_tx.send(()).unwrap();
        seed_treatment(&publisher_root, &publisher_source, 105);
    });
    publisher_started_rx
        .recv_timeout(Duration::from_secs(2))
        .unwrap();
    thread::sleep(Duration::from_millis(100));
    assert!(
        recovery_done_rx.try_recv().is_err(),
        "cleanup recovery must wait for the publication lock"
    );
    drop(publication_lock);
    recovery_done_rx
        .recv_timeout(Duration::from_secs(10))
        .expect("cleanup recovery resumes after publication lock release")
        .unwrap();
    recovery.join().unwrap();
    publisher.join().unwrap();

    let concurrent_index: serde_json::Value = serde_json::from_slice(
        &fs::read(destination.join(".whisper-publications/index.json")).unwrap(),
    )
    .unwrap();
    for published_id in [queued.job_id.0, 105] {
        assert!(
            concurrent_index["records"]
                .as_array()
                .unwrap()
                .iter()
                .any(|record| {
                    record["job_id"] == serde_json::json!(published_id)
                        && record["active"] == serde_json::Value::Bool(true)
                }),
            "concurrent publication {published_id} must remain in the index"
        );
    }
}
