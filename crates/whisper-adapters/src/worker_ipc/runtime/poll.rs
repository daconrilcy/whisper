use super::*;

mod active_live_stop;
mod capture;
mod collection;
mod live_windows;
mod observations;
mod parakeet;
mod readiness;
mod segments;
mod stop_recovery;
mod terminal_routing;
#[cfg(test)]
mod tests;
pub(in crate::worker_ipc) use parakeet::ParakeetVadRun;
#[cfg(test)]
pub(in crate::worker_ipc) use parakeet::{
    ParakeetStoppedExit, TDT_FRAME_OVERLAP_SAMPLES, append_single_pass_parakeet_word_group,
    parakeet_stopped_exit, parakeet_vad_segments, vad_word_overlap_within_one_tdt_frame,
    validate_single_pass_parakeet_words,
};

pub(in crate::worker_ipc) fn take_ready_live_stop(
    pending_persistence: usize,
    deferred: &mut Option<WorkerEvent>,
) -> Option<WorkerEvent> {
    (pending_persistence == 0)
        .then(|| deferred.take())
        .flatten()
}

pub(in crate::worker_ipc) fn bind_unidentified_startup_failure(
    event: WorkerEvent,
    attempt: (JobId, Generation, u64),
    startup_ready: bool,
) -> WorkerEvent {
    if startup_ready {
        return event;
    }
    match event {
        WorkerEvent::Failed {
            job_id: None,
            generation: None,
            instance_id: None,
            code,
            message,
        } => WorkerEvent::Failed {
            job_id: Some(attempt.0),
            generation: Some(attempt.1),
            instance_id: Some(attempt.2),
            code,
            message,
        },
        event => event,
    }
}

pub(in crate::worker_ipc) fn worker_event_identity(
    event: &WorkerEvent,
) -> Option<(JobId, Generation, u64)> {
    match event {
        WorkerEvent::Ready {
            job_id,
            generation,
            instance_id,
            ..
        }
        | WorkerEvent::ImportStarted {
            job_id,
            generation,
            instance_id,
        }
        | WorkerEvent::LiveMp3Packet {
            job_id,
            generation,
            instance_id,
            ..
        }
        | WorkerEvent::WindowFinished {
            job_id,
            generation,
            instance_id,
            ..
        }
        | WorkerEvent::Stopped {
            job_id,
            generation,
            instance_id,
        }
        | WorkerEvent::Progress {
            job_id,
            generation,
            instance_id,
            ..
        }
        | WorkerEvent::End {
            job_id,
            generation,
            instance_id,
            ..
        } => Some((*job_id, *generation, *instance_id)),
        WorkerEvent::GpuSessionAttested {
            job_id,
            generation,
            instance_id,
            ..
        } => Some((*job_id, *generation, *instance_id)),
        WorkerEvent::Segment(segment) => {
            Some((segment.job_id, segment.generation, segment.instance_id))
        }
        WorkerEvent::Failed {
            job_id: Some(job_id),
            generation: Some(generation),
            instance_id: Some(instance_id),
            ..
        } => Some((*job_id, *generation, *instance_id)),
        WorkerEvent::Failed { .. }
        | WorkerEvent::DecodedBlock { .. }
        | WorkerEvent::CudaAvailable
        | WorkerEvent::GpuCandidateAttested { .. }
        | WorkerEvent::GpusQualified { .. } => None,
    }
}

pub(in crate::worker_ipc) fn worker_event_is_stale(
    event: &WorkerEvent,
    attempt: (JobId, Generation, u64),
) -> bool {
    worker_event_identity(event).is_some_and(|identity| identity != attempt)
}

pub(in crate::worker_ipc) fn map_attempt_sequence(
    base: u64,
    replay_prefix: u64,
    attempt_sequence: u64,
) -> Option<u64> {
    attempt_sequence
        .checked_sub(replay_prefix)?
        .checked_sub(1)?
        .checked_add(base)
}

impl ImportRuntime {
    #[warn(clippy::too_many_lines, clippy::cognitive_complexity)]
    pub(in crate::worker_ipc) fn poll(
        &mut self,
    ) -> Option<Result<ImportEvent, (JobId, Generation, String)>> {
        let event = match self.collect_poll_event() {
            collection::PollCollectionControl::Next => return None,
            collection::PollCollectionControl::Return(output) => return output,
            collection::PollCollectionControl::Finish(event) => event,
        };
        self.route_collected_event(event)
    }
}
