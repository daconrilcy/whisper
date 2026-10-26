//! Bounded, per-backend arbitration for unified Parakeet sessions.

use std::collections::HashMap;

use crate::{Generation, JobId};

const MAX_SESSIONS: usize = 2;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SchedulerSession {
    pub job_id: JobId,
    pub generation: Generation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScheduledWork {
    Preview { sequence: u64 },
    Final { sequence: u64 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ScheduledCall {
    pub session: SchedulerSession,
    pub incarnation: u64,
    pub request_id: u64,
    pub attempt: u64,
    pub work: ScheduledWork,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchedulerError {
    Capacity,
    DuplicateSession,
    UnknownSession,
    Terminal,
    CallInFlight,
    CounterExhausted,
    StaleCall,
    Recycling,
    NoRecyclePending,
    CallNotStarted,
}

#[derive(Clone, Debug)]
struct SessionState {
    admission_order: u64,
    next_attempt: u64,
    incarnation: u64,
    final_ready: Option<u64>,
    preview_ready: Option<u64>,
    terminal: bool,
}

/// One scheduler is owned by each backend. It retains only active session
/// counters and one replaceable preview per session; audio backlog stays on disk.
#[derive(Debug, Default)]
pub struct ParakeetScheduler {
    sessions: HashMap<SchedulerSession, SessionState>,
    next_admission_order: u64,
    next_incarnation: u64,
    next_request_id: u64,
    last_final_order: Option<u64>,
    preview_since_final: bool,
    in_flight: Option<ScheduledCall>,
    call_started: bool,
    recycle_pending: bool,
}

impl ParakeetScheduler {
    pub fn admit(&mut self, session: SchedulerSession) -> Result<u64, SchedulerError> {
        if self.sessions.contains_key(&session) {
            return Err(SchedulerError::DuplicateSession);
        }
        if self.sessions.len() >= MAX_SESSIONS {
            return Err(SchedulerError::Capacity);
        }
        if self.recycle_pending {
            return Err(SchedulerError::Recycling);
        }
        let admission_order = self.next_admission_order;
        self.next_admission_order = admission_order
            .checked_add(1)
            .ok_or(SchedulerError::CounterExhausted)?;
        let incarnation = if let Some(incarnation) =
            self.sessions.values().map(|state| state.incarnation).next()
        {
            incarnation
        } else {
            let incarnation = self.next_incarnation;
            self.next_incarnation = incarnation
                .checked_add(1)
                .ok_or(SchedulerError::CounterExhausted)?;
            incarnation
        };
        self.sessions.insert(
            session,
            SessionState {
                admission_order,
                next_attempt: 0,
                incarnation,
                final_ready: None,
                preview_ready: None,
                terminal: false,
            },
        );
        Ok(incarnation)
    }

    pub fn queue_preview(
        &mut self,
        session: SchedulerSession,
        sequence: u64,
    ) -> Result<(), SchedulerError> {
        let state = self
            .sessions
            .get_mut(&session)
            .ok_or(SchedulerError::UnknownSession)?;
        if state.terminal {
            return Err(SchedulerError::Terminal);
        }
        state.preview_ready = Some(sequence);
        Ok(())
    }

    pub fn queue_final(
        &mut self,
        session: SchedulerSession,
        sequence: u64,
    ) -> Result<(), SchedulerError> {
        let state = self
            .sessions
            .get_mut(&session)
            .ok_or(SchedulerError::UnknownSession)?;
        if state.terminal {
            return Err(SchedulerError::Terminal);
        }
        // Each session exposes only its next on-disk final range. The caller
        // publishes another sequence after the current one completes.
        if state.final_ready.is_some() {
            return Err(SchedulerError::CallInFlight);
        }
        state.final_ready = Some(sequence);
        Ok(())
    }

    pub fn clear_preview(&mut self, session: SchedulerSession) -> Result<(), SchedulerError> {
        let state = self
            .sessions
            .get_mut(&session)
            .ok_or(SchedulerError::UnknownSession)?;
        state.preview_ready = None;
        Ok(())
    }

    pub fn next_call(&mut self) -> Result<Option<ScheduledCall>, SchedulerError> {
        if self.in_flight.is_some() {
            return Err(SchedulerError::CallInFlight);
        }
        if self.recycle_pending {
            return Ok(None);
        }
        let finals_ready = self
            .sessions
            .values()
            .any(|state| state.final_ready.is_some());
        let selected = if finals_ready && self.preview_since_final {
            self.select_final()
        } else {
            self.select_preview().or_else(|| self.select_final())
        };
        let Some((session, work)) = selected else {
            return Ok(None);
        };
        let request_id = self.next_request_id;
        let state = self
            .sessions
            .get_mut(&session)
            .ok_or(SchedulerError::UnknownSession)?;
        let attempt = state.next_attempt;
        let call = ScheduledCall {
            session,
            incarnation: state.incarnation,
            request_id,
            attempt,
            work,
        };
        self.in_flight = Some(call);
        self.call_started = false;
        Ok(Some(call))
    }

    /// Called only after preparation succeeded and immediately before the
    /// backend invocation begins. Failed preparation therefore consumes no
    /// request ID, logical attempt, or fairness quota.
    pub fn begin_call(&mut self, call: ScheduledCall) -> Result<(), SchedulerError> {
        if self.in_flight != Some(call) {
            return Err(SchedulerError::StaleCall);
        }
        if self.call_started {
            return Err(SchedulerError::CallInFlight);
        }
        if call.request_id != self.next_request_id {
            return Err(SchedulerError::StaleCall);
        }
        let state = self
            .sessions
            .get_mut(&call.session)
            .ok_or(SchedulerError::UnknownSession)?;
        if call.attempt != state.next_attempt {
            return Err(SchedulerError::StaleCall);
        }
        self.next_request_id = self
            .next_request_id
            .checked_add(1)
            .ok_or(SchedulerError::CounterExhausted)?;
        state.next_attempt = state
            .next_attempt
            .checked_add(1)
            .ok_or(SchedulerError::CounterExhausted)?;
        if matches!(call.work, ScheduledWork::Preview { .. }) {
            self.preview_since_final = true;
        }
        self.call_started = true;
        Ok(())
    }

    pub fn cancel_reservation(&mut self, call: ScheduledCall) -> Result<(), SchedulerError> {
        if self.in_flight != Some(call) {
            return Err(SchedulerError::StaleCall);
        }
        if self.call_started {
            return Err(SchedulerError::CallInFlight);
        }
        if let Some(state) = self.sessions.get_mut(&call.session) {
            match call.work {
                ScheduledWork::Preview { sequence } => {
                    if state.preview_ready.is_none() {
                        state.preview_ready = Some(sequence);
                    }
                }
                ScheduledWork::Final { sequence } => {
                    state.final_ready = Some(sequence);
                }
            }
        }
        self.in_flight = None;
        Ok(())
    }

    pub fn complete(&mut self, call: ScheduledCall) -> Result<(), SchedulerError> {
        if self.in_flight != Some(call) {
            return Err(SchedulerError::StaleCall);
        }
        if !self.call_started {
            return Err(SchedulerError::CallNotStarted);
        }
        self.in_flight = None;
        self.call_started = false;
        match call.work {
            ScheduledWork::Preview { sequence } => {
                if let Some(state) = self.sessions.get_mut(&call.session) {
                    if state.preview_ready == Some(sequence) {
                        state.preview_ready = None;
                    }
                }
            }
            ScheduledWork::Final { .. } => {
                if let Some(state) = self.sessions.get_mut(&call.session) {
                    state.final_ready = None;
                    if state.terminal {
                        self.recycle_pending = true;
                    }
                }
                self.preview_since_final = false;
                self.last_final_order = self
                    .sessions
                    .get(&call.session)
                    .map(|state| state.admission_order);
            }
        }
        self.recycle_if_idle()?;
        Ok(())
    }

    /// Ends a reserved turn when preparation or inference reports an explicit
    /// failure. The caller must issue that failure and recycle the child before
    /// admitting its replacement; no fairness quota is counted as a completed call.
    pub fn fail_call(&mut self, call: ScheduledCall) -> Result<(), SchedulerError> {
        if self.in_flight != Some(call) {
            return Err(SchedulerError::StaleCall);
        }
        if !self.call_started {
            return self.cancel_reservation(call);
        }
        self.in_flight = None;
        self.call_started = false;
        if let Some(state) = self.sessions.get_mut(&call.session) {
            match call.work {
                ScheduledWork::Preview { sequence } => {
                    if state.preview_ready == Some(sequence) {
                        state.preview_ready = None;
                    }
                }
                ScheduledWork::Final { sequence } => {
                    if state.final_ready == Some(sequence) {
                        state.final_ready = None;
                    }
                }
            }
        }
        self.recycle_pending = true;
        Ok(())
    }

    pub fn mark_terminal(&mut self, session: SchedulerSession) -> Result<(), SchedulerError> {
        let state = self
            .sessions
            .get_mut(&session)
            .ok_or(SchedulerError::UnknownSession)?;
        state.terminal = true;
        state.preview_ready = None;
        if state.final_ready.is_none() {
            self.recycle_pending = true;
        }
        self.recycle_if_idle()
    }

    pub fn abort_session(&mut self, session: SchedulerSession) -> Result<(), SchedulerError> {
        let state = self
            .sessions
            .get_mut(&session)
            .ok_or(SchedulerError::UnknownSession)?;
        state.terminal = true;
        state.preview_ready = None;
        state.final_ready = None;
        self.recycle_pending = true;
        Ok(())
    }

    /// Records that the child must be replaced after its active call completes.
    /// The caller must confirm actual process exit before admitting its successor.
    pub fn request_recycle(&mut self) -> Result<(), SchedulerError> {
        self.recycle_pending = true;
        self.recycle_if_idle()
    }

    pub fn confirm_child_exited(&mut self) -> Result<(), SchedulerError> {
        if self.in_flight.is_some() {
            return Err(SchedulerError::CallInFlight);
        }
        if !self.recycle_pending {
            return Err(SchedulerError::NoRecyclePending);
        }
        self.recycle_pending = false;
        let old: Vec<_> = self
            .sessions
            .iter()
            .filter_map(|(id, state)| state.terminal.then_some(*id))
            .collect();
        for id in old {
            self.sessions.remove(&id);
        }
        let incarnation = self.next_incarnation;
        self.next_incarnation = incarnation
            .checked_add(1)
            .ok_or(SchedulerError::CounterExhausted)?;
        for state in self.sessions.values_mut() {
            state.incarnation = incarnation;
        }
        self.next_request_id = 0;
        Ok(())
    }

    pub fn active_sessions(&self) -> usize {
        self.sessions.len()
    }

    pub fn in_flight(&self) -> Option<ScheduledCall> {
        self.in_flight
    }

    fn select_preview(&mut self) -> Option<(SchedulerSession, ScheduledWork)> {
        let (session, sequence, _) = self
            .sessions
            .iter()
            .filter_map(|(session, state)| {
                state
                    .preview_ready
                    .map(|seq| (*session, seq, state.admission_order))
            })
            .max_by_key(|(_, sequence, order)| (*sequence, *order))?;
        self.sessions.get_mut(&session)?.preview_ready = None;
        Some((session, ScheduledWork::Preview { sequence }))
    }

    fn select_final(&mut self) -> Option<(SchedulerSession, ScheduledWork)> {
        let mut eligible: Vec<_> = self
            .sessions
            .iter()
            .filter_map(|(session, state)| {
                state
                    .final_ready
                    .map(|sequence| (*session, sequence, state.admission_order))
            })
            .collect();
        eligible.sort_by_key(|(_, _, order)| *order);
        let (session, sequence, _) = eligible
            .iter()
            .copied()
            .find(|(_, _, order)| Some(*order) > self.last_final_order)
            .or_else(|| eligible.first().copied())?;
        Some((session, ScheduledWork::Final { sequence }))
    }

    fn recycle_if_idle(&mut self) -> Result<(), SchedulerError> {
        if !self.recycle_pending || self.in_flight.is_some() {
            return Ok(());
        }
        // Do not drop the child identity here. The owner must confirm process
        // exit before its incarnation can be reused.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session(job_id: u128) -> SchedulerSession {
        SchedulerSession {
            job_id: JobId(job_id),
            generation: Generation::first(),
        }
    }

    #[test]
    fn preview_is_replaceable_and_final_work_has_p1_priority() {
        let a = session(1);
        let b = session(2);
        let mut scheduler = ParakeetScheduler::default();
        scheduler.admit(a).unwrap();
        scheduler.admit(b).unwrap();
        scheduler.queue_preview(b, 1).unwrap();
        scheduler.queue_preview(b, 2).unwrap();
        scheduler.queue_final(a, 10).unwrap();
        let preview = scheduler.next_call().unwrap().unwrap();
        assert_eq!(preview.work, ScheduledWork::Preview { sequence: 2 });
        assert_eq!(scheduler.next_call(), Err(SchedulerError::CallInFlight));
        scheduler.begin_call(preview).unwrap();
        scheduler.complete(preview).unwrap();
        let final_a = scheduler.next_call().unwrap().unwrap();
        assert_eq!(final_a.session, a);
        assert_eq!(final_a.work, ScheduledWork::Final { sequence: 10 });
        scheduler.begin_call(final_a).unwrap();
        scheduler.complete(final_a).unwrap();
    }

    #[test]
    fn preview_arriving_during_an_inflight_preview_survives_its_completion() {
        let a = session(1);
        let b = session(2);
        let mut scheduler = ParakeetScheduler::default();
        scheduler.admit(a).unwrap();
        scheduler.admit(b).unwrap();
        scheduler.queue_preview(b, 1).unwrap();
        scheduler.queue_final(a, 1).unwrap();
        let first = scheduler.next_call().unwrap().unwrap();
        scheduler.queue_preview(b, 2).unwrap();
        scheduler.begin_call(first).unwrap();
        scheduler.complete(first).unwrap();
        let final_call = scheduler.next_call().unwrap().unwrap();
        assert!(matches!(final_call.work, ScheduledWork::Final { .. }));
        scheduler.begin_call(final_call).unwrap();
        scheduler.complete(final_call).unwrap();
        let latest = scheduler.next_call().unwrap().unwrap();
        assert_eq!(latest.work, ScheduledWork::Preview { sequence: 2 });
    }

    #[test]
    fn final_sessions_alternate_and_old_incarnations_do_not_alias_requests() {
        let a = session(1);
        let b = session(2);
        let mut scheduler = ParakeetScheduler::default();
        let incarnation_a = scheduler.admit(a).unwrap();
        let incarnation_b = scheduler.admit(b).unwrap();
        assert_eq!(incarnation_a, incarnation_b);
        for sequence in 1..=3 {
            scheduler.queue_final(a, sequence).unwrap();
            scheduler.queue_final(b, sequence).unwrap();
            let first = scheduler.next_call().unwrap().unwrap();
            let second_session = if first.session == a { b } else { a };
            scheduler.begin_call(first).unwrap();
            scheduler.complete(first).unwrap();
            let second = scheduler.next_call().unwrap().unwrap();
            assert_eq!(second.session, second_session);
            scheduler.begin_call(second).unwrap();
            scheduler.complete(second).unwrap();
        }
        let call = scheduler.in_flight();
        assert!(call.is_none());
        scheduler.mark_terminal(a).unwrap();
        scheduler.confirm_child_exited().unwrap();
        let reincarnation_b = scheduler.admit(session(3)).unwrap();
        assert!(reincarnation_b > incarnation_a);
        let next_b = scheduler.next_call();
        assert!(next_b.unwrap().is_none());
    }

    #[test]
    fn recycling_waits_for_the_real_inflight_exit_and_preserves_other_session_attempts() {
        let a = session(1);
        let b = session(2);
        let mut scheduler = ParakeetScheduler::default();
        scheduler.admit(a).unwrap();
        scheduler.admit(b).unwrap();
        scheduler.queue_final(b, 1).unwrap();
        let first_b = scheduler.next_call().unwrap().unwrap();
        assert_eq!(first_b.session, b);
        scheduler.begin_call(first_b).unwrap();
        scheduler.mark_terminal(a).unwrap();
        assert_eq!(
            scheduler.confirm_child_exited(),
            Err(SchedulerError::CallInFlight)
        );
        assert_eq!(scheduler.next_call(), Err(SchedulerError::CallInFlight));
        scheduler.complete(first_b).unwrap();
        assert_eq!(scheduler.next_call().unwrap(), None);
        scheduler.confirm_child_exited().unwrap();
        assert_eq!(scheduler.active_sessions(), 1);
        let new_incarnation = scheduler
            .admit(session(3))
            .expect("B retains the new child slot");
        scheduler.queue_final(b, 2).unwrap();
        let second_b = scheduler.next_call().unwrap().unwrap();
        assert_eq!(second_b.session, b);
        assert_eq!(second_b.attempt, first_b.attempt + 1);
        assert_eq!(second_b.incarnation, new_incarnation);
        assert_eq!(second_b.request_id, first_b.request_id);
        assert_ne!(second_b.incarnation, first_b.incarnation);
        assert_eq!(scheduler.complete(first_b), Err(SchedulerError::StaleCall));
        scheduler.begin_call(second_b).unwrap();
        scheduler.complete(second_b).unwrap();
    }

    #[test]
    fn many_windows_reuse_two_session_records_without_growing_the_table() {
        let a = session(1);
        let b = session(2);
        let mut scheduler = ParakeetScheduler::default();
        scheduler.admit(a).unwrap();
        scheduler.admit(b).unwrap();
        for sequence in 1..=10_000 {
            scheduler.queue_final(a, sequence).unwrap();
            let call = scheduler.next_call().unwrap().unwrap();
            assert_eq!(call.session, a);
            assert_eq!(call.attempt, sequence - 1);
            scheduler.begin_call(call).unwrap();
            scheduler.complete(call).unwrap();
            assert_eq!(scheduler.active_sessions(), 2);
        }
        assert_eq!(scheduler.sessions.len(), 2);
    }

    #[test]
    fn a_third_nonterminal_session_is_refused() {
        let mut scheduler = ParakeetScheduler::default();
        scheduler.admit(session(1)).unwrap();
        scheduler.admit(session(2)).unwrap();
        assert_eq!(scheduler.admit(session(3)), Err(SchedulerError::Capacity));
        assert_eq!(scheduler.active_sessions(), 2);
    }

    #[test]
    fn preview_starvation_witness_fails_the_p1_contract() {
        let a = session(1);
        let b = session(2);
        let mut scheduler = ParakeetScheduler::default();
        scheduler.admit(a).unwrap();
        scheduler.admit(b).unwrap();
        scheduler.queue_final(a, 1).unwrap();
        scheduler.queue_preview(b, 1).unwrap();
        let preview = scheduler.next_call().unwrap().unwrap();
        scheduler.begin_call(preview).unwrap();
        scheduler.complete(preview).unwrap();
        let next = scheduler.next_call().unwrap().unwrap();
        assert!(matches!(next.work, ScheduledWork::Final { .. }));

        // Negative mutant models the legacy preview-first policy: a newly
        // produced preview is queued after every preview call, so a final
        // which remains ready is starved indefinitely.
        let mut mutant_preview = Some(1_u64);
        let mut mutant_final_ready = true;
        let mut mutant_trace = Vec::new();
        for next_sequence in 2..=4 {
            let selected = if let Some(sequence) = mutant_preview.take() {
                ScheduledWork::Preview { sequence }
            } else if mutant_final_ready {
                mutant_final_ready = false;
                ScheduledWork::Final { sequence: 1 }
            } else {
                break;
            };
            mutant_trace.push(selected);
            if matches!(selected, ScheduledWork::Preview { .. }) {
                mutant_preview = Some(next_sequence);
            }
        }
        assert!(mutant_final_ready);
        assert!(
            mutant_trace
                .iter()
                .all(|work| matches!(work, ScheduledWork::Preview { .. }))
        );
    }
}
