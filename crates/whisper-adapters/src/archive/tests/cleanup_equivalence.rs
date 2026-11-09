use super::super::*;
use super::*;
use std::collections::{HashMap, HashSet};

fn cleanup_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap()
        .to_string_lossy()
        .into_owned()
}

fn cleanup_normalization_roots(root: &Path) -> [String; 2] {
    [
        root.to_string_lossy().into_owned(),
        root.canonicalize().unwrap().to_string_lossy().into_owned(),
    ]
}

fn normalize_cleanup_json(root: &Path, value: &mut serde_json::Value) {
    fn normalize(
        value: &mut serde_json::Value,
        roots: &[String],
        identities: &mut HashMap<(u64, Vec<u8>, bool), u64>,
        volumes: &mut HashMap<u64, u64>,
    ) {
        match value {
            serde_json::Value::Object(object) => {
                if let (Some(volume), Some(file_id), Some(directory)) = (
                    object
                        .get("volume_serial")
                        .and_then(serde_json::Value::as_u64),
                    object.get("file_id").and_then(serde_json::Value::as_array),
                    object
                        .get("is_directory")
                        .and_then(serde_json::Value::as_bool),
                ) {
                    let bytes = file_id
                        .iter()
                        .filter_map(serde_json::Value::as_u64)
                        .map(|byte| byte as u8)
                        .collect::<Vec<_>>();
                    let next = identities.len() as u64 + 1;
                    let normalized = *identities.entry((volume, bytes, directory)).or_insert(next);
                    let volume_next = volumes.len() as u64 + 1;
                    let normalized_volume = *volumes.entry(volume).or_insert(volume_next);
                    let mut normalized_file_id = [0_u8; 16];
                    normalized_file_id[..8].copy_from_slice(&normalized.to_le_bytes());
                    object.insert("volume_serial".into(), serde_json::json!(normalized_volume));
                    object.insert(
                        "file_id".into(),
                        serde_json::to_value(normalized_file_id).unwrap(),
                    );
                }
                for child in object.values_mut() {
                    normalize(child, roots, identities, volumes);
                }
            }
            serde_json::Value::Array(array) => {
                for child in array {
                    normalize(child, roots, identities, volumes);
                }
            }
            serde_json::Value::String(text) => {
                for root in roots {
                    *text = text.replace(root, "<ROOT>");
                }
            }
            _ => {}
        }
    }

    normalize(
        value,
        &cleanup_normalization_roots(root),
        &mut HashMap::new(),
        &mut HashMap::new(),
    );
}

fn cleanup_tree_snapshot(root: &Path) -> Vec<serde_json::Value> {
    cleanup_tree_snapshot_with_extra_paths(root, &[])
}

fn cleanup_tree_snapshot_with_extra_paths(
    root: &Path,
    extra_paths: &[PathBuf],
) -> Vec<serde_json::Value> {
    fn verify_no_unlisted_entries(directory: &Path, known_paths: &HashSet<PathBuf>) {
        for entry in fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            assert!(
                known_paths.contains(&path),
                "fixture snapshot omitted an entry: {}",
                path.display()
            );
            if entry.file_type().unwrap().is_dir() {
                verify_no_unlisted_entries(&path, known_paths);
            }
        }
    }

    let successor = generation_path(root, JobId(0x7401), Generation::first());
    let old_generation = generation_path(root, JobId(0x7402), Generation::first());
    let cleanup_dir = cleanup_plans_path(root);
    let mut paths = [
        root.to_path_buf(),
        root.join("source.wav"),
        root.join("source.parts.idx"),
        root.join("exports"),
        root.join("exports/cleanup-sentinel.txt"),
        root.join("exports/cleanup-sentinel.srt"),
        root.join("transcriptions"),
        old_generation.parent().unwrap().to_path_buf(),
        old_generation.clone(),
        old_generation.join("owned.txt"),
        old_generation.join("unselected.txt"),
        old_generation.join("added-after-selection.txt"),
        successor.parent().unwrap().to_path_buf(),
        successor.clone(),
        successor.join("manifest.json"),
        successor.join("transcript.txt"),
        successor.join("transcript.srt"),
        successor.join("pending.json"),
        successor.join("segments.jsonl"),
        successor.join("CURRENT"),
        successor.join("r11-publication.json"),
        root.join(".whisper-publications"),
        root.join(".whisper-publications.lock"),
        publication_index_path(root),
        cleanup_dir.clone(),
        cleanup_plan_path(root, "cleanup-equivalence"),
    ]
    .to_vec();
    paths.extend(extra_paths.iter().cloned());
    let known_paths = paths.iter().cloned().collect::<HashSet<_>>();
    verify_no_unlisted_entries(root, &known_paths);

    paths
        .iter()
        .map(|path| cleanup_path_snapshot(root, path))
        .collect()
}

fn cleanup_path_snapshot(root: &Path, path: &Path) -> serde_json::Value {
    let relative_path = cleanup_relative(root, path);
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return serde_json::json!({ "path": relative_path, "present": false });
        }
        Err(error) => panic!("snapshot {}: {error}", path.display()),
    };
    let (bytes, sha256, raw_bytes_hex, raw_json_size, raw_json_sha256, raw_json_hex) =
        if !metadata.is_file() {
            (None, None, None, None, None, None)
        } else if path.extension().and_then(|value| value.to_str()) == Some("json") {
            let raw = fs::read(path).unwrap();
            assert_eq!(
                metadata.len(),
                raw.len() as u64,
                "raw JSON size: {}",
                path.display()
            );
            let raw_json_size = Some(raw.len() as u64);
            let raw_json_sha256 = Some(hex(&sha256_file(path).unwrap()));
            let raw_json_hex = Some(hex(&raw));
            match serde_json::from_slice::<serde_json::Value>(&raw) {
                Ok(mut value) => {
                    normalize_cleanup_json(root, &mut value);
                    (
                        Some(serde_json::to_vec(&value).unwrap().len() as u64),
                        None,
                        None,
                        raw_json_size,
                        raw_json_sha256,
                        raw_json_hex,
                    )
                }
                Err(_) => (
                    Some(raw.len() as u64),
                    Some(hex(&sha256_file(path).unwrap())),
                    Some(hex(&raw)),
                    raw_json_size,
                    raw_json_sha256,
                    raw_json_hex,
                ),
            }
        } else {
            (
                Some(metadata.len()),
                Some(hex(&sha256_file(path).unwrap())),
                None,
                None,
                None,
                None,
            )
        };
    serde_json::json!({
        "path": relative_path,
        "present": true,
        "kind": if metadata.is_dir() { "directory" } else { "file" },
        "bytes": bytes,
        "sha256": sha256,
        "raw_bytes_hex": raw_bytes_hex,
        "raw_json_size": raw_json_size,
        "raw_json_sha256": raw_json_sha256,
        "raw_json_hex": raw_json_hex,
    })
}

struct CleanupSentinelBytes {
    path: PathBuf,
    bytes: Vec<u8>,
}

fn capture_cleanup_sentinels(root: &Path, plan: &CleanupPlan) -> Vec<CleanupSentinelBytes> {
    let successor = generation_path(root, plan.successor.job_id, plan.successor.generation);
    let paths = [
        PathBuf::from(&plan.source_path),
        successor.join("manifest.json"),
        successor.join("pending.json"),
        successor.join("segments.jsonl"),
        successor.join("transcript.txt"),
        successor.join("transcript.srt"),
        successor.join("CURRENT"),
        successor.join("r11-publication.json"),
        publication_index_path(root),
        root.join("exports/cleanup-sentinel.txt"),
        root.join("exports/cleanup-sentinel.srt"),
    ];
    paths
        .into_iter()
        .map(|path| CleanupSentinelBytes {
            bytes: fs::read(&path)
                .unwrap_or_else(|error| panic!("read sentinel {}: {error}", path.display())),
            path,
        })
        .collect()
}

fn assert_cleanup_sentinels(sentinels: &[CleanupSentinelBytes]) {
    for sentinel in sentinels {
        assert_eq!(
            fs::read(&sentinel.path).unwrap(),
            sentinel.bytes,
            "sentinel bytes changed: {}",
            sentinel.path.display()
        );
    }
}

fn raw_cleanup_plan(_root: &Path, plan: &CleanupPlan) -> serde_json::Value {
    serde_json::to_value(plan).unwrap()
}

fn write_cleanup_trace(root: &Path, trace: &serde_json::Value) {
    let raw_bytes = serde_json::to_vec_pretty(trace).unwrap();
    fs::write(root.join("cleanup-equivalence-trace.raw.json"), raw_bytes).unwrap();
    let mut stable_trace = trace.clone();
    strip_raw_json_contents(&mut stable_trace);
    normalize_cleanup_json(root, &mut stable_trace);
    if stable_trace["scenario"] == "D04-locked-file-then-retry" {
        let error = stable_trace["first_resume"]["result"]
            .as_str()
            .expect("D04 trace stores the first resume error")
            .to_owned();
        let normalized = [
            root.to_string_lossy().into_owned(),
            root.canonicalize().unwrap().to_string_lossy().into_owned(),
        ]
        .into_iter()
        .fold(error, |text, root| text.replace(&root, "<ROOT>"));
        stable_trace["first_resume"]["result"] = serde_json::Value::String(normalized);
    }
    let bytes = serde_json::to_vec_pretty(&stable_trace).unwrap();
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert!(
        !text.contains(&root.to_string_lossy().to_string()),
        "the stable trace must not contain its temporary root"
    );
    let references: HashMap<String, String> = serde_json::from_str(include_str!(
        "../../../tests/fixtures/cleanup_equivalence/stable-traces.json"
    ))
    .expect("frozen cleanup trace references");
    let scenario = stable_trace["scenario"]
        .as_str()
        .expect("cleanup trace has a scenario name");
    assert_eq!(
        references.get(scenario).map(String::as_str),
        Some(text.as_str()),
        "cleanup trace differs from the frozen T1 reference for {scenario}"
    );
    fs::write(root.join("cleanup-equivalence-trace.json"), bytes).unwrap();
}

