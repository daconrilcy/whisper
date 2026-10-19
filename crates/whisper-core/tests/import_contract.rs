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
fn refresh_drains_at_most_four_events_and_waits_for_every_persisted_ack() {
    let io = FakeIo::default();
    let mut app = ImportApplication::new(io.clone());
    let job_id = JobId(700);
    let generation = Generation::first();
    app.dispatch(AppCommand::StartImport {
        request: request(job_id),
    })
    .unwrap();

    let sample_count = 99 * 1_280;
    let mut prepared = request(job_id);
    prepared.source_sha256 = Some([4; 32]);
    prepared.source_samples = Some(sample_count);
    io.0.borrow_mut()
        .events
        .push_back(ImportEvent::Prepared(prepared));
    io.0.borrow_mut().events.push_back(ImportEvent::Ready {
        job_id,
        generation,
        backend: "CUDA0".into(),
    });
    io.0.borrow_mut().events.push_back(ImportEvent::Progress {
        job_id,
        generation,
        completed_samples: sample_count,
        total_samples: sample_count,
    });
    for sequence in 1..=99 {
        let start = (sequence - 1) * 1_280;
        io.0.borrow_mut()
            .events
            .push_back(ImportEvent::Segment(WorkerSegment {
                job_id,
                generation,
                instance_id: 5,
                segment_id: SegmentId(sequence),
                range: SourceRange::new(start, start + 1_280, 16_000).unwrap(),
                text: format!("word-{sequence}"),
            }));
    }
    io.0.borrow_mut().events.push_back(ImportEvent::End {
        job_id,
        generation,
        last_sequence: 99,
        last_offset: sample_count,
    });

    let mut previous_segments = 0;
    while !io.0.borrow().events.is_empty() {
        app.dispatch(AppCommand::Refresh).unwrap();
        let after =
            io.0.borrow()
                .effects
                .iter()
                .filter(|effect| matches!(effect, ImportEffect::PersistSegment(_)))
                .count();
        assert!(after - previous_segments <= 4);
        previous_segments = after;
    }
    assert_eq!(
        io.0.borrow()
            .effects
            .iter()
            .filter(|effect| matches!(effect, ImportEffect::PersistSegment(_)))
            .count(),
        99
    );
    let persisted_sequences =
        io.0.borrow()
            .effects
            .iter()
            .filter_map(|effect| match effect {
                ImportEffect::PersistSegment(segment) => Some(segment.segment_id.0),
                _ => None,
            })
            .collect::<Vec<_>>();
    assert_eq!(persisted_sequences, (1..=99).collect::<Vec<_>>());
    assert!(
        !io.0
            .borrow()
            .effects
            .iter()
            .any(|effect| matches!(effect, ImportEffect::Publish { .. })),
        "End cannot publish until all segment receipts arrive"
    );

    for sequence in 1..=98 {
        io.0.borrow_mut().events.push_back(ImportEvent::Persisted {
            job_id,
            generation,
            sequence,
        });
    }
    while !io.0.borrow().events.is_empty() {
        app.dispatch(AppCommand::Refresh).unwrap();
    }
    assert!(
        !io.0
            .borrow()
            .effects
            .iter()
            .any(|effect| matches!(effect, ImportEffect::Publish { .. })),
        "98 of 99 persistence receipts are insufficient for publication"
    );
    io.0.borrow_mut().events.push_back(ImportEvent::Persisted {
        job_id,
        generation,
        sequence: 99,
    });
    let mut final_view = app.view();
    while !io.0.borrow().events.is_empty() {
        final_view = app.dispatch(AppCommand::Refresh).unwrap();
    }
    assert_eq!(final_view.active_job, Some((job_id, JobState::Finalizing)));
    assert!(matches!(
        io.0.borrow().effects.last(),
        Some(ImportEffect::Publish {
            last_sequence: 99,
            ..
        })
    ));
}

