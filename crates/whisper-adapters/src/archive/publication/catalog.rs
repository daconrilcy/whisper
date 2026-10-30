use super::super::{
    ArchiveStore, CompleteRecord, DuplicateImportChoice, Generation, ImportRequest,
    InstalledExportOwnership, JobId, OpenOptions, Path, PathBuf, PendingRecord,
    PublicationIdentity, PublicationIndex, PublicationIntent, PublicationLock, PublicationRecord,
    PublishedImport, TreatmentIdentity, attach_empty_container_proofs, capture_cleanup_target,
    export_install_temporary_path, fs, generation_path, hex, install_publication_exports,
    installed_export_ownership_path, io, load_or_migrate_publication_index,
    persisted_file_identity, sha256_file, verify_artifact, write_new_synced, write_replace_synced,
};

impl ArchiveStore {
    pub fn lock_publications(root: &Path) -> Result<PublicationLock, String> {
        fs::create_dir_all(root.join(".whisper-publications"))
            .map_err(|error| format!("StorageUnavailable: publication directory: {error}"))?;
        let path = root.join(".whisper-publications.lock");
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .open(path)
            .map_err(|error| format!("StorageUnavailable: publication lock: {error}"))?;
        file.lock()
            .map_err(|error| format!("StorageUnavailable: publication lock: {error}"))?;
        Ok(PublicationLock { _file: file })
    }

    pub fn published_imports(
        root: &Path,
        source_sha256: &str,
    ) -> Result<Vec<PublishedImport>, String> {
        Self::published_imports_for_strategy(
            root,
            source_sha256,
            whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
        )
    }

    pub fn published_imports_for_strategy(
        root: &Path,
        source_sha256: &str,
        strategy: whisper_core::parakeet_session::TranscriptionStrategy,
    ) -> Result<Vec<PublishedImport>, String> {
        let entries = Self::scan(root)?;
        if let Some(diagnostic) = entries.iter().find_map(|entry| {
            entry
                .diagnostic
                .as_deref()
                .filter(|message| message.starts_with("StorageCorrupt:"))
        }) {
            return Err(diagnostic.to_owned());
        }
        let mut found = Vec::new();
        let index = load_or_migrate_publication_index(root)?;
        for entry in entries.into_iter().filter(|entry| {
            entry.complete && entry.source_sha256 == source_sha256 && entry.strategy == strategy
        }) {
            if entry.publication_recovery {
                continue;
            }
            found.push(PublishedImport {
                job_id: entry.job_id,
                generation: entry.generation,
                source_sha256: entry.source_sha256,
                export_basename: entry.export_basename,
                active: true,
                success_ordinal: index
                    .records
                    .iter()
                    .find(|record| {
                        record.job_id == entry.job_id && record.generation == entry.generation
                    })
                    .and_then(|record| record.success_ordinal),
                verified_present: true,
            });
        }
        found.sort_by_key(|item| (item.job_id.0, item.generation.get()));
        Ok(found)
    }

    /// Returns every indexed treatment for the encoded audio identity,
    /// including inactive R11 entries. Entries are selectable only when the
    /// complete archive scan verifies their current presence.
    pub fn reprocessing_catalog(
        root: &Path,
        source_sha256: &str,
    ) -> Result<Vec<PublishedImport>, String> {
        let index = load_or_migrate_publication_index(root)?;
        let scanned = Self::scan_raw(root)?;
        let mut candidates = Vec::new();
        for record in index
            .records
            .iter()
            .filter(|record| record.source_sha256 == source_sha256)
        {
            let present = scanned.iter().any(|entry| {
                entry.job_id == record.job_id
                    && entry.generation == record.generation
                    && entry.complete
                    && !entry.publication_recovery
                    && entry.source_sha256 == source_sha256
            });
            candidates.push(PublishedImport {
                job_id: record.job_id,
                generation: record.generation,
                source_sha256: source_sha256.to_owned(),
                export_basename: record.export_basename.clone(),
                active: record.active,
                success_ordinal: record.success_ordinal,
                verified_present: present,
            });
        }
        for entry in scanned.iter().filter(|entry| {
            entry.complete && !entry.publication_recovery && entry.source_sha256 == source_sha256
        }) {
            if !candidates.iter().any(|candidate| {
                candidate.job_id == entry.job_id && candidate.generation == entry.generation
            }) {
                candidates.push(PublishedImport {
                    job_id: entry.job_id,
                    generation: entry.generation,
                    source_sha256: source_sha256.to_owned(),
                    export_basename: entry.export_basename.clone(),
                    active: true,
                    success_ordinal: None,
                    verified_present: true,
                });
            }
        }
        candidates.sort_by_key(|candidate| {
            (
                candidate.success_ordinal.is_none(),
                candidate.success_ordinal.unwrap_or(u64::MAX),
                candidate.job_id.0,
                candidate.generation.get(),
            )
        });
        Ok(candidates)
    }

