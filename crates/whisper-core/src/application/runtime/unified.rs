use super::super::*;

impl<P: ImportIoPort> ImportApplication<P> {
    pub(in crate::application) fn add_unified_session_card(
        &mut self,
        job_id: JobId,
        generation: Generation,
        destination: String,
        source_path: String,
        microphone_capture: bool,
    ) {
        if self
            .view
            .unified_parakeet_sessions
            .iter()
            .any(|session| (session.job_id, session.generation) == (job_id, generation))
        {
            return;
        }
        if self.view.unified_parakeet_sessions.len() >= MAX_PARAKEET_CAPTURE_VIEWS {
            let Some(index) = self
                .view
                .unified_parakeet_sessions
                .iter()
                .position(|session| {
                    matches!(
                        session.phase,
                        crate::ports::UnifiedSessionPhase::Complete
                            | crate::ports::UnifiedSessionPhase::Recoverable
                    )
                })
            else {
                return;
            };
            let removed = self.view.unified_parakeet_sessions.remove(index);
            if self
                .view
                .selected_unified_session
                .as_ref()
                .is_some_and(|selected| {
                    (selected.job_id, selected.generation) == (removed.job_id, removed.generation)
                })
            {
                self.view.selected_unified_session = None;
            }
        }
        self.view
            .unified_parakeet_sessions
            .push(crate::ports::UnifiedParakeetSessionView {
                job_id,
                generation,
                microphone_capture,
                destination,
                source_path,
                phase: crate::ports::UnifiedSessionPhase::Starting,
                captured_samples: 0,
                durable_samples: 0,
                total_samples: None,
                document_revision: 0,
                audio_parts_revision: 0,
                wav_path: None,
                txt_path: None,
                srt_path: None,
                message: None,
            });
        self.view.selected_unified_session = Some(crate::ports::UnifiedParakeetSelectionView {
            job_id,
            generation,
            document_page: None,
            preview: None,
            audio_parts_page: None,
            document_error: None,
            audio_parts_error: None,
        });
        self.current_unified_document_request = None;
        self.current_unified_audio_request = None;
    }

    pub(in crate::application) fn project_unified_import_event(
        &mut self,
        event: &ImportEvent,
        identity: CommandIdentity,
    ) {
        let CommandIdentity { job_id, generation } = identity;
        if let Some(session) = self
            .view
            .unified_parakeet_sessions
            .iter_mut()
            .find(|session| (session.job_id, session.generation) == (job_id, generation))
        {
            match event {
                ImportEvent::Prepared(request) => {
                    session.total_samples = request.source_samples;
                    session.source_path = request.source_path.clone();
                    session.audio_parts_revision = 0;
                    session.wav_path = Some(
                        std::path::PathBuf::from(&request.destination)
                            .join("transcriptions")
                            .join(format!("{:032x}", request.job_id.0))
                            .join(request.generation.get().to_string())
                            .join("parakeet-unified/source.wav")
                            .to_string_lossy()
                            .into_owned(),
                    );
                    session.phase = crate::ports::UnifiedSessionPhase::Starting;
                }
                ImportEvent::Ready { .. } => {
                    session.phase = crate::ports::UnifiedSessionPhase::Capturing;
                }
                ImportEvent::Progress {
                    completed_samples,
                    total_samples,
                    ..
                } => {
                    session.durable_samples = *completed_samples;
                    session.captured_samples = *completed_samples;
                    session.total_samples = (*total_samples > 0).then_some(*total_samples);
                    session.phase = crate::ports::UnifiedSessionPhase::Capturing;
                }
                ImportEvent::Segment(segment) => {
                    session.document_revision = session
                        .document_revision
                        .max(segment.segment_id.0.saturating_sub(1));
                }
                ImportEvent::Persisted { sequence, .. } => {
                    session.document_revision = session.document_revision.max(*sequence);
                }
                ImportEvent::End { last_offset, .. } => {
                    session.total_samples = Some(*last_offset);
                    session.phase = crate::ports::UnifiedSessionPhase::Publishing;
                }
                ImportEvent::Published { .. } => {
                    session.phase = crate::ports::UnifiedSessionPhase::Complete;
                }
                ImportEvent::Failed { message, .. } => {
                    session.phase = crate::ports::UnifiedSessionPhase::Recoverable;
                    session.message = Some(message.clone());
                }
                ImportEvent::Stopped { .. } => {
                    session.phase = crate::ports::UnifiedSessionPhase::Recoverable;
                    session.message = Some("Session arrêtée avant publication".into());
                }
                _ => {}
            }
        }
        if matches!(event, ImportEvent::Prepared(_)) {
            self.current_unified_audio_request = None;
            if let Some(selected) = self.view.selected_unified_session.as_mut()
                && (selected.job_id, selected.generation) == (job_id, generation)
            {
                selected.audio_parts_error = None;
            }
        }
        if matches!(event, ImportEvent::Persisted { .. }) {
            self.current_unified_document_request = None;
            if let Some(selected) = self.view.selected_unified_session.as_mut()
                && (selected.job_id, selected.generation) == (job_id, generation)
            {
                selected.document_error = None;
            }
        }
    }

