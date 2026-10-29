    #[test]
    fn import_application_finalizes_stopped_capture_a_while_capture_b_pumps_and_publishes_segmented_audio()
     {
        const A_ID: u128 = 9_301;
        const B_ID: u128 = 9_302;
        const THIRD_ID: u128 = 9_303;
        const A_SAMPLES: usize = 121 * 16_000;
        const B_SAMPLES: usize = 35 * 16_000;
        const PART_LIMIT_BYTES: u64 = 32_000;
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-capture-ab-{}",
            std::process::id()
        ));
        let archive_root = root.join("archive");
        std::fs::create_dir_all(&archive_root).unwrap();

        let request_a = unified_live_request(&archive_root, A_ID);
        let request_b = unified_live_request(&archive_root, B_ID);
        let request_third = unified_live_request(&archive_root, THIRD_ID);
        assert!(Path::new(&request_a.model_path).is_file());

        let mut capture_a =
            crate::capture::CaptureStream::empty_test_capture(16_000, 1, "test:capture-a");
        for _ in 0..25 {
            capture_a.enqueue_test_block(vec![0.25; 77_440]);
        }
        let mut capture_b =
            crate::capture::CaptureStream::empty_test_capture(16_000, 1, "test:capture-b");
        for _ in 0..7 {
            capture_b.enqueue_test_block(vec![0.15; 5 * 16_000]);
        }
        let mut capture_third = crate::capture::CaptureStream::empty_test_capture(
            16_000,
            1,
            "test:capture-third-after-reap",
        );
        capture_third.enqueue_test_block(vec![0.1; 3 * 16_000]);

        let (worker, calls, maximum_active, call_sources, held_rx, release_tx) =
            held_final_a_worker();
        let b_progress = Arc::new(AtomicU64::new(0));
        let stop_submissions = Arc::new(AtomicU64::new(0));
        let runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(worker)
        .with_controlled_capture(capture_a, Some(PART_LIMIT_BYTES))
        .with_controlled_capture(capture_b, Some(PART_LIMIT_BYTES))
        .with_controlled_capture(capture_third, Some(PART_LIMIT_BYTES));
        let mut app = ImportApplication::new(RuntimeImportPort {
            runtime,
            events: VecDeque::new(),
            capture_progress: Some((JobId(B_ID), Arc::clone(&b_progress))),
            document_revision: None,
            unified_stop_submissions: Some(Arc::clone(&stop_submissions)),
            controlled_capture_blocks: None,
        });

        app.dispatch(AppCommand::StartUnifiedLiveConfigured {
            request: request_a.clone(),
        })
        .unwrap();
        for _ in 0..100 {
            app.dispatch(AppCommand::Refresh).unwrap();
            if calls.load(Ordering::Relaxed) >= 1 {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        assert!(
            calls.load(Ordering::Relaxed) >= 1,
            "capture A preview was not scheduled"
        );
        assert!(matches!(
            app.dispatch(AppCommand::StartUnifiedLiveConfigured {
                request: request_b.clone(),
            }),
            Err(whisper_core::application::ApplicationError::InvalidCommand)
        ));
        assert!(
            !archive_root
                .join("transcriptions")
                .join(format!("{:032x}", B_ID))
                .exists()
        );
        app.dispatch(AppCommand::StopUnifiedLive {
            job_id: request_a.job_id,
            generation: request_a.generation,
        })
        .unwrap();

        let held_deadline = Instant::now() + Duration::from_secs(5);
        let mut held_source = None;
        while Instant::now() < held_deadline {
            app.dispatch(AppCommand::Refresh).unwrap();
            match held_rx.try_recv() {
                Ok(source) => {
                    held_source = Some(source);
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    thread::sleep(Duration::from_millis(2));
                }
                Err(error) => panic!("capture A final worker exited before holding: {error}"),
            }
        }
        assert_eq!(
            held_source.as_deref(),
            Some("A"),
            "capture A final inference was not held"
        );
        let held_call_count = calls.load(Ordering::Relaxed);
        assert!(held_call_count >= 2);
        assert_eq!(maximum_active.load(Ordering::Relaxed), 1);

        app.dispatch(AppCommand::StartUnifiedLiveConfigured {
            request: request_b.clone(),
        })
        .unwrap();
        assert!(
            matches!(
                app.dispatch(AppCommand::StartUnifiedLiveConfigured {
                    request: request_third.clone(),
                }),
                Err(whisper_core::application::ApplicationError::InvalidCommand)
            ),
            "third session must be refused before the runtime opens its capture"
        );
        assert!(
            !root
                .join("archive/transcriptions")
                .join(format!("{:032x}", THIRD_ID))
                .exists()
        );

        let b_pump_deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < b_pump_deadline && b_progress.load(Ordering::Relaxed) == 0 {
            app.dispatch(AppCommand::Refresh).unwrap();
            thread::sleep(Duration::from_millis(2));
        }
        let b_written_before_stop = b_progress.load(Ordering::Relaxed);
        assert!(
            b_written_before_stop > 0,
            "B did not progress during A's held call"
        );
        assert!(
            b_written_before_stop < B_SAMPLES as u64,
            "the capture's delayed converter tail is acknowledged only at Stop"
        );
        assert_eq!(
            calls.load(Ordering::Relaxed),
            held_call_count,
            "B must pump without a second backend call"
        );
        assert_eq!(maximum_active.load(Ordering::Relaxed), 1);

        app.dispatch(AppCommand::StopUnifiedLive {
            job_id: request_b.job_id,
            generation: request_b.generation,
        })
        .unwrap();
        assert_eq!(stop_submissions.load(Ordering::Relaxed), 2);
        release_tx.send(()).unwrap();

        let publication_deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < publication_deadline {
            app.dispatch(AppCommand::Refresh).unwrap();
            let entries = ArchiveStore::scan(&archive_root).unwrap();
            let published = [JobId(A_ID), JobId(B_ID)].into_iter().all(|job_id| {
                entries
                    .iter()
                    .any(|entry| entry.job_id == job_id && entry.complete)
            });
            if published {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }

        let entries = ArchiveStore::scan(&archive_root).unwrap();
        let final_call_count = calls.load(Ordering::Relaxed);
        let entry_for = |job_id| entries.iter().find(|entry| entry.job_id == job_id);
        for (job_id, label) in [(JobId(A_ID), "A"), (JobId(B_ID), "B")] {
            let entry = entry_for(job_id).unwrap_or_else(|| {
                panic!(
                    "capture {label} has no archive entry; A={:?}, B={:?}, app_view={:?}",
                    entry_for(JobId(A_ID)),
                    entry_for(JobId(B_ID)),
                    app.view()
                )
            });
            assert!(
                entry.complete,
                "capture {label} was not published; A={:?}, B={:?}, app_view={:?}",
                entry_for(JobId(A_ID)),
                entry_for(JobId(B_ID)),
                app.view()
            );
        }
        let a_entry = entry_for(JobId(A_ID)).unwrap();
        let b_entry = entry_for(JobId(B_ID)).unwrap();
        let a_basename = a_entry.export_basename.as_deref().unwrap();
        let b_basename = b_entry.export_basename.as_deref().unwrap();
        assert_ne!(
            a_basename, b_basename,
            "simultaneous unpublished generations must hold distinct export reservations"
        );
        for (job_id, expected_samples, label) in [
            (JobId(A_ID), A_SAMPLES as u64, "A"),
            (JobId(B_ID), B_SAMPLES as u64, "B"),
        ] {
            let entry = entries
                .iter()
                .find(|entry| entry.job_id == job_id && entry.complete)
                .unwrap_or_else(|| {
                    panic!(
                        "capture {label} was not fully published; A={:?}, B={:?}, app_view={:?}",
                        entry_for(JobId(A_ID)),
                        entry_for(JobId(B_ID)),
                        app.view()
                    )
                });
            let generation = archive_root
                .join("transcriptions")
                .join(format!("{:032x}", job_id.0))
                .join("1");
            assert!(generation.join("CURRENT").is_file());
            assert_eq!(
                std::fs::read_to_string(generation.join("CURRENT")).unwrap(),
                "manifest.json\n"
            );
            assert!(generation.join("transcript.txt").is_file());
            assert!(generation.join("transcript.srt").is_file());
            let transcript = std::fs::read_to_string(generation.join("transcript.txt")).unwrap();
            if label == "A" {
                assert!(transcript.contains(&format!("A-final-{final_call_count}.")));
            } else {
                assert!(transcript.contains(&format!("B-final-{}.", final_call_count - 1)));
            }
            let basename = entry.export_basename.as_ref().unwrap();
            assert_eq!(
                std::fs::read(archive_root.join("exports").join(format!("{basename}.txt")))
                    .unwrap(),
                std::fs::read(generation.join("transcript.txt")).unwrap(),
                "capture {label} TXT export must belong to its published generation"
            );
            assert_eq!(
                std::fs::read(archive_root.join("exports").join(format!("{basename}.srt")))
                    .unwrap(),
                std::fs::read(generation.join("transcript.srt")).unwrap(),
                "capture {label} SRT export must belong to its published generation"
            );

            let manifest_path = generation.join("captured-source-parts.json");
            let manifest: serde_json::Value =
                serde_json::from_slice(&std::fs::read(manifest_path).unwrap()).unwrap();
            assert_eq!(manifest["source_samples"].as_u64(), Some(expected_samples));
            let parts = manifest["parts"].as_array().unwrap();
            assert!(
                parts.len() > 1,
                "capture {label} did not rotate into multiple parts"
            );
            let mut cursor = 0_u64;
            for part in parts {
                assert_eq!(part["start_sample"].as_u64(), Some(cursor));
                cursor = part["end_sample"].as_u64().unwrap();
                assert!(Path::new(part["path"].as_str().unwrap()).is_file());
            }
            assert_eq!(cursor, expected_samples);

            let source_path = generation.join("parakeet-unified/source.wav");
            let source = crate::parakeet_audio::PcmPartStore::open(source_path);
            let (block_count, block_samples, input_level) = if label == "B" {
                (7, 5 * 16_000, 0.15_f32)
            } else {
                (25, 77_440, 0.25_f32)
            };
            let mut reference_capture =
                crate::capture::CaptureStream::empty_test_capture(16_000, 1, "test:pcm-reference");
            for _ in 0..block_count {
                reference_capture.enqueue_test_block(vec![input_level; block_samples]);
            }
            let mut reference_converter = crate::capture::Pcm16Converter::new(16_000, 1).unwrap();
            let mut expected_pcm = Vec::with_capacity(expected_samples as usize);
            while let Some(block) = reference_capture.try_next_block() {
                expected_pcm.extend(reference_converter.push(&block).unwrap());
                reference_capture.recycle_block(block).unwrap();
            }
            expected_pcm.extend(reference_converter.finish().unwrap());
            assert_eq!(expected_pcm.len() as u64, expected_samples);
            let steady_level = (input_level * 32_767.0).round() as i16;
            assert!(
                expected_pcm[128..expected_pcm.len() - 128]
                    .iter()
                    .all(|sample| sample.abs_diff(steady_level) <= 2)
            );
            let mut start = 0_u64;
            while start < expected_samples {
                let end = start.saturating_add(80_000).min(expected_samples);
                let samples = source.read_window(start..end).unwrap();
                assert_eq!(samples.len() as u64, end - start);
                assert!(
                    samples
                        .iter()
                        .zip(&expected_pcm[start as usize..end as usize])
                        .all(|(actual, expected)| actual.abs_diff(*expected) <= 2),
                    "capture {label} PCM differs from the sequential converter reference in {start}..{end}"
                );
                let interior_start = start.max(128);
                let interior_end = end.min(expected_samples - 128);
                if interior_start < interior_end {
                    let local_start = (interior_start - start) as usize;
                    let local_end = (interior_end - start) as usize;
                    assert!(
                        samples[local_start..local_end]
                            .iter()
                            .all(|sample| sample.abs_diff(steady_level) <= 2)
                    );
                }
                start = end;
            }

            let coverage = std::fs::read_to_string(
                generation.join("parakeet-unified/document/coverage.jsonl"),
            )
            .unwrap();
            let windows = coverage
                .lines()
                .map(|line| {
                    serde_json::from_str::<whisper_core::ports::ParakeetWindowCoverage>(line)
                        .unwrap()
                })
                .collect::<Vec<_>>();
            assert!(!windows.is_empty());
            assert_eq!(windows.first().unwrap().range.start_sample, 0);
            assert_eq!(windows.last().unwrap().range.end_sample, expected_samples);
            assert!(
                windows
                    .windows(2)
                    .all(|pair| { pair[0].range.end_sample == pair[1].range.start_sample })
            );
        }
        assert_eq!(maximum_active.load(Ordering::Relaxed), 1);
        let call_sources = call_sources.lock().unwrap();
        let final_order = call_sources
            .iter()
            .rev()
            .take(3)
            .map(String::as_str)
            .collect::<Vec<_>>();
        assert_eq!(
            final_order.into_iter().rev().collect::<Vec<_>>(),
            ["A:final", "B:final", "A:final"],
            "final windows must alternate after A's held window and preserve its tail"
        );
        drop(call_sources);

        app.dispatch(AppCommand::StartUnifiedLiveConfigured {
            request: request_third.clone(),
        })
        .unwrap();
        app.dispatch(AppCommand::StopUnifiedLive {
            job_id: request_third.job_id,
            generation: request_third.generation,
        })
        .unwrap();
        let third_publication_deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < third_publication_deadline {
            app.dispatch(AppCommand::Refresh).unwrap();
            if ArchiveStore::scan(&archive_root)
                .unwrap()
                .iter()
                .any(|entry| entry.job_id == request_third.job_id && entry.complete)
            {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        assert!(
            ArchiveStore::scan(&archive_root)
                .unwrap()
                .iter()
                .any(|entry| entry.job_id == request_third.job_id && entry.complete)
        );
        assert_eq!(stop_submissions.load(Ordering::Relaxed), 3);
    }

    #[test]
    fn global_close_waits_for_both_capture_publications_and_refuses_new_admission() {
        const A_ID: u128 = 9_321;
        const B_ID: u128 = 9_322;
        const THIRD_ID: u128 = 9_323;
        const PART_LIMIT_BYTES: u64 = 32_000;
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-global-close-{}",
            std::process::id()
        ));
        let archive_root = root.join("archive");
        std::fs::create_dir_all(&archive_root).unwrap();
        let request_a = unified_live_request(&archive_root, A_ID);
        let request_b = unified_live_request(&archive_root, B_ID);
        let request_third = unified_live_request(&archive_root, THIRD_ID);
        assert!(
            Path::new(&request_a.model_path).is_file(),
            "required real Parakeet test model is missing: {}",
            request_a.model_path
        );

        let mut capture_a =
            crate::capture::CaptureStream::empty_test_capture(16_000, 1, "test:close-a");
        capture_a.enqueue_test_block(vec![0.25; 5 * 16_000]);
        capture_a.enqueue_test_block(vec![0.25; 5 * 16_000]);
        let mut capture_b =
            crate::capture::CaptureStream::empty_test_capture(16_000, 1, "test:close-b");
        capture_b.enqueue_test_block(vec![0.15; 5 * 16_000]);
        capture_b.enqueue_test_block(vec![0.15; 5 * 16_000]);

        let (worker, calls, _, _, held_rx, release_tx) = held_final_a_worker();
        let stop_submissions = Arc::new(AtomicU64::new(0));
        let runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(worker)
        .with_controlled_capture(capture_a, Some(PART_LIMIT_BYTES))
        .with_controlled_capture(capture_b, Some(PART_LIMIT_BYTES));
        let mut app = ImportApplication::new(RuntimeImportPort {
            runtime,
            events: VecDeque::new(),
            capture_progress: None,
            document_revision: None,
            unified_stop_submissions: Some(Arc::clone(&stop_submissions)),
            controlled_capture_blocks: None,
        });

        app.dispatch(AppCommand::StartUnifiedLiveConfigured {
            request: request_a.clone(),
        })
        .unwrap();
        let a_preview_deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < a_preview_deadline && calls.load(Ordering::Relaxed) == 0 {
            app.dispatch(AppCommand::Refresh).unwrap();
            thread::sleep(Duration::from_millis(2));
        }
        assert!(
            calls.load(Ordering::Relaxed) > 0,
            "A did not begin processing"
        );
        app.dispatch(AppCommand::StopUnifiedLive {
            job_id: request_a.job_id,
            generation: request_a.generation,
        })
        .unwrap();
        let held_deadline = Instant::now() + Duration::from_secs(5);
        let mut held_source = None;
        while Instant::now() < held_deadline {
            app.dispatch(AppCommand::Refresh).unwrap();
            match held_rx.try_recv() {
                Ok(source) => {
                    held_source = Some(source);
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    thread::sleep(Duration::from_millis(2));
                }
                Err(error) => panic!("A final call exited before the close test held it: {error}"),
            }
        }
        assert_eq!(held_source.as_deref(), Some("A"));

        app.dispatch(AppCommand::StartUnifiedLiveConfigured {
            request: request_b.clone(),
        })
        .unwrap();
        let b_source_path = archive_root
            .join("transcriptions")
            .join(format!("{:032x}", request_b.job_id.0))
            .join(request_b.generation.get().to_string())
            .join("parakeet-unified/source.wav");
        let b_progress_deadline = Instant::now() + Duration::from_secs(5);
        let mut b_card_samples = 0;
        let mut b_written_samples = 0;
        while Instant::now() < b_progress_deadline {
            app.dispatch(AppCommand::Refresh).unwrap();
            b_card_samples = app
                .view()
                .unified_parakeet_sessions
                .iter()
                .find(|session| session.job_id == request_b.job_id)
                .map_or(0, |session| session.durable_samples);
            if b_source_path.is_file() {
                b_written_samples = crate::parakeet_audio::PcmPartStore::open(&b_source_path)
                    .total_samples()
                    .unwrap_or(0);
            }
            if b_card_samples > 0 && b_written_samples > 0 {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        assert!(
            b_written_samples > 0,
            "B's acknowledged audio write did not progress while A's final inference was held"
        );
        assert!(
            b_card_samples > 0,
            "B's acknowledged capture progress did not reach its session card while A was held"
        );
        assert!(
            b_card_samples <= b_written_samples,
            "B's session card progress exceeded the acknowledged audio write"
        );

        app.dispatch(AppCommand::RequestClose).unwrap();
        assert!(
            !app.view().close_ready,
            "close must await A and B publication"
        );
        assert!(matches!(
            app.dispatch(AppCommand::StartUnifiedLiveConfigured {
                request: request_third.clone(),
            }),
            Err(whisper_core::application::ApplicationError::InvalidCommand)
        ));
        assert!(
            !archive_root
                .join("transcriptions")
                .join(format!("{:032x}", THIRD_ID))
                .exists(),
            "the third admission must be refused before opening capture or archive state"
        );
        app.dispatch(AppCommand::RequestClose).unwrap();
        assert_eq!(stop_submissions.load(Ordering::Relaxed), 2);
        let stopped_progress_deadline = Instant::now() + Duration::from_secs(5);
        let mut both_finalizing = false;
        while Instant::now() < stopped_progress_deadline {
            app.dispatch(AppCommand::Refresh).unwrap();
            both_finalizing = [request_a.job_id, request_b.job_id]
                .into_iter()
                .all(|job_id| {
                    app.view()
                        .unified_parakeet_sessions
                        .iter()
                        .find(|session| session.job_id == job_id)
                        .is_some_and(|session| {
                            session.total_samples.is_some_and(|total| {
                                session.captured_samples == total
                                    && session.durable_samples == total
                                    && session.phase
                                        == whisper_core::ports::UnifiedSessionPhase::Finalizing
                            })
                        })
                });
            if both_finalizing {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        assert!(
            both_finalizing,
            "closed captures must expose final flushed progress without leaving Finalizing"
        );
        release_tx.send(()).unwrap();

        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            app.dispatch(AppCommand::Refresh).unwrap();
            let entries = ArchiveStore::scan(&archive_root).unwrap();
            let both_published = [request_a.job_id, request_b.job_id]
                .into_iter()
                .all(|job_id| {
                    entries
                        .iter()
                        .any(|entry| entry.job_id == job_id && entry.complete)
                });
            if both_published && app.view().close_ready {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        let entries = ArchiveStore::scan(&archive_root).unwrap();
        for job_id in [request_a.job_id, request_b.job_id] {
            assert!(
                entries
                    .iter()
                    .any(|entry| entry.job_id == job_id && entry.complete)
            );
            let view = app.view();
            let session = view
                .unified_parakeet_sessions
                .iter()
                .find(|session| session.job_id == job_id)
                .expect("published capture remains visible in its session card");
            let total_samples = session
                .total_samples
                .expect("closed capture reports its final sample count");
            assert_eq!(session.captured_samples, total_samples);
            assert_eq!(session.durable_samples, total_samples);
            assert_eq!(
                session.phase,
                whisper_core::ports::UnifiedSessionPhase::Complete
            );
            let generation = archive_root
                .join("transcriptions")
                .join(format!("{:032x}", job_id.0))
                .join("1");
            assert!(generation.join("CURRENT").is_file());
            assert!(generation.join("transcript.txt").is_file());
            assert!(generation.join("transcript.srt").is_file());
        }
        assert!(
            app.view().close_ready,
            "close is ready only after both commits"
        );
        assert_eq!(stop_submissions.load(Ordering::Relaxed), 2);
        assert_eq!(ArchiveStore::scan(&archive_root).unwrap().len(), 2);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn import_application_import_a_and_capture_b_share_one_held_backend_until_b_is_published() {
        const A_ID: u128 = 9_311;
        const B_ID: u128 = 9_312;
        const B_SAMPLES: usize = 35 * 16_000;
        const PART_LIMIT_BYTES: u64 = 32_000;
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-import-capture-ab-{}",
            std::process::id()
        ));
        let archive_root = root.join("archive");
        std::fs::create_dir_all(&archive_root).unwrap();
        let source_path = root.join("import-a.wav");
        let mut source_writer = crate::parakeet_audio::PcmPartWriter::create(&source_path).unwrap();
        source_writer.append(&vec![0_i16; 4 * 16_000]).unwrap();
        source_writer.finish().unwrap();
        let import_request = unified_import_request(&archive_root, &source_path, A_ID);
        let capture_request = unified_live_request(&archive_root, B_ID);
        assert_real_unified_model(&import_request);

        let mut capture_b =
            crate::capture::CaptureStream::empty_test_capture(16_000, 1, "test:import-capture-b");
        for _ in 0..7 {
            capture_b.enqueue_test_block(vec![0.15; 5 * 16_000]);
        }
        let (worker, calls, maximum_active, _call_sources, held_rx, release_tx) = held_worker(1);
        let b_progress = Arc::new(AtomicU64::new(0));
        let stop_submissions = Arc::new(AtomicU64::new(0));
        let runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(worker)
        .with_controlled_capture(capture_b, Some(PART_LIMIT_BYTES));
        let mut app = ImportApplication::new(RuntimeImportPort {
            runtime,
            events: VecDeque::new(),
            capture_progress: Some((JobId(B_ID), Arc::clone(&b_progress))),
            document_revision: None,
            unified_stop_submissions: Some(Arc::clone(&stop_submissions)),
            controlled_capture_blocks: None,
        });

        app.dispatch(AppCommand::StartImport {
            request: import_request.clone(),
        })
        .unwrap();
        let held_deadline = Instant::now() + Duration::from_secs(5);
        let mut held_source = None;
        while Instant::now() < held_deadline {
            app.dispatch(AppCommand::Refresh).unwrap();
            match held_rx.try_recv() {
                Ok(source) => {
                    held_source = Some(source);
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    thread::sleep(Duration::from_millis(2));
                }
                Err(error) => panic!("import A worker exited before holding: {error}"),
            }
        }
        assert!(
            held_source.is_some(),
            "import A final inference was not held"
        );
        assert_eq!(calls.load(Ordering::Relaxed), 1);

        app.dispatch(AppCommand::StartUnifiedLiveConfigured {
            request: capture_request.clone(),
        })
        .unwrap();
        let b_pump_deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < b_pump_deadline && b_progress.load(Ordering::Relaxed) == 0 {
            app.dispatch(AppCommand::Refresh).unwrap();
            thread::sleep(Duration::from_millis(2));
        }
        let b_written_before_stop = b_progress.load(Ordering::Relaxed);
        assert!(b_written_before_stop > 0);
        assert!(b_written_before_stop < B_SAMPLES as u64);
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        assert_eq!(maximum_active.load(Ordering::Relaxed), 1);

        app.dispatch(AppCommand::StopUnifiedLive {
            job_id: capture_request.job_id,
            generation: capture_request.generation,
        })
        .unwrap();
        app.dispatch(AppCommand::StopUnifiedLive {
            job_id: capture_request.job_id,
            generation: capture_request.generation,
        })
        .unwrap();
        assert_eq!(stop_submissions.load(Ordering::Relaxed), 1);
        release_tx.send(()).unwrap();

        let publication_deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < publication_deadline {
            app.dispatch(AppCommand::Refresh).unwrap();
            let entries = ArchiveStore::scan(&archive_root).unwrap();
            if [JobId(A_ID), JobId(B_ID)].into_iter().all(|job_id| {
                entries
                    .iter()
                    .any(|entry| entry.job_id == job_id && entry.complete)
            }) {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        let entries = ArchiveStore::scan(&archive_root).unwrap();
        for job_id in [JobId(A_ID), JobId(B_ID)] {
            let entry = entries
                .iter()
                .find(|entry| entry.job_id == job_id && entry.complete)
                .expect("both the file import and capture must reach publication");
            let generation = archive_root
                .join("transcriptions")
                .join(format!("{:032x}", job_id.0))
                .join("1");
            assert!(generation.join("transcript.txt").is_file());
            assert!(generation.join("transcript.srt").is_file());
            let basename = entry.export_basename.as_ref().unwrap();
            assert!(
                archive_root
                    .join("exports")
                    .join(format!("{basename}.txt"))
                    .is_file()
            );
            assert!(
                archive_root
                    .join("exports")
                    .join(format!("{basename}.srt"))
                    .is_file()
            );
        }
        let capture_generation = archive_root
            .join("transcriptions")
            .join(format!("{:032x}", B_ID))
            .join("1");
        let manifest: serde_json::Value = serde_json::from_slice(
            &std::fs::read(capture_generation.join("captured-source-parts.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(manifest["source_samples"].as_u64(), Some(B_SAMPLES as u64));
        let parts = manifest["parts"].as_array().unwrap();
        assert!(parts.len() > 1);
        let mut part_cursor = 0_u64;
        for part in parts {
            assert_eq!(part["start_sample"].as_u64(), Some(part_cursor));
            part_cursor = part["end_sample"].as_u64().unwrap();
            assert!(Path::new(part["path"].as_str().unwrap()).is_file());
        }
        assert_eq!(part_cursor, B_SAMPLES as u64);

        let mut reference_capture = crate::capture::CaptureStream::empty_test_capture(
            16_000,
            1,
            "test:import-capture-reference",
        );
        for _ in 0..7 {
            reference_capture.enqueue_test_block(vec![0.15; 5 * 16_000]);
        }
        let mut reference_converter = crate::capture::Pcm16Converter::new(16_000, 1).unwrap();
        let mut expected_pcm = Vec::with_capacity(B_SAMPLES);
        while let Some(block) = reference_capture.try_next_block() {
            expected_pcm.extend(reference_converter.push(&block).unwrap());
            reference_capture.recycle_block(block).unwrap();
        }
        expected_pcm.extend(reference_converter.finish().unwrap());
        assert_eq!(expected_pcm.len(), B_SAMPLES);
        let capture_source = crate::parakeet_audio::PcmPartStore::open(
            capture_generation.join("parakeet-unified/source.wav"),
        );
        let mut sample_cursor = 0_u64;
        while sample_cursor < B_SAMPLES as u64 {
            let end = sample_cursor.saturating_add(80_000).min(B_SAMPLES as u64);
            let samples = capture_source.read_window(sample_cursor..end).unwrap();
            assert_eq!(samples.len() as u64, end - sample_cursor);
            assert!(
                samples
                    .iter()
                    .zip(&expected_pcm[sample_cursor as usize..end as usize])
                    .all(|(actual, expected)| actual.abs_diff(*expected) <= 2)
            );
            sample_cursor = end;
        }
        let coverage = std::fs::read_to_string(
            capture_generation.join("parakeet-unified/document/coverage.jsonl"),
        )
        .unwrap();
        let windows = coverage
            .lines()
            .map(|line| {
                serde_json::from_str::<whisper_core::ports::ParakeetWindowCoverage>(line).unwrap()
            })
            .collect::<Vec<_>>();
        assert!(!windows.is_empty());
        assert_eq!(windows.first().unwrap().range.start_sample, 0);
        assert_eq!(windows.last().unwrap().range.end_sample, B_SAMPLES as u64);
        assert!(
            windows
                .windows(2)
                .all(|pair| pair[0].range.end_sample == pair[1].range.start_sample)
        );
        assert_eq!(maximum_active.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn unified_capture_updates_preview_and_final_document_before_stop_then_publishes_the_tail() {
        const JOB_ID: u128 = 9_321;
        const TOTAL_SAMPLES: u64 = 132 * 16_000;
        const PART_LIMIT_BYTES: u64 = 64_000;
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-live-document-{}",
            std::process::id()
        ));
        let archive_root = root.join("archive");
        std::fs::create_dir_all(&archive_root).unwrap();
        let request = unified_live_request(&archive_root, JOB_ID);
        assert!(Path::new(&request.model_path).is_file());

        let mut capture = crate::capture::CaptureStream::empty_test_capture(
            16_000,
            1,
            "test:unified-document-progress",
        );
        for _ in 0..33 {
            capture.enqueue_test_block(vec![0.2; 4 * 16_000]);
        }
        let document_revision = Arc::new(AtomicU64::new(0));
        let stop_submissions = Arc::new(AtomicU64::new(0));
        let observed_windows = Arc::new(Mutex::new(Vec::new()));
        let runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(Box::new(ObservedTailAwareUnifiedWorker {
            windows: Arc::clone(&observed_windows),
        }))
        .with_controlled_capture(capture, Some(PART_LIMIT_BYTES));
        let mut app = ImportApplication::new(RuntimeImportPort {
            runtime,
            events: VecDeque::new(),
            capture_progress: None,
            document_revision: Some((JobId(JOB_ID), Arc::clone(&document_revision))),
            unified_stop_submissions: Some(Arc::clone(&stop_submissions)),
            controlled_capture_blocks: None,
        });
        app.dispatch(AppCommand::StartUnifiedLiveConfigured {
            request: request.clone(),
        })
        .unwrap();

        let document_directory = archive_root
            .join("transcriptions")
            .join(format!("{:032x}", JOB_ID))
            .join("1/parakeet-unified/document");
        let live_progress_deadline = Instant::now() + Duration::from_secs(30);
        let mut preview_revision = 0_u64;
        let mut preview_range = None;
        while Instant::now() < live_progress_deadline
            && (document_revision.load(Ordering::Relaxed) == 0 || preview_revision < 2)
        {
            app.dispatch(AppCommand::Refresh).unwrap();
            if let Ok(bytes) = std::fs::read(document_directory.join("document.preview.json")) {
                if let Ok(preview) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                    preview_revision = preview["revision"].as_u64().unwrap_or(0);
                    preview_range = Some((
                        preview["segment"]["range"]["start_sample"]
                            .as_u64()
                            .unwrap_or(0),
                        preview["segment"]["range"]["end_sample"]
                            .as_u64()
                            .unwrap_or(0),
                    ));
                }
            }
            thread::sleep(Duration::from_millis(2));
        }
        assert!(
            document_revision.load(Ordering::Relaxed) > 0,
            "a final page must be committed while capture is still open"
        );
        assert!(preview_revision >= 2, "preview must be replaced in place");
        let (preview_start, preview_end) = preview_range.unwrap();
        let observed_windows = observed_windows.lock().unwrap().clone();
        assert!(
            observed_windows.iter().any(|(start, end, pcm_len)| {
                *end - *start == 4 * 16_000 && *pcm_len == 4 * 16_000
            }),
            "controlled worker must receive the 64,000-sample four-second preview PCM window"
        );
        assert!(
            observed_windows.iter().any(|(start, end, pcm_len)| {
                *end - *start == 4 * 16_000
                    && *pcm_len == 4 * 16_000
                    && preview_start >= *start
                    && preview_end <= *end
                    && preview_start < preview_end
            }),
            "preview segment must remain bounded by its actual worker context"
        );
        assert_eq!(stop_submissions.load(Ordering::Relaxed), 0);

        app.dispatch(AppCommand::StopUnifiedLive {
            job_id: request.job_id,
            generation: request.generation,
        })
        .unwrap();
        app.dispatch(AppCommand::StopUnifiedLive {
            job_id: request.job_id,
            generation: request.generation,
        })
        .unwrap();
        assert_eq!(stop_submissions.load(Ordering::Relaxed), 1);

        let publication_deadline = Instant::now() + Duration::from_secs(30);
        while Instant::now() < publication_deadline {
            app.dispatch(AppCommand::Refresh).unwrap();
            if ArchiveStore::scan(&archive_root)
                .unwrap()
                .iter()
                .any(|entry| entry.job_id == request.job_id && entry.complete)
            {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        let entry = ArchiveStore::scan(&archive_root)
            .unwrap()
            .into_iter()
            .find(|entry| entry.job_id == request.job_id && entry.complete)
            .expect("stopped capture must publish the final tail");
        let generation_directory = archive_root
            .join("transcriptions")
            .join(format!("{:032x}", JOB_ID))
            .join("1");
        let transcript =
            std::fs::read_to_string(generation_directory.join("transcript.txt")).unwrap();
        assert!(transcript.contains("terminal-phrase."));
        assert!(generation_directory.join("transcript.srt").is_file());
        let basename = entry.export_basename.unwrap();
        assert!(
            archive_root
                .join("exports")
                .join(format!("{basename}.txt"))
                .is_file()
        );
        assert!(
            archive_root
                .join("exports")
                .join(format!("{basename}.srt"))
                .is_file()
        );

        let manifest: serde_json::Value = serde_json::from_slice(
            &std::fs::read(generation_directory.join("captured-source-parts.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(manifest["source_samples"].as_u64(), Some(TOTAL_SAMPLES));
        let parts = manifest["parts"].as_array().unwrap();
        assert!(parts.len() > 1);
        let mut cursor = 0_u64;
        for part in parts {
            assert_eq!(part["start_sample"].as_u64(), Some(cursor));
            cursor = part["end_sample"].as_u64().unwrap();
            assert!(Path::new(part["path"].as_str().unwrap()).is_file());
        }
        assert_eq!(cursor, TOTAL_SAMPLES);

        let coverage = std::fs::read_to_string(
            generation_directory.join("parakeet-unified/document/coverage.jsonl"),
        )
        .unwrap();
        let windows = coverage
            .lines()
            .map(|line| {
                serde_json::from_str::<whisper_core::ports::ParakeetWindowCoverage>(line).unwrap()
            })
            .collect::<Vec<_>>();
        assert!(
            windows.len() >= 2,
            "the source tail after 120 seconds must be finalized"
        );
        assert_eq!(windows.first().unwrap().range.start_sample, 0);
        assert_eq!(windows.last().unwrap().range.end_sample, TOTAL_SAMPLES);
        assert!(
            windows
                .windows(2)
                .all(|pair| { pair[0].range.end_sample == pair[1].range.start_sample })
        );
    }

    #[test]
    fn controlled_runtime_recycles_scheduler_slots_across_many_unified_imports() {
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-scheduler-lifecycle-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        );
        for sequence in 1..=32_u128 {
            let source_path = root.join(format!("source-{sequence}.wav"));
            let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_path).unwrap();
            writer.append(&vec![0_i16; 48_000]).unwrap();
            writer.finish().unwrap();
            let request = unified_import_request(&root, &source_path, 800 + sequence);
            assert_real_unified_model(&request);
            runtime = runtime.with_controlled_transcriber(controlled_worker().0);
            runtime.run_controlled_unified_import(&request).unwrap();
            assert_eq!(runtime.parakeet_scheduler.active_sessions(), 0);
            assert!(runtime.parakeet_scheduler.in_flight().is_none());
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn import_application_decodes_real_mp3_and_publishes_full_unified_coverage() {
        let fixture = std::env::var_os("WHISPER_PU_L1_MP3_FIXTURE")
            .map(PathBuf::from)
            .expect("WHISPER_PU_L1_MP3_FIXTURE must name the guarded MP3 fixture");
        let fixture = fixture
            .canonicalize()
            .expect("guarded MP3 fixture must exist");
        let source_before = std::fs::read(&fixture).expect("read guarded MP3 fixture");
        assert_eq!(source_before.len(), 22_822);
        assert_eq!(
            format!("{:x}", Sha256::digest(&source_before)),
            "eebce5a2af18d4f929af5c243c50a6bc31b0910a583577cb940e0e4895e7cd80"
        );

        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-mp3-application-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let job_id = JobId(794);
        let request = unified_import_request(&root.join("archive"), &fixture, job_id.0);
        assert_real_unified_model(&request);
        let runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(controlled_worker().0);
        let port = RuntimeImportPort {
            runtime,
            events: VecDeque::new(),
            capture_progress: None,
            document_revision: None,
            unified_stop_submissions: None,
            controlled_capture_blocks: None,
        };
        let mut app = ImportApplication::new(port);
        app.dispatch(AppCommand::StartImport {
            request: request.clone(),
        })
        .unwrap();

        let mut completed = false;
        let mut last_state = None;
        let mut last_message = None;
        for _ in 0..64 {
            let view = app.dispatch(AppCommand::Refresh).unwrap();
            last_state = view.active_job;
            last_message = view.message.clone();
            if last_state == Some((job_id, JobState::Complete)) {
                completed = true;
                break;
            }
            if last_state.is_none() {
                break;
            }
        }
        assert!(
            completed,
            "controlled MP3 import did not publish: state={last_state:?}, message={last_message:?}"
        );

        let destination = root.join("archive");
        let entries = ArchiveStore::scan(&destination).unwrap();
        let entry = entries
            .iter()
            .find(|entry| entry.job_id == job_id && entry.generation == request.generation)
            .unwrap_or_else(|| panic!("MP3 import has no archive entry: {entries:?}"));
        assert!(entry.complete, "MP3 archive entry is incomplete: {entry:?}");
        assert_eq!(
            entry.strategy,
            whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
        );
        assert_eq!(
            ArchiveStore::published_imports_for_strategy(
                &destination,
                &entry.source_sha256,
                whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1,
            )
            .unwrap()
            .len(),
            1
        );
        assert!(
            ArchiveStore::published_imports_for_strategy(
                &destination,
                &entry.source_sha256,
                whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
            )
            .unwrap()
            .is_empty()
        );

        let generation = destination
            .join("transcriptions")
            .join(format!("{:032x}", job_id.0))
            .join(request.generation.get().to_string());
        assert!(generation.join("CURRENT").is_file());
        let decoded_samples = crate::parakeet_audio::PcmPartStore::open(
            generation.join("parakeet-unified/source.wav"),
        )
        .total_samples()
        .unwrap();
        let pending: serde_json::Value =
            serde_json::from_slice(&std::fs::read(generation.join("pending.json")).unwrap())
                .unwrap();
        assert_eq!(pending["source_samples"].as_u64(), Some(decoded_samples));

        let coverage: Vec<whisper_core::ports::ParakeetWindowCoverage> =
            std::fs::read_to_string(generation.join("parakeet-unified/document/coverage.jsonl"))
                .unwrap()
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect();
        assert!(
            !coverage.is_empty(),
            "MP3 coverage must include decoded audio"
        );
        assert_eq!(coverage[0].range.start_sample, 0);
        assert!(
            coverage
                .windows(2)
                .all(|pair| { pair[0].range.end_sample == pair[1].range.start_sample })
        );
        assert_eq!(coverage.last().unwrap().range.end_sample, decoded_samples);

        let internal_txt = std::fs::read(generation.join("transcript.txt")).unwrap();
        let internal_srt = std::fs::read(generation.join("transcript.srt")).unwrap();
        assert!(!internal_txt.is_empty());
        assert!(!internal_srt.is_empty());
        let export_basename = entry.export_basename.as_ref().unwrap();
        assert_eq!(
            std::fs::read(
                destination
                    .join("exports")
                    .join(format!("{export_basename}.txt"))
            )
            .unwrap(),
            internal_txt
        );
        assert_eq!(
            std::fs::read(
                destination
                    .join("exports")
                    .join(format!("{export_basename}.srt"))
            )
            .unwrap(),
            internal_srt
        );

        let source_after = std::fs::read(&fixture).expect("re-read guarded MP3 fixture");
        assert_eq!(
            Sha256::digest(&source_after),
            Sha256::digest(&source_before),
            "the original MP3 source must remain unchanged"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn controlled_bounded_gpu_prepare_uses_identity_without_legacy_session_qualification() {
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-gpu-prepare-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source_path = root.join("source.wav");
        let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_path).unwrap();
        writer.append(&vec![0_i16; 3 * 16_000]).unwrap();
        writer.finish().unwrap();
        let mut request = unified_import_request(&root.join("archive"), &source_path, 799);
        request.config.compute = ComputeChoice::Gpu;
        assert_real_unified_model(&request);
        let mut candidate = fake_gpu_candidate();
        candidate.generation = request.generation.get();
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(controlled_worker().0);
        let mut events = VecDeque::new();

        runtime
            .prepare(request, Some(candidate), &mut events)
            .unwrap();

        assert!(matches!(events.pop_front(), Some(ImportEvent::Prepared(_))));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn controlled_document_drain_reads_one_segment_per_poll_and_preserves_all_pages() {
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-page-drain-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source_path = root.join("source.wav");
        let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_path).unwrap();
        writer.append(&vec![0_i16; 121 * 16_000]).unwrap();
        writer.finish().unwrap();
        let request = unified_import_request(&root.join("archive"), &source_path, 791);
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(controlled_worker().0);
        runtime.run_controlled_unified_import(&request).unwrap();
        let drain = runtime.document_drains.front().unwrap();
        assert_eq!(drain.revision, 2);
        assert_eq!(drain.next_sequence, 1);
        assert!(matches!(
            runtime.poll_unified_document(),
            Some(Ok(ImportEvent::Ready { .. }))
        ));
        for expected in 1..=2 {
            assert!(
                matches!(runtime.poll_unified_document(), Some(Ok(ImportEvent::Segment(segment))) if segment.segment_id.0 == expected)
            );
            assert_eq!(
                runtime.document_drains.front().unwrap().next_sequence,
                expected + 1
            );
        }
        assert!(matches!(
            runtime.poll_unified_document(),
            Some(Ok(ImportEvent::Progress {
                completed_samples,
                total_samples,
                ..
            })) if completed_samples == 121 * 16_000 && total_samples == 121 * 16_000
        ));
        assert!(
            matches!(runtime.poll_unified_document(), Some(Ok(ImportEvent::End { last_sequence: 2, last_offset, .. })) if last_offset == 121 * 16_000)
        );
        assert!(runtime.document_drains.is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn controlled_document_drain_terminates_after_page_read_error() {
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-page-drain-error-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let source_path = root.join("source.wav");
        let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_path).unwrap();
        writer.append(&vec![0_i16; 3 * 16_000]).unwrap();
        writer.finish().unwrap();
        let request = unified_import_request(&root.join("archive"), &source_path, 792);
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(controlled_worker().0);
        runtime.run_controlled_unified_import(&request).unwrap();
        let document_path = root.join(format!(
            "archive/transcriptions/{:032x}/1/parakeet-unified/document",
            request.job_id.0
        ));
        let mut port = RuntimeImportPort {
            runtime,
            events: VecDeque::new(),
            capture_progress: None,
            document_revision: None,
            unified_stop_submissions: None,
            controlled_capture_blocks: None,
        };

        assert!(matches!(port.poll(), Ok(Some(ImportEvent::Ready { .. }))));
        std::fs::remove_dir_all(document_path).unwrap();
        assert!(matches!(
            port.poll(),
            Ok(Some(ImportEvent::Failed { job_id, generation, message }))
                if (job_id, generation) == (request.job_id, request.generation)
                    && message.starts_with("Document:")
        ));
        assert!(port.runtime.document_drains.is_empty());
        assert!(
            matches!(port.poll(), Ok(None)),
            "a terminal page error must be emitted only once"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unified_terminal_failures_release_all_parent_state_and_preserve_source_audio() {
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-failure-recycling-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let child_observations = Arc::new(Mutex::new(Vec::new()));
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_child_observer(Arc::clone(&child_observations));

        for attempt in 0..5_u128 {
            let source_path = root.join(format!("source-{attempt}.wav"));
            let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_path).unwrap();
            writer.append(&vec![1_000_i16; 3 * 16_000]).unwrap();
            writer.finish().unwrap();
            let request =
                unified_import_request(&root.join("archive"), &source_path, 9_400 + attempt);
            runtime.controlled_transcriber = Some(Box::new(FailedUnifiedWorker));
            runtime.start_controlled_unified_import(&request).unwrap();

            let failure_deadline = Instant::now() + Duration::from_secs(5);
            let mut failed = false;
            while Instant::now() < failure_deadline {
                if let Some(Err((job_id, generation, message))) = runtime.poll() {
                    assert_eq!(job_id, request.job_id);
                    assert_eq!(generation, request.generation);
                    assert!(message.contains("controlled worker failure"));
                    failed = true;
                    break;
                }
                thread::sleep(Duration::from_millis(2));
            }
            assert!(
                failed,
                "attempt {attempt} did not reach its terminal failure"
            );
            assert!(runtime.unified_sessions.is_empty());
            assert!(runtime.unified_archives.is_empty());
            assert!(runtime.document_drains.is_empty());
            assert_eq!(runtime.parakeet_scheduler.active_sessions(), 0);
            assert!(runtime.unified_sessions.len() <= 2);
            assert!(runtime.unified_archives.len() <= 2);
            assert!(runtime.document_drains.len() <= 2);
            let work_source = PathBuf::from(&request.destination)
                .join("transcriptions")
                .join(format!("{:032x}", request.job_id.0))
                .join(request.generation.get().to_string())
                .join("parakeet-unified/source.wav");
            assert!(
                work_source.is_file(),
                "recoverable decoded audio was removed"
            );
        }
        let observations = child_observations.lock().unwrap();
        controlled_child_job_bound(&observations).unwrap();
        let commands = observations
            .iter()
            .filter(|item| item.event == "Command")
            .collect::<Vec<_>>();
        assert_eq!(commands.len(), 5);
        assert!(
            commands
                .iter()
                .all(|item| item.retained_job_ids <= UNIFIED_CHILD_JOB_CAPACITY)
        );
        let incarnations = commands
            .iter()
            .map(|item| item.incarnation)
            .collect::<HashSet<_>>();
        assert_eq!(incarnations.len(), 5);
        for command in commands {
            assert!(command.job_id.is_some());
            assert!(
                command
                    .generation
                    .is_some_and(|generation| generation.get() > 0)
            );
            assert_eq!(command.attempt, Some(1));
            assert_eq!(command.wire_request_id, 1);
            let events = observations
                .iter()
                .filter(|item| item.incarnation == command.incarnation)
                .map(|item| item.event)
                .collect::<Vec<_>>();
            assert_eq!(
                events,
                ["Spawn", "Ready", "Command", "Stop", "Exit", "Reaped"]
            );
        }
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn unrecycled_controlled_child_rejects_third_job_with_bounded_table() {
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-child-table-mutant-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let child_observations = Arc::new(Mutex::new(Vec::new()));
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(Box::new(FailedUnifiedWorker))
        .with_unrecycled_child_mutant(Arc::clone(&child_observations));

        for attempt in 0..3_u128 {
            let source_path = root.join(format!("source-{attempt}.wav"));
            let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_path).unwrap();
            writer.append(&vec![1_000_i16; 3 * 16_000]).unwrap();
            writer.finish().unwrap();
            let request =
                unified_import_request(&root.join("archive"), &source_path, 9_440 + attempt);
            runtime.start_controlled_unified_import(&request).unwrap();

            let failure_deadline = Instant::now() + Duration::from_secs(5);
            let mut failed = false;
            while Instant::now() < failure_deadline {
                if let Some(Err((job_id, generation, message))) = runtime.poll() {
                    assert_eq!((job_id, generation), (request.job_id, request.generation));
                    let expected_failure = if attempt < 2 {
                        "controlled worker failure"
                    } else {
                        "ChildSessionCapacity"
                    };
                    assert!(message.contains(expected_failure));
                    failed = true;
                    break;
                }
                thread::sleep(Duration::from_millis(2));
            }
            assert!(
                failed,
                "mutant attempt {attempt} did not terminate in the parent"
            );
            assert!(runtime.unified_sessions.is_empty());
            assert!(runtime.unified_archives.is_empty());
            assert!(runtime.document_drains.is_empty());
            assert_eq!(runtime.parakeet_scheduler.active_sessions(), 0);
            let work_source = PathBuf::from(&request.destination)
                .join("transcriptions")
                .join(format!("{:032x}", request.job_id.0))
                .join(request.generation.get().to_string())
                .join("parakeet-unified/source.wav");
            assert!(
                work_source.is_file(),
                "mutant removed recoverable source audio"
            );
        }

        let observations = child_observations.lock().unwrap().clone();
        let accepted = observations
            .iter()
            .filter(|item| item.event == "Command")
            .collect::<Vec<_>>();
        let rejected = observations
            .iter()
            .filter(|item| item.event == "Rejected")
            .collect::<Vec<_>>();
        assert_eq!(accepted.len(), 2);
        assert_eq!(accepted[0].retained_job_ids, 1);
        assert_eq!(accepted[1].retained_job_ids, 2);
        assert_eq!(rejected.len(), 1);
        assert_eq!(rejected[0].retained_job_ids, 2);
        assert!(
            accepted
                .iter()
                .chain(rejected.iter())
                .all(|item| item.incarnation == accepted[0].incarnation)
        );
        assert_eq!(
            accepted
                .iter()
                .chain(rejected.iter())
                .map(|item| item.wire_request_id)
                .collect::<Vec<_>>(),
            [1, 2, 3],
            "wire request IDs remain positive and monotone inside the unrecycled child"
        );
        assert_eq!(rejected[0].job_id, Some(JobId(9_442)));
        assert!(
            observations
                .iter()
                .all(|item| item.retained_job_ids <= UNIFIED_CHILD_JOB_CAPACITY),
            "the mutant retains the production child admission bound"
        );
        assert!(controlled_child_job_bound(&observations).is_ok());
        assert!(!observations.iter().any(|item| item.event == "Reaped"));

        runtime
            .unified_executor
            .as_mut()
            .unwrap()
            .stop_and_reap()
            .unwrap();
        let observations = child_observations.lock().unwrap();
        assert!(observations.iter().any(|item| item.event == "Stop"));
        assert!(observations.iter().any(|item| item.event == "Exit"));
        assert!(observations.iter().any(|item| item.event == "Reaped"));
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn unified_document_finalization_failure_releases_session_and_allows_replacement() {
        const JOB_ID: u128 = 9_423;
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-finalization-failure-{}",
            std::process::id()
        ));
        let archive_root = root.join("archive");
        std::fs::create_dir_all(&archive_root).unwrap();
        let request = unified_live_request(&archive_root, JOB_ID);
        let session = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: request.job_id,
            generation: request.generation,
        };
        let capture = CaptureStream::empty_test_capture(16_000, 1, "test:finalization-failure");
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(Box::new(TailAwareUnifiedWorker))
        .with_finish_document_failure_for(session);
        runtime
            .start_unified_capture(&request, capture, None, None)
            .unwrap();
        runtime
            .unified_sessions
            .get_mut(&session)
            .unwrap()
            .source
            .enqueue_test_block(vec![0.1; 4 * 16_000]);
        runtime
            .stop_controlled_unified_capture(
                request.job_id,
                request.generation,
                &mut VecDeque::new(),
            )
            .unwrap();

        let deadline = Instant::now() + Duration::from_secs(20);
        let mut terminal_error = None;
        while Instant::now() < deadline {
            if let Some(Err((job_id, generation, message))) = runtime.poll() {
                assert_eq!((job_id, generation), (request.job_id, request.generation));
                terminal_error = Some(message);
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        assert!(
            terminal_error
                .as_deref()
                .is_some_and(|message| message.contains("injected document finalization failure"))
        );
        assert!(
            runtime.poll().is_none(),
            "finalization failure is emitted once"
        );
        assert!(!runtime.unified_sessions.contains_key(&session));
        assert!(!runtime.unified_archives.contains_key(&session));
        assert!(runtime.document_drains.is_empty());
        assert_eq!(runtime.parakeet_scheduler.active_sessions(), 0);
        assert!(
            archive_root
                .join("transcriptions")
                .join(format!("{:032x}", request.job_id.0))
                .join(request.generation.get().to_string())
                .join("parakeet-unified/source.wav")
                .is_file()
        );

        let replacement = unified_live_request(&archive_root, JOB_ID + 1);
        let replacement_session = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: replacement.job_id,
            generation: replacement.generation,
        };
        let mut replacement_capture =
            CaptureStream::empty_test_capture(16_000, 1, "test:finalization-replacement");
        replacement_capture.enqueue_test_block(vec![0.1; 4 * 16_000]);
        runtime.controlled_transcriber = Some(Box::new(TailAwareUnifiedWorker));
        runtime
            .start_unified_capture(&replacement, replacement_capture, None, None)
            .unwrap();
        assert!(runtime.unified_sessions.contains_key(&replacement_session));
        assert_eq!(runtime.unified_sessions.len(), 1);
        assert_eq!(runtime.unified_archives.len(), 1);
        runtime
            .stop_controlled_unified_capture(
                replacement.job_id,
                replacement.generation,
                &mut VecDeque::new(),
            )
            .unwrap();
        let replacement_deadline = Instant::now() + Duration::from_secs(10);
        let mut replacement_ended = false;
        while Instant::now() < replacement_deadline {
            match runtime.poll() {
                Some(Ok(ImportEvent::End {
                    job_id, generation, ..
                })) if (job_id, generation) == (replacement.job_id, replacement.generation) => {
                    replacement_ended = true;
                    break;
                }
                Some(Err((job_id, generation, message)))
                    if (job_id, generation) == (replacement.job_id, replacement.generation) =>
                {
                    panic!("replacement capture failed before terminal End: {message}");
                }
                Some(_) => {}
                None => thread::sleep(Duration::from_millis(2)),
            }
        }
        assert!(replacement_ended, "replacement capture did not reach End");
        assert!(!runtime.unified_sessions.contains_key(&replacement_session));
        assert!(runtime.document_drains.is_empty());
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn capture_identity_does_not_mask_existing_import_source_binding() {
        const JOB_ID: u128 = 9_426;
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-capture-binding-collision-{}",
            std::process::id()
        ));
        let archive_root = root.join("archive");
        let settings_root = root.join("settings");
        std::fs::create_dir_all(&archive_root).unwrap();
        let source_path = root.join("bound-import.wav");
        let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_path).unwrap();
        writer.append(&vec![1_000_i16; 16_000]).unwrap();
        writer.finish().unwrap();

        let mut import_request = unified_import_request(&archive_root, &source_path, JOB_ID);
        let live_request = unified_live_request(&archive_root, JOB_ID);
        assert_eq!(import_request.generation, live_request.generation);
        let session = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: live_request.job_id,
            generation: live_request.generation,
        };
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(Box::new(TailAwareUnifiedWorker));
        runtime.snapshot_registry_root = Some(settings_root.clone());
        runtime.ensure_source_binding(&mut import_request).unwrap();
        assert!(
            runtime
                .source_bindings
                .contains_key(&(JobId(JOB_ID), live_request.generation.get()))
        );

        let mut capture = CaptureStream::empty_test_capture(16_000, 1, "test:binding-collision");
        capture.enqueue_test_block(vec![0.1; 16_000]);
        runtime
            .start_unified_capture(&live_request, capture, None, None)
            .unwrap();
        assert!(runtime.unified_capture_sessions.contains(&session));

        runtime
            .release_source_binding(JobId(JOB_ID), live_request.generation, &archive_root)
            .unwrap();
        assert!(
            !runtime
                .source_bindings
                .contains_key(&(JobId(JOB_ID), live_request.generation.get()))
        );
        assert!(
            crate::source_binding::SourceBinding::inspect_recovery_candidates(&settings_root, &[],)
                .unwrap()
                .is_empty()
        );
        let snapshot_dir = archive_root.join(".whisper-source-snapshots");
        let stem = format!("{:032x}-{}", JOB_ID, live_request.generation.get());
        assert!(!snapshot_dir.join(format!("{stem}.json")).exists());
        assert!(!snapshot_dir.join(format!("{stem}.snapshot")).exists());

        runtime
            .stop_controlled_unified_capture(
                live_request.job_id,
                live_request.generation,
                &mut VecDeque::new(),
            )
            .unwrap();
        drop(runtime);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn capture_with_empty_snapshot_registry_publishes_complete_source_without_registry_state() {
        const JOB_ID: u128 = 9_427;
        const CAPTURE_SAMPLES: u64 = 4 * 16_000;
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-empty-snapshot-registry-{}",
            std::process::id()
        ));
        let archive_root = root.join("archive");
        let settings_root = root.join("settings");
        std::fs::create_dir_all(&archive_root).unwrap();
        std::fs::create_dir_all(&settings_root).unwrap();
        assert_eq!(std::fs::read_dir(&settings_root).unwrap().count(), 0);
        let request = unified_live_request(&archive_root, JOB_ID);
        let mut capture = CaptureStream::empty_test_capture(16_000, 1, "test:empty-registry");
        capture.enqueue_test_block(vec![0.1; CAPTURE_SAMPLES as usize]);
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(Box::new(TailAwareUnifiedWorker))
        .with_controlled_capture(capture, None);
        runtime.snapshot_registry_root = Some(settings_root.clone());
        let mut app = ImportApplication::new(RuntimeImportPort {
            runtime,
            events: VecDeque::new(),
            capture_progress: None,
            document_revision: None,
            unified_stop_submissions: None,
            controlled_capture_blocks: None,
        });
        app.dispatch(AppCommand::StartUnifiedLiveConfigured {
            request: request.clone(),
        })
        .unwrap();

        let capture_deadline = Instant::now() + Duration::from_secs(20);
        let mut pre_stop_progress = None;
        while Instant::now() < capture_deadline {
            app.dispatch(AppCommand::Refresh).unwrap();
            let view = app.view();
            pre_stop_progress = view
                .unified_parakeet_sessions
                .iter()
                .find(|session| {
                    session.job_id == request.job_id && session.generation == request.generation
                })
                .map(|session| (session.captured_samples, session.durable_samples));
            if pre_stop_progress.is_some_and(|(captured, durable)| captured > 0 && durable > 0) {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        let (pre_stop_captured, pre_stop_durable) = pre_stop_progress
            .expect("capture must expose written and durable progress before Stop");
        assert!(pre_stop_captured > 0 && pre_stop_durable > 0);
        assert!(pre_stop_captured <= CAPTURE_SAMPLES && pre_stop_durable <= CAPTURE_SAMPLES);
        app.dispatch(AppCommand::StopUnifiedLive {
            job_id: request.job_id,
            generation: request.generation,
        })
        .unwrap();

        let publication_deadline = Instant::now() + Duration::from_secs(20);
        let mut app_complete = false;
        while Instant::now() < publication_deadline {
            app.dispatch(AppCommand::Refresh).unwrap();
            let archive_complete = ArchiveStore::scan(&archive_root)
                .unwrap()
                .iter()
                .any(|entry| entry.job_id == request.job_id && entry.complete);
            let view = app.view();
            let terminal_session = view.unified_parakeet_sessions.iter().find(|session| {
                session.job_id == request.job_id && session.generation == request.generation
            });
            if let Some(session) = terminal_session {
                if session.phase == whisper_core::ports::UnifiedSessionPhase::Recoverable {
                    panic!(
                        "capture reached Recoverable after archive completion={archive_complete}: {:?}; message={:?}",
                        session, view.message
                    );
                }
                if session.phase == whisper_core::ports::UnifiedSessionPhase::Complete {
                    app_complete = true;
                    break;
                }
            } else if archive_complete {
                panic!(
                    "archive is Complete but the application lost the capture identity before delivering its terminal state: {:?}",
                    view.message
                );
            }
            thread::sleep(Duration::from_millis(2));
        }
        assert!(
            app_complete,
            "application must deliver Complete for the exact capture identity after archive publication"
        );
        let complete_view = app.view();
        let completed_session = complete_view
            .unified_parakeet_sessions
            .iter()
            .find(|session| {
                session.job_id == request.job_id && session.generation == request.generation
            })
            .expect("completed capture remains visible in the application view");
        assert_eq!(
            completed_session.phase,
            whisper_core::ports::UnifiedSessionPhase::Complete
        );
        assert_eq!(completed_session.total_samples, Some(CAPTURE_SAMPLES));
        assert_eq!(completed_session.captured_samples, CAPTURE_SAMPLES);
        assert_eq!(completed_session.durable_samples, CAPTURE_SAMPLES);
        let archive_complete = ArchiveStore::scan(&archive_root)
            .unwrap()
            .iter()
            .any(|entry| entry.job_id == request.job_id && entry.complete);
        assert!(
            archive_complete,
            "capture must reach a real Complete archive"
        );
        let files = ArchiveStore::published_unified_files(
            &archive_root,
            request.job_id,
            request.generation,
        )
        .expect("Complete capture must resolve its CURRENT artifacts");
        let generation_dir = archive_root
            .join("transcriptions")
            .join(format!("{:032x}", request.job_id.0))
            .join(request.generation.get().to_string());
        assert_eq!(
            std::fs::read_to_string(generation_dir.join("CURRENT")).unwrap(),
            "manifest.json\n"
        );
        let source = crate::parakeet_audio::PcmPartStore::open(&files.wav_path);
        assert_eq!(source.total_samples().unwrap(), CAPTURE_SAMPLES);
        assert_eq!(
            source.read_window(0..CAPTURE_SAMPLES).unwrap().len() as u64,
            CAPTURE_SAMPLES,
            "the published PCM source must be readable through its final sample"
        );
        let source_manifest: serde_json::Value = serde_json::from_slice(
            &std::fs::read(generation_dir.join("captured-source-parts.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            source_manifest["source_samples"].as_u64(),
            Some(CAPTURE_SAMPLES)
        );
        assert!(!settings_root.join("reprocessing-snapshots.lock").exists());
        assert!(!settings_root.join("reprocessing-snapshots.json").exists());
        assert_eq!(std::fs::read_dir(&settings_root).unwrap().count(), 0);
        drop(app);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn capture_identity_with_orphaned_snapshot_artifact_fails_closed() {
        const JOB_ID: u128 = 9_428;
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-orphaned-snapshot-artifact-{}",
            std::process::id()
        ));
        let archive_root = root.join("archive");
        let settings_root = root.join("settings");
        let snapshot_dir = archive_root.join(".whisper-source-snapshots");
        std::fs::create_dir_all(&snapshot_dir).unwrap();
        let request = unified_live_request(&archive_root, JOB_ID);
        let stem = format!("{:032x}-{}", JOB_ID, request.generation.get());
        let orphan = snapshot_dir.join(format!("{stem}.json"));
        std::fs::write(&orphan, b"orphaned binding record").unwrap();
        let capture = CaptureStream::empty_test_capture(16_000, 1, "test:orphaned-binding");
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(Box::new(TailAwareUnifiedWorker));
        runtime.snapshot_registry_root = Some(settings_root.clone());
        runtime
            .start_unified_capture(&request, capture, None, None)
            .unwrap();

        let error = runtime
            .release_source_binding(request.job_id, request.generation, &archive_root)
            .unwrap_err();
        assert!(
            error.contains("SourceSnapshotRegistryUnavailable"),
            "{error}"
        );
        assert!(error.contains("durable snapshot artifacts"), "{error}");
        assert!(
            error.contains(&settings_root.display().to_string()),
            "{error}"
        );
        assert!(
            orphan.is_file(),
            "fail-closed handling must preserve the orphan"
        );
        assert!(runtime.unified_sessions.contains_key(
            &whisper_core::parakeet_scheduler::SchedulerSession {
                job_id: request.job_id,
                generation: request.generation,
            }
        ));
        drop(runtime);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn capture_identity_does_not_mask_corrupt_snapshot_registry() {
        const JOB_ID: u128 = 9_429;
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-corrupt-snapshot-registry-{}",
            std::process::id()
        ));
        let archive_root = root.join("archive");
        let settings_root = root.join("settings");
        std::fs::create_dir_all(&archive_root).unwrap();
        std::fs::create_dir_all(&settings_root).unwrap();
        std::fs::write(settings_root.join("reprocessing-snapshots.lock"), b"").unwrap();
        let registry_state = settings_root.join("reprocessing-snapshots.json");
        std::fs::write(&registry_state, b"not valid registry JSON").unwrap();
        let request = unified_live_request(&archive_root, JOB_ID);
        let capture = CaptureStream::empty_test_capture(16_000, 1, "test:corrupt-registry");
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(Box::new(TailAwareUnifiedWorker));
        runtime.snapshot_registry_root = Some(settings_root);
        runtime
            .start_unified_capture(&request, capture, None, None)
            .unwrap();

        let error = runtime
            .release_source_binding(request.job_id, request.generation, &archive_root)
            .unwrap_err();
        assert!(error.contains("SourceSnapshotRegistryCorrupt"), "{error}");
        assert_eq!(
            std::fs::read(&registry_state).unwrap(),
            b"not valid registry JSON"
        );
        assert!(runtime.unified_capture_sessions.contains(
            &whisper_core::parakeet_scheduler::SchedulerSession {
                job_id: request.job_id,
                generation: request.generation,
            }
        ));
        drop(runtime);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn unified_runtime_keeps_reserved_final_turn_through_preparation_with_peer_preview_ready() {
        const A_ID: u128 = 9_424;
        const B_ID: u128 = 9_425;
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-reserved-final-preview-{}",
            std::process::id()
        ));
        let archive_root = root.join("archive");
        std::fs::create_dir_all(&archive_root).unwrap();
        let source_a = root.join("source-a.wav");
        let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_a).unwrap();
        writer.append(&vec![10_000_i16; 121 * 16_000]).unwrap();
        writer.finish().unwrap();
        let source_b = root.join("source-b.wav");
        let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_b).unwrap();
        writer.append(&vec![0_i16; 5 * 16_000]).unwrap();
        writer.finish().unwrap();
        let request_a = unified_import_request(&archive_root, &source_a, A_ID);
        let request_b = unified_import_request(&archive_root, &source_b, B_ID);
        let session_a = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: request_a.job_id,
            generation: request_a.generation,
        };
        let session_b = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: request_b.job_id,
            generation: request_b.generation,
        };
        let (worker, calls, maximum_active, call_sources, held_rx, release_tx) = held_worker(1);
        let child_observations = Arc::new(Mutex::new(Vec::new()));
        let native_paths = crate::parakeet_import::ParakeetWorkerPaths {
            executable: root.join("controlled-native-worker.exe"),
            native_dir: root.join("controlled-native"),
            model: root.join("controlled-model.nemo"),
            runtime_root: root.clone(),
        };
        let native_route = NativeUnifiedRoute {
            paths: native_paths,
            requested_compute: ComputeChoice::Cpu,
            compute: ComputeChoice::Cpu,
            expected_gpu: None,
            candidate: None,
            fallback_reason: None,
        };
        let mut runtime = ImportRuntime::new_with_workers_and_parakeet_registry(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
            None,
            None,
            None,
            None,
            false,
            false,
        )
        .with_controlled_transcriber(worker)
        .with_child_observer(Arc::clone(&child_observations));
        prepare_and_start_controlled_native_route(&mut runtime, &request_a, native_route.clone());
        prepare_and_start_controlled_native_route(&mut runtime, &request_b, native_route);
        runtime.native_unified_candidate_enabled = true;
        let ready_gate = runtime.hold_next_child_ready();
        let mut ready_labels = HashMap::new();

        assert!(runtime.poll_unified_sessions().is_none());
        let reserved = runtime.parakeet_scheduler.in_flight().unwrap();
        assert_eq!(reserved.session, session_a);
        assert!(matches!(
            reserved.work,
            whisper_core::parakeet_scheduler::ScheduledWork::Final { .. }
        ));
        assert_eq!(calls.load(Ordering::Relaxed), 0);
        let retained_final = runtime.unified_sessions[&session_a]
            .pending_prepared
            .clone()
            .expect("the reserved final clip is prepared before READY");
        assert_eq!(retained_final.0, reserved);
        assert!(retained_final.3);
        assert!(
            !child_observations
                .lock()
                .unwrap()
                .iter()
                .any(|item| item.event == "Ready")
        );

        runtime
            .parakeet_scheduler
            .queue_preview(session_b, 5 * 16_000)
            .unwrap();
        assert!(runtime.poll_unified_sessions().is_none());
        assert_eq!(runtime.parakeet_scheduler.in_flight(), Some(reserved));
        assert!(
            runtime.unified_sessions[&session_a]
                .pending_prepared
                .is_some()
        );
        assert_eq!(calls.load(Ordering::Relaxed), 0);
        let (ready_released, ready_changed) = &*ready_gate;
        *ready_released.lock().unwrap() = true;
        ready_changed.notify_all();
        let ready_deadline = Instant::now() + Duration::from_secs(5);
        let mut ready_seen = false;
        let mut a_ready_events = 0;
        let mut held_source = None;
        while Instant::now() < ready_deadline && held_source.is_none() {
            match runtime.poll_unified_sessions() {
                Some(Err((job_id, generation, message))) => panic!(
                    "reserved turn failed after READY release for {job_id:?}/{generation:?}: {message}"
                ),
                Some(Ok(ImportEvent::Ready {
                    job_id,
                    generation,
                    backend,
                })) if (job_id, generation) == (session_a.job_id, session_a.generation) => {
                    a_ready_events += 1;
                    ready_labels
                        .entry((job_id, generation))
                        .or_insert_with(Vec::new)
                        .push(backend);
                    assert_eq!(
                        runtime.unified_sessions[&session_a].pending_prepared,
                        Some(retained_final.clone()),
                        "READY must retain the exact prepared final clip"
                    );
                }
                _ => {}
            }
            ready_seen |= child_observations
                .lock()
                .unwrap()
                .iter()
                .any(|item| item.event == "Ready");
            match held_rx.try_recv() {
                Ok(source) => held_source = Some(source),
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    thread::sleep(Duration::from_millis(2));
                }
                Err(error) => panic!("held worker exited after READY release: {error}"),
            }
        }
        assert!(
            ready_seen,
            "controlled child READY was not observed after release"
        );
        assert_eq!(
            a_ready_events, 1,
            "backend READY is reported exactly once per session"
        );
        assert_eq!(held_source.as_deref(), Some("A"));
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        assert!(
            runtime.unified_sessions[&session_a]
                .pending_prepared
                .is_none()
        );
        assert!(runtime.poll_unified_sessions().is_none());
        assert_eq!(runtime.parakeet_scheduler.in_flight(), Some(reserved));
        assert_eq!(calls.load(Ordering::Relaxed), 1);

        release_tx.send(()).unwrap();
        let peer_preview_deadline = Instant::now() + Duration::from_secs(10);
        let mut b_preview_ready_events = 0;
        let mut retained_preview = None;
        while Instant::now() < peer_preview_deadline && calls.load(Ordering::Relaxed) < 2 {
            match runtime.poll_unified_sessions() {
                Some(Err((job_id, generation, message))) => panic!(
                    "peer preview failed after final A for {job_id:?}/{generation:?}: {message}"
                ),
                Some(Ok(ImportEvent::Ready {
                    job_id,
                    generation,
                    backend,
                })) if (job_id, generation) == (session_b.job_id, session_b.generation) => {
                    b_preview_ready_events += 1;
                    ready_labels
                        .entry((job_id, generation))
                        .or_insert_with(Vec::new)
                        .push(backend);
                    let prepared = runtime.unified_sessions[&session_b]
                        .pending_prepared
                        .clone()
                        .expect("READY must retain the prepared peer preview");
                    assert!(!prepared.3, "the retained peer clip must remain a preview");
                    assert!(matches!(
                        prepared.0.work,
                        whisper_core::parakeet_scheduler::ScheduledWork::Preview { .. }
                    ));
                    assert_eq!(runtime.parakeet_scheduler.in_flight(), Some(prepared.0));
                    retained_preview = Some(prepared);
                }
                _ => {}
            }
            thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(
            b_preview_ready_events, 1,
            "peer backend READY is reported exactly once"
        );
        let retained_preview = retained_preview.expect("the prepared preview survives READY");
        assert_eq!(
            runtime.parakeet_scheduler.in_flight(),
            Some(retained_preview.0)
        );
        assert!(
            runtime.unified_sessions[&session_b]
                .pending_prepared
                .is_none()
        );
        let call_log = call_sources.lock().unwrap();
        assert!(call_log.len() >= 2);
        assert_eq!(call_log[0], "A:final");
        assert_eq!(call_log[1], "B:preview");
        assert_eq!(maximum_active.load(Ordering::Relaxed), 1);
        drop(call_log);

        let mut persisted_sequences = HashMap::new();
        let mut published_sessions = HashSet::new();
        let mut completed_sessions = HashSet::new();
        let recycled_ready_gate = Arc::new((Mutex::new(false), std::sync::Condvar::new()));
        runtime.controlled_ready_gate = Some(Arc::clone(&recycled_ready_gate));
        let ready_deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < ready_deadline {
            match runtime.poll_unified_sessions() {
                Some(Err((job_id, generation, message))) => {
                    panic!("reserved turn session {job_id:?}/{generation:?} failed: {message}");
                }
                Some(Ok(ImportEvent::Ready {
                    job_id,
                    generation,
                    backend,
                })) => {
                    ready_labels
                        .entry((job_id, generation))
                        .or_insert_with(Vec::new)
                        .push(backend);
                }
                _ => {}
            }
            let observations = child_observations.lock().unwrap();
            let recycled_spawned = observations
                .iter()
                .filter(|item| item.event == "Spawn")
                .count()
                >= 2;
            drop(observations);
            if recycled_spawned
                && runtime
                    .unified_sessions
                    .get(&session_a)
                    .is_some_and(|state| state.pending_prepared.is_some())
            {
                break;
            }
            if let Some(event) = runtime.poll_unified_document() {
                match event {
                    Ok(event) => {
                        if let ImportEvent::End {
                            job_id, generation, ..
                        } = &event
                        {
                            completed_sessions.insert((*job_id, *generation));
                        }
                        persist_and_publish_controlled_document_event(
                            &mut runtime,
                            event,
                            &mut persisted_sequences,
                            &mut published_sessions,
                            &mut ready_labels,
                        );
                    }
                    Err((job_id, generation, message)) => {
                        panic!("document drain failed for {job_id:?}/{generation:?}: {message}")
                    }
                }
            }
            thread::sleep(Duration::from_millis(2));
        }
        let reserved_after_recycle = runtime.parakeet_scheduler.in_flight().unwrap();
        assert_eq!(reserved_after_recycle.session, session_a);
        assert!(matches!(
            reserved_after_recycle.work,
            whisper_core::parakeet_scheduler::ScheduledWork::Final { .. }
        ));
        assert!(
            runtime.unified_sessions[&session_a]
                .pending_prepared
                .is_some()
        );
        assert_eq!(calls.load(Ordering::Relaxed), 3);
        {
            let observations = child_observations.lock().unwrap();
            let second_incarnation = observations
                .iter()
                .filter(|item| item.event == "Spawn")
                .nth(1)
                .expect("recycled child spawned")
                .incarnation;
            assert!(
                !observations.iter().any(|item| {
                    item.event == "Ready" && item.incarnation == second_incarnation
                })
            );
        }
        {
            let (released, changed) = &*recycled_ready_gate;
            *released.lock().unwrap() = true;
            changed.notify_all();
        }

        let drain_deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < drain_deadline
            && (!runtime.unified_sessions.is_empty() || !runtime.document_drains.is_empty())
        {
            match runtime.poll_unified_sessions() {
                Some(Err((job_id, generation, message))) => {
                    panic!("reserved turn session {job_id:?}/{generation:?} failed: {message}");
                }
                Some(Ok(ImportEvent::Ready {
                    job_id,
                    generation,
                    backend,
                })) => {
                    ready_labels
                        .entry((job_id, generation))
                        .or_insert_with(Vec::new)
                        .push(backend);
                }
                _ => {}
            }
            if let Some(event) = runtime.poll_unified_document() {
                match event {
                    Ok(event) => {
                        if let ImportEvent::End {
                            job_id, generation, ..
                        } = &event
                        {
                            completed_sessions.insert((*job_id, *generation));
                        }
                        persist_and_publish_controlled_document_event(
                            &mut runtime,
                            event,
                            &mut persisted_sequences,
                            &mut published_sessions,
                            &mut ready_labels,
                        );
                    }
                    Err((job_id, generation, message)) => {
                        panic!("document drain failed for {job_id:?}/{generation:?}: {message}")
                    }
                }
            }
            thread::sleep(Duration::from_millis(2));
        }
        assert!(runtime.unified_sessions.is_empty());
        assert!(runtime.document_drains.is_empty());
        assert_eq!(runtime.parakeet_scheduler.active_sessions(), 0);
        assert!(completed_sessions.contains(&(session_a.job_id, session_a.generation)));
        assert!(completed_sessions.contains(&(session_b.job_id, session_b.generation)));
        assert!(published_sessions.contains(&(session_a.job_id, session_a.generation)));
        assert!(published_sessions.contains(&(session_b.job_id, session_b.generation)));
        assert_eq!(ready_labels.len(), 2, "no other session may receive READY");
        for session in [session_a, session_b] {
            assert_eq!(
                ready_labels.get(&(session.job_id, session.generation)),
                Some(&vec!["CPU".to_owned()]),
                "each synthetic native CPU route reports one truthful READY through publication"
            );
        }
        let archives = ArchiveStore::scan(&archive_root).unwrap();
        for (session, expected_samples) in
            [(session_a, 121 * 16_000_u64), (session_b, 5 * 16_000_u64)]
        {
            let archive = archives
                .iter()
                .find(|entry| {
                    entry.job_id == session.job_id && entry.generation == session.generation
                })
                .expect("completed controlled session must remain in archive history");
            assert!(
                archive.complete,
                "document completion must publish a complete archive"
            );
            let files = ArchiveStore::published_unified_files(
                &archive_root,
                session.job_id,
                session.generation,
            )
            .expect("Published must resolve the generation-bound CURRENT artifacts");
            for path in [&files.wav_path, &files.txt_path, &files.srt_path] {
                assert!(
                    Path::new(path).is_file(),
                    "published artifact missing: {path}"
                );
            }
            let source_path = archive_root
                .join("transcriptions")
                .join(format!("{:032x}", session.job_id.0))
                .join(session.generation.get().to_string())
                .join("parakeet-unified/source.wav");
            assert_eq!(
                crate::parakeet_audio::PcmPartStore::open(&source_path)
                    .total_samples()
                    .unwrap(),
                expected_samples,
                "completed coverage must include the whole source"
            );
        }
        let observations = child_observations.lock().unwrap();
        controlled_child_job_bound(&observations).unwrap();
        let spawns = observations
            .iter()
            .filter(|item| item.event == "Spawn")
            .map(|item| item.incarnation)
            .collect::<Vec<_>>();
        assert_eq!(
            spawns.len(),
            2,
            "terminal B must be reaped before A resumes"
        );
        for incarnation in &spawns {
            let events = observations
                .iter()
                .filter(|item| item.incarnation == *incarnation)
                .map(|item| item.event)
                .collect::<Vec<_>>();
            assert_eq!(events.first(), Some(&"Spawn"));
            assert_eq!(events.get(1), Some(&"Ready"));
            assert!(events.iter().position(|event| *event == "Reaped").is_some());
        }
        let reaped_first = observations
            .iter()
            .position(|item| item.event == "Reaped" && item.incarnation == spawns[0])
            .unwrap();
        let spawned_second = observations
            .iter()
            .position(|item| item.event == "Spawn" && item.incarnation == spawns[1])
            .unwrap();
        assert!(reaped_first < spawned_second);
        drop(observations);
        let final_order = call_sources
            .lock()
            .unwrap()
            .iter()
            .filter(|entry| entry.ends_with(":final"))
            .cloned()
            .collect::<Vec<_>>();
        assert_eq!(
            final_order,
            ["A:final", "B:final", "A:final"],
            "final turns must retain their order across child recycle"
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn unified_child_retains_only_two_job_ids_until_real_worker_recycle() {
        use whisper_core::parakeet_scheduler::{ScheduledCall, ScheduledWork, SchedulerSession};

        let (worker, calls) = controlled_worker();
        let mut executor = UnifiedInferenceExecutor::start(worker).unwrap();
        let command_for = |job_id| UnifiedInferenceCommand {
            call: ScheduledCall {
                session: SchedulerSession {
                    job_id: JobId(job_id),
                    generation: Generation::first(),
                },
                incarnation: 1,
                request_id: 1,
                attempt: 0,
                work: ScheduledWork::Final { sequence: 1 },
            },
            window: whisper_core::parakeet_session::AudioWindow {
                sequence: 1,
                central_start_sample: 0,
                central_end_sample: 1,
                native_start_sample: 0,
                native_end_sample: 1,
            },
            pcm: vec![0],
            scratch_directory: PathBuf::from("."),
        };
        let await_result = |executor: &mut UnifiedInferenceExecutor| {
            let deadline = Instant::now() + Duration::from_secs(5);
            loop {
                if let Some(result) = executor.try_result().unwrap() {
                    break result;
                }
                assert!(
                    Instant::now() < deadline,
                    "controlled child result timed out"
                );
                thread::sleep(Duration::from_millis(2));
            }
        };

        for job_id in [9_428_u128, 9_429] {
            executor.dispatch(command_for(job_id)).unwrap();
            let result = await_result(&mut executor);
            assert_eq!(result.call.session.job_id, JobId(job_id));
            assert!(result.words.is_ok());
        }
        assert_eq!(calls.load(Ordering::Relaxed), 2);

        executor.dispatch(command_for(9_430)).unwrap();
        let rejected = await_result(&mut executor);
        assert_eq!(rejected.call.session.job_id, JobId(9_430));
        assert!(rejected.words.unwrap_err().contains("ChildSessionCapacity"));
        assert_eq!(calls.load(Ordering::Relaxed), 2);

        executor.stop_and_reap().unwrap();
        executor.restart().unwrap();
        executor.dispatch(command_for(9_430)).unwrap();
        let recycled = await_result(&mut executor);
        assert!(recycled.words.is_ok());
        assert_eq!(calls.load(Ordering::Relaxed), 3);
        executor.stop_and_reap().unwrap();
    }

    #[test]
    fn panicking_unified_worker_terminates_the_inflight_session_and_its_peer() {
        const A_ID: u128 = 9_426;
        const B_ID: u128 = 9_427;
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-worker-panic-{}",
            std::process::id()
        ));
        let archive_root = root.join("archive");
        std::fs::create_dir_all(&archive_root).unwrap();
        let source_a = root.join("source-a.wav");
        let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_a).unwrap();
        writer.append(&vec![10_000_i16; 121 * 16_000]).unwrap();
        writer.finish().unwrap();
        let source_b = root.join("source-b.wav");
        let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_b).unwrap();
        writer.append(&vec![0_i16; 5 * 16_000]).unwrap();
        writer.finish().unwrap();
        let request_a = unified_import_request(&archive_root, &source_a, A_ID);
        let request_b = unified_import_request(&archive_root, &source_b, B_ID);
        let session_a = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: request_a.job_id,
            generation: request_a.generation,
        };
        let session_b = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: request_b.job_id,
            generation: request_b.generation,
        };
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(Box::new(TailAwareUnifiedWorker))
        .with_panicking_transcriber_for(session_b);
        runtime.start_controlled_unified_import(&request_a).unwrap();
        runtime.start_controlled_unified_import(&request_b).unwrap();

        let deadline = Instant::now() + Duration::from_secs(20);
        let mut failed_b = false;
        while Instant::now() < deadline {
            if let Some(Err((job_id, generation, message))) = runtime.poll_unified_sessions() {
                assert_eq!(
                    (job_id, generation),
                    (request_b.job_id, request_b.generation)
                );
                assert!(message.contains("InferenceReap: disconnected worker thread panicked"));
                failed_b = true;
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        assert!(failed_b, "the injected worker panic did not terminate B");
        assert!(matches!(
            runtime.poll(),
            Some(Ok(ImportEvent::Failed { job_id, generation, message }))
                if job_id == request_a.job_id
                    && generation == request_a.generation
                    && message.contains("peer session terminated")
        ));
        assert!(!runtime.unified_sessions.contains_key(&session_a));
        assert!(!runtime.unified_sessions.contains_key(&session_b));
        assert!(!runtime.unified_archives.contains_key(&session_a));
        assert!(!runtime.unified_archives.contains_key(&session_b));
        assert_eq!(runtime.parakeet_scheduler.active_sessions(), 0);
        assert!(runtime.parakeet_scheduler.in_flight().is_none());
        assert!(runtime.unified_executor.is_none());
        assert!(
            runtime
                .start_controlled_unified_import(&unified_import_request(
                    &archive_root,
                    &source_a,
                    A_ID + 1,
                ))
                .is_err()
        );
        for request in [&request_a, &request_b] {
            assert!(
                archive_root
                    .join("transcriptions")
                    .join(format!("{:032x}", request.job_id.0))
                    .join(request.generation.get().to_string())
                    .join("parakeet-unified/source.wav")
                    .is_file()
            );
        }
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn unified_document_drains_are_included_in_the_two_session_admission_bound() {
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-drain-capacity-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let (worker, _) = controlled_worker();
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(worker);
        let mut requests = Vec::new();
        for id in [9_410_u128, 9_411] {
            let source_path = root.join(format!("source-{id}.wav"));
            let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_path).unwrap();
            writer.append(&vec![0_i16; 3 * 16_000]).unwrap();
            writer.finish().unwrap();
            let request = unified_import_request(&root.join("archive"), &source_path, id);
            runtime.start_controlled_unified_import(&request).unwrap();
            requests.push(request);
        }

        let drain_deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < drain_deadline
            && (runtime.unified_sessions.len() != 0 || runtime.document_drains.len() != 2)
        {
            if let Some(Err((job_id, generation, message))) = runtime.poll_unified_sessions() {
                panic!("session {job_id:?}/{generation:?} failed before document drain: {message}");
            }
            assert!(runtime.unified_sessions.len() <= 2);
            assert!(runtime.unified_archives.len() <= 2);
            assert!(runtime.document_drains.len() <= 2);
            thread::sleep(Duration::from_millis(2));
        }
        assert!(runtime.unified_sessions.is_empty());
        assert_eq!(runtime.unified_archives.len(), 2);
        assert_eq!(runtime.document_drains.len(), 2);
        // The drain queue itself participates in admission even if an archive
        // handle has already been released by a terminal path.
        runtime.unified_archives.clear();

        let third_source = root.join("source-third.wav");
        let mut writer = crate::parakeet_audio::PcmPartWriter::create(&third_source).unwrap();
        writer.append(&vec![0_i16; 3 * 16_000]).unwrap();
        writer.finish().unwrap();
        let third = unified_import_request(&root.join("archive"), &third_source, 9_412);
        assert!(runtime.start_controlled_unified_import(&third).is_err());
        assert!(runtime.unified_archives.is_empty());
        assert_eq!(runtime.document_drains.len(), 2);
        assert!(
            !root
                .join("archive/transcriptions")
                .join(format!("{:032x}", third.job_id.0))
                .exists()
        );
        assert_eq!(requests.len(), 2);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn disconnected_unified_inference_result_fails_the_inflight_session_and_preserves_its_peer() {
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-disconnect-correlation-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        let session_b = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: JobId(9_421),
            generation: Generation::first(),
        };
        let (worker, calls) = controlled_worker();
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(worker)
        .with_disconnected_result_for(session_b);

        let source_a = root.join("source-a.wav");
        let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_a).unwrap();
        writer.append(&vec![0_i16; 121 * 16_000]).unwrap();
        writer.finish().unwrap();
        let request_a = unified_import_request(&root.join("archive"), &source_a, 9_420);
        let source_b = root.join("source-b.wav");
        let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_b).unwrap();
        writer.append(&vec![0_i16; 3 * 16_000]).unwrap();
        writer.finish().unwrap();
        let request_b =
            unified_import_request(&root.join("archive"), &source_b, session_b.job_id.0);
        runtime.start_controlled_unified_import(&request_a).unwrap();
        runtime.start_controlled_unified_import(&request_b).unwrap();

        let failure_deadline = Instant::now() + Duration::from_secs(20);
        let mut failed_b = false;
        while Instant::now() < failure_deadline {
            if let Some(Err((job_id, generation, message))) = runtime.poll() {
                assert_eq!(job_id, session_b.job_id);
                assert_eq!(generation, session_b.generation);
                assert!(message.contains("result channel disconnected"));
                failed_b = true;
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        assert!(
            failed_b,
            "the injected B result-channel disconnect was not observed"
        );
        let session_a = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: request_a.job_id,
            generation: request_a.generation,
        };
        assert!(runtime.unified_sessions.contains_key(&session_a));
        assert!(!runtime.unified_sessions.contains_key(&session_b));
        assert!(runtime.unified_archives.contains_key(&session_a));
        assert!(!runtime.unified_archives.contains_key(&session_b));
        assert_eq!(runtime.parakeet_scheduler.in_flight(), None);
        assert_eq!(runtime.parakeet_scheduler.active_sessions(), 1);
        assert!(runtime.unified_executor.as_ref().unwrap().thread.is_some());

        let peer_deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < peer_deadline && runtime.unified_sessions.contains_key(&session_a) {
            if let Some(Err((job_id, generation, message))) = runtime.poll() {
                panic!("peer A failed after B disconnect: {job_id:?}/{generation:?}: {message}");
            }
            thread::sleep(Duration::from_millis(2));
        }
        assert!(!runtime.unified_sessions.contains_key(&session_a));
        assert!(
            runtime
                .document_drains
                .iter()
                .any(|drain| (drain.job_id, drain.generation)
                    == (request_a.job_id, request_a.generation))
        );
        assert!(calls.load(Ordering::Relaxed) >= 3);
        assert!(runtime.unified_sessions.len() <= 2);
        assert!(runtime.unified_archives.len() <= 2);
        assert!(runtime.document_drains.len() <= 2);
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn unified_capture_gain_changes_apply_to_the_next_written_pcm_block_and_reject_stale_targets() {
        const JOB_ID: u128 = 9_322;
        let root =
            std::env::temp_dir().join(format!("parakeet-unified-live-gain-{}", std::process::id()));
        let archive_root = root.join("archive");
        std::fs::create_dir_all(&archive_root).unwrap();
        let mut request = unified_live_request(&archive_root, JOB_ID);
        request.microphone = whisper_core::MicrophoneChoice::Device("test:unified-gain".into());
        let mut capture = CaptureStream::empty_test_capture_with_gain(
            16_000,
            1,
            "test:unified-gain",
            whisper_core::microphone_level::GainDb::NEUTRAL,
        );
        capture.enqueue_test_block(vec![0.1; 16_000]);
        let runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("unused-cpu"),
                gpu: PathBuf::from("unused-gpu"),
            },
            Arc::new(PublicationGate::default()),
        )
        .with_controlled_transcriber(Box::new(TailAwareUnifiedWorker))
        .with_controlled_capture(capture, None);
        let controlled_capture_blocks = Arc::new(Mutex::new(VecDeque::new()));
        let mut app = ImportApplication::new(RuntimeImportPort {
            runtime,
            events: VecDeque::new(),
            capture_progress: None,
            document_revision: None,
            unified_stop_submissions: None,
            controlled_capture_blocks: Some(Arc::clone(&controlled_capture_blocks)),
        });

        app.dispatch(AppCommand::StartUnifiedLiveConfigured {
            request: request.clone(),
        })
        .unwrap();
        app.dispatch(AppCommand::Refresh).unwrap();
        app.dispatch(AppCommand::SetMicrophoneGain {
            job_id: request.job_id,
            generation: request.generation,
            target_identity: "test:unified-gain".into(),
            expected_revision: 0,
            gain: whisper_core::microphone_level::GainDb::new(6).unwrap(),
        })
        .unwrap();
        controlled_capture_blocks.lock().unwrap().push_back((
            request.job_id,
            request.generation,
            vec![0.1; 16_000],
        ));
        app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(app.view().microphone_gain_db, 6);
        assert_eq!(app.view().microphone_gain_revision, 1);
        assert_eq!(app.view().microphone_gain_applied_first_frame, Some(16_000));

        app.dispatch(AppCommand::SetMicrophoneGain {
            job_id: request.job_id,
            generation: request.generation,
            target_identity: "test:wrong-source".into(),
            expected_revision: 1,
            gain: whisper_core::microphone_level::GainDb::new(0).unwrap(),
        })
        .unwrap();
        assert!(matches!(
            app.dispatch(AppCommand::SetMicrophoneGain {
                job_id: request.job_id,
                generation: request.generation,
                target_identity: "test:unified-gain".into(),
                expected_revision: 0,
                gain: whisper_core::microphone_level::GainDb::new(0).unwrap(),
            }),
            Err(whisper_core::application::ApplicationError::InvalidCommand)
        ));
        app.dispatch(AppCommand::Refresh).unwrap();
        assert!(
            app.view()
                .message
                .as_deref()
                .is_some_and(|message| message.contains("source active différente"))
        );
        assert_eq!(app.view().microphone_gain_revision, 1);
        assert_eq!(app.view().microphone_gain_requested_revision, 1);
        assert_eq!(app.view().microphone_gain_requested_db, 6);
        app.dispatch(AppCommand::SetMicrophoneGain {
            job_id: request.job_id,
            generation: request.generation,
            target_identity: "test:unified-gain".into(),
            expected_revision: 1,
            gain: whisper_core::microphone_level::GainDb::new(0).unwrap(),
        })
        .unwrap();
        controlled_capture_blocks.lock().unwrap().push_back((
            request.job_id,
            request.generation,
            vec![0.1; 16_000],
        ));
        app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(app.view().microphone_gain_db, 0);
        assert_eq!(app.view().microphone_gain_revision, 2);
        assert_eq!(app.view().microphone_gain_applied_first_frame, Some(32_000));

        app.dispatch(AppCommand::StopUnifiedLive {
            job_id: request.job_id,
            generation: request.generation,
        })
        .unwrap();
        let publication_deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < publication_deadline {
            app.dispatch(AppCommand::Refresh).unwrap();
            if ArchiveStore::scan(&archive_root)
                .unwrap()
                .iter()
                .any(|entry| entry.job_id == request.job_id && entry.complete)
            {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        let generation = archive_root
            .join("transcriptions")
            .join(format!("{:032x}", JOB_ID))
            .join("1");
        let source = crate::parakeet_audio::PcmPartStore::open(
            generation.join("parakeet-unified/source.wav"),
        );
        let before_gain = source.read_window(2_000..14_000).unwrap();
        let after_gain = source.read_window(18_000..30_000).unwrap();
        let after_recovery = source.read_window(34_000..46_000).unwrap();
        assert!(
            before_gain
                .iter()
                .all(|sample| (3_000..=3_600).contains(sample))
        );
        assert!(
            after_gain
                .iter()
                .all(|sample| (6_000..=7_000).contains(sample))
        );
        assert!(
            after_recovery
                .iter()
                .all(|sample| (3_000..=3_600).contains(sample))
        );
        assert!(
            ArchiveStore::scan(&archive_root)
                .unwrap()
                .iter()
                .any(|entry| entry.job_id == request.job_id && entry.complete)
        );
        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn unified_import_and_capture_without_controlled_injection_are_refused_before_hardware_start() {
        let root = std::env::temp_dir().join(format!(
            "parakeet-unified-native-refusal-{}",
            std::process::id()
        ));
        let request = unified_import_request(&root, Path::new("missing.wav"), 777);
        let mut runtime = ImportRuntime::new(
            WorkerPaths {
                cpu: PathBuf::from("missing-cpu-worker"),
                gpu: PathBuf::from("missing-gpu-worker"),
            },
            Arc::new(PublicationGate::default()),
        );
        let mut events = VecDeque::new();
        runtime.effect(ImportEffect::Prepare(request.clone()), &mut events);
        assert!(
            matches!(events.pop_front(), Some(ImportEvent::Failed { message, .. })
            if message.contains("ModelNotQualified") && message.contains("controlled worker"))
        );
        assert!(
            !root.exists(),
            "native refusal must precede archive preparation"
        );
        runtime.effect(ImportEffect::StartWorker(request), &mut events);
        assert!(
            matches!(events.pop_front(), Some(ImportEvent::Failed { message, .. })
            if message.contains("ModelNotQualified") && message.contains("controlled worker"))
        );

        let mut gpu_request = unified_import_request(&root, Path::new("missing-gpu.wav"), 776);
        gpu_request.config.compute = ComputeChoice::Gpu;
        runtime.effect(ImportEffect::StartWorker(gpu_request), &mut events);
        assert!(matches!(
            events.pop_front(),
            Some(ImportEvent::Failed { message, .. })
                if message.contains("ModelNotQualified")
                    && message.contains("controlled worker")
        ));
        assert!(runtime.unified_sessions.is_empty());
        assert!(runtime.unified_executor.is_none());

        let live_request = unified_live_request(&root, 778);
        runtime.effect(ImportEffect::StartUnifiedLive(live_request), &mut events);
        assert!(
            matches!(events.pop_front(), Some(ImportEvent::Failed { message, .. })
            if message.contains("ModelNotQualified")
                && message.contains("controlled worker injection is required")),
            "native capture must be refused before opening a microphone"
        );
        assert!(
            !root.exists(),
            "native capture refusal must precede storage/open side effects"
        );
        assert!(runtime.unified_sessions.is_empty());
        assert!(runtime.unified_executor.is_none());

        let mut gpu_live_request = unified_live_request(&root, 780);
        gpu_live_request.config.compute = ComputeChoice::Gpu;
        runtime.effect(
            ImportEffect::StartUnifiedLive(gpu_live_request),
            &mut events,
        );
        assert!(matches!(
            events.pop_front(),
            Some(ImportEvent::Failed { message, .. })
                if message.contains("ModelNotQualified")
                    && message.contains("controlled worker injection is required")
        ));
        assert!(!root.exists());
        assert!(runtime.unified_sessions.is_empty());
        assert!(runtime.unified_executor.is_none());

        let live_request = unified_live_request(&root, 779);
        let live_identity = (live_request.job_id, live_request.generation);
        let mut app = ImportApplication::new(RuntimeImportPort {
            runtime,
            events: VecDeque::new(),
            capture_progress: None,
            document_revision: None,
            unified_stop_submissions: None,
            controlled_capture_blocks: None,
        });
        app.dispatch(AppCommand::StartUnifiedLiveConfigured {
            request: live_request,
        })
        .unwrap();
        app.dispatch(AppCommand::Refresh).unwrap();
        assert!(
            !root.exists(),
            "the real application route must refuse before opening capture"
        );
        assert!(app.view().unified_parakeet_sessions.iter().any(|session| {
            (session.job_id, session.generation) == live_identity
                && session.phase == whisper_core::ports::UnifiedSessionPhase::Recoverable
                && session.message.as_deref().is_some_and(|message| {
                    message.contains("ModelNotQualified")
                        && message.contains("controlled worker injection is required")
                })
        }));
    }
