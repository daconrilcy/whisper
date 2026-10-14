use std::collections::HashSet;
#[cfg(not(test))]
use std::fs::{self, OpenOptions};
#[cfg(not(test))]
use std::io::BufWriter;
use std::io::Write;
#[cfg(not(test))]
use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};
use std::thread::JoinHandle;

#[derive(Debug)]
struct Entry {
    timestamp_unix_ms: u64,
    session: String,
    event: String,
    duration_ms: Option<u64>,
}

pub(crate) struct ReactivityLog {
    sender: Option<Sender<Entry>>,
    recorded: HashSet<(String, String)>,
    writer: Option<JoinHandle<()>>,
    enabled: bool,
}

impl ReactivityLog {
    #[cfg(not(test))]
    pub(crate) fn start() -> Self {
        let (sender, receiver) = mpsc::channel::<Entry>();
        let writer = match std::thread::Builder::new()
            .name("whisper-reactivity-log".into())
            .spawn(move || {
                let Some(path) = log_path() else {
                    eprintln!("Whisper: LOCALAPPDATA is unavailable; reactivity log disabled");
                    return;
                };
                if let Some(parent) = path.parent()
                    && let Err(error) = fs::create_dir_all(parent)
                {
                    eprintln!("Whisper: cannot create reactivity log directory: {error}");
                    return;
                }
                let file = match OpenOptions::new().create(true).append(true).open(&path) {
                    Ok(file) => file,
                    Err(error) => {
                        eprintln!("Whisper: cannot open {}: {error}", path.display());
                        return;
                    }
                };
                write_entries(BufWriter::new(file), receiver);
            }) {
            Ok(writer) => Some(writer),
            Err(error) => {
                eprintln!("Whisper: cannot start reactivity log writer: {error}");
                None
            }
        };
        Self {
            sender: Some(sender),
            recorded: HashSet::new(),
            writer,
            enabled: true,
        }
    }

    #[cfg(test)]
    pub(crate) fn start() -> Self {
        let (sender, receiver) = mpsc::channel();
        drop(receiver);
        Self {
            sender: Some(sender),
            recorded: HashSet::new(),
            writer: None,
            enabled: false,
        }
    }

    pub(crate) fn record_once(
        &mut self,
        session: impl Into<String>,
        event: impl Into<String>,
        timestamp_unix_ms: u64,
        origin_unix_ms: Option<u64>,
    ) {
        if !self.enabled {
            return;
        }
        let session = session.into();
        let event = event.into();
        if !should_record(&mut self.recorded, &session, &event) {
            return;
        }
        let duration_ms = origin_unix_ms.map(|origin| timestamp_unix_ms.saturating_sub(origin));
        let Some(sender) = self.sender.as_ref() else {
            return;
        };
        if sender
            .send(Entry {
                timestamp_unix_ms,
                session,
                event,
                duration_ms,
            })
            .is_err()
        {
            eprintln!("Whisper: reactivity log writer is unavailable");
        }
    }

    fn stop(&mut self) {
        self.sender.take();
        if let Some(writer) = self.writer.take()
            && writer.join().is_err()
        {
            eprintln!("Whisper: reactivity log writer stopped unexpectedly");
        }
    }
}

impl Drop for ReactivityLog {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(not(test))]
fn log_path() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .map(|root| root.join("Whisper").join("reactivity.log"))
}

fn write_entries(mut writer: impl Write, receiver: mpsc::Receiver<Entry>) {
    for entry in receiver {
        if let Err(error) =
            writeln!(writer, "{}", format_entry(&entry)).and_then(|()| writer.flush())
        {
            eprintln!("Whisper: cannot append reactivity log entry: {error}");
            return;
        }
    }
}

fn format_entry(entry: &Entry) -> String {
    let session = safe_field(&entry.session);
    let event = safe_field(&entry.event);
    let duration = entry
        .duration_ms
        .map_or_else(|| "-".to_owned(), |value| value.to_string());
    format!(
        "timestamp_unix_ms={}\tsession={}\tevent={}\tduration_ms={duration}",
        entry.timestamp_unix_ms, session, event
    )
}

fn safe_field(value: &str) -> String {
    value.replace(['\t', '\r', '\n'], "_")
}

fn should_record(recorded: &mut HashSet<(String, String)>, session: &str, event: &str) -> bool {
    recorded.insert((session.to_owned(), event.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::{Entry, ReactivityLog, format_entry, should_record, write_entries};
    use std::collections::HashSet;
    use std::sync::mpsc;

    #[test]
    fn formats_absolute_timestamp_and_duration_without_multiline_fields() {
        let line = format_entry(&Entry {
            timestamp_unix_ms: 1_800_000_000_123,
            session: "job\t2".into(),
            event: "micro\nclicked".into(),
            duration_ms: Some(250),
        });
        assert_eq!(
            line,
            "timestamp_unix_ms=1800000000123\tsession=job_2\tevent=micro_clicked\tduration_ms=250"
        );
    }

    #[test]
    fn records_each_milestone_once_per_session() {
        let mut recorded = HashSet::new();
        assert!(should_record(&mut recorded, "job-a", "micro_clicked"));
        assert!(!should_record(&mut recorded, "job-a", "micro_clicked"));
        assert!(should_record(&mut recorded, "job-b", "micro_clicked"));
        assert!(should_record(&mut recorded, "job-a", "engine_ready"));
    }

    #[test]
    fn writer_preserves_event_order() {
        let (sender, receiver) = mpsc::channel();
        for event in ["micro_clicked", "first_sample", "engine_ready"] {
            sender
                .send(Entry {
                    timestamp_unix_ms: 1,
                    session: "job-a".into(),
                    event: event.into(),
                    duration_ms: None,
                })
                .unwrap();
        }
        drop(sender);

        let mut output = Vec::new();
        write_entries(&mut output, receiver);
        let output = String::from_utf8(output).unwrap();
        let click = output.find("event=micro_clicked").unwrap();
        let sample = output.find("event=first_sample").unwrap();
        let ready = output.find("event=engine_ready").unwrap();
        assert!(click < sample && sample < ready);
    }

    #[test]
    fn dropping_logger_drains_all_previously_sent_entries() {
        let (sender, receiver) = mpsc::channel();
        let (journal_sender, journal_receiver) = mpsc::channel();
        let writer = std::thread::spawn(move || {
            let mut journal = Vec::new();
            write_entries(&mut journal, receiver);
            journal_sender.send(journal).unwrap();
        });
        let mut logger = ReactivityLog {
            sender: Some(sender),
            recorded: HashSet::new(),
            writer: Some(writer),
            enabled: true,
        };

        for event in ["micro_clicked", "first_sample", "engine_ready"] {
            logger.record_once("job-a", event, 1_800_000_000_123, None);
        }
        drop(logger);

        let contents = String::from_utf8(journal_receiver.recv().unwrap()).unwrap();
        assert_eq!(contents.lines().count(), 3);
        assert!(contents.contains("event=micro_clicked"));
        assert!(contents.contains("event=first_sample"));
        assert!(contents.contains("event=engine_ready"));
    }
}
