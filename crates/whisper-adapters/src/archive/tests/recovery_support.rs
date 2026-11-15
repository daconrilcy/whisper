struct R7RecoveryScenario {
    root: PathBuf,
    job_id: JobId,
    generation: Generation,
    directory: PathBuf,
    text: &'static [u8],
    srt: &'static [u8],
    previous: ImportRequest,
    prior_publications: Vec<PublishedImport>,
    intent: PublicationIntent,
    exports: PathBuf,
    third_party: PathBuf,
    third_party_bytes: Vec<u8>,
}

impl R7RecoveryScenario {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "whisper-r7-export-recovery-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let job_id = JobId(0x7711);
        let generation = Generation::first();
        let text = b"R7 transcript";
        let srt = b"1\n00:00:00,000 --> 00:00:01,000\nR7 transcript\n";
        let (directory, manifest) = r7_create_current_archive(&root, job_id, generation, text, srt);
        let prior_publications = r7_create_previous_publication(&root, &manifest);
        let (intent, previous) =
            r7_create_current_intent(&root, job_id, generation, &manifest, &prior_publications);
        let exports = root.join("exports");
        let third_party = exports.join("r7-audio.srt");
        Self {
            root,
            job_id,
            generation,
            directory,
            text,
            srt,
            previous,
            prior_publications,
            intent,
            exports,
            third_party,
            third_party_bytes: srt.to_vec(),
        }
    }

    fn assert_interrupted_collision(&self) -> String {
        let no_hard_links = r7_no_hard_links;
        let calls = std::cell::Cell::new(0);
        let third_party = &self.third_party;
        let third_party_bytes = &self.third_party_bytes;
        let fail_links_and_collide_after_txt = |_: &Path, _target: &Path| {
            let call = calls.get() + 1;
            calls.set(call);
            if call == 4 {
                fs::write(third_party, third_party_bytes)?;
            }
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "test filesystem does not support hard links",
            ))
        };
        let first = install_publication_exports_with(
            &self.root,
            &self.directory,
            &self.intent,
            &fail_links_and_collide_after_txt,
        );
        let error = first.unwrap_err();
        assert!(error.contains("export name collision for srt"), "{error}");
        assert_eq!(
            fs::read(self.exports.join("r7-audio.txt")).unwrap(),
            self.text
        );
        let third_party_identity = persisted_file_identity(&self.third_party).unwrap();
        assert_eq!(fs::read(&self.third_party).unwrap(), self.third_party_bytes);
        assert_eq!(
            persisted_file_identity(&self.third_party).unwrap(),
            third_party_identity
        );
        assert!(!self.directory.join("CURRENT").exists());

        // Inject a process interruption immediately after the exclusive copy
        // creates the final SRT, before its file identity can be persisted.
        fs::remove_file(&self.third_party).unwrap();
        let interrupted_copy = |extension: &str, _path: &Path| {
            if extension == "srt" {
                Err("test process interruption after create_new".into())
            } else {
                Ok(())
            }
        };
        let create_new_cut = install_publication_exports_with_copy_hook(
            &self.root,
            &self.directory,
            &self.intent,
            &no_hard_links,
            &interrupted_copy,
        )
        .unwrap_err();
        assert!(create_new_cut.contains("test process interruption"));
        assert_eq!(fs::read(&self.third_party).unwrap(), self.third_party_bytes);
        let unrecorded_identity = persisted_file_identity(&self.third_party).unwrap();
        let recovery_error = install_publication_exports_with(
            &self.root,
            &self.directory,
            &self.intent,
            &no_hard_links,
        )
        .unwrap_err();
        assert!(
            recovery_error.contains("export name collision for srt"),
            "{recovery_error}"
        );
        assert_eq!(fs::read(&self.third_party).unwrap(), self.third_party_bytes);
        assert_eq!(
            persisted_file_identity(&self.third_party).unwrap(),
            unrecorded_identity
        );
        assert!(!self.directory.join("CURRENT").exists());
        assert_eq!(
            ArchiveStore::publication_reconfirmation_required(
                &self.root,
                self.job_id,
                self.generation
            )
            .unwrap(),
            Some("r7-audio".into())
        );
        unrecorded_identity
    }

    fn reconfirm_publication(&self) -> PublicationIntent {
        let new_basename =
            ArchiveStore::suggest_duplicate_export_basename(&self.root, &self.previous.source_path)
                .unwrap();
        assert_eq!(new_basename, "r7-audio (2)");
        ArchiveStore::reconfirm_interrupted_publication(
            &self.root,
            &self.previous,
            PublicationReconfirmation {
                source_sha256: &self.intent.source_sha256,
                source_samples: self.intent.source_samples,
                expected_active: &self.prior_publications,
                choice: DuplicateImportChoice::Replace,
                reserved_basename: "r7-audio",
                confirmed_basename: &new_basename,
            },
        )
        .unwrap();
        let confirmed_intent: PublicationIntent = serde_json::from_slice(
            &fs::read(publication_reservation_path(
                &self.root,
                self.job_id,
                self.generation,
            ))
            .unwrap(),
        )
        .unwrap();
        assert_eq!(confirmed_intent.export_basename, new_basename);
        confirmed_intent
    }

    fn commit_recovered_publication(
        &self,
        confirmed_intent: PublicationIntent,
        unrecorded_identity: String,
    ) {
        let no_hard_links = r7_no_hard_links;
        install_publication_exports_with(
            &self.root,
            &self.directory,
            &confirmed_intent,
            &no_hard_links,
        )
        .unwrap();
        assert_eq!(
            fs::read(self.exports.join("r7-audio (2).txt")).unwrap(),
            self.text
        );
        assert_eq!(
            fs::read(self.exports.join("r7-audio (2).srt")).unwrap(),
            self.srt
        );
        self.assert_third_party_file_unchanged(&unrecorded_identity);
        assert_eq!(
            fs::read(self.directory.join("transcript.txt")).unwrap(),
            self.text
        );
        assert_eq!(
            fs::read(self.directory.join("transcript.srt")).unwrap(),
            self.srt
        );
        assert_eq!(
            ArchiveStore::published_imports(&self.root, &self.intent.source_sha256).unwrap(),
            self.prior_publications,
            "the prior publication remains active until the recovered generation commits"
        );
        assert!(!self.directory.join("CURRENT").exists());

        // Repeating export installation is idempotent before the commit point.
        install_publication_exports_with(
            &self.root,
            &self.directory,
            &confirmed_intent,
            &no_hard_links,
        )
        .unwrap();
        assert_eq!(
            fs::read(self.exports.join("r7-audio (2).srt")).unwrap(),
            self.srt
        );
        r7_commit_archive_generation(self, &confirmed_intent);
        let active =
            ArchiveStore::published_imports(&self.root, &self.intent.source_sha256).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(
            (active[0].job_id, active[0].generation),
            (self.job_id, self.generation)
        );
        assert!(self.directory.join("CURRENT").is_file());
        assert_eq!(
            fs::read(self.exports.join("r7-audio (2).txt")).unwrap(),
            self.text
        );
        assert_eq!(
            fs::read(self.exports.join("r7-audio (2).srt")).unwrap(),
            self.srt
        );
        self.assert_third_party_file_unchanged(&unrecorded_identity);
    }

    fn assert_third_party_file_unchanged(&self, unrecorded_identity: &str) {
        assert_eq!(fs::read(&self.third_party).unwrap(), self.third_party_bytes);
        assert_eq!(
            persisted_file_identity(&self.third_party).unwrap(),
            unrecorded_identity
        );
    }
}