    pub fn reserve_new_import(
        root: &Path,
        request: &ImportRequest,
        source_sha256: &str,
    ) -> Result<(), String> {
        let basename = reserve_export_basename(root, &request.source_path)?;
        write_publication_reservation(
            root,
            request,
            source_sha256,
            request.source_samples.unwrap_or(0),
            Vec::new(),
            DuplicateImportChoice::CreateNew,
            basename,
        )
    }

    /// Reserves a new import whose only final TXT/SRT pair lives in its
    /// transcription generation directory.
    pub fn reserve_create_only_import(
        root: &Path,
        request: &ImportRequest,
        source_sha256: &str,
        source_samples: u64,
    ) -> Result<(), String> {
        write_publication_reservation_with_storage(
            root,
            request,
            source_sha256,
            source_samples,
            Vec::new(),
            DuplicateImportChoice::CreateNew,
            "transcript".into(),
            true,
        )
    }

    /// Reserves the first publication in a destination without creating an
    /// `exports/` copy. Existing legacy reservation APIs retain their format.
    pub fn reserve_create_only_new_import(
        root: &Path,
        request: &ImportRequest,
        source_sha256: &str,
    ) -> Result<(), String> {
        write_publication_reservation_with_storage(
            root,
            request,
            source_sha256,
            request.source_samples.unwrap_or(0),
            Vec::new(),
            DuplicateImportChoice::CreateNew,
            "transcript".into(),
            true,
        )
    }

    pub fn reserve_duplicate_import(
        root: &Path,
        request: &ImportRequest,
        source_sha256: &str,
        source_samples: u64,
        expected_active: &[PublishedImport],
        choice: DuplicateImportChoice,
        export_basename: &str,
    ) -> Result<(), String> {
        let current = Self::published_imports(root, source_sha256)?;
        if current != expected_active {
            return Err("DuplicateStateChanged: publications changed before enqueue".into());
        }
        let expected = current
            .iter()
            .map(|item| PublicationIdentity {
                job_id: item.job_id,
                generation: item.generation,
            })
            .collect();
        let basename = reserve_export_basename(root, &request.source_path)?;
        if basename != export_basename {
            return Err(
                "DuplicateExportNameChanged: proposed export name is no longer available".into(),
            );
        }
        write_publication_reservation(
            root,
            request,
            source_sha256,
            source_samples,
            expected,
            choice,
            basename,
        )
    }

