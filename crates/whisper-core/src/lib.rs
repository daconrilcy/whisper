#![forbid(unsafe_code)]

pub mod application;
pub mod domain;
pub mod ipc;
pub mod ports;

pub use application::{
    AppCommand, AppFacade, AppView, Application, ApplicationError, CommandIdentity,
};
pub use domain::{Generation, JobConfig, JobId, JobState, SegmentId, SourceRange, TransitionError};
pub use ports::{
    Clock, DecodeRequest, DecodedPcmBlock, DecoderPort, JobRepository, SourceIdentity, WorkerPort,
};
