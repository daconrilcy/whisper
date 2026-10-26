use whisper_core::microphone::MicrophoneChoice;
use whisper_core::microphone_level::{
    GainDb, LevelMeasurement, MeasurementStatus, MicrophoneAcquisitionContext, MicrophoneIdentity,
    MicrophoneSpectrum, MicrophoneSpectrumUpdate, SelectionContext, SelectionRevision, SpectrumBin,
};

struct DefaultChangingPort {
    devices: std::rc::Rc<std::cell::RefCell<Vec<whisper_core::MicrophoneDevice>>>,
    default: std::rc::Rc<std::cell::RefCell<Option<String>>>,
}

#[test]
fn spectrum_context_binds_source_and_rejects_stale_foreign_and_future_updates() {
    use whisper_core::{Generation, JobId};
    let mut context = MicrophoneAcquisitionContext::new(
        JobId(91),
        Generation::first(),
        &MicrophoneChoice::WindowsDefault,
    );
    let spectrum = MicrophoneSpectrum {
        job_id: JobId(91),
        generation: Generation::first(),
        source_identity: "wasapi:actual".into(),
        sequence: 1,
        epoch: 0,
        first_frame: 0,
        last_frame: 2048,
        sample_rate_hz: 48_000,
        captured_at_monotonic_ms: 100,
        gain_first_db: 0,
        gain_first_revision: 0,
        gain_last_db: 0,
        gain_last_revision: 0,
        mixed_gain: false,
        dropped_updates: 0,
        pre_gain: vec![SpectrumBin::Silence; 1025],
        post_gain_limiter: vec![SpectrumBin::Silence; 1025],
    };
    assert_eq!(
        context.accept_spectrum_update(&MicrophoneSpectrumUpdate::Available(spectrum.clone()), 601),
        None
    );
    assert_eq!(
        context.accept_spectrum_update(&MicrophoneSpectrumUpdate::Available(spectrum.clone()), 99),
        None
    );
    assert!(
        context
            .accept_spectrum_update(&MicrophoneSpectrumUpdate::Available(spectrum.clone()), 600)
            .unwrap()
    );
    assert_eq!(
        context.effective_source_identity.as_deref(),
        Some("wasapi:actual")
    );
    assert_eq!(
        context.accept_spectrum_update(&MicrophoneSpectrumUpdate::Available(spectrum.clone()), 600),
        None
    );
    let mut foreign = spectrum.clone();
    foreign.source_identity = "wasapi:other".into();
    foreign.sequence = 2;
    assert_eq!(
        context.accept_spectrum_update(&MicrophoneSpectrumUpdate::Available(foreign), 600),
        None
    );
    let unavailable = MicrophoneSpectrumUpdate::Unavailable {
        job_id: JobId(91),
        generation: Generation::first(),
        source_identity: "wasapi:actual".into(),
        sequence: 2,
        epoch: 1,
        dropped_updates: 1,
        captured_at_monotonic_ms: 600,
    };
    assert_eq!(
        context.accept_spectrum_update(&unavailable, 600),
        Some(false)
    );
    assert!(!context.spectrum_visible);
}

#[test]
fn retry_barrier_rejects_same_owner_spectrum_acquired_before_cpu_retry() {
    use whisper_core::{Generation, JobId};
    let now = whisper_core::microphone_level::monotonic_millis();
    let mut context = MicrophoneAcquisitionContext::new(
        JobId(92),
        Generation::first(),
        &MicrophoneChoice::Device("wasapi:retry".into()),
    );
    context.bind_source("wasapi:retry");
    context.spectrum_min_epoch = Some(4);
    let barrier = now.saturating_add(10);
    context.spectrum_not_before_monotonic_ms = Some(barrier);
    let mut old = MicrophoneSpectrum {
        job_id: JobId(92),
        generation: Generation::first(),
        source_identity: "wasapi:retry".into(),
        sequence: 88,
        epoch: 3,
        first_frame: 80_000,
        last_frame: 82_048,
        sample_rate_hz: 48_000,
        captured_at_monotonic_ms: now.saturating_sub(1),
        gain_first_db: 0,
        gain_first_revision: 0,
        gain_last_db: 0,
        gain_last_revision: 0,
        mixed_gain: false,
        dropped_updates: 0,
        pre_gain: vec![SpectrumBin::Silence; 1025],
        post_gain_limiter: vec![SpectrumBin::Silence; 1025],
    };
    assert_eq!(
        context.accept_spectrum_update(&MicrophoneSpectrumUpdate::Available(old.clone()), now,),
        None,
        "same job/generation cannot admit a queued pre-retry epoch"
    );
    old.epoch = 4;
    old.captured_at_monotonic_ms = now;
    assert_eq!(
        context.accept_spectrum_update(
            &MicrophoneSpectrumUpdate::Available(old.clone()),
            barrier.saturating_add(1),
        ),
        None,
        "a retry epoch cannot admit a pair acquired before its time barrier"
    );
    old.sequence = 1;
    old.captured_at_monotonic_ms = barrier;
    assert_eq!(
        context.accept_spectrum_update(
            &MicrophoneSpectrumUpdate::Available(old),
            barrier.saturating_add(1),
        ),
        Some(true),
        "fresh post-retry acquisition is admitted"
    );
}

