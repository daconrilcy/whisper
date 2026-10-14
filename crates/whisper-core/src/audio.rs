/// Ignore a short final window with less than 100 ms of audible signal.
/// Import windows are five seconds; direct microphone windows are at least five
/// seconds until the session is stopped. The short window is therefore a tail.
pub fn is_quiet_short_tail(pcm: &[f32]) -> bool {
    const MAX_TAIL_SAMPLES: usize = 5 * 16_000;
    const FRAME_SAMPLES: usize = 320;
    const MIN_ACTIVE_SAMPLES: usize = 1_600;
    const ACTIVE_MEAN_SQUARE: f64 = 0.000_031_622_8; // -45 dBFS

    !pcm.is_empty()
        && pcm.len() < MAX_TAIL_SAMPLES
        && pcm
            .chunks(FRAME_SAMPLES)
            .filter(|frame| {
                frame
                    .iter()
                    .map(|sample| f64::from(*sample).powi(2))
                    .sum::<f64>()
                    / frame.len() as f64
                    >= ACTIVE_MEAN_SQUARE
            })
            .map(<[f32]>::len)
            .sum::<usize>()
            < MIN_ACTIVE_SAMPLES
}

#[cfg(test)]
mod tests {
    use super::is_quiet_short_tail;

    #[test]
    fn almost_silent_file_end_produces_no_text() {
        let mut quiet = vec![0.0; 2 * 16_000];
        quiet[..640].fill(0.03);
        assert!(is_quiet_short_tail(&quiet));
    }

    #[test]
    fn short_word_after_silence_is_kept_for_transcription() {
        let mut phrase = vec![0.0; 2 * 16_000];
        phrase[16_000..16_000 + 1_920].fill(0.01);
        assert!(!is_quiet_short_tail(&phrase));
        assert!(!is_quiet_short_tail(&vec![0.0; 5 * 16_000]));
    }
}
