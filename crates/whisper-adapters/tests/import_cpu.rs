use std::cell::Cell;
use std::rc::Rc;
use std::{
    path::PathBuf,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use whisper_adapters::WorkerTransport;
use whisper_adapters::{
    archive::{ArchiveStore, sha256_file},
    decoder::DecoderProxy,
    worker_ipc::{AsyncImportIo, ChildWorkerTransport},
};
use whisper_core::{
    AppCommand, Application, ComputeChoice, Generation, ImportApplication, ImportRequest,
    JobConfig, JobId, JobState, LanguageChoice, SourceRange,
    ipc::{WorkerCommand, WorkerEvent},
    ports::{DecodeRequest, DecoderPort, DuplicateImportChoice, PortError, SourceIdentity},
};

const MODEL_HASH: &str = "1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69";
const SOURCE_HASH: &str = "716e788c790b600fe472d0ebb868094c9d9b97fdf1d581133dd3df46a30de467";

#[test]
fn r11_duplicate_import_prompts_before_queue_and_replace_keeps_all_old_archives() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let model = repo.join("models/ggml-large-v3-turbo.bin");
    if !model.is_file() {
        eprintln!("R11 duplicate import NOT RUN: local Turbo model missing");
        return;
    }
    let root = std::env::temp_dir().join(format!(
        "whisper-r11-duplicate-{}-{}",
        std::process::id(),
        unix_nanos()
    ));
    let destination = root.join("archive");
    let original_source = root.join("source.wav");
    let renamed_source = root.join("renamed-audio.wav");
    std::fs::create_dir_all(&root).unwrap();
    write_r11_silence_wav(&original_source, 16_000);
    std::fs::copy(&original_source, &renamed_source).unwrap();

    let first = r11_request(
        JobId(((std::process::id() as u128) << 64) | 101),
        &original_source,
        &destination,
        &model,
    );
    publish_r11_fixture(&first, DuplicateImportChoice::CreateNew, &[]);
    let first_publications = ArchiveStore::published_imports(
        &destination,
        &hex(&sha256_file(&original_source).unwrap()),
    )
    .unwrap();
    let mut second = r11_request(
        JobId(((std::process::id() as u128) << 64) | 102),
        &original_source,
        &destination,
        &model,
    );
    ArchiveStore::identify_source(&mut second).unwrap();
    {
        let _lock = ArchiveStore::lock_publications(&destination).unwrap();
        ArchiveStore::reserve_duplicate_import(
            &destination,
            &second,
            &hex(&sha256_file(&original_source).unwrap()),
            second.source_samples.unwrap(),
            &first_publications,
            DuplicateImportChoice::CreateNew,
            &ArchiveStore::suggest_duplicate_export_basename(&destination, &second.source_path)
                .unwrap(),
        )
        .unwrap();
    }
    publish_r11_fixture_reserved(&second);

    let source_hash = hex(&sha256_file(&original_source).unwrap());
    let before = ArchiveStore::published_imports(&destination, &source_hash).unwrap();
    assert_eq!(before.len(), 2);
    let other_destination = root.join("other-destination");
    let other_destination_fixture = r11_request(
        JobId(((std::process::id() as u128) << 64) | 108),
        &original_source,
        &other_destination,
        &model,
    );
    publish_r11_fixture(
        &other_destination_fixture,
        DuplicateImportChoice::CreateNew,
        &[],
    );
    let worker = root.join("not-started-worker.exe");
    let mut app = ImportApplication::new(AsyncImportIo::start(worker));
    let start = |job_id| r11_request(job_id, &renamed_source, &destination, &model);

    app.dispatch(AppCommand::EnqueueImport {
        request: r11_request(
            JobId(((std::process::id() as u128) << 64) | 109),
            &original_source,
            &destination,
            &model,
        ),
    })
    .unwrap();
    let same_path_prompt = wait_r11_prompt(&mut app);
    assert_eq!(same_path_prompt.publications.len(), 2);
    app.dispatch(AppCommand::CancelDuplicateImport).unwrap();
    wait_r11_queue_empty(&mut app);

    app.dispatch(AppCommand::EnqueueImport {
        request: r11_request(
            JobId(((std::process::id() as u128) << 64) | 110),
            &original_source,
            &other_destination,
            &model,
        ),
    })
    .unwrap();
    let scoped_prompt = wait_r11_prompt(&mut app);
    assert_eq!(
        scoped_prompt.publications.len(),
        1,
        "duplicate search is scoped to the selected destination"
    );
    assert_eq!(
        scoped_prompt.publications[0].job_id,
        other_destination_fixture.job_id
    );
    app.dispatch(AppCommand::CancelDuplicateImport).unwrap();
    wait_r11_queue_empty(&mut app);

    app.dispatch(AppCommand::EnqueueImport {
        request: start(JobId(((std::process::id() as u128) << 64) | 103)),
    })
    .unwrap();
    let prompt = wait_r11_prompt(&mut app);
    assert_eq!(
        prompt.publications.len(),
        2,
        "same audio content is detected after rename"
    );
    assert!(
        app.dispatch(AppCommand::Refresh).unwrap().queue.is_empty(),
        "prompt precedes durable queue admission"
    );
    let candidate_dir = destination
        .join("transcriptions")
        .join(format!("{:032x}", prompt.request.job_id.0))
        .join("1");
    assert!(
        !candidate_dir.exists(),
        "no pending archive exists before choice"
    );
    app.dispatch(AppCommand::CancelDuplicateImport).unwrap();
    assert_eq!(
        ArchiveStore::published_imports(&destination, &source_hash)
            .unwrap()
            .len(),
        2
    );

    app.dispatch(AppCommand::EnqueueImport {
        request: start(JobId(((std::process::id() as u128) << 64) | 104)),
    })
    .unwrap();
    let prompt = wait_r11_prompt(&mut app);
    app.dispatch(AppCommand::ResolveDuplicateImport {
        prompt: Box::new(prompt),
        choice: DuplicateImportChoice::CreateNew,
    })
    .unwrap();
    let created = wait_r11_queue_entry(&mut app);
    assert!(created.request.job_id != first.job_id && created.request.job_id != second.job_id);
    assert!(
        !destination
            .join("transcriptions")
            .join(format!("{:032x}", created.request.job_id.0))
            .join("1/pending.json")
            .exists()
    );
    publish_r11_fixture_reserved(&created.request);
    assert_eq!(
        ArchiveStore::published_imports(&destination, &source_hash)
            .unwrap()
            .len(),
        3
    );
    app.dispatch(AppCommand::RemoveQueued {
        job_id: created.request.job_id,
        generation: created.request.generation,
    })
    .unwrap();
    wait_r11_queue_empty(&mut app);

    app.dispatch(AppCommand::EnqueueImport {
        request: start(JobId(((std::process::id() as u128) << 64) | 105)),
    })
    .unwrap();
    let prompt = wait_r11_prompt(&mut app);
    assert_eq!(prompt.publications.len(), 3);
    let preexisting_export = std::fs::read(destination.join("exports/renamed-audio.txt")).unwrap();
    app.dispatch(AppCommand::ResolveDuplicateImport {
        prompt: Box::new(prompt.clone()),
        choice: DuplicateImportChoice::Replace,
    })
    .unwrap();
    let stale_replacement = wait_r11_queue_entry(&mut app);
    assert_eq!(
        ArchiveStore::published_imports(&destination, &source_hash)
            .unwrap()
            .len(),
        3,
        "old publications stay active until replacement succeeds"
    );
    let mut candidate = ArchiveStore::new(&destination);
    let prepared = candidate
        .prepare(stale_replacement.request.clone())
        .unwrap();
    candidate
        .persist_segment(&whisper_core::ports::WorkerSegment {
            job_id: prepared.job_id,
            generation: prepared.generation,
            instance_id: 1,
            segment_id: whisper_core::SegmentId(1),
            range: SourceRange::new(0, prepared.source_samples.unwrap(), 16_000).unwrap(),
            text: "candidate transcript".into(),
        })
        .unwrap();
    candidate
        .prepare_publish(prepared.job_id, prepared.generation, 1)
        .unwrap();
    assert!(
        std::fs::read(destination.join("exports/renamed-audio.txt")).unwrap() == preexisting_export,
        "prepared result does not replace the prior export before commit"
    );
    let competing_publications =
        ArchiveStore::published_imports(&destination, &source_hash).unwrap();
    let competitor = r11_request(
        JobId(((std::process::id() as u128) << 64) | 106),
        &original_source,
        &destination,
        &model,
    );
    publish_r11_fixture(
        &competitor,
        DuplicateImportChoice::CreateNew,
        &competing_publications,
    );
    let conflict = candidate
        .commit_publish(prepared.job_id, prepared.generation)
        .unwrap_err();
    assert!(
        conflict.contains("DuplicateStateChanged"),
        "concurrent publication must be rejected before CURRENT: {conflict}"
    );
    assert!(
        !destination
            .join("transcriptions")
            .join(format!("{:032x}", prepared.job_id.0))
            .join("1/CURRENT")
            .exists()
    );
    assert_eq!(
        ArchiveStore::published_imports(&destination, &source_hash)
            .unwrap()
            .len(),
        4
    );
    app.dispatch(AppCommand::RemoveQueued {
        job_id: stale_replacement.request.job_id,
        generation: stale_replacement.request.generation,
    })
    .unwrap();
    wait_r11_queue_empty(&mut app);

    app.dispatch(AppCommand::EnqueueImport {
        request: start(JobId(((std::process::id() as u128) << 64) | 107)),
    })
    .unwrap();
    let prompt = wait_r11_prompt(&mut app);
    assert_eq!(prompt.publications.len(), 4);
    app.dispatch(AppCommand::ResolveDuplicateImport {
        prompt: Box::new(prompt),
        choice: DuplicateImportChoice::Replace,
    })
    .unwrap();
    let replacement = wait_r11_queue_entry(&mut app);
    assert_eq!(
        ArchiveStore::published_imports(&destination, &source_hash)
            .unwrap()
            .len(),
        4,
        "old publications stay active until replacement succeeds"
    );
    publish_r11_fixture_reserved(&replacement.request);
    app.dispatch(AppCommand::RemoveQueued {
        job_id: replacement.request.job_id,
        generation: replacement.request.generation,
    })
    .unwrap();
    wait_r11_queue_empty(&mut app);
    let active = ArchiveStore::published_imports(&destination, &source_hash).unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(
        (active[0].job_id, active[0].generation),
        (replacement.request.job_id, replacement.request.generation)
    );
    assert_eq!(
        std::fs::read_dir(destination.join("transcriptions-remplacees"))
            .unwrap()
            .count(),
        8
    );
    for old in [
        first.job_id,
        second.job_id,
        created.request.job_id,
        competitor.job_id,
    ] {
        assert!(
            destination
                .join("transcriptions")
                .join(format!("{:032x}", old.0))
                .join("1/CURRENT")
                .is_file(),
            "replaced generation remains in internal archive"
        );
    }
    r11_collision_reprompts_before_queue(&original_source, &renamed_source, &model, &root);
    r11_resume_keeps_replace_reservation(&original_source, &model, &root);
    r11_replace_with_missing_old_exports_and_third_party_name(&original_source, &model, &root);
    r11_byte_identical_post_reservation_collision_is_never_claimed(&original_source, &model, &root);
    r11_exports_recover_after_one_or_two_export_interruption(&original_source, &model, &root);
    r11_editing_visible_export_preserves_internal_generation(&original_source, &model, &root);
    drop(app);
    let _ = std::fs::remove_dir_all(root);
}

