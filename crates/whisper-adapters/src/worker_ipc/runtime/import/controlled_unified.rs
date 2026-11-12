use super::*;

struct ScheduledWindowTranscriber<'a> {
    scheduler: &'a mut whisper_core::parakeet_scheduler::ParakeetScheduler,
    call: whisper_core::parakeet_scheduler::ScheduledCall,
    transcriber: &'a mut dyn crate::parakeet_pipeline::WindowTranscriber,
}

impl crate::parakeet_pipeline::WindowTranscriber for ScheduledWindowTranscriber<'_> {
    fn instance_id(&self) -> u64 {
        self.transcriber.instance_id()
    }

    fn transcribe(
        &mut self,
        window: whisper_core::parakeet_session::AudioWindow,
        pcm: &[i16],
    ) -> Result<Vec<whisper_core::parakeet_ipc::Word>, String> {
        self.scheduler
            .begin_call(self.call)
            .map_err(|error| format!("SchedulerInvoke: {error:?}"))?;
        self.transcriber.transcribe(window, pcm)
    }
}

struct ControlledImportAudio<'a> {
    finalizer: crate::parakeet_pipeline::ParakeetFinalizer,
    source: &'a crate::parakeet_audio::PcmPartStore,
    document: &'a mut crate::parakeet_document::ParakeetDocumentStore,
    transcriber: &'a mut dyn crate::parakeet_pipeline::WindowTranscriber,
    total_samples: u64,
}

impl ImportRuntime {
    #[cfg(test)]
    pub(in crate::worker_ipc) fn run_controlled_unified_import(
        &mut self,
        request: &ImportRequest,
    ) -> Result<(), String> {
        let mut transcriber = self.controlled_transcriber.take().ok_or_else(|| {
            "ModelNotQualified: controlled worker injection is required".to_owned()
        })?;
        let (source, mut document, session, total_samples) =
            self.prepare_controlled_unified_import(request)?;
        let result = self.process_controlled_unified_import(
            request,
            &source,
            &mut document,
            transcriber.as_mut(),
            session,
            total_samples,
        )?;
        self.publish_controlled_unified_document(request, document, session, result)
    }

    fn prepare_controlled_unified_import(
        &mut self,
        request: &ImportRequest,
    ) -> Result<
        (
            crate::parakeet_audio::PcmPartStore,
            crate::parakeet_document::ParakeetDocumentStore,
            whisper_core::parakeet_scheduler::SchedulerSession,
            u64,
        ),
        String,
    > {
        let work = PathBuf::from(&request.destination)
            .join("transcriptions")
            .join(format!("{:032x}", request.job_id.0))
            .join(request.generation.get().to_string())
            .join("parakeet-unified");
        fs::create_dir_all(&work)
            .map_err(|error| format!("StorageUnavailable: unified workspace: {error}"))?;
        let source_path = work.join("source.wav");
        let mut writer = crate::parakeet_audio::PcmPartWriter::create(&source_path)
            .map_err(|error| format!("AudioSource: {error}"))?;
        crate::parakeet_audio::decode_file_to_parts(&request.source_path, &mut writer)?;
        writer
            .finish()
            .map_err(|error| format!("AudioSource: {error}"))?;
        let source = crate::parakeet_audio::PcmPartStore::open(&source_path);
        let document = crate::parakeet_document::ParakeetDocumentStore::create(
            work.join("document"),
            request.job_id,
            request.generation,
        )
        .map_err(|error| format!("Document: {error:?}"))?;
        let session = whisper_core::parakeet_scheduler::SchedulerSession {
            job_id: request.job_id,
            generation: request.generation,
        };
        self.parakeet_scheduler
            .admit(session)
            .map_err(|error| format!("SchedulerAdmission: {error:?}"))?;
        let total_samples = source
            .total_samples()
            .map_err(|error| format!("AudioSource: {error}"))?;
        Ok((source, document, session, total_samples))
    }

    fn process_controlled_unified_import(
        &mut self,
        request: &ImportRequest,
        source: &crate::parakeet_audio::PcmPartStore,
        document: &mut crate::parakeet_document::ParakeetDocumentStore,
        transcriber: &mut dyn crate::parakeet_pipeline::WindowTranscriber,
        session: whisper_core::parakeet_scheduler::SchedulerSession,
        total_samples: u64,
    ) -> Result<crate::parakeet_pipeline::PipelineResult, String> {
        let mut audio = ControlledImportAudio {
            finalizer: crate::parakeet_pipeline::ParakeetFinalizer::new(
                request.job_id,
                request.generation,
            ),
            source,
            document,
            transcriber,
            total_samples,
        };
        while audio.finalizer.processed_samples() < total_samples {
            let call =
                self.next_controlled_import_call(session, audio.finalizer.next_window_sequence())?;
            self.process_controlled_import_call(call, session, &mut audio)?;
        }
        match audio.finalizer.step(
            &mut &*audio.source,
            audio.document,
            audio.transcriber,
            audio.total_samples,
            true,
        )? {
            crate::parakeet_pipeline::FinalizationStep::Complete(result) => Ok(result),
            _ => Err("SchedulerSource: final document did not complete".into()),
        }
    }

