use std::{env, io::Write, path::PathBuf, time::Duration};

pub(crate) fn record_r7_timing(stage: &str, elapsed: Duration, details: serde_json::Value) {
    if env::var_os("PARAKEET_R7_TIMING_ACTIVE").as_deref() != Some(std::ffi::OsStr::new("1")) {
        return;
    }
    let (Some(root), Some(path)) = (
        env::var_os("PARAKEET_R7_EXEC_ROOT").map(PathBuf::from),
        env::var_os("PARAKEET_R7_TIMING_LOG").map(PathBuf::from),
    ) else {
        eprintln!("R7 worker timing skipped: root or log path missing");
        return;
    };
    let (Ok(root), Some(parent)) = (root.canonicalize(), path.parent()) else {
        eprintln!("R7 worker timing skipped: root or log parent unavailable");
        return;
    };
    let Ok(parent) = parent.canonicalize() else {
        eprintln!("R7 worker timing skipped: log parent unavailable");
        return;
    };
    if !parent.starts_with(&root) {
        eprintln!("R7 worker timing skipped: log path escaped R7");
        return;
    }
    let row = serde_json::json!({
        "process": "private-worker",
        "pid": std::process::id(),
        "stage": stage,
        "elapsed_ms": elapsed.as_secs_f64() * 1000.0,
        "details": details,
    });
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        Ok(mut file) => {
            if serde_json::to_writer(&mut file, &row).is_err() || file.write_all(b"\n").is_err() {
                eprintln!("R7 worker timing write failed");
            }
        }
        Err(error) => eprintln!("R7 worker timing open failed: {error}"),
    }
}
