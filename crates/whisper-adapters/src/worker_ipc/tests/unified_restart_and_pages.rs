struct ServiceRuntimeTestPort {
    service: Rc<RefCell<AsyncImportIo>>,
    runtime: Rc<RefCell<ImportRuntime>>,
    effects: std::sync::mpsc::Receiver<ImportEffect>,
    stops: std::sync::mpsc::Receiver<ImportEffect>,
    events: std::sync::mpsc::SyncSender<ImportEvent>,
    pending: VecDeque<ImportEvent>,
}

impl ImportIoPort for ServiceRuntimeTestPort {
    fn submit(&mut self, effect: ImportEffect) -> Result<(), PortError> {
        self.service.borrow_mut().submit(effect)?;
        while let Ok(effect) = self.effects.try_recv() {
            self.runtime.borrow_mut().effect(effect, &mut self.pending);
        }
        while let Ok(effect) = self.stops.try_recv() {
            self.runtime.borrow_mut().effect(effect, &mut self.pending);
        }
        Ok(())
    }

    fn poll(&mut self) -> Result<Option<ImportEvent>, PortError> {
        let event = self.pending.pop_front().or_else(|| {
            self.runtime.borrow_mut().poll().map(|result| match result {
                Ok(event) => event,
                Err((job_id, generation, message)) => ImportEvent::Failed {
                    job_id,
                    generation,
                    message,
                },
            })
        });
        if let Some(event) = event {
            self.events
                .try_send(event)
                .map_err(|_| PortError::Unavailable)?;
        }
        self.service.borrow_mut().poll()
    }
}
fn install_foreign_corrupt_archive(root: &Path) -> (PathBuf, Vec<u8>, String) {
    let job = JobId(0x2110);
    let generation = Generation::first();
    let source_sha = "ef".repeat(32);
    let dir = root
        .join("transcriptions")
        .join(format!("{:032x}", job.0))
        .join("1");
    fs::create_dir_all(&dir).unwrap();
    let text = b"expected historical text";
    let damaged = b"damaged historical text with extra bytes".to_vec();
    let srt = b"historical srt";
    fs::write(dir.join("transcript.txt"), &damaged).unwrap();
    fs::write(dir.join("transcript.srt"), srt).unwrap();
    let model =
        whisper_core::model_registry::model(whisper_core::model_catalog::ModelId::ParakeetTdt06BV3)
            .artifact;
    let pending = serde_json::json!({"version":4, "job_id":job.0, "generation":generation.get(),
        "source_path":"old.wav", "source_sha256":source_sha, "source_samples":16000,
        "model_path":"old-model.gguf", "model_sha256":model.sha256, "model_size":model.size_bytes,
        "strategy":whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1, "qualification":null});
    let artifact = |path: &str, bytes: &[u8]| serde_json::json!({"path":path,"sha256":format!("{:x}", Sha256::digest(bytes)),"bytes":bytes.len()});
    let manifest = serde_json::json!({"version":1,"job_id":job.0,"generation":generation.get(),
        "source_sha256":source_sha,"segments":1,"text":artifact("transcript.txt",text),"srt":artifact("transcript.srt",srt)});
    fs::write(
        dir.join("pending.json"),
        serde_json::to_vec(&pending).unwrap(),
    )
    .unwrap();
    fs::write(
        dir.join("manifest.json"),
        serde_json::to_vec(&manifest).unwrap(),
    )
    .unwrap();
    fs::write(dir.join("CURRENT"), b"manifest.json\n").unwrap();
    let index_dir = root.join(".whisper-publications");
    fs::create_dir_all(&index_dir).unwrap();
    let index = serde_json::json!({"version":1,"records":[{"job_id":job.0,"generation":generation.get(),"source_sha256":source_sha,"export_basename":null,"active":true,"success_ordinal":1}]});
    fs::write(
        index_dir.join("index.json"),
        serde_json::to_vec(&index).unwrap(),
    )
    .unwrap();
    (dir.join("transcript.txt"), damaged, source_sha)
}
#[test]
#[cfg_attr(
    not(feature = "native-tests"),
    ignore = "requires models/parakeet-tdt-0.6b-v3.q8_0.gguf and target/parakeet-tests/refacto-parakeet-live/p3-workers; mixed synthetic unified capture scenario"
)]
// TODO(DEBT-10): extract the synthetic scenario from its real model/catalogue prerequisite.
fn unified_capture_after_direct_stop_uses_its_own_gate_and_publishes_audio() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/parakeet-tests/refacto-parakeet-live/p3-workers")
        .join(format!("unified-restart-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let (foreign_file, foreign_bytes, foreign_sha) = install_foreign_corrupt_archive(&root);
    assert!(
        ArchiveStore::published_imports_for_strategy(
            &root,
            &foreign_sha,
            whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
        )
        .unwrap_err()
        .contains("StorageCorrupt:")
    );
    assert!(
        ArchiveStore::published_imports_for_strategy(
            &root,
            &foreign_sha.to_uppercase(),
            whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
        )
        .unwrap_err()
        .contains("StorageCorrupt:")
    );
    let gate = Arc::new(PublicationGate::default());
    let (mut service, effects, stops, events, _auto_effects, _auto_stops, _auto_events) =
        AsyncImportIo::for_test(Arc::clone(&gate));
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: PathBuf::from("unused-cpu"),
            gpu: PathBuf::from("unused-gpu"),
        },
        Arc::clone(&gate),
    )
    .with_controlled_transcriber(Box::new(TailAwareUnifiedWorker));
    let mut turbo = direct_live_request(JobId(0x98501));
    turbo.destination = root.join("turbo").to_string_lossy().into_owned();
    fs::create_dir_all(&turbo.destination).unwrap();
    service
        .submit(ImportEffect::StartDirectLive(turbo.clone()))
        .unwrap();
    assert!(matches!(
        effects.try_recv().unwrap(),
        ImportEffect::StartDirectLive(_)
    ));
    let mut direct_capture = CaptureStream::empty_test_capture(16_000, 1, "test:turbo");
    direct_capture.enqueue_test_block(vec![0.1; 16_000]);
    runtime.direct_live = Some(
        crate::direct_live::DirectLive::from_test_capture(turbo.clone(), direct_capture).unwrap(),
    );
    runtime
        .direct_live
        .as_mut()
        .unwrap()
        .pump_capture()
        .unwrap();
    service
        .submit(ImportEffect::Stop {
            job_id: turbo.job_id,
            generation: turbo.generation,
        })
        .unwrap();
    assert!(gate.is_cancelled());
    let mut pending = VecDeque::new();
    runtime.effect(stops.try_recv().unwrap(), &mut pending);
    assert!(
        pending
            .iter()
            .any(|event| matches!(event, ImportEvent::LiveFinalizing { .. }))
    );
    runtime
        .direct_live
        .as_mut()
        .unwrap()
        .finish_test_wav()
        .unwrap();
    runtime.direct_live.take();
    pending.clear();
    let request = unified_live_request(&root, 0x98502);
    let mut capture =
        CaptureStream::empty_test_capture(16_000, 1, &format!("test:unified-{}", request.job_id.0));
    capture.enqueue_test_block(vec![0.2; 32_000]);
    runtime = runtime.with_controlled_capture(capture, Some(32_000));
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    let runtime = Rc::new(RefCell::new(runtime));
    let service = Rc::new(RefCell::new(service));
    let mut app = ImportApplication::new(ServiceRuntimeTestPort {
        service: Rc::clone(&service),
        runtime: Rc::clone(&runtime),
        effects,
        stops,
        events,
        pending,
    });
    app.dispatch(AppCommand::StartUnifiedLiveConfigured {
        request: request.clone(),
    })
    .unwrap();
    assert!(
        gate.is_cancelled(),
        "the new capture must not rearm the Turbo gate"
    );
    assert!(
        runtime.borrow().unified_sessions.contains_key(&session),
        "start cannot silently omit its session"
    );
    assert!(matches!(
        service.borrow_mut().submit(ImportEffect::StopUnifiedLive {
            job_id: JobId(0xFFFF),
            generation: request.generation,
        }),
        Err(PortError::StaleResponse)
    ));
    let until = Instant::now() + Duration::from_secs(20);
    let mut captured = 0;
    let mut durable = 0;
    while Instant::now() < until && (captured == 0 || durable == 0) {
        app.dispatch(AppCommand::Refresh).unwrap();
        let view = app.view();
        if let Some(current) = view
            .unified_parakeet_sessions
            .iter()
            .find(|current| current.job_id == request.job_id)
        {
            captured = current.captured_samples;
            durable = current.durable_samples;
        }
        thread::sleep(Duration::from_millis(2));
    }
    assert!(captured > 0 && durable > 0);
    let first_durable = durable;
    let first = ArchiveStore::read_unified_audio_parts_page(
        &root,
        request.job_id,
        request.generation,
        0,
        0,
        128,
    )
    .unwrap();
    for _ in 0..100 {
        runtime
            .borrow_mut()
            .unified_sessions
            .get_mut(&session)
            .unwrap()
            .source
            .enqueue_test_block(vec![0.3; 320]);
        app.dispatch(AppCommand::Refresh).unwrap();
        thread::sleep(Duration::from_millis(20));
    }
    let telemetry_deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < telemetry_deadline {
        app.dispatch(AppCommand::Refresh).unwrap();
        let view = app.view();
        if view.microphone_meter.is_some() && view.microphone_spectrum.is_some() {
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }
    let view = app.view();
    assert_eq!(
        view.microphone_meter
            .as_ref()
            .map(|meter| (meter.job_id, meter.generation)),
        Some((request.job_id, request.generation))
    );
    assert_eq!(
        view.microphone_spectrum
            .as_ref()
            .map(|spectrum| (spectrum.job_id, spectrum.generation)),
        Some((request.job_id, request.generation))
    );

    let until = Instant::now() + Duration::from_secs(20);
    while Instant::now() < until && durable <= first_durable {
        app.dispatch(AppCommand::Refresh).unwrap();
        let view = app.view();
        if let Some(current) = view
            .unified_parakeet_sessions
            .iter()
            .find(|current| current.job_id == request.job_id)
        {
            captured = current.captured_samples;
            durable = current.durable_samples;
        }
        thread::sleep(Duration::from_millis(2));
    }
    assert!(durable > first_durable && durable <= 64_000);
    assert!(captured >= durable && captured <= 64_000);
    assert_eq!(
        ArchiveStore::read_unified_audio_parts_page(
            &root,
            request.job_id,
            request.generation,
            first.revision,
            0,
            128
        )
        .unwrap_err(),
        "StaleResponse: unified audio parts changed"
    );
    let fresh = ArchiveStore::read_unified_audio_parts_page(
        &root,
        request.job_id,
        request.generation,
        0,
        0,
        128,
    )
    .unwrap();
    assert_ne!(fresh.revision, first.revision);
    app.dispatch(AppCommand::StopUnifiedLive {
        job_id: request.job_id,
        generation: request.generation,
    })
    .unwrap();
    assert!(app.view().microphone_meter.is_none() && app.view().microphone_spectrum.is_none());
    let until = Instant::now() + Duration::from_secs(20);
    let mut completed = false;
    while Instant::now() < until {
        app.dispatch(AppCommand::Refresh).unwrap();
        let view = app.view();
        if let Some(current) = view
            .unified_parakeet_sessions
            .iter()
            .find(|current| current.job_id == request.job_id)
        {
            if current.phase == whisper_core::ports::UnifiedSessionPhase::Recoverable {
                panic!("capture finalization failed: {:?}", current.message);
            }
            if current.phase == whisper_core::ports::UnifiedSessionPhase::Complete {
                completed = true;
                break;
            }
        }
        thread::sleep(Duration::from_millis(2));
    }
    assert!(
        completed,
        "core must persist, publish and report Complete after Stop"
    );
    let files =
        ArchiveStore::published_unified_files(&root, request.job_id, request.generation).unwrap();
    assert!(Path::new(&files.txt_path).is_file());
    assert!(Path::new(&files.srt_path).is_file());
    let unknown = root
        .join("transcriptions")
        .join(format!("{:032x}", 0x2111_u128))
        .join("1");
    fs::create_dir_all(&unknown).unwrap();
    fs::write(unknown.join("pending.json"), b"invalid pending").unwrap();
    assert!(
        ArchiveStore::published_imports_for_strategy(
            &root,
            &"42".repeat(32),
            whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
        )
        .unwrap_err()
        .contains("StorageCorrupt:"),
        "unknown corrupt identity stays fail-closed"
    );
    let source = crate::parakeet_audio::PcmPartStore::open(&files.wav_path);
    assert_eq!(fs::read(&foreign_file).unwrap(), foreign_bytes);
    assert!(ArchiveStore::scan(&root).unwrap().iter().any(|entry| {
        entry.job_id == JobId(0x2110)
            && entry
                .diagnostic
                .as_deref()
                .is_some_and(|message| message.contains("StorageCorrupt:"))
    }));
    assert_eq!(source.total_samples().unwrap(), 64_000);
    assert_eq!(source.read_window(0..64_000).unwrap().len(), 64_000);
    assert!(gate.is_cancelled());
}

