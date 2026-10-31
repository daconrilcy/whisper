use super::{
    ArchiveQualificationKey, ArchiveStore, ArtifactRecord, BufRead, BufReader, CompleteRecord,
    Digest, DuplicateImportChoice, File, Generation, JobId, PARAKEET_L2A_WORKER_SHA256,
    PARAKEET_L2B_RUNTIME_SHA256, PARAKEET_L2B_VAD_WORKER_SHA256, PARAKEET_R5_2_WORKER_SHA256,
    PCM_STAGING_HEADER, Path, PathBuf, PendingRecord, PublicationIntent, Read, Seek, SeekFrom,
    Sha256, cleanup_targets_match_intent, fs, generation_path, hex, read_cleanup_plan, sha256_file,
};

impl ArchiveQualificationKey {
    pub(super) fn permits_bounded_resume_to(&self, current: &Self) -> bool {
        self.model == current.model
            && self.artifact == current.artifact
            && self.artifact_revision == current.artifact_revision
            && self.worker_protocol == current.worker_protocol
            && self.inference_profile == current.inference_profile
    }

    pub fn parakeet_cpu_test() -> Self {
        let record = whisper_core::model_registry::model(
            whisper_core::model_catalog::ModelId::ParakeetTdt06BV3,
        );
        let key = whisper_core::model_registry::configuration_key(
            record,
            whisper_core::model_catalog::Backend::Cpu,
        );
        Self {
            model: "parakeet-tdt-0.6b-v3".into(),
            artifact: key.artifact,
            artifact_revision: key.artifact_revision,
            runtime: key.runtime,
            runtime_build: key.runtime_build,
            worker: key.worker,
            worker_build: key.worker_build,
            worker_protocol: key.worker_protocol,
            backend: "CPU".into(),
            inference_profile: key.inference_profile,
            gpu_pci_bus_id: None,
            gpu_uuid: None,
            gpu_pnp_instance_id: None,
        }
    }

    pub fn parakeet_gpu_test() -> Self {
        let mut key = Self::parakeet_cpu_test();
        key.runtime_build =
            "cbf7543a711a341df4387fb9047fcd55d3d5c572465cbe9bbb7d14b5963e6b10".into();
        key.worker_build =
            "5054612a05fbfd1039e108def3aa4e598268d2b4274b8c1dc96cd6b8e3f2cd6d".into();
        key.backend = "CUDA0".into();
        key.gpu_pci_bus_id =
            Some(whisper_core::model_registry::PARAKEET_TEST_GPU_PCI_BUS_ID.to_owned());
        key.gpu_uuid = Some(whisper_core::model_registry::PARAKEET_TEST_GPU_UUID.to_owned());
        key.gpu_pnp_instance_id =
            Some(whisper_core::model_registry::PARAKEET_TEST_GPU_PNP_INSTANCE_ID.to_owned());
        key
    }

    fn parakeet_personal(backend: whisper_core::model_catalog::Backend) -> Self {
        let record = whisper_core::model_registry::model(
            whisper_core::model_catalog::ModelId::ParakeetTdt06BV3,
        );
        let configuration =
            whisper_core::model_registry::personal_configuration_key(record, backend);
        let mut key = Self::parakeet_cpu_test();
        key.artifact_revision = configuration.artifact_revision;
        key.runtime_build = configuration.runtime_build;
        key.worker_build = configuration.worker_build;
        key.backend = match backend {
            whisper_core::model_catalog::Backend::Cpu => "CPU",
            whisper_core::model_catalog::Backend::Gpu => "CUDA0",
        }
        .to_owned();
        key.inference_profile = configuration.inference_profile;
        if backend == whisper_core::model_catalog::Backend::Gpu {
            key.gpu_pci_bus_id =
                Some(whisper_core::model_registry::PARAKEET_PERSONAL_GPU_PCI_BUS_ID.to_owned());
            key.gpu_uuid =
                Some(whisper_core::model_registry::PARAKEET_PERSONAL_GPU_UUID.to_owned());
            key.gpu_pnp_instance_id = Some(
                whisper_core::model_registry::PARAKEET_PERSONAL_GPU_PNP_INSTANCE_ID.to_owned(),
            );
        }
        key
    }

