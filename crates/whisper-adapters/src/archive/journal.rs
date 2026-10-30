use super::{
    ArchiveStore, BufRead, BufReader, File, Generation, Instant, JobId, OpenOptions, Path, Read,
    Seek, SeekFrom, SegmentRecord, SourceRange, WorkerSegment, Write,
};

impl ArchiveStore {
    pub fn persist_segment(&mut self, segment: &WorkerSegment) -> Result<u64, String> {
        let started = Instant::now();
        let active = self
            .current
            .as_mut()
            .ok_or_else(|| "StorageUnavailable: no prepared archive".to_owned())?;
        if segment.job_id != active.job_id || segment.generation != active.generation {
            return Err("StaleResponse: segment does not match pending generation".into());
        }
        if segment.segment_id.0 != active.expected_sequence
            || segment.range.sample_rate_hz != 16_000
            || segment.range.start_sample >= segment.range.end_sample
            || segment.range.end_sample > active.source_samples
            || segment.text.trim().is_empty()
        {
            return Err("StorageCorrupt: invalid segment sequence/range/content".into());
        }
        let record = SegmentRecord {
            sequence: segment.segment_id.0,
            range: segment.range,
            text: segment.text.clone(),
        };
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(active.directory.join("segments.jsonl"))
            .map_err(|e| format!("StorageUnavailable: {e}"))?;
        serde_json::to_writer(&mut file, &record).map_err(|e| e.to_string())?;
        file.write_all(b"\n").map_err(|e| e.to_string())?;
        file.sync_all()
            .map_err(|e| format!("StorageUnavailable: segment sync failed: {e}"))?;
        active.expected_sequence += 1;
        crate::diagnostics::record_stage_timing(
            "archive_persist_segment",
            started.elapsed(),
            serde_json::json!({ "sequence": record.sequence, "text_bytes": record.text.len() }),
        );
        Ok(record.sequence)
    }
}

pub(super) fn read_segments(path: &Path) -> Result<Vec<SegmentRecord>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|e| format!("StorageCorrupt: segment journal: {e}"))?;
    repair_uncommitted_segment_tail(&mut file)?;
    file.seek(SeekFrom::Start(0))
        .map_err(|e| format!("StorageCorrupt: segment seek: {e}"))?;
    BufReader::new(file)
        .lines()
        .map(|line| {
            let line = line.map_err(|e| format!("StorageCorrupt: segment: {e}"))?;
            serde_json::from_str(&line).map_err(|e| format!("StorageCorrupt: segment: {e}"))
        })
        .collect()
}

pub(super) fn validate_segment_journal(path: &Path, expected_count: u64) -> Result<(), String> {
    let mut observed = 0_u64;
    visit_segments(path, |index, record| {
        let expected = index as u64 + 1;
        if record.sequence != expected {
            return Err("StorageCorrupt: segment journal sequence is not contiguous".into());
        }
        observed = expected;
        Ok(())
    })?;
    if observed != expected_count {
        return Err("StorageCorrupt: segment journal is incomplete".into());
    }
    Ok(())
}

pub(super) fn visit_segments(
    path: &Path,
    mut visit: impl FnMut(usize, SegmentRecord) -> Result<(), String>,
) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|error| format!("StorageCorrupt: segment journal: {error}"))?;
    repair_uncommitted_segment_tail(&mut file)?;
    file.seek(SeekFrom::Start(0))
        .map_err(|error| format!("StorageCorrupt: segment seek: {error}"))?;
    let mut reader = BufReader::new(file);
    let mut index = 0_usize;
    while let Some(line) = read_segment_journal_line(&mut reader)? {
        let record: SegmentRecord = serde_json::from_slice(&line)
            .map_err(|error| format!("StorageCorrupt: segment: {error}"))?;
        visit(index, record)?;
        index = index
            .checked_add(1)
            .ok_or_else(|| "StorageCorrupt: segment count exceeds address space".to_owned())?;
    }
    Ok(())
}

pub(super) fn visit_segments_read_only(
    path: &Path,
    mut visit: impl FnMut(usize, SegmentRecord) -> Result<(), String>,
) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    let file =
        File::open(path).map_err(|error| format!("StorageCorrupt: segment journal: {error}"))?;
    let mut reader = BufReader::new(file);
    let mut index = 0_usize;
    while let Some(line) = read_segment_journal_line(&mut reader)? {
        let record: SegmentRecord = serde_json::from_slice(&line)
            .map_err(|error| format!("StorageCorrupt: segment: {error}"))?;
        visit(index, record)?;
        index = index
            .checked_add(1)
            .ok_or_else(|| "StorageCorrupt: segment count exceeds address space".to_owned())?;
    }
    Ok(())
}

