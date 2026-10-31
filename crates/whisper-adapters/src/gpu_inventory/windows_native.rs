#![allow(unsafe_code)]
#![deny(unsafe_op_in_unsafe_fn)]

use windows::Win32::Devices::Display::GUID_DEVINTERFACE_DISPLAY_ADAPTER;
use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, DXGI_ERROR_NOT_FOUND, IDXGIFactory1};
use windows::{
    Win32::Devices::DeviceAndDriverInstallation::{
        DIGCF_ALLCLASSES, DIGCF_DEVICEINTERFACE, SP_DEVICE_INTERFACE_DATA,
        SP_DEVICE_INTERFACE_DETAIL_DATA_W, SP_DEVINFO_DATA, SetupDiDestroyDeviceInfoList,
        SetupDiGetClassDevsW, SetupDiGetDeviceInstanceIdW, SetupDiGetDeviceInterfaceDetailW,
        SetupDiOpenDeviceInterfaceW,
    },
    Win32::Devices::Display::{
        DISPLAYCONFIG_ADAPTER_NAME, DISPLAYCONFIG_DEVICE_INFO_GET_ADAPTER_NAME,
        DisplayConfigGetDeviceInfo,
    },
    Win32::Foundation::LUID,
    core::PCWSTR,
};

#[derive(Clone, Debug)]
pub struct DxgiAdapter {
    pub ordinal: u32,
    pub luid: LUID,
}

pub fn luid_u64(luid: LUID) -> u64 {
    ((luid.HighPart as u32 as u64) << 32) | u64::from(luid.LowPart)
}

pub fn adapter_instance_id(luid: LUID) -> Result<String, String> {
    // SAFETY: structure is initialized and DisplayConfig writes only this record.
    let mut name = DISPLAYCONFIG_ADAPTER_NAME::default();
    name.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_ADAPTER_NAME;
    name.header.size = std::mem::size_of::<DISPLAYCONFIG_ADAPTER_NAME>() as u32;
    name.header.adapterId = luid;
    // SAFETY: initialized header and output structure remain live for the call.
    let status = unsafe { DisplayConfigGetDeviceInfo(&mut name.header) };
    if status != 0 {
        return Err(format!("DisplayConfigGetDeviceInfo failed: {status}"));
    }
    let path = name.adapterDevicePath;
    // SAFETY: requests a read-only device information set for display interfaces.
    let set = unsafe {
        SetupDiGetClassDevsW(
            Some(&GUID_DEVINTERFACE_DISPLAY_ADAPTER),
            PCWSTR::null(),
            None,
            DIGCF_ALLCLASSES | DIGCF_DEVICEINTERFACE,
        )
    }
    .map_err(|e| format!("SetupDiGetClassDevsW: {e}"))?;
    let result = (|| {
        let mut interface = SP_DEVICE_INTERFACE_DATA {
            cbSize: std::mem::size_of::<SP_DEVICE_INTERFACE_DATA>() as u32,
            ..Default::default()
        };
        // SAFETY: path is the fixed NUL-terminated DisplayConfig output; set remains live.
        unsafe { SetupDiOpenDeviceInterfaceW(set, PCWSTR(path.as_ptr()), 0, Some(&mut interface)) }
            .map_err(|e| format!("SetupDiOpenDeviceInterfaceW: {e}"))?;
        let mut required = 0_u32;
        // SAFETY: this is the documented size-query call with no output buffer.
        let _ = unsafe {
            SetupDiGetDeviceInterfaceDetailW(set, &interface, None, 0, Some(&mut required), None)
        };
        if required < std::mem::size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32
            || required > 65_536
        {
            return Err(format!("invalid device interface size {required}"));
        }
        let mut storage = vec![0_u64; (required as usize).div_ceil(8)];
        let detail = storage
            .as_mut_ptr()
            .cast::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>();
        // SAFETY: aligned storage is sized from the API's bounded required length.
        unsafe {
            (*detail).cbSize = std::mem::size_of::<SP_DEVICE_INTERFACE_DETAIL_DATA_W>() as u32;
        }
        let mut devinfo = SP_DEVINFO_DATA {
            cbSize: std::mem::size_of::<SP_DEVINFO_DATA>() as u32,
            ..Default::default()
        };
        // SAFETY: detail storage and initialized devinfo remain writable for the call.
        unsafe {
            SetupDiGetDeviceInterfaceDetailW(
                set,
                &interface,
                Some(detail),
                required,
                Some(&mut required),
                Some(&mut devinfo),
            )
        }
        .map_err(|e| format!("SetupDiGetDeviceInterfaceDetailW: {e}"))?;
        let mut id = vec![0_u16; 1024];
        let mut id_required = 0_u32;
        // SAFETY: live device info and bounded UTF-16 output buffer are supplied.
        unsafe {
            SetupDiGetDeviceInstanceIdW(set, &devinfo, Some(&mut id), Some(&mut id_required))
        }
        .map_err(|e| format!("SetupDiGetDeviceInstanceIdW: {e}"))?;
        let end = id.iter().position(|unit| *unit == 0).unwrap_or(id.len());
        Ok(String::from_utf16_lossy(&id[..end]))
    })();
    // SAFETY: releases exactly the information-set handle acquired above.
    let close = unsafe { SetupDiDestroyDeviceInfoList(set) };
    if let Err(error) = close {
        return Err(format!("SetupDiDestroyDeviceInfoList: {error}"));
    }
    result
}

/// Enumerate the Windows display adapters in DXGI order. Adapter identity is
/// kept as the opaque LUID; names are presentation only.
pub fn enumerate_dxgi() -> Result<Vec<DxgiAdapter>, String> {
    // SAFETY: the factory and adapter COM interfaces are owned wrappers from
    // windows-rs and are dropped on every return path.
    unsafe {
        let factory: IDXGIFactory1 =
            CreateDXGIFactory1().map_err(|e| format!("DXGI indisponible : {e}"))?;
        let mut adapters = Vec::new();
        for ordinal in 0..128_u32 {
            let adapter = match factory.EnumAdapters1(ordinal) {
                Ok(adapter) => adapter,
                Err(error) if error.code().0 as u32 == DXGI_ERROR_NOT_FOUND.0 as u32 => break,
                Err(error) => return Err(format!("Énumération DXGI interrompue : {error}")),
            };
            let desc = adapter
                .GetDesc1()
                .map_err(|e| format!("Description DXGI indisponible : {e}"))?;
            adapters.push(DxgiAdapter {
                ordinal,
                luid: desc.AdapterLuid,
            });
        }
        Ok(adapters)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_dxgi_enumeration_has_ordered_ranks_and_setupapi_identity_join() {
        let adapters = enumerate_dxgi().expect("DXGI inventory should be readable");
        assert!(!adapters.is_empty());
        for (expected, adapter) in adapters.iter().enumerate() {
            assert_eq!(adapter.ordinal as usize, expected);
        }
        let joined = adapters
            .iter()
            .filter_map(|adapter| adapter_instance_id(adapter.luid).ok())
            .collect::<Vec<_>>();
        assert!(
            !joined.is_empty(),
            "at least one DXGI adapter should resolve to a PnP instance ID"
        );
        assert!(joined.iter().all(|identity| !identity.trim().is_empty()));
    }
}
