use whisper_core::parakeet_ipc::Word;
use whisper_core::parakeet_windows::{
    AbsoluteWordSamples, FRAME_SAMPLES, MARGIN_SAMPLES, MAX_NATIVE_SAMPLES, MAX_REFERENCE_SAMPLES,
    SAMPLE_RATE, Silence, TimingError, Window, WindowError, absolute_word_samples,
    admissible_silences, plan_windows, word_belongs_to_central_range,
};

fn audio(seconds: usize, level: i16) -> Vec<i16> {
    vec![level; seconds * SAMPLE_RATE as usize]
}

fn add_silence(samples: &mut [i16], second: usize, frames: usize) {
    let start = second * SAMPLE_RATE as usize;
    samples[start..start + frames * FRAME_SAMPLES].fill(0);
}

#[test]
fn silence_threshold_and_minimum_duration_are_inclusive() {
    let mut pcm = vec![64; FRAME_SAMPLES * 20];
    assert_eq!(admissible_silences(&pcm).len(), 1);
    pcm.fill(65);
    assert!(admissible_silences(&pcm).is_empty());
    pcm.resize(FRAME_SAMPLES * 19, 0);
    assert!(admissible_silences(&pcm).is_empty());
    pcm.fill(0);
    pcm.resize(FRAME_SAMPLES * 21, 0);
    assert_eq!(
        admissible_silences(&pcm)[0].end_sample,
        (21 * FRAME_SAMPLES) as u64
    );
}

#[test]
fn chooses_earliest_of_equal_distance_silences_near_ninety_seconds() {
    let mut pcm = audio(180, 100);
    add_silence(&mut pcm, 80, 20);
    add_silence(&mut pcm, 100, 20);
    let silences = admissible_silences(&pcm);
    let windows = plan_windows(&pcm).unwrap();
    assert_eq!(windows[0].cut, Some(silences[0]));
    assert_eq!(
        windows[0].cut.unwrap().center_sample,
        80 * SAMPLE_RATE + 3_200
    );
}

#[test]
fn extends_after_one_hundred_twenty_seconds_to_the_next_silence() {
    let mut pcm = audio(200, 100);
    add_silence(&mut pcm, 130, 20);
    let windows = plan_windows(&pcm).unwrap();
    assert_eq!(
        windows[0].cut.unwrap().center_sample,
        130 * SAMPLE_RATE + 3_200
    );
    assert!(windows[0].native_end_sample - windows[0].native_start_sample <= MAX_NATIVE_SAMPLES);
}

#[test]
fn eof_short_tail_and_integral_silence_are_processed_without_forced_cuts() {
    let tail = plan_windows(&audio(37, 100)).unwrap();
    assert_eq!(tail.len(), 1);
    assert_eq!(tail[0].central_end_sample, 37 * SAMPLE_RATE);
    let silence = plan_windows(&audio(12, 0)).unwrap();
    assert_eq!(silence.len(), 1);
    assert_eq!(silence[0].cut, None);
}

#[test]
fn no_silence_before_native_bound_returns_an_explicit_error() {
    let pcm = vec![100; (MAX_NATIVE_SAMPLES + 10 * SAMPLE_RATE) as usize];
    assert!(matches!(
        plan_windows(&pcm),
        Err(WindowError::NoSilenceBeforeNativeLimit { .. })
    ));
}

#[test]
fn reference_profile_duration_is_bounded_and_window_margins_are_joint() {
    assert!(MAX_REFERENCE_SAMPLES == 3_600 * SAMPLE_RATE);
    let mut pcm = audio(200, 100);
    add_silence(&mut pcm, 90, 20);
    add_silence(&mut pcm, 180, 20);
    let windows = plan_windows(&pcm).unwrap();
    for pair in windows.windows(2) {
        assert_eq!(pair[0].central_end_sample, pair[1].central_start_sample);
        assert_eq!(
            pair[1].native_start_sample + MARGIN_SAMPLES,
            pair[1].central_start_sample
        );
        assert!(pair[0].native_end_sample >= pair[0].central_end_sample + MARGIN_SAMPLES);
    }
    let boundary = windows[0].central_end_sample;
    assert!(!word_belongs_to_central_range(
        boundary - 8,
        boundary + 8,
        windows[0].central_start_sample,
        boundary
    ));
    assert!(word_belongs_to_central_range(
        boundary - 8,
        boundary + 8,
        boundary,
        windows[1].central_end_sample
    ));
}

#[test]
fn timestamps_use_cumulative_sample_offsets_and_reject_internal_overflow() {
    let offset = 1_234_567;
    let window = Window {
        index: 1,
        central_start_sample: offset,
        central_end_sample: offset + 10_000,
        native_start_sample: offset,
        native_end_sample: offset + 10_000,
        cut: Some(Silence {
            start_sample: offset + 1_000,
            end_sample: offset + 7_400,
            center_sample: offset + 4_200,
        }),
    };
    let words = [Word {
        text: "word".into(),
        start_ms: 1,
        end_ms: 3,
    }];
    assert_eq!(
        absolute_word_samples(&window, offset + 20_000, &words).unwrap(),
        [AbsoluteWordSamples {
            start_sample: offset + 16,
            end_sample: offset + 48,
        }]
    );
    let overflowing = [Word {
        text: "edge".into(),
        start_ms: 620,
        end_ms: 640,
    }];
    assert_eq!(
        absolute_word_samples(&window, offset + 20_000, &overflowing),
        Err(TimingError::InternalWindowOverflow { index: 0 })
    );
}

#[test]
fn only_the_final_file_word_can_be_capped_by_one_tdt_frame() {
    let offset = 32_001;
    let window = Window {
        index: 0,
        central_start_sample: offset,
        central_end_sample: offset + 16_000,
        native_start_sample: offset,
        native_end_sample: offset + 16_000,
        cut: None,
    };
    let final_word = [Word {
        text: "last".into(),
        start_ms: 960,
        end_ms: 1_080,
    }];
    assert_eq!(
        absolute_word_samples(&window, offset + 16_000, &final_word).unwrap(),
        [AbsoluteWordSamples {
            start_sample: offset + 15_360,
            end_sample: offset + 16_000,
        }]
    );
    let too_far = [Word {
        text: "last".into(),
        start_ms: 960,
        end_ms: 1_081,
    }];
    assert!(matches!(
        absolute_word_samples(&window, offset + 16_000, &too_far),
        Err(TimingError::FinalWordOverflow { .. })
    ));
    let not_last = [
        Word {
            text: "first".into(),
            start_ms: 900,
            end_ms: 1_080,
        },
        Word {
            text: "last".into(),
            start_ms: 960,
            end_ms: 1_080,
        },
    ];
    assert!(matches!(
        absolute_word_samples(&window, offset + 16_000, &not_last),
        Err(TimingError::InternalWindowOverflow { index: 0 })
    ));
}
