use super::*;

pub(in crate::worker_ipc) const TDT_FRAME_OVERLAP_SAMPLES: u64 = 80 * 16;
pub(in crate::worker_ipc) fn vad_word_overlap_within_one_tdt_frame(
    start_sample: u64,
    previous_end_sample: u64,
) -> bool {
    start_sample.saturating_add(TDT_FRAME_OVERLAP_SAMPLES) >= previous_end_sample
}

pub(in crate::worker_ipc) fn append_vad_parakeet_word_group(
    groups: &mut Vec<(u64, u64, Vec<String>)>,
    start_sample: u64,
    end_sample: u64,
    token: &str,
) -> Result<(), String> {
    if start_sample >= end_sample {
        return Err("InvalidTimestamp: Parakeet word range is empty".into());
    }
    let boundary = groups.last().is_some_and(|(_, _, tokens)| {
        tokens
            .last()
            .is_some_and(|last| last.ends_with('.') || last.ends_with('!') || last.ends_with('?'))
    });
    if groups.is_empty() || (boundary && start_sample >= groups.last().unwrap().1) {
        groups.push((start_sample, end_sample, vec![token.to_owned()]));
    } else if let Some((group_start, group_end, tokens)) = groups.last_mut() {
        *group_start = (*group_start).min(start_sample);
        *group_end = (*group_end).max(end_sample);
        tokens.push(token.to_owned());
    }
    Ok(())
}

pub(in crate::worker_ipc) fn validate_single_pass_parakeet_words(
    words: &[whisper_core::parakeet_ipc::Word],
    duration_samples: u64,
) -> Result<Vec<(u64, u64, String)>, String> {
    let mut validated = Vec::with_capacity(words.len());
    let mut previous_end = 0_u64;
    for (index, word) in words.iter().enumerate() {
        let start = u64::from(word.start_ms) * 16;
        let end = u64::from(word.end_ms) * 16;
        let text = word.text.trim();
        if text.is_empty() || start >= end || end > duration_samples {
            return Err(format!(
                "InvalidTimestamp: Parakeet word {index} is empty or outside the audio duration"
            ));
        }
        if index > 0 && start < previous_end {
            return Err(format!(
                "InvalidTimestamp: Parakeet word {index} overlaps or is unordered"
            ));
        }
        previous_end = end;
        validated.push((start, end, text.to_owned()));
    }
    Ok(validated)
}

pub(in crate::worker_ipc) fn append_single_pass_parakeet_word_group(
    groups: &mut Vec<(u64, u64, Vec<String>)>,
    start_sample: u64,
    end_sample: u64,
    token: &str,
) -> Result<(), String> {
    let boundary = groups.last().is_some_and(|(_, _, tokens)| {
        tokens
            .last()
            .is_some_and(|last| last.ends_with('.') || last.ends_with('!') || last.ends_with('?'))
    });
    if start_sample >= end_sample {
        return Err("InvalidTimestamp: sentence boundary leaves an empty word range".into());
    }
    if groups.is_empty() || boundary {
        groups.push((start_sample, end_sample, vec![token.to_owned()]));
    } else if let Some((_, group_end, tokens)) = groups.last_mut() {
        *group_end = end_sample;
        tokens.push(token.to_owned());
    }
    Ok(())
}
pub(in crate::worker_ipc) struct ParakeetVadRun {
    pub(in crate::worker_ipc) request: ImportRequest,
    pub(in crate::worker_ipc) decoder: Option<crate::decoder::DecoderProxy<ChildWorkerTransport>>,
    pub(in crate::worker_ipc) decode_offset: u64,
    pub(in crate::worker_ipc) total_samples: u64,
    pub(in crate::worker_ipc) source_sha256: String,
    pub(in crate::worker_ipc) pcm: Vec<i16>,
    pub(in crate::worker_ipc) windows: Vec<whisper_core::parakeet_windows::Window>,
    pub(in crate::worker_ipc) next_window: usize,
    pub(in crate::worker_ipc) next_segment_sequence: u64,
    pub(in crate::worker_ipc) request_count: u64,
    pub(in crate::worker_ipc) current_attempt: u64,
    pub(in crate::worker_ipc) current_path: Option<PathBuf>,
    pub(in crate::worker_ipc) current_source_sha256: Option<String>,
    pub(in crate::worker_ipc) pending_confirmation:
        Option<(whisper_core::parakeet_windows::Window, u64)>,
    pub(in crate::worker_ipc) worker_ready: bool,
    pub(in crate::worker_ipc) ready_event_sent: bool,
}