fn write_additional_cleanup_trace(root: &Path, trace: &serde_json::Value) {
    let raw = serde_json::to_vec_pretty(trace).unwrap();
    fs::write(root.join("cleanup-equivalence-trace.raw.json"), raw).unwrap();
    let mut stable = trace.clone();
    strip_raw_json_contents(&mut stable);
    normalize_cleanup_json(root, &mut stable);
    let bytes = serde_json::to_vec_pretty(&stable).unwrap();
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert!(
        !text.contains(&root.to_string_lossy().to_string()),
        "the stable trace must not contain its temporary root"
    );
    let references: HashMap<String, String> = serde_json::from_str(include_str!(
        "../../../tests/fixtures/cleanup_equivalence/preflight-traces.json"
    ))
    .expect("additional cleanup trace references");
    let scenario = stable["scenario"]
        .as_str()
        .expect("additional cleanup trace has a scenario name");
    assert_eq!(
        references.get(scenario).map(String::as_str),
        Some(text.as_str()),
        "cleanup trace differs from its additional frozen T1 reference for {scenario}"
    );
    fs::write(root.join("cleanup-equivalence-trace.json"), bytes).unwrap();
}

fn write_progress_cleanup_trace(root: &Path, trace: &serde_json::Value) {
    let raw = serde_json::to_vec_pretty(trace).unwrap();
    fs::write(root.join("cleanup-equivalence-trace.raw.json"), raw).unwrap();
    let mut stable = trace.clone();
    strip_raw_json_contents(&mut stable);
    normalize_cleanup_json(root, &mut stable);
    let bytes = serde_json::to_vec_pretty(&stable).unwrap();
    assert!(
        !String::from_utf8_lossy(&bytes).contains(&root.to_string_lossy().to_string()),
        "the stable trace must not contain its temporary root"
    );
    let references: HashMap<String, String> = serde_json::from_str(include_str!(
        "../../../tests/fixtures/cleanup_equivalence/progress-traces.json"
    ))
    .expect("cleanup progress trace references");
    let scenario = stable["scenario"]
        .as_str()
        .expect("cleanup progress trace has a scenario name");
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert_eq!(
        references.get(scenario).map(String::as_str),
        Some(text.as_str()),
        "cleanup trace differs from its progress T1 reference for {scenario}"
    );
    fs::write(root.join("cleanup-equivalence-trace.json"), bytes).unwrap();
}

fn write_rejection_cleanup_trace(root: &Path, trace: &serde_json::Value) {
    let raw = serde_json::to_vec_pretty(trace).unwrap();
    fs::write(root.join("cleanup-equivalence-trace.raw.json"), raw).unwrap();

    let mut stable = trace.clone();
    strip_raw_json_contents(&mut stable);
    normalize_cleanup_json(root, &mut stable);
    let bytes = serde_json::to_vec_pretty(&stable).unwrap();
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert!(
        !text.contains(&root.to_string_lossy().to_string()),
        "the stable trace must not contain its temporary root"
    );
    let references: HashMap<String, String> = serde_json::from_str(include_str!(
        "../../../tests/fixtures/cleanup_equivalence/final-traces.json"
    ))
    .expect("final cleanup trace references");
    let scenario = stable["scenario"]
        .as_str()
        .expect("final cleanup trace has a scenario name");
    assert_eq!(
        references.get(scenario).map(String::as_str),
        Some(text.as_str()),
        "cleanup trace differs from its final T1 reference for {scenario}"
    );
    fs::write(root.join("cleanup-equivalence-trace.json"), bytes).unwrap();
}

fn write_c02_cleanup_trace(root: &Path, trace: &serde_json::Value) {
    let raw = serde_json::to_vec_pretty(trace).unwrap();
    fs::write(root.join("cleanup-equivalence-trace.raw.json"), raw).unwrap();

    let mut stable = trace.clone();
    strip_raw_json_contents(&mut stable);
    normalize_cleanup_json(root, &mut stable);
    let bytes = serde_json::to_vec_pretty(&stable).unwrap();
    assert!(
        !String::from_utf8_lossy(&bytes).contains(&root.to_string_lossy().to_string()),
        "the stable trace must not contain its temporary root"
    );
    let references: HashMap<String, String> = serde_json::from_str(include_str!(
        "../../../tests/fixtures/cleanup_equivalence/c02-traces.json"
    ))
    .expect("C02 cleanup trace references");
    let scenario = stable["scenario"]
        .as_str()
        .expect("C02 cleanup trace has a scenario name");
    let text = String::from_utf8(bytes.clone()).unwrap();
    assert_eq!(
        references.get(scenario).map(String::as_str),
        Some(text.as_str()),
        "cleanup trace differs from its C02 reference for {scenario}"
    );
    fs::write(root.join("cleanup-equivalence-trace.json"), bytes).unwrap();
}

fn strip_raw_json_contents(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(object) => {
            object.remove("raw_json_size");
            object.remove("raw_json_sha256");
            object.remove("raw_json_hex");
            for child in object.values_mut() {
                strip_raw_json_contents(child);
            }
        }
        serde_json::Value::Array(array) => {
            for child in array {
                strip_raw_json_contents(child);
            }
        }
        _ => {}
    }
}

fn write_single_resume_trace(
    root: &Path,
    scenario: &str,
    plan: &CleanupPlan,
    tree_before: Vec<serde_json::Value>,
    result: &str,
    sentinels: &[CleanupSentinelBytes],
) {
    assert_cleanup_sentinels(sentinels);
    let tree_after = cleanup_tree_snapshot(root);
    let persisted_after = read_cleanup_plan(root, &plan.cleanup_id).unwrap();
    write_cleanup_trace(
        root,
        &serde_json::json!({
            "scenario": scenario,
            "tree_before": tree_before,
            "memory_plan_before": raw_cleanup_plan(root, plan),
            "persisted_plan_before": raw_cleanup_plan(root, plan),
            "operation": result,
            "tree_after": tree_after,
            "memory_plan_after": null,
            "persisted_plan_after": raw_cleanup_plan(root, &persisted_after),
        }),
    );
}

fn cleanup_source_fixture(root: &Path) -> (PathBuf, String) {
    let source_path = root.join("source.wav");
    let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_path).unwrap();
    writer.append(&vec![0_i16; 160]).unwrap();
    writer.finish().unwrap();
    let source_sha256 = hex(&sha256_file(&source_path).unwrap());
    fs::create_dir_all(root.join("exports")).unwrap();
    fs::write(
        root.join("exports/cleanup-sentinel.txt"),
        b"keep export text\n",
    )
    .unwrap();
    fs::write(
        root.join("exports/cleanup-sentinel.srt"),
        b"keep export subtitle\n",
    )
    .unwrap();
    (source_path, source_sha256)
}

struct CleanupFixtureSuccessor {
    treatment: TreatmentIdentity,
    path: PathBuf,
    identity: CleanupObjectIdentity,
    files: Vec<CleanupObject>,
    index: CleanupObject,
}

fn cleanup_successor_fixture(root: &Path, source_sha256: &str) -> CleanupFixtureSuccessor {
    use crate::windows_file_identity::inspect;

    let successor = TreatmentIdentity {
        destination: root.canonicalize().unwrap().to_string_lossy().into_owned(),
        job_id: JobId(0x7401),
        generation: Generation::first(),
    };
    let successor_path = generation_path(root, successor.job_id, successor.generation);
    fs::create_dir_all(&successor_path).unwrap();
    fs::write(successor_path.join("transcript.txt"), b"new transcript").unwrap();
    fs::write(successor_path.join("transcript.srt"), b"new subtitle\n").unwrap();
    fs::write(successor_path.join("segments.jsonl"), b"").unwrap();
    let pending = PendingRecord {
        version: 4,
        job_id: successor.job_id,
        generation: successor.generation,
        source_path: "source.wav".into(),
        source_sha256: source_sha256.into(),
        source_samples: 160,
        model_path: "synthetic-model.gguf".into(),
        model_sha256: APPROVED_MODEL_SHA256.into(),
        model_size: APPROVED_MODEL_SIZE,
        strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
        qualification: None,
    };
    fs::write(
        successor_path.join("pending.json"),
        serde_json::to_vec(&pending).unwrap(),
    )
    .unwrap();
    let text = verified_artifact(&successor_path, &successor_path.join("transcript.txt")).unwrap();
    let srt = verified_artifact(&successor_path, &successor_path.join("transcript.srt")).unwrap();
    fs::write(
        successor_path.join("manifest.json"),
        serde_json::to_vec(&CompleteRecord {
            version: 1,
            job_id: successor.job_id,
            generation: successor.generation,
            source_sha256: source_sha256.into(),
            segments: 0,
            text,
            srt,
        })
        .unwrap(),
    )
    .unwrap();
    let cleanup_id = "cleanup-equivalence";
    fs::write(
        successor_path.join("r11-publication.json"),
        serde_json::to_vec(&PublicationIntent {
            version: 1,
            operation_id: cleanup_id.into(),
            choice: DuplicateImportChoice::Replace,
            source_sha256: source_sha256.into(),
            source_samples: 160,
            expected_active: Vec::new(),
            export_basename: "fixture".into(),
            internal_only: true,
            cleanup_id: Some(cleanup_id.into()),
            cleanup_targets: Vec::new(),
        })
        .unwrap(),
    )
    .unwrap();
    fs::write(successor_path.join("CURRENT"), b"manifest.json\n").unwrap();
    let index = PublicationIndex {
        version: 1,
        records: vec![PublicationRecord {
            job_id: successor.job_id,
            generation: successor.generation,
            source_sha256: source_sha256.into(),
            export_basename: Some("fixture".into()),
            active: true,
            success_ordinal: Some(1),
        }],
    };
    write_publication_index(root, &index).unwrap();

    let mut files = capture_successor_files(root, &successor_path).unwrap();
    files.push(capture_cleanup_file(root, &successor_path.join("CURRENT")).unwrap());
    let successor_identity = identity_snapshot(inspect(&successor_path, true).unwrap()).unwrap();
    let index_proof = capture_cleanup_file(root, &publication_index_path(root)).unwrap();
    CleanupFixtureSuccessor {
        treatment: successor,
        path: successor_path,
        identity: successor_identity,
        files,
        index: index_proof,
    }
}