fn r11_collision_reprompts_before_queue(
    source: &std::path::Path,
    renamed_source: &std::path::Path,
    model: &std::path::Path,
    root: &std::path::Path,
) {
    let destination = root.join("collision-destination");
    let existing = r11_request(
        JobId(((std::process::id() as u128) << 64) | 112),
        source,
        &destination,
        model,
    );
    publish_r11_fixture(&existing, DuplicateImportChoice::CreateNew, &[]);
    let mut app = ImportApplication::new(AsyncImportIo::start(root.join("collision-worker.exe")));
    app.dispatch(AppCommand::EnqueueImport {
        request: r11_request(
            JobId(((std::process::id() as u128) << 64) | 113),
            renamed_source,
            &destination,
            model,
        ),
    })
    .unwrap();
    let first_prompt = wait_r11_prompt(&mut app);
    let original_name = first_prompt.export_basename.clone();
    let collision_path = destination
        .join("exports")
        .join(format!("{original_name}.txt"));
    std::fs::write(&collision_path, b"third-party file").unwrap();
    app.dispatch(AppCommand::ResolveDuplicateImport {
        prompt: Box::new(first_prompt),
        choice: DuplicateImportChoice::CreateNew,
    })
    .unwrap();
    let updated_prompt = wait_r11_prompt(&mut app);
    assert_ne!(updated_prompt.export_basename, original_name);
    assert!(
        updated_prompt
            .revalidation_notice
            .as_deref()
            .is_some_and(|message| message.contains("nom de sortie"))
    );
    let view = app.dispatch(AppCommand::Refresh).unwrap();
    assert!(
        view.queue.is_empty(),
        "name collision requires renewed consent"
    );
    assert!(!ArchiveStore::job_identity_exists(
        &destination,
        updated_prompt.request.job_id,
        updated_prompt.request.generation,
    ));
    app.dispatch(AppCommand::ResolveDuplicateImport {
        prompt: Box::new(updated_prompt),
        choice: DuplicateImportChoice::CreateNew,
    })
    .unwrap();
    let queued = wait_r11_queue_entry(&mut app);
    let intent = r11_read_publication_intent(&destination, &queued.request);
    assert_eq!(intent["export_basename"], format!("{original_name} (2)"));
    assert_eq!(std::fs::read(collision_path).unwrap(), b"third-party file");
}

fn r11_resume_keeps_replace_reservation(
    source: &std::path::Path,
    model: &std::path::Path,
    root: &std::path::Path,
) {
    let destination = root.join("resume-replacement-destination");
    let first = r11_request(
        JobId(((std::process::id() as u128) << 64) | 114),
        source,
        &destination,
        model,
    );
    let second = r11_request(
        JobId(((std::process::id() as u128) << 64) | 115),
        source,
        &destination,
        model,
    );
    publish_r11_fixture(&first, DuplicateImportChoice::CreateNew, &[]);
    let first_active =
        ArchiveStore::published_imports(&destination, &hex(&sha256_file(source).unwrap())).unwrap();
    publish_r11_fixture(&second, DuplicateImportChoice::CreateNew, &first_active);
    let old_active =
        ArchiveStore::published_imports(&destination, &hex(&sha256_file(source).unwrap())).unwrap();
    assert_eq!(old_active.len(), 2);
    let old_exports: Vec<_> = ["source.txt", "source (2).txt"]
        .into_iter()
        .map(|name| std::fs::read(destination.join("exports").join(name)).unwrap())
        .collect();

    let mut interrupted = r11_request(
        JobId(((std::process::id() as u128) << 64) | 116),
        source,
        &destination,
        model,
    );
    ArchiveStore::identify_source(&mut interrupted).unwrap();
    let export_basename =
        ArchiveStore::suggest_duplicate_export_basename(&destination, &interrupted.source_path)
            .unwrap();
    {
        let _lock = ArchiveStore::lock_publications(&destination).unwrap();
        ArchiveStore::reserve_duplicate_import(
            &destination,
            &interrupted,
            &hex(&interrupted.source_sha256.unwrap()),
            interrupted.source_samples.unwrap(),
            &old_active,
            DuplicateImportChoice::Replace,
            &export_basename,
        )
        .unwrap();
    }
    let previous_generation = interrupted.generation;
    let mut interrupted_archive = ArchiveStore::new(&destination);
    interrupted = interrupted_archive.prepare(interrupted).unwrap();
    interrupted_archive
        .persist_segment(&whisper_core::ports::WorkerSegment {
            job_id: interrupted.job_id,
            generation: interrupted.generation,
            instance_id: 1,
            segment_id: whisper_core::SegmentId(1),
            range: SourceRange::new(0, 8_000, 16_000).unwrap(),
            text: "prefix".into(),
        })
        .unwrap();
    assert_eq!(
        ArchiveStore::published_imports(&destination, &hex(&sha256_file(source).unwrap()),)
            .unwrap(),
        old_active,
        "interrupted Replace leaves all old publications active"
    );

    let next_generation = previous_generation.next().unwrap();
    let mut resumed_request = interrupted.clone();
    resumed_request.generation = next_generation;
    let mut resumed_archive = ArchiveStore::new(&destination);
    let (resumed, prefix) = resumed_archive
        .resume_from_with_qualification(&interrupted, resumed_request, None, None)
        .unwrap();
    assert_eq!(prefix.len(), 1);
    let intent = r11_read_publication_intent(&destination, &resumed);
    assert_eq!(intent["choice"], "Replace");
    assert_eq!(intent["export_basename"], export_basename);
    assert_eq!(intent["expected_active"].as_array().unwrap().len(), 2);
    assert!(!r11_reservation_path(&destination, resumed.job_id, previous_generation).exists());
    assert_eq!(
        ArchiveStore::published_imports(&destination, &hex(&sha256_file(source).unwrap()),)
            .unwrap(),
        old_active,
        "resume does not activate replacement before successful publication"
    );
    for (name, expected) in ["source.txt", "source (2).txt"]
        .into_iter()
        .zip(old_exports)
    {
        assert_eq!(
            std::fs::read(destination.join("exports").join(name)).unwrap(),
            expected
        );
    }
    resumed_archive
        .persist_segment(&whisper_core::ports::WorkerSegment {
            job_id: resumed.job_id,
            generation: resumed.generation,
            instance_id: 1,
            segment_id: whisper_core::SegmentId(2),
            range: SourceRange::new(8_000, 16_000, 16_000).unwrap(),
            text: "suffix".into(),
        })
        .unwrap();
    resumed_archive
        .publish(resumed.job_id, resumed.generation, 2)
        .unwrap();
    let active =
        ArchiveStore::published_imports(&destination, &hex(&sha256_file(source).unwrap())).unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(
        (active[0].job_id, active[0].generation),
        (resumed.job_id, resumed.generation)
    );
    assert!(
        r11_generation_path(&destination, first.job_id, first.generation)
            .join("CURRENT")
            .exists()
    );
    assert!(
        r11_generation_path(&destination, second.job_id, second.generation)
            .join("CURRENT")
            .exists()
    );
}