    pub fn is_registered_personal_import(&self) -> bool {
        let cpu = Self::parakeet_personal(whisper_core::model_catalog::Backend::Cpu);
        let gpu = Self::parakeet_personal(whisper_core::model_catalog::Backend::Gpu);
        let cpu_ready = cpu.worker_build.len() == 64 && cpu.runtime_build.len() == 64;
        let gpu_ready = gpu.worker_build.len() == 64 && gpu.runtime_build.len() == 64;
        (cpu_ready && self == &cpu) || (gpu_ready && self == &gpu)
    }

    pub(crate) fn is_personal_candidate(&self) -> bool {
        #[cfg(feature = "personal-qualification")]
        {
            return crate::parakeet_import::ParakeetWorkerPaths::personal_candidate_keys_from_env()
                .iter()
                .any(|candidate| candidate == self);
        }
        #[cfg(not(feature = "personal-qualification"))]
        false
    }

    pub fn is_parakeet_import_test(&self) -> bool {
        let expected = Self::parakeet_cpu_test();
        let gpu = Self::parakeet_gpu_test();
        let build_matches_profile = match self.inference_profile.as_str() {
            "import-single-pass" => {
                self.runtime_build == expected.runtime_build
                    && self.worker_build == expected.worker_build
            }
            whisper_core::parakeet_windows::PROFILE_IMPORT_VAD => {
                self.runtime_build == expected.runtime_build
                    && self.runtime_build == PARAKEET_L2B_RUNTIME_SHA256
                    && self.worker_build == PARAKEET_L2B_VAD_WORKER_SHA256
            }
            _ => false,
        };
        let cpu_match = self.model == expected.model
            && self.artifact == expected.artifact
            && self.artifact_revision == expected.artifact_revision
            && self.runtime == expected.runtime
            && self.worker == expected.worker
            && build_matches_profile
            && self.worker_protocol == expected.worker_protocol
            && self.backend == "CPU";
        let gpu_match = self.model == gpu.model
            && self.artifact == gpu.artifact
            && self.artifact_revision == gpu.artifact_revision
            && self.runtime == gpu.runtime
            && self.runtime_build == gpu.runtime_build
            && self.worker == gpu.worker
            && self.worker_build == gpu.worker_build
            && self.worker_protocol == gpu.worker_protocol
            && self.backend == gpu.backend
            && self.inference_profile == "import-single-pass"
            && self.gpu_pci_bus_id == gpu.gpu_pci_bus_id
            && self.gpu_uuid == gpu.gpu_uuid
            && self.gpu_pnp_instance_id == gpu.gpu_pnp_instance_id;
        cpu_match
            || gpu_match
            || self.is_registered_personal_import()
            || self.is_personal_candidate()
    }

    pub fn is_parakeet_vad_test(&self) -> bool {
        self.is_parakeet_import_test()
            && self.inference_profile == whisper_core::parakeet_windows::PROFILE_IMPORT_VAD
    }

    pub(super) fn is_retired_l2a_key(&self) -> bool {
        self.model == "parakeet-tdt-0.6b-v3"
            && self.artifact == "parakeet-tdt-0.6b-v3.q8_0.gguf"
            && self.artifact_revision
                == whisper_core::model_registry::model(
                    whisper_core::model_catalog::ModelId::ParakeetTdt06BV3,
                )
                .artifact
                .sha256
            && self.runtime == "nemo-speech-asr 0.2.0"
            && self.runtime_build == PARAKEET_L2B_RUNTIME_SHA256
            && self.worker == "parakeet-worker"
            && self.worker_build == PARAKEET_L2A_WORKER_SHA256
            && self.worker_protocol == 2
            && self.backend == "CPU"
            && self.inference_profile == "import-single-pass"
    }

