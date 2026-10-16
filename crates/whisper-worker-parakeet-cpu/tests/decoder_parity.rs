use std::{
    env,
    fs::{self, File},
    io::{BufRead, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};

use serde_json::Value;
use sha2::{Digest, Sha256};
use whisper_core::{
    Generation, JobId, SourceRange,
    ipc::{IpcEnvelope, WorkerCommand, WorkerEvent},
    parakeet_ipc::PARAKEET_MAX_AUDIO_SAMPLES,
    ports::{DecodeRequest, SourceIdentity},
};

#[test]
#[ignore = "compares the real Turbo decoder and Parakeet worker decoder on WAV and MP3"]
fn worker_decoder_matches_turbo_decoder_for_wav_and_mp3_within_one_i16_unit() {
    let runtime = required_path("PARAKEET_L2_RUNTIME_ROOT");
    let output_dir = required_path("PARAKEET_L2_OUTPUT_DIR");
    let source = required_path("PARAKEET_L2_PARITY_WAV");
    let turbo_worker = required_path("PARAKEET_L2_TURBO_DECODE_WORKER");
    assert!(output_dir.starts_with(&runtime));
    assert!(source.is_file());

    let mp3 = output_dir.join("a11-parity-source.mp3");
    let ffmpeg = Command::new("ffmpeg")
        .args(["-v", "error", "-y", "-i"])
        .arg(&source)
        .args([
            "-map_metadata",
            "-1",
            "-vn",
            "-ac",
            "1",
            "-ar",
            "16000",
            "-codec:a",
            "libmp3lame",
            "-q:a",
            "2",
        ])
        .arg(&mp3)
        .status()
        .expect("ffmpeg must be available for A11");
    assert!(ffmpeg.success(), "ffmpeg MP3 derivation failed: {ffmpeg}");

    let mut reports = Vec::new();
    for (label, input) in [("wav", source.as_path()), ("mp3", mp3.as_path())] {
        let pcm_path = output_dir.join(format!("a11-{label}.f32le"));
        let output = Command::new(env!("CARGO_BIN_EXE_whisper-worker-parakeet-cpu"))
            .args(["--decode-only"])
            .arg(input)
            .arg(&pcm_path)
            .env("PARAKEET_L2_RUNTIME_ROOT", &runtime)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "Parakeet decoder failed for {label}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let metadata: Value = serde_json::from_slice(&output.stdout).unwrap();
        let sample_count = metadata["output_samples"].as_u64().unwrap() as usize;
        assert!(sample_count > 0 && sample_count <= PARAKEET_MAX_AUDIO_SAMPLES);
        assert_eq!(
            fs::metadata(&pcm_path).unwrap().len(),
            sample_count as u64 * 4
        );
        let mut file = File::open(&pcm_path).unwrap();
        let mut local_i16 = Vec::with_capacity(sample_count);
        for _ in 0..sample_count {
            let mut bytes = [0_u8; 4];
            file.read_exact(&mut bytes).unwrap();
            let sample = f32::from_le_bytes(bytes);
            assert!(sample.is_finite());
            local_i16.push((sample.clamp(-1.0, 1.0) * i16::MAX as f32).round() as i16);
        }

        let turbo = decode_with_turbo(&turbo_worker, input, &runtime, sample_count as u64);
        assert_eq!(
            turbo.len(),
            local_i16.len(),
            "sample count mismatch for {label}"
        );
        let max_abs_diff = local_i16
            .iter()
            .zip(&turbo)
            .map(|(left, right)| (i32::from(*left) - i32::from(*right)).unsigned_abs())
            .max()
            .unwrap_or(0);
        assert!(
            max_abs_diff <= 1,
            "A11 {label}: maximum PCM delta {max_abs_diff} exceeds 1 i16 unit"
        );
        let report = serde_json::json!({
            "format": label, "source": input, "source_sha256": sha256(input),
            "codec": metadata["codec"], "source_rate": metadata["source_rate"],
            "channels": metadata["channels"], "output_samples": sample_count,
            "turbo_decoder_worker": turbo_worker, "max_abs_i16_difference": max_abs_diff,
            "count_equal": true, "alignment": "same sample index; origin 0",
        });
        eprintln!(
            "A11_DECODER_PARITY {}",
            serde_json::to_string(&report).unwrap()
        );
        reports.push(report);
    }
    fs::write(
        output_dir.join("decoder-parity-report.json"),
        serde_json::to_vec_pretty(&reports).unwrap(),
    )
    .unwrap();
}

fn decode_with_turbo(worker: &Path, source: &Path, runtime: &Path, sample_count: u64) -> Vec<i16> {
    let source_hash = sha256_bytes(source);
    let identity = SourceIdentity {
        source_id: source.to_string_lossy().into_owned(),
        sha256: source_hash,
    };
    let mut process = TurboDecoder::start(worker, runtime);
    let mut samples = Vec::with_capacity(sample_count as usize);
    let mut request_id = 1_u64;
    let mut start = 0_u64;
    while start < sample_count {
        let count = (sample_count - start).min(80_000);
        let request = DecodeRequest {
            source: identity.clone(),
            job_id: JobId(1),
            generation: Generation::first(),
            range: SourceRange::new(start, start + count, 16_000).unwrap(),
            max_samples: count as usize,
        };
        let event = process.decode(request_id, request);
        let WorkerEvent::DecodedBlock {
            request_id: response_id,
            block: Some(block),
        } = event
        else {
            panic!("Turbo decoder returned an unexpected response: {event:?}");
        };
        assert_eq!(response_id, request_id);
        assert_eq!(block.block.range.start_sample, start);
        assert_eq!(block.block.range.end_sample, start + count);
        samples.extend(block.block.samples);
        start += count;
        request_id += 1;
    }
    process.shutdown();
    assert!(
        process.wait().success(),
        "Turbo decode worker failed on exit"
    );
    samples
}

struct TurboDecoder {
    child: Child,
    stdin: Option<ChildStdin>,
    stdout: BufReader<ChildStdout>,
}
impl TurboDecoder {
    fn start(worker: &Path, runtime: &Path) -> Self {
        let mut child = Command::new(worker)
            .env("WHISPER_L1B_RUNTIME_ROOT", runtime)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        Self {
            stdin: Some(child.stdin.take().unwrap()),
            stdout: BufReader::new(child.stdout.take().unwrap()),
            child,
        }
    }
    fn send(&mut self, command: WorkerCommand) -> WorkerEvent {
        let bytes = serde_json::to_vec(&IpcEnvelope::new(command)).unwrap();
        let stdin = self.stdin.as_mut().unwrap();
        stdin.write_all(&bytes).unwrap();
        stdin.write_all(b"\n").unwrap();
        stdin.flush().unwrap();
        let mut line = Vec::new();
        self.stdout.read_until(b'\n', &mut line).unwrap();
        assert!(!line.is_empty() && line.len() <= 1_048_576);
        serde_json::from_slice::<IpcEnvelope<WorkerEvent>>(&line)
            .unwrap()
            .message
    }
    fn decode(&mut self, id: u64, request: DecodeRequest) -> WorkerEvent {
        self.send(WorkerCommand::DecodeBlock {
            request_id: id,
            request,
        })
    }
    fn shutdown(&mut self) {
        let stdin = self.stdin.as_mut().unwrap();
        stdin
            .write_all(&serde_json::to_vec(&IpcEnvelope::new(WorkerCommand::Shutdown)).unwrap())
            .unwrap();
        stdin.write_all(b"\n").unwrap();
        stdin.flush().unwrap();
        drop(self.stdin.take());
    }
    fn wait(&mut self) -> std::process::ExitStatus {
        self.child.wait().unwrap()
    }
}

fn required_path(name: &str) -> PathBuf {
    let value = env::var_os(name).unwrap_or_else(|| panic!("missing required {name}"));
    PathBuf::from(value)
        .canonicalize()
        .unwrap_or_else(|e| panic!("invalid {name}: {e}"))
}
fn sha256(path: &Path) -> String {
    format!("{:x}", Sha256::digest(fs::read(path).unwrap()))
}
fn sha256_bytes(path: &Path) -> [u8; 32] {
    Sha256::digest(fs::read(path).unwrap()).into()
}