    /// Freezes verified replacement targets and writes the cleanup plan in the
    /// same durable publication reservation as the new import.
    pub fn reserve_reprocessing_import(
        root: &Path,
        request: &ImportRequest,
        source_sha256: &str,
        source_samples: u64,
        expected_catalog: &[PublishedImport],
        selected: &[TreatmentIdentity],
        choice: DuplicateImportChoice,
    ) -> Result<(), String> {
        if !matches!(
            &choice,
            DuplicateImportChoice::ReplaceLatest
                | DuplicateImportChoice::ReplaceAll
                | DuplicateImportChoice::ReplaceSelectedPath(_)
        ) {
            return Err("InvalidInput: choix de remplacement RTC-02 invalide".into());
        }
        if selected.is_empty() {
            return Err("DuplicateStateChanged: aucun traitement vérifié à remplacer".into());
        }
        let current = Self::reprocessing_catalog(root, source_sha256)?;
        let verified_current = current
            .iter()
            .filter(|item| item.verified_present)
            .cloned()
            .collect::<Vec<_>>();
        if verified_current != expected_catalog {
            return Err("DuplicateStateChanged: le catalogue des traitements a changé".into());
        }
        if selected
            .iter()
            .enumerate()
            .any(|(index, item)| selected[..index].iter().any(|previous| previous == item))
        {
            return Err("InvalidInput: cible de remplacement dupliquée".into());
        }
        let destination = root
            .canonicalize()
            .map_err(|error| format!("StorageUnavailable: destination: {error}"))?;
        let mut targets = Vec::with_capacity(selected.len());
        for treatment in selected {
            if Path::new(&treatment.destination)
                .canonicalize()
                .ok()
                .as_deref()
                != Some(destination.as_path())
            {
                return Err(
                    "InvalidInput: le traitement choisi n'appartient pas à la destination courante"
                        .into(),
                );
            }
            let candidate = current
                .iter()
                .find(|candidate| {
                    candidate.job_id == treatment.job_id
                        && candidate.generation == treatment.generation
                        && candidate.source_sha256 == source_sha256
                        && candidate.verified_present
                })
                .ok_or_else(|| {
                    "InvalidInput: le traitement choisi n'est pas vérifié pour cet audio".to_owned()
                })?;
            targets.push(capture_cleanup_target(&destination, candidate)?);
        }
        attach_empty_container_proofs(&destination, &current, selected, &mut targets)?;
        let cleanup_id = format!("{:032x}-{}", request.job_id.0, request.generation.get());
        let intent = PublicationIntent {
            version: 1,
            operation_id: cleanup_id.clone(),
            choice,
            source_sha256: source_sha256.to_owned(),
            source_samples,
            expected_active: Self::published_imports_for_strategy(
                &destination,
                source_sha256,
                request.strategy,
            )?
            .iter()
            .map(|item| PublicationIdentity {
                job_id: item.job_id,
                generation: item.generation,
            })
            .collect(),
            export_basename: "transcript".into(),
            internal_only: true,
            cleanup_id: Some(cleanup_id),
            cleanup_targets: targets,
        };
        write_publication_intent(&destination, request, &intent)
    }

    pub fn suggest_duplicate_export_basename(
        root: &Path,
        source_path: &str,
    ) -> Result<String, String> {
        reserve_export_basename(root, source_path)
    }

    /// Returns the reserved basename when a visible export cannot be proven
    /// to belong to the interrupted publication. Content equality is never
    /// used as ownership evidence.
    pub fn publication_reconfirmation_required(
        root: &Path,
        job_id: JobId,
        generation: Generation,
    ) -> Result<Option<String>, String> {
        let _lock = Self::lock_publications(root)?;
        publication_reconfirmation_required_unlocked(root, job_id, generation)
    }

