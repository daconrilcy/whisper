#[cfg(test)]
#[derive(Debug)]
enum NativeQualificationEventKind {
    PreparedRoute {
        session: whisper_core::parakeet_scheduler::SchedulerSession,
        requested_compute: ComputeChoice,
        compute: ComputeChoice,
        has_candidate: bool,
        has_expected_gpu: bool,
        fallback_reason: Option<String>,
    },
    SourceBindingMap {
        operation: &'static str,
        bindings: Vec<(JobId, u64)>,
    },
    Starting {
        incarnation: u64,
    },
    Spawned {
        incarnation: u64,
        pid: u32,
    },
    Ready {
        incarnation: u64,
        pid: u32,
        backend: String,
        profile: String,
        model_sha256: String,
        runtime_sha256: String,
        build_sha256: String,
        gpu_attested: bool,
    },
    RequestSent {
        incarnation: u64,
        pid: u32,
        session: whisper_core::parakeet_scheduler::SchedulerSession,
        generation: Generation,
        attempt: u64,
        request_id: u64,
        work: NativeQualificationWork,
    },
    Result {
        incarnation: u64,
        pid: u32,
        session: whisper_core::parakeet_scheduler::SchedulerSession,
        generation: Generation,
        attempt: u64,
        request_id: u64,
        work: NativeQualificationWork,
        word_count: usize,
    },
    CaptureAcknowledged {
        session: whisper_core::parakeet_scheduler::SchedulerSession,
        written_samples: u64,
        durable_samples: u64,
    },
    Reaped {
        incarnation: u64,
        pid: u32,
    },
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NativeQualificationWork {
    Final(u64),
    Preview(u64),
}

#[cfg(test)]
#[derive(Debug)]
struct NativeQualificationEvent {
    at: Instant,
    kind: NativeQualificationEventKind,
}

#[cfg(test)]
struct NativeQualificationObserver {
    next_incarnation: AtomicU64,
    events: SyncSender<NativeQualificationEvent>,
    overflowed: std::sync::atomic::AtomicBool,
}

#[cfg(test)]
impl NativeQualificationObserver {
    fn channel() -> (Arc<Self>, Receiver<NativeQualificationEvent>) {
        let (events, receiver) = mpsc::sync_channel(512);
        (
            Arc::new(Self {
                next_incarnation: AtomicU64::new(1),
                events,
                overflowed: std::sync::atomic::AtomicBool::new(false),
            }),
            receiver,
        )
    }

    fn allocate_incarnation(&self) -> Result<u64, String> {
        self.next_incarnation
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .map_err(|_| "NativeHarness: incarnation identity exhausted".to_owned())
    }

    fn record(&self, kind: NativeQualificationEventKind) {
        if self
            .events
            .try_send(NativeQualificationEvent {
                at: Instant::now(),
                kind,
            })
            .is_err()
        {
            self.overflowed.store(true, Ordering::Release);
        }
    }

    fn assert_healthy(&self) {
        assert!(
            !self.overflowed.load(Ordering::Acquire),
            "native qualification observer overflowed its bounded channel"
        );
    }
}

#[cfg(test)]
#[derive(Clone, Debug, Eq, PartialEq)]
struct ControlledChildObservation {
    incarnation: u64,
    event: &'static str,
    job_id: Option<JobId>,
    generation: Option<Generation>,
    attempt: Option<u64>,
    wire_request_id: u64,
    retained_job_ids: usize,
}

#[cfg(test)]
fn observe_controlled_child(
    observer: Option<&Arc<Mutex<Vec<ControlledChildObservation>>>>,
    incarnation: u64,
    event: &'static str,
    job_id: Option<JobId>,
    generation: Option<Generation>,
    attempt: Option<u64>,
    wire_request_id: u64,
    retained_job_ids: usize,
) {
    if let Some(observer) = observer {
        observer
            .lock()
            .expect("controlled child observation mutex")
            .push(ControlledChildObservation {
                incarnation,
                event,
                job_id,
                generation,
                attempt,
                wire_request_id,
                retained_job_ids,
            });
    }
}

#[cfg(test)]
struct ScheduledWindowTranscriber<'a> {
    scheduler: &'a mut whisper_core::parakeet_scheduler::ParakeetScheduler,
    call: whisper_core::parakeet_scheduler::ScheduledCall,
    transcriber: &'a mut dyn crate::parakeet_pipeline::WindowTranscriber,
}

#[cfg(test)]
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
