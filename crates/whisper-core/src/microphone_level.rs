use crate::microphone::MicrophoneChoice;
use std::{sync::OnceLock, time::Instant};

static MONOTONIC_ORIGIN: OnceLock<Instant> = OnceLock::new();

pub fn monotonic_millis() -> u64 {
    monotonic_millis_at(Instant::now())
}

pub fn monotonic_millis_at(moment: Instant) -> u64 {
    let origin = *MONOTONIC_ORIGIN.get_or_init(Instant::now);
    moment
        .checked_duration_since(origin)
        .unwrap_or_default()
        .as_millis()
        .min(u64::MAX as u128) as u64
}

pub const MIN_GAIN_DB: i8 = -24;
pub const MAX_GAIN_DB: i8 = 24;

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct GainDb(i8);

impl GainDb {
    pub const NEUTRAL: Self = Self(0);

    pub fn new(value: i8) -> Result<Self, GainError> {
        if (MIN_GAIN_DB..=MAX_GAIN_DB).contains(&value) {
            Ok(Self(value))
        } else {
            Err(GainError::OutOfRange(value))
        }
    }

    pub const fn value(self) -> i8 {
        self.0
    }

    pub fn linear_factor(self) -> f64 {
        10_f64.powf(f64::from(self.0) / 20.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GainError {
    OutOfRange(i8),
}

impl std::fmt::Display for GainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OutOfRange(value) => write!(
                f,
                "gain microphone hors limites ({MIN_GAIN_DB}..={MAX_GAIN_DB} dB): {value}"
            ),
        }
    }
}

impl std::error::Error for GainError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MicrophoneIdentity(String);

impl MicrophoneIdentity {
    pub fn new(value: impl Into<String>) -> Result<Self, MicrophoneLevelError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(MicrophoneLevelError::EmptyIdentity);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SelectionRevision(u64);

impl SelectionRevision {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn value(self) -> u64 {
        self.0
    }
}

/// Captures the user's selection and the identity currently resolved for it.
/// A Windows-default selection has no target until the default can be queried.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectionContext {
    revision: SelectionRevision,
    preferred: MicrophoneChoice,
    target: Option<MicrophoneIdentity>,
}

impl SelectionContext {
    pub fn new(
        revision: SelectionRevision,
        preferred: MicrophoneChoice,
        target: Option<MicrophoneIdentity>,
    ) -> Result<Self, MicrophoneLevelError> {
        if let MicrophoneChoice::Device(id) = &preferred
            && target.as_ref().is_some_and(|target| target.as_str() != id)
        {
            return Err(MicrophoneLevelError::IdentityDoesNotMatchSelection);
        }
        Ok(Self {
            revision,
            preferred,
            target,
        })
    }

    pub const fn revision(&self) -> SelectionRevision {
        self.revision
    }

    pub fn preferred(&self) -> &MicrophoneChoice {
        &self.preferred
    }

    pub fn target(&self) -> Option<&MicrophoneIdentity> {
        self.target.as_ref()
    }

