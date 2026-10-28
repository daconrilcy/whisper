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
use symphonia::core::audio::Channels;
use symphonia::core::codecs::CODEC_TYPE_MP3;
use symphonia::core::{
    formats::FormatOptions, io::MediaSourceStream, meta::MetadataOptions, probe::Hint,
};
use whisper_core::{
    Generation, JobId, SourceRange,
    ports::{
        DuplicateImportChoice, ImportRequest, LiveRequest, PublishedImport, ReplacementCleanupInfo,
        TreatmentIdentity, WorkerSegment,
    },
};

#[cfg(test)]
const APPROVED_MODEL_SIZE: u64 = 1_624_555_275;
#[cfg(test)]
const APPROVED_MODEL_SHA256: &str =
    "1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69";

fn is_registered_model(hash: &str, size: u64) -> bool {
    whisper_core::model_registry::MODELS.iter().any(|record| {
        record.artifact.sha256.eq_ignore_ascii_case(hash) && record.artifact.size_bytes == size
    })
}
const PCM_STAGING_HEADER: &[u8] = b"WHISPCM1\0\x80\x3e\0\0\x01\0";
static L04_CAMPAIGN_OCCURRENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[cfg(test)]
#[derive(Default)]
struct PublicationCopyFault {
    root: Option<PathBuf>,
    cut_after_extension: Option<String>,
}

#[cfg(test)]
static PUBLICATION_COPY_FAULT: std::sync::OnceLock<std::sync::Mutex<PublicationCopyFault>> =
    std::sync::OnceLock::new();

#[cfg(test)]
pub(crate) fn inject_copy_fallback_publication_cut(root: &Path, extension: &str) {
    let fault = PUBLICATION_COPY_FAULT.get_or_init(Default::default);
    let mut fault = fault.lock().expect("publication fault mutex");
    fault.root = Some(root.to_path_buf());
    fault.cut_after_extension = Some(extension.to_owned());
}

#[cfg(test)]
pub(crate) fn clear_copy_fallback_publication_cut() {
    if let Some(fault) = PUBLICATION_COPY_FAULT.get() {
        *fault.lock().expect("publication fault mutex") = PublicationCopyFault::default();
    }
}

#[cfg(test)]
fn publication_copy_fault_applies(root: &Path) -> bool {
    PUBLICATION_COPY_FAULT
        .get()
        .and_then(|fault| fault.lock().ok())
        .is_some_and(|fault| fault.root.as_deref() == Some(root))
}

#[cfg(test)]
fn publication_copy_fault_after(root: &Path, extension: &str) -> Result<(), String> {
    let Some(fault) = PUBLICATION_COPY_FAULT.get() else {
        return Ok(());
    };
    let mut fault = fault
        .lock()
        .map_err(|_| "test publication fault mutex poisoned".to_owned())?;
    if fault.root.as_deref() == Some(root)
        && fault.cut_after_extension.as_deref() == Some(extension)
    {
        fault.cut_after_extension = None;
        return Err(format!(
            "test interruption after final {extension} create_new"
        ));
    }
    Ok(())
}

fn l04_campaign_after(before: &str) -> Option<&'static str> {
    match before {
        "attempt.before_record" => Some("attempt.after_sync"),
        "coverage.before_record" => Some("coverage.after_sync"),
        "live.before_mp3_sync" => Some("live.after_mp3_sync"),
        "live.before_manifest" => Some("live.after_manifest"),
        "live.before_current" => Some("live.after_current"),
        "import.before_manifest" => Some("import.after_manifest"),
        "import.before_current" => Some("import.after_current"),
        "recovery.before_coverage" => Some("recovery.after_coverage"),
        "live.before_spawn" => Some("live.after_spawn"),
        "live.before_reserve" => Some("live.after_reserve"),
        "pcm.before_drain" => Some("pcm.after_drain"),
        "pcm.before_final_drain" => Some("pcm.after_final_drain"),
        "pcm.before_sync_data" => Some("pcm.after_sync_data"),
        "pcm.before_checkpoint" => Some("pcm.after_checkpoint"),
        _ => None,
    }
}

fn l04_campaign_is_after(hook: &str) -> bool {
    matches!(
        hook,
        "attempt.after_sync"
            | "coverage.after_sync"
            | "live.after_mp3_sync"
            | "live.after_manifest"
            | "live.after_current"
            | "import.after_manifest"
            | "import.after_current"
            | "recovery.after_coverage"
            | "live.after_spawn"
            | "live.after_reserve"
            | "pcm.after_drain"
            | "pcm.after_final_drain"
            | "pcm.after_sync_data"
            | "pcm.after_checkpoint"
    )
}

pub(crate) fn l04_campaign_assert_archive(root: &Path) -> Result<(), String> {
    if option_env!("WHISPER_L04_CAMPAIGN_BUILD").is_none() {
        return Ok(());
    }
    let Some(directory) = std::env::var_os("WHISPER_L04_CAMPAIGN_RUN") else {
        return Ok(());
    };
    let run = PathBuf::from(directory)
        .canonicalize()
        .map_err(|error| format!("CampaignGuard: run path: {error}"))?;
    let allowed = Path::new(r"C:\WhisperLive\L04-campaign\runs")
        .canonicalize()
        .map_err(|error| format!("CampaignGuard: runs root: {error}"))?;
    if run.parent() != Some(allowed.as_path()) {
        return Err("CampaignGuard: run must be a direct child of campaign runs".into());
    }
    let expected = run
        .join("archive")
        .canonicalize()
        .map_err(|error| format!("CampaignGuard: run archive: {error}"))?;
    if expected.parent() != Some(run.as_path()) {
        return Err("CampaignGuard: run archive leaves the run directory".into());
    }
    let actual = root
        .canonicalize()
        .map_err(|error| format!("CampaignGuard: destination archive: {error}"))?;
    if actual != expected {
        return Err("CampaignGuard: destination is not the run archive".into());
    }
    Ok(())
}

fn l04_campaign_write_marker(path: &Path, value: &serde_json::Value) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("CampaignGuard: marker open: {error}"))?;
    serde_json::to_writer(&mut file, value)
        .map_err(|error| format!("CampaignGuard: marker write: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("CampaignGuard: marker sync: {error}"))
}

fn l04_campaign_snapshot(run: &Path) -> Result<Vec<serde_json::Value>, String> {
    let archive = run
        .join("archive")
        .canonicalize()
        .map_err(|error| format!("CampaignGuard: archive path: {error}"))?;
    let mut pending = vec![archive.clone()];
    let mut records = Vec::new();
    while let Some(directory) = pending.pop() {
        for item in fs::read_dir(&directory)
            .map_err(|error| format!("CampaignGuard: archive listing: {error}"))?
        {
            let item = item.map_err(|error| format!("CampaignGuard: archive item: {error}"))?;
            let path = item
                .path()
                .canonicalize()
                .map_err(|error| format!("CampaignGuard: archive item path: {error}"))?;
            if !path.starts_with(&archive) || records.len() + pending.len() >= 1024 {
                return Err("CampaignGuard: archive snapshot escapes bounds".into());
            }
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            // The live archive holds this coordination file locked for the
            // whole passage. It is not a durable payload and cannot be read
            // reliably while the writer is active on Windows.
            if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".writer.lock"))
            {
                continue;
            }
            let mut file = File::open(&path)
                .map_err(|error| format!("CampaignGuard: archive file: {error}"))?;
            let mut hasher = Sha256::new();
            let mut buffer = [0_u8; 64 * 1024];
            loop {
                let count = file
                    .read(&mut buffer)
                    .map_err(|error| format!("CampaignGuard: archive hash: {error}"))?;
                if count == 0 {
                    break;
                }
                hasher.update(&buffer[..count]);
            }
            records.push(serde_json::json!({
                "path": path.strip_prefix(&archive).unwrap().to_string_lossy(),
                "bytes": file.metadata().map_err(|error| format!("CampaignGuard: archive metadata: {error}"))?.len(),
                "sha256": format!("{:x}", hasher.finalize()),
            }));
        }
    }
    records.sort_by(|left, right| left["path"].as_str().cmp(&right["path"].as_str()));
    Ok(records)
}

pub(crate) fn l04_campaign_operation_result<T, E: std::fmt::Display>(
    before: &str,
    job_id: u128,
    generation: u64,
    instance_id: u64,
    result: &Result<T, E>,
    artifact: Option<&Path>,
) -> Result<(), String> {
    if option_env!("WHISPER_L04_CAMPAIGN_BUILD").is_none() {
        return Ok(());
    }
    let Some(directory) = std::env::var_os("WHISPER_L04_CAMPAIGN_RUN") else {
        return Ok(());
    };
    let run = PathBuf::from(directory)
        .canonicalize()
        .map_err(|error| format!("CampaignGuard: result run: {error}"))?;
    let allowed = Path::new(r"C:\WhisperLive\L04-campaign\runs")
        .canonicalize()
        .map_err(|error| format!("CampaignGuard: result root: {error}"))?;
    if run.parent() != Some(allowed.as_path()) {
        return Err("CampaignGuard: result run outside campaign".into());
    }
    let control: serde_json::Value = serde_json::from_slice(
        &fs::read(run.join("run.json"))
            .map_err(|error| format!("CampaignGuard: result control: {error}"))?,
    )
    .map_err(|error| format!("CampaignGuard: result control JSON: {error}"))?;
    let selected = control.get("hook").and_then(|value| value.as_str());
    if selected != Some(before) && selected != l04_campaign_after(before) {
        return Ok(());
    }
    let target = control
        .get("trigger_count")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| "CampaignGuard: missing trigger count".to_owned())?;
    if target == 0 || L04_CAMPAIGN_OCCURRENCE.load(std::sync::atomic::Ordering::Acquire) != target {
        return Ok(());
    }
    if control.get("job_id").and_then(|value| value.as_str()) != Some(job_id.to_string().as_str())
        || control.get("generation").and_then(|value| value.as_u64()) != Some(generation)
        || control.get("instance_id").and_then(|value| value.as_u64()) != Some(instance_id)
    {
        return Err("CampaignGuard: result identity mismatch".into());
    }
    let artifact = artifact.and_then(|path| path.canonicalize().ok()).map(|path| -> Result<serde_json::Value, String> {
        let archive = run.join("archive").canonicalize()
            .map_err(|error| format!("CampaignGuard: archive path: {error}"))?;
        if !path.starts_with(&archive) {
            return Err("CampaignGuard: artifact outside run archive".into());
        }
        let mut file = File::open(&path)
            .map_err(|error| format!("CampaignGuard: artifact open: {error}"))?;
        let mut hasher = Sha256::new();
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let read = file.read(&mut buffer)
                .map_err(|error| format!("CampaignGuard: artifact read: {error}"))?;
            if read == 0 { break; }
            hasher.update(&buffer[..read]);
        }
        Ok(serde_json::json!({
            "path": path.strip_prefix(&archive).unwrap().to_string_lossy(),
            "bytes": file.metadata().map_err(|error| format!("CampaignGuard: artifact metadata: {error}"))?.len(),
            "sha256": format!("{:x}", hasher.finalize()),
        }))
    }).transpose()?;
    l04_campaign_write_marker(
        &run.join("operation-result.json"),
        &serde_json::json!({
            "before_hook": before, "after_hook": l04_campaign_after(before),
            "job_id": job_id.to_string(), "generation": generation,
            "instance_id": instance_id, "pid": std::process::id(),
            "completed_unix_ms": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).map_or(0, |duration| duration.as_millis()),
            "syscall_result": if result.is_ok() { "ok" } else { "error" },
            "error": result.as_ref().err().map(ToString::to_string),
            "artifact": artifact,
            "archive_files": l04_campaign_snapshot(&run)?,
        }),
    )
}

/// A build-local, one-shot barrier for isolated L04 fault campaigns. The
/// release build contains no usable gate unless the campaign build variable
/// was present when rustc compiled this crate.
pub(crate) fn l04_campaign_gate(
    hook: &str,
    job_id: u128,
    generation: u64,
    instance_id: u64,
) -> Result<(), String> {
    if option_env!("WHISPER_L04_CAMPAIGN_BUILD").is_none() {
        return Ok(());
    }
    let Some(directory) = std::env::var_os("WHISPER_L04_CAMPAIGN_RUN") else {
        return Ok(());
    };
    let directory = PathBuf::from(directory);
    let allowed = Path::new(r"C:\WhisperLive\L04-campaign\runs")
        .canonicalize()
        .map_err(|error| format!("CampaignGuard: runs root: {error}"))?;
    let actual = directory
        .canonicalize()
        .map_err(|error| format!("CampaignGuard: run path: {error}"))?;
    if actual.parent() != Some(allowed.as_path()) {
        return Err("CampaignGuard: run must be a direct child of campaign runs".into());
    }
    let run_file = actual
        .join("run.json")
        .canonicalize()
        .map_err(|error| format!("CampaignGuard: run.json path: {error}"))?;
    if run_file.parent() != Some(actual.as_path()) {
        return Err("CampaignGuard: run.json leaves the run directory".into());
    }
    let control: serde_json::Value = serde_json::from_slice(
        &fs::read(run_file).map_err(|error| format!("CampaignGuard: run.json: {error}"))?,
    )
    .map_err(|error| format!("CampaignGuard: invalid run.json: {error}"))?;
    if control.get("schema").and_then(|value| value.as_str()) != Some("whisper-l04-fault/1") {
        return Err("CampaignGuard: unknown schema".into());
    }
    let selected = control
        .get("hook")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "CampaignGuard: missing hook".to_owned())?;
    if !matches!(
        selected,
        "attempt.before_record"
            | "attempt.after_sync"
            | "coverage.before_record"
            | "coverage.after_sync"
            | "live.before_mp3_sync"
            | "live.after_mp3_sync"
            | "live.before_manifest"
            | "live.after_manifest"
            | "live.before_current"
            | "live.after_current"
            | "import.before_manifest"
            | "import.after_manifest"
            | "import.before_current"
            | "import.after_current"
            | "recovery.before_scan_live"
            | "recovery.before_coverage"
            | "recovery.after_coverage"
            | "live.before_spawn"
            | "live.after_spawn"
            | "live.before_reserve"
            | "live.after_reserve"
            | "stop.before_admit"
            | "live.before_window_ticket"
            | "diagnostic.before_startup_purge"
            | "diagnostic.before_purge"
            | "diagnostic.before_rotate"
            | "diagnostic.before_append"
            | "pcm.before_drain"
            | "pcm.after_drain"
            | "pcm.before_final_drain"
            | "pcm.after_final_drain"
            | "pcm.before_sync_data"
            | "pcm.after_sync_data"
            | "pcm.before_checkpoint"
            | "pcm.after_checkpoint"
            | "worker.before_command"
            | "worker.before_encode"
            | "worker.after_window"
            | "worker.before_finish_encode"
            | "worker.before_stopped"
    ) {
        return Err("CampaignGuard: unknown hook".into());
    }
    let paired_start = l04_campaign_after(hook) == Some(selected);
    if selected != hook && !paired_start {
        return Ok(());
    }
    if control.get("job_id").and_then(|value| value.as_str()) != Some(job_id.to_string().as_str())
        || control.get("generation").and_then(|value| value.as_u64()) != Some(generation)
        || control.get("instance_id").and_then(|value| value.as_u64()) != Some(instance_id)
    {
        return Err("CampaignGuard: identity mismatch".into());
    }
    let target = control
        .get("trigger_count")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| "CampaignGuard: missing trigger count".to_owned())?;
    if target == 0 {
        return Err("CampaignGuard: invalid trigger count".into());
    }
    let binary = std::env::current_exe()
        .map_err(|error| format!("CampaignGuard: current executable: {error}"))?;
    let binary_sha256 = format!(
        "{:x}",
        Sha256::digest(
            fs::read(&binary)
                .map_err(|error| format!("CampaignGuard: executable read: {error}"))?
        )
    );
    if control
        .get("binary")
        .and_then(|value| value.get("sha256"))
        .and_then(|value| value.as_str())
        != Some(binary_sha256.as_str())
    {
        return Err("CampaignGuard: binary hash mismatch".into());
    }
    let occurrence = if paired_start || (selected == hook && !l04_campaign_is_after(hook)) {
        L04_CAMPAIGN_OCCURRENCE.fetch_add(1, std::sync::atomic::Ordering::AcqRel) + 1
    } else {
        L04_CAMPAIGN_OCCURRENCE.load(std::sync::atomic::Ordering::Acquire)
    };
    if occurrence < target {
        return Ok(());
    }
    if occurrence > target {
        return Err("CampaignGuard: duplicate trigger count".into());
    }
    if paired_start || (selected == hook && l04_campaign_after(hook).is_some()) {
        l04_campaign_write_marker(
            &actual.join("operation-start.json"),
            &serde_json::json!({
                "before_hook": hook, "after_hook": l04_campaign_after(hook).unwrap_or(selected),
                "job_id": job_id.to_string(), "generation": generation,
                "instance_id": instance_id, "pid": std::process::id(),
                "started_unix_ms": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH).map_or(0, |duration| duration.as_millis()),
                "binary_sha256": binary_sha256,
                "trigger_count": occurrence,
                "archive_files": l04_campaign_snapshot(&actual)?,
            }),
        )?;
        if paired_start {
            return Ok(());
        }
    }
    let entered = actual.join("entered.json");
    let mut marker = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&entered)
        .map_err(|error| format!("CampaignGuard: duplicate trigger: {error}"))?;
    serde_json::to_writer(
        &mut marker,
        &serde_json::json!({
            "hook": hook, "job_id": job_id.to_string(), "generation": generation,
            "instance_id": instance_id, "pid": std::process::id(),
            "trigger_count": occurrence,
            "binary": binary.to_string_lossy(), "binary_sha256": binary_sha256,
            "entered_unix_ms": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).map_or(0, |duration| duration.as_millis()),
            "preceding_operation_result": if hook.contains(".after_") {
                "returned_ok"
            } else {
                "not_attested_at_this_barrier"
            },
        }),
    )
    .map_err(|error| format!("CampaignGuard: marker: {error}"))?;
    marker
        .sync_all()
        .map_err(|error| format!("CampaignGuard: marker sync: {error}"))?;
    drop(marker);
    let decision = actual.join("decision.txt");
    for _ in 0..1200 {
        if decision.exists() {
            let decision_file = decision
                .canonicalize()
                .map_err(|error| format!("CampaignGuard: decision path: {error}"))?;
            if decision_file.parent() != Some(actual.as_path()) {
                return Err("CampaignGuard: decision leaves the run directory".into());
            }
            let action = fs::read_to_string(decision_file)
                .map_err(|error| format!("CampaignGuard: decision read: {error}"))?;
            let action = action.trim();
            if !matches!(action, "release" | "fail" | "crash") {
                return Err("CampaignGuard: unknown decision".into());
            }
            let observed = serde_json::json!({
                "hook": hook, "action": action, "pid": std::process::id(),
                "observed_unix_ms": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH).map_or(0, |duration| duration.as_millis()),
            });
            let mut receipt = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(actual.join("decision-observed.json"))
                .map_err(|error| format!("CampaignGuard: decision receipt: {error}"))?;
            serde_json::to_writer(&mut receipt, &observed)
                .map_err(|error| format!("CampaignGuard: decision receipt write: {error}"))?;
            receipt
                .sync_all()
                .map_err(|error| format!("CampaignGuard: decision receipt sync: {error}"))?;
            return match action {
                "release" => Ok(()),
                "fail" => Err(format!("CampaignInjected: {hook}")),
                "crash" => std::process::abort(),
                _ => unreachable!("validated above"),
            };
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    Err("CampaignGuard: decision timeout".into())
}

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

