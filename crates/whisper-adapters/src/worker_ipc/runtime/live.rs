use super::*;

pub(in crate::worker_ipc) enum LiveEffect {
    Return,
    Continue(Result<(), (JobId, Generation, String)>),
}

pub(in crate::worker_ipc) fn spawn_live_attempt(
    request: &LiveRequest,
    selection: &crate::supervisor::WorkerSelection,
    candidate: Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
) -> Result<(ChildWorkerTransport, u64), (JobId, Generation, String)> {
    let identity = (request.job_id, request.generation);
    let instance_id = NEXT_INSTANCE.fetch_add(1, Ordering::Relaxed);
    if instance_id == 0 {
        return Err((
            identity.0,
            identity.1,
            "WorkerExited: instance identity exhausted".into(),
        ));
    }
    crate::archive::campaign_gate(
        "live.before_spawn",
        identity.0.0,
        identity.1.get(),
        instance_id,
    )
    .map_err(|error| (identity.0, identity.1, error))?;
    let mut worker_config = request.config.clone();
    worker_config.compute = match selection.backend {
        whisper_core::BackendAttempt::Cpu => whisper_core::ComputeChoice::Cpu,
        whisper_core::BackendAttempt::Gpu => whisper_core::ComputeChoice::Gpu,
    };
    let spawn_result = (|| -> Result<ChildWorkerTransport, String> {
        let mut transport = ChildWorkerTransport::spawn(&selection.path)
            .map_err(|error| format!("WorkerExited: {error:?}"))?;
        let command = match candidate {
            Some(candidate) if selection.backend == whisper_core::BackendAttempt::Gpu => {
                WorkerCommand::StartGpuLiveWorker {
                    job_id: identity.0,
                    generation: identity.1,
                    instance_id,
                    model_path: request.model_path.clone(),
                    model_sha256: request.model_sha256,
                    config: worker_config,
                    candidate,
                }
            }
            _ => WorkerCommand::StartLiveWorker {
                job_id: identity.0,
                generation: identity.1,
                instance_id,
                model_path: request.model_path.clone(),
                model_sha256: request.model_sha256,
                config: worker_config,
            },
        };
        transport
            .request(command)
            .map_err(|error| format!("ProtocolMismatch: {error:?}"))?;
        Ok(transport)
    })();
    crate::archive::record_campaign_operation_result(
        "live.before_spawn",
        identity.0.0,
        identity.1.get(),
        instance_id,
        &spawn_result,
        None,
    )
    .map_err(|error| (identity.0, identity.1, error))?;
    let transport = spawn_result.map_err(|error| (identity.0, identity.1, error))?;
    crate::archive::campaign_gate(
        "live.after_spawn",
        identity.0.0,
        identity.1.get(),
        instance_id,
    )
    .map_err(|error| (identity.0, identity.1, error))?;
    Ok((transport, instance_id))
}

pub(in crate::worker_ipc) struct MicrophoneTestCapture {
    pub(in crate::worker_ipc) job_id: JobId,
    pub(in crate::worker_ipc) generation: Generation,
    pub(in crate::worker_ipc) capture: crate::capture::CaptureStream,
    converter: Pcm16Converter,
    pub(in crate::worker_ipc) probe: crate::microphone_probe::MicrophoneProbe,
    spectrum_reader: crate::microphone_probe::MicrophoneSpectrumReader,
    last_meter_sequence: u64,
    next_spectrum_telemetry: bool,
    last_acknowledged_gain_revision: Option<u64>,
    pending_events: VecDeque<ImportEvent>,
}

impl MicrophoneTestCapture {
    pub(in crate::worker_ipc) fn start(
        job_id: JobId,
        generation: Generation,
        microphone: &whisper_core::microphone::MicrophoneChoice,
        gain_db: i8,
    ) -> Result<(Self, String), String> {
        let gain = whisper_core::microphone_level::GainDb::new(gain_db)
            .map_err(|error| format!("MicrophoneLevel: {error}"))?;
        let capture = crate::capture::start_direct_with_gain(microphone, gain)?;
        Self::from_capture(job_id, generation, capture)
    }

