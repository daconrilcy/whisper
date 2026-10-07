use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc::{self, SyncSender, TrySendError},
    },
    thread::{self, JoinHandle},
};

pub const SYNC_EVERY_FRAMES: usize = 25;
pub const MAX_PENDING_FRAMES: usize = 50;
const HEADER: &[u8] = b"WHISPCM1\0\x80\x3e\0\0\x01\0";

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct PcmCheckpoint {
    pub durable_samples: u64,
    pub payload_sha256: [u8; 32],
    pub synced_at_unix_ms: u64,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct StoredCheckpoint {
    version: u32,
    checkpoint: PcmCheckpoint,
}

pub struct PcmStaging {
    path: PathBuf,
    file: File,
    frames_since_sync: usize,
    samples_written: u64,
    durable_samples: u64,
    checksum: Sha256,
}

enum WriterCommand {
    Frame(Vec<i16>),
    Tail(Vec<i16>),
    Drain(SyncSender<io::Result<(u64, [u8; 32])>>),
}

/// Bounded disk writer used by live capture. Capture submits frames without waiting
/// for storage; Stop sends an ordered drain request and waits for its durable checkpoint.
pub struct PcmWriter {
    path: PathBuf,
    sender: Option<SyncSender<WriterCommand>>,
    worker: Option<JoinHandle<()>>,
    durable_samples: Arc<AtomicU64>,
    checksum: [u8; 32],
}

impl PcmStaging {
    /// Creates the durable pending PCM before a capture device is started.
    pub fn create(path: impl AsRef<Path>) -> io::Result<Self> {
        let path = path.as_ref().to_owned();
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        let mut file = file;
        file.write_all(HEADER)?;
        file.sync_all()?;
        let staging = Self {
            path,
            file,
            frames_since_sync: 0,
            samples_written: 0,
            durable_samples: 0,
            checksum: Sha256::new(),
        };
        staging.persist_checkpoint()?;
        Ok(staging)
    }

    pub fn append_frame(&mut self, samples: &[i16]) -> io::Result<u64> {
        if samples.len() != 320 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "PCM frame must contain 320 samples",
            ));
        }
        let mut bytes = [0u8; 640];
        for (chunk, sample) in bytes.as_chunks_mut::<2>().0.iter_mut().zip(samples) {
            chunk.copy_from_slice(&sample.to_le_bytes());
        }
        self.file.write_all(&bytes)?;
        self.checksum.update(bytes);
        self.samples_written = self
            .samples_written
            .checked_add(samples.len() as u64)
            .ok_or_else(|| io::Error::other("sample offset overflow"))?;
        self.frames_since_sync += 1;
        if self.frames_since_sync >= SYNC_EVERY_FRAMES {
            self.sync()?;
        }
        Ok(self.samples_written)
    }

    /// Stores the true final PCM remainder without shifting the passage's sample axis.
    /// VAD/inference may inspect a separately zero-padded copy, while this journal
    /// retains only samples captured before Stop.
    pub fn append_tail(&mut self, samples: &[i16]) -> io::Result<u64> {
        if samples.len() >= 320 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "PCM tail must be shorter than one frame",
            ));
        }
        let mut bytes = Vec::with_capacity(samples.len() * 2);
        for sample in samples {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        self.file.write_all(&bytes)?;
        self.checksum.update(&bytes);
        self.samples_written = self
            .samples_written
            .checked_add(samples.len() as u64)
            .ok_or_else(|| io::Error::other("sample offset overflow"))?;
        Ok(self.samples_written)
    }

    pub fn sync(&mut self) -> io::Result<()> {
        self.file.sync_data()?;
        self.frames_since_sync = 0;
        self.durable_samples = self.samples_written;
        self.persist_checkpoint()?;
        Ok(())
    }
    pub fn drain(&mut self) -> io::Result<u64> {
        self.sync()?;
        Ok(self.samples_written)
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn durable_samples(&self) -> u64 {
        self.durable_samples
    }
    pub fn checksum(&self) -> [u8; 32] {
        self.checksum.clone().finalize().into()
    }

    pub fn into_writer(self) -> PcmWriter {
        let path = self.path.clone();
        let durable_samples = Arc::new(AtomicU64::new(self.durable_samples()));
        let writer_durable = durable_samples.clone();
        let (sender, receiver) = mpsc::sync_channel(MAX_PENDING_FRAMES);
        let worker = thread::spawn(move || writer_loop(self, receiver, writer_durable));
        PcmWriter {
            path,
            sender: Some(sender),
            worker: Some(worker),
            durable_samples,
            checksum: [0; 32],
        }
    }

    pub fn inspect_checkpoint(path: impl AsRef<Path>) -> Result<PcmCheckpoint, String> {
        let path = path.as_ref();
        let stored: StoredCheckpoint = serde_json::from_slice(
            &fs::read(checkpoint_path(path))
                .map_err(|e| format!("Recoverable: PCM checkpoint unavailable: {e}"))?,
        )
        .map_err(|e| format!("StorageCorrupt: PCM checkpoint: {e}"))?;
        if stored.version != 1 {
            return Err("StorageCorrupt: unsupported PCM checkpoint version".into());
        }
        let expected_len = (HEADER.len() as u64)
            .checked_add(
                stored
                    .checkpoint
                    .durable_samples
                    .checked_mul(2)
                    .ok_or_else(|| "StorageCorrupt: PCM checkpoint length overflow".to_owned())?,
            )
            .ok_or_else(|| "StorageCorrupt: PCM checkpoint length overflow".to_owned())?;
        let mut file =
            File::open(path).map_err(|e| format!("Recoverable: PCM staging unavailable: {e}"))?;
        if file.metadata().map_err(|e| e.to_string())?.len() < expected_len {
            return Err(
                "StorageCorrupt: PCM staging is shorter than its durable checkpoint".into(),
            );
        }
        let mut header = vec![0; HEADER.len()];
        file.read_exact(&mut header).map_err(|e| e.to_string())?;
        if header != HEADER {
            return Err("StorageCorrupt: PCM staging header mismatch".into());
        }
        let mut hash = Sha256::new();
        let mut remaining = expected_len - HEADER.len() as u64;
        let mut buffer = [0u8; 64 * 1024];
        while remaining > 0 {
            let read_len = remaining.min(buffer.len() as u64) as usize;
            let count = file
                .read(&mut buffer[..read_len])
                .map_err(|e| e.to_string())?;
            if count == 0 {
                return Err("StorageCorrupt: truncated durable PCM payload".into());
            }
            hash.update(&buffer[..count]);
            remaining -= count as u64;
        }
        if <[u8; 32]>::from(hash.finalize()) != stored.checkpoint.payload_sha256 {
            return Err("StorageCorrupt: PCM checkpoint hash mismatch".into());
        }
        Ok(stored.checkpoint)
    }

    fn persist_checkpoint(&self) -> io::Result<()> {
        let checkpoint = PcmCheckpoint {
            durable_samples: self.durable_samples,
            payload_sha256: self.checksum.clone().finalize().into(),
            synced_at_unix_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(io::Error::other)?
                .as_millis()
                .min(u128::from(u64::MAX)) as u64,
        };
        let stored = StoredCheckpoint {
            version: 1,
            checkpoint,
        };
        let path = checkpoint_path(&self.path);
        let temporary = path.with_extension("state.tmp");
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temporary)?;
        serde_json::to_writer(&mut file, &stored).map_err(io::Error::other)?;
        file.sync_all()?;
        drop(file);
        fs::rename(temporary, path)?;
        Ok(())
    }
}

