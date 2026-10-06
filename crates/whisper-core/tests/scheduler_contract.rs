use std::{cell::RefCell, collections::VecDeque, rc::Rc};
use whisper_core::{
    AppCommand, Application, ComputeChoice, Generation, ImportApplication, ImportEffect,
    ImportEvent, ImportIoPort, ImportRequest, JobConfig, JobId, LanguageChoice,
    ports::{ImportQueueEntry, ImportQueueStatus, PortError},
};

#[derive(Default)]
struct State {
    effects: Vec<ImportEffect>,
    events: VecDeque<ImportEvent>,
}
#[derive(Clone, Default)]
struct Fake(Rc<RefCell<State>>);
impl ImportIoPort for Fake {
    fn submit(&mut self, effect: ImportEffect) -> Result<(), PortError> {
        self.0.borrow_mut().effects.push(effect);
        Ok(())
    }
    fn poll(&mut self) -> Result<Option<ImportEvent>, PortError> {
        Ok(self.0.borrow_mut().events.pop_front())
    }
}
fn request() -> ImportRequest {
    ImportRequest {
        job_id: JobId(4),
        generation: Generation::first(),
        source_path: "input.mp3".into(),
        source_sha256: None,
        source_samples: None,
        model_path: "model.bin".into(),
        model_sha256: [0; 32],
        destination: "out".into(),
        config: JobConfig {
            language: LanguageChoice::Manual("fr".into()),
            compute: ComputeChoice::Cpu,
        },
    }
}

#[test]
fn admission_and_restoration_never_start_a_worker_without_user_choice() {
    let io = Fake::default();
    let mut app = ImportApplication::new(io.clone());
    let queued = request();
    app.dispatch(AppCommand::EnqueueImport {
        request: queued.clone(),
    })
    .unwrap();
    assert!(matches!(
        io.0.borrow().effects.as_slice(),
        [ImportEffect::Enqueue(_)]
    ));

    io.0.borrow_mut()
        .events
        .push_back(ImportEvent::Queue(vec![ImportQueueEntry {
            sequence: 1,
            request: queued.clone(),
            status: ImportQueueStatus::AwaitingChoice,
        }]));
    let view = app.dispatch(AppCommand::Refresh).unwrap();
    assert_eq!(view.queue.len(), 1);
    assert_eq!(
        io.0.borrow().effects.len(),
        1,
        "restore does not prepare or launch"
    );

    app.dispatch(AppCommand::ProcessQueued { request: queued })
        .unwrap();
    assert!(matches!(
        io.0.borrow().effects.last(),
        Some(ImportEffect::ProcessQueued(_))
    ));
}
