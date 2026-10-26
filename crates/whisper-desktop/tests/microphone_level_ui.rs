use whisper_core::{Generation, JobId, MicrophoneDevice, microphone_level::MeterLevel};
use whisper_desktop::ui::{
    MicrophoneMeterPresentation, microphone_meter_presentation, microphone_source_change_notice,
};

fn meter(
    status: whisper_core::microphone_level::MeasurementStatus,
) -> whisper_core::microphone_level::MicrophoneMeter {
    whisper_core::microphone_level::MicrophoneMeter {
        job_id: JobId(7),
        generation: Generation::first(),
        source_identity: "wasapi:mic-a".into(),
        sequence: 1,
        first_frame: 0,
        last_frame: 4800,
        sample_rate_hz: 48_000,
        analyzed_channels: 1,
        gain_first_db: 0,
        gain_first_revision: 0,
        gain_last_db: 0,
        gain_last_revision: 0,
        mixed_gain: false,
        pre_gain: MeterLevel {
            status,
            sample_count: 4800,
            rms_micros: 100_000,
            peak_micros: 200_000,
        },
        post_gain_limiter: MeterLevel {
            status,
            sample_count: 4800,
            rms_micros: 100_000,
            peak_micros: 200_000,
        },
        limiter_samples: 0,
        dropped_updates: 0,
        captured_at_monotonic_ms: 1000,
    }
}

#[test]
fn meter_presentation_distinguishes_waiting_absent_silence_stale_and_old_capture() {
    let active = Some((JobId(7), Generation::first()));
    assert_eq!(
        microphone_meter_presentation(None, active, 1000),
        MicrophoneMeterPresentation::Awaiting
    );
    assert_eq!(
        microphone_meter_presentation(
            Some(&meter(
                whisper_core::microphone_level::MeasurementStatus::Absent
            )),
            active,
            1000,
        ),
        MicrophoneMeterPresentation::Absent
    );
    assert_eq!(
        microphone_meter_presentation(
            Some(&meter(
                whisper_core::microphone_level::MeasurementStatus::Silence
            )),
            active,
            1000,
        ),
        MicrophoneMeterPresentation::Silence
    );
    assert_eq!(
        microphone_meter_presentation(
            Some(&meter(
                whisper_core::microphone_level::MeasurementStatus::Signal
            )),
            active,
            1501,
        ),
        MicrophoneMeterPresentation::Stale
    );
    assert_eq!(
        microphone_meter_presentation(
            Some(&meter(
                whisper_core::microphone_level::MeasurementStatus::Signal
            )),
            Some((JobId(7), Generation::first().next().unwrap())),
            1000,
        ),
        MicrophoneMeterPresentation::WrongCapture
    );
    assert!(matches!(
        microphone_meter_presentation(
            Some(&meter(
                whisper_core::microphone_level::MeasurementStatus::Signal
            )),
            active,
            1500,
        ),
        MicrophoneMeterPresentation::Signal { .. }
    ));
}

#[test]
fn microphone_labels_changed_default_notice_uses_names_and_keeps_identity_comparison() {
    let devices = vec![
        MicrophoneDevice {
            id: "wasapi:default-a".into(),
            name: "Micro de bureau".into(),
            unavailable_reason: None,
        },
        MicrophoneDevice {
            id: "wasapi:default-b".into(),
            name: "Micro de bureau".into(),
            unavailable_reason: None,
        },
    ];
    let notice = microphone_source_change_notice(
        Some("wasapi:default-a"),
        Some("wasapi:default-b"),
        &devices,
    )
    .unwrap();
    assert!(notice.contains("prévu a changé (Micro de bureau (2))"));
    assert!(notice.contains("flux actif reste sur Micro de bureau (1)"));
    assert!(!notice.contains("wasapi:"));
    assert!(notice.contains("nouvelle action"));
    assert_eq!(
        microphone_source_change_notice(
            Some("wasapi:default-a"),
            Some("wasapi:default-a"),
            &devices
        ),
        None
    );
    assert_eq!(
        microphone_source_change_notice(None, Some("wasapi:default-b"), &devices),
        None
    );

    let unknown_change = microphone_source_change_notice(
        Some("wasapi:missing-a"),
        Some("wasapi:missing-b"),
        &devices,
    )
    .unwrap();
    assert!(unknown_change.contains("prévu a changé (Microphone indisponible)"));
    assert!(unknown_change.contains("reste sur Microphone indisponible"));
    assert!(!unknown_change.contains("wasapi:"));

    let invalid_id_devices = vec![
        MicrophoneDevice {
            id: String::new(),
            name: "Nom à ne pas afficher pour une identité vide".into(),
            unavailable_reason: None,
        },
        MicrophoneDevice {
            id: "   ".into(),
            name: "Nom à ne pas afficher pour une identité blanche".into(),
            unavailable_reason: None,
        },
    ];
    let invalid_id_change = microphone_source_change_notice(
        Some(""),
        Some("   "),
        &invalid_id_devices,
    )
    .unwrap();
    assert!(invalid_id_change.contains("prévu a changé (Microphone indisponible)"));
    assert!(invalid_id_change.contains("reste sur Microphone indisponible"));
    assert!(!invalid_id_change.contains("Nom à ne pas afficher"));
}