impl ArchiveQualificationKey {
    fn permits_bounded_resume_to(&self, current: &Self) -> bool {
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

    fn is_retired_l2a_key(&self) -> bool {
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

    fn is_retired_r5_2_key(&self) -> bool {
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

impl LiveArchive {
    pub fn prepare(root: impl AsRef<Path>, mut request: LiveRequest) -> Result<Self, String> {
        l04_campaign_assert_archive(root.as_ref())?;
        let directory = root
            .as_ref()
            .join("transcriptions")
            .join(format!("{:032x}", request.job_id.0))
            .join("live");
        fs::create_dir_all(&directory).map_err(|e| format!("StorageUnavailable: {e}"))?;
        let lock_path = directory.join(format!("group-{:032x}.writer.lock", request.job_id.0));
        let writer_lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(lock_path)
            .map_err(|error| format!("StorageUnavailable: archive lock open: {error}"))?;
        writer_lock.try_lock().map_err(|error| {
            format!("Recoverable: another live archive writer owns the passage: {error}")
        })?;
        let mut previous_completed_at_ms = None;
        if request.generation.get() > 1 {
            let previous =
                directory.join(format!("manifest-{}.json", request.generation.get() - 1));
            if previous.is_file() {
                let record: LiveCompleteRecord =
                    serde_json::from_slice(&fs::read(previous).map_err(|e| e.to_string())?)
                        .map_err(|e| {
                            format!("StorageCorrupt: previous live passage manifest: {e}")
                        })?;
                if record.version != 1
                    || record.job_id != request.job_id
                    || record.generation.get() + 1 != request.generation.get()
                    || !is_sha256(&record.pcm_sha256)
                {
                    return Err("StorageCorrupt: previous live passage identity mismatch".into());
                }
                let current = fs::read_to_string(directory.join("CURRENT")).map_err(|e| {
                    format!("StorageCorrupt: previous live passage is not committed: {e}")
                })?;
                if current != format!("manifest-{}.json\n", request.generation.get() - 1) {
                    return Err(
                        "Recoverable: previous live passage is not the committed group tail".into(),
                    );
                }
                verify_artifact(&directory, &record.staging)?;
                verify_pcm_staging(
                    &directory,
                    &record.staging,
                    record.audio_samples,
                    &record.pcm_sha256,
                )?;
                verify_artifact(&directory, &record.mp3)?;
                verify_artifact(&directory, &record.text)?;
                verify_artifact(&directory, &record.srt)?;
                request.group_offset_samples = record
                    .group_offset_samples
                    .checked_add(record.audio_samples)
                    .ok_or_else(|| "StorageCorrupt: live group sample axis overflow".to_owned())?;
                previous_completed_at_ms = Some(record.completed_at_unix_ms);
            } else {
                let pending_path = directory.join(format!(
                    "passage-{}.pending.json",
                    request.generation.get() - 1
                ));
                if pending_path.is_file() {
                    let previous_request: LiveRequest = serde_json::from_slice(
                        &fs::read(&pending_path).map_err(|e| e.to_string())?,
                    )
                    .map_err(|e| format!("StorageCorrupt: previous live pending record: {e}"))?;
                    if previous_request.job_id != request.job_id
                        || previous_request.generation.get() + 1 != request.generation.get()
                        || previous_request.destination != request.destination
                    {
                        return Err(
                            "StorageCorrupt: previous recoverable passage identity mismatch".into(),
                        );
                    }
                    // A pending passage is not a committed group tail, even when
                    // its current PCM and coverage happen to agree. Keep its exact
                    // identity recoverable and require explicit reconciliation
                    // before any later generation can be admitted.
                    return Err(format!(
                        "Recoverable: passage {} remains unresolved; reconcile this passage before starting the next one",
                        previous_request.generation.get()
                    ));
                }
            }
        }
        let pending_path =
            directory.join(format!("passage-{}.pending.json", request.generation.get()));
        let mut pending = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&pending_path)
            .map_err(|e| format!("StorageUnavailable: pending passage: {e}"))?;
        serde_json::to_writer(&mut pending, &request).map_err(|e| e.to_string())?;
        pending
            .sync_all()
            .map_err(|e| format!("StorageUnavailable: pending sync: {e}"))?;
        let mp3_pending_path =
            directory.join(format!("passage-{}.mp3.pending", request.generation.get()));
        let mp3 = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&mp3_pending_path)
            .map_err(|e| format!("StorageUnavailable: passage MP3: {e}"))?;
        Ok(Self {
            directory,
            request,
            _writer_lock: writer_lock,
            pending_path,
            mp3_pending_path,
            mp3: Some(mp3),
            expected_sequence: 1,
            expected_window_sequence: 1,
            confirmed_samples: 0,
            confirmed_segment_count: 0,
            next_attempt: 1,
            next_attempt_record_sequence: 1,
            active_attempt: None,
            segments: Vec::new(),
            previous_completed_at_ms,
        })
    }

    /// Finalizes the group's pause offset immediately before opening the live capture path.
    pub fn capture_started(&mut self) -> Result<(), String> {
        if let Some(completed_at) = self.previous_completed_at_ms.take() {
            let pause_samples = unix_time_ms()?
                .saturating_sub(completed_at)
                .saturating_mul(16);
            self.request.group_offset_samples = self
                .request
                .group_offset_samples
                .checked_add(pause_samples)
                .ok_or_else(|| "StorageCorrupt: live group pause offset overflow".to_owned())?;
            let bytes = serde_json::to_vec(&self.request).map_err(|e| e.to_string())?;
            write_replace_synced(&self.pending_path, &bytes)?;
        }
        Ok(())
    }

    pub fn append_mp3_packet(&mut self, packet: &[u8]) -> Result<(), String> {
        self.mp3
            .as_mut()
            .ok_or_else(|| "StorageUnavailable: MP3 passage is already finalized".to_owned())?
            .write_all(packet)
            .map_err(|e| format!("StorageUnavailable: MP3 write: {e}"))
    }

    pub fn reserve_attempt(&mut self) -> Result<u64, String> {
        if self.active_attempt.is_some() {
            return Err("StorageCorrupt: prior attempt has not been retired".into());
        }
        let attempt = self.next_attempt;
        self.append_attempt_record(attempt, LiveAttemptState::Reserved)?;
        self.active_attempt = Some((attempt, LiveAttemptState::Reserved));
        self.next_attempt = attempt
            .checked_add(1)
            .ok_or_else(|| "StorageCorrupt: live attempt sequence exhausted".to_owned())?;
        Ok(attempt)
    }

    pub fn mark_attempt_started(&mut self, attempt: u64) -> Result<(), String> {
        if self.active_attempt != Some((attempt, LiveAttemptState::Reserved)) {
            return Err("StorageCorrupt: Started requires the matching Reserved attempt".into());
        }
        self.append_attempt_record(attempt, LiveAttemptState::Started)?;
        self.active_attempt = Some((attempt, LiveAttemptState::Started));
        Ok(())
    }

    pub fn retire_attempt(&mut self, attempt: u64) -> Result<(), String> {
        if self.active_attempt != Some((attempt, LiveAttemptState::Started)) {
            return Err("StorageCorrupt: Retired requires the matching Started attempt".into());
        }
        self.append_attempt_record(attempt, LiveAttemptState::Retired)?;
        self.active_attempt = None;
        Ok(())
    }

    pub fn restart_mp3_attempt(&mut self) -> Result<(), String> {
        let file = self
            .mp3
            .as_mut()
            .ok_or_else(|| "StorageUnavailable: MP3 passage is already finalized".to_owned())?;
        file.set_len(0)
            .and_then(|()| file.seek(SeekFrom::Start(0)))
            .and_then(|_| file.sync_all())
            .map_err(|error| format!("StorageUnavailable: reset MP3 attempt: {error}"))?;
        Ok(())
    }

    pub fn discard_unconfirmed_segments(&mut self) -> Result<(), String> {
        self.segments.truncate(self.confirmed_segment_count);
        let path = self.directory.join(format!(
            "passage-{}.segments.jsonl",
            self.request.generation.get()
        ));
        let mut bytes = Vec::new();
        for segment in &self.segments {
            serde_json::to_writer(&mut bytes, segment).map_err(|error| error.to_string())?;
            bytes.extend_from_slice(b"\n");
        }
        write_replace_synced(&path, &bytes)?;
        self.expected_sequence = u64::try_from(self.confirmed_segment_count)
            .ok()
            .and_then(|sequence| sequence.checked_add(1))
            .ok_or_else(|| "StorageCorrupt: live segment sequence exhausted".to_owned())?;
        Ok(())
    }

    fn append_attempt_record(
        &mut self,
        attempt: u64,
        state: LiveAttemptState,
    ) -> Result<(), String> {
        let payload = LiveAttemptPayload {
            version: 1,
            journal_sequence: self.next_attempt_record_sequence,
            job_id: self.request.job_id,
            generation: self.request.generation,
            attempt,
            state,
        };
        let canonical = serde_json::to_vec(&payload).map_err(|error| error.to_string())?;
        let record = LiveAttemptRecord {
            payload,
            sha256: format!("{:x}", Sha256::digest(canonical)),
        };
        let bytes = serde_json::to_vec(&record).map_err(|error| error.to_string())?;
        if bytes.len() + 1 > 1_048_576 {
            return Err("StorageCorrupt: attempt journal record exceeds 1 MiB".into());
        }
        let path = self.directory.join(format!(
            "passage-{}.attempts.jsonl",
            self.request.generation.get()
        ));
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|error| format!("StorageUnavailable: attempt journal: {error}"))?;
        l04_campaign_gate(
            "attempt.before_record",
            self.request.job_id.0,
            self.request.generation.get(),
            0,
        )?;
        let operation = file
            .write_all(&bytes)
            .and_then(|()| file.write_all(b"\n"))
            .and_then(|()| file.sync_all())
            .map_err(|error| format!("StorageUnavailable: attempt journal sync: {error}"));
        l04_campaign_operation_result(
            "attempt.before_record",
            self.request.job_id.0,
            self.request.generation.get(),
            0,
            &operation,
            Some(&path),
        )?;
        operation?;
        l04_campaign_gate(
            "attempt.after_sync",
            self.request.job_id.0,
            self.request.generation.get(),
            0,
        )?;
        self.next_attempt_record_sequence = self
            .next_attempt_record_sequence
            .checked_add(1)
            .ok_or_else(|| "StorageCorrupt: attempt journal sequence exhausted".to_owned())?;
        Ok(())
    }

    pub fn persist_segment(
        &mut self,
        segment: &WorkerSegment,
        audio_durable_samples: u64,
    ) -> Result<u64, String> {
        if segment.job_id != self.request.job_id
            || segment.generation != self.request.generation
            || segment.segment_id.0 != self.expected_sequence
            || segment.range.sample_rate_hz != 16_000
            || segment.range.start_sample >= segment.range.end_sample
            || segment.range.end_sample > audio_durable_samples
            || segment.text.trim().is_empty()
        {
            return Err(
                "StorageCorrupt: live fragment is not covered by a durable PCM prefix".into(),
            );
        }
        let range = SourceRange::new(
            self.request
                .group_offset_samples
                .saturating_add(segment.range.start_sample),
            self.request
                .group_offset_samples
                .saturating_add(segment.range.end_sample),
            16_000,
        )
        .ok_or_else(|| "StorageCorrupt: live group offset overflow".to_owned())?;
        let record = SegmentRecord {
            sequence: segment.segment_id.0,
            range,
            text: segment.text.trim().to_owned(),
        };
        let path = self.directory.join(format!(
            "passage-{}.segments.jsonl",
            self.request.generation.get()
        ));
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| format!("StorageUnavailable: {e}"))?;
        serde_json::to_writer(&mut file, &record).map_err(|e| e.to_string())?;
        file.write_all(b"\n").map_err(|e| e.to_string())?;
        file.sync_all()
            .map_err(|e| format!("StorageUnavailable: fragment sync: {e}"))?;
        self.expected_sequence = self
            .expected_sequence
            .checked_add(1)
            .ok_or_else(|| "segment sequence exhausted".to_owned())?;
        self.segments.push(record);
        Ok(self.segments.last().map_or(0, |item| item.range.end_sample))
    }