    pub(in crate::worker_ipc) fn from_capture(
        job_id: JobId,
        generation: Generation,
        capture: crate::capture::CaptureStream,
    ) -> Result<(Self, String), String> {
        let source_identity = capture
            .source_identity()
            .map(str::to_owned)
            .ok_or_else(|| "MicrophoneLevel: identité du flux ouvert indisponible".to_owned())?;
        let converter = Pcm16Converter::new(capture.sample_rate_hz(), capture.channels())?;
        let probe = crate::microphone_probe::MicrophoneProbe::start(
            capture.sample_rate_hz(),
            Some(source_identity.clone()),
            job_id,
            generation,
        )?;
        Ok((
            Self {
                job_id,
                generation,
                capture,
                converter,
                probe,
                spectrum_reader: crate::microphone_probe::MicrophoneSpectrumReader::default(),
                last_meter_sequence: 0,
                next_spectrum_telemetry: false,
                last_acknowledged_gain_revision: None,
                pending_events: VecDeque::new(),
            },
            source_identity,
        ))
    }

    pub(in crate::worker_ipc) fn set_gain(
        &self,
        target_identity: &str,
        expected_revision: u64,
        gain: whisper_core::microphone_level::GainDb,
    ) -> Result<whisper_core::microphone_level::GainRevision, String> {
        if self.capture.source_identity() != Some(target_identity) {
            return Err("MicrophoneLevel: source active différente de la cible".into());
        }
        self.capture.set_gain(expected_revision, gain)
    }

    pub(in crate::worker_ipc) fn pump(&mut self) -> Result<Option<ImportEvent>, String> {
        if self.capture.is_saturated() {
            return Err("Tampon du test microphone saturé".into());
        }
        if self.capture.failure() {
            return Err(self
                .capture
                .failure_detail()
                .unwrap_or_else(|| "Flux du test microphone interrompu".into()));
        }
        for _ in 0..crate::capture::CAPTURE_SLOTS {
            let Some(block) = self.capture.try_next_block() else {
                break;
            };
            let gain = block.gain();
            let first_frame = block.first_frame();
            if self
                .last_acknowledged_gain_revision
                .is_none_or(|revision| gain.revision > revision)
            {
                self.last_acknowledged_gain_revision = Some(gain.revision);
                if let Some(target_identity) = self.capture.source_identity().map(str::to_owned) {
                    self.pending_events
                        .push_back(ImportEvent::MicrophoneGainApplied {
                            job_id: self.job_id,
                            generation: self.generation,
                            target_identity,
                            gain_db: gain.gain.value(),
                            gain_revision: gain.revision,
                            first_frame,
                        });
                }
            }
            let converted = self.converter.push_with_probe(&block)?;
            self.capture.recycle_block(block)?;
            self.probe.submit(converted.probe);
        }
        if let Some(event) = self.pending_events.pop_front() {
            return Ok(Some(event));
        }
        if self.next_spectrum_telemetry {
            self.next_spectrum_telemetry = false;
            if let Some(update) = self.spectrum_reader.poll(&self.probe, Instant::now()) {
                return Ok(Some(ImportEvent::MicrophoneSpectrum(update)));
            }
        } else {
            self.next_spectrum_telemetry = true;
        }
        let Some(meter) = self
            .probe
            .latest_at(Instant::now())
            .and_then(|snapshot| snapshot.meter())
        else {
            return Ok(None);
        };
        if meter.sequence <= self.last_meter_sequence {
            return Ok(None);
        }
        self.last_meter_sequence = meter.sequence;
        Ok(Some(ImportEvent::MicrophoneMeter(meter)))
    }

    pub(in crate::worker_ipc) fn stop(mut self) -> ImportEvent {
        self.probe.invalidate();
        self.capture.stop();
        ImportEvent::MicrophoneTestStopped {
            job_id: self.job_id,
            generation: self.generation,
        }
    }
}

