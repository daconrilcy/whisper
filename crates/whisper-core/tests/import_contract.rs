use std::{cell::RefCell, collections::VecDeque, rc::Rc};
use whisper_core::{
    AppCommand, Application, ComputeChoice, Generation, ImportApplication, ImportEffect,
    ImportEvent, ImportIoPort, ImportRequest, JobConfig, JobId, JobState, LanguageChoice,
    SegmentId, SourceRange,
    ports::{PortError, WorkerSegment},
};

#[derive(Default)]
struct FakeState {
    effects: Vec<ImportEffect>,
    events: VecDeque<ImportEvent>,
    refuse_publish: bool,
}
#[derive(Clone, Default)]
struct FakeIo(Rc<RefCell<FakeState>>);
impl ImportIoPort for FakeIo {
    fn submit(&mut self, effect: ImportEffect) -> Result<(), PortError> {
        if self.0.borrow().refuse_publish && matches!(effect, ImportEffect::Publish { .. }) {
            return Err(PortError::Failed("publication refused".into()));
        }
        self.0.borrow_mut().effects.push(effect);
        Ok(())
    }
    fn poll(&mut self) -> Result<Option<ImportEvent>, PortError> {
        Ok(self.0.borrow_mut().events.pop_front())
    }
}

fn request(job_id: JobId) -> ImportRequest {
    ImportRequest {
        job_id,
        generation: Generation::first(),
        source_path: "sample.wav".into(),
        source_sha256: None,
        source_samples: None,
        model_path: "approved-model.bin".into(),
        model_sha256: [7; 32],
        destination: "archive".into(),
        config: JobConfig {
            language: LanguageChoice::Manual("fr".into()),
            compute: ComputeChoice::Cpu,
        },
    }
}

#[test]
fn application_publishes_only_after_end_and_synced_segment_receipt() {
    let io = FakeIo::default();
    let mut app = ImportApplication::new(io.clone());
    let job_id = JobId(7);
    let generation = Generation::first();
    app.dispatch(AppCommand::StartImport {
        request: request(job_id),
    })
    .unwrap();
    assert!(matches!(
        io.0.borrow().effects.as_slice(),
        [ImportEffect::Prepare(_)]
    ));
    let mut prepared = request(job_id);
    prepared.source_sha256 = Some([3; 32]);
    prepared.source_samples = Some(80);
    io.0.borrow_mut()
        .events
        .push_back(ImportEvent::Prepared(prepared));
    app.dispatch(AppCommand::Refresh).unwrap();
    assert!(matches!(
        io.0.borrow().effects.last(),
        Some(ImportEffect::StartWorker(_))
    ));
    io.0.borrow_mut().events.push_back(ImportEvent::Ready {
        job_id,
        generation,
        backend: "CPU".into(),
    });
    assert_eq!(
        app.dispatch(AppCommand::Refresh).unwrap().active_job,
        Some((job_id, JobState::Running))
    );

    io.0.borrow_mut()
        .events
        .push_back(ImportEvent::Segment(WorkerSegment {
            job_id,
            generation,
            instance_id: 4,
            segment_id: SegmentId(1),
            range: SourceRange::new(0, 80, 16_000).unwrap(),
            text: "bonjour".into(),
        }));
    app.dispatch(AppCommand::Refresh).unwrap();
    assert!(matches!(
        io.0.borrow().effects.last(),
        Some(ImportEffect::PersistSegment(_))
    ));
    io.0.borrow_mut().events.push_back(ImportEvent::Progress {
        job_id,
        generation,
        completed_samples: 80,
        total_samples: 80,
    });
    app.dispatch(AppCommand::Refresh).unwrap();
    io.0.borrow_mut().events.push_back(ImportEvent::End {
        job_id,
        generation,
        last_sequence: 1,
        last_offset: 80,
    });
    app.dispatch(AppCommand::Refresh).unwrap();
    assert!(
        !matches!(
            io.0.borrow().effects.last(),
            Some(ImportEffect::Publish { .. })
        ),
        "End alone cannot publish an unpersisted segment"
    );
    io.0.borrow_mut().events.push_back(ImportEvent::Persisted {
        job_id,
        generation,
        sequence: 1,
    });
    assert_eq!(
        app.dispatch(AppCommand::Refresh).unwrap().active_job,
        Some((job_id, JobState::Finalizing))
    );
    assert!(matches!(
        io.0.borrow().effects.last(),
        Some(ImportEffect::Publish {
            last_sequence: 1,
            ..
        })
    ));
    io.0.borrow_mut()
        .events
        .push_back(ImportEvent::Published { job_id, generation });
    assert_eq!(
        app.dispatch(AppCommand::Refresh).unwrap().active_job,
        Some((job_id, JobState::Complete))
    );
}

