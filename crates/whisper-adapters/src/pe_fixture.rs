use std::{fs, path::Path, process::Command};

#[path = "../tests/support/pe_fixture.rs"]
mod shared;

pub(crate) use shared::copy_verified_pe;

pub(crate) fn compile_failing_worker(root: &Path, executable: &Path) {
    fs::create_dir_all(executable.parent().expect("worker parent"))
        .expect("create worker directory");
    let source = root.join("failing-worker-fixture.rs");
    fs::write(&source, "fn main() { std::process::exit(17); }\n").expect("write worker source");
    let output = Command::new("rustc")
        .arg("--edition=2021")
        .arg(&source)
        .arg("-o")
        .arg(executable)
        .output()
        .expect("run rustc for worker fixture");
    assert!(
        output.status.success(),
        "rustc worker fixture failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(is_pe_image(executable), "worker fixture must be a PE image");
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