    pub(super) fn is_retired_r5_2_key(&self) -> bool {
        self.model == "parakeet-tdt-0.6b-v3"
            && self.artifact == "parakeet-tdt-0.6b-v3.q8_0.gguf"
            && self.artifact_revision
                == whisper_core::model_registry::model(
                    whisper_core::model_catalog::ModelId::ParakeetTdt06BV3,
                )
                .artifact
                .sha256
            && self.runtime == "nemo-speech-asr 0.2.0"
            && self.runtime_build == PARAKEET_L2B_RUNTIME_SHA256
            && self.worker == "parakeet-worker"
            && self.worker_build == PARAKEET_R5_2_WORKER_SHA256
            && self.worker_protocol == 2
            && self.backend == "CPU"
            && self.inference_profile == "import-single-pass"
    }
}

impl ArchiveStore {
    pub fn job_identity_exists(root: &Path, job_id: JobId, generation: Generation) -> bool {
        root.join("transcriptions")
            .join(format!("{:032x}", job_id.0))
            .join(generation.get().to_string())
            .exists()
    }

    pub fn read_unified_document_page(
        root: &Path,
        job_id: JobId,
        generation: Generation,
        expected_revision: u64,
        first_sequence: u64,
        limit: usize,
    ) -> Result<whisper_core::ports::ParakeetDocumentPage, String> {
        const INDEX_HEADER: &[u8; 8] = b"PUDOC001";
        const INDEX_ENTRY_BYTES: u64 = 16;
        if first_sequence == 0 || limit == 0 {
            return Err("InvalidInput: document page cursor or limit".into());
        }
        let directory = generation_path(root, job_id, generation).join("parakeet-unified/document");
        let mut index = File::open(directory.join("document.idx"))
            .map_err(|error| format!("DocumentUnavailable: {error}"))?;
        let mut header = [0_u8; 8];
        index
            .read_exact(&mut header)
            .map_err(|error| format!("DocumentIndex: {error}"))?;
        let index_size = index
            .metadata()
            .map_err(|error| format!("DocumentIndex: {error}"))?
            .len();
        if &header != INDEX_HEADER
            || index_size < INDEX_HEADER.len() as u64
            || !(index_size - INDEX_HEADER.len() as u64).is_multiple_of(INDEX_ENTRY_BYTES)
        {
            return Err("StorageCorrupt: unified document index".into());
        }
        let revision = (index_size - INDEX_HEADER.len() as u64) / INDEX_ENTRY_BYTES;
        if revision != expected_revision {
            return Err("StaleResponse: unified document revision changed".into());
        }
        let limit = limit.min(128);
        let mut document = BufReader::new(
            File::open(directory.join("document.jsonl"))
                .map_err(|error| format!("DocumentUnavailable: {error}"))?,
        );
        let mut entries = Vec::with_capacity(limit);
        let mut sequence = first_sequence;
        while sequence <= revision && entries.len() < limit {
            let record_offset = INDEX_HEADER.len() as u64
                + (sequence - 1)
                    .checked_mul(INDEX_ENTRY_BYTES)
                    .ok_or_else(|| "DocumentIndex: sequence overflow".to_owned())?;
            index
                .seek(SeekFrom::Start(record_offset))
                .map_err(|error| error.to_string())?;
            let mut raw = [0_u8; 16];
            index
                .read_exact(&mut raw)
                .map_err(|error| format!("StorageCorrupt: document index record: {error}"))?;
            let offset = u64::from_le_bytes(raw[..8].try_into().unwrap());
            let length = u64::from_le_bytes(raw[8..].try_into().unwrap());
            let line_length = usize::try_from(length)
                .map_err(|_| "StorageCorrupt: document record length".to_owned())?;
            if line_length == 0 || line_length > 1_048_576 {
                return Err("StorageCorrupt: document record length".into());
            }
            document
                .seek(SeekFrom::Start(offset))
                .map_err(|error| error.to_string())?;
            let mut line = String::with_capacity(line_length);
            document
                .by_ref()
                .take(length)
                .read_line(&mut line)
                .map_err(|error| error.to_string())?;
            if line.len() != line_length {
                return Err("StorageCorrupt: short unified document record".into());
            }
            let entry: whisper_core::ports::ParakeetDocumentEntry =
                serde_json::from_str(line.trim_end())
                    .map_err(|error| format!("StorageCorrupt: unified document entry: {error}"))?;
            if entry.segment.job_id != job_id
                || entry.segment.generation != generation
                || entry.segment.segment_id.0 != sequence
                || entry.revision != sequence
            {
                return Err("StaleResponse: unified document identity changed".into());
            }
            entries.push(entry);
            sequence += 1;
        }
        Ok(whisper_core::ports::ParakeetDocumentPage {
            revision,
            entries,
            next_sequence: (sequence <= revision).then_some(sequence),
        })
    }

