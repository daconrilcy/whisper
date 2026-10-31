use super::super::{
    ArtifactRecord, CompleteRecord, Digest, File, Generation, Hash, Hasher,
    InstalledExportOwnership, JobId, OpenOptions, Path, PathBuf, PublicationIntent, Sha256, fs,
    hex, installed_export_ownership_path, io, parse_job_id, publication_index_path,
    read_publication_index, sha256_file, verify_artifact, write_new_synced, write_replace_synced,
};
#[cfg(test)]
use super::super::{publication_copy_fault_after, publication_copy_fault_applies};

pub(in crate::archive) fn install_publication_exports(
    root: &Path,
    directory: &Path,
    intent: &PublicationIntent,
) -> Result<(), String> {
    install_publication_exports_with(root, directory, intent, &|source, target| {
        fs::hard_link(source, target)
    })
}

pub(in crate::archive) fn install_publication_exports_with(
    root: &Path,
    directory: &Path,
    intent: &PublicationIntent,
    hard_linker: &dyn Fn(&Path, &Path) -> io::Result<()>,
) -> Result<(), String> {
    #[cfg(test)]
    let force_copy = publication_copy_fault_applies(root);
    #[cfg(test)]
    let link = |source: &Path, target: &Path| {
        if force_copy {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "test forces copy fallback",
            ))
        } else {
            hard_linker(source, target)
        }
    };
    #[cfg(not(test))]
    let link = |source: &Path, target: &Path| hard_linker(source, target);

    #[cfg(test)]
    let after_copy = |extension: &str, _path: &Path| publication_copy_fault_after(root, extension);
    #[cfg(not(test))]
    let after_copy = |_: &str, _: &Path| Ok(());

    install_publication_exports_with_copy_hook(root, directory, intent, &link, &after_copy)
}

