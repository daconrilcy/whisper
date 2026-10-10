use std::time::Instant;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, SyncSender, TrySendError},
    },
    thread::{self, JoinHandle},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use whisper_core::{BackendAttempt, ComputeChoice, ComputePolicy, ComputePolicyError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProgressPhase {
    Preparing,
    Capturing,
    Import,
    Inference,
    Draining,
    Finalizing,
    Cancelling,
    Quitting,
}

#[derive(Clone, Debug)]
pub struct WorkerPaths {
    pub cpu: PathBuf,
    pub gpu: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkerSelection {
    pub path: PathBuf,
    pub backend: BackendAttempt,
}

impl WorkerPaths {
    pub fn select(&self, choice: ComputeChoice) -> Result<WorkerSelection, ComputePolicyError> {
        let gpu_available = self.gpu.is_file();
        let backend = ComputePolicy::initial(choice, gpu_available)?;
        let path = match backend {
            BackendAttempt::Cpu => self.cpu.clone(),
            BackendAttempt::Gpu => self.gpu.clone(),
        };
        if !path.is_file() {
            return Err(if backend == BackendAttempt::Gpu {
                ComputePolicyError::GpuUnavailable
            } else {
                ComputePolicyError::CpuUnavailable
            });
        }
        Ok(WorkerSelection { path, backend })
    }
}

const DIAGNOSTIC_FILE_LIMIT: u64 = 2 * 1024 * 1024;
const DIAGNOSTIC_FILE_COUNT: usize = 4;
const DIAGNOSTIC_MAX_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);
const DIAGNOSTIC_QUEUE_CAPACITY: usize = 128;
const DIAGNOSTIC_RECORD_LIMIT: usize = 4096;

#[derive(Clone, Copy, Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticPhase {
    Preparing,
    Capturing,
    Import,
    Inference,
    Draining,
    Finalizing,
    Cancelling,
    Quitting,
}

impl From<ProgressPhase> for DiagnosticPhase {
    fn from(value: ProgressPhase) -> Self {
        match value {
            ProgressPhase::Preparing => Self::Preparing,
            ProgressPhase::Capturing => Self::Capturing,
            ProgressPhase::Import => Self::Import,
            ProgressPhase::Inference => Self::Inference,
            ProgressPhase::Draining => Self::Draining,
            ProgressPhase::Finalizing => Self::Finalizing,
            ProgressPhase::Cancelling => Self::Cancelling,
            ProgressPhase::Quitting => Self::Quitting,
        }
    }
}

impl From<ProgressPhase> for whisper_core::DiagnosticPhase {
    fn from(value: ProgressPhase) -> Self {
        match value {
            ProgressPhase::Preparing => Self::Preparing,
            ProgressPhase::Capturing => Self::Capturing,
            ProgressPhase::Import => Self::Import,
            ProgressPhase::Inference => Self::Inference,
            ProgressPhase::Draining => Self::Draining,
            ProgressPhase::Finalizing => Self::Finalizing,
            ProgressPhase::Cancelling => Self::Cancelling,
            ProgressPhase::Quitting => Self::Quitting,
        }
    }
}

#[derive(Clone, Copy, Debug, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticCode {
    WorkerStarted,
    BackendReady,
    Progress,
    WorkerFailed,
    WorkerStopped,
    LogDegraded,
    ProgressWarning,
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct DiagnosticRecord {
    schema: u8,
    timestamp_unix_ms: u64,
    job_id: u128,
    generation: u64,
    phase: DiagnosticPhase,
    code: DiagnosticCode,
    backend: Option<&'static str>,
    completed_samples: Option<u64>,
    total_samples: Option<u64>,
}

#[derive(serde::Deserialize)]
struct CampaignDiagnosticIdentity {
    job_id: u128,
    generation: u64,
}

pub struct DiagnosticSink {
    sender: Option<SyncSender<Vec<u8>>>,
    writer: Option<JoinHandle<()>>,
    dropped: Arc<AtomicU64>,
    degraded: Arc<std::sync::atomic::AtomicBool>,
}