    pub fn read_unified_audio_parts_page(
        root: &Path,
        job_id: JobId,
        generation: Generation,
        expected_revision: u64,
        offset: u64,
        limit: usize,
    ) -> Result<whisper_core::ports::UnifiedAudioPartPage, String> {
        if limit == 0 {
            return Err("InvalidInput: audio part page limit".into());
        }
        let source_path =
            generation_path(root, job_id, generation).join("parakeet-unified/source.wav");
        let index_path = source_path.with_extension("parts.idx");
        let before =
            sha256_file(&index_path).map_err(|error| format!("AudioPartsUnavailable: {error}"))?;
        let index_bytes = fs::metadata(&index_path)
            .map_err(|error| format!("AudioPartsUnavailable: {error}"))?
            .len();
        const INDEX_HEADER_BYTES: u64 = 8;
        const INDEX_RECORD_BYTES: u64 = 24;
        if index_bytes < INDEX_HEADER_BYTES
            || !(index_bytes - INDEX_HEADER_BYTES).is_multiple_of(INDEX_RECORD_BYTES)
        {
            return Err("StorageCorrupt: unified audio part index length".into());
        }
        let part_count = (index_bytes - INDEX_HEADER_BYTES) / INDEX_RECORD_BYTES;
        let revision = u64::from_le_bytes(before[..8].try_into().unwrap()).max(1);
        if expected_revision != 0 && expected_revision != revision {
            return Err("StaleResponse: unified audio parts changed".into());
        }
        let store = crate::parakeet_audio::PcmPartStore::open(&source_path);
        let page = store
            .read_page(offset, limit.min(128))
            .map_err(|error| format!("StorageCorrupt: audio part index: {error:?}"))?;
        if sha256_file(&index_path).map_err(|error| error.to_string())? != before {
            return Err("StaleResponse: unified audio parts changed during read".into());
        }
        let entries = page
            .into_iter()
            .map(|part| whisper_core::ports::UnifiedAudioPart {
                number: part.number,
                start_sample: part.start_sample,
                end_sample: part.end_sample,
                path: part.path.to_string_lossy().into_owned(),
            })
            .collect::<Vec<_>>();
        if entries.iter().any(|part| !Path::new(&part.path).is_file()) {
            return Err("StorageCorrupt: indexed unified audio part is missing".into());
        }
        let next = offset.saturating_add(entries.len() as u64);
        let next_offset = (next < part_count).then_some(next);
        Ok(whisper_core::ports::UnifiedAudioPartPage {
            revision,
            offset,
            entries,
            next_offset,
        })
    }

