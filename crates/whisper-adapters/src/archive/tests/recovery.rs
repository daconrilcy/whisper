    #[test]
    fn r7_copy_fallback_recovers_between_exports_without_clobbering_a_byte_identical_collision() {
        let scenario = R7RecoveryScenario::new();
        let unrecorded_identity = scenario.assert_interrupted_collision();
        let confirmed_intent = scenario.reconfirm_publication();
        scenario.commit_recovered_publication(confirmed_intent, unrecorded_identity);
        fs::remove_dir_all(&scenario.root).unwrap();
    }

    #[test]
    fn segment_recovery_keeps_synced_prefix_and_drops_only_a_partial_tail() {
        let path = std::env::temp_dir().join(format!(
            "whisper-segments-{}-{}.jsonl",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let record = SegmentRecord {
            sequence: 1,
            range: SourceRange::new(0, 160, 16_000).unwrap(),
            text: "confirmed".into(),
        };
        let mut contents = serde_json::to_vec(&record).unwrap();
        contents.extend_from_slice(b"\n{\"sequence\":");
        fs::write(&path, contents).unwrap();

        assert_eq!(read_segments(&path).unwrap(), vec![record]);
        assert!(fs::read(&path).unwrap().ends_with(b"\n"));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn scan_preserves_healthy_entries_and_reports_each_corrupt_generation() {
        let root = std::env::temp_dir().join(format!(
            "whisper-archive-scan-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let healthy = create_healthy_archive_scan_fixture(&root);
        create_corrupt_archive_scan_fixtures(&root, &healthy);
        assert_corrupt_archive_scan_results(&root);
    }

    fn create_healthy_archive_scan_fixture(root: &Path) -> PathBuf {
        let healthy = root
            .join("transcriptions")
            .join(format!("{:032x}", 7_u128))
            .join("1");
        fs::create_dir_all(&healthy).unwrap();
        fs::write(healthy.join("transcript.txt"), b"healthy").unwrap();
        fs::write(
            healthy.join("transcript.srt"),
            b"1\n00:00:00,000 --> 00:00:00,010\nhealthy\n",
        )
        .unwrap();
        let text = verified_artifact(&healthy, &healthy.join("transcript.txt")).unwrap();
        let srt = verified_artifact(&healthy, &healthy.join("transcript.srt")).unwrap();
        let pending = PendingRecord {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
            version: 1,
            job_id: JobId(7),
            generation: Generation::first(),
            source_path: "fixture.wav".into(),
            source_sha256: "a".repeat(64),
            source_samples: 160,
            model_path: "model.bin".into(),
            model_sha256: APPROVED_MODEL_SHA256.into(),
            model_size: APPROVED_MODEL_SIZE,
            qualification: None,
        };
        fs::write(
            healthy.join("pending.json"),
            serde_json::to_vec(&pending).unwrap(),
        )
        .unwrap();
        fs::write(
            healthy.join("segments.jsonl"),
            serde_json::to_vec(&SegmentRecord {
                sequence: 1,
                range: SourceRange::new(0, 160, 16_000).unwrap(),
                text: "healthy".into(),
            })
            .unwrap()
            .into_iter()
            .chain(std::iter::once(b'\n'))
            .collect::<Vec<_>>(),
        )
        .unwrap();
        let manifest = CompleteRecord {
            version: 1,
            job_id: JobId(7),
            generation: Generation::first(),
            source_sha256: "a".repeat(64),
            segments: 1,
            text,
            srt,
        };
        fs::write(
            healthy.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        fs::write(healthy.join("CURRENT"), b"manifest.json\n").unwrap();
        healthy
    }

    fn create_corrupt_archive_scan_fixtures(root: &Path, healthy: &Path) {
        let bad_current = root
            .join("transcriptions")
            .join(format!("{:032x}", 11_u128))
            .join("1");
        fs::create_dir_all(&bad_current).unwrap();
        fs::copy(
            healthy.join("pending.json"),
            bad_current.join("pending.json"),
        )
        .unwrap();
        fs::copy(
            healthy.join("manifest.json"),
            bad_current.join("manifest.json"),
        )
        .unwrap();
        fs::copy(
            healthy.join("segments.jsonl"),
            bad_current.join("segments.jsonl"),
        )
        .unwrap();
        fs::copy(
            healthy.join("transcript.txt"),
            bad_current.join("transcript.txt"),
        )
        .unwrap();
        fs::copy(
            healthy.join("transcript.srt"),
            bad_current.join("transcript.srt"),
        )
        .unwrap();
        fs::write(bad_current.join("CURRENT"), b"../../other/manifest.json\n").unwrap();

        let transplanted = root
            .join("transcriptions")
            .join(format!("{:032x}", 12_u128))
            .join("3");
        fs::create_dir_all(&transplanted).unwrap();
        for name in [
            "pending.json",
            "manifest.json",
            "segments.jsonl",
            "transcript.txt",
            "transcript.srt",
            "CURRENT",
        ] {
            fs::copy(healthy.join(name), transplanted.join(name)).unwrap();
        }

        let bad_pending = root
            .join("transcriptions")
            .join(format!("{:032x}", 8_u128))
            .join("1");
        fs::create_dir_all(&bad_pending).unwrap();
        fs::write(bad_pending.join("pending.json"), b"{corrupt").unwrap();
        let bad_manifest = root
            .join("transcriptions")
            .join(format!("{:032x}", 9_u128))
            .join("2");
        fs::create_dir_all(&bad_manifest).unwrap();
        fs::write(bad_manifest.join("CURRENT"), b"manifest.json\n").unwrap();
        fs::write(bad_manifest.join("manifest.json"), b"{corrupt").unwrap();
        fs::write(
            root.join("transcriptions")
                .join(format!("{:032x}", 10_u128)),
            b"not a job directory",
        )
        .unwrap();
        fs::write(
            root.join("transcriptions").join("queue.jsonl"),
            b"durable queue journal",
        )
        .unwrap();
    }

    fn assert_corrupt_archive_scan_results(root: &Path) {
        let first = ArchiveStore::scan(root).unwrap();
        let second = ArchiveStore::scan(root).unwrap();
        assert_eq!(
            first, second,
            "corruption diagnostics are stable across scans"
        );
        assert!(
            !first.iter().any(|item| item.job_id == JobId(0)),
            "non-directory queue journal is not a synthetic corrupt job"
        );
        assert!(
            first
                .iter()
                .any(|item| item.job_id == JobId(7) && item.complete)
        );
        assert!(
            first
                .iter()
                .any(|item| item.job_id == JobId(8) && item.diagnostic.is_some())
        );
        assert!(
            first
                .iter()
                .any(|item| item.job_id == JobId(9) && item.diagnostic.is_some())
        );
        assert!(
            first
                .iter()
                .any(|item| item.job_id == JobId(10) && item.diagnostic.is_some())
        );
        for job_id in [JobId(11), JobId(12)] {
            assert!(
                first
                    .iter()
                    .any(|item| item.job_id == job_id && item.diagnostic.is_some())
            );
            assert!(
                !first
                    .iter()
                    .any(|item| item.job_id == job_id && item.complete)
            );
        }
    }

    #[test]
    fn published_running_row_is_removed_durably_and_recovery_scans_are_stable() {
        use crate::queue_store::{QueueStatus, QueueStore};
        use whisper_core::SegmentId;

        let root = std::env::temp_dir().join(format!(
            "whisper-publish-queue-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let request = ImportRequest {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
            job_id: JobId(71),
            generation: Generation::first(),
            source_path: "source.wav".into(),
            source_sha256: Some([0; 32]),
            source_samples: Some(160),
            model_path: "model.bin".into(),
            model_sha256: [0; 32],
            destination: root.to_string_lossy().into_owned(),
            config: whisper_core::JobConfig {
                language: whisper_core::LanguageChoice::Manual("fr".into()),
                compute: whisper_core::ComputeChoice::Cpu,
            },
        };
        let mut other = request.clone();
        other.job_id = JobId(72);
        let mut queue = QueueStore::open(&root).unwrap();
        queue.enqueue(request.clone()).unwrap();
        queue.enqueue(other.clone()).unwrap();
        queue
            .set_status(request.job_id, request.generation, QueueStatus::Running)
            .unwrap();

        let directory = root
            .join("transcriptions")
            .join(format!("{:032x}", request.job_id.0))
            .join(request.generation.get().to_string());
        fs::create_dir_all(&directory).unwrap();
        let pending = PendingRecord {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
            version: 1,
            job_id: request.job_id,
            generation: request.generation,
            source_path: request.source_path.clone(),
            source_sha256: hex(&[0; 32]),
            source_samples: 160,
            model_path: request.model_path.clone(),
            model_sha256: APPROVED_MODEL_SHA256.into(),
            model_size: APPROVED_MODEL_SIZE,
            qualification: None,
        };
        fs::write(
            directory.join("pending.json"),
            serde_json::to_vec(&pending).unwrap(),
        )
        .unwrap();
        let mut archive = ArchiveStore::new(&root);
        archive.current = Some(ActiveArchive {
            job_id: request.job_id,
            generation: request.generation,
            directory,
            source_sha256: [0; 32],
            source_samples: 160,
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
            expected_sequence: 1,
            publication: PublicationIntent::default(),
            segmented_source_manifest: None,
        });
        archive
            .persist_segment(&WorkerSegment {
                job_id: request.job_id,
                generation: request.generation,
                instance_id: 1,
                segment_id: SegmentId(1),
                range: SourceRange::new(0, 160, 16_000).unwrap(),
                text: "confirmed".into(),
            })
            .unwrap();
        archive
            .publish(request.job_id, request.generation, 1)
            .unwrap();

        assert_recovery_scans_after_publication(&root, &request, &other);
    }

    fn assert_recovery_scans_after_publication(
        root: &Path,
        request: &ImportRequest,
        other: &ImportRequest,
    ) {
        use crate::queue_store::QueueStore;

        let mut reopened = QueueStore::open(root).unwrap();
        reopened.remove(request.job_id, request.generation).unwrap();
        drop(reopened);
        let first = crate::recovery::scan(root).unwrap();
        let second = crate::recovery::scan(root).unwrap();
        assert_eq!(
            first, second,
            "recovery is stable after publication cleanup"
        );
        assert_eq!(first.queue.len(), 1);
        assert_eq!(first.queue[0].request.job_id, other.job_id);
        assert!(ArchiveStore::scan(root).unwrap().iter().any(|entry| {
            entry.job_id == request.job_id
                && entry.generation == request.generation
                && entry.complete
        }));
        assert!(
            !QueueStore::open(root)
                .unwrap()
                .entries()
                .iter()
                .any(|entry| entry.request.job_id == request.job_id)
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn resume_rejects_canonical_cross_destination_without_mutating_either_root() {
        let root = std::env::temp_dir().join(format!(
            "whisper-resume-cross-destination-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let previous_destination = root.join("previous");
        let requested_destination = root.join("requested");
        fs::create_dir_all(&previous_destination).unwrap();
        fs::create_dir_all(&requested_destination).unwrap();
        let previous_sentinel = previous_destination.join("sentinel");
        let requested_sentinel = requested_destination.join("sentinel");
        fs::write(&previous_sentinel, b"previous destination unchanged").unwrap();
        fs::write(&requested_sentinel, b"requested destination unchanged").unwrap();
        let entries = |directory: &Path| {
            let mut entries: Vec<_> = fs::read_dir(directory)
                .unwrap()
                .map(|entry| entry.unwrap().file_name())
                .collect();
            entries.sort();
            entries
        };
        let previous_entries = entries(&previous_destination);
        let requested_entries = entries(&requested_destination);
        let job_id = JobId(38_001);
        let previous = ImportRequest {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
            job_id,
            generation: Generation::first(),
            source_path: "source.wav".into(),
            source_sha256: Some([1; 32]),
            source_samples: Some(160),
            model_path: "model.bin".into(),
            model_sha256: [2; 32],
            destination: previous_destination.to_string_lossy().into_owned(),
            config: whisper_core::JobConfig {
                language: whisper_core::LanguageChoice::Manual("fr".into()),
                compute: whisper_core::ComputeChoice::Cpu,
            },
        };
        let mut request = previous.clone();
        request.generation = previous.generation.next().unwrap();
        request.destination = requested_destination.to_string_lossy().into_owned();
        let mut archive = ArchiveStore::new(&previous_destination);

        let error = archive
            .resume_from_with_qualification(&previous, request, None, None)
            .unwrap_err();

        assert!(
            error.contains("resume destination differs from checkpoint destination"),
            "unexpected cross-destination resume error: {error}"
        );
        assert_eq!(
            fs::read(&previous_sentinel).unwrap().as_slice(),
            b"previous destination unchanged"
        );
        assert_eq!(
            fs::read(&requested_sentinel).unwrap().as_slice(),
            b"requested destination unchanged"
        );
        assert_eq!(entries(&previous_destination), previous_entries);
        assert_eq!(entries(&requested_destination), requested_entries);
        assert!(archive.current.is_none());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn resumed_prefix_copy_retries_after_pending_creation_without_duplication() {
        use whisper_core::SegmentId;

        let root = std::env::temp_dir().join(format!(
            "whisper-resume-copy-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let request = ImportRequest {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
            job_id: JobId(81),
            generation: Generation::first().next().unwrap(),
            source_path: "source.wav".into(),
            source_sha256: Some([0; 32]),
            source_samples: Some(320),
            model_path: "model.bin".into(),
            model_sha256: [0; 32],
            destination: root.to_string_lossy().into_owned(),
            config: whisper_core::JobConfig {
                language: whisper_core::LanguageChoice::Manual("fr".into()),
                compute: whisper_core::ComputeChoice::Cpu,
            },
        };
        let target = root
            .join("transcriptions")
            .join(format!("{:032x}", request.job_id.0))
            .join(request.generation.get().to_string());
        fs::create_dir_all(&target).unwrap();
        let pending = interrupted_prefix_pending_record(&request);
        assert_eq!(initialize_pending(&target, &pending).unwrap(), 1);
        let prefix = interrupted_prefix_segments();
        let mut interrupted_archive = ArchiveStore::new(&root);
        interrupted_archive.current = Some(ActiveArchive {
            job_id: request.job_id,
            generation: request.generation,
            directory: target.clone(),
            source_sha256: [0; 32],
            source_samples: 320,
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
            expected_sequence: 1,
            publication: PublicationIntent::default(),
            segmented_source_manifest: None,
        });
        interrupted_archive
            .persist_segment(&WorkerSegment {
                job_id: request.job_id,
                generation: request.generation,
                instance_id: 0,
                segment_id: SegmentId(1),
                range: prefix[0].range,
                text: prefix[0].text.clone(),
            })
            .unwrap();

        let mut retried_archive = ArchiveStore::new(&root);
        let expected_sequence = initialize_pending(&target, &pending).unwrap();
        assert_eq!(
            expected_sequence, 2,
            "reopened pending preserves the partial prefix"
        );
        retried_archive.current = Some(ActiveArchive {
            job_id: request.job_id,
            generation: request.generation,
            directory: target.clone(),
            source_sha256: [0; 32],
            source_samples: 320,
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
            expected_sequence,
            publication: PublicationIntent::default(),
            segmented_source_manifest: None,
        });
        retried_archive
            .copy_resume_prefix(&request, &prefix)
            .unwrap();
        let mut second_retry = ArchiveStore::new(&root);
        let expected_sequence = initialize_pending(&target, &pending).unwrap();
        assert_eq!(
            expected_sequence, 3,
            "retry sees the complete copied prefix"
        );
        second_retry.current = Some(ActiveArchive {
            job_id: request.job_id,
            generation: request.generation,
            directory: target.clone(),
            source_sha256: [0; 32],
            source_samples: 320,
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
            expected_sequence,
            publication: PublicationIntent::default(),
            segmented_source_manifest: None,
        });
        second_retry.copy_resume_prefix(&request, &prefix).unwrap();
        assert_eq!(
            read_segments(&target.join("segments.jsonl")).unwrap(),
            prefix
        );
        fs::remove_dir_all(root).unwrap();
    }

    fn interrupted_prefix_pending_record(request: &ImportRequest) -> PendingRecord {
        PendingRecord {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
            version: 1,
            job_id: request.job_id,
            generation: request.generation,
            source_path: request.source_path.clone(),
            source_sha256: hex(&[0; 32]),
            source_samples: 320,
            model_path: request.model_path.clone(),
            model_sha256: APPROVED_MODEL_SHA256.into(),
            model_size: APPROVED_MODEL_SIZE,
            qualification: None,
        }
    }

    fn interrupted_prefix_segments() -> Vec<SegmentRecord> {
        vec![
            SegmentRecord {
                sequence: 1,
                range: SourceRange::new(0, 160, 16_000).unwrap(),
                text: "one".into(),
            },
            SegmentRecord {
                sequence: 2,
                range: SourceRange::new(160, 320, 16_000).unwrap(),
                text: "two".into(),
            },
        ]
    }
