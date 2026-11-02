    #[test]
    fn enqueue_then_stop_still_targets_the_live_identity() {
        let recorder = RecordingApplication::default();
        let mut app = DesktopApp::new(recorder.clone());
        let live = (JobId(20), Generation::first());
        app.active_identity = Some(live);
        app.start_import();
        assert_eq!(app.active_identity, Some(live));
        assert!(matches!(
            recorder.0.borrow().first(),
            Some(AppCommand::EnqueueImport { .. })
        ));
        app.stop_active();
        assert_eq!(
            recorder.0.borrow().last(),
            Some(&AppCommand::Stop {
                job_id: live.0,
                generation: live.1,
            })
        );
    }

    #[test]
    fn import_uses_selected_gpu() {
        let recorder = RecordingApplication::default();
        let mut app = DesktopApp::new(recorder.clone());
        assert_eq!(app.compute_choice, ComputeChoice::Cpu);
        app.compute_choice = ComputeChoice::Gpu;
        app.start_import();
        assert!(matches!(
            recorder.0.borrow().first(),
            Some(AppCommand::EnqueueImport { request })
                if request.config.compute == ComputeChoice::Gpu
        ));
    }

    #[test]
    fn pending_and_late_gpu_inventory_does_not_change_compute_or_gate_transcription() {
        for choice in [ComputeChoice::Auto, ComputeChoice::Cpu, ComputeChoice::Gpu] {
            let recorder = RecordingApplication::default();
            let port = ControlledGpuInventory::default();
            let responses = Rc::clone(&port.0);
            let mut app = DesktopApp::new(recorder.clone())
                .with_gpu_inventory(GpuInventoryApp::new(Box::new(port)));
            app.paths.audio = "sample.wav".into();
            app.paths.model = "model.bin".into();
            app.paths.destination = "output".into();
            app.compute_choice = choice;

            app.start_import();
            app.start_live(Generation::first());
            assert_eq!(app.compute_choice, choice);
            assert!(matches!(
                app.gpu_inventory.as_ref().unwrap().state(),
                GpuInventoryState::Loading
            ));
            {
                let commands = recorder.0.borrow();
                assert!(
                    matches!(&commands[0], AppCommand::EnqueueImport { request } if request.config.compute == choice)
                );
                assert!(
                    matches!(&commands[1], AppCommand::StartDirectLiveConfigured { request } if request.config.compute == choice)
                );
            }

            responses.borrow_mut().push_back((
                1,
                Ok(GpuCollection {
                    observations: vec![GpuObservation {
                        identity: "late-gpu".into(),
                        name: Some("Late GPU".into()),
                        state: Some(GpuState::Ready),
                        presence: GpuPresence::Current,
                    }],
                    error: None,
                }),
            ));
            app.gpu_inventory.as_mut().unwrap().poll();

            assert!(matches!(
                app.gpu_inventory.as_ref().unwrap().state(),
                GpuInventoryState::Complete(_)
            ));
            assert_eq!(app.compute_choice, choice);
            let commands = recorder.0.borrow();
            assert!(
                matches!(&commands[0], AppCommand::EnqueueImport { request } if request.config.compute == choice)
            );
            assert!(
                matches!(&commands[1], AppCommand::StartDirectLiveConfigured { request } if request.config.compute == choice)
            );
        }
    }

    #[test]
    fn auto_preference_is_sent_to_both_paths_without_becoming_cpu() {
        let recorder = RecordingApplication::default();
        let mut app = DesktopApp::new(recorder.clone());
        app.compute_choice = ComputeChoice::Auto;
        app.start_import();
        app.start_live(Generation::first());
        let commands = recorder.0.borrow();
        assert!(matches!(&commands[0], AppCommand::EnqueueImport { request }
            if request.config.compute == ComputeChoice::Auto));
        assert!(
            matches!(&commands[1], AppCommand::StartDirectLiveConfigured { request }
            if request.config.compute == ComputeChoice::Auto)
        );
        assert_eq!(app.compute_choice, ComputeChoice::Auto);
    }

    #[test]
    fn cuda_startup_selects_gpu_for_import_and_microphone() {
        let recorder = RecordingApplication::default();
        let mut app = DesktopApp::with_startup_compute(recorder.clone(), ComputeChoice::Gpu, None);
        assert_eq!(app.compute_choice, ComputeChoice::Gpu);
        app.start_import();
        app.start_live(Generation::first());
        let commands = recorder.0.borrow();
        assert!(
            matches!(&commands[0], AppCommand::EnqueueImport { request } if request.config.compute == ComputeChoice::Gpu)
        );
        assert!(
            matches!(&commands[1], AppCommand::StartDirectLiveConfigured { request } if request.config.compute == ComputeChoice::Gpu)
        );
    }

    #[test]
    fn asynchronous_gpu_probe_sets_the_existing_startup_choice() {
        let (sender, receiver) = std::sync::mpsc::channel();
        let mut app = DesktopApp::with_startup_compute(
            RecordingApplication::default(),
            ComputeChoice::Cpu,
            Some("Vérification du GPU en cours…".into()),
        )
        .with_gpu_probe(receiver);
        app.poll_gpu_probe();
        assert!(app.gpu_probe.is_some());
        sender.send(Ok(())).unwrap();
        app.poll_gpu_probe();
        assert!(app.gpu_probe.is_none());
        assert_eq!(app.compute_choice, ComputeChoice::Gpu);
        assert!(app.startup_message.is_none());
    }

    #[test]
    fn unavailable_cuda_leaves_cpu_usable_and_explains_choice() {
        let recorder = RecordingApplication::default();
        let message = "GPU indisponible : choisissez CPU".to_owned();
        let mut app = DesktopApp::with_startup_compute(
            recorder.clone(),
            ComputeChoice::Cpu,
            Some(message.clone()),
        );
        assert_eq!(app.startup_message.as_deref(), Some(message.as_str()));
        app.start_import();
        app.start_live(Generation::first());
        let commands = recorder.0.borrow();
        assert!(
            matches!(&commands[0], AppCommand::EnqueueImport { request } if request.config.compute == ComputeChoice::Cpu)
        );
        assert!(
            matches!(&commands[1], AppCommand::StartDirectLiveConfigured { request } if request.config.compute == ComputeChoice::Cpu)
        );
    }

    #[test]
    fn gpu_startup_notice_uses_plain_language_and_hides_raw_failure_from_summary() {
        let technical_error =
            "GPU indisponible : WorkerExited without End: worker exited without End";
        assert!(technical_error.contains("WorkerExited without End"));
        assert_eq!(
            crate::ui::presentation::gpu_startup_notice_message(true, true),
            Some("GPU indisponible pour la transcription. Vous pouvez utiliser le CPU.")
        );
        assert!(!"GPU indisponible pour la transcription. Vous pouvez utiliser le CPU."
            .contains(technical_error));
        assert_eq!(
            crate::ui::presentation::gpu_startup_notice_message(false, true),
            Some("La disponibilité du GPU n’a pas pu être confirmée. Vous pouvez utiliser le CPU.")
        );
        assert_eq!(
            crate::ui::presentation::gpu_startup_notice_message(false, false),
            None
        );
    }

    fn gpu_inventory_response(response: Result<GpuCollection, String>) -> GpuInventoryApp {
        let responses = Rc::new(RefCell::new(VecDeque::from([(1, response)])));
        let mut inventory = GpuInventoryApp::new(Box::new(ControlledGpuInventory(responses)));
        inventory.poll();
        inventory
    }

    fn gpu_observation() -> GpuObservation {
        GpuObservation {
            identity: "pci:0000:00:02.0".into(),
            name: Some("Intel Iris Xe Graphics".into()),
            state: Some(GpuState::Ready),
            presence: GpuPresence::Current,
        }
    }

    fn render_gpu_notice(
        context: &egui::Context,
        startup_message: Option<&str>,
        inventory: Option<&GpuInventoryApp>,
    ) -> (String, Option<egui::Rect>) {
        let mut header_rect = None;
        let output = context.run_ui(Default::default(), |ui| {
            header_rect = crate::ui::presentation::gpu_startup_notices_ui(
                ui,
                startup_message,
                inventory,
            )
            .map(|response| response.rect);
        });
        fn collect(shape: &egui::Shape, text: &mut String) {
            match shape {
                egui::Shape::Text(shape) => {
                    text.push_str(&shape.galley.job.text);
                    text.push('\n');
                }
                egui::Shape::Vec(shapes) => shapes.iter().for_each(|shape| collect(shape, text)),
                _ => {}
            }
        }
        let mut text = String::new();
        for clipped in output.shapes {
            collect(&clipped.shape, &mut text);
        }
        (text, header_rect)
    }

    fn immediate_egui_context() -> egui::Context {
        let context = egui::Context::default();
        context.all_styles_mut(|style| style.animation_time = 0.0);
        context
    }

    fn click_gpu_notice_header(
        context: &egui::Context,
        rect: egui::Rect,
        startup_message: Option<&str>,
        inventory: Option<&GpuInventoryApp>,
    ) -> String {
        let position = rect.center();
        let _ = context.run_ui(
            egui::RawInput {
                events: vec![
                    egui::Event::PointerMoved(position),
                    egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: Default::default(),
                    },
                ],
                ..Default::default()
            },
            |ui| {
                crate::ui::presentation::gpu_startup_notices_ui(ui, startup_message, inventory);
            },
        );
        let output = context.run_ui(
            egui::RawInput {
                events: vec![
                    egui::Event::PointerMoved(position),
                    egui::Event::PointerButton {
                        pos: position,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers: Default::default(),
                    },
                ],
                ..Default::default()
            },
            |ui| {
                crate::ui::presentation::gpu_startup_notices_ui(ui, startup_message, inventory);
            },
        );
        let mut text = String::new();
        for clipped in output.shapes {
            match &clipped.shape {
                egui::Shape::Text(shape) => {
                    text.push_str(&shape.galley.job.text);
                    text.push('\n');
                }
                egui::Shape::Vec(shapes) => shapes.iter().for_each(|shape| {
                    if let egui::Shape::Text(shape) = shape {
                        text.push_str(&shape.galley.job.text);
                        text.push('\n');
                    }
                }),
                _ => {}
            }
        }
        text
    }

    #[test]
    fn gpu_startup_and_partial_inventory_diagnostics_are_collapsed_and_expand_on_request() {
        let worker_error =
            "GPU indisponible : WorkerExited without End: worker exited without End";
        let inventory_error = "Transport GPU interrompu : collecte impossible";
        let inventory = gpu_inventory_response(Ok(GpuCollection {
            observations: vec![gpu_observation()],
            error: Some(inventory_error.into()),
        }));
        assert!(matches!(inventory.state(), GpuInventoryState::Partial(_)));

        let context = immediate_egui_context();
        let (collapsed, header) = render_gpu_notice(&context, Some(worker_error), Some(&inventory));
        assert!(collapsed.contains("GPU indisponible pour la transcription. Vous pouvez utiliser le CPU."));
        assert!(collapsed.contains("Détails de la détection GPU"));
        assert!(!collapsed.contains("WorkerExited without End"));
        assert!(!collapsed.contains(inventory_error));

        let expanded = click_gpu_notice_header(
            &context,
            header.expect("diagnostic details header is rendered"),
            Some(worker_error),
            Some(&inventory),
        );
        assert!(expanded.contains("WorkerExited without End"));
        assert!(expanded.contains(inventory_error));
    }

    #[test]
    fn failed_inventory_is_collapsed_while_available_and_loading_states_remain_distinct() {
        let error = "nvidia-smi indisponible : code 255";
        let failed = gpu_inventory_response(Err(error.into()));
        let context = egui::Context::default();
        let (collapsed, header) = render_gpu_notice(&context, None, Some(&failed));
        assert!(collapsed.contains("La disponibilité du GPU n’a pas pu être confirmée."));
        assert!(!collapsed.contains(error));
        assert!(header.is_some());

        let context = egui::Context::default();
        let loading = GpuInventoryApp::new(Box::new(ControlledGpuInventory::default()));
        let (loading_text, header) = render_gpu_notice(&context, Some("Vérification du GPU en cours…"), Some(&loading));
        assert!(loading_text.contains("Vérification du GPU en cours"));
        assert!(loading_text.contains("Inventaire GPU en cours"));
        assert!(!loading_text.contains("GPU indisponible pour la transcription"));
        assert!(header.is_none());

        let complete = gpu_inventory_response(Ok(GpuCollection {
            observations: vec![gpu_observation()],
            error: None,
        }));
        let (complete_text, header) = render_gpu_notice(&context, None, Some(&complete));
        assert!(complete_text.contains("Inventaire GPU complet selon Windows"));
        assert!(!complete_text.contains("GPU indisponible pour la transcription"));
        assert!(header.is_none());
    }

    #[test]
    fn microphone_uses_selected_gpu() {
        let recorder = RecordingApplication::default();
        let mut app = DesktopApp::new(recorder.clone());
        app.compute_choice = ComputeChoice::Gpu;
        app.start_live(Generation::first());
        assert!(matches!(
            recorder.0.borrow().first(),
            Some(AppCommand::StartDirectLiveConfigured { request })
                if request.config.compute == ComputeChoice::Gpu
        ));
    }

    #[test]
    fn microphone_labels_changed_windows_default_notice_keeps_identity_and_gain() {
        let recorder = RecordingApplication::default();
        let default_id = Rc::new(RefCell::new("wasapi:A".to_owned()));
        let paths = whisper_core::path_settings::PathSettingsApp::new(
            whisper_core::path_settings::PathDefaults {
                audio_directory: "audio".into(),
                model_path: "model".into(),
                destination: "output".into(),
            },
            Box::new(whisper_core::path_settings::NullPathPort),
        );
        let mut app = DesktopApp::with_microphones(
            recorder.clone(),
            ComputeChoice::Cpu,
            None,
            paths,
            Box::new(ChangingDefaultMicrophone(default_id.clone())),
        );
        let a = whisper_core::microphone_level::MicrophoneIdentity::new("wasapi:A").unwrap();
        let b = whisper_core::microphone_level::MicrophoneIdentity::new("wasapi:B").unwrap();
        app.paths
            .set_microphone_gain(&a, whisper_core::GainDb::new(12).unwrap());
        app.paths
            .set_microphone_gain(&b, whisper_core::GainDb::new(-12).unwrap());

        app.start_microphone_test();
        assert!(
            matches!(recorder.0.borrow().last(), Some(AppCommand::StartMicrophoneTest {
            microphone: MicrophoneChoice::Device(id), gain_db: 12, ..
        }) if id == "wasapi:A")
        );
        app.view.microphone_test_active = false;
        app.view.active_job = None;
        *default_id.borrow_mut() = "wasapi:B".into();

        let count = recorder.0.borrow().len();
        app.start_live(Generation::first());
        assert_eq!(
            recorder.0.borrow().len(),
            count,
            "first transition action only announces identity change"
        );
        assert!(app.microphone_notice.as_deref().is_some_and(|notice| {
            notice.contains("Microphone indisponible")
                && notice.contains("Microphone Windows")
                && !notice.contains("wasapi:")
        }));

        app.start_live(Generation::first());
        assert!(
            matches!(recorder.0.borrow().last(), Some(AppCommand::StartDirectLiveConfigured {
            request: whisper_core::ports::LiveRequest { microphone: MicrophoneChoice::Device(id), microphone_gain_db: -12, .. }
        }) if id == "wasapi:B")
        );
    }

    #[test]
    fn gain_gesture_during_capture_open_targets_the_pinned_identity() {
        let recorder = RecordingApplication::default();
        let paths = whisper_core::path_settings::PathSettingsApp::new(
            whisper_core::path_settings::PathDefaults {
                audio_directory: "audio".into(),
                model_path: "model".into(),
                destination: "output".into(),
            },
            Box::new(whisper_core::path_settings::NullPathPort),
        );
        let mut app = DesktopApp::with_microphones(
            recorder.clone(),
            ComputeChoice::Cpu,
            None,
            paths,
            Box::new(OneMicrophone),
        );
        app.view.direct_capture_phase =
            Some(whisper_core::application::DirectCapturePhase::Initializing);
        app.active_identity = Some((JobId(910), Generation::first()));
        app.set_microphone_gain(6);
        assert!(matches!(
            recorder.0.borrow().last(),
            Some(AppCommand::SetMicrophoneGain {
                job_id: JobId(910), expected_revision: 0, gain, target_identity, ..
            }) if target_identity == "wasapi:chosen" && gain.value() == 6
        ));
    }

    #[test]
    fn new_microphone_start_gets_a_new_group_while_continuation_keeps_its_identity() {
        let recorder = RecordingApplication::default();
        let mut app = DesktopApp::new(recorder.clone());
        app.start_live(Generation::first());
        let first = app.live_request.clone().unwrap();
        app.start_live(Generation::first());
        let second = app.live_request.clone().unwrap();
        assert_ne!(first.job_id, second.job_id);
        assert_eq!(second.group_offset_samples, 0);
        app.start_live(second.generation.next().unwrap());
        let continued = app.live_request.as_ref().unwrap();
        assert_eq!(continued.job_id, second.job_id);
        assert_eq!(continued.generation.get(), 2);
        assert_eq!(recorder.0.borrow().len(), 3);
    }

    #[test]
    fn first_render_timestamp_is_recorded_after_the_label_in_an_egui_frame() {
        let context = eframe::egui::Context::default();
        let mut identity = None;
        let mut first_render = None;
        let _ = context.run_ui(Default::default(), |ui| {
            eframe::egui::CentralPanel::default().show(ui, |ui| {
                let label = ui.label("provisional transcript");
                record_first_render(
                    (JobId(21), Generation::first(), 1),
                    &label,
                    &mut identity,
                    &mut first_render,
                );
            });
        });
        assert_eq!(identity, Some((JobId(21), Generation::first(), 1)));
        assert!(first_render.is_some());
    }

    #[test]
    fn first_effective_action_is_not_recorded_when_application_rejects_start() {
        let mut app = DesktopApp::new(RejectStartApplication);
        app.start_live(Generation::first());
        assert!(app.live_clicked_at_unix_ms.is_some());
        assert_eq!(
            app.view.message.as_deref(),
            Some("start admission rejected")
        );
    }

    #[test]
    fn changing_catalog_snapshots_regray_and_restore_parakeet_in_the_open_ui() {
        let (catalog_sender, catalog_receiver) = std::sync::mpsc::channel();
        let (_preference_event_sender, preference_events) =
            std::sync::mpsc::channel::<super::ModelPreferenceEvent>();
        let (preference_commands, _preference_command_receiver) =
            model_preference_command_channel();
        let mut app = DesktopApp::new(RecordingApplication::default())
            .with_model_selection_runtime(ModelSelectionRuntime {
                catalog: catalog_receiver,
                preference_events,
                preference_commands,
            });
        app.selection.model = Some(ModelId::ParakeetTdt06BV3);

        let snapshot = |generation, parakeet_availability| CatalogSnapshot {
            generation,
            entries: whisper_core::model_registry::MODELS
                .iter()
                .map(|record| {
                    let availability = if record.artifact.id == ModelId::WhisperLargeV3Turbo {
                        whisper_core::model_catalog::ModelAvailability::Ready
                    } else {
                        parakeet_availability
                    };
                    whisper_adapters::model_catalog::CatalogEntry {
                        id: record.artifact.id,
                        display_name: record.display_name.to_owned(),
                        path: std::path::PathBuf::from("dynamic-test-models")
                            .join(record.artifact.file_name),
                        availability,
                        expected_size_bytes: record.artifact.size_bytes,
                        observed_size_bytes: Some(record.artifact.size_bytes),
                        expected_sha256: record.artifact.sha256.to_owned(),
                        observed_sha256: Some(record.artifact.sha256.to_owned()),
                        diagnostic: None,
                    }
                })
                .collect(),
        };
        let is_selectable = |app: &DesktopApp<RecordingApplication>| {
            app.projection
                .as_ref()
                .unwrap()
                .models
                .iter()
                .find(|model| model.id == ModelId::ParakeetTdt06BV3)
                .unwrap()
                .selectable
        };

        catalog_sender
            .send(snapshot(
                1,
                whisper_core::model_catalog::ModelAvailability::Ready,
            ))
            .unwrap();
        app.poll_model_selection();
        assert!(is_selectable(&app));
        assert_eq!(app.selection.model, Some(ModelId::ParakeetTdt06BV3));

        catalog_sender
            .send(snapshot(
                2,
                whisper_core::model_catalog::ModelAvailability::Missing,
            ))
            .unwrap();
        app.poll_model_selection();
        assert!(!is_selectable(&app));
        assert_eq!(app.selection.model, Some(ModelId::ParakeetTdt06BV3));
        assert!(
            app.projection
                .as_ref()
                .unwrap()
                .models
                .iter()
                .find(|model| model.id == ModelId::WhisperLargeV3Turbo)
                .unwrap()
                .selectable
        );

        catalog_sender
            .send(snapshot(
                3,
                whisper_core::model_catalog::ModelAvailability::Ready,
            ))
            .unwrap();
        app.poll_model_selection();
        assert!(is_selectable(&app));
        assert_eq!(app.selection.model, Some(ModelId::ParakeetTdt06BV3));
    }

    #[test]
    fn gpu_compute_control_requires_a_ready_qualified_runtime_catalog_entry() {
        let record = whisper_core::model_registry::model(ModelId::ParakeetTdt06BV3);
        let entry = || whisper_adapters::model_catalog::CatalogEntry {
            id: ModelId::ParakeetTdt06BV3,
            display_name: record.display_name.to_owned(),
            path: std::path::PathBuf::from(record.artifact.file_name),
            availability: whisper_core::model_catalog::ModelAvailability::Ready,
            expected_size_bytes: record.artifact.size_bytes,
            observed_size_bytes: Some(record.artifact.size_bytes),
            expected_sha256: record.artifact.sha256.to_owned(),
            observed_sha256: Some(record.artifact.sha256.to_owned()),
            diagnostic: None,
        };
        let catalog = |entry| CatalogSnapshot {
            generation: 1,
            entries: vec![entry],
        };

        assert!(parakeet_gpu_catalog_ready(&catalog(entry())));

        let mut divergent = entry();
        divergent.diagnostic = Some("GPU indisponible : DLL différente".into());
        assert!(!parakeet_gpu_catalog_ready(&catalog(divergent)));

        let mut missing = entry();
        missing.availability = whisper_core::model_catalog::ModelAvailability::Missing;
        assert!(!parakeet_gpu_catalog_ready(&catalog(missing)));
    }
