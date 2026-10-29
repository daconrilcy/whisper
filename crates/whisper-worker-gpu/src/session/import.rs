use super::*;
pub(super) fn run_import(
    rx: &Receiver<Incoming>,
    output: &mut impl io::Write,
    cached_engine: &mut Option<CachedGpuEngine>,
    request: ImportRequest,
) -> Result<(), String> {
    let ImportRequest {
        job_id,
        generation,
        instance_id,
        source_path,
        source_sha256,
        expected_source_samples,
        model_path,
        model_sha256,
        config,
        gpu_candidate,
    } = request;
    if config.compute == ComputeChoice::Cpu {
        return emit_failure(
            output,
            job_id,
            generation,
            instance_id,
            WorkerErrorCode::BackendUnavailable,
            "L01 requires GPU compute",
        );
    }
    if let Err(error) = verify_sha256(Path::new(&source_path), &source_sha256) {
        let code = if !Path::new(&source_path).is_file() {
            WorkerErrorCode::SourceMissing
        } else {
            WorkerErrorCode::SourceChanged
        };
        return emit_failure(
            output,
            job_id,
            generation,
            instance_id,
            code,
            &format!("SourceChanged: {error}"),
        );
    }
    if let Err(error) = verify_sha256(Path::new(&model_path), &model_sha256) {
        let code = if !Path::new(&model_path).is_file() {
            WorkerErrorCode::ModelUnavailable
        } else {
            WorkerErrorCode::ModelHashMismatch
        };
        return emit_failure(
            output,
            job_id,
            generation,
            instance_id,
            code,
            &format!("ModelHashMismatch: {error}"),
        );
    }
    #[cfg(feature = "l01-memory-qualification")]
    memory_qualification::record(
        "model_context.load.begin",
        serde_json::json!({
            "model_path_bytes": std::fs::metadata(&model_path).map(|metadata| metadata.len()).ok(),
            "model_context_native_bytes": "opaque; process counters characterize total",
        }),
    );
    let engine = match cached_gpu_engine(
        cached_engine,
        &model_path,
        model_sha256,
        gpu_candidate.as_ref(),
    ) {
        Ok(engine) => engine,
        Err(error) => {
            return emit_gpu_initialization_failure(
                output,
                job_id,
                generation,
                instance_id,
                &error,
            );
        }
    };
    if let Some(candidate) = gpu_candidate {
        write_frame(
            output,
            &IpcEnvelope::new(WorkerEvent::GpuSessionAttested {
                job_id,
                generation,
                instance_id,
                candidate: candidate.pnp_instance_id.clone(),
                dxgi_rank: candidate.dxgi_rank,
                luid: candidate.luid,
                pci_id: candidate.pci_id.clone(),
                cuda_ordinal: candidate.cuda_ordinal,
                ggml_ordinal: candidate.ggml_ordinal,
            }),
        )
        .map_err(|error| error.to_string())?;
    }
    #[cfg(feature = "l01-memory-qualification")]
    memory_qualification::record(
        "model_context.load.complete",
        serde_json::json!({
            "model_path_bytes": std::fs::metadata(&model_path).map(|metadata| metadata.len()).ok(),
            "native_context_and_initial_state_bytes": "opaque; included in worker process counters",
        }),
    );
    write_frame(
        output,
        &IpcEnvelope::new(WorkerEvent::Ready {
            job_id,
            generation,
            instance_id,
            backend: BackendKind::Cuda,
        }),
    )
    .map_err(|e| e.to_string())?;
    if config.compute == ComputeChoice::Auto {
        // Ready attests initialization only. Audio must wait for the parent's
        // decision so a failed GPU attempt can still switch to CPU beforehand.
        let incoming = rx
            .recv()
            .map_err(|_| "parent closed IPC before BeginImport".to_owned())??;
        if incoming.version != IPC_PROTOCOL_VERSION {
            return emit_failure(
                output,
                job_id,
                generation,
                instance_id,
                WorkerErrorCode::UnsupportedProtocol,
                "ProtocolMismatch: unsupported BeginImport version",
            );
        }
        match incoming.message {
            WorkerCommand::BeginImport {
                job_id: j,
                generation: g,
                instance_id: i,
            } if (j, g, i) == (job_id, generation, instance_id) => {}
            WorkerCommand::Stop {
                job_id: j,
                generation: g,
            } if (j, g) == (job_id, generation) => {
                return write_frame(
                    output,
                    &IpcEnvelope::new(WorkerEvent::Stopped {
                        job_id,
                        generation,
                        instance_id,
                    }),
                )
                .map_err(|e| e.to_string());
            }
            WorkerCommand::Shutdown => return Ok(()),
            _ => {
                return emit_failure(
                    output,
                    job_id,
                    generation,
                    instance_id,
                    WorkerErrorCode::InvalidRequest,
                    "InvalidRequest: expected correlated BeginImport",
                );
            }
        }
        write_frame(
            output,
            &IpcEnvelope::new(WorkerEvent::ImportStarted {
                job_id,
                generation,
                instance_id,
            }),
        )
        .map_err(|e| e.to_string())?;
    }
    if take_stop(rx, job_id, generation, instance_id, output)? {
        return write_frame(
            output,
            &IpcEnvelope::new(WorkerEvent::Stopped {
                job_id,
                generation,
                instance_id,
            }),
        )
        .map_err(|e| e.to_string());
    }
    let language = match &config.language {
        whisper_core::domain::LanguageChoice::Automatic => None,
        whisper_core::domain::LanguageChoice::Manual(value) => Some(value.as_str()),
    };
    let mut sequence = 0_u64;
    let mut stopped = false;
    let result = decoder::decode_wav_windows(Path::new(&source_path), |window_start, pcm| {
        if take_stop(rx, job_id, generation, instance_id, output)? {
            stopped = true;
            return Err("cancelled".into());
        }
        if pcm.is_empty() || pcm.len() > decoder::MAX_WINDOW_SAMPLES {
            return Err("decoded window violates the 80,000-sample bound".into());
        }
        #[cfg(feature = "l01-memory-qualification")]
        memory_qualification::record(
            "inference.begin",
            serde_json::json!({
                "source_start_sample": window_start,
                "pcm_samples": pcm.len(),
                "pcm_capacity_samples": pcm.capacity(),
                "pcm_capacity_bytes": pcm.capacity() * std::mem::size_of::<f32>(),
                "active_inference_windows": 1,
                "native_context_bytes": "opaque; process counters characterize total",
            }),
        );
        let segments = engine.transcribe(&pcm, language)?;
        #[cfg(feature = "l01-memory-qualification")]
        memory_qualification::record(
            "inference.complete",
            serde_json::json!({
                "source_start_sample": window_start,
                "pcm_samples": pcm.len(),
                "pcm_capacity_samples": pcm.capacity(),
                "segment_count": segments.len(),
                "segment_vec_capacity": segments.capacity(),
                "segment_object_bytes": segments.capacity() * std::mem::size_of::<native_engine::TextSegment>(),
                "segment_text_bytes_live": segments.iter().map(|segment| segment.text.len()).sum::<usize>(),
                "segment_text_capacity_bytes_live": segments.iter().map(|segment| segment.text.capacity()).sum::<usize>(),
            }),
        );
        for segment in segments {
            let start = (segment.start_centiseconds.max(0) as u64)
                .saturating_mul(160)
                .min(pcm.len() as u64);
            let end = (segment.end_centiseconds.max(0) as u64)
                .saturating_mul(160)
                .min(pcm.len() as u64);
            let start = window_start.saturating_add(start);
            let end = window_start.saturating_add(end);
            if end <= start || segment.text.trim().is_empty() {
                continue;
            }
            let range = SourceRange::new(start, end, decoder::OUTPUT_RATE)
                .ok_or_else(|| "invalid inference segment range".to_owned())?;
            sequence = sequence
                .checked_add(1)
                .ok_or_else(|| "segment sequence exhausted".to_owned())?;
            let segment = WorkerSegment {
                job_id,
                generation,
                instance_id,
                segment_id: SegmentId(sequence),
                range,
                text: segment.text.trim().to_owned(),
            };
            write_frame(
                output,
                &IpcEnvelope::new(WorkerEvent::Segment(WorkerSegmentDto::from(segment))),
            )
            .map_err(|e| e.to_string())?;
        }
        write_frame(
            output,
            &IpcEnvelope::new(WorkerEvent::Progress {
                job_id,
                generation,
                instance_id,
                completed_samples: window_start + pcm.len() as u64,
                total_samples: 0,
            }),
        )
        .map_err(|e| e.to_string())?;
        if take_stop(rx, job_id, generation, instance_id, output)? {
            stopped = true;
            return Err("cancelled".into());
        }
        Ok(())
    });
    if stopped {
        return write_frame(
            output,
            &IpcEnvelope::new(WorkerEvent::Stopped {
                job_id,
                generation,
                instance_id,
            }),
        )
        .map_err(|e| e.to_string());
    }
    let (decoded_frames, output_samples) = match result {
        Ok(result) => result,
        Err(error) => {
            let code = if error.starts_with("UnsupportedLanguage:") {
                WorkerErrorCode::UnsupportedLanguage
            } else if error.starts_with("UnsupportedFormat:") {
                WorkerErrorCode::UnsupportedFormat
            } else {
                WorkerErrorCode::DecodeFailed
            };
            return emit_failure(output, job_id, generation, instance_id, code, &error);
        }
    };
    if take_stop(rx, job_id, generation, instance_id, output)? {
        return write_frame(
            output,
            &IpcEnvelope::new(WorkerEvent::Stopped {
                job_id,
                generation,
                instance_id,
            }),
        )
        .map_err(|e| e.to_string());
    }
    if decoded_frames == 0 || output_samples == 0 {
        return emit_failure(
            output,
            job_id,
            generation,
            instance_id,
            WorkerErrorCode::DecodeFailed,
            "WAV contains no audio samples",
        );
    }
    if output_samples != expected_source_samples {
        return emit_failure(
            output,
            job_id,
            generation,
            instance_id,
            WorkerErrorCode::SourceChanged,
            "SourceChanged: decoded duration differs from prepared WAV duration",
        );
    }
    verify_sha256(Path::new(&source_path), &source_sha256)
        .map_err(|e| format!("SourceChangedDuringImport: {e}"))?;
    verify_sha256(Path::new(&model_path), &model_sha256)
        .map_err(|e| format!("ModelHashMismatchDuringImport: {e}"))?;
    #[cfg(feature = "l01-memory-qualification")]
    memory_qualification::record(
        "worker.finalized",
        serde_json::json!({
            "decoded_frames": decoded_frames,
            "output_samples": output_samples,
            "window_limit_samples": decoder::MAX_WINDOW_SAMPLES,
            "retained_full_source_pcm_samples": 0,
            "last_sequence": sequence,
        }),
    );
    write_frame(
        output,
        &IpcEnvelope::new(WorkerEvent::Progress {
            job_id,
            generation,
            instance_id,
            completed_samples: output_samples,
            total_samples: output_samples,
        }),
    )
    .map_err(|e| e.to_string())?;
    write_frame(
        output,
        &IpcEnvelope::new(WorkerEvent::End {
            job_id,
            generation,
            instance_id,
            last_sequence: sequence,
            last_offset: output_samples,
        }),
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}
