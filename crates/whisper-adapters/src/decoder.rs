use crate::WorkerTransport;
use whisper_core::{
    ipc::{WorkerCommand, WorkerEvent},
    ports::{DecodeRequest, DecodedPcmBlock, DecoderPort, PortError},
};

/// Decoder requests cross IPC; the adapter itself never opens or decodes audio files.
pub struct DecoderProxy<T> {
    transport: T,
    next_request_id: u64,
}

impl<T> DecoderProxy<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            next_request_id: 1,
        }
    }

    pub fn into_inner(self) -> T {
        self.transport
    }
}

impl<T: WorkerTransport> DecoderPort for DecoderProxy<T> {
    fn decode(&mut self, request: DecodeRequest) -> Result<Vec<DecodedPcmBlock>, PortError> {
        if request.max_samples == 0 {
            return Err(PortError::InvalidInput);
        }
        let request_id = self.next_request_id;
        self.next_request_id = request_id.checked_add(1).ok_or(PortError::Unavailable)?;
        self.transport.request(WorkerCommand::DecodeBlock {
            request_id,
            request: request.clone(),
        })?;
        match self.transport.receive()? {
            Some(WorkerEvent::DecodedBlock {
                request_id: response_id,
                block,
            }) if response_id == request_id => match block {
                Some(block)
                    if block.source == request.source
                        && block.job_id == request.job_id
                        && block.generation == request.generation
                        && range_is_within(block.block.range, request.range)
                        && block.block.samples.len() <= request.max_samples =>
                {
                    Ok(vec![block])
                }
                Some(block)
                    if block.job_id != request.job_id
                        || block.generation != request.generation
                        || block.source != request.source
                        || !range_is_within(block.block.range, request.range) =>
                {
                    Err(PortError::StaleResponse)
                }
                Some(_) => Err(PortError::InvalidInput),
                None => Ok(Vec::new()),
            },
            Some(WorkerEvent::Failed { message, .. }) => Err(PortError::Failed(message)),
            _ => Err(PortError::Unavailable),
        }
    }
}

fn range_is_within(block: whisper_core::SourceRange, request: whisper_core::SourceRange) -> bool {
    block.sample_rate_hz == request.sample_rate_hz
        && block.start_sample >= request.start_sample
        && block.start_sample < block.end_sample
        && block.end_sample <= request.end_sample
}
