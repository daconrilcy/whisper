#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct BackendRegistryEntry {
    pub(super) registry_index: u32,
    pub(super) gpu_or_igpu: bool,
    pub(super) name: String,
    pub(super) pci_id: Option<String>,
}

pub(super) fn validate_native_gpu_log(
    text: &str,
    expected_backend: &str,
    overflowed: bool,
) -> Result<(), String> {
    if overflowed {
        return Err("native GPU attestation log exceeded its bound".into());
    }
    let mut using = Vec::new();
    let mut failure = None;
    for line in text.lines() {
        let lower = line.to_ascii_lowercase();
        if lower.contains("failed to initialize") && lower.contains("backend") {
            failure = Some(line.trim().to_owned());
        }
        if lower.contains("no gpu found") {
            failure = Some(line.trim().to_owned());
        }
        if let Some(start) = line.find("using ")
            && let Some(end) = line[start + 6..].find(" backend")
        {
            using.push(line[start + 6..start + 6 + end].trim().to_owned());
        }
    }
    if let Some(failure) = failure {
        return Err(format!(
            "native GPU initialization reported failure: {failure}"
        ));
    }
    if using.len() != 1 || using.first().is_none_or(|name| name != expected_backend) {
        return Err(format!(
            "native GPU backend attestation missing or contradictory: expected {expected_backend}, observed {using:?}"
        ));
    }
    Ok(())
}

pub(super) fn whisper_gpu_position(
    entries: &[BackendRegistryEntry],
    target_registry_index: u32,
) -> Result<u32, String> {
    let mut position = 0_u32;
    for entry in entries {
        if !entry.gpu_or_igpu {
            continue;
        }
        if entry.registry_index == target_registry_index {
            return Ok(position);
        }
        position = position
            .checked_add(1)
            .ok_or("Whisper GPU index overflow")?;
    }
    Err(format!(
        "GGML registry device {target_registry_index} is not selectable by Whisper"
    ))
}

pub(super) fn validate_selected_cuda_registry_device(
    selected: &BackendRegistryEntry,
    expected_pci_id: &str,
) -> Result<(), String> {
    if !selected.gpu_or_igpu
        || !selected.name.starts_with("CUDA")
        || selected
            .pci_id
            .as_deref()
            .is_none_or(|pci| normalize_pci(pci) != normalize_pci(expected_pci_id))
    {
        return Err(
            "selected GGML registry position no longer identifies the CUDA PCI device".into(),
        );
    }
    Ok(())
}

pub(super) fn normalize_pci(value: &str) -> String {
    let normalized = value.trim().replace('-', ":").to_ascii_lowercase();
    normalized
        .strip_prefix("00000000:")
        .unwrap_or(&normalized)
        .to_owned()
}

#[derive(Clone, Debug)]
pub struct CudaDevice {
    pub ordinal: u32,
    pub luid: Option<u64>,
    pub pci_id: Option<String>,
    pub compute: Option<(i32, i32)>,
}

#[derive(Clone, Debug)]
pub struct GgmlDevice {
    pub ordinal: u32,
    pub pci_id: Option<String>,
    pub ready: bool,
}
