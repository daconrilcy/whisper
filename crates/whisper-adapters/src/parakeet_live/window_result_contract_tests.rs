use super::*;

fn worker_identity() -> whisper_core::parakeet_ipc::WorkerIdentity {
    whisper_core::parakeet_ipc::WorkerIdentity {
        model_sha256: "model-sha".into(),
        runtime_sha256: "runtime-sha".into(),
        build_sha256: "worker-sha".into(),
        profile: "import-single-pass".into(),
    }
}

fn result_identity<'a>(
    identity: &'a whisper_core::parakeet_ipc::WorkerIdentity,
) -> WindowResultIdentity<'a> {
    WindowResultIdentity {
        request_id: 7,
        job_id: "0000000000000000000000000000002a",
        generation: 3,
        attempt: 7,
        identity,
        source_sha256: "0123456789abcdef",
        duration_samples: 64_000,
    }
}

#[test]
fn correlated_cpu_result_does_not_require_native_call_marker() {
    let identity = worker_identity();
    let result = result_identity(&identity);
    assert!(window_result_matches(result, result, false, false));
}

#[test]
fn expected_gpu_result_requires_native_call_marker() {
    let identity = worker_identity();
    let result = result_identity(&identity);
    assert!(!window_result_matches(result, result, true, false));
    assert!(window_result_matches(result, result, true, true));
}

#[test]
fn foreign_result_is_rejected_even_without_gpu_marker_requirement() {
    let identity = worker_identity();
    let expected = result_identity(&identity);
    let foreign = WindowResultIdentity {
        request_id: expected.request_id + 1,
        ..expected
    };
    assert!(!window_result_matches(expected, foreign, false, false));
}

#[test]
fn configured_compute_selects_expected_ready_backend() {
    assert_eq!(expected_live_worker_backend(ComputeChoice::Cpu), "CPU");
    assert_eq!(expected_live_worker_backend(ComputeChoice::Auto), "CPU");
    assert_eq!(expected_live_worker_backend(ComputeChoice::Gpu), "CUDA0");
}
