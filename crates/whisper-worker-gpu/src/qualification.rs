#[cfg(feature = "recette-hooks")]
use sha2::{Digest, Sha256};

#[cfg(feature = "recette-hooks")]
static L04_CAMPAIGN_OCCURRENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

#[cfg(feature = "recette-hooks")]
pub(crate) fn l04_campaign_worker_result<T, E: std::fmt::Display>(
    hook: &str,
    job_id: u128,
    generation: u64,
    instance_id: u64,
    result: &Result<T, E>,
) -> Result<(), String> {
    if option_env!("WHISPER_L04_CAMPAIGN_BUILD").is_none() {
        return Ok(());
    }
    let Some(directory) = std::env::var_os("WHISPER_L04_CAMPAIGN_RUN") else {
        return Ok(());
    };
    let run = std::path::PathBuf::from(directory)
        .canonicalize()
        .map_err(|error| format!("CampaignGuard: result run: {error}"))?;
    let allowed = std::path::Path::new(r"C:\WhisperLive\L04-campaign\runs")
        .canonicalize()
        .map_err(|error| format!("CampaignGuard: result root: {error}"))?;
    if run.parent() != Some(allowed.as_path()) {
        return Err("CampaignGuard: result run outside campaign".into());
    }
    let control: serde_json::Value = serde_json::from_slice(
        &std::fs::read(run.join("run.json"))
            .map_err(|error| format!("CampaignGuard: result control: {error}"))?,
    )
    .map_err(|error| format!("CampaignGuard: result JSON: {error}"))?;
    if control.get("hook").and_then(|value| value.as_str()) != Some(hook) {
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
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(run.join("operation-result.json"))
        .map_err(|error| format!("CampaignGuard: result marker: {error}"))?;
    serde_json::to_writer(
        &mut file,
        &serde_json::json!({
            "hook": hook, "job_id": job_id.to_string(), "generation": generation,
            "instance_id": instance_id, "trigger_count": target,
            "pid": std::process::id(), "syscall_result": match result {
                Ok(_) => "ok".to_owned(), Err(error) => format!("error: {error}"),
            },
            "completed_unix_ms": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).map_or(0, |duration| duration.as_millis()),
        }),
    )
    .map_err(|error| format!("CampaignGuard: result write: {error}"))?;
    file.sync_all()
        .map_err(|error| format!("CampaignGuard: result sync: {error}"))
}

#[cfg(not(feature = "recette-hooks"))]
pub(crate) fn l04_campaign_worker_result<T, E: std::fmt::Display>(
    _hook: &str,
    _job_id: u128,
    _generation: u64,
    _instance_id: u64,
    _result: &Result<T, E>,
) -> Result<(), String> {
    Ok(())
}

#[cfg(feature = "recette-hooks")]
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
    let allowed = std::path::Path::new(r"C:\WhisperLive\L04-campaign\runs")
        .canonicalize()
        .map_err(|error| format!("CampaignGuard: runs root: {error}"))?;
    let actual = std::path::PathBuf::from(directory)
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
        &std::fs::read(run_file).map_err(|error| format!("CampaignGuard: run.json: {error}"))?,
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
    if selected != hook {
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
            std::fs::read(&binary)
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
    let occurrence = L04_CAMPAIGN_OCCURRENCE.fetch_add(1, std::sync::atomic::Ordering::AcqRel) + 1;
    if occurrence < target {
        return Ok(());
    }
    if occurrence > target {
        return Err("CampaignGuard: duplicate trigger count".into());
    }
    if matches!(
        hook,
        "worker.before_encode" | "worker.before_finish_encode" | "worker.before_stopped"
    ) {
        let mut start = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(actual.join("operation-start.json"))
            .map_err(|error| format!("CampaignGuard: operation start: {error}"))?;
        serde_json::to_writer(&mut start, &serde_json::json!({
            "hook": hook, "job_id": job_id.to_string(), "generation": generation,
            "instance_id": instance_id, "trigger_count": occurrence,
            "pid": std::process::id(), "binary_sha256": binary_sha256,
            "started_unix_ms": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).map_or(0, |duration| duration.as_millis()),
        })).map_err(|error| format!("CampaignGuard: operation start write: {error}"))?;
        start
            .sync_all()
            .map_err(|error| format!("CampaignGuard: operation start sync: {error}"))?;
    }
    let mut marker = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(actual.join("entered.json"))
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
    std::io::Write::flush(&mut marker)
        .map_err(|error| format!("CampaignGuard: marker flush: {error}"))?;
    marker
        .sync_all()
        .map_err(|error| format!("CampaignGuard: marker sync: {error}"))?;
    drop(marker);
    for _ in 0..1200 {
        let decision = actual.join("decision.txt");
        if decision.exists() {
            let decision_file = decision
                .canonicalize()
                .map_err(|error| format!("CampaignGuard: decision path: {error}"))?;
            if decision_file.parent() != Some(actual.as_path()) {
                return Err("CampaignGuard: decision leaves the run directory".into());
            }
            let action = std::fs::read_to_string(decision_file)
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
            let mut receipt = std::fs::OpenOptions::new()
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
pub(crate) fn l04_campaign_gate(
    _hook: &str,
    _job_id: u128,
    _generation: u64,
    _instance_id: u64,
) -> Result<(), String> {
    Ok(())
}
