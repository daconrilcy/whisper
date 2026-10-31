use super::*;

impl DirectLive {
    pub(super) fn handle_worker(
        &mut self,
        event: WorkerEvent,
    ) -> Result<Option<ImportEvent>, String> {
        match event {
            WorkerEvent::GpuSessionAttested {
                job_id,
                generation,
                instance_id,
                candidate,
                dxgi_rank,
                luid,
                pci_id,
                cuda_ordinal,
                ggml_ordinal,
            } if self.matches(job_id, generation, instance_id) => {
                let verified = self.gpu_candidate.as_ref().is_some_and(|expected| {
                    expected.pnp_instance_id == candidate
                        && expected.dxgi_rank == dxgi_rank
                        && expected.luid == luid
                        && expected.pci_id.eq_ignore_ascii_case(&pci_id)
                        && expected.cuda_ordinal == cuda_ordinal
                        && expected.ggml_ordinal == ggml_ordinal
                });
                if !verified {
                    return Err("BackendMismatch: worker attested a different GPU candidate".into());
                }
                self.gpu_attested = true;
                Ok(None)
            }
            WorkerEvent::Failed { message, .. }
                if self.request.config.compute == ComputeChoice::Cpu
                    && !self.worker_ready
                    && !self.legacy_fallback
                    && message.contains("unknown variant `StartDirectLiveWorker`") =>
            {
                self.worker.as_mut().expect("worker owned").abort();
                self.worker = Some(
                    ChildWorkerTransport::spawn(&self.worker_path)
                        .map_err(|error| format!("Moteur CPU indisponible : {error:?}"))?,
                );
                self.legacy_fallback = true;
                self.worker
                    .as_mut()
                    .expect("worker owned")
                    .request(WorkerCommand::StartLiveWorker {
                        job_id: self.request.job_id,
                        generation: self.request.generation,
                        instance_id: self.instance,
                        model_path: self.request.model_path.clone(),
                        model_sha256: self.request.model_sha256,
                        config: self.request.config.clone(),
                    })
                    .map_err(|error| format!("Démarrage du moteur CPU impossible : {error:?}"))?;
                Ok(None)
            }
            WorkerEvent::Ready {
                job_id,
                generation,
                instance_id,
                backend,
            } if self.matches(job_id, generation, instance_id)
                && backend == self.backend_kind() =>
            {
                if gpu_ready_requires_attestation(self.gpu_candidate.is_some(), self.gpu_attested) {
                    return Err(
                        "BackendMismatch: GPU Ready arrived before candidate attestation".into(),
                    );
                }
                self.worker_ready = true;
                Ok(Some(ImportEvent::Ready {
                    job_id,
                    generation,
                    backend: format!(
                        "{} (micro){}",
                        self.engine_name(),
                        self.fallback_reason
                            .as_ref()
                            .map(|reason| format!(
                                " — GPU non utilisable ({reason}) ; CPU prend le relais"
                            ))
                            .unwrap_or_default()
                    ),
                }))
            }
            WorkerEvent::Ready {
                job_id,
                generation,
                instance_id,
                backend,
            } if self.matches(job_id, generation, instance_id)
                && self.auto_preference
                && !self.worker_ready
                && self.request.config.compute == ComputeChoice::Gpu =>
            {
                self.retry_cpu(format!("attestation GPU incompatible : {backend:?}"))?;
                Ok(None)
            }
            WorkerEvent::LiveMp3Packet {
                job_id,
                generation,
                instance_id,
                ..
            } if self.legacy_fallback && self.matches(job_id, generation, instance_id) => Ok(None),
            WorkerEvent::Segment(dto)
                if self.matches(dto.job_id, dto.generation, dto.instance_id) =>
            {
                let within_window = self.inflight.is_some_and(|(_, range)| {
                    dto.range.start_sample >= range.start_sample
                        && dto.range.end_sample <= range.end_sample
                });
                if dto.segment_id != SegmentId(self.next_segment)
                    || !within_window
                    || dto.range.sample_rate_hz != 16_000
                    || dto.range.start_sample >= dto.range.end_sample
                    || dto.range.end_sample > self.next_offset
                    || self
                        .window_segments
                        .last()
                        .or_else(|| self.segments.last())
                        .is_some_and(|last| {
                            dto.range.start_sample < last.range.start_sample
                                || dto.range.end_sample < last.range.end_sample
                        })
                {
                    return Err(format!(
                        "Moteur {} : segment hors ordre",
                        self.engine_name()
                    ));
                }
                self.next_segment += 1;
                let segment = WorkerSegment {
                    job_id: dto.job_id,
                    generation: dto.generation,
                    instance_id: dto.instance_id,
                    segment_id: dto.segment_id,
                    range: dto.range,
                    text: dto.text,
                };
                self.window_segments.push(segment);
                Ok(None)
            }
            WorkerEvent::WindowFinished {
                job_id,
                generation,
                instance_id,
                sequence,
                range,
                last_segment_sequence,
            } if self.matches(job_id, generation, instance_id) => {
                if self.inflight != Some((sequence, range))
                    || last_segment_sequence != self.next_segment - 1
                {
                    return Err(format!(
                        "Moteur {} : fin de fenêtre incohérente",
                        self.engine_name()
                    ));
                }
                self.inflight = None;
                for segment in self.window_segments.drain(..) {
                    self.ready_events
                        .push_back(ImportEvent::DirectConfirmed(segment.clone()));
                    self.segments.push(segment);
                }
                Ok(self.ready_events.pop_front())
            }
            WorkerEvent::Stopped {
                job_id,
                generation,
                instance_id,
            } if self.matches(job_id, generation, instance_id) && self.stop_sent => {
                let wav_path = self.wav.take().expect("wav ready").finish()?;
                let (txt_path, srt_path) = self.export(false)?;
                self.done = true;
                Ok(Some(ImportEvent::DirectPublished {
                    job_id,
                    generation,
                    txt_path,
                    srt_path,
                    wav_path,
                }))
            }
            WorkerEvent::Failed {
                code: WorkerErrorCode::ModelUnavailable | WorkerErrorCode::ModelHashMismatch,
                message,
                ..
            } => Err(format!(
                "Modèle local indisponible. Vérifiez le chemin du modèle du MVP : {message}"
            )),
            WorkerEvent::Failed { code, message, .. }
                if should_retry_auto_gpu_before_ready(
                    self.auto_preference,
                    self.worker_ready,
                    self.request.config.compute,
                    code,
                ) =>
            {
                self.retry_cpu(message)?;
                Ok(None)
            }
            WorkerEvent::Failed { message, .. } => Err(format!(
                "Moteur {} interrompu. Réessayez après vérification : {message}",
                self.engine_name()
            )),
            _ => Err(format!(
                "Moteur {} : réponse inattendue ou périmée",
                self.engine_name()
            )),
        }
    }

