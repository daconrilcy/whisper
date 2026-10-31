use super::Path;
#[cfg(feature = "recette-hooks")]
use super::{Digest, File, OpenOptions, PathBuf, Read, Sha256, fs};

#[cfg(feature = "recette-hooks")]
static CAMPAIGN_HOOK_OCCURRENCE: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(0);

#[cfg(feature = "recette-hooks")]
fn campaign_after_hook(before: &str) -> Option<&'static str> {
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

#[cfg(feature = "recette-hooks")]
fn campaign_is_after_hook(hook: &str) -> bool {
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

#[cfg(feature = "recette-hooks")]
pub(crate) fn assert_campaign_archive(root: &Path) -> Result<(), String> {
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

#[cfg(not(feature = "recette-hooks"))]
pub(crate) fn assert_campaign_archive(_root: &Path) -> Result<(), String> {
    Ok(())
}

#[cfg(feature = "recette-hooks")]
fn write_campaign_marker(path: &Path, value: &serde_json::Value) -> Result<(), String> {
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

#[cfg(feature = "recette-hooks")]
pub(super) fn campaign_archive_snapshot(run: &Path) -> Result<Vec<serde_json::Value>, String> {
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

#[cfg(feature = "recette-hooks")]
pub(crate) fn record_campaign_operation_result<T, E: std::fmt::Display>(
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
    if selected != Some(before) && selected != campaign_after_hook(before) {
        return Ok(());
    }
    let target = control
        .get("trigger_count")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| "CampaignGuard: missing trigger count".to_owned())?;
    if target == 0 || CAMPAIGN_HOOK_OCCURRENCE.load(std::sync::atomic::Ordering::Acquire) != target
    {
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
    write_campaign_marker(
        &run.join("operation-result.json"),
        &serde_json::json!({
            "before_hook": before, "after_hook": campaign_after_hook(before),
            "job_id": job_id.to_string(), "generation": generation,
            "instance_id": instance_id, "pid": std::process::id(),
            "completed_unix_ms": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).map_or(0, |duration| duration.as_millis()),
            "syscall_result": if result.is_ok() { "ok" } else { "error" },
            "error": result.as_ref().err().map(ToString::to_string),
            "artifact": artifact,
            "archive_files": campaign_archive_snapshot(&run)?,
        }),
    )
}

#[cfg(not(feature = "recette-hooks"))]
pub(crate) fn record_campaign_operation_result<T, E: std::fmt::Display>(
    _hook: &str,
    _job_id: u128,
    _generation: u64,
    _instance_id: u64,
    _result: &Result<T, E>,
    _path: Option<&Path>,
) -> Result<(), String> {
    Ok(())
}

/// A build-local, one-shot barrier for explicitly enabled L04 fault campaigns.
#[cfg(feature = "recette-hooks")]
pub(crate) fn campaign_gate(
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
    let paired_start = campaign_after_hook(hook) == Some(selected);
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
    let occurrence = if paired_start || (selected == hook && !campaign_is_after_hook(hook)) {
        CAMPAIGN_HOOK_OCCURRENCE.fetch_add(1, std::sync::atomic::Ordering::AcqRel) + 1
    } else {
        CAMPAIGN_HOOK_OCCURRENCE.load(std::sync::atomic::Ordering::Acquire)
    };
    if occurrence < target {
        return Ok(());
    }
    if occurrence > target {
        return Err("CampaignGuard: duplicate trigger count".into());
    }
    if paired_start || (selected == hook && campaign_after_hook(hook).is_some()) {
        write_campaign_marker(
            &actual.join("operation-start.json"),
            &serde_json::json!({
                "before_hook": hook, "after_hook": campaign_after_hook(hook).unwrap_or(selected),
                "job_id": job_id.to_string(), "generation": generation,
                "instance_id": instance_id, "pid": std::process::id(),
                "started_unix_ms": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH).map_or(0, |duration| duration.as_millis()),
                "binary_sha256": binary_sha256,
                "trigger_count": occurrence,
                "archive_files": campaign_archive_snapshot(&actual)?,
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

#[cfg(not(feature = "recette-hooks"))]
pub(crate) fn campaign_gate(
    _hook: &str,
    _job_id: u128,
    _generation: u64,
    _instance_id: u64,
) -> Result<(), String> {
    Ok(())
}