pub(in crate::archive) fn install_publication_exports_with_copy_hook(
    root: &Path,
    directory: &Path,
    intent: &PublicationIntent,
    hard_linker: &dyn Fn(&Path, &Path) -> io::Result<()>,
    after_final_copy: &dyn Fn(&str, &Path) -> Result<(), String>,
) -> Result<(), String> {
    let ops = ExportInstallOps {
        hard_linker,
        after_final_copy,
    };
    let manifest: CompleteRecord = serde_json::from_slice(
        &fs::read(directory.join("manifest.json"))
            .map_err(|error| format!("StorageCorrupt: export manifest: {error}"))?,
    )
    .map_err(|error| format!("StorageCorrupt: export manifest: {error}"))?;
    if manifest.job_id != intent_job_id(directory)?
        || manifest.generation != intent_generation(directory)?
        || (!intent.source_sha256.is_empty() && manifest.source_sha256 != intent.source_sha256)
    {
        return Err("StorageCorrupt: export manifest identity mismatch".into());
    }
    let exports = root.join("exports");
    fs::create_dir_all(&exports)
        .map_err(|error| format!("StorageUnavailable: exports: {error}"))?;
    verify_artifact(directory, &manifest.text)?;
    verify_artifact(directory, &manifest.srt)?;
    let mut ownership = InstalledExportOwnership {
        version: 1,
        job_id: manifest.job_id,
        generation: manifest.generation,
        source_sha256: manifest.source_sha256.clone(),
        export_basename: intent.export_basename.clone(),
        text_sha256: manifest.text.sha256.clone(),
        srt_sha256: manifest.srt.sha256.clone(),
        text_file_identity: None,
        srt_file_identity: None,
        text_temporary_identity: None,
        srt_temporary_identity: None,
    };
    let ownership_path = installed_export_ownership_path(directory);
    if ownership_path.exists() {
        let existing: InstalledExportOwnership = serde_json::from_slice(
            &fs::read(&ownership_path)
                .map_err(|error| format!("StorageCorrupt: export ownership record: {error}"))?,
        )
        .map_err(|error| format!("StorageCorrupt: export ownership record: {error}"))?;
        if !same_export_ownership(&existing, &ownership) {
            return Err("StorageCorrupt: export ownership identity mismatch".into());
        }
        if existing.version == 1 {
            verify_owned_export(&exports, directory, &existing, &manifest.text, "txt")?;
            verify_owned_export(&exports, directory, &existing, &manifest.srt, "srt")?;
            return Ok(());
        }
        if !matches!(existing.version, 2 | 3) {
            return Err("StorageCorrupt: unsupported export ownership version".into());
        }
        ownership = existing;
        if ownership.version == 2 {
            ownership.text_temporary_identity = ownership.text_file_identity.clone();
            ownership.srt_temporary_identity = ownership.srt_file_identity.clone();
            ownership.version = 3;
            write_replace_synced(
                &ownership_path,
                &serde_json::to_vec(&ownership).map_err(|error| error.to_string())?,
            )?;
        }
        install_owned_export(
            &exports,
            directory,
            &ownership_path,
            &mut ownership,
            &manifest.text,
            "txt",
            &ops,
        )?;
        install_owned_export(
            &exports,
            directory,
            &ownership_path,
            &mut ownership,
            &manifest.srt,
            "srt",
            &ops,
        )?;
        verify_owned_export(&exports, directory, &ownership, &manifest.text, "txt")?;
        verify_owned_export(&exports, directory, &ownership, &manifest.srt, "srt")?;
        return Ok(());
    }

    // First reject collisions before creating any temporary output. A byte
    // match alone never proves that an existing path belongs to this app.
    for extension in ["txt", "srt"] {
        let final_path = exports.join(format!("{}.{}", intent.export_basename, extension));
        if final_path.exists() {
            let staged_export = directory.join(format!("export.{extension}.pending"));
            if same_file::is_same_file(&final_path, &staged_export).unwrap_or(false) {
                continue;
            }
            if !is_archived_export_file(root, &final_path, extension)? {
                return Err(format!(
                    "StorageUnavailable: export name collision for {extension}"
                ));
            }
            let index_path = publication_index_path(root);
            if index_path.exists()
                && read_publication_index(root)?.records.iter().any(|record| {
                    record.active
                        && record.export_basename.as_deref()
                            == Some(intent.export_basename.as_str())
                })
            {
                return Err(format!(
                    "StorageUnavailable: active export name collision for {extension}"
                ));
            }
            fs::remove_file(&final_path).map_err(|error| {
                format!("StorageUnavailable: remove verified orphan export: {error}")
            })?;
        }
    }

    // Build owned temporary files on the destination volume. Prefer hard
    // links, with exclusive copies as the filesystem fallback. Final install
    // also uses a no-clobber hard link, then create_new copy if unsupported.
    for (extension, artifact) in [("txt", &manifest.text), ("srt", &manifest.srt)] {
        let final_path = exports.join(format!("{}.{}", intent.export_basename, extension));
        let staged_export = directory.join(format!("export.{extension}.pending"));
        if hex(&sha256_file(&staged_export)?) != artifact.sha256 {
            return Err(format!(
                "StorageCorrupt: staged {extension} export failed verification"
            ));
        }
        if final_path.exists()
            && same_file::is_same_file(&final_path, &staged_export).unwrap_or(false)
        {
            let identity = persisted_file_identity(&final_path)?;
            set_export_file_identity(&mut ownership, extension, identity.clone());
            set_export_temporary_identity(&mut ownership, extension, identity);
            continue;
        }
        let temporary = export_install_temporary_path(directory, extension);
        if temporary.exists() {
            fs::remove_file(&temporary).map_err(|error| {
                format!("StorageUnavailable: clear stale {extension} export temporary: {error}")
            })?;
        }
        if hard_linker(&staged_export, &temporary).is_err() {
            copy_file_create_new(&staged_export, &temporary)?;
        }
        if hex(&sha256_file(&temporary)?) != artifact.sha256 {
            return Err(format!(
                "StorageCorrupt: temporary {extension} export failed verification"
            ));
        }
        let identity = persisted_file_identity(&temporary)?;
        set_export_temporary_identity(&mut ownership, extension, identity);
    }
    ownership.version = 3;
    write_new_synced(
        &ownership_path,
        &serde_json::to_vec(&ownership).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("StorageUnavailable: export ownership record: {error}"))?;
    install_owned_export(
        &exports,
        directory,
        &ownership_path,
        &mut ownership,
        &manifest.text,
        "txt",
        &ops,
    )?;
    install_owned_export(
        &exports,
        directory,
        &ownership_path,
        &mut ownership,
        &manifest.srt,
        "srt",
        &ops,
    )?;
    verify_owned_export(&exports, directory, &ownership, &manifest.text, "txt")?;
    verify_owned_export(&exports, directory, &ownership, &manifest.srt, "srt")?;
    Ok(())
}

