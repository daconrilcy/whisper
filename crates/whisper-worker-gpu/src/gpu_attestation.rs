use whisper_core::gpu_inventory::GpuRuntimeIdentity;

pub fn attest_native_identity(
    candidate: &GpuRuntimeIdentity,
    cuda: &[crate::native_ffi::CudaDevice],
    ggml: &[crate::native_ffi::GgmlDevice],
) -> Result<(), String> {
    if candidate.pnp_instance_id.trim().is_empty() {
        return Err("candidate PnP identity is empty".into());
    }
    let cuda_matches = cuda
        .iter()
        .filter(|device| {
            device.ordinal == candidate.cuda_ordinal
                && device.luid == Some(candidate.luid)
                && device
                    .pci_id
                    .as_deref()
                    .is_some_and(|pci| normalize_pci(pci) == normalize_pci(&candidate.pci_id))
        })
        .count();
    if cuda_matches != 1 {
        return Err("candidate CUDA ordinal, LUID, or PCI identity changed".into());
    }
    let ggml_matches = ggml
        .iter()
        .filter(|device| {
            device.ordinal == candidate.ggml_ordinal
                && device.ready
                && device
                    .pci_id
                    .as_deref()
                    .is_some_and(|pci| normalize_pci(pci) == normalize_pci(&candidate.pci_id))
        })
        .count();
    if ggml_matches != 1 {
        return Err("candidate GGML ordinal or PCI identity changed".into());
    }
    Ok(())
}

fn normalize_pci(value: &str) -> String {
    let normalized = value.trim().replace('-', ":").to_ascii_lowercase();
    normalized
        .strip_prefix("00000000:")
        .unwrap_or(&normalized)
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate() -> GpuRuntimeIdentity {
        GpuRuntimeIdentity {
            generation: 12,
            pnp_instance_id: "PCI\\VEN_10DE&DEV_1234".into(),
            dxgi_rank: 3,
            luid: 0x1234,
            pci_id: "0000:01:00.0".into(),
            cuda_ordinal: 1,
            ggml_ordinal: 4,
        }
    }

    fn cuda(ordinal: u32, luid: u64) -> crate::native_ffi::CudaDevice {
        crate::native_ffi::CudaDevice {
            ordinal,
            luid: Some(luid),
            pci_id: Some("0000:01:00.0".into()),
            compute: Some((6, 1)),
        }
    }

    fn ggml(ordinal: u32) -> crate::native_ffi::GgmlDevice {
        crate::native_ffi::GgmlDevice {
            ordinal,
            pci_id: Some("0000:01:00.0".into()),
            ready: true,
        }
    }

    #[test]
    fn reattestation_requires_exact_cuda_and_ggml_ordinals_and_identity() {
        assert!(attest_native_identity(&candidate(), &[cuda(1, 0x1234)], &[ggml(4)]).is_ok());
        assert!(attest_native_identity(&candidate(), &[cuda(0, 0x1234)], &[ggml(4)]).is_err());
        assert!(attest_native_identity(&candidate(), &[cuda(1, 0x1234)], &[ggml(3)]).is_err());
        assert!(attest_native_identity(&candidate(), &[cuda(1, 0x9999)], &[ggml(4)]).is_err());
    }
}