fn r11_replace_with_missing_old_exports_and_third_party_name(
    source: &std::path::Path,
    model: &std::path::Path,
    root: &std::path::Path,
) {
    let destination = root.join("removed-exports-destination");
    let old = r11_request(
        JobId(((std::process::id() as u128) << 64) | 117),
        source,
        &destination,
        model,
    );
    publish_r11_fixture(&old, DuplicateImportChoice::CreateNew, &[]);
    let old_active =
        ArchiveStore::published_imports(&destination, &hex(&sha256_file(source).unwrap())).unwrap();
    let old_name = old_active[0].export_basename.as_deref().unwrap();
    let old_txt = destination.join("exports").join(format!("{old_name}.txt"));
    let old_srt = destination.join("exports").join(format!("{old_name}.srt"));
    std::fs::remove_file(old_txt).unwrap();
    std::fs::remove_file(old_srt).unwrap();
    let third_party = b"user-owned file at former export name";
    let third_party_path = destination.join("exports").join(format!("{old_name}.txt"));
    std::fs::write(&third_party_path, third_party).unwrap();

    let mut replacement = r11_request(
        JobId(((std::process::id() as u128) << 64) | 118),
        source,
        &destination,
        model,
    );
    ArchiveStore::identify_source(&mut replacement).unwrap();
    let basename =
        ArchiveStore::suggest_duplicate_export_basename(&destination, &replacement.source_path)
            .unwrap();
    assert_ne!(basename, old_name);
    {
        let _lock = ArchiveStore::lock_publications(&destination).unwrap();
        ArchiveStore::reserve_duplicate_import(
            &destination,
            &replacement,
            &hex(&replacement.source_sha256.unwrap()),
            replacement.source_samples.unwrap(),
            &old_active,
            DuplicateImportChoice::Replace,
            &basename,
        )
        .unwrap();
    }
    let mut archive = ArchiveStore::new(&destination);
    let prepared = archive.prepare(replacement).unwrap();
    archive
        .persist_segment(&whisper_core::ports::WorkerSegment {
            job_id: prepared.job_id,
            generation: prepared.generation,
            instance_id: 1,
            segment_id: whisper_core::SegmentId(1),
            range: SourceRange::new(0, prepared.source_samples.unwrap(), 16_000).unwrap(),
            text: "replacement publication".into(),
        })
        .unwrap();
    archive
        .publish(prepared.job_id, prepared.generation, 1)
        .unwrap();
    assert_eq!(std::fs::read(third_party_path).unwrap(), third_party);
    let active =
        ArchiveStore::published_imports(&destination, &hex(&sha256_file(source).unwrap())).unwrap();
    assert_eq!(active.len(), 1);
    let manifest: serde_json::Value = serde_json::from_slice(
        &std::fs::read(
            r11_generation_path(&destination, prepared.job_id, prepared.generation)
                .join("manifest.json"),
        )
        .unwrap(),
    )
    .unwrap();
    for (extension, artifact) in [("txt", &manifest["text"]), ("srt", &manifest["srt"])] {
        let exported = destination
            .join("exports")
            .join(format!("{basename}.{extension}"));
        assert!(exported.is_file(), "replacement {extension} is visible");
        assert_eq!(
            hex(&sha256_file(&exported).unwrap()),
            artifact["sha256"].as_str().unwrap(),
            "replacement {extension} matches its verified manifest"
        );
    }
    let backups = destination.join("transcriptions-remplacees");
    assert!(
        !backups
            .join(format!("{old_name}-{:032x}-1.txt", old.job_id.0))
            .exists(),
        "the unrelated same-name file is never moved into replacement history"
    );
}

fn r11_byte_identical_post_reservation_collision_is_never_claimed(
    source: &std::path::Path,
    model: &std::path::Path,
    root: &std::path::Path,
) {
    let destination = root.join("byte-identical-collision-destination");
    let initial = r11_request(
        JobId(((std::process::id() as u128) << 64) | 119),
        source,
        &destination,
        model,
    );
    publish_r11_fixture(&initial, DuplicateImportChoice::CreateNew, &[]);
    let source_hash = hex(&sha256_file(source).unwrap());
    let old_active = ArchiveStore::published_imports(&destination, &source_hash).unwrap();
    assert_eq!(old_active.len(), 1);

    let mut interrupted = r11_request(
        JobId(((std::process::id() as u128) << 64) | 120),
        source,
        &destination,
        model,
    );
    ArchiveStore::identify_source(&mut interrupted).unwrap();
    let reserved_name =
        ArchiveStore::suggest_duplicate_export_basename(&destination, &interrupted.source_path)
            .unwrap();
    {
        let _lock = ArchiveStore::lock_publications(&destination).unwrap();
        ArchiveStore::reserve_duplicate_import(
            &destination,
            &interrupted,
            &source_hash,
            interrupted.source_samples.unwrap(),
            &old_active,
            DuplicateImportChoice::Replace,
            &reserved_name,
        )
        .unwrap();
    }
    let mut interrupted_archive = ArchiveStore::new(&destination);
    let prepared = interrupted_archive.prepare(interrupted.clone()).unwrap();
    interrupted_archive
        .persist_segment(&whisper_core::ports::WorkerSegment {
            job_id: prepared.job_id,
            generation: prepared.generation,
            instance_id: 1,
            segment_id: whisper_core::SegmentId(1),
            range: SourceRange::new(0, prepared.source_samples.unwrap(), 16_000).unwrap(),
            text: "synthetic transcript".into(),
        })
        .unwrap();
    interrupted_archive
        .prepare_publish(prepared.job_id, prepared.generation, 1)
        .unwrap();
    let generation = r11_generation_path(&destination, prepared.job_id, prepared.generation);
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(generation.join("manifest.json")).unwrap()).unwrap();
    let expected_txt =
        std::fs::read(generation.join(manifest["text"]["path"].as_str().unwrap())).unwrap();
    let third_party_path = destination
        .join("exports")
        .join(format!("{reserved_name}.txt"));
    std::fs::write(&third_party_path, &expected_txt).unwrap();

    let error = interrupted_archive
        .commit_publish(prepared.job_id, prepared.generation)
        .unwrap_err();
    assert!(error.contains("export name collision"), "{error}");
    assert_eq!(std::fs::read(&third_party_path).unwrap(), expected_txt);
    assert!(
        !generation.join("installed-exports.json").exists(),
        "a byte-identical external file is never recorded as an application export"
    );
    let still_active = ArchiveStore::published_imports(&destination, &source_hash).unwrap();
    assert_eq!(still_active.len(), 1);
    assert_eq!(still_active[0].job_id, old_active[0].job_id);

    let mut retry = r11_request(
        JobId(((std::process::id() as u128) << 64) | 121),
        source,
        &destination,
        model,
    );
    ArchiveStore::identify_source(&mut retry).unwrap();
    let retry_name =
        ArchiveStore::suggest_duplicate_export_basename(&destination, &retry.source_path).unwrap();
    assert_ne!(retry_name, reserved_name);
    {
        let _lock = ArchiveStore::lock_publications(&destination).unwrap();
        ArchiveStore::reserve_duplicate_import(
            &destination,
            &retry,
            &source_hash,
            retry.source_samples.unwrap(),
            &old_active,
            DuplicateImportChoice::Replace,
            &retry_name,
        )
        .unwrap();
    }
    let mut retry_archive = ArchiveStore::new(&destination);
    let retry_prepared = retry_archive.prepare(retry).unwrap();
    retry_archive
        .persist_segment(&whisper_core::ports::WorkerSegment {
            job_id: retry_prepared.job_id,
            generation: retry_prepared.generation,
            instance_id: 1,
            segment_id: whisper_core::SegmentId(1),
            range: SourceRange::new(0, retry_prepared.source_samples.unwrap(), 16_000).unwrap(),
            text: "replacement transcript".into(),
        })
        .unwrap();
    retry_archive
        .publish(retry_prepared.job_id, retry_prepared.generation, 1)
        .unwrap();
    assert_eq!(std::fs::read(&third_party_path).unwrap(), expected_txt);
    let backup_collision = destination
        .join("transcriptions-remplacees")
        .join(format!("{reserved_name}-{:032x}-1.txt", prepared.job_id.0));
    assert!(
        !backup_collision.exists(),
        "Replace never moves the byte-identical third-party export"
    );
}

fn r11_exports_recover_after_one_or_two_export_interruption(
    source: &std::path::Path,
    model: &std::path::Path,
    root: &std::path::Path,
) {
    r11_recover_after_single_export_collision(source, model, root);
    r11_recover_after_both_exports_before_current(source, model, root);
}

fn r11_editing_visible_export_preserves_internal_generation(
    source: &std::path::Path,
    model: &std::path::Path,
    root: &std::path::Path,
) {
    let destination = root.join("edited-export-destination");
    let request = r11_request(
        JobId(((std::process::id() as u128) << 64) | 124),
        source,
        &destination,
        model,
    );
    publish_r11_fixture(&request, DuplicateImportChoice::CreateNew, &[]);

    let source_hash = hex(&sha256_file(source).unwrap());
    let published = ArchiveStore::published_imports(&destination, &source_hash).unwrap();
    assert_eq!(published.len(), 1);
    let publication = &published[0];
    let basename = publication.export_basename.as_ref().unwrap();
    let generation = r11_generation_path(&destination, publication.job_id, publication.generation);
    let manifest_bytes = std::fs::read(generation.join("manifest.json")).unwrap();
    let manifest: serde_json::Value = serde_json::from_slice(&manifest_bytes).unwrap();
    let expected_hash = manifest["text"]["sha256"].as_str().unwrap().to_owned();
    let internal_path = generation.join("transcript.txt");
    let internal_before = std::fs::read(&internal_path).unwrap();
    assert_eq!(hex(&sha256_file(&internal_path).unwrap()), expected_hash);

    let visible_path = destination.join("exports").join(format!("{basename}.txt"));
    std::fs::write(&visible_path, b"edited by user\n").unwrap();

    assert_eq!(std::fs::read(&internal_path).unwrap(), internal_before);
    assert_eq!(hex(&sha256_file(&internal_path).unwrap()), expected_hash);
    assert_eq!(
        std::fs::read(generation.join("manifest.json")).unwrap(),
        manifest_bytes,
        "editing the export leaves the generation manifest unchanged"
    );
    let scanned = ArchiveStore::scan(&destination).unwrap();
    let entry = scanned
        .iter()
        .find(|entry| entry.job_id == publication.job_id)
        .expect("published generation remains visible after editing its export");
    assert!(entry.complete, "internal generation remains complete");
    assert!(
        entry.diagnostic.is_none(),
        "scan remains free of corruption"
    );
    assert_eq!(
        ArchiveStore::published_imports(&destination, &source_hash).unwrap(),
        published,
        "duplicate detection still reads the intact internal generation"
    );
}