impl ImportRuntime {
    pub(in crate::worker_ipc) fn start_live(
        &mut self,
        request: LiveRequest,
        candidate: Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
    ) -> Result<(), (JobId, Generation, String)> {
        let id = (request.job_id, request.generation);
        crate::archive::assert_campaign_archive(Path::new(&request.destination))
            .map_err(|error| (id.0, id.1, error))?;
        if self.active.is_some() || self.live.is_some() || self.pending_live.is_some() {
            return Err((
                id.0,
                id.1,
                "Busy: another live/import session is active".into(),
            ));
        }
        let selection = self
            .workers
            .select(
                if request.config.compute == whisper_core::ComputeChoice::Auto
                    && candidate.is_none()
                {
                    whisper_core::ComputeChoice::Cpu
                } else {
                    request.config.compute
                },
            )
            .map_err(|error| {
                (
                    id.0,
                    id.1,
                    format!("BackendUnavailable: compute policy rejected request ({error:?})"),
                )
            })?;
        self.auto_fallback_reason = (request.config.compute == whisper_core::ComputeChoice::Auto
            && selection.backend == whisper_core::BackendAttempt::Cpu
            && candidate.is_none())
        .then(|| "GPU identity proof unavailable before processing".to_owned());
        if selection.backend == whisper_core::BackendAttempt::Gpu && candidate.is_none() {
            return Err((
                id.0,
                id.1,
                "BackendMismatch: selected GPU has no current identity proof".into(),
            ));
        }
        self.start_diagnostics(Path::new(&request.destination));
        self.diagnostic_degraded_reported = false;
        self.progress = Some(ProgressSupervisor::starting(std::time::Instant::now()));
        self.live_pending_persistence = 0;
        self.pending_attempt_reset = None;
        self.pending_recovery_counters = None;
        self.deferred_live_stopped = None;
        self.deferred_live_window_finished = None;
        let directory = PathBuf::from(&request.destination)
            .join("transcriptions")
            .join(format!("{:032x}", request.job_id.0))
            .join("live");
        fs::create_dir_all(&directory)
            .map_err(|e| (id.0, id.1, format!("StorageUnavailable: {e}")))?;
        let live_archive = LiveArchive::prepare(&request.destination, request.clone())
            .map_err(|e| (id.0, id.1, e))?;
        let mut live_archive = live_archive;
        let attempt_number = campaign_reserve_attempt(&mut live_archive, id.0, id.1)
            .map_err(|error| (id.0, id.1, error))?;
        let staging =
            PcmStaging::create(directory.join(format!("passage-{}.pcm", request.generation.get())))
                .map_err(|e| (id.0, id.1, format!("StorageUnavailable: PCM staging: {e}")))?
                .with_campaign_identity(id.0.0, id.1.get())
                .into_writer();
        let (transport, instance_id) = spawn_live_attempt(&request, &selection, candidate.clone())?;
        live_archive
            .mark_attempt_started(attempt_number)
            .map_err(|error| (id.0, id.1, error))?;
        self.active = Some(ImportRequestState {
            job_id: id.0,
            generation: id.1,
            instance_id,
            backend: match selection.backend {
                whisper_core::BackendAttempt::Cpu => BackendKind::Cpu,
                whisper_core::BackendAttempt::Gpu => BackendKind::Cuda,
            },
            attempt_number,
            import_request: None,
            recovery: LiveRecoveryState::None,
            startup_ready: false,
            gpu_candidate: candidate.clone(),
            gpu_attested: false,
        });
        if let Some(diagnostics) = &self.diagnostics {
            diagnostics.record(DiagnosticRecord::new(
                id.0.0,
                id.1.get(),
                DiagnosticPhase::Preparing,
                DiagnosticCode::WorkerStarted,
                Some(match selection.backend {
                    whisper_core::BackendAttempt::Cpu => "cpu",
                    whisper_core::BackendAttempt::Gpu => "cuda",
                }),
                None,
            ));
        }
        self.transport = Some(transport);
        self.pending_live = Some(PendingLiveState {
            request,
            staging,
            live_archive,
            recovery: LiveRecoveryState::None,
        });
        Ok(())
    }

