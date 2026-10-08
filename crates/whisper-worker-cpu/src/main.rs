mod decoder;
mod encoder;
// `read_frame` is shared with the inherited IPC module; its unbounded writer
// remains unused by this executable, which writes through the bounded sink below.
#[allow(dead_code)]
mod ipc;
mod native_engine;

#[cfg(feature = "l01-memory-qualification")]
mod memory_qualification {
    use serde_json::Value;
    use std::{
        fs::{File, OpenOptions},
        io::{BufWriter, Write},
        path::PathBuf,
        sync::{Mutex, OnceLock},
        time::{SystemTime, UNIX_EPOCH},
    };

    static REPORT: OnceLock<Option<Mutex<BufWriter<File>>>> = OnceLock::new();

    pub fn record(stage: &str, details: Value) {
        let report = REPORT.get_or_init(|| {
            let path = std::env::var_os("WHISPER_L01_MEMORY_TRACE").map(PathBuf::from)?;
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
                .ok()
                .map(|file| Mutex::new(BufWriter::new(file)))
        });
        let Some(report) = report else { return };
        let mut details = details;
        let mut process_memory = if matches!(
            stage,
            "worker.input.reader.created"
                | "model_context.load.complete"
                | "codec.ready"
                | "native.state.create.begin"
                | "native.inference.complete"
                | "native.state.destroyed"
                | "worker.finalized"
        ) {
            Some(process_memory_sample())
        } else {
            None
        };
        let elapsed_unix_us = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_micros());
        if let Some(sample) = process_memory.as_mut() {
            if let Some(sample) = sample.as_object_mut() {
                sample.insert(
                    "sampled_at_unix_us".into(),
                    serde_json::json!(elapsed_unix_us),
                );
            }
            if let Some(details) = details.as_object_mut() {
                details.insert("process_memory".into(), sample.clone());
            }
        }
        if let Ok(mut output) = report.lock() {
            let record = serde_json::json!({
                "elapsed_unix_us": elapsed_unix_us,
                "stage": stage,
                "details": details,
            });
            let _ = serde_json::to_writer(&mut *output, &record);
            let _ = output.write_all(b"\n");
            let _ = output.flush();
        }
    }

    fn process_memory_sample() -> Value {
        use std::process::{Command, Stdio};

        let pid = std::process::id();
        let script = format!(
            "$p=Get-Process -Id {pid} -ErrorAction Stop; [ordered]@{{ private_bytes=$p.PrivateMemorySize64; working_set_bytes=$p.WorkingSet64; peak_working_set_bytes=$p.PeakWorkingSet64 }} | ConvertTo-Json -Compress"
        );
        match Command::new("powershell.exe")
            .args(["-NoProfile", "-Command", &script])
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
        {
            Ok(output) if output.status.success() => serde_json::from_slice(&output.stdout)
                .unwrap_or_else(|error| {
                    serde_json::json!({
                        "error": format!("invalid process-counter JSON: {error}"),
                        "stdout": String::from_utf8_lossy(&output.stdout),
                    })
                }),
            Ok(output) => serde_json::json!({
                "error": format!("process-counter PowerShell exited with {}", output.status),
                "stderr": String::from_utf8_lossy(&output.stderr),
            }),
            Err(error) => serde_json::json!({
                "error": format!("cannot query process counters: {error}"),
            }),
        }
    }
}

use ipc::read_frame;
use native_engine::CpuEngine;
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{self, BufReader, BufWriter, Read},
    path::Path,
    sync::mpsc::{self, Receiver, SyncSender},
    thread,
};
use whisper_core::{
    SegmentId,
    domain::{ComputeChoice, SourceRange},
    ipc::{
        BackendKind, IPC_PROTOCOL_VERSION, IpcEnvelope, WorkerCommand, WorkerErrorCode,
        WorkerEvent, WorkerSegmentDto,
    },
    ports::{DecodeRequest, DecodedPcmBlock, PcmBlock, WorkerSegment},
};

type Incoming = Result<IpcEnvelope<WorkerCommand>, String>;

const MAX_IPC_FRAME_BYTES: usize = ipc::MAX_FRAME_BYTES;

struct BoundedJsonFrame(Vec<u8>);

impl io::Write for BoundedJsonFrame {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let limit = MAX_IPC_FRAME_BYTES - 1;
        if self.0.len().saturating_add(bytes.len()) > limit {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "IPC frame exceeds 1 MiB",
            ));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn write_frame<T: serde::Serialize>(output: &mut impl io::Write, value: &T) -> io::Result<()> {
    let mut frame = BoundedJsonFrame(Vec::with_capacity(4096));
    serde_json::to_writer(&mut frame, value).map_err(|error| {
        io::Error::new(
            error.io_error_kind().unwrap_or(io::ErrorKind::InvalidData),
            error,
        )
    })?;
    output.write_all(&frame.0)?;
    output.write_all(b"\n")?;
    output.flush()
}