    /// Reassigns an interrupted publication only after the user confirms the
    /// newly proposed name. The ambiguous prior export is left untouched.
    pub fn reconfirm_interrupted_publication(
        root: &Path,
        previous: &ImportRequest,
        source_sha256: &str,
        source_samples: u64,
        expected_active: &[PublishedImport],
        choice: DuplicateImportChoice,
        reserved_basename: &str,
        confirmed_basename: &str,
    ) -> Result<(), String> {
        let _lock = Self::lock_publications(root)?;
        let current = Self::published_imports(root, source_sha256)?;
        if current != expected_active {
            return Err(
                "DuplicateStateChanged: publications changed before resume confirmation".into(),
            );
        }
        let path = publication_reservation_path(root, previous.job_id, previous.generation);
        let mut intent: PublicationIntent =
            serde_json::from_slice(&fs::read(&path).map_err(|error| {
                format!("StorageCorrupt: interrupted publication reservation: {error}")
            })?)
            .map_err(|error| {
                format!("StorageCorrupt: interrupted publication reservation: {error}")
            })?;
        if intent.source_sha256 != source_sha256
            || intent.source_samples != source_samples
            || intent.export_basename != reserved_basename
            || publication_reconfirmation_required_unlocked(
                root,
                previous.job_id,
                previous.generation,
            )?
            .as_deref()
                != Some(reserved_basename)
        {
            return Err(
                "DuplicateStateChanged: interrupted export changed before confirmation".into(),
            );
        }
        let available = reserve_export_basename(root, &previous.source_path)?;
        if available != confirmed_basename {
            return Err(
                "DuplicateExportNameChanged: proposed export name is no longer available".into(),
            );
        }
        intent.choice = choice;
        intent.expected_active = current
            .iter()
            .map(|item| PublicationIdentity {
                job_id: item.job_id,
                generation: item.generation,
            })
            .collect();
        intent.export_basename = confirmed_basename.to_owned();

        let directory = generation_path(root, previous.job_id, previous.generation);
        let ownership_path = installed_export_ownership_path(&directory);
        if ownership_path.is_file() {
            let mut ownership: InstalledExportOwnership =
                serde_json::from_slice(&fs::read(&ownership_path).map_err(|error| {
                    format!("StorageCorrupt: export ownership record: {error}")
                })?)
                .map_err(|error| format!("StorageCorrupt: export ownership record: {error}"))?;
            if ownership.job_id != previous.job_id
                || ownership.generation != previous.generation
                || ownership.source_sha256 != source_sha256
                || (ownership.export_basename != reserved_basename
                    && ownership.export_basename != confirmed_basename)
            {
                return Err(
                    "StorageCorrupt: interrupted export ownership does not match reservation"
                        .into(),
                );
            }
            ownership.export_basename = confirmed_basename.to_owned();
            ownership.version = 3;
            ownership.text_file_identity = None;
            ownership.srt_file_identity = None;
            ownership.text_temporary_identity = Some(persisted_file_identity(
                &export_install_temporary_path(&directory, "txt"),
            )?);
            ownership.srt_temporary_identity = Some(persisted_file_identity(
                &export_install_temporary_path(&directory, "srt"),
            )?);
            if hex(&sha256_file(&export_install_temporary_path(
                &directory, "txt",
            ))?) != ownership.text_sha256
                || hex(&sha256_file(&export_install_temporary_path(
                    &directory, "srt",
                ))?) != ownership.srt_sha256
            {
                return Err(
                    "StorageCorrupt: staged exports changed before resume confirmation".into(),
                );
            }
            write_replace_synced(
                &ownership_path,
                &serde_json::to_vec(&ownership).map_err(|error| error.to_string())?,
            )
            .map_err(|error| {
                format!("StorageUnavailable: update interrupted export ownership: {error}")
            })?;
        }
        write_replace_synced(
            &path,
            &serde_json::to_vec(&intent).map_err(|error| error.to_string())?,
        )
        .map_err(|error| format!("StorageUnavailable: update interrupted publication: {error}"))?;
        Ok(())
    }

    pub fn clear_duplicate_reservation(
        root: &Path,
        job_id: JobId,
        generation: Generation,
    ) -> Result<(), String> {
        let path = publication_reservation_path(root, job_id, generation);
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(format!(
                "StorageUnavailable: publication reservation cleanup: {error}"
            )),
        }
    }

    pub fn reconcile_publications(root: &Path) -> Result<(), String> {
        let _lock = Self::lock_publications(root)?;
        let path = publication_index_path(root);
        if !path.is_file() {
            return Ok(());
        }
        reconcile_replaced_exports(root, &read_publication_index(root)?)
    }

    pub fn resolve_import_publication(
        root: &Path,
        job_id: JobId,
        generation: Generation,
    ) -> Result<(), String> {
        let _lock = Self::lock_publications(root)?;
        let dir = generation_path(root, job_id, generation);
        let intent = load_publication_intent(&dir)?;
        if !dir.join("CURRENT").is_file() {
            return Err("InvalidInput: publication is not complete".into());
        }
        let current = if intent.internal_only {
            Vec::new()
        } else {
            Self::published_imports(root, &intent.source_sha256)?
        };
        let expected: Vec<_> = intent
            .expected_active
            .iter()
            .map(|item| (item.job_id, item.generation))
            .collect();
        let actual: Vec<_> = current
            .iter()
            .map(|item| (item.job_id, item.generation))
            .collect();
        if !intent.internal_only && expected != actual {
            return Err(
                "DuplicateStateChanged: publication set changed; re-import to confirm".into(),
            );
        }
        if !intent.internal_only {
            install_publication_exports(root, &dir, &intent)?;
        }
        let index = apply_publication_intent(
            load_or_migrate_publication_index(root)?,
            job_id,
            generation,
            &intent,
        );
        write_publication_index(root, &index)?;
        if !intent.internal_only {
            reconcile_replaced_exports(root, &index)?;
        }
        Self::clear_duplicate_reservation(root, job_id, generation)
    }
}