    fn next_controlled_import_call(
        &mut self,
        session: whisper_core::parakeet_scheduler::SchedulerSession,
        sequence: u64,
    ) -> Result<whisper_core::parakeet_scheduler::ScheduledCall, String> {
        if let Err(error) = self.parakeet_scheduler.queue_final(session, sequence) {
            self.parakeet_scheduler
                .abort_session(session)
                .map_err(|cleanup| format!("SchedulerQueue: {error:?}; {cleanup:?}"))?;
            self.parakeet_scheduler
                .confirm_child_exited()
                .map_err(|cleanup| format!("SchedulerQueue: {error:?}; {cleanup:?}"))?;
            return Err(format!("SchedulerQueue: {error:?}"));
        }
        match self.parakeet_scheduler.next_call() {
            Ok(Some(call)) => Ok(call),
            Ok(None) => {
                self.parakeet_scheduler
                    .abort_session(session)
                    .map_err(|error| format!("SchedulerReady: {error:?}"))?;
                self.parakeet_scheduler
                    .confirm_child_exited()
                    .map_err(|error| format!("SchedulerReap: {error:?}"))?;
                Err("SchedulerReady: final range was not selected".into())
            }
            Err(error) => {
                self.parakeet_scheduler
                    .abort_session(session)
                    .map_err(|cleanup| format!("SchedulerSelect: {error:?}; {cleanup:?}"))?;
                self.parakeet_scheduler
                    .confirm_child_exited()
                    .map_err(|cleanup| format!("SchedulerSelect: {error:?}; {cleanup:?}"))?;
                Err(format!("SchedulerSelect: {error:?}"))
            }
        }
    }

    fn process_controlled_import_call(
        &mut self,
        call: whisper_core::parakeet_scheduler::ScheduledCall,
        session: whisper_core::parakeet_scheduler::SchedulerSession,
        audio: &mut ControlledImportAudio<'_>,
    ) -> Result<(), String> {
        let step = {
            let mut scheduled = ScheduledWindowTranscriber {
                scheduler: &mut self.parakeet_scheduler,
                call,
                transcriber: audio.transcriber,
            };
            audio.finalizer.step(
                &mut &*audio.source,
                audio.document,
                &mut scheduled,
                audio.total_samples,
                true,
            )
        };
        match step {
            Ok(crate::parakeet_pipeline::FinalizationStep::WindowProcessed { .. }) => self
                .parakeet_scheduler
                .complete(call)
                .map_err(|error| format!("SchedulerComplete: {error:?}")),
            Ok(crate::parakeet_pipeline::FinalizationStep::Complete(result)) => self
                .fail_controlled_import_call(
                    call,
                    session,
                    format!(
                        "SchedulerSource: completed without a final call ({})",
                        result.windows_completed
                    ),
                ),
            Ok(crate::parakeet_pipeline::FinalizationStep::WaitingForAudio { .. }) => self
                .fail_controlled_import_call(
                    call,
                    session,
                    "SchedulerSource: closed unified audio unexpectedly waits".into(),
                ),
            Err(error) => {
                self.parakeet_scheduler
                    .fail_call(call)
                    .map_err(|scheduler_error| {
                        format!("{error}; SchedulerFailure: {scheduler_error:?}")
                    })?;
                self.parakeet_scheduler
                    .mark_terminal(session)
                    .map_err(|scheduler_error| {
                        format!("{error}; SchedulerTerminal: {scheduler_error:?}")
                    })?;
                let _ = self.parakeet_scheduler.confirm_child_exited();
                Err(error)
            }
        }
    }

    fn fail_controlled_import_call(
        &mut self,
        call: whisper_core::parakeet_scheduler::ScheduledCall,
        session: whisper_core::parakeet_scheduler::SchedulerSession,
        message: String,
    ) -> Result<(), String> {
        self.parakeet_scheduler
            .fail_call(call)
            .map_err(|error| format!("SchedulerComplete: {error:?}"))?;
        self.parakeet_scheduler
            .abort_session(session)
            .map_err(|error| format!("SchedulerTerminal: {error:?}"))?;
        self.parakeet_scheduler
            .confirm_child_exited()
            .map_err(|error| format!("SchedulerReap: {error:?}"))?;
        Err(message)
    }

    fn publish_controlled_unified_document(
        &mut self,
        request: &ImportRequest,
        document: crate::parakeet_document::ParakeetDocumentStore,
        session: whisper_core::parakeet_scheduler::SchedulerSession,
        result: crate::parakeet_pipeline::PipelineResult,
    ) -> Result<(), String> {
        self.parakeet_scheduler
            .mark_terminal(session)
            .map_err(|error| format!("SchedulerTerminal: {error:?}"))?;
        self.parakeet_scheduler
            .confirm_child_exited()
            .map_err(|error| format!("SchedulerReap: {error:?}"))?;
        self.document_drains.push_back(UnifiedDocumentDrain {
            document,
            job_id: request.job_id,
            generation: request.generation,
            revision: result.document_revision,
            total_samples: result.total_samples,
            next_sequence: 1,
            ready_pending: true,
            progress_pending: true,
        });
        Ok(())
    }
}
