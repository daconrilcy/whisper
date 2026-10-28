//! Durable paginated final transcript for one Parakeet session.

use std::{
    fs::{self, File, OpenOptions},
    io::{BufRead, BufReader, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};
use whisper_core::{
    Generation, JobId,
    parakeet_windows::{WINDOW_MAX_SAMPLES, WINDOW_MIN_SAMPLES},
    ports::{
        ParakeetDocumentEntry, ParakeetDocumentPage, ParakeetDocumentPort, ParakeetPreview,
        ParakeetWindowCoverage, PortError, WorkerSegment,
    },
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResumeCoverageCheckpoint {
    pub window_sequence: u64,
    pub covered_samples: u64,
    pub confirmed_segments: u64,
}

pub struct ResumeCoverageReader {
    directory: PathBuf,
    reader: BufReader<File>,
    document_index: File,
    document: BufReader<File>,
    job_id: JobId,
    generation: Generation,
    source_samples: u64,
    document_revision: u64,
    next_document_sequence: u64,
    next_window_sequence: u64,
    covered_samples: u64,
    confirmed_segments: u64,
    finished: bool,
}

const INDEX_HEADER: &[u8; 8] = b"PUDOC001";
const INDEX_ENTRY_BYTES: u64 = 16;
const MAX_PAGE_SIZE: usize = 128;

pub struct ParakeetDocumentStore {
    directory: PathBuf,
    job_id: JobId,
    generation: Generation,
    revision: u64,
    preview_revision: u64,
    next_sequence: u64,
    next_window: u64,
    covered_samples: u64,
    covered_segment_sequence: u64,
    finalized: bool,
}

impl ParakeetDocumentStore {
    pub fn create(
        directory: impl AsRef<Path>,
        job_id: JobId,
        generation: Generation,
    ) -> Result<Self, PortError> {
        let directory = directory.as_ref().to_path_buf();
        fs::create_dir_all(&directory).map_err(storage_error)?;
        let mut index = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join("document.idx"))
            .map_err(storage_error)?;
        index.write_all(INDEX_HEADER).map_err(storage_error)?;
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join("document.jsonl"))
            .map_err(storage_error)?;
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(directory.join("coverage.jsonl"))
            .map_err(storage_error)?;
        Ok(Self {
            directory,
            job_id,
            generation,
            revision: 0,
            preview_revision: 0,
            next_sequence: 1,
            next_window: 1,
            covered_samples: 0,
            covered_segment_sequence: 0,
            finalized: false,
        })
    }

    pub fn revision(&self) -> u64 {
        self.revision
    }

    pub fn open_resume_coverage(
        directory: impl AsRef<Path>,
        job_id: JobId,
        generation: Generation,
        source_samples: u64,
    ) -> Result<Option<ResumeCoverageReader>, String> {
        ResumeCoverageReader::open(directory.as_ref(), job_id, generation, source_samples)
    }

    pub fn latest_resume_checkpoint(
        directory: impl AsRef<Path>,
        job_id: JobId,
        generation: Generation,
        source_samples: u64,
        maximum_confirmed_segments: u64,
    ) -> Result<Option<ResumeCoverageCheckpoint>, String> {
        let Some(mut reader) =
            Self::open_resume_coverage(directory, job_id, generation, source_samples)?
        else {
            return Ok(None);
        };
        let mut latest = None;
        loop {
            let page = reader.read_page(MAX_PAGE_SIZE)?;
            if page.is_empty() {
                break;
            }
            for coverage in page {
                if coverage.last_segment_sequence <= maximum_confirmed_segments {
                    latest = Some(ResumeCoverageCheckpoint {
                        window_sequence: coverage.sequence,
                        covered_samples: coverage.range.end_sample,
                        confirmed_segments: coverage.last_segment_sequence,
                    });
                }
            }
        }
        reader.validate_document_remainder()?;
        Ok(latest)
    }

    fn check_identity(&self, job_id: JobId, generation: Generation) -> Result<(), PortError> {
        if self.job_id == job_id && self.generation == generation {
            Ok(())
        } else {
            Err(PortError::StaleResponse)
        }
    }
}