fn main() {
    let (tx, rx) = mpsc::sync_channel(8);
    thread::spawn(move || input_loop(tx));
    let stdout = io::stdout();
    let mut output = BufWriter::new(stdout.lock());
    #[cfg(feature = "l01-memory-qualification")]
    memory_qualification::record(
        "worker.started",
        serde_json::json!({
            "input_event_channel_capacity": 8,
            "stdout_bufwriter_capacity_bytes": output.capacity(),
            "ipc_frame_limit_bytes": ipc::MAX_FRAME_BYTES,
        }),
    );
    if let Err(message) = run(rx, &mut output) {
        let event = IpcEnvelope::new(WorkerEvent::Failed {
            job_id: None,
            generation: None,
            instance_id: None,
            code: WorkerErrorCode::Internal,
            message,
        });
        let _ = write_frame(&mut output, &event);
        std::process::exit(2);
    }
}

fn input_loop(tx: SyncSender<Incoming>) {
    let stdin = io::stdin();
    let mut input = BufReader::new(stdin.lock());
    #[cfg(feature = "l01-memory-qualification")]
    {
        #[cfg(windows)]
        use std::os::windows::io::AsRawHandle;
        let details = {
            #[cfg(windows)]
            {
                serde_json::json!({
                    "stdin_bufreader_capacity_bytes": input.capacity(),
                    "worker_pid": std::process::id(),
                    "stdin_read_pipe_handle": input.get_ref().as_raw_handle() as usize,
                    "kernel_pipe_capacity": "query through worker read handle in Windows sampler",
                })
            }
            #[cfg(not(windows))]
            {
                serde_json::json!({
                    "stdin_bufreader_capacity_bytes": input.capacity(),
                    "kernel_pipe_capacity": "Windows-only qualification",
                })
            }
        };
        memory_qualification::record("worker.input.reader.created", details);
    }
    loop {
        match read_frame::<IpcEnvelope<WorkerCommand>>(&mut input) {
            Ok(Some(envelope)) => {
                if tx.send(Ok(envelope)).is_err() {
                    break;
                }
            }
            Ok(None) => break,
            Err(error) => {
                let _ = tx.send(Err(error.to_string()));
                break;
            }
        }
    }
}

