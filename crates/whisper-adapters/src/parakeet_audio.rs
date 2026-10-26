//! Disk-backed, globally indexed PCM source for unified Parakeet sessions.
//!
//! WAV parts contain at most 1 GiB of mono 16 kHz PCM. The first part keeps
//! the requested basename; subsequent parts use `<stem>.part-NNNNNN.wav`.

use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, SyncSender, TrySendError},
    thread::{self, JoinHandle},
};
use symphonia::core::{
    audio::SampleBuffer,
    codecs::{Decoder, DecoderOptions},
    errors::Error as SymphoniaError,
    formats::{FormatOptions, FormatReader},
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint,
};

const PCM_RATE: u32 = 16_000;
const WAV_HEADER_BYTES: u64 = 44;
const PART_LIMIT_BYTES: u64 = 1 << 30;
const INDEX_HEADER: &[u8; 8] = b"PUAUD001";
const INDEX_RECORD_BYTES: u64 = 24;
const MAX_PAGE_RECORDS: usize = 512;
const MAX_DECODE_BLOCK_SAMPLES: usize = 80_000;
const MAX_WINDOW_SAMPLES: u64 = (120 * PCM_RATE as u64) + 2 * (200 * PCM_RATE as u64 / 1_000);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AudioPart {
    pub number: u64,
    pub start_sample: u64,
    pub end_sample: u64,
    pub path: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AudioStoreError {
    Io(String),
    InvalidRange,
    CorruptIndex,
    PartNumberExhausted,
    SampleOffsetOverflow,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CaptureWriteAck {
    pub sequence: u64,
    pub range: std::ops::Range<u64>,
    pub written_samples: u64,
    pub flushed_samples: u64,
    pub synchronized_samples: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CaptureWriterError {
    Saturated,
    Closed,
    OutOfOrder,
    InvalidRange,
    Audio(AudioStoreError),
}

enum CaptureWriterCommand {
    Append {
        sequence: u64,
        samples: Vec<i16>,
    },
    ReadWindow {
        range: std::ops::Range<u64>,
        reply: mpsc::SyncSender<Result<Vec<i16>, CaptureWriterError>>,
    },
    SnapshotEnd(mpsc::SyncSender<Result<u64, CaptureWriterError>>),
    Close(mpsc::SyncSender<Result<u64, CaptureWriterError>>),
    Shutdown,
}

/// A bounded callback-to-disk queue. The callback only calls `try_append`;
/// indexing, file IO and reads are serialized on the independent writer thread.
pub struct CapturePcmWriter {
    commands: SyncSender<CaptureWriterCommand>,
    acknowledgements: Receiver<Result<CaptureWriteAck, CaptureWriterError>>,
    thread: Option<JoinHandle<()>>,
    next_sequence: u64,
    closed_result: Option<Result<u64, CaptureWriterError>>,
}

impl CapturePcmWriter {
    pub fn start(base_path: impl AsRef<Path>, capacity: usize) -> Result<Self, AudioStoreError> {
        Self::start_with_limit(base_path, capacity, PART_LIMIT_BYTES)
    }

    fn start_with_limit(
        base_path: impl AsRef<Path>,
        capacity: usize,
        part_limit_bytes: u64,
    ) -> Result<Self, AudioStoreError> {
        if capacity == 0 {
            return Err(AudioStoreError::InvalidRange);
        }
        let base_path = base_path.as_ref().to_path_buf();
        let writer = PcmPartWriter::create_with_limit(&base_path, part_limit_bytes)?;
        let (commands, command_rx) = mpsc::sync_channel(capacity);
        let (ack_tx, acknowledgements) = mpsc::sync_channel(capacity);
        let thread = thread::Builder::new()
            .name("parakeet-pcm-writer".into())
            .spawn(move || {
                let mut writer = Some(writer);
                let mut next_sequence = 1_u64;
                let mut written_samples = 0_u64;
                let mut closed_samples = None;
                while let Ok(command) = command_rx.recv() {
                    match command {
                        CaptureWriterCommand::Append { sequence, samples } => {
                            let result = if sequence != next_sequence {
                                Err(CaptureWriterError::OutOfOrder)
                            } else {
                                writer
                                    .as_mut()
                                    .ok_or(AudioStoreError::InvalidRange)
                                    .and_then(|writer| writer.append(&samples))
                                    .map(|range| {
                                        next_sequence = next_sequence.saturating_add(1);
                                        written_samples = range.end;
                                        CaptureWriteAck {
                                            sequence,
                                            range,
                                            written_samples,
                                            flushed_samples: written_samples,
                                            synchronized_samples: 0,
                                        }
                                    })
                                    .map_err(CaptureWriterError::Audio)
                            };
                            if ack_tx.send(result).is_err() {
                                break;
                            }
                        }
                        CaptureWriterCommand::ReadWindow { range, reply } => {
                            let result = if range.end > written_samples {
                                Err(CaptureWriterError::InvalidRange)
                            } else {
                                PcmPartStore::open(&base_path)
                                    .read_window(range)
                                    .map_err(CaptureWriterError::Audio)
                            };
                            let _ = reply.send(result);
                        }
                        CaptureWriterCommand::SnapshotEnd(reply) => {
                            let _ = reply.send(Ok(written_samples));
                        }
                        CaptureWriterCommand::Close(reply) => {
                            let result = if let Some(writer) = writer.take() {
                                writer.finish().map_err(CaptureWriterError::Audio)
                            } else {
                                closed_samples.ok_or(CaptureWriterError::Closed)
                            };
                            if let Ok(total) = &result {
                                closed_samples = Some(*total);
                            }
                            let _ = reply.send(result);
                        }
                        CaptureWriterCommand::Shutdown => {
                            if let Some(writer) = writer.take() {
                                let _ = writer.finish();
                            }
                            break;
                        }
                    }
                }
            })?;
        Ok(Self {
            commands,
            acknowledgements,
            thread: Some(thread),
            next_sequence: 1,
            closed_result: None,
        })
    }

    #[cfg(test)]
    pub(crate) fn start_with_test_part_limit(
        base_path: impl AsRef<Path>,
        capacity: usize,
        part_limit_bytes: u64,
    ) -> Result<Self, AudioStoreError> {
        Self::start_with_limit(base_path, capacity, part_limit_bytes)
    }

    /// Nonblocking admission from the capture pump. Saturation is surfaced to
    /// the caller as an explicit loss incident; blocks are never silently dropped.
    pub fn try_append(&mut self, samples: &[i16]) -> Result<u64, CaptureWriterError> {
        if self.closed_result.is_some() {
            return Err(CaptureWriterError::Closed);
        }
        if samples.is_empty() || samples.len() > MAX_DECODE_BLOCK_SAMPLES {
            return Err(CaptureWriterError::InvalidRange);
        }
        let sequence = self.next_sequence;
        let command = CaptureWriterCommand::Append {
            sequence,
            samples: samples.to_vec(),
        };
        match self.commands.try_send(command) {
            Ok(()) => {
                self.next_sequence = sequence
                    .checked_add(1)
                    .ok_or(CaptureWriterError::OutOfOrder)?;
                Ok(sequence)
            }
            Err(TrySendError::Full(_)) => Err(CaptureWriterError::Saturated),
            Err(TrySendError::Disconnected(_)) => Err(CaptureWriterError::Closed),
        }
    }

    pub fn try_acknowledgement(&self) -> Result<Option<CaptureWriteAck>, CaptureWriterError> {
        match self.acknowledgements.try_recv() {
            Ok(result) => result.map(Some),
            Err(mpsc::TryRecvError::Empty) => Ok(None),
            Err(mpsc::TryRecvError::Disconnected) => Err(CaptureWriterError::Closed),
        }
    }

    pub fn submitted_blocks(&self) -> u64 {
        self.next_sequence.saturating_sub(1)
    }

    pub fn written_samples(&self) -> Result<u64, CaptureWriterError> {
        let (reply, result) = mpsc::sync_channel(1);
        self.commands
            .send(CaptureWriterCommand::SnapshotEnd(reply))
            .map_err(|_| CaptureWriterError::Closed)?;
        result.recv().map_err(|_| CaptureWriterError::Closed)?
    }

    /// Reads only acknowledged samples, with the writer thread serializing the
    /// index snapshot and file read against later appends and rotations.
    pub fn read_written_window(
        &self,
        range: std::ops::Range<u64>,
    ) -> Result<Vec<i16>, CaptureWriterError> {
        let (reply, result) = mpsc::sync_channel(1);
        self.commands
            .send(CaptureWriterCommand::ReadWindow { range, reply })
            .map_err(|_| CaptureWriterError::Closed)?;
        result.recv().map_err(|_| CaptureWriterError::Closed)?
    }

    pub fn close(&mut self) -> Result<u64, CaptureWriterError> {
        if let Some(result) = &self.closed_result {
            return result.clone();
        }
        let (reply, result) = mpsc::sync_channel(1);
        let mut write_error = None;
        loop {
            match self
                .commands
                .try_send(CaptureWriterCommand::Close(reply.clone()))
            {
                Ok(()) => break,
                Err(TrySendError::Full(_)) => {
                    self.drain_acknowledgements_for_close(&mut write_error);
                    thread::yield_now();
                }
                Err(TrySendError::Disconnected(_)) => return Err(CaptureWriterError::Closed),
            }
        }
        let total = loop {
            match result.try_recv() {
                Ok(total) => break total,
                Err(mpsc::TryRecvError::Empty) => {
                    self.drain_acknowledgements_for_close(&mut write_error);
                    thread::yield_now();
                }
                Err(mpsc::TryRecvError::Disconnected) => return Err(CaptureWriterError::Closed),
            }
        };
        let total = match total {
            Ok(total) => total,
            Err(error) => {
                self.closed_result = Some(Err(error.clone()));
                return Err(error);
            }
        };
        if let Some(error) = write_error {
            self.closed_result = Some(Err(error.clone()));
            return Err(error);
        }
        self.closed_result = Some(Ok(total));
        Ok(total)
    }

    fn drain_acknowledgements_for_close(&self, write_error: &mut Option<CaptureWriterError>) {
        loop {
            match self.acknowledgements.try_recv() {
                Ok(Err(error)) => {
                    write_error.get_or_insert(error);
                }
                Ok(Ok(_)) => {}
                Err(mpsc::TryRecvError::Empty | mpsc::TryRecvError::Disconnected) => break,
            }
        }
    }
}

impl Drop for CapturePcmWriter {
    fn drop(&mut self) {
        let mut ignored_error = None;
        loop {
            match self.commands.try_send(CaptureWriterCommand::Shutdown) {
                Ok(()) => break,
                Err(TrySendError::Full(_)) => {
                    self.drain_acknowledgements_for_close(&mut ignored_error);
                    thread::yield_now();
                }
                Err(TrySendError::Disconnected(_)) => break,
            }
        }
        if let Some(thread) = self.thread.take() {
            while !thread.is_finished() {
                self.drain_acknowledgements_for_close(&mut ignored_error);
                thread::yield_now();
            }
            let _ = thread.join();
        }
    }
}

impl std::fmt::Display for AudioStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Parakeet audio store: {self:?}")
    }
}
impl std::error::Error for AudioStoreError {}
impl From<std::io::Error> for AudioStoreError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

/// Append-only source writer. A completed part has a classic PCM WAV header;
/// the active part is finalized explicitly with `finish`.
pub struct PcmPartWriter {
    base_path: PathBuf,
    index_path: PathBuf,
    index: File,
    part: File,
    part_number: u64,
    part_start: u64,
    part_samples: u64,
    total_samples: u64,
    part_limit_bytes: u64,
    #[cfg(test)]
    fail_next_write: bool,
    finished: bool,
}

impl PcmPartWriter {
    pub fn create(base_path: impl AsRef<Path>) -> Result<Self, AudioStoreError> {
        Self::create_with_limit(base_path, PART_LIMIT_BYTES)
    }

    fn create_with_limit(
        base_path: impl AsRef<Path>,
        part_limit_bytes: u64,
    ) -> Result<Self, AudioStoreError> {
        if part_limit_bytes < 2 || !part_limit_bytes.is_multiple_of(2) {
            return Err(AudioStoreError::InvalidRange);
        }
        let base_path = base_path.as_ref().to_path_buf();
        let parent = base_path.parent().ok_or(AudioStoreError::InvalidRange)?;
        fs::create_dir_all(parent)?;
        let index_path = base_path.with_extension("parts.idx");
        let mut index = OpenOptions::new()
            .write(true)
            .read(true)
            .create_new(true)
            .open(&index_path)?;
        index.write_all(INDEX_HEADER)?;
        let part = OpenOptions::new()
            .write(true)
            .read(true)
            .create_new(true)
            .open(&base_path)?;
        let mut writer = Self {
            base_path,
            index_path,
            index,
            part,
            part_number: 1,
            part_start: 0,
            part_samples: 0,
            total_samples: 0,
            part_limit_bytes,
            #[cfg(test)]
            fail_next_write: false,
            finished: false,
        };
        writer.part.write_all(&[0_u8; WAV_HEADER_BYTES as usize])?;
        writer.write_record()?;
        Ok(writer)
    }

    pub fn written_samples(&self) -> u64 {
        self.total_samples
    }

    pub fn append(&mut self, mut samples: &[i16]) -> Result<std::ops::Range<u64>, AudioStoreError> {
        if self.finished {
            return Err(AudioStoreError::InvalidRange);
        }
        let start = self.total_samples;
        while !samples.is_empty() {
            let remaining_bytes = self
                .part_limit_bytes
                .saturating_sub(self.part_samples.saturating_mul(2));
            let capacity = (remaining_bytes / 2) as usize;
            if capacity == 0 {
                self.rotate()?;
                continue;
            }
            let count = capacity.min(samples.len()).min(MAX_DECODE_BLOCK_SAMPLES);
            let bytes = samples[..count]
                .iter()
                .flat_map(|sample| sample.to_le_bytes())
                .collect::<Vec<_>>();
            #[cfg(test)]
            if self.fail_next_write {
                self.fail_next_write = false;
                return Err(AudioStoreError::Io("injected part write failure".into()));
            }
            self.part.write_all(&bytes)?;
            self.part_samples = self
                .part_samples
                .checked_add(count as u64)
                .ok_or(AudioStoreError::SampleOffsetOverflow)?;
            self.total_samples = self
                .total_samples
                .checked_add(count as u64)
                .ok_or(AudioStoreError::SampleOffsetOverflow)?;
            self.write_record()?;
            samples = &samples[count..];
            if self.part_samples.saturating_mul(2) == self.part_limit_bytes && !samples.is_empty() {
                self.rotate()?;
            }
        }
        Ok(start..self.total_samples)
    }

    pub fn finish(mut self) -> Result<u64, AudioStoreError> {
        self.finalize_part()?;
        self.index.sync_all()?;
        self.finished = true;
        Ok(self.total_samples)
    }

    fn rotate(&mut self) -> Result<(), AudioStoreError> {
        self.finalize_part()?;
        let next_number = self
            .part_number
            .checked_add(1)
            .ok_or(AudioStoreError::PartNumberExhausted)?;
        let next_path = part_path(&self.base_path, next_number);
        let mut next_part = OpenOptions::new()
            .write(true)
            .read(true)
            .create_new(true)
            .open(next_path)?;
        next_part.write_all(&[0_u8; WAV_HEADER_BYTES as usize])?;
        self.write_record_values(next_number, self.total_samples, self.total_samples)?;
        self.part = next_part;
        self.part_number = next_number;
        self.part_start = self.total_samples;
        self.part_samples = 0;
        Ok(())
    }

    fn finalize_part(&mut self) -> Result<(), AudioStoreError> {
        if self.part_samples.saturating_mul(2) > self.part_limit_bytes {
            return Err(AudioStoreError::InvalidRange);
        }
        let data_bytes =
            u32::try_from(self.part_samples * 2).map_err(|_| AudioStoreError::InvalidRange)?;
        let mut header = Vec::with_capacity(WAV_HEADER_BYTES as usize);
        header.extend_from_slice(b"RIFF");
        header.extend_from_slice(&(36_u32 + data_bytes).to_le_bytes());
        header.extend_from_slice(b"WAVEfmt ");
        header.extend_from_slice(&16_u32.to_le_bytes());
        header.extend_from_slice(&1_u16.to_le_bytes());
        header.extend_from_slice(&1_u16.to_le_bytes());
        header.extend_from_slice(&PCM_RATE.to_le_bytes());
        header.extend_from_slice(&(PCM_RATE * 2).to_le_bytes());
        header.extend_from_slice(&2_u16.to_le_bytes());
        header.extend_from_slice(&16_u16.to_le_bytes());
        header.extend_from_slice(b"data");
        header.extend_from_slice(&data_bytes.to_le_bytes());
        self.part.seek(SeekFrom::Start(0))?;
        self.part.write_all(&header)?;
        self.part.sync_all()?;
        self.write_record()
    }

    fn write_record(&mut self) -> Result<(), AudioStoreError> {
        self.write_record_values(self.part_number, self.part_start, self.total_samples)
    }

    fn write_record_values(
        &mut self,
        number: u64,
        start: u64,
        end: u64,
    ) -> Result<(), AudioStoreError> {
        let offset = INDEX_HEADER.len() as u64 + (number - 1) * INDEX_RECORD_BYTES;
        self.index.seek(SeekFrom::Start(offset))?;
        self.index.write_all(&number.to_le_bytes())?;
        self.index.write_all(&start.to_le_bytes())?;
        self.index.write_all(&end.to_le_bytes())?;
        self.index.flush()?;
        Ok(())
    }
}

/// Bounded index reader and random-access PCM source.
pub struct PcmPartStore {
    base_path: PathBuf,
    index_path: PathBuf,
}

/// Stateful linear conversion with an explicit output length of
/// `ceil(input_frames * 16_000 / input_rate)`. EOF extends the last sample only
/// far enough to fill that length; no packet boundary resets the timeline.
struct LinearPcmResampler {
    input_rate: u32,
    input_frames: u64,
    next_output: u64,
    previous: Option<f32>,
    finished: bool,
}

impl LinearPcmResampler {
    fn new(input_rate: u32) -> Self {
        Self {
            input_rate,
            input_frames: 0,
            next_output: 0,
            previous: None,
            finished: false,
        }
    }

    fn push(&mut self, current: f32) -> Result<Vec<i16>, String> {
        if self.finished {
            return Err("DecodeAudio: resampler already drained".into());
        }
        let index = self.input_frames;
        self.input_frames = self
            .input_frames
            .checked_add(1)
            .ok_or_else(|| "AudioTooLong: source sample count overflow".to_owned())?;
        let current = current.clamp(-1.0, 1.0);
        let mut output = Vec::new();
        if let Some(previous) = self.previous {
            while (self.next_output as u128) * u128::from(self.input_rate)
                <= (index as u128) * u128::from(PCM_RATE)
            {
                let numerator = (self.next_output as u128) * u128::from(self.input_rate);
                let base = (numerator / u128::from(PCM_RATE)) as u64;
                let fraction = (numerator % u128::from(PCM_RATE)) as f32 / PCM_RATE as f32;
                let sample = if base == index {
                    current
                } else {
                    previous + (current - previous) * fraction
                };
                output.push((sample.clamp(-1.0, 1.0) * 32767.0).round() as i16);
                self.next_output = self
                    .next_output
                    .checked_add(1)
                    .ok_or_else(|| "AudioTooLong: output sample count overflow".to_owned())?;
            }
        } else {
            output.push((current * 32767.0).round() as i16);
            self.next_output = 1;
        }
        self.previous = Some(current);
        Ok(output)
    }

    fn finish(&mut self) -> Result<Vec<i16>, String> {
        if self.finished {
            return Ok(Vec::new());
        }
        self.finished = true;
        let target = ((self.input_frames as u128) * u128::from(PCM_RATE))
            .div_ceil(u128::from(self.input_rate));
        let target = u64::try_from(target)
            .map_err(|_| "AudioTooLong: output sample count overflow".to_owned())?;
        let last = self
            .previous
            .ok_or_else(|| "DecodeAudio: no samples".to_owned())?;
        let count = target.saturating_sub(self.next_output);
        let capacity = usize::try_from(count)
            .map_err(|_| "AudioTooLong: EOF drain exceeds address space".to_owned())?;
        let sample = (last * 32767.0).round() as i16;
        self.next_output = target;
        Ok(vec![sample; capacity])
    }
}

impl PcmPartStore {
    pub fn open(base_path: impl AsRef<Path>) -> Self {
        let base_path = base_path.as_ref().to_path_buf();
        let index_path = base_path.with_extension("parts.idx");
        Self {
            base_path,
            index_path,
        }
    }

    pub fn base_path(&self) -> &Path {
        &self.base_path
    }

    pub fn read_page(&self, offset: u64, limit: usize) -> Result<Vec<AudioPart>, AudioStoreError> {
        let mut index = File::open(&self.index_path)?;
        let mut header = [0_u8; 8];
        index.read_exact(&mut header)?;
        if &header != INDEX_HEADER {
            return Err(AudioStoreError::CorruptIndex);
        }
        let count = (index.metadata()?.len().saturating_sub(8)) / INDEX_RECORD_BYTES;
        if (index.metadata()?.len().saturating_sub(8)) % INDEX_RECORD_BYTES != 0 {
            return Err(AudioStoreError::CorruptIndex);
        }
        let page_limit = limit.min(MAX_PAGE_RECORDS) as u64;
        if offset >= count || page_limit == 0 {
            return Ok(Vec::new());
        }
        index.seek(SeekFrom::Start(8 + offset * INDEX_RECORD_BYTES))?;
        let mut page = Vec::with_capacity(page_limit as usize);
        for _ in 0..page_limit.min(count - offset) {
            let number = read_u64(&mut index)?;
            let start_sample = read_u64(&mut index)?;
            let end_sample = read_u64(&mut index)?;
            if number != offset + page.len() as u64 + 1 || start_sample > end_sample {
                return Err(AudioStoreError::CorruptIndex);
            }
            page.push(AudioPart {
                number,
                start_sample,
                end_sample,
                path: part_path(&self.base_path, number),
            });
        }
        Ok(page)
    }

    pub fn total_samples(&self) -> Result<u64, AudioStoreError> {
        let mut offset = 0_u64;
        let mut covered = 0_u64;
        loop {
            let page = self.read_page(offset, MAX_PAGE_RECORDS)?;
            if page.is_empty() {
                return Ok(covered);
            }
            for part in &page {
                if part.start_sample != covered || part.end_sample < part.start_sample {
                    return Err(AudioStoreError::CorruptIndex);
                }
                covered = part.end_sample;
            }
            offset = offset
                .checked_add(page.len() as u64)
                .ok_or(AudioStoreError::PartNumberExhausted)?;
        }
    }

    pub fn read_range(&self, range: std::ops::Range<u64>) -> Result<Vec<i16>, AudioStoreError> {
        if range.start > range.end {
            return Err(AudioStoreError::InvalidRange);
        }
        let capacity = usize::try_from(range.end - range.start)
            .map_err(|_| AudioStoreError::SampleOffsetOverflow)?;
        if capacity > MAX_DECODE_BLOCK_SAMPLES {
            return Err(AudioStoreError::InvalidRange);
        }
        self.read_bounded(range, capacity)
    }

    /// Reads one native inference window. The allocation is bounded to 120.4 s.
    pub fn read_window(&self, range: std::ops::Range<u64>) -> Result<Vec<i16>, AudioStoreError> {
        if range.start > range.end || range.end - range.start > MAX_WINDOW_SAMPLES {
            return Err(AudioStoreError::InvalidRange);
        }
        let capacity = usize::try_from(range.end - range.start)
            .map_err(|_| AudioStoreError::SampleOffsetOverflow)?;
        self.read_bounded(range, capacity)
    }

    fn read_bounded(
        &self,
        range: std::ops::Range<u64>,
        capacity: usize,
    ) -> Result<Vec<i16>, AudioStoreError> {
        let mut output = Vec::with_capacity(capacity);
        let mut offset = 0_u64;
        loop {
            let page = self.read_page(offset, MAX_PAGE_RECORDS)?;
            if page.is_empty() {
                break;
            }
            for part in &page {
                let start = range.start.max(part.start_sample);
                let end = range.end.min(part.end_sample);
                if start >= end {
                    continue;
                }
                let mut file = File::open(&part.path)?;
                let pcm_offset = WAV_HEADER_BYTES
                    .checked_add(
                        (start - part.start_sample)
                            .checked_mul(2)
                            .ok_or(AudioStoreError::SampleOffsetOverflow)?,
                    )
                    .ok_or(AudioStoreError::SampleOffsetOverflow)?;
                file.seek(SeekFrom::Start(pcm_offset))?;
                let bytes_len = usize::try_from((end - start) * 2)
                    .map_err(|_| AudioStoreError::SampleOffsetOverflow)?;
                let mut bytes = vec![0_u8; bytes_len];
                file.read_exact(&mut bytes)?;
                output.extend(
                    bytes
                        .chunks_exact(2)
                        .map(|pair| i16::from_le_bytes([pair[0], pair[1]])),
                );
            }
            offset += page.len() as u64;
            if offset > 0 && page.last().is_some_and(|part| part.end_sample >= range.end) {
                break;
            }
        }
        if output.len() != capacity {
            return Err(AudioStoreError::InvalidRange);
        }
        Ok(output)
    }
}

/// Decodes WAV/MP3 once from start to EOF and writes bounded mono 16 kHz
/// blocks to the part store. Linear interpolation matches the existing
/// Parakeet decoder's input conversion while retaining state across packets.
pub fn decode_file_to_parts(
    source_path: impl AsRef<Path>,
    output: &mut PcmPartWriter,
) -> Result<u64, String> {
    decode_file_to_parts_with(source_path, output, || false)
}

/// Variant used by the import lifecycle to observe cancellation between input
/// packets and bounded output writes.
pub fn decode_file_to_parts_with(
    source_path: impl AsRef<Path>,
    output: &mut PcmPartWriter,
    mut cancelled: impl FnMut() -> bool,
) -> Result<u64, String> {
    let file =
        File::open(source_path.as_ref()).map_err(|error| format!("SourceMissing: {error}"))?;
    let stream = MediaSourceStream::new(Box::new(file), Default::default());
    let probed = symphonia::default::get_probe()
        .format(
            &Hint::new(),
            stream,
            &FormatOptions {
                enable_gapless: true,
                ..Default::default()
            },
            &MetadataOptions::default(),
        )
        .map_err(|error| format!("UnsupportedFormat: {error}"))?;
    let mut format: Box<dyn FormatReader> = probed.format;
    let track = format
        .default_track()
        .ok_or_else(|| "UnsupportedFormat: no default audio track".to_owned())?;
    let track_id = track.id;
    let params = track.codec_params.clone();
    let input_rate = params
        .sample_rate
        .ok_or_else(|| "UnsupportedFormat: missing sample rate".to_owned())?;
    let channels = params
        .channels
        .ok_or_else(|| "UnsupportedFormat: missing channels".to_owned())?
        .count();
    if input_rate == 0 || channels == 0 || channels > 32 {
        return Err("UnsupportedFormat: unsupported audio layout".into());
    }
    let mut decoder: Box<dyn Decoder> = symphonia::default::get_codecs()
        .make(&params, &DecoderOptions::default())
        .map_err(|error| format!("DecoderInit: {error}"))?;

    let mut output_samples = Vec::with_capacity(MAX_DECODE_BLOCK_SAMPLES);
    let mut resampler = LinearPcmResampler::new(input_rate);
    loop {
        if cancelled() {
            return Err("Cancelled: source decoding stopped between packets".into());
        }
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(SymphoniaError::IoError(error))
                if error.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(error) => return Err(format!("DecodePacket: {error}")),
        };
        if packet.track_id() != track_id {
            continue;
        }
        let decoded = decoder
            .decode(&packet)
            .map_err(|error| format!("DecodeAudio: {error}"))?;
        if decoded.spec().rate != input_rate || decoded.spec().channels.count() != channels {
            return Err("UnsupportedFormat: audio format changes within the source".into());
        }
        let mut buffer = SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
        buffer.copy_interleaved_ref(decoded);
        for frame in buffer.samples().chunks_exact(channels) {
            let current = frame.iter().copied().sum::<f32>() / channels as f32;
            output_samples.extend(resampler.push(current)?);
            if output_samples.len() >= MAX_DECODE_BLOCK_SAMPLES {
                if cancelled() {
                    return Err(
                        "Cancelled: source decoding stopped before the next bounded write".into(),
                    );
                }
                output
                    .append(&output_samples)
                    .map_err(|error| format!("StorageUnavailable: {error}"))?;
                output_samples.clear();
            }
        }
    }
    output_samples.extend(resampler.finish()?);
    if !output_samples.is_empty() {
        for block in output_samples.chunks(MAX_DECODE_BLOCK_SAMPLES) {
            if cancelled() {
                return Err("Cancelled: source decoding stopped before final bounded write".into());
            }
            output
                .append(block)
                .map_err(|error| format!("StorageUnavailable: {error}"))?;
        }
    }
    Ok(output.written_samples())
}

