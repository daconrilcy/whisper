pub mod archive;
pub mod decoder;
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