pub(in crate::archive) fn publication_index_path(root: &Path) -> PathBuf {
    root.join(".whisper-publications").join("index.json")
}

pub(in crate::archive) fn publication_reservation_path(
    root: &Path,
    job_id: JobId,
    generation: Generation,
) -> PathBuf {
    root.join(".whisper-publications")
        .join("reservations")
        .join(format!("{:032x}-{}.json", job_id.0, generation.get()))
}

fn publication_reconfirmation_required_unlocked(
    root: &Path,
    job_id: JobId,
    generation: Generation,
) -> Result<Option<String>, String> {
    let reservation_path = publication_reservation_path(root, job_id, generation);
    if !reservation_path.is_file() {
        return Ok(None);
    }
    let intent: PublicationIntent = serde_json::from_slice(
        &fs::read(&reservation_path)
            .map_err(|error| format!("StorageCorrupt: publication reservation: {error}"))?,
    )
    .map_err(|error| format!("StorageCorrupt: publication reservation: {error}"))?;
    let directory = generation_path(root, job_id, generation);
    let ownership_path = installed_export_ownership_path(&directory);
    let ownership = match fs::read(&ownership_path) {
        Ok(bytes) => Some(
            serde_json::from_slice::<InstalledExportOwnership>(&bytes)
                .map_err(|error| format!("StorageCorrupt: export ownership record: {error}"))?,
        ),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(format!(
                "StorageUnavailable: export ownership record: {error}"
            ));
        }
    };
    let exports = root.join("exports");
    for extension in ["txt", "srt"] {
        let final_path = exports.join(format!("{}.{}", intent.export_basename, extension));
        if !final_path.exists() {
            continue;
        }
        let identity = persisted_file_identity(&final_path)?;
        let proven = ownership.as_ref().is_some_and(|record| {
            if record.job_id != job_id
                || record.generation != generation
                || record.source_sha256 != intent.source_sha256
                || record.export_basename != intent.export_basename
            {
                return false;
            }
            let final_identity = match extension {
                "txt" => record.text_file_identity.as_deref(),
                "srt" => record.srt_file_identity.as_deref(),
                _ => None,
            };
            if final_identity == Some(identity.as_str()) {
                return true;
            }
            let temporary_identity = match extension {
                "txt" => record.text_temporary_identity.as_deref(),
                "srt" => record.srt_temporary_identity.as_deref(),
                _ => None,
            };
            let temporary = export_install_temporary_path(&directory, extension);
            temporary_identity == Some(identity.as_str())
                && same_file::is_same_file(&final_path, &temporary).unwrap_or(false)
        });
        if !proven {
            return Ok(Some(intent.export_basename));
        }
    }
    Ok(None)
}

pub(in crate::archive) fn write_publication_reservation(
    root: &Path,
    request: &ImportRequest,
    source_sha256: &str,
    source_samples: u64,
    expected_active: Vec<PublicationIdentity>,
    choice: DuplicateImportChoice,
    export_basename: String,
) -> Result<(), String> {
    write_publication_reservation_with_storage(
        root,
        request,
        source_sha256,
        source_samples,
        expected_active,
        choice,
        export_basename,
        false,
    )
}

