use std::{
    collections::VecDeque,
    fs,
    io::{Seek, SeekFrom, Write},
    path::PathBuf,
};

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

const MIN_WINDOW_SAMPLES: usize = 80_000;
const MAX_WINDOW_SAMPLES: usize = 128_000;
const PAUSE_SAMPLES: usize = 3_200;
const PAUSE_STEP_SAMPLES: usize = 1_600;
const PAUSE_RMS_LIMIT: u64 = 400;
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
    wav: Option<WavOutput>,
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
            wav: None,
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
                let tail = converter.finish()?;
                self.wav.as_mut().expect("wav ready").append(&tail)?;
                self.pending.extend(tail);
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
            self.wav.as_mut().expect("wav ready").append(&samples)?;
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
                self.wav = Some(WavOutput::create(&self.request)?);
                if !self.stop_requested {
                    let capture = crate::capture::start_direct_default().map_err(|error| {
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
                let wav_path = self.wav.take().expect("wav ready").finish()?;
                let (txt_path, srt_path) = self.export()?;
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

#[cfg(test)]
mod tests {
    use super::{MAX_WINDOW_SAMPLES, MIN_WINDOW_SAMPLES, WavOutput, window_size, write_wav_header};
    use std::collections::VecDeque;
    use whisper_core::{ComputeChoice, JobConfig, LanguageChoice};

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
        let directory =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/direct-live-tests");
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
            config: JobConfig {
                language: LanguageChoice::Automatic,
                compute: ComputeChoice::Cpu,
            },
        };
        let mut wav = WavOutput::create(&request).unwrap();
        wav.append(&[-32768, 0]).unwrap();
        wav.append(&[32767]).unwrap();
        let path = wav.finish().unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(bytes.len(), 50);
        assert_eq!(u32::from_le_bytes(bytes[40..44].try_into().unwrap()), 6);
        assert_eq!(&bytes[44..], &[0, 128, 0, 0, 255, 127]);
        std::fs::remove_file(path).unwrap();
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
