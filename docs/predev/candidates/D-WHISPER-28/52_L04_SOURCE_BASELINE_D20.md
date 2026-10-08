# Baseline de source L04 — D-WHISPER-20

Révision observée : `ffd93125604be5dc6439587b7edad4f420f5cccc` (branche `main`), le 2026-10-07.

Cette baseline est le D0 propre au lot L04 après les lots L00–L03. Elle complète la baseline de conception D19 sans la remplacer et lie au manifeste DESIGN les dix fichiers déjà présents. Les cinq créations futures sont intentionnellement absentes et ne reçoivent aucun hash. Ce document ne constitue pas le préflight ni une qualification produit.

Le snapshot P24 décrivait le même périmètre au `c75ae195b4431abd8f8c1dfd7abb45e81f0ad79b`. `git diff --name-only c75ae195b4431abd8f8c1dfd7abb45e81f0ad79b ffd93125604be5dc6439587b7edad4f420f5cccc -- crates Cargo.lock Cargo.toml rust-toolchain.toml` est vide : entre ces révisions, seuls des documents hors du périmètre code ont changé. Les hashes observés ci-dessous égalent les snapshots recopiés sans conversion depuis P24.

## Allowlist L04 (15 chemins)

- `crates/whisper-adapters/src/supervisor.rs` — ABSENT; création future; aucun snapshot baseline
- `crates/whisper-adapters/tests/worker_control.rs` — ABSENT; création future; aucun snapshot baseline
- `crates/whisper-core/tests/compute_policy.rs` — ABSENT; création future; aucun snapshot baseline
- `crates/whisper-worker-cpu/src/native_engine.rs` — PRÉSENT; SHA-256 `9d415e118185e0c97917098bf79902f649722a909f1486ef619c3626e88203d7`; snapshot DESIGN `sources/L04-consumer-baseline/crates/whisper-worker-cpu/src/native_engine.rs.txt`
- `crates/whisper-worker-gpu/src/native_engine.rs` — ABSENT; création future; aucun snapshot baseline
- `crates/whisper-adapters/src/lib.rs` — PRÉSENT; SHA-256 `9b71f5efe1d6669ca6e38e865915b6e4bfb6504558e076f520eaf86f20d09174`; snapshot DESIGN `sources/L04-consumer-baseline/crates/whisper-adapters/src/lib.rs.txt`
- `crates/whisper-adapters/src/worker_ipc.rs` — PRÉSENT; SHA-256 `4f1bfa7a9bdf0811046204831224a348901dfc36b64cec225ebab53ed284e2c2`; snapshot DESIGN `sources/L04-consumer-baseline/crates/whisper-adapters/src/worker_ipc.rs.txt`
- `crates/whisper-core/src/application.rs` — PRÉSENT; SHA-256 `47c32fcc0f9d2709670723ffea636a0324bdef51a62f9d0c9b35dbc4f2294c69`; snapshot DESIGN `sources/L04-consumer-baseline/crates/whisper-core/src/application.rs.txt`
- `crates/whisper-core/src/compute_policy.rs` — ABSENT; création future; aucun snapshot baseline
- `crates/whisper-core/src/lib.rs` — PRÉSENT; SHA-256 `5b047a792ee30b73fed1e3cab2c8bc1a47b8f41f781ceae750905cbd27607d3a`; snapshot DESIGN `sources/L04-consumer-baseline/crates/whisper-core/src/lib.rs.txt`
- `crates/whisper-desktop/src/root.rs` — PRÉSENT; SHA-256 `9a3713112e3dfc3872e15e3cb581c7c6a53222e5f62864144ba4d111caa98599`; snapshot DESIGN `sources/L04-consumer-baseline/crates/whisper-desktop/src/root.rs.txt`
- `crates/whisper-desktop/src/ui.rs` — PRÉSENT; SHA-256 `148900588d475028b2c0c27c65ed05116dd93aecf62568c78cef6a554b913648`; snapshot DESIGN `sources/L04-consumer-baseline/crates/whisper-desktop/src/ui.rs.txt`
- `crates/whisper-worker-gpu/src/main.rs` — PRÉSENT; SHA-256 `047430149342e5fec4e77d8b9009ee377af8c0b3c50a9c96b3fb52536d96f953`; snapshot DESIGN `sources/L04-consumer-baseline/crates/whisper-worker-gpu/src/main.rs.txt`
- `crates/whisper-worker-gpu/Cargo.toml` — PRÉSENT; SHA-256 `5ef21c0437965d8ea28c01bdd33f5c5d3ec867e2498ab4329b3a24ad54553b89`; snapshot DESIGN `sources/L04-config-baseline/crates__whisper-worker-gpu__Cargo.toml.txt`
- `Cargo.lock` — PRÉSENT; SHA-256 `9d714d4315a5dd63865faa6bc8eb9bcb84eb9abdfdd2182a1fafbb4c2b60b67f`; snapshot DESIGN `sources/L04-config-baseline/Cargo.lock.txt`

