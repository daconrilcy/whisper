use super::*;
use crate::ports::{ParakeetCaptureIdentity, ParakeetCaptureView, ParakeetCaptureViewState};

impl<P: ImportIoPort> ImportApplication<P> {
    pub(super) fn track_parakeet_capture_event(&mut self, event: &ImportEvent) {
        match event {
            ImportEvent::ParakeetLiveStarted { .. }
            | ImportEvent::ParakeetInferenceReady { .. } => self.track_capture_start(event),
            ImportEvent::ParakeetHypothesis { .. } | ImportEvent::ParakeetConfirmed { .. } => {
                self.track_capture_text(event)
            }
            ImportEvent::ParakeetUnresolved { .. } | ImportEvent::ParakeetResolved { .. } => {
                self.track_capture_ranges(event)
            }
            ImportEvent::ParakeetCaptureStopped { .. } => self.track_capture_final(event),
            ImportEvent::Failed { .. } => self.track_capture_failure(event),
            ImportEvent::Stopped { .. } => self.track_capture_stopped(event),
            _ => {}
        }
    }
    fn ensure_parakeet_capture(
        captures: &mut Vec<ParakeetCaptureView>,
        identity: ParakeetCaptureIdentity,
    ) -> usize {
        if let Some(index) = captures
            .iter()
            .position(|capture| capture.identity == identity)
        {
            return index;
        }
        captures.push(ParakeetCaptureView {
            identity,
            wav_path: None,
            direct_txt_path: None,
            direct_srt_path: None,
            companion_json_path: None,
            confirmed_segments: Vec::new(),
            provisional_text: None,
            provisional_range: None,
            provisional_sequence: None,
            unresolved: Vec::new(),
            capture_state: ParakeetCaptureViewState::Starting,
            capture_complete: None,
            capture_diagnostic: None,
            requested_compute: None,
            effective_backend: None,
            automatic_final: None,
            automatic_final_identity: None,
        });
        captures.len() - 1
    }

    fn track_capture_start(&mut self, event: &ImportEvent) {
        match event {
            ImportEvent::ParakeetLiveStarted {
                job_id,
                generation,
                session_id,
                ..
            } => {
                let index = Self::ensure_parakeet_capture(
                    &mut self.view.parakeet_captures,
                    ParakeetCaptureIdentity {
                        job_id: *job_id,
                        generation: *generation,
                        session_id: *session_id,
                    },
                );
                let capture = &mut self.view.parakeet_captures[index];
                if matches!(
                    capture.capture_state,
                    ParakeetCaptureViewState::Finalized | ParakeetCaptureViewState::Failed
                ) {
                    return;
                }
                capture.capture_state = ParakeetCaptureViewState::Capturing;
                if let Some((identity, requested)) = self.active_live_compute
                    && identity
                        == (CommandIdentity {
                            job_id: *job_id,
                            generation: *generation,
                        })
                {
                    capture.requested_compute = Some(requested);
                }
                self.trim_parakeet_capture_views();
            }
            ImportEvent::ParakeetInferenceReady {
                job_id,
                generation,
                session_id,
                backend,
            } => {
                if let Some(capture) =
                    self.find_parakeet_capture_mut(*job_id, *generation, *session_id)
                    && matches!(
                        capture.capture_state,
                        ParakeetCaptureViewState::Capturing | ParakeetCaptureViewState::Finalizing
                    )
                {
                    capture.effective_backend = Some(backend.clone());
                }
            }
            _ => unreachable!("capture event group already selected"),
        }
    }

