use crate::{
    decoder,
    integrity::sha256_file,
    runtime_paths::{ensure_inside, required_runtime_root},
};
use std::{env, io::Write, path::PathBuf};
use whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS;

pub(crate) fn decode_only_mode() -> Result<(), String> {
    let mut args = env::args_os().skip(2);
    let source = args
        .next()
        .map(PathBuf::from)
        .ok_or("DecodeOnly: missing source path")?;
    let output = args
        .next()
        .map(PathBuf::from)
        .ok_or("DecodeOnly: missing output path")?;
    if args.next().is_some() {
        return Err("DecodeOnly: unexpected argument".into());
    }
    let runtime = required_runtime_root(PARAKEET_PROFILE_IMPORT_SINGLE_PASS)?;
    let output_parent = output
        .parent()
        .ok_or("DecodeOnly: output has no parent")?
        .canonicalize()
        .map_err(|e| format!("DecodeOnlyOutput: {e}"))?;
    ensure_inside(&output_parent, &runtime)?;
    let decoded = decoder::decode_file(&source)?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&output)
        .map_err(|e| format!("DecodeOnlyOutput: {e}"))?;
    for sample in &decoded.samples {
        file.write_all(&sample.to_le_bytes())
            .map_err(|e| e.to_string())?;
    }
    file.sync_all().map_err(|e| e.to_string())?;
    println!(
        "{}",
        serde_json::json!({
            "source_sha256": sha256_file(&source)?, "output_samples": decoded.samples.len(),
            "source_frames": decoded.source_frames, "source_rate": decoded.source_rate,
            "channels": decoded.channels, "codec": decoded.codec,
        })
    );
    Ok(())
}