    pub fn published_unified_files(
        root: &Path,
        job_id: JobId,
        generation: Generation,
    ) -> Result<whisper_core::ports::UnifiedPublicationFiles, String> {
        let directory = generation_path(root, job_id, generation);
        let current = fs::read(directory.join("CURRENT"))
            .map_err(|error| format!("PublicationUnavailable: CURRENT: {error}"))?;
        if current != b"manifest.json\n" {
            return Err("PublicationUnavailable: unified session is not CURRENT".into());
        }
        let manifest: CompleteRecord = serde_json::from_slice(
            &fs::read(directory.join("manifest.json"))
                .map_err(|error| format!("PublicationUnavailable: manifest: {error}"))?,
        )
        .map_err(|error| format!("StorageCorrupt: unified manifest: {error}"))?;
        if manifest.job_id != job_id || manifest.generation != generation {
            return Err("StaleResponse: unified publication identity changed".into());
        }
        let intent: PublicationIntent = serde_json::from_slice(
            &fs::read(directory.join("r11-publication.json"))
                .map_err(|error| format!("PublicationUnavailable: export intent: {error}"))?,
        )
        .map_err(|error| format!("StorageCorrupt: unified export intent: {error}"))?;
        let wav_path = directory.join("parakeet-unified/source.wav");
        let (wav_path, txt_path, srt_path) = if intent.internal_only {
            let canonical_directory = canonical_generation_directory(root, &directory)?;
            validate_internal_publication_intent(
                root,
                job_id,
                generation,
                &intent,
                &manifest.source_sha256,
            )?;
            let txt_path = verify_internal_unified_artifact(
                &directory,
                &canonical_directory,
                &manifest.text,
                "transcript.txt",
            )?;
            let srt_path = verify_internal_unified_artifact(
                &directory,
                &canonical_directory,
                &manifest.srt,
                "transcript.srt",
            )?;
            let wav_path = canonical_internal_unified_path(
                &wav_path,
                &canonical_directory,
                "parakeet-unified/source.wav",
            )?;
            let index_path = canonical_internal_unified_path(
                &wav_path.with_extension("parts.idx"),
                &canonical_directory,
                "parakeet-unified/source.parts.idx",
            )?;
            let index_before = sha256_file(&index_path)
                .map_err(|error| format!("AudioPartsUnavailable: {error}"))?;
            let audio = crate::parakeet_audio::PcmPartStore::open(&wav_path);
            let total_samples = audio
                .total_samples()
                .map_err(|error| format!("AudioPartsUnavailable: {error}"))?;
            if total_samples == 0 || total_samples != intent.source_samples {
                return Err(
                    "StorageCorrupt: internal audio index total does not match publication".into(),
                );
            }
            let mut offset = 0_u64;
            let mut covered = 0_u64;
            let mut expected_number = 1_u64;
            let mut first_page = true;
            loop {
                let page =
                    Self::read_unified_audio_parts_page(root, job_id, generation, 0, offset, 128)?;
                if page.entries.is_empty() {
                    break;
                }
                for part in &page.entries {
                    if part.number != expected_number
                        || part.start_sample != covered
                        || part.end_sample <= part.start_sample
                        || part.end_sample > total_samples
                    {
                        return Err(
                            "StorageCorrupt: internal audio parts are not contiguous".into()
                        );
                    }
                    let part_path = canonical_internal_unified_path(
                        Path::new(&part.path),
                        &canonical_directory,
                        if expected_number == 1 {
                            "parakeet-unified/source.wav"
                        } else {
                            "parakeet-unified/source part"
                        },
                    )?;
                    if first_page && expected_number == 1 && part_path != wav_path {
                        return Err(
                            "StorageCorrupt: first internal audio part is not source.wav".into(),
                        );
                    }
                    first_page = false;
                    covered = part.end_sample;
                    expected_number = expected_number.saturating_add(1);
                }
                offset = offset
                    .checked_add(page.entries.len() as u64)
                    .ok_or_else(|| {
                        "StorageCorrupt: internal audio page offset overflow".to_owned()
                    })?;
                if page.next_offset.is_none() {
                    break;
                }
            }
            if covered != total_samples || sha256_file(&index_path)? != index_before {
                return Err(
                    "SourceChanged: internal audio index changed during publication read".into(),
                );
            }
            (wav_path, txt_path, srt_path)
        } else {
            let txt_path = root
                .join("exports")
                .join(format!("{}.txt", intent.export_basename));
            let srt_path = root
                .join("exports")
                .join(format!("{}.srt", intent.export_basename));
            verify_artifact(&directory, &manifest.text)?;
            verify_artifact(&directory, &manifest.srt)?;
            if !wav_path.with_extension("parts.idx").is_file()
                || !txt_path.is_file()
                || !srt_path.is_file()
            {
                return Err(
                    "PublicationUnavailable: unified audio or TXT/SRT reference is missing".into(),
                );
            }
            (wav_path, txt_path, srt_path)
        };
        Ok(whisper_core::ports::UnifiedPublicationFiles {
            wav_path: wav_path.to_string_lossy().into_owned(),
            txt_path: txt_path.to_string_lossy().into_owned(),
            srt_path: srt_path.to_string_lossy().into_owned(),
        })
    }
}