impl DiagnosticSink {
    pub fn start(destination: &Path) -> Result<Self, std::io::Error> {
        crate::archive::l04_campaign_assert_archive(destination).map_err(std::io::Error::other)?;
        let directory = destination.join(".whisper-diagnostics");
        fs::create_dir_all(&directory)?;
        crate::archive::l04_campaign_gate("diagnostic.before_startup_purge", 0, 0, 0)
            .map_err(std::io::Error::other)?;
        let purge_result = expire_old_diagnostics(&directory, SystemTime::now());
        crate::archive::l04_campaign_operation_result(
            "diagnostic.before_startup_purge",
            0,
            0,
            0,
            &purge_result,
            None,
        )
        .map_err(std::io::Error::other)?;
        purge_result?;
        let (sender, receiver) = mpsc::sync_channel::<Vec<u8>>(DIAGNOSTIC_QUEUE_CAPACITY);
        let dropped = Arc::new(AtomicU64::new(0));
        let degraded = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let writer_degraded = degraded.clone();
        let writer = thread::Builder::new()
            .name("whisper-diagnostics".into())
            .spawn(move || {
                while let Ok(line) = receiver.recv() {
                    if append_rotating(&directory, &line).is_err() {
                        writer_degraded.store(true, Ordering::Release);
                        break;
                    }
                }
            })?;
        Ok(Self {
            sender: Some(sender),
            writer: Some(writer),
            dropped,
            degraded,
        })
    }

    pub fn record(&self, record: DiagnosticRecord) {
        let Ok(mut bytes) = serde_json::to_vec(&record) else {
            self.dropped.fetch_add(1, Ordering::Relaxed);
            return;
        };
        if bytes.len() + 1 > DIAGNOSTIC_RECORD_LIMIT {
            self.dropped.fetch_add(1, Ordering::Relaxed);
            self.degraded.store(true, Ordering::Release);
            return;
        }
        bytes.push(b'\n');
        let Some(sender) = &self.sender else {
            self.dropped.fetch_add(1, Ordering::Relaxed);
            self.degraded.store(true, Ordering::Release);
            return;
        };
        match sender.try_send(bytes) {
            Ok(()) => {}
            Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => {
                self.dropped.fetch_add(1, Ordering::Relaxed);
                self.degraded.store(true, Ordering::Release);
            }
        }
    }

    pub fn dropped_count(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    pub fn is_degraded(&self) -> bool {
        self.degraded.load(Ordering::Acquire)
    }
}

impl Drop for DiagnosticSink {
    fn drop(&mut self) {
        self.sender.take();
        if let Some(writer) = self.writer.take() {
            let _ = writer.join();
        }
    }
}

impl DiagnosticRecord {
    pub fn new(
        job_id: u128,
        generation: u64,
        phase: DiagnosticPhase,
        code: DiagnosticCode,
        backend: Option<&'static str>,
        progress: Option<(u64, u64)>,
    ) -> Self {
        Self {
            schema: 1,
            timestamp_unix_ms: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |duration| {
                    duration.as_millis().min(u128::from(u64::MAX)) as u64
                }),
            job_id,
            generation,
            phase,
            code,
            backend,
            completed_samples: progress.map(|value| value.0),
            total_samples: progress.map(|value| value.1),
        }
    }
}