pub(super) fn validate_unified_resume_prefix(
    segment_journal: &Path,
    document_directory: &Path,
    job_id: JobId,
    generation: Generation,
    source_samples: u64,
    checkpoint: crate::parakeet_document::ResumeCoverageCheckpoint,
) -> Result<Option<SourceRange>, String> {
    if checkpoint.confirmed_segments == 0 && checkpoint.window_sequence == 0 {
        if checkpoint.covered_samples != 0 {
            return Err("StorageCorrupt: empty resume checkpoint has a sample offset".into());
        }
        return Ok(None);
    }
    let mut document = crate::parakeet_document::ParakeetDocumentStore::open_resume_coverage(
        document_directory,
        job_id,
        generation,
        source_samples,
    )?
    .ok_or_else(|| "StorageCorrupt: resume checkpoint document is absent".to_owned())?;
    let mut checkpoint_seen = false;
    loop {
        let page = document.read_page(128)?;
        if page.is_empty() {
            break;
        }
        for coverage in page {
            if coverage.sequence == checkpoint.window_sequence {
                if coverage.range.end_sample != checkpoint.covered_samples
                    || coverage.last_segment_sequence != checkpoint.confirmed_segments
                {
                    return Err(
                        "StorageCorrupt: resume checkpoint changed during validation".into(),
                    );
                }
                checkpoint_seen = true;
                break;
            }
        }
        if checkpoint_seen {
            break;
        }
    }
    if !checkpoint_seen {
        return Err("StorageCorrupt: selected resume checkpoint is missing".into());
    }
    if checkpoint.confirmed_segments == 0 {
        return Ok(None);
    }
    let mut document = crate::parakeet_document::ParakeetDocumentStore::open_resume_coverage(
        document_directory,
        job_id,
        generation,
        source_samples,
    )?
    .ok_or_else(|| "StorageCorrupt: resume checkpoint document is absent".to_owned())?;
    let mut archive = BufReader::new(
        File::open(segment_journal)
            .map_err(|error| format!("StorageCorrupt: resume segment journal: {error}"))?,
    );
    let mut next_sequence = 1_u64;
    let mut anchor = None;
    while next_sequence <= checkpoint.confirmed_segments {
        let remaining = checkpoint
            .confirmed_segments
            .checked_sub(next_sequence)
            .and_then(|count| count.checked_add(1))
            .ok_or_else(|| "StorageCorrupt: resume prefix sequence underflow".to_owned())?;
        let page_limit = usize::try_from(remaining.min(128))
            .map_err(|_| "StorageCorrupt: resume prefix page limit".to_owned())?;
        let page = document.read_document_page(next_sequence, page_limit)?;
        if page.is_empty() {
            return Err("StorageCorrupt: resume document prefix is incomplete".into());
        }
        for entry in page {
            let Some(line) = read_segment_journal_line(&mut archive)? else {
                return Err("StorageCorrupt: resume segment prefix is incomplete".into());
            };
            let record: SegmentRecord = serde_json::from_slice(&line)
                .map_err(|error| format!("StorageCorrupt: resume segment record: {error}"))?;
            if record.sequence != next_sequence
                || entry.segment.segment_id.0 != next_sequence
                || entry.segment.range != record.range
                || entry.segment.text != record.text
            {
                return Err("StorageCorrupt: resume archive and document prefixes differ".into());
            }
            anchor = Some(record.range);
            next_sequence = next_sequence
                .checked_add(1)
                .ok_or_else(|| "StorageCorrupt: resume segment sequence exhausted".to_owned())?;
        }
    }
    if checkpoint.window_sequence == 0
        || checkpoint.covered_samples == 0
        || checkpoint.covered_samples > source_samples
        || anchor.is_none_or(|range| {
            range.start_sample + (range.end_sample - range.start_sample) / 2
                >= checkpoint.covered_samples
        })
    {
        return Err("StorageCorrupt: resume checkpoint boundary is invalid".into());
    }
    Ok(anchor)
}

fn read_segment_journal_line(reader: &mut BufReader<File>) -> Result<Option<Vec<u8>>, String> {
    const MAX_SEGMENT_RECORD_BYTES: usize = 1_048_576;
    let mut line = Vec::new();
    loop {
        let available = reader
            .fill_buf()
            .map_err(|error| format!("StorageCorrupt: resume segment read: {error}"))?;
        if available.is_empty() {
            if line.is_empty() {
                return Ok(None);
            }
            if serde_json::from_slice::<SegmentRecord>(&line).is_ok() {
                return Ok(Some(line));
            }
            return Err("StorageCorrupt: truncated resume segment record".into());
        }
        let take = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(available.len(), |index| index + 1);
        if line.len().saturating_add(take) > MAX_SEGMENT_RECORD_BYTES {
            return Err("StorageCorrupt: resume segment record exceeds its bound".into());
        }
        let complete = available.get(take - 1) == Some(&b'\n');
        line.extend_from_slice(&available[..take]);
        reader.consume(take);
        if complete {
            line.pop();
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            return Ok(Some(line));
        }
    }
}

fn repair_uncommitted_segment_tail(file: &mut File) -> Result<(), String> {
    const SCAN_BYTES: usize = 8 * 1024;
    let length = file
        .metadata()
        .map_err(|e| format!("StorageCorrupt: segment metadata: {e}"))?
        .len();
    if length == 0 {
        return Ok(());
    }
    file.seek(SeekFrom::End(-1))
        .map_err(|e| format!("StorageCorrupt: segment seek: {e}"))?;
    let mut last = [0_u8; 1];
    file.read_exact(&mut last)
        .map_err(|e| format!("StorageCorrupt: segment read: {e}"))?;
    if last[0] == b'\n' {
        return Ok(());
    }
    let mut end = length;
    let mut valid_len = 0_u64;
    let mut chunk = vec![0_u8; SCAN_BYTES];
    while end > 0 {
        let start = end.saturating_sub(SCAN_BYTES as u64);
        let count = (end - start) as usize;
        file.seek(SeekFrom::Start(start))
            .map_err(|e| format!("StorageCorrupt: segment tail scan: {e}"))?;
        file.read_exact(&mut chunk[..count])
            .map_err(|e| format!("StorageCorrupt: segment tail scan: {e}"))?;
        if let Some(index) = chunk[..count].iter().rposition(|byte| *byte == b'\n') {
            valid_len = start + index as u64 + 1;
            break;
        }
        end = start;
    }
    file.set_len(valid_len)
        .and_then(|()| file.sync_all())
        .map_err(|e| format!("StorageUnavailable: segment tail repair failed: {e}"))
}
