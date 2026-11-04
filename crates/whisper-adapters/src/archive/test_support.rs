use super::LiveArchive;
use std::fs::File;

pub(crate) fn replace_mp3_handle_with_read_only(archive: &mut LiveArchive) -> std::io::Result<()> {
    archive.mp3 = Some(File::open(&archive.mp3_pending_path)?);
    Ok(())
}
