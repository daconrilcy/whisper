use super::{
    ActiveArchive, ArchiveQualificationKey, ArchiveStore, Digest, DuplicateImportChoice,
    ImportRequest, ImportVadWindowPayload, ImportVadWindowRecord, Instant, OpenOptions, Path,
    PathBuf, PendingRecord, PublicationIdentity, PublicationIntent, Sha256, Write,
    assert_campaign_archive, fs, hex, initialize_pending, is_r5_2_vad_recipe_destination,
    is_registered_model, publication_reservation_path, read_import_vad_windows, read_segments,
    reserve_export_basename, segmented_pcm_manifest, sha256_file, source_output_samples,
    validate_pending, write_publication_intent, write_replace_synced,
};

impl ArchiveStore {
    /// Changes the backend key only while an import is still pending and has no transcript
    /// segments. Auto may use this once when a GPU preparation fails before native inference.
    pub fn replace_pending_qualification(
        &mut self,
        qualification: ArchiveQualificationKey,
    ) -> Result<(), String> {
        let active = self
            .current
            .as_ref()
            .ok_or_else(|| "InvalidInput: no active pending import".to_owned())?;
        if active.expected_sequence != 1
            || !read_segments(&active.directory.join("segments.jsonl"))?.is_empty()
        {
            return Err("InvalidInput: backend cannot change after transcript output".into());
        }
        let path = active.directory.join("pending.json");
        let mut pending: PendingRecord = serde_json::from_slice(
            &fs::read(&path).map_err(|error| format!("StorageCorrupt: pending record: {error}"))?,
        )
        .map_err(|error| format!("StorageCorrupt: pending record: {error}"))?;
        if pending.job_id != active.job_id || pending.generation != active.generation {
            return Err("StorageCorrupt: active pending identity changed".into());
        }
        if !pending
            .qualification
            .as_ref()
            .is_some_and(ArchiveQualificationKey::is_parakeet_import_test)
            || !qualification.is_parakeet_import_test()
            || pending.version == 3
        {
            return Err("ModelNotQualified: pending backend key cannot be replaced".into());
        }
        pending.qualification = Some(qualification);
        pending.version = 2;
        write_replace_synced(
            &path,
            &serde_json::to_vec(&pending).map_err(|error| error.to_string())?,
        )
    }

