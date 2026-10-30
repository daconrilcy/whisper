use super::*;

impl ImportRuntime {
    pub(in crate::worker_ipc) fn start_worker_with_candidate(
        &mut self,
        request: whisper_core::ports::ImportRequest,
        candidate: Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
    ) -> Result<(), (JobId, Generation, String)> {
        self.start_worker_with_candidate_override(request, false, candidate)
    }

    pub(in crate::worker_ipc) fn start_worker_with_cpu_override(
        &mut self,
        request: whisper_core::ports::ImportRequest,
        force_cpu: bool,
    ) -> Result<(), (JobId, Generation, String)> {
        self.start_worker_with_candidate_override(request, force_cpu, None)
    }

    pub(in crate::worker_ipc) fn start_worker_with_candidate_override(
        &mut self,
        request: whisper_core::ports::ImportRequest,
        force_cpu: bool,
        candidate: Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
    ) -> Result<(), (JobId, Generation, String)> {
        let parakeet_sha = decode_sha256(
            whisper_core::model_registry::model(
                whisper_core::model_catalog::ModelId::ParakeetTdt06BV3,
            )
            .artifact
            .sha256,
        )
        .map_err(|error| (request.job_id, request.generation, error))?;
        if request.model_sha256 == parakeet_sha {
            return self.start_parakeet_worker(request, force_cpu, candidate);
        }
        let id = (request.job_id, request.generation);
        if !force_cpu {
            self.auto_fallback_reason = None;
        }
        crate::archive::assert_campaign_archive(Path::new(&request.destination))
            .map_err(|error| (id.0, id.1, error))?;
        if self.publication_gate.is_cancelled() {
            return Err((
                id.0,
                id.1,
                "Cancelled: generation invalidated before worker start".into(),
            ));
        }
        let source_sha256 = request.source_sha256.ok_or((
            id.0,
            id.1,
            "SourceMissing: source identity was not prepared".into(),
        ))?;
        let expected_source_samples = request.source_samples.ok_or((
            id.0,
            id.1,
            "InvalidInput: prepared WAV duration is missing".into(),
        ))?;
        let model_sha256 = request.model_sha256;
        self.start_diagnostics(Path::new(&request.destination));
        self.diagnostic_degraded_reported = false;
        self.progress = Some(ProgressSupervisor::starting(std::time::Instant::now()));
        let selection = self
            .workers
            .select(if force_cpu {
                whisper_core::ComputeChoice::Cpu
            } else {
                request.config.compute
            })
            .map_err(|error| {
                (
                    id.0,
                    id.1,
                    format!("BackendUnavailable: compute policy rejected request ({error:?})"),
                )
            })?;
        let expected_backend = match selection.backend {
            whisper_core::BackendAttempt::Cpu => BackendKind::Cpu,
            whisper_core::BackendAttempt::Gpu => BackendKind::Cuda,
        };
        if request.config.compute == whisper_core::ComputeChoice::Auto
            && expected_backend == BackendKind::Cpu
            && !force_cpu
        {
            self.auto_fallback_reason = Some("worker CUDA absent".into());
        }
        let mut worker_config = request.config.clone();
        if expected_backend == BackendKind::Cpu {
            worker_config.compute = whisper_core::ComputeChoice::Cpu;
        }
        let cached = self.idle_worker.take().and_then(|mut idle| {
            idle.matches_identity(worker_config.compute, &request.model_path, model_sha256)
                .then_some(idle.transport)
        });
        let mut transport = match cached
            .map(Ok)
            .unwrap_or_else(|| ChildWorkerTransport::spawn(&selection.path))
        {
            Ok(transport) => transport,
            Err(error)
                if request.config.compute == whisper_core::ComputeChoice::Auto
                    && expected_backend == BackendKind::Cuda
                    && !force_cpu =>
            {
                self.auto_fallback_reason = Some(format!("lancement CUDA impossible : {error:?}"));
                return self.start_worker_with_cpu_override(request, true);
            }
            Err(error) => return Err((id.0, id.1, format!("WorkerExited: {error:?}"))),
        };
        let instance_id = NEXT_INSTANCE.fetch_add(1, Ordering::Relaxed);
        if instance_id == 0 {
            return Err((
                id.0,
                id.1,
                "WorkerExited: instance identity exhausted".into(),
            ));
        }
        let retry_request = request.clone();
        let attested_candidate = candidate.clone();
        let command = match candidate {
            Some(candidate) if expected_backend == BackendKind::Cuda && !force_cpu => {
                WorkerCommand::StartGpuImport {
                    retain_after_publish: true,
                    job_id: id.0,
                    generation: id.1,
                    instance_id,
                    source_path: request.source_path,
                    source_sha256,
                    expected_source_samples,
                    model_path: request.model_path,
                    model_sha256,
                    config: worker_config,
                    candidate,
                }
            }
            _ => WorkerCommand::StartImport {
                retain_after_publish: true,
                job_id: id.0,
                generation: id.1,
                instance_id,
                source_path: request.source_path,
                source_sha256,
                expected_source_samples,
                model_path: request.model_path,
                model_sha256,
                config: worker_config,
            },
        };
        if let Err(error) = transport.request(command) {
            transport.abort();
            if request.config.compute == whisper_core::ComputeChoice::Auto
                && expected_backend == BackendKind::Cuda
                && !force_cpu
            {
                self.auto_fallback_reason = Some(format!("commande CUDA refusée : {error:?}"));
                return self.start_worker_with_cpu_override(retry_request, true);
            }
            return Err((id.0, id.1, format!("ProtocolMismatch: {error:?}")));
        }
        self.active = Some(ImportRequestState {
            job_id: id.0,
            generation: id.1,
            instance_id,
            backend: expected_backend,
            attempt_number: 0,
            import_request: Some(retry_request),
            recovery: LiveRecoveryState::None,
            startup_ready: false,
            gpu_candidate: attested_candidate,
            gpu_attested: false,
        });
        if let Some(diagnostics) = &self.diagnostics {
            diagnostics.record(DiagnosticRecord::new(
                id.0.0,
                id.1.get(),
                DiagnosticPhase::Preparing,
                DiagnosticCode::WorkerStarted,
                Some(match expected_backend {
                    BackendKind::Cpu => "cpu",
                    BackendKind::Cuda => "cuda",
                }),
                None,
            ));
        }
        self.transport = Some(transport);
        Ok(())
    }