    pub(super) fn engine_name(&self) -> &'static str {
        compute_name(self.request.config.compute)
    }

    pub(super) fn retry_cpu(&mut self, reason: String) -> Result<(), String> {
        self.worker.as_mut().expect("worker owned").abort();
        if let Some(probe) = &self.probe {
            probe.invalidate();
        }
        if !self.cpu_worker_path.is_file() {
            return Err(format!("GPU non utilisable ({reason}) ; CPU indisponible"));
        }
        self.instance = self
            .instance
            .checked_add(1)
            .ok_or("Identité worker épuisée")?;
        self.request.config.compute = ComputeChoice::Cpu;
        let worker = spawn_direct_worker(
            &self.request,
            &self.cpu_worker_path,
            self.instance,
            None,
            None,
        )
        .map_err(|error| format!("GPU non utilisable ({reason}) ; CPU indisponible : {error}"))?;
        self.worker = Some(worker);
        self.worker_path = self.cpu_worker_path.clone();
        clear_gpu_proof_for_cpu(&mut self.gpu_candidate, &mut self.gpu_attested);
        self.fallback_reason = Some(reason);
        Ok(())
    }

    fn backend_kind(&self) -> BackendKind {
        match self.request.config.compute {
            ComputeChoice::Cpu => BackendKind::Cpu,
            ComputeChoice::Gpu => BackendKind::Cuda,
            ComputeChoice::Auto => unreachable!(),
        }
    }

    fn matches(&self, job_id: JobId, generation: Generation, instance: u64) -> bool {
        (job_id, generation) == self.identity() && instance == self.instance
    }
}

pub(super) fn gpu_ready_requires_attestation(has_candidate: bool, attested: bool) -> bool {
    has_candidate && !attested
}

pub(super) fn should_retry_auto_gpu_before_ready(
    auto_preference: bool,
    worker_ready: bool,
    compute: ComputeChoice,
    code: WorkerErrorCode,
) -> bool {
    auto_preference
        && !worker_ready
        && compute == ComputeChoice::Gpu
        && matches!(
            code,
            WorkerErrorCode::BackendUnavailable
                | WorkerErrorCode::BackendMismatch
                | WorkerErrorCode::WorkerExited
                | WorkerErrorCode::InferenceFailed
        )
}

pub(super) fn clear_gpu_proof_for_cpu(
    candidate: &mut Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
    attested: &mut bool,
) {
    *candidate = None;
    *attested = false;
}

pub(super) fn spawn_direct_worker(
    request: &LiveRequest,
    worker_path: &std::path::Path,
    instance: u64,
    reusable_worker: Option<ChildWorkerTransport>,
    gpu_candidate: Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
) -> Result<ChildWorkerTransport, String> {
    let mut worker = match reusable_worker {
        Some(worker) => worker,
        None => ChildWorkerTransport::spawn(worker_path)
            .map_err(|error| format!("lancement impossible : {error:?}"))?,
    };
    let command = match gpu_candidate {
        Some(candidate) if request.config.compute == ComputeChoice::Gpu => {
            WorkerCommand::StartGpuDirectLiveWorker {
                job_id: request.job_id,
                generation: request.generation,
                instance_id: instance,
                model_path: request.model_path.clone(),
                model_sha256: request.model_sha256,
                config: request.config.clone(),
                candidate,
            }
        }
        _ => WorkerCommand::StartDirectLiveWorker {
            job_id: request.job_id,
            generation: request.generation,
            instance_id: instance,
            model_path: request.model_path.clone(),
            model_sha256: request.model_sha256,
            config: request.config.clone(),
        },
    };
    if let Err(error) = worker.request(command) {
        worker.abort();
        return Err(format!("commande de démarrage refusée : {error:?}"));
    }
    Ok(worker)
}

pub(super) fn compute_name(choice: ComputeChoice) -> &'static str {
    match choice {
        ComputeChoice::Cpu => "CPU",
        ComputeChoice::Gpu => "GPU",
        ComputeChoice::Auto => "Auto",
    }
}
