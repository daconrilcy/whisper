use std::{
    collections::VecDeque,
    fs,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use whisper_core::{
    ComputeChoice, Generation, ImportEvent, JobId, LiveRequest, SegmentId, SourceRange,
    ipc::{BackendKind, WorkerCommand, WorkerErrorCode, WorkerEvent},
    ports::WorkerSegment,
};

use crate::{
    WorkerTransport,
    capture::{CaptureStream, Pcm16Converter},
    supervisor::{DiagnosticCode, DiagnosticPhase, DiagnosticRecord, DiagnosticSink, WorkerPaths},
    worker_ipc::ChildWorkerTransport,
};

const MIN_WINDOW_SAMPLES: usize = 80_000;
const MAX_WINDOW_SAMPLES: usize = 128_000;
const PAUSE_SAMPLES: usize = 3_200;
const PAUSE_STEP_SAMPLES: usize = 1_600;
const PAUSE_RMS_LIMIT: u64 = 400;
// Only the next inference window is cached in RAM. The WAV is the backlog.
const MAX_BUFFERED_SAMPLES: usize = MAX_WINDOW_SAMPLES;
const MICROPHONE_ERROR: &str = "Microphone déconnecté, indisponible ou incompatible. Vérifiez sa connexion, son mode micro et l'autorisation Windows ; transcription incomplète";

pub(crate) struct DirectLive {
    request: LiveRequest,
    auto_preference: bool,
    fallback_reason: Option<String>,
    cpu_worker_path: PathBuf,
    worker_path: PathBuf,
    gpu_candidate: Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
    worker: Option<ChildWorkerTransport>,
    instance: u64,
    worker_ready: bool,
    gpu_attested: bool,
    legacy_fallback: bool,
    capture: Option<CaptureStream>,
    converter: Option<Pcm16Converter>,
    wav: Option<WavOutput>,
    diagnostics: Option<DiagnosticSink>,
    pending: VecDeque<i16>,
    next_offset: u64,
    next_window: u64,
    inflight: Option<(u64, SourceRange)>,
    next_segment: u64,
    segments: Vec<WorkerSegment>,
    window_segments: Vec<WorkerSegment>,
    ready_events: VecDeque<ImportEvent>,
    capture_announced: bool,
    first_admitted_at_unix_ms: Option<u64>,
    stop_requested: bool,
    stop_sent: bool,
    done: bool,
}

impl DirectLive {
    pub(crate) fn start(
        mut request: LiveRequest,
        workers: &WorkerPaths,
        instance: u64,
        reusable_worker: Option<ChildWorkerTransport>,
        mut gpu_candidate: Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
    ) -> Result<Self, String> {
        if request.destination.trim().is_empty() || request.model_path.trim().is_empty() {
            return Err("Micro : modèle ou dossier de sortie invalide".into());
        }
        let auto_preference = request.config.compute == ComputeChoice::Auto;
        let selected = if auto_preference {
            Some(
                workers
                    .select(ComputeChoice::Auto)
                    .map_err(|error| format!("Moteur Auto indisponible : {error:?}"))?,
            )
        } else {
            None
        };
        let mut fallback_reason = selected
            .as_ref()
            .filter(|selection| selection.backend == whisper_core::BackendAttempt::Cpu)
            .map(|_| "worker CUDA absent".to_owned());
        if auto_preference {
            request.config.compute = match selected.as_ref().expect("auto selection").backend {
                whisper_core::BackendAttempt::Cpu => ComputeChoice::Cpu,
                whisper_core::BackendAttempt::Gpu => ComputeChoice::Gpu,
            };
            if request.config.compute == ComputeChoice::Cpu {
                gpu_candidate = None;
            }
        }
        if request.config.compute == ComputeChoice::Gpu && gpu_candidate.is_none() {
            if auto_preference {
                request.config.compute = ComputeChoice::Cpu;
                gpu_candidate = None;
                fallback_reason = Some("GPU identity proof unavailable before processing".into());
            } else {
                return Err("BackendMismatch: selected GPU has no current identity proof".into());
            }
        }
        let mut worker_path = match request.config.compute {
            ComputeChoice::Cpu => workers.cpu.clone(),
            ComputeChoice::Gpu => workers.gpu.clone(),
            ComputeChoice::Auto => unreachable!(),
        };
        let mut worker = match spawn_direct_worker(
            &request,
            &worker_path,
            instance,
            reusable_worker,
            gpu_candidate.clone(),
        ) {
            Ok(worker) => worker,
            Err(reason) if auto_preference && request.config.compute == ComputeChoice::Gpu => {
                fallback_reason = Some(reason);
                request.config.compute = ComputeChoice::Cpu;
                gpu_candidate = None;
                worker_path = workers.cpu.clone();
                spawn_direct_worker(&request, &worker_path, instance, None, None).map_err(
                    |error| {
                        format!(
                            "GPU non utilisable ({}) ; CPU indisponible : {error}",
                            fallback_reason.as_deref().unwrap_or("raison inconnue")
                        )
                    },
                )?
            }
            Err(error) => {
                return Err(format!(
                    "Moteur {} indisponible : {error}",
                    compute_name(request.config.compute)
                ));
            }
        };
        let mut capture = match crate::capture::start_direct(&request.microphone) {
            Ok(capture) => capture,
            Err(error) => {
                worker.abort();
                return Err(format!(
                    "{MICROPHONE_ERROR} : ouverture impossible : {error}"
                ));
            }
        };
        let converter = match Pcm16Converter::new(capture.sample_rate_hz(), capture.channels()) {
            Ok(converter) => converter,
            Err(error) => {
                capture.stop();
                worker.abort();
                return Err(error);
            }
        };
        let wav = match WavOutput::create(&request) {
            Ok(wav) => wav,
            Err(error) => {
                capture.stop();
                worker.abort();
                return Err(error);
            }
        };
        Ok(Self {
            diagnostics: DiagnosticSink::start(Path::new(&request.destination)).ok(),
            request,
            auto_preference,
            fallback_reason,
            cpu_worker_path: workers.cpu.clone(),
            worker_path,
            gpu_candidate,
            worker: Some(worker),
            instance,
            worker_ready: false,
            gpu_attested: false,
            legacy_fallback: false,
            capture: Some(capture),
            converter: Some(converter),
            wav: Some(wav),
            pending: VecDeque::new(),
            next_offset: 0,
            next_window: 1,
            inflight: None,
            next_segment: 1,
            segments: Vec::new(),
            window_segments: Vec::new(),
            ready_events: VecDeque::new(),
            capture_announced: false,
            first_admitted_at_unix_ms: None,
            stop_requested: false,
            stop_sent: false,
            done: false,
        })
    }

    pub(crate) fn identity(&self) -> (JobId, Generation) {
        (self.request.job_id, self.request.generation)
    }

    pub(crate) fn done(&self) -> bool {
        self.done
    }

    pub(crate) fn take_idle_worker(&mut self) -> Option<ChildWorkerTransport> {
        self.done.then(|| self.worker.take()).flatten()
    }

    pub(crate) fn cache_identity(&self) -> (ComputeChoice, &str, [u8; 32]) {
        (
            self.request.config.compute,
            &self.request.model_path,
            self.request.model_sha256,
        )
    }

    pub(crate) fn stop(&mut self) -> Result<(), String> {
        if self.stop_requested {
            return Ok(());
        }
        self.stop_requested = true;
        if let Some(capture) = self.capture.as_mut() {
            capture.stop();
            self.drain_capture()?;
            if let Some(converter) = self.converter.as_mut() {
                let tail = converter.finish()?;
                self.wav.as_mut().expect("wav ready").append(&tail)?;
            }
        }
        Ok(())
    }

    pub(crate) fn poll(&mut self) -> Result<Option<ImportEvent>, String> {
        if self.done {
            return Ok(None);
        }
        self.pump_capture()?;
        if !self.capture_announced && self.wav.as_ref().is_some_and(|wav| wav.samples > 0) {
            self.capture_announced = true;
            self.ready_events
                .push_back(ImportEvent::DirectCaptureActive {
                    job_id: self.request.job_id,
                    generation: self.request.generation,
                    first_sample_at_unix_ms: self
                        .capture
                        .as_ref()
                        .and_then(CaptureStream::first_sample_at_unix_ms)
                        .unwrap_or(0),
                    first_admitted_at_unix_ms: self.first_admitted_at_unix_ms.unwrap_or(0),
                });
        }
        if let Some(event) = self.ready_events.pop_front() {
            return Ok(Some(event));
        }
        let received = match self.worker.as_mut().expect("worker owned").receive() {
            Ok(event) => event,
            Err(error)
                if self.auto_preference
                    && !self.worker_ready
                    && self.request.config.compute == ComputeChoice::Gpu =>
            {
                self.retry_cpu(format!("worker CUDA interrompu : {error:?}"))?;
                return Ok(None);
            }
            Err(error) => {
                return Err(format!(
                    "Moteur {} interrompu : {error:?}",
                    self.engine_name()
                ));
            }
        };
        if let Some(event) = received
            && let Some(result) = self.handle_worker(event)?
        {
            return Ok(Some(result));
        }
        self.send_next()?;
        Ok(None)
    }

    pub(crate) fn pump_capture(&mut self) -> Result<(), String> {
        if !self.stop_requested && self.capture.is_some() {
            self.drain_capture()?;
        }
        Ok(())
    }

    fn drain_capture(&mut self) -> Result<(), String> {
        let xruns = self.capture.as_ref().expect("capture ready").take_xruns();
        if xruns > 0 {
            self.record_capture_issue(
                DiagnosticCode::CaptureWarning,
                format!("WASAPI Xrun: {xruns} discontinuity event(s); stream still active"),
            );
        }
        let capture = self.capture.as_mut().expect("capture ready");
        if capture.is_saturated() {
            capture.stop();
            return Err("Tampon du micro saturé ; transcription incomplète".into());
        }
        if capture.failure() {
            let detail = capture
                .failure_detail()
                .unwrap_or_else(|| "flux audio interrompu ou trame invalide".into());
            capture.stop();
            self.record_capture_issue(DiagnosticCode::CaptureFailed, detail);
            return Err(self.microphone_error());
        }
        for _ in 0..if self.stop_requested { 100 } else { 32 } {
            let Some(block) = capture.try_next_block() else {
                break;
            };
            let samples = self
                .converter
                .as_mut()
                .expect("converter ready")
                .push(&block)?;
            capture.recycle_block(block)?;
            self.wav.as_mut().expect("wav ready").append(&samples)?;
            if !samples.is_empty() && self.first_admitted_at_unix_ms.is_none() {
                self.first_admitted_at_unix_ms = Some(
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_or(0, |elapsed| elapsed.as_millis() as u64),
                );
            }
        }
        Ok(())
    }

    fn record_capture_issue(&self, code: DiagnosticCode, detail: String) {
        let context = format!(
            "microphone={:?}; compute={:?}; input_rate={:?}; input_channels={:?}; {detail}",
            self.request.microphone,
            self.request.config.compute,
            self.capture.as_ref().map(CaptureStream::sample_rate_hz),
            self.capture.as_ref().map(CaptureStream::channels),
        );
        if let Some(diagnostics) = &self.diagnostics {
            diagnostics.record(
                DiagnosticRecord::new(
                    self.request.job_id.0,
                    self.request.generation.get(),
                    DiagnosticPhase::Capturing,
                    code,
                    Some(if self.request.config.compute == ComputeChoice::Gpu {
                        "cuda"
                    } else {
                        "cpu"
                    }),
                    None,
                )
                .with_detail(context.clone()),
            );
        }
        eprintln!("Capture microphone : {context}");
    }

    fn microphone_error(&self) -> String {
        if self.diagnostics.is_some() {
            format!(
                "{MICROPHONE_ERROR}. Détails : {}",
                Path::new(&self.request.destination)
                    .join(".whisper-diagnostics")
                    .join("worker-0.jsonl")
                    .display()
            )
        } else {
            MICROPHONE_ERROR.into()
        }
    }

    fn send_next(&mut self) -> Result<(), String> {
        if !self.worker_ready || self.inflight.is_some() || self.stop_sent {
            return Ok(());
        }
        self.fill_pending_from_wav()?;
        let Some(count) = window_size(&self.pending, self.stop_requested) else {
            return Ok(());
        };
        if count > 0 {
            let samples: Vec<i16> = self.pending.drain(..count).collect();
            let end = self
                .next_offset
                .checked_add(count as u64)
                .ok_or_else(|| "Horloge micro épuisée".to_owned())?;
            let range = SourceRange::new(self.next_offset, end, 16_000)
                .ok_or_else(|| "Intervalle micro invalide".to_owned())?;
            self.worker
                .as_mut()
                .expect("worker owned")
                .request(WorkerCommand::LiveWindow {
                    job_id: self.request.job_id,
                    generation: self.request.generation,
                    instance_id: self.instance,
                    sequence: self.next_window,
                    range,
                    samples,
                })
                .map_err(|error| format!("Fenêtre {} refusée : {error:?}", self.engine_name()))?;
            self.inflight = Some((self.next_window, range));
            self.next_window += 1;
            self.next_offset = end;
        } else if self.stop_requested {
            self.worker
                .as_mut()
                .expect("worker owned")
                .request(WorkerCommand::Stop {
                    job_id: self.request.job_id,
                    generation: self.request.generation,
                })
                .map_err(|error| format!("Arrêt {} refusé : {error:?}", self.engine_name()))?;
            self.stop_sent = true;
        }
        Ok(())
    }

    fn fill_pending_from_wav(&mut self) -> Result<(), String> {
        match self.wav.as_mut() {
            Some(wav) => fill_pending_from_wav(wav, self.next_offset, &mut self.pending),
            None => Ok(()),
        }
    }

    fn handle_worker(&mut self, event: WorkerEvent) -> Result<Option<ImportEvent>, String> {
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

    fn engine_name(&self) -> &'static str {
        compute_name(self.request.config.compute)
    }

    fn retry_cpu(&mut self, reason: String) -> Result<(), String> {
        self.worker.as_mut().expect("worker owned").abort();
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

    pub(crate) fn fail(&mut self, error: String) -> String {
        if self.wav.is_none() {
            return error;
        }
        self.worker.as_mut().expect("worker owned").abort();
        if let Some(capture) = self.capture.as_mut() {
            capture.stop();
            while let Some(block) = capture.try_next_block() {
                if let Some(converter) = self.converter.as_mut()
                    && let Ok(samples) = converter.push(&block)
                    && let Some(wav) = self.wav.as_mut()
                {
                    let _ = wav.append(&samples);
                }
                let _ = capture.recycle_block(block);
            }
        }
        if let Some(converter) = self.converter.as_mut()
            && let Ok(tail) = converter.finish()
            && let Some(wav) = self.wav.as_mut()
        {
            let _ = wav.append(&tail);
        }
        let Some(wav) = self.wav.take() else {
            return error;
        };
        finalize_incomplete_outputs(&self.request, wav, &self.segments, error)
    }

    fn export(&self, incomplete: bool) -> Result<(String, String), String> {
        export_segments(&self.request, &self.segments, incomplete)
    }
}

fn gpu_ready_requires_attestation(has_candidate: bool, attested: bool) -> bool {
    has_candidate && !attested
}

fn should_retry_auto_gpu_before_ready(
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

fn clear_gpu_proof_for_cpu(
    candidate: &mut Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
    attested: &mut bool,
) {
    *candidate = None;
    *attested = false;
}

fn finalize_incomplete_outputs(
    request: &LiveRequest,
    wav: WavOutput,
    segments: &[WorkerSegment],
    error: String,
) -> String {
    let wav_path = match wav.finish() {
        Ok(path) => path,
        Err(save_error) => {
            return format!("{error}. WAV incomplet non finalisé : {save_error}");
        }
    };
    let gpu = request.config.compute == ComputeChoice::Gpu;
    match export_segments(request, segments, true) {
        Ok((txt, srt)) if gpu => format!(
            "{error}. Session GPU arrêtée ; résultats incomplets : {wav_path} | {txt} | {srt}. Réimportez le WAV en CPU pour obtenir le texte complet."
        ),
        Ok((txt, srt)) => format!(
            "{error}. Session micro arrêtée ; résultats incomplets : {wav_path} | {txt} | {srt}."
        ),
        Err(save_error) if gpu => format!(
            "{error}. Session GPU arrêtée ; WAV conservé : {wav_path}. TXT/SRT incomplets non enregistrés : {save_error}. Réimportez le WAV en CPU."
        ),
        Err(save_error) => format!(
            "{error}. Session micro arrêtée ; WAV conservé : {wav_path}. TXT/SRT incomplets non enregistrés : {save_error}."
        ),
    }
}

fn export_segments(
    request: &LiveRequest,
    segments: &[WorkerSegment],
    incomplete: bool,
) -> Result<(String, String), String> {
    let directory = PathBuf::from(&request.destination);
    fs::create_dir_all(&directory)
        .map_err(|error| format!("Dossier de sortie inaccessible : {error}"))?;
    let stem = format!("micro-{:032x}", request.job_id.0);
    let suffix = if incomplete { "-incomplet" } else { "" };
    let txt = directory.join(format!("{stem}{suffix}.txt"));
    let srt = directory.join(format!("{stem}{suffix}.srt"));
    let text = segments
        .iter()
        .map(|segment| segment.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let subtitles = segments
        .iter()
        .enumerate()
        .map(|(index, segment)| {
            format!(
                "{}\n{} --> {}\n{}\n\n",
                index + 1,
                timestamp(segment.range.start_sample),
                timestamp(segment.range.end_sample),
                segment.text
            )
        })
        .collect::<String>();
    fs::write(&txt, text)
        .map_err(|error| format!("TXT non enregistré. Vérifiez le dossier de sortie : {error}"))?;
    fs::write(&srt, subtitles)
        .map_err(|error| format!("SRT non enregistré. Vérifiez le dossier de sortie : {error}"))?;
    Ok((
        txt.to_string_lossy().into_owned(),
        srt.to_string_lossy().into_owned(),
    ))
}

fn spawn_direct_worker(
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

fn compute_name(choice: ComputeChoice) -> &'static str {
    match choice {
        ComputeChoice::Cpu => "CPU",
        ComputeChoice::Gpu => "GPU",
        ComputeChoice::Auto => "Auto",
    }
}

fn window_size(pending: &VecDeque<i16>, stopping: bool) -> Option<usize> {
    let available = pending.len().min(MAX_WINDOW_SAMPLES);
    if available >= MIN_WINDOW_SAMPLES {
        for end in (MIN_WINDOW_SAMPLES..=available).step_by(PAUSE_STEP_SAMPLES) {
            let energy: u64 = pending
                .range(end - PAUSE_SAMPLES..end)
                .map(|sample| i64::from(*sample).unsigned_abs().pow(2))
                .sum();
            if energy <= PAUSE_RMS_LIMIT.pow(2) * PAUSE_SAMPLES as u64 {
                return Some(end);
            }
        }
        if available == MAX_WINDOW_SAMPLES {
            return Some(available);
        }
    }
    stopping.then_some(available)
}

fn fill_pending_from_wav(
    wav: &mut WavOutput,
    next_offset: u64,
    pending: &mut VecDeque<i16>,
) -> Result<(), String> {
    let loaded_until = next_offset
        .checked_add(pending.len() as u64)
        .ok_or_else(|| "Backlog WAV hors limites".to_owned())?;
    let available = wav.samples.saturating_sub(loaded_until);
    let free = MAX_BUFFERED_SAMPLES.saturating_sub(pending.len());
    let count = available.min(free as u64) as usize;
    if count > 0 {
        pending.extend(wav.read_samples(loaded_until, count)?);
    }
    Ok(())
}

struct WavOutput {
    file: fs::File,
    path: PathBuf,
    samples: u64,
}

impl WavOutput {
    fn create(request: &LiveRequest) -> Result<Self, String> {
        let directory = PathBuf::from(&request.destination);
        fs::create_dir_all(&directory)
            .map_err(|error| format!("Dossier de sortie inaccessible : {error}"))?;
        let path = directory.join(format!("micro-{:032x}.wav", request.job_id.0));
        let mut file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| format!("WAV non créé : {error}"))?;
        write_wav_header(&mut file, 0)?;
        Ok(Self {
            file,
            path,
            samples: 0,
        })
    }

    fn append(&mut self, samples: &[i16]) -> Result<(), String> {
        let total = self
            .samples
            .checked_add(samples.len() as u64)
            .ok_or_else(|| "WAV trop long".to_owned())?;
        if total > (u32::MAX as u64 - 36) / 2 {
            return Err("WAV trop long pour le format PCM".into());
        }
        let mut bytes = Vec::with_capacity(samples.len() * 2);
        for sample in samples {
            bytes.extend_from_slice(&sample.to_le_bytes());
        }
        self.file
            .write_all(&bytes)
            .map_err(|error| format!("WAV non écrit : {error}"))?;
        self.samples = total;
        Ok(())
    }

    fn read_samples(&mut self, offset: u64, count: usize) -> Result<Vec<i16>, String> {
        if offset
            .checked_add(count as u64)
            .is_none_or(|end| end > self.samples)
        {
            return Err("Backlog WAV hors limites".into());
        }
        self.file
            .flush()
            .map_err(|error| format!("WAV non synchronisé : {error}"))?;
        self.file
            .seek(SeekFrom::Start(44 + offset * 2))
            .map_err(|error| format!("Backlog WAV inaccessible : {error}"))?;
        let mut bytes = vec![0; count * 2];
        self.file
            .read_exact(&mut bytes)
            .map_err(|error| format!("Backlog WAV tronqué : {error}"))?;
        self.file
            .seek(SeekFrom::End(0))
            .map_err(|error| format!("WAV non repositionné : {error}"))?;
        Ok(bytes
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| i16::from_le_bytes(*pair))
            .collect())
    }

    fn finish(mut self) -> Result<String, String> {
        self.file
            .seek(SeekFrom::Start(0))
            .map_err(|error| format!("WAV non finalisé : {error}"))?;
        write_wav_header(&mut self.file, self.samples as u32)?;
        self.file
            .flush()
            .map_err(|error| format!("WAV non finalisé : {error}"))?;
        Ok(self.path.to_string_lossy().into_owned())
    }
}

impl Drop for WavOutput {
    fn drop(&mut self) {
        if self.file.seek(SeekFrom::Start(0)).is_ok() {
            let _ = write_wav_header(&mut self.file, self.samples as u32);
            let _ = self.file.flush();
        }
    }
}

fn write_wav_header(writer: &mut impl Write, samples: u32) -> Result<(), String> {
    let data_bytes = samples * 2;
    writer
        .write_all(b"RIFF")
        .and_then(|_| writer.write_all(&(36 + data_bytes).to_le_bytes()))
        .and_then(|_| writer.write_all(b"WAVEfmt "))
        .and_then(|_| writer.write_all(&16u32.to_le_bytes()))
        .and_then(|_| writer.write_all(&1u16.to_le_bytes()))
        .and_then(|_| writer.write_all(&1u16.to_le_bytes()))
        .and_then(|_| writer.write_all(&16_000u32.to_le_bytes()))
        .and_then(|_| writer.write_all(&32_000u32.to_le_bytes()))
        .and_then(|_| writer.write_all(&2u16.to_le_bytes()))
        .and_then(|_| writer.write_all(&16u16.to_le_bytes()))
        .and_then(|_| writer.write_all(b"data"))
        .and_then(|_| writer.write_all(&data_bytes.to_le_bytes()))
        .map_err(|error| format!("En-tête WAV non écrit : {error}"))
}

impl Drop for DirectLive {
    fn drop(&mut self) {
        if let Some(worker) = self.worker.as_mut() {
            worker.abort();
        }
    }
}

fn timestamp(samples: u64) -> String {
    let millis = samples.saturating_mul(1_000) / 16_000;
    format!(
        "{:02}:{:02}:{:02},{:03}",
        millis / 3_600_000,
        millis / 60_000 % 60,
        millis / 1_000 % 60,
        millis % 1_000
    )
}

#[cfg(test)]
mod tests {
    use super::{
        DirectLive, MAX_BUFFERED_SAMPLES, MAX_WINDOW_SAMPLES, MIN_WINDOW_SAMPLES, PathBuf,
        Pcm16Converter, WavOutput, clear_gpu_proof_for_cpu, fill_pending_from_wav,
        finalize_incomplete_outputs, gpu_ready_requires_attestation,
        should_retry_auto_gpu_before_ready, window_size, write_wav_header,
    };
    use std::collections::VecDeque;
    use whisper_core::ports::WorkerSegment;
    use whisper_core::{ComputeChoice, JobConfig, LanguageChoice, SegmentId, SourceRange};

    fn test_directory(label: &str) -> PathBuf {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "direct-live-{label}-{}-{nonce}",
            std::process::id()
        ))
    }

    #[test]
    fn cpu_ready_after_auto_fallback_needs_no_gpu_candidate_attestation() {
        let mut candidate = Some(whisper_core::gpu_inventory::GpuRuntimeIdentity {
            generation: 1,
            pnp_instance_id: "PCI\\VEN_10DE&DEV_A".into(),
            dxgi_rank: 0,
            luid: 1,
            pci_id: "0000:01:00.0".into(),
            cuda_ordinal: 0,
            ggml_ordinal: 1,
        });
        let mut attested = false;
        assert!(gpu_ready_requires_attestation(
            candidate.is_some(),
            attested
        ));
        clear_gpu_proof_for_cpu(&mut candidate, &mut attested);
        assert!(candidate.is_none());
        assert!(!attested);
        assert!(!gpu_ready_requires_attestation(
            candidate.is_some(),
            attested
        ));
        assert!(should_retry_auto_gpu_before_ready(
            true,
            false,
            ComputeChoice::Gpu,
            super::WorkerErrorCode::BackendMismatch,
        ));
        assert!(!should_retry_auto_gpu_before_ready(
            false,
            false,
            ComputeChoice::Gpu,
            super::WorkerErrorCode::BackendMismatch,
        ));
        assert!(!should_retry_auto_gpu_before_ready(
            true,
            true,
            ComputeChoice::Gpu,
            super::WorkerErrorCode::BackendMismatch,
        ));
    }

    fn startup_request(
        destination: &std::path::Path,
        compute: ComputeChoice,
    ) -> whisper_core::LiveRequest {
        whisper_core::LiveRequest {
            job_id: whisper_core::JobId(800),
            generation: whisper_core::Generation::first(),
            model_path: "unused-model".into(),
            model_sha256: [0; 32],
            destination: destination.to_string_lossy().into_owned(),
            group_offset_samples: 0,
            auto_stop_after_speech_samples: None,
            microphone: Default::default(),
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute,
            },
        }
    }

    fn startup_gpu_candidate() -> whisper_core::gpu_inventory::GpuRuntimeIdentity {
        whisper_core::gpu_inventory::GpuRuntimeIdentity {
            generation: 1,
            pnp_instance_id: "PCI\\VEN_10DE&DEV_TEST".into(),
            dxgi_rank: 0,
            luid: 1,
            pci_id: "0000:01:00.0".into(),
            cuda_ordinal: 0,
            ggml_ordinal: 0,
        }
    }

    fn startup_worker(directory: &std::path::Path, name: &str) -> std::path::PathBuf {
        let source = directory.join(format!("auto-{name}.rs"));
        let executable = directory.join(format!("auto-{name}.exe"));
        let marker = directory
            .join(format!("auto-{name}.marker"))
            .to_string_lossy()
            .replace('\\', "\\\\")
            .replace('"', "\\\"");
        let program = r#"
use std::io::{self, BufRead, Write};
const MARKER: &str = "__MARKER__";
fn field(line: &str, name: &str) -> String {
    let tail = line.split_once(&format!("\"{name}\":" )).unwrap().1.trim_start();
    tail.chars().take_while(|c| c.is_ascii_digit()).collect()
}
fn main() {
    let start = io::stdin().lock().lines().next().unwrap().unwrap();
    let (job, generation, instance) = (field(&start, "job_id"), field(&start, "generation"), field(&start, "instance_id"));
    if "__NAME__" == "gpu-stop-fail" {
        std::thread::sleep(std::time::Duration::from_millis(100));
        println!("{{\"version\":2,\"message\":{{\"Failed\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"code\":\"BackendUnavailable\",\"message\":\"injected delayed CUDA initialization failure\"}}}}}}");
        io::stdout().flush().unwrap();
        std::fs::write(MARKER, "failure-sent").unwrap();
        std::thread::sleep(std::time::Duration::from_secs(1));
        return;
    }
    if "__NAME__" == "cpu-marker" {
        std::fs::write(MARKER, "cpu-started").unwrap();
    }
    if "__NAME__" == "cpu-ready" {
        println!("{{\"version\":2,\"message\":{{\"Ready\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"backend\":\"Cpu\"}}}}}}");
        io::stdout().flush().unwrap();
        for command in io::stdin().lock().lines().flatten() {
            if command.contains("\"Stop\"") {
                println!("{{\"version\":2,\"message\":{{\"Stopped\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance}}}}}}}");
                io::stdout().flush().unwrap();
                return;
            }
        }
        return;
    }
    if "__NAME__" == "gpu" {
        println!("{{\"version\":2,\"message\":{{\"Failed\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"code\":\"BackendUnavailable\",\"message\":\"injected CUDA initialization failure\"}}}}}}");
        io::stdout().flush().unwrap();
    }
}
"#.replace("__NAME__", name).replace("__MARKER__", &marker);
        std::fs::write(&source, program).unwrap();
        let output = std::process::Command::new("rustc")
            .args([
                "--edition=2021",
                "--crate-name",
                "whisper_direct_startup_test",
            ])
            .arg(&source)
            .arg("-o")
            .arg(&executable)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        executable
    }

    #[test]
    fn auto_microphone_falls_back_while_capture_is_open_and_keeps_auto_preference() {
        let directory = test_directory("auto-startup");
        std::fs::create_dir_all(&directory).unwrap();
        let cpu = startup_worker(&directory, "cpu");
        let gpu = startup_worker(&directory, "gpu");
        let workers = crate::supervisor::WorkerPaths {
            cpu: cpu.clone(),
            gpu,
        };
        let mut live = super::DirectLive::start(
            startup_request(&directory, ComputeChoice::Auto),
            &workers,
            1,
            None,
            Some(startup_gpu_candidate()),
        )
        .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while live.request.config.compute == ComputeChoice::Gpu
            && std::time::Instant::now() < deadline
        {
            let _ = live.poll().unwrap();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(live.request.config.compute, ComputeChoice::Cpu);
        assert!(live.auto_preference);
        assert!(
            live.fallback_reason
                .as_ref()
                .is_some_and(|reason| reason.contains("CUDA"))
        );
        assert!(!live.worker_ready && live.capture.is_some());
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        let mut cpu_failure = None;
        while std::time::Instant::now() < deadline {
            match live.poll() {
                Err(error) => {
                    cpu_failure = Some(error);
                    break;
                }
                Ok(_) => std::thread::sleep(std::time::Duration::from_millis(10)),
            }
        }
        assert!(
            cpu_failure
                .as_deref()
                .is_some_and(|error| error.contains("CPU"))
        );
        assert_eq!(live.request.config.compute, ComputeChoice::Cpu);
        live.worker.as_mut().unwrap().abort();
        drop(live);
        std::fs::remove_file(directory.join(format!("micro-{:032x}.wav", 800))).unwrap();
        let absent = crate::supervisor::WorkerPaths {
            cpu,
            gpu: directory.join("missing-gpu.exe"),
        };
        let mut without_gpu = super::DirectLive::start(
            startup_request(&directory, ComputeChoice::Auto),
            &absent,
            3,
            None,
            None,
        )
        .unwrap();
        assert_eq!(without_gpu.request.config.compute, ComputeChoice::Cpu);
        assert_eq!(
            without_gpu.fallback_reason.as_deref(),
            Some("worker CUDA absent")
        );
        assert!(!without_gpu.worker_ready && without_gpu.capture.is_some());
        without_gpu.worker.as_mut().unwrap().abort();
        drop(without_gpu);
        std::fs::remove_file(directory.join(format!("micro-{:032x}.wav", 800))).unwrap();
        let mut forced = super::DirectLive::start(
            startup_request(&directory, ComputeChoice::Gpu),
            &workers,
            4,
            None,
            Some(startup_gpu_candidate()),
        )
        .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        let mut failed = false;
        while std::time::Instant::now() < deadline {
            if forced.poll().is_err() {
                failed = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(failed && !forced.auto_preference);
        assert_eq!(forced.request.config.compute, ComputeChoice::Gpu);
        assert!(forced.fallback_reason.is_none());
        forced.worker.as_mut().unwrap().abort();
        drop(forced);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn auto_without_gpu_worker_clears_candidate_and_accepts_cpu_ready() {
        let directory = test_directory("no-gpu-worker");
        std::fs::create_dir_all(&directory).unwrap();
        let cpu = startup_worker(&directory, "cpu-ready");
        let workers = crate::supervisor::WorkerPaths {
            cpu,
            gpu: directory.join("missing-gpu-worker.exe"),
        };
        let mut live = super::DirectLive::start(
            startup_request(&directory, ComputeChoice::Auto),
            &workers,
            21,
            None,
            Some(startup_gpu_candidate()),
        )
        .unwrap();
        assert_eq!(live.request.config.compute, ComputeChoice::Cpu);
        assert!(live.auto_preference);
        assert_eq!(live.fallback_reason.as_deref(), Some("worker CUDA absent"));
        assert!(live.gpu_candidate.is_none());
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while !live.worker_ready && std::time::Instant::now() < deadline {
            let _ = live.poll().unwrap();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(
            live.worker_ready,
            "CPU Ready must be accepted after Auto fallback"
        );
        assert_eq!(live.request.config.compute, ComputeChoice::Cpu);
        assert_eq!(live.fallback_reason.as_deref(), Some("worker CUDA absent"));
        live.stop().unwrap();
        drop(live);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn auto_microphone_stop_before_gpu_ready_keeps_cpu_retry_for_recorded_audio() {
        let directory = test_directory("stop-before-ready");
        std::fs::create_dir_all(&directory).unwrap();
        let cpu = startup_worker(&directory, "cpu-marker");
        let gpu = startup_worker(&directory, "gpu-stop-fail");
        let failure_marker = directory.join("auto-gpu-stop-fail.marker");
        let cpu_marker = directory.join("auto-cpu-marker.marker");
        let workers = crate::supervisor::WorkerPaths { cpu, gpu };
        let mut live = super::DirectLive::start(
            startup_request(&directory, ComputeChoice::Auto),
            &workers,
            20,
            None,
            Some(startup_gpu_candidate()),
        )
        .unwrap();
        live.stop().unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while !failure_marker.is_file() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(
            failure_marker.is_file(),
            "GPU failure was emitted after Stop"
        );
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while !cpu_marker.exists() && std::time::Instant::now() < deadline {
            let _ = live.poll();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(
            cpu_marker.exists(),
            "Stop keeps a CPU attempt before inference"
        );
        assert_eq!(live.request.config.compute, ComputeChoice::Cpu);
        assert!(live.auto_preference);
        drop(live);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn wav_header_describes_mono_16khz_pcm_and_exact_audio_length() {
        let mut bytes = Vec::new();
        write_wav_header(&mut bytes, 32_000).unwrap();
        assert_eq!(bytes.len(), 44);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(u32::from_le_bytes(bytes[4..8].try_into().unwrap()), 64_036);
        assert_eq!(&bytes[8..16], b"WAVEfmt ");
        assert_eq!(u16::from_le_bytes(bytes[20..22].try_into().unwrap()), 1);
        assert_eq!(u16::from_le_bytes(bytes[22..24].try_into().unwrap()), 1);
        assert_eq!(
            u32::from_le_bytes(bytes[24..28].try_into().unwrap()),
            16_000
        );
        assert_eq!(u16::from_le_bytes(bytes[34..36].try_into().unwrap()), 16);
        assert_eq!(&bytes[36..40], b"data");
        assert_eq!(
            u32::from_le_bytes(bytes[40..44].try_into().unwrap()),
            64_000
        );
    }

    #[test]
    fn wav_output_keeps_the_exact_converted_samples() {
        let directory = test_directory("wav-output");
        let job_id = whisper_core::JobId(
            ((std::process::id() as u128) << 64)
                | std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
        );
        let request = whisper_core::LiveRequest {
            job_id,
            generation: whisper_core::Generation::first(),
            model_path: "unused".into(),
            model_sha256: [0; 32],
            destination: directory.to_string_lossy().into_owned(),
            group_offset_samples: 0,
            auto_stop_after_speech_samples: None,
            microphone: Default::default(),
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Cpu,
            },
        };
        let mut wav = WavOutput::create(&request).unwrap();
        wav.append(&[-32768, 0]).unwrap();
        assert_eq!(wav.read_samples(0, 2).unwrap(), [-32768, 0]);
        wav.append(&[32767]).unwrap();
        assert_eq!(wav.read_samples(1, 2).unwrap(), [0, 32767]);
        let path = wav.finish().unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(bytes.len(), 50);
        assert_eq!(u32::from_le_bytes(bytes[40..44].try_into().unwrap()), 6);
        assert_eq!(&bytes[44..], &[0, 128, 0, 0, 255, 127]);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn stop_before_engine_ready_flushes_captured_audio_to_the_wav() {
        let directory = test_directory("stop-before-engine-ready");
        let mut request = startup_request(&directory, ComputeChoice::Cpu);
        request.job_id = whisper_core::JobId(
            ((std::process::id() as u128) << 64)
                | std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
        );
        let capture =
            crate::capture::CaptureStream::from_test_blocks(16_000, 1, vec![vec![0.25; 16_000]]);
        let converter = Pcm16Converter::new(16_000, 1).unwrap();
        let wav = WavOutput::create(&request).unwrap();
        let job_id = request.job_id;
        let mut live = DirectLive {
            request,
            auto_preference: false,
            fallback_reason: None,
            cpu_worker_path: PathBuf::new(),
            worker_path: PathBuf::new(),
            worker: None,
            instance: 1,
            worker_ready: false,
            gpu_attested: false,
            gpu_candidate: None,
            legacy_fallback: false,
            capture: Some(capture),
            converter: Some(converter),
            wav: Some(wav),
            diagnostics: None,
            pending: VecDeque::new(),
            next_offset: 0,
            next_window: 1,
            inflight: None,
            next_segment: 1,
            segments: Vec::new(),
            window_segments: Vec::new(),
            ready_events: VecDeque::new(),
            capture_announced: false,
            first_admitted_at_unix_ms: None,
            stop_requested: false,
            stop_sent: false,
            done: false,
        };

        live.stop().unwrap();

        assert!(!live.worker_ready);
        assert!(live.stop_requested);
        assert_eq!(live.wav.as_ref().unwrap().samples, 16_000);
        let samples = live.wav.as_mut().unwrap().read_samples(0, 16_000).unwrap();
        assert_eq!(samples.len(), 16_000);
        assert!(samples[320..15_680].iter().all(|sample| *sample == 8_192));
        drop(live);
        let path = directory.join(format!("micro-{:032x}.wav", job_id.0));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn wav_backlog_fills_bounded_windows_without_drops_or_reordering() {
        let directory = test_directory("wav-backlog");
        let job_id = whisper_core::JobId(
            ((std::process::id() as u128) << 64)
                | std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
        );
        let request = startup_request(&directory, ComputeChoice::Cpu);
        let mut request = request;
        request.job_id = job_id;
        let mut wav = WavOutput::create(&request).unwrap();
        let samples: Vec<i16> = (0..(MAX_BUFFERED_SAMPLES * 2 + 731))
            .map(|sample| (sample % 30_000) as i16)
            .collect();
        wav.append(&samples).unwrap();

        let mut pending = VecDeque::new();
        let mut next_offset = 0_u64;
        let mut observed = Vec::with_capacity(samples.len());
        while observed.len() < samples.len() {
            fill_pending_from_wav(&mut wav, next_offset, &mut pending).unwrap();
            assert!(!pending.is_empty());
            let count = pending.len().min(MAX_WINDOW_SAMPLES);
            observed.extend(pending.drain(..count));
            next_offset += count as u64;
        }
        assert_eq!(observed, samples);
        let path = wav.finish().unwrap();
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn simulated_gpu_failure_keeps_wav_and_confirmed_text_with_incomplete_suffix() {
        let directory = test_directory("simulated-gpu-failure");
        let job_id = whisper_core::JobId(
            ((std::process::id() as u128) << 64)
                | std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
        );
        let generation = whisper_core::Generation::first();
        let request = whisper_core::LiveRequest {
            job_id,
            generation,
            model_path: "unused".into(),
            model_sha256: [0; 32],
            destination: directory.to_string_lossy().into_owned(),
            group_offset_samples: 0,
            auto_stop_after_speech_samples: None,
            microphone: Default::default(),
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Gpu,
            },
        };
        let mut wav = WavOutput::create(&request).unwrap();
        wav.append(&[100, 200, 300, 400]).unwrap();
        let confirmed = WorkerSegment {
            job_id,
            generation,
            instance_id: 1,
            segment_id: SegmentId(1),
            range: SourceRange::new(0, 2, 16_000).unwrap(),
            text: "parole confirmée".into(),
        };
        let message = finalize_incomplete_outputs(
            &request,
            wav,
            &[confirmed],
            "GPU interrompu après un bloc".into(),
        );
        let stem = format!("micro-{:032x}", job_id.0);
        let wav_path = directory.join(format!("{stem}.wav"));
        let txt_path = directory.join(format!("{stem}-incomplet.txt"));
        let srt_path = directory.join(format!("{stem}-incomplet.srt"));
        assert!(message.contains("Réimportez le WAV en CPU"));
        assert!(message.contains(&wav_path.to_string_lossy().to_string()));
        assert_eq!(std::fs::read(&wav_path).unwrap().len(), 52);
        assert_eq!(
            std::fs::read_to_string(&txt_path).unwrap(),
            "parole confirmée"
        );
        assert!(
            std::fs::read_to_string(&srt_path)
                .unwrap()
                .contains("parole confirmée")
        );
        assert!(!directory.join(format!("{stem}.txt")).exists());
        for path in [wav_path, txt_path, srt_path] {
            std::fs::remove_file(path).unwrap();
        }
    }

    #[test]
    fn microphone_interruption_finalizes_cpu_outputs_without_gpu_message() {
        let directory = test_directory("microphone-interruption");
        let mut request = startup_request(&directory, ComputeChoice::Cpu);
        request.job_id = whisper_core::JobId(
            ((std::process::id() as u128) << 64)
                | std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
        );
        let mut wav = WavOutput::create(&request).unwrap();
        wav.append(&[100, 200]).unwrap();
        let message = finalize_incomplete_outputs(
            &request,
            wav,
            &[],
            "Micro interrompu par Windows : périphérique déconnecté".into(),
        );
        assert!(message.contains("Session micro arrêtée"));
        assert!(!message.contains("Session GPU"));
        let stem = format!("micro-{:032x}", request.job_id.0);
        for path in [
            directory.join(format!("{stem}.wav")),
            directory.join(format!("{stem}-incomplet.txt")),
            directory.join(format!("{stem}-incomplet.srt")),
        ] {
            assert!(path.is_file());
            std::fs::remove_file(path).unwrap();
        }
    }

    #[test]
    fn live_window_waits_for_five_seconds_then_prefers_a_pause_before_eight() {
        let mut pending = VecDeque::from(vec![1_000_i16; 96_000]);
        assert_eq!(window_size(&pending, false), None);
        for sample in pending.range_mut(80_000..90_000) {
            *sample = 0;
        }
        let cut = window_size(&pending, false).unwrap();
        assert!(cut > MIN_WINDOW_SAMPLES && cut < 90_000);
        assert_eq!(
            window_size(&VecDeque::from(vec![1_000; MAX_WINDOW_SAMPLES]), false),
            Some(MAX_WINDOW_SAMPLES)
        );
        assert_eq!(
            window_size(&VecDeque::from(vec![1_000; 32_000]), true),
            Some(32_000)
        );
    }
}
