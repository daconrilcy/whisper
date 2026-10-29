use super::*;

#[test]
fn inaccessible_source_propagates_error() {
    let path = std::env::temp_dir().join("whisper-source-does-not-exist.wav");
    assert!(
        decode_wav_windows(&path, |_, _| Ok(()))
            .unwrap_err()
            .starts_with("source open failed:")
    );
}