    pub(in crate::worker_ipc) fn activate_live_after_ready(
        &mut self,
    ) -> Result<(), (JobId, Generation, String)> {
        let Some(pending) = self.pending_live.take() else {
            return Ok(());
        };
        let (job_id, generation) = (pending.request.job_id, pending.request.generation);
        let initial_gain =
            whisper_core::microphone_level::GainDb::new(pending.request.microphone_gain_db)
                .map_err(|error| (job_id, generation, error.to_string()))?;
        let mut capture = crate::capture::start_default_with_gain(initial_gain)
            .map_err(|error| (job_id, generation, error))?;
        let converter = match Pcm16Converter::new(capture.sample_rate_hz(), capture.channels()) {
            Ok(converter) => converter,
            Err(error) => {
                capture.stop();
                return Err((job_id, generation, error));
            }
        };
        let probe = crate::microphone_probe::MicrophoneProbe::start(
            capture.sample_rate_hz(),
            capture.source_identity().map(str::to_owned),
            pending.request.job_id,
            pending.request.generation,
        )
        .map_err(|error| (job_id, generation, error))?;
        let mut live_archive = pending.live_archive;
        if let Err(error) = live_archive.capture_started() {
            capture.stop();
            return Err((job_id, generation, error));
        }
        self.live_archive = Some(live_archive);
        self.live = Some(LiveCaptureState {
            request: pending.request,
            capture,
            probe,
            spectrum_reader: crate::microphone_probe::MicrophoneSpectrumReader::default(),
            last_meter_sequence: 0,
            next_spectrum_telemetry: false,
            telemetry_due_after_event: false,
            converter,
            vad: VoiceActivity::new(),
            staging: pending.staging,
            frame_pending: Vec::with_capacity(320),
            inference_cursor: 0,
            inference_sequence: 1,
            pending_window: None,
            replay_until: 0,
            replay_windows_queued: 0,
            replay_windows_to_skip: 0,
            segment_sequence_base: 1,
            window_sequence_base: 1,
            recovery: LiveRecoveryState::None,
            barrier_m: None,
            inference_admission_closed: false,
            worker_ready: true,
            stop_requested: false,
            stop_enqueued: false,
            started_at_unix_ms: unix_time_ms(),
            captured: 0,
            admitted: 0,
            speech: 0,
            confirmed_fragment_end: 0,
            last_reported_durable: 0,
            failure: None,
        });
        Ok(())
    }
    pub(in crate::worker_ipc) fn start_cpu_live_attempt(
        &mut self,
        job_id: JobId,
        generation: Generation,
    ) -> Result<ImportEvent, (JobId, Generation, String)> {
        if self.publication_gate.is_cancelled() {
            return Err((
                job_id,
                generation,
                "Cancelled: CPU retry suppressed by latched Stop/Quit".into(),
            ));
        }
        let request = self
            .live
            .as_ref()
            .filter(|live| (live.request.job_id, live.request.generation) == (job_id, generation))
            .map(|live| live.request.clone())
            .ok_or((
                job_id,
                generation,
                "StaleResponse: live passage is unavailable".into(),
            ))?;
        let selection = self
            .workers
            .select(whisper_core::ComputeChoice::Cpu)
            .map_err(|error| {
                (
                    job_id,
                    generation,
                    format!("BackendUnavailable: CPU fallback unavailable ({error:?})"),
                )
            })?;
        let (confirmed_samples, next_segment_sequence, next_window_sequence, attempt_number) = {
            let archive = self.live_archive.as_mut().ok_or((
                job_id,
                generation,
                "StorageUnavailable: live archive is missing".into(),
            ))?;
            archive
                .discard_unconfirmed_segments()
                .and_then(|()| archive.restart_mp3_attempt())
                .map_err(|error| (job_id, generation, error))?;
            let confirmed_samples = archive.confirmed_samples();
            let next_segment_sequence = archive.next_segment_sequence();
            let next_window_sequence = archive.next_window_sequence();
            let attempt_number = campaign_reserve_attempt(archive, job_id, generation)
                .map_err(|error| (job_id, generation, error))?;
            (
                confirmed_samples,
                next_segment_sequence,
                next_window_sequence,
                attempt_number,
            )
        };
        let (transport, instance_id) = match spawn_live_attempt(&request, &selection, None) {
            Ok(spawned) => spawned,
            Err(error) => {
                if let Some(live) = self.live.as_mut() {
                    live.stop_capture();
                    let _ = live.staging.drain();
                }
                self.live = None;
                self.live_archive = None;
                self.active = None;
                self.transport = None;
                return Err(error);
            }
        };
        self.live_archive
            .as_mut()
            .ok_or((
                job_id,
                generation,
                "StorageUnavailable: live archive disappeared after CPU spawn".into(),
            ))?
            .mark_attempt_started(attempt_number)
            .map_err(|error| (job_id, generation, error))?;
        self.active = Some(ImportRequestState {
            job_id,
            generation,
            instance_id,
            backend: BackendKind::Cpu,
            attempt_number,
            import_request: None,
            recovery: LiveRecoveryState::None,
            startup_ready: false,
            gpu_candidate: None,
            gpu_attested: false,
        });
        self.transport = Some(transport);
        let live = self.live.as_mut().ok_or((
            job_id,
            generation,
            "StorageUnavailable: live capture state disappeared".into(),
        ))?;
        live.replay_until = confirmed_samples;
        live.replay_windows_queued = 0;
        live.replay_windows_to_skip = next_window_sequence.saturating_sub(1);
        live.segment_sequence_base = next_segment_sequence;
        live.window_sequence_base = next_window_sequence;
        live.pending_window = None;
        live.inference_cursor = 0;
        live.inference_sequence = 1;
        live.inference_admission_closed = false;
        live.worker_ready = false;
        live.stop_enqueued = false;
        live.recovery = LiveRecoveryState::None;
        live.barrier_m = None;
        if let Some(diagnostics) = &self.diagnostics {
            diagnostics.record(DiagnosticRecord::new(
                job_id.0,
                generation.get(),
                DiagnosticPhase::Preparing,
                DiagnosticCode::WorkerStarted,
                Some("cpu"),
                None,
            ));
        }
        self.pending_attempt_reset = Some(ImportEvent::LiveAttemptReset {
            job_id,
            generation,
            confirmed_samples,
            next_segment_sequence,
        });
        Ok(ImportEvent::LiveAttemptStarting { job_id, generation })
    }

