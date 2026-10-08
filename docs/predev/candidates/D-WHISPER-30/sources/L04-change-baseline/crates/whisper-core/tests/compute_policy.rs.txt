use whisper_core::{
    BackendAttempt, BackendDecision, ComputeChoice, ComputePolicy, ComputePolicyError,
};

#[test]
fn forced_cpu_never_selects_or_initializes_gpu() {
    assert_eq!(
        ComputePolicy::initial(ComputeChoice::Cpu, true),
        Ok(BackendAttempt::Cpu)
    );
    assert_eq!(
        ComputePolicy::after_failure(ComputeChoice::Cpu, BackendAttempt::Cpu),
        BackendDecision::Fail
    );
}

#[test]
fn auto_uses_gpu_when_available_and_allows_only_gpu_to_cpu_fallback() {
    assert_eq!(
        ComputePolicy::initial(ComputeChoice::Auto, true),
        Ok(BackendAttempt::Gpu)
    );
    assert_eq!(
        ComputePolicy::after_failure(ComputeChoice::Auto, BackendAttempt::Gpu),
        BackendDecision::RetryCpu
    );
    assert_eq!(
        ComputePolicy::validate_fallback(
            ComputeChoice::Auto,
            BackendAttempt::Gpu,
            BackendAttempt::Cpu
        ),
        Ok(())
    );
    assert_eq!(
        ComputePolicy::validate_fallback(
            ComputeChoice::Auto,
            BackendAttempt::Cpu,
            BackendAttempt::Gpu
        ),
        Err(ComputePolicyError::InvalidFallback)
    );
}

#[test]
fn forced_gpu_failure_waits_for_a_human_choice() {
    assert_eq!(
        ComputePolicy::initial(ComputeChoice::Gpu, true),
        Ok(BackendAttempt::Gpu)
    );
    assert_eq!(
        ComputePolicy::initial(ComputeChoice::Gpu, false),
        Err(ComputePolicyError::GpuUnavailable)
    );
    assert_eq!(
        ComputePolicy::after_failure(ComputeChoice::Gpu, BackendAttempt::Gpu),
        BackendDecision::AwaitUserChoice
    );
}