pub(super) fn is_registered_model(hash: &str, size: u64) -> bool {
    whisper_core::model_registry::MODELS.iter().any(|record| {
        record.artifact.sha256.eq_ignore_ascii_case(hash) && record.artifact.size_bytes == size
    })
}

pub(super) fn safe_artifact_path(path: &str) -> bool {
    !path.is_empty()
        && Path::new(path)
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_)))
}

pub(super) fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(super) fn parse_job_id(value: &str) -> Option<JobId> {
    u128::from_str_radix(value, 16).ok().map(JobId)
}

pub(super) fn verified_artifact(directory: &Path, path: &Path) -> Result<ArtifactRecord, String> {
    let metadata = fs::metadata(path).map_err(|e| e.to_string())?;
    Ok(ArtifactRecord {
        path: path
            .strip_prefix(directory)
            .map_err(|e| e.to_string())?
            .to_string_lossy()
            .into_owned(),
        sha256: hex(&sha256_file(path)?),
        bytes: metadata.len(),
    })
}
pub(super) fn verify_artifact(directory: &Path, artifact: &ArtifactRecord) -> Result<(), String> {
    let path = directory.join(&artifact.path);
    let metadata =
        fs::metadata(&path).map_err(|e| format!("StorageCorrupt: {}: {e}", artifact.path))?;
    if metadata.len() != artifact.bytes || hex(&sha256_file(&path)?) != artifact.sha256 {
        return Err(format!(
            "StorageCorrupt: artifact verification failed: {}",
            artifact.path
        ));
    }
    Ok(())
}

fn verify_internal_unified_artifact(
    directory: &Path,
    canonical_directory: &Path,
    artifact: &ArtifactRecord,
    expected_name: &str,
) -> Result<PathBuf, String> {
    if artifact.path != expected_name {
        return Err(format!(
            "StorageCorrupt: unexpected internal publication path: {}",
            artifact.path
        ));
    }
    let path = canonical_internal_unified_path(
        &directory.join(&artifact.path),
        canonical_directory,
        expected_name,
    )?;
    let metadata = fs::metadata(&path)
        .map_err(|error| format!("StorageCorrupt: {}: {error}", artifact.path))?;
    if metadata.len() != artifact.bytes || hex(&sha256_file(&path)?) != artifact.sha256 {
        return Err(format!(
            "StorageCorrupt: artifact verification failed: {}",
            artifact.path
        ));
    }
    Ok(path)
}

fn canonical_internal_unified_path(
    path: &Path,
    canonical_directory: &Path,
    description: &str,
) -> Result<PathBuf, String> {
    let canonical = fs::canonicalize(path)
        .map_err(|error| format!("PublicationUnavailable: {description}: {error}"))?;
    if !canonical.starts_with(canonical_directory) {
        return Err(format!(
            "StorageCorrupt: internal publication path escapes generation: {description}"
        ));
    }
    Ok(canonical)
}

pub(super) fn canonical_generation_directory(
    root: &Path,
    directory: &Path,
) -> Result<PathBuf, String> {
    let canonical_root = root
        .canonicalize()
        .map_err(|error| format!("PublicationUnavailable: archive root: {error}"))?;
    let canonical_directory = directory
        .canonicalize()
        .map_err(|error| format!("PublicationUnavailable: generation directory: {error}"))?;
    if !canonical_directory.starts_with(&canonical_root) {
        return Err("StorageCorrupt: generation directory escapes archive root".into());
    }
    Ok(canonical_directory)
}