fn cleanup_old_target_fixture(
    root: &Path,
    successor: &TreatmentIdentity,
    source_sha256: &str,
    unowned_obstacle: bool,
) -> CleanupTarget {
    use crate::windows_file_identity::inspect;

    let old_job = JobId(0x7402);
    let old_directory = generation_path(root, old_job, Generation::first());
    fs::create_dir_all(&old_directory).unwrap();
    let owned_file = old_directory.join("owned.txt");
    fs::write(&owned_file, b"owned old artifact").unwrap();
    let mut objects = vec![CleanupObject {
        relative_path: cleanup_relative(root, &owned_file),
        identity: identity_snapshot(inspect(&owned_file, false).unwrap()).unwrap(),
        sha256: Some(hex(&sha256_file(&owned_file).unwrap())),
        owned: true,
        removed: false,
        error: None,
    }];
    if unowned_obstacle {
        let obstacle = old_directory.join("unselected.txt");
        fs::write(&obstacle, b"third party").unwrap();
        objects.push(CleanupObject {
            relative_path: cleanup_relative(root, &obstacle),
            identity: identity_snapshot(inspect(&obstacle, false).unwrap()).unwrap(),
            sha256: Some(hex(&sha256_file(&obstacle).unwrap())),
            owned: false,
            removed: false,
            error: Some("fixture obstacle".into()),
        });
    }
    objects.push(CleanupObject {
        relative_path: cleanup_relative(root, &old_directory),
        identity: identity_snapshot(inspect(&old_directory, true).unwrap()).unwrap(),
        sha256: None,
        owned: true,
        removed: false,
        error: None,
    });
    let old_container = old_directory.parent().unwrap();
    CleanupTarget {
        treatment: TreatmentIdentity {
            destination: successor.destination.clone(),
            job_id: old_job,
            generation: Generation::first(),
        },
        source_sha256: source_sha256.into(),
        ancestors: [root.join("transcriptions"), old_container.to_path_buf()]
            .into_iter()
            .map(|path| CleanupAncestor {
                relative_path: cleanup_relative(root, &path),
                identity: identity_snapshot(inspect(&path, true).unwrap()).unwrap(),
            })
            .collect(),
        objects,
        container: Some(CleanupObject {
            relative_path: cleanup_relative(root, old_container),
            identity: identity_snapshot(inspect(old_container, true).unwrap()).unwrap(),
            sha256: None,
            owned: true,
            removed: false,
            error: None,
        }),
    }
}

fn cleanup_fixture(root: &Path, unowned_obstacle: bool) -> CleanupPlan {
    let (source_path, source_sha256) = cleanup_source_fixture(root);
    let successor = cleanup_successor_fixture(root, &source_sha256);
    let target =
        cleanup_old_target_fixture(root, &successor.treatment, &source_sha256, unowned_obstacle);
    let targets = vec![target];
    let intent_path = successor.path.join("r11-publication.json");
    let mut intent: PublicationIntent =
        serde_json::from_slice(&fs::read(intent_path).unwrap()).unwrap();
    intent.cleanup_targets = targets.clone();
    fs::write(
        successor.path.join("r11-publication.json"),
        serde_json::to_vec(&intent).unwrap(),
    )
    .unwrap();
    let plan = CleanupPlan {
        version: 1,
        cleanup_id: "cleanup-equivalence".into(),
        destination: successor.treatment.destination.clone(),
        source_path: source_path.to_string_lossy().into_owned(),
        source_sha256,
        successor: successor.treatment,
        successor_directory: successor.identity,
        successor_files: successor.files,
        successor_index: Some(successor.index),
        published: true,
        targets,
        last_error: None,
    };
    write_cleanup_plan(root, &plan).unwrap();
    plan
}

fn cleanup_fixture_without_targets(root: &Path) -> CleanupPlan {
    let mut plan = cleanup_fixture(root, false);
    plan.targets.clear();
    let successor_path = generation_path(root, plan.successor.job_id, plan.successor.generation);
    let intent_path = successor_path.join("r11-publication.json");
    let mut intent: PublicationIntent =
        serde_json::from_slice(&fs::read(&intent_path).unwrap()).unwrap();
    intent.cleanup_targets.clear();
    fs::write(&intent_path, serde_json::to_vec(&intent).unwrap()).unwrap();
    plan.successor_files = capture_successor_files(root, &successor_path).unwrap();
    plan.successor_files
        .push(capture_cleanup_file(root, &successor_path.join("CURRENT")).unwrap());
    write_cleanup_plan(root, &plan).unwrap();
    plan
}

fn assert_cleanup_preflight_rejection(root: &Path, plan: &CleanupPlan, expected_error: &str) {
    let sentinels = capture_cleanup_sentinels(root, plan);
    let tree_before = cleanup_tree_snapshot(root);
    let memory_before = raw_cleanup_plan(root, plan);
    write_cleanup_plan(root, plan).unwrap();
    let target_file = root.join(&plan.targets[0].objects[0].relative_path);
    let first = ArchiveStore::resume_replacement_cleanup(root, &plan.cleanup_id).unwrap_err();
    assert!(first.contains(expected_error), "{first}");
    assert!(
        target_file.is_file(),
        "preflight rejection must precede removal"
    );
    let persisted = read_cleanup_plan(root, &plan.cleanup_id).unwrap();
    assert_eq!(persisted.last_error.as_deref(), Some(first.as_str()));
    assert_cleanup_sentinels(&sentinels);
    write_cleanup_trace(
        root,
        &serde_json::json!({
            "scenario": expected_error,
            "tree_before": tree_before,
            "memory_plan_before": memory_before,
            "first_resume": {
                "result": first,
                "tree_after": cleanup_tree_snapshot(root),
                "persisted_plan_after": raw_cleanup_plan(root, &persisted),
            },
        }),
    );
}

fn record_additional_preflight_rejection(
    root: &Path,
    plan: &CleanupPlan,
    expected_error: &str,
) -> serde_json::Value {
    let sentinels = capture_cleanup_sentinels(root, plan);
    let tree_before = cleanup_tree_snapshot(root);
    let memory_before = raw_cleanup_plan(root, plan);
    let persisted_before =
        raw_cleanup_plan(root, &read_cleanup_plan(root, &plan.cleanup_id).unwrap());
    write_cleanup_plan(root, plan).unwrap();
    let target = root.join(&plan.targets[0].objects[0].relative_path);
    let error = ArchiveStore::resume_replacement_cleanup(root, &plan.cleanup_id).unwrap_err();
    assert!(
        error.contains(expected_error),
        "expected {expected_error:?}, got {error:?}"
    );
    assert!(target.is_file(), "preflight rejection must precede removal");
    assert_cleanup_sentinels(&sentinels);
    let persisted_after = read_cleanup_plan(root, &plan.cleanup_id).unwrap();
    serde_json::json!({
        "expected_error": expected_error,
        "result": error,
        "tree_before": tree_before,
        "memory_plan_before": memory_before,
        "persisted_plan_before": persisted_before,
        "tree_after": cleanup_tree_snapshot(root),
        "persisted_plan_after": raw_cleanup_plan(root, &persisted_after),
    })
}

fn record_locked_cleanup_rejection(
    root: &Path,
    plan: &mut CleanupPlan,
    expected_error: &str,
) -> serde_json::Value {
    let sentinels = capture_cleanup_sentinels(root, plan);
    let tree_before = cleanup_tree_snapshot(root);
    let memory_before = raw_cleanup_plan(root, plan);
    let persisted_before =
        raw_cleanup_plan(root, &read_cleanup_plan(root, &plan.cleanup_id).unwrap());
    let error = {
        let _lock = ArchiveStore::lock_publications(root).unwrap();
        super::super::cleanup::run_cleanup_plan_locked(root, plan).unwrap_err()
    };
    assert!(
        error.contains(expected_error),
        "expected {expected_error:?}, got {error:?}"
    );
    assert!(
        root.join(&plan.targets[0].objects[0].relative_path)
            .is_file(),
        "preflight rejection must precede removal"
    );
    assert_cleanup_sentinels(&sentinels);
    let persisted_after = read_cleanup_plan(root, &plan.cleanup_id).unwrap();
    serde_json::json!({
        "expected_error": expected_error,
        "result": error,
        "tree_before": tree_before,
        "memory_plan_before": memory_before,
        "persisted_plan_before": persisted_before,
        "tree_after": cleanup_tree_snapshot(root),
        "memory_plan_after": raw_cleanup_plan(root, plan),
        "persisted_plan_after": raw_cleanup_plan(root, &persisted_after),
    })
}

