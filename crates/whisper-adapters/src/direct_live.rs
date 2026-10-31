mod capture;
mod outputs;
mod wav;
mod worker;

#[cfg(test)]
mod d2_corpus;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;

use outputs::finalize_incomplete_outputs;
pub(crate) use outputs::{
    ParakeetExportOutcome, automatic_final_output_paths, export_parakeet_manifest,
    export_parakeet_outputs, export_segments,
};
pub(crate) use wav::WavOutput;
use wav::fill_pending_from_wav;
#[cfg(test)]
use wav::write_wav_header;
#[cfg(test)]
use worker::{
    clear_gpu_proof_for_cpu, gpu_ready_requires_attestation, should_retry_auto_gpu_before_ready,
};
use worker::{compute_name, spawn_direct_worker};

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

impl Drop for DirectLive {
    fn drop(&mut self) {
        if let Some(worker) = self.worker.as_mut() {
            worker.abort();
        }
    }
}
