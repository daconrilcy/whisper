use std::{
    fs::{File, OpenOptions},
    io::{BufWriter, Write},
    path::PathBuf,
    time::Instant,
};

pub struct NativeImportTiming {
    writer: BufWriter<File>,
    root: Instant,
}

impl NativeImportTiming {
    pub fn for_auto_test() -> Option<Self> {
        if std::env::var_os("WHISPER_DIAG_BANC_TRACE").as_deref() != Some(std::ffi::OsStr::new("1"))
            || std::thread::current().name()
                != Some("native_personal_auto_without_candidate_reports_preaudio_cpu_fallback")
        {
            return None;
        }
        let run_root = PathBuf::from(
            std::env::var_os("WHISPER_L04_CAMPAIGN_RUN")
                .expect("guarded run root is required when native timing is enabled"),
        );
        let root = run_root
            .canonicalize()
            .expect("canonicalize guarded run root for trace");
        assert!(root.is_dir(), "guarded run root must already exist");
        let unique = format!(
            "native-import-timing-{}-{}.jsonl",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let path = root.join(unique);
        let parent = path
            .parent()
            .unwrap()
            .canonicalize()
            .expect("resolve trace parent");
        assert!(
            parent.starts_with(&root),
            "trace must remain under the guarded run root"
        );
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .expect("create opt-in native import trace");
        let mut trace = Self {
            writer: BufWriter::new(file),
            root: Instant::now(),
        };
        trace.event("test_start", serde_json::json!({"test":"native_personal_auto_without_candidate_reports_preaudio_cpu_fallback"}));
        Some(trace)
    }

    pub fn event(&mut self, name: &str, details: serde_json::Value) {
        let record = serde_json::json!({"event":name,"monotonic_ms":self.root.elapsed().as_secs_f64()*1000.0,"details":details});
        serde_json::to_writer(&mut self.writer, &record).expect("serialize native timing event");
        self.writer
            .write_all(b"\n")
            .expect("write native timing event");
    }
}

impl Drop for NativeImportTiming {
    fn drop(&mut self) {
        self.event("test_end", serde_json::Value::Null);
        self.writer.flush().expect("flush native timing trace");
    }
}