fn run(rx: Receiver<Incoming>, output: &mut impl io::Write) -> Result<(), String> {
    let first = rx
        .recv()
        .map_err(|_| "parent closed IPC before StartImport".to_owned())??;
    if first.version != IPC_PROTOCOL_VERSION {
        write_frame(
            output,
            &IpcEnvelope::new(WorkerEvent::Failed {
                job_id: None,
                generation: None,
                instance_id: None,
                code: WorkerErrorCode::UnsupportedProtocol,
                message: format!(
                    "ProtocolMismatch: expected v{IPC_PROTOCOL_VERSION}, got v{}",
                    first.version
                ),
            }),
        )
        .map_err(|e| e.to_string())?;
        return Ok(());
    }
    let (
        job_id,
        generation,
        instance_id,
        source_path,
        source_sha256,
        expected_source_samples,
        model_path,
        model_sha256,
        config,
    ) = match first.message {
        WorkerCommand::DecodeBlock {
            request_id,
            request,
        } => {
            return run_decode_service(rx, output, request_id, request);
        }
        WorkerCommand::StartLiveWorker {
            job_id,
            generation,
            instance_id,
            model_path,
            model_sha256,
            config,
        } => {
            if config.compute != ComputeChoice::Cpu
                || verify_sha256(Path::new(&model_path), &model_sha256).is_err()
            {
                return emit_failure(
                    output,
                    job_id,
                    generation,
                    instance_id,
                    WorkerErrorCode::ModelHashMismatch,
                    "ModelHashMismatch: live model identity could not be verified",
                );
            }
            let engine = match CpuEngine::load(&model_path) {
                Ok(engine) => engine,
                Err(error) => {
                    return emit_failure(
                        output,
                        job_id,
                        generation,
                        instance_id,
                        WorkerErrorCode::ModelUnavailable,
                        &error,
                    );
                }
            };
            write_frame(
                output,
                &IpcEnvelope::new(WorkerEvent::Ready {
                    job_id,
                    generation,
                    instance_id,
                    backend: BackendKind::Cpu,
                }),
            )
            .map_err(|e| e.to_string())?;
            return run_live_service(rx, output, engine, job_id, generation, instance_id, config);
        }
        WorkerCommand::StartImport {
            job_id,
            generation,
            instance_id,
            source_path,
            source_sha256,
            expected_source_samples,
            model_path,
            model_sha256,
            config,
        } => (
            job_id,
            generation,
            instance_id,
            source_path,
            source_sha256,
            expected_source_samples,
            model_path,
            model_sha256,
            config,
        ),
        _ => {
            return Err(
                "first IPC command must be StartImport, StartLiveWorker or DecodeBlock".into(),
            );
        }
    };
    if config.compute != ComputeChoice::Cpu {
        return emit_failure(
            output,
            job_id,
            generation,
            instance_id,
            WorkerErrorCode::BackendUnavailable,
            "L01 requires CPU compute",
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
    let engine = match CpuEngine::load(&model_path) {
        Ok(engine) => engine,
        Err(error) => {
            return emit_failure(
                output,
                job_id,
                generation,
                instance_id,
                WorkerErrorCode::ModelUnavailable,
                &error,
            );
        }
    };
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
            backend: BackendKind::Cpu,
        }),
    )
    .map_err(|e| e.to_string())?;
    if take_stop(&rx, job_id, generation, instance_id, output)? {
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
        if take_stop(&rx, job_id, generation, instance_id, output)? {
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
        if take_stop(&rx, job_id, generation, instance_id, output)? {
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
    if take_stop(&rx, job_id, generation, instance_id, output)? {
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

fn run_live_service(
    rx: Receiver<Incoming>,
    output: &mut impl io::Write,
    engine: CpuEngine,
    job_id: whisper_core::JobId,
    generation: whisper_core::Generation,
    instance_id: u64,
    config: whisper_core::JobConfig,
) -> Result<(), String> {
    let mut expected_sequence = 1_u64;
    let mut next_segment_id = 1_u64;
    let mut next_packet_id = 1_u64;
    let mut mp3 = encoder::LiveMp3Encoder::new();
    loop {
        let envelope = rx
            .recv()
            .map_err(|_| "live IPC closed before Stop".to_owned())??;
        if envelope.version != IPC_PROTOCOL_VERSION {
            return Err("ProtocolMismatch: live command version changed".into());
        }
        let transcribe_window = matches!(&envelope.message, WorkerCommand::LiveWindow { .. });
        match envelope.message {
            WorkerCommand::LiveWindow {
                job_id: target,
                generation: target_generation,
                instance_id: target_instance,
                sequence,
                range,
                samples,
            }
            | WorkerCommand::LiveReplayWindow {
                job_id: target,
                generation: target_generation,
                instance_id: target_instance,
                sequence,
                range,
                samples,
            } => {
                if target != job_id
                    || target_generation != generation
                    || target_instance != instance_id
                    || sequence != expected_sequence
                    || range.sample_rate_hz != 16_000
                    || range.start_sample >= range.end_sample
                    || range.end_sample - range.start_sample != samples.len() as u64
                    || samples.is_empty()
                    || samples.len() > 80_000
                {
                    return emit_failure(
                        output,
                        job_id,
                        generation,
                        instance_id,
                        WorkerErrorCode::InvalidRequest,
                        "InvalidRequest: live PCM identity or window bounds are invalid",
                    );
                }
                for chunk in samples.chunks(16_000) {
                    for bytes in mp3.push_pcm(chunk)? {
                        let sequence = next_packet_id;
                        next_packet_id = next_packet_id
                            .checked_add(1)
                            .ok_or_else(|| "live MP3 packet sequence exhausted".to_owned())?;
                        write_frame(
                            output,
                            &IpcEnvelope::new(WorkerEvent::LiveMp3Packet {
                                job_id,
                                generation,
                                instance_id,
                                sequence,
                                bytes,
                            }),
                        )
                        .map_err(|e| e.to_string())?;
                    }
                }
                expected_sequence = expected_sequence
                    .checked_add(1)
                    .ok_or_else(|| "live window sequence exhausted".to_owned())?;
                if transcribe_window {
                    let pcm: Vec<f32> = samples
                        .iter()
                        .map(|sample| *sample as f32 / 32768.0)
                        .collect();
                    let language = match &config.language {
                        whisper_core::LanguageChoice::Automatic => None,
                        whisper_core::LanguageChoice::Manual(code) => Some(code.as_str()),
                    };
                    let segments = match engine.transcribe(&pcm, language) {
                        Ok(segments) => segments,
                        Err(error) => {
                            return emit_failure(
                                output,
                                job_id,
                                generation,
                                instance_id,
                                WorkerErrorCode::InferenceFailed,
                                &error,
                            );
                        }
                    };
                    for segment in segments {
                        let start = range
                            .start_sample
                            .saturating_add(
                                (segment.start_centiseconds.max(0) as u64).saturating_mul(160),
                            )
                            .min(range.end_sample);
                        let end = range
                            .start_sample
                            .saturating_add(
                                (segment.end_centiseconds.max(0) as u64).saturating_mul(160),
                            )
                            .min(range.end_sample);
                        if end <= start || segment.text.trim().is_empty() {
                            continue;
                        }
                        let segment_id = next_segment_id;
                        next_segment_id = next_segment_id
                            .checked_add(1)
                            .ok_or_else(|| "live segment id exhausted".to_owned())?;
                        let message = WorkerEvent::Segment(WorkerSegmentDto {
                            job_id,
                            generation,
                            instance_id,
                            segment_id: SegmentId(segment_id),
                            range: SourceRange::new(start, end, 16_000)
                                .ok_or_else(|| "live segment range is invalid".to_owned())?,
                            text: segment.text.trim().to_owned(),
                        });
                        write_frame(output, &IpcEnvelope::new(message))
                            .map_err(|e| e.to_string())?;
                    }
                }
                write_frame(
                    output,
                    &IpcEnvelope::new(WorkerEvent::WindowFinished {
                        job_id,
                        generation,
                        instance_id,
                        sequence,
                        range,
                        last_segment_sequence: next_segment_id - 1,
                    }),
                )
                .map_err(|e| e.to_string())?;
            }
            WorkerCommand::Stop {
                job_id: target,
                generation: target_generation,
            } if target == job_id && target_generation == generation => {
                for bytes in mp3.finish()? {
                    let sequence = next_packet_id;
                    next_packet_id = next_packet_id
                        .checked_add(1)
                        .ok_or_else(|| "live MP3 packet sequence exhausted".to_owned())?;
                    write_frame(
                        output,
                        &IpcEnvelope::new(WorkerEvent::LiveMp3Packet {
                            job_id,
                            generation,
                            instance_id,
                            sequence,
                            bytes,
                        }),
                    )
                    .map_err(|e| e.to_string())?;
                }
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
                    "InvalidRequest: unexpected command in live worker",
                );
            }
        }
    }
}

fn take_stop(
    rx: &Receiver<Incoming>,
    job_id: whisper_core::JobId,
    generation: whisper_core::Generation,
    instance_id: u64,
    output: &mut impl io::Write,
) -> Result<bool, String> {
    loop {
        match rx.try_recv() {
            Ok(Ok(envelope)) if envelope.version != IPC_PROTOCOL_VERSION => {
                return emit_failure(
                    output,
                    job_id,
                    generation,
                    instance_id,
                    WorkerErrorCode::UnsupportedProtocol,
                    "ProtocolMismatch",
                )
                .map(|_| true);
            }
            Ok(Ok(IpcEnvelope {
                message:
                    WorkerCommand::Stop {
                        job_id: stop_job,
                        generation: stop_generation,
                    },
                ..
            })) if stop_job == job_id && stop_generation == generation => return Ok(true),
            Ok(Ok(IpcEnvelope {
                message: WorkerCommand::Shutdown,
                ..
            })) => return Ok(true),
            Ok(Ok(_)) => continue,
            Ok(Err(error)) => return Err(format!("IPC input error: {error}")),
            Err(mpsc::TryRecvError::Empty) => return Ok(false),
            Err(mpsc::TryRecvError::Disconnected) => return Ok(false),
        }
    }
}

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

fn run_decode_service(
    rx: Receiver<Incoming>,
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

fn verify_sha256(path: &Path, expected: &[u8; 32]) -> Result<(), String> {
    let mut file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    if digest.finalize().as_slice() != expected {
        return Err(format!("SHA-256 mismatch for {}", path.display()));
    }
    Ok(())
}

fn emit_failure(
    output: &mut impl io::Write,
    job_id: whisper_core::JobId,
    generation: whisper_core::Generation,
    instance_id: u64,
    code: WorkerErrorCode,
    message: &str,
) -> Result<(), String> {
    write_frame(
        output,
        &IpcEnvelope::new(WorkerEvent::Failed {
            job_id: Some(job_id),
            generation: Some(generation),
            instance_id: Some(instance_id),
            code,
            message: message.into(),
        }),
    )
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod bounded_writer_tests {
    use super::*;

    #[test]
    fn writer_accepts_limit_and_rejects_limit_plus_one_without_growing_past_it() {
        let exact = "x".repeat(MAX_IPC_FRAME_BYTES - 3);
        let mut output = Vec::new();
        write_frame(&mut output, &exact).unwrap();
        assert_eq!(output.len(), MAX_IPC_FRAME_BYTES);

        let oversized = "x".repeat(MAX_IPC_FRAME_BYTES - 2);
        assert_eq!(
            write_frame(&mut Vec::new(), &oversized).unwrap_err().kind(),
            io::ErrorKind::InvalidData
        );
    }
}