fn same_export_ownership(
    existing: &InstalledExportOwnership,
    expected: &InstalledExportOwnership,
) -> bool {
    existing.job_id == expected.job_id
        && existing.generation == expected.generation
        && existing.source_sha256 == expected.source_sha256
        && existing.export_basename == expected.export_basename
        && existing.text_sha256 == expected.text_sha256
        && existing.srt_sha256 == expected.srt_sha256
}

pub(in crate::archive) fn export_install_temporary_path(
    directory: &Path,
    extension: &str,
) -> PathBuf {
    directory.join(format!("export.{extension}.install.pending"))
}

pub(in crate::archive) fn persisted_file_identity(path: &Path) -> Result<String, String> {
    let handle = same_file::Handle::from_path(path)
        .map_err(|error| format!("StorageUnavailable: export file identity: {error}"))?;
    let mut hasher = FileIdentityHasher::default();
    handle.hash(&mut hasher);
    Ok(hex(&Sha256::digest(hasher.bytes)))
}

#[derive(Default)]
struct FileIdentityHasher {
    bytes: Vec<u8>,
}

impl Hasher for FileIdentityHasher {
    fn finish(&self) -> u64 {
        0
    }

    fn write(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }

    fn write_u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    fn write_u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn write_usize(&mut self, value: usize) {
        self.write_u64(value as u64);
    }
}

fn set_export_file_identity(
    ownership: &mut InstalledExportOwnership,
    extension: &str,
    identity: String,
) {
    match extension {
        "txt" => ownership.text_file_identity = Some(identity),
        "srt" => ownership.srt_file_identity = Some(identity),
        _ => unreachable!("validated export extension"),
    }
}

fn set_export_temporary_identity(
    ownership: &mut InstalledExportOwnership,
    extension: &str,
    identity: String,
) {
    match extension {
        "txt" => ownership.text_temporary_identity = Some(identity),
        "srt" => ownership.srt_temporary_identity = Some(identity),
        _ => unreachable!("validated export extension"),
    }
}

fn copy_file_create_new(source: &Path, target: &Path) -> Result<(), String> {
    let mut source_file = File::open(source)
        .map_err(|error| format!("StorageUnavailable: open staged export copy: {error}"))?;
    let mut target_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)
        .map_err(|error| format!("StorageUnavailable: create export copy: {error}"))?;
    io::copy(&mut source_file, &mut target_file)
        .and_then(|_| target_file.sync_all())
        .map_err(|error| format!("StorageUnavailable: sync export copy: {error}"))
}

struct ExportInstallOps<'a> {
    hard_linker: &'a dyn Fn(&Path, &Path) -> io::Result<()>,
    after_final_copy: &'a dyn Fn(&str, &Path) -> Result<(), String>,
}

fn install_owned_export(
    exports: &Path,
    generation: &Path,
    ownership_path: &Path,
    ownership: &mut InstalledExportOwnership,
    artifact: &ArtifactRecord,
    extension: &str,
    ops: &ExportInstallOps<'_>,
) -> Result<(), String> {
    let expected_final_identity = match extension {
        "txt" => ownership.text_file_identity.clone(),
        "srt" => ownership.srt_file_identity.clone(),
        _ => return Err("StorageCorrupt: unsupported owned export type".into()),
    };
    let expected_temporary_identity = match extension {
        "txt" => ownership.text_temporary_identity.clone(),
        "srt" => ownership.srt_temporary_identity.clone(),
        _ => return Err("StorageCorrupt: unsupported owned export type".into()),
    }
    .ok_or_else(|| format!("StorageCorrupt: missing {extension} temporary identity"))?;
    let final_path = exports.join(format!("{}.{}", ownership.export_basename, extension));
    if final_path.exists() {
        let actual_identity = persisted_file_identity(&final_path)?;
        if expected_final_identity.as_deref() == Some(actual_identity.as_str()) {
            if hex(&sha256_file(&final_path)?) != artifact.sha256 {
                return Err(format!(
                    "StorageCorrupt: installed {extension} export failed verification"
                ));
            }
            return Ok(());
        }
        let temporary = export_install_temporary_path(generation, extension);
        if ownership.version == 3
            && actual_identity == expected_temporary_identity
            && same_file::is_same_file(&final_path, &temporary).unwrap_or(false)
        {
            set_export_file_identity(ownership, extension, actual_identity);
            write_replace_synced(
                ownership_path,
                &serde_json::to_vec(ownership).map_err(|error| error.to_string())?,
            )?;
            return Ok(());
        }
        if expected_final_identity.is_none() {
            return Err(format!(
                "StorageUnavailable: export name collision for {extension}"
            ));
        }
        return Err(format!(
            "StorageUnavailable: export name collision for {extension}"
        ));
    }
    let temporary = export_install_temporary_path(generation, extension);
    if !temporary.is_file()
        || persisted_file_identity(&temporary)? != expected_temporary_identity
        || hex(&sha256_file(&temporary)?) != artifact.sha256
    {
        return Err(format!(
            "StorageCorrupt: owned {extension} export temporary is unavailable or changed"
        ));
    }
    if (ops.hard_linker)(&temporary, &final_path).is_ok() {
        set_export_file_identity(ownership, extension, expected_temporary_identity.clone());
    } else {
        if final_path.exists() {
            return Err(format!(
                "StorageUnavailable: export name collision for {extension}"
            ));
        }
        copy_file_create_new(&temporary, &final_path)?;
        (ops.after_final_copy)(extension, &final_path)?;
        set_export_file_identity(ownership, extension, persisted_file_identity(&final_path)?);
    }
    write_replace_synced(
        ownership_path,
        &serde_json::to_vec(ownership).map_err(|error| error.to_string())?,
    )?;
    Ok(())
}

