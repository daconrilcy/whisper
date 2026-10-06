use crate::domain::{Generation, JobConfig, JobId, JobState};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandIdentity {
    pub job_id: JobId,
    pub generation: Generation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AppCommand {
    Refresh,
    StartImport {
        job_id: JobId,
        generation: Generation,
        source_id: String,
        config: JobConfig,
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
}

impl AppCommand {
    pub fn identity(&self) -> Option<CommandIdentity> {
        match self {
            Self::Refresh => None,
            Self::StartImport {
                job_id, generation, ..
            }
            | Self::StartLive {
                job_id, generation, ..
            }
            | Self::Stop { job_id, generation }
            | Self::Quit { job_id, generation } => Some(CommandIdentity {
                job_id: *job_id,
                generation: *generation,
            }),
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
                job_id,
                generation,
                source_id: "source-1".into(),
                config: config.clone(),
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