#[test]
fn stop_invalidates_generation_and_rejects_late_segment_but_accepts_stop_receipt() {
    let io = FakeIo::default();
    let mut app = ImportApplication::new(io.clone());
    let job_id = JobId(9);
    let old = Generation::first();
    app.dispatch(AppCommand::StartImport {
        request: request(job_id),
    })
    .unwrap();
    app.dispatch(AppCommand::Stop {
        job_id,
        generation: old,
    })
    .unwrap();
    io.0.borrow_mut()
        .events
        .push_back(ImportEvent::Segment(WorkerSegment {
            job_id,
            generation: old,
            instance_id: 3,
            segment_id: SegmentId(1),
            range: SourceRange::new(0, 80, 16_000).unwrap(),
            text: "late".into(),
        }));
    assert_eq!(
        app.dispatch(AppCommand::Refresh).unwrap().active_job,
        Some((job_id, JobState::Cancelling))
    );
    assert!(
        !io.0
            .borrow()
            .effects
            .iter()
            .any(|effect| matches!(effect, ImportEffect::PersistSegment(_)))
    );
    io.0.borrow_mut().events.push_back(ImportEvent::Stopped {
        job_id,
        generation: old,
    });
    assert_eq!(
        app.dispatch(AppCommand::Refresh).unwrap().active_job,
        Some((job_id, JobState::Cancelled))
    );
}

#[test]
fn stop_during_preparation_and_after_end_never_completes_old_generation() {
    let job_id = JobId(31);
    let generation = Generation::first();
    let io = FakeIo::default();
    let mut app = ImportApplication::new(io.clone());
    app.dispatch(AppCommand::StartImport {
        request: request(job_id),
    })
    .unwrap();
    app.dispatch(AppCommand::Stop { job_id, generation })
        .unwrap();
    let mut prepared = request(job_id);
    prepared.source_sha256 = Some([3; 32]);
    prepared.source_samples = Some(80);
    io.0.borrow_mut()
        .events
        .push_back(ImportEvent::Prepared(prepared));
    app.dispatch(AppCommand::Refresh).unwrap();
    io.0.borrow_mut()
        .events
        .push_back(ImportEvent::Stopped { job_id, generation });
    assert_eq!(
        app.dispatch(AppCommand::Refresh).unwrap().active_job,
        Some((job_id, JobState::Cancelled))
    );
    assert!(
        !io.0
            .borrow()
            .effects
            .iter()
            .any(|effect| matches!(effect, ImportEffect::StartWorker(_)))
    );

    let io = FakeIo::default();
    let mut app = ImportApplication::new(io.clone());
    app.dispatch(AppCommand::StartImport {
        request: request(job_id),
    })
    .unwrap();
    let mut prepared = request(job_id);
    prepared.source_sha256 = Some([3; 32]);
    prepared.source_samples = Some(1);
    io.0.borrow_mut()
        .events
        .push_back(ImportEvent::Prepared(prepared));
    app.dispatch(AppCommand::Refresh).unwrap();
    io.0.borrow_mut().events.push_back(ImportEvent::Progress {
        job_id,
        generation,
        completed_samples: 1,
        total_samples: 1,
    });
    app.dispatch(AppCommand::Refresh).unwrap();
    io.0.borrow_mut().events.push_back(ImportEvent::End {
        job_id,
        generation,
        last_sequence: 0,
        last_offset: 1,
    });
    assert_eq!(
        app.dispatch(AppCommand::Refresh).unwrap().active_job,
        Some((job_id, JobState::Finalizing))
    );
    app.dispatch(AppCommand::Stop { job_id, generation })
        .unwrap();
    io.0.borrow_mut()
        .events
        .push_back(ImportEvent::Published { job_id, generation });
    assert_eq!(
        app.dispatch(AppCommand::Refresh).unwrap().active_job,
        Some((job_id, JobState::Cancelling))
    );
    io.0.borrow_mut()
        .events
        .push_back(ImportEvent::Stopped { job_id, generation });
    assert_eq!(
        app.dispatch(AppCommand::Refresh).unwrap().active_job,
        Some((job_id, JobState::Cancelled))
    );
    assert!(!matches!(
        app.view().active_job,
        Some((_, JobState::Complete))
    ));
}

