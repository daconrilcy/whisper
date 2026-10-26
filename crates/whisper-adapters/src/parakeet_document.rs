//! Durable paginated final transcript for one Parakeet session.

use std::{
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};
use whisper_core::{
    Generation, JobId,
    ports::{ParakeetDocumentEntry, ParakeetDocumentPage, ParakeetDocumentPort, ParakeetWindowCoverage, PortError, WorkerSegment},
};

const INDEX_HEADER: &[u8; 8] = b"PUDOC001";
const INDEX_ENTRY_BYTES: u64 = 16;
const MAX_PAGE_SIZE: usize = 128;

pub struct ParakeetDocumentStore {
    directory: PathBuf,
    job_id: JobId,
    generation: Generation,
    revision: u64,
    next_sequence: u64,
    next_window: u64,
    covered_samples: u64,
    covered_segment_sequence: u64,
    finalized: bool,
}

impl ParakeetDocumentStore {
    pub fn create(directory: impl AsRef<Path>, job_id: JobId, generation: Generation) -> Result<Self, PortError> {
        let directory = directory.as_ref().to_path_buf();
        fs::create_dir_all(&directory).map_err(storage_error)?;
        let mut index = OpenOptions::new().write(true).create_new(true).open(directory.join("document.idx")).map_err(storage_error)?;
        index.write_all(INDEX_HEADER).map_err(storage_error)?;
        OpenOptions::new().write(true).create_new(true).open(directory.join("document.jsonl")).map_err(storage_error)?;
        OpenOptions::new().write(true).create_new(true).open(directory.join("coverage.jsonl")).map_err(storage_error)?;
        Ok(Self { directory, job_id, generation, revision: 0, next_sequence: 1, next_window: 1, covered_samples: 0, covered_segment_sequence: 0, finalized: false })
    }

    pub fn revision(&self) -> u64 { self.revision }

    fn check_identity(&self, job_id: JobId, generation: Generation) -> Result<(), PortError> {
        if self.job_id == job_id && self.generation == generation { Ok(()) } else { Err(PortError::StaleResponse) }
    }
}

impl ParakeetDocumentPort for ParakeetDocumentStore {
    fn append_final(&mut self, segment: WorkerSegment) -> Result<u64, PortError> {
        self.check_identity(segment.job_id, segment.generation)?;
        if self.finalized { return Err(PortError::Committing); }
        if segment.segment_id.0 != self.next_sequence
            || segment.range.sample_rate_hz != 16_000
            || segment.range.start_sample >= segment.range.end_sample
            || segment.text.trim().is_empty()
        { return Err(PortError::InvalidInput); }
        let revision = self.revision.checked_add(1).ok_or(PortError::Unavailable)?;
        let next_sequence = self.next_sequence.checked_add(1).ok_or(PortError::Unavailable)?;
        let entry = ParakeetDocumentEntry { segment, revision };
        let mut document = OpenOptions::new().append(true).open(self.directory.join("document.jsonl")).map_err(storage_error)?;
        let offset = document.metadata().map_err(storage_error)?.len();
        serde_json::to_writer(&mut document, &entry).map_err(|error| PortError::Failed(error.to_string()))?;
        document.write_all(b"\n").map_err(storage_error)?;
        document.sync_all().map_err(storage_error)?;
        let length = document.metadata().map_err(storage_error)?.len().checked_sub(offset).ok_or(PortError::Unavailable)?;
        drop(document);
        let mut index = OpenOptions::new().append(true).open(self.directory.join("document.idx")).map_err(storage_error)?;
        index.write_all(&offset.to_le_bytes()).map_err(storage_error)?;
        index.write_all(&length.to_le_bytes()).map_err(storage_error)?;
        index.sync_all().map_err(storage_error)?;
        self.revision = revision;
        self.next_sequence = next_sequence;
        Ok(revision)
    }

