use std::{
    collections::VecDeque,
    sync::{
        Arc, RwLock,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc::{self, SyncSender, TrySendError},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use rustfft::{FftPlanner, num_complex::Complex};
use whisper_core::{
    Generation, JobId,
    microphone_level::{
        GainRevision, LevelMeasurement, MeterLevel, MicrophoneMeter, MicrophoneSpectrum,
        MicrophoneSpectrumUpdate, SpectrumBin, monotonic_millis, monotonic_millis_at,
    },
};

use crate::capture::ProbeAudioBlock;

const PROBE_QUEUE_BLOCKS: usize = 4;
const MAX_BLOCK_DURATION_MS: u64 = 20;
const RMS_WINDOW_MS: u64 = 100;
const RMS_MAX_RATE_HZ: u64 = 20;
const PSD_MAX_RATE_HZ: u64 = 10;
const FFT_SIZE: usize = 2048;
const SNAPSHOT_STALE_AFTER: Duration = Duration::from_millis(500);

#[derive(Clone, Debug)]
pub struct SpectralDensity {
    pub sample_rate_hz: u32,
    pub first_frame: u64,
    pub last_frame: u64,
    /// One-sided PSD in FS²/Hz, including DC and Nyquist.
    pub bins: Vec<f64>,
}

#[derive(Clone, Debug)]
pub struct ProbeSnapshot {
    pub job_id: JobId,
    pub generation: Generation,
    pub source_identity: Option<String>,
    pub sequence: u64,
    pub first_frame: u64,
    pub last_frame: u64,
    pub rms_first_frame: u64,
    pub rms_last_frame: u64,
    pub captured_at: Instant,
    pub captured_at_monotonic_ms: u64,
    pub published_at: Instant,
    pub gain_first: GainRevision,
    pub gain_last: GainRevision,
    pub mixed_gain: bool,
    pub pre_gain: Option<LevelMeasurement>,
    pub post_gain_limiter: Option<LevelMeasurement>,
    pub limiter_samples: u64,
    pub dropped_updates: u64,
    pub sample_rate_hz: u32,
    pub analyzed_channels: u16,
    pub pre_gain_psd: Option<SpectralDensity>,
    pub post_gain_limiter_psd: Option<SpectralDensity>,
    /// Latest FFT pair; RMS-only publications retain its original timestamp.
    pub spectrum: Option<CachedSpectrum>,
    epoch: u64,
}

#[derive(Clone, Debug)]
pub struct CachedSpectrum {
    pub sequence: u64,
    pub epoch: u64,
    pub captured_at: Instant,
    pub captured_at_monotonic_ms: u64,
    pub gain_first: GainRevision,
    pub gain_last: GainRevision,
    pub mixed_gain: bool,
    pub pre_gain: SpectralDensity,
    pub post_gain_limiter: SpectralDensity,
}

/// Per-consumer cursor. It emits only the newest pair and one invalidation
/// when a previously visible pair becomes stale or its epoch is lost.
#[derive(Default)]
pub struct MicrophoneSpectrumReader {
    last_epoch: Option<u64>,
    last_sequence: u64,
    visible: bool,
}

impl MicrophoneSpectrumReader {
    pub fn poll(
        &mut self,
        probe: &MicrophoneProbe,
        now: Instant,
    ) -> Option<MicrophoneSpectrumUpdate> {
        let epoch = probe.epoch();
        let spectrum = probe.latest_spectrum_at(now);
        if let Some(spectrum) = spectrum {
            if self.last_epoch != Some(spectrum.epoch) {
                let was_visible = self.visible;
                self.last_epoch = Some(spectrum.epoch);
                self.last_sequence = 0;
                self.visible = false;
                if was_visible {
                    return Some(MicrophoneSpectrumUpdate::Unavailable {
                        job_id: probe.job_id,
                        generation: probe.generation,
                        source_identity: probe.source_identity.clone()?,
                        sequence: spectrum.sequence,
                        epoch: spectrum.epoch,
                        dropped_updates: probe.dropped_updates(),
                        captured_at_monotonic_ms: monotonic_millis(),
                    });
                }
            }
            if spectrum.sequence > self.last_sequence {
                self.last_sequence = spectrum.sequence;
                self.visible = true;
                let source_identity = probe.source_identity.clone()?;
                let convert = |density: &SpectralDensity| {
                    density
                        .bins
                        .iter()
                        .copied()
                        .map(SpectrumBin::from_power)
                        .collect::<Option<Vec<_>>>()
                };
                let (Some(pre_gain), Some(post_gain_limiter)) = (
                    convert(&spectrum.pre_gain),
                    convert(&spectrum.post_gain_limiter),
                ) else {
                    self.visible = false;
                    return Some(MicrophoneSpectrumUpdate::Unavailable {
                        job_id: probe.job_id,
                        generation: probe.generation,
                        source_identity,
                        sequence: spectrum.sequence,
                        epoch: spectrum.epoch,
                        dropped_updates: probe.dropped_updates(),
                        captured_at_monotonic_ms: monotonic_millis(),
                    });
                };
                return Some(MicrophoneSpectrumUpdate::Available(MicrophoneSpectrum {
                    job_id: probe.job_id,
                    generation: probe.generation,
                    source_identity,
                    sequence: spectrum.sequence,
                    epoch: spectrum.epoch,
                    first_frame: spectrum.pre_gain.first_frame,
                    last_frame: spectrum.pre_gain.last_frame,
                    sample_rate_hz: spectrum.pre_gain.sample_rate_hz,
                    captured_at_monotonic_ms: spectrum.captured_at_monotonic_ms,
                    gain_first_db: spectrum.gain_first.gain.value(),
                    gain_first_revision: spectrum.gain_first.revision,
                    gain_last_db: spectrum.gain_last.gain.value(),
                    gain_last_revision: spectrum.gain_last.revision,
                    mixed_gain: spectrum.mixed_gain,
                    dropped_updates: probe.dropped_updates(),
                    pre_gain,
                    post_gain_limiter,
                }));
            }
            // A fresh snapshot remains the current graph until a newer FFT
            // replaces it. Re-polling the same publication is not a loss of
            // spectrum; only a missing/stale snapshot below invalidates it.
            return None;
        }
        if self.visible {
            self.visible = false;
            self.last_epoch = Some(epoch);
            return Some(MicrophoneSpectrumUpdate::Unavailable {
                job_id: probe.job_id,
                generation: probe.generation,
                source_identity: probe.source_identity.clone()?,
                sequence: self.last_sequence,
                epoch,
                dropped_updates: probe.dropped_updates(),
                captured_at_monotonic_ms: monotonic_millis(),
            });
        }
        self.last_epoch = Some(epoch);
        None
    }
}

#[derive(Clone)]
struct ProbeChunk {
    epoch: u64,
    first_frame: u64,
    acquired_at: Instant,
    gain: GainRevision,
    pre_gain: Vec<f32>,
    post_gain_limiter: Vec<f32>,
    limiter_mask: Vec<bool>,
}

pub struct MicrophoneProbe {
    sample_rate_hz: u32,
    chunk_samples: usize,
    sender: Option<SyncSender<ProbeChunk>>,
    epoch: Arc<AtomicU64>,
    epoch_started_at: Arc<RwLock<Instant>>,
    dropped_updates: Arc<AtomicU64>,
    valid: Arc<AtomicBool>,
    latest: Arc<RwLock<Option<Arc<ProbeSnapshot>>>>,
    job_id: JobId,
    generation: Generation,
    source_identity: Option<String>,
    worker: Option<JoinHandle<()>>,
    #[cfg(test)]
    processed_chunks: Arc<AtomicU64>,
}

impl MicrophoneProbe {
    pub fn start(
        sample_rate_hz: u32,
        source_identity: Option<String>,
        job_id: JobId,
        generation: Generation,
    ) -> Result<Self, String> {
        if sample_rate_hz < 16_000 {
            return Err("MicrophoneProbe: sample rate below 16 kHz".into());
        }
        let (sender, receiver) = mpsc::sync_channel::<ProbeChunk>(PROBE_QUEUE_BLOCKS);
        let epoch = Arc::new(AtomicU64::new(0));
        let epoch_started_at = Arc::new(RwLock::new(Instant::now()));
        let dropped_updates = Arc::new(AtomicU64::new(0));
        let valid = Arc::new(AtomicBool::new(false));
        let latest = Arc::new(RwLock::new(None));
        let worker_epoch = epoch.clone();
        let worker_dropped = dropped_updates.clone();
        let worker_valid = valid.clone();
        let worker_latest = latest.clone();
        let probe_source_identity = source_identity.clone();
        #[cfg(test)]
        let processed_chunks = Arc::new(AtomicU64::new(0));
        #[cfg(test)]
        let worker_processed_chunks = processed_chunks.clone();
        let worker = thread::Builder::new()
            .name("whisper-microphone-probe".into())
            .spawn(move || {
                let mut analyzer =
                    ProbeAnalyzer::new(sample_rate_hz, source_identity, job_id, generation);
                while let Ok(chunk) = receiver.recv() {
                    let current_epoch = worker_epoch.load(Ordering::Acquire);
                    if chunk.epoch != current_epoch {
                        analyzer.reset(chunk.epoch, chunk.first_frame);
                        worker_valid.store(false, Ordering::Release);
                        #[cfg(test)]
                        worker_processed_chunks.fetch_add(1, Ordering::Release);
                        continue;
                    }
                    let Some(snapshot) =
                        analyzer.push(chunk, worker_dropped.load(Ordering::Relaxed))
                    else {
                        #[cfg(test)]
                        worker_processed_chunks.fetch_add(1, Ordering::Release);
                        continue;
                    };
                    if analyzer.epoch == worker_epoch.load(Ordering::Acquire) {
                        if let Ok(mut latest) = worker_latest.write() {
                            *latest = Some(Arc::new(snapshot));
                            worker_valid.store(true, Ordering::Release);
                        }
                    }
                    #[cfg(test)]
                    worker_processed_chunks.fetch_add(1, Ordering::Release);
                }
            })
            .map_err(|error| format!("MicrophoneProbe: analyzer thread unavailable: {error}"))?;
        Ok(Self {
            sample_rate_hz,
            chunk_samples: ((u64::from(sample_rate_hz) * MAX_BLOCK_DURATION_MS).div_ceil(1000)
                as usize)
                .max(1),
            sender: Some(sender),
            epoch,
            epoch_started_at,
            dropped_updates,
            valid,
            latest,
            job_id,
            generation,
            source_identity: probe_source_identity,
            worker: Some(worker),
            #[cfg(test)]
            processed_chunks,
        })
    }

    /// Splits probe input into bounded chunks and never waits for the analyzer.
    pub fn submit(&self, block: ProbeAudioBlock) {
        let count = block.mono_pre_gain.len();
        if block.sample_rate_hz != self.sample_rate_hz
            || block.mono_post_gain_limiter.len() != count
            || block.limiter_mask.len() != count
        {
            self.mark_dropped();
            return;
        }
        let epoch_started_at = match self.epoch_started_at.read() {
            Ok(value) => *value,
            Err(_) => {
                self.mark_dropped();
                return;
            }
        };
        if block.acquired_at < epoch_started_at {
            self.dropped_updates.fetch_add(1, Ordering::Relaxed);
            return;
        }
        for start in (0..count).step_by(self.chunk_samples) {
            let end = (start + self.chunk_samples).min(count);
            let epoch = self.epoch.load(Ordering::Acquire);
            let chunk = ProbeChunk {
                epoch,
                first_frame: block.first_frame.saturating_add(start as u64),
                acquired_at: block.acquired_at,
                gain: block.gain,
                pre_gain: block.mono_pre_gain[start..end].to_vec(),
                post_gain_limiter: block.mono_post_gain_limiter[start..end].to_vec(),
                limiter_mask: block.limiter_mask[start..end].to_vec(),
            };
            let Some(sender) = self.sender.as_ref() else {
                return;
            };
            match sender.try_send(chunk) {
                Ok(()) => {}
                Err(TrySendError::Full(_)) => self.mark_dropped(),
                Err(TrySendError::Disconnected(_)) => {
                    self.mark_dropped();
                    return;
                }
            }
        }
    }

    pub fn latest_at(&self, now: Instant) -> Option<Arc<ProbeSnapshot>> {
        if !self.valid.load(Ordering::Acquire) {
            return None;
        }
        let snapshot = self.latest.read().ok()?.as_ref()?.clone();
        if snapshot.epoch_is_current(self.epoch.load(Ordering::Acquire))
            && now
                .checked_duration_since(snapshot.captured_at)
                .is_some_and(|age| age <= SNAPSHOT_STALE_AFTER)
        {
            Some(snapshot)
        } else {
            None
        }
    }

    pub fn latest_spectrum_at(&self, now: Instant) -> Option<CachedSpectrum> {
        if !self.valid.load(Ordering::Acquire) {
            return None;
        }
        let snapshot = self.latest.read().ok()?.as_ref()?.clone();
        let spectrum = snapshot.spectrum.clone()?;
        (spectrum.epoch == self.epoch.load(Ordering::Acquire)
            && now
                .checked_duration_since(spectrum.captured_at)
                .is_some_and(|age| age <= SNAPSHOT_STALE_AFTER))
        .then_some(spectrum)
    }

    pub fn epoch(&self) -> u64 {
        self.epoch.load(Ordering::Acquire)
    }

    /// Starts a fresh spectral acquisition epoch and rejects audio acquired before it.
    pub fn advance_epoch(&self) -> (u64, u64) {
        let barrier = Instant::now();
        if let Ok(mut started_at) = self.epoch_started_at.write() {
            *started_at = barrier;
        }
        let epoch = self
            .epoch
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                Some(current.saturating_add(1))
            })
            .unwrap_or_else(|current| current)
            .saturating_add(1);
        (epoch, monotonic_millis_at(barrier))
    }

    pub fn dropped_updates(&self) -> u64 {
        self.dropped_updates.load(Ordering::Relaxed)
    }

    #[cfg(test)]
    pub(crate) fn processed_chunks(&self) -> u64 {
        self.processed_chunks.load(Ordering::Acquire)
    }

    /// Invalidates visible measurements as soon as the owning capture stops.
    pub fn invalidate(&self) {
        self.epoch.fetch_add(1, Ordering::AcqRel);
        self.valid.store(false, Ordering::Release);
    }

    fn mark_dropped(&self) {
        self.dropped_updates.fetch_add(1, Ordering::Relaxed);
        self.epoch.fetch_add(1, Ordering::AcqRel);
        self.valid.store(false, Ordering::Release);
    }
}

