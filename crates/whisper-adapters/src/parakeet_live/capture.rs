use super::*;

/// Capture-to-source bridge used by the unified path. It drains the existing
/// nonblocking device ring and sends converted PCM to an independent bounded
/// writer; inference never owns or blocks this writer thread.
pub(crate) struct UnifiedCapturePump {
    capture: Option<CaptureStream>,
    converter: Option<Pcm16Converter>,
    writer: Option<CapturePcmWriter>,
    source_identity: Option<String>,
    last_acknowledged_gain_revision: Option<u64>,
    gain_events: std::collections::VecDeque<AppliedCaptureGain>,
    acknowledgements_received: u64,
    closed: Option<Result<(u64, Vec<CaptureWriteAck>), String>>,
}

pub(crate) struct AppliedCaptureGain {
    pub(crate) target_identity: String,
    pub(crate) gain_db: i8,
    pub(crate) gain_revision: u64,
    pub(crate) first_frame: u64,
}

impl UnifiedCapturePump {
    pub(crate) fn from_capture(
        capture: CaptureStream,
        source_path: PathBuf,
    ) -> Result<Self, String> {
        Self::from_capture_inner(capture, source_path, None)
    }

    #[cfg(test)]
    pub(crate) fn from_capture_with_test_part_limit(
        capture: CaptureStream,
        source_path: PathBuf,
        part_limit_bytes: u64,
    ) -> Result<Self, String> {
        Self::from_capture_inner(capture, source_path, Some(part_limit_bytes))
    }

    fn from_capture_inner(
        mut capture: CaptureStream,
        source_path: PathBuf,
        test_part_limit: Option<u64>,
    ) -> Result<Self, String> {
        let converter = match Pcm16Converter::new(capture.sample_rate_hz(), capture.channels()) {
            Ok(converter) => converter,
            Err(error) => {
                capture.stop();
                return Err(error);
            }
        };
        let source_identity = capture.source_identity().map(str::to_owned);
        let writer = if let Some(part_limit) = test_part_limit {
            #[cfg(test)]
            {
                CapturePcmWriter::start_with_test_part_limit(
                    source_path,
                    crate::capture::CAPTURE_SLOTS,
                    part_limit,
                )
            }
            #[cfg(not(test))]
            {
                let _ = part_limit;
                Err(crate::parakeet_audio::AudioStoreError::InvalidRange)
            }
        } else {
            CapturePcmWriter::start(source_path, crate::capture::CAPTURE_SLOTS)
        };
        let writer = match writer {
            Ok(writer) => writer,
            Err(error) => {
                capture.stop();
                return Err(format!("StorageUnavailable: {error}"));
            }
        };
        Ok(Self {
            capture: Some(capture),
            converter: Some(converter),
            writer: Some(writer),
            source_identity,
            last_acknowledged_gain_revision: None,
            gain_events: std::collections::VecDeque::new(),
            acknowledgements_received: 0,
            closed: None,
        })
    }

    pub(crate) fn pump(&mut self) -> Result<Vec<CaptureWriteAck>, String> {
        if self.closed.is_some() {
            return Err("Capture Parakeet arrêtée".into());
        }
        let capture = self.capture.as_mut().ok_or("Capture Parakeet arrêtée")?;
        if capture.is_saturated() {
            return Err("CaptureLost: microphone ring saturated".into());
        }
        if capture.failure() {
            return Err("CaptureLost: microphone stream failed".into());
        }
        let mut gain_updates = Vec::new();
        for _ in 0..MAX_CAPTURE_BLOCKS_PER_PUMP {
            let Some(block) = capture.try_next_block() else {
                break;
            };
            let block_gain = block.gain();
            let block_first_frame = block.first_frame();
            let samples = self
                .converter
                .as_mut()
                .ok_or("Capture converter already drained")?
                .push(&block)?;
            if !samples.is_empty() {
                self.writer
                    .as_mut()
                    .ok_or("Capture writer already closed")?
                    .try_append(&samples)
                    .map_err(|error| format!("CaptureLost: audio writer: {error:?}"))?;
            }
            gain_updates.push((block_gain, block_first_frame));
        }
        for (gain, first_frame) in gain_updates {
            self.record_gain_acknowledgement(gain, first_frame);
        }
        self.collect_available_acknowledgements()
    }