    pub(in crate::worker_ipc) fn start_cpu_pending_live_attempt(
        &mut self,
        job_id: JobId,
        generation: Generation,
    ) -> Result<ImportEvent, (JobId, Generation, String)> {
        if self.publication_gate.is_cancelled() {
            return Err((
                job_id,
                generation,
                "Cancelled: CPU retry suppressed by latched Stop/Quit".into(),
            ));
        }
        let (request, attempt_number) = {
            let pending = self
                .pending_live
                .as_mut()
                .filter(|pending| {
                    (pending.request.job_id, pending.request.generation) == (job_id, generation)
                })
                .ok_or((
                    job_id,
                    generation,
                    "StaleResponse: pending live passage is unavailable".into(),
                ))?;
            pending
                .live_archive
                .restart_mp3_attempt()
                .map_err(|error| (job_id, generation, error))?;
            (
                pending.request.clone(),
                campaign_reserve_attempt(&mut pending.live_archive, job_id, generation)
                    .map_err(|error| (job_id, generation, error))?,
            )
        };
        let selection = self
            .workers
            .select(whisper_core::ComputeChoice::Cpu)
            .map_err(|error| {
                (
                    job_id,
                    generation,
                    format!("BackendUnavailable: CPU fallback unavailable ({error:?})"),
                )
            })?;
        let (transport, instance_id) = spawn_live_attempt(&request, &selection, None)?;
        self.pending_live
            .as_mut()
            .ok_or((
                job_id,
                generation,
                "StaleResponse: pending live passage is unavailable".into(),
            ))?
            .live_archive
            .mark_attempt_started(attempt_number)
            .map_err(|error| (job_id, generation, error))?;
        self.active = Some(ImportRequestState {
            job_id,
            generation,
            instance_id,
            backend: BackendKind::Cpu,
            attempt_number,
            import_request: None,
            recovery: LiveRecoveryState::None,
            startup_ready: false,
            gpu_candidate: None,
            gpu_attested: false,
        });
        self.transport = Some(transport);
        if let Some(pending) = self.pending_live.as_mut() {
            pending.recovery = LiveRecoveryState::None;
        }
        Ok(ImportEvent::LiveAttemptStarting { job_id, generation })
    }