impl ResumeCoverageReader {
    fn open(
        directory: &Path,
        job_id: JobId,
        generation: Generation,
        source_samples: u64,
    ) -> Result<Option<Self>, String> {
        if !directory.exists() {
            return Ok(None);
        }
        let mut document_index = File::open(directory.join("document.idx"))
            .map_err(|error| format!("StorageCorrupt: resume document index: {error}"))?;
        let mut header = [0_u8; 8];
        document_index
            .read_exact(&mut header)
            .map_err(|error| format!("StorageCorrupt: resume document index: {error}"))?;
        let index_size = document_index
            .metadata()
            .map_err(|error| format!("StorageCorrupt: resume document index: {error}"))?
            .len();
        if &header != INDEX_HEADER
            || index_size < INDEX_HEADER.len() as u64
            || (index_size - INDEX_HEADER.len() as u64) % INDEX_ENTRY_BYTES != 0
        {
            return Err("StorageCorrupt: resume document index".into());
        }
        let document_revision = (index_size - INDEX_HEADER.len() as u64) / INDEX_ENTRY_BYTES;
        let reader = match File::open(directory.join("coverage.jsonl")) {
            Ok(file) => BufReader::new(file),
            Err(error)
                if error.kind() == std::io::ErrorKind::NotFound && document_revision == 0 =>
            {
                return Ok(None);
            }
            Err(error) => {
                return Err(format!("StorageCorrupt: resume coverage: {error}"));
            }
        };
        let document = BufReader::new(
            File::open(directory.join("document.jsonl"))
                .map_err(|error| format!("StorageCorrupt: resume document: {error}"))?,
        );
        Ok(Some(Self {
            directory: directory.to_path_buf(),
            reader,
            document_index,
            document,
            job_id,
            generation,
            source_samples,
            document_revision,
            next_document_sequence: 1,
            next_window_sequence: 1,
            covered_samples: 0,
            confirmed_segments: 0,
            finished: false,
        }))
    }

    pub fn read_page(&mut self, limit: usize) -> Result<Vec<ParakeetWindowCoverage>, String> {
        if limit == 0 {
            return Err("InvalidInput: resume coverage page limit".into());
        }
        let mut page = Vec::with_capacity(limit.min(MAX_PAGE_SIZE));
        while page.len() < limit.min(MAX_PAGE_SIZE) && !self.finished {
            let Some(line) = read_bounded_line(&mut self.reader, 64 * 1024)? else {
                self.finished = true;
                break;
            };
            let coverage: ParakeetWindowCoverage = serde_json::from_slice(&line)
                .map_err(|error| format!("StorageCorrupt: resume coverage record: {error}"))?;
            let window_samples = coverage
                .range
                .end_sample
                .saturating_sub(coverage.range.start_sample);
            let terminal_window = coverage.range.end_sample == self.source_samples;
            if coverage.sequence != self.next_window_sequence
                || coverage.range.sample_rate_hz != 16_000
                || coverage.range.start_sample != self.covered_samples
                || coverage.range.end_sample <= coverage.range.start_sample
                || coverage.range.end_sample > self.source_samples
                || window_samples > WINDOW_MAX_SAMPLES
                || (!terminal_window && window_samples < WINDOW_MIN_SAMPLES)
                || coverage.last_segment_sequence < self.confirmed_segments
                || coverage.last_segment_sequence > self.document_revision
            {
                return Err(
                    "StorageCorrupt: resume coverage is not a valid contiguous checkpoint".into(),
                );
            }
            self.validate_document_through(coverage)?;
            self.covered_samples = coverage.range.end_sample;
            self.confirmed_segments = coverage.last_segment_sequence;
            self.next_window_sequence = self
                .next_window_sequence
                .checked_add(1)
                .ok_or_else(|| "StorageCorrupt: resume window sequence exhausted".to_owned())?;
            page.push(coverage);
        }
        Ok(page)
    }

