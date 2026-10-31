use super::super::gpu_evidence_matches;
#[cfg(test)]
use super::super::{
    NativeQualificationEventKind, NativeQualificationObserver, NativeQualificationWork,
};
use super::super::{NativeUnifiedRoute, write_pcm16_wav};
use std::path::Path;
#[cfg(test)]
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::{fs, io, thread, time::Duration};
use whisper_core::ComputeChoice;

pub(in crate::worker_ipc) fn validate_native_window_pcm(
    window: whisper_core::parakeet_session::AudioWindow,
    pcm_len: usize,
) -> Result<u64, String> {
    let native_samples = window
        .native_end_sample
        .checked_sub(window.native_start_sample)
        .ok_or_else(|| "InvalidWindow: native bounds are reversed".to_owned())?;
    let pcm_samples = u64::try_from(pcm_len)
        .map_err(|_| "InvalidWindow: PCM length exceeds the addressable range".to_owned())?;
    if native_samples == 0
        || native_samples != pcm_samples
        || native_samples > whisper_core::parakeet_ipc::PARAKEET_MAX_AUDIO_SAMPLES as u64
        || window.central_start_sample < window.native_start_sample
        || window.central_end_sample > window.native_end_sample
        || window.central_start_sample >= window.central_end_sample
    {
        return Err("InvalidWindow: bounded PCM does not match its RIFF range".into());
    }
    Ok(native_samples)
}

pub(in crate::worker_ipc) struct NativeParakeetWindowTranscriber {
    route: NativeUnifiedRoute,
    process: Option<crate::parakeet_import::ParakeetProcess>,
    identity: Option<whisper_core::parakeet_ipc::WorkerIdentity>,
    scheduler_incarnation: Option<u64>,
    #[cfg(test)]
    harness_observer: Option<Arc<NativeQualificationObserver>>,
    #[cfg(test)]
    harness_incarnation: Option<u64>,
}

impl Drop for NativeParakeetWindowTranscriber {
    fn drop(&mut self) {
        if let Some(mut process) = self.process.take() {
            let _ = process.terminate();
        }
    }
}

impl NativeParakeetWindowTranscriber {
    pub(in crate::worker_ipc) fn new(route: NativeUnifiedRoute) -> Self {
        Self {
            route,
            process: None,
            identity: None,
            scheduler_incarnation: None,
            #[cfg(test)]
            harness_observer: None,
            #[cfg(test)]
            harness_incarnation: None,
        }
    }

    #[cfg(test)]
    pub(in crate::worker_ipc) fn with_harness_observer(
        mut self,
        observer: Arc<NativeQualificationObserver>,
    ) -> Self {
        self.harness_observer = Some(observer);
        self
    }

    pub(in crate::worker_ipc) fn model_sha256() -> &'static str {
        whisper_core::model_registry::model(whisper_core::model_catalog::ModelId::ParakeetTdt06BV3)
            .artifact
            .sha256
    }

    pub(in crate::worker_ipc) fn verify_ready(
        &self,
        process: &mut crate::parakeet_import::ParakeetProcess,
        identity: whisper_core::parakeet_ipc::WorkerIdentity,
        backend: String,
        gpu_evidence: Option<whisper_core::parakeet_ipc::GpuEvidence>,
    ) -> Result<(), String> {
        let profile = whisper_core::parakeet_ipc::PARAKEET_PROFILE_IMPORT_SINGLE_PASS;
        let key = match self.route.compute {
            ComputeChoice::Cpu => self.route.paths.qualification_key(profile)?,
            ComputeChoice::Gpu => self.route.paths.gpu_qualification_key(profile)?,
            ComputeChoice::Auto => {
                return Err("NativeRoute: Auto must be resolved before READY".into());
            }
        };
        self.route.paths.verify_qualification(
            Self::model_sha256(),
            &key.runtime_build,
            &key.worker_build,
        )?;
        if identity.profile != profile
            || !identity
                .model_sha256
                .eq_ignore_ascii_case(Self::model_sha256())
            || !identity
                .runtime_sha256
                .eq_ignore_ascii_case(&key.runtime_build)
            || !identity
                .build_sha256
                .eq_ignore_ascii_case(&key.worker_build)
        {
            return Err("NativeReadyIdentityMismatch: model/runtime/worker/profile differs".into());
        }
        match self.route.compute {
            ComputeChoice::Cpu if backend != "CPU" => {
                Err(format!("BackendMismatch: expected CPU, received {backend}"))
            }
            ComputeChoice::Gpu => {
                let expected = self.route.expected_gpu.as_ref().ok_or_else(|| {
                    "GpuIdentityMissing: candidate identity was not captured before startup"
                        .to_owned()
                })?;
                if backend != "CUDA0"
                    || gpu_evidence.as_ref().is_none_or(|evidence| {
                        !gpu_evidence_matches(Some(evidence), Some(expected), process.pid())
                    })
                {
                    return Err(
                        "GpuAttestationMismatch: READY did not attest the selected GPU and process"
                            .into(),
                    );
                }
                Ok(())
            }
            ComputeChoice::Auto => Err("NativeRoute: unresolved Auto backend".into()),
            ComputeChoice::Cpu => Ok(()),
        }
    }
}