fn intent_job_id(directory: &Path) -> Result<JobId, String> {
    let (job_id, _) = parse_generation_directory(directory)?;
    Ok(job_id)
}

fn intent_generation(directory: &Path) -> Result<Generation, String> {
    let (_, generation) = parse_generation_directory(directory)?;
    Ok(generation)
}

fn parse_generation_directory(directory: &Path) -> Result<(JobId, Generation), String> {
    let generation = directory
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.parse::<u64>().ok())
        .filter(|value| *value > 0)
        .ok_or_else(|| "StorageCorrupt: non-canonical export generation path".to_owned())?;
    let job_id = directory
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .and_then(parse_job_id)
        .ok_or_else(|| "StorageCorrupt: non-canonical export job path".to_owned())?;
    let generation = serde_json::from_value(serde_json::Value::from(generation))
        .map_err(|_| "StorageCorrupt: invalid export generation".to_owned())?;
    Ok((job_id, generation))
}

fn verify_owned_export(
    exports: &Path,
    generation: &Path,
    ownership: &InstalledExportOwnership,
    artifact: &ArtifactRecord,
    extension: &str,
) -> Result<(), String> {
    let expected_hash = match extension {
        "txt" => &ownership.text_sha256,
        "srt" => &ownership.srt_sha256,
        _ => return Err("StorageCorrupt: unsupported owned export type".into()),
    };
    if expected_hash != &artifact.sha256 {
        return Err(format!(
            "StorageCorrupt: owned {extension} export provenance differs from manifest"
        ));
    }
    let path = exports.join(format!("{}.{}", ownership.export_basename, extension));
    let actual = hex(&sha256_file(&path)
        .map_err(|error| format!("StorageUnavailable: owned {extension} export: {error}"))?);
    if &actual != expected_hash {
        return Err(format!(
            "StorageCorrupt: owned {extension} export no longer matches its provenance"
        ));
    }
    let staged_export = generation.join(format!("export.{extension}.pending"));
    if hex(&sha256_file(&staged_export).map_err(|error| {
        format!("StorageUnavailable: verify staged {extension} identity: {error}")
    })?) != *expected_hash
    {
        return Err(format!(
            "StorageCorrupt: staged {extension} export no longer matches its provenance"
        ));
    }
    let identity_matches = match (ownership.version, extension) {
        (2, "txt") => {
            ownership.text_file_identity.as_deref()
                == Some(persisted_file_identity(&path)?.as_str())
        }
        (2, "srt") => {
            ownership.srt_file_identity.as_deref() == Some(persisted_file_identity(&path)?.as_str())
        }
        (3, "txt") => {
            ownership.text_file_identity.as_deref()
                == Some(persisted_file_identity(&path)?.as_str())
        }
        (3, "srt") => {
            ownership.srt_file_identity.as_deref() == Some(persisted_file_identity(&path)?.as_str())
        }
        (1, _) => same_file::is_same_file(&path, &staged_export).map_err(|error| {
            format!("StorageUnavailable: verify owned {extension} identity: {error}")
        })?,
        _ => false,
    };
    let temporary_identity_matches = match (ownership.version, extension) {
        (3, "txt") => {
            ownership.text_temporary_identity.as_deref()
                == Some(
                    persisted_file_identity(&export_install_temporary_path(generation, extension))?
                        .as_str(),
                )
        }
        (3, "srt") => {
            ownership.srt_temporary_identity.as_deref()
                == Some(
                    persisted_file_identity(&export_install_temporary_path(generation, extension))?
                        .as_str(),
                )
        }
        _ => true,
    };
    if !identity_matches || !temporary_identity_matches {
        return Err(format!(
            "StorageCorrupt: owned {extension} export has different file identity (final_match={identity_matches}, temporary_match={temporary_identity_matches})"
        ));
    }
    Ok(())
}

