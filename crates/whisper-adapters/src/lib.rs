pub mod archive;
pub mod capture;
pub mod decoder;
mod diagnostics;
mod direct_live;
pub mod gpu_inventory;
pub mod journal;
pub mod microphone_probe;
pub mod model_catalog;
pub mod parakeet_audio;
pub mod parakeet_document;
pub mod parakeet_import;
mod parakeet_live;
pub mod parakeet_pipeline;
pub mod path_settings;
pub mod queue_store;
pub mod recovery;
pub mod source_binding;
pub mod staging;
pub mod supervisor;
pub mod vad;
#[cfg(windows)]
mod windows_file_identity;
pub mod worker_ipc;

/// Transport boundary supplied by the composition root in a later lot.
pub trait WorkerTransport {
    fn request(
        &mut self,
        command: whisper_core::ipc::WorkerCommand,
    ) -> Result<(), whisper_core::ports::PortError>;
    fn receive(
        &mut self,
    ) -> Result<Option<whisper_core::ipc::WorkerEvent>, whisper_core::ports::PortError>;
}
