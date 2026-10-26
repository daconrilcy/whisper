use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::Command,
    thread,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use whisper_adapters::{
    archive::{ArchiveStore, sha256_file},
    model_catalog::LocalModelCatalog,
    source_binding::SourceBinding,
};
use whisper_core::ports::DuplicateImportChoice;
use whisper_core::{
    AppCommand, Application, ComputeChoice, Generation, ImportApplication, JobConfig, JobId,
    JobState, ports::ImportRequest,
};

fn root() -> PathBuf {
    let base = PathBuf::from(std::env::var_os("PARAKEET_L2_ARCHIVE_ROOT").expect("campaign root"));
    let path = base.join(format!(
        "reprocessing-create-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

fn wav(path: &std::path::Path) {
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

fn request(root: &std::path::Path, source: &std::path::Path, id: u128) -> ImportRequest {
    ImportRequest {
        strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
        job_id: JobId(id),
        generation: Generation::first(),
        source_path: source.to_string_lossy().into_owned(),
        source_sha256: None,
        source_samples: None,
        model_path: "model.bin".into(),
        model_sha256: [0; 32],
        destination: root.to_string_lossy().into_owned(),
        config: JobConfig {
            language: whisper_core::LanguageChoice::Automatic,
            compute: whisper_core::ComputeChoice::Cpu,
        },
    }
}

fn sha_hex(sha: [u8; 32]) -> String {
    sha.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn registry(root: &std::path::Path) -> PathBuf {
    root.join("settings/reprocessing-snapshots")
}

fn transcribe(root: &std::path::Path, source: &std::path::Path, id: u128) {
    let mut request = request(root, source, id);
    let binding = SourceBinding::acquire(&request, &registry(root)).unwrap();
    request.source_sha256 = Some(binding.source_sha256());
    request.source_samples = Some(binding.source_samples());
    let sha = sha_hex(binding.source_sha256());
    ArchiveStore::reserve_create_only_new_import(root, &request, &sha).unwrap();
    let generation = root
        .join("transcriptions")
        .join(format!("{:032x}", id))
        .join("1");
    fs::create_dir_all(&generation).unwrap();
    let text = format!("transcription {id}");
    let srt = format!("1\n00:00:00,000 --> 00:00:01,000\ntranscription {id}\n");
    fs::write(generation.join("transcript.txt"), &text).unwrap();
    fs::write(generation.join("transcript.srt"), &srt).unwrap();
    let model = whisper_core::model_registry::model(
        whisper_core::model_catalog::ModelId::WhisperLargeV3Turbo,
    );
    let source_sha = sha.clone();
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
    let artifact = |path: &str, contents: &str| {
        serde_json::json!({
            "path": path,
            "sha256": sha_hex(Sha256::digest(contents.as_bytes()).into()),
            "bytes": contents.len()
        })
    };
    let manifest = serde_json::json!({
        "version": 1,
        "job_id": id,
        "generation": 1,
        "source_sha256": sha,
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
    ArchiveStore::resolve_import_publication(root, JobId(id), Generation::first()).unwrap();
    binding.retire().unwrap();
}

fn compile_snapshot_worker(root: &std::path::Path, capture_path: &std::path::Path) -> PathBuf {
    let source_path = root.join("rtc01-snapshot-worker.rs");
    let executable = root.join("rtc01-snapshot-worker.exe");
    let capture = format!("{capture_path:?}");
    let program = r#"
use std::{io::{self, BufRead, Write}};
const CAPTURE: &str = __CAPTURE__;
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
    std::fs::write(CAPTURE, &line).unwrap();
    let job = number(&line, "job_id");
    let generation = number(&line, "generation");
    let instance = number(&line, "instance_id");
    let samples = number(&line, "expected_source_samples");
    emit(format!("\"Ready\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"backend\":\"Cpu\"}}"));
    emit(format!("\"Segment\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"segment_id\":1,\"range\":{{\"start_sample\":0,\"end_sample\":{samples},\"sample_rate_hz\":16000}},\"text\":\"snapshot-backed transcript\"}}"));
    emit(format!("\"Progress\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"completed_samples\":{samples},\"total_samples\":{samples}}}"));
    emit(format!("\"End\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"last_sequence\":1,\"last_offset\":{samples}}}"));
}
"#
    .replace("__CAPTURE__", &capture);
    fs::write(&source_path, program).unwrap();
    let output = Command::new("rustc")
        .args(["--edition=2021", "--crate-name", "rtc01_snapshot_worker"])
        .arg(&source_path)
        .arg("-o")
        .arg(&executable)
        .output()
        .expect("rustc fake worker");
    assert!(
        output.status.success(),
        "fake worker compile failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    executable
}

#[test]
fn create_new_publishes_only_its_internal_pair_and_preserves_existing_treatment() {
    let root = root();
    let source = root.join("same-audio.wav");
    wav(&source);

    // The first publication is internal-only and does not create exports/.
    transcribe(&root, &source, 21);
    let identity = SourceBinding::acquire(&request(&root, &source, 99), &registry(&root)).unwrap();
    let source_sha = sha_hex(identity.source_sha256());
    identity.retire().unwrap();
    let published = ArchiveStore::published_imports(&root, &source_sha).unwrap();
    assert_eq!(published.len(), 1);
    assert_eq!(published[0].export_basename, None);
    let old_dir = root
        .join("transcriptions")
        .join(format!("{:032x}", 21_u128))
        .join("1");

    // Keep the existing identity list, then corrupt the old transcript. CreateNew
    // must not rescan or repair the old generation during reservation/publication.
    fs::write(old_dir.join("transcript.txt"), b"legacy corrupt generation").unwrap();
    let old_corrupt_bytes = fs::read(old_dir.join("transcript.txt")).unwrap();
    transcribe(&root, &source, 22);
    assert_eq!(
        fs::read(old_dir.join("transcript.txt")).unwrap(),
        old_corrupt_bytes
    );
    let new_dir = root
        .join("transcriptions")
        .join(format!("{:032x}", 22_u128))
        .join("1");
    assert!(new_dir.join("transcript.txt").is_file());
    assert!(new_dir.join("transcript.srt").is_file());
    assert!(!root.join("exports").exists());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn application_create_new_processes_the_bound_snapshot_and_preserves_prior_publication() {
    let root = root();
    let destination = root.join("user-destination");
    fs::create_dir_all(&destination).unwrap();
    let source = root.join("same-audio.wav");
    wav(&source);

    // Seed one valid prior treatment, then exercise the real application and adapter path.
    transcribe(&destination, &source, 121);
    let source_hash = sha_hex(sha256_file(&source).unwrap());
    let old_publications = ArchiveStore::published_imports(&destination, &source_hash).unwrap();
    assert_eq!(old_publications.len(), 1);
    let old_generation = destination
        .join("transcriptions")
        .join(format!("{:032x}", 121_u128))
        .join("1");
    let old_txt = fs::read(old_generation.join("transcript.txt")).unwrap();
    let old_srt = fs::read(old_generation.join("transcript.srt")).unwrap();

    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let model = repository.join("models/ggml-large-v3-turbo.bin");
    assert!(
        model.is_file(),
        "local Turbo model required for catalog admission"
    );
    let canonical_model = fs::canonicalize(&model).unwrap();
    // Match desktop composition: catalog inspection is completed before an import is
    // submitted, so the asynchronous admission path reuses its verified model identity.
    let catalog_started = Instant::now();
    let catalog = LocalModelCatalog::new(repository.join("models")).inspect(1);
    let catalog_elapsed = catalog_started.elapsed();
    let model_sha = catalog
        .entries
        .iter()
        .find(|entry| entry.id == whisper_core::model_catalog::ModelId::WhisperLargeV3Turbo)
        .and_then(|entry| entry.observed_sha256.as_ref())
        .expect("Turbo model must be present in the inspected catalog");
    let model_hash = model_sha
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect::<Vec<_>>()
        .try_into()
        .expect("SHA-256 has 32 bytes");
    eprintln!("[rtc01-r1] catalog inspection: {catalog_elapsed:?}");
    let fake_worker = compile_snapshot_worker(&root, &root.join("worker-start.json"));
    let io = whisper_adapters::worker_ipc::AsyncImportIo::start_with_model_catalog_and_parakeet_routes_and_snapshot_registry(
        whisper_adapters::supervisor::WorkerPaths {
            cpu: fake_worker.clone(),
            gpu: fake_worker,
        },
        repository.join("models"),
        canonical_model.to_string_lossy().into_owned(),
        None,
        None,
        Some(registry(&root)),
    );
    let mut app = ImportApplication::new(io);
    let mut incoming = request(&destination, &source, 122);
    incoming.model_path = canonical_model.to_string_lossy().into_owned();
    incoming.model_sha256 = model_hash;
    incoming.config.compute = ComputeChoice::Cpu;
    app.dispatch(AppCommand::EnqueueImport { request: incoming })
        .unwrap();
    let enqueue_started = Instant::now();

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    let prompt = loop {
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        if let Some(prompt) = view.duplicate_import_prompt {
            eprintln!(
                "[rtc01-r1] duplicate prompt after {:?}",
                enqueue_started.elapsed()
            );
            break prompt;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "duplicate prompt did not arrive: {view:?}"
        );
        thread::sleep(std::time::Duration::from_millis(10));
    };
    app.dispatch(AppCommand::ResolveDuplicateImport {
        prompt,
        choice: DuplicateImportChoice::CreateNew,
    })
    .unwrap();

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    let queued = loop {
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        if let Some(entry) = view
            .queue
            .iter()
            .find(|entry| entry.request.job_id != JobId(121))
        {
            break entry.request.clone();
        }
        assert!(
            std::time::Instant::now() < deadline,
            "CreateNew was not durably queued"
        );
        thread::sleep(std::time::Duration::from_millis(10));
    };
    app.dispatch(AppCommand::ProcessQueued {
        request: queued.clone(),
    })
    .unwrap();

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    loop {
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        if view.active_job == Some((queued.job_id, JobState::Complete)) {
            break;
        }
        assert_ne!(
            view.active_job,
            Some((queued.job_id, JobState::Recoverable)),
            "fake worker import failed: {:?}",
            view.message
        );
        assert!(
            std::time::Instant::now() < deadline,
            "CreateNew application import timed out"
        );
        thread::sleep(std::time::Duration::from_millis(10));
    }

    let start_frame = fs::read_to_string(root.join("worker-start.json")).unwrap();
    assert!(start_frame.contains(".whisper-source-snapshots"));
    assert!(start_frame.contains(".snapshot"));
    assert!(!start_frame.contains("same-audio.wav"));

    let new_generation = destination
        .join("transcriptions")
        .join(format!("{:032x}", queued.job_id.0))
        .join(queued.generation.get().to_string());
    assert_eq!(
        fs::read_to_string(new_generation.join("transcript.txt")).unwrap(),
        "snapshot-backed transcript"
    );
    assert!(new_generation.join("transcript.srt").is_file());
    assert!(!destination.join("exports").exists());
    assert_eq!(
        fs::read(old_generation.join("transcript.txt")).unwrap(),
        old_txt
    );
    assert_eq!(
        fs::read(old_generation.join("transcript.srt")).unwrap(),
        old_srt
    );
    let publications = ArchiveStore::published_imports(&destination, &source_hash).unwrap();
    assert_eq!(publications.len(), 2);
    assert!(
        publications
            .iter()
            .all(|publication| publication.export_basename.is_none())
    );
    fs::remove_dir_all(root).unwrap();
}
