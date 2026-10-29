use sha2::{Digest, Sha256};
use std::{fs::File, io::Read, path::Path};

pub(crate) fn verify_sha256(path: &Path, expected: &[u8; 32]) -> Result<(), String> {
    let mut file = File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    if digest.finalize().as_slice() != expected {
        return Err(format!("SHA-256 mismatch for {}", path.display()));
    }
    Ok(())
}