    /// Durably records that a central VAD interval is fully processed after all
    /// of its transcript segments have been persisted.
    pub fn confirm_import_vad_window(
        &mut self,
        window: whisper_core::parakeet_windows::Window,
        last_segment_sequence: u64,
    ) -> Result<(), String> {
        let active = self
            .current
            .as_ref()
            .ok_or_else(|| "StorageUnavailable: import archive is not active".to_owned())?;
        let pending: PendingRecord = serde_json::from_slice(
            &fs::read(active.directory.join("pending.json"))
                .map_err(|error| format!("StorageCorrupt: pending record: {error}"))?,
        )
        .map_err(|error| format!("StorageCorrupt: pending record: {error}"))?;
        validate_pending(&pending, active.job_id, active.generation.get())?;
        let qualification = pending
            .qualification
            .clone()
            .filter(ArchiveQualificationKey::is_parakeet_vad_test)
            .ok_or_else(|| "ModelNotQualified: import-vad archive key is absent".to_owned())?;
        let records = read_import_vad_windows(&active.directory, &pending)?;
        let expected_start = records.last().map_or(0, |record| record.central_end_sample);
        let expected_sequence = records.len() as u64 + 1;
        if window.index as u64 + 1 != expected_sequence
            || window.central_start_sample != expected_start
            || window.central_start_sample >= window.central_end_sample
            || window.central_end_sample > active.source_samples
            || window.native_start_sample > window.central_start_sample
            || window.native_end_sample < window.central_end_sample
            || window.native_end_sample - window.native_start_sample
                > whisper_core::parakeet_windows::MAX_NATIVE_SAMPLES
            || last_segment_sequence.checked_add(1) != Some(active.expected_sequence)
        {
            return Err("StorageCorrupt: import-vad window confirmation is not contiguous".into());
        }
        let payload = ImportVadWindowPayload {
            version: 1,
            job_id: active.job_id,
            generation: active.generation,
            sequence: expected_sequence,
            central_start_sample: window.central_start_sample,
            central_end_sample: window.central_end_sample,
            native_start_sample: window.native_start_sample,
            native_end_sample: window.native_end_sample,
            last_segment_sequence,
            qualification,
        };
        let canonical = serde_json::to_vec(&payload).map_err(|error| error.to_string())?;
        let bytes = serde_json::to_vec(&ImportVadWindowRecord {
            payload,
            sha256: format!("{:x}", Sha256::digest(canonical)),
        })
        .map_err(|error| error.to_string())?;
        let path = active.directory.join("vad-windows.jsonl");
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|error| format!("StorageUnavailable: VAD checkpoint: {error}"))?;
        file.write_all(&bytes)
            .and_then(|()| file.write_all(b"\n"))
            .and_then(|()| file.sync_all())
            .map_err(|error| format!("StorageUnavailable: VAD checkpoint sync: {error}"))?;
        Ok(())
    }

    pub fn confirmed_import_vad_offset(&self) -> Result<u64, String> {
        let active = self
            .current
            .as_ref()
            .ok_or_else(|| "StorageUnavailable: import archive is not active".to_owned())?;
        let pending: PendingRecord = serde_json::from_slice(
            &fs::read(active.directory.join("pending.json"))
                .map_err(|error| format!("StorageCorrupt: pending record: {error}"))?,
        )
        .map_err(|error| format!("StorageCorrupt: pending record: {error}"))?;
        Ok(read_import_vad_windows(&active.directory, &pending)?
            .last()
            .map_or(0, |record| record.central_end_sample))
    }

    pub fn prepare(&mut self, request: ImportRequest) -> Result<ImportRequest, String> {
        self.prepare_with_catalog_path(request, None)
    }

    /// Prepares using a revalidated catalog file while preserving the historical model identity
    /// in the durable checkpoint. The returned request points at the executable catalog path.
    pub fn prepare_with_catalog_path(
        &mut self,
        request: ImportRequest,
        executable_model_path: Option<&str>,
    ) -> Result<ImportRequest, String> {
        self.prepare_inner(request, executable_model_path, None)
    }

    pub fn prepare_with_qualification(
        &mut self,
        request: ImportRequest,
        executable_model_path: Option<&str>,
        qualification: Option<ArchiveQualificationKey>,
    ) -> Result<ImportRequest, String> {
        self.prepare_inner(request, executable_model_path, qualification)
    }

    /// Prepares a bounded-window capture whose logical source consists of every
    /// acknowledged PCM part plus its contiguous part index.
    pub fn prepare_segmented_capture(
        &mut self,
        request: ImportRequest,
        source: &crate::parakeet_audio::PcmPartStore,
        executable_model_path: Option<&str>,
        qualification: Option<ArchiveQualificationKey>,
    ) -> Result<ImportRequest, String> {
        if request.strategy
            != whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
            || Path::new(&request.source_path) != source.base_path()
        {
            return Err(
                "InvalidInput: segmented capture request does not name its PCM source".into(),
            );
        }
        let (source_sha, source_samples, manifest) = segmented_pcm_manifest(source)?;
        let prepared = self.prepare_inner_impl(
            request,
            executable_model_path,
            qualification,
            None,
            Some((source_sha, source_samples)),
            None,
        )?;
        let directory = self
            .current
            .as_ref()
            .ok_or_else(|| {
                "StorageUnavailable: segmented capture archive was not prepared".to_owned()
            })?
            .directory
            .clone();
        let manifest_path = directory.join("captured-source-parts.json");
        let bytes = serde_json::to_vec(&manifest).map_err(|error| error.to_string())?;
        write_replace_synced(&manifest_path, &bytes)
            .map_err(|error| format!("StorageUnavailable: capture source manifest: {error}"))?;
        let active = self.current.as_mut().unwrap();
        active.segmented_source_manifest = Some((manifest_path, Sha256::digest(&bytes).into()));
        Ok(prepared)
    }

    pub fn prepare_with_source_binding(
        &mut self,
        request: ImportRequest,
        executable_model_path: Option<&str>,
        qualification: Option<ArchiveQualificationKey>,
        binding: &crate::source_binding::SourceBinding,
    ) -> Result<ImportRequest, String> {
        let started = Instant::now();
        let result = self.prepare_inner_impl(
            request,
            executable_model_path,
            qualification,
            Some(binding),
            None,
            None,
        );
        crate::diagnostics::record_stage_timing(
            "archive_prepare_pending",
            started.elapsed(),
            serde_json::json!({ "success": result.is_ok(), "source_binding": true }),
        );
        result
    }

    fn prepare_inner(
        &mut self,
        request: ImportRequest,
        executable_model_path: Option<&str>,
        qualification: Option<ArchiveQualificationKey>,
    ) -> Result<ImportRequest, String> {
        self.prepare_inner_with_publication_lock(
            request,
            executable_model_path,
            qualification,
            None,
        )
    }

    pub(super) fn prepare_inner_with_publication_lock(
        &mut self,
        request: ImportRequest,
        executable_model_path: Option<&str>,
        qualification: Option<ArchiveQualificationKey>,
        locked_destination: Option<&Path>,
    ) -> Result<ImportRequest, String> {
        let started = Instant::now();
        let result = self.prepare_inner_impl(
            request,
            executable_model_path,
            qualification,
            None,
            None,
            locked_destination,
        );
        crate::diagnostics::record_stage_timing(
            "archive_prepare_pending",
            started.elapsed(),
            serde_json::json!({ "success": result.is_ok() }),
        );
        result
    }

    fn prepare_inner_impl(
        &mut self,
        mut request: ImportRequest,
        executable_model_path: Option<&str>,
        qualification: Option<ArchiveQualificationKey>,
        source_binding: Option<&crate::source_binding::SourceBinding>,
        segmented_source_identity: Option<([u8; 32], u64)>,
        locked_destination: Option<&Path>,
    ) -> Result<ImportRequest, String> {
        assert_campaign_archive(&self.root)?;
        if self.current.is_some() {
            return Err("Busy: an import is already active".into());
        }
        let (source_sha, source_samples) = if let Some((sha, samples)) = segmented_source_identity {
            if source_binding.is_some()
                || request
                    .source_sha256
                    .is_some_and(|expected| expected != sha)
                || request
                    .source_samples
                    .is_some_and(|expected| expected != samples)
            {
                return Err(
                    "SourceChanged: segmented source identity differs from the accepted request"
                        .into(),
                );
            }
            (sha, samples)
        } else if let Some(binding) = source_binding {
            if binding.original_path() != request.source_path
                || binding.strategy() != request.strategy
                || request
                    .source_sha256
                    .is_some_and(|sha| sha != binding.source_sha256())
                || request
                    .source_samples
                    .is_some_and(|samples| samples != binding.source_samples())
            {
                return Err(
                    "SourceChanged: accepted request differs from its source snapshot".into(),
                );
            }
            (binding.source_sha256(), binding.source_samples())
        } else {
            let source = Path::new(&request.source_path);
            if !source.is_file() {
                return Err("SourceMissing: selected audio source does not exist".into());
            }
            let source_sha = sha256_file(source)?;
            if request
                .source_sha256
                .is_some_and(|expected| expected != source_sha)
            {
                return Err("SourceChanged: source hash differs from the accepted identity".into());
            }
            let source_samples = source_output_samples(source)?;
            if request
                .source_samples
                .is_some_and(|expected| expected != source_samples)
            {
                return Err(
                    "SourceChanged: source duration differs from the accepted identity".into(),
                );
            }
            if sha256_file(source)? != source_sha {
                return Err(
                    "SourceChanged: audio source changed while its duration was inspected".into(),
                );
            }
            (source_sha, source_samples)
        };
        let durable_model_path = request.model_path.clone();
        let model_path = executable_model_path.unwrap_or(&request.model_path);
        let model_meta = fs::metadata(model_path).map_err(|e| format!("ModelMissing: {e}"))?;
        let (_, model_hash) = crate::model_catalog::cached_model_sha256(Path::new(model_path))
            .map_err(|error| format!("ModelHashRead: {error}"))?;
        let is_parakeet_model = hex(&model_hash)
            == whisper_core::model_registry::model(
                whisper_core::model_catalog::ModelId::ParakeetTdt06BV3,
            )
            .artifact
            .sha256;
        let vad_qualification = qualification
            .as_ref()
            .is_some_and(ArchiveQualificationKey::is_parakeet_vad_test);
        let single_pass_limit_samples =
            whisper_core::model_registry::PARAKEET_MAX_SINGLE_PASS_DURATION_MS.saturating_mul(16);
        let unified = request.strategy
            == whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1;
        if unified && (!is_parakeet_model || qualification.is_some()) {
            return Err(
                "ModelNotQualified: unified windows require the controlled Parakeet path".into(),
            );
        }
        if !unified
            && is_parakeet_model
            && !vad_qualification
            && source_samples > single_pass_limit_samples
        {
            return Err(
                "DurationUnsupported: Fichier trop long pour Parakeet, utilisez Turbo".into(),
            );
        }
        if qualification
            .as_ref()
            .is_some_and(ArchiveQualificationKey::is_retired_r5_2_key)
        {
            return Err(
                "ModelNotQualified: l'ancienne clé R5-2 Parakeet est retirée pour tout nouvel import".into(),
            );
        }
        let single_pass_qualification = qualification.as_ref().is_some_and(|key| {
            key.is_parakeet_import_test() && key.inference_profile == "import-single-pass"
        });
        if !unified
            && is_parakeet_model
            && ((source_samples > whisper_core::parakeet_ipc::PARAKEET_MAX_AUDIO_SAMPLES as u64
                && !vad_qualification)
                || (source_samples
                    <= whisper_core::parakeet_ipc::PARAKEET_MAX_AUDIO_SAMPLES as u64
                    && vad_qualification))
        {
            return Err(
                "DurationUnsupported: la durée et le profil Parakeet ne correspondent pas".into(),
            );
        }
        if !is_registered_model(&hex(&model_hash), model_meta.len())
            || request.model_sha256 != model_hash
        {
            return Err("ModelHashMismatch: model is not a registered approved artifact".into());
        }
        let is_parakeet = hex(&model_hash)
            == whisper_core::model_registry::model(
                whisper_core::model_catalog::ModelId::ParakeetTdt06BV3,
            )
            .artifact
            .sha256;
        if (!unified && is_parakeet && !single_pass_qualification && !vad_qualification)
            || (!is_parakeet && qualification.is_some())
        {
            return Err("ModelNotQualified: exact Test worker/runtime key is required".into());
        }
        if vad_qualification && !is_r5_2_vad_recipe_destination(Path::new(&request.destination)) {
            return Err(
                "ModelNotQualified: import-vad est réservé aux recettes R5-2; activation produit en attente".into(),
            );
        }
        let destination = PathBuf::from(&request.destination);
        fs::create_dir_all(&destination).map_err(|e| format!("StorageUnavailable: {e}"))?;
        let lock_already_held = if let Some(locked) = locked_destination {
            let locked = fs::canonicalize(locked).map_err(|error| {
                format!("StorageUnavailable: resolve held publication destination: {error}")
            })?;
            let current = fs::canonicalize(&destination).map_err(|error| {
                format!("StorageUnavailable: resolve requested publication destination: {error}")
            })?;
            locked == current
        } else {
            false
        };
        let _publication_lock = if lock_already_held {
            None
        } else {
            Some(Self::lock_publications(&destination)?)
        };
        let directory = destination
            .join("transcriptions")
            .join(format!("{:032x}", request.job_id.0))
            .join(request.generation.get().to_string());
        if directory.join("CURRENT").is_file() {
            return Err(
                "ImportIdentityConflict: generation is already published; a new JobId is required"
                    .into(),
            );
        }
        let reservation =
            publication_reservation_path(&destination, request.job_id, request.generation);
        let publication = if reservation.is_file() {
            serde_json::from_slice(&fs::read(&reservation).map_err(|error| error.to_string())?)
                .map_err(|error| format!("StorageCorrupt: publication reservation: {error}"))?
        } else {
            let expected_active = Self::published_imports_for_strategy(
                &destination,
                &hex(&source_sha),
                request.strategy,
            )?
            .into_iter()
            .map(|item| PublicationIdentity {
                job_id: item.job_id,
                generation: item.generation,
            })
            .collect();
            let export_basename = reserve_export_basename(&destination, &request.source_path)?;
            PublicationIntent {
                version: 1,
                operation_id: format!(
                    "{}-{:032x}-{}",
                    if request.strategy
                        == whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1
                    {
                        "legacy"
                    } else {
                        request.strategy.stable_id()
                    },
                    request.job_id.0,
                    request.generation.get()
                ),
                choice: DuplicateImportChoice::CreateNew,
                source_sha256: hex(&source_sha),
                source_samples,
                expected_active,
                export_basename,
                internal_only: false,
                cleanup_id: None,
                cleanup_targets: Vec::new(),
            }
        };
        if publication.source_sha256 != hex(&source_sha)
            || publication.source_samples != source_samples
        {
            return Err("SourceChanged: publication reservation does not match source".into());
        }
        if !reservation.is_file() {
            write_publication_intent(&destination, &request, &publication)?;
        }
        fs::create_dir_all(&directory).map_err(|e| format!("StorageUnavailable: {e}"))?;
        let pending = PendingRecord {
            strategy: request.strategy,
            version: if unified {
                4
            } else if vad_qualification {
                3
            } else if qualification.is_some() {
                2
            } else {
                1
            },
            job_id: request.job_id,
            generation: request.generation,
            source_path: request.source_path.clone(),
            source_sha256: hex(&source_sha),
            source_samples,
            model_path: durable_model_path,
            model_sha256: hex(&model_hash),
            model_size: model_meta.len(),
            qualification,
        };
        let expected_sequence = initialize_pending(&directory, &pending)?;
        request.source_sha256 = Some(source_sha);
        request.source_samples = Some(source_samples);
        self.current = Some(ActiveArchive {
            job_id: request.job_id,
            generation: request.generation,
            directory,
            source_sha256: source_sha,
            source_samples,
            strategy: request.strategy,
            expected_sequence,
            publication,
            segmented_source_manifest: None,
        });
        if let Some(executable_model_path) = executable_model_path {
            request.model_path = executable_model_path.to_owned();
        }
        Ok(request)
    }
}