fn part_path(base: &Path, number: u64) -> PathBuf {
    if number == 1 {
        return base.to_path_buf();
    }
    let stem = base
        .file_stem()
        .and_then(|part| part.to_str())
        .unwrap_or("audio");
    let extension = base
        .extension()
        .and_then(|part| part.to_str())
        .unwrap_or("wav");
    base.with_file_name(format!("{stem}.part-{number:06}.{extension}"))
}

fn read_u64(reader: &mut impl Read) -> Result<u64, AudioStoreError> {
    let mut bytes = [0_u8; 8];
    reader.read_exact(&mut bytes)?;
    Ok(u64::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir() -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("whisper-parakeet-audio-{stamp}"))
    }

    #[test]
    fn capture_writer_acknowledges_written_blocks_and_reads_a_serialized_snapshot() {
        let root = temp_dir();
        let path = root.join("capture.wav");
        let mut writer = CapturePcmWriter::start(&path, 2).unwrap();
        assert_eq!(writer.try_append(&[10, 11, 12]).unwrap(), 1);
        assert_eq!(writer.try_append(&[13, 14, 15]).unwrap(), 2);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let mut acknowledgements = Vec::new();
        while acknowledgements.len() < 2 && std::time::Instant::now() < deadline {
            if let Some(ack) = writer.try_acknowledgement().unwrap() {
                acknowledgements.push(ack);
            } else {
                std::thread::yield_now();
            }
        }
        assert_eq!(
            acknowledgements.len(),
            2,
            "writer must acknowledge every block"
        );
        assert_eq!(acknowledgements[0].range, 0..3);
        assert_eq!(acknowledgements[1].range, 3..6);
        assert_eq!(acknowledgements[1].written_samples, 6);
        assert_eq!(acknowledgements[1].flushed_samples, 6);
        assert_eq!(acknowledgements[1].synchronized_samples, 0);
        assert_eq!(
            writer.read_written_window(0..6).unwrap(),
            vec![10, 11, 12, 13, 14, 15]
        );
        assert_eq!(writer.close().unwrap(), 6);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn stored_audio_is_contiguous_and_range_reads_are_bounded() {
        let root = temp_dir();
        let path = root.join("capture.wav");
        let mut writer = PcmPartWriter::create(&path).unwrap();
        assert_eq!(
            writer
                .append(&(0..1000).map(|sample| sample as i16).collect::<Vec<_>>())
                .unwrap(),
            0..1000
        );
        assert_eq!(writer.finish().unwrap(), 1000);
        let store = PcmPartStore::open(&path);
        let parts = store.read_page(0, 1).unwrap();
        assert_eq!(parts.len(), 1);
        assert_eq!((parts[0].start_sample, parts[0].end_sample), (0, 1000));
        assert_eq!(
            store.read_range(100..105).unwrap(),
            vec![100, 101, 102, 103, 104]
        );
        assert_eq!(
            store.read_range(0..80_001),
            Err(AudioStoreError::InvalidRange)
        );
        assert!(
            PcmPartWriter::create(&path).is_err(),
            "part creation must be exclusive"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn linear_resampler_drains_a_short_44100_source_without_resetting_at_packet_boundary() {
        let mut resampler = LinearPcmResampler::new(44_100);
        let mut output = Vec::new();
        // The first input packet ends after sample 0; state is retained while
        // the next packet supplies the remaining two source samples.
        for sample in [-1.0_f32] {
            output.extend(resampler.push(sample).unwrap());
        }
        for sample in [0.25_f32, 1.0] {
            output.extend(resampler.push(sample).unwrap());
        }
        output.extend(resampler.finish().unwrap());
        assert_eq!(output.len(), 2);
        assert_eq!(output[0], -32_767);
        assert_eq!(
            output[1], 32_767,
            "EOF drain extends the final value to the chosen output length"
        );
    }

    #[test]
    fn part_rotation_preserves_every_sample_across_a_test_only_small_boundary() {
        let root = temp_dir();
        let path = root.join("capture.wav");
        let mut writer = PcmPartWriter::create_with_limit(&path, 8).unwrap();
        writer.append(&[10, 11, 12]).unwrap();
        writer.append(&[13, 14, 15]).unwrap();
        assert_eq!(writer.finish().unwrap(), 6);
        let store = PcmPartStore::open(&path);
        let parts = store.read_page(0, 10).unwrap();
        assert_eq!(parts.len(), 2);
        assert_eq!((parts[0].start_sample, parts[0].end_sample), (0, 4));
        assert_eq!((parts[1].start_sample, parts[1].end_sample), (4, 6));
        assert_eq!(store.read_range(2..6).unwrap(), vec![12, 13, 14, 15]);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn capture_writer_reads_every_acknowledged_sample_across_test_rotation() {
        let root = temp_dir();
        let path = root.join("capture.wav");
        let mut writer = CapturePcmWriter::start_with_limit(&path, 2, 8).unwrap();
        let first = writer.try_append(&[10, 11, 12]).unwrap();
        let second = writer.try_append(&[13, 14, 15]).unwrap();
        assert_eq!(first, 1);
        assert_eq!(second, 2);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let mut acknowledgements = Vec::new();
        while acknowledgements.len() < 2 {
            if let Some(ack) = writer.try_acknowledgement().unwrap() {
                acknowledgements.push(ack);
            } else {
                assert!(
                    std::time::Instant::now() < deadline,
                    "writer acknowledgement timed out"
                );
                thread::yield_now();
            }
        }
        assert_eq!(acknowledgements[0].range, 0..3);
        assert_eq!(acknowledgements[0].written_samples, 3);
        assert_eq!(acknowledgements[1].range, 3..6);
        assert_eq!(acknowledgements[1].written_samples, 6);
        assert_eq!(
            writer.read_written_window(0..6).unwrap(),
            vec![10, 11, 12, 13, 14, 15]
        );
        assert_eq!(writer.close().unwrap(), 6);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_part_creation_and_write_keep_the_written_prefix_readable() {
        let root = temp_dir();
        let path = root.join("capture.wav");
        let mut writer = PcmPartWriter::create_with_limit(&path, 8).unwrap();
        writer.append(&[1, 2, 3, 4]).unwrap();
        let next_part = part_path(&path, 2);
        File::create(&next_part).unwrap();
        assert!(
            writer.append(&[5]).is_err(),
            "a colliding rotated part must fail closed"
        );
        assert_eq!(writer.written_samples(), 4);
        assert_eq!(writer.finish().unwrap(), 4);
        let store = PcmPartStore::open(&path);
        assert_eq!(store.read_range(0..4).unwrap(), vec![1, 2, 3, 4]);
        fs::remove_dir_all(root).unwrap();

        let root = temp_dir();
        let path = root.join("write.wav");
        let mut writer = PcmPartWriter::create_with_limit(&path, 8).unwrap();
        writer.append(&[6, 7]).unwrap();
        writer.fail_next_write = true;
        assert!(writer.append(&[8]).is_err());
        assert_eq!(writer.written_samples(), 2);
        writer.finish().unwrap();
        assert_eq!(
            PcmPartStore::open(&path).read_range(0..2).unwrap(),
            vec![6, 7]
        );
        fs::remove_dir_all(root).unwrap();
    }
}
