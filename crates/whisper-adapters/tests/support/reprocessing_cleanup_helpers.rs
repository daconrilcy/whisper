use super::*;

pub(super) struct ReplacementScenario {
    pub(super) destination: PathBuf,
    pub(super) source: PathBuf,
    pub(super) source_sha: String,
    pub(super) publication_index: PathBuf,
    pub(super) prepublication_index: Vec<u8>,
    pub(super) app: ImportApplication<whisper_adapters::worker_ipc::AsyncImportIo>,
    pub(super) queued: ImportRequest,
    pub(super) latest_generation: PathBuf,
    pub(super) latest_txt: PathBuf,
    pub(super) latest_srt: PathBuf,
    pub(super) older_generation: PathBuf,
    pub(super) third_generation: PathBuf,
    pub(super) late_unknown_generation_file: PathBuf,
    pub(super) late_container_file: PathBuf,
    pub(super) legacy_container_file: PathBuf,
    pub(super) original_source: Vec<u8>,
    pub(super) cleanup_id: String,
    pub(super) plan_path: PathBuf,
    pub(super) recovery_plan_before_cleanup: Vec<u8>,
}

pub(super) fn start_replacement_import_app(
    root: &Path,
    destination: &Path,
    source: &Path,
) -> ImportApplication<whisper_adapters::worker_ipc::AsyncImportIo> {
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
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect::<Vec<_>>()
        .try_into()
        .unwrap();
    let worker = compile_worker(root);
    let io = whisper_adapters::worker_ipc::AsyncImportIo::start_with_model_catalog_and_parakeet_routes_and_snapshot_registry(
        whisper_adapters::supervisor::WorkerPaths { cpu: worker.clone(), gpu: worker },
        repository.join("models"),
        fs::canonicalize(&model_path).unwrap().to_string_lossy().into_owned(),
        None,
        None,
        Some(snapshot_registry(root)),
    );
    let mut app = ImportApplication::new(io);
    let mut incoming = request(destination, source, 104);
    incoming.model_path = fs::canonicalize(&model_path)
        .unwrap()
        .to_string_lossy()
        .into_owned();
    incoming.model_sha256 = model_sha;
    app.dispatch(AppCommand::EnqueueImport { request: incoming })
        .unwrap();
    app
}

