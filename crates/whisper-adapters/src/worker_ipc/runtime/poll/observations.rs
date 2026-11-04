#![warn(clippy::too_many_lines, clippy::cognitive_complexity)]

use super::*;

impl ImportRuntime {
    pub(super) fn take_observation_notice(&mut self) -> Option<ImportEvent> {
        if let Some(progress) = self.progress.as_mut()
            && let Some((phase, idle_ms)) = progress.take_warning(std::time::Instant::now())
            && let Some(active) = self.active.as_ref()
        {
            if let Some(diagnostics) = &self.diagnostics {
                diagnostics.record(DiagnosticRecord::new(
                    active.job_id.0,
                    active.generation.get(),
                    phase.into(),
                    DiagnosticCode::ProgressWarning,
                    Some(match active.backend {
                        BackendKind::Cpu => "cpu",
                        BackendKind::Cuda => "cuda",
                    }),
                    None,
                ));
            }
            return Some(ImportEvent::Diagnostic {
                job_id: active.job_id,
                generation: active.generation,
                notice: DiagnosticNotice::ProgressStalled {
                    phase: phase.into(),
                    idle_ms,
                },
            });
        }
        let diagnostics_degraded = self.diagnostic_start_failed
            || self
                .diagnostics
                .as_ref()
                .is_some_and(DiagnosticSink::is_degraded);
        if !self.diagnostic_degraded_reported
            && diagnostics_degraded
            && let Some(active) = self.active.as_ref()
        {
            self.diagnostic_degraded_reported = true;
            if let Some(diagnostics) = self.diagnostics.as_ref() {
                diagnostics.record(DiagnosticRecord::new(
                    active.job_id.0,
                    active.generation.get(),
                    DiagnosticPhase::Finalizing,
                    DiagnosticCode::LogDegraded,
                    Some(match active.backend {
                        BackendKind::Cpu => "cpu",
                        BackendKind::Cuda => "cuda",
                    }),
                    None,
                ));
            }
            return Some(ImportEvent::Diagnostic {
                job_id: active.job_id,
                generation: active.generation,
                notice: DiagnosticNotice::LoggingUnavailable,
            });
        }
        None
    }

    pub(super) fn observe_worker_event(
        &mut self,
        event: &WorkerEvent,
        job_id: JobId,
        generation: Generation,
        backend: BackendKind,
        live: bool,
    ) {
        if let Some(diagnostics) = &self.diagnostics {
            let backend = Some(match backend {
                BackendKind::Cpu => "cpu",
                BackendKind::Cuda => "cuda",
            });
            let record = match event {
                WorkerEvent::Ready { .. } => Some(DiagnosticRecord::new(
                    job_id.0,
                    generation.get(),
                    DiagnosticPhase::Preparing,
                    DiagnosticCode::BackendReady,
                    backend,
                    None,
                )),
                WorkerEvent::Progress {
                    completed_samples,
                    total_samples,
                    ..
                } => Some(DiagnosticRecord::new(
                    job_id.0,
                    generation.get(),
                    if live {
                        DiagnosticPhase::Inference
                    } else {
                        DiagnosticPhase::Import
                    },
                    DiagnosticCode::Progress,
                    backend,
                    Some((*completed_samples, *total_samples)),
                )),
                WorkerEvent::Failed { .. } => Some(DiagnosticRecord::new(
                    job_id.0,
                    generation.get(),
                    DiagnosticPhase::Finalizing,
                    DiagnosticCode::WorkerFailed,
                    backend,
                    None,
                )),
                WorkerEvent::Stopped { .. } => Some(DiagnosticRecord::new(
                    job_id.0,
                    generation.get(),
                    DiagnosticPhase::Cancelling,
                    DiagnosticCode::WorkerStopped,
                    backend,
                    None,
                )),
                _ => None,
            };
            if let Some(record) = record {
                diagnostics.record(record);
            }
        }
        if let Some(progress) = self.progress.as_mut() {
            match event {
                WorkerEvent::Ready { .. } => {
                    let now = std::time::Instant::now();
                    progress.close(ProgressPhase::Preparing);
                    progress.open(ProgressPhase::Inference, now);
                    if self.live.is_some() || self.pending_live.is_some() {
                        progress.open(ProgressPhase::Capturing, now);
                        progress.open(ProgressPhase::Draining, now);
                    } else {
                        progress.open(ProgressPhase::Import, now);
                    }
                }
                WorkerEvent::Progress {
                    completed_samples, ..
                } => {
                    let now = std::time::Instant::now();
                    progress.observe(ProgressPhase::Inference, *completed_samples, now);
                    progress.observe(ProgressPhase::Import, *completed_samples, now);
                }
                WorkerEvent::Segment(dto) => progress.observe(
                    ProgressPhase::Inference,
                    dto.segment_id.0,
                    std::time::Instant::now(),
                ),
                WorkerEvent::End { .. } => {
                    let now = std::time::Instant::now();
                    progress.close(ProgressPhase::Inference);
                    progress.close(ProgressPhase::Import);
                    progress.open(ProgressPhase::Finalizing, now);
                }
                WorkerEvent::Stopped { .. } | WorkerEvent::Failed { .. } => progress.close_all(),
                _ => {}
            }
        }
    }
}