    pub fn permits(&self, revision: SelectionRevision, identity: &MicrophoneIdentity) -> bool {
        self.revision == revision && self.target.as_ref() == Some(identity)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GainRevision {
    pub revision: u64,
    pub gain: GainDb,
}

impl GainRevision {
    pub const fn neutral() -> Self {
        Self {
            revision: 0,
            gain: GainDb::NEUTRAL,
        }
    }

    pub fn next(self, gain: GainDb) -> Result<Self, MicrophoneLevelError> {
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(MicrophoneLevelError::RevisionExhausted)?;
        Ok(Self { revision, gain })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MeasurementStatus {
    Absent,
    Silence,
    Signal,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LevelMeasurement {
    pub status: MeasurementStatus,
    pub sample_count: usize,
    pub rms: f64,
    pub peak: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MeterLevel {
    pub status: MeasurementStatus,
    pub sample_count: usize,
    /// Linear amplitude scaled by 1,000,000 to retain `ImportEvent: Eq`.
    pub rms_micros: u64,
    pub peak_micros: u64,
}

impl From<LevelMeasurement> for MeterLevel {
    fn from(value: LevelMeasurement) -> Self {
        fn scaled(amplitude: f64) -> u64 {
            if !amplitude.is_finite() || amplitude <= 0.0 {
                0
            } else {
                (amplitude * 1_000_000.0).round().min(u64::MAX as f64) as u64
            }
        }
        Self {
            status: value.status,
            sample_count: value.sample_count,
            rms_micros: scaled(value.rms),
            peak_micros: scaled(value.peak),
        }
    }
}

impl MeterLevel {
    pub fn rms(&self) -> f64 {
        self.rms_micros as f64 / 1_000_000.0
    }

    pub fn peak(&self) -> f64 {
        self.peak_micros as f64 / 1_000_000.0
    }

    pub fn rms_dbfs(&self) -> Option<f64> {
        (self.status == MeasurementStatus::Signal && self.rms_micros > 0)
            .then(|| 20.0 * self.rms().log10())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MicrophoneMeter {
    pub job_id: crate::domain::JobId,
    pub generation: crate::domain::Generation,
    pub source_identity: String,
    pub sequence: u64,
    pub first_frame: u64,
    pub last_frame: u64,
    pub sample_rate_hz: u32,
    pub analyzed_channels: u16,
    pub gain_first_db: i8,
    pub gain_first_revision: u64,
    pub gain_last_db: i8,
    pub gain_last_revision: u64,
    pub mixed_gain: bool,
    pub pre_gain: MeterLevel,
    pub post_gain_limiter: MeterLevel,
    pub limiter_samples: u64,
    pub dropped_updates: u64,
    pub captured_at_monotonic_ms: u64,
}

pub const MICROPHONE_SPECTRUM_FFT_SIZE: u64 = 2048;
pub const MICROPHONE_SPECTRUM_BIN_COUNT: usize = MICROPHONE_SPECTRUM_FFT_SIZE as usize / 2 + 1;

/// A PSD value expressed in milli-dBFS/Hz. Silence is represented explicitly
/// so the transport remains finite and Eq-compatible.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SpectrumBin {
    Silence,
    MilliDb(i32),
}

impl SpectrumBin {
    pub fn from_power(power_fs2_per_hz: f64) -> Option<Self> {
        if !power_fs2_per_hz.is_finite() || power_fs2_per_hz < 0.0 {
            return None;
        }
        if power_fs2_per_hz == 0.0 {
            return Some(Self::Silence);
        }
        let milli_db = (10_000.0 * power_fs2_per_hz.log10()).round();
        if !milli_db.is_finite() || milli_db < f64::from(i32::MIN) || milli_db > f64::from(i32::MAX)
        {
            return None;
        }
        Some(Self::MilliDb(milli_db as i32))
    }

    pub fn dbfs_per_hz(self) -> Option<f64> {
        match self {
            Self::Silence => None,
            Self::MilliDb(value) => Some(f64::from(value) / 1000.0),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MicrophoneSpectrum {
    pub job_id: crate::domain::JobId,
    pub generation: crate::domain::Generation,
    pub source_identity: String,
    pub sequence: u64,
    pub epoch: u64,
    pub first_frame: u64,
    /// Exclusive end of the FFT window.
    pub last_frame: u64,
    pub sample_rate_hz: u32,
    pub captured_at_monotonic_ms: u64,
    pub gain_first_db: i8,
    pub gain_first_revision: u64,
    pub gain_last_db: i8,
    pub gain_last_revision: u64,
    pub mixed_gain: bool,
    pub dropped_updates: u64,
    pub pre_gain: Vec<SpectrumBin>,
    pub post_gain_limiter: Vec<SpectrumBin>,
}

impl MicrophoneSpectrum {
    pub fn is_valid(&self) -> bool {
        let gain_first_valid = GainDb::new(self.gain_first_db).is_ok();
        let gain_last_valid = GainDb::new(self.gain_last_db).is_ok();
        !self.source_identity.trim().is_empty()
            && self.sample_rate_hz >= 16_000
            && self.last_frame.saturating_sub(self.first_frame) == MICROPHONE_SPECTRUM_FFT_SIZE
            && self.pre_gain.len() == MICROPHONE_SPECTRUM_BIN_COUNT
            && self.post_gain_limiter.len() == MICROPHONE_SPECTRUM_BIN_COUNT
            && gain_first_valid
            && gain_last_valid
            && self.gain_first_revision <= self.gain_last_revision
            && (self.mixed_gain
                || (self.gain_first_revision == self.gain_last_revision
                    && self.gain_first_db == self.gain_last_db))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MicrophoneSpectrumUpdate {
    Available(MicrophoneSpectrum),
    Unavailable {
        job_id: crate::domain::JobId,
        generation: crate::domain::Generation,
        source_identity: String,
        sequence: u64,
        epoch: u64,
        dropped_updates: u64,
        captured_at_monotonic_ms: u64,
    },
}

impl MicrophoneSpectrumUpdate {
    pub fn owner(&self) -> (crate::domain::JobId, crate::domain::Generation) {
        match self {
            Self::Available(spectrum) => (spectrum.job_id, spectrum.generation),
            Self::Unavailable {
                job_id, generation, ..
            } => (*job_id, *generation),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MicrophoneAcquisitionContext {
    pub job_id: crate::domain::JobId,
    pub generation: crate::domain::Generation,
    pub expected_source_identity: Option<String>,
    pub effective_source_identity: Option<String>,
    pub spectrum_epoch: Option<u64>,
    pub spectrum_sequence: u64,
    pub spectrum_visible: bool,
    pub spectrum_min_epoch: Option<u64>,
    pub spectrum_not_before_monotonic_ms: Option<u64>,
}

impl MicrophoneAcquisitionContext {
    pub fn new(
        job_id: crate::domain::JobId,
        generation: crate::domain::Generation,
        microphone: &MicrophoneChoice,
    ) -> Self {
        let expected_source_identity = match microphone {
            MicrophoneChoice::WindowsDefault => None,
            MicrophoneChoice::Device(identity) => Some(identity.clone()),
        };
        Self {
            job_id,
            generation,
            expected_source_identity,
            effective_source_identity: None,
            spectrum_epoch: None,
            spectrum_sequence: 0,
            spectrum_visible: false,
            spectrum_min_epoch: None,
            spectrum_not_before_monotonic_ms: None,
        }
    }

    /// Binds the first real source reported by the opened stream, while
    /// refusing any stream that differs from the user's explicit selection.
    pub fn bind_source(&mut self, source_identity: &str) -> bool {
        if source_identity.trim().is_empty()
            || self
                .expected_source_identity
                .as_deref()
                .is_some_and(|expected| expected != source_identity)
            || self
                .effective_source_identity
                .as_deref()
                .is_some_and(|current| current != source_identity)
        {
            return false;
        }
        self.effective_source_identity = Some(source_identity.to_owned());
        true
    }

    pub fn permits_source(&self, source_identity: &str) -> bool {
        self.effective_source_identity
            .as_deref()
            .is_some_and(|current| current == source_identity)
    }

    /// Returns `Some(true)` for a fresh pair, `Some(false)` for an accepted
    /// invalidation, and `None` for a stale, malformed, or foreign update.
    pub fn accept_spectrum_update(
        &mut self,
        update: &MicrophoneSpectrumUpdate,
        now_monotonic_ms: u64,
    ) -> Option<bool> {
        if update.owner() != (self.job_id, self.generation) {
            return None;
        }
        match update {
            MicrophoneSpectrumUpdate::Available(spectrum) => {
                if !spectrum.is_valid()
                    || now_monotonic_ms < spectrum.captured_at_monotonic_ms
                    || now_monotonic_ms.saturating_sub(spectrum.captured_at_monotonic_ms) > 500
                    || !self.bind_source(&spectrum.source_identity)
                    || self
                        .spectrum_min_epoch
                        .is_some_and(|minimum| spectrum.epoch < minimum)
                    || self
                        .spectrum_not_before_monotonic_ms
                        .is_some_and(|barrier| spectrum.captured_at_monotonic_ms < barrier)
                    || self
                        .spectrum_epoch
                        .is_some_and(|epoch| spectrum.epoch < epoch)
                {
                    return None;
                }
                if self.spectrum_epoch != Some(spectrum.epoch) {
                    self.spectrum_epoch = Some(spectrum.epoch);
                    self.spectrum_sequence = 0;
                    self.spectrum_visible = false;
                }
                if spectrum.sequence <= self.spectrum_sequence {
                    return None;
                }
                self.spectrum_sequence = spectrum.sequence;
                self.spectrum_visible = true;
                Some(true)
            }
            MicrophoneSpectrumUpdate::Unavailable {
                source_identity,
                sequence,
                epoch,
                captured_at_monotonic_ms,
                ..
            } => {
                if source_identity.trim().is_empty()
                    || now_monotonic_ms < *captured_at_monotonic_ms
                    || now_monotonic_ms.saturating_sub(*captured_at_monotonic_ms) > 500
                    || !self.bind_source(source_identity)
                    || self
                        .spectrum_min_epoch
                        .is_some_and(|minimum| *epoch < minimum)
                    || self
                        .spectrum_not_before_monotonic_ms
                        .is_some_and(|barrier| *captured_at_monotonic_ms < barrier)
                    || self.spectrum_epoch.is_some_and(|current| *epoch < current)
                {
                    return None;
                }
                if self.spectrum_epoch != Some(*epoch) {
                    self.spectrum_epoch = Some(*epoch);
                    self.spectrum_sequence = 0;
                    self.spectrum_visible = false;
                    self.spectrum_sequence = *sequence;
                    return Some(false);
                }
                if *sequence < self.spectrum_sequence
                    || (*sequence == self.spectrum_sequence && !self.spectrum_visible)
                {
                    return None;
                }
                self.spectrum_sequence = *sequence;
                self.spectrum_visible = false;
                Some(false)
            }
        }
    }
}

impl LevelMeasurement {
    pub const fn absent() -> Self {
        Self {
            status: MeasurementStatus::Absent,
            sample_count: 0,
            rms: 0.0,
            peak: 0.0,
        }
    }

    pub fn from_samples(samples: &[f32]) -> Self {
        if samples.is_empty() {
            return Self::absent();
        }
        let mut square_sum = 0.0_f64;
        let mut peak = 0.0_f64;
        for sample in samples {
            let value = f64::from(*sample);
            if !value.is_finite() {
                return Self::absent();
            }
            square_sum += value * value;
            peak = peak.max(value.abs());
        }
        let rms = (square_sum / samples.len() as f64).sqrt();
        Self {
            status: if peak == 0.0 {
                MeasurementStatus::Silence
            } else {
                MeasurementStatus::Signal
            },
            sample_count: samples.len(),
            rms,
            peak,
        }
    }

    pub fn rms_dbfs(self) -> Option<f64> {
        (self.status == MeasurementStatus::Signal && self.rms.is_finite() && self.rms > 0.0)
            .then(|| 20.0 * self.rms.log10())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MicrophoneLevelError {
    EmptyIdentity,
    IdentityDoesNotMatchSelection,
    StaleSelection,
    NoTarget,
    RevisionExhausted,
}

impl std::fmt::Display for MicrophoneLevelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyIdentity => f.write_str("identité microphone vide"),
            Self::IdentityDoesNotMatchSelection => {
                f.write_str("identité microphone différente de la sélection")
            }
            Self::StaleSelection => f.write_str("contexte de sélection microphone périmé"),
            Self::NoTarget => f.write_str("aucun microphone identifié à régler"),
            Self::RevisionExhausted => f.write_str("compteur de révision microphone épuisé"),
        }
    }
}

impl std::error::Error for MicrophoneLevelError {}

#[cfg(test)]
mod tests {
    use super::{
        GainDb, GainRevision, LevelMeasurement, MeasurementStatus, MicrophoneIdentity,
        SelectionContext, SelectionRevision,
    };
    use crate::microphone::MicrophoneChoice;

    #[test]
    fn gain_bounds_are_inclusive_and_invalid_values_are_rejected() {
        assert_eq!(GainDb::new(-24).unwrap().value(), -24);
        assert_eq!(GainDb::new(24).unwrap().value(), 24);
        assert!(GainDb::new(-25).is_err());
        assert!(GainDb::new(25).is_err());
        assert_eq!(GainDb::NEUTRAL.linear_factor(), 1.0);
    }

    #[test]
    fn selection_context_targets_only_its_current_identity_and_revision() {
        let first = MicrophoneIdentity::new("wasapi:one").unwrap();
        let second = MicrophoneIdentity::new("wasapi:two").unwrap();
        let context = SelectionContext::new(
            SelectionRevision::new(4),
            MicrophoneChoice::Device("wasapi:one".into()),
            Some(first.clone()),
        )
        .unwrap();
        assert!(context.permits(SelectionRevision::new(4), &first));
        assert!(!context.permits(SelectionRevision::new(3), &first));
        assert!(!context.permits(SelectionRevision::new(4), &second));
        assert!(
            SelectionContext::new(
                SelectionRevision::new(5),
                MicrophoneChoice::Device("wasapi:one".into()),
                Some(second),
            )
            .is_err()
        );
        let unresolved_default = SelectionContext::new(
            SelectionRevision::new(6),
            MicrophoneChoice::WindowsDefault,
            None,
        )
        .unwrap();
        assert!(unresolved_default.target().is_none());
    }

    #[test]
    fn silence_and_absent_measurement_are_distinct() {
        assert_eq!(
            LevelMeasurement::from_samples(&[0.0, 0.0]).status,
            MeasurementStatus::Silence
        );
        assert_eq!(
            LevelMeasurement::from_samples(&[]).status,
            MeasurementStatus::Absent
        );
        assert_eq!(
            LevelMeasurement::from_samples(&[0.5]).status,
            MeasurementStatus::Signal
        );
    }

    #[test]
    fn gain_revisions_are_monotonic_and_checked() {
        let first = GainRevision::neutral()
            .next(GainDb::new(3).unwrap())
            .unwrap();
        assert_eq!(first.revision, 1);
        assert!(first.next(GainDb::NEUTRAL).is_ok());
        assert!(
            GainRevision {
                revision: u64::MAX,
                gain: GainDb::NEUTRAL,
            }
            .next(GainDb::NEUTRAL)
            .is_err()
        );
    }
}
