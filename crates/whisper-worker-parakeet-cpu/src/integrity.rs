use crate::record_r7_timing;
use sha2::{Digest, Sha256};
use std::{fs::File, io, path::Path, time::Instant};

pub(crate) fn runtime_sha256(native_dir: &Path, native_version: &str) -> Result<String, String> {
    let mut hasher = Sha256::new();
    hasher.update(native_version.as_bytes());
    for name in [
        "nemo_speech_asr_c.dll",
        "nemo_speech_asr.dll",
        "ggml.dll",
        "ggml-base.dll",
        "ggml-cpu.dll",
    ] {
        hasher.update(name.as_bytes());
        hasher.update(sha256_file(&native_dir.join(name))?.as_bytes());
    }
    #[cfg(feature = "parakeet-cuda")]
    for name in [
        "ggml-cuda.dll",
        "cudart64_12.dll",
        "cublas64_12.dll",
        "cublasLt64_12.dll",
    ] {
        hasher.update(name.as_bytes());
        hasher.update(sha256_file(&native_dir.join(name))?.as_bytes());
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub(crate) fn sha256_file(path: &Path) -> Result<String, String> {
    let started = Instant::now();
    let mut file = File::open(path).map_err(|e| format!("HashOpen: {e}"))?;
    let mut hasher = Sha256::new();
    let bytes = io::copy(&mut file, &mut hasher).map_err(|e| format!("HashRead: {e}"))?;
    let sha = format!("{:x}", hasher.finalize());
    record_r7_timing(
        "sha256",
        started.elapsed(),
        serde_json::json!({
            "file": path.file_name().and_then(|name| name.to_str()).unwrap_or("unknown"),
            "bytes": bytes,
            "sha256": sha,
        }),
    );
    Ok(sha)
}

pub(crate) fn constant_time_eq(left: &str, right: &str) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.bytes()
        .zip(right.bytes())
        .fold(0_u8, |diff, (a, b)| diff | (a ^ b))
        == 0
}
