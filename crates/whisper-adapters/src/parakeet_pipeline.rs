//! Bounded-window Parakeet finalization over a disk-backed audio source.

use whisper_core::{
    Generation, JobId, SourceRange,
    parakeet_session::{AudioWindow, SessionIdentity, TranscriptionStrategy, WindowCursor},
    parakeet_windows::{MARGIN_SAMPLES, WINDOW_MAX_SAMPLES},
    ports::{ParakeetDocumentPort, ParakeetWindowCoverage, PortError, WorkerSegment},
};

use crate::parakeet_audio::{AudioStoreError, CapturePcmWriter, PcmPartStore};

const PREVIEW_SAMPLES: u64 = 4 * 16_000;

pub trait WindowTranscriber: Send {
    fn instance_id(&self) -> u64;
    fn transcribe(
        &mut self,
        window: AudioWindow,
        pcm: &[i16],
    ) -> Result<Vec<whisper_core::parakeet_ipc::Word>, String>;
}

pub trait ReadablePcmSource {
    fn read_window(&mut self, range: std::ops::Range<u64>) -> Result<Vec<i16>, String>;
}

impl ReadablePcmSource for PcmPartStore {
    fn read_window(&mut self, range: std::ops::Range<u64>) -> Result<Vec<i16>, String> {
        PcmPartStore::read_window(self, range).map_err(store_error)
    }
}

impl ReadablePcmSource for CapturePcmWriter {
    fn read_window(&mut self, range: std::ops::Range<u64>) -> Result<Vec<i16>, String> {
        CapturePcmWriter::read_written_window(self, range)
            .map_err(|error| format!("AudioSource: {error:?}"))
    }
}

