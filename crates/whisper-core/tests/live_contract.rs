use std::{cell::RefCell, collections::VecDeque, rc::Rc};
use whisper_core::{
    AppCommand, Application, ComputeChoice, Generation, ImportApplication, ImportEffect,
    ImportEvent, JobConfig, JobId, LanguageChoice, LiveRequest,
    ports::{ImportIoPort, PortError},
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

fn request() -> LiveRequest {
    LiveRequest {
        job_id: JobId(42),
        generation: Generation::first(),
        model_path: "model.bin".into(),
        model_sha256: [0; 32],
        destination: "out".into(),
        group_offset_samples: 0,
        config: JobConfig {
            language: LanguageChoice::Manual("fr".into()),
            compute: ComputeChoice::Cpu,
        },
    }
}

#[test]
fn starting_live_emits_one_explicit_effect_and_initializes_separate_counters() {
    let io = Fake::default();
    let mut app = ImportApplication::new(io.clone());
    let request = request();

    let view = app
        .dispatch(AppCommand::StartLiveConfigured {
            request: request.clone(),
        })
        .unwrap();

    assert_eq!(
        view.active_job,
        Some((request.job_id, whisper_core::JobState::Preparing))
    );
    assert_eq!(
        (
            view.captured_samples,
            view.admitted_samples,
            view.audio_durable_samples,
            view.confirmed_fragment_samples,
            view.speech_samples
        ),
        (0, 0, 0, 0, 0)
    );
    assert!(matches!(
        io.0.borrow().effects.as_slice(),
        [ImportEffect::StartLive(actual)] if actual == &request
    ));
}

#[test]
fn restored_live_recovery_is_visible_without_automatically_starting_a_new_passage() {
    use whisper_core::ports::{ArchiveHistoryItem, LiveRecoveryInfo};

    let io = Fake::default();
    let mut app = ImportApplication::new(io.clone());
    let request = request();
    io.0.borrow_mut()
        .events
        .push_back(ImportEvent::History(vec![ArchiveHistoryItem {
            job_id: request.job_id,
            generation: request.generation,
            complete: false,
            source_sha256: String::new(),
            diagnostic: Some("Recoverable: interrupted passage".into()),
            live_recovery: Some(LiveRecoveryInfo {
                request,
                durable_samples: 16_000,
            }),
        }]));

    let view = app.dispatch(AppCommand::Refresh).unwrap();

    assert_eq!(view.history.len(), 1);
    assert!(view.history[0].live_recovery.is_some());
    assert!(io.0.borrow().effects.is_empty());
}