    pub(crate) fn set_gain(
        &self,
        target_identity: &str,
        expected_revision: u64,
        gain: whisper_core::microphone_level::GainDb,
    ) -> Result<whisper_core::microphone_level::GainRevision, String> {
        let capture = self.capture.as_ref().ok_or("Capture Parakeet arrêtée")?;
        if capture.source_identity() != Some(target_identity) {
            return Err("MicrophoneLevel: source active différente de la cible".into());
        }
        capture.set_gain(expected_revision, gain)
    }

    #[cfg(test)]
    pub(crate) fn enqueue_test_block(&mut self, samples: Vec<f32>) {
        self.capture
            .as_mut()
            .expect("test capture remains open")
            .enqueue_test_block(samples);
    }

    pub(crate) fn take_gain_events(&mut self) -> Vec<AppliedCaptureGain> {
        self.gain_events.drain(..).collect()
    }

    fn record_gain_acknowledgement(
        &mut self,
        gain: whisper_core::microphone_level::GainRevision,
        first_frame: u64,
    ) {
        if self
            .last_acknowledged_gain_revision
            .is_none_or(|revision| gain.revision > revision)
        {
            self.last_acknowledged_gain_revision = Some(gain.revision);
            if let Some(target_identity) = self.source_identity.clone() {
                self.gain_events.push_back(AppliedCaptureGain {
                    target_identity,
                    gain_db: gain.gain.value(),
                    gain_revision: gain.revision,
                    first_frame,
                });
            }
        }
    }

    pub(crate) fn stop(&mut self) -> Result<(u64, Vec<CaptureWriteAck>), String> {
        if let Some(result) = &self.closed {
            return result.clone();
        }
        let result = self.stop_inner();
        self.closed = Some(result.clone());
        result
    }

    fn stop_inner(&mut self) -> Result<(u64, Vec<CaptureWriteAck>), String> {
        if let Some(mut capture) = self.capture.take() {
            if capture.is_saturated() || capture.failure() {
                return Err("CaptureLost: microphone ring overflow or stream failure".into());
            }
            capture.stop();
            while let Some(block) = capture.try_next_block() {
                let block_gain = block.gain();
                let block_first_frame = block.first_frame();
                let samples = self
                    .converter
                    .as_mut()
                    .ok_or("Capture converter already drained")?
                    .push(&block)?;
                if !samples.is_empty() {
                    self.append_during_stop(&samples)?;
                }
                self.record_gain_acknowledgement(block_gain, block_first_frame);
            }
        }
        let tail = self
            .converter
            .take()
            .ok_or("Capture converter already drained")?
            .finish()?;
        if !tail.is_empty() {
            self.append_during_stop(&tail)?;
        }
        let expected_acknowledgements = self
            .writer
            .as_ref()
            .ok_or("Capture writer already closed")?
            .submitted_blocks();
        let acknowledgements = self.collect_acknowledgements_until(expected_acknowledgements)?;
        let total = self
            .writer
            .as_mut()
            .ok_or("Capture writer already closed")?
            .close()
            .map_err(|error| format!("StorageUnavailable: final source close: {error:?}"))?;
        Ok((total, acknowledgements))
    }

    fn collect_available_acknowledgements(&mut self) -> Result<Vec<CaptureWriteAck>, String> {
        let mut acknowledgements = Vec::new();
        while let Some(ack) = self
            .writer
            .as_ref()
            .ok_or("Capture writer already closed")?
            .try_acknowledgement()
            .map_err(|error| format!("StorageUnavailable: writer acknowledgement: {error:?}"))?
        {
            self.acknowledgements_received = self.acknowledgements_received.saturating_add(1);
            acknowledgements.push(ack);
        }
        Ok(acknowledgements)
    }

