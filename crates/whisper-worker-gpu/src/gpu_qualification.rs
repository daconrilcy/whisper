use whisper_core::ipc::{GpuQualificationResult, GpuQualificationTarget};

pub const BUILD_ARCHITECTURES: &str = env!("CMAKE_CUDA_ARCHITECTURES");

pub fn qualify(targets: &[GpuQualificationTarget]) -> Vec<GpuQualificationResult> {
    let cuda = crate::native_ffi::cuda_devices();
    let ggml = crate::native_ffi::ggml_devices();
    targets
        .iter()
        .map(|target| match &cuda {
            Ok(devices) => qualify_one(target, devices, &ggml, None),
            Err(error) => qualify_one(target, &[], &[], Some(error)),
        })
        .collect()
}

fn qualify_one(
    target: &GpuQualificationTarget,
    cuda: &[crate::native_ffi::CudaDevice],
    ggml: &[crate::native_ffi::GgmlDevice],
    driver_error: Option<&str>,
) -> GpuQualificationResult {
    let mut identity_match = "Missing";
    let mut support = "Undetermined";
    let mut availability = "NotVerified";
    let mut cause = driver_error.map(str::to_owned);
    let windows_identity = target.identity.to_ascii_uppercase();
    let uniquely_joined_windows_device = target.windows_rank.is_some() && target.luid.is_some();
    if uniquely_joined_windows_device
        && windows_identity.starts_with("PCI\\VEN_8086&DEV_")
        && BUILD_ARCHITECTURES == "61-real"
    {
        return GpuQualificationResult {
            identity: target.identity.clone(),
            windows_rank: target.windows_rank,
            luid: target.luid,
            pci_id: target.pci_id.clone(),
            cuda_ordinal: None,
            ggml_ordinal: None,
            // This is a build capability exclusion based on Windows identity,
            // not a successful Windows-to-CUDA device join.
            identity_match: "Missing".into(),
            support: "ConfirmedUnsupported".into(),
            availability: "NotVerified".into(),
            cause: Some("Intel adapter excluded: the confirmed worker backend is CUDA-only".into()),
        };
    }
    let matches = match target.luid {
        Some(luid) => cuda
            .iter()
            .filter(|device| device.luid == Some(luid))
            .collect::<Vec<_>>(),
        None => {
            cause = Some("Windows adapter LUID is missing".into());
            Vec::new()
        }
    };
    if driver_error.is_some() {
        availability = "Unavailable";
    } else if matches.len() > 1 {
        identity_match = "Ambiguous";
        cause = Some("Multiple CUDA devices match the Windows LUID".into());
    } else if let Some(device) = matches.first() {
        if let Some(pci) = &device.pci_id {
            let ggml_matches = ggml
                .iter()
                .filter(|candidate| {
                    candidate
                        .pci_id
                        .as_ref()
                        .is_some_and(|id| normalize_pci(id) == normalize_pci(pci))
                })
                .collect::<Vec<_>>();
            if ggml_matches.len() > 1 {
                identity_match = "Ambiguous";
                cause = Some("Multiple GGML CUDA devices match the PCI identity".into());
            } else if let Some(engine) = ggml_matches.first() {
                identity_match = "Unique";
                if let Some((major, minor)) = device.compute {
                    if major == 6 && minor >= 1 {
                        support = "ConfirmedSupported";
                    } else {
                        support = "ConfirmedUnsupported";
                    }
                }
                if engine.ready && support == "ConfirmedSupported" {
                    availability = "Available";
                } else if !engine.ready {
                    availability = "Unavailable";
                    cause = Some(
                        "GGML CUDA backend initialization or bounded allocation failed".into(),
                    );
                }
            } else {
                cause = Some("No GGML CUDA device matches the CUDA PCI identity".into());
            }
        } else {
            cause = Some("CUDA PCI identity is unavailable".into());
        }
    }
    GpuQualificationResult {
        identity: target.identity.clone(),
        windows_rank: target.windows_rank,
        luid: target.luid,
        pci_id: matches.first().and_then(|device| device.pci_id.clone()),
        cuda_ordinal: matches.first().map(|device| device.ordinal),
        ggml_ordinal: matches.first().and_then(|device| {
            device.pci_id.as_ref().and_then(|pci| {
                ggml.iter()
                    .find(|candidate| {
                        candidate
                            .pci_id
                            .as_ref()
                            .is_some_and(|id| normalize_pci(id) == normalize_pci(pci))
                    })
                    .map(|engine| engine.ordinal)
            })
        }),
        identity_match: identity_match.into(),
        support: support.into(),
        availability: availability.into(),
        cause,
    }
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
    use crate::native_ffi::{CudaDevice, GgmlDevice};

    #[test]
    fn unique_luid_pci_ggml_join_requires_covered_architecture_and_ready_backend() {
        let target = GpuQualificationTarget {
            generation: 1,
            identity: "controlled".into(),
            name: Some("GPU".into()),
            state: Some("Ready".into()),
            presence: "Current".into(),
            windows_rank: Some(0),
            luid: Some(1),
            pci_id: Some("0000:01:00.0".into()),
        };
        let response = qualify_one(
            &target,
            &[CudaDevice {
                ordinal: 0,
                luid: Some(1),
                pci_id: Some("0000:01:00.0".into()),
                compute: Some((6, 1)),
            }],
            &[GgmlDevice {
                ordinal: 0,
                pci_id: Some("0000:01:00.0".into()),
                ready: true,
            }],
            None,
        );
        assert_eq!(response.identity_match, "Unique");
        assert_eq!(response.support, "ConfirmedSupported");
        assert_eq!(response.availability, "Available");
        assert_eq!(BUILD_ARCHITECTURES, "61-real");
        let response = qualify_one(
            &target,
            &[CudaDevice {
                ordinal: 0,
                luid: Some(1),
                pci_id: Some("0000:01:00.0".into()),
                compute: Some((8, 0)),
            }],
            &[GgmlDevice {
                ordinal: 0,
                pci_id: Some("0000:01:00.0".into()),
                ready: true,
            }],
            None,
        );
        assert_eq!(response.support, "ConfirmedUnsupported");
        assert_ne!(response.availability, "Available");
    }

    #[test]
    fn duplicate_or_missing_join_endpoints_never_become_supported() {
        let target = GpuQualificationTarget {
            generation: 1,
            identity: "controlled".into(),
            name: None,
            state: None,
            presence: "Current".into(),
            windows_rank: Some(0),
            luid: Some(9),
            pci_id: None,
        };
        let cuda = CudaDevice {
            ordinal: 0,
            luid: Some(9),
            pci_id: Some("0000:01:00.0".into()),
            compute: Some((6, 1)),
        };
        let engine = GgmlDevice {
            ordinal: 0,
            pci_id: Some("0000:01:00.0".into()),
            ready: true,
        };
        let ambiguous = qualify_one(
            &target,
            &[cuda.clone(), cuda.clone()],
            std::slice::from_ref(&engine),
            None,
        );
        assert_eq!(ambiguous.identity_match, "Ambiguous");
        assert_eq!(ambiguous.support, "Undetermined");
        let missing = qualify_one(&target, &[], &[], None);
        assert_eq!(missing.identity_match, "Missing");
        assert_eq!(missing.support, "Undetermined");
        let ambiguous_engine = qualify_one(
            &target,
            std::slice::from_ref(&cuda),
            &[engine.clone(), engine],
            None,
        );
        assert_eq!(ambiguous_engine.identity_match, "Ambiguous");
        assert_eq!(ambiguous_engine.support, "Undetermined");
    }

    #[test]
    fn missing_driver_or_required_symbol_is_unavailable_without_support_claim() {
        let target = GpuQualificationTarget {
            generation: 1,
            identity: "controlled".into(),
            name: None,
            state: None,
            presence: "Current".into(),
            windows_rank: Some(0),
            luid: Some(9),
            pci_id: None,
        };
        let result = qualify_one(
            &target,
            &[],
            &[],
            Some("CUDA symbol cuDeviceGetLuid unavailable"),
        );
        assert_eq!(result.availability, "Unavailable");
        assert_eq!(result.support, "Undetermined");
        assert!(result.cause.unwrap().contains("cuDeviceGetLuid"));
    }

    #[test]
    fn intel_exclusion_uses_windows_identity_and_build_capability_without_cuda_join() {
        let intel = GpuQualificationTarget {
            generation: 1,
            identity: "PCI\\VEN_8086&DEV_A780".into(),
            name: Some("Intel UHD".into()),
            state: Some("Ready".into()),
            presence: "Current".into(),
            windows_rank: Some(1),
            luid: Some(8),
            pci_id: None,
        };
        let result = qualify_one(&intel, &[], &[], None);
        assert_eq!(result.identity_match, "Missing");
        assert_eq!(result.support, "ConfirmedUnsupported");

        let driver_absent = qualify_one(&intel, &[], &[], Some("CUDA driver unavailable"));
        assert_eq!(driver_absent.identity_match, "Missing");
        assert_eq!(driver_absent.support, "ConfirmedUnsupported");

        let absent_nvidia = GpuQualificationTarget {
            generation: 1,
            identity: "PCI\\VEN_10DE&DEV_1B06".into(),
            ..intel.clone()
        };
        let result = qualify_one(&absent_nvidia, &[], &[], None);
        assert_eq!(result.support, "Undetermined");

        let ambiguous_windows_identity = GpuQualificationTarget {
            generation: 1,
            windows_rank: None,
            ..intel
        };
        let result = qualify_one(&ambiguous_windows_identity, &[], &[], None);
        assert_eq!(result.identity_match, "Missing");
        assert_eq!(result.support, "Undetermined");
    }
}