    pub fn read_document_page(
        &self,
        first_sequence: u64,
        limit: usize,
    ) -> Result<Vec<ParakeetDocumentEntry>, String> {
        if first_sequence == 0 || limit == 0 {
            return Err("InvalidInput: resume document page".into());
        }
        let limit = limit.min(MAX_PAGE_SIZE);
        let index_path = self.directory.join("document.idx");
        let mut index = File::open(index_path)
            .map_err(|error| format!("StorageCorrupt: resume document index: {error}"))?;
        let mut header = [0_u8; 8];
        index
            .read_exact(&mut header)
            .map_err(|error| format!("StorageCorrupt: resume document index: {error}"))?;
        if &header != INDEX_HEADER {
            return Err("StorageCorrupt: resume document index header".into());
        }
        let count = self.document_revision;
        let mut document = BufReader::new(
            File::open(self.directory.join("document.jsonl"))
                .map_err(|error| format!("StorageCorrupt: resume document: {error}"))?,
        );
        let mut entries = Vec::with_capacity(limit);
        let mut sequence = first_sequence;
        while sequence <= count && entries.len() < limit {
            let offset = INDEX_HEADER.len() as u64
                + (sequence - 1)
                    .checked_mul(INDEX_ENTRY_BYTES)
                    .ok_or_else(|| "StorageCorrupt: resume document index overflow".to_owned())?;
            index
                .seek(SeekFrom::Start(offset))
                .map_err(|error| format!("StorageCorrupt: resume document index: {error}"))?;
            let record_offset = read_u64(&mut index)
                .map_err(|error| format!("StorageCorrupt: resume document index: {error:?}"))?;
            let length = read_u64(&mut index)
                .map_err(|error| format!("StorageCorrupt: resume document index: {error:?}"))?;
            let length = usize::try_from(length)
                .map_err(|_| "StorageCorrupt: resume document record length".to_owned())?;
            if length == 0 || length > 1_048_576 {
                return Err("StorageCorrupt: resume document record length".into());
            }
            document
                .seek(SeekFrom::Start(record_offset))
                .map_err(|error| format!("StorageCorrupt: resume document seek: {error}"))?;
            let mut line = vec![0; length];
            document
                .read_exact(&mut line)
                .map_err(|error| format!("StorageCorrupt: resume document record: {error}"))?;
            let entry: ParakeetDocumentEntry = serde_json::from_slice(&line)
                .map_err(|error| format!("StorageCorrupt: resume document entry: {error}"))?;
            if entry.segment.job_id != self.job_id
                || entry.segment.generation != self.generation
                || entry.segment.segment_id.0 != sequence
                || entry.revision != sequence
                || entry.segment.range.sample_rate_hz != 16_000
                || entry.segment.range.start_sample >= entry.segment.range.end_sample
                || entry.segment.range.end_sample > self.source_samples
                || entry.segment.text.trim().is_empty()
            {
                return Err("StorageCorrupt: resume document identity or sequence".into());
            }
            entries.push(entry);
            sequence = sequence
                .checked_add(1)
                .ok_or_else(|| "StorageCorrupt: resume document sequence exhausted".to_owned())?;
        }
        Ok(entries)
    }

    fn validate_document_remainder(&self) -> Result<(), String> {
        let mut sequence = self.next_document_sequence;
        while sequence <= self.document_revision {
            let page = self.read_document_page(sequence, MAX_PAGE_SIZE)?;
            if page.is_empty() {
                return Err("StorageCorrupt: resume document has a missing indexed record".into());
            }
            sequence = page
                .last()
                .and_then(|entry| entry.segment.segment_id.0.checked_add(1))
                .ok_or_else(|| "StorageCorrupt: resume document sequence exhausted".to_owned())?;
        }
        Ok(())
    }

    fn validate_document_through(
        &mut self,
        coverage: ParakeetWindowCoverage,
    ) -> Result<(), String> {
        while self.next_document_sequence <= coverage.last_segment_sequence {
            let index_offset = INDEX_HEADER.len() as u64
                + (self.next_document_sequence - 1)
                    .checked_mul(INDEX_ENTRY_BYTES)
                    .ok_or_else(|| "StorageCorrupt: resume document index overflow".to_owned())?;
            self.document_index
                .seek(SeekFrom::Start(index_offset))
                .map_err(|error| format!("StorageCorrupt: resume document index: {error}"))?;
            let offset = read_u64(&mut self.document_index)
                .map_err(|error| format!("StorageCorrupt: resume document index: {error:?}"))?;
            let length = read_u64(&mut self.document_index)
                .map_err(|error| format!("StorageCorrupt: resume document index: {error:?}"))?;
            let line_length = usize::try_from(length)
                .map_err(|_| "StorageCorrupt: resume document record length".to_owned())?;
            if line_length == 0 || line_length > 1_048_576 {
                return Err("StorageCorrupt: resume document record length".into());
            }
            self.document
                .seek(SeekFrom::Start(offset))
                .map_err(|error| format!("StorageCorrupt: resume document seek: {error}"))?;
            let mut line = vec![0; line_length];
            self.document
                .read_exact(&mut line)
                .map_err(|error| format!("StorageCorrupt: resume document record: {error}"))?;
            let entry: ParakeetDocumentEntry = serde_json::from_slice(&line)
                .map_err(|error| format!("StorageCorrupt: resume document entry: {error}"))?;
            let segment = entry.segment;
            let midpoint = segment.range.start_sample
                + (segment.range.end_sample - segment.range.start_sample) / 2;
            if segment.job_id != self.job_id
                || segment.generation != self.generation
                || segment.segment_id.0 != self.next_document_sequence
                || entry.revision != self.next_document_sequence
                || segment.range.sample_rate_hz != 16_000
                || segment.range.start_sample >= segment.range.end_sample
                || segment.range.end_sample > self.source_samples
                || segment.text.trim().is_empty()
                || coverage.range.start_sample > midpoint
                || midpoint >= coverage.range.end_sample
            {
                return Err(
                    "StorageCorrupt: resume coverage does not match its document prefix".into(),
                );
            }
            self.next_document_sequence = self
                .next_document_sequence
                .checked_add(1)
                .ok_or_else(|| "StorageCorrupt: resume segment sequence exhausted".to_owned())?;
        }
        Ok(())
    }
}