    fn track_capture_text(&mut self, event: &ImportEvent) {
        match event {
            ImportEvent::ParakeetHypothesis {
                job_id,
                generation,
                session_id,
                sequence,
                range,
                text,
                ..
            } => {
                if *sequence > 0
                    && range.sample_rate_hz == 16_000
                    && range.start_sample < range.end_sample
                    && !text.trim().is_empty()
                    && let Some(capture) =
                        self.find_parakeet_capture_mut(*job_id, *generation, *session_id)
                    && capture.capture_state == ParakeetCaptureViewState::Capturing
                    && capture
                        .provisional_sequence
                        .is_none_or(|prior| *sequence > prior)
                {
                    capture.provisional_text = Some(text.clone());
                    capture.provisional_range = Some(*range);
                    capture.provisional_sequence = Some(*sequence);
                }
            }
            ImportEvent::ParakeetConfirmed {
                job_id,
                generation,
                session_id,
                segment,
            } => {
                if segment.job_id == *job_id
                    && segment.generation == *generation
                    && segment.instance_id == *session_id
                    && segment.range.sample_rate_hz == 16_000
                    && segment.range.start_sample < segment.range.end_sample
                    && !segment.text.trim().is_empty()
                    && let Some(capture) =
                        self.find_parakeet_capture_mut(*job_id, *generation, *session_id)
                    && matches!(
                        capture.capture_state,
                        ParakeetCaptureViewState::Capturing | ParakeetCaptureViewState::Finalizing
                    )
                {
                    if let Some(prior) = capture
                        .confirmed_segments
                        .iter()
                        .find(|prior| prior.segment_id == segment.segment_id)
                    {
                        if prior != segment {
                            capture.capture_diagnostic = Some(
                                "Événement confirmé incohérent ignoré pour cette capture".into(),
                            );
                        }
                    } else {
                        capture.confirmed_segments.push(segment.clone());
                        capture.confirmed_segments.sort_by_key(|item| {
                            (
                                item.range.start_sample,
                                item.range.end_sample,
                                item.segment_id.0,
                            )
                        });
                        if capture
                            .provisional_range
                            .is_some_and(|range| range.end_sample <= segment.range.end_sample)
                        {
                            capture.provisional_text = None;
                            capture.provisional_range = None;
                            capture.provisional_sequence = None;
                        }
                    }
                }
            }
            _ => unreachable!("capture event group already selected"),
        }
    }

    fn track_capture_ranges(&mut self, event: &ImportEvent) {
        match event {
            ImportEvent::ParakeetUnresolved {
                job_id,
                generation,
                session_id,
                range,
            } => {
                if range.sample_rate_hz == 16_000
                    && range.start_sample < range.end_sample
                    && let Some(capture) =
                        self.find_parakeet_capture_mut(*job_id, *generation, *session_id)
                    && matches!(
                        capture.capture_state,
                        ParakeetCaptureViewState::Capturing | ParakeetCaptureViewState::Finalizing
                    )
                    && !capture.unresolved.contains(range)
                {
                    capture.unresolved.push(*range);
                    capture
                        .unresolved
                        .sort_by_key(|item| (item.start_sample, item.end_sample));
                }
            }
            ImportEvent::ParakeetResolved {
                job_id,
                generation,
                session_id,
                range,
            } => {
                if range.sample_rate_hz == 16_000
                    && range.start_sample < range.end_sample
                    && let Some(capture) =
                        self.find_parakeet_capture_mut(*job_id, *generation, *session_id)
                    && matches!(
                        capture.capture_state,
                        ParakeetCaptureViewState::Capturing | ParakeetCaptureViewState::Finalizing
                    )
                {
                    capture.unresolved.retain(|item| item != range);
                }
            }
            _ => unreachable!("capture event group already selected"),
        }
    }

    fn track_capture_final(&mut self, event: &ImportEvent) {
        match event {
            ImportEvent::ParakeetCaptureStopped {
                job_id,
                generation,
                session_id,
                wav_path,
                txt_path,
                srt_path,
                complete,
                final_segments,
                unresolved,
                diagnostic,
                automatic_final,
            } => {
                let identity = ParakeetCaptureIdentity {
                    job_id: *job_id,
                    generation: *generation,
                    session_id: *session_id,
                };
                let index =
                    Self::ensure_parakeet_capture(&mut self.view.parakeet_captures, identity);
                let capture = &mut self.view.parakeet_captures[index];
                if matches!(
                    capture.capture_state,
                    ParakeetCaptureViewState::Finalized | ParakeetCaptureViewState::Failed
                ) {
                    return;
                }
                capture.capture_state = if wav_path.is_empty() {
                    ParakeetCaptureViewState::Failed
                } else {
                    ParakeetCaptureViewState::Finalized
                };
                if let Some((active_identity, requested)) = self.active_live_compute
                    && active_identity
                        == (CommandIdentity {
                            job_id: *job_id,
                            generation: *generation,
                        })
                {
                    capture.requested_compute = Some(requested);
                    self.active_live_compute = None;
                }
                if self.active_live_parakeet
                    == Some(CommandIdentity {
                        job_id: *job_id,
                        generation: *generation,
                    })
                {
                    self.active_live_parakeet = None;
                }
                capture.wav_path = (!wav_path.is_empty()).then(|| wav_path.clone());
                capture.direct_txt_path = txt_path.clone();
                capture.direct_srt_path = srt_path.clone();
                capture.companion_json_path = (!wav_path.is_empty()).then(|| {
                    std::path::Path::new(wav_path)
                        .with_extension("json")
                        .to_string_lossy()
                        .into_owned()
                });
                capture.capture_complete = Some(*complete);
                capture.capture_diagnostic = diagnostic.clone();
                capture.unresolved = unresolved.clone();
                Self::apply_final_segments(capture, identity, final_segments);
                if let Some(automatic) = automatic_final {
                    let final_identity = (automatic.request.job_id, automatic.request.generation);
                    capture.automatic_final_identity = Some(final_identity);
                    capture.requested_compute = Some(automatic.request.config.compute);
                    capture.companion_json_path =
                        Some(automatic.outputs.companion_json_path.clone());
                    capture.automatic_final = Some(crate::ports::AutomaticFinalTranscriptionView {
                        status: crate::ports::AutomaticFinalTranscriptionStatus::Pending,
                        txt_path: automatic.outputs.txt_path.clone(),
                        srt_path: automatic.outputs.srt_path.clone(),
                        model_sha256: automatic.request.model_sha256,
                        requested_compute: automatic.request.config.compute,
                        effective_backend: None,
                        diagnostic: None,
                    });
                } else {
                    capture.automatic_final = Some(crate::ports::AutomaticFinalTranscriptionView {
                        status:
                            crate::ports::AutomaticFinalTranscriptionStatus::FailedOrUnavailable,
                        txt_path: String::new(),
                        srt_path: String::new(),
                        model_sha256: [0; 32],
                        requested_compute: crate::domain::ComputeChoice::Auto,
                        effective_backend: None,
                        diagnostic: Some(
                            "Import automatique indisponible pour cette capture".into(),
                        ),
                    });
                }
                self.trim_parakeet_capture_views();
            }
            _ => unreachable!("capture event group already selected"),
        }
    }

