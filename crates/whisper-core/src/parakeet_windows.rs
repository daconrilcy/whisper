//! Pure window planning for long Parakeet imports.

use crate::parakeet_ipc::Word;

pub const SAMPLE_RATE: u64 = 16_000;
pub const FRAME_SAMPLES: usize = 320;
pub const WINDOW_MIN_SAMPLES: u64 = 60 * SAMPLE_RATE;
pub const WINDOW_TARGET_SAMPLES: u64 = 90 * SAMPLE_RATE;
pub const WINDOW_MAX_SAMPLES: u64 = 120 * SAMPLE_RATE;
pub const MARGIN_SAMPLES: u64 = 200 * SAMPLE_RATE / 1_000;
pub const SILENCE_MIN_FRAMES: usize = 20;
pub const MAX_NATIVE_SAMPLES: u64 = 1_800 * SAMPLE_RATE;
pub const MAX_REFERENCE_SAMPLES: u64 = 3_600 * SAMPLE_RATE;
pub const PROFILE_IMPORT_VAD: &str = "import-vad";
pub const PROFILE_QUALIFICATION_REFERENCE: &str = "qualification-reference";
const RMS_THRESHOLD_SQUARED: u64 = 64 * 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Silence {
    pub start_sample: u64,
    pub end_sample: u64,
    pub center_sample: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Window {
    pub index: usize,
    pub central_start_sample: u64,
    pub central_end_sample: u64,
    pub native_start_sample: u64,
    pub native_end_sample: u64,
    pub cut: Option<Silence>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AbsoluteWordSamples {
    pub start_sample: u64,
    pub end_sample: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TimingError {
    InvalidLocalRange { index: usize },
    InternalWindowOverflow { index: usize },
    FinalWordOverflow { index: usize, overflow_samples: u64 },
    InvalidAbsoluteRange { index: usize },
}

impl std::fmt::Display for TimingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "invalid Parakeet word timing: {self:?}")
    }
}

impl std::error::Error for TimingError {}

/// Converts worker-relative millisecond timestamps to the source sample timebase.
/// Only the final word of the final window may be capped, and only by one TDT frame.
pub fn absolute_word_samples(
    window: &Window,
    total_samples: u64,
    words: &[Word],
) -> Result<Vec<AbsoluteWordSamples>, TimingError> {
    let window_samples = window.native_end_sample - window.native_start_sample;
    let mut result = Vec::with_capacity(words.len());
    for (index, word) in words.iter().enumerate() {
        let local_start = u64::from(word.start_ms) * SAMPLE_RATE / 1_000;
        let mut local_end = u64::from(word.end_ms) * SAMPLE_RATE / 1_000;
        if local_start >= local_end {
            return Err(TimingError::InvalidLocalRange { index });
        }
        if local_end > window_samples {
            let overflow_samples = local_end - window_samples;
            let is_final_file_word =
                window.native_end_sample == total_samples && index + 1 == words.len();
            if !is_final_file_word || overflow_samples > 80 * SAMPLE_RATE / 1_000 {
                return Err(if is_final_file_word {
                    TimingError::FinalWordOverflow {
                        index,
                        overflow_samples,
                    }
                } else {
                    TimingError::InternalWindowOverflow { index }
                });
            }
            local_end = window_samples;
        }
        let start_sample = window.native_start_sample + local_start;
        let end_sample = window.native_start_sample + local_end;
        if start_sample >= end_sample || end_sample > total_samples {
            return Err(TimingError::InvalidAbsoluteRange { index });
        }
        result.push(AbsoluteWordSamples {
            start_sample,
            end_sample,
        });
    }
    Ok(result)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WindowError {
    EmptyAudio,
    NoSilenceBeforeNativeLimit { from_sample: u64, limit: u64 },
    InvalidWindow,
}

impl std::fmt::Display for WindowError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyAudio => formatter.write_str("Audio vide"),
            Self::NoSilenceBeforeNativeLimit { from_sample, limit } => write!(
                formatter,
                "NoAdmissibleSilence: aucun silence admissible après {from_sample} avant {limit}"
            ),
            Self::InvalidWindow => formatter.write_str("InvalidWindow: bornes VAD invalides"),
        }
    }
}

