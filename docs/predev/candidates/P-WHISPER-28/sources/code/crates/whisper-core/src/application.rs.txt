use crate::domain::{Generation, JobConfig, JobId, JobState};
use crate::ports::{
    ArchiveHistoryItem, ImportEffect, ImportEvent, ImportIoPort, ImportRequest, PortError,
};

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
    StartLive {
        job_id: JobId,
        generation: Generation,
        config: JobConfig,
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
}

impl AppCommand {
    pub fn identity(&self) -> Option<CommandIdentity> {
        match self {
            Self::Refresh => None,
            Self::StartLive {
                job_id, generation, ..
            }
            | Self::Stop { job_id, generation }
            | Self::Quit { job_id, generation } => Some(CommandIdentity {
                job_id: *job_id,
                generation: *generation,
            }),
            Self::StartImport { request } => Some(CommandIdentity {
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
    pub queued_jobs: usize,
    pub message: Option<String>,
    pub progress: Option<(u64, u64)>,
    pub last_result: Option<String>,
    pub history: Vec<ArchiveHistoryItem>,
}

/// Owns admission and lifecycle for the first WAV import. Adapters only execute effects.
pub struct ImportApplication<P> {
    io: P,
    view: AppView,
    active: Option<CommandIdentity>,
    request: Option<ImportRequest>,
    next_sequence: u64,
    pending_persistence: usize,
    ended_at: Option<u64>,
    stopping: Option<CommandIdentity>,
    max_segment_offset: u64,
    final_source_samples: Option<u64>,
}

impl<P: ImportIoPort> ImportApplication<P> {
    pub fn new(io: P) -> Self {
        Self {
            io,
            view: AppView::default(),
            active: None,
            request: None,
            next_sequence: 0,
            pending_persistence: 0,
            ended_at: None,
            stopping: None,
            max_segment_offset: 0,
            final_source_samples: None,
        }
    }

    fn fail(&mut self, error: PortError) -> ApplicationError {
        self.view.message = Some(format!("Import indisponible : {error:?}"));
        ApplicationError::PortUnavailable
    }

    fn poll_once(&mut self) -> Result<(), ApplicationError> {
        let Some(event) = self.io.poll().map_err(|e| self.fail(e))? else {
            return Ok(());
        };
        if let ImportEvent::History(history) = event {
            self.view.history = history;
            return Ok(());
        }
        let (job_id, generation) = event.identity();
        let identity = CommandIdentity { job_id, generation };
        if self.active != Some(identity)
            && !(matches!(
                event,
                ImportEvent::Stopped { .. } | ImportEvent::Failed { .. }
            ) && self.stopping == Some(identity))
        {
            return Ok(());
        }
        match event {
            ImportEvent::History(_) => {
                unreachable!("history events are handled before job admission")
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
                self.io
                    .submit(ImportEffect::StartWorker(request))
                    .map_err(|e| self.fail(e))?;
                self.view.message = Some("Source vérifiée ; démarrage du moteur CPU…".into());
            }
            ImportEvent::Ready { backend, .. } => {
                self.view.active_job = Some((job_id, JobState::Running));
                self.view.message = Some(format!("Moteur actif : {backend}"));
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
                if segment.segment_id.0 != self.next_sequence
                    || segment.range.sample_rate_hz != 16_000
                    || segment.range.start_sample >= segment.range.end_sample
                    || self
                        .request
                        .as_ref()
                        .and_then(|request| request.source_samples)
                        .is_none_or(|limit| segment.range.end_sample > limit)
                {
                    return Err(ApplicationError::InvalidCommand);
                }
                self.max_segment_offset = self.max_segment_offset.max(segment.range.end_sample);
                self.next_sequence = self
                    .next_sequence
                    .checked_add(1)
                    .ok_or(ApplicationError::InvalidCommand)?;
                self.io
                    .submit(ImportEffect::PersistSegment(segment))
                    .map_err(|e| self.fail(e))?;
                self.pending_persistence += 1;
            }
            ImportEvent::Persisted { sequence, .. } => {
                self.pending_persistence = self.pending_persistence.saturating_sub(1);
                if sequence >= self.next_sequence {
                    return Err(ApplicationError::InvalidCommand);
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
                self.view.active_job = Some((job_id, JobState::Complete));
                self.view.last_result = Some("Transcription publiée et vérifiée".into());
                self.view.message = None;
                self.active = None;
            }
            ImportEvent::Failed { message, .. } => {
                if self.stopping == Some(identity) {
                    self.view.active_job = Some((job_id, JobState::Cancelled));
                    self.view.message =
                        Some(format!("Arrêt confirmé; archive conservée ({message})."));
                    self.active = None;
                    self.stopping = None;
                } else {
                    self.view.active_job = Some((job_id, JobState::Recoverable));
                    self.view.message = Some(message);
                    self.active = None;
                }
            }
            ImportEvent::Stopped { .. } => {
                self.view.active_job = Some((job_id, JobState::Cancelled));
                self.view.message =
                    Some("Import arrêté ; la source et l’archive pending sont conservées.".into());
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
}

impl<P: ImportIoPort> Application for ImportApplication<P> {
    fn dispatch(&mut self, command: AppCommand) -> Result<AppView, ApplicationError> {
        match command {
            AppCommand::Refresh => self.poll_once()?,
            AppCommand::ScanHistory { destination } => {
                self.io
                    .submit(ImportEffect::ScanHistory { destination })
                    .map_err(|e| self.fail(e))?;
            }
            AppCommand::StartImport { request } => {
                if self.active.is_some() {
                    return Err(ApplicationError::InvalidCommand);
                }
                let identity = CommandIdentity {
                    job_id: request.job_id,
                    generation: request.generation,
                };
                let snapshot = request.clone();
                self.io
                    .submit(ImportEffect::Prepare(request))
                    .map_err(|e| self.fail(e))?;
                self.request = Some(snapshot);
                self.active = Some(identity);
                self.stopping = None;
                self.next_sequence = 1;
                self.pending_persistence = 0;
                self.ended_at = None;
                self.max_segment_offset = 0;
                self.final_source_samples = None;
                self.view.progress = None;
                self.view.active_job = Some((identity.job_id, JobState::Preparing));
                self.view.message = Some("Vérification de la source et du modèle…".into());
            }
            AppCommand::Stop { job_id, generation } => {
                if self.active != Some(CommandIdentity { job_id, generation }) {
                    return Err(ApplicationError::InvalidCommand);
                }
                let next = generation.next().ok_or(ApplicationError::InvalidCommand)?;
                match self.io.submit(ImportEffect::Stop { job_id, generation }) {
                    Ok(()) => {
                        self.stopping = Some(CommandIdentity { job_id, generation });
                        self.active = Some(CommandIdentity {
                            job_id,
                            generation: next,
                        });
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
                self.view.active_job = Some((job_id, JobState::Cancelling));
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
            Self::Prepared(request) => (request.job_id, request.generation),
            Self::History(_) => (JobId(0), Generation::first()),
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ComputeChoice, LanguageChoice};

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
}