#[test]
#[cfg_attr(
    not(feature = "native-tests"),
    ignore = "requires models/parakeet-tdt-0.6b-v3.q8_0.gguf and target/parakeet-tests/refacto-parakeet-live/p3-workers; mixed synthetic unified capture scenario"
)]
// TODO(DEBT-10): extract the synthetic scenario from its real model/catalogue prerequisite.
fn unified_start_and_immediate_stop_keep_the_new_session_identity() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/parakeet-tests/refacto-parakeet-live/p3-workers")
        .join(format!("unified-start-stop-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let gate = Arc::new(PublicationGate::default());
    gate.activate();
    assert!(gate.request_stop().unwrap());
    let (mut service, effects, stops, _events, _auto_effects, _auto_stops, _auto_events) =
        AsyncImportIo::for_test(Arc::clone(&gate));
    let request = unified_live_request(&root, 0x98503);
    let mut capture = CaptureStream::empty_test_capture(16_000, 1, "test:immediate-stop");
    capture.enqueue_test_block(vec![0.2; 16_000]);
    let mut runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: PathBuf::from("unused-cpu"),
            gpu: PathBuf::from("unused-gpu"),
        },
        Arc::clone(&gate),
    )
    .with_controlled_transcriber(Box::new(TailAwareUnifiedWorker))
    .with_controlled_capture(capture, None);
    service
        .submit(ImportEffect::StartUnifiedLive(request.clone()))
        .unwrap();
    service
        .submit(ImportEffect::StopUnifiedLive {
            job_id: request.job_id,
            generation: request.generation,
        })
        .unwrap();
    let mut pending = VecDeque::new();
    runtime.effect(effects.try_recv().unwrap(), &mut pending);
    runtime.effect(stops.try_recv().unwrap(), &mut pending);
    assert!(
        !pending
            .iter()
            .any(|event| matches!(event, ImportEvent::Failed { .. })),
        "{pending:?}"
    );
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    assert!(
        runtime
            .unified_sessions
            .get(&session)
            .is_some_and(|state| state.source_closed)
    );
    assert!(gate.is_cancelled());
}