    pub(in crate::worker_ipc) fn start_parakeet_worker(
        &mut self,
        request: whisper_core::ports::ImportRequest,
        force_cpu: bool,
        candidate: Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
    ) -> Result<(), (JobId, Generation, String)> {
        let id = (request.job_id, request.generation);
        if self.active.is_some() || self.parakeet.is_some() || self.transport.is_some() {
            return Err((id.0, id.1, "Busy: another worker is active".into()));
        }
        if !matches!(
            &request.config.language,
            whisper_core::LanguageChoice::Automatic
        ) {
            return Err((
                id.0,
                id.1,
                "ModelNotQualified: Parakeet requiert la langue automatique".into(),
            ));
        }
        if request.source_sha256.is_none() {
            return Err((
                id.0,
                id.1,
                "SourceMissing: source identity was not prepared".into(),
            ));
        }
        let source_samples = request.source_samples.ok_or((
            id.0,
            id.1,
            "InvalidInput: source duration was not prepared".into(),
        ))?;
        if source_samples == 0 {
            return Err((id.0, id.1, "InvalidInput: source duration is empty".into()));
        }
        if self.publication_gate.is_cancelled() {
            return Err((
                id.0,
                id.1,
                "Cancelled: generation invalidated before worker start".into(),
            ));
        }
        let record = whisper_core::model_registry::model(
            whisper_core::model_catalog::ModelId::ParakeetTdt06BV3,
        );
        let profile =
            if source_samples > whisper_core::parakeet_ipc::PARAKEET_MAX_AUDIO_SAMPLES as u64 {
                whisper_core::parakeet_windows::PROFILE_IMPORT_VAD
            } else {
                whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS
            };
        if profile != whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS
            || source_samples
                > whisper_core::model_registry::PARAKEET_MAX_SINGLE_PASS_DURATION_MS * 16
        {
            return Err((
                id.0,
                id.1,
                "DurationUnsupported: Fichier trop long pour Parakeet, utilisez Turbo".into(),
            ));
        }
        let auto = request.config.compute == whisper_core::ComputeChoice::Auto && !force_cpu;
        let candidate_is_current = candidate
            .as_ref()
            .is_some_and(|candidate| candidate.generation == id.1.get());
        let mut use_gpu = !force_cpu
            && match request.config.compute {
                whisper_core::ComputeChoice::Cpu => false,
                whisper_core::ComputeChoice::Gpu => true,
                whisper_core::ComputeChoice::Auto => candidate_is_current,
            };
        if request.config.compute == whisper_core::ComputeChoice::Gpu && !candidate_is_current {
            return Err((
                id.0,
                id.1,
                "BackendMismatch: selected GPU has no current runtime identity proof".into(),
            ));
        }
        let mut expected_gpu = if use_gpu {
            match candidate
                .as_ref()
                .map(crate::parakeet_import::expected_gpu_for_candidate)
                .transpose()
            {
                Ok(expected) => expected,
                Err(error) if auto => {
                    use_gpu = false;
                    self.auto_fallback_reason = Some(format!(
                        "GPU identity preflight failed before native inference: {error}"
                    ));
                    self.replace_parakeet_pending_with_cpu_key(id)?;
                    None
                }
                Err(error) => return Err((id.0, id.1, error)),
            }
        } else {
            None
        };
        if !use_gpu && request.config.compute == whisper_core::ComputeChoice::Auto && !force_cpu {
            self.auto_fallback_reason = Some(if candidate.is_some() {
                "GPU candidate identity is stale before processing".into()
            } else {
                "GPU candidate proof unavailable before processing".into()
            });
            if candidate.is_some() {
                self.replace_parakeet_pending_with_cpu_key(id)?;
            }
        }
        let selected_paths = if use_gpu {
            self.gpu_parakeet_paths.as_ref()
        } else {
            self.parakeet_paths.as_ref()
        };
        let mut qualification = selected_paths
            .ok_or_else(|| {
                (
                    id.0,
                    id.1,
                    if use_gpu {
                        "BackendUnavailable: worker GPU Test Parakeet non configuré".into()
                    } else {
                        "ModelNotQualified: worker CPU Test Parakeet non configuré".into()
                    },
                )
            })
            .and_then(|paths| {
                if use_gpu {
                    paths.gpu_qualification_key(profile)
                } else {
                    paths.qualification_key(profile)
                }
                .map_err(|error| (id.0, id.1, error))
            });
        if use_gpu
            && qualification
                .as_ref()
                .is_ok_and(|key| !gpu_qualification_candidate_allowed(key))
        {
            qualification = Err((
                id.0,
                id.1,
                "ModelNotQualified: GPU Test key is not admitted".into(),
            ));
        }
        if use_gpu && qualification.is_err() && auto {
            use_gpu = false;
            expected_gpu = None;
            self.auto_fallback_reason =
                Some("GPU qualification unavailable before native inference".into());
            self.replace_parakeet_pending_with_cpu_key(id)?;
            qualification = self
                .parakeet_paths
                .as_ref()
                .ok_or((
                    id.0,
                    id.1,
                    "ModelNotQualified: worker CPU Test Parakeet non configuré".into(),
                ))
                .and_then(|paths| {
                    paths
                        .qualification_key(profile)
                        .map_err(|error| (id.0, id.1, error))
                });
        }
        let qualification = qualification?;
        let paths = if use_gpu {
            self.gpu_parakeet_paths.as_ref()
        } else {
            self.parakeet_paths.as_ref()
        }
        .ok_or((
            id.0,
            id.1,
            "ModelNotQualified: Parakeet worker is absent".into(),
        ))?;
        paths
            .verify_qualification(
                record.artifact.sha256,
                &qualification.runtime_build,
                &qualification.worker_build,
            )
            .map_err(|error| (id.0, id.1, error))?;
        crate::archive::assert_campaign_archive(Path::new(&request.destination))
            .map_err(|error| (id.0, id.1, error))?;
        let process_result = crate::parakeet_import::ParakeetProcess::spawn(paths);
        let process = match process_result {
            Ok(process) => process,
            Err(error) if auto && use_gpu => {
                return self
                    .fallback_parakeet_to_cpu(
                        request,
                        format!("GPU worker preparation failed: {error}"),
                    )
                    .map_err(|message| (id.0, id.1, message));
            }
            Err(error) => return Err((id.0, id.1, format!("WorkerExited: {error}"))),
        };
        let vad_run = if profile == whisper_core::parakeet_windows::PROFILE_IMPORT_VAD {
            let source_sha256 = request
                .source_sha256
                .map(|sha| format_sha256(&sha))
                .ok_or((id.0, id.1, "SourceMissing: source hash is absent".into()))?;
            let decoder_transport = ChildWorkerTransport::spawn(&self.workers.cpu)
                .map_err(|error| (id.0, id.1, format!("DecoderUnavailable: {error:?}")))?;
            Some(ParakeetVadRun {
                request: request.clone(),
                decoder: Some(crate::decoder::DecoderProxy::new(decoder_transport)),
                decode_offset: 0,
                total_samples: source_samples,
                source_sha256,
                pcm: Vec::new(),
                windows: Vec::new(),
                next_window: 0,
                next_segment_sequence: self.archive.next_segment_sequence(),
                request_count: 0,
                current_attempt: 0,
                current_path: None,
                current_source_sha256: None,
                pending_confirmation: None,
                worker_ready: false,
                ready_event_sent: false,
            })
        } else {
            None
        };
        let instance_id = NEXT_INSTANCE.fetch_add(1, Ordering::Relaxed);
        if instance_id == 0 {
            return Err((
                id.0,
                id.1,
                "WorkerExited: instance identity exhausted".into(),
            ));
        }
        self.start_diagnostics(Path::new(&request.destination));
        self.progress = Some(ProgressSupervisor::starting(Instant::now()));
        self.active = Some(ImportRequestState {
            job_id: id.0,
            generation: id.1,
            instance_id,
            backend: if use_gpu {
                BackendKind::Cuda
            } else {
                BackendKind::Cpu
            },
            attempt_number: 1,
            import_request: Some(request),
            recovery: LiveRecoveryState::None,
            startup_ready: false,
            gpu_candidate: if use_gpu { candidate } else { None },
            gpu_attested: false,
        });
        self.parakeet_expected_gpu = expected_gpu;
        self.parakeet_native_call_started = false;
        let _ = source_samples;
        self.parakeet = Some(process);
        self.parakeet_vad = vad_run;
        if self.parakeet_vad.is_some() {
            self.resume_prefix.clear();
        }
        Ok(())
    }