#[test]
fn persist_backpressure_retains_ninth_segment_until_queue_capacity_returns() {
    struct BoundedState {
        events: VecDeque<ImportEvent>,
        effects_rx: std::sync::mpsc::Receiver<ImportEffect>,
        persist_attempts: usize,
    }
    #[derive(Clone)]
    struct BoundedIo {
        effects_tx: std::sync::mpsc::SyncSender<ImportEffect>,
        state: Rc<RefCell<BoundedState>>,
    }
    impl ImportIoPort for BoundedIo {
        fn submit(&mut self, effect: ImportEffect) -> Result<(), PortError> {
            let is_segment = matches!(&effect, ImportEffect::PersistSegment(_));
            if is_segment {
                self.state.borrow_mut().persist_attempts += 1;
            }
            self.effects_tx
                .try_send(effect)
                .map_err(|error| match error {
                    std::sync::mpsc::TrySendError::Full(_) if is_segment => PortError::Backpressure,
                    std::sync::mpsc::TrySendError::Full(_)
                    | std::sync::mpsc::TrySendError::Disconnected(_) => PortError::Unavailable,
                })
        }

        fn poll(&mut self) -> Result<Option<ImportEvent>, PortError> {
            Ok(self.state.borrow_mut().events.pop_front())
        }
    }

    let (effects_tx, effects_rx) = std::sync::mpsc::sync_channel(8);
    let state = Rc::new(RefCell::new(BoundedState {
        events: VecDeque::new(),
        effects_rx,
        persist_attempts: 0,
    }));
    let mut app = ImportApplication::new(BoundedIo {
        effects_tx,
        state: state.clone(),
    });
    let job_id = JobId(702);
    let generation = Generation::first();
    app.dispatch(AppCommand::StartImport {
        request: request(job_id),
    })
    .unwrap();
    assert!(matches!(
        state.borrow_mut().effects_rx.try_recv(),
        Ok(ImportEffect::Prepare(_))
    ));

    let mut prepared = request(job_id);
    prepared.source_sha256 = Some([6; 32]);
    prepared.source_samples = Some(9 * 80);
    state
        .borrow_mut()
        .events
        .push_back(ImportEvent::Prepared(prepared));
    app.dispatch(AppCommand::Refresh).unwrap();
    assert!(matches!(
        state.borrow_mut().effects_rx.try_recv(),
        Ok(ImportEffect::StartWorker(_))
    ));

    state.borrow_mut().events.push_back(ImportEvent::Ready {
        job_id,
        generation,
        backend: "CPU".into(),
    });
    state.borrow_mut().events.push_back(ImportEvent::Progress {
        job_id,
        generation,
        completed_samples: 9 * 80,
        total_samples: 9 * 80,
    });
    for sequence in 1..=9 {
        state
            .borrow_mut()
            .events
            .push_back(ImportEvent::Segment(WorkerSegment {
                job_id,
                generation,
                instance_id: 1,
                segment_id: SegmentId(sequence),
                range: SourceRange::new((sequence - 1) * 80, sequence * 80, 16_000).unwrap(),
                text: format!("segment-{sequence}"),
            }));
    }
    state.borrow_mut().events.push_back(ImportEvent::End {
        job_id,
        generation,
        last_sequence: 9,
        last_offset: 9 * 80,
    });

    while state.borrow().persist_attempts < 9 {
        app.dispatch(AppCommand::Refresh).unwrap();
    }
    assert_eq!(state.borrow().persist_attempts, 9);
    let mut accepted = state.borrow_mut().effects_rx.try_iter().collect::<Vec<_>>();
    assert_eq!(
        accepted.len(),
        8,
        "the bounded consumer stays blocked with exactly eight queued effects"
    );
    assert_eq!(
        state.borrow().events.len(),
        1,
        "End remains unread while the ninth segment is retained for retry"
    );

    let submitted_sequences = |effects: &[ImportEffect]| {
        effects
            .iter()
            .filter_map(|effect| match effect {
                ImportEffect::PersistSegment(segment) => Some(segment.segment_id.0),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(submitted_sequences(&accepted), (1..=8).collect::<Vec<_>>());

    app.dispatch(AppCommand::Refresh).unwrap();
    accepted.extend(state.borrow_mut().effects_rx.try_iter());
    assert_eq!(submitted_sequences(&accepted), (1..=9).collect::<Vec<_>>());
    assert!(
        !accepted
            .iter()
            .any(|effect| matches!(effect, ImportEffect::Publish { .. })),
        "the end marker cannot publish while persistence acknowledgments are pending"
    );

    for sequence in 1..=9 {
        state.borrow_mut().events.push_back(ImportEvent::Persisted {
            job_id,
            generation,
            sequence,
        });
    }
    while !state.borrow().events.is_empty() {
        app.dispatch(AppCommand::Refresh).unwrap();
    }
    let final_effects = state.borrow_mut().effects_rx.try_iter().collect::<Vec<_>>();
    assert!(matches!(
        final_effects.as_slice(),
        [ImportEffect::Publish {
            last_sequence: 9,
            ..
        }]
    ));
    assert_eq!(state.borrow().persist_attempts, 10);
}

#[test]
fn disconnected_persist_queue_is_fatal_and_does_not_retry_the_segment() {
    struct BoundedState {
        events: VecDeque<ImportEvent>,
        effects_rx: Option<std::sync::mpsc::Receiver<ImportEffect>>,
        persist_attempts: usize,
    }
    #[derive(Clone)]
    struct BoundedIo {
        effects_tx: std::sync::mpsc::SyncSender<ImportEffect>,
        state: Rc<RefCell<BoundedState>>,
    }
    impl ImportIoPort for BoundedIo {
        fn submit(&mut self, effect: ImportEffect) -> Result<(), PortError> {
            let is_segment = matches!(&effect, ImportEffect::PersistSegment(_));
            if is_segment {
                self.state.borrow_mut().persist_attempts += 1;
            }
            self.effects_tx
                .try_send(effect)
                .map_err(|error| match error {
                    std::sync::mpsc::TrySendError::Full(_) if is_segment => PortError::Backpressure,
                    std::sync::mpsc::TrySendError::Full(_)
                    | std::sync::mpsc::TrySendError::Disconnected(_) => PortError::Unavailable,
                })
        }

        fn poll(&mut self) -> Result<Option<ImportEvent>, PortError> {
            Ok(self.state.borrow_mut().events.pop_front())
        }
    }

    let (effects_tx, effects_rx) = std::sync::mpsc::sync_channel(8);
    let state = Rc::new(RefCell::new(BoundedState {
        events: VecDeque::new(),
        effects_rx: Some(effects_rx),
        persist_attempts: 0,
    }));
    let mut app = ImportApplication::new(BoundedIo {
        effects_tx,
        state: state.clone(),
    });
    let job_id = JobId(703);
    let generation = Generation::first();
    app.dispatch(AppCommand::StartImport {
        request: request(job_id),
    })
    .unwrap();
    assert!(matches!(
        state.borrow_mut().effects_rx.as_mut().unwrap().try_recv(),
        Ok(ImportEffect::Prepare(_))
    ));
    let mut prepared = request(job_id);
    prepared.source_sha256 = Some([7; 32]);
    prepared.source_samples = Some(80);
    state
        .borrow_mut()
        .events
        .push_back(ImportEvent::Prepared(prepared));
    app.dispatch(AppCommand::Refresh).unwrap();
    assert!(matches!(
        state.borrow_mut().effects_rx.as_mut().unwrap().try_recv(),
        Ok(ImportEffect::StartWorker(_))
    ));
    state.borrow_mut().effects_rx.take();
    state
        .borrow_mut()
        .events
        .push_back(ImportEvent::Segment(WorkerSegment {
            job_id,
            generation,
            instance_id: 1,
            segment_id: SegmentId(1),
            range: SourceRange::new(0, 80, 16_000).unwrap(),
            text: "segment-1".into(),
        }));

    assert!(app.dispatch(AppCommand::Refresh).is_err());
    assert_eq!(state.borrow().persist_attempts, 1);
    assert!(app.view().message.as_deref().is_some_and(|message| {
        message.contains("Import indisponible") && message.contains("Unavailable")
    }));
    app.dispatch(AppCommand::Refresh).unwrap();
    assert_eq!(
        state.borrow().persist_attempts,
        1,
        "a disconnected queue is fatal, not a retryable backpressure signal"
    );
}

#[test]
fn ready_and_following_failure_are_observable_on_separate_refreshes() {
    let io = FakeIo::default();
    let mut app = ImportApplication::new(io.clone());
    let job_id = JobId(701);
    let generation = Generation::first();
    app.dispatch(AppCommand::StartImport {
        request: request(job_id),
    })
    .unwrap();

    let mut prepared = request(job_id);
    prepared.source_sha256 = Some([5; 32]);
    prepared.source_samples = Some(80);
    io.0.borrow_mut()
        .events
        .push_back(ImportEvent::Prepared(prepared));
    io.0.borrow_mut().events.push_back(ImportEvent::Ready {
        job_id,
        generation,
        backend: "CUDA0".into(),
    });
    io.0.borrow_mut().events.push_back(ImportEvent::Failed {
        job_id,
        generation,
        message: "WorkerExited: injected after NativeCallStarted".into(),
    });

    assert_eq!(
        app.dispatch(AppCommand::Refresh).unwrap().active_job,
        Some((job_id, JobState::Running))
    );
    let failed_view = app.dispatch(AppCommand::Refresh).unwrap();
    assert_eq!(
        failed_view.active_job,
        Some((job_id, JobState::Recoverable))
    );
    assert!(failed_view.last_result.is_none());
    assert!(failed_view.message.as_deref().is_some_and(|message| {
        message.contains("WorkerExited: injected after NativeCallStarted")
    }));
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