fn write_publication_reservation_with_storage(
    root: &Path,
    request: &ImportRequest,
    source_sha256: &str,
    source_samples: u64,
    expected_active: Vec<PublicationIdentity>,
    choice: DuplicateImportChoice,
    export_basename: String,
    internal_only: bool,
) -> Result<(), String> {
    let path = publication_reservation_path(root, request.job_id, request.generation);
    if path.exists() {
        return Err("DuplicateStateChanged: reservation already exists".into());
    }
    fs::create_dir_all(path.parent().expect("reservation parent"))
        .map_err(|error| format!("StorageUnavailable: reservation directory: {error}"))?;
    let operation_id = format!("{:032x}-{}", request.job_id.0, request.generation.get());
    let intent = PublicationIntent {
        version: 1,
        operation_id,
        choice,
        source_sha256: source_sha256.to_owned(),
        source_samples,
        expected_active,
        export_basename,
        internal_only,
        cleanup_id: None,
        cleanup_targets: Vec::new(),
    };
    write_publication_intent(root, request, &intent)
}

pub(in crate::archive) fn write_publication_intent(
    root: &Path,
    request: &ImportRequest,
    intent: &PublicationIntent,
) -> Result<(), String> {
    let path = publication_reservation_path(root, request.job_id, request.generation);
    if path.exists() {
        let existing: PublicationIntent =
            serde_json::from_slice(&fs::read(&path).map_err(|error| {
                format!("StorageCorrupt: existing publication reservation: {error}")
            })?)
            .map_err(|error| {
                format!("StorageCorrupt: existing publication reservation: {error}")
            })?;
        if existing == *intent {
            return Ok(());
        }
        return Err("DuplicateStateChanged: reservation already exists with another intent".into());
    }
    fs::create_dir_all(path.parent().expect("reservation parent"))
        .map_err(|error| format!("StorageUnavailable: reservation directory: {error}"))?;
    write_new_synced(
        &path,
        &serde_json::to_vec(intent).map_err(|error| error.to_string())?,
    )
}

pub(in crate::archive) fn load_publication_intent(
    directory: &Path,
) -> Result<PublicationIntent, String> {
    let marker = directory.join("r11-publication.json");
    if marker.is_file() {
        return serde_json::from_slice(
            &fs::read(marker)
                .map_err(|error| format!("StorageCorrupt: publication marker: {error}"))?,
        )
        .map_err(|error| format!("StorageCorrupt: publication marker: {error}"));
    }
    let pending: PendingRecord = serde_json::from_slice(
        &fs::read(directory.join("pending.json")).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("StorageCorrupt: pending publication: {error}"))?;
    let reservation = publication_reservation_path(
        directory.ancestors().nth(3).unwrap_or(directory),
        pending.job_id,
        pending.generation,
    );
    if reservation.is_file() {
        return serde_json::from_slice(&fs::read(reservation).map_err(|error| error.to_string())?)
            .map_err(|error| format!("StorageCorrupt: publication reservation: {error}"));
    }
    let source = Path::new(&pending.source_path);
    Ok(PublicationIntent {
        version: 1,
        operation_id: format!(
            "legacy-{:032x}-{}",
            pending.job_id.0,
            pending.generation.get()
        ),
        choice: DuplicateImportChoice::CreateNew,
        source_sha256: pending.source_sha256,
        source_samples: pending.source_samples,
        expected_active: Vec::new(),
        export_basename: safe_export_stem(
            source
                .file_stem()
                .and_then(|name| name.to_str())
                .unwrap_or("transcription"),
        ),
        internal_only: false,
        cleanup_id: None,
        cleanup_targets: Vec::new(),
    })
}

pub(in crate::archive) fn read_publication_index(root: &Path) -> Result<PublicationIndex, String> {
    let path = publication_index_path(root);
    let index: PublicationIndex = serde_json::from_slice(
        &fs::read(path).map_err(|error| format!("StorageCorrupt: publication index: {error}"))?,
    )
    .map_err(|error| format!("StorageCorrupt: publication index: {error}"))?;
    if index.version != 1 {
        return Err("StorageCorrupt: unsupported publication index version".into());
    }
    Ok(index)
}