fn append_rotating(directory: &Path, line: &[u8]) -> Result<(), std::io::Error> {
    crate::archive::l04_campaign_assert_archive(directory.parent().ok_or_else(|| {
        std::io::Error::other("CampaignGuard: diagnostic directory has no parent")
    })?)
    .map_err(std::io::Error::other)?;
    let identity: Option<CampaignDiagnosticIdentity> = if option_env!("WHISPER_L04_CAMPAIGN_BUILD")
        .is_some()
        && std::env::var_os("WHISPER_L04_CAMPAIGN_RUN").is_some()
    {
        Some(serde_json::from_slice(line).map_err(std::io::Error::other)?)
    } else {
        None
    };
    let job_id = identity.as_ref().map_or(0, |value| value.job_id);
    let generation = identity.as_ref().map_or(0, |value| value.generation);
    let gate = |hook| {
        crate::archive::l04_campaign_gate(hook, job_id, generation, 0)
            .map_err(std::io::Error::other)
    };
    gate("diagnostic.before_purge")?;
    let now = SystemTime::now();
    let purge_result = expire_old_diagnostics(directory, now);
    crate::archive::l04_campaign_operation_result(
        "diagnostic.before_purge",
        job_id,
        generation,
        0,
        &purge_result,
        None,
    )
    .map_err(std::io::Error::other)?;
    purge_result?;
    let active = diagnostic_path(directory, 0);
    let should_rotate = fs::metadata(&active).is_ok_and(|metadata| {
        metadata.len().saturating_add(line.len() as u64) > DIAGNOSTIC_FILE_LIMIT
    });
    if should_rotate {
        gate("diagnostic.before_rotate")?;
        let rotate_result = (|| -> Result<(), std::io::Error> {
            let oldest = diagnostic_path(directory, DIAGNOSTIC_FILE_COUNT - 1);
            remove_if_present(&oldest)?;
            for index in (1..DIAGNOSTIC_FILE_COUNT).rev() {
                let previous = diagnostic_path(directory, index - 1);
                if previous.exists() {
                    fs::rename(previous, diagnostic_path(directory, index))?;
                }
            }
            Ok(())
        })();
        crate::archive::l04_campaign_operation_result(
            "diagnostic.before_rotate",
            job_id,
            generation,
            0,
            &rotate_result,
            None,
        )
        .map_err(std::io::Error::other)?;
        rotate_result?;
    }
    gate("diagnostic.before_append")?;
    let append_result = (|| -> Result<(), std::io::Error> {
        let mut file = OpenOptions::new().create(true).append(true).open(&active)?;
        file.write_all(line)?;
        file.flush()
    })();
    crate::archive::l04_campaign_operation_result(
        "diagnostic.before_append",
        job_id,
        generation,
        0,
        &append_result,
        None,
    )
    .map_err(std::io::Error::other)?;
    append_result
}

fn expire_old_diagnostics(directory: &Path, now: SystemTime) -> Result<(), std::io::Error> {
    for index in 0..DIAGNOSTIC_FILE_COUNT {
        let path = diagnostic_path(directory, index);
        if let Ok(metadata) = fs::metadata(&path)
            && metadata.modified().ok().is_some_and(|modified| {
                now.duration_since(modified).unwrap_or_default() > DIAGNOSTIC_MAX_AGE
            })
        {
            remove_if_present(&path)?;
        }
    }
    Ok(())
}

fn remove_if_present(path: &Path) -> Result<(), std::io::Error> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn diagnostic_path(directory: &Path, index: usize) -> PathBuf {
    directory.join(format!("worker-{index}.jsonl"))
}

pub struct ProgressWatch {
    phase: ProgressPhase,
    last_progress: Instant,
    completed: u64,
    warned: bool,
}

impl ProgressWatch {
    pub fn new(phase: ProgressPhase, now: Instant) -> Self {
        Self {
            phase,
            last_progress: now,
            completed: 0,
            warned: false,
        }
    }

    pub fn transition(&mut self, phase: ProgressPhase, now: Instant) {
        if self.phase != phase {
            self.phase = phase;
            self.last_progress = now;
            self.completed = 0;
            self.warned = false;
        }
    }

    pub fn observe(&mut self, completed: u64, now: Instant) {
        if completed > self.completed {
            self.completed = completed;
            self.last_progress = now;
            self.warned = false;
        }
    }

    pub fn warning(&mut self, now: Instant) -> Option<(ProgressPhase, u64)> {
        let idle = now.saturating_duration_since(self.last_progress);
        if !self.warned && idle >= Duration::from_secs(60) {
            self.warned = true;
            Some((
                self.phase,
                idle.as_millis().min(u128::from(u64::MAX)) as u64,
            ))
        } else {
            None
        }
    }
}

#[derive(Default)]
pub struct ProgressSupervisor {
    obligations: Vec<ProgressWatch>,
}