fn read_bounded_line(
    reader: &mut BufReader<File>,
    maximum: usize,
) -> Result<Option<Vec<u8>>, String> {
    let mut line = Vec::new();
    loop {
        let available = reader
            .fill_buf()
            .map_err(|error| format!("StorageCorrupt: resume coverage read: {error}"))?;
        if available.is_empty() {
            if line.is_empty() {
                return Ok(None);
            }
            return Err("StorageCorrupt: truncated resume coverage record".into());
        }
        let take = available
            .iter()
            .position(|byte| *byte == b'\n')
            .map_or(available.len(), |index| index + 1);
        if line.len().saturating_add(take) > maximum {
            return Err("StorageCorrupt: resume coverage record exceeds its bound".into());
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

impl ParakeetDocumentPort for ParakeetDocumentStore {
    fn append_final(&mut self, segment: WorkerSegment) -> Result<u64, PortError> {
        self.check_identity(segment.job_id, segment.generation)?;
        if self.finalized {
            return Err(PortError::Committing);
        }
        if segment.segment_id.0 != self.next_sequence
            || segment.range.sample_rate_hz != 16_000
            || segment.range.start_sample >= segment.range.end_sample
            || segment.text.trim().is_empty()
        {
            return Err(PortError::InvalidInput);
        }
        let revision = self.revision.checked_add(1).ok_or(PortError::Unavailable)?;
        let next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or(PortError::Unavailable)?;
        let entry = ParakeetDocumentEntry { segment, revision };
        let mut document = OpenOptions::new()
            .append(true)
            .open(self.directory.join("document.jsonl"))
            .map_err(storage_error)?;
        let offset = document.metadata().map_err(storage_error)?.len();
        serde_json::to_writer(&mut document, &entry)
            .map_err(|error| PortError::Failed(error.to_string()))?;
        document.write_all(b"\n").map_err(storage_error)?;
        document.sync_all().map_err(storage_error)?;
        let length = document
            .metadata()
            .map_err(storage_error)?
            .len()
            .checked_sub(offset)
            .ok_or(PortError::Unavailable)?;
        drop(document);
        let mut index = OpenOptions::new()
            .append(true)
            .open(self.directory.join("document.idx"))
            .map_err(storage_error)?;
        index
            .write_all(&offset.to_le_bytes())
            .map_err(storage_error)?;
        index
            .write_all(&length.to_le_bytes())
            .map_err(storage_error)?;
        index.sync_all().map_err(storage_error)?;
        self.revision = revision;
        self.next_sequence = next_sequence;
        Ok(revision)
    }

    fn record_coverage(&mut self, coverage: ParakeetWindowCoverage) -> Result<(), PortError> {
        if self.finalized
            || coverage.sequence != self.next_window
            || coverage.range.sample_rate_hz != 16_000
            || coverage.range.start_sample != self.covered_samples
            || coverage.range.end_sample <= coverage.range.start_sample
            || coverage.last_segment_sequence.checked_add(1) != Some(self.next_sequence)
        {
            return Err(PortError::InvalidInput);
        }
        let mut sequence = self.covered_segment_sequence.saturating_add(1);
        while sequence <= coverage.last_segment_sequence {
            let page = self.read_page(
                self.job_id,
                self.generation,
                self.revision,
                sequence,
                MAX_PAGE_SIZE,
            )?;
            if page.entries.is_empty() {
                return Err(PortError::Failed(
                    "StorageCorrupt: coverage references a missing document entry".into(),
                ));
            }
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
                {
                    return Err(PortError::Failed(
                        "StorageCorrupt: final word is outside its central audio window".into(),
                    ));
                }
                sequence = entry.segment.segment_id.0.saturating_add(1);
            }
        }
        let mut journal = OpenOptions::new()
            .append(true)
            .open(self.directory.join("coverage.jsonl"))
            .map_err(storage_error)?;
        serde_json::to_writer(&mut journal, &coverage)
            .map_err(|error| PortError::Failed(error.to_string()))?;
        journal.write_all(b"\n").map_err(storage_error)?;
        journal.sync_all().map_err(storage_error)?;
        self.covered_samples = coverage.range.end_sample;
        self.covered_segment_sequence = coverage.last_segment_sequence;
        self.next_window = self
            .next_window
            .checked_add(1)
            .ok_or(PortError::Unavailable)?;
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
        if expected_revision != self.revision {
            return Err(PortError::StaleResponse);
        }
        if first_sequence == 0 || limit == 0 {
            return Err(PortError::InvalidInput);
        }
        let limit = limit.min(MAX_PAGE_SIZE);
        let count = self.revision;
        let mut index = File::open(self.directory.join("document.idx")).map_err(storage_error)?;
        let mut header = [0_u8; 8];
        std::io::Read::read_exact(&mut index, &mut header).map_err(storage_error)?;
        let index_size = index.metadata().map_err(storage_error)?.len();
        if &header != INDEX_HEADER || index_size < 8 || (index_size - 8) % INDEX_ENTRY_BYTES != 0 {
            return Err(PortError::Failed("StorageCorrupt: document index".into()));
        }
        let mut document = BufReader::new(
            File::open(self.directory.join("document.jsonl")).map_err(storage_error)?,
        );
        let mut entries = Vec::with_capacity(limit);
        let mut sequence = first_sequence;
        while sequence <= count && entries.len() < limit {
            let index_offset = 8_u64
                .checked_add(
                    (sequence - 1)
                        .checked_mul(INDEX_ENTRY_BYTES)
                        .ok_or(PortError::Unavailable)?,
                )
                .ok_or(PortError::Unavailable)?;
            index
                .seek(SeekFrom::Start(index_offset))
                .map_err(storage_error)?;
            let offset = read_u64(&mut index)?;
            let length = read_u64(&mut index)?;
            let line_length = usize::try_from(length).map_err(|_| PortError::Unavailable)?;
            if line_length == 0 || line_length > 1_048_576 {
                return Err(PortError::Failed(
                    "StorageCorrupt: document record size".into(),
                ));
            }
            document
                .seek(SeekFrom::Start(offset))
                .map_err(storage_error)?;
            let mut line = String::with_capacity(line_length);
            document
                .by_ref()
                .take(length)
                .read_line(&mut line)
                .map_err(storage_error)?;
            if line.len() as u64 != length {
                return Err(PortError::Failed(
                    "StorageCorrupt: short document record".into(),
                ));
            }
            let entry: ParakeetDocumentEntry =
                serde_json::from_str(line.trim_end()).map_err(|error| {
                    PortError::Failed(format!("StorageCorrupt: document entry: {error}"))
                })?;
            if entry.segment.job_id != job_id
                || entry.segment.generation != generation
                || entry.segment.segment_id.0 != sequence
                || entry.revision != sequence
            {
                return Err(PortError::StaleResponse);
            }
            entries.push(entry);
            sequence += 1;
        }
        let next_sequence = (sequence <= count).then_some(sequence);
        Ok(ParakeetDocumentPage {
            revision: self.revision,
            entries,
            next_sequence,
        })
    }

    fn finalize(
        &mut self,
        job_id: JobId,
        generation: Generation,
        total_samples: u64,
    ) -> Result<u64, PortError> {
        self.check_identity(job_id, generation)?;
        if self.finalized {
            return Ok(self.revision);
        }
        if total_samples == 0 || self.covered_samples != total_samples {
            return Err(PortError::Failed(
                "IncompleteAudio: final document lacks contiguous source coverage".into(),
            ));
        }
        let marker = serde_json::json!({ "version": 1, "job_id": format!("{:032x}", job_id.0), "generation": generation.get(), "revision": self.revision });
        let path = self.directory.join("document.final.json");
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(storage_error)?;
        serde_json::to_writer(&mut file, &marker)
            .map_err(|error| PortError::Failed(error.to_string()))?;
        file.sync_all().map_err(storage_error)?;
        self.finalized = true;
        Ok(self.revision)
    }

    fn replace_preview(
        &mut self,
        job_id: JobId,
        generation: Generation,
        segment: Option<WorkerSegment>,
    ) -> Result<u64, PortError> {
        self.check_identity(job_id, generation)?;
        if self.finalized {
            return Err(PortError::Committing);
        }
        let next_revision = self
            .preview_revision
            .checked_add(1)
            .ok_or(PortError::Unavailable)?;
        let path = self.directory.join("document.preview.json");
        match segment {
            Some(segment) => {
                if segment.job_id != job_id
                    || segment.generation != generation
                    || segment.range.start_sample >= segment.range.end_sample
                    || segment.range.sample_rate_hz != 16_000
                    || segment.text.trim().is_empty()
                {
                    return Err(PortError::InvalidInput);
                }
                let preview = ParakeetPreview {
                    segment,
                    revision: next_revision,
                };
                let mut file = OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(path)
                    .map_err(storage_error)?;
                serde_json::to_writer(&mut file, &preview)
                    .map_err(|error| PortError::Failed(error.to_string()))?;
                file.sync_all().map_err(storage_error)?;
            }
            None => match fs::remove_file(path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(storage_error(error)),
            },
        }
        self.preview_revision = next_revision;
        Ok(next_revision)
    }

    fn read_preview(
        &mut self,
        job_id: JobId,
        generation: Generation,
    ) -> Result<Option<ParakeetPreview>, PortError> {
        self.check_identity(job_id, generation)?;
        let path = self.directory.join("document.preview.json");
        let mut file = match File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(storage_error(error)),
        };
        let mut json = String::new();
        file.read_to_string(&mut json).map_err(storage_error)?;
        let preview: ParakeetPreview = serde_json::from_str(&json)
            .map_err(|error| PortError::Failed(format!("StorageCorrupt: preview: {error}")))?;
        if preview.segment.job_id != job_id
            || preview.segment.generation != generation
            || preview.revision != self.preview_revision
        {
            return Err(PortError::StaleResponse);
        }
        Ok(Some(preview))
    }
}

fn read_u64(reader: &mut impl std::io::Read) -> Result<u64, PortError> {
    let mut bytes = [0_u8; 8];
    reader.read_exact(&mut bytes).map_err(storage_error)?;
    Ok(u64::from_le_bytes(bytes))
}

fn storage_error(error: std::io::Error) -> PortError {
    PortError::Failed(format!("StorageUnavailable: {error}"))
}

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
            range: whisper_core::SourceRange::new(sequence * 100, sequence * 100 + 50, 16_000)
                .unwrap(),
            text: format!("word-{sequence}"),
        }
    }

    #[test]
    fn document_reads_bounded_pages_at_a_stable_revision_and_finalizes_once() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("whisper-parakeet-document-{stamp}"));
        let id = JobId(23);
        let generation = Generation::first();
        let mut document = ParakeetDocumentStore::create(&directory, id, generation).unwrap();
        for sequence in 1..=3 {
            document
                .append_final(segment(id, generation, sequence))
                .unwrap();
        }
        let first = document.read_page(id, generation, 3, 1, 2).unwrap();
        assert_eq!(first.entries.len(), 2);
        assert_eq!(first.next_sequence, Some(3));
        let second = document.read_page(id, generation, 3, 3, 2).unwrap();
        assert_eq!(second.entries[0].segment.text, "word-3");
        assert_eq!(
            document.read_page(id, generation, 2, 1, 1),
            Err(PortError::StaleResponse)
        );
        document
            .record_coverage(ParakeetWindowCoverage {
                sequence: 1,
                range: whisper_core::SourceRange::new(0, 350, 16_000).unwrap(),
                last_segment_sequence: 3,
            })
            .unwrap();
        assert_eq!(document.finalize(id, generation, 350).unwrap(), 3);
        assert_eq!(document.finalize(id, generation, 350).unwrap(), 3);
        assert_eq!(
            document.append_final(segment(id, generation, 4)),
            Err(PortError::Committing)
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn document_preview_is_a_single_revisable_record_separate_from_final_pages() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("whisper-parakeet-preview-{stamp}"));
        let id = JobId(24);
        let generation = Generation::first();
        let mut document = ParakeetDocumentStore::create(&directory, id, generation).unwrap();
        let first = segment(id, generation, 1);
        let mut second = segment(id, generation, 2);
        second.text = "replacement".into();
        assert_eq!(
            document
                .replace_preview(id, generation, Some(first))
                .unwrap(),
            1
        );
        assert_eq!(
            document
                .replace_preview(id, generation, Some(second))
                .unwrap(),
            2
        );
        let preview = document.read_preview(id, generation).unwrap().unwrap();
        assert_eq!(preview.revision, 2);
        assert_eq!(preview.segment.text, "replacement");
        assert_eq!(
            document.revision(),
            0,
            "preview must not enter the final page index"
        );
        assert_eq!(document.replace_preview(id, generation, None), Ok(3));
        assert_eq!(document.read_preview(id, generation), Ok(None));
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn resume_coverage_selects_the_last_checkpoint_within_the_archived_prefix() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("whisper-parakeet-resume-doc-{stamp}"));
        let id = JobId(25);
        let generation = Generation::first();
        let window = WINDOW_MAX_SAMPLES;
        let source_samples = 2 * window;
        let mut document = ParakeetDocumentStore::create(&directory, id, generation).unwrap();
        for (sequence, start, end) in [
            (1, 10, 50),
            (2, window - 6_400, window - 3_200),
            (3, window + 100, window + 200),
        ] {
            let mut item = segment(id, generation, sequence);
            item.range = whisper_core::SourceRange::new(start, end, 16_000).unwrap();
            document.append_final(item).unwrap();
            if sequence == 2 {
                document
                    .record_coverage(ParakeetWindowCoverage {
                        sequence: 1,
                        range: whisper_core::SourceRange::new(0, window, 16_000).unwrap(),
                        last_segment_sequence: 2,
                    })
                    .unwrap();
            }
        }
        document
            .record_coverage(ParakeetWindowCoverage {
                sequence: 2,
                range: whisper_core::SourceRange::new(window, source_samples, 16_000).unwrap(),
                last_segment_sequence: 3,
            })
            .unwrap();
        drop(document);

        let checkpoint =
            ParakeetDocumentStore::latest_resume_checkpoint(
                &directory,
                id,
                generation,
                source_samples,
                2,
            )
            .unwrap()
            .unwrap();
        assert_eq!(
            checkpoint,
            ResumeCoverageCheckpoint {
                window_sequence: 1,
                covered_samples: window,
                confirmed_segments: 2,
            }
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn resume_coverage_rejects_impossible_window_geometry_and_accepts_short_terminal_eof() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let id = JobId(251);
        let generation = Generation::first();
        let source_samples = WINDOW_MAX_SAMPLES + 16_000;

        let invalid_directory = std::env::temp_dir()
            .join(format!("whisper-parakeet-short-resume-invalid-{stamp}"));
        let mut invalid = ParakeetDocumentStore::create(&invalid_directory, id, generation).unwrap();
        let mut short_segment = segment(id, generation, 1);
        short_segment.range = whisper_core::SourceRange::new(0, 8_000, 16_000).unwrap();
        invalid.append_final(short_segment).unwrap();
        invalid
            .record_coverage(ParakeetWindowCoverage {
                sequence: 1,
                range: whisper_core::SourceRange::new(0, 16_000, 16_000).unwrap(),
                last_segment_sequence: 1,
            })
            .unwrap();
        drop(invalid);
        let invalid = ParakeetDocumentStore::latest_resume_checkpoint(
            &invalid_directory,
            id,
            generation,
            source_samples,
            1,
        );
        assert!(
            invalid
                .as_ref()
                .is_err_and(|error| error.contains("resume coverage is not a valid contiguous checkpoint")),
            "a short checkpoint before source EOF must be rejected: {invalid:?}"
        );

        let oversized_directory = std::env::temp_dir()
            .join(format!("whisper-parakeet-oversized-resume-window-{stamp}"));
        let mut oversized =
            ParakeetDocumentStore::create(&oversized_directory, id, generation).unwrap();
        let mut oversized_segment = segment(id, generation, 1);
        oversized_segment.range =
            whisper_core::SourceRange::new(0, source_samples, 16_000).unwrap();
        oversized.append_final(oversized_segment).unwrap();
        oversized
            .record_coverage(ParakeetWindowCoverage {
                sequence: 1,
                range: whisper_core::SourceRange::new(0, source_samples, 16_000).unwrap(),
                last_segment_sequence: 1,
            })
            .unwrap();
        drop(oversized);
        let oversized = ParakeetDocumentStore::latest_resume_checkpoint(
            &oversized_directory,
            id,
            generation,
            source_samples,
            1,
        );
        assert!(
            oversized
                .as_ref()
                .is_err_and(|error| error.contains("resume coverage is not a valid contiguous checkpoint")),
            "a checkpoint wider than the scheduler's maximum window must be rejected: {oversized:?}"
        );

        let valid_directory = std::env::temp_dir()
            .join(format!("whisper-parakeet-short-resume-terminal-{stamp}"));
        let mut valid = ParakeetDocumentStore::create(&valid_directory, id, generation).unwrap();
        let mut full_window_segment = segment(id, generation, 1);
        full_window_segment.range =
            whisper_core::SourceRange::new(0, WINDOW_MAX_SAMPLES, 16_000).unwrap();
        valid.append_final(full_window_segment).unwrap();
        valid
            .record_coverage(ParakeetWindowCoverage {
                sequence: 1,
                range: whisper_core::SourceRange::new(0, WINDOW_MAX_SAMPLES, 16_000).unwrap(),
                last_segment_sequence: 1,
            })
            .unwrap();
        let mut terminal_segment = segment(id, generation, 2);
        terminal_segment.range = whisper_core::SourceRange::new(
            WINDOW_MAX_SAMPLES,
            source_samples,
            16_000,
        )
        .unwrap();
        valid.append_final(terminal_segment).unwrap();
        valid
            .record_coverage(ParakeetWindowCoverage {
                sequence: 2,
                range: whisper_core::SourceRange::new(
                    WINDOW_MAX_SAMPLES,
                    source_samples,
                    16_000,
                )
                .unwrap(),
                last_segment_sequence: 2,
            })
            .unwrap();
        drop(valid);
        let checkpoint = ParakeetDocumentStore::latest_resume_checkpoint(
            &valid_directory,
            id,
            generation,
            source_samples,
            2,
        )
        .unwrap()
        .unwrap();
        assert_eq!(checkpoint.window_sequence, 2);
        assert_eq!(checkpoint.covered_samples, source_samples);
        assert_eq!(checkpoint.confirmed_segments, 2);

        fs::remove_dir_all(invalid_directory).unwrap();
        fs::remove_dir_all(oversized_directory).unwrap();
        fs::remove_dir_all(valid_directory).unwrap();
    }

    #[test]
    fn resume_coverage_pages_are_capped_at_128_and_keep_the_global_cursor() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("whisper-parakeet-resume-pages-{stamp}"));
        let id = JobId(26);
        let generation = Generation::first();
        let source_samples = 130 * WINDOW_MAX_SAMPLES;
        let mut document = ParakeetDocumentStore::create(&directory, id, generation).unwrap();
        for sequence in 1..=130_u64 {
            let start = (sequence - 1) * WINDOW_MAX_SAMPLES;
            let end = sequence * WINDOW_MAX_SAMPLES;
            document
                .append_final(WorkerSegment {
                    job_id: id,
                    generation,
                    instance_id: 1,
                    segment_id: whisper_core::SegmentId(sequence),
                    range: whisper_core::SourceRange::new(start, end, 16_000).unwrap(),
                    text: format!("segment-{sequence}"),
                })
                .unwrap();
            document
                .record_coverage(ParakeetWindowCoverage {
                    sequence,
                    range: whisper_core::SourceRange::new(start, end, 16_000).unwrap(),
                    last_segment_sequence: sequence,
                })
                .unwrap();
        }
        drop(document);

        let mut reader =
            ParakeetDocumentStore::open_resume_coverage(
                &directory,
                id,
                generation,
                source_samples,
            )
                .unwrap()
                .unwrap();
        let first = reader.read_page(512).unwrap();
        assert_eq!(first.len(), 128);
        let second = reader.read_page(512).unwrap();
        assert_eq!(second.len(), 2);
        assert_eq!(second.last().unwrap().sequence, 130);
        assert_eq!(
            ParakeetDocumentStore::latest_resume_checkpoint(
                &directory,
                id,
                generation,
                source_samples,
                129,
            )
            .unwrap(),
            Some(ResumeCoverageCheckpoint {
                window_sequence: 129,
                covered_samples: 129 * WINDOW_MAX_SAMPLES,
                confirmed_segments: 129,
            })
        );
        drop(reader);
        fs::remove_dir_all(directory).unwrap();
    }
}