    pub(in crate::application) fn handle_unified_capture_event(
        &mut self,
        event: ImportEvent,
    ) -> Result<(), ApplicationError> {
        let key = event.identity();
        match event {
            event @ (ImportEvent::ParakeetLiveStarted { .. }
            | ImportEvent::ParakeetInferenceReady { .. }
            | ImportEvent::Ready { .. }
            | ImportEvent::ParakeetCaptureProgress { .. }) => {
                self.handle_unified_capture_progress(event, key)
            }
            event @ (ImportEvent::ParakeetHypothesis { .. }
            | ImportEvent::ParakeetConfirmed { .. }
            | ImportEvent::ParakeetCaptureStopped { .. }) => {
                self.handle_unified_capture_text(event, key)
            }
            event @ (ImportEvent::MicrophoneGainApplied { .. }
            | ImportEvent::MicrophoneGainRejected { .. }) => {
                self.handle_unified_microphone_event(event, key)
            }
            event @ (ImportEvent::Prepared(_)
            | ImportEvent::Progress { .. }
            | ImportEvent::Segment(_)) => self.handle_unified_prepared_segment(event, key),
            event @ (ImportEvent::Persisted { .. } | ImportEvent::End { .. }) => {
                self.handle_unified_persist_end(event, key)
            }
            event @ ImportEvent::UnifiedDocumentPage { .. } => {
                self.handle_unified_document_page(event, key)
            }
            event @ ImportEvent::UnifiedAudioPartsPage { .. } => {
                self.handle_unified_audio_page(event, key)
            }
            event @ ImportEvent::UnifiedPublicationArtifacts { .. } => {
                self.handle_unified_artifacts(event, key)
            }
            event @ (ImportEvent::Published { .. }
            | ImportEvent::Failed { .. }
            | ImportEvent::Stopped { .. }) => self.handle_unified_terminal(event, key),
            _ => Ok(()),
        }
    }

    fn handle_unified_capture_progress(
        &mut self,
        event: ImportEvent,
        key: (JobId, Generation),
    ) -> Result<(), ApplicationError> {
        match event {
            ImportEvent::ParakeetLiveStarted { .. }
            | ImportEvent::ParakeetInferenceReady { .. } => {
                if let Some(session) = self
                    .view
                    .unified_parakeet_sessions
                    .iter_mut()
                    .find(|s| (s.job_id, s.generation) == key)
                {
                    session.phase = crate::ports::UnifiedSessionPhase::Capturing;
                }
            }
            ImportEvent::Ready { backend, .. } => {
                if let Some(session) = self
                    .view
                    .unified_parakeet_sessions
                    .iter_mut()
                    .find(|s| (s.job_id, s.generation) == key)
                {
                    session.phase = crate::ports::UnifiedSessionPhase::Capturing;
                    session.message = Some(format!("Moteur actif : {backend}"));
                }
            }
            ImportEvent::ParakeetCaptureProgress {
                captured_samples,
                audio_durable_samples,
                ..
            } => {
                self.current_unified_audio_request = None;
                if let Some(session) = self
                    .view
                    .unified_parakeet_sessions
                    .iter_mut()
                    .find(|s| (s.job_id, s.generation) == key)
                {
                    session.captured_samples = session.captured_samples.max(captured_samples);
                    session.durable_samples = session
                        .durable_samples
                        .max(audio_durable_samples)
                        .min(session.captured_samples);
                    session.audio_parts_revision = 0;
                    if matches!(
                        session.phase,
                        crate::ports::UnifiedSessionPhase::Starting
                            | crate::ports::UnifiedSessionPhase::Capturing
                    ) {
                        session.phase = crate::ports::UnifiedSessionPhase::Capturing;
                    }
                }
                if let Some(selected) = self.view.selected_unified_session.as_mut()
                    && (selected.job_id, selected.generation) == key
                {
                    selected.audio_parts_error = None;
                }
            }
            _ => unreachable!("unified event group already selected"),
        }
        Ok(())
    }