#[test]
#[cfg_attr(
    not(feature = "native-tests"),
    ignore = "requires models/parakeet-tdt-0.6b-v3.q8_0.gguf and target/parakeet-tests/refacto-parakeet-live/p3-workers; mixed synthetic unified capture scenario"
)]
// TODO(DEBT-10): extract the synthetic scenario from its real model/catalogue prerequisite.
fn unified_capture_alone_routes_probe_and_rejects_old_or_wrong_source_events() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../target/parakeet-tests/refacto-parakeet-live/p3-workers")
        .join(format!("unified-probe-{}", std::process::id()));
    fs::create_dir(&root).unwrap();
    let gate = Arc::new(PublicationGate::default());
    let (service, effects, stops, events, _auto_effects, _auto_stops, _auto_events) =
        AsyncImportIo::for_test(Arc::clone(&gate));
    let inject = events.clone();
    let request = unified_live_request(&root, 0x98509);
    let source_identity = format!("test:unified-{}", request.job_id.0);
    let mut capture = CaptureStream::empty_test_capture(16_000, 1, &source_identity);
    capture.enqueue_test_block(vec![0.2; 32_000]);
    let runtime = ImportRuntime::new(
        WorkerPaths {
            cpu: PathBuf::from("unused-cpu"),
            gpu: PathBuf::from("unused-gpu"),
        },
        gate,
    )
    .with_controlled_transcriber(Box::new(TailAwareUnifiedWorker))
    .with_controlled_capture(capture, None);
    let runtime = Rc::new(RefCell::new(runtime));
    let mut app = ImportApplication::new(ServiceRuntimeTestPort {
        service: Rc::new(RefCell::new(service)),
        runtime: Rc::clone(&runtime),
        effects,
        stops,
        events,
        pending: VecDeque::new(),
    });
    app.dispatch(AppCommand::StartUnifiedLiveConfigured {
        request: request.clone(),
    })
    .unwrap();
    let session = whisper_core::parakeet_scheduler::SchedulerSession {
        job_id: request.job_id,
        generation: request.generation,
    };
    feed_capture_until_probes_arrive(|enqueue| {
        if enqueue {
            runtime
                .borrow_mut()
                .unified_sessions
                .get_mut(&session)
                .unwrap()
                .source
                .enqueue_test_block(vec![0.2; 320]);
        }
        app.dispatch(AppCommand::Refresh).unwrap();
        let view = app.view();
        view.microphone_meter.is_some() && view.microphone_spectrum.is_some()
    });
    let view = app.view();
    let meter = view
        .microphone_meter
        .clone()
        .expect("unified meter arrives");
    let spectrum = view
        .microphone_spectrum
        .clone()
        .expect("unified spectrum arrives");
    assert_probe_identity(
        (
            meter.job_id,
            meter.generation,
            meter.source_identity.as_str(),
        ),
        (request.job_id, request.generation, source_identity.as_str()),
    );
    assert_probe_identity(
        (
            spectrum.job_id,
            spectrum.generation,
            spectrum.source_identity.as_str(),
        ),
        (request.job_id, request.generation, source_identity.as_str()),
    );
    assert_ignored_old_meter(&inject, meter.clone(), request.job_id, || {
        app.dispatch(AppCommand::Refresh).unwrap();
        app.view().microphone_meter.as_ref().unwrap().job_id
    });
    assert_ignored_wrong_spectrum(&inject, spectrum, &source_identity, || {
        app.dispatch(AppCommand::Refresh).unwrap();
        app.view()
            .microphone_spectrum
            .as_ref()
            .unwrap()
            .source_identity
            .clone()
    });
    app.dispatch(AppCommand::StopUnifiedLive {
        job_id: request.job_id,
        generation: request.generation,
    })
    .unwrap();
    assert!(app.view().microphone_meter.is_none() && app.view().microphone_spectrum.is_none());
}

