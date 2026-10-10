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
        let output = resampler.output_buffer_allocate(true);
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
        if !self.pending.is_empty() {
            let partial = [&self.pending[..]];
            let (_, produced) = self
                .resampler
                .process_partial_into_buffer(Some(&partial), &mut self.output, None)
                .map_err(|error| format!("CaptureConversion: final input pad failed: {error}"))?;
            self.pending.clear();
            self.collect_compensated(&mut result, produced, target);
        }
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
    fn empty_and_exact_chunk_capture_finish_without_an_empty_input_buffer() {
        let mut empty = Pcm16Converter::new(48_000, 1).unwrap();
        assert!(empty.finish().unwrap().is_empty());
        assert!(empty.finish().unwrap().is_empty());

        let mut complete = Pcm16Converter::new(48_000, 1).unwrap();
        let input_frames = complete.chunk;
        let emitted = complete
            .push(&CapturedBlock {
                sample_rate_hz: 48_000,
                channels: 1,
                samples: vec![0.0; input_frames],
            })
            .unwrap()
            .len();
        let tail = complete.finish().unwrap();
        assert_eq!(emitted + tail.len(), input_frames / 3);
    }

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