impl Drop for MicrophoneProbe {
    fn drop(&mut self) {
        self.valid.store(false, Ordering::Release);
        self.sender.take();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl ProbeSnapshot {
    fn epoch_is_current(&self, epoch: u64) -> bool {
        self.epoch == epoch
    }

    pub fn meter(&self) -> Option<MicrophoneMeter> {
        Some(MicrophoneMeter {
            job_id: self.job_id,
            generation: self.generation,
            source_identity: self.source_identity.clone()?,
            sequence: self.sequence,
            first_frame: self.rms_first_frame,
            last_frame: self.rms_last_frame,
            sample_rate_hz: self.sample_rate_hz,
            analyzed_channels: self.analyzed_channels,
            gain_first_db: self.gain_first.gain.value(),
            gain_first_revision: self.gain_first.revision,
            gain_last_db: self.gain_last.gain.value(),
            gain_last_revision: self.gain_last.revision,
            mixed_gain: self.mixed_gain,
            pre_gain: MeterLevel::from(self.pre_gain?),
            post_gain_limiter: MeterLevel::from(self.post_gain_limiter?),
            limiter_samples: self.limiter_samples,
            dropped_updates: self.dropped_updates,
            captured_at_monotonic_ms: self.captured_at_monotonic_ms,
        })
    }
}

struct ProbeSample {
    pre_gain: f32,
    post_gain_limiter: f32,
    limiter: bool,
    gain: GainRevision,
}

struct ProbeAnalyzer {
    sample_rate_hz: u32,
    source_identity: Option<String>,
    job_id: JobId,
    generation: Generation,
    epoch: u64,
    expected_frame: Option<u64>,
    samples: VecDeque<ProbeSample>,
    max_window_samples: usize,
    rms_window_samples: usize,
    rms_publish_samples: u64,
    psd_publish_samples: u64,
    last_rms_publish_frame: u64,
    last_psd_publish_frame: u64,
    last_rms: Option<CachedRms>,
    last_spectrum: Option<CachedSpectrum>,
    spectrum_sequence: u64,
    sequence: u64,
    fft: Arc<dyn rustfft::Fft<f64>>,
    fft_buffer: Vec<Complex<f64>>,
    fft_scratch: Vec<Complex<f64>>,
}

#[derive(Clone, Copy)]
struct CachedRms {
    first_frame: u64,
    last_frame: u64,
    gain_first: GainRevision,
    gain_last: GainRevision,
    mixed_gain: bool,
    pre_gain: LevelMeasurement,
    post_gain_limiter: LevelMeasurement,
    limiter_samples: u64,
}

impl ProbeAnalyzer {
    fn new(
        sample_rate_hz: u32,
        source_identity: Option<String>,
        job_id: JobId,
        generation: Generation,
    ) -> Self {
        let mut planner = FftPlanner::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);
        let scratch_len = fft.get_inplace_scratch_len();
        Self {
            sample_rate_hz,
            source_identity,
            job_id,
            generation,
            epoch: 0,
            expected_frame: None,
            samples: VecDeque::with_capacity(
                ((u64::from(sample_rate_hz) * RMS_WINDOW_MS).div_ceil(1000) as usize).max(FFT_SIZE),
            ),
            max_window_samples: ((u64::from(sample_rate_hz) * RMS_WINDOW_MS).div_ceil(1000)
                as usize)
                .max(FFT_SIZE),
            rms_window_samples: (u64::from(sample_rate_hz) * RMS_WINDOW_MS).div_ceil(1000) as usize,
            rms_publish_samples: (u64::from(sample_rate_hz) / RMS_MAX_RATE_HZ).max(1),
            psd_publish_samples: (u64::from(sample_rate_hz) / PSD_MAX_RATE_HZ).max(1),
            last_rms_publish_frame: 0,
            last_psd_publish_frame: 0,
            last_rms: None,
            last_spectrum: None,
            spectrum_sequence: 0,
            sequence: 0,
            fft,
            fft_buffer: vec![Complex::new(0.0, 0.0); FFT_SIZE],
            fft_scratch: vec![Complex::new(0.0, 0.0); scratch_len],
        }
    }

    fn reset(&mut self, epoch: u64, first_frame: u64) {
        self.epoch = epoch;
        self.expected_frame = Some(first_frame);
        self.samples.clear();
        self.last_rms_publish_frame = first_frame;
        self.last_psd_publish_frame = first_frame;
        self.last_rms = None;
        self.last_spectrum = None;
    }

    fn push(&mut self, chunk: ProbeChunk, dropped_updates: u64) -> Option<ProbeSnapshot> {
        if chunk.epoch != self.epoch {
            self.reset(chunk.epoch, chunk.first_frame);
        }
        if self.expected_frame != Some(chunk.first_frame)
            || chunk.pre_gain.len() != chunk.post_gain_limiter.len()
            || chunk.pre_gain.len() != chunk.limiter_mask.len()
            || chunk.pre_gain.is_empty()
        {
            self.reset(chunk.epoch, chunk.first_frame);
        }
        for ((pre_gain, post_gain_limiter), limiter) in chunk
            .pre_gain
            .iter()
            .zip(&chunk.post_gain_limiter)
            .zip(&chunk.limiter_mask)
        {
            if !pre_gain.is_finite() || !post_gain_limiter.is_finite() {
                self.reset(chunk.epoch, chunk.first_frame);
                return None;
            }
            self.samples.push_back(ProbeSample {
                pre_gain: *pre_gain,
                post_gain_limiter: *post_gain_limiter,
                limiter: *limiter,
                gain: chunk.gain,
            });
            if self.samples.len() > self.max_window_samples {
                self.samples.pop_front();
            }
        }
        let last_frame = chunk
            .first_frame
            .saturating_add(chunk.pre_gain.len() as u64);
        self.expected_frame = Some(last_frame);
        let rms_ready = self.samples.len() >= self.rms_window_samples
            && last_frame.saturating_sub(self.last_rms_publish_frame) >= self.rms_publish_samples;
        let psd_ready = self.samples.len() >= FFT_SIZE
            && last_frame.saturating_sub(self.last_psd_publish_frame) >= self.psd_publish_samples;
        let publish_rms = self.samples.len() >= self.rms_window_samples && rms_ready;
        if !publish_rms && !psd_ready {
            return None;
        }
        if publish_rms {
            let rms_start = self.samples.len().saturating_sub(self.rms_window_samples);
            let rms_samples = self.samples.iter().skip(rms_start).collect::<Vec<_>>();
            let pre_values = rms_samples
                .iter()
                .map(|sample| sample.pre_gain)
                .collect::<Vec<_>>();
            let post_values = rms_samples
                .iter()
                .map(|sample| sample.post_gain_limiter)
                .collect::<Vec<_>>();
            let gain_first = rms_samples.first()?.gain;
            let gain_last = rms_samples.last()?.gain;
            let mixed_gain = rms_samples.iter().any(|sample| sample.gain != gain_first);
            let limiter_samples = rms_samples.iter().filter(|sample| sample.limiter).count() as u64;
            let rms_sample_count = rms_samples.len();
            self.last_rms = Some(CachedRms {
                first_frame: last_frame.saturating_sub(rms_sample_count as u64),
                last_frame,
                gain_first,
                gain_last,
                mixed_gain,
                pre_gain: LevelMeasurement::from_samples(&pre_values),
                post_gain_limiter: LevelMeasurement::from_samples(&post_values),
                limiter_samples,
            });
        }
        if psd_ready {
            self.last_psd_publish_frame = last_frame;
            let pre_gain = self.psd(false, last_frame);
            let post_gain_limiter = self.psd(true, last_frame);
            let start = self.samples.len() - FFT_SIZE;
            let fft_samples = self.samples.iter().skip(start).collect::<Vec<_>>();
            let gain_first = fft_samples.first()?.gain;
            let gain_last = fft_samples.last()?.gain;
            let mixed_gain = fft_samples.iter().any(|sample| sample.gain != gain_first);
            self.spectrum_sequence = self.spectrum_sequence.saturating_add(1);
            self.last_spectrum = Some(CachedSpectrum {
                sequence: self.spectrum_sequence,
                epoch: self.epoch,
                captured_at: chunk.acquired_at,
                captured_at_monotonic_ms: monotonic_millis_at(chunk.acquired_at),
                gain_first,
                gain_last,
                mixed_gain,
                pre_gain,
                post_gain_limiter,
            });
        }
        if publish_rms {
            self.last_rms_publish_frame = last_frame;
        }
        self.sequence = self.sequence.saturating_add(1);
        let rms = self.last_rms;
        Some(ProbeSnapshot {
            job_id: self.job_id,
            generation: self.generation,
            source_identity: self.source_identity.clone(),
            sequence: self.sequence,
            first_frame: last_frame.saturating_sub(FFT_SIZE as u64),
            last_frame,
            rms_first_frame: rms.map_or(last_frame, |rms| rms.first_frame),
            rms_last_frame: rms.map_or(last_frame, |rms| rms.last_frame),
            captured_at: chunk.acquired_at,
            captured_at_monotonic_ms: monotonic_millis_at(chunk.acquired_at),
            published_at: Instant::now(),
            gain_first: rms.map_or_else(GainRevision::neutral, |rms| rms.gain_first),
            gain_last: rms.map_or_else(GainRevision::neutral, |rms| rms.gain_last),
            mixed_gain: rms.is_some_and(|rms| rms.mixed_gain),
            pre_gain: rms.map(|rms| rms.pre_gain),
            post_gain_limiter: rms.map(|rms| rms.post_gain_limiter),
            limiter_samples: rms.map_or(0, |rms| rms.limiter_samples),
            dropped_updates,
            sample_rate_hz: self.sample_rate_hz,
            analyzed_channels: 1,
            pre_gain_psd: psd_ready
                .then(|| self.last_spectrum.as_ref().map(|s| s.pre_gain.clone()))
                .flatten(),
            post_gain_limiter_psd: psd_ready
                .then(|| {
                    self.last_spectrum
                        .as_ref()
                        .map(|s| s.post_gain_limiter.clone())
                })
                .flatten(),
            spectrum: self.last_spectrum.clone(),
            epoch: self.epoch,
        })
    }

    fn psd(&mut self, post_gain_limiter: bool, last_frame: u64) -> SpectralDensity {
        let start = self.samples.len() - FFT_SIZE;
        let mut window_power = 0.0;
        for (index, sample) in self.samples.iter().skip(start).enumerate() {
            let hann = 0.5 * (1.0 - (std::f64::consts::TAU * index as f64 / FFT_SIZE as f64).cos());
            let value = if post_gain_limiter {
                sample.post_gain_limiter
            } else {
                sample.pre_gain
            };
            let weighted = f64::from(value) * hann;
            self.fft_buffer[index] = Complex::new(weighted, 0.0);
            window_power += hann * hann;
        }
        self.fft
            .process_with_scratch(&mut self.fft_buffer, &mut self.fft_scratch);
        let denominator = f64::from(self.sample_rate_hz) * window_power;
        let bins = (0..=FFT_SIZE / 2)
            .map(|index| {
                let one_sided = if index == 0 || index == FFT_SIZE / 2 {
                    1.0
                } else {
                    2.0
                };
                one_sided * self.fft_buffer[index].norm_sqr() / denominator
            })
            .collect();
        SpectralDensity {
            sample_rate_hz: self.sample_rate_hz,
            first_frame: last_frame.saturating_sub(FFT_SIZE as u64),
            last_frame,
            bins,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        capture::{CaptureStream, Pcm16Converter},
        direct_live::WavOutput,
    };

    fn gain(revision: u64, value: i8) -> GainRevision {
        GainRevision {
            revision,
            gain: whisper_core::GainDb::new(value).unwrap(),
        }
    }

    #[test]
    fn probe_and_spectrum_reader_leave_stt_pcm_and_final_wav_byte_identical() {
        let directory = std::env::temp_dir().join(format!(
            "microphone-spectrum-pcm-{}-{}",
            std::process::id(),
            monotonic_millis()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let request = whisper_core::LiveRequest {
            job_id: JobId(6201),
            generation: Generation::first(),
            model_path: "unused".into(),
            model_sha256: [0; 32],
            destination: directory.to_string_lossy().into_owned(),
            group_offset_samples: 0,
            auto_stop_after_speech_samples: None,
            microphone: whisper_core::MicrophoneChoice::Device("test:mic".into()),
            microphone_gain_db: 0,
            config: whisper_core::JobConfig {
                language: whisper_core::LanguageChoice::Automatic,
                compute: whisper_core::ComputeChoice::Cpu,
            },
        };
        let mut plain_capture = CaptureStream::empty_test_capture(16_000, 1, "test:mic");
        let mut measured_capture = CaptureStream::empty_test_capture(16_000, 1, "test:mic");
        let mut plain_converter = Pcm16Converter::new(16_000, 1).unwrap();
        let mut measured_converter = Pcm16Converter::new(16_000, 1).unwrap();
        let mut plain_wav = WavOutput::create(&request).unwrap();
        let mut measured_request = request.clone();
        measured_request.job_id = JobId(6202);
        let mut measured_wav = WavOutput::create(&measured_request).unwrap();
        let probe = MicrophoneProbe::start(
            16_000,
            Some("test:mic".into()),
            JobId(6201),
            Generation::first(),
        )
        .unwrap();
        let mut reader = MicrophoneSpectrumReader::default();
        for block_index in 0..5 {
            let samples = (block_index * 960..(block_index + 1) * 960)
                .map(|frame| 0.5 * (std::f32::consts::TAU * 64.0 * frame as f32 / 2048.0).sin())
                .collect::<Vec<_>>();
            plain_capture.enqueue_test_block(samples.clone());
            measured_capture.enqueue_test_block(samples);
            let plain = plain_capture.try_next_block().unwrap();
            let measured = measured_capture.try_next_block().unwrap();
            let expected = plain_converter.push(&plain).unwrap();
            let observed = measured_converter.push_with_probe(&measured).unwrap();
            assert_eq!(
                expected, observed.samples,
                "probe must not change STT input PCM"
            );
            plain_wav.append(&expected).unwrap();
            measured_wav.append(&observed.samples).unwrap();
            probe.submit(observed.probe);
            plain_capture.recycle_block(plain).unwrap();
            measured_capture.recycle_block(measured).unwrap();
            std::thread::sleep(Duration::from_millis(20));
        }
        let plain_tail = plain_converter.finish().unwrap();
        let measured_tail = measured_converter.finish().unwrap();
        assert_eq!(plain_tail, measured_tail);
        plain_wav.append(&plain_tail).unwrap();
        measured_wav.append(&measured_tail).unwrap();
        let expected = plain_wav
            .read_samples(0, plain_wav.sample_count() as usize)
            .unwrap();
        let actual = measured_wav
            .read_samples(0, measured_wav.sample_count() as usize)
            .unwrap();
        assert_eq!(
            expected, actual,
            "probe leaves the final WAV sample stream unchanged"
        );
        let deadline = Instant::now() + Duration::from_secs(2);
        while reader.poll(&probe, Instant::now()).is_none() {
            assert!(
                Instant::now() < deadline,
                "spectrum reader never observed the analysis"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        let plain_path = plain_wav.finish().unwrap();
        let measured_path = measured_wav.finish().unwrap();
        assert_eq!(
            std::fs::read(&plain_path).unwrap(),
            std::fs::read(&measured_path).unwrap()
        );
        drop(probe);
        let _ = std::fs::remove_file(plain_path);
        let _ = std::fs::remove_file(measured_path);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn repeated_fresh_spectrum_snapshot_does_not_emit_unavailable() {
        let probe = MicrophoneProbe::start(
            48_000,
            Some("wasapi:stable-spectrum".into()),
            JobId(6203),
            Generation::first(),
        )
        .unwrap();
        for first_frame in (0..6).map(|block| block * 960) {
            let samples = (first_frame..first_frame + 960)
                .map(|frame| 0.4 * (std::f32::consts::TAU * 440.0 * frame as f32 / 48_000.0).sin())
                .collect::<Vec<_>>();
            probe.submit(ProbeAudioBlock {
                sample_rate_hz: 48_000,
                first_frame,
                acquired_at: Instant::now(),
                gain: gain(0, 0),
                mono_pre_gain: samples.clone(),
                mono_post_gain_limiter: samples,
                limiter_mask: vec![false; 960],
                pre_pcm_limiter_count: 0,
            });
            thread::sleep(Duration::from_millis(20));
        }
        let deadline = Instant::now() + Duration::from_secs(2);
        while probe.processed_chunks.load(Ordering::Acquire) < 6 {
            assert!(
                Instant::now() < deadline,
                "probe did not finish its audio blocks"
            );
            thread::sleep(Duration::from_millis(2));
        }
        let mut reader = MicrophoneSpectrumReader::default();
        let first = loop {
            if let Some(update) = reader.poll(&probe, Instant::now()) {
                break update;
            }
            assert!(Instant::now() < deadline, "spectrum was not published");
            thread::sleep(Duration::from_millis(2));
        };
        assert!(matches!(first, MicrophoneSpectrumUpdate::Available(_)));

        assert!(reader.poll(&probe, Instant::now()).is_none());
        assert!(reader.poll(&probe, Instant::now()).is_none());
        let stale_at = Instant::now() + Duration::from_millis(501);
        assert!(matches!(
            reader.poll(&probe, stale_at),
            Some(MicrophoneSpectrumUpdate::Unavailable { .. })
        ));
        assert!(
            reader
                .poll(&probe, stale_at + Duration::from_millis(1))
                .is_none()
        );
    }

    #[test]
    fn retry_epoch_discards_audio_acquired_before_the_retry_barrier() {
        let probe = MicrophoneProbe::start(
            48_000,
            Some("mic:retry".into()),
            JobId(811),
            Generation::first(),
        )
        .unwrap();
        let acquired_before_retry = Instant::now();
        let (epoch, barrier_ms) = probe.advance_epoch();
        assert_eq!(epoch, 1);
        let block = |first_frame, acquired_at| {
            let samples = (0..600)
                .map(|index| {
                    0.4 * (std::f32::consts::TAU * 64.0 * (first_frame + index) as f32
                        / FFT_SIZE as f32)
                        .sin()
                })
                .collect::<Vec<_>>();
            ProbeAudioBlock {
                sample_rate_hz: 48_000,
                first_frame,
                acquired_at,
                gain: gain(0, 0),
                mono_pre_gain: samples.clone(),
                mono_post_gain_limiter: samples,
                limiter_mask: vec![false; 600],
                pre_pcm_limiter_count: 0,
            }
        };
        let mut old_block = block(0, acquired_before_retry);
        old_block.mono_pre_gain.resize(4_800, 0.4);
        old_block.mono_post_gain_limiter.resize(4_800, 0.4);
        old_block.limiter_mask.resize(4_800, false);
        probe.submit(old_block);
        assert!(probe.latest_spectrum_at(Instant::now()).is_none());

        for frame in [0, 600, 1_200, 1_800, 2_400, 3_000, 3_600, 4_200] {
            probe.submit(block(frame, Instant::now()));
            thread::sleep(Duration::from_millis(10));
        }
        let deadline = Instant::now() + Duration::from_secs(2);
        let spectrum = loop {
            if let Some(spectrum) = probe.latest_spectrum_at(Instant::now()) {
                break spectrum;
            }
            assert!(
                Instant::now() < deadline,
                "fresh retry spectrum did not arrive"
            );
            thread::sleep(Duration::from_millis(2));
        };
        assert_eq!(spectrum.epoch, epoch);
        assert!(spectrum.captured_at_monotonic_ms >= barrier_ms);
    }

    #[test]
    fn rms_peak_and_periodic_hann_psd_match_independent_parseval_oracle() {
        let rate = 48_000_u32;
        let values: Vec<f32> = (0..9600)
            .map(|index| {
                let phase = std::f64::consts::TAU * 64.0 * index as f64 / FFT_SIZE as f64;
                (0.5 * phase.sin()) as f32
            })
            .collect();
        let mut analyzer = ProbeAnalyzer::new(rate, None, JobId(1), Generation::first());
        let mut snapshot = None;
        for start in (0..values.len()).step_by(960) {
            let end = (start + 960).min(values.len());
            let data = values[start..end].to_vec();
            let next = analyzer.push(
                ProbeChunk {
                    epoch: 0,
                    first_frame: start as u64,
                    acquired_at: Instant::now(),
                    gain: gain(0, 0),
                    pre_gain: data.clone(),
                    post_gain_limiter: data,
                    limiter_mask: vec![false; end - start],
                },
                0,
            );
            if let Some(candidate) = next
                && candidate.pre_gain.is_some()
                && candidate.pre_gain_psd.is_some()
            {
                snapshot = Some(candidate);
            }
        }
        let snapshot = snapshot.unwrap();
        let rms = snapshot.pre_gain.unwrap();
        assert!((rms.peak - 0.5).abs() <= 1e-6);
        assert!((rms.rms - 0.5 / 2.0_f64.sqrt()).abs() <= 1e-6);
        let psd = snapshot.pre_gain_psd.unwrap();
        let df = f64::from(rate) / FFT_SIZE as f64;
        let integrated = psd.bins.iter().sum::<f64>() * df;
        assert!((psd.bins[64] * df - 0.5_f64.powi(2) / 3.0).abs() < 1e-4);
        let window = (0..FFT_SIZE)
            .map(|index| {
                let hann =
                    0.5 * (1.0 - (std::f64::consts::TAU * index as f64 / FFT_SIZE as f64).cos());
                let sample = f64::from(values[values.len() - FFT_SIZE + index]);
                (hann * sample).powi(2)
            })
            .sum::<f64>()
            / (0..FFT_SIZE)
                .map(|index| {
                    let hann = 0.5
                        * (1.0 - (std::f64::consts::TAU * index as f64 / FFT_SIZE as f64).cos());
                    hann * hann
                })
                .sum::<f64>();
        assert!((integrated - window).abs() <= 1e-12 + 1e-6 * window);
        assert!((integrated - 0.5_f64.powi(2) / 2.0).abs() < 1e-4);
    }

    #[test]
    fn multitone_parseval_and_unclipped_gain_ratio_match_independent_oracle() {
        let rate = 48_000_u32;
        let pre_gain: Vec<f32> = (0..9600)
            .map(|index| {
                let n = index as f64;
                (0.2 * (std::f64::consts::TAU * 64.0 * n / FFT_SIZE as f64).sin()
                    + 0.1 * (std::f64::consts::TAU * 211.0 * n / FFT_SIZE as f64).sin())
                    as f32
            })
            .collect();
        let post_gain_limiter: Vec<f32> = pre_gain.iter().map(|sample| sample * 1.5).collect();
        let expected_window = (0..FFT_SIZE)
            .map(|index| {
                let hann =
                    0.5 * (1.0 - (std::f64::consts::TAU * index as f64 / FFT_SIZE as f64).cos());
                let sample = f64::from(pre_gain[pre_gain.len() - FFT_SIZE + index]);
                (hann * sample).powi(2)
            })
            .sum::<f64>()
            / (0..FFT_SIZE)
                .map(|index| {
                    let hann = 0.5
                        * (1.0 - (std::f64::consts::TAU * index as f64 / FFT_SIZE as f64).cos());
                    hann * hann
                })
                .sum::<f64>();
        let mut analyzer = ProbeAnalyzer::new(rate, None, JobId(6), Generation::first());
        let snapshot = analyzer
            .push(
                ProbeChunk {
                    epoch: 0,
                    first_frame: 0,
                    acquired_at: Instant::now(),
                    gain: gain(1, 4),
                    limiter_mask: vec![false; pre_gain.len()],
                    pre_gain,
                    post_gain_limiter,
                },
                0,
            )
            .unwrap();
        let pre = snapshot.pre_gain_psd.unwrap();
        let post = snapshot.post_gain_limiter_psd.unwrap();
        let df = f64::from(rate) / FFT_SIZE as f64;
        let pre_power = pre.bins.iter().sum::<f64>() * df;
        let post_power = post.bins.iter().sum::<f64>() * df;
        assert!((pre_power - expected_window).abs() <= 1e-12 + 1e-6 * expected_window);
        assert!((post_power / pre_power - 2.25).abs() <= 1e-6);
    }

    #[test]
    fn dephased_psd_snapshot_reuses_rms_without_publishing_it_early() {
        let mut analyzer = ProbeAnalyzer::new(
            48_000,
            Some("wasapi:test".into()),
            JobId(7),
            Generation::first(),
        );
        let first = vec![0.25; 4_800];
        let rms = analyzer
            .push(
                ProbeChunk {
                    epoch: 0,
                    first_frame: 0,
                    acquired_at: Instant::now(),
                    gain: gain(0, 0),
                    pre_gain: first.clone(),
                    post_gain_limiter: first,
                    limiter_mask: vec![false; 4_800],
                },
                0,
            )
            .unwrap();
        assert_eq!(rms.rms_last_frame, 4_800);
        let previous_rms = rms.pre_gain.unwrap();
        let previous_rms_frame = analyzer.last_rms_publish_frame;

        // Put the PSD cadence 25 ms after the RMS boundary. At 48 kHz one
        // 1200-frame chunk is below the 50 ms RMS publication interval.
        analyzer.last_psd_publish_frame = 1_200;
        let next = vec![0.5; 1_200];
        let snapshot = analyzer
            .push(
                ProbeChunk {
                    epoch: 0,
                    first_frame: 4_800,
                    acquired_at: Instant::now(),
                    gain: gain(0, 0),
                    pre_gain: next.clone(),
                    post_gain_limiter: next,
                    limiter_mask: vec![false; 1_200],
                },
                0,
            )
            .unwrap();
        assert_eq!(snapshot.last_frame, 6_000);
        assert_eq!(snapshot.rms_last_frame, 4_800);
        assert_eq!(snapshot.pre_gain, Some(previous_rms));
        assert!(snapshot.pre_gain_psd.is_some());
        assert_eq!(analyzer.last_rms_publish_frame, previous_rms_frame);
        assert!(
            snapshot
                .meter()
                .unwrap()
                .last_frame
                .saturating_sub(previous_rms_frame)
                == 0
        );
    }

    #[test]
    fn mixed_gain_and_limiter_counts_follow_the_rms_window() {
        let mut analyzer = ProbeAnalyzer::new(16_000, None, JobId(2), Generation::first());
        let mut result = None;
        for (block_index, revision, count) in [(0_u64, gain(0, 0), 1600), (1600, gain(1, 6), 800)] {
            let mask = vec![block_index > 0; count];
            result = analyzer.push(
                ProbeChunk {
                    epoch: 0,
                    first_frame: block_index,
                    acquired_at: Instant::now(),
                    gain: revision,
                    pre_gain: vec![0.5; count],
                    post_gain_limiter: vec![0.9; count],
                    limiter_mask: mask,
                },
                0,
            );
        }
        let snapshot = result.unwrap();
        assert!(snapshot.mixed_gain);
        assert_eq!(snapshot.gain_first.revision, 0);
        assert_eq!(snapshot.gain_last.revision, 1);
        assert_eq!(snapshot.limiter_samples, 800);
        assert!(snapshot.pre_gain.unwrap().rms > 0.49);
        assert!(snapshot.post_gain_limiter.unwrap().peak <= 1.0);
    }

    #[test]
    fn dropped_probe_blocks_force_a_full_rebuild_and_old_snapshots_expire() {
        let mut analyzer = ProbeAnalyzer::new(16_000, None, JobId(3), Generation::first());
        let first = vec![0.25; 1600];
        let _ = analyzer.push(
            ProbeChunk {
                epoch: 0,
                first_frame: 0,
                acquired_at: Instant::now(),
                gain: gain(0, 0),
                pre_gain: first.clone(),
                post_gain_limiter: first.clone(),
                limiter_mask: vec![false; first.len()],
            },
            0,
        );
        let resumed = vec![0.25; 1600];
        let snapshot = analyzer.push(
            ProbeChunk {
                epoch: 1,
                first_frame: 3200,
                acquired_at: Instant::now(),
                gain: gain(0, 0),
                pre_gain: resumed.clone(),
                post_gain_limiter: resumed.clone(),
                limiter_mask: vec![false; resumed.len()],
            },
            1,
        );
        assert!(snapshot.is_some());
        assert_eq!(snapshot.unwrap().dropped_updates, 1);
        let probe = MicrophoneProbe::start(16_000, None, JobId(4), Generation::first()).unwrap();
        probe.invalidate();
        assert!(probe.latest_at(Instant::now()).is_none());
    }

    #[test]
    fn bounded_nonblocking_queue_counts_overflow_and_readers_hold_independent_snapshots() {
        let (sender, _receiver) = mpsc::sync_channel(PROBE_QUEUE_BLOCKS);
        let epoch = AtomicU64::new(0);
        let dropped = AtomicU64::new(0);
        for index in 0..PROBE_QUEUE_BLOCKS + 2 {
            let chunk = ProbeChunk {
                epoch: epoch.load(Ordering::Acquire),
                first_frame: index as u64,
                acquired_at: Instant::now(),
                gain: gain(0, 0),
                pre_gain: vec![0.0],
                post_gain_limiter: vec![0.0],
                limiter_mask: vec![false],
            };
            if let Err(TrySendError::Full(_)) = sender.try_send(chunk) {
                dropped.fetch_add(1, Ordering::Relaxed);
                epoch.fetch_add(1, Ordering::AcqRel);
            }
        }
        assert_eq!(dropped.load(Ordering::Relaxed), 2);
        assert_eq!(epoch.load(Ordering::Acquire), 2);

        let snapshot = Arc::new(ProbeSnapshot {
            job_id: JobId(5),
            generation: Generation::first(),
            source_identity: Some("test:mic".into()),
            sequence: 1,
            first_frame: 0,
            last_frame: 2048,
            rms_first_frame: 0,
            rms_last_frame: 2048,
            captured_at: Instant::now(),
            captured_at_monotonic_ms: 0,
            published_at: Instant::now(),
            gain_first: gain(0, 0),
            gain_last: gain(0, 0),
            mixed_gain: false,
            pre_gain: None,
            post_gain_limiter: None,
            limiter_samples: 0,
            dropped_updates: 0,
            sample_rate_hz: 16_000,
            analyzed_channels: 1,
            pre_gain_psd: None,
            post_gain_limiter_psd: None,
            spectrum: None,
            epoch: 0,
        });
        let latest = Arc::new(RwLock::new(Some(snapshot)));
        let suspended_reader = latest.read().unwrap().as_ref().unwrap().clone();
        let independent_reader = latest.read().unwrap().as_ref().unwrap().clone();
        assert!(Arc::ptr_eq(&suspended_reader, &independent_reader));
        drop(independent_reader);
        assert_eq!(suspended_reader.last_frame, 2048);
    }
}