fn assert_probe_identity(actual: (JobId, Generation, &str), expected: (JobId, Generation, &str)) {
    assert_eq!(actual, expected);
}

fn feed_capture_until_probes_arrive(mut refresh: impl FnMut(bool) -> bool) {
    for _ in 0..100 {
        refresh(true);
        thread::sleep(Duration::from_millis(20));
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if refresh(false) {
            break;
        }
        thread::sleep(Duration::from_millis(5));
    }
}

fn assert_ignored_old_meter(
    inject: &std::sync::mpsc::SyncSender<ImportEvent>,
    meter: whisper_core::microphone_level::MicrophoneMeter,
    expected_job_id: JobId,
    refresh_and_current_job: impl FnOnce() -> JobId,
) {
    let mut old_meter = meter;
    old_meter.job_id = JobId(expected_job_id.0 - 1);
    old_meter.sequence += 100;
    inject
        .try_send(ImportEvent::MicrophoneMeter(old_meter))
        .unwrap();
    assert_eq!(refresh_and_current_job(), expected_job_id);
}

fn assert_ignored_wrong_spectrum(
    inject: &std::sync::mpsc::SyncSender<ImportEvent>,
    mut spectrum: whisper_core::microphone_level::MicrophoneSpectrum,
    expected_source_identity: &str,
    refresh_and_current_identity: impl FnOnce() -> String,
) {
    spectrum.source_identity = "test:wrong".into();
    spectrum.sequence += 100;
    inject
        .try_send(ImportEvent::MicrophoneSpectrum(
            whisper_core::microphone_level::MicrophoneSpectrumUpdate::Available(spectrum),
        ))
        .unwrap();
    assert_eq!(refresh_and_current_identity(), expected_source_identity);
}

