    fn reprocessing_candidate(
        job_id: u128,
        source_sha256: &str,
        success_ordinal: Option<u64>,
        active: bool,
        verified_present: bool,
    ) -> PublishedImport {
        PublishedImport {
            job_id: JobId(job_id),
            generation: Generation::first(),
            source_sha256: source_sha256.to_owned(),
            export_basename: None,
            active,
            success_ordinal,
            verified_present,
        }
    }

    fn reprocessing_test_root(name: &str) -> PathBuf {
        let base =
            PathBuf::from(std::env::var_os("PARAKEET_RTC02_EXEC_ROOT").expect("RTC-02 root"));
        let root = base
            .join("archive")
            .join(format!("{name}-{}", std::process::id()));
        let treatment = root.join("transcriptions/00000000000000000000000000000001/1");
        fs::create_dir_all(&treatment).unwrap();
        root
    }

    #[test]
    fn emptied_transcriptions_keeps_history_but_offers_no_duplicate_targets() {
        let root = reprocessing_test_root("empty-destination-history");
        let source_sha = "ab".repeat(32);
        let index_dir = root.join(".whisper-publications");
        fs::create_dir_all(&index_dir).unwrap();
        fs::write(
            index_dir.join("index.json"),
            serde_json::to_vec(&serde_json::json!({
                "version": 1,
                "records": [{
                    "job_id": 1,
                    "generation": 1,
                    "source_sha256": source_sha,
                    "export_basename": null,
                    "active": true,
                    "success_ordinal": 1
                }]
            }))
            .unwrap(),
        )
        .unwrap();
        fs::remove_dir_all(root.join("transcriptions")).unwrap();

        let historical = ArchiveStore::reprocessing_catalog(&root, &source_sha).unwrap();
        assert_eq!(historical.len(), 1, "durable history must remain intact");
        assert!(!historical[0].verified_present);
        assert!(verified_present_publications(historical).is_empty());
        assert!(
            !root.join("transcriptions").exists(),
            "catalog inspection must not recreate the user's emptied destination"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn reprocessing_latest_requires_known_unique_order_and_all_includes_inactive_present() {
        let root = reprocessing_test_root("catalog-choice");
        let sha = "a".repeat(64);
        let catalog = vec![
            reprocessing_candidate(1, &sha, None, true, true),
            reprocessing_candidate(2, &sha, None, false, true),
            reprocessing_candidate(3, &sha, Some(3), false, false),
        ];
        let unknown = resolve_reprocessing_targets(
            &root,
            &catalog[..2],
            &DuplicateImportChoice::ReplaceLatest,
        );
        assert!(
            unknown
                .unwrap_err()
                .contains("ordre historique est inconnu")
        );

        let all = resolve_reprocessing_targets(&root, &catalog, &DuplicateImportChoice::ReplaceAll)
            .unwrap();
        assert_eq!(all.len(), 2);
        assert!(all.iter().any(|item| item.job_id == JobId(2)));
        assert!(!all.iter().any(|item| item.job_id == JobId(3)));

        let known_order = vec![
            reprocessing_candidate(1, &sha, Some(1), true, true),
            reprocessing_candidate(2, &sha, Some(2), false, true),
            reprocessing_candidate(3, &sha, Some(3), false, false),
        ];
        let latest = resolve_reprocessing_targets(
            &root,
            &known_order,
            &DuplicateImportChoice::ReplaceLatest,
        )
        .unwrap();
        assert_eq!(latest.len(), 1);
        assert_eq!(latest[0].job_id, JobId(2));
    }

    #[test]
    fn browse_accepts_only_exact_verified_treatment_in_current_destination() {
        let root = reprocessing_test_root("browse-choice");
        let sha = "b".repeat(64);
        let candidate = reprocessing_candidate(1, &sha, Some(1), true, true);
        let selected = root.join("transcriptions/00000000000000000000000000000001/1");
        let accepted = resolve_reprocessing_targets(
            &root,
            std::slice::from_ref(&candidate),
            &DuplicateImportChoice::ReplaceSelectedPath(selected.to_string_lossy().into_owned()),
        )
        .unwrap();
        assert_eq!(accepted.len(), 1);
        assert_eq!(accepted[0].job_id, JobId(1));

        let unrelated_audio = root.join("transcriptions/00000000000000000000000000000002/1");
        fs::create_dir_all(&unrelated_audio).unwrap();
        assert!(
            resolve_reprocessing_targets(
                &root,
                std::slice::from_ref(&candidate),
                &DuplicateImportChoice::ReplaceSelectedPath(
                    unrelated_audio.to_string_lossy().into_owned()
                ),
            )
            .is_err()
        );

        let tier = root.join("transcriptions/not-a-treatment");
        fs::create_dir_all(&tier).unwrap();
        assert!(
            resolve_reprocessing_targets(
                &root,
                std::slice::from_ref(&candidate),
                &DuplicateImportChoice::ReplaceSelectedPath(tier.to_string_lossy().into_owned()),
            )
            .is_err()
        );

        let other_destination = root.parent().unwrap().join("external-treatment");
        fs::create_dir_all(&other_destination).unwrap();
        assert!(
            resolve_reprocessing_targets(
                &root,
                &[candidate],
                &DuplicateImportChoice::ReplaceSelectedPath(
                    other_destination.to_string_lossy().into_owned()
                ),
            )
            .is_err()
        );
    }

    fn test_artifact_root(name: &str) -> PathBuf {
        let base = std::env::var_os("PARAKEET_R6_TEST_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::temp_dir().join("whisper-adapter-tests"));
        base.join(format!("{name}-{}", std::process::id()))
    }

    fn not_run_without_campaign_variables(variables: &[&str]) -> bool {
        let missing = variables
            .iter()
            .filter(|name| std::env::var_os(name).is_none())
            .copied()
            .collect::<Vec<_>>();
        if missing.is_empty() {
            return false;
        }
        eprintln!(
            "NOT RUN: missing campaign variable(s): {}",
            missing.join(", ")
        );
        true
    }

    #[test]
    fn timestamp_guard_accepts_exactly_one_tdt_frame_overlap() {
        assert_eq!(TDT_FRAME_OVERLAP_SAMPLES, 80 * 16);
        assert!(vad_word_overlap_within_one_tdt_frame(10_000, 11_280));
        assert!(!vad_word_overlap_within_one_tdt_frame(10_000, 11_281));
    }

    #[test]
    fn single_pass_adapter_accepts_normalized_words_and_rejects_any_overlap() {
        let normalized = [
            vad_test_word("faire?", 17_760, 17_920),
            vad_test_word("Je", 18_000, 18_080),
        ];
        assert_eq!(
            validate_single_pass_parakeet_words(&normalized, 27_000 * 16).unwrap(),
            vec![
                (17_760 * 16, 17_920 * 16, "faire?".into()),
                (18_000 * 16, 18_080 * 16, "Je".into()),
            ]
        );
        let overlapping = [
            vad_test_word("faire?", 17_760, 18_080),
            vad_test_word("Je", 18_000, 18_080),
        ];
        assert!(
            validate_single_pass_parakeet_words(&overlapping, 27_000 * 16)
                .unwrap_err()
                .contains("overlaps or is unordered")
        );
    }

    #[test]
    fn single_pass_punctuation_boundary_keeps_original_words_with_positive_ranges() {
        let words = [
            vad_test_word("faire?", 17_760, 17_920),
            vad_test_word("Je", 18_000, 18_080),
        ];
        let validated = validate_single_pass_parakeet_words(&words, 27_000 * 16).unwrap();
        let mut groups = Vec::new();
        for (start, end, token) in validated {
            append_single_pass_parakeet_word_group(&mut groups, start, end, &token).unwrap();
        }
        assert_eq!(groups.len(), 2);
        assert_eq!(
            groups[0],
            (17_760 * 16, 17_920 * 16, vec!["faire?".to_owned()])
        );
        assert_eq!(groups[1], (18_000 * 16, 18_080 * 16, vec!["Je".to_owned()]));
    }

    fn vad_test_window() -> whisper_core::parakeet_windows::Window {
        whisper_core::parakeet_windows::Window {
            index: 0,
            central_start_sample: 0,
            central_end_sample: 400_000,
            native_start_sample: 0,
            native_end_sample: 400_000,
            cut: None,
        }
    }

    fn vad_test_word(text: &str, start_ms: u32, end_ms: u32) -> whisper_core::parakeet_ipc::Word {
        whisper_core::parakeet_ipc::Word {
            text: text.into(),
            start_ms,
            end_ms,
        }
    }

    #[test]
    fn vad_sentence_overlap_within_one_frame_stays_in_one_valid_segment() {
        let words = [
            vad_test_word("?", 17_920, 18_080),
            vad_test_word("Je", 18_000, 18_080),
        ];
        let segments = parakeet_vad_segments(
            &words,
            vad_test_window(),
            400_000,
            JobId(1),
            Generation::first(),
            1,
            1,
        )
        .unwrap();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].text, "? Je");
        assert_eq!(segments[0].range.start_sample, 17_920 * 16);
        assert_eq!(segments[0].range.end_sample, 18_080 * 16);
        assert_eq!(segments[0].text.split_whitespace().count(), 2);
    }

    #[test]
    fn vad_overlapped_sentence_word_extends_group_to_later_end() {
        let words = [
            vad_test_word("?", 17_920, 18_080),
            vad_test_word("Je", 18_000, 18_100),
        ];
        let segments = parakeet_vad_segments(
            &words,
            vad_test_window(),
            400_000,
            JobId(1),
            Generation::first(),
            1,
            1,
        )
        .unwrap();
        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].range.end_sample, 18_100 * 16);
        assert_eq!(segments[0].text, "? Je");
    }

    #[test]
    fn vad_sentence_overlap_over_one_tdt_frame_is_rejected() {
        let words = [
            vad_test_word("?", 17_920, 18_080),
            vad_test_word("Je", 17_999, 18_080),
        ];
        let error = parakeet_vad_segments(
            &words,
            vad_test_window(),
            400_000,
            JobId(1),
            Generation::first(),
            1,
            1,
        )
        .unwrap_err();
        assert!(error.contains("unordered"));
    }

    #[test]
    fn gpu_attestation_ack_is_bound_to_the_current_session() {
        let event = WorkerEvent::GpuSessionAttested {
            job_id: JobId(42),
            generation: Generation::first(),
            instance_id: 9,
            candidate: "PCI\\VEN_10DE&DEV_1234".into(),
            dxgi_rank: 2,
            luid: 0x1234,
            pci_id: "0000:01:00.0".into(),
            cuda_ordinal: 1,
            ggml_ordinal: 4,
        };
        let current = (JobId(42), Generation::first(), 9);
        assert_eq!(worker_event_identity(&event), Some(current));
        assert!(!worker_event_is_stale(&event, current));
        assert!(worker_event_is_stale(
            &event,
            (JobId(42), Generation::first(), 10)
        ));
        assert!(worker_event_is_stale(
            &event,
            (JobId(43), Generation::first(), 9)
        ));
    }

    #[test]
    fn missing_cuda_worker_is_reported_without_changing_cpu_path() {
        let missing = std::env::temp_dir().join("whisper-absent-gpu-worker-for-test.exe");
        assert!(!missing.is_file());
        let error = probe_cuda_worker(&missing).unwrap_err();
        assert!(error.contains("GPU indisponible"));
        assert!(error.contains("choisissez CPU"));
    }

    fn compile_fake_worker(root: &Path, mode: &str, marker: &Path) -> PathBuf {
        let source = root.join(format!("{mode}-worker.rs"));
        let executable = root.join(format!("{mode}-worker.exe"));
        let marker = marker
            .to_string_lossy()
            .replace('\\', "\\\\")
            .replace('"', "\\\"");
        let program = r#"
use std::io::{self, BufRead, Write};
const MODE: &str = "__MODE__";
const MARKER: &str = "__MARKER__";
fn field(line: &str, name: &str) -> String {
    let token = format!("\"{name}\":");
    let tail = line.split_once(&token).expect("wire identity field").1.trim_start();
    tail.chars().take_while(|c| c.is_ascii_digit()).collect()
}
fn main() {
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    let start = lines.next().expect("start command").expect("start frame");
    let job = field(&start, "job_id");
    let generation = field(&start, "generation");
    let instance = field(&start, "instance_id");
    let attested = format!("{{\"version\":2,\"message\":{{\"GpuSessionAttested\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"candidate\":\"PCI\\\\VEN_10DE&DEV_1B06&SUBSYS_85F11043&REV_A1\\\\4&8BD6E8D&0&0008\",\"dxgi_rank\":2,\"luid\":4660,\"pci_id\":\"0000:01:00.0\",\"cuda_ordinal\":1,\"ggml_ordinal\":4}}}}}}");
    if MODE == "gpu-ready-exit" {
        println!("{attested}");
        println!("{{\"version\":2,\"message\":{{\"Ready\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"backend\":\"Cuda\"}}}}}}");
        let _ = io::stdout().flush();
        return;
    }
    if MODE == "gpu-ready" {
        println!("{attested}");
        println!("{{\"version\":2,\"message\":{{\"Ready\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"backend\":\"Cuda\"}}}}}}");
        let _ = io::stdout().flush();
        for command in lines.flatten() {
            if command.contains("\"BeginImport\"") {
                std::fs::write(MARKER, format!("{job}:{generation}:{instance}")).expect("begin marker");
                println!("{{\"version\":2,\"message\":{{\"ImportStarted\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance}}}}}}}");
                let _ = io::stdout().flush();
            }
            if command.contains("\"Stop\"") {
                println!("{{\"version\":2,\"message\":{{\"Stopped\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance}}}}}}}");
                let _ = io::stdout().flush();
                return;
            }
        }
        return;
    }
    if MODE == "gpu" {
        println!("{{\"version\":2,\"message\":{{\"Failed\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"code\":\"BackendUnavailable\",\"message\":\"injected GPU pre-Ready failure\"}}}}}}");
        let _ = io::stdout().flush();
        return;
    }
    if MODE == "cpu-fail" {
        std::fs::write(MARKER, format!("{job}:{generation}:{instance}")).expect("failed CPU marker");
        println!("{{\"version\":2,\"message\":{{\"Failed\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"code\":\"BackendUnavailable\",\"message\":\"injected CPU startup failure\"}}}}}}");
        let _ = io::stdout().flush();
        return;
    }
    if !start.contains("\"compute\":\"Cpu\"") { return; }
    if MODE == "cpu-count" {
        use std::io::Write as _;
        let mut marker = std::fs::OpenOptions::new().create(true).append(true).open(MARKER).expect("ready marker");
        writeln!(marker, "{job}:{generation}:{instance}").expect("append ready marker");
    } else {
        std::fs::write(MARKER, format!("{job}:{generation}:{instance}")).expect("ready marker");
    }
    println!("{{\"version\":2,\"message\":{{\"Ready\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"backend\":\"Cpu\"}}}}}}");
    let _ = io::stdout().flush();
    if MODE == "cpu" || MODE == "cpu-count" { return; }
    for command in lines.flatten() {
        if command.contains("\"Stop\"") {
            println!("{{\"version\":2,\"message\":{{\"Stopped\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance}}}}}}}");
            let _ = io::stdout().flush();
            return;
        }
    }
}
"#
            .replace("__MODE__", mode)
            .replace("__MARKER__", &marker);
        std::fs::write(&source, program).unwrap();
        let output = Command::new("rustc")
            .args(["--edition=2021", "--crate-name", "whisper_test_worker"])
            .arg(&source)
            .arg("-o")
            .arg(&executable)
            .output()
            .expect("rustc test worker");
        assert!(
            output.status.success(),
            "rustc test worker: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        executable
    }

    fn compile_multi_session_worker(root: &Path, marker: &Path) -> PathBuf {
        let source = root.join("multi-session-worker.rs");
        let executable = root.join("multi-session-worker.exe");
        let marker = serde_json::to_string(&marker.to_string_lossy().to_string()).unwrap();
        let program = r#"
use std::io::{self, BufRead, Write};
const MARKER: &str = __MARKER__;
fn field(line: &str, name: &str) -> String {
    let token = format!("\"{name}\":");
    let tail = line.split_once(&token).expect("wire identity field").1.trim_start();
    tail.chars().take_while(|c| c.is_ascii_digit()).collect()
}
fn event(name: &str, job: &str, generation: &str, instance: &str) {
    let body = match name {
        "Ready" => format!("\"Ready\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"backend\":\"Cpu\"}}"),
        _ => format!("\"End\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"last_sequence\":0,\"last_offset\":0}}"),
    };
    println!("{{\"version\":2,\"message\":{{{body}}}}}");
    let _ = io::stdout().flush();
}
fn main() {
    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();
    let mut cached_identity: Option<String> = None;
    for line in lines.flatten() {
        if line.contains("\"Shutdown\"") { return; }
        if line.contains("\"StartImport\"") {
            let job = field(&line, "job_id");
            let generation = field(&line, "generation");
            let instance = field(&line, "instance_id");
            let identity = format!("{}:{}", line.contains("model.bin"), line.contains("[9,9,9,9]"));
            let result = if cached_identity.as_ref() == Some(&identity) { "hit" } else { "load" };
            cached_identity = Some(identity);
            use std::io::Write as _;
            let mut file = std::fs::OpenOptions::new().create(true).append(true).open(MARKER).unwrap();
            writeln!(file, "{result}").unwrap();
            event("Ready", &job, &generation, &instance);
            event("End", &job, &generation, &instance);
        }
    }
}
"#
        .replace("__MARKER__", &marker);
        std::fs::write(&source, program).unwrap();
        let output = Command::new("rustc")
            .args([
                "--edition=2021",
                "--crate-name",
                "whisper_test_multi_session_worker",
            ])
            .arg(&source)
            .arg("-o")
            .arg(&executable)
            .output()
            .expect("rustc multi-session test worker");
        assert!(
            output.status.success(),
            "rustc test worker: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        executable
    }

    fn receive_worker_event(
        worker: &mut ChildWorkerTransport,
        timeout: Duration,
    ) -> Result<WorkerEvent, PortError> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(event) = worker.receive()? {
                return Ok(event);
            }
            assert!(Instant::now() < deadline, "worker IPC event timed out");
            thread::sleep(Duration::from_millis(2));
        }
    }

    fn retained_import_command(job_id: u128, instance_id: u64) -> WorkerCommand {
        WorkerCommand::StartImport {
            retain_after_publish: true,
            job_id: JobId(job_id),
            generation: Generation::first(),
            instance_id,
            source_path: "source.wav".into(),
            source_sha256: [7; 32],
            expected_source_samples: 16_000,
            model_path: "model.bin".into(),
            model_sha256: [9; 32],
            config: JobConfig {
                language: LanguageChoice::Manual("fr".into()),
                compute: ComputeChoice::Cpu,
            },
        }
    }

    #[test]
    fn retained_worker_reads_two_correlated_import_sessions_and_hits_model_cache() {
        let root = test_artifact_root("whisper-retained-imports");
        std::fs::create_dir_all(&root).unwrap();
        let marker = root.join("cache-events.txt");
        let executable = compile_multi_session_worker(&root, &marker);
        let mut worker = ChildWorkerTransport::spawn(&executable).unwrap();

        for (job_id, instance_id) in [(801_u128, 41_u64), (802, 42)] {
            worker
                .request(retained_import_command(job_id, instance_id))
                .unwrap();
            assert!(matches!(
                receive_worker_event(&mut worker, Duration::from_secs(3)).unwrap(),
                WorkerEvent::Ready { job_id: actual, generation, instance_id: actual_instance, backend: BackendKind::Cpu }
                    if actual == JobId(job_id) && generation == Generation::first() && actual_instance == instance_id
            ));
            assert!(matches!(
                receive_worker_event(&mut worker, Duration::from_secs(3)).unwrap(),
                WorkerEvent::End { job_id: actual, generation, instance_id: actual_instance, .. }
                    if actual == JobId(job_id) && generation == Generation::first() && actual_instance == instance_id
            ));
        }

        assert_eq!(std::fs::read_to_string(&marker).unwrap(), "load\nhit\n");
        worker.request(WorkerCommand::Shutdown).unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if worker.try_wait().unwrap().is_some() {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "retained worker did not shut down"
            );
            thread::sleep(Duration::from_millis(2));
        }
        drop(worker);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_publish_drops_pending_worker_without_starting_retention() {
        let root = test_artifact_root("whisper-publish-failure-cache");
        std::fs::create_dir_all(&root).unwrap();
        let marker = root.join("cache-events.txt");
        let executable = compile_multi_session_worker(&root, &marker);
        let mut transport = ChildWorkerTransport::spawn(&executable).unwrap();
        transport.request(retained_import_command(803, 43)).unwrap();
        assert!(matches!(
            receive_worker_event(&mut transport, Duration::from_secs(3)).unwrap(),
            WorkerEvent::Ready { .. }
        ));

        let gate = Arc::new(PublicationGate::default());
        gate.activate();
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: executable.clone(),
                gpu: root.join("missing-gpu.exe"),
            },
            gate,
        );
        runtime.archive = ArchiveStore::new(root.join("archive"));
        runtime.pending_idle_worker = Some(IdleWorker {
            transport,
            compute: ComputeChoice::Cpu,
            model_path: "model.bin".into(),
            model_sha256: [9; 32],
            expires_at: Instant::now(),
        });

        let error = runtime
            .publish(JobId(803), Generation::first(), 0, &mut VecDeque::new())
            .expect_err("an unprepared archive cannot publish");

        assert_eq!((error.0, error.1), (JobId(803), Generation::first()));
        assert!(runtime.pending_idle_worker.is_none());
        assert!(runtime.idle_worker.is_none());
        assert!(runtime.transport.is_none());
        drop(runtime);
        std::fs::remove_dir_all(root).unwrap();
    }

    fn compile_verified_gpu_init_failure_worker(
        root: &Path,
        model_path: &Path,
        model_bytes: &[u8],
        model_sha256: [u8; 32],
        error_code: &str,
    ) -> PathBuf {
        let source = root.join("gpu-init-failure-worker.rs");
        let executable = root.join("gpu-init-failure-worker.exe");
        let model_path = model_path.to_string_lossy();
        let model_path_json = serde_json::to_string(&*model_path).unwrap();
        let model_path_json_literal = serde_json::to_string(&model_path_json).unwrap();
        let model_sha256_json = serde_json::to_string(&model_sha256).unwrap();
        let model_sha256_json_literal = serde_json::to_string(&model_sha256_json).unwrap();
        let error_code_literal = format!("\\\"{error_code}\\\"");
        let model_bytes_literal =
            serde_json::to_string(std::str::from_utf8(model_bytes).unwrap()).unwrap();
        let program = r#"
use std::io::{self, BufRead, Write};
const MODEL_PATH_JSON: &str = __MODEL_PATH_JSON__;
const MODEL_PATH: &str = __MODEL_PATH__;
const MODEL_SHA256_JSON: &str = __MODEL_SHA256_JSON__;
const MODEL_BYTES: &str = __MODEL_BYTES__;
fn field(line: &str, name: &str) -> String {
    let token = format!("\"{name}\":");
    let tail = line.split_once(&token).expect("wire identity field").1.trim_start();
    tail.chars().take_while(|c| c.is_ascii_digit()).collect()
}
fn main() {
    let stdin = io::stdin();
    let start = stdin.lock().lines().next().expect("start command").expect("start frame");
    if !start.contains(&format!("\"model_path\":{MODEL_PATH_JSON}"))
        || !start.contains(&format!("\"model_sha256\":{MODEL_SHA256_JSON}"))
        || std::fs::read(MODEL_PATH).ok().as_deref()
            != Some(MODEL_BYTES.as_bytes())
    {
        return;
    }
    let job = field(&start, "job_id");
    let generation = field(&start, "generation");
    let instance = field(&start, "instance_id");
    println!("{{\"version\":2,\"message\":{{\"Failed\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"code\":__ERROR_CODE__,\"message\":\"GPU candidate revalidation failed after model hash verification\"}}}}}}");
    let _ = io::stdout().flush();
}
"#
        .replace("__ERROR_CODE__", &error_code_literal)
        .replace("__MODEL_PATH_JSON__", &model_path_json_literal)
        .replace("__MODEL_PATH__", &model_path_json)
        .replace("__MODEL_SHA256_JSON__", &model_sha256_json_literal)
        .replace("__MODEL_BYTES__", &model_bytes_literal);
        std::fs::write(&source, program).unwrap();
        let output = Command::new("rustc")
            .args(["--edition=2021", "--crate-name", "whisper_gpu_init_failure"])
            .arg(&source)
            .arg("-o")
            .arg(&executable)
            .output()
            .expect("rustc test worker");
        assert!(
            output.status.success(),
            "rustc test worker: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        executable
    }

    fn poll_until(
        runtime: &mut ImportRuntime,
        timeout: Duration,
        mut predicate: impl FnMut(&Result<ImportEvent, (JobId, Generation, String)>) -> bool,
    ) -> Result<ImportEvent, (JobId, Generation, String)> {
        let deadline = std::time::Instant::now() + timeout;
        loop {
            if let Some(event) = runtime.poll()
                && predicate(&event)
            {
                return event;
            }
            if std::time::Instant::now() >= deadline {
                let tail = runtime
                    .transport
                    .as_mut()
                    .map(|transport| (transport.try_wait(), transport.receive()));
                panic!(
                    "worker transport timed out; tail={tail:?}; active={:?}",
                    runtime.active.as_ref().map(|active| (
                        active.job_id,
                        active.generation,
                        active.backend
                    ))
                );
            }
            thread::sleep(Duration::from_millis(2));
        }
    }

    fn prepared_import(job_id: JobId) -> ImportRequest {
        let mut request = test_request(job_id, Generation::first());
        request.source_sha256 = Some([7; 32]);
        request.source_samples = Some(16_000);
        request.config.compute = ComputeChoice::Auto;
        request
    }

    fn fake_gpu_candidate() -> whisper_core::gpu_inventory::GpuRuntimeIdentity {
        whisper_core::gpu_inventory::GpuRuntimeIdentity {
            generation: 8,
            pnp_instance_id: whisper_core::model_registry::PARAKEET_TEST_GPU_PNP_INSTANCE_ID.into(),
            dxgi_rank: 2,
            luid: 0x1234,
            pci_id: "0000:01:00.0".into(),
            cuda_ordinal: 1,
            ggml_ordinal: 4,
        }
    }

    fn stop_worker(runtime: &mut ImportRuntime, job_id: JobId, generation: Generation) {
        let mut events = VecDeque::new();
        runtime.stop(job_id, generation, &mut events).unwrap();
        let stopped = poll_until(runtime, Duration::from_secs(3), |event| {
            matches!(event, Ok(ImportEvent::Stopped { .. }) | Err(_))
        });
        assert!(matches!(stopped, Ok(ImportEvent::Stopped { .. }) | Err(_)));
    }

    #[test]
    fn auto_without_gpu_starts_cpu_with_normalized_compute_and_same_identity() {
        let root = std::env::temp_dir().join(format!("whisper-auto-no-gpu-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let marker = root.join("cpu-ready");
        let cpu = compile_fake_worker(&root, "cpu", &marker);
        let gate = Arc::new(PublicationGate::default());
        gate.activate();
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu,
                gpu: root.join("missing-gpu.exe"),
            },
            gate,
        );
        let request = prepared_import(JobId(701));
        runtime.start_worker(request.clone()).unwrap();
        let ready = poll_until(&mut runtime, Duration::from_secs(3), |event| {
            matches!(event, Ok(ImportEvent::Ready { .. }) | Err(_))
        });
        assert!(
            matches!(ready, Ok(ImportEvent::Ready { .. })),
            "transport event: {ready:?}"
        );
        assert!(matches!(
            ready,
            Ok(ImportEvent::Ready { job_id, generation, backend })
                if job_id == request.job_id && generation == request.generation
                    && backend.starts_with("CPU") && backend.contains("worker CUDA absent")
                    && backend.contains("CPU prend le relais")
        ));
        let identity = std::fs::read_to_string(&marker).unwrap();
        assert!(identity.starts_with("701:1:"));
        stop_worker(&mut runtime, request.job_id, request.generation);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn auto_cpu_startup_failure_ends_without_another_attempt() {
        let root = test_artifact_root("whisper-auto-cpu-fail");
        std::fs::create_dir_all(&root).unwrap();
        let marker = root.join("cpu-attempt");
        let cpu = compile_fake_worker(&root, "cpu-fail", &marker);
        let gate = Arc::new(PublicationGate::default());
        gate.activate();
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu,
                gpu: root.join("missing-gpu.exe"),
            },
            gate,
        );
        let request = prepared_import(JobId(709));
        runtime.start_worker(request.clone()).unwrap();
        let failure = poll_until(&mut runtime, Duration::from_secs(3), |event| event.is_err());
        assert!(matches!(failure, Err((job_id, generation, _))
            if (job_id, generation) == (request.job_id, request.generation)));
        assert_eq!(runtime.active.as_ref().map(|active| active.backend), None);
        assert!(
            std::fs::read_to_string(&marker)
                .unwrap()
                .starts_with("709:1:")
        );
        if let Some(transport) = runtime.transport.as_mut() {
            transport.abort();
        }
        drop(runtime);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn auto_gpu_ready_is_attested_and_explicitly_admitted() {
        let root = test_artifact_root("whisper-auto-gpu-ready");
        std::fs::create_dir_all(&root).unwrap();
        let marker = root.join("begin-import");
        let gpu = compile_fake_worker(&root, "gpu-ready", &marker);
        let gate = Arc::new(PublicationGate::default());
        gate.activate();
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: root.join("unused-cpu.exe"),
                gpu,
            },
            gate,
        );
        let request = prepared_import(JobId(706));
        runtime
            .start_worker_with_selection(request.clone(), Some(Some(fake_gpu_candidate())))
            .unwrap();
        let ready = poll_until(&mut runtime, Duration::from_secs(3), |event| {
            matches!(event, Ok(ImportEvent::Ready { .. }) | Err(_))
        });
        assert!(matches!(ready, Ok(ImportEvent::Ready { backend, .. }) if backend == "CUDA"));
        let started = std::time::Instant::now();
        while !marker.is_file() && started.elapsed() < Duration::from_secs(3) {
            let _ = runtime.poll();
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(
            marker.is_file(),
            "BeginImport must reach the attested GPU worker"
        );
        let started = std::time::Instant::now();
        while !runtime.active.as_ref().unwrap().startup_ready
            && started.elapsed() < Duration::from_secs(3)
        {
            let _ = runtime.poll();
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(runtime.active.as_ref().unwrap().startup_ready);
        runtime.transport.as_mut().unwrap().abort();
        runtime.transport = None;
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn auto_import_gpu_exit_after_ready_before_begin_retries_once_on_cpu() {
        let root = test_artifact_root("whisper-auto-gpu-exit-before-begin");
        std::fs::create_dir_all(&root).unwrap();
        let gpu_begin = root.join("gpu-begin");
        let cpu_started = root.join("cpu-started");
        let cpu = compile_fake_worker(&root, "cpu-count", &cpu_started);
        let gpu = compile_fake_worker(&root, "gpu-ready-exit", &gpu_begin);
        let gate = Arc::new(PublicationGate::default());
        gate.activate();
        let mut runtime = ImportRuntime::new(WorkerPaths { cpu, gpu }, gate);
        let request = prepared_import(JobId(709));
        runtime
            .start_worker_with_selection(request.clone(), Some(Some(fake_gpu_candidate())))
            .unwrap();
        let ready = poll_until(&mut runtime, Duration::from_secs(5), |event| {
            matches!(event, Ok(ImportEvent::Ready { backend, .. }) if backend.starts_with("CPU"))
                || event.is_err()
        });
        assert!(
            matches!(&ready, Ok(ImportEvent::Ready { job_id, generation, backend })
            if *job_id == request.job_id && *generation == request.generation
                && backend.starts_with("CPU") && backend.contains("GPU non utilisable")
                && backend.contains("CPU prend le relais")),
            "unexpected recovery result: {ready:?}"
        );
        let active = runtime.active.as_ref().unwrap();
        assert_eq!(active.backend, BackendKind::Cpu);
        assert!(cpu_started.is_file(), "CPU worker reached Ready");
        assert_eq!(
            std::fs::read_to_string(&cpu_started)
                .unwrap()
                .lines()
                .count(),
            1,
            "the single CPU fallback starts once"
        );
        assert!(!gpu_begin.exists(), "GPU never admitted audio processing");
        stop_worker(&mut runtime, request.job_id, request.generation);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn auto_import_unlaunchable_gpu_uses_one_cpu_attempt() {
        let root = test_artifact_root("whisper-auto-gpu-spawn");
        std::fs::create_dir_all(&root).unwrap();
        let marker = root.join("cpu-ready");
        let cpu = compile_fake_worker(&root, "cpu", &marker);
        let gpu = root.join("broken-gpu.exe");
        std::fs::write(&gpu, b"not an executable").unwrap();
        let gate = Arc::new(PublicationGate::default());
        gate.activate();
        let mut runtime = ImportRuntime::new(WorkerPaths { cpu, gpu }, gate);
        let request = prepared_import(JobId(708));
        runtime.start_worker(request.clone()).unwrap();
        let ready = poll_until(&mut runtime, Duration::from_secs(3), |event| {
            matches!(event, Ok(ImportEvent::Ready { .. }) | Err(_))
        });
        assert!(matches!(ready, Ok(ImportEvent::Ready { backend, .. })
            if backend.starts_with("CPU") && backend.contains("lancement CUDA impossible")
                && backend.contains("CPU prend le relais")));
        assert!(marker.is_file());
        stop_worker(&mut runtime, request.job_id, request.generation);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn gpu_import_failed_then_eof_retries_cpu_and_accepts_same_job_ready() {
        let root =
            std::env::temp_dir().join(format!("whisper-import-retry-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let marker = root.join("cpu-ready");
        let model_path = root.join("verified-model.bin");
        let model_bytes = b"verified model fixture";
        std::fs::write(&model_path, model_bytes).unwrap();
        let model_sha256: [u8; 32] = Sha256::digest(model_bytes).into();
        let cpu = compile_fake_worker(&root, "cpu", &marker);
        let gpu = compile_verified_gpu_init_failure_worker(
            &root,
            &model_path,
            model_bytes,
            model_sha256,
            "BackendMismatch",
        );
        let gate = Arc::new(PublicationGate::default());
        gate.activate();
        let mut runtime = ImportRuntime::new(WorkerPaths { cpu, gpu }, gate);
        let request = prepared_import(JobId(702));
        let mut request = request;
        request.model_path = model_path.to_string_lossy().into_owned();
        request.model_sha256 = model_sha256;
        runtime.start_worker(request.clone()).unwrap();
        assert_eq!(runtime.active.as_ref().unwrap().backend, BackendKind::Cuda);
        let ready = poll_until(&mut runtime, Duration::from_secs(5), |event| {
            matches!(event, Ok(ImportEvent::Ready { .. }))
        });
        assert!(matches!(
            ready,
            Ok(ImportEvent::Ready { job_id, generation, backend })
                if job_id == request.job_id && generation == request.generation
                    && backend.starts_with("CPU") && backend.contains("GPU non utilisable")
                    && backend.contains("CPU prend le relais")
        ));
        assert_eq!(runtime.active.as_ref().unwrap().backend, BackendKind::Cpu);
        assert!(
            std::fs::read_to_string(&marker)
                .unwrap()
                .starts_with("702:1:")
        );
        assert_eq!(std::fs::read_to_string(&marker).unwrap().lines().count(), 1);
        stop_worker(&mut runtime, request.job_id, request.generation);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn gpu_live_failed_then_eof_retries_cpu_and_accepts_ready_before_capture_activation() {
        let root = std::env::temp_dir().join(format!("whisper-live-retry-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let marker = root.join("cpu-ready");
        let model_path = root.join("verified-model.bin");
        let model_bytes = b"verified model fixture";
        std::fs::write(&model_path, model_bytes).unwrap();
        let model_sha256: [u8; 32] = Sha256::digest(model_bytes).into();
        let cpu = compile_fake_worker(&root, "cpu-marker", &marker);
        let gpu = compile_verified_gpu_init_failure_worker(
            &root,
            &model_path,
            model_bytes,
            model_sha256,
            "BackendUnavailable",
        );
        let gate = Arc::new(PublicationGate::default());
        gate.activate();
        let mut runtime = ImportRuntime::new(WorkerPaths { cpu, gpu }, gate);
        let request = whisper_core::LiveRequest {
            job_id: JobId(704),
            generation: Generation::first(),
            model_path: model_path.to_string_lossy().into_owned(),
            model_sha256,
            destination: root.to_string_lossy().into_owned(),
            group_offset_samples: 0,
            auto_stop_after_speech_samples: None,
            microphone: Default::default(),
            microphone_gain_db: 0,
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Auto,
            },
        };
        runtime.start_live(request.clone(), None).unwrap();
        let observed = poll_until(&mut runtime, Duration::from_secs(5), |event| {
            matches!(event, Ok(ImportEvent::Ready { backend, .. }) if backend.starts_with("CPU"))
                || event.is_err()
        });
        assert_eq!(runtime.active.as_ref().unwrap().backend, BackendKind::Cpu);
        assert!(
            std::fs::read_to_string(&marker)
                .unwrap()
                .starts_with("704:1:")
        );
        if let Ok(ImportEvent::Ready {
            job_id,
            generation,
            backend,
        }) = observed
        {
            assert_eq!((job_id, generation), (request.job_id, request.generation));
            assert!(backend.starts_with("CPU"));
        } else if let Err((job_id, generation, message)) = observed {
            assert_eq!((job_id, generation), (request.job_id, request.generation));
            assert!(
                !message.contains("WorkerExited"),
                "CPU Ready was consumed before capture activation failed: {message}"
            );
        } else {
            panic!("unexpected event while awaiting CPU Ready after GPU EOF");
        }
        stop_worker(&mut runtime, request.job_id, request.generation);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn forced_gpu_initialization_failure_waits_for_explicit_cpu_choice() {
        let root =
            std::env::temp_dir().join(format!("whisper-forced-gpu-choice-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let marker = root.join("cpu-ready");
        let model_path = root.join("verified-model.bin");
        let model_bytes = b"verified model fixture";
        std::fs::write(&model_path, model_bytes).unwrap();
        let model_sha256: [u8; 32] = Sha256::digest(model_bytes).into();
        let cpu = compile_fake_worker(&root, "cpu", &marker);
        let gpu = compile_verified_gpu_init_failure_worker(
            &root,
            &model_path,
            model_bytes,
            model_sha256,
            "BackendUnavailable",
        );
        let gate = Arc::new(PublicationGate::default());
        gate.activate();
        let mut runtime = ImportRuntime::new(WorkerPaths { cpu, gpu }, gate);
        let request = whisper_core::LiveRequest {
            job_id: JobId(705),
            generation: Generation::first(),
            model_path: model_path.to_string_lossy().into_owned(),
            model_sha256,
            destination: root.to_string_lossy().into_owned(),
            group_offset_samples: 0,
            auto_stop_after_speech_samples: None,
            microphone: Default::default(),
            microphone_gain_db: 0,
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Gpu,
            },
        };
        assert!(
            runtime
                .start_live(request.clone(), None)
                .unwrap_err()
                .2
                .contains("selected GPU has no current identity proof")
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn stop_latched_after_gpu_failure_prevents_cpu_worker_launch() {
        let root =
            std::env::temp_dir().join(format!("whisper-stop-no-retry-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let marker = root.join("cpu-ready");
        let cpu = compile_fake_worker(&root, "cpu", &marker);
        let gpu = compile_fake_worker(&root, "gpu", &root.join("gpu-unused"));
        let gate = Arc::new(PublicationGate::default());
        gate.activate();
        let mut runtime = ImportRuntime::new(WorkerPaths { cpu, gpu }, gate.clone());
        let request = prepared_import(JobId(703));
        runtime.start_worker(request.clone()).unwrap();
        let _ = poll_until(&mut runtime, Duration::from_secs(3), |event| {
            matches!(event, Ok(ImportEvent::LiveAttemptStarting { .. }))
        });
        gate.request_stop().unwrap();
        let stopped = poll_until(&mut runtime, Duration::from_secs(5), |event| {
            matches!(event, Ok(ImportEvent::Stopped { .. }) | Err(_))
        });
        assert!(matches!(stopped, Ok(ImportEvent::Stopped { .. })));
        assert!(runtime.active.is_none());
        assert!(!marker.exists(), "CPU retry must not launch after Stop");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn auto_import_gpu_failure_retains_request_until_correlated_stop_for_cpu_retry() {
        let mut request = test_request(JobId(88), Generation::first());
        request.config.compute = ComputeChoice::Auto;
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("cpu.exe"),
                gpu: PathBuf::from("gpu.exe"),
            },
            Arc::new(PublicationGate::default()),
        );
        runtime.active = Some(ImportRequestState {
            job_id: request.job_id,
            generation: request.generation,
            instance_id: 9,
            backend: BackendKind::Cuda,
            attempt_number: 0,
            import_request: Some(request.clone()),
            recovery: LiveRecoveryState::None,
            startup_ready: false,
            gpu_candidate: None,
            gpu_attested: false,
        });
        let unbound = WorkerEvent::Failed {
            job_id: None,
            generation: None,
            instance_id: None,
            code: whisper_core::ipc::WorkerErrorCode::BackendUnavailable,
            message: "GPU unavailable before Ready".into(),
        };
        let bound = bind_unidentified_startup_failure(
            unbound,
            (request.job_id, request.generation, 9),
            false,
        );
        let WorkerEvent::Failed {
            job_id: Some(job_id),
            generation: Some(generation),
            instance_id: Some(instance_id),
            code,
            message,
        } = bound
        else {
            panic!("unidentified startup failure must bind to the launched attempt");
        };
        assert_eq!(
            (job_id, generation, instance_id),
            (request.job_id, request.generation, 9)
        );
        assert_eq!(code, whisper_core::ipc::WorkerErrorCode::BackendUnavailable);

        let event = runtime
            .handle_worker_failure(job_id, generation, code, message)
            .unwrap();
        assert!(matches!(event, ImportEvent::LiveAttemptStarting { .. }));
        let active = runtime.active.as_ref().unwrap();
        assert_eq!(active.recovery, LiveRecoveryState::AutoRetryPending);
        assert_eq!(active.import_request.as_ref(), Some(&request));
        assert!(
            runtime.transport.is_none(),
            "retry must wait for correlated Stopped and child exit"
        );

        let stale = WorkerEvent::Failed {
            job_id: Some(request.job_id),
            generation: Some(request.generation),
            instance_id: Some(8),
            code: whisper_core::ipc::WorkerErrorCode::BackendUnavailable,
            message: "stale GPU attempt".into(),
        };
        let stale_after_binding = bind_unidentified_startup_failure(
            stale.clone(),
            (request.job_id, request.generation, 9),
            false,
        );
        assert_eq!(stale_after_binding, stale);
        assert_ne!(
            worker_event_identity(&stale_after_binding),
            Some((request.job_id, request.generation, 9)),
            "correlated stale failures remain rejectable"
        );
        let after_ready = WorkerEvent::Failed {
            job_id: None,
            generation: None,
            instance_id: None,
            code: whisper_core::ipc::WorkerErrorCode::BackendUnavailable,
            message: "failure after Ready".into(),
        };
        assert_eq!(
            bind_unidentified_startup_failure(
                after_ready.clone(),
                (request.job_id, request.generation, 9),
                true,
            ),
            after_ready,
            "identity binding is limited to the startup phase"
        );
    }

    #[test]
    fn auto_import_does_not_retry_cpu_after_gpu_ready() {
        let request = prepared_import(JobId(707));
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("cpu.exe"),
                gpu: PathBuf::from("gpu.exe"),
            },
            Arc::new(PublicationGate::default()),
        );
        runtime.active = Some(ImportRequestState {
            job_id: request.job_id,
            generation: request.generation,
            instance_id: 10,
            backend: BackendKind::Cuda,
            attempt_number: 0,
            import_request: Some(request.clone()),
            recovery: LiveRecoveryState::None,
            startup_ready: true,
            gpu_candidate: None,
            gpu_attested: false,
        });
        let result = runtime.handle_worker_failure(
            request.job_id,
            request.generation,
            whisper_core::ipc::WorkerErrorCode::InferenceFailed,
            "injected failure after audio started".into(),
        );
        assert!(result.is_err());
        assert!(runtime.active.is_none());
        assert!(runtime.auto_fallback_reason.is_none());
    }

    #[test]
    fn queued_import_record_accepts_legacy_live_offset_field() {
        let request = ImportRequest {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
            job_id: JobId(1),
            generation: Generation::first(),
            source_path: "queued.wav".into(),
            source_sha256: None,
            source_samples: None,
            model_path: "model.bin".into(),
            model_sha256: [1; 32],
            destination: "archive".into(),
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Cpu,
            },
        };
        let mut legacy = serde_json::to_value(&request).unwrap();
        legacy
            .as_object_mut()
            .unwrap()
            .insert("group_offset_samples".into(), serde_json::json!(1234));
        assert_eq!(
            serde_json::from_value::<ImportRequest>(legacy).unwrap(),
            request
        );
    }

    #[test]
    fn pending_reservation_rejects_three_event_burst_at_twenty_slots() {
        let burst = ImportEffect::ResumeInterrupted {
            previous: Box::new(test_request(JobId(1), Generation::first())),
            request: test_request(JobId(2), Generation::first()),
        };
        assert_eq!(effect_event_reservation(&burst), 3);
        assert!(20 + effect_event_reservation(&burst) > EVENTS_PER_STAGE);
        assert!(18 + effect_event_reservation(&burst) <= EVENTS_PER_STAGE);
    }

    #[test]
    fn stale_mp3_and_failed_events_have_old_attempt_identity() {
        let old = (JobId(7), Generation::first(), 41);
        let packet = WorkerEvent::LiveMp3Packet {
            job_id: old.0,
            generation: old.1,
            instance_id: old.2,
            sequence: 1,
            bytes: vec![1, 2, 3],
        };
        let failed = WorkerEvent::Failed {
            job_id: Some(old.0),
            generation: Some(old.1),
            instance_id: Some(old.2),
            code: whisper_core::ipc::WorkerErrorCode::InvalidRequest,
            message: "old attempt failed".into(),
        };
        let current = (JobId(7), Generation::first(), 42);
        assert_ne!(worker_event_identity(&packet), Some(current));
        assert_ne!(worker_event_identity(&failed), Some(current));
        assert!(worker_event_is_stale(&packet, current));
        assert!(worker_event_is_stale(&failed, current));
    }

    #[test]
    fn failed_worker_message_does_not_hide_later_correlated_stop() {
        let id = (JobId(8), Generation::first(), 51);
        let failed = WorkerEvent::Failed {
            job_id: Some(id.0),
            generation: Some(id.1),
            instance_id: Some(id.2),
            code: whisper_core::ipc::WorkerErrorCode::InvalidRequest,
            message: "reported failure".into(),
        };
        let stopped = WorkerEvent::Stopped {
            job_id: id.0,
            generation: id.1,
            instance_id: id.2,
        };
        let mut bytes = Vec::new();
        write_frame(&mut bytes, &IpcEnvelope::new(failed)).unwrap();
        write_frame(&mut bytes, &IpcEnvelope::new(stopped.clone())).unwrap();
        let (tx, rx) = mpsc::sync_channel(4);
        read_events(Cursor::new(bytes), tx);
        assert!(matches!(
            rx.recv().unwrap(),
            WorkerReaderMessage::Event(Ok(WorkerEvent::Failed { .. }))
        ));
        assert_eq!(rx.recv().unwrap(), WorkerReaderMessage::Event(Ok(stopped)));
    }

    #[test]
    fn stale_stopped_does_not_hide_later_correlated_stop() {
        let job_id = JobId(8);
        let generation = Generation::first();
        let stale = WorkerEvent::Stopped {
            job_id,
            generation,
            instance_id: 52,
        };
        let current = WorkerEvent::Stopped {
            job_id,
            generation,
            instance_id: 51,
        };
        let mut bytes = Vec::new();
        write_frame(&mut bytes, &IpcEnvelope::new(stale.clone())).unwrap();
        write_frame(&mut bytes, &IpcEnvelope::new(current.clone())).unwrap();
        let (tx, rx) = mpsc::sync_channel(3);
        read_events(Cursor::new(bytes), tx);
        assert_eq!(rx.recv().unwrap(), WorkerReaderMessage::Event(Ok(stale)));
        assert_eq!(rx.recv().unwrap(), WorkerReaderMessage::Event(Ok(current)));
        assert!(matches!(
            rx.recv().unwrap(),
            WorkerReaderMessage::TransportEnded
        ));
    }

    #[test]
    fn worker_reader_signals_transport_end_after_session_end() {
        let mut bytes = Vec::new();
        write_frame(
            &mut bytes,
            &IpcEnvelope::new(WorkerEvent::End {
                job_id: JobId(18),
                generation: Generation::first(),
                instance_id: 1,
                last_sequence: 0,
                last_offset: 0,
            }),
        )
        .unwrap();
        let (tx, rx) = mpsc::sync_channel(2);
        read_events(Cursor::new(bytes), tx);
        assert!(matches!(
            rx.recv().unwrap(),
            WorkerReaderMessage::Event(Ok(WorkerEvent::End {
                job_id: JobId(18),
                ..
            }))
        ));
        assert_eq!(rx.recv().unwrap(), WorkerReaderMessage::TransportEnded);
    }

    #[test]
    fn slow_worker_backpressure_keeps_command_queue_bounded_and_retries_window() {
        let (sender, receiver) = mpsc::sync_channel(1);
        let command = || WorkerCommand::LiveWindow {
            job_id: JobId(1),
            generation: Generation::first(),
            instance_id: 1,
            sequence: 1,
            range: whisper_core::SourceRange::new(0, 80_000, 16_000).unwrap(),
            samples: vec![1; 80_000],
        };
        try_enqueue_bounded(&sender, command()).unwrap();
        let retry = try_enqueue_bounded(&sender, command()).unwrap_err();
        assert!(matches!(
            retry,
            EnqueueError::Full(command) if matches!(*command, WorkerCommand::LiveWindow { .. })
        ));
        drop(receiver.recv().unwrap());
        assert!(matches!(receiver.try_recv(), Err(TryRecvError::Empty)));
        try_enqueue_bounded(&sender, command()).unwrap();
        assert!(matches!(
            receiver.try_recv(),
            Ok(WorkerCommand::LiveWindow { .. })
        ));
    }

    #[test]
    fn capture_failure_stop_is_retained_when_data_channel_is_full() {
        let (data_tx, data_rx) = mpsc::sync_channel(1);
        let (control_tx, control_rx) = mpsc::sync_channel(1);
        let window = WorkerCommand::LiveWindow {
            job_id: JobId(2),
            generation: Generation::first(),
            instance_id: 1,
            sequence: 1,
            range: whisper_core::SourceRange::new(0, 80_000, 16_000).unwrap(),
            samples: vec![2; 80_000],
        };
        try_enqueue_worker_command(&data_tx, &control_tx, window).unwrap();
        let stop = WorkerCommand::Stop {
            job_id: JobId(2),
            generation: Generation::first(),
        };
        try_enqueue_worker_command(&data_tx, &control_tx, stop.clone()).unwrap();
        assert!(matches!(
            try_enqueue_worker_command(&data_tx, &control_tx, stop),
            Err(EnqueueError::Full(command)) if matches!(*command, WorkerCommand::Stop { .. })
        ));
        assert!(matches!(
            data_rx.try_recv(),
            Ok(WorkerCommand::LiveWindow { .. })
        ));
        assert!(matches!(
            control_rx.try_recv(),
            Ok(WorkerCommand::Stop { .. })
        ));
    }

    #[test]
    fn stdin_frames_preserve_live_start_and_admitted_window_before_normal_stop() {
        let (commands_tx, commands_rx) = mpsc::sync_channel(4);
        let (_emergency_tx, emergency_rx) = mpsc::sync_channel(1);
        let job_id = JobId(22);
        let generation = Generation::first();
        let config = JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        };
        try_enqueue_bounded(
            &commands_tx,
            WorkerCommand::StartLiveWorker {
                job_id,
                generation,
                instance_id: 1,
                model_path: "model.bin".into(),
                model_sha256: [22; 32],
                config,
            },
        )
        .unwrap();
        try_enqueue_bounded(
            &commands_tx,
            WorkerCommand::LiveWindow {
                job_id,
                generation,
                instance_id: 1,
                sequence: 1,
                range: whisper_core::SourceRange::new(0, 320, 16_000).unwrap(),
                samples: vec![3; 320],
            },
        )
        .unwrap();
        try_enqueue_bounded(&commands_tx, WorkerCommand::Stop { job_id, generation }).unwrap();

        let mut wire = Vec::new();
        for _ in 0..3 {
            let WorkerCommandPoll::Command(command) =
                take_next_worker_command(&commands_rx, &emergency_rx)
            else {
                panic!("expected a queued normal command");
            };
            write_frame(&mut wire, &IpcEnvelope::new(command)).unwrap();
        }
        assert!(matches!(emergency_rx.try_recv(), Err(TryRecvError::Empty)));

        let mut frames = BufReader::new(Cursor::new(wire));
        assert!(matches!(
            read_frame::<IpcEnvelope<WorkerCommand>>(&mut frames)
                .unwrap()
                .unwrap()
                .message,
            WorkerCommand::StartLiveWorker { .. }
        ));
        assert!(matches!(
            read_frame::<IpcEnvelope<WorkerCommand>>(&mut frames)
                .unwrap()
                .unwrap()
                .message,
            WorkerCommand::LiveWindow { sequence: 1, .. }
        ));
        assert!(matches!(
            read_frame::<IpcEnvelope<WorkerCommand>>(&mut frames)
                .unwrap()
                .unwrap()
                .message,
            WorkerCommand::Stop { .. }
        ));
    }

    #[test]
    fn stopped_worker_waits_until_live_segment_persistence_is_acknowledged() {
        let stopped = WorkerEvent::Stopped {
            job_id: JobId(3),
            generation: Generation::first(),
            instance_id: 1,
        };
        let mut deferred = Some(stopped.clone());
        assert!(take_ready_live_stop(1, &mut deferred).is_none());
        assert_eq!(deferred, Some(stopped.clone()));
        assert_eq!(take_ready_live_stop(0, &mut deferred), Some(stopped));
        assert!(deferred.is_none());
    }

    #[test]
    fn replay_windows_map_after_confirmed_prefix_without_duplicate_coverage() {
        assert_eq!(map_attempt_sequence(4, 3, 4), Some(4));
        assert_eq!(map_attempt_sequence(4, 3, 5), Some(5));
        assert_eq!(map_attempt_sequence(4, 3, 3), None);
        assert_eq!(map_attempt_sequence(u64::MAX, 0, 2), None);
    }

    #[test]
    fn queued_live_start_is_applied_before_an_immediate_stop() {
        struct Runtime(Arc<std::sync::Mutex<Vec<&'static str>>>);
        impl ServiceRuntime for Runtime {
            fn apply_effect(&mut self, effect: ImportEffect, pending: &mut VecDeque<ImportEvent>) {
                match effect {
                    ImportEffect::StartLive(request) => {
                        self.0.lock().unwrap().push("start");
                        pending.push_back(ImportEvent::Ready {
                            job_id: request.job_id,
                            generation: request.generation,
                            backend: "CPU".into(),
                        });
                    }
                    ImportEffect::Stop { job_id, generation } => {
                        self.0.lock().unwrap().push("stop");
                        pending.push_back(ImportEvent::Stopped { job_id, generation });
                    }
                    _ => {}
                }
            }

            fn poll_event(&mut self) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
                None
            }
        }

        let job_id = JobId(4);
        let generation = Generation::first();
        let (effect_tx, effect_rx) = mpsc::sync_channel(2);
        let (stop_tx, stop_rx) = mpsc::sync_channel(1);
        let (event_tx, event_rx) = mpsc::sync_channel(4);
        drop(event_rx);
        effect_tx
            .send(ImportEffect::StartLive(whisper_core::LiveRequest {
                job_id,
                generation,
                model_path: "model.bin".into(),
                model_sha256: [0; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                microphone_gain_db: 0,
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            }))
            .unwrap();
        stop_tx
            .send(ImportEffect::Stop { job_id, generation })
            .unwrap();
        let order = Arc::new(std::sync::Mutex::new(Vec::new()));
        let runtime = Runtime(order.clone());
        std::thread::spawn(move || service_loop(effect_rx, stop_rx, event_tx, runtime))
            .join()
            .unwrap();
        assert_eq!(*order.lock().unwrap(), vec!["start", "stop"]);
    }

    #[test]
    fn gpu_choice_keeps_drained_counters_ahead_of_following_events() {
        let job_id = JobId(20);
        let generation = Generation::first();
        let mut stop_events = VecDeque::new();
        stop_events.push_back(ImportEvent::LiveFinalizing { job_id, generation });
        stop_events.push_back(ImportEvent::LiveCounters {
            job_id,
            generation,
            captured_samples: 640,
            admitted_samples: 640,
            audio_durable_samples: 640,
            confirmed_fragment_samples: 0,
            speech_samples: 0,
            diagnostic: None,
        });
        let gate = Arc::new(PublicationGate::default());
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu.exe"),
                gpu: PathBuf::from("unused-gpu.exe"),
            },
            gate,
        );
        runtime.pending_recovery_counters = recovery_stop_counters(stop_events);
        assert!(matches!(
            runtime.poll(),
            Some(Ok(ImportEvent::LiveCounters {
                audio_durable_samples: 640,
                ..
            }))
        ));
        assert!(runtime.pending_recovery_counters.is_none());
    }

    #[test]
    fn stop_counters_reach_application_before_final_window() {
        struct Runtime {
            stopped: bool,
            window_sent: bool,
        }
        impl ServiceRuntime for Runtime {
            fn apply_effect(&mut self, effect: ImportEffect, pending: &mut VecDeque<ImportEvent>) {
                if let ImportEffect::Stop { job_id, generation } = effect {
                    self.stopped = true;
                    pending.push_back(ImportEvent::LiveCounters {
                        job_id,
                        generation,
                        captured_samples: 254_240,
                        admitted_samples: 254_240,
                        audio_durable_samples: 254_240,
                        confirmed_fragment_samples: 160_000,
                        speech_samples: 0,
                        diagnostic: None,
                    });
                    pending.push_back(ImportEvent::LiveFinalizing { job_id, generation });
                    assert!(pending.len() <= EVENTS_PER_STAGE);
                }
            }

            fn poll_event(&mut self) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
                if !self.stopped {
                    return None;
                }
                if self.window_sent {
                    return Some(Ok(ImportEvent::History(Vec::new())));
                }
                self.window_sent = true;
                Some(Ok(ImportEvent::WindowFinished {
                    job_id: JobId(20),
                    generation: Generation::first(),
                    window_sequence: 4,
                    confirmed_samples: 254_240,
                }))
            }
        }

        let (effect_tx, effect_rx) = mpsc::sync_channel(1);
        let (stop_tx, stop_rx) = mpsc::sync_channel(1);
        let (event_tx, event_rx) = mpsc::sync_channel(EVENTS_PER_STAGE);
        drop(effect_tx);
        stop_tx
            .send(ImportEffect::Stop {
                job_id: JobId(20),
                generation: Generation::first(),
            })
            .unwrap();
        let service = thread::spawn(move || {
            service_loop(
                effect_rx,
                stop_rx,
                event_tx,
                Runtime {
                    stopped: false,
                    window_sent: false,
                },
            )
        });
        assert!(matches!(
            event_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            ImportEvent::LiveCounters {
                audio_durable_samples: 254_240,
                ..
            }
        ));
        assert!(matches!(
            event_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            ImportEvent::LiveFinalizing { .. }
        ));
        assert!(matches!(
            event_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            ImportEvent::WindowFinished {
                confirmed_samples: 254_240,
                ..
            }
        ));
        drop(event_rx);
        service.join().unwrap();
    }

    #[test]
    fn direct_micro_start_and_single_stop_are_applied_in_order() {
        struct Runtime(mpsc::Sender<&'static str>);
        impl ServiceRuntime for Runtime {
            fn apply_effect(&mut self, effect: ImportEffect, pending: &mut VecDeque<ImportEvent>) {
                match effect {
                    ImportEffect::StartDirectLive(_) => self.0.send("start").unwrap(),
                    ImportEffect::Stop { job_id, generation } => {
                        self.0.send("stop").unwrap();
                        pending.push_back(ImportEvent::Stopped { job_id, generation });
                    }
                    _ => {}
                }
            }

            fn poll_event(&mut self) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
                Some(Ok(ImportEvent::History(Vec::new())))
            }
        }

        let (effect_tx, effect_rx) = mpsc::sync_channel(1);
        let (stop_tx, stop_rx) = mpsc::sync_channel(1);
        let (event_tx, event_rx) = mpsc::sync_channel(2);
        let (action_tx, action_rx) = mpsc::channel();
        let job_id = JobId(25);
        let generation = Generation::first();
        effect_tx
            .send(ImportEffect::StartDirectLive(whisper_core::LiveRequest {
                job_id,
                generation,
                model_path: "model.bin".into(),
                model_sha256: [25; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                microphone_gain_db: 0,
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            }))
            .unwrap();
        stop_tx
            .send(ImportEffect::Stop { job_id, generation })
            .unwrap();
        let service =
            thread::spawn(move || service_loop(effect_rx, stop_rx, event_tx, Runtime(action_tx)));

        assert_eq!(
            action_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            "start"
        );
        assert_eq!(
            action_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            "stop"
        );
        drop(event_rx);
        service.join().unwrap();
    }

    #[test]
    fn saturated_pending_queue_cannot_start_a_live_job_after_stop_and_allows_next_start() {
        struct Runtime {
            gate: Arc<PublicationGate>,
            remaining_events: usize,
            saturated: Option<mpsc::SyncSender<()>>,
            actions: mpsc::Sender<&'static str>,
        }
        impl ServiceRuntime for Runtime {
            fn apply_effect(&mut self, effect: ImportEffect, pending: &mut VecDeque<ImportEvent>) {
                match effect {
                    ImportEffect::StartLive(_) => {
                        if self.gate.is_cancelled() {
                            self.actions.send("late-start-skipped").unwrap();
                        } else {
                            self.actions.send("new-start-accepted").unwrap();
                        }
                    }
                    ImportEffect::Stop { job_id, generation } => {
                        self.actions.send("stop-without-active-worker").unwrap();
                        pending.push_back(ImportEvent::Stopped { job_id, generation });
                    }
                    _ => {}
                }
            }

            fn poll_event(&mut self) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
                if self.remaining_events == 0 {
                    return None;
                }
                self.remaining_events -= 1;
                if self.remaining_events == 0
                    && let Some(saturated) = self.saturated.take()
                {
                    saturated.send(()).unwrap();
                }
                Some(Ok(ImportEvent::History(Vec::new())))
            }
        }

        let gate = Arc::new(PublicationGate::default());
        let (effect_tx, effect_rx) = mpsc::sync_channel(2);
        let (stop_tx, stop_rx) = mpsc::sync_channel(1);
        let (event_tx, event_rx) = mpsc::sync_channel(EVENTS_PER_STAGE);
        for _ in 0..EVENTS_PER_STAGE {
            event_tx.send(ImportEvent::History(Vec::new())).unwrap();
        }
        let (saturated_tx, saturated_rx) = mpsc::sync_channel(1);
        let (actions_tx, actions_rx) = mpsc::channel();
        let runtime = Runtime {
            gate: gate.clone(),
            remaining_events: EVENTS_PER_STAGE,
            saturated: Some(saturated_tx),
            actions: actions_tx,
        };
        let service = thread::spawn(move || service_loop(effect_rx, stop_rx, event_tx, runtime));
        saturated_rx.recv_timeout(Duration::from_secs(2)).unwrap();

        let first_id = JobId(24);
        let generation = Generation::first();
        gate.activate();
        effect_tx
            .send(ImportEffect::StartLive(whisper_core::LiveRequest {
                job_id: first_id,
                generation,
                model_path: "model.bin".into(),
                model_sha256: [24; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                microphone_gain_db: 0,
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            }))
            .unwrap();
        gate.request_stop().unwrap();
        stop_tx
            .send(ImportEffect::Stop {
                job_id: first_id,
                generation,
            })
            .unwrap();

        for _ in 0..EVENTS_PER_STAGE {
            event_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        }
        let mut saw_stopped = false;
        while !saw_stopped {
            saw_stopped = matches!(
                event_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
                ImportEvent::Stopped { job_id, .. } if job_id == first_id
            );
        }
        assert_eq!(
            actions_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            "stop-without-active-worker"
        );
        assert_eq!(
            actions_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            "late-start-skipped"
        );

        gate.activate();
        effect_tx
            .send(ImportEffect::StartLive(whisper_core::LiveRequest {
                job_id: JobId(25),
                generation,
                model_path: "model.bin".into(),
                model_sha256: [25; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                microphone_gain_db: 0,
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            }))
            .unwrap();
        assert_eq!(
            actions_rx.recv_timeout(Duration::from_secs(2)).unwrap(),
            "new-start-accepted"
        );
        gate.request_stop().unwrap();
        stop_tx
            .send(ImportEffect::Stop {
                job_id: JobId(25),
                generation,
            })
            .unwrap();
        drop(event_rx);
        service.join().unwrap();
    }

    #[test]
    fn stop_before_commit_atomically_prevents_publication() {
        let gate = PublicationGate::default();
        gate.activate();
        assert!(gate.request_stop().unwrap());
        let mut disk_commit = false;
        assert_eq!(
            gate.commit(|| {
                disk_commit = true;
                Ok(())
            }),
            Err("Cancelled: publication invalidated by Stop".into())
        );
        assert!(!disk_commit, "CURRENT commit must not run after Stop wins");
    }

    #[test]
    fn cancelled_gate_skips_live_start_before_any_worker_or_capture_side_effect() {
        let gate = Arc::new(PublicationGate::default());
        gate.activate();
        gate.request_stop().unwrap();
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("missing-cpu-worker"),
                gpu: PathBuf::from("missing-gpu-worker"),
            },
            gate,
        );
        let mut events = VecDeque::new();
        runtime.effect(
            ImportEffect::StartLive(whisper_core::LiveRequest {
                job_id: JobId(26),
                generation: Generation::first(),
                model_path: "model.bin".into(),
                model_sha256: [26; 32],
                destination: "unused-archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                microphone_gain_db: 0,
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            }),
            &mut events,
        );
        assert!(runtime.active.is_none());
        assert!(runtime.transport.is_none());
        assert!(runtime.pending_live.is_none());
        assert!(runtime.live.is_none());
        assert!(events.is_empty());
    }

    #[test]
    fn effect_queue_accepts_eight_and_rejects_the_ninth_without_blocking() {
        let gate = Arc::new(PublicationGate::default());
        let (mut io, effects, _, _, _, _, _) = AsyncImportIo::for_test(gate);
        for index in 0..8 {
            io.submit(ImportEffect::ScanHistory {
                destination: format!("archive-{index}"),
            })
            .unwrap();
        }
        assert_eq!(
            io.submit(ImportEffect::ScanHistory {
                destination: "archive-overflow".into(),
            }),
            Err(PortError::Unavailable),
            "the ninth queued effect is refused rather than blocking or growing the queue"
        );
        assert_eq!(effects.try_iter().count(), 8);
    }

    #[test]
    fn persist_segment_queue_full_is_distinct_from_disconnection() {
        let gate = Arc::new(PublicationGate::default());
        let (mut io, effects, _, _, _, _, _) = AsyncImportIo::for_test(gate);
        let segment = |sequence| WorkerSegment {
            job_id: JobId(904),
            generation: Generation::first(),
            instance_id: 1,
            segment_id: whisper_core::SegmentId(sequence),
            range: whisper_core::SourceRange::new((sequence - 1) * 80, sequence * 80, 16_000)
                .unwrap(),
            text: format!("segment-{sequence}"),
        };

        for sequence in 1..=8 {
            io.submit(ImportEffect::PersistSegment(segment(sequence)))
                .unwrap();
        }
        assert_eq!(
            io.submit(ImportEffect::PersistSegment(segment(9))),
            Err(PortError::Backpressure),
            "a full persistence queue is retryable"
        );
        assert_eq!(
            effects
                .try_iter()
                .filter_map(|effect| match effect {
                    ImportEffect::PersistSegment(segment) => Some(segment.segment_id.0),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            (1..=8).collect::<Vec<_>>()
        );

        let gate = Arc::new(PublicationGate::default());
        let (mut disconnected, effects, _, _, _, _, _) = AsyncImportIo::for_test(gate);
        drop(effects);
        assert_eq!(
            disconnected.submit(ImportEffect::PersistSegment(segment(1))),
            Err(PortError::Unavailable),
            "a disconnected consumer remains fatal and is never classified as backpressure"
        );
    }

    #[test]
    fn event_65_backpressures_all_production_stages_then_delivers_in_order() {
        struct WireRuntime {
            worker_events: Receiver<WorkerReaderMessage>,
            consumed: Arc<std::sync::atomic::AtomicUsize>,
            shutdown: Receiver<()>,
        }
        impl ServiceRuntime for WireRuntime {
            fn apply_effect(&mut self, _: ImportEffect, _: &mut VecDeque<ImportEvent>) {}

            fn poll_event(&mut self) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
                match self.worker_events.try_recv() {
                    Ok(WorkerReaderMessage::Event(Ok(WorkerEvent::Progress {
                        job_id,
                        generation,
                        completed_samples,
                        total_samples,
                        ..
                    }))) => {
                        self.consumed.fetch_add(1, Ordering::AcqRel);
                        Some(Ok(ImportEvent::Progress {
                            job_id,
                            generation,
                            completed_samples,
                            total_samples,
                        }))
                    }
                    Ok(WorkerReaderMessage::Event(Ok(WorkerEvent::Failed {
                        job_id: Some(job_id),
                        generation: Some(generation),
                        message,
                        ..
                    }))) => {
                        self.consumed.fetch_add(1, Ordering::AcqRel);
                        Some(Err((job_id, generation, message)))
                    }
                    Ok(WorkerReaderMessage::Event(Err(error))) => Some(Err((
                        JobId(903),
                        Generation::first(),
                        format!("worker reader: {error:?}"),
                    ))),
                    Ok(WorkerReaderMessage::Event(Ok(_)) | WorkerReaderMessage::TransportEnded) => {
                        None
                    }
                    Err(TryRecvError::Empty) => match self.shutdown.try_recv() {
                        Ok(()) => Some(Ok(ImportEvent::History(Vec::new()))),
                        Err(_) => None,
                    },
                    Err(TryRecvError::Disconnected) => match self.shutdown.try_recv() {
                        Ok(()) => Some(Ok(ImportEvent::History(Vec::new()))),
                        Err(_) => None,
                    },
                }
            }
        }

        const BACKLOG: usize = EVENTS_PER_STAGE * 3 + 1;
        const EVENT_COUNT: usize = BACKLOG + 1;
        let job_id = JobId(903);
        let generation = Generation::first();
        let mut wire = Vec::new();
        for completed_samples in 1..=BACKLOG as u64 {
            let envelope = IpcEnvelope::new(WorkerEvent::Progress {
                job_id,
                generation,
                instance_id: 1,
                completed_samples,
                total_samples: EVENT_COUNT as u64,
            });
            serde_json::to_writer(&mut wire, &envelope).unwrap();
            wire.push(b'\n');
        }
        let failure = IpcEnvelope::new(WorkerEvent::Failed {
            job_id: Some(job_id),
            generation: Some(generation),
            instance_id: Some(1),
            code: whisper_core::ipc::WorkerErrorCode::Saturated,
            message: "65th event after bounded backlog".into(),
        });
        serde_json::to_writer(&mut wire, &failure).unwrap();
        wire.push(b'\n');

        let (mut io, effect_rx, stop_rx, app_tx, _, _, _) =
            AsyncImportIo::for_test(Arc::new(PublicationGate::default()));
        let app_capacity_probe = app_tx.clone();
        let (worker_tx, worker_rx) = mpsc::sync_channel(EVENTS_PER_STAGE);
        let consumed = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let consumed_probe = consumed.clone();
        let (shutdown_tx, shutdown_rx) = mpsc::channel();
        let service = thread::spawn(move || {
            service_loop(
                effect_rx,
                stop_rx,
                app_tx,
                WireRuntime {
                    worker_events: worker_rx,
                    consumed,
                    shutdown: shutdown_rx,
                },
            )
        });
        let (reader_done_tx, reader_done_rx) = mpsc::channel();
        let reader = thread::spawn(move || {
            read_events(Cursor::new(wire), worker_tx);
            reader_done_tx.send(()).unwrap();
        });

        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        while consumed_probe.load(Ordering::Acquire) < EVENTS_PER_STAGE * 2 {
            assert!(
                std::time::Instant::now() < deadline,
                "pipeline did not fill"
            );
            thread::sleep(Duration::from_millis(5));
        }
        assert!(matches!(
            app_capacity_probe.try_send(ImportEvent::History(Vec::new())),
            Err(TrySendError::Full(_))
        ));
        assert!(
            matches!(reader_done_rx.try_recv(), Err(mpsc::TryRecvError::Empty)),
            "the wire reader must be backpressured while all 64 backlog slots are occupied"
        );
        drop(app_capacity_probe);

        let mut progress = Vec::with_capacity(BACKLOG);
        let mut visible_failure = None;
        let deadline = std::time::Instant::now() + Duration::from_secs(3);
        while progress.len() + usize::from(visible_failure.is_some()) < EVENT_COUNT {
            assert!(
                std::time::Instant::now() < deadline,
                "drained events were lost"
            );
            match io.poll().unwrap() {
                Some(ImportEvent::Progress {
                    completed_samples, ..
                }) => progress.push(completed_samples),
                Some(ImportEvent::Failed { message, .. }) => visible_failure = Some(message),
                Some(other) => panic!("unexpected event after saturation: {other:?}"),
                None => thread::sleep(Duration::from_millis(1)),
            }
        }
        assert_eq!(progress, (1..=BACKLOG as u64).collect::<Vec<_>>());
        assert_eq!(
            visible_failure.as_deref(),
            Some("65th event after bounded backlog")
        );
        reader_done_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        reader.join().unwrap();
        drop(io);
        shutdown_tx.send(()).unwrap();
        service.join().unwrap();
    }

    #[test]
    fn stop_control_is_admitted_when_effect_queue_is_full() {
        let gate = Arc::new(PublicationGate::default());
        let (mut io, effects, stops, _, _, _, _) = AsyncImportIo::for_test(gate);
        let job_id = JobId(901);
        let generation = Generation::first();
        io.submit(ImportEffect::Prepare(test_request(job_id, generation)))
            .unwrap();
        for index in 0..7 {
            io.submit(ImportEffect::ScanHistory {
                destination: format!("archive-{index}"),
            })
            .unwrap();
        }
        assert_eq!(effects.try_iter().count(), 8);
        io.submit(ImportEffect::Stop { job_id, generation })
            .unwrap();
        assert!(matches!(stops.try_recv(), Ok(ImportEffect::Stop { .. })));
    }

    #[test]
    fn frame_limit_counts_newline_and_rejects_oversize_before_write() {
        let gate = Arc::new(PublicationGate::default());
        let (mut io, _effects, _, _, _, _, _) = AsyncImportIo::for_test(gate);
        let mut low = 0;
        let mut high = MAX_FRAME_BYTES;
        while low < high {
            let middle = (low + high).div_ceil(2);
            let effect = ImportEffect::ScanHistory {
                destination: "x".repeat(middle),
            };
            let encoded_len = serde_json::to_vec(&effect).unwrap().len() + 1;
            if encoded_len <= MAX_FRAME_BYTES {
                low = middle;
            } else {
                high = middle - 1;
            }
        }
        let accepted = ImportEffect::ScanHistory {
            destination: "x".repeat(low),
        };
        assert_eq!(
            serialize_frame(&accepted).unwrap().len() + 1,
            MAX_FRAME_BYTES
        );
        io.submit(accepted).unwrap();
        let rejected = ImportEffect::ScanHistory {
            destination: "x".repeat(low + 1),
        };
        assert_eq!(io.submit(rejected), Err(PortError::InvalidInput));
    }

    #[test]
    fn framed_ipc_rejects_incomplete_oversized_and_v1_messages() {
        let incomplete = b"{\"version\":2".as_slice();
        assert!(read_frame::<IpcEnvelope<WorkerEvent>>(&mut Cursor::new(incomplete)).is_err());

        let oversized = vec![b'x'; MAX_FRAME_BYTES];
        assert!(read_frame::<serde_json::Value>(&mut Cursor::new(oversized)).is_err());

        let v1 = b"{\"version\":1,\"message\":{}}\n".as_slice();
        let envelope = read_frame::<IpcEnvelope<serde_json::Value>>(&mut Cursor::new(v1))
            .unwrap()
            .unwrap();
        assert!(!envelope.is_supported());
    }

    #[test]
    fn worker_eof_without_end_is_reported_as_failure() {
        let (tx, rx) = mpsc::sync_channel(2);
        read_events(Cursor::new(Vec::<u8>::new()), tx);
        assert!(
            matches!(rx.try_recv(), Ok(WorkerReaderMessage::Event(Err(PortError::Failed(message)))) if message.contains("without End"))
        );
    }

    fn test_destination(job_id: JobId, generation: Generation) -> PathBuf {
        std::env::temp_dir()
            .join("worker-ipc-tests")
            .join(format!("{}-{job_id:?}-{generation:?}", std::process::id()))
    }

    fn test_request(job_id: JobId, generation: Generation) -> ImportRequest {
        ImportRequest {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
            job_id,
            generation,
            source_path: "source.wav".into(),
            source_sha256: None,
            source_samples: None,
            model_path: "model.bin".into(),
            model_sha256: [0; 32],
            destination: test_destination(job_id, generation)
                .to_string_lossy()
                .into_owned(),
            config: JobConfig {
                language: LanguageChoice::Manual("fr".into()),
                compute: ComputeChoice::Cpu,
            },
        }
    }