pub(super) fn confirm_replacement_and_queue(
    app: &mut ImportApplication<whisper_adapters::worker_ipc::AsyncImportIo>,
    destination: &Path,
    latest_generation: &Path,
) -> (ImportRequest, PathBuf, PathBuf, PathBuf) {
    let unknown_generation_file = latest_generation.join("user-note.txt");
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
    assert_eq!(prompt.publications.len(), 4);
    assert_eq!(
        prompt
            .publications
            .iter()
            .map(|item| item.success_ordinal)
            .collect::<Vec<_>>(),
        vec![Some(1), Some(2), Some(3), Some(4)]
    );
    app.dispatch(AppCommand::ResolveDuplicateImport {
        prompt: Box::new(prompt),
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
    let late_container_file = destination
        .join("transcriptions/00000000000000000000000000000067")
        .join("keep-container-note.txt");
    fs::write(&late_container_file, b"keep this later container data").unwrap();
    let legacy_container_file = destination
        .join("transcriptions/00000000000000000000000000000066")
        .join("legacy-container-note.txt");
    fs::write(&legacy_container_file, b"container predates RTC-04").unwrap();
    (
        queued,
        late_unknown_generation_file,
        late_container_file,
        legacy_container_file,
    )
}

pub(super) fn verify_replacement_history(
    destination: &Path,
    source: &Path,
) -> (String, PathBuf, Vec<u8>) {
    let source_sha = sha_hex(sha256_file(source).unwrap());
    let initial_catalog = ArchiveStore::reprocessing_catalog(destination, &source_sha).unwrap();
    let publication_index = destination.join(".whisper-publications/index.json");
    let prepublication_index = fs::read(&publication_index).unwrap();
    assert_eq!(initial_catalog.len(), 4);
    assert_eq!(
        initial_catalog
            .iter()
            .map(|item| item.success_ordinal)
            .collect::<Vec<_>>(),
        vec![Some(1), Some(2), Some(3), Some(4)]
    );
    assert!(
        initial_catalog
            .iter()
            .all(|item| item.active && item.verified_present)
    );

    let canonical_destination = destination.canonicalize().unwrap();
    let partial_request = request(destination, source, 900);
    let partial_selected = vec![whisper_core::ports::TreatmentIdentity {
        destination: canonical_destination.to_string_lossy().into_owned(),
        job_id: JobId(101),
        generation: Generation::first(),
    }];
    ArchiveStore::reserve_reprocessing_import(
        destination,
        &partial_request,
        &sha_hex(sha256_file(source).unwrap()),
        16_000,
        &initial_catalog,
        &partial_selected,
        DuplicateImportChoice::ReplaceSelectedPath(
            destination
                .join("transcriptions/00000000000000000000000000000065/1")
                .to_string_lossy()
                .into_owned(),
        ),
    )
    .unwrap();
    let partial_intent_path = destination
        .join(".whisper-publications/reservations/00000000000000000000000000000384-1.json");
    let partial_intent: serde_json::Value =
        serde_json::from_slice(&fs::read(&partial_intent_path).unwrap()).unwrap();
    assert!(partial_intent["cleanup_targets"][0]["container"].is_null());
    assert!(
        destination
            .join("transcriptions/00000000000000000000000000000065")
            .is_dir()
    );
    ArchiveStore::clear_duplicate_reservation(destination, JobId(900), Generation::first())
        .unwrap();

    let all_generations_request = request(destination, source, 901);
    let all_generations_selected = [Generation::first(), Generation::first().next().unwrap()]
        .into_iter()
        .map(|generation| whisper_core::ports::TreatmentIdentity {
            destination: canonical_destination.to_string_lossy().into_owned(),
            job_id: JobId(101),
            generation,
        })
        .collect::<Vec<_>>();
    ArchiveStore::reserve_reprocessing_import(
        destination,
        &all_generations_request,
        &sha_hex(sha256_file(source).unwrap()),
        16_000,
        &initial_catalog,
        &all_generations_selected,
        DuplicateImportChoice::ReplaceAll,
    )
    .unwrap();
    let all_intent_path = destination
        .join(".whisper-publications/reservations/00000000000000000000000000000385-1.json");
    let all_intent: serde_json::Value =
        serde_json::from_slice(&fs::read(&all_intent_path).unwrap()).unwrap();
    assert!(
        all_intent["cleanup_targets"]
            .as_array()
            .unwrap()
            .iter()
            .any(|target| !target["container"].is_null())
    );
    ArchiveStore::clear_duplicate_reservation(destination, JobId(901), Generation::first())
        .unwrap();
    (source_sha, publication_index, prepublication_index)
}

pub(super) fn prepare_replacement_scenario() -> ReplacementScenario {
    let root = root();
    let destination = root.join("destination");
    fs::create_dir_all(&destination).unwrap();
    let source = root.join("audio.wav");
    wav(&source);
    seed_treatment(&destination, &source, 101);
    seed_treatment(&destination, &source, 102);
    seed_treatment(&destination, &source, 103);
    seed_treatment_generation(
        &destination,
        &source,
        101,
        Generation::first().next().unwrap(),
    );
    add_owned_and_unowned_export_fixtures(&destination, 102);
    let (source_sha, publication_index, prepublication_index) =
        verify_replacement_history(&destination, &source);
    let latest_generation = destination.join("transcriptions/00000000000000000000000000000066/1");
    let latest_txt = latest_generation.join("transcript.txt");
    let latest_srt = latest_generation.join("transcript.srt");
    let older_generation = destination.join("transcriptions/00000000000000000000000000000065/1");
    let third_generation = destination.join("transcriptions/00000000000000000000000000000067/1");
    let unknown_generation_file = latest_generation.join("user-note.txt");
    fs::write(&unknown_generation_file, b"keep this unrelated file").unwrap();
    let mut app = start_replacement_import_app(&root, &destination, &source);
    let (queued, late_unknown_generation_file, late_container_file, legacy_container_file) =
        confirm_replacement_and_queue(&mut app, &destination, &latest_generation);
    ReplacementScenario {
        destination,
        source,
        source_sha,
        publication_index,
        prepublication_index,
        app,
        queued,
        latest_generation,
        latest_txt,
        latest_srt,
        older_generation,
        third_generation,
        late_unknown_generation_file,
        late_container_file,
        legacy_container_file,
        original_source: Vec::new(),
        cleanup_id: String::new(),
        plan_path: PathBuf::new(),
        recovery_plan_before_cleanup: Vec::new(),
    }
}

pub(super) fn publish_replacement_and_check_source_change(scenario: &mut ReplacementScenario) {
    let source = &scenario.source;
    let destination = &scenario.destination;
    let app = &mut scenario.app;
    let queued = &scenario.queued;
    let latest_txt = &scenario.latest_txt;
    let latest_srt = &scenario.latest_srt;
    let older_generation = &scenario.older_generation;
    let third_generation = &scenario.third_generation;
    // A changed original must block every deletion. After it is restored, an
    // unknown child forces a durable partial cleanup and explicit resume.
    let original_source = fs::read(source).unwrap();
    let mut changed_source = original_source.clone();
    *changed_source.last_mut().unwrap() ^= 1;
    fs::write(source, &changed_source).unwrap();
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
        let cleanups = ArchiveStore::replacement_cleanups(destination).unwrap();
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
    scenario.original_source = original_source;
    scenario.cleanup_id = cleanup.cleanup_id;
}

pub(super) fn replay_publication_after_crash(scenario: &mut ReplacementScenario) {
    let destination = &scenario.destination;
    let queued = &scenario.queued;
    let publication_index = &scenario.publication_index;
    let prepublication_index = &scenario.prepublication_index;
    let source_sha = &scenario.source_sha;
    let source = &scenario.source;
    let original_source = &scenario.original_source;
    let latest_txt = &scenario.latest_txt;
    let latest_srt = &scenario.latest_srt;
    let older_generation = &scenario.older_generation;
    let third_generation = &scenario.third_generation;
    // Recreate the durable state at the crash boundary after CURRENT and the
    // publication index were written, but before the plan's published proofs
    // were refreshed. Discovery must replay the publication intent idempotently
    // and must not start deleting old generations as part of recovery.
    let plan_path = destination
        .join(".whisper-publications/replacement-cleanups")
        .join(format!("{}.json", scenario.cleanup_id));
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
    fs::write(publication_index, prepublication_index).unwrap();
    let before_current = ArchiveStore::replacement_cleanups(destination).unwrap();
    assert!(
        before_current
            .iter()
            .all(|item| item.cleanup_id != scenario.cleanup_id)
    );
    assert_eq!(
        fs::read(publication_index).unwrap(),
        prepublication_index.as_slice()
    );
    assert!(latest_txt.is_file() && older_generation.join("transcript.txt").is_file());
    assert!(third_generation.join("transcript.txt").is_file());

    // Reconstruct after CURRENT but before its publication-index update. The
    // next discovery must replay the index row idempotently and not clean old
    // targets until the user explicitly resumes.
    fs::rename(&held_current, &successor_current).unwrap();

    let recovered = ArchiveStore::replacement_cleanups(destination).unwrap();
    assert!(
        recovered
            .iter()
            .any(|item| item.cleanup_id == scenario.cleanup_id)
    );
    assert!(latest_txt.is_file() && latest_srt.is_file());
    assert!(older_generation.join("transcript.txt").is_file());
    assert!(third_generation.join("transcript.txt").is_file());
    let recovered_index = fs::read(destination.join(".whisper-publications/index.json")).unwrap();
    let recovered_catalog = ArchiveStore::reprocessing_catalog(destination, source_sha).unwrap();
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
    let _ = ArchiveStore::replacement_cleanups(destination).unwrap();
    let index_after_second_recovery =
        fs::read(destination.join(".whisper-publications/index.json")).unwrap();
    assert_eq!(index_after_first_recovery, index_after_second_recovery);
    assert!(latest_txt.is_file() && older_generation.join("transcript.txt").is_file());
    let recovery_plan_before_cleanup = fs::read(&plan_path).unwrap();

    fs::write(source, original_source).unwrap();
    scenario.plan_path = plan_path;
    scenario.recovery_plan_before_cleanup = recovery_plan_before_cleanup;
}

pub(super) fn reject_corrupt_replacement_proofs(scenario: &ReplacementScenario) {
    let destination = &scenario.destination;
    let latest_txt = &scenario.latest_txt;
    let latest_srt = &scenario.latest_srt;
    let older_generation = &scenario.older_generation;
    let third_generation = &scenario.third_generation;
    let queued = &scenario.queued;
    let publication_index = &scenario.publication_index;
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
    assert!(ArchiveStore::resume_replacement_cleanup(destination, &scenario.cleanup_id).is_err());
    assert!(latest_txt.is_file() && latest_srt.is_file());
    assert!(moved_exports.join("voice-bench.txt").is_file());
    assert!(moved_exports.join("voice-bench.srt").is_file());
    fs::remove_dir(&exports).unwrap();
    fs::rename(&moved_exports, &exports).unwrap();

    // Tampering with the durable plan to alias a cleanup target to a successor
    // object must be rejected before deleting any old generation.
    let plan_path = destination
        .join(".whisper-publications/replacement-cleanups")
        .join(format!("{}.json", scenario.cleanup_id));
    let original_plan = fs::read(&plan_path).unwrap();
    let mut plan: serde_json::Value = serde_json::from_slice(&original_plan).unwrap();
    let successor_identity = plan["successor_files"][0]["identity"].clone();
    plan["targets"][0]["objects"][0]["identity"] = successor_identity;
    fs::write(&plan_path, serde_json::to_vec(&plan).unwrap()).unwrap();
    assert!(ArchiveStore::resume_replacement_cleanup(destination, &scenario.cleanup_id).is_err());
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
        publication_index.to_path_buf(),
    ] {
        let original = fs::read(&path).unwrap();
        fs::write(&path, b"corrupted successor proof").unwrap();
        assert!(
            ArchiveStore::resume_replacement_cleanup(destination, &scenario.cleanup_id).is_err()
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
            ArchiveStore::resume_replacement_cleanup(destination, &scenario.cleanup_id).is_err(),
            "missing proof must prevent deletion: {}",
            proof_path.display()
        );
        assert!(latest_txt.is_file() && latest_srt.is_file());
        assert!(older_generation.join("transcript.txt").is_file());
        assert!(third_generation.join("transcript.txt").is_file());
        fs::rename(&held_path, &proof_path).unwrap();
    }
}