#[test]
fn end_offset_and_segment_ranges_are_validated_before_publication() {
    let job_id = JobId(41);
    let generation = Generation::first();
    let io = FakeIo::default();
    let mut app = ImportApplication::new(io.clone());
    app.dispatch(AppCommand::StartImport {
        request: request(job_id),
    })
    .unwrap();
    let mut prepared = request(job_id);
    prepared.source_sha256 = Some([3; 32]);
    prepared.source_samples = Some(80);
    io.0.borrow_mut()
        .events
        .push_back(ImportEvent::Prepared(prepared));
    app.dispatch(AppCommand::Refresh).unwrap();
    io.0.borrow_mut()
        .events
        .push_back(ImportEvent::Segment(WorkerSegment {
            job_id,
            generation,
            instance_id: 1,
            segment_id: SegmentId(1),
            range: SourceRange::new(100, 120, 16_000).unwrap(),
            text: "segment beyond End".into(),
        }));
    assert_eq!(
        app.dispatch(AppCommand::Refresh),
        Err(whisper_core::application::ApplicationError::InvalidCommand),
        "segment range beyond prepared source duration must be rejected"
    );
    assert!(
        !io.0
            .borrow()
            .effects
            .iter()
            .any(|effect| matches!(effect, ImportEffect::PersistSegment(_)))
    );
    io.0.borrow_mut().events.push_back(ImportEvent::Progress {
        job_id,
        generation,
        completed_samples: 80,
        total_samples: 80,
    });
    app.dispatch(AppCommand::Refresh).unwrap();
    io.0.borrow_mut().events.push_back(ImportEvent::End {
        job_id,
        generation,
        last_sequence: 0,
        last_offset: 81,
    });
    assert_eq!(
        app.dispatch(AppCommand::Refresh),
        Err(whisper_core::application::ApplicationError::InvalidCommand)
    );
    assert!(
        !io.0
            .borrow()
            .effects
            .iter()
            .any(|effect| matches!(effect, ImportEffect::Publish { .. }))
    );
}

#[test]
fn failed_stop_receipt_is_visible_and_does_not_leave_cancelling_stuck() {
    let io = FakeIo::default();
    let mut app = ImportApplication::new(io.clone());
    let job_id = JobId(51);
    let generation = Generation::first();
    app.dispatch(AppCommand::StartImport {
        request: request(job_id),
    })
    .unwrap();
    app.dispatch(AppCommand::Stop { job_id, generation })
        .unwrap();
    io.0.borrow_mut().events.push_back(ImportEvent::Failed {
        job_id,
        generation,
        message: "worker stopped before readiness".into(),
    });
    app.dispatch(AppCommand::Refresh).unwrap();
    let view = app.view();
    assert_eq!(view.active_job, Some((job_id, JobState::Cancelled)));
    assert!(
        view.message
            .as_deref()
            .is_some_and(|message| message.contains("Arrêt confirmé"))
    );
}

#[test]
fn invalid_wire_constructible_ranges_are_rejected_before_persistence() {
    for range in [
        SourceRange {
            start_sample: 8,
            end_sample: 8,
            sample_rate_hz: 16_000,
        },
        SourceRange {
            start_sample: 0,
            end_sample: 8,
            sample_rate_hz: 0,
        },
        SourceRange {
            start_sample: 0,
            end_sample: 8,
            sample_rate_hz: 48_000,
        },
    ] {
        let io = FakeIo::default();
        let mut app = ImportApplication::new(io.clone());
        let job_id = JobId(61);
        let generation = Generation::first();
        app.dispatch(AppCommand::StartImport {
            request: request(job_id),
        })
        .unwrap();
        let mut prepared = request(job_id);
        prepared.source_sha256 = Some([3; 32]);
        prepared.source_samples = Some(80);
        io.0.borrow_mut()
            .events
            .push_back(ImportEvent::Prepared(prepared));
        app.dispatch(AppCommand::Refresh).unwrap();
        io.0.borrow_mut()
            .events
            .push_back(ImportEvent::Segment(WorkerSegment {
                job_id,
                generation,
                instance_id: 1,
                segment_id: SegmentId(1),
                range,
                text: "invalid range".into(),
            }));
        assert_eq!(
            app.dispatch(AppCommand::Refresh),
            Err(whisper_core::application::ApplicationError::InvalidCommand)
        );
        assert!(!io.0.borrow().effects.iter().any(|effect| matches!(
            effect,
            ImportEffect::PersistSegment(_) | ImportEffect::Publish { .. }
        )));
    }
}

#[test]
fn refused_publish_never_becomes_complete() {
    let io = FakeIo::default();
    let mut app = ImportApplication::new(io.clone());
    let job_id = JobId(62);
    let generation = Generation::first();
    app.dispatch(AppCommand::StartImport {
        request: request(job_id),
    })
    .unwrap();
    let mut prepared = request(job_id);
    prepared.source_sha256 = Some([3; 32]);
    prepared.source_samples = Some(1);
    io.0.borrow_mut()
        .events
        .push_back(ImportEvent::Prepared(prepared));
    app.dispatch(AppCommand::Refresh).unwrap();
    io.0.borrow_mut().events.push_back(ImportEvent::Progress {
        job_id,
        generation,
        completed_samples: 1,
        total_samples: 1,
    });
    app.dispatch(AppCommand::Refresh).unwrap();
    io.0.borrow_mut().events.push_back(ImportEvent::End {
        job_id,
        generation,
        last_sequence: 0,
        last_offset: 1,
    });
    io.0.borrow_mut().refuse_publish = true;
    assert_eq!(
        app.dispatch(AppCommand::Refresh),
        Err(whisper_core::application::ApplicationError::PortUnavailable)
    );
    assert_ne!(app.view().active_job, Some((job_id, JobState::Complete)));
    assert!(!matches!(
        io.0.borrow().effects.last(),
        Some(ImportEffect::Publish { .. })
    ));
}
