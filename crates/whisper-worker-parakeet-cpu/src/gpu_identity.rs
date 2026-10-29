#[cfg(feature = "parakeet-cuda")]
use sha2::{Digest, Sha256};
use std::path::Path;
#[cfg(feature = "parakeet-cuda")]
use std::path::PathBuf;
use whisper_core::parakeet_ipc::{ExpectedGpu, GpuEvidence};

pub(crate) fn worker_gpu_identity(backend: &str) -> Result<Option<ExpectedGpu>, String> {
    #[cfg(feature = "parakeet-cuda")]
    {
        if backend != "CUDA0" {
            return Err(format!("BackendMismatch: CUDA feature reported {backend}"));
        }
        return query_single_nvidia_gpu().map(Some);
    }
    #[cfg(not(feature = "parakeet-cuda"))]
    {
        if backend != "CPU" {
            return Err(format!("BackendMismatch: CPU worker reported {backend}"));
        }
        Ok(None)
    }
}

pub(crate) fn configured_backend() -> &'static str {
    #[cfg(feature = "parakeet-cuda")]
    {
        "CUDA0"
    }
    #[cfg(not(feature = "parakeet-cuda"))]
    {
        "CPU"
    }
}

pub(crate) fn build_gpu_evidence(
    runtime: &Path,
    backend: &str,
    gpu: Option<&ExpectedGpu>,
) -> Result<Option<GpuEvidence>, String> {
    #[cfg(feature = "parakeet-cuda")]
    {
        let gpu = gpu.ok_or("GpuEvidenceMissing: CUDA device identity is unavailable")?;
        if backend != "CUDA0" {
            return Err(format!("BackendMismatch: expected CUDA0, got {backend}"));
        }
        let log_path = std::env::var_os("PARAKEET_L3_ASR_LOG")
            .map(PathBuf::from)
            .ok_or("GpuLogMissing: PARAKEET_L3_ASR_LOG is required")?;
        let parent = log_path
            .parent()
            .ok_or("GpuLogPath: log path has no parent")?
            .canonicalize()
            .map_err(|error| format!("GpuLogPath: {error}"))?;
        if !parent.starts_with(runtime) {
            return Err("GpuLogPath: captured log is outside the runtime root".into());
        }
        let canonical_log = parent.join(
            log_path
                .file_name()
                .ok_or("GpuLogPath: log path has no file name")?,
        );
        let bytes =
            std::fs::read(&canonical_log).map_err(|error| format!("GpuLogRead: {error}"))?;
        let has_cuda_status = bytes.split(|byte| *byte == b'\n').any(|line| {
            std::str::from_utf8(line)
                .ok()
                .is_some_and(|line| line.starts_with("[asr]") && line.contains("backend=CUDA0"))
        });
        if !has_cuda_status {
            return Err("GpuLogMissing: captured log lacks [asr] backend=CUDA0".into());
        }
        let asr_log_sha256 = format!("{:x}", Sha256::digest(&bytes));
        return Ok(Some(GpuEvidence {
            backend: backend.to_owned(),
            pci_bus_id: gpu.pci_bus_id.clone(),
            uuid: gpu.uuid.clone(),
            pid: std::process::id(),
            asr_log_sha256,
        }));
    }
    #[cfg(not(feature = "parakeet-cuda"))]
    {
        let _ = (runtime, backend, gpu);
        Ok(None)
    }
}

pub(crate) fn validate_expected_gpu(
    backend: &str,
    expected: Option<&ExpectedGpu>,
    startup_identity: Option<&ExpectedGpu>,
) -> Result<(), (&'static str, String)> {
    #[cfg(feature = "parakeet-cuda")]
    {
        if backend != "CUDA0" {
            return Err(("BackendMismatch", format!("expected CUDA0, got {backend}")));
        }
        let expected = expected.ok_or((
            "GpuIdentityRequired",
            "CUDA request must include expected PCI bus ID and UUID".into(),
        ))?;
        let actual = query_single_nvidia_gpu().map_err(|error| ("GpuInventoryFailed", error))?;
        if startup_identity != Some(&actual)
            || expected.pci_bus_id != actual.pci_bus_id
            || !expected.uuid.eq_ignore_ascii_case(&actual.uuid)
        {
            return Err((
                "WrongGpuDevice",
                format!(
                    "expected {} / {}, current device is {} / {}",
                    expected.pci_bus_id, expected.uuid, actual.pci_bus_id, actual.uuid
                ),
            ));
        }
        Ok(())
    }
    #[cfg(not(feature = "parakeet-cuda"))]
    {
        let _ = startup_identity;
        if backend != "CPU" || expected.is_some() {
            return Err((
                "GpuUnavailable",
                "CPU worker cannot accept a GPU identity".into(),
            ));
        }
        Ok(())
    }
}

#[cfg(feature = "parakeet-cuda")]
fn query_single_nvidia_gpu() -> Result<ExpectedGpu, String> {
    let output = std::process::Command::new("nvidia-smi")
        .args(["--query-gpu=count,name,pci.bus_id,uuid", "--format=csv"])
        .output()
        .map_err(|error| format!("GpuInventory: unable to start nvidia-smi: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "GpuInventory: nvidia-smi exited with {}",
            output.status
        ));
    }
    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| format!("GpuInventory: nvidia-smi output is not UTF-8: {error}"))?;
    parse_single_nvidia_gpu(&stdout)
}

#[cfg(feature = "parakeet-cuda")]
fn parse_single_nvidia_gpu(stdout: &str) -> Result<ExpectedGpu, String> {
    let rows = stdout
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.trim_start().starts_with("count,"))
        .collect::<Vec<_>>();
    if rows.len() != 1 {
        return Err(format!(
            "GpuInventory: expected one NVIDIA GPU row, found {}",
            rows.len()
        ));
    }
    let fields = rows[0].split(',').map(str::trim).collect::<Vec<_>>();
    if fields.len() != 4 || fields[0] != "1" || fields[2].is_empty() || fields[3].is_empty() {
        return Err(format!(
            "GpuInventory: malformed or non-unique NVIDIA GPU row: {}",
            rows[0]
        ));
    }
    Ok(ExpectedGpu {
        pci_bus_id: fields[2].to_owned(),
        uuid: fields[3].to_owned(),
    })
}

#[cfg(all(test, feature = "parakeet-cuda"))]
mod tests;
