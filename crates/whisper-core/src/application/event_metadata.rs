use crate::domain::{Generation, JobId};
use crate::ports::ImportEvent;

impl ImportEvent {
    pub(super) fn identity(&self) -> (JobId, Generation) {
        match self {
            Self::DirectCaptureActive {
                job_id, generation, ..
            } => (*job_id, *generation),
            Self::DirectConfirmed(segment) => (segment.job_id, segment.generation),
            Self::DirectPublished {
                job_id, generation, ..
            } => (*job_id, *generation),
            Self::MicrophoneGainApplied {
                job_id, generation, ..
            }
            | Self::MicrophoneGainRejected {
                job_id, generation, ..
            } => (*job_id, *generation),
            Self::MicrophoneTestStarted {
                job_id, generation, ..
            }
            | Self::MicrophoneTestStopped { job_id, generation } => (*job_id, *generation),
            Self::MicrophoneMeter(meter) => (meter.job_id, meter.generation),
            Self::MicrophoneSpectrum(update) => update.owner(),
            Self::LiveProvisional { segment, .. } => (segment.job_id, segment.generation),
            Self::ParakeetLiveStarted {
                job_id, generation, ..
            }
            | Self::ParakeetInferenceReady {
                job_id, generation, ..
            }
            | Self::ParakeetHypothesis {
                job_id, generation, ..
            }
            | Self::ParakeetConfirmed {
                job_id, generation, ..
            }
            | Self::ParakeetUnresolved {
                job_id, generation, ..
            }
            | Self::ParakeetResolved {
                job_id, generation, ..
            }
            | Self::ParakeetCaptureProgress {
                job_id, generation, ..
            }
            | Self::ParakeetAudioUnusable {
                job_id, generation, ..
            }
            | Self::ParakeetInferenceUnavailable {
                job_id, generation, ..
            }
            | Self::ParakeetCaptureStopped {
                job_id, generation, ..
            } => (*job_id, *generation),
            Self::LiveCounters {
                job_id, generation, ..
            } => (*job_id, *generation),
            Self::WindowFinished {
                job_id, generation, ..
            } => (*job_id, *generation),
            Self::AwaitingCpuChoice { job_id, generation } => (*job_id, *generation),
            Self::LiveAttemptReset {
                job_id, generation, ..
            } => (*job_id, *generation),
            Self::LiveRetryingOnCpu {
                job_id, generation, ..
            } => (*job_id, *generation),
            Self::LiveAttemptStarting { job_id, generation } => (*job_id, *generation),
            Self::LiveFinalizing { job_id, generation }
            | Self::Diagnostic {
                job_id, generation, ..
            } => (*job_id, *generation),
            Self::Ready {
                job_id, generation, ..
            }
            | Self::Progress {
                job_id, generation, ..
            }
            | Self::Persisted {
                job_id, generation, ..
            }
            | Self::End {
                job_id, generation, ..
            }
            | Self::Published { job_id, generation }
            | Self::UnifiedDocumentPage {
                job_id, generation, ..
            }
            | Self::UnifiedAudioPartsPage {
                job_id, generation, ..
            }
            | Self::UnifiedPublicationArtifacts {
                job_id, generation, ..
            }
            | Self::Failed {
                job_id, generation, ..
            }
            | Self::Stopped { job_id, generation } => (*job_id, *generation),
            Self::ResumeCheckpoint {
                job_id, generation, ..
            } => (*job_id, *generation),
            Self::Prepared(request) => (request.job_id, request.generation),
            Self::History(_)
            | Self::ReplacementCleanups(_)
            | Self::Queue(_)
            | Self::QueueAdmission { .. }
            | Self::DuplicateImportFound(_)
            | Self::SourceSnapshotRecoveryInspected(_)
            | Self::SourceSnapshotRecoveryCompleted { .. } => (JobId(0), Generation::first()),
            Self::Segment(segment) => (segment.job_id, segment.generation),
        }
    }
}
