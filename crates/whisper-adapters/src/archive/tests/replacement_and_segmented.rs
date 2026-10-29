    #[test]
    fn r11_replace_deactivates_every_matching_publication_and_keeps_other_audio() {
        let generation = Generation::first();
        let original = PublicationIndex {
            version: 1,
            records: vec![
                PublicationRecord {
                    job_id: JobId(1),
                    generation,
                    source_sha256: "a".repeat(64),
                    export_basename: Some("audio".into()),
                    active: true,
                    success_ordinal: None,
                },
                PublicationRecord {
                    job_id: JobId(2),
                    generation,
                    source_sha256: "a".repeat(64),
                    export_basename: Some("audio (2)".into()),
                    active: true,
                    success_ordinal: None,
                },
                PublicationRecord {
                    job_id: JobId(3),
                    generation,
                    source_sha256: "b".repeat(64),
                    export_basename: Some("other".into()),
                    active: true,
                    success_ordinal: None,
                },
            ],
        };
        let intent = PublicationIntent {
            version: 1,
            operation_id: "r11-test".into(),
            choice: DuplicateImportChoice::Replace,
            source_sha256: "a".repeat(64),
            source_samples: 16000,
            expected_active: vec![
                PublicationIdentity {
                    job_id: JobId(1),
                    generation,
                },
                PublicationIdentity {
                    job_id: JobId(2),
                    generation,
                },
            ],
            export_basename: "audio (3)".into(),
            internal_only: false,
            cleanup_id: None,
            cleanup_targets: Vec::new(),
        };
        let replaced = apply_publication_intent(original, JobId(4), generation, &intent);
        assert_eq!(
            replaced
                .records
                .iter()
                .filter(|record| record.source_sha256 == "a".repeat(64) && record.active)
                .count(),
            1
        );
        assert!(
            replaced
                .records
                .iter()
                .any(|record| record.job_id == JobId(1) && !record.active)
        );
        assert!(
            replaced
                .records
                .iter()
                .any(|record| record.job_id == JobId(2) && !record.active)
        );
        assert!(
            replaced
                .records
                .iter()
                .any(|record| record.job_id == JobId(3) && record.active)
        );
        assert!(
            replaced
                .records
                .iter()
                .any(|record| record.job_id == JobId(4) && record.active)
        );
    }

    #[test]
    fn r11_create_new_preserves_existing_publications_and_index_write_is_atomic() {
        let root = std::env::temp_dir().join(format!(
            "whisper-r11-index-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let generation = Generation::first();
        let index = PublicationIndex {
            version: 1,
            records: vec![PublicationRecord {
                job_id: JobId(1),
                generation,
                source_sha256: "a".repeat(64),
                export_basename: Some("audio".into()),
                active: true,
                success_ordinal: None,
            }],
        };
        let intent = PublicationIntent {
            version: 1,
            operation_id: "new".into(),
            choice: DuplicateImportChoice::CreateNew,
            source_sha256: "a".repeat(64),
            source_samples: 16000,
            expected_active: vec![PublicationIdentity {
                job_id: JobId(1),
                generation,
            }],
            export_basename: "audio (2)".into(),
            internal_only: false,
            cleanup_id: None,
            cleanup_targets: Vec::new(),
        };
        let next = apply_publication_intent(index, JobId(2), generation, &intent);
        assert_eq!(
            next.records.iter().filter(|record| record.active).count(),
            2
        );
        write_publication_index(&root, &next).unwrap();
        assert_eq!(read_publication_index(&root).unwrap(), next);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn r11_export_names_remain_reserved_by_the_index_after_visible_files_are_removed() {
        let root = std::env::temp_dir().join(format!(
            "whisper-r11-indexed-name-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        write_publication_index(
            &root,
            &PublicationIndex {
                version: 1,
                records: vec![PublicationRecord {
                    job_id: JobId(81),
                    generation: Generation::first(),
                    source_sha256: "a".repeat(64),
                    export_basename: Some("audio".into()),
                    active: false,
                    success_ordinal: None,
                }],
            },
        )
        .unwrap();
        assert_eq!(
            reserve_export_basename(&root, "C:/input/audio.wav").unwrap(),
            "audio (2)",
            "an indexed basename remains reserved even after its visible exports disappear"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn r11_replaced_export_reconciliation_never_moves_a_third_party_same_name_file() {
        let root = std::env::temp_dir().join(format!(
            "whisper-r11-third-party-export-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let generation = Generation::first();
        let job_id = JobId(82);
        let directory = generation_path(&root, job_id, generation);
        fs::create_dir_all(&directory).unwrap();
        let text = b"old generated transcription";
        let srt = b"old generated subtitles";
        let text_path = directory.join("transcript.txt");
        let srt_path = directory.join("transcript.srt");
        fs::write(&text_path, text).unwrap();
        fs::write(&srt_path, srt).unwrap();
        let artifact = |path: &str, bytes: &[u8]| ArtifactRecord {
            path: path.into(),
            sha256: hex(&Sha256::digest(bytes)),
            bytes: bytes.len() as u64,
        };
        fs::write(
            directory.join("manifest.json"),
            serde_json::to_vec(&CompleteRecord {
                version: 1,
                job_id,
                generation,
                source_sha256: "a".repeat(64),
                segments: 1,
                text: artifact("transcript.txt", text),
                srt: artifact("transcript.srt", srt),
            })
            .unwrap(),
        )
        .unwrap();
        let exports = root.join("exports");
        fs::create_dir_all(&exports).unwrap();
        let third_party = b"unrelated user file";
        fs::write(exports.join("audio.txt"), third_party).unwrap();
        let index = PublicationIndex {
            version: 1,
            records: vec![PublicationRecord {
                job_id,
                generation,
                source_sha256: "a".repeat(64),
                export_basename: Some("audio".into()),
                active: false,
                success_ordinal: None,
            }],
        };
        reconcile_replaced_exports(&root, &index).unwrap();
        assert_eq!(fs::read(exports.join("audio.txt")).unwrap(), third_party);
        assert!(
            !root
                .join("transcriptions-remplacees/audio-00000000000000000000000000000052-1.txt")
                .exists()
        );
        assert_eq!(fs::read(exports.join("audio.txt")).unwrap(), third_party);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn r11_recovery_between_current_and_index_keeps_legacy_visible_until_explicit_activation() {
        let root = std::env::temp_dir().join(format!(
            "whisper-r11-cut-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let source_sha256 = "a".repeat(64);
        let generation = Generation::first();
        let legacy_job = JobId(31);
        let r11_job = JobId(32);

        let write_complete = |job_id: JobId, marker: Option<PublicationIntent>| {
            let directory = generation_path(&root, job_id, generation);
            fs::create_dir_all(&directory).unwrap();
            let pending = PendingRecord {
                strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
                version: 1,
                job_id,
                generation,
                source_path: "legacy-source.wav".into(),
                source_sha256: source_sha256.clone(),
                source_samples: 160,
                model_path: "model.bin".into(),
                model_sha256: APPROVED_MODEL_SHA256.into(),
                model_size: APPROVED_MODEL_SIZE,
                qualification: None,
            };
            fs::write(
                directory.join("pending.json"),
                serde_json::to_vec(&pending).unwrap(),
            )
            .unwrap();
            let segment = SegmentRecord {
                sequence: 1,
                range: SourceRange::new(0, 160, 16_000).unwrap(),
                text: format!("transcript-{}", job_id.0),
            };
            fs::write(
                directory.join("segments.jsonl"),
                [serde_json::to_vec(&segment).unwrap(), vec![b'\n']].concat(),
            )
            .unwrap();
            fs::write(directory.join("transcript.txt"), segment.text.as_bytes()).unwrap();
            fs::write(
                directory.join("transcript.srt"),
                b"1\n00:00:00,000 --> 00:00:00,010\ntext\n",
            )
            .unwrap();
            let manifest = CompleteRecord {
                version: 1,
                job_id,
                generation,
                source_sha256: source_sha256.clone(),
                segments: 1,
                text: verified_artifact(&directory, &directory.join("transcript.txt")).unwrap(),
                srt: verified_artifact(&directory, &directory.join("transcript.srt")).unwrap(),
            };
            fs::write(
                directory.join("manifest.json"),
                serde_json::to_vec(&manifest).unwrap(),
            )
            .unwrap();
            fs::copy(
                directory.join("transcript.txt"),
                directory.join("export.txt.pending"),
            )
            .unwrap();
            fs::copy(
                directory.join("transcript.srt"),
                directory.join("export.srt.pending"),
            )
            .unwrap();
            if let Some(intent) = marker {
                fs::write(
                    directory.join("r11-publication.json"),
                    serde_json::to_vec(&intent).unwrap(),
                )
                .unwrap();
            }
            fs::write(directory.join("CURRENT"), b"manifest.json\n").unwrap();
            directory
        };

        write_complete(legacy_job, None);
        let intent = PublicationIntent {
            version: 1,
            operation_id: "r11-cut-between-current-index".into(),
            choice: DuplicateImportChoice::Replace,
            source_sha256: source_sha256.clone(),
            source_samples: 160,
            expected_active: vec![PublicationIdentity {
                job_id: legacy_job,
                generation,
            }],
            export_basename: "legacy-source (2)".into(),
            internal_only: false,
            cleanup_id: None,
            cleanup_targets: Vec::new(),
        };
        write_complete(r11_job, Some(intent));

        let before_recovery = ArchiveStore::scan(&root).unwrap();
        assert!(before_recovery.iter().any(|item| {
            item.job_id == legacy_job && item.generation == generation && item.complete
        }));
        assert!(before_recovery.iter().any(|item| {
            item.job_id == r11_job
                && item.generation == generation
                && !item.complete
                && item.publication_recovery
        }));
        assert_eq!(
            ArchiveStore::published_imports(&root, &source_sha256)
                .unwrap()
                .iter()
                .map(|item| item.job_id)
                .collect::<Vec<_>>(),
            vec![legacy_job],
            "a CURRENT-before-index cut keeps the legacy publication visible"
        );

        ArchiveStore::resolve_import_publication(&root, r11_job, generation).unwrap();
        let after_recovery = ArchiveStore::published_imports(&root, &source_sha256).unwrap();
        assert_eq!(
            after_recovery
                .iter()
                .map(|item| item.job_id)
                .collect::<Vec<_>>(),
            vec![r11_job],
            "explicit activation applies Replace after revalidating the visible legacy set"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn segmented_capture_identity_references_every_rotated_pcm_part() {
        let root = std::env::temp_dir().join(format!(
            "whisper-segmented-capture-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let source_path = root.join("capture.wav");
        let mut writer =
            crate::parakeet_audio::CapturePcmWriter::start_with_test_part_limit(&source_path, 4, 4)
                .unwrap();
        writer.try_append(&[11, 22, 33, 44, 55]).unwrap();
        assert_eq!(writer.close().unwrap(), 5);

        let source = crate::parakeet_audio::PcmPartStore::open(&source_path);
        let (identity, samples, manifest) = segmented_pcm_manifest(&source).unwrap();
        assert_eq!(samples, 5);
        assert_eq!(manifest.source_samples, samples);
        assert_eq!(manifest.source_sha256, hex(&identity));
        assert_eq!(manifest.parts.len(), 3);
        assert_eq!(
            manifest
                .parts
                .iter()
                .map(|part| (part.start_sample, part.end_sample))
                .collect::<Vec<_>>(),
            vec![(0, 2), (2, 4), (4, 5)]
        );
        assert!(manifest.parts.iter().all(|part| {
            let path = Path::new(&part.path);
            path.is_file()
                && hex(&sha256_file(path).unwrap()) == part.sha256
                && fs::metadata(path).unwrap().len() == part.bytes
        }));
        fs::remove_dir_all(root).unwrap();
    }
