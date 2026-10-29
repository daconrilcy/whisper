#[cfg(feature = "parakeet-qualification-reference")]
use crate::reference_decoder::decode_reference_wav;
use crate::{
    decoder,
    integrity::{constant_time_eq, sha256_file},
    native_engine, record_r7_timing,
};
use std::{path::Path, time::Instant};
use whisper_core::parakeet_windows::PROFILE_QUALIFICATION_REFERENCE;

#[cfg(test)]
mod tests;

fn classify_native_error(error: String) -> (&'static str, String) {
    let code = match error.as_str() {
        value if value.starts_with("InvalidTimestamp") => "InvalidTimestamp",
        value if value.starts_with("NativeCallStarted:") => "IpcWrite",
        _ => "InferenceFailed",
    };
    (code, error)
}

pub(crate) fn transcribe_one(
    engine: &mut native_engine::Engine,
    source: &Path,
    expected_source_sha: &str,
    profile: &str,
    on_native_call_started: impl FnOnce() -> Result<(), String>,
) -> Result<
    (
        u64,
        String,
        Vec<whisper_core::parakeet_ipc::Word>,
        String,
        u64,
    ),
    (&'static str, String),
> {
    let canonical = source
        .canonicalize()
        .map_err(|e| ("SourceUnavailable", e.to_string()))?;
    if !canonical.is_file() {
        return Err(("SourceUnavailable", "source is not a file".into()));
    }
    let actual_sha = sha256_file(&canonical).map_err(|e| ("SourceUnavailable", e))?;
    if !constant_time_eq(expected_source_sha, &actual_sha) {
        return Err((
            "SourceHashMismatch",
            "source SHA-256 differs from request".into(),
        ));
    }
    let decode_started = Instant::now();
    let decoded = if profile == PROFILE_QUALIFICATION_REFERENCE {
        #[cfg(feature = "parakeet-qualification-reference")]
        {
            decode_reference_wav(&canonical).map_err(|error| ("DecodeFailed", error))?
        }
        #[cfg(not(feature = "parakeet-qualification-reference"))]
        {
            return Err((
                "QualificationReferenceUnavailable",
                "worker lacks the private reference feature".into(),
            ));
        }
    } else {
        decoder::decode_file(&canonical).map_err(|error| ("DecodeFailed", error))?
    };
    record_r7_timing(
        "audio_decode",
        decode_started.elapsed(),
        serde_json::json!({
            "samples": decoded.samples.len(), "source_sha256": actual_sha,
        }),
    );
    let duration_samples = decoded.samples.len() as u64;
    let result = if profile == PROFILE_QUALIFICATION_REFERENCE {
        #[cfg(feature = "parakeet-qualification-reference")]
        {
            engine.transcribe_reference_with_start_callback(&decoded, on_native_call_started)
        }
        #[cfg(not(feature = "parakeet-qualification-reference"))]
        {
            Err("QualificationReferenceUnavailable".to_owned())
        }
    } else {
        engine.transcribe_with_start_callback(&decoded, on_native_call_started)
    };
    let (text, words, native_inference_ms) = result.map_err(classify_native_error)?;
    Ok((
        duration_samples,
        text,
        words,
        actual_sha,
        native_inference_ms,
    ))
}
