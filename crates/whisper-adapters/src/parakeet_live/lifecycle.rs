use super::*;

impl ParakeetLive {
    pub(crate) fn start(
        request: LiveRequest,
        paths: ParakeetWorkerPaths,
        expected_gpu: Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
        session_id: u64,
    ) -> Result<Self, String> {
        Self::start_with_capture(request, paths, expected_gpu, session_id, None)
    }

    #[cfg(test)]
    pub(crate) fn start_test_capture(
        request: LiveRequest,
        paths: ParakeetWorkerPaths,
        session_id: u64,
        capture: CaptureStream,
    ) -> Result<Self, String> {
        Self::start_with_capture(request, paths, None, session_id, Some(capture))
    }

    pub(super) fn start_with_capture(
        request: LiveRequest,
        paths: ParakeetWorkerPaths,
        expected_gpu: Option<whisper_core::gpu_inventory::GpuRuntimeIdentity>,
        session_id: u64,
        test_capture: Option<CaptureStream>,
    ) -> Result<Self, String> {
        #[cfg(test)]
        let d2_worker_diagnostics = if test_capture.is_some() {
            D2WorkerDiagnostics::for_d2_destination(Path::new(&request.destination))?
        } else {
            D2WorkerDiagnostics::default()
        };
        #[cfg(not(test))]
        let d2_worker_diagnostics = D2WorkerDiagnostics::default();
        if request.destination.trim().is_empty() || request.model_path.trim().is_empty() {
            return Err("Micro Parakeet : modèle ou dossier de sortie invalide".into());
        }
        paths.validate()?;
        if request.config.compute == ComputeChoice::Gpu && expected_gpu.is_none() {
            return Err("BackendMismatch: Parakeet GPU sans identité courante".into());
        }

        // Open the microphone and durable WAV before starting the potentially slow worker.
        let initial_gain = whisper_core::microphone_level::GainDb::new(request.microphone_gain_db)
            .map_err(|error| format!("MicrophoneLevel: {error}"))?;
        let mut capture = match test_capture {
            Some(capture) => capture,
            None => crate::capture::start_direct_with_gain(&request.microphone, initial_gain)?,
        };
        let converter = match Pcm16Converter::new(capture.sample_rate_hz(), capture.channels()) {
            Ok(converter) => converter,
            Err(error) => {
                capture.stop();
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
                return Err(error);
            }
        };
        let wav = match WavOutput::create(&request) {
            Ok(wav) => wav,
            Err(error) => {
                capture.stop();
                return Err(error);
            }
        };
        let window_root = PathBuf::from(&request.destination).join(format!(
            ".parakeet-live-{:032x}-{}-{}",
            request.job_id.0,
            request.generation.get(),
            session_id
        ));
        fs::create_dir(&window_root)
            .map_err(|error| format!("Fenêtres Parakeet non créées : {error}"))?;
        let trace = TraceSink::from_environment();
        let trace_enabled = trace.enabled();

        let (inference_request, request_rx) = mpsc::sync_channel(1);
        let (event_tx, inference_events) = mpsc::sync_channel(2);
        let cancel_inference = Arc::new(AtomicBool::new(false));
        let worker_cancel = Arc::clone(&cancel_inference);
        let worker_paths = paths.clone();
        let worker_request = request.clone();
        let worker_log_root = window_root.clone();
        let worker_diagnostics = d2_worker_diagnostics.clone();
        let backend = match request.config.compute {
            ComputeChoice::Cpu => "CPU".to_owned(),
            ComputeChoice::Gpu => "GPU".to_owned(),
            ComputeChoice::Auto => "Auto".to_owned(),
        };
        let worker_gpu = expected_gpu.clone();
        let initial_event = ImportEvent::ParakeetLiveStarted {
            job_id: request.job_id,
            generation: request.generation,
            session_id,
            backend,
        };
        thread::Builder::new()
            .name(format!("parakeet-live-{session_id}"))
            .spawn(move || {
                inference_thread(
                    InferenceThreadContext {
                        paths: worker_paths,
                        request: worker_request,
                        candidate: worker_gpu,
                        session_log_root: worker_log_root,
                    },
                    worker_cancel,
                    request_rx,
                    event_tx,
                    trace_enabled,
                    worker_diagnostics,
                )
            })
            .map_err(|error| format!("Worker Parakeet non démarré : {error}"))?;

        Ok(Self {
            request,
            gpu_candidate: expected_gpu,
            session_id,
            inference_request,
            inference_events,
            cancel_inference,
            capture: Some(capture),
            probe,
            last_meter_sequence: 0,
            spectrum_reader: crate::microphone_probe::MicrophoneSpectrumReader::default(),
            next_spectrum_telemetry: false,
            telemetry_due_after_event: false,
            last_acknowledged_gain_revision: None,
            converter: Some(converter),
            wav: Some(wav),
            window_root,
            next_window_start: 0,
            next_sequence: 1,
            last_reported_samples: 0,
            capture_origin_unix_ms: None,
            inflight: false,
            stopped: false,
            unavailable: false,
            ready_sent: false,
            finalizing: false,
            pending_events: std::collections::VecDeque::from([initial_event]),
            confirmation: Confirmation::default(),
            recovery_attempts: RecoveryAttemptLedger::default(),
            trace,
            #[cfg(test)]
            d2_worker_diagnostics,
        })
    }