impl PcmWriter {
    /// Reads only a fully synced range so inference can be dispatched from the
    /// durable spool without retaining an unbounded in-memory backlog.
    pub fn read_durable_samples(
        &self,
        start_sample: u64,
        sample_count: usize,
    ) -> io::Result<Option<Vec<i16>>> {
        let end_sample = start_sample
            .checked_add(sample_count as u64)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "sample range overflow"))?;
        if end_sample > self.durable_samples() {
            return Ok(None);
        }
        let byte_count = sample_count
            .checked_mul(2)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "byte range overflow"))?;
        let offset = (HEADER.len() as u64)
            .checked_add(start_sample.checked_mul(2).ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "sample offset overflow")
            })?)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "file offset overflow"))?;
        let mut file = File::open(&self.path)?;
        file.seek(SeekFrom::Start(offset))?;
        let mut bytes = vec![0; byte_count];
        file.read_exact(&mut bytes)?;
        Ok(Some(
            bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|chunk| i16::from_le_bytes(*chunk))
                .collect(),
        ))
    }

    /// Nonblocking capture path: a full queue becomes an explicit interruption.
    pub fn try_append_frame(&self, samples: Vec<i16>) -> io::Result<()> {
        validate_frame(&samples)?;
        self.sender
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "PCM writer has stopped"))?
            .try_send(WriterCommand::Frame(samples))
            .map_err(|error| match error {
                TrySendError::Full(_) => {
                    io::Error::new(io::ErrorKind::WouldBlock, "PCM writer queue is full")
                }
                TrySendError::Disconnected(_) => {
                    io::Error::new(io::ErrorKind::BrokenPipe, "PCM writer has stopped")
                }
            })
    }

    /// Stop path may wait for a queue slot, then drains all earlier frames in FIFO order.
    pub fn append_frame(&self, samples: Vec<i16>) -> io::Result<()> {
        validate_frame(&samples)?;
        self.sender
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "PCM writer has stopped"))?
            .send(WriterCommand::Frame(samples))
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "PCM writer has stopped"))
    }

    pub fn append_tail(&self, samples: Vec<i16>) -> io::Result<()> {
        if samples.len() >= 320 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "PCM tail must be shorter than one frame",
            ));
        }
        self.sender
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "PCM writer has stopped"))?
            .send(WriterCommand::Tail(samples))
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "PCM writer has stopped"))
    }

    pub fn drain(&mut self) -> io::Result<u64> {
        let (reply_tx, reply_rx) = mpsc::sync_channel(0);
        self.sender
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::BrokenPipe, "PCM writer has stopped"))?
            .send(WriterCommand::Drain(reply_tx))
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "PCM writer has stopped"))?;
        let (durable_samples, checksum) = reply_rx
            .recv()
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "PCM writer has stopped"))??;
        self.durable_samples
            .store(durable_samples, Ordering::Release);
        self.checksum = checksum;
        Ok(durable_samples)
    }

    pub fn durable_samples(&self) -> u64 {
        self.durable_samples.load(Ordering::Acquire)
    }

    pub fn checksum(&self) -> [u8; 32] {
        self.checksum
    }
}

impl Drop for PcmWriter {
    fn drop(&mut self) {
        self.sender.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn writer_loop(
    mut staging: PcmStaging,
    receiver: mpsc::Receiver<WriterCommand>,
    durable_samples: Arc<AtomicU64>,
) {
    while let Ok(command) = receiver.recv() {
        match command {
            WriterCommand::Frame(samples) => {
                if staging.append_frame(&samples).is_err() {
                    return;
                }
                durable_samples.store(staging.durable_samples(), Ordering::Release);
            }
            WriterCommand::Tail(samples) => {
                if staging.append_tail(&samples).is_err() {
                    return;
                }
            }
            WriterCommand::Drain(reply) => {
                let result = staging.drain().map(|_| {
                    durable_samples.store(staging.durable_samples(), Ordering::Release);
                    (staging.durable_samples(), staging.checksum())
                });
                let _ = reply.send(result);
            }
        }
    }
}

fn validate_frame(samples: &[i16]) -> io::Result<()> {
    if samples.len() == 320 {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "PCM frame must contain 320 samples",
        ))
    }
}

fn checkpoint_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("passage.pcm");
    path.with_file_name(format!("{name}.state.json"))
}
