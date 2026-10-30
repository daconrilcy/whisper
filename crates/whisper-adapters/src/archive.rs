use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    hash::{Hash, Hasher},
    io::{self, BufRead, BufReader, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::Arc,
    time::Instant,
};
use whisper_core::{
    Generation, JobId, SourceRange,
    ports::{
        DuplicateImportChoice, ImportRequest, LiveRequest, PublishedImport, ReplacementCleanupInfo,
        TreatmentIdentity, WorkerSegment,
    },
};

mod campaign;
mod identity;
mod import;
mod journal;
mod live;
mod publication;
mod recovery;
mod source;
mod storage;

#[cfg(all(test, feature = "recette-hooks"))]
use campaign::campaign_archive_snapshot;
pub(crate) use campaign::{
    assert_campaign_archive, campaign_gate, record_campaign_operation_result,
};
#[cfg(test)]
use identity::canonical_generation_directory;
use identity::{
    is_r5_2_vad_recipe_destination, is_registered_model, is_sha256, parse_job_id,
    safe_artifact_path, verified_artifact, verify_artifact, verify_pcm_staging,
};
use journal::{
    read_segments, validate_segment_journal, validate_unified_resume_prefix, visit_segments,
    visit_segments_read_only,
};
use publication::{
    apply_publication_intent, export_install_temporary_path, install_publication_exports,
    load_publication_intent, persisted_file_identity, publication_index_path,
    publication_reservation_path, read_publication_index, reserve_export_basename,
    write_publication_index, write_publication_intent,
};
#[cfg(test)]
use publication::{
    install_publication_exports_with, install_publication_exports_with_copy_hook,
    reconcile_replaced_exports, write_publication_reservation,
};
#[cfg(test)]
use recovery::scan_generation;
pub use recovery::scan_live_recoveries;
use recovery::{
    initialize_pending, live_current_generation, read_import_vad_windows,
    read_verified_live_manifest, reconcile_alias, scan_live, validate_pending,
};
pub use source::sha256_file;
pub(crate) use source::source_output_samples;
use source::{hex, segmented_pcm_manifest};
use storage::{
    copy_replace_streamed, write_new_synced, write_replace_streamed, write_replace_synced,
};