    pub(crate) fn identity(&self) -> (JobId, Generation) {
        (self.request.job_id, self.request.generation)
    }

    #[cfg(test)]
    pub(crate) fn enqueue_d2_capture_block(&mut self, samples: Vec<f32>) {
        self.capture
            .as_mut()
            .expect("D2 capture remains active")
            .enqueue_test_block(samples);
    }

    pub(crate) fn automatic_final_import(
        &self,
        wav_path: &str,
        job_id: JobId,
    ) -> Result<whisper_core::ports::AutomaticFinalImport, String> {
        let outputs = crate::direct_live::automatic_final_output_paths(wav_path)?;
        if Path::new(&outputs.txt_path).exists() || Path::new(&outputs.srt_path).exists() {
            return Err("Les fichiers de transcription automatique existent déjà".into());
        }
        Ok(whisper_core::ports::AutomaticFinalImport {
            request: whisper_core::ports::ImportRequest {
                strategy: whisper_core::parakeet_session::TranscriptionStrategy::LegacyV1,
                job_id,
                generation: self.request.generation,
                source_path: wav_path.to_owned(),
                source_sha256: None,
                source_samples: None,
                model_path: self.request.model_path.clone(),
                model_sha256: self.request.model_sha256,
                destination: self.request.destination.clone(),
                config: self.request.config.clone(),
            },
            outputs,
            gpu_candidate: self.gpu_candidate.clone(),
        })
    }