    pub(in crate::worker_ipc) fn replace_parakeet_pending_with_cpu_key(
        &mut self,
        id: (JobId, Generation),
    ) -> Result<(), (JobId, Generation, String)> {
        let key = self
            .parakeet_paths
            .as_ref()
            .ok_or((id.0, id.1, "CPU Parakeet worker is absent".into()))?
            .qualification_key(whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS)
            .map_err(|error| (id.0, id.1, error))?;
        self.archive
            .replace_pending_qualification(key)
            .map_err(|error| (id.0, id.1, error))
    }

    pub(in crate::worker_ipc) fn fallback_parakeet_to_cpu(
        &mut self,
        request: ImportRequest,
        reason: String,
    ) -> Result<(), String> {
        if self.parakeet_native_call_started {
            return Err(
                "BackendMismatch: CPU fallback is forbidden after NativeCallStarted".into(),
            );
        }
        let id = (request.job_id, request.generation);
        self.replace_parakeet_pending_with_cpu_key(id)
            .map_err(|(_, _, error)| error)?;
        if let Some(process) = self.parakeet.as_mut() {
            process.terminate().map_err(|error| {
                format!("WorkerExited: GPU preparation failure; termination failed: {error}")
            })?;
        }
        self.parakeet = None;
        self.active = None;
        self.parakeet_expected_gpu = None;
        self.parakeet_native_call_started = false;
        self.auto_fallback_reason = Some(reason);
        self.start_parakeet_worker(request, true, None)
            .map_err(|(_, _, error)| error)
    }