pub(in crate::archive) fn write_publication_index(
    root: &Path,
    index: &PublicationIndex,
) -> Result<(), String> {
    let path = publication_index_path(root);
    fs::create_dir_all(path.parent().expect("index parent"))
        .map_err(|error| format!("StorageUnavailable: publication index directory: {error}"))?;
    write_replace_synced(
        &path,
        &serde_json::to_vec(index).map_err(|error| error.to_string())?,
    )
}

pub(in crate::archive) fn apply_publication_intent(
    mut index: PublicationIndex,
    job_id: JobId,
    generation: Generation,
    intent: &PublicationIntent,
) -> PublicationIndex {
    if intent.choice == DuplicateImportChoice::Replace {
        for record in &mut index.records {
            if record.active && record.source_sha256 == intent.source_sha256 {
                record.active = false;
            }
        }
    }
    for target in &intent.cleanup_targets {
        for record in &mut index.records {
            if record.job_id == target.treatment.job_id
                && record.generation == target.treatment.generation
                && record.source_sha256 == target.source_sha256
            {
                record.active = false;
            }
        }
    }
    let existing_ordinal = index
        .records
        .iter()
        .find(|record| record.job_id == job_id && record.generation == generation)
        .and_then(|record| record.success_ordinal);
    let next_ordinal = index
        .records
        .iter()
        .filter_map(|record| record.success_ordinal)
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    index
        .records
        .retain(|record| !(record.job_id == job_id && record.generation == generation));
    index.records.push(PublicationRecord {
        job_id,
        generation,
        source_sha256: intent.source_sha256.clone(),
        export_basename: (!intent.internal_only).then(|| intent.export_basename.clone()),
        active: true,
        success_ordinal: existing_ordinal.or(Some(next_ordinal)),
    });
    index
}

pub(in crate::archive) fn reserve_export_basename(
    root: &Path,
    source_path: &str,
) -> Result<String, String> {
    let stem = safe_export_stem(
        Path::new(source_path)
            .file_stem()
            .and_then(|name| name.to_str())
            .unwrap_or("transcription"),
    );
    let exports = root.join("exports");
    let mut reserved = std::collections::HashSet::new();
    for record in load_or_migrate_publication_index(root)?.records {
        if let Some(name) = record.export_basename {
            reserved.insert(name);
        }
    }
    let reservation_dir = root.join(".whisper-publications").join("reservations");
    if reservation_dir.exists() {
        for entry in fs::read_dir(&reservation_dir)
            .map_err(|error| format!("StorageUnavailable: reservation listing: {error}"))?
        {
            let path = entry
                .map_err(|error| format!("StorageUnavailable: reservation entry: {error}"))?
                .path();
            let intent: PublicationIntent = serde_json::from_slice(
                &fs::read(path)
                    .map_err(|error| format!("StorageCorrupt: reservation read: {error}"))?,
            )
            .map_err(|error| format!("StorageCorrupt: reservation JSON: {error}"))?;
            reserved.insert(intent.export_basename);
        }
    }
    let mut suffix = 1u64;
    loop {
        let candidate = if suffix == 1 {
            stem.clone()
        } else {
            format!("{stem} ({suffix})")
        };
        if !reserved.contains(&candidate)
            && !exports.join(format!("{candidate}.txt")).exists()
            && !exports.join(format!("{candidate}.srt")).exists()
        {
            return Ok(candidate);
        }
        suffix += 1;
    }
}

fn safe_export_stem(stem: &str) -> String {
    let cleaned: String = stem
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ' ') {
                ch
            } else {
                '_'
            }
        })
        .collect();
    let cleaned = cleaned.trim().trim_matches('.');
    if cleaned.is_empty() {
        "transcription".into()
    } else {
        cleaned.chars().take(80).collect()
    }
}

