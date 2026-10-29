use super::*;

#[test]
fn accepts_expected_digest_and_rejects_mismatch_and_missing_file() {
    let path = std::env::temp_dir().join(format!("worker-common-integrity-{}", std::process::id()));
    std::fs::write(&path, b"abc").unwrap();
    let expected: [u8; 32] = Sha256::digest(b"abc").into();
    assert!(verify_sha256(&path, &expected).is_ok());
    assert!(
        verify_sha256(&path, &[0; 32])
            .unwrap_err()
            .contains("SHA-256 mismatch")
    );
    assert!(verify_sha256(&path.with_extension("absent"), &expected).is_err());
}
