use super::{File, Instant, OpenOptions, Path, Write, fs, io};

pub(super) fn write_replace_streamed(
    path: &Path,
    write_content: impl FnOnce(&mut std::io::BufWriter<File>) -> Result<(), String>,
) -> Result<(), String> {
    let temporary = path.with_extension(format!(
        "{}.tmp",
        path.extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or("data")
    ));
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&temporary)
        .map_err(|error| format!("StorageUnavailable: {error}"))?;
    let mut writer = std::io::BufWriter::new(file);
    write_content(&mut writer)?;
    writer
        .flush()
        .map_err(|error| format!("StorageUnavailable: {error}"))?;
    let file = writer
        .into_inner()
        .map_err(|error| format!("StorageUnavailable: {}", error.error()))?;
    file.sync_all()
        .map_err(|error| format!("StorageUnavailable: {error}"))?;
    drop(file);
    fs::rename(temporary, path).map_err(|error| format!("StorageUnavailable: {error}"))
}

pub(super) fn copy_replace_streamed(source: &Path, destination: &Path) -> Result<(), String> {
    write_replace_streamed(destination, |writer| {
        let mut source = File::open(source).map_err(|error| error.to_string())?;
        io::copy(&mut source, writer).map_err(|error| error.to_string())?;
        Ok(())
    })
}

pub(super) fn write_new_synced(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("StorageUnavailable: {e}"))?;
    file.write_all(bytes).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())
}
pub(super) fn write_replace_synced(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let started = Instant::now();
    let temporary = path.with_extension(format!(
        "{}.tmp",
        path.extension().and_then(|x| x.to_str()).unwrap_or("data")
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&temporary)
        .map_err(|e| e.to_string())?;
    file.write_all(bytes).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    drop(file);
    fs::rename(temporary, path).map_err(|e| e.to_string())?;
    crate::diagnostics::record_stage_timing(
        "archive_write_replace",
        started.elapsed(),
        serde_json::json!({
            "file": path.file_name().and_then(|name| name.to_str()).unwrap_or("unknown"),
            "bytes": bytes.len(),
        }),
    );
    Ok(())
}