fn sync_cleanup_intent_targets(root: &Path, plan: &CleanupPlan) {
    let intent_path = generation_path(root, plan.successor.job_id, plan.successor.generation)
        .join("r11-publication.json");
    let mut intent: PublicationIntent =
        serde_json::from_slice(&fs::read(&intent_path).unwrap()).unwrap();
    intent.cleanup_targets = plan.targets.clone();
    fs::write(&intent_path, serde_json::to_vec(&intent).unwrap()).unwrap();
}

#[test]
fn cleanup_characterization_rejects_incomplete_successor_proofs_before_removal() {
    let root = archive_characterization_root("cleanup-c02");
    let mut plan = cleanup_fixture(&root, false);
    plan.successor_files.pop();
    assert_cleanup_preflight_rejection(&root, &plan, "preuve durable du successeur incomplète");
}

#[test]
fn cleanup_characterization_rejects_successor_directory_with_another_real_identity() {
    let root = archive_characterization_root("cleanup-c02-successor-directory-identity");
    let mut plan = cleanup_fixture(&root, false);
    let successor_path = generation_path(&root, plan.successor.job_id, plan.successor.generation);
    let actual_identity =
        identity_snapshot(crate::windows_file_identity::inspect(&successor_path, true).unwrap())
            .unwrap();
    let foreign_identity = identity_snapshot(
        crate::windows_file_identity::inspect(&root.join("exports"), true).unwrap(),
    )
    .unwrap();
    assert_ne!(actual_identity, foreign_identity);
    plan.successor_directory = foreign_identity;

    let case = record_additional_preflight_rejection(&root, &plan, "CleanupIdentityChanged:");

    write_c02_cleanup_trace(
        &root,
        &serde_json::json!({
            "scenario": "C02-successor-directory-has-another-real-identity",
            "case": case,
        }),
    );
}

#[test]
fn cleanup_characterization_locked_executor_rejects_moved_successor_directory() {
    let root = archive_characterization_root("cleanup-c02-successor-directory-moved");
    let mut plan = cleanup_fixture(&root, false);
    let successor_path = generation_path(&root, plan.successor.job_id, plan.successor.generation);
    let moved_successor = successor_path.with_file_name("1-moved");
    fs::rename(&successor_path, &moved_successor).unwrap();
    let extra_paths = [
        moved_successor.clone(),
        moved_successor.join("manifest.json"),
        moved_successor.join("transcript.txt"),
        moved_successor.join("transcript.srt"),
        moved_successor.join("pending.json"),
        moved_successor.join("segments.jsonl"),
        moved_successor.join("CURRENT"),
        moved_successor.join("r11-publication.json"),
    ];
    drop(ArchiveStore::lock_publications(&root).unwrap());
    let tree_before = cleanup_tree_snapshot_with_extra_paths(&root, &extra_paths);
    let memory_plan_before = raw_cleanup_plan(&root, &plan);
    let persisted_plan_before =
        raw_cleanup_plan(&root, &read_cleanup_plan(&root, &plan.cleanup_id).unwrap());
    let target = root.join(&plan.targets[0].objects[0].relative_path);

    let error = {
        let _lock = ArchiveStore::lock_publications(&root).unwrap();
        super::super::cleanup::run_cleanup_plan_locked(&root, &mut plan).unwrap_err()
    };

    assert_eq!(
        error,
        "CleanupIdentityChanged: le nouveau traitement a été déplacé"
    );
    assert!(
        target.is_file(),
        "the moved successor is rejected before removal"
    );
    let persisted_plan_after = read_cleanup_plan(&root, &plan.cleanup_id).unwrap();
    let tree_after = cleanup_tree_snapshot_with_extra_paths(&root, &extra_paths);
    let memory_plan_after = raw_cleanup_plan(&root, &plan);
    let persisted_plan_after = raw_cleanup_plan(&root, &persisted_plan_after);
    assert_eq!(tree_before, tree_after);
    assert_eq!(memory_plan_before, memory_plan_after);
    assert_eq!(persisted_plan_before, persisted_plan_after);

    write_c02_cleanup_trace(
        &root,
        &serde_json::json!({
            "scenario": "C02-successor-directory-canonical-path-diverges",
            "result": error,
            "tree_before": tree_before,
            "memory_plan_before": memory_plan_before,
            "persisted_plan_before": persisted_plan_before,
            "tree_after": tree_after,
            "memory_plan_after": memory_plan_after,
            "persisted_plan_after": persisted_plan_after,
        }),
    );
}

#[test]
fn cleanup_characterization_rejects_unexpected_successor_proof_path_before_removal() {
    let root = archive_characterization_root("cleanup-c03");
    let mut plan = cleanup_fixture(&root, false);
    plan.successor_files[0].relative_path = "transcriptions/not-the-manifest.json".into();
    assert_cleanup_preflight_rejection(&root, &plan, "chemin de preuve successeur inattendu");
}

#[test]
fn cleanup_characterization_rejects_changed_current_bytes_before_removal() {
    let root = archive_characterization_root("cleanup-c03-current");
    let mut plan = cleanup_fixture(&root, false);
    let current =
        generation_path(&root, plan.successor.job_id, plan.successor.generation).join("CURRENT");
    fs::write(&current, b"other.json\n").unwrap();
    plan.successor_files[5] = capture_cleanup_file(&root, &current).unwrap();
    write_cleanup_plan(&root, &plan).unwrap();
    let old_target = root.join(&plan.targets[0].objects[0].relative_path);

    let error = ArchiveStore::resume_replacement_cleanup(&root, &plan.cleanup_id).unwrap_err();

    assert!(error.contains("CURRENT ne référence"), "{error}");
    assert!(old_target.is_file());
}

#[test]
fn cleanup_characterization_rejects_truncated_successor_segments_before_removal() {
    let root = archive_characterization_root("cleanup-c04");
    let mut plan = cleanup_fixture(&root, false);
    let segments = generation_path(&root, plan.successor.job_id, plan.successor.generation)
        .join("segments.jsonl");
    fs::write(&segments, b"{truncated").unwrap();
    plan.successor_files[4] = capture_cleanup_file(&root, &segments).unwrap();
    assert_cleanup_preflight_rejection(&root, &plan, "journal segments tronqué");
}

#[test]
fn cleanup_characterization_rejects_malformed_successor_manifest_before_removal() {
    let root = archive_characterization_root("cleanup-c04-manifest");
    let mut plan = cleanup_fixture(&root, false);
    let manifest = generation_path(&root, plan.successor.job_id, plan.successor.generation)
        .join("manifest.json");
    fs::write(&manifest, b"{truncated").unwrap();
    plan.successor_files[0] = capture_cleanup_file(&root, &manifest).unwrap();
    assert_cleanup_preflight_rejection(&root, &plan, "manifeste successeur:");
}

#[test]
fn cleanup_characterization_rejects_divergent_publication_intent_before_removal() {
    let root = archive_characterization_root("cleanup-c05");
    let plan = cleanup_fixture(&root, false);
    let intent_path = generation_path(&root, plan.successor.job_id, plan.successor.generation)
        .join("r11-publication.json");
    let mut intent: PublicationIntent =
        serde_json::from_slice(&fs::read(&intent_path).unwrap()).unwrap();
    intent.source_sha256 = "divergent-source".into();
    fs::write(&intent_path, serde_json::to_vec(&intent).unwrap()).unwrap();
    assert_cleanup_preflight_rejection(&root, &plan, "intention de publication divergente");
}

#[test]
fn cleanup_characterization_rejects_changed_source_before_removal() {
    let root = archive_characterization_root("cleanup-c01");
    let plan = cleanup_fixture(&root, false);
    fs::write(&plan.source_path, b"changed source bytes").unwrap();
    assert_cleanup_preflight_rejection(&root, &plan, "SourceChanged:");
}

#[test]
fn cleanup_characterization_rejects_target_for_another_destination_before_removal() {
    let root = archive_characterization_root("cleanup-c08");
    let mut plan = cleanup_fixture(&root, false);
    plan.targets[0].treatment.destination.push_str("\\other");
    sync_cleanup_intent_targets(&root, &plan);
    assert_cleanup_preflight_rejection(&root, &plan, "cible chevauche le successeur");
}

#[test]
fn cleanup_characterization_rejects_changed_target_hash_before_removal() {
    let root = archive_characterization_root("cleanup-c10");
    let mut plan = cleanup_fixture(&root, false);
    plan.targets[0].objects[0].sha256 = Some("00".repeat(32));
    sync_cleanup_intent_targets(&root, &plan);
    assert_cleanup_preflight_rejection(&root, &plan, "empreinte modifiée");
}