    pub fn persist_window_finished(
        &mut self,
        sequence: u64,
        range: SourceRange,
        last_segment_sequence: u64,
        audio_durable_samples: u64,
    ) -> Result<u64, String> {
        if sequence != self.expected_window_sequence
            || range.sample_rate_hz != 16_000
            || range.start_sample != self.confirmed_samples
            || range.end_sample <= range.start_sample
            || range.end_sample > audio_durable_samples
            || last_segment_sequence.checked_add(1) != Some(self.expected_sequence)
        {
            return Err("StorageCorrupt: live window confirmation is not contiguous".into());
        }
        let group_start = self
            .request
            .group_offset_samples
            .checked_add(range.start_sample)
            .ok_or_else(|| "StorageCorrupt: live window group offset overflow".to_owned())?;
        let group_end = self
            .request
            .group_offset_samples
            .checked_add(range.end_sample)
            .ok_or_else(|| "StorageCorrupt: live window group offset overflow".to_owned())?;
        let new_segments = self
            .segments
            .get(self.confirmed_segment_count..)
            .ok_or_else(|| "StorageCorrupt: confirmed segment cursor is invalid".to_owned())?;
        if new_segments.iter().any(|segment| {
            segment.sequence > last_segment_sequence
                || segment.range.start_sample < group_start
                || segment.range.end_sample > group_end
        }) || new_segments
            .last()
            .map_or(self.confirmed_segment_count as u64, |segment| {
                segment.sequence
            })
            != last_segment_sequence
        {
            return Err("StorageCorrupt: window confirmation does not cover its segments".into());
        }
        let payload = CoverageRecordPayload {
            version: 2,
            job_id: self.request.job_id,
            generation: self.request.generation,
            window_sequence: sequence,
            start_sample: range.start_sample,
            end_sample: range.end_sample,
            last_segment_sequence,
        };
        let canonical = serde_json::to_vec(&payload).map_err(|error| error.to_string())?;
        let checksum = format!("{:x}", Sha256::digest(canonical));
        let bytes = serde_json::to_vec(&CoverageRecord {
            payload,
            sha256: checksum,
        })
        .map_err(|error| error.to_string())?;
        if bytes.len() + 1 > 1_048_576 {
            return Err("StorageCorrupt: coverage confirmation exceeds record limit".into());
        }
        let path = self.coverage_path();
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|error| format!("StorageUnavailable: coverage confirmation: {error}"))?;
        l04_campaign_gate(
            "coverage.before_record",
            self.request.job_id.0,
            self.request.generation.get(),
            0,
        )?;
        let operation = file
            .write_all(&bytes)
            .and_then(|()| file.write_all(b"\n"))
            .and_then(|()| file.sync_all())
            .map_err(|error| format!("StorageUnavailable: coverage sync: {error}"));
        l04_campaign_operation_result(
            "coverage.before_record",
            self.request.job_id.0,
            self.request.generation.get(),
            0,
            &operation,
            Some(&path),
        )?;
        operation?;
        l04_campaign_gate(
            "coverage.after_sync",
            self.request.job_id.0,
            self.request.generation.get(),
            0,
        )?;
        self.expected_window_sequence = self
            .expected_window_sequence
            .checked_add(1)
            .ok_or_else(|| "StorageCorrupt: window sequence exhausted".to_owned())?;
        self.confirmed_samples = range.end_sample;
        self.confirmed_segment_count = self.segments.len();
        Ok(self.confirmed_samples)
    }

    pub fn confirmed_samples(&self) -> u64 {
        self.confirmed_samples
    }

    pub fn next_segment_sequence(&self) -> u64 {
        self.expected_sequence
    }

    pub fn next_window_sequence(&self) -> u64 {
        self.expected_window_sequence
    }

    fn coverage_path(&self) -> PathBuf {
        self.directory.join(format!(
            "passage-{}.coverage.jsonl",
            self.request.generation.get()
        ))
    }

    pub fn publish(
        &mut self,
        audio_durable_samples: u64,
        confirmed_fragment_end: u64,
        pcm_sha256: [u8; 32],
    ) -> Result<(), String> {
        if audio_durable_samples == 0 {
            return Err(
                "Recoverable: no audio samples were durably captured; passage was not published"
                    .into(),
            );
        }
        if self.confirmed_samples != audio_durable_samples {
            return Err(
                "Recoverable: durable PCM is not fully covered by completed windows".into(),
            );
        }
        if self
            .segments
            .iter()
            .any(|segment| segment.range.end_sample > confirmed_fragment_end)
        {
            return Err("StorageCorrupt: fragment confirmation does not cover the journal".into());
        }
        l04_campaign_gate(
            "live.before_mp3_sync",
            self.request.job_id.0,
            self.request.generation.get(),
            0,
        )?;
        let operation = self
            .mp3
            .as_mut()
            .ok_or_else(|| "StorageUnavailable: MP3 passage is already finalized".to_owned())?
            .sync_all()
            .map_err(|e| format!("StorageUnavailable: MP3 sync: {e}"));
        l04_campaign_operation_result(
            "live.before_mp3_sync",
            self.request.job_id.0,
            self.request.generation.get(),
            0,
            &operation,
            Some(&self.mp3_pending_path),
        )?;
        operation?;
        l04_campaign_gate(
            "live.after_mp3_sync",
            self.request.job_id.0,
            self.request.generation.get(),
            0,
        )?;
        drop(self.mp3.take());
        let mp3_path = self
            .directory
            .join(format!("passage-{}.mp3", self.request.generation.get()));
        fs::rename(&self.mp3_pending_path, &mp3_path)
            .map_err(|e| format!("StorageUnavailable: MP3 publish: {e}"))?;
        let mut segments = Vec::new();
        for generation in 1..=self.request.generation.get() {
            let path = self
                .directory
                .join(format!("passage-{generation}.segments.jsonl"));
            if path.exists() {
                segments.extend(read_segments(&path)?);
            }
        }
        let text = segments
            .iter()
            .map(|segment| segment.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let srt = segments
            .iter()
            .enumerate()
            .map(|(index, segment)| {
                format!(
                    "{}\n{} --> {}\n{}\n",
                    index + 1,
                    timestamp(segment.range.start_sample),
                    timestamp(segment.range.end_sample),
                    segment.text
                )
            })
            .collect::<String>();
        let passage = self.request.generation.get();
        let text_path = self.directory.join(format!("transcript-{passage}.txt"));
        let srt_path = self.directory.join(format!("transcript-{passage}.srt"));
        write_replace_synced(&text_path, text.as_bytes())?;
        write_replace_synced(&srt_path, srt.as_bytes())?;
        // Manifests reference immutable passage snapshots so later passages cannot
        // make an earlier generation fail hash verification.
        let record = LiveCompleteRecord {
            version: 1,
            job_id: self.request.job_id,
            generation: self.request.generation,
            audio_samples: audio_durable_samples,
            confirmed_samples: self.confirmed_samples,
            group_offset_samples: self.request.group_offset_samples,
            completed_at_unix_ms: unix_time_ms()?,
            pcm_sha256: hex(&pcm_sha256),
            staging: verified_artifact(
                &self.directory,
                &self
                    .directory
                    .join(format!("passage-{}.pcm", self.request.generation.get())),
            )?,
            mp3: verified_artifact(&self.directory, &mp3_path)?,
            text: verified_artifact(&self.directory, &text_path)?,
            srt: verified_artifact(&self.directory, &srt_path)?,
        };
        let manifest_name = format!("manifest-{}.json", self.request.generation.get());
        let manifest_path = self.directory.join(&manifest_name);
        l04_campaign_gate(
            "live.before_manifest",
            self.request.job_id.0,
            self.request.generation.get(),
            0,
        )?;
        let manifest_result = write_replace_synced(
            &manifest_path,
            &serde_json::to_vec(&record).map_err(|e| e.to_string())?,
        );
        l04_campaign_operation_result(
            "live.before_manifest",
            self.request.job_id.0,
            self.request.generation.get(),
            0,
            &manifest_result,
            Some(&manifest_path),
        )?;
        manifest_result?;
        l04_campaign_gate(
            "live.after_manifest",
            self.request.job_id.0,
            self.request.generation.get(),
            0,
        )?;
        verify_artifact(&self.directory, &record.mp3)?;
        verify_pcm_staging(
            &self.directory,
            &record.staging,
            audio_durable_samples,
            &record.pcm_sha256,
        )?;
        verify_artifact(&self.directory, &record.text)?;
        verify_artifact(&self.directory, &record.srt)?;
        l04_campaign_gate(
            "live.before_current",
            self.request.job_id.0,
            self.request.generation.get(),
            0,
        )?;
        let current_path = self.directory.join("CURRENT");
        let current_result =
            write_replace_synced(&current_path, format!("{manifest_name}\n").as_bytes());
        l04_campaign_operation_result(
            "live.before_current",
            self.request.job_id.0,
            self.request.generation.get(),
            0,
            &current_result,
            Some(&current_path),
        )?;
        current_result?;
        l04_campaign_gate(
            "live.after_current",
            self.request.job_id.0,
            self.request.generation.get(),
            0,
        )?;
        // CURRENT is the commit point. Update compatibility aliases only after it;
        // scan_live repairs aliases from the committed immutable snapshot after a crash.
        let _ = write_replace_synced(&self.directory.join("transcript.txt"), text.as_bytes());
        let _ = write_replace_synced(&self.directory.join("transcript.srt"), srt.as_bytes());
        let _ = fs::remove_file(&self.pending_path);
        self._writer_lock
            .unlock()
            .map_err(|error| format!("StorageUnavailable: archive lock release: {error}"))?;
        Ok(())
    }
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
            || (index_size - INDEX_HEADER.len() as u64) % INDEX_ENTRY_BYTES != 0
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
            || (index_bytes - INDEX_HEADER_BYTES) % INDEX_RECORD_BYTES != 0
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
        if intent.internal_only {
            return Err("PublicationUnavailable: unified exports are internal only".into());
        }
        let wav_path = directory.join("parakeet-unified/source.wav");
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
        Ok(whisper_core::ports::UnifiedPublicationFiles {
            wav_path: wav_path.to_string_lossy().into_owned(),
            txt_path: txt_path.to_string_lossy().into_owned(),
            srt_path: srt_path.to_string_lossy().into_owned(),
        })
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
            expected_active: Self::published_imports(&destination, source_sha256)?
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

    /// Explicitly commits a verified live manifest that was durable before a
    /// crash but was not yet selected by CURRENT.
    pub fn resolve_live_publication(root: &Path, request: &LiveRequest) -> Result<(), String> {
        l04_campaign_assert_archive(root)?;
        let generation = request.generation.get();
        let directory = root
            .join("transcriptions")
            .join(format!("{:032x}", request.job_id.0))
            .join("live");
        let lock_path = directory.join(format!("group-{:032x}.writer.lock", request.job_id.0));
        let writer_lock = OpenOptions::new()
            .read(true)
            .write(true)
            .open(lock_path)
            .map_err(|error| format!("Recoverable: live writer lock unavailable: {error}"))?;
        writer_lock.try_lock().map_err(|error| {
            format!("Recoverable: another live archive writer owns the group: {error}")
        })?;

        let current = live_current_generation(&directory)?;
        let expected_current = generation.checked_sub(1);
        if current != expected_current {
            return Err(
                "Recoverable: live publication is no longer the next group generation".into(),
            );
        }
        let pending_path = directory.join(format!("passage-{generation}.pending.json"));
        let pending: LiveRequest =
            serde_json::from_slice(&fs::read(&pending_path).map_err(|error| {
                format!("Recoverable: live publication request unavailable: {error}")
            })?)
            .map_err(|error| format!("StorageCorrupt: live publication request: {error}"))?;
        if pending != *request {
            return Err("Recoverable: live publication request identity changed".into());
        }

        let record = read_verified_live_manifest(&directory, request.job_id, request.generation)?;
        if record.confirmed_samples != record.audio_samples {
            return Err("Recoverable: live publication does not cover all durable audio".into());
        }
        let entries = scan_live(&directory, request.job_id)?;
        if entries
            .iter()
            .any(|entry| entry.generation.get() > generation)
        {
            return Err(
                "Recoverable: a later live generation prevents publication resolution".into(),
            );
        }
        let target = entries
            .iter()
            .find(|entry| entry.generation == request.generation)
            .ok_or_else(|| "Recoverable: verified live publication is unavailable".to_owned())?;
        if target.complete {
            return Err("Recoverable: live publication is already committed".into());
        }

        let current_path = directory.join("CURRENT");
        let manifest_name = format!("manifest-{generation}.json");
        write_replace_synced(&current_path, format!("{manifest_name}\n").as_bytes())?;
        reconcile_alias(&directory, &record.text, "transcript.txt")?;
        reconcile_alias(&directory, &record.srt, "transcript.srt")?;
        let _ = fs::remove_file(pending_path);
        writer_lock
            .unlock()
            .map_err(|error| format!("StorageUnavailable: archive lock release: {error}"))?;
        Ok(())
    }

    /// Captures the source identity at durable queue admission without creating an archive.
    pub fn identify_source(request: &mut ImportRequest) -> Result<(), String> {
        let source = Path::new(&request.source_path);
        if !source.is_file() {
            return Err("SourceMissing: selected audio source does not exist".into());
        }
        let source_sha = sha256_file(source)?;
        let source_samples = source_output_samples(source)?;
        if sha256_file(source)? != source_sha {
            return Err("SourceChanged: source changed while its identity was inspected".into());
        }
        request.source_sha256 = Some(source_sha);
        request.source_samples = Some(source_samples);
        Ok(())
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
        crate::worker_ipc::record_r7_timing(
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

    fn prepare_inner_with_publication_lock(
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
        crate::worker_ipc::record_r7_timing(
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
        l04_campaign_assert_archive(&self.root)?;
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

    /// Starts the next generation by copying only the previously confirmed prefix.
    pub fn resume_from(
        &mut self,
        previous: &ImportRequest,
        request: ImportRequest,
    ) -> Result<(ImportRequest, Vec<SegmentRecord>), String> {
        self.resume_from_with_catalog_path(previous, request, None)
    }

    /// Validates the checkpoint's durable identity, then prepares with the current catalog path.
    pub fn resume_from_with_catalog_path(
        &mut self,
        previous: &ImportRequest,
        request: ImportRequest,
        executable_model_path: Option<&str>,
    ) -> Result<(ImportRequest, Vec<SegmentRecord>), String> {
        self.resume_from_with_qualification(previous, request, executable_model_path, None)
    }

    pub fn resume_from_with_qualification(
        &mut self,
        previous: &ImportRequest,
        request: ImportRequest,
        executable_model_path: Option<&str>,
        qualification: Option<ArchiveQualificationKey>,
    ) -> Result<(ImportRequest, Vec<SegmentRecord>), String> {
        if request.job_id != previous.job_id
            || previous.generation.next() != Some(request.generation)
        {
            return Err("InvalidInput: resume generation is not the next generation".into());
        }
        let previous_destination = PathBuf::from(&previous.destination);
        let requested_destination = PathBuf::from(&request.destination);
        let same_destination = if previous_destination == requested_destination {
            true
        } else {
            let previous_destination =
                fs::canonicalize(&previous_destination).map_err(|error| {
                    format!("InvalidInput: cannot resolve resume checkpoint destination: {error}")
                })?;
            let requested_destination =
                fs::canonicalize(&requested_destination).map_err(|error| {
                    format!("InvalidInput: cannot resolve requested resume destination: {error}")
                })?;
            previous_destination == requested_destination
        };
        if !same_destination {
            return Err(
                "InvalidInput: resume destination differs from checkpoint destination".into(),
            );
        }
        let previous_dir = PathBuf::from(&previous.destination)
            .join("transcriptions")
            .join(format!("{:032x}", previous.job_id.0))
            .join(previous.generation.get().to_string());
        let pending: PendingRecord = serde_json::from_slice(
            &fs::read(previous_dir.join("pending.json"))
                .map_err(|e| format!("StorageCorrupt: resume checkpoint is unavailable: {e}"))?,
        )
        .map_err(|e| format!("StorageCorrupt: resume checkpoint: {e}"))?;
        validate_pending(&pending, previous.job_id, previous.generation.get())?;
        if pending.strategy != previous.strategy || pending.strategy != request.strategy {
            return Err("StrategyMismatch: resume checkpoint and request strategies differ".into());
        }
        if pending
            .qualification
            .as_ref()
            .is_some_and(ArchiveQualificationKey::is_retired_l2a_key)
        {
            return Err(
                "ModelNotQualified: l'ancienne clé Parakeet L2a est retirée pour toute reprise; le pending est conservé".into(),
            );
        }
        if pending
            .qualification
            .as_ref()
            .is_some_and(ArchiveQualificationKey::is_retired_r5_2_key)
        {
            return Err(
                "ModelNotQualified: l'ancienne clé R5-2 Parakeet est retirée pour toute reprise; le pending est conservé".into(),
            );
        }
        if pending.strategy == whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1
            && pending.model_sha256
                == whisper_core::model_registry::model(
                    whisper_core::model_catalog::ModelId::ParakeetTdt06BV3,
                )
                .artifact
                .sha256
            && pending.source_samples
                > whisper_core::model_registry::PARAKEET_MAX_SINGLE_PASS_DURATION_MS
                    .saturating_mul(16)
        {
            return Err("ModelNotQualified: Fichier trop long pour Parakeet, utilisez Turbo; le pending est conservé".into());
        }
        if pending.source_path != previous.source_path
            || Some(pending.source_sha256.as_str())
                != previous.source_sha256.map(|sha| hex(&sha)).as_deref()
            || Some(pending.source_samples) != previous.source_samples
            || pending.model_path != previous.model_path
            || pending.model_sha256 != hex(&previous.model_sha256)
            || pending.strategy != previous.strategy
            || (if pending.strategy
                == whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
            {
                match (&pending.qualification, &qualification) {
                    (Some(previous), Some(current)) => !previous.permits_bounded_resume_to(current),
                    (None, _) => false,
                    (Some(_), None) => true,
                }
            } else {
                pending.qualification != qualification
            })
        {
            return Err(
                "SourceChanged: resume checkpoint identity differs from the queued request".into(),
            );
        }
        let bounded_resume = pending.strategy
            == whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1;
        let mut resume_checkpoint = None;
        let mut prefix = if bounded_resume {
            let source_samples = pending.source_samples;
            let mut archived_count = 0_u64;
            visit_segments_read_only(&previous_dir.join("segments.jsonl"), |index, segment| {
                let sequence = u64::try_from(index)
                    .ok()
                    .and_then(|value| value.checked_add(1))
                    .ok_or_else(|| "StorageCorrupt: resume sequence overflow".to_owned())?;
                if segment.sequence != sequence
                    || segment.range.sample_rate_hz != 16_000
                    || segment.range.start_sample >= segment.range.end_sample
                    || segment.range.end_sample > source_samples
                    || segment.text.trim().is_empty()
                {
                    return Err("StorageCorrupt: resume segment journal is invalid".into());
                }
                archived_count = sequence;
                Ok(())
            })?;
            let previous_document_directory =
                previous_dir.join("parakeet-unified").join("document");
            let checkpoint =
                crate::parakeet_document::ParakeetDocumentStore::latest_resume_checkpoint(
                    &previous_document_directory,
                    previous.job_id,
                    previous.generation,
                    source_samples,
                    archived_count,
                )?
                .unwrap_or(crate::parakeet_document::ResumeCoverageCheckpoint {
                    window_sequence: 0,
                    covered_samples: 0,
                    confirmed_segments: 0,
                });
            let join_anchor = validate_unified_resume_prefix(
                &previous_dir.join("segments.jsonl"),
                &previous_document_directory,
                previous.job_id,
                previous.generation,
                source_samples,
                checkpoint,
            )?;
            resume_checkpoint = Some(UnifiedArchiveResumeCheckpoint {
                pipeline: crate::parakeet_pipeline::ParakeetResumeCheckpoint {
                    window_sequence: checkpoint.window_sequence,
                    covered_samples: checkpoint.covered_samples,
                    confirmed_segments: checkpoint.confirmed_segments,
                    join_anchor,
                },
                previous_document_directory,
                previous_job_id: previous.job_id,
                previous_generation: previous.generation,
            });
            Vec::new()
        } else {
            read_segments(&previous_dir.join("segments.jsonl"))?
        };
        if !bounded_resume
            && pending
                .qualification
                .as_ref()
                .is_some_and(ArchiveQualificationKey::is_parakeet_vad_test)
        {
            let windows = read_import_vad_windows(&previous_dir, &pending)?;
            let confirmed_segments = windows
                .last()
                .map_or(0, |window| window.last_segment_sequence);
            prefix.truncate(usize::try_from(confirmed_segments).map_err(|_| {
                "StorageCorrupt: VAD confirmed segment prefix exceeds address space".to_owned()
            })?);
        }
        if !bounded_resume
            && prefix.iter().enumerate().any(|(index, segment)| {
                segment.sequence != index as u64 + 1
                    || segment.range.sample_rate_hz != 16_000
                    || segment.range.start_sample >= segment.range.end_sample
                    || segment.range.end_sample > pending.source_samples
                    || segment.text.trim().is_empty()
            })
            || prefix
                .windows(2)
                .any(|pair| pair[1].range.start_sample < pair[0].range.end_sample)
        {
            return Err(
                "StorageCorrupt: confirmed resume prefix is not a valid ordered checkpoint".into(),
            );
        }
        let destination = PathBuf::from(&previous.destination);
        let _publication_lock = Self::lock_publications(&destination)?;
        let previous_reservation =
            publication_reservation_path(&destination, previous.job_id, previous.generation);
        if previous_reservation.is_file() {
            let mut intent: PublicationIntent = serde_json::from_slice(
                &fs::read(&previous_reservation)
                    .map_err(|error| format!("StorageCorrupt: resume reservation: {error}"))?,
            )
            .map_err(|error| format!("StorageCorrupt: resume reservation: {error}"))?;
            if intent.source_sha256 != pending.source_sha256
                || intent.source_samples != pending.source_samples
            {
                return Err("SourceChanged: resume reservation does not match checkpoint".into());
            }
            let current = Self::published_imports_for_strategy(
                &destination,
                &intent.source_sha256,
                pending.strategy,
            )?;
            let current_ids: Vec<_> = current
                .iter()
                .map(|item| PublicationIdentity {
                    job_id: item.job_id,
                    generation: item.generation,
                })
                .collect();
            if current_ids != intent.expected_active {
                return Err("DuplicateStateChanged: publications changed before resume".into());
            }
            intent.operation_id = if request.strategy
                == whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1
            {
                format!("{:032x}-{}", request.job_id.0, request.generation.get())
            } else {
                format!(
                    "{}-{:032x}-{}",
                    request.strategy.stable_id(),
                    request.job_id.0,
                    request.generation.get()
                )
            };
            let next_reservation =
                publication_reservation_path(&destination, request.job_id, request.generation);
            write_publication_intent(&destination, &request, &intent)?;
            if previous_reservation != next_reservation {
                fs::remove_file(&previous_reservation).map_err(|error| {
                    format!("StorageUnavailable: settle previous publication reservation: {error}")
                })?;
            }
        }
        let prepared = self.prepare_inner_with_publication_lock(
            request,
            executable_model_path,
            qualification,
            Some(&destination),
        )?;
        if let Some(checkpoint) = &resume_checkpoint {
            self.copy_unified_resume_prefix(
                &prepared,
                &previous_dir.join("segments.jsonl"),
                checkpoint.pipeline.confirmed_segments,
            )?;
            self.unified_resume_checkpoint = resume_checkpoint;
        } else {
            self.copy_resume_prefix(&prepared, &prefix)?;
        }
        if pending
            .qualification
            .as_ref()
            .is_some_and(ArchiveQualificationKey::is_parakeet_vad_test)
        {
            let records = read_import_vad_windows(&previous_dir, &pending)?;
            if !records.is_empty() {
                let target = PathBuf::from(&prepared.destination)
                    .join("transcriptions")
                    .join(format!("{:032x}", prepared.job_id.0))
                    .join(prepared.generation.get().to_string())
                    .join("vad-windows.jsonl");
                fs::copy(previous_dir.join("vad-windows.jsonl"), target)
                    .map_err(|error| format!("StorageUnavailable: copy VAD checkpoint: {error}"))?;
            }
        }
        Ok((prepared, prefix))
    }

    fn copy_resume_prefix(
        &mut self,
        request: &ImportRequest,
        prefix: &[SegmentRecord],
    ) -> Result<(), String> {
        let target_dir = PathBuf::from(&request.destination)
            .join("transcriptions")
            .join(format!("{:032x}", request.job_id.0))
            .join(request.generation.get().to_string());
        let existing = read_segments(&target_dir.join("segments.jsonl"))?;
        if existing.len() > prefix.len()
            || existing
                .iter()
                .zip(prefix)
                .any(|(stored, expected)| stored != expected)
        {
            return Err("StorageCorrupt: resumed generation contains a divergent prefix".into());
        }
        for segment in prefix.iter().skip(existing.len()) {
            self.persist_segment(&WorkerSegment {
                job_id: request.job_id,
                generation: request.generation,
                instance_id: 0,
                segment_id: whisper_core::SegmentId(segment.sequence),
                range: segment.range,
                text: segment.text.clone(),
            })?;
        }
        Ok(())
    }

    fn copy_unified_resume_prefix(
        &mut self,
        request: &ImportRequest,
        source_journal: &Path,
        confirmed_segments: u64,
    ) -> Result<(), String> {
        let mut copied = 0_u64;
        visit_segments_read_only(source_journal, |index, record| {
            let sequence = u64::try_from(index)
                .ok()
                .and_then(|value| value.checked_add(1))
                .ok_or_else(|| "StorageCorrupt: resume sequence overflow".to_owned())?;
            if sequence > confirmed_segments {
                return Ok(());
            }
            if record.sequence != sequence {
                return Err("StorageCorrupt: resumed prefix sequence changed".into());
            }
            self.persist_segment(&WorkerSegment {
                job_id: request.job_id,
                generation: request.generation,
                instance_id: 0,
                segment_id: whisper_core::SegmentId(sequence),
                range: record.range,
                text: record.text,
            })?;
            copied = sequence;
            Ok(())
        })?;
        if copied != confirmed_segments {
            return Err("StorageCorrupt: confirmed resume prefix is incomplete".into());
        }
        Ok(())
    }

    pub fn persist_segment(&mut self, segment: &WorkerSegment) -> Result<u64, String> {
        let started = Instant::now();
        let active = self
            .current
            .as_mut()
            .ok_or_else(|| "StorageUnavailable: no prepared archive".to_owned())?;
        if segment.job_id != active.job_id || segment.generation != active.generation {
            return Err("StaleResponse: segment does not match pending generation".into());
        }
        if segment.segment_id.0 != active.expected_sequence
            || segment.range.sample_rate_hz != 16_000
            || segment.range.start_sample >= segment.range.end_sample
            || segment.range.end_sample > active.source_samples
            || segment.text.trim().is_empty()
        {
            return Err("StorageCorrupt: invalid segment sequence/range/content".into());
        }
        let record = SegmentRecord {
            sequence: segment.segment_id.0,
            range: segment.range,
            text: segment.text.clone(),
        };
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(active.directory.join("segments.jsonl"))
            .map_err(|e| format!("StorageUnavailable: {e}"))?;
        serde_json::to_writer(&mut file, &record).map_err(|e| e.to_string())?;
        file.write_all(b"\n").map_err(|e| e.to_string())?;
        file.sync_all()
            .map_err(|e| format!("StorageUnavailable: segment sync failed: {e}"))?;
        active.expected_sequence += 1;
        crate::worker_ipc::record_r7_timing(
            "archive_persist_segment",
            started.elapsed(),
            serde_json::json!({ "sequence": record.sequence, "text_bytes": record.text.len() }),
        );
        Ok(record.sequence)
    }

    pub fn prepare_publish(
        &mut self,
        job_id: JobId,
        generation: Generation,
        expected_count: u64,
    ) -> Result<(), String> {
        let started = Instant::now();
        let active = self
            .current
            .as_ref()
            .ok_or_else(|| "StorageUnavailable: no pending archive".to_owned())?;
        if active.job_id != job_id
            || active.generation != generation
            || active.expected_sequence != expected_count.saturating_add(1)
        {
            return Err(
                "StorageCorrupt: End does not match the durably persisted segment sequence".into(),
            );
        }
        if let Some((path, expected_sha)) = &active.segmented_source_manifest
            && (!path.is_file() || sha256_file(path)? != *expected_sha)
        {
            return Err(
                "StorageCorrupt: captured PCM part manifest changed before publication".into(),
            );
        }
        let journal_path = active.directory.join("segments.jsonl");
        validate_segment_journal(&journal_path, expected_count)?;
        let text_path = active.directory.join("transcript.txt");
        let srt_path = active.directory.join("transcript.srt");
        write_replace_streamed(&text_path, |writer| {
            visit_segments(&journal_path, |index, record| {
                if index > 0 {
                    writer.write_all(b"\n").map_err(|error| error.to_string())?;
                }
                writer
                    .write_all(record.text.as_bytes())
                    .map_err(|error| error.to_string())
            })
        })?;
        write_replace_streamed(&srt_path, |writer| {
            visit_segments(&journal_path, |index, record| {
                let start = timestamp_checked(record.range.start_sample)?;
                let end = timestamp_checked(record.range.end_sample)?;
                writeln!(
                    writer,
                    "{}\n{} --> {}\n{}",
                    index + 1,
                    start,
                    end,
                    record.text
                )
                .map_err(|error| error.to_string())
            })
        })?;
        let text_artifact = verified_artifact(&active.directory, &text_path)?;
        let srt_artifact = verified_artifact(&active.directory, &srt_path)?;
        let manifest = CompleteRecord {
            version: 1,
            job_id,
            generation,
            source_sha256: hex(&active.source_sha256),
            segments: expected_count,
            text: text_artifact,
            srt: srt_artifact,
        };
        let manifest_bytes = serde_json::to_vec(&manifest).map_err(|e| e.to_string())?;
        let manifest_path = active.directory.join("manifest.json");
        l04_campaign_gate("import.before_manifest", job_id.0, generation.get(), 0)?;
        let operation = write_replace_synced(&manifest_path, &manifest_bytes);
        l04_campaign_operation_result(
            "import.before_manifest",
            job_id.0,
            generation.get(),
            0,
            &operation,
            Some(&manifest_path),
        )?;
        operation?;
        l04_campaign_gate("import.after_manifest", job_id.0, generation.get(), 0)?;
        let verified: CompleteRecord =
            serde_json::from_slice(&fs::read(&manifest_path).map_err(|e| e.to_string())?)
                .map_err(|e| format!("StorageCorrupt: manifest: {e}"))?;
        verify_artifact(&active.directory, &verified.text)?;
        verify_artifact(&active.directory, &verified.srt)?;
        let marker_path = active.directory.join("r11-publication.json");
        write_replace_synced(
            &marker_path,
            &serde_json::to_vec(&active.publication).map_err(|error| error.to_string())?,
        )?;
        if !active.publication.internal_only {
            for (extension, artifact) in [("txt", &verified.text), ("srt", &verified.srt)] {
                let staged = active.directory.join(format!("export.{extension}.pending"));
                let source = active.directory.join(&artifact.path);
                copy_replace_streamed(&source, &staged)?;
                if hex(&sha256_file(&staged)?) != artifact.sha256 {
                    return Err(format!(
                        "StorageCorrupt: staged export {extension} does not match the manifest"
                    ));
                }
            }
        }
        crate::worker_ipc::record_r7_timing(
            "archive_prepare_publish",
            started.elapsed(),
            serde_json::json!({ "segments": expected_count, "text_bytes": fs::metadata(&text_path).map(|meta| meta.len()).unwrap_or(0), "srt_bytes": fs::metadata(&srt_path).map(|meta| meta.len()).unwrap_or(0) }),
        );
        Ok(())
    }

    pub fn commit_publish(&mut self, job_id: JobId, generation: Generation) -> Result<(), String> {
        let started = Instant::now();
        let active = self
            .current
            .as_ref()
            .ok_or_else(|| "StorageUnavailable: no pending archive".to_owned())?;
        if active.job_id != job_id || active.generation != generation {
            return Err("StaleResponse: publish commit targets another generation".into());
        }
        let root = self.root.clone();
        let _lock = Self::lock_publications(&root)?;
        if !active.publication.internal_only {
            let current = Self::published_imports_for_strategy(
                &root,
                &active.publication.source_sha256,
                active.strategy,
            )?;
            let expected: Vec<_> = active
                .publication
                .expected_active
                .iter()
                .map(|item| (item.job_id, item.generation))
                .collect();
            let actual: Vec<_> = current
                .iter()
                .map(|item| (item.job_id, item.generation))
                .collect();
            if expected != actual {
                return Err(
                    "DuplicateStateChanged: publications changed during processing; confirm again"
                        .into(),
                );
            }
        }
        let publication = active.publication.clone();
        if let Some(cleanup_id) = publication.cleanup_id.as_deref() {
            #[cfg(windows)]
            {
                let pending: PendingRecord = serde_json::from_slice(
                    &fs::read(active.directory.join("pending.json"))
                        .map_err(|error| format!("StorageCorrupt: replacement pending: {error}"))?,
                )
                .map_err(|error| format!("StorageCorrupt: replacement pending: {error}"))?;
                let successor = TreatmentIdentity {
                    destination: root
                        .canonicalize()
                        .map_err(|error| {
                            format!("StorageUnavailable: canonicaliser destination: {error}")
                        })?
                        .to_string_lossy()
                        .into_owned(),
                    job_id,
                    generation,
                };
                let successor_directory = identity_snapshot(
                    crate::windows_file_identity::inspect(&active.directory, true)?,
                )?;
                let successor_files = capture_successor_files(&root, &active.directory)?;
                let mut plan = CleanupPlan {
                    version: 1,
                    cleanup_id: cleanup_id.to_owned(),
                    destination: root
                        .canonicalize()
                        .map_err(|error| {
                            format!("StorageUnavailable: canonicaliser destination: {error}")
                        })?
                        .to_string_lossy()
                        .into_owned(),
                    source_path: pending.source_path,
                    source_sha256: publication.source_sha256.clone(),
                    successor,
                    successor_directory,
                    successor_files,
                    successor_index: None,
                    published: false,
                    targets: publication.cleanup_targets.clone(),
                    last_error: None,
                };
                let path = cleanup_plan_path(&root, cleanup_id);
                if path.exists() {
                    let existing = read_cleanup_plan(&root, cleanup_id)?;
                    if existing != plan {
                        return Err(
                            "DuplicateStateChanged: nettoyage durable déjà différent".into()
                        );
                    }
                    plan = existing;
                } else {
                    write_cleanup_plan(&root, &plan)?;
                }
            }
            #[cfg(not(windows))]
            return Err("Unsupported: le remplacement RTC-02 nécessite les handles Windows".into());
        }
        if !publication.internal_only {
            install_publication_exports_with(
                &root,
                &active.directory,
                &publication,
                self.export_hard_linker.as_ref(),
            )?;
        }
        l04_campaign_gate("import.before_current", job_id.0, generation.get(), 0)?;
        let current_path = active.directory.join("CURRENT");
        let operation = write_replace_synced(&current_path, b"manifest.json\n");
        l04_campaign_operation_result(
            "import.before_current",
            job_id.0,
            generation.get(),
            0,
            &operation,
            Some(&current_path),
        )?;
        operation?;
        l04_campaign_gate("import.after_current", job_id.0, generation.get(), 0)?;
        let index = apply_publication_intent(
            load_or_migrate_publication_index(&root)?,
            job_id,
            generation,
            &publication,
        );
        write_publication_index(&root, &index)?;
        if let Some(cleanup_id) = publication.cleanup_id.as_deref() {
            #[cfg(windows)]
            {
                let mut plan = read_cleanup_plan(&root, cleanup_id)?;
                plan.successor_files = capture_successor_files(&root, &active.directory)?;
                plan.successor_files.push(capture_cleanup_file(
                    &root,
                    &active.directory.join("CURRENT"),
                )?);
                plan.successor_index =
                    Some(capture_cleanup_file(&root, &publication_index_path(&root))?);
                plan.published = true;
                write_cleanup_plan(&root, &plan)?;
                if let Err(error) = run_cleanup_plan_locked(&root, &mut plan) {
                    plan.last_error = Some(error);
                    write_cleanup_plan(&root, &plan)?;
                }
            }
        }
        if !publication.internal_only {
            reconcile_replaced_exports(&root, &index)?;
        }
        Self::clear_duplicate_reservation(&root, job_id, generation)?;
        self.current = None;
        crate::worker_ipc::record_r7_timing(
            "archive_commit_publish",
            started.elapsed(),
            serde_json::json!({ "job_id": format!("{:032x}", job_id.0), "generation": generation.get() }),
        );
        Ok(())
    }

    pub fn publish(
        &mut self,
        job_id: JobId,
        generation: Generation,
        expected_count: u64,
    ) -> Result<(), String> {
        self.prepare_publish(job_id, generation, expected_count)?;
        self.commit_publish(job_id, generation)
    }

    pub fn scan(root: &Path) -> Result<Vec<ArchiveEntry>, String> {
        let started = Instant::now();
        let result = Self::scan_publications(root);
        crate::worker_ipc::record_r7_timing(
            "archive-publication-scan",
            started.elapsed(),
            serde_json::json!({
                "scan_operation": "ArchiveStore::scan",
                "destination": root.to_string_lossy(),
                "phase": "publication-scan",
                "outcome": if result.is_ok() { "ok" } else { "error" },
            }),
        );
        result
    }

    fn scan_publications(root: &Path) -> Result<Vec<ArchiveEntry>, String> {
        let mut entries = Self::scan_raw(root)?;
        let index_path = publication_index_path(root);
        let index = if index_path.exists() {
            Some(read_publication_index(root)?)
        } else {
            None
        };
        entries.retain_mut(|entry| {
            if !entry.complete {
                return true;
            }
            let directory = generation_path(root, entry.job_id, entry.generation);
            let record = index.as_ref().and_then(|value| {
                value.records.iter().find(|record| {
                    record.job_id == entry.job_id && record.generation == entry.generation
                })
            });
            if directory.join("r11-publication.json").is_file() {
                if let Some(record) = record {
                    if !record.active {
                        return false;
                    }
                    entry.export_basename = record.export_basename.clone();
                    return true;
                }
                entry.complete = false;
                entry.publication_recovery = true;
                entry.diagnostic = Some(
                    "Recoverable: publication R11 complète, en attente d'activation explicite"
                        .into(),
                );
                return true;
            }
            if let Some(record) = record {
                if !record.active {
                    return false;
                }
                entry.export_basename = record.export_basename.clone();
            }
            true
        });
        Ok(entries)
    }

    fn scan_raw(root: &Path) -> Result<Vec<ArchiveEntry>, String> {
        l04_campaign_assert_archive(root)?;
        let mut entries = Vec::new();
        let base = root.join("transcriptions");
        if !base.exists() {
            return Ok(entries);
        }
        for job in fs::read_dir(base).map_err(|e| e.to_string())? {
            let job = match job {
                Ok(job) => job,
                Err(error) => {
                    entries.push(corrupt_entry(JobId(0), "job directory", error.to_string()));
                    continue;
                }
            };
            // The durable FIFO journal shares this parent with per-job directories.
            // It is not an archive and must not be reported as a corrupt JobId(0).
            if job.file_name() == "queue.jsonl" {
                continue;
            }
            let generations = match fs::read_dir(job.path()) {
                Ok(generations) => generations,
                Err(error) => {
                    let job_id =
                        parse_job_id(&job.file_name().to_string_lossy()).unwrap_or(JobId(0));
                    entries.push(corrupt_entry(job_id, "job directory", error.to_string()));
                    continue;
                }
            };
            for generation in generations {
                let generation = match generation {
                    Ok(generation) => generation,
                    Err(error) => {
                        let job_id =
                            parse_job_id(&job.file_name().to_string_lossy()).unwrap_or(JobId(0));
                        entries.push(corrupt_entry(
                            job_id,
                            "generation directory",
                            error.to_string(),
                        ));
                        continue;
                    }
                };
                let dir = generation.path();
                let job_id = parse_job_id(&job.file_name().to_string_lossy()).unwrap_or(JobId(0));
                if generation.file_name() == "live" {
                    match scan_live(&dir, job_id) {
                        Ok(live_entries) => entries.extend(live_entries),
                        Err(error) => entries.push(corrupt_entry(job_id, "live", error)),
                    }
                    continue;
                }
                match scan_generation(
                    &dir,
                    &job.file_name().to_string_lossy(),
                    &generation.file_name().to_string_lossy(),
                ) {
                    Ok(Some(entry)) => entries.push(entry),
                    Ok(None) => continue,
                    Err(error) => entries.push(corrupt_entry(
                        job_id,
                        &generation.file_name().to_string_lossy(),
                        error,
                    )),
                }
            }
        }
        entries.sort_by_key(|entry| (entry.job_id.0, entry.generation.get()));
        Ok(entries)
    }
}

fn publication_index_path(root: &Path) -> PathBuf {
    root.join(".whisper-publications").join("index.json")
}

fn publication_reservation_path(root: &Path, job_id: JobId, generation: Generation) -> PathBuf {
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

fn write_publication_reservation(
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

fn write_publication_intent(
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

fn load_publication_intent(directory: &Path) -> Result<PublicationIntent, String> {
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

fn read_publication_index(root: &Path) -> Result<PublicationIndex, String> {
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

fn write_publication_index(root: &Path, index: &PublicationIndex) -> Result<(), String> {
    let path = publication_index_path(root);
    fs::create_dir_all(path.parent().expect("index parent"))
        .map_err(|error| format!("StorageUnavailable: publication index directory: {error}"))?;
    write_replace_synced(
        &path,
        &serde_json::to_vec(index).map_err(|error| error.to_string())?,
    )
}

fn install_publication_exports(
    root: &Path,
    directory: &Path,
    intent: &PublicationIntent,
) -> Result<(), String> {
    install_publication_exports_with(root, directory, intent, &|source, target| {
        fs::hard_link(source, target)
    })
}

fn install_publication_exports_with(
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

fn install_publication_exports_with_copy_hook(
    root: &Path,
    directory: &Path,
    intent: &PublicationIntent,
    hard_linker: &dyn Fn(&Path, &Path) -> io::Result<()>,
    after_final_copy: &dyn Fn(&str, &Path) -> Result<(), String>,
) -> Result<(), String> {
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
            hard_linker,
            after_final_copy,
        )?;
        install_owned_export(
            &exports,
            directory,
            &ownership_path,
            &mut ownership,
            &manifest.srt,
            "srt",
            hard_linker,
            after_final_copy,
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
        hard_linker,
        after_final_copy,
    )?;
    install_owned_export(
        &exports,
        directory,
        &ownership_path,
        &mut ownership,
        &manifest.srt,
        "srt",
        hard_linker,
        after_final_copy,
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

fn export_install_temporary_path(directory: &Path, extension: &str) -> PathBuf {
    directory.join(format!("export.{extension}.install.pending"))
}

fn persisted_file_identity(path: &Path) -> Result<String, String> {
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

fn install_owned_export(
    exports: &Path,
    generation: &Path,
    ownership_path: &Path,
    ownership: &mut InstalledExportOwnership,
    artifact: &ArtifactRecord,
    extension: &str,
    hard_linker: &dyn Fn(&Path, &Path) -> io::Result<()>,
    after_final_copy: &dyn Fn(&str, &Path) -> Result<(), String>,
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
    if hard_linker(&temporary, &final_path).is_ok() {
        set_export_file_identity(ownership, extension, expected_temporary_identity.clone());
    } else {
        if final_path.exists() {
            return Err(format!(
                "StorageUnavailable: export name collision for {extension}"
            ));
        }
        copy_file_create_new(&temporary, &final_path)?;
        after_final_copy(extension, &final_path)?;
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

fn apply_publication_intent(
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

fn reserve_export_basename(root: &Path, source_path: &str) -> Result<String, String> {
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

fn reconcile_replaced_exports(root: &Path, index: &PublicationIndex) -> Result<(), String> {
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

fn scan_live(dir: &Path, expected_job: JobId) -> Result<Vec<ArchiveEntry>, String> {
    l04_campaign_gate("recovery.before_scan_live", expected_job.0, 0, 0)?;
    if format!("{:032x}", expected_job.0)
        != dir
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .unwrap_or("")
    {
        return Err("live job folder is not canonical".into());
    }
    let mut entries = Vec::new();
    let mut verified = std::collections::BTreeSet::new();
    let committed_generation = live_current_generation(dir)?;
    for item in fs::read_dir(dir).map_err(|error| error.to_string())? {
        let item = item.map_err(|error| error.to_string())?;
        let name = item.file_name().to_string_lossy().into_owned();
        if let Some(number) = name
            .strip_prefix("manifest-")
            .and_then(|s| s.strip_suffix(".json"))
        {
            let generation_number = number
                .parse::<u64>()
                .ok()
                .filter(|value| value.to_string() == number && *value > 0)
                .ok_or_else(|| format!("invalid live manifest name: {name}"))?;
            let generation: Generation =
                serde_json::from_value(serde_json::Value::from(generation_number))
                    .map_err(|error| format!("invalid live generation: {error}"))?;
            read_verified_live_manifest(dir, expected_job, generation)?;
            verified.insert(generation_number);
            let complete =
                committed_generation.is_some_and(|committed| generation_number <= committed);
            entries.push(ArchiveEntry {
                job_id: expected_job,
                generation,
                complete,
                source_sha256: String::new(),
                diagnostic: (!complete).then(|| {
                    "Recoverable: passage manifest exists but CURRENT did not commit it".into()
                }),
                export_basename: None,
                publication_recovery: false,
                strategy: legacy_transcription_strategy(),
            });
        }
    }
    if let Some(number) = committed_generation {
        if !verified.contains(&number) {
            return Err("StorageCorrupt: live CURRENT target is not a verified passage".into());
        }
        let manifest_path = dir.join(format!("manifest-{number}.json"));
        let record: LiveCompleteRecord =
            serde_json::from_slice(&fs::read(manifest_path).map_err(|error| error.to_string())?)
                .map_err(|error| format!("StorageCorrupt: committed live manifest: {error}"))?;
        reconcile_alias(dir, &record.text, "transcript.txt")?;
        reconcile_alias(dir, &record.srt, "transcript.srt")?;
    }
    for item in fs::read_dir(dir).map_err(|error| error.to_string())? {
        let item = item.map_err(|error| error.to_string())?;
        let name = item.file_name().to_string_lossy().into_owned();
        if let Some(number) = name
            .strip_prefix("passage-")
            .and_then(|s| s.strip_suffix(".pending.json"))
        {
            let generation_number = number
                .parse::<u64>()
                .ok()
                .filter(|value| value.to_string() == number && *value > 0)
                .ok_or_else(|| format!("invalid pending live passage name: {name}"))?;
            let generation: Generation =
                serde_json::from_value(serde_json::Value::from(generation_number))
                    .map_err(|error| format!("invalid pending generation: {error}"))?;
            let request: LiveRequest =
                serde_json::from_slice(&fs::read(item.path()).map_err(|e| e.to_string())?)
                    .map_err(|error| format!("StorageCorrupt: live pending record: {error}"))?;
            if request.job_id != expected_job || request.generation != generation {
                return Err(format!(
                    "StorageCorrupt: live pending identity mismatch: {name}"
                ));
            }
            if !entries.iter().any(|entry| entry.generation == generation) {
                entries.push(ArchiveEntry { job_id: expected_job, generation, complete: false,
                    source_sha256: String::new(), diagnostic: Some("Recoverable: live passage was not atomically published; explicit resume or discard required".into()), export_basename: None, publication_recovery: false, strategy: legacy_transcription_strategy() });
            }
        }
    }
    entries.sort_by_key(|entry| entry.generation.get());
    Ok(entries)
}

fn reconcile_alias(dir: &Path, source: &ArtifactRecord, alias: &str) -> Result<(), String> {
    let contents = fs::read(dir.join(&source.path)).map_err(|error| error.to_string())?;
    let alias_path = dir.join(alias);
    if fs::read(&alias_path).ok().as_deref() != Some(contents.as_slice()) {
        write_replace_synced(&alias_path, &contents)?;
    }
    Ok(())
}

fn live_current_generation(dir: &Path) -> Result<Option<u64>, String> {
    let current_path = dir.join("CURRENT");
    if !current_path.exists() {
        return Ok(None);
    }
    let pointer = fs::read_to_string(&current_path).map_err(|e| e.to_string())?;
    let target = pointer.strip_suffix('\n').ok_or_else(|| {
        "StorageCorrupt: live CURRENT pointer is not newline terminated".to_owned()
    })?;
    if pointer != format!("{target}\n")
        || !target.starts_with("manifest-")
        || !target.ends_with(".json")
        || !safe_artifact_path(target)
    {
        return Err("StorageCorrupt: live CURRENT pointer is invalid".into());
    }
    target
        .strip_prefix("manifest-")
        .and_then(|s| s.strip_suffix(".json"))
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|number| {
            *number > 0
                && number.to_string()
                    == target
                        .trim_start_matches("manifest-")
                        .trim_end_matches(".json")
        })
        .map(Some)
        .ok_or_else(|| "StorageCorrupt: live CURRENT target is invalid".into())
}

fn read_verified_live_manifest(
    dir: &Path,
    expected_job: JobId,
    generation: Generation,
) -> Result<LiveCompleteRecord, String> {
    let name = format!("manifest-{}.json", generation.get());
    let record: LiveCompleteRecord = serde_json::from_slice(
        &fs::read(dir.join(&name))
            .map_err(|error| format!("StorageCorrupt: live manifest: {error}"))?,
    )
    .map_err(|error| format!("StorageCorrupt: live manifest: {error}"))?;
    if record.version != 1
        || record.job_id != expected_job
        || record.generation != generation
        || record.audio_samples == 0
        || !is_sha256(&record.pcm_sha256)
        || !safe_artifact_path(&record.staging.path)
        || !safe_artifact_path(&record.mp3.path)
        || !safe_artifact_path(&record.text.path)
        || !safe_artifact_path(&record.srt.path)
    {
        return Err(format!(
            "StorageCorrupt: live manifest identity or shape mismatch: {name}"
        ));
    }
    verify_artifact(dir, &record.staging)?;
    verify_pcm_staging(
        dir,
        &record.staging,
        record.audio_samples,
        &record.pcm_sha256,
    )?;
    verify_artifact(dir, &record.mp3)?;
    verify_artifact(dir, &record.text)?;
    verify_artifact(dir, &record.srt)?;
    Ok(record)
}

pub fn scan_live_recoveries(
    root: &Path,
) -> Result<Vec<whisper_core::ports::LiveRecoveryInfo>, String> {
    let base = root.join("transcriptions");
    if !base.exists() {
        return Ok(Vec::new());
    }
    let mut recoveries = Vec::new();
    for job_entry in fs::read_dir(base).map_err(|e| e.to_string())? {
        let job_entry = job_entry.map_err(|e| e.to_string())?;
        let job_name = job_entry.file_name().to_string_lossy().into_owned();
        let Some(job_id) =
            parse_job_id(&job_name).filter(|id| format!("{:032x}", id.0) == job_name)
        else {
            continue;
        };
        let dir = job_entry.path().join("live");
        if !dir.is_dir() {
            continue;
        }
        let current_generation = live_current_generation(&dir)?;
        let mut latest_generation = 0u64;
        let mut latest_pending: Option<(u64, PathBuf)> = None;
        let mut manifest_generations = std::collections::BTreeSet::new();
        for item in fs::read_dir(&dir).map_err(|e| e.to_string())? {
            let item = item.map_err(|e| e.to_string())?;
            let name = item.file_name().to_string_lossy().into_owned();
            if let Some(number) = name
                .strip_prefix("manifest-")
                .and_then(|s| s.strip_suffix(".json"))
                && let Ok(generation) = number.parse::<u64>()
                && generation > 0
                && generation.to_string() == number
            {
                latest_generation = latest_generation.max(generation);
                manifest_generations.insert(generation);
            }
            if let Some(number) = name
                .strip_prefix("passage-")
                .and_then(|s| s.strip_suffix(".pending.json"))
                && let Ok(generation) = number.parse::<u64>()
                && generation > 0
                && generation.to_string() == number
            {
                if generation >= latest_pending.as_ref().map_or(0, |(latest, _)| *latest) {
                    latest_pending = Some((generation, item.path()));
                }
                latest_generation = latest_generation.max(generation);
            }
        }
        let Some((generation, pending_path)) = latest_pending else {
            continue;
        };
        let is_next_generation = match current_generation {
            Some(current) => current.checked_add(1) == Some(generation),
            None => generation == 1,
        };
        if generation != latest_generation || !is_next_generation {
            continue;
        }
        let request: LiveRequest =
            serde_json::from_slice(&fs::read(pending_path).map_err(|e| e.to_string())?)
                .map_err(|e| format!("StorageCorrupt: live recovery request: {e}"))?;
        if request.job_id != job_id || request.generation.get() != generation {
            continue;
        }
        let pcm_path = dir.join(format!("passage-{generation}.pcm"));
        let Ok(checkpoint) = crate::staging::PcmStaging::inspect_checkpoint(&pcm_path) else {
            continue;
        };
        let publication_ready = manifest_generations.contains(&generation);
        let manifest = if publication_ready {
            Some(read_verified_live_manifest(
                &dir,
                job_id,
                request.generation,
            )?)
        } else {
            None
        };
        l04_campaign_gate("recovery.before_coverage", job_id.0, generation, 0)?;
        let scan_result =
            scan_confirmed_live_prefix(&dir, generation, &request, checkpoint.durable_samples);
        l04_campaign_operation_result(
            "recovery.before_coverage",
            job_id.0,
            generation,
            0,
            &scan_result,
            None,
        )?;
        let confirmed_samples = scan_result?;
        if let Some(manifest) = &manifest
            && (manifest.audio_samples != checkpoint.durable_samples
                || manifest.confirmed_samples != confirmed_samples
                || manifest.confirmed_samples != manifest.audio_samples)
        {
            continue;
        }
        l04_campaign_gate("recovery.after_coverage", job_id.0, generation, 0)?;
        recoveries.push(whisper_core::ports::LiveRecoveryInfo {
            request,
            durable_samples: checkpoint.durable_samples,
            confirmed_samples,
            publication_ready,
        });
    }
    Ok(recoveries)
}

fn scan_confirmed_live_prefix(
    dir: &Path,
    generation: u64,
    request: &LiveRequest,
    durable_samples: u64,
) -> Result<u64, String> {
    let coverage_path = dir.join(format!("passage-{generation}.coverage.jsonl"));
    if !coverage_path.exists() {
        return Ok(0);
    }
    let segment_path = dir.join(format!("passage-{generation}.segments.jsonl"));
    let segments = read_bounded_jsonl::<SegmentRecord>(&segment_path)?;
    let coverage = read_bounded_jsonl::<CoverageRecord>(&coverage_path)?;
    let mut confirmed_samples = 0_u64;
    let mut expected_window_sequence = 1_u64;
    let mut confirmed_segment_sequence = 0_u64;
    for record in coverage {
        let payload = record.payload;
        let canonical = serde_json::to_vec(&payload).map_err(|error| error.to_string())?;
        let checksum = format!("{:x}", Sha256::digest(canonical));
        if record.sha256 != checksum
            || payload.version != 2
            || payload.job_id != request.job_id
            || payload.generation != request.generation
            || payload.window_sequence != expected_window_sequence
            || payload.start_sample != confirmed_samples
            || payload.end_sample <= payload.start_sample
            || payload.end_sample > durable_samples
            || payload.last_segment_sequence < confirmed_segment_sequence
            || payload.last_segment_sequence as usize > segments.len()
        {
            return Err(
                "StorageCorrupt: live coverage journal is not a verified contiguous prefix".into(),
            );
        }
        let start = confirmed_segment_sequence as usize;
        let end = payload.last_segment_sequence as usize;
        let group_start = request
            .group_offset_samples
            .checked_add(payload.start_sample)
            .ok_or_else(|| "StorageCorrupt: coverage offset overflow".to_owned())?;
        let group_end = request
            .group_offset_samples
            .checked_add(payload.end_sample)
            .ok_or_else(|| "StorageCorrupt: coverage offset overflow".to_owned())?;
        if segments[start..end]
            .iter()
            .enumerate()
            .any(|(offset, segment)| {
                segment.sequence != confirmed_segment_sequence + offset as u64 + 1
                    || segment.range.sample_rate_hz != 16_000
                    || segment.range.start_sample < group_start
                    || segment.range.end_sample > group_end
                    || segment.range.start_sample >= segment.range.end_sample
            })
        {
            return Err(
                "StorageCorrupt: coverage journal does not cover its persisted segments".into(),
            );
        }
        confirmed_samples = payload.end_sample;
        confirmed_segment_sequence = payload.last_segment_sequence;
        expected_window_sequence = expected_window_sequence
            .checked_add(1)
            .ok_or_else(|| "StorageCorrupt: coverage sequence exhausted".to_owned())?;
    }
    Ok(confirmed_samples)
}

fn read_bounded_jsonl<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<Vec<T>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = File::open(path).map_err(|error| error.to_string())?;
    let mut input = BufReader::new(file);
    let mut records = Vec::new();
    loop {
        let mut bytes = Vec::with_capacity(4096);
        let mut complete = false;
        loop {
            let available = input.fill_buf().map_err(|error| error.to_string())?;
            if available.is_empty() {
                break;
            }
            let consumed = available
                .iter()
                .position(|byte| *byte == b'\n')
                .map_or(available.len(), |index| index + 1);
            if bytes.len().saturating_add(consumed) > 1_048_576 {
                return Err("StorageCorrupt: JSONL record exceeds 1 MiB".into());
            }
            let has_newline = available.get(consumed.saturating_sub(1)) == Some(&b'\n');
            bytes.extend_from_slice(&available[..consumed]);
            input.consume(consumed);
            if has_newline {
                complete = true;
                break;
            }
        }
        if bytes.is_empty() || !complete {
            break;
        }
        bytes.pop();
        let record = serde_json::from_slice(&bytes)
            .map_err(|error| format!("StorageCorrupt: JSONL record: {error}"))?;
        records.push(record);
    }
    Ok(records)
}

fn safe_artifact_path(path: &str) -> bool {
    !path.is_empty()
        && Path::new(path)
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_)))
}

fn unix_time_ms() -> Result<u64, String> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .map_err(|error| format!("ClockUnavailable: system clock precedes Unix epoch: {error}"))
}

fn scan_generation(
    dir: &Path,
    job_folder: &str,
    generation_folder: &str,
) -> Result<Option<ArchiveEntry>, String> {
    let expected_job = parse_job_id(job_folder)
        .filter(|id| format!("{:032x}", id.0) == job_folder)
        .ok_or_else(|| "job folder is not a canonical 32-digit identity".to_owned())?;
    let expected_generation = generation_folder
        .parse::<u64>()
        .ok()
        .filter(|number| number.to_string() == generation_folder && *number > 0)
        .ok_or_else(|| "generation folder is not a canonical positive integer".to_owned())?;
    if dir.join("CURRENT").is_file() {
        let pointer = fs::read(dir.join("CURRENT")).map_err(|e| e.to_string())?;
        if pointer.as_slice() != b"manifest.json\n" {
            return Err("CURRENT must reference exactly manifest.json".into());
        }
        let manifest: CompleteRecord = serde_json::from_slice(
            &fs::read(dir.join("manifest.json")).map_err(|e| e.to_string())?,
        )
        .map_err(|e| format!("StorageCorrupt: manifest: {e}"))?;
        let pending: PendingRecord =
            serde_json::from_slice(&fs::read(dir.join("pending.json")).map_err(|e| e.to_string())?)
                .map_err(|e| format!("StorageCorrupt: pending: {e}"))?;
        validate_pending(&pending, expected_job, expected_generation)?;
        let _ = read_import_vad_windows(dir, &pending)?;
        if manifest.version != 1
            || manifest.job_id != expected_job
            || manifest.job_id != pending.job_id
            || manifest.generation.get() != expected_generation
            || manifest.generation != pending.generation
            || manifest.source_sha256 != pending.source_sha256
            || !is_sha256(&manifest.source_sha256)
            || manifest.text.path != "transcript.txt"
            || manifest.srt.path != "transcript.srt"
            || !is_sha256(&manifest.text.sha256)
            || !is_sha256(&manifest.srt.sha256)
        {
            return Err(
                "StorageCorrupt: manifest identity/schema does not match pending generation".into(),
            );
        }
        verify_artifact(dir, &manifest.text)?;
        verify_artifact(dir, &manifest.srt)?;
        let records = read_segments(&dir.join("segments.jsonl"))?;
        if records.len() as u64 != manifest.segments
            || records.iter().enumerate().any(|(index, record)| {
                record.sequence != index as u64 + 1
                    || record.range.sample_rate_hz != 16_000
                    || record.range.start_sample >= record.range.end_sample
                    || record.range.end_sample > pending.source_samples
            })
        {
            return Err(
                "StorageCorrupt: manifest segment count/ranges do not match the journal".into(),
            );
        }
        let transcript = fs::read(dir.join("transcript.txt")).map_err(|e| e.to_string())?;
        let expected_text = records
            .iter()
            .map(|record| record.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        if transcript != expected_text.as_bytes() {
            return Err(
                "StorageCorrupt: transcript does not match the verified segment journal".into(),
            );
        }
        return Ok(Some(ArchiveEntry {
            job_id: expected_job,
            generation: pending.generation,
            complete: true,
            source_sha256: pending.source_sha256,
            diagnostic: None,
            export_basename: None,
            publication_recovery: false,
            strategy: pending.strategy,
        }));
    }
    if dir.join("pending.json").is_file() {
        let pending: PendingRecord =
            serde_json::from_slice(&fs::read(dir.join("pending.json")).map_err(|e| e.to_string())?)
                .map_err(|e| format!("StorageCorrupt: pending: {e}"))?;
        validate_pending(&pending, expected_job, expected_generation)?;
        let _ = read_import_vad_windows(dir, &pending)?;
        return Ok(Some(ArchiveEntry {
            job_id: pending.job_id,
            generation: pending.generation,
            complete: false,
            source_sha256: pending.source_sha256,
            diagnostic: None,
            export_basename: None,
            publication_recovery: false,
            strategy: pending.strategy,
        }));
    }
    if dir.join("manifest.json").exists() || dir.join("segments.jsonl").exists() {
        return Err("StorageCorrupt: artifacts exist without a pending or CURRENT marker".into());
    }
    Ok(None)
}

fn validate_pending(pending: &PendingRecord, job_id: JobId, generation: u64) -> Result<(), String> {
    let parakeet_sha =
        whisper_core::model_registry::model(whisper_core::model_catalog::ModelId::ParakeetTdt06BV3)
            .artifact
            .sha256;
    let unified = pending.version == 4
        && pending.strategy
            == whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
        && pending.qualification.is_none()
        && pending.model_sha256 == parakeet_sha;
    let qualification_valid = if unified {
        true
    } else if pending.model_sha256 == parakeet_sha {
        (pending.version == 2
            && pending.qualification.as_ref().is_some_and(|key| {
                key == &ArchiveQualificationKey::parakeet_cpu_test()
                    || key == &ArchiveQualificationKey::parakeet_gpu_test()
                    || key.is_registered_personal_import()
                    || key.is_retired_l2a_key()
                    || key.is_retired_r5_2_key()
                    || key.is_personal_candidate()
            }))
            || (pending.version == 3
                && pending
                    .qualification
                    .as_ref()
                    .is_some_and(ArchiveQualificationKey::is_parakeet_vad_test))
    } else {
        pending.version == 1 && pending.qualification.is_none()
    };
    if (!matches!(pending.version, 1 | 2 | 3) && !unified)
        || (pending.version < 4 && pending.strategy != legacy_transcription_strategy())
        || pending.job_id != job_id
        || pending.generation.get() != generation
        || !is_sha256(&pending.source_sha256)
        || pending.source_samples == 0
        || !is_registered_model(&pending.model_sha256, pending.model_size)
        || !qualification_valid
    {
        return Err("StorageCorrupt: pending identity/schema is invalid".into());
    }
    Ok(())
}

fn read_import_vad_windows(
    directory: &Path,
    pending: &PendingRecord,
) -> Result<Vec<ImportVadWindowPayload>, String> {
    let Some(qualification) = pending
        .qualification
        .as_ref()
        .filter(|key| key.is_parakeet_vad_test())
    else {
        if directory.join("vad-windows.jsonl").exists() {
            return Err("StorageCorrupt: VAD checkpoint exists without an import-vad key".into());
        }
        return Ok(Vec::new());
    };
    let path = directory.join("vad-windows.jsonl");
    if !path.exists() {
        return Ok(Vec::new());
    }
    let entries = read_bounded_jsonl::<ImportVadWindowRecord>(&path)?;
    let segments = read_segments(&directory.join("segments.jsonl"))?;
    let mut output = Vec::with_capacity(entries.len());
    let mut previous_end = 0_u64;
    let mut previous_segment_sequence = 0_u64;
    for (index, entry) in entries.into_iter().enumerate() {
        let canonical = serde_json::to_vec(&entry.payload).map_err(|error| error.to_string())?;
        let expected_hash = format!("{:x}", Sha256::digest(canonical));
        let payload = entry.payload;
        if entry.sha256 != expected_hash
            || payload.version != 1
            || payload.job_id != pending.job_id
            || payload.generation != pending.generation
            || payload.sequence != index as u64 + 1
            || payload.central_start_sample != previous_end
            || payload.central_start_sample >= payload.central_end_sample
            || payload.central_end_sample > pending.source_samples
            || payload.native_start_sample > payload.central_start_sample
            || payload.native_end_sample < payload.central_end_sample
            || payload.native_end_sample - payload.native_start_sample
                > whisper_core::parakeet_windows::MAX_NATIVE_SAMPLES
            || payload.last_segment_sequence < previous_segment_sequence
            || payload.last_segment_sequence > segments.len() as u64
            || payload.qualification != *qualification
        {
            return Err("StorageCorrupt: import-vad checkpoint identity/range is invalid".into());
        }
        let owned_segments = segments
            .get(previous_segment_sequence as usize..payload.last_segment_sequence as usize)
            .ok_or_else(|| {
                "StorageCorrupt: VAD window references missing transcript segments".to_owned()
            })?;
        if owned_segments.iter().any(|segment| {
            let center = segment.range.start_sample
                + (segment.range.end_sample - segment.range.start_sample) / 2;
            center < payload.central_start_sample || center >= payload.central_end_sample
        }) {
            return Err("StorageCorrupt: transcript segment belongs to another VAD window".into());
        }
        previous_end = payload.central_end_sample;
        previous_segment_sequence = payload.last_segment_sequence;
        output.push(payload);
    }
    Ok(output)
}

fn initialize_pending(directory: &Path, pending: &PendingRecord) -> Result<u64, String> {
    let pending_path = directory.join("pending.json");
    if pending_path.exists() {
        if directory.join("CURRENT").exists() {
            return Err("InvalidInput: generation is already published".into());
        }
        let existing: PendingRecord = serde_json::from_slice(
            &fs::read(&pending_path).map_err(|e| format!("StorageCorrupt: pending record: {e}"))?,
        )
        .map_err(|e| format!("StorageCorrupt: pending record: {e}"))?;
        if existing != *pending {
            return Err("SourceChanged: existing pending generation has another identity".into());
        }
        Ok(read_segments(&directory.join("segments.jsonl"))?.len() as u64 + 1)
    } else {
        write_new_synced(
            &pending_path,
            &serde_json::to_vec(pending).map_err(|e| e.to_string())?,
        )?;
        Ok(1)
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn parse_job_id(value: &str) -> Option<JobId> {
    u128::from_str_radix(value, 16).ok().map(JobId)
}

fn corrupt_entry(job_id: JobId, location: &str, error: String) -> ArchiveEntry {
    ArchiveEntry {
        job_id,
        generation: Generation::first(),
        complete: false,
        source_sha256: String::new(),
        diagnostic: Some(format!("StorageCorrupt: {location}: {error}")),
        export_basename: None,
        publication_recovery: false,
        strategy: legacy_transcription_strategy(),
    }
}

fn read_segments(path: &Path) -> Result<Vec<SegmentRecord>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|e| format!("StorageCorrupt: segment journal: {e}"))?;
    repair_uncommitted_segment_tail(&mut file)?;
    file.seek(SeekFrom::Start(0))
        .map_err(|e| format!("StorageCorrupt: segment seek: {e}"))?;
    BufReader::new(file)
        .lines()
        .map(|line| {
            let line = line.map_err(|e| format!("StorageCorrupt: segment: {e}"))?;
            serde_json::from_str(&line).map_err(|e| format!("StorageCorrupt: segment: {e}"))
        })
        .collect()
}

fn validate_segment_journal(path: &Path, expected_count: u64) -> Result<(), String> {
    let mut observed = 0_u64;
    visit_segments(path, |index, record| {
        let expected = index as u64 + 1;
        if record.sequence != expected {
            return Err("StorageCorrupt: segment journal sequence is not contiguous".into());
        }
        observed = expected;
        Ok(())
    })?;
    if observed != expected_count {
        return Err("StorageCorrupt: segment journal is incomplete".into());
    }
    Ok(())
}

fn visit_segments(
    path: &Path,
    mut visit: impl FnMut(usize, SegmentRecord) -> Result<(), String>,
) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|error| format!("StorageCorrupt: segment journal: {error}"))?;
    repair_uncommitted_segment_tail(&mut file)?;
    file.seek(SeekFrom::Start(0))
        .map_err(|error| format!("StorageCorrupt: segment seek: {error}"))?;
    let mut reader = BufReader::new(file);
    let mut index = 0_usize;
    while let Some(line) = read_segment_journal_line(&mut reader)? {
        let record: SegmentRecord = serde_json::from_slice(&line)
            .map_err(|error| format!("StorageCorrupt: segment: {error}"))?;
        visit(index, record)?;
        index = index
            .checked_add(1)
            .ok_or_else(|| "StorageCorrupt: segment count exceeds address space".to_owned())?;
    }
    Ok(())
}

fn visit_segments_read_only(
    path: &Path,
    mut visit: impl FnMut(usize, SegmentRecord) -> Result<(), String>,
) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    let file =
        File::open(path).map_err(|error| format!("StorageCorrupt: segment journal: {error}"))?;
    let mut reader = BufReader::new(file);
    let mut index = 0_usize;
    while let Some(line) = read_segment_journal_line(&mut reader)? {
        let record: SegmentRecord = serde_json::from_slice(&line)
            .map_err(|error| format!("StorageCorrupt: segment: {error}"))?;
        visit(index, record)?;
        index = index
            .checked_add(1)
            .ok_or_else(|| "StorageCorrupt: segment count exceeds address space".to_owned())?;
    }
    Ok(())
}

fn validate_unified_resume_prefix(
    segment_journal: &Path,
    document_directory: &Path,
    job_id: JobId,
    generation: Generation,
    source_samples: u64,
    checkpoint: crate::parakeet_document::ResumeCoverageCheckpoint,
) -> Result<Option<SourceRange>, String> {
    if checkpoint.confirmed_segments == 0 && checkpoint.window_sequence == 0 {
        if checkpoint.covered_samples != 0 {
            return Err("StorageCorrupt: empty resume checkpoint has a sample offset".into());
        }
        return Ok(None);
    }
    let mut document = crate::parakeet_document::ParakeetDocumentStore::open_resume_coverage(
        document_directory,
        job_id,
        generation,
        source_samples,
    )?
    .ok_or_else(|| "StorageCorrupt: resume checkpoint document is absent".to_owned())?;
    let mut checkpoint_seen = false;
    loop {
        let page = document.read_page(128)?;
        if page.is_empty() {
            break;
        }
        for coverage in page {
            if coverage.sequence == checkpoint.window_sequence {
                if coverage.range.end_sample != checkpoint.covered_samples
                    || coverage.last_segment_sequence != checkpoint.confirmed_segments
                {
                    return Err(
                        "StorageCorrupt: resume checkpoint changed during validation".into(),
                    );
                }
                checkpoint_seen = true;
                break;
            }
        }
        if checkpoint_seen {
            break;
        }
    }
    if !checkpoint_seen {
        return Err("StorageCorrupt: selected resume checkpoint is missing".into());
    }
    if checkpoint.confirmed_segments == 0 {
        return Ok(None);
    }
    let mut document = crate::parakeet_document::ParakeetDocumentStore::open_resume_coverage(
        document_directory,
        job_id,
        generation,
        source_samples,
    )?
    .ok_or_else(|| "StorageCorrupt: resume checkpoint document is absent".to_owned())?;
    let mut archive = BufReader::new(
        File::open(segment_journal)
            .map_err(|error| format!("StorageCorrupt: resume segment journal: {error}"))?,
    );
    let mut next_sequence = 1_u64;
    let mut anchor = None;
    while next_sequence <= checkpoint.confirmed_segments {
        let remaining = checkpoint
            .confirmed_segments
            .checked_sub(next_sequence)
            .and_then(|count| count.checked_add(1))
            .ok_or_else(|| "StorageCorrupt: resume prefix sequence underflow".to_owned())?;
        let page_limit = usize::try_from(remaining.min(128))
            .map_err(|_| "StorageCorrupt: resume prefix page limit".to_owned())?;
        let page = document.read_document_page(next_sequence, page_limit)?;
        if page.is_empty() {
            return Err("StorageCorrupt: resume document prefix is incomplete".into());
        }
        for entry in page {
            let Some(line) = read_segment_journal_line(&mut archive)? else {
                return Err("StorageCorrupt: resume segment prefix is incomplete".into());
            };
            let record: SegmentRecord = serde_json::from_slice(&line)
                .map_err(|error| format!("StorageCorrupt: resume segment record: {error}"))?;
            if record.sequence != next_sequence
                || entry.segment.segment_id.0 != next_sequence
                || entry.segment.range != record.range
                || entry.segment.text != record.text
            {
                return Err("StorageCorrupt: resume archive and document prefixes differ".into());
            }
            anchor = Some(record.range);
            next_sequence = next_sequence
                .checked_add(1)
                .ok_or_else(|| "StorageCorrupt: resume segment sequence exhausted".to_owned())?;
        }
    }
    if checkpoint.window_sequence == 0
        || checkpoint.covered_samples == 0
        || checkpoint.covered_samples > source_samples
        || anchor.is_none_or(|range| {
            range.start_sample + (range.end_sample - range.start_sample) / 2
                >= checkpoint.covered_samples
        })
    {
        return Err("StorageCorrupt: resume checkpoint boundary is invalid".into());
    }
    Ok(anchor)
}

fn read_segment_journal_line(reader: &mut BufReader<File>) -> Result<Option<Vec<u8>>, String> {
    const MAX_SEGMENT_RECORD_BYTES: usize = 1_048_576;
    let mut line = Vec::new();
    loop {
        let available = reader
            .fill_buf()
            .map_err(|error| format!("StorageCorrupt: resume segment read: {error}"))?;
        if available.is_empty() {
            if line.is_empty() {
                return Ok(None);
            }
            if serde_json::from_slice::<SegmentRecord>(&line).is_ok() {
                return Ok(Some(line));
            }
            return Err("StorageCorrupt: truncated resume segment record".into());
        }
        let take = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(available.len(), |index| index + 1);
        if line.len().saturating_add(take) > MAX_SEGMENT_RECORD_BYTES {
            return Err("StorageCorrupt: resume segment record exceeds its bound".into());
        }
        let complete = available.get(take - 1) == Some(&b'\n');
        line.extend_from_slice(&available[..take]);
        reader.consume(take);
        if complete {
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            return Ok(Some(line));
        }
    }
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

fn write_replace_streamed(
    path: &Path,
    write_content: impl FnOnce(&mut std::io::BufWriter<File>) -> Result<(), String>,
) -> Result<(), String> {
    let temporary = path.with_extension(format!(
        "{}.tmp",
        path.extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("data")
    ));
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&temporary)
        .map_err(|error| format!("StorageUnavailable: {error}"))?;
    let mut writer = std::io::BufWriter::new(file);
    write_content(&mut writer)?;
    writer
        .flush()
        .map_err(|error| format!("StorageUnavailable: {error}"))?;
    let file = writer
        .into_inner()
        .map_err(|error| format!("StorageUnavailable: {}", error.error()))?;
    file.sync_all()
        .map_err(|error| format!("StorageUnavailable: {error}"))?;
    drop(file);
    fs::rename(temporary, path).map_err(|error| format!("StorageUnavailable: {error}"))
}

fn copy_replace_streamed(source: &Path, destination: &Path) -> Result<(), String> {
    write_replace_streamed(destination, |writer| {
        let mut source = File::open(source).map_err(|error| error.to_string())?;
        io::copy(&mut source, writer).map_err(|error| error.to_string())?;
        Ok(())
    })
}

fn repair_uncommitted_segment_tail(file: &mut File) -> Result<(), String> {
    const SCAN_BYTES: usize = 8 * 1024;
    let length = file
        .metadata()
        .map_err(|e| format!("StorageCorrupt: segment metadata: {e}"))?
        .len();
    if length == 0 {
        return Ok(());
    }
    file.seek(SeekFrom::End(-1))
        .map_err(|e| format!("StorageCorrupt: segment seek: {e}"))?;
    let mut last = [0_u8; 1];
    file.read_exact(&mut last)
        .map_err(|e| format!("StorageCorrupt: segment read: {e}"))?;
    if last[0] == b'\n' {
        return Ok(());
    }
    let mut end = length;
    let mut valid_len = 0_u64;
    let mut chunk = vec![0_u8; SCAN_BYTES];
    while end > 0 {
        let start = end.saturating_sub(SCAN_BYTES as u64);
        let count = (end - start) as usize;
        file.seek(SeekFrom::Start(start))
            .map_err(|e| format!("StorageCorrupt: segment tail scan: {e}"))?;
        file.read_exact(&mut chunk[..count])
            .map_err(|e| format!("StorageCorrupt: segment tail scan: {e}"))?;
        if let Some(index) = chunk[..count].iter().rposition(|byte| *byte == b'\n') {
            valid_len = start + index as u64 + 1;
            break;
        }
        end = start;
    }
    file.set_len(valid_len)
        .and_then(|()| file.sync_all())
        .map_err(|e| format!("StorageUnavailable: segment tail repair failed: {e}"))
}
fn write_new_synced(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("StorageUnavailable: {e}"))?;
    file.write_all(bytes).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())
}
fn write_replace_synced(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let started = Instant::now();
    let temporary = path.with_extension(format!(
        "{}.tmp",
        path.extension().and_then(|x| x.to_str()).unwrap_or("data")
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&temporary)
        .map_err(|e| e.to_string())?;
    file.write_all(bytes).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    drop(file);
    fs::rename(temporary, path).map_err(|e| e.to_string())?;
    crate::worker_ipc::record_r7_timing(
        "archive_write_replace",
        started.elapsed(),
        serde_json::json!({
            "file": path.file_name().and_then(|name| name.to_str()).unwrap_or("unknown"),
            "bytes": bytes.len(),
        }),
    );
    Ok(())
}
fn segmented_pcm_manifest(
    source: &crate::parakeet_audio::PcmPartStore,
) -> Result<([u8; 32], u64, CapturedSourceManifest), String> {
    let total_samples = source
        .total_samples()
        .map_err(|error| format!("AudioSource: {error}"))?;
    if total_samples == 0 {
        return Err("InvalidInput: captured PCM source is empty".into());
    }
    let index_path = source.base_path().with_extension("parts.idx");
    let index_before = fs::read(&index_path)
        .map_err(|error| format!("AudioSource: part index unavailable: {error}"))?;
    let mut digest = Sha256::new();
    digest.update(b"whisper-parakeet-pcm-parts-v1\0");
    digest.update(total_samples.to_le_bytes());
    digest.update((index_before.len() as u64).to_le_bytes());
    digest.update(&index_before);
    let mut parts = Vec::new();
    let mut offset = 0_u64;
    let mut covered = 0_u64;
    loop {
        let page = source
            .read_page(offset, 512)
            .map_err(|error| format!("AudioSource: part index: {error}"))?;
        if page.is_empty() {
            break;
        }
        for part in &page {
            if part.start_sample != covered || part.end_sample < part.start_sample {
                return Err("AudioSource: captured part index is not contiguous".into());
            }
            let expected_bytes = part
                .end_sample
                .saturating_sub(part.start_sample)
                .checked_mul(2)
                .and_then(|bytes| bytes.checked_add(44))
                .ok_or_else(|| "AudioSource: part byte count overflow".to_owned())?;
            let metadata = fs::metadata(&part.path).map_err(|error| {
                format!("AudioSource: part {} unavailable: {error}", part.number)
            })?;
            if metadata.len() != expected_bytes {
                return Err(format!(
                    "AudioSource: captured part {} has an invalid length",
                    part.number
                ));
            }
            digest.update(part.number.to_le_bytes());
            digest.update(part.start_sample.to_le_bytes());
            digest.update(part.end_sample.to_le_bytes());
            digest.update(metadata.len().to_le_bytes());
            let part_sha = sha256_file(&part.path)?;
            digest.update(part_sha);
            parts.push(CapturedPartReference {
                number: part.number,
                start_sample: part.start_sample,
                end_sample: part.end_sample,
                path: part.path.to_string_lossy().into_owned(),
                sha256: hex(&part_sha),
                bytes: metadata.len(),
            });
            covered = part.end_sample;
        }
        offset = offset
            .checked_add(page.len() as u64)
            .ok_or_else(|| "AudioSource: part index page overflow".to_owned())?;
    }
    if covered != total_samples
        || fs::read(&index_path).ok().as_deref() != Some(index_before.as_slice())
    {
        return Err(
            "SourceChanged: captured PCM parts changed while their identity was computed".into(),
        );
    }
    let identity: [u8; 32] = digest.finalize().into();
    let manifest = CapturedSourceManifest {
        version: 1,
        source_sha256: hex(&identity),
        source_samples: total_samples,
        index_path: index_path.to_string_lossy().into_owned(),
        index_sha256: hex(&Sha256::digest(&index_before)),
        parts,
    };
    Ok((identity, total_samples, manifest))
}

fn verified_artifact(directory: &Path, path: &Path) -> Result<ArtifactRecord, String> {
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
fn verify_artifact(directory: &Path, artifact: &ArtifactRecord) -> Result<(), String> {
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
fn verify_pcm_staging(
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
pub fn sha256_file(path: &Path) -> Result<[u8; 32], String> {
    let started = Instant::now();
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    let digest: [u8; 32] = hash.finalize().into();
    crate::worker_ipc::record_r7_timing(
        "archive_sha256",
        started.elapsed(),
        serde_json::json!({
            "file": path.file_name().and_then(|name| name.to_str()).unwrap_or("unknown"),
            "bytes": file.metadata().map(|meta| meta.len()).unwrap_or(0),
            "sha256": hex(&digest),
        }),
    );
    Ok(digest)
}

pub(crate) fn source_output_samples(path: &Path) -> Result<u64, String> {
    let started = Instant::now();
    let result = source_output_samples_inner(path);
    crate::worker_ipc::record_r7_timing(
        "archive_audio_probe",
        started.elapsed(),
        serde_json::json!({
            "file": path.file_name().and_then(|name| name.to_str()).unwrap_or("unknown"),
            "samples": result.as_ref().ok().copied(),
            "error": result.as_ref().err(),
        }),
    );
    result
}

fn source_output_samples_inner(path: &Path) -> Result<u64, String> {
    let file = File::open(path).map_err(|e| format!("SourceMissing: {e}"))?;
    let source = MediaSourceStream::new(Box::new(file), Default::default());
    let hint = Hint::new();
    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            source,
            &FormatOptions {
                enable_gapless: true,
                ..Default::default()
            },
            &MetadataOptions::default(),
        )
        .map_err(|e| format!("UnsupportedFormat: audio probe failed: {e}"))?;
    let track = probed
        .format
        .default_track()
        .ok_or_else(|| "UnsupportedFormat: audio has no track".to_owned())?;
    if track.codec_params.codec == CODEC_TYPE_MP3
        && (track.codec_params.channels.is_none()
            || track.codec_params.sample_rate.is_none()
            || track.codec_params.n_frames.is_none())
    {
        return Err("UnsupportedFormat: MP3 profile or duration metadata is incomplete".into());
    }
    if track.codec_params.codec == CODEC_TYPE_MP3 {
        let sample_rate = track.codec_params.sample_rate.unwrap_or_default();
        let channels = track.codec_params.channels;
        let stereo = Channels::FRONT_LEFT | Channels::FRONT_RIGHT;
        let supported_layout = channels == Some(Channels::FRONT_LEFT) || channels == Some(stereo);
        if !matches!(
            sample_rate,
            8_000 | 11_025 | 12_000 | 16_000 | 22_050 | 24_000 | 32_000 | 44_100 | 48_000
        ) || !supported_layout
        {
            return Err(
                "UnsupportedFormat: MP3 sample rate or channel layout is outside Q-07".into(),
            );
        }
    }
    let frames = track
        .codec_params
        .n_frames
        .ok_or_else(|| "UnsupportedFormat: audio duration is unavailable".to_owned())?;
    let input_rate = u64::from(
        track
            .codec_params
            .sample_rate
            .filter(|rate| *rate > 0)
            .ok_or_else(|| "UnsupportedFormat: audio sample rate is unavailable".to_owned())?,
    );
    if frames == 0 {
        return Err("UnsupportedFormat: audio contains no samples".into());
    }
    frames
        .saturating_sub(1)
        .checked_mul(16_000)
        .map(|scaled| scaled / input_rate + 1)
        .ok_or_else(|| "UnsupportedFormat: audio duration exceeds supported range".into())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
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

fn is_r5_2_vad_recipe_destination(path: &Path) -> bool {
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
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

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

        let audio_first =
            ArchiveStore::read_unified_audio_parts_page(&root, job_id, generation, 0, 0, 512)
                .unwrap();
        assert_eq!(audio_first.entries.len(), 128);
        assert_eq!(audio_first.entries.first().unwrap().number, 1);
        assert_eq!(audio_first.entries.last().unwrap().number, 128);
        assert_eq!(audio_first.next_offset, Some(128));
        let audio_last = ArchiveStore::read_unified_audio_parts_page(
            &root,
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

        fs::remove_dir_all(root).unwrap();
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

        let snapshot = l04_campaign_snapshot(&root).unwrap();
        assert_eq!(snapshot.len(), 1);
        assert_eq!(snapshot[0]["path"], "passage-1.pcm");
        assert_eq!(snapshot[0]["bytes"], 11);

        lock.unlock().unwrap();
        drop(lock);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn r7_copy_fallback_recovers_between_exports_without_clobbering_a_byte_identical_collision() {
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
        let directory = generation_path(&root, job_id, generation);
        fs::create_dir_all(&directory).unwrap();
        let text = b"R7 transcript";
        let srt = b"1\n00:00:00,000 --> 00:00:01,000\nR7 transcript\n";
        fs::write(directory.join("transcript.txt"), text).unwrap();
        fs::write(directory.join("transcript.srt"), srt).unwrap();
        fs::write(directory.join("export.txt.pending"), text).unwrap();
        fs::write(directory.join("export.srt.pending"), srt).unwrap();
        let artifact = |path: &str, content: &[u8]| ArtifactRecord {
            path: path.to_owned(),
            sha256: hex(&Sha256::digest(content)),
            bytes: content.len() as u64,
        };
        let manifest = CompleteRecord {
            version: 1,
            job_id,
            generation,
            source_sha256: "77".repeat(32),
            segments: 1,
            text: artifact("transcript.txt", text),
            srt: artifact("transcript.srt", srt),
        };
        let previous_job_id = JobId(0x7710);
        let previous_generation = Generation::first();
        let previous_directory = generation_path(&root, previous_job_id, previous_generation);
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
            text: artifact("transcript.txt", previous_text),
            srt: artifact("transcript.srt", previous_srt),
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
            &root,
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
            ArchiveStore::published_imports(&root, &manifest.source_sha256).unwrap();
        assert_eq!(prior_publications.len(), 1);
        assert_eq!(prior_publications[0].job_id, previous_job_id);
        fs::write(
            directory.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
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
            &root,
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
        let exports = root.join("exports");
        fs::create_dir_all(&exports).unwrap();
        let no_hard_links = |_: &Path, _: &Path| {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "test filesystem does not support hard links",
            ))
        };
        let calls = std::cell::Cell::new(0);
        let third_party = exports.join("r7-audio.srt");
        let third_party_bytes = srt.to_vec();
        let fail_links_and_collide_after_txt = |_: &Path, _target: &Path| {
            let call = calls.get() + 1;
            calls.set(call);
            if call == 4 {
                fs::write(&third_party, &third_party_bytes)?;
            }
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "test filesystem does not support hard links",
            ))
        };
        let first = install_publication_exports_with(
            &root,
            &directory,
            &intent,
            &fail_links_and_collide_after_txt,
        );
        let error = first.unwrap_err();
        assert!(error.contains("export name collision for srt"), "{error}");
        assert_eq!(fs::read(exports.join("r7-audio.txt")).unwrap(), text);
        let third_party_identity = persisted_file_identity(&third_party).unwrap();
        assert_eq!(fs::read(&third_party).unwrap(), third_party_bytes);
        assert_eq!(
            persisted_file_identity(&third_party).unwrap(),
            third_party_identity
        );
        assert!(!directory.join("CURRENT").exists());

        // Inject a process interruption immediately after the exclusive copy
        // creates the final SRT, before its file identity can be persisted.
        fs::remove_file(&third_party).unwrap();
        let interrupted_copy = |extension: &str, _path: &Path| {
            if extension == "srt" {
                Err("test process interruption after create_new".into())
            } else {
                Ok(())
            }
        };
        let create_new_cut = install_publication_exports_with_copy_hook(
            &root,
            &directory,
            &intent,
            &no_hard_links,
            &interrupted_copy,
        )
        .unwrap_err();
        assert!(create_new_cut.contains("test process interruption"));
        assert_eq!(fs::read(&third_party).unwrap(), third_party_bytes);
        let unrecorded_identity = persisted_file_identity(&third_party).unwrap();
        let recovery_error =
            install_publication_exports_with(&root, &directory, &intent, &no_hard_links)
                .unwrap_err();
        assert!(
            recovery_error.contains("export name collision for srt"),
            "{recovery_error}"
        );
        assert_eq!(fs::read(&third_party).unwrap(), third_party_bytes);
        assert_eq!(
            persisted_file_identity(&third_party).unwrap(),
            unrecorded_identity
        );
        assert!(!directory.join("CURRENT").exists());
        assert_eq!(
            ArchiveStore::publication_reconfirmation_required(&root, job_id, generation).unwrap(),
            Some("r7-audio".into())
        );
        let new_basename =
            ArchiveStore::suggest_duplicate_export_basename(&root, &previous.source_path).unwrap();
        assert_eq!(new_basename, "r7-audio (2)");
        ArchiveStore::reconfirm_interrupted_publication(
            &root,
            &previous,
            &intent.source_sha256,
            intent.source_samples,
            &prior_publications,
            DuplicateImportChoice::Replace,
            "r7-audio",
            &new_basename,
        )
        .unwrap();
        let confirmed_intent: PublicationIntent = serde_json::from_slice(
            &fs::read(publication_reservation_path(&root, job_id, generation)).unwrap(),
        )
        .unwrap();
        assert_eq!(confirmed_intent.export_basename, new_basename);
        install_publication_exports_with(&root, &directory, &confirmed_intent, &no_hard_links)
            .unwrap();
        assert_eq!(fs::read(exports.join("r7-audio (2).txt")).unwrap(), text);
        assert_eq!(fs::read(exports.join("r7-audio (2).srt")).unwrap(), srt);
        assert_eq!(fs::read(&third_party).unwrap(), third_party_bytes);
        assert_eq!(
            persisted_file_identity(&third_party).unwrap(),
            unrecorded_identity
        );
        assert_eq!(fs::read(directory.join("transcript.txt")).unwrap(), text);
        assert_eq!(fs::read(directory.join("transcript.srt")).unwrap(), srt);
        assert_eq!(
            ArchiveStore::published_imports(&root, &intent.source_sha256).unwrap(),
            prior_publications,
            "the prior publication remains active until the recovered generation commits"
        );
        assert!(!directory.join("CURRENT").exists());

        // Repeating export installation is idempotent before the commit point.
        install_publication_exports_with(&root, &directory, &confirmed_intent, &no_hard_links)
            .unwrap();
        assert_eq!(fs::read(exports.join("r7-audio (2).srt")).unwrap(), srt);
        let mut segments = serde_json::to_vec(&SegmentRecord {
            sequence: 1,
            range: SourceRange::new(0, 16_000, 16_000).unwrap(),
            text: "R7 transcript".into(),
        })
        .unwrap();
        segments.push(b'\n');
        fs::write(directory.join("segments.jsonl"), segments).unwrap();
        fs::write(directory.join("CURRENT"), b"manifest.json\n").unwrap();
        let index = apply_publication_intent(
            read_publication_index(&root).unwrap(),
            job_id,
            generation,
            &confirmed_intent,
        );
        write_publication_index(&root, &index).unwrap();
        reconcile_replaced_exports(&root, &index).unwrap();
        let active = ArchiveStore::published_imports(&root, &intent.source_sha256).unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(
            (active[0].job_id, active[0].generation),
            (job_id, generation)
        );
        assert!(directory.join("CURRENT").is_file());
        assert_eq!(fs::read(exports.join("r7-audio (2).txt")).unwrap(), text);
        assert_eq!(fs::read(exports.join("r7-audio (2).srt")).unwrap(), srt);
        assert_eq!(fs::read(&third_party).unwrap(), third_party_bytes);
        assert_eq!(
            persisted_file_identity(&third_party).unwrap(),
            unrecorded_identity
        );
        fs::remove_dir_all(root).unwrap();
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

        let first = ArchiveStore::scan(&root).unwrap();
        let second = ArchiveStore::scan(&root).unwrap();
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

        let mut reopened = QueueStore::open(&root).unwrap();
        reopened.remove(request.job_id, request.generation).unwrap();
        drop(reopened);
        let first = crate::recovery::scan(&root).unwrap();
        let second = crate::recovery::scan(&root).unwrap();
        assert_eq!(
            first, second,
            "recovery is stable after publication cleanup"
        );
        assert_eq!(first.queue.len(), 1);
        assert_eq!(first.queue[0].request.job_id, other.job_id);
        assert!(ArchiveStore::scan(&root).unwrap().iter().any(|entry| {
            entry.job_id == request.job_id
                && entry.generation == request.generation
                && entry.complete
        }));
        assert!(
            !QueueStore::open(&root)
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
        let pending = PendingRecord {
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
        };
        assert_eq!(initialize_pending(&target, &pending).unwrap(), 1);
        let prefix = vec![
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
        ];
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

    #[test]
    fn l04_campaign_gate_rejects_escape_and_duplicate_trigger() {
        if option_env!("WHISPER_L04_CAMPAIGN_BUILD").is_none() {
            assert!(l04_campaign_gate("coverage.after_sync", 1, 1, 0).is_ok());
            return;
        }
        if let Ok(mode) = std::env::var("WHISPER_L04_GATE_TEST_CHILD") {
            if mode == "outside" {
                assert!(
                    l04_campaign_gate("coverage.after_sync", 1, 1, 0)
                        .unwrap_err()
                        .contains("direct child")
                );
            } else if mode == "zero" {
                assert!(l04_campaign_gate("recovery.before_scan_live", 1, 0, 0).is_ok());
            } else {
                let control_path = std::env::var_os("WHISPER_L04_CAMPAIGN_RUN")
                    .map(PathBuf::from)
                    .unwrap()
                    .join("run.json");
                let original_control = fs::read(&control_path).unwrap();
                let archive = control_path.parent().unwrap().join("archive");
                assert!(l04_campaign_assert_archive(&archive).is_ok());
                assert!(l04_campaign_assert_archive(control_path.parent().unwrap()).is_err());
                fs::write(
                    &control_path,
                    br#"{"schema":"whisper-l04-fault/1","hook":"coverage.after_sync","job_id":"1","generation":1,"instance_id":0,"binary":{"sha256":"invalid"}}"#,
                )
                .unwrap();
                assert!(
                    l04_campaign_gate("coverage.before_record", 1, 1, 0)
                        .unwrap_err()
                        .contains("missing trigger count")
                );
                fs::write(
                    &control_path,
                    br#"{"schema":"whisper-l04-fault/1","hook":"coverage.after_sync","job_id":"1","generation":1,"instance_id":0,"trigger_count":1,"binary":{"sha256":"invalid"}}"#,
                )
                .unwrap();
                assert!(
                    l04_campaign_gate("coverage.before_record", 1, 1, 0)
                        .unwrap_err()
                        .contains("binary hash mismatch")
                );
                fs::write(&control_path, original_control).unwrap();
                assert!(l04_campaign_gate("coverage.before_record", 1, 1, 0).is_ok());
                assert!(l04_campaign_gate("coverage.after_sync", 1, 1, 0).is_ok());
                assert!(
                    l04_campaign_gate("coverage.after_sync", 1, 1, 0)
                        .unwrap_err()
                        .contains("duplicate trigger")
                );
                assert!(
                    l04_campaign_gate("coverage.after_sync", 2, 1, 0)
                        .unwrap_err()
                        .contains("identity mismatch")
                );
                fs::write(
                    std::env::var_os("WHISPER_L04_CAMPAIGN_RUN")
                        .map(PathBuf::from)
                        .unwrap()
                        .join("run.json"),
                    br#"{"schema":"whisper-l04-fault/1","hook":"unknown","job_id":"1","generation":1,"instance_id":0}"#,
                )
                .unwrap();
                assert!(
                    l04_campaign_gate("coverage.after_sync", 1, 1, 0)
                        .unwrap_err()
                        .contains("unknown hook")
                );
            }
            return;
        }
        let runs = Path::new(r"C:\WhisperLive\L04-campaign\runs");
        fs::create_dir_all(runs).unwrap();
        let binary = std::env::current_exe().unwrap();
        let binary_sha256 = format!("{:x}", Sha256::digest(fs::read(&binary).unwrap()));
        let test_name = "archive::tests::l04_campaign_gate_rejects_escape_and_duplicate_trigger";
        let outside = std::process::Command::new(&binary)
            .args(["--exact", test_name])
            .env("WHISPER_L04_GATE_TEST_CHILD", "outside")
            .env("WHISPER_L04_CAMPAIGN_RUN", std::env::temp_dir())
            .status()
            .unwrap();
        assert!(outside.success());
        let run = runs.join(format!(
            "gate-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&run).unwrap();
        fs::create_dir(run.join("archive")).unwrap();
        fs::write(
            run.join("run.json"),
            serde_json::to_vec(&serde_json::json!({
                "schema":"whisper-l04-fault/1", "hook":"coverage.after_sync",
                "job_id":"1", "generation":1, "instance_id":0, "trigger_count":1,
                "binary":{"sha256":binary_sha256},
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(run.join("decision.txt"), b"release\n").unwrap();
        let inside = std::process::Command::new(binary)
            .args(["--exact", test_name])
            .env("WHISPER_L04_GATE_TEST_CHILD", "inside")
            .env("WHISPER_L04_CAMPAIGN_RUN", &run)
            .status()
            .unwrap();
        assert!(inside.success());
        let marker: serde_json::Value =
            serde_json::from_slice(&fs::read(run.join("entered.json")).unwrap()).unwrap();
        assert_eq!(marker["binary_sha256"], binary_sha256);
        assert_eq!(marker["preceding_operation_result"], "returned_ok");
        assert!(marker["entered_unix_ms"].as_u64().is_some());
        let receipt: serde_json::Value =
            serde_json::from_slice(&fs::read(run.join("decision-observed.json")).unwrap()).unwrap();
        assert_eq!(receipt["action"], "release");
        assert!(receipt["observed_unix_ms"].as_u64().is_some());
        fs::remove_dir_all(run).unwrap();
        let zero = runs.join(format!("gate-zero-{}", std::process::id()));
        fs::create_dir(&zero).unwrap();
        fs::write(
            zero.join("run.json"),
            serde_json::to_vec(&serde_json::json!({
                "schema":"whisper-l04-fault/1", "hook":"recovery.before_scan_live",
                "job_id":"1", "generation":0, "instance_id":0, "trigger_count":1,
                "binary":{"sha256":binary_sha256},
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(zero.join("decision.txt"), b"release\n").unwrap();
        let zero_result = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", test_name])
            .env("WHISPER_L04_GATE_TEST_CHILD", "zero")
            .env("WHISPER_L04_CAMPAIGN_RUN", &zero)
            .status()
            .unwrap();
        assert!(zero_result.success());
        fs::remove_dir_all(zero).unwrap();
    }

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
}
