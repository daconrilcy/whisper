use super::*;

impl DirectLive {
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

    pub(super) fn take_probe_spectrum(&mut self) -> Option<ImportEvent> {
        self.spectrum_reader
            .poll(self.probe.as_ref()?, std::time::Instant::now())
            .map(ImportEvent::MicrophoneSpectrum)
    }

    pub(super) fn drain_capture(&mut self) -> Result<(), String> {
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
}
