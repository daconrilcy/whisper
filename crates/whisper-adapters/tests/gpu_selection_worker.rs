use whisper_core::{
    ComputeChoice, Generation, JobConfig, JobId, LanguageChoice,
    gpu_inventory::GpuRuntimeIdentity,
    ipc::{BackendKind, IpcEnvelope, WorkerCommand, WorkerEvent},
};

#[cfg(windows)]
use sha2::{Digest, Sha256};

fn candidate() -> GpuRuntimeIdentity {
    GpuRuntimeIdentity {
        generation: 8,
        pnp_instance_id: "PCI\\VEN_10DE&DEV_1B06".into(),
        dxgi_rank: 2,
        luid: 0x1234,
        pci_id: "0000:01:00.0".into(),
        cuda_ordinal: 1,
        ggml_ordinal: 4,
    }
}

#[test]
fn gpu_import_and_live_commands_keep_candidate_identity_and_distinct_ordinals() {
    let candidate = candidate();
    let config = JobConfig {
        language: LanguageChoice::Automatic,
        compute: ComputeChoice::Gpu,
    };
    let commands = [
        WorkerCommand::StartGpuImport {
            retain_after_publish: true,
            job_id: JobId(3),
            generation: Generation::first(),
            instance_id: 11,
            source_path: "voice.wav".into(),
            source_sha256: [7; 32],
            expected_source_samples: 16_000,
            model_path: "model.bin".into(),
            model_sha256: [9; 32],
            config: config.clone(),
            candidate: candidate.clone(),
        },
        WorkerCommand::StartGpuLiveWorker {
            job_id: JobId(4),
            generation: Generation::first(),
            instance_id: 12,
            model_path: "model.bin".into(),
            model_sha256: [9; 32],
            config: config.clone(),
            candidate: candidate.clone(),
        },
        WorkerCommand::StartGpuDirectLiveWorker {
            job_id: JobId(5),
            generation: Generation::first(),
            instance_id: 13,
            model_path: "model.bin".into(),
            model_sha256: [9; 32],
            config,
            candidate: candidate.clone(),
        },
    ];
    for command in commands {
        let encoded = serde_json::to_vec(&IpcEnvelope::new(command.clone())).unwrap();
        let decoded: IpcEnvelope<WorkerCommand> = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded.message, command);
    }

    let ack = WorkerEvent::GpuSessionAttested {
        job_id: JobId(5),
        generation: Generation::first(),
        instance_id: 13,
        candidate: candidate.pnp_instance_id,
        dxgi_rank: candidate.dxgi_rank,
        luid: candidate.luid,
        pci_id: candidate.pci_id,
        cuda_ordinal: candidate.cuda_ordinal,
        ggml_ordinal: candidate.ggml_ordinal,
    };
    let encoded = serde_json::to_vec(&IpcEnvelope::new(ack.clone())).unwrap();
    let decoded: IpcEnvelope<WorkerEvent> = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(decoded.message, ack);
}

