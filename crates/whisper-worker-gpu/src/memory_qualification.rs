use serde_json::Value;
use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    path::PathBuf,
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

static REPORT: OnceLock<Option<Mutex<BufWriter<File>>>> = OnceLock::new();

pub fn record(stage: &str, details: Value) {
    let report = REPORT.get_or_init(|| {
        let path = std::env::var_os("WHISPER_L01_MEMORY_TRACE").map(PathBuf::from)?;
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .ok()
            .map(|file| Mutex::new(BufWriter::new(file)))
    });
    let Some(report) = report else { return };
    let mut details = details;
    let mut process_memory = if matches!(
        stage,
        "worker.input.reader.created"
            | "model_context.load.complete"
            | "codec.ready"
            | "native.state.create.begin"
            | "native.inference.complete"
            | "native.state.destroyed"
            | "worker.finalized"
    ) {
        Some(process_memory_sample())
    } else {
        None
    };
    let elapsed_unix_us = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_micros());
    if let Some(sample) = process_memory.as_mut() {
        if let Some(sample) = sample.as_object_mut() {
            sample.insert(
                "sampled_at_unix_us".into(),
                serde_json::json!(elapsed_unix_us),
            );
        }
        if let Some(details) = details.as_object_mut() {
            details.insert("process_memory".into(), sample.clone());
        }
    }
    if let Ok(mut output) = report.lock() {
        let record = serde_json::json!({
            "elapsed_unix_us": elapsed_unix_us,
            "stage": stage,
            "details": details,
        });
        let _ = serde_json::to_writer(&mut *output, &record);
        let _ = output.write_all(b"\n");
        let _ = output.flush();
    }
}

fn process_memory_sample() -> Value {
    use std::process::{Command, Stdio};

    let pid = std::process::id();
    let script = format!(
        "$p=Get-Process -Id {pid} -ErrorAction Stop; [ordered]@{{ private_bytes=$p.PrivateMemorySize64; working_set_bytes=$p.WorkingSet64; peak_working_set_bytes=$p.PeakWorkingSet64 }} | ConvertTo-Json -Compress"
    );
    match Command::new("powershell.exe")
        .args(["-NoProfile", "-Command", &script])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
    {
        Ok(output) if output.status.success() => serde_json::from_slice(&output.stdout)
            .unwrap_or_else(|error| {
                serde_json::json!({
                    "error": format!("invalid process-counter JSON: {error}"),
                    "stdout": String::from_utf8_lossy(&output.stdout),
                })
            }),
        Ok(output) => serde_json::json!({
            "error": format!("process-counter PowerShell exited with {}", output.status),
            "stderr": String::from_utf8_lossy(&output.stderr),
        }),
        Err(error) => serde_json::json!({
            "error": format!("cannot query process counters: {error}"),
        }),
    }
}
