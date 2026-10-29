use super::*;
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
