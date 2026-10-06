use crate::WorkerTransport;
use std::{thread, time::Duration};
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
        if request.max_samples == 0
            || request.max_samples > 80_000
            || request.range.start_sample >= request.range.end_sample
            || request.range.sample_rate_hz != 16_000
        {
            return Err(PortError::InvalidInput);
        }
        let request_id = self.next_request_id;
        self.next_request_id = request_id.checked_add(1).ok_or(PortError::Unavailable)?;
        self.transport.request(WorkerCommand::DecodeBlock {
            request_id,
            request: request.clone(),
        })?;
        let response = loop {
            if let Some(response) = self.transport.receive()? {
                break response;
            }
            thread::sleep(Duration::from_millis(5));
        };
        match Some(response) {
            Some(WorkerEvent::DecodedBlock {
                request_id: response_id,
                block,
            }) if response_id == request_id => match block {
                Some(block)
                    if block.source == request.source
                        && block.job_id == request.job_id
                        && block.generation == request.generation
                        && range_is_within(
                            block.block.range,
                            request.range,
                            request.max_samples,
                        )
                        && block.block.samples.len() <= request.max_samples
                        && block.block.channels == 1
                        && block.block.samples.len() as u64
                            == block.block.range.end_sample - block.block.range.start_sample =>
                {
                    Ok(vec![block])
                }
                Some(block)
                    if block.job_id != request.job_id
                        || block.generation != request.generation
                        || block.source != request.source
                        || !range_is_within(
                            block.block.range,
                            request.range,
                            request.max_samples,
                        ) =>
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

fn range_is_within(
    block: whisper_core::SourceRange,
    request: whisper_core::SourceRange,
    max_samples: usize,
) -> bool {
    block.sample_rate_hz == request.sample_rate_hz
        && block.start_sample == request.start_sample
        && block.start_sample < block.end_sample
        && block.end_sample
            <= request
                .end_sample
                .min(request.start_sample.saturating_add(max_samples as u64))
}

#[cfg(test)]
mod tests {
    use super::*;
    use whisper_core::{
        Generation, JobId, SourceRange,
        ipc::WorkerEvent,
        ports::{DecodedPcmBlock, PcmBlock, SourceIdentity},
    };

    struct Stub(Option<WorkerEvent>);
    impl WorkerTransport for Stub {
        fn request(&mut self, _: WorkerCommand) -> Result<(), PortError> {
            Ok(())
        }
        fn receive(&mut self) -> Result<Option<WorkerEvent>, PortError> {
            Ok(self.0.take())
        }
    }

    fn valid_response(samples: usize) -> WorkerEvent {
        let range = SourceRange::new(0, samples as u64, 16_000).unwrap();
        WorkerEvent::DecodedBlock {
            request_id: 1,
            block: Some(DecodedPcmBlock {
                source: SourceIdentity {
                    source_id: "sample.wav".into(),
                    sha256: [1; 32],
                },
                job_id: JobId(1),
                generation: Generation::first(),
                block: PcmBlock {
                    sequence: 0,
                    range,
                    channels: 1,
                    samples: vec![0; samples],
                },
            }),
        }
    }

    #[test]
    fn decoder_bounds_accept_one_and_eighty_thousand_and_reject_zero_or_oversized() {
        for count in [1, 80_000] {
            let request = DecodeRequest {
                source: SourceIdentity {
                    source_id: "sample.wav".into(),
                    sha256: [1; 32],
                },
                job_id: JobId(1),
                generation: Generation::first(),
                range: SourceRange::new(0, count as u64, 16_000).unwrap(),
                max_samples: count,
            };
            assert_eq!(
                DecoderProxy::new(Stub(Some(valid_response(count))))
                    .decode(request)
                    .unwrap()[0]
                    .block
                    .samples
                    .len(),
                count
            );
        }
        for max_samples in [0, 80_001] {
            let request = DecodeRequest {
                source: SourceIdentity {
                    source_id: "sample.wav".into(),
                    sha256: [1; 32],
                },
                job_id: JobId(1),
                generation: Generation::first(),
                range: SourceRange::new(0, 1, 16_000).unwrap(),
                max_samples,
            };
            assert_eq!(
                DecoderProxy::new(Stub(None)).decode(request),
                Err(PortError::InvalidInput)
            );
        }
    }

    #[test]
    fn decoder_rejects_a_block_outside_the_requested_window() {
        let request = DecodeRequest {
            source: SourceIdentity {
                source_id: "sample.wav".into(),
                sha256: [1; 32],
            },
            job_id: JobId(1),
            generation: Generation::first(),
            range: SourceRange::new(0, 80_000, 16_000).unwrap(),
            max_samples: 80_000,
        };
        let outside = DecodedPcmBlock {
            source: request.source.clone(),
            job_id: request.job_id,
            generation: request.generation,
            block: PcmBlock {
                sequence: 0,
                range: SourceRange::new(1, 80_001, 16_000).unwrap(),
                channels: 1,
                samples: vec![0; 80_000],
            },
        };
        let event = WorkerEvent::DecodedBlock {
            request_id: 1,
            block: Some(outside),
        };
        assert_eq!(
            DecoderProxy::new(Stub(Some(event))).decode(request),
            Err(PortError::StaleResponse)
        );
    }

    #[test]
    fn decoder_rejects_a_partial_block_that_skips_the_requested_prefix() {
        let request = DecodeRequest {
            source: SourceIdentity {
                source_id: "sample.wav".into(),
                sha256: [1; 32],
            },
            job_id: JobId(1),
            generation: Generation::first(),
            range: SourceRange::new(0, 16_000, 16_000).unwrap(),
            max_samples: 320,
        };
        let response = DecodedPcmBlock {
            source: request.source.clone(),
            job_id: request.job_id,
            generation: request.generation,
            block: PcmBlock {
                sequence: 0,
                range: SourceRange::new(1, 321, 16_000).unwrap(),
                channels: 1,
                samples: vec![0; 320],
            },
        };
        let event = WorkerEvent::DecodedBlock {
            request_id: 1,
            block: Some(response),
        };

        assert_eq!(
            DecoderProxy::new(Stub(Some(event))).decode(request),
            Err(PortError::StaleResponse)
        );
    }
}