impl ProgressSupervisor {
    pub fn starting(now: Instant) -> Self {
        Self {
            obligations: vec![ProgressWatch::new(ProgressPhase::Preparing, now)],
        }
    }

    pub fn open(&mut self, phase: ProgressPhase, now: Instant) {
        if !self.obligations.iter().any(|watch| watch.phase == phase) {
            self.obligations.push(ProgressWatch::new(phase, now));
        }
    }

    pub fn close(&mut self, phase: ProgressPhase) {
        self.obligations.retain(|watch| watch.phase != phase);
    }

    pub fn observe(&mut self, phase: ProgressPhase, completed: u64, now: Instant) {
        self.obligations
            .iter_mut()
            .filter(|watch| watch.phase == phase)
            .for_each(|watch| watch.observe(completed, now));
    }

    pub fn take_warning(&mut self, now: Instant) -> Option<(ProgressPhase, u64)> {
        self.obligations
            .iter_mut()
            .find_map(|watch| watch.warning(now))
    }

    pub fn close_all(&mut self) {
        self.obligations.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::Digest;
    use std::{
        fs::FileTimes,
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

    struct TempDirectory(PathBuf);

    impl TempDirectory {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "whisper-supervisor-{}-{}",
                std::process::id(),
                NEXT_TEMP.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn record() -> DiagnosticRecord {
        DiagnosticRecord::new(
            7,
            2,
            DiagnosticPhase::Inference,
            DiagnosticCode::Progress,
            Some("cpu"),
            Some((10, 20)),
        )
    }

    #[test]
    fn startup_expires_only_known_stale_diagnostic_files() {
        let temp = TempDirectory::new();
        let directory = temp.0.join(".whisper-diagnostics");
        fs::create_dir_all(&directory).unwrap();
        let stale = diagnostic_path(&directory, 0);
        fs::write(&stale, b"stale").unwrap();
        OpenOptions::new()
            .write(true)
            .open(&stale)
            .unwrap()
            .set_times(FileTimes::new().set_modified(SystemTime::now() - DIAGNOSTIC_MAX_AGE * 2))
            .unwrap();
        let sentinel = directory.join("keep-me.jsonl");
        fs::write(&sentinel, b"unknown file").unwrap();

        let sink = DiagnosticSink::start(&temp.0).unwrap();
        drop(sink);

        assert!(!stale.exists());
        assert_eq!(fs::read(sentinel).unwrap(), b"unknown file");
    }

    #[test]
    fn dropping_sink_drains_queued_records_and_closes_writer() {
        let temp = TempDirectory::new();
        let sink = DiagnosticSink::start(&temp.0).unwrap();
        for _ in 0..32 {
            sink.record(record());
        }
        drop(sink);

        let output = fs::read(diagnostic_path(&temp.0.join(".whisper-diagnostics"), 0)).unwrap();
        assert_eq!(output.iter().filter(|byte| **byte == b'\n').count(), 32);
    }

    #[test]
    fn bounded_queue_drops_without_blocking_and_marks_sink_degraded() {
        let temp = TempDirectory::new();
        let sink = DiagnosticSink::start(&temp.0).unwrap();
        let oversized = DiagnosticRecord {
            schema: 1,
            timestamp_unix_ms: 0,
            job_id: 1,
            generation: 1,
            phase: DiagnosticPhase::Inference,
            code: DiagnosticCode::Progress,
            backend: Some("cpu"),
            completed_samples: Some(u64::MAX),
            total_samples: Some(u64::MAX),
        };
        for _ in 0..(DIAGNOSTIC_QUEUE_CAPACITY + 1_000) {
            sink.record(oversized.clone());
        }
        assert!(sink.dropped_count() > 0);
        assert!(sink.is_degraded());
    }

    #[test]
    fn rotation_stays_within_four_files_and_preserves_unknown_files() {
        let temp = TempDirectory::new();
        let directory = temp.0.join(".whisper-diagnostics");
        fs::create_dir_all(&directory).unwrap();
        let sentinel = directory.join("other.log");
        fs::write(&sentinel, b"preserve").unwrap();
        let line = vec![b'x'; DIAGNOSTIC_FILE_LIMIT as usize];
        for _ in 0..8 {
            append_rotating(&directory, &line).unwrap();
        }
        let known: Vec<_> = (0..DIAGNOSTIC_FILE_COUNT)
            .map(|index| diagnostic_path(&directory, index))
            .filter(|path| path.exists())
            .collect();
        assert!(known.len() <= DIAGNOSTIC_FILE_COUNT);
        let total: u64 = known
            .iter()
            .map(|path| fs::metadata(path).unwrap().len())
            .sum();
        assert!(total <= DIAGNOSTIC_FILE_LIMIT * DIAGNOSTIC_FILE_COUNT as u64);
        assert_eq!(fs::read(sentinel).unwrap(), b"preserve");
    }

    #[test]
    fn writer_io_failure_marks_sink_degraded() {
        let temp = TempDirectory::new();
        let directory = temp.0.join(".whisper-diagnostics");
        fs::create_dir_all(&directory).unwrap();
        fs::create_dir(diagnostic_path(&directory, 0)).unwrap();
        let sink = DiagnosticSink::start(&temp.0).unwrap();
        sink.record(record());
        for _ in 0..100 {
            if sink.is_degraded() {
                return;
            }
            thread::sleep(Duration::from_millis(5));
        }
        panic!("diagnostic writer did not report its I/O failure");
    }

    #[test]
    fn campaign_startup_purge_can_fail_before_deleting_old_diagnostics() {
        if option_env!("WHISPER_L04_CAMPAIGN_BUILD").is_none() {
            return;
        }
        if std::env::var_os("WHISPER_L04_STARTUP_TEST_CHILD").is_some() {
            let destination = PathBuf::from(std::env::var_os("WHISPER_L04_CAMPAIGN_RUN").unwrap())
                .join("archive");
            let diagnostics = destination.join(".whisper-diagnostics");
            fs::create_dir_all(&diagnostics).unwrap();
            let stale = diagnostics.join("worker-0.jsonl");
            let unknown = diagnostics.join("keep.txt");
            fs::write(&stale, b"old").unwrap();
            fs::write(&unknown, b"unknown").unwrap();
            let old = SystemTime::now() - DIAGNOSTIC_MAX_AGE - Duration::from_secs(60);
            fs::File::options()
                .write(true)
                .open(&stale)
                .unwrap()
                .set_times(FileTimes::new().set_modified(old))
                .unwrap();
            let error = DiagnosticSink::start(&destination).err().unwrap();
            assert!(error.to_string().contains("CampaignInjected"));
            assert_eq!(fs::read(stale).unwrap(), b"old");
            assert_eq!(fs::read(unknown).unwrap(), b"unknown");
            return;
        }
        let runs = Path::new(r"C:\WhisperLive\L04-campaign\runs");
        fs::create_dir_all(runs).unwrap();
        let binary = std::env::current_exe().unwrap();
        let binary_sha256 = format!("{:x}", sha2::Sha256::digest(fs::read(&binary).unwrap()));
        let run = runs.join(format!("purge-test-{}", std::process::id()));
        fs::create_dir(&run).unwrap();
        fs::create_dir(run.join("archive")).unwrap();
        fs::write(
            run.join("run.json"),
            serde_json::to_vec(&serde_json::json!({
                "schema":"whisper-l04-fault/1", "hook":"diagnostic.before_startup_purge",
                "job_id":"0", "generation":0, "instance_id":0, "trigger_count":1,
                "binary":{"sha256":binary_sha256},
            }))
            .unwrap(),
        )
        .unwrap();
        fs::write(run.join("decision.txt"), b"fail\n").unwrap();
        let result = std::process::Command::new(binary)
            .args(["--exact", "supervisor::tests::campaign_startup_purge_can_fail_before_deleting_old_diagnostics"])
            .env("WHISPER_L04_STARTUP_TEST_CHILD", "1")
            .env("WHISPER_L04_CAMPAIGN_RUN", &run)
            .status().unwrap();
        assert!(result.success());
        fs::remove_dir_all(run).unwrap();
    }
}
