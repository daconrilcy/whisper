#![forbid(unsafe_code)]

pub mod application;
pub mod audio;
pub mod compute_policy;
pub mod domain;
pub mod engine_capabilities;
pub mod gpu_inventory;
pub mod gpu_inventory_app;
pub mod gpu_selection;
pub mod ipc;
pub mod microphone;
pub mod model_catalog;
pub mod model_registry;
pub mod model_selection_app;
pub mod parakeet_ipc;
pub mod parakeet_windows;
pub mod path_settings;
pub mod ports;

pub use application::{
    AppCommand, AppFacade, AppView, Application, ApplicationError, CommandIdentity,
    DirectCapturePhase, DirectEnginePhase, ImportApplication,
};
pub use compute_policy::{BackendAttempt, BackendDecision, ComputePolicy, ComputePolicyError};
pub use domain::{
    ComputeChoice, Generation, JobConfig, JobId, JobState, LanguageChoice, SegmentId, SourceRange,
    TransitionError,
};
pub use microphone::{
    MicrophoneChoice, MicrophoneDevice, MicrophonePort, MicrophoneSettings, NullMicrophonePort,
};
pub use ports::{
    ArchiveHistoryItem, Clock, DecodeRequest, DecodedPcmBlock, DecoderPort, DiagnosticNotice,
    DiagnosticPhase, ImportEffect, ImportEvent, ImportIoPort, ImportRequest, JobRepository,
    LiveRecoveryInfo, LiveRequest, SourceIdentity, WorkerPort,
};
