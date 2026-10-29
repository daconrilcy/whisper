use std::{
    cell::RefCell,
    fs,
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

#[derive(Clone)]
struct TimingContext {
    operation: String,
    operation_id: String,
    destination: String,
}

thread_local! {
    static TIMING_CONTEXT: RefCell<Option<TimingContext>> = const { RefCell::new(None) };
}

pub(crate) struct TimingContextGuard(Option<TimingContext>);

impl TimingContextGuard {
    pub(crate) fn enter(operation: &str, operation_id: String, destination: &Path) -> Self {
        let context = TimingContext {
            operation: operation.to_owned(),
            operation_id,
            destination: destination.to_string_lossy().into_owned(),
        };
        Self(TIMING_CONTEXT.with(|current| current.replace(Some(context))))
    }
}

impl Drop for TimingContextGuard {
    fn drop(&mut self) {
        let previous = self.0.take();
        TIMING_CONTEXT.with(|current| {
            current.replace(previous);
        });
    }
}

pub(crate) fn record_stage_timing(stage: &str, elapsed: Duration, details: serde_json::Value) {
    if std::env::var_os("PARAKEET_R7_TIMING_ACTIVE").as_deref() != Some(std::ffi::OsStr::new("1")) {
        return;
    }
    let Some(path) = std::env::var_os("PARAKEET_R7_TIMING_LOG").map(PathBuf::from) else {
        eprintln!("R7 timing skipped: log path is absent");
        return;
    };
    let Some(root) = std::env::var_os("PARAKEET_R7_EXEC_ROOT").map(PathBuf::from) else {
        eprintln!("R7 timing skipped: execution root is absent");
        return;
    };
    let Ok(root) = root.canonicalize() else {
        eprintln!("R7 timing skipped: execution root is unavailable");
        return;
    };
    let Some(parent) = path.parent() else {
        eprintln!("R7 timing skipped: log path has no parent");
        return;
    };
    let Ok(parent) = parent.canonicalize() else {
        eprintln!("R7 timing skipped: log parent is unavailable");
        return;
    };
    if !parent.starts_with(&root) {
        eprintln!("R7 timing skipped: log path escaped R7");
        return;
    }
    static PROCESS_START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    let at_ms = PROCESS_START
        .get_or_init(Instant::now)
        .elapsed()
        .as_millis();
    let mut file = match fs::OpenOptions::new().create(true).append(true).open(path) {
        Ok(file) => file,
        Err(error) => {
            eprintln!("R7 timing log open failed: {error}");
            return;
        }
    };
    let context = TIMING_CONTEXT.with(|current| current.borrow().clone());
    let row = serde_json::json!({
        "process": "adapter",
        "origin": if context.is_some() { "product" } else { "unscoped" },
        "pid": std::process::id(),
        "at_ms": at_ms,
        "stage": stage,
        "elapsed_ms": elapsed.as_secs_f64() * 1000.0,
        "operation": context.as_ref().map(|value| value.operation.as_str()),
        "operation_id": context.as_ref().map(|value| value.operation_id.as_str()),
        "destination": context.as_ref().map(|value| value.destination.as_str()),
        "details": details,
    });
    if serde_json::to_writer(&mut file, &row).is_err() || file.write_all(b"\n").is_err() {
        eprintln!("R7 timing log write failed");
    }
}