#[test]
fn cleanup_characterization_rejects_invalid_container_proof_before_removal() {
    let root = archive_characterization_root("cleanup-c09");
    let mut plan = cleanup_fixture(&root, false);
    plan.targets[0].container.as_mut().unwrap().sha256 = Some("00".repeat(32));
    sync_cleanup_intent_targets(&root, &plan);
    assert_cleanup_preflight_rejection(&root, &plan, "preuve du conteneur invalide");
}

#[test]
fn cleanup_characterization_missing_container_path_is_recorded_as_removed() {
    let root = archive_characterization_root("cleanup-c07-container-absent");
    let mut plan = cleanup_fixture(&root, false);
    let real_container = root.join(
        plan.targets[0]
            .container
            .as_ref()
            .unwrap()
            .relative_path
            .clone(),
    );
    plan.targets[0].container.as_mut().unwrap().relative_path =
        "transcriptions/not-the-container".into();
    sync_cleanup_intent_targets(&root, &plan);
    let sentinels = capture_cleanup_sentinels(&root, &plan);
    let tree_before = cleanup_tree_snapshot(&root);
    let memory_before = raw_cleanup_plan(&root, &plan);

    write_cleanup_plan(&root, &plan).unwrap();
    ArchiveStore::resume_replacement_cleanup(&root, &plan.cleanup_id).unwrap();
    let persisted = read_cleanup_plan(&root, &plan.cleanup_id).unwrap();
    assert!(persisted.targets[0].container.as_ref().unwrap().removed);
    assert!(
        persisted.targets[0]
            .container
            .as_ref()
            .unwrap()
            .error
            .is_none()
    );
    assert!(
        real_container.is_dir(),
        "the existing container path is retained"
    );
    assert_cleanup_sentinels(&sentinels);
    write_cleanup_trace(
        &root,
        &serde_json::json!({
            "scenario": "C07-missing-container-path-recorded-removed",
            "tree_before": tree_before,
            "memory_plan_before": memory_before,
            "resume": "Ok",
            "tree_after": cleanup_tree_snapshot(&root),
            "persisted_plan_after": raw_cleanup_plan(&root, &persisted),
        }),
    );
}

#[test]
fn cleanup_characterization_rejects_changed_target_identity_before_removal() {
    let root = archive_characterization_root("cleanup-c10-identity");
    let mut plan = cleanup_fixture(&root, false);
    plan.targets[0].objects[0].identity.file_id[0] ^= 1;
    sync_cleanup_intent_targets(&root, &plan);
    assert_cleanup_preflight_rejection(&root, &plan, "CleanupIdentityChanged:");
}

#[test]
fn cleanup_characterization_preserves_object_marked_removed_while_present() {
    let root = archive_characterization_root("cleanup-c10-removed-present");
    let mut plan = cleanup_fixture(&root, false);
    let object_path = root.join(&plan.targets[0].objects[0].relative_path);
    plan.targets[0].objects[0].removed = true;
    sync_cleanup_intent_targets(&root, &plan);
    write_cleanup_plan(&root, &plan).unwrap();

    let error = ArchiveStore::resume_replacement_cleanup(&root, &plan.cleanup_id).unwrap_err();

    assert!(error.starts_with("StorageUnavailable:"), "{error}");
    assert!(
        object_path.is_file(),
        "the recorded removed object is retained"
    );
    let persisted = read_cleanup_plan(&root, &plan.cleanup_id).unwrap();
    assert!(persisted.targets[0].objects[0].removed);
    assert!(persisted.last_error.is_some());
}

#[test]
fn cleanup_characterization_rejects_modified_publication_index_before_removal() {
    let root = archive_characterization_root("cleanup-c05-index");
    let mut plan = cleanup_fixture(&root, false);
    plan.successor_index.as_mut().unwrap().sha256 = Some("00".repeat(32));
    assert_cleanup_preflight_rejection(&root, &plan, "index de publication modifié");
}

#[test]
fn cleanup_characterization_nonempty_generation_persists_partial_removal_and_resumes() {
    let root = archive_characterization_root("cleanup-d06");
    let plan = cleanup_fixture(&root, true);
    let sentinels = capture_cleanup_sentinels(&root, &plan);
    let tree_before = cleanup_tree_snapshot(&root);
    let memory_before = raw_cleanup_plan(&root, &plan);
    let disk_before = raw_cleanup_plan(&root, &read_cleanup_plan(&root, &plan.cleanup_id).unwrap());
    let target = &plan.targets[0];
    let owned = root.join(&target.objects[0].relative_path);
    let obstacle = root.join(&target.objects[1].relative_path);
    let generation_dir = root.join(&target.objects[2].relative_path);

    let first = ArchiveStore::resume_replacement_cleanup(&root, &plan.cleanup_id).unwrap_err();
    assert!(first.starts_with("StorageUnavailable:"), "{first}");
    assert!(
        !owned.exists(),
        "the owned file is removed before directory disposition"
    );
    assert!(obstacle.exists(), "the unowned file is retained");
    assert!(
        generation_dir.is_dir(),
        "the nonempty generation directory remains"
    );
    let persisted = read_cleanup_plan(&root, &plan.cleanup_id).unwrap();
    let tree_after_failure = cleanup_tree_snapshot(&root);
    let disk_after_failure = raw_cleanup_plan(&root, &persisted);
    assert!(persisted.targets[0].objects[0].removed);
    assert!(!persisted.targets[0].objects[1].removed);
    assert!(persisted.last_error.is_some());
    assert_cleanup_sentinels(&sentinels);

    fs::remove_file(&obstacle).unwrap();
    let tree_before_retry = cleanup_tree_snapshot(&root);
    ArchiveStore::resume_replacement_cleanup(&root, &plan.cleanup_id).unwrap();
    let resumed = read_cleanup_plan(&root, &plan.cleanup_id).unwrap();
    let tree_after_retry = cleanup_tree_snapshot(&root);
    assert!(
        resumed.targets[0]
            .objects
            .iter()
            .all(|object| object.removed)
    );
    assert!(resumed.targets[0].container.as_ref().unwrap().removed);
    assert!(resumed.last_error.is_none());
    assert!(!generation_dir.exists());
    assert!(!generation_dir.parent().unwrap().exists());
    assert_cleanup_sentinels(&sentinels);
    write_cleanup_trace(
        &root,
        &serde_json::json!({
            "scenario": "D06-nonempty-generation-partial-removal-resume",
            "before": tree_before,
            "memory_plan_before": memory_before,
            "persisted_plan_before": disk_before,
            "first_resume": {
                "result": first,
                "tree_after": tree_after_failure,
                "persisted_plan_after": disk_after_failure,
            },
            "remove_unowned_obstacle": {
                "tree_before": tree_before_retry,
                "relative_path": cleanup_relative(&root, &obstacle),
            },
            "second_resume": {
                "result": "Ok",
                "tree_after": tree_after_retry,
                "plan_as_loaded_after": raw_cleanup_plan(&root, &resumed),
                "persisted_plan_after": raw_cleanup_plan(&root, &resumed),
            },
        }),
    );
}

#[test]
fn cleanup_characterization_private_executor_is_called_with_publication_lock_held() {
    let root = archive_characterization_root("cleanup-d06-locked-executor");
    let mut plan = cleanup_fixture(&root, true);
    let sentinels = capture_cleanup_sentinels(&root, &plan);
    let target = &plan.targets[0];
    let owned = root.join(&target.objects[0].relative_path);
    let obstacle = root.join(&target.objects[1].relative_path);
    let generation_dir = root.join(&target.objects[2].relative_path);
    let lock = ArchiveStore::lock_publications(&root).unwrap();

    let first = super::super::cleanup::run_cleanup_plan_locked(&root, &mut plan).unwrap_err();

    drop(lock);
    assert!(first.starts_with("StorageUnavailable:"), "{first}");
    assert!(!owned.exists());
    assert!(obstacle.exists());
    assert!(generation_dir.is_dir());
    assert!(plan.targets[0].objects[0].removed);
    assert!(plan.targets[0].objects[1].error.is_some());
    assert!(plan.last_error.is_some());
    let persisted = read_cleanup_plan(&root, &plan.cleanup_id).unwrap();
    assert_eq!(persisted.last_error, plan.last_error);
    assert_eq!(persisted.targets[0].objects, plan.targets[0].objects);
    assert_cleanup_sentinels(&sentinels);
}

