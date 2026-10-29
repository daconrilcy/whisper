use super::*;

fn handle_decode(
    request_id: u64,
    request: &DecodeRequest,
    output: &mut impl io::Write,
) -> Result<(), String> {
    if request_id == 0
        || request.max_samples == 0
        || request.max_samples > decoder::MAX_WINDOW_SAMPLES
        || request.range.sample_rate_hz != decoder::OUTPUT_RATE
        || request.range.start_sample >= request.range.end_sample
    {
        return write_frame(
            output,
            &IpcEnvelope::new(WorkerEvent::Failed {
                job_id: Some(request.job_id),
                generation: Some(request.generation),
                instance_id: Some(0),
                code: WorkerErrorCode::InvalidRequest,
                message: "InvalidRequest: decode bounds must be 1..=80,000 samples".into(),
            }),
        )
        .map_err(|e| e.to_string());
    }
    let source = Path::new(&request.source.source_id);
    if let Err(error) = verify_sha256(source, &request.source.sha256) {
        return write_frame(
            output,
            &IpcEnvelope::new(WorkerEvent::Failed {
                job_id: Some(request.job_id),
                generation: Some(request.generation),
                instance_id: Some(0),
                code: WorkerErrorCode::SourceChanged,
                message: format!("SourceChanged: {error}"),
            }),
        )
        .map_err(|e| e.to_string());
    }
    let samples = match decoder::decode_requested_block(request) {
        Ok(samples) => samples,
        Err(error) => {
            return write_frame(
                output,
                &IpcEnvelope::new(WorkerEvent::Failed {
                    job_id: Some(request.job_id),
                    generation: Some(request.generation),
                    instance_id: Some(0),
                    code: WorkerErrorCode::DecodeFailed,
                    message: error,
                }),
            )
            .map_err(|e| e.to_string());
        }
    };
    verify_sha256(source, &request.source.sha256).map_err(|e| format!("SourceChanged: {e}"))?;
    let Some(samples) = samples else {
        return write_frame(
            output,
            &IpcEnvelope::new(WorkerEvent::DecodedBlock {
                request_id,
                block: None,
            }),
        )
        .map_err(|e| e.to_string());
    };
    let block = DecodedPcmBlock {
        source: request.source.clone(),
        job_id: request.job_id,
        generation: request.generation,
        block: PcmBlock {
            sequence: 0,
            range: SourceRange::new(
                request.range.start_sample,
                request.range.start_sample + samples.len() as u64,
                decoder::OUTPUT_RATE,
            )
            .ok_or_else(|| "InvalidRequest: decoded range is empty".to_owned())?,
            channels: 1,
            samples,
        },
    };
    write_frame(
        output,
        &IpcEnvelope::new(WorkerEvent::DecodedBlock {
            request_id,
            block: Some(block),
        }),
    )
    .map_err(|e| e.to_string())
}

pub(super) fn run_decode_service(
    rx: &Receiver<Incoming>,
    output: &mut impl io::Write,
    mut request_id: u64,
    mut request: DecodeRequest,
) -> Result<(), String> {
    loop {
        handle_decode(request_id, &request, output)?;
        let incoming = match rx.recv() {
            Ok(incoming) => incoming,
            Err(_) => return Ok(()),
        };
        let envelope = incoming?;
        if envelope.version != IPC_PROTOCOL_VERSION {
            write_frame(
                output,
                &IpcEnvelope::new(WorkerEvent::Failed {
                    job_id: None,
                    generation: None,
                    instance_id: None,
                    code: WorkerErrorCode::UnsupportedProtocol,
                    message: format!(
                        "ProtocolMismatch: expected v{IPC_PROTOCOL_VERSION}, got v{}",
                        envelope.version
                    ),
                }),
            )
            .map_err(|e| e.to_string())?;
            continue;
        }
        match envelope.message {
            WorkerCommand::DecodeBlock {
                request_id: next_id,
                request: next_request,
            } => {
                request_id = next_id;
                request = next_request;
            }
            WorkerCommand::Shutdown => return Ok(()),
            _ => {
                write_frame(
                    output,
                    &IpcEnvelope::new(WorkerEvent::Failed {
                        job_id: None,
                        generation: None,
                        instance_id: None,
                        code: WorkerErrorCode::InvalidRequest,
                        message: "InvalidRequest: decode service accepts DecodeBlock or Shutdown"
                            .into(),
                    }),
                )
                .map_err(|e| e.to_string())?;
            }
        }
    }
}
