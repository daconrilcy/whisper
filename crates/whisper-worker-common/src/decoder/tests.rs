use super::*;
fn decode_wav_windows(
    path: &Path,
    callback: impl FnMut(u64, Vec<f32>) -> Result<(), String>,
) -> Result<(u64, u64), String> {
    super::decode_wav_windows(path, callback, None)
}
fn decode_requested_block(request: &DecodeRequest) -> Result<Option<Vec<i16>>, String> {
    super::decode_requested_block(request, None)
}
use std::{fs::File, io::Write, path::PathBuf, time::SystemTime};
use whisper_core::{
    Generation, JobId, SourceRange,
    ports::{DecodeRequest, SourceIdentity},
};

#[test]
fn long_source_is_streamed_as_bounded_contiguous_windows() {
    const SOURCE_SAMPLES: u32 = 2_000_123;
    let path = std::env::temp_dir().join(format!(
        "whisper-r7-stream-{}-{}.wav",
        std::process::id(),
        SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    write_silence_wav(&path, SOURCE_SAMPLES);

    let mut observed_windows = Vec::new();
    let mut next_expected_start = 0_u64;
    let (decoded_frames, total_samples) = decode_wav_windows(&path, |start, window| {
        assert!(!window.is_empty());
        assert!(window.len() <= MAX_WINDOW_SAMPLES);
        assert_eq!(
            start, next_expected_start,
            "no gap or overlap between windows"
        );
        next_expected_start += window.len() as u64;
        observed_windows.push((start, window.len()));
        Ok(())
    })
    .unwrap();
    assert_eq!(decoded_frames, u64::from(SOURCE_SAMPLES));
    assert_eq!(total_samples, u64::from(SOURCE_SAMPLES));
    assert_eq!(next_expected_start, u64::from(SOURCE_SAMPLES));
    let mut expected_windows = (0..25)
        .map(|index| (index * 80_000, 80_000))
        .collect::<Vec<_>>();
    expected_windows.push((2_000_000, 123));
    assert_eq!(observed_windows, expected_windows);

    let bounded = decode_requested_block(&DecodeRequest {
        source: SourceIdentity {
            source_id: path.to_string_lossy().into_owned(),
            sha256: [0; 32],
        },
        job_id: JobId(904),
        generation: Generation::first(),
        range: SourceRange::new(40_000, 160_000, OUTPUT_RATE).unwrap(),
        max_samples: MAX_WINDOW_SAMPLES,
    })
    .unwrap()
    .unwrap();
    assert_eq!(bounded.len(), MAX_WINDOW_SAMPLES);
    drop(bounded);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn lame_gapless_delay_and_padding_are_trimmed_once() {
    let Some(path) = std::env::var_os("WHISPER_L02_GAPLESS_MP3").map(PathBuf::from) else {
        eprintln!("V-IMPORT-MP3 decoder proof NOT RUN: set WHISPER_L02_GAPLESS_MP3");
        return;
    };
    let file = File::open(&path).unwrap();
    let stream = MediaSourceStream::new(Box::new(file), Default::default());
    let probed = symphonia::default::get_probe()
        .format(
            &Hint::new(),
            stream,
            &FormatOptions {
                enable_gapless: true,
                ..Default::default()
            },
            &MetadataOptions::default(),
        )
        .unwrap();
    let (rate, frame_count) = {
        let track = probed.format.default_track().unwrap();
        let params = &track.codec_params;
        assert_eq!(params.codec, CODEC_TYPE_MP3);
        assert!(params.delay.unwrap_or_default() > 0);
        assert!(params.padding.unwrap_or_default() > 0);
        (
            u64::from(params.sample_rate.unwrap()),
            params.n_frames.unwrap(),
        )
    };
    let expected_output = frame_count
        .saturating_sub(1)
        .saturating_mul(u64::from(OUTPUT_RATE))
        / rate
        + 1;
    drop(probed);

    let (decoded_frames, output_samples) = decode_wav_windows(&path, |_, window| {
        assert!(!window.is_empty());
        Ok(())
    })
    .unwrap();
    assert_eq!(
        decoded_frames, frame_count,
        "MP3 delay/padding trimmed once"
    );
    assert_eq!(
        output_samples, expected_output,
        "source timeline duration retained"
    );
}

fn write_silence_wav(path: &Path, samples: u32) {
    let data_bytes = samples * 2;
    let mut file = File::create(path).unwrap();
    file.write_all(b"RIFF").unwrap();
    file.write_all(&(36 + data_bytes).to_le_bytes()).unwrap();
    file.write_all(b"WAVEfmt ").unwrap();
    file.write_all(&16_u32.to_le_bytes()).unwrap();
    file.write_all(&1_u16.to_le_bytes()).unwrap();
    file.write_all(&1_u16.to_le_bytes()).unwrap();
    file.write_all(&OUTPUT_RATE.to_le_bytes()).unwrap();
    file.write_all(&(OUTPUT_RATE * 2).to_le_bytes()).unwrap();
    file.write_all(&2_u16.to_le_bytes()).unwrap();
    file.write_all(&16_u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&data_bytes.to_le_bytes()).unwrap();
    let silence = [0_u8; 8192];
    let mut remaining = data_bytes as usize;
    while remaining > 0 {
        let count = remaining.min(silence.len());
        file.write_all(&silence[..count]).unwrap();
        remaining -= count;
    }
    file.sync_all().unwrap();
}

#[test]
fn capture_non_silent_pcm_baseline() {
    use sha2::{Digest, Sha256};
    let samples: Vec<i16> = (0..80_123)
        .map(|i| ((i * 997) % 60_001 - 30_000) as i16)
        .collect();
    let path =
        std::env::temp_dir().join(format!("worker-common-baseline-{}.wav", std::process::id()));
    let data_bytes = (samples.len() * 2) as u32;
    let mut file = File::create(&path).unwrap();
    file.write_all(b"RIFF").unwrap();
    file.write_all(&(36 + data_bytes).to_le_bytes()).unwrap();
    file.write_all(b"WAVEfmt ").unwrap();
    file.write_all(&16_u32.to_le_bytes()).unwrap();
    file.write_all(&1_u16.to_le_bytes()).unwrap();
    file.write_all(&1_u16.to_le_bytes()).unwrap();
    file.write_all(&OUTPUT_RATE.to_le_bytes()).unwrap();
    file.write_all(&(OUTPUT_RATE * 2).to_le_bytes()).unwrap();
    file.write_all(&2_u16.to_le_bytes()).unwrap();
    file.write_all(&16_u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&data_bytes.to_le_bytes()).unwrap();
    for sample in &samples {
        file.write_all(&sample.to_le_bytes()).unwrap();
    }
    drop(file);
    let mut bits = Sha256::new();
    let mut windows = Vec::new();
    let totals = decode_wav_windows(&path, |start, window| {
        windows.push((start, window.len()));
        for sample in window {
            bits.update(sample.to_bits().to_le_bytes());
        }
        Ok(())
    })
    .unwrap();
    let block = decode_requested_block(&DecodeRequest {
        source: SourceIdentity {
            source_id: path.to_string_lossy().into_owned(),
            sha256: [0; 32],
        },
        job_id: JobId(904),
        generation: Generation::first(),
        range: SourceRange::new(39_000, 80_123, OUTPUT_RATE).unwrap(),
        max_samples: MAX_WINDOW_SAMPLES,
    })
    .unwrap()
    .unwrap();
    let mut i16_hash = Sha256::new();
    for sample in &block {
        i16_hash.update(sample.to_le_bytes());
    }
    assert_eq!(totals, (80_123, 80_123));
    assert_eq!(windows, vec![(0, 80_000), (80_000, 123)]);
    assert_eq!(
        format!("{:x}", bits.finalize()),
        "ddf0671a8ebde21d794887896c0ab077c68b160ef61545e4bd1ac96f853e58e2"
    );
    assert_eq!(block.len(), 41_123);
    assert_eq!(
        format!("{:x}", i16_hash.finalize()),
        "2bd3f3ceacabf3d2871b636e973a8742a2c94673af20da99df36b3c5ca7070bc"
    );
}

#[test]
fn invalid_range_missing_source_and_callback_failure_are_preserved() {
    let path =
        std::env::temp_dir().join(format!("worker-common-failures-{}.wav", std::process::id()));
    write_silence_wav(&path, 16);
    let request = |start, end, rate, max| DecodeRequest {
        source: SourceIdentity {
            source_id: path.to_string_lossy().into_owned(),
            sha256: [0; 32],
        },
        job_id: JobId(905),
        generation: Generation::first(),
        range: SourceRange {
            start_sample: start,
            end_sample: end,
            sample_rate_hz: rate,
        },
        max_samples: max,
    };
    for r in [
        request(0, 1, OUTPUT_RATE, 0),
        request(0, 1, 1, 1),
        request(1, 1, OUTPUT_RATE, 1),
        request(0, 1, OUTPUT_RATE, MAX_WINDOW_SAMPLES + 1),
    ] {
        assert!(
            decode_requested_block(&r)
                .unwrap_err()
                .starts_with("InvalidRequest:")
        );
    }
    assert_eq!(
        decode_requested_block(&request(16, 20, OUTPUT_RATE, 4)).unwrap(),
        None
    );
    assert_eq!(
        decode_wav_windows(&path, |_, _| Err("callback stopped".to_owned())).unwrap_err(),
        "callback stopped"
    );
    let missing = path.with_extension("absent");
    assert!(
        decode_wav_windows(&missing, |_, _| Ok(()))
            .unwrap_err()
            .starts_with("source open failed:")
    );
}

#[test]
fn range_crossing_source_end_returns_available_pcm() {
    use std::io::{Seek, SeekFrom};
    let path = std::env::temp_dir().join(format!("worker-common-eof-{}.wav", std::process::id()));
    write_silence_wav(&path, 16);
    let mut file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
    file.seek(SeekFrom::End(-8)).unwrap();
    for sample in [16_384_i16, -16_384, 8_192, -8_192] {
        file.write_all(&sample.to_le_bytes()).unwrap();
    }
    drop(file);
    let request = DecodeRequest {
        source: SourceIdentity {
            source_id: path.to_string_lossy().into_owned(),
            sha256: [0; 32],
        },
        job_id: JobId(906),
        generation: Generation::first(),
        range: SourceRange::new(12, 20, OUTPUT_RATE).unwrap(),
        max_samples: 8,
    };
    let pcm = decode_requested_block(&request)
        .unwrap()
        .expect("start precedes EOF");
    assert_eq!(pcm.len(), 4);
    assert_eq!(pcm, [16_384, -16_384, 8_192, -8_192]);
}

#[test]
fn existing_invalid_audio_source_reports_unsupported_format() {
    let path =
        std::env::temp_dir().join(format!("worker-common-invalid-{}.wav", std::process::id()));
    std::fs::write(&path, [0_u8; 128]).unwrap();
    let request = DecodeRequest {
        source: SourceIdentity {
            source_id: path.to_string_lossy().into_owned(),
            sha256: [0; 32],
        },
        job_id: JobId(907),
        generation: Generation::first(),
        range: SourceRange::new(0, 4, OUTPUT_RATE).unwrap(),
        max_samples: 4,
    };
    let error = decode_requested_block(&request).unwrap_err();
    assert!(
        error.starts_with("UnsupportedFormat: audio probe failed:"),
        "{error}"
    );
}