    pub(crate) fn stop(&mut self) -> Result<ImportEvent, String> {
        if self.stopped {
            return Err("Session Parakeet déjà arrêtée".into());
        }
        self.finalizing = true;
        self.probe.invalidate();
        let mut failures = Vec::new();
        if let Some(mut capture) = self.capture.take() {
            capture.stop();
            if let Err(error) = self.drain_stopped_capture(&mut capture) {
                failures.push(format!("Drain microphone incomplet : {error}"));
            }
        }
        if let Some(mut converter) = self.converter.take() {
            match converter.finish() {
                Ok(tail) => {
                    if let Some(wav) = self.wav.as_mut()
                        && let Err(error) = wav.append(&tail)
                    {
                        failures.push(format!("Dernier bloc audio non écrit : {error}"));
                    }
                }
                Err(error) => failures.push(format!("Conversion audio incomplète : {error}")),
            }
        }
        let durable_end = self.wav.as_ref().map_or(0, WavOutput::sample_count);
        let deadline = Instant::now() + FINALIZE_TIMEOUT;
        while !self.unavailable && Instant::now() < deadline {
            if let Err(error) = self.schedule_window(true) {
                failures.push(format!("Dernière fenêtre non préparée : {error}"));
                break;
            }
            if let Err(error) = self.poll() {
                failures.push(format!("Finalisation Parakeet interrompue : {error}"));
                break;
            }
            let remaining = durable_end.saturating_sub(self.next_window_start);
            if self.ready_sent && !self.inflight && remaining < SAMPLE_RATE {
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        if !self.unavailable && Instant::now() >= deadline {
            failures.push("Délai de finalisation Parakeet dépassé".into());
        }
        if failures.is_empty() && !self.unavailable {
            // One bounded reread per unresolved interval, at most four intervals.
            // The source is the already durable WAV, never an inferred text edit.
            for range in self.confirmation.recovery_ranges() {
                if Instant::now() >= deadline {
                    break;
                }
                if self.recovery_attempts.len() >= MAX_RECOVERY_ATTEMPTS {
                    break;
                }
                // Live and finalization rereads share one ledger. Overlapping
                // targets represent the same unresolved audio portion and may
                // not consume a second attempt.
                if self.recovery_attempts.contains_overlap(range) {
                    continue;
                }
                let local_start = range
                    .start_sample
                    .saturating_sub(self.request.group_offset_samples);
                let start = local_start.saturating_sub(SAMPLE_RATE);
                let count = durable_end.saturating_sub(start).min(WINDOW_SAMPLES);
                if count < SAMPLE_RATE {
                    continue;
                }
                match self.submit_window(start, count, false, Some(range)) {
                    Ok(true) => self.recovery_attempts.record(range),
                    Ok(false) => {
                        failures.push("Récupération Parakeet non admise".into());
                        break;
                    }
                    Err(error) => {
                        self.recovery_attempts.record(range);
                        failures.push(format!("Récupération Parakeet impossible : {error}"));
                        break;
                    }
                }
                while self.inflight && !self.unavailable && Instant::now() < deadline {
                    if let Err(error) = self.poll() {
                        failures.push(format!("Récupération Parakeet interrompue : {error}"));
                        break;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                if self.inflight || self.unavailable || !failures.is_empty() {
                    break;
                }
            }
        }
        if self.inflight && !failures.iter().any(|item| item.contains("Délai")) {
            failures.push("Délai de finalisation Parakeet dépassé".into());
        }
        let finalization = self.confirmation.finalize_observed(self.trace.enabled());
        if self.trace.enabled() {
            self.trace.record(serde_json::json!({
                "event": "confirmation_finalized",
                "job_id": format!("{:032x}", self.request.job_id.0),
                "generation": self.request.generation.get(),
                "session_id": self.session_id,
                "at_unix_ms": unix_time_ms(),
                "cursor_before_sample": finalization.cursor_before,
                "cursor_after_sample": finalization.cursor_after,
                "published_blocks": finalization.published.iter().map(block_trace_json).collect::<Vec<_>>(),
                "first_refusal": finalization.first_refusal.as_ref().map(refusal_trace_json),
                "unresolved_before": finalization.unresolved_before.iter().map(range_trace_json).collect::<Vec<_>>(),
                "unresolved_after": finalization.unresolved_after.iter().map(range_trace_json).collect::<Vec<_>>(),
                "publication_phase": "after_capture_stopped",
            }));
        }
        self.cancel_inference.store(true, Ordering::Release);
        let path = match self.wav.take() {
            Some(wav) => match wav.finish() {
                Ok(path) => path,
                Err(error) => {
                    failures.push(format!("Finalisation WAV impossible : {error}"));
                    String::new()
                }
            },
            None => {
                failures.push("WAV de capture indisponible à la clôture".into());
                String::new()
            }
        };
        let _ = self.inference_request.try_send(InferenceRequest::Stop);
        // The worker writes its stderr log beneath this session root. Keep that
        // diagnostic evidence; a non-empty root is not a capture-stop failure.
        self.stopped = true;
        let mut diagnostic = Vec::new();
        if self.unavailable {
            diagnostic.push("une ou plusieurs hypothèses n'ont pas été produites".to_owned());
        }
        diagnostic.extend(failures);
        let global_durable_end = self
            .request
            .group_offset_samples
            .saturating_add(durable_end);
        let mut complete =
            diagnostic.is_empty() && !self.confirmation.has_remainder(global_durable_end);
        if !complete && diagnostic.is_empty() {
            diagnostic.push("une portion audio demeure non confirmée".into());
        }
        let segments = self.confirmed_segments();
        let unresolved = self.confirmation.remaining_ranges(global_durable_end);
        let outputs = crate::direct_live::export_parakeet_outputs(
            &self.request,
            &path,
            durable_end,
            &segments,
            &unresolved,
            complete,
        );
        complete = outputs.complete;
        diagnostic.extend(outputs.diagnostics);
        if self.trace.enabled() {
            self.trace.record(serde_json::json!({
                "event": "capture_stopped_returned",
                "job_id": format!("{:032x}", self.request.job_id.0),
                "generation": self.request.generation.get(),
                "session_id": self.session_id,
                "at_unix_ms": unix_time_ms(),
                "complete": complete && diagnostic.is_empty(),
                "confirmed_segment_count": segments.len(),
                "unresolved_ranges": self.confirmation.remaining_ranges(global_durable_end).iter().map(range_trace_json).collect::<Vec<_>>(),
                "publication_phase": "after_capture_stopped",
            }));
            self.trace.finish();
        }
        Ok(ImportEvent::ParakeetCaptureStopped {
            job_id: self.request.job_id,
            generation: self.request.generation,
            session_id: self.session_id,
            wav_path: path,
            txt_path: outputs.txt_path,
            srt_path: outputs.srt_path,
            complete: complete && diagnostic.is_empty(),
            final_segments: segments,
            unresolved,
            diagnostic: (!diagnostic.is_empty()).then(|| diagnostic.join(" ; ")),
            automatic_final: None,
        })
    }

    pub(super) fn drain_stopped_capture(
        &mut self,
        capture: &mut CaptureStream,
    ) -> Result<(), String> {
        for _ in 0..crate::capture::CAPTURE_SLOTS {
            let Some(block) = capture.try_next_block() else {
                break;
            };
            let samples = self
                .converter
                .as_mut()
                .expect("converter retained until tail")
                .push(&block)?;
            capture.recycle_block(block)?;
            self.wav.as_mut().expect("active WAV").append(&samples)?;
        }
        Ok(())
    }

    pub(crate) fn stop_after_capture_error(&mut self, error: String) -> ImportEvent {
        self.finalizing = true;
        self.stopped = true;
        self.probe.invalidate();
        self.cancel_inference.store(true, Ordering::Release);
        if let Some(mut capture) = self.capture.take() {
            capture.stop();
            let _ = self.drain_stopped_capture(&mut capture);
        }
        if let Some(mut converter) = self.converter.take()
            && let Ok(tail) = converter.finish()
            && let Some(wav) = self.wav.as_mut()
        {
            let _ = wav.append(&tail);
        }
        let durable_end = self.wav.as_ref().map_or(0, WavOutput::sample_count);
        let wav_path = self.wav.take().and_then(|wav| wav.finish().ok());
        let _ = self.inference_request.try_send(InferenceRequest::Stop);
        self.unavailable = true;
        let segments = self.confirmed_segments();
        let unresolved = self.confirmation.remaining_ranges(
            self.request
                .group_offset_samples
                .saturating_add(durable_end),
        );
        let wav_path = wav_path.unwrap_or_default();
        let outputs = crate::direct_live::export_parakeet_outputs(
            &self.request,
            &wav_path,
            durable_end,
            &segments,
            &unresolved,
            false,
        );
        if self.trace.enabled() {
            self.trace.record(serde_json::json!({
                "event": "capture_stopped_after_error",
                "job_id": format!("{:032x}", self.request.job_id.0),
                "generation": self.request.generation.get(),
                "session_id": self.session_id,
                "at_unix_ms": unix_time_ms(),
                "error": bounded_text(&error, TRACE_MAX_TEXT_BYTES).0,
                "confirmed_segment_count": segments.len(),
                "unresolved_ranges": self.confirmation.remaining_ranges(
                    self.request.group_offset_samples.saturating_add(durable_end),
                ).iter().map(range_trace_json).collect::<Vec<_>>(),
                "publication_phase": "after_capture_error",
            }));
            self.trace.finish();
        }
        ImportEvent::ParakeetCaptureStopped {
            job_id: self.request.job_id,
            generation: self.request.generation,
            session_id: self.session_id,
            wav_path,
            txt_path: outputs.txt_path,
            srt_path: outputs.srt_path,
            complete: false,
            final_segments: segments,
            unresolved,
            diagnostic: Some(
                std::iter::once(format!(
                    "{error}; l'audio déjà écrit dans le WAV est conservé"
                ))
                .chain(outputs.diagnostics)
                .collect::<Vec<_>>()
                .join(" ; "),
            ),
            automatic_final: None,
        }
    }

    pub(super) fn confirmed_segments(&self) -> Vec<whisper_core::ports::WorkerSegment> {
        self.confirmation
            .confirmed()
            .iter()
            .map(|block| whisper_core::ports::WorkerSegment {
                job_id: self.request.job_id,
                generation: self.request.generation,
                instance_id: self.session_id,
                segment_id: SegmentId(block.segment_id),
                range: block.range,
                text: block.text.clone(),
            })
            .collect()
    }
}

impl Drop for ParakeetLive {
    fn drop(&mut self) {
        let _ = self.inference_request.try_send(InferenceRequest::Stop);
        // The bounded worker channel is closed when this session is dropped. Keep the
        // WAV and any unfinished window as recovery evidence; never delete user audio.
    }
}
