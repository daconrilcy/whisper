use std::{collections::VecDeque, fs, path::PathBuf};

use whisper_core::{
    Generation, ImportEvent, JobId, LiveRequest, SegmentId, SourceRange,
    ipc::{BackendKind, WorkerCommand, WorkerErrorCode, WorkerEvent},
    ports::WorkerSegment,
};

use crate::{
    WorkerTransport,
    capture::{CaptureStream, Pcm16Converter},
    supervisor::WorkerPaths,
    worker_ipc::ChildWorkerTransport,
};

const WINDOW_SAMPLES: usize = 32_000;
const MAX_BUFFERED_SAMPLES: usize = 16_000 * 60 * 10;

pub(crate) struct DirectLive {
    request: LiveRequest,
    worker_path: PathBuf,
    worker: ChildWorkerTransport,
    instance: u64,
    worker_ready: bool,
    legacy_fallback: bool,
    capture: Option<CaptureStream>,
    converter: Option<Pcm16Converter>,
    pending: VecDeque<i16>,
    next_offset: u64,
    next_window: u64,
    inflight: Option<(u64, SourceRange)>,
    next_segment: u64,
    segments: Vec<WorkerSegment>,
    window_segments: Vec<WorkerSegment>,
    ready_events: VecDeque<ImportEvent>,
    stop_requested: bool,
    stop_sent: bool,
    done: bool,
}

