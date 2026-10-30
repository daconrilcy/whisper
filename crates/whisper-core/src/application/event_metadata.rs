use crate::domain::{Generation, JobId};
use crate::ports::ImportEvent;

impl ImportEvent {
    pub(super) fn kind_name(&self) -> &'static str {
        match self {
            Self::DirectCaptureActive { .. } => "direct_capture_active",
            Self::DirectConfirmed(_) => "direct_confirmed",
            Self::DirectPublished { .. } => "direct_published",
            Self::MicrophoneGainApplied { .. } => "microphone_gain_applied",
            Self::MicrophoneGainRejected { .. } => "microphone_gain_rejected",
            Self::MicrophoneTestStarted { .. } => "microphone_test_started",
            Self::MicrophoneMeter(_) => "microphone_meter",
            Self::MicrophoneSpectrum(_) => "microphone_spectrum",
            Self::MicrophoneTestStopped { .. } => "microphone_test_stopped",
            Self::Diagnostic { .. } => "diagnostic",
            Self::LiveProvisional { .. } => "live_provisional",
            Self::ParakeetLiveStarted { .. } => "parakeet_live_started",
            Self::ParakeetInferenceReady { .. } => "parakeet_inference_ready",
            Self::ParakeetHypothesis { .. } => "parakeet_hypothesis",
            Self::ParakeetConfirmed { .. } => "parakeet_confirmed",
            Self::ParakeetUnresolved { .. } => "parakeet_unresolved",
            Self::ParakeetResolved { .. } => "parakeet_resolved",
            Self::ParakeetCaptureProgress { .. } => "parakeet_capture_progress",
            Self::ParakeetAudioUnusable { .. } => "parakeet_audio_unusable",
            Self::ParakeetInferenceUnavailable { .. } => "parakeet_inference_unavailable",
            Self::ParakeetCaptureStopped { .. } => "parakeet_capture_stopped",
            Self::LiveCounters { .. } => "live_counters",
            Self::WindowFinished { .. } => "window_finished",
            Self::AwaitingCpuChoice { .. } => "awaiting_cpu_choice",
            Self::LiveAttemptReset { .. } => "live_attempt_reset",
            Self::LiveAttemptStarting { .. } => "live_attempt_starting",
            Self::LiveRetryingOnCpu { .. } => "live_retrying_on_cpu",
            Self::LiveFinalizing { .. } => "live_finalizing",
            Self::Queue(_) => "queue",
            Self::QueueAdmission { .. } => "queue_admission",
            Self::SourceSnapshotRecoveryInspected(_) => "source_snapshot_recovery_inspected",
            Self::SourceSnapshotRecoveryCompleted { .. } => "source_snapshot_recovery_completed",
            Self::DuplicateImportFound(_) => "duplicate_import_found",
            Self::ResumeCheckpoint { .. } => "resume_checkpoint",
            Self::Prepared(_) => "prepared",
            Self::History(_) => "history",
            Self::ReplacementCleanups(_) => "replacement_cleanups",
            Self::Ready { .. } => "ready",
            Self::Progress { .. } => "progress",
            Self::Segment(_) => "segment",
            Self::Persisted { .. } => "persisted",
            Self::End { .. } => "end",
            Self::Published { .. } => "published",
            Self::UnifiedDocumentPage { .. } => "unified_document_page",
            Self::UnifiedAudioPartsPage { .. } => "unified_audio_parts_page",
            Self::UnifiedPublicationArtifacts { .. } => "unified_publication_artifacts",
            Self::Failed { .. } => "failed",
            Self::Stopped { .. } => "stopped",
        }
    }
}

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