#[test]
fn corrupt_archive_alias_cannot_borrow_canonical_foreign_identity() {
    let root = test_artifact_root("parakeet-archive-alias");
    fs::create_dir_all(root.parent().unwrap()).unwrap();
    fs::create_dir(&root).unwrap();
    let (canonical_file, canonical_bytes, _) = install_foreign_corrupt_archive(&root);
    let unrelated_sha = "42".repeat(32);
    let strategy = whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1;
    assert!(
        ArchiveStore::published_imports_for_strategy(&root, &unrelated_sha, strategy).is_ok(),
        "unambiguous foreign corruption may be skipped"
    );
    let alias = root.join("transcriptions/2110/1");
    fs::create_dir_all(&alias).unwrap();
    let alias_file = alias.join("pending.json");
    let alias_bytes = b"untrusted alias pending";
    fs::write(&alias_file, alias_bytes).unwrap();
    let error =
        ArchiveStore::published_imports_for_strategy(&root, &unrelated_sha, strategy).unwrap_err();
    assert!(error.contains("StorageCorrupt:"), "{error}");
    assert_eq!(fs::read(&canonical_file).unwrap(), canonical_bytes);
    assert_eq!(fs::read(&alias_file).unwrap(), alias_bytes);
    fs::remove_dir_all(&root).unwrap();
}