    fn record_coverage(&mut self, coverage: ParakeetWindowCoverage) -> Result<(), PortError> {
        if self.finalized || coverage.sequence != self.next_window
            || coverage.range.sample_rate_hz != 16_000
            || coverage.range.start_sample != self.covered_samples
            || coverage.range.end_sample <= coverage.range.start_sample
            || coverage.last_segment_sequence.checked_add(1) != Some(self.next_sequence)
        { return Err(PortError::InvalidInput); }
        let mut sequence = self.covered_segment_sequence.saturating_add(1);
        while sequence <= coverage.last_segment_sequence {
            let page = self.read_page(self.job_id, self.generation, self.revision, sequence, MAX_PAGE_SIZE)?;
            if page.entries.is_empty() { return Err(PortError::Failed("StorageCorrupt: coverage references a missing document entry".into())); }
            for entry in page.entries {
                if entry.segment.segment_id.0 > coverage.last_segment_sequence {
                    sequence = entry.segment.segment_id.0;
                    break;
                }
                let range = entry.segment.range;
                let midpoint = range.start_sample + (range.end_sample - range.start_sample) / 2;
                if range.start_sample >= range.end_sample
                    || coverage.range.start_sample > midpoint
                    || midpoint >= coverage.range.end_sample
                { return Err(PortError::Failed("StorageCorrupt: final word is outside its central audio window".into())); }
                sequence = entry.segment.segment_id.0.saturating_add(1);
            }
        }
        let mut journal = OpenOptions::new().append(true).open(self.directory.join("coverage.jsonl")).map_err(storage_error)?;
        serde_json::to_writer(&mut journal, &coverage).map_err(|error| PortError::Failed(error.to_string()))?;
        journal.write_all(b"\n").map_err(storage_error)?;
        journal.sync_all().map_err(storage_error)?;
        self.covered_samples = coverage.range.end_sample;
        self.covered_segment_sequence = coverage.last_segment_sequence;
        self.next_window = self.next_window.checked_add(1).ok_or(PortError::Unavailable)?;
        Ok(())
    }

    fn read_page(
        &mut self,
        job_id: JobId,
        generation: Generation,
        expected_revision: u64,
        first_sequence: u64,
        limit: usize,
    ) -> Result<ParakeetDocumentPage, PortError> {
        self.check_identity(job_id, generation)?;
        if expected_revision != self.revision { return Err(PortError::StaleResponse); }
        if first_sequence == 0 || limit == 0 { return Err(PortError::InvalidInput); }
        let limit = limit.min(MAX_PAGE_SIZE);
        let count = self.revision;
        let mut index = File::open(self.directory.join("document.idx")).map_err(storage_error)?;
        let mut header = [0_u8; 8];
        std::io::Read::read_exact(&mut index, &mut header).map_err(storage_error)?;
        let index_size = index.metadata().map_err(storage_error)?.len();
        if &header != INDEX_HEADER || index_size < 8 || (index_size - 8) % INDEX_ENTRY_BYTES != 0 {
            return Err(PortError::Failed("StorageCorrupt: document index".into()));
        }
        let mut document = BufReader::new(File::open(self.directory.join("document.jsonl")).map_err(storage_error)?);
        let mut entries = Vec::with_capacity(limit);
        let mut sequence = first_sequence;
        while sequence <= count && entries.len() < limit {
            let index_offset = 8_u64.checked_add((sequence - 1).checked_mul(INDEX_ENTRY_BYTES).ok_or(PortError::Unavailable)?).ok_or(PortError::Unavailable)?;
            index.seek(SeekFrom::Start(index_offset)).map_err(storage_error)?;
            let offset = read_u64(&mut index)?;
            let length = read_u64(&mut index)?;
            let line_length = usize::try_from(length).map_err(|_| PortError::Unavailable)?;
            if line_length == 0 || line_length > 1_048_576 { return Err(PortError::Failed("StorageCorrupt: document record size".into())); }
            document.seek(SeekFrom::Start(offset)).map_err(storage_error)?;
            let mut line = String::with_capacity(line_length);
            document.by_ref().take(length).read_line(&mut line).map_err(storage_error)?;
            if line.len() as u64 != length { return Err(PortError::Failed("StorageCorrupt: short document record".into())); }
            let entry: ParakeetDocumentEntry = serde_json::from_str(line.trim_end()).map_err(|error| PortError::Failed(format!("StorageCorrupt: document entry: {error}")))?;
            if entry.segment.job_id != job_id || entry.segment.generation != generation || entry.segment.segment_id.0 != sequence || entry.revision != sequence {
                return Err(PortError::StaleResponse);
            }
            entries.push(entry);
            sequence += 1;
        }
        let next_sequence = (sequence <= count).then_some(sequence);
        Ok(ParakeetDocumentPage { revision: self.revision, entries, next_sequence })
    }