fn validate_internal_publication_intent(
    root: &Path,
    job_id: JobId,
    generation: Generation,
    intent: &PublicationIntent,
    manifest_source_sha256: &str,
) -> Result<(), String> {
    let canonical_root = root
        .canonicalize()
        .map_err(|error| format!("PublicationUnavailable: archive root: {error}"))?;
    let current_pending = read_internal_pending(root, job_id, generation.get())?;
    if current_pending.version != 4
        || current_pending.job_id != job_id
        || current_pending.generation != generation
        || current_pending.strategy
            != whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
        || current_pending.source_sha256 != manifest_source_sha256
        || current_pending.source_samples != intent.source_samples
    {
        return Err("StaleResponse: internal publication pending identity changed".into());
    }
    let bare_operation_id = format!("{:032x}-{}", job_id.0, generation.get());
    let resumed_operation_id = format!(
        "{}-{bare_operation_id}",
        current_pending.strategy.stable_id()
    );
    if intent.version != 1
        || (intent.operation_id != bare_operation_id && intent.operation_id != resumed_operation_id)
        || intent.source_sha256 != manifest_source_sha256
        || intent.export_basename != "transcript"
    {
        return Err("StaleResponse: internal publication intent identity changed".into());
    }

    match &intent.choice {
        DuplicateImportChoice::CreateNew => {
            if intent.cleanup_id.is_some() || !intent.cleanup_targets.is_empty() {
                return Err("StaleResponse: create-only intent has replacement metadata".into());
            }
        }
        DuplicateImportChoice::ReplaceLatest => {
            if intent.cleanup_targets.len() != 1 {
                return Err("StaleResponse: ReplaceLatest target count changed".into());
            }
            validate_internal_reprocessing_provenance(
                root,
                &canonical_root,
                job_id,
                generation,
                &current_pending,
                intent,
            )?;
        }
        DuplicateImportChoice::ReplaceAll => {
            if intent.cleanup_targets.is_empty() {
                return Err("StaleResponse: ReplaceAll has no persisted cleanup targets".into());
            }
            validate_internal_reprocessing_provenance(
                root,
                &canonical_root,
                job_id,
                generation,
                &current_pending,
                intent,
            )?;
        }
        DuplicateImportChoice::ReplaceSelectedPath(selected_path) => {
            if selected_path.trim().is_empty() || intent.cleanup_targets.len() != 1 {
                return Err("StaleResponse: selected replacement target changed".into());
            }
            validate_internal_reprocessing_provenance(
                root,
                &canonical_root,
                job_id,
                generation,
                &current_pending,
                intent,
            )?;
        }
        DuplicateImportChoice::Replace => {
            return Err("StaleResponse: legacy replacement is not an internal intent".into());
        }
    }
    Ok(())
}

fn validate_internal_reprocessing_provenance(
    root: &Path,
    canonical_root: &Path,
    job_id: JobId,
    generation: Generation,
    current_pending: &PendingRecord,
    intent: &PublicationIntent,
) -> Result<(), String> {
    let cleanup_id = intent
        .cleanup_id
        .as_deref()
        .ok_or_else(|| "StaleResponse: reprocessing intent has no cleanup identity".to_owned())?;
    let origin_generation = parse_cleanup_origin_generation(cleanup_id, job_id, generation)?;
    let origin_pending = if origin_generation == generation.get() {
        current_pending.clone()
    } else {
        read_internal_pending(root, job_id, origin_generation)?
    };
    if origin_pending.version != 4
        || origin_pending.job_id != job_id
        || origin_pending.generation.get() != origin_generation
        || origin_pending.strategy != current_pending.strategy
        || origin_pending.source_sha256 != intent.source_sha256
        || origin_pending.source_samples != intent.source_samples
    {
        return Err(
            "StaleResponse: cleanup origin does not match persisted source identity".into(),
        );
    }

    let cleanup_plan = read_cleanup_plan(root, cleanup_id)?;
    if cleanup_plan.version != 1
        || cleanup_plan.cleanup_id != cleanup_id
        || cleanup_plan.destination.as_str() != canonical_root.to_string_lossy().as_ref()
        || cleanup_plan.source_path != current_pending.source_path
        || cleanup_plan.source_sha256 != intent.source_sha256
        || cleanup_plan.successor.job_id != job_id
        || cleanup_plan.successor.generation != generation
        || !cleanup_plan.published
        || !cleanup_targets_match_intent(&intent.cleanup_targets, &cleanup_plan.targets)
        || intent
            .cleanup_targets
            .iter()
            .enumerate()
            .any(|(index, target)| {
                target.source_sha256 != intent.source_sha256
                    || target.treatment.destination.as_str()
                        != canonical_root.to_string_lossy().as_ref()
                    || (target.treatment.job_id == job_id
                        && target.treatment.generation == generation)
                    || intent.cleanup_targets[..index]
                        .iter()
                        .any(|previous| previous.treatment == target.treatment)
            })
    {
        return Err(
            "StaleResponse: persisted cleanup provenance does not match reprocessing intent".into(),
        );
    }
    Ok(())
}

