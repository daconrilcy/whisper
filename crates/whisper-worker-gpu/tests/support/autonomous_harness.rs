#[path = "../../../whisper-worker-common/tests/support/native_boundary.rs"]
mod native_boundary;

#[path = "../../src/decoder.rs"]
mod decoder;
#[path = "../../src/diagnostics.rs"]
mod diagnostics;
#[path = "../../src/engine_cache.rs"]
mod engine_cache;
#[path = "../../src/gpu_attestation.rs"]
mod gpu_attestation;
#[path = "../../src/gpu_qualification.rs"]
mod gpu_qualification;
#[path = "../../src/integrity.rs"]
mod integrity;
#[path = "../../src/ipc.rs"]
mod ipc;
#[path = "../../src/worker.rs"]
mod worker;

mod native_engine {
    pub(crate) struct GpuEngine;

    impl GpuEngine {
        pub(crate) fn load(
            _model_path: &str,
            _device_ordinal: u32,
            _expected_backend: String,
        ) -> Result<Self, String> {
            super::native_boundary::forbid("GpuEngine::load")
        }

        pub(crate) fn attest(&self) -> Result<(), String> {
            super::native_boundary::forbid("GpuEngine::attest")
        }

        pub(crate) fn matches_device(&self, _ordinal: u32, _backend: &str) -> bool {
            super::native_boundary::forbid("GpuEngine::matches_device")
        }
    }
}

mod native_ffi {
    #[path = "../../../src/native_ffi/pure.rs"]
    mod pure;
    use self::pure::{
        BackendRegistryEntry, normalize_pci, validate_native_gpu_log,
        validate_selected_cuda_registry_device, whisper_gpu_position,
    };
    pub use self::pure::{CudaDevice, GgmlDevice};

    #[cfg(test)]
    #[path = "../../../src/native_ffi/tests.rs"]
    mod tests;

    pub fn cuda_devices() -> Result<Vec<CudaDevice>, String> {
        crate::native_boundary::forbid("native_ffi::cuda_devices")
    }

    pub fn ggml_devices() -> Vec<GgmlDevice> {
        crate::native_boundary::forbid("native_ffi::ggml_devices")
    }

    pub fn whisper_gpu_device_index(
        _target_registry_index: u32,
        _expected_pci_id: &str,
    ) -> Result<(u32, String), String> {
        crate::native_boundary::forbid("native_ffi::whisper_gpu_device_index")
    }

    pub fn verify_windows_identity(
        _candidate: &whisper_core::gpu_inventory::GpuRuntimeIdentity,
    ) -> Result<(), String> {
        crate::native_boundary::forbid("native_ffi::verify_windows_identity")
    }
}

mod session {
    use crate::{diagnostics::emit_failure, ipc::Incoming};
    use std::{
        io,
        sync::mpsc::{self, Receiver},
    };
    use whisper_core::ipc::{
        IPC_PROTOCOL_VERSION, IpcEnvelope, WorkerCommand, WorkerErrorCode, WorkerEvent,
    };
    use whisper_core::{Generation, JobId};

    #[path = "../../../src/session/control.rs"]
    mod control;

    pub(crate) fn run_session(
        _rx: &std::sync::mpsc::Receiver<crate::ipc::Incoming>,
        _output: &mut impl std::io::Write,
        _first: whisper_core::ipc::IpcEnvelope<whisper_core::ipc::WorkerCommand>,
        _cached_engine: &mut Option<super::engine_cache::CachedGpuEngine>,
    ) -> Result<(), String> {
        crate::native_boundary::forbid("session::run_session")
    }
}

#[test]
fn negative_probe_native_port_terminates_process_with_78() {
    let _ = native_ffi::cuda_devices();
}