    fn handle_unified_capture_text(
        &mut self,
        event: ImportEvent,
        key: (JobId, Generation),
    ) -> Result<(), ApplicationError> {
        match event {
            ImportEvent::ParakeetHypothesis {
                sequence,
                range,
                text,
                ..
            } => {
                if let Some(selected) = self.view.selected_unified_session.as_mut()
                    && (selected.job_id, selected.generation) == key
                {
                    let revision = self
                        .view
                        .unified_parakeet_sessions
                        .iter()
                        .find(|s| (s.job_id, s.generation) == key)
                        .map_or(0, |s| s.document_revision);
                    selected.preview = Some(crate::ports::ParakeetPreview {
                        segment: crate::ports::WorkerSegment {
                            job_id: key.0,
                            generation: key.1,
                            instance_id: 0,
                            segment_id: crate::domain::SegmentId(sequence),
                            range,
                            text,
                        },
                        revision,
                    });
                }
            }
            ImportEvent::ParakeetConfirmed { segment, .. } => {
                self.current_unified_document_request = None;
                if let Some(session) = self
                    .view
                    .unified_parakeet_sessions
                    .iter_mut()
                    .find(|s| (s.job_id, s.generation) == key)
                {
                    session.document_revision = session.document_revision.max(segment.segment_id.0);
                }
                if let Some(selected) = self.view.selected_unified_session.as_mut()
                    && (selected.job_id, selected.generation) == key
                {
                    selected.document_error = None;
                    if selected
                        .preview
                        .as_ref()
                        .is_some_and(|preview| preview.segment.segment_id.0 <= segment.segment_id.0)
                    {
                        selected.preview = None;
                    }
                }
            }
            ImportEvent::ParakeetCaptureStopped { wav_path, .. } => {
                if let Some(session) = self
                    .view
                    .unified_parakeet_sessions
                    .iter_mut()
                    .find(|s| (s.job_id, s.generation) == key)
                {
                    session.wav_path = Some(wav_path);
                    session.phase = crate::ports::UnifiedSessionPhase::Finalizing;
                }
            }
            _ => unreachable!("unified event group already selected"),
        }
        Ok(())
    }

