use crate::domain::{Generation, JobId};
use crate::ports::ImportEvent;

pub(super) fn record_application_timing(
    phase: &str,
    job_id: Option<JobId>,
    generation: Option<Generation>,
    sequence: Option<u64>,
) {
    if std::env::var_os("PARAKEET_L3D_TIMING_ACTIVE").as_deref() != Some(std::ffi::OsStr::new("1"))
    {
        return;
    }
    static ORIGIN: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    static REFRESHES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let elapsed_us = ORIGIN
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_micros();
    let refresh = if phase == "refresh" {
        REFRESHES.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1
    } else {
        REFRESHES.load(std::sync::atomic::Ordering::Relaxed)
    };
    eprintln!(
        "L3D_APP_TRACE pid={} elapsed_us={elapsed_us} refresh={refresh} phase={phase} job_id={job_id:?} generation={generation:?} sequence={sequence:?}",
        std::process::id()
    );
}

pub(super) fn trace_application_event(event: &ImportEvent) {
    let (kind, job_id, generation, sequence) = match event {
        ImportEvent::DirectCaptureActive {
            job_id, generation, ..
        }
        | ImportEvent::DirectPublished {
            job_id, generation, ..
        }
        | ImportEvent::MicrophoneGainApplied {
            job_id, generation, ..
        }
        | ImportEvent::MicrophoneGainRejected {
            job_id, generation, ..
        }
        | ImportEvent::MicrophoneTestStarted {
            job_id, generation, ..
        }
        | ImportEvent::MicrophoneTestStopped { job_id, generation }
        | ImportEvent::Diagnostic {
            job_id, generation, ..
        }
        | ImportEvent::ParakeetLiveStarted {
            job_id, generation, ..
        }
        | ImportEvent::ParakeetInferenceReady {
            job_id, generation, ..
        }
        | ImportEvent::ParakeetHypothesis {
            job_id, generation, ..
        }
        | ImportEvent::ParakeetConfirmed {
            job_id, generation, ..
        }
        | ImportEvent::ParakeetUnresolved {
            job_id, generation, ..
        }
        | ImportEvent::ParakeetResolved {
            job_id, generation, ..
        }
        | ImportEvent::ParakeetCaptureProgress {
            job_id, generation, ..
        }
        | ImportEvent::ParakeetAudioUnusable {
            job_id, generation, ..
        }
        | ImportEvent::ParakeetInferenceUnavailable {
            job_id, generation, ..
        }
        | ImportEvent::ParakeetCaptureStopped {
            job_id, generation, ..
        }
        | ImportEvent::LiveCounters {
            job_id, generation, ..
        }
        | ImportEvent::WindowFinished {
            job_id, generation, ..
        }
        | ImportEvent::AwaitingCpuChoice { job_id, generation }
        | ImportEvent::LiveAttemptReset {
            job_id, generation, ..
        }
        | ImportEvent::LiveAttemptStarting { job_id, generation }
        | ImportEvent::LiveRetryingOnCpu {
            job_id, generation, ..
        }
        | ImportEvent::LiveFinalizing { job_id, generation }
        | ImportEvent::MicrophoneMeter(crate::microphone_level::MicrophoneMeter {
            job_id,
            generation,
            ..
        })
        | ImportEvent::MicrophoneSpectrum(
            crate::microphone_level::MicrophoneSpectrumUpdate::Available(
                crate::microphone_level::MicrophoneSpectrum {
                    job_id, generation, ..
                },
            ),
        )
        | ImportEvent::MicrophoneSpectrum(
            crate::microphone_level::MicrophoneSpectrumUpdate::Unavailable {
                job_id,
                generation,
                ..
            },
        )
        | ImportEvent::ResumeCheckpoint {
            job_id, generation, ..
        }
        | ImportEvent::Ready {
            job_id, generation, ..
        }
        | ImportEvent::Progress {
            job_id, generation, ..
        }
        | ImportEvent::Persisted {
            job_id, generation, ..
        }
        | ImportEvent::End {
            job_id, generation, ..
        }
        | ImportEvent::Published { job_id, generation }
        | ImportEvent::UnifiedDocumentPage {
            job_id, generation, ..
        }
        | ImportEvent::UnifiedAudioPartsPage {
            job_id, generation, ..
        }
        | ImportEvent::UnifiedPublicationArtifacts {
            job_id, generation, ..
        }
        | ImportEvent::Failed {
            job_id, generation, ..
        }
        | ImportEvent::Stopped { job_id, generation } => {
            let sequence = match event {
                ImportEvent::WindowFinished {
                    window_sequence, ..
                } => Some(*window_sequence),
                ImportEvent::Persisted { sequence, .. } => Some(*sequence),
                ImportEvent::End { last_sequence, .. } => Some(*last_sequence),
                _ => None,
            };
            (
                event.kind_name(),
                Some(*job_id),
                Some(*generation),
                sequence,
            )
        }
        ImportEvent::DirectConfirmed(segment)
        | ImportEvent::LiveProvisional { segment, .. }
        | ImportEvent::Segment(segment) => (
            event.kind_name(),
            Some(segment.job_id),
            Some(segment.generation),
            Some(segment.segment_id.0),
        ),
        ImportEvent::Prepared(request) => (
            event.kind_name(),
            Some(request.job_id),
            Some(request.generation),
            None,
        ),
        ImportEvent::History(_)
        | ImportEvent::ReplacementCleanups(_)
        | ImportEvent::Queue(_)
        | ImportEvent::QueueAdmission { .. }
        | ImportEvent::DuplicateImportFound(_)
        | ImportEvent::SourceSnapshotRecoveryInspected(_)
        | ImportEvent::SourceSnapshotRecoveryCompleted { .. } => {
            (event.kind_name(), None, None, None)
        }
    };
    record_application_timing(kind, job_id, generation, sequence);
}