#[cfg(windows)]
#[test]
fn real_gpu_import_uses_current_windows_candidate_and_local_model_when_enabled() {
    use std::time::{Duration, Instant};
    use whisper_adapters::{WorkerTransport, worker_ipc::ChildWorkerTransport};
    use whisper_core::gpu_inventory_app::GpuInventoryApp;

    let Some(model_path) = std::env::var_os("WHISPER_L04_MODEL") else {
        eprintln!("V-L04 REAL GPU NOT RUN: WHISPER_L04_MODEL absent");
        return;
    };
    let Some(source_path) = std::env::var_os("WHISPER_L04_WAV") else {
        eprintln!("V-L04 REAL GPU NOT RUN: WHISPER_L04_WAV absent");
        return;
    };
    let Some(worker_path) = std::env::var_os("WHISPER_L04_GPU_WORKER") else {
        eprintln!("V-L04 REAL GPU NOT RUN: WHISPER_L04_GPU_WORKER absent");
        return;
    };
    let model_path = std::path::PathBuf::from(model_path);
    let source_path = std::path::PathBuf::from(source_path);
    let worker_path = std::path::PathBuf::from(worker_path);
    assert!(
        model_path.is_file(),
        "local large-v3-turbo model is missing"
    );
    assert!(source_path.is_file(), "local WAV is missing");
    assert!(worker_path.is_file(), "release GPU worker is missing");

    let mut inventory = GpuInventoryApp::new(Box::new(
        whisper_adapters::gpu_inventory::WindowsGpuInventoryPort::new(worker_path.clone()),
    ));
    let inventory_deadline = Instant::now() + Duration::from_secs(30);
    while matches!(
        inventory.qualified_state(),
        whisper_core::gpu_inventory_app::GpuQualifiedInventoryState::Loading
    ) && Instant::now() < inventory_deadline
    {
        inventory.poll();
        std::thread::sleep(Duration::from_millis(10));
    }
    let candidate = inventory
        .runtime_identities()
        .iter()
        .find(|identity| {
            identity
                .pnp_instance_id
                .to_ascii_uppercase()
                .contains("VEN_10DE")
        })
        .cloned()
        .expect("no currently qualified NVIDIA runtime identity was returned");

    let model_sha256 = sha256_file(&model_path);
    let source_bytes = std::fs::read(&source_path).unwrap();
    let source_sha256: [u8; 32] = Sha256::digest(&source_bytes).into();
    let expected_source_samples = wav_pcm_samples_at_16khz(&source_bytes);
    let before = source_sha256;
    let job_id = JobId(0x4750_5500_0000_0000_u128 | u128::from(std::process::id()));
    let generation = Generation::first();
    let instance_id = 77;
    let command = WorkerCommand::StartGpuImport {
        retain_after_publish: true,
        job_id,
        generation,
        instance_id,
        source_path: source_path.to_string_lossy().into_owned(),
        source_sha256,
        expected_source_samples,
        model_path: model_path.to_string_lossy().into_owned(),
        model_sha256,
        config: JobConfig {
            language: LanguageChoice::Automatic,
            compute: ComputeChoice::Gpu,
        },
        candidate: candidate.clone(),
    };
    let started_at = Instant::now();
    let mut child = ChildWorkerTransport::spawn(&worker_path).expect("launch release GPU worker");
    child.request(command).expect("send GPU import request");

    let deadline = Instant::now() + Duration::from_secs(300);
    let mut attested = false;
    let mut ready_at = None;
    let mut transcript = String::new();
    let mut first_text_elapsed = None;
    let mut got_end = false;
    let mut completed_at = None;
    while Instant::now() < deadline {
        let event = match child.receive() {
            Ok(Some(event)) => event,
            Ok(None) => {
                std::thread::sleep(Duration::from_millis(5));
                continue;
            }
            Err(error) => panic!("GPU worker transport rejected event: {error:?}"),
        };
        match event {
            WorkerEvent::GpuSessionAttested {
                job_id: actual_job,
                generation: actual_generation,
                instance_id: actual_instance,
                candidate: pnp,
                dxgi_rank,
                luid,
                pci_id,
                cuda_ordinal,
                ggml_ordinal,
            } => {
                assert_eq!(
                    (actual_job, actual_generation, actual_instance),
                    (job_id, generation, instance_id)
                );
                assert_eq!(pnp, candidate.pnp_instance_id);
                assert_eq!(dxgi_rank, candidate.dxgi_rank);
                assert_eq!(luid, candidate.luid);
                assert_eq!(pci_id, candidate.pci_id);
                assert_eq!(cuda_ordinal, candidate.cuda_ordinal);
                assert_eq!(ggml_ordinal, candidate.ggml_ordinal);
                attested = true;
            }
            WorkerEvent::Ready {
                job_id: actual_job,
                generation: actual_generation,
                instance_id: actual_instance,
                backend,
            } => {
                assert!(attested, "Ready arrived before fresh GPU session identity");
                assert_eq!(
                    (actual_job, actual_generation, actual_instance),
                    (job_id, generation, instance_id)
                );
                assert_eq!(backend, BackendKind::Cuda);
                ready_at = Some(Instant::now());
            }
            WorkerEvent::Segment(segment) => {
                first_text_elapsed.get_or_insert_with(|| started_at.elapsed());
                transcript.push_str(&segment.text);
            }
            WorkerEvent::End {
                job_id: actual_job,
                generation: actual_generation,
                instance_id: actual_instance,
                ..
            } => {
                assert_eq!(
                    (actual_job, actual_generation, actual_instance),
                    (job_id, generation, instance_id)
                );
                got_end = true;
                completed_at = Some(Instant::now());
                break;
            }
            WorkerEvent::Failed { code, message, .. } => {
                panic!("real GPU import failed ({code:?}): {message}")
            }
            _ => {}
        }
    }
    assert!(
        got_end,
        "GPU worker did not finish before the 300-second bound"
    );
    let ready_at = ready_at.expect("GPU Ready event missing");
    assert!(
        !transcript.trim().is_empty(),
        "GPU produced no transcript text"
    );
    child
        .request(WorkerCommand::Shutdown)
        .expect("shutdown GPU worker");
    let shutdown_deadline = Instant::now() + Duration::from_secs(5);
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait for GPU worker shutdown") {
            break status;
        }
        assert!(
            Instant::now() < shutdown_deadline,
            "GPU worker shutdown timed out"
        );
        std::thread::sleep(Duration::from_millis(5));
    };
    assert!(status.success(), "GPU worker exited with {status}");
    assert_eq!(
        Sha256::digest(std::fs::read(&source_path).unwrap()).as_slice(),
        before
    );
    println!(
        "V-L04 REAL GPU PASS: backend=CUDA; pnp={}; dxgi_rank={}; luid={:#x}; pci={}; cuda_ordinal={}; ggml_ordinal={}; ready_ms={}; first_text_ms={}; import_ms={}; source_samples={}; transcript={:?}",
        candidate.pnp_instance_id,
        candidate.dxgi_rank,
        candidate.luid,
        candidate.pci_id,
        candidate.cuda_ordinal,
        candidate.ggml_ordinal,
        ready_at.duration_since(started_at).as_millis(),
        first_text_elapsed
            .expect("first text timestamp missing")
            .as_millis(),
        completed_at
            .expect("completion timestamp missing")
            .duration_since(started_at)
            .as_millis(),
        expected_source_samples,
        transcript,
    );
}