impl ReadablePcmSource for &PcmPartStore {
    fn read_window(&mut self, range: std::ops::Range<u64>) -> Result<Vec<i16>, String> {
        PcmPartStore::read_window(self, range).map_err(store_error)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PipelineResult {
    pub identity: SessionIdentity,
    pub total_samples: u64,
    pub windows_completed: u64,
    pub document_revision: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FinalizationStep {
    WaitingForAudio {
        processed_samples: u64,
    },
    WindowProcessed {
        processed_samples: u64,
        windows_completed: u64,
        document_revision: u64,
    },
    Complete(PipelineResult),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PreparedFinalWindow {
    WaitingForAudio { processed_samples: u64 },
    Ready { window: AudioWindow, pcm: Vec<i16> },
    Complete,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedPreviewWindow {
    pub window: AudioWindow,
    pub pcm: Vec<i16>,
}

#[derive(Clone, Copy, Debug)]
struct PendingFinalWindow {
    window: AudioWindow,
    source_end: u64,
}

pub struct ParakeetFinalizer {
    identity: SessionIdentity,
    cursor: WindowCursor,
    windows_completed: u64,
    next_segment: u64,
    document_revision: u64,
    completed: Option<PipelineResult>,
    pending_window: Option<PendingFinalWindow>,
}

impl ParakeetFinalizer {
    pub fn new(job_id: JobId, generation: Generation) -> Self {
        Self {
            identity: SessionIdentity {
                job_id,
                generation,
                strategy: TranscriptionStrategy::BoundedWindowsV1,
            },
            cursor: WindowCursor::default(),
            windows_completed: 0,
            next_segment: 1,
            document_revision: 0,
            completed: None,
            pending_window: None,
        }
    }

    pub fn processed_samples(&self) -> u64 {
        self.cursor.next_start()
    }

    pub fn next_window_sequence(&self) -> u64 {
        self.windows_completed.saturating_add(1)
    }

    /// Reads and reserves one final clip without invoking the backend. The
    /// scheduler may hold this prepared turn while another runtime duty runs.
    pub fn prepare_next<S: ReadablePcmSource + ?Sized>(
        &mut self,
        source: &mut S,
        available_samples: u64,
        source_closed: bool,
    ) -> Result<PreparedFinalWindow, String> {
        if self.completed.is_some() {
            return Ok(PreparedFinalWindow::Complete);
        }
        if self.pending_window.is_some() {
            return Err("Pipeline: final window already prepared".into());
        }
        if available_samples == 0 {
            return if source_closed {
                Err("InvalidInput: empty Parakeet source".into())
            } else {
                Ok(PreparedFinalWindow::WaitingForAudio {
                    processed_samples: 0,
                })
            };
        }
        if available_samples < self.cursor.next_start() {
            return Err("StorageCorrupt: source boundary regressed".into());
        }
        if !source_closed
            && available_samples.saturating_sub(self.cursor.next_start())
                < WINDOW_MAX_SAMPLES + MARGIN_SAMPLES
        {
            return Ok(PreparedFinalWindow::WaitingForAudio {
                processed_samples: self.cursor.next_start(),
            });
        }
        let central_start = self.cursor.next_start();
        let buffer_start = central_start.saturating_sub(MARGIN_SAMPLES);
        let buffer_end = central_start
            .saturating_add(WINDOW_MAX_SAMPLES)
            .saturating_add(MARGIN_SAMPLES)
            .min(available_samples);
        let samples = source.read_window(buffer_start..buffer_end)?;
        let window = self
            .cursor
            .next_window(buffer_start, &samples, available_samples, source_closed)
            .map_err(|error| format!("WindowPlanning: {error:?}"))?;
        let Some(window) = window else {
            if source_closed && self.cursor.next_start() == available_samples {
                return Ok(PreparedFinalWindow::Complete);
            }
            if source_closed {
                return Err(
                    "WindowPlanning: closed source did not produce its next window".to_owned(),
                );
            }
            return Ok(PreparedFinalWindow::WaitingForAudio {
                processed_samples: self.cursor.next_start(),
            });
        };
        let local_start = usize::try_from(window.native_start_sample - buffer_start)
            .map_err(|_| "InvalidInput: window start exceeds address space".to_owned())?;
        let local_end = usize::try_from(window.native_end_sample - buffer_start)
            .map_err(|_| "InvalidInput: window end exceeds address space".to_owned())?;
        let clip = samples
            .get(local_start..local_end)
            .ok_or_else(|| "StorageCorrupt: planned clip lies outside the source page".to_owned())?
            .to_vec();
        self.pending_window = Some(PendingFinalWindow {
            window,
            source_end: available_samples,
        });
        Ok(PreparedFinalWindow::Ready { window, pcm: clip })
    }

    pub fn commit_prepared<D: ParakeetDocumentPort>(
        &mut self,
        document: &mut D,
        instance_id: u64,
        words: &[whisper_core::parakeet_ipc::Word],
    ) -> Result<FinalizationStep, String> {
        let pending = self
            .pending_window
            .take()
            .ok_or_else(|| "Pipeline: no prepared final window".to_owned())?;
        self.commit_window(document, pending, instance_id, words)
    }

    pub fn finish_document<D: ParakeetDocumentPort>(
        &mut self,
        document: &mut D,
        total_samples: u64,
        source_closed: bool,
    ) -> Result<FinalizationStep, String> {
        if let Some(result) = &self.completed {
            return Ok(FinalizationStep::Complete(result.clone()));
        }
        if !source_closed
            || self.pending_window.is_some()
            || self.cursor.next_start() != total_samples
        {
            return Err("Pipeline: document is not ready to finalize".into());
        }
        if total_samples == 0 {
            return Err("InvalidInput: empty Parakeet source".into());
        }
        self.document_revision = document
            .finalize(
                self.identity.job_id,
                self.identity.generation,
                total_samples,
            )
            .map_err(port_error)?;
        let result = PipelineResult {
            identity: self.identity,
            total_samples,
            windows_completed: self.windows_completed,
            document_revision: self.document_revision,
        };
        self.completed = Some(result.clone());
        Ok(FinalizationStep::Complete(result))
    }

    fn commit_window<D: ParakeetDocumentPort>(
        &mut self,
        document: &mut D,
        pending: PendingFinalWindow,
        instance_id: u64,
        words: &[whisper_core::parakeet_ipc::Word],
    ) -> Result<FinalizationStep, String> {
        let window = pending.window;
        let segments = validated_segments(
            words,
            window,
            pending.source_end,
            instance_id,
            self.identity.job_id,
            self.identity.generation,
            self.next_segment,
        )?;
        for segment in segments {
            self.document_revision = document.append_final(segment).map_err(port_error)?;
            self.next_segment = self
                .next_segment
                .checked_add(1)
                .ok_or_else(|| "InvalidInput: segment sequence exhausted".to_owned())?;
        }
        let range = SourceRange::new(
            window.central_start_sample,
            window.central_end_sample,
            16_000,
        )
        .ok_or_else(|| "InvalidInput: invalid central coverage range".to_owned())?;
        document
            .record_coverage(ParakeetWindowCoverage {
                sequence: window.sequence,
                range,
                last_segment_sequence: self.next_segment - 1,
            })
            .map_err(port_error)?;
        self.windows_completed = self
            .windows_completed
            .checked_add(1)
            .ok_or_else(|| "InvalidInput: window sequence exhausted".to_owned())?;
        Ok(FinalizationStep::WindowProcessed {
            processed_samples: self.cursor.next_start(),
            windows_completed: self.windows_completed,
            document_revision: self.document_revision,
        })
    }

    /// Processes at most one final window. The caller may invoke this while a
    /// capture writer is still advancing the source; `source_closed` controls
    /// whether a short tail may be emitted and whether the document can finish.
    pub fn step<
        S: ReadablePcmSource + ?Sized,
        T: WindowTranscriber + ?Sized,
        D: ParakeetDocumentPort,
    >(
        &mut self,
        source: &mut S,
        document: &mut D,
        transcriber: &mut T,
        available_samples: u64,
        source_closed: bool,
    ) -> Result<FinalizationStep, String> {
        match self.prepare_next(source, available_samples, source_closed)? {
            PreparedFinalWindow::WaitingForAudio { processed_samples } => {
                Ok(FinalizationStep::WaitingForAudio { processed_samples })
            }
            PreparedFinalWindow::Ready { window, pcm } => {
                let words = transcriber.transcribe(window, &pcm)?;
                self.commit_prepared(document, transcriber.instance_id(), &words)
            }
            PreparedFinalWindow::Complete => {
                self.finish_document(document, available_samples, source_closed)
            }
        }
    }
}

pub fn finalize_source<T: WindowTranscriber + ?Sized, D: ParakeetDocumentPort>(
    source: &PcmPartStore,
    document: &mut D,
    transcriber: &mut T,
    job_id: JobId,
    generation: Generation,
) -> Result<PipelineResult, String> {
    let total_samples = source.total_samples().map_err(store_error)?;
    let mut source = source;
    let mut finalizer = ParakeetFinalizer::new(job_id, generation);
    loop {
        if let FinalizationStep::Complete(result) =
            finalizer.step(&mut source, document, transcriber, total_samples, true)?
        {
            return Ok(result);
        }
    }
}

/// Transcribes the latest four seconds as one replaceable document preview.
/// The caller schedules this through the same backend arbiter as final windows.
pub fn transcribe_preview<S, T, D>(
    source: &mut S,
    document: &mut D,
    transcriber: &mut T,
    job_id: JobId,
    generation: Generation,
    available_samples: u64,
    sequence: u64,
) -> Result<u64, String>
where
    S: ReadablePcmSource + ?Sized,
    T: WindowTranscriber + ?Sized,
    D: ParakeetDocumentPort,
{
    let PreparedPreviewWindow { window, pcm } =
        prepare_preview(source, available_samples, sequence)?;
    let words = transcriber.transcribe(window, &pcm)?;
    commit_preview(
        document,
        window,
        available_samples,
        transcriber.instance_id(),
        job_id,
        generation,
        &words,
    )
}

pub fn commit_preview<D: ParakeetDocumentPort>(
    document: &mut D,
    window: AudioWindow,
    available_samples: u64,
    instance_id: u64,
    job_id: JobId,
    generation: Generation,
    words: &[whisper_core::parakeet_ipc::Word],
) -> Result<u64, String> {
    let segments = validated_segments(
        &words,
        window,
        available_samples,
        instance_id,
        job_id,
        generation,
        1,
    )?;
    document
        .replace_preview(job_id, generation, segments.into_iter().next())
        .map_err(port_error)
}

pub fn prepare_preview<S: ReadablePcmSource + ?Sized>(
    source: &mut S,
    available_samples: u64,
    sequence: u64,
) -> Result<PreparedPreviewWindow, String> {
    if available_samples < PREVIEW_SAMPLES || sequence == 0 {
        return Err("Preview: four seconds of acknowledged audio are required".into());
    }
    let start = available_samples - PREVIEW_SAMPLES;
    let pcm = source.read_window(start..available_samples)?;
    Ok(PreparedPreviewWindow {
        window: AudioWindow {
            sequence,
            central_start_sample: start,
            central_end_sample: available_samples,
            native_start_sample: start,
            native_end_sample: available_samples,
        },
        pcm,
    })
}

fn validated_segments(
    words: &[whisper_core::parakeet_ipc::Word],
    window: AudioWindow,
    total_samples: u64,
    instance_id: u64,
    job_id: JobId,
    generation: Generation,
    first_sequence: u64,
) -> Result<Vec<WorkerSegment>, String> {
    let native_samples = window
        .native_end_sample
        .checked_sub(window.native_start_sample)
        .ok_or_else(|| "InvalidTimestamp: native window is reversed".to_owned())?;
    let mut groups = Vec::<(u64, u64, Vec<String>)>::new();
    let mut previous_end = 0_u64;
    for (index, word) in words.iter().enumerate() {
        let local_start = u64::from(word.start_ms)
            .checked_mul(16)
            .ok_or_else(|| format!("InvalidTimestamp: word {index} start overflow"))?;
        let mut local_end = u64::from(word.end_ms)
            .checked_mul(16)
            .ok_or_else(|| format!("InvalidTimestamp: word {index} end overflow"))?;
        let token = word.text.trim();
        if token.is_empty()
            || local_start >= local_end
            || local_start.saturating_add(80 * 16) < previous_end
        {
            return Err(format!(
                "InvalidTimestamp: word {index} is invalid or unordered"
            ));
        }
        if local_end > native_samples {
            let overflow = local_end - native_samples;
            let final_file_word =
                window.native_end_sample == total_samples && index + 1 == words.len();
            if !final_file_word || overflow > 80 * 16 {
                return Err(format!(
                    "InvalidTimestamp: word {index} exceeds its native window"
                ));
            }
            local_end = native_samples;
        }
        if local_start >= local_end {
            return Err(format!(
                "InvalidTimestamp: word {index} is empty after final-window capping"
            ));
        }
        previous_end = previous_end.max(local_end);
        let start = window
            .native_start_sample
            .checked_add(local_start)
            .ok_or_else(|| format!("InvalidTimestamp: word {index} global start overflow"))?;
        let end = window
            .native_start_sample
            .checked_add(local_end)
            .ok_or_else(|| format!("InvalidTimestamp: word {index} global end overflow"))?;
        let midpoint = start + (end - start) / 2;
        if midpoint < window.central_start_sample || midpoint >= window.central_end_sample {
            continue;
        }
        let punctuation_boundary = groups.last().is_some_and(|(_, _, tokens)| {
            tokens.last().is_some_and(|last| {
                last.ends_with('.') || last.ends_with('!') || last.ends_with('?')
            })
        });
        if groups.is_empty() || punctuation_boundary {
            groups.push((start, end, vec![token.to_owned()]));
        } else if let Some((group_start, group_end, tokens)) = groups.last_mut() {
            *group_start = (*group_start).min(start);
            *group_end = (*group_end).max(end);
            tokens.push(token.to_owned());
        }
    }
    groups
        .into_iter()
        .enumerate()
        .map(|(index, (start, end, words))| {
            let sequence = first_sequence
                .checked_add(index as u64)
                .ok_or_else(|| "InvalidInput: segment sequence exhausted".to_owned())?;
            let range = SourceRange::new(start, end, 16_000)
                .ok_or_else(|| "InvalidTimestamp: grouped word range is invalid".to_owned())?;
            Ok(WorkerSegment {
                job_id,
                generation,
                instance_id,
                segment_id: whisper_core::SegmentId(sequence),
                range,
                text: words.join(" "),
            })
        })
        .collect()
}

fn store_error(error: AudioStoreError) -> String {
    format!("AudioSource: {error}")
}
fn port_error(error: PortError) -> String {
    format!("Document: {error:?}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        parakeet_audio::{CapturePcmWriter, PcmPartStore, PcmPartWriter},
        parakeet_document::ParakeetDocumentStore,
    };
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    struct ControlledWorker;
    impl WindowTranscriber for ControlledWorker {
        fn instance_id(&self) -> u64 {
            7
        }
        fn transcribe(
            &mut self,
            window: AudioWindow,
            _: &[i16],
        ) -> Result<Vec<whisper_core::parakeet_ipc::Word>, String> {
            let start_ms = ((window.central_start_sample - window.native_start_sample) / 16) as u32;
            Ok(vec![whisper_core::parakeet_ipc::Word {
                text: "word.".into(),
                start_ms,
                end_ms: start_ms + 500,
            }])
        }
    }

    fn word(start_ms: u32, end_ms: u32) -> whisper_core::parakeet_ipc::Word {
        whisper_core::parakeet_ipc::Word {
            text: "last".into(),
            start_ms,
            end_ms,
        }
    }

    #[test]
    fn only_the_last_word_of_the_last_window_gets_the_bounded_eof_tolerance() {
        let final_window = AudioWindow {
            sequence: 1,
            central_start_sample: 0,
            central_end_sample: 16_000,
            native_start_sample: 0,
            native_end_sample: 16_000,
        };
        let accepted = validated_segments(
            &[word(950, 1_010)],
            final_window,
            16_000,
            7,
            JobId(1),
            Generation::first(),
            1,
        )
        .unwrap();
        assert_eq!(accepted[0].range.end_sample, 16_000);
        assert!(
            validated_segments(
                &[word(950, 1_081)],
                final_window,
                16_000,
                7,
                JobId(1),
                Generation::first(),
                1
            )
            .is_err()
        );

        let internal_window = AudioWindow {
            native_end_sample: 16_000,
            ..final_window
        };
        assert!(
            validated_segments(
                &[word(950, 1_010)],
                internal_window,
                32_000,
                7,
                JobId(1),
                Generation::first(),
                1
            )
            .is_err()
        );
        assert!(
            validated_segments(
                &[word(950, 1_010), word(1_000, 1_020)],
                final_window,
                16_000,
                7,
                JobId(1),
                Generation::first(),
                1
            )
            .is_err()
        );
    }

    #[test]
    fn controlled_worker_processes_a_long_source_in_contiguous_windows() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("whisper-parakeet-pipeline-{stamp}"));
        fs::create_dir_all(&root).unwrap();
        let audio_path = root.join("source.wav");
        let mut writer = PcmPartWriter::create(&audio_path).unwrap();
        let total = 121 * 16_000_u64;
        writer.append(&vec![0_i16; total as usize]).unwrap();
        writer.finish().unwrap();
        let source = PcmPartStore::open(&audio_path);
        let mut document =
            ParakeetDocumentStore::create(root.join("document"), JobId(33), Generation::first())
                .unwrap();
        let result = finalize_source(
            &source,
            &mut document,
            &mut ControlledWorker,
            JobId(33),
            Generation::first(),
        )
        .unwrap();
        assert_eq!(result.total_samples, total);
        assert_eq!(result.windows_completed, 2);
        assert_eq!(document.revision(), 2);
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn finalization_advances_from_acknowledged_capture_then_closes_the_tail() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("whisper-parakeet-streaming-{stamp}"));
        fs::create_dir_all(&root).unwrap();
        let audio_path = root.join("source.wav");
        let mut capture = CapturePcmWriter::start(&audio_path, 2).unwrap();
        let first_len = (WINDOW_MAX_SAMPLES + MARGIN_SAMPLES) as usize;
        capture.try_append(&vec![100_i16; first_len]).unwrap();
        let first_ack = wait_for_ack(&capture);
        let source = PcmPartStore::open(&audio_path);
        let mut document =
            ParakeetDocumentStore::create(root.join("document"), JobId(34), Generation::first())
                .unwrap();
        let mut finalizer = ParakeetFinalizer::new(JobId(34), Generation::first());
        assert!(matches!(
            finalizer
                .step(
                    &mut capture,
                    &mut document,
                    &mut ControlledWorker,
                    first_ack.written_samples,
                    false,
                )
                .unwrap(),
            FinalizationStep::WindowProcessed {
                windows_completed: 1,
                ..
            }
        ));
        assert_eq!(finalizer.processed_samples(), WINDOW_MAX_SAMPLES);

        capture.try_append(&vec![200_i16; 32_000]).unwrap();
        let tail_ack = wait_for_ack(&capture);
        assert_eq!(
            tail_ack.range,
            first_ack.written_samples..tail_ack.written_samples
        );
        let total_samples = capture.close().unwrap();
        assert_eq!(total_samples, first_len as u64 + 32_000);
        assert!(matches!(
            finalizer
                .step(
                    &mut capture,
                    &mut document,
                    &mut ControlledWorker,
                    total_samples,
                    true,
                )
                .unwrap(),
            FinalizationStep::WindowProcessed {
                windows_completed: 2,
                ..
            }
        ));
        let FinalizationStep::Complete(result) = finalizer
            .step(
                &mut capture,
                &mut document,
                &mut ControlledWorker,
                total_samples,
                true,
            )
            .unwrap()
        else {
            panic!("closed source must finalize its remaining document coverage");
        };
        assert_eq!(result.total_samples, total_samples);
        assert_eq!(result.windows_completed, 2);
        assert_eq!(result.document_revision, 2);
        fs::remove_dir_all(root).unwrap();
    }

    fn wait_for_ack(writer: &CapturePcmWriter) -> crate::parakeet_audio::CaptureWriteAck {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            if let Some(ack) = writer.try_acknowledgement().unwrap() {
                return ack;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "capture write ack timed out"
            );
            std::thread::yield_now();
        }
    }
}
