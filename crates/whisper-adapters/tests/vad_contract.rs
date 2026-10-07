use whisper_adapters::vad::VoiceActivity;

#[test]
fn vad_accepts_only_20ms_mono_16khz_frames() {
    let mut vad = VoiceActivity::new();
    assert!(vad.is_speech(&[0; 319]).is_err());
    assert!(!vad.is_speech(&[0; 320]).unwrap());
}