fn r7_no_hard_links(_: &Path, _: &Path) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "test filesystem does not support hard links",
    ))
}

fn r7_create_current_archive(
    root: &Path,
    job_id: JobId,
    generation: Generation,
    text: &[u8],
    srt: &[u8],
) -> (PathBuf, CompleteRecord) {
    let directory = generation_path(root, job_id, generation);
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join("transcript.txt"), text).unwrap();
    fs::write(directory.join("transcript.srt"), srt).unwrap();
    fs::write(directory.join("export.txt.pending"), text).unwrap();
    fs::write(directory.join("export.srt.pending"), srt).unwrap();
    let manifest = CompleteRecord {
        version: 1,
        job_id,
        generation,
        source_sha256: "77".repeat(32),
        segments: 1,
        text: r7_artifact_record("transcript.txt", text),
        srt: r7_artifact_record("transcript.srt", srt),
    };
    (directory, manifest)
}

fn r7_artifact_record(path: &str, content: &[u8]) -> ArtifactRecord {
    ArtifactRecord {
        path: path.to_owned(),
        sha256: hex(&Sha256::digest(content)),
        bytes: content.len() as u64,
    }
}

fn r7_create_previous_publication(root: &Path, manifest: &CompleteRecord) -> Vec<PublishedImport> {
    let previous_job_id = JobId(0x7710);
    let previous_generation = Generation::first();
    let previous_directory = generation_path(root, previous_job_id, previous_generation);
    fs::create_dir_all(&previous_directory).unwrap();
    let previous_text = b"previous publication";
    let previous_srt = b"1\n00:00:00,000 --> 00:00:01,000\nprevious publication\n";
    fs::write(previous_directory.join("transcript.txt"), previous_text).unwrap();
    fs::write(previous_directory.join("transcript.srt"), previous_srt).unwrap();
    let mut previous_segments = serde_json::to_vec(&SegmentRecord {
        sequence: 1,
        range: SourceRange::new(0, 16_000, 16_000).unwrap(),
        text: "previous publication".into(),
    })
    .unwrap();
    previous_segments.push(b'\n');
    fs::write(previous_directory.join("segments.jsonl"), previous_segments).unwrap();
    let previous_manifest = CompleteRecord {
        version: 1,
        job_id: previous_job_id,
        generation: previous_generation,
        source_sha256: manifest.source_sha256.clone(),
        segments: 1,
        text: r7_artifact_record("transcript.txt", previous_text),
        srt: r7_artifact_record("transcript.srt", previous_srt),
    };
    fs::write(
        previous_directory.join("manifest.json"),
        serde_json::to_vec(&previous_manifest).unwrap(),
    )
    .unwrap();
    fs::write(
        previous_directory.join("pending.json"),
        serde_json::to_vec(&PendingRecord {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
            version: 1,
            job_id: previous_job_id,
            generation: previous_generation,
            source_path: "r7-audio.wav".into(),
            source_sha256: manifest.source_sha256.clone(),
            source_samples: 16_000,
            model_path: "model.bin".into(),
            model_sha256: APPROVED_MODEL_SHA256.into(),
            model_size: APPROVED_MODEL_SIZE,
            qualification: None,
        })
        .unwrap(),
    )
    .unwrap();
    fs::write(previous_directory.join("CURRENT"), b"manifest.json\n").unwrap();
    write_publication_index(
        root,
        &PublicationIndex {
            version: 1,
            records: vec![PublicationRecord {
                job_id: previous_job_id,
                generation: previous_generation,
                source_sha256: manifest.source_sha256.clone(),
                export_basename: Some("r7-prior".into()),
                active: true,
                success_ordinal: None,
            }],
        },
    )
    .unwrap();
    let prior_publications =
        ArchiveStore::published_imports(root, &manifest.source_sha256).unwrap();
    assert_eq!(prior_publications.len(), 1);
    assert_eq!(prior_publications[0].job_id, previous_job_id);
    prior_publications
}