fn r11_recover_after_single_export_collision(
    source: &std::path::Path,
    model: &std::path::Path,
    root: &std::path::Path,
) {
    let destination = root.join("one-export-interruption-destination");
    let initial = r11_request(
        JobId(((std::process::id() as u128) << 64) | 122),
        source,
        &destination,
        model,
    );
    publish_r11_fixture(&initial, DuplicateImportChoice::CreateNew, &[]);
    let source_hash = hex(&sha256_file(source).unwrap());
    let old_active = ArchiveStore::published_imports(&destination, &source_hash).unwrap();
    let old_name = old_active[0].export_basename.clone().unwrap();
    let old_txt =
        std::fs::read(destination.join("exports").join(format!("{old_name}.txt"))).unwrap();

    let mut interrupted = r11_request(
        JobId(((std::process::id() as u128) << 64) | 123),
        source,
        &destination,
        model,
    );
    ArchiveStore::identify_source(&mut interrupted).unwrap();
    let export_basename =
        ArchiveStore::suggest_duplicate_export_basename(&destination, &interrupted.source_path)
            .unwrap();
    {
        let _lock = ArchiveStore::lock_publications(&destination).unwrap();
        ArchiveStore::reserve_duplicate_import(
            &destination,
            &interrupted,
            &source_hash,
            interrupted.source_samples.unwrap(),
            &old_active,
            DuplicateImportChoice::Replace,
            &export_basename,
        )
        .unwrap();
    }
    let mut first_archive = ArchiveStore::new(&destination);
    let interrupted = first_archive.prepare(interrupted).unwrap();
    first_archive
        .persist_segment(&whisper_core::ports::WorkerSegment {
            job_id: interrupted.job_id,
            generation: interrupted.generation,
            instance_id: 1,
            segment_id: whisper_core::SegmentId(1),
            range: SourceRange::new(0, 8_000, 16_000).unwrap(),
            text: "confirmed prefix".into(),
        })
        .unwrap();
    first_archive
        .prepare_publish(interrupted.job_id, interrupted.generation, 1)
        .unwrap();
    let failed_generation =
        r11_generation_path(&destination, interrupted.job_id, interrupted.generation);
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(failed_generation.join("manifest.json")).unwrap())
            .unwrap();
    let expected_srt =
        std::fs::read(failed_generation.join(manifest["srt"]["path"].as_str().unwrap())).unwrap();
    let third_party_srt = destination
        .join("exports")
        .join(format!("{export_basename}.srt"));
    std::fs::write(&third_party_srt, &expected_srt).unwrap();

    let error = first_archive
        .commit_publish(interrupted.job_id, interrupted.generation)
        .unwrap_err();
    assert!(error.contains("export name collision for srt"), "{error}");
    let partial_txt = destination
        .join("exports")
        .join(format!("{export_basename}.txt"));
    assert!(
        !partial_txt.exists(),
        "a collision detected before installation leaves no partial TXT export"
    );
    assert_eq!(std::fs::read(&third_party_srt).unwrap(), expected_srt);
    assert!(
        !failed_generation.join("CURRENT").exists(),
        "the interrupted generation is not activated"
    );
    assert_eq!(
        ArchiveStore::published_imports(&destination, &source_hash).unwrap(),
        old_active,
        "old publications remain active after the partial export write"
    );
    assert_eq!(
        std::fs::read(destination.join("exports").join(format!("{old_name}.txt"))).unwrap(),
        old_txt
    );

    // Remove the test collision to simulate the obstruction being resolved by
    // the user; the interrupted app-created hard link must remain recoverable.
    std::fs::remove_file(&third_party_srt).unwrap();
    drop(first_archive);
    let mut resumed_request = interrupted.clone();
    resumed_request.generation = interrupted.generation.next().unwrap();
    let mut resumed_archive = ArchiveStore::new(&destination);
    let (resumed, prefix) = resumed_archive
        .resume_from_with_qualification(&interrupted, resumed_request, None, None)
        .unwrap();
    assert_eq!(prefix.len(), 1);
    assert_eq!(
        ArchiveStore::published_imports(&destination, &source_hash).unwrap(),
        old_active,
        "resume keeps the prior publications active until commit"
    );
    resumed_archive
        .persist_segment(&whisper_core::ports::WorkerSegment {
            job_id: resumed.job_id,
            generation: resumed.generation,
            instance_id: 1,
            segment_id: whisper_core::SegmentId(2),
            range: SourceRange::new(8_000, 16_000, 16_000).unwrap(),
            text: "confirmed suffix".into(),
        })
        .unwrap();
    resumed_archive
        .publish(resumed.job_id, resumed.generation, 2)
        .unwrap();
    let active = ArchiveStore::published_imports(&destination, &source_hash).unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(
        (active[0].job_id, active[0].generation),
        (resumed.job_id, resumed.generation)
    );
}

fn r11_recover_after_both_exports_before_current(
    source: &std::path::Path,
    model: &std::path::Path,
    root: &std::path::Path,
) {
    let destination = root.join("both-exports-interruption-destination");
    let initial = r11_request(
        JobId(((std::process::id() as u128) << 64) | 124),
        source,
        &destination,
        model,
    );
    publish_r11_fixture(&initial, DuplicateImportChoice::CreateNew, &[]);
    let source_hash = hex(&sha256_file(source).unwrap());
    let old_active = ArchiveStore::published_imports(&destination, &source_hash).unwrap();
    let old_name = old_active[0].export_basename.clone().unwrap();
    let old_exports = ["txt", "srt"].map(|extension| {
        std::fs::read(
            destination
                .join("exports")
                .join(format!("{old_name}.{extension}")),
        )
        .unwrap()
    });

    let mut interrupted = r11_request(
        JobId(((std::process::id() as u128) << 64) | 125),
        source,
        &destination,
        model,
    );
    ArchiveStore::identify_source(&mut interrupted).unwrap();
    let export_basename =
        ArchiveStore::suggest_duplicate_export_basename(&destination, &interrupted.source_path)
            .unwrap();
    {
        let _lock = ArchiveStore::lock_publications(&destination).unwrap();
        ArchiveStore::reserve_duplicate_import(
            &destination,
            &interrupted,
            &source_hash,
            interrupted.source_samples.unwrap(),
            &old_active,
            DuplicateImportChoice::Replace,
            &export_basename,
        )
        .unwrap();
    }
    let mut first_archive = ArchiveStore::new(&destination);
    let interrupted = first_archive.prepare(interrupted).unwrap();
    first_archive
        .persist_segment(&whisper_core::ports::WorkerSegment {
            job_id: interrupted.job_id,
            generation: interrupted.generation,
            instance_id: 1,
            segment_id: whisper_core::SegmentId(1),
            range: SourceRange::new(0, 8_000, 16_000).unwrap(),
            text: "confirmed prefix".into(),
        })
        .unwrap();
    first_archive
        .prepare_publish(interrupted.job_id, interrupted.generation, 1)
        .unwrap();
    let failed_generation =
        r11_generation_path(&destination, interrupted.job_id, interrupted.generation);
    let current_path = failed_generation.join("CURRENT");
    std::fs::create_dir(&current_path).unwrap();
    let error = first_archive
        .commit_publish(interrupted.job_id, interrupted.generation)
        .unwrap_err();
    assert!(
        !error.is_empty(),
        "CURRENT directory should block publication"
    );
    let ownership_path = failed_generation.join("installed-exports.json");
    assert!(ownership_path.is_file());
    for extension in ["txt", "srt"] {
        assert!(
            destination
                .join("exports")
                .join(format!("{export_basename}.{extension}"))
                .is_file()
        );
    }
    assert_eq!(
        ArchiveStore::published_imports(&destination, &source_hash).unwrap(),
        old_active,
        "old publications remain active when both exports exist but CURRENT did not commit"
    );
    for (extension, expected) in ["txt", "srt"].into_iter().zip(old_exports) {
        assert_eq!(
            std::fs::read(
                destination
                    .join("exports")
                    .join(format!("{old_name}.{extension}"))
            )
            .unwrap(),
            expected
        );
    }

    std::fs::remove_dir(&current_path).unwrap();
    drop(first_archive);
    let mut resumed_request = interrupted.clone();
    resumed_request.generation = interrupted.generation.next().unwrap();
    let mut resumed_archive = ArchiveStore::new(&destination);
    let (resumed, prefix) = resumed_archive
        .resume_from_with_qualification(&interrupted, resumed_request, None, None)
        .unwrap();
    assert_eq!(prefix.len(), 1);
    assert_eq!(
        ArchiveStore::published_imports(&destination, &source_hash).unwrap(),
        old_active,
        "resume leaves the old generation visible until the new commit"
    );
    resumed_archive
        .persist_segment(&whisper_core::ports::WorkerSegment {
            job_id: resumed.job_id,
            generation: resumed.generation,
            instance_id: 1,
            segment_id: whisper_core::SegmentId(2),
            range: SourceRange::new(8_000, 16_000, 16_000).unwrap(),
            text: "confirmed suffix".into(),
        })
        .unwrap();
    resumed_archive
        .publish(resumed.job_id, resumed.generation, 2)
        .unwrap();
    let active = ArchiveStore::published_imports(&destination, &source_hash).unwrap();
    assert_eq!(active.len(), 1);
    assert_eq!(
        (active[0].job_id, active[0].generation),
        (resumed.job_id, resumed.generation)
    );
    assert!(
        !r11_generation_path(&destination, interrupted.job_id, interrupted.generation)
            .join("CURRENT")
            .is_file(),
        "the incomplete generation remains unactivated"
    );
}

fn r11_reservation_path(
    destination: &std::path::Path,
    job_id: JobId,
    generation: Generation,
) -> PathBuf {
    destination
        .join(".whisper-publications/reservations")
        .join(format!("{:032x}-{}.json", job_id.0, generation.get()))
}

fn r11_generation_path(
    destination: &std::path::Path,
    job_id: JobId,
    generation: Generation,
) -> PathBuf {
    destination
        .join("transcriptions")
        .join(format!("{:032x}", job_id.0))
        .join(generation.get().to_string())
}

fn r11_read_publication_intent(
    destination: &std::path::Path,
    request: &ImportRequest,
) -> serde_json::Value {
    serde_json::from_slice(
        &std::fs::read(r11_reservation_path(
            destination,
            request.job_id,
            request.generation,
        ))
        .unwrap(),
    )
    .unwrap()
}

