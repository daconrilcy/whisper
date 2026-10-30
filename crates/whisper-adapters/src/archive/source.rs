use super::{
    ArchiveStore, CapturedPartReference, CapturedSourceManifest, Digest, File, ImportRequest,
    Instant, Path, Read, Sha256, fs,
};
use symphonia::core::audio::Channels;
use symphonia::core::codecs::CODEC_TYPE_MP3;
use symphonia::core::{
    formats::FormatOptions, io::MediaSourceStream, meta::MetadataOptions, probe::Hint,
};

impl ArchiveStore {
    /// Captures the source identity at durable queue admission without creating an archive.
    pub fn identify_source(request: &mut ImportRequest) -> Result<(), String> {
        let source = Path::new(&request.source_path);
        if !source.is_file() {
            return Err("SourceMissing: selected audio source does not exist".into());
        }
        let source_sha = sha256_file(source)?;
        let source_samples = source_output_samples(source)?;
        if sha256_file(source)? != source_sha {
            return Err("SourceChanged: source changed while its identity was inspected".into());
        }
        request.source_sha256 = Some(source_sha);
        request.source_samples = Some(source_samples);
        Ok(())
    }
}

pub(super) fn segmented_pcm_manifest(
    source: &crate::parakeet_audio::PcmPartStore,
) -> Result<([u8; 32], u64, CapturedSourceManifest), String> {
    let total_samples = source
        .total_samples()
        .map_err(|error| format!("AudioSource: {error}"))?;
    if total_samples == 0 {
        return Err("InvalidInput: captured PCM source is empty".into());
    }
    let index_path = source.base_path().with_extension("parts.idx");
    let index_before = fs::read(&index_path)
        .map_err(|error| format!("AudioSource: part index unavailable: {error}"))?;
    let mut digest = Sha256::new();
    digest.update(b"whisper-parakeet-pcm-parts-v1\0");
    digest.update(total_samples.to_le_bytes());
    digest.update((index_before.len() as u64).to_le_bytes());
    digest.update(&index_before);
    let mut parts = Vec::new();
    let mut offset = 0_u64;
    let mut covered = 0_u64;
    loop {
        let page = source
            .read_page(offset, 512)
            .map_err(|error| format!("AudioSource: part index: {error}"))?;
        if page.is_empty() {
            break;
        }
        for part in &page {
            if part.start_sample != covered || part.end_sample < part.start_sample {
                return Err("AudioSource: captured part index is not contiguous".into());
            }
            let expected_bytes = part
                .end_sample
                .saturating_sub(part.start_sample)
                .checked_mul(2)
                .and_then(|bytes| bytes.checked_add(44))
                .ok_or_else(|| "AudioSource: part byte count overflow".to_owned())?;
            let metadata = fs::metadata(&part.path).map_err(|error| {
                format!("AudioSource: part {} unavailable: {error}", part.number)
            })?;
            if metadata.len() != expected_bytes {
                return Err(format!(
                    "AudioSource: captured part {} has an invalid length",
                    part.number
                ));
            }
            digest.update(part.number.to_le_bytes());
            digest.update(part.start_sample.to_le_bytes());
            digest.update(part.end_sample.to_le_bytes());
            digest.update(metadata.len().to_le_bytes());
            let part_sha = sha256_file(&part.path)?;
            digest.update(part_sha);
            parts.push(CapturedPartReference {
                number: part.number,
                start_sample: part.start_sample,
                end_sample: part.end_sample,
                path: part.path.to_string_lossy().into_owned(),
                sha256: hex(&part_sha),
                bytes: metadata.len(),
            });
            covered = part.end_sample;
        }
        offset = offset
            .checked_add(page.len() as u64)
            .ok_or_else(|| "AudioSource: part index page overflow".to_owned())?;
    }
    if covered != total_samples
        || fs::read(&index_path).ok().as_deref() != Some(index_before.as_slice())
    {
        return Err(
            "SourceChanged: captured PCM parts changed while their identity was computed".into(),
        );
    }
    let identity: [u8; 32] = digest.finalize().into();
    let manifest = CapturedSourceManifest {
        version: 1,
        source_sha256: hex(&identity),
        source_samples: total_samples,
        index_path: index_path.to_string_lossy().into_owned(),
        index_sha256: hex(&Sha256::digest(&index_before)),
        parts,
    };
    Ok((identity, total_samples, manifest))
}