/// Classifies consecutive 20 ms frames using the fixed integer RMS-64 rule.
pub fn admissible_silences(samples: &[i16]) -> Vec<Silence> {
    let mut result = Vec::new();
    let mut run_start = None;
    let full_frames = samples.len() / FRAME_SAMPLES;
    for frame_index in 0..full_frames {
        let frame = &samples[frame_index * FRAME_SAMPLES..(frame_index + 1) * FRAME_SAMPLES];
        let energy = frame
            .iter()
            .map(|sample| i64::from(*sample).unsigned_abs().pow(2))
            .sum::<u64>();
        let silent = energy <= frame.len() as u64 * RMS_THRESHOLD_SQUARED;
        if silent {
            run_start.get_or_insert(frame_index);
        } else if let Some(start) = run_start.take() {
            push_silence(&mut result, start, frame_index);
        }
    }
    if let Some(start) = run_start {
        push_silence(&mut result, start, full_frames);
    }
    result
}

fn push_silence(result: &mut Vec<Silence>, start_frame: usize, end_frame: usize) {
    if end_frame - start_frame < SILENCE_MIN_FRAMES {
        return;
    }
    let start_sample = (start_frame * FRAME_SAMPLES) as u64;
    let end_sample = (end_frame * FRAME_SAMPLES) as u64;
    result.push(Silence {
        start_sample,
        end_sample,
        center_sample: start_sample + (end_sample - start_sample) / 2,
    });
}

/// Produces joint central ranges with context-only 200 ms margins around internal cuts.
pub fn plan_windows(samples: &[i16]) -> Result<Vec<Window>, WindowError> {
    let total = samples.len() as u64;
    if total == 0 {
        return Err(WindowError::EmptyAudio);
    }
    let silences = admissible_silences(samples);
    let mut windows = Vec::new();
    let mut central_start = 0_u64;
    while central_start < total {
        let remaining = total - central_start;
        if remaining <= WINDOW_MAX_SAMPLES {
            windows.push(make_window(
                windows.len(),
                central_start,
                total,
                None,
                total,
            )?);
            break;
        }
        let target = central_start.saturating_add(WINDOW_TARGET_SAMPLES);
        let minimum = central_start.saturating_add(WINDOW_MIN_SAMPLES);
        let maximum = central_start.saturating_add(WINDOW_MAX_SAMPLES);
        let preferred = silences
            .iter()
            .copied()
            .filter(|silence| (minimum..=maximum).contains(&silence.center_sample))
            .min_by_key(|silence| {
                (
                    silence.center_sample.abs_diff(target),
                    silence.center_sample,
                )
            });
        let cut = preferred.or_else(|| {
            silences
                .iter()
                .copied()
                .find(|silence| silence.center_sample > maximum)
        });
        let Some(cut) = cut else {
            return Err(WindowError::NoSilenceBeforeNativeLimit {
                from_sample: central_start,
                limit: central_start.saturating_add(MAX_NATIVE_SAMPLES),
            });
        };
        let central_end = cut.center_sample;
        if central_end <= central_start {
            return Err(WindowError::InvalidWindow);
        }
        let window = make_window(windows.len(), central_start, central_end, Some(cut), total)?;
        if window.native_end_sample - window.native_start_sample > MAX_NATIVE_SAMPLES {
            return Err(WindowError::NoSilenceBeforeNativeLimit {
                from_sample: central_start,
                limit: central_start.saturating_add(MAX_NATIVE_SAMPLES),
            });
        }
        central_start = central_end;
        windows.push(window);
    }
    Ok(windows)
}

