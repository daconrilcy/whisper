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

#[test]
fn enqueue_is_acknowledged_only_after_the_durable_queue_event() {
    let io = Fake::default();
    let mut app = ImportApplication::new(io.clone());
    let queued = request();
    let view = app
        .dispatch(AppCommand::EnqueueImport {
            request: queued.clone(),
        })
        .unwrap();
    assert!(view.queue.is_empty());
    assert!(view.message.as_deref().unwrap().contains("en cours"));

    io.0.borrow_mut()
        .events
        .push_back(ImportEvent::Queue(vec![ImportQueueEntry {
            sequence: 1,
            request: queued,
            status: ImportQueueStatus::Queued,
        }]));
    let acknowledged = app.dispatch(AppCommand::Refresh).unwrap();
    assert_eq!(acknowledged.queue.len(), 1);
    assert!(
        acknowledged
            .message
            .as_deref()
            .unwrap()
            .contains("file durable")
    );
    assert!(
        io.0.borrow()
            .effects
            .iter()
            .all(|effect| { !matches!(effect, ImportEffect::ProcessQueued(_)) })
    );
}

#[test]
fn enqueue_failure_is_visible_without_marking_a_job_active() {
    let io = Fake::default();
    let mut app = ImportApplication::new(io.clone());
    let request = request();
    let identity = (request.job_id, request.generation);
    app.dispatch(AppCommand::EnqueueImport { request }).unwrap();
    io.0.borrow_mut().events.push_back(ImportEvent::Failed {
        job_id: identity.0,
        generation: identity.1,
        message: "SourceMissing: selected audio source does not exist".into(),
    });
    let failed = app.dispatch(AppCommand::Refresh).unwrap();
    assert_eq!(failed.active_job, None);
    assert!(failed.message.as_deref().unwrap().contains("SourceMissing"));
}

#[test]
fn interrupted_resume_requires_an_explicit_command_and_advances_generation() {
    let io = Fake::default();
    let mut app = ImportApplication::new(io.clone());
    let previous = request();
    app.dispatch(AppCommand::ResumeInterrupted {
        request: previous.clone(),
    })
    .unwrap();
    let effects = io.0.borrow().effects.clone();
    let Some(ImportEffect::ResumeInterrupted {
        previous: actual,
        request: resumed,
    }) = effects.last()
    else {
        panic!("resume must go through the durable resume effect");
    };
    assert_eq!(actual, &previous);
    assert_eq!(resumed.job_id, previous.job_id);
    assert_eq!(resumed.generation, previous.generation.next().unwrap());
    assert_eq!(effects.len(), 1);
}