impl DirectLive {
    pub(crate) fn start(
        request: LiveRequest,
        workers: &WorkerPaths,
        instance: u64,
    ) -> Result<Self, String> {
        if request.config.compute != whisper_core::ComputeChoice::Cpu
            || request.destination.trim().is_empty()
            || request.model_path.trim().is_empty()
        {
            return Err("Micro : modèle ou dossier de sortie invalide".into());
        }
        let mut worker = ChildWorkerTransport::spawn(&workers.cpu)
            .map_err(|error| format!("Moteur CPU indisponible : {error:?}"))?;
        if let Err(error) = worker.request(WorkerCommand::StartDirectLiveWorker {
            job_id: request.job_id,
            generation: request.generation,
            instance_id: instance,
            model_path: request.model_path.clone(),
            model_sha256: request.model_sha256,
            config: request.config.clone(),
        }) {
            worker.abort();
            return Err(format!("Démarrage du moteur impossible : {error:?}"));
        }
        Ok(Self {
            request,
            worker_path: workers.cpu.clone(),
            worker,
            instance,
            worker_ready: false,
            legacy_fallback: false,
            capture: None,
            converter: None,
            pending: VecDeque::new(),
            next_offset: 0,
            next_window: 1,
            inflight: None,
            next_segment: 1,
            segments: Vec::new(),
            window_segments: Vec::new(),
            ready_events: VecDeque::new(),
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

    pub(crate) fn stop(&mut self) -> Result<(), String> {
        if self.stop_requested {
            return Ok(());
        }
        self.stop_requested = true;
        if let Some(capture) = self.capture.as_mut() {
            capture.stop();
            self.drain_capture()?;
            if let Some(converter) = self.converter.as_mut() {
                self.pending.extend(converter.finish()?);
                if self.pending.len() > MAX_BUFFERED_SAMPLES {
                    return Err(
                        "Transcription CPU en retard : tampon audio saturé ; session incomplète"
                            .into(),
                    );
                }
            }
        }
        Ok(())
    }

    pub(crate) fn poll(&mut self) -> Result<Option<ImportEvent>, String> {
        if self.done {
            return Ok(None);
        }
        self.pump_capture()?;
        if let Some(event) = self.ready_events.pop_front() {
            return Ok(Some(event));
        }
        if let Some(event) = self
            .worker
            .receive()
            .map_err(|error| format!("Moteur CPU interrompu : {error:?}"))?
        {
            if let Some(result) = self.handle_worker(event)? {
                return Ok(Some(result));
            }
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
            return Err(format!(
                "Micro interrompu par Windows : {detail} ; transcription incomplète"
            ));
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
            self.pending.extend(samples);
            if self.pending.len() > MAX_BUFFERED_SAMPLES {
                capture.stop();
                return Err(
                    "Transcription CPU en retard : tampon audio saturé ; session incomplète".into(),
                );
            }
        }
        Ok(())
    }

    fn send_next(&mut self) -> Result<(), String> {
        if !self.worker_ready || self.inflight.is_some() || self.stop_sent {
            return Ok(());
        }
        let count = if self.pending.len() >= WINDOW_SAMPLES {
            WINDOW_SAMPLES
        } else if self.stop_requested {
            self.pending.len()
        } else {
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
                .request(WorkerCommand::LiveWindow {
                    job_id: self.request.job_id,
                    generation: self.request.generation,
                    instance_id: self.instance,
                    sequence: self.next_window,
                    range,
                    samples,
                })
                .map_err(|error| format!("Fenêtre CPU refusée : {error:?}"))?;
            self.inflight = Some((self.next_window, range));
            self.next_window += 1;
            self.next_offset = end;
        } else if self.stop_requested {
            self.worker
                .request(WorkerCommand::Stop {
                    job_id: self.request.job_id,
                    generation: self.request.generation,
                })
                .map_err(|error| format!("Arrêt CPU refusé : {error:?}"))?;
            self.stop_sent = true;
        }
        Ok(())
    }

    fn handle_worker(&mut self, event: WorkerEvent) -> Result<Option<ImportEvent>, String> {
        match event {
            WorkerEvent::Failed { message, .. }
                if !self.worker_ready
                    && !self.legacy_fallback
                    && message.contains("unknown variant `StartDirectLiveWorker`") =>
            {
                self.worker.abort();
                self.worker = ChildWorkerTransport::spawn(&self.worker_path)
                    .map_err(|error| format!("Moteur CPU indisponible : {error:?}"))?;
                self.legacy_fallback = true;
                self.worker
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
            } if self.matches(job_id, generation, instance_id) && backend == BackendKind::Cpu => {
                self.worker_ready = true;
                if !self.stop_requested {
                    let capture = crate::capture::start_default().map_err(|error| {
                        format!("Micro Windows indisponible. Vérifiez son branchement et son autorisation : {error}")
                    })?;
                    let converter =
                        Pcm16Converter::new(capture.sample_rate_hz(), capture.channels())?;
                    self.capture = Some(capture);
                    self.converter = Some(converter);
                }
                Ok(Some(ImportEvent::Ready {
                    job_id,
                    generation,
                    backend: "CPU (micro)".into(),
                }))
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
                    return Err("Moteur CPU : segment hors ordre".into());
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
                    return Err("Moteur CPU : fin de fenêtre incohérente".into());
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
                let (txt_path, srt_path) = self.export()?;
                self.done = true;
                Ok(Some(ImportEvent::DirectPublished {
                    job_id,
                    generation,
                    txt_path,
                    srt_path,
                }))
            }
            WorkerEvent::Failed {
                code: WorkerErrorCode::ModelUnavailable | WorkerErrorCode::ModelHashMismatch,
                message,
                ..
            } => Err(format!(
                "Modèle local indisponible. Vérifiez le chemin du modèle du MVP : {message}"
            )),
            WorkerEvent::Failed { message, .. } => Err(format!(
                "Moteur CPU interrompu. Réessayez après vérification : {message}"
            )),
            _ => Err("Moteur CPU : réponse inattendue ou périmée".into()),
        }
    }

    fn matches(&self, job_id: JobId, generation: Generation, instance: u64) -> bool {
        (job_id, generation) == self.identity() && instance == self.instance
    }

    fn export(&self) -> Result<(String, String), String> {
        let directory = PathBuf::from(&self.request.destination);
        fs::create_dir_all(&directory)
            .map_err(|error| format!("Dossier de sortie inaccessible : {error}"))?;
        let stem = format!("micro-{:032x}", self.request.job_id.0);
        let txt = directory.join(format!("{stem}.txt"));
        let srt = directory.join(format!("{stem}.srt"));
        let text = self
            .segments
            .iter()
            .map(|segment| segment.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let subtitles = self
            .segments
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
        fs::write(&txt, text).map_err(|error| {
            format!("TXT non enregistré. Vérifiez le dossier de sortie : {error}")
        })?;
        fs::write(&srt, subtitles).map_err(|error| {
            format!("SRT non enregistré. Vérifiez le dossier de sortie : {error}")
        })?;
        Ok((
            txt.to_string_lossy().into_owned(),
            srt.to_string_lossy().into_owned(),
        ))
    }
}

impl Drop for DirectLive {
    fn drop(&mut self) {
        self.worker.abort();
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