impl whisper_core::MicrophonePort for DefaultChangingPort {
    fn list_inputs(&self) -> Result<Vec<whisper_core::MicrophoneDevice>, String> {
        Ok(self.devices.borrow().clone())
    }

    fn default_input_identity(&self) -> Result<Option<String>, String> {
        Ok(self.default.borrow().clone())
    }
}

#[test]
fn gain_validation_and_neutral_conversion_match_the_microphone_contract() {
    assert!(GainDb::new(-24).is_ok());
    assert!(GainDb::new(24).is_ok());
    assert!(GainDb::new(-25).is_err());
    assert!(GainDb::new(25).is_err());
    assert_eq!(GainDb::NEUTRAL.linear_factor(), 1.0);
    assert!((GainDb::new(6).unwrap().linear_factor() - 1.995_262_314_968_879_5).abs() < 1e-12);
}

#[test]
fn command_context_rejects_old_selection_and_keeps_same_named_devices_distinct() {
    let first = MicrophoneIdentity::new("wasapi:instance-a").unwrap();
    let second = MicrophoneIdentity::new("wasapi:instance-b").unwrap();
    let selected = SelectionContext::new(
        SelectionRevision::new(9),
        MicrophoneChoice::Device("wasapi:instance-a".into()),
        Some(first.clone()),
    )
    .unwrap();

    assert!(selected.permits(SelectionRevision::new(9), &first));
    assert!(!selected.permits(SelectionRevision::new(8), &first));
    assert!(!selected.permits(SelectionRevision::new(9), &second));
    assert!(
        SelectionContext::new(
            SelectionRevision::new(10),
            MicrophoneChoice::Device("wasapi:instance-a".into()),
            Some(second),
        )
        .is_err()
    );
}

#[test]
fn absent_measurement_is_not_a_valid_silent_measurement() {
    assert_eq!(
        LevelMeasurement::from_samples(&[]).status,
        MeasurementStatus::Absent
    );
    let silence = LevelMeasurement::from_samples(&[0.0; 1600]);
    assert_eq!(silence.status, MeasurementStatus::Silence);
    assert_eq!(silence.sample_count, 1600);
    assert_eq!(silence.rms, 0.0);
    assert_eq!(silence.rms_dbfs(), None);
    let signal = LevelMeasurement::from_samples(&[0.25; 1600]);
    assert_eq!(signal.status, MeasurementStatus::Signal);
    assert!((signal.rms - 0.25).abs() < 1e-12);
    assert!((signal.rms_dbfs().unwrap() - 20.0 * 0.25_f64.log10()).abs() <= 0.001);
}

#[test]
fn windows_default_identity_resolves_before_open_and_is_refreshed_explicitly() {
    use whisper_core::microphone::{MicrophoneChoice, MicrophoneDevice, MicrophoneSettings};

    let device = |id: &str| MicrophoneDevice {
        id: id.into(),
        name: "Microphone".into(),
        unavailable_reason: None,
    };
    let devices = std::rc::Rc::new(std::cell::RefCell::new(vec![
        device("wasapi:default-a"),
        device("wasapi:default-b"),
    ]));
    let default = std::rc::Rc::new(std::cell::RefCell::new(Some("wasapi:default-a".into())));
    let port = DefaultChangingPort {
        devices: devices.clone(),
        default: default.clone(),
    };
    let mut settings = MicrophoneSettings::new(Box::new(port), MicrophoneChoice::WindowsDefault);
    assert_eq!(settings.target_identity(), Some("wasapi:default-a"));
    assert_eq!(
        settings.capture_choice(),
        MicrophoneChoice::Device("wasapi:default-a".into())
    );

    // Refresh resolves the new Windows default for the next explicit action;
    // it does not open a stream or change the preferred WindowsDefault choice.
    *default.borrow_mut() = Some("wasapi:default-b".into());
    settings.refresh().unwrap();
    assert_eq!(settings.preferred(), &MicrophoneChoice::WindowsDefault);
    assert_eq!(settings.target_identity(), Some("wasapi:default-b"));
    assert_eq!(
        settings.capture_choice(),
        MicrophoneChoice::Device("wasapi:default-b".into())
    );
}
