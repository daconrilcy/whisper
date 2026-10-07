use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
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
        let mut staging = Self {
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
        for (chunk, sample) in bytes.chunks_exact_mut(2).zip(samples) {
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
            let count = file
                .read(&mut buffer[..remaining.min(buffer.len() as u64) as usize])
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

fn checkpoint_path(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("passage.pcm");
    path.with_file_name(format!("{name}.state.json"))
}