fn publish_r11_fixture(
    request: &ImportRequest,
    choice: DuplicateImportChoice,
    expected: &[whisper_core::ports::PublishedImport],
) {
    let mut request = request.clone();
    ArchiveStore::identify_source(&mut request).unwrap();
    if !expected.is_empty() {
        let _lock =
            ArchiveStore::lock_publications(PathBuf::from(&request.destination).as_path()).unwrap();
        ArchiveStore::reserve_duplicate_import(
            PathBuf::from(&request.destination).as_path(),
            &request,
            &hex(&request.source_sha256.unwrap()),
            request.source_samples.unwrap(),
            expected,
            choice,
            &ArchiveStore::suggest_duplicate_export_basename(
                PathBuf::from(&request.destination).as_path(),
                &request.source_path,
            )
            .unwrap(),
        )
        .unwrap();
    }
    publish_r11_fixture_reserved(&request);
}

fn publish_r11_fixture_reserved(request: &ImportRequest) {
    let mut archive = ArchiveStore::new(&request.destination);
    let prepared = archive.prepare(request.clone()).unwrap();
    archive
        .persist_segment(&whisper_core::ports::WorkerSegment {
            job_id: prepared.job_id,
            generation: prepared.generation,
            instance_id: 1,
            segment_id: whisper_core::SegmentId(1),
            range: SourceRange::new(0, prepared.source_samples.unwrap(), 16_000).unwrap(),
            text: "synthetic transcript".into(),
        })
        .unwrap();
    archive
        .publish(prepared.job_id, prepared.generation, 1)
        .unwrap();
}

fn r11_request(
    job_id: JobId,
    source: &std::path::Path,
    destination: &std::path::Path,
    model: &std::path::Path,
) -> ImportRequest {
    ImportRequest {
        strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
        job_id,
        generation: Generation::first(),
        source_path: source.to_string_lossy().into_owned(),
        source_sha256: None,
        source_samples: None,
        model_path: model.to_string_lossy().into_owned(),
        model_sha256: parse_hash(MODEL_HASH),
        destination: destination.to_string_lossy().into_owned(),
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        },
    }
}

