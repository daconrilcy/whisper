    #[test]
    fn unified_capture_runtime_rejects_third_before_capture_factory() {
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-admission-before-open-{}",
            std::process::id()
        ));
        let route = controlled_native_route(&root);
        let request_a = unified_live_request(&root, 9_280);
        let request_b = unified_live_request(&root, 9_281);
        let request_third = unified_live_request(&root, 9_282);
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(controlled_worker().0);
        runtime.native_unified_candidate_enabled = true;

        let mut capture_a = CaptureStream::empty_test_capture(16_000, 1, "test:admission-a");
        capture_a.enqueue_test_block(vec![0.1; 160]);
        runtime
            .start_unified_capture(&request_a, capture_a, None, Some(route.clone()))
            .unwrap();

        let session_a = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: request_a.job_id,
            generation: request_a.generation,
        };
        let mut events_a = VecDeque::new();
        runtime
            .stop_controlled_unified_capture(request_a.job_id, request_a.generation, &mut events_a)
            .unwrap();
        let stopped_a = runtime
            .unified_sessions
            .get(&session_a)
            .expect("A remains admitted while its closed source finalizes");
        assert!(stopped_a.source_closed);
        assert!(stopped_a.available_samples > 0);
        assert_eq!(stopped_a.available_samples, stopped_a.durable_samples);
        assert_eq!(stopped_a.finalizer.processed_samples(), 0);
        assert_eq!(runtime.parakeet_scheduler.active_sessions(), 1);
        assert!(runtime.unified_archives.contains_key(&session_a));
        assert!(events_a.iter().any(|event| matches!(
            event,
            ImportEvent::Prepared(request)
                if (request.job_id, request.generation)
                    == (request_a.job_id, request_a.generation)
        )));

        let b_factory_calls = AtomicU64::new(0);
        runtime
            .start_native_unified_capture_with(&request_b, route.clone(), |_, _| {
                b_factory_calls.fetch_add(1, Ordering::Relaxed);
                let mut capture = CaptureStream::empty_test_capture(16_000, 1, "test:admission-b");
                capture.enqueue_test_block(vec![0.1; 160]);
                Ok(capture)
            })
            .unwrap();
        assert_eq!(b_factory_calls.load(Ordering::Relaxed), 1);
        assert_eq!(runtime.unified_sessions.len(), 2);
        assert!(runtime.unified_sessions.contains_key(&session_a));
        assert!(
            runtime
                .unified_sessions
                .values()
                .any(|session| session.source.is_capture() && !session.source_closed)
        );

        let third_factory_calls = AtomicU64::new(0);
        let result = runtime.start_native_unified_capture_with(&request_third, route, |_, _| {
            third_factory_calls.fetch_add(1, Ordering::Relaxed);
            Ok(CaptureStream::empty_test_capture(
                16_000,
                1,
                "test:must-not-open",
            ))
        });
        assert!(
            result
                .as_ref()
                .is_err_and(|error| error.contains("SchedulerCapacity")),
            "third capture should be refused by the runtime: {result:?}"
        );
        assert_eq!(third_factory_calls.load(Ordering::Relaxed), 0);
        assert_eq!(runtime.unified_sessions.len(), 2);
        assert_eq!(runtime.unified_archives.len(), 2);

        let mut events_b = VecDeque::new();
        runtime
            .stop_controlled_unified_capture(request_b.job_id, request_b.generation, &mut events_b)
            .unwrap();
        drop(runtime);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn native_unified_capture_open_failure_leaves_no_session_or_capacity() {
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-open-failure-{}",
            std::process::id()
        ));
        let request = unified_live_request(&root, 9_283);
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(controlled_worker().0);
        runtime.native_unified_candidate_enabled = true;
        let factory_calls = AtomicU64::new(0);

        let result = runtime.start_native_unified_capture_with(
            &request,
            controlled_native_route(&root),
            |microphone, gain| {
                factory_calls.fetch_add(1, Ordering::Relaxed);
                assert_eq!(microphone, &request.microphone);
                assert_eq!(gain.value(), request.microphone_gain_db);
                Err("controlled source unavailable".into())
            },
        );

        assert_eq!(result.unwrap_err(), "controlled source unavailable");
        assert_eq!(factory_calls.load(Ordering::Relaxed), 1);
        assert!(runtime.unified_sessions.is_empty());
        assert!(runtime.unified_archives.is_empty());
        assert_eq!(runtime.parakeet_scheduler.active_sessions(), 0);
        assert!(!runtime.unified_admission_full());
        drop(runtime);
        let _ = std::fs::remove_dir_all(&root);
    }

    fn assert_real_unified_model(request: &ImportRequest) {
        assert!(
            Path::new(&request.model_path).is_file(),
            "required real Parakeet test model is missing: {}",
            request.model_path
        );
    }

    fn assert_app_view_internal_publication(
        session: &whisper_core::ports::UnifiedParakeetSessionView,
        archive_root: &Path,
    ) {
        #[derive(serde::Deserialize)]
        struct Artifact {
            path: String,
            sha256: String,
            bytes: u64,
        }

        #[derive(serde::Deserialize)]
        struct Manifest {
            version: u32,
            job_id: JobId,
            generation: Generation,
            source_sha256: String,
            segments: u64,
            text: Artifact,
            srt: Artifact,
        }

        assert_eq!(
            session.phase,
            whisper_core::ports::UnifiedSessionPhase::Complete
        );
        let generation_directory = archive_root
            .join("transcriptions")
            .join(format!("{:032x}", session.job_id.0))
            .join(session.generation.get().to_string());
        assert_eq!(
            std::fs::read(generation_directory.join("CURRENT")).unwrap(),
            b"manifest.json\n"
        );
        let manifest: Manifest = serde_json::from_slice(
            &std::fs::read(generation_directory.join("manifest.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(manifest.version, 1);
        assert_eq!(manifest.job_id, session.job_id);
        assert_eq!(manifest.generation, session.generation);
        assert!(manifest.segments > 0);
        let canonical_generation = generation_directory.canonicalize().unwrap();
        let expected_artifact = |artifact: &Artifact, expected_name: &str| {
            assert_eq!(artifact.path, expected_name);
            let path = generation_directory.join(&artifact.path);
            let canonical = path.canonicalize().unwrap();
            assert!(canonical.starts_with(&canonical_generation));
            let metadata = std::fs::metadata(&canonical).unwrap();
            assert_eq!(metadata.len(), artifact.bytes);
            assert_eq!(
                crate::parakeet_import::sha256_file(&canonical).unwrap(),
                artifact.sha256
            );
            canonical
        };
        let expected_txt = expected_artifact(&manifest.text, "transcript.txt");
        let expected_srt = expected_artifact(&manifest.srt, "transcript.srt");
        let expected_wav = generation_directory
            .join("parakeet-unified/source.wav")
            .canonicalize()
            .unwrap();
        assert!(expected_wav.starts_with(&canonical_generation));
        let wav_path = session.wav_path.as_deref().expect("AppView WAV reference");
        let txt_path = session.txt_path.as_deref().expect("AppView TXT reference");
        let srt_path = session.srt_path.as_deref().expect("AppView SRT reference");
        assert_eq!(Path::new(wav_path), expected_wav);
        assert_eq!(Path::new(txt_path), expected_txt);
        assert_eq!(Path::new(srt_path), expected_srt);
        let audio = crate::parakeet_audio::PcmPartStore::open(Path::new(wav_path));
        assert_eq!(
            audio.total_samples().unwrap(),
            session.total_samples.unwrap()
        );
        let mut offset = 0_u64;
        let mut covered = 0_u64;
        let mut expected_number = 1_u64;
        loop {
            let page = audio.read_page(offset, 128).unwrap();
            if page.is_empty() {
                break;
            }
            for part in &page {
                assert_eq!(part.number, expected_number);
                assert_eq!(part.start_sample, covered);
                assert!(part.end_sample > part.start_sample);
                let canonical_part = part.path.canonicalize().unwrap();
                assert!(canonical_part.starts_with(&canonical_generation));
                if expected_number == 1 {
                    assert_eq!(canonical_part, expected_wav);
                }
                covered = part.end_sample;
                expected_number += 1;
            }
            offset += page.len() as u64;
        }
        assert_eq!(covered, session.total_samples.unwrap());
        assert_eq!(session.captured_samples, covered);
        assert_eq!(session.durable_samples, covered);
        let entry = ArchiveStore::scan(archive_root)
            .unwrap()
            .into_iter()
            .find(|entry| entry.job_id == session.job_id && entry.generation == session.generation)
            .expect("AppView publication must remain in archive history");
        assert!(entry.complete);
        assert_eq!(entry.source_sha256, manifest.source_sha256);
        assert_eq!(entry.export_basename, None);
    }

    fn assert_app_view_external_publication(
        session: &whisper_core::ports::UnifiedParakeetSessionView,
        archive_root: &Path,
        expected_basename: &str,
        expected_source_sha256: &str,
        expected_samples: u64,
    ) {
        #[derive(serde::Deserialize)]
        struct Artifact {
            path: String,
            sha256: String,
            bytes: u64,
        }

        #[derive(serde::Deserialize)]
        struct Manifest {
            version: u32,
            job_id: JobId,
            generation: Generation,
            source_sha256: String,
            segments: u64,
            text: Artifact,
            srt: Artifact,
        }

        #[derive(serde::Deserialize)]
        struct PublicationIntent {
            version: u32,
            choice: DuplicateImportChoice,
            source_sha256: String,
            source_samples: u64,
            export_basename: String,
            internal_only: bool,
        }

        let assert_no_reparse = |path: &Path| {
            let relative = path.strip_prefix(archive_root).unwrap_or_else(|_| {
                panic!(
                    "publication path escaped archive root {}: {}",
                    archive_root.display(),
                    path.display()
                )
            });
            let mut current = archive_root.to_path_buf();
            for component in relative.components() {
                current.push(component.as_os_str());
                let metadata = std::fs::symlink_metadata(&current).unwrap_or_else(|error| {
                    panic!("stat publication path {}: {error}", current.display())
                });
                assert!(
                    !metadata.file_type().is_symlink(),
                    "publication path must not contain a reparse link: {}",
                    current.display()
                );
            }
        };

        assert_eq!(
            session.phase,
            whisper_core::ports::UnifiedSessionPhase::Complete
        );
        let canonical_root = archive_root.canonicalize().unwrap();
        assert_eq!(
            Path::new(&session.destination).canonicalize().unwrap(),
            canonical_root
        );
        let generation_directory = archive_root
            .join("transcriptions")
            .join(format!("{:032x}", session.job_id.0))
            .join(session.generation.get().to_string());
        let canonical_generation = generation_directory.canonicalize().unwrap();
        assert!(canonical_generation.starts_with(&canonical_root));
        assert_no_reparse(&generation_directory);

        let current_path = generation_directory.join("CURRENT");
        assert_no_reparse(&current_path);
        assert_eq!(std::fs::read(&current_path).unwrap(), b"manifest.json\n");
        let manifest_path = generation_directory.join("manifest.json");
        assert_no_reparse(&manifest_path);
        let manifest: Manifest = serde_json::from_slice(&std::fs::read(&manifest_path).unwrap())
            .expect("parse the completed external publication manifest");
        assert_eq!(manifest.version, 1);
        assert_eq!(manifest.job_id, session.job_id);
        assert_eq!(manifest.generation, session.generation);
        assert!(manifest.segments > 0);
        assert_eq!(manifest.source_sha256, expected_source_sha256);

        let intent_path = generation_directory.join("r11-publication.json");
        assert_no_reparse(&intent_path);
        let intent: PublicationIntent =
            serde_json::from_slice(&std::fs::read(&intent_path).unwrap())
                .expect("parse the external publication intent");
        assert_eq!(intent.version, 1);
        assert_eq!(intent.choice, DuplicateImportChoice::CreateNew);
        assert!(!intent.internal_only);
        assert_eq!(intent.export_basename, expected_basename);
        assert_eq!(intent.source_sha256, manifest.source_sha256);
        assert_eq!(intent.source_samples, expected_samples);
        assert_eq!(session.total_samples, Some(expected_samples));

        let internal_artifact = |artifact: &Artifact, expected_name: &str| {
            assert_eq!(artifact.path, expected_name);
            let path = generation_directory.join(&artifact.path);
            assert_no_reparse(&path);
            let canonical = path.canonicalize().unwrap();
            assert!(canonical.starts_with(&canonical_generation));
            assert!(std::fs::metadata(&canonical).unwrap().is_file());
            assert_eq!(std::fs::metadata(&canonical).unwrap().len(), artifact.bytes);
            assert_eq!(
                crate::parakeet_import::sha256_file(&canonical).unwrap(),
                artifact.sha256
            );
        };
        internal_artifact(&manifest.text, "transcript.txt");
        internal_artifact(&manifest.srt, "transcript.srt");

        let exports_directory = archive_root.join("exports");
        assert_no_reparse(&exports_directory);
        let canonical_exports = exports_directory.canonicalize().unwrap();
        assert!(canonical_exports.starts_with(&canonical_root));
        let external_artifact = |path: &Path, artifact: &Artifact| {
            assert_no_reparse(path);
            let canonical = path.canonicalize().unwrap();
            assert_eq!(canonical.parent(), Some(canonical_exports.as_path()));
            assert!(std::fs::metadata(&canonical).unwrap().is_file());
            assert_eq!(std::fs::metadata(&canonical).unwrap().len(), artifact.bytes);
            assert_eq!(
                crate::parakeet_import::sha256_file(&canonical).unwrap(),
                artifact.sha256
            );
            canonical
        };
        let expected_txt = external_artifact(
            &exports_directory.join(format!("{expected_basename}.txt")),
            &manifest.text,
        );
        let expected_srt = external_artifact(
            &exports_directory.join(format!("{expected_basename}.srt")),
            &manifest.srt,
        );
        let wav_path = generation_directory.join("parakeet-unified/source.wav");
        assert_no_reparse(&wav_path);
        let canonical_wav = wav_path.canonicalize().unwrap();
        assert!(canonical_wav.starts_with(&canonical_generation));
        let wav_index_path = wav_path.with_extension("parts.idx");
        assert_no_reparse(&wav_index_path);
        let canonical_index = wav_index_path.canonicalize().unwrap();
        assert!(canonical_index.starts_with(&canonical_generation));
        let index_sha256 = crate::parakeet_import::sha256_file(&canonical_index).unwrap();

        let app_wav = session.wav_path.as_deref().expect("AppView WAV reference");
        let app_txt = session
            .txt_path
            .as_deref()
            .expect("AppView external TXT reference");
        let app_srt = session
            .srt_path
            .as_deref()
            .expect("AppView external SRT reference");
        assert_eq!(Path::new(app_wav).canonicalize().unwrap(), canonical_wav);
        assert_eq!(Path::new(app_txt).canonicalize().unwrap(), expected_txt);
        assert_eq!(Path::new(app_srt).canonicalize().unwrap(), expected_srt);

        let audio = crate::parakeet_audio::PcmPartStore::open(&canonical_wav);
        let total_samples = audio.total_samples().unwrap();
        assert_eq!(Some(total_samples), session.total_samples);
        assert_eq!(total_samples, expected_samples);
        let mut offset = 0_u64;
        let mut covered = 0_u64;
        let mut expected_number = 1_u64;
        loop {
            let page = audio.read_page(offset, 128).unwrap();
            if page.is_empty() {
                break;
            }
            for part in &page {
                assert_eq!(part.number, expected_number);
                assert_eq!(part.start_sample, covered);
                assert!(part.end_sample > part.start_sample);
                let canonical_part = part.path.canonicalize().unwrap();
                assert!(canonical_part.starts_with(&canonical_generation));
                assert_no_reparse(&part.path);
                if expected_number == 1 {
                    assert_eq!(canonical_part, canonical_wav);
                }
                covered = part.end_sample;
                expected_number += 1;
            }
            offset += page.len() as u64;
        }
        assert_eq!(covered, total_samples);
        assert_eq!(session.captured_samples, covered);
        assert_eq!(session.durable_samples, covered);
        assert!(std::fs::metadata(canonical_index).unwrap().is_file());
        assert_eq!(
            crate::parakeet_import::sha256_file(&wav_index_path.canonicalize().unwrap()).unwrap(),
            index_sha256,
            "WAV parts index must remain unchanged while the external AppView is read"
        );

        let entry = ArchiveStore::scan(archive_root)
            .unwrap()
            .into_iter()
            .find(|entry| entry.job_id == session.job_id && entry.generation == session.generation)
            .expect("external AppView publication must remain in archive history");
        assert!(entry.complete);
        assert_eq!(entry.source_sha256, manifest.source_sha256);
        assert_eq!(entry.source_sha256, expected_source_sha256);
        assert_eq!(entry.export_basename.as_deref(), Some(expected_basename));
    }

    fn enqueue_import_with_choice(
        app: &mut ImportApplication<RuntimeImportPort>,
        request: ImportRequest,
        choice: Option<DuplicateImportChoice>,
    ) -> ImportRequest {
        let requested_job = request.job_id;
        app.dispatch(AppCommand::EnqueueImport { request })
            .expect("enqueue import through the application");
        let deadline = Instant::now() + Duration::from_secs(60);
        let mut choice = choice;
        loop {
            let view = app
                .dispatch(AppCommand::Refresh)
                .expect("refresh import admission and queue");
            if let Some(prompt) = view.duplicate_import_prompt.clone() {
                assert_eq!(prompt.request.job_id, requested_job);
                app.dispatch(AppCommand::ResolveDuplicateImport {
                    prompt,
                    choice: choice
                        .take()
                        .expect("duplicate import requires its explicit test choice"),
                })
                .expect("resolve duplicate import choice through the application");
                continue;
            }
            if let Some(entry) = view.queue.iter().find(|entry| {
                entry.request.job_id == requested_job
                    && matches!(entry.status, whisper_core::ports::ImportQueueStatus::Queued)
            }) {
                return entry.request.clone();
            }
            assert!(
                Instant::now() < deadline,
                "import did not enter the durable queue: view={view:?}"
            );
            thread::sleep(Duration::from_millis(2));
        }
    }

    fn process_queued_import_to_app_view(
        app: &mut ImportApplication<RuntimeImportPort>,
        request: ImportRequest,
        archive_root: &Path,
    ) -> whisper_core::ports::UnifiedParakeetSessionView {
        let identity = (request.job_id, request.generation);
        app.dispatch(AppCommand::ProcessQueued { request })
            .expect("process the durable import through the application");
        let deadline = Instant::now() + Duration::from_secs(60);
        let mut last_message = None;
        loop {
            let view = app
                .dispatch(AppCommand::Refresh)
                .expect("refresh processed import");
            last_message = view.message.clone();
            if let Some(session) = view
                .unified_parakeet_sessions
                .iter()
                .find(|session| (session.job_id, session.generation) == identity)
            {
                if session.phase == whisper_core::ports::UnifiedSessionPhase::Recoverable {
                    panic!("queued import failed: {session:?}; {last_message:?}");
                }
                if session.phase == whisper_core::ports::UnifiedSessionPhase::Complete {
                    assert_app_view_internal_publication(session, archive_root);
                    return session.clone();
                }
            }
            assert!(
                Instant::now() < deadline,
                "queued import did not complete: {last_message:?}; view={:?}",
                app.view()
            );
            thread::sleep(Duration::from_millis(2));
        }
    }

    #[test]
    fn import_application_uses_real_archive_path_for_controlled_short_and_long_unified_imports() {
        for (index, (seconds, compute)) in [
            (3_u64, ComputeChoice::Cpu),
            (3_u64, ComputeChoice::Gpu),
            (121_u64, ComputeChoice::Cpu),
            (121_u64, ComputeChoice::Gpu),
        ]
        .into_iter()
        .enumerate()
        {
            let root = std::env::temp_dir().join(format!(
                "parakeet-unified-application-{}-{}",
                std::process::id(),
                index
            ));
            std::fs::create_dir_all(&root).unwrap();
            let source_path = root.join("source.wav");
            let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_path).unwrap();
            writer
                .append(&vec![0_i16; (seconds * 16_000) as usize])
                .unwrap();
            writer.finish().unwrap();
            let mut request =
                unified_import_request(&root.join("archive"), &source_path, 700 + index as u128);
            request.config.compute = compute;
            assert_real_unified_model(&request);
            let (worker, calls) = controlled_worker();
            let mut runtime = ImportRuntime::new(
                WorkerPaths {
                    cpu: PathBuf::from("unused-cpu"),
                    gpu: PathBuf::from("unused-gpu"),
                },
                Arc::new(PublicationGate::default()),
            )
            .with_controlled_transcriber(worker);
            if compute == ComputeChoice::Gpu {
                let mut candidate = fake_gpu_candidate();
                candidate.generation = request.generation.get();
                runtime
                    .gpu_candidates
                    .insert((request.job_id, request.generation.get()), Some(candidate));
            }
            let port = RuntimeImportPort {
                runtime,
                events: VecDeque::new(),
                capture_progress: None,
                document_revision: None,
                unified_stop_submissions: None,
                controlled_capture_blocks: None,
            };
            let mut app = ImportApplication::new(port);
            let expected_job_id = request.job_id;
            let expected_generation = request.generation;
            app.dispatch(AppCommand::StartImport { request }).unwrap();
            let terminal_deadline = Instant::now() + Duration::from_secs(20);
            let mut application_complete = false;
            let mut last_phase = "identity not visible".to_owned();
            let mut last_message = None;
            while Instant::now() < terminal_deadline {
                let view = app.dispatch(AppCommand::Refresh).unwrap();
                last_message = view.message.clone();
                let session = view.unified_parakeet_sessions.iter().find(|session| {
                    session.job_id == expected_job_id && session.generation == expected_generation
                });
                if let Some(session) = session {
                    last_phase = format!("{:?}", session.phase);
                    if session.phase == whisper_core::ports::UnifiedSessionPhase::Recoverable {
                        panic!(
                            "controlled import {expected_job_id:?}/{expected_generation:?} entered Recoverable ({seconds}s, {compute:?}): phase={}, message={:?}",
                            last_phase, last_message
                        );
                    }
                    if session.phase == whisper_core::ports::UnifiedSessionPhase::Complete {
                        application_complete = true;
                        break;
                    }
                } else {
                    last_phase = "identity not visible".into();
                }
                thread::sleep(Duration::from_millis(2));
            }
            if !application_complete {
                let archive_status = match ArchiveStore::scan(&root.join("archive")) {
                    Ok(entries) => {
                        let complete = entries.iter().any(|entry| {
                            entry.job_id == expected_job_id
                                && entry.generation == expected_generation
                                && entry.complete
                        });
                        format!("scan complete={complete}; entries={}", entries.len())
                    }
                    Err(error) => format!("scan error={error}"),
                };
                let current = root
                    .join(format!(
                        "archive/transcriptions/{:032x}/{}/CURRENT",
                        expected_job_id.0,
                        expected_generation.get()
                    ))
                    .is_file();
                panic!(
                    "controlled import {expected_job_id:?}/{expected_generation:?} did not reach application Complete within the bounded wait ({seconds}s, {compute:?}); phase={last_phase}, message={last_message:?}, archive={archive_status}, CURRENT={current}"
                );
            }
            assert!(application_complete);
            let entries = ArchiveStore::scan(&root.join("archive")).unwrap();
            let entry = entries
                .iter()
                .find(|entry| {
                    entry.job_id == expected_job_id
                        && entry.generation == expected_generation
                        && entry.complete
                        && entry.strategy
                            == whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
                })
                .unwrap_or_else(|| {
                    panic!(
                        "application reported Complete before the matching unified archive was committed: {expected_job_id:?}/{expected_generation:?}"
                    )
                });
            assert_eq!(
                ArchiveStore::published_imports_for_strategy(
                    &root.join("archive"),
                    &entry.source_sha256,
                    whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1,
                )
                .unwrap()
                .len(),
                1
            );
            assert!(
                ArchiveStore::published_imports_for_strategy(
                    &root.join("archive"),
                    &entry.source_sha256,
                    whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
                )
                .unwrap()
                .is_empty()
            );
            let archive = root.join(format!(
                "archive/transcriptions/{:032x}/1",
                700 + index as u128
            ));
            assert!(archive.join("transcript.txt").is_file());
            assert!(archive.join("transcript.srt").is_file());
            assert!(
                std::fs::read_to_string(archive.join("transcript.txt"))
                    .unwrap()
                    .contains("controlled.")
            );
            let coverage =
                std::fs::read_to_string(archive.join("parakeet-unified/document/coverage.jsonl"))
                    .unwrap();
            assert_eq!(coverage.lines().count(), if seconds > 120 { 2 } else { 1 });
            assert_eq!(
                calls.load(Ordering::Relaxed),
                coverage.lines().count() as u64,
                "each final window must be invoked through the scheduler"
            );
            let basename = entry.export_basename.as_ref().unwrap();
            assert!(
                root.join("archive/exports")
                    .join(format!("{basename}.txt"))
                    .is_file()
            );
            assert!(
                root.join("archive/exports")
                    .join(format!("{basename}.srt"))
                    .is_file()
            );
            std::fs::remove_dir_all(&root).unwrap();
        }
    }

    #[test]
    fn import_application_processes_durable_queued_unified_imports_for_cpu_gpu_auto_and_long_audio()
    {
        let event_trace = Arc::new(Mutex::new(Vec::new()));
        IMPORT_PORT_EVENT_TRACE.with(|trace| {
            *trace.borrow_mut() = Some(Arc::clone(&event_trace));
        });
        for (index, (seconds, compute)) in [
            (3_u64, ComputeChoice::Cpu),
            (3_u64, ComputeChoice::Gpu),
            (3_u64, ComputeChoice::Auto),
            (121_u64, ComputeChoice::Cpu),
        ]
        .into_iter()
        .enumerate()
        {
            let root = test_artifact_root("queued-unified-import").join(format!(
                "{}-{index}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir_all(&root).unwrap();
            let source_path = root.join("source.wav");
            let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_path).unwrap();
            writer
                .append(&vec![0_i16; (seconds * 16_000) as usize])
                .unwrap();
            writer.finish().unwrap();
            let source_sha = hex_sha256(&source_path);
            let mut request = unified_import_request(
                &root.join("archive"),
                &source_path,
                0x44_000 + index as u128,
            );
            request.config.compute = compute;
            assert_real_unified_model(&request);

            let (worker, calls) = controlled_worker();
            let mut runtime = ImportRuntime::new(
                WorkerPaths {
                    cpu: PathBuf::from("unused-cpu"),
                    gpu: PathBuf::from("unused-gpu"),
                },
                Arc::new(PublicationGate::default()),
            )
            .with_controlled_transcriber(worker);
            if compute == ComputeChoice::Gpu {
                let mut candidate = fake_gpu_candidate();
                candidate.generation = request.generation.get();
                runtime
                    .gpu_candidates
                    .insert((request.job_id, request.generation.get()), Some(candidate));
            }
            let mut app = ImportApplication::new(RuntimeImportPort {
                runtime,
                events: VecDeque::new(),
                capture_progress: None,
                document_revision: None,
                unified_stop_submissions: None,
                controlled_capture_blocks: None,
            });
            let identity = (request.job_id, request.generation);
            app.dispatch(AppCommand::EnqueueImport {
                request: request.clone(),
            })
            .unwrap();
            let queued_view = app.dispatch(AppCommand::Refresh).unwrap();
            let queued = queued_view
                .queue
                .iter()
                .find(|entry| {
                    (entry.request.job_id, entry.request.generation) == identity
                        && matches!(entry.status, whisper_core::ports::ImportQueueStatus::Queued)
                })
                .expect("the application must expose the durable FIFO request after Enqueue")
                .request
                .clone();
            assert_eq!(queued.source_sha256, Some(source_sha));
            assert_eq!(queued.strategy, request.strategy);

            app.dispatch(AppCommand::ProcessQueued { request: queued })
                .unwrap();
            let deadline = Instant::now() + Duration::from_secs(60);
            let mut phase = "session not visible".to_owned();
            let mut message = None;
            let mut complete = false;
            while Instant::now() < deadline {
                let view = match app.dispatch(AppCommand::Refresh) {
                    Ok(view) => view,
                    Err(error) => {
                        let trace = event_trace.lock().expect("import event trace lock");
                        IMPORT_PORT_EVENT_TRACE.with(|trace| *trace.borrow_mut() = None);
                        panic!(
                            "durable queued import Refresh failed ({seconds}s, {compute:?}, identity={identity:?}): {error:?}; session={:?}; events={trace:?}",
                            app.view().unified_parakeet_sessions.iter().find(|session| (
                                session.job_id,
                                session.generation
                            )
                                == identity)
                        );
                    }
                };
                message = view.message.clone();
                if let Some(session) = view
                    .unified_parakeet_sessions
                    .iter()
                    .find(|session| (session.job_id, session.generation) == identity)
                {
                    phase = format!("{:?}", session.phase);
                    if session.phase == whisper_core::ports::UnifiedSessionPhase::Recoverable {
                        panic!(
                            "durable queued import failed ({seconds}s, {compute:?}): {phase}; message={message:?}"
                        );
                    }
                    if session.phase == whisper_core::ports::UnifiedSessionPhase::Complete {
                        complete = true;
                        break;
                    }
                }
                thread::sleep(Duration::from_millis(2));
            }
            assert!(
                complete,
                "durable queued import did not reach Complete ({seconds}s, {compute:?}): phase={phase}, message={message:?}"
            );
            assert!(calls.load(Ordering::Relaxed) > 0);
            let archive_root = root.join("archive");
            let entry = ArchiveStore::scan(&archive_root)
                .unwrap()
                .into_iter()
                .find(|entry| (entry.job_id, entry.generation) == identity && entry.complete)
                .expect("the queued import must publish a complete archive");
            assert_eq!(entry.strategy, request.strategy);
            assert!(
                archive_root
                    .join("exports")
                    .join(format!("{}.txt", entry.export_basename.as_deref().unwrap()))
                    .is_file()
            );
            assert!(
                archive_root
                    .join("exports")
                    .join(format!("{}.srt", entry.export_basename.as_deref().unwrap()))
                    .is_file()
            );
            assert_eq!(hex_sha256(&source_path), source_sha);
            drop(app);
            std::fs::remove_dir_all(&root).unwrap();
        }
        IMPORT_PORT_EVENT_TRACE.with(|trace| *trace.borrow_mut() = None);
    }

    #[test]
    fn queued_create_only_publication_paths_reach_app_view() {
        let root = test_artifact_root("queued-create-only-app-view").join(format!(
            "{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let archive_root = root.join("archive");
        let settings_root = root.join("settings");
        std::fs::create_dir_all(&archive_root).unwrap();
        let source_path = root.join("source.wav");
        let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_path).unwrap();
        writer.append(&vec![0_i16; 3 * 16_000]).unwrap();
        writer.finish().unwrap();
        let mut request = unified_import_request(&archive_root, &source_path, 0x51_101);
        request.config.compute = ComputeChoice::Cpu;
        assert_real_unified_model(&request);
        let (worker, calls) = controlled_worker();
        let runtime = ImportRuntime::new_with_workers_and_parakeet_registry(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
            None,
            None,
            None,
            Some(settings_root),
            true,
            false,
        )
        .with_controlled_transcriber(worker);
        let mut app = ImportApplication::new(RuntimeImportPort {
            runtime,
            events: VecDeque::new(),
            capture_progress: None,
            document_revision: None,
            unified_stop_submissions: None,
            controlled_capture_blocks: None,
        });
        let identity = (request.job_id, request.generation);
        app.dispatch(AppCommand::EnqueueImport {
            request: request.clone(),
        })
        .expect("application must durably enqueue the create-only import");
        let queued = app
            .dispatch(AppCommand::Refresh)
            .unwrap()
            .queue
            .into_iter()
            .find(|entry| {
                (entry.request.job_id, entry.request.generation) == identity
                    && matches!(entry.status, whisper_core::ports::ImportQueueStatus::Queued)
            })
            .expect("AppView must expose the exact queued identity")
            .request;
        assert_eq!(queued.strategy, request.strategy);
        app.dispatch(AppCommand::ProcessQueued { request: queued })
            .expect("ProcessQueued must prepare the real create-only route");

        let deadline = Instant::now() + Duration::from_secs(30);
        let mut observed = None;
        let mut last_message = None;
        while Instant::now() < deadline {
            let view = app.dispatch(AppCommand::Refresh).unwrap();
            last_message = view.message.clone();
            if let Some(session) = view
                .unified_parakeet_sessions
                .iter()
                .find(|session| (session.job_id, session.generation) == identity)
            {
                if session.phase == whisper_core::ports::UnifiedSessionPhase::Recoverable {
                    panic!("create-only queued import failed: {session:?}; {last_message:?}");
                }
                if session.phase == whisper_core::ports::UnifiedSessionPhase::Complete {
                    observed = Some(session.clone());
                    break;
                }
            }
            thread::sleep(Duration::from_millis(2));
        }
        let complete = observed.unwrap_or_else(|| {
            panic!("create-only queued import did not complete: {last_message:?}")
        });
        assert!(calls.load(Ordering::Relaxed) > 0);
        assert_app_view_internal_publication(&complete, &archive_root);
        assert!(
            last_message
                .as_deref()
                .is_none_or(|message| !message.contains("PublicationUnavailable")),
            "AppView must not expose the former internal-exports refusal: {last_message:?}"
        );
        assert!(!archive_root.join("exports/transcript.txt").exists());
        assert!(!archive_root.join("exports/transcript.srt").exists());
        drop(app);
        eprintln!(
            "Preserved queued publication test artifacts at {}",
            root.display()
        );
    }

    #[test]
    fn resumed_create_only_publication_paths_reach_app_view() {
        let root = test_artifact_root("resumed-create-only-app-view").join(format!(
            "{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let archive_root = root.join("archive");
        let settings_root = root.join("settings");
        std::fs::create_dir_all(&archive_root).unwrap();
        let source_path = root.join("source.wav");
        let checkpoint_samples = whisper_core::parakeet_windows::WINDOW_MAX_SAMPLES;
        let total_samples = checkpoint_samples + 32_000;
        let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_path).unwrap();
        writer.append(&vec![0_i16; total_samples as usize]).unwrap();
        writer.finish().unwrap();
        let mut previous = unified_import_request(&archive_root, &source_path, 0x51_102);
        previous.config.compute = ComputeChoice::Cpu;
        assert_real_unified_model(&previous);
        ArchiveStore::identify_source(&mut previous).unwrap();
        let source_sha = previous.source_sha256.unwrap();
        ArchiveStore::reserve_create_only_new_import(
            &archive_root,
            &previous,
            &format_sha256(&source_sha),
        )
        .unwrap();

        let mut archive = ArchiveStore::new(&archive_root);
        archive.prepare(previous.clone()).unwrap();
        let prefix = whisper_core::ports::WorkerSegment {
            job_id: previous.job_id,
            generation: previous.generation,
            instance_id: 1,
            segment_id: whisper_core::SegmentId(1),
            range: whisper_core::SourceRange::new(
                checkpoint_samples - 6_400,
                checkpoint_samples - 3_200,
                16_000,
            )
            .unwrap(),
            text: "confirmed create-only prefix".into(),
        };
        let unconfirmed = whisper_core::ports::WorkerSegment {
            job_id: previous.job_id,
            generation: previous.generation,
            instance_id: 1,
            segment_id: whisper_core::SegmentId(2),
            range: whisper_core::SourceRange::new(
                checkpoint_samples,
                checkpoint_samples + 8_000,
                16_000,
            )
            .unwrap(),
            text: "unconfirmed create-only tail".into(),
        };
        archive.persist_segment(&prefix).unwrap();
        archive.persist_segment(&unconfirmed).unwrap();
        let previous_directory = archive_root
            .join("transcriptions")
            .join(format!("{:032x}", previous.job_id.0))
            .join(previous.generation.get().to_string());
        let previous_document_directory =
            previous_directory.join("parakeet-unified").join("document");
        let mut document = crate::parakeet_document::ParakeetDocumentStore::create(
            &previous_document_directory,
            previous.job_id,
            previous.generation,
        )
        .unwrap();
        document.append_final(prefix).unwrap();
        document
            .record_coverage(whisper_core::ports::ParakeetWindowCoverage {
                sequence: 1,
                range: whisper_core::SourceRange::new(0, checkpoint_samples, 16_000).unwrap(),
                last_segment_sequence: 1,
            })
            .unwrap();
        document.append_final(unconfirmed).unwrap();
        drop(document);

        let mut queue = crate::queue_store::QueueStore::open(&archive_root).unwrap();
        queue.enqueue(previous.clone()).unwrap();
        queue
            .set_status(
                previous.job_id,
                previous.generation,
                crate::queue_store::QueueStatus::Interrupted,
            )
            .unwrap();
        drop(crate::source_binding::SourceBinding::acquire(&previous, &settings_root).unwrap());

        let (worker, calls) = controlled_worker();
        let runtime = ImportRuntime::new_with_workers_and_parakeet_registry(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
            None,
            None,
            None,
            Some(settings_root.clone()),
            true,
            false,
        )
        .with_controlled_transcriber(worker);
        let mut app = ImportApplication::new(RuntimeImportPort {
            runtime,
            events: VecDeque::new(),
            capture_progress: None,
            document_revision: None,
            unified_stop_submissions: None,
            controlled_capture_blocks: None,
        });
        let identity = (previous.job_id, previous.generation.next().unwrap());
        app.dispatch(AppCommand::ResumeInterrupted {
            request: previous.clone(),
        })
        .expect("application must resume the durable interrupted request");
        let deadline = Instant::now() + Duration::from_secs(60);
        let mut complete = None;
        let mut last_message = None;
        while Instant::now() < deadline {
            let view = app.dispatch(AppCommand::Refresh).unwrap();
            last_message = view.message.clone();
            if let Some(session) = view
                .unified_parakeet_sessions
                .iter()
                .find(|session| (session.job_id, session.generation) == identity)
            {
                if session.phase == whisper_core::ports::UnifiedSessionPhase::Recoverable {
                    panic!("create-only resume failed: {session:?}; {last_message:?}");
                }
                if session.phase == whisper_core::ports::UnifiedSessionPhase::Complete {
                    complete = Some(session.clone());
                    break;
                }
            }
            thread::sleep(Duration::from_millis(2));
        }
        let completed_session = complete
            .unwrap_or_else(|| panic!("create-only resume did not complete: {last_message:?}"));
        assert!(calls.load(Ordering::Relaxed) > 0);
        assert_app_view_internal_publication(&completed_session, &archive_root);
        let published_text =
            std::fs::read_to_string(completed_session.txt_path.as_deref().unwrap()).unwrap();
        let published_srt =
            std::fs::read_to_string(completed_session.srt_path.as_deref().unwrap()).unwrap();
        assert_eq!(
            published_text
                .matches("confirmed create-only prefix")
                .count(),
            1
        );
        assert_eq!(
            published_srt
                .matches("confirmed create-only prefix")
                .count(),
            1
        );
        assert!(!published_text.contains("unconfirmed create-only tail"));
        assert!(!published_srt.contains("unconfirmed create-only tail"));
        assert!(!archive_root.join("exports/transcript.txt").exists());
        assert!(!archive_root.join("exports/transcript.srt").exists());
        assert_eq!(
            crate::parakeet_import::sha256_file(&source_path).unwrap(),
            format_sha256(&source_sha)
        );
        assert!(
            crate::source_binding::SourceBinding::reopen_for_identity(
                &settings_root,
                &archive_root,
                identity.0,
                identity.1,
            )
            .unwrap()
            .is_none(),
            "completed resumed publication must release the transferred source binding"
        );

        let first_txt_path = PathBuf::from(completed_session.txt_path.as_deref().unwrap());
        let first_srt_path = PathBuf::from(completed_session.srt_path.as_deref().unwrap());
        let first_txt_bytes = std::fs::read(&first_txt_path).unwrap();
        let first_srt_bytes = std::fs::read(&first_srt_path).unwrap();
        let mut second_request = unified_import_request(&archive_root, &source_path, 0x51_104);
        second_request.config.compute = ComputeChoice::Cpu;
        assert_real_unified_model(&second_request);
        let second_queued = enqueue_import_with_choice(
            &mut app,
            second_request,
            Some(DuplicateImportChoice::CreateNew),
        );
        let second = process_queued_import_to_app_view(&mut app, second_queued, &archive_root);
        let second_txt_path = PathBuf::from(second.txt_path.as_deref().unwrap());
        let second_srt_path = PathBuf::from(second.srt_path.as_deref().unwrap());
        assert!(second_txt_path.is_file());
        assert!(second_srt_path.is_file());

        let mut reprocessing_request =
            unified_import_request(&archive_root, &source_path, 0x51_105);
        reprocessing_request.config.compute = ComputeChoice::Cpu;
        assert_real_unified_model(&reprocessing_request);
        let interrupted = enqueue_import_with_choice(
            &mut app,
            reprocessing_request,
            Some(DuplicateImportChoice::ReplaceLatest),
        );
        let checkpoint_samples = whisper_core::parakeet_windows::WINDOW_MAX_SAMPLES;
        let mut interrupted_archive = ArchiveStore::new(&archive_root);
        interrupted_archive.prepare(interrupted.clone()).unwrap();
        let confirmed_prefix = whisper_core::ports::WorkerSegment {
            job_id: interrupted.job_id,
            generation: interrupted.generation,
            instance_id: 7,
            segment_id: whisper_core::SegmentId(1),
            range: whisper_core::SourceRange::new(
                checkpoint_samples - 6_400,
                checkpoint_samples - 3_200,
                16_000,
            )
            .unwrap(),
            text: "confirmed ReplaceLatest prefix".into(),
        };
        interrupted_archive
            .persist_segment(&confirmed_prefix)
            .unwrap();
        let interrupted_directory = archive_root
            .join("transcriptions")
            .join(format!("{:032x}", interrupted.job_id.0))
            .join(interrupted.generation.get().to_string());
        let interrupted_document_directory = interrupted_directory
            .join("parakeet-unified")
            .join("document");
        let mut interrupted_document = crate::parakeet_document::ParakeetDocumentStore::create(
            &interrupted_document_directory,
            interrupted.job_id,
            interrupted.generation,
        )
        .unwrap();
        interrupted_document.append_final(confirmed_prefix).unwrap();
        interrupted_document
            .record_coverage(whisper_core::ports::ParakeetWindowCoverage {
                sequence: 1,
                range: whisper_core::SourceRange::new(0, checkpoint_samples, 16_000).unwrap(),
                last_segment_sequence: 1,
            })
            .unwrap();
        drop(interrupted_document);
        crate::queue_store::QueueStore::open(&archive_root)
            .unwrap()
            .set_status(
                interrupted.job_id,
                interrupted.generation,
                crate::queue_store::QueueStatus::Interrupted,
            )
            .unwrap();

        let reprocessed_identity = (interrupted.job_id, interrupted.generation.next().unwrap());
        app.dispatch(AppCommand::ResumeInterrupted {
            request: interrupted.clone(),
        })
        .expect("resume the durable ReplaceLatest request");
        let deadline = Instant::now() + Duration::from_secs(60);
        let mut reprocessed = None;
        let mut resume_message = None;
        while Instant::now() < deadline {
            let view = app.dispatch(AppCommand::Refresh).unwrap();
            resume_message = view.message.clone();
            if let Some(session) = view
                .unified_parakeet_sessions
                .iter()
                .find(|session| (session.job_id, session.generation) == reprocessed_identity)
            {
                if session.phase == whisper_core::ports::UnifiedSessionPhase::Recoverable {
                    panic!("ReplaceLatest resume failed: {session:?}; {resume_message:?}");
                }
                if session.phase == whisper_core::ports::UnifiedSessionPhase::Complete {
                    assert_app_view_internal_publication(session, &archive_root);
                    reprocessed = Some(session.clone());
                    break;
                }
            }
            thread::sleep(Duration::from_millis(2));
        }
        let reprocessed = reprocessed
            .unwrap_or_else(|| panic!("ReplaceLatest resume did not complete: {resume_message:?}"));
        let reprocessed_txt =
            std::fs::read_to_string(reprocessed.txt_path.as_deref().unwrap()).unwrap();
        let reprocessed_srt =
            std::fs::read_to_string(reprocessed.srt_path.as_deref().unwrap()).unwrap();
        assert_eq!(
            reprocessed_txt
                .matches("confirmed ReplaceLatest prefix")
                .count(),
            1
        );
        assert_eq!(
            reprocessed_srt
                .matches("confirmed ReplaceLatest prefix")
                .count(),
            1
        );
        assert_eq!(std::fs::read(&first_txt_path).unwrap(), first_txt_bytes);
        assert_eq!(std::fs::read(&first_srt_path).unwrap(), first_srt_bytes);
        let first_result = ArchiveStore::published_unified_files(
            &archive_root,
            completed_session.job_id,
            completed_session.generation,
        )
        .expect("the earlier unselected result must remain readable");
        assert_eq!(Path::new(&first_result.txt_path), first_txt_path);
        assert_eq!(Path::new(&first_result.srt_path), first_srt_path);

        let reprocessed_intent_path = archive_root
            .join("transcriptions")
            .join(format!("{:032x}", reprocessed_identity.0.0))
            .join(reprocessed_identity.1.get().to_string())
            .join("r11-publication.json");
        let valid_reprocessed_intent = std::fs::read(&reprocessed_intent_path).unwrap();
        let source_sha256_hex = format_sha256(&source_sha);
        let catalog =
            ArchiveStore::reprocessing_catalog(&archive_root, &source_sha256_hex).unwrap();
        let publication_for = |job_id, generation| {
            catalog
                .iter()
                .find(|entry| entry.job_id == job_id && entry.generation == generation)
                .unwrap_or_else(|| panic!("published treatment missing from catalog: {catalog:?}"))
        };
        let earlier = publication_for(completed_session.job_id, completed_session.generation);
        let selected = publication_for(second.job_id, second.generation);
        let current = publication_for(reprocessed.job_id, reprocessed.generation);
        assert!(earlier.active && earlier.verified_present, "{catalog:?}");
        assert!(
            !selected.active,
            "selected prior result remains active: {catalog:?}"
        );
        assert!(current.active && current.verified_present, "{catalog:?}");
        assert_eq!(
            catalog.iter().filter(|entry| entry.active).count(),
            2,
            "ReplaceLatest must deactivate only its selected prior result: {catalog:?}"
        );

        let intent_value: serde_json::Value =
            serde_json::from_slice(&valid_reprocessed_intent).unwrap();
        let cleanup_id = intent_value["cleanup_id"].as_str().unwrap();
        let cleanup_plan_path = archive_root
            .join(".whisper-publications")
            .join("replacement-cleanups")
            .join(format!("{cleanup_id}.json"));
        let cleanup_plan: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&cleanup_plan_path).unwrap()).unwrap();
        let targets = cleanup_plan["targets"].as_array().unwrap();
        assert_eq!(cleanup_plan["published"], true, "{cleanup_plan}");
        assert_eq!(
            cleanup_plan["successor"]["job_id"],
            serde_json::json!(reprocessed.job_id.0),
            "{cleanup_plan}"
        );
        assert_eq!(
            cleanup_plan["successor"]["generation"],
            serde_json::json!(reprocessed.generation.get()),
            "{cleanup_plan}"
        );
        assert_eq!(targets.len(), 1, "{cleanup_plan}");
        assert_eq!(
            targets[0]["treatment"]["job_id"],
            serde_json::json!(second.job_id.0),
            "cleanup must target only the selected prior publication: {cleanup_plan}"
        );
        assert_eq!(
            targets[0]["treatment"]["generation"],
            serde_json::json!(second.generation.get()),
            "cleanup must target only the selected prior publication: {cleanup_plan}"
        );
        let mut altered_operation =
            serde_json::from_slice::<serde_json::Value>(&valid_reprocessed_intent).unwrap();
        altered_operation["operation_id"] = serde_json::Value::String(format!(
            "bounded-windows-v1-{:032x}-{}",
            reprocessed_identity.0.0 + 1,
            reprocessed_identity.1.get()
        ));
        std::fs::write(
            &reprocessed_intent_path,
            serde_json::to_vec(&altered_operation).unwrap(),
        )
        .unwrap();
        assert!(
            ArchiveStore::published_unified_files(
                &archive_root,
                reprocessed_identity.0,
                reprocessed_identity.1
            )
            .unwrap_err()
            .contains("StaleResponse"),
            "a prefixed operation identity for a foreign job must be rejected"
        );
        let mut altered_cleanup =
            serde_json::from_slice::<serde_json::Value>(&valid_reprocessed_intent).unwrap();
        altered_cleanup["cleanup_id"] = serde_json::Value::String(format!(
            "{:032x}-{}",
            reprocessed_identity.0.0 + 1,
            interrupted.generation.get()
        ));
        std::fs::write(
            &reprocessed_intent_path,
            serde_json::to_vec(&altered_cleanup).unwrap(),
        )
        .unwrap();
        assert!(
            ArchiveStore::published_unified_files(
                &archive_root,
                reprocessed_identity.0,
                reprocessed_identity.1
            )
            .unwrap_err()
            .contains("StaleResponse"),
            "cleanup provenance for a foreign job must be rejected"
        );
        std::fs::write(&reprocessed_intent_path, valid_reprocessed_intent).unwrap();
        assert_app_view_internal_publication(&reprocessed, &archive_root);
        drop(app);
        eprintln!(
            "Preserved resumed publication test artifacts at {}",
            root.display()
        );
    }

    #[test]
    #[cfg(windows)]
    fn reprocessing_create_only_publication_paths_reach_app_view_and_preserve_earlier_result() {
        let root = test_artifact_root("reprocessing-create-only-app-view").join(format!(
            "{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let archive_root = root.join("archive");
        let settings_root = root.join("settings");
        std::fs::create_dir_all(&archive_root).unwrap();
        let source_path = root.join("source.wav");
        let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_path).unwrap();
        writer.append(&vec![0_i16; 3 * 16_000]).unwrap();
        writer.finish().unwrap();

        let (worker, calls) = controlled_worker();
        let runtime = ImportRuntime::new_with_workers_and_parakeet_registry(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
            None,
            None,
            None,
            Some(settings_root),
            true,
            false,
        )
        .with_controlled_transcriber(worker);
        let mut app = ImportApplication::new(RuntimeImportPort {
            runtime,
            events: VecDeque::new(),
            capture_progress: None,
            document_revision: None,
            unified_stop_submissions: None,
            controlled_capture_blocks: None,
        });

        let run_queued = |app: &mut ImportApplication<RuntimeImportPort>,
                          request: ImportRequest,
                          confirmation: Option<DuplicateImportChoice>| {
            let requested_job = request.job_id;
            app.dispatch(AppCommand::EnqueueImport { request })
                .expect("enqueue create-only or reprocessing import");
            let deadline = Instant::now() + Duration::from_secs(60);
            let mut confirmation = confirmation;
            let queued = loop {
                let view = app
                    .dispatch(AppCommand::Refresh)
                    .expect("refresh import admission and queue");
                if let Some(prompt) = view.duplicate_import_prompt.clone() {
                    assert_eq!(prompt.request.job_id, requested_job);
                    let choice = confirmation
                        .take()
                        .expect("only repeated source imports require a choice");
                    app.dispatch(AppCommand::ResolveDuplicateImport { prompt, choice })
                        .expect("confirm duplicate or reprocessing choice");
                    continue;
                }
                if let Some(entry) = view.queue.iter().find(|entry| {
                    entry.request.job_id == requested_job
                        && matches!(entry.status, whisper_core::ports::ImportQueueStatus::Queued)
                }) {
                    break entry.request.clone();
                }
                assert!(
                    Instant::now() < deadline,
                    "import did not enter the durable queue: view={view:?}"
                );
                thread::sleep(Duration::from_millis(2));
            };
            let identity = (queued.job_id, queued.generation);
            app.dispatch(AppCommand::ProcessQueued { request: queued })
                .expect("ProcessQueued must use the real reprocessing route");
            let deadline = Instant::now() + Duration::from_secs(60);
            let mut last_message = None;
            loop {
                let view = app
                    .dispatch(AppCommand::Refresh)
                    .expect("refresh reprocessing session");
                last_message = view.message.clone();
                if let Some(session) = view
                    .unified_parakeet_sessions
                    .iter()
                    .find(|session| (session.job_id, session.generation) == identity)
                {
                    if session.phase == whisper_core::ports::UnifiedSessionPhase::Recoverable {
                        panic!("reprocessing import failed: {session:?}; {last_message:?}");
                    }
                    if session.phase == whisper_core::ports::UnifiedSessionPhase::Complete {
                        assert_app_view_internal_publication(session, &archive_root);
                        return session.clone();
                    }
                }
                assert!(
                    Instant::now() < deadline,
                    "reprocessing import did not complete: {last_message:?}; view={:?}",
                    app.view()
                );
                thread::sleep(Duration::from_millis(2));
            }
        };

        let mut first_request = unified_import_request(&archive_root, &source_path, 0x51_111);
        first_request.config.compute = ComputeChoice::Cpu;
        assert_real_unified_model(&first_request);
        let first = run_queued(&mut app, first_request, None);
        let first_txt = first.txt_path.as_deref().unwrap().to_owned();
        let first_srt = first.srt_path.as_deref().unwrap().to_owned();
        let first_txt_bytes = std::fs::read(&first_txt).unwrap();
        let first_srt_bytes = std::fs::read(&first_srt).unwrap();

        let mut second_request = unified_import_request(&archive_root, &source_path, 0x51_112);
        second_request.config.compute = ComputeChoice::Cpu;
        let second = run_queued(
            &mut app,
            second_request,
            Some(DuplicateImportChoice::CreateNew),
        );
        let second_txt = second.txt_path.as_deref().unwrap().to_owned();
        assert!(Path::new(&first_txt).is_file());
        assert!(Path::new(&second_txt).is_file());

        let mut replacement_request = unified_import_request(&archive_root, &source_path, 0x51_113);
        replacement_request.config.compute = ComputeChoice::Cpu;
        let replacement = run_queued(
            &mut app,
            replacement_request,
            Some(DuplicateImportChoice::ReplaceLatest),
        );
        assert_app_view_internal_publication(&replacement, &archive_root);
        assert_eq!(std::fs::read(&first_txt).unwrap(), first_txt_bytes);
        assert_eq!(std::fs::read(&first_srt).unwrap(), first_srt_bytes);
        let preserved =
            ArchiveStore::published_unified_files(&archive_root, first.job_id, first.generation)
                .expect("the unselected earlier publication must remain readable");
        assert_eq!(preserved.txt_path, first_txt);
        assert_eq!(preserved.srt_path, first_srt);
        assert!(calls.load(Ordering::Relaxed) >= 3);
        drop(app);
        eprintln!(
            "Preserved reprocessing publication test artifacts at {}",
            root.display()
        );
    }

    #[test]
    fn import_application_resumes_a_verified_interrupted_unified_import() {
        let root = test_artifact_root("queued-unified-resume").join(format!(
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
        let checkpoint_samples = whisper_core::parakeet_windows::WINDOW_MAX_SAMPLES;
        let total_samples = checkpoint_samples + 32_000;
        writer.append(&vec![0_i16; total_samples as usize]).unwrap();
        writer.finish().unwrap();
        let source_sha = hex_sha256(&source_path);
        let mut previous = unified_import_request(&root.join("archive"), &source_path, 0x44_100);
        assert_real_unified_model(&previous);
        ArchiveStore::identify_source(&mut previous).unwrap();
        assert_eq!(previous.source_sha256, Some(source_sha));
        let mut archive = ArchiveStore::new(&previous.destination);
        archive.prepare(previous.clone()).unwrap();
        archive
            .persist_segment(&whisper_core::ports::WorkerSegment {
                job_id: previous.job_id,
                generation: previous.generation,
                instance_id: 1,
                segment_id: whisper_core::SegmentId(1),
                range: whisper_core::SourceRange::new(
                    checkpoint_samples - 6_400,
                    checkpoint_samples - 3_200,
                    16_000,
                )
                .unwrap(),
                text: "prefix confirmé".into(),
            })
            .unwrap();
        archive
            .persist_segment(&whisper_core::ports::WorkerSegment {
                job_id: previous.job_id,
                generation: previous.generation,
                instance_id: 1,
                segment_id: whisper_core::SegmentId(2),
                range: whisper_core::SourceRange::new(
                    checkpoint_samples,
                    checkpoint_samples + 8_000,
                    16_000,
                )
                .unwrap(),
                text: "uncovered tail not promoted".into(),
            })
            .unwrap();
        let old_document_directory = root
            .join("archive")
            .join("transcriptions")
            .join(format!("{:032x}", previous.job_id.0))
            .join(previous.generation.get().to_string())
            .join("parakeet-unified/document");
        let mut old_document = crate::parakeet_document::ParakeetDocumentStore::create(
            &old_document_directory,
            previous.job_id,
            previous.generation,
        )
        .unwrap();
        old_document
            .append_final(whisper_core::ports::WorkerSegment {
                job_id: previous.job_id,
                generation: previous.generation,
                instance_id: 1,
                segment_id: whisper_core::SegmentId(1),
                range: whisper_core::SourceRange::new(
                    checkpoint_samples - 6_400,
                    checkpoint_samples - 3_200,
                    16_000,
                )
                .unwrap(),
                text: "prefix confirmé".into(),
            })
            .unwrap();
        old_document
            .record_coverage(whisper_core::ports::ParakeetWindowCoverage {
                sequence: 1,
                range: whisper_core::SourceRange::new(0, checkpoint_samples, 16_000).unwrap(),
                last_segment_sequence: 1,
            })
            .unwrap();
        old_document
            .append_final(whisper_core::ports::WorkerSegment {
                job_id: previous.job_id,
                generation: previous.generation,
                instance_id: 1,
                segment_id: whisper_core::SegmentId(2),
                range: whisper_core::SourceRange::new(
                    checkpoint_samples,
                    checkpoint_samples + 8_000,
                    16_000,
                )
                .unwrap(),
                text: "uncovered tail not promoted".into(),
            })
            .unwrap();
        drop(old_document);
        let previous_directory = root
            .join("archive")
            .join("transcriptions")
            .join(format!("{:032x}", previous.job_id.0))
            .join(previous.generation.get().to_string());
        let old_segments = std::fs::read(previous_directory.join("segments.jsonl")).unwrap();
        let old_document_bytes =
            std::fs::read(old_document_directory.join("document.jsonl")).unwrap();
        let old_coverage_bytes =
            std::fs::read(old_document_directory.join("coverage.jsonl")).unwrap();
        let old_pending_bytes = std::fs::read(previous_directory.join("pending.json")).unwrap();
        let mut queue = crate::queue_store::QueueStore::open(&previous.destination).unwrap();
        queue.enqueue(previous.clone()).unwrap();
        queue
            .set_status(
                previous.job_id,
                previous.generation,
                crate::queue_store::QueueStatus::Interrupted,
            )
            .unwrap();
        let queue_path = Path::new(&previous.destination).join("transcriptions/queue.jsonl");
        let old_queue_bytes = std::fs::read(&queue_path).unwrap();
        let settings_root = root.join("settings");
        drop(crate::source_binding::SourceBinding::acquire(&previous, &settings_root).unwrap());

        let (worker, calls) = controlled_worker();
        let (observer, source_binding_events) = NativeQualificationObserver::channel();
        let runtime = ImportRuntime::new_with_workers_and_parakeet_registry(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
            None,
            None,
            None,
            Some(settings_root.clone()),
            true,
            false,
        )
        .with_controlled_transcriber(worker)
        .with_native_harness_observer(observer);
        let mut app = ImportApplication::new(RuntimeImportPort {
            runtime,
            events: VecDeque::new(),
            capture_progress: None,
            document_revision: None,
            unified_stop_submissions: None,
            controlled_capture_blocks: None,
        });

        let resumed_identity = (previous.job_id, previous.generation.next().unwrap());
        let segment_journal = previous_directory.join("segments.jsonl");
        let mut malformed_segment_journal = old_segments.clone();
        malformed_segment_journal.extend_from_slice(b"{\"sequence\":3");
        std::fs::write(&segment_journal, &malformed_segment_journal).unwrap();
        app.dispatch(AppCommand::ResumeInterrupted {
            request: previous.clone(),
        })
        .unwrap();
        let rejected_view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(
            rejected_view.active_job,
            Some((previous.job_id, JobState::Recoverable)),
            "malformed interrupted journal must be refused after verified binding transfer"
        );
        assert!(
            rejected_view
                .message
                .as_deref()
                .is_some_and(
                    |message| message.contains("StorageCorrupt") && message.contains("truncated")
                ),
            "resume rejection must identify the unterminated journal: {:?}",
            rejected_view.message
        );
        assert_eq!(
            std::fs::read(&segment_journal).unwrap(),
            malformed_segment_journal
        );
        assert_eq!(
            std::fs::read(old_document_directory.join("document.jsonl")).unwrap(),
            old_document_bytes
        );
        assert_eq!(
            std::fs::read(old_document_directory.join("coverage.jsonl")).unwrap(),
            old_coverage_bytes
        );
        assert_eq!(
            std::fs::read(previous_directory.join("pending.json")).unwrap(),
            old_pending_bytes
        );
        assert_eq!(std::fs::read(&queue_path).unwrap(), old_queue_bytes);
        assert_eq!(hex_sha256(&source_path), source_sha);
        let previous_binding = crate::source_binding::SourceBinding::reopen_for_identity(
            &settings_root,
            Path::new(&previous.destination),
            previous.job_id,
            previous.generation,
        )
        .unwrap()
        .expect("malformed resume must restore the interrupted generation N binding");
        drop(previous_binding);
        assert!(
            crate::source_binding::SourceBinding::reopen_for_identity(
                &settings_root,
                Path::new(&previous.destination),
                resumed_identity.0,
                resumed_identity.1,
            )
            .unwrap()
            .is_none(),
            "failed resume must not leave a generation N+1 binding"
        );
        let failed_resume_binding_maps = source_binding_map_events(&source_binding_events);
        assert!(
            failed_resume_binding_maps
                .iter()
                .any(|(operation, bindings)| {
                    *operation == "resume-transfer"
                        && bindings == &vec![(resumed_identity.0, resumed_identity.1.get())]
                })
        );
        assert!(
            failed_resume_binding_maps
                .iter()
                .any(|(operation, bindings)| {
                    *operation == "resume-rollback"
                        && bindings == &vec![(previous.job_id, previous.generation.get())]
                })
        );

        std::fs::write(&segment_journal, &old_segments).unwrap();
        let mut valid_unterminated_segments = old_segments.clone();
        assert_eq!(valid_unterminated_segments.pop(), Some(b'\n'));
        std::fs::write(&segment_journal, &valid_unterminated_segments).unwrap();
        app.dispatch(AppCommand::ResumeInterrupted {
            request: previous.clone(),
        })
        .unwrap();
        assert_eq!(
            std::fs::read(&segment_journal).unwrap(),
            valid_unterminated_segments,
            "a complete final JSON record without LF is readable but never repaired"
        );
        drop(
            crate::source_binding::SourceBinding::reopen_for_identity(
                &settings_root,
                Path::new(&previous.destination),
                resumed_identity.0,
                resumed_identity.1,
            )
            .unwrap()
            .expect("the durable source binding must follow the resumed generation"),
        );
        let deadline = Instant::now() + Duration::from_secs(60);
        let mut phase = "session not visible".to_owned();
        let mut message = None;
        let mut complete = false;
        while Instant::now() < deadline {
            let view = app.dispatch(AppCommand::Refresh).unwrap();
            message = view.message.clone();
            if let Some(session) = view
                .unified_parakeet_sessions
                .iter()
                .find(|session| (session.job_id, session.generation) == resumed_identity)
            {
                phase = format!("{:?}", session.phase);
                if session.phase == whisper_core::ports::UnifiedSessionPhase::Recoverable {
                    panic!("verified queue resume failed: phase={phase}, message={message:?}");
                }
                if session.phase == whisper_core::ports::UnifiedSessionPhase::Complete {
                    complete = true;
                    break;
                }
            }
            thread::sleep(Duration::from_millis(2));
        }
        assert!(
            complete,
            "verified unified resume did not reach Complete: phase={phase}, message={message:?}"
        );
        assert!(calls.load(Ordering::Relaxed) > 0);
        assert_eq!(
            std::fs::read(&segment_journal).unwrap(),
            valid_unterminated_segments,
            "resume must leave a valid unterminated final record byte-for-byte intact"
        );
        let resumed_archive = ArchiveStore::scan(&root.join("archive"))
            .unwrap()
            .into_iter()
            .find(|entry| (entry.job_id, entry.generation) == resumed_identity && entry.complete)
            .expect("the resumed generation must be the completed unified archive");
        assert_eq!(
            resumed_archive.strategy,
            whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
        );
        let resumed_document = crate::archive::ArchiveStore::read_unified_document_page(
            &root.join("archive"),
            resumed_identity.0,
            resumed_identity.1,
            2,
            1,
            128,
        )
        .unwrap();
        assert_eq!(resumed_document.entries.len(), 2);
        assert_eq!(resumed_document.entries[0].segment.text, "prefix confirmé");
        assert_eq!(resumed_document.entries[0].segment.segment_id.0, 1);
        assert_eq!(resumed_document.entries[1].segment.segment_id.0, 2);
        assert_ne!(
            resumed_document.entries[1].segment.text,
            "uncovered tail not promoted"
        );
        assert!(calls.load(Ordering::Relaxed) > 0);
        let export_basename = resumed_archive
            .export_basename
            .as_deref()
            .expect("completed resumed import must expose its export basename");
        assert!(
            root.join("archive/exports")
                .join(format!("{export_basename}.txt"))
                .is_file()
        );
        assert!(
            root.join("archive/exports")
                .join(format!("{export_basename}.srt"))
                .is_file()
        );
        let exported_text = std::fs::read_to_string(
            root.join("archive/exports")
                .join(format!("{export_basename}.txt")),
        )
        .unwrap();
        let exported_srt = std::fs::read_to_string(
            root.join("archive/exports")
                .join(format!("{export_basename}.srt")),
        )
        .unwrap();
        assert_eq!(exported_text.matches("prefix confirmé").count(), 1);
        assert_eq!(exported_srt.matches("prefix confirmé").count(), 1);
        assert!(!exported_text.contains("uncovered tail not promoted"));
        assert!(!exported_srt.contains("uncovered tail not promoted"));
        assert_eq!(
            std::fs::read(old_document_directory.join("document.jsonl")).unwrap(),
            old_document_bytes
        );
        assert_eq!(
            std::fs::read(old_document_directory.join("coverage.jsonl")).unwrap(),
            old_coverage_bytes
        );
        assert_eq!(hex_sha256(&source_path), source_sha);
        assert!(
            crate::source_binding::SourceBinding::reopen_for_identity(
                &settings_root,
                Path::new(&previous.destination),
                resumed_identity.0,
                resumed_identity.1,
            )
            .unwrap()
            .is_none()
        );
        let source_binding_maps = source_binding_map_events(&source_binding_events);
        assert!(source_binding_maps.iter().any(|(operation, bindings)| {
            *operation == "resume-transfer"
                && bindings == &vec![(resumed_identity.0, resumed_identity.1.get())]
        }));
        assert!(
            source_binding_maps
                .iter()
                .any(|(operation, bindings)| *operation == "release" && bindings.is_empty())
        );
        let mut retained = Vec::new();
        for offset in 0..2 {
            let mut capacity_request = previous.clone();
            capacity_request.job_id = JobId(previous.job_id.0 + 10 + offset);
            capacity_request.generation = Generation::first();
            retained.push(
                crate::source_binding::SourceBinding::acquire(&capacity_request, &settings_root)
                    .expect("both slots must be reusable after successful resumed publication"),
            );
        }
        let mut third_request = previous.clone();
        third_request.job_id = JobId(previous.job_id.0 + 12);
        third_request.generation = Generation::first();
        let capacity_error =
            match crate::source_binding::SourceBinding::acquire(&third_request, &settings_root) {
                Err(error) => error,
                Ok(binding) => {
                    binding.retire().unwrap();
                    panic!("a third retained source snapshot must be refused")
                }
            };
        assert!(capacity_error.contains("SourceSnapshotCapacity"));
        for binding in retained {
            binding.retire().unwrap();
        }
        drop(app);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn failed_unified_resume_preserves_interrupted_source_binding_and_queue_identity() {
        let root = test_artifact_root("queued-unified-resume-refused").join(format!(
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
        writer.append(&vec![0_i16; 3 * 16_000]).unwrap();
        writer.finish().unwrap();
        let source_sha = hex_sha256(&source_path);
        let mut previous = unified_import_request(&root.join("archive"), &source_path, 0x44_200);
        assert_real_unified_model(&previous);
        ArchiveStore::identify_source(&mut previous).unwrap();
        let mut archive = ArchiveStore::new(&previous.destination);
        archive.prepare(previous.clone()).unwrap();
        let mut queue = crate::queue_store::QueueStore::open(&previous.destination).unwrap();
        queue.enqueue(previous.clone()).unwrap();
        queue
            .set_status(
                previous.job_id,
                previous.generation,
                crate::queue_store::QueueStatus::Interrupted,
            )
            .unwrap();
        let settings_root = root.join("settings");
        drop(crate::source_binding::SourceBinding::acquire(&previous, &settings_root).unwrap());
        let resumed_generation = previous.generation.next().unwrap();
        let mut conflicting_resumed = previous.clone();
        conflicting_resumed.generation = resumed_generation;
        drop(
            crate::source_binding::SourceBinding::acquire(&conflicting_resumed, &settings_root)
                .unwrap(),
        );

        let (observer, source_binding_events) = NativeQualificationObserver::channel();
        let runtime = ImportRuntime::new_with_workers_and_parakeet_registry(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
            None,
            None,
            None,
            Some(settings_root.clone()),
            true,
            false,
        )
        .with_controlled_transcriber(controlled_worker().0)
        .with_native_harness_observer(observer);
        let mut app = ImportApplication::new(RuntimeImportPort {
            runtime,
            events: VecDeque::new(),
            capture_progress: None,
            document_revision: None,
            unified_stop_submissions: None,
            controlled_capture_blocks: None,
        });

        app.dispatch(AppCommand::ResumeInterrupted {
            request: previous.clone(),
        })
        .unwrap();
        let mut observed_message = None;
        let mut observed_recoverable = false;
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            let view = app.dispatch(AppCommand::Refresh).unwrap();
            observed_message = view.message.clone();
            observed_recoverable = view.unified_parakeet_sessions.iter().any(|session| {
                (session.job_id, session.generation) == (previous.job_id, resumed_generation)
                    && session.phase == whisper_core::ports::UnifiedSessionPhase::Recoverable
            });
            if observed_recoverable {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        assert!(
            observed_recoverable,
            "resume conflict must become terminal and visible: {observed_message:?}"
        );
        let source_binding_maps = source_binding_map_events(&source_binding_events);
        assert!(
            source_binding_maps.iter().any(|(operation, bindings)| {
                *operation == "resume-transfer"
                    && bindings.contains(&(previous.job_id, previous.generation.get()))
                    && bindings.contains(&(previous.job_id, resumed_generation.get()))
            }),
            "conflict must retain both in-memory identities: {source_binding_maps:?}"
        );
        let queue = crate::queue_store::QueueStore::open(&previous.destination).unwrap();
        assert!(queue.entries().iter().any(|entry| {
            (entry.request.job_id, entry.request.generation)
                == (previous.job_id, previous.generation)
                && entry.status == crate::queue_store::QueueStatus::Interrupted
        }));
        let old_binding = crate::source_binding::SourceBinding::reopen_for_identity(
            &settings_root,
            Path::new(&previous.destination),
            previous.job_id,
            previous.generation,
        )
        .unwrap()
        .expect("failed resume must keep the original durable binding");
        assert_eq!(old_binding.source_sha256(), previous.source_sha256.unwrap());
        drop(old_binding);
        let conflicting_binding = crate::source_binding::SourceBinding::reopen_for_identity(
            &settings_root,
            Path::new(&previous.destination),
            previous.job_id,
            resumed_generation,
        )
        .unwrap()
        .expect("an ambiguous resumed binding must not be silently removed");
        assert_eq!(
            conflicting_binding.source_sha256(),
            previous.source_sha256.unwrap()
        );
        drop(conflicting_binding);
        assert_eq!(hex_sha256(&source_path), source_sha);
        assert!(
            observed_message
                .as_deref()
                .is_some_and(|message| message.contains("SourceSnapshotMismatch"))
        );
        drop(app);
        let retained = crate::source_binding::SourceBinding::reopen_for_identity(
            &settings_root,
            Path::new(&previous.destination),
            previous.job_id,
            previous.generation,
        )
        .unwrap()
        .expect("the original binding remains available after the refusal");
        retained.retire().unwrap();
        crate::source_binding::SourceBinding::reopen_for_identity(
            &settings_root,
            Path::new(&previous.destination),
            previous.job_id,
            resumed_generation,
        )
        .unwrap()
        .expect("conflicting resumed binding remains available for explicit recovery")
        .retire()
        .unwrap();
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn failed_unified_resume_after_binding_transfer_restores_registry_and_queue_identity() {
        let root =
            test_artifact_root("queued-unified-resume-after-transfer-failure").join(format!(
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
        writer.append(&vec![0_i16; 3 * 16_000]).unwrap();
        writer.finish().unwrap();
        let source_sha = hex_sha256(&source_path);
        let mut previous = unified_import_request(&root.join("archive"), &source_path, 0x44_300);
        assert_real_unified_model(&previous);
        ArchiveStore::identify_source(&mut previous).unwrap();
        let mut archive = ArchiveStore::new(&previous.destination);
        archive.prepare(previous.clone()).unwrap();
        archive
            .persist_segment(&whisper_core::ports::WorkerSegment {
                job_id: previous.job_id,
                generation: previous.generation,
                instance_id: 1,
                segment_id: whisper_core::SegmentId(1),
                range: whisper_core::SourceRange::new(0, 16_000, 16_000).unwrap(),
                text: "préfixe confirmé avant panne d'archive".into(),
            })
            .unwrap();
        drop(archive);

        let mut queue = crate::queue_store::QueueStore::open(&previous.destination).unwrap();
        queue.enqueue(previous.clone()).unwrap();
        queue
            .set_status(
                previous.job_id,
                previous.generation,
                crate::queue_store::QueueStatus::Interrupted,
            )
            .unwrap();
        let settings_root = root.join("settings");
        drop(crate::source_binding::SourceBinding::acquire(&previous, &settings_root).unwrap());

        // Keep the durable queue identity valid, but make ArchiveStore's later
        // prefix read fail after ResumeInterrupted has transferred the binding.
        let previous_archive_dir = Path::new(&previous.destination)
            .join("transcriptions")
            .join(format!("{:032x}", previous.job_id.0))
            .join(previous.generation.get().to_string());
        std::fs::write(
            previous_archive_dir.join("segments.jsonl"),
            b"{broken checkpoint}\n",
        )
        .unwrap();

        let transfer_gate = Arc::new((Mutex::new(false), std::sync::Condvar::new()));
        let (observer, source_binding_events) = NativeQualificationObserver::channel();
        let runtime = ImportRuntime::new_with_workers_and_parakeet_registry(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
            None,
            None,
            None,
            Some(settings_root.clone()),
            true,
            false,
        )
        .with_controlled_transcriber(controlled_worker().0)
        .with_resume_binding_transfer_gate(Arc::clone(&transfer_gate))
        .with_native_harness_observer(observer);
        let mut app = ImportApplication::new(RuntimeImportPort {
            runtime,
            events: VecDeque::new(),
            capture_progress: None,
            document_revision: None,
            unified_stop_submissions: None,
            controlled_capture_blocks: None,
        });
        let resumed_generation = previous.generation.next().unwrap();
        let (transfer_observed_tx, transfer_observed_rx) = mpsc::sync_channel(1);
        let watcher_gate = Arc::clone(&transfer_gate);
        let watcher_settings_root = settings_root.clone();
        let watcher_destination = previous.destination.clone();
        let watcher_job_id = previous.job_id;
        let watcher_previous_generation = previous.generation;
        let watcher_resumed_generation = resumed_generation;
        let transfer_watcher = thread::spawn(move || {
            let observation: Result<(bool, [u8; 32]), String> = (|| {
                let (transferred, changed) = &*watcher_gate;
                let transferred = transferred
                    .lock()
                    .map_err(|_| "transfer gate poisoned before registry observation".to_owned())?;
                let (transferred, timeout) = changed
                    .wait_timeout_while(transferred, Duration::from_secs(10), |transferred| {
                        !*transferred
                    })
                    .map_err(|_| "transfer gate wait poisoned".to_owned())?;
                if timeout.timed_out() || !*transferred {
                    return Err(
                        "resume did not expose the transferred binding before timeout".into(),
                    );
                }
                drop(transferred);

                let old_binding = crate::source_binding::SourceBinding::reopen_for_identity(
                    &watcher_settings_root,
                    Path::new(&watcher_destination),
                    watcher_job_id,
                    watcher_previous_generation,
                )
                .map_err(|error| format!("read interrupted binding during transfer: {error}"))?;
                let new_binding = crate::source_binding::SourceBinding::reopen_for_identity(
                    &watcher_settings_root,
                    Path::new(&watcher_destination),
                    watcher_job_id,
                    watcher_resumed_generation,
                )
                .map_err(|error| format!("read resumed binding during transfer: {error}"))?
                .ok_or_else(|| "registry did not contain the transferred N+1 binding".to_owned())?;
                let old_absent = old_binding.is_none();
                let new_sha = new_binding.source_sha256();
                drop(old_binding);
                drop(new_binding);

                let (transferred, changed) = &*watcher_gate;
                *transferred
                    .lock()
                    .map_err(|_| "transfer gate poisoned while releasing dispatch".to_owned())? =
                    false;
                changed.notify_all();
                Ok((old_absent, new_sha))
            })();
            // Ensure a failed observation cannot leave the dispatch blocked on the gate.
            if observation.is_err() {
                let (transferred, changed) = &*watcher_gate;
                if let Ok(mut transferred) = transferred.lock() {
                    *transferred = false;
                    changed.notify_all();
                }
            }
            let _ = transfer_observed_tx.send(observation);
        });
        app.dispatch(AppCommand::ResumeInterrupted {
            request: previous.clone(),
        })
        .unwrap();
        let (old_binding_absent, transferred_sha) = transfer_observed_rx
            .recv_timeout(Duration::from_secs(12))
            .expect("watcher must observe the durable transfer before dispatch resumes")
            .unwrap_or_else(|error| panic!("durable binding transfer observation failed: {error}"));
        transfer_watcher
            .join()
            .expect("registry watcher must finish without moving ImportApplication");
        assert!(
            old_binding_absent,
            "registry must remove N before exposing N+1"
        );
        assert_eq!(transferred_sha, previous.source_sha256.unwrap());
        let mut observed_message = None;
        let mut observed_recoverable = false;
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            let view = app.dispatch(AppCommand::Refresh).unwrap();
            observed_message = view.message.clone();
            observed_recoverable = view.unified_parakeet_sessions.iter().any(|session| {
                (session.job_id, session.generation) == (previous.job_id, resumed_generation)
                    && session.phase == whisper_core::ports::UnifiedSessionPhase::Recoverable
            });
            if observed_recoverable {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        assert!(
            observed_recoverable,
            "archive failure after binding transfer must be terminal: {observed_message:?}"
        );
        assert!(
            observed_message
                .as_deref()
                .is_some_and(|message| message.contains("StorageCorrupt: segment"))
        );
        let source_binding_maps = source_binding_map_events(&source_binding_events);
        assert!(
            source_binding_maps.iter().any(|(operation, bindings)| {
                *operation == "resume-transfer"
                    && bindings == &vec![(previous.job_id, resumed_generation.get())]
            }),
            "successful transfer must leave only N+1 in memory: {source_binding_maps:?}"
        );
        assert!(
            source_binding_maps.iter().any(|(operation, bindings)| {
                *operation == "resume-rollback"
                    && bindings == &vec![(previous.job_id, previous.generation.get())]
            }),
            "rollback must restore only N in memory: {source_binding_maps:?}"
        );
        let queue = crate::queue_store::QueueStore::open(&previous.destination).unwrap();
        assert!(queue.entries().iter().any(|entry| {
            (entry.request.job_id, entry.request.generation)
                == (previous.job_id, previous.generation)
                && entry.status == crate::queue_store::QueueStatus::Interrupted
        }));
        assert_eq!(hex_sha256(&source_path), source_sha);
        let restored = crate::source_binding::SourceBinding::reopen_for_identity(
            &settings_root,
            Path::new(&previous.destination),
            previous.job_id,
            previous.generation,
        )
        .unwrap()
        .expect("failed post-transfer resume must restore the original durable binding");
        assert_eq!(restored.source_sha256(), previous.source_sha256.unwrap());
        drop(restored);
        assert!(
            crate::source_binding::SourceBinding::reopen_for_identity(
                &settings_root,
                Path::new(&previous.destination),
                previous.job_id,
                resumed_generation,
            )
            .unwrap()
            .is_none()
        );

        drop(app);
        crate::source_binding::SourceBinding::reopen_for_identity(
            &settings_root,
            Path::new(&previous.destination),
            previous.job_id,
            previous.generation,
        )
        .unwrap()
        .expect("restored binding remains available for cleanup")
        .retire()
        .unwrap();
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    #[cfg(windows)]
    #[ignore = "requires a separately authorized PU-L4 storage campaign and guarded WAV fixture over 1 GiB; uses controlled inference"]
    fn controlled_import_publishes_audio_over_one_gibibyte_across_physical_parts() {
        let execution_root = std::env::var_os("WHISPER_L04_CAMPAIGN_RUN")
            .map(PathBuf::from)
            .expect("guarded PU-L4 execution root is required")
            .canonicalize()
            .expect("guarded execution root must exist");
        let source_path = std::env::var_os("WHISPER_PU_L4_ROTATION_SOURCE")
            .map(PathBuf::from)
            .expect("guarded physical-rotation WAV fixture is required")
            .canonicalize()
            .expect("physical-rotation WAV fixture must exist");
        assert!(
            source_path.starts_with(&execution_root),
            "physical-rotation input must be copied under the guarded lot"
        );
        let source_bytes = std::fs::metadata(&source_path)
            .expect("stat physical-rotation fixture")
            .len();
        assert!(
            source_bytes >= 1_073_741_869,
            "physical-rotation fixture contains {source_bytes} bytes; expected >1 GiB"
        );
        let mut source = std::fs::File::open(&source_path).expect("open physical-rotation fixture");
        let mut header = [0_u8; 44];
        std::io::Read::read_exact(&mut source, &mut header).expect("read fixture WAV header");
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
        assert_eq!(
            data_bytes % 2,
            0,
            "PCM16 fixture must contain complete samples"
        );
        assert_eq!(source_bytes, 44 + u64::from(data_bytes));
        assert!(
            u64::from(data_bytes) > (1_u64 << 30),
            "PCM payload must cross the unchanged production rotation threshold"
        );
        let expected_samples = u64::from(data_bytes / 2);
        let mut fixture_buffer = vec![0_u8; 160_000];
        let mut fixture_bytes_remaining = u64::from(data_bytes);
        while fixture_bytes_remaining > 0 {
            let byte_count = fixture_bytes_remaining.min(fixture_buffer.len() as u64) as usize;
            std::io::Read::read_exact(&mut source, &mut fixture_buffer[..byte_count])
                .expect("read bounded physical-rotation fixture samples");
            assert!(
                fixture_buffer[..byte_count]
                    .iter()
                    .all(|sample| *sample == 0),
                "physical-rotation fixture must match the deterministic zero-sample pattern"
            );
            fixture_bytes_remaining -= byte_count as u64;
        }

        let destination = execution_root.join(format!(
            "controlled-rotation-archive-{}",
            std::process::id()
        ));
        let request = unified_import_request(&destination, &source_path, 9_510);
        assert_real_unified_model(&request);
        let (worker, calls) = controlled_worker();
        let runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(worker);
        let mut app = ImportApplication::new(RuntimeImportPort {
            runtime,
            events: VecDeque::new(),
            capture_progress: None,
            document_revision: None,
            unified_stop_submissions: None,
            controlled_capture_blocks: None,
        });
        app.dispatch(AppCommand::StartImport {
            request: request.clone(),
        })
        .expect("start real application route with controlled inference");

        let deadline = Instant::now() + Duration::from_secs(30 * 60);
        let terminal = loop {
            let view = app
                .dispatch(AppCommand::Refresh)
                .expect("refresh controlled physical-rotation import");
            if view.unified_parakeet_sessions.iter().any(|session| {
                session.job_id == request.job_id
                    && session.generation == request.generation
                    && matches!(
                        session.phase,
                        whisper_core::ports::UnifiedSessionPhase::Complete
                            | whisper_core::ports::UnifiedSessionPhase::Recoverable
                    )
            }) {
                break view;
            }
            assert!(
                Instant::now() < deadline,
                "controlled physical import timed out"
            );
            thread::sleep(Duration::from_millis(10));
        };
        let session = terminal
            .unified_parakeet_sessions
            .iter()
            .find(|session| {
                session.job_id == request.job_id && session.generation == request.generation
            })
            .expect("application view must retain the physical-import identity");
        assert_eq!(
            session.phase,
            whisper_core::ports::UnifiedSessionPhase::Complete,
            "controlled storage/publication path failed: {terminal:?}"
        );
        let archive = ArchiveStore::scan(&destination)
            .expect("scan controlled physical-rotation archive")
            .into_iter()
            .find(|entry| entry.job_id == request.job_id && entry.generation == request.generation)
            .expect("controlled import must retain its archive identity");
        assert!(archive.complete, "archive must publish all physical parts");
        let files =
            ArchiveStore::published_unified_files(&destination, request.job_id, request.generation)
                .expect("published audio, TXT and SRT references must exist");
        for path in [&files.wav_path, &files.txt_path, &files.srt_path] {
            assert!(
                Path::new(path.as_str()).is_file(),
                "published artifact missing: {path}"
            );
        }
        let audio = crate::parakeet_audio::PcmPartStore::open(&files.wav_path);
        let mut index_offset = 0_u64;
        let mut covered_samples = 0_u64;
        let mut part_count = 0_u64;
        loop {
            let page = audio
                .read_page(index_offset, 128)
                .expect("read bounded output index page");
            if page.is_empty() {
                break;
            }
            for part in &page {
                assert_eq!(part.number, part_count + 1);
                assert_eq!(part.start_sample, covered_samples);
                assert!(part.end_sample > part.start_sample);
                let part_bytes = std::fs::metadata(&part.path)
                    .expect("published physical WAV part must exist")
                    .len();
                assert_eq!(part_bytes, 44 + (part.end_sample - part.start_sample) * 2);
                let mut physical_part =
                    std::fs::File::open(&part.path).expect("open published physical WAV part");
                let mut part_header = [0_u8; 44];
                std::io::Read::read_exact(&mut physical_part, &mut part_header)
                    .expect("read physical WAV part header");
                assert_eq!(&part_header[0..4], b"RIFF");
                assert_eq!(&part_header[36..40], b"data");
                assert_eq!(
                    u64::from(u32::from_le_bytes(part_header[40..44].try_into().unwrap())),
                    (part.end_sample - part.start_sample) * 2
                );
                covered_samples = part.end_sample;
                part_count += 1;
            }
            index_offset += page.len() as u64;
        }
        assert!(
            part_count > 1,
            "published output must cross the physical rotation threshold"
        );
        assert_eq!(covered_samples, expected_samples);
        assert_eq!(audio.total_samples().unwrap(), expected_samples);
        let mut read_offset = 0_u64;
        while read_offset < expected_samples {
            let end = (read_offset + 80_000).min(expected_samples);
            let samples = audio
                .read_range(read_offset..end)
                .expect("read bounded PCM from every published part");
            assert_eq!(samples.len() as u64, end - read_offset);
            assert!(
                samples.iter().all(|sample| *sample == 0),
                "published samples {read_offset}..{end} differ from the fixture pattern"
            );
            read_offset = end;
        }
        assert!(
            calls.load(Ordering::Relaxed) > 0,
            "controlled finalizer must process windows"
        );
    }