    pub(in crate::worker_ipc) fn finish_live_on_cpu(
        &mut self,
        job_id: JobId,
        generation: Generation,
    ) -> Result<ImportEvent, (JobId, Generation, String)> {
        if self.pending_live.as_ref().is_some_and(|pending| {
            (pending.request.job_id, pending.request.generation) == (job_id, generation)
                && pending.recovery == LiveRecoveryState::AwaitingChoice
        }) {
            return self.start_cpu_pending_live_attempt(job_id, generation);
        }
        if !self.live.as_ref().is_some_and(|live| {
            (live.request.job_id, live.request.generation) == (job_id, generation)
                && live.recovery == LiveRecoveryState::AwaitingChoice
        }) {
            return Err((
                job_id,
                generation,
                "InvalidInput: CPU finish requires a pending user choice".into(),
            ));
        }
        self.start_cpu_live_attempt(job_id, generation)
    }

    pub(in crate::worker_ipc) fn start_microphone_test(
        &mut self,
        job_id: JobId,
        generation: Generation,
        microphone: whisper_core::microphone::MicrophoneChoice,
        gain_db: i8,
        events: &mut VecDeque<ImportEvent>,
    ) -> Result<(), (JobId, Generation, String)> {
        if self.microphone_test.is_some()
            || self.direct_live.is_some()
            || self.parakeet_live.is_some()
            || self.live.is_some()
            || self.active.is_some()
        {
            Err((
                job_id,
                generation,
                "Une capture microphone est déjà ouverte".into(),
            ))
        } else {
            match MicrophoneTestCapture::start(job_id, generation, &microphone, gain_db) {
                Ok((capture, source_identity)) => {
                    self.microphone_test = Some(capture);
                    events.push_back(ImportEvent::MicrophoneTestStarted {
                        job_id,
                        generation,
                        source_identity,
                        gain_db,
                    });
                    Ok(())
                }
                Err(error) => Err((job_id, generation, error)),
            }
        }
    }
    pub(in crate::worker_ipc) fn start_direct_live(
        &mut self,
        request: LiveRequest,
        events: &mut VecDeque<ImportEvent>,
    ) -> LiveEffect {
        LiveEffect::Continue({
            let id = (request.job_id, request.generation);
            let candidate = self.gpu_candidates.remove(&(id.0, id.1.get())).flatten();
            let parakeet = match self.model_catalog.as_ref() {
                Some(catalog) => match catalog.is_parakeet_live_request(&request) {
                    Ok(is_parakeet) => is_parakeet,
                    Err(error) => {
                        events.push_back(ImportEvent::Failed {
                            job_id: id.0,
                            generation: id.1,
                            message: format!("Modèle indisponible : {error}"),
                        });
                        return LiveEffect::Return;
                    }
                },
                None => false,
            };
            let automatic_import_active = parakeet
                && self.active.as_ref().is_some_and(|active| {
                    self.automatic_final_outputs
                        .contains_key(&(active.job_id, active.generation.get()))
                });
            if self.publication_gate.is_cancelled() {
                Ok(())
            } else if (self.active.is_some() && !automatic_import_active)
                || self.live.is_some()
                || self.direct_live.is_some()
                || self.parakeet_live.is_some()
            {
                Err((id.0, id.1, "Une transcription est déjà en cours".into()))
            } else {
                if parakeet {
                    let catalog = self.model_catalog.as_ref().expect("catalog checked above");
                    match catalog.bind_parakeet_live(&request, candidate) {
                        Ok((request, paths, candidate)) => {
                            let session_id = NEXT_INSTANCE.fetch_add(1, Ordering::Relaxed);
                            match crate::parakeet_live::ParakeetLive::start(
                                request, paths, candidate, session_id,
                            ) {
                                Ok(session) => {
                                    self.parakeet_live = Some(session);
                                    Ok(())
                                }
                                Err(error) => Err((id.0, id.1, error)),
                            }
                        }
                        Err(error) => Err((id.0, id.1, format!("Modèle indisponible : {error}"))),
                    }
                } else {
                    let request = match self.bind_live_request(&request) {
                        Ok(request) => request,
                        Err(error) => {
                            events.push_back(ImportEvent::Failed {
                                job_id: id.0,
                                generation: id.1,
                                message: format!("Modèle indisponible : {error}"),
                            });
                            return LiveEffect::Return;
                        }
                    };
                    let instance = NEXT_INSTANCE.fetch_add(1, Ordering::Relaxed);
                    let reusable_worker = self.idle_worker.take().and_then(|mut idle| {
                        idle.matches(&request, &self.workers)
                            .then_some(idle.transport)
                    });
                    match crate::direct_live::DirectLive::start(
                        request,
                        &self.workers,
                        instance,
                        reusable_worker,
                        candidate,
                    ) {
                        Ok(session) => {
                            self.direct_live = Some(session);
                            Ok(())
                        }
                        Err(error) => Err((id.0, id.1, error)),
                    }
                }
            }
        })
    }
    pub(in crate::worker_ipc) fn start_unified_live(
        &mut self,
        request: LiveRequest,
    ) -> Result<(), (JobId, Generation, String)> {
        let id = (request.job_id, request.generation);
        let candidate = self
            .gpu_candidates
            .remove(&(request.job_id, request.generation.get()))
            .flatten();
        if self.publication_gate.is_cancelled() {
            Ok(())
        } else if self.native_unified_candidate_enabled {
            let result = (|| {
                let bound = self
                            .model_catalog
                            .as_ref()
                            .ok_or_else(|| "ModelCatalogUnavailable: native capture requires the personal model catalogue".to_owned())?
                            .bind_native_unified_live(&request)?;
                let import_request = ImportRequest {
                    strategy:
                        whisper_core::parakeet_session::TranscriptionStrategy::BoundedWindowsV1,
                    job_id: bound.job_id,
                    generation: bound.generation,
                    source_path: bound.model_path.clone(),
                    source_sha256: None,
                    source_samples: None,
                    model_path: bound.model_path.clone(),
                    model_sha256: bound.model_sha256,
                    destination: bound.destination.clone(),
                    config: bound.config.clone(),
                };
                let route = native_unified_route(
                    &import_request,
                    self.parakeet_paths.as_ref(),
                    self.gpu_parakeet_paths.as_ref(),
                    candidate.as_ref(),
                )?;
                self.validate_unified_capture_admission(&bound, Some(&route))?;

                #[cfg(test)]
                if let Some(capture) = self.controlled_captures.pop_front() {
                    let part_limit = self.controlled_part_limits.pop_front().flatten();
                    return self.start_unified_capture(&bound, capture, part_limit, Some(route));
                }

                #[cfg(test)]
                {
                    Err(
                        "ModelNotQualified: controlled capture injection is required in tests"
                            .into(),
                    )
                }
                #[cfg(not(test))]
                {
                    self.start_native_unified_capture_with(
                        &bound,
                        route,
                        crate::capture::start_direct_with_gain,
                    )
                }
            })();
            result.map_err(|error| (id.0, id.1, error))
        } else {
            let result = (|| {
                let bound = self
                    .bind_live_request(&request)
                    .map_err(|error| format!("Modèle indisponible : {error}"))?;
                self.validate_unified_capture_admission(&bound, None)?;
                #[cfg(test)]
                if let Some(capture) = self.controlled_captures.pop_front() {
                    let part_limit = self.controlled_part_limits.pop_front().flatten();
                    return self.start_unified_capture(&bound, capture, part_limit, None);
                }
                Err("ModelNotQualified: native microphone capture remains closed until T9 qualification; use the controlled capture harness".into())
            })();
            result.map_err(|error| (id.0, id.1, error))
        }
    }
    pub(in crate::worker_ipc) fn start_legacy_live(
        &mut self,
        request: LiveRequest,
    ) -> Result<(), (JobId, Generation, String)> {
        let id = (request.job_id, request.generation);
        let candidate = self
            .gpu_candidates
            .remove(&(request.job_id, request.generation.get()))
            .flatten();
        if self.publication_gate.is_cancelled() {
            Ok(())
        } else {
            match self.bind_live_request(&request) {
                Ok(request) => self.start_live(request, candidate),
                Err(error) => Err((id.0, id.1, format!("Modèle indisponible : {error}"))),
            }
        }
    }
    pub(in crate::worker_ipc) fn set_microphone_gain(
        &mut self,
        job_id: JobId,
        generation: Generation,
        target_identity: String,
        expected_revision: u64,
        gain_db: i8,
        events: &mut VecDeque<ImportEvent>,
    ) -> Result<(), (JobId, Generation, String)> {
        let result = whisper_core::microphone_level::GainDb::new(gain_db)
            .map_err(|error| error.to_string())
            .and_then(|gain| {
                let unified =
                    whisper_core::parakeet_scheduler::SchedulerSession { job_id, generation };
                if let Some(state) = self.unified_sessions.get(&unified)
                    && state.source.is_capture()
                    && !state.source_closed
                {
                    self.unified_sessions
                        .get(&unified)
                        .expect("unified capture was found")
                        .source
                        .set_gain(&target_identity, expected_revision, gain)
                        .map(|_| ())
                } else if let Some(direct) = self.direct_live.as_ref()
                    && direct.identity() == (job_id, generation)
                {
                    direct
                        .set_gain(&target_identity, expected_revision, gain)
                        .map(|_| ())
                } else if let Some(parakeet) = self.parakeet_live.as_ref()
                    && parakeet.identity() == (job_id, generation)
                {
                    parakeet
                        .set_gain(&target_identity, expected_revision, gain)
                        .map(|_| ())
                } else if let Some(test) = self.microphone_test.as_ref()
                    && (test.job_id, test.generation) == (job_id, generation)
                {
                    test.set_gain(&target_identity, expected_revision, gain)
                        .map(|_| ())
                } else if let Some(live) = self.live.as_ref()
                    && (live.request.job_id, live.request.generation) == (job_id, generation)
                    && live.capture.source_identity() == Some(target_identity.as_str())
                {
                    live.capture.set_gain(expected_revision, gain).map(|_| ())
                } else {
                    Err("MicrophoneLevel: capture cible absente ou périmée".into())
                }
            });
        match result {
            Ok(applied) => {
                let _ = applied;
                Ok(())
            }
            Err(message) => {
                events.push_back(ImportEvent::MicrophoneGainRejected {
                    job_id,
                    generation,
                    target_identity,
                    message,
                });
                Ok(())
            }
        }
    }
    pub(in crate::worker_ipc) fn finish_live_on_cpu_effect(
        &mut self,
        job_id: JobId,
        generation: Generation,
        events: &mut VecDeque<ImportEvent>,
    ) -> Result<(), (JobId, Generation, String)> {
        match self.finish_live_on_cpu(job_id, generation) {
            Ok(event) => {
                events.push_back(event);
                Ok(())
            }
            Err(error) => Err(error),
        }
    }
    pub(in crate::worker_ipc) fn stop_unified_live_effect(
        &mut self,
        job_id: JobId,
        generation: Generation,
        events: &mut VecDeque<ImportEvent>,
    ) -> Result<(), (JobId, Generation, String)> {
        let session = whisper_core::parakeet_scheduler::SchedulerSession { job_id, generation };
        let publication_gate = self.unified_publication_gates.get(&session).cloned();
        let stopped = if let Some(publication_gate) = publication_gate {
            match publication_gate.request_stop() {
                Ok(_) => {
                    let stopped = self.stop_controlled_unified_capture(job_id, generation, events);
                    publication_gate.rollback_stop();
                    stopped
                }
                Err(error) => Err(format!("Publication: {error:?}")),
            }
        } else {
            self.stop_controlled_unified_capture(job_id, generation, events)
        };
        stopped.map_err(|error| (job_id, generation, error))
    }
    pub(in crate::worker_ipc) fn stop_microphone_test(
        &mut self,
        job_id: JobId,
        generation: Generation,
        events: &mut VecDeque<ImportEvent>,
    ) -> Result<(), (JobId, Generation, String)> {
        match self.microphone_test.take() {
            Some(test) if (test.job_id, test.generation) == (job_id, generation) => {
                events.push_back(test.stop());
                Ok(())
            }
            Some(test) => {
                self.microphone_test = Some(test);
                Err((job_id, generation, "Arrêt du test microphone périmé".into()))
            }
            None => Err((job_id, generation, "Test microphone absent".into())),
        }
    }
}