    fn apply_final_segments(
        capture: &mut ParakeetCaptureView,
        identity: ParakeetCaptureIdentity,
        segments: &[crate::ports::WorkerSegment],
    ) {
        for segment in segments {
            if segment.job_id != identity.job_id
                || segment.generation != identity.generation
                || segment.instance_id != identity.session_id
                || segment.range.sample_rate_hz != 16_000
                || segment.range.start_sample >= segment.range.end_sample
                || segment.text.trim().is_empty()
            {
                capture.capture_diagnostic =
                    Some("Segment final invalide ignoré dans la présentation".into());
                continue;
            }
            if let Some(prior) = capture
                .confirmed_segments
                .iter()
                .find(|prior| prior.segment_id == segment.segment_id)
            {
                if prior != segment {
                    capture.capture_diagnostic =
                        Some("Segment final incohérent ignoré dans la présentation".into());
                }
            } else {
                capture.confirmed_segments.push(segment.clone());
            }
        }
        capture.confirmed_segments.sort_by_key(|item| {
            (
                item.range.start_sample,
                item.range.end_sample,
                item.segment_id.0,
            )
        });
        capture.provisional_text = None;
        capture.provisional_range = None;
        capture.provisional_sequence = None;
    }

    fn track_capture_failure(&mut self, event: &ImportEvent) {
        match event {
            ImportEvent::Failed {
                job_id,
                generation,
                message,
            } => {
                let identity = CommandIdentity {
                    job_id: *job_id,
                    generation: *generation,
                };
                let is_parakeet_attempt = self.active_live_parakeet == Some(identity);
                let requested_compute = self.active_live_compute.and_then(|(identity, compute)| {
                    (identity
                        == CommandIdentity {
                            job_id: *job_id,
                            generation: *generation,
                        })
                    .then_some(compute)
                });
                if requested_compute.is_some() {
                    self.active_live_compute = None;
                }
                if is_parakeet_attempt {
                    self.active_live_parakeet = None;
                }
                let existing = self
                    .view
                    .parakeet_captures
                    .iter_mut()
                    .rev()
                    .find(|capture| {
                        capture.identity.job_id == *job_id
                            && capture.identity.generation == *generation
                    });
                if let Some(capture) = existing {
                    if capture.capture_state == ParakeetCaptureViewState::Starting {
                        capture.capture_state = ParakeetCaptureViewState::Failed;
                        capture.capture_diagnostic = Some(message.clone());
                    }
                } else if is_parakeet_attempt && let Some(requested_compute) = requested_compute {
                    self.view.parakeet_captures.push(ParakeetCaptureView {
                        identity: ParakeetCaptureIdentity {
                            job_id: *job_id,
                            generation: *generation,
                            session_id: 0,
                        },
                        wav_path: None,
                        direct_txt_path: None,
                        direct_srt_path: None,
                        companion_json_path: None,
                        confirmed_segments: Vec::new(),
                        provisional_text: None,
                        provisional_range: None,
                        provisional_sequence: None,
                        unresolved: Vec::new(),
                        capture_state: ParakeetCaptureViewState::Failed,
                        capture_complete: None,
                        capture_diagnostic: Some(message.clone()),
                        requested_compute: Some(requested_compute),
                        effective_backend: None,
                        automatic_final: None,
                        automatic_final_identity: None,
                    });
                    self.trim_parakeet_capture_views();
                }
            }
            _ => unreachable!("capture event group already selected"),
        }
    }

