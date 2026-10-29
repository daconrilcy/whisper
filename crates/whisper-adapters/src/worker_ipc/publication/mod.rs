use std::sync::atomic::{AtomicU8, Ordering};
use whisper_core::ports::PortError;

const GATE_IDLE: u8 = 0;
pub(super) const GATE_ACTIVE: u8 = 1;
pub(super) const GATE_CANCELLED: u8 = 2;
const GATE_COMMITTING: u8 = 3;
pub(super) const GATE_COMMITTED: u8 = 4;

#[derive(Default)]
pub(super) struct PublicationGate(pub(super) AtomicU8);

impl PublicationGate {
    pub(super) fn activate(&self) {
        self.0.store(GATE_ACTIVE, Ordering::Release);
    }

    pub(super) fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire) == GATE_CANCELLED
    }

    pub(super) fn request_stop(&self) -> Result<bool, PortError> {
        match self.0.compare_exchange(
            GATE_ACTIVE,
            GATE_CANCELLED,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => Ok(true),
            Err(GATE_CANCELLED) => Ok(false),
            Err(GATE_COMMITTING | GATE_COMMITTED) => Err(PortError::Committing),
            Err(GATE_IDLE) => Err(PortError::Unavailable),
            Err(_) => Err(PortError::Unavailable),
        }
    }

    pub(super) fn rollback_stop(&self) {
        let _ = self.0.compare_exchange(
            GATE_CANCELLED,
            GATE_ACTIVE,
            Ordering::AcqRel,
            Ordering::Acquire,
        );
    }

    pub(super) fn commit<T>(
        &self,
        commit: impl FnOnce() -> Result<T, String>,
    ) -> Result<T, String> {
        match self.0.compare_exchange(
            GATE_ACTIVE,
            GATE_COMMITTING,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            Ok(_) => {
                let result = commit();
                if result.is_ok() {
                    self.0.store(GATE_COMMITTED, Ordering::Release);
                }
                result
            }
            Err(GATE_CANCELLED) => Err("Cancelled: publication invalidated by Stop".into()),
            Err(GATE_COMMITTING | GATE_COMMITTED) => {
                Err("Publication: commit is already in progress".into())
            }
            Err(_) => Err("Publication: generation is not active".into()),
        }
    }
}

#[cfg(test)]
mod tests;
