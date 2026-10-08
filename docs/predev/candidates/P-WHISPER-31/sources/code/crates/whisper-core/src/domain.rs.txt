use serde::{Deserialize, Serialize};
use std::num::NonZeroU64;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct JobId(pub u128);

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Generation(NonZeroU64);

impl Generation {
    pub fn first() -> Self {
        Self(NonZeroU64::MIN)
    }

    pub fn get(self) -> u64 {
        self.0.get()
    }

    pub fn next(self) -> Option<Self> {
        self.get()
            .checked_add(1)
            .and_then(NonZeroU64::new)
            .map(Self)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub struct SegmentId(pub u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceRange {
    pub start_sample: u64,
    pub end_sample: u64,
    pub sample_rate_hz: u32,
}

impl SourceRange {
    pub fn new(start_sample: u64, end_sample: u64, sample_rate_hz: u32) -> Option<Self> {
        (start_sample < end_sample && sample_rate_hz > 0).then_some(Self {
            start_sample,
            end_sample,
            sample_rate_hz,
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct JobConfig {
    pub language: LanguageChoice,
    pub compute: ComputeChoice,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum LanguageChoice {
    Automatic,
    Manual(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ComputeChoice {
    Cpu,
    Auto,
    Gpu,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum JobState {
    Preparing,
    Capturing,
    Draining,
    Finalizing,
    Complete,
    Recoverable,
    Queued,
    AwaitingChoice,
    Running,
    Cancelling,
    Interrupted,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitionError {
    InvalidTransition { from: JobState, to: JobState },
}

impl JobState {
    pub fn transition(self, to: Self) -> Result<Self, TransitionError> {
        use JobState::*;
        let valid = matches!(
            (self, to),
            (Preparing, Capturing)
                | (Preparing, Queued)
                | (Preparing, Recoverable)
                | (Capturing, Draining)
                | (Capturing, Recoverable)
                | (Draining, Finalizing)
                | (Draining, Recoverable)
                | (Finalizing, Complete)
                | (Finalizing, Recoverable)
                | (Queued, Running)
                | (Queued, AwaitingChoice)
                | (Queued, Cancelled)
                | (AwaitingChoice, Queued)
                | (AwaitingChoice, Cancelled)
                | (Running, Cancelling)
                | (Running, Interrupted)
                | (Running, Finalizing)
                | (Running, Recoverable)
                | (Cancelling, Cancelled)
                | (Cancelling, Recoverable)
                | (Interrupted, Queued)
                | (Interrupted, Recoverable)
        );
        if valid {
            Ok(to)
        } else {
            Err(TransitionError::InvalidTransition { from: self, to })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_is_positive_and_monotonic() {
        let first = Generation::first();
        assert_eq!(first.get(), 1);
        assert_eq!(first.next().unwrap().get(), 2);
    }

    #[test]
    fn source_range_rejects_empty_or_unclocked_ranges() {
        assert!(SourceRange::new(4, 4, 16_000).is_none());
        assert!(SourceRange::new(4, 8, 0).is_none());
        assert_eq!(SourceRange::new(4, 8, 16_000).unwrap().end_sample, 8);
    }

    #[test]
    fn live_lifecycle_requires_drain_before_finalization() {
        assert!(JobState::Capturing.transition(JobState::Complete).is_err());
        assert_eq!(
            JobState::Capturing.transition(JobState::Draining),
            Ok(JobState::Draining)
        );
        assert_eq!(
            JobState::Draining.transition(JobState::Finalizing),
            Ok(JobState::Finalizing)
        );
        assert_eq!(
            JobState::Finalizing.transition(JobState::Complete),
            Ok(JobState::Complete)
        );
    }

    #[test]
    fn interrupted_import_needs_explicit_return_to_queue() {
        assert_eq!(
            JobState::Running.transition(JobState::Interrupted),
            Ok(JobState::Interrupted)
        );
        assert!(JobState::Interrupted.transition(JobState::Running).is_err());
        assert_eq!(
            JobState::Interrupted.transition(JobState::Queued),
            Ok(JobState::Queued)
        );
    }
}
