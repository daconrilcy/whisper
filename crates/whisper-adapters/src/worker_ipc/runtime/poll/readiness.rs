#![warn(clippy::too_many_lines, clippy::cognitive_complexity)]

use super::*;

pub(super) type PollResult = Result<ImportEvent, (JobId, Generation, String)>;

pub(super) enum AdmissionAckControl {
    Accepted,
    Rejected(String),
}

pub(super) enum AttestationControl {
    Accepted,
    Mismatch,
}

pub(super) struct GpuSessionAttestation {
    pub(super) candidate: String,
    pub(super) dxgi_rank: u32,
    pub(super) luid: u64,
    pub(super) pci_id: String,
    pub(super) cuda_ordinal: u32,
    pub(super) ggml_ordinal: u32,
}

pub(super) enum ReadyControl {
    ReturnImmediately(PollResult),
    Finish(PollResult),
}

impl ImportRuntime {
    pub(super) fn poll_import_started_ack(
        &mut self,
        job_id: JobId,
        generation: Generation,
        instance_id: u64,
    ) -> AdmissionAckControl {
        let Some(active) = self.active.as_mut().filter(|active| {
            active.job_id == job_id
                && active.generation == generation
                && active.instance_id == instance_id
                && active.backend == BackendKind::Cuda
                && active.import_request.as_ref().is_some_and(|request| {
                    request.config.compute == whisper_core::ComputeChoice::Auto
                })
                && !active.startup_ready
        }) else {
            return AdmissionAckControl::Rejected(
                "ProtocolMismatch: unexpected Auto import admission acknowledgement".into(),
            );
        };
        active.startup_ready = true;
        AdmissionAckControl::Accepted
    }

    pub(super) fn poll_gpu_session_attestation(
        &mut self,
        attestation: GpuSessionAttestation,
    ) -> AttestationControl {
        let matches_candidate = self.active.as_ref().is_some_and(|active| {
            active.gpu_candidate.as_ref().is_some_and(|expected| {
                expected.pnp_instance_id == attestation.candidate
                    && expected.dxgi_rank == attestation.dxgi_rank
                    && expected.luid == attestation.luid
                    && expected.pci_id.eq_ignore_ascii_case(&attestation.pci_id)
                    && expected.cuda_ordinal == attestation.cuda_ordinal
                    && expected.ggml_ordinal == attestation.ggml_ordinal
            })
        });
        if matches_candidate {
            if let Some(active) = self.active.as_mut() {
                active.gpu_attested = true;
            }
            AttestationControl::Accepted
        } else {
            AttestationControl::Mismatch
        }
    }

    pub(super) fn poll_ready_backend(
        &mut self,
        job_id: JobId,
        generation: Generation,
        instance_id: u64,
        backend: BackendKind,
    ) -> ReadyControl {
        if backend == BackendKind::Cuda
            && self
                .active
                .as_ref()
                .is_some_and(|active| active.gpu_candidate.is_some() && !active.gpu_attested)
        {
            return ReadyControl::ReturnImmediately(self.handle_worker_failure(
                job_id,
                generation,
                whisper_core::ipc::WorkerErrorCode::BackendMismatch,
                "BackendMismatch: GPU Ready arrived before candidate attestation".into(),
            ));
        }
        // The generic Turbo GPU worker pauses at Ready and requires a
        // BeginImport command. Parakeet's separate IPC has no such
        // command: poll_parakeet_worker already sent Transcribe with
        // the correlated attempt before returning this Ready event.
        let auto_gpu_import = backend == BackendKind::Cuda
            && self.parakeet.is_none()
            && self
                .active
                .as_ref()
                .and_then(|active| active.import_request.as_ref())
                .is_some_and(|request| request.config.compute == whisper_core::ComputeChoice::Auto);
        if !auto_gpu_import && let Some(active) = self.active.as_mut() {
            active.startup_ready = true;
        }
        if auto_gpu_import && !self.publication_gate.is_cancelled() {
            let begin = WorkerCommand::BeginImport {
                job_id,
                generation,
                instance_id,
            };
            if let Err(error) = self
                .transport
                .as_mut()
                .expect("ready worker")
                .request(begin)
            {
                return ReadyControl::ReturnImmediately(self.handle_worker_failure(
                    job_id,
                    generation,
                    whisper_core::ipc::WorkerErrorCode::WorkerExited,
                    format!("WorkerExited before BeginImport admission: {error:?}"),
                ));
            }
        }
        if let Some(live) = self
            .live
            .as_mut()
            .filter(|live| live.request.job_id == job_id && live.request.generation == generation)
        {
            live.worker_ready = true;
        }
        if self.pending_live.is_some()
            && !self.publication_gate.is_cancelled()
            && let Err(failure) = self.activate_live_after_ready()
        {
            if let Some(transport) = self.transport.as_mut() {
                let _ = transport.try_request(WorkerCommand::Stop { job_id, generation });
            }
            ReadyControl::Finish(Err(failure))
        } else {
            ReadyControl::Finish(Ok(ImportEvent::Ready {
                job_id,
                generation,
                backend: match backend {
                    BackendKind::Cpu => "CPU",
                    BackendKind::Cuda => "CUDA",
                }
                .to_owned()
                    + &self
                        .auto_fallback_reason
                        .as_ref()
                        .filter(|_| backend == BackendKind::Cpu)
                        .map(|reason| {
                            format!(" — GPU non utilisable ({reason}) ; CPU prend le relais")
                        })
                        .unwrap_or_default(),
            }))
        }
    }
}
