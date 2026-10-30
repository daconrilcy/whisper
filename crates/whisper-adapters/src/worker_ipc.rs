mod backends;
mod ipc;
mod parakeet_unified;
mod publication;
mod runtime;
use runtime::ImportRuntime;
#[cfg(test)]
use runtime::{
    FinishedLiveWindow, ImportRequestState, LIVE_CAPTURE_EVENTS_BEFORE_WORKER_POLL,
    LiveCaptureState, LiveRecoveryState, ParakeetStoppedExit, ParakeetVadRun, PendingLiveState,
    PreflightResult, TDT_FRAME_OVERLAP_SAMPLES, acceptance_barrier,
    append_single_pass_parakeet_word_group, append_vad_parakeet_word_group,
    bind_unidentified_startup_failure, campaign_reserve_attempt, collision_free_job_id,
    live_capture_poll_allowed, map_attempt_sequence, parakeet_stopped_exit, parakeet_vad_segments,
    pending_unified_event_identity, queued_request_identity_matches, recovery_stop_counters,
    scan_archive_history, take_ready_live_stop, unix_time_ms,
    vad_word_overlap_within_one_tdt_frame, validate_single_pass_parakeet_words,
    worker_event_identity, worker_event_is_stale,
};
#[cfg(all(test, feature = "recette-hooks"))]
use runtime::{canonical_extended_acceptance_root, take_acceptance_worker_fault};

#[cfg(test)]
use runtime::MicrophoneTestCapture;
#[cfg(test)]
use runtime::{manual_gpu_failure_event, retry_native_gpu_startup_on_cpu};
#[cfg(test)]
use runtime::{resolve_reprocessing_targets, verified_present_publications};
mod service;
mod support;

pub(crate) use backends::gpu_evidence_matches;
use backends::{
    IdleWorker, NativeUnifiedRoute, RuntimeModelCatalog, WORKER_IDLE_RETENTION,
    archive_qualification_for_request, can_retry_native_auto_on_cpu,
    gpu_qualification_candidate_allowed, native_cpu_route, native_unified_route,
    request_is_parakeet, validate_current_queued_gpu_candidate, validate_queued_gpu_preflight,
};
pub use backends::{probe_cuda_worker, qualify_gpu_worker};

#[cfg(test)]
use parakeet_unified::validate_native_window_pcm;
use parakeet_unified::{
    NativeParakeetWindowTranscriber, UnifiedDocumentDrain, UnifiedInferenceCommand,
    UnifiedInferenceExecutor, UnifiedSessionSource, UnifiedSessionState,
    restore_unified_document_prefix,
};

use publication::PublicationGate;
#[cfg(test)]
use publication::{GATE_ACTIVE, GATE_CANCELLED, GATE_COMMITTED};

pub use ipc::ChildWorkerTransport;
use ipc::{EnqueueError, MAX_FRAME_BYTES, serialize_frame};
#[cfg(test)]
use ipc::{
    WorkerCommandPoll, WorkerReaderMessage, read_events, read_frame, take_next_worker_command,
    try_enqueue_bounded, try_enqueue_worker_command, write_frame,
};
pub use service::AsyncImportIo;
#[cfg(test)]
use service::automatic_final::{
    NEXT_AUTO_EXPORT_STAGE, inspect_automatic_final_queue_with, publish_automatic_staged_pair,
};
use service::automatic_final::{
    attach_automatic_final_import, inspect_automatic_final_queue, publish_automatic_final_exports,
    set_automatic_final_manifest_status,
};
use service::{ServiceRuntime, import_effect_identity};
#[cfg(test)]
use service::{effect_event_reservation, service_loop};
#[cfg(feature = "l01-memory-qualification")]
use support::memory_trace;
use support::{
    EVENTS_PER_STAGE, NEXT_IMPORT_JOB_ID, NEXT_INSTANCE, UNIFIED_CHILD_JOB_CAPACITY, decode_sha256,
    format_sha256, write_pcm16_wav,
};

use crate::{
    WorkerTransport,
    archive::{ArchiveStore, LiveArchive},
    capture::{CaptureStream, Pcm16Converter},
    diagnostics::{TimingContextGuard, record_stage_timing},
    staging::{PcmStaging, PcmWriter},
    supervisor::{
        DiagnosticCode, DiagnosticPhase, DiagnosticRecord, DiagnosticSink, ProgressPhase,
        ProgressSupervisor, WorkerPaths,
    },
    vad::VoiceActivity,
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fs, io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, atomic::Ordering},
    time::{Instant, SystemTime, UNIX_EPOCH},
};
#[cfg(test)]
use std::{
    io::{BufReader, Write},
    process::Command,
    sync::{
        atomic::AtomicU64,
        mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError},
    },
    thread,
    time::Duration,
};
#[cfg(test)]
use whisper_core::ImportIoPort;
#[cfg(test)]
use whisper_core::ipc::IpcEnvelope;
use whisper_core::{
    ArchiveHistoryItem, ComputeChoice, DiagnosticNotice, Generation, ImportEffect, ImportEvent,
    JobId,
    ipc::{BackendKind, WorkerCommand, WorkerEvent},
    ports::{
        DecodeRequest, DecoderPort, DuplicateImportChoice, DuplicateImportPrompt, ImportRequest,
        LiveRequest, PortError, PublishedImport, SourceIdentity, TreatmentIdentity, WorkerSegment,
    },
};

#[cfg(test)]
include!("worker_ipc/tests/stopped_ack.rs");

#[cfg(test)]
include!("worker_ipc/tests/runtime_support.rs");

#[cfg(test)]
mod tests;
