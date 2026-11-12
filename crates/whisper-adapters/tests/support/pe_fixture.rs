use std::{fs, path::Path};

#[allow(dead_code)]
pub fn copy_current_executable(executable: &Path) {
    copy_verified_pe(
        &std::env::current_exe().expect("test executable path"),
        executable,
    );
}

pub fn copy_verified_pe(source: &Path, executable: &Path) {
    assert!(is_pe_image(source), "copy source must be a PE image");
    fs::create_dir_all(executable.parent().expect("worker parent"))
        .expect("create worker directory");
    fs::copy(source, executable).expect("copy verified PE fixture");
    assert!(is_pe_image(executable), "copied fixture must be a PE image");
}

fn is_pe_image(path: &Path) -> bool {
    let Ok(bytes) = fs::read(path) else {
        return false;
    };
    if bytes.get(..2) != Some(&b"MZ"[..]) {
        return false;
    }
    let Some(offset_bytes) = bytes.get(0x3c..0x40) else {
        return false;
    };
    let offset = u32::from_le_bytes(offset_bytes.try_into().expect("four-byte PE offset")) as usize;
    bytes.get(offset..offset.saturating_add(4)) == Some(&b"PE\0\0"[..])
}
