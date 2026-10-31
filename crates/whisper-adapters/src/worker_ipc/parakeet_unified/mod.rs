use super::NativeUnifiedRoute;
use std::path::PathBuf;
use whisper_core::{
    Generation, JobId,
    ports::{ImportRequest, WorkerSegment},
};
mod executor;
mod native;
pub(super) use executor::{UnifiedInferenceCommand, UnifiedInferenceExecutor};
pub(super) use native::NativeParakeetWindowTranscriber;
#[cfg(test)]
pub(super) use native::validate_native_window_pcm;

pub(in crate::worker_ipc) struct UnifiedDocumentDrain {
    pub(in crate::worker_ipc) document: crate::parakeet_document::ParakeetDocumentStore,
    pub(in crate::worker_ipc) job_id: JobId,
    pub(in crate::worker_ipc) generation: Generation,
    pub(in crate::worker_ipc) revision: u64,
    pub(in crate::worker_ipc) total_samples: u64,
    pub(in crate::worker_ipc) next_sequence: u64,
    pub(in crate::worker_ipc) ready_pending: bool,
    pub(in crate::worker_ipc) progress_pending: bool,
}

pub(in crate::worker_ipc) fn restore_unified_document_prefix(
    document: &mut crate::parakeet_document::ParakeetDocumentStore,
    checkpoint: &crate::archive::UnifiedArchiveResumeCheckpoint,
    job_id: JobId,
    generation: Generation,
    source_samples: u64,
) -> Result<(), String> {
    use whisper_core::ports::ParakeetDocumentPort;

    let saved = checkpoint.pipeline;
    if saved.confirmed_segments == 0 && saved.window_sequence == 0 && saved.covered_samples == 0 {
        return Ok(());
    }
    let mut reader = crate::parakeet_document::ParakeetDocumentStore::open_resume_coverage(
        &checkpoint.previous_document_directory,
        checkpoint.previous_job_id,
        checkpoint.previous_generation,
        source_samples,
    )?
    .ok_or_else(|| "StorageCorrupt: resume document checkpoint disappeared".to_owned())?;
    let mut next_segment = 1_u64;
    let mut last_coverage = None;
    loop {
        let page = reader.read_page(128)?;
        if page.is_empty() {
            break;
        }
        for coverage in page {
            if coverage.sequence > saved.window_sequence {
                break;
            }
            while next_segment <= coverage.last_segment_sequence {
                let entries = reader.read_document_page(next_segment, 128)?;
                if entries.is_empty() {
                    return Err("StorageCorrupt: resume document prefix ended early".into());
                }
                for entry in entries {
                    if entry.segment.segment_id.0 > coverage.last_segment_sequence
                        || entry.segment.segment_id.0 > saved.confirmed_segments
                    {
                        break;
                    }
                    let segment = WorkerSegment {
                        job_id,
                        generation,
                        instance_id: entry.segment.instance_id,
                        segment_id: entry.segment.segment_id,
                        range: entry.segment.range,
                        text: entry.segment.text,
                    };
                    document
                        .append_final(segment)
                        .map_err(|error| format!("Document: {error:?}"))?;
                    next_segment = next_segment.checked_add(1).ok_or_else(|| {
                        "StorageCorrupt: resume segment sequence exhausted".to_owned()
                    })?;
                }
            }
            document
                .record_coverage(coverage)
                .map_err(|error| format!("Document: {error:?}"))?;
            last_coverage = Some(coverage.sequence);
            if coverage.sequence == saved.window_sequence {
                break;
            }
        }
        if last_coverage == Some(saved.window_sequence) {
            break;
        }
    }
    if last_coverage != Some(saved.window_sequence)
        || next_segment != saved.confirmed_segments.saturating_add(1)
        || document.revision() != saved.confirmed_segments
        || saved.covered_samples == 0
    {
        return Err("StorageCorrupt: resume checkpoint prefix could not be restored".into());
    }
    Ok(())
}

pub(in crate::worker_ipc) enum UnifiedSessionSource {
    Closed(crate::parakeet_audio::PcmPartStore),
    Capture(Box<crate::parakeet_live::UnifiedCapturePump>),
}

