    use super::*;
    use crate::domain::{ComputeChoice, LanguageChoice};
    use std::collections::VecDeque;

    #[derive(Default)]
    struct ScanTestIo {
        effects: Vec<ImportEffect>,
        events: VecDeque<ImportEvent>,
        stop_committing: bool,
    }

    impl ImportIoPort for ScanTestIo {
        fn submit(&mut self, effect: ImportEffect) -> Result<(), PortError> {
            if self.stop_committing && matches!(effect, ImportEffect::Stop { .. }) {
                return Err(PortError::Committing);
            }
            self.effects.push(effect);
            Ok(())
        }

        fn poll(&mut self) -> Result<Option<ImportEvent>, PortError> {
            Ok(self.events.pop_front())
        }
    }

    #[test]
    fn close_blocks_direct_and_indirect_admissions_without_submitting_effects() {
        let mut app = ImportApplication::new(ScanTestIo::default());
        app.dispatch(AppCommand::RequestClose).unwrap();
        assert!(app.view.close_requested);
        assert!(app.view.close_ready);

        let job_id = JobId(991);
        let generation = Generation::first();
        let request = ImportRequest {
            strategy: crate::parakeet_session::TranscriptionStrategy::BoundedWindowsV1,
            job_id,
            generation,
            source_path: "source.wav".into(),
            source_sha256: None,
            source_samples: None,
            model_path: "model.nmodel".into(),
            model_sha256: [1; 32],
            destination: "archive".into(),
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Cpu,
            },
        };
        let prompt = DuplicateImportPrompt {
            request: request.clone(),
            source_sha256: "00".repeat(32),
            source_samples: 16_000,
            publications: Vec::new(),
            export_basename: "source".into(),
            revalidation_notice: None,
            resume_from: None,
            reserved_export_basename: None,
        };
        let live = LiveRequest {
            job_id,
            generation,
            model_path: "model.nmodel".into(),
            model_sha256: [1; 32],
            destination: "archive".into(),
            group_offset_samples: 0,
            auto_stop_after_speech_samples: None,
            microphone: crate::microphone::MicrophoneChoice::WindowsDefault,
            microphone_gain_db: 0,
            config: request.config.clone(),
        };
        let commands = vec![
            AppCommand::StartImport {
                request: request.clone(),
            },
            AppCommand::EnqueueImport {
                request: request.clone(),
            },
            AppCommand::ProcessQueued {
                request: request.clone(),
            },
            AppCommand::ResumeInterrupted {
                request: ImportRequest {
                    generation: generation.next().unwrap(),
                    ..request.clone()
                },
            },
            AppCommand::StartLive {
                job_id,
                generation,
                config: request.config.clone(),
            },
            AppCommand::StartLiveConfigured {
                request: live.clone(),
            },
            AppCommand::StartUnifiedLiveConfigured {
                request: live.clone(),
            },
            AppCommand::StartDirectLiveConfigured { request: live },
            AppCommand::StartMicrophoneTest {
                job_id,
                generation,
                microphone: crate::microphone::MicrophoneChoice::WindowsDefault,
                gain_db: 0,
            },
            AppCommand::ResolveDuplicateImport {
                prompt: Box::new(prompt),
                choice: DuplicateImportChoice::CreateNew,
            },
        ];
        for command in commands {
            assert_eq!(app.dispatch(command), Err(ApplicationError::InvalidCommand));
        }
        assert!(app.io.effects.is_empty());
        assert!(app.view.close_ready);
    }

    #[test]
    fn empty_snapshot_recovery_inspection_closes_dialog_and_reports_result() {
        let mut io = ScanTestIo::default();
        io.events
            .push_back(ImportEvent::SourceSnapshotRecoveryInspected(Vec::new()));
        let mut app = ImportApplication::new(io);

        let view = app.dispatch(AppCommand::Refresh).unwrap();

        assert!(view.source_snapshot_recovery.is_none());
        assert!(
            view.message
                .as_deref()
                .is_some_and(|message| message.contains("aucune copie audio retenue récupérable"))
        );
    }

    #[test]
    fn snapshot_recovery_completion_closes_dialog_and_keeps_error_and_protected_reason_visible() {
        let mut io = ScanTestIo::default();
        io.events
            .push_back(ImportEvent::SourceSnapshotRecoveryCompleted {
                freed: 0,
                candidates: vec![SourceSnapshotRecoveryCandidate {
                    job_id: JobId(812),
                    generation: Generation::first(),
                    destination: "archive/protected".into(),
                    recoverable: false,
                    reason: Some("copie liée à une demande active".into()),
                }],
                error: Some("registre indisponible".into()),
            });
        let mut app = ImportApplication::new(io);

        let view = app.dispatch(AppCommand::Refresh).unwrap();

        assert!(view.source_snapshot_recovery.is_none());
        assert!(!view.source_snapshot_recovery_available);
        let message = view.message.as_deref().unwrap();
        assert!(message.contains("registre indisponible"));
        assert!(message.contains("copie liée à une demande active"));
        assert!(message.contains("0000000000000000000000000000032c"));
    }

    #[test]
    fn successful_snapshot_recovery_completion_closes_dialog_and_reports_recovered_copies() {
        let mut io = ScanTestIo::default();
        io.events
            .push_back(ImportEvent::SourceSnapshotRecoveryCompleted {
                freed: 2,
                candidates: Vec::new(),
                error: None,
            });
        let mut app = ImportApplication::new(io);

        let view = app.dispatch(AppCommand::Refresh).unwrap();

        assert!(view.source_snapshot_recovery.is_none());
        assert!(!view.source_snapshot_recovery_available);
        assert!(view
            .message
            .as_deref()
            .is_some_and(|message| message.contains("2 copie(s) audio retenue(s) récupérée(s)")));
    }

    #[test]
    fn close_removes_durable_queued_work_before_reporting_ready() {
        let mut app = ImportApplication::new(ScanTestIo::default());
        let request = ImportRequest {
            strategy: crate::parakeet_session::TranscriptionStrategy::BoundedWindowsV1,
            job_id: JobId(992),
            generation: Generation::first(),
            source_path: "source.wav".into(),
            source_sha256: None,
            source_samples: None,
            model_path: "model.nmodel".into(),
            model_sha256: [2; 32],
            destination: "archive".into(),
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Cpu,
            },
        };
        app.view.queue.push(ImportQueueEntry {
            sequence: 1,
            request: request.clone(),
            status: ImportQueueStatus::Queued,
        });
        app.view.queued_jobs = 1;

        app.dispatch(AppCommand::RequestClose).unwrap();

        assert!(!app.view.close_ready);
        assert!(matches!(
            app.io.effects.as_slice(),
            [ImportEffect::RemoveQueued { job_id, generation, destination }]
                if *job_id == request.job_id
                    && *generation == request.generation
                    && destination == "archive"
        ));
        app.io.events.push_back(ImportEvent::Queue(Vec::new()));
        app.dispatch(AppCommand::Refresh).unwrap();
        assert!(app.view.close_ready);
    }

    #[test]
    fn close_keeps_failed_queue_removal_recoverable_and_retries_it() {
        let mut app = ImportApplication::new(ScanTestIo::default());
        let request = ImportRequest {
            strategy: crate::parakeet_session::TranscriptionStrategy::BoundedWindowsV1,
            job_id: JobId(996),
            generation: Generation::first(),
            source_path: "source.wav".into(),
            source_sha256: None,
            source_samples: None,
            model_path: "model.nmodel".into(),
            model_sha256: [8; 32],
            destination: "archive".into(),
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Cpu,
            },
        };
        app.view.queue.push(ImportQueueEntry {
            sequence: 1,
            request: request.clone(),
            status: ImportQueueStatus::Queued,
        });
        app.view.queued_jobs = 1;

        app.dispatch(AppCommand::RequestClose).unwrap();
        app.io.events.push_back(ImportEvent::Failed {
            job_id: request.job_id,
            generation: request.generation,
            message: "source binding still in use".into(),
        });
        let view = app.dispatch(AppCommand::Refresh).unwrap();

        assert!(!view.close_ready);
        assert_eq!(
            view.queue.len(),
            1,
            "failed removal keeps queued work visible"
        );
        assert!(app.close_queue_removals.is_empty());
        assert!(view.message.as_deref().is_some_and(|message| {
            message.contains("source binding still in use")
                && message.contains("La file est conservée")
        }));

        app.dispatch(AppCommand::RequestClose).unwrap();
        assert!(matches!(
            app.io.effects.last(),
            Some(ImportEffect::RemoveQueued { job_id, generation, .. })
                if *job_id == request.job_id && *generation == request.generation
        ));
        app.io.events.push_back(ImportEvent::Queue(Vec::new()));
        assert!(app.dispatch(AppCommand::Refresh).unwrap().close_ready);
    }

    #[test]
    fn request_close_waits_for_an_already_committing_import_result() {
        let mut io = ScanTestIo::default();
        io.stop_committing = true;
        let mut app = ImportApplication::new(io);
        let job_id = JobId(995);
        let generation = Generation::first();
        let request = ImportRequest {
            strategy: crate::parakeet_session::TranscriptionStrategy::LegacyV1,
            job_id,
            generation,
            source_path: "source.wav".into(),
            source_sha256: Some([3; 32]),
            source_samples: Some(16_000),
            model_path: "model.bin".into(),
            model_sha256: [4; 32],
            destination: "archive".into(),
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Cpu,
            },
        };
        app.dispatch(AppCommand::StartImport { request }).unwrap();

        app.dispatch(AppCommand::RequestClose).unwrap();
        assert!(!app.view.close_ready);
        assert_eq!(app.active, Some(CommandIdentity { job_id, generation }));
        assert_eq!(app.view.active_job, Some((job_id, JobState::Finalizing)));

        app.io
            .events
            .push_back(ImportEvent::Published { job_id, generation });
        app.dispatch(AppCommand::Refresh).unwrap();
        assert!(app.view.close_ready);
        assert_eq!(app.view.active_job, Some((job_id, JobState::Complete)));
    }
