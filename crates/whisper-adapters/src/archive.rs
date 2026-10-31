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

pub struct PublicationReconfirmation<'a> {
    pub source_sha256: &'a str,
    pub source_samples: u64,
    pub expected_active: &'a [PublishedImport],
    pub choice: DuplicateImportChoice,
    pub reserved_basename: &'a str,
    pub confirmed_basename: &'a str,
}

mod campaign;
mod cleanup;
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
#[cfg(any(test, windows))]
use cleanup::cleanup_plan_path;
#[cfg(test)]
use cleanup::cleanup_plans_path;
use cleanup::{
    attach_empty_container_proofs, capture_cleanup_target, cleanup_targets_match_intent,
    read_cleanup_plan,
};
#[cfg(windows)]
use cleanup::{
    capture_cleanup_file, capture_successor_files, identity_snapshot, run_cleanup_plan_locked,
    write_cleanup_plan,
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

type ExportHardLinker = dyn Fn(&Path, &Path) -> io::Result<()> + Send + Sync;

pub struct ArchiveStore {
    root: PathBuf,
    current: Option<ActiveArchive>,
    export_hard_linker: Arc<ExportHardLinker>,
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