    fn finalize(&mut self, job_id: JobId, generation: Generation, total_samples: u64) -> Result<u64, PortError> {
        self.check_identity(job_id, generation)?;
        if self.finalized { return Ok(self.revision); }
        if total_samples == 0 || self.covered_samples != total_samples {
            return Err(PortError::Failed("IncompleteAudio: final document lacks contiguous source coverage".into()));
        }
        let marker = serde_json::json!({ "version": 1, "job_id": format!("{:032x}", job_id.0), "generation": generation.get(), "revision": self.revision });
        let path = self.directory.join("document.final.json");
        let mut file = OpenOptions::new().write(true).create_new(true).open(path).map_err(storage_error)?;
        serde_json::to_writer(&mut file, &marker).map_err(|error| PortError::Failed(error.to_string()))?;
        file.sync_all().map_err(storage_error)?;
        self.finalized = true;
        Ok(self.revision)
    }
}

fn read_u64(reader: &mut impl std::io::Read) -> Result<u64, PortError> {
    let mut bytes = [0_u8; 8];
    reader.read_exact(&mut bytes).map_err(storage_error)?;
    Ok(u64::from_le_bytes(bytes))
}

fn storage_error(error: std::io::Error) -> PortError { PortError::Failed(format!("StorageUnavailable: {error}")) }

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn segment(id: JobId, generation: Generation, sequence: u64) -> WorkerSegment {
        WorkerSegment {
            job_id: id,
            generation,
            instance_id: 5,
            segment_id: whisper_core::SegmentId(sequence),
            range: whisper_core::SourceRange::new(sequence * 100, sequence * 100 + 50, 16_000).unwrap(),
            text: format!("word-{sequence}"),
        }
    }

    #[test]
    fn document_reads_bounded_pages_at_a_stable_revision_and_finalizes_once() {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let directory = std::env::temp_dir().join(format!("whisper-parakeet-document-{stamp}"));
        let id = JobId(23);
        let generation = Generation::first();
        let mut document = ParakeetDocumentStore::create(&directory, id, generation).unwrap();
        for sequence in 1..=3 { document.append_final(segment(id, generation, sequence)).unwrap(); }
        let first = document.read_page(id, generation, 3, 1, 2).unwrap();
        assert_eq!(first.entries.len(), 2);
        assert_eq!(first.next_sequence, Some(3));
        let second = document.read_page(id, generation, 3, 3, 2).unwrap();
        assert_eq!(second.entries[0].segment.text, "word-3");
        assert_eq!(document.read_page(id, generation, 2, 1, 1), Err(PortError::StaleResponse));
        document.record_coverage(ParakeetWindowCoverage {
            sequence: 1,
            range: whisper_core::SourceRange::new(0, 350, 16_000).unwrap(),
            last_segment_sequence: 3,
        }).unwrap();
        assert_eq!(document.finalize(id, generation, 350).unwrap(), 3);
        assert_eq!(document.finalize(id, generation, 350).unwrap(), 3);
        assert_eq!(document.append_final(segment(id, generation, 4)), Err(PortError::Committing));
        fs::remove_dir_all(directory).unwrap();
    }
}
