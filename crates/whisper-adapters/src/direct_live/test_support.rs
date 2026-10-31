use super::*;

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
}