## Inventaire et reprise

`code_state.baseline` et `code_state.current` au préflight L04 doivent contenir exactement les dix fichiers présents ci-dessus, avec leurs snapshots D20. `ledger` démarre vide pour ce D0 L04. Les cinq fichiers absents restent hors baseline et current jusqu’à leur création autorisée.

L’identité d’observation de P24 (`c75ae…`) est conservée comme historique. L’autre inventory archivé (`1310bbcd…`) demeure inchangé. Cette observation D20, datée au HEAD courant, est la référence de ce lot. La comparaison ne prétend pas que le worktree global est propre.

L’autorisation de préparation de cette proposition de succession DESIGN est capturée dans `USER_AUTHORIZATION_D20_PROPOSAL.md`; l’autorisation distincte d’implémentation et de préflight est capturée dans `USER_AUTHORIZATION_L04.md`.

## D26 snapshots code pour ARCH-L04-BOUNDS-v1

### crates/whisper-adapters/src/capture.rs SHA256 6473322c054b2aead741a1c7e97567f1666e7fb4deedd7865423ead57396677c

````rust
use cpal::{
    Device, SampleFormat, Stream,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use rtrb::{Consumer, Producer, RingBuffer};
use rubato::{FftFixedInOut, Resampler};
use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};

pub const CAPTURE_SLOTS: usize = 100;
const CAPTURE_SATURATED: u8 = 1;
const CAPTURE_FAILED: u8 = 2;

pub struct CapturedBlock {
    pub sample_rate_hz: u32,
    pub channels: u16,
    samples: Vec<f32>,
}

impl CapturedBlock {
    pub fn samples(&self) -> &[f32] {
        &self.samples
    }
}

pub struct CaptureStream {
    stream: Option<Stream>,
    ready: Consumer<Vec<f32>>,
    recycle: Producer<Vec<f32>>,
    status: Arc<AtomicU8>,
    rate: u32,
    channels: u16,
}

pub struct Pcm16Converter {
    rate: u32,
    channels: u16,
    chunk: usize,
    pending: Vec<f32>,
    resampler: FftFixedInOut<f32>,
    output: Vec<Vec<f32>>,
    input_frames: u64,
    emitted_frames: u64,
    delay_remaining: usize,
    finished: bool,
}

impl Pcm16Converter {
    pub fn new(rate: u32, channels: u16) -> Result<Self, String> {
        if rate < 16_000 || channels == 0 {
            return Err("CaptureUnsupported: invalid input format".into());
        }
        let desired_chunk = ((rate as usize) * 20).div_ceil(1000).max(1);
        let resampler = FftFixedInOut::new(rate as usize, 16_000, desired_chunk, 1)
            .map_err(|error| format!("CaptureConversion: {error}"))?;
        let chunk = resampler.input_frames_next();
        let output = resampler.output_buffer_allocate(false);
        let delay_remaining = resampler.output_delay();
        Ok(Self {
            rate,
            channels,
            chunk,
            pending: Vec::with_capacity(chunk * 2),
            resampler,
            output,
            input_frames: 0,
            emitted_frames: 0,
            delay_remaining,
            finished: false,
        })
    }

