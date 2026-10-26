use std::{
    collections::VecDeque,
    fs,
    fs::OpenOptions,
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
    microphone_probe::{MicrophoneProbe, MicrophoneSpectrumReader},
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
    probe: Option<MicrophoneProbe>,
    last_meter_sequence: u64,
    spectrum_reader: MicrophoneSpectrumReader,
    next_spectrum_telemetry: bool,
    telemetry_due_after_worker_event: bool,
    last_acknowledged_gain_revision: Option<u64>,
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
    #[cfg(test)]
    pub(crate) fn from_test_capture(
        request: LiveRequest,
        capture: CaptureStream,
    ) -> Result<Self, String> {
        let converter = Pcm16Converter::new(capture.sample_rate_hz(), capture.channels())?;
        let probe = MicrophoneProbe::start(
            capture.sample_rate_hz(),
            capture.source_identity().map(str::to_owned),
            request.job_id,
            request.generation,
        )?;
        let wav = WavOutput::create(&request)?;
        Ok(Self {
            request,
            auto_preference: false,
            fallback_reason: None,
            cpu_worker_path: PathBuf::new(),
            worker_path: PathBuf::new(),
            gpu_candidate: None,
            worker: None,
            instance: 1,
            worker_ready: false,
            gpu_attested: false,
            legacy_fallback: false,
            capture: Some(capture),
            probe: Some(probe),
            last_meter_sequence: 0,
            spectrum_reader: MicrophoneSpectrumReader::default(),
            next_spectrum_telemetry: false,
            telemetry_due_after_worker_event: false,
            last_acknowledged_gain_revision: None,
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
        })
    }

    #[cfg(test)]
    pub(crate) fn from_test_capture_with_cpu_worker(
        request: LiveRequest,
        capture: CaptureStream,
        worker_path: PathBuf,
        instance: u64,
    ) -> Result<Self, String> {
        if request.config.compute != ComputeChoice::Cpu {
            return Err("D2 live harness requires an explicit CPU request".into());
        }
        let worker = spawn_direct_worker(&request, &worker_path, instance, None, None)?;
        let converter = Pcm16Converter::new(capture.sample_rate_hz(), capture.channels())?;
        let probe = MicrophoneProbe::start(
            capture.sample_rate_hz(),
            capture.source_identity().map(str::to_owned),
            request.job_id,
            request.generation,
        )?;
        let wav = WavOutput::create(&request)?;
        Ok(Self {
            request,
            auto_preference: false,
            fallback_reason: None,
            cpu_worker_path: worker_path.clone(),
            worker_path,
            gpu_candidate: None,
            worker: Some(worker),
            instance,
            worker_ready: false,
            gpu_attested: false,
            legacy_fallback: false,
            capture: Some(capture),
            probe: Some(probe),
            last_meter_sequence: 0,
            spectrum_reader: MicrophoneSpectrumReader::default(),
            next_spectrum_telemetry: false,
            telemetry_due_after_worker_event: false,
            last_acknowledged_gain_revision: None,
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
        })
    }

    #[cfg(test)]
    pub(crate) fn finish_test_wav(&mut self) -> Result<String, String> {
        self.wav.take().ok_or("test WAV is absent")?.finish()
    }

    #[cfg(test)]
    pub(crate) fn test_worker_ready(&self) -> bool {
        self.worker_ready
    }

    #[cfg(test)]
    pub(crate) fn enqueue_test_capture_block(&mut self, samples: Vec<f32>) {
        self.capture
            .as_mut()
            .expect("test capture remains active")
            .enqueue_test_block(samples);
    }

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
        let initial_gain = whisper_core::microphone_level::GainDb::new(request.microphone_gain_db)
            .map_err(|error| format!("MicrophoneLevel: {error}"))?;
        let mut capture =
            match crate::capture::start_direct_with_gain(&request.microphone, initial_gain) {
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
        let probe = match MicrophoneProbe::start(
            capture.sample_rate_hz(),
            capture.source_identity().map(str::to_owned),
            request.job_id,
            request.generation,
        ) {
            Ok(probe) => probe,
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
            probe: Some(probe),
            last_meter_sequence: 0,
            spectrum_reader: MicrophoneSpectrumReader::default(),
            next_spectrum_telemetry: false,
            telemetry_due_after_worker_event: false,
            last_acknowledged_gain_revision: None,
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

    pub(crate) fn set_gain(
        &self,
        target_identity: &str,
        expected_revision: u64,
        gain: whisper_core::microphone_level::GainDb,
    ) -> Result<whisper_core::microphone_level::GainRevision, String> {
        let capture = self.capture.as_ref().ok_or("Capture microphone arrêtée")?;
        if capture.source_identity() != Some(target_identity) {
            return Err("MicrophoneLevel: source active différente de la cible".into());
        }
        capture.set_gain(expected_revision, gain)
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
        if let Some(probe) = &self.probe {
            probe.invalidate();
        }
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
        if self.telemetry_due_after_worker_event {
            self.telemetry_due_after_worker_event = false;
            if let Some(event) = self.take_next_probe_telemetry() {
                return Ok(Some(event));
            }
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
            self.telemetry_due_after_worker_event = true;
            return Ok(Some(result));
        }
        self.send_next()?;
        Ok(self.take_next_probe_telemetry())
    }

    fn take_next_probe_telemetry(&mut self) -> Option<ImportEvent> {
        if self.next_spectrum_telemetry {
            self.next_spectrum_telemetry = false;
            self.take_probe_spectrum()
                .or_else(|| self.take_probe_meter())
        } else {
            self.next_spectrum_telemetry = true;
            self.take_probe_meter()
                .or_else(|| self.take_probe_spectrum())
        }
    }

    pub(crate) fn pump_capture(&mut self) -> Result<(), String> {
        if !self.stop_requested && self.capture.is_some() {
            self.drain_capture()?;
        }
        Ok(())
    }

    fn take_probe_meter(&mut self) -> Option<ImportEvent> {
        let snapshot = self.probe.as_ref()?.latest_at(std::time::Instant::now())?;
        if snapshot.sequence <= self.last_meter_sequence {
            return None;
        }
        let meter = snapshot.meter()?;
        self.last_meter_sequence = snapshot.sequence;
        Some(ImportEvent::MicrophoneMeter(meter))
    }

    fn take_probe_spectrum(&mut self) -> Option<ImportEvent> {
        self.spectrum_reader
            .poll(self.probe.as_ref()?, std::time::Instant::now())
            .map(ImportEvent::MicrophoneSpectrum)
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
            let block_gain = block.gain();
            let block_first_frame = block.first_frame();
            if self
                .last_acknowledged_gain_revision
                .is_none_or(|revision| block_gain.revision > revision)
            {
                self.last_acknowledged_gain_revision = Some(block_gain.revision);
                if let Some(target_identity) = capture.source_identity().map(str::to_owned) {
                    self.ready_events
                        .push_back(ImportEvent::MicrophoneGainApplied {
                            job_id: self.request.job_id,
                            generation: self.request.generation,
                            target_identity,
                            gain_db: block_gain.gain.value(),
                            gain_revision: block_gain.revision,
                            first_frame: block_first_frame,
                        });
                }
            }
            let converted = self
                .converter
                .as_mut()
                .expect("converter ready")
                .push_with_probe(&block)?;
            capture.recycle_block(block)?;
            if let Some(probe) = &self.probe {
                probe.submit(converted.probe);
            }
            self.wav
                .as_mut()
                .expect("wav ready")
                .append(&converted.samples)?;
            if !converted.samples.is_empty() && self.first_admitted_at_unix_ms.is_none() {
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

pub(crate) fn export_segments(
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
    let mut ordered = segments.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|segment| {
        (
            segment.range.start_sample,
            segment.range.end_sample,
            segment.segment_id.0,
        )
    });
    let text = ordered
        .iter()
        .map(|segment| segment.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let subtitles = ordered
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
    let mut txt_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&txt)
        .map_err(|error| format!("TXT non enregistré. Vérifiez le dossier de sortie : {error}"))?;
    if let Err(error) = txt_file
        .write_all(text.as_bytes())
        .and_then(|()| txt_file.flush())
    {
        drop(txt_file);
        let _ = fs::remove_file(&txt);
        return Err(format!(
            "TXT non enregistré. Vérifiez le dossier de sortie : {error}"
        ));
    }
    drop(txt_file);
    let mut srt_file = match OpenOptions::new().write(true).create_new(true).open(&srt) {
        Ok(file) => file,
        Err(error) => {
            let cleanup = fs::remove_file(&txt).err();
            let suffix = cleanup.map_or_else(String::new, |cleanup| {
                format!("; TXT temporaire créé par cette exportation non retiré : {cleanup}")
            });
            return Err(format!(
                "SRT non enregistré. Vérifiez le dossier de sortie : {error}{suffix}"
            ));
        }
    };
    if let Err(error) = srt_file
        .write_all(subtitles.as_bytes())
        .and_then(|()| srt_file.flush())
    {
        drop(srt_file);
        let srt_cleanup = fs::remove_file(&srt).err();
        let txt_cleanup = fs::remove_file(&txt).err();
        let suffix = [srt_cleanup, txt_cleanup]
            .into_iter()
            .flatten()
            .map(|cleanup| format!("; sortie créée par cette exportation non retirée : {cleanup}"))
            .collect::<String>();
        return Err(format!(
            "SRT non enregistré. Vérifiez le dossier de sortie : {error}{suffix}"
        ));
    }
    Ok((
        txt.to_string_lossy().into_owned(),
        srt.to_string_lossy().into_owned(),
    ))
}

pub(crate) fn automatic_final_output_paths(
    wav_path: &str,
) -> Result<whisper_core::ports::AutomaticFinalOutputPaths, String> {
    let wav = Path::new(wav_path);
    let parent = wav
        .parent()
        .ok_or_else(|| "WAV sans dossier parent".to_owned())?;
    let stem = wav
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Nom de capture WAV invalide".to_owned())?;
    Ok(whisper_core::ports::AutomaticFinalOutputPaths {
        txt_path: parent
            .join(format!("{stem}-auto-non-verifie.txt"))
            .to_string_lossy()
            .into_owned(),
        srt_path: parent
            .join(format!("{stem}-auto-non-verifie.srt"))
            .to_string_lossy()
            .into_owned(),
        companion_json_path: wav.with_extension("json").to_string_lossy().into_owned(),
    })
}

pub(crate) fn export_parakeet_manifest(
    request: &LiveRequest,
    wav_path: &str,
    audio_samples: u64,
    segments: &[WorkerSegment],
    unresolved: &[SourceRange],
    complete: bool,
) -> Result<String, String> {
    let wav = PathBuf::from(wav_path);
    let mut json = wav.clone();
    json.set_extension("json");
    let mut ordered = segments.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|segment| {
        (
            segment.range.start_sample,
            segment.range.end_sample,
            segment.segment_id.0,
        )
    });
    let automatic_outputs = automatic_final_output_paths(wav_path)?;
    let document = serde_json::json!({
        "version": 1,
        "capture_id": format!("{:032x}", request.job_id.0),
        "capture_name": wav.file_name().and_then(|name| name.to_str()).unwrap_or_default(),
        "audio_samples": audio_samples,
        "sample_rate_hz": 16_000,
        "complete": complete,
        "confirmed_intervals": ordered.iter().map(|segment| serde_json::json!({
            "segment_id": segment.segment_id.0,
            "start_sample": segment.range.start_sample.saturating_sub(request.group_offset_samples),
            "end_sample": segment.range.end_sample.saturating_sub(request.group_offset_samples),
            "sample_rate_hz": segment.range.sample_rate_hz,
            "text": segment.text.as_str(),
            "status": "confirmed",
        })).collect::<Vec<_>>(),
        "unresolved_intervals": unresolved.iter().map(|range| serde_json::json!({
            "start_sample": range.start_sample.saturating_sub(request.group_offset_samples),
            "end_sample": range.end_sample.saturating_sub(request.group_offset_samples),
            "sample_rate_hz": range.sample_rate_hz,
            "status": "unresolved",
        })).collect::<Vec<_>>(),
        "automatic_final_transcription": {
            "status": "pending",
            "txt_path": automatic_outputs.txt_path,
            "srt_path": automatic_outputs.srt_path,
            "model_sha256": request.model_sha256.iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>(),
            "requested_compute": format!("{:?}", request.config.compute).to_ascii_lowercase(),
            "effective_backend": serde_json::Value::Null,
            "diagnostic": serde_json::Value::Null,
        },
    });
    let bytes =
        serde_json::to_vec_pretty(&document).map_err(|error| format!("JSON invalide : {error}"))?;
    fs::write(&json, bytes).map_err(|error| format!("écriture JSON impossible : {error}"))?;
    Ok(json.to_string_lossy().into_owned())
}

pub(crate) struct ParakeetExportOutcome {
    pub txt_path: Option<String>,
    pub srt_path: Option<String>,
    pub complete: bool,
    pub diagnostics: Vec<String>,
}

pub(crate) fn export_parakeet_outputs(
    request: &LiveRequest,
    wav_path: &str,
    audio_samples: u64,
    segments: &[WorkerSegment],
    unresolved: &[SourceRange],
    mut complete: bool,
) -> ParakeetExportOutcome {
    let mut diagnostics = Vec::new();
    let (txt_path, srt_path) = match export_segments(request, segments, !complete) {
        Ok(paths) => (Some(paths.0), Some(paths.1)),
        Err(error) => {
            diagnostics.push(format!("TXT/SRT non enregistrés : {error}"));
            complete = false;
            match export_segments(request, segments, true) {
                Ok(paths) => (Some(paths.0), Some(paths.1)),
                Err(retry_error) => {
                    diagnostics.push(format!(
                        "TXT/SRT incomplets non enregistrés : {retry_error}"
                    ));
                    (None, None)
                }
            }
        }
    };
    if wav_path.is_empty() {
        diagnostics.push("JSON de capture non enregistré : chemin WAV indisponible".into());
        complete = false;
    } else if let Err(error) = export_parakeet_manifest(
        request,
        wav_path,
        audio_samples,
        segments,
        unresolved,
        complete,
    ) {
        diagnostics.push(format!("JSON de capture non enregistré : {error}"));
        complete = false;
    }
    ParakeetExportOutcome {
        txt_path,
        srt_path,
        complete,
        diagnostics,
    }
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

pub(crate) struct WavOutput {
    file: fs::File,
    path: PathBuf,
    samples: u64,
}

impl WavOutput {
    pub(crate) fn create(request: &LiveRequest) -> Result<Self, String> {
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

    pub(crate) fn append(&mut self, samples: &[i16]) -> Result<(), String> {
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

    pub(crate) fn read_samples(&mut self, offset: u64, count: usize) -> Result<Vec<i16>, String> {
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

    pub(crate) fn sample_count(&self) -> u64 {
        self.samples
    }

    pub(crate) fn finish(mut self) -> Result<String, String> {
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
        Pcm16Converter, WavOutput, clear_gpu_proof_for_cpu, export_parakeet_manifest,
        export_parakeet_outputs, export_segments, fill_pending_from_wav,
        finalize_incomplete_outputs, gpu_ready_requires_attestation,
        should_retry_auto_gpu_before_ready, window_size, write_wav_header,
    };
    use crate::microphone_probe::{MicrophoneProbe, MicrophoneSpectrumReader};
    use std::collections::VecDeque;
    use std::path::Path;
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
    fn direct_capture_publishes_frequency_spectrum_without_changing_wav_audio() {
        let directory = test_directory("spectrum-direct");
        std::fs::create_dir_all(&directory).unwrap();
        let mut request = startup_request(&directory, ComputeChoice::Cpu);
        request.microphone = whisper_core::MicrophoneChoice::Device("test:mic".into());
        let mut capture = crate::capture::CaptureStream::empty_test_capture(16_000, 1, "test:mic");
        let mut live = super::DirectLive::from_test_capture(request, capture).unwrap();
        let input_blocks = (0..5)
            .map(|block| {
                (block * 960..(block + 1) * 960)
                    .map(|frame| 0.5 * (std::f32::consts::TAU * 64.0 * frame as f32 / 2048.0).sin())
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        for audio in &input_blocks {
            live.capture
                .as_mut()
                .unwrap()
                .enqueue_test_block(audio.clone());
            live.pump_capture().unwrap();
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let event = loop {
            if let Some(event) = live.take_probe_spectrum() {
                break event;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "direct spectrum not published"
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        };
        let whisper_core::ImportEvent::MicrophoneSpectrum(
            whisper_core::microphone_level::MicrophoneSpectrumUpdate::Available(spectrum),
        ) = event
        else {
            panic!("expected a current direct spectrum")
        };
        assert_eq!(spectrum.source_identity, "test:mic");
        let peak = spectrum
            .pre_gain
            .iter()
            .enumerate()
            .filter_map(|(index, bin)| match bin {
                whisper_core::microphone_level::SpectrumBin::MilliDb(value) => Some((index, value)),
                whisper_core::microphone_level::SpectrumBin::Silence => None,
            })
            .max_by_key(|(_, value)| *value)
            .unwrap()
            .0;
        let hz = peak as f64 * f64::from(spectrum.sample_rate_hz) / 2048.0;
        assert!((hz - 500.0).abs() < 10.0, "expected 500 Hz, got {hz} Hz");
        let baseline_dir = directory.join("without-probe");
        std::fs::create_dir_all(&baseline_dir).unwrap();
        let mut baseline_request = startup_request(&baseline_dir, ComputeChoice::Cpu);
        baseline_request.job_id = whisper_core::JobId(7_202);
        let mut baseline_capture =
            crate::capture::CaptureStream::empty_test_capture(16_000, 1, "test:mic");
        let mut baseline_converter = crate::capture::Pcm16Converter::new(16_000, 1).unwrap();
        let mut baseline_wav = WavOutput::create(&baseline_request).unwrap();
        for audio in &input_blocks {
            baseline_capture.enqueue_test_block(audio.clone());
            let block = baseline_capture.try_next_block().unwrap();
            let pcm = baseline_converter.push(&block).unwrap();
            baseline_wav.append(&pcm).unwrap();
            baseline_capture.recycle_block(block).unwrap();
        }
        baseline_wav
            .append(&baseline_converter.finish().unwrap())
            .unwrap();
        let baseline_path = baseline_wav.finish().unwrap();
        live.stop().unwrap();
        let live_samples = live.wav.as_ref().unwrap().sample_count();
        assert_eq!(
            live_samples, 4_800,
            "probe preserves saved PCM sample count"
        );
        let live_path = live.finish_test_wav().unwrap();
        assert_eq!(
            std::fs::read(live_path).unwrap(),
            std::fs::read(baseline_path).unwrap(),
            "Turbo WAV with spectrum reader is byte-identical to the no-reader baseline"
        );
        drop(live);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn direct_capture_poll_keeps_spectrum_text_and_terminal_progress_within_four_events() {
        let directory = test_directory("spectrum-direct-poll");
        std::fs::create_dir_all(&directory).unwrap();
        let mut request = startup_request(&directory, ComputeChoice::Cpu);
        request.microphone = whisper_core::MicrophoneChoice::Device("test:mic".into());
        let capture = crate::capture::CaptureStream::empty_test_capture(16_000, 1, "test:mic");
        let worker = startup_worker(&directory, "cpu-poll-spectrum");
        let mut live =
            DirectLive::from_test_capture_with_cpu_worker(request, capture, worker, 91).unwrap();

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        let mut saw_ready = false;
        while !saw_ready && std::time::Instant::now() < deadline {
            if matches!(
                live.poll().unwrap(),
                Some(whisper_core::ImportEvent::Ready { .. })
            ) {
                saw_ready = true;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(saw_ready, "simulated direct worker did not become ready");

        let mut saw_spectrum = false;
        let mut spectrum_updates = 0;
        let mut saw_confirmed_text = false;
        let mut saw_terminal = false;
        for block_index in 0..100 {
            let first_frame = block_index * 960;
            let audio = (first_frame..first_frame + 960)
                .map(|frame| 0.5 * (std::f32::consts::TAU * 64.0 * frame as f32 / 2048.0).sin())
                .collect::<Vec<_>>();
            live.capture.as_mut().unwrap().enqueue_test_block(audio);
            let mut events_in_refresh = 0;
            for _ in 0..4 {
                events_in_refresh += 1;
                if let Some(event) = live.poll().unwrap() {
                    match event {
                        whisper_core::ImportEvent::MicrophoneSpectrum(
                            whisper_core::microphone_level::MicrophoneSpectrumUpdate::Available(_),
                        ) => {
                            saw_spectrum = true;
                            spectrum_updates += 1;
                        }
                        whisper_core::ImportEvent::DirectConfirmed(segment) => {
                            saw_confirmed_text = !segment.text.trim().is_empty();
                        }
                        whisper_core::ImportEvent::DirectPublished { .. } => saw_terminal = true,
                        _ => {}
                    }
                }
                if saw_terminal {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(3));
            }
            assert!(events_in_refresh <= 4);
            std::thread::sleep(std::time::Duration::from_millis(8));
        }

        live.stop().unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while !saw_terminal && std::time::Instant::now() < deadline {
            if let Some(event) = live.poll().unwrap() {
                match event {
                    whisper_core::ImportEvent::MicrophoneSpectrum(
                        whisper_core::microphone_level::MicrophoneSpectrumUpdate::Available(_),
                    ) => {
                        saw_spectrum = true;
                        spectrum_updates += 1;
                    }
                    whisper_core::ImportEvent::DirectConfirmed(segment) => {
                        saw_confirmed_text = !segment.text.trim().is_empty();
                    }
                    whisper_core::ImportEvent::DirectPublished { .. } => saw_terminal = true,
                    _ => {}
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        assert!(
            saw_spectrum,
            "poll() did not publish spectrum during capture"
        );
        assert!(
            spectrum_updates > 1,
            "spectrum did not refresh during the stream"
        );
        assert!(saw_confirmed_text, "poll() starved confirmed worker text");
        assert!(saw_terminal, "poll() starved the worker terminal event");
        assert!(live.done());
        drop(live);
        let _ = std::fs::remove_dir_all(directory);
    }

    #[test]
    fn parakeet_live_automatic_final_paths_are_distinct_and_explicitly_unverified() {
        let wav = Path::new(r"C:\captures\micro-123.wav");
        let paths = super::automatic_final_output_paths(&wav.to_string_lossy()).unwrap();
        assert_eq!(
            paths.txt_path,
            r"C:\captures\micro-123-auto-non-verifie.txt"
        );
        assert_eq!(
            paths.srt_path,
            r"C:\captures\micro-123-auto-non-verifie.srt"
        );
        assert_eq!(paths.companion_json_path, r"C:\captures\micro-123.json");
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
            microphone_gain_db: 0,
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
    if "__NAME__" == "cpu-poll-spectrum" {
        println!("{{\"version\":2,\"message\":{{\"Ready\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"backend\":\"Cpu\"}}}}}}");
        io::stdout().flush().unwrap();
        let mut segment_id = 1;
        for command in io::stdin().lock().lines().flatten() {
            if command.contains("\"LiveWindow\"") {
                let sequence = field(&command, "sequence");
                let start_sample = field(&command, "start_sample");
                let end_sample = field(&command, "end_sample");
                let sample_rate_hz = field(&command, "sample_rate_hz");
                println!("{{\"version\":2,\"message\":{{\"Segment\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"segment_id\":{segment_id},\"range\":{{\"start_sample\":{start_sample},\"end_sample\":{end_sample},\"sample_rate_hz\":{sample_rate_hz}}},\"text\":\"simulated confirmed text\"}}}}}}");
                io::stdout().flush().unwrap();
                println!("{{\"version\":2,\"message\":{{\"WindowFinished\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance},\"sequence\":{sequence},\"range\":{{\"start_sample\":{start_sample},\"end_sample\":{end_sample},\"sample_rate_hz\":{sample_rate_hz}}},\"last_segment_sequence\":{segment_id}}}}}}}");
                io::stdout().flush().unwrap();
                segment_id += 1;
            } else if command.contains("\"Stop\"") {
                println!("{{\"version\":2,\"message\":{{\"Stopped\":{{\"job_id\":{job},\"generation\":{generation},\"instance_id\":{instance}}}}}}}");
                io::stdout().flush().unwrap();
                return;
            }
        }
        return;
    }
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
            microphone_gain_db: 0,
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
        let probe = MicrophoneProbe::start(
            capture.sample_rate_hz(),
            capture.source_identity().map(str::to_owned),
            request.job_id,
            request.generation,
        )
        .unwrap();
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
            probe: Some(probe),
            last_meter_sequence: 0,
            spectrum_reader: MicrophoneSpectrumReader::default(),
            next_spectrum_telemetry: false,
            telemetry_due_after_worker_event: false,
            last_acknowledged_gain_revision: None,
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
    fn gain_command_changes_only_subsequently_acquired_audio_and_stop_finalizes_wav() {
        let directory = test_directory("gain-next-acquisition");
        let mut request = startup_request(&directory, ComputeChoice::Cpu);
        request.microphone = whisper_core::MicrophoneChoice::Device("test:mic".into());
        request.job_id = whisper_core::JobId(
            ((std::process::id() as u128) << 64)
                | std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
        );
        let mut capture = crate::capture::CaptureStream::empty_test_capture(16_000, 1, "test:mic");
        capture.enqueue_test_block(vec![0.25; 16_000]);
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
            probe: None,
            last_meter_sequence: 0,
            spectrum_reader: MicrophoneSpectrumReader::default(),
            next_spectrum_telemetry: false,
            telemetry_due_after_worker_event: false,
            last_acknowledged_gain_revision: None,
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

        live.pump_capture().unwrap();
        let applied = live
            .set_gain("test:mic", 0, whisper_core::GainDb::new(6).unwrap())
            .unwrap();
        assert_eq!(applied.revision, 1);
        live.capture
            .as_mut()
            .unwrap()
            .enqueue_test_block(vec![0.25; 16_000]);
        live.stop().unwrap();

        let wav = live.wav.as_mut().unwrap();
        assert_eq!(wav.samples, 32_000);
        let samples = wav.read_samples(0, 32_000).unwrap();
        assert_eq!(samples.len(), 32_000);
        assert!(samples[320..15_680].iter().all(|sample| *sample == 8_192));
        assert!(
            samples[16_320..31_680]
                .iter()
                .all(|sample| (16_300..=16_400).contains(sample))
        );
        drop(live);
        std::fs::remove_file(directory.join(format!("micro-{:032x}.wav", job_id.0))).unwrap();
    }

    #[test]
    fn test_capture_gain_preserves_probe_a_and_applies_once_to_wav_pcm() {
        let directory = test_directory("gain-single-application");
        let mut request = startup_request(&directory, ComputeChoice::Cpu);
        request.microphone = whisper_core::MicrophoneChoice::Device("test:gain-once".into());
        request.microphone_gain_db = 6;
        let gain = whisper_core::GainDb::new(6).unwrap();
        let source = (0..16_000)
            .map(|index| if index < 8_000 { 0.25_f32 } else { 0.75_f32 })
            .collect::<Vec<_>>();

        let mut capture = crate::capture::CaptureStream::empty_test_capture_with_gain(
            16_000,
            1,
            "test:gain-once",
            gain,
        );
        capture.enqueue_test_block(source.clone());
        let block = capture.try_next_block().unwrap();
        assert_eq!(block.gain().revision, 0);
        assert_eq!(block.gain().gain, gain);
        assert!(
            block
                .samples()
                .iter()
                .zip(&source)
                .all(|(actual, expected)| (actual - expected).abs() <= f32::EPSILON)
        );

        let mut probe_converter = Pcm16Converter::new(16_000, 1).unwrap();
        let probe = probe_converter.push_with_probe(&block).unwrap().probe;
        assert!(
            probe
                .mono_pre_gain
                .iter()
                .zip(&source)
                .all(|(actual, expected)| (actual - expected).abs() <= f32::EPSILON)
        );
        let factor = 10.0_f32.powf(6.0 / 20.0);
        for (index, actual) in probe.mono_post_gain_limiter.iter().enumerate() {
            let expected = (source[index] * factor).clamp(-1.0, 1.0);
            assert!((actual - expected).abs() <= 1.0e-6);
        }
        capture.recycle_block(block).unwrap();

        let mut live = DirectLive::from_test_capture(request, capture).unwrap();
        live.enqueue_test_capture_block(source);
        live.stop().unwrap();
        let wav = live.wav.as_mut().unwrap();
        assert_eq!(wav.samples, 16_000);
        let pcm = wav.read_samples(0, 16_000).unwrap();
        let expected_unclamped = ((0.25_f32 * factor).clamp(-1.0, 1.0) * 32_767.0).round() as i16;
        assert!(
            pcm[500..7_500]
                .iter()
                .all(|sample| *sample == expected_unclamped)
        );
        assert!(pcm[8_500..15_500].iter().all(|sample| *sample == i16::MAX));
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
            microphone_gain_db: 0,
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

    #[test]
    fn parakeet_exports_sort_time_and_write_versioned_companion_intervals() {
        let directory = test_directory("parakeet-companion");
        std::fs::create_dir_all(&directory).unwrap();
        let request = startup_request(&directory, ComputeChoice::Cpu);
        let wav_path = directory.join(format!("micro-{:032x}.wav", request.job_id.0));
        std::fs::write(&wav_path, b"wav").unwrap();
        let later = WorkerSegment {
            job_id: request.job_id,
            generation: request.generation,
            instance_id: 7,
            segment_id: SegmentId(1),
            range: SourceRange::new(8_000, 12_000, 16_000).unwrap(),
            text: "fonctionne".into(),
        };
        let earlier = WorkerSegment {
            segment_id: SegmentId(2),
            range: SourceRange::new(2_000, 6_000, 16_000).unwrap(),
            text: "test".into(),
            ..later.clone()
        };
        let segments = [later, earlier];
        let unresolved = [SourceRange::new(0, 1_000, 16_000).unwrap()];
        let (_, _) = export_segments(&request, &segments, true).unwrap();
        let manifest = export_parakeet_manifest(
            &request,
            &wav_path.to_string_lossy(),
            16_000,
            &segments,
            &unresolved,
            false,
        )
        .unwrap();
        let text_path = directory.join(format!("micro-{:032x}-incomplet.txt", request.job_id.0));
        assert_eq!(
            std::fs::read_to_string(text_path).unwrap(),
            "test\nfonctionne"
        );
        let document: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&manifest).unwrap()).unwrap();
        assert_eq!(document["version"], 1);
        assert_eq!(document["complete"], false);
        assert_eq!(document["audio_samples"], 16_000);
        assert_eq!(document["confirmed_intervals"][0]["segment_id"], 2);
        assert_eq!(document["confirmed_intervals"][0]["text"], "test");
        assert_eq!(document["unresolved_intervals"][0]["start_sample"], 0);
        for path in [
            wav_path,
            directory.join(format!("micro-{:032x}-incomplet.txt", request.job_id.0)),
            directory.join(format!("micro-{:032x}-incomplet.srt", request.job_id.0)),
            directory.join(format!("micro-{:032x}.json", request.job_id.0)),
        ] {
            std::fs::remove_file(path).unwrap();
        }
    }

    #[test]
    fn parakeet_srt_failure_removes_created_txt_and_retries_coherent_incomplete_outputs() {
        let directory = test_directory("parakeet-export-failure");
        std::fs::create_dir_all(&directory).unwrap();
        let request = startup_request(&directory, ComputeChoice::Cpu);
        let stem = format!("micro-{:032x}", request.job_id.0);
        let wav_path = directory.join(format!("{stem}.wav"));
        std::fs::write(&wav_path, b"wav").unwrap();
        let blocked_complete_srt = directory.join(format!("{stem}.srt"));
        std::fs::create_dir(&blocked_complete_srt).unwrap();
        let segment = WorkerSegment {
            job_id: request.job_id,
            generation: request.generation,
            instance_id: 7,
            segment_id: SegmentId(1),
            range: SourceRange::new(1_000, 2_000, 16_000).unwrap(),
            text: "confirmé".into(),
        };

        let outcome = export_parakeet_outputs(
            &request,
            &wav_path.to_string_lossy(),
            16_000,
            &[segment],
            &[],
            true,
        );

        assert!(!outcome.complete);
        assert_eq!(outcome.diagnostics.len(), 1);
        assert!(outcome.diagnostics[0].contains("SRT non enregistré"));
        let txt_path = outcome.txt_path.unwrap();
        let srt_path = outcome.srt_path.unwrap();
        assert!(txt_path.ends_with("-incomplet.txt"));
        assert!(srt_path.ends_with("-incomplet.srt"));
        assert!(std::path::Path::new(&txt_path).is_file());
        assert!(std::path::Path::new(&srt_path).is_file());
        assert!(!directory.join(format!("{stem}.txt")).exists());
        assert!(blocked_complete_srt.is_dir());
        let manifest_path = directory.join(format!("{stem}.json"));
        let manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
        assert_eq!(manifest["complete"], false);

        std::fs::remove_dir(blocked_complete_srt).unwrap();
        for path in [
            wav_path,
            PathBuf::from(txt_path),
            PathBuf::from(srt_path),
            manifest_path,
        ] {
            std::fs::remove_file(path).unwrap();
        }
    }
}

#[cfg(test)]
mod d2_corpus {
    use crate::{
        capture::CaptureStream, direct_live::DirectLive, parakeet_import::ParakeetWorkerPaths,
    };
    use sha2::{Digest, Sha256};
    use std::{
        collections::VecDeque,
        fs::{self, OpenOptions},
        io::Write,
        path::{Path, PathBuf},
        sync::atomic::{AtomicU64, Ordering},
        time::{Duration, Instant},
    };
    use whisper_core::{
        ComputeChoice, Generation, ImportEvent, JobConfig, JobId, LanguageChoice, LiveRequest,
        microphone_level::GainDb, model_catalog::ModelId,
    };

    static CASE_ID: AtomicU64 = AtomicU64::new(1);
    const INPUTS: [(&str, &str, &str, &str, &str); 4] = [
        (
            "fr_pauses",
            "sound example/voxpopuli/clips/long_examples/fr_long_2020.wav",
            "docs/cadrage/feas-pm-01-corpus/candidate_fr_pauses_copie.txt",
            "541285a38fb6927c1bb4384825d2a7565a4a41efeade91c5f13de60bfefc5c0e",
            "abd2ac3ec6bc0fceb7c044f56d8ee0c29c4b2ca4a2ba8ca364ee169cb47e78cf",
        ),
        (
            "en_pauses",
            "sound example/voxpopuli/clips/long_examples/en_long_2020.wav",
            "docs/cadrage/feas-pm-01-corpus/candidate_en_pauses_copie.txt",
            "d73e3c08972ccb39eb3fc70ffb202c95c2b386fb3e59b5fafb753dab2c2ec18a",
            "c2971f66c57658ca613d578df807559604b5581c8b5ef72102900700fcf33cb1",
        ),
        (
            "fr_continu",
            "sound example/voxpopuli/clips/long_examples/fr_continuous60_2020_CANDIDATE.wav",
            "docs/cadrage/feas-pm-01-corpus/candidate_fr_continu_copie.txt",
            "322757782eda5332bdec4ff8bc7bbd87a5bbb76330d0d4da7f6e4d271f5a0df2",
            "ed150b6360d409b4d575b23253db5849dd709ab4f1b99416f29ba7d18a921ca7",
        ),
        (
            "en_continu",
            "sound example/voxpopuli/clips/long_examples/en_continuous60_2020_CANDIDATE.wav",
            "docs/cadrage/feas-pm-01-corpus/candidate_en_continu_copie.txt",
            "1fdcf0d8c00c426c7ed78c6724087db857b3acde465ea1f8bd77a3be8ca4b7c8",
            "93a7ae7ac4c940f98d64eecf5ef6ce0bed883da4202a4df5762b4f4782bf8bad",
        ),
    ];
    const CONDITIONS: [(&str, i8, Option<i8>); 4] = [
        ("0db", 0, None),
        ("minus6db", -6, None),
        ("plus6db", 6, None),
        ("abrupt-minus6-to-plus6", -6, Some(6)),
    ];
    const D2_POLL_INTERVAL: Duration = Duration::from_millis(5);
    const D2_MAX_EVENTS_PER_DRAIN: usize = 32;
    const D2_MAX_DRAIN_BATCHES: usize = 8;

    fn d2_frame_deadline(playback_origin: Instant, frame_end: usize) -> Instant {
        let frames = frame_end as u64;
        let seconds = frames / 16_000;
        let remainder = frames % 16_000;
        playback_origin
            + Duration::from_secs(seconds)
            + Duration::from_nanos(remainder * 1_000_000_000 / 16_000)
    }

    fn d2_rebase_if_late(playback_origin: &mut Instant, frame_end: usize, now: Instant) -> bool {
        let deadline = d2_frame_deadline(*playback_origin, frame_end);
        if now < deadline {
            return false;
        }
        let elapsed = deadline.duration_since(*playback_origin);
        *playback_origin = now.checked_sub(elapsed).unwrap_or(now);
        true
    }

    fn drain_d2_events<E>(
        poll: &mut impl FnMut() -> Result<Option<E>, String>,
        record: &mut impl FnMut(E),
    ) -> Result<usize, String> {
        let mut drained = 0;
        for _ in 0..D2_MAX_EVENTS_PER_DRAIN {
            let Some(event) = poll()? else { break };
            record(event);
            drained += 1;
        }
        Ok(drained)
    }

    fn pace_d2_frame_end<E>(
        playback_origin: &mut Instant,
        frame_end: usize,
        poll: &mut impl FnMut() -> Result<Option<E>, String>,
        record: &mut impl FnMut(E),
    ) -> Result<(), String> {
        let deadline = d2_frame_deadline(*playback_origin, frame_end);
        loop {
            let drained = drain_d2_events(poll, record)?;
            let now = Instant::now();
            let remaining = deadline.saturating_duration_since(now);
            if remaining.is_zero() {
                d2_rebase_if_late(playback_origin, frame_end, now);
                return Ok(());
            }
            if drained == D2_MAX_EVENTS_PER_DRAIN {
                continue;
            }
            std::thread::sleep(remaining.min(D2_POLL_INTERVAL));
        }
    }

    fn record_parakeet_poll_event(
        event: ImportEvent,
        applied: &mut Vec<serde_json::Value>,
        poll_events: &mut Vec<String>,
    ) {
        poll_events.push(format!("{event:?}"));
        if let ImportEvent::MicrophoneGainApplied {
            gain_db,
            gain_revision,
            first_frame,
            ..
        } = event
        {
            applied.push(serde_json::json!({"gain_db":gain_db,"revision":gain_revision,"first_frame":first_frame}));
        }
    }

    fn drain_d2_events_after_playback<E>(
        poll: &mut impl FnMut() -> Result<Option<E>, String>,
        record: &mut impl FnMut(E),
    ) -> Result<(), String> {
        for _ in 0..D2_MAX_DRAIN_BATCHES {
            if drain_d2_events(poll, record)? < D2_MAX_EVENTS_PER_DRAIN {
                break;
            }
        }
        Ok(())
    }

    #[test]
    fn d2_corpus_real_cpu_turbo_and_parakeet() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let root =
            PathBuf::from(std::env::var_os("WHISPER_MIC_LEVEL_D2_ROOT").expect("guarded D2 root"));
        fs::create_dir_all(&root).unwrap();
        let checkpoint = root.join("checkpoint.jsonl");
        let engine_filter =
            std::env::var("WHISPER_MIC_LEVEL_D2_ENGINE").expect("guarded D2 engine filter");
        assert_eq!(engine_filter, "parakeet-cpu", "D2 replay is Parakeet-only");
        record(
            &checkpoint,
            serde_json::json!({"event":"campaign-start","engine_filter":engine_filter,"cases":16,"timeout_seconds":1162}),
        );
        assert!(
            std::env::var_os("CUDA_CACHE_PATH").is_none(),
            "CPU D2 must not redirect CUDA cache"
        );
        let local = PathBuf::from(std::env::var_os("LOCALAPPDATA").expect("LOCALAPPDATA"));
        let turbo_worker = local.join("Whisper/app/whisper-worker-cpu.exe");
        let turbo_model = local.join("Whisper/models/ggml-large-v3-turbo.bin");
        assert!(turbo_worker.is_file() && turbo_model.is_file());
        let parakeet = ParakeetWorkerPaths {
            executable: local.join("Whisper/worker/whisper-worker-parakeet-cpu.exe"),
            native_dir: local.join("Whisper/native/cpu"),
            model: local.join("Whisper/models/parakeet-tdt-0.6b-v3.q8_0.gguf"),
            runtime_root: local.join("Whisper"),
        };
        parakeet.validate().expect("personal Parakeet CPU route");
        let engines = [
            ("turbo-cpu", turbo_model, ModelId::WhisperLargeV3Turbo),
            (
                "parakeet-cpu",
                parakeet.model.clone(),
                ModelId::ParakeetTdt06BV3,
            ),
        ];
        for (engine, model_path, model_id) in engines
            .into_iter()
            .filter(|(engine, _, _)| *engine == engine_filter.as_str())
        {
            for (corpus, wav_rel, ref_rel, wav_hash, ref_hash) in INPUTS {
                let input = repo.join(wav_rel);
                let reference_path = repo.join(ref_rel);
                assert_eq!(
                    hash(&input),
                    wav_hash,
                    "approved WAV hash changed: {corpus}"
                );
                assert_eq!(
                    hash(&reference_path),
                    ref_hash,
                    "approved reference hash changed: {corpus}"
                );
                let reference = fs::read_to_string(reference_path).unwrap();
                let (rate, channels, samples) = read_wav16(&input);
                assert_eq!(
                    (rate, channels),
                    (16_000, 1),
                    "approved corpus WAV format changed"
                );
                for (condition, initial, transition) in CONDITIONS {
                    let ordinal = CASE_ID.fetch_add(1, Ordering::Relaxed);
                    let case_root =
                        root.join(format!("{ordinal:02}-{engine}-{corpus}-{condition}"));
                    fs::create_dir_all(&case_root).unwrap();
                    record(
                        &checkpoint,
                        serde_json::json!({"event":"case-start","ordinal":ordinal,"engine":engine,"corpus":corpus,"condition":condition,"input_frames":samples.len()}),
                    );
                    let req = LiveRequest {
                        job_id: JobId(((std::process::id() as u128) << 64) | u128::from(ordinal)),
                        generation: Generation::first(),
                        model_path: model_path.to_string_lossy().into_owned(),
                        model_sha256: decode_sha256(
                            whisper_core::model_registry::model(model_id)
                                .artifact
                                .sha256,
                        ),
                        destination: case_root.to_string_lossy().into_owned(),
                        group_offset_samples: 0,
                        auto_stop_after_speech_samples: None,
                        microphone: whisper_core::MicrophoneChoice::Device(
                            "d2:approved-wav".into(),
                        ),
                        microphone_gain_db: initial,
                        config: JobConfig {
                            language: LanguageChoice::Automatic,
                            compute: ComputeChoice::Cpu,
                        },
                    };
                    let capture = CaptureStream::empty_test_capture_with_gain(
                        rate,
                        channels,
                        "d2:approved-wav",
                        GainDb::new(initial).unwrap(),
                    );
                    let start = Instant::now();
                    let (wav, txt, srt, frames, transcript, gain_frames) = if engine == "turbo-cpu"
                    {
                        turbo_case(req, capture, &turbo_worker, &samples, transition)
                    } else {
                        parakeet_case(req, capture, &parakeet, &samples, transition)
                    };
                    assert!(
                        Path::new(&wav).is_file()
                            && Path::new(&txt).is_file()
                            && Path::new(&srt).is_file()
                    );
                    assert_eq!(
                        frames,
                        samples.len() as u64,
                        "output frame continuity mismatch"
                    );
                    assert_eq!(
                        gain_frames.len(),
                        if transition.is_some() { 2 } else { 1 },
                        "gain application acknowledgements"
                    );
                    assert_eq!(gain_frames[0]["gain_db"], initial);
                    assert_eq!(gain_frames[0]["revision"], 0);
                    assert_eq!(gain_frames[0]["first_frame"], 0);
                    if let Some(gain) = transition {
                        assert_eq!(gain_frames[1]["gain_db"], gain);
                        assert_eq!(gain_frames[1]["revision"], 1);
                        assert_eq!(gain_frames[1]["first_frame"], samples.len() / 2);
                    }
                    record(
                        &checkpoint,
                        serde_json::json!({
                            "event":"case-complete","ordinal":ordinal,"engine":engine,"corpus":corpus,"condition":condition,
                            "input_wav_sha256":wav_hash,"reference_sha256":ref_hash,"output_wav_sha256":hash(Path::new(&wav)),"input_frames":samples.len(),"output_frames":frames,
                            "wav":wav,"txt":txt,"srt":srt,"elapsed_ms":start.elapsed().as_millis(),
                            "approved_reference":reference,"transcript":transcript,"gain_frame_revisions":gain_frames,"comparison":"descriptive only; no WER threshold"
                        }),
                    );
                    eprintln!("D2_CASE_DONE {ordinal}/16 {engine} {corpus} {condition}");
                }
            }
        }
        record(
            &checkpoint,
            serde_json::json!({"event":"campaign-complete","engine_filter":engine_filter,"cases":16}),
        );
    }

    fn turbo_case(
        req: LiveRequest,
        capture: CaptureStream,
        worker: &Path,
        samples: &[i16],
        transition: Option<i8>,
    ) -> (String, String, String, u64, String, Vec<serde_json::Value>) {
        let mut live =
            DirectLive::from_test_capture_with_cpu_worker(req, capture, worker.to_path_buf(), 1)
                .unwrap();
        let mut applied = Vec::new();
        let mut offset = 0;
        let midpoint = samples.len() / 2;
        let mut changed = false;
        while offset < samples.len() {
            if !changed && offset >= midpoint {
                if let Some(gain) = transition {
                    live.set_gain("d2:approved-wav", 0, GainDb::new(gain).unwrap())
                        .unwrap();
                    changed = true;
                }
            }
            let boundary = if !changed && transition.is_some() {
                midpoint
            } else {
                samples.len()
            };
            let end = (offset + 1600).min(boundary).max(offset + 1);
            live.enqueue_test_capture_block(
                samples[offset..end]
                    .iter()
                    .map(|sample| f32::from(*sample) / 32768.0)
                    .collect(),
            );
            if let Some(ImportEvent::MicrophoneGainApplied {
                gain_db,
                gain_revision,
                first_frame,
                ..
            }) = live.poll().unwrap()
            {
                applied.push(serde_json::json!({"gain_db":gain_db,"revision":gain_revision,"first_frame":first_frame}));
            }
            offset = end;
        }
        live.stop().unwrap();
        let deadline = Instant::now() + Duration::from_secs(3500);
        let mut published = None;
        while !live.done() && Instant::now() < deadline {
            if let Some(event) = live.poll().unwrap() {
                match event {
                    ImportEvent::DirectPublished { wav_path, txt_path, srt_path, .. } => published = Some((wav_path,txt_path,srt_path)),
                    ImportEvent::MicrophoneGainApplied { gain_db, gain_revision, first_frame, .. } => applied.push(serde_json::json!({"gain_db":gain_db,"revision":gain_revision,"first_frame":first_frame})),
                    _ => {}
                }
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(live.done(), "Turbo CPU direct capture did not finish");
        let (wav, txt, srt) = published.expect("DirectPublished");
        let frames = pcm_frames(&wav);
        let transcript = fs::read_to_string(&txt).unwrap();
        assert!(!transcript.trim().is_empty(), "Turbo CPU transcript empty");
        (wav, txt, srt, frames, transcript, applied)
    }

    fn parakeet_case(
        req: LiveRequest,
        capture: CaptureStream,
        paths: &ParakeetWorkerPaths,
        samples: &[i16],
        transition: Option<i8>,
    ) -> (String, String, String, u64, String, Vec<serde_json::Value>) {
        let session = CASE_ID.load(Ordering::Relaxed);
        let destination = PathBuf::from(&req.destination);
        let mut live = crate::parakeet_live::ParakeetLive::start_test_capture(
            req,
            paths.clone(),
            session,
            capture,
        )
        .unwrap();
        let mut applied = Vec::new();
        let mut poll_events = Vec::new();
        let mut playback_origin = Instant::now();
        let mut offset = 0;
        let midpoint = samples.len() / 2;
        let mut changed = false;
        while offset < samples.len() {
            if !changed && offset >= midpoint {
                if let Some(gain) = transition {
                    live.set_gain("d2:approved-wav", 0, GainDb::new(gain).unwrap())
                        .unwrap();
                    changed = true;
                }
            }
            let boundary = if !changed && transition.is_some() {
                midpoint
            } else {
                samples.len()
            };
            let end = (offset + 1600).min(boundary).max(offset + 1);
            live.enqueue_d2_capture_block(
                samples[offset..end]
                    .iter()
                    .map(|sample| f32::from(*sample) / 32768.0)
                    .collect(),
            );
            live.pump_capture().unwrap();
            let mut poll = || live.poll();
            let mut record =
                |event| record_parakeet_poll_event(event, &mut applied, &mut poll_events);
            pace_d2_frame_end(&mut playback_origin, end, &mut poll, &mut record).unwrap();
            offset = end;
        }
        {
            let mut poll = || live.poll();
            let mut record =
                |event| record_parakeet_poll_event(event, &mut applied, &mut poll_events);
            drain_d2_events_after_playback(&mut poll, &mut record).unwrap();
        }
        let event = live.stop().unwrap();
        let ImportEvent::ParakeetCaptureStopped {
            wav_path,
            txt_path: Some(txt),
            srt_path: Some(srt),
            final_segments,
            complete,
            diagnostic,
            unresolved,
            ..
        } = event
        else {
            panic!("missing Parakeet finalized outputs: {event:?}")
        };
        record_engine_status(
            &destination,
            serde_json::json!({
                "engine":"parakeet-cpu",
                "complete":complete,
                "diagnostic":diagnostic.clone(),
                "confirmed_segments":final_segments.len(),
                "unresolved":unresolved.iter().map(|range| format!("{range:?}")).collect::<Vec<_>>(),
                "gain_frame_revisions":applied.clone(),
                "poll_events":poll_events,
                "worker_events_path":destination.join("d2-worker-events.jsonl"),
                "wav_path":wav_path.clone(),
                "txt_path":txt.clone(),
                "srt_path":srt.clone(),
            }),
        );
        assert!(
            !final_segments.is_empty(),
            "Parakeet CPU returned no segments"
        );
        let frames = pcm_frames(&wav_path);
        let transcript = fs::read_to_string(&txt).unwrap();
        assert!(
            !transcript.trim().is_empty(),
            "Parakeet CPU transcript empty"
        );
        (wav_path, txt, srt, frames, transcript, applied)
    }

    fn record_engine_status(root: &Path, status: serde_json::Value) {
        record(&root.join("d2-engine-status.json"), status);
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn d2_frame_deadlines_follow_the_monotone_one_x_sample_clock() {
            let origin = Instant::now();
            let after_100ms = d2_frame_deadline(origin, 1_600);
            let after_200ms = d2_frame_deadline(origin, 3_200);
            assert_eq!(
                after_100ms.duration_since(origin),
                Duration::from_millis(100)
            );
            assert_eq!(
                after_200ms.duration_since(origin),
                Duration::from_millis(200)
            );
            assert!(after_200ms > after_100ms);
        }

        #[test]
        fn d2_event_drain_stops_after_its_bounded_batch() {
            let mut pending = (0..D2_MAX_EVENTS_PER_DRAIN + 3).collect::<VecDeque<_>>();
            let mut poll = || Ok(pending.pop_front());
            let mut observed = Vec::new();
            let mut record = |event| observed.push(event);
            assert_eq!(
                drain_d2_events(&mut poll, &mut record).unwrap(),
                D2_MAX_EVENTS_PER_DRAIN
            );
            drop(record);
            drop(poll);
            assert_eq!(observed.len(), D2_MAX_EVENTS_PER_DRAIN);
            assert_eq!(pending.len(), 3);
            let mut poll = || Ok(pending.pop_front());
            let mut record = |event| observed.push(event);
            assert_eq!(drain_d2_events(&mut poll, &mut record).unwrap(), 3);
            assert_eq!(observed.len(), D2_MAX_EVENTS_PER_DRAIN + 3);
        }

        #[test]
        fn d2_driver_paces_frames_and_drains_events_before_deadline() {
            let origin = Instant::now();
            let mut pending = VecDeque::from([1, 2]);
            let mut polls = 0;
            let mut poll = || {
                polls += 1;
                Ok(pending.pop_front())
            };
            let mut observed = Vec::new();
            let mut record = |event| observed.push(event);
            let mut playback_origin = origin;
            pace_d2_frame_end(&mut playback_origin, 1_600, &mut poll, &mut record).unwrap();

            assert!(Instant::now().duration_since(origin) >= Duration::from_millis(100));
            assert_eq!(observed, vec![1, 2]);
            assert!(polls > 2, "driver must poll throughout the frame interval");
        }

        #[test]
        fn d2_expired_frame_deadline_rebases_without_catch_up_burst() {
            let now = Instant::now();
            let mut playback_origin = now - Duration::from_secs(8);
            assert!(d2_rebase_if_late(&mut playback_origin, 1_600, now));

            assert_eq!(d2_frame_deadline(playback_origin, 1_600), now);
            assert_eq!(
                d2_frame_deadline(playback_origin, 3_200),
                now + Duration::from_millis(100)
            );
            assert!(!d2_rebase_if_late(&mut playback_origin, 3_200, now));
        }

        #[test]
        fn d2_post_playback_drain_has_a_bounded_batch_count() {
            let mut pending =
                (0..D2_MAX_EVENTS_PER_DRAIN * (D2_MAX_DRAIN_BATCHES + 2)).collect::<VecDeque<_>>();
            let mut observed = Vec::new();
            let mut poll = || Ok(pending.pop_front());
            let mut record = |event| observed.push(event);
            drain_d2_events_after_playback(&mut poll, &mut record).unwrap();

            assert_eq!(
                observed.len(),
                D2_MAX_EVENTS_PER_DRAIN * D2_MAX_DRAIN_BATCHES
            );
            assert_eq!(pending.len(), D2_MAX_EVENTS_PER_DRAIN * 2);
        }

        #[test]
        fn test_status_writer_preserves_diagnostics() {
            let root = PathBuf::from(
                std::env::var_os("WHISPER_MIC_LEVEL_D2_ROOT").expect("guarded D2 root"),
            );
            fs::create_dir_all(&root).unwrap();
            let case_root = root.join(format!("status-selftest-{}", std::process::id()));
            fs::create_dir_all(&case_root).unwrap();
            let status = serde_json::json!({
                "engine":"parakeet-cpu",
                "complete":false,
                "confirmed_segments":0,
                "diagnostic":"synthetic worker failure",
                "worker_events_path":"synthetic-events.jsonl",
            });
            record_engine_status(&case_root, status.clone());
            let saved: serde_json::Value =
                serde_json::from_slice(&fs::read(case_root.join("d2-engine-status.json")).unwrap())
                    .unwrap();
            assert_eq!(saved, status);
        }
    }

    fn read_wav16(path: &Path) -> (u32, u16, Vec<i16>) {
        let bytes = fs::read(path).unwrap();
        assert_eq!(&bytes[..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        let (mut rate, mut channels, mut format, mut bits, mut pcm) =
            (None, None, None, None, None);
        let mut at = 12;
        while at + 8 <= bytes.len() {
            let size = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().unwrap()) as usize;
            let begin = at + 8;
            let end = begin.checked_add(size).unwrap();
            assert!(end <= bytes.len());
            match &bytes[at..at + 4] {
                b"fmt " => {
                    format = Some(u16::from_le_bytes(
                        bytes[begin..begin + 2].try_into().unwrap(),
                    ));
                    channels = Some(u16::from_le_bytes(
                        bytes[begin + 2..begin + 4].try_into().unwrap(),
                    ));
                    rate = Some(u32::from_le_bytes(
                        bytes[begin + 4..begin + 8].try_into().unwrap(),
                    ));
                    bits = Some(u16::from_le_bytes(
                        bytes[begin + 14..begin + 16].try_into().unwrap(),
                    ));
                }
                b"data" => pcm = Some(&bytes[begin..end]),
                _ => {}
            }
            at = end + (size % 2);
        }
        assert_eq!(format, Some(1));
        assert_eq!(bits, Some(16));
        let samples = pcm
            .unwrap()
            .chunks_exact(2)
            .map(|p| i16::from_le_bytes([p[0], p[1]]))
            .collect();
        (rate.unwrap(), channels.unwrap(), samples)
    }
    fn pcm_frames(path: &str) -> u64 {
        let bytes = fs::read(path).unwrap();
        assert_eq!(&bytes[..4], b"RIFF");
        (bytes.len() - 44) as u64 / 2
    }
    fn hash(path: &Path) -> String {
        format!("{:x}", Sha256::digest(fs::read(path).unwrap()))
    }
    fn decode_sha256(value: &str) -> [u8; 32] {
        assert_eq!(value.len(), 64);
        std::array::from_fn(|index| {
            u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).unwrap()
        })
    }
    fn record(path: &Path, value: serde_json::Value) {
        let mut f = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap();
        serde_json::to_writer(&mut f, &value).unwrap();
        f.write_all(b"\n").unwrap();
        f.sync_data().unwrap();
    }
}
