use super::*;
use std::{
    env,
    io::{Read, Write},
    path::PathBuf,
};

#[test]
#[ignore = "prepares explicit campaign fixtures"]
fn prepare_reference_inputs_for_campaign() {
    let root = PathBuf::from(env::var_os("WHISPER_TEST_WORK_ROOT").expect("work root required"));
    let allowed = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/parakeet-tests/main-facades")
        .canonicalize()
        .unwrap();
    let root = root.canonicalize().unwrap();
    assert!(
        root.starts_with(&allowed),
        "fixture root is outside campaign"
    );
    let source =
        PathBuf::from(env::var_os("WHISPER_TEST_SOURCE_WAV").expect("source WAV required"));
    let mut input = std::fs::File::open(source).unwrap();
    let mut header = [0_u8; 44];
    input.read_exact(&mut header).unwrap();
    assert_eq!(&header[..4], b"RIFF");
    assert_eq!(&header[8..12], b"WAVE");
    assert_eq!(&header[36..40], b"data");
    let short_bytes = 10_u32 * 16_000 * 2;
    header[4..8].copy_from_slice(&(36 + short_bytes).to_le_bytes());
    header[40..44].copy_from_slice(&short_bytes.to_le_bytes());
    let mut short = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join("reference-short.wav"))
        .unwrap();
    short.write_all(&header).unwrap();
    assert_eq!(
        std::io::copy(&mut input.take(short_bytes.into()), &mut short).unwrap(),
        u64::from(short_bytes)
    );
    short.sync_all().unwrap();
    let long_bytes = 3_600_u32 * 16_000 * 2;
    header[4..8].copy_from_slice(&(36 + long_bytes).to_le_bytes());
    header[40..44].copy_from_slice(&long_bytes.to_le_bytes());
    let mut long = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join("reference-sixty-minutes.wav"))
        .unwrap();
    long.write_all(&header).unwrap();
    long.set_len(44 + u64::from(long_bytes)).unwrap();
    long.sync_all().unwrap();
}

#[cfg(feature = "parakeet-qualification-reference")]
#[test]
fn reference_wav_decoder_matches_product_decoder_and_reads_sixty_minutes() {
    let short =
        PathBuf::from(env::var_os("PARAKEET_L2_FR_WAV").expect("PARAKEET_L2_FR_WAV is required"));
    let product = decoder::decode_file(&short).unwrap();
    let reference = decode_reference_wav(&short).unwrap();
    assert_eq!(reference.samples, product.samples);

    let long = PathBuf::from(
        env::var_os("PARAKEET_L2_EN_LONG_WAV").expect("PARAKEET_L2_EN_LONG_WAV is required"),
    );
    let decoded = decode_reference_wav(&long).unwrap();
    assert_eq!(decoded.samples.len(), 3_600 * 16_000);
}

#[cfg(feature = "parakeet-qualification-reference")]
#[test]
fn reference_wav_decoder_rejects_audio_over_sixty_minutes() {
    let runtime = PathBuf::from(
        env::var_os("PARAKEET_L2_RUNTIME_ROOT").expect("PARAKEET_L2_RUNTIME_ROOT is required"),
    );
    let path = runtime.join(format!("reference-over-limit-{}.wav", std::process::id()));
    let data_bytes = (whisper_core::parakeet_windows::MAX_REFERENCE_SAMPLES + 1) * 2;
    let mut header = Vec::with_capacity(44);
    header.extend_from_slice(b"RIFF");
    header.extend_from_slice(&((36_u64 + data_bytes) as u32).to_le_bytes());
    header.extend_from_slice(b"WAVEfmt ");
    header.extend_from_slice(&16_u32.to_le_bytes());
    header.extend_from_slice(&1_u16.to_le_bytes());
    header.extend_from_slice(&1_u16.to_le_bytes());
    header.extend_from_slice(&16_000_u32.to_le_bytes());
    header.extend_from_slice(&32_000_u32.to_le_bytes());
    header.extend_from_slice(&2_u16.to_le_bytes());
    header.extend_from_slice(&16_u16.to_le_bytes());
    header.extend_from_slice(b"data");
    header.extend_from_slice(&(data_bytes as u32).to_le_bytes());
    assert_eq!(header.len(), 44);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .expect("create isolated over-limit fixture");
    file.write_all(&header).unwrap();
    file.set_len(44 + data_bytes).unwrap();
    drop(file);

    let error = decode_reference_wav(&path).unwrap_err();
    assert!(error.contains("AudioTooLong"), "{error}");
    fs::remove_file(path).unwrap();
}