fn wait_r11_prompt(
    app: &mut ImportApplication<AsyncImportIo>,
) -> whisper_core::ports::DuplicateImportPrompt {
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if let Some(prompt) = app
            .dispatch(AppCommand::Refresh)
            .unwrap()
            .duplicate_import_prompt
        {
            return prompt;
        }
        assert!(
            Instant::now() < deadline,
            "duplicate import decision timed out"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn wait_r11_queue_entry(
    app: &mut ImportApplication<AsyncImportIo>,
) -> whisper_core::ports::ImportQueueEntry {
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if let Some(entry) = app
            .dispatch(AppCommand::Refresh)
            .unwrap()
            .queue
            .first()
            .cloned()
        {
            return entry;
        }
        assert!(
            Instant::now() < deadline,
            "confirmed import was not durably queued"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn wait_r11_queue_empty(app: &mut ImportApplication<AsyncImportIo>) {
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if app.dispatch(AppCommand::Refresh).unwrap().queue.is_empty() {
            return;
        }
        assert!(
            Instant::now() < deadline,
            "synthetic completed queue row was not removable"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

fn write_r11_silence_wav(path: &std::path::Path, sample_count: u32) {
    use std::io::Write;
    let data_bytes = sample_count * 2;
    let mut file = std::fs::File::create(path).unwrap();
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

fn unix_nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos()
}

#[test]
fn real_wav_runs_through_cpu_worker_and_verified_archive() {
    let Some(source) = std::env::var_os("WHISPER_L01_FIXTURE").map(PathBuf::from) else {
        eprintln!("V-IMPORT NOT RUN: set WHISPER_L01_FIXTURE to the D19-manifested micro_test.wav");
        return;
    };
    let Some(model) = std::env::var_os("WHISPER_L01_MODEL").map(PathBuf::from) else {
        eprintln!("V-IMPORT NOT RUN: set WHISPER_L01_MODEL to the approved D19 model");
        return;
    };
    assert_eq!(
        hex(&sha256_file(&source).unwrap()),
        SOURCE_HASH,
        "manifested human-labelled source identity"
    );
    assert_eq!(
        hex(&sha256_file(&model).unwrap()),
        MODEL_HASH,
        "approved D19 model identity"
    );
    let before = sha256_file(&source).unwrap();
    let destination = std::env::temp_dir().join(format!(
        "whisper-l01-import-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&destination).unwrap();
    let worker = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("whisper-worker-cpu.exe");
    assert!(
        worker.is_file(),
        "CPU worker binary must exist at {}",
        worker.display()
    );
    let memory_observation = MemoryObservation::start(&worker);
    let mut app = ImportApplication::new(AsyncImportIo::start(worker));
    let job_id = JobId(((std::process::id() as u128) << 64) | 1);
    app.dispatch(AppCommand::StartImport {
        request: ImportRequest {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
            job_id,
            generation: Generation::first(),
            source_path: source.to_string_lossy().into_owned(),
            source_sha256: None,
            source_samples: None,
            model_path: model.to_string_lossy().into_owned(),
            model_sha256: parse_hash(MODEL_HASH),
            destination: destination.to_string_lossy().into_owned(),
            config: JobConfig {
                language: LanguageChoice::Manual("fr".into()),
                compute: ComputeChoice::Cpu,
            },
        },
    })
    .unwrap();
    let started = Instant::now();
    let mut saw_cpu = false;
    loop {
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        saw_cpu |= view.message.as_deref() == Some("Moteur actif : CPU");
        if view
            .active_job
            .is_some_and(|(_, state)| state == JobState::Complete)
        {
            break;
        }
        if view
            .active_job
            .is_some_and(|(_, state)| state == JobState::Recoverable)
        {
            panic!("real import failed: {:?}", view.message);
        }
        assert!(
            started.elapsed() < Duration::from_secs(300),
            "real CPU transcription exceeded the campaign bound"
        );
        thread::sleep(Duration::from_millis(20));
    }
    if let Some(observation) = memory_observation {
        observation.finish(3);
    }
    assert!(saw_cpu, "worker never attested CPU");
    assert_eq!(
        before,
        sha256_file(&source).unwrap(),
        "import must not modify its source"
    );
    let first = ArchiveStore::scan(&destination).unwrap();
    let second = ArchiveStore::scan(&destination).unwrap();
    assert_eq!(first, second, "two archive scans must be stable");
    let entry = first
        .iter()
        .find(|entry| entry.job_id == job_id)
        .expect("published job in history");
    assert!(entry.complete);
    let generation_dir = destination
        .join("transcriptions")
        .join(format!("{:032x}", job_id.0))
        .join("1");
    let transcript = std::fs::read_to_string(generation_dir.join("transcript.txt")).unwrap();
    let subtitles = std::fs::read_to_string(generation_dir.join("transcript.srt")).unwrap();
    assert!(
        !transcript.trim().is_empty(),
        "real worker must return transcript text"
    );
    assert!(!subtitles.trim().is_empty(), "real worker must publish SRT");
    eprintln!(
        "V-IMPORT PASS: source_sha256={SOURCE_HASH}; model_sha256={MODEL_HASH}; cpu_attested={saw_cpu}; transcript_bytes={}; srt_bytes={}; elapsed_ms={}",
        transcript.len(),
        subtitles.len(),
        started.elapsed().as_millis()
    );
}

#[cfg(all(windows, feature = "l01-memory-qualification"))]
#[test]
fn generated_long_wav_runs_eight_full_native_windows_with_stage_memory_samples() {
    let Some(model) = std::env::var_os("WHISPER_L01_MODEL").map(PathBuf::from) else {
        eprintln!("V-IMPORT LONG NOT RUN: set WHISPER_L01_MODEL to the approved D19 model");
        return;
    };
    assert_eq!(
        hex(&sha256_file(&model).unwrap()),
        MODEL_HASH,
        "approved D19 model identity"
    );
    assert!(
        std::env::var_os("WHISPER_L01_MEMORY_TRACE").is_some()
            && std::env::var_os("WHISPER_L01_ADAPTER_MEMORY_TRACE").is_some(),
        "long memory qualification requires worker and adapter trace paths"
    );

    const SOURCE_SAMPLES: u32 = 640_123;
    let source = std::env::temp_dir().join(format!(
        "whisper-l01-generated-{}-{}.wav",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    write_generated_silence_wav(&source, SOURCE_SAMPLES);
    let destination = std::env::temp_dir().join(format!(
        "whisper-l01-long-import-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&destination).unwrap();
    let worker = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("whisper-worker-cpu.exe");
    let observation = MemoryObservation::start(&worker)
        .expect("Windows process sampler requires both trace paths");
    let mut app = ImportApplication::new(AsyncImportIo::start(worker));
    let job_id = JobId(((std::process::id() as u128) << 64) | 2);
    app.dispatch(AppCommand::StartImport {
        request: ImportRequest {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
            job_id,
            generation: Generation::first(),
            source_path: source.to_string_lossy().into_owned(),
            source_sha256: None,
            source_samples: None,
            model_path: model.to_string_lossy().into_owned(),
            model_sha256: parse_hash(MODEL_HASH),
            destination: destination.to_string_lossy().into_owned(),
            config: JobConfig {
                language: LanguageChoice::Manual("fr".into()),
                compute: ComputeChoice::Cpu,
            },
        },
    })
    .unwrap();

    let started = Instant::now();
    let mut saw_cpu = false;
    loop {
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        saw_cpu |= view.message.as_deref() == Some("Moteur actif : CPU");
        if view
            .active_job
            .is_some_and(|(_, state)| state == JobState::Complete)
        {
            break;
        }
        if view
            .active_job
            .is_some_and(|(_, state)| state == JobState::Recoverable)
        {
            panic!("generated long import failed: {:?}", view.message);
        }
        assert!(
            started.elapsed() < Duration::from_secs(900),
            "eight-full-window real CPU qualification exceeded 15 minutes"
        );
        thread::sleep(Duration::from_millis(20));
    }
    observation.finish(9);
    assert!(saw_cpu, "worker never attested CPU");
    let entries = ArchiveStore::scan(&destination).unwrap();
    assert!(
        entries
            .iter()
            .any(|entry| entry.job_id == job_id && entry.complete)
    );
    let _ = std::fs::remove_file(&source);
    let _ = std::fs::remove_dir_all(&destination);
    eprintln!(
        "V-IMPORT LONG PASS: source_samples={SOURCE_SAMPLES}; native_windows=9 (8 full plus tail); elapsed_ms={}",
        started.elapsed().as_millis()
    );
}

#[cfg(all(windows, feature = "l01-memory-qualification"))]
fn write_generated_silence_wav(path: &std::path::Path, sample_count: u32) {
    use std::io::Write;

    let data_bytes = sample_count * 2;
    let mut file = std::fs::File::create(path).unwrap();
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
    let silence = vec![0_u8; data_bytes as usize];
    file.write_all(&silence).unwrap();
}

#[test]
fn stop_during_native_inference_is_prompt_and_never_publishes_complete() {
    let (Some(source), Some(model)) = (
        std::env::var_os("WHISPER_L01_FIXTURE").map(PathBuf::from),
        std::env::var_os("WHISPER_L01_MODEL").map(PathBuf::from),
    ) else {
        eprintln!("V-IMPORT Stop-in-inference NOT RUN: fixture/model environment missing");
        return;
    };
    assert_eq!(hex(&sha256_file(&source).unwrap()), SOURCE_HASH);
    assert_eq!(hex(&sha256_file(&model).unwrap()), MODEL_HASH);
    let worker = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("whisper-worker-cpu.exe");
    assert!(worker.is_file(), "CPU worker missing: {}", worker.display());
    let destination = std::env::temp_dir().join(format!(
        "whisper-l01-stop-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&destination).unwrap();
    let mut app = ImportApplication::new(AsyncImportIo::start(worker.clone()));
    let job_id = JobId(((std::process::id() as u128) << 64) | 2);
    let generation = Generation::first();
    app.dispatch(AppCommand::StartImport {
        request: ImportRequest {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
            job_id,
            generation,
            source_path: source.to_string_lossy().into_owned(),
            source_sha256: None,
            source_samples: None,
            model_path: model.to_string_lossy().into_owned(),
            model_sha256: parse_hash(MODEL_HASH),
            destination: destination.to_string_lossy().into_owned(),
            config: JobConfig {
                language: LanguageChoice::Manual("fr".into()),
                compute: ComputeChoice::Cpu,
            },
        },
    })
    .unwrap();

    let mut ready = false;
    let deadline = Instant::now() + Duration::from_secs(90);
    let mut previous_cpu = worker_cpu_time(&worker);
    let mut inference_cpu = Duration::ZERO;
    while Instant::now() < deadline {
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        ready |= view.message.as_deref() == Some("Moteur actif : CPU");
        let current_cpu = worker_cpu_time(&worker);
        if ready {
            inference_cpu += current_cpu.saturating_sub(previous_cpu);
            assert!(
                view.progress.is_none(),
                "worker reported progress before native inference returned: {view:?}"
            );
            if inference_cpu >= Duration::from_secs(2) {
                break;
            }
        }
        previous_cpu = current_cpu;
        thread::sleep(Duration::from_millis(100));
    }
    assert!(
        ready,
        "worker never became Ready; import did not reach inference"
    );
    assert!(
        inference_cpu >= Duration::from_secs(2),
        "worker did not accrue 2s CPU while Ready and before Progress; CPU={inference_cpu:?}"
    );

    let submitted = Instant::now();
    let view = app
        .dispatch(AppCommand::Stop { job_id, generation })
        .unwrap();
    let submission_time = submitted.elapsed();
    assert!(
        submission_time < Duration::from_millis(200),
        "Stop dispatch blocked for {submission_time:?}"
    );
    assert_eq!(view.active_job, Some((job_id, JobState::Cancelling)));

    let deadline = Instant::now() + Duration::from_secs(300);
    let terminal = loop {
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        if let Some((
            _,
            state @ (JobState::Cancelled | JobState::Recoverable | JobState::Complete),
        )) = view.active_job
        {
            break (state, view);
        }
        assert!(
            Instant::now() < deadline,
            "worker did not reach terminal state after Stop: {view:?}"
        );
        thread::sleep(Duration::from_millis(100));
    };
    assert_eq!(
        terminal.0,
        JobState::Cancelled,
        "Stop terminal state must be truthful: {:?}",
        terminal.1
    );
    assert_ne!(terminal.0, JobState::Complete);
    let entries = ArchiveStore::scan(&destination).unwrap();
    assert!(
        entries.iter().all(|entry| !entry.complete),
        "Stop must not publish a complete archive: {entries:?}"
    );
    let _ = std::fs::remove_dir_all(destination);
}

fn worker_cpu_time(worker: &std::path::Path) -> Duration {
    let output = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-Command"])
        .arg(format!(
            "$p = Get-Process -Name whisper-worker-cpu -ErrorAction SilentlyContinue | Where-Object {{ $_.Path -eq '{}' }} | Select-Object -First 1; if ($null -eq $p) {{ '0' }} else {{ [string]$p.CPU }}",
            worker.display().to_string().replace('\'', "''")
        ))
        .output()
        .expect("PowerShell is required to witness native worker CPU time on Windows");
    assert!(output.status.success(), "PowerShell CPU query failed");
    let seconds = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<f64>()
        .expect("worker CPU time query returned invalid output");
    Duration::from_secs_f64(seconds.max(0.0))
}

#[test]
fn real_worker_serves_correlated_bounded_decode_blocks() {
    let Some(source) = std::env::var_os("WHISPER_L01_FIXTURE").map(PathBuf::from) else {
        eprintln!("V-IMPORT decode block NOT RUN: set WHISPER_L01_FIXTURE");
        return;
    };
    assert_eq!(hex(&sha256_file(&source).unwrap()), SOURCE_HASH);
    let worker = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("whisper-worker-cpu.exe");
    assert!(worker.is_file(), "CPU worker missing: {}", worker.display());
    let mut decoder = DecoderProxy::new(ChildWorkerTransport::spawn(&worker).unwrap());
    for (index, (count, limit)) in [(1_u64, 1_usize), (80_000, 80_000)].into_iter().enumerate() {
        let request = DecodeRequest {
            source: SourceIdentity {
                source_id: source.to_string_lossy().into_owned(),
                sha256: parse_hash(SOURCE_HASH),
            },
            job_id: JobId(81),
            generation: Generation::first(),
            range: SourceRange::new(0, count, 16_000).unwrap(),
            max_samples: limit,
        };
        let blocks = decoder
            .decode(request.clone())
            .unwrap_or_else(|error| panic!("decode request {index} failed: {error:?}"));
        assert_eq!(blocks.len(), 1, "worker response must correlate to request");
        let block = &blocks[0];
        assert_eq!(block.job_id, request.job_id);
        assert_eq!(block.generation, request.generation);
        assert_eq!(block.block.range, request.range);
        assert_eq!(block.block.channels, 1);
        assert_eq!(block.block.samples.len(), count as usize);
    }
    let eof_request = DecodeRequest {
        source: SourceIdentity {
            source_id: source.to_string_lossy().into_owned(),
            sha256: parse_hash(SOURCE_HASH),
        },
        job_id: JobId(81),
        generation: Generation::first(),
        range: SourceRange::new(300_000, 300_100, 16_000).unwrap(),
        max_samples: 100,
    };
    assert!(
        decoder.decode(eof_request).unwrap().is_empty(),
        "the same worker transport must correlate an explicit EOF/None response"
    );
}

#[test]
fn decoder_rejects_invalid_window_bounds_before_transport_admission() {
    struct NoCallTransport(Rc<Cell<usize>>);
    impl WorkerTransport for NoCallTransport {
        fn request(&mut self, _: WorkerCommand) -> Result<(), PortError> {
            self.0.set(self.0.get() + 1);
            Ok(())
        }
        fn receive(&mut self) -> Result<Option<WorkerEvent>, PortError> {
            Ok(None)
        }
    }

    let calls = Rc::new(Cell::new(0));
    for (range, max_samples) in [
        (SourceRange::new(0, 1, 16_000).unwrap(), 0),
        (SourceRange::new(0, 80_001, 16_000).unwrap(), 80_001),
        (
            SourceRange {
                start_sample: 1,
                end_sample: 1,
                sample_rate_hz: 16_000,
            },
            1,
        ),
    ] {
        let request = DecodeRequest {
            source: SourceIdentity {
                source_id: "source.wav".into(),
                sha256: [4; 32],
            },
            job_id: JobId(101),
            generation: Generation::first(),
            range,
            max_samples,
        };
        let mut decoder = DecoderProxy::new(NoCallTransport(calls.clone()));
        assert_eq!(decoder.decode(request), Err(PortError::InvalidInput));
    }
    assert_eq!(calls.get(), 0, "invalid requests must not reach the worker");
}

#[cfg(windows)]
struct MemoryObservation {
    sampler: Child,
    trace_path: PathBuf,
    adapter_trace_path: PathBuf,
    samples_path: PathBuf,
    stop_path: PathBuf,
    finished: bool,
}

#[cfg(windows)]
impl MemoryObservation {
    fn start(_: &std::path::Path) -> Option<Self> {
        use std::os::windows::process::CommandExt;

        let trace_path = std::env::var_os("WHISPER_L01_MEMORY_TRACE").map(PathBuf::from)?;
        let adapter_trace_path = std::env::var_os("WHISPER_L01_ADAPTER_MEMORY_TRACE")
            .map(PathBuf::from)
            .expect("set adapter trace path whenever worker trace is enabled");
        if let Some(parent) = trace_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        if let Some(parent) = adapter_trace_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let samples_path = trace_path.with_extension("process-samples.jsonl");
        let stop_path = trace_path.with_extension("sampler-stop");
        for path in [&trace_path, &adapter_trace_path, &samples_path, &stop_path] {
            let _ = std::fs::remove_file(path);
        }
        let script = r#"
$samplePath = $env:WHISPER_L01_PROCESS_SAMPLES
$stopPath = $env:WHISPER_L01_SAMPLER_STOP
$adapterTracePath = $env:WHISPER_L01_ADAPTER_TRACE
$workerTracePath = $env:WHISPER_L01_WORKER_TRACE
$testPid = [int]$env:WHISPER_L01_TEST_PID
$pipeQueryDone = $false
$workerPipeQueryDone = $false
$pipeApi = @'
using System;
using System.Runtime.InteropServices;
public static class L01PipeApi {
 [DllImport("kernel32.dll", SetLastError=true)] public static extern IntPtr OpenProcess(uint access, bool inherit, int pid);
 [DllImport("kernel32.dll")] public static extern IntPtr GetCurrentProcess();
 [DllImport("kernel32.dll", SetLastError=true)] public static extern bool DuplicateHandle(IntPtr sourceProcess, IntPtr sourceHandle, IntPtr targetProcess, out IntPtr targetHandle, uint access, bool inherit, uint options);
 [DllImport("kernel32.dll", SetLastError=true)] public static extern bool GetNamedPipeInfo(IntPtr pipe, out uint flags, out uint outBuffer, out uint inBuffer, out uint maxInstances);
 [DllImport("kernel32.dll")] public static extern bool CloseHandle(IntPtr handle);
}
'@
Add-Type $pipeApi
$writer = [System.IO.StreamWriter]::new($samplePath, $false, [System.Text.UTF8Encoding]::new($false))
try {
    while (-not (Test-Path -LiteralPath $stopPath)) {
        $now = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds() * 1000
        $hostProcess = Get-Process -Id $testPid -ErrorAction SilentlyContinue
        if ($null -ne $hostProcess) {
            $writer.WriteLine((ConvertTo-Json -Compress -InputObject ([ordered]@{ timestamp_unix_us=$now; role='import_test_host'; pid=$hostProcess.Id; private_bytes=$hostProcess.PrivateMemorySize64; working_set_bytes=$hostProcess.WorkingSet64; peak_working_set_bytes=$hostProcess.PeakWorkingSet64 })))
        }
        if (-not $pipeQueryDone -and (Test-Path -LiteralPath $adapterTracePath)) {
            $spawn = Get-Content -LiteralPath $adapterTracePath | ForEach-Object { try { $_ | ConvertFrom-Json } catch { $null } } | Where-Object { $_.stage -eq 'adapter.worker.spawned' } | Select-Object -First 1
            if ($null -ne $spawn -and $null -ne $spawn.details.stdin_pipe_handle) {
                $sourceProcess = [L01PipeApi]::OpenProcess(0x40, $false, $testPid)
                $currentProcess = [L01PipeApi]::GetCurrentProcess()
                $pipeResults = @()
                foreach ($pipeName in @('stdin_pipe_handle', 'stdout_pipe_handle')) {
                    $rawHandle = [IntPtr]([long]$spawn.details.$pipeName)
                    $duplicate = [IntPtr]::Zero
                    $duplicateOk = [L01PipeApi]::DuplicateHandle($sourceProcess, $rawHandle, $currentProcess, [ref]$duplicate, 0, $false, 2)
                    if ($duplicateOk) {
                        [uint32]$flags=0; [uint32]$outBuffer=0; [uint32]$inBuffer=0; [uint32]$instances=0
                        $queryOk = [L01PipeApi]::GetNamedPipeInfo($duplicate, [ref]$flags, [ref]$outBuffer, [ref]$inBuffer, [ref]$instances)
                        $pipeResults += [ordered]@{ pipe=$pipeName; query_succeeded=$queryOk; output_buffer_bytes=$outBuffer; input_buffer_bytes=$inBuffer; win32_error=$(if ($queryOk) { 0 } else { [Runtime.InteropServices.Marshal]::GetLastWin32Error() }) }
                        [void][L01PipeApi]::CloseHandle($duplicate)
                    } else { $pipeResults += [ordered]@{ pipe=$pipeName; query_succeeded=$false; output_buffer_bytes=$null; input_buffer_bytes=$null; win32_error=[Runtime.InteropServices.Marshal]::GetLastWin32Error() } }
                }
                if ($sourceProcess -ne [IntPtr]::Zero) { [void][L01PipeApi]::CloseHandle($sourceProcess) }
                $writer.WriteLine((ConvertTo-Json -Compress -InputObject ([ordered]@{ timestamp_unix_us=$now; role='kernel_pipe_inventory'; results=$pipeResults })))
                $pipeQueryDone = $true
            }
        }
        if (-not $workerPipeQueryDone -and (Test-Path -LiteralPath $workerTracePath)) {
            $stdinRecord = Get-Content -LiteralPath $workerTracePath | ForEach-Object { try { $_ | ConvertFrom-Json } catch { $null } } | Where-Object { $_.stage -eq 'worker.input.reader.created' } | Select-Object -First 1
            if ($null -ne $stdinRecord -and $null -ne $stdinRecord.details.stdin_read_pipe_handle) {
                $sourceWorker = [L01PipeApi]::OpenProcess(0x40, $false, [int]$stdinRecord.details.worker_pid)
                $currentProcess = [L01PipeApi]::GetCurrentProcess()
                $rawHandle = [IntPtr]([long]$stdinRecord.details.stdin_read_pipe_handle)
                $duplicate = [IntPtr]::Zero
                $duplicateOk = [L01PipeApi]::DuplicateHandle($sourceWorker, $rawHandle, $currentProcess, [ref]$duplicate, 0, $false, 2)
                if ($duplicateOk) {
                    [uint32]$flags=0; [uint32]$outBuffer=0; [uint32]$inBuffer=0; [uint32]$instances=0
                    $queryOk = [L01PipeApi]::GetNamedPipeInfo($duplicate, [ref]$flags, [ref]$outBuffer, [ref]$inBuffer, [ref]$instances)
                    $result = [ordered]@{ role='worker_stdin_kernel_pipe_inventory'; query_succeeded=$queryOk; output_buffer_bytes=$outBuffer; input_buffer_bytes=$inBuffer; win32_error=$(if ($queryOk) { 0 } else { [Runtime.InteropServices.Marshal]::GetLastWin32Error() }) }
                    [void][L01PipeApi]::CloseHandle($duplicate)
                } else {
                    $result = [ordered]@{ role='worker_stdin_kernel_pipe_inventory'; query_succeeded=$false; output_buffer_bytes=$null; input_buffer_bytes=$null; win32_error=[Runtime.InteropServices.Marshal]::GetLastWin32Error() }
                }
                if ($sourceWorker -ne [IntPtr]::Zero) { [void][L01PipeApi]::CloseHandle($sourceWorker) }
                $result['timestamp_unix_us'] = $now
                $writer.WriteLine((ConvertTo-Json -Compress -InputObject $result))
                $workerPipeQueryDone = $true
            }
        }
        $writer.Flush()
        Start-Sleep -Milliseconds 100
    }
} finally { $writer.Dispose() }
"#;
        let sampler = Command::new("powershell.exe")
            .args(["-NoProfile", "-Command", script])
            .env("WHISPER_L01_PROCESS_SAMPLES", &samples_path)
            .env("WHISPER_L01_SAMPLER_STOP", &stop_path)
            .env("WHISPER_L01_ADAPTER_TRACE", &adapter_trace_path)
            .env("WHISPER_L01_WORKER_TRACE", &trace_path)
            .env("WHISPER_L01_TEST_PID", std::process::id().to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(0x0800_0000)
            .spawn()
            .expect("PowerShell is required for Windows process-memory sampling");
        Some(Self {
            sampler,
            trace_path,
            adapter_trace_path,
            samples_path,
            stop_path,
            finished: false,
        })
    }

    fn finish(mut self, expected_windows: usize) {
        std::fs::File::create(&self.stop_path).unwrap();
        assert!(
            self.sampler.wait().unwrap().success(),
            "memory sampler failed"
        );
        self.finished = true;

        let trace = std::fs::read_to_string(&self.trace_path)
            .expect("feature-enabled worker must write stage inventory");
        let trace = trace
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .collect::<Vec<_>>();
        let adapter_trace = std::fs::read_to_string(&self.adapter_trace_path)
            .expect("feature-enabled adapter must write queue and IPC inventory");
        let adapter_trace = adapter_trace
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .collect::<Vec<_>>();
        let samples = std::fs::read_to_string(&self.samples_path)
            .expect("OS sampler must write process samples");
        let samples = samples
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .collect::<Vec<_>>();

        for stage in [
            "model_context.load.begin",
            "native.context.create.complete",
            "codec.ready",
            "codec.sample_buffer.live",
            "pcm.window.callback.begin",
            "inference.begin",
            "native.inference.complete",
            "ipc.serialize.buffer",
            "worker.finalized",
        ] {
            assert!(
                trace.iter().any(|record| record["stage"] == stage),
                "missing memory stage record {stage}"
            );
        }
        for stage in [
            "adapter.channels.created",
            "adapter.worker.spawned",
            "adapter.pending.created",
            "adapter.pending.after_worker_event",
            "adapter.ipc.serialize.buffer",
        ] {
            assert!(
                adapter_trace.iter().any(|record| record["stage"] == stage),
                "missing adapter memory stage record {stage}"
            );
        }
        let channels = adapter_trace
            .iter()
            .find(|record| record["stage"] == "adapter.channels.created")
            .unwrap();
        assert_eq!(channels["details"]["effect_channel_capacity"], 8);
        assert_eq!(channels["details"]["stop_channel_capacity"], 1);
        assert_eq!(channels["details"]["app_event_channel_capacity"], 21);
        let windows = trace
            .iter()
            .filter(|record| record["stage"] == "pcm.window.callback.begin")
            .collect::<Vec<_>>();
        assert_eq!(
            windows.len(),
            expected_windows,
            "unexpected WAV window count"
        );
        let mut expected_start = 0_u64;
        for record in &windows {
            let details = &record["details"];
            assert_eq!(
                details["source_start_sample"].as_u64(),
                Some(expected_start)
            );
            let count = details["outgoing_window_len"].as_u64().unwrap();
            assert!((1..=80_000).contains(&count));
            assert!(details["outgoing_window_capacity"].as_u64().unwrap() <= 80_000);
            assert!(
                details["simultaneous_pcm_window_capacity_bytes"]
                    .as_u64()
                    .unwrap()
                    <= 640_000
            );
            expected_start += count;
        }
        let total = trace
            .iter()
            .find(|record| record["stage"] == "codec.complete")
            .expect("codec completion record")["details"]["output_samples"]
            .as_u64()
            .unwrap();
        assert_eq!(
            expected_start, total,
            "windows cover each source sample once"
        );
        for record in trace
            .iter()
            .filter(|record| record["stage"] == "codec.sample_buffer.live")
        {
            let details = &record["details"];
            assert_eq!(
                details["sample_buffer_capacity_bytes"].as_u64().unwrap(),
                details["sample_buffer_capacity_samples"].as_u64().unwrap() * 4
            );
        }
        for record in trace
            .iter()
            .filter(|record| record["stage"] == "ipc.serialize.buffer")
        {
            assert!(
                record["details"]["frame_len_with_newline_bytes"]
                    .as_u64()
                    .unwrap()
                    <= 1_048_576
            );
        }
        let worker_samples = trace
            .iter()
            .filter(|record| record["details"]["process_memory"]["private_bytes"].is_number())
            .collect::<Vec<_>>();
        assert!(
            worker_samples.len() >= expected_windows * 3,
            "worker memory was not sampled at each inference stage"
        );
        let pipe_inventory = samples
            .iter()
            .find(|sample| sample["role"] == "kernel_pipe_inventory");
        if let Some(inventory) = pipe_inventory {
            eprintln!(
                "V-IMPORT-MEMORY kernel_pipe_inventory={}",
                inventory["results"]
            );
        } else {
            eprintln!(
                "V-IMPORT-MEMORY kernel_pipe_inventory=NOT MEASURED (sampler did not observe child pipe handles)"
            );
        }
        let worker_stdin_pipe = samples
            .iter()
            .find(|sample| sample["role"] == "worker_stdin_kernel_pipe_inventory")
            .expect("worker-side stdin handle must permit kernel pipe query");
        assert_eq!(worker_stdin_pipe["query_succeeded"], true);
        assert_eq!(worker_stdin_pipe["output_buffer_bytes"], 65_536);
        assert_eq!(worker_stdin_pipe["input_buffer_bytes"], 65_536);
        eprintln!(
            "V-IMPORT-MEMORY worker stdin kernel pipe={} bytes per direction",
            worker_stdin_pipe["input_buffer_bytes"]
        );
        for stage in [
            "worker.input.reader.created",
            "model_context.load.complete",
            "codec.ready",
        ] {
            let record = trace
                .iter()
                .find(|record| record["stage"] == stage)
                .unwrap();
            let measured = process_memory_for(record);
            let timestamp = record["elapsed_unix_us"].as_i64().unwrap();
            let sample_delta = (measured["sampled_at_unix_us"].as_i64().unwrap() - timestamp).abs();
            assert!(
                sample_delta <= 1_000_000,
                "no nearby process sample for {stage}"
            );
            eprintln!(
                "V-IMPORT-MEMORY stage={stage} delta_us={sample_delta} private_bytes={} working_set_bytes={} peak_working_set_bytes={}",
                measured["private_bytes"],
                measured["working_set_bytes"],
                measured["peak_working_set_bytes"]
            );
        }
        let state_destroyed = trace
            .iter()
            .filter(|record| record["stage"] == "native.state.destroyed")
            .collect::<Vec<_>>();
        assert_eq!(
            state_destroyed.len(),
            expected_windows,
            "each native WhisperState must have an explicit destruction marker"
        );
        let state_creation_begins = trace
            .iter()
            .filter(|record| record["stage"] == "native.state.create.begin")
            .collect::<Vec<_>>();
        let inference_completes = trace
            .iter()
            .filter(|record| record["stage"] == "native.inference.complete")
            .collect::<Vec<_>>();
        assert_eq!(state_creation_begins.len(), expected_windows);
        assert_eq!(inference_completes.len(), expected_windows);
        let mut post_destroy_private_bytes = Vec::with_capacity(state_destroyed.len());
        for (window_index, ((before, during), destroyed)) in state_creation_begins
            .iter()
            .zip(&inference_completes)
            .zip(&state_destroyed)
            .enumerate()
        {
            let before_sample = process_memory_for(before);
            let during_sample = process_memory_for(during);
            let after_sample = process_memory_for(destroyed);
            let before_at = before["elapsed_unix_us"].as_i64().unwrap();
            let during_at = during["elapsed_unix_us"].as_i64().unwrap();
            let destroyed_at = destroyed["elapsed_unix_us"].as_i64().unwrap();
            let before_delta =
                (before_sample["sampled_at_unix_us"].as_i64().unwrap() - before_at).abs();
            let during_delta =
                (during_sample["sampled_at_unix_us"].as_i64().unwrap() - during_at).abs();
            let after_delta =
                (after_sample["sampled_at_unix_us"].as_i64().unwrap() - destroyed_at).abs();
            assert!(
                before_delta <= 1_000_000,
                "no nearby pre-state sample for window {window_index}"
            );
            assert!(
                during_delta <= 1_000_000,
                "no nearby pre-drop sample for window {window_index}"
            );
            assert!(
                after_delta <= 1_000_000,
                "no nearby post-drop sample for window {window_index}"
            );
            let private_bytes = after_sample["private_bytes"].as_u64().unwrap();
            post_destroy_private_bytes.push(private_bytes);
            let samples = destroyed["details"]["pcm_samples"].as_u64().unwrap();
            assert!((1..=80_000).contains(&samples));
            eprintln!(
                "V-IMPORT-MEMORY inference window={} pcm_samples={} before_private_bytes={} inference_complete_private_bytes={} after_drop_private_bytes={} sample_deltas_us={}/{}/{}",
                window_index + 1,
                samples,
                before_sample["private_bytes"],
                during_sample["private_bytes"],
                private_bytes,
                before_delta,
                during_delta,
                after_delta
            );
        }
        let private_min = *post_destroy_private_bytes.iter().min().unwrap();
        let private_max = *post_destroy_private_bytes.iter().max().unwrap();
        let first_to_last_delta = *post_destroy_private_bytes.last().unwrap() as i128
            - *post_destroy_private_bytes.first().unwrap() as i128;
        let rising_steps = post_destroy_private_bytes
            .windows(2)
            .filter(|pair| pair[1] > pair[0])
            .count();
        let falling_steps = post_destroy_private_bytes
            .windows(2)
            .filter(|pair| pair[1] < pair[0])
            .count();
        eprintln!(
            "V-IMPORT-MEMORY trend=characterization_only post_destroy_private_min={} post_destroy_private_max={} first_to_last_delta={} increasing_steps={} decreasing_steps={}",
            private_min, private_max, first_to_last_delta, rising_steps, falling_steps
        );
        let host_samples = samples
            .iter()
            .filter(|sample| sample["role"] == "import_test_host")
            .collect::<Vec<_>>();
        for stage in [
            "adapter.channels.created",
            "adapter.pending.created",
            "adapter.pending.after_worker_event",
        ] {
            let record = adapter_trace
                .iter()
                .find(|record| record["stage"] == stage)
                .unwrap();
            let timestamp = record["elapsed_unix_us"].as_i64().unwrap();
            let nearest = host_samples
                .iter()
                .min_by_key(|sample| {
                    (sample["timestamp_unix_us"].as_i64().unwrap() - timestamp).abs()
                })
                .unwrap();
            eprintln!(
                "V-IMPORT-MEMORY stage={stage} host_private_bytes={} host_working_set_bytes={} host_peak_working_set_bytes={}",
                nearest["private_bytes"],
                nearest["working_set_bytes"],
                nearest["peak_working_set_bytes"]
            );
        }
        eprintln!(
            "V-IMPORT-MEMORY characterization only: worker_samples={} host_samples={} adapter_records={} trace={} adapter_trace={} process_samples={}; native allocator and kernel pipe capacities remain un-attributed",
            worker_samples.len(),
            host_samples.len(),
            adapter_trace.len(),
            self.trace_path.display(),
            self.adapter_trace_path.display(),
            self.samples_path.display()
        );
    }
}

#[cfg(windows)]
fn process_memory_for(record: &serde_json::Value) -> &serde_json::Value {
    let stage = record["stage"].as_str().unwrap();
    let sample = &record["details"]["process_memory"];
    assert!(
        sample["private_bytes"].is_number(),
        "no process sample recorded at {stage}: {sample}"
    );
    sample
}

#[cfg(windows)]
impl Drop for MemoryObservation {
    fn drop(&mut self) {
        if !self.finished {
            let _ = std::fs::File::create(&self.stop_path);
            let _ = self.sampler.kill();
            let _ = self.sampler.wait();
        }
    }
}

#[cfg(not(windows))]
struct MemoryObservation;

#[cfg(not(windows))]
impl MemoryObservation {
    fn start(_: &std::path::Path) -> Option<Self> {
        None
    }

    fn finish(self, _: usize) {}
}

fn parse_hash(text: &str) -> [u8; 32] {
    let mut output = [0_u8; 32];
    for (i, pair) in text.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        output[i] = (digit(pair[0]) << 4) | digit(pair[1]);
    }
    output
}
fn digit(value: u8) -> u8 {
    match value {
        b'0'..=b'9' => value - b'0',
        b'a'..=b'f' => value - b'a' + 10,
        _ => 0,
    }
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