    /// Downmixes and resamples off the device callback; caller returns the block to its pool.
    pub fn push(&mut self, block: &CapturedBlock) -> Result<Vec<i16>, String> {
        if self.finished {
            return Err("CaptureConversion: stream was already drained".into());
        }
        if block.sample_rate_hz != self.rate
            || block.channels != self.channels
            || !block
                .samples
                .len()
                .is_multiple_of(usize::from(self.channels))
        {
            return Err(
                "CaptureInterrupted: stream format changed or delivered a partial frame".into(),
            );
        }
        for frame in block.samples.chunks_exact(usize::from(self.channels)) {
            let mono = frame.iter().copied().sum::<f32>() / f32::from(self.channels);
            self.pending.push(mono.clamp(-1.0, 1.0));
            self.input_frames = self
                .input_frames
                .checked_add(1)
                .ok_or_else(|| "CaptureConversion: input sample count overflow".to_owned())?;
        }
        let mut result = Vec::new();
        while self.pending.len() >= self.chunk {
            let input = [&self.pending[..self.chunk]];
            let (_, produced) = self
                .resampler
                .process_into_buffer(&input, &mut self.output, None)
                .map_err(|error| format!("CaptureConversion: {error}"))?;
            self.collect_compensated(&mut result, produced, u64::MAX);
            self.pending.drain(..self.chunk);
        }
        Ok(result)
    }

    /// Pads the final input chunk and drains rubato's delayed overlap. The first
    /// output-delay frames are discarded and the tail is trimmed to the exact
    /// input-duration axis, so Stop cannot shift later passage timestamps.
    pub fn finish(&mut self) -> Result<Vec<i16>, String> {
        if self.finished {
            return Ok(Vec::new());
        }
        let target = ((u128::from(self.input_frames) * 16_000) / u128::from(self.rate)) as u64;
        let mut result = Vec::new();
        let partial = [&self.pending[..]];
        let (_, produced) = self
            .resampler
            .process_partial_into_buffer(Some(&partial), &mut self.output, None)
            .map_err(|error| format!("CaptureConversion: final input pad failed: {error}"))?;
        self.pending.clear();
        self.collect_compensated(&mut result, produced, target);
        while self.emitted_frames < target {
            let (_, produced) = self
                .resampler
                .process_partial_into_buffer::<Vec<f32>, Vec<f32>>(None, &mut self.output, None)
                .map_err(|error| {
                    format!("CaptureConversion: delayed output drain failed: {error}")
                })?;
            self.collect_compensated(&mut result, produced, target);
        }
        self.finished = true;
        Ok(result)
    }

    fn collect_compensated(&mut self, destination: &mut Vec<i16>, produced: usize, target: u64) {
        let skipped = self.delay_remaining.min(produced);
        self.delay_remaining -= skipped;
        let count = (produced - skipped) as u64;
        let keep = count.min(target.saturating_sub(self.emitted_frames)) as usize;
        destination.extend(
            self.output[0][skipped..skipped + keep]
                .iter()
                .map(|sample| (sample.clamp(-1.0, 1.0) * 32767.0).round() as i16),
        );
        self.emitted_frames = self.emitted_frames.saturating_add(keep as u64);
    }
}

fn capture_into<T: Copy>(
    data: &[T],
    convert: impl Fn(T) -> f32,
    channels: u16,
    capacity: usize,
    free: &mut Consumer<Vec<f32>>,
    ready: &mut Producer<Vec<f32>>,
    status: &AtomicU8,
) {
    let channels = usize::from(channels);
    if channels == 0 || !data.len().is_multiple_of(channels) {
        status.fetch_or(CAPTURE_FAILED, Ordering::Release);
        return;
    }
    for samples in data.chunks(capacity) {
        let Ok(mut block) = free.pop() else {
            status.fetch_or(CAPTURE_SATURATED, Ordering::Release);
            return;
        };
        if samples.len() > block.capacity() {
            status.fetch_or(CAPTURE_FAILED, Ordering::Release);
            return;
        }
        block.clear();
        block.extend(samples.iter().copied().map(&convert));
        if let Err(error) = ready.push(block) {
            drop(error); // Impossible while the two 100-slot pools remain balanced.
            status.fetch_or(CAPTURE_SATURATED, Ordering::Release);
            return;
        }
    }
}

