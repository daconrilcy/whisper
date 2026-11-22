    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    #[cfg_attr(not(feature = "native-tests"), ignore = "requires WHISPER_L04_CAMPAIGN_BUILD=1 at compile time and WHISPER_L04_CAMPAIGN_RUN at runtime")]
    fn recette_policy_l04_requires_the_feature() {
        assert_eq!(option_env!("WHISPER_L04_CAMPAIGN_BUILD"), Some("1"));
        assert!(std::env::var_os("WHISPER_L04_CAMPAIGN_RUN").is_some());
        let result = campaign_gate("not-a-campaign-hook", 1, 1, 0);
        #[cfg(feature = "recette-hooks")]
        assert!(result.unwrap_err().starts_with("CampaignGuard:"));
        #[cfg(not(feature = "recette-hooks"))]
        assert!(result.is_ok());
    }

    #[test]
    fn bounded_resume_accepts_a_current_backend_for_the_same_model_profile() {
        let cpu = ArchiveQualificationKey::parakeet_cpu_test();
        let gpu = ArchiveQualificationKey::parakeet_gpu_test();
        assert!(cpu.permits_bounded_resume_to(&gpu));
        let mut other_profile = gpu.clone();
        other_profile.inference_profile.push_str("-other");
        assert!(!cpu.permits_bounded_resume_to(&other_profile));
        let mut other_model = gpu;
        other_model.model.push_str("-other");
        assert!(!cpu.permits_bounded_resume_to(&other_model));
    }

    #[test]
    fn dual_cpu_identity_requires_the_compiled_worker_and_runtime_hashes() {
        let key = ArchiveQualificationKey::parakeet_dual_cpu();
        let expected = option_env!("WHISPER_DUAL_PARAKEET_CPU_WORKER_SHA256")
            .zip(option_env!("WHISPER_DUAL_PARAKEET_CPU_RUNTIME_SHA256"));
        assert_eq!(key.is_some(), expected.is_some_and(|(worker, runtime)| {
            [worker, runtime].into_iter().all(|value| {
                value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
            })
        }));
        if let (Some(key), Some((worker, runtime))) = (key, expected) {
            assert_eq!(key.worker_build, worker.to_ascii_lowercase());
            assert_eq!(key.runtime_build, runtime.to_ascii_lowercase());
            assert_eq!(key.backend, "CPU");
            assert!(key.gpu_uuid.is_none());
        }
    }

    #[test]
    fn create_only_publication_returns_verified_internal_unified_artifacts_without_external_exports()
     {
        let scenario = InternalPublicationScenario::new();
        scenario.assert_published_internal_artifacts();
        scenario.assert_untrusted_artifacts_rejected();
        eprintln!("Preserved internal publication test artifacts at {}", scenario.root.display());
    }

    struct InternalPublicationScenario {
        root: PathBuf,
        directory: PathBuf,
        source_path: PathBuf,
        text: &'static [u8],
        srt: &'static [u8],
        manifest: CompleteRecord,
        intent: PublicationIntent,
    }

    impl InternalPublicationScenario {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "whisper-create-only-publication-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            let job_id = JobId(0x51_001);
            let generation = Generation::first();
            let directory = generation_path(&root, job_id, generation);
            let source_path = directory.join("parakeet-unified/source.wav");
            fs::create_dir_all(source_path.parent().unwrap()).unwrap();
            let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_path).unwrap();
            writer.append(&vec![0_i16; 16_000]).unwrap();
            assert_eq!(writer.finish().unwrap(), 16_000);
            let text = b"verified internal transcript";
            let srt = b"1\n00:00:00,000 --> 00:00:01,000\nverified internal transcript\n";
            fs::write(directory.join("transcript.txt"), text).unwrap();
            fs::write(directory.join("transcript.srt"), srt).unwrap();
            let (manifest, intent) = persist_internal_publication_records(
                &directory,
                &root,
                &source_path,
                (job_id, generation),
            );
            Self {
                root,
                directory,
                source_path,
                text,
                srt,
                manifest,
                intent,
            }
        }

        fn assert_published_internal_artifacts(&self) {
            let exports = self.root.join("exports");
            fs::create_dir_all(&exports).unwrap();
            let sentinel_txt = b"pre-existing external TXT";
            let sentinel_srt = b"pre-existing external SRT";
            fs::write(exports.join("transcript.txt"), sentinel_txt).unwrap();
            fs::write(exports.join("transcript.srt"), sentinel_srt).unwrap();
            let reserved_request = ImportRequest {
                strategy: whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1,
                job_id: JobId(self.manifest.job_id.0 + 1),
                generation: self.manifest.generation,
                source_path: self.source_path.to_string_lossy().into_owned(),
                source_sha256: Some([0x52; 32]),
                source_samples: Some(16_000),
                model_path: "unused-model".into(),
                model_sha256: [0; 32],
                destination: self.root.to_string_lossy().into_owned(),
                config: whisper_core::JobConfig {
                    language: whisper_core::LanguageChoice::Automatic,
                    compute: whisper_core::ComputeChoice::Cpu,
                },
            };
            ArchiveStore::reserve_create_only_new_import(
                &self.root,
                &reserved_request,
                &"52".repeat(32),
            )
            .unwrap();
            let reservation_path = publication_reservation_path(
                &self.root,
                reserved_request.job_id,
                reserved_request.generation,
            );
            let reservation_before = fs::read(&reservation_path).unwrap();
            let index_path = publication_index_path(&self.root);
            let index_before = fs::read(&index_path).unwrap();

            let files = self.published_files_for(self.manifest.job_id).unwrap();
            let expected_wav = self.source_path.canonicalize().unwrap();
            let expected_txt = self.directory.join("transcript.txt").canonicalize().unwrap();
            let expected_srt = self.directory.join("transcript.srt").canonicalize().unwrap();
            assert_eq!(Path::new(&files.wav_path), expected_wav.as_path());
            assert_eq!(Path::new(&files.txt_path), expected_txt.as_path());
            assert_eq!(Path::new(&files.srt_path), expected_srt.as_path());
            assert_eq!(fs::read(&files.txt_path).unwrap(), self.text);
            assert_eq!(fs::read(&files.srt_path).unwrap(), self.srt);
            assert_eq!(fs::read(exports.join("transcript.txt")).unwrap(), sentinel_txt);
            assert_eq!(fs::read(exports.join("transcript.srt")).unwrap(), sentinel_srt);
            assert_eq!(fs::read(&reservation_path).unwrap(), reservation_before);
            assert_eq!(fs::read(&index_path).unwrap(), index_before);
        }

        fn assert_untrusted_artifacts_rejected(&self) {
            let intent_path = self.directory.join("r11-publication.json");
            let valid_intent = fs::read(&intent_path).unwrap();
            let mut altered_intent = self.intent.clone();
            altered_intent.operation_id = format!(
                "untrusted-prefix-{:032x}-{}",
                self.manifest.job_id.0,
                self.manifest.generation.get()
            );
            fs::write(&intent_path, serde_json::to_vec(&altered_intent).unwrap()).unwrap();
            self.assert_rejected_containing(
                "StaleResponse",
                "an arbitrary operation prefix must not authorize internal artifacts",
            );
            fs::write(&intent_path, valid_intent).unwrap();

            let mut corrupt_manifest = self.manifest.clone();
            corrupt_manifest.text.path = "../exports/transcript.txt".into();
            write_publication_manifest(&self.directory, &corrupt_manifest);
            self.assert_rejected_containing("unexpected internal publication path", "expected internal path validation rejection");

            write_publication_manifest(&self.directory, &self.manifest);
            fs::write(self.directory.join("transcript.txt"), b"altered transcript").unwrap();
            self.assert_rejected_containing("artifact verification failed", "expected altered transcript rejection");
            fs::write(self.directory.join("transcript.txt"), self.text).unwrap();

            fs::remove_file(self.directory.join("transcript.srt")).unwrap();
            assert!(
                self.published_files_for(self.manifest.job_id).is_err(),
                "a missing internal SRT must remain unavailable"
            );
            fs::write(self.directory.join("transcript.srt"), self.srt).unwrap();

            let mut mismatched_manifest = self.manifest.clone();
            mismatched_manifest.job_id = JobId(self.manifest.job_id.0 + 2);
            write_publication_manifest(&self.directory, &mismatched_manifest);
            self.assert_rejected_containing("StaleResponse", "expected stale response rejection");
            write_publication_manifest(&self.directory, &self.manifest);

            let index = self.source_path.with_extension("parts.idx");
            let valid_index = fs::read(&index).unwrap();
            fs::write(&index, [valid_index.as_slice(), b"x"].concat()).unwrap();
            assert!(self.published_files_for(self.manifest.job_id).is_err());
            fs::write(&index, valid_index).unwrap();
            assert!(self.published_files_for(JobId(self.manifest.job_id.0 + 3)).is_err());
        }

        fn published_files_for(
            &self,
            job_id: JobId,
        ) -> Result<whisper_core::ports::UnifiedPublicationFiles, String> {
            ArchiveStore::published_unified_files(&self.root, job_id, self.manifest.generation)
        }

        fn assert_rejected_containing(&self, diagnostic: &str, message: &str) {
            assert!(
                self.published_files_for(self.manifest.job_id)
                    .unwrap_err()
                    .contains(diagnostic),
                "{message}"
            );
        }
    }

    fn persist_internal_publication_records(
        directory: &Path,
        root: &Path,
        source_path: &Path,
        identity: (JobId, Generation),
    ) -> (CompleteRecord, PublicationIntent) {
        let (job_id, generation) = identity;
        let manifest = CompleteRecord {
            version: 1,
            job_id,
            generation,
            source_sha256: "51".repeat(32),
            segments: 1,
            text: verified_artifact(directory, &directory.join("transcript.txt")).unwrap(),
            srt: verified_artifact(directory, &directory.join("transcript.srt")).unwrap(),
        };
        let pending = PendingRecord {
            version: 4,
            job_id,
            generation,
            source_path: source_path.to_string_lossy().into_owned(),
            source_sha256: manifest.source_sha256.clone(),
            source_samples: 16_000,
            model_path: "unused-model".into(),
            model_sha256: "00".repeat(32),
            model_size: 0,
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1,
            qualification: None,
        };
        fs::write(
            directory.join("pending.json"),
            serde_json::to_vec(&pending).unwrap(),
        )
        .unwrap();
        write_publication_manifest(directory, &manifest);
        let intent = PublicationIntent {
            version: 1,
            operation_id: format!("{:032x}-{}", job_id.0, generation.get()),
            choice: DuplicateImportChoice::CreateNew,
            source_sha256: manifest.source_sha256.clone(),
            source_samples: 16_000,
            expected_active: Vec::new(),
            export_basename: "transcript".into(),
            internal_only: true,
            cleanup_id: None,
            cleanup_targets: Vec::new(),
        };
        fs::write(
            directory.join("r11-publication.json"),
            serde_json::to_vec(&intent).unwrap(),
        )
        .unwrap();
        fs::write(directory.join("CURRENT"), b"manifest.json\n").unwrap();
        write_publication_index(
            root,
            &PublicationIndex {
                version: 1,
                records: vec![PublicationRecord {
                    job_id,
                    generation,
                    source_sha256: manifest.source_sha256.clone(),
                    export_basename: None,
                    active: true,
                    success_ordinal: Some(1),
                }],
            },
        )
        .unwrap();
        (manifest, intent)
    }

    fn write_publication_manifest(directory: &Path, manifest: &CompleteRecord) {
        fs::write(directory.join("manifest.json"), serde_json::to_vec(manifest).unwrap()).unwrap();
    }
    #[test]
    #[cfg(windows)]
    fn reprocessing_reservation_tracks_only_current_strategy_publications_for_resume() {
        let root = std::env::temp_dir().join(format!(
            "whisper-reprocessing-strategy-reservation-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let source_path = root.join("shared-source.wav");
        fs::write(&source_path, b"test source identity").unwrap();
        let source_sha256 = "ab".repeat(32);
        let generation = Generation::first();
        let jobs = (JobId(0x5301), JobId(0x5302));
        write_reprocessing_publications(
            &root,
            &source_path,
            &source_sha256,
            generation,
            jobs,
        );

        let catalog = ArchiveStore::reprocessing_catalog(&root, &source_sha256).unwrap();
        assert_eq!(catalog.len(), 2);
        let canonical_destination = root.canonicalize().unwrap().to_string_lossy().into_owned();
        assert_reprocessing_reservations(
            &root,
            &source_path,
            &source_sha256,
            generation,
            &catalog,
            &canonical_destination,
            jobs,
        );
        let mut changed_index = read_publication_index(&root).unwrap();
        changed_index
            .records
            .iter_mut()
            .find(|record| record.job_id == jobs.0)
            .unwrap()
            .active = false;
        write_publication_index(&root, &changed_index).unwrap();
        let changed_request = ImportRequest {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1,
            job_id: JobId(0x5305),
            generation,
            source_path: source_path.to_string_lossy().into_owned(),
            source_sha256: Some([0xab; 32]),
            source_samples: Some(16_000),
            model_path: "test-model".into(),
            model_sha256: [0; 32],
            destination: canonical_destination,
            config: whisper_core::JobConfig {
                language: whisper_core::LanguageChoice::Automatic,
                compute: whisper_core::ComputeChoice::Cpu,
            },
        };
        assert!(
            ArchiveStore::reserve_reprocessing_import(
                &root,
                &changed_request,
                &source_sha256,
                16_000,
                &catalog,
                &[TreatmentIdentity {
                    destination: root.canonicalize().unwrap().to_string_lossy().into_owned(),
                    job_id: jobs.0,
                    generation,
                }],
                DuplicateImportChoice::ReplaceLatest,
            )
            .unwrap_err()
            .contains("DuplicateStateChanged"),
            "a changed publication catalog must still be refused"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[cfg(windows)]
    fn write_reprocessing_publications(
        root: &Path,
        source_path: &Path,
        source_sha256: &str,
        generation: Generation,
        jobs: (JobId, JobId),
    ) {
        let mut index_records = Vec::new();
        for (job_id, strategy) in [
            (
                jobs.0,
                whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1,
            ),
            (
                jobs.1,
                whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
            ),
        ] {
            let directory = generation_path(root, job_id, generation);
            fs::create_dir_all(&directory).unwrap();
            let text = format!("strategy-specific result for {}", job_id.0);
            let srt = format!("1\n00:00:00,000 --> 00:00:01,000\n{text}\n");
            fs::write(directory.join("transcript.txt"), text.as_bytes()).unwrap();
            fs::write(directory.join("transcript.srt"), srt.as_bytes()).unwrap();
            let mut segments = serde_json::to_vec(&SegmentRecord {
                sequence: 1,
                range: SourceRange::new(0, 16_000, 16_000).unwrap(),
                text: text.clone(),
            })
            .unwrap();
            segments.push(b'\n');
            fs::write(directory.join("segments.jsonl"), segments).unwrap();

            let bounded = strategy
                == whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1;
            let (model_id, version) = if bounded {
                (whisper_core::model_catalog::ModelId::ParakeetTdt06BV3, 4)
            } else {
                (whisper_core::model_catalog::ModelId::WhisperLargeV3Turbo, 1)
            };
            let model = whisper_core::model_registry::model(model_id).artifact;
            let pending = PendingRecord {
                version,
                job_id,
                generation,
                source_path: source_path.to_string_lossy().into_owned(),
                source_sha256: source_sha256.to_owned(),
                source_samples: 16_000,
                model_path: model.file_name.into(),
                model_sha256: model.sha256.into(),
                model_size: model.size_bytes,
                strategy,
                qualification: None,
            };
            fs::write(
                directory.join("pending.json"),
                serde_json::to_vec(&pending).unwrap(),
            )
            .unwrap();
            let manifest = CompleteRecord {
                version: 1,
                job_id,
                generation,
                source_sha256: source_sha256.to_owned(),
                segments: 1,
                text: verified_artifact(&directory, &directory.join("transcript.txt")).unwrap(),
                srt: verified_artifact(&directory, &directory.join("transcript.srt")).unwrap(),
            };
            fs::write(
                directory.join("manifest.json"),
                serde_json::to_vec(&manifest).unwrap(),
            )
            .unwrap();
            fs::write(directory.join("CURRENT"), b"manifest.json\n").unwrap();
            let success_ordinal = index_records.len() as u64 + 1;
            index_records.push(PublicationRecord {
                job_id,
                generation,
                source_sha256: source_sha256.to_owned(),
                export_basename: Some(format!("result-{}", job_id.0)),
                active: true,
                success_ordinal: Some(success_ordinal),
            });
        }
        write_publication_index(
            root,
            &PublicationIndex {
                version: 1,
                records: index_records,
            },
        )
        .unwrap();
    }

    #[cfg(windows)]
    fn assert_reprocessing_reservations(
        root: &Path,
        source_path: &Path,
        source_sha256: &str,
        generation: Generation,
        catalog: &[PublishedImport],
        canonical_destination: &str,
        jobs: (JobId, JobId),
    ) {
        for (job_id, strategy, request_job) in [
            (
                jobs.0,
                whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1,
                JobId(0x5303),
            ),
            (
                jobs.1,
                whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
                JobId(0x5304),
            ),
        ] {
            let selected = TreatmentIdentity {
                destination: canonical_destination.to_owned(),
                job_id,
                generation,
            };
            let request = ImportRequest {
                strategy,
                job_id: request_job,
                generation,
                source_path: source_path.to_string_lossy().into_owned(),
                source_sha256: Some([0xab; 32]),
                source_samples: Some(16_000),
                model_path: "test-model".into(),
                model_sha256: [0; 32],
                destination: canonical_destination.to_owned(),
                config: whisper_core::JobConfig {
                    language: whisper_core::LanguageChoice::Automatic,
                    compute: whisper_core::ComputeChoice::Cpu,
                },
            };
            ArchiveStore::reserve_reprocessing_import(
                root,
                &request,
                source_sha256,
                16_000,
                catalog,
                std::slice::from_ref(&selected),
                DuplicateImportChoice::ReplaceLatest,
            )
            .unwrap();
            let intent: PublicationIntent = serde_json::from_slice(
                &fs::read(publication_reservation_path(root, request_job, generation)).unwrap(),
            )
            .unwrap();
            let expected = if strategy
                == whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
            {
                jobs.0
            } else {
                jobs.1
            };
            assert_eq!(
                intent.expected_active,
                vec![PublicationIdentity {
                    job_id: expected,
                    generation,
                }],
                "reprocessing reservations must snapshot only publications of their strategy"
            );
        }
    }

    #[test]
    fn canonical_generation_must_be_contained_by_archive_root() {
        let root = std::env::temp_dir().join(format!(
            "whisper-generation-containment-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let archive_root = root.join("archive");
        let inside_generation = archive_root.join("transcriptions/job/1");
        let outside_generation = root.join("aliased-generation");
        fs::create_dir_all(&inside_generation).unwrap();
        fs::create_dir_all(&outside_generation).unwrap();

        let canonical_root = archive_root.canonicalize().unwrap();
        assert!(
            canonical_generation_directory(&archive_root, &inside_generation)
                .unwrap()
                .starts_with(&canonical_root)
        );
        assert!(
            canonical_generation_directory(&archive_root, &outside_generation)
                .unwrap_err()
                .contains("generation directory escapes archive root")
        );
        let resolved_alias = inside_generation.join("../../../../aliased-generation");
        assert!(
            canonical_generation_directory(&archive_root, &resolved_alias)
                .unwrap_err()
                .contains("generation directory escapes archive root"),
            "a path resolving outside the archive root must fail closed"
        );

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn unified_archive_pages_cap_text_and_audio_entries_at_128_with_real_cursors() {
        use whisper_core::ports::ParakeetDocumentPort;

        let root = std::env::temp_dir().join(format!(
            "whisper-unified-pages-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let job_id = JobId(9_801);
        let generation = Generation::first();
        let generation_directory = generation_path(&root, job_id, generation);
        let document_directory = generation_directory.join("parakeet-unified/document");
        let mut document = crate::parakeet_document::ParakeetDocumentStore::create(
            &document_directory,
            job_id,
            generation,
        )
        .unwrap();
        for sequence in 1..=130_u64 {
            document
                .append_final(WorkerSegment {
                    job_id,
                    generation,
                    instance_id: 1,
                    segment_id: whisper_core::SegmentId(sequence),
                    range: SourceRange::new((sequence - 1) * 16_000, sequence * 16_000, 16_000)
                        .unwrap(),
                    text: format!("segment-{sequence}"),
                })
                .unwrap();
        }

        let document_first =
            ArchiveStore::read_unified_document_page(&root, job_id, generation, 130, 1, 512)
                .unwrap();
        assert_eq!(document_first.entries.len(), 128);
        assert_eq!(
            document_first.entries.first().unwrap().segment.segment_id.0,
            1
        );
        assert_eq!(
            document_first.entries.last().unwrap().segment.segment_id.0,
            128
        );
        assert_eq!(document_first.next_sequence, Some(129));
        let document_last = ArchiveStore::read_unified_document_page(
            &root,
            job_id,
            generation,
            130,
            document_first.next_sequence.unwrap(),
            512,
        )
        .unwrap();
        assert_eq!(document_last.entries.len(), 2);
        assert_eq!(document_last.entries[0].segment.segment_id.0, 129);
        assert_eq!(document_last.entries[1].segment.segment_id.0, 130);
        assert_eq!(document_last.next_sequence, None);

        let source_path = generation_directory.join("parakeet-unified/source.wav");
        fs::create_dir_all(source_path.parent().unwrap()).unwrap();
        fs::write(&source_path, b"").unwrap();
        let mut index = File::create(source_path.with_extension("parts.idx")).unwrap();
        index.write_all(b"PUAUD001").unwrap();
        for number in 1..=130_u64 {
            index.write_all(&number.to_le_bytes()).unwrap();
            index
                .write_all(&((number - 1) * 16_000).to_le_bytes())
                .unwrap();
            index.write_all(&(number * 16_000).to_le_bytes()).unwrap();
            let part_path = if number == 1 {
                source_path.clone()
            } else {
                source_path.with_file_name(format!("source.part-{number:06}.wav"))
            };
            fs::write(part_path, b"").unwrap();
        }
        index.sync_all().unwrap();
        drop(index);

        assert_audio_part_pages(&root, job_id, generation);

        fs::remove_dir_all(root).unwrap();
    }

    fn assert_audio_part_pages(root: &Path, job_id: JobId, generation: Generation) {
        let audio_first =
            ArchiveStore::read_unified_audio_parts_page(root, job_id, generation, 0, 0, 512)
                .unwrap();
        assert_eq!(audio_first.entries.len(), 128);
        assert_eq!(audio_first.entries.first().unwrap().number, 1);
        assert_eq!(audio_first.entries.last().unwrap().number, 128);
        assert_eq!(audio_first.next_offset, Some(128));
        let audio_last = ArchiveStore::read_unified_audio_parts_page(
            root,
            job_id,
            generation,
            audio_first.revision,
            audio_first.next_offset.unwrap(),
            512,
        )
        .unwrap();
        assert_eq!(audio_last.entries.len(), 2);
        assert_eq!(audio_last.entries[0].number, 129);
        assert_eq!(audio_last.entries[1].number, 130);
        assert_eq!(audio_last.next_offset, None);
        assert!(
            audio_last
                .entries
                .iter()
                .all(|part| Path::new(&part.path).is_file())
        );
    }

    #[test]
    fn pending_archive_versions_bind_strategy_and_keep_legacy_default() {
        let job_id = JobId(901);
        let generation = Generation::first();
        let mut legacy: PendingRecord = serde_json::from_value(serde_json::json!({
            "version": 1, "job_id": job_id, "generation": generation,
            "source_path": "source.wav", "source_sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "source_samples": 16000, "model_path": "turbo.bin",
            "model_sha256": APPROVED_MODEL_SHA256, "model_size": APPROVED_MODEL_SIZE,
            "qualification": null
        })).unwrap();
        assert_eq!(legacy.strategy, legacy_transcription_strategy());
        assert!(validate_pending(&legacy, job_id, generation.get()).is_ok());
        legacy.strategy = whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1;
        assert!(validate_pending(&legacy, job_id, generation.get()).is_err());

        let parakeet = whisper_core::model_registry::model(
            whisper_core::model_catalog::ModelId::ParakeetTdt06BV3,
        );
        let unified = PendingRecord {
            version: 4,
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1,
            job_id,
            generation,
            source_path: "source.wav".into(),
            source_sha256: "a".repeat(64),
            source_samples: 121 * 16_000,
            model_path: parakeet.artifact.file_name.into(),
            model_sha256: parakeet.artifact.sha256.into(),
            model_size: parakeet.artifact.size_bytes,
            qualification: None,
        };
        assert!(validate_pending(&unified, job_id, generation.get()).is_ok());
        let mut missing = serde_json::to_value(&unified).unwrap();
        missing.as_object_mut().unwrap().remove("strategy");
        let missing: PendingRecord = serde_json::from_value(missing).unwrap();
        assert!(validate_pending(&missing, job_id, generation.get()).is_err());
        let mut mismatched = unified;
        mismatched.strategy = whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1;
        assert!(validate_pending(&mismatched, job_id, generation.get()).is_err());
    }

    #[test]
    fn legacy_cleanup_target_without_container_proof_deserializes_without_parent_authority() {
        let legacy = serde_json::json!({
            "treatment": {
                "destination": "C:\\\\legacy-destination",
                "job_id": 7,
                "generation": 1
            },
            "source_sha256": "00",
            "ancestors": [],
            "objects": []
        });
        let target: CleanupTarget = serde_json::from_value(legacy).unwrap();

        assert!(target.container.is_none());
        assert!(
            cleanup_targets_match_intent(
                std::slice::from_ref(&target),
                std::slice::from_ref(&target)
            ),
            "legacy intent and plan remain compatible without adding container ownership"
        );
    }

    #[cfg(feature = "recette-hooks")]
    #[test]
    fn campaign_snapshot_skips_an_active_writer_lock() {
        let root = std::env::temp_dir().join(format!(
            "whisper-campaign-snapshot-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let archive = root.join("archive");
        fs::create_dir_all(&archive).unwrap();
        let lock = File::create(archive.join("group-1.writer.lock")).unwrap();
        lock.try_lock().unwrap();
        fs::write(archive.join("passage-1.pcm"), b"durable pcm").unwrap();

        let snapshot = campaign_archive_snapshot(&root).unwrap();
        assert_eq!(snapshot.len(), 1);
        assert_eq!(snapshot[0]["path"], "passage-1.pcm");
        assert_eq!(snapshot[0]["bytes"], 11);

        lock.unlock().unwrap();
        drop(lock);
        fs::remove_dir_all(root).unwrap();
    }
