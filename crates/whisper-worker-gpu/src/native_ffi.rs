#![allow(unsafe_code)]
#![deny(unsafe_op_in_unsafe_fn)]

use std::sync::{Mutex, OnceLock};

const MAX_NATIVE_LOG_BYTES: usize = 16 * 1024;

#[derive(Default)]
struct NativeLogPhase {
    active: bool,
    bytes: usize,
    text: String,
    overflowed: bool,
}

static NATIVE_LOG_PHASE: OnceLock<Mutex<NativeLogPhase>> = OnceLock::new();

fn native_log_phase() -> &'static Mutex<NativeLogPhase> {
    NATIVE_LOG_PHASE.get_or_init(|| Mutex::new(NativeLogPhase::default()))
}

pub fn begin_native_gpu_phase() -> Result<(), String> {
    install_native_log_callback();
    let mut phase = native_log_phase()
        .lock()
        .map_err(|_| "native GPU log collector is poisoned".to_owned())?;
    phase.active = true;
    phase.bytes = 0;
    phase.text.clear();
    phase.overflowed = false;
    Ok(())
}

pub fn finish_native_gpu_phase(expected_backend: &str) -> Result<(), String> {
    let mut phase = native_log_phase()
        .lock()
        .map_err(|_| "native GPU log collector is poisoned".to_owned())?;
    phase.active = false;
    if phase.overflowed {
        return Err("native GPU attestation log exceeded its bound".into());
    }
    validate_native_gpu_log(&phase.text, expected_backend, false)
}