pub fn start_default() -> Result<CaptureStream, String> {
    let host = cpal::host_from_id(cpal::HostId::Wasapi)
        .map_err(|error| format!("CaptureUnavailable: WASAPI: {error}"))?;
    let device = host
        .default_input_device()
        .ok_or_else(|| "CaptureUnavailable: no default WASAPI input device".to_owned())?;
    start_device(device)
}

pub fn start_device(device: Device) -> Result<CaptureStream, String> {
    let supported = device
        .default_input_config()
        .map_err(|error| format!("CaptureUnavailable: {error}"))?;
    let rate = supported.sample_rate();
    let channels = supported.channels();
    if rate < 16_000 || channels == 0 {
        return Err(
            "CaptureUnsupported: default device format cannot be converted to mono 16 kHz".into(),
        );
    }
    let format = supported.sample_format();
    let config = supported.config();
    let slot_samples = ((rate as usize) * 20)
        .div_ceil(1000)
        .checked_mul(usize::from(channels))
        .ok_or_else(|| "CaptureUnsupported: input dimensions overflow".to_owned())?;
    let (mut data_tx, ready) = RingBuffer::new(CAPTURE_SLOTS);
    // The producer is retained by the consumer thread for recycling; the callback
    // owns the consumer endpoint and receives the preallocated slots from here.
    let (mut recycle, mut free) = RingBuffer::new(CAPTURE_SLOTS);
    for _ in 0..CAPTURE_SLOTS {
        recycle
            .push(Vec::with_capacity(slot_samples))
            .map_err(|_| "CaptureUnavailable: cannot initialize bounded slot pool")?;
    }
    let status = Arc::new(AtomicU8::new(0));
    let callback_status = status.clone();
    let error_status = status.clone();
    let stream = match format {
        SampleFormat::F32 => device.build_input_stream(
            config,
            move |data: &[f32], _| {
                capture_into(
                    data,
                    |x| x,
                    channels,
                    slot_samples,
                    &mut free,
                    &mut data_tx,
                    &callback_status,
                )
            },
            move |_| {
                error_status.fetch_or(CAPTURE_FAILED, Ordering::Release);
            },
            None,
        ),
        SampleFormat::I16 => device.build_input_stream(
            config,
            move |data: &[i16], _| {
                capture_into(
                    data,
                    |x| f32::from(x) / 32768.0,
                    channels,
                    slot_samples,
                    &mut free,
                    &mut data_tx,
                    &callback_status,
                )
            },
            move |_| {
                error_status.fetch_or(CAPTURE_FAILED, Ordering::Release);
            },
            None,
        ),
        SampleFormat::U16 => device.build_input_stream(
            config,
            move |data: &[u16], _| {
                capture_into(
                    data,
                    |x| (f32::from(x) - 32768.0) / 32768.0,
                    channels,
                    slot_samples,
                    &mut free,
                    &mut data_tx,
                    &callback_status,
                )
            },
            move |_| {
                error_status.fetch_or(CAPTURE_FAILED, Ordering::Release);
            },
            None,
        ),
        other => return Err(format!("CaptureUnsupported: sample format {other:?}")),
    }
    .map_err(|error| format!("CaptureUnavailable: {error}"))?;
    stream
        .play()
        .map_err(|error| format!("CaptureUnavailable: {error}"))?;
    Ok(CaptureStream {
        stream: Some(stream),
        ready,
        recycle,
        status,
        rate,
        channels,
    })
}

