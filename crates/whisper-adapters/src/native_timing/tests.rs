use super::{trace_enabled, trace_path, write_record};
use serde_json::Value;
use std::{
    fs,
    time::{Instant, SystemTime, UNIX_EPOCH},
};

#[test]
fn trace_path_stays_directly_under_existing_root() {
    let root = temporary_root();
    let path = trace_path(&root, 7, 11).expect("derive trace path");
    assert_eq!(path.parent(), Some(root.as_path()));
    assert!(
        path.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("native-product-timing-7-"))
    );
    fs::remove_dir_all(root).expect("remove temporary test root");
}

#[test]
fn records_are_jsonl_with_monotonic_timestamps() {
    let mut output = Vec::new();
    let origin = Instant::now();
    write_record(&mut output, origin, "first", Value::Null, 7, "test-thread")
        .expect("write first event");
    write_record(&mut output, origin, "second", Value::Null, 7, "test-thread")
        .expect("write second event");
    let records = output
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice::<Value>(line).expect("parse JSONL event"))
        .collect::<Vec<_>>();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0]["event"], "first");
    assert_eq!(records[1]["event"], "second");
    assert!(
        records[0]["monotonic_ms"].as_f64().unwrap()
            <= records[1]["monotonic_ms"].as_f64().unwrap()
    );
    assert_eq!(records[0]["pid"], 7);
    assert_eq!(records[0]["thread"], "test-thread");
}

#[test]
fn tracing_requires_the_exact_opt_in_value() {
    use std::ffi::OsStr;

    assert!(trace_enabled(Some(OsStr::new("1"))));
    assert!(!trace_enabled(Some(OsStr::new("0"))));
    assert!(!trace_enabled(Some(OsStr::new("01"))));
    assert!(!trace_enabled(None));
}

fn temporary_root() -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    let root =
        std::env::temp_dir().join(format!("native-timing-test-{}-{nonce}", std::process::id()));
    fs::create_dir(&root).expect("create temporary test root");
    root.canonicalize()
        .expect("canonicalize temporary test root")
}
