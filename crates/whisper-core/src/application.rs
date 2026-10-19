use crate::domain::{Generation, JobConfig, JobId, JobState};
use crate::ports::{
    ArchiveHistoryItem, DiagnosticNotice, ImportEffect, ImportEvent, ImportIoPort,
    ImportQueueEntry, ImportRequest, LiveRequest, PortError,
};

const MAX_EVENTS_PER_REFRESH: usize = 4;

fn l3d_timing_line(
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

fn l3d_event_trace(event: &ImportEvent) {
    let (kind, job_id, generation, sequence) = match event {
        ImportEvent::DirectCaptureActive {
            job_id, generation, ..
        }
        | ImportEvent::DirectPublished {
            job_id, generation, ..
        }
        | ImportEvent::Diagnostic {
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
        | ImportEvent::LiveRetryingOnCpu { job_id, generation }
        | ImportEvent::LiveFinalizing { job_id, generation }
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
        ImportEvent::History(_) | ImportEvent::Queue(_) => (event.kind_name(), None, None, None),
    };
    l3d_timing_line(kind, job_id, generation, sequence);
}

impl ImportEvent {
    fn kind_name(&self) -> &'static str {
        match self {
            Self::DirectCaptureActive { .. } => "direct_capture_active",
            Self::DirectConfirmed(_) => "direct_confirmed",
            Self::DirectPublished { .. } => "direct_published",
            Self::Diagnostic { .. } => "diagnostic",
            Self::LiveProvisional { .. } => "live_provisional",
            Self::LiveCounters { .. } => "live_counters",
            Self::WindowFinished { .. } => "window_finished",
            Self::AwaitingCpuChoice { .. } => "awaiting_cpu_choice",
            Self::LiveAttemptReset { .. } => "live_attempt_reset",
            Self::LiveAttemptStarting { .. } => "live_attempt_starting",
            Self::LiveRetryingOnCpu { .. } => "live_retrying_on_cpu",
            Self::LiveFinalizing { .. } => "live_finalizing",
            Self::Queue(_) => "queue",
            Self::ResumeCheckpoint { .. } => "resume_checkpoint",
            Self::Prepared(_) => "prepared",
            Self::History(_) => "history",
            Self::Ready { .. } => "ready",
            Self::Progress { .. } => "progress",
            Self::Segment(_) => "segment",
            Self::Persisted { .. } => "persisted",
            Self::End { .. } => "end",
            Self::Published { .. } => "published",
            Self::Failed { .. } => "failed",
            Self::Stopped { .. } => "stopped",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandIdentity {
    pub job_id: JobId,
    pub generation: Generation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AppCommand {
    Refresh,
    StartImport {
        request: ImportRequest,
    },
    EnqueueImport {
        request: ImportRequest,
    },
    ProcessQueued {
        request: ImportRequest,
    },
    ResumeInterrupted {
        request: ImportRequest,
    },
    RemoveQueued {
        job_id: JobId,
        generation: Generation,
    },
    StartLive {
        job_id: JobId,
        generation: Generation,
        config: JobConfig,
    },
    StartLiveConfigured {
        request: LiveRequest,
    },
    StartDirectLiveConfigured {
        request: LiveRequest,
    },
    FinishLiveOnCpu {
        job_id: JobId,
        generation: Generation,
    },
    Stop {
        job_id: JobId,
        generation: Generation,
    },
    Quit {
        job_id: JobId,
        generation: Generation,
    },
    ScanHistory {
        destination: String,
    },
    ResolveLivePublication {
        request: LiveRequest,
    },
}

impl AppCommand {
    pub fn identity(&self) -> Option<CommandIdentity> {
        match self {
            Self::Refresh => None,
            Self::StartLive {
                job_id, generation, ..
            }
            | Self::Stop { job_id, generation }
            | Self::Quit { job_id, generation }
            | Self::FinishLiveOnCpu { job_id, generation } => Some(CommandIdentity {
                job_id: *job_id,
                generation: *generation,
            }),
            Self::StartImport { request } | Self::EnqueueImport { request } => {
                Some(CommandIdentity {
                    job_id: request.job_id,
                    generation: request.generation,
                })
            }
            Self::ProcessQueued { request } | Self::ResumeInterrupted { request } => {
                Some(CommandIdentity {
                    job_id: request.job_id,
                    generation: request.generation,
                })
            }
            Self::RemoveQueued { job_id, generation } => Some(CommandIdentity {
                job_id: *job_id,
                generation: *generation,
            }),
            Self::StartLiveConfigured { request }
            | Self::StartDirectLiveConfigured { request }
            | Self::ResolveLivePublication { request } => Some(CommandIdentity {
                job_id: request.job_id,
                generation: request.generation,
            }),
            Self::ScanHistory { .. } => None,
        }
    }

    pub fn targets(&self, job_id: JobId, generation: Generation) -> bool {
        self.identity() == Some(CommandIdentity { job_id, generation })
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct AppView {
    pub active_job: Option<(JobId, JobState)>,
    pub active_generation: Option<Generation>,
    pub queued_jobs: usize,
    pub message: Option<String>,
    pub diagnostics_degraded: bool,
    pub progress: Option<(u64, u64)>,
    pub last_result: Option<String>,
    pub history: Vec<ArchiveHistoryItem>,
    pub queue: Vec<ImportQueueEntry>,
    pub history_scan_pending: bool,
    pub queue_scan_pending: bool,
    pub captured_samples: u64,
    pub admitted_samples: u64,
    pub audio_durable_samples: u64,
    pub confirmed_fragment_samples: u64,
    pub confirmed_samples: u64,
    pub speech_samples: u64,
    pub provisional_text: Option<String>,
    pub provisional_sequence: Option<u64>,
    pub provisional_range: Option<crate::domain::SourceRange>,
    pub provisional_captured_at_unix_ms: Option<u64>,
    pub provisional_available_at_unix_ms: Option<u64>,
    pub confirmed_text: Option<String>,
    pub direct_first_sample_at_unix_ms: Option<u64>,
    pub direct_first_admitted_at_unix_ms: Option<u64>,
    pub direct_ready_at_unix_ms: Option<u64>,
    pub direct_capture_phase: Option<DirectCapturePhase>,
    pub direct_engine_phase: Option<DirectEnginePhase>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DirectCapturePhase {
    Initializing,
    Active,
    Stopped,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DirectEnginePhase {
    Preparing,
    Ready,
    Failed,
    Released,
}

/// Owns queue admission and import lifecycle. Adapters only execute effects.
pub struct ImportApplication<P> {
    io: P,
    view: AppView,
    active: Option<CommandIdentity>,
    request: Option<ImportRequest>,
    direct_live: bool,
    next_sequence: u64,
    pending_persistence: usize,
    pending_segments: std::collections::VecDeque<crate::ports::WorkerSegment>,
    pending_live_segments: std::collections::BTreeMap<u64, crate::ports::WorkerSegment>,
    pending_enqueues: Vec<CommandIdentity>,
    auto_stop_after_speech_samples: Option<u64>,
    ended_at: Option<u64>,
    stopping: Option<CommandIdentity>,
    max_segment_offset: u64,
    final_source_samples: Option<u64>,
    model_guard: Option<crate::model_selection_app::ModelCommandGuard>,
}

impl<P: ImportIoPort> ImportApplication<P> {
    pub fn new(io: P) -> Self {
        Self {
            io,
            view: AppView::default(),
            active: None,
            request: None,
            direct_live: false,
            next_sequence: 0,
            pending_persistence: 0,
            pending_segments: std::collections::VecDeque::new(),
            pending_live_segments: std::collections::BTreeMap::new(),
            pending_enqueues: Vec::new(),
            auto_stop_after_speech_samples: None,
            ended_at: None,
            stopping: None,
            max_segment_offset: 0,
            final_source_samples: None,
            model_guard: None,
        }
    }

    /// Enables model admission for every model-bearing command before any effect is submitted.
    pub fn with_model_guard(
        mut self,
        guard: crate::model_selection_app::ModelCommandGuard,
    ) -> Self {
        self.model_guard = Some(guard);
        self
    }

    fn refuse_model_command(&mut self, reason: &str) -> AppView {
        self.view.message = Some(format!("Modèle indisponible : {reason}"));
        self.view.clone()
    }

    fn fail(&mut self, error: PortError) -> ApplicationError {
        self.view.message = Some(format!("Import indisponible : {error:?}"));
        ApplicationError::PortUnavailable
    }

    fn poll_once(&mut self) -> Result<(), ApplicationError> {
        if !self.retry_pending_segment()? {
            return Ok(());
        }
        for _ in 0..MAX_EVENTS_PER_REFRESH {
            let Some(event) = self.io.poll().map_err(|e| self.fail(e))? else {
                return Ok(());
            };
            let became_ready = matches!(&event, ImportEvent::Ready { .. });
            l3d_event_trace(&event);
            self.handle_event(event)?;
            if !self.pending_segments.is_empty() {
                return Ok(());
            }
            if became_ready {
                return Ok(());
            }
        }
        Ok(())
    }

    fn handle_event(&mut self, event: ImportEvent) -> Result<(), ApplicationError> {
        match &event {
            ImportEvent::History(history) => {
                self.view.history = history.clone();
                self.view.history_scan_pending = false;
                return Ok(());
            }
            ImportEvent::Queue(queue) => {
                self.view.queue_scan_pending = false;
                let acknowledged = self
                    .pending_enqueues
                    .iter()
                    .filter(|identity| {
                        queue.iter().any(|entry| {
                            entry.request.job_id == identity.job_id
                                && entry.request.generation == identity.generation
                        })
                    })
                    .count();
                self.pending_enqueues.retain(|identity| {
                    !queue.iter().any(|entry| {
                        entry.request.job_id == identity.job_id
                            && entry.request.generation == identity.generation
                    })
                });
                if acknowledged > 0 {
                    self.view.message = Some(format!(
                        "{acknowledged} import(s) ajouté(s) à la file durable. Choisissez Traiter pour les démarrer."
                    ));
                }
                self.view.queued_jobs = queue.len();
                self.view.queue = queue.clone();
                return Ok(());
            }
            ImportEvent::Failed {
                job_id, message, ..
            } if *job_id == JobId(0) && self.view.queue_scan_pending => {
                self.view.queue_scan_pending = false;
                self.view.message = Some(format!("File durable indisponible : {message}"));
                return Ok(());
            }
            _ => {}
        }
        let (job_id, generation) = event.identity();
        let identity = CommandIdentity { job_id, generation };
        let enqueue_failed = matches!(
            &event,
            ImportEvent::Failed { .. } if self.pending_enqueues.contains(&identity)
        );
        if self.active != Some(identity)
            && !(matches!(
                event,
                ImportEvent::Stopped { .. } | ImportEvent::Failed { .. }
            ) && self.stopping == Some(identity))
            && !enqueue_failed
        {
            return Ok(());
        }
        if self.stopping == Some(identity)
            && self.request.is_some()
            && matches!(event, ImportEvent::Segment(_))
        {
            return Ok(());
        }
        match event {
            ImportEvent::DirectCaptureActive {
                first_sample_at_unix_ms,
                first_admitted_at_unix_ms,
                ..
            } => {
                self.view.direct_first_sample_at_unix_ms = Some(first_sample_at_unix_ms);
                self.view.direct_first_admitted_at_unix_ms = Some(first_admitted_at_unix_ms);
                if self.stopping != Some(identity) {
                    self.view.direct_capture_phase = Some(DirectCapturePhase::Active);
                    self.view.active_job = Some((job_id, JobState::Capturing));
                    self.view.message = Some("Capture active — moteur en préparation".into());
                }
            }
            ImportEvent::DirectConfirmed(segment) => {
                if segment.segment_id.0 != self.next_sequence
                    || segment.range.sample_rate_hz != 16_000
                    || segment.range.start_sample >= segment.range.end_sample
                    || segment.text.trim().is_empty()
                {
                    return Err(ApplicationError::InvalidCommand);
                }
                self.next_sequence = self
                    .next_sequence
                    .checked_add(1)
                    .ok_or(ApplicationError::InvalidCommand)?;
                let text = self.view.confirmed_text.get_or_insert_with(String::new);
                if !text.is_empty() {
                    text.push('\n');
                }
                text.push_str(&segment.text);
            }
            ImportEvent::DirectPublished {
                txt_path,
                srt_path,
                wav_path,
                ..
            } => {
                self.view.direct_capture_phase = Some(DirectCapturePhase::Stopped);
                self.view.direct_engine_phase = Some(DirectEnginePhase::Released);
                self.view.active_job = Some((job_id, JobState::Complete));
                self.view.last_result = Some(format!(
                    "Transcription micro enregistrée : {txt_path} | {srt_path} | {wav_path}"
                ));
                self.view.message = None;
                self.active = None;
                self.stopping = None;
                self.direct_live = false;
            }
            ImportEvent::Diagnostic { notice, .. } => match notice {
                DiagnosticNotice::ProgressStalled { phase, idle_ms } => {
                    self.view.message = Some(format!(
                        "Aucun progrès observé pendant {} s ({phase:?}) ; l’opération continue.",
                        idle_ms / 1_000
                    ));
                }
                DiagnosticNotice::LoggingUnavailable => {
                    self.view.diagnostics_degraded = true;
                }
            },
            ImportEvent::LiveProvisional {
                segment,
                captured_at_unix_ms,
                available_at_unix_ms,
            } => {
                let sequence = segment.segment_id.0;
                self.view.provisional_text = Some(segment.text.clone());
                self.view.provisional_sequence = Some(sequence);
                self.view.provisional_range = Some(segment.range);
                self.view.provisional_captured_at_unix_ms = Some(captured_at_unix_ms);
                self.view.provisional_available_at_unix_ms = Some(available_at_unix_ms);
                self.pending_live_segments.insert(sequence, segment.clone());
                self.accept_segment(segment, job_id, generation)?;
            }
            ImportEvent::LiveCounters {
                captured_samples,
                admitted_samples,
                audio_durable_samples,
                confirmed_fragment_samples,
                speech_samples,
                diagnostic,
                ..
            } => {
                self.view.captured_samples = captured_samples;
                self.view.admitted_samples = admitted_samples;
                self.view.audio_durable_samples = audio_durable_samples;
                self.view.confirmed_fragment_samples = confirmed_fragment_samples;
                self.view.speech_samples = speech_samples;
                if let Some(message) = diagnostic {
                    self.view.message = Some(message);
                }
                if self.stopping.is_none()
                    && self
                        .auto_stop_after_speech_samples
                        .is_some_and(|limit| speech_samples >= limit)
                {
                    self.io
                        .submit(ImportEffect::Stop { job_id, generation })
                        .map_err(|e| self.fail(e))?;
                    self.stopping = Some(identity);
                    self.view.active_job = Some((
                        job_id,
                        if self.request.is_none() {
                            JobState::Draining
                        } else {
                            JobState::Cancelling
                        },
                    ));
                    self.view.message =
                        Some("Durée de parole atteinte ; finalisation du passage…".into());
                }
            }
            ImportEvent::LiveFinalizing { .. } => {
                self.view.active_job = Some((job_id, JobState::Finalizing));
            }
            ImportEvent::AwaitingCpuChoice { .. } => {
                self.view.active_job = Some((job_id, JobState::AwaitingChoice));
                self.view.message = Some(
                    "Le GPU s’est arrêté. Choisissez explicitement de terminer ce passage sur CPU."
                        .into(),
                );
            }
            ImportEvent::LiveAttemptReset {
                confirmed_samples,
                next_segment_sequence,
                ..
            } => {
                if confirmed_samples < self.view.confirmed_samples
                    || confirmed_samples > self.view.audio_durable_samples
                    || next_segment_sequence == 0
                {
                    return Err(ApplicationError::InvalidCommand);
                }
                self.view.confirmed_samples = confirmed_samples;
                self.next_sequence = next_segment_sequence;
                self.pending_live_segments.clear();
                self.view.provisional_text = None;
                self.view.provisional_sequence = None;
                self.view.provisional_range = None;
                self.view.provisional_captured_at_unix_ms = None;
                self.view.provisional_available_at_unix_ms = None;
            }
            ImportEvent::LiveAttemptStarting { .. } => {
                self.view.active_job = Some((job_id, JobState::Preparing));
                self.view.message = Some("Attente de l’attestation du worker CPU…".into());
            }
            ImportEvent::LiveRetryingOnCpu { .. } => {
                self.view.active_job = Some((job_id, JobState::Capturing));
                self.view.message = Some(
                    "Le GPU a échoué ; la capture continue et le passage sera repris sur CPU."
                        .into(),
                );
            }
            ImportEvent::WindowFinished {
                confirmed_samples, ..
            } => {
                if confirmed_samples < self.view.confirmed_samples
                    || confirmed_samples > self.view.audio_durable_samples
                {
                    return Err(ApplicationError::InvalidCommand);
                }
                self.view.confirmed_samples = confirmed_samples;
            }
            ImportEvent::History(_) => {
                unreachable!("non-lifecycle events are handled before job admission")
            }
            ImportEvent::Queue(_) => unreachable!("queue events are handled before job admission"),
            ImportEvent::ResumeCheckpoint {
                confirmed_segments,
                confirmed_offset,
                ..
            } => {
                self.next_sequence = confirmed_segments
                    .checked_add(1)
                    .ok_or(ApplicationError::InvalidCommand)?;
                self.max_segment_offset = confirmed_offset;
            }
            ImportEvent::Prepared(request) => {
                if request.source_sha256.is_none()
                    || request.source_samples.is_none_or(|samples| samples == 0)
                {
                    return Err(ApplicationError::InvalidCommand);
                }
                let Some(accepted) = self.request.as_ref() else {
                    return Err(ApplicationError::InvalidCommand);
                };
                let mut expected = accepted.clone();
                expected.source_sha256 = request.source_sha256;
                expected.source_samples = request.source_samples;
                if expected != request {
                    return Err(ApplicationError::InvalidCommand);
                }
                self.request = Some(request.clone());
                if self.stopping != Some(identity) {
                    self.io
                        .submit(ImportEffect::StartWorker(request))
                        .map_err(|e| self.fail(e))?;
                    self.view.active_job = Some((job_id, JobState::Preparing));
                    self.view.message = Some("Source vérifiée ; préparation du moteur…".into());
                }
            }
            ImportEvent::Ready { backend, .. } => {
                if self.direct_live {
                    self.view.direct_engine_phase = Some(DirectEnginePhase::Ready);
                    self.view.direct_ready_at_unix_ms = Some(
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map_or(0, |elapsed| elapsed.as_millis() as u64),
                    );
                }
                if self.stopping != Some(identity) {
                    self.view.active_job = Some((
                        job_id,
                        if self.request.is_none() {
                            JobState::Capturing
                        } else {
                            JobState::Running
                        },
                    ));
                    self.view.message = Some(if self.direct_live {
                        format!("Capture et transcription opérationnelles — moteur : {backend}")
                    } else {
                        format!("Moteur actif : {backend}")
                    });
                }
            }
            ImportEvent::Progress {
                completed_samples,
                total_samples,
                ..
            } => {
                let source_samples = self
                    .request
                    .as_ref()
                    .and_then(|request| request.source_samples);
                if source_samples.is_none_or(|limit| completed_samples > limit)
                    || (total_samples > 0
                        && (completed_samples != total_samples
                            || Some(total_samples) != source_samples))
                {
                    return Err(ApplicationError::InvalidCommand);
                }
                if total_samples > 0 {
                    self.final_source_samples = Some(total_samples);
                }
                self.view.progress = Some((completed_samples, total_samples));
            }
            ImportEvent::Segment(segment) => {
                self.accept_segment(segment, job_id, generation)?;
            }
            ImportEvent::Persisted { sequence, .. } => {
                self.pending_persistence = self.pending_persistence.saturating_sub(1);
                if sequence >= self.next_sequence {
                    return Err(ApplicationError::InvalidCommand);
                }
                if self.request.is_none() {
                    self.view.confirmed_fragment_samples = self
                        .view
                        .confirmed_fragment_samples
                        .max(self.max_segment_offset);
                    if let Some(segment) = self.pending_live_segments.remove(&sequence) {
                        self.view.confirmed_text = Some(segment.text);
                        if self.view.provisional_sequence == Some(sequence) {
                            self.view.provisional_text = None;
                            self.view.provisional_sequence = None;
                            self.view.provisional_range = None;
                            self.view.provisional_captured_at_unix_ms = None;
                            self.view.provisional_available_at_unix_ms = None;
                        }
                    }
                }
                self.maybe_publish(job_id, generation)?;
            }
            ImportEvent::End {
                last_sequence,
                last_offset,
                ..
            } => {
                if last_sequence != self.next_sequence.saturating_sub(1)
                    || last_offset == 0
                    || self.final_source_samples != Some(last_offset)
                    || self
                        .request
                        .as_ref()
                        .and_then(|request| request.source_samples)
                        != Some(last_offset)
                    || self.max_segment_offset > last_offset
                {
                    return Err(ApplicationError::InvalidCommand);
                }
                self.ended_at = Some(last_sequence);
                self.maybe_publish(job_id, generation)?;
            }
            ImportEvent::Published { .. } => {
                if self.stopping == Some(identity) && self.request.is_some() {
                    self.view.active_job = Some((job_id, JobState::Cancelling));
                } else {
                    self.view.active_job = Some((job_id, JobState::Complete));
                    self.view.last_result = Some("Transcription publiée et vérifiée".into());
                    self.view.message = None;
                    self.active = None;
                    self.stopping = None;
                    self.view.provisional_text = None;
                    self.view.provisional_sequence = None;
                    self.view.provisional_range = None;
                    self.view.provisional_captured_at_unix_ms = None;
                    self.view.provisional_available_at_unix_ms = None;
                }
            }
            ImportEvent::Failed { message, .. } => {
                if self.direct_live {
                    self.view.direct_capture_phase = Some(DirectCapturePhase::Failed);
                    self.view.direct_engine_phase = Some(DirectEnginePhase::Failed);
                }
                if self.pending_enqueues.contains(&identity) {
                    self.pending_enqueues.retain(|pending| *pending != identity);
                    self.view.message = Some(message);
                } else if self.stopping == Some(identity) && self.request.is_some() {
                    self.view.active_job = Some((job_id, JobState::Cancelled));
                    self.view.message =
                        Some(format!("Arrêt confirmé; archive conservée ({message})."));
                    self.active = None;
                    self.stopping = None;
                } else {
                    self.view.active_job = Some((job_id, JobState::Recoverable));
                    self.view.message = Some(message);
                    self.active = None;
                    self.stopping = None;
                    self.direct_live = false;
                }
            }
            ImportEvent::Stopped { .. } => {
                self.view.active_job = Some((job_id, JobState::Cancelled));
                self.view.message =
                    Some("Arrêt confirmé ; la source et l’archive pending sont conservées.".into());
                self.active = None;
                self.stopping = None;
                self.ended_at = None;
                self.pending_persistence = 0;
            }
        }
        Ok(())
    }

    fn maybe_publish(
        &mut self,
        job_id: JobId,
        generation: Generation,
    ) -> Result<(), ApplicationError> {
        if self.pending_persistence == 0
            && let Some(last_sequence) = self.ended_at
        {
            self.io
                .submit(ImportEffect::Publish {
                    job_id,
                    generation,
                    last_sequence,
                })
                .map_err(|e| self.fail(e))?;
            self.ended_at = None;
            self.view.active_job = Some((job_id, JobState::Finalizing));
        }
        Ok(())
    }

    fn accept_segment(
        &mut self,
        segment: crate::ports::WorkerSegment,
        _job_id: crate::domain::JobId,
        _generation: crate::domain::Generation,
    ) -> Result<(), ApplicationError> {
        if segment.segment_id.0 != self.next_sequence
            || segment.range.sample_rate_hz != 16_000
            || segment.range.start_sample >= segment.range.end_sample
            || self
                .request
                .as_ref()
                .and_then(|request| request.source_samples)
                .is_some_and(|limit| segment.range.end_sample > limit)
        {
            return Err(ApplicationError::InvalidCommand);
        }
        self.pending_segments.push_back(segment);
        let _ = self.retry_pending_segment()?;
        Ok(())
    }

    /// Try one bounded persistence submission. The event remains owned by the application
    /// until the adapter confirms that the effect entered its bounded queue.
    fn retry_pending_segment(&mut self) -> Result<bool, ApplicationError> {
        let Some(segment) = self.pending_segments.front() else {
            return Ok(true);
        };
        let next_sequence = self
            .next_sequence
            .checked_add(1)
            .ok_or(ApplicationError::InvalidCommand)?;
        match self
            .io
            .submit(ImportEffect::PersistSegment(segment.clone()))
        {
            Ok(()) => {}
            Err(PortError::Backpressure) => return Ok(false),
            Err(error) => {
                self.pending_segments.clear();
                return Err(self.fail(error));
            }
        }
        let segment = self.pending_segments.pop_front().unwrap();
        self.max_segment_offset = self.max_segment_offset.max(segment.range.end_sample);
        self.next_sequence = next_sequence;
        self.pending_persistence += 1;
        Ok(true)
    }
}

impl<P: ImportIoPort> Application for ImportApplication<P> {
    fn set_gpu_candidate(
        &mut self,
        job_id: JobId,
        generation: Generation,
        candidate: Option<crate::gpu_inventory::GpuRuntimeIdentity>,
    ) -> Result<(), ApplicationError> {
        self.io
            .submit(ImportEffect::SetGpuCandidate {
                job_id,
                generation,
                candidate,
            })
            .map_err(|error| {
                self.view.message =
                    Some(format!("Sélection GPU non transmise au worker : {error:?}"));
                ApplicationError::PortUnavailable
            })
    }

    fn dispatch(&mut self, command: AppCommand) -> Result<AppView, ApplicationError> {
        let admission = self.model_guard.as_ref().and_then(|guard| match &command {
            AppCommand::StartImport { request }
            | AppCommand::EnqueueImport { request }
            | AppCommand::ProcessQueued { request }
            | AppCommand::ResumeInterrupted { request } => {
                Some(guard.admit_import(request, crate::model_selection_app::InferencePath::Import))
            }
            AppCommand::StartLiveConfigured { request }
            | AppCommand::StartDirectLiveConfigured { request } => Some(guard.admit_live(request)),
            _ => None,
        });
        if let Some(Err(error)) = admission {
            let reason = match error {
                crate::model_selection_app::SelectionRefusal::UnknownHistoricalModelPath => {
                    "chemin ou empreinte non reconnu dans le catalogue"
                }
                crate::model_selection_app::SelectionRefusal::Capability(_) => {
                    "combinaison modèle, calcul ou langue non admissible"
                }
            };
            return Ok(self.refuse_model_command(reason));
        }

        match command {
            AppCommand::Refresh => {
                let identity = self.active;
                l3d_timing_line(
                    "refresh",
                    identity.map(|identity| identity.job_id),
                    identity.map(|identity| identity.generation),
                    None,
                );
                self.poll_once()?;
            }
            AppCommand::ScanHistory { destination } => {
                self.io
                    .submit(ImportEffect::ScanHistory {
                        destination: destination.clone(),
                    })
                    .map_err(|e| self.fail(e))?;
                self.io
                    .submit(ImportEffect::ScanQueue { destination })
                    .map_err(|e| self.fail(e))?;
                self.view.history_scan_pending = true;
                self.view.queue_scan_pending = true;
            }
            AppCommand::ResolveLivePublication { request } => {
                if self.active.is_some() || request.destination.trim().is_empty() {
                    return Err(ApplicationError::InvalidCommand);
                }
                self.io
                    .submit(ImportEffect::ResolveLivePublication(request))
                    .map_err(|error| self.fail(error))?;
                self.view.history_scan_pending = true;
                self.view.message = Some(
                    "Vérification du passage récupérable et résolution explicite en cours…".into(),
                );
            }
            AppCommand::RemoveQueued { job_id, generation } => {
                self.io
                    .submit(ImportEffect::RemoveQueued {
                        job_id,
                        generation,
                        destination: self
                            .view
                            .queue
                            .iter()
                            .find(|item| {
                                item.request.job_id == job_id
                                    && item.request.generation == generation
                            })
                            .map(|item| item.request.destination.clone())
                            .ok_or(ApplicationError::InvalidCommand)?,
                    })
                    .map_err(|e| self.fail(e))?;
            }
            command @ (AppCommand::StartLiveConfigured { .. }
            | AppCommand::StartDirectLiveConfigured { .. }) => {
                let direct = matches!(&command, AppCommand::StartDirectLiveConfigured { .. });
                let request = match command {
                    AppCommand::StartLiveConfigured { request }
                    | AppCommand::StartDirectLiveConfigured { request } => request,
                    _ => unreachable!(),
                };
                if self.active.is_some() {
                    return Err(ApplicationError::InvalidCommand);
                }
                let identity = CommandIdentity {
                    job_id: request.job_id,
                    generation: request.generation,
                };
                self.auto_stop_after_speech_samples = request
                    .auto_stop_after_speech_samples
                    .filter(|limit| *limit > 0);
                let effect = if direct {
                    ImportEffect::StartDirectLive(request)
                } else {
                    ImportEffect::StartLive(request)
                };
                self.io.submit(effect).map_err(|e| self.fail(e))?;
                self.request = None;
                self.direct_live = direct;
                self.active = Some(identity);
                self.stopping = None;
                self.next_sequence = 1;
                self.pending_persistence = 0;
                self.pending_live_segments.clear();
                self.max_segment_offset = 0;
                self.ended_at = None;
                self.final_source_samples = None;
                self.view.progress = None;
                self.view.diagnostics_degraded = false;
                self.view.active_job = Some((identity.job_id, JobState::Preparing));
                self.view.active_generation = Some(identity.generation);
                self.view.captured_samples = 0;
                self.view.admitted_samples = 0;
                self.view.audio_durable_samples = 0;
                self.view.confirmed_fragment_samples = 0;
                self.view.confirmed_samples = 0;
                self.view.speech_samples = 0;
                self.view.provisional_text = None;
                self.view.provisional_sequence = None;
                self.view.provisional_range = None;
                self.view.provisional_captured_at_unix_ms = None;
                self.view.provisional_available_at_unix_ms = None;
                self.view.confirmed_text = None;
                self.view.direct_first_sample_at_unix_ms = None;
                self.view.direct_first_admitted_at_unix_ms = None;
                self.view.direct_ready_at_unix_ms = None;
                self.view.direct_capture_phase = direct.then_some(DirectCapturePhase::Initializing);
                self.view.direct_engine_phase = direct.then_some(DirectEnginePhase::Preparing);
                self.view.message = Some(if direct {
                    "Microphone en cours d'initialisation".into()
                } else {
                    "Préparation du staging durable et du worker…".into()
                });
            }
            AppCommand::FinishLiveOnCpu { job_id, generation } => {
                let identity = CommandIdentity { job_id, generation };
                if self.active != Some(identity)
                    || self.view.active_job != Some((job_id, JobState::AwaitingChoice))
                {
                    return Err(ApplicationError::InvalidCommand);
                }
                self.io
                    .submit(ImportEffect::FinishLiveOnCpu { job_id, generation })
                    .map_err(|error| self.fail(error))?;
                self.view.active_job = Some((job_id, JobState::Preparing));
                self.view.message = Some(
                    "Choix CPU confirmé ; finalisation du passage conservé sans relancer le micro…"
                        .into(),
                );
            }
            AppCommand::StartImport { request } => {
                if self.active.is_some() {
                    return Err(ApplicationError::InvalidCommand);
                }
                let identity = CommandIdentity {
                    job_id: request.job_id,
                    generation: request.generation,
                };
                self.io
                    .submit(ImportEffect::Prepare(request.clone()))
                    .map_err(|e| self.fail(e))?;
                l3d_timing_line(
                    "start_import",
                    Some(request.job_id),
                    Some(request.generation),
                    None,
                );
                self.request = Some(request);
                self.auto_stop_after_speech_samples = None;
                self.active = Some(identity);
                self.stopping = None;
                self.next_sequence = 1;
                self.pending_persistence = 0;
                self.ended_at = None;
                self.max_segment_offset = 0;
                self.final_source_samples = None;
                self.view.progress = None;
                self.view.diagnostics_degraded = false;
                self.view.active_job = Some((identity.job_id, JobState::Preparing));
                self.view.active_generation = Some(identity.generation);
                self.view.message = Some("Vérification de la source et du modèle…".into());
            }
            AppCommand::EnqueueImport { request } => {
                if self.direct_live && self.active.is_some() {
                    return Err(ApplicationError::InvalidCommand);
                }
                let identity = CommandIdentity {
                    job_id: request.job_id,
                    generation: request.generation,
                };
                self.io
                    .submit(ImportEffect::Enqueue(request))
                    .map_err(|e| self.fail(e))?;
                self.pending_enqueues.push(identity);
                self.view.message =
                    Some("Vérification de la source et ajout à la file durable en cours…".into());
            }
            AppCommand::ProcessQueued { request } => {
                if self.active.is_some() {
                    return Err(ApplicationError::InvalidCommand);
                }
                let identity = CommandIdentity {
                    job_id: request.job_id,
                    generation: request.generation,
                };
                self.io
                    .submit(ImportEffect::ProcessQueued(request.clone()))
                    .map_err(|e| self.fail(e))?;
                self.request = Some(request);
                self.active = Some(identity);
                self.stopping = None;
                self.next_sequence = 1;
                self.pending_persistence = 0;
                self.ended_at = None;
                self.max_segment_offset = 0;
                self.final_source_samples = None;
                self.view.progress = None;
                self.view.diagnostics_degraded = false;
                self.view.active_job = Some((identity.job_id, JobState::Queued));
                self.view.active_generation = Some(identity.generation);
                self.view.message = Some("Vérification du choix explicite et préparation…".into());
            }
            AppCommand::ResumeInterrupted { request } => {
                if self.active.is_some() {
                    return Err(ApplicationError::InvalidCommand);
                }
                let previous = request;
                let mut resumed = previous.clone();
                resumed.generation = previous
                    .generation
                    .next()
                    .ok_or(ApplicationError::InvalidCommand)?;
                let identity = CommandIdentity {
                    job_id: resumed.job_id,
                    generation: resumed.generation,
                };
                self.io
                    .submit(ImportEffect::ResumeInterrupted {
                        previous: Box::new(previous),
                        request: resumed.clone(),
                    })
                    .map_err(|e| self.fail(e))?;
                self.request = Some(resumed);
                self.active = Some(identity);
                self.stopping = None;
                self.next_sequence = 1;
                self.pending_persistence = 0;
                self.ended_at = None;
                self.max_segment_offset = 0;
                self.final_source_samples = None;
                self.view.progress = None;
                self.view.diagnostics_degraded = false;
                self.view.active_job = Some((identity.job_id, JobState::Queued));
                self.view.active_generation = Some(identity.generation);
                self.view.message = Some(
                    "Reprise confirmée : vérification du préfixe durable avant recalcul…".into(),
                );
            }
            AppCommand::Stop { job_id, generation } | AppCommand::Quit { job_id, generation } => {
                if self.active != Some(CommandIdentity { job_id, generation }) {
                    return Err(ApplicationError::InvalidCommand);
                }
                match self.io.submit(ImportEffect::Stop { job_id, generation }) {
                    Ok(()) => {
                        self.stopping = Some(CommandIdentity { job_id, generation });
                    }
                    Err(PortError::Committing) => {
                        self.view.active_job = Some((job_id, JobState::Finalizing));
                        self.view.message = Some(
                            "Publication en cours ; l’arrêt est trop tardif. Attendez sa fin."
                                .into(),
                        );
                        return Ok(self.view.clone());
                    }
                    Err(error) => return Err(self.fail(error)),
                }
                self.view.active_job = Some((
                    job_id,
                    if self.request.is_none() {
                        JobState::Draining
                    } else {
                        JobState::Cancelling
                    },
                ));
                if self.direct_live {
                    self.view.direct_capture_phase = Some(DirectCapturePhase::Stopped);
                }
            }
            _ => return Err(ApplicationError::InvalidCommand),
        }
        Ok(self.view.clone())
    }

    fn view(&self) -> AppView {
        self.view.clone()
    }
}

impl ImportEvent {
    fn identity(&self) -> (JobId, Generation) {
        match self {
            Self::DirectCaptureActive {
                job_id, generation, ..
            } => (*job_id, *generation),
            Self::DirectConfirmed(segment) => (segment.job_id, segment.generation),
            Self::DirectPublished {
                job_id, generation, ..
            } => (*job_id, *generation),
            Self::LiveProvisional { segment, .. } => (segment.job_id, segment.generation),
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
            Self::LiveRetryingOnCpu { job_id, generation } => (*job_id, *generation),
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
            | Self::Failed {
                job_id, generation, ..
            }
            | Self::Stopped { job_id, generation } => (*job_id, *generation),
            Self::ResumeCheckpoint {
                job_id, generation, ..
            } => (*job_id, *generation),
            Self::Prepared(request) => (request.job_id, request.generation),
            Self::History(_) | Self::Queue(_) => (JobId(0), Generation::first()),
            Self::Segment(segment) => (segment.job_id, segment.generation),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApplicationError {
    InvalidCommand,
    PortUnavailable,
    Failed(String),
}

/// Application owns orchestration; concrete infrastructure is supplied by a composition root.
pub trait Application {
    fn dispatch(&mut self, command: AppCommand) -> Result<AppView, ApplicationError>;
    fn view(&self) -> AppView;

    fn set_gpu_candidate(
        &mut self,
        job_id: JobId,
        generation: Generation,
        candidate: Option<crate::gpu_inventory::GpuRuntimeIdentity>,
    ) -> Result<(), ApplicationError> {
        let _ = (job_id, generation, candidate);
        Ok(())
    }
}

pub struct AppFacade<A> {
    application: A,
}

impl<A: Application> AppFacade<A> {
    pub fn new(application: A) -> Self {
        Self { application }
    }

    pub fn dispatch(&mut self, command: AppCommand) -> Result<AppView, ApplicationError> {
        self.application.dispatch(command)
    }

    pub fn view(&self) -> AppView {
        self.application.view()
    }

    pub fn set_gpu_candidate(
        &mut self,
        job_id: JobId,
        generation: Generation,
        candidate: Option<crate::gpu_inventory::GpuRuntimeIdentity>,
    ) -> Result<(), ApplicationError> {
        self.application
            .set_gpu_candidate(job_id, generation, candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ComputeChoice, LanguageChoice};
    use std::collections::VecDeque;

    #[derive(Default)]
    struct ScanTestIo {
        effects: Vec<ImportEffect>,
        events: VecDeque<ImportEvent>,
    }

    impl ImportIoPort for ScanTestIo {
        fn submit(&mut self, effect: ImportEffect) -> Result<(), PortError> {
            self.effects.push(effect);
            Ok(())
        }

        fn poll(&mut self) -> Result<Option<ImportEvent>, PortError> {
            Ok(self.events.pop_front())
        }
    }

    #[test]
    fn destination_scan_stays_pending_until_history_and_queue_are_consumed() {
        let mut app = ImportApplication::new(ScanTestIo::default());
        let view = app
            .dispatch(AppCommand::ScanHistory {
                destination: "archive".into(),
            })
            .unwrap();
        assert!(view.history_scan_pending);
        assert!(view.queue_scan_pending);

        app.io.events.push_back(ImportEvent::History(Vec::new()));
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert!(!view.history_scan_pending);
        assert!(view.queue_scan_pending);

        app.io.events.push_back(ImportEvent::Queue(Vec::new()));
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert!(!view.history_scan_pending);
        assert!(!view.queue_scan_pending);
    }

    #[test]
    fn queue_scan_failure_clears_pending_state_and_is_visible() {
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.dispatch(AppCommand::ScanHistory {
            destination: "missing-or-invalid".into(),
        })
        .unwrap();
        app.io.events.push_back(ImportEvent::History(Vec::new()));
        app.dispatch(AppCommand::Refresh).unwrap();
        app.io.events.push_back(ImportEvent::Failed {
            job_id: JobId(0),
            generation: Generation::first(),
            message: "StorageCorrupt: destination is not a directory".into(),
        });

        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert!(!view.queue_scan_pending);
        assert!(
            view.message
                .as_deref()
                .is_some_and(|message| message.contains("destination is not a directory"))
        );
    }

    #[test]
    fn model_guard_blocks_each_import_and_live_effect_route_before_submission() {
        let generation = Generation::first();
        let config = JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        };
        let import = ImportRequest {
            job_id: JobId(70),
            generation,
            source_path: "source.wav".into(),
            source_sha256: None,
            source_samples: None,
            model_path: "C:/unknown/model.bin".into(),
            model_sha256: [0; 32],
            destination: "archive".into(),
            config: config.clone(),
        };
        let live = LiveRequest {
            job_id: JobId(70),
            generation,
            model_path: import.model_path.clone(),
            model_sha256: [0; 32],
            destination: "archive".into(),
            group_offset_samples: 0,
            auto_stop_after_speech_samples: None,
            microphone: crate::microphone::MicrophoneChoice::WindowsDefault,
            config,
        };
        let snapshot = crate::model_registry::engine_snapshot(
            1,
            |_| crate::model_catalog::ModelAvailability::Ready,
            Vec::new(),
        );
        let commands = [
            AppCommand::StartImport {
                request: import.clone(),
            },
            AppCommand::EnqueueImport {
                request: import.clone(),
            },
            AppCommand::ProcessQueued {
                request: import.clone(),
            },
            AppCommand::ResumeInterrupted { request: import },
            AppCommand::StartLiveConfigured {
                request: live.clone(),
            },
            AppCommand::StartDirectLiveConfigured { request: live },
        ];

        for command in commands {
            let mut app = ImportApplication::new(ScanTestIo::default()).with_model_guard(
                crate::model_selection_app::ModelCommandGuard::new(
                    crate::model_registry::engine_snapshot(
                        1,
                        |_| crate::model_catalog::ModelAvailability::Ready,
                        Vec::new(),
                    ),
                    1,
                    ["C:/known/turbo.bin".to_owned()],
                ),
            );
            let view = app.dispatch(command).unwrap();
            assert!(app.io.effects.is_empty());
            assert!(view.message.as_deref().is_some_and(|message| {
                message.contains("Modèle indisponible") && message.contains("non reconnu")
            }));
        }
        drop(snapshot);
    }

    #[test]
    fn turbo_existing_matrix_dispatches_all_1206_requests_to_effect_submission() {
        use crate::{
            model_catalog::{ModelAvailability, ModelId},
            model_registry::{TURBO_LANGUAGE_INPUTS, engine_snapshot, model},
            model_selection_app::{InferencePath, ModelCommandGuard},
        };

        let model_path = r"C:\Users\Cyril\AppData\Local\Programs\WhisperBuildDeps\models\ggml-large-v3-turbo.bin";
        let mut sha256 = [0; 32];
        let digest = model(ModelId::WhisperLargeV3Turbo).artifact.sha256;
        for (index, byte) in sha256.iter_mut().enumerate() {
            *byte = u8::from_str_radix(&digest[index * 2..index * 2 + 2], 16).unwrap();
        }
        let mut submitted = 0usize;

        for compute in [ComputeChoice::Cpu, ComputeChoice::Gpu, ComputeChoice::Auto] {
            for path in [InferencePath::Import, InferencePath::Live] {
                let mut languages = vec![LanguageChoice::Automatic];
                for (code, english_name) in TURBO_LANGUAGE_INPUTS {
                    languages.push(LanguageChoice::Manual(code.into()));
                    languages.push(LanguageChoice::Manual(english_name.into()));
                }
                for language in languages {
                    let generation = Generation::first();
                    let job_id = JobId(submitted as u128);
                    let config = JobConfig { language, compute };
                    let guard = ModelCommandGuard::new(
                        engine_snapshot(1, |_| ModelAvailability::Ready, Vec::new()),
                        1,
                        [model_path.to_owned()],
                    );
                    let mut app =
                        ImportApplication::new(ScanTestIo::default()).with_model_guard(guard);
                    match path {
                        InferencePath::Import => {
                            app.dispatch(AppCommand::StartImport {
                                request: ImportRequest {
                                    job_id,
                                    generation,
                                    source_path: "source.wav".into(),
                                    source_sha256: None,
                                    source_samples: None,
                                    model_path: model_path.into(),
                                    model_sha256: sha256,
                                    destination: "archive".into(),
                                    config,
                                },
                            })
                            .unwrap();
                            assert!(matches!(
                                app.io.effects.as_slice(),
                                [ImportEffect::Prepare(_)]
                            ));
                        }
                        InferencePath::Live => {
                            app.dispatch(AppCommand::StartLiveConfigured {
                                request: LiveRequest {
                                    job_id,
                                    generation,
                                    model_path: model_path.into(),
                                    model_sha256: sha256,
                                    destination: "archive".into(),
                                    group_offset_samples: 0,
                                    auto_stop_after_speech_samples: None,
                                    microphone: crate::microphone::MicrophoneChoice::WindowsDefault,
                                    config,
                                },
                            })
                            .unwrap();
                            assert!(matches!(
                                app.io.effects.as_slice(),
                                [ImportEffect::StartLive(_)]
                            ));
                        }
                    }
                    submitted += 1;
                }
            }
        }
        assert_eq!(submitted, 1_206);
    }

    #[test]
    fn repeated_start_is_identifiable_and_stale_stop_does_not_target_new_generation() {
        let job_id = JobId(7);
        let current = Generation::first().next().unwrap();
        let stale = Generation::first();
        let config = JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        };
        let start = AppCommand::StartLive {
            job_id,
            generation: current,
            config,
        };
        let repeated_start = start.clone();
        let stale_stop = AppCommand::Stop {
            job_id,
            generation: stale,
        };

        assert_eq!(start, repeated_start);
        assert!(start.targets(job_id, current));
        assert!(!stale_stop.targets(job_id, current));
    }

    #[test]
    fn every_job_lifecycle_command_carries_identity() {
        let job_id = JobId(9);
        let generation = Generation::first();
        let config = JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        };
        let commands = [
            AppCommand::StartImport {
                request: ImportRequest {
                    job_id,
                    generation,
                    source_path: "source.wav".into(),
                    source_sha256: None,
                    source_samples: None,
                    model_path: "model.bin".into(),
                    model_sha256: [0; 32],
                    destination: "archive".into(),
                    config: config.clone(),
                },
            },
            AppCommand::ProcessQueued {
                request: ImportRequest {
                    job_id,
                    generation,
                    source_path: "source.wav".into(),
                    source_sha256: None,
                    source_samples: None,
                    model_path: "model.bin".into(),
                    model_sha256: [0; 32],
                    destination: "archive".into(),
                    config: config.clone(),
                },
            },
            AppCommand::EnqueueImport {
                request: ImportRequest {
                    job_id,
                    generation,
                    source_path: "source.wav".into(),
                    source_sha256: None,
                    source_samples: None,
                    model_path: "model.bin".into(),
                    model_sha256: [0; 32],
                    destination: "archive".into(),
                    config: config.clone(),
                },
            },
            AppCommand::StartLive {
                job_id,
                generation,
                config,
            },
            AppCommand::Stop { job_id, generation },
            AppCommand::Quit { job_id, generation },
        ];
        assert!(commands.iter().all(|command| command.identity().is_some()));
        assert_eq!(AppCommand::Refresh.identity(), None);
    }

    #[test]
    fn live_stop_keeps_generation_until_segments_are_persisted_and_published() {
        let job_id = JobId(10);
        let generation = Generation::first();
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.dispatch(AppCommand::StartLiveConfigured {
            request: LiveRequest {
                job_id,
                generation,
                model_path: "model.bin".into(),
                model_sha256: [1; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            },
        })
        .unwrap();
        app.io.events.push_back(ImportEvent::LiveCounters {
            job_id,
            generation,
            captured_samples: 320,
            admitted_samples: 320,
            audio_durable_samples: 320,
            confirmed_fragment_samples: 0,
            speech_samples: 320,
            diagnostic: None,
        });
        app.dispatch(AppCommand::Refresh).unwrap();
        app.dispatch(AppCommand::Stop { job_id, generation })
            .unwrap();
        assert_eq!(app.active, Some(CommandIdentity { job_id, generation }));

        app.io.events.push_back(ImportEvent::LiveProvisional {
            segment: crate::ports::WorkerSegment {
                job_id,
                generation,
                instance_id: 1,
                segment_id: crate::domain::SegmentId(1),
                range: crate::domain::SourceRange::new(0, 320, 16_000).unwrap(),
                text: "bonjour".into(),
            },
            captured_at_unix_ms: 1,
            available_at_unix_ms: 2,
        });
        app.dispatch(AppCommand::Refresh).unwrap();
        assert!(matches!(
            app.io.effects.last(),
            Some(ImportEffect::PersistSegment(_))
        ));
        app.io.events.push_back(ImportEvent::Persisted {
            job_id,
            generation,
            sequence: 1,
        });
        app.dispatch(AppCommand::Refresh).unwrap();
        app.io
            .events
            .push_back(ImportEvent::Published { job_id, generation });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.active_job, Some((job_id, JobState::Complete)));
        assert!(
            view.last_result
                .as_deref()
                .is_some_and(|text| text.contains("vérifiée"))
        );
    }

    #[test]
    fn final_stop_counters_allow_last_window_and_match_published_view() {
        let job_id = JobId(20);
        let generation = Generation::first();
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.dispatch(AppCommand::StartLiveConfigured {
            request: LiveRequest {
                job_id,
                generation,
                model_path: "model.bin".into(),
                model_sha256: [1; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            },
        })
        .unwrap();
        app.io.events.push_back(ImportEvent::LiveCounters {
            job_id,
            generation,
            captured_samples: 248_640,
            admitted_samples: 248_640,
            audio_durable_samples: 248_640,
            confirmed_fragment_samples: 160_000,
            speech_samples: 0,
            diagnostic: None,
        });
        app.dispatch(AppCommand::Refresh).unwrap();
        app.dispatch(AppCommand::Stop { job_id, generation })
            .unwrap();
        app.io.events.push_back(ImportEvent::LiveCounters {
            job_id,
            generation,
            captured_samples: 254_240,
            admitted_samples: 254_240,
            audio_durable_samples: 254_240,
            confirmed_fragment_samples: 160_000,
            speech_samples: 0,
            diagnostic: None,
        });
        app.io.events.push_back(ImportEvent::WindowFinished {
            job_id,
            generation,
            window_sequence: 4,
            confirmed_samples: 254_240,
        });
        app.io
            .events
            .push_back(ImportEvent::Published { job_id, generation });
        app.dispatch(AppCommand::Refresh).unwrap();
        app.dispatch(AppCommand::Refresh).unwrap();
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.active_job, Some((job_id, JobState::Complete)));
        assert_eq!(view.captured_samples, 254_240);
        assert_eq!(view.admitted_samples, 254_240);
        assert_eq!(view.audio_durable_samples, 254_240);
        assert_eq!(view.confirmed_samples, 254_240);
    }

    #[test]
    fn live_state_advances_through_capture_drain_and_finalization() {
        let job_id = JobId(19);
        let generation = Generation::first();
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.dispatch(AppCommand::StartLiveConfigured {
            request: LiveRequest {
                job_id,
                generation,
                model_path: "model.bin".into(),
                model_sha256: [9; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            },
        })
        .unwrap();
        assert_eq!(app.view.active_job, Some((job_id, JobState::Preparing)));
        app.io.events.push_back(ImportEvent::Ready {
            job_id,
            generation,
            backend: "CPU".into(),
        });
        assert_eq!(
            app.dispatch(AppCommand::Refresh).unwrap().active_job,
            Some((job_id, JobState::Capturing))
        );
        assert_eq!(
            app.dispatch(AppCommand::Stop { job_id, generation })
                .unwrap()
                .active_job,
            Some((job_id, JobState::Draining))
        );
        app.io
            .events
            .push_back(ImportEvent::LiveFinalizing { job_id, generation });
        assert_eq!(
            app.dispatch(AppCommand::Refresh).unwrap().active_job,
            Some((job_id, JobState::Finalizing))
        );
        app.io
            .events
            .push_back(ImportEvent::Published { job_id, generation });
        assert_eq!(
            app.dispatch(AppCommand::Refresh).unwrap().active_job,
            Some((job_id, JobState::Complete))
        );
    }

    #[test]
    fn immediate_live_stop_does_not_reenter_capturing_on_late_ready() {
        let job_id = JobId(23);
        let generation = Generation::first();
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.dispatch(AppCommand::StartLiveConfigured {
            request: LiveRequest {
                job_id,
                generation,
                model_path: "model.bin".into(),
                model_sha256: [11; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            },
        })
        .unwrap();
        app.dispatch(AppCommand::Stop { job_id, generation })
            .unwrap();
        app.io.events.push_back(ImportEvent::Ready {
            job_id,
            generation,
            backend: "CPU".into(),
        });
        assert_eq!(
            app.dispatch(AppCommand::Refresh).unwrap().active_job,
            Some((job_id, JobState::Draining))
        );
        app.io
            .events
            .push_back(ImportEvent::Stopped { job_id, generation });
        assert_eq!(
            app.dispatch(AppCommand::Refresh).unwrap().active_job,
            Some((job_id, JobState::Cancelled))
        );
    }

    #[test]
    fn live_auto_stop_acts_only_when_a_positive_speech_limit_is_configured() {
        for (limit, should_stop) in [(Some(640), true), (None, false), (Some(0), false)] {
            let job_id = JobId(11);
            let generation = Generation::first();
            let mut app = ImportApplication::new(ScanTestIo::default());
            app.dispatch(AppCommand::StartLiveConfigured {
                request: LiveRequest {
                    job_id,
                    generation,
                    model_path: "model.bin".into(),
                    model_sha256: [2; 32],
                    destination: "archive".into(),
                    group_offset_samples: 0,
                    auto_stop_after_speech_samples: limit,
                    microphone: Default::default(),
                    config: JobConfig {
                        language: LanguageChoice::Automatic,
                        compute: ComputeChoice::Cpu,
                    },
                },
            })
            .unwrap();
            app.io.events.push_back(ImportEvent::LiveCounters {
                job_id,
                generation,
                captured_samples: 960,
                admitted_samples: 960,
                audio_durable_samples: 640,
                confirmed_fragment_samples: 0,
                speech_samples: 640,
                diagnostic: None,
            });
            let view = app.dispatch(AppCommand::Refresh).unwrap();
            assert_eq!(
                app.io
                    .effects
                    .iter()
                    .any(|effect| matches!(effect, ImportEffect::Stop { .. })),
                should_stop,
                "limit {limit:?}"
            );
            assert_eq!(
                view.active_job,
                Some((
                    job_id,
                    if should_stop {
                        JobState::Draining
                    } else {
                        JobState::Preparing
                    }
                ))
            );
        }
    }

    #[test]
    fn live_provisional_text_is_exposed_and_persisted_by_the_application() {
        let job_id = JobId(12);
        let generation = Generation::first();
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.dispatch(AppCommand::StartLiveConfigured {
            request: LiveRequest {
                job_id,
                generation,
                model_path: "model.bin".into(),
                model_sha256: [3; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            },
        })
        .unwrap();
        let segment = crate::ports::WorkerSegment {
            job_id,
            generation,
            instance_id: 1,
            segment_id: crate::domain::SegmentId(1),
            range: crate::domain::SourceRange::new(0, 320, 16_000).unwrap(),
            text: "bonjour provisoire".into(),
        };
        app.io.events.push_back(ImportEvent::LiveProvisional {
            segment,
            captured_at_unix_ms: 1_000,
            available_at_unix_ms: 1_020,
        });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.provisional_text.as_deref(), Some("bonjour provisoire"));
        assert_eq!(view.provisional_sequence, Some(1));
        assert_eq!(view.provisional_captured_at_unix_ms, Some(1_000));
        assert_eq!(view.provisional_available_at_unix_ms, Some(1_020));
        assert!(matches!(
            app.io.effects.last(),
            Some(ImportEffect::PersistSegment(_))
        ));
        app.io.events.push_back(ImportEvent::Persisted {
            job_id,
            generation,
            sequence: 1,
        });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.provisional_text, None);
        assert_eq!(view.confirmed_text.as_deref(), Some("bonjour provisoire"));
    }

    #[test]
    fn import_can_be_enqueued_during_live_without_replacing_the_live_job() {
        let job_id = JobId(13);
        let generation = Generation::first();
        let config = JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        };
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.dispatch(AppCommand::StartLiveConfigured {
            request: LiveRequest {
                job_id,
                generation,
                model_path: "model.bin".into(),
                model_sha256: [4; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                config: config.clone(),
            },
        })
        .unwrap();
        let import = ImportRequest {
            job_id: JobId(14),
            generation,
            source_path: "later.wav".into(),
            source_sha256: None,
            source_samples: None,
            model_path: "model.bin".into(),
            model_sha256: [4; 32],
            destination: "archive".into(),
            config,
        };
        app.dispatch(AppCommand::EnqueueImport { request: import })
            .unwrap();
        assert_eq!(app.active, Some(CommandIdentity { job_id, generation }));
        assert!(matches!(
            app.io.effects.last(),
            Some(ImportEffect::Enqueue(_))
        ));
        assert_eq!(app.view.active_job, Some((job_id, JobState::Preparing)));
    }

    #[test]
    fn queued_import_starts_only_after_live_publication_and_explicit_processing() {
        let live_id = JobId(16);
        let generation = Generation::first();
        let config = JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Cpu,
        };
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.dispatch(AppCommand::StartLiveConfigured {
            request: LiveRequest {
                job_id: live_id,
                generation,
                model_path: "model.bin".into(),
                model_sha256: [6; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                config: config.clone(),
            },
        })
        .unwrap();
        let queued = ImportRequest {
            job_id: JobId(17),
            generation,
            source_path: "later.wav".into(),
            source_sha256: None,
            source_samples: None,
            model_path: "model.bin".into(),
            model_sha256: [6; 32],
            destination: "archive".into(),
            config,
        };
        app.dispatch(AppCommand::EnqueueImport {
            request: queued.clone(),
        })
        .unwrap();
        app.dispatch(AppCommand::Stop {
            job_id: live_id,
            generation,
        })
        .unwrap();
        assert!(
            matches!(app.io.effects.last(), Some(ImportEffect::Stop { job_id, .. }) if *job_id == live_id)
        );
        assert_eq!(
            app.active,
            Some(CommandIdentity {
                job_id: live_id,
                generation
            })
        );

        app.io.events.push_back(ImportEvent::Published {
            job_id: live_id,
            generation,
        });
        app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(app.active, None);
        app.dispatch(AppCommand::ProcessQueued { request: queued })
            .unwrap();
        assert_eq!(
            app.active,
            Some(CommandIdentity {
                job_id: JobId(17),
                generation
            })
        );
        assert!(
            matches!(app.io.effects.last(), Some(ImportEffect::ProcessQueued(request)) if request.job_id == JobId(17))
        );
    }

    #[test]
    fn stopped_import_ignores_late_segments_and_remains_cancelled() {
        let job_id = JobId(15);
        let generation = Generation::first();
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.dispatch(AppCommand::StartImport {
            request: ImportRequest {
                job_id,
                generation,
                source_path: "source.wav".into(),
                source_sha256: None,
                source_samples: None,
                model_path: "model.bin".into(),
                model_sha256: [5; 32],
                destination: "archive".into(),
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            },
        })
        .unwrap();
        app.dispatch(AppCommand::Stop { job_id, generation })
            .unwrap();
        let effects_before_late_segment = app.io.effects.len();
        app.io
            .events
            .push_back(ImportEvent::Segment(crate::ports::WorkerSegment {
                job_id,
                generation,
                instance_id: 1,
                segment_id: crate::domain::SegmentId(1),
                range: crate::domain::SourceRange::new(0, 320, 16_000).unwrap(),
                text: "late".into(),
            }));
        app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(app.io.effects.len(), effects_before_late_segment);
        app.io
            .events
            .push_back(ImportEvent::Stopped { job_id, generation });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.active_job, Some((job_id, JobState::Cancelled)));
    }

    #[test]
    fn direct_capture_and_engine_progress_independently_through_pre_ready_stop() {
        let job_id = JobId(19);
        let generation = Generation::first();
        let mut app = ImportApplication::new(ScanTestIo::default());
        let request = LiveRequest {
            job_id,
            generation,
            model_path: "model.bin".into(),
            model_sha256: [9; 32],
            destination: "archive".into(),
            group_offset_samples: 0,
            auto_stop_after_speech_samples: None,
            microphone: Default::default(),
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Cpu,
            },
        };
        let view = app
            .dispatch(AppCommand::StartDirectLiveConfigured { request })
            .unwrap();
        assert_eq!(
            view.direct_capture_phase,
            Some(DirectCapturePhase::Initializing)
        );
        assert_eq!(view.direct_engine_phase, Some(DirectEnginePhase::Preparing));
        app.io.events.push_back(ImportEvent::DirectCaptureActive {
            job_id,
            generation,
            first_sample_at_unix_ms: 100,
            first_admitted_at_unix_ms: 110,
        });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.direct_capture_phase, Some(DirectCapturePhase::Active));
        assert_eq!(view.direct_engine_phase, Some(DirectEnginePhase::Preparing));
        let view = app
            .dispatch(AppCommand::Stop { job_id, generation })
            .unwrap();
        assert_eq!(view.direct_capture_phase, Some(DirectCapturePhase::Stopped));
        assert_eq!(view.direct_engine_phase, Some(DirectEnginePhase::Preparing));
        app.io.events.push_back(ImportEvent::DirectCaptureActive {
            job_id,
            generation,
            first_sample_at_unix_ms: 100,
            first_admitted_at_unix_ms: 110,
        });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.direct_capture_phase, Some(DirectCapturePhase::Stopped));
        assert_eq!(view.direct_first_sample_at_unix_ms, Some(100));
        assert_eq!(view.direct_first_admitted_at_unix_ms, Some(110));
        app.io.events.push_back(ImportEvent::Ready {
            job_id,
            generation,
            backend: "CPU".into(),
        });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.active_job, Some((job_id, JobState::Draining)));
        assert_eq!(view.direct_engine_phase, Some(DirectEnginePhase::Ready));
        app.io.events.push_back(ImportEvent::DirectPublished {
            job_id,
            generation,
            txt_path: "a.txt".into(),
            srt_path: "a.srt".into(),
            wav_path: "a.wav".into(),
        });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.active_job, Some((job_id, JobState::Complete)));
        assert_eq!(view.direct_engine_phase, Some(DirectEnginePhase::Released));
    }

    #[test]
    fn live_capture_failure_remains_recoverable_with_its_diagnostic() {
        let job_id = JobId(16);
        let generation = Generation::first();
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.dispatch(AppCommand::StartLiveConfigured {
            request: LiveRequest {
                job_id,
                generation,
                model_path: "model.bin".into(),
                model_sha256: [6; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            },
        })
        .unwrap();
        let diagnostic = "CaptureSaturated: Stop retained while worker data channel is full";
        app.io.events.push_back(ImportEvent::Failed {
            job_id,
            generation,
            message: diagnostic.into(),
        });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.active_job, Some((job_id, JobState::Recoverable)));
        assert_eq!(view.message.as_deref(), Some(diagnostic));
    }

    #[test]
    fn failed_live_stop_retains_recoverable_diagnostic_and_confirmed_fragment() {
        let job_id = JobId(20);
        let generation = Generation::first();
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.dispatch(AppCommand::StartLiveConfigured {
            request: LiveRequest {
                job_id,
                generation,
                model_path: "model.bin".into(),
                model_sha256: [10; 32],
                destination: "archive".into(),
                group_offset_samples: 0,
                auto_stop_after_speech_samples: None,
                microphone: Default::default(),
                config: JobConfig {
                    language: LanguageChoice::Automatic,
                    compute: ComputeChoice::Cpu,
                },
            },
        })
        .unwrap();
        app.io.events.push_back(ImportEvent::LiveProvisional {
            segment: crate::ports::WorkerSegment {
                job_id,
                generation,
                instance_id: 1,
                segment_id: crate::domain::SegmentId(1),
                range: crate::domain::SourceRange::new(0, 320, 16_000).unwrap(),
                text: "confirmé".into(),
            },
            captured_at_unix_ms: 1,
            available_at_unix_ms: 2,
        });
        app.dispatch(AppCommand::Refresh).unwrap();
        app.io.events.push_back(ImportEvent::Persisted {
            job_id,
            generation,
            sequence: 1,
        });
        app.dispatch(AppCommand::Refresh).unwrap();
        app.dispatch(AppCommand::Stop { job_id, generation })
            .unwrap();
        let diagnostic = "StorageUnavailable: final audio sync failed";
        app.io.events.push_back(ImportEvent::Failed {
            job_id,
            generation,
            message: diagnostic.into(),
        });
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        assert_eq!(view.active_job, Some((job_id, JobState::Recoverable)));
        assert_eq!(view.message.as_deref(), Some(diagnostic));
        assert_eq!(view.confirmed_text.as_deref(), Some("confirmé"));
        assert_eq!(view.confirmed_fragment_samples, 320);
    }
}