    fn track_capture_stopped(&mut self, event: &ImportEvent) {
        match event {
            ImportEvent::Stopped { job_id, generation } => {
                if self.active_live_compute.is_some_and(|(identity, _)| {
                    identity
                        == (CommandIdentity {
                            job_id: *job_id,
                            generation: *generation,
                        })
                }) {
                    self.active_live_compute = None;
                }
                if self.active_live_parakeet
                    == Some(CommandIdentity {
                        job_id: *job_id,
                        generation: *generation,
                    })
                {
                    self.active_live_parakeet = None;
                }
                if let Some(capture) =
                    self.view
                        .parakeet_captures
                        .iter_mut()
                        .rev()
                        .find(|capture| {
                            capture.identity.job_id == *job_id
                                && capture.identity.generation == *generation
                                && capture.capture_state != ParakeetCaptureViewState::Finalized
                        })
                {
                    capture.capture_state = ParakeetCaptureViewState::Failed;
                    capture.capture_diagnostic =
                        Some("Capture arrêtée avant confirmation de la finalisation WAV".into());
                }
            }
            _ => unreachable!("capture event group already selected"),
        }
    }

    fn find_parakeet_capture_mut(
        &mut self,
        job_id: JobId,
        generation: Generation,
        session_id: u64,
    ) -> Option<&mut crate::ports::ParakeetCaptureView> {
        self.view.parakeet_captures.iter_mut().find(|capture| {
            capture.identity.job_id == job_id
                && capture.identity.generation == generation
                && capture.identity.session_id == session_id
        })
    }

    fn trim_parakeet_capture_views(&mut self) {
        while self.view.parakeet_captures.len() > MAX_PARAKEET_CAPTURE_VIEWS {
            let removable = self.view.parakeet_captures.iter().position(|capture| {
                capture.capture_state == crate::ports::ParakeetCaptureViewState::Finalized
                    && capture.automatic_final.as_ref().is_none_or(|final_view| {
                        matches!(
                        final_view.status,
                        crate::ports::AutomaticFinalTranscriptionStatus::ProducedUnverified
                            | crate::ports::AutomaticFinalTranscriptionStatus::FailedOrUnavailable
                    )
                    })
            });
            let Some(index) = removable else { break };
            self.view.parakeet_captures.remove(index);
        }
    }

    pub(super) fn append_parakeet_confirmed(
        &mut self,
        session_id: u64,
        segment: crate::ports::WorkerSegment,
    ) -> Result<(), ApplicationError> {
        if segment.instance_id != session_id
            || segment.segment_id.0 == 0
            || segment.range.sample_rate_hz != 16_000
            || segment.range.start_sample >= segment.range.end_sample
            || segment.text.trim().is_empty()
            || self.parakeet_confirmed.iter().any(|existing| {
                existing.range.start_sample < segment.range.end_sample
                    && segment.range.start_sample < existing.range.end_sample
                    && existing.segment_id != segment.segment_id
            })
        {
            return Err(ApplicationError::InvalidCommand);
        }
        if self
            .parakeet_confirmed
            .iter()
            .any(|existing| existing.segment_id == segment.segment_id)
        {
            return Err(ApplicationError::InvalidCommand);
        }
        self.next_sequence = self.next_sequence.max(
            segment
                .segment_id
                .0
                .checked_add(1)
                .ok_or(ApplicationError::InvalidCommand)?,
        );
        self.parakeet_confirmed.push(segment);
        self.parakeet_confirmed.sort_by_key(|item| {
            (
                item.range.start_sample,
                item.range.end_sample,
                item.segment_id.0,
            )
        });
        self.view.confirmed_text = Some(
            self.parakeet_confirmed
                .iter()
                .map(|item| item.text.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
        );
        self.view.confirmed_samples = self
            .parakeet_confirmed
            .iter()
            .map(|item| item.range.end_sample)
            .max()
            .unwrap_or(0);
        Ok(())
    }
}
