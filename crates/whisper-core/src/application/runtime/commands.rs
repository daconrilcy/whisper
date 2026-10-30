use super::super::*;

impl<P: ImportIoPort> ImportApplication<P> {
    fn refuse_model_command(&mut self, reason: &str) -> AppView {
        self.view.message = Some(format!("Modèle indisponible : {reason}"));
        self.view.clone()
    }

    fn dispatch_admitted(
        &mut self,
        command: AppCommand,
        admitted_model: Option<crate::model_catalog::ModelId>,
    ) -> Result<AppView, ApplicationError> {
        match command {
            AppCommand::Refresh => {
                let identity = self.active;
                record_application_timing(
                    "refresh",
                    identity.map(|identity| identity.job_id),
                    identity.map(|identity| identity.generation),
                    None,
                );
                self.poll_once()?;
            }
            command @ (AppCommand::SelectUnifiedSession { .. }
            | AppCommand::ReadUnifiedDocumentPage { .. }
            | AppCommand::ReadUnifiedAudioPartsPage { .. }) => {
                return self.handle_unified_selection_command(command);
            }
            AppCommand::RequestClose => return self.request_close(),
            command @ (AppCommand::StartMicrophoneTest { .. }
            | AppCommand::StopMicrophoneTest { .. }) => {
                return self.handle_microphone_test_command(command);
            }
            command @ (AppCommand::ScanHistory { .. }
            | AppCommand::InspectSourceSnapshotRecovery
            | AppCommand::ConfirmSourceSnapshotRecovery
            | AppCommand::CancelSourceSnapshotRecovery
            | AppCommand::ResolveLivePublication { .. }
            | AppCommand::RemoveQueued { .. }) => return self.handle_queue_command(command),
            command @ AppCommand::StartUnifiedLiveConfigured { .. } => {
                return self.start_unified_live_command(command);
            }
            command @ (AppCommand::StartLiveConfigured { .. }
            | AppCommand::StartDirectLiveConfigured { .. }) => {
                return self.start_live_command(command, admitted_model);
            }
            command @ AppCommand::SetMicrophoneGain { .. } => {
                return self.handle_microphone_gain_command(command);
            }
            command @ AppCommand::FinishLiveOnCpu { .. } => {
                return self.finish_live_cpu_command(command);
            }
            command @ AppCommand::StartImport { .. } => {
                return self.handle_start_import_command(command);
            }
            command @ (AppCommand::EnqueueImport { .. }
            | AppCommand::ResolveDuplicateImport { .. }
            | AppCommand::CancelDuplicateImport
            | AppCommand::ResolveImportPublication { .. }
            | AppCommand::ResumeReplacementCleanup { .. }) => {
                return self.handle_queue_command(command);
            }
            command @ (AppCommand::ProcessQueued { .. } | AppCommand::ResumeInterrupted { .. }) => {
                return self.handle_resume_import_command(command);
            }
            command @ AppCommand::StopUnifiedLive { .. } => {
                return self.stop_unified_live_command(command);
            }
            AppCommand::Stop { job_id, generation } | AppCommand::Quit { job_id, generation } => {
                return self.stop_or_quit(job_id, generation);
            }
            _ => return Err(ApplicationError::InvalidCommand),
        }
        Ok(self.view.clone())
    }
}

impl<P: ImportIoPort> Application for ImportApplication<P> {
    fn set_gpu_candidate(
        &mut self,
        job_id: JobId,
        generation: Generation,
        candidate: Option<crate::gpu_inventory::GpuRuntimeIdentity>,
    ) -> Result<(), ApplicationError> {
        self.io
            .submit(ImportEffect::SetGpuCandidate {
                job_id,
                generation,
                candidate,
            })
            .map_err(|error| {
                self.view.message =
                    Some(format!("Sélection GPU non transmise au worker : {error:?}"));
                ApplicationError::PortUnavailable
            })
    }

    fn dispatch(&mut self, command: AppCommand) -> Result<AppView, ApplicationError> {
        let allowed_while_closing = matches!(
            &command,
            AppCommand::Refresh
                | AppCommand::RequestClose
                | AppCommand::SelectUnifiedSession { .. }
                | AppCommand::ReadUnifiedDocumentPage { .. }
                | AppCommand::ReadUnifiedAudioPartsPage { .. }
                | AppCommand::Stop { .. }
                | AppCommand::StopUnifiedLive { .. }
                | AppCommand::StopMicrophoneTest { .. }
                | AppCommand::Quit { .. }
                | AppCommand::RemoveQueued { .. }
                | AppCommand::CancelDuplicateImport
                | AppCommand::CancelSourceSnapshotRecovery
        );
        if self.view.close_requested && !allowed_while_closing {
            return Err(ApplicationError::InvalidCommand);
        }
        let admission = self.model_guard.as_ref().and_then(|guard| match &command {
            AppCommand::StartImport { request }
            | AppCommand::EnqueueImport { request }
            | AppCommand::ProcessQueued { request }
            | AppCommand::ResumeInterrupted { request } => {
                Some(guard.admit_import(request, crate::model_selection_app::InferencePath::Import))
            }
            AppCommand::StartLiveConfigured { request }
            | AppCommand::StartUnifiedLiveConfigured { request }
            | AppCommand::StartDirectLiveConfigured { request } => Some(guard.admit_live(request)),
            _ => None,
        });
        let admitted_model = admission
            .as_ref()
            .and_then(|result| result.as_ref().ok())
            .map(|selection| selection.admission.model);
        if let Some(Err(error)) = admission {
            let reason = match error {
                crate::model_selection_app::SelectionRefusal::UnknownHistoricalModelPath => {
                    "chemin ou empreinte non reconnu dans le catalogue"
                }
                crate::model_selection_app::SelectionRefusal::Capability(_) => {
                    "combinaison modèle, calcul ou langue non admissible"
                }
            };
            return Ok(self.refuse_model_command(reason));
        }

        self.dispatch_admitted(command, admitted_model)
    }

    fn view(&self) -> AppView {
        self.view.clone()
    }
}