    fn collect_acknowledgements_until(
        &mut self,
        expected: u64,
    ) -> Result<Vec<CaptureWriteAck>, String> {
        let mut acknowledgements = Vec::new();
        while self.acknowledgements_received < expected {
            if let Some(ack) = self
                .writer
                .as_ref()
                .ok_or("Capture writer already closed")?
                .try_acknowledgement()
                .map_err(|error| format!("StorageUnavailable: writer acknowledgement: {error:?}"))?
            {
                self.acknowledgements_received = self.acknowledgements_received.saturating_add(1);
                acknowledgements.push(ack);
            } else {
                thread::yield_now();
            }
        }
        Ok(acknowledgements)
    }

    fn append_during_stop(&mut self, samples: &[i16]) -> Result<(), String> {
        loop {
            match self
                .writer
                .as_mut()
                .ok_or("Capture writer already closed")?
                .try_append(samples)
            {
                Ok(_) => return Ok(()),
                Err(crate::parakeet_audio::CaptureWriterError::Saturated) => {
                    self.collect_available_acknowledgements()?;
                    thread::yield_now();
                }
                Err(error) => return Err(format!("CaptureLost: final audio write: {error:?}")),
            }
        }
    }
}

impl crate::parakeet_pipeline::ReadablePcmSource for UnifiedCapturePump {
    fn read_window(&mut self, range: std::ops::Range<u64>) -> Result<Vec<i16>, String> {
        self.writer
            .as_ref()
            .ok_or("Capture writer already closed")?
            .read_written_window(range)
            .map_err(|error| format!("AudioSource: {error:?}"))
    }
}

impl ParakeetLive {
    pub(crate) fn pump_capture(&mut self) -> Result<(), String> {
        if self.stopped {
            return Ok(());
        }
        let capture = self.capture.as_mut().expect("active capture");
        if capture.is_saturated() {
            return Err("Tampon microphone saturé ; capture incomplète".into());
        }
        if capture.failure() {
            return Err(capture
                .failure_detail()
                .unwrap_or_else(|| "Capture microphone interrompue".into()));
        }
        for _ in 0..MAX_CAPTURE_BLOCKS_PER_PUMP {
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
                    self.pending_events
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
                .expect("active converter")
                .push_with_probe(&block)?;
            capture.recycle_block(block)?;
            self.probe.submit(converted.probe);
            self.wav
                .as_mut()
                .expect("active WAV")
                .append(&converted.samples)?;
        }
        let samples = self.wav.as_ref().expect("active WAV").sample_count();
        if self.capture_origin_unix_ms.is_none() {
            self.capture_origin_unix_ms = capture.first_sample_at_unix_ms();
        }
        if samples > self.last_reported_samples {
            self.last_reported_samples = samples;
            self.queue_progress(ImportEvent::ParakeetCaptureProgress {
                job_id: self.request.job_id,
                generation: self.request.generation,
                session_id: self.session_id,
                captured_samples: samples,
                audio_durable_samples: samples,
            });
        }
        self.schedule_window(false)
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

    pub(super) fn take_next_probe_telemetry(&mut self) -> Option<ImportEvent> {
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

    pub(super) fn take_probe_meter(&mut self) -> Option<ImportEvent> {
        let snapshot = self.probe.latest_at(Instant::now())?;
        if snapshot.sequence <= self.last_meter_sequence {
            return None;
        }
        let meter = snapshot.meter()?;
        self.last_meter_sequence = snapshot.sequence;
        Some(ImportEvent::MicrophoneMeter(meter))
    }

    pub(super) fn take_probe_spectrum(&mut self) -> Option<ImportEvent> {
        self.spectrum_reader
            .poll(&self.probe, Instant::now())
            .map(ImportEvent::MicrophoneSpectrum)
    }
}