#[cfg(test)]
include!("archive/tests/publication_support.rs");
const PCM_STAGING_HEADER: &[u8] = b"WHISPCM1\0\x80\x3e\0\0\x01\0";
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SegmentRecord {
    pub sequence: u64,
    pub range: SourceRange,
    pub text: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct PendingRecord {
    version: u32,
    job_id: JobId,
    generation: Generation,
    source_path: String,
    source_sha256: String,
    source_samples: u64,
    model_path: String,
    model_sha256: String,
    model_size: u64,
    #[serde(default = "legacy_transcription_strategy")]
    strategy: whisper_core::parakeet_session::TranscriptionStrategy,
    #[serde(default)]
    qualification: Option<ArchiveQualificationKey>,
}

fn legacy_transcription_strategy() -> whisper_core::parakeet_session::TranscriptionStrategy {
    whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct ImportVadWindowPayload {
    version: u32,
    job_id: JobId,
    generation: Generation,
    sequence: u64,
    central_start_sample: u64,
    central_end_sample: u64,
    native_start_sample: u64,
    native_end_sample: u64,
    last_segment_sequence: u64,
    qualification: ArchiveQualificationKey,
}

#[derive(Serialize, Deserialize)]
struct ImportVadWindowRecord {
    payload: ImportVadWindowPayload,
    sha256: String,
}

const PARAKEET_L2B_VAD_WORKER_SHA256: &str =
    "e7672a520db87926b3da9b8e5b585a1e7e57ce53ff8b1d0fe95251046aa01f4d";
const PARAKEET_L2B_RUNTIME_SHA256: &str =
    "d5a2cc8ef441dd5934972adae00f997241bea17a7bc5977a8c7f0122aaa958ff";
const PARAKEET_L2A_WORKER_SHA256: &str =
    "545a4830755da613a1255be97fbc6a6a513ef837d669d8a9870d3bbee9dfd587";
const PARAKEET_R5_2_WORKER_SHA256: &str =
    "e7672a520db87926b3da9b8e5b585a1e7e57ce53ff8b1d0fe95251046aa01f4d";

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ArchiveQualificationKey {
    pub model: String,
    pub artifact: String,
    pub artifact_revision: String,
    pub runtime: String,
    pub runtime_build: String,
    pub worker: String,
    pub worker_build: String,
    pub worker_protocol: u32,
    pub backend: String,
    pub inference_profile: String,
    #[serde(default)]
    pub gpu_pci_bus_id: Option<String>,
    #[serde(default)]
    pub gpu_uuid: Option<String>,
    #[serde(default)]
    pub gpu_pnp_instance_id: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct ArtifactRecord {
    path: String,
    sha256: String,
    bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct CompleteRecord {
    version: u32,
    job_id: JobId,
    generation: Generation,
    source_sha256: String,
    segments: u64,
    text: ArtifactRecord,
    srt: ArtifactRecord,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct InstalledExportOwnership {
    version: u32,
    job_id: JobId,
    generation: Generation,
    source_sha256: String,
    export_basename: String,
    text_sha256: String,
    srt_sha256: String,
    #[serde(default)]
    text_file_identity: Option<String>,
    #[serde(default)]
    srt_file_identity: Option<String>,
    #[serde(default)]
    text_temporary_identity: Option<String>,
    #[serde(default)]
    srt_temporary_identity: Option<String>,
}

fn installed_export_ownership_path(directory: &Path) -> PathBuf {
    directory.join("installed-exports.json")
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArchiveEntry {
    pub job_id: JobId,
    pub generation: Generation,
    pub complete: bool,
    pub source_sha256: String,
    pub diagnostic: Option<String>,
    pub export_basename: Option<String>,
    pub publication_recovery: bool,
    pub strategy: whisper_core::parakeet_session::TranscriptionStrategy,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct PublicationIndex {
    version: u32,
    records: Vec<PublicationRecord>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct PublicationRecord {
    job_id: JobId,
    generation: Generation,
    source_sha256: String,
    export_basename: Option<String>,
    active: bool,
    #[serde(default)]
    success_ordinal: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct PublicationIdentity {
    job_id: JobId,
    generation: Generation,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct PublicationIntent {
    version: u32,
    operation_id: String,
    choice: DuplicateImportChoice,
    source_sha256: String,
    source_samples: u64,
    expected_active: Vec<PublicationIdentity>,
    export_basename: String,
    #[serde(default)]
    internal_only: bool,
    #[serde(default)]
    cleanup_id: Option<String>,
    #[serde(default)]
    cleanup_targets: Vec<CleanupTarget>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct CleanupObjectIdentity {
    volume_serial: u64,
    file_id: [u8; 16],
    is_directory: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct CleanupObject {
    relative_path: String,
    identity: CleanupObjectIdentity,
    sha256: Option<String>,
    #[serde(default = "cleanup_object_owned_default")]
    owned: bool,
    removed: bool,
    error: Option<String>,
}

fn cleanup_object_owned_default() -> bool {
    true
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct CleanupAncestor {
    relative_path: String,
    identity: CleanupObjectIdentity,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct CleanupTarget {
    treatment: TreatmentIdentity,
    source_sha256: String,
    ancestors: Vec<CleanupAncestor>,
    objects: Vec<CleanupObject>,
    #[serde(default)]
    container: Option<CleanupObject>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct CleanupPlan {
    version: u32,
    cleanup_id: String,
    destination: String,
    source_path: String,
    source_sha256: String,
    successor: TreatmentIdentity,
    successor_directory: CleanupObjectIdentity,
    #[serde(default)]
    successor_files: Vec<CleanupObject>,
    #[serde(default)]
    successor_index: Option<CleanupObject>,
    published: bool,
    targets: Vec<CleanupTarget>,
    last_error: Option<String>,
}

impl Default for PublicationIntent {
    fn default() -> Self {
        Self {
            version: 1,
            operation_id: "legacy-test".into(),
            choice: DuplicateImportChoice::CreateNew,
            source_sha256: String::new(),
            source_samples: 0,
            expected_active: Vec::new(),
            export_basename: "transcription".into(),
            internal_only: false,
            cleanup_id: None,
            cleanup_targets: Vec::new(),
        }
    }
}

pub struct PublicationLock {
    _file: File,
}

pub struct ArchiveStore {
    root: PathBuf,
    current: Option<ActiveArchive>,
    export_hard_linker: Arc<dyn Fn(&Path, &Path) -> io::Result<()> + Send + Sync>,
    unified_resume_checkpoint: Option<UnifiedArchiveResumeCheckpoint>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnifiedArchiveResumeCheckpoint {
    pub pipeline: crate::parakeet_pipeline::ParakeetResumeCheckpoint,
    pub previous_document_directory: PathBuf,
    pub previous_job_id: JobId,
    pub previous_generation: Generation,
}

#[derive(Serialize, Deserialize)]
struct LiveCompleteRecord {
    version: u32,
    job_id: JobId,
    generation: Generation,
    audio_samples: u64,
    #[serde(default)]
    confirmed_samples: u64,
    group_offset_samples: u64,
    completed_at_unix_ms: u64,
    pcm_sha256: String,
    staging: ArtifactRecord,
    mp3: ArtifactRecord,
    text: ArtifactRecord,
    srt: ArtifactRecord,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct CoverageRecordPayload {
    version: u32,
    job_id: JobId,
    generation: Generation,
    window_sequence: u64,
    start_sample: u64,
    end_sample: u64,
    last_segment_sequence: u64,
}

#[derive(Serialize, Deserialize)]
struct CoverageRecord {
    payload: CoverageRecordPayload,
    sha256: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum LiveAttemptState {
    Reserved,
    Started,
    Retired,
}

#[derive(Serialize, Deserialize)]
struct LiveAttemptRecord {
    payload: LiveAttemptPayload,
    sha256: String,
}

#[derive(Serialize, Deserialize)]
struct LiveAttemptPayload {
    version: u32,
    journal_sequence: u64,
    job_id: JobId,
    generation: Generation,
    attempt: u64,
    state: LiveAttemptState,
}

/// Passage-oriented durable publisher for live jobs. `CURRENT` is written only after
/// the MP3, cumulative transcript, subtitle, and manifest have been synced and verified.
pub struct LiveArchive {
    directory: PathBuf,
    request: LiveRequest,
    _writer_lock: File,
    pending_path: PathBuf,
    mp3_pending_path: PathBuf,
    mp3: Option<File>,
    expected_sequence: u64,
    expected_window_sequence: u64,
    confirmed_samples: u64,
    confirmed_segment_count: usize,
    next_attempt: u64,
    next_attempt_record_sequence: u64,
    active_attempt: Option<(u64, LiveAttemptState)>,
    segments: Vec<SegmentRecord>,
    previous_completed_at_ms: Option<u64>,
}

struct ActiveArchive {
    job_id: JobId,
    generation: Generation,
    directory: PathBuf,
    source_sha256: [u8; 32],
    source_samples: u64,
    strategy: whisper_core::parakeet_session::TranscriptionStrategy,
    expected_sequence: u64,
    publication: PublicationIntent,
    segmented_source_manifest: Option<(PathBuf, [u8; 32])>,
}

#[derive(Serialize, Deserialize)]
struct CapturedSourceManifest {
    version: u32,
    source_sha256: String,
    source_samples: u64,
    index_path: String,
    index_sha256: String,
    parts: Vec<CapturedPartReference>,
}

#[derive(Serialize, Deserialize)]
struct CapturedPartReference {
    number: u64,
    start_sample: u64,
    end_sample: u64,
    path: String,
    sha256: String,
    bytes: u64,
}

impl ArchiveStore {
    #[cfg(windows)]
    pub fn browse_treatment_folder(destination: &Path) -> Option<String> {
        let initial = destination.join("transcriptions");
        rfd::FileDialog::new()
            .set_title("Choisir le traitement Whisper à remplacer")
            .set_directory(if initial.is_dir() {
                initial
            } else {
                destination.to_path_buf()
            })
            .pick_folder()
            .map(|path| path.to_string_lossy().into_owned())
    }

    #[cfg(not(windows))]
    pub fn browse_treatment_folder(_destination: &Path) -> Option<String> {
        None
    }

    pub fn replacement_cleanups(root: &Path) -> Result<Vec<ReplacementCleanupInfo>, String> {
        let directory = cleanup_plans_path(root);
        if !directory.exists() {
            return Ok(Vec::new());
        }
        // Discovery may recover a publication that reached CURRENT but did
        // not finish writing its index/cleanup proof. Serialize that replay
        // with all other publication-index writers to prevent lost updates.
        let _lock = Self::lock_publications(root)?;
        let mut pending = Vec::new();
        for entry in fs::read_dir(&directory)
            .map_err(|error| format!("StorageUnavailable: lister les reprises: {error}"))?
        {
            let entry =
                entry.map_err(|error| format!("StorageUnavailable: entrée reprise: {error}"))?;
            if entry.path().extension().and_then(|value| value.to_str()) != Some("json") {
                continue;
            }
            let mut plan: CleanupPlan = serde_json::from_slice(
                &fs::read(entry.path())
                    .map_err(|error| format!("StorageCorrupt: lire la reprise: {error}"))?,
            )
            .map_err(|error| format!("StorageCorrupt: reprise JSON: {error}"))?;
            if !plan.published && recover_published_cleanup_successor(root, &mut plan)? {
                plan.published = true;
                write_cleanup_plan(root, &plan)?;
            }
            if !plan.published {
                continue;
            }
            let remaining = cleanup_remaining(&plan);
            let preserved = cleanup_preserved(root, &plan);
            if remaining.is_empty() && preserved.is_empty() {
                continue;
            }
            pending.push(ReplacementCleanupInfo {
                cleanup_id: plan.cleanup_id,
                successor: plan.successor,
                remaining,
                preserved,
                last_error: plan.last_error,
            });
        }
        pending.sort_by(|a, b| a.cleanup_id.cmp(&b.cleanup_id));
        Ok(pending)
    }

    pub fn resume_replacement_cleanup(root: &Path, cleanup_id: &str) -> Result<(), String> {
        let _lock = Self::lock_publications(root)?;
        let mut plan = read_cleanup_plan(root, cleanup_id)?;
        if !plan.published && recover_published_cleanup_successor(root, &mut plan)? {
            plan.published = true;
            write_cleanup_plan(root, &plan)?;
        }
        if !plan.published {
            return Err("CleanupNotPublished: le nouveau traitement n'est pas publié".into());
        }
        match run_cleanup_plan_locked(root, &mut plan) {
            Ok(()) => Ok(()),
            Err(error) => {
                plan.last_error = Some(error.clone());
                write_cleanup_plan(root, &plan)?;
                Err(error)
            }
        }
    }

    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            current: None,
            export_hard_linker: Arc::new(|source, target| fs::hard_link(source, target)),
            unified_resume_checkpoint: None,
        }
    }

    pub fn take_unified_resume_checkpoint(&mut self) -> Option<UnifiedArchiveResumeCheckpoint> {
        self.unified_resume_checkpoint.take()
    }
    pub fn root_path(&self) -> PathBuf {
        self.root.clone()
    }

    pub fn next_segment_sequence(&self) -> u64 {
        self.current
            .as_ref()
            .map_or(1, |active| active.expected_sequence)
    }
}

fn generation_path(root: &Path, job_id: JobId, generation: Generation) -> PathBuf {
    root.join("transcriptions")
        .join(format!("{:032x}", job_id.0))
        .join(generation.get().to_string())
}

#[cfg(windows)]
fn capture_cleanup_target(
    root: &Path,
    candidate: &PublishedImport,
) -> Result<CleanupTarget, String> {
    use crate::windows_file_identity::inspect;
    let treatment = TreatmentIdentity {
        destination: root.to_string_lossy().into_owned(),
        job_id: candidate.job_id,
        generation: candidate.generation,
    };
    let directory = generation_path(root, candidate.job_id, candidate.generation);
    let canonical = directory
        .canonicalize()
        .map_err(|error| format!("CleanupUnverified: traitement absent: {error}"))?;
    if canonical != directory {
        return Err("CleanupUnverified: chemin de traitement redirigé".into());
    }
    let manifest: CompleteRecord = serde_json::from_slice(
        &fs::read(directory.join("manifest.json"))
            .map_err(|error| format!("CleanupUnverified: manifeste illisible: {error}"))?,
    )
    .map_err(|error| format!("CleanupUnverified: manifeste illisible: {error}"))?;
    if manifest.job_id != candidate.job_id
        || manifest.generation != candidate.generation
        || manifest.source_sha256 != candidate.source_sha256
    {
        return Err("CleanupUnverified: manifeste différent du catalogue".into());
    }
    verify_artifact(&directory, &manifest.text)?;
    verify_artifact(&directory, &manifest.srt)?;
    let mut ancestors = Vec::new();
    for ancestor in [
        root.join("transcriptions"),
        directory.parent().unwrap().to_path_buf(),
    ] {
        capture_cleanup_ancestor(root, &ancestor, &mut ancestors)?;
    }

    const KNOWN_GENERATION_FILES: &[&str] = &[
        "CURRENT",
        "manifest.json",
        "pending.json",
        "segments.jsonl",
        "transcript.txt",
        "transcript.srt",
        "r11-publication.json",
        "installed-exports.json",
        "vad-windows.jsonl",
    ];
    let mut objects = Vec::new();
    let generation_identity = identity_snapshot(inspect(&directory, true)?)?;
    for entry in fs::read_dir(&directory)
        .map_err(|error| format!("CleanupUnverified: lister le traitement: {error}"))?
    {
        let entry =
            entry.map_err(|error| format!("CleanupUnverified: entrée traitement: {error}"))?;
        let file_type = entry
            .file_type()
            .map_err(|error| format!("CleanupUnverified: type d'entrée: {error}"))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if file_type.is_symlink() {
            return Err(format!(
                "CleanupUnverified: lien symbolique conservé sans suivi: {}",
                entry.path().display()
            ));
        }
        let known = file_type.is_file() && KNOWN_GENERATION_FILES.contains(&name.as_str());
        let identity = identity_snapshot(inspect(&entry.path(), file_type.is_dir())?)?;
        objects.push(CleanupObject {
            relative_path: relative_to_root(root, &entry.path())?,
            identity,
            sha256: file_type
                .is_file()
                .then(|| sha256_file(&entry.path()).map(|sha| hex(&sha)))
                .transpose()?,
            owned: known,
            removed: false,
            error: (!known).then(|| "élément conservé : propriété non vérifiée".into()),
        });
    }
    objects.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    objects.push(CleanupObject {
        relative_path: relative_to_root(root, &directory)?,
        identity: generation_identity,
        sha256: None,
        owned: true,
        removed: false,
        error: None,
    });

    let ownership_path = installed_export_ownership_path(&directory);
    let ownership = match fs::read(&ownership_path) {
        Ok(bytes) => Some(
            serde_json::from_slice::<InstalledExportOwnership>(&bytes)
                .map_err(|error| format!("CleanupUnverified: preuve export invalide: {error}"))?,
        ),
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => {
            return Err(format!("CleanupUnverified: lire la preuve export: {error}"));
        }
    };
    if let Some(export_basename) = candidate.export_basename.as_deref() {
        let suffix = format!("{:032x}-{}", candidate.job_id.0, candidate.generation.get());
        for extension in ["txt", "srt"] {
            let candidates = [
                root.join("exports")
                    .join(format!("{export_basename}.{extension}")),
                root.join("transcriptions-remplacees")
                    .join(format!("{export_basename}-{suffix}.{extension}")),
            ];
            for path in candidates {
                let metadata = match fs::symlink_metadata(&path) {
                    Ok(metadata) => metadata,
                    Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                    Err(error) => {
                        return Err(format!(
                            "CleanupUnverified: inspecter export {}: {error}",
                            path.display()
                        ));
                    }
                };
                if metadata.file_type().is_symlink() || !metadata.is_file() {
                    return Err(format!(
                        "CleanupUnverified: export non régulier conservé: {}",
                        path.display()
                    ));
                }
                let parent = path
                    .parent()
                    .ok_or_else(|| "CleanupUnverified: export sans dossier parent".to_owned())?;
                capture_cleanup_ancestor(root, parent, &mut ancestors)?;
                let sha = hex(&sha256_file(&path)?);
                let identity = identity_snapshot(inspect(&path, false)?)?;
                let proof = ownership
                    .as_ref()
                    .filter(|record| {
                        matches!(record.version, 2 | 3)
                            && record.job_id == candidate.job_id
                            && record.generation == candidate.generation
                            && record.source_sha256 == candidate.source_sha256
                            && record.export_basename == export_basename
                    })
                    .and_then(|record| match extension {
                        "txt" => Some((
                            record.text_sha256.as_str(),
                            record.text_file_identity.as_deref(),
                        )),
                        "srt" => Some((
                            record.srt_sha256.as_str(),
                            record.srt_file_identity.as_deref(),
                        )),
                        _ => None,
                    });
                let owned = proof.is_some_and(|(expected_sha, expected_identity)| {
                    sha == expected_sha
                        && persisted_file_identity(&path).ok().as_deref() == expected_identity
                });
                objects.push(CleanupObject {
                    relative_path: relative_to_root(root, &path)?,
                    identity,
                    sha256: Some(sha),
                    owned,
                    removed: false,
                    error: (!owned).then(|| "export conservé : propriété non vérifiée".into()),
                });
            }
        }
    }
    Ok(CleanupTarget {
        treatment,
        source_sha256: candidate.source_sha256.clone(),
        ancestors,
        objects,
        container: None,
    })
}

#[cfg(windows)]
fn attach_empty_container_proofs(
    root: &Path,
    catalog: &[PublishedImport],
    selected: &[TreatmentIdentity],
    targets: &mut [CleanupTarget],
) -> Result<(), String> {
    use crate::windows_file_identity::inspect;

    let mut jobs = selected.iter().map(|item| item.job_id).collect::<Vec<_>>();
    jobs.sort_unstable_by_key(|job_id| job_id.0);
    jobs.dedup();
    for job_id in jobs {
        let selected_generations = selected
            .iter()
            .filter(|item| item.job_id == job_id)
            .map(|item| item.generation.get())
            .collect::<std::collections::HashSet<_>>();
        let container_path = root
            .join("transcriptions")
            .join(format!("{:032x}", job_id.0));
        let entries = fs::read_dir(&container_path)
            .map_err(|error| format!("CleanupUnverified: lister le conteneur: {error}"))?;
        let mut found_generations = std::collections::HashSet::new();
        let mut eligible = true;
        for entry in entries {
            let entry =
                entry.map_err(|error| format!("CleanupUnverified: entrée conteneur: {error}"))?;
            let file_type = entry
                .file_type()
                .map_err(|error| format!("CleanupUnverified: type conteneur: {error}"))?;
            let Some(generation) = entry
                .file_name()
                .to_str()
                .and_then(|name| name.parse::<u64>().ok())
                .filter(|generation| *generation > 0)
            else {
                eligible = false;
                break;
            };
            if !file_type.is_dir()
                || file_type.is_symlink()
                || !selected_generations.contains(&generation)
                || !catalog.iter().any(|candidate| {
                    candidate.job_id == job_id
                        && candidate.generation.get() == generation
                        && candidate.verified_present
                })
            {
                eligible = false;
                break;
            }
            found_generations.insert(generation);
        }
        if !eligible || found_generations != selected_generations {
            continue;
        }
        let identity = identity_snapshot(inspect(&container_path, true)?)?;
        let relative_path = relative_to_root(root, &container_path)?;
        let Some(target_index) = targets
            .iter()
            .rposition(|target| target.treatment.job_id == job_id)
        else {
            continue;
        };
        let target = &mut targets[target_index];
        if !target.ancestors.iter().any(|ancestor| {
            ancestor.relative_path == relative_path && ancestor.identity == identity
        }) {
            return Err("CleanupUnverified: identité du conteneur absente des ancêtres".into());
        }
        target.container = Some(CleanupObject {
            relative_path,
            identity,
            sha256: None,
            owned: true,
            removed: false,
            error: None,
        });
    }
    Ok(())
}

#[cfg(not(windows))]
fn attach_empty_container_proofs(
    _root: &Path,
    _catalog: &[PublishedImport],
    _selected: &[TreatmentIdentity],
    _targets: &mut [CleanupTarget],
) -> Result<(), String> {
    Ok(())
}

#[cfg(windows)]
fn capture_cleanup_ancestor(
    root: &Path,
    path: &Path,
    ancestors: &mut Vec<CleanupAncestor>,
) -> Result<(), String> {
    use crate::windows_file_identity::inspect;
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("CleanupUnverified: dossier parent: {error}"))?;
    let expected = root
        .canonicalize()
        .map_err(|error| format!("CleanupUnverified: destination: {error}"))?
        .join(relative_to_root(root, path)?);
    if canonical != expected {
        return Err(format!(
            "CleanupUnverified: parent redirigé: {}",
            path.display()
        ));
    }
    let relative_path = relative_to_root(root, path)?;
    if let Some(existing) = ancestors
        .iter()
        .find(|ancestor| ancestor.relative_path == relative_path)
    {
        return Ok(());
    }
    ancestors.push(CleanupAncestor {
        relative_path,
        identity: identity_snapshot(inspect(path, true)?)?,
    });
    Ok(())
}

#[cfg(windows)]
fn capture_cleanup_file(root: &Path, path: &Path) -> Result<CleanupObject, String> {
    use crate::windows_file_identity::inspect;
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("CleanupUnverified: artefact successeur absent: {error}"))?;
    let expected = root
        .canonicalize()
        .map_err(|error| format!("CleanupUnverified: destination: {error}"))?
        .join(relative_to_root(root, path)?);
    if canonical != expected {
        return Err(format!(
            "CleanupUnverified: artefact successeur redirigé: {}",
            path.display()
        ));
    }
    Ok(CleanupObject {
        relative_path: relative_to_root(root, path)?,
        identity: identity_snapshot(inspect(path, false)?)?,
        sha256: Some(hex(&sha256_file(path)?)),
        owned: true,
        removed: false,
        error: None,
    })
}

#[cfg(windows)]
fn capture_successor_files(root: &Path, directory: &Path) -> Result<Vec<CleanupObject>, String> {
    let manifest_path = directory.join("manifest.json");
    let manifest: CompleteRecord = serde_json::from_slice(
        &fs::read(&manifest_path)
            .map_err(|error| format!("CleanupUnverified: lire manifeste successeur: {error}"))?,
    )
    .map_err(|error| format!("CleanupUnverified: décoder manifeste successeur: {error}"))?;
    verify_artifact(directory, &manifest.text)?;
    verify_artifact(directory, &manifest.srt)?;
    [
        manifest_path,
        directory.join(&manifest.text.path),
        directory.join(&manifest.srt.path),
        directory.join("pending.json"),
        directory.join("segments.jsonl"),
    ]
    .iter()
    .map(|path| capture_cleanup_file(root, path))
    .collect()
}

#[cfg(windows)]
fn recover_published_cleanup_successor(
    root: &Path,
    plan: &mut CleanupPlan,
) -> Result<bool, String> {
    use crate::windows_file_identity::VerifiedFile;

    let canonical_root = root
        .canonicalize()
        .map_err(|error| format!("CleanupUnsafePath: destination: {error}"))?;
    if canonical_root.to_string_lossy() != plan.destination {
        return Err("CleanupUnsafePath: destination de reprise divergente".into());
    }
    let successor_root = Path::new(&plan.destination);
    let successor_path = generation_path(
        successor_root,
        plan.successor.job_id,
        plan.successor.generation,
    );
    if successor_path.canonicalize().ok().as_deref() != Some(successor_path.as_path()) {
        return Err("CleanupIdentityChanged: le nouveau traitement a été déplacé".into());
    }
    let successor_directory = VerifiedFile::open(&successor_path, true)?;
    successor_directory.verify(&runtime_identity(&plan.successor_directory))?;

    const PREPUBLICATION_FILES: [&str; 5] = [
        "manifest.json",
        "transcript.txt",
        "transcript.srt",
        "pending.json",
        "segments.jsonl",
    ];
    if plan.successor_files.len() != PREPUBLICATION_FILES.len() {
        return Err("CleanupIdentityChanged: preuves prépublication incomplètes".into());
    }
    let mut held = Vec::new();
    let mut manifest = None;
    let mut pending = None;
    let mut segments = None;
    for (proof, expected_name) in plan.successor_files.iter().zip(PREPUBLICATION_FILES) {
        let expected_path = generation_path(root, plan.successor.job_id, plan.successor.generation)
            .join(expected_name);
        if proof.relative_path != relative_to_root(root, &expected_path)? {
            return Err("CleanupIdentityChanged: chemin de preuve successeur inattendu".into());
        }
        let path = safe_cleanup_path(root, &proof.relative_path)?;
        let mut handle = VerifiedFile::open(&path, false)?;
        handle.verify(&runtime_identity(&proof.identity))?;
        let bytes = handle.read_all()?;
        if Some(hex(&Sha256::digest(&bytes))) != proof.sha256 {
            return Err(format!(
                "CleanupIdentityChanged: preuve successeur divergente {expected_name}"
            ));
        }
        match expected_name {
            "manifest.json" => {
                manifest = Some(serde_json::from_slice::<CompleteRecord>(&bytes).map_err(
                    |error| format!("CleanupIdentityChanged: manifeste successeur: {error}"),
                )?);
            }
            "pending.json" => {
                pending = Some(serde_json::from_slice::<PendingRecord>(&bytes).map_err(
                    |error| format!("CleanupIdentityChanged: pending successeur: {error}"),
                )?);
            }
            "segments.jsonl" => segments = Some(bytes),
            _ => {}
        }
        held.push(handle);
    }
    let manifest = manifest.expect("prepublication proof list contains manifest");
    let pending = pending.expect("prepublication proof list contains pending");
    if manifest.job_id != plan.successor.job_id
        || manifest.generation != plan.successor.generation
        || manifest.source_sha256 != plan.source_sha256
        || manifest.text.path != "transcript.txt"
        || manifest.srt.path != "transcript.srt"
        || pending.job_id != plan.successor.job_id
        || pending.generation != plan.successor.generation
        || pending.source_sha256 != plan.source_sha256
    {
        return Err("CleanupIdentityChanged: contrat du successeur divergent".into());
    }
    let segment_bytes = segments.expect("prepublication proof list contains segments");
    if !segment_bytes.is_empty() && !segment_bytes.ends_with(b"\n") {
        return Err("CleanupIdentityChanged: journal segments tronqué".into());
    }
    let parsed_segments = segment_bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| {
            serde_json::from_slice::<SegmentRecord>(line)
                .map_err(|error| format!("CleanupIdentityChanged: journal segments: {error}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if parsed_segments.len() as u64 != manifest.segments
        || parsed_segments
            .iter()
            .enumerate()
            .any(|(index, segment)| segment.sequence != index as u64 + 1)
    {
        return Err("CleanupIdentityChanged: journal segments incohérent".into());
    }

    let intent = load_publication_intent(&successor_path)?;
    if intent.cleanup_id.as_deref() != Some(plan.cleanup_id.as_str()) {
        return Err("CleanupIdentityChanged: identifiant d'intention divergent".into());
    }
    if intent.source_sha256 != plan.source_sha256 {
        return Err("CleanupIdentityChanged: empreinte de source d'intention divergente".into());
    }
    if !cleanup_targets_match_intent(&intent.cleanup_targets, &plan.targets) {
        return Err("CleanupIdentityChanged: cibles de l'intention divergentes".into());
    }

    let current_path =
        generation_path(root, plan.successor.job_id, plan.successor.generation).join("CURRENT");
    let mut current = match VerifiedFile::open(&current_path, false) {
        Ok(current) => current,
        Err(_) => return Ok(false),
    };
    if current.read_all()? != b"manifest.json\n" {
        return Err("CleanupIdentityChanged: CURRENT ne référence pas le manifeste attendu".into());
    }

    // CURRENT is the commit marker. If a crash occurred before the index write,
    // replaying this intent idempotently restores the successor's active row.
    let index = apply_publication_intent(
        load_or_migrate_publication_index(root)?,
        plan.successor.job_id,
        plan.successor.generation,
        &intent,
    );
    write_publication_index(root, &index)?;
    plan.successor_files.push(CleanupObject {
        relative_path: relative_to_root(root, &current_path)?,
        identity: identity_snapshot(current.identity.clone())?,
        sha256: Some(current.sha256()?),
        owned: true,
        removed: false,
        error: None,
    });
    plan.successor_index = Some(capture_cleanup_file(root, &publication_index_path(root))?);
    let _held_through_recovery = (successor_directory, held, current);
    Ok(true)
}

#[cfg(not(windows))]
fn recover_published_cleanup_successor(
    _root: &Path,
    _plan: &mut CleanupPlan,
) -> Result<bool, String> {
    Ok(false)
}

#[cfg(not(windows))]
fn capture_cleanup_target(
    _root: &Path,
    _candidate: &PublishedImport,
) -> Result<CleanupTarget, String> {
    Err("Unsupported: le nettoyage RTC-02 nécessite les handles Windows qualifiés".into())
}

#[cfg(windows)]
fn identity_snapshot(
    identity: crate::windows_file_identity::FileIdentity,
) -> Result<CleanupObjectIdentity, String> {
    Ok(CleanupObjectIdentity {
        volume_serial: identity.volume_serial,
        file_id: identity.file_id,
        is_directory: identity.is_directory,
    })
}

#[cfg(not(windows))]
fn identity_snapshot(_identity: ()) -> Result<CleanupObjectIdentity, String> {
    unreachable!()
}

fn relative_to_root(root: &Path, path: &Path) -> Result<String, String> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| "CleanupUnsafePath: chemin hors destination".to_owned())?;
    if relative
        .components()
        .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err("CleanupUnsafePath: chemin relatif non canonique".into());
    }
    Ok(relative.to_string_lossy().into_owned())
}

fn cleanup_plans_path(root: &Path) -> PathBuf {
    root.join(".whisper-publications")
        .join("replacement-cleanups")
}

fn cleanup_plan_path(root: &Path, cleanup_id: &str) -> PathBuf {
    cleanup_plans_path(root).join(format!("{cleanup_id}.json"))
}

fn read_cleanup_plan(root: &Path, cleanup_id: &str) -> Result<CleanupPlan, String> {
    if cleanup_id.is_empty()
        || !cleanup_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err("InvalidInput: identifiant de nettoyage invalide".into());
    }
    serde_json::from_slice(
        &fs::read(cleanup_plan_path(root, cleanup_id))
            .map_err(|error| format!("StorageUnavailable: lire la reprise: {error}"))?,
    )
    .map_err(|error| format!("StorageCorrupt: reprise de nettoyage: {error}"))
}

fn write_cleanup_plan(root: &Path, plan: &CleanupPlan) -> Result<(), String> {
    if root
        .canonicalize()
        .map_err(|error| format!("CleanupUnsafePath: destination: {error}"))?
        .to_string_lossy()
        != plan.destination
    {
        return Err("CleanupUnsafePath: destination de reprise divergente".into());
    }
    let directory = cleanup_plans_path(root);
    fs::create_dir_all(&directory)
        .map_err(|error| format!("StorageUnavailable: dossier de reprise: {error}"))?;
    write_replace_synced(
        &cleanup_plan_path(root, &plan.cleanup_id),
        &serde_json::to_vec(plan).map_err(|error| error.to_string())?,
    )
}

fn cleanup_remaining(plan: &CleanupPlan) -> Vec<String> {
    plan.targets
        .iter()
        .flat_map(|target| target.objects.iter().chain(target.container.iter()))
        .filter(|object| object.owned && !object.removed)
        .map(|object| {
            object.error.as_ref().map_or_else(
                || object.relative_path.clone(),
                |error| format!("{} ({error})", object.relative_path),
            )
        })
        .collect()
}

fn cleanup_targets_match_intent(intent: &[CleanupTarget], plan: &[CleanupTarget]) -> bool {
    intent.len() == plan.len()
        && intent.iter().zip(plan).all(|(expected, actual)| {
            if expected.treatment != actual.treatment
                || expected.source_sha256 != actual.source_sha256
                || expected.ancestors != actual.ancestors
                || !cleanup_container_matches(
                    expected.container.as_ref(),
                    actual.container.as_ref(),
                )
            {
                return false;
            }

            let mut expected_objects = expected.objects.iter();
            let mut matched_expected = 0;
            let mut actual_paths = std::collections::HashSet::new();
            for actual_object in &actual.objects {
                if !actual_paths.insert(&actual_object.relative_path) {
                    return false;
                }
                if let Some(expected_object) = expected_objects.clone().next()
                    && expected_object.relative_path == actual_object.relative_path
                    && expected_object.identity == actual_object.identity
                    && expected_object.sha256 == actual_object.sha256
                    && expected_object.owned == actual_object.owned
                {
                    expected_objects.next();
                    matched_expected += 1;
                } else if actual_object.owned {
                    // A plan may gain late, unowned entries for reporting, but
                    // it must never gain or replace a deletion target.
                    return false;
                }
            }
            matched_expected == expected.objects.len()
        })
}

fn cleanup_container_matches(
    expected: Option<&CleanupObject>,
    actual: Option<&CleanupObject>,
) -> bool {
    match (expected, actual) {
        (None, None) => true,
        (Some(expected), Some(actual)) => {
            expected.relative_path == actual.relative_path
                && expected.identity == actual.identity
                && expected.sha256 == actual.sha256
                && (expected.owned == actual.owned
                    || (expected.owned && !actual.owned && !actual.removed))
        }
        _ => false,
    }
}

fn cleanup_preserved(root: &Path, plan: &CleanupPlan) -> Vec<String> {
    plan.targets
        .iter()
        .flat_map(|target| target.objects.iter().chain(target.container.iter()))
        .filter(|object| !object.owned && !object.removed)
        .filter(|object| {
            let Ok(path) = safe_cleanup_path(root, &object.relative_path) else {
                return true;
            };
            !matches!(
                fs::symlink_metadata(path),
                Err(ref error) if error.kind() == io::ErrorKind::NotFound
            )
        })
        .map(|object| {
            format!(
                "{} ({})",
                object.relative_path,
                object.error.as_deref().unwrap_or("propriété non vérifiée")
            )
        })
        .collect()
}

#[cfg(windows)]
fn runtime_identity(value: &CleanupObjectIdentity) -> crate::windows_file_identity::FileIdentity {
    crate::windows_file_identity::FileIdentity {
        volume_serial: value.volume_serial,
        file_id: value.file_id,
        is_directory: value.is_directory,
    }
}

#[cfg(windows)]
fn run_cleanup_plan_locked(root: &Path, plan: &mut CleanupPlan) -> Result<(), String> {
    use crate::windows_file_identity::{
        VerifiedFile, delete_if_identity_matches, open_and_hash_source,
    };
    if !plan.published
        || root.canonicalize().ok().as_deref()
            != Some(
                Path::new(&plan.destination)
                    .canonicalize()
                    .map_err(|error| format!("CleanupUnsafePath: destination: {error}"))?
                    .as_path(),
            )
    {
        return Err("CleanupUnsafePath: destination de reprise divergente".into());
    }
    let source = open_and_hash_source(Path::new(&plan.source_path))?;
    if hex(&source.sha256) != plan.source_sha256 {
        return Err("SourceChanged: l'audio original ne correspond plus au snapshot traité".into());
    }
    let _source_handle = source;

    // Build from the canonical destination captured in the durable intent so
    // Windows' `\\?\` canonical path prefix does not cause a false mismatch.
    let successor_root = Path::new(&plan.destination);
    let successor_path = generation_path(
        successor_root,
        plan.successor.job_id,
        plan.successor.generation,
    );
    if successor_path.canonicalize().ok().as_deref() != Some(successor_path.as_path()) {
        return Err("CleanupIdentityChanged: le nouveau traitement a été déplacé".into());
    }
    let successor_handle = VerifiedFile::open(&successor_path, true)?;
    successor_handle.verify(&runtime_identity(&plan.successor_directory))?;
    if plan.successor_files.len() != 6 || plan.successor_index.is_none() {
        return Err("CleanupIdentityChanged: preuve durable du successeur incomplète".into());
    }
    let expected_successor_names = [
        "manifest.json",
        "transcript.txt",
        "transcript.srt",
        "pending.json",
        "segments.jsonl",
        "CURRENT",
    ];
    let mut held_successor_files = Vec::new();
    let mut successor_manifest = None;
    let mut successor_pending = None;
    let mut successor_segments = None;
    for (index, expected_name) in expected_successor_names.iter().enumerate() {
        let proof = &plan.successor_files[index];
        if proof.relative_path
            != relative_to_root(successor_root, &successor_path.join(expected_name))?
        {
            return Err("CleanupIdentityChanged: chemin de preuve successeur inattendu".into());
        }
        let path = safe_cleanup_path(root, &proof.relative_path)?;
        let mut handle = VerifiedFile::open(&path, false)?;
        handle.verify(&runtime_identity(&proof.identity))?;
        if Some(handle.sha256()?).as_deref() != proof.sha256.as_deref() {
            return Err(format!(
                "CleanupIdentityChanged: preuve successeur divergente {expected_name}"
            ));
        }
        if *expected_name == "CURRENT" && handle.read_all()? != b"manifest.json\n" {
            return Err(
                "CleanupIdentityChanged: CURRENT ne référence pas le manifeste attendu".into(),
            );
        }
        if *expected_name == "manifest.json" {
            let manifest: CompleteRecord =
                serde_json::from_slice(&handle.read_all()?).map_err(|error| {
                    format!("CleanupIdentityChanged: manifeste successeur: {error}")
                })?;
            if manifest.job_id != plan.successor.job_id
                || manifest.generation != plan.successor.generation
                || manifest.source_sha256 != plan.source_sha256
                || manifest.text.path != "transcript.txt"
                || manifest.srt.path != "transcript.srt"
            {
                return Err(
                    "CleanupIdentityChanged: manifeste successeur ne correspond pas".into(),
                );
            }
            successor_manifest = Some(manifest);
        }
        if *expected_name == "pending.json" {
            successor_pending = Some(
                serde_json::from_slice::<PendingRecord>(&handle.read_all()?).map_err(|error| {
                    format!("CleanupIdentityChanged: pending successeur: {error}")
                })?,
            );
        }
        if *expected_name == "segments.jsonl" {
            successor_segments = Some(handle.read_all()?);
        }
        held_successor_files.push(handle);
    }
    let manifest = successor_manifest.expect("six successor proofs include manifest");
    let pending = successor_pending.expect("six successor proofs include pending");
    let segment_bytes = successor_segments.expect("six successor proofs include segments");
    if manifest.source_sha256 != plan.source_sha256
        || pending.job_id != plan.successor.job_id
        || pending.generation != plan.successor.generation
        || pending.source_sha256 != plan.source_sha256
    {
        return Err("CleanupIdentityChanged: identité du successeur divergente".into());
    }
    if !segment_bytes.is_empty() && !segment_bytes.ends_with(b"\n") {
        return Err("CleanupIdentityChanged: journal segments tronqué".into());
    }
    let segments = segment_bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| {
            serde_json::from_slice::<SegmentRecord>(line)
                .map_err(|error| format!("CleanupIdentityChanged: journal segments: {error}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if segments.len() as u64 != manifest.segments
        || segments
            .iter()
            .enumerate()
            .any(|(index, segment)| segment.sequence != index as u64 + 1)
    {
        return Err("CleanupIdentityChanged: journal segments incohérent".into());
    }
    let intent = load_publication_intent(&successor_path)?;
    if intent.cleanup_id.as_deref() != Some(plan.cleanup_id.as_str())
        || intent.source_sha256 != plan.source_sha256
        || !cleanup_targets_match_intent(&intent.cleanup_targets, &plan.targets)
    {
        return Err("CleanupIdentityChanged: intention de publication divergente".into());
    }
    let index_proof = plan.successor_index.as_ref().unwrap();
    if index_proof.relative_path != relative_to_root(root, &publication_index_path(root))? {
        return Err("CleanupIdentityChanged: chemin de l'index successeur inattendu".into());
    }
    let index_path = safe_cleanup_path(root, &index_proof.relative_path)?;
    let mut index_handle = VerifiedFile::open(&index_path, false)?;
    index_handle.verify(&runtime_identity(&index_proof.identity))?;
    if Some(index_handle.sha256()?).as_deref() != index_proof.sha256.as_deref() {
        return Err("CleanupIdentityChanged: index de publication modifié".into());
    }
    let index: PublicationIndex = serde_json::from_slice(&index_handle.read_all()?)
        .map_err(|error| format!("CleanupIdentityChanged: index de publication: {error}"))?;
    if index.version != 1
        || !index.records.iter().any(|record| {
            record.job_id == plan.successor.job_id
                && record.generation == plan.successor.generation
                && record.source_sha256 == plan.source_sha256
                && record.active
        })
    {
        return Err("CleanupIdentityChanged: successeur absent de l'index actif".into());
    }
    let successor_identity_set: std::collections::HashSet<_> = std::iter::once((
        plan.successor_directory.volume_serial,
        plan.successor_directory.file_id,
        plan.successor_directory.is_directory,
    ))
    .chain(plan.successor_files.iter().map(|object| {
        (
            object.identity.volume_serial,
            object.identity.file_id,
            object.identity.is_directory,
        )
    }))
    .chain(std::iter::once((
        index_proof.identity.volume_serial,
        index_proof.identity.file_id,
        index_proof.identity.is_directory,
    )))
    .collect();
    for target in &plan.targets {
        for object in target.objects.iter().chain(target.container.iter()) {
            if successor_identity_set.contains(&(
                object.identity.volume_serial,
                object.identity.file_id,
                object.identity.is_directory,
            )) {
                return Err(
                    "CleanupUnsafePath: un objet successeur est aliasé par une cible".into(),
                );
            }
        }
    }
    let _successor_handles = (successor_handle, held_successor_files, index_handle);

    // Preflight every target before the first disposition. The original source,
    // successor and all ancestor handles remain open with no write/delete sharing.
    let mut held_ancestors_by_path: std::collections::HashMap<
        String,
        (CleanupObjectIdentity, VerifiedFile),
    > = std::collections::HashMap::new();
    let mut removed_container_paths = std::collections::HashSet::new();
    for target in &mut plan.targets {
        if let Some(container) = &mut target.container {
            if container.removed {
                removed_container_paths.insert(container.relative_path.clone());
            } else if fs::symlink_metadata(safe_cleanup_path(root, &container.relative_path)?)
                .is_err_and(|error| error.kind() == io::ErrorKind::NotFound)
            {
                container.removed = true;
                container.error = None;
                removed_container_paths.insert(container.relative_path.clone());
            }
        }
    }
    if !removed_container_paths.is_empty() {
        write_cleanup_plan(root, plan)?;
    }
    for target in &plan.targets {
        if target.treatment.destination != plan.destination
            || target.source_sha256 != plan.source_sha256
            || (target.treatment.job_id == plan.successor.job_id
                && target.treatment.generation == plan.successor.generation)
        {
            return Err(
                "CleanupUnsafePath: cible chevauche le successeur ou autre destination".into(),
            );
        }
        if let Some(container) = &target.container
            && !container.removed
        {
            let expected_path = root
                .join("transcriptions")
                .join(format!("{:032x}", target.treatment.job_id.0));
            if !container.identity.is_directory
                || container.sha256.is_some()
                || container.relative_path != relative_to_root(root, &expected_path)?
                || !target.ancestors.iter().any(|ancestor| {
                    ancestor.relative_path == container.relative_path
                        && ancestor.identity == container.identity
                })
            {
                return Err("CleanupIdentityChanged: preuve du conteneur invalide".into());
            }
            let path = safe_cleanup_path(root, &container.relative_path)?;
            let canonical = path.canonicalize().map_err(|error| {
                format!(
                    "CleanupIdentityChanged: conteneur inaccessible {}: {error}",
                    container.relative_path
                )
            })?;
            let canonical_root = Path::new(&plan.destination)
                .canonicalize()
                .map_err(|error| format!("CleanupIdentityChanged: destination: {error}"))?;
            if canonical != canonical_root.join(&container.relative_path) {
                return Err(format!(
                    "CleanupUnsafePath: conteneur redirigé {}",
                    container.relative_path
                ));
            }
        }
        for ancestor in &target.ancestors {
            if removed_container_paths.contains(&ancestor.relative_path) {
                continue;
            }
            if let Some(previous) = held_ancestors_by_path.get(&ancestor.relative_path) {
                if previous.0 != ancestor.identity {
                    return Err(format!(
                        "CleanupIdentityChanged: identités d'ancêtre divergentes {}",
                        ancestor.relative_path
                    ));
                }
                continue;
            }
            let path = safe_cleanup_path(root, &ancestor.relative_path)?;
            let handle = VerifiedFile::open(&path, true)?;
            handle
                .verify(&runtime_identity(&ancestor.identity))
                .map_err(|error| format!("{error}: ancêtre {}", ancestor.relative_path))?;
            held_ancestors_by_path.insert(
                ancestor.relative_path.clone(),
                (ancestor.identity.clone(), handle),
            );
        }
    }
    for target in &mut plan.targets {
        for object in &mut target.objects {
            if object.removed {
                continue;
            }
            let path = safe_cleanup_path(root, &object.relative_path)?;
            if fs::symlink_metadata(&path)
                .is_err_and(|error| error.kind() == io::ErrorKind::NotFound)
            {
                object.removed = true;
                object.error = None;
                continue;
            }
            let path = safe_cleanup_path(root, &object.relative_path)?;
            let actual =
                crate::windows_file_identity::inspect(&path, object.identity.is_directory)?;
            if identity_snapshot(actual)? != object.identity {
                return Err(format!("CleanupIdentityChanged: {}", object.relative_path));
            }
            if let Some(expected_sha) = &object.sha256 {
                if hex(&sha256_file(&path)?) != *expected_sha {
                    return Err(format!(
                        "CleanupIdentityChanged: empreinte modifiée {}",
                        object.relative_path
                    ));
                }
            }
        }
        if let Some(container) = &target.container
            && !container.removed
        {
            let Some((identity, _)) = held_ancestors_by_path.get(&container.relative_path) else {
                return Err(format!(
                    "CleanupIdentityChanged: ancêtre du conteneur absent {}",
                    container.relative_path
                ));
            };
            if identity != &container.identity {
                return Err(format!(
                    "CleanupIdentityChanged: identité du conteneur divergente {}",
                    container.relative_path
                ));
            }
        }
    }
    let mut held_target_directories = std::collections::HashMap::new();
    for target in &mut plan.targets {
        let directory = generation_path(root, target.treatment.job_id, target.treatment.generation);
        let directory_relative = relative_to_root(root, &directory)?;
        let directory_proof = target
            .objects
            .iter()
            .find(|object| {
                object.relative_path == directory_relative && object.identity.is_directory
            })
            .ok_or_else(|| "CleanupIdentityChanged: preuve du dossier cible absente".to_owned())?;
        if directory_proof.removed {
            continue;
        }
        let directory_handle = VerifiedFile::open(&directory, true)?;
        directory_handle
            .verify(&runtime_identity(&directory_proof.identity))
            .map_err(|error| format!("{error}: génération {directory_relative}"))?;
        for entry in fs::read_dir(&directory).map_err(|error| {
            format!("CleanupIdentityChanged: inventorier le dossier cible: {error}")
        })? {
            let entry =
                entry.map_err(|error| format!("CleanupIdentityChanged: entrée cible: {error}"))?;
            let relative = relative_to_root(root, &entry.path())?;
            if target
                .objects
                .iter()
                .any(|object| object.relative_path == relative)
            {
                continue;
            }
            let file_type = entry.file_type().map_err(|error| error.to_string())?;
            if file_type.is_symlink() {
                return Err(format!(
                    "CleanupUnsafePath: nouvel élément reparse conservé: {relative}"
                ));
            }
            let identity = identity_snapshot(crate::windows_file_identity::inspect(
                &entry.path(),
                file_type.is_dir(),
            )?)?;
            let sha = file_type
                .is_file()
                .then(|| sha256_file(&entry.path()).map(|value| hex(&value)))
                .transpose()?;
            target.objects.push(CleanupObject {
                relative_path: relative.clone(),
                identity,
                sha256: sha,
                owned: false,
                removed: false,
                error: Some(format!(
                    "élément ajouté après confirmation, conservé: {relative}"
                )),
            });
        }
        held_target_directories.insert(directory_relative, directory_handle);
    }
    write_cleanup_plan(root, plan)?;

    for target_index in 0..plan.targets.len() {
        let mut indices: Vec<usize> = (0..plan.targets[target_index].objects.len()).collect();
        indices.sort_by_key(|index| {
            let object = &plan.targets[target_index].objects[*index];
            (
                object.identity.is_directory,
                std::cmp::Reverse(
                    object
                        .relative_path
                        .chars()
                        .filter(|character| matches!(character, '\\' | '/'))
                        .count(),
                ),
            )
        });
        for object_index in indices {
            let object = &mut plan.targets[target_index].objects[object_index];
            if object.removed || !object.owned {
                continue;
            }
            let path = safe_cleanup_path(root, &object.relative_path)?;
            let result = (|| {
                if fs::symlink_metadata(&path)
                    .is_err_and(|error| error.kind() == io::ErrorKind::NotFound)
                {
                    return Ok(());
                }
                if object.identity.is_directory {
                    // Only the generation directory is included; parent folders are
                    // deliberately retained, so no broad recursive removal occurs.
                    let handle = held_target_directories
                        .remove(&object.relative_path)
                        .ok_or_else(|| {
                            "CleanupIdentityChanged: handle du dossier cible absent".to_owned()
                        })?;
                    handle.mark_delete(&runtime_identity(&object.identity), None)
                } else {
                    delete_if_identity_matches(
                        &path,
                        &runtime_identity(&object.identity),
                        object.sha256.as_deref(),
                    )
                }
            })();
            match result {
                Ok(()) => {
                    object.removed = true;
                    object.error = None;
                }
                Err(error) => {
                    object.error = Some(error.clone());
                    plan.last_error = Some(error.clone());
                    write_cleanup_plan(root, plan)?;
                    return Err(error);
                }
            }
            write_cleanup_plan(root, plan)?;
        }
    }
    for target_index in 0..plan.targets.len() {
        let Some(container_snapshot) = plan.targets[target_index].container.clone() else {
            continue;
        };
        if container_snapshot.removed || !container_snapshot.owned {
            continue;
        }
        let path = safe_cleanup_path(root, &container_snapshot.relative_path)?;
        if fs::symlink_metadata(&path).is_err_and(|error| error.kind() == io::ErrorKind::NotFound) {
            let container = plan.targets[target_index].container.as_mut().unwrap();
            container.removed = true;
            container.error = None;
            write_cleanup_plan(root, plan)?;
            continue;
        }
        let canonical = path
            .canonicalize()
            .map_err(|error| format!("CleanupIdentityChanged: conteneur inaccessible: {error}"))?;
        let expected = Path::new(&plan.destination)
            .canonicalize()
            .map_err(|error| format!("CleanupIdentityChanged: destination: {error}"))?
            .join(&container_snapshot.relative_path);
        if canonical != expected {
            return Err(format!(
                "CleanupUnsafePath: conteneur redirigé {}",
                container_snapshot.relative_path
            ));
        }
        let mut entries = fs::read_dir(&path)
            .map_err(|error| format!("CleanupIdentityChanged: lire conteneur: {error}"))?;
        if entries
            .next()
            .transpose()
            .map_err(|error| {
                format!("CleanupIdentityChanged: lire le contenu du conteneur: {error}")
            })?
            .is_some()
        {
            let container = plan.targets[target_index].container.as_mut().unwrap();
            container.owned = false;
            container.error = Some("contenu non sélectionné présent : conteneur conservé".into());
            write_cleanup_plan(root, plan)?;
            continue;
        }
        let identity = container_snapshot.identity.clone();
        let relative_path = container_snapshot.relative_path.clone();
        let handle = held_ancestors_by_path
            .remove(&relative_path)
            .ok_or_else(|| {
                format!("CleanupIdentityChanged: handle du conteneur absent {relative_path}")
            })?;
        if let Err(error) = handle.1.mark_delete(&runtime_identity(&identity), None) {
            let error = format!("{error}: conteneur {}", relative_path);
            let container = plan.targets[target_index].container.as_mut().unwrap();
            container.error = Some(error.clone());
            plan.last_error = Some(error.clone());
            write_cleanup_plan(root, plan)?;
            return Err(error);
        }
        let container = plan.targets[target_index].container.as_mut().unwrap();
        container.removed = true;
        container.error = None;
        write_cleanup_plan(root, plan)?;
    }
    let _keep_ancestors = (held_ancestors_by_path, held_target_directories);
    plan.last_error = None;
    write_cleanup_plan(root, plan)
}

#[cfg(not(windows))]
fn run_cleanup_plan_locked(_root: &Path, _plan: &mut CleanupPlan) -> Result<(), String> {
    Err("Unsupported: le nettoyage RTC-02 nécessite les handles Windows".into())
}

fn safe_cleanup_path(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let path = Path::new(relative);
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err("CleanupUnsafePath: chemin relatif invalide dans le plan".into());
    }
    Ok(root.join(path))
}

fn load_or_migrate_publication_index(root: &Path) -> Result<PublicationIndex, String> {
    let path = publication_index_path(root);
    if path.exists() {
        return read_publication_index(root);
    }
    let mut records = Vec::new();
    for entry in ArchiveStore::scan_raw(root)?
        .into_iter()
        .filter(|entry| entry.complete)
    {
        let directory = generation_path(root, entry.job_id, entry.generation);
        if directory.join("r11-publication.json").exists() {
            continue;
        }
        records.push(PublicationRecord {
            job_id: entry.job_id,
            generation: entry.generation,
            source_sha256: entry.source_sha256,
            export_basename: None,
            active: true,
            success_ordinal: None,
        });
    }
    Ok(PublicationIndex {
        version: 1,
        records,
    })
}

fn unix_time_ms() -> Result<u64, String> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .map_err(|error| format!("ClockUnavailable: system clock precedes Unix epoch: {error}"))
}

fn timestamp_checked(sample: u64) -> Result<String, String> {
    let seconds = sample / 16_000;
    let remainder = sample % 16_000;
    let milliseconds = seconds
        .checked_mul(1_000)
        .and_then(|whole| whole.checked_add(remainder * 1_000 / 16_000))
        .ok_or_else(|| "InvalidTimestamp: global sample time exceeds SRT range".to_owned())?;
    Ok(format!(
        "{:02}:{:02}:{:02},{:03}",
        milliseconds / 3_600_000,
        (milliseconds / 60_000) % 60,
        (milliseconds / 1_000) % 60,
        milliseconds % 1_000
    ))
}

fn timestamp(sample: u64) -> String {
    let ms = sample.saturating_mul(1000) / 16_000;
    format!(
        "{:02}:{:02}:{:02},{:03}",
        ms / 3_600_000,
        (ms / 60_000) % 60,
        (ms / 1000) % 60,
        ms % 1000
    )
}

#[cfg(test)]
mod tests;