impl crate::parakeet_pipeline::WindowTranscriber for NativeParakeetWindowTranscriber {
    fn instance_id(&self) -> u64 {
        self.process
            .as_ref()
            .map_or(0, |process| u64::from(process.pid()))
    }

    fn initialize_instance(
        &mut self,
        stop_requested: &std::sync::atomic::AtomicBool,
    ) -> Result<(), String> {
        if !self.route.paths.is_personal_runtime() {
            return Err("NativeCandidateUnavailable: personal runtime layout required".into());
        }
        #[cfg(test)]
        let incarnation = self
            .harness_observer
            .as_ref()
            .map(|observer| observer.allocate_incarnation())
            .transpose()?;
        #[cfg(test)]
        if let (Some(observer), Some(incarnation)) = (self.harness_observer.as_ref(), incarnation) {
            observer.record(NativeQualificationEventKind::Starting { incarnation });
        }
        let log_root = self.route.paths.runtime_root.join("logs");
        fs::create_dir_all(&log_root).map_err(|error| format!("NativeLogDirectory: {error}"))?;
        let mut process =
            crate::parakeet_import::ParakeetProcess::spawn_live(&self.route.paths, &log_root)?;
        #[cfg(test)]
        if let (Some(observer), Some(incarnation)) = (self.harness_observer.as_ref(), incarnation) {
            observer.record(NativeQualificationEventKind::Spawned {
                incarnation,
                pid: process.pid(),
            });
        }
        let ready = (|| loop {
            if stop_requested.load(Ordering::Acquire) {
                return Err("NativeWorkerStartCancelled".into());
            }
            match process.poll()? {
                Some(whisper_core::parakeet_ipc::Event::Ready {
                    identity,
                    backend,
                    gpu_evidence,
                    ..
                }) => {
                    #[cfg(test)]
                    let ready_backend = backend.clone();
                    self.verify_ready(&mut process, identity.clone(), backend, gpu_evidence)?;
                    process.accept_ready(identity.clone())?;
                    #[cfg(test)]
                    if let (Some(observer), Some(incarnation)) =
                        (self.harness_observer.as_ref(), incarnation)
                    {
                        observer.record(NativeQualificationEventKind::Ready {
                            incarnation,
                            pid: process.pid(),
                            backend: ready_backend,
                            profile: identity.profile.clone(),
                            model_sha256: identity.model_sha256.clone(),
                            runtime_sha256: identity.runtime_sha256.clone(),
                            build_sha256: identity.build_sha256.clone(),
                            gpu_attested: self.route.compute == ComputeChoice::Gpu,
                        });
                    }
                    return Ok(identity);
                }
                Some(whisper_core::parakeet_ipc::Event::Failed { message, .. }) => {
                    return Err(format!("NativeWorkerReadyFailed: {message}"));
                }
                Some(event) => {
                    return Err(format!("NativeWorkerUnexpectedBeforeReady: {event:?}"));
                }
                None => thread::sleep(Duration::from_millis(5)),
            }
        })();
        match ready {
            Ok(identity) => {
                self.identity = Some(identity);
                self.process = Some(process);
                #[cfg(test)]
                {
                    self.harness_incarnation = incarnation;
                }
                Ok(())
            }
            Err(error) => {
                #[cfg(test)]
                let pid = process.pid();
                let reap = process.terminate();
                match reap {
                    Ok(_) => {
                        #[cfg(test)]
                        if let (Some(observer), Some(incarnation)) =
                            (self.harness_observer.as_ref(), incarnation)
                        {
                            observer
                                .record(NativeQualificationEventKind::Reaped { incarnation, pid });
                        }
                        Err(error)
                    }
                    Err(reap_error) => Err(format!("{error}; NativeWorkerReap: {reap_error}")),
                }
            }
        }
    }