impl CaptureStream {
    pub fn sample_rate_hz(&self) -> u32 {
        self.rate
    }
    pub fn channels(&self) -> u16 {
        self.channels
    }
    pub fn try_next_block(&mut self) -> Option<CapturedBlock> {
        self.ready.pop().ok().map(|samples| CapturedBlock {
            sample_rate_hz: self.rate,
            channels: self.channels,
            samples,
        })
    }
    pub fn recycle_block(&mut self, mut block: CapturedBlock) -> Result<(), String> {
        block.samples.clear();
        self.recycle
            .push(block.samples)
            .map_err(|_| "CaptureInterrupted: callback slot pool could not be recycled".to_owned())
    }
    pub fn is_saturated(&self) -> bool {
        self.status.load(Ordering::Acquire) & CAPTURE_SATURATED != 0
    }
    pub fn failure(&self) -> bool {
        self.status.load(Ordering::Acquire) & CAPTURE_FAILED != 0
    }
    pub fn stop(&mut self) {
        self.stream.take();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn callback_reuses_recycled_slots_without_blocking_or_saturating() {
        let (mut captured_tx, mut captured_rx) = RingBuffer::new(CAPTURE_SLOTS);
        let (mut recycle_tx, mut free_rx) = RingBuffer::new(CAPTURE_SLOTS);
        for _ in 0..CAPTURE_SLOTS {
            recycle_tx.push(Vec::with_capacity(4)).unwrap();
        }
        let status = AtomicU8::new(0);

        for value in 0..CAPTURE_SLOTS * 2 {
            let data = [value as f32; 4];
            capture_into(
                &data,
                |sample| sample,
                2,
                4,
                &mut free_rx,
                &mut captured_tx,
                &status,
            );
            let block = captured_rx
                .pop()
                .expect("capture callback queued one block");
            assert_eq!(block.as_slice(), data.as_slice());
            recycle_tx.push(block).expect("consumer returns its slot");
        }

        assert_eq!(status.load(Ordering::Acquire), 0);
    }

    #[test]
    fn callback_marks_partial_channel_frames_as_failed() {
        let (mut captured_tx, _captured_rx) = RingBuffer::new(CAPTURE_SLOTS);
        let (mut recycle_tx, mut free_rx) = RingBuffer::new(CAPTURE_SLOTS);
        recycle_tx.push(Vec::with_capacity(4)).unwrap();
        let status = AtomicU8::new(0);

        capture_into(
            &[0.0f32; 3],
            |sample| sample,
            2,
            4,
            &mut free_rx,
            &mut captured_tx,
            &status,
        );

        assert_ne!(status.load(Ordering::Acquire) & CAPTURE_FAILED, 0);
    }
}
````

### crates/whisper-adapters/src/staging.rs SHA256 cdbb8953032cccdc7e8401b763defcc452bef66e0da0e610b0db123f8a3e5507

````rust
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
````

### crates/whisper-worker-cpu/src/encoder.rs SHA256 25d59e138604f9d58a4a3f6952eee47d33a10104706b038ed7c19bb4b50680cd

````rust
use rusty_mp3::{Mp3Encoder, Mp3EncoderConfig};

pub struct LiveMp3Encoder {
    encoder: Mp3Encoder,
    finished: bool,
}

impl LiveMp3Encoder {
    pub fn new() -> Self {
        Self {
            encoder: Mp3Encoder::new(Mp3EncoderConfig {
                bitrate_kbps: 64,
                vbr_quality: None,
            }),
            finished: false,
        }
    }

    pub fn push_pcm(&mut self, samples: &[i16]) -> Result<Vec<Vec<u8>>, String> {
        if self.finished {
            return Err("InvalidRequest: live MP3 encoder is already finalized".into());
        }
        let mut packets = Vec::new();
        for chunk in samples.chunks(16_000) {
            self.encoder
                .push_pcm_s16(chunk, 1, 16_000)
                .map_err(|error| format!("MP3 encoding failed: {error}"))?;
            packets.extend(self.drain_packets()?);
        }
        Ok(packets)
    }

