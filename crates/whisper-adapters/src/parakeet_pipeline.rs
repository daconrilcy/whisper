//! Bounded-window Parakeet finalization over a disk-backed audio source.

use whisper_core::{
    JobId, Generation, SourceRange,
    parakeet_session::{AudioWindow, SessionIdentity, WindowCursor, TranscriptionStrategy},
    parakeet_windows::{MARGIN_SAMPLES, WINDOW_MAX_SAMPLES},
    ports::{ParakeetDocumentPort, ParakeetWindowCoverage, PortError, WorkerSegment},
};

use crate::parakeet_audio::{AudioStoreError, PcmPartStore};

pub trait WindowTranscriber {
    fn instance_id(&self) -> u64;
    fn transcribe(&mut self, window: AudioWindow, pcm: &[i16]) -> Result<Vec<whisper_core::parakeet_ipc::Word>, String>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PipelineResult {
    pub identity: SessionIdentity,
    pub total_samples: u64,
    pub windows_completed: u64,
    pub document_revision: u64,
}

pub fn finalize_source<T: WindowTranscriber, D: ParakeetDocumentPort>(
    source: &PcmPartStore,
    document: &mut D,
    transcriber: &mut T,
    job_id: JobId,
    generation: Generation,
) -> Result<PipelineResult, String> {
    let total_samples = source.total_samples().map_err(store_error)?;
    if total_samples == 0 { return Err("InvalidInput: empty Parakeet source".into()); }
    let identity = SessionIdentity { job_id, generation, strategy: TranscriptionStrategy::BoundedWindowsV1 };
    let mut cursor = WindowCursor::default();
    let mut windows_completed = 0_u64;
    let mut next_segment = 1_u64;

    while cursor.next_start() < total_samples {
        let central_start = cursor.next_start();
        let buffer_start = central_start.saturating_sub(MARGIN_SAMPLES);
        let buffer_end = central_start.saturating_add(WINDOW_MAX_SAMPLES)
            .saturating_add(MARGIN_SAMPLES).min(total_samples);
        let samples = source.read_window(buffer_start..buffer_end).map_err(store_error)?;
        let window = cursor.next_window(buffer_start, &samples, total_samples, true)
            .map_err(|error| format!("WindowPlanning: {error:?}"))?
            .ok_or_else(|| "WindowPlanning: closed source did not produce its next window".to_owned())?;
        let local_start = usize::try_from(window.native_start_sample - buffer_start)
            .map_err(|_| "InvalidInput: window start exceeds address space".to_owned())?;
        let local_end = usize::try_from(window.native_end_sample - buffer_start)
            .map_err(|_| "InvalidInput: window end exceeds address space".to_owned())?;
        let clip = samples.get(local_start..local_end)
            .ok_or_else(|| "StorageCorrupt: planned clip lies outside the source page".to_owned())?;
        let words = transcriber.transcribe(window, clip)?;
        let segments = validated_segments(&words, window, total_samples, transcriber.instance_id(), job_id, generation, next_segment)?;
        for segment in segments {
            document.append_final(segment).map_err(port_error)?;
            next_segment = next_segment.checked_add(1).ok_or_else(|| "InvalidInput: segment sequence exhausted".to_owned())?;
        }
        let last_segment_sequence = next_segment - 1;
        let range = SourceRange::new(window.central_start_sample, window.central_end_sample, 16_000)
            .ok_or_else(|| "InvalidInput: invalid central coverage range".to_owned())?;
        document.record_coverage(ParakeetWindowCoverage {
            sequence: window.sequence,
            range,
            last_segment_sequence,
        }).map_err(port_error)?;
        windows_completed = windows_completed.checked_add(1).ok_or_else(|| "InvalidInput: window sequence exhausted".to_owned())?;
    }
    let document_revision = document.finalize(job_id, generation, total_samples).map_err(port_error)?;
    Ok(PipelineResult { identity, total_samples, windows_completed, document_revision })
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
    let native_samples = window.native_end_sample.checked_sub(window.native_start_sample)
        .ok_or_else(|| "InvalidTimestamp: native window is reversed".to_owned())?;
    let mut groups = Vec::<(u64, u64, Vec<String>)>::new();
    let mut previous_end = 0_u64;
    for (index, word) in words.iter().enumerate() {
        let local_start = u64::from(word.start_ms).checked_mul(16)
            .ok_or_else(|| format!("InvalidTimestamp: word {index} start overflow"))?;
        let mut local_end = u64::from(word.end_ms).checked_mul(16)
            .ok_or_else(|| format!("InvalidTimestamp: word {index} end overflow"))?;
        let token = word.text.trim();
        if token.is_empty() || local_start >= local_end || local_start.saturating_add(80 * 16) < previous_end {
            return Err(format!("InvalidTimestamp: word {index} is invalid or unordered"));
        }
        if local_end > native_samples {
            let overflow = local_end - native_samples;
            let final_file_word = window.native_end_sample == total_samples && index + 1 == words.len();
            if !final_file_word || overflow > 80 * 16 {
                return Err(format!("InvalidTimestamp: word {index} exceeds its native window"));
            }
            local_end = native_samples;
        }
        if local_start >= local_end {
            return Err(format!("InvalidTimestamp: word {index} is empty after final-window capping"));
        }
        previous_end = previous_end.max(local_end);
        let start = window.native_start_sample.checked_add(local_start)
            .ok_or_else(|| format!("InvalidTimestamp: word {index} global start overflow"))?;
        let end = window.native_start_sample.checked_add(local_end)
            .ok_or_else(|| format!("InvalidTimestamp: word {index} global end overflow"))?;
        let midpoint = start + (end - start) / 2;
        if midpoint < window.central_start_sample || midpoint >= window.central_end_sample { continue; }
        let punctuation_boundary = groups.last().is_some_and(|(_, _, tokens)| {
            tokens.last().is_some_and(|last| last.ends_with('.') || last.ends_with('!') || last.ends_with('?'))
        });
        if groups.is_empty() || punctuation_boundary {
            groups.push((start, end, vec![token.to_owned()]));
        } else if let Some((group_start, group_end, tokens)) = groups.last_mut() {
            *group_start = (*group_start).min(start);
            *group_end = (*group_end).max(end);
            tokens.push(token.to_owned());
        }
    }
    groups.into_iter().enumerate().map(|(index, (start, end, words))| {
        let sequence = first_sequence.checked_add(index as u64)
            .ok_or_else(|| "InvalidInput: segment sequence exhausted".to_owned())?;
        let range = SourceRange::new(start, end, 16_000)
            .ok_or_else(|| "InvalidTimestamp: grouped word range is invalid".to_owned())?;
        Ok(WorkerSegment { job_id, generation, instance_id, segment_id: whisper_core::SegmentId(sequence), range, text: words.join(" ") })
    }).collect()
}

fn store_error(error: AudioStoreError) -> String { format!("AudioSource: {error}") }
fn port_error(error: PortError) -> String { format!("Document: {error:?}") }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parakeet_audio::PcmPartWriter, parakeet_document::ParakeetDocumentStore};
    use std::{fs, path::PathBuf, time::{SystemTime, UNIX_EPOCH}};

    struct ControlledWorker;
    impl WindowTranscriber for ControlledWorker {
        fn instance_id(&self) -> u64 { 7 }
        fn transcribe(&mut self, window: AudioWindow, _: &[i16]) -> Result<Vec<whisper_core::parakeet_ipc::Word>, String> {
            let start_ms = ((window.central_start_sample - window.native_start_sample) / 16) as u32;
            Ok(vec![whisper_core::parakeet_ipc::Word {
                text: "word.".into(), start_ms, end_ms: start_ms + 500,
            }])
        }
    }

    fn word(start_ms: u32, end_ms: u32) -> whisper_core::parakeet_ipc::Word {
        whisper_core::parakeet_ipc::Word { text: "last".into(), start_ms, end_ms }
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
        let accepted = validated_segments(&[word(950, 1_010)], final_window, 16_000, 7, JobId(1), Generation::first(), 1).unwrap();
        assert_eq!(accepted[0].range.end_sample, 16_000);
        assert!(validated_segments(&[word(950, 1_081)], final_window, 16_000, 7, JobId(1), Generation::first(), 1).is_err());

        let internal_window = AudioWindow { native_end_sample: 16_000, ..final_window };
        assert!(validated_segments(&[word(950, 1_010)], internal_window, 32_000, 7, JobId(1), Generation::first(), 1).is_err());
        assert!(validated_segments(&[word(950, 1_010), word(1_000, 1_020)], final_window, 16_000, 7, JobId(1), Generation::first(), 1).is_err());
    }

    #[test]
    fn controlled_worker_processes_a_long_source_in_contiguous_windows() {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("whisper-parakeet-pipeline-{stamp}"));
        fs::create_dir_all(&root).unwrap();
        let audio_path = root.join("source.wav");
        let mut writer = PcmPartWriter::create(&audio_path).unwrap();
        let total = 121 * 16_000_u64;
        writer.append(&vec![0_i16; total as usize]).unwrap();
        writer.finish().unwrap();
        let source = PcmPartStore::open(&audio_path);
        let mut document = ParakeetDocumentStore::create(root.join("document"), JobId(33), Generation::first()).unwrap();
        let result = finalize_source(&source, &mut document, &mut ControlledWorker, JobId(33), Generation::first()).unwrap();
        assert_eq!(result.total_samples, total);
        assert_eq!(result.windows_completed, 2);
        assert_eq!(document.revision(), 2);
        fs::remove_dir_all(root).unwrap();
    }
}
