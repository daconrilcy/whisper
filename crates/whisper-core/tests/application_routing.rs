use std::{cell::RefCell, collections::VecDeque, rc::Rc};

use whisper_core::{
    AppCommand, Application, ApplicationError, ComputeChoice, Generation, ImportApplication,
    ImportEffect, ImportEvent, ImportIoPort, JobConfig, JobId, LanguageChoice,
    ports::{ArchiveHistoryItem, PortError},
};

#[derive(Default)]
struct State {
    effects: Vec<ImportEffect>,
    events: VecDeque<ImportEvent>,
}

#[derive(Clone, Default)]
struct MemoryPort(Rc<RefCell<State>>);

impl ImportIoPort for MemoryPort {
    fn submit(&mut self, effect: ImportEffect) -> Result<(), PortError> {
        self.0.borrow_mut().effects.push(effect);
        Ok(())
    }

    fn poll(&mut self) -> Result<Option<ImportEvent>, PortError> {
        Ok(self.0.borrow_mut().events.pop_front())
    }
}

#[test]
fn legacy_start_live_is_rejected_without_effects() {
    let io = MemoryPort::default();
    let mut app = ImportApplication::new(io.clone());
    let before = app.view();
    let result = app.dispatch(AppCommand::StartLive {
        job_id: JobId(1),
        generation: Generation::first(),
        config: JobConfig {
            language: LanguageChoice::Manual("fr".into()),
            compute: ComputeChoice::Cpu,
        },
    });

    assert_eq!(result, Err(ApplicationError::InvalidCommand));
    assert_eq!(app.view(), before);
    assert!(io.0.borrow().effects.is_empty());
}

#[test]
fn history_event_is_applied_without_an_active_job() {
    let io = MemoryPort::default();
    let mut app = ImportApplication::new(io.clone());
    let entry = ArchiveHistoryItem {
        job_id: JobId(12),
        generation: Generation::first(),
        complete: true,
        source_sha256: "sha256".into(),
        diagnostic: None,
        live_recovery: None,
    };
    io.0.borrow_mut()
        .events
        .push_back(ImportEvent::History(vec![entry.clone()]));

    let view = app.dispatch(AppCommand::Refresh).unwrap();

    assert!(view.active_job.is_none());
    assert_eq!(view.history, vec![entry]);
    assert!(!view.history_scan_pending);
    assert!(io.0.borrow().effects.is_empty());
}