fn is_archived_export_file(root: &Path, candidate: &Path, extension: &str) -> Result<bool, String> {
    let transcriptions = root.join("transcriptions");
    if !transcriptions.is_dir() {
        return Ok(false);
    }
    for job_entry in fs::read_dir(&transcriptions)
        .map_err(|error| format!("StorageUnavailable: export provenance jobs: {error}"))?
    {
        let job_dir = job_entry
            .map_err(|error| format!("StorageUnavailable: export provenance job: {error}"))?
            .path();
        if !job_dir.is_dir() {
            continue;
        }
        for generation_entry in fs::read_dir(&job_dir).map_err(|error| {
            format!("StorageUnavailable: export provenance generations: {error}")
        })? {
            let generation = generation_entry
                .map_err(|error| {
                    format!("StorageUnavailable: export provenance generation: {error}")
                })?
                .path();
            if !generation.is_dir() {
                continue;
            }
            let Ok(bytes) = fs::read(generation.join("manifest.json")) else {
                continue;
            };
            let Ok(manifest) = serde_json::from_slice::<CompleteRecord>(&bytes) else {
                continue;
            };
            let (Ok(job_id), Ok(generation_id)) =
                (intent_job_id(&generation), intent_generation(&generation))
            else {
                continue;
            };
            if manifest.job_id != job_id || manifest.generation != generation_id {
                continue;
            }
            let artifact = match extension {
                "txt" => &manifest.text,
                "srt" => &manifest.srt,
                _ => continue,
            };
            if verify_artifact(&generation, artifact).is_err() {
                continue;
            }
            let staged_export = generation.join(format!("export.{extension}.pending"));
            if hex(&sha256_file(&staged_export)?) != artifact.sha256 {
                continue;
            }
            let owned = match fs::read(installed_export_ownership_path(&generation)) {
                Ok(ownership_bytes) => {
                    let Ok(ownership) =
                        serde_json::from_slice::<InstalledExportOwnership>(&ownership_bytes)
                    else {
                        continue;
                    };
                    let expected_name = format!("{}.{}", ownership.export_basename, extension);
                    let expected_hash = match extension {
                        "txt" => &ownership.text_sha256,
                        "srt" => &ownership.srt_sha256,
                        _ => continue,
                    };
                    if ownership.job_id != job_id
                        || ownership.generation != generation_id
                        || candidate.file_name().and_then(|name| name.to_str())
                            != Some(&expected_name)
                        || expected_hash != &artifact.sha256
                    {
                        continue;
                    }
                    match ownership.version {
                        1 => same_file::is_same_file(candidate, &staged_export).unwrap_or(false),
                        2 | 3 => {
                            let expected_identity = match extension {
                                "txt" => ownership.text_file_identity.as_deref(),
                                "srt" => ownership.srt_file_identity.as_deref(),
                                _ => None,
                            };
                            expected_identity.is_some_and(|expected| {
                                persisted_file_identity(candidate)
                                    .is_ok_and(|actual| actual == expected)
                            })
                        }
                        _ => false,
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    // A crash may leave a legacy R11 hard link before its
                    // ownership record was written. Only its file identity,
                    // never content equality, establishes that provenance.
                    same_file::is_same_file(candidate, &staged_export).unwrap_or(false)
                }
                Err(_) => false,
            };
            if owned {
                return Ok(true);
            }
        }
    }
    Ok(false)
}