    fn handle_unified_prepared_segment(
        &mut self,
        event: ImportEvent,
        key: (JobId, Generation),
    ) -> Result<(), ApplicationError> {
        match event {
            ImportEvent::Prepared(request) => {
                let capture = self
                    .unified_captures
                    .get_mut(&key)
                    .ok_or(ApplicationError::InvalidCommand)?;
                if !capture.stop_submitted
                    || (request.job_id, request.generation)
                        != (capture.identity.job_id, capture.identity.generation)
                    || request.strategy
                        != crate::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
                    || request.source_sha256.is_none()
                    || request.source_samples.is_none_or(|samples| samples == 0)
                {
                    return Err(ApplicationError::InvalidCommand);
                }
                capture.total_samples = request.source_samples;
                if let Some(session) = self
                    .view
                    .unified_parakeet_sessions
                    .iter_mut()
                    .find(|s| (s.job_id, s.generation) == key)
                {
                    session.total_samples = request.source_samples;
                    session.wav_path = Some(request.source_path);
                    session.phase = crate::ports::UnifiedSessionPhase::Finalizing;
                    session.audio_parts_revision = 0;
                }
                self.current_unified_audio_request = None;
                if let Some(selected) = self.view.selected_unified_session.as_mut()
                    && (selected.job_id, selected.generation) == key
                {
                    selected.audio_parts_error = None;
                }
            }
            ImportEvent::Progress {
                job_id,
                generation,
                completed_samples,
                total_samples,
            } => {
                let capture = self
                    .unified_captures
                    .get(&key)
                    .ok_or(ApplicationError::InvalidCommand)?;
                if (job_id, generation) != (capture.identity.job_id, capture.identity.generation)
                    || capture.total_samples != Some(total_samples)
                    || completed_samples != total_samples
                {
                    return Err(ApplicationError::InvalidCommand);
                }
            }
            ImportEvent::Segment(segment) => {
                let capture = self
                    .unified_captures
                    .get_mut(&key)
                    .ok_or(ApplicationError::InvalidCommand)?;
                if segment.segment_id.0 != capture.next_sequence
                    || (segment.job_id, segment.generation)
                        != (capture.identity.job_id, capture.identity.generation)
                {
                    return Err(ApplicationError::InvalidCommand);
                }
                capture.next_sequence = capture
                    .next_sequence
                    .checked_add(1)
                    .ok_or(ApplicationError::InvalidCommand)?;
                capture.pending_segments.push_back(segment);
                self.retry_unified_capture_segment(key)?;
            }
            _ => unreachable!("unified event group already selected"),
        }
        Ok(())
    }

    fn handle_unified_persist_end(
        &mut self,
        event: ImportEvent,
        key: (JobId, Generation),
    ) -> Result<(), ApplicationError> {
        match event {
            ImportEvent::Persisted { sequence, .. } => {
                let capture = self
                    .unified_captures
                    .get_mut(&key)
                    .ok_or(ApplicationError::InvalidCommand)?;
                if !capture.pending_sequences.remove(&sequence) {
                    return Err(ApplicationError::InvalidCommand);
                }
                capture.pending_persistence = capture.pending_sequences.len();
                if let Some(session) = self
                    .view
                    .unified_parakeet_sessions
                    .iter_mut()
                    .find(|s| (s.job_id, s.generation) == key)
                {
                    session.document_revision = session.document_revision.max(sequence);
                    session.phase = crate::ports::UnifiedSessionPhase::Capturing;
                }
                self.current_unified_document_request = None;
                if let Some(selected) = self.view.selected_unified_session.as_mut()
                    && (selected.job_id, selected.generation) == key
                {
                    selected.document_error = None;
                }
                self.maybe_publish_unified_capture(key)?;
            }
            ImportEvent::End {
                job_id,
                generation,
                last_sequence,
                last_offset,
            } => {
                let capture = self
                    .unified_captures
                    .get_mut(&key)
                    .ok_or(ApplicationError::InvalidCommand)?;
                if !capture.stop_submitted
                    || (capture.identity.job_id, capture.identity.generation)
                        != (job_id, generation)
                    || last_sequence != capture.next_sequence.saturating_sub(1)
                    || last_offset == 0
                    || capture.total_samples != Some(last_offset)
                {
                    return Err(ApplicationError::InvalidCommand);
                }
                capture.last_sequence = Some(last_sequence);
                capture.total_samples = Some(last_offset);
                if let Some(session) = self
                    .view
                    .unified_parakeet_sessions
                    .iter_mut()
                    .find(|s| (s.job_id, s.generation) == key)
                {
                    session.total_samples = Some(last_offset);
                    session.phase = crate::ports::UnifiedSessionPhase::Publishing;
                }
                self.maybe_publish_unified_capture(key)?;
            }
            _ => unreachable!("unified event group already selected"),
        }
        Ok(())
    }

