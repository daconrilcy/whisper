use std::time::{Duration, Instant};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
use whisper_adapters::supervisor::ProgressPhase;
use whisper_adapters::supervisor::{ProgressSupervisor, ProgressWatch, WorkerPaths};
use whisper_core::{BackendAttempt, ComputeChoice, ComputePolicyError};

static NEXT_DIR: AtomicU64 = AtomicU64::new(1);

fn paths() -> (PathBuf, WorkerPaths) {
    let root = std::env::temp_dir().join(format!(
        "whisper-worker-control-{}-{}",
        std::process::id(),
        NEXT_DIR.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&root).unwrap();
    let workers = WorkerPaths {
        cpu: root.join("whisper-worker-cpu.exe"),
        gpu: root.join("whisper-worker-gpu.exe"),
    };
    (root, workers)
}

#[test]
fn cpu_choice_uses_only_cpu_worker_even_when_gpu_exists() {
    let (root, workers) = paths();
    fs::write(&workers.cpu, b"cpu").unwrap();
    fs::write(&workers.gpu, b"gpu").unwrap();
    let selected = workers.select(ComputeChoice::Cpu).unwrap();
    assert_eq!(selected.backend, BackendAttempt::Cpu);
    assert_eq!(selected.path, workers.cpu);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn auto_prefers_gpu_and_uses_cpu_when_gpu_binary_is_absent() {
    let (root, workers) = paths();
    fs::write(&workers.cpu, b"cpu").unwrap();
    let selected = workers.select(ComputeChoice::Auto).unwrap();
    assert_eq!(selected.backend, BackendAttempt::Cpu);
    assert_eq!(selected.path, workers.cpu);
    fs::write(&workers.gpu, b"gpu").unwrap();
    let selected = workers.select(ComputeChoice::Auto).unwrap();
    assert_eq!(selected.backend, BackendAttempt::Gpu);
    assert_eq!(selected.path, workers.gpu);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn forced_gpu_fails_closed_when_worker_is_absent() {
    let (root, workers) = paths();
    fs::write(&workers.cpu, b"cpu").unwrap();
    assert_eq!(
        workers.select(ComputeChoice::Gpu).unwrap_err(),
        ComputePolicyError::GpuUnavailable
    );
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn missing_progress_warns_once_without_turning_into_a_stop() {
    let start = Instant::now();
    let mut watch = ProgressWatch::new(ProgressPhase::Inference, start);
    assert_eq!(watch.warning(start + Duration::from_secs(59)), None);
    assert_eq!(
        watch.warning(start + Duration::from_secs(60)),
        Some((ProgressPhase::Inference, 60_000))
    );
    assert_eq!(watch.warning(start + Duration::from_secs(61)), None);
    watch.observe(1, start + Duration::from_secs(62));
    assert_eq!(watch.warning(start + Duration::from_secs(121)), None);
    assert_eq!(
        watch.warning(start + Duration::from_secs(122)),
        Some((ProgressPhase::Inference, 60_000))
    );
}

#[test]
fn capture_progress_does_not_hide_a_stalled_inference_obligation() {
    let start = Instant::now();
    let mut monitor = ProgressSupervisor::starting(start);
    monitor.close(ProgressPhase::Preparing);
    monitor.open(ProgressPhase::Inference, start);
    monitor.open(ProgressPhase::Capturing, start);
    monitor.observe(
        ProgressPhase::Capturing,
        10_000,
        start + Duration::from_secs(50),
    );
    assert_eq!(
        monitor.take_warning(start + Duration::from_secs(60)),
        Some((ProgressPhase::Inference, 60_000))
    );
    assert_eq!(monitor.take_warning(start + Duration::from_secs(60)), None);
    assert_eq!(
        monitor.take_warning(start + Duration::from_secs(110)),
        Some((ProgressPhase::Capturing, 60_000))
    );
}
