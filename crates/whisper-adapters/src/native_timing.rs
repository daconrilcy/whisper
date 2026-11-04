use std::{
    fs::{File, OpenOptions},
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use serde_json::Value;

static TRACE: OnceLock<Mutex<Option<TraceWriter>>> = OnceLock::new();

struct TraceWriter {
    output: BufWriter<File>,
    origin: Instant,
}

pub(crate) struct Phase {
    name: &'static str,
    details: Value,
    started: Instant,
}

pub(crate) fn phase(name: &'static str, details: Value) -> Option<Phase> {
    if !enabled() {
        return None;
    }
    record(&format!("{name}.begin"), details.clone());
    Some(Phase {
        name,
        details,
        started: Instant::now(),
    })
}

impl Drop for Phase {
    fn drop(&mut self) {
        record(
            &format!("{}.end", self.name),
            serde_json::json!({
                "duration_ms": self.started.elapsed().as_secs_f64() * 1000.0,
                "context": self.details.clone(),
            }),
        );
    }
}

pub(crate) fn record(name: &str, details: Value) {
    if !enabled() {
        return;
    }
    let Some(mut slot) = trace_lock() else {
        return;
    };
    if slot.is_none() {
        *slot = open_trace().ok();
    }
    let result = {
        let Some(trace) = slot.as_mut() else {
            return;
        };
        let result = write_record(
            &mut trace.output,
            trace.origin,
            name,
            details,
            std::process::id(),
            &format!("{:?}", std::thread::current().id()),
        );
        if result.is_ok() && name.ends_with(".end") {
            let _ = trace.output.flush();
        }
        result
    };
    if result.is_err() {
        *slot = None;
    }
}

fn write_record(
    output: &mut impl Write,
    origin: Instant,
    name: &str,
    details: Value,
    pid: u32,
    thread: &str,
) -> io::Result<()> {
    let record = serde_json::json!({
        "event": name,
        "monotonic_ms": origin.elapsed().as_secs_f64() * 1000.0,
        "pid": pid,
        "thread": thread,
        "details": details,
    });
    serde_json::to_writer(&mut *output, &record).map_err(io::Error::other)?;
    output.write_all(b"\n")
}

fn enabled() -> bool {
    trace_enabled(std::env::var_os("WHISPER_DIAG_BANC_TRACE").as_deref())
}

fn trace_enabled(value: Option<&std::ffi::OsStr>) -> bool {
    value == Some(std::ffi::OsStr::new("1"))
}

fn trace_lock() -> Option<std::sync::MutexGuard<'static, Option<TraceWriter>>> {
    TRACE.get_or_init(|| Mutex::new(None)).lock().ok()
}

fn open_trace() -> io::Result<TraceWriter> {
    let root = PathBuf::from(
        std::env::var_os("WHISPER_L04_CAMPAIGN_RUN")
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "guarded run root missing"))?,
    )
    .canonicalize()?;
    if !root.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "guarded run root is not a directory",
        ));
    }
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(io::Error::other)?
        .as_nanos();
    let path = trace_path(&root, std::process::id(), nanos)?;
    let output = OpenOptions::new().write(true).create_new(true).open(path)?;
    Ok(TraceWriter {
        output: BufWriter::new(output),
        origin: Instant::now(),
    })
}

fn trace_path(root: &Path, pid: u32, nonce: u128) -> io::Result<PathBuf> {
    let path = root.join(format!("native-product-timing-{pid}-{nonce}.jsonl"));
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "trace path has no parent"))?
        .canonicalize()?;
    if !parent.starts_with(root) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "trace path escaped guarded run root",
        ));
    }
    Ok(path)
}

#[cfg(test)]
mod tests;
