use crate::{
    archive::{ArchiveEntry, ArchiveStore},
    queue_store::QueueStore,
};
use std::path::Path;
use whisper_core::ports::{ImportQueueEntry as QueueEntry, ImportQueueStatus as QueueStatus};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoverySnapshot {
    pub queue: Vec<QueueEntry>,
    pub archives: Vec<ArchiveEntry>,
}

/// Restores state for display and explicit user choice; it never starts a worker.
pub fn scan(destination: &Path) -> Result<RecoverySnapshot, String> {
    let mut queue = QueueStore::open(destination)?;
    let archives = ArchiveStore::scan(destination)?;
    for entry in queue.entries() {
        if archives.iter().any(|archive| {
            archive.complete
                && archive.job_id == entry.request.job_id
                && archive.generation == entry.request.generation
        }) {
            queue.remove(entry.request.job_id, entry.request.generation)?;
            continue;
        }
        if entry.status == QueueStatus::Running {
            queue.set_status(
                entry.request.job_id,
                entry.request.generation,
                QueueStatus::Interrupted,
            )?;
        } else if entry.status == QueueStatus::Queued {
            queue.set_status(
                entry.request.job_id,
                entry.request.generation,
                QueueStatus::AwaitingChoice,
            )?;
        }
    }
    Ok(RecoverySnapshot {
        queue: queue.entries(),
        archives,
    })
}