fn r7_create_current_intent(
    root: &Path,
    job_id: JobId,
    generation: Generation,
    manifest: &CompleteRecord,
    prior_publications: &[PublishedImport],
) -> (PublicationIntent, ImportRequest) {
    let directory = generation_path(root, job_id, generation);
    fs::write(
        directory.join("manifest.json"),
        serde_json::to_vec(manifest).unwrap(),
    )
    .unwrap();
    fs::write(
        directory.join("pending.json"),
        serde_json::to_vec(&PendingRecord {
            strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
            version: 1,
            job_id,
            generation,
            source_path: "r7-audio.wav".into(),
            source_sha256: manifest.source_sha256.clone(),
            source_samples: 16_000,
            model_path: "model.bin".into(),
            model_sha256: APPROVED_MODEL_SHA256.into(),
            model_size: APPROVED_MODEL_SIZE,
            qualification: None,
        })
        .unwrap(),
    )
    .unwrap();
    let intent = PublicationIntent {
        version: 1,
        operation_id: "r7-copy-fallback-recovery".into(),
        choice: DuplicateImportChoice::Replace,
        source_sha256: manifest.source_sha256.clone(),
        source_samples: 16_000,
        expected_active: prior_publications
            .iter()
            .map(|publication| PublicationIdentity {
                job_id: publication.job_id,
                generation: publication.generation,
            })
            .collect(),
        export_basename: "r7-audio".into(),
        internal_only: false,
        cleanup_id: None,
        cleanup_targets: Vec::new(),
    };
    let previous = ImportRequest {
        strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
        job_id,
        generation,
        source_path: "r7-audio.wav".into(),
        source_sha256: Some([0x77; 32]),
        source_samples: Some(16_000),
        model_path: "model.bin".into(),
        model_sha256: [0; 32],
        destination: root.to_string_lossy().into_owned(),
        config: whisper_core::JobConfig {
            language: whisper_core::LanguageChoice::Automatic,
            compute: whisper_core::ComputeChoice::Cpu,
        },
    };
    write_publication_reservation(
        root,
        &previous,
        &intent.source_sha256,
        intent.source_samples,
        prior_publications
            .iter()
            .map(|publication| PublicationIdentity {
                job_id: publication.job_id,
                generation: publication.generation,
            })
            .collect(),
        intent.choice.clone(),
        intent.export_basename.clone(),
    )
    .unwrap();
    fs::create_dir_all(root.join("exports")).unwrap();
    (intent, previous)
}

fn r7_commit_archive_generation(
    scenario: &R7RecoveryScenario,
    confirmed_intent: &PublicationIntent,
) {
    let mut segments = serde_json::to_vec(&SegmentRecord {
        sequence: 1,
        range: SourceRange::new(0, 16_000, 16_000).unwrap(),
        text: "R7 transcript".into(),
    })
    .unwrap();
    segments.push(b'\n');
    fs::write(scenario.directory.join("segments.jsonl"), segments).unwrap();
    fs::write(scenario.directory.join("CURRENT"), b"manifest.json\n").unwrap();
    let index = apply_publication_intent(
        read_publication_index(&scenario.root).unwrap(),
        scenario.job_id,
        scenario.generation,
        confirmed_intent,
    );
    write_publication_index(&scenario.root, &index).unwrap();
    reconcile_replaced_exports(&scenario.root, &index).unwrap();
}