    pub(in crate::worker_ipc) fn handle_worker_failure(
        &mut self,
        job_id: JobId,
        generation: Generation,
        code: whisper_core::ipc::WorkerErrorCode,
        message: String,
    ) -> Result<ImportEvent, (JobId, Generation, String)> {
        if self.live.is_none()
            && let Some(pending) = self.pending_live.as_mut().filter(|pending| {
                (pending.request.job_id, pending.request.generation) == (job_id, generation)
            })
        {
            let backend = self.active.as_ref().map(|active| active.backend);
            let retryable = matches!(
                code,
                whisper_core::ipc::WorkerErrorCode::BackendUnavailable
                    | whisper_core::ipc::WorkerErrorCode::BackendMismatch
                    | whisper_core::ipc::WorkerErrorCode::WorkerExited
                    | whisper_core::ipc::WorkerErrorCode::InferenceFailed
            );
            let decision = if retryable && backend == Some(BackendKind::Cuda) {
                whisper_core::ComputePolicy::after_failure(
                    pending.request.config.compute,
                    whisper_core::BackendAttempt::Gpu,
                    false,
                )
            } else {
                whisper_core::BackendDecision::Fail
            };
            match decision {
                whisper_core::BackendDecision::RetryCpu => {
                    pending.recovery = LiveRecoveryState::AutoRetryPending
                }
                whisper_core::BackendDecision::AwaitUserChoice => {
                    pending.recovery = LiveRecoveryState::AwaitingChoicePending
                }
                _ => return Err((job_id, generation, message)),
            }
            if let Some(transport) = self.transport.as_mut() {
                let _ = transport.try_request(WorkerCommand::Stop { job_id, generation });
            }
            return Ok(match decision {
                whisper_core::BackendDecision::RetryCpu => ImportEvent::LiveRetryingOnCpu {
                    job_id,
                    generation,
                    spectrum_min_epoch: 0,
                    spectrum_not_before_monotonic_ms:
                        whisper_core::microphone_level::monotonic_millis(),
                },
                _ => ImportEvent::LiveFinalizing { job_id, generation },
            });
        }
        if self.live.is_none()
            && let Some(active) = self.active.as_mut().filter(|active| {
                active.job_id == job_id
                    && active.generation == generation
                    && active.import_request.is_some()
            })
        {
            let retryable = matches!(
                code,
                whisper_core::ipc::WorkerErrorCode::BackendUnavailable
                    | whisper_core::ipc::WorkerErrorCode::BackendMismatch
                    | whisper_core::ipc::WorkerErrorCode::WorkerExited
                    | whisper_core::ipc::WorkerErrorCode::InferenceFailed
            );
            let decision = if retryable && active.backend == BackendKind::Cuda {
                whisper_core::ComputePolicy::after_failure(
                    active
                        .import_request
                        .as_ref()
                        .expect("checked above")
                        .config
                        .compute,
                    whisper_core::BackendAttempt::Gpu,
                    active.startup_ready,
                )
            } else {
                whisper_core::BackendDecision::Fail
            };
            if decision == whisper_core::BackendDecision::RetryCpu {
                self.auto_fallback_reason
                    .get_or_insert_with(|| message.clone());
                active.recovery = LiveRecoveryState::AutoRetryPending;
                if let Some(transport) = self.transport.as_mut() {
                    let _ = transport.try_request(WorkerCommand::Stop { job_id, generation });
                }
                return Ok(ImportEvent::LiveAttemptStarting { job_id, generation });
            }
        }
        if let Some(live) = self.live.as_mut() {
            let backend = self.active.as_ref().map(|active| active.backend);
            let choice = live.request.config.compute;
            let retryable = matches!(
                code,
                whisper_core::ipc::WorkerErrorCode::BackendUnavailable
                    | whisper_core::ipc::WorkerErrorCode::BackendMismatch
                    | whisper_core::ipc::WorkerErrorCode::WorkerExited
                    | whisper_core::ipc::WorkerErrorCode::InferenceFailed
            );
            let decision = if retryable && backend == Some(BackendKind::Cuda) {
                whisper_core::ComputePolicy::after_failure(
                    choice,
                    whisper_core::BackendAttempt::Gpu,
                    self.active
                        .as_ref()
                        .is_some_and(|active| active.startup_ready),
                )
            } else {
                whisper_core::BackendDecision::Fail
            };
            match decision {
                whisper_core::BackendDecision::RetryCpu => {
                    live.recovery = LiveRecoveryState::AutoRetryPending;
                    live.inference_admission_closed = true;
                    live.pending_window = None;
                    live.barrier_m = Some(live.inference_sequence.saturating_sub(1));
                }
                whisper_core::BackendDecision::AwaitUserChoice => {
                    live.recovery = LiveRecoveryState::AwaitingChoicePending;
                    live.inference_admission_closed = true;
                    live.pending_window = None;
                    live.barrier_m = Some(live.inference_sequence.saturating_sub(1));
                    live.stop_capture();
                }
                whisper_core::BackendDecision::Fail => {
                    live.stop_capture();
                    let _ = live.staging.drain();
                    live.failure = Some(message.clone());
                }
                whisper_core::BackendDecision::Start(_) => {
                    live.stop_capture();
                    let _ = live.staging.drain();
                    live.failure = Some(message.clone());
                }
            }
            if decision == whisper_core::BackendDecision::AwaitUserChoice {
                let mut stop_events = VecDeque::new();
                self.stop(job_id, generation, &mut stop_events)?;
                self.pending_recovery_counters = recovery_stop_counters(stop_events);
            }
            if let Some(transport) = self.transport.as_mut() {
                let _ = transport.try_request(WorkerCommand::Stop { job_id, generation });
            }
            let retry_spectrum_barrier = self.live.as_ref().map(|live| live.probe.advance_epoch());
            return match decision {
                whisper_core::BackendDecision::RetryCpu => {
                    let (spectrum_min_epoch, spectrum_not_before_monotonic_ms) =
                        retry_spectrum_barrier.unwrap_or_else(|| {
                            (0, whisper_core::microphone_level::monotonic_millis())
                        });
                    Ok(ImportEvent::LiveRetryingOnCpu {
                        job_id,
                        generation,
                        spectrum_min_epoch,
                        spectrum_not_before_monotonic_ms,
                    })
                }
                whisper_core::BackendDecision::AwaitUserChoice => {
                    Ok(ImportEvent::LiveFinalizing { job_id, generation })
                }
                whisper_core::BackendDecision::Fail => Err((job_id, generation, message)),
                whisper_core::BackendDecision::Start(_) => Err((job_id, generation, message)),
            };
        }
        self.pending_live = None;
        self.live_archive = None;
        self.transport = None;
        self.active = None;
        Err((job_id, generation, message))
    }
}
