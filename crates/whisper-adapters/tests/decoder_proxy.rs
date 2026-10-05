use whisper_adapters::{WorkerTransport, decoder::DecoderProxy};
use whisper_core::{
    domain::{Generation, JobId, SourceRange},
    ipc::{WorkerCommand, WorkerEvent},
    ports::{DecodeRequest, DecodedPcmBlock, DecoderPort, PcmBlock, PortError, SourceIdentity},
};

struct StubTransport {
    response: Option<WorkerEvent>,
    command: Option<WorkerCommand>,
}

impl WorkerTransport for StubTransport {
    fn request(&mut self, command: WorkerCommand) -> Result<(), PortError> {
        self.command = Some(command);
        Ok(())
    }

    fn receive(&mut self) -> Result<Option<WorkerEvent>, PortError> {
        Ok(self.response.take())
    }
}

fn request() -> DecodeRequest {
    DecodeRequest {
        source: SourceIdentity {
            source_id: "source-17".into(),
            sha256: [7; 32],
        },
        job_id: JobId(17),
        generation: Generation::first().next().unwrap(),
        range: SourceRange::new(320, 640, 16_000).unwrap(),
        max_samples: 320,
    }
}

fn response(request: &DecodeRequest, generation: Generation, range: SourceRange) -> WorkerEvent {
    WorkerEvent::DecodedBlock {
        request_id: 1,
        block: Some(DecodedPcmBlock {
            source: request.source.clone(),
            job_id: request.job_id,
            generation,
            block: PcmBlock {
                sequence: 2,
                range,
                channels: 1,
                samples: vec![0; 320],
            },
        }),
    }
}

#[test]
fn decoder_proxy_rejects_a_stale_generation_before_returning_audio() {
    let request = request();
    let stale_generation = Generation::first();
    let transport = StubTransport {
        response: Some(response(&request, stale_generation, request.range)),
        command: None,
    };
    let mut decoder = DecoderProxy::new(transport);

    assert_eq!(
        decoder.decode(request.clone()),
        Err(PortError::StaleResponse)
    );
    match decoder.into_inner().command.unwrap() {
        WorkerCommand::DecodeBlock { request: sent, .. } => assert_eq!(sent, request),
        command => panic!("unexpected command: {command:?}"),
    }
}

#[test]
fn decoder_proxy_accepts_only_matching_source_job_generation_and_range() {
    let request = request();
    let transport = StubTransport {
        response: Some(response(&request, request.generation, request.range)),
        command: None,
    };
    let mut decoder = DecoderProxy::new(transport);

    let blocks = decoder.decode(request).unwrap();
    assert_eq!(blocks.len(), 1);
}

#[test]
fn decoder_proxy_accepts_bounded_partial_range_and_rejects_out_of_range_block() {
    let mut request = request();
    request.range = SourceRange::new(0, 16_000, 16_000).unwrap();
    request.max_samples = 320;
    let partial_range = SourceRange::new(0, 320, 16_000).unwrap();
    let transport = StubTransport {
        response: Some(response(&request, request.generation, partial_range)),
        command: None,
    };
    let mut decoder = DecoderProxy::new(transport);

    let blocks = decoder.decode(request.clone()).unwrap();
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].block.range, partial_range);
    assert_eq!(blocks[0].block.samples.len(), 320);

    let out_of_range = SourceRange::new(15_900, 16_220, 16_000).unwrap();
    let transport = StubTransport {
        response: Some(response(&request, request.generation, out_of_range)),
        command: None,
    };
    let mut decoder = DecoderProxy::new(transport);
    assert_eq!(decoder.decode(request), Err(PortError::StaleResponse));
}