#[test]
fn cleanup_characterization_locked_file_preserves_target_until_retry() {
    use std::{fs::OpenOptions, os::windows::fs::OpenOptionsExt};

    let root = archive_characterization_root("cleanup-d04");
    let plan = cleanup_fixture(&root, false);
    let sentinels = capture_cleanup_sentinels(&root, &plan);
    let target = &plan.targets[0];
    let owned = root.join(&target.objects[0].relative_path);
    let generation_dir = root.join(&target.objects[1].relative_path);
    let tree_before = cleanup_tree_snapshot(&root);
    let lock = OpenOptions::new()
        .read(true)
        .share_mode(0x1 | 0x2)
        .open(&owned)
        .unwrap();

    let first = ArchiveStore::resume_replacement_cleanup(&root, &plan.cleanup_id).unwrap_err();
    assert!(first.starts_with("StorageUnavailable:"), "{first}");
    assert!(owned.exists(), "the locked owned file remains present");
    assert!(generation_dir.is_dir());
    let persisted = read_cleanup_plan(&root, &plan.cleanup_id).unwrap();
    assert!(!persisted.targets[0].objects[0].removed);
    assert!(persisted.last_error.is_some());
    assert_cleanup_sentinels(&sentinels);
    let tree_after_failure = cleanup_tree_snapshot(&root);
    let persisted_after_failure = raw_cleanup_plan(&root, &persisted);

    drop(lock);
    ArchiveStore::resume_replacement_cleanup(&root, &plan.cleanup_id).unwrap();
    let resumed = read_cleanup_plan(&root, &plan.cleanup_id).unwrap();
    let tree_after_retry = cleanup_tree_snapshot(&root);
    assert!(
        resumed.targets[0]
            .objects
            .iter()
            .all(|object| object.removed)
    );
    assert!(resumed.targets[0].container.as_ref().unwrap().removed);
    assert!(!owned.exists());
    assert!(!generation_dir.exists());
    assert!(!generation_dir.parent().unwrap().exists());
    assert_cleanup_sentinels(&sentinels);
    write_cleanup_trace(
        &root,
        &serde_json::json!({
            "scenario": "D04-locked-file-then-retry",
            "tree_before": tree_before,
            "memory_plan_before": raw_cleanup_plan(&root, &plan),
            "first_resume": {
                "result": first,
                "tree_after": tree_after_failure,
                "persisted_plan_after": persisted_after_failure,
            },
            "release_lock": "standard file handle dropped",
            "second_resume": {
                "result": "Ok",
                "tree_after": tree_after_retry,
                "persisted_plan_after": raw_cleanup_plan(&root, &resumed),
            },
        }),
    );
}

#[test]
fn cleanup_characterization_nominal_plan_removes_only_owned_target() {
    let root = archive_characterization_root("cleanup-nominal");
    let plan = cleanup_fixture(&root, false);
    let sentinels = capture_cleanup_sentinels(&root, &plan);
    let target = &plan.targets[0];
    let owned = root.join(&target.objects[0].relative_path);
    let generation_dir = root.join(&target.objects[1].relative_path);
    let container = root.join(&target.container.as_ref().unwrap().relative_path);
    let tree_before = cleanup_tree_snapshot(&root);

    ArchiveStore::resume_replacement_cleanup(&root, &plan.cleanup_id).unwrap();
    let completed = read_cleanup_plan(&root, &plan.cleanup_id).unwrap();
    assert!(
        completed.targets[0]
            .objects
            .iter()
            .all(|object| object.removed)
    );
    assert!(completed.targets[0].container.as_ref().unwrap().removed);
    assert!(completed.last_error.is_none());
    assert!(!owned.exists());
    assert!(!generation_dir.exists());
    assert!(!container.exists());
    write_single_resume_trace(
        &root,
        "D02-nominal-owned-target-removal",
        &plan,
        tree_before,
        "Ok",
        &sentinels,
    );
}

#[test]
fn cleanup_characterization_absent_file_is_recorded_then_remaining_target_is_removed() {
    let root = archive_characterization_root("cleanup-absent");
    let plan = cleanup_fixture(&root, false);
    let sentinels = capture_cleanup_sentinels(&root, &plan);
    let target = &plan.targets[0];
    let owned = root.join(&target.objects[0].relative_path);
    let generation_dir = root.join(&target.objects[1].relative_path);
    fs::remove_file(&owned).unwrap();
    let tree_before = cleanup_tree_snapshot(&root);

    ArchiveStore::resume_replacement_cleanup(&root, &plan.cleanup_id).unwrap();
    let completed = read_cleanup_plan(&root, &plan.cleanup_id).unwrap();
    assert!(
        completed.targets[0]
            .objects
            .iter()
            .all(|object| object.removed)
    );
    assert!(completed.targets[0].container.as_ref().unwrap().removed);
    assert!(completed.last_error.is_none());
    assert!(!generation_dir.exists());
    write_single_resume_trace(
        &root,
        "D03-already-absent-file",
        &plan,
        tree_before,
        "Ok",
        &sentinels,
    );
}

#[test]
fn cleanup_characterization_empty_target_list_preserves_successor_and_persists_empty_plan() {
    let root = archive_characterization_root("cleanup-empty");
    let plan = cleanup_fixture_without_targets(&root);
    let sentinels = capture_cleanup_sentinels(&root, &plan);
    let successor_path = generation_path(&root, plan.successor.job_id, plan.successor.generation);
    let tree_before = cleanup_tree_snapshot(&root);

    ArchiveStore::resume_replacement_cleanup(&root, &plan.cleanup_id).unwrap();
    let completed = read_cleanup_plan(&root, &plan.cleanup_id).unwrap();
    assert!(completed.targets.is_empty());
    assert!(completed.last_error.is_none());
    assert!(successor_path.join("CURRENT").is_file());
    assert!(successor_path.join("manifest.json").is_file());
    write_single_resume_trace(
        &root,
        "D01-empty-target-list",
        &plan,
        tree_before,
        "Ok",
        &sentinels,
    );
}

#[test]
fn cleanup_characterization_new_unselected_file_is_preserved_after_partial_removal() {
    let root = archive_characterization_root("cleanup-added-file");
    let plan = cleanup_fixture(&root, false);
    let sentinels = capture_cleanup_sentinels(&root, &plan);
    let target = &plan.targets[0];
    let owned = root.join(&target.objects[0].relative_path);
    let generation_dir = root.join(&target.objects[1].relative_path);
    let added = generation_dir.join("added-after-selection.txt");
    fs::write(&added, b"unselected data").unwrap();
    let tree_before = cleanup_tree_snapshot(&root);

    let first = ArchiveStore::resume_replacement_cleanup(&root, &plan.cleanup_id).unwrap_err();
    assert!(first.starts_with("StorageUnavailable:"), "{first}");
    assert!(
        !owned.exists(),
        "the previously selected owned file is removed"
    );
    assert!(
        added.exists(),
        "the unselected file added after selection is kept"
    );
    assert!(generation_dir.is_dir());
    let persisted = read_cleanup_plan(&root, &plan.cleanup_id).unwrap();
    let added_record = persisted.targets[0]
        .objects
        .iter()
        .find(|object| object.relative_path == cleanup_relative(&root, &added))
        .expect("the new item is added to the durable plan");
    assert!(!added_record.owned);
    assert!(!added_record.removed);
    assert!(
        added_record
            .error
            .as_deref()
            .unwrap()
            .contains("ajouté après confirmation")
    );
    assert!(persisted.last_error.is_some());
    write_single_resume_trace(
        &root,
        "D05-added-unselected-file-preserved",
        &plan,
        tree_before,
        &first,
        &sentinels,
    );
}

#[test]
fn cleanup_characterization_rejects_successor_proof_with_another_real_identity() {
    let root = archive_characterization_root("cleanup-c03-real-identity");
    let mut plan = cleanup_fixture(&root, false);
    let foreign = capture_cleanup_file(&root, Path::new(&plan.source_path)).unwrap();
    plan.successor_files[0].identity = foreign.identity;

    let case = record_additional_preflight_rejection(&root, &plan, "CleanupIdentityChanged:");

    write_additional_cleanup_trace(
        &root,
        &serde_json::json!({
            "scenario": "C03-successor-proof-has-another-real-file-identity",
            "case": case,
        }),
    );
}

#[test]
fn cleanup_characterization_rejects_successor_file_with_stale_hash_proof() {
    let root = archive_characterization_root("cleanup-c03-stale-hash");
    let plan = cleanup_fixture(&root, false);
    let transcript = generation_path(&root, plan.successor.job_id, plan.successor.generation)
        .join("transcript.txt");
    fs::write(&transcript, b"changed transcript bytes").unwrap();

    let case = record_additional_preflight_rejection(
        &root,
        &plan,
        "preuve successeur divergente transcript.txt",
    );

    write_additional_cleanup_trace(
        &root,
        &serde_json::json!({
            "scenario": "C03-successor-transcript-hash-proof-is-stale",
            "case": case,
        }),
    );
}

#[test]
fn cleanup_characterization_rejects_semantically_divergent_manifest_with_fresh_proof() {
    let root = archive_characterization_root("cleanup-c04-manifest-identity");
    let mut plan = cleanup_fixture(&root, false);
    let manifest_path = generation_path(&root, plan.successor.job_id, plan.successor.generation)
        .join("manifest.json");
    let mut manifest: CompleteRecord =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest.job_id = JobId(0x7403);
    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    plan.successor_files[0] = capture_cleanup_file(&root, &manifest_path).unwrap();

    let case = record_additional_preflight_rejection(
        &root,
        &plan,
        "manifeste successeur ne correspond pas",
    );

    write_additional_cleanup_trace(
        &root,
        &serde_json::json!({
            "scenario": "C04-manifest-identity-diverges-with-fresh-proof",
            "case": case,
        }),
    );
}

#[test]
fn cleanup_characterization_rejects_semantically_divergent_pending_with_fresh_proof() {
    let root = archive_characterization_root("cleanup-c04-pending-identity");
    let mut plan = cleanup_fixture(&root, false);
    let pending_path = generation_path(&root, plan.successor.job_id, plan.successor.generation)
        .join("pending.json");
    let mut pending: PendingRecord =
        serde_json::from_slice(&fs::read(&pending_path).unwrap()).unwrap();
    pending.job_id = JobId(0x7403);
    fs::write(&pending_path, serde_json::to_vec(&pending).unwrap()).unwrap();
    plan.successor_files[3] = capture_cleanup_file(&root, &pending_path).unwrap();

    let case =
        record_additional_preflight_rejection(&root, &plan, "identité du successeur divergente");

    write_additional_cleanup_trace(
        &root,
        &serde_json::json!({
            "scenario": "C04-pending-identity-diverges-with-fresh-proof",
            "case": case,
        }),
    );
}