#[cfg(windows)]
fn wav_pcm_samples_at_16khz(bytes: &[u8]) -> u64 {
    assert_eq!(&bytes[..4], b"RIFF");
    let mut sample_rate = None;
    let mut block_align = None;
    let mut data_bytes = None;
    let mut cursor = 12;
    while cursor + 8 <= bytes.len() {
        let kind = &bytes[cursor..cursor + 4];
        let size = u32::from_le_bytes(bytes[cursor + 4..cursor + 8].try_into().unwrap()) as usize;
        let start = cursor + 8;
        let end = start.checked_add(size).expect("WAV chunk overflow");
        assert!(end <= bytes.len(), "truncated WAV chunk");
        if kind == b"fmt " && size >= 16 {
            block_align = Some(u16::from_le_bytes(
                bytes[start + 12..start + 14].try_into().unwrap(),
            ));
            sample_rate = Some(u32::from_le_bytes(
                bytes[start + 4..start + 8].try_into().unwrap(),
            ));
        } else if kind == b"data" {
            data_bytes = Some(size as u64);
        }
        cursor = end + (size & 1);
    }
    let rate = u64::from(sample_rate.expect("WAV fmt chunk"));
    let align = u64::from(block_align.expect("WAV block alignment"));
    (data_bytes.expect("WAV data chunk") / align) * 16_000 / rate
}

#[cfg(windows)]
fn sha256_file(path: &std::path::Path) -> [u8; 32] {
    use std::io::Read;
    let mut file = std::fs::File::open(path).unwrap();
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).unwrap();
        if count == 0 {
            break;
        }
        hasher.update(&buffer[..count]);
    }
    hasher.finalize().into()
}
