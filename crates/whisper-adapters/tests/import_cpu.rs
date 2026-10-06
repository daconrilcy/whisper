use std::cell::Cell;
use std::rc::Rc;
use std::{
    path::PathBuf,
    process::{Child, Command, Stdio},
    thread,
    time::{Duration, Instant},
};
use whisper_adapters::WorkerTransport;
use whisper_adapters::{
    archive::{ArchiveStore, sha256_file},
    decoder::DecoderProxy,
    worker_ipc::{AsyncImportIo, ChildWorkerTransport},
};
use whisper_core::{
    AppCommand, Application, ComputeChoice, Generation, ImportApplication, ImportRequest,
    JobConfig, JobId, JobState, LanguageChoice, SourceRange,
    ipc::{WorkerCommand, WorkerEvent},
    ports::{DecodeRequest, DecoderPort, PortError, SourceIdentity},
};

const MODEL_HASH: &str = "1fc70f774d38eb169993ac391eea357ef47c88757ef72ee5943879b7e8e2bc69";
const SOURCE_HASH: &str = "716e788c790b600fe472d0ebb868094c9d9b97fdf1d581133dd3df46a30de467";

#[test]
fn real_wav_runs_through_cpu_worker_and_verified_archive() {
    let Some(source) = std::env::var_os("WHISPER_L01_FIXTURE").map(PathBuf::from) else {
        eprintln!("V-IMPORT NOT RUN: set WHISPER_L01_FIXTURE to the D19-manifested micro_test.wav");
        return;
    };
    let Some(model) = std::env::var_os("WHISPER_L01_MODEL").map(PathBuf::from) else {
        eprintln!("V-IMPORT NOT RUN: set WHISPER_L01_MODEL to the approved D19 model");
        return;
    };
    assert_eq!(
        hex(&sha256_file(&source).unwrap()),
        SOURCE_HASH,
        "manifested human-labelled source identity"
    );
    assert_eq!(
        hex(&sha256_file(&model).unwrap()),
        MODEL_HASH,
        "approved D19 model identity"
    );
    let before = sha256_file(&source).unwrap();
    let destination = std::env::temp_dir().join(format!(
        "whisper-l01-import-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&destination).unwrap();
    let worker = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("whisper-worker-cpu.exe");
    assert!(
        worker.is_file(),
        "CPU worker binary must exist at {}",
        worker.display()
    );
    let memory_observation = MemoryObservation::start(&worker);
    let mut app = ImportApplication::new(AsyncImportIo::start(worker));
    let job_id = JobId(((std::process::id() as u128) << 64) | 1);
    app.dispatch(AppCommand::StartImport {
        request: ImportRequest {
            job_id,
            generation: Generation::first(),
            source_path: source.to_string_lossy().into_owned(),
            source_sha256: None,
            source_samples: None,
            model_path: model.to_string_lossy().into_owned(),
            model_sha256: parse_hash(MODEL_HASH),
            destination: destination.to_string_lossy().into_owned(),
            config: JobConfig {
                language: LanguageChoice::Manual("fr".into()),
                compute: ComputeChoice::Cpu,
            },
        },
    })
    .unwrap();
    let started = Instant::now();
    let mut saw_cpu = false;
    loop {
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        saw_cpu |= view.message.as_deref() == Some("Moteur actif : CPU");
        if view
            .active_job
            .is_some_and(|(_, state)| state == JobState::Complete)
        {
            break;
        }
        if view
            .active_job
            .is_some_and(|(_, state)| state == JobState::Recoverable)
        {
            panic!("real import failed: {:?}", view.message);
        }
        assert!(
            started.elapsed() < Duration::from_secs(300),
            "real CPU transcription exceeded the campaign bound"
        );
        thread::sleep(Duration::from_millis(20));
    }
    if let Some(observation) = memory_observation {
        observation.finish(3);
    }
    assert!(saw_cpu, "worker never attested CPU");
    assert_eq!(
        before,
        sha256_file(&source).unwrap(),
        "import must not modify its source"
    );
    let first = ArchiveStore::scan(&destination).unwrap();
    let second = ArchiveStore::scan(&destination).unwrap();
    assert_eq!(first, second, "two archive scans must be stable");
    let entry = first
        .iter()
        .find(|entry| entry.job_id == job_id)
        .expect("published job in history");
    assert!(entry.complete);
    let generation_dir = destination
        .join("transcriptions")
        .join(format!("{:032x}", job_id.0))
        .join("1");
    let transcript = std::fs::read_to_string(generation_dir.join("transcript.txt")).unwrap();
    let subtitles = std::fs::read_to_string(generation_dir.join("transcript.srt")).unwrap();
    assert!(
        !transcript.trim().is_empty(),
        "real worker must return transcript text"
    );
    assert!(!subtitles.trim().is_empty(), "real worker must publish SRT");
    eprintln!(
        "V-IMPORT PASS: source_sha256={SOURCE_HASH}; model_sha256={MODEL_HASH}; cpu_attested={saw_cpu}; transcript_bytes={}; srt_bytes={}; elapsed_ms={}",
        transcript.len(),
        subtitles.len(),
        started.elapsed().as_millis()
    );
}

#[cfg(all(windows, feature = "l01-memory-qualification"))]
#[test]
fn generated_long_wav_runs_eight_full_native_windows_with_stage_memory_samples() {
    let Some(model) = std::env::var_os("WHISPER_L01_MODEL").map(PathBuf::from) else {
        eprintln!("V-IMPORT LONG NOT RUN: set WHISPER_L01_MODEL to the approved D19 model");
        return;
    };
    assert_eq!(
        hex(&sha256_file(&model).unwrap()),
        MODEL_HASH,
        "approved D19 model identity"
    );
    assert!(
        std::env::var_os("WHISPER_L01_MEMORY_TRACE").is_some()
            && std::env::var_os("WHISPER_L01_ADAPTER_MEMORY_TRACE").is_some(),
        "long memory qualification requires worker and adapter trace paths"
    );

    const SOURCE_SAMPLES: u32 = 640_123;
    let source = std::env::temp_dir().join(format!(
        "whisper-l01-generated-{}-{}.wav",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    write_generated_silence_wav(&source, SOURCE_SAMPLES);
    let destination = std::env::temp_dir().join(format!(
        "whisper-l01-long-import-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&destination).unwrap();
    let worker = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("whisper-worker-cpu.exe");
    let observation = MemoryObservation::start(&worker)
        .expect("Windows process sampler requires both trace paths");
    let mut app = ImportApplication::new(AsyncImportIo::start(worker));
    let job_id = JobId(((std::process::id() as u128) << 64) | 2);
    app.dispatch(AppCommand::StartImport {
        request: ImportRequest {
            job_id,
            generation: Generation::first(),
            source_path: source.to_string_lossy().into_owned(),
            source_sha256: None,
            source_samples: None,
            model_path: model.to_string_lossy().into_owned(),
            model_sha256: parse_hash(MODEL_HASH),
            destination: destination.to_string_lossy().into_owned(),
            config: JobConfig {
                language: LanguageChoice::Manual("fr".into()),
                compute: ComputeChoice::Cpu,
            },
        },
    })
    .unwrap();

    let started = Instant::now();
    let mut saw_cpu = false;
    loop {
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        saw_cpu |= view.message.as_deref() == Some("Moteur actif : CPU");
        if view
            .active_job
            .is_some_and(|(_, state)| state == JobState::Complete)
        {
            break;
        }
        if view
            .active_job
            .is_some_and(|(_, state)| state == JobState::Recoverable)
        {
            panic!("generated long import failed: {:?}", view.message);
        }
        assert!(
            started.elapsed() < Duration::from_secs(900),
            "eight-full-window real CPU qualification exceeded 15 minutes"
        );
        thread::sleep(Duration::from_millis(20));
    }
    observation.finish(9);
    assert!(saw_cpu, "worker never attested CPU");
    let entries = ArchiveStore::scan(&destination).unwrap();
    assert!(
        entries
            .iter()
            .any(|entry| entry.job_id == job_id && entry.complete)
    );
    let _ = std::fs::remove_file(&source);
    let _ = std::fs::remove_dir_all(&destination);
    eprintln!(
        "V-IMPORT LONG PASS: source_samples={SOURCE_SAMPLES}; native_windows=9 (8 full plus tail); elapsed_ms={}",
        started.elapsed().as_millis()
    );
}

#[cfg(all(windows, feature = "l01-memory-qualification"))]
fn write_generated_silence_wav(path: &std::path::Path, sample_count: u32) {
    use std::io::Write;

    let data_bytes = sample_count * 2;
    let mut file = std::fs::File::create(path).unwrap();
    file.write_all(b"RIFF").unwrap();
    file.write_all(&(36 + data_bytes).to_le_bytes()).unwrap();
    file.write_all(b"WAVEfmt ").unwrap();
    file.write_all(&16_u32.to_le_bytes()).unwrap();
    file.write_all(&1_u16.to_le_bytes()).unwrap();
    file.write_all(&1_u16.to_le_bytes()).unwrap();
    file.write_all(&16_000_u32.to_le_bytes()).unwrap();
    file.write_all(&32_000_u32.to_le_bytes()).unwrap();
    file.write_all(&2_u16.to_le_bytes()).unwrap();
    file.write_all(&16_u16.to_le_bytes()).unwrap();
    file.write_all(b"data").unwrap();
    file.write_all(&data_bytes.to_le_bytes()).unwrap();
    let silence = vec![0_u8; data_bytes as usize];
    file.write_all(&silence).unwrap();
}

#[test]
fn stop_during_native_inference_is_prompt_and_never_publishes_complete() {
    let (Some(source), Some(model)) = (
        std::env::var_os("WHISPER_L01_FIXTURE").map(PathBuf::from),
        std::env::var_os("WHISPER_L01_MODEL").map(PathBuf::from),
    ) else {
        eprintln!("V-IMPORT Stop-in-inference NOT RUN: fixture/model environment missing");
        return;
    };
    assert_eq!(hex(&sha256_file(&source).unwrap()), SOURCE_HASH);
    assert_eq!(hex(&sha256_file(&model).unwrap()), MODEL_HASH);
    let worker = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("whisper-worker-cpu.exe");
    assert!(worker.is_file(), "CPU worker missing: {}", worker.display());
    let destination = std::env::temp_dir().join(format!(
        "whisper-l01-stop-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&destination).unwrap();
    let mut app = ImportApplication::new(AsyncImportIo::start(worker.clone()));
    let job_id = JobId(((std::process::id() as u128) << 64) | 2);
    let generation = Generation::first();
    app.dispatch(AppCommand::StartImport {
        request: ImportRequest {
            job_id,
            generation,
            source_path: source.to_string_lossy().into_owned(),
            source_sha256: None,
            source_samples: None,
            model_path: model.to_string_lossy().into_owned(),
            model_sha256: parse_hash(MODEL_HASH),
            destination: destination.to_string_lossy().into_owned(),
            config: JobConfig {
                language: LanguageChoice::Manual("fr".into()),
                compute: ComputeChoice::Cpu,
            },
        },
    })
    .unwrap();

    let mut ready = false;
    let deadline = Instant::now() + Duration::from_secs(90);
    let mut previous_cpu = worker_cpu_time(&worker);
    let mut inference_cpu = Duration::ZERO;
    while Instant::now() < deadline {
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        ready |= view.message.as_deref() == Some("Moteur actif : CPU");
        let current_cpu = worker_cpu_time(&worker);
        if ready {
            inference_cpu += current_cpu.saturating_sub(previous_cpu);
            assert!(
                view.progress.is_none(),
                "worker reported progress before native inference returned: {view:?}"
            );
            if inference_cpu >= Duration::from_secs(2) {
                break;
            }
        }
        previous_cpu = current_cpu;
        thread::sleep(Duration::from_millis(100));
    }
    assert!(
        ready,
        "worker never became Ready; import did not reach inference"
    );
    assert!(
        inference_cpu >= Duration::from_secs(2),
        "worker did not accrue 2s CPU while Ready and before Progress; CPU={inference_cpu:?}"
    );

    let submitted = Instant::now();
    let view = app
        .dispatch(AppCommand::Stop { job_id, generation })
        .unwrap();
    let submission_time = submitted.elapsed();
    assert!(
        submission_time < Duration::from_millis(200),
        "Stop dispatch blocked for {submission_time:?}"
    );
    assert_eq!(view.active_job, Some((job_id, JobState::Cancelling)));

    let deadline = Instant::now() + Duration::from_secs(300);
    let terminal = loop {
        let view = app.dispatch(AppCommand::Refresh).unwrap();
        if let Some((
            _,
            state @ (JobState::Cancelled | JobState::Recoverable | JobState::Complete),
        )) = view.active_job
        {
            break (state, view);
        }
        assert!(
            Instant::now() < deadline,
            "worker did not reach terminal state after Stop: {view:?}"
        );
        thread::sleep(Duration::from_millis(100));
    };
    assert_eq!(
        terminal.0,
        JobState::Cancelled,
        "Stop terminal state must be truthful: {:?}",
        terminal.1
    );
    assert_ne!(terminal.0, JobState::Complete);
    let entries = ArchiveStore::scan(&destination).unwrap();
    assert!(
        entries.iter().all(|entry| !entry.complete),
        "Stop must not publish a complete archive: {entries:?}"
    );
    let _ = std::fs::remove_dir_all(destination);
}

fn worker_cpu_time(worker: &std::path::Path) -> Duration {
    let output = std::process::Command::new("powershell.exe")
        .args(["-NoProfile", "-Command"])
        .arg(format!(
            "$p = Get-Process -Name whisper-worker-cpu -ErrorAction SilentlyContinue | Where-Object {{ $_.Path -eq '{}' }} | Select-Object -First 1; if ($null -eq $p) {{ '0' }} else {{ [string]$p.CPU }}",
            worker.display().to_string().replace('\'', "''")
        ))
        .output()
        .expect("PowerShell is required to witness native worker CPU time on Windows");
    assert!(output.status.success(), "PowerShell CPU query failed");
    let seconds = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<f64>()
        .expect("worker CPU time query returned invalid output");
    Duration::from_secs_f64(seconds.max(0.0))
}

#[test]
fn real_worker_serves_correlated_bounded_decode_blocks() {
    let Some(source) = std::env::var_os("WHISPER_L01_FIXTURE").map(PathBuf::from) else {
        eprintln!("V-IMPORT decode block NOT RUN: set WHISPER_L01_FIXTURE");
        return;
    };
    assert_eq!(hex(&sha256_file(&source).unwrap()), SOURCE_HASH);
    let worker = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("whisper-worker-cpu.exe");
    assert!(worker.is_file(), "CPU worker missing: {}", worker.display());
    let mut decoder = DecoderProxy::new(ChildWorkerTransport::spawn(&worker).unwrap());
    for (index, (count, limit)) in [(1_u64, 1_usize), (80_000, 80_000)].into_iter().enumerate() {
        let request = DecodeRequest {
            source: SourceIdentity {
                source_id: source.to_string_lossy().into_owned(),
                sha256: parse_hash(SOURCE_HASH),
            },
            job_id: JobId(81),
            generation: Generation::first(),
            range: SourceRange::new(0, count, 16_000).unwrap(),
            max_samples: limit,
        };
        let blocks = decoder
            .decode(request.clone())
            .unwrap_or_else(|error| panic!("decode request {index} failed: {error:?}"));
        assert_eq!(blocks.len(), 1, "worker response must correlate to request");
        let block = &blocks[0];
        assert_eq!(block.job_id, request.job_id);
        assert_eq!(block.generation, request.generation);
        assert_eq!(block.block.range, request.range);
        assert_eq!(block.block.channels, 1);
        assert_eq!(block.block.samples.len(), count as usize);
    }
    let eof_request = DecodeRequest {
        source: SourceIdentity {
            source_id: source.to_string_lossy().into_owned(),
            sha256: parse_hash(SOURCE_HASH),
        },
        job_id: JobId(81),
        generation: Generation::first(),
        range: SourceRange::new(300_000, 300_100, 16_000).unwrap(),
        max_samples: 100,
    };
    assert!(
        decoder.decode(eof_request).unwrap().is_empty(),
        "the same worker transport must correlate an explicit EOF/None response"
    );
}

#[test]
fn decoder_rejects_invalid_window_bounds_before_transport_admission() {
    struct NoCallTransport(Rc<Cell<usize>>);
    impl WorkerTransport for NoCallTransport {
        fn request(&mut self, _: WorkerCommand) -> Result<(), PortError> {
            self.0.set(self.0.get() + 1);
            Ok(())
        }
        fn receive(&mut self) -> Result<Option<WorkerEvent>, PortError> {
            Ok(None)
        }
    }

    let calls = Rc::new(Cell::new(0));
    for (range, max_samples) in [
        (SourceRange::new(0, 1, 16_000).unwrap(), 0),
        (SourceRange::new(0, 80_001, 16_000).unwrap(), 80_001),
        (
            SourceRange {
                start_sample: 1,
                end_sample: 1,
                sample_rate_hz: 16_000,
            },
            1,
        ),
    ] {
        let request = DecodeRequest {
            source: SourceIdentity {
                source_id: "source.wav".into(),
                sha256: [4; 32],
            },
            job_id: JobId(101),
            generation: Generation::first(),
            range,
            max_samples,
        };
        let mut decoder = DecoderProxy::new(NoCallTransport(calls.clone()));
        assert_eq!(decoder.decode(request), Err(PortError::InvalidInput));
    }
    assert_eq!(calls.get(), 0, "invalid requests must not reach the worker");
}

#[cfg(windows)]
struct MemoryObservation {
    sampler: Child,
    trace_path: PathBuf,
    adapter_trace_path: PathBuf,
    samples_path: PathBuf,
    stop_path: PathBuf,
    finished: bool,
}

#[cfg(windows)]
impl MemoryObservation {
    fn start(_: &std::path::Path) -> Option<Self> {
        use std::os::windows::process::CommandExt;

        let trace_path = std::env::var_os("WHISPER_L01_MEMORY_TRACE").map(PathBuf::from)?;
        let adapter_trace_path = std::env::var_os("WHISPER_L01_ADAPTER_MEMORY_TRACE")
            .map(PathBuf::from)
            .expect("set adapter trace path whenever worker trace is enabled");
        if let Some(parent) = trace_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        if let Some(parent) = adapter_trace_path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let samples_path = trace_path.with_extension("process-samples.jsonl");
        let stop_path = trace_path.with_extension("sampler-stop");
        for path in [&trace_path, &adapter_trace_path, &samples_path, &stop_path] {
            let _ = std::fs::remove_file(path);
        }
        let script = r#"
$samplePath = $env:WHISPER_L01_PROCESS_SAMPLES
$stopPath = $env:WHISPER_L01_SAMPLER_STOP
$adapterTracePath = $env:WHISPER_L01_ADAPTER_TRACE
$workerTracePath = $env:WHISPER_L01_WORKER_TRACE
$testPid = [int]$env:WHISPER_L01_TEST_PID
$pipeQueryDone = $false
$workerPipeQueryDone = $false
$pipeApi = @'
using System;
using System.Runtime.InteropServices;
public static class L01PipeApi {
 [DllImport("kernel32.dll", SetLastError=true)] public static extern IntPtr OpenProcess(uint access, bool inherit, int pid);
 [DllImport("kernel32.dll")] public static extern IntPtr GetCurrentProcess();
 [DllImport("kernel32.dll", SetLastError=true)] public static extern bool DuplicateHandle(IntPtr sourceProcess, IntPtr sourceHandle, IntPtr targetProcess, out IntPtr targetHandle, uint access, bool inherit, uint options);
 [DllImport("kernel32.dll", SetLastError=true)] public static extern bool GetNamedPipeInfo(IntPtr pipe, out uint flags, out uint outBuffer, out uint inBuffer, out uint maxInstances);
 [DllImport("kernel32.dll")] public static extern bool CloseHandle(IntPtr handle);
}
'@
Add-Type $pipeApi
$writer = [System.IO.StreamWriter]::new($samplePath, $false, [System.Text.UTF8Encoding]::new($false))
try {
    while (-not (Test-Path -LiteralPath $stopPath)) {
        $now = [DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds() * 1000
        $hostProcess = Get-Process -Id $testPid -ErrorAction SilentlyContinue
        if ($null -ne $hostProcess) {
            $writer.WriteLine((ConvertTo-Json -Compress -InputObject ([ordered]@{ timestamp_unix_us=$now; role='import_test_host'; pid=$hostProcess.Id; private_bytes=$hostProcess.PrivateMemorySize64; working_set_bytes=$hostProcess.WorkingSet64; peak_working_set_bytes=$hostProcess.PeakWorkingSet64 })))
        }
        if (-not $pipeQueryDone -and (Test-Path -LiteralPath $adapterTracePath)) {
            $spawn = Get-Content -LiteralPath $adapterTracePath | ForEach-Object { try { $_ | ConvertFrom-Json } catch { $null } } | Where-Object { $_.stage -eq 'adapter.worker.spawned' } | Select-Object -First 1
            if ($null -ne $spawn -and $null -ne $spawn.details.stdin_pipe_handle) {
                $sourceProcess = [L01PipeApi]::OpenProcess(0x40, $false, $testPid)
                $currentProcess = [L01PipeApi]::GetCurrentProcess()
                $pipeResults = @()
                foreach ($pipeName in @('stdin_pipe_handle', 'stdout_pipe_handle')) {
                    $rawHandle = [IntPtr]([long]$spawn.details.$pipeName)
                    $duplicate = [IntPtr]::Zero
                    $duplicateOk = [L01PipeApi]::DuplicateHandle($sourceProcess, $rawHandle, $currentProcess, [ref]$duplicate, 0, $false, 2)
                    if ($duplicateOk) {
                        [uint32]$flags=0; [uint32]$outBuffer=0; [uint32]$inBuffer=0; [uint32]$instances=0
                        $queryOk = [L01PipeApi]::GetNamedPipeInfo($duplicate, [ref]$flags, [ref]$outBuffer, [ref]$inBuffer, [ref]$instances)
                        $pipeResults += [ordered]@{ pipe=$pipeName; query_succeeded=$queryOk; output_buffer_bytes=$outBuffer; input_buffer_bytes=$inBuffer; win32_error=$(if ($queryOk) { 0 } else { [Runtime.InteropServices.Marshal]::GetLastWin32Error() }) }
                        [void][L01PipeApi]::CloseHandle($duplicate)
                    } else { $pipeResults += [ordered]@{ pipe=$pipeName; query_succeeded=$false; output_buffer_bytes=$null; input_buffer_bytes=$null; win32_error=[Runtime.InteropServices.Marshal]::GetLastWin32Error() } }
                }
                if ($sourceProcess -ne [IntPtr]::Zero) { [void][L01PipeApi]::CloseHandle($sourceProcess) }
                $writer.WriteLine((ConvertTo-Json -Compress -InputObject ([ordered]@{ timestamp_unix_us=$now; role='kernel_pipe_inventory'; results=$pipeResults })))
                $pipeQueryDone = $true
            }
        }
        if (-not $workerPipeQueryDone -and (Test-Path -LiteralPath $workerTracePath)) {
            $stdinRecord = Get-Content -LiteralPath $workerTracePath | ForEach-Object { try { $_ | ConvertFrom-Json } catch { $null } } | Where-Object { $_.stage -eq 'worker.input.reader.created' } | Select-Object -First 1
            if ($null -ne $stdinRecord -and $null -ne $stdinRecord.details.stdin_read_pipe_handle) {
                $sourceWorker = [L01PipeApi]::OpenProcess(0x40, $false, [int]$stdinRecord.details.worker_pid)
                $currentProcess = [L01PipeApi]::GetCurrentProcess()
                $rawHandle = [IntPtr]([long]$stdinRecord.details.stdin_read_pipe_handle)
                $duplicate = [IntPtr]::Zero
                $duplicateOk = [L01PipeApi]::DuplicateHandle($sourceWorker, $rawHandle, $currentProcess, [ref]$duplicate, 0, $false, 2)
                if ($duplicateOk) {
                    [uint32]$flags=0; [uint32]$outBuffer=0; [uint32]$inBuffer=0; [uint32]$instances=0
                    $queryOk = [L01PipeApi]::GetNamedPipeInfo($duplicate, [ref]$flags, [ref]$outBuffer, [ref]$inBuffer, [ref]$instances)
                    $result = [ordered]@{ role='worker_stdin_kernel_pipe_inventory'; query_succeeded=$queryOk; output_buffer_bytes=$outBuffer; input_buffer_bytes=$inBuffer; win32_error=$(if ($queryOk) { 0 } else { [Runtime.InteropServices.Marshal]::GetLastWin32Error() }) }
                    [void][L01PipeApi]::CloseHandle($duplicate)
                } else {
                    $result = [ordered]@{ role='worker_stdin_kernel_pipe_inventory'; query_succeeded=$false; output_buffer_bytes=$null; input_buffer_bytes=$null; win32_error=[Runtime.InteropServices.Marshal]::GetLastWin32Error() }
                }
                if ($sourceWorker -ne [IntPtr]::Zero) { [void][L01PipeApi]::CloseHandle($sourceWorker) }
                $result['timestamp_unix_us'] = $now
                $writer.WriteLine((ConvertTo-Json -Compress -InputObject $result))
                $workerPipeQueryDone = $true
            }
        }
        $writer.Flush()
        Start-Sleep -Milliseconds 100
    }
} finally { $writer.Dispose() }
"#;
        let sampler = Command::new("powershell.exe")
            .args(["-NoProfile", "-Command", script])
            .env("WHISPER_L01_PROCESS_SAMPLES", &samples_path)
            .env("WHISPER_L01_SAMPLER_STOP", &stop_path)
            .env("WHISPER_L01_ADAPTER_TRACE", &adapter_trace_path)
            .env("WHISPER_L01_WORKER_TRACE", &trace_path)
            .env("WHISPER_L01_TEST_PID", std::process::id().to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(0x0800_0000)
            .spawn()
            .expect("PowerShell is required for Windows process-memory sampling");
        Some(Self {
            sampler,
            trace_path,
            adapter_trace_path,
            samples_path,
            stop_path,
            finished: false,
        })
    }

    fn finish(mut self, expected_windows: usize) {
        std::fs::File::create(&self.stop_path).unwrap();
        assert!(
            self.sampler.wait().unwrap().success(),
            "memory sampler failed"
        );
        self.finished = true;

        let trace = std::fs::read_to_string(&self.trace_path)
            .expect("feature-enabled worker must write stage inventory");
        let trace = trace
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .collect::<Vec<_>>();
        let adapter_trace = std::fs::read_to_string(&self.adapter_trace_path)
            .expect("feature-enabled adapter must write queue and IPC inventory");
        let adapter_trace = adapter_trace
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .collect::<Vec<_>>();
        let samples = std::fs::read_to_string(&self.samples_path)
            .expect("OS sampler must write process samples");
        let samples = samples
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .collect::<Vec<_>>();

        for stage in [
            "model_context.load.begin",
            "native.context.create.complete",
            "codec.ready",
            "codec.sample_buffer.live",
            "pcm.window.callback.begin",
            "inference.begin",
            "native.inference.complete",
            "ipc.serialize.buffer",
            "worker.finalized",
        ] {
            assert!(
                trace.iter().any(|record| record["stage"] == stage),
                "missing memory stage record {stage}"
            );
        }
        for stage in [
            "adapter.channels.created",
            "adapter.worker.spawned",
            "adapter.pending.created",
            "adapter.pending.after_worker_event",
            "adapter.ipc.serialize.buffer",
        ] {
            assert!(
                adapter_trace.iter().any(|record| record["stage"] == stage),
                "missing adapter memory stage record {stage}"
            );
        }
        let channels = adapter_trace
            .iter()
            .find(|record| record["stage"] == "adapter.channels.created")
            .unwrap();
        assert_eq!(channels["details"]["effect_channel_capacity"], 8);
        assert_eq!(channels["details"]["stop_channel_capacity"], 1);
        assert_eq!(channels["details"]["app_event_channel_capacity"], 21);
        let windows = trace
            .iter()
            .filter(|record| record["stage"] == "pcm.window.callback.begin")
            .collect::<Vec<_>>();
        assert_eq!(
            windows.len(),
            expected_windows,
            "unexpected WAV window count"
        );
        let mut expected_start = 0_u64;
        for record in &windows {
            let details = &record["details"];
            assert_eq!(
                details["source_start_sample"].as_u64(),
                Some(expected_start)
            );
            let count = details["outgoing_window_len"].as_u64().unwrap();
            assert!((1..=80_000).contains(&count));
            assert!(details["outgoing_window_capacity"].as_u64().unwrap() <= 80_000);
            assert!(
                details["simultaneous_pcm_window_capacity_bytes"]
                    .as_u64()
                    .unwrap()
                    <= 640_000
            );
            expected_start += count;
        }
        let total = trace
            .iter()
            .find(|record| record["stage"] == "codec.complete")
            .expect("codec completion record")["details"]["output_samples"]
            .as_u64()
            .unwrap();
        assert_eq!(
            expected_start, total,
            "windows cover each source sample once"
        );
        for record in trace
            .iter()
            .filter(|record| record["stage"] == "codec.sample_buffer.live")
        {
            let details = &record["details"];
            assert_eq!(
                details["sample_buffer_capacity_bytes"].as_u64().unwrap(),
                details["sample_buffer_capacity_samples"].as_u64().unwrap() * 4
            );
        }
        for record in trace
            .iter()
            .filter(|record| record["stage"] == "ipc.serialize.buffer")
        {
            assert!(
                record["details"]["frame_len_with_newline_bytes"]
                    .as_u64()
                    .unwrap()
                    <= 1_048_576
            );
        }
        let worker_samples = trace
            .iter()
            .filter(|record| record["details"]["process_memory"]["private_bytes"].is_number())
            .collect::<Vec<_>>();
        assert!(
            worker_samples.len() >= expected_windows * 3,
            "worker memory was not sampled at each inference stage"
        );
        let pipe_inventory = samples
            .iter()
            .find(|sample| sample["role"] == "kernel_pipe_inventory");
        if let Some(inventory) = pipe_inventory {
            eprintln!(
                "V-IMPORT-MEMORY kernel_pipe_inventory={}",
                inventory["results"]
            );
        } else {
            eprintln!(
                "V-IMPORT-MEMORY kernel_pipe_inventory=NOT MEASURED (sampler did not observe child pipe handles)"
            );
        }
        let worker_stdin_pipe = samples
            .iter()
            .find(|sample| sample["role"] == "worker_stdin_kernel_pipe_inventory")
            .expect("worker-side stdin handle must permit kernel pipe query");
        assert_eq!(worker_stdin_pipe["query_succeeded"], true);
        assert_eq!(worker_stdin_pipe["output_buffer_bytes"], 65_536);
        assert_eq!(worker_stdin_pipe["input_buffer_bytes"], 65_536);
        eprintln!(
            "V-IMPORT-MEMORY worker stdin kernel pipe={} bytes per direction",
            worker_stdin_pipe["input_buffer_bytes"]
        );
        for stage in [
            "worker.input.reader.created",
            "model_context.load.complete",
            "codec.ready",
        ] {
            let record = trace
                .iter()
                .find(|record| record["stage"] == stage)
                .unwrap();
            let measured = process_memory_for(record);
            let timestamp = record["elapsed_unix_us"].as_i64().unwrap();
            let sample_delta = (measured["sampled_at_unix_us"].as_i64().unwrap() - timestamp).abs();
            assert!(
                sample_delta <= 1_000_000,
                "no nearby process sample for {stage}"
            );
            eprintln!(
                "V-IMPORT-MEMORY stage={stage} delta_us={sample_delta} private_bytes={} working_set_bytes={} peak_working_set_bytes={}",
                measured["private_bytes"],
                measured["working_set_bytes"],
                measured["peak_working_set_bytes"]
            );
        }
        let state_destroyed = trace
            .iter()
            .filter(|record| record["stage"] == "native.state.destroyed")
            .collect::<Vec<_>>();
        assert_eq!(
            state_destroyed.len(),
            expected_windows,
            "each native WhisperState must have an explicit destruction marker"
        );
        let state_creation_begins = trace
            .iter()
            .filter(|record| record["stage"] == "native.state.create.begin")
            .collect::<Vec<_>>();
        let inference_completes = trace
            .iter()
            .filter(|record| record["stage"] == "native.inference.complete")
            .collect::<Vec<_>>();
        assert_eq!(state_creation_begins.len(), expected_windows);
        assert_eq!(inference_completes.len(), expected_windows);
        let mut post_destroy_private_bytes = Vec::with_capacity(state_destroyed.len());
        for (window_index, ((before, during), destroyed)) in state_creation_begins
            .iter()
            .zip(&inference_completes)
            .zip(&state_destroyed)
            .enumerate()
        {
            let before_sample = process_memory_for(before);
            let during_sample = process_memory_for(during);
            let after_sample = process_memory_for(destroyed);
            let before_at = before["elapsed_unix_us"].as_i64().unwrap();
            let during_at = during["elapsed_unix_us"].as_i64().unwrap();
            let destroyed_at = destroyed["elapsed_unix_us"].as_i64().unwrap();
            let before_delta =
                (before_sample["sampled_at_unix_us"].as_i64().unwrap() - before_at).abs();
            let during_delta =
                (during_sample["sampled_at_unix_us"].as_i64().unwrap() - during_at).abs();
            let after_delta =
                (after_sample["sampled_at_unix_us"].as_i64().unwrap() - destroyed_at).abs();
            assert!(
                before_delta <= 1_000_000,
                "no nearby pre-state sample for window {window_index}"
            );
            assert!(
                during_delta <= 1_000_000,
                "no nearby pre-drop sample for window {window_index}"
            );
            assert!(
                after_delta <= 1_000_000,
                "no nearby post-drop sample for window {window_index}"
            );
            let private_bytes = after_sample["private_bytes"].as_u64().unwrap();
            post_destroy_private_bytes.push(private_bytes);
            let samples = destroyed["details"]["pcm_samples"].as_u64().unwrap();
            assert!((1..=80_000).contains(&samples));
            eprintln!(
                "V-IMPORT-MEMORY inference window={} pcm_samples={} before_private_bytes={} inference_complete_private_bytes={} after_drop_private_bytes={} sample_deltas_us={}/{}/{}",
                window_index + 1,
                samples,
                before_sample["private_bytes"],
                during_sample["private_bytes"],
                private_bytes,
                before_delta,
                during_delta,
                after_delta
            );
        }
        let private_min = *post_destroy_private_bytes.iter().min().unwrap();
        let private_max = *post_destroy_private_bytes.iter().max().unwrap();
        let first_to_last_delta = *post_destroy_private_bytes.last().unwrap() as i128
            - *post_destroy_private_bytes.first().unwrap() as i128;
        let rising_steps = post_destroy_private_bytes
            .windows(2)
            .filter(|pair| pair[1] > pair[0])
            .count();
        let falling_steps = post_destroy_private_bytes
            .windows(2)
            .filter(|pair| pair[1] < pair[0])
            .count();
        eprintln!(
            "V-IMPORT-MEMORY trend=characterization_only post_destroy_private_min={} post_destroy_private_max={} first_to_last_delta={} increasing_steps={} decreasing_steps={}",
            private_min, private_max, first_to_last_delta, rising_steps, falling_steps
        );
        let host_samples = samples
            .iter()
            .filter(|sample| sample["role"] == "import_test_host")
            .collect::<Vec<_>>();
        for stage in [
            "adapter.channels.created",
            "adapter.pending.created",
            "adapter.pending.after_worker_event",
        ] {
            let record = adapter_trace
                .iter()
                .find(|record| record["stage"] == stage)
                .unwrap();
            let timestamp = record["elapsed_unix_us"].as_i64().unwrap();
            let nearest = host_samples
                .iter()
                .min_by_key(|sample| {
                    (sample["timestamp_unix_us"].as_i64().unwrap() - timestamp).abs()
                })
                .unwrap();
            eprintln!(
                "V-IMPORT-MEMORY stage={stage} host_private_bytes={} host_working_set_bytes={} host_peak_working_set_bytes={}",
                nearest["private_bytes"],
                nearest["working_set_bytes"],
                nearest["peak_working_set_bytes"]
            );
        }
        eprintln!(
            "V-IMPORT-MEMORY characterization only: worker_samples={} host_samples={} adapter_records={} trace={} adapter_trace={} process_samples={}; native allocator and kernel pipe capacities remain un-attributed",
            worker_samples.len(),
            host_samples.len(),
            adapter_trace.len(),
            self.trace_path.display(),
            self.adapter_trace_path.display(),
            self.samples_path.display()
        );
    }
}

#[cfg(windows)]
fn process_memory_for(record: &serde_json::Value) -> &serde_json::Value {
    let stage = record["stage"].as_str().unwrap();
    let sample = &record["details"]["process_memory"];
    assert!(
        sample["private_bytes"].is_number(),
        "no process sample recorded at {stage}: {sample}"
    );
    sample
}

#[cfg(windows)]
impl Drop for MemoryObservation {
    fn drop(&mut self) {
        if !self.finished {
            let _ = std::fs::File::create(&self.stop_path);
            let _ = self.sampler.kill();
            let _ = self.sampler.wait();
        }
    }
}

#[cfg(not(windows))]
struct MemoryObservation;

#[cfg(not(windows))]
impl MemoryObservation {
    fn start(_: &std::path::Path) -> Option<Self> {
        None
    }

    fn finish(self, _: usize) {}
}

fn parse_hash(text: &str) -> [u8; 32] {
    let mut output = [0_u8; 32];
    for (i, pair) in text.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        output[i] = (digit(pair[0]) << 4) | digit(pair[1]);
    }
    output
}
fn digit(value: u8) -> u8 {
    match value {
        b'0'..=b'9' => value - b'0',
        b'a'..=b'f' => value - b'a' + 10,
        _ => 0,
    }
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