#[test]
fn cleanup_characterization_rejects_out_of_sequence_segment_with_fresh_proofs() {
    let root = archive_characterization_root("cleanup-c04-segment-sequence");
    let mut plan = cleanup_fixture(&root, false);
    let successor_path = generation_path(&root, plan.successor.job_id, plan.successor.generation);
    let segments_path = successor_path.join("segments.jsonl");
    let segment = SegmentRecord {
        sequence: 2,
        range: whisper_core::SourceRange::new(0, 160, 16_000).unwrap(),
        text: "fixture segment".into(),
    };
    let mut bytes = serde_json::to_vec(&segment).unwrap();
    bytes.push(b'\n');
    fs::write(&segments_path, bytes).unwrap();
    plan.successor_files[4] = capture_cleanup_file(&root, &segments_path).unwrap();
    let manifest_path = successor_path.join("manifest.json");
    let mut manifest: CompleteRecord =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    manifest.segments = 1;
    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    plan.successor_files[0] = capture_cleanup_file(&root, &manifest_path).unwrap();

    let case = record_additional_preflight_rejection(&root, &plan, "journal segments incohérent");

    write_additional_cleanup_trace(
        &root,
        &serde_json::json!({
            "scenario": "C04-segment-sequence-is-not-contiguous-with-fresh-proofs",
            "case": case,
        }),
    );
}

#[test]
fn cleanup_characterization_rejects_index_version_with_fresh_proof() {
    let root = archive_characterization_root("cleanup-c05-index-version");
    let mut plan = cleanup_fixture(&root, false);
    let index_path = publication_index_path(&root);
    let mut index: PublicationIndex =
        serde_json::from_slice(&fs::read(&index_path).unwrap()).unwrap();
    index.version = 2;
    write_publication_index(&root, &index).unwrap();
    plan.successor_index = Some(capture_cleanup_file(&root, &index_path).unwrap());

    let case =
        record_additional_preflight_rejection(&root, &plan, "successeur absent de l'index actif");

    write_additional_cleanup_trace(
        &root,
        &serde_json::json!({
            "scenario": "C05-index-version-diverges-with-fresh-proof",
            "case": case,
        }),
    );
}

#[test]
fn cleanup_characterization_rejects_inactive_successor_index_record_with_fresh_proof() {
    let root = archive_characterization_root("cleanup-c05-index-inactive");
    let mut plan = cleanup_fixture(&root, false);
    let index_path = publication_index_path(&root);
    let mut index: PublicationIndex =
        serde_json::from_slice(&fs::read(&index_path).unwrap()).unwrap();
    index.records[0].active = false;
    write_publication_index(&root, &index).unwrap();
    plan.successor_index = Some(capture_cleanup_file(&root, &index_path).unwrap());

    let case =
        record_additional_preflight_rejection(&root, &plan, "successeur absent de l'index actif");

    write_additional_cleanup_trace(
        &root,
        &serde_json::json!({
            "scenario": "C05-inactive-index-record-diverges-with-fresh-proof",
            "case": case,
        }),
    );
}

#[test]
fn cleanup_characterization_locked_executor_rejects_divergent_destination() {
    let root = archive_characterization_root("cleanup-c01-destination");
    let mut plan = cleanup_fixture(&root, false);
    plan.destination = root
        .join("exports")
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .into_owned();

    let case =
        record_locked_cleanup_rejection(&root, &mut plan, "destination de reprise divergente");

    write_progress_cleanup_trace(
        &root,
        &serde_json::json!({
            "scenario": "C01-locked-executor-rejects-divergent-destination",
            "case": case,
        }),
    );
}

#[test]
fn cleanup_characterization_locked_executor_rejects_unpublished_plan() {
    let root = archive_characterization_root("cleanup-c01-unpublished");
    let mut plan = cleanup_fixture(&root, false);
    plan.published = false;

    let case =
        record_locked_cleanup_rejection(&root, &mut plan, "destination de reprise divergente");

    write_progress_cleanup_trace(
        &root,
        &serde_json::json!({
            "scenario": "C01-locked-executor-rejects-unpublished-plan",
            "case": case,
        }),
    );
}

#[test]
fn cleanup_characterization_persists_all_removed_containers_before_later_rejection() {
    let root = archive_characterization_root("cleanup-c07-all-removed-write");
    let mut plan = cleanup_fixture(&root, false);
    plan.targets[0].container.as_mut().unwrap().removed = true;
    let foreign_identity = capture_cleanup_file(&root, Path::new(&plan.source_path))
        .unwrap()
        .identity;
    plan.targets[0].objects[0].identity = foreign_identity;
    sync_cleanup_intent_targets(&root, &plan);
    let persisted_before = read_cleanup_plan(&root, &plan.cleanup_id).unwrap();
    assert!(
        !persisted_before.targets[0]
            .container
            .as_ref()
            .unwrap()
            .removed
    );

    let case = record_locked_cleanup_rejection(&root, &mut plan, "CleanupIdentityChanged:");
    let persisted_after = read_cleanup_plan(&root, &plan.cleanup_id).unwrap();
    assert!(
        persisted_after.targets[0]
            .container
            .as_ref()
            .unwrap()
            .removed
    );
    assert!(!persisted_after.targets[0].objects[0].removed);
    assert!(persisted_after.last_error.is_none());

    write_progress_cleanup_trace(
        &root,
        &serde_json::json!({
            "scenario": "C07-all-containers-removed-plan-written-before-target-rejection",
            "persisted_plan_before": raw_cleanup_plan(&root, &persisted_before),
            "case": case,
            "persisted_plan_after": raw_cleanup_plan(&root, &persisted_after),
        }),
    );
}

#[test]
fn cleanup_characterization_preserves_parent_container_with_unselected_generation() {
    let root = archive_characterization_root("cleanup-c13-parent-nonempty");
    let mut plan = cleanup_fixture(&root, false);
    let owned_file = root.join(&plan.targets[0].objects[0].relative_path);
    let generation = root.join(&plan.targets[0].objects[1].relative_path);
    let container = root.join(&plan.targets[0].container.as_ref().unwrap().relative_path);
    plan.targets[0].objects[1].owned = false;
    plan.targets[0].objects[1].error = Some("fixture generation kept".into());
    sync_cleanup_intent_targets(&root, &plan);
    write_cleanup_plan(&root, &plan).unwrap();
    let sentinels = capture_cleanup_sentinels(&root, &plan);
    let tree_before = cleanup_tree_snapshot(&root);
    let memory_before = raw_cleanup_plan(&root, &plan);
    let persisted_before =
        raw_cleanup_plan(&root, &read_cleanup_plan(&root, &plan.cleanup_id).unwrap());

    ArchiveStore::resume_replacement_cleanup(&root, &plan.cleanup_id).unwrap();

    let persisted_after = read_cleanup_plan(&root, &plan.cleanup_id).unwrap();
    assert!(!owned_file.exists());
    assert!(generation.is_dir());
    assert!(container.is_dir());
    let retained = persisted_after.targets[0].container.as_ref().unwrap();
    assert!(!retained.removed);
    assert!(!retained.owned);
    assert_eq!(
        retained.error.as_deref(),
        Some("contenu non sélectionné présent : conteneur conservé")
    );
    assert!(persisted_after.last_error.is_none());
    assert_cleanup_sentinels(&sentinels);
    let tree_after = cleanup_tree_snapshot(&root);

    write_progress_cleanup_trace(
        &root,
        &serde_json::json!({
            "scenario": "C13-parent-container-kept-for-unselected-generation",
            "tree_before": tree_before,
            "memory_plan_before": memory_before,
            "persisted_plan_before": persisted_before,
            "tree_after": tree_after,
            "persisted_plan_after": raw_cleanup_plan(&root, &persisted_after),
        }),
    );
}

#[test]
fn cleanup_characterization_persists_missing_index_read_error_without_removal() {
    let root = archive_characterization_root("cleanup-c15-index-read-error");
    let plan = cleanup_fixture(&root, false);
    let index_path = publication_index_path(&root);
    let sentinels = capture_cleanup_sentinels(&root, &plan);
    fs::remove_file(&index_path).unwrap();
    let tree_before = cleanup_tree_snapshot(&root);
    let memory_before = raw_cleanup_plan(&root, &plan);
    let persisted_before =
        raw_cleanup_plan(&root, &read_cleanup_plan(&root, &plan.cleanup_id).unwrap());
    let owned = root.join(&plan.targets[0].objects[0].relative_path);

    let error = ArchiveStore::resume_replacement_cleanup(&root, &plan.cleanup_id).unwrap_err();

    assert!(
        error.starts_with("StorageUnavailable: ouvrir l'objet exact"),
        "{error}"
    );
    assert!(owned.is_file());
    for sentinel in &sentinels {
        if sentinel.path == index_path {
            assert!(!sentinel.path.exists(), "the missing index remains absent");
        } else {
            assert_eq!(
                fs::read(&sentinel.path).unwrap(),
                sentinel.bytes,
                "sentinel changed: {}",
                sentinel.path.display()
            );
        }
    }
    let persisted_after = read_cleanup_plan(&root, &plan.cleanup_id).unwrap();
    assert!(
        persisted_after
            .last_error
            .as_deref()
            .is_some_and(|value| value.contains("ouvrir l'objet exact"))
    );
    let tree_after = cleanup_tree_snapshot(&root);

    write_progress_cleanup_trace(
        &root,
        &serde_json::json!({
            "scenario": "C15-missing-index-read-error-is-persisted",
            "error": error,
            "tree_before": tree_before,
            "memory_plan_before": memory_before,
            "persisted_plan_before": persisted_before,
            "tree_after": tree_after,
            "persisted_plan_after": raw_cleanup_plan(&root, &persisted_after),
        }),
    );
}