    fn handle_unified_document_page(
        &mut self,
        event: ImportEvent,
        key: (JobId, Generation),
    ) -> Result<(), ApplicationError> {
        match event {
            ImportEvent::UnifiedDocumentPage {
                request_id, result, ..
            } => {
                if self.current_unified_document_request != Some((key, request_id))
                    || !self
                        .view
                        .selected_unified_session
                        .as_ref()
                        .is_some_and(|selected| (selected.job_id, selected.generation) == key)
                {
                    return Ok(());
                }
                self.current_unified_document_request = None;
                match result {
                    Ok(page) => {
                        let current_revision = self
                            .view
                            .unified_parakeet_sessions
                            .iter()
                            .find(|s| (s.job_id, s.generation) == key)
                            .map(|s| s.document_revision);
                        if current_revision == Some(page.revision)
                            && self
                                .view
                                .selected_unified_session
                                .as_ref()
                                .is_some_and(|selected| {
                                    (selected.job_id, selected.generation) == key
                                })
                        {
                            self.view
                                .selected_unified_session
                                .as_mut()
                                .unwrap()
                                .document_page = Some(page);
                            self.view
                                .selected_unified_session
                                .as_mut()
                                .unwrap()
                                .document_error = None;
                        }
                    }
                    Err(message) => {
                        if let Some(selected) = self.view.selected_unified_session.as_mut()
                            && (selected.job_id, selected.generation) == key
                        {
                            selected.document_error = Some(message);
                        }
                    }
                }
            }
            _ => unreachable!("unified event group already selected"),
        }
        Ok(())
    }

    fn handle_unified_audio_page(
        &mut self,
        event: ImportEvent,
        key: (JobId, Generation),
    ) -> Result<(), ApplicationError> {
        match event {
            ImportEvent::UnifiedAudioPartsPage {
                request_id, result, ..
            } => {
                if self.current_unified_audio_request != Some((key, request_id))
                    || !self
                        .view
                        .selected_unified_session
                        .as_ref()
                        .is_some_and(|selected| (selected.job_id, selected.generation) == key)
                {
                    return Ok(());
                }
                self.current_unified_audio_request = None;
                match result {
                    Ok(page) => {
                        let current_revision = self
                            .view
                            .unified_parakeet_sessions
                            .iter_mut()
                            .find(|s| (s.job_id, s.generation) == key);
                        if current_revision.is_some_and(|session| {
                            session.audio_parts_revision == 0
                                || session.audio_parts_revision == page.revision
                        }) && self
                            .view
                            .selected_unified_session
                            .as_ref()
                            .is_some_and(|selected| (selected.job_id, selected.generation) == key)
                        {
                            let session = self
                                .view
                                .unified_parakeet_sessions
                                .iter_mut()
                                .find(|s| (s.job_id, s.generation) == key)
                                .unwrap();
                            session.audio_parts_revision = page.revision;
                            self.view
                                .selected_unified_session
                                .as_mut()
                                .unwrap()
                                .audio_parts_page = Some(page);
                            self.view
                                .selected_unified_session
                                .as_mut()
                                .unwrap()
                                .audio_parts_error = None;
                        }
                    }
                    Err(message) => {
                        if let Some(selected) = self.view.selected_unified_session.as_mut()
                            && (selected.job_id, selected.generation) == key
                        {
                            selected.audio_parts_error = Some(message);
                        }
                    }
                }
            }
            _ => unreachable!("unified event group already selected"),
        }
        Ok(())
    }

    fn handle_unified_artifacts(
        &mut self,
        event: ImportEvent,
        key: (JobId, Generation),
    ) -> Result<(), ApplicationError> {
        match event {
            ImportEvent::UnifiedPublicationArtifacts { result, .. } => {
                if let Some(session) = self
                    .view
                    .unified_parakeet_sessions
                    .iter_mut()
                    .find(|s| (s.job_id, s.generation) == key)
                {
                    match result {
                        Ok(files) => {
                            session.wav_path = Some(files.wav_path);
                            session.txt_path = Some(files.txt_path);
                            session.srt_path = Some(files.srt_path);
                            session.message = None;
                        }
                        Err(message) => session.message = Some(message),
                    }
                }
            }
            _ => unreachable!("unified event group already selected"),
        }
        Ok(())
    }

