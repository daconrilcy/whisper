#[cfg(test)]
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
impl std::fmt::Debug for NativeQualificationEventKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PreparedRoute {
                session,
                requested_compute,
                compute,
                has_candidate,
                has_expected_gpu,
                fallback_reason,
            } => f
                .debug_struct("PreparedRoute")
                .field("session", session)
                .field("requested_compute", requested_compute)
                .field("compute", compute)
                .field("has_candidate", has_candidate)
                .field("has_expected_gpu", has_expected_gpu)
                .field("fallback_reason", fallback_reason)
                .finish(),
            Self::SourceBindingMap {
                operation,
                bindings,
            } => f
                .debug_struct("SourceBindingMap")
                .field("operation", operation)
                .field("bindings", bindings)
                .finish(),
            Self::Starting { incarnation } => f
                .debug_struct("Starting")
                .field("incarnation", incarnation)
                .finish(),
            Self::Spawned { incarnation, pid } => f
                .debug_struct("Spawned")
                .field("incarnation", incarnation)
                .field("pid", pid)
                .finish(),
            Self::Ready {
                incarnation,
                pid,
                backend,
                profile,
                model_sha256,
                runtime_sha256,
                build_sha256,
                gpu_attested,
            } => f
                .debug_struct("Ready")
                .field("incarnation", incarnation)
                .field("pid", pid)
                .field("backend", backend)
                .field("profile", profile)
                .field("model_sha256", model_sha256)
                .field("runtime_sha256", runtime_sha256)
                .field("build_sha256", build_sha256)
                .field("gpu_attested", gpu_attested)
                .finish(),
            Self::RequestSent {
                incarnation,
                pid,
                session,
                generation,
                attempt,
                request_id,
                work,
            } => f
                .debug_struct("RequestSent")
                .field("incarnation", incarnation)
                .field("pid", pid)
                .field("session", session)
                .field("generation", generation)
                .field("attempt", attempt)
                .field("request_id", request_id)
                .field("work", work)
                .finish(),
            Self::Result {
                incarnation,
                pid,
                session,
                generation,
                attempt,
                request_id,
                work,
                word_count,
            } => f
                .debug_struct("Result")
                .field("incarnation", incarnation)
                .field("pid", pid)
                .field("session", session)
                .field("generation", generation)
                .field("attempt", attempt)
                .field("request_id", request_id)
                .field("work", work)
                .field("word_count", word_count)
                .finish(),
            Self::CaptureAcknowledged {
                session,
                written_samples,
                durable_samples,
            } => f
                .debug_struct("CaptureAcknowledged")
                .field("session", session)
                .field("written_samples", written_samples)
                .field("durable_samples", durable_samples)
                .finish(),
            Self::Reaped { incarnation, pid } => f
                .debug_struct("Reaped")
                .field("incarnation", incarnation)
                .field("pid", pid)
                .finish(),
        }
    }
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
    observation: ControlledChildObservation,
) {
    if let Some(observer) = observer {
        observer
            .lock()
            .expect("controlled child observation mutex")
            .push(observation);
    }
}