#[test]
fn cleanup_characterization_keeps_targets_when_plan_write_temp_is_a_directory() {
    let root = archive_characterization_root("cleanup-c15-plan-write-error");
    let plan = cleanup_fixture(&root, false);
    let temporary_path = cleanup_plan_path(&root, &plan.cleanup_id).with_extension("json.tmp");
    fs::create_dir(&temporary_path).unwrap();
    let sentinels = capture_cleanup_sentinels(&root, &plan);
    let extra_paths = [temporary_path.clone()];
    let tree_before = cleanup_tree_snapshot_with_extra_paths(&root, &extra_paths);
    let memory_before = raw_cleanup_plan(&root, &plan);
    let persisted_before =
        raw_cleanup_plan(&root, &read_cleanup_plan(&root, &plan.cleanup_id).unwrap());
    let owned = root.join(&plan.targets[0].objects[0].relative_path);

    let error = ArchiveStore::resume_replacement_cleanup(&root, &plan.cleanup_id).unwrap_err();

    assert!(
        error.contains("os error"),
        "expected the plan-temp open to fail: {error}"
    );
    assert!(temporary_path.is_dir());
    assert!(
        owned.is_file(),
        "the plan write fails before target removal"
    );
    assert_cleanup_sentinels(&sentinels);
    let persisted_after = read_cleanup_plan(&root, &plan.cleanup_id).unwrap();
    assert_eq!(
        persisted_after.last_error,
        persisted_before["last_error"].as_str().map(str::to_owned)
    );
    let tree_after = cleanup_tree_snapshot_with_extra_paths(&root, &extra_paths);

    write_progress_cleanup_trace(
        &root,
        &serde_json::json!({
            "scenario": "C15-plan-temp-directory-write-error-preserves-target",
            "error": error,
            "tree_before": tree_before,
            "memory_plan_before": memory_before,
            "persisted_plan_before": persisted_before,
            "tree_after": tree_after,
            "persisted_plan_after": raw_cleanup_plan(&root, &persisted_after),
        }),
    );
}

#[test]
fn cleanup_characterization_rejects_malformed_lf_terminated_segments_with_fresh_proof() {
    let root = archive_characterization_root("cleanup-c04-segments-invalid-json");
    let mut plan = cleanup_fixture(&root, false);
    let segments_path = generation_path(&root, plan.successor.job_id, plan.successor.generation)
        .join("segments.jsonl");
    fs::write(&segments_path, b"{truncated\n").unwrap();
    plan.successor_files[4] = capture_cleanup_file(&root, &segments_path).unwrap();

    let case = record_additional_preflight_rejection(&root, &plan, "journal segments:");

    write_rejection_cleanup_trace(
        &root,
        &serde_json::json!({
            "scenario": "C04-lf-terminated-segments-json-is-malformed-with-fresh-proof",
            "case": case,
        }),
    );
}

#[test]
fn cleanup_characterization_rejects_malformed_pending_with_fresh_proof() {
    let root = archive_characterization_root("cleanup-c04-pending-invalid-json");
    let mut plan = cleanup_fixture(&root, false);
    let pending_path = generation_path(&root, plan.successor.job_id, plan.successor.generation)
        .join("pending.json");
    fs::write(&pending_path, b"{truncated").unwrap();
    plan.successor_files[3] = capture_cleanup_file(&root, &pending_path).unwrap();

    let case = record_additional_preflight_rejection(&root, &plan, "pending successeur:");

    write_rejection_cleanup_trace(
        &root,
        &serde_json::json!({
            "scenario": "C04-pending-json-is-malformed-with-fresh-proof",
            "case": case,
        }),
    );
}

#[test]
fn cleanup_characterization_rejects_segment_count_mismatch_with_fresh_proofs() {
    let root = archive_characterization_root("cleanup-c04-segment-count");
    let mut plan = cleanup_fixture(&root, false);
    let successor = generation_path(&root, plan.successor.job_id, plan.successor.generation);
    let segments_path = successor.join("segments.jsonl");
    let segment = SegmentRecord {
        sequence: 1,
        range: whisper_core::SourceRange::new(0, 160, 16_000).unwrap(),
        text: "fixture segment".into(),
    };
    let mut bytes = serde_json::to_vec(&segment).unwrap();
    bytes.push(b'\n');
    fs::write(&segments_path, bytes).unwrap();
    plan.successor_files[4] = capture_cleanup_file(&root, &segments_path).unwrap();
    let manifest_path = successor.join("manifest.json");
    let manifest: CompleteRecord =
        serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
    plan.successor_files[0] = capture_cleanup_file(&root, &manifest_path).unwrap();
    assert_eq!(manifest.segments, 0);

    let case = record_additional_preflight_rejection(&root, &plan, "journal segments incohérent");

    write_rejection_cleanup_trace(
        &root,
        &serde_json::json!({
            "scenario": "C04-segment-count-mismatches-valid-jsonl-with-fresh-proofs",
            "case": case,
        }),
    );
}

#[test]
fn cleanup_characterization_rejects_unexpected_index_proof_path() {
    let root = archive_characterization_root("cleanup-c05-index-path");
    let mut plan = cleanup_fixture(&root, false);
    plan.successor_index.as_mut().unwrap().relative_path =
        "transcriptions/not-the-publication-index.json".into();

    let case = record_additional_preflight_rejection(
        &root,
        &plan,
        "chemin de l'index successeur inattendu",
    );

    write_rejection_cleanup_trace(
        &root,
        &serde_json::json!({
            "scenario": "C05-index-proof-path-is-unexpected",
            "case": case,
        }),
    );
}

#[test]
fn cleanup_characterization_rejects_real_foreign_index_identity() {
    let root = archive_characterization_root("cleanup-c05-index-identity");
    let mut plan = cleanup_fixture(&root, false);
    plan.successor_index.as_mut().unwrap().identity =
        capture_cleanup_file(&root, Path::new(&plan.source_path))
            .unwrap()
            .identity;

    let case = record_additional_preflight_rejection(
        &root,
        &plan,
        "l'identifiant Windows du fichier a changé",
    );

    write_rejection_cleanup_trace(
        &root,
        &serde_json::json!({
            "scenario": "C05-index-proof-has-another-real-file-identity",
            "case": case,
        }),
    );
}

#[test]
fn cleanup_characterization_rejects_malformed_index_json_with_fresh_proof() {
    let root = archive_characterization_root("cleanup-c05-index-invalid-json");
    let mut plan = cleanup_fixture(&root, false);
    let index_path = publication_index_path(&root);
    fs::write(&index_path, b"{truncated").unwrap();
    plan.successor_index = Some(capture_cleanup_file(&root, &index_path).unwrap());

    let case = record_additional_preflight_rejection(&root, &plan, "index de publication:");

    write_rejection_cleanup_trace(
        &root,
        &serde_json::json!({
            "scenario": "C05-index-json-is-malformed-with-fresh-proof",
            "case": case,
        }),
    );
}

#[test]
fn cleanup_characterization_rejects_target_intent_divergence_without_syncing_it() {
    let root = archive_characterization_root("cleanup-c05-intent-targets");
    let mut plan = cleanup_fixture(&root, false);
    let distinct_source_sha256 = "0".repeat(64);
    assert_eq!(distinct_source_sha256.len(), 64);
    assert!(
        distinct_source_sha256
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    );
    assert_ne!(plan.targets[0].source_sha256, distinct_source_sha256);
    plan.targets[0].source_sha256 = distinct_source_sha256;

    let case =
        record_additional_preflight_rejection(&root, &plan, "intention de publication divergente");

    write_rejection_cleanup_trace(
        &root,
        &serde_json::json!({
            "scenario": "C05-cleanup-targets-diverge-from-publication-intent",
            "case": case,
        }),
    );
}

#[test]
fn cleanup_characterization_rejects_real_foreign_container_identity() {
    let root = archive_characterization_root("cleanup-c09-container-identity");
    let mut plan = cleanup_fixture(&root, false);
    let foreign = identity_snapshot(
        crate::windows_file_identity::inspect(&root.join("exports"), true).unwrap(),
    )
    .unwrap();
    let container = plan.targets[0].container.as_mut().unwrap();
    let relative_path = container.relative_path.clone();
    container.identity = foreign.clone();
    let ancestor = plan.targets[0]
        .ancestors
        .iter_mut()
        .find(|ancestor| ancestor.relative_path == relative_path)
        .unwrap();
    ancestor.identity = foreign;
    sync_cleanup_intent_targets(&root, &plan);

    let case = record_additional_preflight_rejection(
        &root,
        &plan,
        "l'identifiant Windows du fichier a changé",
    );

    write_rejection_cleanup_trace(
        &root,
        &serde_json::json!({
            "scenario": "C09-container-proof-has-another-real-directory-identity",
            "case": case,
        }),
    );
}