fn parse_cleanup_origin_generation(
    cleanup_id: &str,
    job_id: JobId,
    current_generation: Generation,
) -> Result<u64, String> {
    let prefix = format!("{:032x}-", job_id.0);
    let generation_text = cleanup_id
        .strip_prefix(&prefix)
        .ok_or_else(|| "StaleResponse: cleanup identity belongs to another job".to_owned())?;
    let origin_generation = generation_text
        .parse::<u64>()
        .map_err(|_| "StaleResponse: cleanup generation identity is invalid".to_owned())?;
    if origin_generation == 0
        || origin_generation > current_generation.get()
        || origin_generation.to_string() != generation_text
    {
        return Err("StaleResponse: cleanup generation is not a valid prior identity".into());
    }
    Ok(origin_generation)
}

fn read_internal_pending(
    root: &Path,
    job_id: JobId,
    generation: u64,
) -> Result<PendingRecord, String> {
    let pending_path = root
        .join("transcriptions")
        .join(format!("{:032x}", job_id.0))
        .join(generation.to_string())
        .join("pending.json");
    serde_json::from_slice(
        &fs::read(&pending_path)
            .map_err(|error| format!("PublicationUnavailable: pending identity: {error}"))?,
    )
    .map_err(|error| format!("StorageCorrupt: pending identity: {error}"))
}

pub(super) fn verify_pcm_staging(
    directory: &Path,
    artifact: &ArtifactRecord,
    samples: u64,
    expected_pcm_sha256: &str,
) -> Result<(), String> {
    let expected_bytes = (PCM_STAGING_HEADER.len() as u64)
        .checked_add(
            samples
                .checked_mul(2)
                .ok_or_else(|| "StorageCorrupt: PCM sample byte count overflow".to_owned())?,
        )
        .ok_or_else(|| "StorageCorrupt: PCM staging byte count overflow".to_owned())?;
    if artifact.bytes != expected_bytes {
        return Err(
            "StorageCorrupt: PCM staging byte count does not match its sample range".into(),
        );
    }
    let path = directory.join(&artifact.path);
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut header = vec![0; PCM_STAGING_HEADER.len()];
    file.read_exact(&mut header)
        .map_err(|e| format!("StorageCorrupt: PCM staging header: {e}"))?;
    if header != PCM_STAGING_HEADER {
        return Err("StorageCorrupt: PCM staging version/header mismatch".into());
    }
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    if hex(&hash.finalize()) != expected_pcm_sha256 {
        return Err("StorageCorrupt: PCM payload SHA-256 mismatch".into());
    }
    Ok(())
}

fn canonicalize_for_new_path(path: &Path) -> Option<PathBuf> {
    let mut unresolved = Vec::new();
    let mut existing = path;
    let canonical_base = loop {
        match fs::canonicalize(existing) {
            Ok(path) => break path,
            Err(_) => {
                let name = existing.file_name()?;
                if name == "." || name == ".." {
                    return None;
                }
                unresolved.push(name.to_owned());
                existing = existing.parent()?;
            }
        }
    };
    let mut result = canonical_base;
    for component in unresolved.iter().rev() {
        result.push(component);
    }
    Some(result)
}

pub(super) fn is_r5_2_vad_recipe_destination(path: &Path) -> bool {
    let Some(runtime_root) = std::env::var_os("PARAKEET_L2_RUNTIME_ROOT").map(PathBuf::from) else {
        return false;
    };
    let Some(archive_root) = std::env::var_os("PARAKEET_L2_ARCHIVE_ROOT").map(PathBuf::from) else {
        return false;
    };
    let Ok(recipe_root) = runtime_root.join("r5-2").canonicalize() else {
        return false;
    };
    let Ok(archive_root) = archive_root.canonicalize() else {
        return false;
    };
    let Some(destination) = canonicalize_for_new_path(path) else {
        return false;
    };
    archive_root.starts_with(&recipe_root) && destination.starts_with(&archive_root)
}