    fn handle_unified_terminal(
        &mut self,
        event: ImportEvent,
        key: (JobId, Generation),
    ) -> Result<(), ApplicationError> {
        match event {
            ImportEvent::Published { job_id, generation } => {
                if let Some(capture) = self.unified_captures.get(&key) {
                    self.io
                        .submit(ImportEffect::ScanHistory {
                            destination: capture.destination.clone(),
                        })
                        .map_err(|error| self.fail(error))?;
                }
                self.view.message = Some(format!(
                    "Transcription micro publiée ({}:{})",
                    job_id.0,
                    generation.get()
                ));
                if let Some(session) = self
                    .view
                    .unified_parakeet_sessions
                    .iter_mut()
                    .find(|s| (s.job_id, s.generation) == key)
                {
                    session.phase = crate::ports::UnifiedSessionPhase::Complete;
                }
                self.unified_captures.remove(&key);
            }
            ImportEvent::Failed { message, .. } => {
                self.view.message = Some(format!("Échec de la transcription micro : {message}"));
                if let Some(session) = self
                    .view
                    .unified_parakeet_sessions
                    .iter_mut()
                    .find(|s| (s.job_id, s.generation) == key)
                {
                    session.phase = crate::ports::UnifiedSessionPhase::Recoverable;
                    session.message = Some(message);
                }
                self.unified_captures.remove(&key);
            }
            ImportEvent::Stopped { .. } => {
                if let Some(session) = self
                    .view
                    .unified_parakeet_sessions
                    .iter_mut()
                    .find(|s| (s.job_id, s.generation) == key)
                {
                    session.phase = crate::ports::UnifiedSessionPhase::Recoverable;
                    session.message = Some("Session arrêtée avant publication".into());
                }
                self.unified_captures.remove(&key);
            }
            _ => unreachable!("unified event group already selected"),
        }
        Ok(())
    }

    pub(in crate::application) fn retry_pending_unified_capture_segments(
        &mut self,
    ) -> Result<(), ApplicationError> {
        let keys = self.unified_captures.keys().copied().collect::<Vec<_>>();
        for key in keys {
            self.retry_unified_capture_segment(key)?;
        }
        Ok(())
    }

    fn retry_unified_capture_segment(
        &mut self,
        key: (JobId, Generation),
    ) -> Result<(), ApplicationError> {
        loop {
            let Some(segment) = self
                .unified_captures
                .get(&key)
                .and_then(|capture| capture.pending_segments.front().cloned())
            else {
                return Ok(());
            };
            match self
                .io
                .submit(ImportEffect::PersistSegment(segment.clone()))
            {
                Ok(()) => {
                    let capture = self
                        .unified_captures
                        .get_mut(&key)
                        .ok_or(ApplicationError::InvalidCommand)?;
                    capture.pending_segments.pop_front();
                    capture.pending_sequences.insert(segment.segment_id.0);
                    capture.pending_persistence = capture.pending_sequences.len();
                }
                Err(PortError::Backpressure) => return Ok(()),
                Err(error) => return Err(self.fail(error)),
            }
        }
    }

    fn maybe_publish_unified_capture(
        &mut self,
        key: (JobId, Generation),
    ) -> Result<(), ApplicationError> {
        self.retry_unified_capture_segment(key)?;
        let Some(capture) = self.unified_captures.get(&key) else {
            return Ok(());
        };
        let ready = capture.stop_submitted
            && !capture.publish_submitted
            && capture.pending_segments.is_empty()
            && capture.pending_sequences.is_empty()
            && capture
                .last_sequence
                .is_some_and(|last| last.saturating_add(1) == capture.next_sequence);
        if ready {
            let capture = self.unified_captures.get(&key).unwrap();
            self.io
                .submit(ImportEffect::Publish {
                    job_id: capture.identity.job_id,
                    generation: capture.identity.generation,
                    last_sequence: capture.last_sequence.unwrap(),
                })
                .map_err(|error| self.fail(error))?;
            if let Some(capture) = self.unified_captures.get_mut(&key) {
                capture.publish_submitted = true;
            }
        }
        Ok(())
    }

    pub(in crate::application) fn handle_unified_selection_command(
        &mut self,
        command: AppCommand,
    ) -> Result<AppView, ApplicationError> {
        match command {
            command @ AppCommand::SelectUnifiedSession { .. } => {
                self.select_unified_session(command)
            }
            command @ AppCommand::ReadUnifiedDocumentPage { .. } => {
                self.read_unified_document(command)
            }
            command @ AppCommand::ReadUnifiedAudioPartsPage { .. } => {
                self.read_unified_audio(command)
            }
            _ => unreachable!("unified selection command already selected"),
        }
    }