    pub fn finish(&mut self) -> Result<Vec<Vec<u8>>, String> {
        if !self.finished {
            self.encoder.finish();
            self.finished = true;
        }
        self.drain_packets()
    }

    fn drain_packets(&mut self) -> Result<Vec<Vec<u8>>, String> {
        let mut packets = Vec::new();
        loop {
            match self.encoder.next_packet() {
                Ok(packet) => packets.push(packet),
                Err(rusty_mp3::Error::Again | rusty_mp3::Error::Eof) => break,
                Err(error) => return Err(format!("MP3 encoding failed: {error}")),
            }
        }
        Ok(packets)
    }
}

impl Default for LiveMp3Encoder {
    fn default() -> Self {
        Self::new()
    }
}
````

### crates/whisper-worker-cpu/src/ipc.rs SHA256 1b06be095f4586f8e2cddc128fcc9e7307ab208d87323e18f97eddca97f06f97

````rust
use serde::{Serialize, de::DeserializeOwned};
use std::io::{self, BufRead, Write};

pub const MAX_FRAME_BYTES: usize = 1_048_576;

pub fn read_frame<T: DeserializeOwned>(input: &mut impl BufRead) -> io::Result<Option<T>> {
    let mut bytes = Vec::with_capacity(4096);
    loop {
        let available = input.fill_buf()?;
        if available.is_empty() {
            if bytes.is_empty() {
                return Ok(None);
            }
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "incomplete IPC frame",
            ));
        }
        let upto = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(available.len(), |i| i + 1);
        let new_len = bytes.len().saturating_add(upto);
        if new_len > MAX_FRAME_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "IPC frame exceeds 1 MiB",
            ));
        }
        let finished = available[upto - 1] == b'\n';
        bytes.extend_from_slice(&available[..upto]);
        input.consume(upto);
        if finished {
            break;
        }
    }
    #[cfg(feature = "l01-memory-qualification")]
    crate::memory_qualification::record(
        "ipc.read_frame.buffer",
        serde_json::json!({
            "frame_payload_len_bytes": bytes.len().saturating_sub(1),
            "frame_buffer_capacity_bytes": bytes.capacity(),
            "frame_limit_bytes_including_newline": MAX_FRAME_BYTES,
        }),
    );
    bytes.pop();
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

pub fn write_frame<T: Serialize>(output: &mut impl Write, value: &T) -> io::Result<()> {
    let bytes = serde_json::to_vec(value).map_err(io::Error::other)?;
    #[cfg(feature = "l01-memory-qualification")]
    crate::memory_qualification::record(
        "ipc.serialize.buffer",
        serde_json::json!({
            "serialized_payload_len_bytes": bytes.len(),
            "serialized_vec_capacity_bytes": bytes.capacity(),
            "frame_len_with_newline_bytes": bytes.len() + 1,
            "frame_limit_bytes": MAX_FRAME_BYTES,
        }),
    );
    if bytes.len() + 1 > MAX_FRAME_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "IPC frame exceeds 1 MiB",
        ));
    }
    output.write_all(&bytes)?;
    output.write_all(b"\n")?;
    output.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_oversized_and_incomplete_frames_before_deserialization() {
        let mut oversized = vec![b'a'; MAX_FRAME_BYTES];
        oversized.push(b'\n');
        assert_eq!(
            read_frame::<serde_json::Value>(&mut oversized.as_slice())
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidData
        );
        let mut incomplete = br#"{"version":2}"#.as_slice();
        assert_eq!(
            read_frame::<serde_json::Value>(&mut incomplete)
                .unwrap_err()
                .kind(),
            io::ErrorKind::UnexpectedEof
        );
    }

    #[test]
    fn rejects_serialized_frames_over_one_mibibyte() {
        let oversized = "x".repeat(MAX_FRAME_BYTES);
        assert_eq!(
            write_frame(&mut Vec::new(), &oversized).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }
}
````
