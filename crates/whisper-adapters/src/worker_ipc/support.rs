use std::sync::atomic::AtomicU64;
#[cfg(feature = "l01-memory-qualification")]
use std::{
    fs::{File, OpenOptions},
    io::Write,
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

pub(super) static NEXT_IMPORT_JOB_ID: AtomicU64 = AtomicU64::new(1);
pub(super) static NEXT_INSTANCE: AtomicU64 = AtomicU64::new(1);

#[cfg(feature = "l01-memory-qualification")]
static MEMORY_REPORT: OnceLock<Option<Mutex<std::io::BufWriter<File>>>> = OnceLock::new();

#[cfg(feature = "l01-memory-qualification")]
pub(super) fn memory_trace(stage: &str, details: serde_json::Value) {
    let report = MEMORY_REPORT.get_or_init(|| {
        let path = std::env::var_os("WHISPER_L01_ADAPTER_MEMORY_TRACE")?;
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .ok()
            .map(|file| Mutex::new(std::io::BufWriter::new(file)))
    });
    let Some(report) = report else { return };
    let elapsed_unix_us = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_micros());
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