impl crate::parakeet_pipeline::ReadablePcmSource for UnifiedSessionSource {
    fn read_window(&mut self, range: std::ops::Range<u64>) -> Result<Vec<i16>, String> {
        match self {
            Self::Closed(source) => source.read_window(range),
            Self::Capture(source) => source.read_window(range),
        }
    }
}

impl UnifiedSessionSource {
    pub(in crate::worker_ipc) fn is_capture(&self) -> bool {
        matches!(self, Self::Capture(_))
    }

    pub(in crate::worker_ipc) fn pump_capture(
        &mut self,
    ) -> Result<Vec<crate::parakeet_audio::CaptureWriteAck>, String> {
        match self {
            Self::Closed(_) => Ok(Vec::new()),
            Self::Capture(capture) => capture.pump(),
        }
    }

    pub(in crate::worker_ipc) fn stop_capture(
        &mut self,
    ) -> Result<Option<(u64, Vec<crate::parakeet_audio::CaptureWriteAck>)>, String> {
        match self {
            Self::Closed(_) => Ok(None),
            Self::Capture(capture) => capture.stop().map(Some),
        }
    }

    pub(in crate::worker_ipc) fn set_gain(
        &self,
        target_identity: &str,
        expected_revision: u64,
        gain: whisper_core::microphone_level::GainDb,
    ) -> Result<whisper_core::microphone_level::GainRevision, String> {
        match self {
            Self::Capture(capture) => capture.set_gain(target_identity, expected_revision, gain),
            Self::Closed(_) => Err("MicrophoneLevel: unified capture is not open".into()),
        }
    }

    pub(in crate::worker_ipc) fn take_gain_events(
        &mut self,
    ) -> Vec<crate::parakeet_live::AppliedCaptureGain> {
        match self {
            Self::Capture(capture) => capture.take_gain_events(),
            Self::Closed(_) => Vec::new(),
        }
    }

    #[cfg(test)]
    pub(in crate::worker_ipc) fn enqueue_test_block(&mut self, samples: Vec<f32>) {
        if let Self::Capture(capture) = self {
            capture.enqueue_test_block(samples);
        }
    }

    pub(in crate::worker_ipc) fn pump(
        &mut self,
    ) -> Result<Vec<crate::parakeet_audio::CaptureWriteAck>, String> {
        self.pump_capture()
    }
}

pub(in crate::worker_ipc) struct UnifiedSessionState {
    pub(in crate::worker_ipc) request: ImportRequest,
    pub(in crate::worker_ipc) document_drain_start_sequence: u64,
    pub(in crate::worker_ipc) source: UnifiedSessionSource,
    pub(in crate::worker_ipc) available_samples: u64,
    pub(in crate::worker_ipc) durable_samples: u64,
    pub(in crate::worker_ipc) source_closed: bool,
    pub(in crate::worker_ipc) document: crate::parakeet_document::ParakeetDocumentStore,
    pub(in crate::worker_ipc) inference_directory: PathBuf,
    pub(in crate::worker_ipc) native_route: Option<NativeUnifiedRoute>,
    pub(in crate::worker_ipc) backend_ready_reported: bool,
    pub(in crate::worker_ipc) finalizer: crate::parakeet_pipeline::ParakeetFinalizer,
    pub(in crate::worker_ipc) pending_call: Option<whisper_core::parakeet_scheduler::ScheduledCall>,
    pub(in crate::worker_ipc) pending_prepared: Option<(
        whisper_core::parakeet_scheduler::ScheduledCall,
        whisper_core::parakeet_session::AudioWindow,
        Vec<i16>,
        bool,
    )>,
    pub(in crate::worker_ipc) preview_sequence: u64,
    pub(in crate::worker_ipc) pending_window: Option<whisper_core::parakeet_session::AudioWindow>,
    pub(in crate::worker_ipc) pending_available_samples: Option<u64>,
    pub(in crate::worker_ipc) failure: Option<String>,
}