fn validate_native_gpu_log(
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

fn install_native_log_callback() {
    static INSTALLED: OnceLock<()> = OnceLock::new();
    INSTALLED.get_or_init(|| {
        // SAFETY: callback is process-lifetime, uses bounded storage, and never unwinds across C.
        unsafe {
            whisper_rs::whisper_rs_sys::whisper_log_set(
                Some(native_log_callback),
                std::ptr::null_mut(),
            );
        }
    });
}

unsafe extern "C" fn native_log_callback(
    _level: whisper_rs::whisper_rs_sys::ggml_log_level,
    text: *const std::ffi::c_char,
    _user_data: *mut std::ffi::c_void,
) {
    if text.is_null() {
        return;
    }
    // SAFETY: native callback guarantees a valid NUL-terminated message for the callback duration.
    let message = unsafe { std::ffi::CStr::from_ptr(text) }.to_bytes();
    let Ok(mut phase) = native_log_phase().lock() else {
        return;
    };
    if !phase.active {
        return;
    }
    if message.len() > MAX_NATIVE_LOG_BYTES.saturating_sub(phase.bytes) {
        phase.overflowed = true;
        return;
    }
    phase.bytes += message.len();
    phase.text.push_str(&String::from_utf8_lossy(message));
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct BackendRegistryEntry {
    registry_index: u32,
    gpu_or_igpu: bool,
    name: String,
    pci_id: Option<String>,
}

fn whisper_gpu_position(
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

fn validate_selected_cuda_registry_device(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_gpu_attestation_requires_a_fresh_matching_log() {
        assert!(validate_native_gpu_log("", "CUDA0", false).is_err());
        assert!(
            validate_native_gpu_log(
                "whisper_backend_init_gpu: using CUDA0 backend\n",
                "CUDA0",
                false
            )
            .is_ok()
        );
        assert!(
            validate_native_gpu_log(
                "whisper_backend_init_gpu: using CUDA1 backend\n",
                "CUDA0",
                false
            )
            .is_err()
        );
    }

    #[test]
    fn native_gpu_attestation_rejects_failure_after_using_backend() {
        let lines = "whisper_backend_init_gpu: using CUDA0 backend\nwhisper_backend_init_gpu: failed to initialize CUDA0 backend\n";
        assert!(validate_native_gpu_log(lines, "CUDA0", false).is_err());
    }

    #[test]
    fn whisper_position_counts_gpu_and_integrated_gpu_not_cuda_ordinals() {
        let registry = [
            BackendRegistryEntry {
                registry_index: 0,
                gpu_or_igpu: false,
                name: "CPU".into(),
                pci_id: None,
            },
            BackendRegistryEntry {
                registry_index: 1,
                gpu_or_igpu: true,
                name: "CUDA0".into(),
                pci_id: Some("0000:00:00.0".into()),
            },
            BackendRegistryEntry {
                registry_index: 2,
                gpu_or_igpu: true,
                name: "IGPU0".into(),
                pci_id: Some("0000:00:01.0".into()),
            },
            BackendRegistryEntry {
                registry_index: 5,
                gpu_or_igpu: true,
                name: "CUDA1".into(),
                pci_id: Some("0000:01:00.0".into()),
            },
        ];
        assert_eq!(whisper_gpu_position(&registry, 5).unwrap(), 2);
        assert!(whisper_gpu_position(&registry, 0).is_err());
        assert!(validate_selected_cuda_registry_device(&registry[3], "0000:01:00.0").is_ok());
        assert!(validate_selected_cuda_registry_device(&registry[3], "0000:02:00.0").is_err());
        assert!(validate_selected_cuda_registry_device(&registry[2], "0000:00:01.0").is_err());
    }
}

pub fn whisper_gpu_device_index(
    target_registry_index: u32,
    expected_pci_id: &str,
) -> Result<(u32, String), String> {
    use std::ffi::CStr;
    use whisper_rs::whisper_rs_sys as sys;
    let mut entries = Vec::new();
    // SAFETY: registry devices and names are owned by GGML and remain valid for this enumeration.
    unsafe {
        for index in 0..sys::ggml_backend_dev_count() {
            let device = sys::ggml_backend_dev_get(index);
            if device.is_null() {
                continue;
            }
            let kind = sys::ggml_backend_dev_type(device);
            let name = sys::ggml_backend_dev_name(device);
            if name.is_null() {
                continue;
            }
            let mut props = std::mem::zeroed();
            sys::ggml_backend_dev_get_props(device, &mut props);
            let pci_id = (!props.device_id.is_null()).then(|| {
                CStr::from_ptr(props.device_id)
                    .to_string_lossy()
                    .into_owned()
            });
            entries.push(BackendRegistryEntry {
                registry_index: index as u32,
                gpu_or_igpu: kind == sys::ggml_backend_dev_type_GGML_BACKEND_DEVICE_TYPE_GPU
                    || kind == sys::ggml_backend_dev_type_GGML_BACKEND_DEVICE_TYPE_IGPU,
                name: CStr::from_ptr(name).to_string_lossy().into_owned(),
                pci_id,
            });
        }
    }
    let position = whisper_gpu_position(&entries, target_registry_index)?;
    let selected = entries
        .iter()
        .find(|entry| entry.registry_index == target_registry_index)
        .ok_or_else(|| "selected GGML device disappeared from registry".to_owned())?;
    validate_selected_cuda_registry_device(selected, expected_pci_id)?;
    Ok((position, selected.name.clone()))
}

fn normalize_pci(value: &str) -> String {
    let normalized = value.trim().replace('-', ":").to_ascii_lowercase();
    normalized
        .strip_prefix("00000000:")
        .unwrap_or(&normalized)
        .to_owned()
}

pub fn cuda_backend_available() -> bool {
    use std::ffi::CStr;
    use whisper_rs::whisper_rs_sys as sys;

    // SAFETY: ggml owns its device registry and strings. The backend handle is
    // used only in this call and freed exactly once before returning.
    unsafe {
        for index in 0..sys::ggml_backend_dev_count() {
            let device = sys::ggml_backend_dev_get(index);
            if device.is_null()
                || sys::ggml_backend_dev_type(device)
                    != sys::ggml_backend_dev_type_GGML_BACKEND_DEVICE_TYPE_GPU
            {
                continue;
            }
            let name = sys::ggml_backend_dev_name(device);
            if name.is_null() || !CStr::from_ptr(name).to_bytes().starts_with(b"CUDA") {
                continue;
            }
            let backend = sys::ggml_backend_dev_init(device, std::ptr::null());
            if !backend.is_null() {
                sys::ggml_backend_free(backend);
                return true;
            }
        }
    }
    false
}

#[cfg(windows)]
pub fn cuda_devices() -> Result<Vec<CudaDevice>, String> {
    use std::ffi::CStr;
    use windows::{
        Win32::{
            Foundation::{FreeLibrary, HMODULE},
            System::LibraryLoader::{GetProcAddress, LOAD_LIBRARY_SEARCH_SYSTEM32, LoadLibraryExW},
        },
        core::{PCSTR, PCWSTR},
    };

    struct Driver(HMODULE);
    impl Drop for Driver {
        fn drop(&mut self) {
            // SAFETY: this is the single owned handle returned by LoadLibraryExW.
            unsafe {
                let _ = FreeLibrary(self.0);
            }
        }
    }
    let name: Vec<u16> = "nvcuda.dll\0".encode_utf16().collect();
    // SAFETY: name is NUL-terminated and the system32-only flag restricts DLL lookup.
    let module =
        unsafe { LoadLibraryExW(PCWSTR(name.as_ptr()), None, LOAD_LIBRARY_SEARCH_SYSTEM32) }
            .map_err(|error| format!("nvcuda.dll unavailable from System32: {error}"))?;
    let _driver = Driver(module);
    unsafe fn symbol<T: Copy>(module: HMODULE, name: &'static [u8]) -> Result<T, String> {
        // SAFETY: the module is live and name is a static NUL-terminated export name.
        let proc = unsafe { GetProcAddress(module, PCSTR(name.as_ptr())) }.ok_or_else(|| {
            format!(
                "CUDA symbol {} unavailable",
                CStr::from_bytes_with_nul(name).unwrap().to_string_lossy()
            )
        })?;
        // SAFETY: caller requests the documented CUDA function signature for this symbol.
        let source: unsafe extern "system" fn() -> isize = proc;
        if std::mem::size_of::<T>() != std::mem::size_of_val(&source) {
            return Err("unexpected CUDA function pointer width".into());
        }
        // SAFETY: the preceding runtime width check establishes function-pointer size equality.
        Ok(unsafe { std::mem::transmute_copy(&source) })
    }
    type Init = unsafe extern "system" fn(u32) -> i32;
    type Count = unsafe extern "system" fn(*mut i32) -> i32;
    type Get = unsafe extern "system" fn(*mut i32, i32) -> i32;
    type Luid = unsafe extern "system" fn(*mut i8, *mut u32, i32) -> i32;
    type Pci = unsafe extern "system" fn(*mut i8, i32, i32) -> i32;
    type Capability = unsafe extern "system" fn(*mut i32, *mut i32, i32) -> i32;
    // SAFETY: resolves the documented `cuInit` ABI from the still-live module.
    let init: Init = unsafe { symbol(module, b"cuInit\0")? };
    // SAFETY: resolves the documented device-count ABI from the still-live module.
    let count_fn: Count = unsafe { symbol(module, b"cuDeviceGetCount\0")? };
    // SAFETY: resolves the documented device-handle ABI from the still-live module.
    let get: Get = unsafe { symbol(module, b"cuDeviceGet\0")? };
    // SAFETY: resolves the documented LUID ABI from the still-live module.
    let get_luid: Luid = unsafe { symbol(module, b"cuDeviceGetLuid\0")? };
    // SAFETY: resolves the documented PCI ID ABI from the still-live module.
    let get_pci: Pci = unsafe { symbol(module, b"cuDeviceGetPCIBusId\0")? };
    // SAFETY: resolves the documented compute-capability ABI from the still-live module.
    let capability: Capability = unsafe { symbol(module, b"cuDeviceComputeCapability\0")? };
    // SAFETY: exported CUDA function pointers have documented signatures and the module stays loaded.
    let status = unsafe { init(0) };
    if status != 0 {
        return Err(format!("cuInit failed: {status}"));
    }
    let mut count = 0;
    // SAFETY: CUDA writes the device count through this valid output pointer.
    let status = unsafe { count_fn(&mut count) };
    if status != 0 || !(0..=128).contains(&count) {
        return Err(format!(
            "cuDeviceGetCount failed or out of bounds: {status}, {count}"
        ));
    }
    let mut devices = Vec::new();
    for ordinal in 0..count {
        let mut device = -1;
        // SAFETY: ordinal is within the validated CUDA device count.
        if unsafe { get(&mut device, ordinal) } != 0 {
            continue;
        }
        let mut luid = [0_i8; 8];
        let mut node = 0_u32;
        // SAFETY: CUDA writes LUID and node mask into these fixed local outputs.
        let luid = (unsafe { get_luid(luid.as_mut_ptr(), &mut node, device) } == 0)
            .then(|| u64::from_le_bytes(luid.map(|byte| byte as u8)));
        let mut pci = [0_i8; 64];
        // SAFETY: CUDA receives the fixed buffer length and terminates the PCI string on success.
        let pci =
            (unsafe { get_pci(pci.as_mut_ptr(), pci.len() as i32, device) } == 0).then(|| {
                // SAFETY: CUDA's successful PCI query writes a NUL-terminated string into this buffer.
                unsafe { CStr::from_ptr(pci.as_ptr()) }
                    .to_string_lossy()
                    .into_owned()
            });
        let (mut major, mut minor) = (0, 0);
        // SAFETY: CUDA writes the compute-capability pair for this valid device.
        let compute =
            (unsafe { capability(&mut major, &mut minor, device) } == 0).then_some((major, minor));
        devices.push(CudaDevice {
            ordinal: ordinal as u32,
            luid,
            pci_id: pci,
            compute,
        });
    }
    Ok(devices)
}

#[cfg(not(windows))]
pub fn cuda_devices() -> Result<Vec<CudaDevice>, String> {
    Err("CUDA device inspection is available only on Windows".into())
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

pub fn ggml_devices() -> Vec<GgmlDevice> {
    use std::ffi::CStr;
    use whisper_rs::whisper_rs_sys as sys;
    let mut found = Vec::new();
    // SAFETY: handles and strings are borrowed from GGML's live registry; backend and buffer
    // handles are released in reverse ownership order on every iteration.
    unsafe {
        for index in 0..sys::ggml_backend_dev_count() {
            let dev = sys::ggml_backend_dev_get(index);
            if dev.is_null()
                || sys::ggml_backend_dev_type(dev)
                    != sys::ggml_backend_dev_type_GGML_BACKEND_DEVICE_TYPE_GPU
            {
                continue;
            }
            let name = sys::ggml_backend_dev_name(dev);
            if name.is_null() || !CStr::from_ptr(name).to_bytes().starts_with(b"CUDA") {
                continue;
            }
            let mut props = std::mem::zeroed();
            sys::ggml_backend_dev_get_props(dev, &mut props);
            let pci_id = (!props.device_id.is_null()).then(|| {
                CStr::from_ptr(props.device_id)
                    .to_string_lossy()
                    .into_owned()
            });
            let backend = sys::ggml_backend_dev_init(dev, std::ptr::null());
            let ready = if backend.is_null() {
                false
            } else {
                let buffer_type = sys::ggml_backend_dev_buffer_type(dev);
                let buffer = if buffer_type.is_null() {
                    std::ptr::null_mut()
                } else {
                    sys::ggml_backend_buft_alloc_buffer(buffer_type, 1024 * 1024)
                };
                let matches_device = !buffer.is_null()
                    && sys::ggml_backend_buft_get_device(sys::ggml_backend_buffer_get_type(buffer))
                        == dev;
                if !buffer.is_null() {
                    sys::ggml_backend_buffer_free(buffer);
                }
                sys::ggml_backend_free(backend);
                matches_device
            };
            found.push(GgmlDevice {
                ordinal: index as u32,
                pci_id,
                ready,
            });
        }
    }
    found
}

#[cfg(windows)]
pub fn verify_windows_identity(
    candidate: &whisper_core::gpu_inventory::GpuRuntimeIdentity,
) -> Result<(), String> {
    use windows::Win32::{
        Devices::Display::{
            DISPLAYCONFIG_ADAPTER_NAME, DISPLAYCONFIG_DEVICE_INFO_GET_ADAPTER_NAME,
            DisplayConfigGetDeviceInfo,
        },
        Graphics::Dxgi::{CreateDXGIFactory1, DXGI_ERROR_NOT_FOUND, IDXGIFactory1},
    };
    // SAFETY: windows-rs owns and releases the factory and adapter COM interfaces.
    unsafe {
        let factory: IDXGIFactory1 = CreateDXGIFactory1().map_err(|error| error.to_string())?;
        let mut matched = Vec::new();
        for ordinal in 0..128_u32 {
            let adapter = match factory.EnumAdapters1(ordinal) {
                Ok(adapter) => adapter,
                Err(error) if error.code().0 as u32 == DXGI_ERROR_NOT_FOUND.0 as u32 => break,
                Err(error) => return Err(format!("DXGI enumeration failed: {error}")),
            };
            let desc = adapter.GetDesc1().map_err(|error| error.to_string())?;
            let luid = ((desc.AdapterLuid.HighPart as u32 as u64) << 32)
                | u64::from(desc.AdapterLuid.LowPart);
            if ordinal == candidate.dxgi_rank && luid == candidate.luid {
                let mut name = DISPLAYCONFIG_ADAPTER_NAME::default();
                name.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_ADAPTER_NAME;
                name.header.size = std::mem::size_of::<DISPLAYCONFIG_ADAPTER_NAME>() as u32;
                name.header.adapterId = desc.AdapterLuid;
                let status = DisplayConfigGetDeviceInfo(&mut name.header);
                if status != 0 {
                    return Err(format!("DisplayConfigGetDeviceInfo failed: {status}"));
                }
                let path = name.adapterDevicePath;
                let identity = device_interface_instance_id(path.as_ptr())?;
                matched.push(identity);
            }
        }
        if matched.len() != 1 || !matched[0].eq_ignore_ascii_case(&candidate.pnp_instance_id) {
            return Err("PnP identity, DXGI rank, and LUID no longer form one exact join".into());
        }
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn verify_windows_identity(
    _: &whisper_core::gpu_inventory::GpuRuntimeIdentity,
) -> Result<(), String> {
    Err("Windows display identity verification is unavailable on this platform".into())
}

#[cfg(windows)]
fn device_interface_instance_id(path: *const u16) -> Result<String, String> {
    use windows::{
        Win32::Devices::{
            DeviceAndDriverInstallation::{
                DIGCF_ALLCLASSES, DIGCF_DEVICEINTERFACE, SP_DEVICE_INTERFACE_DATA,
                SP_DEVICE_INTERFACE_DETAIL_DATA_W, SP_DEVINFO_DATA, SetupDiDestroyDeviceInfoList,
                SetupDiGetClassDevsW, SetupDiGetDeviceInstanceIdW,
                SetupDiGetDeviceInterfaceDetailW, SetupDiOpenDeviceInterfaceW,
            },
            Display::GUID_DEVINTERFACE_DISPLAY_ADAPTER,
        },
        core::PCWSTR,
    };
    // SAFETY: requests a read-only information set for display adapter interfaces.
    let set = unsafe {
        SetupDiGetClassDevsW(
            Some(&GUID_DEVINTERFACE_DISPLAY_ADAPTER),
            PCWSTR::null(),
            None,
            DIGCF_ALLCLASSES | DIGCF_DEVICEINTERFACE,
        )
    }
    .map_err(|error| format!("SetupDiGetClassDevsW: {error}"))?;
    let result = (|| {
        let mut interface = SP_DEVICE_INTERFACE_DATA::default();
        interface.cbSize = std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32;
        // SAFETY: the DisplayConfig path is NUL terminated and the information set stays live.
        unsafe { SetupDiOpenDeviceInterfaceW(set, PCWSTR(path), 0, Some(&mut interface)) }
            .map_err(|error| format!("SetupDiOpenDeviceInterfaceW: {error}"))?;
        let mut required = 0_u32;
        // SAFETY: documented size-query call; no output buffer is supplied.
        let _ = unsafe {
            SetupDiGetDeviceInterfaceDetailW(set, &interface, None, 0, Some(&mut required), None)
        };
        if required < std::mem::size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32
            || required > 65_536
        {
            return Err(format!("invalid SetupAPI detail size: {required}"));
        }
        let mut storage = vec![0_u64; (required as usize).div_ceil(8)];
        let detail = storage
            .as_mut_ptr()
            .cast::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>();
        // SAFETY: aligned, bounded storage is large enough for the API-reported structure.
        unsafe {
            (*detail).cbSize = std::mem::size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;
        }
        let mut device = SP_DEVINFO_DATA::default();
        device.cbSize = std::mem::size_of::<SP_DEVINFO_DATA>() as u32;
        // SAFETY: initialized output and bounded detail storage are live for the API call.
        unsafe {
            SetupDiGetDeviceInterfaceDetailW(
                set,
                &interface,
                Some(detail),
                required,
                Some(&mut required),
                Some(&mut device),
            )
        }
        .map_err(|error| format!("SetupDiGetDeviceInterfaceDetailW: {error}"))?;
        let mut id = vec![0_u16; 1024];
        let mut id_required = 0_u32;
        // SAFETY: the live device info and bounded UTF-16 buffer are valid outputs.
        unsafe { SetupDiGetDeviceInstanceIdW(set, &device, Some(&mut id), Some(&mut id_required)) }
            .map_err(|error| format!("SetupDiGetDeviceInstanceIdW: {error}"))?;
        let end = id.iter().position(|unit| *unit == 0).unwrap_or(id.len());
        Ok(String::from_utf16_lossy(&id[..end]))
    })();
    // SAFETY: releases exactly the information-set handle acquired above.
    unsafe { SetupDiDestroyDeviceInfoList(set) }
        .map_err(|error| format!("SetupDiDestroyDeviceInfoList: {error}"))?;
    result
}