    fn select_unified_session(&mut self, command: AppCommand) -> Result<AppView, ApplicationError> {
        match command {
            AppCommand::SelectUnifiedSession { job_id, generation } => {
                if !self
                    .view
                    .unified_parakeet_sessions
                    .iter()
                    .any(|session| (session.job_id, session.generation) == (job_id, generation))
                {
                    return Err(ApplicationError::InvalidCommand);
                }
                self.current_unified_document_request = None;
                self.current_unified_audio_request = None;
                self.view.selected_unified_session =
                    Some(crate::ports::UnifiedParakeetSelectionView {
                        job_id,
                        generation,
                        document_page: None,
                        preview: None,
                        audio_parts_page: None,
                        document_error: None,
                        audio_parts_error: None,
                    });
            }
            _ => unreachable!("unified command already selected"),
        }
        Ok(self.view.clone())
    }

    fn read_unified_document(&mut self, command: AppCommand) -> Result<AppView, ApplicationError> {
        match command {
            AppCommand::ReadUnifiedDocumentPage {
                job_id,
                generation,
                expected_revision,
                first_sequence,
            } => {
                let session_destination =
                    self.view.unified_parakeet_sessions.iter().find(|session| {
                        (session.job_id, session.generation) == (job_id, generation)
                    });
                if first_sequence == 0
                    || !self
                        .view
                        .selected_unified_session
                        .as_ref()
                        .is_some_and(|selected| {
                            (selected.job_id, selected.generation) == (job_id, generation)
                        })
                    || session_destination
                        .is_none_or(|session| session.document_revision != expected_revision)
                {
                    return Err(ApplicationError::InvalidCommand);
                }
                let request_id = self
                    .next_unified_page_request_id
                    .checked_add(1)
                    .ok_or(ApplicationError::InvalidCommand)?;
                self.io
                    .submit(ImportEffect::ReadUnifiedDocumentPage {
                        job_id,
                        generation,
                        request_id,
                        destination: session_destination.unwrap().destination.clone(),
                        expected_revision,
                        first_sequence,
                        limit: 128,
                    })
                    .map_err(|error| self.fail(error))?;
                self.next_unified_page_request_id = request_id;
                self.current_unified_document_request = Some(((job_id, generation), request_id));
            }
            _ => unreachable!("unified command already selected"),
        }
        Ok(self.view.clone())
    }

    fn read_unified_audio(&mut self, command: AppCommand) -> Result<AppView, ApplicationError> {
        match command {
            AppCommand::ReadUnifiedAudioPartsPage {
                job_id,
                generation,
                expected_revision,
                offset,
            } => {
                let session_destination =
                    self.view.unified_parakeet_sessions.iter().find(|session| {
                        (session.job_id, session.generation) == (job_id, generation)
                    });
                if !self
                    .view
                    .selected_unified_session
                    .as_ref()
                    .is_some_and(|selected| {
                        (selected.job_id, selected.generation) == (job_id, generation)
                    })
                    || session_destination.is_none_or(|session| {
                        expected_revision != 0 && session.audio_parts_revision != expected_revision
                    })
                {
                    return Err(ApplicationError::InvalidCommand);
                }
                let request_id = self
                    .next_unified_page_request_id
                    .checked_add(1)
                    .ok_or(ApplicationError::InvalidCommand)?;
                self.io
                    .submit(ImportEffect::ReadUnifiedAudioPartsPage {
                        job_id,
                        generation,
                        request_id,
                        destination: session_destination.unwrap().destination.clone(),
                        expected_revision,
                        offset,
                        limit: 128,
                    })
                    .map_err(|error| self.fail(error))?;
                self.next_unified_page_request_id = request_id;
                self.current_unified_audio_request = Some(((job_id, generation), request_id));
            }
            _ => unreachable!("unified command already selected"),
        }
        Ok(self.view.clone())
    }

