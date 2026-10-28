//! Pure session and window-cursor contracts for unified Parakeet processing.

use crate::{
    domain::{Generation, JobId},
    parakeet_windows::{
        FRAME_SAMPLES, MARGIN_SAMPLES, SAMPLE_RATE, WINDOW_MAX_SAMPLES, WINDOW_MIN_SAMPLES,
        WINDOW_TARGET_SAMPLES,
    },
};

const SILENCE_MIN_FRAMES: usize = 20;
const RMS_THRESHOLD_SQUARED: u64 = 64 * 64;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SessionIdentity {
    pub job_id: JobId,
    pub generation: Generation,
    pub strategy: TranscriptionStrategy,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptionStrategy {
    LegacyV1,
    BoundedWindowsV1,
}

impl TranscriptionStrategy {
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::LegacyV1 => "legacy-v1",
            Self::BoundedWindowsV1 => "bounded-windows-v1",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AudioFrontiers {
    pub received: u64,
    pub admitted: u64,
    pub written: u64,
    pub processed: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrontierError {
    Regressed,
    OutOfOrder,
}

impl AudioFrontiers {
    /// Advances one named frontier without conflating receipt, admission,
    /// durable write, and final recognition.
    pub fn advance(&mut self, frontier: AudioFrontier, sample: u64) -> Result<(), FrontierError> {
        let current_value = match frontier {
            AudioFrontier::Received => self.received,
            AudioFrontier::Admitted => self.admitted,
            AudioFrontier::Written => self.written,
            AudioFrontier::Processed => self.processed,
        };
        if sample < current_value {
            return Err(FrontierError::Regressed);
        }
        let (previous, next) = match frontier {
            AudioFrontier::Received => (0, sample),
            AudioFrontier::Admitted => (self.admitted, self.received),
            AudioFrontier::Written => (self.written, self.admitted),
            AudioFrontier::Processed => (self.processed, self.written),
        };
        if sample > next || sample < previous {
            return Err(FrontierError::OutOfOrder);
        }
        match frontier {
            AudioFrontier::Received => self.received = sample,
            AudioFrontier::Admitted => self.admitted = sample,
            AudioFrontier::Written => self.written = sample,
            AudioFrontier::Processed => self.processed = sample,
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AudioFrontier {
    Received,
    Admitted,
    Written,
    Processed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DocumentRevision {
    pub current: u64,
    pub final_revision: Option<u64>,
}

impl DocumentRevision {
    pub fn revise(&mut self) -> Option<u64> {
        if self.final_revision.is_some() {
            return None;
        }
        self.current = self.current.checked_add(1)?;
        Some(self.current)
    }
    pub fn finalize(&mut self) -> Option<u64> {
        if let Some(revision) = self.final_revision {
            return Some(revision);
        }
        let revision = self.current.checked_add(1)?;
        self.current = revision;
        self.final_revision = Some(revision);
        Some(revision)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AudioWindow {
    pub sequence: u64,
    pub central_start_sample: u64,
    pub central_end_sample: u64,
    pub native_start_sample: u64,
    pub native_end_sample: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowCursorError {
    InvalidBuffer,
    SequenceExhausted,
    OffsetOverflow,
}

/// Incremental, contiguous window planner. `samples` begins at `buffer_start`
/// and only needs to cover the next window plus its 200 ms context.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowCursor {
    next_start: u64,
    next_sequence: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WindowCursorCheckpoint {
    pub next_start_sample: u64,
    pub next_sequence: u64,
}

impl Default for WindowCursor {
    fn default() -> Self {
        Self {
            next_start: 0,
            next_sequence: 1,
        }
    }
}

impl WindowCursor {
    pub fn from_checkpoint(
        checkpoint: WindowCursorCheckpoint,
    ) -> Result<Self, WindowCursorError> {
        if checkpoint.next_sequence == 0
            || checkpoint.next_sequence == u64::MAX
            || (checkpoint.next_start_sample == 0) != (checkpoint.next_sequence == 1)
        {
            return Err(WindowCursorError::InvalidBuffer);
        }
        Ok(Self {
            next_start: checkpoint.next_start_sample,
            next_sequence: checkpoint.next_sequence,
        })
    }

    pub fn checkpoint(&self) -> WindowCursorCheckpoint {
        WindowCursorCheckpoint {
            next_start_sample: self.next_start,
            next_sequence: self.next_sequence,
        }
    }

    pub fn next_start(&self) -> u64 {
        self.next_start
    }

    pub fn next_window(
        &mut self,
        buffer_start: u64,
        samples: &[i16],
        source_end: u64,
        source_closed: bool,
    ) -> Result<Option<AudioWindow>, WindowCursorError> {
        let buffer_end = buffer_start
            .checked_add(samples.len() as u64)
            .ok_or(WindowCursorError::OffsetOverflow)?;
        if buffer_start > self.next_start || buffer_end > source_end || self.next_start > source_end
        {
            return Err(WindowCursorError::InvalidBuffer);
        }
        if buffer_start > self.next_start.saturating_sub(MARGIN_SAMPLES) {
            return Ok(None);
        }
        if self.next_start == source_end {
            return Ok(None);
        }
        let available = buffer_end.saturating_sub(self.next_start);
        let remaining = source_end - self.next_start;
        if remaining <= WINDOW_MAX_SAMPLES && source_closed {
            if available < remaining {
                return Ok(None);
            }
            return self.emit(source_end, source_end).map(Some);
        }
        let needed = WINDOW_MAX_SAMPLES
            .saturating_add(MARGIN_SAMPLES)
            .min(remaining);
        if available < needed {
            return Ok(None);
        }

        let min_cut = self
            .next_start
            .checked_add(WINDOW_MIN_SAMPLES)
            .ok_or(WindowCursorError::OffsetOverflow)?;
        let max_cut = self
            .next_start
            .checked_add(WINDOW_MAX_SAMPLES)
            .ok_or(WindowCursorError::OffsetOverflow)?;
        let target = self
            .next_start
            .checked_add(WINDOW_TARGET_SAMPLES)
            .ok_or(WindowCursorError::OffsetOverflow)?;
        let local_start = usize::try_from(self.next_start - buffer_start)
            .map_err(|_| WindowCursorError::InvalidBuffer)?;
        let local_end = usize::try_from(max_cut + MARGIN_SAMPLES - buffer_start)
            .map_err(|_| WindowCursorError::InvalidBuffer)?;
        let search = samples
            .get(local_start..local_end)
            .ok_or(WindowCursorError::InvalidBuffer)?;
        let cut = find_silences(search, self.next_start, min_cut, max_cut)
            .into_iter()
            .min_by_key(|center| (center.abs_diff(target), *center))
            .unwrap_or(max_cut);
        self.emit(
            cut,
            cut.checked_add(MARGIN_SAMPLES)
                .ok_or(WindowCursorError::OffsetOverflow)?
                .min(source_end),
        )
        .map(Some)
    }

    fn emit(
        &mut self,
        central_end: u64,
        native_end: u64,
    ) -> Result<AudioWindow, WindowCursorError> {
        if central_end <= self.next_start {
            return Err(WindowCursorError::InvalidBuffer);
        }
        let window = AudioWindow {
            sequence: self.next_sequence,
            central_start_sample: self.next_start,
            central_end_sample: central_end,
            native_start_sample: self.next_start.saturating_sub(MARGIN_SAMPLES),
            native_end_sample: native_end.max(central_end),
        };
        self.next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or(WindowCursorError::SequenceExhausted)?;
        self.next_start = central_end;
        Ok(window)
    }
}

fn find_silences(samples: &[i16], global_start: u64, min_cut: u64, max_cut: u64) -> Vec<u64> {
    let mut centers = Vec::new();
    let full_frames = samples.len() / FRAME_SAMPLES;
    let mut run_start = None;
    for frame in 0..full_frames {
        let data = &samples[frame * FRAME_SAMPLES..(frame + 1) * FRAME_SAMPLES];
        let energy = data
            .iter()
            .map(|sample| i64::from(*sample).unsigned_abs().pow(2))
            .sum::<u64>();
        if energy <= data.len() as u64 * RMS_THRESHOLD_SQUARED {
            run_start.get_or_insert(frame);
        } else if let Some(start) = run_start.take() {
            add_silence_center(&mut centers, start, frame, global_start, min_cut, max_cut);
        }
    }
    if let Some(start) = run_start {
        add_silence_center(
            &mut centers,
            start,
            full_frames,
            global_start,
            min_cut,
            max_cut,
        );
    }
    centers
}

fn add_silence_center(out: &mut Vec<u64>, start: usize, end: usize, base: u64, min: u64, max: u64) {
    if end - start < SILENCE_MIN_FRAMES {
        return;
    }
    let center = base.saturating_add((start as u64 + end as u64) * FRAME_SAMPLES as u64 / 2);
    if (min..=max).contains(&center) {
        out.push(center);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontiers_are_distinct_monotonic_and_ordered() {
        let mut state = AudioFrontiers::default();
        assert_eq!(state.advance(AudioFrontier::Received, 100), Ok(()));
        assert_eq!(
            state.advance(AudioFrontier::Written, 10),
            Err(FrontierError::OutOfOrder)
        );
        state.advance(AudioFrontier::Admitted, 80).unwrap();
        state.advance(AudioFrontier::Written, 50).unwrap();
        state.advance(AudioFrontier::Processed, 20).unwrap();
        assert_eq!(
            state.advance(AudioFrontier::Processed, 19),
            Err(FrontierError::Regressed)
        );
        assert_eq!(
            (
                state.received,
                state.admitted,
                state.written,
                state.processed
            ),
            (100, 80, 50, 20)
        );
    }

    #[test]
    fn continuous_speech_falls_back_to_120_seconds_and_cursor_is_contiguous() {
        let mut cursor = WindowCursor::default();
        let first_len = (WINDOW_MAX_SAMPLES + MARGIN_SAMPLES) as usize;
        let first = cursor
            .next_window(0, &vec![200_i16; first_len], first_len as u64, false)
            .unwrap()
            .unwrap();
        assert_eq!(first.central_end_sample, WINDOW_MAX_SAMPLES);
        assert_eq!(
            first.native_end_sample - first.native_start_sample,
            WINDOW_MAX_SAMPLES + MARGIN_SAMPLES
        );
        let final_start = first.central_end_sample;
        let tail = 17_000_u64;
        let buffer_start = final_start - MARGIN_SAMPLES;
        let second = cursor
            .next_window(
                buffer_start,
                &vec![0; (tail + MARGIN_SAMPLES) as usize],
                final_start + tail,
                true,
            )
            .unwrap()
            .unwrap();
        assert_eq!(second.central_start_sample, first.central_end_sample);
        assert_eq!(second.central_end_sample, final_start + tail);
    }

    #[test]
    fn final_revision_is_stable_and_cannot_be_revised() {
        let mut revision = DocumentRevision {
            current: 0,
            final_revision: None,
        };
        assert_eq!(revision.revise(), Some(1));
        assert_eq!(revision.finalize(), Some(2));
        assert_eq!(revision.revise(), None);
        assert_eq!(revision.finalize(), Some(2));
    }

    #[test]
    fn window_cursor_checkpoint_restores_exact_next_window() {
        let mut cursor = WindowCursor::default();
        let first_len = (WINDOW_MAX_SAMPLES + MARGIN_SAMPLES) as usize;
        let first = cursor
            .next_window(0, &vec![200_i16; first_len], first_len as u64, false)
            .unwrap()
            .unwrap();
        let checkpoint = cursor.checkpoint();
        let mut restored = WindowCursor::from_checkpoint(checkpoint).unwrap();
        let source_end = first.central_end_sample + 31_000;
        let next = restored
            .next_window(
                first.central_end_sample - MARGIN_SAMPLES,
                &vec![0; (31_000 + MARGIN_SAMPLES) as usize],
                source_end,
                true,
            )
            .unwrap()
            .unwrap();
        assert_eq!(next.sequence, 2);
        assert_eq!(next.central_start_sample, first.central_end_sample);
        assert_eq!(next.central_end_sample, source_end);
        assert_eq!(restored.checkpoint().next_start_sample, source_end);
    }

    #[test]
    fn window_cursor_rejects_inconsistent_checkpoint_indices() {
        for checkpoint in [
            WindowCursorCheckpoint {
                next_start_sample: 0,
                next_sequence: 2,
            },
            WindowCursorCheckpoint {
                next_start_sample: 1,
                next_sequence: 1,
            },
            WindowCursorCheckpoint {
                next_start_sample: 1,
                next_sequence: 0,
            },
            WindowCursorCheckpoint {
                next_start_sample: 1,
                next_sequence: u64::MAX,
            },
        ] {
            assert_eq!(
                WindowCursor::from_checkpoint(checkpoint),
                Err(WindowCursorError::InvalidBuffer)
            );
        }
    }
}
