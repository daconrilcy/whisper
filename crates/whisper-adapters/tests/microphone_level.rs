use whisper_adapters::capture::{CAPTURE_SLOTS, Pcm16Converter};
use whisper_core::microphone_level::{GainDb, MAX_GAIN_DB, MIN_GAIN_DB};

#[test]
fn capture_path_accepts_the_declared_microphone_gain_range_and_bounded_queue() {
    assert_eq!(CAPTURE_SLOTS, 100);
    assert_eq!(GainDb::new(MIN_GAIN_DB).unwrap().value(), -24);
    assert_eq!(GainDb::new(MAX_GAIN_DB).unwrap().value(), 24);
    assert!(GainDb::new(MIN_GAIN_DB - 1).is_err());
    assert!(GainDb::new(MAX_GAIN_DB + 1).is_err());
    assert!(Pcm16Converter::new(48_000, 1).is_ok());
}