/// The private single-pass comparison harness is capped at one hour. This
/// check intentionally does not constrain product VAD window planning.
pub fn validate_reference_duration(total_samples: u64) -> Result<(), WindowError> {
    if total_samples == 0 {
        return Err(WindowError::EmptyAudio);
    }
    if total_samples > MAX_REFERENCE_SAMPLES {
        return Err(WindowError::InvalidWindow);
    }
    Ok(())
}

fn make_window(
    index: usize,
    central_start: u64,
    central_end: u64,
    cut: Option<Silence>,
    total: u64,
) -> Result<Window, WindowError> {
    if central_start >= central_end || central_end > total {
        return Err(WindowError::InvalidWindow);
    }
    let native_start = central_start.saturating_sub(MARGIN_SAMPLES);
    let native_end = if cut.is_some() {
        central_end.saturating_add(MARGIN_SAMPLES).min(total)
    } else {
        total
    };
    if native_start >= native_end || native_end - native_start > MAX_NATIVE_SAMPLES {
        return Err(WindowError::InvalidWindow);
    }
    Ok(Window {
        index,
        central_start_sample: central_start,
        central_end_sample: central_end,
        native_start_sample: native_start,
        native_end_sample: native_end,
        cut,
    })
}

/// Selects a word for exactly one central interval using its center timestamp.
pub fn word_belongs_to_central_range(
    start_sample: u64,
    end_sample: u64,
    central_start_sample: u64,
    central_end_sample: u64,
) -> bool {
    start_sample < end_sample
        && central_start_sample <= start_sample + (end_sample - start_sample) / 2
        && start_sample + (end_sample - start_sample) / 2 < central_end_sample
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pcm_with_level(samples: usize, level: i16) -> Vec<i16> {
        vec![level; samples]
    }

    #[test]
    fn rms_64_frame_threshold_and_400ms_run_are_exact() {
        let mut audio = pcm_with_level(FRAME_SAMPLES * 20, 64);
        assert_eq!(admissible_silences(&audio).len(), 1);
        audio.fill(65);
        assert!(admissible_silences(&audio).is_empty());
        audio.resize(FRAME_SAMPLES * 19, 0);
        assert!(admissible_silences(&audio).is_empty());
    }

    #[test]
    fn silence_is_centered_and_earliest_wins_equal_distance() {
        let mut audio = pcm_with_level(180 * SAMPLE_RATE as usize, 100);
        audio[80 * SAMPLE_RATE as usize..80 * SAMPLE_RATE as usize + 20 * FRAME_SAMPLES].fill(0);
        audio[100 * SAMPLE_RATE as usize..100 * SAMPLE_RATE as usize + 20 * FRAME_SAMPLES].fill(0);
        let silences = admissible_silences(&audio);
        assert_eq!(silences.len(), 2);
        assert_eq!(
            silences[0].center_sample,
            80 * SAMPLE_RATE + 10 * FRAME_SAMPLES as u64
        );
        let windows = plan_windows(&audio).unwrap();
        assert_eq!(windows[0].cut, Some(silences[0]));
    }

    #[test]
    fn word_center_has_one_owner_at_joint_boundary() {
        let boundary = 90 * SAMPLE_RATE;
        assert!(!word_belongs_to_central_range(
            boundary - 8,
            boundary + 8,
            0,
            boundary
        ));
        assert!(word_belongs_to_central_range(
            boundary - 8,
            boundary + 8,
            boundary,
            2 * boundary
        ));
    }

    #[test]
    fn reference_limit_does_not_cap_product_window_planning() {
        assert_eq!(plan_windows(&[]), Err(WindowError::EmptyAudio));
        assert_eq!(validate_reference_duration(1), Ok(()));
        assert_eq!(
            validate_reference_duration(MAX_REFERENCE_SAMPLES + 1),
            Err(WindowError::InvalidWindow)
        );
    }
}
