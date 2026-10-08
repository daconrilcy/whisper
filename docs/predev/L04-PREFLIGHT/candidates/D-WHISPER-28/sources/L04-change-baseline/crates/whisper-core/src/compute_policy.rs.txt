use crate::domain::ComputeChoice;

/// Backend selected by policy before a worker is started.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackendAttempt {
    Cpu,
    Gpu,
}

/// A backend choice and the action permitted after its failure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BackendDecision {
    Start(BackendAttempt),
    RetryCpu,
    AwaitUserChoice,
    Fail,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComputePolicyError {
    CpuUnavailable,
    GpuUnavailable,
    InvalidFallback,
}

/// Keeps requested compute mode separate from the backend actually attested by a worker.
pub struct ComputePolicy;

impl ComputePolicy {
    pub fn initial(
        choice: ComputeChoice,
        gpu_available: bool,
    ) -> Result<BackendAttempt, ComputePolicyError> {
        match choice {
            ComputeChoice::Cpu => Ok(BackendAttempt::Cpu),
            ComputeChoice::Auto if gpu_available => Ok(BackendAttempt::Gpu),
            ComputeChoice::Auto => Ok(BackendAttempt::Cpu),
            ComputeChoice::Gpu if gpu_available => Ok(BackendAttempt::Gpu),
            ComputeChoice::Gpu => Err(ComputePolicyError::GpuUnavailable),
        }
    }

    /// Decide recovery only after a backend failure has been established.
    pub fn after_failure(choice: ComputeChoice, attempted: BackendAttempt) -> BackendDecision {
        match (choice, attempted) {
            (ComputeChoice::Auto, BackendAttempt::Gpu) => BackendDecision::RetryCpu,
            (ComputeChoice::Gpu, BackendAttempt::Gpu) => BackendDecision::AwaitUserChoice,
            (ComputeChoice::Cpu, BackendAttempt::Cpu) => BackendDecision::Fail,
            _ => BackendDecision::Fail,
        }
    }

    pub fn validate_fallback(
        choice: ComputeChoice,
        from: BackendAttempt,
        to: BackendAttempt,
    ) -> Result<(), ComputePolicyError> {
        match (choice, from, to) {
            (ComputeChoice::Auto, BackendAttempt::Gpu, BackendAttempt::Cpu) => Ok(()),
            _ => Err(ComputePolicyError::InvalidFallback),
        }
    }
}