impl Drop for ParakeetVadRun {
    fn drop(&mut self) {
        if let Some(path) = self.current_path.take() {
            let _ = fs::remove_file(path);
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::worker_ipc) enum ParakeetStoppedExit {
    Wait,
    Success,
    Failure,
}

pub(in crate::worker_ipc) fn parakeet_stopped_exit(status: Option<bool>) -> ParakeetStoppedExit {
    match status {
        None => ParakeetStoppedExit::Wait,
        Some(true) => ParakeetStoppedExit::Success,
        Some(false) => ParakeetStoppedExit::Failure,
    }
}

pub(in crate::worker_ipc) fn parakeet_vad_segments(
    words: &[whisper_core::parakeet_ipc::Word],
    window: whisper_core::parakeet_windows::Window,
    total_samples: u64,
    job_id: JobId,
    generation: Generation,
    instance_id: u64,
    first_sequence: u64,
) -> Result<Vec<whisper_core::ipc::WorkerSegmentDto>, String> {
    let absolute =
        whisper_core::parakeet_windows::absolute_word_samples(&window, total_samples, words)
            .map_err(|error| format!("InvalidTimestamp: {error}"))?;
    let mut groups: Vec<(u64, u64, Vec<String>)> = Vec::new();
    let mut previous_end = 0_u64;
    for (word, time) in words.iter().zip(absolute) {
        let token = word.text.trim();
        if token.is_empty()
            || !vad_word_overlap_within_one_tdt_frame(time.start_sample, previous_end)
            || time.start_sample >= time.end_sample
        {
            return Err("InvalidTimestamp: Parakeet VAD word is empty or unordered".into());
        }
        previous_end = previous_end.max(time.end_sample);
        if !whisper_core::parakeet_windows::word_belongs_to_central_range(
            time.start_sample,
            time.end_sample,
            window.central_start_sample,
            window.central_end_sample,
        ) {
            continue;
        }
        append_vad_parakeet_word_group(&mut groups, time.start_sample, time.end_sample, token)?;
    }
    groups
        .into_iter()
        .enumerate()
        .map(|(index, (start, end, tokens))| {
            let sequence = first_sequence
                .checked_add(index as u64)
                .ok_or_else(|| "InvalidInput: Parakeet segment sequence exhausted".to_owned())?;
            let range = whisper_core::SourceRange::new(start, end, 16_000)
                .ok_or_else(|| "InvalidTimestamp: VAD segment range is invalid".to_owned())?;
            Ok(whisper_core::ipc::WorkerSegmentDto {
                job_id,
                generation,
                instance_id,
                segment_id: whisper_core::SegmentId(sequence),
                range,
                text: tokens.join(" "),
            })
        })
        .collect()
}

impl ImportRuntime {
    pub(super) fn advance_parakeet_vad_decode(
        &mut self,
    ) -> Result<(), (JobId, Generation, String)> {
        let Some(vad) = self.parakeet_vad.as_mut() else {
            return Ok(());
        };
        if vad.decode_offset >= vad.total_samples || vad.decoder.is_none() {
            return Ok(());
        }
        let request = vad.request.clone();
        let start = vad.decode_offset;
        let end = start.saturating_add(80_000).min(vad.total_samples);
        let Some(range) = whisper_core::SourceRange::new(start, end, 16_000) else {
            return Err((
                request.job_id,
                request.generation,
                "InvalidInput: invalid VAD decode range".into(),
            ));
        };
        let sha256 = decode_sha256(&vad.source_sha256)
            .map_err(|error| (request.job_id, request.generation, error))?;
        let decoded = vad
            .decoder
            .as_mut()
            .ok_or((
                request.job_id,
                request.generation,
                "DecoderUnavailable".into(),
            ))?
            .decode(DecodeRequest {
                source: SourceIdentity {
                    source_id: request.source_path.clone(),
                    sha256,
                },
                job_id: request.job_id,
                generation: request.generation,
                range,
                max_samples: 80_000,
            })
            .map_err(|error| {
                (
                    request.job_id,
                    request.generation,
                    format!("DecoderFailed: {error:?}"),
                )
            })?;
        let Some(block) = decoded.into_iter().next() else {
            return Err((
                request.job_id,
                request.generation,
                "DecoderFailed: empty VAD decode block".into(),
            ));
        };
        if block.block.range.start_sample != start
            || block.block.range.end_sample != end
            || block.block.samples.len() as u64 != end - start
        {
            return Err((
                request.job_id,
                request.generation,
                "DecoderFailed: VAD decode block has a gap or wrong length".into(),
            ));
        }
        vad.pcm
            .try_reserve(block.block.samples.len())
            .map_err(|error| {
                (
                    request.job_id,
                    request.generation,
                    format!("MemoryUnavailable: VAD PCM allocation: {error}"),
                )
            })?;
        vad.pcm.extend(block.block.samples);
        vad.decode_offset = end;
        if vad.decode_offset == vad.total_samples {
            vad.decoder = None;
            if vad.pcm.len() as u64 != vad.total_samples {
                return Err((
                    request.job_id,
                    request.generation,
                    "DecoderFailed: decoded sample count differs from inspected duration".into(),
                ));
            }
            vad.windows = whisper_core::parakeet_windows::plan_windows(&vad.pcm)
                .map_err(|error| (request.job_id, request.generation, format!("{error}")))?;
            let confirmed = self
                .archive
                .confirmed_import_vad_offset()
                .map_err(|error| (request.job_id, request.generation, error))?;
            if confirmed > 0 {
                let next = vad
                    .windows
                    .iter()
                    .position(|window| window.central_start_sample == confirmed)
                    .or_else(|| (confirmed == vad.total_samples).then_some(vad.windows.len()))
                    .ok_or((
                        request.job_id,
                        request.generation,
                        "StorageCorrupt: confirmed VAD offset is not a planned boundary".into(),
                    ))?;
                vad.next_window = next;
            }
            vad.next_segment_sequence = self.archive.next_segment_sequence();
        }
        Ok(())
    }

    pub(super) fn begin_next_parakeet_vad_window(
        &mut self,
    ) -> Result<(), (JobId, Generation, String)> {
        let Some(vad) = self.parakeet_vad.as_mut() else {
            return Ok(());
        };
        let request = vad.request.clone();
        if vad.next_window >= vad.windows.len() {
            let last_sequence = vad.next_segment_sequence.saturating_sub(1);
            self.parakeet_events.push_back(WorkerEvent::Progress {
                job_id: request.job_id,
                generation: request.generation,
                instance_id: self.active.as_ref().map_or(0, |active| active.instance_id),
                completed_samples: vad.total_samples,
                total_samples: vad.total_samples,
            });
            self.parakeet_end = Some((
                request.job_id,
                request.generation,
                last_sequence,
                vad.total_samples,
            ));
            self.parakeet
                .as_mut()
                .ok_or((
                    request.job_id,
                    request.generation,
                    "WorkerExited: Parakeet process is absent".into(),
                ))?
                .shutdown()
                .map_err(|error| (request.job_id, request.generation, error))?;
            return Ok(());
        }
        let window = vad.windows[vad.next_window];
        let start = usize::try_from(window.native_start_sample).map_err(|_| {
            (
                request.job_id,
                request.generation,
                "InvalidInput: VAD offset exceeds address space".into(),
            )
        })?;
        let end = usize::try_from(window.native_end_sample).map_err(|_| {
            (
                request.job_id,
                request.generation,
                "InvalidInput: VAD offset exceeds address space".into(),
            )
        })?;
        let samples = vad.pcm.get(start..end).ok_or((
            request.job_id,
            request.generation,
            "StorageCorrupt: VAD window is outside decoded PCM".into(),
        ))?;
        let directory = PathBuf::from(&request.destination)
            .join("transcriptions")
            .join(format!("{:032x}", request.job_id.0))
            .join(request.generation.get().to_string())
            .join(".parakeet-vad");
        fs::create_dir_all(&directory).map_err(|error| {
            (
                request.job_id,
                request.generation,
                format!("StorageUnavailable: VAD temp directory: {error}"),
            )
        })?;
        let path = directory.join(format!("window-{:06}.wav", window.index));
        write_pcm16_wav(&path, samples)
            .map_err(|error| (request.job_id, request.generation, error))?;
        let source_sha256 = crate::archive::sha256_file(&path)
            .map(|sha| format_sha256(&sha))
            .map_err(|error| {
                (
                    request.job_id,
                    request.generation,
                    format!("StorageUnavailable: VAD window hash: {error}"),
                )
            })?;
        vad.current_source_sha256 = Some(source_sha256.clone());
        let attempt = window.index as u64 + 1;
        let process = self.parakeet.as_mut().ok_or((
            request.job_id,
            request.generation,
            "WorkerExited: Parakeet process is absent".into(),
        ))?;
        process
            .transcribe(
                format!("{:032x}", request.job_id.0),
                request.generation.get(),
                attempt,
                &path,
                source_sha256,
            )
            .map_err(|error| (request.job_id, request.generation, error))?;
        vad.current_attempt = attempt;
        vad.request_count = vad.request_count.saturating_add(1);
        vad.current_path = Some(path);
        Ok(())
    }

    fn handle_parakeet_vad_result(
        &mut self,
        request_id: u64,
        job_id: String,
        generation: u64,
        attempt: u64,
        identity: whisper_core::parakeet_ipc::WorkerIdentity,
        source_sha256: String,
        duration_samples: u64,
        words: Vec<whisper_core::parakeet_ipc::Word>,
    ) -> Option<Result<WorkerEvent, PortError>> {
        let active = self.active.as_ref()?;
        let id = (active.job_id, active.generation, active.instance_id);
        let Some(run) = self.parakeet_vad.as_mut() else {
            return Some(Err(PortError::Failed(
                "StaleResponse: VAD state disappeared".into(),
            )));
        };
        let Some(window) = run.windows.get(run.next_window).copied() else {
            return Some(Err(PortError::Failed(
                "ProtocolMismatch: VAD result has no pending window".into(),
            )));
        };
        let expected_identity = self
            .parakeet
            .as_ref()
            .and_then(|process| process.identity());
        let expected_duration = window.native_end_sample - window.native_start_sample;
        if request_id != run.request_count
            || job_id != format!("{:032x}", active.job_id.0)
            || generation != active.generation.get()
            || attempt != run.current_attempt
            || Some(&identity) != expected_identity
            || run.current_source_sha256.as_deref() != Some(source_sha256.as_str())
            || duration_samples != expected_duration
        {
            return Some(Err(PortError::Failed(
                "ProtocolMismatch: Parakeet VAD result identity, source, attempt, or duration differs".into(),
            )));
        }
        if let Some(path) = run.current_path.take() {
            let _ = fs::remove_file(path);
        }
        run.current_source_sha256 = None;
        let segments = match parakeet_vad_segments(
            &words,
            window,
            run.total_samples,
            id.0,
            id.1,
            id.2,
            run.next_segment_sequence,
        ) {
            Ok(segments) => segments,
            Err(error) => return Some(Err(PortError::Failed(error))),
        };
        run.next_segment_sequence =
            match run.next_segment_sequence.checked_add(segments.len() as u64) {
                Some(next) => next,
                None => {
                    return Some(Err(PortError::Failed(
                        "InvalidInput: Parakeet segment sequence exhausted".into(),
                    )));
                }
            };
        let last_sequence = run.next_segment_sequence.saturating_sub(1);
        run.pending_confirmation = Some((window, last_sequence));
        self.parakeet_events.push_back(WorkerEvent::Progress {
            job_id: id.0,
            generation: id.1,
            instance_id: id.2,
            completed_samples: window.central_end_sample,
            total_samples: 0,
        });
        for segment in segments {
            self.parakeet_events
                .push_back(WorkerEvent::Segment(segment));
        }
        if self.parakeet_events.len() == 1 {
            if let Err(error) = self.confirm_and_advance_parakeet_vad_window(window, last_sequence)
            {
                return Some(Err(PortError::Failed(error)));
            }
        }
        self.parakeet_events.pop_front().map(Ok)
    }

    pub(in crate::worker_ipc::runtime) fn confirm_and_advance_parakeet_vad_window(
        &mut self,
        window: whisper_core::parakeet_windows::Window,
        last_segment_sequence: u64,
    ) -> Result<(), String> {
        self.archive
            .confirm_import_vad_window(window, last_segment_sequence)?;
        let Some(run) = self.parakeet_vad.as_mut() else {
            return Err("StaleResponse: VAD state disappeared at confirmation".into());
        };
        run.pending_confirmation = None;
        run.next_window = run
            .next_window
            .checked_add(1)
            .ok_or_else(|| "InvalidInput: VAD window sequence exhausted".to_owned())?;
        self.begin_next_parakeet_vad_window()
            .map_err(|(_, _, error)| error)
    }

    pub(super) fn poll_parakeet_worker(&mut self) -> Option<Result<WorkerEvent, PortError>> {
        #[cfg(feature = "recette-hooks")]
        let destination = self
            .active
            .as_ref()
            .and_then(|active| active.import_request.as_ref())
            .map(|request| PathBuf::from(&request.destination));
        #[cfg(feature = "recette-hooks")]
        if let Some(destination) = destination {
            match take_acceptance_worker_fault(&destination, self.parakeet_native_call_started) {
                Ok(Some(message)) => {
                    let process = self.parakeet.as_mut()?;
                    if message.starts_with("IpcDecode:") {
                        if let Err(error) = process.inject_incomplete_result_for_acceptance() {
                            return Some(Err(PortError::Failed(error)));
                        }
                    } else {
                        return Some(match process.terminate() {
                            Ok(status) => {
                                Err(PortError::Failed(format!("{message}; child={status}")))
                            }
                            Err(error) => Err(PortError::Failed(format!(
                                "{message}; termination failed: {error}"
                            ))),
                        });
                    }
                }
                Ok(None) => {}
                Err(error) => return Some(Err(PortError::Failed(error))),
            }
        }
        if let Some(event) = self.parakeet_events.pop_front() {
            return Some(Ok(event));
        }
        if self.parakeet_stopped_ack {
            let status = self.parakeet.as_mut()?.try_wait();
            return match status {
                Ok(None) if parakeet_stopped_exit(None) == ParakeetStoppedExit::Wait => None,
                Ok(None) => None,
                Ok(Some(status))
                    if parakeet_stopped_exit(Some(status.success()))
                        == ParakeetStoppedExit::Success =>
                {
                    self.parakeet = None;
                    self.parakeet_stopped_ack = false;
                    if let Some((job_id, generation, last_sequence, last_offset)) =
                        self.parakeet_end.take()
                    {
                        let instance_id = self.active.as_ref()?.instance_id;
                        Some(Ok(WorkerEvent::End {
                            job_id,
                            generation,
                            instance_id,
                            last_sequence,
                            last_offset,
                        }))
                    } else {
                        Some(Err(PortError::Failed(
                            "WorkerExited: Parakeet stopped without a transcription result".into(),
                        )))
                    }
                }
                Ok(Some(status)) => {
                    self.parakeet = None;
                    self.parakeet_stopped_ack = false;
                    self.parakeet_end = None;
                    Some(Err(PortError::Failed(format!("WorkerExited: {status}"))))
                }
                Err(error) => Some(Err(PortError::Failed(error))),
            };
        }
        let received = match self.parakeet.as_mut()?.poll() {
            Ok(Some(event)) => event,
            Ok(None) => {
                let status = self.parakeet.as_mut()?.try_wait();
                return match status {
                    Ok(None) => None,
                    Ok(Some(status)) if status.success() => {
                        self.parakeet = None;
                        self.parakeet_stopped_ack = false;
                        self.parakeet_end = None;
                        Some(Err(PortError::Failed(
                            "WorkerExited: process ended before result acknowledgement".into(),
                        )))
                    }
                    Ok(Some(status)) => {
                        self.parakeet = None;
                        self.parakeet_stopped_ack = false;
                        Some(Err(PortError::Failed(format!("WorkerExited: {status}"))))
                    }
                    Err(error) => Some(Err(PortError::Failed(error))),
                };
            }
            Err(error) => {
                if let Some(process) = self.parakeet.as_mut() {
                    let _ = process.terminate();
                }
                self.parakeet = None;
                self.parakeet_stopped_ack = false;
                return Some(Err(PortError::Failed(error)));
            }
        };
        match received {
            whisper_core::parakeet_ipc::Event::Ready {
                identity,
                backend,
                gpu_evidence,
                ..
            } => {
                let (
                    request,
                    expected_backend,
                    attempt_number,
                    active_job_id,
                    active_generation,
                    active_instance_id,
                ) = {
                    let Some(active) = self.active.as_ref() else {
                        return Some(Err(PortError::Failed(
                            "StaleResponse: Parakeet Ready without an active import".into(),
                        )));
                    };
                    let Some(request) = active.import_request.clone() else {
                        return Some(Err(PortError::Failed(
                            "StaleResponse: Parakeet request is unavailable".into(),
                        )));
                    };
                    (
                        request,
                        active.backend,
                        active.attempt_number,
                        active.job_id,
                        active.generation,
                        active.instance_id,
                    )
                };
                let profile = if self.parakeet_vad.is_some() {
                    whisper_core::parakeet_windows::PROFILE_IMPORT_VAD
                } else {
                    whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS
                };
                let record = whisper_core::model_registry::model(
                    whisper_core::model_catalog::ModelId::ParakeetTdt06BV3,
                );
                let expected_gpu = self.parakeet_expected_gpu.clone();
                let expected_worker_paths = if expected_backend == BackendKind::Cuda {
                    self.gpu_parakeet_paths.as_ref()
                } else {
                    self.parakeet_paths.as_ref()
                };
                let key = expected_worker_paths
                    .and_then(|paths| {
                        if expected_backend == BackendKind::Cuda {
                            paths.gpu_qualification_key(profile).ok()
                        } else {
                            paths.qualification_key(profile).ok()
                        }
                    })
                    .unwrap_or_else(crate::archive::ArchiveQualificationKey::parakeet_cpu_test);
                let backend_matches = match expected_backend {
                    BackendKind::Cpu => backend == "CPU" && gpu_evidence.is_none(),
                    BackendKind::Cuda => {
                        backend == "CUDA0"
                            && self.parakeet.as_ref().is_some_and(|process| {
                                gpu_evidence_matches(
                                    gpu_evidence.as_ref(),
                                    expected_gpu.as_ref(),
                                    process.pid(),
                                )
                            })
                    }
                };
                if !backend_matches
                    || identity.model_sha256 != record.artifact.sha256
                    || identity.runtime_sha256 != key.runtime_build
                    || identity.build_sha256 != key.worker_build
                    || identity.profile != profile
                {
                    if expected_backend == BackendKind::Cuda
                        && request.config.compute == whisper_core::ComputeChoice::Auto
                    {
                        return match self.fallback_parakeet_to_cpu(
                            request,
                            "GPU worker Ready proof did not match the qualified key".into(),
                        ) {
                            Ok(()) => None,
                            Err(error) => Some(Err(PortError::Failed(error))),
                        };
                    }
                    return Some(Err(PortError::Failed(
                        "BackendMismatch: Parakeet worker identity differs from the qualified key"
                            .into(),
                    )));
                }
                let process = self.parakeet.as_mut()?;
                if let Err(error) = process.accept_ready(identity) {
                    if expected_backend == BackendKind::Cuda
                        && request.config.compute == whisper_core::ComputeChoice::Auto
                    {
                        return match self.fallback_parakeet_to_cpu(
                            request,
                            format!("GPU worker readiness failed: {error}"),
                        ) {
                            Ok(()) => None,
                            Err(error) => Some(Err(PortError::Failed(error))),
                        };
                    }
                    return Some(Err(PortError::Failed(error)));
                }
                if expected_backend == BackendKind::Cuda
                    && let Some(active) = self.active.as_mut()
                {
                    active.gpu_attested = true;
                }
                if let Some(vad) = self.parakeet_vad.as_mut() {
                    vad.worker_ready = true;
                    return None;
                }
                let source_sha = request
                    .source_sha256
                    .map(|sha| format_sha256(&sha))
                    .unwrap_or_default();
                if let Err(error) = process.transcribe_with_gpu(
                    format!("{:032x}", request.job_id.0),
                    request.generation.get(),
                    attempt_number,
                    Path::new(&request.source_path),
                    source_sha,
                    expected_gpu,
                ) {
                    if expected_backend == BackendKind::Cuda
                        && request.config.compute == whisper_core::ComputeChoice::Auto
                    {
                        return match self.fallback_parakeet_to_cpu(
                            request,
                            format!("GPU request preparation failed: {error}"),
                        ) {
                            Ok(()) => None,
                            Err(error) => Some(Err(PortError::Failed(error))),
                        };
                    }
                    return Some(Err(PortError::Failed(error)));
                }
                #[cfg(feature = "recette-hooks")]
                match take_acceptance_worker_fault(Path::new(&request.destination), true) {
                    Ok(Some(message)) if message.starts_with("IpcDecode:") => {
                        if let Err(error) = self
                            .parakeet
                            .as_ref()?
                            .inject_incomplete_result_for_acceptance()
                        {
                            return Some(Err(PortError::Failed(error)));
                        }
                    }
                    Ok(Some(message)) => {
                        let process = self.parakeet.as_mut()?;
                        if let Err(error) = process.terminate() {
                            return Some(Err(PortError::Failed(format!(
                                "{message}; termination failed: {error}"
                            ))));
                        }
                    }
                    Ok(None) => {}
                    Err(error) => return Some(Err(PortError::Failed(error))),
                }
                Some(Ok(WorkerEvent::Ready {
                    job_id: active_job_id,
                    generation: active_generation,
                    instance_id: active_instance_id,
                    backend: expected_backend,
                }))
            }
            whisper_core::parakeet_ipc::Event::Result {
                request_id,
                job_id,
                generation,
                attempt,
                identity,
                source_sha256,
                duration_samples,
                words,
                gpu_evidence,
                ..
            } => {
                if self.parakeet_vad.is_some() {
                    return self.handle_parakeet_vad_result(
                        request_id,
                        job_id,
                        generation,
                        attempt,
                        identity,
                        source_sha256,
                        duration_samples,
                        words,
                    );
                }
                let Some(active) = self.active.as_ref() else {
                    return Some(Err(PortError::Failed(
                        "StaleResponse: Parakeet result without active import".into(),
                    )));
                };
                let Some(request) = active.import_request.as_ref() else {
                    return Some(Err(PortError::Failed(
                        "StaleResponse: Parakeet request is unavailable".into(),
                    )));
                };
                let current_identity = self.parakeet.as_ref().and_then(|p| p.identity());
                let expected_gpu = self.parakeet_expected_gpu.as_ref();
                let gpu_result_matches = if active.backend == BackendKind::Cuda {
                    self.parakeet.as_ref().is_some_and(|process| {
                        gpu_evidence_matches(gpu_evidence.as_ref(), expected_gpu, process.pid())
                    }) && self.parakeet_native_call_started
                } else {
                    gpu_evidence.is_none()
                };
                if !gpu_result_matches
                    || request_id != 1
                    || job_id != format!("{:032x}", active.job_id.0)
                    || generation != active.generation.get()
                    || attempt != active.attempt_number
                    || request
                        .source_sha256
                        .map(|sha| format_sha256(&sha))
                        .as_deref()
                        != Some(source_sha256.as_str())
                    || request.source_samples != Some(duration_samples)
                    || current_identity != Some(&identity)
                    || words.is_empty()
                {
                    return Some(Err(PortError::Failed(
                        "ProtocolMismatch: Parakeet result identity, duration, or words differ"
                            .into(),
                    )));
                }
                let (job_id, generation, instance_id) =
                    (active.job_id, active.generation, active.instance_id);
                self.parakeet_events.push_back(WorkerEvent::Progress {
                    job_id,
                    generation,
                    instance_id,
                    completed_samples: duration_samples,
                    total_samples: duration_samples,
                });
                let validation_started = Instant::now();
                let validated_words =
                    match validate_single_pass_parakeet_words(&words, duration_samples) {
                        Ok(words) => words,
                        Err(error) => return Some(Err(PortError::Failed(error))),
                    };
                crate::diagnostics::record_stage_timing(
                    "application_word_validation",
                    validation_started.elapsed(),
                    serde_json::json!({
                        "input_words": words.len(),
                        "validated_words": validated_words.len(),
                    }),
                );
                let grouping_started = Instant::now();
                let mut groups: Vec<(u64, u64, Vec<String>)> = Vec::new();
                for (start, end, token) in validated_words {
                    if let Err(error) =
                        append_single_pass_parakeet_word_group(&mut groups, start, end, &token)
                    {
                        return Some(Err(PortError::Failed(error)));
                    }
                }
                crate::diagnostics::record_stage_timing(
                    "application_word_grouping",
                    grouping_started.elapsed(),
                    serde_json::json!({ "groups": groups.len() }),
                );
                let last_sequence = groups.len() as u64;
                for (sequence, (start, end, tokens)) in groups.into_iter().enumerate() {
                    let Some(range) = whisper_core::SourceRange::new(start, end, 16_000) else {
                        return Some(Err(PortError::Failed(
                            "InvalidTimestamp: Parakeet sentence range is invalid".into(),
                        )));
                    };
                    self.parakeet_events.push_back(WorkerEvent::Segment(
                        whisper_core::ipc::WorkerSegmentDto {
                            job_id,
                            generation,
                            instance_id,
                            segment_id: whisper_core::SegmentId(sequence as u64 + 1),
                            range,
                            text: tokens.join(" "),
                        },
                    ));
                }
                self.parakeet_end = Some((job_id, generation, last_sequence, duration_samples));
                if let Some(process) = self.parakeet.as_mut()
                    && let Err(error) = process.shutdown()
                {
                    return Some(Err(PortError::Failed(error)));
                }
                self.parakeet_events.pop_front().map(Ok)
            }
            whisper_core::parakeet_ipc::Event::Failed {
                request_id,
                job_id: reported_job_id,
                generation: reported_generation,
                attempt,
                identity,
                code,
                message,
                ..
            } => {
                let Some(active) = self.active.as_ref() else {
                    return Some(Err(PortError::Failed(
                        "StaleResponse: Parakeet failure without active import".into(),
                    )));
                };
                if reported_job_id
                    .as_deref()
                    .is_some_and(|reported| reported != format!("{:032x}", active.job_id.0))
                    || reported_generation
                        .is_some_and(|reported| reported != active.generation.get())
                    || identity.as_ref().is_some_and(|reported| {
                        self.parakeet
                            .as_ref()
                            .and_then(|process| process.identity())
                            .is_some_and(|expected| expected != reported)
                    })
                {
                    return Some(Err(PortError::Failed(
                        "StaleResponse: Parakeet failure identity differs from active import"
                            .into(),
                    )));
                }
                let cpu_retry = self.active.as_ref().and_then(|active| {
                    let request = active.import_request.as_ref()?;
                    (active.backend == BackendKind::Cuda
                        && request.config.compute == whisper_core::ComputeChoice::Auto
                        && !self.parakeet_native_call_started
                        && request_id.is_none_or(|request_id| request_id == 1)
                        && attempt.is_none_or(|attempt| attempt == active.attempt_number))
                    .then(|| request.clone())
                });
                if let Some(request) = cpu_retry {
                    return match self.fallback_parakeet_to_cpu(
                        request,
                        format!("GPU worker failed before native inference: {code}: {message}"),
                    ) {
                        Ok(()) => None,
                        Err(error) => Some(Err(PortError::Failed(error))),
                    };
                }
                let (job_id, generation, instance_id) =
                    (active.job_id, active.generation, active.instance_id);
                if let Some(process) = self.parakeet.as_mut() {
                    let _ = process.terminate();
                }
                self.parakeet = None;
                self.parakeet_stopped_ack = false;
                Some(Ok(WorkerEvent::Failed {
                    job_id: Some(job_id),
                    generation: Some(generation),
                    instance_id: Some(instance_id),
                    code: match code.as_str() {
                        "ModelHashMismatch" => {
                            whisper_core::ipc::WorkerErrorCode::ModelHashMismatch
                        }
                        "ProtocolVersionMismatch" | "InvalidMessage" => {
                            whisper_core::ipc::WorkerErrorCode::ProtocolMismatch
                        }
                        _ => whisper_core::ipc::WorkerErrorCode::Internal,
                    },
                    message: format!("{code}: {message}"),
                }))
            }
            whisper_core::parakeet_ipc::Event::NativeCallStarted {
                request_id,
                job_id,
                generation,
                attempt,
                pid,
            } => {
                let valid = self.active.as_ref().is_some_and(|active| {
                    active.import_request.is_some()
                        && request_id == 1
                        && job_id == format!("{:032x}", active.job_id.0)
                        && generation == active.generation.get()
                        && attempt == active.attempt_number
                        && (active.backend != BackendKind::Cuda || active.gpu_attested)
                        && self
                            .parakeet
                            .as_ref()
                            .is_some_and(|process| process.pid() == pid)
                });
                if !valid {
                    return Some(Err(PortError::Failed(
                        "ProtocolMismatch: NativeCallStarted is not bound to the active Parakeet request".into(),
                    )));
                }
                self.parakeet_native_call_started = true;
                None
            }
            whisper_core::parakeet_ipc::Event::Stopped => {
                self.parakeet_stopped_ack = true;
                self.poll_parakeet_worker()
            }
        }
    }
}
