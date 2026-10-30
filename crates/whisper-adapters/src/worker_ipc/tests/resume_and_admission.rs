    fn compile_copy_resume_worker(root: &Path, ready: &Path, release: &Path) -> PathBuf {
        let source = root.join("r7-resume-worker.rs");
        let executable = root.join("r7-resume-worker.exe");
        let ready = serde_json::to_string(&ready.to_string_lossy().to_string()).unwrap();
        let release = serde_json::to_string(&release.to_string_lossy().to_string()).unwrap();
        let program = r#"
use std::{io::{self, BufRead, Write}, thread, time::Duration};
const READY: &str = __READY__;
const RELEASE: &str = __RELEASE__;
fn field(line: &str, name: &str) -> String {
    let token = format!("\"{name}\":");
    line.split_once(&token).expect("wire field").1.trim_start()
        .chars().take_while(|c| c.is_ascii_digit()).collect()
}
fn emit(body: String) { println!("{{\"version\":2,\"message\":{{{body}}}}}"); io::stdout().flush().unwrap(); }
fn main() {
    let start = io::stdin().lock().lines().next().unwrap().unwrap();
    let job = field(&start, "job_id");
    let generation = field(&start, "generation");
    let instance = field(&start, "instance_id");
    let samples = field(&start, "expected_source_samples");
    emit(format!("\"Ready\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"backend\":\"Cpu\"}}"));
    std::fs::write(READY, b"ready").unwrap();
    while !std::path::Path::new(RELEASE).is_file() { thread::sleep(Duration::from_millis(5)); }
    emit(format!("\"Segment\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"segment_id\":1,\"range\":{{\"start_sample\":0,\"end_sample\":16000,\"sample_rate_hz\":16000}},\"text\":\"r7 confirmed transcript\"}}"));
    emit(format!("\"Segment\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"segment_id\":2,\"range\":{{\"start_sample\":16000,\"end_sample\":{samples},\"sample_rate_hz\":16000}},\"text\":\"r7 resumed transcript\"}}"));
    emit(format!("\"Progress\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"completed_samples\":{samples},\"total_samples\":{samples}}}"));
    emit(format!("\"End\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"last_sequence\":2,\"last_offset\":{samples}}}"));
}
"#
            .replace("__READY__", &ready)
            .replace("__RELEASE__", &release);
        std::fs::write(&source, program).unwrap();
        let output = Command::new("rustc")
            .args(["--edition=2021", "--crate-name", "whisper_r7_resume_worker"])
            .arg(&source)
            .arg("-o")
            .arg(&executable)
            .output()
            .expect("compile deterministic R7 resume worker");
        assert!(
            output.status.success(),
            "rustc fake worker failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        executable
    }

    fn write_copy_resume_wav(path: &Path, samples: u32) {
        use std::io::Write;
        let data_bytes = samples * 2;
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

    fn copy_resume_request(
        job_id: JobId,
        source: &Path,
        destination: &Path,
        model: &Path,
        model_sha256: [u8; 32],
    ) -> ImportRequest {
        ImportRequest {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
            job_id,
            generation: Generation::first(),
            source_path: source.to_string_lossy().into_owned(),
            source_sha256: None,
            source_samples: None,
            model_path: model.to_string_lossy().into_owned(),
            model_sha256,
            destination: destination.to_string_lossy().into_owned(),
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Cpu,
            },
        }
    }

    fn copy_resume_hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    fn copy_resume_generation_path(root: &Path, job_id: JobId, generation: Generation) -> PathBuf {
        root.join("transcriptions")
            .join(format!("{:032x}", job_id.0))
            .join(generation.get().to_string())
    }

    fn wait_copy_resume_prompt(
        app: &mut ImportApplication<AsyncImportIo>,
    ) -> whisper_core::ports::DuplicateImportPrompt {
        let deadline = Instant::now() + Duration::from_secs(60);
        loop {
            let _ = app.dispatch(AppCommand::Refresh);
            let view = app.view();
            if let Some(prompt) = view.duplicate_import_prompt {
                return prompt;
            }
            assert!(
                Instant::now() < deadline,
                "resume confirmation prompt timed out"
            );
            thread::sleep(Duration::from_millis(5));
        }
    }

    fn run_integrated_copy_resume_case(root: &Path, replace_existing: bool, model: &Path) {
        use whisper_core::ports::DuplicateImportChoice;

        let destination = root.join(if replace_existing { "replace" } else { "first" });
        let source = root.join(if replace_existing {
            "replace-source.wav"
        } else {
            "first-source.wav"
        });
        std::fs::create_dir_all(&destination).unwrap();
        write_copy_resume_wav(&source, 32_000);
        let model_sha256 = crate::model_catalog::cached_model_sha256(model).unwrap().1;
        let source_hash = |request: &ImportRequest| copy_resume_hex(&request.source_sha256.unwrap());

        let old_publications = if replace_existing {
            let mut previous = copy_resume_request(
                JobId(((std::process::id() as u128) << 64) | 0x771),
                &source,
                &destination,
                model,
                model_sha256,
            );
            ArchiveStore::identify_source(&mut previous).unwrap();
            let mut old_archive = ArchiveStore::new(&destination);
            let prepared = old_archive.prepare(previous).unwrap();
            old_archive
                .persist_segment(&whisper_core::ports::WorkerSegment {
                    job_id: prepared.job_id,
                    generation: prepared.generation,
                    instance_id: 1,
                    segment_id: whisper_core::SegmentId(1),
                    range: whisper_core::SourceRange::new(0, 32_000, 16_000).unwrap(),
                    text: "older published transcript".into(),
                })
                .unwrap();
            old_archive
                .publish(prepared.job_id, prepared.generation, 1)
                .unwrap();
            ArchiveStore::published_imports(&destination, &source_hash(&prepared)).unwrap()
        } else {
            Vec::new()
        };

        let mut interrupted = copy_resume_request(
            JobId(
                ((std::process::id() as u128) << 64) | if replace_existing { 0x772 } else { 0x773 },
            ),
            &source,
            &destination,
            model,
            model_sha256,
        );
        ArchiveStore::identify_source(&mut interrupted).unwrap();
        let choice = if replace_existing {
            DuplicateImportChoice::Replace
        } else {
            DuplicateImportChoice::CreateNew
        };
        if replace_existing {
            let basename = ArchiveStore::suggest_duplicate_export_basename(
                &destination,
                &interrupted.source_path,
            )
            .unwrap();
            ArchiveStore::reserve_duplicate_import(
                &destination,
                &interrupted,
                &source_hash(&interrupted),
                interrupted.source_samples.unwrap(),
                &old_publications,
                choice.clone(),
                &basename,
            )
            .unwrap();
        } else {
            crate::archive::ArchiveStore::reserve_new_import(
                &destination,
                &interrupted,
                &source_hash(&interrupted),
            )
            .unwrap();
        }
        let mut interrupted_archive = ArchiveStore::new(&destination);
        interrupted = interrupted_archive.prepare(interrupted).unwrap();
        interrupted_archive
            .persist_segment(&whisper_core::ports::WorkerSegment {
                job_id: interrupted.job_id,
                generation: interrupted.generation,
                instance_id: 1,
                segment_id: whisper_core::SegmentId(1),
                range: whisper_core::SourceRange::new(0, 16_000, 16_000).unwrap(),
                text: "r7 confirmed transcript".into(),
            })
            .unwrap();
        let generation =
            copy_resume_generation_path(&destination, interrupted.job_id, interrupted.generation);
        crate::archive::inject_copy_fallback_publication_cut(&destination, "srt");
        let interrupted_result =
            interrupted_archive.publish(interrupted.job_id, interrupted.generation, 1);
        assert!(
            interrupted_result
                .as_ref()
                .is_err_and(|error| error.contains("test interruption after final srt create_new")),
            "the fault must occur after final SRT create_new and before identity persistence: {interrupted_result:?}"
        );
        assert!(!generation.join("CURRENT").exists());
        assert_eq!(
            ArchiveStore::published_imports(&destination, &source_hash(&interrupted)).unwrap(),
            old_publications,
            "old active publications stay visible after the export-copy interruption"
        );

        let intent_path = destination
            .join(".whisper-publications/reservations")
            .join(format!("{:032x}-1.json", interrupted.job_id.0));
        let intent: serde_json::Value =
            serde_json::from_slice(&std::fs::read(intent_path).unwrap()).unwrap();
        let interrupted_basename = intent["export_basename"].as_str().unwrap().to_owned();
        let ambiguous_txt = destination
            .join("exports")
            .join(format!("{interrupted_basename}.txt"));
        let ambiguous_srt = destination
            .join("exports")
            .join(format!("{interrupted_basename}.srt"));
        let ambiguous_txt_bytes = std::fs::read(&ambiguous_txt).unwrap();
        let ambiguous_srt_bytes = std::fs::read(&ambiguous_srt).unwrap();
        let ambiguous_txt_identity = same_file::Handle::from_path(&ambiguous_txt).unwrap();
        let ambiguous_srt_identity = same_file::Handle::from_path(&ambiguous_srt).unwrap();
        let transcript_srt = std::fs::read(generation.join("transcript.srt")).unwrap();

        // This is an independently created, byte-identical third-party collision at
        // the next candidate name. The restart flow must suggest another name.
        let collision_basename =
            ArchiveStore::suggest_duplicate_export_basename(&destination, &interrupted.source_path)
                .unwrap();
        let third_party = destination
            .join("exports")
            .join(format!("{collision_basename}.srt"));
        std::fs::write(&third_party, &transcript_srt).unwrap();
        let third_party_identity = same_file::Handle::from_path(&third_party).unwrap();
        let third_party_bytes = std::fs::read(&third_party).unwrap();
        let final_basename =
            ArchiveStore::suggest_duplicate_export_basename(&destination, &interrupted.source_path)
                .unwrap();

        let mut queue = crate::queue_store::QueueStore::open(&destination).unwrap();
        queue.enqueue(interrupted.clone()).unwrap();
        queue
            .set_status(
                interrupted.job_id,
                interrupted.generation,
                crate::queue_store::QueueStatus::Interrupted,
            )
            .unwrap();

        let ready = root.join(format!("resume-ready-{}", interrupted.job_id.0));
        let release = root.join(format!("resume-release-{}", interrupted.job_id.0));
        let worker = compile_copy_resume_worker(root, &ready, &release);
        let mut app = ImportApplication::new(AsyncImportIo::start(worker));
        app.dispatch(AppCommand::ResumeInterrupted {
            request: interrupted.clone(),
        })
        .unwrap();
        let prompt = wait_copy_resume_prompt(&mut app);
        assert_eq!(prompt.resume_from, Some(interrupted.clone()));
        assert_eq!(prompt.export_basename, final_basename);
        assert!(
            prompt
                .revalidation_notice
                .as_deref()
                .is_some_and(|notice| { notice.contains("Confirmez explicitement") }),
            "ambiguous export must require visible reconfirmation"
        );
        assert_eq!(prompt.publications, old_publications);
        app.dispatch(AppCommand::ResolveDuplicateImport { prompt, choice })
            .unwrap();

        let deadline = Instant::now() + Duration::from_secs(60);
        while !ready.is_file() {
            assert!(
                Instant::now() < deadline,
                "resumed worker did not reach Ready; view={:?}",
                app.view()
            );
            let _ = app.dispatch(AppCommand::Refresh).unwrap();
            thread::sleep(Duration::from_millis(5));
        }
        let resumed = interrupted.clone_with_next_generation();
        let current_queue = crate::queue_store::QueueStore::open(&destination).unwrap();
        let row = current_queue
            .entries()
            .into_iter()
            .find(|entry| entry.request.job_id == resumed.job_id)
            .expect("resumed request is durable in queue");
        assert_eq!(row.request.generation, resumed.generation);
        assert_eq!(row.status, crate::queue_store::QueueStatus::Running);
        assert_eq!(
            ArchiveStore::published_imports(&destination, &source_hash(&interrupted)).unwrap(),
            old_publications,
            "Replace leaves all old publications active until the resumed generation commits"
        );
        assert_eq!(std::fs::read(&ambiguous_txt).unwrap(), ambiguous_txt_bytes);
        assert_eq!(std::fs::read(&ambiguous_srt).unwrap(), ambiguous_srt_bytes);
        assert_eq!(
            same_file::Handle::from_path(&ambiguous_txt).unwrap(),
            ambiguous_txt_identity
        );
        assert_eq!(
            same_file::Handle::from_path(&ambiguous_srt).unwrap(),
            ambiguous_srt_identity
        );

        std::fs::write(&release, b"continue").unwrap();
        let deadline = Instant::now() + Duration::from_secs(60);
        let completed = loop {
            let view = app.dispatch(AppCommand::Refresh).unwrap_or_else(|error| {
                panic!("resume refresh failed: {error:?}; view={:?}", app.view())
            });
            if view.active_job == Some((resumed.job_id, JobState::Complete)) {
                break view;
            }
            assert!(
                Instant::now() < deadline,
                "resumed worker did not publish; view={:?}",
                app.view()
            );
            thread::sleep(Duration::from_millis(5));
        };
        assert!(completed.last_result.is_some());
        let published =
            ArchiveStore::published_imports(&destination, &source_hash(&interrupted)).unwrap();
        assert_eq!(published.len(), 1);
        assert_eq!(
            (published[0].job_id, published[0].generation),
            (resumed.job_id, resumed.generation)
        );
        let resumed_generation =
            copy_resume_generation_path(&destination, resumed.job_id, resumed.generation);
        assert!(resumed_generation.join("CURRENT").is_file());
        assert_eq!(
            std::fs::read(resumed_generation.join("transcript.txt")).unwrap(),
            b"r7 confirmed transcript\nr7 resumed transcript"
        );
        assert!(
            destination
                .join("exports")
                .join(format!("{final_basename}.txt"))
                .is_file()
        );
        assert!(
            destination
                .join("exports")
                .join(format!("{final_basename}.srt"))
                .is_file()
        );
        assert_eq!(std::fs::read(&third_party).unwrap(), third_party_bytes);
        assert_eq!(
            same_file::Handle::from_path(&third_party).unwrap(),
            third_party_identity,
            "byte-identical third-party collision is never moved or adopted"
        );
        assert_eq!(std::fs::read(&ambiguous_txt).unwrap(), ambiguous_txt_bytes);
        assert_eq!(std::fs::read(&ambiguous_srt).unwrap(), ambiguous_srt_bytes);
        drop(app);
        crate::archive::clear_copy_fallback_publication_cut();
        std::fs::remove_dir_all(destination).unwrap();
    }

    trait NextGeneration {
        fn clone_with_next_generation(&self) -> Self;
    }

    impl NextGeneration for ImportRequest {
        fn clone_with_next_generation(&self) -> Self {
            let mut resumed = self.clone();
            resumed.generation = self.generation.next().unwrap();
            resumed
        }
    }

    #[test]
    fn r7_restart_resume_integrates_copy_crash_dialog_queue_and_publication() {
        let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let model = repository.join("models/ggml-large-v3-turbo.bin");
        if !model.is_file() {
            eprintln!("NOT RUN: local Turbo model missing for integrated R7 resume test");
            return;
        }
        let root = test_artifact_root("r7-integrated-copy-resume").join(format!(
            "{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        run_integrated_copy_resume_case(&root, false, &model);
        run_integrated_copy_resume_case(&root, true, &model);
        crate::archive::clear_copy_fallback_publication_cut();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn queued_request_identity_allows_only_compute_choice_and_verified_model_path_binding() {
        let mut durable = test_request(JobId(910), Generation::first());
        durable.source_sha256 = Some([1; 32]);
        durable.source_samples = Some(16_000);
        let mut requested = durable.clone();
        requested.config.compute = ComputeChoice::Cpu;
        assert!(queued_request_identity_matches(&durable, &requested, true));

        requested.config.language = LanguageChoice::Automatic;
        assert!(!queued_request_identity_matches(&durable, &requested, true));
        requested = durable.clone();
        requested.config.compute = ComputeChoice::Auto;
        requested.model_path = "verified-catalog-path.gguf".into();
        assert!(queued_request_identity_matches(&durable, &requested, true));
        assert!(!queued_request_identity_matches(
            &durable, &requested, false
        ));
    }

    #[test]
    fn queued_manual_gpu_preflight_requires_a_current_candidate_and_gpu_key() {
        let mut request = test_request(JobId(911), Generation::first());
        request.model_sha256 = decode_sha256(
            whisper_core::model_registry::model(
                whisper_core::model_catalog::ModelId::ParakeetTdt06BV3,
            )
            .artifact
            .sha256,
        )
        .unwrap();
        request.config.compute = ComputeChoice::Gpu;
        let key = crate::archive::ArchiveQualificationKey::parakeet_gpu_test();
        let mut candidate = fake_gpu_candidate();
        candidate.generation = request.generation.get();

        assert!(validate_queued_gpu_preflight(&request, Some(&candidate), Some(&key)).is_ok());
        assert_eq!(
            validate_queued_gpu_preflight(&request, None, Some(&key)).unwrap_err(),
            "BackendMismatch: selected GPU has no current runtime identity proof"
        );
        let mut stale = candidate.clone();
        stale.generation += 1;
        assert_eq!(
            validate_queued_gpu_preflight(&request, Some(&stale), Some(&key)).unwrap_err(),
            "BackendMismatch: selected GPU identity belongs to a stale generation"
        );
        assert_eq!(
            validate_queued_gpu_preflight(&request, Some(&candidate), None).unwrap_err(),
            "ModelNotQualified: GPU Test key is not admitted"
        );
    }

    #[test]
    fn queued_manual_gpu_without_current_proof_refuses_before_pending_creation() {
        for stale_proof in [false, true] {
            let job_id = JobId(if stale_proof { 913 } else { 912 });
            let generation = Generation::first();
            let root = test_artifact_root(if stale_proof {
                "r10-queued-gpu-stale-proof"
            } else {
                "r10-queued-gpu-missing-proof"
            });
            let destination = root.join("archive");
            let mut request = test_request(job_id, generation);
            request.model_sha256 = decode_sha256(
                whisper_core::model_registry::model(
                    whisper_core::model_catalog::ModelId::ParakeetTdt06BV3,
                )
                .artifact
                .sha256,
            )
            .unwrap();
            request.source_sha256 = Some([9; 32]);
            request.source_samples = Some(16_000);
            request.destination = destination.to_string_lossy().into_owned();
            request.config.compute = ComputeChoice::Gpu;
            crate::queue_store::QueueStore::open(&destination)
                .unwrap()
                .enqueue(request.clone())
                .unwrap();

            let mut runtime = ImportRuntime::new(
                WorkerPaths {
                    cpu: root.join("unused-cpu.exe"),
                    gpu: root.join("unused-gpu.exe"),
                },
                Arc::new(PublicationGate::default()),
            );
            if stale_proof {
                let mut stale = fake_gpu_candidate();
                stale.generation = generation.get() + 1;
                runtime
                    .gpu_candidates
                    .insert((job_id, generation.get()), Some(stale));
            }

            let mut events = VecDeque::new();
            runtime.effect(ImportEffect::ProcessQueued(request.clone()), &mut events);
            let expected_message = if stale_proof {
                "BackendMismatch: selected GPU identity belongs to a stale generation"
            } else {
                "BackendMismatch: selected GPU has no current runtime identity proof"
            };
            assert!(events.iter().any(|event| matches!(event,
                ImportEvent::Failed { job_id: failed_job, generation: failed_generation, message }
                    if *failed_job == job_id
                        && *failed_generation == generation
                        && message == expected_message
            )));

            let pending = destination
                .join("transcriptions")
                .join(format!("{:032x}", job_id.0))
                .join(generation.get().to_string())
                .join("pending.json");
            assert!(!pending.exists(), "refusal must precede pending creation");
            let queue = crate::queue_store::QueueStore::open(&destination).unwrap();
            assert_eq!(
                queue.entries()[0].request.config.compute,
                ComputeChoice::Gpu
            );
            assert_eq!(
                queue.entries()[0].status,
                crate::queue_store::QueueStatus::Queued
            );
            std::fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn queued_unified_gpu_refusal_preserves_durable_entry_and_source_before_preparation() {
        for stale_proof in [false, true] {
            let root = test_artifact_root(if stale_proof {
                "queued-unified-stale-gpu-proof"
            } else {
                "queued-unified-missing-gpu-proof"
            })
            .join(format!(
                "{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir_all(&root).unwrap();
            let source_path = root.join("source.wav");
            let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_path).unwrap();
            writer.append(&vec![0_i16; 16_000]).unwrap();
            writer.finish().unwrap();
            let source_sha = hex_sha256(&source_path);
            let destination = root.join("archive");
            let mut request = unified_import_request(
                &destination,
                &source_path,
                if stale_proof { 0x44_201 } else { 0x44_200 },
            );
            request.config.compute = ComputeChoice::Gpu;
            let identity = (request.job_id, request.generation);

            let (observer, native_events) = NativeQualificationObserver::channel();
            let native_observer = Arc::clone(&observer);
            let mut runtime = ImportRuntime::new(
                WorkerPaths {
                    cpu: root.join("unused-cpu.exe"),
                    gpu: root.join("unused-gpu.exe"),
                },
                Arc::new(PublicationGate::default()),
            )
            .with_native_harness_observer(observer);
            if stale_proof {
                let mut stale = fake_gpu_candidate();
                stale.generation = request.generation.get() + 1;
                runtime
                    .gpu_candidates
                    .insert((request.job_id, request.generation.get()), Some(stale));
            }
            let mut app = ImportApplication::new(RuntimeImportPort {
                runtime,
                events: VecDeque::new(),
                capture_progress: None,
                document_revision: None,
                unified_stop_submissions: None,
                controlled_capture_blocks: None,
            });
            app.dispatch(AppCommand::EnqueueImport {
                request: request.clone(),
            })
            .unwrap();
            let deadline = Instant::now() + Duration::from_secs(10);
            let queued = loop {
                let view = app.dispatch(AppCommand::Refresh).unwrap();
                if let Some(entry) = view.queue.iter().find(|entry| {
                    (entry.request.job_id, entry.request.generation) == identity
                        && matches!(entry.status, whisper_core::ports::ImportQueueStatus::Queued)
                }) {
                    break entry.request.clone();
                }
                assert!(Instant::now() < deadline, "Enqueue did not become durable");
                thread::sleep(Duration::from_millis(2));
            };
            assert_eq!(queued.source_sha256, Some(source_sha));

            app.dispatch(AppCommand::ProcessQueued { request: queued })
                .unwrap();
            let deadline = Instant::now() + Duration::from_secs(10);
            let expected = if stale_proof {
                "BackendMismatch: selected GPU identity belongs to a stale generation"
            } else {
                "BackendMismatch: selected GPU has no current runtime identity proof"
            };
            let mut refusal = None;
            while Instant::now() < deadline {
                let view = app.dispatch(AppCommand::Refresh).unwrap();
                if view.unified_parakeet_sessions.iter().any(|session| {
                    (session.job_id, session.generation) == identity
                        && session.phase == whisper_core::ports::UnifiedSessionPhase::Recoverable
                }) {
                    refusal = view.message;
                    break;
                }
                thread::sleep(Duration::from_millis(2));
            }
            assert!(
                refusal
                    .as_deref()
                    .is_some_and(|message| message.contains(expected)),
                "queued GPU refusal should be visible before preparation: {refusal:?}"
            );
            let mut observed_native_events = Vec::new();
            native_event_drain(&native_events, &mut observed_native_events);
            assert!(
                !native_observer.overflowed.load(Ordering::Relaxed),
                "native refusal observer must remain within its fixed capacity"
            );
            assert!(
                !observed_native_events.iter().any(|event| matches!(
                    &event.kind,
                    NativeQualificationEventKind::PreparedRoute { session, .. }
                        if session.job_id == identity.0 && session.generation == identity.1
                )),
                "stale or absent GPU proof must be rejected before route preparation"
            );
            assert!(
                !destination
                    .join("transcriptions")
                    .join(format!("{:032x}", request.job_id.0))
                    .join(request.generation.get().to_string())
                    .join("pending.json")
                    .exists()
            );
            let queue = crate::queue_store::QueueStore::open(&destination).unwrap();
            assert_eq!(queue.entries().len(), 1);
            assert_eq!(
                queue.entries()[0].status,
                crate::queue_store::QueueStatus::Queued
            );
            assert_eq!(hex_sha256(&source_path), source_sha);
            drop(app);
            std::fs::remove_dir_all(root).unwrap();
        }
    }

    #[test]
    fn queued_unified_non_candidate_refusal_does_not_prepare_or_start() {
        let root = test_artifact_root("queued-unified-non-candidate").join(format!(
            "{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source_path = root.join("source.wav");
        let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_path).unwrap();
        writer.append(&vec![0_i16; 16_000]).unwrap();
        writer.finish().unwrap();
        let source_sha = hex_sha256(&source_path);
        let destination = root.join("archive");
        let request = unified_import_request(&destination, &source_path, 0x44_300);
        let identity = (request.job_id, request.generation);
        let (observer, native_events) = NativeQualificationObserver::channel();
        let native_observer = Arc::clone(&observer);
        let app_runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: root.join("unused-cpu.exe"),
                gpu: root.join("unused-gpu.exe"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_native_harness_observer(observer);
        assert!(!app_runtime.native_unified_candidate_enabled);
        let mut app = ImportApplication::new(RuntimeImportPort {
            runtime: app_runtime,
            events: VecDeque::new(),
            capture_progress: None,
            document_revision: None,
            unified_stop_submissions: None,
            controlled_capture_blocks: None,
        });
        app.dispatch(AppCommand::EnqueueImport {
            request: request.clone(),
        })
        .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        let queued = loop {
            let view = app.dispatch(AppCommand::Refresh).unwrap();
            if let Some(entry) = view.queue.iter().find(|entry| {
                (entry.request.job_id, entry.request.generation) == identity
                    && matches!(entry.status, whisper_core::ports::ImportQueueStatus::Queued)
            }) {
                break entry.request.clone();
            }
            assert!(Instant::now() < deadline, "Enqueue did not become durable");
            thread::sleep(Duration::from_millis(2));
        };
        app.dispatch(AppCommand::ProcessQueued { request: queued })
            .unwrap();

        let deadline = Instant::now() + Duration::from_secs(10);
        let mut refused = None;
        while Instant::now() < deadline {
            let view = app.dispatch(AppCommand::Refresh).unwrap();
            if let Some(session) = view.unified_parakeet_sessions.iter().find(|session| {
                (session.job_id, session.generation) == identity
                    && session.phase == whisper_core::ports::UnifiedSessionPhase::Recoverable
            }) {
                refused = session.message.clone().or(view.message);
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        assert!(
            refused.as_deref().is_some_and(|message| {
                message.contains("ModelNotQualified") && message.contains("controlled worker")
            }),
            "non-candidate refusal should remain visible: {refused:?}"
        );
        let mut observed_native_events = Vec::new();
        native_event_drain(&native_events, &mut observed_native_events);
        assert!(!native_observer.overflowed.load(Ordering::Relaxed));
        assert!(
            !observed_native_events.iter().any(|event| matches!(
                &event.kind,
                NativeQualificationEventKind::PreparedRoute { session, .. }
                    if session.job_id == identity.0 && session.generation == identity.1
            )),
            "non-candidate admission must not prepare a native route"
        );
        assert!(
            !observed_native_events.iter().any(|event| matches!(
                &event.kind,
                NativeQualificationEventKind::Starting { .. }
                    | NativeQualificationEventKind::Spawned { .. }
                    | NativeQualificationEventKind::Ready { .. }
                    | NativeQualificationEventKind::RequestSent { .. }
            )),
            "non-candidate refusal must not start a native worker"
        );
        assert!(
            !destination
                .join("transcriptions")
                .join(format!("{:032x}", request.job_id.0))
                .join(request.generation.get().to_string())
                .join("pending.json")
                .exists()
        );
        let queue = crate::queue_store::QueueStore::open(&destination).unwrap();
        assert_eq!(queue.entries().len(), 1);
        assert_eq!(
            queue.entries()[0].status,
            crate::queue_store::QueueStatus::Queued
        );
        assert_eq!(hex_sha256(&source_path), source_sha);
        drop(app);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn queued_unified_capacity_refusal_keeps_fifo_entry_without_preparing_route() {
        let root = test_artifact_root("queued-unified-capacity").join(format!(
            "{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source_path = root.join("source.wav");
        let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_path).unwrap();
        writer.append(&vec![0_i16; 16_000]).unwrap();
        writer.finish().unwrap();
        let source_sha = hex_sha256(&source_path);
        let destination = root.join("archive");
        let request = unified_import_request(&destination, &source_path, 0x44_400);
        let identity = (request.job_id, request.generation);
        let route = controlled_native_route(&root);
        let (observer, native_events) = NativeQualificationObserver::channel();
        let native_observer = Arc::clone(&observer);
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: root.join("unused-cpu.exe"),
                gpu: root.join("unused-gpu.exe"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_native_harness_observer(observer);
        runtime.native_unified_candidate_enabled = true;
        for occupied in [0x44_401, 0x44_402] {
            runtime
                .register_prepared_native_route(
                    whisper_core::parakeet_scheduler::SchedulerSession {
                        job_id: JobId(occupied),
                        generation: Generation::first(),
                    },
                    route.clone(),
                )
                .unwrap();
        }
        let mut app = ImportApplication::new(RuntimeImportPort {
            runtime,
            events: VecDeque::new(),
            capture_progress: None,
            document_revision: None,
            unified_stop_submissions: None,
            controlled_capture_blocks: None,
        });
        app.dispatch(AppCommand::EnqueueImport {
            request: request.clone(),
        })
        .unwrap();
        let enqueue_deadline = Instant::now() + Duration::from_secs(10);
        let queued = loop {
            let view = app.dispatch(AppCommand::Refresh).unwrap();
            if let Some(entry) = view.queue.iter().find(|entry| {
                (entry.request.job_id, entry.request.generation) == identity
                    && matches!(entry.status, whisper_core::ports::ImportQueueStatus::Queued)
            }) {
                break entry.request.clone();
            }
            assert!(
                Instant::now() < enqueue_deadline,
                "Enqueue did not become durable"
            );
            thread::sleep(Duration::from_millis(2));
        };
        app.dispatch(AppCommand::ProcessQueued { request: queued })
            .unwrap();

        let deadline = Instant::now() + Duration::from_secs(10);
        let mut message = None;
        while Instant::now() < deadline {
            let view = app.dispatch(AppCommand::Refresh).unwrap();
            if let Some(session) = view.unified_parakeet_sessions.iter().find(|session| {
                (session.job_id, session.generation) == identity
                    && session.phase == whisper_core::ports::UnifiedSessionPhase::Recoverable
            }) {
                message = session.message.clone().or(view.message);
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        assert!(
            message
                .as_deref()
                .is_some_and(|message| message.contains("SchedulerCapacity")),
            "capacity refusal should remain visible: {message:?}"
        );
        let mut observed_native_events = Vec::new();
        native_event_drain(&native_events, &mut observed_native_events);
        assert!(!native_observer.overflowed.load(Ordering::Relaxed));
        let prepared_sessions = observed_native_events
            .iter()
            .filter_map(|event| match &event.kind {
                NativeQualificationEventKind::PreparedRoute { session, .. } => Some(*session),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(
            prepared_sessions.len(),
            2,
            "both pre-admitted routes remain present"
        );
        assert!(prepared_sessions.iter().any(|session| {
            session.job_id == JobId(0x44_401) && session.generation == Generation::first()
        }));
        assert!(prepared_sessions.iter().any(|session| {
            session.job_id == JobId(0x44_402) && session.generation == Generation::first()
        }));
        assert!(
            !prepared_sessions.iter().any(|session| {
                session.job_id == identity.0 && session.generation == identity.1
            }),
            "third refused import must not prepare a route"
        );
        let pending = destination
            .join("transcriptions")
            .join(format!("{:032x}", request.job_id.0))
            .join(request.generation.get().to_string())
            .join("pending.json");
        assert!(!pending.exists());
        assert_eq!(
            crate::queue_store::QueueStore::open(&destination)
                .unwrap()
                .entries()[0]
                .status,
            crate::queue_store::QueueStatus::Queued
        );
        assert_eq!(hex_sha256(&source_path), source_sha);
        drop(app);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(all(feature = "l3-gpu-qualification", feature = "recette-hooks"))]
    #[test]
    fn l3c_acceptance_fault_is_confined_to_its_canonical_execution_root() {
        assert!(option_env!("PARAKEET_L2_ACCEPTANCE_TEST_BUILD").is_some());
        let root = canonical_extended_acceptance_root(
            std::env::var_os("PARAKEET_L3C_EXEC_ROOT").map(PathBuf::from),
            std::env::var_os("PARAKEET_L3D_EXEC_ROOT").map(PathBuf::from),
        )
        .expect("exactly one canonical L3c or L3d execution root");
        let destination = root.join("r8/acceptance-test");
        std::fs::create_dir_all(destination.join(".l2a2-acceptance")).unwrap();
        std::fs::write(
            destination.join(".l2a2-acceptance/worker-death.enabled"),
            b"enabled",
        )
        .unwrap();
        assert!(
            take_acceptance_worker_fault(&destination, true)
                .unwrap()
                .is_some()
        );

        let outside = root
            .parent()
            .expect("L3c root parent exists")
            .join("l3a")
            .canonicalize()
            .expect("existing L3a root is a canonical path outside L3c and L3b");
        assert!(
            take_acceptance_worker_fault(&outside, true)
                .unwrap_err()
                .starts_with("PathOutsideRuntime")
        );

        let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .canonicalize()
            .unwrap();
        for lot in ["l3c", "l3d"] {
            let canonical = repository
                .join(format!("target/parakeet-tests/l3/{lot}"))
                .canonicalize()
                .unwrap();
            assert_eq!(
                canonical_extended_acceptance_root(
                    (lot == "l3c").then(|| canonical.clone()),
                    (lot == "l3d").then(|| canonical.clone()),
                ),
                Some(canonical),
                "the test-only acceptance boundary keeps canonical {lot}"
            );
        }
    }

    #[test]
    fn over_limit_parakeet_import_is_refused_before_pending_creation() {
        if not_run_without_campaign_variables(&[
            "PARAKEET_L2_MODEL",
            "PARAKEET_L2_RUNTIME_ROOT",
            "PARAKEET_L2_EN_WAV",
            "PARAKEET_L2_WORKER",
            "PARAKEET_L2_NATIVE_DIR",
        ]) {
            return;
        }
        let model = PathBuf::from(std::env::var_os("PARAKEET_L2_MODEL").expect("R6 model path"));
        let runtime =
            PathBuf::from(std::env::var_os("PARAKEET_L2_RUNTIME_ROOT").expect("R6 runtime root"));
        let source =
            PathBuf::from(std::env::var_os("PARAKEET_L2_EN_WAV").expect("R6 short source"));
        let worker = PathBuf::from(std::env::var_os("PARAKEET_L2_WORKER").expect("R6 worker path"));
        let native_dir =
            PathBuf::from(std::env::var_os("PARAKEET_L2_NATIVE_DIR").expect("R6 native directory"));
        let destination = test_artifact_root("r6-over-limit-import");
        let mut request = test_request(JobId(0x6a31), Generation::first());
        request.source_path = source.to_string_lossy().into_owned();
        request.source_samples =
            Some(whisper_core::model_registry::PARAKEET_MAX_SINGLE_PASS_DURATION_MS * 16 + 1);
        request.model_path = model.to_string_lossy().into_owned();
        request.model_sha256 = decode_sha256(
            whisper_core::model_registry::model(
                whisper_core::model_catalog::ModelId::ParakeetTdt06BV3,
            )
            .artifact
            .sha256,
        )
        .unwrap();
        request.destination = destination.to_string_lossy().into_owned();
        request.config = JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        };
        let catalog = RuntimeModelCatalog {
            catalog: crate::model_catalog::LocalModelCatalog::new(
                model.parent().expect("model directory").to_path_buf(),
            ),
            known_historical_turbo_path: runtime
                .join("historical/ggml-large-v3-turbo.bin")
                .to_string_lossy()
                .into_owned(),
            parakeet_paths: Some(crate::parakeet_import::ParakeetWorkerPaths {
                executable: worker,
                native_dir,
                model,
                runtime_root: runtime,
            }),
            gpu_parakeet_paths: None,
        };

        let error = catalog.bind_import(&request).unwrap_err();
        assert!(
            error.contains("Fichier trop long pour Parakeet, utilisez Turbo"),
            "{error}"
        );
        assert!(
            !destination.exists(),
            "refusal must happen before archive/pending creation"
        );
    }

    #[test]
    fn injected_catalog_guards_all_worker_effect_routes_including_checkpoint_resume() {
        if not_run_without_campaign_variables(&["WHISPER_L1B_RUNTIME_ROOT"]) {
            return;
        }
        use std::{
            fs,
            path::PathBuf,
            time::{SystemTime, UNIX_EPOCH},
        };
        let root =
            PathBuf::from(std::env::var_os("WHISPER_L1B_RUNTIME_ROOT").expect("bounded runtime"))
                .join(format!(
                    "runtime-guards-{}-{}",
                    std::process::id(),
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                ));
        fs::create_dir_all(&root).unwrap();
        let gate = Arc::new(PublicationGate::default());
        let catalog = RuntimeModelCatalog {
            catalog: crate::model_catalog::LocalModelCatalog::new(root.join("empty-model-catalog")),
            known_historical_turbo_path: r"C:\known\ggml-large-v3-turbo.bin".into(),
            parakeet_paths: None,
            gpu_parakeet_paths: None,
        };
        let mut runtime = ImportRuntime::new_with_catalog(
            WorkerPaths {
                cpu: root.join("cpu.exe"),
                gpu: root.join("gpu.exe"),
            },
            gate,
            Some(catalog),
        );
        let mut events = VecDeque::new();
        let mut request = test_request(JobId(901), Generation::first());
        request.source_sha256 = Some([7; 32]);
        request.source_samples = Some(16_000);
        request.model_path = r"C:\unknown\model.bin".into();
        let destination = root.join("archive");
        request.destination = destination.to_string_lossy().into_owned();

        runtime.effect(ImportEffect::Prepare(request.clone()), &mut events);
        runtime.effect(ImportEffect::StartWorker(request.clone()), &mut events);
        runtime.effect(ImportEffect::Enqueue(request.clone()), &mut events);

        let mut queue = crate::queue_store::QueueStore::open(&destination).unwrap();
        queue.enqueue(request.clone()).unwrap();
        runtime.effect(ImportEffect::ProcessQueued(request.clone()), &mut events);

        let next_generation = request.generation.next().unwrap();
        queue
            .set_status(
                request.job_id,
                request.generation,
                crate::queue_store::QueueStatus::Interrupted,
            )
            .unwrap();
        let mut resumed = request.clone();
        resumed.generation = next_generation;
        runtime.effect(
            ImportEffect::ResumeInterrupted {
                previous: Box::new(request.clone()),
                request: resumed,
            },
            &mut events,
        );

        let live = whisper_core::ports::LiveRequest {
            job_id: JobId(902),
            generation: Generation::first(),
            model_path: request.model_path.clone(),
            model_sha256: request.model_sha256,
            destination: destination.to_string_lossy().into_owned(),
            group_offset_samples: 0,
            auto_stop_after_speech_samples: None,
            microphone: whisper_core::microphone::MicrophoneChoice::WindowsDefault,
            microphone_gain_db: 0,
            config: request.config.clone(),
        };
        runtime.effect(ImportEffect::StartLive(live.clone()), &mut events);
        runtime.effect(ImportEffect::StartDirectLive(live), &mut events);

        assert!(runtime.active.is_none());
        assert!(runtime.live.is_none());
        assert!(runtime.direct_live.is_none());
        assert_eq!(events.len(), 7);
        assert!(events.iter().all(|event| matches!(event, ImportEvent::Failed { message, .. } if message.contains("ModelUnknown"))));
    }

    #[test]
    fn resume_keeps_legacy_checkpoint_identity_and_returns_catalog_path_for_execution() {
        if not_run_without_campaign_variables(&["WHISPER_L1B_RUNTIME_ROOT"]) {
            return;
        }
        use std::{
            fs,
            path::PathBuf,
            time::{SystemTime, UNIX_EPOCH},
        };
        let root =
            PathBuf::from(std::env::var_os("WHISPER_L1B_RUNTIME_ROOT").expect("bounded runtime"))
                .join(format!(
                    "runtime-resume-{}-{}",
                    std::process::id(),
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap()
                        .as_nanos()
                ));
        fs::create_dir_all(root.join("legacy")).unwrap();
        let repository_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let model = PathBuf::from(std::env::var_os("WHISPER_L1B_RUNTIME_ROOT").expect("bounded runtime"))
            .join("models/ggml-large-v3-turbo.bin");
        let legacy_model = root.join("legacy/ggml-large-v3-turbo.bin");
        fs::hard_link(&model, &legacy_model).unwrap();
        let source = repository_root.join("sound example/spkr0.wav");
        assert!(
            source.is_file(),
            "the accepted English fixture must be present"
        );
        let destination = root.join("archive");
        let mut request = test_request(JobId(903), Generation::first());
        request.source_path = source.to_string_lossy().into_owned();
        request.model_path = legacy_model.to_string_lossy().into_owned();
        request.model_sha256 = hex_sha256(&model);
        request.destination = destination.to_string_lossy().into_owned();
        crate::archive::ArchiveStore::identify_source(&mut request).unwrap();
        let mut archive = ArchiveStore::new(&request.destination);
        archive.prepare(request.clone()).unwrap();
        archive
            .persist_segment(&whisper_core::ports::WorkerSegment {
                job_id: request.job_id,
                generation: request.generation,
                instance_id: 1,
                segment_id: whisper_core::SegmentId(1),
                range: whisper_core::SourceRange::new(0, 16_000, 16_000).unwrap(),
                text: "confirmed prefix".into(),
            })
            .unwrap();
        let mut queue = crate::queue_store::QueueStore::open(&destination).unwrap();
        queue.enqueue(request.clone()).unwrap();
        queue
            .set_status(
                request.job_id,
                request.generation,
                crate::queue_store::QueueStatus::Interrupted,
            )
            .unwrap();
        let mut resumed = request.clone();
        resumed.generation = request.generation.next().unwrap();

        let mut runtime = ImportRuntime::new_with_catalog(
            WorkerPaths {
                cpu: root.join("cpu.exe"),
                gpu: root.join("gpu.exe"),
            },
            Arc::new(PublicationGate::default()),
            Some(RuntimeModelCatalog {
                catalog: crate::model_catalog::LocalModelCatalog::new(
                    model.parent().unwrap().to_path_buf(),
                ),
                known_historical_turbo_path: legacy_model.to_string_lossy().into_owned(),
                parakeet_paths: None,
                gpu_parakeet_paths: None,
            }),
        );
        let mut events = VecDeque::new();
        runtime.effect(
            ImportEffect::ResumeInterrupted {
                previous: Box::new(request.clone()),
                request: resumed.clone(),
            },
            &mut events,
        );
        let checkpoint = events.iter().find_map(|event| match event {
            ImportEvent::ResumeCheckpoint {
                confirmed_segments,
                confirmed_offset,
                ..
            } => Some((*confirmed_segments, *confirmed_offset)),
            _ => None,
        });
        assert_eq!(checkpoint, Some((1, 16_000)));
        let prepared = events
            .iter()
            .find_map(|event| match event {
                ImportEvent::Prepared(request) => Some(request),
                _ => None,
            })
            .expect("verified resume emits a worker request");
        assert_eq!(
            PathBuf::from(&prepared.model_path).canonicalize().unwrap(),
            model.canonicalize().unwrap()
        );
        assert_eq!(prepared.generation, resumed.generation);
        let queue = crate::queue_store::QueueStore::open(&destination).unwrap();
        assert_eq!(queue.entries()[0].request.model_path, request.model_path);
        let pending_path = destination
            .join("transcriptions")
            .join(format!("{:032x}", request.job_id.0))
            .join(resumed.generation.get().to_string())
            .join("pending.json");
        let pending: serde_json::Value =
            serde_json::from_slice(&fs::read(pending_path).unwrap()).unwrap();
        assert_eq!(pending["model_path"], request.model_path);
    }

    fn hex_sha256(path: &std::path::Path) -> [u8; 32] {
        let mut file = std::fs::File::open(path).unwrap();
        let mut hasher = sha2::Sha256::new();
        let mut buffer = [0; 1024 * 1024];
        loop {
            let count = std::io::Read::read(&mut file, &mut buffer).unwrap();
            if count == 0 {
                break;
            }
            hasher.update(&buffer[..count]);
        }
        let digest = hasher.finalize();
        digest.into()
    }

    #[test]
    fn stop_during_blocked_commit_returns_promptly_and_reports_finalizing() {
        use std::{
            fs::{self, OpenOptions},
            io::Write,
            sync::{Barrier, mpsc},
            time::{SystemTime, UNIX_EPOCH},
        };

        let gate = Arc::new(PublicationGate::default());
        let (io, effects, stops, event_tx, _, _, _) = AsyncImportIo::for_test(gate.clone());
        let mut app = ImportApplication::new(io);
        let job_id = JobId(88);
        let generation = Generation::first();
        app.dispatch(AppCommand::StartImport {
            request: ImportRequest {
                strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
                job_id,
                generation,
                source_path: "fixture.wav".into(),
                source_sha256: None,
                source_samples: None,
                model_path: "model.bin".into(),
                model_sha256: [0; 32],
                destination: "archive".into(),
                config: JobConfig {
                    language: LanguageChoice::Manual("fr".into()),
                    compute: ComputeChoice::Cpu,
                },
            },
        })
        .unwrap();
        assert!(matches!(effects.try_recv(), Ok(ImportEffect::Prepare(_))));
        let commit_directory = std::env::temp_dir().join(format!(
            "whisper-stop-commit-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&commit_directory).unwrap();
        let staged_pointer = commit_directory.join("CURRENT.tmp");
        let current_pointer = commit_directory.join("CURRENT");

        let commit_entered = Arc::new(Barrier::new(2));
        let allow_disk_commit = Arc::new(Barrier::new(2));
        let publisher = {
            let gate = gate.clone();
            let commit_entered = commit_entered.clone();
            let allow_disk_commit = allow_disk_commit.clone();
            let staged_pointer = staged_pointer.clone();
            let current_pointer = current_pointer.clone();
            thread::spawn(move || {
                gate.commit(|| {
                    commit_entered.wait();
                    allow_disk_commit.wait();
                    let mut file = OpenOptions::new()
                        .write(true)
                        .create_new(true)
                        .open(&staged_pointer)
                        .map_err(|error| error.to_string())?;
                    file.write_all(b"manifest.json\n")
                        .map_err(|error| error.to_string())?;
                    file.sync_all().map_err(|error| error.to_string())?;
                    drop(file);
                    fs::rename(staged_pointer, current_pointer)
                        .map_err(|error| error.to_string())?;
                    Ok(())
                })
            })
        };
        commit_entered.wait();

        let (result_tx, result_rx) = mpsc::channel();
        let (finish_ui_tx, finish_ui_rx) = mpsc::channel();
        let (complete_tx, complete_rx) = mpsc::channel();
        let ui = thread::spawn(move || {
            result_tx
                .send(app.dispatch(AppCommand::Stop { job_id, generation }))
                .unwrap();
            finish_ui_rx.recv().unwrap();
            event_tx
                .send(ImportEvent::Published { job_id, generation })
                .unwrap();
            complete_tx.send(app.dispatch(AppCommand::Refresh)).unwrap();
        });
        let immediate_result = result_rx.recv_timeout(Duration::from_millis(200));
        allow_disk_commit.wait();
        publisher.join().unwrap().unwrap();
        assert!(
            immediate_result.is_ok(),
            "Stop submission must not wait for disk commit"
        );
        let view = immediate_result.unwrap().unwrap();
        assert_eq!(view.active_job, Some((job_id, JobState::Finalizing)));
        assert!(
            view.message
                .as_deref()
                .is_some_and(|message| message.contains("trop tardif"))
        );
        assert!(matches!(stops.try_recv(), Err(TryRecvError::Empty)));
        assert_eq!(fs::read(&current_pointer).unwrap(), b"manifest.json\n");
        finish_ui_tx.send(()).unwrap();
        let complete = complete_rx
            .recv_timeout(Duration::from_secs(1))
            .unwrap()
            .unwrap();
        assert_eq!(complete.active_job, Some((job_id, JobState::Complete)));
        assert_eq!(gate.0.load(Ordering::Acquire), GATE_COMMITTED);
        ui.join().unwrap();
        fs::remove_file(current_pointer).unwrap();
        fs::remove_dir(commit_directory).unwrap();
    }