pub(in crate::archive) fn reconcile_replaced_exports(
    root: &Path,
    index: &PublicationIndex,
) -> Result<(), String> {
    let backup = root.join("transcriptions-remplacees");
    for record in index.records.iter().filter(|record| !record.active) {
        let Some(name) = &record.export_basename else {
            continue;
        };
        if index
            .records
            .iter()
            .any(|current| current.active && current.export_basename.as_ref() == Some(name))
        {
            continue;
        }
        let generation = generation_path(root, record.job_id, record.generation);
        let manifest: CompleteRecord =
            serde_json::from_slice(&fs::read(generation.join("manifest.json")).map_err(
                |error| format!("StorageCorrupt: replaced export manifest is unavailable: {error}"),
            )?)
            .map_err(|error| format!("StorageCorrupt: replaced export manifest: {error}"))?;
        if manifest.job_id != record.job_id || manifest.generation != record.generation {
            return Err("StorageCorrupt: replaced export manifest identity mismatch".into());
        }
        if manifest.source_sha256 != record.source_sha256 {
            return Err("StorageCorrupt: replaced export source identity mismatch".into());
        }
        verify_artifact(&generation, &manifest.text)?;
        verify_artifact(&generation, &manifest.srt)?;
        let ownership_path = installed_export_ownership_path(&generation);
        let Ok(ownership_bytes) = fs::read(&ownership_path) else {
            // Legacy exports without an app-written provenance record remain
            // visible. Their content alone cannot establish ownership.
            continue;
        };
        let ownership: InstalledExportOwnership = serde_json::from_slice(&ownership_bytes)
            .map_err(|error| format!("StorageCorrupt: export ownership record: {error}"))?;
        if !matches!(ownership.version, 1 | 2 | 3)
            || ownership.job_id != record.job_id
            || ownership.generation != record.generation
            || ownership.source_sha256 != record.source_sha256
            || ownership.export_basename != *name
            || ownership.text_sha256 != manifest.text.sha256
            || ownership.srt_sha256 != manifest.srt.sha256
        {
            return Err("StorageCorrupt: replaced export provenance mismatch".into());
        }
        fs::create_dir_all(&backup)
            .map_err(|error| format!("StorageUnavailable: replaced export backup: {error}"))?;
        let suffix = format!(
            "{}-{}",
            format!("{:032x}", record.job_id.0),
            record.generation.get()
        );
        for (ext, artifact) in [("txt", &manifest.text), ("srt", &manifest.srt)] {
            let source = root.join("exports").join(format!("{name}.{ext}"));
            if !source.exists() {
                continue;
            }
            let expected_owned_hash = if ext == "txt" {
                &ownership.text_sha256
            } else {
                &ownership.srt_sha256
            };
            let identity_matches = match (ownership.version, ext) {
                (1, _) => same_file::is_same_file(
                    &source,
                    generation.join(format!("export.{ext}.pending")),
                )
                .unwrap_or(false),
                (2, "txt") => {
                    ownership.text_file_identity.as_deref()
                        == Some(persisted_file_identity(&source)?.as_str())
                }
                (2, "srt") => {
                    ownership.srt_file_identity.as_deref()
                        == Some(persisted_file_identity(&source)?.as_str())
                }
                (3, "txt") => {
                    ownership.text_file_identity.as_deref()
                        == Some(persisted_file_identity(&source)?.as_str())
                }
                (3, "srt") => {
                    ownership.srt_file_identity.as_deref()
                        == Some(persisted_file_identity(&source)?.as_str())
                }
                _ => false,
            };
            if hex(&sha256_file(&source)?) != artifact.sha256
                || &artifact.sha256 != expected_owned_hash
                || !identity_matches
            {
                // Another file may now occupy this visible name. Never move it as if
                // it belonged to the replaced generation.
                continue;
            }
            let target = backup.join(format!("{name}-{suffix}.{ext}"));
            if target.exists() {
                continue;
            }
            fs::rename(source, target)
                .map_err(|error| format!("StorageUnavailable: archive replaced export: {error}"))?;
        }
    }
    Ok(())
}