pub fn sha256_file(path: &Path) -> Result<[u8; 32], String> {
    let started = Instant::now();
    let mut file = File::open(path).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    let digest: [u8; 32] = hash.finalize().into();
    crate::diagnostics::record_stage_timing(
        "archive_sha256",
        started.elapsed(),
        serde_json::json!({
            "file": path.file_name().and_then(|name| name.to_str()).unwrap_or("unknown"),
            "bytes": file.metadata().map(|meta| meta.len()).unwrap_or(0),
            "sha256": hex(&digest),
        }),
    );
    Ok(digest)
}

pub(crate) fn source_output_samples(path: &Path) -> Result<u64, String> {
    let started = Instant::now();
    let result = source_output_samples_inner(path);
    crate::diagnostics::record_stage_timing(
        "archive_audio_probe",
        started.elapsed(),
        serde_json::json!({
            "file": path.file_name().and_then(|name| name.to_str()).unwrap_or("unknown"),
            "samples": result.as_ref().ok().copied(),
            "error": result.as_ref().err(),
        }),
    );
    result
}

fn source_output_samples_inner(path: &Path) -> Result<u64, String> {
    let file = File::open(path).map_err(|e| format!("SourceMissing: {e}"))?;
    let source = MediaSourceStream::new(Box::new(file), Default::default());
    let hint = Hint::new();
    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            source,
            &FormatOptions {
                enable_gapless: true,
                ..Default::default()
            },
            &MetadataOptions::default(),
        )
        .map_err(|e| format!("UnsupportedFormat: audio probe failed: {e}"))?;
    let track = probed
        .format
        .default_track()
        .ok_or_else(|| "UnsupportedFormat: audio has no track".to_owned())?;
    if track.codec_params.codec == CODEC_TYPE_MP3
        && (track.codec_params.channels.is_none()
            || track.codec_params.sample_rate.is_none()
            || track.codec_params.n_frames.is_none())
    {
        return Err("UnsupportedFormat: MP3 profile or duration metadata is incomplete".into());
    }
    if track.codec_params.codec == CODEC_TYPE_MP3 {
        let sample_rate = track.codec_params.sample_rate.unwrap_or_default();
        let channels = track.codec_params.channels;
        let stereo = Channels::FRONT_LEFT | Channels::FRONT_RIGHT;
        let supported_layout = channels == Some(Channels::FRONT_LEFT) || channels == Some(stereo);
        if !matches!(
            sample_rate,
            8_000 | 11_025 | 12_000 | 16_000 | 22_050 | 24_000 | 32_000 | 44_100 | 48_000
        ) || !supported_layout
        {
            return Err(
                "UnsupportedFormat: MP3 sample rate or channel layout is outside Q-07".into(),
            );
        }
    }
    let frames = track
        .codec_params
        .n_frames
        .ok_or_else(|| "UnsupportedFormat: audio duration is unavailable".to_owned())?;
    let input_rate = u64::from(
        track
            .codec_params
            .sample_rate
            .filter(|rate| *rate > 0)
            .ok_or_else(|| "UnsupportedFormat: audio sample rate is unavailable".to_owned())?,
    );
    if frames == 0 {
        return Err("UnsupportedFormat: audio contains no samples".into());
    }
    frames
        .saturating_sub(1)
        .checked_mul(16_000)
        .map(|scaled| scaled / input_rate + 1)
        .ok_or_else(|| "UnsupportedFormat: audio duration exceeds supported range".into())
}

pub(super) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