    fn stop_instance(&mut self) -> Result<(), String> {
        self.scheduler_incarnation = None;
        self.identity = None;
        #[cfg(test)]
        let incarnation = self.harness_incarnation.take();
        if let Some(mut process) = self.process.take() {
            #[cfg(test)]
            let pid = process.pid();
            process
                .terminate()
                .map(|_| {
                    #[cfg(test)]
                    if let (Some(observer), Some(incarnation)) =
                        (self.harness_observer.as_ref(), incarnation)
                    {
                        observer.record(NativeQualificationEventKind::Reaped { incarnation, pid });
                    }
                })
                .map_err(|error| format!("NativeWorkerReap: {error}"))
        } else {
            Ok(())
        }
    }

    fn transcribe(
        &mut self,
        _window: whisper_core::parakeet_session::AudioWindow,
        _pcm: &[i16],
    ) -> Result<Vec<whisper_core::parakeet_ipc::Word>, String> {
        Err("NativeContextMissing: scheduled session identity is required".into())
    }

    fn transcribe_scheduled(
        &mut self,
        call: whisper_core::parakeet_scheduler::ScheduledCall,
        scratch_directory: &Path,
        window: whisper_core::parakeet_session::AudioWindow,
        pcm: &[i16],
    ) -> Result<Vec<whisper_core::parakeet_ipc::Word>, String> {
        if let Some(incarnation) = self.scheduler_incarnation {
            if incarnation != call.incarnation {
                return Err(
                    "SchedulerIncarnationMismatch: call belongs to a recycled executor".into(),
                );
            }
        } else {
            self.scheduler_incarnation = Some(call.incarnation);
        }
        let expected_samples = validate_native_window_pcm(window, pcm.len())?;
        let expected_instance = self
            .process
            .as_ref()
            .map(|process| u64::from(process.pid()))
            .ok_or_else(|| "NativeWorkerUnavailable: process is not READY".to_owned())?;
        let process = self
            .process
            .as_mut()
            .ok_or_else(|| "NativeWorkerUnavailable: process is not READY".to_owned())?;
        let identity = self
            .identity
            .as_ref()
            .ok_or_else(|| "NativeWorkerUnavailable: identity is not READY".to_owned())?;
        fs::create_dir_all(scratch_directory)
            .map_err(|error| format!("NativeWindowDirectory: {error}"))?;
        let suffix = match call.work {
            whisper_core::parakeet_scheduler::ScheduledWork::Final { sequence }
            | whisper_core::parakeet_scheduler::ScheduledWork::Preview { sequence } => sequence,
        };
        let source_path = scratch_directory.join(format!(
            "window-{:032x}-{}-{}-{}-{suffix}.wav",
            call.session.job_id.0,
            call.session.generation.get(),
            call.incarnation,
            call.request_id
        ));
        let operation = (|| {
            write_pcm16_wav(&source_path, pcm)?;
            let source_sha256 = crate::parakeet_import::sha256_file(&source_path)?;
            let job_id = format!("{:032x}", call.session.job_id.0);
            let attempt = call
                .attempt
                .checked_add(1)
                .ok_or_else(|| "SessionAttemptIdentityExhausted".to_owned())?;
            let wire_request_id = process.transcribe_with_gpu_correlated(
                job_id.clone(),
                call.session.generation.get(),
                attempt,
                &source_path,
                source_sha256.clone(),
                self.route.expected_gpu.clone(),
            )?;
            #[cfg(test)]
            if let (Some(observer), Some(incarnation)) =
                (self.harness_observer.as_ref(), self.harness_incarnation)
            {
                let work = match call.work {
                    whisper_core::parakeet_scheduler::ScheduledWork::Final { sequence } => {
                        NativeQualificationWork::Final(sequence)
                    }
                    whisper_core::parakeet_scheduler::ScheduledWork::Preview { sequence } => {
                        NativeQualificationWork::Preview(sequence)
                    }
                };
                observer.record(NativeQualificationEventKind::RequestSent {
                    incarnation,
                    pid: process.pid(),
                    session: call.session,
                    generation: call.session.generation,
                    attempt,
                    request_id: wire_request_id,
                    work,
                });
            }
            let native_marker_required = self.route.compute == ComputeChoice::Gpu;
            let mut native_call_started = false;
            loop {
                match process.poll()? {
                    Some(whisper_core::parakeet_ipc::Event::NativeCallStarted {
                        request_id,
                        job_id: actual_job,
                        generation,
                        attempt: actual_attempt,
                        pid,
                    }) if request_id == wire_request_id
                        && actual_job == job_id
                        && generation == call.session.generation.get()
                        && actual_attempt == attempt
                        && u64::from(pid) == expected_instance =>
                    {
                        native_call_started = true;
                    }
                    Some(whisper_core::parakeet_ipc::Event::Result {
                        request_id,
                        job_id: actual_job,
                        generation,
                        attempt: actual_attempt,
                        identity: actual_identity,
                        source_sha256: actual_sha,
                        duration_samples,
                        words,
                        gpu_evidence,
                        ..
                    }) if request_id == wire_request_id
                        && actual_job == job_id
                        && generation == call.session.generation.get()
                        && actual_attempt == attempt
                        && actual_identity == *identity
                        && actual_sha.eq_ignore_ascii_case(&source_sha256)
                        && duration_samples == expected_samples
                        && (!native_marker_required || native_call_started)
                        && (self.route.compute != ComputeChoice::Gpu
                            || gpu_evidence.as_ref().is_some_and(|evidence| {
                                self.route.expected_gpu.as_ref().is_some_and(|expected| {
                                    gpu_evidence_matches(
                                        Some(evidence),
                                        Some(expected),
                                        process.pid(),
                                    )
                                })
                            })) =>
                    {
                        #[cfg(test)]
                        if let (Some(observer), Some(incarnation)) =
                            (self.harness_observer.as_ref(), self.harness_incarnation)
                        {
                            observer.record(NativeQualificationEventKind::Result {
                                incarnation,
                                pid: process.pid(),
                                session: call.session,
                                generation: call.session.generation,
                                attempt,
                                request_id: wire_request_id,
                                work: match call.work {
                                    whisper_core::parakeet_scheduler::ScheduledWork::Final {
                                        sequence,
                                    } => NativeQualificationWork::Final(sequence),
                                    whisper_core::parakeet_scheduler::ScheduledWork::Preview {
                                        sequence,
                                    } => NativeQualificationWork::Preview(sequence),
                                },
                                word_count: words.len(),
                            });
                        }
                        return Ok(words);
                    }
                    Some(whisper_core::parakeet_ipc::Event::Failed {
                        request_id: Some(request_id),
                        job_id: Some(actual_job),
                        generation: Some(generation),
                        attempt: Some(actual_attempt),
                        identity: Some(actual_identity),
                        message,
                        ..
                    }) if request_id == wire_request_id
                        && actual_job == job_id
                        && generation == call.session.generation.get()
                        && actual_attempt == attempt
                        && actual_identity == *identity =>
                    {
                        return Err(message);
                    }
                    Some(event) => return Err(format!("NativeResponseMismatch: {event:?}")),
                    None => thread::sleep(Duration::from_millis(5)),
                }
            }
        })();
        let remove_result = fs::remove_file(&source_path);
        match (operation, remove_result) {
            (Ok(words), Ok(())) => Ok(words),
            (Ok(words), Err(error)) if error.kind() == io::ErrorKind::NotFound => Ok(words),
            (Err(error), _) => Err(error),
            (Ok(_), Err(error)) => Err(format!("NativeWindowCleanup: {error}")),
        }
    }
}