    pub(in crate::application) fn start_unified_live_command(
        &mut self,
        command: AppCommand,
    ) -> Result<AppView, ApplicationError> {
        match command {
            AppCommand::StartUnifiedLiveConfigured { request } => {
                let import_can_share = self.active.is_some()
                    && self.request.as_ref().is_some_and(|active| {
                        active.strategy
                            == crate::parakeet_session::TranscriptionStrategy::BoundedWindowsV1
                            && active.model_path == request.model_path
                            && active.model_sha256 == request.model_sha256
                            && active.config.compute == request.config.compute
                    });
                let open_capture_exists = self
                    .unified_captures
                    .values()
                    .any(|capture| !capture.stop_submitted);
                let admitted_sessions = self.unified_captures.len()
                    + if import_can_share && self.active.is_some() {
                        1
                    } else {
                        0
                    };
                let no_terminal_card_to_evict =
                    !self.view.unified_parakeet_sessions.iter().any(|session| {
                        matches!(
                            session.phase,
                            crate::ports::UnifiedSessionPhase::Complete
                                | crate::ports::UnifiedSessionPhase::Recoverable
                        )
                    });
                if open_capture_exists
                    || admitted_sessions >= 2
                    || (self.view.unified_parakeet_sessions.len() >= MAX_PARAKEET_CAPTURE_VIEWS
                        && no_terminal_card_to_evict)
                    || (self.active.is_some() && !import_can_share)
                    || self
                        .unified_captures
                        .contains_key(&(request.job_id, request.generation))
                    || self.active
                        == Some(CommandIdentity {
                            job_id: request.job_id,
                            generation: request.generation,
                        })
                {
                    return Err(ApplicationError::InvalidCommand);
                }
                let identity = CommandIdentity {
                    job_id: request.job_id,
                    generation: request.generation,
                };
                let destination = request.destination.clone();
                let source_path = std::path::PathBuf::from(&request.destination)
                    .join("transcriptions")
                    .join(format!("{:032x}", request.job_id.0))
                    .join(request.generation.get().to_string())
                    .join("parakeet-unified/source.wav")
                    .to_string_lossy()
                    .into_owned();
                let microphone = request.microphone.clone();
                let microphone_gain_db = request.microphone_gain_db;
                self.io
                    .submit(ImportEffect::StartUnifiedLive(request))
                    .map_err(|error| self.fail(error))?;
                self.view.microphone_source_identity = match microphone {
                    crate::microphone::MicrophoneChoice::Device(identity) => Some(identity),
                    crate::microphone::MicrophoneChoice::WindowsDefault => None,
                };
                self.view.microphone_gain_db = microphone_gain_db;
                self.view.microphone_gain_requested_db = microphone_gain_db;
                self.view.microphone_gain_requested_revision = 0;
                self.view.microphone_gain_revision = 0;
                self.view.microphone_gain_applied_first_frame = None;
                self.microphone_gain_view_owner = Some(identity);
                self.unified_captures.insert(
                    (identity.job_id, identity.generation),
                    UnifiedCaptureAdmission {
                        identity,
                        destination: destination.clone(),
                        stop_submitted: false,
                        publish_submitted: false,
                        microphone_gain_requested_revision: 0,
                        microphone_gain_requested_target_identity: None,
                        next_sequence: 1,
                        pending_persistence: 0,
                        pending_segments: std::collections::VecDeque::new(),
                        pending_sequences: std::collections::BTreeSet::new(),
                        last_sequence: None,
                        total_samples: None,
                    },
                );
                self.add_unified_session_card(
                    identity.job_id,
                    identity.generation,
                    destination,
                    source_path,
                    true,
                );
            }
            _ => unreachable!("unified command already selected"),
        }
        Ok(self.view.clone())
    }

    pub(in crate::application) fn stop_unified_live_command(
        &mut self,
        command: AppCommand,
    ) -> Result<AppView, ApplicationError> {
        match command {
            AppCommand::StopUnifiedLive { job_id, generation } => {
                let key = (job_id, generation);
                let stop_submitted = self
                    .unified_captures
                    .get(&key)
                    .ok_or(ApplicationError::InvalidCommand)?
                    .stop_submitted;
                if !stop_submitted {
                    self.io
                        .submit(ImportEffect::StopUnifiedLive { job_id, generation })
                        .map_err(|error| self.fail(error))?;
                    if let Some(capture) = self.unified_captures.get_mut(&key) {
                        capture.stop_submitted = true;
                    }
                }
            }
            _ => unreachable!("unified command already selected"),
        }
        Ok(self.view.clone())
    }
}
